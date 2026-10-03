//! Die Annahmeschleife des Dienstes: `axum::serve` mit einer Einstellung mehr.
//!
//! Hyper schreibt Kopfnamen klein (`content-type`), Pythons `BaseHTTPRequestHandler` schreibt sie
//! `Content-Type`. Nach RFC 9110 §5.1 sind beide Namen gleich. Ein Client, der den Namen als Text
//! sucht, sieht den Unterschied aber: `tests/test_idor_und_csp.py` liest `Content-Security-Policy`
//! mit `dict.get`. `axum::serve` bietet keinen Zugriff auf den Hyper-Builder, darum läuft die
//! Schleife hier selbst. Sie stellt `title_case_headers` ein und sonst nichts: gleiche Dienste,
//! gleiches Ende bei SIGTERM (offene Verbindungen laufen aus, neue kommen nicht mehr an).
//!
//! `preserve_header_case` hilft hier nicht: Hyper merkt sich damit die Schreibweise der
//! EMPFANGENEN Namen. Für eine Antwort gibt es keinen öffentlichen Weg, die Schreibweise zu setzen.
use std::convert::Infallible;
use std::future::Future;
use std::io;
use std::pin::pin;
use std::time::Duration;

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio::task::JoinSet;
use tower::ServiceExt;

/// Wartezeit nach einem Fehler in `accept` (zu viele offene Dateien): wie bei `axum::serve`.
const ANNAHME_PAUSE: Duration = Duration::from_secs(1);

/// Nimmt Verbindungen an `listener` an und beantwortet sie mit `dienst`, bis `halt` fertig ist.
/// Danach kommen keine neuen Verbindungen an; die offenen enden nach ihrer laufenden Anfrage, und
/// die Funktion kehrt zurück, wenn die letzte zu ist.
///
/// Die Kopfnamen der Antworten stehen in Title-Case (`Content-Type`).
///
/// # Errors
/// Nie im Normalfall: ein Fehler in `accept` führt zu einer Pause und einem neuen Versuch. Der
/// Rückgabetyp bleibt `io::Result`, damit `main` denselben Typ behält wie bei `axum::serve`.
pub async fn dienen<S>(
    listener: TcpListener,
    dienst: S,
    halt: impl Future<Output = ()>,
) -> io::Result<()>
where
    S: tower::Service<Request, Response = Response, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send,
{
    let (stopp_tx, stopp_rx) = watch::channel(false);
    let mut verbindungen = JoinSet::new();
    let mut halt = pin!(halt);
    loop {
        let angenommen = tokio::select! {
            () = &mut halt => break,
            angenommen = listener.accept() => angenommen,
        };
        let Ok((strom, _)) = angenommen else {
            tokio::time::sleep(ANNAHME_PAUSE).await;
            continue;
        };
        let hyper_dienst = TowerToHyperService::new(
            dienst
                .clone()
                .map_request(|req: axum::http::Request<_>| req.map(Body::new)),
        );
        let mut stopp = stopp_rx.clone();
        verbindungen.spawn(async move {
            let mut baumeister = Builder::new(TokioExecutor::new());
            baumeister.http1().title_case_headers(true);
            let mut verbindung =
                pin!(baumeister.serve_connection_with_upgrades(TokioIo::new(strom), hyper_dienst));
            loop {
                tokio::select! {
                    _ = verbindung.as_mut() => break,
                    _ = stopp.changed() => verbindung.as_mut().graceful_shutdown(),
                }
            }
        });
    }
    drop(listener);
    // Der Sender lebt bis hierher; `send` meldet jeder offenen Verbindung das Ende.
    let _ = stopp_tx.send(true);
    while verbindungen.join_next().await.is_some() {}
    Ok(())
}
