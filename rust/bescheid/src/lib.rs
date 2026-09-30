//! `bescheid` — der Rechenring aus `produkt/bescheid/` (`REWRITE_PLAN.md` §7, Schritt 7).
//!
//! Schritt 7a: die zwei Blattmodule
//! - [`abzuege`] (`bescheid_abzuege.py`): Kinder, Sonderausgaben, aussergewoehnliche Belastungen,
//! - [`einkuenfte`] (`bescheid_einkuenfte.py`): Gewinn, Kapital, § 23, DBA, § 35.
//!
//! **Konventionen dieser Crate** (gelten fuer alle Module, auch die spaeteren Schritte):
//! - Python-`f: dict` (Feld-Snapshot `feld_id -> {wert, zustand, herkunft}`) ist [`Felder`].
//! - Python-`store`/`bindung`/`nur_bestaetigt` sind gebuendelt in [`Instanzquelle`]; die Kombination
//!   "Store da, Bindung fehlt" ist in Python ein `AttributeError` beim ersten Zugriff und hier
//!   [`BescheidFehler::BindungFehlt`].
//! - Rueckgabe-Einheit steht im Typ: [`Euro`] oder [`Cent`], wie Python sie liefert.
//! - Jedes Python-`_c(fid)` (`int(v) if Zahl else 0`) ist [`feld_int_oder_null`] bzw.
//!   [`feld_euro_oder_null`] (`PARITÄT: fail-open default`): der Name traegt den Vermerk an JEDER
//!   Aufrufstelle.
//! - Python rechnet mit unbeschraenkten `int`; hier ist jeder Zwischenschritt `checked_*` und ein
//!   Ueberlauf ein [`BescheidFehler::Ueberlauf`] (fail-closed statt Wrap-Around).
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic
    )
)]

pub mod abzuege;
pub mod deklaration;
pub mod einkuenfte;
pub mod zweige;

use domain::{Cent, Euro, Zustand};
use elster::Instanz;
use engine::zugriff::teil1::fehler::EngineFehler as EngineFehler1;
use engine::zugriff::teil2::EngineFehler as EngineFehler2;
use rust_decimal::Decimal;
use serde_json::{Number, Value};
use store::{SnapshotFehler, SnapshotFeld, Store};

/// Materialisierter Snapshot `feld_id -> Feld` (Python `f`/`f_dict`/`felder`).
pub type Felder = elster::Felder;
/// Bindungstabelle `feld_id -> Bindung` (Python `bindung: dict`).
pub type BindungIndex<'a> = elster::BindungIndex<'a>;

/// Fehler der Ring-Funktionen. Jede Variante entspricht einer Python-Ausnahme an derselben Stelle
/// ([`BescheidFehler::python_klasse`]); `None` dort heisst: Python haette weitergerechnet.
#[derive(Debug, thiserror::Error)]
pub enum BescheidFehler {
    /// Accessor (Teil 1): Catala-Laufzeitfehler, Parameter, Ueberlauf.
    #[error(transparent)]
    Engine(#[from] EngineFehler1),
    /// Accessor (Teil 2).
    #[error(transparent)]
    EngineTeil2(#[from] EngineFehler2),
    /// `est_mapping.instanzen` → `store.materialisiere`.
    #[error(transparent)]
    Snapshot(#[from] SnapshotFehler),
    /// Python: `bindung.items()` auf `None` → `AttributeError`, sobald ein Store vorliegt.
    #[error("Store vorhanden, Bindung fehlt (Python: AttributeError auf None)")]
    BindungFehlt,
    /// Python: `slots[<schluessel>]` → `KeyError`.
    #[error("Slot {0} fehlt (Python: KeyError)")]
    SlotFehlt(String),
    /// Python: `int()`/`.strip()` auf einem Wert des falschen Typs.
    #[error("Python {klasse}: {was}")]
    Python {
        klasse: &'static str,
        was: &'static str,
    },
    /// `i64`-Ueberlauf (Python rechnet unbeschraenkt).
    #[error("i64-Ueberlauf in {0}")]
    Ueberlauf(&'static str),
}

impl BescheidFehler {
    /// Name der Python-Ausnahmeklasse an derselben Stelle; `None`: Python kennt hier keinen Fehler
    /// (Ueberlauf) oder der Engine-Fehler hat kein Python-Gegenstueck.
    ///
    /// ```
    /// use bescheid::BescheidFehler;
    /// assert_eq!(BescheidFehler::BindungFehlt.python_klasse(), Some("AttributeError"));
    /// assert_eq!(BescheidFehler::Ueberlauf("Addition").python_klasse(), None);
    /// ```
    #[must_use]
    pub fn python_klasse(&self) -> Option<&'static str> {
        match self {
            Self::Engine(e) => e.python_typ(),
            Self::EngineTeil2(_) => Some("CatalaError"), // PARITÄT: Klasse gruppiert, wie im Accessor-Test
            Self::Snapshot(_) => Some("ValueError"),
            Self::BindungFehlt => Some("AttributeError"),
            Self::SlotFehlt(_) => Some("KeyError"),
            Self::Python { klasse, .. } => Some(klasse),
            Self::Ueberlauf(_) => None,
        }
    }
}

/// Die drei Python-Parameter `store`, `bindung`, `nur_bestaetigt` der Instanz-Summen.
///
/// `nur_bestaetigt` ist die Sicherheits-Invariante des Rings: bei `true` bewegt eine vorlaeufige
/// Instanz die festgesetzte Steuer nie (`bescheid_zweige.py`, Zwei-Signal-Invariant).
#[derive(Clone, Copy)]
pub struct Instanzquelle<'a> {
    pub store: Option<&'a Store>,
    pub bindung: Option<&'a BindungIndex<'a>>,
    pub nur_bestaetigt: bool,
}

impl<'a> Instanzquelle<'a> {
    /// Beide Quellen da? Die Python-Bedingung `store is not None and bindung is not None`.
    ///
    /// ```
    /// use bescheid::Instanzquelle;
    /// let q = Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
    /// assert!(q.beide().is_none());
    /// ```
    #[must_use]
    pub fn beide(&self) -> Option<(&'a Store, &'a BindungIndex<'a>)> {
        self.store.zip(self.bindung)
    }

    /// `est_mapping.instanzen(store, bindung, gruppe)` mit der Python-Vorbedingung `store is None →
    /// leere Summe` (die Aufrufer geben dann 0/[] zurueck). Store ohne Bindung ist ein Fehler.
    ///
    /// # Errors
    /// [`BescheidFehler::BindungFehlt`], [`BescheidFehler::Snapshot`].
    ///
    /// ```
    /// use bescheid::{BescheidFehler, Instanzquelle};
    /// let ohne_store = Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
    /// assert!(ohne_store.instanzen("kind").unwrap().is_empty());
    /// let st = bescheid::testhilfe::store(&[("kind_kv", serde_json::json!(1), true)]);
    /// let ohne_bindung = Instanzquelle { store: Some(&st), bindung: None, nur_bestaetigt: true };
    /// assert!(matches!(ohne_bindung.instanzen("kind"), Err(BescheidFehler::BindungFehlt)));
    /// ```
    pub fn instanzen(&self, gruppe: &str) -> Result<Vec<Instanz>, BescheidFehler> {
        match (self.store, self.bindung) {
            (None, _) => Ok(Vec::new()),
            (Some(_), None) => Err(BescheidFehler::BindungFehlt),
            (Some(s), Some(b)) => {
                let inst = elster::instanzen(s, b, gruppe)?;
                // Invariante der Instanz-Naht: index-sortiert, je Index genau eine Instanz.
                debug_assert!(inst.is_sorted_by(|a, b| a.index < b.index));
                Ok(inst)
            }
        }
    }

    /// Python `not nur_bestaetigt or inst["zustand"] == "bestaetigt"`.
    ///
    /// ```
    /// use bescheid::testhilfe::{index, store};
    /// use bescheid::Instanzquelle;
    /// use serde_json::json;
    /// let st = store(&[("kind_kv", json!(1), false)]);
    /// let streng = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
    /// let offen = Instanzquelle { nur_bestaetigt: false, ..streng };
    /// let inst = streng.instanzen("kind").unwrap();
    /// assert!(!streng.zaehlt(&inst[0])); // vorläufig: bewegt die festgesetzte Steuer nie
    /// assert!(offen.zaehlt(&inst[0]));
    /// ```
    #[must_use]
    pub fn zaehlt(&self, inst: &Instanz) -> bool {
        // Invariante der meet-Regel: eine bestaetigte Instanz hat nur bestaetigte Felder.
        debug_assert!(
            inst.zustand != Zustand::Bestaetigt
                || inst
                    .felder
                    .values()
                    .all(|f| f.zustand == Zustand::Bestaetigt)
        );
        !self.nur_bestaetigt || inst.zustand == Zustand::Bestaetigt
    }
}

/// `int(v)` fuer eine JSON-Zahl (Python schneidet Floats Richtung 0 ab).
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`] jenseits von `i64` (Python: unbeschraenkte Ganzzahl).
#[allow(clippy::cast_possible_truncation)] // Bereich vorher geprueft
pub(crate) fn zahl_int(n: &Number) -> Result<i64, BescheidFehler> {
    if let Some(i) = n.as_i64() {
        return Ok(i);
    }
    if n.as_u64().is_some() {
        return Err(BescheidFehler::Ueberlauf("int(zahl)"));
    }
    match n.as_f64() {
        Some(x) if x.is_finite() && x.trunc().abs() < 9.223_372_036_854_776e18 => {
            Ok(x.trunc() as i64)
        }
        _ => Err(BescheidFehler::Ueberlauf("int(float)")),
    }
}

/// JSON-Zahl → exakte `Decimal` (Python `int`/`float` als Geld-Wert, kein `f64` mehr im Rechenweg).
///
/// PARITÄT: ein Float wird mit seinem binaeren Wert uebernommen (`from_f64_retain`), nicht ueber
/// seine Kurzschreibweise; jenseits des `Decimal`-Bereichs (~7,9e28) saturiert der Wert mit
/// Vorzeichen — alle Aufrufer fragen nur Vorzeichen und Schwellen.
pub(crate) fn zahl_dezimal(n: &Number) -> Decimal {
    if let Some(i) = n.as_i64() {
        return Decimal::from(i);
    }
    if let Some(u) = n.as_u64() {
        return Decimal::from(u);
    }
    n.as_f64().map_or(Decimal::ZERO, |x| {
        Decimal::from_f64_retain(x).unwrap_or(if x.is_sign_negative() {
            Decimal::MIN
        } else {
            Decimal::MAX
        })
    })
}

/// Summe zweier `Decimal`-Werte; bei Ueberlauf saturiert sie Richtung Vorzeichen von `b`.
pub(crate) fn dezimal_plus(a: Decimal, b: Decimal) -> Decimal {
    a.checked_add(b).unwrap_or(if b.is_sign_negative() {
        Decimal::MIN
    } else {
        Decimal::MAX
    })
}

/// `f.get(fid, {}).get("wert")`.
pub(crate) fn wert<'a>(f: &'a Felder, fid: &str) -> Option<&'a Value> {
    f.get(fid).map(|x: &SnapshotFeld| &x.wert)
}

/// Python `_c(fid)`: `int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0`.
///
/// PARITÄT: fail-open default — ein fehlendes oder nicht-numerisches Feld ist 0, kein Fehler.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`] bei Werten jenseits von `i64`.
///
/// ```
/// use bescheid::{feld_int_oder_null, testhilfe::{felder, store}};
/// use serde_json::json;
/// let f = felder(&store(&[("a", json!(7), true), ("b", json!("text"), true)]));
/// assert_eq!(feld_int_oder_null(&f, "a").unwrap(), 7);
/// assert_eq!(feld_int_oder_null(&f, "b").unwrap(), 0); // nicht numerisch: 0 (PARITÄT: fail-open default)
/// assert_eq!(feld_int_oder_null(&f, "fehlt").unwrap(), 0);
/// ```
pub fn feld_int_oder_null(f: &Felder, fid: &str) -> Result<i64, BescheidFehler> {
    zahl_oder_null(wert(f, fid))
}

/// Wie [`feld_int_oder_null`] fuer einen schon gelesenen Wert.
pub(crate) fn zahl_oder_null(v: Option<&Value>) -> Result<i64, BescheidFehler> {
    match v {
        Some(Value::Number(n)) => zahl_int(n),
        _ => Ok(0),
    }
}

/// `isinstance(v, (int, float)) and not isinstance(v, bool) and v > 0` (ohne `int()`).
pub(crate) fn ist_positive_zahl(v: Option<&Value>) -> bool {
    let Some(Value::Number(n)) = v else {
        return false;
    };
    n.as_i64().map_or_else(
        || n.as_u64().is_some() || n.as_f64().is_some_and(|x| x > 0.0),
        |i| i > 0,
    )
}

/// Wie [`ist_positive_zahl`], liefert bei `true` `int(v)`.
pub(crate) fn positive_zahl(v: Option<&Value>) -> Result<Option<i64>, BescheidFehler> {
    match v {
        Some(Value::Number(n)) if ist_positive_zahl(v) => zahl_int(n).map(Some),
        _ => Ok(None),
    }
}

/// Python `wert is True`.
pub(crate) fn ist_true(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Bool(true)))
}

/// Python `wert is False`.
pub(crate) fn ist_false(v: Option<&Value>) -> bool {
    matches!(v, Some(Value::Bool(false)))
}

/// Python `wert == "zusammen"` (`veranlagung`).
pub(crate) fn ist_zusammen(f: &Felder) -> bool {
    matches!(wert(f, "veranlagung"), Some(Value::String(s)) if s == "zusammen")
}

/// Python-Wahrheitswert eines JSON-Werts (`not x`).
pub(crate) fn py_wahr(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// Pythons `int(x)` fuer einen Slot-Wert: Bool, Zahl, Text mit Vorzeichen/Ziffern/`_`.
///
/// ponytail: nur ASCII-Ziffern; Python akzeptiert auch andere Unicode-Dezimalziffern. Slot-Werte
/// sind durchweg Ganzzahlen (`intervall::SlotFehler`, gemessen 22 von 22).
///
/// # Errors
/// [`BescheidFehler::Python`] (`TypeError`/`ValueError`), [`BescheidFehler::Ueberlauf`].
pub(crate) fn py_int(v: &Value) -> Result<i64, BescheidFehler> {
    match v {
        Value::Bool(b) => Ok(i64::from(*b)),
        Value::Number(n) => zahl_int(n),
        Value::String(s) => {
            let s = s.trim_matches(py_leerraum);
            let rumpf = s.strip_prefix(['+', '-']).unwrap_or(s);
            let form_ok = !rumpf.is_empty()
                && !rumpf.starts_with('_')
                && !rumpf.ends_with('_')
                && !rumpf.contains("__")
                && rumpf.bytes().all(|b| b.is_ascii_digit() || b == b'_');
            if !form_ok {
                return Err(BescheidFehler::Python {
                    klasse: "ValueError",
                    was: "int(text)",
                });
            }
            s.replace('_', "")
                .parse()
                .map_err(|_| BescheidFehler::Ueberlauf("int(text)"))
        }
        _ => Err(BescheidFehler::Python {
            klasse: "TypeError",
            was: "int(kein Zahlwert)",
        }),
    }
}

/// `str.isspace()` je Zeichen (Rusts `is_whitespace` kennt `\x1c`..`\x1f` nicht, Python schon).
pub(crate) fn py_leerraum(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `a + b` mit Ueberlauf-Fehler.
pub(crate) fn plus(a: i64, b: i64) -> Result<i64, BescheidFehler> {
    a.checked_add(b)
        .ok_or(BescheidFehler::Ueberlauf("Addition"))
}

/// `a - b` mit Ueberlauf-Fehler.
pub(crate) fn minus(a: i64, b: i64) -> Result<i64, BescheidFehler> {
    a.checked_sub(b)
        .ok_or(BescheidFehler::Ueberlauf("Subtraktion"))
}

/// Summe mehrerer Cent-/Euro-Werte, links nach rechts.
pub(crate) fn summe(werte: &[i64]) -> Result<i64, BescheidFehler> {
    werte.iter().try_fold(0_i64, |acc, x| plus(acc, *x))
}

/// Python `cent // 100` (rundet gegen −∞).
pub(crate) fn cent_zu_euro(cent: i64) -> Euro {
    Cent::new(cent).floor_euro()
}

/// `Euro + Euro`.
pub(crate) fn euro_plus(a: Euro, b: Euro) -> Result<Euro, BescheidFehler> {
    plus(a.get(), b.get()).map(Euro::new)
}

/// Python-`_c(fid) // 100` in einem Zug: Cent-Feld → Euro.
pub(crate) fn feld_euro_oder_null(f: &Felder, fid: &str) -> Result<Euro, BescheidFehler> {
    Ok(cent_zu_euro(feld_int_oder_null(f, fid)?))
}

/// Gemeinsame Test- und Doctest-Hilfen: echte Params und Bindung, Stores aus Wert-Listen.
///
/// `pub` und versteckt, damit die Doctests der oeffentlichen Funktionen sie nutzen koennen (ein
/// Doctest ist eine eigene Crate und sieht kein `cfg(test)`). Nicht Teil der stabilen API.
#[doc(hidden)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::missing_panics_doc,
    clippy::must_use_candidate
)]
pub mod testhilfe {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::OnceLock;

    use bindung::{Bindung, Params};
    use serde_json::{json, Value};
    use store::{EventId, Store, StoreDatei};

    use crate::{BindungIndex, Felder};

    fn repo() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// Die echten Jahresparameter aus `params/` (einmal geladen).
    ///
    /// ```
    /// let p = bescheid::testhilfe::params();
    /// assert!(p.grundfreibetrag(domain::Vz::Vz2025).is_ok());
    /// ```
    pub fn params() -> &'static Params {
        static P: OnceLock<Params> = OnceLock::new();
        P.get_or_init(|| Params::lade(&repo()).unwrap())
    }

    fn bindungen() -> &'static [Bindung] {
        static B: OnceLock<Vec<Bindung>> = OnceLock::new();
        B.get_or_init(|| {
            let reg = bindung::lade_registry(&repo().join("produkt/bindung")).unwrap();
            reg.dateien
                .into_iter()
                .flat_map(|(_, d)| d.bindungen)
                .collect()
        })
    }

    /// Der Bindungs-Index ueber alle Laufzeit-Bindungen.
    ///
    /// ```
    /// assert!(bescheid::testhilfe::index().contains_key("geburtsjahr"));
    /// ```
    pub fn index() -> &'static BindungIndex<'static> {
        static I: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
        I.get_or_init(|| store::baue_nachschlag(bindungen()))
    }

    /// Store aus `(feld_id, wert, bestaetigt)`; `event_id` ist der echte Hash des Inhalts.
    ///
    /// ```
    /// let st = bescheid::testhilfe::store(&[("geburtsjahr", serde_json::json!(1960), true)]);
    /// assert_eq!(bescheid::testhilfe::felder(&st).len(), 1);
    /// ```
    pub fn store(events: &[(&str, Value, bool)]) -> Store {
        let events: Vec<Value> = events
            .iter()
            .enumerate()
            .map(|(i, (fid, wert, bestaetigt))| {
                let mut e = json!({
                    "ts": format!("2026-01-01T00:00:{i:02}+00:00"), "feld_id": fid, "wert": wert,
                    "zustand": if *bestaetigt { "bestaetigt" } else { "vorlaeufig" },
                    "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    "schreiber": "ui:laie",
                    "signal": {"signal_1": null, "signal_2": if *bestaetigt { json!("ok") } else { Value::Null }},
                    "ersetzt": null,
                });
                e["event_id"] = json!(EventId::von_json(&e).to_string());
                e
            })
            .collect();
        let datei: StoreDatei = serde_json::from_value(
            json!({"version": 1, "veranlagungszeitraum": 2025, "events": events}),
        )
        .unwrap();
        Store::aus_datei(datei)
    }

    /// Leerer Gesamtfall (alle Betraege 0) fuer Doctests der Zweig-Hilfen.
    ///
    /// ```
    /// let g = bescheid::testhilfe::leerer_gesamtfall(domain::Vz::Vz2025, true);
    /// assert!(g.zusammenveranlagung && g.einkuenfte_gewinn.get() == 0);
    /// ```
    pub fn leerer_gesamtfall(
        vz: domain::Vz,
        zusammen: bool,
    ) -> engine::zugriff::teil2::gesamt::GesamtfallEingabe {
        crate::zweige::leerer_gesamtfall(vz, zusammen)
    }

    /// Der materialisierte Snapshot eines Stores.
    ///
    /// ```
    /// let f = bescheid::testhilfe::felder(&bescheid::testhilfe::store(&[("a", serde_json::json!(1), false)]));
    /// assert!(f.contains_key("a"));
    /// ```
    pub fn felder(store: &Store) -> Felder {
        store.materialisiere(None).unwrap().0
    }
}
