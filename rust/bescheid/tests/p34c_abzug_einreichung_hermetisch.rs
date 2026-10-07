//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung (Abweichung Nr. 29 in `rust/fixtures/README.md`): die Rechnung zieht die
//! gezahlte auslaendische Steuer ab, wenn `dba_abzug_statt_anrechnung` wahr ist; das ELSTER-XML kennt den Abzug nicht
//! und schriebe die Steuer immer als anzurechnende Steuer (Kz `E0601901`). Der Marker selbst hat kein Kz. Ein Bescheid,
//! dessen Abzug die Erklaerung als Anrechnung meldet, waere die bekannte Naht-Luecke. Darum sperrt `deklariere` die Abgabe,
//! wenn die Wahl `true` ist UND die gezahlte Steuer ueber 0 liegt (`einreichungs_xml` -> `DeklarationUnvollstaendig` ->
//! 409 `deklaration_unvollstaendig`, `api/src/einreichen.rs`). Alles andere aendert sich nicht.
//!
//! Der Test geht ueber `einreichungs_xml` und nicht ueber `POST /einreichen`: der HTTP-Weg ruft danach `ERiC`. Die Sperre
//! liegt VOR dem Writer, ihr Ergebnis braucht kein ERiC-Schema. Die Akte ist `rust/fixtures/e2e/gesamt.json` (vollstaendig,
//! mit Stammdaten, ohne Auslandsangaben und ohne Kapital), dazu die Auslandsangaben dieses Tests.
//!
//! Fehlt das ERiC-Schema, endet der Lauf OHNE Sperre am Writer mit `XmlNichtBaubar`, nachdem Guard und Deklaration liefen
//! (wie in `unfallkosten_einreichung_hermetisch.rs`); mit Schema ist es ein XML.
//!
//! Die Rechnung (der Zweig, der die Sperre erzwingt) steht in `p34c_abzug_rechnung.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::collections::HashMap;
use std::path::Path;

use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
use bescheid::testhilfe::{index, params};
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Zustand};
use elster::deklariere;
use elster::testhilfe::schemas_da;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";

/// Ein Wert mit dem Zustand `bestaetigt` (zwei Signale) oder `vorlaeufig` (ohne zweites Signal).
fn setze(s: &mut Store, feld: &str, wert: Value, zustand: Zustand) {
    let leer = HashMap::new();
    let bestaetigt = zustand == Zustand::Bestaetigt;
    s.append_roh(
        &NeuesEventRoh {
            feld_id: feld.to_owned(),
            wert: wert.into(),
            zustand,
            herkunft: HerkunftVektor::Voll(Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }),
            schreiber: "ui:laie".to_owned(),
            signal: Signal {
                signal_1: Some(None),
                signal_2: bestaetigt.then(|| format!("ok@{feld}")),
                signal_2_fehlt: !bestaetigt,
            },
            signal_2_fremd: None,
            ersetzt: None,
            ts: Some("2026-09-01T00:00:00+00:00".to_owned()),
        },
        None,
        BindungNachschlag::neu(&leer),
    )
    .unwrap();
}

/// Die vollstaendige Akte mit Auslandsangaben. `wahl` und `steuer` (in CENT) sind `None`, wenn nie beantwortet; die
/// Auslandseinkuenfte stehen bei jeder Angabe zur Steuer und zur Wahl mit 5.000 Euro dabei.
fn akte(wahl: Option<bool>, steuer: Option<i64>) -> Store {
    akte_mit(wahl, steuer, Zustand::Bestaetigt)
}

/// Wie [`akte`], mit dem Zustand der gezahlten Steuer (`Vorlaeufig`: noch nicht bestaetigt).
fn akte_mit(wahl: Option<bool>, steuer: Option<i64>, steuer_zustand: Zustand) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    if wahl.is_some() || steuer.is_some() {
        setze(&mut s, EINKUENFTE, json!(500_000), Zustand::Bestaetigt);
    }
    if let Some(c) = steuer {
        setze(&mut s, STEUER, json!(c), steuer_zustand);
    }
    if let Some(w) = wahl {
        setze(&mut s, WAHL, json!(w), Zustand::Bestaetigt);
    }
    s
}

fn lauf(s: &Store) -> Result<String, EinreichFehler> {
    einreichungs_xml(s, index(), params(), "BY", Some("74931".to_owned())).map(|e| e.xml)
}

/// Die Eintraege der Sperre als `(feld_id, grund)`; leer, wenn `lauf` nicht mit `DeklarationUnvollstaendig` endet.
fn sperre(r: &Result<String, EinreichFehler>) -> Vec<(String, String)> {
    match r {
        Err(EinreichFehler::DeklarationUnvollstaendig(e)) => e
            .iter()
            .map(|x| (x.feld_id.clone(), x.grund.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Der Kz-Wert aus `deklariere` als Text (ohne Anfuehrungszeichen); `None`, wenn der Kz fehlt.
fn kz_wert(s: &Store, kz: &str) -> Option<String> {
    let (felder, _) = s.materialisiere(None).unwrap();
    let d = deklariere(&felder, index(), 2025, None).unwrap();
    d.deklaration
        .get(kz)
        .map(|v| v.to_string().trim_matches('"').to_owned())
}

/// KONTROLLE zuerst: die Anrechnung (ohne Wahl oder mit Wahl `false`) kommt durch `deklariere`, sie endet im XML (mit
/// Schema) oder am Writer (ohne), und die gezahlte Steuer steht als anzurechnende Steuer unter `E0601901`, die
/// Einkuenfte unter `E0601401`. Sonst belegte die Sperre unten nichts: ein Fall, der aus anderem Grund sperrt, laesst sie
/// gruen.
#[test]
fn kontrolle_die_anrechnung_ist_einreichbar_und_traegt_die_steuer() {
    for wahl in [None, Some(false)] {
        let s = akte(wahl, Some(70_000));
        let r = lauf(&s);
        assert_eq!(r.is_ok(), schemas_da(2025), "Wahl {wahl:?}: {r:?}");
        assert!(
            sperre(&r).is_empty(),
            "Wahl {wahl:?}: die Anrechnung sperrt schon: {r:?}"
        );
        assert_eq!(
            kz_wert(&s, "E0601901").as_deref(),
            Some("700"),
            "Wahl {wahl:?}"
        );
        assert_eq!(
            kz_wert(&s, "E0601401").as_deref(),
            Some("5000"),
            "Wahl {wahl:?}"
        );
    }
}

/// Die Wahl `true` und eine gezahlte Steuer ueber 0 sperren die Abgabe mit EIGENEM Grund (409
/// `deklaration_unvollstaendig`); dieser Grund nennt Abzug, Anrechnung und die Sperre, kein anderes Feld steht in der
/// Liste. Ein Cent genuegt, wie bei den Unfallkosten.
#[test]
fn abzug_gewaehlt_und_steuer_ueber_null_sperren_die_abgabe_mit_eigenem_grund() {
    for cent in [70_000, 1] {
        let r = lauf(&akte(Some(true), Some(cent)));
        let e = sperre(&r);
        assert_eq!(
            e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
            [WAHL],
            "{cent} Cent: die Sperre nennt genau dieses Feld, erhalten {r:?}"
        );
        let grund = &e.first().unwrap().1;
        for teil in ["Abzug", "Anrechnung", "gesperrt", "nein"] {
            assert!(grund.contains(teil), "Grund ohne `{teil}`: {grund}");
        }
    }
}

/// Nichts sperrt, wenn eine der beiden Bedingungen fehlt: Wahl `true` ohne Steuer (nie beantwortet oder 0), Wahl `false`
/// oder nie beantwortet mit Steuer. Der Lauf endet dann wie der der Anrechnung: im XML oder am Writer, nie in der Sperre.
#[test]
fn ohne_wahl_oder_ohne_steuer_sperrt_nichts() {
    let faelle = [
        ("Wahl ja, Steuer nie beantwortet", Some(true), None),
        ("Wahl ja, Steuer 0", Some(true), Some(0)),
        ("Wahl nein, Steuer 700 Euro", Some(false), Some(70_000)),
        ("Wahl nie beantwortet, Steuer 700 Euro", None, Some(70_000)),
    ];
    for (name, wahl, steuer) in faelle {
        let r = lauf(&akte(wahl, steuer));
        assert!(sperre(&r).is_empty(), "{name} sperrt: {r:?}");
        assert_eq!(r.is_ok(), schemas_da(2025), "{name}: {r:?}");
    }
}

/// Eine noch nicht bestaetigte Steuer sperrt mit IHREM Grund ("Pflicht-Bestaetigung fehlt"), nicht zusaetzlich mit dem des
/// Abzugs: die Liste nennt nur die Steuer, nicht die Wahl. Sonst stuende dieselbe Luecke doppelt in der Liste.
#[test]
fn eine_unbestaetigte_steuer_sperrt_nur_mit_ihrem_eigenen_grund() {
    let r = lauf(&akte_mit(Some(true), Some(70_000), Zustand::Vorlaeufig));
    let e = sperre(&r);
    assert_eq!(
        e.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(),
        [STEUER],
        "erhalten {r:?}"
    );
    assert!(e.first().unwrap().1.contains("Bestätigung"), "{e:?}");
}

/// Der Marker steht NIE im XML-Teil der Deklaration: er kommt mit Grund in `nicht_deklariert` (kein Kz, kein erfundener
/// Wert), bei Wahl `true` und Steuer ueber 0 zusaetzlich unter `unvollstaendig`, sonst nicht. Die Steuer bleibt dabei
/// unter `E0601901`: die Sperre ersetzt sie nicht, sie haelt nur die Abgabe an.
#[test]
fn der_marker_steht_mit_grund_in_nicht_deklariert_und_die_steuer_bleibt_unveraendert() {
    for (wahl, steuer, gesperrt) in [
        (true, 70_000_i64, true),
        (true, 0, false),
        (false, 70_000, false),
    ] {
        let s = akte(Some(wahl), Some(steuer));
        let (felder, _) = s.materialisiere(None).unwrap();
        let d = deklariere(&felder, index(), 2025, None).unwrap();
        let nicht: Vec<_> = d
            .nicht_deklariert
            .iter()
            .filter(|e| e.feld_id == WAHL)
            .collect();
        assert_eq!(
            nicht.len(),
            1,
            "Wahl {wahl}, {steuer} Cent: {:?}",
            d.nicht_deklariert
        );
        assert!(!nicht.first().unwrap().grund.is_empty());
        assert_eq!(
            d.unvollstaendig().iter().any(|e| e.feld_id == WAHL),
            gesperrt,
            "Wahl {wahl}, {steuer} Cent: {:?}",
            d.unvollstaendig()
        );
        // Cent -> volle Euro: 70.000 Cent stehen als "700" im Kz, eine bestaetigte 0 als "0".
        assert_eq!(
            kz_wert(&s, "E0601901"),
            Some((steuer / 100).to_string()),
            "Wahl {wahl}, {steuer} Cent"
        );
    }
}
