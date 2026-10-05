//! Deklarations-Abdeckung (Weg B voll, Stufe 3, Waechter W8): Bindung × Transform-Tabellen.
//!
//! Gegenstueck zu `tests/test_deklarations_abdeckung.py` (Python-Task #11). Fuenf Aussagen ueber die
//! stille Unter- und Ueber-Deklaration, alle ohne Schema und ohne Python, jede mit einem Mutanten am
//! echten Ort belegt:
//!
//! 1. Jedes eingetragene `elster_kz` eines fragbaren Felds erscheint in der Deklaration (kein
//!    stiller Wegfall). Der Round-Trip-Test in `tests/eigenschaften.rs` ueberspringt ein fehlendes
//!    Kz mit `continue`; der Proptest deckt nur Cent.
//! 2. Jedes fragbare Feld ohne Kz steht mit Grund in `nicht_deklariert` oder ist Quelle einer
//!    Transform-Tabelle.
//! 3. Kein Phantom-Kz: jedes Kz der Deklaration geht auf eine Bindung, ein Transform-Ziel oder eine
//!    benannte Konstante zurueck.
//! 4. Die Kz der Person B (`PARTNER_INSTANZ`) sind Kz der Person A.
//! 5. Die Transform-Konfiguration ist stimmig: ihre Quellfelder gibt es in der Bindung, kein Kz hat
//!    zwei Felder, kein Transform-Ziel kollidiert mit einem 1:1-Kz.
//!
//! Die Eingabe ist der Python-Fall: alle fragbaren Felder bestaetigt, mit ihrem `beispielwert`.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Herkunft, PruefTiefe, Zustand};
use store::SnapshotFeld;

use crate::tabellen::{
    DOKUMENTIERT_AGGREGAT, MULTIPLIKATION, NEGATION, P23_BETRAGSFELDER, PARTNER_INSTANZ,
    PARTNER_VERZWEIGUNG, VERZWEIGUNG,
};
use crate::{deklariere, Deklaration, Felder, IBAN_TRANSFORM_ZIEL_KZ, KONSTANTE_KZ};

/// Die Bindung des Dienstes (`rust/bindung/daten`), nie ein fester Pfad im Test.
fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

fn kz_von(b: &Bindung) -> Option<&str> {
    b.elster_kz.as_ref().map(domain::Kz::as_str)
}

/// Der Python-Fall: jedes fragbare Feld bestaetigt, mit seinem `beispielwert`.
fn snapshot() -> Felder {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    let herkunft = Herkunft {
        herkunft: a("laie"),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: a("nutzer"),
    };
    bindungen()
        .iter()
        .filter(|b| b.askable)
        .map(|b| {
            (
                b.feld_id.clone(),
                SnapshotFeld {
                    wert: b.beispielwert.clone().into(),
                    zustand: Zustand::Bestaetigt,
                    herkunft: herkunft.clone().into(),
                },
            )
        })
        .collect()
}

fn deklaration() -> &'static Deklaration {
    static CELL: OnceLock<Deklaration> = OnceLock::new();
    CELL.get_or_init(|| deklariere(&snapshot(), index(), 2025, None).expect("deklariert"))
}

/// Felder, die eine Transform-Tabelle als Quelle liest (Klassen d, e, a, f, g, g×f, h, j).
fn transform_quellen() -> BTreeSet<&'static str> {
    let mut q: BTreeSet<&'static str> = BTreeSet::new();
    q.extend(NEGATION.iter().map(|(f, _)| *f));
    q.extend(MULTIPLIKATION.iter().copied());
    q.extend(DOKUMENTIERT_AGGREGAT.iter().flat_map(|(_, f)| f.iter().copied()));
    for v in VERZWEIGUNG.iter().chain(PARTNER_VERZWEIGUNG) {
        q.insert(v.feld);
        q.insert(v.art_feld);
    }
    q.extend(PARTNER_INSTANZ.iter().map(|(f, _)| *f));
    q.extend(P23_BETRAGSFELDER.iter().copied());
    q.insert("stammdaten_iban");
    q
}

/// Die Ziel-Kz der Verzweigung (Person A); die der Person B sind dieselben.
fn verzweigung_ziel_kz() -> BTreeSet<&'static str> {
    VERZWEIGUNG
        .iter()
        .flat_map(|v| v.kz.paare().into_iter().map(|(_, kz)| kz))
        .collect()
}

#[test]
fn es_gibt_ueberhaupt_bindungen_und_der_fall_traegt_sie() {
    // Positivkontrolle: ohne sie waeren alle Aussagen unten leer wahr.
    assert!(
        bindungen().len() >= 300,
        "nur {} Bindungen geladen",
        bindungen().len()
    );
    let n = bindungen().iter().filter(|b| b.askable).count();
    assert!(n >= 150, "nur {n} fragbare Felder");
    assert!(
        deklaration().deklaration.len() >= 100,
        "die Deklaration des Python-Falls traegt nur {} Kz",
        deklaration().deklaration.len()
    );
}

/// 1. Kein stiller Wegfall eines eingetragenen Kz.
#[test]
fn jedes_eingetragene_kz_wird_deklariert() {
    let d = deklaration();
    let unter: Vec<String> = bindungen()
        .iter()
        .filter(|b| b.askable)
        .filter_map(|b| kz_von(b).map(|kz| (b, kz)))
        .filter(|(_, kz)| !d.deklaration.contains_key(*kz))
        .map(|(b, kz)| format!("{} ({kz})", b.feld_id))
        .collect();
    assert!(
        unter.is_empty(),
        "eingetragene Kz fallen still aus der Deklaration (Unter-Deklaration): {unter:?}"
    );
}

/// 2. Jedes fragbare Feld ohne Kz ist bewusst nicht deklariert (mit Grund) oder Transform-Quelle.
#[test]
fn jedes_feld_ohne_kz_ist_nicht_deklariert_oder_transform_quelle() {
    let d = deklaration();
    let nicht: BTreeSet<&str> = d.nicht_deklariert.iter().map(|e| e.feld_id.as_str()).collect();
    let quellen = transform_quellen();
    let verschwunden: Vec<&str> = bindungen()
        .iter()
        .filter(|b| b.askable && kz_von(b).is_none())
        .map(|b| b.feld_id.as_str())
        .filter(|f| !nicht.contains(f) && !quellen.contains(f))
        .collect();
    assert!(
        verschwunden.is_empty(),
        "Felder ohne Kz weder nicht_deklariert noch Transform-Quelle (still verschwunden): {verschwunden:?}"
    );
}

/// 3. Kein Phantom-Kz in der Deklaration.
#[test]
fn kein_phantom_kz_in_der_deklaration() {
    let mut erlaubt: BTreeSet<&str> = bindungen().iter().filter_map(kz_von).collect();
    erlaubt.extend(NEGATION.iter().map(|(_, kz)| *kz));
    erlaubt.extend(verzweigung_ziel_kz());
    erlaubt.extend(KONSTANTE_KZ.iter().copied());
    erlaubt.extend(IBAN_TRANSFORM_ZIEL_KZ.iter().copied());
    let phantome: Vec<&str> = deklaration()
        .deklaration
        .keys()
        .map(String::as_str)
        .filter(|kz| !erlaubt.contains(kz))
        .collect();
    assert!(
        phantome.is_empty(),
        "Phantom-Kz in der Deklaration ohne Bindungs- oder Transform-Herkunft: {phantome:?}"
    );
}

/// 4. Die Kz der Person B sind Kz der Person A (Person B ist ein zweites Sub-Dokument mit denselben Kz).
#[test]
fn person_b_instanz_kz_sind_person_a_kz() {
    let a_kz: BTreeSet<&str> = bindungen().iter().filter_map(kz_von).collect();
    let fremd: Vec<&str> = PARTNER_INSTANZ
        .iter()
        .map(|(_, kz)| *kz)
        .filter(|kz| !a_kz.contains(kz))
        .collect();
    assert!(
        fremd.is_empty(),
        "Person-B-Kz ohne Person-A-Entsprechung (Phantom): {fremd:?}"
    );
}

/// 5. Die Transform-Konfiguration ist stimmig.
#[test]
fn transform_konfig_ist_konsistent() {
    let ids: BTreeSet<&str> = bindungen().iter().map(|b| b.feld_id.as_str()).collect();
    let fehlt: Vec<&str> = transform_quellen()
        .into_iter()
        .filter(|f| !ids.contains(f))
        .collect();
    assert!(
        fehlt.is_empty(),
        "Transform-Quellen fehlen in der Bindung: {fehlt:?}"
    );
    let mut je_kz: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for b in bindungen() {
        if let Some(kz) = kz_von(b) {
            je_kz.entry(kz).or_default().push(&b.feld_id);
        }
    }
    let kollisionen: BTreeMap<&str, &Vec<&str>> =
        je_kz.iter().filter(|(_, f)| f.len() > 1).map(|(k, f)| (*k, f)).collect();
    assert!(
        kollisionen.is_empty(),
        "1:1-Kz-Kollision (mehrere Felder je Kz): {kollisionen:?}"
    );
    let mut ziel: BTreeSet<&str> = NEGATION.iter().map(|(_, kz)| *kz).collect();
    ziel.extend(DOKUMENTIERT_AGGREGAT.iter().map(|(kz, _)| *kz));
    ziel.extend(verzweigung_ziel_kz());
    let kollidiert: Vec<&&str> = ziel.iter().filter(|kz| je_kz.contains_key(*kz)).collect();
    assert!(
        kollidiert.is_empty(),
        "Transform-Ziel-Kz kollidiert mit einem 1:1-Kz: {kollidiert:?}"
    );
}
