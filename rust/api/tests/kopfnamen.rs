//! Kopfnamen der Antworten in Title-Case (`Content-Type`, nicht `content-type`), wie Pythons
//! `BaseHTTPRequestHandler`. `api::dienen::dienen` stellt das ein. Der Vergleich mit Python steht in
//! `rust/parity/tests/api_http_paritaet.rs` (Kopfnamen der fünf verglichenen Köpfe plus `Date`).
//! Dieser Test läuft auch ohne Python.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;
use std::time::Duration;

use api::konfig::Konfig;
use api::{app, Zustand};
use auth::Auth;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

fn zustand(tmp: &Path) -> Zustand {
    let konfig = Konfig {
        wurzel: Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        faelle: tmp.join("faelle"),
        audit_dir: tmp.join("faelle"),
    };
    let auth = Auth::neu(
        "testgeheimnis".into(),
        tmp.join("users.json"),
        Some(konfig.audit_pfad()),
    );
    Zustand::neu(konfig, auth)
}

/// Die Namen der Kopfzeilen einer Antwort, in der Schreibweise auf dem Draht.
async fn kopfnamen(port: u16, pfad: &str) -> Vec<String> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let anfrage = format!("GET {pfad} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    s.write_all(anfrage.as_bytes()).await.unwrap();
    let mut roh = Vec::new();
    s.read_to_end(&mut roh).await.unwrap();
    let text = String::from_utf8_lossy(&roh);
    let kopf = text.split("\r\n\r\n").next().unwrap();
    kopf.lines()
        .skip(1)
        .filter_map(|z| z.split_once(':'))
        .map(|(n, _)| n.to_owned())
        .collect()
}

#[tokio::test]
async fn antwortkoepfe_stehen_in_title_case_und_der_dienst_endet_auf_signal() {
    let tmp = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let (halt_tx, halt_rx) = tokio::sync::oneshot::channel::<()>();
    let dienst = tokio::spawn(api::dienen::dienen(
        listener,
        app(zustand(tmp.path())),
        async {
            let _ = halt_rx.await;
        },
    ));

    // JSON-Pfad und statische Datei: beide Antwortwege des Dispatchers.
    for pfad in ["/health", "/static/style.css", "/nix"] {
        let namen = kopfnamen(port, pfad).await;
        for soll in [
            "Content-Type",
            "Content-Length",
            "Content-Security-Policy",
            "X-Content-Type-Options",
            "Referrer-Policy",
            "Date",
        ] {
            assert!(
                namen.iter().any(|n| n == soll),
                "{pfad}: Kopf {soll} fehlt in dieser Schreibweise: {namen:?}"
            );
        }
        assert!(
            namen
                .iter()
                .all(|n| n.chars().next().is_some_and(|c| c.is_ascii_uppercase())),
            "{pfad}: ein Name beginnt klein: {namen:?}"
        );
    }

    halt_tx.send(()).unwrap();
    let ende = tokio::time::timeout(Duration::from_secs(10), dienst).await;
    assert!(ende.is_ok(), "der Dienst endete nicht auf das Signal");
    assert!(ende.unwrap().unwrap().is_ok());
}
