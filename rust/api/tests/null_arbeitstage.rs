//! Fragen-Reduktion Teil 2, Phase 3 (Entscheidungen d-55 und d-56, Julius 2026-10-22): Wer bestaetigt, dass er an 0 Tagen zur Arbeit
//! gefahren ist, beantwortet die Fragen zum Arbeitsweg nicht, die an diesen Tagen haengen. Ohne `PARITY=1`, ohne Python, ohne Netz.
//!
//! **Worum es geht.** Bei 0 Arbeitstagen ist die Entfernungspauschale 0, egal wie weit der Weg ist und ob ein Auto im Spiel war. Die
//! Fragen nach Entfernung, Auto, Ziel, Adresse und Bus und Bahn aendern die Steuer dann nicht (wer nicht fuhr, hat keine Wege zur ersten
//! Taetigkeitsstaette, § 9 Abs. 2 `EStG`), standen aber in der Liste (Entfernung, Auto und Bus und Bahn gehoeren zum Pflicht-Kegel: ohne
//! ihre Antwort kam keine Zahl, `input_kegel_nicht_bestaetigt`).
//!
//! **Was gebaut ist.** (1) Entfernung, Auto und Bus und Bahn leitet der Store aus den bestaetigten 0 Tagen ab (Ableitungsart
//! `neutral_bei_null`, `rust/store/src/ableitung.rs`, `store.rs::rechne_ab`); sie sind dann bestaetigt und stehen nicht in der Liste. Die
//! Sperre `input_kegel_nicht_bestaetigt` bleibt unveraendert. (2) Ziel und Adresse fallen per `feld_bedingung` aus der Liste, sie
//! bekommen KEINEN Wert (ABWEICHUNG vom Wortlaut von d-55: ein abgeleiteter Wert stuende als `E0203003` und `E0203501` in der Erklaerung).
//! (3) Die Unfallkosten fallen ueber die vorhandene `feld_bedingung` an der Entfernung mit weg. `agb_notwendig_angemessen` bleibt. (4)
//! `ep_arbeitstage` ist `eingangsfrage`, sonst stuenden Ziel und Adresse (Vordruck-Reihenfolge nach Kennzahl) davor.
//!
//! **Bekannte Grenze.** Eine EIGENE bestaetigte Antwort des Nutzers auf Bus und Bahn (vor oder nach der 0) bleibt stehen und wird weiter
//! abgezogen: der Rechenkern kennt die Tage dort nicht (`abziehbarer_betrag = oepnv_kosten_jahr`, wenn sie ueber der Pauschale liegen).
//! Die Steuer ist dann dieselbe wie heute (`die_zahl_...`, Teil b). Der Regelfix ist ein eigenes Ticket.
//!
//! **Belegt wird** Frage fuer Frage ueber `POST /event`, `GET /fragen`, `GET /ergebnis`, `GET /stand` und `GET /deklaration`, so wie die
//! Oberflaeche sie sieht. Die Messungen gegen `bb12df08` stehen in den `messung_*` (`--ignored`).
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

/// Die Weg-Felder, die bei bestaetigten 0 Tagen aus der Liste fallen: drei abgeleitet, zwei ausgeblendet, eines folgt der Entfernung.
const WEGFALL: [&str; 6] = ["ep_eigenes_kfz", "ep_entfernung_km", "ep_oepnv_kosten", "ep_ziel_des_weges", "ep_ziel_adresse", "ep_unfallkosten"];
/// Die Kegel-Felder, die der Store bei 0 Tagen ableitet.
const ABGELEITET: [&str; 3] = ["ep_entfernung_km", "ep_eigenes_kfz", "ep_oepnv_kosten"];
const OEPNV: &str = "ep_oepnv_kosten";
/// 40.000 EUR Lohn, einzeln, alle Kreuze "nein", 0 Tage, Bus und Bahn 0: Steuer in Cent, gemessen auf `bb12df08` (Entfernung 30 km mit
/// Auto, 0 km ohne Auto und 30 km ohne Auto liefern dieselbe Zahl).
const STEUER_NULL_TAGE: i64 = 691_900;
/// Dieselbe Basis mit 1.380 EUR Fahrkarten, die der Nutzer selbst eintraegt: Steuer in Cent. Die Differenz zu `STEUER_NULL_TAGE` (6.500
/// Cent) ist Grenzsatz mal Mehrabzug: 1.380 EUR Fahrkarten plus 56 EUR Verpflegung der Basis ergeben 1.436 EUR Werbungskosten, 206 EUR
/// ueber dem Arbeitnehmer-Pauschbetrag (`messung_kurve_fahrkarten`).
const STEUER_NULL_TAGE_FAHRKARTEN: i64 = 685_400;

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

/// Ein Vorschlag eines Vorschlags-Schreibers (`llm:chat`, `import:beleg`): vorlaeufig, ehrliche Herkunft, ohne `signal_2`.
fn vorschlag_von(schreiber: &str, herkunft: &str, feld_id: &str, wert: &Value) -> Value {
    json!({
        "feld_id": feld_id,
        "herkunft": {"haftung": "nutzer", "herkunft": herkunft, "pruef_tiefe": "ungeprueft"},
        "schreiber": schreiber,
        "signal": {"signal_1": null, "signal_2": null},
        "wert": wert,
        "zustand": "vorlaeufig",
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

type Paar<'a> = (&'a str, Value, bool);

/// Der Pflicht-Kegel von `an_gesamt` ohne den Arbeitsweg, samt der Angaben, die den Guard zufriedenstellen, mit neutralen Werten
/// (`ergebnis_scheibe_naht::kegel_an_voll` ohne `ep_*`): 40.000 EUR Lohn, einzeln.
fn basis() -> Vec<Paar<'static>> {
    let ja = |f: &'static str, w: Value| (f, w, true);
    vec![
        ja("bruttoarbeitslohn", json!(4_000_000)),
        ja("veranlagung", json!("einzel")),
        ja("vor_an_anteil_rv", json!(0)),
        ja("vor_ag_anteil_rv", json!(0)),
        ja("vor_rv_ausserhalb_lstb", json!(0)),
        ja("versicherungsart", json!("gesetzlich_an")),
        ja("basis_kv", json!(0)),
        ja("basis_pv", json!(0)),
        ja("vorsorge_arbeitslosenversicherung", json!(0)),
        ja("vorsorge_erwerbsunfaehigkeit", json!(0)),
        ja("vorsorge_unfall_haftpflicht", json!(0)),
        ja("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        ja("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        ja("mit_anspruch_auf_zuschuss", json!(false)),
        ja("dhf_unterkunftskosten_monat", json!(0)),
        ja("dhf_im_inland", json!(false)),
        ja("dhf_beruflich_veranlasst", json!(false)),
        ja("dhf_eigener_hausstand", json!(false)),
        ja("dhf_finanzielle_beteiligung", json!(false)),
        ja("tage_24h", json!(1)),
        ja("tage_an_abreise", json!(1)),
        ja("tage_ueber_8h_eintaegig", json!(1)),
        ja("kein_gewinn", json!(true)),
        ja("kein_kap", json!(true)),
        ja("kein_vuv", json!(true)),
        ja("kein_sonstige", json!(true)),
        ja("verlustvortrag_bestand", json!(0)),
        ja("fam_anzahl_kinder", json!(0)),
        ja("dhf_monate", json!(0)),
        ja("vpf_monate_am_ort", json!(2)),
        ja("vpf_keine_mahlzeitengestellung", json!(true)),
    ]
}

fn plus<'a>(mut paare: Vec<Paar<'a>>, mehr: &[(&'a str, Value)]) -> Vec<Paar<'a>> {
    paare.extend(mehr.iter().map(|(f, w)| (*f, w.clone(), true)));
    paare
}

/// Legt einen Fall der Scheibe `scheibe` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(scheibe: &str, paare: &[Paar<'_>]) -> Dienst {
    let d = dienst();
    fall_anlegen(&d, "nz", scheibe, 2025).await;
    for (feld, wert, bestaetigt) in paare {
        schreibe(&d, "nz", feld, wert, *bestaetigt).await;
    }
    d
}

async fn fall_anlegen(d: &Dienst, id: &str, scheibe: &str, vz: i64) {
    let kopf = json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": vz});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
}

async fn schreibe(d: &Dienst, id: &str, feld: &str, wert: &Value, bestaetigt: bool) {
    let (status, antwort) = sende(d, "POST", &format!("/fall/{id}/event"), Some(&ereignis(feld, wert, bestaetigt))).await;
    assert_eq!(status, 201, "POST /event {feld}: {antwort}");
}

/// Die Korrektur wie in der Oberflaeche: ein neues Ereignis mit `ersetzt` auf das aktive Ereignis des Feldes (auch auf einen
/// Vorschlag: so bestaetigt die Oberflaeche ihn).
async fn korrigiere(d: &Dienst, id: &str, feld: &str, wert: &Value) {
    let (_, stand) = sende(d, "GET", &format!("/fall/{id}/stand"), None).await;
    let alt = stand["felder"][feld]["event_id"].clone();
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie", "signal": {"signal_1": alt, "signal_2": "ok"}, "ersetzt": alt,
    });
    let (status, antwort) = sende(d, "POST", &format!("/fall/{id}/event"), Some(&rumpf)).await;
    assert!(status == 200 || status == 201, "Korrektur {feld}: {status} {antwort}");
}

async fn fragen_ids(d: &Dienst) -> Vec<String> {
    let (status, fragen) = sende(d, "GET", "/fall/nz/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"].as_array().unwrap().iter().map(|q| q["feld_id"].as_str().unwrap().to_owned()).collect()
}

async fn ergebnis(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/nz/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// `(wert, zustand)` eines Feldes im Stand, oder `None`, wenn der Stand das Feld nicht kennt.
async fn im_stand(d: &Dienst, feld: &str) -> Option<(Value, String)> {
    let (status, stand) = sende(d, "GET", "/fall/nz/stand", None).await;
    assert_eq!(status, 200, "GET /stand: {stand}");
    let f = stand["felder"].get(feld)?;
    Some((f["wert"].clone(), f["zustand"].as_str().unwrap_or("").to_owned()))
}

fn steht(ids: &[String], feld: &str) -> bool {
    ids.iter().any(|i| i == feld)
}

/// Welche der `felder` stehen in der Liste?
fn davon<'a>(ids: &[String], felder: &[&'a str]) -> Vec<&'a str> {
    felder.iter().copied().filter(|f| steht(ids, f)).collect()
}

fn kegel_offen(e: &Value) -> Vec<String> {
    e["offen"].as_array().map(|o| o.iter().map(|x| x.as_str().unwrap().to_owned()).collect()).unwrap_or_default()
}

const SCHEIBEN: [&str; 2] = ["an_gesamt", "gesamt"];

/// AK1: Bei bestaetigten 0 Tagen fehlen genau sechs Fragen in der Liste: Auto, Entfernung, Bus und Bahn, Ziel, Adresse und
/// Unfallkosten. In `gesamt` bleibt die AgB-Frage. KONTROLLE: Mit 220 Tagen stehen alle sechs, und die Listen unterscheiden sich um
/// genau diese sechs Fragen.
#[tokio::test]
async fn bei_bestaetigten_null_tagen_fallen_genau_sechs_fragen_weg() {
    for scheibe in SCHEIBEN {
        let null = fragen_ids(&fall(scheibe, &[("ep_arbeitstage", json!(0), true)]).await).await;
        let viele = fragen_ids(&fall(scheibe, &[("ep_arbeitstage", json!(220), true)]).await).await;
        assert_eq!(davon(&viele, &WEGFALL), WEGFALL.to_vec(), "{scheibe}: Kontrolle, bei 220 Tagen stehen alle sechs: {viele:?}");
        assert_eq!(davon(&null, &WEGFALL), Vec::<&str>::new(), "{scheibe}: bei 0 Tagen stehen Weg-Fragen in der Liste");
        let weg: Vec<&String> = viele.iter().filter(|i| !null.contains(i)).collect();
        assert_eq!(weg.len(), WEGFALL.len(), "{scheibe}: die Listen unterscheiden sich um mehr als die sechs Fragen: {weg:?}");
        assert_eq!(viele.len() - null.len(), 6, "{scheibe}: Fragenzahl 220 Tage gegen 0 Tage");
        assert!(null.iter().all(|i| viele.contains(i)), "{scheibe}: bei 0 Tagen kommt eine Frage dazu");
    }
    let gesamt = fragen_ids(&fall("gesamt", &[("ep_arbeitstage", json!(0), true)]).await).await;
    assert!(steht(&gesamt, "agb_notwendig_angemessen"), "die AgB-Frage bleibt (d-55)");
}

/// AK1 (Reihenfolge): Die Frage nach den Tagen steht vor jeder anderen Weg-Frage (`eingangsfrage`). Sonst stuenden Ziel und Adresse
/// nach der Reihenfolge des Vordrucks (E0203003, E0203501) davor, und der Nutzer beantwortete sie, bevor die 0 sie streicht.
#[tokio::test]
async fn die_frage_nach_den_tagen_kommt_vor_den_weg_fragen() {
    for scheibe in SCHEIBEN {
        let ids = fragen_ids(&fall(scheibe, &[]).await).await;
        let pos = |f: &str| ids.iter().position(|i| i == f).unwrap_or_else(|| panic!("{scheibe}: {f} fehlt im leeren Fall"));
        let tage = pos("ep_arbeitstage");
        for f in WEGFALL {
            assert!(tage < pos(f), "{scheibe}: ep_arbeitstage (Stelle {tage}) steht nach {f} (Stelle {})", pos(f));
        }
    }
}

/// AK2: Ohne bestaetigte 0 leitet der Store nichts ab: bei offenen, vorlaeufigen und bestaetigten Tagen ueber 0 stehen Entfernung und
/// Auto in der Liste, und der Stand kennt sie nicht (oder nur als Vorschlag). Der Mutant "ohne `== 0`" und der Mutant "ohne
/// Bestaetigung" muessen diesen Test rot faerben.
#[tokio::test]
async fn ohne_bestaetigte_null_wird_nichts_abgeleitet() {
    for (name, paare) in [
        ("leerer Fall", vec![]),
        ("0 Tage nur als Vorschlag", vec![("ep_arbeitstage", json!(0), false)]),
        ("1 Tag", vec![("ep_arbeitstage", json!(1), true)]),
        ("220 Tage", vec![("ep_arbeitstage", json!(220), true)]),
    ] {
        let d = fall("an_gesamt", &paare).await;
        let ids = fragen_ids(&d).await;
        assert_eq!(davon(&ids, &WEGFALL), WEGFALL.to_vec(), "{name}: die Weg-Fragen fehlen in der Liste");
        for f in ABGELEITET {
            let stand = im_stand(&d, f).await;
            assert!(stand.is_none(), "{name}: {f} steht im Stand: {stand:?}");
        }
    }
}

/// AK3 und AK-Euro, zweigeteilt (main, 2026-10-22). (a) Bei bestaetigten 0 Tagen ohne eingetragene Fahrkartenkosten kommt die Zahl, ohne
/// dass Entfernung, Auto und Bus und Bahn beantwortet sind, und sie ist auf den Cent dieselbe wie mit eigenen Antworten (30 km mit Auto,
/// 0 km ohne Auto, 30 km ohne Auto; Bus und Bahn 0) und dieselbe wie auf `bb12df08`. (b) Mit Fahrkartenkosten, die der Nutzer selbst
/// eintraegt, bleibt der Wert stehen (vor der 0 wie danach) und die Zahl bleibt wie heute: die BEKANNTE GRENZE des Rechenkerns, der
/// die Tage dort nicht kennt. Die Sperre `input_kegel_nicht_bestaetigt` bleibt: 220 Tage ohne die drei Antworten nennen alle drei.
#[tokio::test]
async fn die_zahl_kommt_bei_null_tagen_ohne_die_weg_fragen_und_ist_dieselbe_wie_mit_eigenen_antworten() {
    // (a) Nichts eingetragen.
    let abgeleitet = ergebnis(&fall("an_gesamt", &plus(basis(), &[("ep_arbeitstage", json!(0))])).await).await;
    assert_eq!(abgeleitet["grund"], "bestaetigt", "{abgeleitet}");
    assert_eq!(abgeleitet["zahl_cent"], json!(STEUER_NULL_TAGE), "Steuer mit abgeleiteten Weg-Feldern: {abgeleitet}");
    for (name, km, kfz) in [("30 km, Auto", 30, true), ("0 km, kein Auto", 0, false), ("30 km, kein Auto", 30, false)] {
        let eigen = plus(basis(), &[("ep_arbeitstage", json!(0)), (OEPNV, json!(0)), ("ep_entfernung_km", json!(km)), ("ep_eigenes_kfz", json!(kfz))]);
        let e = ergebnis(&fall("an_gesamt", &eigen).await).await;
        assert_eq!(e["zahl_cent"], abgeleitet["zahl_cent"], "{name}: eigene Antworten bei 0 Tagen aendern die Steuer: {e}");
    }
    // (b) Fahrkarten, die der Nutzer selbst eintraegt: davor, danach und als einzige Weg-Antwort. Immer dieselbe Zahl wie heute.
    let explizit = ergebnis(&fall("an_gesamt", &plus(basis(), &[("ep_arbeitstage", json!(0)), (OEPNV, json!(138_000)), ("ep_entfernung_km", json!(0)), ("ep_eigenes_kfz", json!(false))])).await).await;
    assert_eq!(explizit["zahl_cent"], json!(STEUER_NULL_TAGE_FAHRKARTEN), "Kontrolle, die Zahl mit Fahrkarten ist eine andere: {explizit}");
    for (name, paare) in [
        ("Fahrkarten vor der 0", plus(basis(), &[(OEPNV, json!(138_000)), ("ep_arbeitstage", json!(0))])),
        ("Fahrkarten nach der 0", plus(basis(), &[("ep_arbeitstage", json!(0)), (OEPNV, json!(138_000))])),
    ] {
        let e = ergebnis(&fall("an_gesamt", &paare).await).await;
        assert_eq!((e["grund"].clone(), e["zahl_cent"].clone()), (json!("bestaetigt"), json!(STEUER_NULL_TAGE_FAHRKARTEN)), "{name}: {e}");
    }
    // Die Sperre bleibt: bei 220 Tagen leitet der Store nichts ab, und die drei Antworten fehlen.
    let viele = ergebnis(&fall("an_gesamt", &plus(basis(), &[("ep_arbeitstage", json!(220))])).await).await;
    assert_eq!(viele["grund"], "input_kegel_nicht_bestaetigt", "{viele}");
    assert_eq!(kegel_offen(&viele), vec!["ep_eigenes_kfz".to_owned(), "ep_entfernung_km".to_owned(), OEPNV.to_owned()], "{viele}");
    assert!(viele["zahl_cent"].is_null(), "{viele}");
}

/// AK4: Der Neutralwert ist abgeleitet und bestaetigt; eine bestaetigte Antwort des Nutzers davor bleibt stehen, eine danach ersetzt ihn
/// ohne Kennung (schmale Ausnahme in Auflage B, nur fuer Felder mit der Ableitungsart `neutral_bei_null`). Der Mutant "Ausnahme
/// entfernt" muss diesen Test rot faerben (422 "fail-closed (B)").
#[tokio::test]
async fn eigene_antworten_gehen_vor_der_ableitung_und_ersetzen_sie_ohne_kennung() {
    // Abgeleitet: Wert und Zustand.
    let d = fall("an_gesamt", &[("ep_arbeitstage", json!(0), true)]).await;
    assert_eq!(im_stand(&d, "ep_entfernung_km").await, Some((json!(0), "bestaetigt".to_owned())));
    assert_eq!(im_stand(&d, "ep_eigenes_kfz").await, Some((json!(false), "bestaetigt".to_owned())));
    assert_eq!(im_stand(&d, OEPNV).await, Some((json!(0), "bestaetigt".to_owned())));
    // Danach die eigene Antwort ohne `ersetzt`: 201, sie gilt.
    schreibe(&d, "nz", "ep_entfernung_km", &json!(30), true).await;
    schreibe(&d, "nz", "ep_eigenes_kfz", &json!(true), true).await;
    schreibe(&d, "nz", OEPNV, &json!(138_000), true).await;
    assert_eq!(im_stand(&d, "ep_entfernung_km").await, Some((json!(30), "bestaetigt".to_owned())));
    assert_eq!(im_stand(&d, "ep_eigenes_kfz").await, Some((json!(true), "bestaetigt".to_owned())));
    assert_eq!(im_stand(&d, OEPNV).await, Some((json!(138_000), "bestaetigt".to_owned())));
    // Davor die eigene Antwort: die Ableitung laesst sie stehen.
    let d = fall("an_gesamt", &[("ep_entfernung_km", json!(25), true), ("ep_eigenes_kfz", json!(true), true), (OEPNV, json!(138_000), true), ("ep_arbeitstage", json!(0), true)]).await;
    assert_eq!(im_stand(&d, "ep_entfernung_km").await, Some((json!(25), "bestaetigt".to_owned())));
    assert_eq!(im_stand(&d, "ep_eigenes_kfz").await, Some((json!(true), "bestaetigt".to_owned())));
    assert_eq!(im_stand(&d, OEPNV).await, Some((json!(138_000), "bestaetigt".to_owned())));
    // KONTROLLE: Die Ausnahme gilt nicht fuer die Antwort eines Nutzers auf die Antwort eines Nutzers (Auflage B bleibt).
    let (status, antwort) = sende(&d, "POST", "/fall/nz/event", Some(&ereignis("ep_entfernung_km", &json!(26), true))).await;
    assert_eq!(status, 422, "{antwort}");
    assert!(antwort["fehler"].as_str().unwrap().contains("fail-closed (B)"), "{antwort}");
}

/// AK5 (Korrektur ueber HTTP): 0 -> 220 -> 0. Mit 220 Tagen kommen die sechs Fragen zurueck, die drei Neutralwerte sind zurueckgezogen
/// (nur noch ein Vorschlag mit dem Neutralwert), die Zahl ist gesperrt, und KEINE Route gibt 500 (`Null` auf der Entfernung brach
/// `GET /stand` und `GET /fragen`). Mit 0 Tagen sind sie wieder weg. Die eigene Antwort ueberlebt beide Wege. Der Mutant "Rueckzug
/// entfernt" (die Rechnung bleibt nach der Korrektur stehen) und der Mutant "Rueckzug als `Null`" muessen diesen Test rot faerben.
#[tokio::test]
async fn die_korrektur_von_null_auf_mehr_und_zurueck_oeffnet_und_schliesst_die_fragen() {
    let d = fall("an_gesamt", &plus(basis(), &[("ep_arbeitstage", json!(0))])).await;
    assert_eq!(ergebnis(&d).await["zahl_cent"], json!(STEUER_NULL_TAGE));

    korrigiere(&d, "nz", "ep_arbeitstage", &json!(220)).await;
    for pfad in ["stand", "fragen", "ergebnis", "deklaration", "preflight", "graph"] {
        let (status, antwort) = sende(&d, "GET", &format!("/fall/nz/{pfad}"), None).await;
        assert_eq!(status, 200, "GET /{pfad} nach 0 -> 220: {antwort}");
    }
    let ids = fragen_ids(&d).await;
    assert_eq!(davon(&ids, &WEGFALL), WEGFALL.to_vec(), "nach 220 Tagen fehlen Weg-Fragen: {ids:?}");
    for (f, neutral) in [("ep_entfernung_km", json!(0)), ("ep_eigenes_kfz", json!(false)), (OEPNV, json!(0))] {
        assert_eq!(im_stand(&d, f).await, Some((neutral, "vorlaeufig".to_owned())), "{f} nach 220 Tagen");
    }
    let e = ergebnis(&d).await;
    assert_eq!(e["grund"], "input_kegel_nicht_bestaetigt", "{e}");
    assert!(e["zahl_cent"].is_null(), "die Zahl steht trotz offener Weg-Fragen: {e}");
    assert_eq!(kegel_offen(&e), vec!["ep_eigenes_kfz".to_owned(), "ep_entfernung_km".to_owned(), OEPNV.to_owned()], "{e}");

    korrigiere(&d, "nz", "ep_arbeitstage", &json!(0)).await;
    assert_eq!(davon(&fragen_ids(&d).await, &WEGFALL), Vec::<&str>::new());
    assert_eq!(ergebnis(&d).await["zahl_cent"], json!(STEUER_NULL_TAGE), "die Steuer nach 0 -> 220 -> 0");

    // Die eigenen Antworten ueberleben: 30 km und Fahrkarten gegeben, dann 0 -> 220 -> 0.
    let d = fall("an_gesamt", &[("ep_arbeitstage", json!(0), true), ("ep_entfernung_km", json!(30), true), (OEPNV, json!(138_000), true)]).await;
    korrigiere(&d, "nz", "ep_arbeitstage", &json!(220)).await;
    korrigiere(&d, "nz", "ep_arbeitstage", &json!(0)).await;
    assert_eq!(im_stand(&d, "ep_entfernung_km").await, Some((json!(30), "bestaetigt".to_owned())));
    assert_eq!(im_stand(&d, OEPNV).await, Some((json!(138_000), "bestaetigt".to_owned())));
}

/// AK6 (Vorjahr ueber HTTP): `POST /vorjahr` legt Entfernung, Auto, Bus und Bahn, Ziel und Adresse des Vorjahres (und die Tage) als
/// Vorschlag ab. (a) Erst Vorjahr, dann die bestaetigte 0 (mit `ersetzt` auf den Vorschlag der Tage): Entfernung, Auto und Bus und Bahn
/// sind durch den Neutralwert ersetzt (auch der Vorschlag von 1.380 EUR Fahrkarten), die Zahl kommt ohne weitere Antwort. (b) Erst
/// die bestaetigte 0, dann Vorjahr: die Uebernahme laesst die abgeleiteten Felder stehen (kein 422, kein 500). (c) KONTROLLE: Mit
/// bestaetigten 220 Tagen bleiben die Vorschlaege Vorschlaege.
#[tokio::test]
async fn der_vorjahr_vorschlag_wird_von_der_bestaetigten_null_ersetzt() {
    let d = dienst();
    fall_anlegen(&d, "vq", "gesamt", 2024).await;
    for (f, w) in [
        ("ep_arbeitstage", json!(220)),
        ("ep_entfernung_km", json!(30)),
        ("ep_eigenes_kfz", json!(true)),
        (OEPNV, json!(138_000)),
        ("ep_ziel_des_weges", json!("1")),
        ("ep_ziel_adresse", json!("80331 München, Marienplatz 1")),
    ] {
        schreibe(&d, "vq", f, &w, true).await;
    }
    let vorjahr = json!({"vorjahr_fall_id": "vq"});
    let vorschlaege = [("ep_entfernung_km", json!(30)), ("ep_eigenes_kfz", json!(true)), (OEPNV, json!(138_000))];
    let neutral = [("ep_entfernung_km", json!(0)), ("ep_eigenes_kfz", json!(false)), (OEPNV, json!(0))];

    // (a) Erst Vorjahr, dann 0.
    fall_anlegen(&d, "nz", "an_gesamt", 2025).await;
    let (status, antwort) = sende(&d, "POST", "/fall/nz/vorjahr", Some(&vorjahr)).await;
    assert_eq!(status, 200, "POST /vorjahr: {antwort}");
    for (f, wert) in &vorschlaege {
        assert_eq!(im_stand(&d, f).await, Some((wert.clone(), "vorlaeufig".to_owned())), "Kontrolle, {f} kommt als Vorschlag: {antwort}");
    }
    korrigiere(&d, "nz", "ep_arbeitstage", &json!(0)).await;
    for (f, wert) in &neutral {
        assert_eq!(im_stand(&d, f).await, Some((wert.clone(), "bestaetigt".to_owned())), "{f}: der Neutralwert ersetzt den Vorschlag");
    }
    assert_eq!(davon(&fragen_ids(&d).await, &WEGFALL), Vec::<&str>::new());
    for (f, w, _) in basis() {
        schreibe(&d, "nz", f, &w, true).await;
    }
    let e = ergebnis(&d).await;
    assert_eq!((e["grund"].clone(), e["zahl_cent"].clone()), (json!("bestaetigt"), json!(STEUER_NULL_TAGE)), "{e}");

    // (b) Erst 0, dann Vorjahr.
    fall_anlegen(&d, "nz2", "an_gesamt", 2025).await;
    schreibe(&d, "nz2", "ep_arbeitstage", &json!(0), true).await;
    let (status, antwort) = sende(&d, "POST", "/fall/nz2/vorjahr", Some(&vorjahr)).await;
    assert_eq!(status, 200, "Vorjahr nach der 0: {antwort}");
    let (_, stand) = sende(&d, "GET", "/fall/nz2/stand", None).await;
    for (f, wert) in &neutral {
        assert_eq!((stand["felder"][f]["wert"].clone(), stand["felder"][f]["zustand"].clone()), (wert.clone(), json!("bestaetigt")), "{f}: {stand}");
    }

    // (c) KONTROLLE: bestaetigte 220 Tage leiten nichts ab.
    fall_anlegen(&d, "nz3", "an_gesamt", 2025).await;
    let (status, _) = sende(&d, "POST", "/fall/nz3/vorjahr", Some(&vorjahr)).await;
    assert_eq!(status, 200);
    korrigiere(&d, "nz3", "ep_arbeitstage", &json!(220)).await;
    let (_, stand) = sende(&d, "GET", "/fall/nz3/stand", None).await;
    for (f, wert) in &vorschlaege {
        assert_eq!((stand["felder"][f]["wert"].clone(), stand["felder"][f]["zustand"].clone()), (wert.clone(), json!("vorlaeufig")), "{f}: {stand}");
    }
}

/// AK7: Bei bestaetigten 0 Tagen steht kein `E0203003` und kein `E0203501` in der Erklaerung (Ziel und Adresse fallen aus der Liste,
/// sie bekommen keinen Wert). KONTROLLE: Wer bei 220 Tagen Ziel und Adresse beantwortet, hat beide Kennzahlen in der Erklaerung. Der
/// Mutant "`feld_bedingung` an Ziel und Adresse entfernt" faerbt `bei_bestaetigten_null_tagen_fallen_genau_sechs_fragen_weg` rot, der
/// Mutant "Ziel und Adresse abgeleitet" diesen Test.
#[tokio::test]
async fn bei_null_tagen_steht_kein_ziel_und_keine_adresse_in_der_erklaerung() {
    let ziel = [("ep_ziel_des_weges", json!("1")), ("ep_ziel_adresse", json!("80331 München, Marienplatz 1"))];
    let kz = |d: &Value| -> Vec<String> {
        d["deklaration"].as_object().map(|o| o.keys().filter(|k| k.starts_with("E0203")).cloned().collect()).unwrap_or_default()
    };
    let null = fall("an_gesamt", &plus(basis(), &[("ep_arbeitstage", json!(0))])).await;
    let (status, dekl) = sende(&null, "GET", "/fall/nz/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    assert_eq!(kz(&dekl), vec!["E0203611".to_owned()], "bei 0 Tagen steht nur die abgeleitete Null bei Bus und Bahn: {dekl}");
    let viele = fall("an_gesamt", &plus(plus(basis(), &[("ep_arbeitstage", json!(220)), (OEPNV, json!(0)), ("ep_entfernung_km", json!(30)), ("ep_eigenes_kfz", json!(true))]), &ziel)).await;
    let (status, dekl) = sende(&viele, "GET", "/fall/nz/deklaration", None).await;
    assert_eq!(status, 200, "{dekl}");
    let kennzahlen = kz(&dekl);
    assert!(kennzahlen.contains(&"E0203003".to_owned()) && kennzahlen.contains(&"E0203501".to_owned()), "Kontrolle, bei 220 Tagen stehen Ziel und Adresse: {dekl}");
}

/// Zu den abgeleiteten Feldern (Entfernung, Auto, Bus und Bahn): ein Vorschlag der KI und ein Beleg gehen ueber `POST /event` weiter
/// durch Auflage B und K1. Sie geben 4xx (hier 422), nie 5xx, und lassen den Neutralwert stehen.
#[tokio::test]
async fn vorschlaege_auf_abgeleitete_felder_geben_eine_abweisung_und_nie_einen_serverfehler() {
    let d = fall("an_gesamt", &[("ep_arbeitstage", json!(0), true)]).await;
    for (schreiber, herkunft) in [("llm:chat", "llm_vorschlag"), ("import:beleg", "beleg_import")] {
        for (feld, wert) in [("ep_entfernung_km", json!(25)), ("ep_eigenes_kfz", json!(true)), (OEPNV, json!(138_000))] {
            let (status, antwort) = sende(&d, "POST", "/fall/nz/event", Some(&vorschlag_von(schreiber, herkunft, feld, &wert))).await;
            println!("MESSUNG ii {schreiber} auf {feld}: {status} {antwort}");
            assert!((400..500).contains(&status), "{schreiber} auf {feld}: Status {status}: {antwort}");
            assert_eq!(im_stand(&d, feld).await.map(|s| s.1), Some("bestaetigt".to_owned()), "{schreiber} auf {feld}: der Neutralwert gilt nicht mehr");
        }
    }
}

/// MESSUNG: Fragenzahl je Scheibe im leeren Fall, mit bestaetigten 0 und mit bestaetigten 220 Tagen, und die Stelle von
/// `ep_arbeitstage`. Laeuft auf dem Bau und auf `bb12df08`. Kein Beleg.
#[tokio::test]
#[ignore = "Messung fuer den Bericht, kein Beleg"]
async fn messung_fragenzahl() {
    for scheibe in ["an_gesamt", "gesamt", "ep", "n_vor_gwg"] {
        for (name, paare) in [
            ("leer", vec![]),
            ("0 Tage", vec![("ep_arbeitstage", json!(0), true)]),
            ("220 Tage", vec![("ep_arbeitstage", json!(220), true)]),
        ] {
            let ids = fragen_ids(&fall(scheibe, &paare).await).await;
            let stelle = ids.iter().position(|i| i == "ep_arbeitstage");
            println!("MESSUNG fragenzahl [{scheibe}] [{name}]: {} Fragen, ep_arbeitstage an Stelle {stelle:?}", ids.len());
        }
    }
}

/// MESSUNG (Nachrechnen der Fahrkarten-Differenz, main 2026-10-22): bei 0 Tagen, eigenen Antworten (0 km, kein Auto) und 0 oder
/// 1.380 EUR Fahrkarten. Das ganze `GET /ergebnis` und die Zeilen der Erklaerung, je Fall. Kein Beleg.
#[tokio::test]
#[ignore = "Messung fuer den Bericht, kein Beleg"]
async fn messung_zerlegung_fahrkarten() {
    for (name, oepnv, extra) in [
        ("Fahrkarten 0", 0, vec![]),
        ("Fahrkarten 1.380 EUR", 138_000, vec![]),
        ("Fahrkarten 1.380 EUR, Konfession keine", 138_000, vec![("kist_konfession", json!("keine"))]),
    ] {
        let paare = plus(plus(basis(), &extra), &[("ep_arbeitstage", json!(0)), (OEPNV, json!(oepnv)), ("ep_entfernung_km", json!(0)), ("ep_eigenes_kfz", json!(false))]);
        let d = fall("an_gesamt", &paare).await;
        let (_, e) = sende(&d, "GET", "/fall/nz/ergebnis", None).await;
        let ohne_trace: serde_json::Map<String, Value> = e.as_object().unwrap().iter().filter(|(k, _)| *k != "trace").map(|(k, v)| (k.clone(), v.clone())).collect();
        println!("MESSUNG zerlegung [{name}] ergebnis ohne trace: {}", Value::Object(ohne_trace));
        let spur = e["trace"].as_object().unwrap();
        println!("MESSUNG zerlegung [{name}] trace-Schluessel: {:?}", spur.keys().collect::<Vec<_>>());
        for (k, v) in spur {
            if k != "regeln" && k != "basis_snapshot" {
                println!("MESSUNG zerlegung [{name}] trace.{k}: {v}");
            }
        }
        let (_, dekl) = sende(&d, "GET", "/fall/nz/deklaration", None).await;
        println!("MESSUNG zerlegung [{name}] erklaerung: {}", dekl["deklaration"]);
    }
}

/// MESSUNG: die Steuer bei 0 Tagen in Abhaengigkeit von den Fahrkarten (Cent), mit und ohne die drei Verpflegungs-Tage der Basis. Wo
/// knickt die Kurve? Kein Beleg.
#[tokio::test]
#[ignore = "Messung fuer den Bericht, kein Beleg"]
async fn messung_kurve_fahrkarten() {
    for (name, ohne_vpf) in [("Basis mit Verpflegungstagen", false), ("ohne Verpflegungstage", true)] {
        let mut zeile = Vec::new();
        for oepnv in [0, 5_000, 20_000, 50_000, 100_000, 123_000, 130_000, 138_000, 150_000, 200_000, 400_000] {
            let mut paare = plus(basis(), &[("ep_arbeitstage", json!(0)), (OEPNV, json!(oepnv)), ("ep_entfernung_km", json!(0)), ("ep_eigenes_kfz", json!(false))]);
            if ohne_vpf {
                for p in &mut paare {
                    if ["tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig"].contains(&p.0) {
                        p.1 = json!(0);
                    }
                }
            }
            let d = fall("an_gesamt", &paare).await;
            let e = ergebnis(&d).await;
            zeile.push(format!("{oepnv}->{}", e["zahl_cent"]));
        }
        println!("MESSUNG kurve [{name}]: {}", zeile.join(" "));
    }
}

/// MESSUNG: alle Routen nach der Korrektur 0 -> 220 (Rueckzug der Neutralwerte). Kein Beleg.
#[tokio::test]
#[ignore = "Messung"]
async fn messung_nach_rueckzug() {
    let d = fall("an_gesamt", &plus(basis(), &[("ep_arbeitstage", json!(0)), (OEPNV, json!(0))])).await;
    korrigiere(&d, "nz", "ep_arbeitstage", &json!(220)).await;
    for pfad in ["stand", "ergebnis", "fragen", "deklaration", "preflight", "graph"] {
        let (s, a) = sende(&d, "GET", &format!("/fall/nz/{pfad}"), None).await;
        let kurz: String = a.to_string().chars().take(160).collect();
        println!("MESSUNG rueckzug {pfad}: {s} {kurz}");
    }
}

/// MESSUNG (Zusatzmessung (i) von main): bei 0 Tagen gespeicherte Unfallkosten, Ziel und Adresse. Euro je Szene, einmal mit eigenen
/// Antworten auf Entfernung und Auto (laeuft auch auf `bb12df08`), einmal mit den abgeleiteten (nur auf dem Bau). Kein Beleg.
#[tokio::test]
#[ignore = "Messung fuer den Bericht, kein Beleg"]
async fn messung_euro_unfallkosten_ziel_adresse() {
    let unfall = ("ep_unfallkosten", json!(150_000));
    let ziel = [("ep_ziel_des_weges", json!("1")), ("ep_ziel_adresse", json!("80331 München, Marienplatz 1"))];
    let szenen: Vec<(&str, Vec<(&str, Value)>)> = vec![
        ("ohne Unfall, ohne Ziel", vec![]),
        ("Unfall 1.500 EUR", vec![unfall.clone()]),
        ("Unfall 1.500 EUR, Ziel und Adresse", vec![unfall.clone(), ziel[0].clone(), ziel[1].clone()]),
        ("Ziel und Adresse", vec![ziel[0].clone(), ziel[1].clone()]),
    ];
    for (name, gespeichert) in &szenen {
        for (variante, weg) in [
            ("eigen 30 km Auto", Some((30, true))),
            ("eigen 0 km kein Auto", Some((0, false))),
            ("abgeleitet", None),
        ] {
            for reihenfolge in ["gespeichert vor der 0", "gespeichert nach der 0"] {
                let tage = plus(basis(), &[("ep_arbeitstage", json!(0))]);
                let mut paare = if reihenfolge.ends_with("vor der 0") {
                    let mut p = plus(basis(), gespeichert);
                    p.push(("ep_arbeitstage", json!(0), true));
                    p
                } else {
                    plus(tage, gespeichert)
                };
                if let Some((km, kfz)) = weg {
                    paare = plus(paare, &[(OEPNV, json!(0)), ("ep_entfernung_km", json!(km)), ("ep_eigenes_kfz", json!(kfz))]);
                }
                let d = fall("an_gesamt", &paare).await;
                let e = ergebnis(&d).await;
                let (status, dekl) = sende(&d, "GET", "/fall/nz/deklaration", None).await;
                let kz: Vec<String> = dekl["deklaration"]
                    .as_object()
                    .map(|o| o.iter().filter(|(k, _)| k.starts_with("E0203") || k.starts_with("E0205")).map(|(k, v)| format!("{k}={v}")).collect())
                    .unwrap_or_default();
                println!("MESSUNG i [{name}] [{variante}] [{reihenfolge}]: grund={} zahl_cent={} erklaerung {status} {kz:?}", e["grund"], e["zahl_cent"]);
            }
        }
    }
}
