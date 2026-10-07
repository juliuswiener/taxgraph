//! Geringwertiges Wirtschaftsgut (GWG, § 6 Abs. 2 `EStG`): die Folgefrage "Mehrwertsteuer selbst getragen"
//! (`gwg_ohne_vorsteuerabzug`), gebaut 2026-10-07. Ohne `PARITY=1`, ohne Python, ohne Netz. Python kennt das Feld nicht
//! (Abweichung Nr. 27 in `rust/fixtures/README.md`).
//!
//! **Worum es geht.** Ein Kleinunternehmer bekommt die Mehrwertsteuer nicht zurueck und muss die Frage "Ist der Preis ohne
//! Mehrwertsteuer?" wahrheitsgemaess mit "nein" beantworten. Bis zu diesem Bau sperrte das seine Rechnung (`gwg_mehrwertsteuer_offen`).
//! Jetzt folgt eine Frage: "Hast du die Mehrwertsteuer selbst getragen?" Mit "ja" zaehlt der Preis mit Mehrwertsteuer als Abzug.
//!
//! **Warum es zaehlt.** Ohne die Folgefrage kommt der Kleinunternehmer nie zu einer Zahl; wuerde die Software stattdessen still
//! 0 EUR abziehen, zahlte er bei 790 EUR Geraet rund 279 bis 304 EUR Steuer zu viel (je nach Einkommen). Umgekehrt darf ein
//! "nein", die fehlende Antwort und ein Preis ueber 800 EUR den Abzug nie oeffnen.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_n_vor_gwg.yaml` (Feld mit `feld_bedingung` auf `gwg_netto_ohne_vorsteuer`),
//! `rust/bescheid/src/einkuenfte.rs::gwg_abzug` (Rechnung), `rust/bescheid/src/deklaration/sperre/gesamt.rs::gwg` (Sperre). Die
//! Rechnung und die Sperrfaelle pruefen `rust/bescheid/tests/offene_defekte.rs`; hier steht die HTTP-Naht.
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

const NETTO: &str = "gwg_netto_ohne_vorsteuer";
const FOLGE: &str = "gwg_ohne_vorsteuerabzug";

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

fn ereignis(feld_id: &str, wert: &Value) -> Value {
    json!({
        "feld_id": feld_id,
        "herkunft": {"haftung": "nutzer", "herkunft": "laie", "pruef_tiefe": "ungeprueft"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld_id}")},
        "wert": wert,
        "zustand": "bestaetigt",
        "ts": "2026-01-01T00:00:00+00:00",
    })
}

/// Legt den Fall `gw` der Scheibe `gesamt` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, paare: &[(&str, Value)]) {
    let kopf = json!({"fall_id": "gw", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) = sende(d, "POST", "/fall/gw/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Pflicht-Kegel `gesamt` (35 Felder), Einzelveranlagung, 60.000 EUR Arbeitslohn, mit offenem EUeR-Weg (`kein_gewinn` nein, drei
/// Nullen), damit die GWG-Zeile das Einzige ist, das die Steuer bewegt; dieselbe Liste wie
/// `kette_endstand_hermetisch.rs::kegel_gesamt` mit dem Gewinn-Kreuz auf "nein" (aus `tests/_kegel.py::kegel_fuer`).
fn kegel() -> Paare {
    let mut p: Paare = vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(6_000_000)),
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
        ("kein_gewinn", json!(false)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("betriebseinnahmen", json!(0)),
        ("afa_jahresbetrag", json!(0)),
    ];
    p.push(("sonstige_betriebsausgaben", json!(0)));
    p
}

/// Kegel mit `sonstige` Cent als sonstige Betriebsausgabe (die Referenz ohne Geraet).
fn kegel_mit_sonstigen(sonstige_cent: i64) -> Paare {
    kegel()
        .into_iter()
        .map(|(f, w)| {
            if f == "sonstige_betriebsausgaben" {
                (f, json!(sonstige_cent))
            } else {
                (f, w)
            }
        })
        .collect()
}

/// Ein Geraet (Instanz 1) mit Betrag und den Antworten; `None` = nicht beantwortet.
fn geraet(betrag_cent: i64, netto: Option<bool>, folge: Option<bool>) -> Paare {
    let mut p: Paare = vec![
        ("gwg_anzahl", json!(1)),
        ("gwg_anschaffungskosten_netto", json!(betrag_cent)),
        ("gwg_bewegliches_selbstaendig_nutzbar", json!(true)),
        ("gwg_verzeichnis_ab_250", json!(true)),
    ];
    if let Some(n) = netto {
        p.push((NETTO, json!(n)));
    }
    if let Some(f) = folge {
        p.push((FOLGE, json!(f)));
    }
    p
}

fn fragen_ids(fragen: &Value) -> Vec<&str> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|q| q["feld_id"].as_str())
        .collect()
}

/// Die Folgefrage steht, solange die Netto-Frage offen ist (fail-closed) oder "nein" sagt, und faellt weg, sobald die Netto-Frage
/// "ja" ist. Sie steht HINTER der Netto-Frage.
#[tokio::test]
async fn die_folgefrage_steht_nur_wenn_die_netto_frage_nicht_mit_ja_beantwortet_ist() {
    for (netto, erwartet) in [(None, true), (Some(false), true), (Some(true), false)] {
        let d = dienst();
        let mut paare = kegel();
        paare.extend(geraet(79_000, netto, None));
        fall(&d, &paare).await;
        let (status, fragen) = sende(&d, "GET", "/fall/gw/fragen", None).await;
        assert_eq!(status, 200, "Netto-Frage {netto:?}: {fragen}");
        let ids = fragen_ids(&fragen);
        assert_eq!(ids.contains(&FOLGE), erwartet, "Netto-Frage {netto:?}: {ids:?}");
        if netto.is_none() {
            assert!(ids.contains(&NETTO), "KONTROLLE: die Netto-Frage selbst steht noch: {ids:?}");
        }
        if let (Some(a), Some(b)) = (
            ids.iter().position(|f| *f == NETTO),
            ids.iter().position(|f| *f == FOLGE),
        ) {
            assert!(a < b, "die Netto-Frage kommt vor ihrer Folgefrage: {ids:?}");
        }
    }
}

/// Zwei Geraete, das erste mit Netto-Frage "ja", das zweite mit "nein": die Bedingung gilt ueber ALLE Geraete (`feld_bedingung`
/// schliesst erst aus, wenn jede Instanz bestaetigt abweicht), die Folgefrage steht also, und zwar fuer beide Geraete
/// (`instanz_anzahl` 2). Die Antwort zum ersten Geraet zaehlt nicht (Netto-Frage "ja"); das ist eine Frage zu viel, kein Fehler
/// der Rechnung. Pinnt die Grenze der Bedingung: wird sie je Instanz, faellt die Frage fuer das erste Geraet weg.
#[tokio::test]
async fn die_bedingung_gilt_ueber_alle_geraete_die_frage_steht_fuer_beide() {
    let d = dienst();
    let mut paare = kegel();
    paare.extend([
        ("gwg_anzahl", json!(2)),
        ("gwg_anschaffungskosten_netto", json!(50_000)),
        ("gwg_bewegliches_selbstaendig_nutzbar", json!(true)),
        ("gwg_verzeichnis_ab_250", json!(true)),
        (NETTO, json!(true)),
        ("gwg_anschaffungskosten_netto__2", json!(79_000)),
        ("gwg_bewegliches_selbstaendig_nutzbar__2", json!(true)),
        ("gwg_verzeichnis_ab_250__2", json!(true)),
        ("gwg_netto_ohne_vorsteuer__2", json!(false)),
    ]);
    fall(&d, &paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/gw/fragen", None).await;
    assert_eq!(status, 200, "{fragen}");
    let q = fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == FOLGE)
        .unwrap_or_else(|| panic!("die Folgefrage steht nicht: {:?}", fragen_ids(&fragen)));
    assert_eq!(q["instanz_anzahl"], json!(2), "fuer beide Geraete: {q}");
}

/// Der Schreibweg (`POST /event`) nimmt `true` und `false` an und weist alles andere mit dem Grund `(Typ)` ab.
#[tokio::test]
async fn der_schreibweg_nimmt_ja_und_nein_an_und_weist_den_rest_ab() {
    for wert in [json!(true), json!(false)] {
        let d = dienst();
        let mut paare = kegel();
        paare.extend(geraet(79_000, Some(false), None));
        fall(&d, &paare).await;
        let (status, antwort) = sende(&d, "POST", "/fall/gw/event", Some(&ereignis(FOLGE, &wert))).await;
        assert_eq!(status, 201, "{wert}: {antwort}");
    }
    let d = dienst();
    let mut paare = kegel();
    paare.extend(geraet(79_000, Some(false), None));
    fall(&d, &paare).await;
    for wert in [json!(1), json!("ja"), json!(""), json!(0.5), Value::Null] {
        let (status, antwort) = sende(&d, "POST", "/fall/gw/event", Some(&ereignis(FOLGE, &wert))).await;
        assert_eq!(status, 422, "{wert}: Status {status}, erwartet 422: {antwort}");
        let text = antwort["fehler"].as_str().unwrap_or_default();
        assert!(text.contains("(Typ)"), "{wert}: erwartet den Grund (Typ), Antwort: {antwort}");
    }
}

/// `GET /ergebnis`: `(grund, zahl_cent)`.
async fn ergebnis(paare: &[(&str, Value)]) -> (String, Option<i64>) {
    let d = dienst();
    fall(&d, paare).await;
    let (status, a) = sende(&d, "GET", "/fall/gw/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {a}");
    (a["grund"].as_str().unwrap_or("").to_owned(), a["zahl_cent"].as_i64())
}

/// Ende zu Ende: der Kleinunternehmer (netto nein, Folgefrage ja, 790 EUR) rechnet dieselbe Steuer wie ein Fall, der 790 EUR als
/// sonstige Betriebsausgabe fuehrt, und die Steuer liegt unter der des Falls ganz ohne Geraet. Ein "nein", die fehlende Antwort
/// und ein Preis ueber 800 EUR sperren mit `gwg_mehrwertsteuer_offen` und liefern keine Zahl.
#[tokio::test]
async fn die_folgefrage_oeffnet_den_abzug_nur_mit_ja_und_bis_800_euro() {
    let (g_ohne, ohne) = ergebnis(&kegel()).await;
    let (g_ref, referenz) = ergebnis(&kegel_mit_sonstigen(79_000)).await;
    assert_eq!((g_ohne.as_str(), g_ref.as_str()), ("bestaetigt", "bestaetigt"), "KONTROLLE: die Referenzen rechnen");
    let (ohne, referenz) = (ohne.unwrap(), referenz.unwrap());
    assert!(referenz < ohne, "KONTROLLE: 790 EUR Abzug senken die Steuer: {referenz} gegen {ohne} Cent");

    let mut ja = kegel();
    ja.extend(geraet(79_000, Some(false), Some(true)));
    assert_eq!(ergebnis(&ja).await, ("bestaetigt".to_owned(), Some(referenz)), "Kleinunternehmer, 790 EUR");

    for (name, betrag, folge) in [
        ("Folgefrage nein", 79_000, Some(false)),
        ("Folgefrage unbeantwortet", 79_000, None),
        ("Folgefrage ja, 850 EUR brutto", 85_000, Some(true)),
    ] {
        let mut p = kegel();
        p.extend(geraet(betrag, Some(false), folge));
        assert_eq!(
            ergebnis(&p).await,
            ("gwg_mehrwertsteuer_offen".to_owned(), None),
            "{name}"
        );
    }
}
