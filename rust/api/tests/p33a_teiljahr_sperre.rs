//! § 33a Abs. 1 EStG, Schritt 1 (Julius-Regel "erst Sperren, dann Rechnungen", 2026-10-08; Ticket `p33a-rechnet-ohne-zwoelftelung-anteil-und-
//! haelftelung`): wer eine Person nur einen Teil des Jahres unterstuetzt (`p33a_unterstuetzungszeitraum` ist nicht das ganze Jahr) oder wessen
//! Unterhalt eine weitere Person mitgetragen hat (`p33a_weitere_person_beteiligt` = ja), bekommt keine Zahl (`p33a_unterhalt_nicht_gerechnet`).
//! Vorher zog die Rechnung in beiden Faellen den vollen Hoechstbetrag ab (12.096 EUR im VZ 2025): § 33a Abs. 3 S. 1 kuerzt um ein Zwoelftel je
//! Monat ohne Unterstuetzung, Abs. 1 S. 7 teilt den Betrag unter mehreren Zahlern. Die Rechnung dafuer ist Schritt 2 und 3.
//!
//! Gemessen wird ueber die HTTP-Schnittstelle, so wie die Oberflaeche es sieht, in der Scheibe `gesamt`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

type Paare = Vec<(&'static str, Value)>;

const UNTERHALT: &str = "p33a_unterhalt_aufwendungen";
const ZEITRAUM: &str = "p33a_unterstuetzungszeitraum";
const WEITERE: &str = "p33a_weitere_person_beteiligt";
const GRUND: &str = "p33a_unterhalt_nicht_gerechnet";
const GANZES_JAHR: &str = "01.01-31.12";
/// 15.000 EUR Unterhalt: ueber dem Hoechstbetrag von 12.096 EUR (VZ 2025), der Abzug ist also der Hoechstbetrag.
const BETRAG: i64 = 1_500_000;

struct Dienst {
    zustand: Zustand,
    token: String,
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst { zustand, token, _tmp: tmp }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let text = body.map(ToString::to_string);
    let mut b = Request::builder().method(methode).uri(pfad).header("authorization", format!("Bearer {}", d.token));
    if let Some(t) = &text {
        b = b.header("content-type", "application/json").header("content-length", t.len().to_string());
    }
    let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (teile.status.as_u16(), serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// Schreibt eine Antwort des Nutzers. `bestaetigt = false` schreibt sie vorlaeufig (ohne zweites Signal), wie ein Vorjahreswert oder ein
/// KI-Vorschlag. Hat das Feld schon ein aktives Ereignis, ersetzt die Antwort es (wie die Oberflaeche).
async fn schreibe(d: &Dienst, feld: &str, wert: Value, bestaetigt: bool) {
    let (_, stand) = sende(d, "GET", "/fall/pt/stand", None).await;
    let (zustand, signal_2) = if bestaetigt { ("bestaetigt", json!(format!("ok@{feld}"))) } else { ("vorlaeufig", Value::Null) };
    let mut rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": zustand, "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": signal_2},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    if let Some(id) = stand["felder"][feld]["event_id"].as_str() {
        rumpf["ersetzt"] = json!(id);
    }
    let (status, antwort) = sende(d, "POST", "/fall/pt/event", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST {feld}: {antwort}");
}

async fn fall_mit(paare: &[(&'static str, Value)]) -> Dienst {
    fall_in("gesamt", paare).await
}

async fn fall_in(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "pt", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        schreibe(&d, feld, wert.clone(), true).await;
    }
    d
}

async fn ergebnis(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/pt/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

async fn fragen(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/pt/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    antwort
}

fn fragen_ids(f: &Value) -> Vec<String> {
    f["fragen"].as_array().unwrap().iter().map(|q| q["feld_id"].as_str().unwrap().to_owned()).collect()
}

/// Pflichtfelder der Scheibe `gesamt` (der Kegel), Einzelveranlagung, Lohn 150.000 EUR, sonst neutral (wie `kein_kind_sperre::kegel_gesamt`).
fn kegel_gesamt() -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(15_000_000)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]
}

/// Pflichtfelder der Scheibe `rentner_gesamt` (wie `kein_kind_sperre::kegel_rentner`).
fn kegel_rentner() -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(2_000_000)),
        ("rentner_renten_beginn_jahr", json!(2_025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("versicherungsart", json!("gesetzlich_an")),
        ("basis_kv", json!(0)),
        ("basis_pv", json!(0)),
        ("vorsorge_arbeitslosenversicherung", json!(0)),
        ("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ("vorsorge_unfall_haftpflicht", json!(0)),
        ("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ("mit_anspruch_auf_zuschuss", json!(false)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("rentner_rentenfreibetrag", json!(0)),
    ]
}

fn mit(mut basis: Paare, mehr: Paare) -> Paare {
    basis.extend(mehr);
    basis
}

/// Der Fall mit 15.000 EUR Unterhalt und den Angaben `extra`.
fn mit_unterhalt(extra: Paare) -> Paare {
    mit(mit(kegel_gesamt(), vec![(UNTERHALT, json!(BETRAG))]), extra)
}

fn zahl(e: &Value) -> Option<i64> {
    e["zahl_cent"].as_i64()
}

/// AK1: Teiljahr und weitere Person halten die Zahl zurueck, jede Gegenprobe rechnet. Die Tabelle prueft BEIDE Seiten: ein Test nur auf
/// "keine Zahl" bliebe auch bei einem unvollstaendigen Kegel gruen, deshalb steht jede Sperre neben Faellen mit Zahl und Grund `bestaetigt`.
#[tokio::test]
async fn teiljahr_und_weitere_person_halten_die_zahl_zurueck() {
    let faelle: Vec<(&str, Paare, bool)> = vec![
        ("nichts beantwortet", vec![], false),
        ("ganzes Jahr", vec![(ZEITRAUM, json!(GANZES_JAHR))], false),
        ("weitere Person nein", vec![(WEITERE, json!(false))], false),
        ("ganzes Jahr, weitere Person nein", vec![(ZEITRAUM, json!(GANZES_JAHR)), (WEITERE, json!(false))], false),
        ("halbes Jahr", vec![(ZEITRAUM, json!("01.01-30.06"))], true),
        ("elf Monate", vec![(ZEITRAUM, json!("01.02-31.12"))], true),
        ("ein Tag zu frueh zu Ende", vec![(ZEITRAUM, json!("01.01-30.12"))], true),
        ("ein Tag zu spaet angefangen", vec![(ZEITRAUM, json!("02.01-31.12"))], true),
        ("Schalttag am Anfang", vec![(ZEITRAUM, json!("29.02-31.12"))], true),
        ("weitere Person ja", vec![(WEITERE, json!(true))], true),
        ("ganzes Jahr, weitere Person ja", vec![(ZEITRAUM, json!(GANZES_JAHR)), (WEITERE, json!(true))], true),
        ("halbes Jahr, weitere Person nein", vec![(ZEITRAUM, json!("01.01-30.06")), (WEITERE, json!(false))], true),
    ];
    for (name, extra, gesperrt) in faelle {
        let d = fall_mit(&mit_unterhalt(extra)).await;
        let e = ergebnis(&d).await;
        if gesperrt {
            assert_eq!(e["grund"], GRUND, "{name}: {e}");
            assert!(e["zahl_cent"].is_null(), "{name}: eine Zahl trotz Sperre: {e}");
        } else {
            assert_eq!(e["grund"], "bestaetigt", "{name}: {e}");
            assert!(zahl(&e).is_some(), "{name}: keine Zahl: {e}");
        }
    }
}

/// AK1b: dieselbe Sperre in der Scheibe `rentner_gesamt` (der `gesamt_guard`-Zweig gilt fuer beide Scheiben). Kontrolle je Fall: ganzes
/// Jahr und keine weitere Person rechnen.
#[tokio::test]
async fn die_rentner_scheibe_sperrt_ebenso() {
    let basis = |extra: Paare| mit(mit(kegel_rentner(), vec![(UNTERHALT, json!(BETRAG))]), extra);
    for (name, extra, gesperrt) in [
        ("ganzes Jahr", vec![(ZEITRAUM, json!(GANZES_JAHR)), (WEITERE, json!(false))], false),
        ("halbes Jahr", vec![(ZEITRAUM, json!("01.01-30.06"))], true),
        ("weitere Person ja", vec![(WEITERE, json!(true))], true),
    ] {
        let e = ergebnis(&fall_in("rentner_gesamt", &basis(extra)).await).await;
        if gesperrt {
            assert_eq!(e["grund"], GRUND, "rentner_gesamt, {name}: {e}");
            assert!(e["zahl_cent"].is_null(), "rentner_gesamt, {name}: {e}");
        } else {
            assert_eq!(e["grund"], "bestaetigt", "rentner_gesamt, {name}: {e}");
            assert!(zahl(&e).is_some(), "rentner_gesamt, {name}: {e}");
        }
    }
}

/// AK2 (enge Sperre): ohne Unterhaltsbetrag aendern Teiljahr und weitere Person nichts, sonst sperrte jeder Fall mit einer Antwort auf die
/// Folgefragen. KONTROLLE im selben Test: dieselben Antworten MIT Betrag sperren.
#[tokio::test]
async fn ohne_unterhaltsbetrag_sperrt_nichts() {
    let extra = || -> Paare { vec![(ZEITRAUM, json!("01.01-30.06")), (WEITERE, json!(true))] };
    let ohne = fall_mit(&mit(kegel_gesamt(), extra())).await;
    let e = ergebnis(&ohne).await;
    assert_eq!(e["grund"], "bestaetigt", "ohne Betrag: {e}");
    let null = fall_mit(&mit(mit(kegel_gesamt(), vec![(UNTERHALT, json!(0))]), extra())).await;
    let e = ergebnis(&null).await;
    assert_eq!(e["grund"], "bestaetigt", "Betrag 0: {e}");
    let mit_betrag = fall_mit(&mit_unterhalt(extra())).await;
    let e = ergebnis(&mit_betrag).await;
    assert_eq!(e["grund"], GRUND, "Kontrolle, mit Betrag: {e}");
}

/// AK3: die Fragenliste nennt den Grund samt Klartext, und der Klartext sagt, WARUM keine Zahl kommt. Die Antwort laesst sich aendern:
/// wer sie auf das ganze Jahr (oder "keine weitere Person") zurueckstellt, bekommt die Zahl der Gegenprobe. Der Test belegt NICHT, dass die
/// Zurueckstellung richtig ist, nur dass die Sperre keine Sackgasse ohne Ausgang baut.
#[tokio::test]
async fn die_fragenliste_nennt_grund_und_klartext_und_die_antwort_laesst_sich_aendern() {
    let d = fall_mit(&mit_unterhalt(vec![(ZEITRAUM, json!("01.01-30.06")), (WEITERE, json!(true))])).await;
    let f = fragen(&d).await;
    assert_eq!(f["ring_gesperrt"], GRUND, "{f}");
    let klartext = f["ring_gesperrt_klartext"].as_str().unwrap();
    for muss in ["Unterhalt", "Zwölftel", "weitere", "ändere deine Antwort"] {
        assert!(klartext.contains(muss), "Klartext nennt {muss:?} nicht: {klartext}");
    }
    // Nur das Zeitraum-Feld zurueck: die weitere Person sperrt weiter.
    schreibe(&d, ZEITRAUM, json!(GANZES_JAHR), true).await;
    assert_eq!(ergebnis(&d).await["grund"], GRUND, "die weitere Person sperrt allein");
    schreibe(&d, WEITERE, json!(false), true).await;
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], "bestaetigt", "{e}");
    let gegenprobe = ergebnis(&fall_mit(&mit_unterhalt(vec![])).await).await;
    assert_eq!(zahl(&e), zahl(&gegenprobe), "die Zahl nach der Zurueckstellung ist die des Falls ohne beide Antworten");
}

/// AK4: auch eine VORLAEUFIGE Antwort (Vorjahreswert, KI-Vorschlag) sperrt, und die Frage bleibt in der Liste. Der Nutzer bestaetigt sie oder
/// aendert sie; bis dahin wuerde die Zahl sonst mit dem vollen Hoechstbetrag rechnen, obwohl der Vorschlag ein Teiljahr nennt.
#[tokio::test]
async fn eine_vorlaeufige_antwort_sperrt_und_die_frage_bleibt_in_der_liste() {
    for (feld, wert) in [(ZEITRAUM, json!("01.01-30.06")), (WEITERE, json!(true))] {
        let d = fall_mit(&mit_unterhalt(vec![])).await;
        schreibe(&d, feld, wert.clone(), false).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], GRUND, "{feld} vorlaeufig: {e}");
        assert!(fragen_ids(&fragen(&d).await).iter().any(|f| f == feld), "{feld}: die Frage fehlt in der Liste");
        // Bestaetigt mit dem Normalwert: die Sperre faellt, die Frage auch.
        schreibe(&d, feld, if feld == ZEITRAUM { json!(GANZES_JAHR) } else { json!(false) }, true).await;
        assert_eq!(ergebnis(&d).await["grund"], "bestaetigt", "{feld} bestaetigt mit dem Normalwert");
    }
}

/// Der Status der Regel, an der das Feld haengt (`ausgeschlossen`, `relevant`, ...), wie die Oberflaeche ihn aus `/stand` und der Frage liest.
async fn regelstatus(d: &Dienst, feld: &str) -> Value {
    let (_, frage) = sende(d, "GET", &format!("/fall/pt/feld/{feld}/frage"), None).await;
    let regel = frage["frage"]["regel_id"].as_str().unwrap_or("?").to_owned();
    let (_, stand) = sende(d, "GET", "/fall/pt/stand", None).await;
    stand["relevanz"][&regel]["status"].clone()
}

/// AK5 (Julius, 2026-10-22: "Haelt eine Sperre die Zahl zurueck, muss die zugehoerige Frage sichtbar sein"): das Kreuz `kein_unterhalt` ("ich
/// zahle keinen Unterhalt") verbirgt den Unterhaltsblock, ein gespeicherter Betrag gewinnt trotzdem (Entscheidung vom 2026-10-09). Dann
/// steht die Frage, deren Antwort die Sperre ausloest, in der Liste, und genau sie: nicht die andere. Nach der Antwort faellt die Sperre.
/// Vorbedingung im Test: die Regel der Frage ist `ausgeschlossen` (sonst waere der Test auch ohne den Helfer gruen). KONTROLLEN: ohne
/// Kreuz steht die bestaetigte Antwort nicht noch einmal in der Liste; mit Kreuz, aber ohne Ursache, gibt es weder Sperre noch Frage.
#[tokio::test]
async fn das_kreuz_verbirgt_die_ursache_nicht_vor_dem_nutzer() {
    let kreuz = ("kein_unterhalt", json!(true));
    // (Name, Antworten, Ursachen in der Liste, Ursachen NICHT in der Liste, Antworten, die die Sperre aufloesen)
    let faelle: Vec<(&str, Paare, Vec<&str>, Vec<&str>, Paare)> = vec![
        ("Teiljahr", vec![(ZEITRAUM, json!("01.01-30.06"))], vec![ZEITRAUM], vec![WEITERE], vec![(ZEITRAUM, json!(GANZES_JAHR))]),
        ("weitere Person", vec![(WEITERE, json!(true))], vec![WEITERE], vec![ZEITRAUM], vec![(WEITERE, json!(false))]),
        (
            "beides",
            vec![(ZEITRAUM, json!("01.01-30.06")), (WEITERE, json!(true))],
            vec![ZEITRAUM, WEITERE],
            vec![],
            vec![(ZEITRAUM, json!(GANZES_JAHR)), (WEITERE, json!(false))],
        ),
    ];
    for (name, antworten, in_liste, nicht_in_liste, aufloesung) in faelle {
        let d = fall_mit(&mit_unterhalt(antworten.clone())).await;
        // Erst das Kreuz: davor ist der Block sichtbar, die Antworten stehen als Belegt-Zeilen.
        schreibe(&d, kreuz.0, kreuz.1.clone(), true).await;
        for feld in [ZEITRAUM, WEITERE] {
            assert_eq!(regelstatus(&d, feld).await, json!("ausgeschlossen"), "{name}: das Kreuz verbirgt {feld} nicht, der Test misst nichts");
        }
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], GRUND, "{name}: {e}");
        let ids = fragen_ids(&fragen(&d).await);
        for feld in &in_liste {
            assert!(ids.iter().any(|i| i == feld), "{name}: die Frage {feld} fehlt in der Liste: {ids:?}");
        }
        for feld in &nicht_in_liste {
            assert!(!ids.iter().any(|i| i == feld), "{name}: {feld} steht in der Liste, obwohl er die Sperre nicht ausloest: {ids:?}");
        }
        // Die Antwort aendern, wie die Oberflaeche es tut (`ersetzt` auf das aktive Ereignis): die Sperre faellt Antwort fuer Antwort.
        let anzahl = aufloesung.len();
        for (i, (feld, wert)) in aufloesung.into_iter().enumerate() {
            schreibe(&d, feld, wert, true).await;
            let e = ergebnis(&d).await;
            if i + 1 < anzahl {
                assert_eq!(e["grund"], GRUND, "{name}: nach {feld} sperrt noch die zweite Ursache: {e}");
                assert!(!fragen_ids(&fragen(&d).await).iter().any(|i| i == feld), "{name}: {feld} steht nach der Antwort noch in der Liste");
            } else {
                assert_eq!(e["grund"], "bestaetigt", "{name}: nach der letzten Antwort faellt die Sperre nicht: {e}");
                assert!(zahl(&e).is_some(), "{name}: {e}");
            }
        }
        let ids = fragen_ids(&fragen(&d).await);
        assert!(!ids.iter().any(|i| i == ZEITRAUM || i == WEITERE), "{name}: nach der Aufloesung stehen die Fragen wieder in der Liste: {ids:?}");
    }
    // KONTROLLE 1: ohne Kreuz steht die bestaetigte Ursache nicht noch einmal in der Liste (die Zeile steht unter "Schon beantwortet").
    let d = fall_mit(&mit_unterhalt(vec![(ZEITRAUM, json!("01.01-30.06")), (WEITERE, json!(true))])).await;
    assert_eq!(ergebnis(&d).await["grund"], GRUND);
    let ids = fragen_ids(&fragen(&d).await);
    assert!(!ids.iter().any(|i| i == ZEITRAUM || i == WEITERE), "ohne Kreuz: die Antworten stehen noch einmal in der Liste: {ids:?}");
    // KONTROLLE 2: Kreuz und Betrag, aber keine Ursache: keine Sperre, keine Frage.
    let d = fall_mit(&mit_unterhalt(vec![kreuz.clone()])).await;
    assert_eq!(ergebnis(&d).await["grund"], "bestaetigt");
    let ids = fragen_ids(&fragen(&d).await);
    assert!(!ids.iter().any(|i| i == ZEITRAUM || i == WEITERE), "Kreuz ohne Ursache: {ids:?}");
}

/// MESSUNG (kein Beleg): die Zahl in Cent bei 15.000 EUR Unterhalt im ganzen Jahr, im halben Jahr (heute dieselbe Zahl) und bei dem
/// Betrag, den ein richtiges halbes Jahr abzoege (Hoechstbetrag 12.096 EUR halbiert = 6.048 EUR). Die Differenz ist die zu niedrige Steuer.
#[tokio::test]
#[ignore = "Messung fuer den Bericht, kein Beleg"]
async fn messung_teiljahr_euro() {
    let ohne = ergebnis(&fall_mit(&kegel_gesamt()).await).await;
    let voll = ergebnis(&fall_mit(&mit_unterhalt(vec![(ZEITRAUM, json!(GANZES_JAHR))])).await).await;
    let halb = ergebnis(&fall_mit(&mit_unterhalt(vec![(ZEITRAUM, json!("01.01-30.06"))])).await).await;
    let halb_richtig = ergebnis(&fall_mit(&mit(kegel_gesamt(), vec![(UNTERHALT, json!(604_800)), (ZEITRAUM, json!(GANZES_JAHR))])).await).await;
    let zu_zweit = ergebnis(&fall_mit(&mit_unterhalt(vec![(WEITERE, json!(true))])).await).await;
    println!("MESSUNG ohne Unterhalt: grund={} zahl_cent={}", ohne["grund"], ohne["zahl_cent"]);
    println!("MESSUNG 15.000 EUR, ganzes Jahr: grund={} zahl_cent={}", voll["grund"], voll["zahl_cent"]);
    println!("MESSUNG 15.000 EUR, 01.01-30.06: grund={} zahl_cent={}", halb["grund"], halb["zahl_cent"]);
    println!("MESSUNG 6.048 EUR, ganzes Jahr (richtiger Wert fuer das halbe Jahr): grund={} zahl_cent={}", halb_richtig["grund"], halb_richtig["zahl_cent"]);
    println!("MESSUNG 15.000 EUR, weitere Person ja: grund={} zahl_cent={}", zu_zweit["grund"], zu_zweit["zahl_cent"]);
    if let (Some(h), Some(r)) = (zahl(&halb), zahl(&halb_richtig)) {
        println!("MESSUNG Steuer zu niedrig im halben Jahr: {} Cent", r - h);
    }
}

/// MESSUNG (kein Beleg): steht die Frage im Fall "Kreuz `kein_unterhalt` = ja, gespeicherter Betrag, Teiljahr" in der Liste? Der gespeicherte
/// Wert gewinnt gegen das Kreuz (Entscheidung vom 2026-10-09), die Zeitraum-Frage koennte das Kreuz verbergen.
#[tokio::test]
#[ignore = "Messung fuer den Bericht, kein Beleg"]
async fn messung_kreuz_nein_neben_unterhalt() {
    for (name, extra) in [
        ("Kreuz gesetzt (keine Unterhaltszahlung), Betrag, halbes Jahr", vec![(UNTERHALT, json!(BETRAG)), (ZEITRAUM, json!("01.01-30.06")), ("kein_unterhalt", json!(true))]),
        ("Kreuz gesetzt, Betrag, weitere Person", vec![(UNTERHALT, json!(BETRAG)), (WEITERE, json!(true)), ("kein_unterhalt", json!(true))]),
        ("Kreuz gesetzt, Betrag, nichts weiter", vec![(UNTERHALT, json!(BETRAG)), ("kein_unterhalt", json!(true))]),
        ("Kreuz abgewaehlt (Unterhalt gezahlt), Betrag, halbes Jahr", vec![(UNTERHALT, json!(BETRAG)), (ZEITRAUM, json!("01.01-30.06")), ("kein_unterhalt", json!(false))]),
        ("Kreuz abgewaehlt, Betrag, nichts weiter", vec![(UNTERHALT, json!(BETRAG)), ("kein_unterhalt", json!(false))]),
        ("kein Kreuz, Betrag, halbes Jahr", vec![(UNTERHALT, json!(BETRAG)), (ZEITRAUM, json!("01.01-30.06"))]),
    ] {
        let d = fall_mit(&mit(kegel_gesamt(), extra)).await;
        let e = ergebnis(&d).await;
        let f = fragen(&d).await;
        let ids = fragen_ids(&f);
        let p33a: Vec<&String> = ids.iter().filter(|i| i.starts_with("p33a_") || i.as_str() == "kein_unterhalt").collect();
        let (_, stand) = sende(&d, "GET", "/fall/pt/stand", None).await;
        let mut regeln = Vec::new();
        for fid in [ZEITRAUM, WEITERE, "kein_unterhalt"] {
            let (_, frage) = sende(&d, "GET", &format!("/fall/pt/feld/{fid}/frage"), None).await;
            let regel = frage["frage"]["regel_id"].as_str().unwrap_or("?").to_owned();
            regeln.push(format!("{fid}: {regel} = {}", stand["relevanz"][&regel]["status"]));
        }
        println!(
            "MESSUNG [{name}]: grund={} zahl_cent={} Fragen {p33a:?}; Regelstatus {regeln:?}",
            e["grund"], e["zahl_cent"]
        );
    }
}
