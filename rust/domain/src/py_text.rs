//! Pythons Textsemantik fuer [`crate::PyWert`]: `str.strip()`, `int(str)`, `repr(str)` und
//! `repr(float)` -- je eine Umsetzung statt der Kopien in bescheid, elster, llm, auth und api.
use std::fmt::Write as _;
use std::sync::LazyLock;

use crate::PyFehler;

/// `sys.get_int_max_str_digits()` in der Voreinstellung (gemessen `CPython` 3.12.9 und 3.14.7):
/// `int(text)` mit mehr Ziffern wirft `ValueError` (D17).
const INT_MAX_STR_DIGITS: usize = 4300;

/// `str.isspace()`: Unicode-`White_Space` (= `char::is_whitespace`, 25 Zeichen) plus die vier
/// Informationstrenner U+001C..U+001F. 29 Codepunkte, in `CPython` 3.12.9 (Unicode 15.0.0) und
/// 3.14.7 (Unicode 16.0.0) gleich (D7).
fn ist_py_leerraum(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python `s.strip()` ohne Argument (D7).
///
/// ```
/// assert_eq!(domain::py_strip("\u{1c} a\u{3000}"), "a");
/// assert_eq!(domain::py_strip("\u{200b}a"), "\u{200b}a"); // U+200B ist kein Leerraum
/// ```
#[must_use]
pub fn py_strip(s: &str) -> &str {
    s.trim_matches(ist_py_leerraum)
}

/// `int(s)` fuer Text als normierter Dezimaltext (`-` nur bei negativem Wert, ohne fuehrende
/// Nullen), wie `PyLong_FromUnicodeObject`.
///
/// `CPython` kopiert Zeichen unter U+007F unveraendert in einen ASCII-Puffer, macht aus uebrigem
/// Leerraum ein Leerzeichen und aus Nd-Ziffern ASCII-Ziffern, und entfernt danach aussen nur
/// ASCII-Leerraum. Wirksam ist damit genau `str::trim` (25 Zeichen): U+001C..U+001F am Rand sind
/// ein `ValueError`, obwohl `str.strip()` sie entfernt (D16). Danach ein Vorzeichen,
/// Dezimalziffern jeder Schrift (D6) mit einzelnen `_` zwischen Ziffern. Erst nach der Syntax
/// zaehlt die Ziffern-Grenze (D17; fuehrende Nullen zaehlen mit, `_` nicht).
pub(crate) fn int_aus_text(s: &str) -> Result<String, PyFehler> {
    // `%.200R`: `repr` des Originaltexts, auf 200 Zeichen gekuerzt.
    let ungueltig = || {
        PyFehler::WertFehler(format!(
            "invalid literal for int() with base 10: {}",
            repr_str(s).chars().take(200).collect::<String>()
        ))
    };
    let t = s.trim();
    let (negativ, rumpf) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let mut ziffern = String::with_capacity(rumpf.len());
    // Startwert `true`: kein `_` vor der ersten Ziffer, und ein leerer Rumpf scheitert unten.
    let mut nach_trenner = true;
    for c in rumpf.chars() {
        if c == '_' {
            if nach_trenner {
                return Err(ungueltig());
            }
            nach_trenner = true;
        } else {
            ziffern.push(ascii_ziffer(c).ok_or_else(ungueltig)?);
            nach_trenner = false;
        }
    }
    if nach_trenner {
        return Err(ungueltig());
    }
    if ziffern.len() > INT_MAX_STR_DIGITS {
        return Err(PyFehler::WertFehler(format!(
            "Exceeds the limit ({INT_MAX_STR_DIGITS} digits) for integer string conversion: \
             value has {} digits; use sys.set_int_max_str_digits() to increase the limit",
            ziffern.len()
        )));
    }
    let ohne_nullen = ziffern.trim_start_matches('0');
    Ok(if ohne_nullen.is_empty() {
        "0".to_owned()
    } else if negativ {
        format!("-{ohne_nullen}")
    } else {
        ohne_nullen.to_owned()
    })
}

/// Eine Dezimalziffer jeder Schrift (Kategorie Nd, `str.isdecimal()`) als ASCII-Ziffer
/// (Umsetzung aus llm `py.rs:157-184`, D6). Unicode legt Nd-Zeichen in lueckenlosen Zehnerfolgen
/// 0-9 ab, aneinanderstossende Folgen beginnen jeweils bei 0: der Abstand zum Anfang des
/// zusammenhaengenden Nd-Bereichs modulo 10 ist der Ziffernwert.
fn ascii_ziffer(c: char) -> Option<char> {
    if c.is_ascii_digit() {
        return Some(c);
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
    char::from_digit((u32::from(c) - start) % 10, 10)
}

/// `\p{Nd}` nach der Unicode-Tabelle von `regex` (16.0.0 = `CPython` 3.14; `CPython` 3.12 kennt
/// 80 dieser Zeichen noch nicht, s. Test `nd_menge_wie_cpython_3_14`).
pub(crate) fn ist_nd(c: char) -> bool {
    static ND: LazyLock<Option<regex::Regex>> =
        LazyLock::new(|| regex::Regex::new(r"^\p{Nd}$").ok());
    let mut puffer = [0u8; 4];
    ND.as_ref()
        .is_some_and(|r| r.is_match(c.encode_utf8(&mut puffer)))
}

/// `repr(s)`: `'...'`, oder `"..."`, wenn `'` vorkommt und `"` nicht. Umsetzung aus elster
/// `py.rs:218` (llm `py.rs:391` escapet dieselbe Menge, D9).
///
/// ponytail: `str.isprintable()` ist angenaehert. Escaped werden Cc, Zs/Zl/Zp ausser U+0020 und
/// die gaengigen Cf (U+00AD, U+200B..U+200F, U+202A..U+202E, U+2060..U+2064, U+FEFF). Die
/// uebrigen 153 Cf, 137.468 Co und 819.533 (3.14) bzw. 825.345 (3.12) Cn escapet `CPython`, diese
/// Umsetzung nicht (gemessen). Upgrade: Kategorientabelle aus `unicodedata` generieren.
pub(crate) fn repr_str(s: &str) -> String {
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
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if !druckbar(c) => {
                let n = u32::from(c);
                let _ = if n <= 0xff {
                    write!(out, "\\x{n:02x}")
                } else if n <= 0xffff {
                    write!(out, "\\u{n:04x}")
                } else {
                    write!(out, "\\U{n:08x}")
                };
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

pub(crate) fn druckbar(c: char) -> bool {
    if c == ' ' {
        return true;
    }
    !(c.is_control()
        || matches!(
            c,
            '\u{a0}'
                | '\u{ad}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200f}'
                | '\u{2028}'..='\u{202f}'
                | '\u{205f}'..='\u{2064}'
                | '\u{3000}'
                | '\u{feff}'
        ))
}

/// `repr(float)`: kuerzeste Ziffernfolge, Festkomma fuer Exponenten `-4 <= e < 16`, sonst
/// `d.ddde+XX` (Umsetzung aus elster `py.rs:176`).
pub(crate) fn repr_float(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_owned();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    let vorzeichen = if f.is_sign_negative() { "-" } else { "" };
    let e_form = format!("{:e}", f.abs());
    let (mantisse, exp) = e_form.split_once('e').unwrap_or((e_form.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let ziffern: String = mantisse.chars().filter(char::is_ascii_digit).collect();
    let (erste, rest) = ziffern.split_at(1.min(ziffern.len()));
    let rumpf = if (-4..16).contains(&exp) {
        let exp_u = usize::try_from(exp.unsigned_abs()).unwrap_or(0);
        if exp >= 0 {
            if ziffern.len() <= exp_u + 1 {
                format!("{ziffern}{}.0", "0".repeat(exp_u + 1 - ziffern.len()))
            } else {
                let (ganz, bruch) = ziffern.split_at(exp_u + 1);
                format!("{ganz}.{bruch}")
            }
        } else {
            format!("0.{}{ziffern}", "0".repeat(exp_u - 1))
        }
    } else {
        let mant = if rest.is_empty() {
            erste.to_owned()
        } else {
            format!("{erste}.{rest}")
        };
        let zeichen = if exp < 0 { '-' } else { '+' };
        format!("{mant}e{zeichen}{:02}", exp.unsigned_abs())
    };
    format!("{vorzeichen}{rumpf}")
}

#[cfg(test)]
mod tests {
    use super::{ascii_ziffer, int_aus_text, ist_nd, py_strip, repr_float, repr_str};
    use crate::PyFehler;

    /// Alle Codepunkte ausser den Surrogaten.
    fn alle_zeichen() -> impl Iterator<Item = char> {
        (0..=0x10_ffff).filter_map(char::from_u32)
    }

    /// D7: was `str.strip()` entfernt, in `CPython` 3.12.9 und 3.14.7 gleich (= `str.isspace()`).
    /// Erzeugt mit `python3 -c 'print([hex(c) for c in range(0x110000) if not chr(c).strip()])'`.
    const STRIP: [u32; 29] = [
        0x9, 0xa, 0xb, 0xc, 0xd, 0x1c, 0x1d, 0x1e, 0x1f, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001,
        0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029,
        0x202f, 0x205f, 0x3000,
    ];

    /// D16: was `int(text)` aussen entfernt, in `CPython` 3.12.9 und 3.14.7 gleich: `STRIP` ohne
    /// U+001C..U+001F. Erzeugt mit
    /// ```text
    /// def ok(t):
    ///     try: return int(t) == 7
    ///     except ValueError: return False
    /// print([hex(c) for c in range(0x110000) if ok(chr(c) + "7" + chr(c))])
    /// ```
    const INT_LEERRAUM: [u32; 25] = [
        0x9, 0xa, 0xb, 0xc, 0xd, 0x20, 0x85, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004,
        0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000,
    ];

    /// D6: die Null jedes Nd-Blocks (`str.isdecimal()`, je zehn Ziffern 0..9), `CPython` 3.14.7
    /// (Unicode 16.0.0, 760 Zeichen). Erzeugt mit `python3 -c 'import unicodedata as u;
    /// print([hex(c) for c in range(0x110000) if chr(c).isdecimal() and u.decimal(chr(c)) == 0])'`.
    /// `CPython` 3.12.9 (Unicode 15.0.0, 680 Zeichen) fehlen die acht mit `16.0` markierten Bloecke.
    const ND_NULLEN: [u32; 76] = [
        0x30, 0x660, 0x6f0, 0x7c0, 0x966, 0x9e6, 0xa66, 0xae6, 0xb66, 0xbe6, 0xc66, 0xce6, 0xd66,
        0xde6, 0xe50, 0xed0, 0xf20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0, 0x1a80, 0x1a90,
        0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0, 0xaa50, 0xabf0,
        0xff10, 0x104a0, 0x10d30, 0x10d40, // 16.0
        0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0, 0x11450, 0x114d0, 0x11650, 0x116c0, 0x116d0,
        0x116da, // 16.0
        0x11730, 0x118e0, 0x11950, 0x11bf0, // 16.0
        0x11c50, 0x11d50, 0x11da0, 0x11f50, 0x16130, // 16.0
        0x16a60, 0x16ac0, 0x16b50, 0x16d70, 0x1ccf0, // 16.0
        0x1d7ce, 0x1d7d8, 0x1d7e2, 0x1d7ec, 0x1d7f6, 0x1e140, 0x1e2f0, 0x1e4f0,
        0x1e5f1, // 16.0
        0x1e950, 0x1fbf0,
    ];

    #[test]
    fn py_strip_leerraum_wie_cpython() {
        let mut puffer = [0u8; 4];
        let rust: Vec<u32> = alle_zeichen()
            .filter(|c| py_strip(c.encode_utf8(&mut puffer)).is_empty())
            .map(u32::from)
            .collect();
        assert_eq!(rust, STRIP);
    }

    #[test]
    fn int_leerraum_wie_cpython() {
        let rust: Vec<u32> = alle_zeichen()
            .filter(|c| int_aus_text(&format!("{c}7{c}")).as_deref() == Ok("7"))
            .map(u32::from)
            .collect();
        assert_eq!(rust, INT_LEERRAUM);
    }

    #[test]
    fn nd_menge_wie_cpython_3_14() {
        let rust: Vec<u32> = alle_zeichen()
            .filter(|c| ist_nd(*c))
            .map(u32::from)
            .collect();
        let python: Vec<u32> = ND_NULLEN.iter().flat_map(|null| *null..null + 10).collect();
        assert_eq!(rust, python);
        for null in ND_NULLEN {
            for k in 0..10 {
                let c = char::from_u32(null + k).unwrap();
                assert_eq!(ascii_ziffer(c), char::from_digit(k, 10), "{c:?}");
            }
        }
        // 3.14.7: -13; 3.12.9: `ValueError`, U+1E5F2 ist dort noch keine Ziffer.
        assert_eq!(int_aus_text("-\u{1e5f2}_\u{ff13}").as_deref(), Ok("-13"));
        assert!(int_aus_text("\u{b2}").is_err()); // `²` ist No, nicht Nd
    }

    /// D17, gemessen 3.12.9 und 3.14.7: die Grenze zaehlt Ziffern samt fuehrender Nullen, ohne `_`
    /// und Leerraum, und erst nach der Syntax; der Text in der Meldung ist `%.200R`.
    #[test]
    fn ziffern_grenze_wie_cpython() {
        let grenze = |n: usize| {
            Err(PyFehler::WertFehler(format!(
                "Exceeds the limit (4300 digits) for integer string conversion: value has {n} \
                 digits; use sys.set_int_max_str_digits() to increase the limit"
            )))
        };
        assert_eq!(int_aus_text(&"0".repeat(4301)), grenze(4301));
        assert_eq!(
            int_aus_text(&format!(" {}", "1".repeat(4301))),
            grenze(4301)
        );
        assert_eq!(int_aus_text(&"1".repeat(4300)).map(|t| t.len()), Ok(4300));
        assert_eq!(
            int_aus_text(&format!("{}1", "1_".repeat(2150))).map(|t| t.len()),
            Ok(2151)
        );
        assert_eq!(
            int_aus_text(&format!("{}x", "1".repeat(4301))),
            Err(PyFehler::WertFehler(format!(
                "invalid literal for int() with base 10: '{}",
                "1".repeat(199)
            )))
        );
    }

    /// `int(text)` (D12, D16), gemessen 3.12.9 und 3.14.7.
    #[test]
    fn int_text_wie_cpython() {
        for (t, python) in [
            ("-9223372036854775808", "-9223372036854775808"),
            ("18446744073709551616", "18446744073709551616"),
            (" +1_000\u{3000}", "1000"),
            ("-0", "0"),
            ("007", "7"),
            ("0_0", "0"),
        ] {
            assert_eq!(int_aus_text(t).as_deref(), Ok(python), "{t:?}");
        }
        for t in [
            "", " ", "_1", "1_", "1__0", "+_1", "- 1", "1 2", "0x10", "1.0", "\u{1c}1",
        ] {
            assert_eq!(
                int_aus_text(t),
                Err(PyFehler::WertFehler(format!(
                    "invalid literal for int() with base 10: {}",
                    repr_str(t)
                ))),
                "{t:?}"
            );
        }
    }

    /// `repr(float)`, gemessen 3.12.9 und 3.14.7.
    #[test]
    fn repr_float_wie_cpython() {
        for (x, python) in [
            (1e16, "1e+16"),
            (1e-5, "1e-05"),
            (0.0001, "0.0001"),
            (1.234_567_890_123_456_8e17, "1.2345678901234568e+17"),
            (1.5, "1.5"),
            (-0.0, "-0.0"),
            (5e-324, "5e-324"),
            (f64::MAX, "1.7976931348623157e+308"),
            (9_999_999_999_999_998.0, "9999999999999998.0"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e15, "1000000000000000.0"),
            (123_456_789.0, "123456789.0"),
            (1e22, "1e+22"),
            (2f64.powi(63), "9.223372036854776e+18"),
            (-1e-7, "-1e-07"),
            (100.0, "100.0"),
            (f64::NAN, "nan"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
        ] {
            assert_eq!(repr_float(x), python, "{x:e}");
        }
    }

    /// `repr(str)` (D9), gemessen 3.12.9 und 3.14.7.
    #[test]
    fn repr_str_wie_cpython() {
        for (s, python) in [
            ("it's", "\"it's\""),
            ("a\"b'c", r#"'a"b\'c'"#),
            ("\\", r"'\\'"),
            ("\t\n\r", r"'\t\n\r'"),
            ("\0\u{1f}\u{7f}", r"'\x00\x1f\x7f'"),
            ("\u{85}\u{a0}\u{ad}", r"'\x85\xa0\xad'"),
            (
                "\u{200b}\u{2028}\u{3000}\u{feff}",
                r"'\u200b\u2028\u3000\ufeff'",
            ),
            ("\u{1f600}", "'\u{1f600}'"),
            ("äöü€", "'äöü€'"),
            (" ", "' '"),
        ] {
            assert_eq!(repr_str(s), python, "{s:?}");
        }
        // Die Luecke aus dem ponytail an `repr_str`: `CPython` escapet Cf U+0600, Co U+E000 und
        // Cn U+0378 (`'\u0600'`, `'\ue000'`, `'\u0378'`), `repr_str` laesst sie stehen.
        for s in ["\u{600}", "\u{e000}", "\u{378}"] {
            assert_eq!(repr_str(s), format!("'{s}'"));
        }
    }
}
