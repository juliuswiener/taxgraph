//! Abschreibungen: § 6 Abs. 2 GWG-Sofortabzug (`catala_p6_2_gwg`, Catala) und § 7 Abs. 1
//! lineare `AfA` (`catala_p7_linear_afa`, Hand-Python). Alle EURO.
use domain::{Cent, Euro};

use super::fehler::{in_cent, ok, EngineFehler};
use crate::gwg::{self, GwgEingabe};

/// Eingabe fuer [`p6_2_gwg`].
#[derive(Debug, Clone, Copy)]
pub struct P62GwgEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub gwg_anschaffungskosten_netto: Euro,
}

/// `catala_p6_2_gwg` -- § 6 Abs. 2 `EStG` Sofortabzug EINES geringwertigen Wirtschaftsguts,
/// EURO: Anschaffungskosten netto bis 800 voll, darueber 0 (dann `AfA`). Der Aufrufer summiert
/// ueber alle GWG-Instanzen.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use engine::zugriff::teil1::afa::{p6_2_gwg, P62GwgEingabe};
/// let e = P62GwgEingabe { gwg_anschaffungskosten_netto: domain::Euro::new(801) };
/// assert_eq!(p6_2_gwg(&e).unwrap(), domain::Euro::new(0));
/// ```
pub fn p6_2_gwg(e: &P62GwgEingabe) -> Result<Euro, EngineFehler> {
    let c = gwg::berechnen(GwgEingabe { anschaffungskosten_netto: in_cent(e.gwg_anschaffungskosten_netto)? })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`p7_linear_afa`]. Zwei Signaturen: ALT `anschaffungskosten_cent` (hat Vorrang,
/// wenn > 0) und NEU `anschaffungskosten` in Euro mit Monat und Anschaffungsjahr-Flag.
#[derive(Debug, Clone, Copy)]
pub struct P7LinearAfaEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub anschaffungskosten_cent: Cent,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open). Nur gelesen, wenn `anschaffungskosten_cent <= 0`.
    pub anschaffungskosten: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Ergebnis 0)
    pub nutzungsdauer: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: kein Pro-rata, voller Jahresbetrag)
    pub anschaffung_monat: i64,
    /// PARITÄT: Python setzt fehlend = None/falsy (fail-open: voller Jahresbetrag statt Pro-rata)
    pub ist_anschaffungsjahr: bool,
}

/// `catala_p7_linear_afa` -- § 7 Abs. 1 S. 1/2/4 `EStG` lineare `AfA`, EURO.
///
/// Jahresbetrag = Anschaffungskosten / Nutzungsdauer (abgerundet). Im Anschaffungsjahr (S. 4)
/// pro rata temporis: je vollem Monat vor der Anschaffung ein Zwoelftel weniger, also
/// `(13 - monat) / 12` des Jahresbetrags (abgerundet). Folgejahr oder Monat ausserhalb 1-12:
/// voller Jahresbetrag. Nutzungsdauer oder Kosten <= 0: 0.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use engine::zugriff::teil1::afa::{p7_linear_afa, P7LinearAfaEingabe};
/// let e = P7LinearAfaEingabe {
///     anschaffungskosten_cent: domain::Cent::new(0), anschaffungskosten: domain::Euro::new(1200),
///     nutzungsdauer: 3, anschaffung_monat: 10, ist_anschaffungsjahr: true,
/// };
/// assert_eq!(p7_linear_afa(&e).unwrap(), domain::Euro::new(100));
/// ```
pub fn p7_linear_afa(e: &P7LinearAfaEingabe) -> Result<Euro, EngineFehler> {
    let ak = if e.anschaffungskosten_cent.get() > 0 {
        e.anschaffungskosten_cent.floor_euro().get()
    } else {
        e.anschaffungskosten.get()
    };
    let nd = e.nutzungsdauer;
    if nd <= 0 || ak <= 0 {
        return Ok(Euro::new(0));
    }
    let jahresbetrag = ok(ak.checked_div_euclid(nd), "afa jahresbetrag")?;
    if e.ist_anschaffungsjahr && (1..=12).contains(&e.anschaffung_monat) {
        let monate_im_jahr = 13 - e.anschaffung_monat;
        return ok(
            jahresbetrag.checked_mul(monate_im_jahr).and_then(|x| x.checked_div_euclid(12)),
            "afa pro rata",
        )
        .map(Euro::new);
    }
    Ok(Euro::new(jahresbetrag))
}
