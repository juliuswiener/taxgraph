//! d-54 (Julius, 2026-10-22): wer in `gesamt` ("Arbeitnehmer, Vermieter oder Kapital") "keine Kinder" ankreuzt (`kein_kind` = ja), obwohl
//! bestaetigte Angaben zu Kindern gespeichert sind, bekommt keine Zahl (`flag_konsistenz_offen`), sondern die Rueckfrage. Vorher rechnete die
//! App die gespeicherten Kinder still weiter: zu wenig Steuer (Sweep `korrektur_sweep`, zwei Kinder mit Betreuungskosten: -2.345 EUR; Schulgeld
//! allein -741 EUR bei 150.000 EUR Lohn).
//!
//! Nicht betroffen: `rentner_gesamt` (Entscheidung 3 vom 2026-10-09, ein gespeicherter Wert zaehlt weiter) und ein Kreuz, das nicht bestaetigt
//! "ja" ist. Gemessen wird ueber die HTTP-Schnittstelle, so wie die Oberflaeche es sieht.
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

/// Schreibt eine Antwort des Nutzers. `bestaetigt = false` schreibt sie vorlaeufig (ohne zweites Signal), wie ein KI-Vorschlag. Hat das Feld
/// schon ein aktives Ereignis, ersetzt die Antwort es (wie die Oberflaeche).
async fn schreibe(d: &Dienst, feld: &str, wert: Value, bestaetigt: bool) {
    let (_, stand) = sende(d, "GET", "/fall/kk/stand", None).await;
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
    let (status, antwort) = sende(d, "POST", "/fall/kk/event", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST {feld}: {antwort}");
}

async fn antworte(d: &Dienst, feld: &str, wert: Value) {
    schreibe(d, feld, wert, true).await;
}

async fn fall_mit(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "kk", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        antworte(&d, feld, wert.clone()).await;
    }
    d
}

async fn ergebnis(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/kk/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Pflichtfelder der Scheibe `gesamt` (der Kegel), Einzelveranlagung, Lohn 150.000 EUR (der Kinderfreibetrag gewinnt dann gegen das
/// Kindergeld), sonst neutral.
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

/// Pflichtfelder der Scheibe `rentner_gesamt` (wie `kette_endstand_hermetisch.rs::kegel_rentner`).
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

/// Die Angaben zu Kindern, die bei `kein_kind` = ja trotzdem rechnen oder den Fall als "mit Kindern" ausweisen: Feld, Wert, Beschreibung.
fn kind_angaben() -> Vec<(&'static str, Value, &'static str)> {
    vec![
        ("fam_anzahl_kinder", json!(2), "zwei Kinder"),
        ("p33a_ausbildung_anzahl_kinder", json!(1), "ein Kind in Ausbildung"),
        ("schulgeld", json!(600_000), "Schulgeld"),
        ("kinderbetreuungskosten", json!(400_000), "Kinderbetreuungskosten (erstes Kind)"),
        ("kinderbetreuungskosten__2", json!(300_000), "Kinderbetreuungskosten (zweites Kind, Instanz)"),
        ("kind_kv", json!(100_000), "Krankenversicherung des Kindes"),
        ("kind_pv", json!(50_000), "Pflegeversicherung des Kindes"),
    ]
}

/// AK2: `kein_kind` = ja neben jeder der gespeicherten Kind-Angaben: keine Zahl, Grund `flag_konsistenz_offen`, ein Klartext. Positivkontrolle in
/// derselben Schleife: dieselbe Angabe mit `kein_kind` = nein gibt eine Zahl (ein Test, der nur "keine Zahl" prueft, bliebe auch bei einem
/// unvollstaendigen Kegel gruen).
#[tokio::test]
async fn kein_kind_ja_neben_gespeicherten_kind_angaben_sperrt_in_gesamt() {
    for (feld, wert, was) in kind_angaben() {
        let d = fall_mit("gesamt", &mit(kegel_gesamt(), vec![(feld, wert.clone()), ("kein_kind", json!(true))])).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("flag_konsistenz_offen"), "{was}: {e}");
        assert!(e["zahl_cent"].is_null(), "{was}: trotz Sperre eine Zahl: {e}");
        assert!(e["klartext"].as_str().is_some_and(|t| t.contains("keine Kinder")), "{was}: der Klartext nennt die Kinder nicht: {e}");

        let d = fall_mit("gesamt", &mit(kegel_gesamt(), vec![(feld, wert), ("kein_kind", json!(false))])).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("bestaetigt"), "{was} mit kein_kind = nein: Positivkontrolle {e}");
        assert!(e["zahl_cent"].as_i64().is_some(), "{was} mit kein_kind = nein: keine Zahl {e}");
    }
}

/// AK2/AK4: Die Sperre greift nur bei bestaetigtem "ja" UND bestaetigtem Betrag > 0. Ein unbeantwortetes oder vorlaeufiges Kreuz, ein vorlaeufiger
/// Betrag und die Anzahl 0 sperren nicht (wie die Sperre der sieben anderen Kreuze: Schweigen und Vorschlaege urteilen nicht).
#[tokio::test]
async fn nur_bestaetigtes_ja_mit_bestaetigtem_betrag_sperrt() {
    let ohne_kreuz = fall_mit("gesamt", &mit(kegel_gesamt(), vec![("schulgeld", json!(600_000))])).await;
    let e = ergebnis(&ohne_kreuz).await;
    assert_eq!(e["grund"], json!("bestaetigt"), "unbeantwortetes Kreuz: {e}");

    let vorlaeufiges_kreuz = fall_mit("gesamt", &mit(kegel_gesamt(), vec![("schulgeld", json!(600_000))])).await;
    schreibe(&vorlaeufiges_kreuz, "kein_kind", json!(true), false).await;
    let e = ergebnis(&vorlaeufiges_kreuz).await;
    assert_ne!(e["grund"], json!("flag_konsistenz_offen"), "vorlaeufiges Kreuz sperrt: {e}");

    let vorlaeufiger_betrag = fall_mit("gesamt", &kegel_gesamt()).await;
    schreibe(&vorlaeufiger_betrag, "schulgeld", json!(600_000), false).await;
    antworte(&vorlaeufiger_betrag, "kein_kind", json!(true)).await;
    let e = ergebnis(&vorlaeufiger_betrag).await;
    assert_ne!(e["grund"], json!("flag_konsistenz_offen"), "vorlaeufiger Betrag sperrt: {e}");

    let anzahl_null = fall_mit("gesamt", &mit(kegel_gesamt(), vec![("fam_anzahl_kinder", json!(0)), ("schulgeld", json!(0)), ("kein_kind", json!(true))])).await;
    let e = ergebnis(&anzahl_null).await;
    assert_eq!(e["grund"], json!("bestaetigt"), "Anzahl 0 und Schulgeld 0: {e}");
    assert!(e["zahl_cent"].as_i64().is_some(), "Anzahl 0 und Schulgeld 0: keine Zahl {e}");
}

/// AK4: `rentner_gesamt` bleibt unberuehrt. Ein gespeicherter Wert zaehlt trotz "keine Kinder" weiter (Entscheidung 3 vom 2026-10-09, Test
/// `rentner_eingangsfrage::ein_gespeicherter_wert_zaehlt_trotz_nein_weiter`); die Rentner-Liste hat das Kreuz seit Fluss J.
#[tokio::test]
async fn rentner_gesamt_rechnet_gespeicherte_kinder_weiter() {
    for (feld, wert, was) in kind_angaben() {
        let d = fall_mit("rentner_gesamt", &mit(kegel_rentner(), vec![(feld, wert), ("kein_kind", json!(true))])).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("bestaetigt"), "Rentner mit {was}: {e}");
        assert!(e["zahl_cent"].as_i64().is_some(), "Rentner mit {was}: keine Zahl {e}");
    }
}

/// AK1 (Ausweg auf Dienst-Ebene): Das Kreuz bleibt in der Sperre erreichbar, so wie die Oberflaeche es prueft (`korrigiereBestaetigt`):
/// `GET /feld/kein_kind/frage` liefert die Frage, ihre Regel ist nicht `ausgeschlossen`. Zwei Wege hinaus: (A) das Kreuz auf "nein" (die Kinder
/// gelten), die Zahl entspricht dem Fall, der von Anfang an "nein" sagte; (B) die Kinder entfernen (Kreuz auf nein, Angaben auf 0, Kreuz auf ja),
/// die Zahl entspricht dem Fall ohne Kinder. Beide Zahlen sind der Cent-Vergleich gegen den frischen Fall.
#[tokio::test]
async fn die_sperre_laesst_zwei_wege_hinaus_und_beide_rechnen_wie_ein_frischer_fall() {
    let kinder = vec![("fam_anzahl_kinder", json!(2)), ("schulgeld", json!(600_000)), ("p33a_ausbildung_anzahl_kinder", json!(1))];

    let frisch_mit = fall_mit("gesamt", &mit(kegel_gesamt(), mit(kinder.clone(), vec![("kein_kind", json!(false))]))).await;
    let zahl_mit_kindern = ergebnis(&frisch_mit).await["zahl_cent"].as_i64().expect("frisch mit Kindern: keine Zahl");
    let frisch_ohne = fall_mit("gesamt", &mit(kegel_gesamt(), vec![("kein_kind", json!(true))])).await;
    let zahl_ohne_kinder = ergebnis(&frisch_ohne).await["zahl_cent"].as_i64().expect("frisch ohne Kinder: keine Zahl");
    assert!(zahl_mit_kindern < zahl_ohne_kinder, "die Kinder wirken nicht: {zahl_mit_kindern} gegen {zahl_ohne_kinder} (der Test misst sonst nichts)");

    // Weg A.
    let d = fall_mit("gesamt", &mit(kegel_gesamt(), mit(kinder.clone(), vec![("kein_kind", json!(true))]))).await;
    assert_eq!(ergebnis(&d).await["grund"], json!("flag_konsistenz_offen"));
    let (status, frage) = sende(&d, "GET", "/fall/kk/feld/kein_kind/frage", None).await;
    assert_eq!(status, 200, "die Frage zum Kreuz ist nicht erreichbar: {frage}");
    let regel = frage["frage"]["regel_id"].as_str().expect("keine regel_id").to_owned();
    let (_, stand) = sende(&d, "GET", "/fall/kk/stand", None).await;
    assert_ne!(stand["relevanz"][&regel]["status"], json!("ausgeschlossen"), "die Regel des Kreuzes ist abgeschaltet: die Belegt-Zeile waere tot {stand}");
    antworte(&d, "kein_kind", json!(false)).await;
    let e = ergebnis(&d).await;
    assert_eq!(e["zahl_cent"].as_i64(), Some(zahl_mit_kindern), "Weg A: {e}");

    // Weg B: Kreuz auf "nein" (die Kinder-Felder kommen zurueck), die Angaben auf 0, das Kreuz wieder auf "ja".
    let d = fall_mit("gesamt", &mit(kegel_gesamt(), mit(kinder, vec![("kein_kind", json!(true))]))).await;
    antworte(&d, "kein_kind", json!(false)).await;
    for feld in ["fam_anzahl_kinder", "schulgeld", "p33a_ausbildung_anzahl_kinder"] {
        antworte(&d, feld, json!(0)).await;
    }
    antworte(&d, "kein_kind", json!(true)).await;
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], json!("bestaetigt"), "Weg B: {e}");
    assert_eq!(e["zahl_cent"].as_i64(), Some(zahl_ohne_kinder), "Weg B: {e}");
}
