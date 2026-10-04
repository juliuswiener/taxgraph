//! Der Ueberlauf an der HTTP-Naht (`rust/api`): wo Python mit beliebig grossen `int` weiterrechnet und Rust in `i64` ueberlaeuft,
//! antwortet `GET /fall/{id}/stand`, `/fragen`, `/ergebnis` und `/deklaration` mit 422 und der Meldung "Ein eingegebener Betrag ist zu
//! gross ..." -- nie mit einer Zahl, nie mit einem 500 (`api/src/stand.rs`, `ueberlauf_422`, `intervall_fehler`, `bescheid_fehler`;
//! Bericht h8-befund-12). Seit h8-befund-12 ist das ein 422: ein Betrag, den `POST /event` annimmt (z. B. `bruttoarbeitslohn` =
//! `i64::MIN`), ist eine Eingabe des Nutzers und kein Programmfehler; vorher antwortete Rust 500 `OverflowError`/`CatalaError`.
//! Standardlauf, ohne Python-Server.
//!
//! Die Mutanten A1/A2/A4 (Bericht h8-hermetisch5, Bestand 0aa91677, `cargo test -p api`, 201 passed / 0 failed; damals mit
//! 500 `OverflowError`; seit h8-befund-12 liegen ihre Stellen hinter dem 422 und sind nur noch ueber `Dezimal`/`NichtCentGenau`
//! erreichbar): A1 `slot_klasse`
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

/// Der Anfang der 422-Meldung bei einem Ueberlauf (`api/src/stand.rs`, `ueberlauf_422`).
const MELDUNG: &str = "Ein eingegebener Betrag ist zu groß für die Berechnung";

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

/// A1, A2, A4 (Konvention, kein Orakel): ein Ueberlauf an der Naht ist ein 422 mit der Meldung "Ein eingegebener Betrag ist zu
/// gross ...", nie ein 500 und nie eine Zahl (h8-befund-12; vorher 500 `OverflowError`).
/// Python (`orakel_ap5.py`, Bereichspruefung des Stores aus, `api.stand`/`api.ergebnis` im selben Prozess) antwortet in ALLEN
/// Faellen unten mit 200 und einer Zahl (`bestaetigt`, `zahl_cent` 0, ausser der Kontrolle: 661100); der 500 ist die fail-closed-
/// Konvention von Rust (`stand.rs`: "GEWOLLTE ABWEICHUNG ... Python rechnet mit beliebig grossen `int` weiter"), also "zwischen",
/// und der Text der Meldung ist Rust-eigen. Der Status und der Meldungsanfang sind die Messgroesse:
/// - A2 `slot_klasse(SlotFehler::Ueberlauf)`: `basis_kv` `i64::MAX` plus `basis_pv` 1 (derselbe Slot `basis_kv_pv`), Route `stand`;
/// - A4 `bescheid_fehler`: dieselben Felder, Route `ergebnis` (`feste_zahl` meldet `Ueberlauf("feste_zahl")`);
/// - A1 `slot_klasse(SlotFehler::Slot(Ueberlauf))`: `tage_24h` `i64::MAX`, die Addition im Ring ueberlaeuft, Route `stand`.
///
/// Die Gegenproben (`basis_pv` 0, keine Aenderung) liefern 200 und die Python-Zahl.
#[tokio::test]
async fn ein_ueberlauf_an_der_naht_ist_ein_422_mit_klarer_meldung_und_nie_ein_500() {
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
    // (Fall, Route): 422 und der Anfang der Meldung
    let ueberlaeufe = [
        ("slot", "stand"),
        ("slot", "ergebnis"),
        ("ring", "stand"),
        ("ring", "ergebnis"),
    ];
    for (id, r) in ueberlaeufe {
        let (s, a) = route(&d, id, r).await;
        let fehler = a["fehler"].as_str().unwrap_or("");
        if s != 422 || !fehler.starts_with(MELDUNG) {
            falsch.push(format!("{id} {r}: {s} {a}, erwartet 422 mit {MELDUNG:?}"));
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
/// meldet denselben Ring-Ueberlauf als 422 mit der Betragsmeldung (vorher `OverflowError`) -- sonst wuerde der Test nichts messen.
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
        let (status, erwartet) = if ok {
            (422, MELDUNG.to_owned())
        } else {
            (
                500,
                format!(
                    "ValueError: Veranlagungsjahr {vz} ist kein Steuerjahr (erwartet 2024..2100). "
                ),
            )
        };
        if s != status || !fehler.starts_with(&erwartet) {
            falsch.push(format!(
                "Jahr {vz}: {s} {a}, erwartet {status} mit {erwartet:?}"
            ));
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
/// Das Jahr 2023 (passt in `u16`, ist kein `Vz`) steht seit h8-ueberlauf-luecke mit in der Liste: nur dieser Fall macht
/// `Vz::try_from(j)` an der Stelle von `fragen` sichtbar (Mutant V2), denn `jahr()` lehnt das Jahr dort sonst ohnehin ab.
#[tokio::test]
async fn ein_jahr_ausserhalb_u16_mit_kaputtem_guard_feld_meldet_den_jahresfehler_konvention() {
    let d = dienst();
    let kaputt: [(&'static str, Value); 3] = [
        ("antrag_ermaessigter_satz", json!(true)),
        ("rentner_veraeusserungsgewinn", json!("abc")),
        ("geburtsjahr", json!(1960)),
    ];
    let mut falsch = Vec::new();
    for vz in ["2025", "67561", "2023"] {
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
                "ValueError: Python ValueError: int(kein Zahlwert)".to_owned()
            } else {
                format!("ValueError: kein unterstuetzter Veranlagungszeitraum: {vz}")
            };
            if s != 500 || !fehler.starts_with(&erwartet) {
                falsch.push(format!(
                    "Jahr {vz} {r}: {s} {a}, erwartet 500 mit {erwartet:?}"
                ));
            }
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Legt Fall `id` (`an_gesamt`, 2025) an und schreibt den vollen Kegel ueber `POST /fall/{id}/event`, wie die Oberflaeche es tut
/// -- mit der Bereichspruefung des Stores, ohne den Umweg ueber die Akte. `Err` nennt das erste Feld, das der Store abweist.
async fn fall_ueber_event(
    d: &Dienst,
    id: &str,
    aendern: &[(&'static str, Value)],
) -> Result<(), String> {
    let rumpf = json!({"fall_id": id, "scheibe": "an_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&rumpf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let mut felder = kegel_an_voll();
    for (f, w) in aendern {
        felder.retain(|(g, _)| g != f);
        felder.push((f, w.clone()));
    }
    for (feld, wert) in felder {
        let event = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        });
        let (s, a) = sende(d, "POST", &format!("/fall/{id}/event"), Some(&event)).await;
        if s != 201 {
            return Err(format!("{feld}: {s} {a}"));
        }
    }
    Ok(())
}

/// h8-befund-12: ein Betrag, den `POST /event` annimmt und den die Rechnung nicht in `i64` fasst, ist ein 422 mit der Betragsmeldung --
/// auf `stand`, `fragen` und `ergebnis`, nie ein 500 (vorher `CatalaError` oder `OverflowError`, je nach Accessor). Gemessen mit
/// echtem Python-Server und echtem Rust-Server (beide `TAXGRAPH_NO_AUTH=1`, Skripte `mess12b.py`, `mess12c.py`, 22 Zahlfelder je
/// `i64::MAX`, `i64::MIN`, ±10^17): Python antwortet in jedem dieser Faelle auf allen Routen mit 200 und rechnet weiter (`ergebnis`
/// `bestaetigt`, `zahl_cent` 0 bzw. 661100); der 422 ist Rusts gewollte Abweichung ("zwischen": nur das Zwischenprodukt Euro in Cent
/// liegt ausserhalb `i64`). Elf Eingaben erreichen die Stelle: `bruttoarbeitslohn` `i64::MIN`, `ep_oepnv_kosten` `i64::MAX`,
/// `ep_entfernung_km` `i64::MAX` und 10^17 (alle drei Routen, Jahresbetrag der Entfernungspauschale `ep_gesamt`; vor h8-ep-fenster nur
/// `ergebnis`, und `stand`/`fragen` rechneten still mit dem Wert mod 2^63), und sieben Vorsorgebetraege `i64::MIN`.
/// Alle anderen Zahlfelder weist `POST /event` selbst ab (Bereich, Vorzeichen) oder die Rechnung kommt durch.
///
/// Gegenproben ("knapp", Python = Rust): `bruttoarbeitslohn` `i64::MAX` (`zahl_cent` 4150517416582623200), 10^17 (44999999997974100)
/// und -10^17 (0), `basis_kv` `i64::MAX` (0) und -10^17 (661100) rechnen auf allen drei Routen mit 200. Der Store weist
/// `ep_arbeitstage` `i64::MAX` ab (422, Bereich 0..=366): der `CatalaError` der Akte von Hand ist ueber HTTP nicht erreichbar.
#[tokio::test]
async fn ein_betrag_ausserhalb_i64_ueber_post_event_ist_ein_422_und_nie_ein_500() {
    let d = dienst();
    let (hoch, tief) = (i64::MAX, i64::MIN);
    let vorsorge = [
        "basis_kv",
        "basis_pv",
        "vorsorge_arbeitslosenversicherung",
        "vorsorge_erwerbsunfaehigkeit",
        "vorsorge_unfall_haftpflicht",
        "vorsorge_rv_alt_mit_ueberschuss",
        "vorsorge_rv_alt_ohne_ueberschuss",
    ];
    // (Feld, Wert, Routen mit 422); die uebrigen der drei Routen antworten 200 wie Python
    let mut ueberlaeufe: Vec<(&'static str, i64, &[&str])> = vec![
        ("bruttoarbeitslohn", tief, &["stand", "fragen", "ergebnis"]),
        ("ep_oepnv_kosten", hoch, &["stand", "fragen", "ergebnis"]),
        ("ep_entfernung_km", hoch, &["stand", "fragen", "ergebnis"]),
        (
            "ep_entfernung_km",
            10_i64.pow(17),
            &["stand", "fragen", "ergebnis"],
        ),
    ];
    ueberlaeufe.extend(
        vorsorge
            .iter()
            .map(|f| (*f, tief, &["stand", "fragen", "ergebnis"][..])),
    );
    let mut falsch = Vec::new();
    for (i, (feld, wert, routen)) in ueberlaeufe.iter().enumerate() {
        let id = format!("u{i}");
        if let Err(e) = fall_ueber_event(&d, &id, &[(feld, json!(wert))]).await {
            falsch.push(format!("{feld}={wert}: POST /event abgewiesen: {e}"));
            continue;
        }
        for r in ["stand", "fragen", "ergebnis"] {
            let (status, antwort) = route(&d, &id, r).await;
            let fehler = antwort["fehler"].as_str().unwrap_or("");
            let ok = if routen.contains(&r) {
                status == 422
                    && fehler.starts_with(MELDUNG)
                    && !fehler.contains(&wert.unsigned_abs().to_string())
            } else {
                status == 200
            };
            if !ok {
                falsch.push(format!("{feld}={wert} {r}: {status} {antwort}"));
            }
        }
    }
    // Gegenproben: Python = Rust, 200 auf allen drei Routen; `ergebnis` traegt die Python-Zahl.
    let knapp: [(&'static str, i64, i64); 5] = [
        ("bruttoarbeitslohn", hoch, 4_150_517_416_582_623_200),
        ("bruttoarbeitslohn", 10_i64.pow(17), 44_999_999_997_974_100),
        ("bruttoarbeitslohn", -(10_i64.pow(17)), 0),
        ("basis_kv", hoch, 0),
        ("basis_kv", -(10_i64.pow(17)), 661_100),
    ];
    for (i, (feld, wert, zahl)) in knapp.iter().enumerate() {
        let id = format!("k{i}");
        if let Err(e) = fall_ueber_event(&d, &id, &[(feld, json!(wert))]).await {
            falsch.push(format!("{feld}={wert}: POST /event abgewiesen: {e}"));
            continue;
        }
        for r in ["stand", "fragen", "ergebnis"] {
            let (status, antwort) = route(&d, &id, r).await;
            let ok = status == 200
                && (r != "ergebnis"
                    || (antwort["grund"] == "bestaetigt" && antwort["zahl_cent"] == *zahl));
            if !ok {
                falsch.push(format!("Gegenprobe {feld}={wert} {r}: {status} {antwort}"));
            }
        }
    }
    // Der Store weist die Felder ab, deren Ueberlauf nur ueber eine Akte von Hand erreichbar ist.
    for (feld, wert) in [
        ("ep_arbeitstage", hoch),
        ("tage_24h", hoch),
        ("ep_oepnv_kosten", tief),
    ] {
        match fall_ueber_event(&d, &format!("a{feld}"), &[(feld, json!(wert))]).await {
            Err(e) if e.starts_with(&format!("{feld}: 422 ")) => {}
            r => falsch.push(format!(
                "{feld}={wert}: 422 vom Store erwartet, gekommen {r:?}"
            )),
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// V1-V6 (Bericht h8-ueberlauf-luecke): ein Jahr, das in `u16` PASST, aber kein `Vz` ist (2023; die Aufzaehlung kennt 2024..=2026).
/// Die Kette `u16::try_from(jahr)` -> `Vz::try_from(j)` an den Routen `stand` (zwei Stellen), `fragen`, `ergebnis` und `deklaration`
/// laesst nur das zweite Glied sichtbar werden, wenn das Jahr schon in `u16` liegt: ein Mutant, der `Vz::try_from(j).ok()` durch
/// `Some(Vz::Vz2025)` ersetzt, rechnet mit Jahr 2025 weiter. Alle fuenf lieferten unter `cargo test -p api` gruen (Mutanten V1-V5).
/// Sichtbar wird das ueber dieselben Faelle wie A8-A11: Abs.-3-Sperre (Jahr 2025: 200 mit Sperre; 2023: 500) und Ring-Ueberlauf der
/// `deklaration` (Jahr 2025: 422 mit der Betragsmeldung; 2023: 500).
///
/// Rust: 500 `ValueError: kein unterstuetzter Veranlagungszeitraum: 2023` auf `stand`, `fragen` und `ergebnis`; `deklaration` meldet
/// Pythons Text. Python (`orakel_luecke_vz.py`, 2023, Faelle wie `orakel_a8.py`/`orakel_a11.py`): `deklaration` -> `ValueError`
/// "Veranlagungsjahr 2023 ist kein Steuerjahr (erwartet 2024..2100). ..." (Rust gleich); `stand` und `ergebnis` antworten 200 mit der
/// Sperre (Python rechnet mit dem Jahr 2023 weiter: "Konvention, kein Orakel", Rust ist strenger, siehe `stand::jahr`), `fragen`
/// antwortet 500 (`FileNotFoundError`, `params/2023` fehlt) -- dort stuetzt Python den Status.
#[tokio::test]
async fn ein_jahr_das_in_u16_passt_aber_kein_steuerjahr_ist_wird_nie_auf_2025_gesetzt() {
    let d = dienst();
    let abs3: [(&'static str, Value); 3] = [
        ("antrag_ermaessigter_satz", json!(true)),
        ("rentner_veraeusserungsgewinn", json!(600_000_000)),
        ("geburtsjahr", json!(1960)),
    ];
    let mut falsch = Vec::new();
    // stand, fragen, ergebnis (V1-V4)
    for vz in ["2025", "2023"] {
        let id = format!("s{vz}");
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
                        .starts_with("ValueError: kein unterstuetzter Veranlagungszeitraum: 2023")
            };
            if !ok {
                falsch.push(format!("Jahr {vz} {r}: {s} {a}"));
            }
        }
    }
    // deklaration (V6, api/src/deklaration.rs:46): Ring-Ueberlauf, der Parameter des Jahres braucht
    for (vz, ok) in [("2023", false), ("2025", true)] {
        let id = format!("d{vz}");
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
        let (status, erwartet) = if ok {
            (422, MELDUNG.to_owned())
        } else {
            (
                500,
                format!(
                    "ValueError: Veranlagungsjahr {vz} ist kein Steuerjahr (erwartet 2024..2100). "
                ),
            )
        };
        if s != status || !fehler.starts_with(&erwartet) {
            falsch.push(format!(
                "deklaration, Jahr {vz}: {s} {a}, erwartet {status} mit {erwartet:?}"
            ));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// (Fall, Felder, Route, erwarteter Status, Marke im Meldungstext)
type HttpFall = (
    &'static str,
    Vec<(&'static str, Value)>,
    &'static str,
    u16,
    &'static str,
);

/// Zwei Ueberlaufstellen der Engine, die ueber die HTTP-Haut mit gueltigen Feldern erreichbar sind (Bericht h8-ueberlauf-luecke,
/// Sonde `sonde_luecke`): Mahlzeiten-Anzahlen (`int` ohne Bereich: 16470307208669242 Fruehstuecke zu 560 ct und ein Mittagessen zu
/// 1120 ct laufen als Summe ueber, Marke `vpf k28`) und eine Entfernung 1e15 km bei 366 Arbeitstagen (`Tage * (km - 20) * 38 ct`,
/// Marke `ab21_roh`; nur `ergebnis` rechnet den erhoehten Teil ab dem 21. km). Mit Kfz kommt der Jahresbetrag der Entfernungspauschale
/// (Marke `ep_gesamt`, h8-ep-fenster) schon vorher aus dem Bereich: `stand`, `fragen` und `ergebnis` 422; ohne Kfz deckelt der Scope
/// auf 4500 EUR, `stand` und `fragen` bleiben 200 und nur `ergebnis` meldet `ab21_roh`. `deklaration` rechnet die Pauschale nicht.
/// Rust meldet 422 mit der Betragsmeldung, nie 500 und nie eine Zahl. Python (`orakel_luecke_http.py`, Bereichspruefung des Stores
/// aus) antwortet in beiden Faellen auf allen vier Routen mit 200 (`ergebnis`: 662900 bzw. 0 Cent): der 422 ist die fail-closed-
/// Konvention von Rust, "zwischen". Gegenproben (ein Fruehstueck allein, 30 km): 200 wie in Python; der Test prueft dort
/// nur den Status.
#[tokio::test]
async fn mahlzeiten_und_entfernung_ausserhalb_i64_sind_ueber_http_ein_422_und_nie_ein_500() {
    let d = dienst();
    let frueh = json!(16_470_307_208_669_242_i64);
    let mahlzeit = |extra: Vec<(&'static str, Value)>| {
        let mut f = vec![("vpf_keine_mahlzeitengestellung", json!(false))];
        f.extend(extra);
        f
    };
    let faelle: Vec<HttpFall> = vec![
        (
            "m1",
            mahlzeit(vec![
                ("vpf_fruehstuecke_gestellt_anzahl", frueh.clone()),
                ("vpf_mittagessen_gestellt_anzahl", json!(1)),
            ]),
            "stand",
            422,
            "vpf k28",
        ),
        ("m1", Vec::new(), "fragen", 422, "vpf k28"),
        ("m1", Vec::new(), "ergebnis", 422, "vpf k28"),
        (
            "m2",
            mahlzeit(vec![("vpf_fruehstuecke_gestellt_anzahl", frueh)]),
            "stand",
            200,
            "",
        ),
        (
            "k1",
            vec![
                ("ep_entfernung_km", json!(1_000_000_000_000_000_i64)),
                ("ep_arbeitstage", json!(366)),
            ],
            "ergebnis",
            422,
            "ep_gesamt",
        ),
        ("k1", Vec::new(), "stand", 422, "ep_gesamt"),
        ("k1", Vec::new(), "fragen", 422, "ep_gesamt"),
        (
            "k3",
            vec![
                ("ep_entfernung_km", json!(1_000_000_000_000_000_i64)),
                ("ep_arbeitstage", json!(366)),
                ("ep_eigenes_kfz", json!(false)),
            ],
            "ergebnis",
            422,
            "ab21_roh",
        ),
        ("k3", Vec::new(), "stand", 200, ""),
        ("k3", Vec::new(), "fragen", 200, ""),
        (
            "k2",
            vec![
                ("ep_entfernung_km", json!(30)),
                ("ep_arbeitstage", json!(220)),
            ],
            "ergebnis",
            200,
            "",
        ),
    ];
    let mut falsch = Vec::new();
    let mut angelegt = Vec::new();
    for (id, felder, r, status, marke) in faelle {
        if !angelegt.contains(&id) {
            fall_geaendert(&d, id, &felder).await;
            angelegt.push(id);
        }
        let (s, a) = route(&d, id, r).await;
        let fehler = a["fehler"].as_str().unwrap_or("");
        let ok = s == status
            && (status != 422 || (fehler.starts_with(MELDUNG) && fehler.contains(marke)));
        if !ok {
            falsch.push(format!(
                "{id} {r}: {s} {a}, erwartet {status} mit {marke:?}"
            ));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}
