//! Die Bindung der Anlage R-AV/bAV (Abweichung Nr. 55 in `rust/fixtures/README.md`; Entscheidung
//! `renten-mit-leistungsmitteilung-werden-in-stufen-gebaut-anlage-r-av-bav-stufe-1-person-a`) gegen den VORDRUCK, nicht gegen
//! sich selbst.
//!
//! Der Vordruck steht eingefroren in `sources/bfinv/rav_bav_2025.txt`. Der Test liest daraus bei jedem Lauf, welches Vordruck-Kz in
//! der Zeile 4, 10, 12, 15 und 16 der ersten Rente steht, und prueft, dass das Bindungsfeld dieser Zeile das Schema-Kz traegt, das
//! dazu gehoert. Das Paar Vordruck-Kz / E-Nummer folgt aus gleichem Text und gleicher Reihenfolge; dass die E-Nummern im Schema
//! unter `RAV_bAV/Leist_AV_betr/Einz` stehen und ERiC das Paar annimmt, prueft `elster` (Schema) und `bescheid` (`checkESt`).
//!
//! Die Hilfe jedes Betragsfelds nennt seine Zeile. Ein Feld auf der falschen Zeile haette einen Text, der den Nutzer in die falsche
//! Zeile des Formulars schickt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bindung::Bindung;
use regex::Regex;

/// `(feld_id, Zeile, Vordruck-Kz der ersten Rente, E-Nummer im Schema)`.
const ZEILEN: [(&str, u32, u32, &str); 5] = [
    ("rav_bav_nr1_betrag", 4, 500, "E1803101"),
    ("rav_bav_nr4_betrag", 10, 505, "E1803301"),
    ("rav_bav_nr4_beginn", 12, 506, "E1803401"),
    ("rav_bav_nr5_betrag", 15, 507, "E1803501"),
    ("rav_bav_nr5_beginn", 16, 508, "E1803601"),
];

const GRUPPE: &str = "rente_av_bav";

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn bindungen() -> BTreeMap<String, Bindung> {
    bindung::lade_registry_der_wurzel(&wurzel())
        .unwrap()
        .dateien
        .into_iter()
        .flat_map(|(_, d)| d.bindungen)
        .map(|b| (b.feld_id.clone(), b))
        .collect()
}

/// Zeile -> Vordruck-Kz der ERSTEN Rente: das erste dreistellige Kz nach der Zeilennummer. Eine Zeile ohne Kz (die Zeilen 2 und 3
/// des Vordrucks tragen Name und Steuernummer) fehlt in der Karte.
fn vordruck_kz(text: &str) -> BTreeMap<u32, u32> {
    let re = Regex::new(r"^\s{10,16}(\d{1,2})\s+.*?\s(\d{3})(?:\s|$)").unwrap();
    text.lines()
        .filter_map(|l| {
            let t = re.captures(l)?;
            Some((t[1].parse().ok()?, t[2].parse().ok()?))
        })
        .collect()
}

fn vordruck() -> String {
    let pfad = wurzel().join("sources/bfinv/rav_bav_2025.txt");
    std::fs::read_to_string(&pfad)
        .unwrap_or_else(|e| panic!("{} nicht lesbar: {e}", pfad.display()))
}

/// Jede Zeile des Vordrucks traegt das erwartete Vordruck-Kz, und das Bindungsfeld dieser Zeile traegt die dazu gehoerende
/// E-Nummer, liegt in der Gruppe `rente_av_bav` und nennt seine Zeile in der Hilfe.
#[test]
fn bindungsfelder_tragen_das_kz_der_zeile_des_vordrucks() {
    let karte = vordruck_kz(&vordruck());
    let b = bindungen();
    for (feld, zeile, kz, e_nr) in ZEILEN {
        assert_eq!(
            karte.get(&zeile).copied(),
            Some(kz),
            "Vordruck Zeile {zeile}: Vordruck-Kz {kz} erwartet, gefunden {:?} (Karte {karte:?})",
            karte.get(&zeile)
        );
        let eintrag = b
            .get(feld)
            .unwrap_or_else(|| panic!("{feld}: nicht in der Bindung"));
        assert_eq!(
            eintrag.elster_kz.as_ref().map(domain::Kz::as_str),
            Some(e_nr),
            "{feld} (Zeile {zeile}): E-Nummer"
        );
        assert_eq!(
            eintrag.instanz_gruppe.as_deref(),
            Some(GRUPPE),
            "{feld}: Gruppe"
        );
        assert!(
            eintrag.hilfe_kurz.contains(&format!("Zeile {zeile} ")),
            "{feld}: die Hilfe nennt Zeile {zeile} nicht: {}",
            eintrag.hilfe_kurz
        );
    }
}

/// KONTROLLE: der Leser findet im Vordruck die Kz der ersten Rente (links), nicht die der zweiten (550, 555, ...). Ohne sie bewiese
/// der Vergleich oben nichts: ein Leser, der immer dieselbe Zahl lieferte, liesse ihn gruen, ein Leser, der die falsche Spalte las,
/// ebenso.
#[test]
fn kontrolle_der_leser_findet_die_zeilen_der_ersten_rente() {
    let karte = vordruck_kz(&vordruck());
    let gelesen: Vec<(u32, u32)> = ZEILEN.iter().map(|(_, z, _, _)| (*z, karte[z])).collect();
    assert_eq!(
        gelesen,
        [(4, 500), (10, 505), (12, 506), (15, 507), (16, 508)]
    );
    // Zeile 5 (Pensionsfonds) steht im Vordruck und ist keine Zeile der Bindung (Stufe 2): der Leser sieht sie, die Tabelle nicht.
    assert_eq!(karte.get(&5).copied(), Some(501));
    // Ein anderes Kz in der Zeile wird gelesen, nicht eine feste Zahl.
    let verfaelscht = vordruck().replace("4 teilung", "4 teilung 999");
    assert_eq!(
        vordruck_kz(&verfaelscht).get(&4).copied(),
        Some(999),
        "der Leser liest das Kz der Zeile, nicht eine feste Zahl"
    );
}

/// Die Gruppe fuehrt ihr Zaehlfeld und genau die sieben Eingabefelder; das Kreuz `kein_rav_bav` ist ein Screening-Flag.
#[test]
fn die_gruppe_fuehrt_genau_diese_felder() {
    let b = bindungen();
    let mut in_gruppe: Vec<&str> = b
        .values()
        .filter(|e| e.instanz_gruppe.as_deref() == Some(GRUPPE))
        .map(|e| e.feld_id.as_str())
        .collect();
    in_gruppe.sort_unstable();
    assert_eq!(
        in_gruppe,
        [
            "rav_bav_andere_nummern",
            "rav_bav_basisrente",
            "rav_bav_nr1_betrag",
            "rav_bav_nr4_beginn",
            "rav_bav_nr4_betrag",
            "rav_bav_nr5_beginn",
            "rav_bav_nr5_betrag",
        ]
    );
    assert!(b.contains_key("rav_bav_anzahl"), "Zaehlfeld");
    assert_eq!(b["kein_rav_bav"].screening, Some(true), "Kreuz");
    // Die zwei Rechenfelder tragen kein Kz, die fuenf Zeilenfelder je eines.
    let ohne_kz: Vec<&str> = in_gruppe
        .iter()
        .copied()
        .filter(|f| b[*f].elster_kz.is_none())
        .collect();
    assert_eq!(ohne_kz, ["rav_bav_andere_nummern", "rav_bav_basisrente"]);
}
