//! § 23 `EStG`: der private Verkauf wird gerechnet statt gesperrt (Abweichung Nr. 54 in `rust/fixtures/README.md`; ohne
//! `PARITY=1`, ohne Python).
//!
//! Bisher sperrte jeder bejahte Verkauf (`einkunftsart_nicht_ring_faehig`). Jetzt rechnet die Software die Hauptperson (Grundstueck,
//! anderes Wirtschaftsgut) samt Frist, Eigennutzung, taeglichem Gebrauch, Abschreibung und Freigrenze. Was sie nicht rechnet, sperrt
//! mit dem Verkauf im Klartext: Kryptowerte, Beteiligungen, Zusammenveranlagung, Teilnutzung (`p23_verkauf_nicht_gerechnet`);
//! fehlende oder unstimmige Angaben sperren mit `p23_angaben_offen`. Die Wächter aus `p23_waechter_hermetisch.rs` (Kreuz nie
//! beantwortet / widerspruechlich -> `flag_konsistenz_offen`) bleiben unveraendert.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: von Hand aus dem eingefrorenen Wortlaut des § 23 `EStG`
//! (`sources/gesetze-im-internet/estg_p23_2024-12-23.txt` und `estg_p23_2025-12-27.txt`), nicht aus der Ausgabe des Dienstes. Die
//! Baselines R0 und G0 sind die des Python-Orakels (siehe `p23_waechter_hermetisch.rs`). Jeder Zuschlag ist 42 % des steuerpflichtigen
//! Gewinns in vollen Euro: die Faelle liegen in der Zone 4 des Tarifs 2025 (zvE 68.481 bis 277.825 EUR; Rente bzw. Lohn von 200.000
//! EUR), dort steigt die `ESt` je Euro um 0,42, und jeder Gewinn hier ist ein Vielfaches von 50 EUR, so dass 0,42 x Gewinn glatt
//! aufgeht (kein Rundungsrest). 50.000 EUR -> 21.000 EUR (2.100.000 Cent), 40.000 -> 16.800, 45.000 -> 18.900, 30.000 -> 12.600,
//! 20.000 -> 8.400, 1.100 -> 462, 1.000 -> 420.
//!
//! Fristen (Abs. 1 Satz 1 Nr. 1 und 2, Fristende nach § 188 Abs. 2 BGB, abgeleitet, nicht im Korpus): Grundstueck bis zum 10. Jahrestag
//! des Kaufs einschliesslich steuerpflichtig, am Tag danach frei; anderes Wirtschaftsgut ebenso mit einem Jahr, mit Einkunftsquelle
//! (Satz 4) mit zehn Jahren. Eigennutzung frei (Nr. 1 Satz 3), taeglicher Gebrauch frei (Nr. 2 Satz 2). Freigrenze Abs. 3 Satz 5:
//! unter 1.000 EUR Gesamtgewinn des Jahres frei, ab 1.000,00 EUR der ganze Gewinn steuerpflichtig. Abschreibung mindert die
//! Anschaffungskosten (Abs. 3 Satz 4).
//!
//! ponytail: die Baselines sind eingefroren wie in `p23_waechter_hermetisch.rs`; aendert sich Tarif oder Kegel, rechnet man R0/G0 von
//! Hand nach.
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

/// Baseline Rentner einzel ohne Verkauf: 59.170,00 EUR festzusetzende `ESt`.
const R0: i64 = 5_917_000;
/// Baseline `gesamt` (AN 200.000 EUR, ohne Verkauf).
const G0: i64 = 7_255_600;
/// Zuschlag auf die `ESt` je 50.000 EUR steuerpflichtigem Gewinn (Zone 4: 42 %).
const PLUS_50_000: i64 = 2_100_000;

const NICHT_GERECHNET: &str = "p23_verkauf_nicht_gerechnet";
const ANGABEN_OFFEN: &str = "p23_angaben_offen";

type Paare = Vec<(String, Value)>;

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

/// Legt einen Fall der Scheibe an, schreibt jedes Paar ueber die echte Route `POST /event` (als Nutzer-Klick, bestaetigt; ein Feld
/// aus `vorlaeufig` ohne zweites Signal) und liefert `GET /ergebnis`.
async fn ergebnis_mit(scheibe: &str, paare: &[(String, Value)], vorlaeufig: &[&str]) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "p23", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let bestaetigt = !vorlaeufig.contains(&feld.as_str());
        let rumpf = json!({
            "feld_id": feld, "wert": wert,
            "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
            "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": bestaetigt.then(|| format!("ok@{feld}"))},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&rumpf)).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/p23/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

async fn ergebnis(scheibe: &str, paare: &[(String, Value)]) -> Value {
    ergebnis_mit(scheibe, paare, &[]).await
}

/// Legt einen Fall an, schreibt jedes Paar (bestaetigt) und liefert `GET /deklaration`, die Vorschau dessen, was die Abgabe schreibt.
async fn vorschau(scheibe: &str, paare: &[(String, Value)]) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "p23", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&rumpf)).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/p23/deklaration", None).await;
    assert_eq!(status, 200, "GET /deklaration: {antwort}");
    antwort
}

fn p(feld: &str, wert: Value) -> (String, Value) {
    (feld.to_owned(), wert)
}

/// Pflicht-Kegel `rentner_gesamt` ohne das Kreuz `kein_p23_verkauf` (wie `p23_waechter_hermetisch.rs`), Rente 20.000 EUR ab 2025.
fn kegel_rentner(veranlagung: &str) -> Paare {
    let mut k = vec![
        p("rentner_renten_art", json!("gesetzliche_rente")),
        p("rentner_jahresrente", json!(20_000_000)),
        p("rentner_renten_beginn_jahr", json!(2025)),
        p("rentner_alter_bei_rentenbeginn", json!(65)),
        p("rentner_grad_der_behinderung", json!(0)),
        p("rentner_hilflos_blind_taubblind", json!(false)),
        p("rentner_pflegegrad", json!(0)),
        p("rentner_gepflegter_hilflos", json!(false)),
        p("rentner_hinterbliebenenbezuege", json!(false)),
        p("veranlagung", json!(veranlagung)),
        p("kein_gewinn", json!(true)),
        p("kein_kap", json!(true)),
        p("kein_vuv", json!(true)),
        p("kein_sonstige", json!(false)),
        p("vor_an_anteil_rv", json!(0)),
        p("vor_ag_anteil_rv", json!(0)),
        p("vor_rv_ausserhalb_lstb", json!(0)),
        p("versicherungsart", json!("gesetzlich_an")),
        p("basis_kv", json!(0)),
        p("basis_pv", json!(0)),
        p("vorsorge_arbeitslosenversicherung", json!(0)),
        p("vorsorge_erwerbsunfaehigkeit", json!(0)),
        p("vorsorge_unfall_haftpflicht", json!(0)),
        p("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        p("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        p("mit_anspruch_auf_zuschuss", json!(false)),
        p("agb_zwangslaeufig", json!(true)),
        p("agb_notwendig_angemessen", json!(true)),
    ];
    if veranlagung == "zusammen" {
        for f in [
            "kap_kapitalertraege_partner",
            "kap_gewinn_aktien_partner",
            "kap_gewinn_sonstige_partner",
            "kap_verlust_aktien_partner",
            "kap_verlust_sonstige_partner",
        ] {
            k.push(p(f, json!(0)));
        }
    }
    k
}

/// Pflicht-Kegel `gesamt` ohne das Kreuz, AN 200.000 EUR, `kein_sonstige = true` (der Nutzer trennt die Fragen).
fn kegel_gesamt() -> Paare {
    vec![
        p("vv_einnahmen", json!(0)),
        p("vv_gebaeude_afa", json!(0)),
        p("vv_schuldzinsen", json!(0)),
        p("vv_erhaltungsaufwand", json!(0)),
        p("vv_sonstige_wk", json!(0)),
        p("vv_entgelt_quote_prozent", json!(0)),
        p("veranlagung", json!("einzel")),
        p("bruttoarbeitslohn", json!(20_000_000)),
        p("ep_arbeitstage", json!(0)),
        p("ep_entfernung_km", json!(0)),
        p("ep_oepnv_kosten", json!(0)),
        p("ep_eigenes_kfz", json!(false)),
        p("vor_an_anteil_rv", json!(0)),
        p("vor_ag_anteil_rv", json!(0)),
        p("vor_rv_ausserhalb_lstb", json!(0)),
        p("versicherungsart", json!("gesetzlich_an")),
        p("basis_kv", json!(0)),
        p("basis_pv", json!(0)),
        p("vorsorge_arbeitslosenversicherung", json!(0)),
        p("vorsorge_erwerbsunfaehigkeit", json!(0)),
        p("vorsorge_unfall_haftpflicht", json!(0)),
        p("vorsorge_rv_alt_mit_ueberschuss", json!(0)),
        p("vorsorge_rv_alt_ohne_ueberschuss", json!(0)),
        p("mit_anspruch_auf_zuschuss", json!(false)),
        p("kap_kapitalertraege", json!(0)),
        p("kap_gewinn_aktien", json!(0)),
        p("kap_verlust_aktien", json!(0)),
        p("kap_gewinn_sonstige", json!(0)),
        p("kap_verlust_sonstige", json!(0)),
        p("kein_gewinn", json!(true)),
        p("kein_kap", json!(true)),
        p("kein_vuv", json!(true)),
        p("kein_sonstige", json!(true)),
        p("agb_zwangslaeufig", json!(true)),
        p("agb_notwendig_angemessen", json!(true)),
    ]
}

/// Feldname der Instanz `n`: 1 traegt den Basisnamen, jede weitere `basis__n`.
fn name(basis: &str, n: u8) -> String {
    if n == 1 {
        basis.to_owned()
    } else {
        format!("{basis}__{n}")
    }
}

/// Ein vollstaendiger Verkauf der Instanz `n`: Art `typ`, Kaufdatum `a`, Verkaufsdatum `v` (TT.MM.JJJJ), Preis und Anschaffungskosten
/// in Cent, Werbungskosten und Abschreibung 0. Beim Grundstueck "keine Eigennutzung, kein Gebaeude der letzten zehn Jahre", beim anderen
/// Wirtschaftsgut "kein taeglicher Gebrauch, keine Einkunftsquelle".
fn verkauf(n: u8, typ: &str, a: &str, v: &str, preis: i64, ak: i64) -> Paare {
    let mut k = vec![
        p(&name("p23_veraeusserungs_typ", n), json!(typ)),
        p(&name("p23_bezeichnung", n), json!("Testobjekt")),
        p(&name("p23_anschaffungsdatum", n), json!(a)),
        p(&name("p23_veraeusserungsdatum", n), json!(v)),
        p(&name("p23_veraeusserungspreis", n), json!(preis)),
        p(&name("p23_anschaffung_herstellungskosten", n), json!(ak)),
        p(&name("p23_werbungskosten", n), json!(0)),
        p(&name("p23_afa", n), json!(0)),
    ];
    if typ == "anderes_wg" {
        k.push(p(&name("p23_taeglicher_gebrauch", n), json!(false)));
        k.push(p(&name("p23_einkunftsquelle", n), json!(false)));
    } else {
        k.push(p(&name("p23_eigennutzung", n), json!("nein")));
        k.push(p(&name("p23_gebaeude_in_frist", n), json!(false)));
    }
    k
}

/// Wie `grundstueck_pflichtig`, aber selbst bewohnt (Eigennutzung `art`) mit 120 m² Wohnflaeche, bewohnt vom 15.06.2020 bis zum 10.03.2025.
fn bewohnt(n: u8, art: &str) -> Paare {
    let k = mit(grundstueck_pflichtig(n), n, "p23_eigennutzung", json!(art));
    let k = mit(k, n, "p23_wohnflaeche", json!(120));
    let k = mit(k, n, "p23_eigennutzung_von", json!("15.06.2020"));
    mit(k, n, "p23_eigennutzung_bis", json!("10.03.2025"))
}

/// Ersetzt den Wert von `basis` in Instanz `n` (oder haengt das Feld an).
fn mit(mut k: Paare, n: u8, basis: &str, wert: Value) -> Paare {
    let ziel = name(basis, n);
    match k.iter_mut().find(|(f, _)| *f == ziel) {
        Some(eintrag) => eintrag.1 = wert,
        None => k.push((ziel, wert)),
    }
    k
}

/// Nimmt das Feld `basis` der Instanz `n` heraus (nie eingetragen).
fn ohne(mut k: Paare, n: u8, basis: &str) -> Paare {
    let ziel = name(basis, n);
    k.retain(|(f, _)| *f != ziel);
    k
}

/// Das Kreuz "ich habe verkauft" (`kein_p23_verkauf` = nein) und die Verkaeufe an den Kegel.
fn akte(mut kegel: Paare, verkaeufe: Paare) -> Paare {
    kegel.push(p("kein_p23_verkauf", json!(false)));
    kegel.extend(verkaeufe);
    kegel
}

/// Ein Grundstueck, das nach Frist und Eigennutzung steuerpflichtig ist: gekauft 15.06.2020, verkauft 10.03.2025, Gewinn 50.000 EUR.
fn grundstueck_pflichtig(n: u8) -> Paare {
    verkauf(n, "grundstueck", "15.06.2020", "10.03.2025", 10_000_000, 5_000_000)
}

fn erwarte_zahl(name: &str, a: &Value, zahl: i64) {
    assert_eq!(
        (a["grund"].as_str(), a["zahl_cent"].as_i64()),
        (Some("bestaetigt"), Some(zahl)),
        "{name}: erwartet bestaetigt mit {zahl} Cent. Antwort: {a}"
    );
}

fn erwarte_sperre(name: &str, a: &Value, grund: &str) {
    assert!(
        a["zahl_cent"].as_i64().is_none() && a["grund"] == grund,
        "{name}: erwartet Sperre {grund:?} ohne Zahl; erhalten grund={} zahl_cent={}. Antwort: {a}",
        a["grund"],
        a["zahl_cent"]
    );
}

async fn rentner(name: &str, verkaeufe: Paare, erwartet_cent: i64) {
    let a = ergebnis("rentner_gesamt", &akte(kegel_rentner("einzel"), verkaeufe)).await;
    erwarte_zahl(name, &a, erwartet_cent);
}

async fn rentner_gesperrt(name: &str, verkaeufe: Paare, grund: &str) {
    let a = ergebnis("rentner_gesamt", &akte(kegel_rentner("einzel"), verkaeufe)).await;
    erwarte_sperre(name, &a, grund);
}

// ---- Grundstueck: Frist, Eigennutzung --------------------------------------------------------------

/// Kontrolle: der steuerpflichtige Verkauf aendert die Zahl (sonst haetten die "frei"-Faelle unten nichts bewiesen).
#[tokio::test]
async fn grundstueck_innerhalb_von_zehn_jahren_wird_besteuert() {
    rentner("A1", grundstueck_pflichtig(1), R0 + PLUS_50_000).await;
}

#[tokio::test]
async fn grundstueck_nach_zehn_jahren_ist_steuerfrei() {
    let v = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    rentner("A2", v, R0).await;
}

/// Der Grenztag: Kauf 10.03.2015, Verkauf am 10.03.2025 ist noch der letzte Tag der Frist, der 11.03.2025 der erste danach.
#[tokio::test]
async fn grundstueck_am_zehnten_jahrestag_ist_steuerpflichtig_am_tag_danach_frei() {
    let letzter = verkauf(1, "grundstueck", "10.03.2015", "10.03.2025", 10_000_000, 5_000_000);
    rentner("A3a", letzter, R0 + PLUS_50_000).await;
    let danach = verkauf(1, "grundstueck", "10.03.2015", "11.03.2025", 10_000_000, 5_000_000);
    rentner("A3b", danach, R0).await;
}

#[tokio::test]
async fn grundstueck_nur_selbst_bewohnt_ist_steuerfrei() {
    rentner("A4", bewohnt(1, "ausschliesslich"), R0).await;
}

#[tokio::test]
async fn grundstueck_in_den_letzten_drei_jahren_bewohnt_ist_steuerfrei() {
    rentner("A5", bewohnt(1, "letzte_drei_jahre"), R0).await;
}

/// Ein selbst bewohnter Verkauf innerhalb der Frist braucht Wohnflaeche und Zeitraum (Zeile 36 der Anlage SO): ohne eines davon ist die
/// Angabe offen, und der Bescheid nennt keine Zahl. Gegenprobe: dasselbe Grundstueck, vor mehr als zehn Jahren gekauft, steht nicht in der
/// Erklaerung und braucht beides nicht (A4c).
#[tokio::test]
async fn selbst_bewohnter_verkauf_ohne_flaeche_oder_zeitraum_sperrt_mit_angaben_offen() {
    for fehlt in ["p23_wohnflaeche", "p23_eigennutzung_von", "p23_eigennutzung_bis"] {
        rentner_gesperrt(fehlt, ohne(bewohnt(1, "ausschliesslich"), 1, fehlt), ANGABEN_OFFEN).await;
    }
    rentner_gesperrt("A4b", mit(bewohnt(1, "ausschliesslich"), 1, "p23_wohnflaeche", json!(0)), ANGABEN_OFFEN).await;
    let alt = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    rentner("A4c", mit(alt, 1, "p23_eigennutzung", json!("ausschliesslich")), R0).await;
}

/// Teilweise bewohnt: der steuerpflichtige Anteil ist nicht gerechnet, also gesperrt statt falsch.
#[tokio::test]
async fn grundstueck_teilweise_bewohnt_sperrt_mit_dem_verkauf() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_eigennutzung", json!("teilweise"));
    rentner_gesperrt("A6", v, NICHT_GERECHNET).await;
}

/// Geerbt: der Nutzer traegt das Kaufdatum des Erblassers ein (§ 23 Abs. 1 Satz 3); 01.09.2010 liegt mehr als zehn Jahre zurueck.
#[tokio::test]
async fn geerbtes_grundstueck_zaehlt_ab_dem_kaufdatum_des_erblassers() {
    let frei = verkauf(1, "grundstueck", "01.09.2010", "20.03.2025", 10_000_000, 5_000_000);
    rentner("A7a", frei, R0).await;
    let pflichtig = verkauf(1, "grundstueck", "01.09.2020", "20.03.2025", 10_000_000, 5_000_000);
    rentner("A7b", pflichtig, R0 + PLUS_50_000).await;
}

/// Gekauft 2014: die Frist von zehn Jahren ist abgelaufen, der Verkauf ist frei. Hat der Nutzer in den letzten zehn Jahren darauf aber
/// ein Gebaeude gebaut, ausgebaut oder erweitert, ist dieser Teil steuerpflichtig (§ 23 Abs. 1 Nr. 1 Satz 2 `EStG`; Anleitung Anlage SO,
/// Zeilen 34 bis 42). Die Software rechnet die Aufteilung zwischen Boden und Gebaeude nicht und sperrt, statt 0 EUR zu melden.
/// Kontrolle zuerst: dasselbe Grundstueck ohne Gebaeude ist frei (sonst bewiese die Sperre nichts).
#[tokio::test]
async fn altes_grundstueck_mit_neuem_gebaeude_sperrt_mit_dem_verkauf() {
    let alt = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    rentner("A8a", alt.clone(), R0).await;
    rentner_gesperrt("A8b", mit(alt, 1, "p23_gebaeude_in_frist", json!(true)), NICHT_GERECHNET).await;
}

/// Das Gebaeude aendert nur, was es aendern muss: laeuft die Frist noch, ist das ganze Grundstueck steuerpflichtig; bei Eigennutzung sind
/// Gebaeude und Boden frei.
#[tokio::test]
async fn gebaeude_der_letzten_zehn_jahre_aendert_nichts_bei_laufender_frist_und_bei_eigennutzung() {
    let laufend = mit(grundstueck_pflichtig(1), 1, "p23_gebaeude_in_frist", json!(true));
    rentner("A9a", laufend, R0 + PLUS_50_000).await;
    let alt = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    let bewohnt = mit(mit(alt, 1, "p23_eigennutzung", json!("ausschliesslich")), 1, "p23_gebaeude_in_frist", json!(true));
    rentner("A9b", bewohnt, R0).await;
}

// ---- anderes Wirtschaftsgut ------------------------------------------------------------------------

/// Kauf 15.02.2025, Verkauf 20.08.2025: nach sechs Monaten, innerhalb des einen Jahres.
#[tokio::test]
async fn wirtschaftsgut_innerhalb_eines_jahres_wird_besteuert() {
    let v = verkauf(1, "anderes_wg", "15.02.2025", "20.08.2025", 10_000_000, 5_000_000);
    rentner("B1", v, R0 + PLUS_50_000).await;
}

/// Kauf 10.01.2024: am 10.01.2025 genau ein Jahr (letzter Tag der Frist, steuerpflichtig), am 11.01.2025 frei.
#[tokio::test]
async fn wirtschaftsgut_am_ersten_jahrestag_ist_steuerpflichtig_am_tag_danach_frei() {
    let letzter = verkauf(1, "anderes_wg", "10.01.2024", "10.01.2025", 10_000_000, 5_000_000);
    rentner("B2a", letzter, R0 + PLUS_50_000).await;
    let danach = verkauf(1, "anderes_wg", "10.01.2024", "11.01.2025", 10_000_000, 5_000_000);
    rentner("B2b", danach, R0).await;
}

/// Mit Einkunftsquelle (zum Beispiel eine Muenze, die Ertraege abwirft) gilt die Frist von zehn Jahren (Satz 4): fuenf Jahre sind
/// dann steuerpflichtig, ohne Einkunftsquelle frei.
#[tokio::test]
async fn wirtschaftsgut_mit_einkunftsquelle_hat_zehn_jahre_frist() {
    let ohne_quelle = verkauf(1, "anderes_wg", "10.03.2020", "10.03.2025", 10_000_000, 5_000_000);
    rentner("B3a", ohne_quelle.clone(), R0).await;
    let mit_quelle = mit(ohne_quelle, 1, "p23_einkunftsquelle", json!(true));
    rentner("B3b", mit_quelle, R0 + PLUS_50_000).await;
}

/// Gegenstaende des taeglichen Gebrauchs sind frei (Nr. 2 Satz 2), auch innerhalb des einen Jahres.
#[tokio::test]
async fn wirtschaftsgut_des_taeglichen_gebrauchs_ist_steuerfrei() {
    let v = verkauf(1, "anderes_wg", "15.02.2025", "20.08.2025", 10_000_000, 5_000_000);
    let v = mit(v, 1, "p23_taeglicher_gebrauch", json!(true));
    rentner("B4", v, R0).await;
}

// ---- Gewinn: Abschreibung, Werbungskosten, Freigrenze ----------------------------------------------

/// Preis 100.000, Anschaffung 80.000, Abschreibung 10.000: Gewinn 100.000 - (80.000 - 10.000) = 30.000 EUR. Ohne die Abschreibung
/// waeren es 20.000 EUR.
#[tokio::test]
async fn abschreibung_mindert_die_anschaffungskosten_und_erhoeht_den_gewinn() {
    let ohne_afa = verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 10_000_000, 8_000_000);
    rentner("C1a", ohne_afa.clone(), R0 + 840_000).await;
    let mit_afa = mit(ohne_afa, 1, "p23_afa", json!(1_000_000));
    rentner("C1b", mit_afa, R0 + 1_260_000).await;
}

/// Abschreibung groesser als die Anschaffungskosten ergibt keinen Sinn: gesperrt, nicht still auf 0 gekappt.
#[tokio::test]
async fn abschreibung_ueber_den_anschaffungskosten_sperrt_mit_angaben_offen() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_afa", json!(5_000_001));
    rentner_gesperrt("C2", v, ANGABEN_OFFEN).await;
}

/// Preis 100.000, Anschaffung 50.000, Werbungskosten 5.000: Gewinn 45.000 EUR.
#[tokio::test]
async fn werbungskosten_mindern_den_gewinn() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_werbungskosten", json!(500_000));
    rentner("C3", v, R0 + 1_890_000).await;
}

/// Freigrenze Abs. 3 Satz 5: 999,99 EUR frei, 1.000,00 EUR und 1.000,01 EUR steuerpflichtig, und zwar der ganze Gewinn (1.000 EUR
/// in vollen Euro -> 420 EUR Zuschlag).
#[tokio::test]
async fn freigrenze_von_1000_euro_gilt_je_jahr_und_dann_fuer_den_ganzen_gewinn() {
    let mit_gewinn = |ak: i64| verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 6_000_000, ak);
    rentner("D1a", mit_gewinn(5_900_001), R0).await; // 999,99 EUR
    rentner("D1b", mit_gewinn(5_900_000), R0 + 42_000).await; // 1.000,00 EUR
    rentner("D1c", mit_gewinn(5_899_999), R0 + 42_000).await; // 1.000,01 EUR, abgerundet auf 1.000
}

/// Die Freigrenze gilt fuer die Summe aller Verkaeufe des Jahres: 600,00 + 500,00 = 1.100,00 EUR -> 462 EUR Zuschlag.
#[tokio::test]
async fn freigrenze_zaehlt_alle_verkaeufe_des_jahres_zusammen() {
    let mut v = verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 1_000_000, 940_000);
    v.extend(verkauf(2, "grundstueck", "15.06.2020", "11.03.2025", 1_000_000, 950_000));
    rentner("D2", v, R0 + 46_200).await;
}

/// Ein Verlust aus einem zweiten Verkauf mindert den Gewinn des ersten (Satz 7): 50.000 - 10.000 = 40.000 EUR.
#[tokio::test]
async fn verlust_eines_verkaufs_mindert_den_gewinn_des_anderen() {
    let mut v = grundstueck_pflichtig(1);
    v.extend(verkauf(2, "grundstueck", "15.06.2020", "11.03.2025", 4_000_000, 5_000_000));
    rentner("D3", v, R0 + 1_680_000).await;
}

/// Ein Verlust allein bringt keine Erstattung auf andere Einkuenfte: 0 Zuschlag.
#[tokio::test]
async fn ein_verlust_allein_aendert_die_steuer_nicht() {
    let v = verkauf(1, "grundstueck", "15.06.2020", "10.03.2025", 4_000_000, 5_000_000);
    rentner("D4", v, R0).await;
}

/// Ein steuerfreier Verkauf (hier Frist abgelaufen, Gewinn 50.000 EUR) zaehlt nicht zur Freigrenze: der steuerpflichtige Gewinn von
/// 600 EUR bleibt darunter und damit frei.
#[tokio::test]
async fn steuerfreier_verkauf_zaehlt_nicht_zur_freigrenze() {
    let mut v = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    v.extend(verkauf(2, "grundstueck", "15.06.2020", "11.03.2025", 1_000_000, 940_000));
    rentner("D5", v, R0).await;
}

// ---- Sperren: was die Software nicht rechnet, und fehlende Angaben ---------------------------------

#[tokio::test]
async fn kryptowert_sperrt_mit_dem_verkauf() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_veraeusserungs_typ", json!("kryptowert"));
    rentner_gesperrt("E1", v, NICHT_GERECHNET).await;
}

#[tokio::test]
async fn beteiligung_sperrt_mit_dem_verkauf() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_veraeusserungs_typ", json!("beteiligung"));
    rentner_gesperrt("E2", v, NICHT_GERECHNET).await;
}

/// Das Klartext-Ergebnis nennt den Verkauf (der Nutzer erfaehrt, warum nicht gerechnet wird), nicht nur einen Kennwert.
#[tokio::test]
async fn die_sperre_nennt_den_privaten_verkauf_im_klartext() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_veraeusserungs_typ", json!("kryptowert"));
    let a = ergebnis("rentner_gesamt", &akte(kegel_rentner("einzel"), v)).await;
    let text = a.to_string();
    assert!(text.contains("privaten Verkauf"), "{a}");
    assert!(text.contains("Kryptowerte"), "{a}");
}

/// Ehepaar mit Verkauf: nicht gerechnet, auch wenn der Verkauf selbst vollstaendig und steuerpflichtig waere.
#[tokio::test]
async fn zusammenveranlagung_mit_verkauf_sperrt_mit_dem_verkauf() {
    let a = ergebnis(
        "rentner_gesamt",
        &akte(kegel_rentner("zusammen"), grundstueck_pflichtig(1)),
    )
    .await;
    erwarte_sperre("E3", &a, NICHT_GERECHNET);
}

/// Zwei Verkaeufe: der erste ist unvollstaendig, der zweite ein Kryptowert. "Nicht gerechnet" geht vor "Angabe fehlt": ein
/// Nachtragen des Datums hilft dem Nutzer nicht.
#[tokio::test]
async fn nicht_gerechnet_geht_vor_angaben_offen() {
    let mut v = ohne(grundstueck_pflichtig(1), 1, "p23_anschaffungsdatum");
    v.extend(verkauf(2, "kryptowert", "15.06.2020", "10.03.2025", 10_000_000, 5_000_000));
    rentner_gesperrt("E4", v, NICHT_GERECHNET).await;
}

/// Kreuz "ich habe verkauft", aber kein einziger Verkauf eingetragen: eine offene Angabe, kein "kein Verkauf".
#[tokio::test]
async fn bejaht_ohne_einen_verkauf_sperrt_mit_angaben_offen() {
    rentner_gesperrt("F1", Vec::new(), ANGABEN_OFFEN).await;
}

#[tokio::test]
async fn fehlendes_kaufdatum_sperrt_mit_angaben_offen() {
    let v = ohne(grundstueck_pflichtig(1), 1, "p23_anschaffungsdatum");
    rentner_gesperrt("F2", v, ANGABEN_OFFEN).await;
}

#[tokio::test]
async fn fehlendes_verkaufsdatum_sperrt_mit_angaben_offen() {
    let v = ohne(grundstueck_pflichtig(1), 1, "p23_veraeusserungsdatum");
    rentner_gesperrt("F3", v, ANGABEN_OFFEN).await;
}

/// Ein Tag, den es im Kalender nicht gibt (31.02.): die Schreibweise passt, der Tag nicht. Die Rechnung nimmt ihn nicht an.
#[tokio::test]
async fn kalendertag_den_es_nicht_gibt_sperrt_mit_angaben_offen() {
    let kein_tag = mit(grundstueck_pflichtig(1), 1, "p23_veraeusserungsdatum", json!("31.02.2025"));
    rentner_gesperrt("F4", kein_tag, ANGABEN_OFFEN).await;
}

/// Ein Datum in anderer Schreibweise (ISO) lehnt schon das Speichern ab: ELSTER will TT.MM.JJJJ (`checkESt` gibt bei ISO 610001002).
#[tokio::test]
async fn datum_in_iso_schreibweise_wird_beim_speichern_abgewiesen() {
    let d = dienst();
    let kopf = json!({"fall_id": "p23", "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let rumpf = json!({
        "feld_id": "p23_anschaffungsdatum", "wert": "2020-06-15", "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": "ok@iso"},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&rumpf)).await;
    assert_eq!(status, 422, "ISO-Datum muss abgewiesen werden: {antwort}");
    // Kontrolle: dasselbe Feld mit TT.MM.JJJJ wird angenommen.
    let rumpf = json!({
        "feld_id": "p23_anschaffungsdatum", "wert": "15.06.2020", "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": "ok@de"},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&rumpf)).await;
    assert_eq!(status, 201, "TT.MM.JJJJ muss angenommen werden: {antwort}");
}

/// Das Verkaufsdatum liegt im Steuerjahr 2025; ein Verkauf 2024 gehoert in die Erklaerung 2024.
#[tokio::test]
async fn verkauf_ausserhalb_des_steuerjahres_sperrt_mit_angaben_offen() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_veraeusserungsdatum", json!("10.03.2024"));
    rentner_gesperrt("F5", v, ANGABEN_OFFEN).await;
}

/// Verkauf vor dem Kauf: ein Leerverkauf rechnet die Software nicht, sie fragt nach.
#[tokio::test]
async fn verkauf_vor_dem_kauf_sperrt_mit_angaben_offen() {
    let v = verkauf(1, "grundstueck", "20.03.2025", "10.03.2025", 10_000_000, 5_000_000);
    rentner_gesperrt("F6", v, ANGABEN_OFFEN).await;
}

#[tokio::test]
async fn fehlender_preis_sperrt_mit_angaben_offen() {
    let v = ohne(grundstueck_pflichtig(1), 1, "p23_veraeusserungspreis");
    rentner_gesperrt("F7", v, ANGABEN_OFFEN).await;
}

#[tokio::test]
async fn grundstueck_ohne_antwort_zur_eigennutzung_sperrt_mit_angaben_offen() {
    let v = ohne(grundstueck_pflichtig(1), 1, "p23_eigennutzung");
    rentner_gesperrt("F8", v, ANGABEN_OFFEN).await;
}

#[tokio::test]
async fn grundstueck_ohne_antwort_zum_gebaeude_sperrt_mit_angaben_offen() {
    let v = ohne(grundstueck_pflichtig(1), 1, "p23_gebaeude_in_frist");
    rentner_gesperrt("F8b", v, ANGABEN_OFFEN).await;
}

#[tokio::test]
async fn wirtschaftsgut_ohne_antwort_zum_taeglichen_gebrauch_sperrt_mit_angaben_offen() {
    let v = verkauf(1, "anderes_wg", "15.02.2025", "20.08.2025", 10_000_000, 5_000_000);
    let v = ohne(v, 1, "p23_taeglicher_gebrauch");
    rentner_gesperrt("F9", v, ANGABEN_OFFEN).await;
}

/// Ein Verkauf, dessen Preis erst vorlaeufig ist, zaehlt nicht als beantwortet: gesperrt, nicht gerechnet und nicht weggelassen.
#[tokio::test]
async fn vorlaeufiger_verkauf_sperrt_mit_angaben_offen() {
    let a = ergebnis_mit(
        "rentner_gesamt",
        &akte(kegel_rentner("einzel"), grundstueck_pflichtig(1)),
        &["p23_veraeusserungspreis"],
    )
    .await;
    erwarte_sperre("F10", &a, ANGABEN_OFFEN);
}

/// Der zweite Verkauf ist unvollstaendig, der erste vollstaendig und steuerpflichtig: die Zahl darf den ersten nicht allein tragen.
#[tokio::test]
async fn ein_unvollstaendiger_zweiter_verkauf_sperrt_den_ganzen_fall() {
    let mut v = grundstueck_pflichtig(1);
    v.extend(ohne(
        verkauf(2, "grundstueck", "15.06.2020", "11.03.2025", 10_000_000, 5_000_000),
        2,
        "p23_veraeusserungsdatum",
    ));
    rentner_gesperrt("F11", v, ANGABEN_OFFEN).await;
}

// ---- die Scheibe `gesamt` --------------------------------------------------------------------------

#[tokio::test]
async fn gesamt_rechnet_den_steuerpflichtigen_verkauf() {
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), grundstueck_pflichtig(1))).await;
    erwarte_zahl("H1", &a, G0 + PLUS_50_000);
}

#[tokio::test]
async fn gesamt_laesst_den_steuerfreien_verkauf_weg() {
    let v = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), v)).await;
    erwarte_zahl("H2", &a, G0);
}

/// Die Vorschau zeigt, was die Abgabe schreibt: ein Verkauf mit abgelaufener Frist steht nicht darin (die Anleitung zur Anlage SO erfasst
/// nur Verkaeufe innerhalb der Frist), der steuerpflichtige daneben schon. Der selbst bewohnte Verkauf innerhalb der Frist steht mit der
/// Zeile 36 (Haken, Zeitraum, Flaeche) und dem Gewinn 0 darin; ein selbst bewohnter nach Ablauf der Frist nicht.
#[tokio::test]
async fn die_vorschau_zeigt_den_verkauf_ausserhalb_der_erklaerung_nicht() {
    let frei = mit(
        verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000),
        1,
        "p23_bezeichnung",
        json!("Freier Verkauf"),
    );
    let mut v = frei;
    v.extend(mit(grundstueck_pflichtig(2), 2, "p23_bezeichnung", json!("Steuerpflichtiger Verkauf")));
    v.extend(mit(bewohnt(3, "ausschliesslich"), 3, "p23_bezeichnung", json!("Selbst bewohnter Verkauf")));
    // Selbst bewohnt, aber vor mehr als zehn Jahren gekauft: nicht in der Erklaerung, und ohne Flaeche und Zeitraum.
    let alt_bewohnt = mit(verkauf(4, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000), 4, "p23_eigennutzung", json!("ausschliesslich"));
    v.extend(mit(alt_bewohnt, 4, "p23_bezeichnung", json!("Alter bewohnter Verkauf")));
    let d = vorschau("gesamt", &akte(kegel_gesamt(), v)).await;
    let text = d["anlage_instanzen"].to_string();
    assert!(text.contains("Steuerpflichtiger Verkauf") && text.contains("E0306801"), "der steuerpflichtige Verkauf fehlt: {text}");
    assert!(text.contains("Selbst bewohnter Verkauf"), "der selbst bewohnte Verkauf fehlt in der Vorschau: {text}");
    for kz in ["E0306301", "E0306302", "E0306303"] {
        assert_eq!(text.matches(kz).count(), 1, "{kz} steht genau einmal (nur der selbst bewohnte Verkauf innerhalb der Frist): {text}");
    }
    assert!(!text.contains("Freier Verkauf"), "der Verkauf mit abgelaufener Frist steht in der Vorschau: {text}");
    assert!(!text.contains("Alter bewohnter Verkauf"), "der selbst bewohnte Verkauf nach Ablauf der Frist steht in der Vorschau: {text}");
    assert_eq!(text.matches("E0306801").count(), 2, "genau zwei Gewinn-Kz (steuerpflichtig und selbst bewohnt): {text}");
}

#[tokio::test]
async fn gesamt_sperrt_das_alte_grundstueck_mit_neuem_gebaeude() {
    let v = verkauf(1, "grundstueck", "15.06.2014", "10.03.2025", 10_000_000, 5_000_000);
    let v = mit(v, 1, "p23_gebaeude_in_frist", json!(true));
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), v)).await;
    erwarte_sperre("H2b", &a, NICHT_GERECHNET);
}

#[tokio::test]
async fn gesamt_sperrt_den_kryptoverkauf_mit_dem_verkauf() {
    let v = mit(grundstueck_pflichtig(1), 1, "p23_veraeusserungs_typ", json!("kryptowert"));
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), v)).await;
    erwarte_sperre("H3", &a, NICHT_GERECHNET);
}

#[tokio::test]
async fn gesamt_sperrt_den_unvollstaendigen_verkauf() {
    let v = ohne(grundstueck_pflichtig(1), 1, "p23_veraeusserungsdatum");
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), v)).await;
    erwarte_sperre("H4", &a, ANGABEN_OFFEN);
}

/// Der Verkauf steht nur unter `__2` (Zaehl-Instanz 2, nichts unter dem Basisnamen): er wird gefunden und gerechnet.
#[tokio::test]
async fn gesamt_findet_den_verkauf_in_der_zweiten_zaehl_instanz() {
    let a = ergebnis("gesamt", &akte(kegel_gesamt(), grundstueck_pflichtig(2))).await;
    erwarte_zahl("H5", &a, G0 + PLUS_50_000);
}

/// Der Wächter aus `p23_waechter_hermetisch.rs` bleibt: Verkauf eingetragen, Kreuz "kein Verkauf" -> `flag_konsistenz_offen`, keine Zahl.
#[tokio::test]
async fn kreuz_kein_verkauf_trotz_eingetragenem_verkauf_sperrt_weiter_mit_dem_flag() {
    let mut akte = kegel_rentner("einzel");
    akte.push(p("kein_p23_verkauf", json!(true)));
    akte.extend(grundstueck_pflichtig(1));
    let a = ergebnis("rentner_gesamt", &akte).await;
    erwarte_sperre("K1", &a, "flag_konsistenz_offen");
}

/// Ein Minus in einem Betragsfeld des Verkaufs nimmt schon das Speichern nicht an (`nicht_negativ`, 422 `fail-closed (Vorzeichen)`): ein
/// negativer Preis senkte den Gewinn, negative Kosten, negative Werbungskosten und eine negative Abschreibung hoben ihn, ohne dass der
/// Nutzer es merkt. Die 0 und ein positiver Betrag gehen durch; das gilt auch fuer die Instanz `__2`. Die Pruefung im Bescheid
/// (`lese()`) bleibt als zweite Sicherung fuer geladene Akten (`privatverkauf_einreichung_hermetisch.rs`, `rentner_ehegatte_hermetisch.rs`).
#[tokio::test]
async fn ein_minus_in_den_betragsfeldern_wird_beim_speichern_abgewiesen() {
    let d = dienst();
    let kopf = json!({"fall_id": "p23", "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let setze = |feld: &'static str, wert: i64| {
        json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}{wert}")},
            "ts": "2026-01-01T00:00:00+00:00",
        })
    };
    // Je Feld ein Minus (abgewiesen) und ein Wert, der durchgeht: 5.000.000 Cent bei der ersten, 0 bei der zweiten Instanz.
    for (feld, ok) in [
        ("p23_afa", 5_000_000),
        ("p23_afa__2", 0),
        ("p23_veraeusserungspreis", 10_000_000),
        ("p23_anschaffung_herstellungskosten", 5_000_000),
        ("p23_werbungskosten", 500_000),
        ("p23_veraeusserungspreis__2", 0),
    ] {
        let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&setze(feld, -5_000_000))).await;
        assert_eq!(status, 422, "{feld}: ein Minus muss abgewiesen werden: {antwort}");
        assert!(antwort.to_string().contains("Vorzeichen"), "{feld}: {antwort}");
        let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&setze(feld, ok))).await;
        assert_eq!(status, 201, "{feld}={ok} muss angenommen werden: {antwort}");
    }
}

/// Die Wohnflaeche (Zeile 36) nimmt nur ganze Quadratmeter von 1 bis 100.000 an; die 0 bleibt zulaessig ("nichts anzugeben", `lese()`
/// sperrt sie bei einem selbst bewohnten Verkauf innerhalb der Frist). Ein Minus, ein Bruch und mehr als 100.000 weist das Speichern ab.
#[tokio::test]
async fn die_wohnflaeche_nimmt_nur_ganze_quadratmeter_im_bereich_an() {
    let d = dienst();
    let kopf = json!({"fall_id": "p23", "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let setze = |feld: &'static str, wert: Value| {
        json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}{wert}")},
            "ts": "2026-01-01T00:00:00+00:00",
        })
    };
    // Ein angenommener Wert belegt sein Feld (ein zweites Event braucht `ersetzt`): jeder bekommt eine eigene Instanz.
    for (feld, wert, erwartet) in [
        ("p23_wohnflaeche", json!(-1), 422),
        ("p23_wohnflaeche", json!(100_001), 422),
        ("p23_wohnflaeche", json!(1.5), 422),
        ("p23_wohnflaeche", json!(0), 201),
        ("p23_wohnflaeche__2", json!(120), 201),
        ("p23_wohnflaeche__3", json!(100_000), 201),
    ] {
        let (status, antwort) = sende(&d, "POST", "/fall/p23/event", Some(&setze(feld, wert.clone()))).await;
        assert_eq!(status, erwartet, "{feld} = {wert}: {antwort}");
    }
}

// ---- Fragetexte -------------------------------------------------------------------------------------

/// Was der Nutzer liest (`GET /feld/{fid}/frage`): in der Scheibe `gesamt` zaehlt das Kreuz `kein_sonstige` den Verkauf nicht mehr
/// mit (sonst sperrte sein "Ja" mit `einkunftsart_nicht_ring_faehig`, bevor die Frage zum Verkauf kommt), `kein_p23_verkauf` fragt
/// nach ihm, und die neuen Felder fragen im Du.
#[tokio::test]
async fn die_fragetexte_trennen_den_verkauf_von_den_sonstigen_einkuenften() {
    let d = dienst();
    let kopf = json!({"fall_id": "p23", "scheibe": "gesamt", "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    let text = |q: &Value| format!("{} {}", q["fragetext_laie"].as_str().unwrap(), q["hilfe_kurz"].as_str().unwrap());
    let mut texte = std::collections::BTreeMap::new();
    for fid in [
        "kein_sonstige",
        "kein_p23_verkauf",
        "p23_veraeusserungs_typ",
        "p23_eigennutzung",
        "p23_afa",
        "p23_anschaffungsdatum",
        "p23_gebaeude_in_frist",
        "p23_wohnflaeche",
        "p23_eigennutzung_von",
        "p23_eigennutzung_bis",
    ] {
        let (status, q) = sende(&d, "GET", &format!("/fall/p23/feld/{fid}/frage"), None).await;
        assert_eq!(status, 200, "{fid}: {q}");
        texte.insert(fid, text(&q["frage"]));
    }
    let sonstige = &texte["kein_sonstige"];
    assert!(
        !sonstige.contains("Jahre nach dem Kauf") && !sonstige.contains("Jahr nach dem Kauf"),
        "kein_sonstige zaehlt den Verkauf mit seiner Frist mit: {sonstige}"
    );
    assert!(sonstige.contains("zählst du hier nicht mit"), "kein_sonstige sagt nicht, dass der Verkauf getrennt gefragt wird: {sonstige}");
    assert!(texte["kein_p23_verkauf"].contains("verkauft"), "{}", texte["kein_p23_verkauf"]);
    assert!(texte["p23_veraeusserungs_typ"].contains("Kryptowert"), "{}", texte["p23_veraeusserungs_typ"]);
    assert!(texte["p23_eigennutzung"].contains("selbst bewohnt"), "{}", texte["p23_eigennutzung"]);
    assert!(texte["p23_anschaffungsdatum"].contains("Erb"), "Erbfall nicht erklaert: {}", texte["p23_anschaffungsdatum"]);
    assert!(
        texte["p23_gebaeude_in_frist"].contains("Gebäude") && texte["p23_gebaeude_in_frist"].contains("zehn Jahren"),
        "{}",
        texte["p23_gebaeude_in_frist"]
    );
    // Zeile 36: Wohnflaeche in Quadratmetern, Zeitraum von-bis; der Erbe zaehlt ab dem eigenen Einzug.
    assert!(texte["p23_wohnflaeche"].contains("Quadratmeter"), "{}", texte["p23_wohnflaeche"]);
    assert!(texte["p23_eigennutzung_von"].contains("Einzug"), "{}", texte["p23_eigennutzung_von"]);
    assert!(texte["p23_eigennutzung_bis"].contains("Übergabe"), "{}", texte["p23_eigennutzung_bis"]);
    // Die Anleitung Anlage SO nennt drei Quellen der abgezogenen Abschreibung und die Anschaffung nach 2008.
    assert!(
        texte["p23_afa"].contains("Arbeitnehmer") && texte["p23_afa"].contains("Kapitalerträgen") && texte["p23_afa"].contains("31.12.2008"),
        "{}",
        texte["p23_afa"]
    );
    for (fid, t) in &texte {
        assert!(!t.contains("Sie ") && !t.contains(" Ihre"), "{fid} siezt: {t}");
    }
}
