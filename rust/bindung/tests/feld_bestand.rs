//! Die Feld-Bestand-Ratsche (`rust/bindung/daten/FELD_BESTAND.yaml`): Was einmal in der Bindung
//! stand, steht entweder noch da oder unter `entfernt:` mit einer Begruendung.
//!
//! Was schiefgeht, wenn das fehlt: Wer ein Feld aus der Bindung streicht, loescht damit die
//! Angaben, die Nutzer dazu gemacht haben. Sie bleiben im Store liegen und wirken nicht mehr; der
//! Dienst stuerzt nicht ab und meldet nichts. Der Test erzwingt, dass jemand das Streichen
//! begruendet und sagt, was aus vorhandenen Angaben wird.
//!
//! Die Ratsche kennt nur diese eine Richtung. Ein neues Feld braucht keinen Eintrag, es kann
//! nichts verlieren; eine Liste, die jedes neue Feld mitfuehren muesste, verrottete.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use bindung::{
    lade_feld_bestand, lade_feld_bestand_der_wurzel, lade_registry_der_wurzel, verschwundene,
    wiederaufgetauchte, zu_knappe_begruendungen, FeldBestandFehler, MIN_ZEICHEN_BEGRUENDUNG,
};

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Die `feld_id`s, die die Registry des Dienstes heute kennt.
fn heutige_felder() -> BTreeSet<String> {
    lade_registry_der_wurzel(&wurzel())
        .unwrap()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter().map(|b| b.feld_id.clone()))
        .collect()
}

fn menge(namen: &[&str]) -> BTreeSet<String> {
    namen.iter().map(|n| (*n).to_string()).collect()
}

#[test]
fn kein_feld_ist_unbegruendet_verschwunden() {
    let bestand = lade_feld_bestand_der_wurzel(&wurzel()).unwrap();
    let erfasst: BTreeSet<String> = bestand.felder.iter().cloned().collect();
    let weg = verschwundene(&erfasst, &heutige_felder(), &bestand.entfernt);
    assert!(
        weg.is_empty(),
        "{} feld_id(s) sind aus der Bindung verschwunden, ohne dass jemand gesagt hat, was aus \
         vorhandenen Nutzerangaben wird: {:?}\nEntweder das Feld wiederherstellen oder es in \
         FELD_BESTAND.yaml unter `entfernt:` eintragen, mit Grund UND mit dem, was mit bereits \
         erfassten Angaben geschieht.",
        weg.len(),
        &weg[..weg.len().min(15)]
    );
}

#[test]
fn entfernte_felder_sind_begruendet() {
    let bestand = lade_feld_bestand_der_wurzel(&wurzel()).unwrap();
    let knapp = zu_knappe_begruendungen(&bestand.entfernt);
    assert!(
        knapp.is_empty(),
        "Begruendung fehlt oder ist zu knapp (mehr als {MIN_ZEICHEN_BEGRUENDUNG} Zeichen, und sie \
         muss sagen, was mit bereits erfassten Angaben geschieht): {knapp:?}"
    );
}

#[test]
fn entfernte_felder_sind_wirklich_weg() {
    let bestand = lade_feld_bestand_der_wurzel(&wurzel()).unwrap();
    let tot = wiederaufgetauchte(&heutige_felder(), &bestand.entfernt);
    assert!(
        tot.is_empty(),
        "{tot:?} stehen unter `entfernt:`, sind aber wieder in der Bindung: Eintrag streichen, \
         sonst deckt er das naechste echte Verschwinden mit ab."
    );
}

/// Gegenprobe gegen die bequemste Art, die Ratsche loszuwerden: die Liste leeren. Dann waere
/// `erfasst - heute` immer leer und der Test gruen, ohne etwas zu pruefen.
#[test]
fn die_erfassung_ist_nicht_leer() {
    let bestand = lade_feld_bestand_der_wurzel(&wurzel()).unwrap();
    assert!(
        bestand.felder.len() >= 250,
        "nur {} Felder erfasst (erwartet mindestens 250, bei der Erfassung waren es 307): eine \
         geleerte Liste macht die Ratsche wirkungslos, ohne dass ein Test rot wird",
        bestand.felder.len()
    );
    // Die Liste hat keine Doppelten: ein Doppelter wuerde die Zahl oben aufblaehen.
    let eindeutig: BTreeSet<&String> = bestand.felder.iter().collect();
    assert_eq!(
        eindeutig.len(),
        bestand.felder.len(),
        "doppelte feld_id in felder:"
    );
}

/// Ohne diese Probe waere nicht belegt, dass der Vergleich anschlaegt: im Normalfall ist die
/// Ratsche gruen, man sieht sie also nie arbeiten. Hier laeuft sie auf erfundenen Mengen.
#[test]
fn die_ratsche_erkennt_ihren_eigenen_fehlerfall() {
    let erfasst = menge(&["a", "b", "c"]);
    let heute = menge(&["a", "b"]);

    // c fehlt und ist nicht begruendet: Verlust.
    let leer: BTreeMap<String, String> = BTreeMap::new();
    assert_eq!(
        verschwundene(&erfasst, &heute, &leer),
        vec!["c".to_string()]
    );

    // c ist begruendet entfernt: kein Verlust.
    let entfernt = BTreeMap::from([("c".to_string(), "x".repeat(40))]);
    assert!(verschwundene(&erfasst, &heute, &entfernt).is_empty());

    // Ein Eintrag fuer ein anderes Feld deckt c nicht.
    let fremd = BTreeMap::from([("z".to_string(), "x".repeat(40))]);
    assert_eq!(
        verschwundene(&erfasst, &heute, &fremd),
        vec!["c".to_string()]
    );

    // Ein neues Feld (d, nicht erfasst) ist kein Verlust.
    let mit_neuem = menge(&["a", "b", "d"]);
    assert_eq!(
        verschwundene(&erfasst, &mit_neuem, &leer),
        vec!["c".to_string()]
    );

    // Zombie: b steht unter `entfernt`, ist aber noch da.
    let zombie = BTreeMap::from([("b".to_string(), "x".repeat(40))]);
    assert_eq!(wiederaufgetauchte(&heute, &zombie), vec!["b".to_string()]);
    assert!(wiederaufgetauchte(&heute, &entfernt).is_empty());

    // Begruendung: 29 und genau 30 Zeichen sind zu knapp, 31 und 32 reichen. Die Zahlen stehen hier
    // ausgeschrieben und nicht als Konstante des Laders: wanderte die Konstante, wanderte sonst die
    // Erwartung mit.
    for (laenge, knapp) in [(29, true), (30, true), (31, false), (32, false)] {
        let eintrag = BTreeMap::from([("c".to_string(), "x".repeat(laenge))]);
        assert_eq!(
            zu_knappe_begruendungen(&eintrag),
            if knapp { vec!["c".to_string()] } else { vec![] },
            "{laenge} Zeichen"
        );
    }
    assert_eq!(MIN_ZEICHEN_BEGRUENDUNG, 30);

    // Gezaehlt wird nach Zeichen, nicht nach Bytes: 16 Umlaute sind 32 Bytes, aber nur 16 Zeichen.
    let wenige_umlaute = BTreeMap::from([("c".to_string(), "ä".repeat(16))]);
    assert_eq!(
        zu_knappe_begruendungen(&wenige_umlaute),
        vec!["c".to_string()]
    );
    let viele_umlaute = BTreeMap::from([("c".to_string(), "ä".repeat(31))]);
    assert!(zu_knappe_begruendungen(&viele_umlaute).is_empty());
}

fn temp_verzeichnis(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("feld-bestand-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Fehlt `FELD_BESTAND.yaml`, ist das ein Fehler: eine fehlende Datei wuerde sonst als leere
/// Liste gelesen, und die Ratsche meldete nichts.
#[test]
fn fehlende_datei_ist_ein_fehler() {
    let fehler = lade_feld_bestand(&temp_verzeichnis("leer")).unwrap_err();
    assert!(matches!(fehler, FeldBestandFehler::Io { .. }), "{fehler}");
}

/// Ein Tippfehler im Schluessel (`entfernd:` statt `entfernt:`) darf nicht still einen leeren
/// Abschnitt ergeben: dann gaelte ein begruendetes Entfernen als unbegruendet, oder schlimmer,
/// eine Liste verschwaende unbemerkt.
#[test]
fn unbekannter_schluessel_ist_ein_fehler() {
    let d = temp_verzeichnis("tippfehler");
    std::fs::write(d.join("FELD_BESTAND.yaml"), "felder: [a]\nentfernd: {}\n").unwrap();
    let fehler = lade_feld_bestand(&d).unwrap_err();
    assert!(matches!(fehler, FeldBestandFehler::Yaml { .. }), "{fehler}");
    std::fs::write(d.join("FELD_BESTAND.yaml"), "felder: [a]\nentfernt: {}\n").unwrap();
    assert_eq!(lade_feld_bestand(&d).unwrap().felder, vec!["a".to_string()]);
}
