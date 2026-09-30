//! `_dhf_vpf_grund` (`bescheid_deklaration.py`, geschachtelt in `_an_gesamt_sperrgrund`): dHf,
//! Verpflegung, Uebernachtung und Arbeitsmittel (§ 9 Abs. 1 Nr. 5/5a/6/7, Abs. 4a). Gilt fuer JEDE
//! Scheibe, die diese Felder ring-verdrahtet.
use domain::Sperrgrund;
use serde_json::Value;

use super::{bestaetigt, ganzzahl, positiv, zahl_f64, Grund, K};
use crate::deklaration::konstanten::{
    ARBEITSMITTEL_KOSTEN, DHF_BEDINGUNGEN, DHF_KOSTEN, UEBERNACHTUNG_BEDINGUNGEN,
    UEBERNACHTUNG_KOSTEN, VERPFLEGUNG_TAGE, VERPFLEGUNG_TAGE_NACH_FRIST,
};
use crate::{ist_false, ist_true, wert, BescheidFehler, Felder};

/// Die vier Tatbestaende in Python-Reihenfolge; die erste Sperre gewinnt.
pub(super) fn dhf_vpf_grund(k: &K<'_>) -> Grund {
    sperre_o!(dhf(k.f));
    sperre!(verpflegung(k.f));
    sperre_o!(uebernachtung(k.f));
    Ok(arbeitsmittel(k.f))
}

/// Ausland-dHf → nicht ring-faehig; offene Geltungsbedingung → offen.
fn dhf(f: &Felder) -> Option<Sperrgrund> {
    if !positiv(f, DHF_KOSTEN) {
        return None;
    }
    let inland = wert(f, "dhf_im_inland");
    if ist_false(inland) {
        return Some(Sperrgrund::AuslandDhfNichtRingFaehig);
    }
    // Naht-Fix (gate-naht-guard-liest-zustand): nicht nur "is True" auf dem Rohwert.
    if !ist_true(inland) || !bestaetigt(f, "dhf_im_inland") {
        return Some(Sperrgrund::DhfTatbestandOffen);
    }
    if DHF_BEDINGUNGEN.iter().any(|b| !bestaetigt(f, b)) {
        return Some(Sperrgrund::DhfTatbestandOffen);
    }
    None
}

/// Python `sum((wert or 0) for ...)`: Ganzzahlen exakt, Floats getrennt (nur `> 0` und `== 0` werden gefragt).
#[derive(Default)]
struct PySumme {
    ganz: i128,
    komma: f64,
    ist_float: bool,
}

impl PySumme {
    /// `+ (wert or 0)`.
    ///
    /// PARITÄT: ein nicht-leerer Text oder eine nicht-leere Liste ist `0 + "12"` = `TypeError` —
    /// die Ausnahme steigt aus dem Guard auf (HTTP 500 in Python).
    fn addiere(&mut self, v: Option<&Value>) -> Result<(), BescheidFehler> {
        match v {
            Some(Value::Bool(true)) => self.ganz += 1,
            Some(Value::Number(n)) => {
                if let Some(i) = n.as_i64() {
                    self.ganz += i128::from(i);
                } else if let Some(u) = n.as_u64() {
                    self.ganz += i128::from(u);
                } else if let Some(x) = n.as_f64() {
                    self.komma += x;
                    self.ist_float = true;
                }
            }
            Some(Value::String(s)) if !s.is_empty() => return Err(typfehler()),
            Some(Value::Array(a)) if !a.is_empty() => return Err(typfehler()),
            Some(Value::Object(o)) if !o.is_empty() => return Err(typfehler()),
            _ => {}
        }
        Ok(())
    }

    #[allow(clippy::cast_precision_loss)] // Vergleich gegen 0, Groessenordnung genuegt
    fn gesamt(&self) -> f64 {
        if self.ist_float {
            self.ganz as f64 + self.komma
        } else {
            self.ganz as f64
        }
    }

    fn positiv(&self) -> bool {
        self.gesamt() > 0.0
    }

    fn null(&self) -> bool {
        self.gesamt().abs() < f64::EPSILON
    }
}

fn typfehler() -> BescheidFehler {
    BescheidFehler::Python {
        klasse: "TypeError",
        was: "unsupported operand type(s) for +: 'int' and 'str'",
    }
}

/// `sum(wert or 0 for f in ids if nur(f))`.
fn summe(f: &Felder, ids: &[&str], nur_bestaetigt: bool) -> Result<PySumme, BescheidFehler> {
    let mut s = PySumme::default();
    for id in ids {
        if !nur_bestaetigt || bestaetigt(f, id) {
            s.addiere(wert(f, id))?;
        }
    }
    Ok(s)
}

/// § 9 Abs. 4a: 3-Monats-Frist, Mahlzeitenkuerzung. Tage > 0 verlangen bestaetigte Begleitangaben.
fn verpflegung(f: &Felder) -> Grund {
    if !summe(f, &VERPFLEGUNG_TAGE, false)?.positiv() {
        return Ok(None);
    }
    // Ohne bestaetigtes vpf_monate_am_ort greift die Aufteilung (S. 6) nicht — fail-closed.
    let monate = ganzzahl(wert(f, "vpf_monate_am_ort"));
    if monate.is_none() || !bestaetigt(f, "vpf_monate_am_ort") {
        return Ok(Some(Sperrgrund::VerpflegungDreimonatsfristAufteilungOffen));
    }
    if monate.is_some_and(|m| m > 3) {
        sperre!(nach_frist(f));
    }
    // Mahlzeitenfrage: Anzahl-Felder (neu) oder vpf_keine_mahlzeitengestellung = true (alt).
    let zahlen_bestaetigt = [
        "vpf_fruehstuecke_gestellt_anzahl",
        "vpf_mittagessen_gestellt_anzahl",
        "vpf_abendessen_gestellt_anzahl",
    ]
    .iter()
    .any(|z| bestaetigt(f, z));
    let keine_mahlzeiten = bestaetigt(f, "vpf_keine_mahlzeitengestellung")
        && ist_true(wert(f, "vpf_keine_mahlzeitengestellung"));
    Ok(if zahlen_bestaetigt || keine_mahlzeiten {
        None
    } else {
        Some(Sperrgrund::VerpflegungReduktionOffen)
    })
}

/// Mehr als 3 Monate am Ort: je Kategorie mit Tagen > 0 ist die Nach-Frist-Angabe Pflicht; alle
/// Nach-Frist-Angaben = 0 verlangen die Antwort zur Unterbrechung.
fn nach_frist(f: &Felder) -> Grund {
    for (tage, nach) in VERPFLEGUNG_TAGE.iter().zip(&VERPFLEGUNG_TAGE_NACH_FRIST) {
        if positiv(f, tage) && !bestaetigt(f, nach) {
            return Ok(Some(Sperrgrund::VerpflegungDreimonatsfristAufteilungOffen));
        }
    }
    // Nur bestaetigte Tage zaehlen; beide Summen vor dem Vergleich (beide koennen TypeError werfen).
    let tage_gesamt = summe(f, &VERPFLEGUNG_TAGE, true)?;
    let nach_gesamt = summe(f, &VERPFLEGUNG_TAGE_NACH_FRIST, true)?;
    // Nur der ZUSTAND der Antwort zaehlt, nicht ihr Wert.
    Ok(
        if tage_gesamt.positiv()
            && nach_gesamt.null()
            && !bestaetigt(f, "vpf_frist_nicht_unterbrochen")
        {
            Some(Sperrgrund::VerpflegungDreimonatsfristUnterbrechungOffen)
        } else {
            None
        },
    )
}

/// § 9 Abs. 1 Nr. 5a: Tatbestand (Inland/Ausland als Ortsangabe, drei Bedingungen) und Zeitraum
/// muessen BESTAETIGT sein. Ausland und ueberspannender Zeitraum sperren NICHT (Ring rechnet beides).
fn uebernachtung(f: &Felder) -> Option<Sperrgrund> {
    if !positiv(f, UEBERNACHTUNG_KOSTEN) {
        return None;
    }
    let ort_ist_bool = matches!(wert(f, "uebernachtung_im_inland"), Some(Value::Bool(_)));
    if !ort_ist_bool
        || !bestaetigt(f, "uebernachtung_im_inland")
        || UEBERNACHTUNG_BEDINGUNGEN.iter().any(|b| !bestaetigt(f, b))
    {
        return Some(Sperrgrund::UebernachtungTatbestandOffen);
    }
    // bisher/monate: bestaetigte int, sonst rechnet der Ring (bestaetigt-only) mit dem Default 0.
    let ganz_und_bestaetigt = |fid: &str| ganzzahl(wert(f, fid)).is_some() && bestaetigt(f, fid);
    if ganz_und_bestaetigt("uebernachtung_monate_bisher")
        && ganz_und_bestaetigt("uebernachtung_monate")
    {
        None
    } else {
        Some(Sperrgrund::UebernachtungZeitraumOffen)
    }
}

/// § 9 Abs. 1 Nr. 6/7 (GWG-Sofortabzug ≤ 800 EUR, sonst § 7-AfA). Schwelle in CENT (80.000).
fn arbeitsmittel(f: &Felder) -> Option<Sperrgrund> {
    let am = zahl_f64(wert(f, ARBEITSMITTEL_KOSTEN)).filter(|a| *a > 0.0)?;
    if am <= 80_000.0 {
        // "is not True" auf dem Rohwert liess ein vorlaeufiges True durch (Naht-Fix).
        let gewaehlt = ist_true(wert(f, "am_gwg_sofortabzug_gewaehlt"))
            && bestaetigt(f, "am_gwg_sofortabzug_gewaehlt");
        return (!gewaehlt).then_some(Sperrgrund::ArbeitsmittelAfaUeberGwgOffen);
    }
    // > 800 EUR: § 7 Abs. 1 lineare AfA — die Nutzungsdauer MUSS beantwortet sein.
    if ganzzahl(wert(f, "arbeitsmittel_nutzungsdauer")).is_none_or(|n| n <= 0) {
        return Some(Sperrgrund::ArbeitsmittelAfaUeberGwgOffen);
    }
    // Anschaffungsjahr = true: der Monat MUSS beantwortet sein. False/unbeantwortet = Folgejahr.
    if ist_true(wert(f, "am_afa_ist_anschaffungsjahr")) {
        let monat_ok =
            ganzzahl(wert(f, "am_anschaffung_monat")).is_some_and(|m| (1..=12).contains(&m));
        if !bestaetigt(f, "am_afa_ist_anschaffungsjahr") || !monat_ok {
            return Some(Sperrgrund::ArbeitsmittelAfaUeberGwgOffen);
        }
    }
    None
}
