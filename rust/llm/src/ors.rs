//! OpenRouteService-Transport (`produkt/haut/ors_client.py`): Schluessel und Basis aus der Umgebung,
//! zwei GET-Geocodes und ein POST-Routing, jede Antwort als JSON-Wert.
//!
//! Hier steht nur, WAS gesendet und WIE die Antwort gelesen wird. Wie Python die Antwort auswertet
//! (`features[0]`, `routes[0]["summary"]["distance"]`, Pythons Ausnahmen bei falscher Gestalt), baut
//! `api::entfernung` nach: dieser Teil entscheidet ueber 200, 500 und 503, nicht der Transport.
//!
//! Die Basis-URL kommt aus `$ORS_API_BASE` (Standard [`STANDARD_BASIS`]); gesetzt wird sie nur im
//! Vergleichslauf gegen einen lokalen Stub. Der Schluessel steht in keiner Fehlermeldung, in keinem
//! `Debug` und nie in einer URL, die ein Fehler traegt (`tests/test_ors_key_leckt_nicht.py`).
//!
//! # Abweichungen von `urllib`
//! `ponytail`: kein Folgen von Weiterleitungen (3xx zaehlt wie jeder Status ausserhalb 2xx als
//! nicht verfuegbar), keine UTF-16/32- und keine BOM-Dekodierung des Antwortkoerpers
//! (`json.loads(bytes)` kennt sie), keine Antwort mit `NaN`/`Infinity`-Literal (`json.loads` liest
//! sie, `serde_json` nicht). In allen drei Faellen antwortet Rust mit 503 und Python (wo es die
//! Antwort liest) nicht.
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::client::Schluessel;
use crate::http::{senden, Transport};

/// `_BASE` (`ors_client.py:15`).
pub const STANDARD_BASIS: &str = "https://api.openrouteservice.org";

/// `_TIMEOUT` (`ors_client.py:16`): je Verbindungsaufbau und Leseoperation, ohne Wanduhr-Frist.
const TIMEOUT: Duration = Duration::from_secs(8);

/// Pythons `OrsNichtVerfuegbar`: kein Schluessel oder der Dienst antwortete nicht verwertbar. Der
/// Aufrufer faellt auf die manuelle Eingabe zurueck (503). Der Text nennt weder Schluessel noch URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NichtVerfuegbar(pub String);

impl std::fmt::Display for NichtVerfuegbar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for NichtVerfuegbar {}

/// Basis und Schluessel des Dienstes.
#[derive(Debug, Clone)]
pub struct Ors {
    basis: String,
    schluessel: Schluessel,
}

impl Ors {
    /// `_key()` und `_BASE`: `$ORS_API_KEY` (mit `strip()`), `$ORS_API_BASE` (leer = Standard).
    ///
    /// # Errors
    /// [`NichtVerfuegbar`], wenn `$ORS_API_KEY` fehlt oder leer ist.
    pub fn aus_env() -> Result<Self, NichtVerfuegbar> {
        let env = |k: &str| std::env::var(k).unwrap_or_default();
        Self::aus_werten(&env("ORS_API_KEY"), &env("ORS_API_BASE"))
    }

    /// Wie [`Ors::aus_env`], mit den Werten statt der Umgebung.
    ///
    /// ```
    /// let o = llm::ors::Ors::aus_werten("  geheim\n", "").unwrap();
    /// assert!(!format!("{o:?}").contains("geheim"));
    /// assert!(llm::ors::Ors::aus_werten(" ", "http://127.0.0.1:1").is_err());
    /// ```
    ///
    /// # Errors
    /// [`NichtVerfuegbar`], wenn der Schluessel leer ist.
    pub fn aus_werten(schluessel: &str, basis: &str) -> Result<Self, NichtVerfuegbar> {
        let schluessel = crate::py::strip(schluessel);
        if schluessel.is_empty() {
            return Err(NichtVerfuegbar(
                "kein ORS_API_KEY in der Umgebung (.env.maps nicht geladen?)".into(),
            ));
        }
        let basis = crate::py::strip(basis).trim_end_matches('/');
        Ok(Self {
            basis: if basis.is_empty() {
                STANDARD_BASIS.to_owned()
            } else {
                basis.to_owned()
            },
            schluessel: Schluessel::neu(schluessel.to_owned()),
        })
    }

    /// `geocode`, erster Teil: `GET /geocode/search` mit dem Schluessel im Query (`ors_client.py:61-62`).
    ///
    /// # Errors
    /// [`NichtVerfuegbar`] bei Netz-, HTTP- und JSON-Fehlern.
    pub fn geocode(&self, adresse: &str) -> Result<Value, NichtVerfuegbar> {
        let q = urlencode(&[
            ("api_key", self.schluessel.als_str()),
            ("text", adresse),
            ("size", "1"),
            ("boundary.country", "DE"),
        ]);
        Self::hole(
            "GET",
            &format!("{}/geocode/search?{q}", self.basis),
            &[],
            None,
        )
    }

    /// `_distanz_meter`, erster Teil: `POST /v2/directions/driving-car` (`ors_client.py:73-76`).
    /// `von`/`nach` sind die Koordinatenlisten, wie `geocode` sie lieferte.
    ///
    /// # Errors
    /// [`NichtVerfuegbar`] bei Netz-, HTTP- und JSON-Fehlern.
    pub fn route(&self, von: &Value, nach: &Value) -> Result<Value, NichtVerfuegbar> {
        let mut body = String::from("{\"coordinates\": [");
        dumps(von, &mut body);
        body.push_str(", ");
        dumps(nach, &mut body);
        body.push_str("], \"preference\": \"shortest\", \"units\": \"m\"}");
        Self::hole(
            "POST",
            &format!("{}/v2/directions/driving-car", self.basis),
            &[
                ("Authorization", self.schluessel.als_str()),
                ("Content-Type", "application/json"),
            ],
            Some(body.as_bytes()),
        )
    }

    /// `_hole`: jeder Fehler — Netz, HTTP, JSON — wird [`NichtVerfuegbar`], ohne URL und Schluessel.
    fn hole(
        methode: &str,
        url: &str,
        kopf: &[(&str, &str)],
        koerper: Option<&[u8]>,
    ) -> Result<Value, NichtVerfuegbar> {
        // Python kennt hier keine Wanduhr-Frist, nur den Socket-Timeout; ein Tag ist "nie".
        let ende = Instant::now() + Duration::from_hours(24);
        let antwort = senden(methode, url, kopf, koerper, TIMEOUT, ende)
            .map_err(|t| nicht_verfuegbar(typ_von(&t)))?;
        if !(200..300).contains(&antwort.status) {
            return Err(nicht_verfuegbar("HTTPError"));
        }
        serde_json::from_slice(&antwort.koerper).map_err(|_| nicht_verfuegbar("JSONDecodeError"))
    }
}

fn nicht_verfuegbar(typ: &str) -> NichtVerfuegbar {
    NichtVerfuegbar(format!("ORS-Aufruf fehlgeschlagen: {typ}"))
}

fn typ_von(t: &Transport) -> &'static str {
    match t {
        Transport::Netz(_) => "URLError",
        Transport::Zeit | Transport::Frist => "TimeoutError",
        Transport::Kaputt(_) => "Exception",
    }
}

/// `urllib.parse.urlencode`: `quote_plus`, UTF-8; unveraendert bleiben `A-Za-z0-9_.-~`.
fn urlencode(paare: &[(&str, &str)]) -> String {
    let mut out = String::new();
    for (i, (k, v)) in paare.iter().enumerate() {
        if i > 0 {
            out.push('&');
        }
        quote_plus(k, &mut out);
        out.push('=');
        quote_plus(v, &mut out);
    }
    out
}

fn quote_plus(s: &str, out: &mut String) {
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => {
                out.push(char::from(b));
            }
            b' ' => out.push('+'),
            _ => {
                let _ = write!(out, "%{b:02X}");
            }
        }
    }
}

/// `json.dumps(wert)` mit Pythons Voreinstellungen: Trenner `", "` und `": "`, `ensure_ascii`
/// (alles ausserhalb von ASCII als `\uXXXX`, ueber `U+FFFF` als Ersatzpaar), Kommazahlen wie
/// `float.__repr__`. `ponytail`: Objektschluessel stehen sortiert, Python haelt die Einfuegereihenfolge;
/// in Koordinatenlisten kommen keine Objekte vor.
fn dumps(wert: &Value, out: &mut String) {
    match wert {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => match (n.as_i64(), n.as_u64(), n.as_f64()) {
            (Some(i), _, _) => {
                let _ = write!(out, "{i}");
            }
            (_, Some(u), _) => {
                let _ = write!(out, "{u}");
            }
            (_, _, Some(f)) => out.push_str(&domain::repr_float(f)),
            _ => out.push_str("null"),
        },
        Value::String(s) => dumps_text(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, e) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                dumps(e, out);
            }
            out.push(']');
        }
        Value::Object(o) => {
            out.push('{');
            for (i, (k, e)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                dumps_text(k, out);
                out.push_str(": ");
                dumps(e, out);
            }
            out.push('}');
        }
    }
}

fn dumps_text(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            _ => {
                let mut einheiten = [0u16; 2];
                for e in c.encode_utf16(&mut einheiten) {
                    let _ = write!(out, "\\u{e:04x}");
                }
            }
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    use serde_json::json;

    use super::*;

    const SCHLUESSEL: &str = "SYNTHETISCH-ORS-SCHLUESSEL";

    /// Ein lokaler Dienst: antwortet der Reihe nach mit `antworten` (roh) und gibt jede empfangene
    /// Anfrage (Kopf + Koerper als Text) zurueck.
    fn dienst(antworten: Vec<String>) -> (String, std::thread::JoinHandle<Vec<String>>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let basis = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
        let h = std::thread::spawn(move || {
            let mut gesehen = Vec::new();
            for a in antworten {
                let (mut s, _) = l.accept().unwrap();
                let mut puffer = Vec::new();
                let mut b = [0u8; 4096];
                loop {
                    let n = s.read(&mut b).unwrap();
                    puffer.extend_from_slice(&b[..n]);
                    let text = String::from_utf8_lossy(&puffer).into_owned();
                    if let Some(i) = text.find("\r\n\r\n") {
                        let laenge = text[..i]
                            .lines()
                            .find_map(|z| z.strip_prefix("Content-Length: "))
                            .map_or(0, |v| v.trim().parse::<usize>().unwrap());
                        if puffer.len() >= i + 4 + laenge {
                            break;
                        }
                    }
                    assert!(n > 0, "Verbindung zu frueh geschlossen");
                }
                gesehen.push(String::from_utf8_lossy(&puffer).into_owned());
                s.write_all(a.as_bytes()).unwrap();
            }
            gesehen
        });
        (basis, h)
    }

    fn ok(json: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{json}",
            json.len()
        )
    }

    #[test]
    fn geocode_sendet_get_mit_schluessel_im_query() {
        let (basis, h) = dienst(vec![ok(r#"{"features": []}"#)]);
        let ors = Ors::aus_werten(SCHLUESSEL, &format!("{basis}/")).unwrap();
        let antwort = ors.geocode("Musterstr. 1, 80331 München").unwrap();
        assert_eq!(antwort, json!({"features": []}));
        let anfrage = &h.join().unwrap()[0];
        let zeile = anfrage.lines().next().unwrap();
        // Dieselbe Zeile erzeugt `urllib.parse.urlencode` (Pythons Lauf E2 in berichte/haertung8.md).
        assert_eq!(
            zeile,
            format!("GET /geocode/search?api_key={SCHLUESSEL}&text=Musterstr.+1%2C+80331+M%C3%BCnchen&size=1&boundary.country=DE HTTP/1.1")
        );
        assert!(!anfrage.contains("Authorization"), "{anfrage}");
        assert!(!anfrage.contains("Content-Length"), "{anfrage}");
    }

    #[test]
    fn route_sendet_post_mit_rohem_schluessel_im_header_und_dem_python_body() {
        let (basis, h) = dienst(vec![ok(r#"{"routes": []}"#)]);
        let ors = Ors::aus_werten(SCHLUESSEL, &basis).unwrap();
        let antwort = ors.route(&json!([11.5, 48.1]), &json!([8, 9.0])).unwrap();
        assert_eq!(antwort, json!({"routes": []}));
        let anfrage = &h.join().unwrap()[0];
        assert!(
            anfrage.starts_with("POST /v2/directions/driving-car HTTP/1.1\r\n"),
            "{anfrage}"
        );
        assert!(
            anfrage.contains(&format!("\r\nAuthorization: {SCHLUESSEL}\r\n")),
            "{anfrage}"
        );
        assert!(
            anfrage.contains("\r\nContent-Type: application/json\r\n"),
            "{anfrage}"
        );
        // `json.dumps({"coordinates": [[11.5, 48.1], [8, 9.0]], "preference": "shortest", "units": "m"})`
        assert!(
            anfrage.ends_with(r#"{"coordinates": [[11.5, 48.1], [8, 9.0]], "preference": "shortest", "units": "m"}"#),
            "{anfrage}"
        );
    }

    #[test]
    fn dumps_wie_json_dumps() {
        let mut s = String::new();
        dumps(
            &json!([
                "ä\n\"\\\u{1}\u{7f}😀",
                1e22,
                1e-7,
                -0.0,
                123_456_789_012_345_678_i64,
                true,
                null
            ]),
            &mut s,
        );
        // python3 -c 'import json; print(json.dumps(["ä\n\"\\\u0001\u007f😀", 1e22, 1e-7, -0.0, 123456789012345678, True, None]))'
        assert_eq!(
            s,
            r#"["\u00e4\n\"\\\u0001\u007f\ud83d\ude00", 1e+22, 1e-07, -0.0, 123456789012345678, true, null]"#
        );
    }

    #[test]
    fn urlencode_wie_python() {
        // python3 -c 'from urllib.parse import urlencode; print(urlencode({"api_key": "k e/y+ä", "text": "Musterstr. 1, 80331 München ~_-.", "size": 1, "boundary.country": "DE"}))'
        assert_eq!(
            urlencode(&[
                ("api_key", "k e/y+ä"),
                ("text", "Musterstr. 1, 80331 München ~_-."),
                ("size", "1"),
                ("boundary.country", "DE"),
            ]),
            "api_key=k+e%2Fy%2B%C3%A4&text=Musterstr.+1%2C+80331+M%C3%BCnchen+~_-.&size=1&boundary.country=DE"
        );
    }

    #[test]
    fn jeder_fehler_ist_nicht_verfuegbar_ohne_schluessel_und_url() {
        // HTTP-Status, kaputtes JSON, Verbindung verweigert: alle drei sind 503 in Python.
        let (basis, h) = dienst(vec![
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                .into(),
            ok("das ist kein JSON"),
        ]);
        let ors = Ors::aus_werten(SCHLUESSEL, &basis).unwrap();
        let mut fehler = vec![ors.geocode("a").unwrap_err(), ors.geocode("b").unwrap_err()];
        h.join().unwrap();
        let tot = {
            let l = TcpListener::bind("127.0.0.1:0").unwrap();
            format!("http://127.0.0.1:{}", l.local_addr().unwrap().port())
        };
        fehler.push(
            Ors::aus_werten(SCHLUESSEL, &tot)
                .unwrap()
                .geocode("c")
                .unwrap_err(),
        );
        assert_eq!(
            fehler.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
            [
                "ORS-Aufruf fehlgeschlagen: HTTPError",
                "ORS-Aufruf fehlgeschlagen: JSONDecodeError",
                "ORS-Aufruf fehlgeschlagen: URLError"
            ]
        );
        for e in &fehler {
            let text = format!("{e} {e:?}");
            assert!(
                !text.contains(SCHLUESSEL) && !text.contains("127.0.0.1"),
                "{text}"
            );
        }
    }

    #[test]
    fn kein_schluessel_ist_nicht_verfuegbar_ohne_anfrage() {
        assert!(Ors::aus_werten("", "").is_err());
        assert!(Ors::aus_werten(" \t\n", "").is_err());
        let o = Ors::aus_werten("k", "  ").unwrap();
        assert_eq!(o.basis, STANDARD_BASIS);
    }
}
