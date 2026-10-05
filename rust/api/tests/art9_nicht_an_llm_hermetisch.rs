//! Besondere Kategorien nach Art. 9 DSGVO (Gesundheit, Konfession) gehen nicht an das Sprachmodell, und
//! ein NEUES solches Bindungsfeld faellt auf, statt vergessen zu werden. Ohne Netz, ohne Python.
//!
//! Der Schutz ist KEIN Typ (`Art9<T>`, wie die Portierkarte es versprach), sondern ein Filter am Aufrufort:
//! `chat::erklaer_kontext` laesst den Wert jedes bestaetigten Feldes aus, das `llm::pii::ist_besondere_kategorie`
//! trifft, und nennt nur die Zahl der ausgelassenen Angaben. Die Regel (ein Muster ueber `feld_id` und Fragetext)
//! und der Aufrufort verrotten auf zwei verschiedenen Wegen, darum zwei Pruefungen:
//!
//! 1. Der Weg: eine Akte mit je einem bestaetigten Wert in jedem verdaechtigen Bindungsfeld, ueber die echte
//!    Registry und den echten HTTP-Weg `POST /fall/{id}/chat` (drei Stufen) und `POST /fall/{id}/kontoauszug`
//!    (der zweite Weg zum Modell). Kein Anfragekoerper, den der Attrappen-Dienst sieht, traegt einen der Werte.
//! 2. Die Regel: jedes Bindungsfeld, dessen `feld_id` oder Fragetext ein Art.-9-Merkmal nennt, wird von der Regel
//!    erfasst. Die Merkmalsliste ist UNABHAENGIG vom Muster in `pii.rs` geschrieben, sonst hielte der Test die
//!    Regel nur gegen sich selbst. Ein Feld, das nur das Suchwort trifft, steht mit Grund in der
//!    Unverfaenglich-Liste; ein toter Eintrag dort ist ebenfalls ein Fehler.
//!
//! Wege zum Modell (Stand der Anlage): (a) Chat Stufe 1 und 2 tragen nur den gefilterten Freitext des Nutzers und
//! den Katalog (Fragetexte, keine Werte); (b) Chat Stufe 3 traegt zusaetzlich `erklaer_kontext`, die einzige Stelle
//! mit Feldwerten aus der Akte; (c) der Kontoauszug-Klassifikator traegt nur `Zweck` (maskiert) und `Betrag` einer
//! Buchung, Typen `Maskiert` und `i64`, kein Zugriff auf die Akte. Was der Nutzer selbst in den Freitext tippt, ist
//! ausdruecklich NICHT Gegenstand: ein Textfilter kann „80“ nicht von einem Betrag unterscheiden, die Sperre
//! gehoert an die Quelle, das Feld. Die OpenRouteService-Anfragen (`llm::ors`) gehen nicht an ein Sprachmodell.
//!
//! Die Umgebung (`$LLM_API_BASE`) gilt fuer den ganzen Prozess; darum steht der Weg in EINEM Test, die beiden
//! Registry-Pruefungen lesen sie nicht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::{Path, PathBuf};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use llm::pii::ist_besondere_kategorie;
use serde_json::{json, Value};
use tower::ServiceExt;

#[path = "../../llm/tests/stub/mod.rs"]
mod stub;
use stub::{antwort, Aktion, Stub};

/// Begriffe, die eine besondere Kategorie ankuendigen, unabhaengig vom Muster in `llm::pii` formuliert.
const VERDAECHTIG: [&str; 14] = [
    "behinder",
    "konfession",
    "kirche",
    "pflege",
    "hilflos",
    "blind",
    "taubblind",
    "merkzeichen",
    "berufsunfähig",
    "berufsunfaehig",
    "erwerbsunfähig",
    "krankheitskosten",
    "heilbehandlung",
    "schwerbehind",
];

/// GEPRUEFT, KEIN ART. 9: das Suchwort greift daneben. Jeder Eintrag mit Grund; ohne die Liste muesste das Suchwort
/// so eng werden, dass es echte Faelle verpasst, oder die Sperre so weit, dass sie den Chat verstuemmelt.
const GEPRUEFT_UNVERFAENGLICH: [(&str, &str); 14] = [
    ("basis_pv", "Beitrag zur Pflegeversicherung, kein Merkmal"),
    ("basis_pv_partner", "Beitrag zur Pflegeversicherung, kein Merkmal"),
    ("kind_pv", "Beitrag zur Pflegeversicherung des Kindes, kein Merkmal"),
    ("p33a_unterhalt_kv_pv", "Beitraege fuer eine unterhaltsberechtigte Person, kein Merkmal"),
    ("realsplitting_empfaenger_kv_pv", "Beitraege fuer den Ex-Partner, kein Merkmal"),
    ("realsplitting_empfaenger_kv_krankengeld", "Anteil eines Beitrags, kein Merkmal"),
    ("versicherungsart", "gesetzlich/privat, sagt nichts ueber Gesundheit"),
    ("versicherungsart_partner", "gesetzlich/privat, sagt nichts ueber Gesundheit"),
    ("kind_kindschaftsverhaeltnis_a", "Verwandtschaft (leiblich/Adoptiv/Pflegekind)"),
    ("kind_kindschaftsverhaeltnis_b", "Verwandtschaft (leiblich/Adoptiv/Pflegekind)"),
    ("kind_anderer_elternteil_kindschaftsverhaeltnis", "Verwandtschaft, kein Merkmal"),
    ("kind_unter_14_haushaltszugehoerig", "Alter und Haushaltszugehoerigkeit, kein Merkmal"),
    ("hh_dienstleistung_art", "Art der Haushaltsleistung, z. B. 'Gartenpflege'"),
    (
        "antrag_ermaessigter_satz",
        "Antrag auf ermaessigten Satz; die Voraussetzung selbst steht in dauernd_berufsunfaehig und ist gesperrt",
    ),
];

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `(feld_id, Fragetext oder "")` jeder Bindung, die der Dienst kennt: alle Dateien in
/// `rust/bindung/daten`, geladen wie der Dienst es tut (`bindung::lade_registry_der_wurzel`).
fn bindungen() -> Vec<(String, String)> {
    let registry = bindung::lade_registry_der_wurzel(&wurzel()).unwrap();
    let mut alle = Vec::new();
    for (_, datei) in &registry.dateien {
        for b in &datei.bindungen {
            alle.push((
                b.feld_id.clone(),
                b.fragetext_laie.clone().unwrap_or_default(),
            ));
        }
    }
    alle
}

fn verdaechtig(feld_id: &str, frage: &str) -> bool {
    let text = format!("{feld_id} {frage}").to_lowercase();
    VERDAECHTIG.iter().any(|w| text.contains(w))
}

fn unverfaenglich(feld_id: &str) -> bool {
    GEPRUEFT_UNVERFAENGLICH.iter().any(|(f, _)| *f == feld_id)
}

// ------------------------------------------------------------------ die Regel

/// Der eigentliche Wert dieser Datei: kein Bindungsfeld mit Art.-9-Merkmal rutscht an der Regel vorbei.
#[test]
fn kein_bindungsfeld_mit_art9_merkmal_rutscht_an_der_regel_vorbei() {
    let alle = bindungen();
    let durchgerutscht: Vec<String> = alle
        .iter()
        .filter(|(f, q)| verdaechtig(f, q) && !ist_besondere_kategorie(f, q) && !unverfaenglich(f))
        .map(|(f, q)| format!("{f}: „{}…", q.chars().take(70).collect::<String>()))
        .collect();
    assert!(
        durchgerutscht.is_empty(),
        "Bindungsfelder nennen ein Merkmal nach Art. 9 DSGVO, werden von llm::pii::ist_besondere_kategorie \
         aber nicht erfasst, ihr Wert ginge an den Anbieter des Sprachmodells:\n  {}\n\nEntweder das Muster in \
         llm/src/pii.rs erweitern oder das Feld in GEPRUEFT_UNVERFAENGLICH eintragen, MIT dem Grund.",
        durchgerutscht.join("\n  ")
    );
    // Tote Eintraege: was die Regel inzwischen erfasst, gehoert nicht mehr in die Liste, sonst deckt der
    // Eintrag beim naechsten Mal ein echtes Merkmal mit ab. Und ein Feld, das es nicht mehr gibt, auch nicht.
    let zombies: Vec<&str> = GEPRUEFT_UNVERFAENGLICH
        .iter()
        .map(|(f, _)| *f)
        .filter(|f| {
            alle.iter()
                .any(|(a, q)| a == f && ist_besondere_kategorie(a, q))
        })
        .collect();
    assert!(
        zombies.is_empty(),
        "{zombies:?} stehen als unverfaenglich drin, die Regel erfasst sie aber: Eintrag streichen"
    );
    let fehlend: Vec<&str> = GEPRUEFT_UNVERFAENGLICH
        .iter()
        .map(|(f, _)| *f)
        .filter(|f| !alle.iter().any(|(a, _)| a == f))
        .collect();
    assert!(
        fehlend.is_empty(),
        "{fehlend:?} gibt es in keiner Bindung mehr: Eintrag streichen"
    );
}

/// Gegenprobe gegen die bequemste Art, den Test oben gruen zu bekommen: die Regel so weit fassen, dass sie alles
/// trifft, oder so eng, dass sie nichts trifft. Zu viele Treffer sperren den Chat aus, ohne dass es jemand merkt.
#[test]
fn die_regel_trifft_weder_alles_noch_nichts() {
    let alle = bindungen();
    let treffer: Vec<&str> = alle
        .iter()
        .filter(|(f, q)| ist_besondere_kategorie(f, q))
        .map(|(f, _)| f.as_str())
        .collect();
    assert!(
        (12..=60).contains(&treffer.len()),
        "{} von {} Bindungsfeldern als besondere Kategorie erkannt (erwartet 12 bis 60). Zu wenige: die Regel \
         greift nicht mehr. Zu viele: sie sperrt den Chat aus.\n{treffer:?}",
        treffer.len(),
        alle.len()
    );
    for pflicht in [
        "kist_konfession",
        "rentner_grad_der_behinderung",
        "rentner_hilflos_blind_taubblind",
        "kind_grad_der_behinderung",
    ] {
        assert!(
            treffer.contains(&pflicht),
            "{pflicht} wird nicht als besondere Kategorie erkannt"
        );
    }
    // Die Gegenrichtung: ein Betrag nennt die besondere Kategorie nicht; aus „Kirchensteuer gezahlt: 412 EUR“
    // folgt keine Konfession.
    for harmlos in [
        "kist_gezahlt",
        "kist_erstattet",
        "basis_kv",
        "basis_pv",
        "bruttoarbeitslohn",
    ] {
        assert!(
            !ist_besondere_kategorie(harmlos, ""),
            "{harmlos} ist ein Betrag, kein Merkmal"
        );
    }
}

// ------------------------------------------------------------------ der Weg

struct Dienst {
    zustand: Zustand,
    token: String,
    _tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: wurzel(),
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

async fn post(d: &Dienst, pfad: &str, rumpf: &Value) -> (u16, Value) {
    let text = rumpf.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("authorization", format!("Bearer {}", d.token))
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string())
        .body(Body::from(text))
        .unwrap();
    let r = app(d.zustand.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("kein JSON: {e}: {}", String::from_utf8_lossy(&bytes)));
    (teile.status.as_u16(), json)
}

/// Bestaetigte Ereignisse direkt in die Akte schreiben (wie `setze_bestaetigt` in
/// `chat_llm_attrappe_hermetisch.rs`): der Weg zum Modell liest sie aus dem Store, nicht aus dem Rumpf.
fn setze_bestaetigt(d: &Dienst, fall: &str, paare: &[(String, Value)]) {
    let pfad = d.zustand.konfig.faelle.join(format!("{fall}.json"));
    let mut datei = store::lade(&pfad).unwrap();
    for (i, (fid, wert)) in paare.iter().enumerate() {
        let mut e = json!({
            "ts": format!("2026-01-01T00:{:02}:{:02}+00:00", i / 60, i % 60), "feld_id": fid, "wert": wert,
            "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": "ok"},
        });
        e["event_id"] = json!(store::EventId::von_json(&e).to_string());
        datei.events.push(serde_json::from_value(e).unwrap());
    }
    store::speichere(&pfad, &datei).unwrap();
}

/// Eine Modell-Antwort mit `inhalt` als Text der ersten Wahl.
fn modell(inhalt: &Value) -> Aktion {
    Aktion::Roh(antwort(
        200,
        &json!({
            "provider": "StubAnbieter",
            "choices": [{"finish_reason": "stop", "message": {"content": inhalt.to_string()}}]
        })
        .to_string(),
    ))
}

/// Alle Nachrichten einer Anfrage, die das Modell erreichen, mit Rolle, zusammengefuegt.
fn nachrichten(koerper: &[u8]) -> String {
    let v: Value = serde_json::from_slice(koerper).unwrap();
    v["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            format!(
                "{}: {}",
                m["role"].as_str().unwrap(),
                m["content"].as_str().unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const TEXT: &str = "Ich habe 100 Euro Lohn bekommen.";

#[tokio::test]
async fn art9_werte_der_akte_erreichen_das_sprachmodell_auf_keinem_weg() {
    // Drei Stufen des Chats, danach die Klassifikation einer Buchung: vier Antworten, in dieser Reihenfolge.
    let stub = Stub::starte(vec![
        modell(
            &json!({"aussagen": [{"text": "Der Nutzer hatte 100 Euro Lohn", "beleg": "100 Euro Lohn"}]}),
        ),
        modell(&json!({"zuordnungen": []})),
        modell(
            &json!({"vorschlaege": [], "rueckfragen": [], "antwort": "Das habe ich verstanden.", "unsicher": false}),
        ),
        modell(&json!({"kategorie": null})),
    ]);
    std::env::set_var("LLM_API_BASE", stub.basis(""));
    std::env::set_var("LLM_MODEL", "stub/modell");
    std::env::set_var("LLM_API_KEY", "SYNTHETISCH-LLM-SCHLUESSEL");
    std::env::remove_var("TAXGRAPH_FLOW");

    let alle = bindungen();
    // Was nicht hinausgehen darf: alles, was die UNABHAENGIGE Liste verdaechtigt (ohne die geprueften
    // Ausnahmen), und alles, was die Regel trifft. Jedes Feld bekommt einen eigenen, unverwechselbaren Wert.
    let art9: Vec<(&str, &str, String)> = alle
        .iter()
        .filter(|(f, q)| (verdaechtig(f, q) && !unverfaenglich(f)) || ist_besondere_kategorie(f, q))
        .enumerate()
        .map(|(n, (f, q))| (f.as_str(), q.as_str(), format!("ZEUGE{n:03}ARTNEUN")))
        .collect();
    assert!(
        art9.len() >= 12,
        "nur {} Felder zu sperren: die Registry ist leer gelesen?",
        art9.len()
    );
    let ausgelassen = art9
        .iter()
        .filter(|(f, q, _)| ist_besondere_kategorie(f, q))
        .count();

    // Das Kontrollfeld: ein harmloser Betrag MUSS im Kontext stehen, sonst waere „nichts geht hinaus“ gruen,
    // weil der Kontext leer ist.
    let kontroll_frage = alle
        .iter()
        .find(|(f, _)| f == "bruttoarbeitslohn")
        .map(|(_, q)| q.clone())
        .unwrap();
    let mut paare: Vec<(String, Value)> = vec![("bruttoarbeitslohn".into(), json!(1_234_567))];
    paare.extend(
        art9.iter()
            .map(|(f, _, wert)| ((*f).to_owned(), json!(wert))),
    );

    let d = dienst();
    let (s, a) = post(
        &d,
        "/fall",
        &json!({"fall_id": "a9", "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    )
    .await;
    assert_eq!(s, 201, "{a}");
    setze_bestaetigt(&d, "a9", &paare);

    // --- Weg 1: der Chat, drei Stufen.
    let (s, a) = post(&d, "/fall/a9/chat", &json!({"text": TEXT})).await;
    assert_eq!(s, 200, "{a}");
    // --- Weg 2: der Kontoauszug, eine Buchung, die kein Stichwort trifft und darum das Modell fragt.
    let (s, a) = post(
        &d,
        "/fall/a9/kontoauszug",
        &json!({"format": "csv", "inhalt": "datum;betrag;verwendungszweck\n07.03.2025;-9,99;Einkauf\n"}),
    )
    .await;
    assert_eq!(s, 200, "{a}");

    let anfragen = stub.anfragen();
    assert_eq!(
        anfragen.len(),
        4,
        "drei Stufen des Chats und eine Buchung: das Modell wurde nicht viermal gefragt"
    );
    let nachr: Vec<String> = anfragen.iter().map(|a| nachrichten(&a.koerper)).collect();
    let roh: Vec<String> = anfragen
        .iter()
        .map(|a| String::from_utf8_lossy(&a.koerper).into_owned())
        .collect();

    // Kontrolle: der Kontext der dritten Stufe ist nicht leer und der harmlose Betrag steht drin.
    assert!(
        nachr[2].contains(&format!("Das hat der Nutzer bereits bestätigt:\n- {kontroll_frage} → 12345,67 EUR")),
        "der harmlose Betrag fehlt im Kontext, die Sperre greift zu weit oder der Kontext fehlt:\n{}",
        nachr[2]
    );
    // Kontrolle Weg 2: das Modell sah genau die Buchung.
    assert!(
        nachr[3].contains("Zweck: Einkauf\nBetrag: -9.99 EUR"),
        "{}",
        nachr[3]
    );

    // Der Kern: weder ein Wert noch die Frage-Zeile eines gesperrten Feldes steht in irgendeiner Anfrage.
    for (f, q, wert) in &art9 {
        for (i, r) in roh.iter().enumerate() {
            assert!(
                !r.contains(wert.as_str()),
                "Wert von {f} steht in Anfrage {} an das Modell",
                i + 1
            );
        }
        if !q.is_empty() {
            for (i, n) in nachr.iter().enumerate() {
                assert!(
                    !n.contains(&format!("- {q} → ")),
                    "die Antwortzeile von {f} steht in Anfrage {} an das Modell",
                    i + 1
                );
            }
        }
    }

    // Die Auslassung wird genannt, nicht verschwiegen: eine stille Kuerzung liesse das Modell nach Dingen
    // fragen, die laengst beantwortet sind.
    assert!(
        nachr[2].contains(&format!(
            "({ausgelassen} weitere Angaben liegen vor, dürfen dir aber nicht übermittelt werden"
        )),
        "die Zahl der ausgelassenen Angaben ({ausgelassen}) wird nicht genannt:\n{}",
        nachr[2]
    );
}
