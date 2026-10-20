//! Schmaler WebDriver-Client: nur Standardbibliothek und `serde_json`, kein neues Crate.
//!
//! Gemessen in Stufe 1 (Nr. 72, Sonde `spike_webdriver.py`):
//! - `chromedriver` schliesst die Verbindung nicht, auch nicht bei `Connection: close`. Lesen bis zum Ende
//!   der Verbindung haengt; der Client liest den Kopf und dann genau `Content-Length` Bytes.
//! - Headless-Chromium hat eine Mindestbreite von 500 px. Die Sichtflaeche (360 px) kommt per CDP-Durchreichung
//!   (`POST /session/{id}/goog/cdp/execute`, `Emulation.setDeviceMetricsOverride`), ein reiner HTTP-Weg.
//! - Es gibt keine Netz-Abfangfunktion (`page.route`). Die Fluesse brauchen sie nicht: der echte Dienst
//!   antwortet echt, das Sprachmodell ersetzt ein lokaler Ersatzdienst (`modell.rs`).
//!
//! Aufraeumen: [`Sitzung`] schliesst beim Fallenlassen die Sitzung und beendet `chromedriver`, auch bei einer
//! Panik. Ein Browser-Test laeuft nur, solange er die Sperre [`seriell`] haelt: ein Browser und ein Dienst
//! brauchen je ca. 0,5 bis 1 GB (Schaetzung), und viele Rechner haben nur 5 bis 9 GB frei.
//!
//! Fehlt Browser oder Treiber, ist der Test ROT. Nur `TAXGRAPH_OHNE_BROWSER=1` erlaubt das Ueberspringen
//! (lokal; die CI setzt es seit 2026-10-15 nicht mehr, Julius d-8; gemessen im Lauf 38459954734: der Runner bringt Chrome
//! und `chromedriver` mit, die Tests liefen ohne Skip-Zeile). Die Sperre [`seriell`] gilt nur innerhalb EINES Prozesses; unter
//! `cargo nextest` (ein Prozess je Test) stellt die Gruppe `oberflaeche-browser` in `rust/.config/nextest.toml` die Reihenfolge her.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

static SERIELL: Mutex<()> = Mutex::new(());

/// Die Sperre fuer einen Browser-Test. Wer sie haelt, darf Dienst und Browser starten.
pub fn seriell() -> MutexGuard<'static, ()> {
    SERIELL.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `true`, wenn `TAXGRAPH_OHNE_BROWSER=1` das Ueberspringen erlaubt.
pub fn ohne_browser_erlaubt() -> bool {
    std::env::var("TAXGRAPH_OHNE_BROWSER").is_ok_and(|v| v == "1")
}

fn finde(umgebung: &str, kandidaten: &[&str]) -> Option<PathBuf> {
    if let Ok(p) = std::env::var(umgebung) {
        let p = PathBuf::from(p);
        return p.is_file().then_some(p);
    }
    kandidaten.iter().map(PathBuf::from).find(|p| p.is_file())
}

/// Ein HTTP/1.1-Aufruf gegen `127.0.0.1:port`. Rumpf und Antwort sind JSON; ein leerer Rumpf ist `Null`.
/// Liest den Kopf und genau `Content-Length` Bytes; fehlt die Angabe, bis zum Ende der Verbindung.
pub fn http(port: u16, methode: &str, pfad: &str, rumpf: Option<&Value>, kopf: &[(&str, &str)]) -> (u16, Value) {
    let (status, _kopf, body) = http_roh(port, methode, pfad, rumpf, kopf);
    let wert = if body.iter().all(u8::is_ascii_whitespace) {
        Value::Null
    } else {
        serde_json::from_slice(&body)
            .unwrap_or_else(|e| panic!("{methode} {pfad}: kein JSON ({e}): {}", String::from_utf8_lossy(&body)))
    };
    (status, wert)
}

/// Wie [`http`], aber mit Antwortkopf und rohem Rumpf (fuer Seiten, die kein JSON sind).
pub fn http_roh(
    port: u16,
    methode: &str,
    pfad: &str,
    rumpf: Option<&Value>,
    kopf: &[(&str, &str)],
) -> (u16, String, Vec<u8>) {
    let daten = rumpf.map(Value::to_string).unwrap_or_default();
    let mut anfrage = format!("{methode} {pfad} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    if rumpf.is_some() {
        anfrage.push_str("Content-Type: application/json\r\n");
    }
    anfrage.push_str(&format!("Content-Length: {}\r\n", daten.len()));
    for (k, v) in kopf {
        anfrage.push_str(&format!("{k}: {v}\r\n"));
    }
    anfrage.push_str("\r\n");
    anfrage.push_str(&daten);
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap_or_else(|e| panic!("{methode} {pfad}: keine Verbindung: {e}"));
    s.set_read_timeout(Some(Duration::from_secs(120))).unwrap();
    s.write_all(anfrage.as_bytes()).unwrap();
    let mut puffer = Vec::new();
    let mut stueck = vec![0u8; 65536];
    let kopf_ende = loop {
        if let Some(i) = puffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        let n = s.read(&mut stueck).unwrap_or(0);
        assert!(n > 0, "{methode} {pfad}: Verbindung endet vor dem Antwortkopf");
        puffer.extend_from_slice(&stueck[..n]);
    };
    let kopf_text = String::from_utf8_lossy(&puffer[..kopf_ende]).into_owned();
    let mut body = puffer[kopf_ende + 4..].to_vec();
    let status: u16 = kopf_text.split_whitespace().nth(1).and_then(|z| z.parse().ok()).unwrap_or(0);
    assert!(
        !kopf_text.to_ascii_lowercase().contains("transfer-encoding: chunked"),
        "{methode} {pfad}: Antwort ist chunked, der schmale Client liest nur Content-Length"
    );
    let laenge = kopf_text.lines().find_map(|z| {
        let z = z.to_ascii_lowercase();
        z.strip_prefix("content-length:").and_then(|v| v.trim().parse::<usize>().ok())
    });
    loop {
        if laenge.is_some_and(|l| body.len() >= l) {
            break;
        }
        let n = s.read(&mut stueck).unwrap_or(0);
        if n == 0 {
            break;
        }
        body.extend_from_slice(&stueck[..n]);
    }
    (status, kopf_text, body)
}

fn freier_port() -> u16 {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().port()
}

/// Eine laufende Browser-Sitzung. Beim Fallenlassen: Sitzung schliessen, `chromedriver` beenden.
pub struct Sitzung {
    treiber: Child,
    port: u16,
    sid: String,
    profil: tempfile::TempDir,
}

impl Drop for Sitzung {
    fn drop(&mut self) {
        let _ = http_versuch(self.port, "DELETE", &format!("/session/{}", self.sid));
        let _ = self.treiber.kill();
        let _ = self.treiber.wait();
    }
}

fn http_versuch(port: u16, methode: &str, pfad: &str) -> std::io::Result<()> {
    let mut s = TcpStream::connect(("127.0.0.1", port))?;
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    s.write_all(format!("{methode} {pfad} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\n\r\n").as_bytes())?;
    let mut puffer = [0u8; 4096];
    let _ = s.read(&mut puffer)?;
    Ok(())
}

impl Sitzung {
    /// Startet `chromedriver` und einen frischen Headless-Browser mit Wegwerf-Profil in der Sichtbreite
    /// `breite` mal `hoehe`. `None`, wenn Browser oder Treiber fehlen und `TAXGRAPH_OHNE_BROWSER=1` gesetzt ist.
    pub fn neu(breite: u32, hoehe: u32) -> Option<Self> {
        let treiber = finde("CHROMEDRIVER", &["/usr/bin/chromedriver", "/usr/local/bin/chromedriver", "/usr/lib/chromium/chromedriver"]);
        let browser = finde(
            "CHROME_BIN",
            &["/usr/bin/chromium", "/usr/bin/chromium-browser", "/usr/bin/google-chrome", "/usr/bin/google-chrome-stable"],
        );
        let (Some(treiber), Some(browser)) = (treiber, browser) else {
            assert!(
                ohne_browser_erlaubt(),
                "Browser oder chromedriver fehlt (CHROME_BIN / CHROMEDRIVER setzen); \
                 nur TAXGRAPH_OHNE_BROWSER=1 erlaubt das Ueberspringen"
            );
            { use std::io::Write; let _ = writeln!(std::io::stderr(), "UEBERSPRUNGEN: kein Browser oder chromedriver, TAXGRAPH_OHNE_BROWSER=1"); }
            return None;
        };
        let port = freier_port();
        let kind = Command::new(treiber)
            .arg(format!("--port={port}"))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("chromedriver startet nicht");
        let profil = tempfile::tempdir().unwrap();
        let mut sitzung = Self { treiber: kind, port, sid: String::new(), profil };
        let ende = Instant::now() + Duration::from_secs(20);
        while http_versuch(port, "GET", "/status").is_err() {
            assert!(Instant::now() < ende, "chromedriver antwortet nicht auf /status");
            std::thread::sleep(Duration::from_millis(100));
        }
        let (status, antwort) = http(
            port,
            "POST",
            "/session",
            Some(&json!({"capabilities": {"alwaysMatch": {
                "browserName": "chrome",
                "goog:loggingPrefs": {"browser": "ALL"},
                "goog:chromeOptions": {
                    "binary": browser,
                    "args": [
                        "--headless=new", "--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage",
                        format!("--user-data-dir={}", sitzung.profil.path().display()),
                        format!("--window-size={breite},{hoehe}"),
                    ],
                },
            }}})),
            &[],
        );
        assert_eq!(status, 200, "Sitzung startet nicht: {antwort}");
        antwort["value"]["sessionId"].as_str().expect("keine sessionId").clone_into(&mut sitzung.sid);
        sitzung.befehl("POST", "/timeouts", Some(&json!({"script": 120_000, "pageLoad": 60_000})));
        sitzung.sichtflaeche(breite, hoehe);
        Some(sitzung)
    }

    /// Ein WebDriver-Befehl. Jeder Status ausser 200 ist ein Fehler mit Pfad und Antwort.
    pub fn befehl(&self, methode: &str, pfad: &str, rumpf: Option<&Value>) -> Value {
        let (status, antwort) = self.befehl_roh(methode, pfad, rumpf);
        assert_eq!(status, 200, "{methode} {pfad} -> {status} {antwort}");
        antwort["value"].clone()
    }

    fn befehl_roh(&self, methode: &str, pfad: &str, rumpf: Option<&Value>) -> (u16, Value) {
        let leer = json!({});
        let rumpf = rumpf.or((methode == "POST").then_some(&leer));
        http(self.port, methode, &format!("/session/{}{pfad}", self.sid), rumpf, &[])
    }

    /// Ein CDP-Befehl ueber die Durchreichung von `chromedriver`.
    pub fn cdp(&self, befehl: &str, parameter: &Value) -> Value {
        self.befehl("POST", "/goog/cdp/execute", Some(&json!({"cmd": befehl, "params": parameter})))
    }

    /// `prefers-reduced-motion: reduce` an oder aus.
    pub fn reduzierte_bewegung(&self, an: bool) {
        let wert = if an { "reduce" } else { "no-preference" };
        self.cdp("Emulation.setEmulatedMedia", &json!({"features": [{"name": "prefers-reduced-motion", "value": wert}]}));
    }

    /// Die Sichtflaeche auf `breite` mal `hoehe` setzen.
    pub fn sichtflaeche(&self, breite: u32, hoehe: u32) {
        self.cdp(
            "Emulation.setDeviceMetricsOverride",
            &json!({"width": breite, "height": hoehe, "deviceScaleFactor": 1, "mobile": false}),
        );
    }

    pub fn oeffne(&self, url: &str) {
        self.befehl("POST", "/url", Some(&json!({"url": url})));
    }

    pub fn neu_laden(&self) {
        self.befehl("POST", "/refresh", None);
    }

    /// Ein JavaScript-Rumpf, `return` liefert den Wert. Argumente stehen in `arguments`.
    pub fn skript(&self, js: &str, args: &[Value]) -> Value {
        self.befehl("POST", "/execute/sync", Some(&json!({"script": js, "args": args})))
    }

    /// Ein JavaScript-Ausdruck (ohne `return`).
    pub fn ausdruck(&self, expr: &str) -> Value {
        self.skript(&format!("return ({expr});"), &[])
    }

    /// Ein asynchrones Skript: der Rumpf ruft den letzten Parameter (`arguments[arguments.length - 1]`) mit dem Ergebnis.
    pub fn skript_async(&self, js: &str, args: &[Value]) -> Value {
        self.befehl("POST", "/execute/async", Some(&json!({"script": js, "args": args})))
    }

    /// Wartet, bis der Ausdruck wahr ist (nicht `null`, `false`, `0`, `""`); sonst Fehler mit `was`.
    pub fn warte_bis(&self, expr: &str, sekunden: u64, was: &str) -> Value {
        let ende = Instant::now() + Duration::from_secs(sekunden);
        loop {
            let (status, antwort) = self.befehl_roh(
                "POST",
                "/execute/sync",
                Some(&json!({"script": format!("return ({expr});"), "args": []})),
            );
            if status == 200 {
                let v = &antwort["value"];
                if !(v.is_null() || *v == json!(false) || *v == json!(0) || *v == json!("")) {
                    return v.clone();
                }
            }
            assert!(Instant::now() < ende, "Zeitlimit {sekunden} s: {was} (Ausdruck: {expr}; zuletzt {status} {antwort})");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Wie [`Self::warte_bis`], aber `None` statt eines Fehlers, wenn die Zeit ablaeuft.
    pub fn try_warte_bis(&self, expr: &str, sekunden: u64) -> Option<Value> {
        let ende = Instant::now() + Duration::from_secs(sekunden);
        loop {
            let (status, antwort) = self.befehl_roh(
                "POST",
                "/execute/sync",
                Some(&json!({"script": format!("return ({expr});"), "args": []})),
            );
            if status == 200 {
                let v = &antwort["value"];
                if !(v.is_null() || *v == json!(false) || *v == json!(0) || *v == json!("")) {
                    return Some(v.clone());
                }
            }
            if Instant::now() >= ende {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// Eine JavaScript-Funktion (als Text, auch `async`) mit Argumenten aufrufen und ihr Versprechen abwarten.
    /// Wirft die Funktion, ist das ein Fehler des Tests.
    pub fn auswerten(&self, funktion: &str, args: &[Value]) -> Value {
        let js = format!(
            "const done = arguments[arguments.length - 1]; const a = Array.prototype.slice.call(arguments, 0, -1); \
             Promise.resolve().then(() => ({funktion})(...a)).then(\
             v => done({{w: v === undefined ? null : v}}), e => done({{f: String(e)}}));"
        );
        let r = self.skript_async(&js, args);
        assert!(r.get("f").is_none(), "Skriptfehler: {}", r["f"]);
        r["w"].clone()
    }

    /// Das erste Element zu `css` (Element-Kennung) oder `None`.
    pub fn finde(&self, css: &str) -> Option<String> {
        let (status, antwort) = self.befehl_roh("POST", "/element", Some(&json!({"using": "css selector", "value": css})));
        (status == 200).then(|| antwort["value"].as_object().and_then(|o| o.values().next()).and_then(Value::as_str).map(str::to_owned))?
    }

    /// Wartet auf ein SICHTBARES Element zu `css` (wie Playwrights `wait_for_selector`).
    pub fn warte_sichtbar(&self, css: &str, sekunden: u64) {
        let sel = serde_json::to_string(css).unwrap();
        self.warte_bis(&format!("{}({sel})", SICHTBAR_JS), sekunden, &format!("{css} wird sichtbar"));
    }

    /// Wartet, bis `css` nicht mehr sichtbar ist oder fehlt.
    pub fn warte_unsichtbar(&self, css: &str, sekunden: u64) {
        let sel = serde_json::to_string(css).unwrap();
        self.warte_bis(&format!("!{}({sel})", SICHTBAR_JS), sekunden, &format!("{css} wird unsichtbar"));
    }

    /// Ist das Element da und sichtbar (nicht `hidden`, nicht `display:none`, hat eine Box)?
    pub fn sichtbar(&self, css: &str) -> bool {
        let sel = serde_json::to_string(css).unwrap();
        self.ausdruck(&format!("{}({sel})", SICHTBAR_JS)) == json!(true)
    }

    pub fn anzahl(&self, css: &str) -> usize {
        let sel = serde_json::to_string(css).unwrap();
        self.ausdruck(&format!("document.querySelectorAll({sel}).length")).as_u64().unwrap_or(0) as usize
    }

    /// Klick wie ein Nutzer: wartet auf ein sichtbares, klickbares Element, dann WebDriver-Klick.
    pub fn klick(&self, css: &str) {
        self.warte_sichtbar(css, 10);
        let ende = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(id) = self.finde(css) {
                let (status, antwort) = self.befehl_roh("POST", &format!("/element/{id}/click"), None);
                if status == 200 {
                    return;
                }
                assert!(Instant::now() < ende, "Klick auf {css} scheitert: {status} {antwort}");
            } else {
                assert!(Instant::now() < ende, "Element {css} fehlt");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Ein Klick-Versuch ohne Warten: `true`, wenn der Klick das Element erreicht hat. Ein gesperrtes (`inert`) oder
    /// verdecktes Element erreicht er nicht (`element click intercepted`); `is_enabled()` taugte dafuer nicht.
    pub fn klick_versuch(&self, css: &str) -> bool {
        self.finde(css).is_some_and(|id| self.befehl_roh("POST", &format!("/element/{id}/click"), None).0 == 200)
    }

    /// Maus auf das Element, Taste `ms` Millisekunden halten, loslassen (Halte-Geste fuer KI-Vorschlaege).
    pub fn halte(&self, css: &str, ms: u64) {
        self.warte_sichtbar(css, 10);
        let id = self.finde(css).unwrap_or_else(|| panic!("Element {css} fehlt"));
        self.befehl(
            "POST",
            "/actions",
            Some(&json!({"actions": [{
                "type": "pointer", "id": "maus", "parameters": {"pointerType": "mouse"},
                "actions": [
                    {"type": "pointerMove", "duration": 0, "x": 0, "y": 0, "origin": {"element-6066-11e4-a52e-4f735466cecf": id}},
                    {"type": "pointerDown", "button": 0},
                    {"type": "pause", "duration": ms},
                    {"type": "pointerUp", "button": 0},
                ],
            }]})),
        );
    }

    /// Setzt den Wert wie Playwrights `fill`: Fokus, Wert, `input`- und `change`-Ereignis. Leerer Text leert das Feld.
    pub fn fuelle(&self, css: &str, text: &str) {
        self.warte_sichtbar(css, 10);
        self.skript(
            "const e = document.querySelector(arguments[0]); e.focus(); e.value = arguments[1]; \
             e.dispatchEvent(new Event('input', {bubbles: true})); e.dispatchEvent(new Event('change', {bubbles: true}));",
            &[json!(css), json!(text)],
        );
    }

    /// Tippt `text` wie ein Nutzer: Fokus, Feld leeren, Tasten senden (WebDriver `clear` und `value`), nicht `.value` setzen.
    /// Die `input`-Ereignisse kommen vom Browser selbst.
    pub fn tippe(&self, css: &str, text: &str) {
        self.warte_sichtbar(css, 10);
        let id = self.finde(css).unwrap_or_else(|| panic!("Element {css} fehlt"));
        self.befehl("POST", &format!("/element/{id}/clear"), None);
        self.befehl("POST", &format!("/element/{id}/value"), Some(&json!({"text": text})));
    }

    /// Waehlt in einem Dropdown (`<select>`) die Option mit `wert`: Klick auf das `<option>`, wie ihn WebDriver kennt.
    pub fn waehle_option(&self, select_css: &str, wert: &str) {
        self.warte_sichtbar(select_css, 10);
        let css = format!("{select_css} option[value=\"{wert}\"]");
        let id = self.finde(&css).unwrap_or_else(|| panic!("Option {css} fehlt"));
        self.befehl("POST", &format!("/element/{id}/click"), None);
    }

    /// Text, wie ihn der Nutzer sieht (`innerText`); leer, wenn das Element fehlt.
    pub fn text(&self, css: &str) -> String {
        let sel = serde_json::to_string(css).unwrap();
        self.ausdruck(&format!("(document.querySelector({sel}) || {{}}).innerText || ''")).as_str().unwrap_or("").to_owned()
    }

    /// `textContent` (auch von versteckten Elementen); leer, wenn das Element fehlt.
    pub fn textinhalt(&self, css: &str) -> String {
        let sel = serde_json::to_string(css).unwrap();
        self.ausdruck(&format!("(document.querySelector({sel}) || {{}}).textContent || ''")).as_str().unwrap_or("").to_owned()
    }

    /// Der Wert eines Eingabefelds.
    pub fn wert(&self, css: &str) -> String {
        let sel = serde_json::to_string(css).unwrap();
        self.ausdruck(&format!("(document.querySelector({sel}) || {{}}).value ?? ''")).as_str().unwrap_or("").to_owned()
    }

    /// Box des Elements: `(links, oben, breite, hoehe)` in CSS-Pixeln.
    pub fn box_von(&self, css: &str) -> (f64, f64, f64, f64) {
        let sel = serde_json::to_string(css).unwrap();
        let r = self.ausdruck(&format!(
            "(() => {{ const r = document.querySelector({sel}).getBoundingClientRect(); return [r.left, r.top, r.width, r.height]; }})()"
        ));
        let z = |i: usize| r[i].as_f64().unwrap_or(f64::NAN);
        (z(0), z(1), z(2), z(3))
    }

    /// Eintraege der Browser-Konsole seit dem letzten Aufruf (`SEVERE` = Fehler).
    pub fn konsole(&self) -> Vec<(String, String)> {
        let (status, antwort) = self.befehl_roh("POST", "/se/log", Some(&json!({"type": "browser"})));
        if status != 200 {
            return Vec::new();
        }
        antwort["value"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|e| (e["level"].as_str().unwrap_or("").to_owned(), e["message"].as_str().unwrap_or("").to_owned()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// JavaScript-Funktion: sichtbar = da, nicht `hidden`, `display` und `visibility` zeigen, hat eine Box.
const SICHTBAR_JS: &str = "((s) => { const e = document.querySelector(s); if (!e) return false; \
     const st = getComputedStyle(e); return !e.hidden && st.display !== 'none' && st.visibility !== 'hidden' && e.getClientRects().length > 0; })";
