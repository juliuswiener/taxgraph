//! Der Dienst `taxgraph-api` als Kindprozess auf Wegwerf-Daten, plus Hilfen zum Anlegen und Lesen von Faellen.
//!
//! Der Dienst laeuft mit leerer Umgebung: kein `ELSTER_HERSTELLER_ID`, kein `ERIC_DIR` (auf einen Pfad, der
//! nicht existiert), kein Sprachmodell (ausser der Test setzt den Ersatzdienst), kein Entfernungsdienst.
//! `TAXGRAPH_DATEN`, Audit-Verzeichnis und Nutzerdatei liegen in einem Wegwerf-Verzeichnis; der echte Bestand
//! `~/.local/share/taxgraph` wird nie beruehrt, `HOME` zeigt ebenfalls dorthin.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::browser::http;

pub fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Wie der Dienst gestartet wird.
#[derive(Default, Clone)]
pub struct Start {
    /// `false` (Vorgabe): `TAXGRAPH_NO_AUTH=1`, Einzelnutzer. `true`: Anmeldung ist Pflicht.
    pub auth: bool,
    /// Basis-URL des Ersatzdienstes fuer das Sprachmodell (`LLM_API_BASE`), oder leer.
    pub llm_basis: Option<String>,
    /// Zusaetzliche Umgebungsvariablen (z. B. `ERIC_DIR` der Attrappe).
    pub umgebung: Vec<(String, String)>,
}

pub struct Dienst {
    kind: Child,
    pub port: u16,
    pub basis: String,
    tmp: tempfile::TempDir,
}

impl Drop for Dienst {
    fn drop(&mut self) {
        let _ = self.kind.kill();
        let _ = self.kind.wait();
    }
}

impl Dienst {
    pub fn starte(opt: &Start) -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let daten = tmp.path().join("daten");
        std::fs::create_dir_all(daten.join("faelle")).unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_taxgraph-api"));
        cmd.arg("0")
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("HOME", tmp.path())
            .env("TAXGRAPH_ROOT", wurzel())
            .env("TAXGRAPH_DATEN", &daten)
            .env("TAXGRAPH_AUDIT_DIR", daten.join("faelle"))
            .env("TAXGRAPH_USER_STORE", daten.join("users.json"))
            .env("TAXGRAPH_JWT_SECRET", "oberflaeche-geheimnis")
            .env("TAXGRAPH_FLOW", "0")
            .env("ERIC_DIR", "/nicht/vorhanden")
            .current_dir(wurzel());
        // `lade_env_dateien` liest `.env.maps`, `.env.llm`, `.env` aus TAXGRAPH_ROOT und setzt nur Schluessel, die
        // noch FEHLEN (`var_os().is_none()`). Im Fork-Baum und in der CI gibt es diese Dateien nicht, in einem
        // Arbeitsbaum mit echten Schluesseln schon (Nr. 72, N0 in main: echter Sprachmodell-Aufruf). Leere Werte
        // zaehlen als gesetzt und sperren die Uebernahme.
        for k in ["LLM_API_BASE", "LLM_API_KEY", "LLM_MODEL", "ORS_API_KEY", "ORS_API_BASE", "ELSTER_HERSTELLER_ID", "TAXGRAPH_KI_DEBUG"] {
            cmd.env(k, "");
        }
        cmd.env("TAXGRAPH_NO_AUTH", if opt.auth { "0" } else { "1" });
        if let Some(b) = &opt.llm_basis {
            cmd.env("LLM_API_BASE", b).env("LLM_API_KEY", "platzhalter").env("LLM_MODEL", "platzhalter");
        }
        for (k, v) in &opt.umgebung {
            cmd.env(k, v);
        }
        let log = tmp.path().join("dienst.log");
        let ausgabe = std::fs::File::create(&log).unwrap();
        let kind = cmd
            .stdin(Stdio::null())
            .stdout(ausgabe.try_clone().unwrap())
            .stderr(ausgabe)
            .spawn()
            .expect("taxgraph-api startet nicht");
        let mut dienst = Self { kind, port: 0, basis: String::new(), tmp };
        let ende = Instant::now() + Duration::from_secs(60);
        loop {
            let text = std::fs::read_to_string(&log).unwrap_or_default();
            if let Some(rest) = text.split("http://127.0.0.1:").nth(1) {
                dienst.port = rest.split_whitespace().next().and_then(|p| p.parse().ok()).expect("kein Port in der Startzeile");
                break;
            }
            assert!(Instant::now() < ende, "taxgraph-api meldet keinen Port: {text}");
            assert!(dienst.kind.try_wait().unwrap().is_none(), "taxgraph-api endet beim Start: {text}");
            std::thread::sleep(Duration::from_millis(50));
        }
        dienst.basis = format!("http://127.0.0.1:{}", dienst.port);
        dienst
    }

    pub fn daten(&self) -> PathBuf {
        self.tmp.path().join("daten")
    }

    pub fn get(&self, pfad: &str) -> (u16, Value) {
        http(self.port, "GET", pfad, None, &[])
    }

    pub fn post(&self, pfad: &str, rumpf: &Value) -> (u16, Value) {
        http(self.port, "POST", pfad, Some(rumpf), &[])
    }

    /// Ein Fall, wie ihn die Kachel anlegt (`POST /fall`).
    pub fn fall(&self, scheibe: &str, fall_id: &str) -> String {
        let (status, antwort) = self.post("/fall", &json!({"scheibe": scheibe, "veranlagungszeitraum": 2025, "fall_id": fall_id}));
        assert!(status == 200 || status == 201, "POST /fall -> {status} {antwort}");
        antwort["fall_id"].as_str().unwrap_or(fall_id).to_owned()
    }

    /// Ein bestaetigter Wert vom Laien, wie ihn die Oberflaeche schreibt.
    pub fn schreibe(&self, fall: &str, feld: &str, wert: &Value) -> (u16, Value) {
        self.post(
            &format!("/fall/{fall}/event"),
            &json!({
                "feld_id": feld, "wert": wert, "zustand": "bestaetigt",
                "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                "schreiber": "ui:laie", "signal": {"signal_1": null, "signal_2": "ok"},
            }),
        )
    }

    /// Die Akte des Falls (`<daten>/faelle/<fall>.json`).
    pub fn akte(&self, fall: &str) -> Value {
        let p = self.daten().join("faelle").join(format!("{fall}.json"));
        serde_json::from_slice(&std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))).unwrap()
    }

    /// Alle Ereignisse des Falls zu `feld` (Reihenfolge der Akte).
    pub fn ereignisse(&self, fall: &str, feld: &str) -> Vec<Value> {
        self.akte(fall)["events"]
            .as_array()
            .map(|a| a.iter().filter(|e| e["feld_id"] == feld).cloned().collect())
            .unwrap_or_default()
    }

    /// Das letzte Ereignis zu `feld` oder `Null`.
    pub fn letzter_wert(&self, fall: &str, feld: &str) -> Value {
        self.ereignisse(fall, feld).last().map_or(Value::Null, |e| e["wert"].clone())
    }
}
