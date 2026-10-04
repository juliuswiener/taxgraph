//! § 9 Abs. 1 S. 3 Nr. 4/4a, Abs. 2 `EStG` Entfernungspauschale.
use catala_sys::{CatalaFehler, EntfernungspauschaleErgebnis};
use domain::{Cent, Km};

use crate::dezimal::{self, DezimalUeberlauf};

/// Eingabe fuer [`berechnen`], 1:1 `Entfernungspauschale.Berechnung__Berechnung_in`.
#[derive(Debug, Clone, Copy)]
pub struct EntfernungspauschaleEingabe {
    pub entfernung_km_roh: Km,
    pub arbeitstage: i64,
    pub eigenes_oder_ueberlassenes_kfz: bool,
    pub oepnv_kosten_jahr: Cent,
    pub satz_bis_20_km: Cent,
    pub satz_ab_21_km: Cent,
    pub staffelgrenze_km: i64,
    pub hoechstbetrag: Cent,
}

/// [`berechnen`] kann an der Decimal->Bruch-Umwandlung, an der Vorab-Pruefung des Jahresbetrags ODER am Catala-Scope selbst scheitern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EntfernungspauschaleFehler {
    #[error(transparent)]
    Dezimal(#[from] DezimalUeberlauf),
    #[error(transparent)]
    Catala(#[from] CatalaFehler),
    /// Der Jahresbetrag in Cent passt nicht in `i64` (`ep_gesamt`). Python rechnet mit beliebig grossen Ganzzahlen; kein
    /// Gegenstueck, fail-closed statt Wert mod 2^63.
    #[error("i64-Ueberlauf in {0}")]
    Ueberlauf(&'static str),
}

/// Vorab-Pruefung des Jahresbetrags, den der Scope bildet: `Tage * (km_bis_grenze * Satz1 + km_ueber_grenze * Satz2)` in Cent.
///
/// Der Scope rechnet in GMP exakt. Der C-Shim liest seine Ausgabe mit `mpz_get_si`, und das gibt bei einem Wert ausserhalb `long`
/// still dessen untere 63 Bit zurueck (Wert mod 2^63), keinen Fehler (Bericht h8-ep-fenster). `ep_ab_21km` prueft nur die
/// Teilprodukte; zwischen ihrer Grenze und der des Gesamtbetrags lag ein Fenster, in dem Rust eine falsche Zahl lieferte.
/// Mit Kfz gibt der Scope den Betrag aus, also ist ein Betrag ausserhalb `i64` ein Ueberlauf. Ohne Kfz deckelt er vorher auf den
/// Hoechstbetrag: ein positiver Betrag ausserhalb `i64` ist dort ein richtiger Wert (der Hoechstbetrag), kein Fehler.
///
/// Sitzt in [`berechnen`], der einzigen Stelle, die den Scope aufruft: jeder Zugang (`werbungskosten::entfernungspauschale`,
/// `sachverhalt::Sachverhalt::Entfernungspauschale`) ist damit erfasst.
fn gesamt_pruefen(s: &EntfernungspauschaleEingabe) -> Result<(), EntfernungspauschaleFehler> {
    // Passt der volle km nicht in i64, meldet der Scope selbst den Dezimal-Fehler (Zaehler ausserhalb i64): nichts zu pruefen.
    let Some(km) = s.entfernung_km_roh.volle_km() else {
        return Ok(());
    };
    let grenze = s.staffelgrenze_km;
    let (bis, ueber) = if km > grenze {
        (
            grenze,
            km.checked_sub(grenze)
                .ok_or(EntfernungspauschaleFehler::Ueberlauf("ep_gesamt"))?,
        )
    } else {
        (km, 0)
    };
    let (satz_bis, satz_ab) = (s.satz_bis_20_km.get(), s.satz_ab_21_km.get());
    let gesamt = bis
        .checked_mul(satz_bis)
        .zip(ueber.checked_mul(satz_ab))
        .and_then(|(a, b)| a.checked_add(b))
        .and_then(|pro_tag| pro_tag.checked_mul(s.arbeitstage));
    let positiv = [bis, ueber, satz_bis, satz_ab, s.arbeitstage]
        .iter()
        .all(|x| *x >= 0);
    if gesamt.is_none() && (s.eigenes_oder_ueberlassenes_kfz || !positiv) {
        return Err(EntfernungspauschaleFehler::Ueberlauf("ep_gesamt"));
    }
    Ok(())
}

/// § 9 Abs. 1 S. 3 Nr. 4/4a `EStG`: einfache Entfernung, Arbeitstage, Kfz-Flag, OePNV-Kosten,
/// VZ-Saetze und Hoechstbetrag auf die Entfernungspauschale (vor und nach Guenstigerpruefung).
///
/// # Errors
/// Siehe [`EntfernungspauschaleFehler`].
///
/// ```
/// use engine::entfernungspauschale::{berechnen, EntfernungspauschaleEingabe};
/// use domain::{Cent, Km};
/// use rust_decimal::Decimal;
/// let ergebnis = berechnen(EntfernungspauschaleEingabe {
///     entfernung_km_roh: Km::new(Decimal::new(106, 1)),
///     arbeitstage: 200,
///     eigenes_oder_ueberlassenes_kfz: false,
///     oepnv_kosten_jahr: Cent::new(0),
///     satz_bis_20_km: Cent::new(30),
///     satz_ab_21_km: Cent::new(38),
///     staffelgrenze_km: 20,
///     hoechstbetrag: Cent::new(450_000),
/// })
/// .unwrap();
/// assert_eq!(ergebnis.entfernungspauschale_cent, 60_000);
/// ```
pub fn berechnen(
    eingabe: EntfernungspauschaleEingabe,
) -> Result<EntfernungspauschaleErgebnis, EntfernungspauschaleFehler> {
    gesamt_pruefen(&eingabe)?;
    let (num, den) = dezimal::zu_bruch(eingabe.entfernung_km_roh.get())?;
    let ergebnis = catala_sys::entfernungspauschale(catala_sys::EntfernungspauschaleEingabe {
        entfernung_km_roh_num: num,
        entfernung_km_roh_den: den,
        arbeitstage: eingabe.arbeitstage,
        eigenes_oder_ueberlassenes_kfz: eingabe.eigenes_oder_ueberlassenes_kfz,
        oepnv_kosten_jahr_cent: eingabe.oepnv_kosten_jahr.get(),
        satz_bis_20_km_cent: eingabe.satz_bis_20_km.get(),
        satz_ab_21_km_cent: eingabe.satz_ab_21_km.get(),
        staffelgrenze_km: eingabe.staffelgrenze_km,
        hoechstbetrag_cent: eingabe.hoechstbetrag.get(),
    })?;
    Ok(ergebnis)
}

#[cfg(test)]
mod tests {
    use super::{berechnen, EntfernungspauschaleEingabe};
    use domain::{Cent, Km};
    use rust_decimal::Decimal;

    #[test]
    fn angefangener_km_bleibt_unberuecksichtigt() {
        let ergebnis = berechnen(EntfernungspauschaleEingabe {
            entfernung_km_roh: Km::new(Decimal::new(106, 1)),
            arbeitstage: 200,
            eigenes_oder_ueberlassenes_kfz: false,
            oepnv_kosten_jahr: Cent::new(0),
            satz_bis_20_km: Cent::new(30),
            satz_ab_21_km: Cent::new(38),
            staffelgrenze_km: 20,
            hoechstbetrag: Cent::new(450_000),
        })
        .unwrap();
        assert_eq!(ergebnis.entfernungspauschale_cent, 60_000);
    }
}
