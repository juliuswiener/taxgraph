//! Python-Semantik, wo Rust sie anders definiert — die Grundlage jeder Paritaet in `llm` und
//! `eingang`.
//!
//! - `\b`, `\w`, `\s` in Pythons `re` (Unicode-`str`-Muster) sind NICHT die der `regex`-Crate:
//!   Python-Wortzeichen = `str.isalnum()` oder `_` = `\p{L}\p{N}_`; Rust nimmt Marken (`\p{M}`)
//!   und Verbinder (`\p{Pc}`) dazu und laesst `\p{No}` (`²`, `½`) weg. Pythons `\s` = `str.isspace()`
//!   enthaelt `\x1c`–`\x1f`, Rusts `White_Space` nicht. [`PyRegex`] uebersetzt die drei Klassen
//!   deshalb in explizite Klassen bzw. Lookarounds (`fancy-regex`, Backtracking wie Python).
//! - `str.split()`, `str.strip()`, `str.splitlines()` folgen derselben Leerraum-Definition.
//! - `str()`/`repr()` eines JSON-Werts fuer Prompt-Texte und Kuerzungen.
use serde_json::Value;

const W: &str = r"[\p{L}\p{N}_]";
const S: &str = r"[\s\x1c-\x1f]";

/// Pythons `\b` als Lookaround ueber [`W`].
fn wortgrenze() -> String {
    format!("(?:(?<={W})(?!{W})|(?<!{W})(?={W}))")
}

/// Ein nach Python-Semantik uebersetztes Muster. Uebersetzt werden `\b`, `\w`, `\s` AUSSERHALB
/// von Zeichenklassen (die Muster dieses Hauses fuehren sie nie innerhalb).
///
/// FAIL-CLOSED: Ein Laufzeitfehler (Backtracking-Grenze von `fancy-regex`) liefert `None`; jeder
/// Aufrufer entscheidet dann zur sicheren Seite (PII: alles maskieren; Gate: verwerfen).
#[derive(Debug)]
pub struct PyRegex(Option<fancy_regex::Regex>);

impl PyRegex {
    /// Uebersetzt und kompiliert `muster`. `(?i)` am Anfang wie `re.I`.
    ///
    /// ```
    /// let r = llm::py::PyRegex::neu(r"\bab\b");
    /// assert_eq!(r.sucht("x ab y"), Some(true));
    /// assert_eq!(r.sucht("xab"), Some(false));
    /// ```
    #[must_use]
    pub fn neu(muster: &str) -> Self {
        let b = wortgrenze();
        let mut out = String::with_capacity(muster.len() * 2);
        let mut zeichen = muster.chars().peekable();
        while let Some(c) = zeichen.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match zeichen.next() {
                Some('b') => out.push_str(&b),
                Some('w') => out.push_str(W),
                Some('s') => out.push_str(S),
                Some(anderes) => {
                    out.push('\\');
                    out.push(anderes);
                }
                None => out.push('\\'),
            }
        }
        Self(fancy_regex::Regex::new(&out).ok())
    }

    /// `pattern.search(text) is not None`; `None` bei einem Laufzeitfehler.
    ///
    /// ```
    /// assert_eq!(llm::py::PyRegex::neu(r"a\s+b").sucht("a\x1cb"), Some(true));
    /// ```
    #[must_use]
    pub fn sucht(&self, text: &str) -> Option<bool> {
        self.0.as_ref()?.is_match(text).ok()
    }

    /// `pattern.findall(text)` ohne Gruppen: alle nicht ueberlappenden Treffer.
    ///
    /// ```
    /// let r = llm::py::PyRegex::neu(r"\d+");
    /// assert_eq!(r.finde_alle("a1 b22"), Some(vec!["1".to_string(), "22".to_string()]));
    /// ```
    #[must_use]
    pub fn finde_alle(&self, text: &str) -> Option<Vec<String>> {
        let re = self.0.as_ref()?;
        re.find_iter(text)
            .map(|m| m.ok().map(|m| m.as_str().to_owned()))
            .collect()
    }

    /// `pattern.match(text)` bzw. `pattern.search(text)`: Gruppen des ersten Treffers.
    /// `Some(None)` = kein Treffer, `None` = Laufzeitfehler.
    ///
    /// ```
    /// let r = llm::py::PyRegex::neu(r"^(\d)(x)?");
    /// assert_eq!(r.gruppen("1y"), Some(Some(vec![Some("1".into()), None])));
    /// ```
    #[must_use]
    pub fn gruppen(&self, text: &str) -> Option<Option<Vec<Option<String>>>> {
        let re = self.0.as_ref()?;
        let caps = re.captures(text).ok()?;
        Some(caps.map(|c| {
            c.iter()
                .skip(1)
                .map(|g| g.map(|m| m.as_str().to_owned()))
                .collect()
        }))
    }

    /// `pattern.subn(ersatz, text)`: `(neuer_text, anzahl)`; `ersatz` bekommt die Gruppen
    /// (Index 0 = ganzer Treffer).
    ///
    /// ```
    /// let r = llm::py::PyRegex::neu(r"(\d)\d+");
    /// let (t, n) = r.ersetze("a 123 b 45", |g| format!("{}*", g[0].as_deref().unwrap_or(""))).unwrap();
    /// assert_eq!((t.as_str(), n), ("a 1* b 4*", 2));
    /// ```
    pub fn ersetze(
        &self,
        text: &str,
        ersatz: impl Fn(&[Option<String>]) -> String,
    ) -> Option<(String, usize)> {
        let re = self.0.as_ref()?;
        let mut out = String::with_capacity(text.len());
        let mut letzte = 0;
        let mut n = 0;
        for caps in re.captures_iter(text) {
            let caps = caps.ok()?;
            let ganz = caps.get(0)?;
            let gruppen: Vec<Option<String>> = caps
                .iter()
                .skip(1)
                .map(|g| g.map(|m| m.as_str().to_owned()))
                .collect();
            out.push_str(text.get(letzte..ganz.start())?);
            out.push_str(&ersatz(&gruppen));
            letzte = ganz.end();
            n += 1;
        }
        out.push_str(text.get(letzte..)?);
        Some((out, n))
    }
}

/// Pythons Wortzeichen (`\w` fuer `str`): `isalnum()` oder `_` = `\p{L}\p{N}_`.
///
/// ```
/// assert!(llm::py::ist_wortzeichen('²'));
/// assert!(!llm::py::ist_wortzeichen('\u{301}'));
/// ```
#[must_use]
pub fn ist_wortzeichen(c: char) -> bool {
    static WORT: std::sync::LazyLock<Option<regex::Regex>> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[\p{L}\p{N}_]$").ok());
    let mut puffer = [0u8; 4];
    // Kompiliert das Muster nicht (unerreichbar, Test haelt es fest), gilt jedes Zeichen als
    // Wortzeichen — kurze Belege fallen dann durch das Gate (fail-closed).
    WORT.as_ref()
        .is_none_or(|r| r.is_match(c.encode_utf8(&mut puffer)))
}

/// Ist `c` eine Unicode-Dezimalziffer (`\p{Nd}`, Pythons `str.isdecimal()`)?
fn ist_nd(c: char) -> bool {
    static ND: std::sync::LazyLock<Option<regex::Regex>> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^\p{Nd}$").ok());
    let mut puffer = [0u8; 4];
    ND.as_ref()
        .is_some_and(|r| r.is_match(c.encode_utf8(&mut puffer)))
}

/// Wert einer Dezimalziffer. Unicode legt `Nd`-Zeichen in lueckenlosen Zehnerfolgen 0–9 ab, und
/// aneinanderstossende Folgen beginnen jeweils bei 0 — der Abstand zum Anfang des
/// zusammenhaengenden `Nd`-Bereichs modulo 10 ist also der Ziffernwert.
fn ziffernwert(c: char) -> Option<u32> {
    if c.is_ascii_digit() {
        return c.to_digit(10);
    }
    if !ist_nd(c) {
        return None;
    }
    let mut start = u32::from(c);
    while let Some(davor) = start
        .checked_sub(1)
        .and_then(char::from_u32)
        .filter(|d| ist_nd(*d))
    {
        start = u32::from(davor);
    }
    Some((u32::from(c) - start) % 10)
}

/// Bildet jede Unicode-Dezimalziffer auf ihre ASCII-Ziffer ab — wie Pythons `int()`, `float()`
/// und `Decimal()` sie lesen (`float('-١٢,٠٠'.replace(',', '.'))` = -12.0).
///
/// ```
/// assert_eq!(llm::py::ascii_ziffern("-١٢,٠٠"), "-12,00");
/// assert_eq!(llm::py::ascii_ziffern("x²"), "x²");
/// ```
#[must_use]
pub fn ascii_ziffern(s: &str) -> std::borrow::Cow<'_, str> {
    if s.is_ascii() {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(
        s.chars()
            .map(|c| {
                ziffernwert(c)
                    .and_then(|w| char::from_digit(w, 10))
                    .unwrap_or(c)
            })
            .collect(),
    )
}

/// `" ".join((s or "").lower().split())` — `_normalisiert` (`api_llm.py:360-364`).
///
/// ```
/// assert_eq!(llm::py::normalisiert("  Ich  FAHRE\n20 km "), "ich fahre 20 km");
/// ```
#[must_use]
pub fn normalisiert(s: &str) -> String {
    split_leer(&s.to_lowercase()).join(" ")
}

/// `str.isspace()` fuer ein Zeichen.
///
/// ```
/// assert!(llm::py::ist_leerraum('\x1f'));
/// assert!(!llm::py::ist_leerraum('x'));
/// ```
#[must_use]
pub fn ist_leerraum(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `s.strip()`.
///
/// ```
/// assert_eq!(llm::py::strip("\x1c a \t"), "a");
/// ```
#[must_use]
pub fn strip(s: &str) -> &str {
    s.trim_matches(ist_leerraum)
}

/// `s.split()` ohne Argument.
///
/// ```
/// assert_eq!(llm::py::split_leer(" a\x1db  c "), vec!["a", "b", "c"]);
/// ```
#[must_use]
pub fn split_leer(s: &str) -> Vec<&str> {
    s.split(ist_leerraum).filter(|t| !t.is_empty()).collect()
}

/// `s.splitlines()`: trennt an `\n \r \r\n \v \f \x1c \x1d \x1e \x85    `, ohne
/// abschliessende Leerzeile.
///
/// ```
/// assert_eq!(llm::py::splitlines("a\r\nb\x0bc\n"), vec!["a", "b", "c"]);
/// ```
#[must_use]
pub fn splitlines(s: &str) -> Vec<&str> {
    let ist_umbruch = |c: char| {
        matches!(
            c,
            '\n' | '\r'
                | '\x0b'
                | '\x0c'
                | '\x1c'
                | '\x1d'
                | '\x1e'
                | '\u{85}'
                | '\u{2028}'
                | '\u{2029}'
        )
    };
    let mut out = Vec::new();
    let mut start = 0;
    let mut iter = s.char_indices().peekable();
    while let Some((i, c)) = iter.next() {
        if ist_umbruch(c) {
            out.push(s.get(start..i).unwrap_or(""));
            let mut ende = i + c.len_utf8();
            if c == '\r' {
                if let Some(&(j, '\n')) = iter.peek() {
                    ende = j + 1;
                    iter.next();
                }
            }
            start = ende;
        }
    }
    if start < s.len() {
        out.push(s.get(start..).unwrap_or(""));
    }
    out
}

/// `s[:n]` in Codepunkten.
///
/// ```
/// assert_eq!(llm::py::vorne("äöü", 2), "äö");
/// ```
#[must_use]
pub fn vorne(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((i, _)) => s.get(..i).unwrap_or(s),
        None => s,
    }
}

/// `len(s)` in Codepunkten.
///
/// ```
/// assert_eq!(llm::py::laenge("äb"), 2);
/// ```
#[must_use]
pub fn laenge(s: &str) -> usize {
    s.chars().count()
}

/// Pythons Wahrheitswert.
///
/// ```
/// assert!(!llm::py::wahr(&serde_json::json!([])));
/// assert!(llm::py::wahr(&serde_json::json!("0")));
/// ```
#[must_use]
pub fn wahr(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// `str(x)` eines JSON-Werts, wie `json.loads` ihn geliefert haette.
///
/// ```
/// use serde_json::json;
/// assert_eq!(llm::py::py_str(&json!("a")), "a");
/// assert_eq!(llm::py::py_str(&json!(1.0)), "1.0");
/// assert_eq!(llm::py::py_str(&json!(true)), "True");
/// assert_eq!(llm::py::py_str(&json!(null)), "None");
/// ```
#[must_use]
pub fn py_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        andere => py_repr(andere),
    }
}

/// `repr(x)` eines JSON-Werts. PARITAET-Grenze: Objekte erscheinen in `serde_json`-Ordnung
/// (sortiert), Python zeigt die Einfuege-Reihenfolge — siehe [`GeordneteMap`] fuer Stellen,
/// an denen die Reihenfolge in einen Prompt geht.
///
/// ```
/// use serde_json::json;
/// assert_eq!(llm::py::py_repr(&json!(["a", 1, null])), "['a', 1, None]");
/// assert_eq!(llm::py::py_repr(&json!("it's")), "\"it's\"");
/// ```
#[must_use]
pub fn py_repr(v: &Value) -> String {
    match v {
        Value::Null => "None".into(),
        Value::Bool(true) => "True".into(),
        Value::Bool(false) => "False".into(),
        Value::Number(n) => match (n.as_i64(), n.as_u64(), n.as_f64()) {
            (Some(i), _, _) => i.to_string(),
            (None, Some(u), _) => u.to_string(),
            (None, None, Some(f)) => py_float_repr(f),
            _ => n.to_string(),
        },
        Value::String(s) => py_repr_str(s),
        Value::Array(a) => format!("[{}]", a.iter().map(py_repr).collect::<Vec<_>>().join(", ")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!("{}: {}", py_repr_str(k), py_repr(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// `repr(s)` einer Zeichenkette: `'…'`, oder `"…"`, wenn `'` vorkommt und `"` nicht.
///
/// ```
/// assert_eq!(llm::py::py_repr_str("a\nb"), "'a\\nb'");
/// ```
#[must_use]
pub fn py_repr_str(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if ist_druckbar(c) => out.push(c),
            c => {
                use std::fmt::Write as _;
                let n = u32::from(c);
                let _ = if n <= 0xff {
                    write!(out, "\\x{n:02x}")
                } else if n <= 0xffff {
                    write!(out, "\\u{n:04x}")
                } else {
                    write!(out, "\\U{n:08x}")
                };
            }
        }
    }
    out.push(quote);
    out
}

/// `str.isprintable()` fuer ein Zeichen — Naeherung: Steuerzeichen (`Cc`), Trenner ausser dem
/// Leerzeichen (`Zl`, `Zp`, `Zs` ohne U+0020) und Format-/Surrogat-/unbelegte Zeichen sind nicht
/// druckbar. Rust kennt die Unicode-Kategorie nicht ohne Tabelle; abgedeckt sind die Faelle,
/// die in Modellantworten und Bindungstexten vorkommen (PARITAET-Grenze, Bericht).
fn ist_druckbar(c: char) -> bool {
    if c == ' ' {
        return true;
    }
    !(c.is_control()
        || c.is_whitespace()
        || matches!(c, '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{feff}'))
}

/// `repr(float)` in Pythons Kurzform: kuerzeste Ziffernfolge, Festkomma fuer Exponenten
/// `-4 <= e < 16`, sonst `d.ddde+XX`.
///
/// ```
/// assert_eq!(llm::py::py_float_repr(1e16), "1e+16");
/// assert_eq!(llm::py::py_float_repr(1e15), "1000000000000000.0");
/// assert_eq!(llm::py::py_float_repr(1.5e-7), "1.5e-07");
/// assert_eq!(llm::py::py_float_repr(0.0001), "0.0001");
/// ```
#[must_use]
pub fn py_float_repr(f: f64) -> String {
    if f.is_nan() {
        return "nan".into();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let wiss = format!("{f:e}");
    let (mantisse, exp) = wiss.split_once('e').unwrap_or((&wiss, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (vorzeichen, mantisse) = mantisse
        .strip_prefix('-')
        .map_or(("", mantisse), |m| ("-", m));
    let ziffern: String = mantisse.chars().filter(char::is_ascii_digit).collect();
    if (-4..16).contains(&exp) {
        let punkt = exp + 1;
        let fest = if punkt <= 0 {
            format!(
                "0.{}{}",
                "0".repeat(usize::try_from(-punkt).unwrap_or(0)),
                ziffern
            )
        } else {
            let p = usize::try_from(punkt).unwrap_or(0);
            if ziffern.len() <= p {
                format!("{}{}.0", ziffern, "0".repeat(p - ziffern.len()))
            } else {
                format!(
                    "{}.{}",
                    ziffern.get(..p).unwrap_or(""),
                    ziffern.get(p..).unwrap_or("")
                )
            }
        };
        format!("{vorzeichen}{fest}")
    } else {
        let (kopf, rest) = ziffern.split_at(1.min(ziffern.len()));
        let m = if rest.is_empty() {
            kopf.to_owned()
        } else {
            format!("{kopf}.{rest}")
        };
        let e = if exp < 0 {
            format!("-{:02}", -exp)
        } else {
            format!("+{exp:02}")
        };
        format!("{vorzeichen}{m}e{e}")
    }
}

/// Pythons `int(x)` fuer einen JSON-Wert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PyInt {
    Wert(i64),
    /// `ValueError` (auch `int(nan)`, Text ohne Zahl).
    WertFehler,
    /// `TypeError` (`None`, Liste, Objekt).
    TypFehler,
    /// `OverflowError` (`int(inf)`) oder ausserhalb von `i64` (Python: grosse Ganzzahl).
    Ueberlauf,
}

/// `int(x)`.
///
/// ```
/// use llm::py::{py_int, PyInt};
/// use serde_json::json;
/// assert_eq!(py_int(&json!(" 7 ")), PyInt::Wert(7));
/// assert_eq!(py_int(&json!(2.9)), PyInt::Wert(2));
/// assert_eq!(py_int(&json!("2.9")), PyInt::WertFehler);
/// assert_eq!(py_int(&json!(null)), PyInt::TypFehler);
/// ```
#[must_use]
pub fn py_int(v: &Value) -> PyInt {
    match v {
        Value::Bool(b) => PyInt::Wert(i64::from(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                PyInt::Wert(i)
            } else if n.as_u64().is_some() {
                PyInt::Ueberlauf
            } else {
                n.as_f64().map_or(PyInt::WertFehler, py_int_float)
            }
        }
        Value::String(s) => py_int_text(s),
        _ => PyInt::TypFehler,
    }
}

#[allow(clippy::cast_possible_truncation)] // Bereich vorher geprueft; `trunc` ist ganzzahlig
fn py_int_float(f: f64) -> PyInt {
    if f.is_nan() {
        PyInt::WertFehler
    } else if f.is_infinite() || f.trunc().abs() >= 9.223_372_036_854_776e18 {
        PyInt::Ueberlauf
    } else {
        PyInt::Wert(f.trunc() as i64)
    }
}

/// `int(s)` fuer Text: Leerraum aussen, ein Vorzeichen, Dezimalziffern (jede Schrift, s.
/// [`ascii_ziffern`]) mit einzelnen `_` zwischen Ziffern.
///
/// ```
/// assert_eq!(llm::py::py_int_text("-1_0"), llm::py::PyInt::Wert(-10));
/// assert_eq!(llm::py::py_int_text("1__0"), llm::py::PyInt::WertFehler);
/// ```
#[must_use]
pub fn py_int_text(s: &str) -> PyInt {
    let s = ascii_ziffern(strip(s));
    let s = s.as_ref();
    let rumpf = s.strip_prefix(['+', '-']).unwrap_or(s);
    let ok = !rumpf.is_empty()
        && !rumpf.starts_with('_')
        && !rumpf.ends_with('_')
        && !rumpf.contains("__")
        && rumpf.bytes().all(|b| b.is_ascii_digit() || b == b'_');
    if !ok {
        return PyInt::WertFehler;
    }
    s.replace('_', "")
        .parse::<i64>()
        .map_or(PyInt::Ueberlauf, PyInt::Wert)
}

/// Eine JSON-Objekt-Form, die ihre Quell-Reihenfolge behaelt (fuer `repr(dict)` in Prompts;
/// `serde_json::Map` sortiert ohne das Feature `preserve_order`).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize)]
#[serde(transparent)]
pub struct GeordneteMap(#[serde(serialize_with = "ser_paare")] pub Vec<(String, Value)>);

fn ser_paare<S: serde::Serializer>(p: &[(String, Value)], s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut m = s.serialize_map(Some(p.len()))?;
    for (k, v) in p {
        m.serialize_entry(k, v)?;
    }
    m.end()
}

impl GeordneteMap {
    /// `repr(dict)` in Quell-Reihenfolge.
    ///
    /// ```
    /// let m = llm::py::GeordneteMap(vec![("min".into(), 0.into()), ("max".into(), 5.into())]);
    /// assert_eq!(m.py_repr(), "{'min': 0, 'max': 5}");
    /// ```
    #[must_use]
    pub fn py_repr(&self) -> String {
        // Ein `dict` fuehrt jeden Schluessel einmal. `visit_map` faltet doppelte zusammen, wer
        // das offene Feld direkt fuellt, nicht.
        debug_assert_eq!(
            self.0
                .iter()
                .map(|(k, _)| k)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            self.0.len()
        );
        format!(
            "{{{}}}",
            self.0
                .iter()
                .map(|(k, v)| format!("{}: {}", py_repr_str(k), py_repr(v)))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

impl<'de> serde::Deserialize<'de> for GeordneteMap {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Besucher;
        impl<'de> serde::de::Visitor<'de> for Besucher {
            type Value = GeordneteMap;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("ein JSON-Objekt")
            }
            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut a: A,
            ) -> Result<GeordneteMap, A::Error> {
                let mut paare = Vec::new();
                while let Some((k, v)) = a.next_entry::<String, Value>()? {
                    match paare
                        .iter_mut()
                        .find(|(alt, _): &&mut (String, Value)| *alt == k)
                    {
                        Some(eintrag) => eintrag.1 = v,
                        None => paare.push((k, v)),
                    }
                }
                Ok(GeordneteMap(paare))
            }
        }
        d.deserialize_map(Besucher)
    }
}

#[cfg(test)]
mod tests {
    use super::{py_float_repr, splitlines, PyRegex};

    #[test]
    fn wortgrenze_wie_python_bei_hochzahl_und_marke() {
        // Python: '²'.isalnum() -> True, also keine Grenze zwischen '5' und '²'.
        assert_eq!(PyRegex::neu(r"\b5\b").sucht("5²"), Some(false));
        // Python: U+0301 (Marke) ist kein Wortzeichen -> Grenze.
        assert_eq!(PyRegex::neu(r"\ba\b").sucht("a\u{301}"), Some(true));
    }

    #[test]
    fn float_repr_randfaelle() {
        assert_eq!(py_float_repr(100.0), "100.0");
        assert_eq!(py_float_repr(-0.5), "-0.5");
        assert_eq!(py_float_repr(1.5e300), "1.5e+300");
        assert_eq!(py_float_repr(123_456.789), "123456.789");
    }

    #[test]
    fn splitlines_ohne_letzte_leerzeile() {
        assert_eq!(splitlines(""), Vec::<&str>::new());
        assert_eq!(splitlines("\n"), vec![""]);
        assert_eq!(splitlines("a\n\nb"), vec!["a", "", "b"]);
    }
}

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{
        ganzzahl_text, int_ausnahmen, json_wert, klasse, pruefe, py, py_absteigend, text, Ergebnis,
    };
    use domain::{py_strip, PyWert};
    use proptest::collection::{btree_map, vec};
    use proptest::prelude::*;
    use serde_json::{json, Map, Value};

    use super::{
        py_float_repr, py_int, py_repr, py_repr_str, py_str, strip, wahr, GeordneteMap, PyInt,
    };

    /// Ausnahmen von `py_int` und `py_int_text`.
    const INT: &[&str] = &["D11", "D16", "D17"];
    /// Ausnahmen von `py_repr`, `py_str` und `GeordneteMap::py_repr`.
    const REPR: &[&str] = &["D8"];

    /// `Ueberlauf` ist hier die i64-Grenze: `int(inf)` erreicht ein JSON-Wert nicht.
    fn alt_int(v: &Value) -> Ergebnis<i64> {
        match py_int(v) {
            PyInt::Wert(i) => Ok(i),
            PyInt::WertFehler => Err(Some("ValueError")),
            PyInt::TypFehler => Err(Some("TypeError")),
            PyInt::Ueberlauf => Err(None),
        }
    }

    fn int_wie(v: &Value) -> Result<(), TestCaseError> {
        let (alt, neu) = (alt_int(v), klasse(py(v).int()));
        pruefe(v, &alt, &neu, || int_ausnahmen(v, &alt, &neu), INT)
    }

    /// D8: gleich, sobald `PyWert` die Reihenfolge des Alt-Helfers hat.
    fn d8(alt: &str, gleich_geordnet: &str) -> Vec<&'static str> {
        if alt == gleich_geordnet {
            vec!["D8"]
        } else {
            Vec::new()
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn wahr_wie_pywert(v in json_wert()) {
            pruefe(&v, &wahr(&v), &py(&v).truthy(), Vec::new, &[])?;
        }

        #[test]
        fn strip_wie_pywert(s in text()) {
            pruefe(&s, &strip(&s), &py_strip(&s), Vec::new, &[])?;
        }

        #[test]
        fn int_wie_pywert(v in json_wert()) {
            int_wie(&v)?;
        }

        #[test]
        fn int_text_wie_pywert(s in ganzzahl_text()) {
            int_wie(&Value::String(s))?;
        }

        #[test]
        fn repr_wie_pywert(v in json_wert()) {
            let alt = py_repr(&v);
            pruefe(&v, &alt, &py_absteigend(&v).repr(), || d8(&alt, &py(&v).repr()), REPR)?;
        }

        #[test]
        fn str_wie_pywert(v in json_wert()) {
            let alt = py_str(&v);
            pruefe(&v, &alt, &py_absteigend(&v).py_str(), || d8(&alt, &py(&v).py_str()), REPR)?;
        }

        #[test]
        fn repr_str_wie_pywert(s in text()) {
            pruefe(&s, &py_repr_str(&s), &PyWert::Text(s.clone()).repr(), Vec::new, &[])?;
        }

        #[test]
        fn float_repr_wie_pywert(f in any::<f64>()) {
            pruefe(&f, &py_float_repr(f), &PyWert::Gleit(f).repr(), Vec::new, &[])?;
        }

        /// Beide aussen absteigend; `GeordneteMap` haelt innen die sortierte Reihenfolge.
        #[test]
        fn geordnete_map_wie_pywert(m in btree_map(text(), json_wert(), 0..4)) {
            let alt = GeordneteMap(m.iter().rev().map(|(k, w)| (k.clone(), w.clone())).collect())
                .py_repr();
            let innen_sortiert = PyWert::Objekt(m.iter().rev().map(|(k, w)| (k.clone(), py(w))).collect());
            let v = Value::Object(m.into_iter().collect::<Map<_, _>>());
            pruefe(&v, &alt, &py_absteigend(&v).repr(), || d8(&alt, &innen_sortiert.repr()), REPR)?;
        }

        /// Doppelter Schluessel wie `json.loads`: der letzte Wert gilt, am Platz des ersten.
        #[test]
        fn geordnete_map_doppelt_wie_pywert(paare in vec(("[ab]", json_wert()), 0..6)) {
            let datei = format!(
                "{{{}}}",
                paare.iter().map(|(k, w)| format!("{}: {w}", json!(k))).collect::<Vec<_>>().join(", ")
            );
            let alt = serde_json::from_str::<GeordneteMap>(&datei).unwrap().py_repr();
            let neu = serde_json::from_str::<PyWert>(&datei).unwrap().repr();
            pruefe(&datei, &alt, &neu, Vec::new, &[])?;
        }
    }

    /// D8: `repr(dict)` folgt in `CPython` auf jeder Ebene der Reihenfolge der Datei.
    /// `GeordneteMap` haelt sie nur aussen, `py_repr` sortiert.
    #[test]
    fn d8_reihenfolge_der_datei() {
        let datei = r#"{"z": {"b": 1, "a": 2}, "y": 0}"#;
        let m: GeordneteMap = serde_json::from_str(datei).unwrap();
        assert_eq!(m.py_repr(), "{'z': {'a': 2, 'b': 1}, 'y': 0}");
        let v: Value = serde_json::from_str(datei).unwrap();
        assert_eq!(py_repr(&v), "{'y': 0, 'z': {'a': 2, 'b': 1}}");
        let w: PyWert = serde_json::from_str(datei).unwrap();
        assert_eq!(w.repr(), "{'z': {'b': 1, 'a': 2}, 'y': 0}");
    }

    /// D11: `int(-2.0**63)` ist in `CPython` `i64::MIN`.
    #[test]
    fn d11_minus_2_hoch_63() {
        let v = json!(-9_223_372_036_854_775_808.0);
        assert_eq!(alt_int(&v), Err(None));
        assert_eq!(klasse(py(&v).int()), Ok(i64::MIN));
    }

    /// D16: `int("\x1c42")` wirft in `CPython` `ValueError`, obwohl `str.strip()` U+001C entfernt.
    #[test]
    fn d16_steuerzeichen_am_rand() {
        let v = json!("\u{1c}42");
        assert_eq!(alt_int(&v), Ok(42));
        assert_eq!(klasse(py(&v).int()), Err(Some("ValueError")));
    }

    /// D17: `int()` mit mehr als 4300 Ziffern wirft in `CPython` `ValueError`.
    #[test]
    fn d17_mehr_als_4300_ziffern() {
        let nullen = json!(format!("{}5", "0".repeat(4300)));
        let sieben = json!("7".repeat(4301));
        assert_eq!(alt_int(&nullen), Ok(5));
        assert_eq!(alt_int(&sieben), Err(None));
        for v in [nullen, sieben] {
            assert_eq!(klasse(py(&v).int()), Err(Some("ValueError")));
        }
    }
}
