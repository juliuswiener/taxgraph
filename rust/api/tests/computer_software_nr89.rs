//! Computer und Software: die Frage „Das Geraet steht auf der Liste“ und ihre Folgen fuer die Fragen (Abweichung Nr. 89 in
//! `rust/fixtures/README.md`). Ohne `PARITY=1`, ohne Python, ohne Netz. Python kennt das Feld nicht.
//!
//! **Worum es geht.** Das BMF-Schreiben vom 22.02.2022 (`BStBl` I S. 187) legt fuer Computerhardware und Software eine Nutzungsdauer von
//! einem Jahr fest (Rz. 1). Das ist eine normale Abschreibung nach § 7 Abs. 1 `EStG`, kein `GWG` (Rz. 1.1); der volle Abzug im
//! Anschaffungsjahr wird nicht beanstandet (Rz. 1.4). Die Software fragt dafuer „steht das Geraet auf der Liste im Schreiben“ (Rz. 2 bis 4)
//! und rechnet bei „ja“ mit einem Jahr, ohne Umsatzsteuersatz, ohne Wahl des Sofortabzugs und ohne Zwoelftel nach dem Kaufmonat.
//!
//! **Warum es zaehlt.** Ohne die Frage setzt ein Notebook zu 1.500 EUR nach der App nur einen Teil im Kaufjahr ab (die Hilfe nannte „3 Jahre
//! fuer Laptop“). Mit „ja“ ist der Abzug voll; ein falsches „ja“ fuer ein Geraet ausserhalb der Liste (zum Beispiel ein Smartphone, das
//! das Schreiben nicht nennt) setzt zu viel ab. Darum nennt die Frage die Liste.
//!
//! **Wo es sitzt.** `rust/bindung/daten/bindung_n_vor_gwg.yaml` (`am_computer_software`; `feld_bedingung` mit `groesser_als` 80.000 auf
//! `am_anschaffungskosten`; `am_umsatzsteuersatz` traegt dazu eine `und`-Bedingung), die Rechnung in `rust/bescheid/src/zweige/wk.rs`, die
//! Sperre in `rust/bescheid/src/deklaration/sperre/werbungskosten.rs`. Die Rechnung halten `gesamt_mischfaelle_hermetisch.rs` und
//! `kette_endstand_hermetisch.rs`, die Sperre `sperre_scheiben_hermetisch.rs`.
//!
//! **Belegt wird (positiver Beleg).** Die Frage steht erst bei einem BESTAETIGTEN Preis ueber 800,00 EUR. Nur ein bestaetigtes „ja“ nimmt die
//! Satzfrage weg; „nein“, Schweigen und ein vorlaeufiges „ja“ lassen sie stehen (fail-closed). Die Fragen nach dem Anschaffungsjahr und nach
//! der eigenen Nutzungsdauer bleiben bei „ja“ in der Queue: die Sperre verlangt das Anschaffungsjahr, die eigene Nutzungsdauer ist die
//! Wahl nach Rz. 1.2. Beide Scheiben, die Arbeitsmittel fragen (`gesamt`, `an_gesamt`).
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

const LISTE: &str = "am_computer_software";
const SATZ: &str = "am_umsatzsteuersatz";
const PREIS: &str = "am_anschaffungskosten";
const ANSCHAFFUNGSJAHR: &str = "am_afa_ist_anschaffungsjahr";
const NUTZUNGSDAUER: &str = "arbeitsmittel_nutzungsdauer";

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

/// Ein Ereignis, bestaetigt (zwei Signale) oder vorlaeufig (nur das erste, `signal_2` leer).
fn ereignis(feld_id: &str, wert: &Value, bestaetigt: bool) -> Value {
    let signal_2 = if bestaetigt {
        json!(format!("ok@{feld_id}"))
    } else {
        Value::Null
    };
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

/// Legt den Fall `am` der Scheibe `scheibe` (VZ 2025) an und schreibt jedes Paar ueber die echte Route `POST /event`.
async fn fall(d: &Dienst, scheibe: &str, paare: &[(&str, Value, bool)]) {
    let kopf = json!({"fall_id": "am", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert, bestaetigt) in paare {
        let (status, antwort) =
            sende(d, "POST", "/fall/am/event", Some(&ereignis(feld, wert, *bestaetigt))).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
}

/// Die Feld-Ids der Fragen eines Falls in der Reihenfolge der Queue.
async fn fragen_von(scheibe: &str, paare: &[(&str, Value, bool)]) -> Vec<String> {
    let d = dienst();
    fall(&d, scheibe, paare).await;
    let (status, fragen) = sende(&d, "GET", "/fall/am/fragen", None).await;
    assert_eq!(status, 200, "GET /fragen: {fragen}");
    fragen["fragen"]
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["feld_id"].as_str().unwrap().to_owned())
        .collect()
}

fn steht(fragen: &[String], feld: &str) -> bool {
    fragen.iter().any(|f| f == feld)
}

/// Die Frage nach der Liste steht erst bei einem BESTAETIGTEN Preis ueber 800,00 EUR (80.000 ct), schon bei einem Cent mehr. Bis
/// 800,00 EUR gilt der Sofortabzug als `GWG` ohnehin voll, darunter braucht es die Liste nicht. Bei Schweigen, 0, 800,00 EUR und einem
/// nur vorlaeufigen Preis steht sie nicht; die Preisfrage steht dann in der Queue und schaltet sie frei.
#[tokio::test]
async fn die_frage_zur_liste_steht_erst_bei_bestaetigtem_preis_ueber_800_euro() {
    for scheibe in ["gesamt", "an_gesamt"] {
        for (name, preis, erwartet) in [
            ("ohne Preis", None, false),
            ("Preis 0", Some((0, true)), false),
            ("Preis 500,00 EUR", Some((50_000, true)), false),
            ("Preis 800,00 EUR: die Grenze", Some((80_000, true)), false),
            ("Preis 800,01 EUR", Some((80_001, true)), true),
            ("Preis 900,00 EUR", Some((90_000, true)), true),
            ("Preis 1.500,00 EUR", Some((150_000, true)), true),
            ("Preis 900,00 EUR nur vorlaeufig", Some((90_000, false)), false),
        ] {
            let paare: Vec<(&str, Value, bool)> = preis
                .map(|(p, b)| (PREIS, json!(p), b))
                .into_iter()
                .collect();
            let fragen = fragen_von(scheibe, &paare).await;
            assert_eq!(
                steht(&fragen, LISTE),
                erwartet,
                "{scheibe}, {name}: Frage zur Liste"
            );
            if !matches!(preis, Some((_, true))) {
                assert!(
                    steht(&fragen, PREIS),
                    "{scheibe}, {name}: die Preisfrage steht in der Queue und schaltet die Frage zur Liste frei"
                );
            }
        }
    }
}

/// Nur ein BESTAETIGTES „ja“ nimmt die Satzfrage weg: das Geraet der Liste durchlaeuft die Netto-Pruefung nicht (Abweichung Nr. 88).
/// „nein“, keine Antwort und ein nur vorlaeufiges „ja“ lassen sie stehen. Positivkontrolle: in diesen drei Faellen steht sie (die Probe
/// sieht die Satzfrage, wenn sie da ist). Auch ueber 952,00 EUR (dort steht die Satzfrage ohne „ja“ ebenfalls).
#[tokio::test]
async fn nur_ein_bestaetigtes_ja_nimmt_die_satzfrage_weg() {
    for scheibe in ["gesamt", "an_gesamt"] {
        for preis in [90_000, 150_000] {
            for (name, antwort, satz_da) in [
                ("keine Antwort", None, true),
                ("nein bestaetigt", Some((false, true)), true),
                ("ja nur vorlaeufig", Some((true, false)), true),
                ("ja bestaetigt", Some((true, true)), false),
            ] {
                let mut paare: Vec<(&str, Value, bool)> = vec![(PREIS, json!(preis), true)];
                if let Some((wert, bestaetigt)) = antwort {
                    paare.push((LISTE, json!(wert), bestaetigt));
                }
                let fragen = fragen_von(scheibe, &paare).await;
                assert_eq!(
                    steht(&fragen, SATZ),
                    satz_da,
                    "{scheibe}, {preis} ct, {name}: Satzfrage"
                );
            }
        }
    }
}

/// Bei „ja“ bleiben die Frage nach dem Anschaffungsjahr (die Sperre verlangt sie, sonst entschiede ein stilles „Folgejahr“ zwischen
/// Abzug und 0) und die nach der eigenen Nutzungsdauer (Rz. 1.2: abweichen ist erlaubt) in der Queue. Sonst waere die Sperre eine Sackgasse.
/// Positivkontrolle: ohne die Antwort stehen beide ebenfalls.
#[tokio::test]
async fn bei_ja_bleiben_anschaffungsjahr_und_nutzungsdauer_in_der_queue() {
    for scheibe in ["gesamt", "an_gesamt"] {
        for (name, antwort) in [("ohne Antwort", None), ("ja bestaetigt", Some(true))] {
            let mut paare: Vec<(&str, Value, bool)> = vec![(PREIS, json!(150_000), true)];
            if let Some(a) = antwort {
                paare.push((LISTE, json!(a), true));
            }
            let fragen = fragen_von(scheibe, &paare).await;
            for feld in [ANSCHAFFUNGSJAHR, NUTZUNGSDAUER] {
                assert!(steht(&fragen, feld), "{scheibe}, {name}: {feld} steht nicht in der Queue");
            }
        }
    }
}

/// Die Frage zur Liste kommt vor der Satzfrage: wer „ja“ antwortet, soll den Satz nie sehen. Preis 900,00 EUR, sonst nichts beantwortet.
#[tokio::test]
async fn die_frage_zur_liste_kommt_vor_der_satzfrage() {
    for scheibe in ["gesamt", "an_gesamt"] {
        let fragen = fragen_von(scheibe, &[(PREIS, json!(90_000), true)]).await;
        let pos = |feld: &str| fragen.iter().position(|f| f == feld);
        let (liste, satz) = (pos(LISTE), pos(SATZ));
        assert!(
            liste.is_some() && satz.is_some() && liste < satz,
            "{scheibe}: Liste {liste:?}, Satz {satz:?}"
        );
    }
}

/// Die Bedingung steuert nur die Queue, nicht den Schreibweg: `POST /event` nimmt ein Ja/Nein auch an, wenn die Frage gerade nicht
/// steht (ein Beleg-Import). Alles andere ist ein Typfehler, kein stilles „ja“.
#[tokio::test]
async fn der_schreibweg_nimmt_nur_ein_ja_oder_nein_an() {
    for (wert, erwartet) in [
        (json!(true), 201),
        (json!(false), 201),
        (json!("ja"), 422),
        (json!(1), 422),
        (json!("true"), 422),
    ] {
        let d = dienst();
        fall(&d, "gesamt", &[]).await;
        let (status, antwort) =
            sende(&d, "POST", "/fall/am/event", Some(&ereignis(LISTE, &wert, true))).await;
        assert_eq!(status, erwartet, "{wert}: {antwort}");
    }
}

/// AK2: Kein Text der Bindung nennt „3 Jahre fuer Laptop“; kein Text nennt ein Computergeraet zusammen mit „sofort absetzen“ oder „ohne
/// 800-Euro-Grenze“ (die Rechtsgrundlage ist die Nutzungsdauer von einem Jahr, nicht das `GWG`). Die zwei Nutzungsdauer-Felder verweisen
/// auf das Schreiben und nennen ein Jahr. Die Frage zur Liste nennt das Schreiben, die Randnummern der Liste und die Kennzeichnungspflicht
/// (Rz. 4), und sie sagt, dass ein Smartphone nicht auf der Liste steht.
#[test]
fn die_hilfetexte_nennen_fuer_computer_ein_jahr_und_kein_sofortabzug() {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let reg = bindung::lade_registry_der_wurzel(&wurzel).unwrap();
    let alle: Vec<&bindung::Bindung> = reg
        .dateien
        .iter()
        .flat_map(|(_, d)| d.bindungen.iter())
        .collect();
    let text_von = |fid: &str| -> String {
        let b = alle
            .iter()
            .find(|b| b.feld_id == fid)
            .unwrap_or_else(|| panic!("KONTROLLE: Feld {fid} fehlt in der Registry"));
        format!("{} {}", b.fragetext_laie.clone().unwrap_or_default(), b.hilfe_kurz)
    };

    // Positivkontrolle: die Pruefung sieht Texte (viele Felder, jedes mit Hilfetext) und die Felder, um die es geht.
    assert!(alle.len() > 300, "nur {} Felder gelesen", alle.len());
    for fid in [LISTE, SATZ, PREIS, NUTZUNGSDAUER, "gwg_nutzungsdauer"] {
        assert!(!text_von(fid).trim().is_empty(), "{fid} ohne Text");
    }

    let geraet = ["Laptop", "Notebook", "Computer", "Rechner"];
    let sofort = ["sofort absetzen", "sofort komplett absetzen", "ohne 800", "ohne die 800", "ohne Grenze"];
    let mut verstoesse: Vec<String> = Vec::new();
    for b in &alle {
        for (art, text) in [
            ("Fragetext", b.fragetext_laie.clone().unwrap_or_default()),
            ("Hilfetext", b.hilfe_kurz.clone()),
        ] {
            if text.contains("3 Jahre für Laptop") || text.contains("3 Jahre fuer Laptop") {
                verstoesse.push(format!("{} {art}: nennt 3 Jahre fuer Laptop", b.feld_id));
            }
            if geraet.iter().any(|g| text.contains(g)) && sofort.iter().any(|s| text.contains(s)) {
                verstoesse.push(format!("{} {art}: Computer zusammen mit Sofortabzug", b.feld_id));
            }
        }
    }
    assert_eq!(verstoesse.len(), 0, "{verstoesse:#?}");

    for fid in [NUTZUNGSDAUER, "gwg_nutzungsdauer"] {
        let t = text_von(fid);
        assert!(
            t.contains("BMF-Schreiben zur Nutzungsdauer von Computerhardware und Software") && t.contains("ein Jahr"),
            "{fid}: nennt weder das Schreiben noch ein Jahr: {t}"
        );
    }
    let t = text_von(LISTE);
    for muss in [
        "BMF-Schreiben zur Nutzungsdauer von Computerhardware und Software",
        "Smartphone",
        "umweltgerechten Gestaltung von Computern",
        "Bestandsverzeichnis",
        "ein Jahr",
    ] {
        assert!(t.contains(muss), "{LISTE}: Text nennt {muss:?} nicht: {t}");
    }
}
