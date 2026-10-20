//! Die fremden Programme des Crates (`curl`, `gzip -dc`, `pdftotext`) laufen alle durch `unterprozess::lauf`. Hier: Ausgabe,
//! Eingabe über stdin ohne Verklemmung, Zeitlimit mit `kill` (das Kind ist danach weg), ein fehlendes Programm, und `hole` /
//! `text_aus_antwort` mit falschen `curl` und `gzip` (kleine Skripte in einem Temp-Ordner, kein Netz).
//!
//! HERMETISCH: die falschen Programme sind Skripte mit `#!/bin/sh`. Wer eine Skriptdatei schreibt und gleich startet, kann in
//! einem Prozess mit mehreren Fäden auf `ETXTBSY` stoßen, wenn ein anderer Faden gerade forkt; `cargo nextest` (Standard in
//! diesem Projekt, ein Prozess je Test) hat das Problem nicht; unter `cargo test` fängt `skript` es mit einer Probe ab.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::explicit_write)]

use std::ffi::OsStr;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use quellen::einfrieren::{hole, text_aus_antwort, Werkzeuge};
use quellen::unterprozess::{lauf, UnterprozessFehler};
use quellen::Fehler;

/// Schreibt ein Skript, macht es ausführbar und startet es einmal zur Probe (`QUELLEN_PROBELAUF` beendet es sofort), bis das
/// Betriebssystem kein `ETXTBSY` mehr meldet: ein Faden eines laufenden `cargo test` kann gerade geforkt haben, mit der
/// Schreib-Kennung der Datei im Kind. Nach einer geglückten Probe hält niemand sie mehr offen.
pub fn skript(dir: &Path, name: &str, inhalt: &str) -> PathBuf {
    let pfad = dir.join(name);
    fs::write(&pfad, format!("#!/bin/sh\n[ -n \"$QUELLEN_PROBELAUF\" ] && exit 0\n{inhalt}\n")).unwrap();
    fs::set_permissions(&pfad, fs::Permissions::from_mode(0o755)).unwrap();
    for _ in 0..100 {
        match std::process::Command::new(&pfad).env("QUELLEN_PROBELAUF", "1").output() {
            Ok(_) => return pfad,
            Err(e) if e.raw_os_error() == Some(26) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("{}: {e}", pfad.display()),
        }
    }
    panic!("{}: Text file busy bis zuletzt", pfad.display());
}

fn os(s: &str) -> &OsStr {
    OsStr::new(s)
}

#[test]
fn die_ausgabe_beider_rohre_und_der_status_kommen_zurueck() {
    let t = tempfile::tempdir().unwrap();
    let p = skript(t.path(), "tool", "echo aus; echo fehl >&2; exit 3");
    let a = lauf(p.to_str().unwrap(), &[], None, Duration::from_secs(10)).unwrap();
    assert_eq!((a.stdout, a.stderr, a.status.code()), (b"aus\n".to_vec(), b"fehl\n".to_vec(), Some(3)));
}

#[test]
fn eine_grosse_eingabe_geht_hin_und_die_gleiche_grosse_ausgabe_zurueck_ohne_verklemmung() {
    // 4 MiB über `cat`: ein Kind, das erst alles liest, und ein Elternteil, das erst alles schreibt, klemmten sich sonst
    // bei vollem Rohr gegenseitig fest.
    let eingabe: Vec<u8> = (0..4_000_000u32).map(|i| u8::try_from(i % 251).unwrap()).collect();
    let a = lauf("cat", &[], Some(eingabe.clone()), Duration::from_secs(20)).unwrap();
    assert!(a.status.success());
    assert!(a.stdout == eingabe, "{} Bytes zurueck, {} hin", a.stdout.len(), eingabe.len());
}

#[test]
fn nach_dem_zeitlimit_ist_das_kind_beendet_und_abgeholt() {
    let t = tempfile::tempdir().unwrap();
    let pidfile = t.path().join("pid");
    // `exec`: die Shell wird zu `sleep`, `$$` bleibt die PID des Kindes, `kill` trifft das schlafende Programm selbst
    let p = skript(t.path(), "haengt", &format!("echo $$ > '{}'\nexec sleep 30", pidfile.display()));
    let start = Instant::now();
    let fehler = lauf(p.to_str().unwrap(), &[], None, Duration::from_secs(1)).unwrap_err();
    assert!(matches!(fehler, UnterprozessFehler::Zeitlimit { sekunden: 1, .. }), "{fehler:?}");
    assert!(start.elapsed() < Duration::from_secs(10), "{:?}", start.elapsed());
    let pid = fs::read_to_string(&pidfile).unwrap().trim().to_owned();
    assert_ne!(pid, "", "die PID steht in der Datei");
    // abgeholt (`wait` nach `kill`): kein Eintrag mehr unter /proc, auch kein Zombie
    assert!(!Path::new(&format!("/proc/{pid}")).exists(), "Kind {pid} lebt noch oder ist ein Zombie");
}

#[test]
fn ein_fehlendes_programm_ist_ein_fehler_mit_dem_namen() {
    let fehler = lauf("programm-das-es-nicht-gibt-quellen", &[], None, Duration::from_secs(5)).unwrap_err();
    assert!(matches!(&fehler, UnterprozessFehler::Start { programm } if programm == "programm-das-es-nicht-gibt-quellen"));
    assert!(fehler.to_string().contains("programm-das-es-nicht-gibt-quellen nicht gefunden"), "{fehler}");
}

#[test]
fn file_urls_lesen_die_datei_auch_mit_localhost_und_prozentzeichen() {
    let t = tempfile::tempdir().unwrap();
    let datei = t.path().join("ein ordner").join("a%b.html");
    fs::create_dir_all(datei.parent().unwrap()).unwrap();
    fs::write(&datei, b"<p>inhalt</p>").unwrap();
    let w = Werkzeuge::default();
    let pfad = datei.display().to_string();
    let kodiert = pfad.replace(' ', "%20").replace("a%b", "a%25b");
    assert_eq!(hole(&format!("file://{kodiert}"), &w).unwrap(), b"<p>inhalt</p>");
    assert_eq!(hole(&format!("file://localhost{kodiert}"), &w).unwrap(), b"<p>inhalt</p>");
}

#[test]
fn file_urls_ohne_absoluten_pfad_und_fehlende_dateien_und_fremde_schemata_sind_abruf_fehler() {
    let w = Werkzeuge::default();
    for url in ["file://relativ/x.html", "file:///nicht/da/x.html", "ftp://example.invalid/x", "gopher://x", "x.html", "javascript:alert(1)"] {
        let e = hole(url, &w).unwrap_err();
        assert!(matches!(&e, Fehler::Abruf { url: u, .. } if u == url), "{url}: {e:?}");
    }
    let e = hole("ftp://example.invalid/x", &w).unwrap_err().to_string();
    assert!(e.contains("nur http://, https:// und file:// sind erlaubt"), "{e}");
}

#[test]
fn http_urls_gehen_ueber_curl_mit_festen_optionen_und_die_url_steht_hinter_url() {
    let t = tempfile::tempdir().unwrap();
    let args = t.path().join("args");
    let curl = skript(t.path(), "curl", &format!("printf '%s\\n' \"$@\" > '{}'\nprintf 'antwort'", args.display()));
    let w = Werkzeuge { curl: curl.display().to_string(), ..Werkzeuge::default() };
    let url = "https://www.example.invalid/a?b=1&c=--url";
    assert_eq!(hole(url, &w).unwrap(), b"antwort");
    let ruf: Vec<String> = fs::read_to_string(&args).unwrap().lines().map(str::to_owned).collect();
    // -f: Fehlerstatus ab 400 ist ein Fehler; -L: Weiterleitungen folgen; --proto: nur http und https, auch nach einer
    // Weiterleitung nicht file:// oder ftp://; die URL steht hinter --url, nie als freies Argument (kein Optionseinschub)
    assert_eq!(&ruf[..9], ["-sS", "-f", "-L", "--proto", "=http,https", "--max-time", "180", "-A", "Mozilla/5.0"]);
    assert_eq!(&ruf[9..], ["--url", url]);
}

#[test]
fn ein_curl_fehler_nennt_die_url_und_den_grund() {
    let t = tempfile::tempdir().unwrap();
    let curl = skript(t.path(), "curl", "echo 'curl: (22) The requested URL returned error: 404' >&2\nexit 22");
    let w = Werkzeuge { curl: curl.display().to_string(), ..Werkzeuge::default() };
    let e = hole("https://www.example.invalid/weg", &w).unwrap_err().to_string();
    assert!(e.contains("https://www.example.invalid/weg") && e.contains("returned error: 404"), "{e}");
}

#[test]
fn ein_fehlendes_curl_ist_ein_fehler_mit_dem_namen() {
    let w = Werkzeuge { curl: "curl-das-es-nicht-gibt".into(), ..Werkzeuge::default() };
    let e = hole("https://www.example.invalid/x", &w).unwrap_err();
    assert!(matches!(&e, Fehler::Unterprozess(UnterprozessFehler::Start { programm }) if programm == "curl-das-es-nicht-gibt"), "{e:?}");
}

fn gzip_da() -> bool {
    std::process::Command::new("gzip").arg("--version").output().is_ok_and(|o| o.status.success())
}

#[test]
fn rohes_gzip_wird_entpackt_und_text_ohne_magic_nicht_angefasst() {
    if !gzip_da() {
        writeln!(std::io::stderr(), "UEBERSPRUNGEN: gzip fehlt").unwrap();
        return;
    }
    let w = Werkzeuge::default();
    let gz = lauf("gzip", &[os("-c")], Some("Grüße § 9".as_bytes().to_vec()), Duration::from_secs(10)).unwrap().stdout;
    assert_eq!(&gz[..2], [0x1f, 0x8b]);
    assert_eq!(text_aus_antwort(gz, &w).unwrap(), "Grüße § 9");
    assert_eq!(text_aus_antwort(b"<p>klartext</p>".to_vec(), &w).unwrap(), "<p>klartext</p>");
    // nur das erste Byte stimmt: kein gzip
    assert_eq!(text_aus_antwort(vec![0x1f, b'x'], &w).unwrap(), "\u{1f}x");
}

#[test]
fn gzip_mit_magic_aber_kaputtem_inhalt_ist_ein_fehler() {
    if !gzip_da() {
        writeln!(std::io::stderr(), "UEBERSPRUNGEN: gzip fehlt").unwrap();
        return;
    }
    let e = text_aus_antwort(vec![0x1f, 0x8b, 0x08, 0x00, 1, 2, 3, 4, 5, 6, 7, 8, 9], &Werkzeuge::default()).unwrap_err();
    assert!(matches!(e, Fehler::Entpacken { .. }), "{e:?}");
}

#[test]
fn ein_fehlendes_gzip_ist_ein_fehler_mit_dem_namen() {
    let w = Werkzeuge { gzip: "gzip-das-es-nicht-gibt".into(), ..Werkzeuge::default() };
    let e = text_aus_antwort(vec![0x1f, 0x8b, 0, 0], &w).unwrap_err();
    assert!(matches!(&e, Fehler::Unterprozess(UnterprozessFehler::Start { programm }) if programm == "gzip-das-es-nicht-gibt"), "{e:?}");
}
