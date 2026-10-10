//! Die Freigrenze der privaten Veraeusserungsgeschaefte (§ 23 Abs. 3 Satz 5 `EStG`) gegen das GESETZ, nicht gegen die YAML
//! (Abweichung Nr. 54 in `rust/fixtures/README.md`; Entscheidung `privatverkauf-wird-in-stufen-gebaut-hauptperson-zuerst-krypto-und-ehegatte-bleiben-gesperrt`,
//! Punkt 6).
//!
//! Wie `parteispenden_werte_gegen_gesetz.rs`: der Erwartungswert steht nicht im Test. Er wird bei jedem Lauf aus dem eingefrorenen
//! Wortlaut gelesen und mit dem verglichen, was `bindung::Params` fuer das Jahr liefert (der Lesepfad, den die Rechnung nimmt).
//!
//! WELCHE FASSUNG FUER WELCHES JAHR. Die amtliche Seite zeigt nur die geltende Fassung. Fuer 2024 und 2025 liegen deshalb Archivkopien
//! dieser Seite (`web.archive.org`, 2024-12-23 und 2025-12-27), fuer 2026 die Abrufkopie vom 2026-07-13. Die Kopie vom Jahresende gilt
//! fuer dieses Jahr (ABGELEITET aus den Abrufdaten, nicht aus dem Gesetzestext). Gemessen 2026-10-10: alle drei Fassungen sind
//! wortgleich; die Freigrenze ist in allen drei Jahren 1 000 Euro. Eine Zahl der neuesten Fassung wird nicht auf fruehere Jahre
//! uebertragen, bevor die Fassung je Jahr vorliegt: dieser Test haelt das fest.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};

use bindung::Params;
use domain::{Euro, Vz};
use regex::Regex;

/// `(Veranlagungsjahr, Vz, Datei des Wortlauts)`.
const JAHRE: [(u16, Vz, &str); 3] = [
    (2024, Vz::Vz2024, "estg_p23_2024-12-23"),
    (2025, Vz::Vz2025, "estg_p23_2025-12-27"),
    (2026, Vz::Vz2026, "estg_p23_2026-07-13"),
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

/// Die Freigrenze aus Satz 5 in Euro. Ein Fehler heisst: der Text ist nicht mehr der Satz, den der Parser kennt (nie ein stiller
/// Standardwert).
fn freigrenze(text: &str) -> Result<i64, String> {
    let wortlaut = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let re = Regex::new(
        r"Gewinne bleiben steuerfrei, wenn der aus den privaten Veräußerungsgeschäften erzielte Gesamtgewinn im Kalenderjahr weniger als (\d+(?: \d{3})*) Euro betragen hat",
    )
    .unwrap();
    let t = re.captures(&wortlaut).ok_or("Satz 5 nicht gefunden")?;
    t[1].replace(' ', "")
        .parse::<i64>()
        .map_err(|e| format!("keine Zahl {:?}: {e}", &t[1]))
}

/// Die Freigrenze jedes Jahres gleich dem Wortlaut seiner Fassung.
#[test]
fn freigrenze_jedes_jahres_ist_der_wert_im_gesetz() {
    let p = Params::lade(&wurzel()).unwrap();
    for (jahr, vz, datei) in JAHRE {
        let im_gesetz = freigrenze(&quelle(datei)).unwrap();
        assert_eq!(
            p.privatverkauf_freigrenze(vz).unwrap(),
            Euro::new(im_gesetz),
            "{jahr}: Freigrenze (Wortlaut {datei}: {im_gesetz} Euro)"
        );
    }
}

/// KONTROLLE: der Parser liest die Zahl, die der Wortlaut tatsaechlich traegt. Ohne sie bewiese der Vergleich oben nichts: ein Parser,
/// der immer 1 000 lieferte, und eine Datei mit 1 000 liessen ihn gruen.
#[test]
fn kontrolle_der_parser_liest_den_wortlaut() {
    let gelesen: Vec<_> = JAHRE
        .iter()
        .map(|(_, _, d)| freigrenze(&quelle(d)).unwrap())
        .collect();
    assert_eq!(gelesen, [1_000, 1_000, 1_000], "2024, 2025, 2026");
    // Ein anderer Betrag im Satz wird gelesen, nicht ueberschrieben (die Fassung bis 2022 nannte 600 Euro).
    let alt = quelle(JAHRE[2].2).replace("weniger als 1 000 Euro", "weniger als 600 Euro");
    assert_eq!(freigrenze(&alt).unwrap(), 600);
    // Der Parser meldet einen Text ohne den Satz als Fehler und liefert keinen Wert.
    assert!(freigrenze("Gewinne bleiben steuerfrei.").is_err());
}
