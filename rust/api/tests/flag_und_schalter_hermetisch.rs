//! `FLAG_NEGIERT`, `fremd_arten` und die Bool-Schalter der Scheiben-`Cfg` im Standardlauf (ohne `PARITY=1`,
//! ohne Python), je Fall ueber `GET /ergebnis`.
//!
//! Fortsetzung von `p23_waechter_hermetisch.rs` (dort `kein_p23_verkauf`) fuer den Rest derselben Tabellen:
//!
//! - `rust/konsistenz/src/flag.rs`, `FLAG_NEGIERT`: je Flag der Schluessel und jedes Betragsfeld der
//!   Basenliste (`kein_kap`, `kein_vuv`, `kein_sonstige`, `kein_kap_partner`, `kein_gewinn`);
//! - `rust/bescheid/src/deklaration.rs`, `Cfg::fuer`: `fremd_arten` (`kein_sonstige` auf `gesamt`,
//!   `kein_vuv` auf `rentner_gesamt`) sowie die Schalter `guard`, `gesamt_guard`, `partner_19`, `rentner`
//!   und die Optionen `multi_objekt`, `multi_rente`.
//!
//! WARUM: ihre Gegenprobe lief bisher nur im Differenz-Harness (`rust/parity/`) und damit nur mit
//! `PARITY=1`; die CI faehrt Parity nicht. Das Mutationsprotokoll (je Mutant `cargo test --workspace
//! --exclude parity --no-fail-fast`, 2026-10-03) steht im Bericht `flag-negiert-alle`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl und jeder Grund unten ist die Ausgabe des Python-Servers
//! (`server.py`, HEAD 4a2f6ea4, 2026-10-03, Sonde `sonde_fna.py`, dieselben Events ueber
//! `POST /fall/{id}/event`). Kein Wert ist aus dem Rust-Code abgelesen. Die Baselines (`G0`, `R0`, `A0`, `GZ0`,
//! `RZ0`) sind die Positivkontrolle: Kegel vollstaendig, Engine liefert eine Zahl; ohne sie waere "gesperrt"
//! nicht von "Kegel offen" zu unterscheiden. Die Fehlermeldung rechnet die Differenz gegen die Baseline aus.
//!
//! ponytail: die Erwartungswerte sind eingefroren. Faellt Python als Orakel weg und aendern sich Tarif oder
//! Kegel, rechnet man die Baselines von Hand nach und zieht die Konstanten nach.
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

/// `ep` mit vollem Kegel (der Ring liefert 0 EUR).
const EP0: i64 = 0;
/// 10.000,00 EUR in Cent: der Betrag, den ein Flag-Widerspruch traegt.
const B: i64 = 1_000_000;
/// Marke der Abweisungs-Faelle: `POST /event` weist das Feld mit 400 ab, weil es nicht zur Scheibe
/// gehoert. Zahl und Grund sind in der Tabelle durch diese Marke ersetzt.
const AW: i64 = -1;
/// `gesamt` einzel: AN 200.000 EUR, alle Kreuze "nein" (Positivkontrolle).
const G0: i64 = 7_255_600;
/// `rentner_gesamt` einzel: gesetzliche Rente 20.000 EUR ab 2025.
const R0: i64 = 5_917_000;
/// `an_gesamt` einzel: AN 40.000 EUR.
const A0: i64 = 662_900;
/// `gesamt` zusammen (Partner nur Nullen).
const GZ0: i64 = 6_162_800;
/// `rentner_gesamt` zusammen (Partner nur KAP-Nullen).
const RZ0: i64 = 4_824_200;

type Paare = Vec<(&'static str, Value)>;

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

/// Legt einen Fall der Scheibe an, schreibt jedes Paar ueber die echte Route `POST /event` (Nutzer-Klick,
/// bestaetigt) und liefert `GET /ergebnis`.
async fn ergebnis(scheibe: &str, paare: &[(&'static str, Value)]) -> Value {
    let d = dienst();
    let kopf = json!({"fall_id": "fna", "scheibe": scheibe, "veranlagungszeitraum": 2025});
    let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
    for (feld, wert) in paare {
        let rumpf = json!({
            "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
            "ts": "2026-01-01T00:00:00+00:00",
        });
        let (status, antwort) = sende(&d, "POST", "/fall/fna/event", Some(&rumpf)).await;
        assert_eq!(status, 201, "POST /event {feld}: {antwort}");
    }
    let (status, antwort) = sende(&d, "GET", "/fall/fna/ergebnis", None).await;
    assert_eq!(status, 200, "GET /ergebnis: {antwort}");
    antwort
}

/// Ersetzt den Wert von `feld`; fehlt es, haengt es an.
fn mit(mut paare: Paare, feld: &'static str, wert: Value) -> Paare {
    match paare.iter_mut().find(|(f, _)| *f == feld) {
        Some(p) => p.1 = wert,
        None => paare.push((feld, wert)),
    }
    paare
}

fn ohne(mut paare: Paare, feld: &str) -> Paare {
    paare.retain(|(f, _)| *f != feld);
    paare
}

fn plus(mut paare: Paare, mehr: Paare) -> Paare {
    paare.extend(mehr);
    paare
}

/// Pflicht-Kegel `gesamt` (35 Felder) samt `kein_sonstige = true` und `kein_p23_verkauf = true`:
/// AN 200.000 EUR, einzel, alle Kreuze "nein". Aus `tests/_kegel.py::kegel_fuer` (2026-10-03).
fn gesamt() -> Paare {
    vec![
        ("vv_einnahmen", json!(0)),
        ("vv_gebaeude_afa", json!(0)),
        ("vv_schuldzinsen", json!(0)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(0)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(20_000_000)),
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
        ("kein_p23_verkauf", json!(true)),
    ]
}

/// `gesamt` zusammen mit dem Partner-Kegel (`bruttoarbeitslohn_partner` und die fuenf KAP-Felder, Nullen).
fn gesamt_zusammen() -> Paare {
    plus(
        mit(gesamt(), "veranlagung", json!("zusammen")),
        vec![
            ("bruttoarbeitslohn_partner", json!(0)),
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ],
    )
}

/// Pflicht-Kegel `rentner_gesamt` (28 Felder) samt `kein_p23_verkauf = true`: Rente 20.000 EUR ab 2025,
/// `kein_sonstige = false` (die eigene Rente ist die ehrliche Antwort), einzel.
fn rentner() -> Paare {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(20_000_000)),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(0)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(0)),
        ("rentner_gepflegter_hilflos", json!(false)),
        ("rentner_hinterbliebenenbezuege", json!(false)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
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
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
        ("kein_p23_verkauf", json!(true)),
    ]
}

/// `rentner_gesamt` zusammen; `bruttoarbeitslohn_partner` kennt die Scheibe seit Abweichung Nr. 33, aber nicht im
/// Kegel: ihr Partner-Kegel sind die fuenf KAP-Felder (Nullen).
fn rentner_zusammen() -> Paare {
    plus(
        mit(rentner(), "veranlagung", json!("zusammen")),
        partner_kap(),
    )
}

/// Pflicht-Kegel `an_gesamt` (33 Felder): AN 40.000 EUR, einzel, alle vier Kreuze "nein".
fn an() -> Paare {
    vec![
        ("bruttoarbeitslohn", json!(4_000_000)),
        ("veranlagung", json!("einzel")),
        ("ep_arbeitstage", json!(220)),
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
        ("dhf_monate", json!(0)),
        ("dhf_im_inland", json!(false)),
        ("dhf_beruflich_veranlasst", json!(false)),
        ("dhf_eigener_hausstand", json!(false)),
        ("dhf_finanzielle_beteiligung", json!(false)),
        ("tage_24h", json!(0)),
        ("tage_an_abreise", json!(0)),
        ("tage_ueber_8h_eintaegig", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("fam_anzahl_kinder", json!(0)),
        ("verlustvortrag_bestand", json!(0)),
    ]
}

/// Der Partner-Kegel von `rentner_gesamt`: nur die fuenf KAP-Felder (Nullen); `bruttoarbeitslohn_partner` gehoert
/// seit Abweichung Nr. 33 zur Scheibe, aber nicht zum Kegel.
fn partner_kap() -> Paare {
    vec![
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ]
}

/// Der Partner-Kegel von `an_gesamt` bei Zusammenveranlagung: nur `bruttoarbeitslohn_partner`
/// (`AN_GESAMT_PARTNER` in `bescheid/src/deklaration/konstanten.rs:148-150`).
fn partner_kegel_an() -> Paare {
    vec![("bruttoarbeitslohn_partner", json!(0))]
}

/// Partner-Rente 15.000 EUR mit allen vier Kernfeldern (`RENTNER_22_PARTNER`), ohne das Kreuz
/// `kein_sonstige_partner` (es steht nur auf `gesamt`).
fn partner_rente_voll() -> Paare {
    vec![
        ("rentner_jahresrente_partner", json!(1_500_000)),
        ("rentner_renten_art_partner", json!("gesetzliche_rente")),
        ("rentner_renten_beginn_jahr_partner", json!(2025)),
        ("rentner_alter_bei_rentenbeginn_partner", json!(65)),
        ("rentner_rentenfreibetrag_partner", json!(0)),
    ]
}

/// Der Pflicht-Kegel von `ep` (4 Felder) mit Abwesenheitswerten.
fn ep_kegel() -> Paare {
    vec![
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
    ]
}

/// Ein leerer Fall (nur die Scheibe, keine Angaben).
fn leer() -> Paare {
    Vec::new()
}

/// Die zwei Gate-Felder von § 16 Abs. 4 `EStG`, beide bestaetigt: ohne sie sperrt `p16_4_gate_offen` VOR dem
/// Flag-Vergleich, sobald `rentner_veraeusserungsgewinn > 0` ist.
fn p16_gate() -> Paare {
    vec![
        ("rentner_alter_55_oder_berufsunfaehig", json!(false)),
        ("rentner_freibetrag_erstmalig", json!(false)),
    ]
}

/// Ein Event ueber die echte Route `POST /event`; liefert Status und Antwort.
async fn event_post(d: &Dienst, feld: &str, wert: &Value) -> (u16, Value) {
    let rumpf = json!({
        "feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:laie",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00",
    });
    sende(d, "POST", "/fall/fna/event", Some(&rumpf)).await
}

/// Ein Fall: Events rein, `GET /ergebnis` raus. Meldet bei Abweichung die Differenz gegen die Baseline
/// (ein Sperr-Test, der eine Zahl bekommt, hat den ungeprueft besteuerten Betrag in der Zahl).
async fn pruefe(
    name: &str,
    szene: &str,
    events: &[(&'static str, Value)],
    basis: i64,
    zahl: Option<i64>,
    grund: &str,
) {
    if grund == "AW" {
        let d = dienst();
        let kopf = json!({"fall_id": "fna", "scheibe": szene, "veranlagungszeitraum": 2025});
        let (status, antwort) = sende(&d, "POST", "/fall", Some(&kopf)).await;
        assert_eq!(status, 201, "POST /fall: {antwort}");
        let mut abgewiesen: Option<String> = None;
        for (feld, wert) in events {
            let (st, a) = event_post(&d, feld, wert).await;
            if st != 201 {
                assert_eq!(st, 400, "{feld}: erwarteter 400, erhalten {a}");
                abgewiesen = Some(a["fehler"].as_str().unwrap_or_default().to_owned());
                break;
            }
        }
        assert!(
            abgewiesen.is_some(),
            "{name} ({szene}): Erwartet (Python 3fabb4ed, Sonde): ein Event wird mit 400 abgewiesen;              alle Events wurden angenommen."
        );
        assert!(
            abgewiesen
                .as_deref()
                .unwrap_or_default()
                .contains("nicht in dieser Scheibe"),
            "{name} ({szene}): Wortlaut der 400: {abgewiesen:?}"
        );
        return;
    }
    let a = ergebnis(szene, events).await;
    let (g, z) = (a["grund"].as_str(), a["zahl_cent"].as_i64());
    assert!(
        g == Some(grund) && z == zahl,
        // (Abweisungs-Faelle laufen im AW-Zweig oben nie hierher.)
        "{name} ({szene}): erwartet grund={grund:?} zahl_cent={zahl:?}; erhalten grund={g:?} zahl_cent={z:?} \
         (Differenz gegen die Baseline {basis}: {:?} Cent). Antwort: {a}",
        z.map(|v| v - basis)
    );
}

/// Je Zeile ein Fall: `name => Scheibe, Events, Baseline, erwartete Zahl, erwarteter Grund;`
macro_rules! faelle {
    ($($name:ident => $szene:expr, $events:expr, $basis:expr, $zahl:expr, $grund:expr;)+) => {
        $(
            #[tokio::test]
            async fn $name() {
                pruefe(stringify!($name), $szene, &$events, $basis, $zahl, $grund).await;
            }
        )+
    };
}

/// Der Pflicht-Kegel von `ep`, nur die ersten drei Felder (die Luecke selbst).
fn ep_kegel_erste_drei() -> Paare {
    ep_kegel().into_iter().take(3).collect()
}

/// Der Pflicht-Kegel von `an_gesamt`, nur die ersten 20 Felder (die Luecke selbst).
fn an_erste_zwanzig() -> Paare {
    an().into_iter().take(20).collect()
}

faelle! {
    g0_baseline => "gesamt", gesamt(), G0, Some(7_255_600), "bestaetigt";
    r0_baseline => "rentner_gesamt", rentner(), R0, Some(5_917_000), "bestaetigt";
    a0_baseline => "an_gesamt", an(), A0, Some(662_900), "bestaetigt";
    gz0_baseline => "gesamt", gesamt_zusammen(), GZ0, Some(6_162_800), "bestaetigt";
    rz0_baseline => "rentner_gesamt", rentner_zusammen(), RZ0, Some(4_824_200), "bestaetigt";
    g_kein_kap_kap_kapitalertraege => "gesamt", mit(gesamt(), "kap_kapitalertraege", json!(B)), G0, None, "flag_konsistenz_offen";
    r_kein_kap_kap_kapitalertraege => "rentner_gesamt", mit(rentner(), "kap_kapitalertraege", json!(B)), R0, None, "flag_konsistenz_offen";
    g_kein_kap_kap_gewinn_aktien => "gesamt", mit(gesamt(), "kap_gewinn_aktien", json!(B)), G0, None, "flag_konsistenz_offen";
    r_kein_kap_kap_gewinn_aktien => "rentner_gesamt", mit(rentner(), "kap_gewinn_aktien", json!(B)), R0, None, "flag_konsistenz_offen";
    g_kein_kap_kap_verlust_aktien => "gesamt", mit(gesamt(), "kap_verlust_aktien", json!(B)), G0, None, "flag_konsistenz_offen";
    r_kein_kap_kap_verlust_aktien => "rentner_gesamt", mit(rentner(), "kap_verlust_aktien", json!(B)), R0, None, "flag_konsistenz_offen";
    g_kein_kap_kap_gewinn_sonstige => "gesamt", mit(gesamt(), "kap_gewinn_sonstige", json!(B)), G0, None, "flag_konsistenz_offen";
    r_kein_kap_kap_gewinn_sonstige => "rentner_gesamt", mit(rentner(), "kap_gewinn_sonstige", json!(B)), R0, None, "flag_konsistenz_offen";
    g_kein_kap_kap_verlust_sonstige => "gesamt", mit(gesamt(), "kap_verlust_sonstige", json!(B)), G0, None, "flag_konsistenz_offen";
    r_kein_kap_kap_verlust_sonstige => "rentner_gesamt", mit(rentner(), "kap_verlust_sonstige", json!(B)), R0, None, "flag_konsistenz_offen";
    g_kein_vuv_vv_einnahmen => "gesamt", mit(gesamt(), "vv_einnahmen", json!(B)), G0, None, "flag_konsistenz_offen";
    r_kein_sonstige_rentner_jahresrente => "rentner_gesamt", mit(rentner(), "kein_sonstige", json!(true)), R0, None, "flag_konsistenz_offen";
    gz_kein_kap_partner_kap_kapitalertraege_partner => "gesamt", mit(gesamt_zusammen(), "kap_kapitalertraege_partner", json!(B)), GZ0, None, "flag_konsistenz_offen";
    gz_kein_kap_partner_kap_gewinn_aktien_partner => "gesamt", mit(gesamt_zusammen(), "kap_gewinn_aktien_partner", json!(B)), GZ0, None, "flag_konsistenz_offen";
    gz_kein_kap_partner_kap_verlust_aktien_partner => "gesamt", mit(gesamt_zusammen(), "kap_verlust_aktien_partner", json!(B)), GZ0, None, "flag_konsistenz_offen";
    gz_kein_kap_partner_kap_gewinn_sonstige_partner => "gesamt", mit(gesamt_zusammen(), "kap_gewinn_sonstige_partner", json!(B)), GZ0, None, "flag_konsistenz_offen";
    gz_kein_kap_partner_kap_verlust_sonstige_partner => "gesamt", mit(gesamt_zusammen(), "kap_verlust_sonstige_partner", json!(B)), GZ0, None, "flag_konsistenz_offen";
    gz_kein_kap_partner_nie_gefragt => "gesamt", ohne(mit(gesamt_zusammen(), "kap_kapitalertraege_partner", json!(B)), "kein_kap_partner"), GZ0, None, "flag_konsistenz_offen";
    g_kein_gewinn_einkuenfte_gewinn => "gesamt", mit(gesamt(), "einkuenfte_gewinn", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_betriebseinnahmen => "gesamt", mit(gesamt(), "betriebseinnahmen", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_sonstige_betriebsausgaben => "gesamt", mit(gesamt(), "sonstige_betriebsausgaben", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_afa_jahresbetrag => "gesamt", mit(gesamt(), "afa_jahresbetrag", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_gwg_anschaffungskosten_netto => "gesamt", mit(gesamt(), "gwg_anschaffungskosten_netto", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_gewinnanteil => "gesamt", mit(gesamt(), "gewinnanteil", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_verguetung_taetigkeit => "gesamt", mit(gesamt(), "verguetung_taetigkeit", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_verguetung_darlehen => "gesamt", mit(gesamt(), "verguetung_darlehen", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_verguetung_ueberlassung => "gesamt", mit(gesamt(), "verguetung_ueberlassung", json!(B)), G0, None, "flag_konsistenz_offen";
    g_kein_gewinn_rentner_veraeusserungsgewinn => "gesamt", plus(mit(gesamt(), "rentner_veraeusserungsgewinn", json!(B)), p16_gate()), G0, None, "flag_konsistenz_offen";
    r_kein_gewinn_rentner_veraeusserungsgewinn => "rentner_gesamt", plus(mit(rentner(), "rentner_veraeusserungsgewinn", json!(B)), p16_gate()), R0, None, "flag_konsistenz_offen";
    g_fremd_kein_sonstige_false => "gesamt", mit(gesamt(), "kein_sonstige", json!(false)), G0, None, "einkunftsart_nicht_ring_faehig";
    r_fremd_kein_vuv_false => "rentner_gesamt", mit(rentner(), "kein_vuv", json!(false)), R0, None, "einkunftsart_nicht_ring_faehig";
    a_guard_hebt_sperren_auf => "an_gesamt", mit(an(), "kein_gewinn", json!(false)), A0, None, "einkunftsart_nicht_ring_faehig";
    a_verlustvortrag_luecke => "an_gesamt", mit(an(), "verlustvortrag_bestand", json!(B)), A0, None, "verlustvortrag_gehoert_in_gesamt";
    a_partner_kegel_zusammen => "an_gesamt", plus(mit(an(), "veranlagung", json!("zusammen")), partner_kegel_an()), A0, Some(272_800), "bestaetigt";
    a_partner_kegel_fehlt => "an_gesamt", mit(an(), "veranlagung", json!("zusammen")), A0, None, "partner_kegel_offen";
    gz_partner_kap_feld_fehlt => "gesamt", ohne(gesamt_zusammen(), "kap_gewinn_aktien_partner"), GZ0, None, "partner_kegel_offen";
    r_zusammen_ohne_partner_kap_rechnet => "rentner_gesamt", mit(rentner(), "veranlagung", json!("zusammen")), R0, Some(4_824_200), "bestaetigt";
    r_partner_rente_unvollstaendig => "rentner_gesamt", mit(rentner_zusammen(), "rentner_jahresrente_partner", json!(1_500_000)), RZ0, None, "rente_instanz_offen";
    g_zweites_vv_objekt_unvollstaendig => "gesamt", plus(mit(mit(gesamt(), "vv_einnahmen", json!(B)), "kein_vuv", json!(false)), vec![("vv_einnahmen__2", json!(B))]), G0, None, "vv_instanz_offen";
    r_rentenbeginn_jahr_ungueltig => "rentner_gesamt", mit(rentner(), "rentner_renten_beginn_jahr", json!(0)), R0, None, "rentenbeginn_jahr_ungueltig";
    r_rentenbeginn_nach_vz => "rentner_gesamt", mit(rentner(), "rentner_renten_beginn_jahr", json!(2030)), R0, None, "rentenbeginn_nach_vz";
    kv1_gesamt_kv_partner_ohne_versicherung_rechnet => "gesamt", plus(gesamt_zusammen(), vec![("basis_kv_partner", json!(100_000))]), GZ0, Some(6_123_800), "bestaetigt";
    kv8_gesamt_kv_partner_mit_versicherung_rechnet => "gesamt", plus(gesamt_zusammen(), vec![("basis_kv_partner", json!(100_000)), ("versicherungsart_partner", json!("gesetzlich_an"))]), GZ0, Some(6_123_800), "bestaetigt";
    kv3_gesamt_partner_pv_ohne_versicherung_rechnet => "gesamt", plus(gesamt_zusammen(), vec![("basis_pv_partner", json!(100_000))]), GZ0, Some(6_123_800), "bestaetigt";
    kv4_rentner_ohne_kv_partner_rechnet => "rentner_gesamt", rentner_zusammen(), RZ0, Some(4_824_200), "bestaetigt";
    kv5_rentner_kv_partner_ohne_versicherung_sperrt => "rentner_gesamt", plus(rentner_zusammen(), vec![("basis_kv_partner", json!(100_000))]), RZ0, None, "partner_kegel_offen";
    kv7_rentner_kv_partner_mit_versicherung_rechnet => "rentner_gesamt", plus(rentner_zusammen(), vec![("basis_kv_partner", json!(100_000)), ("versicherungsart_partner", json!("gesetzlich_an"))]), RZ0, Some(4_785_200), "bestaetigt";
    pr0_rentner_partnerkern_unvollstaendig_sperrt => "rentner_gesamt", plus(rentner_zusammen(), vec![("rentner_renten_art_partner", json!("gesetzliche_rente"))]), RZ0, None, "rente_instanz_offen";
    pr1_rentner_partnerkern_drei_von_vier_sperrt => "rentner_gesamt", plus(rentner_zusammen(), vec![("rentner_renten_art_partner", json!("gesetzliche_rente")), ("rentner_renten_beginn_jahr_partner", json!(2025)), ("rentner_alter_bei_rentenbeginn_partner", json!(65)), ("rentner_rentenfreibetrag_partner", json!(0))]), RZ0, None, "rente_instanz_offen";
    pr2_rentner_partnerrente_vollstaendig_rechnet => "rentner_gesamt", plus(rentner_zusammen(), vec![("rentner_renten_art_partner", json!("gesetzliche_rente")), ("rentner_renten_beginn_jahr_partner", json!(2025)), ("rentner_alter_bei_rentenbeginn_partner", json!(65)), ("rentner_rentenfreibetrag_partner", json!(0)), ("rentner_jahresrente_partner", json!(1_500_000))]), RZ0, Some(5_346_000), "bestaetigt";
    r_partnerrente_vollstaendig_kreuz_nie_gefragt => "rentner_gesamt", plus(mit(rentner(), "veranlagung", json!("zusammen")), partner_rente_voll()), RZ0, Some(5_346_000), "bestaetigt";
    ri1_rentner_zweite_rente_ohne_kern_sperrt => "rentner_gesamt", plus(mit(mit(rentner(), "veranlagung", json!("zusammen")), "versicherungsart_partner", json!("gesetzlich_an")), vec![("rentner_jahresrente__2", json!(B))]), RZ0, None, "rente_instanz_offen";
    ri2_rentner_einzel_zweite_rente_sperrt => "rentner_gesamt", plus(rentner(), vec![("rentner_jahresrente__2", json!(B))]), R0, None, "rente_instanz_offen";
    ep1_ep_voller_kegel_rechnet => "ep", ep_kegel(), EP0, Some(0), "bestaetigt";
    ep2_ep_kegel_luecke_sperrt_kegel => "ep", ep_kegel_erste_drei(), EP0, None, "input_kegel_nicht_bestaetigt";
    nv1_leer_ohne_gesamtbescheid => "n_vor_gwg", leer(), EP0, None, "kein_scheiben_gesamtbescheid";
    an1_an_kegel_luecke_sperrt_kegel => "an_gesamt", an_erste_zwanzig(), A0, None, "input_kegel_nicht_bestaetigt";
    ag1_an_gesamt_zusammen_ohne_partner_kegel => "an_gesamt", mit(an(), "veranlagung", json!("zusammen")), A0, None, "partner_kegel_offen";
    ag2_an_gesamt_zusammen_mit_partner_kegel => "an_gesamt", plus(mit(an(), "veranlagung", json!("zusammen")), vec![("bruttoarbeitslohn_partner", json!(0))]), A0, Some(272_800), "bestaetigt";
    pb1_gesamt_wahlrecht_pb_offen => "gesamt", plus(mit(gesamt_zusammen(), "rentner_grad_der_behinderung", json!(30)), vec![("behinderungsbedingte_aufwendungen", json!(B))]), GZ0, None, "behinderungsbedingte_aufwendungen_wahlrecht_offen";
    gew1_gesamt_gewinn_positiv_rechnet => "gesamt", plus(mit(gesamt(), "kein_gewinn", json!(false)), vec![("einkuenfte_gewinn", json!(B))]), G0, Some(7_675_600), "bestaetigt";
    gew2_rentner_gewinn_positiv_sperrt_flag => "rentner_gesamt", plus(mit(rentner(), "kein_vuv", json!(false)), vec![("einkuenfte_gewinn", json!(B))]), R0, None, "flag_konsistenz_offen";
    aw_pr3_gesamt_partnerkern_feld_abgewiesen => "gesamt", plus(gesamt_zusammen(), vec![("rentner_renten_art_partner", json!("gesetzliche_rente"))]), AW, Some(AW), "AW";
    aw_pr4_gesamt_rente_a_abgewiesen => "gesamt", plus(gesamt(), vec![("rentner_jahresrente", json!(B))]), AW, Some(AW), "AW";
    aw_ksp_gesamt_partnerrente_abgewiesen => "gesamt", plus(gesamt_zusammen(), vec![("rentner_jahresrente_partner", json!(B))]), AW, Some(AW), "AW";
    aw_ksp_rentner_kreuz_abgewiesen => "rentner_gesamt", plus(mit(rentner(), "veranlagung", json!("zusammen")), vec![("kein_sonstige_partner", json!(true))]), AW, Some(AW), "AW";
    aw_vuv_rentner_abgewiesen => "rentner_gesamt", plus(rentner(), vec![("vv_einnahmen", json!(B))]), AW, Some(AW), "AW";
    aw_sonstige_gesamt_abgewiesen => "gesamt", plus(gesamt(), vec![("rentner_jahresrente", json!(B))]), AW, Some(AW), "AW";
    aw_p32b_an_gesamt_abgewiesen => "an_gesamt", plus(an(), vec![("kap_kapitalertraege", json!(B))]), AW, Some(AW), "AW";
}
