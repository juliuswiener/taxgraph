//! Lokaler Stub-Server fuer die Netz-Tests von `llm` (`client`, `http`, `ors`): je Verbindung EINE Aktion aus dem
//! Skript, danach `Schliessen`; jede Anfrage wird roh festgehalten. Kein Test geht ins Netz.
#![allow(dead_code)]

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Was der Stub auf eine Anfrage tut.
#[derive(Clone)]
pub enum Aktion {
    /// Rohe Antwortbytes senden, dann schliessen.
    Roh(Vec<u8>),
    /// Ohne Antwort schliessen.
    Schliessen,
    /// So lange schweigen (Verbindung offen), dann schliessen.
    Stille(Duration),
    /// `vorne` sofort, dann `rest` Byte fuer Byte im Abstand `pause`.
    Tropfen {
        vorne: Vec<u8>,
        rest: Vec<u8>,
        pause: Duration,
    },
    /// Der Reihe nach: Bytes senden, danach die Pause warten; zuletzt schliessen.
    Stufen(Vec<(Vec<u8>, Duration)>),
    /// Erst warten, dann die Bytes senden, dann schliessen.
    Verzoegert(Duration, Vec<u8>),
}

/// Eine festgehaltene Anfrage: Kopf (Anfragezeile und Kopfzeilen, ohne die Leerzeile) und Koerper.
#[derive(Clone, Debug)]
pub struct Anfrage {
    pub kopf: String,
    pub koerper: Vec<u8>,
}

impl Anfrage {
    /// Wert einer Kopfzeile (Name ohne Beachtung der Gross-/Kleinschreibung).
    pub fn kopfzeile(&self, name: &str) -> Option<String> {
        self.kopf.lines().skip(1).find_map(|l| {
            let (n, w) = l.split_once(':')?;
            n.trim()
                .eq_ignore_ascii_case(name)
                .then(|| w.trim().to_owned())
        })
    }

    pub fn anfragezeile(&self) -> &str {
        self.kopf.lines().next().unwrap_or("")
    }

    /// Die Kopfzeilen ohne `Host`, `User-Agent` und `Content-Length`, im Namen kleingeschrieben, sortiert: der Teil,
    /// den Python und Rust gleich senden (Host traegt den Port, der Agent ist verschieden, die Laenge haengt von der
    /// JSON-Schreibweise ab).
    pub fn kopfzeilen_vergleichbar(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .kopf
            .lines()
            .skip(1)
            .filter_map(|l| {
                let (n, w) = l.split_once(':')?;
                let n = n.trim().to_ascii_lowercase();
                (n != "host" && n != "user-agent" && n != "content-length")
                    .then(|| format!("{n}: {}", w.trim()))
            })
            .collect();
        v.sort();
        v
    }
}

pub struct Stub {
    pub port: u16,
    anfragen: Arc<Mutex<Vec<Anfrage>>>,
    ende: Arc<AtomicBool>,
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.ende.store(true, Ordering::SeqCst);
        // Den wartenden accept() wecken.
        let _ = TcpStream::connect(("127.0.0.1", self.port));
    }
}

fn lies_anfrage(s: &mut TcpStream) -> Option<Anfrage> {
    let mut buf = Vec::new();
    let mut b = [0u8; 4096];
    let kopf_ende = loop {
        let n = s.read(&mut b).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&b[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
    };
    let kopf = String::from_utf8_lossy(&buf[..kopf_ende]).into_owned();
    let laenge: usize = kopf
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|v| v.trim().parse().ok())
        })
        .unwrap_or(0);
    let beginn = kopf_ende + 4;
    while buf.len() < beginn + laenge {
        let n = s.read(&mut b).ok()?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&b[..n]);
    }
    Some(Anfrage {
        kopf,
        koerper: buf[beginn..].to_vec(),
    })
}

fn handle(mut s: TcpStream, a: Aktion) {
    let _ = match a {
        Aktion::Roh(b) => s.write_all(&b),
        Aktion::Schliessen => Ok(()),
        Aktion::Stille(d) => {
            std::thread::sleep(d);
            Ok(())
        }
        Aktion::Tropfen { vorne, rest, pause } => {
            let _ = s.write_all(&vorne);
            for c in rest {
                std::thread::sleep(pause);
                if s.write_all(&[c]).is_err() {
                    break;
                }
            }
            Ok(())
        }
        Aktion::Stufen(stufen) => {
            for (b, pause) in stufen {
                if s.write_all(&b).is_err() {
                    break;
                }
                std::thread::sleep(pause);
            }
            Ok(())
        }
        Aktion::Verzoegert(d, b) => {
            std::thread::sleep(d);
            s.write_all(&b)
        }
    };
}

impl Stub {
    pub fn starte(skript: Vec<Aktion>) -> Self {
        Self::starte_an("127.0.0.1", skript)
    }

    /// Wie `starte`, lauscht aber auf `[::1]`.
    pub fn starte_v6(skript: Vec<Aktion>) -> Self {
        Self::starte_an("[::1]", skript)
    }

    fn starte_an(host: &str, skript: Vec<Aktion>) -> Self {
        let l = TcpListener::bind(format!("{host}:0")).unwrap();
        let port = l.local_addr().unwrap().port();
        let anfragen = Arc::new(Mutex::new(Vec::new()));
        let skript = Arc::new(Mutex::new(skript.into_iter().collect::<VecDeque<_>>()));
        let ende = Arc::new(AtomicBool::new(false));
        let (a, sk, e) = (anfragen.clone(), skript, ende.clone());
        std::thread::spawn(move || {
            for s in l.incoming().flatten() {
                if e.load(Ordering::SeqCst) {
                    return;
                }
                let (a, sk) = (a.clone(), sk.clone());
                std::thread::spawn(move || {
                    let mut s = s;
                    let Some(anfrage) = lies_anfrage(&mut s) else {
                        return;
                    };
                    a.lock().unwrap().push(anfrage);
                    let aktion = sk.lock().unwrap().pop_front().unwrap_or(Aktion::Schliessen);
                    handle(s, aktion);
                });
            }
        });
        Self {
            port,
            anfragen,
            ende,
        }
    }

    /// `http://127.0.0.1:<port><pfad>`
    pub fn basis(&self, pfad: &str) -> String {
        format!("http://127.0.0.1:{}{pfad}", self.port)
    }

    pub fn anfragen(&self) -> Vec<Anfrage> {
        self.anfragen.lock().unwrap().clone()
    }
}

/// Eine vollstaendige Antwort mit Content-Length.
pub fn antwort(status: u16, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}
