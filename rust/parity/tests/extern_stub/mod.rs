//! Stub fuer die zwei externen Dienste der Routen `chat` (LLM) und `entfernung` (Karten-Dienst) im
//! Vergleichslauf `api_http_paritaet` (Folge 6, Vault `decisions/rust-9c-generator-je-route-und-flow-
//! portieren` Punkt 4).
//!
//! Je Server-Instanz laeuft EIN Stub auf einem eigenen Port. Beide Server bekommen dieselben
//! Umgebungsvariablen: `LLM_API_BASE` (`<stub>/llm`), `ORS_API_BASE` (`<stub>/ors`) und
//! synthetische Schluessel. Der Stub antwortet nach einem Skript (Daten, s. [`Schritt`]), zeichnet
//! jede Anfrage auf und ersetzt dabei die Schluessel durch `<KEY>`. So vergleicht der Lauf zwei
//! Dinge: was der Server dem Client antwortet UND was er an den Dienst sendet.
//!
//! Kein Netz: die Basis zeigt auf `127.0.0.1`, die Schluessel sind erfunden (`SCHLUESSEL_*`).
//! Nicht im Stub: Zeitgrenzen (ORS 8 s, LLM 30 s) und Verbindungsabbrueche. Die pruefen die
//! Crate-Tests (`llm_paritaet`, `llm::ors`); ein einziger langsamer Fall (`C5`, 3 s Wiederholung)
//! bleibt hier.
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{json, Value};

/// Der Karten-Schluessel des Laufs. Erfunden; steht nur in der Umgebung der Server und im Stub.
pub(super) const SCHLUESSEL_ORS: &str = "h8-synthetischer-ors-schluessel";
/// Der LLM-Schluessel des Laufs. Erfunden.
pub(super) const SCHLUESSEL_LLM: &str = "h8-synthetischer-llm-schluessel";
/// Das Modell, das beide Server senden.
pub(super) const MODELL: &str = "stub/modell";

/// Was ein Schritt des Skripts antwortet.
#[derive(Clone, Debug)]
pub(super) enum Inhalt {
    /// Der Koerper, wie er ist.
    Roh(String),
    /// Chat-Stufe `zuordnung`: die ersten `n` Regel-Kennungen aus dem Systemprompt der Anfrage, als
    /// Antwort des Modells verpackt. Die Kennungen kennt erst die Anfrage.
    Regeln(usize),
}

/// Ein Schritt des Skripts: die erste Anfrage mit passendem Schluessel bekommt diese Antwort.
#[derive(Clone, Debug)]
pub(super) struct Schritt {
    /// `chat:aussagen`, `chat:zuordnung`, `chat:dialog`, `ors:geocode`, `ors:route` oder `*` (jede).
    pub fuer: &'static str,
    pub status: u16,
    pub inhalt: Inhalt,
    /// Wartezeit vor der Antwort.
    pub warte_ms: u64,
    /// Bleibt im Skript und beantwortet auch die naechsten Treffer (Wiederholungen des Clients).
    pub oft: bool,
}

impl Schritt {
    pub(super) fn roh(fuer: &'static str, status: u16, body: &str) -> Self {
        Self {
            fuer,
            status,
            inhalt: Inhalt::Roh(body.to_owned()),
            warte_ms: 0,
            oft: false,
        }
    }

    /// Eine Modell-Antwort mit `inhalt` als Text der ersten Wahl.
    pub(super) fn chat(fuer: &'static str, inhalt: &Value, finish: &str) -> Self {
        Self::roh(fuer, 200, &chat_huelle(&inhalt.to_string(), finish))
    }

    pub(super) fn regeln(n: usize) -> Self {
        Self {
            inhalt: Inhalt::Regeln(n),
            ..Self::roh("chat:zuordnung", 200, "")
        }
    }

    pub(super) fn immer(mut self) -> Self {
        self.oft = true;
        self
    }
}

/// Die Antwort eines OpenAI-kompatiblen Dienstes (`llm_client._inhalt` liest `choices[0]`).
fn chat_huelle(inhalt_text: &str, finish: &str) -> String {
    json!({
        "provider": "StubAnbieter",
        "choices": [{"finish_reason": finish, "message": {"content": inhalt_text}}]
    })
    .to_string()
}

/// Eine aufgezeichnete Anfrage, schluesselfrei. Gleiche Anfragen zweier Server sind gleich.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Gesehen {
    /// Wie im Skript: `chat:<schema>`, `ors:geocode`, `ors:route`, sonst `unbekannt`.
    pub schluessel: String,
    pub methode: String,
    /// Pfad samt Query, Schluessel als `<KEY>`.
    pub pfad: String,
    /// Nur `content-type` und `authorization` (Schluessel als `<KEY>`); `Host`, `User-Agent`,
    /// `Content-Length` und Verbindungskopf sind Sache des Clients und gehoeren nicht zum Vertrag.
    pub kopf: BTreeMap<String, String>,
    /// Chat: der JSON-Koerper in kanonischer Schreibweise (Python `json.dumps` und `serde_json`
    /// trennen anders); ORS: der Text, wie er kam.
    pub body: String,
    /// Kein Schritt im Skript passte; der Stub antwortete 599.
    pub ungeplant: bool,
}

#[derive(Default)]
struct Innen {
    skript: Vec<Schritt>,
    gesehen: Vec<Gesehen>,
}

/// Ein lokaler Dienst auf `127.0.0.1`, der nach Skript antwortet und mitschreibt.
pub(super) struct Stub {
    port: u16,
    innen: Arc<Mutex<Innen>>,
    halt: Arc<AtomicBool>,
    faden: Option<JoinHandle<()>>,
}

impl Stub {
    pub(super) fn starte() -> Self {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let innen = Arc::new(Mutex::new(Innen::default()));
        let halt = Arc::new(AtomicBool::new(false));
        let (i, h) = (Arc::clone(&innen), Arc::clone(&halt));
        let faden = std::thread::spawn(move || {
            for strom in l.incoming() {
                if h.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(s) = strom {
                    bediene(s, &i);
                }
            }
        });
        Self {
            port,
            innen,
            halt,
            faden: Some(faden),
        }
    }

    /// Basis fuer `LLM_API_BASE`.
    pub(super) fn basis_llm(&self) -> String {
        format!("http://127.0.0.1:{}/llm", self.port)
    }

    /// Basis fuer `ORS_API_BASE`.
    pub(super) fn basis_ors(&self) -> String {
        format!("http://127.0.0.1:{}/ors", self.port)
    }

    /// Die Umgebung, mit der ein Server diesen Stub benutzt.
    pub(super) fn umgebung(&self) -> Vec<(&'static str, String)> {
        vec![
            ("LLM_API_BASE", self.basis_llm()),
            ("LLM_MODEL", MODELL.to_owned()),
            ("LLM_API_KEY", SCHLUESSEL_LLM.to_owned()),
            ("ORS_API_KEY", SCHLUESSEL_ORS.to_owned()),
            ("ORS_API_BASE", self.basis_ors()),
        ]
    }

    /// Ersetzt das Skript und leert die Aufzeichnung.
    pub(super) fn setze(&self, schritte: Vec<Schritt>) {
        let mut i = self.innen.lock().unwrap();
        i.skript = schritte;
        i.gesehen.clear();
    }

    /// Die Anfragen seit dem letzten Aufruf; leert die Aufzeichnung.
    pub(super) fn nimm(&self) -> Vec<Gesehen> {
        std::mem::take(&mut self.innen.lock().unwrap().gesehen)
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.halt.store(true, Ordering::SeqCst);
        // `accept` aufwecken.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(f) = self.faden.take() {
            let _ = f.join();
        }
    }
}

/// Was beim Stub ankam, noch nicht eingeordnet.
struct Eingang {
    methode: String,
    pfad: String,
    kopf: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn lies_anfrage(s: &mut TcpStream) -> Option<Eingang> {
    s.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    let mut roh = Vec::new();
    let mut block = [0u8; 8192];
    let kopf_ende = loop {
        if let Some(p) = roh.windows(4).position(|w| w == b"\r\n\r\n") {
            break p;
        }
        let n = s.read(&mut block).ok()?;
        if n == 0 {
            return None;
        }
        roh.extend_from_slice(&block[..n]);
    };
    let kopf_text = String::from_utf8_lossy(&roh[..kopf_ende]).into_owned();
    let mut zeilen = kopf_text.lines();
    let mut erste = zeilen.next()?.split_whitespace();
    let (methode, pfad) = (erste.next()?.to_owned(), erste.next()?.to_owned());
    let kopf: BTreeMap<String, String> = zeilen
        .filter_map(|z| z.split_once(':'))
        .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_owned()))
        .collect();
    let laenge: usize = kopf.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut body = roh[kopf_ende + 4..].to_vec();
    while body.len() < laenge {
        let n = s.read(&mut block).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&block[..n]);
    }
    Some(Eingang {
        methode,
        pfad,
        kopf,
        body,
    })
}

/// Der Schluessel der Anfrage im Skript.
fn schluessel_von(pfad: &str, body: &[u8]) -> String {
    match pfad.split('?').next().unwrap_or_default() {
        "/ors/geocode/search" => "ors:geocode".into(),
        "/ors/v2/directions/driving-car" => "ors:route".into(),
        "/llm/chat/completions" => {
            let j: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
            let name = j["response_format"]["json_schema"]["name"]
                .as_str()
                .unwrap_or_default();
            format!("chat:{name}")
        }
        _ => "unbekannt".into(),
    }
}

fn ohne_schluessel(text: &str) -> String {
    text.replace(SCHLUESSEL_ORS, "<KEY>")
        .replace(SCHLUESSEL_LLM, "<KEY>")
}

/// Die Regel-Kennungen im Systemprompt: Zeilen `- <kennung>:` oder `- <kennung>` (Pythons
/// `^- ([A-Za-z0-9_]+)(?::|$)`).
fn regeln_aus(body: &[u8]) -> Vec<String> {
    let j: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
    let system = j["messages"][0]["content"].as_str().unwrap_or_default();
    system
        .lines()
        .filter_map(|z| z.strip_prefix("- "))
        .filter_map(|r| {
            let id: String = r
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            let rest = &r[id.len()..];
            (!id.is_empty() && (rest.is_empty() || rest.starts_with(':'))).then_some(id)
        })
        .collect()
}

fn bediene(mut s: TcpStream, innen: &Arc<Mutex<Innen>>) {
    let Some(Eingang {
        methode,
        pfad,
        kopf,
        body,
    }) = lies_anfrage(&mut s)
    else {
        return;
    };
    let schluessel = schluessel_von(&pfad, &body);
    let (schritt, ungeplant) = {
        let mut i = innen.lock().unwrap();
        let treffer = i
            .skript
            .iter()
            .position(|st| st.fuer == "*" || st.fuer == schluessel);
        let schritt = treffer.map(|p| {
            if i.skript[p].oft {
                i.skript[p].clone()
            } else {
                i.skript.remove(p)
            }
        });
        let ungeplant = schritt.is_none();
        let kanonisch = if schluessel.starts_with("chat:") {
            serde_json::from_slice::<Value>(&body)
                .map_or_else(|_| String::from_utf8_lossy(&body).into_owned(), |j| j.to_string())
        } else {
            String::from_utf8_lossy(&body).into_owned()
        };
        i.gesehen.push(Gesehen {
            schluessel: schluessel.clone(),
            methode,
            pfad: ohne_schluessel(&pfad),
            kopf: kopf
                .iter()
                .filter(|(k, _)| matches!(k.as_str(), "content-type" | "authorization"))
                .map(|(k, v)| (k.clone(), ohne_schluessel(v)))
                .collect(),
            body: kanonisch,
            ungeplant,
        });
        (schritt, ungeplant)
    };
    let (status, text) = match schritt {
        Some(st) => {
            if st.warte_ms > 0 {
                std::thread::sleep(Duration::from_millis(st.warte_ms));
            }
            let text = match st.inhalt {
                Inhalt::Roh(t) => t,
                Inhalt::Regeln(n) => {
                    let regeln: Vec<String> = regeln_aus(&body).into_iter().take(n).collect();
                    let inhalt = json!({"zuordnungen": [{"aussage": 0, "regeln": regeln}]});
                    chat_huelle(&inhalt.to_string(), "stop")
                }
            };
            (st.status, text)
        }
        None => (599, json!({"stub": "ungeplant", "pfad": ohne_schluessel(&pfad)}).to_string()),
    };
    debug_assert_eq!(ungeplant, status == 599);
    let antwort = format!(
        "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",
        text.len()
    );
    let _ = s.write_all(antwort.as_bytes());
}

/// Was an zwei Aufzeichnungen verschieden ist (leer = gleich). Gleich heisst: dieselben Anfragen in
/// derselben Reihenfolge mit demselben Pfad, Kopf und Koerper.
pub(super) fn vergleiche_aufzeichnungen(py: &[Gesehen], rs: &[Gesehen]) -> Vec<String> {
    let mut d = Vec::new();
    if py.len() != rs.len() {
        let namen = |v: &[Gesehen]| -> Vec<String> {
            v.iter()
                .map(|g| format!("{} {}", g.methode, g.schluessel))
                .collect()
        };
        d.push(format!(
            "Zahl der Anfragen an den Dienst: py={:?} rs={:?}",
            namen(py),
            namen(rs)
        ));
    }
    for (i, (p, r)) in py.iter().zip(rs).enumerate() {
        for (feld, a, b) in [
            ("schluessel", &p.schluessel, &r.schluessel),
            ("methode", &p.methode, &r.methode),
            ("pfad", &p.pfad, &r.pfad),
            ("body", &p.body, &r.body),
        ] {
            if a != b {
                d.push(format!("Anfrage {i} {feld}: py={} rs={}", kurz(a), kurz(b)));
            }
        }
        if p.kopf != r.kopf {
            d.push(format!("Anfrage {i} Kopf: py={:?} rs={:?}", p.kopf, r.kopf));
        }
    }
    d
}

fn kurz(s: &str) -> String {
    let n = 160;
    if s.chars().count() > n {
        format!("{}...({} Zeichen)", s.chars().take(n).collect::<String>(), s.chars().count())
    } else {
        s.to_owned()
    }
}

// ---------------------------------------------------------------- Szenarien (Daten)

/// Ein Fall fuer `chat` oder `entfernung`: Anfrage, Skript des Dienstes und der gemessene Soll-Ausgang
/// von Python. `status` und `fehler_beginnt` sind die Werte, die Python am 2026-10-03 lieferte
/// (`berichte/haertung8.md`, Folge 6, Messlauf E/C); `anfragen` ist die Folge der Anfragen, die Python
/// an den Stub sendet.
pub(super) struct Szenario {
    pub name: &'static str,
    /// Die Scheibe des Falls, auf den die Anfrage geht (der Fall heisst `x_<scheibe>`).
    pub scheibe: &'static str,
    /// `chat` oder `entfernung`.
    pub route: &'static str,
    pub rumpf: Value,
    pub schritte: Vec<Schritt>,
    pub status: u16,
    /// Bei 500: der Anfang von `fehler` (Pythons Klasse und Meldung); bei 400/422 ebenso.
    pub fehler_beginnt: Option<&'static str>,
    pub anfragen: Vec<&'static str>,
    /// Ein Text, der nicht im Koerper an den Dienst stehen darf (PII-Filter).
    pub nicht_im_dienst: Option<&'static str>,
    /// `chat`: `abgelehnt_gruende[feld_id]`, Zeichen fuer Zeichen (Julius 6c: Klasse und Feld).
    pub grund: Option<(&'static str, String)>,
    /// `chat`: ein Wert, der nirgends in der Antwort stehen darf (Julius 6c: nie der Wert).
    pub nicht_in_antwort: Option<&'static str>,
    /// Bestaetigte Angaben (`feld_id`, Wert), die vor der Anfrage per `POST /event` in den Fall gehen:
    /// der Erklaer-Kontext, den `chat` an das Modell schickt.
    pub vorher: Vec<(&'static str, Value)>,
    /// Texte, die im Koerper einer Anfrage an den Dienst stehen muessen (Pythons Kontext, woertlich).
    pub muss_im_dienst: Vec<&'static str>,
}

const ADRESSEN: (&str, &str) = ("Musterstr. 1, 80331 München", "Beispielweg 2, 80333 München");
const TEXT: &str = "Ich habe 60000 Euro brutto verdient und bin ledig.";

fn adressen() -> Value {
    json!({"von": ADRESSEN.0, "nach": ADRESSEN.1})
}

fn geocode_ok() -> Schritt {
    Schritt::roh(
        "ors:geocode",
        200,
        r#"{"features": [{"geometry": {"coordinates": [11.5, 48.1]}}]}"#,
    )
}

/// Die zweite Adresse bekommt andere Koordinaten, damit ein vertauschtes Paar im Routing auffaellt.
fn geocode_nach() -> Schritt {
    Schritt::roh(
        "ors:geocode",
        200,
        r#"{"features": [{"geometry": {"coordinates": [11.6, 48.2]}}]}"#,
    )
}

fn route_mit(distanz: &str) -> Schritt {
    Schritt::roh(
        "ors:route",
        200,
        &format!(r#"{{"routes": [{{"summary": {{"distance": {distanz}}}}}]}}"#),
    )
}

fn entfernung(
    name: &'static str,
    schritte: Vec<Schritt>,
    status: u16,
    fehler: Option<&'static str>,
    anfragen: &[&'static str],
) -> Szenario {
    Szenario {
        name,
        scheibe: "ep",
        route: "entfernung",
        rumpf: adressen(),
        schritte,
        status,
        fehler_beginnt: fehler,
        anfragen: anfragen.to_vec(),
        nicht_im_dienst: None,
        grund: None,
        nicht_in_antwort: None,
        vorher: vec![],
        muss_im_dienst: vec![],
    }
}

const GGR: &[&str] = &["ors:geocode", "ors:geocode", "ors:route"];

/// Die Szenarien fuer `POST /fall/{id}/entfernung` MIT Schluessel. E1 (ohne Schluessel) und E14
/// (nicht erreichbar) gehen im bestehenden Lauf mit leeren Schluesseln; E13 (Zeitgrenze 8 s) gehoert
/// den Crate-Tests.
pub(super) fn entfernung_szenarien() -> Vec<Szenario> {
    let gut = |d: &str| vec![geocode_ok(), geocode_nach(), route_mit(d)];
    let g1 = |antwort: Schritt| vec![antwort];
    let mut v = vec![
        entfernung("E2 Strecke 30,4 km", gut("30400.0"), 200, None, GGR),
        entfernung("E2c 30,5 km rundet gerade (30)", gut("30500"), 200, None, GGR),
        entfernung("E2d 31,5 km rundet gerade (32)", gut("31500"), 200, None, GGR),
        entfernung(
            "E3 Geocoding antwortet 500",
            g1(Schritt::roh("ors:geocode", 500, "{}")),
            503,
            None,
            &["ors:geocode"],
        ),
        entfernung(
            "E4 Geocoding antwortet kein JSON",
            g1(Schritt::roh("ors:geocode", 200, "nicht json")),
            503,
            None,
            &["ors:geocode"],
        ),
        entfernung(
            "E5 Geocoding ohne Treffer",
            g1(Schritt::roh("ors:geocode", 200, r#"{"features": []}"#)),
            503,
            None,
            &["ors:geocode"],
        ),
        entfernung(
            "E6 Routing ohne Distanz",
            vec![geocode_ok(), geocode_ok(), Schritt::roh("ors:route", 200, r#"{"routes": [{"summary": {}}]}"#)],
            503,
            None,
            GGR,
        ),
        entfernung(
            "E7 Distanz ist Text",
            gut(r#""abc""#),
            500,
            Some("ValueError: could not convert string to float: 'abc'"),
            GGR,
        ),
        entfernung(
            "E7 Distanz ist null",
            gut("null"),
            500,
            Some("TypeError: float() argument must be a string or a real number, not 'NoneType'"),
            GGR,
        ),
        entfernung("E7 Distanz ist Zahl als Text", gut(r#""12000""#), 200, None, GGR),
        entfernung("E7 Distanz ist true", gut("true"), 200, None, GGR),
        entfernung("E7 Distanz ist negativ", gut("-5000"), 200, None, GGR),
        entfernung(
            "E7 Distanz 1e30 m (Magnitude)",
            gut("1e30"),
            422,
            Some("fail-closed (F2/Magnitude)"),
            GGR,
        ),
        entfernung(
            "E7 Distanz ist eine Liste",
            gut("[1]"),
            500,
            Some("TypeError: float() argument must be a string or a real number, not 'list'"),
            GGR,
        ),
        entfernung(
            "E8 features[0] ist Text",
            g1(Schritt::roh("ors:geocode", 200, r#"{"features": ["x"]}"#)),
            500,
            Some("AttributeError: 'str' object has no attribute 'get'"),
            &["ors:geocode"],
        ),
        entfernung(
            "E9 Antwort ist eine Liste",
            g1(Schritt::roh("ors:geocode", 200, "[1]")),
            500,
            Some("AttributeError: 'list' object has no attribute 'get'"),
            &["ors:geocode"],
        ),
        entfernung(
            "E9b Antwort ist eine Zahl",
            g1(Schritt::roh("ors:geocode", 200, "5")),
            500,
            Some("AttributeError: 'int' object has no attribute 'get'"),
            &["ors:geocode"],
        ),
        entfernung(
            "E9c features ist ein Objekt",
            g1(Schritt::roh("ors:geocode", 200, r#"{"features": {"a": 1}}"#)),
            500,
            Some("KeyError: 0"),
            &["ors:geocode"],
        ),
        entfernung(
            "E9d geometry ist Text",
            g1(Schritt::roh("ors:geocode", 200, r#"{"features": [{"geometry": "x"}]}"#)),
            500,
            Some("AttributeError: 'str' object has no attribute 'get'"),
            &["ors:geocode"],
        ),
        entfernung(
            "E9e nur eine Koordinate",
            g1(Schritt::roh("ors:geocode", 200, r#"{"features": [{"geometry": {"coordinates": [11.5]}}]}"#)),
            503,
            None,
            &["ors:geocode"],
        ),
        entfernung(
            "E9f routes ist Text",
            vec![geocode_ok(), geocode_ok(), Schritt::roh("ors:route", 200, r#"{"routes": "x"}"#)],
            503,
            None,
            GGR,
        ),
        entfernung(
            "E9g summary ist eine Liste",
            vec![geocode_ok(), geocode_ok(), Schritt::roh("ors:route", 200, r#"{"routes": [{"summary": []}]}"#)],
            503,
            None,
            GGR,
        ),
        entfernung(
            "E9h routes[0] ist eine Zahl",
            vec![geocode_ok(), geocode_ok(), Schritt::roh("ors:route", 200, r#"{"routes": [5]}"#)],
            500,
            Some("TypeError: argument of type 'int' is not a container or iterable"),
            GGR,
        ),
        entfernung(
            "E9i routes[0] ist der Text summary",
            vec![geocode_ok(), geocode_ok(), Schritt::roh("ors:route", 200, r#"{"routes": ["summary"]}"#)],
            500,
            Some("TypeError: string indices must be integers, not 'str'"),
            GGR,
        ),
        entfernung(
            "E9j summary ist eine Zahl",
            vec![geocode_ok(), geocode_ok(), Schritt::roh("ors:route", 200, r#"{"routes": [{"summary": 5}]}"#)],
            500,
            Some("TypeError: argument of type 'int' is not a container or iterable"),
            GGR,
        ),
        entfernung(
            "E9k Koordinaten in allen Zahlenformen",
            vec![
                Schritt::roh(
                    "ors:geocode",
                    200,
                    r#"{"features": [{"geometry": {"coordinates": [1e30, 5.0, 1E-7, 123456789, "ü", null, true, 0.1, -0.0, 1.5e300]}}]}"#,
                )
                .immer(),
                route_mit("30400.0"),
            ],
            200,
            None,
            GGR,
        ),
        entfernung("E7 Distanz inf (Text)", gut(r#""inf""#), 500, Some("OverflowError: cannot convert float infinity to integer"), GGR),
        entfernung("E7 Distanz nan (Text)", gut(r#""nan""#), 500, Some("ValueError: cannot convert float NaN to integer"), GGR),
        entfernung("E7 Distanz 12_000 mit Leerraum (Text)", gut(r#"" 12_000 ""#), 200, None, GGR),
        entfernung(
            "E7 Distanz 10^25 als Ganzzahl (ueber u64)",
            gut("10000000000000000000000000"),
            422,
            Some("fail-closed (F2/Magnitude): ep_entfernung_km=10000000000000000000000 von berechnet:maps"),
            GGR,
        ),
    ];
    v.push(Szenario {
        rumpf: json!({"von": "Straße 5 & 6/7, 'x' \"y\" +%", "nach": "Zürich ß"}),
        ..entfernung("E2e Sonderzeichen in den Adressen", gut("30400.0"), 200, None, GGR)
    });
    v.push(Szenario {
        scheibe: "an_gesamt",
        ..entfernung("E12b Scheibe an_gesamt hat das Feld", gut("30400.0"), 200, None, GGR)
    });
    // Eingaben, die den Dienst nie erreichen.
    for (name, rumpf, status, fehler) in [
        (
            "E10 von ist eine Zahl",
            json!({"von": 5, "nach": "x"}),
            500,
            Some("AttributeError: 'int' object has no attribute 'strip'"),
        ),
        (
            "E11 von ist leer",
            json!({"von": " ", "nach": "x"}),
            400,
            Some("von und nach (Adressen) sind Pflicht"),
        ),
    ] {
        v.push(Szenario {
            rumpf,
            ..entfernung(name, vec![], status, fehler, &[])
        });
    }
    v.push(Szenario {
        scheibe: "rentner_gesamt",
        ..entfernung(
            "E12 Scheibe ohne Arbeitsweg-Feld",
            vec![],
            400,
            Some("diese Scheibe hat kein Arbeitsweg-km-Feld"),
            &[],
        )
    });
    v
}

fn chat(
    name: &'static str,
    rumpf: Value,
    schritte: Vec<Schritt>,
    status: u16,
    fehler: Option<&'static str>,
    anfragen: &[&'static str],
) -> Szenario {
    Szenario {
        name,
        scheibe: "gesamt",
        route: "chat",
        rumpf,
        schritte,
        status,
        fehler_beginnt: fehler,
        anfragen: anfragen.to_vec(),
        nicht_im_dienst: None,
        grund: None,
        nicht_in_antwort: None,
        vorher: vec![],
        muss_im_dienst: vec![],
    }
}

/// Die drei Stufen eines gelungenen Gesprächs (Stufe 3 mit einem gueltigen, einem scheibenfremden und
/// einem Wert ausserhalb des Bereichs).
fn drei_stufen() -> Vec<Schritt> {
    vec![
        Schritt::chat(
            "chat:aussagen",
            &json!({"aussagen": [
                {"text": "Der Nutzer hat 60000 Euro brutto verdient", "beleg": "60000 Euro brutto verdient"},
                {"text": "Der Nutzer ist ledig", "beleg": "bin ledig"}]}),
            "stop",
        ),
        Schritt::regeln(3),
        Schritt::chat(
            "chat:dialog",
            &json!({"vorschlaege": [
                {"feld_id": "bruttoarbeitslohn", "wert": 6_000_000, "beleg": "60000 Euro brutto",
                 "begruendung": "b", "aussage": 0, "rechenweg": null},
                {"feld_id": "gibt_es_nicht", "wert": 1, "beleg": "bin ledig",
                 "begruendung": "b", "aussage": 1, "rechenweg": null},
                {"feld_id": "fam_anzahl_kinder", "wert": 99, "beleg": "bin ledig",
                 "begruendung": "b", "aussage": 1, "rechenweg": null}],
                "rueckfragen": [], "antwort": "Das habe ich verstanden.", "unsicher": false}),
            "stop",
        ),
    ]
}

const ASD: &[&str] = &["chat:aussagen", "chat:zuordnung", "chat:dialog"];

/// Die Szenarien fuer `POST /fall/{id}/chat` MIT Schluessel. C1/C3 (Schluessel oder Basis fehlen) gehen
/// im bestehenden Lauf; C13 (Dienst haengt) gehoert den Crate-Tests.
pub(super) fn chat_szenarien() -> Vec<Szenario> {
    let text = json!({"text": TEXT});
    let leer_huelle = || Schritt::chat("*", &json!({}), "stop").immer();
    let mut v = vec![
        chat("C2 leerer Text", json!({"text": "  "}), vec![], 200, None, &[]),
        chat("C4 drei Stufen", text.clone(), drei_stufen(), 200, None, ASD),
        chat(
            "C5 Dienst antwortet immer 500",
            text.clone(),
            vec![Schritt::roh("*", 500, r#"{"error": "x"}"#).immer()],
            501,
            None,
            &["chat:aussagen"; 3],
        ),
        chat(
            "C6 Dienst antwortet 403",
            text.clone(),
            vec![Schritt::roh("*", 403, r#"{"error": "Budget limit exceeded"}"#).immer()],
            501,
            None,
            &["chat:aussagen"],
        ),
        chat(
            "C7 Dienst antwortet kein JSON",
            text.clone(),
            vec![Schritt::roh("*", 200, "nicht json").immer()],
            501,
            None,
            &["chat:aussagen"],
        ),
        chat(
            "C8 Stufe 2 scheitert",
            text.clone(),
            {
                let mut s = drei_stufen();
                s[1] = Schritt::roh("chat:zuordnung", 403, r#"{"error": "x"}"#);
                s
            },
            200,
            None,
            &["chat:aussagen", "chat:zuordnung"],
        ),
        chat(
            "C9 Stufe 3 scheitert",
            text.clone(),
            {
                let mut s = drei_stufen();
                s[2] = Schritt::roh("chat:dialog", 403, r#"{"error": "x"}"#);
                s
            },
            200,
            None,
            ASD,
        ),
        chat(
            "C10 Stufe 1 abgeschnitten",
            text.clone(),
            vec![Schritt::chat("*", &json!({}), "length").immer()],
            501,
            None,
            &["chat:aussagen"; 2],
        ),
        chat(
            "C11 text ist eine Zahl",
            json!({"text": 5}),
            vec![],
            500,
            Some("AttributeError: 'int' object has no attribute 'strip'"),
            &[],
        ),
        chat("C12 Stufe 1 liefert {}", text.clone(), vec![leer_huelle()], 200, None, ASD),
    ];
    v.push(Szenario {
        nicht_im_dienst: Some("DE89370400440532013000"),
        ..chat(
            "C14 IBAN im Text",
            json!({"text": "Meine IBAN ist DE89370400440532013000, ich habe 60000 Euro brutto verdient."}),
            drei_stufen(),
            200,
            None,
            ASD,
        )
    });
    // C15: der Store weist den Vorschlag ab, der Wert steht im Text des Nutzers. Der Grund nennt
    // Klasse und Feld (Julius 2026-10-03, 6c), der Wert steht nirgends in der Antwort.
    v.extend([
        abweisung("C15a Typ", "agb_aufwendungen", &json!("GEHEIM-123"), "fail-closed (Typ)", "GEHEIM-123"),
        abweisung("C15b Format", "kind_idnr", &json!("GEHEIM-123"), "fail-closed (Format)", "GEHEIM-123"),
        abweisung(
            "C15c Magnitude",
            "agb_aufwendungen",
            &json!(98_765_432_109_u64),
            "fail-closed (F2/Magnitude)",
            "98765432109",
        ),
    ]);
    // C16/C17: Stufe 1 scheitert, aber der Dienst hat geantwortet und sich genannt: der Ausfall-Eintrag
    // im Audit und im Fluss nennt den Anbieter (`llm_client.letzte_meta()`).
    let mit_anbieter = |inhalt: &str, ende: &str| {
        Schritt::roh(
            "*",
            200,
            &format!(
                r#"{{"provider": "AnbieterX", "choices": [{{"message": {{"content": "{inhalt}"}}, "finish_reason": "{ende}"}}]}}"#
            ),
        )
        .immer()
    };
    v.push(chat(
        "C16 Stufe 1 leere Antwort mit Anbieter",
        text.clone(),
        vec![mit_anbieter("", "stop")],
        501,
        None,
        &["chat:aussagen"; 3],
    ));
    v.push(chat(
        "C17 Stufe 1 abgeschnitten mit Anbieter",
        text.clone(),
        vec![mit_anbieter("{", "length")],
        501,
        None,
        &["chat:aussagen"; 2],
    ));
    // C20: nach C17 (Anbieter genannt) scheitert der Dienst ohne Antwort: der Anbieter ist leer, nicht der
    // von C17 (`_merke("", "")` zu Beginn jedes Aufrufs).
    v.push(chat(
        "C20 Stufe 1 HTTP 500 nach einem Anbieter",
        text.clone(),
        vec![Schritt::roh("*", 500, r#"{"error": "x"}"#).immer()],
        501,
        None,
        &["chat:aussagen"; 3],
    ));
    // C19: Stufe 2 waehlt die Regel mit dem Zaehlfeld `fam_anzahl_kinder` (und eine zweite, in
    // unsortierter Reihenfolge); die Instanz-Felder des Kindes (`kind_idnr`) stehen in anderen Regeln
    // und kommen nur ueber die Instanz-Gruppe in Stufe 3.
    v.push(Szenario {
        muss_im_dienst: vec!["kind_idnr"],
        ..chat(
            "C19 Zaehlfeld der Instanz-Gruppe",
            text.clone(),
            {
                let mut s = drei_stufen();
                s[1] = Schritt::chat(
                    "chat:zuordnung",
                    &json!({"zuordnungen": [
                        {"aussage": 0, "regeln": ["p24b_entlastungsbetrag", "p09_entfernungspauschale"]}]}),
                    "stop",
                );
                s
            },
            200,
            None,
            ASD,
        )
    });
    // C18: der Erklaer-Kontext. Bestaetigte Angaben und das offene Feld gehen in den Prompt von Stufe 3;
    // der Wert besonderer Kategorien (`agb_aufwendungen`) bleibt draussen. Zuletzt, weil die Angaben im
    // Fall bleiben (C15 braucht `agb_aufwendungen` ohne aktives Event).
    v.push(Szenario {
        vorher: vec![
            ("vv_einnahmen", json!(6_200_000)),
            ("kein_vuv", json!(true)),
            ("veranlagung", json!("zusammen")),
            ("ep_arbeitstage", json!(220)),
            ("kap_antrag_guenstigerpruefung", json!(true)),
            ("agb_aufwendungen", json!(500_000)),
        ],
        muss_im_dienst: vec![
            "Die Frage, um die es geht",
            "62000,00 EUR",
            "220 Tage",
            "- None \u{2192} ja",
            "weitere Angaben liegen vor",
        ],
        ..chat(
            "C18 Erklaer-Kontext",
            json!({"text": TEXT, "feld_id": "ep_arbeitstage"}),
            drei_stufen(),
            200,
            None,
            ASD,
        )
    });
    v
}

/// Stufe 3 mit EINEM Vorschlag, den der Store abweist. Der Beleg steht im Text; die Aussage nennt den
/// Wert nicht, damit ein Treffer in der Antwort nur aus dem Ablehnungsgrund stammen kann.
fn abweisung(
    name: &'static str,
    feld_id: &'static str,
    wert: &Value,
    klasse: &'static str,
    geheim: &'static str,
) -> Szenario {
    let rumpf = json!({"text": "Meine Krankheitskosten und GEHEIM-123 stehen auf dem Bescheid."});
    let schritte = vec![
        Schritt::chat(
            "chat:aussagen",
            &json!({"aussagen": [{"text": "Der Nutzer nennt eine Angabe", "beleg": "Krankheitskosten"}]}),
            "stop",
        ),
        Schritt::regeln(3),
        Schritt::chat(
            "chat:dialog",
            &json!({"vorschlaege": [{"feld_id": feld_id, "wert": wert, "beleg": "Krankheitskosten",
                                     "begruendung": "b", "aussage": 0, "rechenweg": null}],
                    "rueckfragen": [], "antwort": "", "unsicher": false}),
            "stop",
        ),
    ];
    Szenario {
        grund: Some((feld_id, format!("{klasse}: {feld_id}"))),
        nicht_in_antwort: Some(geheim),
        ..chat(name, rumpf, schritte, 200, None, ASD)
    }
}

// ---------------------------------------------------------------- Python gegen den Stub

use super::{
    json_body, schreibe_seed, sende, skip, starte_mit, Anfrage, Server,
};

/// Die Fall-Datei einer Scheibe: `x_<scheibe>`.
fn lege_faelle_an(port: u16, szenarien: &[&Szenario]) {
    let mut scheiben: Vec<&str> = szenarien.iter().map(|s| s.scheibe).collect();
    scheiben.sort_unstable();
    scheiben.dedup();
    for s in scheiben {
        let a = Anfrage::neu("anlegen", "POST", "/fall").json(
            &json!({"fall_id": format!("x_{s}"), "scheibe": s, "veranlagungszeitraum": 2025}),
        );
        let r = sende(port, &a);
        assert_eq!(r.status, 201, "Fall x_{s}: {}", String::from_utf8_lossy(&r.body));
    }
}

fn alle_szenarien() -> Vec<Szenario> {
    let mut v = entfernung_szenarien();
    v.extend(chat_szenarien());
    v
}

/// Das Ergebnis eines Szenarios an EINEM Server: Status, Body, Aufzeichnung des Stubs.
pub(super) fn lauf_einzeln(
    server: &Server,
    stub: &Stub,
    s: &Szenario,
) -> (u16, Option<Value>, Vec<Gesehen>) {
    for (feld, wert) in &s.vorher {
        let a = Anfrage::neu(
            &format!("{} {}: vorher {feld}", s.route, s.name),
            "POST",
            &format!("/fall/x_{}/event", s.scheibe),
        )
        .json(&super::ereignis(feld, wert, None));
        let r = sende(server.port, &a);
        assert_eq!(r.status, 201, "{} vorher {feld}: {}", s.name, String::from_utf8_lossy(&r.body));
    }
    stub.setze(s.schritte.clone());
    let a = Anfrage::neu(
        &format!("{} {}", s.route, s.name),
        "POST",
        &format!("/fall/x_{}/{}", s.scheibe, s.route),
    )
    .json(&s.rumpf);
    let r = sende(server.port, &a);
    (r.status, json_body(&r), stub.nimm())
}

/// Prueft ein Ergebnis gegen die Soll-Werte des Szenarios (Python-Messung). Leer = in Ordnung.
pub(super) fn pruefe_soll(s: &Szenario, status: u16, body: Option<&Value>, gesehen: &[Gesehen]) -> Vec<String> {
    let mut d = Vec::new();
    if status != s.status {
        d.push(format!("Status {status}, erwartet {}", s.status));
    }
    if let Some(soll) = s.fehler_beginnt {
        let ist = body.and_then(|b| b["fehler"].as_str()).unwrap_or_default();
        if !ist.starts_with(soll) {
            d.push(format!("fehler {ist:?} beginnt nicht mit {soll:?}"));
        }
    }
    let namen: Vec<&str> = gesehen.iter().map(|g| g.schluessel.as_str()).collect();
    if namen != s.anfragen {
        d.push(format!("Anfragen an den Dienst {namen:?}, erwartet {:?}", s.anfragen));
    }
    if gesehen.iter().any(|g| g.ungeplant) {
        d.push("der Dienst bekam eine Anfrage, die das Skript nicht vorsah".into());
    }
    if let Some(verboten) = s.nicht_im_dienst {
        if gesehen.iter().any(|g| g.body.contains(verboten)) {
            d.push(format!("{verboten:?} steht im Koerper an den Dienst"));
        }
    }
    if let Some((feld_id, soll)) = &s.grund {
        let ist = body
            .and_then(|b| b["abgelehnt_gruende"][feld_id].as_str())
            .unwrap_or_default();
        if ist != soll {
            d.push(format!("abgelehnt_gruende[{feld_id}] {ist:?}, erwartet {soll:?}"));
        }
    }
    if let Some(verboten) = s.nicht_in_antwort {
        if body.is_some_and(|b| b.to_string().contains(verboten)) {
            d.push(format!("{verboten:?} steht in der Antwort"));
        }
    }
    for noetig in &s.muss_im_dienst {
        if !gesehen.iter().any(|g| g.body.contains(noetig)) {
            d.push(format!("{noetig:?} steht in keinem Koerper an den Dienst"));
        }
    }
    d
}

/// Pythons Anfragen an den Dienst, woertlich, fuer zwei Faelle. Sie belegen, dass der Schluessel im
/// echten Verkehr dort steht, wo die Aufzeichnung ihn ersetzt (Query beim Geocoding, `Authorization`
/// beim Routing und beim LLM), und dass die Aufzeichnung ihn tatsaechlich entfernt hat.
pub(super) fn pruefe_woertlich(name: &str, g: &[Gesehen]) -> Vec<String> {
    let soll: Vec<(usize, &str, &str)> = match name {
        n if n.starts_with("E2 ") => vec![
            (0, "pfad", "/ors/geocode/search?api_key=<KEY>&text=Musterstr.+1%2C+80331+M%C3%BCnchen&size=1&boundary.country=DE"),
            (2, "pfad", "/ors/v2/directions/driving-car"),
            (2, "authorization", "<KEY>"),
            (2, "content-type", "application/json"),
            (2, "body", r#"{"coordinates": [[11.5, 48.1], [11.6, 48.2]], "preference": "shortest", "units": "m"}"#),
        ],
        n if n.starts_with("C4 ") => vec![
            (0, "pfad", "/llm/chat/completions"),
            (0, "authorization", "Bearer <KEY>"),
            (0, "content-type", "application/json"),
        ],
        _ => return vec![],
    };
    let mut d = Vec::new();
    for (i, feld, erwartet) in soll {
        let ist = g.get(i).map(|x| match feld {
            "pfad" => x.pfad.clone(),
            "body" => x.body.clone(),
            k => x.kopf.get(k).cloned().unwrap_or_default(),
        });
        if ist.as_deref() != Some(erwartet) {
            d.push(format!("Anfrage {i} {feld}: {ist:?}, erwartet {erwartet:?}"));
        }
    }
    d
}

/// Der Stub selbst: antwortet nach Skript, zeichnet ohne Schluessel auf, meldet Ungeplantes.
#[test]
fn stub_antwortet_nach_skript_und_zeichnet_ohne_schluessel_auf() {
    let stub = Stub::starte();
    stub.setze(vec![
        Schritt::roh("ors:geocode", 200, r#"{"features": []}"#),
        Schritt::roh("ors:route", 500, "{}"),
    ]);
    let hole = |methode: &str, pfad: &str, kopf: &str, body: &str| -> (u16, String) {
        let mut s = TcpStream::connect(("127.0.0.1", stub.port)).unwrap();
        let m = format!(
            "{methode} {pfad} HTTP/1.1\r\nHost: x\r\n{kopf}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        s.write_all(m.as_bytes()).unwrap();
        let mut roh = String::new();
        s.read_to_string(&mut roh).unwrap();
        let status = roh.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, roh.split("\r\n\r\n").nth(1).unwrap().to_owned())
    };
    let q = format!("/ors/geocode/search?api_key={SCHLUESSEL_ORS}&text=x");
    assert_eq!(hole("GET", &q, "", ""), (200, r#"{"features": []}"#.to_owned()));
    let kopf = format!("Authorization: {SCHLUESSEL_ORS}\r\nContent-Type: application/json\r\n");
    assert_eq!(hole("POST", "/ors/v2/directions/driving-car", &kopf, "{}").0, 500);
    // Zweimal dasselbe Geocoding: das Skript hatte nur einen Schritt.
    assert_eq!(hole("GET", &q, "", "").0, 599);

    let g = stub.nimm();
    assert_eq!(
        g.iter().map(|x| (x.schluessel.as_str(), x.ungeplant)).collect::<Vec<_>>(),
        [("ors:geocode", false), ("ors:route", false), ("ors:geocode", true)]
    );
    assert_eq!(g[0].pfad, "/ors/geocode/search?api_key=<KEY>&text=x");
    assert_eq!(g[1].kopf["authorization"], "<KEY>");
    let alles = format!("{g:?}");
    assert!(!alles.contains(SCHLUESSEL_ORS) && !alles.contains(SCHLUESSEL_LLM), "{alles}");
    assert!(stub.nimm().is_empty(), "nimm leert die Aufzeichnung");
}

/// Der Vergleich zweier Aufzeichnungen sieht Zahl, Pfad, Kopf und Koerper — und sonst nichts.
#[test]
fn aufzeichnungen_vergleichen_zahl_pfad_kopf_und_body() {
    let g = |pfad: &str, body: &str| Gesehen {
        schluessel: "ors:geocode".into(),
        methode: "GET".into(),
        pfad: pfad.into(),
        kopf: BTreeMap::from([("authorization".to_owned(), "<KEY>".to_owned())]),
        body: body.into(),
        ungeplant: false,
    };
    let a = [g("/ors/geocode/search?text=x", "")];
    assert!(vergleiche_aufzeichnungen(&a, &a.clone()).is_empty());
    assert_eq!(vergleiche_aufzeichnungen(&a, &[]).len(), 1, "Zahl");
    let pfad = [g("/ors/geocode/search?text=y", "")];
    assert!(vergleiche_aufzeichnungen(&a, &pfad)[0].contains("pfad"));
    let body = [g("/ors/geocode/search?text=x", "{}")];
    assert!(vergleiche_aufzeichnungen(&a, &body)[0].contains("body"));
    let mut kopf = a.clone();
    kopf[0].kopf.insert("content-type".into(), "x".into());
    assert!(vergleiche_aufzeichnungen(&a, &kopf)[0].contains("Kopf"));
}

/// Die Regel-Kennungen kommen aus dem Systemprompt der Anfrage (Stufe `zuordnung`).
#[test]
fn stub_liest_die_regeln_aus_dem_systemprompt() {
    let body = json!({"messages": [{"role": "system", "content":
        "Regeln:\n- p09_entfernungspauschale: Pendeln\n- p10_x\n-kaputt\n- \nNoch: - kein_anfang"}]});
    assert_eq!(
        regeln_aus(body.to_string().as_bytes()),
        ["p09_entfernungspauschale", "p10_x"]
    );
}

/// Python gegen den Stub: jedes Szenario liefert Pythons gemessenen Ausgang UND die erwarteten
/// Anfragen an den Dienst. Das belegt die Tabelle und die Verdrahtung (`LLM_API_BASE`,
/// `ORS_API_BASE`); im Paarlauf (Block 4) vergleicht der Harness dieselben Szenarien mit Rust.
#[test]
fn python_gegen_den_stub_liefert_die_gemessenen_ausgaenge() {
    if skip() {
        return;
    }
    let wurzel = tempfile::tempdir().unwrap();
    let seed = wurzel.path().join("seed");
    schreibe_seed(&seed);
    let stub = Stub::starte();
    let umgebung = stub.umgebung();
    let extra: Vec<(&str, &str)> = umgebung.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let server = starte_mit("python", wurzel.path(), true, false, &seed, &extra);
    let szenarien = alle_szenarien();
    lege_faelle_an(server.port, &szenarien.iter().collect::<Vec<_>>());
    let mut abweichungen = Vec::new();
    for s in &szenarien {
        let (status, body, gesehen) = lauf_einzeln(&server, &stub, s);
        for d in pruefe_soll(s, status, body.as_ref(), &gesehen)
            .into_iter()
            .chain(pruefe_woertlich(s.name, &gesehen))
        {
            abweichungen.push(format!("{}: {d}", s.name));
        }
    }
    println!("EXTERN python gegen Stub: {} Szenarien, {} Abweichungen", szenarien.len(), abweichungen.len());
    assert!(abweichungen.is_empty(), "{abweichungen:#?}");
}

// ---------------------------------------------------------------- Python gegen Rust, je ein Stub

use super::{Modus, Paar};

/// Python und Rust nebeneinander, je ein eigener Stub mit demselben Skript: dieselbe Anfrage muss
/// dieselbe Antwort, dieselben Protokolle (`audit`, `fehler.log`, `flow.jsonl`), denselben Stand der
/// Fallakten UND dieselben Anfragen an den Dienst ergeben (ohne Schluessel, s. [`Gesehen`]).
/// Zusaetzlich gelten die Soll-Werte der Szenarien (an Python gemessen) fuer die Antwort von Rust.
fn paarlauf(route: &str, titel: &str) {
    if skip() {
        return;
    }
    let wurzel = tempfile::tempdir().unwrap();
    let seed = wurzel.path().join("seed");
    schreibe_seed(&seed);
    let (stub_py, stub_rs) = (Stub::starte(), Stub::starte());
    let (env_py, env_rs) = (stub_py.umgebung(), stub_rs.umgebung());
    let extra_py: Vec<(&str, &str)> = env_py.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let extra_rs: Vec<(&str, &str)> = env_rs.iter().map(|(k, v)| (*k, v.as_str())).collect();
    let mut p = Paar::neu_mit(wurzel.path(), true, true, &seed, &extra_py, &extra_rs);
    let szenarien: Vec<Szenario> = alle_szenarien()
        .into_iter()
        .filter(|s| s.route == route)
        .collect();
    let mut scheiben: Vec<&str> = szenarien.iter().map(|s| s.scheibe).collect();
    scheiben.sort_unstable();
    scheiben.dedup();
    for s in scheiben {
        let a = Anfrage::neu("anlegen", "POST", "/fall").json(
            &json!({"fall_id": format!("x_{s}"), "scheibe": s, "veranlagungszeitraum": 2025}),
        );
        p.anfrage(&a, Modus::Voll);
    }
    for s in &szenarien {
        for (feld, wert) in &s.vorher {
            let a = Anfrage::neu(
                &format!("{} {}: vorher {feld}", s.route, s.name),
                "POST",
                &format!("/fall/x_{}/event", s.scheibe),
            )
            .json(&super::ereignis(feld, wert, None));
            p.anfrage(&a, Modus::Voll);
            if p.stat.letzter != 201 {
                p.stat.abweichungen.push(format!("{} vorher {feld}: Status {}", s.name, p.stat.letzter));
            }
        }
        stub_py.setze(s.schritte.clone());
        stub_rs.setze(s.schritte.clone());
        let a = Anfrage::neu(
            &format!("{} {}", s.route, s.name),
            "POST",
            &format!("/fall/x_{}/{}", s.scheibe, s.route),
        )
        .json(&s.rumpf);
        let body_py = p.anfrage(&a, Modus::Voll);
        let status_rs = p.stat.letzter;
        let (gesehen_py, gesehen_rs) = (stub_py.nimm(), stub_rs.nimm());
        let mut d: Vec<String> = vergleiche_aufzeichnungen(&gesehen_py, &gesehen_rs);
        d.extend(
            pruefe_soll(s, status_rs, body_py.as_ref(), &gesehen_rs)
                .into_iter()
                .chain(pruefe_woertlich(s.name, &gesehen_rs)),
        );
        p.stat
            .abweichungen
            .extend(d.into_iter().map(|x| format!("{}: {x}", s.name)));
    }
    p.zustand_vergleichen("nach den Szenarien");
    p.bericht(titel);
    println!("EXTERN {titel}: {} Szenarien, je eigener Stub", szenarien.len());
    assert!(
        p.stat.abweichungen.is_empty(),
        "{:#?}",
        p.stat.abweichungen
    );
}

/// `POST /fall/{id}/entfernung`: Rust gleich Python gegen je einen Stub des Karten-Dienstes.
#[test]
fn extern_paritaet_entfernung() {
    paarlauf("entfernung", "extern/entfernung");
}

/// `POST /fall/{id}/chat`: Rust gleich Python gegen je einen Stub des Sprachmodells.
#[test]
fn extern_paritaet_chat() {
    paarlauf("chat", "extern/chat");
}
