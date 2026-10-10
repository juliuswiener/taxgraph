//! Abweichung Nr. 53 (Stufe 1, nur Anzeigetexte): Eine Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung ist nach
//! § 22 Nr. 5 `EStG` kein Versorgungsbezug und hat eine Leistungsmitteilung statt einer Lohnsteuerbescheinigung. `TaxGraph` hat
//! dafuer keinen Eingabeplatz (Anlage R-AV/bAV ist nicht gebaut). Vor Nr. 53 luden mehrere Texte ausdruecklich zur Versorgung
//! ein ("Betriebsrente oder Direktversicherung"); der Bescheid rechnete dort den Versorgungsfreibetrag (Probefall 6.000 EUR,
//! VZ 2025: Einkommensteuer 945 EUR zu hoch oder 279 EUR zu niedrig). Die Texte trennen jetzt die zwei Faelle.
//!
//! Gemessen wird, was der Nutzer sieht: die Einzelfrage `GET /fall/{id}/feld/{fid}/frage` (Fragetext, Hilfe, Auswahltexte) in
//! jeder Scheibe, die das Feld fuehrt, und der Klartext der zwei Sperrgruende zur Versorgung. Der Wortlaut der Abgabe-Sperre
//! (`VERSORGUNG_SPERRE`) steht in `rust/elster/src/deklaration.rs` (Unit-Test dort).
//!
//! - AK1: der Auswahltext von `altersgrenze_sonstige` (`versorgung_art`, `versorgung_art_partner`) lautet "Pension des
//!   frueheren Arbeitgebers (Werkspension)".
//! - AK2: die Kreuze `kein_lohn_pension` (A und Ehegatte), die Zaehlfrage `rentner_anzahl_renten` und die zwei Sperrtexte
//!   (`versorgungsfreibetrag_offen`, `versorgung_ueber_lohn`) nennen keine "Betriebsrente" mehr und trennen die Pension des
//!   Arbeitgebers von der Rente mit Leistungsmitteilung.
//! - AK3: die Hilfe von `kein_sonstige` und `kein_sonstige_partner` nennt die Rente mit Leistungsmitteilung; dort sperrt die
//!   Gesamtvariante bei "ja".
//! - AK4: die Hilfe von `rentner_renten_art` und `rentner_renten_art_partner` sagt, dass solche Renten dort nicht zu waehlen
//!   sind und die Software sie noch nicht rechnet.
//!
//! Keine Rechnung und keine Sperre aendert sich; nur Texte.
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
use domain::Sperrgrund;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Die Scheiben, in denen die Felder dieser Datei stehen koennen (`rentner_gesamt` und `gesamt`).
const SCHEIBEN: [&str; 2] = ["rentner_gesamt", "gesamt"];

/// Der Satz, der die Rente mit Leistungsmitteilung beim Namen nennt, wie ihn die Hilfe von `kein_sonstige` fuehrt.
const RENTE_LM: &str = "Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung mit Leistungsmitteilung";

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

/// Ein leerer Fall der Scheibe, VZ 2025.
async fn neuer_fall(scheibe: &str) -> Dienst {
    let d = dienst();
    let kopf = json!({"fall_id": "lm", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall ({scheibe}): {antwort}");
    d
}

/// Die Fragen zu `fid` aus jeder Scheibe, die das Feld fuehrt (die Route antwortet 404, wo es fehlt). Mindestens eine muss
/// es fuehren: sonst misst der Test nichts.
async fn fragen_zu(fid: &str) -> Vec<(&'static str, Value)> {
    let mut treffer = Vec::new();
    for scheibe in SCHEIBEN {
        let d = neuer_fall(scheibe).await;
        let (status, antwort) = sende(&d, "GET", &format!("/fall/lm/feld/{fid}/frage"), None).await;
        if status == 200 {
            treffer.push((scheibe, antwort["frage"].clone()));
        } else {
            assert_eq!(status, 404, "{fid} in {scheibe}: {antwort}");
        }
    }
    assert_ne!(treffer.len(), 0, "{fid}: in keiner Scheibe {SCHEIBEN:?}");
    treffer
}

/// Fragetext und Hilfe eines Felds, je Scheibe.
async fn texte_zu(fid: &str) -> Vec<(&'static str, String)> {
    fragen_zu(fid)
        .await
        .into_iter()
        .map(|(scheibe, q)| {
            let text = format!("{} {}", q["fragetext_laie"].as_str().unwrap(), q["hilfe_kurz"].as_str().unwrap());
            (scheibe, text)
        })
        .collect()
}

/// AK1: Der Auswahltext der Werkspension fuehrt nicht mehr zu "Betriebsrente oder Direktversicherung".
#[tokio::test]
async fn der_auswahltext_der_werkspension_nennt_den_frueheren_arbeitgeber() {
    for fid in ["versorgung_art", "versorgung_art_partner"] {
        for (scheibe, q) in fragen_zu(fid).await {
            let label = q["enum_labels"]["altersgrenze_sonstige"].as_str().unwrap();
            assert_eq!(label, "Pension des früheren Arbeitgebers (Werkspension)", "{fid} in {scheibe}");
            for verboten in ["Betriebsrente", "Direktversicherung"] {
                assert!(!label.contains(verboten), "{fid} in {scheibe}: {label}");
            }
        }
    }
}

/// AK2: Die Kreuze zu Lohn und Pension und die Zaehlfrage laden keine Rente mit Leistungsmitteilung in die Versorgung ein:
/// "Betriebsrente" fehlt, die Pension des Arbeitgebers heisst Werkspension, die Rente mit Leistungsmitteilung ist
/// ausgenommen. Das Kreuz sagt weiter, dass die gesetzliche Rente allein "nein" heisst (`rentner_eingangsfrage.rs`).
#[tokio::test]
async fn die_kreuze_zu_lohn_und_pension_trennen_werkspension_und_rente_mit_leistungsmitteilung() {
    for fid in ["kein_lohn_pension", "kein_lohn_pension_partner"] {
        for (scheibe, text) in texte_zu(fid).await {
            assert!(!text.contains("Betriebsrente"), "{fid} in {scheibe}: {text}");
            assert!(text.contains("Werkspension"), "{fid} in {scheibe}: Werkspension fehlt: {text}");
            assert!(
                text.contains(&format!("{RENTE_LM} zählt hier nicht")),
                "{fid} in {scheibe}: die Rente mit Leistungsmitteilung ist nicht ausgenommen: {text}"
            );
            assert!(text.contains("Nur die gesetzliche Rente: Nein"), "{fid} in {scheibe}: {text}");
        }
    }
}

/// AK2: Die Zaehlfrage nennt als zweite Rente keine Betriebsrente mehr: die Rente mit Leistungsmitteilung haette in der
/// Rentner-Scheibe die Anlage R statt der Anlage R-AV/bAV bekommen.
#[tokio::test]
async fn die_zaehlfrage_nennt_keine_betriebsrente_als_zweite_rente() {
    for (scheibe, text) in texte_zu("rentner_anzahl_renten").await {
        assert!(!text.contains("Betriebsrente"), "rentner_anzahl_renten in {scheibe}: {text}");
        assert!(text.contains("private Rentenversicherung"), "rentner_anzahl_renten in {scheibe}: {text}");
    }
}

/// AK2: Die zwei Sperrtexte zur Versorgung nennen keine Betriebsrente und nehmen die Rente mit Leistungsmitteilung aus.
#[test]
fn die_sperrtexte_zur_versorgung_nehmen_die_rente_mit_leistungsmitteilung_aus() {
    for grund in [Sperrgrund::VersorgungsfreibetragOffen, Sperrgrund::VersorgungUeberLohn] {
        let text = grund.klartext().unwrap();
        let name = grund.als_str();
        assert!(!text.contains("Betriebsrente"), "{name}: {text}");
        assert!(
            text.contains("Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung mit Leistungsmitteilung gehört nicht"),
            "{name}: die Rente mit Leistungsmitteilung ist nicht ausgenommen: {text}"
        );
    }
    let offen = Sperrgrund::VersorgungsfreibetragOffen.klartext().unwrap();
    assert!(offen.contains("Werkspension"), "versorgungsfreibetrag_offen: {offen}");
}

/// AK3: Die Hilfe der Kreuze zu sonstigen Einkuenften nennt die Rente mit Leistungsmitteilung; "ja" sperrt die
/// Gesamtvariante (`einkunftsart_nicht_ring_faehig`, fuer den Ehegatten `partner_einkunftsart_nicht_ring_faehig`).
#[tokio::test]
async fn die_kreuze_zu_sonstigen_einkuenften_nennen_die_rente_mit_leistungsmitteilung() {
    for fid in ["kein_sonstige", "kein_sonstige_partner"] {
        for (scheibe, q) in fragen_zu(fid).await {
            let hilfe = q["hilfe_kurz"].as_str().unwrap();
            assert!(hilfe.contains(RENTE_LM), "{fid} in {scheibe}: {hilfe}");
            assert!(!hilfe.contains("Betriebsrente"), "{fid} in {scheibe}: {hilfe}");
        }
    }
}

/// AK4: Die Hilfe der Rentenart sagt, dass die Rente mit Leistungsmitteilung dort nicht zu waehlen ist, weil die Software sie
/// noch nicht rechnet. Die fuenf Rentenarten bleiben unveraendert.
#[tokio::test]
async fn die_rentenart_nimmt_die_rente_mit_leistungsmitteilung_aus() {
    for fid in ["rentner_renten_art", "rentner_renten_art_partner"] {
        for (scheibe, q) in fragen_zu(fid).await {
            let hilfe = q["hilfe_kurz"].as_str().unwrap();
            assert!(
                hilfe.contains(&format!("Eine {RENTE_LM} gehört nicht hierher: die Software rechnet sie noch nicht.")),
                "{fid} in {scheibe}: {hilfe}"
            );
            assert_eq!(
                q["enum_werte"],
                json!(["gesetzliche_rente", "berufsstaendische_versorgung", "private_basisrente", "private_leibrente", "sonstige_leibrente"]),
                "{fid} in {scheibe}: die Rentenarten bleiben"
            );
        }
    }
}
