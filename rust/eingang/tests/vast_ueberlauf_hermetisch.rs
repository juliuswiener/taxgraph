//! Ueberlaufpfade des VaSt-Lesers (`eingang::vast`): Betraege, die kein `i64` faengt, scheitern als
//! `VastFehler::Ueberlauf` (Python: `OverflowError`), Unlesbares als `NichtLesbar` (Python:
//! `ValueError`) — Standardlauf, ohne Python.
//!
//! Die Mutanten (Bericht h8-hermetisch5, Bestand 0aa91677): E6 in `zerlege`
//! (`exp.checked_sub(nachkommastellen)` -> `wrapping_sub`) — nur der Fall mit Exponent exakt
//! `i64::MIN` erreicht die Stelle, alle anderen Exponenten scheitern vorher am `parse`; E7
//! (`inf`/`Infinity`: `Ueberlauf` -> `NichtLesbar`); E8 (`betrag.parse::<i64>()`-Fehlerzweig:
//! `Ueberlauf` -> `NichtLesbar`); E10 in `aus_lersl` (`checked_add` -> `wrapping_add` ohne
//! Fehlerzweig). E9 (19-Stellen-Schranke -> 99) ist verhaltensgleich: was die Schranke nicht
//! faengt, faengt der `parse::<i64>` zwei Zeilen tiefer — dieselbe Klasse, desselbe Text. Am Wert gemessen (Sonde im Messbaum,
//! `e9_original.out` / `e9_mutant.out`): 15 Eingaben, darunter `1e20`, `1e50`, `1e97`, `1e98`, `1e99` (Exponent zwischen den
//! Schranken) und `1e100`, `1e999999` (darueber), liefern mit Original und Mutante dieselbe Antwort, Zeichen fuer Zeichen.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: `produkt/eingang/vast_mapping._cent` und `.aus_lersl` auf
//! dieselben Texte (Orakel-Skript `orakel_ea5.py`, Log: Anlagen zum Bericht). Python trennt
//! `OverflowError` (Zahl zu gross, `Infinity`) von `ValueError` (kein Literal); die Grenze ist
//! symmetrisch `±(2**63 - 1)`, `-2**63` weisen beide Seiten ab. Rust bildet die Klassen 1:1 nach
//! (`VastFehler`-Doku); keine Erwartung ist dem Rust-Code abgelesen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use eingang::vast::{aus_lersl, cent, Leistung, VastFehler};

#[derive(Debug, PartialEq, Eq)]
enum Erw {
    KeinWert,
    Wert(i64),
    Ueberlauf,
    NichtLesbar,
}

fn ist(ist: &Result<Option<i64>, VastFehler>) -> Erw {
    match ist {
        Ok(None) => Erw::KeinWert,
        Ok(Some(w)) => Erw::Wert(*w),
        Err(VastFehler::Ueberlauf(_)) => Erw::Ueberlauf,
        Err(VastFehler::NichtLesbar(_)) => Erw::NichtLesbar,
    }
}

/// (Text, Erwartung) wie `orakel_ea5.json`; `None` = Feld nicht besetzt.
const FAEELLE: &[(&str, Option<&str>, Erw)] = &[
    // Besetzt vs. nicht besetzt: "kein Wert" ist nicht "Wert 0".
    ("nicht besetzt", None, Erw::KeinWert),
    ("leer", Some(""), Erw::KeinWert),
    ("nur Leerzeichen", Some("  "), Erw::KeinWert),
    // Halbe Cent: Python rundet half-even, 0,005 -> 0, 0,015 -> 2.
    ("null", Some("0,00"), Erw::Wert(0)),
    ("0,005 (half-even ab)", Some("0,005"), Erw::Wert(0)),
    ("0,015 (half-even auf, gerade)", Some("0,015"), Erw::Wert(2)),
    // Die Python-`Decimal`-Unterstriche ("1__2", "1_", "_1" lesen bei Python als 12/1/1) laesst
    // Rust als NichtLesbar ab — Parsing-Abweichung, keine Ueberlaufstelle; Befund im Bericht
    // h8-hermetisch5, kein Fall hier (kein Mutant E1-E10 beruehrt `gueltig`).
    // Die i64-Grenze selbst (E8-Gegenproben): MAX Cent passt, beide Vorzeichen.
    (
        "i64::MAX Cent",
        Some("92233720368547758,07"),
        Erw::Wert(9_223_372_036_854_775_807),
    ),
    (
        "i64::MAX Cent negativ",
        Some("-92233720368547758,07"),
        Erw::Wert(-9_223_372_036_854_775_807),
    ),
    (
        "auf 19 Stellen gerundet (half-even)",
        Some("9223372036854775,807"),
        Erw::Wert(922_337_203_685_477_581),
    ),
    // Einen Cent drueber: OverflowError, symmetrisch — auch -2**63 weist Python ab (Doku _cent).
    (
        "i64::MAX + 1 Cent",
        Some("92233720368547758,08"),
        Erw::Ueberlauf,
    ),
    (
        "-2**63 Cent (symmetrisch abgewiesen)",
        Some("-92233720368547758,08"),
        Erw::Ueberlauf,
    ),
    (
        "20 Vorkommastellen",
        Some("922337203685477580,70"),
        Erw::Ueberlauf,
    ),
    ("1e400", Some("1e400"), Erw::Ueberlauf),
    ("1e308 (E-Form riesig)", Some("1e308"), Erw::Ueberlauf),
    // E6: Exponent exakt i64::MIN — das `parse` gelingt, die Subtraktion der Nachkommastelle
    // rutscht darunter; Python: ValueError (nicht lesbar), NICHT OverflowError.
    (
        "Exponent i64::MIN",
        Some("1,0e-9223372036854775808"),
        Erw::NichtLesbar,
    ),
    (
        "Exponent jenseits i64",
        Some("1e9223372036854775808"),
        Erw::NichtLesbar,
    ),
    ("1e-400 (rundet auf 0)", Some("1e-400"), Erw::Wert(0)),
    // E7: die float-Namen sind Python-`Infinity` -> OverflowError, nicht ValueError.
    ("inf", Some("inf"), Erw::Ueberlauf),
    ("Infinity", Some("Infinity"), Erw::Ueberlauf),
    ("INFINITY (gross)", Some("INFINITY"), Erw::Ueberlauf),
    ("-inf", Some("-inf"), Erw::Ueberlauf),
    // Unlesbares bleibt NichtLesbar (Gegenproben gegen eine zu breite Ueberlauf-Klasse).
    ("abc", Some("abc"), Erw::NichtLesbar),
    ("1,2,3", Some("1,2,3"), Erw::NichtLesbar),
    ("kahler Exponent", Some("e5"), Erw::NichtLesbar),
];

#[test]
fn der_vast_leser_scheidet_ueberlauf_von_unlesbar_wie_python() {
    let mut falsch = Vec::new();
    for (name, text, erwartet) in FAEELLE {
        let kam = ist(&cent(*text));
        if &kam != erwartet {
            falsch.push(format!(
                "{name} ({text:?}): erwartet {erwartet:?}, gekommen {kam:?}"
            ));
        }
    }
    assert_eq!(
        FAEELLE.len(),
        24,
        "die Zahl der VaSt-Faelle hat sich verschoben"
    );
    assert_eq!(
        FAEELLE
            .iter()
            .filter(|(_, _, e)| matches!(e, Erw::Ueberlauf))
            .count(),
        9,
        "die Zahl der Ueberlauf-Erwartungen hat sich verschoben"
    );
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// (Erwartung, Leistungen) wie `orakel_ea5.json`; Summe ausserhalb i64: `Ueberlauf` je
/// Summand (Python prueft die laufende Summe gegen i64, `aus_lersl`).
/// `(Name, Leistungen (Art, Betrag), Erwartung)`.
type LerslFall = (
    &'static str,
    &'static [(&'static str, &'static str)],
    Result<i64, ()>,
);

const LERSL: &[LerslFall] = &[
    (
        "zwei Betraege je i64::MAX Cent: die Summe passt in kein i64 (E10)",
        &[
            ("92233720368547758,07", "ALG"),
            ("92233720368547758,07", "Kzg"),
        ],
        Err(()),
    ),
    (
        "i64::MAX Cent plus 1 Cent",
        &[("92233720368547758,07", "ALG"), ("0,01", "")],
        Err(()),
    ),
    (
        "negativ: -i64::MAX Cent minus 1 Euro",
        &[("-92233720368547758,07", "ALG"), ("-1,00", "B")],
        Err(()),
    ),
    ("ein normaler Betrag", &[("1234.00", "ALG")], Ok(123_400)),
    (
        "nur Nullen: kein Satz",
        &[("0.00", "ALG"), ("0.00", "Kzg")],
        Ok(0),
    ),
];

#[test]
fn die_lersl_summe_ueber_i64_meldet_ueberlauf_statt_zu_wickeln() {
    let mut falsch = Vec::new();
    for (name, leistungen, erwartet) in LERSL {
        let l: Vec<Leistung> = leistungen
            .iter()
            .map(|(b, a)| Leistung {
                betrag: Some((*b).to_owned()),
                art: Some((*a).to_owned()),
            })
            .collect();
        let kam = match aus_lersl(&l) {
            Ok(s) => Ok(s.first().map_or(0, |x| x.wert)),
            Err(VastFehler::Ueberlauf(_) | VastFehler::NichtLesbar(_)) => Err(()),
        };
        match (erwartet, &kam) {
            (Ok(w), Ok(k)) if k == w => {}
            (Err(()), Err(())) => {}
            _ => falsch.push(format!("{name}: erwartet {erwartet:?}, gekommen {kam:?}")),
        }
    }
    assert_eq!(
        LERSL.len(),
        5,
        "die Zahl der LErsL-Faelle hat sich verschoben"
    );
    assert!(falsch.is_empty(), "{falsch:#?}");
}
