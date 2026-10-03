//! `Ors::aus_env` liest `$ORS_API_KEY` und `$ORS_API_BASE` zur Aufrufzeit (Python: `ors_client._key`,
//! `ors_client._basis`). Eine eigene Testdatei mit EINEM Test, weil die Umgebung prozessweit gilt.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use std::io::{Read, Write};
use std::net::TcpListener;

use llm::ors::{Ors, STANDARD_BASIS};

#[test]
fn die_umgebung_bestimmt_schluessel_und_basis_zur_aufrufzeit() {
    // Ohne Schluessel: nicht verfuegbar, ohne dass eine Anfrage entsteht.
    std::env::remove_var("ORS_API_KEY");
    std::env::remove_var("ORS_API_BASE");
    assert!(Ors::aus_env().is_err());

    // Ein lokaler Dienst, der eine Anfrage beantwortet und sie zurueckgibt.
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let h = std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut b = [0u8; 4096];
        let n = s.read(&mut b).unwrap();
        s.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"features\": []}",
        )
        .unwrap();
        String::from_utf8_lossy(&b[..n]).into_owned()
    });
    std::env::set_var("ORS_API_KEY", " SYNTHETISCH\n");
    std::env::set_var("ORS_API_BASE", format!("http://127.0.0.1:{port}/"));
    let antwort = Ors::aus_env().unwrap().geocode("x").unwrap();
    assert_eq!(antwort, serde_json::json!({"features": []}));
    let anfrage = h.join().unwrap();
    assert!(
        anfrage.starts_with(
            "GET /geocode/search?api_key=SYNTHETISCH&text=x&size=1&boundary.country=DE HTTP/1.1\r\n"
        ),
        "{anfrage}"
    );

    // Leere Basis = Standard (kein Aufruf: nur die Konfiguration wird geprueft).
    std::env::set_var("ORS_API_BASE", "  ");
    let ors = Ors::aus_env().unwrap();
    assert!(format!("{ors:?}").contains(STANDARD_BASIS), "{ors:?}");
}
