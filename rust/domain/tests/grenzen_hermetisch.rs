//! Grenzen und Operatoren von `domain`, die kein Bestandstest festnagelt (Mutationsmessung N4, Bericht
//! `mutation-bindung-domain`). Jeder Test steht fuer einen Mutanten, der im Standardlauf (ohne `PARITY=1`, ohne Python) gruen
//! blieb. Alles laeuft ueber die `pub`-Schnittstelle.
//!
//! * `money.rs`, `Cent::ceil_euro`: `div_euclid(100)` -> `99` lieferte fuer kleine Betraege dasselbe (Doctest: 200, 201, -150),
//!   fuer 10 000 Cent aber 101 Euro.
//! * `fall_id.rs`, `feld_id.rs`: Untergrenzen (`1..=64`, `n < 2`) und die Kante `basis__` / `__n` / `basis__2x` des
//!   `__`-Suffix-Erkenners.
//! * `meet.rs`, `meet_achse`: `all` -> `any` ergibt fuer EINEN Vektor `konflikt`; Python (`store.py:64`) gibt dort den Wert selbst.
//! * `py_wert.rs`: `gt_null`, `py_eq` (Text, Liste, Objekt), `typname`, `repr` von `False`.
//! * `py_text.rs`: Ziffernwert einer Nd-Ziffer aus einem Block mit mehr als zehn Ziffern (`% 10`).
//! * `wert.rs`: Datumsformat je Ziffernstelle, `Enum` ohne Werteliste.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::num::NonZeroU16;

use domain::{
    meet_herkunft, py_float, Achsenwert, BasisId, Cent, FallId, FeldId, FeldIdFehler, Feldtyp,
    Herkunft, PruefTiefe, PyFehler, PyWert, Wert, WertFehler,
};

// ---- money.rs -------------------------------------------------------------------------------------------------------

/// Aufrunden und Abrunden gegen zwei unabhaengige Formeln: `ceil = (c + 99).div_euclid(100)`, `floor` per Schleife.
#[test]
fn cent_auf_und_abrunden_wie_die_unabhaengige_formel() {
    for c in -3_000_i64..=3_000 {
        let soll_ab = {
            let mut e = 0_i64;
            while e * 100 > c {
                e -= 1;
            }
            while (e + 1) * 100 <= c {
                e += 1;
            }
            e
        };
        assert_eq!(Cent::new(c).floor_euro().get(), soll_ab, "floor {c}");
        assert_eq!(
            Cent::new(c).ceil_euro().get(),
            (c + 99).div_euclid(100),
            "ceil {c}"
        );
    }
    // Grosse Betraege: bei 99 statt 100 als Teiler laege schon 10 000 Cent daneben.
    for (c, ab, auf) in [
        (9_900, 99, 99),
        (10_000, 100, 100),
        (10_001, 100, 101),
        (123_456_789, 1_234_567, 1_234_568),
        (-10_000, -100, -100),
        (-10_001, -101, -100),
        (i64::MAX, 92_233_720_368_547_758, 92_233_720_368_547_759),
        (i64::MAX - 7, 92_233_720_368_547_758, 92_233_720_368_547_758),
    ] {
        assert_eq!(Cent::new(c).floor_euro().get(), ab, "floor {c}");
        assert_eq!(Cent::new(c).ceil_euro().get(), auf, "ceil {c}");
    }
}

// ---- fall_id.rs -----------------------------------------------------------------------------------------------------

/// `[A-Za-z0-9_-]{1,64}`: ein Zeichen ist eine Fall-Kennung, kein Zeichen und 65 Zeichen nicht.
#[test]
fn fall_id_laenge_ein_bis_vierundsechzig() {
    assert!(FallId::new("a").is_ok(), "ein Zeichen");
    assert!(FallId::new("_").is_ok());
    assert!(FallId::new("-").is_ok());
    assert!(FallId::new("a".repeat(64)).is_ok(), "64 Zeichen");
    assert!(FallId::new("").is_err(), "leer");
    assert!(FallId::new("a".repeat(65)).is_err(), "65 Zeichen");
    for schlecht in ["a b", "a.b", "ä", "a\n", "a/b"] {
        assert!(FallId::new(schlecht).is_err(), "{schlecht:?}");
    }
}

// ---- feld_id.rs -----------------------------------------------------------------------------------------------------

fn basis(s: &str) -> BasisId {
    BasisId::new(s).unwrap()
}

/// `a__` (leere Ziffern) und `a__2x` (nicht nur Ziffern) tragen KEINEN Instanz-Suffix: beides sind gueltige Basen.
#[test]
fn doppelstrich_ohne_reine_ziffern_ist_eine_basis() {
    for s in ["a__", "a__2x", "a__x2", "a___", "a__2_3", "a_2"] {
        let b = BasisId::new(s).unwrap_or_else(|e| panic!("{s:?}: {e}"));
        assert_eq!(b.as_str(), s);
        let f: FeldId = s.parse().unwrap_or_else(|e| panic!("{s:?}: {e}"));
        assert_eq!(f.instanznummer().get(), 1, "{s:?}");
        assert_eq!(f.basis().as_str(), s);
        assert_eq!(f.to_string(), s);
    }
}

/// `__2` hat keine Basis: der Fehler nennt den ganzen Text, nicht die leere Basis.
#[test]
fn suffix_ohne_basis_nennt_den_ganzen_text() {
    assert_eq!(
        "__2".parse::<FeldId>(),
        Err(FeldIdFehler::UngueltigeBasis("__2".to_owned()))
    );
    assert_eq!(
        BasisId::new("__2").map(|_| ()),
        Err(FeldIdFehler::UngueltigeBasis("__2".to_owned()))
    );
}

/// Instanz 1 ist keine Instanz-Kodierung; 2 und `u16::MAX` sind es; 65536 und fuehrende Nullen nicht.
#[test]
fn instanzgrenzen() {
    let eins = NonZeroU16::MIN;
    assert_eq!(
        FeldId::instanz(basis("a"), eins).map(|_| ()),
        Err(FeldIdFehler::UngueltigeInstanz("a__1".to_owned())),
        "direkter Aufruf mit n = 1"
    );
    for s in [
        "a__1",
        "a__0",
        "a__01",
        "a__02",
        "a__65536",
        "a__99999999999999999999",
    ] {
        assert_eq!(
            s.parse::<FeldId>(),
            Err(FeldIdFehler::UngueltigeInstanz(s.to_owned())),
            "{s}"
        );
    }
    let zwei: FeldId = "a__2".parse().unwrap();
    assert_eq!(zwei.instanznummer().get(), 2);
    assert_eq!(
        zwei,
        FeldId::instanz(basis("a"), NonZeroU16::new(2).unwrap()).unwrap()
    );
    let gross: FeldId = "a__65535".parse().unwrap();
    assert_eq!(gross.instanznummer().get(), u16::MAX);
    assert_eq!(gross.to_string(), "a__65535");
}

// ---- meet.rs --------------------------------------------------------------------------------------------------------

fn vektor(herkunft: &str, tiefe: PruefTiefe, haftung: &str) -> domain::HerkunftVektor {
    Herkunft {
        herkunft: Achsenwert::new(herkunft).unwrap(),
        pruef_tiefe: tiefe,
        haftung: Achsenwert::new(haftung).unwrap(),
    }
    .into()
}

/// Python `meet_herkunft([v])` gibt die Achsenwerte von `v` zurueck (`store.py:64-72`); es gibt keinen Konflikt mit sich selbst.
#[test]
fn meet_eines_einzelnen_vektors_ist_dieser_vektor() {
    let h = meet_herkunft([vektor("mensch", PruefTiefe::Plausibilisiert, "nutzer")]).unwrap();
    assert_eq!(h.herkunft.as_str(), "mensch");
    assert_eq!(h.pruef_tiefe, PruefTiefe::Plausibilisiert);
    assert_eq!(h.haftung.as_str(), "nutzer");
}

/// Bei drei Vektoren genuegt EIN abweichender: `konflikt` je Achse, wo nicht alle gleich sind; sonst bleibt der Wert.
#[test]
fn meet_von_drei_vektoren_braucht_einigkeit_aller() {
    let a = || vektor("mensch", PruefTiefe::Amtlich, "nutzer");
    let b = || vektor("edaten", PruefTiefe::Amtlich, "amt");
    let h = meet_herkunft([a(), a(), a()]).unwrap();
    assert_eq!(
        (h.herkunft.as_str(), h.haftung.as_str()),
        ("mensch", "nutzer")
    );
    for (name, vs) in [
        ("aab", [a(), a(), b()]),
        ("aba", [a(), b(), a()]),
        ("baa", [b(), a(), a()]),
    ] {
        let h = meet_herkunft(vs).unwrap();
        assert!(h.herkunft.ist_konflikt(), "{name}: herkunft");
        assert!(h.haftung.ist_konflikt(), "{name}: haftung");
    }
    // Die Achsen sind unabhaengig: Einigkeit auf der einen, Streit auf der anderen.
    let h = meet_herkunft([
        vektor("mensch", PruefTiefe::Amtlich, "nutzer"),
        vektor("mensch", PruefTiefe::Ungeprueft, "nutzer"),
        vektor("mensch", PruefTiefe::OrakelBestaetigt, "amt"),
    ])
    .unwrap();
    assert_eq!(h.herkunft.as_str(), "mensch");
    assert!(h.haftung.ist_konflikt());
    assert_eq!(h.pruef_tiefe, PruefTiefe::Ungeprueft);
}

// ---- py_wert.rs -----------------------------------------------------------------------------------------------------

/// `x > 0` fuer jede Zahlenart an der Null und beidseits davon (`Ganz(0) > 0` ist falsch).
#[test]
fn gt_null_an_der_null() {
    for (w, soll) in [
        (PyWert::Bool(false), false),
        (PyWert::Bool(true), true),
        (PyWert::Ganz(-1), false),
        (PyWert::Ganz(0), false),
        (PyWert::Ganz(1), true),
        (PyWert::Ganz(i64::MAX), true),
        (PyWert::Ganz(i64::MIN), false),
        (PyWert::GrossGanz(0), false),
        (PyWert::GrossGanz(1), true),
        (PyWert::GrossGanz(u64::MAX), true),
        (PyWert::Gleit(-0.5), false),
        (PyWert::Gleit(0.0), false),
        (PyWert::Gleit(-0.0), false),
        (PyWert::Gleit(0.5), true),
        (PyWert::Gleit(f64::NAN), false),
    ] {
        assert_eq!(w.gt_null(), Ok(soll), "{w:?}");
    }
    for w in [
        PyWert::Null,
        PyWert::Text("1".into()),
        PyWert::Liste(vec![]),
        PyWert::Objekt(vec![]),
    ] {
        let PyFehler::TypFehler(m) = w.gt_null().unwrap_err() else {
            panic!("{w:?}: kein TypFehler")
        };
        assert_eq!(
            m,
            format!(
                "'>' not supported between instances of '{}' and 'int'",
                w.typname()
            )
        );
    }
}

fn text(s: &str) -> PyWert {
    PyWert::Text(s.to_owned())
}

fn objekt(paare: &[(&str, PyWert)]) -> PyWert {
    PyWert::Objekt(
        paare
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect(),
    )
}

/// `==` auf Text, Liste und dict: gleich, ungleich, und ein Unterschied an JEDER Stelle zaehlt.
#[test]
fn py_eq_fuer_text_liste_und_objekt() {
    assert!(text("a").py_eq(&text("a")));
    assert!(text("").py_eq(&text("")));
    assert!(!text("a").py_eq(&text("b")));
    assert!(!text("a").py_eq(&text("A")));

    let liste = |v: &[i64]| PyWert::Liste(v.iter().map(|n| PyWert::Ganz(*n)).collect());
    assert!(liste(&[1, 2, 3]).py_eq(&liste(&[1, 2, 3])));
    assert!(liste(&[]).py_eq(&liste(&[])));
    for anders in [
        &[0, 2, 3][..],
        &[1, 0, 3],
        &[1, 2, 0],
        &[1, 2],
        &[1, 2, 3, 4],
    ] {
        assert!(!liste(&[1, 2, 3]).py_eq(&liste(anders)), "{anders:?}");
        assert!(
            !liste(anders).py_eq(&liste(&[1, 2, 3])),
            "{anders:?} umgekehrt"
        );
    }

    let o = |a: i64, b: i64| objekt(&[("a", PyWert::Ganz(a)), ("b", PyWert::Ganz(b))]);
    assert!(o(1, 2).py_eq(&o(1, 2)));
    assert!(!o(1, 2).py_eq(&o(1, 3)), "zweiter Wert anders");
    assert!(!o(1, 2).py_eq(&o(0, 2)), "erster Wert anders");
    assert!(
        o(1, 2).py_eq(&objekt(&[("b", PyWert::Ganz(2)), ("a", PyWert::Ganz(1))])),
        "Reihenfolge der Schluessel zaehlt nicht"
    );
    assert!(
        !o(1, 2).py_eq(&objekt(&[("a", PyWert::Ganz(1)), ("c", PyWert::Ganz(2))])),
        "anderer Schluessel"
    );
    assert!(
        !o(1, 2).py_eq(&objekt(&[("a", PyWert::Ganz(1))])),
        "weniger Schluessel"
    );
}

#[test]
fn typname_und_repr_je_variante() {
    for (w, typ, repr) in [
        (PyWert::Null, "NoneType", "None"),
        (PyWert::Bool(true), "bool", "True"),
        (PyWert::Bool(false), "bool", "False"),
        (PyWert::Ganz(-5), "int", "-5"),
        (PyWert::GrossGanz(u64::MAX), "int", "18446744073709551615"),
        (PyWert::Gleit(1.5), "float", "1.5"),
        (text("x"), "str", "'x'"),
        (PyWert::Liste(vec![]), "list", "[]"),
        (PyWert::Objekt(vec![]), "dict", "{}"),
    ] {
        assert_eq!(w.typname(), typ, "{w:?}");
        assert_eq!(w.repr(), repr, "{w:?}");
    }
}

// ---- py_text.rs -----------------------------------------------------------------------------------------------------

/// Der Ziffernwert einer Nd-Ziffer ist der Abstand zum Anfang des zusammenhaengenden Nd-Bereichs modulo 10. Die zwei
/// Bereiche mit mehr als zehn Ziffern (U+1D7CE..U+1D7FF, 50 Stueck; U+116D0..U+116E3, 20 Stueck) pruefen das `% 10`;
/// `unicodedata.decimal` (Python 3.14.7, Unicode 16.0.0) nennt fuer jedes Zeichen `k % 10`.
#[test]
fn nd_ziffern_in_aneinanderstossenden_bloecken() {
    for (start, anzahl) in [(0x1D7CE_u32, 50_u32), (0x116D0, 20)] {
        for k in 0..anzahl {
            let c = char::from_u32(start + k).unwrap();
            assert_eq!(
                py_float(&c.to_string()),
                Some(f64::from(k % 10)),
                "U+{:04X}",
                start + k
            );
        }
    }
    // Mehrstellig und gemischt mit ASCII: \u{1D7D9}\u{1D7DB} = 13.
    assert_eq!(py_float("\u{1D7D9}\u{1D7DB}"), Some(13.0));
    assert_eq!(py_float("1\u{1D7E1}"), Some(19.0));
}

// ---- wert.rs --------------------------------------------------------------------------------------------------------

/// `TT.MM.JJJJ`: jede der acht Ziffernstellen ist eine Ziffer; ein Buchstabe an GENAU einer Stelle weist ab.
#[test]
fn datum_jede_ziffernstelle_wird_geprueft() {
    let ok = |s: &str| Wert::aus_pywert(&text(s), Feldtyp::Datum, None);
    assert_eq!(ok("31.12.2025"), Ok(Wert::Datum("31.12.2025".to_owned())));
    let basis = "01.02.2024";
    for pos in [0, 1, 3, 4, 6, 7, 8, 9] {
        let mut b = basis.as_bytes().to_vec();
        b[pos] = b'x';
        let s = String::from_utf8(b).unwrap();
        assert_eq!(ok(&s), Err(WertFehler::UngueltigesDatum(s.clone())), "{s}");
    }
    for s in [
        "1.02.2024",
        "01.2.2024",
        "01.02.24",
        "01-02-2024",
        "2024-02-01",
        "01.02.2024 ",
        "",
    ] {
        assert_eq!(
            ok(s),
            Err(WertFehler::UngueltigesDatum(s.to_owned())),
            "{s:?}"
        );
    }
}

/// `Enum` ohne Werteliste (oder mit leerer) laesst nichts durch: fail-closed.
#[test]
fn enum_ohne_werteliste_weist_ab() {
    let versuch = |liste: Option<&[String]>| Wert::aus_pywert(&text("ja"), Feldtyp::Enum, liste);
    let unbekannt = Err(WertFehler::UnbekannterEnumWert("ja".to_owned()));
    assert_eq!(versuch(None), unbekannt);
    assert_eq!(versuch(Some(&[])), unbekannt);
    assert_eq!(versuch(Some(&["nein".to_owned()])), unbekannt);
    assert_eq!(
        versuch(Some(&["nein".to_owned(), "ja".to_owned()])),
        Ok(Wert::Enum("ja".to_owned()))
    );
}
