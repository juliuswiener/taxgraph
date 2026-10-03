//! `Store::append_roh`: die Formen, die Pythons `append_event` roh annimmt (`store.py:365`) und
//! die `Store::append` im Typsystem ausschliesst. Die Abweisungen kommen in Pythons Reihenfolge
//! (A, K1, F2, T, dann `signal_2`, dann B).
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use std::collections::HashMap;

use domain::{
    Achsenwert, Feldzustand, Herkunft, HerkunftAlt, HerkunftVektor, PruefTiefe, Schreiber, Signal2,
    Zustand,
};
use serde_json::json;
use store::{
    Abweisung, AbweisungRoh, BindungNachschlag, EventId, NeuesEvent, NeuesEventRoh, Signal, Store,
};

fn herkunft(achse: &str) -> Herkunft {
    Herkunft {
        herkunft: Achsenwert::new(achse).unwrap(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: Achsenwert::new("nutzer").unwrap(),
    }
}

fn roh(schreiber: &str, zustand: Zustand, signal_2: Option<&str>) -> NeuesEventRoh {
    NeuesEventRoh {
        feld_id: "ep_arbeitstage".to_owned(),
        wert: json!(220).into(),
        zustand,
        herkunft: HerkunftVektor::Voll(herkunft("laie")),
        schreiber: schreiber.to_owned(),
        signal: Signal {
            signal_1: Some(None),
            signal_2: signal_2.map(str::to_owned),
        },
        signal_2_fremd: None,
        ersetzt: None,
        ts: Some("2026-01-01T00:00:00+00:00".to_owned()),
    }
}

fn anhaengen(s: &mut Store, neu: &NeuesEventRoh) -> Result<EventId, AbweisungRoh> {
    let leer = HashMap::new();
    s.append_roh(neu, None, BindungNachschlag::neu(&leer))
}

/// Dieselbe Eingabe ueber `append` und `append_roh` gibt dieselbe Kennung: der Rohweg legt nichts
/// anders ab, wo der typisierte Weg es darstellen kann.
#[test]
fn roh_und_typisiert_legen_dasselbe_ab() {
    let leer = HashMap::new();
    let typisiert = NeuesEvent {
        feld_id: "ep_arbeitstage".to_owned(),
        wert: json!(220).into(),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("klick").unwrap(),
        },
        herkunft: herkunft("laie"),
        schreiber: Schreiber::Mensch("ui:x".to_owned()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-01-01T00:00:00+00:00".to_owned()),
    };
    let a = Store::leer(2025, None)
        .append(&typisiert, None, BindungNachschlag::neu(&leer))
        .unwrap();
    let b = anhaengen(
        &mut Store::leer(2025, None),
        &roh("ui:x", Zustand::Bestaetigt, Some("klick")),
    )
    .unwrap();
    assert_eq!(a, b);
}

/// Was Python annimmt und `NeuesEvent` nicht darstellt: `vorlaeufig` MIT `signal_2`, die Alt-Form der
/// Herkunft, ein `signal` ohne `signal_1`. Alles landet so in der Akte, wie es kam.
#[test]
fn nicht_typisierbare_formen_werden_abgelegt() {
    let mut s = Store::leer(2025, None);
    let mut neu = roh("ui:x", Zustand::Vorlaeufig, Some("ok"));
    neu.herkunft = HerkunftVektor::Alt(HerkunftAlt {
        herkunft: Achsenwert::new("laie").unwrap(),
    });
    neu.signal.signal_1 = None;
    anhaengen(&mut s, &neu).unwrap();
    let e = serde_json::to_value(&s.events()[0]).unwrap();
    assert_eq!(e["herkunft"], json!({"herkunft": "laie"}));
    assert_eq!(e["signal"], json!({"signal_2": "ok"}));
    assert_eq!(e["zustand"], "vorlaeufig");
    assert_eq!(e["ts"], "2026-01-01T00:00:00+00:00");
}

#[test]
fn leerer_oder_fehlender_zeitstempel_wird_zur_jetztzeit() {
    for ts in [None, Some(String::new())] {
        let mut s = Store::leer(2025, None);
        let mut neu = roh("ui:x", Zustand::Vorlaeufig, None);
        neu.ts = ts;
        anhaengen(&mut s, &neu).unwrap();
        assert!(s.events()[0].ts.starts_with("20"), "{:?}", s.events()[0].ts);
    }
}

#[test]
fn bestaetigt_braucht_ein_signal_2_das_nicht_leer_ist() {
    // U+001C..U+001F: Pythons `str.strip()` entfernt sie, `str::trim` nicht.
    for signal_2 in [None, Some(""), Some("  "), Some("\u{1c}\u{a0}\t")] {
        let fehler = anhaengen(
            &mut Store::leer(2025, None),
            &roh("ui:x", Zustand::Bestaetigt, signal_2),
        )
        .unwrap_err();
        assert_eq!(fehler, AbweisungRoh::BestaetigtOhneSignal2, "{signal_2:?}");
    }
    assert_eq!(
        AbweisungRoh::BestaetigtOhneSignal2.to_string(),
        "fail-closed: zustand=bestaetigt braucht ein signal_2 (Zwei-Signal)."
    );
}

#[test]
fn signal_2_ohne_text_nennt_den_typ() {
    let mut neu = roh("ui:x", Zustand::Vorlaeufig, None);
    neu.signal_2_fremd = Some("int".to_owned());
    let fehler = anhaengen(&mut Store::leer(2025, None), &neu).unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "fail-closed: signal_2 muss Text oder null sein, nicht int."
    );
    // Auflage A kommt davor: ein `signal_2`, das nicht `null` ist, laesst den Vorschlag auffliegen.
    let mut llm = roh("llm:chat", Zustand::Vorlaeufig, None);
    llm.herkunft = HerkunftVektor::Voll(herkunft("llm_vorschlag"));
    llm.signal_2_fremd = Some("int".to_owned());
    assert!(matches!(
        anhaengen(&mut Store::leer(2025, None), &llm),
        Err(AbweisungRoh::Store(Abweisung::AuflageA { .. }))
    ));
}

/// Pythons Praefix-Klassifikation (`startswith`): `import:beleg2` ist ein Beleg-Schreiber. Als
/// Mensch entkaeme er Auflage A und dem Katalog.
#[test]
fn schreiber_wird_nach_praefix_klassifiziert() {
    for (schreiber, name) in [
        ("import:beleg2", "import:beleg"),
        ("import:kontoauszug-x", "import:kontoauszug"),
        ("import:vorjahr_neu", "import:vorjahr"),
    ] {
        let fehler = anhaengen(
            &mut Store::leer(2025, None),
            &roh(schreiber, Zustand::Bestaetigt, Some("klick")),
        )
        .unwrap_err();
        let AbweisungRoh::Store(Abweisung::AuflageA { schreiber: n, .. }) = fehler else {
            panic!("{schreiber}: {fehler:?}");
        };
        assert_eq!(n, name);
    }
    // Ein Mensch, dessen Name nur so anfaengt wie `llm`, bleibt einer.
    assert!(anhaengen(
        &mut Store::leer(2025, None),
        &roh("llm", Zustand::Bestaetigt, Some("klick")),
    )
    .is_ok());
}

/// Der Name in der Meldung der Auflage A ist der feste Praefix (`store.py:262`).
#[test]
fn auflage_a_nennt_den_praefix_nicht_den_ganzen_schreiber() {
    let fehler = anhaengen(
        &mut Store::leer(2025, None),
        &roh("llm:chat", Zustand::Bestaetigt, Some("klick")),
    )
    .unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "fail-closed (A): llm:-Schreiber muss herkunft=llm_vorschlag, zustand=vorlaeufig, \
         signal_2=null tragen — kein Bestätigen durch die KI."
    );
}

#[test]
fn ersetzt_ohne_kennung_trifft_kein_event() {
    let mut s = Store::leer(2025, None);
    let id = anhaengen(&mut s, &roh("ui:x", Zustand::Vorlaeufig, None)).unwrap();
    for text in ["abc", "", "5", &id.to_string().to_uppercase()] {
        let mut neu = roh("ui:x", Zustand::Vorlaeufig, None);
        neu.ersetzt = Some(text.to_owned());
        assert_eq!(
            anhaengen(&mut s, &neu).unwrap_err(),
            AbweisungRoh::ErsetztZielText(text.to_owned()),
            "{text:?}"
        );
    }
    // Die echte Kennung ersetzt.
    let mut neu = roh("ui:x", Zustand::Vorlaeufig, None);
    neu.ersetzt = Some(id.to_string());
    neu.wert = json!(230).into();
    assert!(anhaengen(&mut s, &neu).is_ok());
}
