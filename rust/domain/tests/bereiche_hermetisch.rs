//! Bereichsgrenzen und Zweige von `domain`, die kein Bestandstest festnagelt (Mutationsmessung N4e, Bericht
//! `mutation-bindung-domain`, Abschnitt N4e). Jeder Test steht fuer Mutanten, die im Standardlauf (ohne `PARITY=1`, ohne
//! Python) gruen blieben: `..=` -> `..`, ein Bereichsende um einen Codepunkt verschoben, ein `match`-Zweig mit dem Rumpf des
//! Nachbarzweigs, ein Funktionsrumpf durch `Default::default()` ersetzt.
//!
//! * `py_text.rs`, `druckbar` (Teil von `repr`): die Grenzen der drei Bereiche `U+2000..=U+200F`, `U+2028..=U+202F`,
//!   `U+205F..=U+2064`. Erwartung: `repr(chr(c))` von `CPython` 3.14.7 (Unicode 16.0.0), je Grenze der letzte Codepunkt innen und der
//!   erste aussen. Zwei Nachbarn (`U+1FFF`, `U+2065`) sind in `CPython` unbelegt (Kategorie Cn) und werden dort escapet; diese
//!   Umsetzung escapet sie nicht (`ponytail`-Kommentar an `repr_str`). Sie stehen als eigener Test, damit ein Umbau auf die volle
//!   Kategorientabelle genau diese zwei Zeilen aendern muss.
//! * `py_text.rs`, `repr_float`: negative Werte mit Betrag ungleich 0 (die Zweig-Bedingung `f == 0.0` ist nur mit `0.0` und `-0.0`
//!   getestet; ein Mutant `f == -1.0` liefert fuer `-1.0` den Text `-0.0`).
//! * `wert.rs`, `nur_xml_zeichen`: die Grenzen der Produktion `Char` aus XML 1.0 (<https://www.w3.org/TR/xml/#charsets>).
//! * `wert.rs`, `Wert::aus_pywert` mit `Feldtyp::Bool`; `zustand.rs`, `Feldzustand::zustand` und `Signal2::as_str`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use domain::{nur_xml_zeichen, repr_float, Feldtyp, Feldzustand, PyWert, Signal2, Wert, Zustand};

const BS: char = '\\';

fn repr(c: char) -> String {
    PyWert::Text(c.to_string()).repr()
}

/// `true`: `CPython` escapet das Zeichen (`\x..` bis U+00FF, sonst `\u....`); `false`: es steht unveraendert zwischen den
/// Anfuehrungszeichen.
const REPR_GRENZEN: &[(u32, bool)] = &[
    // Steuerzeichen und Nachbarn
    (0x08, true),
    (0x1F, true),
    (0x20, false),
    (0x21, false),
    (0x7E, false),
    (0x7F, true),
    (0x9F, true),
    (0xA0, true),
    (0xA1, false),
    (0xAC, false),
    (0xAD, true),
    (0xAE, false),
    (0xFF, false),
    (0x100, false),
    // U+1680 allein
    (0x167F, false),
    (0x1680, true),
    (0x1681, false),
    // U+2000..=U+200F: Zs, Cf
    (0x2000, true),
    (0x2001, true),
    (0x200A, true),
    (0x200B, true),
    (0x200E, true),
    (0x200F, true),
    (0x2010, false),
    // U+2028..=U+202F: Zl, Zp, Cf, Zs
    (0x2027, false),
    (0x2028, true),
    (0x2029, true),
    (0x202A, true),
    (0x202E, true),
    (0x202F, true),
    (0x2030, false),
    // U+205F..=U+2064: Zs, Cf
    (0x205E, false),
    (0x205F, true),
    (0x2060, true),
    (0x2063, true),
    (0x2064, true),
    // U+3000, U+FEFF
    (0x2FFF, false),
    (0x3000, true),
    (0x3001, false),
    (0xFEFD, false),
    (0xFEFF, true),
    (0xFFFD, false),
];

#[test]
fn repr_escapet_an_den_grenzen_wie_cpython() {
    for &(cp, soll) in REPR_GRENZEN {
        let c = char::from_u32(cp).unwrap();
        let erwartet = match (soll, cp <= 0xFF) {
            (true, true) => format!("'{BS}x{cp:02x}'"),
            (true, false) => format!("'{BS}u{cp:04x}'"),
            (false, _) => format!("'{c}'"),
        };
        assert_eq!(repr(c), erwartet, "U+{cp:04X}");
    }
}

/// Bekannte Abweichung von `CPython` (`ponytail` an `repr_str`): `U+1FFF` und `U+2065` sind unbelegt (Cn), `CPython` 3.14.7 escapet
/// sie (als Backslash-u-Escape), diese Umsetzung nicht. Der Test haelt den heutigen Stand fest; wer `druckbar` auf die volle
/// Kategorientabelle umbaut, aendert hier die zwei Erwartungen.
#[test]
fn repr_unbelegte_nachbarn_der_bereiche_bleiben_unescapet() {
    assert_eq!(repr('\u{1FFF}'), "'\u{1FFF}'");
    assert_eq!(repr('\u{2065}'), "'\u{2065}'");
}

/// Erwartung: `repr(x)` von `CPython` 3.14.7.
#[test]
fn repr_float_negativer_wert_ungleich_null() {
    for (f, soll) in [
        (-1.0, "-1.0"),
        (-2.0, "-2.0"),
        (-0.1, "-0.1"),
        (-123.456, "-123.456"),
        (-1e16, "-1e+16"),
        (-5e-324, "-5e-324"),
        (1.0, "1.0"),
        (1e22, "1e+22"),
        (0.0, "0.0"),
        (-0.0, "-0.0"),
    ] {
        assert_eq!(repr_float(f), soll, "{f:e}");
    }
}

/// XML 1.0, Produktion `Char`: #x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF].
#[test]
fn nur_xml_zeichen_an_den_grenzen_der_char_produktion() {
    for cp in [
        0x09, 0x0A, 0x0D, 0x20, 0x21, 0xD7FE, 0xD7FF, 0xE000, 0xE001, 0xFFFC, 0xFFFD, 0x10000,
        0x10001, 0x10_FFFE, 0x10_FFFF,
    ] {
        let c = char::from_u32(cp).unwrap();
        assert!(nur_xml_zeichen(&c.to_string()), "U+{cp:04X} ist erlaubt");
    }
    for cp in [0x00, 0x08, 0x0B, 0x0C, 0x0E, 0x1F, 0xFFFE, 0xFFFF] {
        let c = char::from_u32(cp).unwrap();
        assert!(!nur_xml_zeichen(&c.to_string()), "U+{cp:04X} ist verboten");
    }
    assert!(
        !nur_xml_zeichen("Muster\u{1F}mann"),
        "ein verbotenes Zeichen mitten im Text"
    );
    assert!(nur_xml_zeichen("Müller\tStraße\r\n"));
}

#[test]
fn bool_wert_zu_bool_feld() {
    assert_eq!(
        Wert::aus_pywert(&PyWert::Bool(true), Feldtyp::Bool, None),
        Ok(Wert::Bool(true))
    );
    assert_eq!(
        Wert::aus_pywert(&PyWert::Bool(false), Feldtyp::Bool, None),
        Ok(Wert::Bool(false))
    );
    assert!(Wert::aus_pywert(&PyWert::Ganz(1), Feldtyp::Bool, None).is_err());
}

#[test]
fn feldzustand_und_signal_geben_ihren_inhalt_zurueck() {
    assert_eq!(Feldzustand::Vorlaeufig.zustand(), Zustand::Vorlaeufig);
    let signal = Signal2::new("beweist@fam_anzahl_kinder=2").unwrap();
    assert_eq!(signal.as_str(), "beweist@fam_anzahl_kinder=2");
    assert_eq!(
        Feldzustand::Bestaetigt { signal_2: signal }.zustand(),
        Zustand::Bestaetigt
    );
    // Die Pruefung schneidet Leerraum nur fuer den Test auf "leer" ab; der Text bleibt, wie er kam.
    assert_eq!(Signal2::new(" a ").unwrap().as_str(), " a ");
}
