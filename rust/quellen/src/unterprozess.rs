//! Der einzige Ort, an dem das Crate ein fremdes Programm startet (`curl`, `gzip -dc`, `pdftotext`).
//!
//! Jedes `Command::new` unter `rust/*/src` braucht `wait_timeout` und `kill` in derselben Funktion (Wächter
//! `rust/eingang/tests/unterprozess_zeitlimit.rs`). Darum gibt es genau eine Funktion, [`lauf`]; alle drei Programme gehen
//! durch sie. Nach dem Zeitlimit wird das Kind beendet und abgeholt, nicht nur aufgegeben.

use std::ffi::OsStr;
use std::io::{Read, Write};
use std::process::{Command, ExitStatus, Stdio};
use std::thread::JoinHandle;
use std::time::Duration;

use wait_timeout::ChildExt;

/// Was beim Start oder Warten auf ein fremdes Programm schiefgehen kann.
#[derive(Debug, thiserror::Error)]
pub enum UnterprozessFehler {
    /// Das Programm ist nicht aufrufbar (nicht installiert, nicht im `PATH`).
    #[error("{programm} nicht gefunden (installieren oder in den PATH legen)")]
    Start { programm: String },
    /// Das Programm hat das Zeitlimit überschritten und wurde beendet.
    #[error("{programm} hat das Zeitlimit von {sekunden} s überschritten und wurde beendet")]
    Zeitlimit { programm: String, sekunden: u64 },
    /// Ein- oder Ausgabe zum Kind ist gescheitert.
    #[error("Ein-/Ausgabe zu {programm}: {quelle}")]
    Io {
        programm: String,
        #[source]
        quelle: std::io::Error,
    },
}

/// Ende eines Laufs, der nicht am Zeitlimit gescheitert ist.
#[derive(Debug)]
pub struct Ausgabe {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Liest ein Rohr im eigenen Faden bis zum Ende. Ohne den Faden bliebe ein volles Rohr beim Kind hängen.
fn lies_faden<R: Read + Send + 'static>(rohr: Option<R>) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut puffer = Vec::new();
        if let Some(mut r) = rohr {
            let _ = r.read_to_end(&mut puffer);
        }
        puffer
    })
}

/// Startet `programm args…`, schreibt `eingabe` auf stdin (wenn es eine gibt), sammelt stdout und stderr und wartet
/// höchstens `limit`. Danach: `kill` und `wait`, Fehler `Zeitlimit`.
///
/// # Errors
/// [`UnterprozessFehler`]: Programm fehlt, Zeitlimit, Ein-/Ausgabefehler beim Warten.
pub fn lauf(
    programm: &str,
    args: &[&OsStr],
    eingabe: Option<Vec<u8>>,
    limit: Duration,
) -> Result<Ausgabe, UnterprozessFehler> {
    let io = |quelle| UnterprozessFehler::Io { programm: programm.to_owned(), quelle };
    let mut kind = Command::new(programm)
        .args(args)
        .stdin(if eingabe.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                UnterprozessFehler::Start { programm: programm.to_owned() }
            } else {
                io(e)
            }
        })?;
    if let (Some(daten), Some(mut stdin)) = (eingabe, kind.stdin.take()) {
        // Im eigenen Faden: ein Kind, das erst liest, wenn sein Ausgaberohr leer ist, bräche sonst in ein Rohr zu.
        // Ein Schreibfehler (Kind beendet früh) zeigt sich am Exit-Status, nicht hier.
        std::thread::spawn(move || {
            let _ = stdin.write_all(&daten);
        });
    }
    let aus = lies_faden(kind.stdout.take());
    let fehl = lies_faden(kind.stderr.take());
    let Some(status) = kind.wait_timeout(limit).map_err(io)? else {
        // Die Lesefäden werden nicht abgewartet: ein Enkel, der das Rohr offen hält, hielte uns sonst über das Limit hinaus.
        let _ = kind.kill();
        let _ = kind.wait();
        return Err(UnterprozessFehler::Zeitlimit { programm: programm.to_owned(), sekunden: limit.as_secs() });
    };
    Ok(Ausgabe {
        status,
        stdout: aus.join().unwrap_or_default(),
        stderr: fehl.join().unwrap_or_default(),
    })
}
