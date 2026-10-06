//! Slot-Namen-Waechter (Abnahme-Auftrag Teil B, 2026-10-06): jeder Slot, den der Rechenkern beim Namen liest,
//! muss in der Bindung der Scheibe, die ihn liest, einen Traeger haben.
//!
//! **Worum es geht.** Die Rechnung bekommt ihre Eingaben unter Namen ("Slots"): die Bindung (`rust/bindung/daten`)
//! sagt je Feld, unter welchem Namen es in die Rechnung geht (`signatur_slot`), der Rechenkern
//! (`rust/bescheid/src/zweige/*.rs`) liest diese Namen als Text. Stimmen die zwei Schreibweisen nicht
//! ueberein, merkt der Compiler nichts.
//!
//! **Warum es zaehlt.** Ein falsch geschriebener Slot-Name bricht heute erst zur Laufzeit: `/ergebnis`, `/fragen`
//! und `/stand` der betroffenen Scheibe antworten mit 500 (`SlotFehlt`, Meldung `KeyError: Slot X fehlt`), und zwar fuer
//! jeden Fall dieser Scheibe, nicht nur fuer einen. Python hatte dieselbe Luecke still (`slots.get(name, 0)`
//! lieferte 0 und rechnete weiter: 1.356.800 ct Steuer wurden zu 0 ct, `grund` blieb "bestaetigt"); das Rust-Gegenstueck
//! `slot()` ist fail-closed und damit sichtbar, aber spaet. Das Python-Gate `tests/test_slot_fn_reader_existiert.py`
//! steht in `rust/TESTMAP.tsv` als `NOT_PORTED` mit der Begruendung "Slots sind Struct-Felder, eine Umbenennung ist
//! ein Compilerfehler": das gilt in Rust NICHT, `slot(slots, "arbeitstage")` ist ein Text-Schluessel.
//!
//! **Wo es sitzt.** Der Test liest den Quelltext von `rust/bescheid/src/**` und findet jedes Literal, das an
//! `slot(..)`, `slots.get(..)` oder `SlotFehlt(..)` geht. Zwei Dinge werden geprueft:
//!
//! 1. Die Lesestellen im Quelltext sind GENAU die der Tabelle [`LESESTELLEN`] (Datei, Slot, Quantitaeten). Eine neue
//!    Lesestelle ohne Zeile ist rot, eine Zeile ohne Lesestelle auch. Die Tabelle ist die Wahrheit, der Scan haelt
//!    sie ehrlich (Baumuster wie `REGELN_OHNE_GROUND_TRUTH` in Python).
//! 2. Jede Quantitaet liest ihre Slots aus der Bindung GENAU ihrer Scheibe: `Cfg::felder` und `Cfg::kegel` (die
//!    Pflicht-Achsen der Spannenrechnung) der Scheibe, die die Quantitaet rechnet, haben je Slot einen Traeger. Ein
//!    "kommt irgendwo im Repo vor" genuegt nicht: `jahresrente` steht in zwei Dateien, ein Name kann in der falschen
//!    Scheibe noch existieren (die erste Fassung des Python-Gates war genau dafuer blind).
//!
//! Was er NICHT sieht (gelesen, nicht geprueft): ein Name, der zur Laufzeit gebaut wird (`format!("{k}_{i}")`, eine
//! Variable, eine Konstante); ein Slot, den ein anderer Crate als `bescheid` liest; ein `Slots`-Parameter, der nicht
//! `slots` heisst (das prueft [`jeder_slots_parameter_heisst_slots`], sonst wuerde der Scan ihn uebersehen); ob der
//! Traeger den Slot mit dem richtigen WERT fuellt (nur der Name zaehlt); die Zuordnung "Lesestelle -> Quantitaeten"
//! in der Tabelle selbst (von Hand, vom Review getragen).
//!
//! Die Mindestzahlen verhindern, dass ein Test gruen wird, weil seine Menge leer ist (Datei verschoben, Regex
//! veraltet).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use bescheid::deklaration::Cfg;
use bindung::{Bindungspunkt, Registry};
use domain::Scheibe;
use regex::Regex;

const AB: &str = "abziehbarer_betrag";
const AN: &str = "festzusetzende_est";
const GE: &str = "festzusetzende_est_gesamt";
const RE: &str = "festzusetzende_est_rentner";

/// Die Quantitaeten, die `zweige::baue` rechnet, und die Scheibe, die sie als `gesamt_ring` traegt.
const QUANTITAETEN: &[(&str, Scheibe)] = &[
    (AB, Scheibe::Ep),
    (AN, Scheibe::AnGesamt),
    (GE, Scheibe::Gesamt),
    (RE, Scheibe::RentnerGesamt),
];

/// Jede Stelle im Quelltext von `rust/bescheid/src`, die einen Slot beim Namen liest: (Datei unter `src/`, Slot,
/// Quantitaeten, die ueber diese Stelle laufen). `oepnv_kosten_jahr` liest `abzuege.rs::oepnv_eur`, die
/// `abziehbarer_betrag` und `ep_eingabe` (`wk.rs`) gemeinsam rufen; `ep_eingabe` ruft `festzusetzende_est` und
/// `festzusetzende_est_gesamt`. `festzusetzende_est_rentner` liest keinen Slot (`_slots` in `rentner.rs`).
const LESESTELLEN: &[(&str, &str, &[&str])] = &[
    ("zweige.rs", "arbeitstage", &[AB]),
    ("zweige.rs", "entfernung_km_roh", &[AB]),
    ("zweige.rs", "eigenes_oder_ueberlassenes_kfz", &[AB]),
    ("abzuege.rs", "oepnv_kosten_jahr", &[AB, AN, GE]),
    ("zweige/wk.rs", "arbeitstage", &[AN, GE]),
    ("zweige/wk.rs", "entfernung_km_roh", &[AN, GE]),
    ("zweige/wk.rs", "oepnv_kosten_jahr", &[AN, GE]),
    ("zweige/wk.rs", "eigenes_oder_ueberlassenes_kfz", &[AN, GE]),
    ("zweige/an_gesamt.rs", "bruttoarbeitslohn", &[AN]),
    ("zweige/an_gesamt.rs", "veranlagung", &[AN]),
    ("zweige/an_gesamt.rs", "entfernung_km_roh", &[AN]),
    ("zweige/gesamt.rs", "bruttoarbeitslohn", &[GE]),
    ("zweige/gesamt.rs", "veranlagung", &[GE]),
];

/// Mindestzahlen (gemessen am Stand `67617b83`: 13 Lesestellen; 4 + 6 + 6 + 0 Slots, je in zwei Listen = 32
/// Pruefungen, plus 4 im Teil-Ring von `n_vor_gwg` = 36; 9 Parameter vom Typ `&Slots`). Ein
/// Lauf darunter hat seine Menge verloren und prueft nichts mehr: rot, nicht gruen.
const MIN_LESESTELLEN: usize = 13;
const MIN_PRUEFUNGEN: usize = 36;
const MIN_SLOTS_PARAMETER: usize = 9;

// ---- Der Scan ---------------------------------------------------------------------------------------------------

fn src_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn rs_dateien(dir: &Path, aus: &mut Vec<PathBuf>) {
    for eintrag in std::fs::read_dir(dir).unwrap() {
        let pfad = eintrag.unwrap().path();
        if pfad.is_dir() {
            rs_dateien(&pfad, aus);
        } else if pfad.extension().is_some_and(|e| e == "rs") {
            aus.push(pfad);
        }
    }
}

/// Der Text ohne Kommentarzeilen (`//`, `///`, `//!`): ein Beispiel im Doc-Kommentar liest nichts.
fn code_ohne_kommentarzeilen(text: &str) -> String {
    text.lines()
        .filter(|z| !z.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Die drei Formen, in denen der Rechenkern einen Slot beim Namen nennt.
fn leser_muster() -> [Regex; 3] {
    [
        // `slot(slots, "arbeitstage")`: der Helfer aus `zweige.rs`; das erste Argument ist ein Pfad ohne Komma/Klammer.
        Regex::new(r#"\bslot\(\s*[^,()]+,\s*"([^"\\]*)"\s*\)"#).unwrap(),
        // `slots.get("oepnv_kosten_jahr")`, auch ueber Zeilenumbruch (`abzuege.rs`).
        Regex::new(r#"\bslots\s*\.get\(\s*"([^"\\]*)""#).unwrap(),
        // `BescheidFehler::SlotFehlt("oepnv_kosten_jahr".into())`: der Name steht im Fehler.
        Regex::new(r#"\bSlotFehlt\(\s*"([^"\\]*)""#).unwrap(),
    ]
}

/// Slot-Literale im Text einer Datei.
fn slot_literale(text: &str) -> BTreeSet<String> {
    let code = code_ohne_kommentarzeilen(text);
    leser_muster()
        .iter()
        .flat_map(|m| {
            m.captures_iter(&code)
                .map(|c| c[1].to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

/// `(Datei unter src/, Slot)` fuer jede Datei von `rust/bescheid/src`.
fn gefundene_lesestellen() -> BTreeSet<(String, String)> {
    let mut dateien = Vec::new();
    rs_dateien(&src_wurzel(), &mut dateien);
    let mut aus = BTreeSet::new();
    for d in dateien {
        let rel = d
            .strip_prefix(src_wurzel())
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let text = std::fs::read_to_string(&d).unwrap();
        for s in slot_literale(&text) {
            aus.insert((rel.clone(), s));
        }
    }
    aus
}

fn tabellen_lesestellen() -> BTreeSet<(String, String)> {
    LESESTELLEN
        .iter()
        .map(|(d, s, _)| ((*d).to_owned(), (*s).to_owned()))
        .collect()
}

// ---- Die Bindung ------------------------------------------------------------------------------------------------

fn registry() -> &'static Registry {
    static CELL: OnceLock<Registry> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel).expect("Registry laedt")
    })
}

/// `feld_id -> signatur_slot` ueber die ganze Registry (Felder ohne Slot, mit `geltungsbedingung`, fehlen).
fn slot_je_feld() -> BTreeMap<&'static str, &'static str> {
    registry()
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .filter_map(|b| match &b.quelle.bindungspunkt {
            Bindungspunkt::SignaturSlot(s) => Some((b.feld_id.as_str(), s.as_str())),
            Bindungspunkt::Geltungsbedingung(_) => None,
        })
        .collect()
}

/// Die `feld_id`s einer Bindungsdatei in Dateireihenfolge (`api::zustand::Bindungen::je_datei`).
fn felder_der_datei(name: &str) -> Vec<String> {
    registry()
        .dateien
        .iter()
        .find(|(p, _)| p.file_name().is_some_and(|n| n == name))
        .map(|(_, d)| d.bindungen.iter().map(|b| b.feld_id.clone()).collect())
        .unwrap_or_default()
}

/// Die Slots, die `felder` tragen.
fn traeger(felder: &[String], je_feld: &BTreeMap<&str, &str>) -> BTreeSet<String> {
    felder
        .iter()
        .filter_map(|f| je_feld.get(f.as_str()).map(|s| (*s).to_owned()))
        .collect()
}

/// Was `quantitaet` liest, laut Tabelle.
fn gelesen_von(quantitaet: &str) -> BTreeSet<String> {
    LESESTELLEN
        .iter()
        .filter(|(_, _, qs)| qs.contains(&quantitaet))
        .map(|(_, s, _)| (*s).to_owned())
        .collect()
}

/// `(was, wo)` fuer jeden Slot in `gelesen`, der in `traeger` fehlt.
fn fehlende_traeger(
    wo: &str,
    gelesen: &BTreeSet<String>,
    traeger: &BTreeSet<String>,
) -> Vec<String> {
    gelesen
        .iter()
        .filter(|s| !traeger.contains(*s))
        .map(|s| format!("Slot {s:?} hat in {wo} keinen Traeger"))
        .collect()
}

// ---- Die Tests --------------------------------------------------------------------------------------------------

/// Der Scan findet genau die Lesestellen der Tabelle.
#[test]
fn die_lesestellen_im_quelltext_sind_genau_die_der_tabelle() {
    let gefunden = gefundene_lesestellen();
    let tabelle = tabellen_lesestellen();
    assert!(
        gefunden.len() >= MIN_LESESTELLEN,
        "der Scan fand nur {} Lesestellen (erwartet mindestens {MIN_LESESTELLEN}): Quelltext verschoben oder \
         Regex veraltet; er saehe nichts: {gefunden:?}",
        gefunden.len()
    );
    let neu: Vec<_> = gefunden.difference(&tabelle).collect();
    let tot: Vec<_> = tabelle.difference(&gefunden).collect();
    assert!(
        neu.is_empty(),
        "der Rechenkern liest einen Slot, den die Tabelle LESESTELLEN nicht kennt (Datei, Slot): {neu:?}. \
         Trage die Stelle mit den Quantitaeten ein, die ueber sie laufen; dann prueft der Test den Traeger."
    );
    assert!(
        tot.is_empty(),
        "die Tabelle LESESTELLEN nennt Lesestellen, die der Quelltext nicht (mehr) hat (Datei, Slot): {tot:?}"
    );
}

/// Jeder Slot, den eine Quantitaet liest, hat in der Bindung ihrer Scheibe einen Traeger: in der Feldliste (`felder`,
/// was `/ergebnis` bekommt) und im Kegel (die Pflicht-Achsen der Spannenrechnung, was `/fragen` und `/stand` bekommen).
#[test]
fn jeder_gelesene_slot_hat_in_der_bindung_seiner_scheibe_einen_traeger() {
    let je_feld = slot_je_feld();
    let mut geprueft = 0usize;
    let mut fehler = Vec::new();
    for (quantitaet, scheibe) in QUANTITAETEN {
        let cfg = Cfg::fuer(*scheibe);
        assert_eq!(
            cfg.gesamt_ring(),
            Some(*quantitaet),
            "{scheibe:?} traegt nicht die erwartete Quantitaet {quantitaet}"
        );
        let gelesen = gelesen_von(quantitaet);
        let felder = cfg.felder(felder_der_datei).unwrap();
        let kegel = cfg.kegel(felder_der_datei).unwrap();
        for (wo, liste) in [("den felder", &felder), ("dem kegel", &kegel)] {
            let wo = format!("{scheibe:?} ({quantitaet}), {wo}");
            fehler.extend(fehlende_traeger(&wo, &gelesen, &traeger(liste, &je_feld)));
            geprueft += gelesen.len();
        }
    }
    // Die Teil-Ringe von `n_vor_gwg`: eigene Feldliste je Ring, gerechnet mit der Quantitaet des Rings.
    let nvg = Cfg::fuer(Scheibe::NVorGwg);
    assert!(
        !nvg.teil_ringe().is_empty(),
        "n_vor_gwg hat keine Teil-Ringe mehr: die Pruefung unten saehe nichts"
    );
    for (familie, quantitaet, felder) in nvg.teil_ringe() {
        let gelesen = gelesen_von(quantitaet);
        let liste: Vec<String> = felder.iter().map(|f| (*f).to_owned()).collect();
        let wo = format!("n_vor_gwg, Teil-Ring {familie} ({quantitaet})");
        fehler.extend(fehlende_traeger(&wo, &gelesen, &traeger(&liste, &je_feld)));
        geprueft += gelesen.len();
    }
    assert!(
        fehler.is_empty(),
        "SLOT OHNE TRAEGER, `/ergebnis`, `/fragen` und `/stand` dieser Scheibe antworten mit 500 (SlotFehlt): {fehler:#?}"
    );
    assert!(
        geprueft >= MIN_PRUEFUNGEN,
        "nur {geprueft} Slot-Pruefungen (erwartet mindestens {MIN_PRUEFUNGEN}): LESESTELLEN oder eine Scheibenliste \
         ist kleiner geworden. Absicht? Dann senke MIN_PRUEFUNGEN im selben Commit; sonst fehlt eine Zeile."
    );
}

/// Jede Quantitaet, die `zweige::Quantitaet` kennt, steht in [`QUANTITAETEN`] (und umgekehrt jede Scheibe mit
/// `gesamt_ring`): eine neue Quantitaet ohne Zeile wuerde sonst nie auf ihre Slots geprueft.
#[test]
fn jede_scheibe_mit_gesamt_ring_steht_in_der_quantitaetentabelle() {
    // Die `match`-Form bricht beim Uebersetzen, wenn `Scheibe` eine Variante bekommt.
    fn traegt_gesamt_ring(s: Scheibe) -> bool {
        match s {
            Scheibe::Ep | Scheibe::AnGesamt | Scheibe::Gesamt | Scheibe::RentnerGesamt => true,
            Scheibe::NVorGwg => false,
        }
    }
    for s in [
        Scheibe::Ep,
        Scheibe::NVorGwg,
        Scheibe::AnGesamt,
        Scheibe::Gesamt,
        Scheibe::RentnerGesamt,
    ] {
        let cfg = Cfg::fuer(s);
        assert_eq!(cfg.gesamt_ring().is_some(), traegt_gesamt_ring(s), "{s:?}");
        if let Some(q) = cfg.gesamt_ring() {
            assert!(
                QUANTITAETEN.iter().any(|(n, sch)| *n == q && *sch == s),
                "{s:?} rechnet {q}, das QUANTITAETEN nicht kennt: seine Slots wuerden nie geprueft"
            );
        }
    }
}

/// Der Scan setzt voraus, dass jeder `Slots`-Parameter `slots` heisst; ein anderer Name (`s`, `werte`) liesse
/// `s.get("x")` unsichtbar. Der Test zaehlt die Parameter und verlangt den Namen.
#[test]
fn jeder_slots_parameter_heisst_slots() {
    let m = Regex::new(r"\b(\w+)\s*:\s*&(?:'\w+\s+)?Slots\b").unwrap();
    let mut dateien = Vec::new();
    rs_dateien(&src_wurzel(), &mut dateien);
    let mut namen: Vec<(String, String)> = Vec::new();
    for d in dateien {
        let code = code_ohne_kommentarzeilen(&std::fs::read_to_string(&d).unwrap());
        for c in m.captures_iter(&code) {
            namen.push((
                d.file_name().unwrap().to_string_lossy().into_owned(),
                c[1].to_owned(),
            ));
        }
    }
    assert!(
        namen.len() >= MIN_SLOTS_PARAMETER,
        "nur {} Parameter vom Typ &Slots gefunden (erwartet mindestens {MIN_SLOTS_PARAMETER}): {namen:?}",
        namen.len()
    );
    let fremd: Vec<_> = namen
        .iter()
        .filter(|(_, n)| n != "slots" && n != "_slots")
        .collect();
    assert!(
        fremd.is_empty(),
        "ein `&Slots`-Parameter heisst nicht `slots`; der Scan sieht seine Lesestellen nicht: {fremd:?}"
    );
}

// ---- Die Hilfsfunktionen auf Fehlerfaelle ----------------------------------------------------------------------

/// Der Scan findet alle drei Formen, auch ueber einen Zeilenumbruch, und ignoriert Kommentarzeilen.
#[test]
fn der_scan_findet_die_drei_formen_und_ueberliest_kommentare() {
    let text = r#"
        let a = slot(slots, "eins")?;
        let b = slots
            .get("zwei")
            .ok_or_else(|| BescheidFehler::SlotFehlt("drei".into()))?;
        // slot(slots, "kommentar")
        /// slots.get("doc")
        let c = slot(slots, name)?; // keine Literal-Form
    "#;
    let gefunden: Vec<_> = slot_literale(text).into_iter().collect();
    assert_eq!(gefunden, ["drei", "eins", "zwei"]);
}

/// Die Traeger-Pruefung wird rot, wenn ein gelesener Slot keinen Traeger hat, und gruen, wenn er einen hat.
#[test]
fn die_traeger_pruefung_meldet_einen_fehlenden_slot() {
    let gelesen: BTreeSet<String> = ["a", "b"].iter().map(|s| (*s).to_owned()).collect();
    let je_feld: BTreeMap<&str, &str> = [("fa", "a"), ("fb_x", "b_x")].into_iter().collect();
    let felder = vec!["fa".to_owned(), "fb_x".to_owned(), "ohne_slot".to_owned()];
    let t = traeger(&felder, &je_feld);
    let fehler = fehlende_traeger("test", &gelesen, &t);
    assert_eq!(fehler.len(), 1, "{fehler:?}");
    assert!(fehler[0].contains("\"b\""), "{fehler:?}");
    let ok: BTreeSet<String> = ["a"].iter().map(|s| (*s).to_owned()).collect();
    assert!(fehlende_traeger("test", &ok, &t).is_empty());
}
