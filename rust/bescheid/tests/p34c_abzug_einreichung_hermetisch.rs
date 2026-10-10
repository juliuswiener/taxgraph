//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung (Abweichungen Nr. 29 und Nr. 41 in `rust/fixtures/README.md`): die Rechnung
//! kuerzt die Einkuenfte um die gezahlte auslaendische Steuer, wenn `dba_abzug_statt_anrechnung` wahr ist. Die Erklaerung
//! traegt diesen Betrag seit Nr. 41 unter `E0600920` (Anlage AUS, Zeile 10, "abgezogene auslaendische Steuern nach § 34c
//! Abs. 2", aufgerundet) und NICHT mehr als anzurechnende Steuer unter `E0601901`. Der Marker selbst hat kein Kz.
//!
//! Die ABGABE bleibt gesperrt, wenn die Wahl `true` ist UND die gezahlte Steuer ueber 0 liegt (`einreichungs_xml` ->
//! `DeklarationUnvollstaendig` -> 409 `deklaration_unvollstaendig`, `api/src/einreichen.rs`): der Bescheid kuerzt die
//! Werbungskosten der Anlage N, und diese Zeile (`Weitere_Wk/Sonst`, "Sonstige Werbungskosten") schreibt die Erklaerung noch
//! nicht. Ohne sie wichen Erklaerung und Bescheid um den Abzug voneinander ab. Alles andere aendert sich nicht.
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
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, Sperrgrund, Vz, Zustand};
use elster::testhilfe::schemas_da;
use elster::{deklariere, erzeuge_xml, validiere_xsd_text, XmlOptionen};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEventRoh, Signal, Store};

const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const ART: &str = "dba_einkunftsart";
/// Seit Abweichung Nr. 51 braucht jede Akte mit Auslandseinkuenften einen Staat; Frankreich rechnet wie "kein Staat" (Anrechnung).
const STAAT: &str = "dba_staat";

/// Anlage AUS Zeile 10: "abgezogene ausländische Steuern nach § 34c Abs. 2 `EStG`" (E10-2025.xsd, `Staat_Spez_InvFonds`).
const ABZUG_KZ: &str = "E0600920";
/// Anlage AUS: die anzurechnende ausländische Steuer ("für alle Einkunftsarten").
const ANRECHNUNG_KZ: &str = "E0601901";

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
    akte_voll(wahl, steuer, steuer_zustand, "unselbstaendige_arbeit")
}

/// Wie [`akte_mit`], mit der Einkunftsart der Auslandseinkuenfte. Arbeitslohn und einzeln veranlagt (die Akte `gesamt.json`)
/// ist der Fall, den die Rechnung traegt; jede andere Art sperrt schon den Bescheid (`dba_abzug_offen`,
/// `p34c_abzug_sperre.rs`), bevor die Erklaerung sperren kann.
fn akte_voll(wahl: Option<bool>, steuer: Option<i64>, steuer_zustand: Zustand, art: &str) -> Store {
    let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures/e2e/gesamt.json");
    let roh: Value = serde_json::from_slice(&std::fs::read(pfad).unwrap()).unwrap();
    let mut s = Store::aus_datei(serde_json::from_value(roh).unwrap());
    if wahl.is_some() || steuer.is_some() {
        setze(&mut s, EINKUENFTE, json!(500_000), Zustand::Bestaetigt);
        setze(&mut s, ART, json!(art), Zustand::Bestaetigt);
        setze(&mut s, STAAT, json!("Frankreich"), Zustand::Bestaetigt);
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
/// Einkuenfte unter `E0601401`; `E0600920` (der Abzug) fehlt. Sonst belegte die Sperre unten nichts: ein Fall, der aus
/// anderem Grund sperrt, laesst sie gruen.
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
        assert_eq!(kz_wert(&s, ABZUG_KZ), None, "Wahl {wahl:?}");
    }
}

/// AK4: bei gewaehltem Abzug und gezahlter Steuer ueber 0 traegt `deklariere` die Steuer unter `E0600920` (Anlage AUS,
/// Zeile 10) und NICHT unter `E0601901`. Aufgerundet (ein Aufwand, "zu Ihren Gunsten"): 700,00 -> 700, 700,01 -> 701,
/// 0,01 -> 1. Die Einkuenfte der Anlage AUS (`E0601401`) bleiben unveraendert: ob Zeile 7 "nach Abzug" gemeint ist, ist offen.
#[test]
fn abzug_gewaehlt_traegt_die_steuer_unter_e0600920_aufgerundet_und_nicht_als_anrechnung() {
    for (cent, euro) in [(70_000, "700"), (70_001, "701"), (1, "1")] {
        let s = akte(Some(true), Some(cent));
        assert_eq!(
            kz_wert(&s, ABZUG_KZ).as_deref(),
            Some(euro),
            "{cent} Cent: der Abzug"
        );
        assert_eq!(
            kz_wert(&s, ANRECHNUNG_KZ),
            None,
            "{cent} Cent: dieselbe Steuer darf nicht zusaetzlich als Anrechnung stehen"
        );
        assert_eq!(
            kz_wert(&s, "E0601401").as_deref(),
            Some("5000"),
            "{cent} Cent"
        );
    }
    // Gegenprobe zur Rundung: die Anrechnung rundet dieselben 70.001 Cent ab (Einnahme-Seite, `E0601901`).
    assert_eq!(
        kz_wert(&akte(Some(false), Some(70_001)), ANRECHNUNG_KZ).as_deref(),
        Some("700")
    );
}

/// Der Writer setzt `E0600920` ins XML, wo das Schema es erwartet. Der Writer verweigert selbst jede unvollstaendige
/// Deklaration (`erzeuge_xml`: "kein Submission-XML"), also auch die mit der Abzug-Sperre. Der Test baut darum die
/// Deklaration der Anrechnung (Wahl `false`, ohne Sperre) und tauscht NUR den Schluessel: `E0601901` -> `E0600920`, mit dem
/// Betrag des echten Abzug-Laufs (`abzug_gewaehlt_traegt_die_steuer_unter_e0600920_*`). Das Element steht mit diesem Betrag
/// im Text; die Schema-Pruefung (`xmllint`) faellt fuer den Abzug nicht schlechter aus als fuer die Anrechnung derselben
/// Akte. Das zeigt, dass das Schema `E0600920` an dieser Stelle nimmt; ein ERiC-Lauf (`checkESt`) ist es nicht. Ohne Schema
/// entfaellt der Test.
#[test]
fn der_writer_setzt_e0600920_ins_xml_und_das_schema_nimmt_es_wie_die_anrechnung() {
    if !schemas_da(2025) {
        return;
    }
    let xml = |abzug: bool| {
        let s = akte(Some(false), Some(70_001));
        let (felder, _) = s.materialisiere(None).unwrap();
        let mut d = deklariere(&felder, index(), 2025, None).unwrap();
        if abzug {
            let echt = kz_wert(&akte(Some(true), Some(70_001)), ABZUG_KZ).unwrap();
            d.deklaration.remove(ANRECHNUNG_KZ).unwrap();
            d.deklaration
                .insert(ABZUG_KZ.to_owned(), json!(echt.parse::<i64>().unwrap()));
        }
        let opt = XmlOptionen {
            hersteller_id: Some("74931".to_owned()),
            snapshot: Some(&felder),
            ..XmlOptionen::default()
        };
        erzeuge_xml(&d, &opt).unwrap()
    };
    let (abzug, anrechnung) = (xml(true), xml(false));
    assert!(
        abzug.contains("<E0600920>701</E0600920>"),
        "Abzug-XML ohne E0600920"
    );
    assert!(!abzug.contains("<E0601901>"), "Abzug-XML mit Anrechnung");
    assert!(
        anrechnung.contains("<E0601901>700</E0601901>") && !anrechnung.contains("<E0600920>"),
        "Kontrolle: Anrechnung"
    );
    let (ok_abzug, meldung) = validiere_xsd_text(abzug.as_bytes(), Vz::Vz2025);
    let (ok_anrechnung, _) = validiere_xsd_text(anrechnung.as_bytes(), Vz::Vz2025);
    println!("xmllint 2025: Abzug-XML {ok_abzug}, Anrechnungs-XML {ok_anrechnung}");
    assert_eq!(
        ok_abzug, ok_anrechnung,
        "Abzug-XML gegen das Schema: {meldung}"
    );
}

/// Die Wahl `true` und eine gezahlte Steuer ueber 0 sperren die Abgabe mit EIGENEM Grund (409
/// `deklaration_unvollstaendig`); dieser Grund nennt Abzug, Anrechnung und die Sperre, dazu die Zeile "Sonstiges" in Anlage N
/// (seit Abweichung Nr. 49 steht sie in der Erklaerung, aber kein `checkESt`-Lauf hat sie angenommen) und den
/// `checkESt`-Lauf, kein anderes Feld steht in der Liste. Ein Cent genuegt, wie bei den Unfallkosten.
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
        for teil in [
            "Abzug",
            "Anrechnung",
            "gesperrt",
            "nein",
            "Anlage N",
            "Sonstiges",
            "checkESt",
            "noch nie angenommen",
        ] {
            assert!(grund.contains(teil), "Grund ohne `{teil}`: {grund}");
        }
        assert!(!grund.contains("fehlt noch"), "der Grund behauptet noch, die Zeile fehle: {grund}");
    }
}

/// Ein Abzug in einem Fall, den die Rechnung nicht traegt (hier: Zinsen statt Arbeitslohn), endet schon im BESCHEID mit
/// `Gesperrt(DbaAbzugOffen)`, nicht erst in der Liste der Erklaerung (Abweichung Nr. 41). KONTROLLE: derselbe Fall mit
/// Arbeitslohn kommt in die Erklaerung und sperrt dort mit dem Abzug-Grund (`abzug_gewaehlt_und_steuer_ueber_null_*`).
#[test]
fn ein_nicht_getragener_abzug_endet_im_bescheid_mit_dba_abzug_offen() {
    let s = akte_voll(Some(true), Some(70_000), Zustand::Bestaetigt, "zinsen");
    let r = lauf(&s);
    assert!(
        matches!(r, Err(EinreichFehler::Gesperrt(Sperrgrund::DbaAbzugOffen))),
        "erhalten {r:?}"
    );
    let kontrolle = lauf(&akte(Some(true), Some(70_000)));
    assert!(
        matches!(kontrolle, Err(EinreichFehler::DeklarationUnvollstaendig(_))),
        "Arbeitslohn: erhalten {kontrolle:?}"
    );
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
/// Wert), bei Wahl `true` und Steuer ueber 0 zusaetzlich unter `unvollstaendig`, sonst nicht. Die Steuer steht dabei an
/// genau EINER Stelle: unter `E0600920` bei Abzug mit Steuer ueber 0, sonst unter `E0601901` (auch die bestaetigte 0 bei
/// Wahl `true`: ohne Steuer gibt es nichts abzuziehen).
#[test]
fn der_marker_steht_mit_grund_in_nicht_deklariert_und_die_steuer_steht_an_einer_stelle() {
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
        assert_ne!(nicht.first().unwrap().grund.len(), 0);
        assert_eq!(
            d.unvollstaendig().iter().any(|e| e.feld_id == WAHL),
            gesperrt,
            "Wahl {wahl}, {steuer} Cent: {:?}",
            d.unvollstaendig()
        );
        // Cent -> volle Euro: 70.000 Cent stehen als "700" im Kz, eine bestaetigte 0 als "0".
        let euro = Some((steuer / 100).to_string());
        let (abzug, anrechnung) = if gesperrt { (euro, None) } else { (None, euro) };
        assert_eq!(
            kz_wert(&s, ABZUG_KZ),
            abzug,
            "Wahl {wahl}, {steuer} Cent: Abzug"
        );
        assert_eq!(
            kz_wert(&s, ANRECHNUNG_KZ),
            anrechnung,
            "Wahl {wahl}, {steuer} Cent: Anrechnung"
        );
    }
}
