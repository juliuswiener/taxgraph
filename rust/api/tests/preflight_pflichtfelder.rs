//! Abweichung Nr. 47: `GET /fall/{id}/preflight` nennt offene Pflichtfelder als gelben Hinweis (`bereich: pflichtfelder`),
//! statt "GREEN, keine Eintraege" zu melden, waehrend `GET /fall/{id}/deklaration` `pflichtfelder_vollstaendig: false` sagt.
//! Python kennt den Hinweis nicht (`produkt/konsistenz/preflight.py` hat sieben Listen, keine fuer Pflichtfelder).
//!
//! Der Hinweis sperrt nichts und ist kein Freigabe-Kriterium: Er steht in `items`, die Ampel steigt hoechstens von GREEN auf
//! AMBER, RED bleibt RED. Die Quelle ist `elster::pflichtfelder_luecken`, dieselbe Liste, die `/deklaration` liest (7
//! Stammdaten, zwei "alle oder keins"-Gruppen); sie kennt weniger als `ERiC`. Ein fehlender Hinweis heisst deshalb NICHT
//! "abgabefaehig".
//!
//! Gemessen wird ueber die HTTP-Schnittstelle, so wie die Oberflaeche sie liest (`items`, nicht `status`).
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

/// Die sieben Stammdaten der Pflichtliste (`elster::PFLICHTFELDER`, Bedingung "immer").
const STAMMDATEN: [&str; 7] = [
    "stammdaten_nachname",
    "stammdaten_vorname",
    "stammdaten_geburtsdatum",
    "stammdaten_strasse",
    "stammdaten_plz",
    "stammdaten_wohnort",
    "kist_konfession",
];

/// Die Frage, die Person A fragt, ob neben der Rente Arbeitslohn oder eine Pension da ist (Abweichung Nr. 46).
const EINGANGSFRAGE: &str = "neben deiner Rente Arbeitslohn oder eine Pension";

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
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.path().join("users.json"),
        Some(konfig.audit_pfad()),
    );
    let zustand = Zustand::neu(konfig, auth);
    let token = zustand.auth.stelle_aus("alice").unwrap();
    Dienst {
        zustand,
        token,
        _tmp: tmp,
    }
}

async fn sende(d: &Dienst, methode: &str, pfad: &str, body: Option<&Value>) -> (u16, Value) {
    let text = body.map(ToString::to_string);
    let mut b = Request::builder()
        .method(methode)
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token));
    if let Some(t) = &text {
        b = b
            .header("content-type", "application/json")
            .header("content-length", t.len().to_string());
    }
    let req = b.body(text.map_or_else(Body::empty, Body::from)).unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (
        teile.status.as_u16(),
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// Antwortet auf ein Feld und verlangt 201.
async fn antworte(d: &Dienst, feld: &str, wert: Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    let (status, antwort) = sende(d, "POST", "/fall/pf/event", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST {feld}: {antwort}");
}

/// Legt Fall `pf` in der Scheibe an und schreibt die Paare der Reihe nach.
async fn fall_mit(scheibe: &str, paare: &[(&'static str, Value)]) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "pf", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        antworte(&d, feld, wert.clone()).await;
    }
    d
}

async fn rentner_mit(paare: &[(&'static str, Value)]) -> Dienst {
    fall_mit("rentner_gesamt", paare).await
}

async fn vorab(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/pf/preflight", None).await;
    assert_eq!(status, 200, "GET /preflight: {antwort}");
    antwort
}

async fn deklaration(d: &Dienst) -> Value {
    let (status, antwort) = sende(d, "GET", "/fall/pf/deklaration", None).await;
    assert_eq!(status, 200, "GET /deklaration: {antwort}");
    antwort
}

async fn fragen_ids(d: &Dienst) -> Vec<String> {
    let (status, antwort) = sende(d, "GET", "/fall/pf/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {antwort}");
    antwort["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["feld_id"].as_str().unwrap().to_owned())
        .collect()
}

/// Die Eintraege mit `bereich: pflichtfelder`.
fn pflicht_items(v: &Value) -> Vec<&Value> {
    v["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["bereich"] == "pflichtfelder")
        .collect()
}

/// Der Text des einen Pflichtfeld-Eintrags; verlangt genau einen.
fn pflicht_text(v: &Value) -> String {
    let items = pflicht_items(v);
    assert_eq!(items.len(), 1, "genau ein Pflichtfeld-Item erwartet: {v}");
    assert_eq!(items[0]["typ"], "hinweis", "{v}");
    items[0]["text"].as_str().unwrap().to_owned()
}

/// Rentner mit allen Pflichtangaben des Kegels (Einzelveranlagung). Kein Stammdatum, keine Lohn-Gruppe.
fn kegel() -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(2_400_000)),
        ("rentner_renten_beginn_jahr", json!(2025)),
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
        ("kein_p23_verkauf", json!(true)),
    ]
}

fn stammdaten() -> Paare {
    vec![
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("kist_konfession", json!("keine")),
    ]
}

fn mit(mut a: Paare, b: Paare) -> Paare {
    a.extend(b);
    a
}

/// Die ganze Lohn-Gruppe, wie sie auf der Lohnsteuerbescheinigung steht.
fn lohn_gruppe() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(4_000_000)),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(500_000)),
    ]
}

// ------------------------------------------------------------------------------------------------ AK1, AK2

/// Fall C2 der Messung: Kegel ohne Stammdaten. Vorher GREEN und `[]`, obwohl `/deklaration` sieben Luecken nennt.
#[tokio::test]
async fn ohne_stammdaten_meldet_der_vorab_check_gelb_mit_einem_item() {
    let d = rentner_mit(&kegel()).await;
    let v = vorab(&d).await;
    assert_eq!(v["status"], "AMBER", "{v}");
    assert_eq!(v["items"].as_array().unwrap().len(), 1, "{v}");
    let _ = pflicht_text(&v);
}

/// Faelle B1 und B2: der leere Fall, in der Rentner- und in der Gesamt-Scheibe.
#[tokio::test]
async fn der_leere_fall_ist_nicht_gruen() {
    for scheibe in ["rentner_gesamt", "gesamt"] {
        let d = fall_mit(scheibe, &[]).await;
        let v = vorab(&d).await;
        assert_eq!(v["status"], "AMBER", "{scheibe}: {v}");
        let _ = pflicht_text(&v);
    }
}

/// Sieben fehlende Stammdaten sind ein Satz, nicht sieben Eintraege, und der Satz nennt jede Frage.
#[tokio::test]
async fn sieben_stammdaten_sind_ein_item_mit_sieben_fragen() {
    let d = rentner_mit(&[]).await;
    let text = pflicht_text(&vorab(&d).await);
    assert_eq!(text.matches('»').count(), 7, "{text}");
    // Der Rahmensatz sagt, was die Liste bedeutet: ohne ihn stuende nur eine Reihe von Fragen im Kasten.
    assert!(
        text.starts_with(
            "Für die Abgabe fehlen noch Angaben. Bitte beantworte: »Wie lautet dein Nachname?«"
        ),
        "{text}"
    );
    assert!(text.contains("Wie lautet dein Nachname?"), "{text}");
    assert!(text.contains("Kirche"), "{text}");
    // Die letzte Frage haengt mit "und" an, die davor mit Komma.
    assert!(text.contains("?« und »Gehörst du einer Kirche"), "{text}");
    assert!(text.contains("?«, »Wie lautet dein Vorname?"), "{text}");
}

/// Fehlt genau eine Angabe, steht genau diese eine Frage im Satz, ohne "und" und ohne Komma davor. Das ist der
/// haeufigste Fall im Alltag (alles ausgefuellt bis auf eine Angabe).
#[tokio::test]
async fn eine_offene_frage_steht_allein_im_satz() {
    let ohne_nachname: Paare = stammdaten()
        .into_iter()
        .filter(|(f, _)| *f != "stammdaten_nachname")
        .collect();
    let d = rentner_mit(&mit(kegel(), ohne_nachname)).await;
    let v = vorab(&d).await;
    assert_eq!(v["status"], "AMBER", "{v}");
    let text = pflicht_text(&v);
    assert_eq!(text.matches('»').count(), 1, "{text}");
    assert_eq!(
        text,
        "Für die Abgabe fehlen noch Angaben. Bitte beantworte: »Wie lautet dein Nachname?«."
    );
}

// ------------------------------------------------------------------------------------------------ AK2

/// Der Satz nennt Fragen, nie Feldnamen. Auch die zwei Gruppen ("alle oder keins") tragen einen Fragetext: Wer
/// `p36_lohnsteuer` und `vor_an_anteil_rv` nennt, bekommt die uebrigen drei Mitglieder als Fragen genannt.
#[tokio::test]
async fn der_satz_nennt_fragen_nie_feldnamen() {
    let d = rentner_mit(&[
        ("p36_lohnsteuer", json!(500_000)),
        ("vor_an_anteil_rv", json!(0)),
    ])
    .await;
    let text = pflicht_text(&vorab(&d).await);
    // 7 Stammdaten + Lohn + Steuerklasse + vor_ag_anteil_rv.
    assert_eq!(text.matches('»').count(), 10, "{text}");
    assert!(!text.contains('_'), "ein Feldname im Satz: {text}");
    for feld in STAMMDATEN {
        assert!(!text.contains(feld), "{feld} im Satz: {text}");
    }
}

// ------------------------------------------------------------------------------------------------ AK3

/// Fall A1: Stammdaten da, Lohnsteuer genannt, Lohn und Steuerklasse fehlen. Die Frage zu Lohn und Steuerklasse
/// steht noch sichtbar in der Liste.
#[tokio::test]
async fn eine_halb_gefuellte_lohn_gruppe_nennt_lohn_und_steuerklasse() {
    let d = rentner_mit(&mit(
        mit(kegel(), stammdaten()),
        vec![("p36_lohnsteuer", json!(500_000))],
    ))
    .await;
    let v = vorab(&d).await;
    assert_eq!(v["status"], "AMBER", "{v}");
    assert_eq!(v["items"].as_array().unwrap().len(), 1, "{v}");
    let text = pflicht_text(&v);
    assert!(text.contains("Bruttoarbeitslohn"), "{text}");
    assert!(text.contains("Steuerklasse"), "{text}");
    // Die Frage endet am Fragezeichen: der Klammerzusatz dahinter gehoert zum Hilfetext, nicht in den Satz.
    assert!(
        !text.contains("(steht auf der Lohnsteuerbescheinigung)"),
        "{text}"
    );
    assert!(!text.contains("Nachname"), "keine Stammdaten-Luecke: {text}");
    assert!(!text.contains(EINGANGSFRAGE), "Kreuz nicht gesetzt: {text}");
}

/// Fall A3: Kreuz "nein" (kein Lohn, keine Pension), nichts in der Lohn-Gruppe gespeichert. Alles in Ordnung.
#[tokio::test]
async fn ein_nein_zur_eingangsfrage_ohne_lohn_ist_gruen() {
    let d = rentner_mit(&mit(
        mit(kegel(), stammdaten()),
        vec![("kein_lohn_pension", json!(true))],
    ))
    .await;
    let v = vorab(&d).await;
    assert_eq!(v["status"], "GREEN", "{v}");
    assert_eq!(v["items"], json!([]), "{v}");
}

/// Fall A4: Kreuz "nein", aber eine Lohnsteuer ist gespeichert. Lohn und Steuerklasse sind durch das Kreuz verborgen;
/// der Hinweis nennt die Eingangsfrage, nie eine Frage, die der Nutzer nicht mehr sieht.
#[tokio::test]
async fn ein_nein_mit_gespeicherter_lohnsteuer_nennt_die_eingangsfrage() {
    let d = rentner_mit(&mit(
        mit(kegel(), stammdaten()),
        vec![
            ("p36_lohnsteuer", json!(500_000)),
            ("kein_lohn_pension", json!(true)),
        ],
    ))
    .await;
    // Die Voraussetzung: Lohn und Steuerklasse sind wirklich verborgen.
    let sichtbar = fragen_ids(&d).await;
    for feld in ["bruttoarbeitslohn", "steuerklasse", "p36_lohnsteuer"] {
        assert!(!sichtbar.iter().any(|f| f == feld), "{feld} sichtbar");
    }
    let v = vorab(&d).await;
    assert_eq!(v["status"], "AMBER", "{v}");
    let text = pflicht_text(&v);
    assert!(text.contains(EINGANGSFRAGE), "{text}");
    // Der Nutzer hat "nein" angekreuzt; der Satz sagt, welche Antwort er pruefen soll.
    assert!(text.contains("hast du „nein“ angekreuzt"), "{text}");
    assert!(!text.contains("Bruttoarbeitslohn"), "{text}");
    assert!(!text.contains("Steuerklasse"), "{text}");
    assert!(!text.contains('_'), "{text}");
}

/// Wie A4, aber der gespeicherte Wert ist der Lohn: Dann fehlen Steuerklasse UND Lohnsteuer, beide hinter dem Kreuz.
/// Der Satz nennt nur die Eingangsfrage (ein Fragezeichen-Paar), keine der beiden verborgenen Fragen.
#[tokio::test]
async fn ein_nein_mit_gespeichertem_lohn_nennt_nur_die_eingangsfrage() {
    let d = rentner_mit(&mit(
        mit(kegel(), stammdaten()),
        vec![
            ("bruttoarbeitslohn", json!(4_000_000)),
            ("kein_lohn_pension", json!(true)),
        ],
    ))
    .await;
    let sichtbar = fragen_ids(&d).await;
    for feld in ["bruttoarbeitslohn", "steuerklasse", "p36_lohnsteuer"] {
        assert!(!sichtbar.iter().any(|f| f == feld), "{feld} sichtbar");
    }
    let v = vorab(&d).await;
    assert_eq!(v["status"], "AMBER", "{v}");
    let text = pflicht_text(&v);
    assert!(text.contains(EINGANGSFRAGE), "{text}");
    // Genau eine Frage in »«: die Eingangsfrage. Eine genannte Lohnsteuer- oder Steuerklassen-Frage waere die zweite.
    assert_eq!(text.matches('»').count(), 1, "{text}");
    assert!(!text.contains("Bitte beantworte"), "{text}");
}

/// Kreuz "ja" (es gibt Lohn oder Pension): die Lohn-Gruppe ist sichtbar und wird mit ihren Fragen genannt.
#[tokio::test]
async fn ein_ja_zur_eingangsfrage_laesst_die_lohn_gruppe_beim_namen() {
    let d = rentner_mit(&mit(
        mit(kegel(), stammdaten()),
        vec![
            ("p36_lohnsteuer", json!(500_000)),
            ("kein_lohn_pension", json!(false)),
        ],
    ))
    .await;
    let text = pflicht_text(&vorab(&d).await);
    assert!(text.contains("Bruttoarbeitslohn"), "{text}");
    assert!(text.contains("Steuerklasse"), "{text}");
    assert!(!text.contains(EINGANGSFRAGE), "{text}");
}

/// Die Pflichtliste kennt keine Felder des Ehegatten. Eine gespeicherte Lohnsteuer des Ehegatten ohne Lohn ergibt
/// deshalb KEINEN Pflichtfeld-Hinweis, auch mit Kreuz "nein". Das ist die Grenze der Liste (`ERiC` kennt mehr), kein
/// Freibrief: Ein fehlender Hinweis heisst nicht "abgabefaehig".
#[tokio::test]
async fn die_pflichtliste_kennt_den_ehegatten_nicht() {
    let mut paare = kegel();
    paare.retain(|(f, _)| *f != "veranlagung");
    paare.push(("veranlagung", json!("zusammen")));
    paare.extend(vec![
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]);
    paare.extend(stammdaten());
    paare.push(("p36_lohnsteuer_partner", json!(500_000)));
    paare.push(("kein_lohn_pension_partner", json!(true)));
    let d = rentner_mit(&paare).await;
    let v = vorab(&d).await;
    assert!(pflicht_items(&v).is_empty(), "{v}");
}

// ------------------------------------------------------------------------------------------------ AK4

/// Fall C1: vollstaendig. Kein Pflichtfeld-Hinweis; die Ampel steht auf AMBER wegen des anderen Hinweises
/// (Arbeitstage fehlen) und bleibt davon unberuehrt.
#[tokio::test]
async fn ein_vollstaendiger_fall_bekommt_keinen_pflichtfeld_hinweis() {
    let d = rentner_mit(&mit(mit(kegel(), stammdaten()), lohn_gruppe())).await;
    let v = vorab(&d).await;
    assert!(pflicht_items(&v).is_empty(), "{v}");
    assert_eq!(v["status"], "AMBER", "{v}");
    assert_eq!(v["items"][0]["bereich"], "pauschale", "{v}");
}

/// Ein Widerspruch (Lohnsteuer groesser als der Lohn) macht die Ampel rot. Der Pflichtfeld-Hinweis hebt sie nicht an und
/// senkt sie nicht, und er steht hinter den Widerspruechen.
#[tokio::test]
async fn rot_bleibt_rot_und_der_hinweis_steht_hinten() {
    let d = rentner_mit(&mit(
        kegel(),
        vec![
            ("bruttoarbeitslohn", json!(4_000_000)),
            ("steuerklasse", json!("1")),
            ("p36_lohnsteuer", json!(5_000_000)),
        ],
    ))
    .await;
    let v = vorab(&d).await;
    assert_eq!(v["status"], "RED", "{v}");
    let items = v["items"].as_array().unwrap();
    assert_eq!(items[0]["typ"], "widerspruch", "{v}");
    assert_eq!(items.last().unwrap()["bereich"], "pflichtfelder", "{v}");
    assert_eq!(pflicht_items(&v).len(), 1, "{v}");
}

/// Eine Scheibe, die die Stammdaten gar nicht fragt (`ep`), nennt sie auch nicht: Der Nutzer koennte sie dort nicht
/// beantworten.
#[tokio::test]
async fn eine_scheibe_ohne_stammdaten_nennt_sie_nicht() {
    let d = fall_mit("ep", &[]).await;
    let v = vorab(&d).await;
    assert_eq!(v, json!({"fall_id": "pf", "status": "GREEN", "items": []}));
    // `/deklaration` zaehlt die Stammdaten trotzdem als Luecke; der Vorab-Check bleibt bei dem, was die Scheibe fragt.
    assert_eq!(deklaration(&d).await["pflichtfelder_vollstaendig"], false);
}

// ------------------------------------------------------------------------------------------------ Kopplung

/// Das Ticket: `/preflight` und `/deklaration` sagen in der Rentner-Scheibe dasselbe. Item da genau dann, wenn
/// `pflichtfelder_vollstaendig` falsch ist.
#[tokio::test]
async fn vorab_check_und_deklaration_widersprechen_sich_nicht() {
    let faelle: Vec<(&str, Paare)> = vec![
        ("leer", vec![]),
        ("kegel", kegel()),
        ("A1", mit(mit(kegel(), stammdaten()), vec![("p36_lohnsteuer", json!(500_000))])),
        ("A3", mit(mit(kegel(), stammdaten()), vec![("kein_lohn_pension", json!(true))])),
        (
            "A4",
            mit(
                mit(kegel(), stammdaten()),
                vec![("p36_lohnsteuer", json!(500_000)), ("kein_lohn_pension", json!(true))],
            ),
        ),
        ("C1", mit(mit(kegel(), stammdaten()), lohn_gruppe())),
    ];
    for (name, paare) in faelle {
        let d = rentner_mit(&paare).await;
        let luecken = deklaration(&d).await["pflichtfelder_vollstaendig"] == false;
        let hinweis = !pflicht_items(&vorab(&d).await).is_empty();
        assert_eq!(hinweis, luecken, "{name}: Hinweis {hinweis}, Luecke {luecken}");
    }
}
