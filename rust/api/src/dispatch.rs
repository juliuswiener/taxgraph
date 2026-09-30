//! Der Dispatcher: `Handler._dispatch` (`server.py:176`) als axum-Middleware. Er läuft VOR jedem
//! Handler und in Pythons Reihenfolge: Methode, statische Dateien, Host, Origin, Rumpf, Nutzer,
//! Route — und danach Fehlerlog und Audit. Er wickelt auch den Router-Fallback ein, darum greift
//! er für unbekannte Pfade genauso.
use std::sync::OnceLock;

use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderName};
use axum::middleware::Next;
use axum::response::Response;
use regex::{Captures, Regex};
use serde_json::{json, Value};
use store::audit::AuditAktion;
use store::fehler_log::{protokolliere, FallId, Meta, Stufe};

use crate::antwort::{bytes_antwort, content_type_fuer, json_antwort, methode_nicht_unterstuetzt};
use crate::fehler::Ausgang;
use crate::konfig::Konfig;
use crate::routen::{Eintrag, EINTRAEGE};
use crate::zustand::{Koerper, Nutzer, Treffer, Zustand};

/// Höchstmaß eines Anfrage-Rumpfs (`server.py:52`).
pub const MAX_BODY_BYTES: u64 = 32 * 1024 * 1024;

/// Stand-in für den Typ der Ausnahme im Fehlerlog (Python schreibt `type(e).__name__`; der
/// Klartext steht im Antwort-Body, nicht im Protokoll).
struct Ausnahme;

fn kopf(h: &HeaderMap, name: &HeaderName) -> Option<String> {
    h.get(name)
        .map(|v| String::from_utf8_lossy(v.as_bytes()).into_owned())
}

fn fehler_json(status: u16, meldung: &str) -> Response {
    json_antwort(status, &json!({ "fehler": meldung }))
}

fn routen() -> &'static [(&'static Eintrag, Regex)] {
    static TABELLE: OnceLock<Vec<(&'static Eintrag, Regex)>> = OnceLock::new();
    TABELLE.get_or_init(|| {
        // Ein Muster, das nicht kompiliert, fiele hier still weg; `alle_muster_kompilieren` (Test)
        // zählt sie nach.
        EINTRAEGE
            .iter()
            .filter_map(|e| Regex::new(e.muster).ok().map(|r| (e, r)))
            .collect()
    })
}

/// Erste passende Route (Methode UND Muster) — Pythons `for m, muster, fn in ROUTES`.
fn finde_route<'a>(methode: &str, pfad: &'a str) -> Option<(&'static Eintrag, Captures<'a>)> {
    routen()
        .iter()
        .filter(|(e, _)| e.methode == methode)
        .find_map(|(e, r)| r.captures(pfad).map(|c| (*e, c)))
}

/// Host- und Origin-Prüfung (`server.py:184-196`): Schutz gegen DNS-Rebinding und CSRF.
fn grenzpruefung(h: &HeaderMap, methode: &str) -> Option<Response> {
    let host = kopf(h, &header::HOST).unwrap_or_default();
    if !["127.0.0.1", "localhost", ""].contains(&host.split(':').next().unwrap_or("")) {
        return Some(fehler_json(421, "unerwarteter_host"));
    }
    if matches!(methode, "POST" | "DELETE") {
        if let Some(origin) = kopf(h, &header::ORIGIN) {
            let nach_schema = origin.split_once("//").map_or(origin.as_str(), |(_, r)| r);
            let o_host = nach_schema
                .split('/')
                .next()
                .unwrap_or("")
                .split(':')
                .next()
                .unwrap_or("");
            if !["127.0.0.1", "localhost"].contains(&o_host) {
                return Some(fehler_json(403, "cross_origin_verboten"));
            }
        }
    }
    None
}

/// Liest und parst den POST-Rumpf (`server.py:199-228`). Ohne Rumpf: `{}`.
async fn lies_koerper(h: &HeaderMap, body: Body) -> Result<Value, Box<Response>> {
    let laenge = match kopf(h, &header::CONTENT_LENGTH).filter(|t| !t.is_empty()) {
        None => 0,
        // PARITÄT: Pythons `int()` nimmt auch " 5", "+5" und "5_0"; hyper lehnt solche Köpfe
        // schon vor der Anwendung ab (400, leerer Body). Was hier ankommt, ist eine Ziffernfolge.
        Some(t) => t.parse::<u64>().unwrap_or(u64::MAX),
    };
    if laenge > MAX_BODY_BYTES {
        return Err(Box::new(fehler_json(
            413,
            &format!("Anfrage zu groß (Höchstmaß: {MAX_BODY_BYTES} Bytes)"),
        )));
    }
    if laenge > 0
        && !kopf(h, &header::CONTENT_TYPE)
            .unwrap_or_default()
            .starts_with("application/json")
    {
        return Err(Box::new(fehler_json(
            415,
            "Content-Type muss application/json sein",
        )));
    }
    if laenge == 0 {
        return Ok(json!({}));
    }
    let roh = to_bytes(body, usize::try_from(MAX_BODY_BYTES).unwrap_or(usize::MAX))
        .await
        .map_err(|_| Box::new(fehler_json(400, "ungültiges JSON im Body")))?;
    serde_json::from_slice(&roh).map_err(|_| Box::new(fehler_json(400, "ungültiges JSON im Body")))
}

/// `_extract_user` (`server.py:167`): `Authorization`, optional `Bearer `, dann `verify_token`.
fn nutzer_aus_kopf(z: &Zustand, h: &HeaderMap) -> Option<String> {
    let roh = kopf(h, &header::AUTHORIZATION).unwrap_or_default();
    let roh = roh.trim();
    let token = roh.strip_prefix("Bearer ").unwrap_or(roh);
    if token.is_empty() {
        return None;
    }
    z.auth.pruefe_token(token)
}

/// `_static` (`server.py:148`): nur unterhalb von `static/`, sonst 404.
fn statisch(konfig: &Konfig, pfad: &str) -> Response {
    let nicht_gefunden = || fehler_antwort_mit_pfad(pfad);
    let rel = pfad.strip_prefix("/static/").unwrap_or("index.html");
    let Ok(wurzel) = std::fs::canonicalize(konfig.static_dir()) else {
        return nicht_gefunden();
    };
    // `os.path.join(root, rel)`: ein absolutes `rel` ersetzt die Wurzel — der Vergleich unten fängt es.
    let voll = if rel.starts_with('/') {
        std::path::PathBuf::from(rel)
    } else {
        wurzel.join(rel)
    };
    let Ok(voll) = std::fs::canonicalize(voll) else {
        return nicht_gefunden();
    };
    if !voll.starts_with(&wurzel) || !voll.is_file() {
        return nicht_gefunden();
    }
    let Ok(daten) = std::fs::read(&voll) else {
        return nicht_gefunden();
    };
    bytes_antwort(
        200,
        content_type_fuer(voll.extension().and_then(|e| e.to_str())),
        daten,
    )
}

fn fehler_antwort_mit_pfad(pfad: &str) -> Response {
    json_antwort(404, &json!({ "fehler": "not_found", "pfad": pfad }))
}

/// Fehlerlog und Audit nach dem Handler (`server.py:237-264`).
fn nachlauf(z: &Zustand, treffer: &Treffer, pfad: &str, nutzer: Option<&str>, antwort: &Response) {
    let ausgang = antwort.extensions().get::<Ausgang>().copied();
    if ausgang == Some(Ausgang::Unerwartet) {
        // Nicht protokollierbar heißt: die Antwort geht trotzdem raus (wie Python, dort ohne Schutz).
        let fall = treffer.id.as_deref().map(|id| FallId::pruefe(id, &[]));
        // ponytail: die PII-Muster liefert erst `llm::pii`; bis dahin steht `<gesperrt:pii_filter_fehlt>`.
        let _ = protokolliere(
            &z.konfig.fehler_pfad(),
            treffer.ort,
            &Ausnahme,
            Stufe::Fehler,
            fall,
            Meta::default(),
        );
    }
    if !pfad.starts_with("/fall") {
        return;
    }
    // PARITÄT: `status` bleibt in Python bei 500, wenn der Handler eine `ApiError` wirft — der
    // Audit-Eintrag nennt dann 500, obwohl die Antwort 403/404/… war.
    let status = if ausgang.is_some() {
        500
    } else {
        antwort.status().as_u16()
    };
    let uid = nutzer.unwrap_or("dev");
    let detail = format!("status={status}");
    let (aktion, fall_id) = if pfad == "/fall" {
        (Some(AuditAktion::FallCreate), None)
    } else {
        // PARITÄT: `act = pfad.split("/")[-1]` — bei `DELETE /fall/<id>` ist das die Fall-Kennung
        // selbst, die Aktion heißt dann `fall_<id>` (`server.py:263`).
        let act = pfad.rsplit('/').next().unwrap_or("");
        (
            treffer
                .id
                .as_ref()
                .map(|_| AuditAktion::from(format!("fall_{act}").as_str())),
            treffer.id.as_deref(),
        )
    };
    if let Some(aktion) = aktion {
        let _ = store::audit::anhaengen(
            &z.konfig.audit_pfad(),
            Some(uid),
            aktion,
            fall_id,
            Some(&detail),
        );
    }
}

/// `parse_request` (Python ≥ 3.12): ein Pfad, der mit `//` beginnt, wird zu `/` + Rest ohne führende
/// Schrägstriche — `//health` ist `/health`. Läuft VOR dem Router ([`crate::app`]): axum wählt die
/// Route vor den Layern, ein Umschreiben in der Middleware käme zu spät.
#[must_use]
pub fn normalisiere_pfad(mut req: Request) -> Request {
    let roh = req
        .uri()
        .path_and_query()
        .map_or("/", |pq| pq.as_str())
        .to_owned();
    if roh.starts_with("//") {
        if let Ok(uri) = format!("/{}", roh.trim_start_matches('/')).parse() {
            *req.uri_mut() = uri;
        }
    }
    req
}

/// Die Middleware. Jede Antwort dieses Dienstes geht hier durch.
pub async fn dispatch(State(z): State<Zustand>, req: Request, next: Next) -> Response {
    let methode = req.method().as_str().to_owned();
    // BaseHTTPRequestHandler kennt nur do_GET/do_POST/do_DELETE; der Rest ist 501.
    if !matches!(methode.as_str(), "GET" | "POST" | "DELETE") {
        return methode_nicht_unterstuetzt(&methode);
    }
    let pfad = req.uri().path().to_owned();
    if methode == "GET" && (pfad == "/" || pfad.starts_with("/static/")) {
        return statisch(&z.konfig, &pfad);
    }
    if let Some(r) = grenzpruefung(req.headers(), &methode) {
        return r;
    }
    let (mut parts, body) = req.into_parts();
    let koerper = if methode == "POST" {
        match lies_koerper(&parts.headers, body).await {
            Ok(v) => v,
            Err(r) => return *r,
        }
    } else {
        json!({})
    };
    let nutzer = nutzer_aus_kopf(&z, &parts.headers);
    let Some((eintrag, caps)) = finde_route(&methode, &pfad) else {
        return json_antwort(
            404,
            &json!({ "fehler": "route_not_found", "methode": methode, "pfad": pfad }),
        );
    };
    let gruppe = |n: &str| caps.name(n).map(|m| m.as_str().to_owned());
    let treffer = Treffer {
        ort: eintrag.ort,
        id: gruppe("id"),
        fid: gruppe("fid"),
    };
    parts.extensions.insert(Nutzer(nutzer.clone()));
    parts.extensions.insert(Koerper(koerper));
    parts.extensions.insert(treffer.clone());
    let _sperre = z.sperre.lock().await;
    let antwort = next.run(Request::from_parts(parts, Body::empty())).await;
    nachlauf(&z, &treffer, &pfad, nutzer.as_deref(), &antwort);
    antwort
}

/// Router-Fallback: nur erreichbar, wenn [`EINTRAEGE`] und der axum-Router auseinanderlaufen
/// (der Test `jede_route_hat_einen_handler` hält sie zusammen). Antwortet wie Pythons letzte Zeile.
pub async fn nicht_erreicht(req: Request) -> Response {
    json_antwort(
        404,
        &json!({ "fehler": "route_not_found", "methode": req.method().as_str(), "pfad": req.uri().path() }),
    )
}

#[cfg(test)]
mod tests {
    use super::{routen, EINTRAEGE};

    #[test]
    fn alle_muster_kompilieren() {
        assert_eq!(routen().len(), EINTRAEGE.len());
    }
}
