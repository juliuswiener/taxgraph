//! d-53 (Julius, 2026-10-22, Option 3 "Beide Blöcke"): die Rentner-Ankreuzliste (`rentner_gesamt`) fragt zwei Zeilen zum Ehegatten vorweg,
//! `kein_kap_partner` ("Kapitalerträge des Ehegatten", fünf Fragen) und `kein_sonstige_partner` ("Rente oder sonstige Einkünfte des Ehegatten",
//! fünf Fragen). Ein Ehepaar, das beide mit "nein" beantwortet, beantwortet zehn Fragen weniger.
//!
//! Beide Kreuze stehen in `FLAG_NEGIERT` (`rust/konsistenz`). Darum ist ihr "nein" kein bloßes Verbergen wie bei `kein_lohn_pension_partner`
//! (Entscheidung 3 vom 2026-10-09, ein gespeicherter Wert zählt weiter): ein bestätigter Betrag des Ehegatten neben einer offenen oder mit
//! "keine" beantworteten Zeile sperrt mit `flag_konsistenz_offen`, bis der Nutzer die Zeile klärt. `kein_sonstige_partner` hatte zusätzlich eine
//! Sackgasse: `rente_instanz_offen` verlangte die vier Kernfelder der Rente des Ehegatten weiter, die das Kreuz verbirgt. Ließe man die Prüfung
//! unter dem Kreuz einfach weg, erreichte ein teilweise gespeicherter Rentenfall den Rechenkern (HTTP 500, `aa-Folgejahr 0 ohne fixierten
//! Rentenfreibetrag`); darum meldet `rente_partner` ihn unter dem Kreuz als Widerspruch (`flag_konsistenz_offen`).
//!
//! Gemessen wird über die HTTP-Schnittstelle, so wie die Oberfläche es sieht. Die Zahlen "wie vor dem Bau" sind am Stand `018a7d8f` gemessen
//! (ohne die Zeilen, gespeicherte Werte zählten mit).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::collections::BTreeSet;
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
async fn schreibe_roh(d: &Dienst, feld: &str, wert: Value, bestaetigt: bool) -> (u16, Value) {
    let (_, stand) = sende(d, "GET", "/fall/ek/stand", None).await;
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
    sende(d, "POST", "/fall/ek/event", Some(&rumpf)).await
}

async fn antworte(d: &Dienst, feld: &str, wert: Value) {
    let (status, antwort) = schreibe_roh(d, feld, wert, true).await;
    assert_eq!(status, 201, "POST {feld}: {antwort}");
}

async fn neuer_fall() -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "ek", "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    d
}

async fn fall_mit(paare: &[(&'static str, Value)]) -> Dienst {
    let d = neuer_fall().await;
    for (feld, wert) in paare {
        antworte(&d, feld, wert.clone()).await;
    }
    d
}

async fn ergebnis(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/ek/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

async fn fragen(d: &Dienst) -> Vec<Value> {
    let (status, antwort) = sende(d, "GET", "/fall/ek/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    antwort["fragen"].as_array().unwrap().clone()
}

async fn fragen_ids(d: &Dienst) -> Vec<String> {
    fragen(d).await.iter().map(|q| q["feld_id"].as_str().unwrap().to_owned()).collect()
}

/// Rentner mit allen Pflichtangaben des Kegels (wie `rentner_eingangsfrage::rentner`), Rente 40.000 EUR, OHNE die fuenf Kapital-Antworten
/// des Ehegatten (die Zeile `kein_kap_partner` ersetzt sie).
fn rentner(veranlagung: &'static str) -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(4_000_000)),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!(veranlagung)),
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
        ("kein_p23_verkauf", json!(true)),
    ]
}

fn mit(mut basis: Paare, mehr: Paare) -> Paare {
    basis.extend(mehr);
    basis
}

/// Die fuenf Kapital-Felder des Ehegatten hinter `kein_kap_partner`, einzeln benannt: eine entfernte Bedingung faellt an ihrem Feldnamen auf.
const KAP_BLOCK: [&str; 5] = [
    "kap_kapitalertraege_partner",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
];

/// Die fuenf Rentenfelder des Ehegatten hinter `kein_sonstige_partner`.
const RENTE_BLOCK: [&str; 5] = [
    "rentner_renten_art_partner",
    "rentner_jahresrente_partner",
    "rentner_renten_beginn_jahr_partner",
    "rentner_alter_bei_rentenbeginn_partner",
    "rentner_rentenfreibetrag_partner",
];

/// Kreuz und sein Block: die zwei neuen Zeilen.
const ZEILEN: [(&str, &[&str]); 2] = [("kein_kap_partner", &KAP_BLOCK), ("kein_sonstige_partner", &RENTE_BLOCK)];

/// Die vier Kernfelder der Rente des Ehegatten, die `rente_instanz_offen` zusammen verlangt (ohne den Freibetrag).
fn rente_voll() -> Paare {
    vec![
        ("rentner_renten_art_partner", json!("gesetzliche_rente")),
        ("rentner_jahresrente_partner", json!(1_000_000)),
        ("rentner_renten_beginn_jahr_partner", json!(2025)),
        ("rentner_alter_bei_rentenbeginn_partner", json!(65)),
    ]
}

fn kapital_gespeichert() -> Paare {
    vec![("kap_kapitalertraege_partner", json!(500_000))]
}

fn pos(ids: &[String], feld: &str) -> Option<usize> {
    ids.iter().position(|i| i == feld)
}

fn vorhandene<'a>(ids: &[String], felder: &[&'a str]) -> Vec<&'a str> {
    felder.iter().copied().filter(|f| ids.iter().any(|i| i == f)).collect()
}

fn fehlende<'a>(ids: &[String], felder: &[&'a str]) -> Vec<&'a str> {
    felder.iter().copied().filter(|f| !ids.iter().any(|i| i == f)).collect()
}

// ---------------------------------------------------------------- AK1: die Zeilen -----------------------------------------------

/// Die Zeilen stehen in der Fragenliste, so wie `gesamt` sie zeigt: Ja/Nein-Frage, `frage_invertiert`, `screening`. Sie stehen bei
/// Zusammenveranlagung und solange `veranlagung` offen ist (die Ankreuzliste erscheint vor der Frage nach der Veranlagung), und fehlen bei
/// Einzelveranlagung.
#[tokio::test]
async fn die_zeilen_des_ehegatten_stehen_in_der_liste_aber_nicht_bei_einzelveranlagung() {
    let liste = fragen(&neuer_fall().await).await;
    for (kreuz, _) in ZEILEN {
        let q = liste.iter().find(|q| q["feld_id"] == json!(kreuz)).unwrap_or_else(|| panic!("{kreuz} fehlt in der Fragenliste der Rentner-Scheibe"));
        assert_eq!(q["typ"], json!("bool"), "{kreuz}");
        assert_eq!(q["frage_invertiert"], json!(true), "{kreuz}: das Feld nennt die Abwesenheit");
        assert_eq!(q["screening"], json!(true), "{kreuz}: die Ankreuzliste zeigt es");
        assert!(q["fragetext_laie"].as_str().is_some_and(|t| !t.is_empty()), "{kreuz}: ohne Fragetext");
    }
    let einzel = fragen_ids(&fall_mit(&rentner("einzel")).await).await;
    let zusammen = fragen_ids(&fall_mit(&rentner("zusammen")).await).await;
    for (kreuz, _) in ZEILEN {
        assert!(pos(&einzel, kreuz).is_none(), "{kreuz} steht bei Einzelveranlagung");
        assert!(pos(&zusammen, kreuz).is_some(), "{kreuz} fehlt bei Zusammenveranlagung");
    }
}

/// "Nein" (`true`) verbirgt genau den Block der Zeile und die Zeile selbst, sonst nichts. "Ja" (`false`) laesst den Block stehen; der
/// Wechsel von "nein" auf "ja" zeigt ihn wieder.
#[tokio::test]
async fn nein_verbirgt_den_block_der_zeile_und_nur_ihn_ja_laesst_ihn_stehen() {
    for (kreuz, block) in ZEILEN {
        let d = fall_mit(&rentner("zusammen")).await;
        let vorher = fragen_ids(&d).await;
        assert!(fehlende(&vorher, block).is_empty(), "{kreuz}: vor der Antwort fehlen {:?}", fehlende(&vorher, block));
        assert!(pos(&vorher, kreuz).is_some(), "{kreuz} fehlt vor der Antwort");
        antworte(&d, kreuz, json!(true)).await;
        let nachher = fragen_ids(&d).await;
        assert_eq!(vorhandene(&nachher, block), Vec::<&str>::new(), "{kreuz}: trotz 'nein' gefragt");
        let weg: BTreeSet<&str> = vorher.iter().map(String::as_str).collect::<BTreeSet<_>>().difference(&nachher.iter().map(String::as_str).collect()).copied().collect();
        let soll: BTreeSet<&str> = block.iter().copied().chain([kreuz]).collect();
        assert_eq!(weg, soll, "{kreuz}: es verschwindet mehr oder weniger als der Block und die Zeile");

        let ja = fall_mit(&rentner("zusammen")).await;
        antworte(&ja, kreuz, json!(false)).await;
        assert!(fehlende(&fragen_ids(&ja).await, block).is_empty(), "{kreuz}: nach 'ja' fehlen Felder");
        antworte(&d, kreuz, json!(false)).await;
        assert!(fehlende(&fragen_ids(&d).await, block).is_empty(), "{kreuz}: nach dem Wechsel von 'nein' auf 'ja' fehlen Felder");
    }
}

/// Jede Zeile steht vor den Feldern, die sie abschaltet (sonst beantwortet der Nutzer die Details, bevor er die Zeile sieht).
#[tokio::test]
async fn jede_zeile_steht_vor_den_feldern_die_sie_abschaltet() {
    let ids = fragen_ids(&fall_mit(&rentner("zusammen")).await).await;
    for (kreuz, block) in ZEILEN {
        let k = pos(&ids, kreuz).unwrap_or_else(|| panic!("{kreuz} fehlt in der Liste"));
        for f in block {
            let p = pos(&ids, f).unwrap_or_else(|| panic!("{f} fehlt ohne Antwort auf {kreuz}"));
            assert!(k < p, "{kreuz}@{k} steht nicht vor {f}@{p}");
        }
    }
}

/// "Nein" auf beide Zeilen gibt die bestaetigte Zahl des Ehepaars ohne Einkuenfte des Ehegatten, ohne Zusatzfrage; sie ist die Zahl, die die
/// Antworten 0 auf die fuenf Kapital-Felder geben (vor dem Bau: 164.400 Cent).
#[tokio::test]
async fn nein_auf_beide_zeilen_gibt_die_zahl_ohne_einkuenfte_des_ehegatten() {
    let d = fall_mit(&rentner("zusammen")).await;
    let vorher = fragen_ids(&d).await.len();
    antworte(&d, "kein_kap_partner", json!(true)).await;
    antworte(&d, "kein_sonstige_partner", json!(true)).await;
    // Die zwei Zeilen und ihre zehn Felder verlassen die Liste; ohne die Zeilen (018a7d8f) standen die zehn Felder darin, mit ihnen und
    // ohne Antwort stehen zwei Zeilen mehr: ein Ehepaar, das beide verneint, beantwortet zehn Fragen weniger als vor dem Bau.
    let nachher = fragen_ids(&d).await.len();
    eprintln!("FRAGENZAHL Rentner-Ehepaar: Zeilen offen {vorher}, beide verneint {nachher}, vor dem Bau {}", vorher - 2);
    assert_eq!(vorher - nachher, 2 + KAP_BLOCK.len() + RENTE_BLOCK.len(), "Zeilen offen {vorher}, beide verneint {nachher}");
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], json!("bestaetigt"), "{e}");
    let mit_nullen = mit(
        rentner("zusammen"),
        KAP_BLOCK.iter().map(|f| (*f, json!(0))).collect(),
    );
    let referenz = ergebnis(&fall_mit(&mit_nullen).await).await;
    assert_eq!(e["zahl_cent"], referenz["zahl_cent"], "die Zeilen haben die Rechnung geaendert");
    assert_eq!(e["zahl_cent"], json!(164_400), "Zahl wie vor dem Bau (018a7d8f)");
}

// ---------------------------------------------------------------- AK2: gespeicherter Betrag des Ehegatten ----------------------

/// Zahlen am Stand `018a7d8f` gemessen (ohne die Zeilen): Rente 40.000 EUR, dazu Kapital des Ehegatten 5.000 EUR beziehungsweise seine Rente
/// 10.000 EUR. Mit der beantworteten Zeile "ja" muessen die Zahlen dieselben sein.
const ZAHL_MIT_KAPITAL_VOR_DEM_BAU: i64 = 235_200;
const ZAHL_MIT_RENTE_VOR_DEM_BAU: i64 = 364_600;

/// Ein bestaetigter Betrag des Ehegatten neben der offenen Zeile haelt die Zahl zurueck (`flag_konsistenz_offen`), die Zeile steht dann in
/// der Fragenliste; mit "ja" (der Ehegatte hat die Einkunft) rechnet der Betrag mit, und die Zahl ist die von vor dem Bau.
#[tokio::test]
async fn ein_gespeicherter_betrag_haelt_die_zahl_zurueck_bis_die_zeile_beantwortet_ist() {
    let faelle: [(&str, Paare, i64); 2] = [
        ("kein_kap_partner", kapital_gespeichert(), ZAHL_MIT_KAPITAL_VOR_DEM_BAU),
        ("kein_sonstige_partner", rente_voll(), ZAHL_MIT_RENTE_VOR_DEM_BAU),
    ];
    for (kreuz, werte, vor_dem_bau) in faelle {
        let d = fall_mit(&mit(rentner("zusammen"), werte)).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("flag_konsistenz_offen"), "{kreuz}, Zeile offen: {e}");
        assert!(e["zahl_cent"].is_null(), "{kreuz}, Zeile offen: trotz Sperre eine Zahl: {e}");
        assert!(e["klartext"].as_str().is_some_and(|t| t.contains("Partner")), "{kreuz}: der Klartext nennt den Partner nicht: {e}");
        assert!(pos(&fragen_ids(&d).await, kreuz).is_some(), "{kreuz}: die Sperre haelt die Zahl zurueck, aber die Zeile steht nicht in der Liste");

        antworte(&d, kreuz, json!(false)).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("bestaetigt"), "{kreuz} = ja: {e}");
        assert_eq!(e["zahl_cent"], json!(vor_dem_bau), "{kreuz} = ja: die Zahl weicht von der vor dem Bau ab: {e}");
    }
}

/// Der Weg hinaus ueber "Schon beantwortet", wie die Oberflaeche ihn prueft (`korrigiereBestaetigt`): `GET /feld/<kreuz>/frage` liefert die
/// Frage, und ihre Regel ist nicht `ausgeschlossen` (sonst waere die Belegt-Zeile tot).
async fn belegt_zeile_ist_erreichbar(d: &Dienst, kreuz: &str) {
    let (status, frage) = sende(d, "GET", &format!("/fall/ek/feld/{kreuz}/frage"), None).await;
    assert_eq!(status, 200, "{kreuz}: die Frage zur Zeile ist nicht erreichbar: {frage}");
    let regel = frage["frage"]["regel_id"].as_str().expect("keine regel_id").to_owned();
    let (_, stand) = sende(d, "GET", "/fall/ek/stand", None).await;
    assert_ne!(stand["relevanz"][&regel]["status"], json!("ausgeschlossen"), "{kreuz}: die Regel der Zeile ist abgeschaltet, die Belegt-Zeile waere tot: {stand}");
}

/// "Keine" neben einem bestaetigten Betrag ist ein Widerspruch: `flag_konsistenz_offen`. Die Zeile ist beantwortet und steht nicht mehr in
/// der Liste; der Weg hinaus ist die Belegt-Zeile: die Frage ist erreichbar, die neue Antwort gibt die Zahl von vor dem Bau.
#[tokio::test]
async fn keine_neben_einem_betrag_sperrt_und_die_zeile_ist_ueber_schon_beantwortet_erreichbar() {
    let faelle: [(&str, Paare, i64); 2] = [
        ("kein_kap_partner", kapital_gespeichert(), ZAHL_MIT_KAPITAL_VOR_DEM_BAU),
        ("kein_sonstige_partner", rente_voll(), ZAHL_MIT_RENTE_VOR_DEM_BAU),
    ];
    for (kreuz, werte, vor_dem_bau) in faelle {
        let d = fall_mit(&mit(rentner("zusammen"), werte)).await;
        antworte(&d, kreuz, json!(true)).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("flag_konsistenz_offen"), "{kreuz} = keine neben einem Betrag: {e}");
        assert!(e["zahl_cent"].is_null(), "{kreuz}: trotz Sperre eine Zahl: {e}");
        belegt_zeile_ist_erreichbar(&d, kreuz).await;

        antworte(&d, kreuz, json!(false)).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["zahl_cent"], json!(vor_dem_bau), "{kreuz}: Weg hinaus: {e}");
    }
}

/// Nur ein BESTAETIGTER Betrag zaehlt. Ein vorlaeufiger Betrag (Vorschlag der KI oder des Vorjahrs) und ein Betrag 0 sperren nicht. Beim Block
/// der Rente ist der Betrag 0 der Freibetrag (`rentner_rentenfreibetrag_partner`): die vier Kernfelder zaehlen als Rentenfall, nicht als Betrag
/// (`ein_teilweise_gespeicherter_rentenfall_hinter_dem_kreuz_ist_ein_widerspruch_keine_sackgasse`).
#[tokio::test]
async fn ein_vorlaeufiger_betrag_und_ein_betrag_null_sperren_nicht() {
    let faelle = [
        ("kein_kap_partner", "kap_kapitalertraege_partner", "kap_kapitalertraege_partner"),
        ("kein_sonstige_partner", "rentner_jahresrente_partner", "rentner_rentenfreibetrag_partner"),
    ];
    for (kreuz, feld, feld_null) in faelle {
        let d = fall_mit(&rentner("zusammen")).await;
        let (status, antwort) = schreibe_roh(&d, feld, json!(500_000), false).await;
        assert_eq!(status, 201, "{feld} vorlaeufig: {antwort}");
        let e = ergebnis(&d).await;
        assert_ne!(e["grund"], json!("flag_konsistenz_offen"), "{kreuz}: ein vorlaeufiger Betrag sperrt: {e}");
        antworte(&d, kreuz, json!(true)).await;
        let e = ergebnis(&d).await;
        assert_ne!(e["grund"], json!("flag_konsistenz_offen"), "{kreuz} = keine neben einem vorlaeufigen Betrag sperrt: {e}");

        let d = fall_mit(&mit(rentner("zusammen"), vec![(feld_null, json!(0))])).await;
        antworte(&d, kreuz, json!(true)).await;
        let e = ergebnis(&d).await;
        assert_ne!(e["grund"], json!("flag_konsistenz_offen"), "{kreuz} = keine neben dem Betrag 0 sperrt: {e}");
    }
}

// ---------------------------------------------------------------- AK3: rente_instanz_offen ohne Sackgasse -------------------------

/// `kein_sonstige_partner` = ja verbirgt die Rentenfelder des Ehegatten. Waren davon schon 1 bis 3 der vier Kernfelder gespeichert, sperrte
/// `rente_instanz_offen` weiter (Sackgasse: die verlangten Felder stehen nicht in der Liste), und ohne Sperre liefe der Teilfall in den
/// Rechenkern (HTTP 500). Jetzt meldet `rente_partner` ihn unter dem Kreuz als Widerspruch: `flag_konsistenz_offen`. Kontrolle: ohne das
/// Kreuz sperrt weiter `rente_instanz_offen` und die Felder stehen in der Liste (der Ausweg heute). Der Weg hinaus unter dem Kreuz ist die
/// Zeile (Ausnahme `AUSWEG_KREUZ` im Sperre-Sweep): ihre Frage ist erreichbar, mit "ja" kommen die Felder zurueck, und nach den fehlenden
/// Antworten ist die Zahl die von vor dem Bau.
#[tokio::test]
async fn ein_teilweise_gespeicherter_rentenfall_hinter_dem_kreuz_ist_ein_widerspruch_keine_sackgasse() {
    let kern = ["rentner_alter_bei_rentenbeginn_partner", "rentner_renten_art_partner", "rentner_renten_beginn_jahr_partner"];
    for teil in 1..=kern.len() {
        let (gespeichert, rest): (Paare, Paare) = rente_voll().into_iter().partition(|(f, _)| kern[..teil].contains(f));
        let d = fall_mit(&mit(rentner("zusammen"), gespeichert)).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("rente_instanz_offen"), "{teil} Kernfelder, Zeile offen (Kontrolle): {e}");
        assert!(!vorhandene(&fragen_ids(&d).await, &RENTE_BLOCK).is_empty(), "{teil} Kernfelder: die Sperre verlangt Felder, die nicht in der Liste stehen");

        antworte(&d, "kein_sonstige_partner", json!(true)).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("flag_konsistenz_offen"), "{teil} Kernfelder, Kreuz = keine: {e}");
        assert!(e["zahl_cent"].is_null(), "{teil} Kernfelder, Kreuz = keine: trotz Sperre eine Zahl: {e}");
        assert_eq!(vorhandene(&fragen_ids(&d).await, &RENTE_BLOCK), Vec::<&str>::new(), "{teil} Kernfelder: unter dem Kreuz stehen Rentenfelder in der Liste");
        belegt_zeile_ist_erreichbar(&d, "kein_sonstige_partner").await;

        antworte(&d, "kein_sonstige_partner", json!(false)).await;
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("rente_instanz_offen"), "{teil} Kernfelder, Kreuz zurueck auf ja: {e}");
        assert!(!vorhandene(&fragen_ids(&d).await, &RENTE_BLOCK).is_empty(), "{teil} Kernfelder: nach dem Wechsel auf 'ja' fehlen die Felder");
        for (feld, wert) in rest {
            antworte(&d, feld, wert).await;
        }
        let e = ergebnis(&d).await;
        assert_eq!(e["grund"], json!("bestaetigt"), "{teil} Kernfelder, nach den fehlenden Antworten: {e}");
        assert_eq!(e["zahl_cent"], json!(ZAHL_MIT_RENTE_VOR_DEM_BAU), "{teil} Kernfelder: Weg hinaus: {e}");
    }
}

/// GRENZE (dokumentiert, nicht behoben; d-53 gilt nur fuer die Listenmitgliedschaft, nichts unter `sperre/` darueber hinaus): alle vier
/// Kernfelder gespeichert, der Beginn liegt vor dem Veranlagungsjahr (2020), der Freibetrag fehlt, danach folgt das Kreuz "keine". Dann sperrt
/// `rentenfreibetrag_fixierung_offen` (`beginn_grund`) und verlangt den Freibetrag, den das Kreuz verbirgt. Der Zustand entsteht nur durch eine
/// Rueckkorrektur (im Normalweg steht das Kreuz vor den Detailfragen). Der Ausweg ist das Kreuz selbst: mit "ja" steht der Freibetrag in der
/// Liste, mit seiner Antwort kommt die Zahl. Wird die Grenze behoben (`beginn_grund` unter dem Kreuz), wird dieser Test rot und gehoert angepasst.
#[tokio::test]
async fn grenze_vier_kernfelder_ohne_freibetrag_hinter_dem_kreuz_sperren_mit_dem_verborgenen_feld() {
    let kern = vec![
        ("rentner_renten_art_partner", json!("gesetzliche_rente")),
        ("rentner_jahresrente_partner", json!(0)),
        ("rentner_renten_beginn_jahr_partner", json!(2020)),
        ("rentner_alter_bei_rentenbeginn_partner", json!(65)),
    ];
    let d = fall_mit(&mit(rentner("zusammen"), kern)).await;
    antworte(&d, "kein_sonstige_partner", json!(true)).await;
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], json!("rentenfreibetrag_fixierung_offen"), "Grenze: {e}");
    assert!(e["zahl_cent"].is_null(), "Grenze: trotz Sperre eine Zahl: {e}");
    assert!(pos(&fragen_ids(&d).await, "rentner_rentenfreibetrag_partner").is_none(), "Grenze: der verlangte Freibetrag steht unter dem Kreuz in der Liste");
    belegt_zeile_ist_erreichbar(&d, "kein_sonstige_partner").await;

    antworte(&d, "kein_sonstige_partner", json!(false)).await;
    assert!(pos(&fragen_ids(&d).await, "rentner_rentenfreibetrag_partner").is_some(), "Ausweg: nach 'ja' fehlt der Freibetrag in der Liste");
    antworte(&d, "rentner_rentenfreibetrag_partner", json!(0)).await;
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], json!("bestaetigt"), "Ausweg: nach der Antwort auf den Freibetrag: {e}");
    assert!(e["zahl_cent"].is_i64(), "Ausweg: keine Zahl: {e}");
}

/// Ein vollstaendig gespeicherter Rentenfall mit Jahresrente 0 und Beginn im Veranlagungsjahr (kein Freibetrag noetig) ist kein Widerspruch:
/// das Kreuz "keine" gibt die Zahl.
#[tokio::test]
async fn eine_vollstaendig_gespeicherte_rente_von_null_hinter_dem_kreuz_sperrt_nicht() {
    let rente_null = vec![
        ("rentner_renten_art_partner", json!("gesetzliche_rente")),
        ("rentner_jahresrente_partner", json!(0)),
        ("rentner_renten_beginn_jahr_partner", json!(2025)),
        ("rentner_alter_bei_rentenbeginn_partner", json!(65)),
    ];
    let d = fall_mit(&mit(rentner("zusammen"), rente_null)).await;
    antworte(&d, "kein_sonstige_partner", json!(true)).await;
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], json!("bestaetigt"), "{e}");
    assert_eq!(e["zahl_cent"], json!(164_400), "{e}");
}

// ---------------------------------------------------------------- AK4: Rueckkorrektur -------------------------------

/// Rueckkorrektur (AK4) mit gespeichertem Betrag: "keine" neben einer Antwort, die der Nutzer schon gegeben hat, haelt die Zahl zurueck
/// und loest sich wieder, wenn die Zeile auf "ja" zurueckgenommen wird (die Zahl dann die von vor dem Bau). Dasselbe gilt beim Wechsel
/// der Veranlagung von zusammen auf einzeln: die Zeilen des Ehegatten verlassen die Liste, und es gilt der Grund des Ehegatten-Falls von
/// vor dem Bau.
#[tokio::test]
async fn die_rueckkorrektur_einer_zeile_und_der_veranlagung_gibt_die_zahl_wieder() {
    let d = fall_mit(&mit(rentner("zusammen"), rente_voll())).await;
    let vorher = ergebnis(&d).await;
    assert_eq!(vorher["grund"], json!("flag_konsistenz_offen"), "Zeile offen, Rente des Ehegatten gespeichert: {vorher}");
    assert!(vorher["zahl_cent"].is_null(), "Zeile offen: Zahl trotz Sperre: {vorher}");
    antworte(&d, "kein_sonstige_partner", json!(true)).await;
    assert_eq!(ergebnis(&d).await["grund"], json!("flag_konsistenz_offen"), "Zeile auf keine: {vorher}");
    antworte(&d, "kein_sonstige_partner", json!(false)).await;
    // Die Zahl vor dem Bau (018a7d8f, dort ohne Zeile und ohne Sperre gerechnet): die Rueckkorrektur muss genau sie geben.
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], json!("bestaetigt"), "Rueckkorrektur auf ja: {e}");
    assert_eq!(e["zahl_cent"], json!(ZAHL_MIT_RENTE_VOR_DEM_BAU), "Rueckkorrektur: {e}");

    // Wechsel der Veranlagung: die Zeilen des Ehegatten verlassen die Liste, die Flag-Sperre des Ehepaars bleibt nicht stehen. Uebernimmt
    // die gespeicherten Werte des Ehegatten bei Einzelveranlagung, ist der Grund `partner_konsistenz_offen` (wie vor dem Bau, gemessen
    // am Stand 018a7d8f) — nicht die neue Sperre.
    let (status, _) = schreibe_roh(&d, "veranlagung", json!("einzel"), true).await;
    assert_eq!(status, 201, "POST veranlagung = einzel: {status}");
    let ids = fragen_ids(&d).await;
    assert!(vorhandene(&ids, &[ZEILEN[0].0, ZEILEN[1].0]).is_empty(), "die Zeilen des Ehegatten stehen bei Einzelveranlagung noch in der Liste: {ids:?}");
    assert_eq!(ergebnis(&d).await["grund"], json!("partner_konsistenz_offen"), "Einzelveranlagung mit gespeicherter Ehegatten-Rente");
}
