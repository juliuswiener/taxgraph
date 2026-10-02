//! Eigenschaften des Stores ohne Python-Orakel, gegen die echte Bindung (`REWRITE_PLAN.md` §9).
//!
//! 1. Gepflegter `aktiv`-Index, Neuaufbau aus der Datei und `materialisiere` zeigen nach jedem
//!    Schritt denselben Stand; je `feld_id` ist hoechstens ein Event aktiv.
//! 2. Jeder Snapshot laesst sich aus seinem `bis_event` nachrechnen, auch nach spaeteren Events.
//! 3. `EventId` geht verlustfrei durch Hex-Text und JSON.
//! 4. `speichere` -> `lade` gibt jedes Event unveraendert zurueck, sein `event_id` bleibt gueltig.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, PyWert, Schreiber, Signal2};
use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use serde_json::{json, Value};
use store::{BindungNachschlag, EventId, NeuesEvent, SnapshotFeld, Store};

fn nachschlag() -> BindungNachschlag<'static> {
    static BINDUNGEN: OnceLock<Vec<Bindung>> = OnceLock::new();
    static MAP: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    BindungNachschlag::neu(MAP.get_or_init(|| {
        store::baue_nachschlag(BINDUNGEN.get_or_init(|| {
            let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
            bindung::lade_registry(&pfad)
                .unwrap()
                .dateien
                .into_iter()
                .flat_map(|(_, d)| d.bindungen)
                .collect()
        }))
    }))
}

fn neues_event(
    feld_id: &str,
    wert: &Value,
    bestaetigt: bool,
    ersetzt: Option<EventId>,
    ts: String,
) -> NeuesEvent {
    NeuesEvent {
        feld_id: feld_id.to_string(),
        wert: wert.clone().into(),
        feldzustand: if bestaetigt {
            Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick").unwrap(),
            }
        } else {
            Feldzustand::Vorlaeufig
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("mensch").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("julius".to_string()),
        signal_1: None,
        ersetzt,
        ts: Some(ts),
    }
}

/// Felder der echten Bindung mit typgerechten Werten. Sie erreichen alle drei Ableitungswege:
/// `stammdaten_geburtsdatum` (`ableitung` -> `geburtsjahr`, `rentner_alter_64_erfuellt`),
/// `fam_anzahl_kinder` (`beweist` -> `kein_kind`) und `kind_geburtsdatum` mit `und_feld`
/// (-> `kind_unter_14_haushaltszugehoerig`). Die Ziele stehen selbst im Pool: so trifft ein
/// Mensch-Event auch ein schon abgeleitetes Feld.
fn pool() -> Vec<(&'static str, Value)> {
    vec![
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_geburtsdatum", json!("01.01.1961")),
        ("stammdaten_geburtsdatum", json!("02.01.1961")),
        ("stammdaten_geburtsdatum", json!("24.12.1990")),
        ("geburtsjahr", json!(1955)),
        ("rentner_alter_64_erfuellt", json!(false)),
        ("fam_anzahl_kinder", json!(0)),
        ("fam_anzahl_kinder", json!(1)),
        ("fam_anzahl_kinder", json!(3)),
        ("kein_kind", json!(true)),
        ("kind_geburtsdatum", json!("10.10.2015")),
        ("kind_geburtsdatum", json!("01.01.2000")),
        (
            "kind_betreuung_haushaltszugehoerigkeit_zeitraum",
            json!("01.01-31.12"),
        ),
        ("kind_unter_14_haushaltszugehoerig", json!(false)),
    ]
}

#[derive(Debug, Clone)]
enum Schritt {
    /// Ein Event ohne `ersetzt` (Feld, Wert, bestaetigt).
    Neu((&'static str, Value), bool),
    /// Ein Event mit `ersetzt`: `None` zielt auf das aktive Event des Feldes, `Some(i)` auf
    /// `events[i % len]` — meist ein fremdes oder schon ersetztes Ziel, also eine Abweisung.
    Ersetze((&'static str, Value), bool, Option<usize>),
    Snapshot,
}

fn schritte() -> impl Strategy<Value = Vec<Schritt>> {
    let feld = || prop::sample::select(pool());
    let schritt = prop_oneof![
        3 => (feld(), any::<bool>()).prop_map(|(f, b)| Schritt::Neu(f, b)),
        2 => (feld(), any::<bool>(), proptest::option::of(any::<usize>()))
            .prop_map(|(f, b, z)| Schritt::Ersetze(f, b, z)),
        1 => Just(Schritt::Snapshot),
    ];
    proptest::collection::vec(schritt, 1..40)
}

/// Spielt `schritte` auf einem leeren Store (VZ 2025) ab und ruft `pruefe` nach jedem Schritt.
/// Haelt zugleich fest, dass der Pool nie an Auflage T/F scheitert: ein Event ohne `ersetzt`
/// geht genau dann durch, wenn das Feld noch kein aktives Event hat, und das aktive Event zu
/// ersetzen gelingt immer.
fn spiele(
    schritte: &[Schritt],
    mut pruefe: impl FnMut(&Store) -> Result<(), TestCaseError>,
) -> Result<Store, TestCaseError> {
    let unbekannt = EventId::aus_bytes([0xee; 32]);
    let mut s = Store::leer(2025, None);
    for (i, schritt) in schritte.iter().enumerate() {
        let ts = format!("2026-01-01T00:00:{:02}+00:00", i % 60);
        match schritt {
            Schritt::Neu((feld, wert), bestaetigt) => {
                let war_aktiv = s.aktives(feld).is_some();
                let neu = neues_event(feld, wert, *bestaetigt, None, ts);
                let ergebnis = s.append(&neu, None, nachschlag());
                prop_assert_eq!(ergebnis.is_ok(), !war_aktiv, "{:?}", ergebnis);
            }
            Schritt::Ersetze((feld, wert), bestaetigt, ziel) => {
                let aktiv = s.aktives(feld).map(|e| e.event_id);
                let ziel = match ziel {
                    None => aktiv,
                    Some(i) => s
                        .events()
                        .get(i % s.events().len().max(1))
                        .map(|e| e.event_id),
                };
                let neu = neues_event(feld, wert, *bestaetigt, Some(ziel.unwrap_or(unbekannt)), ts);
                let ergebnis = s.append(&neu, None, nachschlag());
                if ziel.is_some() && ziel == aktiv {
                    prop_assert!(ergebnis.is_ok(), "{:?}", ergebnis);
                }
            }
            Schritt::Snapshot => {
                let leer = s.events().is_empty();
                prop_assert_eq!(s.erzeuge_snapshot(None, Some(ts), None).is_err(), leer);
            }
        }
        pruefe(&s)?;
    }
    Ok(s)
}

/// Drei Lesarten desselben Logs: hoechstens ein nicht ersetztes Event je Feld (eigene Pruefung —
/// Index und Neuaufbau nehmen beide "das letzte gewinnt" und stimmten auch bei zweien ueberein),
/// gepflegter Index == Neuaufbau aus der Datei, `materialisiere` == die aktiven Events.
fn pruefe_stand(s: &Store) -> Result<(), TestCaseError> {
    let ersetzt: HashSet<EventId> = s.events().iter().filter_map(|e| e.ersetzt).collect();
    let mut gesehen = HashSet::new();
    for e in s.events().iter().filter(|e| !ersetzt.contains(&e.event_id)) {
        prop_assert!(
            gesehen.insert(&e.feld_id),
            "zwei aktive Events fuer {}",
            e.feld_id
        );
    }
    let index = |s: &Store| -> BTreeMap<String, EventId> {
        s.aktive()
            .map(|(f, e)| (f.to_string(), e.event_id))
            .collect()
    };
    prop_assert_eq!(index(s), index(&Store::aus_datei(s.datei().clone())));
    let aktive: BTreeMap<String, SnapshotFeld> = s
        .aktive()
        .map(|(f, e)| {
            let feld = SnapshotFeld {
                wert: e.wert.clone(),
                zustand: e.zustand,
                herkunft: e.herkunft.clone(),
            };
            (f.to_string(), feld)
        })
        .collect();
    prop_assert_eq!(s.materialisiere(None).unwrap().0, aktive);
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Symmetrie der Zweige: der beim Anhaengen gepflegte Index (inkl. Ableitungen) und die
    /// Faltung des Logs (`aus_datei`, `materialisiere`) sind nach jedem Schritt derselbe Stand.
    #[test]
    fn index_datei_und_faltung_sind_derselbe_stand(schritte in schritte()) {
        spiele(&schritte, pruefe_stand)?;
    }

    /// Roundtrip: jeder Snapshot ergibt sich aus seinem `bis_event` neu — `felder` und
    /// `snapshot_id` —, auch wenn spaeter Events sein Feld ersetzt haben.
    #[test]
    fn jeder_snapshot_bleibt_nachrechenbar(schritte in schritte()) {
        let s = spiele(&schritte, |_| Ok(()))?;
        for snap in s.snapshots() {
            prop_assert_eq!(
                s.materialisiere(Some(snap.bis_event)),
                Ok((snap.felder.clone(), snap.snapshot_id))
            );
        }
    }
}

/// Gegen leere Eigenschaften: der Pool erreicht alle drei Ableitungswege wirklich.
#[test]
fn der_pool_erreicht_alle_ableitungen() {
    let schritte = [
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("fam_anzahl_kinder", json!(1)),
        ("kind_geburtsdatum", json!("10.10.2015")),
        (
            "kind_betreuung_haushaltszugehoerigkeit_zeitraum",
            json!("01.01-31.12"),
        ),
    ]
    .map(|f| Schritt::Neu(f, true));
    let s = spiele(&schritte, pruefe_stand).unwrap();
    let abgeleitet: BTreeMap<&str, (&Schreiber, &PyWert)> = s
        .events()
        .iter()
        .filter(|e| matches!(e.schreiber, Schreiber::Abgeleitet(_)))
        .map(|e| (e.feld_id.as_str(), (&e.schreiber, &e.wert)))
        .collect();
    let ableitung = Schreiber::Abgeleitet("ableitung".to_string());
    let beweist = Schreiber::Abgeleitet("beweist".to_string());
    assert_eq!(
        abgeleitet,
        BTreeMap::from([
            ("geburtsjahr", (&ableitung, &PyWert::Ganz(1955))),
            // 1955 + 64 < 2025
            (
                "rentner_alter_64_erfuellt",
                (&ableitung, &PyWert::Bool(true))
            ),
            ("kein_kind", (&beweist, &PyWert::Bool(false))),
            // 2025 - 2015 < 14, ausgeloest erst ueber das `und_feld`
            (
                "kind_unter_14_haushaltszugehoerig",
                (&ableitung, &PyWert::Bool(true))
            ),
        ])
    );
}

proptest! {
    /// `EventId` -> Hex-Text -> `EventId`, ebenso ueber JSON, ist verlustfrei; der Text sind
    /// immer 64 Zeichen `[0-9a-f]`, das Muster aus `schema.json`.
    #[test]
    fn event_id_hex_ist_ein_roundtrip(bytes in any::<[u8; 32]>()) {
        let id = EventId::aus_bytes(bytes);
        let text = id.to_string();
        prop_assert!(
            text.len() == 64 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
            "{}", text
        );
        prop_assert_eq!(EventId::parse(&text), Ok(id));
        let json = serde_json::to_string(&id).unwrap();
        prop_assert_eq!(serde_json::from_str::<EventId>(&json).unwrap(), id);
    }
}

/// Befund: `EventId::parse` nimmt mehr an als `schema.json` (`^[a-f0-9]{64}$`), denn
/// `u8::from_str_radix` liest Grossbuchstaben und ein fuehrendes `+`. Python vergleicht `ersetzt`
/// als Text (`store.py:422`) und findet zu `"AB" * 32` kein Ziel; Rust liest `[0xab; 32]`.
#[test]
#[ignore = "Befund: EventId::parse nimmt Grossbuchstaben und '+' an, schema.json nur [a-f0-9]"]
fn event_id_parse_haelt_das_schema_muster() {
    for text in ["AB".repeat(32), "+f".repeat(32)] {
        let ergebnis = EventId::parse(&text);
        assert!(ergebnis.is_err(), "{text} -> {ergebnis:?}");
    }
}

/// Was `lade` heute aus einem von `speichere` geschriebenen Text zurueckliest (gemessen, s.
/// `textfeld_mit_c1_zeichen_laedt_wieder`): kein U+007F..U+009F, kein U+FFFE/U+FFFF und kein
/// Leerzeichen neben U+2028/U+2029.
fn yaml_lesbar(t: &str) -> bool {
    !t.chars()
        .any(|c| matches!(c, '\u{7f}'..='\u{9f}' | '\u{fffe}' | '\u{ffff}'))
        && !["\u{2028} ", " \u{2028}", "\u{2029} ", " \u{2029}"]
            .iter()
            .any(|m| t.contains(m))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Roundtrip: `speichere` (JSON) -> `lade` (YAML) gibt jedes Event unveraendert zurueck, und
    /// jeder geladene `event_id` passt noch zum Inhalt. Die Texte haengen als Ersetzungskette an
    /// einem ungebundenen Feld; dort laesst Auflage T jeden Text durch. Ohne den Filter
    /// [`yaml_lesbar`] ist die Eigenschaft rot: `"\u{7f}"` laedt nicht, `"\u{2028} "` verliert
    /// das Leerzeichen.
    #[test]
    fn speichern_und_laden_ist_verlustfrei(
        texte in proptest::collection::vec(
            domain::testhilfe::text().prop_filter("Befund YAML", |t| yaml_lesbar(t)),
            1..6,
        )
    ) {
        let mut s = Store::leer(2025, None);
        let mut vorher = None;
        for (i, text) in texte.iter().enumerate() {
            let ts = format!("2026-01-01T00:00:{i:02}+00:00");
            let neu = neues_event("eigenschaft_freitext", &json!(text), true, vorher, ts);
            vorher = Some(s.append(&neu, None, nachschlag()).unwrap());
        }
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-eigenschaften-{}",
            std::process::id()
        ));
        let pfad = dir.join("fall.json");
        store::speichere(&pfad, s.datei()).unwrap();
        let geladen = store::lade(&pfad).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(&geladen.events, &s.datei().events);
        for e in &geladen.events {
            prop_assert_eq!(e.berechne_event_id().unwrap(), e.event_id);
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}

/// Befund: `speichere` schreibt JSON, `lade` liest YAML (wie `store.py`/`api.py`). DEL und die
/// C1-Zeichen U+0080..U+009F stehen im JSON roh, und Auflage T laesst sie in einem Textfeld durch
/// (`nur_xml_zeichen`). libyaml weist sie ab: die GANZE Fallakte laedt nicht mehr. NEL (U+0085),
/// U+2028 und U+2029 liest YAML als Zeilenumbruch: aus NEL wird ein Leerzeichen, ein Leerzeichen
/// neben U+2028/U+2029 faellt weg — der `event_id` passt danach nicht mehr zum Text. Die
/// Rust-`api` laedt ueber `lade` (`api/src/eigener_fall.rs:80`), Python ueber `json.load`
/// (`api.py::lade_fall`). Gemessen: alle Unicode-Skalare einzeln (sonst bricht nur U+FFFE/U+FFFF,
/// die Auflage T abweist) und alle 11.111 Texte bis Laenge 4 aus zehn Rand-Zeichen.
#[test]
#[ignore = "Befund: lade (YAML) liest DEL/C1 nicht zurueck, NEL und Leerzeichen an U+2028 aendern sich"]
fn textfeld_mit_c1_zeichen_laedt_wieder() {
    let dir = std::env::temp_dir().join(format!(
        "taxgraph-store-eigenschaften-c1-{}",
        std::process::id()
    ));
    let pfad = dir.join("fall.json");
    let kaputt: Vec<String> = [
        "Maier\u{7f}",
        "Maier\u{80}",
        "Maier\u{85}",
        "Maier\u{9f}",
        "Maier \u{2028}Huber",
        "Maier\u{2029} Huber",
    ]
    .into_iter()
    .filter_map(|text| {
        let mut s = Store::leer(2025, None);
        let ts = "2026-01-01T00:00:00+00:00".to_string();
        let neu = neues_event("stammdaten_nachname", &json!(text), true, None, ts);
        s.append(&neu, None, nachschlag()).unwrap();
        store::speichere(&pfad, s.datei()).unwrap();
        match store::lade(&pfad) {
            Ok(d) if d.events == s.datei().events => None,
            Ok(d) => Some(format!("{text:?}: geladen als {:?}", d.events[0].wert)),
            Err(e) => Some(format!("{text:?}: {e}")),
        }
    })
    .collect();
    std::fs::remove_dir_all(&dir).ok();
    assert!(kaputt.is_empty(), "{kaputt:#?}");
}
