//! Fragen-Reduktion Teil 2, Phase 1 (B1): Die Konfession kommt zuerst, die Zaehler der Haushaltshilfen kommen nach ihrem "ja"
//! (Bindungsart `feld_bedingung.beleg`, Rust-eigen). Ohne `PARITY=1`, ohne Python, ohne Netz.
//!
//! **Worum es geht.** Wer keiner Kirche angehoert, beantwortete bisher die Frage nach der Kirchensteuer des Arbeitgebers und
//! die nach dem Bundesland, bevor die Konfession kam. Wer keine Haushaltshilfe hat, nannte drei Zahlen (Handwerker,
//! Dienstleistungen, Minijobs), bevor die Frage "Hattest du Aufwendungen?" kam. Jetzt kommt die Konfession zuerst, und die
//! Fragen, die ohne "ja" oder ohne Kirche gegenstandslos sind, fehlen in der Liste.
//!
//! **Warum es zaehlt.** Das spart dem Einzelnutzer ohne Kirche fuenf Fragen, jedem anderen drei. Keine Steuer aendert sich
//! (`korrektur_sweep::messung_fragenzahl_und_steuer`, `--ignored`). Umgekehrt darf der Beleg nie eine Frage verstecken, die
//! die Sperre `input_kegel_nicht_bestaetigt` braucht: der Arbeitsweg (`ep_*`) und `agb_notwendig_angemessen` bleiben stehen.
//!
//! **Wo es sitzt.** `rust/interview/src/fragen.rs::feld_ausgeschlossen` (die Auswertung), `rust/bindung/src/bindung_datei.rs`
//! (`FeldBedingung::beleg`), `rust/bindung/daten/bindung_p51a_kirchensteuer.yaml` und `bindung_screening_ausgaben.yaml`.
//!
//! **Belegt wird (positiver Beleg).** Anders als `wert` und `wert_nicht` schliesst `beleg: true` auch bei SCHWEIGEN aus: das
//! Feld bleibt nur, wenn seine Bedingung BESTAETIGT erfuellt ist. Ein vorlaeufiger Wert (Vorjahr, KI) belegt nichts; die
//! Eingangsfrage selbst bleibt in der Liste und schaltet das Feld frei.
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

const KONF: &str = "kist_konfession";
const BUNDESLAND: &str = "kist_bundesland";
const ARBEITGEBER: &str = "kirchensteuer_arbeitgeber";
const HH_JA_NEIN: &str = "hh_hat_aufwendungen";
const ZAEHLER: [&str; 3] = ["hh_anzahl_dienstleistungen", "hh_anzahl_handwerker", "hh_anzahl_minijobs"];
/// Der Arbeitsweg und die Frage nach den aussergewoehnlichen Belastungen: Pflichtfelder des Kegels. Sie bleiben in der Liste.
const PFLICHT_BLEIBT: [&str; 5] = ["ep_arbeitstage", "ep_entfernung_km", "ep_eigenes_kfz", "ep_oepnv_kosten", "agb_notwendig_angemessen"];

struct Dienst {
    zustand: Zustand,
    token: String,
    /// Haelt das Verzeichnis des Falls am Leben, solange der Dienst laeuft.
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

/// Ein Ereignis, bestaetigt (zwei Signale) oder vorlaeufig (nur das erste, `signal_2` leer).
fn ereignis(feld_id: &str, wert: &Value, bestaetigt: bool) -> Value {
    let signal_2 = if bestaetigt { json!(format!("ok@{feld_id}")) } else { Value::Null };
    json!({
        "feld_id": feld_id,
        "herkunft": {"haftung": "nutzer", "herkunft": "laie", "pruef_tiefe": "ungeprueft"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": signal_2},
        "wert": wert,
        "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

/// Eine Antwort `(Feld, Wert)`, bestaetigt.
fn ja<'a>(feld: &'a str, wert: &Value) -> (&'a str, Value, bool) {
    (feld, wert.clone(), true)
}

/// Eine Antwort `(Feld, Wert)`, nur vorlaeufig (ein Vorschlag aus dem Vorjahr oder von der KI).
fn vorschlag<'a>(feld: &'a str, wert: &Value) -> (&'a str, Value, bool) {
    (feld, wert.clone(), false)
}

/// Legt den Fall `bg` der Scheibe `gesamt` (VZ 2025) an, schreibt jedes Paar ueber die echte Route `POST /event` und gibt die
/// Fragenliste zurueck, so wie die Oberflaeche sie sieht.
async fn fragen_nach(paare: &[(&str, Value, bool)]) -> Vec<Value> {
    let d = dienst();
    let kopf = json!({"fall_id": "bg", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert, bestaetigt) in paare {
        let (status, antwort) = sende(&d, "POST", "/fall/bg/event", Some(&ereignis(feld, wert, *bestaetigt))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, fragen) = sende(&d, "GET", "/fall/bg/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"].as_array().unwrap().clone()
}

fn ids(liste: &[Value]) -> Vec<&str> {
    liste.iter().map(|q| q["feld_id"].as_str().unwrap()).collect()
}

fn pos(ids: &[&str], feld: &str) -> Option<usize> {
    ids.iter().position(|i| *i == feld)
}

/// Welche der `felder` stehen in der Liste?
fn davon<'a>(ids: &[&str], felder: &[&'a str]) -> Vec<&'a str> {
    felder.iter().copied().filter(|f| ids.contains(f)).collect()
}

/// Die ersten Eintraege der Liste, fuer die Fehlermeldung.
fn kopf(ids: &[&str]) -> String {
    format!("{:?}", &ids[..ids.len().min(40)])
}

/// AK1 (Konfession zuerst): Im leeren Fall steht die Konfession in der Liste, und weder das Bundesland noch die beiden
/// Kirchensteuer-Fragen aus dem Abschnitt "Kirchensteuer" (`kist_gezahlt`, `kist_erstattet`) stehen vor ihr. Die Kirchensteuer des
/// Arbeitgebers fehlt, bis eine Konfession bestaetigt ist. Auf `86ac3643` stehen alle davor und der Arbeitgeber in der Liste.
#[tokio::test]
async fn die_konfession_steht_vor_bundesland_und_kirchensteuer_fragen() {
    for (name, paare) in [
        ("leerer Fall", vec![]),
        ("Einzelveranlagung", vec![ja("veranlagung", &json!("einzel"))]),
    ] {
        let liste = fragen_nach(&paare).await;
        let ids = ids(&liste);
        let k = pos(&ids, KONF).unwrap_or_else(|| panic!("{name}: die Konfession fehlt in der Liste {}", kopf(&ids)));
        // KONTROLLE: Die Felder, die hinter der Konfession stehen sollen, stehen ueberhaupt in der Liste.
        assert!(pos(&ids, "kist_gezahlt").is_some(), "{name}: Kontrolle, `kist_gezahlt` fehlt: {}", kopf(&ids));
        for dahinter in [BUNDESLAND, "kist_gezahlt", "kist_erstattet"] {
            if let Some(p) = pos(&ids, dahinter) {
                assert!(k < p, "{name}: {dahinter} (Stelle {p}) steht vor der Konfession (Stelle {k}): {}", kopf(&ids));
            }
        }
        assert!(!ids.contains(&ARBEITGEBER), "{name}: `{ARBEITGEBER}` steht vor der bestaetigten Konfession in der Liste");
    }
}

/// AK1: "keine" nimmt das Bundesland und die Kirchensteuer des Arbeitgebers aus der Liste. Bei Zusammenveranlagung gilt das
/// erst, wenn auch der Ehegatte "keine" bestaetigt hat (Entscheidung d-43: das Bundesland haengt an A ODER B).
#[tokio::test]
async fn keine_konfession_nimmt_bundesland_und_arbeitgeber_kirchensteuer_aus_der_liste() {
    let einzel = fragen_nach(&[ja("veranlagung", &json!("einzel")), ja(KONF, &json!("keine"))]).await;
    let einzel = ids(&einzel);
    assert!(!einzel.contains(&BUNDESLAND), "Einzel, keine: Bundesland steht in der Liste: {}", kopf(&einzel));
    assert!(!einzel.contains(&ARBEITGEBER), "Einzel, keine: Arbeitgeber-Kirchensteuer steht in der Liste");

    // Zusammenveranlagung: A "keine", B noch offen -> das Bundesland bleibt (B koennte einer Kirche angehoeren).
    let offen = fragen_nach(&[ja("veranlagung", &json!("zusammen")), ja(KONF, &json!("keine"))]).await;
    let offen = ids(&offen);
    assert!(offen.contains(&"kist_konfession_partner"), "Kontrolle: die Konfession des Ehegatten steht in der Liste");
    assert!(!offen.contains(&ARBEITGEBER), "A keine: Arbeitgeber-Kirchensteuer steht in der Liste");
    // A "keine", B "keine" -> beides entfaellt.
    let beide = fragen_nach(&[
        ja("veranlagung", &json!("zusammen")),
        ja(KONF, &json!("keine")),
        ja("kist_konfession_partner", &json!("keine")),
    ])
    .await;
    let beide = ids(&beide);
    assert!(!beide.contains(&BUNDESLAND), "A und B keine: Bundesland steht in der Liste: {}", kopf(&beide));
    // A "keine", B "evangelisch" -> das Bundesland bleibt (d-43).
    let nur_b = fragen_nach(&[
        ja("veranlagung", &json!("zusammen")),
        ja(KONF, &json!("keine")),
        ja("kist_konfession_partner", &json!("evangelisch")),
    ])
    .await;
    assert!(ids(&nur_b).contains(&BUNDESLAND), "A keine, B evangelisch: das Bundesland fehlt (d-43)");
}

/// AK1: Mit einer Konfession bleiben das Bundesland und die Kirchensteuer des Arbeitgebers in der Liste.
#[tokio::test]
async fn mit_konfession_bleiben_bundesland_und_arbeitgeber_kirchensteuer_in_der_liste() {
    for konfession in ["evangelisch", "roemisch-katholisch"] {
        let liste = fragen_nach(&[ja("veranlagung", &json!("einzel")), ja(KONF, &json!(konfession))]).await;
        let ids = ids(&liste);
        assert!(ids.contains(&BUNDESLAND), "{konfession}: Bundesland fehlt: {}", kopf(&ids));
        assert!(ids.contains(&ARBEITGEBER), "{konfession}: Arbeitgeber-Kirchensteuer fehlt: {}", kopf(&ids));
    }
}

/// AK3 (Partner-Variante): Die Kirchensteuer des Arbeitgebers des Ehegatten folgt derselben Regel wie die von Person A. Sie steht
/// erst, wenn die Konfession des Ehegatten bestaetigt ist und nicht "keine" lautet. Person A hat darauf keinen Einfluss: der Paar-Zwang
/// von `checkESt` gilt je Person.
#[tokio::test]
async fn die_partner_kirchensteuer_steht_erst_nach_bestaetigter_konfession_des_ehegatten() {
    const KONF_PARTNER: &str = "kist_konfession_partner";
    const ARBEITGEBER_PARTNER: &str = "kirchensteuer_arbeitgeber_partner";
    let zusammen = || ja("veranlagung", &json!("zusammen"));
    for (name, paare, erwartet) in [
        ("Schweigen", vec![zusammen()], false),
        ("A evangelisch, B offen", vec![zusammen(), ja(KONF, &json!("evangelisch"))], false),
        ("B keine", vec![zusammen(), ja(KONF_PARTNER, &json!("keine"))], false),
        ("B vorlaeufig evangelisch", vec![zusammen(), vorschlag(KONF_PARTNER, &json!("evangelisch"))], false),
        ("B evangelisch", vec![zusammen(), ja(KONF_PARTNER, &json!("evangelisch"))], true),
        ("A keine, B katholisch", vec![zusammen(), ja(KONF, &json!("keine")), ja(KONF_PARTNER, &json!("roemisch-katholisch"))], true),
    ] {
        let liste = fragen_nach(&paare).await;
        let ids = ids(&liste);
        assert_eq!(ids.contains(&ARBEITGEBER_PARTNER), erwartet, "{name}: {}", kopf(&ids));
        if name == "B vorlaeufig evangelisch" || name == "Schweigen" {
            assert!(ids.contains(&KONF_PARTNER), "{name}: die Konfession des Ehegatten bleibt eine Frage");
        }
    }
}

/// AK5: Eine vorlaeufige Konfession (Vorjahr, KI) belegt nichts: die Kirchensteuer des Arbeitgebers bleibt aus der Liste, die
/// Konfession selbst bleibt eine Frage. Keine Sackgasse: nach der Bestaetigung steht die Arbeitgeber-Frage in der Liste.
#[tokio::test]
async fn eine_vorlaeufige_konfession_belegt_nichts_und_laesst_keine_sackgasse() {
    let liste = fragen_nach(&[ja("veranlagung", &json!("einzel")), vorschlag(KONF, &json!("evangelisch"))]).await;
    let ids = ids(&liste);
    assert!(ids.contains(&KONF), "vorlaeufige Konfession: sie bleibt eine Frage: {}", kopf(&ids));
    assert!(!ids.contains(&ARBEITGEBER), "vorlaeufige Konfession: die Arbeitgeber-Frage steht schon in der Liste");

    let danach = fragen_nach(&[ja("veranlagung", &json!("einzel")), ja(KONF, &json!("evangelisch"))]).await;
    assert!(ids_enthalten(&danach, ARBEITGEBER), "bestaetigte Konfession: die Arbeitgeber-Frage fehlt");
}

fn ids_enthalten(liste: &[Value], feld: &str) -> bool {
    ids(liste).contains(&feld)
}

/// AK1 (Haushaltshilfen): Die Zaehler stehen erst nach "ja" auf `hh_hat_aufwendungen`. Im leeren Fall steht die Ja/Nein-Frage,
/// kein Zaehler; "nein" und ein bloss vorlaeufiges "ja" lassen die Zaehler aus der Liste; ein bestaetigtes "ja" bringt alle drei.
#[tokio::test]
async fn die_zaehler_der_haushaltshilfen_stehen_erst_nach_ja() {
    let leer = fragen_nach(&[]).await;
    let leer_ids = ids(&leer);
    assert!(leer_ids.contains(&HH_JA_NEIN), "Kontrolle: die Ja/Nein-Frage fehlt: {}", kopf(&leer_ids));
    assert_eq!(davon(&leer_ids, &ZAEHLER), Vec::<&str>::new(), "leerer Fall: Zaehler vor der Ja/Nein-Frage");

    let nein = fragen_nach(&[ja(HH_JA_NEIN, &json!(false))]).await;
    assert_eq!(davon(&ids(&nein), &ZAEHLER), Vec::<&str>::new(), "nein: Zaehler in der Liste");

    let vorlaeufig = fragen_nach(&[vorschlag(HH_JA_NEIN, &json!(true))]).await;
    let vorlaeufig_ids = ids(&vorlaeufig);
    assert!(vorlaeufig_ids.contains(&HH_JA_NEIN), "vorlaeufiges ja: die Frage bleibt");
    assert_eq!(davon(&vorlaeufig_ids, &ZAEHLER), Vec::<&str>::new(), "vorlaeufiges ja: Zaehler in der Liste");

    let ja_liste = fragen_nach(&[ja(HH_JA_NEIN, &json!(true))]).await;
    assert_eq!(davon(&ids(&ja_liste), &ZAEHLER), ZAEHLER.to_vec(), "ja: nicht alle drei Zaehler stehen in der Liste");
}

/// AK2 (Haushaltshilfen): Ein gespeicherter Zaehlerwert zaehlt weiter, auch wenn die Ja/Nein-Frage unbeantwortet ist. Der
/// Zaehler steht nicht in der Liste, aber die Betragsfrage der Handwerkerleistung fragt weiter je gespeicherter Instanz.
#[tokio::test]
async fn ein_gespeicherter_zaehler_zaehlt_ohne_ja_weiter() {
    let ohne = fragen_nach(&[]).await;
    let mit = fragen_nach(&[ja("hh_anzahl_handwerker", &json!(2))]).await;
    let mit_ids = ids(&mit);
    assert!(!mit_ids.contains(&"hh_anzahl_handwerker"), "der beantwortete Zaehler steht wieder in der Liste");
    assert!(mit_ids.contains(&HH_JA_NEIN), "die Ja/Nein-Frage bleibt unbeantwortet in der Liste");
    let instanzen = |liste: &[Value]| {
        liste.iter().find(|q| q["feld_id"] == "hh_handwerker_betrag").map(|q| q["instanz_anzahl"].clone())
    };
    assert_eq!(instanzen(&mit), Some(json!(2)), "gespeicherter Zaehler 2: die Betragsfrage fragt nicht zweimal");
    assert_ne!(instanzen(&ohne), instanzen(&mit), "Kontrolle: ohne gespeicherten Zaehler ist die Zahl der Instanzen eine andere");
}

/// AK4 (Haushaltshilfen), AK-Pin: Der Beleg nimmt den Arbeitsweg und `agb_notwendig_angemessen` nicht mit. Beide sind
/// Pflichtfelder des Kegels; fehlten sie in der Liste, bliebe die Steuerzahl bei `input_kegel_nicht_bestaetigt` gesperrt
/// (Bericht Phase 0, Abschnitt 6). Der Mutant "`beleg` auch an `ep_entfernung_km`" muss diesen Test rot faerben.
#[tokio::test]
async fn arbeitsweg_und_agb_bleiben_in_der_liste() {
    for (name, paare) in [
        ("leerer Fall", vec![]),
        ("keine Konfession, kein Aufwand", vec![ja(KONF, &json!("keine")), ja(HH_JA_NEIN, &json!(false))]),
        ("Arbeitstage 0", vec![ja("ep_arbeitstage", &json!(0))]),
    ] {
        let liste = fragen_nach(&paare).await;
        let ids = ids(&liste);
        let erwartet: Vec<&str> = PFLICHT_BLEIBT.iter().copied().filter(|f| !paare.iter().any(|(p, _, _)| p == f)).collect();
        assert_eq!(davon(&ids, &erwartet), erwartet, "{name}: Pflichtfelder fehlen in der Liste: {}", kopf(&ids));
    }
}
