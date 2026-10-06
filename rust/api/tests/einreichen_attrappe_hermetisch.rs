//! `POST /fall/{id}/einreichen` ab dem Aufruf von `ERiC`, hermetisch: gegen die ATTRAPPE von
//! `libericapi.so` (`rust/elster/tests/eric_attrappe/eric_attrappe.c`), ohne Python, ohne
//! `PARITY=1`, ohne Netz.
//!
//! Auftrag 6 (Mutationsmessung der Crate `api`): die Antworten nach dem `ERiC`-Urteil
//! (`pruefe`, `binde_befund`, `abbruch`) hatte nur der Vergleich mit Python in
//! `rust/parity/tests/api_http_paritaet.rs` bewacht. Fuenf Mutanten ueberlebten ohne ihn: `C079`
//! (Status der Urteilsantworten), `C081` (Status 409 beim Sperrgrund), `C087` (Befund ohne Events
//! binden), `C088` (Hash des Befunds gegen Hash der Deklaration), `C091` (rc "Datenartversion
//! unbekannt"). Die Soll-Werte unten sind die der Python-Messung (`berichte/einreichen-bau.md`,
//! Tabelle `szenarien()` in `eric_attrappe/mod.rs`): Status und `grund` je Szenario.
//!
//! Die Attrappe antwortet nach einem Skript (`$ERIC_ATTRAPPE_DIR/skript`) und schreibt auf, was sie
//! bekam (`gesehen/<n>.xml`, `gesehen/<n>.meta`). Sie sendet nichts und braucht weder Zertifikat noch
//! Hersteller-ID. `ERIC_DIR` und `HOME` zeigen in ein Wegwerf-Verzeichnis: die echte Bibliothek
//! unter `~/02_Software/eric` wird nie geladen. `ERiC` laedt je Prozess einmal; darum steht alles in
//! EINEM Test.
//!
//! Das Schema `E10-2025.xsd` braucht der XML-Writer. Es kommt aus der lokalen ERiC-Auslieferung; fehlt
//! es, ist der Test rot, ausser mit `TAXGRAPH_OHNE_XSD=1` (die CI hat keine Auslieferung).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines
)]

use std::path::{Path, PathBuf};
use std::process::Command;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use axum::body::Body;
use axum::http::Request;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn lade_fixture(name: &str) -> Value {
    let p = wurzel().join(format!("rust/fixtures/e2e/{name}.json"));
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn setze(d: &mut Value, feld: &str, wert: &Value) {
    for e in d["events"].as_array_mut().unwrap() {
        if e["feld_id"] == feld {
            e["wert"] = wert.clone();
        }
    }
}

/// Die Faelle: der vollstaendige `gesamt` und drei Abwandlungen, wie in `eric_attrappe/mod.rs::faelle`.
fn faelle() -> Vec<(&'static str, Value)> {
    let gesamt = lade_fixture("gesamt");
    let mut v = vec![("e_ges", gesamt.clone()), ("e_ges2", gesamt.clone())];
    // Ein Wert, dem das zweite Signal fehlt: `deklaration_unvollstaendig`.
    let mut d = gesamt.clone();
    for e in d["events"].as_array_mut().unwrap() {
        if e["feld_id"] == "kap_kapitalertraege" {
            e["zustand"] = json!("vorlaeufig");
            e["signal"]["signal_2"] = Value::Null;
        }
    }
    v.push(("e_vorl", d));
    // Aggregat und Topf der Kapitalertraege zugleich: Sperrgrund `kapital_semantik_offen`.
    let mut d = gesamt.clone();
    setze(&mut d, "kap_kapitalertraege", &json!(500_000));
    setze(&mut d, "kap_gewinn_aktien", &json!(500_000));
    setze(&mut d, "kein_kap", &json!(false));
    v.push(("e_sperre", d));
    // Ohne Geburtsdatum: das Abgabe-Gate des Writers (`xml_nicht_baubar`).
    let mut d = gesamt;
    d["events"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["feld_id"] != "stammdaten_geburtsdatum");
    v.push(("e_ohnegeb", d));
    for n in ["e_pl", "e_pm", "e_rc"] {
        v.push((n, lade_fixture("gesamt")));
    }
    // Ein leerer Fall: der Writer baut kein XML. `binde_befund` sieht nie einen Fall ohne Events.
    v.push((
        "e_leer",
        json!({"version": 1, "veranlagungszeitraum": 2025, "scheibe": "gesamt", "events": [], "snapshots": []}),
    ));
    for (name, d) in &mut v {
        d["fall_id"] = json!(name);
        d["user_id"] = json!("alice");
    }
    v
}

fn baue_attrappe(ziel: &Path) {
    let quelle =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../elster/tests/eric_attrappe/eric_attrappe.c");
    let st = Command::new("cc")
        .args(["-shared", "-fPIC", "-O0", "-Wall", "-Wextra", "-o"])
        .arg(ziel)
        .arg(&quelle)
        .status()
        .expect("cc fehlt: die Attrappe von libericapi.so laesst sich nicht bauen");
    assert!(st.success(), "cc scheiterte an {}", quelle.display());
}

/// Eine Antwort von `ERiC` fuer den naechsten Aufruf: Zeile 1 der Rueckgabecode, danach der Puffer.
fn skript(steuer: &Path, rc: i64, puffer: &str) {
    for n in ["zaehler", "gesehen"] {
        let p = steuer.join(n);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_dir_all(&p);
    }
    std::fs::write(steuer.join("skript"), format!("{rc}\n{puffer}")).unwrap();
}

/// Was die Attrappe sah: `(meta, xml)` je Aufruf, in der Reihenfolge.
fn gesehen(steuer: &Path) -> Vec<(String, Vec<u8>)> {
    let mut v = Vec::new();
    let mut n = 1;
    while let Ok(xml) = std::fs::read(steuer.join(format!("gesehen/{n}.xml"))) {
        let meta =
            std::fs::read_to_string(steuer.join(format!("gesehen/{n}.meta"))).unwrap_or_default();
        v.push((meta, xml));
        n += 1;
    }
    v
}

fn fehlerliste(n: usize) -> String {
    use std::fmt::Write as _;
    let mut s = String::from("<EricAntwort><Rueckgabe>");
    for i in 0..n {
        let _ = write!(
            s,
            "<FehlerRegelpruefung><Nr>{i}</Nr><Text>Regel {i} verletzt</Text></FehlerRegelpruefung>"
        );
    }
    s.push_str("</Rueckgabe></EricAntwort>");
    s
}

async fn einreichen(z: &Zustand, token: &str, fall: &str, rumpf: &Value) -> (u16, Value) {
    let text = rumpf.to_string();
    let req = Request::builder()
        .method("POST")
        .uri(format!("/fall/{fall}/einreichen"))
        .header("authorization", token)
        .header("content-type", "application/json")
        .header("content-length", text.len().to_string())
        .body(Body::from(text))
        .unwrap();
    let r = app(z.clone()).oneshot(req).await.unwrap();
    let (teile, rumpf) = r.into_parts();
    let bytes = rumpf.collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes)
        .unwrap_or_else(|e| panic!("kein JSON: {e}: {}", String::from_utf8_lossy(&bytes)));
    (teile.status.as_u16(), json)
}

fn akte(z: &Zustand, fall: &str) -> Value {
    let p = z.konfig.faelle.join(format!("{fall}.json"));
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn audit_zeilen(z: &Zustand) -> Vec<Value> {
    std::fs::read_to_string(z.konfig.audit_pfad())
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[tokio::test]
async fn einreichen_nach_dem_urteil_von_eric() {
    // Das Schema liegt in der lokalen Auslieferung; zu lesen, solange HOME und ERIC_DIR echt sind.
    let Some(xsd) = elster::finde_schema(2025, "E10-{jahr}.xsd") else {
        assert!(
            std::env::var("TAXGRAPH_OHNE_XSD").as_deref() == Ok("1"),
            "E10-2025.xsd fehlt in der lokalen ERiC-Auslieferung ($ERIC_DIR, ~/02_Software/eric); \
             nur TAXGRAPH_OHNE_XSD=1 erlaubt das Fehlen"
        );
        eprintln!("[einreichen_attrappe] UEBERSPRUNGEN: E10-2025.xsd fehlt, TAXGRAPH_OHNE_XSD=1");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let eric = tmp.path().join("eric");
    std::fs::create_dir_all(eric.join("schema/2025")).unwrap();
    std::fs::copy(xsd, eric.join("schema/2025/E10-2025.xsd")).unwrap();
    std::fs::create_dir_all(eric.join("lib")).unwrap();
    baue_attrappe(&eric.join("lib/libericapi.so"));
    let steuer = tmp.path().join("steuer");
    std::fs::create_dir_all(&steuer).unwrap();
    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("ERIC_DIR", &eric);
    std::env::set_var("ERIC_ATTRAPPE_DIR", &steuer);
    std::env::set_var("HOME", &home);
    // Die oeffentlich bekannte Platzhalter-ID aus den amtlichen ERiC-Beispielen.
    std::env::set_var("ELSTER_HERSTELLER_ID", "74931");

    let konfig = Konfig {
        wurzel: wurzel(),
        faelle: tmp.path().join("faelle"),
        audit_dir: tmp.path().join("faelle"),
    };
    std::fs::create_dir_all(&konfig.faelle).unwrap();
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.path().join("users.json"),
        Some(konfig.audit_pfad()),
    );
    let z = Zustand::neu(konfig, auth);
    for (name, d) in faelle() {
        std::fs::write(
            z.konfig.faelle.join(format!("{name}.json")),
            serde_json::to_vec(&d).unwrap(),
        )
        .unwrap();
    }
    let token = format!("Bearer {}", z.auth.stelle_aus("alice").unwrap());
    let leer = json!({});

    // --- rc 0: plausibel. Das XML an ERiC ist byte-gleich der Vorlage, der Befund bindet an den Snapshot.
    skript(&steuer, 0, "<Ok/>");
    let (status, a) = einreichen(&z, &token, "e_ges", &leer).await;
    assert_eq!(status, 200, "{a}");
    assert_eq!(a["plausibel"], json!(true), "{a}");
    assert_eq!(a["eingereicht"], json!(false), "{a}");
    assert_eq!(a["rc"], json!(0), "{a}");
    assert_eq!(a["klasse"], json!("plausibel"), "{a}");
    assert_eq!(a["befund_gebunden"], json!(true), "{a}");
    assert_eq!(a["vz"], json!(2025), "{a}");
    let g = gesehen(&steuer);
    assert_eq!(g.len(), 1, "ein Aufruf von EricBearbeiteVorgang");
    let soll_xml = std::fs::read(wurzel().join("rust/fixtures/e2e/gesamt.xml")).unwrap();
    assert_eq!(g[0].1, soll_xml, "das XML an ERiC ist nicht gesamt.xml");
    assert_eq!(a["xml_bytes"], json!(soll_xml.len()), "{a}");
    for soll in [
        "datenart=ESt_2025\n",
        "flags=2\n",
        "druck=NULL\n",
        "crypto=NULL\n",
        "serverantwort=NULL\n",
    ] {
        assert!(g[0].0.contains(soll), "{soll:?} fehlt in {:?}", g[0].0);
    }
    // Der Befund steht im Snapshot, den die Antwort nennt.
    let datei = akte(&z, "e_ges");
    let snaps = datei["snapshots"].as_array().unwrap();
    assert_eq!(snaps.len(), 1, "{datei}");
    assert_eq!(a["basis_snapshot"], snaps[0]["snapshot_id"], "{a}");
    assert_eq!(snaps[0]["eric_befund"]["rc"], json!(0));
    assert_eq!(snaps[0]["eric_befund"]["klasse"], json!("plausibel"));
    assert_eq!(
        snaps[0]["eric_befund"]["gebunden_an"],
        snaps[0]["snapshot_id"]
    );
    // Das Protokoll nennt Nutzer, Akte und Jahr.
    let validiert: Vec<Value> = audit_zeilen(&z)
        .into_iter()
        .filter(|l| l["action"] == "fall_validiert")
        .collect();
    assert_eq!(validiert.len(), 1, "{validiert:?}");
    assert_eq!(
        (
            validiert[0]["user_id"].as_str(),
            validiert[0]["fall_id"].as_str(),
            validiert[0]["detail"].as_str()
        ),
        (Some("alice"), Some("e_ges"), Some("vz=2025 rc=0"))
    );

    // --- Derselbe Fall noch einmal: der zweite Befund haengt einen zweiten Snapshot an.
    skript(&steuer, 0, "<Ok/>");
    let (status, a) = einreichen(&z, &token, "e_ges", &leer).await;
    assert_eq!((status, &a["plausibel"]), (200, &json!(true)), "{a}");
    assert_eq!(akte(&z, "e_ges")["snapshots"].as_array().unwrap().len(), 2);

    // --- Plausibilitaetsfehler: 422, der Befund ist trotzdem gebunden.
    skript(&steuer, 610_001_002, &fehlerliste(3));
    let (status, a) = einreichen(&z, &token, "e_pl", &leer).await;
    assert_eq!(status, 422, "{a}");
    assert_eq!(a["grund"], json!("plausibilitaet_verletzt"), "{a}");
    assert_eq!(a["eingereicht"], json!(false), "{a}");
    assert_eq!(a["klasse"], json!("plausibilitaet_fehler"), "{a}");
    assert_eq!(a["befund_gebunden"], json!(true), "{a}");
    assert_eq!(akte(&z, "e_pl")["snapshots"].as_array().unwrap().len(), 1);

    // --- Kein Pruefmodul fuer diesen Veranlagungszeitraum: 422, die Erklaerung wurde NICHT geprueft.
    skript(&steuer, 610_001_042, "<Fehler/>");
    let (status, a) = einreichen(&z, &token, "e_pm", &leer).await;
    assert_eq!(status, 422, "{a}");
    assert_eq!(a["grund"], json!("kein_pruefmodul_fuer_vz"), "{a}");
    assert_eq!(a["klasse"], json!("datenartversion_unbekannt"), "{a}");
    assert!(a["detail"].as_str().unwrap().contains("ESt_2025"), "{a}");

    // --- Ein rc ohne Plausibilitaetsurteil (fail-closed): 422, kein Freibrief.
    skript(&steuer, 12_345, "");
    let (status, a) = einreichen(&z, &token, "e_rc", &leer).await;
    assert_eq!(status, 422, "{a}");
    assert_eq!(a["grund"], json!("rc_kein_plausibilitaetsverdikt"), "{a}");
    assert_eq!(a["rc"], json!(12_345), "{a}");

    // --- Abbruch vor ERiC: Sperrgrund und unvollstaendige Deklaration sind 409, der Writer ist 422.
    // ERiC bekommt in allen vier Faellen keinen Aufruf. Der leere Fall (`e_leer`) belegt, dass
    // `binde_befund` nie einen Fall ohne Events sieht: ohne Events kein XML, ohne XML kein Urteil.
    skript(&steuer, 0, "<Ok/>");
    for (fall, status_soll, grund) in [
        ("e_sperre", 409, "kapital_semantik_offen"),
        ("e_vorl", 409, "deklaration_unvollstaendig"),
        ("e_ohnegeb", 422, "xml_nicht_baubar"),
        ("e_leer", 422, "xml_nicht_baubar"),
    ] {
        let (status, a) = einreichen(&z, &token, fall, &leer).await;
        assert_eq!(
            (status, a["grund"].as_str()),
            (status_soll, Some(grund)),
            "{fall}: {a}"
        );
        assert_eq!(a["eingereicht"], json!(false), "{fall}: {a}");
        assert!(gesehen(&steuer).is_empty(), "{fall}: ERiC wurde gefragt");
    }
}
