//! `PyWert`: ein Wert aus `json.loads`/`yaml.safe_load` mit der Semantik von `CPython` (gemessen an
//! 3.12.9 und 3.14.7). Er loest die Python-Helfer auf `serde_json::Value` in bescheid, elster, llm,
//! auth, api, konsistenz, interview und intervall ab (K1-Inventur; K3-K7 stellen die Aufrufer um).
//!
//! Wo Rust eine Grenze hat, die Python nicht kennt, traegt der Fehler einen eigenen Namen
//! ([`PyFehler::I64Grenze`], [`PyFehler::DezimalGrenze`]) statt eines Python-Klassennamens.
use std::collections::HashMap;
use std::fmt;

use rust_decimal::Decimal;
use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::py_text::{int_aus_text, repr_float, repr_str};

/// Ein Wert, wie ihn `json.loads` oder `yaml.safe_load` liefert.
///
/// Fuer Pythons `==` gilt [`PyWert::py_eq`], NICHT das abgeleitete `PartialEq` (mit
/// `True == 1`). Die strukturelle Gleichheit existiert nur, weil `store::Event` und
/// `store::SnapshotFeld` sie fuer den Wire-Roundtrip ableiten -- sie ist KEIN Wertvergleich:
/// `PyWert::Bool(false) == PyWert::Ganz(0)` ist dort falsch, `py_eq` sagt wahr.
#[derive(Debug, Clone, PartialEq)]
pub enum PyWert {
    /// `None`.
    Null,
    /// `bool`.
    Bool(bool),
    /// `int` im Bereich von `i64`.
    Ganz(i64),
    /// `int` ueber `i64::MAX` bis `u64::MAX` (kleinere legt [`Deserialize`] als `Ganz` ab).
    GrossGanz(u64),
    /// `float`, auch NaN und +-inf (D2).
    Gleit(f64),
    /// `str`.
    Text(String),
    /// `list`.
    Liste(Vec<PyWert>),
    /// `dict` mit Text-Schluesseln in Einfuegereihenfolge, jeder Schluessel einmal (D8).
    Objekt(Vec<(String, PyWert)>),
}

/// Eine Ausnahme, die `CPython` an derselben Stelle wirft (Text wie `str(e)`), oder eine
/// Rust-Grenze, die Python nicht kennt (D4).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PyFehler {
    /// `TypeError`.
    #[error("{0}")]
    TypFehler(String),
    /// `ValueError`.
    #[error("{0}")]
    WertFehler(String),
    /// `OverflowError`, nur wo `CPython` ihn wirft.
    #[error("{0}")]
    Ueberlauf(String),
    /// Ganzzahl ausserhalb von `i64`; `CPython` rechnet hier exakt weiter (D3).
    #[error("{0}")]
    I64Grenze(String),
    /// NaN, +-inf oder ein Betrag ueber `Decimal::MAX` (~7,9e28) (D18).
    #[error("{0}")]
    DezimalGrenze(String),
}

impl PyFehler {
    /// Pythons Klassenname (`type(e).__name__`); `None` fuer eine Rust-Grenze.
    ///
    /// ```
    /// use domain::PyWert;
    /// let e = PyWert::Gleit(f64::INFINITY).int().unwrap_err();
    /// assert_eq!(e.python_klasse(), Some("OverflowError"));
    /// assert_eq!(PyWert::GrossGanz(u64::MAX).int().unwrap_err().python_klasse(), None);
    /// ```
    #[must_use]
    pub const fn python_klasse(&self) -> Option<&'static str> {
        match self {
            Self::TypFehler(_) => Some("TypeError"),
            Self::WertFehler(_) => Some("ValueError"),
            Self::Ueberlauf(_) => Some("OverflowError"),
            Self::I64Grenze(_) | Self::DezimalGrenze(_) => None,
        }
    }
}

impl PyWert {
    /// `bool(x)`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert!(PyWert::Gleit(f64::NAN).truthy());
    /// assert!(!PyWert::Text(String::new()).truthy());
    /// ```
    #[must_use]
    pub fn truthy(&self) -> bool {
        match self {
            Self::Null => false,
            Self::Bool(b) => *b,
            Self::Ganz(n) => *n != 0,
            Self::GrossGanz(u) => *u != 0,
            Self::Gleit(f) => *f != 0.0,
            Self::Text(s) => !s.is_empty(),
            Self::Liste(l) => !l.is_empty(),
            Self::Objekt(o) => !o.is_empty(),
        }
    }

    /// `type(x).__name__`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Null.typname(), "NoneType");
    /// assert_eq!(PyWert::GrossGanz(u64::MAX).typname(), "int");
    /// ```
    #[must_use]
    pub const fn typname(&self) -> &'static str {
        match self {
            Self::Null => "NoneType",
            Self::Bool(_) => "bool",
            Self::Ganz(_) | Self::GrossGanz(_) => "int",
            Self::Gleit(_) => "float",
            Self::Text(_) => "str",
            Self::Liste(_) => "list",
            Self::Objekt(_) => "dict",
        }
    }

    /// `int(x)` als `i64`.
    ///
    /// # Errors
    /// Wie `CPython` (`TypeError`, `ValueError`, `OverflowError`, s. [`PyWert::int_dezimal`]);
    /// [`PyFehler::I64Grenze`], wo Python ueber `i64` hinaus exakt weiterrechnet.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Gleit(-2.7).int(), Ok(-2));
    /// assert_eq!(PyWert::Gleit(-(2f64.powi(63))).int(), Ok(i64::MIN));
    /// assert_eq!(PyWert::Text("-9223372036854775808".into()).int(), Ok(i64::MIN));
    /// assert!(PyWert::Gleit(2f64.powi(63)).int().is_err());
    /// ```
    pub fn int(&self) -> Result<i64, PyFehler> {
        match self {
            Self::Bool(b) => Ok(i64::from(*b)),
            Self::Ganz(n) => Ok(*n),
            _ => self
                .int_dezimal()?
                .parse()
                .map_err(|_| PyFehler::I64Grenze("int() liegt ausserhalb von i64".to_owned())),
        }
    }

    /// `int(x)` als exakter Dezimaltext ohne fuehrende Nullen (D5, Umsetzung aus api
    /// `python::int`): ein Float mit allen Stellen seines abgeschnittenen Werts, Text mit
    /// Unicode-Ziffern (D6), Pythons Leerraum-Regel (D16) und Ziffern-Grenze (D17).
    ///
    /// # Errors
    /// Wie `CPython`: `TypeError` fuer `None`, `list`, `dict`; `ValueError` fuer NaN und
    /// ungueltigen Text; `OverflowError` fuer +-inf.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Gleit(1e20).int_dezimal().unwrap(), "100000000000000000000");
    /// assert_eq!(PyWert::Text(" +1_000\u{3000}".into()).int_dezimal().unwrap(), "1000");
    /// assert_eq!(PyWert::Text("\u{663}\u{664}".into()).int_dezimal().unwrap(), "34");
    /// assert!(PyWert::Text("\u{1c}42".into()).int_dezimal().is_err());
    /// ```
    pub fn int_dezimal(&self) -> Result<String, PyFehler> {
        match self {
            Self::Bool(b) => Ok(u8::from(*b).to_string()),
            Self::Ganz(n) => Ok(n.to_string()),
            Self::GrossGanz(u) => Ok(u.to_string()),
            Self::Gleit(f) if f.is_nan() => Err(PyFehler::WertFehler(
                "cannot convert float NaN to integer".to_owned(),
            )),
            Self::Gleit(f) if f.is_infinite() => Err(PyFehler::Ueberlauf(
                "cannot convert float infinity to integer".to_owned(),
            )),
            Self::Gleit(f) => {
                // `{:.0}` schreibt einen ganzzahligen Float mit allen Stellen; nur -0.0 traegt
                // dabei ein Vorzeichen, das `int` nicht hat.
                let text = format!("{:.0}", f.trunc());
                Ok(if text == "-0" { "0".to_owned() } else { text })
            }
            Self::Text(s) => int_aus_text(s),
            Self::Null | Self::Liste(_) | Self::Objekt(_) => Err(PyFehler::TypFehler(format!(
                "int() argument must be a string, a bytes-like object or a real number, not '{}'",
                self.typname()
            ))),
        }
    }

    /// `x if isinstance(x, int) else None` (`bool` zaehlt als `int`, D10).
    ///
    /// # Errors
    /// [`PyFehler::I64Grenze`] fuer eine Ganzzahl ueber `i64::MAX`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Bool(true).int_mit_bool(), Ok(Some(1)));
    /// assert_eq!(PyWert::Gleit(2.0).int_mit_bool(), Ok(None));
    /// ```
    pub fn int_mit_bool(&self) -> Result<Option<i64>, PyFehler> {
        match self {
            Self::Bool(_) | Self::Ganz(_) | Self::GrossGanz(_) => self.int().map(Some),
            _ => Ok(None),
        }
    }

    /// `x if isinstance(x, int) and not isinstance(x, bool) else None` (D10).
    ///
    /// # Errors
    /// [`PyFehler::I64Grenze`] fuer eine Ganzzahl ueber `i64::MAX`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Ganz(7).int_ohne_bool(), Ok(Some(7)));
    /// assert_eq!(PyWert::Bool(true).int_ohne_bool(), Ok(None));
    /// ```
    pub fn int_ohne_bool(&self) -> Result<Option<i64>, PyFehler> {
        if matches!(self, Self::Bool(_)) {
            return Ok(None);
        }
        self.int_mit_bool()
    }

    /// `x if isinstance(x, (int, float)) and not isinstance(x, bool) else None`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert!(PyWert::Gleit(0.5).zahl_ohne_bool().is_some());
    /// assert!(PyWert::Bool(true).zahl_ohne_bool().is_none());
    /// ```
    #[must_use]
    pub fn zahl_ohne_bool(&self) -> Option<&Self> {
        matches!(self, Self::Ganz(_) | Self::GrossGanz(_) | Self::Gleit(_)).then_some(self)
    }

    /// `x or 0`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Null.oder_null().int(), Ok(0));
    /// assert_eq!(PyWert::Text("5".into()).oder_null().int(), Ok(5));
    /// ```
    #[must_use]
    pub fn oder_null(&self) -> &Self {
        static NULL: PyWert = PyWert::Ganz(0);
        if self.truthy() {
            self
        } else {
            &NULL
        }
    }

    /// `x > 0`.
    ///
    /// # Errors
    /// `TypeError` wie `CPython` fuer `None`, `str`, `list` und `dict`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Gleit(f64::NAN).gt_null(), Ok(false));
    /// assert_eq!(
    ///     PyWert::Text("1".into()).gt_null().unwrap_err().to_string(),
    ///     "'>' not supported between instances of 'str' and 'int'"
    /// );
    /// ```
    pub fn gt_null(&self) -> Result<bool, PyFehler> {
        match self {
            Self::Bool(b) => Ok(*b),
            Self::Ganz(n) => Ok(*n > 0),
            Self::GrossGanz(u) => Ok(*u > 0),
            Self::Gleit(f) => Ok(*f > 0.0),
            Self::Null | Self::Text(_) | Self::Liste(_) | Self::Objekt(_) => {
                Err(PyFehler::TypFehler(format!(
                    "'>' not supported between instances of '{}' and 'int'",
                    self.typname()
                )))
            }
        }
    }

    /// Der Zahlwert von `int`, `float` oder `bool` als `Decimal` (Geldpfade rechnen ohne `f64`).
    /// Ein Float gilt mit seinem binaeren Wert (`from_f64_retain`), nicht mit seiner Kurzform.
    ///
    /// ponytail: `from_f64_retain` trifft den binaeren Wert auf 28 Nachkommastellen bis auf die
    /// letzte (gemessen 1/3: `...162561` statt `...162562`), `CPython` `Decimal(x)` ist exakt; ein
    /// Float unter 1e-28 wird 0 (gemessen 1e-29), `x > 0` kippt dann. Upgrade: exakter Bruch,
    /// falls ein Geldpfad je solche Werte sieht.
    ///
    /// # Errors
    /// [`PyFehler::DezimalGrenze`] fuer NaN, +-inf und Betraege ueber `Decimal::MAX` (D18: keine
    /// stille Saettigung auf einem Geldpfad); `TypeError` wie `CPython`, wo eine reelle Zahl
    /// verlangt ist (`math.isfinite(x)`), fuer alles ausser Zahlen.
    ///
    /// ```
    /// use domain::PyWert;
    /// use rust_decimal::Decimal;
    /// assert_eq!(PyWert::Gleit(0.5).dezimal(), Ok(Decimal::new(5, 1)));
    /// assert!(PyWert::Gleit(1e29).dezimal().is_err());
    /// ```
    pub fn dezimal(&self) -> Result<Decimal, PyFehler> {
        match self {
            Self::Bool(b) => Ok(Decimal::from(u8::from(*b))),
            Self::Ganz(n) => Ok(Decimal::from(*n)),
            Self::GrossGanz(u) => Ok(Decimal::from(*u)),
            Self::Gleit(f) => Decimal::from_f64_retain(*f).ok_or_else(|| {
                PyFehler::DezimalGrenze(
                    "Decimal kennt NaN, inf und Betraege ueber 7,9e28 nicht".to_owned(),
                )
            }),
            Self::Null | Self::Text(_) | Self::Liste(_) | Self::Objekt(_) => Err(
                PyFehler::TypFehler(format!("must be real number, not {}", self.typname())),
            ),
        }
    }

    /// `self == other`: `True == 1 == 1.0`, `False == 0`, Zahl gegen Text nie gleich, `dict`
    /// ohne Reihenfolge, `int` gegen `float` exakt (D13, D14).
    ///
    /// ponytail: `CPython` vergleicht Elemente in `list`/`dict` zuerst per Identitaet, und
    /// `json.loads` wie `yaml.safe_load` liefern je Lader EIN NaN-Objekt: `[nan] == [nan]` ist
    /// dort wahr, wenn beide Seiten aus demselben Lader kommen (gemessen 3.12.9/3.14.7). `PyWert`
    /// kennt keine Identitaet, NaN ist hier nie gleich. Upgrade: Lader des NaN in `Gleit` fuehren.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert!(PyWert::Bool(false).py_eq(&PyWert::Ganz(0)));
    /// assert!(PyWert::GrossGanz(1 << 63).py_eq(&PyWert::Gleit(2f64.powi(63))));
    /// assert!(!PyWert::GrossGanz(u64::MAX).py_eq(&PyWert::Gleit(2f64.powi(64))));
    /// assert!(!PyWert::Text("1".into()).py_eq(&PyWert::Ganz(1)));
    /// ```
    #[must_use]
    pub fn py_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Text(x), Self::Text(y)) => x == y,
            (Self::Liste(x), Self::Liste(y)) => {
                x.len() == y.len() && x.iter().zip(y).all(|(p, q)| p.py_eq(q))
            }
            (Self::Objekt(x), Self::Objekt(y)) => {
                let y: HashMap<&str, &Self> = y.iter().map(|(k, w)| (k.as_str(), w)).collect();
                x.len() == y.len()
                    && x.iter()
                        .all(|(k, v)| y.get(k.as_str()).is_some_and(|w| v.py_eq(w)))
            }
            _ => match (self.zahl(), other.zahl()) {
                (Some(Zahl::Ganz(x)), Some(Zahl::Ganz(y))) => x == y,
                (Some(Zahl::Gleit(x)), Some(Zahl::Gleit(y))) => x == y,
                (Some(Zahl::Ganz(i)), Some(Zahl::Gleit(f)))
                | (Some(Zahl::Gleit(f)), Some(Zahl::Ganz(i))) => ganz_gleich_gleit(i, f),
                _ => false,
            },
        }
    }

    fn zahl(&self) -> Option<Zahl> {
        match self {
            Self::Bool(b) => Some(Zahl::Ganz(i128::from(*b))),
            Self::Ganz(n) => Some(Zahl::Ganz(i128::from(*n))),
            Self::GrossGanz(u) => Some(Zahl::Ganz(i128::from(*u))),
            Self::Gleit(f) => Some(Zahl::Gleit(*f)),
            _ => None,
        }
    }

    /// `repr(x)`; `dict` in Einfuegereihenfolge (D8), `str` nach D9.
    ///
    /// ```
    /// let w: domain::PyWert = serde_json::from_str(r#"{"b": 1, "a": [null, true, 1e16, "it's"]}"#).unwrap();
    /// assert_eq!(w.repr(), r#"{'b': 1, 'a': [None, True, 1e+16, "it's"]}"#);
    /// ```
    #[must_use]
    pub fn repr(&self) -> String {
        match self {
            Self::Null => "None".to_owned(),
            Self::Bool(true) => "True".to_owned(),
            Self::Bool(false) => "False".to_owned(),
            Self::Ganz(n) => n.to_string(),
            Self::GrossGanz(u) => u.to_string(),
            Self::Gleit(f) => repr_float(*f),
            Self::Text(s) => repr_str(s),
            Self::Liste(l) => {
                let teile: Vec<String> = l.iter().map(Self::repr).collect();
                format!("[{}]", teile.join(", "))
            }
            Self::Objekt(o) => {
                let teile: Vec<String> = o
                    .iter()
                    .map(|(k, w)| format!("{}: {}", repr_str(k), w.repr()))
                    .collect();
                format!("{{{}}}", teile.join(", "))
            }
        }
    }

    /// `str(x)`: ein Text bleibt er selbst, alles andere ist sein `repr`.
    ///
    /// ```
    /// use domain::PyWert;
    /// assert_eq!(PyWert::Text("a'b".into()).py_str(), "a'b");
    /// assert_eq!(PyWert::Gleit(1e-5).py_str(), "1e-05");
    /// ```
    #[must_use]
    pub fn py_str(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            _ => self.repr(),
        }
    }
    /// `PyWert` -> `serde_json::Value`, fuer die Hash-Grenze (`canonical_json` nimmt `&Value`).
    ///
    /// # Errors
    /// [`PyFehler::DezimalGrenze`] bei NaN/+-inf -- Auflage 3: ein FEHLER, kein stiller
    /// `null`-Wert, denn `null` ist nicht hash-gleich zu NaN. Der Fehler greift rekursiv, auch
    /// fuer ein NaN in einer `Liste` oder einem `Objekt`.
    ///
    /// `Liste`/`Objekt` gehen ueber `serde_json::Value::Array`/`Object`. Ohne `preserve_order`
    /// ist `serde_json::Map` eine `BTreeMap` und sortiert die Schluessel -- genau das tut
    /// `canonical_json` in Python (`json.dumps(..., sort_keys=True)`, `store.py:24`) und genau
    /// das tat die Vor-K2-Fassung, in der `SnapshotFeld.wert` ein `Value` war. Die
    /// Einfuegereihenfolge von `PyWert::Objekt` geht damit NICHT in den Hash ein. Sie
    /// wiederherzustellen waere der Fehler, nicht sie zu sortieren: ein abgeleitetes
    /// `Serialize` fuer `PyWert` taete genau das (Auflage 1/2).
    ///
    /// Gemessen (2026-10-01): `store.py::materialisiere` nimmt `wert: [1]` an und liefert einen
    /// gueltigen `snapshot_id`; die Vor-K2-Fassung lief mit demselben Generatorfall gruen. Ein
    /// harter Fehler hier waere eine Paritaetsabweichung gegen das Orakel, kein Schutz.
    pub fn zu_json(&self) -> Result<serde_json::Value, PyFehler> {
        match self {
            Self::Null => Ok(serde_json::Value::Null),
            Self::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            Self::Ganz(n) => Ok(serde_json::Value::Number((*n).into())),
            Self::GrossGanz(u) => Ok(serde_json::Value::Number((*u).into())),
            Self::Gleit(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .ok_or_else(|| {
                    PyFehler::DezimalGrenze(format!(
                        "{f} ist in JSON nicht darstellbar (NaN/inf); canonical_json kennt \
                         dafuer keinen Wert, und ein stiller null-Wert waere hash-fremd"
                    ))
                }),
            Self::Text(s) => Ok(serde_json::Value::String(s.clone())),
            Self::Liste(teile) => teile
                .iter()
                .map(Self::zu_json)
                .collect::<Result<Vec<_>, _>>()
                .map(serde_json::Value::Array),
            Self::Objekt(paare) => paare
                .iter()
                .map(|(k, w)| Ok((k.clone(), w.zu_json()?)))
                .collect::<Result<serde_json::Map<String, serde_json::Value>, PyFehler>>()
                .map(serde_json::Value::Object),
        }
    }
}

/// `serde_json::Value` -> `PyWert`: die TOTALE Richtung. JSON ist eine Teilmenge dessen, was
/// `PyWert` traegt — NaN, Betraege ueber `u64::MAX` und die Dict-Einfuegereihenfolge kommen in
/// `Value` gar nicht erst vor. Die Gegenrichtung ist [`PyWert::zu_json`] und KANN scheitern.
impl From<serde_json::Value> for PyWert {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::Ganz(i)
                } else if let Some(u) = n.as_u64() {
                    Self::GrossGanz(u)
                } else {
                    // `Number` traegt genau eines von i64/u64/f64; hier bleibt nur f64. Der
                    // Fallback ist NaN, nicht 0.0: NaN ist der Wert, den `zu_json` als nicht
                    // darstellbar zurueckweist — 0.0 waere eine stille Falschaussage.
                    Self::Gleit(n.as_f64().unwrap_or(f64::NAN))
                }
            }
            serde_json::Value::String(s) => Self::Text(s),
            serde_json::Value::Array(a) => Self::Liste(a.into_iter().map(Into::into).collect()),
            serde_json::Value::Object(m) => {
                Self::Objekt(m.into_iter().map(|(k, w)| (k, w.into())).collect())
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Zahl {
    Ganz(i128),
    Gleit(f64),
}

/// `int == float` wie `CPython` (D14): nicht endlich oder gebrochen ist nie gleich, sonst exakt.
/// Jede ganzzahlige `f64` unter 2^127 passt verlustfrei in `i128`; darueber saettigt `as` auf
/// `i128::MAX`/`MIN`, die keine `PyWert`-Ganzzahl erreicht (alle liegen in (-2^64, 2^64)).
#[allow(
    clippy::cast_possible_truncation,
    reason = "ganzzahlige f64 unter 2^127 sind in i128 exakt, s. Doku"
)]
fn ganz_gleich_gleit(i: i128, f: f64) -> bool {
    f.is_finite() && f.fract() == 0.0 && f as i128 == i
}

impl<'de> Deserialize<'de> for PyWert {
    /// Aus JSON und YAML (D1). Ganzzahlen bis `i64` werden `Ganz`, bis `u64` `GrossGanz`;
    /// `dict` behaelt die Einfuegereihenfolge, ein doppelter Schluessel behaelt seinen ersten
    /// Platz und seinen letzten Wert (wie `json.loads` und `yaml.safe_load`, D8).
    ///
    /// ponytail: eine Ganzzahl ausserhalb von `i64::MIN..=u64::MAX` wird `Gleit`, gerundet auf
    /// 53 Bit; Python haelt sie exakt. `serde_json` ohne `arbitrary_precision` liefert sie ohnehin
    /// nur als `f64` (und lehnt sie ab ~1,8e308 ganz ab). Upgrade: eigene Variante mit
    /// Dezimaltext und eigenem Zahlparser, ohne Feature-Aenderung im Workspace (D1).
    ///
    /// ```
    /// use domain::PyWert;
    /// let w: PyWert = serde_json::from_str(r#"{"a": 1, "b": 2, "a": 3}"#).unwrap();
    /// assert_eq!(w.repr(), "{'a': 3, 'b': 2}");
    /// let w: PyWert = serde_json::from_str("18446744073709551615").unwrap();
    /// assert!(matches!(w, PyWert::GrossGanz(u64::MAX)));
    /// ```
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(BesucherWert)
    }
}

struct BesucherWert;

impl<'de> Visitor<'de> for BesucherWert {
    type Value = PyWert;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("einen JSON- oder YAML-Wert")
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<PyWert, E> {
        Ok(PyWert::Null)
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<PyWert, E> {
        Ok(PyWert::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<PyWert, D::Error> {
        PyWert::deserialize(d)
    }

    fn visit_bool<E: serde::de::Error>(self, b: bool) -> Result<PyWert, E> {
        Ok(PyWert::Bool(b))
    }

    fn visit_i64<E: serde::de::Error>(self, n: i64) -> Result<PyWert, E> {
        Ok(PyWert::Ganz(n))
    }

    fn visit_u64<E: serde::de::Error>(self, n: u64) -> Result<PyWert, E> {
        Ok(i64::try_from(n).map_or(PyWert::GrossGanz(n), PyWert::Ganz))
    }

    /// YAML liefert Ganzzahlen bis `i128`/`u128` hierher, `serde_json` nie.
    fn visit_i128<E: serde::de::Error>(self, n: i128) -> Result<PyWert, E> {
        Ok(aus_i128(n))
    }

    #[allow(
        clippy::cast_precision_loss,
        reason = "ausserhalb von i128 nur noch als f64 (ponytail an Deserialize)"
    )]
    fn visit_u128<E: serde::de::Error>(self, n: u128) -> Result<PyWert, E> {
        Ok(i128::try_from(n).map_or(PyWert::Gleit(n as f64), aus_i128))
    }

    fn visit_f64<E: serde::de::Error>(self, f: f64) -> Result<PyWert, E> {
        Ok(PyWert::Gleit(f))
    }

    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<PyWert, E> {
        Ok(PyWert::Text(s.to_owned()))
    }

    fn visit_string<E: serde::de::Error>(self, s: String) -> Result<PyWert, E> {
        Ok(PyWert::Text(s))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<PyWert, A::Error> {
        let mut liste = Vec::new();
        while let Some(w) = seq.next_element()? {
            liste.push(w);
        }
        Ok(PyWert::Liste(liste))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<PyWert, A::Error> {
        let mut paare: Vec<(String, PyWert)> = Vec::new();
        let mut platz: HashMap<String, usize> = HashMap::new();
        while let Some((k, w)) = map.next_entry::<String, PyWert>()? {
            if let Some(paar) = platz.get(&k).and_then(|&i| paare.get_mut(i)) {
                paar.1 = w;
            } else {
                platz.insert(k.clone(), paare.len());
                paare.push((k, w));
            }
        }
        Ok(PyWert::Objekt(paare))
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "ausserhalb von i64::MIN..=u64::MAX nur noch als f64 (ponytail an Deserialize)"
)]
fn aus_i128(n: i128) -> PyWert {
    if let Ok(i) = i64::try_from(n) {
        PyWert::Ganz(i)
    } else if let Ok(u) = u64::try_from(n) {
        PyWert::GrossGanz(u)
    } else {
        PyWert::Gleit(n as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::{PyFehler, PyWert};
    use rust_decimal::Decimal;

    fn json(t: &str) -> PyWert {
        serde_json::from_str(t).unwrap()
    }

    fn repr_json(t: &str) -> Option<String> {
        serde_json::from_str::<PyWert>(t).ok().map(|w| w.repr())
    }

    fn repr_yaml(t: &str) -> Option<String> {
        serde_yaml_ng::from_str::<PyWert>(t).ok().map(|w| w.repr())
    }

    /// 310 Stellen, ueber `f64::MAX`.
    fn riesig() -> String {
        format!("1{}", "0".repeat(309))
    }

    /// D1, D2, D8: `serde_json` gegen `repr(json.loads(t))`, gemessen 3.12.9 und 3.14.7.
    /// `abweichend`: (Eingabe, `repr` des `PyWert` oder `None` = Dokument laedt nicht, `CPython`).
    #[test]
    fn json_wie_gemessen() {
        for (t, python) in [
            ("18446744073709551615", "18446744073709551615"),
            ("-9223372036854775808", "-9223372036854775808"),
            ("1e-400", "0.0"),
            (r#"{"a": 1, "b": 2, "a": 3}"#, "{'a': 3, 'b': 2}"),
            ("1e16", "1e+16"),
            ("-0.0", "-0.0"),
            ("1.0", "1.0"),
            ("[1, true, null]", "[1, True, None]"),
        ] {
            assert_eq!(repr_json(t).as_deref(), Some(python), "{t}");
        }
        let riesig = riesig();
        for (t, rust, python) in [
            (
                "18446744073709551616",
                Some("1.8446744073709552e+19"),
                "18446744073709551616",
            ),
            (
                "-9223372036854775809",
                Some("-9.223372036854776e+18"),
                "-9223372036854775809",
            ),
            (&riesig, None, "1000…0"),
            ("1e400", None, "inf"),
            ("NaN", None, "nan"),
            ("-Infinity", None, "-inf"),
            (r#""\ud800""#, None, r"'\ud800'"),
            ("-0", Some("-0.0"), "0"),
        ] {
            let ist = repr_json(t);
            assert_eq!(ist.as_deref(), rust, "{t}");
            assert_ne!(ist.as_deref(), Some(python), "{t}");
        }
    }

    /// D1, D2, D8: `serde_yaml_ng` gegen `repr(yaml.safe_load(t))` (`PyYAML` 6.0.3), gemessen
    /// 3.12.9 und 3.14.7. Die meisten Abweichungen sind YAML 1.2 (`serde_yaml_ng`) gegen 1.1
    /// (`PyYAML`) und betreffen jedes Rust-Ziel, nicht nur `PyWert`.
    #[test]
    fn yaml_wie_gemessen() {
        for (t, python) in [
            (".nan", "nan"),
            (".inf", "inf"),
            ("-.inf", "-inf"),
            ("18446744073709551615", "18446744073709551615"),
            ("NaN", "'NaN'"),
            ("Infinity", "'Infinity'"),
            ("1e400", "'1e400'"),
            ("1.0e+16", "1e+16"),
            ("0x1F", "31"),
            ("0b11", "3"),
            ("[~, null, Null, NULL, '']", "[None, None, None, None, '']"),
            ("{a: 1, b: 2, a: 3}", "{'a': 3, 'b': 2}"),
            ("-0", "0"),
            ("-0.0", "-0.0"),
            ("+1", "1"),
            (".5", "0.5"),
            ("1.", "1.0"),
            ("[y, n]", "['y', 'n']"),
        ] {
            assert_eq!(repr_yaml(t).as_deref(), Some(python), "{t}");
        }
        let riesig = riesig();
        let riesig_text = format!("'{riesig}'");
        for (t, rust, python) in [
            (
                "18446744073709551616",
                Some("1.8446744073709552e+19"),
                "18446744073709551616",
            ),
            (
                "-9223372036854775809",
                Some("-9.223372036854776e+18"),
                "-9223372036854775809",
            ),
            (
                "340282366920938463463374607431768211456",
                Some("3.402823669209385e+38"),
                "340282366920938463463374607431768211456",
            ),
            (&riesig, Some(riesig_text.as_str()), "1000…0"),
            ("1e16", Some("1e+16"), "'1e16'"),
            ("1e+16", Some("1e+16"), "'1e+16'"),
            ("1e-05", Some("1e-05"), "'1e-05'"),
            ("1_000", Some("'1_000'"), "1000"),
            ("017", Some("'017'"), "15"),
            ("0o17", Some("15"), "'0o17'"),
            (
                "[yes, no, on, off]",
                Some("['yes', 'no', 'on', 'off']"),
                "[True, False, True, False]",
            ),
            ("{1: a}", Some("{'1': 'a'}"), "{1: 'a'}"),
            ("{true: a}", Some("{'true': 'a'}"), "{True: 'a'}"),
            ("{~: a}", Some("{'~': 'a'}"), "{None: 'a'}"),
            (r#""\ud800""#, None, r"'\ud800'"),
        ] {
            let ist = repr_yaml(t);
            assert_eq!(ist.as_deref(), rust, "{t}");
            assert_ne!(ist.as_deref(), Some(python), "{t}");
        }
    }

    /// D1: was `serde_json::Value` aus YAML nicht laden kann, laedt `PyWert` (K1-Inventur D1/D2).
    #[test]
    fn yaml_laedt_was_value_nicht_laedt() {
        let t = "[18446744073709551616, .nan]";
        assert!(serde_yaml_ng::from_str::<serde_json::Value>(t).is_err());
        assert_eq!(
            repr_yaml(t).as_deref(),
            Some("[1.8446744073709552e+19, nan]")
        );
        let v: serde_json::Value = serde_yaml_ng::from_str(".nan").unwrap();
        assert!(v.is_null(), "Value macht aus NaN null: truthy F statt T");
    }

    /// D2, D4: NaN und +-inf, Meldungen wie `str(e)` in 3.12.9 und 3.14.7.
    #[test]
    fn nan_und_inf_wie_cpython() {
        let nan = PyWert::Gleit(f64::NAN);
        assert!(nan.truthy());
        assert_eq!(
            nan.int(),
            Err(PyFehler::WertFehler(
                "cannot convert float NaN to integer".into()
            ))
        );
        for inf in [f64::INFINITY, f64::NEG_INFINITY] {
            let e = PyWert::Gleit(inf).int().unwrap_err();
            assert_eq!(e.to_string(), "cannot convert float infinity to integer");
            assert_eq!(e.python_klasse(), Some("OverflowError"));
        }
        assert_eq!(nan.gt_null(), Ok(false));
        assert!(!nan.py_eq(&nan));
    }

    /// D3, D4: `GrossGanz` vergleicht exakt; `int()` endet an der Rust-Grenze, `int_dezimal` nicht.
    #[test]
    fn gross_ganz() {
        let gross = PyWert::GrossGanz(u64::MAX);
        assert_eq!(gross.gt_null(), Ok(true));
        assert!(gross.py_eq(&json("18446744073709551615")));
        let e = gross.int().unwrap_err();
        assert!(matches!(e, PyFehler::I64Grenze(_)));
        assert_eq!(e.python_klasse(), None);
        assert_eq!(gross.int_dezimal().as_deref(), Ok("18446744073709551615"));
        assert_eq!(gross.repr(), "18446744073709551615");
        assert!(matches!(gross.int_mit_bool(), Err(PyFehler::I64Grenze(_))));
    }

    /// D4: nur Python-Ausnahmen tragen einen Klassennamen.
    #[test]
    fn python_klasse() {
        let text = String::new;
        assert_eq!(
            PyFehler::TypFehler(text()).python_klasse(),
            Some("TypeError")
        );
        assert_eq!(
            PyFehler::WertFehler(text()).python_klasse(),
            Some("ValueError")
        );
        assert_eq!(
            PyFehler::Ueberlauf(text()).python_klasse(),
            Some("OverflowError")
        );
        assert_eq!(PyFehler::I64Grenze(text()).python_klasse(), None);
        assert_eq!(PyFehler::DezimalGrenze(text()).python_klasse(), None);
    }

    /// D5: `int(float)` exakt, gemessen 3.12.9 und 3.14.7 (`python3 -c 'print(int(1e300))'`).
    #[test]
    fn int_dezimal_exakt() {
        assert_eq!(
            PyWert::Gleit(1e300).int_dezimal().as_deref(),
            Ok(
                "1000000000000000052504760255204420248704468581108159154915854115511802457988908\
                195786371375080447864043704443832883878176942523235360430575644792184786706982848\
                387200926575803737830233794788090059368953234970799945081119038967640880074652742\
                780142494579258788820056842838115669472196386865459400540160"
            )
        );
        assert_eq!(
            PyWert::Gleit(1e19).int_dezimal().as_deref(),
            Ok("10000000000000000000")
        );
        assert_eq!(PyWert::Gleit(-0.5).int_dezimal().as_deref(), Ok("0"));
        assert_eq!(PyWert::Gleit(-2.5).int_dezimal().as_deref(), Ok("-2"));
        assert_eq!(PyWert::Bool(true).int_dezimal().as_deref(), Ok("1"));
        for (w, typ) in [
            (PyWert::Null, "NoneType"),
            (PyWert::Liste(vec![]), "list"),
            (PyWert::Objekt(vec![]), "dict"),
        ] {
            assert_eq!(
                w.int_dezimal(),
                Err(PyFehler::TypFehler(format!(
                    "int() argument must be a string, a bytes-like object or a real number, not \
                     '{typ}'"
                )))
            );
        }
    }

    /// D10: `isinstance(x, int)`: ein Float ist kein `int`, auch wenn er ganzzahlig ist.
    #[test]
    fn int_mit_und_ohne_bool() {
        for w in [PyWert::Gleit(2.0), PyWert::Text("2".into()), PyWert::Null] {
            assert_eq!(w.int_mit_bool(), Ok(None));
            assert_eq!(w.int_ohne_bool(), Ok(None));
        }
        assert_eq!(PyWert::Bool(false).int_mit_bool(), Ok(Some(0)));
        assert_eq!(PyWert::Bool(false).int_ohne_bool(), Ok(None));
        assert_eq!(PyWert::Ganz(-3).int_ohne_bool(), Ok(Some(-3)));
        assert!(PyWert::Ganz(1).zahl_ohne_bool().is_some());
        assert!(PyWert::GrossGanz(u64::MAX).zahl_ohne_bool().is_some());
        assert!(PyWert::Text("1".into()).zahl_ohne_bool().is_none());
    }

    /// D11, D12: die untere `i64`-Grenze ist erreichbar, die obere endet in `I64Grenze`.
    #[test]
    fn i64_grenzen() {
        assert_eq!(PyWert::Gleit(-(2f64.powi(63))).int(), Ok(i64::MIN));
        assert_eq!(
            PyWert::Text("-9223372036854775808".into()).int(),
            Ok(i64::MIN)
        );
        assert_eq!(
            PyWert::Text("9223372036854775807".into()).int(),
            Ok(i64::MAX)
        );
        for w in [
            PyWert::Gleit(2f64.powi(63)),
            PyWert::Text("9223372036854775808".into()),
            PyWert::Text("-9223372036854775809".into()),
        ] {
            assert!(matches!(w.int(), Err(PyFehler::I64Grenze(_))), "{w:?}");
        }
    }

    /// D13, D14 und `==` auf Behaeltern, gemessen 3.12.9 und 3.14.7.
    #[test]
    fn py_eq_wie_cpython() {
        let wahr = [
            (PyWert::Bool(false), PyWert::Ganz(0)),
            (PyWert::Bool(true), PyWert::Gleit(1.0)),
            (PyWert::GrossGanz(1 << 63), PyWert::Gleit(2f64.powi(63))),
            (PyWert::Ganz(i64::MIN), PyWert::Gleit(-(2f64.powi(63)))),
            (PyWert::Gleit(-0.0), PyWert::Ganz(0)),
            (json("[1]"), json("[true]")),
            (json(r#"{"a": 1, "b": 2}"#), json(r#"{"b": 2, "a": 1.0}"#)),
            (PyWert::Null, PyWert::Null),
        ];
        for (a, b) in &wahr {
            assert!(a.py_eq(b) && b.py_eq(a), "{a:?} == {b:?}");
        }
        let falsch = [
            // `2**64-1 == float(2**64)`: die alte 1.8e19-Grenze hielt das fuer gleich (D14).
            (PyWert::GrossGanz(u64::MAX), PyWert::Gleit(2f64.powi(64))),
            // `(2**53+1) == float(2**53+1)`
            (PyWert::Ganz((1 << 53) + 1), PyWert::Gleit(2f64.powi(53))),
            (PyWert::Ganz(1), PyWert::Gleit(1.5)),
            (PyWert::Ganz(0), PyWert::Gleit(f64::NAN)),
            (PyWert::GrossGanz(u64::MAX), PyWert::Gleit(f64::INFINITY)),
            (PyWert::Null, PyWert::Ganz(0)),
            (PyWert::Text("1".into()), PyWert::Ganz(1)),
            (json("[]"), json("{}")),
            (json("[1, 2]"), json("[1]")),
            (json(r#"{"a": 1}"#), json(r#"{"a": 1, "b": 2}"#)),
            (json(r#"{"a": 1}"#), json(r#"{"b": 1}"#)),
        ];
        for (a, b) in &falsch {
            assert!(!a.py_eq(b) && !b.py_eq(a), "{a:?} != {b:?}");
        }
    }

    /// ponytail an `py_eq`: `CPython` haelt `yaml.safe_load("[.nan]") == yaml.safe_load("[.nan]")`
    /// fuer wahr (Identitaet desselben NaN-Objekts), `PyWert` nicht.
    #[test]
    fn nan_in_behaeltern_ohne_identitaet() {
        let a: PyWert = serde_yaml_ng::from_str("[.nan]").unwrap();
        assert!(!a.py_eq(&a.clone()));
    }

    #[test]
    fn truthy_und_oder_null() {
        for w in [
            PyWert::Null,
            PyWert::Bool(false),
            PyWert::Ganz(0),
            PyWert::GrossGanz(0),
            PyWert::Gleit(-0.0),
            PyWert::Text(String::new()),
            PyWert::Liste(vec![]),
            PyWert::Objekt(vec![]),
        ] {
            assert!(!w.truthy(), "{w:?}");
            assert!(matches!(w.oder_null(), PyWert::Ganz(0)), "{w:?}");
        }
        for w in [
            PyWert::Bool(true),
            PyWert::Ganz(-1),
            PyWert::GrossGanz(u64::MAX),
            PyWert::Gleit(f64::NAN),
            PyWert::Gleit(5e-324),
            PyWert::Text(" ".into()),
            PyWert::Liste(vec![PyWert::Null]),
            PyWert::Objekt(vec![(String::new(), PyWert::Null)]),
        ] {
            assert!(w.truthy(), "{w:?}");
            assert_eq!(w.oder_null().repr(), w.repr());
        }
    }

    /// `x > 0`: `TypeError`-Text wie `str(e)` in 3.12.9 und 3.14.7.
    #[test]
    fn gt_null_wie_cpython() {
        for (w, typ) in [
            (PyWert::Null, "NoneType"),
            (PyWert::Text("a".into()), "str"),
            (PyWert::Liste(vec![]), "list"),
            (PyWert::Objekt(vec![]), "dict"),
        ] {
            assert_eq!(
                w.gt_null(),
                Err(PyFehler::TypFehler(format!(
                    "'>' not supported between instances of '{typ}' and 'int'"
                )))
            );
        }
        assert_eq!(PyWert::Gleit(5e-324).gt_null(), Ok(true));
        assert_eq!(PyWert::Gleit(-0.0).gt_null(), Ok(false));
        assert_eq!(PyWert::Bool(true).gt_null(), Ok(true));
    }

    /// D18: keine stille Saettigung; Genauigkeit wie im ponytail an `dezimal` gemessen.
    #[test]
    fn dezimal() {
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 7.93e28, -1e29] {
            let e = PyWert::Gleit(x).dezimal().unwrap_err();
            assert!(matches!(e, PyFehler::DezimalGrenze(_)), "{x}");
            assert_eq!(e.python_klasse(), None);
        }
        assert_eq!(
            PyWert::Gleit(7.9e28).dezimal(),
            Ok("78999999999999996926548246528".parse::<Decimal>().unwrap())
        );
        assert_eq!(
            PyWert::Gleit(1.0 / 3.0).dezimal(),
            Ok("0.3333333333333333148296162561".parse::<Decimal>().unwrap())
        );
        assert_eq!(PyWert::Gleit(1e-29).dezimal(), Ok(Decimal::ZERO));
        assert_eq!(PyWert::Bool(true).dezimal(), Ok(Decimal::ONE));
        assert_eq!(
            PyWert::GrossGanz(u64::MAX).dezimal(),
            Ok(Decimal::from(u64::MAX))
        );
        assert_eq!(
            PyWert::Text("1".into()).dezimal(),
            Err(PyFehler::TypFehler("must be real number, not str".into()))
        );
    }

    /// `repr`/`str`, gemessen 3.12.9 und 3.14.7.
    #[test]
    fn repr_und_str() {
        assert_eq!(PyWert::Gleit(1e-5).py_str(), "1e-05");
        assert_eq!(PyWert::Null.py_str(), "None");
        assert_eq!(json(r#"[1, "a"]"#).py_str(), "[1, 'a']");
        assert_eq!(PyWert::Bool(true).py_str(), "True");
        assert_eq!(PyWert::Text("it's".into()).repr(), "\"it's\"");
        assert_eq!(
            json(r#"{"z": {"y": []}, "a": -0.0}"#).repr(),
            "{'z': {'y': []}, 'a': -0.0}"
        );
    }

    /// Die zwei Gleichheiten auf `PyWert` fallen auseinander -- und zwar genau dort, wo
    /// Pythons `True == 1` greift. Ohne diesen Test waere die Doku oben nur ein Kommentar.
    ///
    /// Das abgeleitete `PartialEq` ist STRUKTURELL und gehoert dem Wire-Roundtrip von
    /// `store::Event`/`store::SnapshotFeld`. Jede Wertpruefung gehoert ueber `py_eq`.
    #[test]
    fn strukturelle_gleichheit_und_py_eq_fallen_auseinander() {
        // Die zwei Faelle, in denen Pythons `==` wahr ist und die Struktur es nicht sieht.
        assert_ne!(PyWert::Bool(false), PyWert::Ganz(0));
        assert!(PyWert::Bool(false).py_eq(&PyWert::Ganz(0)));

        assert_ne!(PyWert::Bool(true), PyWert::Ganz(1));
        assert!(PyWert::Bool(true).py_eq(&PyWert::Ganz(1)));

        // Und die Faelle, in denen beide dasselbe sagen -- damit der Test auch zeigt,
        // dass er nicht einfach immer "auseinander" antwortet.
        assert_eq!(PyWert::Ganz(0), PyWert::Ganz(0));
        assert!(PyWert::Ganz(0).py_eq(&PyWert::Ganz(0)));
        assert_eq!(PyWert::Null, PyWert::Null);
        assert!(PyWert::Null.py_eq(&PyWert::Null));

        // `Text("1")` ist in Python NICHT gleich `1`: beide Gleichheiten sagen hier falsch.
        assert_ne!(PyWert::Text("1".into()), PyWert::Ganz(1));
        assert!(!PyWert::Text("1".into()).py_eq(&PyWert::Ganz(1)));
    }

    /// Auflage 3: NaN/inf sind beim Rueckweg ein FEHLER, kein stiller `null`-Wert.
    #[test]
    fn zu_json_meldet_nan_und_inf_als_fehler() {
        for f in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                matches!(PyWert::Gleit(f).zu_json(), Err(PyFehler::DezimalGrenze(_))),
                "{f} muss ein Fehler sein, kein Wert"
            );
        }
        // Endliche Gleitwerte gehen durch.
        assert_eq!(
            PyWert::Gleit(1.5).zu_json().unwrap(),
            serde_json::json!(1.5)
        );
    }

    /// `Liste`/`Objekt` gehen nach JSON -- gemessen gegen `store.py::materialisiere`, das einen
    /// Listenwert annimmt und einen gueltigen `snapshot_id` liefert.
    #[test]
    fn zu_json_traegt_liste_und_objekt() {
        assert_eq!(
            PyWert::Liste(vec![PyWert::Ganz(1)]).zu_json().unwrap(),
            serde_json::json!([1])
        );
        assert_eq!(
            PyWert::Liste(vec![]).zu_json().unwrap(),
            serde_json::json!([])
        );
        assert_eq!(
            PyWert::Objekt(vec![("a".into(), PyWert::Ganz(1))])
                .zu_json()
                .unwrap(),
            serde_json::json!({"a": 1})
        );
    }

    /// Die Schluessel eines `Objekt` gehen SORTIERT nach JSON, nicht in Einfuegereihenfolge:
    /// `serde_json::Map` ist ohne `preserve_order` eine `BTreeMap`, und `canonical_json` in
    /// Python sortiert ebenfalls (`store.py:24`). Waere es anders, aenderte sich der Hash.
    #[test]
    fn zu_json_sortiert_objekt_schluessel() {
        let verdreht = PyWert::Objekt(vec![
            ("b".into(), PyWert::Ganz(2)),
            ("a".into(), PyWert::Ganz(1)),
        ]);
        let gerade = PyWert::Objekt(vec![
            ("a".into(), PyWert::Ganz(1)),
            ("b".into(), PyWert::Ganz(2)),
        ]);
        assert_eq!(
            verdreht.zu_json().unwrap(),
            serde_json::json!({"a": 1, "b": 2})
        );
        assert_eq!(
            verdreht.zu_json().unwrap(),
            gerade.zu_json().unwrap(),
            "die Einfuegereihenfolge darf den Hash nicht erreichen"
        );
    }

    /// Auflage 3 greift REKURSIV: ein NaN in einer Liste ist ein Fehler, kein stiller Wert.
    /// Die Formen innerhalb tragen dieselbe Grenze wie aussen.
    #[test]
    fn zu_json_meldet_nan_auch_in_liste_und_objekt() {
        let nan = || PyWert::Gleit(f64::NAN);
        assert!(matches!(
            PyWert::Liste(vec![PyWert::Ganz(1), nan()]).zu_json(),
            Err(PyFehler::DezimalGrenze(_))
        ));
        assert!(matches!(
            PyWert::Objekt(vec![("a".into(), nan())]).zu_json(),
            Err(PyFehler::DezimalGrenze(_))
        ));
    }

    /// Die drei Formen, die der reale Bestand traegt (11294/11294), gehen verlustfrei.
    #[test]
    fn zu_json_ist_verlustfrei_fuer_die_bestandsformen() {
        assert_eq!(
            PyWert::Bool(true).zu_json().unwrap(),
            serde_json::json!(true)
        );
        assert_eq!(PyWert::Ganz(-7).zu_json().unwrap(), serde_json::json!(-7));
        assert_eq!(
            PyWert::Text("Maier".into()).zu_json().unwrap(),
            serde_json::json!("Maier")
        );
        assert_eq!(PyWert::Null.zu_json().unwrap(), serde_json::Value::Null);
    }
}
