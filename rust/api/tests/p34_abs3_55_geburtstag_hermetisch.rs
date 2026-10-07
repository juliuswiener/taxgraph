//! § 34 Abs. 3 `EStG`, das 55. Lebensjahr im Jahr der Vollendung (Julius 2026-10-07, Abweichung Nr. 32): wer die Frage sieht,
//! was der Schreibweg mit der Antwort tut und was `GET /ergebnis` daraus macht. Ohne `PARITY=1`, ohne Python, ohne Netz.
//! Python kennt die zwei Felder nicht.
//!
//! **Worum es geht.** Wer seinen Betrieb verkauft, kann einmal im Leben einen ermaessigten Steuersatz beantragen, wenn er das
//! 55. Lebensjahr vollendet hat. Die Software kennt nur das Geburtsjahr. Im Jahr, in dem jemand nach seinem Geburtsjahr 55
//! wird, kann der Geburtstag vor oder nach dem Verkauf liegen. Nur in diesem Jahr fragt die Software nach, je Person.
//!
//! **Warum es zaehlt.** Ohne die Frage rechnet die Software dem Verkaeufer den ermaessigten Satz, den das Finanzamt noch nicht
//! gibt: unten 114.946 statt 191.188 Euro Steuer. Umgekehrt darf die Frage niemanden sperren, der sie nicht sehen sollte: wer
//! in einem anderen Jahr 55 wird oder war, sieht sie nicht, und ohne Antwort rechnet die Software wie bisher.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_an_gesamt.yaml` (`alter_55_vor_verkauf`, `..._partner`, `feld_bedingung
//! alter_im_vz`), `rust/interview/src/fragen.rs::feld_ausgeschlossen` (die Bedingung), `rust/bescheid/src/abzuege.rs`
//! (`abs3_berechtigt`). Die Handrechnung steht in `rust/bescheid/tests/p34_partner_hermetisch.rs`.
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
    })
}

type Paare = Vec<(&'static str, Value)>;

const A: &str = "alter_55_vor_verkauf";
const PARTNER: &str = "alter_55_vor_verkauf_partner";

fn frage<'a>(fragen: &'a Value, feld: &str) -> Option<&'a Value> {
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .find(|q| q["feld_id"] == feld)
}

/// Legt den Fall `lj` (Scheibe `gesamt`, VZ 2025) an und schreibt die Paare als bestaetigte Ereignisse ueber `POST /event`.
async fn fall_mit(d: &Dienst, paare: &[(&'static str, Value)]) {
    let kopf = json!({"fall_id": "lj", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let (status, antwort) = sende(d, "POST", "/fall/lj/event", Some(&ereignis(feld, wert))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Sieht der Fall die Frage `feld`? Ein Aufruf von `GET /fragen`.
async fn sieht(paare: &[(&'static str, Value)], feld: &str) -> bool {
    let d = dienst();
    fall_mit(&d, paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/lj/fragen", None).await;
    assert_eq!(status, 200, "{paare:?}: {fragen}");
    frage(&fragen, feld).is_some()
}

/// Die Frage steht nur im Jahr der Vollendung: Veranlagungsjahr 2025 minus Geburtsjahr gleich 55, also geboren 1970. Davor
/// (1971, 1990), danach (1969, 1960) und mit anderem Geburtsjahr steht sie nicht. Ohne Geburtsjahr steht sie (fail-closed:
/// ein unbekanntes Alter schliesst die Frage nicht aus). Je Person nach dem EIGENEN Geburtsjahr.
#[tokio::test]
async fn die_frage_steht_nur_im_jahr_der_vollendung() {
    for (gj, erwartet) in [(1970, true), (1969, false), (1971, false), (1960, false), (1990, false), (1900, false)] {
        assert_eq!(sieht(&[("geburtsjahr", json!(gj))], A).await, erwartet, "A, geboren {gj}");
        assert_eq!(
            sieht(&[("veranlagung", json!("zusammen")), ("geburtsjahr_partner", json!(gj))], PARTNER).await,
            erwartet,
            "Partner, geboren {gj}"
        );
    }
    assert!(sieht(&[], A).await, "A ohne Geburtsjahr: die Frage bleibt (fail-closed)");
    assert!(
        sieht(&[("veranlagung", json!("zusammen"))], PARTNER).await,
        "Partner ohne Geburtsjahr: die Frage bleibt (fail-closed)"
    );
}

/// Das Geburtsjahr kann auch aus dem Geburtsdatum kommen (die Stammdaten fragen es ohnehin, das Geburtsjahr wird daraus
/// abgeleitet und nicht mehr gefragt): geboren am 15.06.1970 steht die Frage, am 01.03.1971 nicht.
#[tokio::test]
async fn das_geburtsjahr_aus_dem_geburtsdatum_entscheidet_ebenso() {
    for (datum, erwartet) in [("15.06.1970", true), ("01.03.1971", false), ("31.12.1969", false)] {
        assert_eq!(sieht(&[("stammdaten_geburtsdatum", json!(datum))], A).await, erwartet, "A, {datum}");
        assert_eq!(
            sieht(
                &[("veranlagung", json!("zusammen")), ("stammdaten_geburtsdatum_partner", json!(datum))],
                PARTNER
            )
            .await,
            erwartet,
            "Partner, {datum}"
        );
    }
}

/// Die zwei Personen sind unabhaengig: A im Vollendungsjahr, der Partner nicht, und umgekehrt.
#[tokio::test]
async fn die_frage_gilt_je_person_nach_dem_eigenen_geburtsjahr() {
    let d = dienst();
    fall_mit(
        &d,
        &[
            ("veranlagung", json!("zusammen")),
            ("geburtsjahr", json!(1970)),
            ("geburtsjahr_partner", json!(1980)),
        ],
    )
    .await;
    let (_, fragen) = sende(&d, "GET", "/fall/lj/fragen", None).await;
    assert!(frage(&fragen, A).is_some(), "A wird 2025 55");
    assert!(frage(&fragen, PARTNER).is_none(), "der Partner wird 2025 45");
    let d = dienst();
    fall_mit(
        &d,
        &[
            ("veranlagung", json!("zusammen")),
            ("geburtsjahr", json!(1980)),
            ("geburtsjahr_partner", json!(1970)),
        ],
    )
    .await;
    let (_, fragen) = sende(&d, "GET", "/fall/lj/fragen", None).await;
    assert!(frage(&fragen, A).is_none(), "A wird 2025 45");
    assert!(frage(&fragen, PARTNER).is_some(), "der Partner wird 2025 55");
}

/// Die Frage des Partners steht bei Einzelveranlagung nicht (die ganze Regel faellt weg), solange die Veranlagung offen
/// ist und der Partner im Vollendungsjahr steht, steht sie. Frage- und Hilfetext nennen die Person und die Zahl 55.
#[tokio::test]
async fn die_partnerfrage_folgt_der_veranlagung_und_die_texte_nennen_die_person() {
    assert!(!sieht(&[("veranlagung", json!("einzel")), ("geburtsjahr_partner", json!(1970))], PARTNER).await);
    assert!(sieht(&[("geburtsjahr_partner", json!(1970))], PARTNER).await, "Veranlagung offen: die Frage bleibt");
    let d = dienst();
    fall_mit(
        &d,
        &[
            ("veranlagung", json!("zusammen")),
            ("geburtsjahr", json!(1970)),
            ("geburtsjahr_partner", json!(1970)),
        ],
    )
    .await;
    let (_, fragen) = sende(&d, "GET", "/fall/lj/fragen", None).await;
    for (feld, wort) in [(A, "du"), (PARTNER, "partner")] {
        let q = frage(&fragen, feld).unwrap_or_else(|| panic!("{feld} fehlt"));
        assert_eq!(q["typ"], "bool", "{feld}");
        let text = q["fragetext_laie"].as_str().unwrap();
        assert!(text.contains("55") && text.contains(wort), "{feld}: {text}");
    }
}

/// Der Schreibweg (`POST /event`): beide Angaben sind Ja/Nein und nehmen nur Ja/Nein an, `null` und Text nicht.
#[tokio::test]
async fn der_schreibweg_nimmt_nur_ja_nein_an() {
    let d = dienst();
    fall_mit(&d, &[("veranlagung", json!("zusammen"))]).await;
    for feld in [A, PARTNER] {
        let (status, antwort) = sende(&d, "POST", "/fall/lj/event", Some(&ereignis(feld, &json!(false)))).await;
        assert_eq!(status, 201, "{feld}: {antwort}");
        for schlecht in [json!("ja"), json!(1), Value::Null] {
            let (status, antwort) = sende(&d, "POST", "/fall/lj/event", Some(&ereignis(feld, &schlecht))).await;
            assert!(
                (400..500).contains(&status),
                "{feld} {schlecht}: Status {status}, erwartet eine Abweisung: {antwort}"
            );
        }
    }
}

// ---------------------------------------------------------------- die Steuer und die Sperre ueber GET /ergebnis

/// Zusammenveranlagung, A mit 60.000 Euro Lohn; der Partner ohne Lohn. Alles Weitere setzt der Fall.
fn zusammen_basis() -> Paare {
    mit(
        kegel_gesamt(),
        vec![
            ("veranlagung", json!("zusammen")),
            ("bruttoarbeitslohn_partner", json!(0)),
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
            ("kein_gewinn", json!(false)),
        ],
    )
}

/// Person mit 500.000 Euro Gewinn (Gewerbe), im Vollendungsjahr (geboren 1970), beantragt, nie genutzt. `bu` und `antwort`
/// sind je nach Fall gesetzt oder fehlen.
fn person(partner: bool, antwort: Option<bool>, bu: Option<bool>) -> Paare {
    let (gewinn, art, alter, frei, antrag, gj, einmal, buf, ant): (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str, &'static str) =
        if partner {
            (
                "rentner_veraeusserungsgewinn_partner",
                "rentner_veraeusserungs_betriebsart_partner",
                "rentner_alter_55_oder_berufsunfaehig_partner",
                "rentner_freibetrag_erstmalig_partner",
                "antrag_ermaessigter_satz_partner",
                "geburtsjahr_partner",
                "ermaessigung_einmal_genutzt_partner",
                "dauernd_berufsunfaehig_partner",
                PARTNER,
            )
        } else {
            (
                "rentner_veraeusserungsgewinn",
                "rentner_veraeusserungs_betriebsart",
                "rentner_alter_55_oder_berufsunfaehig",
                "rentner_freibetrag_erstmalig",
                "antrag_ermaessigter_satz",
                "geburtsjahr",
                "ermaessigung_einmal_genutzt",
                "dauernd_berufsunfaehig",
                A,
            )
        };
    let mut p: Paare = vec![
        (gewinn, json!(50_000_000)),
        (art, json!("gewerbe")),
        (alter, json!(true)),
        (frei, json!(true)),
        (antrag, json!(true)),
        (gj, json!(1970)),
        (einmal, json!(false)),
    ];
    if let Some(b) = bu {
        p.push((buf, json!(b)));
    }
    if let Some(a) = antwort {
        p.push((ant, json!(a)));
    }
    p
}

async fn ergebnis(paare: Paare) -> Value {
    let d = dienst();
    fall_mit(&d, &paare).await;
    let (status, a) = sende(&d, "GET", "/fall/lj/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {a}");
    a
}

/// Ohne die zweite Person im Gewinn: der Partner mit 0 Euro Gewinn (sonst sperrt "beide Gewinne").
fn partner_ohne_gewinn() -> Paare {
    vec![
        ("rentner_veraeusserungsgewinn_partner", json!(0)),
        ("rentner_veraeusserungs_betriebsart_partner", json!("gewerbe")),
        ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
        ("rentner_freibetrag_erstmalig_partner", json!(true)),
    ]
}

/// A mit 60.000 Euro Lohn und der Gewinn von 500.000 Euro bei EINER der zwei Personen, Zusammenveranlagung, VZ 2025: mit dem
/// ermaessigten Satz 114.946 Euro, mit der Fuenftelregel 191.188 Euro festzusetzende Steuer (Handrechnung in
/// `bescheid/tests/p34_partner_hermetisch.rs`, dort die Tabelle; hier zusammen mit den echten Routen). Im Jahr der Vollendung
/// entscheidet die Antwort: keine Antwort und "ja" rechnen den ermaessigten Satz, "nein" die Fuenftelregel. Mit "nein" und
/// offener Berufsunfaehigkeit sperrt die Software, bis sie beantwortet ist (die Frage kann die Berechtigung herstellen);
/// beantwortet mit Nein ist es die Fuenftelregel, mit Ja der ermaessigte Satz.
#[tokio::test]
async fn ueber_die_routen_entscheidet_die_antwort_im_vollendungsjahr_ueber_den_satz() {
    const ABS3: i64 = 11_494_600;
    const FUENFTEL: i64 = 19_118_800;
    for partner in [false, true] {
        let n = if partner { "Partner" } else { "A" };
        let fall = |antwort: Option<bool>, bu: Option<bool>| {
            let mut e = zusammen_basis();
            if partner {
                e = mit(e, person(true, antwort, bu));
            } else {
                e = mit(e, partner_ohne_gewinn());
                e = mit(e, person(false, antwort, bu));
            }
            e
        };
        for (antwort, bu, soll) in [
            (None, Some(false), ABS3),
            (Some(true), Some(false), ABS3),
            (Some(false), Some(false), FUENFTEL),
            (Some(false), Some(true), ABS3),
        ] {
            let a = ergebnis(fall(antwort, bu)).await;
            assert_eq!(a["grund"], "bestaetigt", "{n} {antwort:?} {bu:?}: {a}");
            assert_eq!(a["zahl_cent"].as_i64(), Some(soll), "{n} Antwort {antwort:?}, berufsunfaehig {bu:?}: {a}");
        }
        // "nein" und die Berufsunfaehigkeit ist offen: die Frage kann die Berechtigung herstellen, also sperrt die Software.
        let offen = ergebnis(fall(Some(false), None)).await;
        let grund = if partner { "berufsunfaehigkeit_partner_offen" } else { "berufsunfaehigkeit_offen" };
        assert_eq!(offen["grund"], grund, "{n}: nein, Berufsunfaehigkeit offen: {offen}");
        assert!(offen["zahl_cent"].is_null(), "{n}: {offen}");
        // "ja" oder keine Antwort und die Berufsunfaehigkeit offen: das Alter berechtigt, die Frage aendert nichts.
        for antwort in [None, Some(true)] {
            let ok = ergebnis(fall(antwort, None)).await;
            assert_eq!(ok["grund"], "bestaetigt", "{n} {antwort:?}, Berufsunfaehigkeit offen: {ok}");
            assert_eq!(ok["zahl_cent"].as_i64(), Some(ABS3), "{n} {antwort:?}: {ok}");
        }
    }
}

/// Basis plus Aenderungen: ein Feld, das die Basis schon traegt, wird ersetzt, ein neues angehaengt.
fn mit(basis: Paare, aenderungen: Paare) -> Paare {
    let mut raus = basis;
    for (feld, wert) in aenderungen {
        match raus.iter_mut().find(|(f, _)| *f == feld) {
            Some(p) => p.1 = wert,
            None => raus.push((feld, wert)),
        }
    }
    raus
}

/// Der volle Pflicht-Kegel der Scheibe `gesamt` (Einzelveranlagung, 60.000 Euro Lohn); `tests/_kegel.py::kegel_fuer`.
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
