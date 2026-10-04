//! Die Fehlerklasse eines Ueberlaufs an der HTTP-Naht (`rust/api`): wo Python mit beliebig grossen `int` weiterrechnet und Rust
//! in `i64` ueberlaeuft, antwortet `GET /fall/{id}/stand` und `/ergebnis` mit 500 und der Klasse `OverflowError`, nie mit
//! `ValueError` und nie mit einer Zahl (`api/src/stand.rs`, `slot_klasse`, `intervall_fehler`, `bescheid_fehler`). Standardlauf,
//! ohne Python-Server.
//!
//! Die Mutanten (Bericht h8-hermetisch5, Bestand 0aa91677, `cargo test -p api`, 201 passed / 0 failed): A1 `slot_klasse`
//! (`SlotFehler::Slot(b)`: `unwrap_or("OverflowError")` -> `"ValueError"`), A2 `slot_klasse` (`SlotFehler::Ueberlauf` ->
//! `"ValueError"`), A4 `bescheid_fehler` (`unwrap_or("OverflowError")` -> `"ValueError"`), A8/A9/A10 Guard-Jahr von `stand`/`fragen`/
//! `ergebnis` und A11 Ring-Jahr von `deklaration` (`try_from(..).ok()` -> `as u16`); alle sieben ueberleben den Bestand und werden
//! mit diesen Tests rot. A3 (`intervall_fehler`, `IntervallFehler::Ueberlauf`) bleibt: in den 44 Einzelfeld-Faellen (22 Zahlfelder
//! des `an_gesamt`-Kegels, je auf `i64::MAX` und `i64::MIN`) kommt die Spanne `max - min` nie ueber `i64` (Bericht: "Ueberlebender,
//! ungeklaert"). A5 (`kontoauszug`, `BetragUeberlauf` -> `ValueError`) ist ueber HTTP unerreichbar: `verwirf_unlesbare_betraege(_json)`
//! wirft jeden Betrag ueber 9.999.999.999 Cent vor `aus_json` und `uebernehme` raus (`kontoauszug_naht.rs`: `i64::MIN`, 1e10,
//! -12345678901234567890 -> 200, `verworfen` 1).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Python (`orakel_ap5.py`, `api.stand`/`api.ergebnis` im selben Prozess, Bereichspruefung des Stores
//! aus) antwortet in jedem Ueberlauf-Fall mit 200; der 500 `OverflowError` ist die fail-closed-Konvention von Rust
//! (`api/src/stand.rs`), der Text hinter der Klasse Rust-eigen. Ausnahme A11: `orakel_a11.py` (Python, `api.deklaration`) lehnt
//! ein Jahr ausserhalb u16 mit `ValueError: Veranlagungsjahr <vz> ist kein Steuerjahr ...` ab, auch mit dem Ring-Fall.
//! Ausnahme A8-A10 (`orakel_a8.py`): Python antwortet auf `stand`/`ergebnis` mit der Sperre (200), auf `fragen` mit 500; der 500
//! `ValueError` von Rust ist dort Konvention.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::path::Path;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEvent, Store};
use tower::ServiceExt;

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

/// Der Pflicht-Kegel von `an_gesamt` (33 Felder) samt der Angaben, die den Guard zufriedenstellen,
/// mit neutralen Werten (`api_http_paritaet::kegel_an_voll`).
fn kegel_an_voll() -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(4_000_000)),
        ("veranlagung", json!("einzel")),
        ("ep_entfernung_km", json!(30)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(true)),
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
        ("dhf_unterkunftskosten_monat", json!(0)),
        ("dhf_im_inland", json!(false)),
        ("dhf_beruflich_veranlasst", json!(false)),
        ("dhf_eigener_hausstand", json!(false)),
        ("dhf_finanzielle_beteiligung", json!(false)),
        ("tage_24h", json!(1)),
        ("tage_an_abreise", json!(1)),
        ("tage_ueber_8h_eintaegig", json!(1)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("verlustvortrag_bestand", json!(0)),
        ("ep_arbeitstage", json!(220)),
        ("fam_anzahl_kinder", json!(0)),
        ("dhf_monate", json!(0)),
        ("vpf_monate_am_ort", json!(2)),
        ("vpf_keine_mahlzeitengestellung", json!(true)),
    ]
}

/// Legt Fall `id` der Scheibe `an_gesamt` an und hängt je Feld ein bestätigtes Event an die Akte
/// (so wie `POST /event` es täte, aber ohne dessen Prüfung gegen die Scheibe).
async fn fall_mit(d: &Dienst, id: &str, felder: &[(&str, Value)]) {
    let rumpf = json!({"fall_id": id, "scheibe": "an_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
    let mut store = Store::aus_datei(store::lade(&pfad).unwrap());
    let leer = HashMap::new();
    for (feld, wert) in felder {
        let neu = NeuesEvent {
            feld_id: (*feld).to_owned(),
            wert: wert.clone().into(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick@naht").unwrap(),
            },
            herkunft: Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            },
            schreiber: Schreiber::Mensch("ui:naht".to_owned()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-01-01T00:00:00+00:00".to_owned()),
        };
        store
            .append(&neu, None, BindungNachschlag::neu(&leer))
            .unwrap();
    }
    store::speichere(&pfad, store.datei()).unwrap();
}

/// Fall `id` der Scheibe `an_gesamt`: der volle Kegel, davon `aendern` ersetzt.
async fn fall_geaendert(d: &Dienst, id: &str, aendern: &[(&'static str, Value)]) {
    let mut felder = kegel_an_voll();
    for (f, w) in aendern {
        felder.retain(|(g, _)| g != f);
        felder.push((f, w.clone()));
    }
    fall_mit(d, id, &felder).await;
}

/// Status und `fehler`-Text einer Route.
async fn route(d: &Dienst, id: &str, route: &str) -> (u16, Value) {
    sende(d, "GET", &format!("/fall/{id}/{route}"), None).await
}

/// A1, A2, A4 (Konvention, kein Orakel): ein Ueberlauf an der Naht ist ein 500 mit der Klasse `OverflowError`, nie `ValueError`.
/// Python (`orakel_ap5.py`, Bereichspruefung des Stores aus, `api.stand`/`api.ergebnis` im selben Prozess) antwortet in ALLEN
/// Faellen unten mit 200 und einer Zahl (`bestaetigt`, `zahl_cent` 0, ausser der Kontrolle: 661100); der 500 ist die fail-closed-
/// Konvention von Rust (`stand.rs`: "wo Python mit beliebig grossen `int` weiterrechnet und Rust ueberlaeuft"), also "zwischen",
/// und der Text hinter der Klasse ist Rust-eigen. Die Klasse ist die Messgroesse:
/// - A2 `slot_klasse(SlotFehler::Ueberlauf)`: `basis_kv` `i64::MAX` plus `basis_pv` 1 (derselbe Slot `basis_kv_pv`), Route `stand`;
/// - A4 `bescheid_fehler`: dieselben Felder, Route `ergebnis` (`feste_zahl` meldet `Ueberlauf("feste_zahl")`);
/// - A1 `slot_klasse(SlotFehler::Slot(Ueberlauf))`: `tage_24h` `i64::MAX`, die Addition im Ring ueberlaeuft, Route `stand`.
///
/// Die Gegenproben (`basis_pv` 0, keine Aenderung) liefern 200 und die Python-Zahl.
#[tokio::test]
async fn ein_ueberlauf_an_der_naht_ist_ein_overflow_error_und_nie_ein_value_error() {
    let d = dienst();
    let m = i64::MAX;
    fall_geaendert(&d, "ok", &[]).await;
    fall_geaendert(
        &d,
        "slot_knapp",
        &[("basis_kv", json!(m)), ("basis_pv", json!(0))],
    )
    .await;
    fall_geaendert(
        &d,
        "slot",
        &[("basis_kv", json!(m)), ("basis_pv", json!(1))],
    )
    .await;
    fall_geaendert(&d, "ring", &[("tage_24h", json!(m))]).await;
    let mut falsch = Vec::new();
    // Gegenproben: Python 200; `zahl_cent` der Kontrolle 661100 und der Randfaelle 0.
    for (id, zahl) in [("ok", 661_100), ("slot_knapp", 0)] {
        let (s, a) = route(&d, id, "ergebnis").await;
        if (s, &a["grund"], &a["zahl_cent"]) != (200, &json!("bestaetigt"), &json!(zahl)) {
            falsch.push(format!("Gegenprobe {id} ergebnis: {s} {a}"));
        }
        let (s, a) = route(&d, id, "stand").await;
        if s != 200 {
            falsch.push(format!("Gegenprobe {id} stand: {s} {a}"));
        }
    }
    // (Fall, Route, Klasse und Text-Anfang des `fehler`-Felds)
    let ueberlaeufe = [
        ("slot", "stand", "OverflowError: "),
        ("slot", "ergebnis", "OverflowError: "),
        ("ring", "stand", "OverflowError: "),
        ("ring", "ergebnis", "OverflowError: "),
    ];
    for (id, r, anfang) in ueberlaeufe {
        let (s, a) = route(&d, id, r).await;
        let fehler = a["fehler"].as_str().unwrap_or("");
        if s != 500 || !fehler.starts_with(anfang) {
            falsch.push(format!("{id} {r}: {s} {a}, erwartet 500 mit {anfang:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// A11 (Python-gestuetzt): ein Jahr ausserhalb `u16` UND ein Ring-Ueberlauf, der Parameter des Jahres braucht (24-h-Verpflegungstage
/// `i64::MAX`, ausserhalb des Bindungsbereichs 1..=366, also von Hand in die Akte; die Kuerzung rechnet `mit_ring_werten` nur mit
/// `params/<Jahr>`). Ohne Parameter fuer das Jahr rechnet der Ring nicht (`vz: None`), `deklariere` lehnt das Jahr ab: dieselbe
/// `ValueError` wie ohne den Ring-Fall, nie der `OverflowError` des Rings, den ein auf 2025 gewickeltes Jahr ausloeste
/// (`orakel_a11.py`: Python, `api.deklaration`, Bereichspruefung des Stores aus, Akte mit Jahr 67561 und -63511 und demselben Event,
/// antwortet mit dem `ValueError` "Veranlagungsjahr <vz> ist kein Steuerjahr ..."). Die Akte wird als `an_gesamt` mit Jahr 2025
/// angelegt und danach von Hand auf Scheibe `ep` (kein Guard) und das Jahr umgeschrieben, wie im Orakel. Gegenprobe: Jahr 2025
/// meldet denselben Ring-Ueberlauf als `OverflowError` -- sonst wuerde der Test nichts messen.
#[tokio::test]
async fn deklaration_mit_ring_ueberlauf_lehnt_ein_jahr_ausserhalb_u16_zuerst_ab() {
    let d = dienst();
    let mut falsch = Vec::new();
    for (vz, ok) in [("67561", false), ("-63511", false), ("2025", true)] {
        let id = format!("r{vz}");
        fall_geaendert(&d, &id, &[("tage_24h", json!(i64::MAX))]).await;
        let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
        let text = std::fs::read_to_string(&pfad).unwrap();
        for anker in ["\"scheibe\":\"an_gesamt\"", "\"veranlagungszeitraum\":2025"] {
            assert_eq!(text.matches(anker).count(), 1, "Anker {anker}: {text}");
        }
        std::fs::write(
            &pfad,
            text.replace("\"scheibe\":\"an_gesamt\"", "\"scheibe\":\"ep\"")
                .replace(
                    "\"veranlagungszeitraum\":2025",
                    &format!("\"veranlagungszeitraum\":{vz}"),
                ),
        )
        .unwrap();
        let (s, a) = route(&d, &id, "deklaration").await;
        let fehler = a["fehler"].as_str().unwrap_or("");
        let erwartet = if ok {
            "OverflowError: ".to_owned()
        } else {
            format!("ValueError: Veranlagungsjahr {vz} ist kein Steuerjahr (erwartet 2024..2100). ")
        };
        if s != 500 || !fehler.starts_with(&erwartet) {
            falsch.push(format!("Jahr {vz}: {s} {a}, erwartet 500 mit {erwartet:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// A8, A9, A10: das Guard-Jahr von `stand`, `fragen` und `ergebnis` wird nie auf ein anderes Jahr gewickelt. Sichtbar wird es nur
/// ueber eine jahresabhaengige Sperre: Abs. 3 (`antrag_ermaessigter_satz`, `rentner_veraeusserungsgewinn` 6 Mio EUR, `geburtsjahr`
/// 1960: mit Jahr 2025 berechtigt, `abs3_ueber_5mio_offen`). Akte von Hand (Scheibe `an_gesamt`, Jahr 67561 = 2025 + 65536).
/// Rust: 500 `ValueError` auf allen drei Routen ("kein unterstuetzter Veranlagungszeitraum", `stand::jahr`); ein gewickeltes Jahr
/// lieferte stattdessen die Sperre fuer 2025. Python (`orakel_a8.py`): `stand` und `ergebnis` antworten 200 mit der Sperre (Python
/// rechnet mit dem Jahr 67561 weiter, "Konvention, kein Orakel": der 500 ist Rusts strengere Regel, siehe `stand::jahr`),
/// `fragen` antwortet 500 (`FileNotFoundError`, `params/67561` fehlt) -- dort stuetzt Python den Status. Gegenprobe: Jahr 2025 liefert
/// auf allen drei Routen die Sperre (Python: dieselbe).
#[tokio::test]
async fn ein_jahr_ausserhalb_u16_mit_jahresabhaengiger_sperre_bleibt_ein_500_konvention() {
    let d = dienst();
    let abs3: [(&'static str, Value); 3] = [
        ("antrag_ermaessigter_satz", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(600_000_000)),
        ("geburtsjahr", json!(1960)),
    ];
    let mut falsch = Vec::new();
    for vz in ["2025", "67561"] {
        let id = format!("a{vz}");
        fall_geaendert(&d, &id, &abs3).await;
        let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
        let text = std::fs::read_to_string(&pfad).unwrap();
        let anker = "\"veranlagungszeitraum\":2025";
        assert_eq!(text.matches(anker).count(), 1, "Anker {anker}: {text}");
        std::fs::write(
            &pfad,
            text.replace(anker, &format!("\"veranlagungszeitraum\":{vz}")),
        )
        .unwrap();
        for r in ["stand", "fragen", "ergebnis"] {
            let (s, a) = route(&d, &id, r).await;
            let gesperrt = a["grund"] == "abs3_ueber_5mio_offen"
                || a["ring_gesperrt"] == "abs3_ueber_5mio_offen";
            let fehler = a["fehler"].as_str().unwrap_or("");
            let ok = if vz == "2025" {
                s == 200 && gesperrt
            } else {
                s == 500
                    && fehler
                        .starts_with("ValueError: kein unterstuetzter Veranlagungszeitraum: 67561")
            };
            if !ok {
                falsch.push(format!("Jahr {vz} {r}: {s} {a}"));
            }
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// A9 (und A8, A10 ein zweites Mal): dasselbe Jahr 67561, aber die Sperre fuehrt nicht zu einer Antwort, sondern der Guard selbst
/// scheitert, wenn er mit einem Jahr rechnen darf: `rentner_veraeusserungsgewinn` ist Text ("abc"), Abs. 3 liest ihn nur mit einem
/// Jahr. `fragen` wirft die Sperre sonst weg (die Fehlermeldung von `jahr()` folgt), der Guard-Fehler waere das Einzige, das ein
/// gewickeltes Jahr hier sichtbar macht. Python (`orakel_a8.py`, Fall `gewinn_text_*`): `ValueError: invalid literal for int() with
/// base 10: 'abc'` auf allen drei Routen, auch mit Jahr 67561 -- Rust meldet dort stattdessen den Jahresfehler ("Konvention, kein
/// Orakel": strenger, das Jahr wird zuerst abgelehnt). Gegenprobe: Jahr 2025 meldet den Guard-Fehler mit Pythons Klasse
/// `ValueError` (der Text ist Rust-eigen, `int(kein Zahlwert)`, nicht `invalid literal ...`).
#[tokio::test]
async fn ein_jahr_ausserhalb_u16_mit_kaputtem_guard_feld_meldet_den_jahresfehler_konvention() {
    let d = dienst();
    let kaputt: [(&'static str, Value); 3] = [
        ("antrag_ermaessigter_satz", json!(true)),
        ("rentner_veraeusserungsgewinn", json!("abc")),
        ("geburtsjahr", json!(1960)),
    ];
    let mut falsch = Vec::new();
    for vz in ["2025", "67561"] {
        let id = format!("k{vz}");
        fall_geaendert(&d, &id, &kaputt).await;
        let pfad = d.zustand.konfig.faelle.join(format!("{id}.json"));
        let text = std::fs::read_to_string(&pfad).unwrap();
        let anker = "\"veranlagungszeitraum\":2025";
        assert_eq!(text.matches(anker).count(), 1, "Anker {anker}: {text}");
        std::fs::write(
            &pfad,
            text.replace(anker, &format!("\"veranlagungszeitraum\":{vz}")),
        )
        .unwrap();
        for r in ["stand", "fragen", "ergebnis"] {
            let (s, a) = route(&d, &id, r).await;
            let fehler = a["fehler"].as_str().unwrap_or("");
            let erwartet = if vz == "2025" {
                "ValueError: Python ValueError: int(kein Zahlwert)"
            } else {
                "ValueError: kein unterstuetzter Veranlagungszeitraum: 67561"
            };
            if s != 500 || !fehler.starts_with(erwartet) {
                falsch.push(format!(
                    "Jahr {vz} {r}: {s} {a}, erwartet 500 mit {erwartet:?}"
                ));
            }
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}
