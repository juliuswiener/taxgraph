//! `_an_gesamt_sperrgrund` (`bescheid_deklaration.py:784`): der K2-Guard. Nicht ring-faehige
//! Werbungskosten und Einkunftsarten sperren den Ring GANZ, nie Fake-0.
//!
//! Der Guard sieht bewusst die ROH-Felder (auch vorlaeufige) — jede Stelle prueft den Zustand selbst.
//! Die Reihenfolge der Pruefungen ist Teil des Verhaltens (die erste Sperre gewinnt) und folgt
//! Python Zeile fuer Zeile; die Aufteilung auf Untermodule folgt den Abschnitten dort.
//!
//! - [`werbungskosten`]: `_dhf_vpf_grund` (dHf, Verpflegung, Uebernachtung, Arbeitsmittel),
//! - [`einkunft`]: DBA, § 32b, § 16 Abs. 4, Kapital, Gewinn, Betrag-offen-Sperren, § 35,
//! - [`gesamt`]: der `gesamt_guard`-Zweig (Partner, Instanzen, Rente, Versorgung, § 33b, § 35a/c).

/// `if let Some(grund) = <ausdruck>? { return Ok(Some(grund)) }` — die erste Sperre gewinnt.
macro_rules! sperre {
    ($e:expr) => {
        if let Some(grund) = $e? {
            return Ok(Some(grund));
        }
    };
}

/// Wie `sperre!` fuer Pruefungen, die nicht fehlschlagen koennen (`Option` statt `Result`).
macro_rules! sperre_o {
    ($e:expr) => {
        if let Some(grund) = $e {
            return Ok(Some(grund));
        }
    };
}

mod einkunft;
mod gesamt;
mod werbungskosten;

use domain::{Sperrgrund, Vz, Zustand};
use konsistenz::{alleinerziehend_mit_zusammen, partner_ohne_zusammen};
use rust_decimal::Decimal;
use serde_json::Value;
use store::SnapshotFeld;

use super::konstanten::{AN_GESAMT_FLAGS, AN_GESAMT_PARTNER, VOR_FELDER, VOR_PARTNER_FELDER};
use super::{c2, Cfg};
use crate::abzuege::abs3_eligible;
use crate::{
    ist_false, ist_positive_zahl, ist_true, ist_zusammen, py_wahr, wert, zahl_dezimal,
    BescheidFehler, Felder, Instanzquelle,
};

/// Ergebnis der Guard-Funktionen: `None` = keine Sperre.
type Grund = Result<Option<Sperrgrund>, BescheidFehler>;

/// Die Eingaben des Guards, gebuendelt (Python: `felder, cfg, vz, store, bindung`).
struct K<'a> {
    f: &'a Felder,
    cfg: Option<&'a Cfg>,
    vz: Option<Vz>,
    q: &'a Instanzquelle<'a>,
}

/// K2-Guard: welcher Sperrgrund haengt an diesem Snapshot? `None` = der Ring darf rechnen.
///
/// `cfg`: die Scheibe (`None` = Alt-Aufrufer ohne Scheiben-Kontext). `vz`: `None` = Python `vz is
/// None` — die zwei § 34-Guards entfallen. `q.store`/`q.bindung` tragen die Instanz-Guards; `q.nur_bestaetigt`
/// wird nicht gelesen (der Guard sieht immer Roh-Felder).
///
/// PARITÄT: ein `vz` ausserhalb 2024–2026 ist nicht darstellbar ([`Vz`]); Python liefe mit dem
/// Rohwert weiter. Der Parity-Test ersetzt solche Faelle durch 2025 auf BEIDEN Seiten.
///
/// # Errors
/// Snapshot-, Bindungs- und Ueberlauf-Fehler; Python-Ausnahmen (z. B. `TypeError` bei `str + int` in
/// der Tage-Summe) als [`BescheidFehler::Python`].
///
/// ```
/// use bescheid::deklaration::an_gesamt_sperrgrund;
/// use bescheid::testhilfe::{felder, store};
/// use bescheid::Instanzquelle;
/// use domain::{Sperrgrund, Vz};
/// use serde_json::json;
/// let q = Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
/// let leer = felder(&store(&[]));
/// assert_eq!(an_gesamt_sperrgrund(&leer, None, Some(Vz::Vz2025), &q).unwrap(), None);
/// // Verpflegungstage ohne bestaetigten Monat am Ort: der Ring sperrt statt zu raten.
/// let f = felder(&store(&[("tage_24h", json!(5), true)]));
/// assert_eq!(
///     an_gesamt_sperrgrund(&f, None, Some(Vz::Vz2025), &q).unwrap(),
///     Some(Sperrgrund::VerpflegungDreimonatsfristAufteilungOffen)
/// );
/// ```
pub fn an_gesamt_sperrgrund(
    felder: &Felder,
    cfg: Option<&Cfg>,
    vz: Option<Vz>,
    q: &Instanzquelle<'_>,
) -> Grund {
    let k = K {
        f: felder,
        cfg,
        vz,
        q,
    };
    // Partner-Behinderungsfeld ohne Zusammenveranlagung; § 24b ↔ Zusammenveranlagung.
    if !partner_ohne_zusammen(felder).is_empty() {
        return Ok(Some(Sperrgrund::PartnerKonsistenzOffen));
    }
    if !alleinerziehend_mit_zusammen(felder).is_empty() {
        return Ok(Some(Sperrgrund::AlleinerziehendKonsistenzOffen));
    }
    sperre!(abs3_guards(&k));
    sperre_o!(an_gesamt_luecken(&k));
    if let Some(cfg) = cfg.filter(|c| c.gesamt_guard) {
        return gesamt::gesamt_guard(&k, cfg);
    }
    sperre!(werbungskosten::dhf_vpf_grund(&k));
    Ok(ohne_gesamt_guard(&k))
}

/// § 34 Abs. 3: Excess ueber 5 Mio und unbeantwortete Berufsunfaehigkeit.
fn abs3_guards(k: &K<'_>) -> Grund {
    let Some(vz) = k.vz else { return Ok(None) };
    if !ist_true(wert(k.f, "antrag_ermaessigter_satz")) {
        return Ok(None);
    }
    if abs3_eligible(k.f, vz)?
        && c2(k.f, "rentner_veraeusserungsgewinn")?.div_euclid(100) > 5_000_000
    {
        return Ok(Some(Sperrgrund::Abs3Ueber5mioOffen));
    }
    // `{**felder, "dauernd_berufsunfaehig": {"wert": True}}`: die Frage an `abs3_eligible` selbst.
    if !bestaetigt(k.f, "dauernd_berufsunfaehig") && !abs3_eligible(k.f, vz)? {
        let mut mit_bu = k.f.clone();
        mit_bu.insert(
            "dauernd_berufsunfaehig".to_owned(),
            SnapshotFeld {
                wert: Value::Bool(true),
                zustand: Zustand::Bestaetigt,
                herkunft: super::berechnet_herkunft()?,
            },
        );
        if abs3_eligible(&mit_bu, vz)? {
            return Ok(Some(Sperrgrund::BerufsunfaehigkeitOffen));
        }
    }
    Ok(None)
}

/// `an_gesamt` Gap-A/B und § 32b: Scheiben OHNE `gesamt_guard` rechnen diese Faelle nicht.
fn an_gesamt_luecken(k: &K<'_>) -> Option<Sperrgrund> {
    let cfg = k.cfg?;
    if cfg.gesamt_guard {
        return None;
    }
    if positiv(k.f, "fam_anzahl_kinder") {
        Some(Sperrgrund::KinderGehoerenInGesamt)
    } else if positiv(k.f, "verlustvortrag_bestand") {
        Some(Sperrgrund::VerlustvortragGehoertInGesamt)
    } else if positiv(k.f, "p32b_progressionseinkuenfte") {
        Some(Sperrgrund::ProgressionGehoertInGesamt)
    } else {
        None
    }
}

/// Der Rest fuer Scheiben ohne `gesamt_guard`: Zusammenveranlagung braucht Person B.
fn ohne_gesamt_guard(k: &K<'_>) -> Option<Sperrgrund> {
    if ist_zusammen(k.f) {
        if AN_GESAMT_PARTNER.iter().any(|pf| !bestaetigt(k.f, pf)) {
            return Some(Sperrgrund::PartnerKegelOffen);
        }
        if VOR_FELDER
            .iter()
            .chain(&VOR_PARTNER_FELDER)
            .any(|vf| positiv(k.f, vf))
        {
            return Some(Sperrgrund::PartnerVorOffen);
        }
    }
    AN_GESAMT_FLAGS
        .iter()
        .any(|fl| ist_false(wert(k.f, fl)))
        .then_some(Sperrgrund::EinkunftsartNichtRingFaehig)
}

// ---------------------------------------------------------------- Lesehilfen (Python-Idiome)

/// `_positiv(fid)`: Zahl (kein Bool) und `> 0`, auch vorlaeufig.
fn positiv(f: &Felder, fid: &str) -> bool {
    ist_positive_zahl(wert(f, fid))
}

/// `(felder.get(fid) or {}).get("zustand") == "bestaetigt"`.
fn bestaetigt(f: &Felder, fid: &str) -> bool {
    f.get(fid).is_some_and(|x| x.zustand == Zustand::Bestaetigt)
}

/// `isinstance(v, int)` — Pythons `bool` ist ein `int`, hier 0/1.
///
/// ponytail: ein `u64` jenseits von `i64` wird zu `i64::MAX` (nur Vergleiche gegen kleine Schwellen).
fn py_int_wert(v: Option<&Value>) -> Option<i64> {
    match v {
        Some(Value::Bool(b)) => Some(i64::from(*b)),
        other => ganzzahl(other),
    }
}

/// `isinstance(v, int) and not isinstance(v, bool)` (Saettigung wie [`py_int_wert`]).
fn ganzzahl(v: Option<&Value>) -> Option<i64> {
    match v {
        Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_u64().map(|_| i64::MAX)),
        _ => None,
    }
}

/// `isinstance(v, (int, float)) and not isinstance(v, bool)`.
fn zahl_wert(v: Option<&Value>) -> Option<Decimal> {
    match v {
        Some(Value::Number(n)) => Some(zahl_dezimal(n)),
        _ => None,
    }
}

/// `(felder.get(fid, {}).get("wert") or 0) > 0` — anders als [`positiv`] wirft ein Text `TypeError`.
///
/// PARITÄT: Python vergleicht `str > int` nicht; die Ausnahme steigt aus dem Guard auf.
fn oder_null_positiv(f: &Felder, fid: &str) -> Result<bool, BescheidFehler> {
    match wert(f, fid) {
        Some(Value::Number(_)) => Ok(ist_positive_zahl(wert(f, fid))),
        Some(Value::Bool(b)) => Ok(*b),
        Some(v) if py_wahr(v) => Err(BescheidFehler::Python {
            klasse: "TypeError",
            was: "'>' zwischen Text/Liste und int",
        }),
        _ => Ok(false),
    }
}
