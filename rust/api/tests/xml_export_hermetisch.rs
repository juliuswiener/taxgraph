//! `taxgraph-xml`: die Erklärung eines Falls als XML-Datei, der Weg vom gespeicherten Fall zu dem,
//! was `taxgraph-versand` prüft (Abweichung Nr. 60). Hermetisch: kein `ERiC`, kein Netz, kein Zertifikat.
//!
//! Drei Dinge hält die Datei fest. Erstens: die Bytes sind die, die `POST /einreichen` an `ERiC` gibt
//! (`rust/fixtures/e2e/gesamt.xml`, dort in `einreichen_attrappe_hermetisch.rs` gegen die Attrappe
//! belegt). Zweitens: wo `POST /einreichen` sperrt, entsteht keine Datei. Drittens: die Datei
//! entsteht atomar, nur für den Besitzer lesbar, und ersetzt nie eine vorhandene.
//!
//! Der XML-Writer braucht `E10-2025.xsd` aus der lokalen `ERiC`-Auslieferung; die CI hat sie nicht
//! (`TAXGRAPH_OHNE_XSD=1`, wie in `ci.yml`). Die Sperrfälle brauchen sie nicht und laufen immer.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use api::konfig::Konfig;
use api::{einreichen::xml_exportieren, ApiFehler, Zustand};
use auth::Auth;
use serde_json::{json, Value};

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn gesamt() -> Value {
    let p = wurzel().join("rust/fixtures/e2e/gesamt.json");
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}

fn setze(d: &mut Value, feld: &str, wert: &Value) {
    for e in d["events"].as_array_mut().unwrap() {
        if e["feld_id"] == feld {
            e["wert"] = wert.clone();
        }
    }
}

/// Die Angestellte ohne Anlagen, die `ERiC` annimmt (`einreichen_eric_echt.rs::basis`, Cent). Die
/// `event_id`s sind fortlaufend: der Leser prueft sie beim Laden nicht (Entscheid `lade-pruefung-nicht-gebaut`).
fn basis_fall() -> Value {
    let felder = [
        ("bruttoarbeitslohn", json!(6_000_000)),
        ("vor_an_anteil_rv", json!(4_200_000)),
        ("vor_ag_anteil_rv", json!(1_200_000)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("stammdaten_nachname", json!("Maier")),
        ("stammdaten_vorname", json!("Hans")),
        ("stammdaten_geburtsdatum", json!("05.05.1955")),
        ("stammdaten_strasse", json!("Musterstr.")),
        ("stammdaten_hausnummer", json!("55")),
        ("stammdaten_plz", json!("55555")),
        ("stammdaten_wohnort", json!("Musterort")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
        // Praefix 9181 passt zur Finanzamtsnummer des Empfaengers (BY).
        ("stammdaten_steuernummer", json!("9181081508155")),
        ("steuerklasse", json!("1")),
        ("p36_lohnsteuer", json!(1_200_000)),
        ("veranlagung", json!("einzel")),
    ];
    let events: Vec<Value> = felder
        .iter()
        .enumerate()
        .map(|(i, (feld, wert))| {
            json!({
                "event_id": format!("{i:064x}"), "ts": "2026-01-01T00:00:00+00:00",
                "feld_id": feld, "wert": wert, "zustand": "bestaetigt",
                "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": format!("ok@{feld}")},
                "ersetzt": null,
            })
        })
        .collect();
    json!({"version": 1, "veranlagungszeitraum": 2025, "scheibe": "gesamt", "events": events, "snapshots": []})
}

/// Die Fälle wie in `einreichen_attrappe_hermetisch.rs`: der vollständige `gesamt` und die Abwandlungen,
/// an denen `POST /einreichen` vor `ERiC` abbricht.
fn faelle() -> Vec<(&'static str, Value)> {
    let mut v = vec![("e_ges", gesamt())];
    let mut d = gesamt();
    for e in d["events"].as_array_mut().unwrap() {
        if e["feld_id"] == "kap_kapitalertraege" {
            e["zustand"] = json!("vorlaeufig");
            e["signal"]["signal_2"] = Value::Null;
        }
    }
    v.push(("e_vorl", d));
    let mut d = gesamt();
    setze(&mut d, "kap_kapitalertraege", &json!(500_000));
    setze(&mut d, "kap_gewinn_aktien", &json!(500_000));
    setze(&mut d, "kein_kap", &json!(false));
    v.push(("e_sperre", d));
    let mut d = gesamt();
    d["events"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["feld_id"] != "stammdaten_geburtsdatum");
    v.push(("e_ohnegeb", d));
    v.push(("e_basis", basis_fall()));
    for (name, d) in &mut v {
        d["fall_id"] = json!(name);
        d["user_id"] = json!("alice");
    }
    v
}

fn dienst(tmp: &Path) -> Zustand {
    let konfig = Konfig {
        wurzel: wurzel(),
        faelle: tmp.join("faelle"),
        audit_dir: tmp.join("faelle"),
    };
    std::fs::create_dir_all(&konfig.faelle).unwrap();
    for (name, d) in faelle() {
        std::fs::write(
            konfig.faelle.join(format!("{name}.json")),
            serde_json::to_vec(&d).unwrap(),
        )
        .unwrap();
    }
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.join("users.json"),
        Some(konfig.audit_pfad()),
    );
    Zustand::neu(konfig, auth)
}

fn status(e: &ApiFehler) -> u16 {
    match e {
        ApiFehler::Status(s, _) => *s,
        ApiFehler::Unerwartet { .. } => 500,
    }
}

/// Was neben dem Ziel liegt: nichts außer dem Ziel selbst, kein halbes `.tmp`.
fn dateien(verzeichnis: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(verzeichnis)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn wo_einreichen_sperrt_entsteht_keine_datei() {
    let tmp = tempfile::tempdir().unwrap();
    let z = dienst(tmp.path());
    let aus = tmp.path().join("aus");
    std::fs::create_dir_all(&aus).unwrap();
    for (fall, status, grund) in [
        ("e_sperre", 409, "kapital_semantik_offen"),
        ("e_vorl", 409, "deklaration_unvollstaendig"),
    ] {
        let ziel = aus.join(format!("{fall}.xml"));
        let a = xml_exportieren(&z, fall, "BY", &ziel)
            .unwrap()
            .expect_err(fall);
        assert_eq!(
            (a.status, a.body["grund"].as_str()),
            (status, Some(grund)),
            "{fall}: {}",
            a.body
        );
        assert_eq!(a.body["eingereicht"], json!(false), "{fall}");
    }
    assert_eq!(dateien(&aus), Vec::<String>::new(), "eine Sperre schrieb doch");
    // Kein Fall, falsche Kennung: Fehler des Aufrufs, keine Antwort des Baus.
    let ziel = aus.join("x.xml");
    let e = xml_exportieren(&z, "gibt_es_nicht", "BY", &ziel).unwrap_err();
    assert_eq!(status(&e), 404, "{e:?}");
    let e = xml_exportieren(&z, "../x", "BY", &ziel).unwrap_err();
    assert_eq!(status(&e), 400, "{e:?}");
    assert_eq!(dateien(&aus), Vec::<String>::new());
}

#[test]
fn die_datei_hat_die_bytes_von_einreichen_und_den_testmerker() {
    if !elster::testhilfe::schemas_da(2025) {
        eprintln!("[xml_export] UEBERSPRUNGEN: E10-2025.xsd fehlt, TAXGRAPH_OHNE_XSD=1");
        return;
    }
    // Die öffentlich bekannte Platzhalter-ID aus den amtlichen `ERiC`-Beispielen, wie in
    // `einreichen_attrappe_hermetisch.rs`; die Soll-Datei trägt genau sie.
    std::env::set_var("ELSTER_HERSTELLER_ID", "74931");
    let tmp = tempfile::tempdir().unwrap();
    let z = dienst(tmp.path());
    let aus = tmp.path().join("aus");
    std::fs::create_dir_all(&aus).unwrap();
    let ziel = aus.join("e_ges.xml");

    let n = xml_exportieren(&z, "e_ges", "BY", &ziel)
        .unwrap()
        .unwrap_or_else(|a| panic!("{}", a.body));
    let soll = std::fs::read(wurzel().join("rust/fixtures/e2e/gesamt.xml")).unwrap();
    let ist = std::fs::read(&ziel).unwrap();
    assert_eq!(ist, soll, "die Datei ist nicht das XML, das einreichen an ERiC gibt");
    assert_eq!(n, soll.len());
    let text = String::from_utf8(ist).unwrap();
    assert_eq!(text.matches("<Testmerker>700000004</Testmerker>").count(), 1);
    // Nur der Besitzer liest: die Datei trägt Name, Anschrift und Steuernummer.
    let modus = std::fs::metadata(&ziel).unwrap().permissions().mode() & 0o777;
    assert_eq!(modus, 0o600, "{modus:o}");
    assert_eq!(dateien(&aus), vec!["e_ges.xml".to_owned()], "ein .tmp blieb liegen");

    // Eine vorhandene Datei bleibt, wie sie war: ein altes XML wird nie still ersetzt.
    let e = xml_exportieren(&z, "e_ges", "BY", &ziel).unwrap_err();
    assert_eq!(status(&e), 409, "{e:?}");
    assert_eq!(std::fs::read(&ziel).unwrap(), soll);
    assert_eq!(dateien(&aus), vec!["e_ges.xml".to_owned()]);

    // Ohne Geburtsdatum baut der Writer nichts: 422, und die Datei entsteht nicht.
    let ziel2 = aus.join("e_ohnegeb.xml");
    let a = xml_exportieren(&z, "e_ohnegeb", "BY", &ziel2)
        .unwrap()
        .expect_err("e_ohnegeb");
    assert_eq!(
        (a.status, a.body["grund"].as_str()),
        (422, Some("xml_nicht_baubar")),
        "{}",
        a.body
    );
    assert!(!ziel2.exists());
}

/// `ERiC` prüft die Datei, die `taxgraph-xml` schreibt, mit der echten Hersteller-ID: `rc=0`. Eine
/// Kontrolle mit einer beschädigten Kopie muss `rc=610001002` melden, sonst belegte das Grün nichts.
/// Nur `ERIC_VALIDIERE`, lokal, kein Netz, kein Zertifikat.
#[test]
#[ignore = "braucht die ERiC-Bibliothek und die registrierte Hersteller-ID (Umgebungs-Gate, Entscheid Julius 2026-09-12): lokal mit `cargo test -p api --test xml_export_hermetisch -- --ignored`"]
fn die_geschriebene_datei_besteht_checkest() {
    assert!(
        std::env::var("ELSTER_HERSTELLER_ID").is_ok_and(|v| !v.trim().is_empty() && v != "74931"),
        "ID fehlt: ELSTER_HERSTELLER_ID ist nicht gesetzt oder die Platzhalter-ID (ERiC prueft mit ihr nichts)"
    );
    let tmp = tempfile::tempdir().unwrap();
    let z = dienst(tmp.path());
    let ziel = tmp.path().join("e_basis.xml");
    xml_exportieren(&z, "e_basis", "BY", &ziel)
        .unwrap()
        .unwrap_or_else(|a| panic!("{}", a.body));
    let xml = std::fs::read_to_string(&ziel).unwrap();
    let (rc, antwort) = elster::validiere(xml.as_bytes(), "ESt_2025").unwrap();
    assert_eq!(rc, 0, "die geschriebene Datei besteht checkESt nicht: {antwort}");
    // Kontrolle: ohne den Nachnamen lehnt `ERiC` dieselbe Datei ab.
    let kaputt = xml.replace("<E0100201>Maier</E0100201>", "");
    // Kein `assert_ne!`: es druckte beide Texte, und der Text traegt die Hersteller-ID.
    assert!(kaputt != xml, "die Kontrolle ersetzte nichts");
    let (rc, _) = elster::validiere(kaputt.as_bytes(), "ESt_2025").unwrap();
    assert_eq!(rc, 610_001_002, "die Kontrolle haette abgelehnt werden muessen");
}

#[test]
fn das_programm_schreibt_oder_sagt_warum_nicht() {
    if !elster::testhilfe::schemas_da(2025) {
        eprintln!("[xml_export] UEBERSPRUNGEN: E10-2025.xsd fehlt, TAXGRAPH_OHNE_XSD=1");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let _ = dienst(tmp.path());
    let aus = tmp.path().join("aus");
    std::fs::create_dir_all(&aus).unwrap();
    let lauf = |fall: &str, ziel: &Path| {
        Command::new(env!("CARGO_BIN_EXE_taxgraph-xml"))
            .args(["--fall", fall, "--ausgabe"])
            .arg(ziel)
            .env("TAXGRAPH_DATEN", tmp.path())
            .env("TAXGRAPH_AUDIT_DIR", tmp.path())
            .env("ELSTER_HERSTELLER_ID", "74931")
            .output()
            .unwrap()
    };
    let soll = std::fs::read(wurzel().join("rust/fixtures/e2e/gesamt.xml")).unwrap();

    let ziel = aus.join("a.xml");
    let o = lauf("e_ges", &ziel);
    assert_eq!(o.status.code(), Some(0), "{}", String::from_utf8_lossy(&o.stderr));
    assert_eq!(std::fs::read(&ziel).unwrap(), soll);
    // Die Ausgabe nennt den Pfad und die Größe, nie die Hersteller-ID.
    let ausgabe = format!(
        "{}{}",
        String::from_utf8_lossy(&o.stdout),
        String::from_utf8_lossy(&o.stderr)
    );
    assert!(ausgabe.contains(&soll.len().to_string()), "{ausgabe}");
    assert!(!ausgabe.contains("74931"), "{ausgabe}");

    // Gesperrt: Exit 2, der Grund auf stderr, keine Datei.
    let ziel = aus.join("b.xml");
    let o = lauf("e_sperre", &ziel);
    assert_eq!(o.status.code(), Some(2), "{}", String::from_utf8_lossy(&o.stderr));
    assert!(String::from_utf8_lossy(&o.stderr).contains("kapital_semantik_offen"));
    assert!(!ziel.exists());

    // Ohne --ausgabe: Aufruffehler, Exit 1.
    let o = Command::new(env!("CARGO_BIN_EXE_taxgraph-xml"))
        .args(["--fall", "e_ges"])
        .env("TAXGRAPH_DATEN", tmp.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(1));
}
