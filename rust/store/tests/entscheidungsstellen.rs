//! Entscheidungsstellen des Stores, am Aufrufort von `Store::append`, `Store::append_roh` und
//! `Store::erzeuge_snapshot` geprueft (N4, Mutationsmessung `rust/store`). Jeder Test steht fuer
//! Mutanten, die die Tests der Crate und `-p api` ueberlebt haben; die Erwartungen stammen aus
//! `store.append_event` (Python, echte Bindung): Fehlerklasse, abgeleitetes Event, abgelegter
//! Schreiber. Die Uhr ist dabei fest (`store._now`), abgeleitete Events vergleicht der Test
//! deshalb nach Feld, Wert, Schreiber und `signal_2`, nie nach Kennung.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{
    Achsenwert, Feldtyp, Feldzustand, Herkunft, HerkunftVektor, PruefTiefe, PyWert, Schreiber,
    Signal2, Zustand,
};
use serde::Deserialize;
use serde_json::{json, Value};
use store::{
    Abweisung, AbweisungRoh, BindungNachschlag, EricBefundEingabe, EricKlasse, EventId,
    EventIdFehler, Katalog, NeuesEvent, NeuesEventRoh, Signal, Store, Veranlagungsjahr,
};

const TS: &str = "2026-01-01T00:00:00+00:00";

fn bindungen() -> &'static Vec<Bindung> {
    static BINDUNGEN: OnceLock<Vec<Bindung>> = OnceLock::new();
    BINDUNGEN.get_or_init(|| {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn echte_karte() -> HashMap<String, &'static Bindung> {
    store::baue_nachschlag(bindungen())
}

fn herkunft(achse: &str) -> Herkunft {
    Herkunft {
        herkunft: Achsenwert::new(achse).unwrap(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: Achsenwert::new("nutzer").unwrap(),
    }
}

/// Ein Mensch bestaetigt (`julius`, `signal_2` = `klick`).
fn bestaetigt(feld: &str, wert: &Value) -> NeuesEvent {
    NeuesEvent {
        feld_id: feld.to_owned(),
        wert: wert.clone().into(),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("klick").unwrap(),
        },
        herkunft: herkunft("mensch"),
        schreiber: Schreiber::Mensch("julius".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: Some(TS.to_owned()),
    }
}

/// Ein Vorschlags-Schreiber legt vorlaeufig ab.
fn vorlaeufig(feld: &str, wert: &Value, schreiber: Schreiber, achse: &str) -> NeuesEvent {
    NeuesEvent {
        feldzustand: Feldzustand::Vorlaeufig,
        herkunft: herkunft(achse),
        schreiber,
        ..bestaetigt(feld, wert)
    }
}

fn leerer_store(vz: i64) -> Store {
    Store::leer(vz, None)
}

/// Die aktiven Events eines Stores als `(feld_id, wert)`, nach Feld sortiert.
fn aktive(s: &Store) -> Vec<(String, PyWert)> {
    let mut v: Vec<(String, PyWert)> = s
        .aktive()
        .map(|(f, e)| (f.to_owned(), e.wert.clone()))
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn hat(s: &Store, feld: &str) -> bool {
    s.aktives(feld).is_some()
}

fn signal_2(s: &Store, feld: &str) -> Option<String> {
    s.aktives(feld)
        .and_then(|e| e.signal.as_ref())
        .and_then(|sig| sig.signal_2.clone())
}

/// Die Herkunft des aktiven Events als JSON (`herkunft`, `pruef_tiefe`, `haftung`).
fn herkunft_json(s: &Store, feld: &str) -> Value {
    serde_json::to_value(&s.aktives(feld).unwrap().herkunft).unwrap()
}

/// Herkunft jedes abgeleiteten Events: er hat die Zahl gesagt, nicht diesen Satz (`store.py`).
fn berechnet_json() -> Value {
    json!({"herkunft": "berechnet", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"})
}

fn anhaengen(s: &mut Store, neu: &NeuesEvent) -> Result<EventId, Abweisung> {
    let karte = echte_karte();
    let katalog = Katalog::aus_bindungen(bindungen().iter());
    s.append(neu, Some(&katalog), BindungNachschlag::neu(&karte))
}

fn roh(feld: &str, wert: &Value, zustand: Zustand, schreiber: &str, achse: &str) -> NeuesEventRoh {
    NeuesEventRoh {
        feld_id: feld.to_owned(),
        wert: wert.clone().into(),
        zustand,
        herkunft: HerkunftVektor::Voll(herkunft(achse)),
        schreiber: schreiber.to_owned(),
        signal: Signal {
            signal_1: Some(None),
            signal_2: (zustand == Zustand::Bestaetigt).then(|| "klick".to_owned()),
            signal_2_fehlt: false,
        },
        signal_2_fremd: None,
        ersetzt: None,
        ts: Some(TS.to_owned()),
    }
}

fn roh_bestaetigt(feld: &str, wert: &Value) -> NeuesEventRoh {
    roh(feld, wert, Zustand::Bestaetigt, "julius", "mensch")
}

fn roh_anhaengen(
    s: &mut Store,
    neu: &NeuesEventRoh,
    katalog: Option<&Katalog>,
) -> Result<EventId, AbweisungRoh> {
    let karte = echte_karte();
    s.append_roh(neu, katalog, BindungNachschlag::neu(&karte))
}

// ---------------------------------------------------------------------------------------------
// Veranlagungsjahr: jede Quelle des Deserializers
// ---------------------------------------------------------------------------------------------

/// `serde_json::from_str` liefert nur `visit_i128`; `from_value` liefert `visit_i64`/`visit_u64`,
/// ein anderer Deserializer `visit_u128`. Ein negatives Jahr behaelt sein Vorzeichen, ein Jahr
/// ueber `i128::MAX` saettigt dort und nicht bei 0.
#[test]
fn veranlagungsjahr_liest_negative_und_grosse_jahre_aus_jeder_quelle() {
    use serde::de::IntoDeserializer;
    assert_eq!(
        serde_json::from_value::<Veranlagungsjahr>(json!(-5)).unwrap(),
        Veranlagungsjahr(-5)
    );
    assert_eq!(
        serde_json::from_value::<Veranlagungsjahr>(json!(u64::MAX)).unwrap(),
        Veranlagungsjahr(i128::from(u64::MAX))
    );
    assert_eq!(
        serde_json::from_str::<Veranlagungsjahr>("-5").unwrap(),
        Veranlagungsjahr(-5)
    );
    let gross: serde::de::value::U128Deserializer<serde::de::value::Error> =
        u128::MAX.into_deserializer();
    let jahr = Veranlagungsjahr::deserialize(gross).unwrap();
    assert_eq!(jahr, Veranlagungsjahr(i128::MAX));
    assert_eq!(jahr.als_i64_saettigend(), i64::MAX);
}

// ---------------------------------------------------------------------------------------------
// append: Auflage A, Reihenfolge der Pruefungen
// ---------------------------------------------------------------------------------------------

/// Python `append_event`: ein `llm:`-Schreiber darf nicht bestaetigen (A). Auch am typisierten
/// `append`, nicht nur an `append_roh`.
#[test]
fn auflage_a_weist_einen_bestaetigenden_llm_schreiber_am_append_ab() {
    let mut neu = bestaetigt("bruttoarbeitslohn", &json!(100));
    neu.schreiber = Schreiber::Llm("chat".to_owned());
    neu.herkunft = herkunft("llm_vorschlag");
    let fehler = anhaengen(&mut leerer_store(2025), &neu).unwrap_err();
    assert!(matches!(fehler, Abweisung::AuflageA { .. }), "{fehler:?}");
}

/// Python prueft A vor jedem Wert (`K4`): ein bestaetigender `llm:`-Schreiber mit einem Wert, den
/// die Akte nicht darstellen kann, scheitert an A, nicht am Wert.
#[test]
fn auflage_a_kommt_vor_der_wertpruefung() {
    let mut neu = bestaetigt("ep_unbekannt", &json!(1));
    neu.wert = PyWert::Gleit(f64::NAN);
    neu.schreiber = Schreiber::Llm("chat".to_owned());
    let fehler = anhaengen(&mut leerer_store(2025), &neu).unwrap_err();
    assert!(matches!(fehler, Abweisung::AuflageA { .. }), "{fehler:?}");
}

/// Python `K3`: das Feld hat ein aktives Event, der zweite Wert hat den falschen Typ und kein
/// `ersetzt`. Die Typpruefung (T) kommt vor Auflage B.
#[test]
fn typpruefung_kommt_vor_auflage_b() {
    let mut s = leerer_store(2025);
    anhaengen(&mut s, &bestaetigt("bruttoarbeitslohn", &json!(100))).unwrap();
    let fehler = anhaengen(&mut s, &bestaetigt("bruttoarbeitslohn", &json!("abc"))).unwrap_err();
    assert!(
        matches!(fehler, Abweisung::TypInkonform { .. }),
        "{fehler:?}"
    );
    // Mit passendem Typ greift B.
    let fehler = anhaengen(&mut s, &bestaetigt("bruttoarbeitslohn", &json!(200))).unwrap_err();
    assert!(
        matches!(fehler, Abweisung::AktivesEventVorhanden { .. }),
        "{fehler:?}"
    );
}

/// Der Ersetzt-Guard trifft nur `llm:`, `import:beleg` und `import:kontoauszug`. Eine
/// Vorjahres-Uebernahme darf ein bestaetigtes Event ersetzen (Python `H2`), ein Beleg nicht
/// (`H3`).
#[test]
fn ersetzt_guard_laesst_das_vorjahr_zu_und_sperrt_den_beleg() {
    let mut s = leerer_store(2025);
    let erste = anhaengen(&mut s, &bestaetigt("bruttoarbeitslohn", &json!(100))).unwrap();
    let mut beleg = vorlaeufig(
        "bruttoarbeitslohn",
        &json!(150),
        Schreiber::ImportBeleg,
        "beleg_import",
    );
    beleg.ersetzt = Some(erste);
    let fehler = anhaengen(&mut s, &beleg).unwrap_err();
    assert!(
        matches!(fehler, Abweisung::AuflageAErsetztGuard { .. }),
        "{fehler:?}"
    );
    let mut vorjahr = vorlaeufig(
        "bruttoarbeitslohn",
        &json!(230),
        Schreiber::ImportVorjahr,
        "vorjahr",
    );
    vorjahr.ersetzt = Some(erste);
    anhaengen(&mut s, &vorjahr).unwrap();
    assert_eq!(
        s.aktives("bruttoarbeitslohn").unwrap().wert,
        PyWert::Ganz(230)
    );
}

// ---------------------------------------------------------------------------------------------
// append: Magnitude (F2) und nicht darstellbarer Wert
// ---------------------------------------------------------------------------------------------

/// Python `I1`-`I4`: ein Vorschlags-Schreiber mit freigegebenem Feld und einem Betrag ab 10^10
/// (`abs`) scheitert an F2; darunter geht er durch, ein Mensch bleibt unberuehrt.
#[test]
fn magnitude_greift_am_append_nur_bei_vorschlaegen() {
    let beleg = |wert: i64| {
        vorlaeufig(
            "bruttoarbeitslohn",
            &json!(wert),
            Schreiber::ImportBeleg,
            "beleg_import",
        )
    };
    for zu_gross in [12_000_000_000, -12_000_000_000, 10_000_000_000] {
        let fehler = anhaengen(&mut leerer_store(2025), &beleg(zu_gross)).unwrap_err();
        assert!(
            matches!(fehler, Abweisung::Magnitude { .. }),
            "{zu_gross}: {fehler:?}"
        );
    }
    anhaengen(&mut leerer_store(2025), &beleg(9_999_999_999)).unwrap();
    anhaengen(
        &mut leerer_store(2025),
        &bestaetigt("bruttoarbeitslohn", &json!(12_000_000_000_i64)),
    )
    .unwrap();
}

/// K2-Auflage 3: der Wert muss nach JSON gehen, und diese Pruefung kommt vor Typ und Katalog. Die
/// Fehlerklasse haengt nicht davon ab, welche Auflage den Wert zuerst saehe.
#[test]
fn nicht_darstellbarer_wert_hat_vorrang_vor_typ_und_katalog() {
    let mut neu = bestaetigt("bruttoarbeitslohn", &json!(1));
    neu.wert = PyWert::Gleit(f64::NAN);
    let fehler = anhaengen(&mut leerer_store(2025), &neu).unwrap_err();
    assert!(
        matches!(fehler, Abweisung::WertNichtDarstellbar { .. }),
        "Typ: {fehler:?}"
    );
    // Ein Beleg fuer ein Feld, das er nicht vorschlagen darf: sonst kaeme der Katalog zuerst.
    let mut beleg = vorlaeufig(
        "ep_arbeitstage",
        &json!(1),
        Schreiber::ImportBeleg,
        "beleg_import",
    );
    beleg.wert = PyWert::Gleit(f64::NAN);
    let fehler = anhaengen(&mut leerer_store(2025), &beleg).unwrap_err();
    assert!(
        matches!(fehler, Abweisung::WertNichtDarstellbar { .. }),
        "Katalog: {fehler:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// beweist (`_leite_ab`)
// ---------------------------------------------------------------------------------------------

/// Python `A`/`A2`/`A3`: 2 und 1 Kinder beweisen `kein_kind = false` mit dem Text
/// `beweist@<feld>=<wert>`; 0 beweist nichts (`ab: 1`).
#[test]
fn beweist_schreibt_das_ziel_mit_dem_text_von_python_und_erst_ab_eins() {
    for (anzahl, soll) in [(2, true), (1, true), (0, false)] {
        let mut s = leerer_store(2025);
        anhaengen(&mut s, &bestaetigt("fam_anzahl_kinder", &json!(anzahl))).unwrap();
        assert_eq!(hat(&s, "kein_kind"), soll, "{anzahl}");
        if soll {
            let e = s.aktives("kein_kind").unwrap();
            assert_eq!(e.wert, PyWert::Bool(false));
            assert_eq!(e.schreiber.to_string(), "abgeleitet:beweist");
            assert_eq!(herkunft_json(&s, "kein_kind"), berechnet_json());
            assert_eq!(
                signal_2(&s, "kein_kind"),
                Some(format!("beweist@fam_anzahl_kinder={anzahl}"))
            );
        }
    }
}

/// Python `B`: `beweist` gilt fuer das Feld selbst (`bindung.get`), nicht fuer seine
/// Instanz-Schreibweise `fam_anzahl_kinder__2`.
#[test]
fn beweist_gilt_nicht_fuer_instanzfelder() {
    let mut s = leerer_store(2025);
    anhaengen(&mut s, &bestaetigt("fam_anzahl_kinder__2", &json!(2))).unwrap();
    assert_eq!(aktive(&s).len(), 1, "{:?}", aktive(&s));
}

/// Ohne `ab` gilt Pythons `regel.get("ab", 1)`: bei 0 nichts, bei 1 ein Beweis. Die echte
/// Bindung nennt `ab` immer; die Standardschwelle erreicht nur eine Bindung ohne.
#[test]
fn beweist_ohne_ab_nimmt_eins_als_schwelle() {
    let mut eigene: Vec<Bindung> = bindungen().clone();
    for b in &mut eigene {
        if b.feld_id == "fam_anzahl_kinder" {
            b.beweist.as_mut().unwrap().ab = None;
        }
    }
    let karte = store::baue_nachschlag(&eigene);
    for (anzahl, soll) in [(0, false), (1, true), (2, true)] {
        let mut s = leerer_store(2025);
        s.append(
            &bestaetigt("fam_anzahl_kinder", &json!(anzahl)),
            None,
            BindungNachschlag::neu(&karte),
        )
        .unwrap();
        assert_eq!(hat(&s, "kein_kind"), soll, "{anzahl}");
    }
}

/// Python: `isinstance(wert, bool) or not isinstance(wert, (int, float))` beweist nichts. Die
/// echte Bindung haelt Text und Wahrheitswerte schon am Typ auf; hier traegt die Quelle den
/// Typ `text` bzw. `bool`, damit der Wert die Ableitung erreicht.
#[test]
fn beweist_liest_nur_zahlen() {
    for (typ, wert) in [
        (Feldtyp::Text, json!("viele")),
        (Feldtyp::Bool, json!(true)),
    ] {
        let mut eigene: Vec<Bindung> = bindungen().clone();
        for b in &mut eigene {
            if b.feld_id == "fam_anzahl_kinder" {
                b.typ = typ;
                b.bereich = None;
            }
        }
        let karte = store::baue_nachschlag(&eigene);
        let mut s = leerer_store(2025);
        s.append(
            &bestaetigt("fam_anzahl_kinder", &wert),
            None,
            BindungNachschlag::neu(&karte),
        )
        .unwrap();
        assert!(hat(&s, "fam_anzahl_kinder"), "{wert}");
        assert!(!hat(&s, "kein_kind"), "{wert}");
    }
}

// ---------------------------------------------------------------------------------------------
// ableitung (`_rechne_ab`)
// ---------------------------------------------------------------------------------------------

const KIND_GEB: &str = "kind_geburtsdatum";
const KIND_ZEITRAUM: &str = "kind_betreuung_haushaltszugehoerigkeit_zeitraum";
const KIND_UNTER_14: &str = "kind_unter_14_haushaltszugehoerig";

/// Python `D1`-`D4`, `K6`: das Kind ist am Jahresende des Veranlagungsjahres unter 14, wenn
/// `vz - geburtsjahr < 14`. Geboren 2011 ist 2025 die Grenze (kein Beweis), 2012 ein Beweis, in
/// beiden Reihenfolgen der Angaben; in der Akte 2024 ist 2011 ein Beweis. Ein vorlaeufiger
/// Zeitraum loest nichts aus.
#[test]
fn ableitung_rechnet_mit_dem_veranlagungsjahr_der_akte() {
    let faelle = [
        (2025, "15.06.2011", false),
        (2025, "15.06.2012", true),
        (2024, "15.06.2011", true),
    ];
    for (vz, geburt, soll) in faelle {
        for geburtsdatum_zuerst in [true, false] {
            let mut s = leerer_store(vz);
            let g = bestaetigt(KIND_GEB, &json!(geburt));
            let z = bestaetigt(KIND_ZEITRAUM, &json!("01.03-31.10"));
            let (erste, zweite) = if geburtsdatum_zuerst { (g, z) } else { (z, g) };
            anhaengen(&mut s, &erste).unwrap();
            anhaengen(&mut s, &zweite).unwrap();
            assert_eq!(
                hat(&s, KIND_UNTER_14),
                soll,
                "{vz} {geburt} {geburtsdatum_zuerst}"
            );
            if soll {
                assert_eq!(s.aktives(KIND_UNTER_14).unwrap().wert, PyWert::Bool(true));
                assert_eq!(herkunft_json(&s, KIND_UNTER_14), berechnet_json());
                assert_eq!(
                    signal_2(&s, KIND_UNTER_14),
                    Some(format!("ableitung@{KIND_GEB}"))
                );
            }
        }
    }
    // D4: der Zeitraum ist nur vorlaeufig -- die Haushaltszugehoerigkeit steht nicht fest.
    let mut s = leerer_store(2025);
    let mut z = bestaetigt(KIND_ZEITRAUM, &json!("01.03-31.10"));
    z.feldzustand = Feldzustand::Vorlaeufig;
    anhaengen(&mut s, &z).unwrap();
    anhaengen(&mut s, &bestaetigt(KIND_GEB, &json!("15.06.2012"))).unwrap();
    assert!(!hat(&s, KIND_UNTER_14));
}

/// Python `D5`: je Instanz; der Quellname im `signal_2` traegt den Suffix. Der Ausloeser ist der
/// Zeitraum, das `signal_2` nennt trotzdem die Quelle, nicht den Ausloeser.
#[test]
fn ableitung_je_instanz_nennt_die_quelle_im_signal_2() {
    for geburtsdatum_zuerst in [true, false] {
        let mut s = leerer_store(2025);
        let g = bestaetigt("kind_geburtsdatum__2", &json!("15.06.2012"));
        let z = bestaetigt(
            "kind_betreuung_haushaltszugehoerigkeit_zeitraum__2",
            &json!("01.03-31.10"),
        );
        let (erste, zweite) = if geburtsdatum_zuerst { (g, z) } else { (z, g) };
        anhaengen(&mut s, &erste).unwrap();
        anhaengen(&mut s, &zweite).unwrap();
        let ziel = format!("{KIND_UNTER_14}__2");
        assert_eq!(s.aktives(&ziel).unwrap().wert, PyWert::Bool(true));
        assert_eq!(
            signal_2(&s, &ziel),
            Some("ableitung@kind_geburtsdatum__2".to_owned())
        );
        assert!(!hat(&s, KIND_UNTER_14));
    }
}

/// Python `D6`/`D7`: ein Feldname mit abschliessendem Zeilenumbruch ist eine Instanz (Pythons `$`),
/// aber er ist nicht gleich dem Namen, den die Ableitung bildet -- sie loest nicht aus.
#[test]
fn ableitung_loest_bei_einem_feldnamen_mit_zeilenumbruch_nicht_aus() {
    let mut s = leerer_store(2025);
    anhaengen(
        &mut s,
        &bestaetigt(
            "kind_betreuung_haushaltszugehoerigkeit_zeitraum__2",
            &json!("01.03-31.10"),
        ),
    )
    .unwrap();
    anhaengen(
        &mut s,
        &bestaetigt("kind_geburtsdatum__2\n", &json!("15.06.2012")),
    )
    .unwrap();
    assert_eq!(aktive(&s).len(), 2, "{:?}", aktive(&s));
    let mut s = leerer_store(2025);
    anhaengen(
        &mut s,
        &bestaetigt("kind_geburtsdatum__2", &json!("15.06.2012")),
    )
    .unwrap();
    anhaengen(
        &mut s,
        &bestaetigt(
            "kind_betreuung_haushaltszugehoerigkeit_zeitraum__2\n",
            &json!("01.03-31.10"),
        ),
    )
    .unwrap();
    assert_eq!(aktive(&s).len(), 2, "{:?}", aktive(&s));
    // D8/D9: tragen BEIDE Felder den Zeilenumbruch, stuenden Quelle und `und_feld` unter dem
    // Namen mit Umbruch in der Akte -- der Suffix bleibt trotzdem `__2`, es entsteht nichts.
    for geburtsdatum_zuerst in [true, false] {
        let g = bestaetigt("kind_geburtsdatum__2\n", &json!("15.06.2012"));
        let z = bestaetigt(
            "kind_betreuung_haushaltszugehoerigkeit_zeitraum__2\n",
            &json!("01.03-31.10"),
        );
        let (erste, zweite) = if geburtsdatum_zuerst { (g, z) } else { (z, g) };
        let mut s = leerer_store(2025);
        anhaengen(&mut s, &erste).unwrap();
        anhaengen(&mut s, &zweite).unwrap();
        assert_eq!(aktive(&s).len(), 2, "{:?}", aktive(&s));
    }
}

/// Python `wert in (None, "", False)`: ein leeres `und_feld` loest nicht aus. Die Bindung zwingt
/// ihm den Typ `text` mit Muster auf, `null` und `false` erreichen die Ableitung nur ueber ein
/// Feld ohne Bindung -- hier fehlt der Eintrag der Karte.
#[test]
fn ableitung_ignoriert_ein_leeres_und_feld() {
    let mut karte = echte_karte();
    karte.remove(KIND_ZEITRAUM);
    for (leer, soll) in [
        (PyWert::Null, false),
        (PyWert::Bool(false), false),
        (PyWert::Text(String::new()), false),
        (PyWert::Bool(true), true),
        (PyWert::Text("x".to_owned()), true),
    ] {
        let mut s = leerer_store(2025);
        let mut z = bestaetigt(KIND_ZEITRAUM, &json!(0));
        z.wert = leer.clone();
        s.append(&z, None, BindungNachschlag::neu(&karte)).unwrap();
        s.append(
            &bestaetigt(KIND_GEB, &json!("15.06.2012")),
            None,
            BindungNachschlag::neu(&karte),
        )
        .unwrap();
        assert_eq!(hat(&s, KIND_UNTER_14), soll, "{leer:?}");
    }
}

/// Eine leere `instanz_gruppe` ist keine Gruppe: die Ableitung haengt den Suffix des
/// Ausloesers nicht an. Die echte Bindung nennt nie eine leere Gruppe.
#[test]
fn ableitung_mit_leerer_instanz_gruppe_hat_keine_instanzen() {
    let mut eigene: Vec<Bindung> = bindungen().clone();
    for b in &mut eigene {
        if b.feld_id == KIND_UNTER_14 {
            b.instanz_gruppe = Some(String::new());
        }
    }
    let karte = store::baue_nachschlag(&eigene);
    let mut s = leerer_store(2025);
    for neu in [
        bestaetigt(
            "kind_betreuung_haushaltszugehoerigkeit_zeitraum__2",
            &json!("01.03-31.10"),
        ),
        bestaetigt("kind_geburtsdatum__2", &json!("15.06.2012")),
    ] {
        s.append(&neu, None, BindungNachschlag::neu(&karte))
            .unwrap();
    }
    // Ohne Gruppe waere `__2` fuer `aus` und `und_feld` nie gleich dem Ausloeser.
    assert!(!hat(&s, &format!("{KIND_UNTER_14}__2")));
    assert!(!hat(&s, KIND_UNTER_14));
}

/// Python `G1`-`G3`, `K5`: 64 Jahre am Jahresbeginn. Wer am 1.1.1961 geboren ist, hat in der Akte
/// 2025 das 64. Lebensjahr vor Beginn des Jahres vollendet, in der Akte 2024 nicht; ein Tag
/// spaeter nie.
#[test]
fn ableitung_alter_am_jahresbeginn_haengt_am_veranlagungsjahr() {
    for (vz, geburt, soll) in [
        (2025, "01.01.1961", true),
        (2025, "02.01.1961", false),
        (2025, "31.12.1960", true),
        (2024, "01.01.1961", false),
    ] {
        let mut s = leerer_store(vz);
        anhaengen(
            &mut s,
            &bestaetigt("stammdaten_geburtsdatum", &json!(geburt)),
        )
        .unwrap();
        assert_eq!(
            s.aktives("rentner_alter_64_erfuellt").unwrap().wert,
            PyWert::Bool(soll),
            "{vz} {geburt}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// append_roh
// ---------------------------------------------------------------------------------------------

/// Die Ableitungen laufen auch ueber den Rohweg (`append_event` ist EIN Schreibpfad): beweist,
/// Ableitung und das Veranlagungsjahr der Akte.
#[test]
fn append_roh_leitet_ab_wie_append() {
    let mut s = leerer_store(2025);
    roh_anhaengen(
        &mut s,
        &roh_bestaetigt("fam_anzahl_kinder", &json!(2)),
        None,
    )
    .unwrap();
    assert_eq!(s.aktives("kein_kind").unwrap().wert, PyWert::Bool(false));
    for (vz, geburt, soll) in [(2025, "01.01.1961", true), (2024, "01.01.1961", false)] {
        let mut s = leerer_store(vz);
        roh_anhaengen(
            &mut s,
            &roh_bestaetigt("stammdaten_geburtsdatum", &json!(geburt)),
            None,
        )
        .unwrap();
        assert_eq!(
            s.aktives("rentner_alter_64_erfuellt").unwrap().wert,
            PyWert::Bool(soll),
            "{vz}"
        );
        assert_eq!(s.aktives("geburtsjahr").unwrap().wert, PyWert::Ganz(1961));
    }
}

/// Python `K1`: ein `import:beleg2` ist ein Beleg-Schreiber (Praefix); der Katalog gilt ihm, und
/// die Akte legt den Namen ab, wie er kam.
#[test]
fn append_roh_prueft_den_katalog_nach_praefix_und_legt_den_namen_ab_wie_er_kam() {
    let beleg2 = roh(
        "bruttoarbeitslohn",
        &json!(100),
        Zustand::Vorlaeufig,
        "import:beleg2",
        "beleg_import",
    );
    // Ohne Katalog scheitert ein Vorschlags-Schreiber (K1).
    let fehler = roh_anhaengen(&mut leerer_store(2025), &beleg2, None).unwrap_err();
    assert!(
        matches!(fehler, AbweisungRoh::Store(Abweisung::KatalogFehlt { .. })),
        "{fehler:?}"
    );
    // Mit Katalog geht das freigegebene Feld durch, und der Schreiber steht unveraendert da.
    let katalog = Katalog::aus_bindungen(bindungen().iter());
    let mut s = leerer_store(2025);
    roh_anhaengen(&mut s, &beleg2, Some(&katalog)).unwrap();
    let e = serde_json::to_value(&s.events()[0]).unwrap();
    assert_eq!(e["schreiber"], "import:beleg2");
}

/// Python `K2`: Typ (T) kommt vor dem Typzwang auf `signal_2`.
#[test]
fn append_roh_prueft_den_typ_vor_dem_signal_2() {
    let mut neu = roh_bestaetigt("bruttoarbeitslohn", &json!("abc"));
    neu.signal_2_fremd = Some("int".to_owned());
    let fehler = roh_anhaengen(&mut leerer_store(2025), &neu, None).unwrap_err();
    assert!(
        matches!(fehler, AbweisungRoh::Store(Abweisung::TypInkonform { .. })),
        "{fehler:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// erzeuge_snapshot
// ---------------------------------------------------------------------------------------------

/// Python `S`: der Snapshot traegt den ERiC-Befund Feld fuer Feld, gebunden an seine eigene
/// Kennung (nicht an `bis_event`).
#[test]
fn snapshot_traegt_den_befund_unveraendert() {
    for gekappt in [true, false] {
        let mut s = leerer_store(2025);
        anhaengen(&mut s, &bestaetigt("bruttoarbeitslohn", &json!(100))).unwrap();
        let befund = EricBefundEingabe {
            rc: 3,
            klasse: EricKlasse::PlausibilitaetFehler,
            gekappt_verdacht: gekappt,
            fehler_anzahl: Some(2),
        };
        let sid = s
            .erzeuge_snapshot(None, Some(TS.to_owned()), Some(befund))
            .unwrap();
        let snap = &s.snapshots()[0];
        let b = snap.eric_befund.as_ref().unwrap();
        assert_eq!(b.rc, 3);
        assert_eq!(b.klasse, EricKlasse::PlausibilitaetFehler);
        assert_eq!(b.gekappt_verdacht, gekappt);
        assert_eq!(b.fehler_anzahl, Some(2));
        assert_eq!(b.gebunden_an, sid);
        assert_ne!(b.gebunden_an, snap.bis_event);
    }
}

// ---------------------------------------------------------------------------------------------
// Wortlaut der Abweisungen (Teil C: abweisung.rs)
// ---------------------------------------------------------------------------------------------

fn text<T: std::fmt::Debug>(r: Result<T, Abweisung>) -> String {
    r.unwrap_err().to_string()
}

/// Ein Vorschlags-Schreiber legt den Wert falsch (`bestaetigt`) ab: A nennt den Praefix des
/// Schreibers und den Satz je Typ. Wortlaut aus `store.append_event` (Python, echter Katalog).
#[test]
fn auflage_a_traegt_den_wortlaut_von_python_je_vorschlags_schreiber() {
    let faelle = [
        (
            Schreiber::Llm("chat".to_owned()),
            "llm_vorschlag",
            "fail-closed (A): llm:-Schreiber muss herkunft=llm_vorschlag, zustand=vorlaeufig, \
             signal_2=null tragen — kein Bestätigen durch die KI.",
        ),
        (
            Schreiber::ImportBeleg,
            "beleg_import",
            "fail-closed (A): import:beleg-Schreiber muss herkunft=beleg_import, zustand=vorlaeufig, \
             signal_2=null tragen — ein Beleg-Import bestätigt nie direkt.",
        ),
        (
            Schreiber::ImportVorjahr,
            "vorjahr",
            "fail-closed (A): import:vorjahr-Schreiber muss herkunft=vorjahr, zustand=vorlaeufig, \
             signal_2=null tragen — eine Vorjahres-Übernahme bestätigt nie direkt.",
        ),
        (
            Schreiber::ImportKontoauszug,
            "kontoauszug",
            "fail-closed (A): import:kontoauszug-Schreiber muss herkunft=kontoauszug, \
             zustand=vorlaeufig, signal_2=null tragen — eine Kontoauszug-Klassifikation bestätigt \
             nie direkt.",
        ),
        (
            Schreiber::Berechnet("maps".to_owned()),
            "berechnet",
            "fail-closed (A): berechnet:-Schreiber muss herkunft=berechnet, zustand=vorlaeufig, \
             signal_2=null tragen — ein berechneter/abgeleiteter Vorschlag bestätigt nie direkt.",
        ),
    ];
    for (schreiber, achse, soll) in faelle {
        let mut neu = bestaetigt("bruttoarbeitslohn", &json!(1));
        neu.schreiber = schreiber;
        neu.herkunft = herkunft(achse);
        assert_eq!(text(anhaengen(&mut leerer_store(2025), &neu)), soll);
    }
}

/// Der Ersetzt-Guard nennt den ganzen Schreiber (`llm:chat`), nicht den Praefix.
#[test]
fn ersetzt_guard_traegt_den_wortlaut_von_python() {
    for (schreiber, achse, name) in [
        (
            Schreiber::Llm("chat".to_owned()),
            "llm_vorschlag",
            "llm:chat",
        ),
        (Schreiber::ImportBeleg, "beleg_import", "import:beleg"),
        (
            Schreiber::ImportKontoauszug,
            "kontoauszug",
            "import:kontoauszug",
        ),
    ] {
        let mut s = leerer_store(2025);
        let erste = anhaengen(&mut s, &bestaetigt("bruttoarbeitslohn", &json!(5))).unwrap();
        let mut neu = vorlaeufig("bruttoarbeitslohn", &json!(6), schreiber, achse);
        neu.ersetzt = Some(erste);
        assert_eq!(
            text(anhaengen(&mut s, &neu)),
            format!(
                "fail-closed (A): {name} darf kein ersetzt tragen — ein Vorschlag ersetzt nie \
                 einen bestätigten Wert; die Übernahme läuft über /event mit menschlichem signal_2."
            )
        );
    }
}

/// Katalog (K1), Magnitude (F2), Typ (Steuerzeichen), Format und Auflage B: Wortlaut aus Python, ausser Format:
/// Es nennt den Wert nicht (Abweichung Nr. 24).
#[test]
fn katalog_magnitude_typ_format_und_b_tragen_den_wortlaut_von_python() {
    let karte = echte_karte();
    let ohne_katalog = leerer_store(2025)
        .append(
            &vorlaeufig(
                "bruttoarbeitslohn",
                &json!(1),
                Schreiber::Llm("chat".to_owned()),
                "llm_vorschlag",
            ),
            None,
            BindungNachschlag::neu(&karte),
        )
        .unwrap_err();
    assert_eq!(
        ohne_katalog.to_string(),
        "fail-closed (Katalog): Vorschlags-Schreiber llm:chat braucht katalog=lade_katalog(bindung)."
    );
    let beleg = |feld: &str, wert: PyWert| {
        let mut neu = vorlaeufig(feld, &json!(1), Schreiber::ImportBeleg, "beleg_import");
        neu.wert = wert;
        neu
    };
    assert_eq!(
        text(anhaengen(
            &mut leerer_store(2025),
            &beleg("ep_arbeitstage", PyWert::Ganz(1))
        )),
        "fail-closed (Katalog): import:beleg darf ep_arbeitstage nicht vorschlagen (human-only \
         oder nicht für Typ 'beleg' freigegeben)."
    );
    // F2: dieselbe Meldung fuer jede Zahlform, der Wert als `repr` wie in Python.
    for (wert, zahl) in [
        (PyWert::Ganz(12_000_000_000), "12000000000"),
        (
            PyWert::GrossGanz(18_000_000_000_000_000_000),
            "18000000000000000000",
        ),
        (PyWert::Gleit(12_000_000_000.0), "12000000000.0"),
        (PyWert::Text("12000000000".to_owned()), "'12000000000'"),
    ] {
        assert_eq!(
            text(anhaengen(
                &mut leerer_store(2025),
                &beleg("bruttoarbeitslohn", wert)
            )),
            format!(
                "fail-closed (F2/Magnitude): bruttoarbeitslohn={zahl} von import:beleg — \
                 vermuteter Einheiten-/Skalierungsfehler (EUR statt Cent)."
            )
        );
    }
    // T: ein Steuerzeichen bleibt aus der Meldung.
    assert_eq!(
        text(anhaengen(
            &mut leerer_store(2025),
            &bestaetigt("ep_ziel_adresse", &json!("a\u{1}b"))
        )),
        "fail-closed (Typ): ep_ziel_adresse=[Steuerzeichen im Text, Wert nicht geloggt] passt \
         nicht zum Bindungstyp 'text' — der Ring läse das sonst still als 0 (Stille-Null-Klasse)."
    );
    // F: das Muster steht unveraendert in der Meldung, der Wert nicht (Abweichung Nr. 24).
    assert_eq!(
        text(anhaengen(
            &mut leerer_store(2025),
            &bestaetigt(KIND_ZEITRAUM, &json!("abc"))
        )),
        "fail-closed (Format): kind_betreuung_haushaltszugehoerigkeit_zeitraum passt nicht \
         zum Muster '^(?:(0[1-9]|[1-2][0-9]|3[0-1])\\.(10|11|12|01|02|03|04|05|06|07|08|09)-\
         (0[1-9]|[1-2][0-9]|3[0-1])\\.(10|11|12|01|02|03|04|05|06|07|08|09))$' der Bindung — ein \
         formal falscher Wert wird spätestens beim Finanzamt abgelehnt."
    );
}

/// Auflage T (Abweichung Nr. 30): der Typ-Text nennt Feld und Typ, nie den Wert. Python nennt ihn
/// (`<feld>=<repr> passt nicht ...`). Auch ein `{:?}` auf die Abweisung zeigt ihn nicht: die Variante traegt ihn nicht.
#[test]
fn typ_text_ohne_wert() {
    // (Feld, falscher Wert, Typ der Bindung, Schreibweise des Wertes in Pythons Text)
    let faelle = [
        ("bruttoarbeitslohn", json!("GEHEIM-123"), "cent", "GEHEIM-123"),
        ("bruttoarbeitslohn", json!(true), "cent", "True"),
        ("ep_arbeitstage", json!(1.5), "int", "1.5"),
        ("ep_ziel_adresse", json!(13_579_246_007_i64), "text", "13579246007"),
    ];
    for (feld, wert, typ, schreibweise) in faelle {
        let fehler = anhaengen(&mut leerer_store(2025), &bestaetigt(feld, &wert)).unwrap_err();
        assert!(matches!(fehler, Abweisung::TypInkonform { .. }), "{feld}: {fehler:?}");
        assert_eq!(
            fehler.to_string(),
            format!(
                "fail-closed (Typ): {feld} passt nicht zum Bindungstyp '{typ}' — der Ring läse das \
                 sonst still als 0 (Stille-Null-Klasse)."
            ),
            "{feld}"
        );
        assert!(
            !format!("{fehler:?}").contains(schreibweise),
            "{feld}: die Abweisung zeigt den Wert {schreibweise:?} im Debug-Text: {fehler:?}"
        );
    }
}

/// Auflage B: das `ersetzt`-Ziel fehlt, gehoert zu einem anderen Feld, ist schon ersetzt.
#[test]
fn ersetzt_ziel_traegt_den_wortlaut_von_python() {
    let mut s = leerer_store(2025);
    let andere = anhaengen(&mut s, &bestaetigt("ep_arbeitstage", &json!(5))).unwrap();
    let erste = anhaengen(&mut s, &bestaetigt("bruttoarbeitslohn", &json!(5))).unwrap();
    let ersetze = |wert: i64, ziel: EventId| {
        let mut neu = bestaetigt("bruttoarbeitslohn", &json!(wert));
        neu.ersetzt = Some(ziel);
        neu
    };
    let unbekannt = EventId::parse(&"a".repeat(64)).unwrap();
    assert_eq!(
        text(anhaengen(&mut s, &ersetze(6, unbekannt))),
        format!(
            "fail-closed (B): ersetzt-Ziel {} existiert nicht.",
            "a".repeat(64)
        )
    );
    assert_eq!(
        text(anhaengen(&mut s, &ersetze(6, andere))),
        "fail-closed (B): ersetzt-Ziel gehört zu anderem feld_id."
    );
    anhaengen(&mut s, &ersetze(6, erste)).unwrap();
    assert_eq!(
        text(anhaengen(&mut s, &ersetze(7, erste))),
        "fail-closed (B): ersetzt-Ziel ist bereits ersetzt."
    );
}

/// Ein Wert, der nicht nach JSON geht, ist eine Verschaerfung ohne Python-Entsprechung: der Text
/// nennt das Feld, dann `=`, dann den Grund.
#[test]
fn wert_nicht_darstellbar_nennt_feld_und_grund() {
    let mut neu = bestaetigt("bruttoarbeitslohn", &json!(1));
    neu.wert = PyWert::Gleit(f64::NAN);
    let meldung = text(anhaengen(&mut leerer_store(2025), &neu));
    assert!(
        meldung.starts_with("fail-closed (Wert): bruttoarbeitslohn=") && meldung.len() > 40,
        "{meldung}"
    );
}

/// Eine Kennung ist genau 64 Zeichen Hex, klein: 65 und 63 scheitern an der LAENGE (ein 65. Zeichen
/// fiele sonst erst beim Zerlegen als ungueltiges Hex auf).
#[test]
fn event_id_parse_nimmt_nur_64_zeichen() {
    assert!(EventId::parse(&"a".repeat(64)).is_ok());
    assert_eq!(
        EventId::parse(&"a".repeat(65)),
        Err(EventIdFehler::FalscheLaenge(65))
    );
    assert_eq!(
        EventId::parse(&"a".repeat(63)),
        Err(EventIdFehler::FalscheLaenge(63))
    );
}

// ---------------------------------------------------------------------------------------------
// Bindungen, die die echte Bindung nicht traegt (Teil C: Muster, Bereich, Datumsform)
// ---------------------------------------------------------------------------------------------

/// Eine Bindungskarte mit einer Aenderung an einem Feld; die Bindungen bleiben im Test.
fn mit_aenderung(feld: &str, aendere: impl Fn(&mut Bindung)) -> Vec<Bindung> {
    let mut eigene: Vec<Bindung> = bindungen().clone();
    for b in &mut eigene {
        if b.feld_id == feld {
            aendere(b);
        }
    }
    eigene
}

fn mit_karte(eigene: &[Bindung], neu: &NeuesEvent, s: &mut Store) -> Result<EventId, Abweisung> {
    let karte = store::baue_nachschlag(eigene);
    s.append(neu, None, BindungNachschlag::neu(&karte))
}

/// Ein `muster` gilt fuer den ganzen Wert (`re.fullmatch`), auch mit einer Alternative: `a|b`
/// nimmt `a` und `b`, aber nicht `ab` (Python `fullmatch`, gemessen).
#[test]
fn muster_mit_alternative_gilt_fuer_den_ganzen_wert() {
    let eigene = mit_aenderung(KIND_ZEITRAUM, |b| b.muster = Some("a|b".to_owned()));
    for (wert, geht) in [("a", true), ("b", true), ("ab", false), ("x", false)] {
        let fehler = mit_karte(
            &eigene,
            &bestaetigt(KIND_ZEITRAUM, &json!(wert)),
            &mut leerer_store(2025),
        );
        assert_eq!(fehler.is_ok(), geht, "{wert}: {fehler:?}");
        if let Err(e) = fehler {
            assert!(matches!(e, Abweisung::FormatInkonform { .. }), "{e:?}");
        }
    }
}

/// Ein Muster, das sich nicht uebersetzen laesst, laesst nichts durch (fail-closed; Python
/// bricht dort mit `re.error` ab, kein Orakel -- `PARITAET` in `passt_muster`).
#[test]
fn ungueltiges_muster_laesst_nichts_durch() {
    let eigene = mit_aenderung(KIND_ZEITRAUM, |b| b.muster = Some("(".to_owned()));
    let fehler = mit_karte(
        &eigene,
        &bestaetigt(KIND_ZEITRAUM, &json!("x")),
        &mut leerer_store(2025),
    )
    .unwrap_err();
    assert!(
        matches!(fehler, Abweisung::FormatInkonform { .. }),
        "{fehler:?}"
    );
}

/// Python `_ausserhalb_bereich`: auch ein `cent`-Ziel schreibt keinen Wert ausserhalb `bereich`.
/// `geburtsjahr` hat Typ `int`; hier traegt es `cent`, der Bereich 1900 bis 2010 bleibt.
#[test]
fn ableitung_schreibt_auch_bei_einem_cent_ziel_nichts_ausserhalb_des_bereichs() {
    let eigene = mit_aenderung("geburtsjahr", |b| b.typ = Feldtyp::Cent);
    for (geburt, soll) in [("01.01.1850", false), ("01.01.1961", true)] {
        let mut s = leerer_store(2025);
        mit_karte(
            &eigene,
            &bestaetigt("stammdaten_geburtsdatum", &json!(geburt)),
            &mut s,
        )
        .unwrap();
        assert_eq!(hat(&s, "geburtsjahr"), soll, "{geburt}");
    }
}

/// Python `_jahr`: ISO und deutsch, nach `strip()`, mit genau vier Ziffern fuers Jahr. Die echte
/// Bindung haelt Leerraum und ein Vorzeichen am `datum`-Typ auf; hier traegt die Quelle den Typ
/// `text` ohne Muster, damit der Wert die Ableitung erreicht.
#[test]
fn ableitung_liest_das_datum_wie_python() {
    let eigene = mit_aenderung("stammdaten_geburtsdatum", |b| {
        b.typ = Feldtyp::Text;
        b.muster = None;
    });
    for (wert, soll) in [
        (" 01.01.1961 ", true),
        ("01.01.1961", true),
        ("1961-01-01", true),
        ("01.01.+961", false),
        ("01.01.-961", false),
        ("+961-01-01", false),
    ] {
        let mut s = leerer_store(2025);
        mit_karte(
            &eigene,
            &bestaetigt("stammdaten_geburtsdatum", &json!(wert)),
            &mut s,
        )
        .unwrap();
        assert_eq!(hat(&s, "geburtsjahr"), soll, "{wert:?}");
        assert_eq!(hat(&s, "rentner_alter_64_erfuellt"), soll, "{wert:?}");
        if soll {
            assert_eq!(s.aktives("geburtsjahr").unwrap().wert, PyWert::Ganz(1961));
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Instanz-Aufloesung und Katalog (Teil C: nachschlag.rs, katalog.rs)
// ---------------------------------------------------------------------------------------------

/// `base__n` -> `base`, gleich mit `est_mapping.parse_instanz` (Python, `_INSTANZ_RE`) bei jeder
/// Eingabe der Tabelle: die LETZTE Trennung gilt, die Basis beginnt mit einem Kleinbuchstaben und
/// besteht aus Kleinbuchstaben, Ziffern und `_`, der Zaehler ist 2 oder mehr ohne fuehrende 0.
#[test]
fn instanz_basis_gleicht_der_enumeration_von_python() {
    let faelle: [(&str, Option<&str>); 23] = [
        ("a__b__2", Some("a__b")),
        ("a__2__3", Some("a__2")),
        ("__2", None),
        ("A_x__2", None),
        ("Ax__2", None),
        ("x1__2", Some("x1")),
        ("1x__2", None),
        ("x-y__2", None),
        ("_x__2", None),
        ("x__10", Some("x")),
        ("x__2\n", Some("x")),
        ("x__2\n\n", None),
        ("x__+2", None),
        ("x__1", None),
        ("x__02", None),
        ("x__", None),
        ("x", None),
        ("xä__2", None),
        ("x_a__3", Some("x_a")),
        ("x__2a", None),
        ("a__10\n", Some("a")),
        ("ab__99", Some("ab")),
        ("x__\u{662}", None),
    ];
    for (feld, soll) in faelle {
        assert_eq!(store::instanz_basis(feld), soll, "{feld:?}");
    }
}

/// Der Katalog ordnet jedem Vorschlags-Schreiber seine Felder zu (`store.lade_katalog`, Python):
/// `llm` nimmt `afa_jahresbetrag`, `maps` nur `ep_entfernung_km`, `beleg` nimmt
/// `agb_aufwendungen` und nicht `afa_jahresbetrag`, `kontoauszug` nur seine fuenf Betragsfelder;
/// `ep_arbeitstage` nimmt nur `llm`, ein abgeleitetes Feld ohne Frage
/// (`rentner_alter_64_erfuellt`, nicht `askable`) keiner.
#[test]
fn katalog_ordnet_die_felder_den_schreibern_zu() {
    let katalog = Katalog::aus_bindungen(bindungen().iter());
    for (schreiber, feld, soll) in [
        ("llm", "afa_jahresbetrag", true),
        ("llm", "rentner_alter_64_erfuellt", false),
        ("llm", "ep_entfernung_km", true),
        ("maps", "ep_entfernung_km", true),
        ("maps", "afa_jahresbetrag", false),
        ("beleg", "agb_aufwendungen", true),
        ("beleg", "afa_jahresbetrag", false),
        ("kontoauszug", "spenden_betrag", true),
        ("kontoauszug", "agb_aufwendungen", false),
        ("llm", "ep_arbeitstage", true),
        ("maps", "ep_arbeitstage", false),
        ("beleg", "ep_arbeitstage", false),
        ("kontoauszug", "ep_arbeitstage", false),
        ("beleg", "spenden_betrag", true),
        ("maps", "spenden_betrag", false),
    ] {
        assert_eq!(katalog.erlaubt(schreiber, feld), soll, "{schreiber} {feld}");
    }
    let maps = bindungen()
        .iter()
        .filter(|b| katalog.erlaubt("maps", &b.feld_id))
        .count();
    assert_eq!(maps, 1);
}

/// Python `_jahr`: ein Wert, der kein Text ist, ist kein Datum. Die echte Bindung haelt Zahlen
/// am `datum`-Typ auf; hier traegt die Quelle den Typ `int`, damit der Wert die Ableitung erreicht.
#[test]
fn ableitung_liest_eine_zahl_nicht_als_datum() {
    let eigene = mit_aenderung("stammdaten_geburtsdatum", |b| {
        b.typ = Feldtyp::Int;
        b.muster = None;
        b.bereich = None;
    });
    let mut s = leerer_store(2025);
    mit_karte(
        &eigene,
        &bestaetigt("stammdaten_geburtsdatum", &json!(19_610_101)),
        &mut s,
    )
    .unwrap();
    assert_eq!(aktive(&s).len(), 1, "{:?}", aktive(&s));
}

/// Python `_rechne_ab`, Ausloeser ueber das `und_feld`: der Quellwert kommt dann aus der Akte, und
/// dort zaehlt nur ein BESTAETIGTES Quellfeld (`qev.get("zustand") != "bestaetigt"` -> weiter). Ein
/// vorlaeufiger Geburtsdatum-Vorschlag rechnet nichts aus, auch wenn der Haushaltszeitraum danach
/// bestaetigt wird (Geld: sonst stuende das Kind als unter 14 in der Akte, ohne dass jemand das
/// Datum bestaetigt hat). Gegenprobe im selben Test: dasselbe Datum bestaetigt -> das Ziel
/// entsteht. Dritte Probe: ohne Quellfeld in der Akte entsteht nichts (`qev is None`).
#[test]
fn und_feld_ausloeser_liest_nur_eine_bestaetigte_quelle() {
    let zeitraum = bestaetigt(KIND_ZEITRAUM, &json!("01.03-31.10"));

    // Quelle nur vorlaeufig (Vorjahres-Uebernahme), und_feld danach bestaetigt: kein Ziel.
    let mut s = leerer_store(2025);
    let vorschlag = vorlaeufig(
        KIND_GEB,
        &json!("15.06.2012"),
        Schreiber::ImportVorjahr,
        "vorjahr",
    );
    anhaengen(&mut s, &vorschlag).unwrap();
    anhaengen(&mut s, &zeitraum).unwrap();
    assert!(!hat(&s, KIND_UNTER_14), "{:?}", aktive(&s));
    assert_eq!(aktive(&s).len(), 2, "{:?}", aktive(&s));

    // Gegenprobe: dieselbe Reihenfolge mit bestaetigter Quelle schreibt das Ziel.
    let mut s = leerer_store(2025);
    anhaengen(&mut s, &bestaetigt(KIND_GEB, &json!("15.06.2012"))).unwrap();
    anhaengen(&mut s, &zeitraum).unwrap();
    assert_eq!(s.aktives(KIND_UNTER_14).unwrap().wert, PyWert::Bool(true));

    // Ohne Quellfeld in der Akte loest das und_feld allein nichts aus.
    let mut s = leerer_store(2025);
    anhaengen(&mut s, &zeitraum).unwrap();
    assert_eq!(aktive(&s).len(), 1, "{:?}", aktive(&s));
}
