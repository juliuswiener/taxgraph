//! Eine einzelne HTTP/1.1-Anfrage (POST fuer den Chat, GET und POST fuer `ors`), gebaut fuer die
//! Fehlerklassen von `llm_client._ein_versuch` (`llm_client.py:276-331`).
//!
//! Warum kein fertiger HTTP-Client: Python unterscheidet zwei Zeitgrenzen, und die Unterscheidung
//! entscheidet ueber Wiederholen oder Aufgeben. Der Socket-Timeout gilt JE LESEOPERATION (ein
//! stehender Anbieter → `TimeoutError` → wiederholbar); die Wanduhr-Frist wird ZWISCHEN den
//! Leseoperationen geprueft (ein troepfelnder Anbieter → Frist → endgueltig). Gaengige Clients
//! kennen nur Gesamt- oder Phasen-Timeouts und koennen die zwei Faelle nicht auseinanderhalten.
use std::fmt::Write as _;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Wie ein Versuch endete, bevor der Client ihn einordnet.
#[derive(Debug)]
pub(crate) enum Transport {
    /// Verbindung/Senden gescheitert oder Leseoperation lief in den Socket-Timeout —
    /// Pythons `URLError`/`TimeoutError`.
    Netz(String),
    Zeit,
    /// Frist zwischen zwei Leseoperationen ueberschritten (`_lies_bis`).
    Frist,
    /// Alles andere (Verbindung ohne Antwort, kaputter Statuszeile, abgerissener Koerper) —
    /// Pythons generischer `except Exception`-Zweig.
    Kaputt(String),
}

/// Eine gelesene Antwort: Status und Koerper.
#[derive(Debug)]
pub(crate) struct Antwort {
    pub status: u16,
    pub koerper: Vec<u8>,
}

enum Strom {
    Klar(TcpStream),
    Tls(Box<rustls::StreamOwned<rustls::ClientConnection, TcpStream>>),
}

impl Read for Strom {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Klar(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for Strom {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Klar(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Klar(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
    }
}

/// Zerlegte Basis-URL (`$LLM_API_BASE`).
struct Ziel {
    tls: bool,
    host: String,
    port: u16,
    pfad: String,
}

fn zerlege(url: &str) -> Result<Ziel, Transport> {
    let (tls, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return Err(Transport::Kaputt("unbekanntes URL-Schema".into()));
    };
    let (autoritaet, pfad) = rest.find('/').map_or((rest, "/"), |i| {
        (rest.get(..i).unwrap_or(""), rest.get(i..).unwrap_or("/"))
    });
    let standard = if tls { 443 } else { 80 };
    let (host, port) = match autoritaet.rsplit_once(':') {
        // `[::1]` ohne Port: das letzte `:` liegt IM Literal, nicht vor einer Portnummer.
        _ if autoritaet.starts_with('[') && autoritaet.ends_with(']') => {
            (autoritaet.to_owned(), standard)
        }
        Some((h, p)) if !h.contains(']') || h.ends_with(']') => (
            h.to_owned(),
            p.parse()
                .map_err(|_| Transport::Kaputt("Port keine Zahl".into()))?,
        ),
        _ => (autoritaet.to_owned(), standard),
    };
    Ok(Ziel {
        tls,
        host,
        port,
        pfad: pfad.to_owned(),
    })
}

/// Wert des `Host:`-Kopfs: der Standardport (443 bei TLS, 80 ohne) bleibt weg, jeder andere steht dabei.
/// Eigene Funktion, weil ein Test den Standardport nicht binden kann (`bind` auf 80/443 gibt Errno 13).
fn host_kopf(tls: bool, host: &str, port: u16) -> String {
    if (tls && port == 443) || (!tls && port == 80) {
        host.to_owned()
    } else {
        format!("{host}:{port}")
    }
}

fn verbinde(ziel: &Ziel, socket: Duration) -> Result<Strom, Transport> {
    let adressen = (ziel.host.trim_matches(['[', ']']), ziel.port)
        .to_socket_addrs()
        .map_err(|e| Transport::Netz(format!("URLError: {e}")))?;
    let mut letzter = String::from("keine Adresse");
    for a in adressen {
        match TcpStream::connect_timeout(&a, socket) {
            Ok(s) => {
                s.set_read_timeout(Some(socket))
                    .map_err(|e| Transport::Netz(e.to_string()))?;
                s.set_write_timeout(Some(socket))
                    .map_err(|e| Transport::Netz(e.to_string()))?;
                return if ziel.tls {
                    tls(ziel, s)
                } else {
                    Ok(Strom::Klar(s))
                };
            }
            Err(e) => letzter = e.to_string(),
        }
    }
    Err(Transport::Netz(format!("URLError: {letzter}")))
}

fn tls(ziel: &Ziel, s: TcpStream) -> Result<Strom, Transport> {
    let wurzeln = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    let konfig = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|e| Transport::Netz(format!("URLError: {e}")))?
    .with_root_certificates(wurzeln)
    .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from(ziel.host.clone())
        .map_err(|e| Transport::Netz(format!("URLError: {e}")))?;
    let verbindung = rustls::ClientConnection::new(Arc::new(konfig), name)
        .map_err(|e| Transport::Netz(format!("URLError: {e}")))?;
    Ok(Strom::Tls(Box::new(rustls::StreamOwned::new(
        verbindung, s,
    ))))
}

fn ist_zeit(e: &std::io::Error) -> bool {
    matches!(
        e.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    )
}

/// POST `koerper` an `<basis>/chat/completions`. `ende` = Wanduhr-Frist fuer das Lesen des
/// Erfolgs-Koerpers; `socket` = Grenze je Verbindungsaufbau/Leseoperation.
pub(crate) fn post(
    basis: &str,
    schluessel: &str,
    koerper: &[u8],
    socket: Duration,
    ende: Instant,
) -> Result<Antwort, Transport> {
    senden(
        "POST",
        &format!("{basis}/chat/completions"),
        &[
            ("Authorization", &format!("Bearer {schluessel}")),
            ("Content-Type", "application/json"),
        ],
        Some(koerper),
        socket,
        ende,
    )
}

/// Eine Anfrage `methode` an `url` (mit Query, falls vorhanden). `kopf` steht nach `Host:` in
/// dieser Reihenfolge; mit `koerper` folgt `Content-Length`. Die Fehlerklassen sind die von
/// [`post`]: der Client ordnet sie ein, hier wird nur gesendet und gelesen.
pub(crate) fn senden(
    methode: &str,
    url: &str,
    kopf: &[(&str, &str)],
    koerper: Option<&[u8]>,
    socket: Duration,
    ende: Instant,
) -> Result<Antwort, Transport> {
    let ziel = zerlege(url)?;
    let mut strom = verbinde(&ziel, socket)?;
    let host = host_kopf(ziel.tls, &ziel.host, ziel.port);
    let mut kopf_text = format!("{methode} {} HTTP/1.1\r\nHost: {host}\r\n", ziel.pfad);
    for (name, wert) in kopf {
        let _ = write!(kopf_text, "{name}: {wert}\r\n");
    }
    if let Some(k) = koerper {
        let _ = write!(kopf_text, "Content-Length: {}\r\n", k.len());
    }
    kopf_text
        .push_str("Accept-Encoding: identity\r\nUser-Agent: taxgraph\r\nConnection: close\r\n\r\n");
    strom
        .write_all(kopf_text.as_bytes())
        .and_then(|()| strom.write_all(koerper.unwrap_or_default()))
        .and_then(|()| strom.flush())
        .map_err(|e| {
            if ist_zeit(&e) {
                Transport::Zeit
            } else {
                Transport::Netz(format!("URLError: {e}"))
            }
        })?;
    let mut leser = BufReader::new(strom);
    let (status, rahmen) = lies_kopf(&mut leser)?;
    if !(200..300).contains(&status) {
        // Python liest den Fehlerkoerper ohne Frist (`e.read()`); scheitert das Lesen, bleibt
        // die Meldung leer und der Status entscheidet trotzdem.
        let koerper = lies_koerper(&mut leser, &rahmen, None).unwrap_or_default();
        return Ok(Antwort { status, koerper });
    }
    let koerper = lies_koerper(&mut leser, &rahmen, Some(ende))?;
    Ok(Antwort { status, koerper })
}

enum Rahmen {
    Laenge(usize),
    Stueckweise,
    BisEnde,
}

fn lies_zeile(leser: &mut impl BufRead) -> Result<String, Transport> {
    let mut zeile = Vec::new();
    match leser.read_until(b'\n', &mut zeile) {
        Ok(_) => Ok(String::from_utf8_lossy(&zeile)
            .trim_end_matches(['\r', '\n'])
            .to_owned()),
        Err(e) if ist_zeit(&e) => Err(Transport::Zeit),
        Err(e) => Err(Transport::Kaputt(e.to_string())),
    }
}

fn lies_kopf(leser: &mut impl BufRead) -> Result<(u16, Rahmen), Transport> {
    let status_zeile = lies_zeile(leser)?;
    if status_zeile.is_empty() {
        return Err(Transport::Kaputt(
            "RemoteDisconnected: Verbindung ohne Antwort geschlossen".into(),
        ));
    }
    let status: u16 = status_zeile
        .split_whitespace()
        .nth(1)
        .filter(|_| status_zeile.starts_with("HTTP/"))
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Transport::Kaputt("BadStatusLine".into()))?;
    let mut rahmen = Rahmen::BisEnde;
    loop {
        let zeile = lies_zeile(leser)?;
        if zeile.is_empty() {
            break;
        }
        let Some((name, wert)) = zeile.split_once(':') else {
            continue;
        };
        let (name, wert) = (name.trim().to_ascii_lowercase(), wert.trim());
        if name == "transfer-encoding" && wert.to_ascii_lowercase().contains("chunked") {
            rahmen = Rahmen::Stueckweise;
        } else if name == "content-length" && !matches!(rahmen, Rahmen::Stueckweise) {
            rahmen = Rahmen::Laenge(
                wert.parse()
                    .map_err(|_| Transport::Kaputt("Content-Length".into()))?,
            );
        }
    }
    Ok((status, rahmen))
}

/// Liest `n` Bytes oder bis EOF (`n = None`) in Bloecken zu 64 KiB, Frist-Pruefung VOR jedem
/// Block.
///
/// PARITAET: `_lies_bis` ruft `r.read(65536)`, und `http.client` fuellt diesen Block ueber einen
/// `BufferedReader` VOLLSTAENDIG (bis 64 KiB, Restlaenge oder EOF), bevor die Schleife wieder auf
/// die Uhr sieht. Eine troepfelnde Antwort unter 64 KiB reisst die Frist in Python deshalb nie —
/// sie begrenzt nur den Socket-Timeout je `recv`. Rust bildet genau das nach (Befund im Bericht;
/// Korrektur = Frist je `recv`, eigener Schritt).
fn lies_bis(
    leser: &mut impl Read,
    n: Option<usize>,
    frist: Option<Instant>,
    out: &mut Vec<u8>,
) -> Result<(), Transport> {
    let mut rest = n;
    let mut puffer = vec![0u8; 65536];
    loop {
        if frist.is_some_and(|f| Instant::now() >= f) {
            return Err(Transport::Frist);
        }
        let max = rest.map_or(puffer.len(), |r| r.min(puffer.len()));
        if max == 0 {
            return Ok(());
        }
        let block = puffer.get_mut(..max).unwrap_or_default();
        let mut gefuellt = 0;
        while gefuellt < block.len() {
            match leser.read(block.get_mut(gefuellt..).unwrap_or_default()) {
                Ok(0) => break,
                Ok(k) => gefuellt += k,
                Err(e) if ist_zeit(&e) => return Err(Transport::Zeit),
                Err(e) => return Err(Transport::Kaputt(e.to_string())),
            }
        }
        if gefuellt == 0 {
            // `read` liefert b"" — Ende. Auch vor erreichter Content-Length: Pythons
            // `HTTPResponse.readinto` meldet das nicht, `json.loads` entscheidet danach.
            return Ok(());
        }
        out.extend_from_slice(block.get(..gefuellt).unwrap_or_default());
        rest = rest.map(|r| r - gefuellt);
    }
}

fn lies_koerper(
    leser: &mut impl BufRead,
    rahmen: &Rahmen,
    frist: Option<Instant>,
) -> Result<Vec<u8>, Transport> {
    let mut out = Vec::new();
    match rahmen {
        Rahmen::Laenge(n) => lies_bis(leser, Some(*n), frist, &mut out)?,
        Rahmen::BisEnde => lies_bis(leser, None, frist, &mut out)?,
        Rahmen::Stueckweise => loop {
            if frist.is_some_and(|f| Instant::now() >= f) {
                return Err(Transport::Frist);
            }
            let zeile = lies_zeile(leser)?;
            let groesse = usize::from_str_radix(zeile.split(';').next().unwrap_or("").trim(), 16)
                .map_err(|_| Transport::Kaputt("IncompleteRead: Chunk-Groesse".into()))?;
            if groesse == 0 {
                break;
            }
            lies_bis(leser, Some(groesse), frist, &mut out)?;
            lies_zeile(leser)?;
        },
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{host_kopf, zerlege};

    #[test]
    fn url_zerlegen() {
        let z = zerlege("http://127.0.0.1:8080/v1/chat/completions")
            .ok()
            .unwrap();
        assert_eq!(
            (z.tls, z.host.as_str(), z.port, z.pfad.as_str()),
            (false, "127.0.0.1", 8080, "/v1/chat/completions")
        );
        let z = zerlege("https://openrouter.ai/api/v1/chat/completions")
            .ok()
            .unwrap();
        assert_eq!((z.tls, z.port), (true, 443));
    }

    /// IPv6-Literal in Klammern: ohne Port 80 (`http`) oder 443 (`https`), mit Port dessen Zahl. Python (`http.client`)
    /// liefert fuer `[::1]` Host `::1` und dieselben Ports; Rust behaelt die Klammern im Host (`verbinde` entfernt sie).
    #[test]
    fn url_zerlegen_ipv6_literal() {
        // `Err` traegt den Fehlertext: eine rote Zeile nennt so den Grund, nicht nur ein `unwrap` auf `None`.
        let fall = |url: &str| {
            zerlege(url)
                .map(|z| (z.tls, z.host, z.port, z.pfad))
                .map_err(|e| format!("{e:?}"))
        };
        assert_eq!(
            fall("http://[::1]/v1"),
            Ok((false, "[::1]".to_owned(), 80, "/v1".to_owned()))
        );
        assert_eq!(
            fall("https://[::1]/v1"),
            Ok((true, "[::1]".to_owned(), 443, "/v1".to_owned()))
        );
        assert_eq!(
            fall("http://[::1]:8080/v1"),
            Ok((false, "[::1]".to_owned(), 8080, "/v1".to_owned()))
        );
        // ohne Pfad: die Autoritaet ist die ganze Rest-Zeichenkette
        assert_eq!(
            fall("http://[::1]"),
            Ok((false, "[::1]".to_owned(), 80, "/".to_owned()))
        );
    }

    /// `Host:`-Kopf an allen vier Ecken von (TLS, Port): nur der Standardport der JEWEILIGEN Schicht entfaellt.
    /// (http, 443) und (https, 80) sind keine Standardports und tragen den Port; das fangen `&&` -> `||` und `443` -> `444`.
    #[test]
    fn host_kopf_laesst_nur_den_standardport_weg() {
        assert_eq!(host_kopf(false, "h", 80), "h");
        assert_eq!(host_kopf(true, "h", 443), "h");
        assert_eq!(host_kopf(false, "h", 443), "h:443");
        assert_eq!(host_kopf(true, "h", 80), "h:80");
        assert_eq!(host_kopf(false, "h", 8080), "h:8080");
        assert_eq!(host_kopf(true, "[::1]", 443), "[::1]");
        assert_eq!(host_kopf(false, "[::1]", 8080), "[::1]:8080");
    }
}
