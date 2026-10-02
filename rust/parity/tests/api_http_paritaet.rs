//! Paritaet `rust/api` gegen `produkt/haut/server.py` ueber HTTP (Schritt 9a, Differenz-Harness).
//!
//! Zwei Server laufen nebeneinander: Python (`python3 -u produkt/haut/server.py 0`) und Rust
//! (`taxgraph-api 0`), je mit EIGENEM Tempverzeichnis (Faelle, Nutzerdatei, Audit), identischem
//! Start-Bestand (`seed_*`) und identischer Umgebung. Dieselbe Anfragefolge geht an beide;
//! verglichen werden Status, die Sicherheits-Header, der JSON-Body, das Audit- und das Fehlerlog
//! je Anfrage sowie am Ende Fall-Dateien und Nutzerdatei.
//!
//! - `handgeschrieben`: Szenarien je Route inkl. Fehlerfaelle, Auth-Modus und `TAXGRAPH_NO_AUTH=1`.
//! - `zufallsfolgen`: proptest-Folgen (Standard 1000) ueber die in 9a fertigen Routen.
//! - `negativkontrolle`: eine gestoerte Rust-Antwort MUSS als Abweichung auffallen.
//! - `dokumentierte_abweichungen`: was diese Stufe bewusst NICHT angleicht, mit Beleg.
//!
//! Stub-Routen (`501 nicht_portiert`) zaehlen nicht als Abweichung, werden aber je Route gezaehlt
//! und gedruckt. Sobald D2/D3 eine Route portieren, vergleicht der Harness sie von selbst.
//!
//! `PARITY=1 cargo test -p parity --test api_http_paritaet -- --nocapture --test-threads=1`
//! Stoerung zum Zeigen der Rotfaerbung: `PARITY_STOERUNG=1` (veraendert EINE Rust-Antwort).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    clippy::many_single_char_names
)]

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::OnceLock;
use std::time::Duration;

use auth::Auth;
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use serde_json::{json, Map, Value};

const GEHEIMNIS: &str = "paritaet-geheimnis-api-9a";

/// Jede Normalisierung, vollstaendig. Was hier nicht steht, wird roh verglichen.
const NORMALISIERUNGEN: &[(&str, &str)] = &[
    ("login.token", "JWT traegt iat und zufaelliges jti — je Anmeldung neu; Laenge und Nutzer bleiben gleich (Content-Length wird roh verglichen)"),
    ("audit.ts", "Zeitstempel der Anfrage"),
    ("audit.null", "Python schreibt fall_id/detail als null, Rust laesst fehlende Felder weg"),
    ("users.password_hash", "bcrypt-Salz ist zufaellig"),
    ("users.created_at", "Zeitstempel der Registrierung"),
    ("fehler.log", "nur Anzahl und `ort`: Typ (Python-Klasse gegen Rust-Typname), Aufrufstelle und die PII-gefilterte Fall-Kennung unterscheiden sich im Bau"),
];

// ---------------------------------------------------------------- Umgebung

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn rust_binary() -> PathBuf {
    static CELL: OnceLock<PathBuf> = OnceLock::new();
    CELL.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let status = Command::new(cargo)
            .args(["build", "-p", "api", "--bin", "taxgraph-api"])
            .current_dir(repo_root().join("rust"))
            .status()
            .unwrap();
        assert!(status.success(), "cargo build -p api schlug fehl");
        let ziel = std::env::var_os("CARGO_TARGET_DIR")
            .map_or_else(|| repo_root().join("rust/target"), PathBuf::from);
        ziel.join("debug/taxgraph-api")
    })
    .clone()
}

/// Ein laufender Server samt Datenverzeichnis.
struct Server {
    child: Child,
    port: u16,
    daten: PathBuf,
}

impl Server {
    fn faelle(&self) -> PathBuf {
        self.daten.join("faelle")
    }
    fn users(&self) -> PathBuf {
        self.daten.join("users.json")
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Startet einen Server. Die Umgebung ist vollstaendig gesetzt, damit `.env*` im Repo-Wurzelverzeichnis
/// (werden von beiden Mains geladen, nur fuer fehlende Schluessel) nichts veraendert; LLM- und
/// Karten-Schluessel sind leer, damit kein Stub-Handler in Python nach draussen telefoniert.
fn starte(art: &'static str, wurzel: &Path, no_auth: bool, seed: &Path) -> Server {
    let daten = wurzel.join(art);
    let faelle = daten.join("faelle");
    std::fs::create_dir_all(&faelle).unwrap();
    for e in std::fs::read_dir(seed).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), faelle.join(e.file_name())).unwrap();
    }
    let mut cmd = if art == "python" {
        let mut c = Command::new("python3");
        c.args(["-u", "produkt/haut/server.py", "0"]);
        c
    } else {
        let mut c = Command::new(rust_binary());
        c.arg("0");
        c
    };
    cmd.current_dir(repo_root())
        .env("TAXGRAPH_DATEN", &daten)
        .env("TAXGRAPH_AUDIT_DIR", &faelle)
        .env("TAXGRAPH_USER_STORE", daten.join("users.json"))
        .env("TAXGRAPH_JWT_SECRET", GEHEIMNIS)
        .env("TAXGRAPH_NO_AUTH", if no_auth { "1" } else { "0" })
        .env("TAXGRAPH_FLOW", "0")
        .env("TAXGRAPH_KI_DEBUG", "0")
        .env("LLM_API_KEY", "")
        .env("LLM_API_BASE", "")
        .env("LLM_MODEL", "")
        .env("ORS_API_KEY", "")
        .env_remove("XDG_DATA_HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut rd = BufReader::new(stdout);
        let mut erste = String::new();
        let _ = rd.read_line(&mut erste);
        let _ = tx.send(erste);
        let mut rest = String::new();
        while rd.read_line(&mut rest).unwrap_or(0) > 0 {
            rest.clear();
        }
    });
    let zeile = rx
        .recv_timeout(Duration::from_secs(90))
        .expect("Server meldet keinen Port");
    let port = zeile
        .split("http://127.0.0.1:")
        .nth(1)
        .and_then(|r| r.split_whitespace().next())
        .and_then(|p| p.parse().ok())
        .unwrap_or_else(|| panic!("{art}: Startzeile unlesbar: {zeile:?}"));
    Server { child, port, daten }
}

/// Der Start-Bestand beider Seiten: fremde, eigene, herrenlose, kaputte und eine Datei mit Jahr 10^38.
fn schreibe_seed(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let datei = |name: &str, inhalt: &str| {
        std::fs::write(dir.join(format!("{name}.json")), inhalt).unwrap();
    };
    let basis = |id: &str, scheibe: &str, vz: &str, besitzer: Option<&str>| {
        let user = besitzer.map_or(String::new(), |u| format!(r#","user_id":"{u}""#));
        format!(
            r#"{{"version":1,"veranlagungszeitraum":{vz},"fall_id":"{id}","scheibe":"{scheibe}","events":[],"snapshots":[]{user}}}"#
        )
    };
    datei("seed_a", &basis("seed_a", "gesamt", "2025", Some("alice")));
    datei("seed_b", &basis("seed_b", "ep", "2024", Some("bob")));
    datei("seed_o", &basis("seed_o", "an_gesamt", "2026", None));
    datei(
        "seed_big",
        &basis(
            "seed_big",
            "ep",
            "99999999999999999999999999999999999999",
            Some("alice"),
        ),
    );
    datei("seed_kaputt", "{nicht json");
}

// ---------------------------------------------------------------- HTTP

#[derive(Clone, Debug)]
struct Anfrage {
    titel: String,
    methode: String,
    pfad: String,
    kopf: Vec<(String, String)>,
    body: Option<Vec<u8>>,
}

impl Anfrage {
    fn neu(titel: &str, methode: &str, pfad: &str) -> Self {
        Self {
            titel: titel.into(),
            methode: methode.into(),
            pfad: pfad.into(),
            kopf: vec![],
            body: None,
        }
    }
    fn kopf(mut self, k: &str, v: &str) -> Self {
        self.kopf.push((k.into(), v.into()));
        self
    }
    fn token(self, t: &str) -> Self {
        self.kopf("Authorization", &format!("Bearer {t}"))
    }
    fn json(self, v: &Value) -> Self {
        self.roh(&serde_json::to_string(v).unwrap(), "application/json")
    }
    fn roh(mut self, text: &str, content_type: &str) -> Self {
        self.body = Some(text.as_bytes().to_vec());
        self.kopf("Content-Type", content_type)
    }
}

#[derive(Clone, Debug)]
struct Antwort {
    status: u16,
    kopf: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn sende(port: u16, a: &Anfrage) -> Antwort {
    let hat = |n: &str| a.kopf.iter().any(|(k, _)| k.eq_ignore_ascii_case(n));
    let mut m = format!("{} {} HTTP/1.1\r\n", a.methode, a.pfad).into_bytes();
    if !hat("host") {
        m.extend(format!("Host: 127.0.0.1:{port}\r\n").bytes());
    }
    for (k, v) in &a.kopf {
        m.extend(format!("{k}: {v}\r\n").bytes());
    }
    if let Some(b) = &a.body {
        if !hat("content-length") {
            m.extend(format!("Content-Length: {}\r\n", b.len()).bytes());
        }
    }
    m.extend(b"Connection: close\r\n\r\n");
    if let Some(b) = &a.body {
        m.extend(b);
    }
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(30))).unwrap();
    s.write_all(&m).unwrap();
    let mut roh = Vec::new();
    // Ein Server, der nach der Antwort schliesst, beendet den Lesevorgang; ein Reset nach der
    // Antwort (Rumpf nicht gelesen) ist kein Fehler.
    let _ = s.read_to_end(&mut roh);
    parse(&roh)
}

fn parse(roh: &[u8]) -> Antwort {
    let ende = roh
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or_else(|| panic!("keine Antwort: {:?}", String::from_utf8_lossy(roh)));
    let kopf_text = String::from_utf8_lossy(&roh[..ende]).into_owned();
    let mut zeilen = kopf_text.lines();
    let status = zeilen
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let kopf = zeilen
        .filter_map(|z| z.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    Antwort {
        status,
        kopf,
        body: roh[ende + 4..].to_vec(),
    }
}

// ---------------------------------------------------------------- Vergleich

#[derive(Clone, Copy, PartialEq, Eq)]
enum Modus {
    Voll,
    /// Nur der Status: die Meldung haengt an einem Parser (Python `json`, `yaml`), den Rust nicht teilt.
    NurStatus,
    /// Status, Header und Body, aber nicht das Audit: Python schreibt einen Nicht-Text-Namen roh ins
    /// Audit (`user_id: 5`), `store::audit` kennt nur Text.
    OhneAudit,
}

#[derive(Default)]
struct Stat {
    anfragen: usize,
    verglichen: usize,
    nur_status: usize,
    stubs: BTreeMap<String, usize>,
    abweichungen: Vec<String>,
    norm: BTreeMap<&'static str, usize>,
    /// Wie oft welcher Status je Anfrageart (erstes Wort des Titels) vorkam — der Beleg, dass die
    /// Folgen die Erfolgs- UND die Fehlerpfade erreichen.
    codes: BTreeMap<String, usize>,
}

/// Eine angehaengte Protokolldatei; liefert je Aufruf nur die neuen Zeilen.
struct Log {
    pfad: PathBuf,
    gelesen: usize,
}

impl Log {
    fn neu(pfad: PathBuf) -> Self {
        Self { pfad, gelesen: 0 }
    }
    fn neue(&mut self) -> Vec<Value> {
        let roh = std::fs::read(&self.pfad).unwrap_or_default();
        let neu = &roh[self.gelesen.min(roh.len())..];
        self.gelesen = roh.len();
        String::from_utf8_lossy(neu)
            .lines()
            .filter(|z| !z.is_empty())
            .map(|z| serde_json::from_str(z).unwrap_or_else(|e| panic!("Logzeile {z:?}: {e}")))
            .collect()
    }
}

fn json_body(a: &Antwort) -> Option<Value> {
    a.kopf
        .get("content-type")
        .filter(|c| c.starts_with("application/json"))?;
    serde_json::from_slice(&a.body).ok()
}

fn normiere_antwort(v: &mut Value, norm: &mut BTreeMap<&'static str, usize>) {
    if let Some(t) = v.get_mut("token").filter(|t| t.is_string()) {
        *t = json!("<TOKEN>");
        *norm.entry("login.token").or_default() += 1;
    }
}

fn vergleiche(
    py: &Antwort,
    rs: &Antwort,
    modus: Modus,
    norm: &mut BTreeMap<&'static str, usize>,
) -> Vec<String> {
    let mut d = vec![];
    if py.status != rs.status {
        d.push(format!("Status py={} rs={}", py.status, rs.status));
    }
    if modus == Modus::NurStatus {
        return d;
    }
    for k in [
        "content-type",
        "content-length",
        "content-security-policy",
        "x-content-type-options",
        "referrer-policy",
    ] {
        if py.kopf.get(k) != rs.kopf.get(k) {
            d.push(format!(
                "Kopf {k}: py={:?} rs={:?}",
                py.kopf.get(k),
                rs.kopf.get(k)
            ));
        }
    }
    match (json_body(py), json_body(rs)) {
        (Some(mut p), Some(mut r)) => {
            normiere_antwort(&mut p, norm);
            normiere_antwort(&mut r, norm);
            if p != r {
                d.push(format!("Body py={p} rs={r}"));
            }
        }
        _ if py.body != rs.body => d.push(format!(
            "Body py={:?} rs={:?}",
            String::from_utf8_lossy(&py.body),
            String::from_utf8_lossy(&rs.body)
        )),
        _ => {}
    }
    d
}

fn normiere_audit(zeilen: Vec<Value>, norm: &mut BTreeMap<&'static str, usize>) -> Vec<Value> {
    zeilen
        .into_iter()
        .map(|z| {
            *norm.entry("audit.ts").or_default() += 1;
            let f = |k: &str| z.get(k).cloned().unwrap_or(Value::Null);
            if z.get("fall_id").is_none() || z.get("detail").is_none() {
                *norm.entry("audit.null").or_default() += 1;
            }
            json!({"user_id": f("user_id"), "action": f("action"), "fall_id": f("fall_id"), "detail": f("detail")})
        })
        .collect()
}

fn orte(zeilen: &[Value]) -> Vec<Value> {
    zeilen
        .iter()
        .map(|z| z.get("ort").cloned().unwrap_or(Value::Null))
        .collect()
}

/// Was eine Datei im Fall-Verzeichnis ist: geparstes JSON (oder der Rohtext) und der Modus.
fn verzeichnis_zustand(s: &Server) -> BTreeMap<String, (Value, u32)> {
    let mut m = BTreeMap::new();
    for e in std::fs::read_dir(s.faelle()).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        if Path::new(&name).extension().is_none_or(|e| e != "json") {
            continue;
        }
        let text = std::fs::read_to_string(e.path()).unwrap();
        let wert = serde_json::from_str(&text).unwrap_or(Value::String(text));
        m.insert(
            name,
            (wert, e.metadata().unwrap().permissions().mode() & 0o777),
        );
    }
    m
}

fn nutzer_zustand(s: &Server, norm: &mut BTreeMap<&'static str, usize>) -> Value {
    let Ok(text) = std::fs::read_to_string(s.users()) else {
        return Value::Null;
    };
    let mut v: Value = serde_json::from_str(&text).unwrap();
    if let Some(users) = v.get_mut("users").and_then(Value::as_object_mut) {
        for u in users.values_mut() {
            u["password_hash"] = json!("<HASH>");
            u["created_at"] = json!("<TS>");
            *norm.entry("users.password_hash").or_default() += 1;
            *norm.entry("users.created_at").or_default() += 1;
        }
    }
    v
}

/// Python und Rust nebeneinander.
struct Paar {
    py: Server,
    rs: Server,
    logs: [Log; 4],
    stat: Stat,
    stoerung_bei: Option<usize>,
}

impl Paar {
    fn neu(wurzel: &Path, no_auth: bool, seed: &Path) -> Self {
        let py = starte("python", wurzel, no_auth, seed);
        let rs = starte("rust", wurzel, no_auth, seed);
        let logs = [
            Log::neu(py.faelle().join("audit.jsonl")),
            Log::neu(rs.faelle().join("audit.jsonl")),
            Log::neu(py.faelle().join("fehler.log")),
            Log::neu(rs.faelle().join("fehler.log")),
        ];
        let stoerung_bei = std::env::var("PARITY_STOERUNG").ok().map(|_| 7);
        Self {
            py,
            rs,
            logs,
            stat: Stat::default(),
            stoerung_bei,
        }
    }

    /// Rust zuerst: antwortet es `501 nicht_portiert`, bekommt Python die Anfrage NICHT. Sonst
    /// liefe dort die echte Logik (schreibt in den Fall, ruft Dienste) und die Verzeichnisse
    /// gingen auseinander — ohne dass einer der beiden Server falsch waere.
    fn anfrage(&mut self, a: &Anfrage, modus: Modus) {
        let mut rs = sende(self.rs.port, a);
        self.stat.anfragen += 1;
        if self.stoerung_bei == Some(self.stat.anfragen) {
            // Negativkontrolle: EINE Rust-Antwort veraendern.
            rs.status += 1;
        }
        let [py_audit, rs_audit, py_fehler, rs_fehler] = &mut self.logs;
        let (ra, rf) = (rs_audit.neue(), rs_fehler.neue());
        let stub = json_body(&rs)
            .filter(|_| rs.status == 501)
            .filter(|b| b["fehler"] == "nicht_portiert");
        if let Some(b) = stub {
            *self
                .stat
                .stubs
                .entry(b["route"].as_str().unwrap_or("?").to_owned())
                .or_default() += 1;
            return;
        }
        let art = a.titel.split_whitespace().next().unwrap_or("").to_owned();
        *self
            .stat
            .codes
            .entry(format!("{art} {}", rs.status))
            .or_default() += 1;
        let py = sende(self.py.port, a);
        let (pa, pf) = (py_audit.neue(), py_fehler.neue());
        let mut d = vergleiche(&py, &rs, modus, &mut self.stat.norm);
        if modus == Modus::NurStatus {
            self.stat.nur_status += 1;
        } else {
            self.stat.verglichen += 1;
        }
        if modus == Modus::Voll {
            let (pa, ra) = (
                normiere_audit(pa, &mut self.stat.norm),
                normiere_audit(ra, &mut self.stat.norm),
            );
            if pa != ra {
                d.push(format!("Audit py={pa:?} rs={ra:?}"));
            }
            *self.stat.norm.entry("fehler.log").or_default() += 1;
            if orte(&pf) != orte(&rf) {
                d.push(format!("Fehlerlog py={:?} rs={:?}", orte(&pf), orte(&rf)));
            }
        }
        if !d.is_empty() {
            self.stat.abweichungen.push(format!(
                "{} {} [{}]: {}",
                a.methode,
                a.pfad,
                a.titel,
                d.join("; ")
            ));
        }
    }

    fn zustand_vergleichen(&mut self, wo: &str) {
        let (p, r) = (verzeichnis_zustand(&self.py), verzeichnis_zustand(&self.rs));
        if p != r {
            let nur_p: Vec<_> = p.keys().filter(|k| !r.contains_key(*k)).collect();
            let nur_r: Vec<_> = r.keys().filter(|k| !p.contains_key(*k)).collect();
            let anders: Vec<_> = p
                .keys()
                .filter(|k| r.get(*k).is_some_and(|x| x != &p[*k]))
                .collect();
            self.stat.abweichungen.push(format!("Fall-Verzeichnis {wo}: nur Python {nur_p:?}, nur Rust {nur_r:?}, verschieden {anders:?}"));
        }
        let (pu, ru) = (
            nutzer_zustand(&self.py, &mut self.stat.norm),
            nutzer_zustand(&self.rs, &mut self.stat.norm),
        );
        if pu != ru {
            self.stat
                .abweichungen
                .push(format!("Nutzerdatei {wo}: py={pu} rs={ru}"));
        }
    }

    fn bericht(&self, titel: &str) {
        let s = &self.stat;
        let stubs: usize = s.stubs.values().sum();
        println!(
            "API-PARITAET {titel}: Anfragen {} | verglichen {} (davon nur Status {}) | Abweichungen {} | 501-Stubs nicht portiert {stubs}",
            s.anfragen, s.verglichen, s.nur_status, s.abweichungen.len()
        );
        for (route, n) in &s.stubs {
            println!("  nicht portiert: {n:4} x {route}");
        }
        let codes: Vec<String> = s.codes.iter().map(|(k, n)| format!("{k}:{n}")).collect();
        println!("  Status je Art (Rust): {}", codes.join(" "));
        println!("  Normalisierungen angewendet: {:?}", s.norm);
        for (name, grund) in NORMALISIERUNGEN
            .iter()
            .filter(|(n, _)| s.norm.contains_key(n))
        {
            println!("    {name}: {grund}");
        }
        for a in s.abweichungen.iter().take(25) {
            println!("  ABWEICHUNG {a}");
        }
    }
}

fn token(name: &str, geheimnis: &str) -> String {
    Auth::neu(geheimnis.into(), "/nie/geschrieben".into(), None)
        .stelle_aus(name)
        .unwrap()
}

// ---------------------------------------------------------------- handgeschrieben

fn handgeschrieben_auth(p: &mut Paar) {
    let alice = token("alice", GEHEIMNIS);
    let bob = token("bob", GEHEIMNIS);
    let fremd = token("alice", "ein-anderes-geheimnis");
    let json_kopf = "application/json";
    macro_rules! a {
        ($x:expr) => {
            p.anfrage(&$x, Modus::Voll)
        };
    }
    let g = |t: &str, pfad: &str| Anfrage::neu(t, "GET", pfad);
    let po = |t: &str, pfad: &str| Anfrage::neu(t, "POST", pfad);
    let de = |t: &str, pfad: &str| Anfrage::neu(t, "DELETE", pfad);

    // Betrieb und Unbekanntes
    a!(g("health", "/health"));
    a!(g("health mit Query", "/health?x=1&y=2"));
    a!(g("ready", "/ready"));
    a!(po("POST auf GET-Route", "/health"));
    a!(de("DELETE auf GET-Route", "/health"));
    a!(g("GET auf POST-Route", "/fall"));
    a!(g("GET auf DELETE-Route", "/fall/seed_a"));
    a!(g("unbekannter Pfad", "/nix"));
    a!(g("Schraegstrich am Ende", "/health/"));
    a!(g("Fall-ID mit Punkt", "/fall/a.b/stand").token(&alice));
    a!(g(
        "Fall-ID 65 Zeichen",
        &format!("/fall/{}/stand", "a".repeat(65))
    )
    .token(&alice));
    a!(g("fid mit Bindestrich", "/fall/seed_a/feld/a-b/warum").token(&alice));
    a!(g("Pfad /fall/", "/fall/"));
    a!(g("Doppel-Schraegstrich", "//health"));
    // Statische Dateien
    for (t, pfad) in [
        ("Wurzel", "/"),
        ("app.js", "/static/app.js"),
        ("graph.js", "/static/graph.js"),
        ("style.css", "/static/style.css"),
        ("graph.css", "/static/graph.css"),
        ("graph.html", "/static/graph.html"),
        ("index.html", "/static/index.html"),
        ("fehlt", "/static/nix.js"),
        ("Verzeichnis", "/static/"),
        ("Traversal", "/static/../../produkt/haut/api.py"),
        ("Traversal 2", "/static/../api.py"),
        ("absolut", "/static//etc/passwd"),
        ("Wurzel mit Query", "/?a=b"),
        ("kodiert", "/static/%2e%2e/api.py"),
    ] {
        a!(g(&format!("static {t}"), pfad));
    }
    a!(g("static mit fremdem Host", "/static/app.js").kopf("Host", "evil.example"));
    // Methoden
    for m in ["PUT", "PATCH", "HEAD", "OPTIONS", "TRACE"] {
        a!(Anfrage::neu(&format!("Methode {m}"), m, "/health"));
        a!(Anfrage::neu(&format!("Methode {m} auf /fall"), m, "/fall"));
    }
    // Host und Origin
    a!(g("Host fremd", "/health").kopf("Host", "evil.example"));
    a!(g("Host localhost", "/health").kopf("Host", "localhost:9"));
    a!(g("Host 127.0.0.1 ohne Port", "/health").kopf("Host", "127.0.0.1"));
    a!(g("Host localhost.evil", "/health").kopf("Host", "localhost.evil"));
    a!(g("Host IPv6", "/health").kopf("Host", "[::1]:80"));
    a!(po("Origin fremd", "/auth/login")
        .kopf("Origin", "http://evil.example")
        .json(&json!({})));
    a!(po("Origin null", "/auth/login")
        .kopf("Origin", "null")
        .json(&json!({})));
    a!(po("Origin localhost", "/auth/login")
        .kopf("Origin", "http://localhost:3000")
        .json(&json!({})));
    a!(po("Origin 127.0.0.1 mit Pfad", "/auth/login")
        .kopf("Origin", "http://127.0.0.1/x")
        .json(&json!({})));
    a!(po("Origin ohne Schema", "/auth/login")
        .kopf("Origin", "localhost:80")
        .json(&json!({})));
    a!(po("Origin leer", "/auth/login")
        .kopf("Origin", "")
        .json(&json!({})));
    a!(de("DELETE Origin fremd", "/fall/seed_a")
        .token(&alice)
        .kopf("Origin", "http://evil.example"));
    a!(g("GET Origin fremd ist erlaubt", "/health").kopf("Origin", "http://evil.example"));
    a!(po("Host vor Origin", "/fall")
        .kopf("Host", "evil.example")
        .kopf("Origin", "http://evil.example"));
    // Rumpf
    a!(po("zu gross", "/fall")
        .token(&alice)
        .kopf("Content-Length", "33554433")
        .kopf("Content-Type", json_kopf));
    a!(po("zu gross, unbekannter Pfad", "/nix").kopf("Content-Length", "33554433"));
    a!(po("Grenze 32 MiB ohne Rumpf bleibt unbehandelt", "/health").kopf("Content-Length", "0"));
    a!(po("kein Content-Type", "/fall")
        .token(&alice)
        .roh("{}", "text/plain"));
    a!(po("Content-Type gross", "/fall")
        .token(&alice)
        .roh("{}", "Application/JSON"));
    a!(po("Content-Type mit charset", "/auth/login").roh("{}", "application/json; charset=utf-8"));
    a!(po("kaputtes JSON", "/fall")
        .token(&alice)
        .roh("{x}", json_kopf));
    a!(po("kaputtes JSON, unbekannter Pfad", "/nix").roh("{x}", json_kopf));
    a!(po("nur Leerraum", "/auth/login").roh("   ", json_kopf));
    // NaN ist kein JSON (RFC 8259). Beide Tueren weisen es ab, gleicher Status, gleicher Wortlaut:
    // server.py mit _nur_endlich als parse_constant, dispatch.rs mit lies_koerper (serde_json).
    a!(po("NaN im Rumpf", "/auth/login").roh(r#"{"username": NaN, "password": "x"}"#, json_kopf));
    a!(po("ohne Rumpf", "/auth/login"));
    a!(po("kaputtes JSON, falscher Text-Typ", "/fall").roh("{x}", "text/plain"));
    a!(de("DELETE mit Rumpf", "/fall/nix")
        .token(&alice)
        .roh("{}", json_kopf));
    // Auth
    let reg = |n: &str, pw: &str| json!({"username": n, "password": pw});
    a!(po("register ok", "/auth/register").json(&reg("nutzer_a", "passwort123")));
    a!(po("register doppelt", "/auth/register").json(&reg("nutzer_a", "passwort123")));
    a!(po("register Name ungueltig", "/auth/register").json(&reg("1abc", "passwort123")));
    a!(po("register Name zu kurz", "/auth/register").json(&reg("ab", "passwort123")));
    a!(po("register Name Zeilenumbruch", "/auth/register").json(&reg("abc\n", "passwort123")));
    a!(po("register Passwort kurz", "/auth/register").json(&reg("nutzer_b", "kurz")));
    a!(po("register Passwort 73 Byte", "/auth/register").json(&reg("nutzer_b", &"x".repeat(73))));
    a!(po("register Passwort 129", "/auth/register").json(&reg("nutzer_b", &"x".repeat(129))));
    a!(po("register fehlt password", "/auth/register").json(&json!({"username": "nutzer_c"})));
    a!(po("register fehlt alles", "/auth/register").json(&json!({})));
    a!(po("register Name Zahl", "/auth/register")
        .json(&json!({"username": 5, "password": "passwort123"})));
    a!(po("register Passwort Zahl", "/auth/register")
        .json(&json!({"username": "nutzer_c", "password": 5})));
    a!(
        po("register Name ungueltig + Passwort Zahl", "/auth/register")
            .json(&json!({"username": "1", "password": 5}))
    );
    a!(po("register Name null", "/auth/register")
        .json(&json!({"username": null, "password": "passwort123"})));
    a!(po("register Body Liste", "/auth/register").json(&json!([])));
    a!(po("register Body Liste mit Feldnamen", "/auth/register")
        .json(&json!(["username", "password"])));
    a!(po("register Body Liste mit Liste", "/auth/register").json(&json!([[1]])));
    a!(po("register Body Text", "/auth/register").json(&json!("text")));
    a!(po("register Body Zahl", "/auth/register").json(&json!(5)));
    a!(po("register Body null", "/auth/register").json(&json!(null)));
    a!(po("register Body true", "/auth/register").json(&json!(true)));
    a!(po("login ok", "/auth/login").json(&reg("nutzer_a", "passwort123")));
    a!(po("login falsches Passwort", "/auth/login").json(&reg("nutzer_a", "falsch12345")));
    a!(po("login unbekannt", "/auth/login").json(&reg("niemand", "passwort123")));
    a!(po("login leerer Name", "/auth/login").json(&reg("", "passwort123")));
    a!(po("login fehlt username", "/auth/login").json(&json!({"password": "x"})));
    a!(po("login Name Liste", "/auth/login").json(&json!({"username": [], "password": "x"})));
    p.anfrage(
        &po("login Name Zahl", "/auth/login").json(&json!({"username": 5, "password": "x"})),
        Modus::OhneAudit,
    );
    a!(po("login Passwort Zahl, unbekannter Nutzer", "/auth/login")
        .json(&json!({"username": "niemand", "password": 5})));
    a!(po("login Body Liste", "/auth/login").json(&json!([1, 2])));
    a!(po("login Body Text", "/auth/login").json(&json!("x")));
    a!(po("login Passwort 73 Byte, unbekannt", "/auth/login")
        .json(&reg("niemand", &"x".repeat(73))));
    a!(po("login Passwort 73 Byte, Nutzer da", "/auth/login")
        .json(&reg("nutzer_a", &"x".repeat(73))));
    a!(po("logout ohne Rumpf", "/auth/logout"));
    a!(po("logout Muell-Token", "/auth/logout").json(&json!({"token": "abc"})));
    a!(po("logout Token Zahl", "/auth/logout").json(&json!({"token": 5})));
    a!(po("logout Token 0", "/auth/logout").json(&json!({"token": 0})));
    a!(po("logout Token Liste", "/auth/logout").json(&json!({"token": [1]})));
    a!(po("logout Body Liste", "/auth/logout").json(&json!([])));
    a!(po("logout Body null", "/auth/logout").json(&json!(null)));
    let wegwerf = token("wegwerf", GEHEIMNIS);
    a!(g("session vor logout", "/auth/session").token(&wegwerf));
    a!(po("logout Token", "/auth/logout").json(&json!({"token": wegwerf})));
    a!(g("session nach logout", "/auth/session").token(&wegwerf));
    let wegwerf2 = token("wegwerf2", GEHEIMNIS);
    a!(po("logout Bearer-Praefix", "/auth/logout")
        .json(&json!({"token": format!("Bearer {wegwerf2}")})));
    a!(g("session nach logout 2", "/auth/session").token(&wegwerf2));
    a!(g("session ok", "/auth/session").token(&alice));
    a!(g("session ohne Token", "/auth/session"));
    a!(g("session Muell", "/auth/session").token("abc"));
    a!(g("session fremdes Geheimnis", "/auth/session").token(&fremd));
    a!(g("session Bearer ohne Token", "/auth/session").kopf("Authorization", "Bearer"));
    a!(g("session Bearer leer", "/auth/session").kopf("Authorization", "Bearer "));
    a!(g("session ohne Bearer", "/auth/session").kopf("Authorization", &alice));
    a!(g("session zwei Leerzeichen", "/auth/session")
        .kopf("Authorization", &format!("Bearer  {alice}")));
    a!(g("session klein geschrieben", "/auth/session")
        .kopf("authorization", &format!("Bearer {alice}")));
    a!(g("session Bearer klein", "/auth/session")
        .kopf("Authorization", &format!("bearer {alice}")));
    // Fall anlegen
    let anl = |fall_id: Value, scheibe: Option<Value>, vz: Option<Value>| {
        let mut m = Map::new();
        m.insert("fall_id".into(), fall_id);
        if let Some(s) = scheibe {
            m.insert("scheibe".into(), s);
        }
        if let Some(v) = vz {
            m.insert("veranlagungszeitraum".into(), v);
        }
        Value::Object(m)
    };
    a!(po("anlegen ohne Token", "/fall").json(&anl(json!("h1"), None, None)));
    a!(po("anlegen Muell-Token", "/fall")
        .token("abc")
        .json(&anl(json!("h1"), None, None)));
    a!(po("anlegen Standard", "/fall")
        .token(&alice)
        .json(&anl(json!("h1"), None, None)));
    a!(po("anlegen doppelt", "/fall")
        .token(&alice)
        .json(&anl(json!("h1"), None, None)));
    a!(po("anlegen doppelt anderer Nutzer", "/fall")
        .token(&bob)
        .json(&anl(json!("h1"), None, None)));
    a!(po("anlegen Seed-ID", "/fall")
        .token(&bob)
        .json(&anl(json!("seed_a"), None, None)));
    for s in ["ep", "n_vor_gwg", "an_gesamt", "gesamt", "rentner_gesamt"] {
        a!(po(&format!("anlegen Scheibe {s}"), "/fall")
            .token(&alice)
            .json(&anl(
                json!(format!("s_{s}")),
                Some(json!(s)),
                Some(json!(2026))
            )));
    }
    a!(po("anlegen Scheibe unbekannt", "/fall")
        .token(&alice)
        .json(&anl(json!("h2"), Some(json!("xyz")), None)));
    a!(po("anlegen Scheibe null", "/fall").token(&alice).json(&anl(
        json!("h2"),
        Some(json!(null)),
        None
    )));
    a!(po("anlegen Scheibe Zahl", "/fall").token(&alice).json(&anl(
        json!("h2"),
        Some(json!(5)),
        None
    )));
    a!(po("anlegen Scheibe Liste", "/fall")
        .token(&alice)
        .json(&anl(json!("h2"), Some(json!(["ep"])), None)));
    a!(po("anlegen Scheibe Objekt", "/fall")
        .token(&alice)
        .json(&anl(json!("h2"), Some(json!({"a": 1})), None)));
    a!(po("anlegen Scheibe mit Apostroph", "/fall")
        .token(&alice)
        .json(&anl(json!("h2"), Some(json!("it's")), None)));
    a!(po("anlegen Scheibe leer", "/fall").token(&alice).json(&anl(
        json!("h2"),
        Some(json!("")),
        None
    )));
    for (t, vz) in [
        ("Text", json!("2025")),
        ("Text mit Leerraum", json!(" 2026 ")),
        ("Text mit Plus", json!("+2024")),
        ("Text mit Unterstrich", json!("2_025")),
        ("Float", json!(2025.7)),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("abc", json!("abc")),
        ("Liste", json!([])),
        ("Objekt", json!({})),
        ("2023", json!(2023)),
        ("0", json!(0)),
        ("negativ", json!(-5)),
        ("riesig als Text", json!("99999999999999999999")),
        ("Text -0", json!("-0")),
        ("leer", json!("")),
    ] {
        a!(po(&format!("anlegen VZ {t}"), "/fall")
            .token(&alice)
            .json(&anl(json!(format!("v_{}", t.len())), None, Some(vz))));
    }
    for (t, id) in [
        ("leer", json!("")),
        ("Leerzeichen", json!("a b")),
        ("65 Zeichen", json!("a".repeat(65))),
        ("64 Zeichen", json!("a".repeat(64))),
        ("Umlaut", json!("äöü")),
        ("Punkt", json!("a.b")),
        ("Schraegstrich", json!("../x")),
        ("Zahl", json!(123)),
        ("Zahl negativ", json!(-5)),
        ("true", json!(true)),
        ("false", json!(false)),
        ("null", json!(null)),
        ("Float", json!(1.5)),
        ("Float exp", json!(1e-5)),
        ("Liste", json!(["a"])),
        ("Objekt", json!({"x": 1})),
        ("Bindestrich", json!("a-b_c")),
        ("Zeilenumbruch", json!("a\n")),
    ] {
        a!(po(&format!("anlegen fall_id {t}"), "/fall")
            .token(&alice)
            .json(&anl(id, None, None)));
    }
    a!(po("anlegen fall_id fehlt", "/fall")
        .token(&alice)
        .json(&json!({})));
    a!(po("anlegen alles ungueltig", "/fall")
        .token(&alice)
        .json(&anl(json!("a b"), Some(json!("x")), Some(json!("y")))));
    a!(po("anlegen Scheibe + VZ ungueltig", "/fall")
        .token(&alice)
        .json(&anl(json!("ok1"), Some(json!("x")), Some(json!("y")))));
    a!(po("anlegen VZ + fall_id ungueltig", "/fall")
        .token(&alice)
        .json(&anl(json!("a b"), None, Some(json!("y")))));
    a!(po("anlegen Body Liste", "/fall")
        .token(&alice)
        .json(&json!([])));
    a!(po("anlegen Body Text", "/fall")
        .token(&alice)
        .json(&json!("x")));
    a!(po("anlegen Body Zahl", "/fall")
        .token(&alice)
        .json(&json!(5)));
    a!(po("anlegen Body null", "/fall")
        .token(&alice)
        .json(&json!(null)));
    a!(po("anlegen Body Liste ohne Token", "/fall").json(&json!([])));
    a!(po("anlegen unbekannte Felder", "/fall")
        .token(&bob)
        .json(&json!({"fall_id": "extra1", "x": 1, "user_id": "alice"})));
    // Stubs und Owner-Check
    let get_stubs = [
        "fragen",
        "stand",
        "feld/x1/warum",
        "feld/x1/frage",
        "ergebnis",
        "preflight",
        "deklaration",
        "graph",
    ];
    let post_stubs = [
        "event",
        "einreichen",
        "chat",
        "flow",
        "entfernung",
        "vorjahr",
        "kontoauszug",
    ];
    for (wer, tok) in [
        ("alice", Some(&alice)),
        ("bob", Some(&bob)),
        ("ohne", None),
        ("fremdes Geheimnis", Some(&fremd)),
    ] {
        for r in get_stubs {
            for id in ["h1", "seed_b", "seed_o", "gibtsnicht"] {
                let x = g(&format!("GET {r} als {wer}"), &format!("/fall/{id}/{r}"));
                a!(tok.map_or(x.clone(), |t| x.token(t)));
            }
        }
        for r in post_stubs {
            for id in ["h1", "seed_b", "gibtsnicht"] {
                let x =
                    po(&format!("POST {r} als {wer}"), &format!("/fall/{id}/{r}")).json(&json!({}));
                a!(tok.map_or(x.clone(), |t| x.token(t)));
            }
        }
    }
    // Ampel (P4): ohne Owner-Check
    for (t, pfad, tok) in [
        ("ohne Token", "/fall/h1/elster-ampel", None),
        ("fremder Fall", "/fall/seed_b/elster-ampel", Some(&alice)),
        ("fehlender Fall", "/fall/gibtsnicht/elster-ampel", None),
        ("ungueltige ID", "/fall/a.b/elster-ampel", None),
        ("Muell-Token", "/fall/h1/elster-ampel", Some(&fremd)),
    ] {
        let x = po(&format!("Ampel {t}"), pfad);
        a!(tok.map_or(x.clone(), |t| x.token(t)));
    }
    a!(po("Ampel kaputtes JSON", "/fall/h1/elster-ampel").roh("{x", json_kopf));
    a!(g("Ampel falsche Methode", "/fall/h1/elster-ampel"));
    // Loeschen
    a!(de("DELETE ohne Token", "/fall/h1"));
    a!(de("DELETE fremder Fall", "/fall/seed_b").token(&alice));
    a!(de("DELETE herrenlos", "/fall/seed_o").token(&alice));
    a!(de("DELETE fehlend ohne Token", "/fall/gibtsnicht"));
    a!(de("DELETE fehlend", "/fall/gibtsnicht").token(&alice));
    a!(de("DELETE ungueltige ID", "/fall/a.b").token(&alice));
    a!(de("DELETE eigener Fall", "/fall/h1").token(&alice));
    a!(de("DELETE eigener Fall erneut", "/fall/h1").token(&alice));
    a!(de("DELETE Seed eigener Fall", "/fall/seed_a").token(&alice));
    a!(de("DELETE Scheibe-Fall", "/fall/s_ep").token(&alice));
    a!(de(
        "DELETE Fall mit 64 Zeichen",
        &format!("/fall/{}", "a".repeat(64))
    )
    .token(&alice));
    a!(de(
        "DELETE Fall mit 65 Zeichen",
        &format!("/fall/{}", "a".repeat(65))
    )
    .token(&alice));
    a!(g("ready am Ende", "/ready"));
    p.zustand_vergleichen("nach den Szenarien (Auth)");
}

fn handgeschrieben_ohne_auth(p: &mut Paar) {
    let alice = token("alice", GEHEIMNIS);
    macro_rules! a {
        ($x:expr) => {
            p.anfrage(&$x, Modus::Voll)
        };
    }
    let g = |t: &str, pfad: &str| Anfrage::neu(t, "GET", pfad);
    let po = |t: &str, pfad: &str| Anfrage::neu(t, "POST", pfad);
    let de = |t: &str, pfad: &str| Anfrage::neu(t, "DELETE", pfad);
    a!(g("health", "/health"));
    a!(g("session ohne Token bleibt 401", "/auth/session"));
    a!(g("session mit Token", "/auth/session").token(&alice));
    a!(po("anlegen ohne Token", "/fall").json(&json!({"fall_id": "n1", "scheibe": "gesamt"})));
    a!(po("anlegen ohne Token doppelt", "/fall").json(&json!({"fall_id": "n1"})));
    a!(po("anlegen mit Token", "/fall")
        .token(&alice)
        .json(&json!({"fall_id": "n2"})));
    a!(po("anlegen Body Liste", "/fall").json(&json!([])));
    a!(po("anlegen ungueltig", "/fall").json(&json!({"fall_id": "a b"})));
    for r in ["fragen", "stand", "ergebnis", "graph"] {
        for id in ["n1", "n2", "seed_a", "seed_b", "seed_o", "gibtsnicht"] {
            a!(g(&format!("GET {r} ohne Auth"), &format!("/fall/{id}/{r}")));
        }
    }
    for id in ["n1", "seed_b"] {
        a!(po("POST chat ohne Auth", &format!("/fall/{id}/chat")).json(&json!({})));
        a!(po("Ampel ohne Auth", &format!("/fall/{id}/elster-ampel")));
    }
    p.anfrage(
        &g("kaputte Fall-Datei GET", "/fall/seed_kaputt/stand"),
        Modus::NurStatus,
    );
    a!(de("DELETE herrenlos ohne Token", "/fall/seed_o"));
    a!(de("DELETE fremder Fall ohne Token", "/fall/seed_b"));
    a!(de("DELETE eigener Fall mit Token", "/fall/seed_a").token(&alice));
    a!(de("DELETE erneut", "/fall/seed_a").token(&alice));
    a!(de("DELETE n1", "/fall/n1"));
    a!(de("DELETE fehlend", "/fall/gibtsnicht"));
    p.anfrage(
        &de("kaputte Fall-Datei DELETE", "/fall/seed_kaputt"),
        Modus::NurStatus,
    );
    // /ready ohne Fall-Verzeichnis: beide Seiten ihr Verzeichnis wegraeumen (Tempverzeichnis).
    p.zustand_vergleichen("vor /ready 503");
    for s in [&p.py, &p.rs] {
        std::fs::remove_dir_all(s.faelle()).unwrap();
    }
    p.anfrage(&g("ready ohne Verzeichnis", "/ready"), Modus::Voll);
    for s in [&p.py, &p.rs] {
        std::fs::create_dir_all(s.faelle()).unwrap();
    }
    // Die Protokolle liegen im weggeraeumten Verzeichnis; ihre Offsets neu beginnen lassen.
    for l in &mut p.logs {
        l.gelesen = 0;
    }
    p.anfrage(&g("ready mit Verzeichnis", "/ready"), Modus::Voll);
}

#[test]
fn handgeschrieben() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let seed = tmp.path().join("seed");
    schreibe_seed(&seed);
    let mut stats = vec![];
    for (name, no_auth, lauf) in [
        ("auth", false, handgeschrieben_auth as fn(&mut Paar)),
        ("ohne_auth", true, handgeschrieben_ohne_auth),
    ] {
        let wurzel = tmp.path().join(name);
        let mut p = Paar::neu(&wurzel, no_auth, &seed);
        lauf(&mut p);
        p.bericht(&format!("handgeschrieben/{name}"));
        stats.push((name, p.stat.abweichungen.len(), p.stat.verglichen));
    }
    assert!(
        stats
            .iter()
            .all(|(_, abw, verglichen)| *abw == 0 && *verglichen > 0),
        "Abweichungen: {stats:?}"
    );
}

// ---------------------------------------------------------------- proptest

#[derive(Clone, Copy, Debug)]
enum Wer {
    Alice,
    Bob,
    Niemand,
    Muell,
    Fremd,
}

#[derive(Clone, Debug)]
enum Op {
    Health,
    Ready,
    Session(Wer),
    Register(u8, u8),
    Login(u8, u8),
    Logout(Wer, u8),
    Anlegen(Wer, Value),
    Loeschen(Wer, u8),
    Ampel(Wer, u8),
    Stub(Wer, u8, u8),
    Unbekannt(u8),
    Grenze(u8),
}

fn wer() -> impl Strategy<Value = Wer> {
    prop_oneof![3 => Just(Wer::Alice), 3 => Just(Wer::Bob), 2 => Just(Wer::Niemand), 1 => Just(Wer::Muell), 1 => Just(Wer::Fremd)]
}

fn feld_fall_id() -> impl Strategy<Value = Option<Value>> {
    prop_oneof![
        30 => (0u8..4).prop_map(|i| Some(json!(format!("@{i}")))),
        2 => Just(None),
        1 => Just(Some(json!(""))),
        1 => Just(Some(json!("a b"))),
        1 => Just(Some(json!("a".repeat(65)))),
        1 => Just(Some(json!(123))),
        1 => Just(Some(json!(true))),
        1 => Just(Some(json!(null))),
        1 => Just(Some(json!(1.5))),
        1 => Just(Some(json!(["a"]))),
        1 => Just(Some(json!("äö"))),
        1 => Just(Some(json!("seed_a"))),
    ]
}

fn feld_scheibe() -> impl Strategy<Value = Option<Value>> {
    prop_oneof![
        8 => Just(None),
        8 => prop::sample::select(vec!["ep", "n_vor_gwg", "an_gesamt", "gesamt", "rentner_gesamt"]).prop_map(|s| Some(json!(s))),
        1 => Just(Some(json!("xyz"))),
        1 => Just(Some(json!(null))),
        1 => Just(Some(json!(5))),
        1 => Just(Some(json!([1]))),
        1 => Just(Some(json!({"a": 1}))),
        1 => Just(Some(json!(""))),
    ]
}

fn feld_vz() -> impl Strategy<Value = Option<Value>> {
    prop_oneof![
        8 => Just(None),
        8 => prop::sample::select(vec![2024, 2025, 2026]).prop_map(|v| Some(json!(v))),
        1 => Just(Some(json!(2023))),
        1 => Just(Some(json!("2025"))),
        1 => Just(Some(json!(" 2026 "))),
        1 => Just(Some(json!(2025.7))),
        1 => Just(Some(json!(true))),
        1 => Just(Some(json!(null))),
        1 => Just(Some(json!("abc"))),
        1 => Just(Some(json!([]))),
        1 => Just(Some(json!(-5))),
    ]
}

fn anlegen_body() -> impl Strategy<Value = Value> {
    let objekt =
        (feld_fall_id(), feld_scheibe(), feld_vz(), any::<bool>()).prop_map(|(f, s, v, extra)| {
            let mut m = Map::new();
            for (k, w) in [("fall_id", f), ("scheibe", s), ("veranlagungszeitraum", v)] {
                if let Some(w) = w {
                    m.insert(k.into(), w);
                }
            }
            if extra {
                m.insert("user_id".into(), json!("alice"));
            }
            Value::Object(m)
        });
    prop_oneof![
        40 => objekt,
        1 => Just(json!([])),
        1 => Just(json!("x")),
        1 => Just(json!(5)),
        1 => Just(json!(null)),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        2 => Just(Op::Health),
        1 => Just(Op::Ready),
        3 => wer().prop_map(Op::Session),
        3 => (0u8..7, 0u8..5).prop_map(|(n, p)| Op::Register(n, p)),
        2 => (0u8..7, 0u8..5).prop_map(|(n, p)| Op::Login(n, p)),
        3 => (wer(), 0u8..8).prop_map(|(w, v)| Op::Logout(w, v)),
        16 => (wer(), anlegen_body()).prop_map(|(w, b)| Op::Anlegen(w, b)),
        8 => (wer(), 0u8..8).prop_map(|(w, i)| Op::Loeschen(w, i)),
        2 => (wer(), 0u8..8).prop_map(|(w, i)| Op::Ampel(w, i)),
        5 => (wer(), 0u8..15, 0u8..8).prop_map(|(w, r, i)| Op::Stub(w, r, i)),
        3 => (0u8..12).prop_map(Op::Unbekannt),
        4 => (0u8..9).prop_map(Op::Grenze),
    ]
}

struct Ctx {
    praefix: String,
    alice: String,
    bob: String,
    muell: String,
    fremd: String,
}

impl Ctx {
    fn mit(&self, a: Anfrage, w: Wer) -> Anfrage {
        match w {
            Wer::Alice => a.token(&self.alice),
            Wer::Bob => a.token(&self.bob),
            Wer::Niemand => a,
            Wer::Muell => a.token(&self.muell),
            Wer::Fremd => a.token(&self.fremd),
        }
    }
    fn id(&self, i: u8) -> String {
        match i {
            0..=3 => format!("{}{}", self.praefix, ["a", "b", "c", "d"][usize::from(i)]),
            4 => "seed_a".into(),
            5 => "seed_o".into(),
            6 => "gibtsnicht".into(),
            _ => "a.b".into(),
        }
    }
}

const STUBS: [(&str, &str); 15] = [
    ("GET", "fragen"),
    ("GET", "stand"),
    ("GET", "feld/x1/warum"),
    ("GET", "feld/x1/frage"),
    ("GET", "ergebnis"),
    ("GET", "preflight"),
    ("GET", "deklaration"),
    ("GET", "graph"),
    ("POST", "event"),
    ("POST", "einreichen"),
    ("POST", "chat"),
    ("POST", "flow"),
    ("POST", "entfernung"),
    ("POST", "vorjahr"),
    ("POST", "kontoauszug"),
];

fn op_zu_anfrage(op: &Op, c: &Ctx) -> Anfrage {
    let namen = [
        "nutzer_a",
        "nutzer_b",
        "ab",
        "a b",
        "1abc",
        "x_langer_name_der_ueber_32_zeichen_geht",
        "gültig",
    ];
    let pws = [
        "passwort123",
        "kurz",
        &"x".repeat(129),
        &"x".repeat(73),
        "pässwort123",
    ];
    match op {
        Op::Health => Anfrage::neu("health", "GET", "/health"),
        Op::Ready => Anfrage::neu("ready", "GET", "/ready"),
        Op::Session(w) => c.mit(Anfrage::neu("session", "GET", "/auth/session"), *w),
        Op::Register(n, p) => Anfrage::neu("register", "POST", "/auth/register")
            .json(&json!({"username": namen[usize::from(*n)], "password": pws[usize::from(*p)]})),
        Op::Login(n, p) => Anfrage::neu("login", "POST", "/auth/login")
            .json(&json!({"username": namen[usize::from(*n)], "password": pws[usize::from(*p)]})),
        Op::Logout(w, v) => {
            let t = match w {
                Wer::Alice => &c.alice,
                Wer::Bob => &c.bob,
                Wer::Fremd => &c.fremd,
                _ => &c.muell,
            };
            let body = match v {
                0 => json!({"token": t}),
                1 => json!({"token": format!("Bearer {t}")}),
                2 => json!({}),
                3 => json!({"token": 5}),
                4 => json!({"token": ""}),
                5 => json!([]),
                6 => json!({"token": null}),
                _ => json!({"token": "abc"}),
            };
            Anfrage::neu("logout", "POST", "/auth/logout").json(&body)
        }
        Op::Anlegen(w, b) => {
            let mut b = b.clone();
            if let Some(Value::String(s)) = b.get_mut("fall_id") {
                if let Some(i) = s.strip_prefix('@').and_then(|r| r.parse::<u8>().ok()) {
                    *s = c.id(i);
                }
            }
            c.mit(Anfrage::neu("anlegen", "POST", "/fall"), *w).json(&b)
        }
        Op::Loeschen(w, i) => c.mit(
            Anfrage::neu("loeschen", "DELETE", &format!("/fall/{}", c.id(*i))),
            *w,
        ),
        Op::Ampel(w, i) => c.mit(
            Anfrage::neu("ampel", "POST", &format!("/fall/{}/elster-ampel", c.id(*i))),
            *w,
        ),
        Op::Stub(w, r, i) => {
            let (m, route) = STUBS[usize::from(*r)];
            let a = Anfrage::neu(
                &format!("stub {route}"),
                m,
                &format!("/fall/{}/{route}", c.id(*i)),
            );
            let a = if m == "POST" { a.json(&json!({})) } else { a };
            c.mit(a, *w)
        }
        Op::Unbekannt(v) => {
            let (m, p) = [
                ("GET", "/nix"),
                ("POST", "/fall/x"),
                ("PUT", "/fall"),
                ("DELETE", "/health"),
                ("GET", "/fall/"),
                ("GET", "/static/nix.js"),
                ("GET", "/static/../produkt/haut/api.py"),
                ("PATCH", "/health"),
                ("POST", "/auth/nix"),
                ("GET", "/health/"),
                ("GET", "/"),
                ("GET", "/static/app.js"),
            ][usize::from(*v)];
            Anfrage::neu("unbekannt", m, p)
        }
        Op::Grenze(v) => match v {
            0 => Anfrage::neu("origin fremd", "POST", "/fall")
                .token(&c.alice)
                .kopf("Origin", "http://evil.example")
                .json(&json!({"fall_id": "x"})),
            1 => Anfrage::neu("host fremd", "GET", "/health").kopf("Host", "evil.example"),
            2 => Anfrage::neu("kaputtes json", "POST", "/fall")
                .token(&c.alice)
                .roh("{x}", "application/json"),
            3 => Anfrage::neu("text/plain", "POST", "/fall")
                .token(&c.alice)
                .roh("{}", "text/plain"),
            4 => Anfrage::neu("zu gross", "POST", "/fall").kopf("Content-Length", "33554433"),
            5 => Anfrage::neu(
                "DELETE origin fremd",
                "DELETE",
                &format!("/fall/{}", c.id(0)),
            )
            .token(&c.alice)
            .kopf("Origin", "http://evil.example"),
            6 => Anfrage::neu("login leer", "POST", "/auth/login"),
            7 => Anfrage::neu("zu gross unbekannt", "POST", "/nix")
                .kopf("Content-Length", "99999999"),
            _ => {
                Anfrage::neu("kaputtes json unbekannt", "POST", "/nix").roh("{", "application/json")
            }
        },
    }
}

#[test]
fn zufallsfolgen() {
    if skip() {
        return;
    }
    let n_folgen: usize = std::env::var("PARITY_FOLGEN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1000);
    let strat = prop::collection::vec(op(), 3..10);
    let mut runner = TestRunner::deterministic();
    let folgen: Vec<Vec<Op>> = (0..n_folgen)
        .map(|_| strat.new_tree(&mut runner).unwrap().current())
        .collect();
    let tmp = tempfile::tempdir().unwrap();
    let seed = tmp.path().join("seed");
    schreibe_seed(&seed);
    let mut p = Paar::neu(tmp.path(), false, &seed);
    for (n, folge) in folgen.iter().enumerate() {
        let c = Ctx {
            praefix: format!("p{n}_"),
            alice: token("alice", GEHEIMNIS),
            bob: token("bob", GEHEIMNIS),
            muell: "abc.def.ghi".into(),
            fremd: token("alice", "ein-anderes-geheimnis"),
        };
        for o in folge {
            p.anfrage(&op_zu_anfrage(o, &c), Modus::Voll);
        }
        if (n + 1) % 100 == 0 {
            println!(
                "  Folge {}/{n_folgen}: Anfragen {}, Abweichungen {}",
                n + 1,
                p.stat.anfragen,
                p.stat.abweichungen.len()
            );
            p.zustand_vergleichen(&format!("nach Folge {}", n + 1));
        }
    }
    p.zustand_vergleichen("am Ende");
    p.bericht(&format!("zufallsfolgen ({n_folgen} Folgen)"));
    assert!(
        p.stat.abweichungen.is_empty(),
        "{} Abweichungen, erste: {:?}",
        p.stat.abweichungen.len(),
        p.stat.abweichungen.first()
    );
    assert!(
        p.stat.verglichen > 1000,
        "zu wenig verglichen: {}",
        p.stat.verglichen
    );
}

// ---------------------------------------------------------------- Wirksamkeit und Grenzen

/// Negativkontrolle: stoert `anfrage` die Rust-Antwort, MUSS eine Abweichung gemeldet werden.
#[test]
fn negativkontrolle() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let seed = tmp.path().join("seed");
    schreibe_seed(&seed);
    let mut p = Paar::neu(tmp.path(), false, &seed);
    p.stoerung_bei = Some(2);
    p.anfrage(&Anfrage::neu("health", "GET", "/health"), Modus::Voll);
    assert!(
        p.stat.abweichungen.is_empty(),
        "ungestoert muss gleich sein: {:?}",
        p.stat.abweichungen
    );
    p.anfrage(
        &Anfrage::neu("health gestoert", "GET", "/health"),
        Modus::Voll,
    );
    p.bericht("negativkontrolle");
    assert_eq!(p.stat.abweichungen.len(), 1, "die Stoerung blieb unbemerkt");
    // Zweite Sorte: Body statt Status.
    let (py, mut rs) = (
        sende(p.py.port, &Anfrage::neu("h", "GET", "/health")),
        sende(p.rs.port, &Anfrage::neu("h", "GET", "/health")),
    );
    rs.body = br#"{"flow": true, "status": "ok"}"#.to_vec();
    let mut norm = BTreeMap::new();
    assert!(
        !vergleiche(&py, &rs, Modus::Voll, &mut norm).is_empty(),
        "Body-Stoerung blieb unbemerkt"
    );
}

/// Was 9a bewusst nicht angleicht. Jede Zeile haelt fest, WIE beide Seiten abweichen; wird eine
/// Abweichung behoben, faellt der Test und die Liste im Bericht ist zu kuerzen.
#[test]
fn dokumentierte_abweichungen() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let seed = tmp.path().join("seed");
    schreibe_seed(&seed);
    let p = Paar::neu(tmp.path(), false, &seed);
    let alice = token("alice", GEHEIMNIS);
    let zweimal = |a: &Anfrage| (sende(p.py.port, a), sende(p.rs.port, a));
    // 1. Content-Length keine Zahl: hyper antwortet vor der Anwendung, leerer Body.
    let (py, rs) = zweimal(&Anfrage::neu("CL abc", "POST", "/fall").kopf("Content-Length", "abc"));
    println!(
        "  CL=abc: py={} {:?} | rs={} {:?}",
        py.status,
        String::from_utf8_lossy(&py.body),
        rs.status,
        String::from_utf8_lossy(&rs.body)
    );
    assert_eq!((py.status, rs.status), (400, 400));
    assert_ne!(py.body, rs.body);
    // 2. Jahr ausserhalb von i64 bei DELETE: Python gibt die Ganzzahl, Rust einen Float.
    let (py, rs) =
        zweimal(&Anfrage::neu("DELETE seed_big", "DELETE", "/fall/seed_big").token(&alice));
    println!(
        "  DELETE seed_big: py={} | rs={}",
        String::from_utf8_lossy(&py.body),
        String::from_utf8_lossy(&rs.body)
    );
    assert_eq!((py.status, rs.status), (200, 200));
    assert!(String::from_utf8_lossy(&py.body).contains("99999999999999999999999999999999999999"));
    assert!(!String::from_utf8_lossy(&rs.body).contains("99999999999999999999999999999999999999"));
    // 3. Login mit Nicht-Text-Passwort fuer einen vorhandenen Nutzer: Python 500 (AttributeError), Rust 401.
    let reg = Anfrage::neu("register", "POST", "/auth/register")
        .json(&json!({"username": "nutzer_z", "password": "passwort123"}));
    let _ = zweimal(&reg);
    let (py, rs) = zweimal(
        &Anfrage::neu("login Zahl", "POST", "/auth/login")
            .json(&json!({"username": "nutzer_z", "password": 5})),
    );
    println!(
        "  login Zahl als Passwort: py={} | rs={}",
        py.status, rs.status
    );
    assert_eq!((py.status, rs.status), (500, 401));
}

/// `_routes()` (`server.py:62`) und `api::routen::EINTRAEGE` stimmen in Methode, Muster und
/// Reihenfolge ueberein — Zeichen fuer Zeichen. Die Tabelle ist die Quelle fuer den Dispatch; eine
/// abweichende Zeile waere eine Route, die Python anders (oder gar nicht) kennt.
#[test]
fn routentabelle_gleich_python() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let skript = "import sys, json; sys.path.insert(0, 'produkt/haut'); import server; \
                  print(json.dumps([[m, p.pattern] for m, p, _ in server._routes()]))";
    let aus = Command::new("python3")
        .args(["-c", skript])
        .current_dir(repo_root())
        .env("TAXGRAPH_DATEN", tmp.path())
        .env("TAXGRAPH_USER_STORE", tmp.path().join("users.json"))
        .output()
        .unwrap();
    assert!(
        aus.status.success(),
        "{}",
        String::from_utf8_lossy(&aus.stderr)
    );
    let py: Vec<(String, String)> = serde_json::from_slice::<Vec<(String, String)>>(
        aus.stdout
            .split(|b| *b == b'\n')
            .rfind(|z| !z.is_empty())
            .unwrap(),
    )
    .unwrap();
    let rs: Vec<(String, String)> = api::routen::EINTRAEGE
        .iter()
        .map(|e| (e.methode.to_owned(), e.muster.to_owned()))
        .collect();
    println!(
        "API-PARITAET Routentabelle: Python {} Routen, Rust {} Routen",
        py.len(),
        rs.len()
    );
    assert_eq!(py.len(), 24);
    assert_eq!(py, rs);
}
