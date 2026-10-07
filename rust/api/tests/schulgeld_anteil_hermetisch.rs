//! Schulgeld: der Anteil am Hoechstbetrag je Kind (§ 10 Abs. 1 Nr. 9 `EStG`, Kz `E0504603`), gebaut 2026-10-07.
//! Ohne `PARITY=1`, ohne Python, ohne Netz. Python kennt das Feld nicht (Abweichung Nr. 26 in
//! `rust/fixtures/README.md`).
//!
//! **Worum es geht.** Schulgeld darf man zu 30 % absetzen, hoechstens 5.000 EUR je Kind und Elternpaar. Eltern, die nicht
//! zusammen veranlagt werden, teilen diesen Hoechstbetrag; im Normalfall je zur Haelfte. Beantragen beide gemeinsam ein
//! anderes Verhaeltnis, nennt jeder seinen Anteil in Prozent. Die Software fragt ihn je Kind, nur bei Einzelveranlagung.
//!
//! **Warum es zaehlt.** Ohne die Frage rechnet die Software immer hoechstens 2.500 EUR je Kind. Wer das Schulgeld allein
//! traegt und 100 % vereinbart hat, bekommt bis zu 2.500 EUR Sonderausgaben je Kind zu wenig, ohne Hinweis. Umgekehrt darf die
//! Frage den Normalfall nie sperren: "je zur Haelfte" ist die haeufigste Antwort, und eine fehlende Antwort heisst dasselbe.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_p10_1_9_schulgeld_gesamt.yaml` (Feld, `feld_bedingung` auf `veranlagung`),
//! `rust/bescheid/src/abzuege.rs::schulgeld_summe` (Rechnung), `rust/elster/src/deklaration.rs` (Kz nur bei Anteil != 50).
//! Die Rechnung selbst pruefen `rust/bescheid/tests/abzuege_instanzen_hermetisch.rs` und der XML-Weg
//! `rust/elster/tests/instanzen_hermetisch.rs`; hier steht die HTTP-Naht.
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

const ANTEIL: &str = "kind_schulgeld_aufteilung_prozent";

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

/// Legt den Fall `sf` der Scheibe `gesamt` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, paare: &[(&str, Value)]) {
    let kopf = json!({"fall_id": "sf", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) = sende(d, "POST", "/fall/sf/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

fn frage<'a>(fragen: &'a Value, feld: &str) -> Option<&'a Value> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == feld)
}

/// Einzelveranlagung und unbeantwortete Veranlagung zeigen die Frage (mit zwei Kindern: je Kind); Zusammenveranlagung
/// zeigt sie nicht.
#[tokio::test]
async fn die_frage_steht_nur_bei_einzelveranlagung_und_solange_sie_offen_ist() {
    for (veranlagung, erwartet) in [
        (Some("einzel"), true),
        (None, true),
        (Some("zusammen"), false),
    ] {
        let d = dienst();
        let mut paare = vec![("fam_anzahl_kinder", json!(2))];
        if let Some(v) = veranlagung {
            paare.push(("veranlagung", json!(v)));
        }
        fall(&d, &paare).await;
        let (status, fragen) = sende(&d, "GET", "/fall/sf/fragen", None).await;
        assert_eq!(status, 200, "{veranlagung:?}: {fragen}");
        let q = frage(&fragen, ANTEIL);
        assert_eq!(q.is_some(), erwartet, "{veranlagung:?}: {q:?}");
        if let Some(q) = q {
            assert_eq!(q["instanz_anzahl"], json!(2), "{veranlagung:?}: je Kind");
            assert_eq!(q["instanz_etikett"], json!("Kind"));
        }
    }
}

/// Der Schreibweg (`POST /event`) nimmt 0 bis 100 je Kind an (Kind 2 ueber `..__2`) und weist alles andere ab: ausserhalb des
/// Bereichs, `null`, leeren Text, Text, eine Kommazahl, einen Wahrheitswert.
#[tokio::test]
async fn der_schreibweg_nimmt_0_bis_100_je_kind_an_und_weist_den_rest_ab() {
    let d = dienst();
    fall(&d, &[("fam_anzahl_kinder", json!(2)), ("veranlagung", json!("einzel"))]).await;
    for (feld, wert) in [
        (ANTEIL, json!(100)),
        ("kind_schulgeld_aufteilung_prozent__2", json!(0)),
    ] {
        let (status, antwort) = sende(&d, "POST", "/fall/sf/event", Some(&ereignis(feld, &wert))).await;
        assert_eq!(status, 201, "{feld} {wert}: {antwort}");
    }
    for wert in [
        json!(101),
        json!(-1),
        Value::Null,
        json!(""),
        json!("50"),
        json!(50.5),
        json!(true),
    ] {
        let (status, antwort) = sende(&d, "POST", "/fall/sf/event", Some(&ereignis(ANTEIL, &wert))).await;
        assert!(
            (400..500).contains(&status),
            "{wert}: Status {status}, erwartet eine Abweisung: {antwort}"
        );
    }
}

/// Pflicht-Kegel `gesamt` (35 Felder ohne das Kreuz `kein_p23_verkauf`), Einzelveranlagung, 60.000 EUR Arbeitslohn;
/// dieselbe Liste wie `kette_endstand_hermetisch.rs::kegel_gesamt` (aus `tests/_kegel.py::kegel_fuer`).
fn kegel_gesamt() -> Paare {
    vec![
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
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]
}

/// `GET /ergebnis` zum Kegel mit einem Kind, 20.000 EUR Schulgeld (30 % = 6.000 EUR) und dem Anteil (`None`: nie gefragt).
/// Liefert `(grund, zahl_cent)`; die Zahl ist die festgesetzte Steuer in Cent.
async fn steuer(anteil: Option<i64>) -> (String, Option<i64>) {
    let d = dienst();
    let mut paare = kegel_gesamt();
    paare.push(("fam_anzahl_kinder", json!(1)));
    paare.push(("schulgeld", json!(2_000_000)));
    if let Some(n) = anteil {
        paare.push((ANTEIL, json!(n)));
    }
    fall(&d, &paare).await;
    let (status, a) = sende(&d, "GET", "/fall/sf/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {a}");
    (a["grund"].as_str().unwrap_or("").to_owned(), a["zahl_cent"].as_i64())
}

/// Ende zu Ende: ein hoeherer Anteil hebt den Deckel und senkt die Steuer, ein kleinerer senkt den Deckel und hebt sie.
/// 50 und die fehlende Antwort rechnen gleich (je zur Haelfte) und SPERREN NIE: `grund` bleibt `bestaetigt`, eine Zahl steht da.
#[tokio::test]
async fn der_anteil_aendert_die_steuer_und_die_fehlende_antwort_sperrt_nie() {
    let ohne = steuer(None).await;
    let (grund, zahl) = &ohne;
    assert_eq!(grund, "bestaetigt", "ohne Anteil: {ohne:?}");
    let ohne_zahl = zahl.expect("ohne Anteil steht eine Zahl da");

    let fuenfzig = steuer(Some(50)).await;
    assert_eq!(fuenfzig, ohne, "Anteil 50 rechnet wie ohne Anteil");

    let hundert = steuer(Some(100)).await;
    assert_eq!(hundert.0, "bestaetigt", "Anteil 100: {hundert:?}");
    let hundert_zahl = hundert.1.expect("Anteil 100 steht eine Zahl da");
    assert!(
        hundert_zahl < ohne_zahl,
        "Anteil 100 hebt den Abzug von 2.500 auf 5.000 EUR, die Steuer muss sinken: {hundert_zahl} gegen {ohne_zahl} Cent"
    );

    let dreissig = steuer(Some(30)).await;
    assert_eq!(dreissig.0, "bestaetigt", "Anteil 30: {dreissig:?}");
    let dreissig_zahl = dreissig.1.expect("Anteil 30 steht eine Zahl da");
    assert!(
        dreissig_zahl > ohne_zahl,
        "Anteil 30 senkt den Abzug von 2.500 auf 1.500 EUR, die Steuer muss steigen: {dreissig_zahl} gegen {ohne_zahl} Cent"
    );

    let null = steuer(Some(0)).await;
    assert_eq!(null.0, "bestaetigt", "Anteil 0: {null:?}");
    assert!(
        null.1.expect("Anteil 0 steht eine Zahl da") > dreissig_zahl,
        "Anteil 0 gibt gar keinen Abzug: {null:?}"
    );
}
