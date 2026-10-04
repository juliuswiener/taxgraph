//! Der Kartendienst-Transport am Draht gegen die Python-Referenz (`ors_client.py`: `geocode`, `_distanz_meter`, `_hole`,
//! `_key`, `_basis`), am Aufrufort von `llm::ors` geprueft (N4, Mutationsmessung `rust/llm`, Teil `ors.rs`). Jede
//! Erwartung ist die Anfrage bzw. Meldung, die der Python-Client gegen einen lokalen Stub erzeugt hat; sie steht hier als
//! Literal, kein Test ruft Python, kein Test geht ins Netz. `Host` traegt den Port und `Content-Length` die Koerperlaenge;
//! der Agent ist Rust-eigen (`taxgraph`, Python sendet `Python-urllib`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

mod stub;

use std::time::Duration;

use llm::ors::{NichtVerfuegbar, Ors, STANDARD_BASIS};
use serde_json::{json, Value};
use stub::{antwort, Aktion, Stub};

const GEO: &str = r#"{"features": [{"geometry": {"coordinates": [8.5, 50.25]}}]}"#;
const ROUTE: &str = r#"{"routes": [{"summary": {"distance": 12345.6}}]}"#;

fn ors_an(stub: &Stub) -> Ors {
    Ors::aus_werten(" sk-ors-123 ", &stub.basis("")).unwrap()
}

/// `urlencode`: Leerzeichen als `+`, `A-Za-z0-9_.-~` bleiben, alles andere als `%XX` (gross) ueber UTF-8; Schluessel zuerst,
/// dann text, size, boundary.country.
#[test]
#[rustfmt::skip]
fn geocode_anfrage_wie_python() {
    let faelle: &[(&str, &str)] = &[
        ("Musterstr. 1, 12345 Köln", "GET /geocode/search?api_key=sk-ors-123&text=Musterstr.+1%2C+12345+K%C3%B6ln&size=1&boundary.country=DE HTTP/1.1"),
        ("A_b-c~d.e*f", "GET /geocode/search?api_key=sk-ors-123&text=A_b-c~d.e%2Af&size=1&boundary.country=DE HTTP/1.1"),
        ("ÄÖ€ ü/?&=+#%", "GET /geocode/search?api_key=sk-ors-123&text=%C3%84%C3%96%E2%82%AC+%C3%BC%2F%3F%26%3D%2B%23%25&size=1&boundary.country=DE HTTP/1.1"),
        ("0123456789 ABCxyz", "GET /geocode/search?api_key=sk-ors-123&text=0123456789+ABCxyz&size=1&boundary.country=DE HTTP/1.1"),
        ("a\nb\tc\r", "GET /geocode/search?api_key=sk-ors-123&text=a%0Ab%09c%0D&size=1&boundary.country=DE HTTP/1.1"),
        ("", "GET /geocode/search?api_key=sk-ors-123&text=&size=1&boundary.country=DE HTTP/1.1"),
        (" ", "GET /geocode/search?api_key=sk-ors-123&text=+&size=1&boundary.country=DE HTTP/1.1"),
        ("é😀", "GET /geocode/search?api_key=sk-ors-123&text=%C3%A9%F0%9F%98%80&size=1&boundary.country=DE HTTP/1.1"),
        ("'\"<>", "GET /geocode/search?api_key=sk-ors-123&text=%27%22%3C%3E&size=1&boundary.country=DE HTTP/1.1"),
        ("a  b", "GET /geocode/search?api_key=sk-ors-123&text=a++b&size=1&boundary.country=DE HTTP/1.1"),
        ("\u{0}\u{7f}\u{1f}", "GET /geocode/search?api_key=sk-ors-123&text=%00%7F%1F&size=1&boundary.country=DE HTTP/1.1"),
        ("ß;:@$!()[]{}|\\^`,", "GET /geocode/search?api_key=sk-ors-123&text=%C3%9F%3B%3A%40%24%21%28%29%5B%5D%7B%7D%7C%5C%5E%60%2C&size=1&boundary.country=DE HTTP/1.1"),
    ];
    for (adresse, zeile) in faelle {
        let stub = Stub::starte(vec![Aktion::Roh(antwort(200, GEO))]);
        let wert = ors_an(&stub).geocode(adresse).unwrap();
        assert_eq!(wert, serde_json::from_str::<Value>(GEO).unwrap());
        let a = &stub.anfragen()[0];
        assert_eq!(a.anfragezeile(), *zeile, "geocode({adresse:?})");
        assert_eq!(a.kopfzeilen_vergleichbar(), vec!["accept-encoding: identity", "connection: close"]);
        assert_eq!(a.kopfzeile("Host"), Some(format!("127.0.0.1:{}", stub.port)));
        assert_eq!(a.kopfzeile("User-Agent").as_deref(), Some("taxgraph"));
        assert!(a.koerper.is_empty());
    }
}

/// Routing: Koerper wie `json.dumps` (Trenner `", "` und `": "`, `ensure_ascii`, Pythons Zahlenschreibweise), Schluessel im
/// Header `Authorization` ohne `Bearer`. Ganze Zahlen ueber `u64` fehlen: `serde_json::Value` kennt sie nicht (ohne
/// `arbitrary_precision`), die Eingabe wird schon beim Einlesen zur Gleitkommazahl; Koordinaten dieser Groesse gibt es nicht.
#[test]
#[rustfmt::skip]
fn route_anfrage_wie_python() {
    let faelle: &[(&str, &str, &str)] = &[
        (r"[8.5, 50.25]", r"[9, 51]", "{\"coordinates\": [[8.5, 50.25], [9, 51]], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r"[-8.5, -50]", r"[0, -0.0]", "{\"coordinates\": [[-8.5, -50], [0, -0.0]], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r"[1e16, 1.5e-7]", r"[123456789012345678, 0.1]", "{\"coordinates\": [[1e+16, 1.5e-07], [123456789012345678, 0.1]], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r"[-9223372036854775808, 0]", r"[9223372036854775807, 18446744073709551615]", "{\"coordinates\": [[-9223372036854775808, 0], [9223372036854775807, 18446744073709551615]], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r#"["a", null]"#, r"[true, false]", "{\"coordinates\": [[\"a\", null], [true, false]], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r#"[[1, 2], {"a": 1, "b": [2]}]"#, r#"["x", 3]"#, "{\"coordinates\": [[[1, 2], {\"a\": 1, \"b\": [2]}], [\"x\", 3]], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r#"["ä\"\\\n\r\t\b\f ~\u007f€😀\u0001"]"#, r"[]", "{\"coordinates\": [[\"\\u00e4\\\"\\\\\\n\\r\\t\\b\\f ~\\u007f\\u20ac\\ud83d\\ude00\\u0001\"], []], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r"{}", r#"[{"k": null}]"#, "{\"coordinates\": [{}, [{\"k\": null}]], \"preference\": \"shortest\", \"units\": \"m\"}"),
        (r"[1.0, 2.5e300]", r"[0.30000000000000004, 1e-300]", "{\"coordinates\": [[1.0, 2.5e+300], [0.30000000000000004, 1e-300]], \"preference\": \"shortest\", \"units\": \"m\"}"),
    ];
    for (von, nach, body) in faelle {
        let stub = Stub::starte(vec![Aktion::Roh(antwort(200, ROUTE))]);
        let wert = ors_an(&stub)
            .route(&serde_json::from_str(von).unwrap(), &serde_json::from_str(nach).unwrap())
            .unwrap();
        assert_eq!(wert, serde_json::from_str::<Value>(ROUTE).unwrap());
        let a = &stub.anfragen()[0];
        assert_eq!(a.anfragezeile(), "POST /v2/directions/driving-car HTTP/1.1");
        assert_eq!(a.kopfzeilen_vergleichbar(), vec!["accept-encoding: identity", "authorization: sk-ors-123", "connection: close", "content-type: application/json"]);
        assert_eq!(a.kopfzeile("Host"), Some(format!("127.0.0.1:{}", stub.port)));
        assert_eq!(a.kopfzeile("Content-Length"), Some(a.koerper.len().to_string()));
        assert_eq!(String::from_utf8_lossy(&a.koerper), *body, "route({von}, {nach})");
    }
}

/// Jeder Fehler — Status ausserhalb 2xx, kein JSON, leerer Koerper — wird `NichtVerfuegbar` mit dem Typnamen der Python-Ausnahme;
/// 2xx und eine Antwort in Stuecken liefern den JSON-Wert.
#[test]
fn antwort_und_fehler_wie_python() {
    let leer = r#"{"features": []}"#;
    let faelle: Vec<(&str, Vec<u8>, Result<Value, &str>)> = vec![
        ("ok_200", antwort(200, leer), Ok(json!({"features": []}))),
        ("ok_201", antwort(201, leer), Ok(json!({"features": []}))),
        ("status_300", antwort(300, leer), Err("ORS-Aufruf fehlgeschlagen: HTTPError")),
        ("status_404", antwort(404, "{}"), Err("ORS-Aufruf fehlgeschlagen: HTTPError")),
        ("status_500", antwort(500, "{}"), Err("ORS-Aufruf fehlgeschlagen: HTTPError")),
        ("kein_json", antwort(200, "<html>"), Err("ORS-Aufruf fehlgeschlagen: JSONDecodeError")),
        ("leerer_koerper", antwort(200, ""), Err("ORS-Aufruf fehlgeschlagen: JSONDecodeError")),
        ("chunked", b"HTTP/1.1 200 X\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n8\r\n{\"featur\r\n8\r\nes\": []}\r\n0\r\n\r\n".to_vec(), Ok(json!({"features": []}))),
    ];
    for (name, roh, erwartet) in faelle {
        let stub = Stub::starte(vec![Aktion::Roh(roh)]);
        let r = ors_an(&stub).geocode("x");
        match (r, erwartet) {
            (Ok(w), Ok(e)) => assert_eq!(w, e, "{name}"),
            (Err(NichtVerfuegbar(m)), Err(e)) => assert_eq!(m, e, "{name}"),
            (r, e) => panic!("{name}: {r:?} statt {e:?}"),
        }
    }
    // Kein Dienst: Verbindung verweigert, Python meldet `URLError`.
    let tot = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let basis = format!("http://127.0.0.1:{}", tot.local_addr().unwrap().port());
    drop(tot);
    let r = Ors::aus_werten("k", &basis).unwrap().geocode("x");
    assert_eq!(r.unwrap_err().0, "ORS-Aufruf fehlgeschlagen: URLError");
    // Eine Verbindung ohne Antwort: Python meldet den Typ `RemoteDisconnected`, Rust nennt jeden sonstigen Fehler `Exception`.
    let stub = Stub::starte(vec![Aktion::Schliessen]);
    assert_eq!(
        ors_an(&stub).geocode("x").unwrap_err().0,
        "ORS-Aufruf fehlgeschlagen: Exception"
    );
}

/// `_TIMEOUT` 8: eine Antwort nach 7,5 s kommt an (Python liest sie).
#[test]
fn antwort_nach_7_5_sekunden_kommt_an() {
    let stub = Stub::starte(vec![Aktion::Verzoegert(
        Duration::from_millis(7500),
        antwort(200, GEO),
    )]);
    assert!(ors_an(&stub).geocode("x").is_ok());
}

/// `_TIMEOUT` 8: Schweigen ueber 8 s ist `TimeoutError` (Python: `type(e).__name__`).
#[test]
fn schweigen_ueber_8_sekunden_ist_timeout_wie_python() {
    let stub = Stub::starte(vec![Aktion::Stille(Duration::from_millis(8600))]);
    assert_eq!(
        ors_an(&stub).geocode("x").unwrap_err().0,
        "ORS-Aufruf fehlgeschlagen: TimeoutError"
    );
}

/// Schluessel gestrippt, Basis gestrippt und ohne Schraegstriche am Ende (`_key`, `_basis`): die Anfragezeile ist die des
/// Python-Clients. Leere Basis ist der Standard, ein leerer Schluessel ist nicht verfuegbar.
#[test]
#[rustfmt::skip]
fn schluessel_und_basis_werden_bereinigt_wie_python() {
    let faelle: &[(&str, &str, &str, &str, &str)] = &[
        ("plain", "k1", "", "", "GET /geocode/search?api_key=k1&text=x&size=1&boundary.country=DE HTTP/1.1"),
        ("schluessel_leerraum", " \tk2\n ", "", "", "GET /geocode/search?api_key=k2&text=x&size=1&boundary.country=DE HTTP/1.1"),
        ("basis_schraegstrich", "k3", "", "/", "GET /geocode/search?api_key=k3&text=x&size=1&boundary.country=DE HTTP/1.1"),
        ("basis_schraegstriche", "k4", "", "///", "GET /geocode/search?api_key=k4&text=x&size=1&boundary.country=DE HTTP/1.1"),
        ("basis_leerraum", "k5", "  ", " \n", "GET /geocode/search?api_key=k5&text=x&size=1&boundary.country=DE HTTP/1.1"),
        ("basis_pfad_und_schraegstrich", "k6", "", "/v9//", "GET /v9/geocode/search?api_key=k6&text=x&size=1&boundary.country=DE HTTP/1.1"),
        ("basis_leerraum_und_schraegstrich", "k7", " ", "/ ", "GET /geocode/search?api_key=k7&text=x&size=1&boundary.country=DE HTTP/1.1"),
    ];
    for (name, schluessel, vor, nach, zeile) in faelle {
        let stub = Stub::starte(vec![Aktion::Roh(antwort(200, GEO))]);
        let basis = format!("{vor}http://127.0.0.1:{}{nach}", stub.port);
        Ors::aus_werten(schluessel, &basis).unwrap().geocode("x").unwrap();
        assert_eq!(stub.anfragen()[0].anfragezeile(), *zeile, "{name}");
    }
    assert_eq!(STANDARD_BASIS, "https://api.openrouteservice.org");
    let o = Ors::aus_werten("k", "").unwrap();
    assert!(format!("{o:?}").contains(STANDARD_BASIS), "{o:?}");
    let o = Ors::aus_werten("k", " /// ").unwrap();
    assert!(format!("{o:?}").contains(STANDARD_BASIS), "{o:?}");
    for leer in ["", " ", "\n\t "] {
        assert_eq!(Ors::aus_werten(leer, "http://x").unwrap_err().0, "kein ORS_API_KEY in der Umgebung (.env.maps nicht geladen?)");
    }
}
