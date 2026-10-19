//! Die drei Python-Bausteine, die die Quellen-Skripte brauchen: `str.strip()`, `len(str)` und `repr(str)`.
//!
//! Dieselben Hilfen stehen in `llm::py`. Sie sind hier ein zweites Mal geschrieben, weil `llm` `rustls` und damit `ring`
//! (einen C-Bau), `domain` und `store` mitzieht, und `make sources-check` wie das Python-Skript davor ohne C-Werkzeugkette
//! laufen soll. Der Test `tests/py_paritaet.rs` hält beide Fassungen gegeneinander fest (`llm` steht nur unter
//! `[dev-dependencies]`), damit sie nicht still auseinanderlaufen.

/// Pythons Leerraum für `str.strip()` und `str.isspace()`: Unicode-Leerraum plus U+001C bis U+001F.
///
/// ```
/// assert!(quellen::py::ist_leerraum('\x1f'));
/// assert!(quellen::py::ist_leerraum('\u{a0}'));
/// assert!(!quellen::py::ist_leerraum('x'));
/// ```
#[must_use]
pub fn ist_leerraum(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `s.strip()`.
///
/// ```
/// assert_eq!(quellen::py::strip("\x1c a \t"), "a");
/// ```
#[must_use]
pub fn strip(s: &str) -> &str {
    s.trim_matches(ist_leerraum)
}

/// `len(s)`: Zeichen (Codepunkte), nicht Bytes.
///
/// ```
/// assert_eq!(quellen::py::laenge("äb§"), 3);
/// ```
#[must_use]
pub fn laenge(s: &str) -> usize {
    s.chars().count()
}

/// `repr(s)`: `'…'`, oder `"…"`, wenn `'` vorkommt und `"` nicht.
///
/// ```
/// assert_eq!(quellen::py::repr_str("a\nb"), "'a\\nb'");
/// assert_eq!(quellen::py::repr_str("§ 9 'x'"), "\"§ 9 'x'\"");
/// ```
#[must_use]
pub fn repr_str(s: &str) -> String {
    use std::fmt::Write as _;
    let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
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

/// `repr(liste_von_str)`: `['a', 'b']`, leer `[]`.
///
/// ```
/// assert_eq!(quellen::py::repr_liste(&["a".to_string(), "b".to_string()]), "['a', 'b']");
/// assert_eq!(quellen::py::repr_liste(&[]), "[]");
/// ```
#[must_use]
pub fn repr_liste(liste: &[String]) -> String {
    format!("[{}]", liste.iter().map(|s| repr_str(s)).collect::<Vec<_>>().join(", "))
}

// ponytail: `str.isprintable()` ohne Unicode-Tabelle, wie `llm::py::ist_druckbar`: Steuerzeichen, Leerraum ausser dem
// Leerzeichen und die unsichtbaren Format-Zeichen (U+00AD, U+200B bis U+200F, U+2028 bis U+202E, U+2060 bis U+2064, U+FEFF)
// gelten als nicht druckbar. Grenze: ein anderes unbelegtes oder privates Zeichen in einem `--erwarte`-Text würde anders
// geschrieben als in Python (`erwartete_anker` der PDF-Metadaten). Ausbau: eine Tabelle der Kategorien `Cc Cf Cs Co Cn Zl Zp Zs`.
fn ist_druckbar(c: char) -> bool {
    if c == ' ' {
        return true;
    }
    !(c.is_control()
        || c.is_whitespace()
        || matches!(c, '\u{ad}' | '\u{200b}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2060}'..='\u{2064}' | '\u{feff}'))
}
