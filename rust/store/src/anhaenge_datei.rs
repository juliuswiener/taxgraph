//! Gemeinsamer Schreib-Helfer fuer `audit.rs`/`fehler_log.rs`: append-only, Modus 0600 bei
//! Neuanlage, niemals anders (`audit.py:60-74`, `fehler_log.py:146-154`). Beide Protokolle fuehren
//! Nutzer-/Fall-Kennungen; ohne das explizite Anlegen erbt die Datei die umask (gemessen 0644,
//! Audit `sec-users-json-world-readable`).
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

/// Oeffnet `pfad` zum Anhaengen; legt ihn bei Neuanlage mit Modus 0600 an
/// (`os.open(pfad, O_WRONLY|O_APPEND|O_CREAT, 0o600)`). Der Modus einer bestehenden Datei bleibt
/// unangetastet — `OpenOptionsExt::mode` wirkt nur bei tatsaechlicher Neuanlage.
fn oeffne_zum_anhaengen(pfad: &Path) -> io::Result<File> {
    if let Some(eltern) = pfad.parent() {
        if !eltern.as_os_str().is_empty() {
            std::fs::create_dir_all(eltern)?;
        }
    }
    OpenOptions::new().append(true).create(true).mode(0o600).open(pfad)
}

/// Haengt `zeile` (ohne eigenen Zeilenumbruch) an und fsynct danach
/// (`audit.py:71-74`: `f.write(line); f.flush(); os.fsync(f.fileno())`).
pub(crate) fn haenge_zeile_an(pfad: &Path, zeile: &str) -> io::Result<()> {
    let mut f = oeffne_zum_anhaengen(pfad)?;
    writeln!(f, "{zeile}")?;
    f.flush()?;
    f.sync_all()
}

#[cfg(test)]
mod tests {
    use super::haenge_zeile_an;

    #[test]
    fn haengt_zwei_zeilen_an_und_setzt_modus_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("taxgraph-store-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("anhaengen.jsonl");
        let _ = std::fs::remove_file(&pfad);
        haenge_zeile_an(&pfad, "eins").unwrap();
        haenge_zeile_an(&pfad, "zwei").unwrap();
        let inhalt = std::fs::read_to_string(&pfad).unwrap();
        assert_eq!(inhalt, "eins\nzwei\n");
        let modus = std::fs::metadata(&pfad).unwrap().permissions().mode() & 0o777;
        assert_eq!(modus, 0o600);
        std::fs::remove_dir_all(&dir).ok();
    }
}
