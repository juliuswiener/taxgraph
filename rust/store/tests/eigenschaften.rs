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

// ---- Das zweite Kind: die Ableitung feuert je Instanz --------------------------------------------
// Gegenstueck zu `tests/test_ableitung_reihenfolge.py` (Abschnitt „Das zweite Kind"). `rechne_ab`
// verglich `feld_id` ohne `__2` mit `aus` und `und_feld`: Kind 2 bekam nie eine Qualifikation, auch
// dann nicht, wenn die Zyklus-Sperre (`ziel in aktiv`) gar nicht griff (Kind 2 allein).

const ZIEL: &str = "kind_unter_14_haushaltszugehoerig";
const ZIEL2: &str = "kind_unter_14_haushaltszugehoerig__2";
const GEB: (&str, &str) = ("kind_geburtsdatum", "01.03.2020");
const HAUS: (&str, &str) = (
    "kind_betreuung_haushaltszugehoerigkeit_zeitraum",
    "01.01-31.12",
);
const GEB2: (&str, &str) = ("kind_geburtsdatum__2", "01.03.2021");
const HAUS2: (&str, &str) = (
    "kind_betreuung_haushaltszugehoerigkeit_zeitraum__2",
    "01.01-31.12",
);

/// Bestaetigte Antworten in dieser Reihenfolge auf einem leeren Store (VZ 2025).
fn store_mit(antworten: &[(&str, Value)]) -> Store {
    let mut s = Store::leer(2025, None);
    for (i, (feld, wert)) in antworten.iter().enumerate() {
        let ts = format!("2026-01-01T00:00:{:02}+00:00", i % 60);
        let neu = neues_event(feld, wert, true, None, ts);
        s.append(&neu, None, nachschlag()).unwrap();
    }
    s
}

fn text(paar: (&'static str, &str)) -> (&'static str, Value) {
    (paar.0, json!(paar.1))
}

/// `(wert, signal_2)` der Ableitung auf `feld`, sonst `None` (kein Event oder von Hand geschrieben).
fn ableitung_auf(s: &Store, feld: &str) -> Option<(PyWert, String)> {
    let e = s.aktives(feld)?;
    (e.schreiber == Schreiber::Abgeleitet("ableitung".to_string())).then(|| {
        let signal_2 = e.signal.as_ref().and_then(|g| g.signal_2.clone());
        (e.wert.clone(), signal_2.unwrap_or_default())
    })
}

#[test]
fn zweites_kind_leitet_seine_qualifikation_ab() {
    // Kind 2 allein: ZIEL ist nicht aktiv, die Zyklus-Sperre kann nicht schuld sein. Beide
    // Ausloeser, in beiden Reihenfolgen.
    for antworten in [[GEB2, HAUS2], [HAUS2, GEB2]] {
        let s = store_mit(&antworten.map(text));
        assert_eq!(
            ableitung_auf(&s, ZIEL2),
            Some((
                PyWert::Bool(true),
                "ableitung@kind_geburtsdatum__2".to_string()
            )),
            "{antworten:?}"
        );
        assert!(s.aktives(ZIEL).is_none(), "Instanz 2 schrieb in Instanz 1");
    }
}

#[test]
fn beide_kinder_leiten_je_instanz_ab() {
    let s = store_mit(&[GEB, HAUS, GEB2, HAUS2].map(text));
    let wahr = |f| ableitung_auf(&s, f).map(|(w, _)| w);
    assert_eq!(wahr(ZIEL), Some(PyWert::Bool(true)));
    assert_eq!(wahr(ZIEL2), Some(PyWert::Bool(true)));
}

#[test]
fn antworten_je_instanz_bleiben_stehen_und_sperren_die_andere_nicht() {
    // Ein Nein auf Instanz 2 bleibt: die Ableitung ueberschreibt keine Nutzerantwort.
    let mut antworten = vec![(ZIEL2, json!(false))];
    antworten.extend([GEB2, HAUS2].map(text));
    let s = store_mit(&antworten);
    let e = s.aktives(ZIEL2).unwrap();
    assert_eq!(e.wert, PyWert::Bool(false));
    assert_eq!(e.schreiber, Schreiber::Mensch("julius".to_string()));

    // Ein Nein auf Instanz 1 sperrt Instanz 2 nicht: die Sperre gilt je Instanz, nicht je
    // Basisfeld. Haette der Fix nur den Suffix im Vergleich abgeschnitten, laege ZIEL in `aktiv`.
    let mut antworten = vec![(ZIEL, json!(false))];
    antworten.extend([GEB2, HAUS2].map(text));
    let s = store_mit(&antworten);
    assert_eq!(s.aktives(ZIEL).unwrap().wert, PyWert::Bool(false));
    assert_eq!(
        ableitung_auf(&s, ZIEL2).map(|(w, _)| w),
        Some(PyWert::Bool(true))
    );
}

#[test]
fn quelle_und_und_feld_verschiedener_instanzen_leiten_nichts_ab() {
    for antworten in [
        vec![GEB2],
        vec![GEB2, GEB],
        vec![GEB2, HAUS],
        vec![GEB, HAUS2],
        vec![("kind_geburtsdatum__2", "01.03.2009"), HAUS2],
        // Pythons `$` passt vor einem abschliessenden `\n`: `…__2\n` ist ein Instanz-Feld des
        // Typpruefers, aber kein `aus` der Instanz 2 (`test_ableitung_reihenfolge.py`).
        vec![("kind_geburtsdatum__2\n", "01.03.2021"), HAUS2],
    ] {
        let s = store_mit(&antworten.iter().copied().map(text).collect::<Vec<_>>());
        assert!(ableitung_auf(&s, ZIEL2).is_none(), "{antworten:?}");
    }
}

#[test]
fn ableitung_ohne_instanz_gruppe_ignoriert_den_suffix() {
    // `geburtsjahr` hat keine `instanz_gruppe`: `stammdaten_geburtsdatum__2` erzeugt kein
    // `geburtsjahr__2` (und, wie bisher, auch kein `geburtsjahr`).
    let s = store_mit(&[("stammdaten_geburtsdatum__2", json!("05.05.1955"))]);
    let felder: Vec<&str> = s.events().iter().map(|e| e.feld_id.as_str()).collect();
    assert_eq!(felder, ["stammdaten_geburtsdatum__2"]);
}

// ---- Die Ableitung schreibt keinen Wert ausserhalb von `bereich` -----------------------------------
// Gegenstueck zu `tests/test_bindung_bereich_serverseitig.py` (Abschnitt „Der Ableitungsweg") und
// Vault `decisions/ableitung-schreibt-keinen-wert-ausserhalb-des-bereichs`. `rechne_ab` haengte sein
// Ergebnis ueber `push_neu` an `pruefe_bindung` vorbei: aus 15.07.1850 wurde `geburtsjahr` 1850
// (Bereich 1900..2010), und die Frage verschwand.

const QUELLEN: [(&str, &str); 2] = [
    ("stammdaten_geburtsdatum", "geburtsjahr"),
    ("stammdaten_geburtsdatum_partner", "geburtsjahr_partner"),
];

#[test]
fn ableitung_schreibt_kein_geburtsjahr_ausserhalb_des_bereichs() {
    for (quelle, ziel) in QUELLEN {
        for datum in [
            "15.07.1850",
            "31.12.1899",
            "01.01.2011",
            "15.07.2025",
            "15.07.2999",
        ] {
            let s = store_mit(&[(quelle, json!(datum))]);
            assert!(
                s.aktives(ziel).is_none(),
                "{quelle}={datum}: Ableitung schrieb {ziel}"
            );
            // Das Datum selbst bleibt in der Akte: nur die Ableitung daraus unterbleibt.
            assert_eq!(s.aktives(quelle).unwrap().wert, PyWert::Text(datum.into()));
        }
    }
}

#[test]
fn ableitung_schreibt_das_jahr_im_bereich_und_die_null() {
    // Raender und Mitte wie bisher. Die 0 (Datum 01.01.0000) geht wie beim Speichern durch
    // (`pruefe_bindung`: eine Regel, eine Stelle); ob eine GERECHNETE 0 dasselbe verdient wie eine
    // getippte, ist offen (Bericht, „Entscheidung fuer Julius").
    for (quelle, ziel) in QUELLEN {
        for (datum, jahr) in [
            ("01.01.1900", 1900),
            ("15.07.1960", 1960),
            ("31.12.2010", 2010),
            ("01.01.0000", 0),
        ] {
            let s = store_mit(&[(quelle, json!(datum))]);
            assert_eq!(
                ableitung_auf(&s, ziel).map(|(w, _)| w),
                Some(PyWert::Ganz(jahr)),
                "{quelle}={datum}"
            );
        }
    }
}

#[test]
fn ableitung_ohne_bereich_am_ziel_laeuft_weiter() {
    // Kontrolle: `rentner_alter_64_erfuellt` (bool, ohne `bereich`) wird auch fuer 1850 abgeleitet.
    let s = store_mit(&[("stammdaten_geburtsdatum", json!("15.07.1850"))]);
    assert_eq!(
        ableitung_auf(&s, "rentner_alter_64_erfuellt").map(|(w, _)| w),
        Some(PyWert::Bool(true))
    );
}

#[test]
fn bestaetigte_null_und_vorjahreswert_am_ziel_sperren_die_ableitung_weiter() {
    // Entscheidung Punkt 4: `ziel in aktiv` bleibt, auch bei einer bestaetigten 0.
    let s = store_mit(&[
        ("geburtsjahr", json!(0)),
        ("stammdaten_geburtsdatum", json!("15.07.1960")),
    ]);
    let e = s.aktives("geburtsjahr").unwrap();
    assert_eq!(
        (&e.wert, &e.schreiber),
        (&PyWert::Ganz(0), &Schreiber::Mensch("julius".to_string()))
    );
    let mut s = Store::leer(2025, None);
    let vorjahr = NeuesEvent {
        feldzustand: Feldzustand::Vorlaeufig,
        schreiber: Schreiber::ImportVorjahr,
        herkunft: Herkunft {
            herkunft: Achsenwert::new("vorjahr").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        ..neues_event(
            "geburtsjahr",
            &json!(1950),
            false,
            None,
            "2026-01-01T00:00:00+00:00".to_string(),
        )
    };
    s.append(&vorjahr, None, nachschlag()).unwrap();
    let datum = neues_event(
        "stammdaten_geburtsdatum",
        &json!("15.07.1960"),
        true,
        None,
        "2026-01-01T00:00:01+00:00".to_string(),
    );
    s.append(&datum, None, nachschlag()).unwrap();
    assert_eq!(s.aktives("geburtsjahr").unwrap().wert, PyWert::Ganz(1950));
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

/// `EventId::parse` haelt `schema.json` (`^[a-f0-9]{64}$`). `u8::from_str_radix` allein liest
/// auch Grossbuchstaben und ein fuehrendes `+`. Python vergleicht `ersetzt` als Text
/// (`store.py:422`) und findet zu `"AB" * 32` kein Ziel; Rust darf daraus kein `[0xab; 32]` lesen.
#[test]
fn event_id_parse_haelt_das_schema_muster() {
    for text in ["AB".repeat(32), "+f".repeat(32)] {
        let ergebnis = EventId::parse(&text);
        assert!(ergebnis.is_err(), "{text} -> {ergebnis:?}");
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Roundtrip: `speichere` (JSON) -> `lade` (JSON) gibt jedes Event unveraendert zurueck, und
    /// jeder geladene `event_id` passt noch zum Inhalt. Die Texte haengen als Ersetzungskette an
    /// einem ungebundenen Feld; dort laesst Auflage T jeden Text durch. Mit dem frueheren
    /// YAML-Leser war die Eigenschaft ohne Filter rot: `"\u{7f}"` lud nicht, `"\u{2028} "`
    /// verlor das Leerzeichen.
    #[test]
    fn speichern_und_laden_ist_verlustfrei(
        texte in proptest::collection::vec(domain::testhilfe::text(), 1..6)
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

/// DEL und die C1-Zeichen U+0080..U+009F stehen im JSON roh, und Auflage T laesst sie in einem
/// Textfeld durch (`nur_xml_zeichen`). `lade` liest JSON wie Python (`api.py::lade_fall`,
/// `json.load`) und gibt sie unveraendert zurueck. Der fruehere YAML-Leser wies DEL/C1 ab (die
/// GANZE Fallakte lud nicht mehr), machte aus NEL (U+0085) ein Leerzeichen und liess ein
/// Leerzeichen neben U+2028/U+2029 fallen — der `event_id` passte danach nicht mehr zum Text.
/// Die Rust-`api` laedt ueber `lade` (`api/src/eigener_fall.rs:80`).
///
/// Seit Auflage Z weist `append` DEL/C1/U+2028 an einem Textfeld ab (ELSTER nimmt sie nicht an,
/// `rust/store/tests/zeichensatz.rs`). Die Akten mit solchen Werten gibt es trotzdem: sie stammen
/// von vor Auflage Z. Der Test legt sie darum ohne Bindung an (`leer`, wie ein Altbestand) und
/// prueft, dass `lade` sie unveraendert zurueckgibt: Laden prueft nie.
#[test]
fn textfeld_mit_c1_zeichen_laedt_wieder() {
    let leer = HashMap::new();
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
        s.append(&neu, None, BindungNachschlag::neu(&leer)).unwrap();
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
