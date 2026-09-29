//! Ein einzelner HTTP/1.1-POST, gebaut fuer die Fehlerklassen von `llm_client._ein_versuch`
//! (`llm_client.py:276-331`).
//!
//! Warum kein fertiger HTTP-Client: Python unterscheidet zwei Zeitgrenzen, und die Unterscheidung
//! entscheidet ueber Wiederholen oder Aufgeben. Der Socket-Timeout gilt JE LESEOPERATION (ein
//! stehender Anbieter → `TimeoutError` → wiederholbar); die Wanduhr-Frist wird ZWISCHEN den
//! Leseoperationen geprueft (ein troepfelnder Anbieter → Frist → endgueltig). Gaengige Clients
//! kennen nur Gesamt- oder Phasen-Timeouts und koennen die zwei Faelle nicht auseinanderhalten.
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
    let (autoritaet, pfad) = rest.find('/').map_or((rest, "/"), |i| (rest.get(..i).unwrap_or(""), rest.get(i..).unwrap_or("/")));
    let (host, port) = match autoritaet.rsplit_once(':') {
        Some((h, p)) if !h.contains(']') || h.ends_with(']') => {
            (h.to_owned(), p.parse().map_err(|_| Transport::Kaputt("Port keine Zahl".into()))?)
        }
        _ => (autoritaet.to_owned(), if tls { 443 } else { 80 }),
    };
    Ok(Ziel { tls, host, port, pfad: pfad.to_owned() })
}

fn verbinde(ziel: &Ziel, socket: Duration) -> Result<Strom, Transport> {
    let adressen = (ziel.host.trim_matches(['[', ']']), ziel.port)
        .to_socket_addrs()
        .map_err(|e| Transport::Netz(format!("URLError: {e}")))?;
    let mut letzter = String::from("keine Adresse");
    for a in adressen {
        match TcpStream::connect_timeout(&a, socket) {
            Ok(s) => {
                s.set_read_timeout(Some(socket)).map_err(|e| Transport::Netz(e.to_string()))?;
                s.set_write_timeout(Some(socket)).map_err(|e| Transport::Netz(e.to_string()))?;
                return if ziel.tls { tls(ziel, s) } else { Ok(Strom::Klar(s)) };
            }
            Err(e) => letzter = e.to_string(),
        }
    }
    Err(Transport::Netz(format!("URLError: {letzter}")))
}

fn tls(ziel: &Ziel, s: TcpStream) -> Result<Strom, Transport> {
    let wurzeln = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
    let konfig = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| Transport::Netz(format!("URLError: {e}")))?
        .with_root_certificates(wurzeln)
        .with_no_client_auth();
    let name = rustls::pki_types::ServerName::try_from(ziel.host.clone())
        .map_err(|e| Transport::Netz(format!("URLError: {e}")))?;
    let verbindung = rustls::ClientConnection::new(Arc::new(konfig), name)
        .map_err(|e| Transport::Netz(format!("URLError: {e}")))?;
    Ok(Strom::Tls(Box::new(rustls::StreamOwned::new(verbindung, s))))
}

fn ist_zeit(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut)
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
    let ziel = zerlege(&format!("{basis}/chat/completions"))?;
    let mut strom = verbinde(&ziel, socket)?;
    let host = if (ziel.tls && ziel.port == 443) || (!ziel.tls && ziel.port == 80) {
        ziel.host.clone()
    } else {
        format!("{}:{}", ziel.host, ziel.port)
    };
    let kopf = format!(
        "POST {} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {schluessel}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccept-Encoding: identity\r\nUser-Agent: taxgraph\r\nConnection: close\r\n\r\n",
        ziel.pfad,
        koerper.len()
    );
    strom
        .write_all(kopf.as_bytes())
        .and_then(|()| strom.write_all(koerper))
        .and_then(|()| strom.flush())
        .map_err(|e| if ist_zeit(&e) { Transport::Zeit } else { Transport::Netz(format!("URLError: {e}")) })?;
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
        Ok(_) => Ok(String::from_utf8_lossy(&zeile).trim_end_matches(['\r', '\n']).to_owned()),
        Err(e) if ist_zeit(&e) => Err(Transport::Zeit),
        Err(e) => Err(Transport::Kaputt(e.to_string())),
    }
}

fn lies_kopf(leser: &mut impl BufRead) -> Result<(u16, Rahmen), Transport> {
    let status_zeile = lies_zeile(leser)?;
    if status_zeile.is_empty() {
        return Err(Transport::Kaputt("RemoteDisconnected: Verbindung ohne Antwort geschlossen".into()));
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
        let Some((name, wert)) = zeile.split_once(':') else { continue };
        let (name, wert) = (name.trim().to_ascii_lowercase(), wert.trim());
        if name == "transfer-encoding" && wert.to_ascii_lowercase().contains("chunked") {
            rahmen = Rahmen::Stueckweise;
        } else if name == "content-length" && !matches!(rahmen, Rahmen::Stueckweise) {
            rahmen = Rahmen::Laenge(wert.parse().map_err(|_| Transport::Kaputt("Content-Length".into()))?);
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
fn lies_bis(leser: &mut impl Read, n: Option<usize>, frist: Option<Instant>, out: &mut Vec<u8>) -> Result<(), Transport> {
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

fn lies_koerper(leser: &mut impl BufRead, rahmen: &Rahmen, frist: Option<Instant>) -> Result<Vec<u8>, Transport> {
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
    use super::zerlege;

    #[test]
    fn url_zerlegen() {
        let z = zerlege("http://127.0.0.1:8080/v1/chat/completions").ok().unwrap();
        assert_eq!((z.tls, z.host.as_str(), z.port, z.pfad.as_str()), (false, "127.0.0.1", 8080, "/v1/chat/completions"));
        let z = zerlege("https://openrouter.ai/api/v1/chat/completions").ok().unwrap();
        assert_eq!((z.tls, z.port), (true, 443));
    }
}
