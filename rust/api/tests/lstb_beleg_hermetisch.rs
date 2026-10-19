//! `POST /fall/{id}/beleg` (Abweichung Nr. 90, Stufe S1): die Beleg-Datei der Lohnsteuerbescheinigung (`VaSt_LStB` 202501) geht als Text im Feld
//! `beleg_xml` in den Dienst; die Beträge der Person A kommen über `eingang::edaten::uebernehme` als BESTÄTIGT in den Fall. Hermetisch: der Beleg
//! ist erfunden (Firma, Name, Beträge; die Steuer-ID `00000000000` beginnt mit 0 und kann nie vergeben sein), kein `ERiC`, kein ELSTER.
//!
//! Was die Route zusagt, und der Test, der es hält:
//! - Alles oder nichts: jeder Fehler (zweiter Beleg, wiederholtes Blatt, falsche Version, falsches Jahr, kaputtes XML, nicht lesbarer Betrag,
//!   Abweisung des Stores nach dem ersten Satz, Fall außerhalb von 2025, Rumpf ohne `beleg_xml`) lässt die Akte Byte für Byte, wie sie war.
//! - Eigene Angaben haben Vorrang; ein zweiter Lauf schreibt nichts.
//! - Anmeldung, Besitz und Existenz des Falls wie bei jeder Fall-Route (401, 403, 404).
//! - Nur Felder der Tabellen `LSTB` und `LSTB_SUMMEN`, nur Person A.
//! - Steuer-ID, Namen und Firma stehen nach keinem Aufruf in einer Datei des Dienstes und in keiner Antwort (Akte, Audit, Fehlerlog).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use eingang::vast::{LSTB, LSTB_SUMMEN};
use eingang::vast_beleg::BELEG_NAMENSRAUM;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

const IDNR: &str = "00000000000";
const NAME: &str = "Mustermann";
const VORNAME: &str = "Erika";
const FIRMA: &str = "Erfundene Firma GmbH";
const GEHEIM: [&str; 4] = [IDNR, NAME, VORNAME, FIRMA];

/// (Element, Text, Zielfeld, Cent des Ziels): erfundene Beträge für alle 15 Elemente, die `LSTB` und `LSTB_SUMMEN` lesen.
const BETRAEGE: [(&str, &str, &str, i64); 15] = [
    ("BruttoArbLohn", "61234.56", "bruttoarbeitslohn", 6_123_456),
    ("LSteuer", "9876.54", "p36_lohnsteuer", 987_654),
    ("ArbnKiSteuer", "612.34", "kist_gezahlt", 61_234),
    ("LeistungenProgVorbeh", "1500.10", "p32b_progressionseinkuenfte", 150_010),
    ("ArbnAnteilKrankVers", "3210.98", "basis_kv", 321_098),
    ("ArbnAnteilPflegVers", "654.32", "basis_pv", 65_432),
    ("StFreiVerpfleg", "120.00", "vpf_steuerfreie_erstattung_betrag", 12_000),
    ("StFreiGeKrankVers", "1111.11", "zuschuss_ag_kv_gesetzlich", 111_111),
    ("StFreiPrKrankVers", "2222.22", "zuschuss_ag_kv_privat", 222_222),
    ("StFreiGePflegVers", "333.33", "zuschuss_ag_pv", 33_333),
    ("ArbnAnteilArblVers", "745.67", "vorsorge_arbeitslosenversicherung", 74_567),
    ("ArbnAnteilRenVers", "5678.90", "vor_an_anteil_rv", 577_891),
    ("ArbnAnteilBerufsVers", "100.01", "vor_an_anteil_rv", 577_891),
    ("ArbgAnteilRenVers", "5678.91", "vor_ag_anteil_rv", 587_893),
    ("ArbgAnteilBerufsVers", "200.02", "vor_ag_anteil_rv", 587_893),
];

/// Die 13 Zielfelder und ihr Cent-Wert.
fn ziele() -> BTreeMap<&'static str, i64> {
    BETRAEGE.iter().map(|(_, _, feld, cent)| (*feld, *cent)).collect()
}

fn grund_alle() -> String {
    BETRAEGE.iter().map(|(e, t, _, _)| format!("      <{e}>{t}</{e}>\n")).collect::<Vec<_>>().concat()
}

fn beleg_mit(ns: &str, version: &str, jahr: &str, grund: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VaSt_LStB xmlns=\"{ns}\" version=\"{version}\">\n  <ArbGName>{FIRMA}</ArbGName>\n  \
         <Allgemein>\n    <IdNr>{IDNR}</IdNr>\n    <Name>{NAME}</Name>\n    <Vorname>{VORNAME}</Vorname>\n  </Allgemein>\n  \
         <Dauer>\n    <Jahr>{jahr}</Jahr>\n  </Dauer>\n  <Besteuerungsmerkmal>\n    <ELStAM><GueltigAb>0101</GueltigAb></ELStAM>\n    \
         <ELStAM><GueltigAb>0107</GueltigAb></ELStAM>\n  </Besteuerungsmerkmal>\n  <Besteuerungsgrund>\n{grund}  </Besteuerungsgrund>\n</VaSt_LStB>\n"
    )
}

fn beleg_gut() -> String {
    beleg_mit(BELEG_NAMENSRAUM, "202501", "2025", &grund_alle())
}

struct Dienst {
    zustand: Zustand,
    tmp: tempfile::TempDir,
}

fn dienst() -> Dienst {
    let tmp = tempfile::tempdir().unwrap();
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    let auth = Auth::neu("testgeheimnis".into(), tmp.path().join("users.json"), Some(konfig.audit_pfad()));
    Dienst { zustand: Zustand::neu(konfig, auth), tmp }
}

/// POST mit JSON-Rumpf; `nutzer = None` schickt kein Token.
async fn sende(d: &Dienst, nutzer: Option<&str>, pfad: &str, body: &Value) -> (u16, Value) {
    let text = body.to_string();
    let mut req = Request::builder()
        .method("POST")
        .uri(pfad)
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string());
    if let Some(n) = nutzer {
        req = req.header("authorization", format!("Bearer {}", d.zustand.auth.stelle_aus(n).unwrap()));
    }
    let r = app(d.zustand.clone()).oneshot(req.body(Body::from(text)).unwrap()).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    (teile.status.as_u16(), serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

async fn beleg_senden(d: &Dienst, nutzer: &str, fall: &str, xml: &str) -> (u16, Value) {
    sende(d, Some(nutzer), &format!("/fall/{fall}/beleg"), &json!({ "beleg_xml": xml })).await
}

async fn fall_anlegen(d: &Dienst, nutzer: &str, id: &str, scheibe: &str, vz: i64) {
    let (status, antwort) =
        sende(d, Some(nutzer), "/fall", &json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": vz})).await;
    assert_eq!(status, 201, "POST /fall: {antwort}");
}

fn akte_pfad(d: &Dienst, id: &str) -> PathBuf {
    d.zustand.konfig.faelle.join(format!("{id}.json"))
}

fn akte_bytes(d: &Dienst, id: &str) -> Vec<u8> {
    std::fs::read(akte_pfad(d, id)).unwrap()
}

fn akte(d: &Dienst, id: &str) -> Value {
    serde_json::from_slice(&akte_bytes(d, id)).unwrap()
}

/// Die aktiven Events der Akte je Feld.
fn events(d: &Dienst, id: &str) -> BTreeMap<String, Value> {
    akte(d, id)["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["feld_id"].as_str().unwrap().to_owned(), e.clone()))
        .collect()
}

fn liste(v: &Value) -> BTreeSet<String> {
    v.as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect()
}

/// Jede Datei unter `wurzel`, rekursiv, mit Inhalt.
fn alle_dateien(wurzel: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut aus = Vec::new();
    for e in std::fs::read_dir(wurzel).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            aus.extend(alle_dateien(&p));
        } else {
            aus.push((p.clone(), std::fs::read(&p).unwrap()));
        }
    }
    aus
}

fn enthaelt(heu: &[u8], nadel: &str) -> bool {
    heu.windows(nadel.len()).any(|w| w == nadel.as_bytes())
}

#[tokio::test]
async fn ein_beleg_geht_als_bestaetigter_cent_wert_der_person_a_in_den_fall() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b1", "gesamt", 2025).await;
    let (s, a) = beleg_senden(&d, "alice", "b1", &beleg_gut()).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["beleg_version"], "202501");
    let soll = ziele();
    let uebernommen = liste(&a["uebernommen"]);
    let uebersprungen = liste(&a["uebersprungen"]);
    assert_eq!(uebersprungen.len(), 0, "in der Scheibe gesamt steht jedes Ziel: {a}");
    assert_eq!(uebernommen, soll.keys().map(|k| (*k).to_owned()).collect::<BTreeSet<_>>(), "alle 13 Ziele: {a}");
    let ev = events(&d, "b1");
    assert_eq!(ev.keys().cloned().collect::<BTreeSet<_>>(), uebernommen, "die Akte trägt genau die übernommenen Felder");
    for (feld, cent) in &soll {
        let e = &ev[*feld];
        assert_eq!(e["zustand"], "bestaetigt", "{feld}");
        assert_eq!(e["wert"], *cent, "{feld}: Cent");
        assert_eq!(e["schreiber"], "import:elster", "{feld}: der Schreiber der eDaten-Übernahme");
    }
    assert!(!ev.keys().any(|k| k.ends_with("_partner")), "nur Person A: {:?}", ev.keys());
}

#[tokio::test]
async fn die_route_schreibt_nur_felder_der_tabellen_lstb_und_lstb_summen() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b2", "gesamt", 2025).await;
    // Dazu Elemente, die der Import bewusst nicht liest (`NICHT_GEMAPPT`): einbehaltener Solidaritätszuschlag, Großbuchstabe, Anzahl U.
    let grund = format!("{}      <Soli>55.55</Soli>\n      <Grossbuchstaben>B</Grossbuchstaben>\n      <AnzahlU>3</AnzahlU>\n", grund_alle());
    let (s, a) = beleg_senden(&d, "alice", "b2", &beleg_mit(BELEG_NAMENSRAUM, "202501", "2025", &grund)).await;
    assert_eq!(s, 200, "{a}");
    let erlaubt: BTreeSet<&str> = LSTB
        .iter()
        .map(|(_, feld, _)| *feld)
        .chain(LSTB_SUMMEN.iter().map(|(feld, _)| *feld))
        .collect();
    let ev = events(&d, "b2");
    let fremd: Vec<&String> = ev.keys().filter(|k| !erlaubt.contains(k.as_str())).collect();
    assert_eq!(fremd.len(), 0, "Felder außerhalb der Tabellen: {fremd:?}");
    assert!(!ev.values().any(|e| e["wert"] == 5555 || e["wert"] == "B" || e["wert"] == 3), "ein nicht gelesenes Element erreichte den Fall");
    assert_eq!(ev.len(), 13);
}

#[tokio::test]
async fn jeder_fehler_antwortet_benannt_und_laesst_die_akte_byte_fuer_byte_wie_sie_war() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b3", "gesamt", 2025).await;
    let vor = akte_bytes(&d, "b3");
    let behaelter = |a: &str, b: &str| {
        let innen = |x: &str| x.replace("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n", "");
        format!("<?xml version=\"1.0\"?>\n<Nutzdaten>\n{}{}</Nutzdaten>\n", innen(a), innen(b))
    };
    let alt_ns = "http://finkonsens.de/elster/elstervast/vastlstb/v202401";
    let wiederholt = format!("{}      <LSteuer>1.00</LSteuer>\n", grund_alle());
    let unlesbar = grund_alle().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer>77zz</LSteuer>");
    // Eine negative Lohnsteuer lässt der Store nach dem Bruttoarbeitslohn abweisen (`nicht_negativ`): zwei Sätze wären schon im Speicher.
    let negativ = grund_alle().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer>-5.00</LSteuer>");
    let proben: Vec<(&str, u16, String)> = vec![
        ("beleg_mehrere", 422, behaelter(&beleg_gut(), &beleg_gut())),
        ("beleg_fehlt", 422, "<Nutzdaten/>".to_owned()),
        ("beleg_element_mehrfach", 422, beleg_mit(BELEG_NAMENSRAUM, "202501", "2025", &wiederholt)),
        ("beleg_version", 422, beleg_mit(alt_ns, "202401", "2024", &grund_alle())),
        ("beleg_jahr", 422, beleg_mit(BELEG_NAMENSRAUM, "202501", "2024", &grund_alle())),
        ("beleg_kaputt", 422, beleg_gut()[..120].to_owned()),
        ("beleg_betrag_unlesbar", 422, beleg_mit(BELEG_NAMENSRAUM, "202501", "2025", &unlesbar)),
        ("beleg_abgewiesen", 422, beleg_mit(BELEG_NAMENSRAUM, "202501", "2025", &negativ)),
    ];
    for (code, status, xml) in &proben {
        let (s, a) = beleg_senden(&d, "alice", "b3", xml).await;
        assert_eq!(s, *status, "{code}: {a}");
        let text = a["fehler"].as_str().unwrap_or_default();
        assert!(text.starts_with(&format!("{code}: ")), "der Fehler trägt seinen Namen vorn ({code}): {a}");
        for geheim in GEHEIM {
            assert!(!a.to_string().contains(geheim), "{code}: die Antwort nennt {geheim}");
        }
        assert_eq!(akte_bytes(&d, "b3"), vor, "{code}: die Akte hat sich geändert");
    }
    // Rumpf ohne Beleg-Text: 400, auch als Liste oder mit einer Zahl.
    for rumpf in [json!({}), json!({"beleg_xml": 5}), json!({"beleg_xml": null}), json!([]), json!({"xml": "<a/>"})] {
        let (s, a) = sende(&d, Some("alice"), "/fall/b3/beleg", &rumpf).await;
        assert_eq!(s, 400, "{rumpf}: {a}");
        assert!(a["fehler"].as_str().unwrap().starts_with("beleg_xml_fehlt: "), "{a}");
        assert_eq!(akte_bytes(&d, "b3"), vor);
    }
    // KONTROLLE: derselbe Fall nimmt den guten Beleg an; sonst bewiese die Schleife oben nichts über die Route.
    let (s, a) = beleg_senden(&d, "alice", "b3", &beleg_gut()).await;
    assert_eq!(s, 200, "{a}");
    assert_ne!(akte_bytes(&d, "b3"), vor);
}

#[tokio::test]
async fn eine_abweisung_nach_dem_ersten_satz_laesst_nichts_stehen_und_der_naechste_lauf_geht() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b4", "gesamt", 2025).await;
    let negativ = grund_alle().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer>-5.00</LSteuer>");
    let (s, a) = beleg_senden(&d, "alice", "b4", &beleg_mit(BELEG_NAMENSRAUM, "202501", "2025", &negativ)).await;
    assert_eq!(s, 422, "{a}");
    assert_eq!(events(&d, "b4").len(), 0, "keine Teilschreibung: {:?}", events(&d, "b4").keys());
    let (s, a) = beleg_senden(&d, "alice", "b4", &beleg_gut()).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(events(&d, "b4").len(), 13);
}

#[tokio::test]
async fn eine_eigene_angabe_hat_vorrang_und_ein_zweiter_lauf_schreibt_nichts() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b5", "gesamt", 2025).await;
    let eigene = json!({"feld_id": "bruttoarbeitslohn", "wert": 5_000_000, "zustand": "bestaetigt", "schreiber": "ui:naht",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": "klick@bruttoarbeitslohn"}, "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null});
    let (s, a) = sende(&d, Some("alice"), "/fall/b5/event", &eigene).await;
    assert_eq!(s, 201, "{a}");
    let (s, a) = beleg_senden(&d, "alice", "b5", &beleg_gut()).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(liste(&a["uebersprungen"]), BTreeSet::from(["bruttoarbeitslohn".to_owned()]), "{a}");
    assert_eq!(a["uebernommen"].as_array().unwrap().len(), 12);
    assert_eq!(events(&d, "b5")["bruttoarbeitslohn"]["wert"], 5_000_000, "die eigene Angabe bleibt");
    let nach_dem_ersten = akte_bytes(&d, "b5");
    let (s, a) = beleg_senden(&d, "alice", "b5", &beleg_gut()).await;
    assert_eq!(s, 200, "{a}");
    assert_eq!(a["uebernommen"].as_array().unwrap().len(), 0, "ein zweiter Lauf schreibt nichts: {a}");
    assert_eq!(a["uebersprungen"].as_array().unwrap().len(), 13);
    assert_eq!(akte_bytes(&d, "b5"), nach_dem_ersten);
}

#[tokio::test]
async fn ein_fall_ausserhalb_von_2025_nimmt_keinen_beleg() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b6", "gesamt", 2024).await;
    let vor = akte_bytes(&d, "b6");
    let (s, a) = beleg_senden(&d, "alice", "b6", &beleg_gut()).await;
    assert_eq!(s, 422, "{a}");
    assert!(a["fehler"].as_str().unwrap().starts_with("beleg_fall_jahr: "), "{a}");
    assert_eq!(akte_bytes(&d, "b6"), vor);
}

#[tokio::test]
async fn ein_fall_der_scheibe_ep_nimmt_nur_felder_seiner_scheibe() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b7", "ep", 2025).await;
    let (s, a) = beleg_senden(&d, "alice", "b7", &beleg_gut()).await;
    assert_eq!(s, 200, "{a}");
    let alle: BTreeSet<String> = liste(&a["uebernommen"]).union(&liste(&a["uebersprungen"])).cloned().collect();
    assert_eq!(alle.len(), 13, "jedes Ziel steht in genau einer der beiden Listen: {a}");
    assert_eq!(liste(&a["uebernommen"]).len(), 0, "die Scheibe ep fragt keines der 13 Ziele: {a}");
    assert_eq!(events(&d, "b7").len(), 0);
}

#[tokio::test]
async fn anmeldung_besitz_und_existenz_des_falls_wie_bei_jeder_fall_route() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b8", "gesamt", 2025).await;
    let vor = akte_bytes(&d, "b8");
    let rumpf = json!({"beleg_xml": beleg_gut()});
    let (s, a) = sende(&d, None, "/fall/b8/beleg", &rumpf).await;
    assert_eq!(s, 401, "kein Token: {a}");
    let (s, a) = sende(&d, Some("bob"), "/fall/b8/beleg", &rumpf).await;
    assert_eq!(s, 403, "ein anderer Nutzer: {a}");
    let (s, a) = sende(&d, Some("alice"), "/fall/gibt-es-nicht/beleg", &rumpf).await;
    assert_eq!(s, 404, "unbekannter Fall: {a}");
    assert_eq!(akte_bytes(&d, "b8"), vor, "401 und 403 lassen die Akte unberührt");
}

#[tokio::test]
async fn steuer_id_namen_und_firma_stehen_nach_keinem_aufruf_in_einer_datei_des_dienstes() {
    let d = dienst();
    fall_anlegen(&d, "alice", "b9", "gesamt", 2025).await;
    let behaelter = format!("<Nutzdaten>{}{}</Nutzdaten>", beleg_gut(), beleg_gut());
    let mut antworten = Vec::new();
    antworten.push(beleg_senden(&d, "alice", "b9", &behaelter).await);
    antworten.push(beleg_senden(&d, "alice", "b9", &beleg_mit("x", "202501", "2025", &grund_alle())).await);
    antworten.push(beleg_senden(&d, "alice", "b9", &beleg_gut()).await);
    antworten.push(beleg_senden(&d, "alice", "b9", &beleg_gut()).await);
    assert_eq!(antworten.iter().map(|(s, _)| *s).collect::<Vec<_>>(), vec![422, 422, 200, 200]);
    for (_, a) in &antworten {
        for geheim in GEHEIM {
            assert!(!a.to_string().contains(geheim), "die Antwort nennt {geheim}: {a}");
        }
    }
    let dateien = alle_dateien(d.tmp.path());
    // KONTROLLE: der Lauf sieht die Akte, die Nutzerdatei und das Audit; im Audit steht der Aufruf.
    assert!(dateien.iter().any(|(p, _)| p.ends_with("b9.json")), "{:?}", dateien.iter().map(|(p, _)| p).collect::<Vec<_>>());
    let audit = dateien.iter().find(|(_, inhalt)| enthaelt(inhalt, "fall_beleg")).expect("das Audit nennt den Aufruf fall_beleg");
    assert!(enthaelt(&audit.1, "status=200"), "das Audit trägt den Status");
    for (pfad, inhalt) in &dateien {
        for geheim in GEHEIM {
            assert!(!enthaelt(inhalt, geheim), "{} enthält {geheim}", pfad.display());
        }
    }
}
