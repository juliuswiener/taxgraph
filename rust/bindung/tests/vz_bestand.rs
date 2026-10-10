//! Die Jahres-Ratsche (`rust/bindung/tests/vz_bestand.yaml`): Jedes Feld traegt in `vz_gueltigkeit` genau die
//! Jahre des Standards, oder es steht unter `abweichend:` mit genau seinen Jahren und einem Grund.
//!
//! Was schiefgeht, wenn das fehlt: Liest niemand die Liste, wandert das Jahr eines Felds ohne Meldung. Gemessen am
//! 2026-10-06: Von 10 Aenderungen an 8 Feldern (Jahr gestrichen, Jahr dazu, 2026 gestrichen, nur 2026, Jahr ohne
//! Schema) liess keine einen Rust-Test rot werden; nur die leere Liste weist der Lader ab. Wird `2024` still
//! gestrichen, laeuft das Pruefwerkzeug fuer das Schema (`elster::pruefe_bindung`) das Jahr nicht mehr ab. Wird
//! `2024` still dazugeschrieben, behauptet das Feld ein Jahr, in dem das Kz im Schema fehlt.
//!
//! Die Ratsche pinnt beide Richtungen (weiter UND enger) und die Menge (ein Feld ohne Eintrag und mit anderen Jahren
//! als der Standard ist rot). Das Gegenstueck gegen das amtliche Schema steht in
//! `rust/elster/tests/vz_gueltigkeit_schema.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bindung::{lade_registry_der_wurzel, zu_knappe_begruendungen, MIN_ZEICHEN_BEGRUENDUNG};
use serde::Deserialize;

type Jahre = BTreeSet<i64>;

/// Eine Abweichung vom Standard: die genauen Jahre und warum.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Abweichung {
    jahre: Vec<i64>,
    grund: String,
}

/// Inhalt von `vz_bestand.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bestand {
    standard: Vec<i64>,
    #[serde(default)]
    abweichend: BTreeMap<String, Abweichung>,
}

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn lade_bestand(pfad: &Path) -> Result<Bestand, String> {
    let text = std::fs::read_to_string(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", pfad.display()))
}

fn bestand() -> Bestand {
    lade_bestand(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vz_bestand.yaml")).unwrap()
}

fn menge(jahre: &[i64]) -> Jahre {
    jahre.iter().copied().collect()
}

/// Die Jahre jedes Felds, die die Registry des Dienstes heute kennt.
fn heutige_jahre() -> BTreeMap<String, Jahre> {
    lade_registry_der_wurzel(&wurzel())
        .unwrap()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .map(|b| (b.feld_id.clone(), menge(&b.vz_gueltigkeit)))
        .collect()
}

/// Die Jahre, die ein Feld nach dem Bestand traegt.
fn soll_jahre(bestand: &Bestand, feld: &str) -> Jahre {
    bestand
        .abweichend
        .get(feld)
        .map_or_else(|| menge(&bestand.standard), |a| menge(&a.jahre))
}

fn art(soll: &Jahre, heute: &Jahre) -> &'static str {
    if soll.is_subset(heute) {
        "weiter"
    } else if heute.is_subset(soll) {
        "enger"
    } else {
        "anders"
    }
}

/// Felder, deren Jahre nicht dem Bestand entsprechen, als Meldungszeile; weiter, enger und anders.
fn geaenderte(bestand: &Bestand, heute: &BTreeMap<String, Jahre>) -> Vec<String> {
    heute
        .iter()
        .filter_map(|(f, jetzt)| {
            let soll = soll_jahre(bestand, f);
            (&soll != jetzt).then(|| {
                format!(
                    "{f}: Bestand {soll:?}, heute {jetzt:?} ({})",
                    art(&soll, jetzt)
                )
            })
        })
        .collect()
}

/// Eintraege unter `abweichend`, die ins Leere zeigen: das Feld gibt es nicht mehr, oder die Jahre gleichen dem
/// Standard (dann deckt der Eintrag die naechste echte Abweichung des Felds mit ab).
fn tote_eintraege(bestand: &Bestand, heute: &BTreeMap<String, Jahre>) -> Vec<String> {
    let standard = menge(&bestand.standard);
    bestand
        .abweichend
        .iter()
        .filter_map(|(f, a)| {
            if !heute.contains_key(f) {
                Some(format!("{f}: das Feld gibt es nicht mehr"))
            } else if menge(&a.jahre) == standard {
                Some(format!("{f}: die Jahre gleichen dem Standard"))
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn keine_jahresliste_wurde_still_geaendert() {
    let anders = geaenderte(&bestand(), &heutige_jahre());
    assert!(
        anders.is_empty(),
        "{} Feld(er) weichen vom Jahres-Bestand ab: {:#?}\nAbsicht? Dann vz_bestand.yaml zusammen mit der \
         Bindung aendern: ein Feld mit anderen Jahren als `standard` braucht einen Eintrag unter `abweichend:` \
         (genau seine Jahre und den Grund); der Grund (Feld, Jahr, warum) gehoert auch in den Commit.",
        anders.len(),
        &anders[..anders.len().min(15)]
    );
}

#[test]
fn abweichungen_zeigen_auf_ein_feld_und_weichen_wirklich_ab() {
    let tot = tote_eintraege(&bestand(), &heutige_jahre());
    assert!(
        tot.is_empty(),
        "Eintraege unter `abweichend:` ohne Wirkung: {tot:?}\nStreichen, sonst deckt der Eintrag die naechste \
         echte Abweichung des Felds mit ab."
    );
}

/// Abweichungen, deren Grund zu knapp ist.
fn knappe_gruende(bestand: &Bestand) -> Vec<String> {
    let gruende: BTreeMap<String, String> = bestand
        .abweichend
        .iter()
        .map(|(f, a)| (f.clone(), a.grund.clone()))
        .collect();
    zu_knappe_begruendungen(&gruende)
}

#[test]
fn abweichungen_sind_begruendet() {
    let knapp = knappe_gruende(&bestand());
    assert!(
        knapp.is_empty(),
        "Begruendung fehlt oder ist zu knapp (mehr als {MIN_ZEICHEN_BEGRUENDUNG} Zeichen, und sie muss sagen, \
         woher die Abweichung kommt): {knapp:?}"
    );
}

/// Gegenprobe gegen die bequemste Art, die Ratsche loszuwerden: den Standard leeren oder die Registry leer
/// laden. Dann liefe jede Aussage leer wahr. Der Standard muss ausserdem jedes Jahr nennen, das die Dienste
/// rechnen (2024 bis 2026), und die meisten Felder muessen ihn tragen.
#[test]
fn die_erfassung_ist_nicht_leer() {
    let b = bestand();
    assert_eq!(
        menge(&b.standard),
        menge(&[2024, 2025, 2026]),
        "der Standard nennt nicht mehr 2024, 2025 und 2026: Absicht (Jahreswechsel)? Dann diese Probe mitziehen"
    );
    let standard = menge(&b.standard);
    let mit_standard = heutige_jahre().values().filter(|j| **j == standard).count();
    assert!(
        mit_standard >= 360,
        "nur {mit_standard} Felder tragen den Standard (erwartet mindestens 360, bei der Erfassung waren es 366 \
         von 368): die Pruefung liefe leer"
    );
}

fn abw(jahre: &[i64], grund: &str) -> Abweichung {
    Abweichung {
        jahre: jahre.to_vec(),
        grund: grund.to_string(),
    }
}

/// Ohne diese Probe waere nicht belegt, dass die Vergleiche anschlagen: im Normalfall ist die Ratsche
/// gruen, man sieht sie also nie arbeiten. Hier laeuft sie auf erfundenen Mengen.
#[test]
fn die_ratsche_erkennt_ihre_eigenen_fehlerfaelle() {
    let bestand = Bestand {
        standard: vec![2024, 2025, 2026],
        abweichend: BTreeMap::from([("b".to_string(), abw(&[2025, 2026], &"x".repeat(40)))]),
    };
    let ist = BTreeMap::from([
        ("a".to_string(), menge(&[2024, 2025, 2026])),
        ("b".to_string(), menge(&[2025, 2026])),
    ]);
    // Gleich: nichts zu melden, die Reihenfolge der Jahre zaehlt nicht.
    assert_eq!(geaenderte(&bestand, &ist).len(), 0);
    let umsortiert = BTreeMap::from([
        ("a".to_string(), menge(&[2026, 2024, 2025])),
        ("b".to_string(), menge(&[2026, 2025])),
    ]);
    assert_eq!(geaenderte(&bestand, &umsortiert).len(), 0);
    assert_eq!(tote_eintraege(&bestand, &ist).len(), 0);

    // Enger: a verliert 2024. Weiter: b bekommt 2024. Anders: a tauscht 2024 gegen 2027. Jedes wird gemeldet.
    for (heute, endung) in [
        (
            BTreeMap::from([
                ("a".to_string(), menge(&[2025, 2026])),
                ("b".to_string(), menge(&[2025, 2026])),
            ]),
            "(enger)",
        ),
        (
            BTreeMap::from([
                ("a".to_string(), menge(&[2024, 2025, 2026])),
                ("b".to_string(), menge(&[2024, 2025, 2026])),
            ]),
            "(weiter)",
        ),
        (
            BTreeMap::from([
                ("a".to_string(), menge(&[2025, 2026, 2027])),
                ("b".to_string(), menge(&[2025, 2026])),
            ]),
            "(anders)",
        ),
    ] {
        let anders = geaenderte(&bestand, &heute);
        assert_eq!(anders.len(), 1, "{anders:?}");
        assert!(anders[0].ends_with(endung), "{anders:?} soll {endung}");
    }
    // Ein Feld nur fuer 2026 (nicht im Standard, ohne Eintrag): gemeldet, auch wenn es neu ist.
    let neu = BTreeMap::from([
        ("a".to_string(), menge(&[2024, 2025, 2026])),
        ("b".to_string(), menge(&[2025, 2026])),
        ("c".to_string(), menge(&[2026])),
    ]);
    let anders = geaenderte(&bestand, &neu);
    assert_eq!(anders.len(), 1, "{anders:?}");
    assert!(anders[0].starts_with("c: "), "{anders:?}");
    // Mit Eintrag ist es erlaubt.
    let mut mit_eintrag = Bestand {
        standard: vec![2024, 2025, 2026],
        abweichend: bestand.abweichend.clone(),
    };
    mit_eintrag
        .abweichend
        .insert("c".to_string(), abw(&[2026], &"x".repeat(40)));
    assert_eq!(geaenderte(&mit_eintrag, &neu).len(), 0);
    // Der Eintrag gilt genau: ein Feld mit anderen Jahren als im Eintrag ist rot.
    let falsch = BTreeMap::from([
        ("a".to_string(), menge(&[2024, 2025, 2026])),
        ("b".to_string(), menge(&[2025, 2026])),
        ("c".to_string(), menge(&[2025, 2026])),
    ]);
    assert_eq!(geaenderte(&mit_eintrag, &falsch).len(), 1);

    // Tote Eintraege: ein Feld, das es nicht mehr gibt, und ein Eintrag mit den Jahren des Standards.
    let nur_a = BTreeMap::from([("a".to_string(), menge(&[2024, 2025, 2026]))]);
    let tot = tote_eintraege(&bestand, &nur_a);
    assert_eq!(tot.len(), 1, "{tot:?}");
    assert!(tot[0].starts_with("b: "), "{tot:?}");
    let ueberfluessig = Bestand {
        standard: vec![2024, 2025, 2026],
        abweichend: BTreeMap::from([("a".to_string(), abw(&[2026, 2025, 2024], &"x".repeat(40)))]),
    };
    assert_eq!(tote_eintraege(&ueberfluessig, &ist).len(), 1);

    // Knappe Begruendungen werden gefunden; 30 Zeichen reichen nicht, 31 schon.
    let knapp = Bestand {
        standard: vec![2024],
        abweichend: BTreeMap::from([
            ("k1".to_string(), abw(&[2025], "kurz")),
            (
                "k2".to_string(),
                abw(&[2025], &"x".repeat(MIN_ZEICHEN_BEGRUENDUNG)),
            ),
            (
                "k3".to_string(),
                abw(&[2025], &"x".repeat(MIN_ZEICHEN_BEGRUENDUNG + 1)),
            ),
        ]),
    };
    assert_eq!(knappe_gruende(&knapp).len(), 2);
}

fn temp_verzeichnis(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("vz-bestand-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Eine fehlende Datei, ein Tippfehler im Schluessel, ein fehlender Standard und ein fremder Schluessel in einer
/// Abweichung sind Fehler, nie ein leerer Bestand: ein leerer Bestand liesse die Ratsche nichts melden.
#[test]
fn fehlende_datei_und_tippfehler_sind_fehler() {
    let d = temp_verzeichnis("laden");
    assert!(lade_bestand(&d.join("gibt_es_nicht.yaml")).is_err());
    let pfad = d.join("b.yaml");
    std::fs::write(&pfad, "abweichend: {}\n").unwrap();
    assert!(
        lade_bestand(&pfad).is_err(),
        "ohne standard: muss ein Fehler sein"
    );
    std::fs::write(&pfad, "standard: [2025]\nabweichend_: {}\n").unwrap();
    assert!(
        lade_bestand(&pfad).is_err(),
        "abweichend_: muss ein Fehler sein"
    );
    std::fs::write(
        &pfad,
        "standard: [2025]\nabweichend: {a: {jahre: [2026], grundd: x}}\n",
    )
    .unwrap();
    assert!(lade_bestand(&pfad).is_err(), "grundd: muss ein Fehler sein");
    std::fs::write(
        &pfad,
        "standard: [2025]\nabweichend: {a: {jahre: [2026]}}\n",
    )
    .unwrap();
    assert!(
        lade_bestand(&pfad).is_err(),
        "ohne grund: muss ein Fehler sein"
    );
    std::fs::write(
        &pfad,
        "standard: [2025]\nabweichend: {a: {jahre: [2026], grund: x}}\n",
    )
    .unwrap();
    assert_eq!(lade_bestand(&pfad).unwrap().abweichend.len(), 1);
}
