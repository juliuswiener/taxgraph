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
//! - `generatoren`: echte Eingaben fuer die Routen aus Stufe 1–3 (9c), Untergrenze je Route.
//! - `negativkontrolle`: eine gestoerte Rust-Antwort MUSS als Abweichung auffallen.
//! - `dokumentierte_abweichungen`: was diese Stufe bewusst NICHT angleicht, mit Beleg.
//!
//! Alle Routen sind portiert: antwortet Rust irgendwo `501 nicht_portiert`, ist das eine Abweichung
//! (`Paar::anfrage`), nie eine gezaehlte Ausnahme. Eine Liste noch nicht portierter Routen gibt es
//! nicht mehr.
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
use std::fmt::Write as _;
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

// Stub fuer Karten-Dienst und LLM samt Szenarien fuer `chat` und `entfernung` (Folge 6).
#[path = "extern_stub/mod.rs"]
mod extern_stub;
// Attrappe von `libericapi.so` und die Szenarien fuer `einreichen` (Folge 6, Stufe 2).
#[path = "eric_attrappe/mod.rs"]
mod eric_attrappe;

const GEHEIMNIS: &str = "paritaet-geheimnis-api-9a";

/// Die Uhr beider Server: abgeleitete Events und Events ohne `ts` tragen sie.
const FESTE_ZEIT: &str = "2026-01-02T03:04:05.123456+00:00";

/// Jede Normalisierung, vollstaendig. Was hier nicht steht, wird roh verglichen.
const NORMALISIERUNGEN: &[(&str, &str)] = &[
    ("login.token", "JWT traegt iat und zufaelliges jti — je Anmeldung neu; Laenge und Nutzer bleiben gleich (Content-Length wird roh verglichen)"),
    ("eric.log-pfad", "Verzeichnisname von eric.log ist zufaellig, nur die 8 Zeichen nach `eric_checkest_` werden ersetzt (Laenge und Content-Length bleiben gleich)"),
    ("audit.ts", "Zeitstempel der Anfrage"),
    ("audit.null", "Python schreibt fall_id/detail als null, Rust laesst fehlende Felder weg"),
    ("flow.ts", "Zeitstempel der Zeile; der Rest der Zeile wird als Text verglichen (Form des `ts` prueft Suite 18, `flow_paritaet`)"),
    ("users.password_hash", "bcrypt-Salz ist zufaellig"),
    ("users.created_at", "Zeitstempel der Registrierung"),
    ("fehler.log", "nur Anzahl und `ort`: Typ (Python-Klasse gegen Rust-Typname), Aufrufstelle und die PII-gefilterte Fall-Kennung unterscheiden sich im Bau"),
];

/// Stufe 1–3 (AK1 in 9c): Untergrenze der Rumpf-Erreichungen je Route im Test `generatoren`, gleich
/// der Zahl seiner Faelle, die den Rumpf erreichen sollen; faellt einer aus, wird der Test rot.
/// Stufe 4 (einreichen, chat, entfernung) steht hier nicht: sie riefe `ERiC`, das LLM oder ORS.
const UNTERGRENZE: &[(&str, usize)] = &[
    ("GET /fall/{id}/fragen", 14),
    ("GET /fall/{id}/stand", 7),
    ("GET /fall/{id}/feld/{fid}/warum", 10),
    ("GET /fall/{id}/feld/{fid}/frage", 677),
    ("GET /fall/{id}/ergebnis", 26),
    ("GET /fall/{id}/preflight", 17),
    // 12 statt 17: seit `/deklaration` bei Sperrgrund 409 antwortet (decisions/deklaration-darf-
    // verweigern), zaehlt `Stat::zaehle` (nur Pythons 2xx) die fuenf Faelle g_an, g_rent, g_rent3, g_an2
    // und g_pf_rot nicht mehr: gemessen 9 Rumpf-Erreichungen vor der Reparatur von g_dk, g_dk2 und
    // g_vz27 (die ohne Sperrgrund wieder 200 antworten) und 12 danach. Die Sperrfaelle zaehlt
    // `generatoren` eigens (`gesperrt_dk`), mit Grund und Koerper.
    ("GET /fall/{id}/deklaration", 12),
    ("GET /fall/{id}/graph", 9),
    ("POST /fall/{id}/event", 12),
    ("POST /fall/{id}/flow", 2),
    ("POST /fall/{id}/kontoauszug", 127),
    ("POST /fall/{id}/vorjahr", 17),
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
            .args([
                "build",
                "-p",
                "api",
                "--bin",
                "taxgraph-api",
                "--features",
                "festzeit",
            ])
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
/// `flow`: `TAXGRAPH_FLOW=1`, sonst antwortet `POST /flow` nur `{"mitgeschrieben": false}` und
/// `flow.jsonl` entsteht nicht. Mit `flow` vergleicht `Paar::anfrage` die neuen Zeilen von
/// `flow.jsonl` beider Server (drittes Log-Paar, `ts` normalisiert).
/// `extra`: zusaetzliche Umgebung, gewinnt gegen die Voreinstellung. Der Lauf gegen den Stub setzt
/// hier `LLM_API_BASE`, `ORS_API_BASE` und die synthetischen Schluessel (`extern_stub`).
fn starte_mit(
    art: &'static str,
    wurzel: &Path,
    no_auth: bool,
    flow: bool,
    seed: &Path,
    extra: &[(&str, &str)],
) -> Server {
    let daten = wurzel.join(art);
    let faelle = daten.join("faelle");
    std::fs::create_dir_all(&faelle).unwrap();
    for e in std::fs::read_dir(seed).unwrap() {
        let e = e.unwrap();
        std::fs::copy(e.path(), faelle.join(e.file_name())).unwrap();
    }
    let mut cmd = if art == "python" {
        let mut c = Command::new("python3");
        c.args(["-u", "rust/parity/tests/festzeit/server.py", "0"]);
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
        .env("TAXGRAPH_FLOW", if flow { "1" } else { "0" })
        .env("TAXGRAPH_KI_DEBUG", "0")
        // Pythons Set-Ausgabe (`zustand muss {…} sein`) haengt am Hash der Texte; fest, damit sie
        // vergleichbar bleibt. Rust ignoriert die Variable.
        .env("PYTHONHASHSEED", "0")
        // Feste Uhr in beiden Servern (`festzeit/server.py`, Rust-Feature `festzeit`).
        .env("TAXGRAPH_JETZT", FESTE_ZEIT)
        .env("LLM_API_KEY", "")
        .env("LLM_API_BASE", "")
        .env("LLM_MODEL", "")
        .env("ORS_API_KEY", "")
        // Nie die echte ERiC-Bibliothek mit einer echten Hersteller-ID: ohne ID baut `einreichen` kein
        // XML und ruft ERiC nie. Eine gesetzte, leere Variable gewinnt gegen `.env` (beide Server
        // setzen nur fehlende Schluessel). Die Szenarien mit Attrappe ueberschreiben beides in `extra`.
        .env("ELSTER_HERSTELLER_ID", "")
        .env("ERIC_DIR", "/nicht/vorhanden")
        .env_remove("XDG_DATA_HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (k, v) in extra {
        cmd.env(k, v);
    }
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
}

#[derive(Default)]
struct Stat {
    anfragen: usize,
    verglichen: usize,
    nur_status: usize,
    abweichungen: Vec<String>,
    norm: BTreeMap<&'static str, usize>,
    /// Wie oft welcher Status je Anfrageart (erstes Wort des Titels) vorkam — der Beleg, dass die
    /// Folgen die Erfolgs- UND die Fehlerpfade erreichen.
    codes: BTreeMap<String, usize>,
    /// Rumpf-Erreichungen je Route aus `UNTERGRENZE`, s. `Stat::zaehle`.
    erreicht: BTreeMap<String, usize>,
    /// Zeilen von `flow.jsonl`, die Python schrieb und Rust im Vergleich gegenueberstand; der Beleg,
    /// dass der Vergleich nicht zwei leere Listen sieht.
    flow_zeilen: usize,
    /// Der Status der letzten Rust-Antwort, fuer Folgen, die ihn gegen eine Erwartung pruefen.
    letzter: u16,
}

impl Stat {
    /// Zaehlt `a` als Rumpf-Erreichung, wenn es eine Route aus `UNTERGRENZE` trifft und Python 2xx
    /// antwortet, ohne den Leerpfad von `flow.melde_ui` (`TAXGRAPH_FLOW=0`). 400 und 422 zaehlen
    /// nicht: sonst erreichte ein leerer Rumpf `{}` event und vorjahr.
    fn zaehle(&mut self, a: &Anfrage, py: &Antwort) {
        let ok = (200..300).contains(&py.status)
            && json_body(py) != Some(json!({"mitgeschrieben": false}));
        if let Some(r) = route(a).filter(|r| ok && UNTERGRENZE.iter().any(|(u, _)| u == r)) {
            *self.erreicht.entry(r).or_default() += 1;
        }
    }
}

/// Die Route aus `api::routen::EINTRAEGE`, die den Pfad bedient, als `"GET /fall/{id}/stand"`.
fn route(a: &Anfrage) -> Option<String> {
    let teile: Vec<&str> = a.pfad.split('/').collect();
    api::routen::EINTRAEGE.iter().find_map(|e| {
        let muster: Vec<&str> = e.axum_pfad.split('/').collect();
        (e.methode == a.methode
            && muster.len() == teile.len()
            && muster
                .iter()
                .zip(&teile)
                .all(|(m, t)| m.starts_with('{') || m == t))
        .then(|| format!("{} {}", e.methode, e.axum_pfad))
    })
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
    /// Die neuen Zeilen als Text, unveraendert.
    fn neue_zeilen(&mut self) -> Vec<String> {
        let roh = std::fs::read(&self.pfad).unwrap_or_default();
        let neu = &roh[self.gelesen.min(roh.len())..];
        self.gelesen = roh.len();
        String::from_utf8_lossy(neu)
            .lines()
            .filter(|z| !z.is_empty())
            .map(str::to_owned)
            .collect()
    }
    fn neue(&mut self) -> Vec<Value> {
        self.neue_zeilen()
            .iter()
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
    // `detail` nennt bei leerem ERiC-Puffer den Pfad von eric.log; sein Verzeichnisname ist zufaellig
    // (`eric_checkest_` und 8 Zeichen, wie `tempfile.mkdtemp`). Die Laenge ist gleich (Content-Length).
    if let Some(Value::String(t)) = v.get_mut("detail") {
        const MARKE: &str = "eric_checkest_";
        if let Some(i) = t.find(MARKE) {
            let ende = (i + MARKE.len() + 8).min(t.len());
            if t.is_char_boundary(ende) {
                t.replace_range(i + MARKE.len()..ende, "<ID>");
                *norm.entry("eric.log-pfad").or_default() += 1;
            }
        }
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

/// `flow.jsonl` als TEXT: nur der Wert von `ts` wird ersetzt, der Rest bleibt Byte fuer Byte —
/// Schluesselreihenfolge und Zahlenschreibweise eingeschlossen, die ein geparstes `Value` (sortiert)
/// verschluckte. Eine Zeile ohne `ts` am Anfang bleibt roh und faellt im Vergleich auf.
fn normiere_flow(zeilen: Vec<String>, norm: &mut BTreeMap<&'static str, usize>) -> Vec<String> {
    zeilen
        .into_iter()
        .map(|z| {
            match z
                .strip_prefix("{\"ts\": \"")
                .and_then(|r| r.split_once("\", "))
            {
                Some((_, rest)) => {
                    *norm.entry("flow.ts").or_default() += 1;
                    format!("{{\"ts\": \"<TS>\", {rest}")
                }
                None => z,
            }
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

/// Der erste Pfad, an dem zwei JSON-Werte auseinandergehen, mit beiden Werten (gekuerzt).
fn erster_unterschied(a: &Value, b: &Value, pfad: &str) -> String {
    let kurz = |v: &Value| v.to_string().chars().take(160).collect::<String>();
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for (k, v) in x {
                match y.get(k) {
                    Some(w) if w == v => {}
                    Some(w) => return erster_unterschied(v, w, &format!("{pfad}/{k}")),
                    None => return format!("{pfad}/{k} fehlt in Rust: {}", kurz(v)),
                }
            }
            let nur: Vec<_> = y.keys().filter(|k| !x.contains_key(*k)).collect();
            format!("{pfad}: nur Rust {nur:?}")
        }
        (Value::Array(x), Value::Array(y)) => {
            for (i, (v, w)) in x.iter().zip(y).enumerate() {
                if v != w {
                    return erster_unterschied(v, w, &format!("{pfad}[{i}]"));
                }
            }
            format!("{pfad}: Laenge py={} rs={}", x.len(), y.len())
        }
        _ => format!("{pfad}: py={} rs={}", kurz(a), kurz(b)),
    }
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
    logs: [Log; 6],
    stat: Stat,
    stoerung_bei: Option<usize>,
}

impl Paar {
    fn neu(wurzel: &Path, no_auth: bool, flow: bool, seed: &Path) -> Self {
        Self::neu_mit(wurzel, no_auth, flow, seed, &[], &[])
    }

    /// Wie [`Paar::neu`], mit eigener Zusatz-Umgebung je Seite (`starte_mit`).
    fn neu_mit(
        wurzel: &Path,
        no_auth: bool,
        flow: bool,
        seed: &Path,
        extra_py: &[(&str, &str)],
        extra_rs: &[(&str, &str)],
    ) -> Self {
        let py = starte_mit("python", wurzel, no_auth, flow, seed, extra_py);
        let rs = starte_mit("rust", wurzel, no_auth, flow, seed, extra_rs);
        let logs = [
            Log::neu(py.faelle().join("audit.jsonl")),
            Log::neu(rs.faelle().join("audit.jsonl")),
            Log::neu(py.faelle().join("fehler.log")),
            Log::neu(rs.faelle().join("fehler.log")),
            Log::neu(py.faelle().join("flow.jsonl")),
            Log::neu(rs.faelle().join("flow.jsonl")),
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

    /// Rust zuerst. Antwortet es `501 nicht_portiert`, ist das eine Abweichung (alle Routen sind
    /// portiert; eine Route, die stumm wieder 501 liefert, faerbt jeden Test rot, der sie trifft), und
    /// es gibt keinen Vergleich. Rueckgabe: der JSON-Body von Python.
    fn anfrage(&mut self, a: &Anfrage, modus: Modus) -> Option<Value> {
        let mut rs = sende(self.rs.port, a);
        self.stat.anfragen += 1;
        if self.stoerung_bei == Some(self.stat.anfragen) {
            // Negativkontrolle: EINE Rust-Antwort veraendern.
            rs.status += 1;
        }
        self.stat.letzter = rs.status;
        let [py_audit, rs_audit, py_fehler, rs_fehler, py_flow, rs_flow] = &mut self.logs;
        let (ra, rf, rfl) = (rs_audit.neue(), rs_fehler.neue(), rs_flow.neue_zeilen());
        let stub = json_body(&rs)
            .filter(|_| rs.status == 501)
            .filter(|b| b["fehler"] == "nicht_portiert");
        if let Some(b) = stub {
            let r = b["route"].as_str().unwrap_or("?");
            let d = format!(
                "{} {} [{}]: 501 nicht_portiert ({r})",
                a.methode, a.pfad, a.titel
            );
            self.stat.abweichungen.push(d);
            return None;
        }
        let art = a.titel.split_whitespace().next().unwrap_or("").to_owned();
        *self
            .stat
            .codes
            .entry(format!("{art} {}", rs.status))
            .or_default() += 1;
        let py = sende(self.py.port, a);
        let (pa, pf, pfl) = (py_audit.neue(), py_fehler.neue(), py_flow.neue_zeilen());
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
            let (pfl, rfl) = (
                normiere_flow(pfl, &mut self.stat.norm),
                normiere_flow(rfl, &mut self.stat.norm),
            );
            self.stat.flow_zeilen += pfl.len();
            if pfl != rfl {
                d.push(format!("Fluss-Mitschnitt py={pfl:?} rs={rfl:?}"));
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
        self.stat.zaehle(a, &py);
        json_body(&py)
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
            let erste: Vec<String> = anders
                .iter()
                .map(|k| format!("{k}: {}", erster_unterschied(&p[*k].0, &r[*k].0, "")))
                .collect();
            self.stat.abweichungen.push(format!("Fall-Verzeichnis {wo}: nur Python {nur_p:?}, nur Rust {nur_r:?}, verschieden {anders:?}; Rechte {:?}; {erste:?}", p.iter().filter(|(k, v)| r.get(*k).is_some_and(|x| x.1 != v.1)).map(|(k, _)| k).collect::<Vec<_>>()));
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
        println!(
            "API-PARITAET {titel}: Anfragen {} | verglichen {} (davon nur Status {}) | Abweichungen {}",
            s.anfragen, s.verglichen, s.nur_status, s.abweichungen.len()
        );
        for (route, _) in UNTERGRENZE {
            let n = s.erreicht.get(*route).unwrap_or(&0);
            println!("  Rumpf erreicht: {n:4} x {route}");
        }
        println!("  flow.jsonl: {} Zeilen verglichen", s.flow_zeilen);
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
    // Ein Nicht-Text als Name steht in beiden Protokollen als `unbekannt`, nie als Rohwert oder `repr`
    // (Vault decisions/login-protokolliert-einen-nicht-text-namen-als-unbekannt). Mit Audit-Vergleich.
    a!(po("login Name Liste gefuellt", "/auth/login")
        .json(&json!({"username": ["x"], "password": "x"})));
    a!(po("login Name Zahl", "/auth/login").json(&json!({"username": 5, "password": "x"})));
    // JSON `true` ist nicht der echte, registrierbare Nutzer "True": sein `repr` darf nicht im Protokoll stehen.
    a!(po("register Name True", "/auth/register").json(&reg("True", "passwort123")));
    a!(po("login Name true", "/auth/login")
        .json(&json!({"username": true, "password": "passwort123"})));
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
    // Fall-Routen (frueher Stubs) und Owner-Check
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
                let x = po(&format!("POST {r} als {wer}"), &format!("/fall/{id}/{r}"))
                    .json(&koerper(r, "seed_a"));
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
    // Die kaputte Akte als QUELLE von `vorjahr`: 500 auf beiden Seiten, der Text der Ausnahme
    // unterscheidet sich (wie oben bei `stand`).
    p.anfrage(
        &po("vorjahr kaputte Quelle", "/fall/seed_a/vorjahr")
            .json(&json!({"vorjahr_fall_id": "seed_kaputt"})),
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
        let mut p = Paar::neu(&wurzel, no_auth, false, &seed);
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

/// Die Routen unter `/fall/{id}/` fuer `Op::Stub` (alle portiert).
const FALL_ROUTEN: [(&str, &str); 15] = [
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
            let (m, route) = FALL_ROUTEN[usize::from(*r)];
            let a = Anfrage::neu(
                &format!("stub {route}"),
                m,
                &format!("/fall/{}/{route}", c.id(*i)),
            );
            let a = if m == "POST" {
                a.json(&koerper(route, &c.id((*i + 1) % 4)))
            } else {
                a
            };
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
    let mut p = Paar::neu(tmp.path(), false, false, &seed);
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

// ---------------------------------------------------------------- Generatoren (9c)

/// Ein Event wie aus der Oberflaeche, per Klick bestaetigt. `ts` steht fest: die `event_id` ist
/// ein Hash ueber das Event (`store.py:31`) und bleibt so je Lauf gleich.
/// ponytail: nur `bestaetigt` mit Herkunft `laie`; `vorlaeufig` und weitere Herkunft beim
/// event-Port ergaenzen, sobald Rust sie verschieden behandelt.
fn ereignis(feld: &str, wert: &Value, ersetzt: Option<&str>) -> Value {
    json!({"feld_id": feld, "wert": wert, "zustand": "bestaetigt", "schreiber": "ui:paritaet",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": format!("klick@{feld}")},
        "ts": "2026-01-01T00:00:00+00:00", "ersetzt": ersetzt})
}

/// Ein Vorschlag des LLM: `vorlaeufig`, Schreiber `llm:`, Herkunft `llm_vorschlag` (Auflage A).
fn ereignis_llm(feld: &str, wert: &Value) -> Value {
    json!({"feld_id": feld, "wert": wert, "zustand": "vorlaeufig", "schreiber": "llm:paritaet",
        "herkunft": {"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": null},
        "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null})
}

/// Ein Beleg-Import: `vorlaeufig` (ein Import bestaetigt nie direkt), `signal_1` ist ein
/// Herkunfts-Objekt mit Umlaut und Euro-Zeichen im Rohtext.
fn ereignis_beleg(feld: &str, wert: &Value) -> Value {
    json!({"feld_id": feld, "wert": wert, "zustand": "vorlaeufig", "schreiber": "import:beleg",
        "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "plausibilisiert", "haftung": "nutzer"},
        "signal": {"signal_1": {"typ": "beleg", "ref": "b1", "confidence": 0.92, "roh_text": "Zinsen: 1.500,00 €"},
                   "signal_2": null},
        "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null})
}

/// Ein Event mit allen Schluesseln, jeder frei waehlbar; `ersetzt` ist `null`, `ts` fest.
#[allow(clippy::needless_pass_by_value)] // die Aufrufer reichen Wertliterale durch
fn roher_event(
    feld: &str,
    wert: Value,
    zustand: &str,
    schreiber: &str,
    herkunft: &str,
    signal_2: Option<&str>,
) -> Value {
    json!({"feld_id": feld, "wert": wert, "zustand": zustand, "schreiber": schreiber,
        "herkunft": {"herkunft": herkunft, "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "signal": {"signal_1": null, "signal_2": signal_2},
        "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null})
}

/// `basis` mit einzelnen Schluesseln ersetzt (`Some`) oder entfernt (`None`).
fn abgewandelt(mut basis: Value, aenderungen: &[(&str, Option<Value>)]) -> Value {
    let o = basis.as_object_mut().unwrap();
    for (k, v) in aenderungen {
        match v {
            Some(v) => {
                o.insert((*k).to_owned(), v.clone());
            }
            None => {
                o.remove(*k);
            }
        }
    }
    basis
}

/// Die Anfrage-Funktion von `generatoren`: Methode, Pfad, Rumpf, Antwort von Python.
type Sender<'a> = dyn FnMut(&str, &str, Option<Value>) -> Option<Value> + 'a;

/// Die Klasse einer Antwort von `POST /event`: Status, bei 422 die Auflage, bei 500 die Python-Klasse.
fn klasse(status: u16, body: Option<&Value>) -> String {
    let fehler = body.and_then(|b| b["fehler"].as_str()).unwrap_or("");
    let art = if status == 500 {
        fehler.split(':').next().map(str::to_owned)
    } else {
        fehler
            .strip_prefix("fail-closed (")
            .and_then(|r| r.split_once(')'))
            .map(|(k, _)| k.to_owned())
            .or_else(|| {
                fehler
                    .starts_with("fail-closed:")
                    .then(|| "signal_2".to_owned())
            })
    };
    art.map_or_else(|| status.to_string(), |k| format!("{status} {k}"))
}

/// Was `event_faelle` sah: je Klasse die Zahl der Antworten, und jeder Fall, dessen Status von der
/// Erwartung abwich (Beleg, dass der Fall den Zweig erreicht, den sein Name nennt).
#[derive(Default)]
struct EventBilanz {
    klassen: BTreeMap<String, usize>,
    falsch: Vec<String>,
}

impl EventBilanz {
    fn pruefe(&mut self, name: &str, soll: u16, ist: u16, body: Option<&Value>) {
        *self.klassen.entry(klasse(ist, body)).or_default() += 1;
        if soll != ist {
            self.falsch
                .push(format!("{name}: erwartet {soll}, Antwort {ist}"));
        }
    }
}

/// `POST /event` in allen Formen: die Tabelle `begleitfelder_formen.json` (je Zeile ein Fall) und
/// die Auflagen in Pythons Reihenfolge — Tuer (400/500), A, K1, F2, T, V, W, F, `signal_2`, B. Jeder
/// Fall bekommt einen frischen Fall, ausser der Folge zu B. Beide Server antworten gleich, oder
/// `Paar::anfrage` meldet es; die Erwartung (`soll`) prueft nur, dass der Fall den Zweig trifft.
fn event_faelle(a: &mut Sender, status: &std::cell::Cell<u16>) -> EventBilanz {
    let mut bilanz = EventBilanz::default();
    let neuer_fall = |a: &mut Sender, id: &str| {
        let b = json!({"fall_id": id, "scheibe": "gesamt", "veranlagungszeitraum": 2025});
        a("POST", "/fall", Some(b));
    };
    let tabelle: Value =
        serde_json::from_str(include_str!("../../fixtures/begleitfelder_formen.json")).unwrap();
    for (i, f) in tabelle["faelle"].as_array().unwrap().iter().enumerate() {
        let id = format!("g_bf{i}");
        neuer_fall(a, &id);
        let mut body = json!({"feld_id": "ep_arbeitstage", "wert": 1, "zustand": "vorlaeufig",
            "schreiber": "ui:laie"});
        for k in ["ts", "herkunft", "signal"] {
            if let Some(v) = f.get(k) {
                body[k] = v.clone();
            }
        }
        let antwort = a("POST", &format!("/fall/{id}/event"), Some(body));
        // Ohne Objekt `herkunft` mit Schluessel `herkunft` weist schon die Tuer ab (400, `api.event`),
        // nicht erst der Store (422): dort prueft die Tabelle nur `append_event`.
        let tuer = !f["herkunft"]
            .as_object()
            .is_some_and(|h| h.contains_key("herkunft"));
        let soll = match (f["python"] == "angenommen", tuer) {
            (true, _) => 201,
            (false, true) => 400,
            (false, false) => 422,
        };
        let name = format!("Tabelle {}", f["name"].as_str().unwrap());
        bilanz.pruefe(&name, soll, status.get(), antwort.as_ref());
    }

    let m = |f: &str, w: Value| ereignis(f, &w, None);
    let von = |schreiber: &str, herkunft: &str, f: &str, w: Value| {
        roher_event(f, w, "vorlaeufig", schreiber, herkunft, None)
    };
    let vl = |f: &str, w: Value| roher_event(f, w, "vorlaeufig", "ui:paritaet", "laie", None);
    let aend = abgewandelt;
    let some = Some;
    let signal = |s2: Value| json!({"signal_1": null, "signal_2": s2});
    let gross = json!(10_000_000_000_i64);
    let basis = m("ep_arbeitstage", json!(1));
    let mut faelle: Vec<(&str, Value, u16)> = vec![];
    // Tuer: Feld, Zustand, Herkunft, Schreiber, Rumpf — und ihre Reihenfolge.
    for (name, k, w, soll) in [
        ("feld_id Zahl", "feld_id", json!(5), 400),
        ("feld_id null", "feld_id", Value::Null, 400),
        ("feld_id wahr", "feld_id", json!(true), 400),
        ("feld_id Umlaut", "feld_id", json!("größe_ä"), 400),
        ("feld_id Apostroph", "feld_id", json!("a'b"), 400),
        ("feld_id Liste", "feld_id", json!([1]), 500),
        ("feld_id Objekt", "feld_id", json!({}), 500),
        (
            "feld_id Instanz ohne Gruppe",
            "feld_id",
            json!("ep_arbeitstage__2"),
            400,
        ),
        ("feld_id Instanz 0", "feld_id", json!("schulgeld__0"), 400),
        (
            "feld_id Instanz fuehrende Null",
            "feld_id",
            json!("schulgeld__02"),
            400,
        ),
        (
            "feld_id Instanz ohne Basis",
            "feld_id",
            json!("gibt_es_nicht__2"),
            400,
        ),
        (
            "feld_id Instanz mit Zeilenende",
            "feld_id",
            json!("schulgeld__2\n"),
            201,
        ),
        ("zustand foo", "zustand", json!("foo"), 400),
        ("zustand gross", "zustand", json!("Bestaetigt"), 400),
        ("zustand leer", "zustand", json!(""), 400),
        ("zustand Zahl", "zustand", json!(5), 400),
        ("zustand null", "zustand", Value::Null, 400),
        ("zustand wahr", "zustand", json!(true), 400),
        ("zustand Liste", "zustand", json!([]), 500),
        ("zustand Objekt", "zustand", json!({}), 500),
        ("herkunft Text", "herkunft", json!("laie"), 400),
        ("herkunft null", "herkunft", Value::Null, 400),
        ("herkunft Liste", "herkunft", json!([]), 400),
        ("herkunft Zahl", "herkunft", json!(5), 400),
        (
            "herkunft Achse Liste",
            "herkunft",
            json!({"herkunft": ["laie"], "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}),
            422,
        ),
        (
            "herkunft pruef_tiefe Liste",
            "herkunft",
            json!({"herkunft": "laie", "pruef_tiefe": [], "haftung": "nutzer"}),
            500,
        ),
        (
            "herkunft pruef_tiefe Objekt",
            "herkunft",
            json!({"herkunft": "laie", "pruef_tiefe": {"a": 1}, "haftung": "nutzer"}),
            500,
        ),
        (
            "herkunft pruef_tiefe null",
            "herkunft",
            json!({"herkunft": "laie", "pruef_tiefe": null, "haftung": "nutzer"}),
            422,
        ),
        (
            "herkunft haftung Zahl",
            "herkunft",
            json!({"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": 5}),
            422,
        ),
        ("herkunft leer", "herkunft", json!({}), 400),
        (
            "herkunft ohne Schluessel herkunft",
            "herkunft",
            json!({"x": 1}),
            400,
        ),
        ("schreiber leer", "schreiber", json!(""), 400),
        ("schreiber Zahl", "schreiber", json!(5), 400),
        ("schreiber null", "schreiber", Value::Null, 400),
        ("schreiber Liste", "schreiber", json!([]), 400),
        ("schreiber wahr", "schreiber", json!(true), 400),
    ] {
        faelle.push((name, aend(basis.clone(), &[(k, some(w))]), soll));
    }
    for (name, k, soll) in [
        ("feld_id fehlt", "feld_id", 400),
        ("zustand fehlt", "zustand", 400),
        ("herkunft fehlt", "herkunft", 400),
        ("schreiber fehlt", "schreiber", 400),
        ("wert fehlt", "wert", 422),
    ] {
        faelle.push((name, aend(basis.clone(), &[(k, None)]), soll));
    }
    faelle.extend([
        ("feld_id unbekannt", m("gibt_es_nicht", json!(1)), 400),
        ("Instanz mit Gruppe", m("schulgeld__2", json!(300_000)), 201),
        (
            "Instanz zweistellig",
            m("kinderbetreuungskosten__11", json!(1000)),
            201,
        ),
        (
            "Reihenfolge feld_id vor zustand",
            aend(
                m("gibt_es_nicht", json!(1)),
                &[("zustand", some(json!("foo")))],
            ),
            400,
        ),
        (
            "Reihenfolge zustand vor herkunft",
            aend(
                basis.clone(),
                &[("zustand", some(json!("foo"))), ("herkunft", None)],
            ),
            400,
        ),
        (
            "Reihenfolge herkunft vor schreiber",
            aend(basis.clone(), &[("herkunft", None), ("schreiber", None)]),
            400,
        ),
        ("Rumpf Liste", json!([1]), 500),
        ("Rumpf Text", json!("x"), 500),
        ("Rumpf null", Value::Null, 500),
        ("Rumpf Zahl", json!(5), 500),
        ("Rumpf wahr", json!(true), 500),
        ("Rumpf leeres Objekt", json!({}), 400),
        // ts: Text jeder Art; leer heisst "jetzt".
        (
            "ts leer",
            aend(basis.clone(), &[("ts", some(json!("")))]),
            201,
        ),
        (
            "ts Text",
            aend(basis.clone(), &[("ts", some(json!("gestern")))]),
            201,
        ),
        (
            "ts Umlaut und Emoji",
            aend(basis.clone(), &[("ts", some(json!("ä😀")))]),
            201,
        ),
    ]);
    // Auflage A: ein Vorschlags-Schreiber deklariert sich ehrlich, Praefix statt Gleichheit.
    let llm = |h: &str| von("llm:t", h, "ep_arbeitstage", json!(200));
    let mit_signal = |b: Value, s2: Value| aend(b, &[("signal", some(signal(s2)))]);
    let best = |b: Value| aend(b, &[("zustand", some(json!("bestaetigt")))]);
    faelle.extend([
        ("A llm falsche Herkunft", llm("laie"), 422),
        (
            "A llm bestaetigt",
            mit_signal(best(llm("llm_vorschlag")), json!("x")),
            422,
        ),
        (
            "A llm mit signal_2",
            mit_signal(llm("llm_vorschlag"), json!("x")),
            422,
        ),
        (
            "A llm signal_2 leerer Text",
            mit_signal(llm("llm_vorschlag"), json!("")),
            422,
        ),
        (
            "A llm signal_2 Zahl",
            mit_signal(llm("llm_vorschlag"), json!(5)),
            422,
        ),
        (
            "A llm ohne signal",
            aend(llm("llm_vorschlag"), &[("signal", None)]),
            201,
        ),
        (
            "A llm Alt-Herkunft",
            aend(
                llm("llm_vorschlag"),
                &[("herkunft", some(json!({"herkunft": "llm_vorschlag"})))],
            ),
            201,
        ),
        ("A llm ok", llm("llm_vorschlag"), 201),
        // `signal` ohne `signal_2`: Python legt es ohne den Schluessel ab (andere Kennung als mit `null`).
        (
            "signal nur signal_1",
            aend(
                vl("ep_arbeitstage", json!(1)),
                &[("signal", some(json!({"signal_1": 5})))],
            ),
            201,
        ),
        (
            "signal nur signal_1 null",
            aend(
                vl("ep_arbeitstage", json!(1)),
                &[("signal", some(json!({"signal_1": null})))],
            ),
            201,
        ),
        (
            "signal nur signal_2",
            aend(
                vl("ep_arbeitstage", json!(1)),
                &[("signal", some(json!({"signal_2": "x"})))],
            ),
            201,
        ),
        (
            "signal_1 Objekt ohne signal_2 (bestaetigt, dann 422)",
            aend(
                m("ep_arbeitstage", json!(1)),
                &[("signal", some(json!({"signal_1": {"a": [1.5, null]}})))],
            ),
            422,
        ),
        (
            "A llm ersetzt Text",
            aend(llm("llm_vorschlag"), &[("ersetzt", some(json!("abc")))]),
            422,
        ),
        (
            "A llm ersetzt falsch",
            aend(llm("llm_vorschlag"), &[("ersetzt", some(json!(false)))]),
            422,
        ),
        (
            "A llm ersetzt leer",
            aend(llm("llm_vorschlag"), &[("ersetzt", some(json!("")))]),
            422,
        ),
        (
            "A beleg falsche Herkunft",
            von(
                "import:beleg",
                "llm_vorschlag",
                "ep_oepnv_kosten",
                json!(100),
            ),
            422,
        ),
        (
            "A beleg ok",
            von(
                "import:beleg",
                "beleg_import",
                "ep_oepnv_kosten",
                json!(100),
            ),
            201,
        ),
        (
            "A beleg Praefix",
            von("import:beleg_xyz", "laie", "ep_oepnv_kosten", json!(100)),
            422,
        ),
        (
            "A beleg ersetzt",
            aend(
                von(
                    "import:beleg",
                    "beleg_import",
                    "ep_oepnv_kosten",
                    json!(100),
                ),
                &[("ersetzt", some(json!("x")))],
            ),
            422,
        ),
        (
            "A vorjahr falsche Herkunft",
            von("import:vorjahr", "laie", "ep_arbeitstage", json!(200)),
            422,
        ),
        (
            "A vorjahr ok",
            von("import:vorjahr", "vorjahr", "ep_arbeitstage", json!(200)),
            201,
        ),
        // Pythons `startswith`: ein Schreiber mit dem Praefix gehoert zur Klasse, auch mit Rest.
        (
            "A vorjahr Praefix",
            von("import:vorjahr_x", "laie", "ep_arbeitstage", json!(200)),
            422,
        ),
        (
            "A kontoauszug Praefix",
            von(
                "import:kontoauszug_x",
                "laie",
                "spenden_betrag",
                json!(5000),
            ),
            422,
        ),
        (
            "A llm Praefix ohne Doppelpunkt",
            von("llm", "laie", "ep_arbeitstage", json!(200)),
            201,
        ),
        (
            "T Cent langer Text (Meldung gekuerzt im Mitschnitt)",
            m("ep_oepnv_kosten", json!("x".repeat(300))),
            422,
        ),
        (
            "T Cent langer Text mit Umlauten",
            m("ep_oepnv_kosten", json!("ä".repeat(250))),
            422,
        ),
        (
            "A vorjahr bestaetigt",
            mit_signal(
                best(von(
                    "import:vorjahr",
                    "vorjahr",
                    "ep_arbeitstage",
                    json!(200),
                )),
                json!("x"),
            ),
            422,
        ),
        (
            "A vorjahr ersetzt unbekannt",
            aend(
                von("import:vorjahr", "vorjahr", "ep_arbeitstage", json!(200)),
                &[("ersetzt", some(json!("zz")))],
            ),
            422,
        ),
        (
            "A kontoauszug falsche Herkunft",
            von("import:kontoauszug", "laie", "spenden_betrag", json!(5000)),
            422,
        ),
        (
            "A kontoauszug ok",
            von(
                "import:kontoauszug",
                "kontoauszug",
                "spenden_betrag",
                json!(5000),
            ),
            201,
        ),
        (
            "A kontoauszug ersetzt",
            aend(
                von(
                    "import:kontoauszug",
                    "kontoauszug",
                    "spenden_betrag",
                    json!(5000),
                ),
                &[("ersetzt", some(json!("x")))],
            ),
            422,
        ),
        (
            "A berechnet falsche Herkunft",
            von("berechnet:maps", "laie", "ep_entfernung_km", json!(30)),
            422,
        ),
        (
            "A berechnet ok",
            von("berechnet:maps", "berechnet", "ep_entfernung_km", json!(30)),
            201,
        ),
        (
            "A berechnet bestaetigt",
            mit_signal(
                best(von(
                    "berechnet:maps",
                    "berechnet",
                    "ep_entfernung_km",
                    json!(30),
                )),
                json!("x"),
            ),
            422,
        ),
        (
            "A berechnet ersetzt unbekannt",
            aend(
                von("berechnet:maps", "berechnet", "ep_entfernung_km", json!(30)),
                &[("ersetzt", some(json!("zz")))],
            ),
            422,
        ),
        (
            "A llmx ist kein Vorschlag",
            aend(basis.clone(), &[("schreiber", some(json!("llmx")))]),
            201,
        ),
        (
            "A LLM gross ist kein Vorschlag",
            aend(basis.clone(), &[("schreiber", some(json!("LLM:foo")))]),
            201,
        ),
        (
            "A engine ist kein Vorschlag",
            aend(
                m("ep_arbeitstage", json!(5)),
                &[("schreiber", some(json!("engine:x")))],
            ),
            201,
        ),
    ]);
    // K1 (Katalog) und F2 (Betrag ab 10^10): nur fuer Vorschlags-Schreiber.
    faelle.extend([
        (
            "K1 llm Feld nur fuer Menschen",
            von(
                "llm:t",
                "llm_vorschlag",
                "kap_antrag_guenstigerpruefung",
                json!(true),
            ),
            422,
        ),
        (
            "K1 beleg fremdes Feld",
            von("import:beleg", "beleg_import", "ep_arbeitstage", json!(200)),
            422,
        ),
        (
            "K1 kontoauszug fremdes Feld",
            von(
                "import:kontoauszug",
                "kontoauszug",
                "ep_arbeitstage",
                json!(200),
            ),
            422,
        ),
        (
            "K1 maps fremdes Feld",
            von("berechnet:maps", "berechnet", "ep_arbeitstage", json!(200)),
            422,
        ),
        (
            "K1 llm Instanz-Feld",
            von("llm:t", "llm_vorschlag", "schulgeld__2", json!(300_000)),
            422,
        ),
        (
            "K1 berechnet:x im Katalog maps",
            von("berechnet:x", "berechnet", "ep_entfernung_km", json!(30)),
            201,
        ),
        (
            "F2 llm 10^10",
            von("llm:t", "llm_vorschlag", "ep_oepnv_kosten", gross.clone()),
            422,
        ),
        (
            "F2 llm -10^10",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_oepnv_kosten",
                json!(-10_000_000_000_i64),
            ),
            422,
        ),
        (
            "F2 llm knapp darunter",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_oepnv_kosten",
                json!(9_999_999_999_i64),
            ),
            201,
        ),
        (
            "F2 llm Zahl als Text",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_oepnv_kosten",
                json!("10000000000"),
            ),
            422,
        ),
        (
            "F2 llm Zahl als Text mit Leerraum",
            von("llm:t", "llm_vorschlag", "ep_oepnv_kosten", json!(" 1e10 ")),
            422,
        ),
        (
            "F2 llm inf als Text",
            von("llm:t", "llm_vorschlag", "ep_oepnv_kosten", json!("inf")),
            422,
        ),
        (
            "F2 llm nan als Text (dann T)",
            von("llm:t", "llm_vorschlag", "ep_oepnv_kosten", json!("nan")),
            422,
        ),
        (
            "F2 llm Text ohne Zahl (dann T)",
            von("llm:t", "llm_vorschlag", "ep_oepnv_kosten", json!("abc")),
            422,
        ),
        (
            "F2 llm wahr (dann T)",
            von("llm:t", "llm_vorschlag", "ep_arbeitstage", json!(true)),
            422,
        ),
        (
            "F2 llm Kommazahl (dann T)",
            von("llm:t", "llm_vorschlag", "ep_oepnv_kosten", json!(1.5)),
            422,
        ),
        (
            "F2 beleg 10^10",
            von(
                "import:beleg",
                "beleg_import",
                "ep_oepnv_kosten",
                gross.clone(),
            ),
            422,
        ),
        (
            "F2 llm int-Feld",
            von("llm:t", "llm_vorschlag", "ep_arbeitstage", gross.clone()),
            422,
        ),
        (
            "F2 llm Textfeld mit Zahl",
            von("llm:t", "llm_vorschlag", "ep_ziel_adresse", json!("1e10")),
            422,
        ),
        (
            "F2 llm Textfeld ohne Zahl",
            von("llm:t", "llm_vorschlag", "ep_ziel_adresse", json!("abc")),
            201,
        ),
        // Pythons `float()` im Text: `_` zwischen Ziffern, Ziffern jeder Schrift, Leerraum aussen.
        (
            "F2 llm Textfeld Zahl mit Unterstrich",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_ziel_adresse",
                json!("1_0000000000"),
            ),
            422,
        ),
        (
            "F2 llm Textfeld Zahl in arabischen Ziffern",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_ziel_adresse",
                json!("١٠٠٠٠٠٠٠٠٠٠٠"),
            ),
            422,
        ),
        (
            "F2 llm Textfeld Zahl mit Leerraum aussen",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_ziel_adresse",
                json!("\u{a0}1e10\u{3000}"),
            ),
            422,
        ),
        (
            "F2 llm Textfeld Exponent mit Unterstrich",
            von("llm:t", "llm_vorschlag", "ep_ziel_adresse", json!("1e1_0")),
            422,
        ),
        (
            "F2 llm Textfeld kleine Zahl mit Unterstrich",
            von("llm:t", "llm_vorschlag", "ep_ziel_adresse", json!("1_0")),
            201,
        ),
        (
            "F2 llm Textfeld doppelter Unterstrich",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_ziel_adresse",
                json!("1__0000000000"),
            ),
            201,
        ),
        (
            "F2 llm Textfeld Zahl hinter Informationstrenner",
            von(
                "llm:t",
                "llm_vorschlag",
                "ep_ziel_adresse",
                json!("1e10\u{1f}"),
            ),
            422,
        ),
        (
            "F2 Mensch darf 10^10",
            m("ep_oepnv_kosten", gross.clone()),
            201,
        ),
    ]);
    // T (Typ), V (Vorzeichen), W (Bereich), F (Format) auf dem Wert eines Menschen.
    faelle.extend([
        ("T Cent als Text", m("ep_oepnv_kosten", json!("50000")), 422),
        ("T Cent Kommazahl", m("ep_oepnv_kosten", json!(1.5)), 422),
        (
            "T Cent ganze Kommazahl",
            m("ep_oepnv_kosten", json!(100.0)),
            422,
        ),
        ("T Cent wahr", m("ep_oepnv_kosten", json!(true)), 422),
        ("T Cent null", m("ep_oepnv_kosten", Value::Null), 422),
        (
            "T Cent Liste",
            m("ep_oepnv_kosten", json!([1, {"a": 2.5}])),
            422,
        ),
        (
            "T Cent Objekt",
            m("ep_oepnv_kosten", json!({"b": "x", "a": null})),
            422,
        ),
        (
            "T Cent Text mit Apostroph",
            m("ep_oepnv_kosten", json!("it's")),
            422,
        ),
        (
            "T Cent Text mit Anfuehrungszeichen",
            m("ep_oepnv_kosten", json!("a'\"b")),
            422,
        ),
        (
            "T Cent Emoji",
            m("ep_oepnv_kosten", json!("😀\u{a0}\t")),
            422,
        ),
        ("T Int als Text", m("ep_arbeitstage", json!("5")), 422),
        (
            "T Aufzaehlung falsch",
            m("veranlagung", json!("gemeinsam")),
            422,
        ),
        ("T Aufzaehlung ok", m("veranlagung", json!("einzel")), 201),
        ("T Aufzaehlung Zahl", m("veranlagung", json!(5)), 422),
        (
            "T Aufzaehlung Zahl als Wert",
            m("ep_ziel_des_weges", json!(1)),
            422,
        ),
        (
            "T Aufzaehlung Text ok",
            m("ep_ziel_des_weges", json!("2")),
            201,
        ),
        (
            "T Aufzaehlung Liste",
            m("veranlagung", json!(["einzel"])),
            422,
        ),
        (
            "T Wahrheitswert als Text",
            m("vv_wohnzwecke", json!("true")),
            422,
        ),
        (
            "T Wahrheitswert als Zahl",
            m("vv_wohnzwecke", json!(1)),
            422,
        ),
        ("T Wahrheitswert ok", m("vv_wohnzwecke", json!(true)), 201),
        (
            "T Datum ISO",
            m("stammdaten_geburtsdatum", json!("1990-01-01")),
            422,
        ),
        (
            "T Datum ok",
            m("stammdaten_geburtsdatum", json!("01.01.1990")),
            201,
        ),
        (
            "T Datum mit Zeilenende",
            m("stammdaten_geburtsdatum", json!("01.01.1990\n")),
            422,
        ),
        (
            "T Datum arabische Ziffern",
            m("stammdaten_geburtsdatum", json!("٠١.٠١.١٩٩٠")),
            422,
        ),
        ("T Text leer", m("ep_ziel_adresse", json!("")), 422),
        (
            "T Text mit Steuerzeichen",
            m("ep_ziel_adresse", json!("a\u{1}b")),
            422,
        ),
        (
            "T Text mit Emoji",
            m("ep_ziel_adresse", json!("München 😀")),
            201,
        ),
        ("T Text Zahl", m("ep_ziel_adresse", json!(5)), 422),
        ("T Text Liste", m("ep_ziel_adresse", json!(["a"])), 422),
        (
            "T Grad der Behinderung Zwischenwert",
            m("kind_grad_der_behinderung", json!(33)),
            422,
        ),
        (
            "T Grad der Behinderung ok",
            m("kind_grad_der_behinderung", json!(40)),
            201,
        ),
        (
            "T Grad der Behinderung 0",
            m("kind_grad_der_behinderung", json!(0)),
            201,
        ),
        (
            "T Grad der Behinderung darueber",
            m("kind_grad_der_behinderung", json!(105)),
            422,
        ),
        (
            "T Grad der Behinderung negativ",
            m("kind_grad_der_behinderung", json!(-5)),
            422,
        ),
        ("V Cent negativ", m("ep_oepnv_kosten", json!(-1)), 422),
        (
            "V Cent negativ gross",
            m("kap_kapitalertraege", json!(-100_000_000_000_000_i64)),
            422,
        ),
        (
            "V Verlustfeld negativ",
            m("kap_verlust_aktien", json!(-5)),
            422,
        ),
        ("V Int negativ", m("ep_entfernung_km", json!(-1)), 422),
        ("V Null", m("ep_oepnv_kosten", json!(0)), 201),
        (
            "V Feld ohne Verbot",
            m("bruttoarbeitslohn", json!(-100)),
            201,
        ),
        (
            "V Int-Feld ohne Verbot",
            m("vpf_monate_am_ort", json!(-1)),
            201,
        ),
        (
            "W Arbeitstage darueber",
            m("ep_arbeitstage", json!(367)),
            422,
        ),
        ("W Arbeitstage negativ", m("ep_arbeitstage", json!(-1)), 422),
        ("W Arbeitstage Null", m("ep_arbeitstage", json!(0)), 201),
        (
            "W Arbeitstage Obergrenze",
            m("ep_arbeitstage", json!(366)),
            201,
        ),
        (
            "W Geburtsjahr darunter",
            m("geburtsjahr_partner", json!(1899)),
            422,
        ),
        (
            "W Geburtsjahr darueber",
            m("geburtsjahr_partner", json!(2011)),
            422,
        ),
        (
            "W Geburtsjahr Null",
            m("geburtsjahr_partner", json!(0)),
            201,
        ),
        (
            "W Geburtsjahr Untergrenze",
            m("geburtsjahr_partner", json!(1900)),
            201,
        ),
        ("W Kinder darueber", m("fam_anzahl_kinder", json!(21)), 422),
        (
            "W Kinder ok mit Ableitung",
            m("fam_anzahl_kinder", json!(2)),
            201,
        ),
        (
            "W Kinder Null mit Ableitung",
            m("fam_anzahl_kinder", json!(0)),
            201,
        ),
        (
            "W Quote darueber",
            m("vv_entgelt_quote_prozent", json!(101)),
            422,
        ),
        (
            "W Versorgungsbeginn darunter",
            m("versorgung_beginn_jahr", json!(1954)),
            422,
        ),
        ("V vor W", m("ep_entfernung_km", json!(-400)), 422),
        (
            "F Zeitraum mit Tippfehler",
            m("kind_betreuung_zeitraum", json!("01.01-31.122")),
            422,
        ),
        (
            "F Zeitraum ok",
            m("kind_betreuung_zeitraum", json!("01.01-31.12")),
            201,
        ),
        (
            "F Zeitraum Tag 32",
            m("kind_betreuung_zeitraum", json!("32.01-31.12")),
            422,
        ),
        (
            "F Kennung zu kurz",
            m("kind_idnr", json!("1234567890")),
            422,
        ),
        ("F Kennung ok", m("kind_idnr", json!("12345678901")), 201),
        (
            "F Kennung mit Zeilenende",
            m("kind_idnr", json!("12345678901\n")),
            422,
        ),
        (
            "F Kennung arabische Ziffern",
            m("kind_idnr", json!("١٢٣٤٥٦٧٨٩٠١")),
            422,
        ),
    ]);
    // `signal_2` (Text oder null) und `bestaetigt` (braucht ein signal_2 mit Inhalt).
    for (name, s2) in [
        ("signal_2 Zahl", json!(5)),
        ("signal_2 wahr", json!(true)),
        ("signal_2 Liste", json!([1])),
        ("signal_2 Objekt", json!({})),
        ("signal_2 Kommazahl", json!(1.5)),
    ] {
        faelle.push((name, mit_signal(vl("ep_arbeitstage", json!(1)), s2), 422));
    }
    for (name, s2) in [
        ("bestaetigt signal_2 leer", json!("")),
        ("bestaetigt signal_2 Leerzeichen", json!("  ")),
        ("bestaetigt signal_2 Tabulator", json!("\t\n")),
        (
            "bestaetigt signal_2 geschuetztes Leerzeichen",
            json!("\u{a0}"),
        ),
        (
            "bestaetigt signal_2 Informationstrenner",
            json!("\u{1c}\u{1f}"),
        ),
        ("bestaetigt signal_2 Nullbreite", json!("\u{200b}")),
        ("bestaetigt signal_2 null", Value::Null),
    ] {
        faelle.push((
            name,
            mit_signal(m("ep_arbeitstage", json!(1)), s2.clone()),
            if name.ends_with("Nullbreite") {
                201
            } else {
                422
            },
        ));
    }
    faelle.push((
        "bestaetigt ohne signal",
        aend(m("ep_arbeitstage", json!(1)), &[("signal", None)]),
        422,
    ));
    faelle.push((
        "bestaetigt signal null",
        aend(
            m("ep_arbeitstage", json!(1)),
            &[("signal", some(Value::Null))],
        ),
        422,
    ));
    for (i, (name, body, soll)) in faelle.iter().enumerate() {
        let id = format!("g_m{i}");
        neuer_fall(a, &id);
        let antwort = a("POST", &format!("/fall/{id}/event"), Some(body.clone()));
        bilanz.pruefe(name, *soll, status.get(), antwort.as_ref());
    }

    // B: ein aktives Event je Feld, `ersetzt` nur mit gueltigem Ziel. Eine Folge auf einem Fall;
    // die Kennungen stehen in den Meldungen und sind je Server gleich (feste Uhr).
    neuer_fall(a, "g_evb");
    let pfad = "/fall/g_evb/event";
    let mut senden = |name: &str, body: Value, soll: u16, bilanz: &mut EventBilanz| {
        let antwort = a("POST", pfad, Some(body));
        bilanz.pruefe(name, soll, status.get(), antwort.as_ref());
        antwort.and_then(|b| b["event_id"].as_str().map(str::to_owned))
    };
    let id1 = senden(
        "B erstes Event",
        m("ep_arbeitstage", json!(200)),
        201,
        &mut bilanz,
    )
    .unwrap();
    let id2 = senden(
        "B anderes Feld",
        m("ep_oepnv_kosten", json!(100)),
        201,
        &mut bilanz,
    )
    .unwrap();
    senden(
        "B schon aktiv",
        m("ep_arbeitstage", json!(201)),
        422,
        &mut bilanz,
    );
    let mit = |ersetzt: Value| {
        aend(
            m("ep_arbeitstage", json!(210)),
            &[("ersetzt", some(ersetzt))],
        )
    };
    for (name, ersetzt) in [
        ("B Ziel unbekannt", json!("nope")),
        ("B Ziel Zahl", json!(5)),
        ("B Ziel wahr", json!(true)),
        ("B Ziel falsch", json!(false)),
        ("B Ziel leer", json!("")),
        ("B Ziel Liste", json!([1, "a"])),
        ("B Ziel Objekt", json!({"a": 1.5, "b": null})),
        ("B Ziel in Grossbuchstaben", json!(id1.to_uppercase())),
        ("B Ziel mit Leerzeichen", json!(format!(" {id1}"))),
        ("B Ziel anderes Feld", json!(id2)),
    ] {
        senden(name, mit(ersetzt), 422, &mut bilanz);
    }
    let id3 = senden("B Ersetzung", mit(json!(id1)), 201, &mut bilanz).unwrap();
    senden("B schon ersetzt", mit(json!(id1)), 422, &mut bilanz);
    senden(
        "B wieder ohne Ziel",
        m("ep_arbeitstage", json!(220)),
        422,
        &mut bilanz,
    );
    senden(
        "B llm ersetzt aktives",
        aend(llm("llm_vorschlag"), &[("ersetzt", some(json!(id3)))]),
        422,
        &mut bilanz,
    );
    let id4 = senden(
        "B Entfernung",
        m("ep_entfernung_km", json!(30)),
        201,
        &mut bilanz,
    )
    .unwrap();
    senden(
        "B berechnet ersetzt aktives",
        aend(
            von("berechnet:maps", "berechnet", "ep_entfernung_km", json!(40)),
            &[("ersetzt", some(json!(id4)))],
        ),
        201,
        &mut bilanz,
    );
    senden(
        "B vorjahr ersetzt aktives",
        aend(
            von("import:vorjahr", "vorjahr", "ep_oepnv_kosten", json!(7)),
            &[("ersetzt", some(json!(id2)))],
        ),
        201,
        &mut bilanz,
    );
    senden(
        "B Kinder",
        m("fam_anzahl_kinder", json!(2)),
        201,
        &mut bilanz,
    );
    senden(
        "B abgeleitetes Feld ist aktiv",
        m("kein_kind", json!(true)),
        422,
        &mut bilanz,
    );
    // Rohtext-Fall fuer `generatoren`: Zahlenschreibweisen in `wert` und `signal_1`.
    neuer_fall(a, "g_evr");
    bilanz
}

/// base64 (RFC 4648, mit Auffuellung); der Test hat kein `base64`-Crate.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut aus = String::new();
    for gruppe in bytes.chunks(3) {
        let teil = |i: usize| u32::from(gruppe.get(i).copied().unwrap_or(0));
        let n = (teil(0) << 16) | (teil(1) << 8) | teil(2);
        for i in 0..4 {
            if i <= gruppe.len() {
                aus.push(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                aus.push('=');
            }
        }
    }
    aus
}

/// Ein PDF mit einer Seite und einer Textzeile je Eintrag (Textlayer; `pdftotext` liest es ohne OCR).
/// Es hat keine Verweistabelle; `pdftotext` baut sie neu auf.
fn pdf_mit_zeilen(zeilen: &[&str]) -> Vec<u8> {
    let mut inhalt = String::from("BT /F1 12 Tf 72 720 Td 14 TL\n");
    for z in zeilen {
        let _ = writeln!(inhalt, "({z}) Tj T*");
    }
    inhalt.push_str("ET");
    let objekte = [
        "<</Type/Catalog/Pages 2 0 R>>".to_owned(),
        "<</Type/Pages/Kids[3 0 R]/Count 1>>".to_owned(),
        "<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>"
            .to_owned(),
        format!("<</Length {}>>\nstream\n{inhalt}\nendstream", inhalt.len()),
        "<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>".to_owned(),
    ];
    let mut aus = String::from("%PDF-1.4\n");
    for (i, o) in objekte.iter().enumerate() {
        let _ = write!(aus, "{} 0 obj\n{o}\nendobj\n", i + 1);
    }
    aus.push_str("trailer\n<</Root 1 0 R/Size 6>>\n%%EOF\n");
    aus.into_bytes()
}

/// Der Ausgangszustand eines Kontoauszug-Falls: Scheibe und Events, die vorher im Fall stehen.
struct Vorlage {
    scheibe: &'static str,
    vorher: Vec<Value>,
}

impl Vorlage {
    fn gesamt() -> Self {
        Self {
            scheibe: "gesamt",
            vorher: vec![],
        }
    }
}

/// Der Lauf von `kontoauszug_faelle` und `vorjahr_faelle`: Sender, Status der letzten Antwort, Bilanz,
/// laufende Nummer, die Route (`kontoauszug`, `vorjahr`) und das Kuerzel der Fall-Kennungen.
struct Lauf<'a, 'b> {
    a: &'a mut Sender<'b>,
    status: &'a std::cell::Cell<u16>,
    bilanz: EventBilanz,
    nr: usize,
    route: &'static str,
    kurz: &'static str,
}

impl Lauf<'_, '_> {
    /// Ein Rumpf an einem frischen Fall der Scheibe `gesamt`.
    fn lauf(&mut self, name: &str, body: Value, soll: u16, erwartet: &[(&str, Value)]) {
        self.lauf_in(name, &Vorlage::gesamt(), body, soll, erwartet);
    }

    /// Ein Rumpf an einem frischen Fall der `vorlage`.
    fn lauf_in(
        &mut self,
        name: &str,
        vorlage: &Vorlage,
        body: Value,
        soll: u16,
        erwartet: &[(&str, Value)],
    ) {
        let id = self.fall(name, vorlage);
        self.post(name, &id, body, soll, erwartet);
    }

    /// Ein frischer Fall der `vorlage` (Jahr 2025) mit ihren Events; liefert die Kennung.
    fn fall(&mut self, name: &str, vorlage: &Vorlage) -> String {
        self.nr += 1;
        let id = format!("g_{}{}", self.kurz, self.nr);
        let b = json!({"fall_id": id, "scheibe": vorlage.scheibe, "veranlagungszeitraum": 2025});
        (self.a)("POST", "/fall", Some(b));
        for e in &vorlage.vorher {
            (self.a)("POST", &format!("/fall/{id}/event"), Some(e.clone()));
            if self.status.get() != 201 {
                self.bilanz.falsch.push(format!(
                    "{name}: Vorbereitung {e} wurde mit {} abgewiesen",
                    self.status.get()
                ));
            }
        }
        id
    }

    /// Ein Rumpf an die Route des Falls `id`. Beide Server antworten gleich, oder `Paar::anfrage`
    /// meldet es; `soll` und `erwartet` (Schluessel der Antwort) pruefen nur, dass der Fall den Zweig
    /// trifft, den sein Name nennt.
    fn post(&mut self, name: &str, id: &str, body: Value, soll: u16, erwartet: &[(&str, Value)]) {
        let antwort = (self.a)("POST", &format!("/fall/{id}/{}", self.route), Some(body));
        self.bilanz
            .pruefe(name, soll, self.status.get(), antwort.as_ref());
        for (k, v) in erwartet {
            let ist = antwort.as_ref().map(|b| &b[*k]);
            if ist != Some(v) {
                self.bilanz.falsch.push(format!(
                    "{name}: {k} erwartet {v}, Antwort {}",
                    antwort
                        .as_ref()
                        .map_or_else(|| "keine".to_owned(), ToString::to_string)
                ));
            }
        }
    }
}

/// `POST /kontoauszug` in allen Formen: CSV, JSON (Liste und Text), PDF, die Betraege, die
/// `verwirf_unlesbare_betraege` einzeln aus dem Auszug nimmt (nie der ganze Auszug), die Kategorien,
/// der Deckel der LLM-Aufrufe und die Formen des Rumpfs. Jeder Fall bekommt einen frischen Fall.
fn kontoauszug_faelle(a: &mut Sender, status: &std::cell::Cell<u16>) -> EventBilanz {
    let mut k = Lauf {
        a,
        status,
        bilanz: EventBilanz::default(),
        nr: 0,
        route: "kontoauszug",
        kurz: "ka",
    };
    let csv = |zeilen: &[&str]| format!("datum;betrag;verwendungszweck\n{}\n", zeilen.join("\n"));
    let auszug = |format: &str, inhalt: Value| json!({"format": format, "inhalt": inhalt});
    let zaehlen = |u: i64, t: i64, v: i64| {
        vec![
            ("uebernommen", json!(u)),
            ("transaktionen", json!(t)),
            ("verworfen", json!(v)),
        ]
    };
    let hinweis_csv = |n: i64| {
        json!(format!(
            "{n} Zeile(n) mit unlesbarem Betrag (keine Zahl oder ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."
        ))
    };

    // ---- CSV
    k.lauf(
        "csv Kategorien",
        auszug(
            "csv",
            json!(csv(&[
                "01.03.2025;-480,00;Maler Huber",
                "02.03.2025;-50,00;Spende Rotes Kreuz",
                "03.03.2025;-300,00;Minijob-Zentrale",
                "04.03.2025;-1.200,00;Rentenversicherung Ruerup",
                "05.03.2025;1.000,00;Gehalt",
                "06.03.2025;-20,00;Reinigung Treppenhaus",
                "07.03.2025;-9,99;Einkauf",
            ])),
        ),
        200,
        &zaehlen(5, 7, 0),
    );
    k.lauf(
        "csv Betragsschreibweisen",
        auszug(
            "csv",
            json!(csv(&[
                "01.03.2025;-abc;Maler",
                "01.03.2025;1,2,3;Maler",
                "01.03.2025;-1e3;Maler",
                "01.03.2025;-1.234;Maler",
                "01.03.2025;inf;Maler",
                "01.03.2025;NaN;Maler",
                "01.03.2025;-;Maler",
                "01.03.2025;-,5;Maler",
                "01.03.2025;12,345;Maler",
                "01.03.2025;-1.234,567;Maler",
                "01.03.2025;-12.34.567,00;Maler",
            ])),
        ),
        200,
        &zaehlen(0, 0, 11),
    );
    // Lesbar, auch wo es seltsam aussieht: Vorzeichen, `€`, Leerzeichen, Tausenderpunkt, Ziffern
    // anderer Schriften (`unicodedata.decimal`).
    for (name, betrag, cent) in [
        ("csv --5", "--5", -500),
        ("csv +-5", "+-5", 500),
        ("csv -+5", "-+5", -500),
        ("csv Euro und Leerzeichen", " - 5,5 € ", -550),
        ("csv Tausender", "-1.234,5", -123_450),
        ("csv Punkt als Komma", "-480.5", -48_050),
        ("csv ganze Zahl", "-480", -48_000),
        ("csv arabische Ziffern", "-٤٨٠,٠٠", -48_000),
        ("csv Vollbreitenziffern", "-４８０,００", -48_000),
        ("csv fuehrende Nullen", "-000480,00", -48_000),
        ("csv zwanzig Nullen", "-00000000000000000000480,00", -48_000),
    ] {
        k.lauf(
            name,
            auszug("csv", json!(csv(&[&format!("01.03.2025;{betrag};Maler")]))),
            200,
            &[
                ("uebernommen", json!(i64::from(cent < 0))),
                ("transaktionen", json!(1)),
                ("verworfen", json!(0)),
            ],
        );
    }
    // Ab 10^10 Cent (100 Mio. €) fliegt die Buchung einzeln raus, die lesbaren bleiben; ueber i64
    // fliegt sie schon beim Lesen.
    k.lauf(
        "csv Betrag ab 10^10 Cent",
        auszug(
            "csv",
            json!(csv(&[
                "01.03.2025;-100000000,00;Maler",
                "01.03.2025;-99999999,99;Maler",
                "01.03.2025;100000000,00;Gehalt",
                "01.03.2025;-92233720368547758,07;Maler",
                "01.03.2025;-92233720368547758,08;Maler",
                "01.03.2025;-123456789012345678901,00;Maler",
                "01.03.2025;99999999,99;Gehalt",
            ])),
        ),
        200,
        &[
            ("uebernommen", json!(1)),
            ("transaktionen", json!(2)),
            ("verworfen", json!(5)),
            ("hinweis", hinweis_csv(5)),
        ],
    );
    k.lauf(
        "csv Spalten-Aliasse",
        auszug(
            "csv",
            json!("Buchungstag;Umsatz;Buchungstext\n01.03.2025;-480,00;Maler\n02.03.2025;-5,00;Spende\n"),
        ),
        200,
        &zaehlen(2, 2, 0),
    );
    k.lauf(
        "csv englische Spalten",
        auszug(
            "csv",
            json!(" Date ; AMOUNT ;Description\n01.03.2025;-480,00;Maler\n"),
        ),
        200,
        &zaehlen(1, 1, 0),
    );
    k.lauf(
        "csv Betrag (EUR) und Zweck",
        auszug(
            "csv",
            json!("Buchungsdatum;Betrag (EUR);Zweck\n01.03.2025;-480,00;Maler\n"),
        ),
        200,
        &zaehlen(1, 1, 0),
    );
    for (name, text, soll) in [
        ("csv leer", "", zaehlen(0, 0, 0)),
        ("csv nur Kopf", "datum;betrag;verwendungszweck\n", zaehlen(0, 0, 0)),
        ("csv ohne Betragsspalte", "datum;zweck\n01.03.2025;Maler\n", zaehlen(0, 0, 0)),
        ("csv Komma statt Semikolon", "datum,betrag,verwendungszweck\n01.03.2025,-480,Maler\n", zaehlen(0, 0, 0)),
        ("csv leerer Betrag zaehlt nicht", "datum;betrag;verwendungszweck\n01.03.2025;;Maler\n01.03.2025;  ;Maler\n", zaehlen(0, 0, 0)),
        ("csv BOM vor dem Kopf", "\u{feff}datum;betrag;verwendungszweck\n01.03.2025;-480,00;Maler\n", zaehlen(1, 1, 0)),
        ("csv CRLF", "datum;betrag;verwendungszweck\r\n01.03.2025;-480,00;Maler\r\n", zaehlen(1, 1, 0)),
        ("csv zu kurze Zeile", "datum;betrag;verwendungszweck\n01.03.2025;-480,00\n", zaehlen(0, 1, 0)),
        ("csv zu lange Zeile", "datum;betrag;verwendungszweck\n01.03.2025;-480,00;Maler;x;y\n", zaehlen(1, 1, 0)),
        ("csv Zweck mit Semikolon", "datum;betrag;verwendungszweck\n01.03.2025;-480,00;\"Maler; Huber\"\n", zaehlen(1, 1, 0)),
        ("csv Kopf mit Leerzeichen und Grossschrift", "  DATUM ; Betrag ;VerwendungsZweck \n01.03.2025;-480,00;MALER\n", zaehlen(1, 1, 0)),
        ("csv doppelte Betragsspalte", "datum;betrag;betrag\n01.03.2025;-480,00;-5,00\n", zaehlen(0, 1, 0)),
        ("csv ohne Datumsspalte", "betrag;zweck\n-480,00;Maler\n", zaehlen(1, 1, 0)),
        ("csv Zeilen ohne Zeilenumbruch am Ende", "datum;betrag;verwendungszweck\n01.03.2025;-480,00;Maler", zaehlen(1, 1, 0)),
        ("csv NUL im Zweck", "datum;betrag;verwendungszweck\n01.03.2025;-480,00;Ma\u{0}ler\n", zaehlen(0, 1, 0)),
    ] {
        k.lauf(name, auszug("csv", json!(text)), 200, &soll);
    }
    // Der Inhalt eines CSV, der kein Text ist, ist leer (`inhalt if isinstance(inhalt, str) else ""`).
    for (name, inhalt) in [
        ("csv Inhalt Zahl", json!(5)),
        ("csv Inhalt null", Value::Null),
        ("csv Inhalt Liste", json!(["a;b"])),
        ("csv Inhalt Objekt", json!({"a": 1})),
        ("csv Inhalt wahr", json!(true)),
    ] {
        k.lauf(name, auszug("csv", inhalt), 200, &zaehlen(0, 0, 0));
    }
    k.lauf(
        "csv ohne Inhalt",
        json!({"format": "csv"}),
        200,
        &zaehlen(0, 0, 0),
    );
    // Die Fehler von `csv.reader`: Python meldet sie als 500 mit der Klasse `Error`.
    k.lauf(
        "csv CR im Feld",
        auszug("csv", json!("datum;betrag;verwendungszweck\n01.03.2025;-480,00;Ma\rler\n")),
        500,
        &[],
    );
    k.lauf(
        "csv Feld ueber 131072 Zeichen",
        auszug(
            "csv",
            json!(format!(
                "datum;betrag;verwendungszweck\n01.03.2025;-480,00;{}\n",
                "x".repeat(131_073)
            )),
        ),
        500,
        &[],
    );
    k.lauf(
        "csv Feld mit 131072 Zeichen",
        auszug(
            "csv",
            json!(format!(
                "datum;betrag;verwendungszweck\n01.03.2025;-480,00;{}\n",
                "x".repeat(131_072)
            )),
        ),
        200,
        &zaehlen(0, 1, 0),
    );

    // ---- JSON: eine Liste im Rumpf
    let tx = |datum: Value, betrag: Value, zweck: Value| {
        json!({"datum": datum, "betrag": betrag, "verwendungszweck": zweck})
    };
    k.lauf(
        "json Liste",
        auszug(
            "json",
            json!([
                tx(json!("01.03.2025"), json!(-48_000), json!("Maler Huber")),
                tx(json!("02.03.2025"), json!(-5000), json!("Spende")),
                tx(json!("03.03.2025"), json!(100_000), json!("Gehalt")),
            ]),
        ),
        200,
        &zaehlen(2, 3, 0),
    );
    // Der Betrag, den `int(tx.get("betrag", 0))` annimmt (Zahl, Text, Wahrheitswert) oder nicht.
    for (name, betrag, tragbar) in [
        ("json Betrag Kommazahl", json!(-480.9), true),
        ("json Betrag Text", json!("-480"), true),
        ("json Betrag Text mit Leerzeichen", json!(" -480 "), true),
        ("json Betrag Text mit Unterstrich", json!("-4_80"), true),
        ("json Betrag arabische Ziffern", json!("-٤٨٠"), true),
        ("json Betrag wahr", json!(true), true),
        ("json Betrag falsch", json!(false), true),
        ("json Betrag Text 1e3", json!("1e3"), false),
        ("json Betrag Text leer", json!(""), false),
        ("json Betrag Text Minus U+2212", json!("−480"), false),
        ("json Betrag null", Value::Null, false),
        ("json Betrag Liste", json!([1]), false),
        ("json Betrag Objekt", json!({}), false),
        ("json Betrag Text NaN", json!("NaN"), false),
        ("json Betrag Text inf", json!("inf"), false),
        ("json Betrag Text Kommazahl", json!("-480.5"), false),
        ("json Betrag Kommazahl 1e10", json!(1e10), false),
        ("json Betrag Kommazahl -1e10", json!(-1e10), false),
        ("json Betrag Kommazahl -1e308", json!(-1e308), false),
        ("json Betrag Kommazahl unter 1e10", json!(-9_999_999_999.9), true),
        ("json Betrag 10^10 - 1", json!(-9_999_999_999_i64), true),
        ("json Betrag 10^10", json!(-10_000_000_000_i64), false),
        ("json Betrag i64 max", json!(i64::MAX), false),
        ("json Betrag i64 min", json!(i64::MIN), false),
        ("json Betrag Text 10^10", json!("-10000000000"), false),
        ("json Betrag Text 10^10 - 1", json!("-9999999999"), true),
    ] {
        let t = i64::from(tragbar);
        k.lauf(
            name,
            auszug(
                "json",
                json!([tx(json!("01.03.2025"), betrag, json!("Maler"))]),
            ),
            200,
            &[("transaktionen", json!(t)), ("verworfen", json!(1 - t))],
        );
    }
    k.lauf(
        "json Betrag fehlt",
        auszug("json", json!([{"datum": "01.03.2025", "verwendungszweck": "Maler"}])),
        200,
        &zaehlen(0, 1, 0),
    );
    k.lauf(
        "json Element kein Objekt",
        auszug(
            "json",
            json!([5, "x", null, [1], true, {}, tx(json!("d"), json!(-100), json!("Maler"))]),
        ),
        200,
        &[
            ("uebernommen", json!(1)),
            ("transaktionen", json!(2)),
            ("verworfen", json!(5)),
        ],
    );
    // Ein Verwendungszweck, der kein Text ist: bei einer Ausgabe 422 (`pruefe_buchungsfelder`, vorher
    // `AttributeError`, 500), bei einer Einnahme nie gelesen, und falsch heisst leer.
    for (name, zweck, soll) in [
        ("json Zweck Zahl", json!(5), 422),
        ("json Zweck Liste", json!(["maler"]), 422),
        ("json Zweck Objekt", json!({"a": 1}), 422),
        ("json Zweck wahr", json!(true), 422),
        ("json Zweck Kommazahl", json!(1.5), 422),
        ("json Zweck null", Value::Null, 200),
        ("json Zweck 0", json!(0), 200),
        ("json Zweck leere Liste", json!([]), 200),
        ("json Zweck falsch", json!(false), 200),
        ("json Zweck leer", json!(""), 200),
    ] {
        k.lauf(
            name,
            auszug("json", json!([tx(json!("d"), json!(-100), zweck)])),
            soll,
            &[],
        );
    }
    k.lauf(
        "json Zweck Zahl bei Einnahme",
        auszug("json", json!([tx(json!("d"), json!(100), json!(5))])),
        200,
        &zaehlen(0, 1, 0),
    );
    k.lauf(
        "json Zweck fehlt",
        auszug("json", json!([{"betrag": -100}])),
        200,
        &zaehlen(0, 1, 0),
    );
    // Das Datum landet unveraendert in `signal_1`; der Vergleich der Akten sieht, wie.
    for (name, datum) in [
        ("json Datum Zahl", json!(5)),
        ("json Datum null", Value::Null),
        ("json Datum Kommazahl", json!(1.5)),
        ("json Datum 1e22", json!(1e22)),
        ("json Datum Objekt", json!({"z": [1, 2.5, null], "a": "ä😀"})),
        ("json Datum Liste", json!([1, "a"])),
        ("json Datum wahr", json!(true)),
        ("json Datum Umlaut", json!("1. März \u{2028}\"\\")),
    ] {
        k.lauf(
            name,
            auszug("json", json!([tx(datum, json!(-5000), json!("Spende"))])),
            200,
            &zaehlen(1, 1, 0),
        );
    }
    k.lauf(
        "json Datum fehlt",
        auszug("json", json!([{"betrag": -5000, "verwendungszweck": "Spende"}])),
        200,
        &zaehlen(1, 1, 0),
    );
    // PII im Zweck: IBAN, Steuernummer und Kontonummer werden maskiert, bevor sie in der Akte stehen.
    k.lauf(
        "json maskierter Zweck",
        auszug(
            "json",
            json!([tx(
                json!("01.03.2025"),
                json!(-48_000),
                json!("Maler Huber DE89 3704 0044 0532 0130 00 StNr 12/345/67890 Konto 1234567890 de89370400440532013000")
            )]),
        ),
        200,
        &zaehlen(1, 1, 0),
    );
    k.lauf(
        "json Grossschrift im Zweck",
        auszug(
            "json",
            json!([
                tx(json!("d"), json!(-100), json!("MALER")),
                tx(json!("d"), json!(-100), json!("SPENDE AN DAS ROTE KREUZ")),
                tx(json!("d"), json!(-100), json!("GEBÄUDEREINIGUNG")),
                tx(json!("d"), json!(-100), json!("MINIJOB-ZENTRALE")),
                tx(json!("d"), json!(-100), json!("RÜRUP-RENTE")),
                tx(json!("d"), json!(-100), json!("SANITÄR MÜLLER")),
            ]),
        ),
        200,
        &zaehlen(5, 6, 0),
    );
    k.lauf(
        "json zwei Buchungen derselben Kategorie",
        auszug(
            "json",
            json!([
                tx(json!("d"), json!(-100), json!("Maler")),
                tx(json!("d"), json!(-200), json!("Klempner")),
                tx(json!("d"), json!(-300), json!("Reinigung")),
            ]),
        ),
        200,
        &zaehlen(2, 3, 0),
    );
    k.lauf(
        "json Einnahme zaehlt nicht",
        auszug("json", json!([tx(json!("d"), json!(5000), json!("Spende"))])),
        200,
        &zaehlen(0, 1, 0),
    );
    k.lauf(
        "json Betrag 0 zaehlt nicht",
        auszug("json", json!([tx(json!("d"), json!(0), json!("Spende"))])),
        200,
        &zaehlen(0, 1, 0),
    );
    k.lauf_in(
        "json Zielfeld schon belegt",
        &Vorlage {
            scheibe: "gesamt",
            vorher: vec![ereignis("spenden_betrag", &json!(1000), None)],
        },
        auszug(
            "json",
            json!([
                tx(json!("d"), json!(-5000), json!("Spende")),
                tx(json!("d"), json!(-100), json!("Maler")),
            ]),
        ),
        200,
        &zaehlen(1, 2, 0),
    );
    k.lauf_in(
        "json Zielfeld fehlt in der Scheibe",
        &Vorlage {
            scheibe: "ep",
            vorher: vec![],
        },
        auszug(
            "json",
            json!([
                tx(json!("d"), json!(-5000), json!("Spende")),
                tx(json!("d"), json!(-100), json!("Maler")),
            ]),
        ),
        200,
        &zaehlen(0, 2, 0),
    );
    k.lauf(
        "json leere Liste",
        auszug("json", json!([])),
        200,
        &zaehlen(0, 0, 0),
    );
    // Mehr unklare Ausgaben als der Deckel (50) zulaesst: der Rest zaehlt in `llm_uebersprungen`,
    // und die sichere Kategorie dahinter wird trotzdem erkannt.
    let mut viele: Vec<Value> = (0..56)
        .map(|i| tx(json!("d"), json!(-100 - i), json!(format!("Einkauf {i}"))))
        .collect();
    viele.push(tx(json!("d"), json!(-5000), json!("Spende")));
    k.lauf(
        "json Deckel der LLM-Aufrufe",
        auszug("json", json!(viele.clone())),
        200,
        &[
            ("uebernommen", json!(1)),
            ("transaktionen", json!(57)),
            ("verworfen", json!(0)),
            ("llm_uebersprungen", json!(6)),
        ],
    );
    let mut viele_und_unlesbar = viele;
    viele_und_unlesbar.push(tx(json!("d"), json!("x"), json!("Maler")));
    k.lauf(
        "json Deckel und unlesbarer Betrag",
        auszug("json", json!(viele_und_unlesbar)),
        200,
        &[("verworfen", json!(1)), ("llm_uebersprungen", json!(6))],
    );
    k.lauf(
        "json genau am Deckel",
        auszug(
            "json",
            json!((0..50)
                .map(|i| tx(json!("d"), json!(-100 - i), json!(format!("Einkauf {i}"))))
                .collect::<Vec<_>>()),
        ),
        200,
        &zaehlen(0, 50, 0),
    );

    // ---- JSON: der Inhalt als Text
    let text = |t: &str| auszug("json", json!(t));
    k.lauf(
        "json Text Liste",
        text(r#"[{"datum": "01.03.2025", "betrag": -48000, "verwendungszweck": "Maler"}]"#),
        200,
        &zaehlen(1, 1, 0),
    );
    // Beträge, die `json.loads` liest und `serde_json` nicht (NaN, Infinity, 1e400) oder anders.
    for (name, betrag, tragbar) in [
        ("json Text NaN", "NaN", false),
        ("json Text Infinity", "Infinity", false),
        ("json Text -Infinity", "-Infinity", false),
        ("json Text 1e400", "1e400", false),
        ("json Text -1e400", "-1e400", false),
        ("json Text 20 Ziffern", "-12345678901234567890", false),
        ("json Text 4300 Ziffern", &format!("-{}", "9".repeat(4300)), false),
        ("json Text 1e5", "-1e5", true),
        ("json Text -0", "-0", true),
        ("json Text -0.0", "-0.0", true),
        ("json Text 1E2", "-1E2", true),
        ("json Text 480.99", "-480.99", true),
        ("json Text 9999999999.5", "-9999999999.5", true),
        ("json Text 1e10", "-1e10", false),
    ] {
        let t = i64::from(tragbar);
        k.lauf(
            name,
            text(&format!(
                r#"[{{"datum": "d", "betrag": {betrag}, "verwendungszweck": "Maler"}}]"#
            )),
            200,
            &[("transaktionen", json!(t)), ("verworfen", json!(1 - t))],
        );
    }
    k.lauf(
        "json Text 4301 Ziffern",
        text(&format!(
            r#"[{{"betrag": -{}, "verwendungszweck": "Maler"}}]"#,
            "9".repeat(4301)
        )),
        400,
        &[("fehler", json!("json-Inhalt nicht parsebar"))],
    );
    // ---- JSON-Text: Datum und Zweck, die die Akte nicht haelt (Vault Backlog
    // `falldatei-mit-nan-liest-rust-als-text`, AK4). Die Rumpf-Tuer sieht den Text nicht; beide Server
    // weisen mit 422 ab, der Wortlaut nennt Feld und Typ, nie den Wert. Vorher: Python 500 beim Schreiben
    // (NaN) oder 200 mit einer Ganzzahl ausserhalb von i64 in der Akte, Rust 200 mit Text.
    let datum_meldung = |typ: &str| {
        json!(format!(
            "Kontoauszug nicht lesbar: datum einer Buchung enthält eine Zahl ({typ}), die die Akte nicht halten kann."
        ))
    };
    let zweck_meldung = |typ: &str| {
        json!(format!(
            "Kontoauszug nicht lesbar: verwendungszweck einer Ausgabe muss Text sein, nicht {typ}."
        ))
    };
    let neun_4300 = "9".repeat(4300);
    for (name, datum, typ) in [
        ("json Text Datum NaN", "NaN", "float"),
        ("json Text Datum Infinity", "Infinity", "float"),
        ("json Text Datum -Infinity", "-Infinity", "float"),
        ("json Text Datum 1e400", "1e400", "float"),
        ("json Text Datum -1e400", "-1e400", "float"),
        ("json Text Datum Liste mit NaN", "[1, NaN]", "float"),
        ("json Text Datum tief Infinity", r#"{"a": {"b": [Infinity]}}"#, "float"),
        ("json Text Datum 2^63", "9223372036854775808", "int"),
        ("json Text Datum 2^64", "18446744073709551616", "int"),
        ("json Text Datum unter i64", "-9223372036854775809", "int"),
        ("json Text Datum Liste mit 2^64", "[18446744073709551616]", "int"),
        ("json Text Datum Objekt unter i64", r#"{"a": -9223372036854775809}"#, "int"),
        ("json Text Datum 4300 Ziffern", neun_4300.as_str(), "int"),
        ("json Text Datum int und NaN", "[18446744073709551616, NaN]", "float"),
    ] {
        k.lauf(
            name,
            text(&format!(
                r#"[{{"datum": {datum}, "betrag": -5000, "verwendungszweck": "Spende"}}]"#
            )),
            422,
            &[("fehler", datum_meldung(typ))],
        );
    }
    for (name, zahl) in [
        ("json Text Datum fuehrende Null", "0123456789012345678901234"),
        ("json Text Datum Minus mit fuehrender Null", "-0123456789012345678901"),
        ("json Text Datum nur Minus", "-"),
        ("json Text Datum zwei Minus", "--5"),
        ("json Text Datum Plus", "+5"),
    ] {
        k.lauf(
            name,
            text(&format!(
                r#"[{{"datum": {zahl}, "betrag": -5000, "verwendungszweck": "Spende"}}]"#
            )),
            400,
            &[("fehler", json!("json-Inhalt nicht parsebar"))],
        );
    }
    k.lauf(
        "json Text Datum einer Einnahme NaN",
        text(r#"[{"datum": NaN, "betrag": 5000, "verwendungszweck": "Spende"}]"#),
        422,
        &[("fehler", datum_meldung("float"))],
    );
    for (name, datum) in [
        ("json Text Datum i64 max", "9223372036854775807"),
        ("json Text Datum i64 min", "-9223372036854775808"),
        ("json Text Datum Kommazahl", "1.5"),
        ("json Text Datum Text NaN", r#""NaN""#),
        ("json Text Datum Liste", "[1, 2.5]"),
        ("json Text Datum wahr", "true"),
        ("json Text Datum null", "null"),
    ] {
        k.lauf(
            name,
            text(&format!(
                r#"[{{"datum": {datum}, "betrag": -5000, "verwendungszweck": "Spende"}}]"#
            )),
            200,
            &zaehlen(1, 1, 0),
        );
    }
    k.lauf(
        "json Text verworfene Buchung mit NaN im Datum",
        text(
            r#"[{"datum": NaN, "betrag": NaN, "verwendungszweck": "Spende"}, {"datum": "d", "betrag": -5000, "verwendungszweck": "Spende"}]"#,
        ),
        200,
        &[
            ("uebernommen", json!(1)),
            ("transaktionen", json!(1)),
            ("verworfen", json!(1)),
        ],
    );
    k.lauf(
        "json Text Datum vor Zweck",
        text(
            r#"[{"datum": "d", "betrag": -100, "verwendungszweck": 5}, {"datum": NaN, "betrag": -100, "verwendungszweck": "Maler"}]"#,
        ),
        422,
        &[("fehler", datum_meldung("float"))],
    );
    for (name, zweck, typ) in [
        ("json Text Zweck NaN", "NaN", "float"),
        ("json Text Zweck -Infinity", "-Infinity", "float"),
        ("json Text Zweck 1e400", "1e400", "float"),
        ("json Text Zweck 2^64", "18446744073709551616", "int"),
        ("json Text Zweck Zahl", "5", "int"),
        ("json Text Zweck Liste", r#"["maler"]"#, "list"),
    ] {
        k.lauf(
            name,
            text(&format!(
                r#"[{{"datum": "d", "betrag": -5000, "verwendungszweck": {zweck}}}]"#
            )),
            422,
            &[("fehler", zweck_meldung(typ))],
        );
    }
    k.lauf(
        "json Text Zweck NaN bei Einnahme",
        text(r#"[{"datum": "d", "betrag": 5000, "verwendungszweck": NaN}]"#),
        200,
        &zaehlen(0, 1, 0),
    );
    for (name, t, soll, fehler) in [
        ("json Text kaputt", "[{", 400, "json-Inhalt nicht parsebar"),
        ("json Text mit BOM", "\u{feff}[]", 400, "json-Inhalt nicht parsebar"),
        ("json Text Komma am Ende", "[1,]", 400, "json-Inhalt nicht parsebar"),
        ("json Text Leerraum", "  ", 400, "json-Inhalt nicht parsebar"),
        ("json Text nur Zahl", "5", 400, "json muss eine Liste von Transaktionen sein"),
        ("json Text Objekt", "{}", 400, "json muss eine Liste von Transaktionen sein"),
        ("json Text null", "null", 400, "json muss eine Liste von Transaktionen sein"),
        ("json Text Text", "\"abc\"", 400, "json muss eine Liste von Transaktionen sein"),
        ("json Text wahr", "true", 400, "json muss eine Liste von Transaktionen sein"),
        ("json Text NaN allein", "NaN", 400, "json muss eine Liste von Transaktionen sein"),
        ("json Text Einzelquote", "['a']", 400, "json-Inhalt nicht parsebar"),
        ("json Text Kommentar", "[] // x", 400, "json-Inhalt nicht parsebar"),
        ("json Text zwei Listen", "[][]", 400, "json-Inhalt nicht parsebar"),
        ("json Text Steuerzeichen im Text", "[\"a\tb\"]", 400, "json-Inhalt nicht parsebar"),
        ("json Text mit Leerraum drumherum", " \n[]\t", 200, ""),
        ("json Text leer", "", 200, ""),
    ] {
        let erwartet: Vec<(&str, Value)> = if fehler.is_empty() {
            zaehlen(0, 0, 0)
        } else {
            vec![("fehler", json!(fehler))]
        };
        k.lauf(name, text(t), soll, &erwartet);
    }
    // Der Inhalt, der weder Liste noch Text ist: `json.loads` wirft `TypeError`, gefangen wie ein
    // `ValueError`; ein falscher Inhalt ist die leere Liste.
    for (name, inhalt, soll, fehler) in [
        ("json Inhalt Zahl", json!(5), 400, "json-Inhalt nicht parsebar"),
        ("json Inhalt wahr", json!(true), 400, "json-Inhalt nicht parsebar"),
        ("json Inhalt Kommazahl", json!(1.5), 400, "json-Inhalt nicht parsebar"),
        ("json Inhalt Objekt", json!({"a": 1}), 400, "json-Inhalt nicht parsebar"),
        ("json Inhalt leeres Objekt", json!({}), 200, ""),
        ("json Inhalt null", Value::Null, 200, ""),
        ("json Inhalt falsch", json!(false), 200, ""),
        ("json Inhalt 0", json!(0), 200, ""),
        ("json Inhalt 0.0", json!(0.0), 200, ""),
    ] {
        let erwartet: Vec<(&str, Value)> = if fehler.is_empty() {
            zaehlen(0, 0, 0)
        } else {
            vec![("fehler", json!(fehler))]
        };
        k.lauf(name, auszug("json", inhalt), soll, &erwartet);
    }
    k.lauf(
        "json ohne Inhalt",
        json!({"format": "json"}),
        200,
        &zaehlen(0, 0, 0),
    );

    // ---- Format
    let leer_csv = json!("datum;betrag;verwendungszweck\n01.03.2025;-480,00;Maler\n");
    for (name, format, soll) in [
        ("Format Grossschrift", json!("CSV"), 200),
        ("Format mit Leerraum", json!(" csv\n"), 200),
        ("Format Tab und NBSP", json!("\u{a0}csv\u{a0}"), 200),
        ("Format mit U+001C und U+001F", json!("\u{1c}csv\u{1f}"), 200),
        ("Format gemischt", json!("Csv"), 200),
        ("Format fehlt", Value::Null, 400),
        ("Format leer", json!(""), 400),
        ("Format Leerraum", json!("  "), 400),
        ("Format Liste leer", json!([]), 400),
        ("Format Objekt leer", json!({}), 400),
        ("Format falsch", json!(false), 400),
        ("Format 0", json!(0), 400),
        ("Format xml", json!("xml"), 400),
        ("Format Zahl", json!(5), 500),
        ("Format wahr", json!(true), 500),
        ("Format Liste", json!(["csv"]), 500),
        ("Format Objekt", json!({"a": 1}), 500),
        ("Format Kommazahl", json!(1.5), 500),
    ] {
        k.lauf(
            name,
            json!({"format": format, "inhalt": leer_csv}),
            soll,
            &[],
        );
    }
    k.lauf("Format fehlt ganz", json!({"inhalt": leer_csv}), 400, &[("fehler", json!("format muss csv, json oder pdf sein"))]);
    k.lauf("Rumpf leeres Objekt", json!({}), 400, &[]);
    for (name, body) in [
        ("Rumpf Liste", json!([1])),
        ("Rumpf Text", json!("csv")),
        ("Rumpf null", Value::Null),
        ("Rumpf Zahl", json!(5)),
        ("Rumpf wahr", json!(true)),
        ("Rumpf Kommazahl", json!(1.5)),
    ] {
        k.lauf(name, body, 500, &[]);
    }

    // ---- PDF
    let pdf = |zeilen: &[&str]| json!({"format": "pdf", "inhalt": base64(&pdf_mit_zeilen(zeilen))});
    k.lauf(
        "pdf Zeilen",
        pdf(&[
            "01.03.2025 Maler Huber -480,00 EUR",
            "02.03.2025 Spende Rotes Kreuz -50,00 EUR",
            "03.03.2025 Minijob-Zentrale -999.999.999,99 EUR",
            "Saldo 1.000,00",
        ]),
        200,
        &[
            ("uebernommen", json!(2)),
            ("transaktionen", json!(2)),
            ("verworfen", json!(1)),
            (
                "hinweis",
                json!("1 Zeile(n) unsicher erkannt (Confidence < 60%) oder mit zu großem Betrag (ab 100 Mio. €) verworfen — bitte manuell prüfen/nachtragen."),
            ),
        ],
    );
    k.lauf(
        "pdf mehrere Betraege in einer Zeile",
        pdf(&[
            "01.03.2025 Maler Huber -480,00 EUR 1.234,56",
            "02.03.2025 Spende -50,00",
            "03.03.2025 ohne Betrag",
            "Kein Datum -5,00",
            "Zwischensumme -100,00",
            "Tagessaldo 5,00",
        ]),
        200,
        &zaehlen(1, 1, 2),
    );
    k.lauf(
        "pdf Betrag ueber i64",
        pdf(&[
            "01.03.2025 Maler -92.233.720.368.547.758,08",
            "02.03.2025 Maler -92.233.720.368.547.758,07",
            "03.03.2025 Spende -1,00",
        ]),
        200,
        &zaehlen(1, 1, 2),
    );
    k.lauf(
        "pdf ohne Text",
        pdf(&[]),
        200,
        &zaehlen(0, 0, 0),
    );
    for (name, inhalt, soll, fehler) in [
        ("pdf Inhalt fehlt", Value::Null, 400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)"),
        ("pdf Inhalt leer", json!(""), 400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)"),
        ("pdf Inhalt Leerraum", json!(" \n\t"), 400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)"),
        ("pdf Inhalt Zahl", json!(5), 400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)"),
        ("pdf Inhalt Liste", json!(["QQ=="]), 400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)"),
        ("pdf base64 Auffuellung fehlt", json!("QQ"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 Auffuellung halb", json!("QQ="), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 Zeilenumbruch", json!("QUJD\nREVG"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 Leerzeichen", json!("QUJD REVG"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 Auffuellung vorn", json!("=QUJD"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 zweimal aufgefuellt", json!("QQ==QQ=="), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 URL-Alphabet", json!("QQ-_"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 nur Minus", json!("QUJ-"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 nur Unterstrich", json!("QUJ_"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 Umlaut", json!("QUJDä"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
        ("pdf base64 Rest 1", json!("QUJDR"), 400, "pdf-Inhalt nicht gültig base64-kodiert"),
    ] {
        k.lauf(
            name,
            json!({"format": "pdf", "inhalt": inhalt}),
            soll,
            &[("fehler", json!(fehler))],
        );
    }
    // Gueltiges base64 von etwas, das kein PDF ist: `pdftotext` kann es nicht oeffnen (422).
    for (name, inhalt) in [
        ("pdf kein PDF", base64(b"Das ist kein PDF.")),
        ("pdf nur Kopfzeile", base64(b"%PDF-1.4\n")),
        ("pdf ein Byte", base64(b"A")),
    ] {
        k.lauf(name, json!({"format": "pdf", "inhalt": inhalt}), 422, &[]);
    }
    k.bilanz
}

/// Setzt in den Akten BEIDER Server den Wert des letzten Events von `feld` im Fall `id`: ein
/// Altwert, den die heutige Wertpruefung abweist (gespeichert vor dieser Pruefung; nur von Hand zu
/// erzeugen). Beide Seiten lesen die Akte bei jeder Anfrage neu.
fn altwert_setzen(tmp: &Path, id: &str, feld: &str, wert: &Value) {
    for art in ["python", "rust"] {
        let pfad = tmp.join(art).join("faelle").join(format!("{id}.json"));
        let mut akte: Value = serde_json::from_slice(&std::fs::read(&pfad).unwrap()).unwrap();
        let e = akte["events"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .rev()
            .find(|e| e["feld_id"] == feld)
            .unwrap();
        e["wert"] = wert.clone();
        std::fs::write(&pfad, serde_json::to_vec(&akte).unwrap()).unwrap();
    }
}

/// Ein Vorjahres-Fall: `bestaetigt` als Events der Schreiber `ui:` (bestaetigt), `vorlaeufig` als
/// Vorschlaege des LLM; die Kennung `id`, die Scheibe und das Jahr stehen fest.
fn vorjahr_quelle(
    k: &mut Lauf,
    (id, scheibe, vz): (&str, &str, i64),
    bestaetigt: &[(&str, Value)],
    vorlaeufig: &[(&str, Value)],
) {
    (k.a)(
        "POST",
        "/fall",
        Some(json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": vz})),
    );
    for (feld, wert) in bestaetigt {
        (k.a)(
            "POST",
            &format!("/fall/{id}/event"),
            Some(ereignis(feld, wert, None)),
        );
    }
    for (feld, wert) in vorlaeufig {
        (k.a)(
            "POST",
            &format!("/fall/{id}/event"),
            Some(ereignis_llm(feld, wert)),
        );
    }
}

/// `POST /vorjahr` in allen Formen: die Uebernahme je Scheibe (nur bestaetigte Felder, nie ueber ein
/// belegtes), Altwerte, die die Wertpruefung abweist (uebersprungen), die Vergleichsgroesse
/// `vorjahr_referenz`, die zweite Fall-Kennung des Rumpfs (400, `TypeError`, 404,
/// 403) und die Formen des Rumpfs. Jeder Fall bekommt einen frischen Fall als Ziel.
fn vorjahr_faelle(a: &mut Sender, status: &std::cell::Cell<u16>, tmp: &Path) -> EventBilanz {
    let mut k = Lauf {
        a,
        status,
        bilanz: EventBilanz::default(),
        nr: 0,
        route: "vorjahr",
        kurz: "vt",
    };
    let stamm = [
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(4_000_000)),
        ("geburtsjahr", json!(1980)),
        ("stammdaten_nachname", json!("Müller")),
        ("hh_handwerker_betrag", json!(48_000)),
        ("ep_entfernung_km", json!(30)),
        ("verlustvortrag_bestand", json!(123_456)),
    ];
    let vorlaeufig = [
        ("ep_arbeitstage", json!(200)),
        ("kap_kapitalertraege", json!(150_000)),
    ];
    vorjahr_quelle(&mut k, ("g_vq1", "gesamt", 2024), &stamm, &vorlaeufig);
    // Altwerte, die die heutige Pruefung abweist: Steuerzeichen (T), Vorzeichen (V), Bereich (W).
    vorjahr_quelle(&mut k, ("g_vq2", "gesamt", 2024), &stamm, &[]);
    altwert_setzen(tmp, "g_vq2", "hh_handwerker_betrag", &json!(-5000));
    altwert_setzen(tmp, "g_vq2", "geburtsjahr", &json!(1899));
    altwert_setzen(tmp, "g_vq2", "stammdaten_nachname", &json!("Maier\u{0}"));
    // Ein Altwert ab 10^10: der Store prueft F2/Magnitude nur mit Katalog, und `uebernehme_vorjahr`
    // gibt keinen mit; er wird uebernommen. (Die Abweisungen, bei denen die Uebernahme abbricht und
    // 422 antwortet, kann ein `import:vorjahr`-Event heute nicht ausloesen.)
    vorjahr_quelle(&mut k, ("g_vq3", "gesamt", 2024), &stamm, &[]);
    altwert_setzen(tmp, "g_vq3", "bruttoarbeitslohn", &json!(10_000_000_000_i64));
    vorjahr_quelle(&mut k, ("g_vq4", "gesamt", 2024), &stamm, &[]);
    altwert_setzen(tmp, "g_vq4", "bruttoarbeitslohn", &json!(10_000_000_000_i64));
    altwert_setzen(tmp, "g_vq4", "hh_handwerker_betrag", &json!(10_000_000_000_i64));
    vorjahr_quelle(&mut k, ("g_vq5", "gesamt", 2024), &[], &vorlaeufig);
    vorjahr_quelle(
        &mut k,
        ("g_vq6", "gesamt", 2024),
        &[("verlustvortrag_bestand", json!(7))],
        &[],
    );
    vorjahr_quelle(&mut k, ("g_vq7", "gesamt", 2024), &[], &[]);
    vorjahr_quelle(
        &mut k,
        ("g_vq8", "ep", 2025),
        &[
            ("ep_entfernung_km", json!(40)),
            ("ep_eigenes_kfz", json!(true)),
            ("ep_arbeitstage", json!(210)),
        ],
        &[],
    );
    let von = |id: &str| json!({"vorjahr_fall_id": id});
    let z = |u: i64, ue: &[&str], id: &str| {
        vec![
            ("uebernommen", json!(u)),
            ("uebersprungen", json!(ue)),
            ("vorjahr_fall_id", json!(id)),
        ]
    };
    k.lauf("vorjahr normal", von("g_vq1"), 200, &z(6, &[], "g_vq1"));
    // Dasselbe Ziel dreimal: die zweite Uebernahme findet alles belegt; die Vergleichsgroesse aus
    // `g_vq6` ersetzt die erste, eine Quelle ohne Bestand laesst sie stehen.
    let id = k.fall("vorjahr Folge", &Vorlage::gesamt());
    k.post("vorjahr Folge 1", &id, von("g_vq1"), 200, &z(6, &[], "g_vq1"));
    k.post("vorjahr Folge 2", &id, von("g_vq1"), 200, &z(0, &[], "g_vq1"));
    k.post("vorjahr Folge 3", &id, von("g_vq6"), 200, &z(0, &[], "g_vq6"));
    k.post("vorjahr Folge 4", &id, von("g_vq5"), 200, &z(0, &[], "g_vq5"));
    k.lauf_in(
        "vorjahr Ziel teils belegt",
        &Vorlage {
            scheibe: "gesamt",
            vorher: vec![
                ereignis("veranlagung", &json!("zusammen"), None),
                ereignis("geburtsjahr", &json!(1970), None),
            ],
        },
        von("g_vq1"),
        200,
        &z(4, &[], "g_vq1"),
    );
    k.lauf_in(
        "vorjahr Ziel ep",
        &Vorlage {
            scheibe: "ep",
            vorher: vec![],
        },
        von("g_vq1"),
        200,
        &z(1, &[], "g_vq1"),
    );
    k.lauf("vorjahr Quelle ep", von("g_vq8"), 200, &z(3, &[], "g_vq8"));
    k.lauf(
        "vorjahr Altwerte abgewiesen",
        von("g_vq2"),
        200,
        &z(
            3,
            &["geburtsjahr", "hh_handwerker_betrag", "stammdaten_nachname"],
            "g_vq2",
        ),
    );
    k.lauf("vorjahr Altwert ab 10^10", von("g_vq3"), 200, &z(6, &[], "g_vq3"));
    k.lauf("vorjahr zwei Altwerte ab 10^10", von("g_vq4"), 200, &z(6, &[], "g_vq4"));
    k.lauf("vorjahr nur vorlaeufig", von("g_vq5"), 200, &z(0, &[], "g_vq5"));
    k.lauf("vorjahr leere Quelle", von("g_vq7"), 200, &z(0, &[], "g_vq7"));
    k.lauf("vorjahr eigener Fall als Quelle", von("seed_a"), 200, &z(0, &[], "seed_a"));
    let id = k.fall("vorjahr Quelle ist Ziel", &Vorlage::gesamt());
    k.post(
        "vorjahr Quelle ist Ziel",
        &id,
        von(&id),
        400,
        &[("fehler", json!("vorjahr_fall_id muss ein ANDERER (Vorjahres-)Fall sein"))],
    );
    let fehlt = "vorjahr_fall_id fehlt oder ungültig";
    k.lauf("vorjahr leerer Rumpf", json!({}), 400, &[("fehler", json!(fehlt))]);
    for (name, wert) in [
        ("null", Value::Null),
        ("leerer Text", json!("")),
        ("0", json!(0)),
        ("falsch", json!(false)),
        ("leere Liste", json!([])),
        ("leeres Objekt", json!({})),
        ("0.0", json!(0.0)),
        ("Schraegstrich", json!("a/b")),
        ("Umlaut", json!("ä")),
        ("65 Zeichen", json!("x".repeat(65))),
        ("Leerzeichen", json!(" ")),
        ("Leerzeichen im Namen", json!("a b")),
        ("Zeilenumbruch am Ende", json!("g_vq1\n")),
        ("Liste", json!(["g_vq1"])),
        ("Objekt", json!({"a": 1})),
        ("Kommazahl", json!(1.5)),
        ("grosse Kommazahl", json!(1e22)),
    ] {
        k.lauf(
            &format!("vorjahr Kennung {name}"),
            json!({"vorjahr_fall_id": wert}),
            400,
            &[("fehler", json!(fehlt))],
        );
    }
    // Eine Ganzzahl, `true` und eine Kommazahl wie 1e-05 bestehen die Pruefung des Textes und
    // scheitern erst in `lade_fall` (`TypeError`).
    for (name, wert) in [
        ("Ganzzahl", json!(5)),
        ("negative Ganzzahl", json!(-5)),
        ("i64 max", json!(i64::MAX)),
        ("wahr", json!(true)),
        ("kleine Kommazahl", json!(1e-5)),
    ] {
        k.lauf(
            &format!("vorjahr Kennung {name}"),
            json!({"vorjahr_fall_id": wert}),
            500,
            &[],
        );
    }
    for (name, id, soll) in [
        ("fehlt", "gibtsnicht", 404),
        ("fremd", "seed_b", 403),
        ("herrenlos", "seed_o", 403),
    ] {
        k.lauf(&format!("vorjahr Quelle {name}"), von(id), soll, &[]);
    }
    for (name, body) in [
        ("Liste", json!([1])),
        ("Text", json!("g_vq1")),
        ("null", Value::Null),
        ("Zahl", json!(5)),
        ("wahr", json!(true)),
    ] {
        k.lauf(&format!("vorjahr Rumpf {name}"), body, 500, &[]);
    }
    k.bilanz
}

/// Rumpf von `POST /event` als Rohtext: `wert` und `signal` stehen so, wie sie hier geschrieben sind.
fn roher_text(feld: &str, wert: &str, signal: &str) -> String {
    format!(
        r#"{{"feld_id": "{feld}", "wert": {wert}, "zustand": "vorlaeufig", "schreiber": "ui:paritaet", "herkunft": {{"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}}, "signal": {signal}, "ts": "2026-01-01T00:00:00+00:00", "ersetzt": null}}"#
    )
}

/// Ein nicht leerer Body je POST-Route aus Stufe 1–3; `quelle` ist der Vorjahres-Fall. Stufe 4
/// bekommt `{}`: sie geht nie an Python (s. `UNTERGRENZE`).
fn koerper(route: &str, quelle: &str) -> Value {
    match route {
        "event" => ereignis("ep_arbeitstage", &json!(220), None),
        "flow" => json!({"art": "weg_gewaehlt", "inhalt": {"weg": "fragebogen"}}),
        "vorjahr" => json!({"vorjahr_fall_id": quelle}),
        _ => json!({}),
    }
}

/// Der Pflicht-Kegel von `an_gesamt` (33 Felder, `SCHEIBEN['an_gesamt']['kegel']`) mit neutralen
/// Werten, ohne `ep_arbeitstage`, `fam_anzahl_kinder` und `dhf_monate`. Alle Achsen, die dann noch
/// offen sind, haben einen `bereich`: der Ring rechnet, und `fragen` bekommt Gewichte.
fn kegel_an() -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(4_000_000)),
        ("veranlagung", json!("einzel")),
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
        ("dhf_im_inland", json!(false)),
        ("dhf_beruflich_veranlasst", json!(false)),
        ("dhf_eigener_hausstand", json!(false)),
        ("dhf_finanzielle_beteiligung", json!(false)),
        ("tage_24h", json!(1)),
        ("tage_an_abreise", json!(1)),
        ("tage_ueber_8h_eintaegig", json!(1)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("verlustvortrag_bestand", json!(0)),
    ]
}

/// `kegel_an` vollstaendig: dazu `ep_arbeitstage`, `fam_anzahl_kinder`, `dhf_monate` und die zwei
/// Verpflegungs-Angaben, die den Guard (§ 9 Abs. 4a) zufriedenstellen. `/ergebnis` nennt eine Zahl.
fn kegel_an_voll() -> Vec<(&'static str, Value)> {
    let mut k = kegel_an();
    k.extend([
        ("ep_arbeitstage", json!(220)),
        ("fam_anzahl_kinder", json!(0)),
        ("dhf_monate", json!(0)),
        ("vpf_monate_am_ort", json!(2)),
        ("vpf_keine_mahlzeitengestellung", json!(true)),
    ]);
    k
}

/// Der Pflicht-Kegel von `gesamt` (35 Felder), vollstaendig bestaetigt: Lohn, V+V, Kapital ohne
/// Betraege. `/ergebnis` nennt eine Zahl und die Rechenweg-Kette.
fn kegel_gesamt() -> Vec<(&'static str, Value)> {
    vec![
        ("vv_einnahmen", json!(1_000_000)),
        ("vv_gebaeude_afa", json!(100_000)),
        ("vv_schuldzinsen", json!(50_000)),
        ("vv_erhaltungsaufwand", json!(0)),
        ("vv_sonstige_wk", json!(0)),
        ("vv_entgelt_quote_prozent", json!(100)),
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(4_000_000)),
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
        ("kap_kapitalertraege", json!(0)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_gewinn_sonstige", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(false)),
        ("kein_sonstige", json!(true)),
        ("agb_zwangslaeufig", json!(false)),
        ("agb_notwendig_angemessen", json!(false)),
    ]
}

/// Der Pflicht-Kegel von `rentner_gesamt` (28 Felder), vollstaendig bestaetigt, als aa-Rente mit
/// Beginn `beginn` und OHNE `rentner_rentenfreibetrag`. Beginn 2020: der Guard sperrt mit
/// `rentenfreibetrag_fixierung_offen`, und der Ring wirft `RentenfreibetragFixierungOffen`, sobald
/// er rechnet. Beginn 2025 (Erstjahr): `/ergebnis` nennt eine Zahl.
fn kegel_rentner(beginn: i64) -> Vec<(&'static str, Value)> {
    vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(1_200_000)),
        ("rentner_renten_beginn_jahr", json!(beginn)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_grad_der_behinderung", json!(50)),
        ("rentner_hilflos_blind_taubblind", json!(false)),
        ("rentner_pflegegrad", json!(1)),
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
        ("agb_zwangslaeufig", json!(false)),
        ("agb_notwendig_angemessen", json!(false)),
    ]
}

/// Echte Eingaben fuer Stufe 1–3 (9c): vier eigene Faelle in zwei Scheiben, Events auf mehreren
/// Feldern samt Ersetzung, Vorjahr-Uebernahme aus einer zweiten Fallakte, UI-Meldungen mit
/// `TAXGRAPH_FLOW=1`, dann jede Lese-Route. Jede Route erreicht ihren Rumpf so oft, wie
/// `UNTERGRENZE` verlangt.
#[test]
fn generatoren() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let seed = tmp.path().join("seed");
    schreibe_seed(&seed);
    let mut p = Paar::neu(tmp.path(), false, true, &seed);
    let alice = token("alice", GEHEIMNIS);
    // Der Status der letzten Anfrage (Rust; gleich dem von Python, sonst ist es eine Abweichung).
    let status = std::cell::Cell::new(0u16);
    let mut a = |m: &str, pfad: &str, body: Option<Value>| {
        let x = Anfrage::neu(&format!("gen {m} {pfad}"), m, pfad).token(&alice);
        let antwort = p.anfrage(&body.map_or(x.clone(), |b| x.json(&b)), Modus::Voll);
        status.set(p.stat.letzter);
        antwort
    };
    for (id, scheibe, vz) in [
        ("g_ep", "ep", 2025),
        ("g_vj", "ep", 2024),
        ("g_neu", "ep", 2025),
        ("g_ges", "gesamt", 2025),
        ("g_an", "an_gesamt", 2025),
        ("g_rent", "rentner_gesamt", 2025),
        ("g_rent2", "rentner_gesamt", 2025),
        // aa-Folgejahr ohne fixierten Rentenfreibetrag: der Ring wirft, `fragen` bleibt vollstaendig.
        ("g_rent3", "rentner_gesamt", 2025),
        // `fragen` mit Gewichten aus dem Ring: `g_an2` (Gesamt-Ring, trotz Sperrgrund), `g_ep2` und
        // `g_vor2` (Gesamt-Ring von `ep`, Teil-Ring von `n_vor_gwg`), je mit einer offenen Achse.
        ("g_an2", "an_gesamt", 2025),
        // `ergebnis` mit Zahl: g_an3 (voller Kegel, kein SolZ), g_an4 (dazu Lohnsteuer und
        // Konfession: KiSt und Abschlusszahlung), g_an5 (vorlaeufige Lohnsteuer sperrt die Zahl),
        // g_ges2 (Kette), g_ges3 (Kinder: Guenstigerpruefung § 31), g_ges4/g_ges5/g_ges6 (offene
        // Hinweise trotz Zahl: Gate fehlt, vorlaeufiges Kind-Feld, vorlaeufiger Verkaufspreis),
        // g_rent4 (Rentner im Erstjahr, Erstattung).
        ("g_an3", "an_gesamt", 2025),
        ("g_an4", "an_gesamt", 2025),
        ("g_an5", "an_gesamt", 2025),
        // g_an6: das Gate `vpf_auswaertige_taetigkeit` (verneint) liegt NICHT in der Scheibe `an_gesamt`
        // (`POST /event` weist es dort ab), und die drei `tage_*` des Kegels fehlen. Die Akte entsteht
        // als `gesamt` und bekommt unten von Hand die Scheibe `an_gesamt`. Pythons Relevanz
        // (Scheiben-Bindung) sieht das Gate nicht, eine Voll-Graph-Sicht schloesse die Regel aus und
        // liesse den Kegel vollstaendig.
        ("g_an6", "gesamt", 2025),
        ("g_ges2", "gesamt", 2025),
        ("g_ges3", "gesamt", 2025),
        ("g_ges4", "gesamt", 2025),
        ("g_ges5", "gesamt", 2025),
        ("g_ges6", "gesamt", 2025),
        // g_ges7: zusammen mit Partner und hohem Einkommen (Freibetrag guenstiger als Kindergeld),
        // g_ges8: `vv_wohnzwecke` verneint nimmt `vv_entgelt_quote_prozent` aus dem Kegel,
        // g_ges9: dieselbe Quote fehlt, ohne dass die Regel abbestellt ist.
        ("g_ges7", "gesamt", 2025),
        ("g_ges8", "gesamt", 2025),
        ("g_ges9", "gesamt", 2025),
        ("g_rent4", "rentner_gesamt", 2025),
        ("g_ep2", "ep", 2025),
        ("g_vor2", "n_vor_gwg", 2025),
        ("g_vor", "n_vor_gwg", 2025),
        // Herkunftsformen und Zustaende fuer `warum`/`graph`; `g_aussen` bekommt weiter unten die
        // Scheibe `ep` (Scheiben-Wechsel von Hand) und behaelt ein Feld, das dort keine Bindung hat.
        ("g_wz", "gesamt", 2025),
        ("g_aussen", "gesamt", 2025),
        // `preflight`: ein roter Fall mit je einem Widerspruch, `g_pf_ae` fuer § 24b, `g_pf_gelb`
        // (nur Hinweise), zwei gruene (leer, und mit Angaben, die nichts melden), `g_pf_un` ohne
        // Flag-Antwort auf `gesamt`, `g_pf_nf` mit demselben Betrag auf `ep` (Flag dort nicht fragbar),
        // `g_vj_vv` als Vorjahr mit Verlustvortrag.
        ("g_vj_vv", "gesamt", 2024),
        ("g_pf_rot", "gesamt", 2025),
        ("g_pf_ae", "gesamt", 2025),
        ("g_pf_gelb", "gesamt", 2025),
        ("g_pf_gruen", "gesamt", 2025),
        ("g_pf_leer", "gesamt", 2025),
        ("g_pf_un", "gesamt", 2025),
        ("g_pf_nf", "gesamt", 2025),
        // `deklaration`: g_dk speist jede Ring-Einspeisung ohne Sperrgrund, g_dk2 ist zusammen mit
        // Partner; g_vz0, g_vz23 und g_vz27 bekommen unten von Hand das Jahr 0, 2023 (kein Steuerjahr)
        // und 2027 (ohne Parameter: der Ring schluckt es, die Deklaration rechnet). `/deklaration`
        // sperrt bei Sperrgrund mit 409; g_dk_* tragen je einen davon (Kapital-Widerspruch,
        // Verpflegung, § 23, Haushaltsnahes, Partner).
        ("g_dk", "gesamt", 2025),
        ("g_dk2", "gesamt", 2025),
        ("g_vz0", "gesamt", 2025),
        ("g_vz23", "gesamt", 2025),
        ("g_vz27", "gesamt", 2025),
        ("g_dk_kap", "gesamt", 2025),
        ("g_dk_vpf", "gesamt", 2025),
        ("g_dk_p23", "gesamt", 2025),
        ("g_dk_hh", "gesamt", 2025),
        ("g_dk_partner", "gesamt", 2025),
    ] {
        let b = json!({"fall_id": id, "scheibe": scheibe, "veranlagungszeitraum": vz});
        a("POST", "/fall", Some(b));
    }
    let mut erster = None;
    for (id, feld, wert) in [
        ("g_ep", "ep_arbeitstage", json!(220)),
        ("g_ep", "ep_entfernung_km", json!(30)),
        ("g_ep", "ep_eigenes_kfz", json!(true)),
        ("g_ep", "ep_oepnv_kosten", json!(0)),
        ("g_ep", "ep_ziel_des_weges", json!("1")),
        (
            "g_ep",
            "ep_ziel_adresse",
            json!("80331 München, Marienplatz 1"),
        ),
        ("g_vj", "ep_entfernung_km", json!(25)),
        ("g_vj", "ep_eigenes_kfz", json!(true)),
        ("g_ges", "bruttoarbeitslohn", json!(4_000_000)),
        ("g_ges", "veranlagung", json!("einzel")),
        ("g_ges", "ep_arbeitstage", json!(230)),
        // an_gesamt: ein dHf-Feld ueber null sperrt den Ring (K2-Guard), `engine` ist `gesperrt`.
        ("g_an", "bruttoarbeitslohn", json!(4_000_000)),
        ("g_an", "veranlagung", json!("einzel")),
        ("g_an", "dhf_unterkunftskosten_monat", json!(50_000)),
        // rentner_gesamt: mit Rentenbeginn rechnet der Ring, ohne ihn sperrt `rentenbeginn_offen`.
        ("g_rent", "rentner_jahresrente", json!(1_200_000)),
        ("g_rent", "rentner_renten_beginn_jahr", json!(2015)),
        ("g_rent", "veranlagung", json!("einzel")),
        // Ohne `kein_sonstige = false` sperrt schon der Flag-Guard; erst damit bleibt der Rentenbeginn.
        ("g_rent2", "rentner_jahresrente", json!(1_200_000)),
        ("g_rent2", "kein_sonstige", json!(false)),
        // g_ep2/g_vor2: alles bestaetigt ausser `ep_arbeitstage` (offene Achse mit `bereich`).
        ("g_ep2", "ep_entfernung_km", json!(30)),
        ("g_ep2", "ep_oepnv_kosten", json!(0)),
        ("g_ep2", "ep_eigenes_kfz", json!(true)),
        ("g_ep2", "ep_ziel_des_weges", json!("1")),
        (
            "g_ep2",
            "ep_ziel_adresse",
            json!("80331 München, Marienplatz 1"),
        ),
        ("g_vor2", "ep_entfernung_km", json!(30)),
        ("g_vor2", "ep_oepnv_kosten", json!(0)),
        ("g_vor2", "ep_eigenes_kfz", json!(true)),
        // n_vor_gwg liest seine Felder aus der YAML und rechnet nur den Teil-Ring.
        ("g_vor", "ep_arbeitstage", json!(220)),
        ("g_vor", "ep_entfernung_km", json!(30)),
        ("g_vor", "ep_eigenes_kfz", json!(true)),
    ] {
        let pfad = format!("/fall/{id}/event");
        let b = a("POST", &pfad, Some(ereignis(feld, &wert, None)));
        erster = erster.or_else(|| b?["event_id"].as_str().map(str::to_owned));
    }
    // Die zwei Kegel: `g_an2` mit `ep_arbeitstage` (eine offene Achse je `bereich`), `g_rent3` voll.
    let mit_kinder = || {
        let mut k = kegel_gesamt();
        k.push(("fam_anzahl_kinder", json!(2)));
        k
    };
    let mut kegel: Vec<(&str, &str, Value)> = vec![];
    let mut fuege = |id: &'static str, k: Vec<(&'static str, Value)>| {
        kegel.extend(k.into_iter().map(|(f, w)| (id, f, w)));
    };
    fuege("g_an2", {
        let mut k = kegel_an();
        k.push(("ep_arbeitstage", json!(220)));
        k
    });
    fuege("g_rent3", kegel_rentner(2020));
    for id in ["g_an3", "g_an4", "g_an5"] {
        fuege(id, kegel_an_voll());
    }
    fuege("g_an6", {
        let mut k = kegel_an_voll();
        k.retain(|(f, _)| !matches!(*f, "tage_24h" | "tage_an_abreise" | "tage_ueber_8h_eintaegig"));
        k.push(("vpf_auswaertige_taetigkeit", json!(false)));
        k
    });
    fuege(
        "g_an4",
        vec![
            ("p36_lohnsteuer", json!(500_000)),
            ("kist_konfession", json!("evangelisch")),
        ],
    );
    fuege("g_ges2", kegel_gesamt());
    fuege("g_ges3", mit_kinder());
    fuege("g_ges4", {
        let mut k = mit_kinder();
        k.push(("kinderbetreuungskosten", json!(120_000)));
        k
    });
    fuege("g_ges5", mit_kinder());
    fuege("g_ges6", kegel_gesamt());
    fuege("g_ges7", {
        let mut k = mit_kinder();
        for e in &mut k {
            match e.0 {
                "veranlagung" => e.1 = json!("zusammen"),
                "bruttoarbeitslohn" => e.1 = json!(40_000_000),
                _ => {}
            }
        }
        k.extend([
            ("bruttoarbeitslohn_partner", json!(3_000_000)),
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ]);
        k
    });
    let ohne_quote = || {
        let mut k = kegel_gesamt();
        k.retain(|(f, _)| *f != "vv_entgelt_quote_prozent");
        k
    };
    fuege("g_ges8", {
        let mut k = ohne_quote();
        k.push(("vv_wohnzwecke", json!(false)));
        k
    });
    fuege("g_ges9", ohne_quote());
    fuege("g_rent4", {
        let mut k = kegel_rentner(2025);
        k.push(("p36_lohnsteuer", json!(1_000)));
        k
    });
    for (id, feld, wert) in kegel {
        let pfad = format!("/fall/{id}/event");
        a("POST", &pfad, Some(ereignis(feld, &wert, None)));
    }
    // Vorlaeufige Angaben neben vollstaendigem Kegel (LLM-Vorschlag, kein signal_2).
    for (id, feld, wert) in [
        ("g_an5", "p36_lohnsteuer", 500_000),
        ("g_ges5", "kinderbetreuungskosten", 120_000),
        ("g_ges6", "p23_veraeusserungspreis", 50_000_000),
    ] {
        let pfad = format!("/fall/{id}/event");
        a("POST", &pfad, Some(ereignis_llm(feld, &json!(wert))));
    }
    for (id, ev) in [
        ("g_wz", ereignis_llm("ep_oepnv_kosten", &json!(120))),
        (
            "g_wz",
            ereignis_beleg("kap_kapitalertraege", &json!(150_000)),
        ),
        (
            "g_wz",
            ereignis("stammdaten_nachname", &json!("Müller-Lüdenscheidt"), None),
        ),
        ("g_wz", ereignis("kein_kap", &json!(true), None)),
        ("g_aussen", ereignis("ep_arbeitstage", &json!(200), None)),
        (
            "g_aussen",
            ereignis("bruttoarbeitslohn", &json!(3_000_000), None),
        ),
    ] {
        a("POST", &format!("/fall/{id}/event"), Some(ev));
    }
    // Das Vorjahr mit bestaetigtem Verlustvortrag; `POST /vorjahr` legt `vorjahr_referenz` an, gegen
    // die `preflight` den neuen Bestand prueft. Erst danach kommt der hoehere Bestand in `g_pf_rot`.
    let mut abgewiesen: Vec<String> = vec![];
    let ev = ereignis("verlustvortrag_bestand", &json!(100_000), None);
    if a("POST", "/fall/g_vj_vv/event", Some(ev)).is_none_or(|b| b.get("event_id").is_none()) {
        abgewiesen.push("g_vj_vv/verlustvortrag_bestand".to_owned());
    }
    let b = a(
        "POST",
        "/fall/g_pf_rot/vorjahr",
        Some(koerper("vorjahr", "g_vj_vv")),
    );
    assert!(b.is_some(), "Vorjahr g_pf_rot aus g_vj_vv abgewiesen");
    for (id, feld, wert) in [
        // Rot: je ein Widerspruch aus jedem Bereich, ausser § 24b (`g_pf_ae`).
        ("g_pf_rot", "bruttoarbeitslohn", json!(4_000_000)),
        ("g_pf_rot", "p36_lohnsteuer", json!(5_000_000)),
        ("g_pf_rot", "kist_gezahlt", json!(1_500_000)),
        ("g_pf_rot", "kirchensteuer_arbeitgeber", json!(5_000)),
        ("g_pf_rot", "kein_kap", json!(true)),
        ("g_pf_rot", "kap_kapitalertraege", json!(10_000)),
        ("g_pf_rot", "veranlagung", json!("einzel")),
        ("g_pf_rot", "kap_kapitalertraege_partner", json!(20_000)),
        ("g_pf_rot", "fam_anzahl_kinder", json!(3)),
        ("g_pf_rot", "kind_vorname", json!("Anna")),
        ("g_pf_rot", "schulgeld", json!(20_000_000)),
        ("g_pf_rot", "schulgeld__2", json!(5_000)),
        ("g_pf_rot", "stammdaten_keine_bankverbindung", json!(true)),
        (
            "g_pf_rot",
            "stammdaten_iban",
            json!("DE89370400440532013000"),
        ),
        ("g_pf_rot", "verlustvortrag_bestand", json!(500_000)),
        ("g_pf_ae", "veranlagung", json!("zusammen")),
        ("g_pf_ae", "fam_alleinstehend", json!(true)),
        ("g_pf_gelb", "bruttoarbeitslohn", json!(3_000_000)),
        ("g_pf_gelb", "kein_vuv", json!(false)),
        ("g_pf_gelb", "vv_einnahmen", json!(1_000_000)),
        ("g_pf_gruen", "bruttoarbeitslohn", json!(4_000_000)),
        ("g_pf_gruen", "ep_arbeitstage", json!(220)),
        ("g_pf_gruen", "p36_lohnsteuer", json!(500_000)),
        // Kein Flag beantwortet: auf `gesamt` ein Widerspruch, auf `ep` (s. u.) nicht.
        ("g_pf_un", "kap_kapitalertraege", json!(10_000)),
        ("g_pf_nf", "kap_kapitalertraege", json!(10_000)),
    ] {
        let ev = ereignis(feld, &wert, None);
        if a("POST", &format!("/fall/{id}/event"), Some(ev))
            .is_none_or(|b| b.get("event_id").is_none())
        {
            abgewiesen.push(format!("{id}/{feld}"));
        }
    }
    // `deklaration`: je Ring-Einspeisung (`mit_ring_werten`) ein Satz Felder auf `gesamt`:
    // Verpflegungskuerzung (E0205508), Kapital-Antrag (E1900401/E1901401), haushaltsnahe Summen,
    // V+V-Summen samt dokumentiertem Aggregat, Einzelzeilen (§ 35c, GewSt, § 22 Nr. 3,
    // Berufsausbildung) und Kinder -- alles ohne Sperrgrund, sonst antwortet `/deklaration` 409 und
    // keine Kz erreichen die Antwort. Dafuer die Antworten auf die Fragen, die der Guard sonst offen
    // sieht: `kein_kap`, `kein_vuv`, `vpf_monate_am_ort`, `hh_rechnung_unbar`,
    // `hh_handwerker_keine_foerderung`, `hh_in_eu_ewr`. § 23 steht in `g_dk_p23`: der Ring rechnet es
    // nicht (`einkunftsart_nicht_ring_faehig`), also sperrt `/deklaration` in jedem Fall mit § 23.
    for (id, feld, wert) in [
        ("g_dk", "bruttoarbeitslohn", json!(6_000_000)),
        ("g_dk", "veranlagung", json!("einzel")),
        ("g_dk", "vpf_monate_am_ort", json!(2)),
        ("g_dk", "tage_24h", json!(20)),
        ("g_dk", "tage_an_abreise", json!(2)),
        ("g_dk", "tage_ueber_8h_eintaegig", json!(3)),
        ("g_dk", "vpf_fruehstuecke_gestellt_anzahl", json!(5)),
        ("g_dk", "vpf_mittagessen_gestellt_anzahl", json!(3)),
        ("g_dk", "vpf_abendessen_gestellt_anzahl", json!(2)),
        ("g_dk", "vpf_mahlzeiten_gezahltes_entgelt", json!(0)),
        ("g_dk", "kein_kap", json!(false)),
        ("g_dk", "kap_kapitalertraege", json!(500_000)),
        ("g_dk", "hh_rechnung_unbar", json!(true)),
        ("g_dk", "hh_handwerker_keine_foerderung", json!(true)),
        ("g_dk", "hh_in_eu_ewr", json!(true)),
        ("g_dk", "hh_minijob_betrag", json!(40_000)),
        ("g_dk", "hh_minijob_betrag__2", json!(10_000)),
        ("g_dk", "hh_dienstleistung_betrag", json!(120_000)),
        ("g_dk", "hh_handwerker_betrag", json!(200_000)),
        ("g_dk", "kein_vuv", json!(false)),
        ("g_dk", "vv_einnahmen", json!(1_000_000)),
        ("g_dk", "vv_gebaeude_afa", json!(100_000)),
        ("g_dk", "vv_schuldzinsen", json!(50_000)),
        ("g_dk", "vv_nebenkosten_umgelegt", json!(80_000)),
        ("g_dk", "p35c_sanierungsaufwendungen", json!(1_000_000)),
        ("g_dk", "p35c_keine_doppelfoerderung", json!(true)),
        ("g_dk", "gewst_messbetrag", json!(50_000)),
        ("g_dk", "gewst_hebesatz", json!(400)),
        ("g_dk", "p22_nr3_einnahmen", json!(100_000)),
        ("g_dk", "p22_nr3_einkuenfte", json!(40_000)),
        ("g_dk", "berufsausbildung_aufwendungen", json!(600_000)),
        ("g_dk", "fam_anzahl_kinder", json!(2)),
        ("g_dk", "kind_vorname", json!("Anna")),
        ("g_dk", "kind_vorname__2", json!("Ben")),
        ("g_dk2", "veranlagung", json!("zusammen")),
        ("g_dk2", "bruttoarbeitslohn", json!(5_000_000)),
        ("g_dk2", "bruttoarbeitslohn_partner", json!(3_000_000)),
        ("g_dk2", "kein_kap", json!(false)),
        ("g_dk2", "kap_kapitalertraege", json!(200_000)),
        ("g_dk2", "kap_kapitalertraege_partner", json!(100_000)),
        // Der Pflicht-Kegel von Person B und die Antwort auf ihr Flag: sonst `flag_konsistenz_offen`.
        ("g_dk2", "kap_gewinn_aktien_partner", json!(0)),
        ("g_dk2", "kap_gewinn_sonstige_partner", json!(0)),
        ("g_dk2", "kap_verlust_aktien_partner", json!(0)),
        ("g_dk2", "kap_verlust_sonstige_partner", json!(0)),
        ("g_dk2", "kein_kap_partner", json!(false)),
        ("g_dk2", "gewst_messbetrag", json!(50_000)),
        ("g_dk2", "gewst_hebesatz", json!(400)),
        ("g_dk2", "gewst_messbetrag_partner", json!(30_000)),
        ("g_dk2", "gewst_hebesatz_partner", json!(380)),
        ("g_vz0", "bruttoarbeitslohn", json!(4_000_000)),
        ("g_vz23", "bruttoarbeitslohn", json!(4_000_000)),
        ("g_vz27", "bruttoarbeitslohn", json!(4_000_000)),
        ("g_vz27", "kein_kap", json!(false)),
        ("g_vz27", "kap_kapitalertraege", json!(500_000)),
        ("g_vz27", "vpf_monate_am_ort", json!(2)),
        ("g_vz27", "tage_24h", json!(10)),
        ("g_vz27", "vpf_fruehstuecke_gestellt_anzahl", json!(3)),
        // Sperrfaelle von `/deklaration`, je einer mit eigenem Grund (alle: Einzelveranlagung):
        // `kapital_semantik_offen` (Aggregat UND Aktien-Topf), `verpflegung_dreimonatsfrist_aufteilung_
        // offen` (`vpf_monate_am_ort` fehlt), `einkunftsart_nicht_ring_faehig` (§ 23),
        // `rechnung_unbar_offen` (Dienstleistung ohne `hh_rechnung_unbar`), `partner_kegel_offen`.
        ("g_dk_kap", "bruttoarbeitslohn", json!(6_000_000)),
        ("g_dk_kap", "veranlagung", json!("einzel")),
        ("g_dk_kap", "kein_kap", json!(false)),
        ("g_dk_kap", "kap_kapitalertraege", json!(500_000)),
        ("g_dk_kap", "kap_gewinn_aktien", json!(300_000)),
        ("g_dk_vpf", "bruttoarbeitslohn", json!(6_000_000)),
        ("g_dk_vpf", "veranlagung", json!("einzel")),
        ("g_dk_vpf", "tage_24h", json!(20)),
        ("g_dk_vpf", "vpf_fruehstuecke_gestellt_anzahl", json!(5)),
        ("g_dk_p23", "bruttoarbeitslohn", json!(6_000_000)),
        ("g_dk_p23", "veranlagung", json!("einzel")),
        ("g_dk_p23", "kein_p23_verkauf", json!(false)),
        ("g_dk_p23", "p23_veraeusserungs_typ", json!("grundstueck")),
        ("g_dk_p23", "p23_veraeusserungspreis", json!(20_000_000)),
        (
            "g_dk_p23",
            "p23_anschaffung_herstellungskosten",
            json!(10_000_000),
        ),
        ("g_dk_p23", "p23_werbungskosten", json!(100_000)),
        ("g_dk_hh", "bruttoarbeitslohn", json!(6_000_000)),
        ("g_dk_hh", "veranlagung", json!("einzel")),
        ("g_dk_hh", "hh_dienstleistung_betrag", json!(120_000)),
        ("g_dk_partner", "veranlagung", json!("zusammen")),
        ("g_dk_partner", "bruttoarbeitslohn", json!(5_000_000)),
    ] {
        let ev = ereignis(feld, &wert, None);
        if a("POST", &format!("/fall/{id}/event"), Some(ev))
            .is_none_or(|b| b.get("event_id").is_none())
        {
            abgewiesen.push(format!("{id}/{feld}"));
        }
    }
    let ev = ereignis_llm("agb_aufwendungen", &json!(50_000));
    if a("POST", "/fall/g_pf_gelb/event", Some(ev)).is_none_or(|b| b.get("event_id").is_none()) {
        abgewiesen.push("g_pf_gelb/agb_aufwendungen".to_owned());
    }
    assert!(
        abgewiesen.is_empty(),
        "Events der preflight- und deklaration-Faelle abgewiesen: {abgewiesen:?}"
    );
    // Scheiben-Wechsel von Hand, in beiden Verzeichnissen gleich: `bruttoarbeitslohn` hat in `ep`
    // keine Bindung mehr. Der Store laesst so ein Event nicht ueber `POST /event` zu (400), eine
    // vorhandene Akte kann es dennoch tragen.
    for (art, (id, scheibe)) in ["python", "rust"].into_iter().flat_map(|art| {
        [
            ("g_aussen", "ep"),
            ("g_pf_nf", "ep"),
            ("g_an6", "an_gesamt"),
        ]
        .map(|f| (art, f))
    }) {
        let pfad = tmp
            .path()
            .join(art)
            .join("faelle")
            .join(format!("{id}.json"));
        let mut akte: Value = serde_json::from_slice(&std::fs::read(&pfad).unwrap()).unwrap();
        akte["scheibe"] = json!(scheibe);
        std::fs::write(&pfad, serde_json::to_vec(&akte).unwrap()).unwrap();
    }
    // Jahr von Hand: 0 und 2023 lassen `deklariere` scheitern, 2027 hat keine Parameter (Ring-Werte
    // ohne Jahr, die Deklaration rechnet trotzdem).
    for (art, (id, vz)) in ["python", "rust"]
        .into_iter()
        .flat_map(|art| [("g_vz0", 0), ("g_vz23", 2023), ("g_vz27", 2027)].map(|f| (art, f)))
    {
        let pfad = tmp
            .path()
            .join(art)
            .join("faelle")
            .join(format!("{id}.json"));
        let mut akte: Value = serde_json::from_slice(&std::fs::read(&pfad).unwrap()).unwrap();
        akte["veranlagungszeitraum"] = json!(vz);
        std::fs::write(&pfad, serde_json::to_vec(&akte).unwrap()).unwrap();
    }
    // Ersetzung des ersten Events; dasselbe Feld ohne `ersetzt` weist der Store ab (422).
    let ersetzung = ereignis("ep_arbeitstage", &json!(210), erster.as_deref());
    a("POST", "/fall/g_ep/event", Some(ersetzung));
    let ohne = ereignis("ep_arbeitstage", &json!(230), None);
    a("POST", "/fall/g_ep/event", Some(ohne));
    // Das zweite Mal uebernimmt nichts: die Felder sind schon belegt.
    let vj = koerper("vorjahr", "g_vj");
    for soll in [2, 0] {
        let b = a("POST", "/fall/g_neu/vorjahr", Some(vj.clone()));
        assert_eq!(b.unwrap()["uebernommen"], soll, "Vorjahr g_neu aus g_vj");
    }
    // `flow` mit Schalter: jede Sorte, die Kappung bei 4000 Zeichen, Abweisungen mit Wortlaut.
    // Beide Server schreiben dazu `flow.jsonl`; `Paar::anfrage` vergleicht die neuen Zeilen.
    for b in [
        koerper("flow", ""),
        json!({"art": "pruefliste_weiter", "inhalt": {"offen": ["ep_arbeitstage"]}}),
        json!({"art": "erfunden", "inhalt": {}}),
        json!({"art": "nachfrage_spaeter"}),
        json!({"art": "nachfragen_gestartet", "inhalt": {"text": "ä".repeat(5000)}}),
        json!({"art": "pruefliste_aendern", "inhalt": [1, 2.5, null, "ä😀\u{2028}\"\\"]}),
        json!({"art": 5}),
        json!({}),
        json!([1]),
    ] {
        a("POST", "/fall/g_ep/flow", Some(b));
    }
    a("POST", "/fall/g_vj/flow", None);
    // `POST /event` in allen Formen (Tabelle der Begleitfelder, Tuer, Auflagen A bis B).
    let ereignisse = event_faelle(&mut a, &status);
    // `POST /kontoauszug` in allen Formen (CSV, JSON, PDF, Betraege, Kategorien, Deckel, Rumpf).
    let auszuege = kontoauszug_faelle(&mut a, &status);
    // `POST /vorjahr` in allen Formen (Quellen unten von Hand, Altwerte, zweite Kennung).
    let vorjahre = vorjahr_faelle(&mut a, &status, tmp.path());
    let mut engines: BTreeMap<String, usize> = BTreeMap::new();
    let mut gruende: Vec<String> = vec![];
    let mut fragen_je_fall: Vec<(&str, usize)> = vec![];
    let mut fragen_gruende: Vec<String> = vec![];
    // Die Felder der Queue je Fall: `frage` fragt danach jedes davon einzeln ab.
    let mut queue_felder: Vec<(&str, Vec<(String, bool)>)> = vec![];
    let mut ergebnisse: Vec<Value> = vec![];
    let mut deklarationen: Vec<Value> = vec![];
    for id in [
        "g_ep", "g_neu", "g_ges", "g_an", "g_rent", "g_rent2", "g_rent3", "g_vor", "g_wz",
        "g_aussen", "g_an2", "g_ep2", "g_vor2", "g_pf_rot", "g_dk", "g_dk2",
    ] {
        for r in ["stand", "fragen", "ergebnis", "graph", "deklaration"] {
            let b = a("GET", &format!("/fall/{id}/{r}"), None);
            if r == "stand" {
                let e = b
                    .as_ref()
                    .and_then(|b| b["engine"].as_str().map(str::to_owned));
                *engines.entry(e.unwrap_or_default()).or_default() += 1;
                gruende.extend(b.and_then(|b| b["ring_gesperrt"].as_str().map(str::to_owned)));
            } else if r == "ergebnis" {
                ergebnisse.extend(b);
            } else if r == "deklaration" {
                // g_dk und g_dk2 tragen die Ring-Einspeisungen und duerfen nicht sperren.
                if id.starts_with("g_dk") {
                    assert_eq!(status.get(), 200, "deklaration {id} sperrt: {b:?}");
                }
                deklarationen.extend(b);
            } else if r == "fragen" {
                // Fragen je Antwort und der Sperrgrund, den `fragen` selbst meldet (ohne den
                // Rentenbeginn-Zweig von `stand`).
                let n = b
                    .as_ref()
                    .map_or(0, |b| b["fragen"].as_array().map_or(0, Vec::len));
                fragen_je_fall.push((id, n));
                queue_felder.push((
                    id,
                    b.iter()
                        .flat_map(|b| b["fragen"].as_array().into_iter().flatten())
                        .filter_map(|f| {
                            Some((
                                f["feld_id"].as_str()?.to_owned(),
                                f["instanz_etikett"].as_str().is_some_and(|e| !e.is_empty()),
                            ))
                        })
                        .collect(),
                ));
                fragen_gruende
                    .extend(b.and_then(|b| b["ring_gesperrt"].as_str().map(str::to_owned)));
            }
        }
    }
    println!("  fragen: Anzahl je Fall {fragen_je_fall:?}, Sperrgruende {fragen_gruende:?}");
    for id in [
        "g_an3", "g_an4", "g_an5", "g_an6", "g_ges2", "g_ges3", "g_ges4", "g_ges5", "g_ges6", "g_ges7",
        "g_ges8", "g_ges9", "g_rent4",
    ] {
        ergebnisse.extend(a("GET", &format!("/fall/{id}/ergebnis"), None));
    }
    // Was Pythons `/ergebnis` in diesen Faellen sagt: gezaehlt wird der Inhalt der Antwort, nicht dass
    // 200 zurueckkam. `engine_unavailable` bleibt ungezaehlt: jede Scheibe mit Ring hat einen Accessor.
    let mut ergebnis_gruende: BTreeMap<String, usize> = BTreeMap::new();
    for e in &ergebnisse {
        *ergebnis_gruende
            .entry(e["grund"].as_str().unwrap_or_default().to_owned())
            .or_default() += 1;
    }
    let mit = |k: &str| ergebnisse.iter().filter(|e| !e[k].is_null()).count();
    let mit_offen = ergebnisse
        .iter()
        .filter(|e| {
            e["grund"] == "bestaetigt" && e["offen"].as_array().is_some_and(|o| !o.is_empty())
        })
        .count();
    let mit_p31 = ergebnisse
        .iter()
        .filter(|e| !e["kette"]["p31"].is_null())
        .count();
    let mit_freibetrag = ergebnisse
        .iter()
        .filter(|e| e["kette"]["p31"]["guenstiger"] == "freibetraege")
        .count();
    println!(
        "  ergebnis: Gruende {ergebnis_gruende:?}, zahl {} solz {} kist {} abschluss {} kette {} p31 {mit_p31} \
         offen trotz Zahl {mit_offen} sperr_felder {} trace {}",
        mit("zahl_cent"), mit("solz_cent"), mit("kist_cent"), mit("abschlusszahlung_cent"),
        mit("kette"), mit("sperr_felder"), mit("trace"),
    );
    // `preflight` auf allen Faellen: welche Ampeln und Bereiche Pythons Antworten tragen, gezaehlt
    // wird, was die Antwort enthaelt — nicht, dass 200 zurueckkam.
    let mut ampeln: BTreeMap<String, usize> = BTreeMap::new();
    let mut bereiche: BTreeMap<String, usize> = BTreeMap::new();
    for id in [
        "g_ep",
        "g_neu",
        "g_ges",
        "g_an",
        "g_rent",
        "g_rent2",
        "g_vor",
        "g_wz",
        "g_aussen",
        "g_vj_vv",
        "g_pf_rot",
        "g_pf_ae",
        "g_pf_gelb",
        "g_pf_gruen",
        "g_pf_leer",
        "g_pf_un",
        "g_pf_nf",
    ] {
        let Some(b) = a("GET", &format!("/fall/{id}/preflight"), None) else {
            continue;
        };
        *ampeln
            .entry(b["status"].as_str().unwrap_or_default().to_owned())
            .or_default() += 1;
        for i in b["items"].as_array().into_iter().flatten() {
            let schluessel = format!(
                "{}/{}",
                i["typ"].as_str().unwrap_or_default(),
                i["bereich"].as_str().unwrap_or_default()
            );
            *bereiche.entry(schluessel).or_default() += 1;
        }
    }
    println!("  preflight: Ampeln {ampeln:?}, Items je Bereich {bereiche:?}");
    // `engine` aus Pythons Antwort; Rust ist dieselbe Antwort (sonst waere eine Abweichung gemeldet).
    // Gezaehlt wird, welche Rechenwege `stand` erreicht — nicht, dass 200 zurueckkam.
    println!("  stand: engine je Antwort {engines:?}, Sperrgruende {gruende:?}");
    for (id, feld) in [
        ("g_ep", "ep_arbeitstage"),
        ("g_ep", "ep_eigenes_kfz"),
        ("g_neu", "ep_entfernung_km"),
        ("g_ges", "bruttoarbeitslohn"),
        // `warum`: LLM-Vorschlag, Beleg-Import, Text mit Umlaut, Bool, Feld ausserhalb der Scheibe.
        ("g_ep", "ep_oepnv_kosten"),
        ("g_wz", "ep_oepnv_kosten"),
        ("g_wz", "kap_kapitalertraege"),
        ("g_wz", "stammdaten_nachname"),
        ("g_wz", "kein_kap"),
        ("g_aussen", "bruttoarbeitslohn"),
        // Ohne Event (404), unbekanntes Feld (404), Grossbuchstaben im Namen (404 mit `repr`).
        ("g_neu", "ep_ziel_adresse"),
        ("g_ges", "nicht_da_feld"),
        ("g_ep", "ABC_Gross"),
    ] {
        for r in ["warum", "frage"] {
            a("GET", &format!("/fall/{id}/feld/{feld}/{r}"), None);
        }
    }
    // `frage` zu jedem Feld der Queue dieser Faelle: Bereich mit und ohne Grund, Aufzaehlungen,
    // Muster, Standardwerte, Vorjahr-Kategorie, Instanz-Etikett. Ein Feld mit Instanz-Etikett wird
    // zusaetzlich mit `__2` gefragt (Aufloesung auf das Basisfeld); `__1` ist keine Instanz mehr und
    // steht in `instanz_eins_wird_abgewiesen`.
    let mut frage_felder = 0_usize;
    let mut frage_instanz = 0_usize;
    for (id, felder) in &queue_felder {
        if !["g_wz", "g_rent3", "g_an2", "g_vor", "g_neu", "g_aussen"].contains(id) {
            continue;
        }
        for (fid, instanz) in felder {
            a("GET", &format!("/fall/{id}/feld/{fid}/frage"), None);
            frage_felder += 1;
            if *instanz {
                a("GET", &format!("/fall/{id}/feld/{fid}__2/frage"), None);
                frage_instanz += 1;
            }
        }
    }
    // Instanz-Suffix an einem Feld ohne Instanz, am unbekannten Feld und mit nicht-numerischem Suffix.
    for fid in ["ep_arbeitstage__2", "nicht_da_feld__2", "ep_arbeitstage__x"] {
        a("GET", &format!("/fall/g_ep/feld/{fid}/frage"), None);
    }
    for id in ["g_vz0", "g_vz23", "g_vz27"] {
        deklarationen.extend(a("GET", &format!("/fall/{id}/deklaration"), None));
        // Nur das Jahr 2027 rechnet (ohne Parameter); 0 und 2023 lassen `deklariere` scheitern (500).
        if id == "g_vz27" {
            assert_eq!(status.get(), 200, "deklaration {id} sperrt");
        }
    }
    // Die Sperrfaelle: `/deklaration` antwortet 409 mit `fall_id`, `grund`, `klartext`; Python und
    // Rust vergleicht `Paar::anfrage` ueber den ganzen Koerper. Ihre Gruende stehen in `gesperrt_dk`.
    for id in [
        "g_dk_kap",
        "g_dk_vpf",
        "g_dk_p23",
        "g_dk_hh",
        "g_dk_partner",
    ] {
        deklarationen.extend(a("GET", &format!("/fall/{id}/deklaration"), None));
        assert_eq!(status.get(), 409, "deklaration {id} sperrt nicht");
    }
    // `deklaration`: welche Kz und welche Bereiche der Antwort Pythons Antworten tragen.
    let mut kz_je: BTreeMap<String, usize> = BTreeMap::new();
    for d in &deklarationen {
        for k in d["deklaration"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, _)| k)
        {
            *kz_je.entry(k.clone()).or_default() += 1;
        }
    }
    let gefuellt = |k: &str| {
        deklarationen
            .iter()
            .filter(|d| match &d[k] {
                Value::Array(a) => !a.is_empty(),
                Value::Object(o) => !o.is_empty(),
                _ => false,
            })
            .count()
    };
    println!(
        "  deklaration: {} Antworten, Kz je Antwort {kz_je:?}, person_b {} kind_anlagen {} anlage_instanzen {} \
         dokumentiert {} nicht_deklariert {} unvollstaendig {} luecken {} vollstaendig {}",
        deklarationen.len(), gefuellt("person_b"), gefuellt("kind_anlagen"), gefuellt("anlage_instanzen"),
        gefuellt("dokumentiert"), gefuellt("nicht_deklariert"), gefuellt("unvollstaendig"),
        gefuellt("pflichtfelder_luecken"),
        deklarationen.iter().filter(|d| d["vollstaendig"] == json!(true)).count(),
    );
    // Die Sperrfaelle unter den Antworten: nur ein 409 traegt `grund` (die 200-Antwort kennt den
    // Schluessel nicht). Aus den fuenf Faellen oben und den Faellen der Schleife davor
    // (g_an, g_rent, g_rent3, g_an2, g_pf_rot).
    let gesperrt_dk: Vec<&Value> = deklarationen
        .iter()
        .filter(|d| d.get("grund").is_some())
        .collect();
    let gruende_dk: BTreeMap<&str, usize> = gesperrt_dk.iter().fold(BTreeMap::new(), |mut m, d| {
        *m.entry(d["grund"].as_str().unwrap_or_default())
            .or_default() += 1;
        m
    });
    println!(
        "  deklaration gesperrt (409): {} Antworten, Gruende {gruende_dk:?}",
        gesperrt_dk.len()
    );
    println!("  frage: Felder der Queue {frage_felder}, davon mit __2 zusaetzlich {frage_instanz}");
    // `flow` mit rohem Text: Reihenfolge der Schluessel, doppelte Schluessel, Zahlenschreibweisen und
    // Escapes — Wege, die ein `json!`-`Value` (sortiert) im Test verschluckte.
    for text in [
        r#"{"inhalt": {"z": 1, "a": [1E5, 1e-7, 0.1], "ä": "😀\u0000"}, "art": "weg_gewaehlt"}"#,
        r#"{"art": "x", "art": "nachfrage_spaeter", "inhalt": {"b": 1, "a": 2, "b": 3}}"#,
        // i64::MAX: die grösste Ganzzahl, die die Tür noch durchlässt (Rest der Zeile: Zahlenschreibweise).
        r#"{"art": "pruefliste_aendern", "inhalt": 9223372036854775807}"#,
        // u64::MAX: ausserhalb von i64 → 400 an der Tür, in Python (`parse_int`) wie in Rust
        // (`hat_ganzzahl_ausserhalb_i64`); es entsteht keine Zeile in `flow.jsonl`.
        r#"{"art": "pruefliste_aendern", "inhalt": 18446744073709551615}"#,
        "null",
        r#""weg_gewaehlt""#,
    ] {
        let x = Anfrage::neu("gen flow roh", "POST", "/fall/g_neu/flow").token(&alice);
        p.anfrage(&x.roh(text, "application/json"), Modus::Voll);
    }
    // `POST /event` mit Zahlenschreibweisen, wie sie ein `json!`-`Value` verschluckte: `signal_1`
    // wird unveraendert abgelegt, `wert` steht in der 422-Meldung (`repr` einer Kommazahl).
    for text in [
        roher_text("ep_arbeitstage", "1", r#"{"signal_1": 1.0, "signal_2": null}"#),
        roher_text("ep_entfernung_km", "1", r#"{"signal_1": 1E5, "signal_2": "a"}"#),
        roher_text(
            "ep_oepnv_kosten",
            "1",
            r#"{"signal_1": {"z": 1, "a": [1e-7, 0.1, -0.0, 1e22], "ä": "😀\u0000\ud83d\ude00"}, "signal_2": null}"#,
        ),
        roher_text(
            "kap_kapitalertraege",
            "1",
            r#"{"signal_1": 9223372036854775807, "signal_2": null}"#,
        ),
        roher_text("kap_gewinn_aktien", "1", r#"{"signal_1": [], "signal_2": "x"}"#),
        // Abgewiesen (Typ): der Mitschnitt `abgewiesen` traegt `wert` in der Reihenfolge des Clients.
        roher_text(
            "ep_oepnv_kosten",
            r#"{"z": 1, "a": [1E5, 1e-7, 1e22], "ä": "x"}"#,
            "null",
        ),
        roher_text("ep_oepnv_kosten", "[1.0, -0.0, 1e16]", "null"),
        roher_text("ep_oepnv_kosten", "1e22", "null"),
        roher_text("ep_oepnv_kosten", "1e5", "null"),
        roher_text("ep_oepnv_kosten", "0.1", "null"),
        roher_text("ep_oepnv_kosten", "-0.0", "null"),
        roher_text("ep_oepnv_kosten", "5e-324", "null"),
        roher_text("ep_oepnv_kosten", "1.7976931348623157e308", "null"),
        roher_text("ep_arbeitstage", "1E2", "null"),
        // Ausserhalb von i64 und nicht endlich: 400 an der Tuer.
        roher_text("ep_oepnv_kosten", "123456789012345678901", "null"),
        roher_text("ep_oepnv_kosten", "1e400", "null"),
        roher_text("ep_oepnv_kosten", "1", r#"{"signal_1": 18446744073709551615}"#),
        // Doppelter Schluessel: der letzte gilt.
        r#"{"feld_id": "gibt_es_nicht", "feld_id": "ep_ziel_adresse", "wert": "x", "wert": "y", "zustand": "vorlaeufig", "schreiber": "ui:paritaet", "herkunft": {"herkunft": "laie"}}"#.to_owned(),
        roher_text("ep_ziel_adresse", r#""a\u0000b""#, "null"),
    ] {
        let x = Anfrage::neu("gen event roh", "POST", "/fall/g_evr/event").token(&alice);
        p.anfrage(&x.roh(&text, "application/json"), Modus::Voll);
    }
    p.zustand_vergleichen("am Ende");
    p.bericht("generatoren");
    println!(
        "  event: {} Faelle, Klassen {:?}",
        ereignisse.klassen.values().sum::<usize>(),
        ereignisse.klassen
    );
    assert!(
        ereignisse.falsch.is_empty(),
        "event: Faelle erreichen nicht den Zweig ihres Namens: {:?}",
        ereignisse.falsch
    );
    for k in [
        "201",
        "400",
        "500 TypeError",
        "500 AttributeError",
        "422 Form",
        "422 A",
        "422 Katalog",
        "422 F2/Magnitude",
        "422 Typ",
        "422 Vorzeichen",
        "422 Bereich",
        "422 Format",
        "422 B",
        "422 signal_2",
    ] {
        assert!(
            ereignisse.klassen.contains_key(k),
            "event: kein Fall der Klasse {k:?}: {:?}",
            ereignisse.klassen
        );
    }
    println!(
        "  kontoauszug: {} Faelle, Klassen {:?}",
        auszuege.klassen.values().sum::<usize>(),
        auszuege.klassen
    );
    assert!(
        auszuege.falsch.is_empty(),
        "kontoauszug: Faelle erreichen nicht den Zweig ihres Namens: {:?}",
        auszuege.falsch
    );
    println!(
        "  vorjahr: {} Faelle, Klassen {:?}",
        vorjahre.klassen.values().sum::<usize>(),
        vorjahre.klassen
    );
    assert!(
        vorjahre.falsch.is_empty(),
        "vorjahr: Faelle erreichen nicht den Zweig ihres Namens: {:?}",
        vorjahre.falsch
    );
    for k in ["200", "400", "422", "500 AttributeError", "500 Error"] {
        assert!(
            auszuege.klassen.contains_key(k),
            "kontoauszug: kein Fall der Klasse {k:?}: {:?}",
            auszuege.klassen
        );
    }
    assert!(p.stat.abweichungen.is_empty(), "{:?}", p.stat.abweichungen);
    for e in ["catala", "catala_teilweise", "gesperrt"] {
        assert!(
            engines.get(e).copied().unwrap_or(0) >= 1,
            "stand erreicht den Rechenweg {e:?} zu selten: {engines:?}"
        );
    }
    // Der Ring wirft hier (`RentenfreibetragFixierungOffen`), und die Liste bleibt trotzdem voll.
    assert!(
        fragen_gruende
            .iter()
            .any(|g| g == "rentenfreibetrag_fixierung_offen"),
        "fragen meldet nie den Sperrgrund der Fixierung: {fragen_gruende:?}"
    );
    assert!(
        fragen_je_fall
            .iter()
            .any(|(id, n)| *id == "g_rent3" && *n > 100),
        "fragen: g_rent3 ohne volle Liste: {fragen_je_fall:?}"
    );
    for g in [
        "bestaetigt",
        "input_kegel_nicht_bestaetigt",
        "ring_betrag_vorlaeufig",
        "kein_scheiben_gesamtbescheid",
        "dhf_tatbestand_offen",
        "partner_konsistenz_offen",
    ] {
        assert!(
            ergebnis_gruende.contains_key(g),
            "ergebnis meldet nie den Grund {g:?}: {ergebnis_gruende:?}"
        );
    }
    assert!(
        mit("kist_cent") >= 1 && mit("abschlusszahlung_cent") >= 2 && mit("kette") >= 3,
        "ergebnis: zu wenige Zahlen mit KiSt, Abschlusszahlung oder Kette"
    );
    assert!(
        mit_p31 >= 2 && mit_freibetrag >= 1 && mit_offen >= 3,
        "ergebnis: p31 {mit_p31}, davon Freibetrag {mit_freibetrag}, offen {mit_offen}"
    );
    assert!(
        mit("sperr_felder") >= 2,
        "ergebnis: zu wenige Sperrgruende mit Feldern"
    );
    for ampel in ["RED", "AMBER", "GREEN"] {
        assert!(
            ampeln.get(ampel).copied().unwrap_or(0) >= 1,
            "preflight meldet nie {ampel:?}: {ampeln:?}"
        );
    }
    // `nicht_gerechnet` fehlt mit Absicht: `NICHT_GERECHNET` ist leer, der Bereich bleibt leer.
    for bereich in [
        "widerspruch/flag",
        "widerspruch/partner",
        "widerspruch/alleinerziehend",
        "widerspruch/plausibilitaet",
        "hinweis/pauschale",
        "hinweis/betrag_vorlaeufig",
    ] {
        let n = bereiche.get(bereich).copied().unwrap_or(0);
        assert!(n >= 1, "preflight liefert nie {bereich}: {bereiche:?}");
    }
    assert!(
        bereiche
            .get("widerspruch/plausibilitaet")
            .copied()
            .unwrap_or(0)
            >= 6,
        "preflight: zu wenige Plausibilitaets-Widersprueche: {bereiche:?}"
    );
    // `deklaration`: jede Einspeisung von `mit_ring_werten` erscheint als Kz in einer Antwort.
    for (kz, was) in [
        ("E0205508", "Verpflegungskuerzung"),
        ("E1900401", "Kapital-Antrag"),
        ("E1901401", "genutzter Sparer-Pauschbetrag"),
        ("E0104109", "haushaltsnah Minijob"),
        ("E0107208", "haushaltsnah Dienstleistung"),
        ("E0111215", "haushaltsnah Handwerker"),
        ("E0701401", "V+V Einnahmen gesamt"),
        ("E0705701", "V+V Werbungskosten"),
        ("E0701601", "V+V Ueberschuss"),
        ("E0700206", "V+V Mieteinnahmen"),
        ("E0305104", "§ 22 Nr. 3 Einnahmen"),
        ("E0305201", "§ 22 Nr. 3 Werbungskosten"),
        ("E0108002", "Berufsausbildung"),
        ("E0240902", "§ 35c Foerderung"),
        ("E0801704", "GewSt zu zahlen"),
    ] {
        assert!(
            kz_je.contains_key(kz),
            "deklaration: {kz} ({was}) kommt in keiner Antwort vor: {kz_je:?}"
        );
    }
    assert!(
        gefuellt("dokumentiert") >= 1
            && gefuellt("anlage_instanzen") >= 1
            && gefuellt("person_b") >= 1,
        "deklaration: Aggregat, Anlage-Instanz oder Person B fehlt in allen Antworten"
    );
    // `/deklaration` sperrt: mindestens die fuenf Faelle g_dk_*, dazu die Sperrfaelle der Schleife
    // oben (gemessen: zehn Antworten, ein Grund je g_dk_* und `flag_konsistenz_offen` aus g_pf_rot).
    // Der Koerper ist genau `fall_id`, `grund`, `klartext`; der Satz ist nie leer.
    for g in [
        "kapital_semantik_offen",
        "verpflegung_dreimonatsfrist_aufteilung_offen",
        "einkunftsart_nicht_ring_faehig",
        "rechnung_unbar_offen",
        "partner_kegel_offen",
        "flag_konsistenz_offen",
    ] {
        assert!(
            gruende_dk.contains_key(g),
            "deklaration sperrt nie mit {g:?}: {gruende_dk:?}"
        );
    }
    assert!(
        gesperrt_dk.len() >= 10,
        "deklaration: nur {} Sperrfaelle",
        gesperrt_dk.len()
    );
    for d in &gesperrt_dk {
        let mut schluessel: Vec<&str> = d.as_object().unwrap().keys().map(String::as_str).collect();
        schluessel.sort_unstable();
        assert_eq!(schluessel, ["fall_id", "grund", "klartext"], "{d}");
        assert!(d["klartext"].as_str().is_some_and(|k| !k.is_empty()), "{d}");
    }
    // Der Rentenbeginn sperrt nur, wenn der Guard davor nichts findet — ein eigener Weg in `stand`.
    for g in ["rentenbeginn_offen", "flag_konsistenz_offen"] {
        assert!(
            gruende.iter().any(|x| x == g),
            "stand meldet nie den Sperrgrund {g:?}: {gruende:?}"
        );
    }
    let zu_wenig: Vec<_> = UNTERGRENZE
        .iter()
        .filter(|(r, n)| p.stat.erreicht.get(*r).copied().unwrap_or(0) < *n)
        .collect();
    assert!(zu_wenig.is_empty(), "Untergrenze verfehlt: {zu_wenig:?}");
    // Der Mitschnitt-Vergleich sah wirklich Zeilen: jede zulaessige Meldung oben schreibt eine.
    assert!(
        p.stat.flow_zeilen >= 8,
        "flow.jsonl: nur {} Zeilen verglichen",
        p.stat.flow_zeilen
    );
}

/// `ENUM_LABELS` aus `api_constants.py`, wie das Modul sie nach dem Laden haelt (samt der
/// `setdefault`-Ableitungen), als `[[feld, [[wert, text], ...]], ...]` in Einfuegereihenfolge.
const ENUM_LABELS_PY: &str = r#"
import importlib.util, json, sys
spec = importlib.util.spec_from_file_location("ac", "produkt/haut/api_constants.py")
ac = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ac)
json.dump([[f, [[w, t] for w, t in l.items()]] for f, l in ac.ENUM_LABELS.items()], sys.stdout)
"#;

/// Die erzeugte Tabelle `api::enum_labels` ist die Tabelle aus `api_constants.py`: Schluessel,
/// Werte, Texte und Reihenfolge. Aendert sich Python, wird dieser Test rot, bis
/// `tools/parity/gen_enum_labels.py` neu laeuft.
#[test]
fn enum_labels_gleich() {
    if skip() {
        return;
    }
    let aus = Command::new("python3")
        .args(["-c", ENUM_LABELS_PY])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert!(
        aus.status.success(),
        "{}",
        String::from_utf8_lossy(&aus.stderr)
    );
    let py: Value = serde_json::from_slice(&aus.stdout).unwrap();
    let rust: Value = api::enum_labels::ENUM_LABELS
        .iter()
        .map(|(f, l)| json!([f, l.iter().map(|(w, t)| json!([w, t])).collect::<Vec<_>>()]))
        .collect();
    assert_eq!(
        py, rust,
        "ENUM_LABELS weicht ab: python3 tools/parity/gen_enum_labels.py"
    );
    let werte: usize = rust
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e[1].as_array().unwrap().len())
        .sum();
    println!(
        "enum_labels_gleich: {} Felder, {werte} Werte, 0 Abweichungen",
        rust.as_array().unwrap().len()
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
    let mut p = Paar::neu(tmp.path(), false, false, &seed);
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
    let p = Paar::neu(tmp.path(), false, false, &seed);
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
    // 1b. `stand` auf einer Akte mit Jahr 10^38 (9c, Stufe 2, gewollte Abweichung): Python rechnet mit
    //     jedem `int` weiter und antwortet 200 mit einer Spanne fuer ein Jahr ohne Parameter; Rust
    //     kennt nur 2024-2026 (`domain::Vz`) und antwortet 500 statt einer Zahl fuer ein falsches Jahr.
    //     Eine solche Akte entsteht nur von Hand (`POST /fall` prueft `params/`).
    let (py, rs) =
        zweimal(&Anfrage::neu("stand seed_big", "GET", "/fall/seed_big/stand").token(&alice));
    println!(
        "  stand seed_big: py={} {} | rs={} {}",
        py.status,
        String::from_utf8_lossy(&py.body)
            .chars()
            .take(200)
            .collect::<String>(),
        rs.status,
        String::from_utf8_lossy(&rs.body)
            .chars()
            .take(200)
            .collect::<String>()
    );
    assert_eq!((py.status, rs.status), (200, 500));
    assert!(String::from_utf8_lossy(&rs.body).contains("kein unterstuetzter Veranlagungszeitraum"));
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
    // 3. Token mit gueltiger Signatur, `sub` verfehlt `_USER_RE` (9c/0b, `auth::Username`):
    //    Python nimmt den Namen als uid (Fall fehlt: 404), Rust zaehlt das Token als keines (401).
    let (py, rs) =
        zweimal(&Anfrage::neu("sub ab", "DELETE", "/fall/nix").token(&token("ab", GEHEIMNIS)));
    println!("  sub=ab: py={} | rs={}", py.status, rs.status);
    assert_eq!((py.status, rs.status), (404, 401));
    // 4. Login mit Nicht-Text-Passwort fuer einen vorhandenen Nutzer: Python 500 (AttributeError), Rust 401.
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
    // 5. `POST /kontoauszug`, JSON als Text (`json_laden` in `api::kontoauszug`). a) `NaN` im Datum einer
    //    gebuchten Buchung und b) `Infinity` im Zweck einer Ausgabe waren hier dokumentierte Abweichungen
    //    (Python 500, Rust 200); beide Server weisen sie seit Backlog `falldatei-mit-nan-liest-rust-als-text`,
    //    AK4, mit 422 ab (Faelle `json Text Datum ...` und `json Text Zweck ...` in `kontoauszug_faelle`).
    //    c) ein einzelnes Surrogat-Escape im JSON-Text: Python nimmt es an und scheitert beim Schreiben
    //       der Akte (500, `UnicodeEncodeError`), `serde_json` weist es schon beim Lesen ab (400).
    let neu = Anfrage::neu("dok Fall", "POST", "/fall").token(&alice).json(
        &json!({"fall_id": "dok_surrogat", "scheibe": "gesamt", "veranlagungszeitraum": 2025}),
    );
    let _ = zweimal(&neu);
    let (py, rs) = zweimal(
        &Anfrage::neu("dok kontoauszug Surrogat", "POST", "/fall/dok_surrogat/kontoauszug")
            .token(&alice)
            .json(&json!({"format": "json", "inhalt":
                r#"[{"datum": "\ud800", "betrag": -5000, "verwendungszweck": "Spende"}]"#})),
    );
    println!(
        "  kontoauszug Surrogat: py={} {} | rs={} {}",
        py.status,
        String::from_utf8_lossy(&py.body),
        rs.status,
        String::from_utf8_lossy(&rs.body)
    );
    assert_eq!((py.status, rs.status), (500, 400));
    // 6. Fremddienst-Zahlen und -Gestalten (Vault `decisions/fremddienst-zahlfehler-nur-in-python-bleiben-
    //    bis-zum-cutover`, Messung `berichte/k9-fremddienst.md`). Vier Wege, auf denen Python bei einer
    //    Antwort des Sprachmodells oder des Karten-Dienstes abstuerzt oder eine Antwort schickt, die kein
    //    JSON ist, und Rust sauber zurueckfaellt: A1 `rechenweg`/`vorschlag_wert` mit NaN/Infinity (Python
    //    200 ohne gueltiges JSON, Rust 200), A2 `aussage` mit Infinity/1e400 (Python 500, Rust 200),
    //    A3 `distance` mit NaN/Infinity (Python 500, Rust 503), B `content`/`kategorie` als Liste/Objekt/
    //    Zahl (Python 500, Rust 200). Nutzerwirkung: Python bricht ab, Rust faellt zurueck; die Akte bleibt
    //    in allen Faellen unberuehrt. Rust ist die Soll-Seite (Korrektheit vor Paritaet, `REWRITE_PLAN.md`
    //    §4); Python wird bis zum Cutover nicht repariert. Die Faelle stehen mit Begruendung in
    //    `extern_stub/fremd_abweichungen.rs`; wird Python repariert oder Rust anders, faellt dieser Eintrag.
    extern_stub::fremd_abweichungen::pruefe(&["A1", "A2", "A3", "B"]);
    // 7. Chat-Antwort mit einem einzelnen Surrogat-Escape in `begruendung` (Messung `berichte/k9-
    //    fremddienst.md`, Beobachtung ohne Ticket). Python weist nur das betroffene Feld ab
    //    (`abgelehnt_gruende`: `UnicodeEncodeError`) und behaelt die uebrigen Vorschlaege der Antwort;
    //    Rust verwirft Stufe 3 als Ganzes (serde_json lehnt das Surrogat beim Lesen ab, 200 ohne
    //    Vorschlag, `kein_feld`). Nutzerwirkung: in Rust fehlen die gesunden Vorschlaege derselben
    //    Antwort; die Akte bleibt unberuehrt. NICHT entschieden, welche Seite korrekt ist (REWRITE_PLAN
    //    §4: wo unklar ist, was korrekt ist, bleibt Python und die Stelle geht als Frage an Julius).
    //    Der Fall in `extern_stub/fremd_abweichungen.rs` haelt beide Seiten fest und faellt, wenn eine
    //    sich aendert; mit der Entscheidung wird der Eintrag zu einer der beiden Formen.
    extern_stub::fremd_abweichungen::pruefe(&["C"]);
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

/// `x__1` ist keine Instanz (Entscheidung 2026-10-03, Zaehler `[2-9]|[1-9][0-9]+`): `POST /event` weist
/// es mit 400 ab, waehrend `x`, `x__2` und `x__10` durchgehen; `frage` loest `x__2` auf das Basisfeld
/// auf, `x__1` nicht (404). Beide Server antworten gleich, sonst meldet `Paar::anfrage` es. Die Basis
/// `vv_einnahmen` traegt eine `instanz_gruppe` und liegt in `gesamt`: an einem Feld ohne Gruppe kaeme
/// die 400 auch vor der Regel (s. `ep_arbeitstage__2` in `event_faelle`).
#[test]
fn instanz_eins_wird_abgewiesen() {
    if skip() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let seed = tmp.path().join("seed");
    schreibe_seed(&seed);
    let mut p = Paar::neu(tmp.path(), false, false, &seed);
    let alice = token("alice", GEHEIMNIS);
    let mut a = |m: &str, pfad: &str, body: Option<Value>| {
        let x = Anfrage::neu(&format!("instanz {m} {pfad}"), m, pfad).token(&alice);
        let antwort = p.anfrage(&body.map_or(x.clone(), |b| x.json(&b)), Modus::Voll);
        (p.stat.letzter, antwort)
    };
    let (status, _) = a(
        "POST",
        "/fall",
        Some(json!({"fall_id": "g_inst", "scheibe": "gesamt", "veranlagungszeitraum": 2025})),
    );
    assert_eq!(status, 201);
    let event = "/fall/g_inst/event";
    for feld in ["vv_einnahmen", "vv_einnahmen__2", "vv_einnahmen__10"] {
        let (status, _) = a("POST", event, Some(ereignis(feld, &json!(1_500_000), None)));
        assert_eq!(status, 201, "{feld}");
    }
    for feld in ["vv_einnahmen__1", "vv_einnahmen__0", "vv_einnahmen__02"] {
        let (status, body) = a("POST", event, Some(ereignis(feld, &json!(1_500_000), None)));
        assert_eq!(status, 400, "{feld}");
        assert_eq!(
            body.unwrap()["fehler"],
            format!("feld_id '{feld}' nicht in dieser Scheibe")
        );
    }
    for (feld, soll) in [
        ("vv_einnahmen", 200),
        ("vv_einnahmen__2", 200),
        ("vv_einnahmen__10", 200),
        ("vv_einnahmen__1", 404),
        ("vv_einnahmen__0", 404),
    ] {
        let (status, _) = a("GET", &format!("/fall/g_inst/feld/{feld}/frage"), None);
        assert_eq!(status, soll, "frage {feld}");
    }
    p.zustand_vergleichen("instanz_eins");
    p.bericht("instanz_eins");
    assert!(
        p.stat.abweichungen.is_empty(),
        "Abweichungen: {:?}",
        p.stat.abweichungen
    );
}
