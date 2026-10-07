//! Satz und Hoechstbetraege der Parteispenden-Ermaessigung gegen das GESETZ, nicht gegen die YAML (Abweichung Nr. 31 in
//! `rust/fixtures/README.md`; Entscheidung `parteispenden-deckel-kommt-je-jahr-aus-der-eingefrorenen-fassung`, Option 1).
//!
//! `werte_gegen_gesetz.rs` und `params_zuordnung_hermetisch.rs` zeigen den Weg: ein Test, der die YAML mit dem Zugriff vergleicht,
//! bliebe gruen, wenn Datei und Zugriff dieselbe falsche Zahl meinten. Hier steht der Erwartungswert nicht im Test. Er wird bei
//! jedem Lauf aus dem eingefrorenen Wortlaut von § 34g Satz 2 unter `sources/gesetze-im-internet/` gelesen und mit dem verglichen,
//! was `bindung::Params` fuer das Jahr liefert (der Lesepfad, den die Rechnung nimmt).
//!
//! WELCHE FASSUNG FUER WELCHES JAHR. Die amtliche Seite zeigt nur die geltende Fassung. Fuer 2024 und 2025 liegen deshalb
//! Archivkopien dieser Seite (`web.archive.org`, 2024-12-18 und 2025-12-18, wortgleich, gemessen 2026-10-07), fuer 2026 die
//! Abrufkopie vom 2026-09-26. Die Zuordnung Kopie -> Jahr steht unten in [`JAHRE`]: eine Kopie vom 18. Dezember eines Jahres gilt
//! fuer dieses Jahr, weil eine Aenderung der Betraege zum Jahreswechsel in Kraft tritt (Steueraenderungsgesetz 2025 zum
//! 1.1.2026). Das ist ABGELEITET (`derived`) aus den Abrufdaten, nicht aus dem Gesetzestext: er nennt sein Inkrafttreten nicht.
//! Nicht belegt ist, ob der Wortlaut zwischen dem Abrufdatum und dem 31.12. des Jahres gleich blieb.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use bindung::{lade_params, Params};
use domain::{Euro, Satz, Vz};
use regex::Regex;
use rust_decimal::Decimal;

/// `(Veranlagungsjahr, Vz, Datei des Wortlauts)`.
const JAHRE: [(u16, Vz, &str); 3] = [
    (2024, Vz::Vz2024, "estg_p34g_2024-12-18"),
    (2025, Vz::Vz2025, "estg_p34g_2025-12-18"),
    (2026, Vz::Vz2026, "estg_p34g_2026-09-26"),
];

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn quelle(name: &str) -> String {
    let pfad = wurzel()
        .join("sources")
        .join("gesetze-im-internet")
        .join(format!("{name}.txt"));
    std::fs::read_to_string(&pfad)
        .unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", pfad.display()))
}

/// `(Prozent, Hoechstbetrag einzeln, Hoechstbetrag zusammen)` aus § 34g Satz 2. Ein Fehler heisst: der Text ist nicht mehr der
/// Satz, den der Parser kennt (nie ein stiller Standardwert).
fn satz_2(text: &str) -> Result<(i64, i64, i64), String> {
    let wortlaut = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let re = Regex::new(
        r"Die Ermäßigung beträgt (\d+) Prozent der Ausgaben, höchstens jeweils (\d+(?: \d{3})*) Euro für Ausgaben nach den Nummern 1 und 2, im Fall der Zusammenveranlagung von Ehegatten höchstens jeweils (\d+(?: \d{3})*) Euro",
    )
    .unwrap();
    let t = re.captures(&wortlaut).ok_or("Satz 2 nicht gefunden")?;
    let zahl = |i: usize| -> Result<i64, String> {
        t[i].replace(' ', "")
            .parse::<i64>()
            .map_err(|e| format!("keine Zahl {:?}: {e}", &t[i]))
    };
    Ok((zahl(1)?, zahl(2)?, zahl(3)?))
}

fn params() -> Params {
    Params::lade(&wurzel()).unwrap()
}

/// Satz, Einzel- und Zusammen-Deckel jedes Jahres gleich dem Wortlaut seiner Fassung.
#[test]
fn satz_und_deckel_jedes_jahres_sind_der_wert_im_gesetz() {
    let p = params();
    for (jahr, vz, datei) in JAHRE {
        let (prozent, einzel, zusammen) = satz_2(&quelle(datei)).unwrap();
        let ist = p.parteispenden_p34g(vz).unwrap();
        assert_eq!(
            ist.satz,
            Satz::new(Decimal::new(prozent, 2)),
            "{jahr}: Satz (Wortlaut {datei}: {prozent} Prozent)"
        );
        assert_eq!(
            ist.hoechstbetrag_einzel,
            Euro::new(einzel),
            "{jahr}: Hoechstbetrag einzeln (Wortlaut {datei})"
        );
        assert_eq!(
            ist.hoechstbetrag_zusammen,
            Euro::new(zusammen),
            "{jahr}: Hoechstbetrag zusammen (Wortlaut {datei})"
        );
    }
}

/// KONTROLLE: der Parser liest die Zahlen, die der Wortlaut tatsaechlich traegt. Ohne sie bewiese der Vergleich oben nichts: ein
/// Parser, der immer dieselbe Zahl lieferte, und eine Datei mit derselben Zahl liessen ihn gruen.
#[test]
fn kontrolle_der_parser_unterscheidet_die_fassungen() {
    let gelesen: Vec<_> = JAHRE
        .iter()
        .map(|(_, _, d)| satz_2(&quelle(d)).unwrap())
        .collect();
    assert_eq!(gelesen[0], (50, 825, 1_650), "2024-12-18");
    assert_eq!(gelesen[1], (50, 825, 1_650), "2025-12-18");
    assert_eq!(gelesen[2], (50, 1_650, 3_300), "2026-09-26");
    // Der Parser meldet einen Text ohne den Satz als Fehler und liefert keinen Wert.
    assert!(satz_2("Die Ermäßigung beträgt viel.").is_err());
}

/// Jede Zahl der Parameterdatei nennt in `datenquelle` die Datei ihres Jahres, nicht die eines anderen: eine 2025er Zahl, die ins
/// 2026er Jahr kopiert wurde, fiele sonst nur auf, wenn die Werte voneinander abweichen.
#[test]
fn jede_zahl_nennt_die_quelle_ihres_jahres() {
    for (jahr, _, datei) in JAHRE {
        let pfad = wurzel()
            .join("params")
            .join(jahr.to_string())
            .join("parteispenden_p34g.yaml");
        let d = lade_params(&pfad).unwrap();
        assert_eq!(d.veranlagungszeitraum, jahr, "{}", pfad.display());
        assert_eq!(d.gueltig_ab, format!("{jahr}-01-01"));
        for schluessel in [
            "ermaessigungssatz",
            "hoechstbetrag_einzel",
            "hoechstbetrag_zusammen",
        ] {
            let eintrag = d
                .werte
                .get(schluessel)
                .unwrap_or_else(|| panic!("{schluessel} fehlt in {}", pfad.display()));
            let quelle = eintrag["datenquelle"].as_str().unwrap();
            assert!(
                quelle.contains(&format!("{datei}.txt")),
                "{jahr} {schluessel}: datenquelle nennt {datei}.txt nicht: {quelle}"
            );
            assert_eq!(
                eintrag["veranlagungszeitraum"].as_u64(),
                Some(u64::from(jahr))
            );
        }
    }
}
