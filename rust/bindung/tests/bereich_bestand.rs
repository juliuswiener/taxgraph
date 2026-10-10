//! Die Bereichs-Ratsche (`rust/bindung/tests/bereich_bestand.yaml`): Ein Feld, das einmal einen
//! `bereich: {min, max}` trug, traegt ihn noch, und zwar genau diesen, oder es steht unter `entfernt:`
//! mit einer Begruendung.
//!
//! Was schiefgeht, wenn das fehlt: Verschwindet `bereich` bei einem Feld oder wird er weiter, nimmt der
//! Dienst danach Werte an, die er vorher mit 422 abwies (-1 Kinder, 367 Arbeitstage), und der Ring
//! rechnet still damit. Der Rand-Test (`rust/api/tests/bereich_rand_hermetisch.rs`) liest die Felder
//! live aus der Registry: faellt ein Feld heraus, faellt es dort aus der Pruefung, ohne dass er rot wird.
//! Gemessen am Mutanten "bereich von `rentner_pflegegrad` geloescht": vorher wurde nur ein eingefrorener
//! Python-Vergleich rot, kein Rust-eigener Test.
//!
//! Die Menge ist eine Einbahn-Ratsche (neue Felder duerfen dazukommen), die Grenzen sind ein genauer Pin:
//! siehe den Kopf der YAML-Datei fuer die Begruendung (enger-dann-weiter-Loch).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use bindung::{lade_registry_der_wurzel, zu_knappe_begruendungen, MIN_ZEICHEN_BEGRUENDUNG};
use serde::Deserialize;

/// Die Grenzen eines Bereichs. Die `grund`-Prosa des Bereichs gehoert nicht zum Pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Grenzen {
    min: i64,
    max: i64,
}

/// Inhalt von `bereich_bestand.yaml`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Bestand {
    #[serde(default)]
    felder: BTreeMap<String, Grenzen>,
    #[serde(default)]
    entfernt: BTreeMap<String, String>,
}

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn lade_bestand(pfad: &Path) -> Result<Bestand, String> {
    let text = std::fs::read_to_string(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
    serde_yaml_ng::from_str(&text).map_err(|e| format!("{}: {e}", pfad.display()))
}

fn bestand() -> Bestand {
    lade_bestand(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/bereich_bestand.yaml")).unwrap()
}

/// Die Felder mit `bereich`, die die Registry des Dienstes heute kennt.
fn heutige_bereiche() -> BTreeMap<String, Grenzen> {
    lade_registry_der_wurzel(&wurzel())
        .unwrap()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .filter_map(|b| {
            let bereich = b.bereich.as_ref()?;
            Some((
                b.feld_id.clone(),
                Grenzen {
                    min: bereich.min,
                    max: bereich.max,
                },
            ))
        })
        .collect()
}

/// Felder im Bestand, die heute keinen `bereich` mehr tragen (oder ganz fehlen) und nicht unter
/// `entfernt` stehen: der stille Verlust.
fn verschwundene(
    bestand: &BTreeMap<String, Grenzen>,
    heute: &BTreeMap<String, Grenzen>,
    entfernt: &BTreeMap<String, String>,
) -> Vec<String> {
    bestand
        .keys()
        .filter(|f| !heute.contains_key(*f) && !entfernt.contains_key(*f))
        .cloned()
        .collect()
}

/// Felder, die in beiden stehen und deren Grenzen nicht gleich sind, als Meldungszeile; weiter UND enger.
fn geaenderte(
    bestand: &BTreeMap<String, Grenzen>,
    heute: &BTreeMap<String, Grenzen>,
) -> Vec<String> {
    bestand
        .iter()
        .filter_map(|(f, alt)| {
            let neu = heute.get(f)?;
            (neu != alt).then(|| {
                format!(
                    "{f}: Bestand {}..{}, heute {}..{} ({})",
                    alt.min,
                    alt.max,
                    neu.min,
                    neu.max,
                    if neu.min < alt.min || neu.max > alt.max {
                        "weiter"
                    } else {
                        "enger"
                    }
                )
            })
        })
        .collect()
}

/// Tote Eintraege: Felder unter `entfernt`, die heute wieder einen `bereich` tragen.
fn wiederaufgetauchte(
    heute: &BTreeMap<String, Grenzen>,
    entfernt: &BTreeMap<String, String>,
) -> Vec<String> {
    entfernt
        .keys()
        .filter(|f| heute.contains_key(*f))
        .cloned()
        .collect()
}

#[test]
fn kein_bereich_ist_unbegruendet_verschwunden() {
    let weg = verschwundene(&bestand().felder, &heutige_bereiche(), &bestand().entfernt);
    assert!(
        weg.is_empty(),
        "{} Feld(er) tragen keinen `bereich` mehr, ohne dass jemand gesagt hat, was aus Werten ausserhalb \
         des alten Bereichs wird: {:?}\nEntweder den `bereich` wiederherstellen oder das Feld in \
         bereich_bestand.yaml unter `entfernt:` eintragen, mit Grund UND dem, was aus Werten \
         ausserhalb des alten Bereichs wird.",
        weg.len(),
        &weg[..weg.len().min(15)]
    );
}

#[test]
fn kein_bereich_wurde_still_geaendert() {
    let anders = geaenderte(&bestand().felder, &heutige_bereiche());
    assert!(
        anders.is_empty(),
        "{} Bereich(e) weichen vom Bestand ab: {anders:#?}\nAbsicht? Dann die Zeile in bereich_bestand.yaml \
         zusammen mit dem Bereich aendern und den Grund (Feld, Grenze, warum) in den Commit schreiben.",
        anders.len()
    );
}

#[test]
fn entfernte_bereiche_sind_wirklich_weg() {
    let tot = wiederaufgetauchte(&heutige_bereiche(), &bestand().entfernt);
    assert!(
        tot.is_empty(),
        "{tot:?} stehen unter `entfernt:`, tragen aber wieder einen `bereich`: Eintrag streichen und das \
         Feld unter `felder:` fuehren, sonst deckt er das naechste echte Verschwinden mit ab."
    );
}

#[test]
fn entfernte_bereiche_sind_begruendet() {
    let knapp = zu_knappe_begruendungen(&bestand().entfernt);
    assert!(
        knapp.is_empty(),
        "Begruendung fehlt oder ist zu knapp (mehr als {MIN_ZEICHEN_BEGRUENDUNG} Zeichen, und sie muss \
         sagen, was aus Werten ausserhalb des alten Bereichs wird): {knapp:?}"
    );
}

/// Gegenprobe gegen die bequemste Art, die Ratsche loszuwerden: die Liste leeren. Dann waere
/// `bestand - heute` immer leer und der Test gruen, ohne etwas zu pruefen. Dazu die Zahl der Felder
/// mit `bereich` in der Registry: ohne sie liefe jede Aussage leer wahr.
#[test]
fn die_erfassung_ist_nicht_leer() {
    let b = bestand();
    assert!(
        b.felder.len() >= 40,
        "nur {} Felder im Bereichs-Bestand (erwartet mindestens 40, bei der Erfassung waren es 42): eine \
         geleerte Liste macht die Ratsche wirkungslos, ohne dass ein Test rot wird",
        b.felder.len()
    );
    let heute = heutige_bereiche();
    assert!(
        heute.len() >= 40,
        "nur {} Felder mit `bereich` in der Registry (erwartet mindestens 40): die Pruefung liefe leer",
        heute.len()
    );
    // Kein Feld steht in `felder` und `entfernt` zugleich: dann waere unklar, was gilt.
    let doppelt: Vec<&String> = b
        .entfernt
        .keys()
        .filter(|f| b.felder.contains_key(*f))
        .collect();
    assert!(doppelt.is_empty(), "in felder UND entfernt: {doppelt:?}");
}

fn paar(feld: &str, min: i64, max: i64) -> (String, Grenzen) {
    (feld.to_string(), Grenzen { min, max })
}

/// Ohne diese Probe waere nicht belegt, dass die Vergleiche anschlagen: im Normalfall ist die Ratsche
/// gruen, man sieht sie also nie arbeiten. Hier laeuft sie auf erfundenen Mengen.
#[test]
fn die_ratsche_erkennt_ihre_eigenen_fehlerfaelle() {
    let bestand = BTreeMap::from([paar("a", 0, 20), paar("b", 1, 5), paar("c", 0, 9)]);
    let leer: BTreeMap<String, String> = BTreeMap::new();

    // Gleich: nichts zu melden.
    assert_eq!(verschwundene(&bestand, &bestand, &leer).len(), 0);
    assert_eq!(geaenderte(&bestand, &bestand).len(), 0);

    // c verliert den bereich, ohne Begruendung: Verlust. Mit Begruendung: kein Verlust.
    let ohne_c = BTreeMap::from([paar("a", 0, 20), paar("b", 1, 5)]);
    assert_eq!(
        verschwundene(&bestand, &ohne_c, &leer),
        vec!["c".to_string()]
    );
    let begruendet = BTreeMap::from([("c".to_string(), "x".repeat(40))]);
    assert_eq!(verschwundene(&bestand, &ohne_c, &begruendet).len(), 0);
    // Ein Eintrag fuer ein anderes Feld deckt c nicht.
    let fremd = BTreeMap::from([("z".to_string(), "x".repeat(40))]);
    assert_eq!(
        verschwundene(&bestand, &ohne_c, &fremd),
        vec!["c".to_string()]
    );

    // Ein neues Feld mit bereich (d, nicht im Bestand) ist kein Verlust und keine Aenderung.
    let mit_neuem = BTreeMap::from([
        paar("a", 0, 20),
        paar("b", 1, 5),
        paar("c", 0, 9),
        paar("d", 0, 3),
    ]);
    assert_eq!(verschwundene(&bestand, &mit_neuem, &leer).len(), 0);
    assert_eq!(geaenderte(&bestand, &mit_neuem).len(), 0);

    // Weiter: min gesenkt, max erhoeht. Beides wird gemeldet, mit "weiter".
    for heute in [
        BTreeMap::from([paar("a", -1, 20), paar("b", 1, 5), paar("c", 0, 9)]),
        BTreeMap::from([paar("a", 0, 21), paar("b", 1, 5), paar("c", 0, 9)]),
    ] {
        let anders = geaenderte(&bestand, &heute);
        assert_eq!(anders.len(), 1, "{anders:?}");
        assert!(
            anders[0].starts_with("a: ") && anders[0].ends_with("(weiter)"),
            "{anders:?}"
        );
    }
    // Enger wird auch gemeldet (sonst: enger, dann wieder weiter, ohne Meldung).
    for heute in [
        BTreeMap::from([paar("a", 1, 20), paar("b", 1, 5), paar("c", 0, 9)]),
        BTreeMap::from([paar("a", 0, 19), paar("b", 1, 5), paar("c", 0, 9)]),
    ] {
        let anders = geaenderte(&bestand, &heute);
        assert_eq!(anders.len(), 1, "{anders:?}");
        assert!(
            anders[0].starts_with("a: ") && anders[0].ends_with("(enger)"),
            "{anders:?}"
        );
    }
    // Ein Feld, das schon unter `entfernt` verschwunden ist, wird nicht als geaendert gemeldet.
    assert_eq!(geaenderte(&bestand, &ohne_c).len(), 0);

    // Zombie: b steht unter `entfernt`, traegt aber noch einen bereich.
    let zombie = BTreeMap::from([("b".to_string(), "x".repeat(40))]);
    assert_eq!(wiederaufgetauchte(&bestand, &zombie), vec!["b".to_string()]);
    assert_eq!(wiederaufgetauchte(&ohne_c, &begruendet).len(), 0);
}

fn temp_verzeichnis(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("bereich-bestand-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Eine fehlende Datei, ein Tippfehler im Schluessel und eine fremde Grenze (`mx:`) sind Fehler, nie ein
/// leerer Bestand: ein leerer Bestand liesse die Ratsche nichts melden.
#[test]
fn fehlende_datei_und_tippfehler_sind_fehler() {
    let d = temp_verzeichnis("laden");
    assert!(lade_bestand(&d.join("gibt_es_nicht.yaml")).is_err());
    let pfad = d.join("b.yaml");
    std::fs::write(&pfad, "felder: {a: {min: 0, max: 1}}\nentfernd: {}\n").unwrap();
    assert!(
        lade_bestand(&pfad).is_err(),
        "entfernd: muss ein Fehler sein"
    );
    std::fs::write(&pfad, "felder: {a: {min: 0, mx: 1}}\n").unwrap();
    assert!(lade_bestand(&pfad).is_err(), "mx: muss ein Fehler sein");
    std::fs::write(&pfad, "felder: {a: {min: 0, max: 1}}\nentfernt: {}\n").unwrap();
    assert_eq!(
        lade_bestand(&pfad).unwrap().felder,
        BTreeMap::from([paar("a", 0, 1)])
    );
}
