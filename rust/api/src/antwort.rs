//! Antworten: JSON wie `Handler._json` (`server.py:139`), Sicherheits-Header wie
//! `_sicherheits_header` (`server.py:116`), statische Dateien wie `_static`.
use axum::body::Body;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::Value;

/// Content-Security-Policy, Wort für Wort `server.py:135-137`.
const CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; \
img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// Ein Handler-Ergebnis `(status, obj)` — das Rust-Gegenstück zum Rückgabewert der
/// `api.py`-Funktionen.
#[derive(Debug, Clone)]
pub struct Antwort {
    pub status: u16,
    pub body: Value,
}

impl Antwort {
    #[must_use]
    pub fn neu(status: u16, body: Value) -> Self {
        Self { status, body }
    }
}

impl IntoResponse for Antwort {
    fn into_response(self) -> Response {
        json_antwort(self.status, &self.body)
    }
}

fn sicherheits_header(h: &mut HeaderMap) {
    h.insert("content-security-policy", HeaderValue::from_static(CSP));
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
}

fn mit_status(status: u16) -> StatusCode {
    StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// `json.dumps(obj, ensure_ascii=False)` als Antwort mit allen Headern von `_json`; ist der Body
/// nicht serialisierbar, eine 500 mit leerem Rumpf statt einer 200 mit leerem JSON.
#[must_use]
pub fn json_antwort(status: u16, body: &Value) -> Response {
    match auth::py_json(body) {
        Ok(text) => bytes_antwort(status, "application/json; charset=utf-8", text.into_bytes()),
        Err(_) => bytes_antwort(500, "application/json; charset=utf-8", Vec::new()),
    }
}

/// Rumpf plus `Content-Type`, `Content-Length` und die drei Sicherheits-Header.
#[must_use]
pub fn bytes_antwort(status: u16, content_type: &'static str, payload: Vec<u8>) -> Response {
    let laenge = payload.len();
    let mut r = Response::new(Body::from(payload));
    *r.status_mut() = mit_status(status);
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    h.insert(header::CONTENT_LENGTH, HeaderValue::from(laenge));
    sicherheits_header(h);
    r
}

/// Die `Content-Type`-Tabelle `_CTYPE` (`server.py:54`); alles andere ist `octet-stream`.
#[must_use]
pub fn content_type_fuer(endung: Option<&str>) -> &'static str {
    match endung {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// `BaseHTTPRequestHandler.send_error(501, "Unsupported method (…)")` — was Python für jede
/// Methode außer GET/POST/DELETE sendet. Seite und Kopf von Python 3.14 (`http.server`), ohne
/// Sicherheits-Header, weil `send_error` sie nicht kennt.
#[must_use]
pub fn methode_nicht_unterstuetzt(methode: &str) -> Response {
    let meldung = format!("Unsupported method ('{methode}')");
    let seite = format!(
        "<!DOCTYPE HTML>\n<html lang=\"en\">\n    <head>\n        <meta charset=\"utf-8\">\n        \
<style type=\"text/css\">\n            :root {{\n                color-scheme: light dark;\n            }}\n        \
</style>\n        <title>Error response</title>\n    </head>\n    <body>\n        \
<h1>Error response</h1>\n        <p>Error code: 501</p>\n        <p>Message: {}.</p>\n        \
<p>Error code explanation: 501 - Server does not support this operation.</p>\n    </body>\n</html>\n",
        meldung.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
    );
    let mut r = Response::new(Body::from(seite.clone()));
    *r.status_mut() = StatusCode::NOT_IMPLEMENTED;
    r.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html;charset=utf-8"),
    );
    r.headers_mut()
        .insert(header::CONTENT_LENGTH, HeaderValue::from(seite.len()));
    r
}
