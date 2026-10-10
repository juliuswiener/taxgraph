//! § 22 Nr. 5 `EStG` im Bescheid: Leistungen mit Leistungsmitteilung der Hauptperson, Anlage R-AV/bAV (Abweichung Nr. 55 in
//! `rust/fixtures/README.md`). Nur Rust: Python kennt die Anlage nicht.
//!
//! * [`rav_bav_einkuenfte`]: die steuerpflichtigen Anteile aller bestaetigten Mitteilungen fuer den Ring, VOR dem Pauschbetrag.
//!   Nr. 1 und Nr. 4 (Zeilen 4 und 10) sind voll steuerpflichtig, Nr. 5 (Zeile 15) mit dem Ertragsanteil
//!   (`engine::zugriff::teil2::rav_bav`). Den Pauschbetrag zieht der Ring einmal fuer alle Renten einer Person ab
//!   (`engine::zugriff::teil2::rente::renten_nach_pauschbetrag`).
//!
//! * [`rav_bav_sperrgrund`]: der Guard. Eine Mitteilung, die die Software nicht rechnet (Ehepaar, weitere Nummern, Basisrente),
//!   sperrt mit [`Sperrgrund::RavBavNichtGerechnet`]; fehlende oder unstimmige Angaben sperren mit
//!   [`Sperrgrund::RavBavAngabenOffen`]. Beides VOR jeder Zahl: eine Mitteilung wird nie still weggelassen.
//!
//! Eine Mitteilung, die [`lese`] nicht liest, traegt in [`rav_bav_einkuenfte`] nichts bei; der Guard sperrt sie.
use bindung::Params;
use domain::{Cent, Euro, PyWert, Sperrgrund, Vz, Zustand};
use elster::Instanz;
use engine::zugriff::teil2::privatverkauf::Tag;
use engine::zugriff::teil2::rav_bav::{alter_bei_beginn, mitteilung_stpfl, Mitteilung};

use crate::{
    euro_plus, ist_false, ist_zusammen, wert, zahl_oder_null, BescheidFehler, Felder, Instanzquelle,
};

/// Die Instanz-Gruppe der Mitteilungen in der Bindung (`bindung_regel_bedingungen.yaml`, `bindung_rav_bav.yaml`).
const GRUPPE: &str = "rente_av_bav";

/// Die steuerpflichtigen Anteile aller Mitteilungen, EURO, vor dem Pauschbetrag. `voll` (Nr. 1 und Nr. 4) geht in die Bemessung
/// des Altersentlastungsbetrags ein, `ertragsanteil` (Nr. 5) nicht (§ 24a Satz 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RavBavEinkuenfte {
    pub voll: Euro,
    pub ertragsanteil: Euro,
}

fn text<'a>(f: &'a Felder, feld: &str) -> Option<&'a str> {
    match wert(f, feld) {
        Some(PyWert::Text(s)) => Some(s.as_str()),
        _ => None,
    }
}

fn bool_antwort(f: &Felder, feld: &str) -> Option<bool> {
    match wert(f, feld) {
        Some(PyWert::Bool(b)) => Some(*b),
        _ => None,
    }
}

/// Ein Jahresbetrag in vollen Euro (abgerundet, wie der Schreiber). `None`: nicht beantwortet oder negativ. Eine Null ist eine
/// Antwort.
fn betrag(f: &Felder, feld: &str) -> Result<Option<Euro>, BescheidFehler> {
    let Some(zahl) = wert(f, feld).and_then(PyWert::zahl_ohne_bool) else {
        return Ok(None);
    };
    let cent = zahl_oder_null(Some(zahl))?;
    Ok((cent >= 0).then(|| Cent::new(cent).floor_euro()))
}

/// Der Beginn einer Leistung: ein Datum im Steuerjahr oder davor (`ERiC` lehnt einen spaeteren Beginn ab, Spike Nr. 55 K5/K10).
/// `jahr` `None` (Alt-Aufrufer ohne Veranlagungsjahr) laesst die Pruefung des Jahres aus.
fn beginn(f: &Felder, feld: &str, jahr: Option<i32>) -> Option<Tag> {
    text(f, feld)
        .and_then(Tag::aus_tt_mm_jjjj)
        .filter(|t| jahr.is_none_or(|j| t.jahr <= j))
}

/// Eine gelesene, in sich stimmige Mitteilung.
struct Gelesen {
    nr1: Euro,
    nr4: Euro,
    nr5: Euro,
    alter_bei_beginn_nr5: Option<i64>,
}

/// Liest eine Mitteilung. `Ok(Err(grund))` heisst: sie ist nicht rechenbar, der Grund sagt warum. Zuerst kommt, was die Software
/// bewusst nicht rechnet (weitere Nummern, Basisrente), dann, was der Nutzer nachtragen kann. Der Beginn gilt nur zu einem Betrag
/// ueber 0: `ERiC` verlangt Betrag und Beginn gemeinsam (Spike Nr. 55 B/K1), ein Beginn ohne Betrag wird also nicht geschrieben.
fn lese(
    inst: &Instanz,
    geburt: Option<Tag>,
    jahr: Option<i32>,
) -> Result<Result<Gelesen, Sperrgrund>, BescheidFehler> {
    use Sperrgrund::{RavBavAngabenOffen as offen, RavBavNichtGerechnet as nicht_gerechnet};
    let f = &inst.felder;
    let (andere, basisrente) = (
        bool_antwort(f, "rav_bav_andere_nummern"),
        bool_antwort(f, "rav_bav_basisrente"),
    );
    if andere == Some(true) || basisrente == Some(true) {
        return Ok(Err(nicht_gerechnet));
    }
    if andere.is_none() || basisrente.is_none() {
        return Ok(Err(offen));
    }
    let (Some(nr1), Some(nr4), Some(nr5)) = (
        betrag(f, "rav_bav_nr1_betrag")?,
        betrag(f, "rav_bav_nr4_betrag")?,
        betrag(f, "rav_bav_nr5_betrag")?,
    ) else {
        return Ok(Err(offen));
    };
    if nr4.get() > 0 && beginn(f, "rav_bav_nr4_beginn", jahr).is_none() {
        return Ok(Err(offen));
    }
    // Das Alter bei Rentenbeginn braucht Beginn und Geburtsdatum; es ist keine Frage (`luecken` in `bindung_rav_bav.yaml`).
    let alter_bei_beginn_nr5 = if nr5.get() > 0 {
        let alter = beginn(f, "rav_bav_nr5_beginn", jahr)
            .zip(geburt)
            .and_then(|(b, g)| alter_bei_beginn(g, b));
        let Some(alter) = alter else {
            return Ok(Err(offen));
        };
        Some(alter)
    } else {
        None
    };
    Ok(Ok(Gelesen {
        nr1,
        nr4,
        nr5,
        alter_bei_beginn_nr5,
    }))
}

fn geburtsdatum(f: &Felder) -> Option<Tag> {
    text(f, "stammdaten_geburtsdatum").and_then(Tag::aus_tt_mm_jjjj)
}

/// Die Einkuenfte aus Mitteilungen fuer den Ring, EURO, vor dem Pauschbetrag. Null ohne Store oder Bindung und solange
/// `kein_rav_bav` nicht auf "nein" ("ich habe eine Rente mit Leistungsmitteilung") steht: dann ist die Regel der Gruppe nicht in
/// Kraft (`regel_bedingungen`). Ein Wert aus einer Mitteilung, die der Guard sperrt, zaehlt nicht.
///
/// **Sicherheit:** bei `nur_bestaetigt` zaehlt eine vorlaeufige Instanz nicht.
///
/// # Errors
/// Instanz-, Accessor- und Ueberlauf-Fehler.
pub fn rav_bav_einkuenfte(
    f: &Felder,
    q: &Instanzquelle<'_>,
    vz: Vz,
    p: &Params,
) -> Result<RavBavEinkuenfte, BescheidFehler> {
    let mut summe = RavBavEinkuenfte {
        voll: Euro::new(0),
        ertragsanteil: Euro::new(0),
    };
    if q.beide().is_none() || !ist_false(wert(f, "kein_rav_bav")) {
        return Ok(summe);
    }
    let geburt = geburtsdatum(f);
    for inst in q.instanzen(GRUPPE)? {
        if !q.zaehlt(&inst) {
            continue;
        }
        let Ok(gelesen) = lese(&inst, geburt, Some(i32::from(vz.jahr())))? else {
            continue;
        };
        let mitteilung = Mitteilung {
            vz,
            nr1: gelesen.nr1,
            nr4: gelesen.nr4,
            nr5: gelesen.nr5,
            alter_bei_beginn_nr5: gelesen.alter_bei_beginn_nr5,
        };
        let (voll, ertragsanteil) = mitteilung_stpfl(&mitteilung, p)?;
        summe.voll = euro_plus(summe.voll, voll)?;
        summe.ertragsanteil = euro_plus(summe.ertragsanteil, ertragsanteil)?;
    }
    Ok(summe)
}

/// Der Guard: sperrt, solange eine Mitteilung nicht rechenbar ist. `kein_rav_bav` = nein heisst "ich habe eine Rente mit
/// Leistungsmitteilung". Reihenfolge: was die Software nie rechnet geht vor dem, was der Nutzer nachtragen kann.
///
/// * Kreuz bejaht, Zusammenveranlagung: [`Sperrgrund::RavBavNichtGerechnet`]. Keine Angabe sagt, wem die Mitteilung gehoert, und die
///   Rente des Ehegatten (Person B) ist nicht gebaut; ohne die Sperre fehlte sie still in der Zahl.
/// * Kreuz bejaht, eine Mitteilung mit weiteren Nummern oder Basisrente: [`Sperrgrund::RavBavNichtGerechnet`].
/// * Kreuz bejaht, keine Mitteilung oder eine unvollstaendige oder vorlaeufige: [`Sperrgrund::RavBavAngabenOffen`].
/// * Kreuz nicht bejaht (nein oder nie gefragt) und trotzdem ein bestaetigter Betrag ueber 0: [`Sperrgrund::RavBavAngabenOffen`], der
///   Widerspruch, den `FLAG_NEGIERT` fuer die uebrigen Kreuze faengt (dessen Konstante ist mit Python abgeglichen und bleibt
///   unveraendert). Sonst wuerde der Betrag still wegfallen.
///
/// `vz` `None` (Alt-Aufrufer ohne Jahr) laesst die Pruefung des Beginnjahres aus.
///
/// # Errors
/// Instanz- und Ueberlauf-Fehler.
pub(crate) fn rav_bav_sperrgrund(
    f: &Felder,
    q: &Instanzquelle<'_>,
    vz: Option<Vz>,
) -> Result<Option<Sperrgrund>, BescheidFehler> {
    let bejaht = ist_false(wert(f, "kein_rav_bav"));
    if bejaht && ist_zusammen(f) {
        return Ok(Some(Sperrgrund::RavBavNichtGerechnet));
    }
    if q.beide().is_none() {
        return Ok(bejaht.then_some(Sperrgrund::RavBavAngabenOffen));
    }
    let instanzen = q.instanzen(GRUPPE)?;
    if !bejaht {
        for inst in &instanzen {
            if inst.zustand != Zustand::Bestaetigt {
                continue;
            }
            for feld in [
                "rav_bav_nr1_betrag",
                "rav_bav_nr4_betrag",
                "rav_bav_nr5_betrag",
            ] {
                if betrag(&inst.felder, feld)?.is_some_and(|b| b.get() > 0) {
                    return Ok(Some(Sperrgrund::RavBavAngabenOffen));
                }
            }
        }
        return Ok(None);
    }
    // Ein bejahtes Kreuz ohne eine einzige Mitteilung ist eine offene Angabe, kein "keine Rente".
    let mut offen = instanzen.is_empty();
    let (geburt, jahr) = (geburtsdatum(f), vz.map(|v| i32::from(v.jahr())));
    for inst in &instanzen {
        if inst.zustand != Zustand::Bestaetigt {
            offen = true;
            continue;
        }
        match lese(inst, geburt, jahr)? {
            Ok(_) => {}
            Err(Sperrgrund::RavBavNichtGerechnet) => {
                return Ok(Some(Sperrgrund::RavBavNichtGerechnet));
            }
            Err(_) => offen = true,
        }
    }
    Ok(offen.then_some(Sperrgrund::RavBavAngabenOffen))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::testhilfe::{felder, store};

    /// Ohne Store gibt es keine Instanzen: ein bejahtes Kreuz ist eine offene Angabe, jedes andere Kreuz laesst nichts zu pruefen.
    #[test]
    fn ohne_store_ist_ein_bejahtes_kreuz_offen_sonst_nichts_zu_pruefen() {
        let q = Instanzquelle {
            store: None,
            bindung: None,
            nur_bestaetigt: true,
        };
        let mit_kreuz = |kein: bool| felder(&store(&[("kein_rav_bav", json!(kein), true)]));
        assert_eq!(
            rav_bav_sperrgrund(&mit_kreuz(false), &q, None).unwrap(),
            Some(Sperrgrund::RavBavAngabenOffen)
        );
        assert_eq!(
            rav_bav_sperrgrund(&mit_kreuz(true), &q, None).unwrap(),
            None
        );
    }

    /// Der Beginn einer Leistung darf im Steuerjahr oder davor liegen; ohne Jahr (Alt-Aufrufer) entfaellt die Grenze, ein Text
    /// ohne Datum gilt nie.
    #[test]
    fn der_beginn_liegt_im_steuerjahr_oder_davor() {
        let f = felder(&store(&[
            ("a", json!("31.12.2025"), true),
            ("b", json!("01.01.2026"), true),
            ("c", json!("2025"), true),
        ]));
        assert!(beginn(&f, "a", Some(2025)).is_some());
        assert!(beginn(&f, "b", Some(2025)).is_none());
        assert!(beginn(&f, "b", None).is_some());
        assert!(beginn(&f, "c", None).is_none());
        assert!(beginn(&f, "fehlt", None).is_none());
    }
}
