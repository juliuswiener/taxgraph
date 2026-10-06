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
//!
//! Dazu die Lesestellen im Quelltext (Auftrag 4, Aussagen 7 bis 9, ganz unten): Jede Stelle, an der
//! Rust-Code ein Feld mit seinem Namen nennt, haengt in beide Richtungen an der Registry.
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use bindung::{Bindung, Bindungspunkt};
use domain::{Achsenwert, Herkunft, PruefTiefe, Zustand};
use store::SnapshotFeld;

use crate::kz_wache::{abtasten_mit, modul_dateien, rs_dateien};
use crate::tabellen::{
    DOKUMENTIERT_AGGREGAT, MULTIPLIKATION, NEGATION, P23_BETRAGSFELDER, PARTNER_INSTANZ,
    P23_ART_FELD, PARTNER_VERZWEIGUNG, VERZWEIGUNG,
};
use crate::{deklariere, Deklaration, Felder, IBAN_TRANSFORM_ZIEL_KZ, KONSTANTE_KZ};

/// Die Registry des Dienstes (`rust/bindung/daten`), nie ein fester Pfad im Test.
fn registry() -> &'static bindung::Registry {
    static CELL: OnceLock<bindung::Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel).expect("Bindung laedt")
    })
}

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        registry()
            .dateien
            .iter()
            .flat_map(|(_, d)| d.bindungen.iter().cloned())
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
    q.insert(P23_ART_FELD);
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

/// Transform-Quellen, die BEWUSST nicht fragbar sind: `(feld_id, Grund)`. Ein Eintrag braucht einen
/// Grund, der sagt, woher der Wert stattdessen kommt. Gemessen am Stand `49f86741` (M): von den 52
/// Transform-Quellen sind genau diese drei nicht fragbar, alle drei Ring-Werte (`hilfe_kurz`
/// „Berechnet", `askable: false`, `elster_kz: null` mit Grund in der Bindung). Jede weitere nicht
/// fragbare Quelle ist ein Verstoss, bis jemand sie hier mit Grund nennt.
const NICHT_FRAGBAR_ERLAUBT: &[(&str, &str)] = &[
    (
        "gewst_zu_zahlen_partner",
        "Ring-Wert: gewerbesteuer() in bescheid/src/deklaration/ring_werte.rs rechnet Messbetrag mal Hebesatz des Partnerbetriebs",
    ),
    (
        "p34_abs3_antragsbetrag",
        "Ring-Wert: p34_antrag() in ring_werte.rs schreibt den Veraeusserungsgewinn, wenn der Chooser Abs. 3 rechnet",
    ),
    (
        "p35c_massnahme_einzelbetrag",
        "Ring-Wert: einzelzeilen() in ring_werte.rs setzt p35c_sanierungsaufwendungen in die Zeile der Massnahmenart",
    ),
];

/// Aus `quellen` die, die in `askable` als nicht fragbar stehen und nicht namentlich erlaubt sind
/// (Verstoesse), und die Ausnahmen, die keinen Zweck mehr haben, weil die Quelle fragbar ist oder
/// keine Quelle mehr (veraltet). `askable`: `feld_id` → fragbar; ein Feld, das fehlt, prueft
/// `transform_konfig_ist_konsistent`.
fn nicht_fragbare_quellen<'a>(
    quellen: &BTreeSet<&'a str>,
    askable: &BTreeMap<&str, bool>,
    erlaubt: &[(&'a str, &str)],
) -> (Vec<&'a str>, Vec<&'a str>) {
    let nicht = |f: &str| askable.get(f) == Some(&false);
    let verstoesse = quellen
        .iter()
        .copied()
        .filter(|f| nicht(f) && !erlaubt.iter().any(|(e, _)| e == f))
        .collect();
    let veraltet = erlaubt
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| !quellen.contains(e) || !nicht(e))
        .collect();
    (verstoesse, veraltet)
}

/// Aussage 6: Jede Transform-Quelle wird gefragt. Eine Quelle, die nicht fragbar ist, liefert der
/// Tabelle nie einen Wert: das Kz bleibt leer, ohne dass ein Test es merkt. Die Mutanten
/// `askable: true -> false` bei `fam_alleinstehend` und `stammdaten_iban` blieben in elster, bescheid
/// und bindung gruen. `bescheid::scheiben_tabellen_konsistenz::jedes_kegel_feld_ist_fragbar` prueft
/// nur die vier `*_KEGEL`-Listen; `fam_anzahl_kinder`, `rentner_jahresrente` und `rentner_renten_art`
/// stehen dort (zufaellig), die beiden anderen nur in den `*_FELDER`-Listen.
#[test]
fn jede_transform_quelle_ist_fragbar() {
    let askable: BTreeMap<&str, bool> = bindungen()
        .iter()
        .map(|b| (b.feld_id.as_str(), b.askable))
        .collect();
    for (feld, grund) in NICHT_FRAGBAR_ERLAUBT {
        assert!(grund.len() >= 20, "{feld}: Ausnahme ohne tragenden Grund");
    }
    let (verstoesse, veraltet) =
        nicht_fragbare_quellen(&transform_quellen(), &askable, NICHT_FRAGBAR_ERLAUBT);
    assert!(
        verstoesse.is_empty(),
        "Transform-Quellen, die nicht askable sind (die Tabelle bekommt nie einen Wert): {verstoesse:?}"
    );
    assert!(
        veraltet.is_empty(),
        "Ausnahmen ohne Zweck (Quelle ist fragbar oder keine Quelle mehr): {veraltet:?}"
    );
}

/// Der Pruefer selbst, an erfundenen Eingaben: er meldet eine nicht fragbare Quelle, laesst eine
/// namentlich erlaubte durch und meldet eine Ausnahme, deren Quelle fragbar ist oder fehlt.
#[test]
fn der_quellen_pruefer_findet_verstoss_ausnahme_und_veraltete_ausnahme() {
    let quellen: BTreeSet<&str> = ["a", "b", "c"].into_iter().collect();
    let askable: BTreeMap<&str, bool> =
        [("a", true), ("b", false), ("c", false), ("x", true)].into_iter().collect();
    let (v, alt) = nicht_fragbare_quellen(&quellen, &askable, &[]);
    assert_eq!((v, alt), (vec!["b", "c"], vec![]));
    let (v, alt) = nicht_fragbare_quellen(&quellen, &askable, &[("b", "berechnet")]);
    assert_eq!((v, alt), (vec!["c"], vec![]));
    // "a" ist fragbar, "x" keine Quelle: beide Ausnahmen sind ohne Zweck.
    let (v, alt) = nicht_fragbare_quellen(&quellen, &askable, &[("a", "g"), ("x", "g")]);
    assert_eq!((v, alt), (vec!["b", "c"], vec!["a", "x"]));
}

// ---------------------------------------------------------------------------------------------
// Aussagen 7 bis 9: Lesestellen im Quelltext (Weg B voll, Stufe 3, Auftrag 4)
// ---------------------------------------------------------------------------------------------
//
// Rust-Code nennt Felder mit ihrem Namen: `wert(f, "veranlagung")`. Verliert das Feld seine Frage
// (`askable: false`), liefert die Lesestelle still null; schreibt jemand den Namen falsch, liest sie
// ein Feld, das es nicht gibt, mit demselben Ergebnis. Die Listen in `scheiben_tabellen.rs` und
// `konstanten.rs` nennen alle 362 gelesenen Felder und taugen deshalb nicht als Beleg fuer eine
// Lesestelle; ihr Waechter ist `bescheid/tests/scheiben_tabellen_konsistenz.rs`. Die Messung steht
// im Bericht `w-kz-auftrag4-messung.md` (M, Stand `45dd25db`).
//
// ponytail: nur exakte Literale. Ein Name, den der Code zur Laufzeit baut (`format!`), sieht der
// Scan nicht: heute `mit_anspruch_auf_zuschuss_partner`, dazu die Felder, die `instanzen("kind")`
// ueber die Gruppe liest. Upgrade: eine Fixture, die jede Lesestelle aufzeichnet (wie das Regal der
// Kz-Wache).

/// Ein Bezeichner-Literal: `[a-z][a-z0-9_]*`, hoechstens 80 Zeichen. Alles andere (Texte, Kz,
/// Gross-Schreibung) ist nie eine `feld_id`.
fn ist_bezeichner(s: &str) -> bool {
    s.len() <= 80
        && s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Ein Bezeichner-Literal im Produktionstext.
struct Literal {
    /// Pfad relativ zu `rust/`, mit `/`.
    rel: String,
    zeile: usize,
    text: String,
}

struct Quelltext {
    /// Alle gescannten Dateien (ohne Testdateien).
    dateien: Vec<String>,
    literale: Vec<Literal>,
}

/// Jedes Bezeichner-Literal im Produktionstext von `rust/*/src` ausser `rust/bindung`:
/// ohne `#[cfg(test)]`-Items, ohne Kommentare, ohne Dateien, die ein `#[cfg(test)] mod x;` meint.
fn quelltext() -> &'static Quelltext {
    static CELL: OnceLock<Quelltext> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut crates: Vec<_> = fs::read_dir(&wurzel)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.join("src").is_dir() && p.file_name().is_some_and(|n| n != "bindung"))
            .collect();
        crates.sort();
        let mut pfade = Vec::new();
        for c in &crates {
            rs_dateien(&wurzel, &c.join("src"), &mut pfade);
        }
        let abgetastet: Vec<_> = pfade
            .into_iter()
            .map(|rel| {
                let text = fs::read_to_string(wurzel.join(&rel)).unwrap();
                let abt = abtasten_mit(&text, ist_bezeichner);
                (rel, text, abt)
            })
            .collect();
        let test_dateien: Vec<String> = abgetastet
            .iter()
            .flat_map(|(rel, _, abt)| abt.test_module.iter().flat_map(|n| modul_dateien(rel, n)))
            .collect();
        let mut q = Quelltext {
            dateien: Vec::new(),
            literale: Vec::new(),
        };
        for (rel, text, abt) in abgetastet {
            if test_dateien.contains(&rel) {
                continue;
            }
            for f in &abt.funde {
                q.literale.push(Literal {
                    rel: rel.clone(),
                    zeile: text[..f.von].matches('\n').count() + 1,
                    text: f.kz.clone(),
                });
            }
            q.dateien.push(rel);
        }
        q
    })
}

/// Dateien, die Felder nur aufzaehlen (Scheibenlisten, Tabellen aus `api_constants.py`). Ein Feld
/// dort zu nennen heisst nicht, es zu lesen.
const LISTEN_DATEIEN: &[&str] = &[
    "bescheid/src/deklaration/scheiben_tabellen.rs",
    "bescheid/src/deklaration/konstanten.rs",
];

/// Der Ring schreibt hier die berechneten Felder (`askable: false`).
const RING_DATEI: &str = "bescheid/src/deklaration/ring_werte.rs";

/// Die `feld_id`, die im Produktionstext ausserhalb der Listen als Literal stehen.
fn gelesene_felder<'a>(literale: &'a [Literal], ids: &BTreeSet<&str>) -> BTreeSet<&'a str> {
    literale
        .iter()
        .filter(|l| !LISTEN_DATEIEN.contains(&l.rel.as_str()) && ids.contains(l.text.as_str()))
        .map(|l| l.text.as_str())
        .collect()
}

/// Nicht fragbare Felder, die der Code liest, und was sie rechtfertigt. Heute keine Ausnahme: jedes
/// der 19 gelesenen nicht fragbaren Felder steht auch in `ring_werte.rs`.
const LESEN_NICHT_FRAGBAR_ERLAUBT: &[(&str, &str)] = &[];

/// Wie der Code ein nicht fragbares Feld nennt.
#[derive(Debug, PartialEq)]
enum Lese {
    /// Kein Literal im Produktionstext.
    KeinLiteral,
    /// Nur in den Listen.
    NurListen,
    /// Der Ring nennt es: er schreibt es.
    Ring,
    /// Gelesen, nicht vom Ring, aber namentlich erlaubt.
    Erlaubt,
    /// Gelesen, nicht fragbar, nicht vom Ring, ohne Ausnahme: `feld (Datei:Zeile)`.
    Verstoss(String),
}

/// Jedes nicht fragbare Feld aus `askable` genau einmal einsortiert.
fn lese_klassen<'a>(
    literale: &[Literal],
    askable: &BTreeMap<&'a str, bool>,
    erlaubt: &[(&str, &str)],
) -> Vec<(&'a str, Lese)> {
    askable
        .iter()
        .filter(|(_, a)| !**a)
        .map(|(f, _)| {
            let alle: Vec<&Literal> = literale.iter().filter(|l| l.text == *f).collect();
            let gelesen: Vec<&&Literal> = alle
                .iter()
                .filter(|l| !LISTEN_DATEIEN.contains(&l.rel.as_str()))
                .collect();
            let klasse = if alle.is_empty() {
                Lese::KeinLiteral
            } else if gelesen.is_empty() {
                Lese::NurListen
            } else if gelesen.iter().any(|l| l.rel == RING_DATEI) {
                Lese::Ring
            } else if erlaubt.iter().any(|(e, _)| e == f) {
                Lese::Erlaubt
            } else {
                Lese::Verstoss(format!("{f} ({}:{})", gelesen[0].rel, gelesen[0].zeile))
            };
            (*f, klasse)
        })
        .collect()
}

/// Ausnahmen, die keinen Zweck mehr haben: ihr Feld steht nicht (mehr) als `Lese::Erlaubt` da.
fn veraltete_ausnahmen<'a>(
    erlaubt: &[(&'a str, &str)],
    klassen: &[(&str, Lese)],
) -> Vec<&'a str> {
    erlaubt
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| !klassen.iter().any(|(f, k)| f == e && *k == Lese::Erlaubt))
        .collect()
}

fn alle_gruende_tragen(listen: &[&[(&str, &str)]]) {
    for (feld, grund) in listen.iter().flat_map(|l| l.iter()) {
        assert!(grund.len() >= 20, "{feld}: Ausnahme ohne tragenden Grund");
    }
}

/// Aussage 7: Ein Feld, das der Code ausserhalb der Listen beim Namen nennt, ist fragbar, oder der
/// Ring schreibt es, oder es steht mit Grund in `LESEN_NICHT_FRAGBAR_ERLAUBT`. Sonst bekommt die
/// Lesestelle nie einen Wert, und kein Test merkt es. Das schuetzt die Felder, die weder in einer
/// Kegel-Liste noch in einer Transform-Tabelle noch in `kz_zuordnung.tsv` stehen (Stand der
/// Messung: 74, z. B. `hh_in_eu_ewr`, `versorgung_art`, `am_gwg_sofortabzug_gewaehlt`). Felder mit
/// `frage_invertiert` schuetzt schon die Validierung der Registry (`InvertiertOhneBoolAskable`).
#[test]
fn jedes_gelesene_nicht_fragbare_feld_schreibt_der_ring() {
    alle_gruende_tragen(&[LESEN_NICHT_FRAGBAR_ERLAUBT]);
    let q = quelltext();
    // Reichweite: die Listen und die Ring-Datei sind im Scan, sonst waere die Aussage leer wahr.
    for d in LISTEN_DATEIEN.iter().chain([&RING_DATEI]) {
        assert!(q.dateien.iter().any(|f| f == d), "{d} fehlt im Scan");
    }
    let askable: BTreeMap<&str, bool> = bindungen()
        .iter()
        .map(|b| (b.feld_id.as_str(), b.askable))
        .collect();
    let ids: BTreeSet<&str> = askable.keys().copied().collect();
    let gelesen = gelesene_felder(&q.literale, &ids);
    assert!(gelesen.len() >= 200, "der Scan liest nur {} Felder", gelesen.len());
    // Der Scan sieht jede Transform-Quelle: die Tabellen sind die eine Wahrheit, die der Kompiler kennt.
    let blind: Vec<&str> = transform_quellen()
        .into_iter()
        .filter(|f| !q.literale.iter().any(|l| l.text == *f))
        .collect();
    assert!(blind.is_empty(), "der Scan findet Transform-Quellen nicht: {blind:?}");

    let klassen = lese_klassen(&q.literale, &askable, LESEN_NICHT_FRAGBAR_ERLAUBT);
    let nicht_fragbar = bindungen().iter().filter(|b| !b.askable).count();
    assert_eq!(
        klassen.len(),
        nicht_fragbar,
        "jedes nicht fragbare Feld der Registry wird genau einmal einsortiert"
    );
    let ring = klassen.iter().filter(|(_, k)| *k == Lese::Ring).count();
    assert!(ring >= 15, "nur {ring} Ring-Felder gefunden");
    let verstoesse: Vec<&str> = klassen
        .iter()
        .filter_map(|(_, k)| match k {
            Lese::Verstoss(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        verstoesse.is_empty(),
        "Felder, die der Code liest, die nicht askable sind und die der Ring nicht schreibt: {verstoesse:?}"
    );
    let veraltet = veraltete_ausnahmen(LESEN_NICHT_FRAGBAR_ERLAUBT, &klassen);
    assert!(veraltet.is_empty(), "Ausnahmen ohne Zweck: {veraltet:?}");
}

/// Der Einsortierer an erfundenen Eingaben: jede Klasse, jede Ausnahme, jede veraltete Ausnahme.
#[test]
fn der_lese_pruefer_kennt_jede_klasse_und_veraltete_ausnahmen() {
    let lit = |rel: &str, text: &str| Literal {
        rel: rel.to_owned(),
        zeile: 7,
        text: text.to_owned(),
    };
    let literale = [
        lit(LISTEN_DATEIEN[0], "nur_liste"),
        lit("a/src/x.rs", "vom_ring"),
        lit(RING_DATEI, "vom_ring"),
        lit("a/src/x.rs", "erlaubt"),
        lit("a/src/x.rs", "verstoss"),
        lit(LISTEN_DATEIEN[1], "verstoss_nur_in_listen"),
    ];
    let askable: BTreeMap<&str, bool> = [
        ("kein_literal", false),
        ("nur_liste", false),
        ("vom_ring", false),
        ("erlaubt", false),
        ("verstoss", false),
        ("fragbar", true),
    ]
    .into_iter()
    .collect();
    let erlaubt = [("erlaubt", "grund"), ("fragbar", "grund"), ("vom_ring", "grund")];
    let klassen = lese_klassen(&literale, &askable, &erlaubt);
    let von = |f: &str| &klassen.iter().find(|(g, _)| *g == f).unwrap().1;
    assert_eq!(klassen.len(), 5, "nur nicht fragbare Felder");
    assert_eq!(*von("kein_literal"), Lese::KeinLiteral);
    assert_eq!(*von("nur_liste"), Lese::NurListen);
    assert_eq!(*von("vom_ring"), Lese::Ring);
    assert_eq!(*von("erlaubt"), Lese::Erlaubt);
    assert_eq!(*von("verstoss"), Lese::Verstoss("verstoss (a/src/x.rs:7)".to_owned()));
    // "fragbar" steht nicht als Erlaubt da, "vom_ring" braucht keine Ausnahme: beide sind veraltet.
    assert_eq!(
        veraltete_ausnahmen(&erlaubt, &klassen),
        vec!["fragbar", "vom_ring"]
    );
    let nur_liste = lese_klassen(&literale, &askable, &[]);
    assert_eq!(nur_liste.len(), 5);
}

/// `feld_id`-nahe Literale, die keine `feld_id` sind: `(Literal, Grund)`.
const NAHE_ERLAUBT: &[(&str, &str)] = &[
    (
        "ep_werbungskosten",
        "Familienname im Ring-Tupel SCHEIBEN_N_VOR_GWG_TEIL_RINGE (scheiben_tabellen.rs), kein Feld; vv_werbungskosten ist ein anderes Feld",
    ),
    (
        "hh_dienstleistung",
        "Gruppenname fuer hh_summe und hh_positiv (abzuege.rs, sperre/gesamt.rs), kein Feld; hh_dienstleistungen ist ein anderes Feld",
    ),
    (
        "schulgeld__",
        "Praefix fuer strip_prefix in konsistenz/src/preflight.rs (schulgeld__1, schulgeld__2), kein Feld; schulgeld ist ein anderes Feld",
    ),
];

/// Kuerzer als das sind Namen zu leicht dicht beieinander (`kind`, `kist`).
const MIN_NAEHE: usize = 8;

/// Hoechstens `max` Einfuegungen, Loeschungen oder Ersetzungen von `a` nach `b` (Levenshtein).
fn abstand_hoechstens(a: &str, b: &str, max: usize) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len().abs_diff(b.len()) > max {
        return false;
    }
    let mut zeile: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut vorher = zeile[0];
        zeile[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let tausch = vorher + usize::from(ca != cb);
            vorher = zeile[j + 1];
            zeile[j + 1] = tausch.min(zeile[j] + 1).min(vorher + 1);
        }
    }
    zeile[b.len()] <= max
}

/// Literale, die keine `feld_id` sind und von einer hoechstens zwei Zeichen entfernt liegen
/// (Tippfehler), ohne die namentlich erlaubten; dazu die Ausnahmen ohne Zweck.
fn tippfehler_naehe<'a>(
    literale: &[Literal],
    ids: &BTreeSet<&str>,
    erlaubt: &[(&'a str, &str)],
) -> (Vec<String>, Vec<&'a str>) {
    let mut gefunden: BTreeMap<&str, String> = BTreeMap::new();
    for l in literale {
        let t = l.text.as_str();
        if t.len() < MIN_NAEHE || ids.contains(t) || gefunden.contains_key(t) {
            continue;
        }
        if let Some(k) = ids
            .iter()
            .find(|k| k.len() >= MIN_NAEHE && abstand_hoechstens(t, k, 2))
        {
            gefunden.insert(t, format!("\"{t}\" ({}:{}) liegt dicht an `{k}`", l.rel, l.zeile));
        }
    }
    let verstoesse = gefunden
        .iter()
        .filter(|(t, _)| !erlaubt.iter().any(|(e, _)| e == *t))
        .map(|(_, s)| s.clone())
        .collect();
    let veraltet = erlaubt
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| !gefunden.contains_key(e))
        .collect();
    (verstoesse, veraltet)
}

/// Aussage 8: Ein Literal, das fast eine `feld_id` ist, ist ein Tippfehler. Die Lesestelle liest
/// dann ein Feld, das es nicht gibt, und liefert still null. Gemessen (M): 407 verschiedene
/// Bezeichner-Literale von mindestens 8 Zeichen, die keine `feld_id` sind; drei liegen in Abstand 2,
/// alle drei sind Namen anderer Dinge (siehe `NAHE_ERLAUBT`). Ein Tausch gegen eine andere `feld_id`
/// bleibt unsichtbar. Das Gate `bescheid/tests/feld_kennung_gate.rs` prueft schon die drei Leser
/// `wert`, `feld_int_oder_null`, `feld_euro_oder_null` in `rust/bescheid/src`; diese Aussage kennt
/// keinen Leser-Namen und sieht jede Crate.
#[test]
fn kein_literal_ist_ein_tippfehler_einer_feld_id() {
    alle_gruende_tragen(&[NAHE_ERLAUBT]);
    let ids: BTreeSet<&str> = bindungen().iter().map(|b| b.feld_id.as_str()).collect();
    let literale = &quelltext().literale;
    let fremd = literale
        .iter()
        .filter(|l| l.text.len() >= MIN_NAEHE && !ids.contains(l.text.as_str()))
        .map(|l| l.text.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    assert!(fremd >= 300, "der Scan sieht nur {fremd} Nicht-Feld-Literale");
    let (verstoesse, veraltet) = tippfehler_naehe(literale, &ids, NAHE_ERLAUBT);
    assert!(
        verstoesse.is_empty(),
        "Literale dicht an einer feld_id (Tippfehler?): {verstoesse:#?}"
    );
    assert!(veraltet.is_empty(), "Ausnahmen ohne Zweck: {veraltet:?}");
}

/// Der Naehe-Pruefer an erfundenen Eingaben: findet einen Tippfehler, laesst eine Ausnahme durch,
/// ignoriert Kurzes und Fernes, meldet eine veraltete Ausnahme.
#[test]
fn der_naehe_pruefer_findet_tippfehler_und_veraltete_ausnahmen() {
    assert!(abstand_hoechstens("veranlagung", "veranlagung", 2));
    assert!(abstand_hoechstens("veranlagng", "veranlagung", 2));
    assert!(abstand_hoechstens("veranlagungx", "veranlagung", 1));
    assert!(abstand_hoechstens("kitten", "sitting", 3));
    assert!(!abstand_hoechstens("kitten", "sitting", 2));
    assert!(!abstand_hoechstens("abc", "abcdef", 2));
    let lit = |text: &str| Literal {
        rel: "a/src/x.rs".to_owned(),
        zeile: 3,
        text: text.to_owned(),
    };
    let ids: BTreeSet<&str> = ["veranlagung", "kind_vorname", "kist"].into_iter().collect();
    let literale = [
        lit("veranlagung"),   // ist eine feld_id
        lit("veranlagng"),    // Tippfehler
        lit("kind_vornam"),   // Tippfehler
        lit("kistx"),         // zu kurz
        lit("ganz_woanders"), // fern
    ];
    let (v, alt) = tippfehler_naehe(&literale, &ids, &[]);
    assert_eq!(v.len(), 2, "{v:?}");
    assert!(v[0].contains("kind_vornam") && v[1].contains("veranlagng"), "{v:?}");
    assert!(alt.is_empty());
    let (v, alt) = tippfehler_naehe(
        &literale,
        &ids,
        &[("veranlagng", "g"), ("nicht_da", "g")],
    );
    assert_eq!(v.len(), 1);
    assert_eq!(alt, vec!["nicht_da"]);
}

/// Fragbare Felder, die kein Rust-Literal und keinen Kanal in der Registry haben. Heute eines.
const OHNE_LESER_ERLAUBT: &[(&str, &str)] = &[(
    "dhf_keine_pflicht_dienstwohnung",
    "gate: false seit 2026-08-20 (bindung_n_vor_gwg.yaml): die Frage ist ein Hinweis, Rust liest das Feld nirgends; ob sie bleibt, ist offen",
)];

/// Hat die Registry selbst einen Leser fuer das Feld: das Kz (`deklariere`), den Slot (`intervall`),
/// das Screening-Flag (Scheibe `gesamt`), das Zaehlfeld einer Instanzgruppe (`api/src/chat.rs`) oder
/// die Bedingung eines anderen Felds (`feld_bedingung.feld`).
fn registry_kanal(
    b: &Bindung,
    anzahl_felder: &BTreeSet<&str>,
    bedingungsziele: &BTreeSet<&str>,
) -> bool {
    b.elster_kz.is_some()
        || matches!(b.quelle.bindungspunkt, Bindungspunkt::SignaturSlot(_))
        || b.screening == Some(true)
        || anzahl_felder.contains(b.feld_id.as_str())
        || bedingungsziele.contains(b.feld_id.as_str())
}

/// Aus `fragbar` (`feld`, hat einen Kanal in der Registry) die Felder ohne jeden Leser und ohne
/// Ausnahme, dazu die Ausnahmen ohne Zweck (das Feld hat einen Leser oder ist nicht fragbar).
fn felder_ohne_leser<'a>(
    fragbar: &[(&'a str, bool)],
    gelesen: &BTreeSet<&str>,
    erlaubt: &[(&'a str, &str)],
) -> (Vec<&'a str>, Vec<&'a str>) {
    let ohne = |f: &str| {
        fragbar
            .iter()
            .any(|(g, kanal)| *g == f && !kanal && !gelesen.contains(f))
    };
    let verstoesse = fragbar
        .iter()
        .map(|(f, _)| *f)
        .filter(|f| ohne(f) && !erlaubt.iter().any(|(e, _)| e == f))
        .collect();
    let veraltet = erlaubt
        .iter()
        .map(|(e, _)| *e)
        .filter(|e| !ohne(e))
        .collect();
    (verstoesse, veraltet)
}

/// Aussage 9: Jedes fragbare Feld hat einen Leser: ein Literal im Produktionstext ausserhalb der
/// Listen oder einen Kanal in der Registry. Sonst fragt die App etwas, das nichts liest (tote
/// Bindung). `scheiben_tabellen_konsistenz` prueft nur, dass das Feld in einer Scheibe steht.
#[test]
fn jedes_fragbare_feld_hat_einen_leser() {
    alle_gruende_tragen(&[OHNE_LESER_ERLAUBT]);
    let q = quelltext();
    let ids: BTreeSet<&str> = bindungen().iter().map(|b| b.feld_id.as_str()).collect();
    let gelesen = gelesene_felder(&q.literale, &ids);
    let anzahl_felder: BTreeSet<&str> = registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.instanz_gruppen.iter().map(|g| g.anzahl_feld.as_str()))
        .collect();
    let bedingungsziele: BTreeSet<&str> = bindungen()
        .iter()
        .filter_map(|b| b.feld_bedingung.as_ref())
        .map(|c| c.feld.as_str())
        .collect();
    assert!(
        anzahl_felder.len() >= 5 && bedingungsziele.len() >= 5,
        "Kanaele leer: {} Zaehlfelder, {} Bedingungsziele",
        anzahl_felder.len(),
        bedingungsziele.len()
    );
    let fragbar: Vec<(&str, bool)> = bindungen()
        .iter()
        .filter(|b| b.askable)
        .map(|b| {
            (
                b.feld_id.as_str(),
                registry_kanal(b, &anzahl_felder, &bedingungsziele),
            )
        })
        .collect();
    assert_eq!(
        fragbar.len(),
        bindungen().iter().filter(|b| b.askable).count(),
        "jedes fragbare Feld der Registry wird geprueft"
    );
    let (verstoesse, veraltet) = felder_ohne_leser(&fragbar, &gelesen, OHNE_LESER_ERLAUBT);
    assert!(
        verstoesse.is_empty(),
        "fragbare Felder ohne Leser (tote Frage?): {verstoesse:?}"
    );
    assert!(veraltet.is_empty(), "Ausnahmen ohne Zweck: {veraltet:?}");
}

/// Der Leser-Pruefer an erfundenen Eingaben.
#[test]
fn der_leser_pruefer_findet_feld_ohne_leser_ausnahme_und_veraltete_ausnahme() {
    let fragbar = [("a", true), ("b", false), ("c", false), ("d", false)];
    let gelesen: BTreeSet<&str> = ["c"].into_iter().collect();
    let (v, alt) = felder_ohne_leser(&fragbar, &gelesen, &[]);
    assert_eq!((v, alt), (vec!["b", "d"], vec![]));
    let (v, alt) = felder_ohne_leser(&fragbar, &gelesen, &[("b", "g")]);
    assert_eq!((v, alt), (vec!["d"], vec![]));
    // "a" hat einen Kanal, "c" ein Literal, "x" ist kein fragbares Feld: alle drei Ausnahmen veraltet.
    let (v, alt) = felder_ohne_leser(&fragbar, &gelesen, &[("a", "g"), ("c", "g"), ("x", "g")]);
    assert_eq!((v, alt), (vec!["b", "d"], vec!["a", "c", "x"]));
}
