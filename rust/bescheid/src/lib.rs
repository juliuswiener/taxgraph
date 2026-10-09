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
// Tor 2b (REWRITE_PLAN §9): kein Float im Rechenpfad. `clippy.toml` sperrt die Typen `f64`/`f32`,
// dies hier die Rechnung mit abgeleitetem Typ (`d.to_f64()? * x`, Literale). Testcode ist ausgenommen.
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::disallowed_types
    )
)]

pub mod abzuege;
pub mod deklaration;
pub mod einkuenfte;
pub mod zweige;

use domain::{Cent, Euro, Lage, PyWert, Sperrgrund, Veranlagung, Zustand};
use elster::Instanz;
use engine::zugriff::teil1::fehler::EngineFehler as EngineFehler1;
use engine::zugriff::teil2::EngineFehler as EngineFehler2;
use rust_decimal::Decimal;
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
    /// Der Kinderfreibetrag je Kind ist nicht zu bestimmen ([`Sperrgrund::KindFreibetragVerteilungOffen`],
    /// [`Sperrgrund::KindZeitraumUnlesbar`]). Der K2-Guard faengt beide vorher ab; kommt der Fehler an, hat ein
    /// Aufrufer ohne Guard gerechnet. Rust-eigen, Python kennt ihn nicht.
    #[error("Kinderfreibetrag je Kind nicht bestimmbar ({0}); der Guard haette sperren muessen")]
    KindFreibetragGesperrt(Sperrgrund),
    /// Der Versorgungsbezug passt nicht zum Bruttoarbeitslohn ([`Sperrgrund::VersorgungUeberLohn`], Abweichung Nr. 50). Der
    /// K2-Guard faengt das vorher ab; kommt der Fehler an, hat ein Aufrufer ohne Guard gerechnet. Rust-eigen.
    #[error("Versorgungsbezug und Bruttoarbeitslohn passen nicht zusammen ({0}); der Guard haette sperren muessen")]
    VersorgungGesperrt(Sperrgrund),
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
            Self::Ueberlauf(_) | Self::KindFreibetragGesperrt(_) | Self::VersorgungGesperrt(_) => None,
        }
    }

    /// `true`, wenn die Rechnung an der `i64`-Grenze scheitert: Python rechnet dort mit unbeschraenkten
    /// `int` weiter und haette eine Zahl geliefert. Das ist ein Fehler der Eingabe (ein Betrag, den die
    /// Rechnung nicht fasst), kein Programmfehler. Gilt fuer [`BescheidFehler::Ueberlauf`] und den
    /// `Ueberlauf` der Teil-1- und Teil-2-Accessoren.
    ///
    /// ```
    /// use bescheid::BescheidFehler;
    /// assert!(BescheidFehler::Ueberlauf("Addition").ist_ueberlauf());
    /// assert!(!BescheidFehler::BindungFehlt.ist_ueberlauf());
    /// ```
    #[must_use]
    pub fn ist_ueberlauf(&self) -> bool {
        matches!(
            self,
            Self::Ueberlauf(_)
                | Self::Engine(EngineFehler1::Ueberlauf(_))
                | Self::EngineTeil2(EngineFehler2::Basis(EngineFehler1::Ueberlauf(_)))
        )
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

/// `domain::PyFehler` → [`BescheidFehler`]: die Python-Klasse bleibt das Kriterium.
///
/// ponytail: `was` ist `&'static str` (wie an jeder bisherigen Konstruktionsstelle), traegt also den
/// Operationsnamen statt Pythons Nachricht. Paritaetskriterium ist die Klasse allein
/// ([`BescheidFehler::python_klasse`]). `I64Grenze`/`DezimalGrenze` haben keine Python-Klasse —
/// `CPython` rechnet dort exakt weiter (D3/D18) — und werden wie bisher
/// [`BescheidFehler::Ueberlauf`] (`python_klasse() == None`).
pub(crate) fn py_klasse(e: &domain::PyFehler, was: &'static str) -> BescheidFehler {
    match e.python_klasse() {
        Some(klasse) => BescheidFehler::Python { klasse, was },
        None => BescheidFehler::Ueberlauf(was),
    }
}

/// `int(v)` fuer einen Store-Wert (Python schneidet Floats Richtung 0 ab).
///
/// # Errors
/// Wie `CPython`; [`BescheidFehler::Ueberlauf`] jenseits von `i64` (Python: unbeschraenkte
/// Ganzzahl).
pub(crate) fn zahl_int(w: &PyWert) -> Result<i64, BescheidFehler> {
    w.int().map_err(|e| py_klasse(&e, "int(zahl)"))
}

/// Store-Wert → exakte `Decimal` (Python `int`/`float` als Geld-Wert, kein `f64` mehr im
/// Rechenweg).
///
/// PARITÄT: ein Float wird mit seinem binaeren Wert uebernommen (`from_f64_retain`), nicht ueber
/// seine Kurzschreibweise; jenseits des `Decimal`-Bereichs (~7,9e28) saturiert der Wert mit
/// Vorzeichen. Die Aufrufer in `deklaration::sperre` fragen nur Vorzeichen und Schwellen (auch
/// einer Summe); `rentenfreibetrag_euro` rechnet daraus einen Betrag (`rf // 100`). Alles, was
/// keine Zahl ist, zaehlt 0 (die Aufrufer fragen ueber `zahl_ohne_bool`).
///
/// PARITÄT NaN: Python vergleicht NaN immer `False`; hier saturiert NaN nach seinem Vorzeichenbit
/// auf `Decimal::MAX`/`MIN` und besteht damit `> 0` bzw. `<= 0`. Als Betrag wird es ±9e18 Euro,
/// Python wirft `ValueError` (`int(nan)`). Aus dem Store heute unerreichbar: Typ `cent`/`int`
/// laesst keinen Float zu, und `lade` liest NaN als Text (B4, Vault:
/// `tickets/falldatei-mit-nan-liest-rust-als-text`). Liest `lade` NaN kuenftig als Float, gilt
/// das Upgrade unten.
///
/// ponytail: die Saettigung ist die Vor-K2-Fassung und bleibt (D18): `PyWert::dezimal` meldet dort
/// `DezimalGrenze`, `CPython` rechnet exakt weiter. Ein Geldpfad dieses Crates kann die Grenze
/// nicht beruehren (Typ `cent`/`int`). Upgrade: `Result<Decimal, _>` bis in die Aufrufer, falls je
/// ein Pfad entsteht, der sie rechnerisch treffen kann.
pub(crate) fn zahl_dezimal(w: &PyWert) -> Decimal {
    match w {
        PyWert::Ganz(n) => Decimal::from(*n),
        PyWert::GrossGanz(u) => Decimal::from(*u),
        PyWert::Gleit(f) => Decimal::from_f64_retain(*f).unwrap_or(if f.is_sign_negative() {
            Decimal::MIN
        } else {
            Decimal::MAX
        }),
        _ => Decimal::ZERO,
    }
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
///
/// K2: der Store traegt [`PyWert`] (`store::SnapshotFeld::wert`), also liest auch der Ring ihn.
/// Der frueher hier stehende `Option<&Value>` war die letzte `Value`-Insel im Lesepfad: jede
/// Aufrufstelle haette sonst selbst konvertieren muessen, und `PyWert -> Value` ist nach Auflage 3
/// **fallibel** (NaN/inf) — eine Konvertierung je Aufrufstelle waere 94 mal dieselbe Entscheidung.
pub(crate) fn wert<'a>(f: &'a Felder, fid: &str) -> Option<&'a PyWert> {
    f.get(fid).map(|x: &SnapshotFeld| &x.wert)
}

/// `veranlagung` gegen seinen Bindungstyp: der typisierte Zugriff neben [`wert`] (Strangler, K7a).
pub(crate) fn feld_veranlagung(f: &Felder) -> Lage<'_, Veranlagung> {
    Lage::veranlagung(wert(f, "veranlagung"))
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
pub(crate) fn zahl_oder_null(v: Option<&PyWert>) -> Result<i64, BescheidFehler> {
    match v.and_then(PyWert::zahl_ohne_bool) {
        Some(z) => zahl_int(z),
        None => Ok(0),
    }
}

/// `isinstance(v, (int, float)) and not isinstance(v, bool) and v > 0` (ohne `int()`).
///
/// `gt_null` wirft fuer Text/`None` `TypeError`; der Vorfilter laesst nur Zahlen durch, also bleibt
/// der Aufruf fehlerfrei.
pub(crate) fn ist_positive_zahl(v: Option<&PyWert>) -> bool {
    v.and_then(PyWert::zahl_ohne_bool)
        .is_some_and(|z| z.gt_null() == Ok(true))
}

/// Wie [`ist_positive_zahl`], liefert bei `true` `int(v)`.
pub(crate) fn positive_zahl(v: Option<&PyWert>) -> Result<Option<i64>, BescheidFehler> {
    match v.and_then(PyWert::zahl_ohne_bool) {
        Some(z) if z.gt_null() == Ok(true) => zahl_int(z).map(Some),
        _ => Ok(None),
    }
}

/// Python `wert is True`.
pub(crate) fn ist_true(v: Option<&PyWert>) -> bool {
    matches!(v, Some(PyWert::Bool(true)))
}

/// Python `wert is False`.
pub(crate) fn ist_false(v: Option<&PyWert>) -> bool {
    matches!(v, Some(PyWert::Bool(false)))
}

/// Python `wert == "zusammen"` (`veranlagung`).
pub(crate) fn ist_zusammen(f: &Felder) -> bool {
    matches!(feld_veranlagung(f), Lage::Gueltig(Veranlagung::Zusammen))
}

/// Pythons `int(x)` fuer einen Slot- oder Store-Wert: Bool, Zahl, Text mit Vorzeichen/Ziffern/`_`.
///
/// `PyWert::int` traegt die vollstaendige Semantik (Unicode-Ziffern D6, 4300-Ziffern-Grenze D17,
/// Pythons Leerraum-Regel D16, `-2.0**63` D11) — die Vor-K2-Fassung daneben kannte nur
/// ASCII-Ziffern und lehnte `i64::MIN` als Ueberlauf ab. Gemessen in [`aequivalenz`].
///
/// ponytail: die Vor-K2-Fassung akzeptierte nur ASCII-Ziffern, weil Slot-Werte durchweg
/// Ganzzahlen sind (`intervall::SlotFehler`, gemessen 22 von 22). `PyWert::int` kann hier mehr;
/// das ist die Portierung, keine Verhaltensänderung am Bestand.
///
/// # Errors
/// [`BescheidFehler::Python`] (`TypeError`/`ValueError`), [`BescheidFehler::Ueberlauf`].
pub(crate) fn py_int(v: &PyWert) -> Result<i64, BescheidFehler> {
    v.int().map_err(|e| py_klasse(&e, "int(kein Zahlwert)"))
}

/// `str.isspace()` je Zeichen (Rusts `is_whitespace` kennt `\x1c`..`\x1f` nicht, Python schon).
pub(crate) fn py_leerraum(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Die Vor-K2-Fassungen der Lesehilfen, auf `serde_json::Value`.
///
/// Sie sind die Referenz, gegen die [`aequivalenz`] die Portierung misst, und sie halten die
/// D-Listen lebendig (Auflage 1 — die Falle fuer den ersten Fall, der sie trifft). Nichts in der
/// Produktion ruft sie; `py_int_alt` und `zahl_int_alt` tragen daneben die Gestalt, die
/// `elster::py::int` vor dem K2-Port hatte.
#[cfg(test)]
pub(crate) mod vor_k2 {
    use domain::Euro;
    use rust_decimal::Decimal;
    use serde_json::{Number, Value};

    use super::{py_leerraum, BescheidFehler};

    /// `int(v)` fuer eine JSON-Zahl (Python schneidet Floats Richtung 0 ab).
    pub(crate) fn zahl_int_alt(n: &Number) -> Result<i64, BescheidFehler> {
        if let Some(i) = n.as_i64() {
            return Ok(i);
        }
        if n.as_u64().is_some() {
            return Err(BescheidFehler::Ueberlauf("int(zahl)"));
        }
        match n.as_f64() {
            Some(x) if x.is_finite() && x.trunc().abs() < 9.223_372_036_854_776e18 =>
            {
                #[allow(clippy::cast_possible_truncation, reason = "Bereich vorher geprueft")]
                Ok(x.trunc() as i64)
            }
            _ => Err(BescheidFehler::Ueberlauf("int(float)")),
        }
    }

    /// JSON-Zahl → `Decimal`, jenseits des `Decimal`-Bereichs vorzeichenrichtig gesaettigt (D18).
    pub(crate) fn zahl_dezimal_alt(n: &Number) -> Decimal {
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

    /// `int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0`.
    pub(crate) fn zahl_oder_null_alt(v: Option<&Value>) -> Result<i64, BescheidFehler> {
        match v {
            Some(Value::Number(n)) => zahl_int_alt(n),
            _ => Ok(0),
        }
    }

    /// `isinstance(v, (int, float)) and not isinstance(v, bool) and v > 0`.
    pub(crate) fn ist_positive_zahl_alt(v: Option<&Value>) -> bool {
        let Some(Value::Number(n)) = v else {
            return false;
        };
        n.as_i64().map_or_else(
            || n.as_u64().is_some() || n.as_f64().is_some_and(|x| x > 0.0),
            |i| i > 0,
        )
    }

    /// Wie [`ist_positive_zahl_alt`], liefert bei `true` `int(v)`.
    pub(crate) fn positive_zahl_alt(v: Option<&Value>) -> Result<Option<i64>, BescheidFehler> {
        match v {
            Some(Value::Number(n)) if ist_positive_zahl_alt(v) => zahl_int_alt(n).map(Some),
            _ => Ok(None),
        }
    }

    /// Python `wert is True`.
    pub(crate) fn ist_true_alt(v: Option<&Value>) -> bool {
        matches!(v, Some(Value::Bool(true)))
    }

    /// Python `wert is False`.
    pub(crate) fn ist_false_alt(v: Option<&Value>) -> bool {
        matches!(v, Some(Value::Bool(false)))
    }

    /// Python-Wahrheitswert (`not x`).
    pub(crate) fn py_wahr_alt(v: &Value) -> bool {
        match v {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Object(o) => !o.is_empty(),
        }
    }

    /// `isinstance(v, int)` — Pythons `bool` ist ein `int`, hier 0/1; ein `u64` jenseits von `i64`
    /// wird zu `i64::MAX` gesaettigt (D3).
    pub(crate) fn py_int_wert_alt(v: Option<&Value>) -> Option<i64> {
        match v {
            Some(Value::Bool(b)) => Some(i64::from(*b)),
            other => ganzzahl_alt(other),
        }
    }

    /// `isinstance(v, int) and not isinstance(v, bool)` (Saettigung wie [`py_int_wert_alt`]).
    pub(crate) fn ganzzahl_alt(v: Option<&Value>) -> Option<i64> {
        match v {
            Some(Value::Number(n)) => n.as_i64().or_else(|| n.as_u64().map(|_| i64::MAX)),
            _ => None,
        }
    }

    /// `isinstance(v, (int, float)) and not isinstance(v, bool)`.
    pub(crate) fn zahl_wert_alt(v: Option<&Value>) -> Option<Decimal> {
        match v {
            Some(Value::Number(n)) => Some(zahl_dezimal_alt(n)),
            _ => None,
        }
    }

    /// `(v or 0) > 0`; anders als `positiv` wirft ein Text `TypeError`.
    pub(crate) fn oder_null_positiv_alt(v: Option<&Value>) -> Result<bool, BescheidFehler> {
        match v {
            Some(Value::Number(_)) => Ok(ist_positive_zahl_alt(v)),
            Some(Value::Bool(b)) => Ok(*b),
            Some(w) if py_wahr_alt(w) => Err(BescheidFehler::Python {
                klasse: "TypeError",
                was: "'>' zwischen Text/Liste und int",
            }),
            _ => Ok(false),
        }
    }

    /// Pythons `int(x)`: Bool, Zahl, Text mit ASCII-Vorzeichen/Ziffern/`_` — die Fassung, die nur
    /// ASCII-Ziffern kannte und `-2.0**63` (D11) als Ueberlauf abwies.
    pub(crate) fn py_int_alt(v: &Value) -> Result<i64, BescheidFehler> {
        match v {
            Value::Bool(b) => Ok(i64::from(*b)),
            Value::Number(n) => zahl_int_alt(n),
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

    /// Die Alt-Fassung von `zweige::wk::ist_int_ueber_3` (`Value`).
    pub(crate) fn ist_int_ueber_3_alt(v: Option<&Value>) -> bool {
        matches!(v, Some(Value::Number(n)) if n.as_i64().map_or(n.is_u64(), |i| i > 3))
    }

    /// Die Alt-Fassung von `zweige::wk::dezimal` (`Value`).
    pub(crate) fn dezimal_alt(v: &Value) -> Result<Decimal, BescheidFehler> {
        let fehler = || BescheidFehler::Python {
            klasse: "ValueError",
            was: "Decimal(str(entfernung_km_roh))",
        };
        match v {
            Value::Number(n) => {
                let t = n.to_string();
                Decimal::from_str_exact(&t)
                    .or_else(|_| Decimal::from_scientific(&t))
                    .map_err(|_| fehler())
            }
            Value::String(t) => Decimal::from_str_exact(t.trim()).map_err(|_| fehler()),
            _ => Err(fehler()),
        }
    }

    /// Die Alt-Fassung von `int(v or 0)`, wie sie `deklaration::c2` und `zweige::tarif::q_roh_cent`
    /// vor K2 trugen: `match wert(f, fid) { Some(v) if py_wahr(v) => py_int(v), _ => Ok(0) }`.
    pub(crate) fn int_oder_null_alt(v: &Value) -> Result<i64, BescheidFehler> {
        if py_wahr_alt(v) {
            py_int_alt(v)
        } else {
            Ok(0)
        }
    }

    /// Die Alt-Fassung von `deklaration::ring_werte::py_int_typ` (`Value`).
    pub(crate) fn py_int_typ_alt(v: &Value) -> Result<Option<i64>, BescheidFehler> {
        match v {
            Value::Bool(b) => Ok(Some(i64::from(*b))),
            Value::Number(n) if n.is_u64() && !n.is_i64() => Err(BescheidFehler::Ueberlauf("int")),
            Value::Number(n) => Ok(n.as_i64()),
            _ => Ok(None),
        }
    }

    /// Die Alt-Fassung von `zweige::rentner::rentenfreibetrag_euro` (`Value`).
    pub(crate) fn rentenfreibetrag_euro_alt(
        rf: Option<&Value>,
        cent_zu_euro_dezimal: impl Fn(Decimal) -> Euro,
    ) -> Option<Euro> {
        match rf {
            Some(Value::Number(n)) => Some(n.as_i64().map_or_else(
                || cent_zu_euro_dezimal(zahl_dezimal_alt(n)),
                super::cent_zu_euro,
            )),
            _ => None,
        }
    }
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
    clippy::must_use_candidate,
    clippy::disallowed_types
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
            let reg = bindung::lade_registry_der_wurzel(&repo()).unwrap();
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

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten. Die `pub(crate)`-Hilfen nutzen die
/// Aequivalenz-Module der Untermodule.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{
        ganzzahl_text, int_ausnahmen, json_wert, klasse, pruefe, py, text, zahl, Ergebnis,
    };
    use domain::{py_strip, Achsenwert, Herkunft, PruefTiefe, PyWert, Zustand};
    use proptest::prelude::*;
    use rust_decimal::Decimal;
    use serde_json::{json, Number, Value};
    use store::SnapshotFeld;

    use super::vor_k2::{
        ist_false_alt, ist_positive_zahl_alt, ist_true_alt, positive_zahl_alt, py_int_alt,
        py_wahr_alt, zahl_dezimal_alt, zahl_int_alt, zahl_oder_null_alt,
    };
    use super::{
        feld_int_oder_null, ist_false, ist_positive_zahl, ist_true, ist_zusammen, positive_zahl,
        py_int, py_leerraum, wert, zahl_dezimal, zahl_int, zahl_oder_null, BescheidFehler, Felder,
    };

    /// Ausnahmen von `py_int` und von `int(v or 0)` (`c2`, `q_roh_cent`).
    pub(crate) const INT: &[&str] = &["D6", "D11", "D16", "D17"];
    /// Ausnahmen von `zahl_int`, `zahl_oder_null` und `feld_int_oder_null`.
    const INT_ZAHL: &[&str] = &["D11"];
    /// Ausnahmen von `zahl_dezimal`, `zahl_wert` und `rentenfreibetrag_euro`.
    pub(crate) const DEZIMAL: &[&str] = &["D18"];

    /// `r` mit der Python-Klasse statt des `BescheidFehler`.
    pub(crate) fn alt_klasse<T>(r: Result<T, BescheidFehler>) -> Ergebnis<T> {
        r.map_err(|e| e.python_klasse())
    }

    /// Ein Snapshot mit dem einen Feld `fid`. Nimmt `Value` wie bisher und konvertiert EINMAL hier
    /// (`PyWert::from` ist total) — die Aufrufer bleiben unveraendert lesbar.
    pub(crate) fn ein_feld(fid: &str, wert: Value, bestaetigt: bool) -> Felder {
        let achse = |s: &str| Achsenwert::new(s).unwrap();
        let herkunft = Herkunft {
            herkunft: achse("laie"),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: achse("nutzer"),
        };
        let zustand = if bestaetigt {
            Zustand::Bestaetigt
        } else {
            Zustand::Vorlaeufig
        };
        let feld = SnapshotFeld {
            wert: PyWert::from(wert),
            zustand,
            herkunft: herkunft.into(),
        };
        Felder::from([(fid.to_owned(), feld)])
    }

    /// `int(v or 0)`.
    pub(crate) fn int_oder_null(v: &Value) -> Ergebnis<i64> {
        klasse(py(v).oder_null().int())
    }

    /// Ein Alt-Helfer auf `int(v or 0)` gegen [`int_oder_null`].
    pub(crate) fn int_oder_null_wie(v: &Value, alt: &Ergebnis<i64>) -> Result<(), TestCaseError> {
        let neu = int_oder_null(v);
        pruefe(v, alt, &neu, || int_ausnahmen(v, alt, &neu), INT)
    }

    /// D18: `dezimal()` meldet die Decimal-Grenze (`grenze`), wo der Alt-Helfer auf `Decimal::MAX`
    /// oder `Decimal::MIN` saettigt; `wie_alt` formt eine `Decimal` wie der Alt-Helfer.
    pub(crate) fn d18<T: PartialEq>(
        alt: &T,
        grenze: bool,
        wie_alt: impl Fn(Decimal) -> T,
    ) -> Vec<&'static str> {
        if grenze
            && [Decimal::MAX, Decimal::MIN]
                .into_iter()
                .any(|d| *alt == wie_alt(d))
        {
            vec!["D18"]
        } else {
            Vec::new()
        }
    }

    /// Ein Wert fuer `veranlagung`, `None` = Feld fehlt. Die festen Werte treffen jede `Lage`;
    /// der falsy-Zweig von `Abweichend` (`""`, `false`, `0`) bekommt eigene Treffer.
    pub(crate) fn veranlagung_json() -> impl Strategy<Value = Option<Value>> {
        let fest = vec![
            json!("einzel"),
            json!("zusammen"),
            json!("einzel "),
            json!(" zusammen"),
            json!("Zusammen"),
            json!(""),
            Value::Null,
            json!(false),
            json!(0),
        ];
        proptest::option::of(prop_oneof![2 => proptest::sample::select(fest), 1 => json_wert()])
    }

    /// Ein Wert fuer ein Enum-Feld mit den Bindungswerten `werte`, `None` = Feld fehlt. Je Wert:
    /// exakt (`Gueltig`), mit Leerraum, gross geschrieben, in Liste und in Objekt (`Abweichend`);
    /// dazu `null` und die falsy-Werte `""`, `false`, `0`, `[]` (K7b).
    pub(crate) fn enum_json(
        werte: impl IntoIterator<Item = &'static str>,
    ) -> impl Strategy<Value = Option<Value>> {
        let mut fest = vec![json!(""), Value::Null, json!(false), json!(0), json!([])];
        for w in werte {
            fest.extend([
                json!(w),
                json!(format!(" {w}")),
                json!(format!("{w} ")),
                json!(w.to_uppercase()),
                json!([w]),
                json!({ "wert": w }),
            ]);
        }
        proptest::option::of(prop_oneof![2 => proptest::sample::select(fest), 1 => json_wert()])
    }

    /// Die Alt-Fassung gegen `CPython` — die Messung, die die D-Nummern festhaelt (Auflage 1).
    fn int_wie_alt(v: &Value) -> Result<(), TestCaseError> {
        let (alt, neu) = (alt_klasse(py_int_alt(v)), klasse(py(v).int()));
        pruefe(v, &alt, &neu, || int_ausnahmen(v, &alt, &neu), INT)
    }

    /// Die Produktion gegen die Alt-Fassung. Jede Abweichung ist ein Befund: sie darf nur mit
    /// D-Nummer auftreten, und die D-Nummern sind hier die aus der Messung gegen `CPython`.
    fn int_wie(v: &Value) -> Result<(), TestCaseError> {
        pruefe(
            v,
            &alt_klasse(py_int_alt(v)),
            &alt_klasse(py_int(&py(v))),
            || int_ausnahmen(v, &alt_klasse(py_int_alt(v)), &alt_klasse(py_int(&py(v)))),
            INT,
        )
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        /// Die Alt-Fassung gegen `CPython`. Die Produktion ruft `truthy` seit K7a direkt: ohne
        /// Ausnahmen, weil `truthy` Pythons `bool(x)` unveraendert abbildet.
        #[test]
        fn py_wahr_alt_wie_pywert(v in json_wert()) {
            pruefe(&v, &py_wahr_alt(&v), &py(&v).truthy(), Vec::new, &[])?;
        }

        /// `wert is True` ist Identitaet, nicht `==`: nur `PyWert::Bool`. Beide Richtungen, weil
        /// die Produktion hier beide Zweige traegt.
        #[test]
        fn ist_true_false_wie_alt(v in json_wert()) {
            let w = py(&v);
            let (alt_t, alt_f) = (ist_true_alt(Some(&v)), ist_false_alt(Some(&v)));
            let (neu_t, neu_f) = (ist_true(Some(&w)), ist_false(Some(&w)));
            pruefe(&v, &alt_t, &neu_t, Vec::new, &[])?;
            pruefe(&v, &alt_f, &neu_f, Vec::new, &[])?;
            pruefe(&v, &alt_t, &matches!(w, PyWert::Bool(true)), Vec::new, &[])?;
            pruefe(&v, &alt_f, &matches!(w, PyWert::Bool(false)), Vec::new, &[])?;
        }

        /// K7a: `ist_zusammen` liest ueber `Lage`; die Fassung davor verglich den Text direkt.
        #[test]
        fn ist_zusammen_wie_alt(v in veranlagung_json()) {
            let f = v.clone().map_or_else(Felder::new, |w| ein_feld("veranlagung", w, true));
            let alt = matches!(wert(&f, "veranlagung"), Some(PyWert::Text(s)) if s == "zusammen");
            pruefe(&v, &alt, &ist_zusammen(&f), Vec::new, &[])?;
        }

        #[test]
        fn py_leerraum_wie_py_strip(s in text()) {
            pruefe(&s, &s.trim_matches(py_leerraum), &py_strip(&s), Vec::new, &[])?;
        }

        /// Die Produktion gegen die Alt-Fassung (jede Abweichung braucht eine D-Nummer).
        #[test]
        fn py_int_wie_alt(v in json_wert()) {
            int_wie(&v)?;
        }

        /// Die Alt-Fassung gegen `CPython` (die D-Nummern entstehen hier).
        #[test]
        fn py_int_alt_wie_pywert(v in json_wert()) {
            int_wie_alt(&v)?;
        }

        #[test]
        fn py_int_text_wie_pywert(s in ganzzahl_text()) {
            int_wie_alt(&Value::String(s.clone()))?;
            int_wie(&Value::String(s))?;
        }

        /// Die Alt-Fassung gegen `CPython`.
        #[test]
        fn zahl_int_alt_wie_pywert(n in zahl()) {
            let v = Value::Number(n.clone());
            let (alt, neu) = (alt_klasse(zahl_int_alt(&n)), klasse(py(&v).int()));
            pruefe(&v, &alt, &neu, || int_ausnahmen(&v, &alt, &neu), INT_ZAHL)?;
        }

        /// Die Produktion gegen die Alt-Fassung. D11 (`-2.0**63`) ist hier erlaubt: der Alt-Helfer
        /// lehnt die Grenze ab, `CPython` rechnet exakt — dieselbe Nummer wie in der Messung gegen
        /// `CPython` darueber, nicht eine neu erfundene.
        #[test]
        fn zahl_int_wie_alt(n in zahl()) {
            let v = Value::Number(n.clone());
            let (alt, neu) = (alt_klasse(zahl_int_alt(&n)), alt_klasse(zahl_int(&py(&v))));
            pruefe(&v, &alt, &neu, || int_ausnahmen(&v, &alt, &neu), INT_ZAHL)?;
        }

        /// `_c(fid)`: `int(v)` fuer eine Zahl ohne `bool`, sonst 0. Drei Messungen: Alt gegen
        /// `CPython`, Produktion gegen Alt, und `feld_int_oder_null` gegen die Produktion.
        #[test]
        fn zahl_oder_null_wie_pywert(v in json_wert()) {
            let w = py(&v);
            let cpython = w.zahl_ohne_bool().map_or(Ok(0), |z| klasse(z.int()));
            let alt = alt_klasse(zahl_oder_null_alt(Some(&v)));
            pruefe(&v, &alt, &cpython, || int_ausnahmen(&v, &alt, &cpython), INT_ZAHL)?;
            let neu = alt_klasse(zahl_oder_null(Some(&w)));
            pruefe(&v, &alt, &neu, || int_ausnahmen(&v, &alt, &neu), INT_ZAHL)?;
            let feld = ein_feld("x", v.clone(), true);
            pruefe(&v, &neu, &alt_klasse(feld_int_oder_null(&feld, "x")), Vec::new, &[])?;
        }

        #[test]
        fn ist_positive_zahl_wie_alt(v in json_wert()) {
            let neu = ist_positive_zahl(Some(&py(&v)));
            pruefe(&v, &ist_positive_zahl_alt(Some(&v)), &neu, Vec::new, &[])?;
            let cpython = py(&v).zahl_ohne_bool().is_some_and(|z| z.gt_null() == Ok(true));
            pruefe(&v, &neu, &cpython, Vec::new, &[])?;
        }

        #[test]
        fn positive_zahl_wie_alt(v in json_wert()) {
            let w = py(&v);
            let alt = alt_klasse(positive_zahl_alt(Some(&v)));
            let neu = alt_klasse(positive_zahl(Some(&w)));
            pruefe(&v, &alt, &neu, Vec::new, &[])?;
            let cpython = match w.zahl_ohne_bool() {
                Some(z) if z.gt_null() == Ok(true) => klasse(z.int()).map(Some),
                _ => Ok(None),
            };
            pruefe(&v, &neu, &cpython, Vec::new, &[])?;
        }

        /// Die Produktion gegen die Alt-Fassung; beide saettigen an derselben Stelle, also ohne
        /// Ausnahmen.
        #[test]
        fn zahl_dezimal_wie_alt(n in zahl()) {
            let v = Value::Number(n.clone());
            let (alt, neu): (Ergebnis<Decimal>, Ergebnis<Decimal>) =
                (Ok(zahl_dezimal_alt(&n)), Ok(zahl_dezimal(&py(&v))));
            pruefe(&v, &alt, &neu, Vec::new, &[])?;
            // Und die Alt-Fassung gegen `CPython`: D18 ist die Saettigung, wo Python exakt bleibt.
            let cpython = klasse(py(&v).dezimal());
            let wie_alt = |d: Decimal| -> Ergebnis<Decimal> { Ok(d) };
            pruefe(&v, &alt, &cpython, || d18(&alt, cpython == Err(None), wie_alt), DEZIMAL)?;
        }
    }

    /// D6: `int("٣")` ist in `CPython` 3. Die Alt-Fassung kennt nur ASCII-Ziffern und meldet
    /// `ValueError`; die Produktion erbt `PyWert::int` und trifft `CPython`.
    #[test]
    fn d6_nd_ziffer() {
        let v = json!("\u{663}");
        assert_eq!(alt_klasse(py_int_alt(&v)), Err(Some("ValueError")));
        assert_eq!(klasse(py(&v).int()), Ok(3));
        assert_eq!(alt_klasse(py_int(&py(&v))), Ok(3));
    }

    /// D11: `int(-2.0**63)` ist in `CPython` `i64::MIN`. Die Alt-Fassung weist den Wert als
    /// Ueberlauf ab (sie prueft `x.trunc().abs() < 9.22e18` und `2**63` liegt genau darauf);
    /// die Produktion erbt `PyWert::int`.
    #[test]
    fn d11_minus_2_hoch_63() {
        let v = json!(-9_223_372_036_854_775_808.0);
        assert_eq!(alt_klasse(py_int_alt(&v)), Err(None));
        assert_eq!(alt_klasse(zahl_oder_null_alt(Some(&v))), Err(None));
        assert_eq!(klasse(py(&v).int()), Ok(i64::MIN));
        assert_eq!(alt_klasse(py_int(&py(&v))), Ok(i64::MIN));
    }

    /// D16: `int("\x1c42")` wirft in `CPython` `ValueError`, obwohl `str.strip()` U+001C entfernt.
    /// Die Alt-Fassung strippt mit `py_leerraum` und liest 42; die Produktion erbt `PyWert::int`,
    /// das Pythons `int()`-Leerraum-Regel enger fasst.
    #[test]
    fn d16_steuerzeichen_am_rand() {
        let v = json!("\u{1c}42");
        assert_eq!(alt_klasse(py_int_alt(&v)), Ok(42));
        assert_eq!(klasse(py(&v).int()), Err(Some("ValueError")));
        assert_eq!(alt_klasse(py_int(&py(&v))), Err(Some("ValueError")));
    }

    /// D17: `int()` mit mehr als 4300 Ziffern wirft in `CPython` `ValueError`.
    #[test]
    fn d17_mehr_als_4300_ziffern() {
        let nullen = json!(format!("{}5", "0".repeat(4300)));
        let sieben = json!("7".repeat(4301));
        assert_eq!(alt_klasse(py_int_alt(&nullen)), Ok(5));
        assert_eq!(alt_klasse(py_int_alt(&sieben)), Err(None));
        for v in [nullen, sieben] {
            assert_eq!(klasse(py(&v).int()), Err(Some("ValueError")));
            assert_eq!(alt_klasse(py_int(&py(&v))), Err(Some("ValueError")));
        }
    }

    /// D18: `Decimal(1e29)` ist in `CPython` exakt. Der Alt-Helfer saettigt still auf
    /// `Decimal::MAX`, `PyWert` meldet die Decimal-Grenze.
    #[test]
    fn d18_dezimal_grenze() {
        let n = Number::from_f64(1e29).unwrap();
        assert_eq!(zahl_dezimal_alt(&n), Decimal::MAX);
        assert_eq!(klasse(py(&Value::Number(n)).dezimal()), Err(None));
    }
}
