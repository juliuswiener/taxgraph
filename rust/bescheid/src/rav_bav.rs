//! § 22 Nr. 5 `EStG` im Bescheid: Leistungen mit Leistungsmitteilung der Hauptperson, Anlage R-AV/bAV (Abweichung Nr. 55 in
//! `rust/fixtures/README.md`). Nur Rust: Python kennt die Anlage nicht.
//!
//! * [`rav_bav_einkuenfte`]: die steuerpflichtigen Anteile aller bestaetigten Mitteilungen fuer den Ring, VOR dem Pauschbetrag.
//!   Nr. 1 und Nr. 4 (Zeilen 4 und 10) sind voll steuerpflichtig, Nr. 5 (Zeile 15) mit dem Ertragsanteil
//!   (`engine::zugriff::teil2::rav_bav`). Den Pauschbetrag zieht der Ring einmal fuer alle Renten einer Person ab
//!   (`engine::zugriff::teil2::rente::renten_nach_pauschbetrag`).
//!
//! Eine Mitteilung, die [`lese`] nicht liest, traegt hier nichts bei; der Guard sperrt sie vor jeder Zahl, die einen Nutzer
//! erreicht (wie `privatverkauf`: eine Mitteilung wird nie still weggelassen).
use bindung::Params;
use domain::{Cent, Euro, PyWert, Sperrgrund, Vz};
use elster::Instanz;
use engine::zugriff::teil2::privatverkauf::Tag;
use engine::zugriff::teil2::rav_bav::{alter_bei_beginn, mitteilung_stpfl, Mitteilung};

use crate::{euro_plus, ist_false, wert, zahl_oder_null, BescheidFehler, Felder, Instanzquelle};

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
fn beginn(f: &Felder, feld: &str, jahr: i32) -> Option<Tag> {
    text(f, feld)
        .and_then(Tag::aus_tt_mm_jjjj)
        .filter(|t| t.jahr <= jahr)
}

/// Liest eine Mitteilung. `Ok(Err(grund))` heisst: sie ist nicht rechenbar, der Grund sagt warum. Zuerst kommt, was die Software
/// bewusst nicht rechnet (weitere Nummern, Basisrente), dann, was der Nutzer nachtragen kann. Der Beginn gilt nur zu einem Betrag
/// ueber 0: `ERiC` verlangt Betrag und Beginn gemeinsam (Spike Nr. 55 B/K1), ein Beginn ohne Betrag wird also nicht geschrieben.
fn lese(
    inst: &Instanz,
    geburt: Option<Tag>,
    vz: Vz,
) -> Result<Result<Mitteilung, Sperrgrund>, BescheidFehler> {
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
    let jahr = i32::from(vz.jahr());
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
    Ok(Ok(Mitteilung {
        vz,
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
        let Ok(m) = lese(&inst, geburt, vz)? else {
            continue;
        };
        let (voll, ertragsanteil) = mitteilung_stpfl(&m, p)?;
        summe.voll = euro_plus(summe.voll, voll)?;
        summe.ertragsanteil = euro_plus(summe.ertragsanteil, ertragsanteil)?;
    }
    Ok(summe)
}
