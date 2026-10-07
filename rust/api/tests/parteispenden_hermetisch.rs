//! Spenden an politische Parteien (`parteispenden_betrag`, gebaut 2026-10-07): die HTTP-Naht. Ohne `PARITY=1`, ohne Python, ohne
//! Netz, ohne `ERiC`. Python kennt das Feld nicht (Abweichung Nr. 31 in `rust/fixtures/README.md`).
//!
//! **Worum es geht.** Der Dialog fragte schon vorher "Hast du ... an gemeinnuetzige Vereine, Stiftungen, Kirchen oder Parteien
//! gespendet?" und nahm danach nur EINEN Betrag entgegen, dessen Fragetext die Parteien nicht mehr nannte. Wer eine
//! Parteispende dort eintrug, bekam sie als gewoehnliche Spende gerechnet (154 Euro Ersparnis bei 500 Euro, 45.000 Euro Lohn,
//! ledig); wer sie wegliess, bekam nichts. Zustehend sind 250 Euro: eine Spende an eine Partei senkt die Steuer um die Haelfte.
//!
//! **Warum es zaehlt.** Die Frage versprach etwas, das die Software nicht annehmen konnte, und beide Auswege kosteten Geld oder
//! machten eine falsche Angabe in der Erklaerung. Hier stehen Frage, Fragetexte (AK3), Schreibweg, Steuer (AK2) und die Wirkung
//! des Screenings.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_sonder_agb_35a.yaml` (Feld, Texte), `rust/bescheid/src/abzuege.rs::
//! steuerermaessigungen` (Rechnung), `params/<vz>/parteispenden_p34g.yaml` (Deckel). Die Rechnung im Einzelnen prueft
//! `rust/bescheid/tests/parteispenden_ermaessigung.rs`, den Kz im XML `parteispenden_einreichung_hermetisch.rs`.
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

const FELD: &str = "parteispenden_betrag";
const ALLGEMEIN: &str = "spenden_betrag";

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
    std::fs::create_dir_all(&konfig.faelle).unwrap();
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

/// Legt den Fall `ps` der Scheibe `gesamt` im Jahr `vz` an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, vz: i64, paare: &[(&str, Value)]) {
    let kopf = json!({"fall_id": "ps", "scheibe": "gesamt", "veranlagungszeitraum": vz});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) =
            sende(d, "POST", "/fall/ps/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Pflicht-Kegel `gesamt` (35 Felder), Einzelveranlagung, `lohn_cent` Arbeitslohn, EUeR-Weg offen (wie
/// `unfallkosten_hermetisch.rs::kegel`, aber ohne Arbeitsweg: `ep_arbeitstage` 0).
fn kegel(lohn_cent: i64) -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(lohn_cent)),
        ("ep_arbeitstage", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("ep_entfernung_km", json!(0)),
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

/// Der Kegel mit weiteren Paaren.
fn mit(lohn_cent: i64, mehr: Paare) -> Paare {
    let mut p = kegel(lohn_cent);
    p.extend(mehr);
    p
}

async fn fragen(d: &Dienst) -> Vec<Value> {
    let (status, a) = sende(d, "GET", "/fall/ps/fragen", None).await;
    assert_eq!(status, 200, "{a}");
    a["fragen"].as_array().unwrap().clone()
}

fn frage<'a>(alle: &'a [Value], feld: &str) -> Option<&'a Value> {
    alle.iter().find(|q| q["feld_id"] == feld)
}

async fn ids(paare: &[(&str, Value)]) -> Vec<String> {
    let d = dienst();
    fall(&d, 2025, paare).await;
    fragen(&d)
        .await
        .iter()
        .filter_map(|q| q["feld_id"].as_str().map(str::to_owned))
        .collect()
}

/// AK1 und die Wirkung des Screenings: die Frage steht, solange das Screening nicht "keine Spenden" bestaetigt. Bestaetigt der
/// Nutzer "keine Spenden", fallen BEIDE Betragsfragen weg (die Regel ist ausgeschlossen); sagt er "ja, gespendet" oder schweigt
/// er, bleiben beide (fail-closed: Schweigen schliesst nichts aus).
#[tokio::test]
async fn die_frage_steht_solange_das_screening_nicht_keine_spenden_sagt() {
    let stumm = ids(&kegel(6_000_000)).await;
    assert!(
        stumm.iter().any(|f| f == FELD),
        "Screening unbeantwortet: {stumm:?}"
    );
    assert!(
        stumm.iter().any(|f| f == ALLGEMEIN),
        "KONTROLLE: die allgemeine Spendenfrage steht dort ebenfalls: {stumm:?}"
    );

    // `keine_spenden` benennt die Abwesenheit: false heisst "ja, ich habe gespendet".
    let gespendet = ids(&mit(6_000_000, vec![("keine_spenden", json!(false))])).await;
    assert!(
        gespendet.iter().any(|f| f == FELD),
        "gespendet: {gespendet:?}"
    );
    assert!(gespendet.iter().any(|f| f == ALLGEMEIN), "{gespendet:?}");

    let keine = ids(&mit(6_000_000, vec![("keine_spenden", json!(true))])).await;
    assert!(
        !keine.is_empty(),
        "KONTROLLE: der Katalog ist nicht leer, die Frage fehlt nicht nur, weil nichts mehr gefragt wird"
    );
    assert!(
        !keine.iter().any(|f| f == FELD),
        "keine Spenden bestaetigt, und die Parteispenden-Frage steht doch: {keine:?}"
    );
    assert!(
        !keine.iter().any(|f| f == ALLGEMEIN),
        "KONTROLLE: die allgemeine Spendenfrage faellt ebenfalls weg: {keine:?}"
    );
}

/// AK3: Screening-Frage und Folgefragen widersprechen sich nicht. Was das Screening verspricht (Vereine, Stiftungen, Kirchen,
/// Parteien), nimmt je eine Folgefrage entgegen: die allgemeine Frage nennt Vereine, Stiftungen und Kirchen und schliesst
/// Parteien ausdruecklich aus, die Parteien-Frage nennt Parteien. Kein Fragetext nennt einen Paragrafen.
#[tokio::test]
async fn screening_und_folgefragen_widersprechen_sich_nicht() {
    let d = dienst();
    fall(&d, 2025, &kegel(6_000_000)).await;
    let alle = fragen(&d).await;
    let screening = frage(&alle, "keine_spenden")
        .unwrap_or_else(|| panic!("das Screening steht nicht in den Fragen"))["fragetext_laie"]
        .as_str()
        .unwrap()
        .to_owned();
    let allgemein = frage(&alle, ALLGEMEIN).unwrap()["fragetext_laie"]
        .as_str()
        .unwrap()
        .to_owned();
    let partei = frage(&alle, FELD)
        .unwrap_or_else(|| panic!("die Parteispenden-Frage steht nicht: {alle:?}"))
        ["fragetext_laie"]
        .as_str()
        .unwrap()
        .to_owned();

    for wort in ["Vereine", "Stiftungen", "Kirchen", "Parteien"] {
        assert!(
            screening.contains(wort),
            "KONTROLLE: das Screening nennt `{wort}`: {screening}"
        );
    }
    for wort in ["Vereine", "Stiftungen", "Kirchen"] {
        assert!(
            allgemein.contains(wort),
            "die allgemeine Frage nennt `{wort}` nicht: {allgemein}"
        );
    }
    assert!(
        allgemein.contains("ohne Spenden an Parteien"),
        "die allgemeine Frage schliesst Parteien nicht aus, so dass der Nutzer sie hier eintraegt: {allgemein}"
    );
    assert!(
        partei.contains("Parteien"),
        "die Parteispenden-Frage nennt Parteien nicht: {partei}"
    );
    assert!(
        !partei.contains("Vereine") && !partei.contains("Kirchen"),
        "die Parteispenden-Frage meint nur Parteien: {partei}"
    );
    for text in [&screening, &allgemein, &partei] {
        for verboten in ["§", "EStG", "Abs."] {
            assert!(
                !text.contains(verboten),
                "Fragetext mit `{verboten}`: {text}"
            );
        }
    }
}

/// Typ, Einheit, Anker und Hilfetext der neuen Frage: ein Betrag in Euro, verankert in § 34g (eingefrorener Wortlaut), der
/// Hilfetext sagt, dass die Spende die Steuer mindert und Mitgliedsbeitraege zaehlen.
#[tokio::test]
async fn die_neue_frage_ist_ein_betrag_mit_anker_in_paragraf_34g() {
    let d = dienst();
    fall(&d, 2025, &kegel(6_000_000)).await;
    let alle = fragen(&d).await;
    let q = frage(&alle, FELD).unwrap_or_else(|| panic!("die Frage steht nicht"));
    assert_eq!(
        (q["typ"].as_str(), q["einheit"].as_str()),
        (Some("cent"), Some("EUR")),
        "{q}"
    );
    let anker = q["anker_ref"]["quelle"].as_str().unwrap();
    assert!(anker.contains("34g"), "der Anker nennt § 34g nicht: {q}");
    let datei = q["anker_ref"]["datei"].as_str().unwrap();
    assert!(
        datei.starts_with("sources/gesetze-im-internet/estg_p34g_"),
        "der Anker zeigt nicht auf den eingefrorenen § 34g: {q}"
    );
    let hilfe = q["hilfe_kurz"].as_str().unwrap();
    for teil in ["Mitgliedsbeiträge", "Steuer"] {
        assert!(hilfe.contains(teil), "Hilfetext ohne `{teil}`: {hilfe}");
    }
}

/// Der Schreibweg (`POST /event`) nimmt Betraege ab 0 an und weist negative, Text-, Dezimal- und leere Werte mit 422 ab.
#[tokio::test]
async fn der_schreibweg_nimmt_betraege_an_und_weist_den_rest_ab() {
    // Ein frischer Fall je Wert: ein zweites Event auf dasselbe Feld verlangt `ersetzt` und wiese jeden Wert ab.
    for wert in [json!(0), json!(50_000)] {
        let d = dienst();
        fall(&d, 2025, &kegel(4_500_000)).await;
        let (status, antwort) =
            sende(&d, "POST", "/fall/ps/event", Some(&ereignis(FELD, &wert))).await;
        assert_eq!(status, 201, "{wert}: {antwort}");
    }
    for wert in [json!(-1), json!("viel"), json!(1.5), Value::Null] {
        let d = dienst();
        fall(&d, 2025, &kegel(4_500_000)).await;
        let (status, antwort) =
            sende(&d, "POST", "/fall/ps/event", Some(&ereignis(FELD, &wert))).await;
        assert_eq!(
            status, 422,
            "{wert}: Status {status}, erwartet 422: {antwort}"
        );
        let text = antwort["fehler"].as_str().unwrap_or_default();
        assert!(
            !text.contains("aktives Event") && !text.contains("nicht in dieser Scheibe"),
            "{wert}: abgewiesen aus dem falschen Grund: {antwort}"
        );
    }
}

/// `GET /ergebnis`: `(grund, zahl_cent)`.
async fn ergebnis(vz: i64, paare: &[(&str, Value)]) -> (String, Option<i64>) {
    let d = dienst();
    fall(&d, vz, paare).await;
    let (status, a) = sende(&d, "GET", "/fall/ps/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {a}");
    (
        a["grund"].as_str().unwrap_or("").to_owned(),
        a["zahl_cent"].as_i64(),
    )
}

/// AK2 Ende zu Ende (der Differenzlauf des Backlogs, hier ueber die echte Route statt `API._bescheid_fn`): 500 Euro Parteispende
/// bei 45.000 Euro Arbeitslohn, ledig, senken die Steuer um GENAU 250 Euro. Die allgemeine Spende senkt sie weniger (die
/// Referenz "heute": das Einkommen sinkt um 500 Euro, nicht die Steuer um 250).
#[tokio::test]
async fn fuenfhundert_euro_an_eine_partei_senken_die_steuer_um_zweihundertfuenfzig() {
    let (g0, ohne) = ergebnis(2025, &kegel(4_500_000)).await;
    let (g1, partei) = ergebnis(2025, &mit(4_500_000, vec![(FELD, json!(50_000))])).await;
    let (g2, allgemein) = ergebnis(2025, &mit(4_500_000, vec![(ALLGEMEIN, json!(50_000))])).await;
    assert_eq!(
        (g0.as_str(), g1.as_str(), g2.as_str()),
        ("bestaetigt", "bestaetigt", "bestaetigt"),
        "KONTROLLE: die drei Laeufe rechnen"
    );
    let (ohne, partei, allgemein) = (ohne.unwrap(), partei.unwrap(), allgemein.unwrap());
    assert_eq!(
        ohne - partei,
        25_000,
        "500 Euro Parteispende: 250 Euro weniger Steuer (Cent)"
    );
    assert!(
        0 < ohne - allgemein && ohne - allgemein < 25_000,
        "KONTROLLE: die allgemeine Spende von 500 Euro spart weniger als 250 Euro: {} Cent",
        ohne - allgemein
    );
    // 0 und leer aendern die Zahl nicht.
    let (_, null) = ergebnis(2025, &mit(4_500_000, vec![(FELD, json!(0))])).await;
    assert_eq!(null, Some(ohne), "Parteispende 0");
}

/// Der Deckel je Jahr ueber die echte Route: 4.000 Euro Parteispende, ledig, geben 825 Euro (2025) und 1.650 Euro (2026) weniger
/// Steuer (Wortlaut 2025-12-18 und 2026-09-26). Beide Jahre rechnen mit demselben Fall.
#[tokio::test]
async fn der_deckel_gilt_je_veranlagungsjahr() {
    for (vz, erwartet) in [(2025, 82_500), (2026, 165_000)] {
        let (_, ohne) = ergebnis(vz, &kegel(4_500_000)).await;
        let (g, mit_spende) = ergebnis(vz, &mit(4_500_000, vec![(FELD, json!(400_000))])).await;
        assert_eq!(g, "bestaetigt", "{vz}");
        assert_eq!(
            ohne.unwrap() - mit_spende.unwrap(),
            erwartet,
            "{vz}: 4.000 Euro Parteispende"
        );
    }
}

/// Die Ermaessigung senkt die Steuer hoechstens auf 0, nie darunter (§ 34g mindert "die tarifliche Einkommensteuer"): bei
/// 14.000 Euro Lohn liegt die Steuer unter 250 Euro, die Spende von 500 Euro macht sie zu 0, keine Erstattung.
#[tokio::test]
async fn die_ermaessigung_macht_die_steuer_nie_negativ() {
    let (_, ohne) = ergebnis(2025, &kegel(1_400_000)).await;
    let ohne = ohne.unwrap();
    assert!(
        0 < ohne && ohne < 25_000,
        "KONTROLLE: bei 14.000 Euro Lohn liegt die Steuer zwischen 0 und 250 Euro, erhalten {ohne} Cent"
    );
    let (g, mit_spende) = ergebnis(2025, &mit(1_400_000, vec![(FELD, json!(50_000))])).await;
    assert_eq!(g, "bestaetigt");
    assert_eq!(
        mit_spende,
        Some(0),
        "die Ermaessigung ueberschreitet die Steuer nicht"
    );
}
