//! `scripts/verify_sources.py` in Rust (`quellen::pruefen::pruefe`): dieselben Urteile und dieselben Ausgabezeilen.
//!
//! HERMETISCH: jeder Test baut einen eigenen kleinen `sources/`-Baum in einem Temp-Ordner. Der Test am echten Baum
//! (`der_echte_baum_ist_ganz_ok`) liest nur. Die Mutationen des Auftrags (ein Byte geändert, Wortlaut gekürzt, `datei:`
//! fehlt, `sha256` fehlt) laufen an einer Kopie einer echten Quelle (`mutationen_an_einer_kopie_einer_echten_quelle`);
//! der Vergleich mit dem Python-Skript auf dem echten Baum steht im Bericht des Auftrags.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::explicit_write)]

use std::fs;
use std::path::{Path, PathBuf};

use quellen::pruefen::{pruefe, Bericht};
use sha2::{Digest, Sha256};

const MARKE: &str = "--- Wortlaut (abgerufener Ausschnitt) ---\n\n";

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Ein Wortlaut aus `n` Zeichen `x`.
fn wortlaut(n: usize) -> String {
    "x".repeat(n)
}

/// Legt `<wurzel>/sources/<unter>/<name>.txt` und `.meta.yaml` an; die Metadaten tragen den echten `sha256` des Textes.
fn quelle(wurzel: &Path, unter: &str, name: &str, text: &str) {
    let dir = wurzel.join("sources").join(unter);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(format!("{name}.txt")), text).unwrap();
    let meta = format!("dokument:\n  norm: \"N\"\n  datei: \"{name}.txt\"\n  sha256: \"{}\"\n", sha(text.as_bytes()));
    fs::write(dir.join(format!("{name}.meta.yaml")), meta).unwrap();
}

/// Ein Text, dessen Wortlaut nach der Marke `--- Wortlaut` (ohne umgebenden Leerraum) genau aus `zeichen` besteht.
fn mit_wortlaut(zeichen: &str) -> String {
    format!("Kopf\n--- Wortlaut {zeichen}\n")
}

fn mit_kopf(wortlaut: &str) -> String {
    format!("Quelle: u\nAbgerufen: d\n\n{MARKE}{wortlaut}\n")
}

fn lauf(wurzel: &Path) -> Bericht {
    pruefe(wurzel).unwrap()
}

#[test]
fn ein_unversehrter_baum_ist_ok_und_jede_quelle_hat_eine_zeile() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "eins", &mit_kopf(&wortlaut(80)));
    quelle(t.path(), "b", "zwei", &mit_kopf(&wortlaut(80)));
    let b = lauf(t.path());
    assert_eq!(
        b,
        Bericht {
            zeilen: vec!["OK       sources/a/eins.txt".into(), "OK       sources/b/zwei.txt".into()],
            ok: true
        }
    );
}

#[test]
fn ein_geaendertes_byte_ist_mismatch_und_die_naechste_quelle_wird_trotzdem_geprueft() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "eins", &mit_kopf(&wortlaut(80)));
    quelle(t.path(), "b", "zwei", &mit_kopf(&wortlaut(80)));
    let pfad = t.path().join("sources/a/eins.txt");
    let mut bytes = fs::read(&pfad).unwrap();
    let letzte = bytes.len() - 2;
    bytes[letzte] = b'y';
    fs::write(&pfad, bytes).unwrap();
    let b = lauf(t.path());
    assert_eq!(b.zeilen, vec!["MISMATCH sources/a/eins.txt", "OK       sources/b/zwei.txt"]);
    assert!(!b.ok);
}

#[test]
fn der_wortlaut_zaehlt_ab_60_zeichen_und_in_zeichen_nicht_in_bytes() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "knapp", &mit_wortlaut(&wortlaut(59)));
    quelle(t.path(), "a", "genau", &mit_wortlaut(&wortlaut(60)));
    quelle(t.path(), "a", "umlaut59", &mit_wortlaut(&"ä".repeat(59)));
    quelle(t.path(), "a", "umlaut60", &mit_wortlaut(&"ä".repeat(60)));
    let b = lauf(t.path());
    assert_eq!(
        b.zeilen,
        vec![
            "OK       sources/a/genau.txt",
            "LEER     sources/a/knapp.txt (Wortlaut nur 59 Zeichen)",
            "LEER     sources/a/umlaut59.txt (Wortlaut nur 59 Zeichen)",
            "OK       sources/a/umlaut60.txt",
        ]
    );
    assert!(!b.ok);
}

#[test]
fn der_wortlaut_wird_ohne_umgebenden_leerraum_gezaehlt_auch_u_001c() {
    let t = tempfile::tempdir().unwrap();
    // 59 Zeichen, umgeben von Leerraum, den Python strip() entfernt (auch U+001C bis U+001F, U+00A0).
    quelle(t.path(), "a", "leer", &format!("--- Wortlaut\x1c \u{a0}\n{}\n\x1f\t ", wortlaut(59)));
    let b = lauf(t.path());
    assert_eq!(b.zeilen, vec!["LEER     sources/a/leer.txt (Wortlaut nur 59 Zeichen)"]);
}

#[test]
fn ohne_marke_zaehlt_die_ganze_datei_und_nur_die_erste_marke_teilt() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "ohne60", &wortlaut(60));
    quelle(t.path(), "a", "ohne59", &wortlaut(59));
    // Zwei Marken: nach der ERSTEN zählt alles (20 + 12 + 30 = 62 Zeichen); nach der letzten wären es 30 (LEER).
    quelle(t.path(), "a", "zweimal", &format!("--- Wortlaut{}--- Wortlaut{}", "a".repeat(20), "b".repeat(30)));
    let b = lauf(t.path());
    assert_eq!(
        b.zeilen,
        vec![
            "LEER     sources/a/ohne59.txt (Wortlaut nur 59 Zeichen)",
            "OK       sources/a/ohne60.txt",
            "OK       sources/a/zweimal.txt"
        ]
    );
}

#[test]
fn zeilenenden_werden_wie_in_python_uebersetzt_und_ungueltiges_utf8_zaehlt_als_ersatzzeichen() {
    let t = tempfile::tempdir().unwrap();
    // open(..).read() macht aus "\r\n" und "\r" je ein "\n": 30 Zeilen "x\r\n" sind 30 x + 29 \n = 59 Zeichen, nicht 90.
    quelle(t.path(), "a", "crlf30", &format!("--- Wortlaut {}", "x\r\n".repeat(30)));
    quelle(t.path(), "a", "crlf60", &format!("--- Wortlaut {}", "x\r\n".repeat(60)));
    quelle(t.path(), "a", "cr59", &format!("--- Wortlaut {}", "x\r".repeat(59)));
    // ungültige Bytes werden Ersatzzeichen (errors="replace") und zählen mit: 40 + 2 + 30
    let dir = t.path().join("sources/a");
    let roh = [b"--- Wortlaut ".as_slice(), &[b'x'; 40], &[0xff, 0xfe], &[b'y'; 30]].concat();
    fs::write(dir.join("roh.txt"), &roh).unwrap();
    fs::write(dir.join("roh.meta.yaml"), format!("dokument:\n  datei: \"roh.txt\"\n  sha256: \"{}\"\n", sha(&roh))).unwrap();
    assert_eq!(
        lauf(t.path()).zeilen,
        vec![
            "OK       sources/a/cr59.txt",
            "LEER     sources/a/crlf30.txt (Wortlaut nur 59 Zeichen)",
            "OK       sources/a/crlf60.txt",
            "OK       sources/a/roh.txt"
        ]
    );
}

#[test]
fn eine_metadatei_ohne_gueltiges_utf8_ist_ein_fehler_und_die_zeilen_davor_stehen_schon_da() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "eins", &mit_kopf(&wortlaut(80)));
    fs::write(t.path().join("sources/b.meta.yaml"), [0xffu8, 0xfe, b'\n']).unwrap();
    let mut gesehen = Vec::new();
    let r = quellen::pruefen::pruefe_zeilenweise(t.path(), &mut |z| gesehen.push(z.to_owned()));
    assert!(r.is_err(), "Python bricht mit UnicodeDecodeError ab");
    assert_eq!(gesehen, vec!["OK       sources/a/eins.txt"]);
}

#[test]
fn eine_fehlende_datei_ist_missing_file_mit_zwei_leerzeichen() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "da", &mit_kopf(&wortlaut(80)));
    quelle(t.path(), "a", "weg", &mit_kopf(&wortlaut(80)));
    fs::remove_file(t.path().join("sources/a/weg.txt")).unwrap();
    let b = lauf(t.path());
    assert_eq!(
        b.zeilen,
        vec!["OK       sources/a/da.txt", "MISSING FILE:  weg.txt (referenced by sources/a/weg.meta.yaml)"]
    );
    assert!(!b.ok);
}

#[test]
fn fehlt_datei_oder_sha256_in_der_meta_ist_es_missing_fields() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "ohnesha", &mit_kopf(&wortlaut(80)));
    quelle(t.path(), "a", "ohnedatei", &mit_kopf(&wortlaut(80)));
    fs::write(t.path().join("sources/a/ohnesha.meta.yaml"), "dokument:\n  datei: \"ohnesha.txt\"\n").unwrap();
    fs::write(t.path().join("sources/a/ohnedatei.meta.yaml"), "dokument:\n  sha256: \"abc\"\n").unwrap();
    let b = lauf(t.path());
    assert_eq!(
        b.zeilen,
        vec!["MISSING FIELDS: sources/a/ohnedatei.meta.yaml", "MISSING FIELDS: sources/a/ohnesha.meta.yaml"]
    );
    assert!(!b.ok);
}

#[test]
fn pdf_sha256_und_pdf_datei_werden_nicht_mit_sha256_und_datei_verwechselt() {
    let t = tempfile::tempdir().unwrap();
    let text = mit_kopf(&wortlaut(80));
    quelle(t.path(), "a", "doc", &text);
    // pdf_* steht VOR den gesuchten Feldern: das Skript sucht ab Zeilenanfang und nimmt nur `datei:` und `sha256:`.
    let meta = format!(
        "dokument:\n  pdf_datei: \"doc.pdf\"\n  pdf_sha256: \"falsch\"\n  datei: \"doc.txt\"\n  sha256: \"{}\"\n",
        sha(text.as_bytes())
    );
    fs::write(t.path().join("sources/a/doc.meta.yaml"), meta).unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec!["OK       sources/a/doc.txt"]);
}

#[test]
fn werte_mit_und_ohne_anfuehrungszeichen_und_mit_leerraum_am_rand() {
    let t = tempfile::tempdir().unwrap();
    let text = mit_kopf(&wortlaut(80));
    quelle(t.path(), "a", "doc", &text);
    let s = sha(text.as_bytes());
    fs::write(t.path().join("sources/a/doc.meta.yaml"), format!("dokument:\n    datei:   doc.txt   \n\tsha256:\t\"{s}\"  \n")).unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec!["OK       sources/a/doc.txt"]);
}

#[test]
fn eine_meta_mit_nur_cr_als_zeilenende_wird_wie_in_python_gelesen() {
    let t = tempfile::tempdir().unwrap();
    let text = mit_kopf(&wortlaut(80));
    quelle(t.path(), "a", "doc", &text);
    // Python macht aus jedem alleinstehenden "\r" ein "\n"; ohne die Übersetzung stünde alles in EINER Zeile.
    let meta = format!("dokument:\r  datei: \"doc.txt\"\r  sha256: \"{}\"\r", sha(text.as_bytes()));
    fs::write(t.path().join("sources/a/doc.meta.yaml"), meta).unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec!["OK       sources/a/doc.txt"]);
}

#[test]
fn ein_feld_aus_nur_leerzeichen_gilt_als_fehlend() {
    let t = tempfile::tempdir().unwrap();
    let text = mit_kopf(&wortlaut(80));
    quelle(t.path(), "a", "doc", &text);
    // `field()` liefert " " und strip() macht "" daraus: `not datei` ist wahr.
    let meta = format!("dokument:\n  datei:   \n  sha256: \"{}\"\n", sha(text.as_bytes()));
    fs::write(t.path().join("sources/a/doc.meta.yaml"), meta).unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec!["MISSING FIELDS: sources/a/doc.meta.yaml"]);
}

#[test]
fn die_reihenfolge_ist_die_der_pfadtexte_nicht_der_pfadteile() {
    let t = tempfile::tempdir().unwrap();
    // "a.meta.yaml" < "a/x.meta.yaml", weil '.' (0x2E) vor '/' (0x2F) kommt; nach Pfadteilen käme "a" vor "a.meta.yaml".
    // Und "B" (0x42) vor "a" (0x61).
    quelle(t.path(), "", "a", &mit_kopf(&wortlaut(80)));
    quelle(t.path(), "a", "x", &mit_kopf(&wortlaut(80)));
    quelle(t.path(), "B", "y", &mit_kopf(&wortlaut(80)));
    let b = lauf(t.path());
    assert_eq!(
        b.zeilen,
        vec!["OK       sources/B/y.txt", "OK       sources/a.txt", "OK       sources/a/x.txt"]
    );
}

#[test]
fn versteckte_ordner_und_dateien_zaehlen_nicht() {
    let t = tempfile::tempdir().unwrap();
    quelle(t.path(), "a", "sichtbar", &mit_kopf(&wortlaut(80)));
    quelle(t.path(), ".versteckt", "x", &mit_kopf(&wortlaut(10)));
    let dir = t.path().join("sources/a");
    fs::write(dir.join(".dot.meta.yaml"), "dokument:\n").unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec!["OK       sources/a/sichtbar.txt"]);
}

#[test]
fn ohne_jede_metadatei_ist_das_urteil_rot_mit_einer_meldung() {
    let t = tempfile::tempdir().unwrap();
    fs::create_dir_all(t.path().join("sources")).unwrap();
    assert_eq!(lauf(t.path()), Bericht { zeilen: vec!["no source metadata found".into()], ok: false });
    // auch ohne den Ordner sources/
    let leer = tempfile::tempdir().unwrap();
    assert_eq!(lauf(leer.path()), Bericht { zeilen: vec!["no source metadata found".into()], ok: false });
}

fn echte_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Alle `*.meta.yaml` unter `sources/`, unabhängig von `pruefe` gezählt (rekursiv, ohne versteckte Namen).
fn zaehle_metas(dir: &Path) -> usize {
    let mut n = 0;
    for e in fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if e.path().is_dir() {
            n += zaehle_metas(&e.path());
        } else if name.ends_with(".meta.yaml") {
            n += 1;
        }
    }
    n
}

#[test]
fn der_echte_baum_ist_ganz_ok() {
    let wurzel = echte_wurzel();
    let erwartet = zaehle_metas(&wurzel.join("sources"));
    assert!(erwartet > 100, "der Zähler sieht den echten Baum nicht: {erwartet}");
    let b = lauf(&wurzel);
    assert_eq!(b.zeilen.len(), erwartet);
    let nicht_ok: Vec<&String> = b.zeilen.iter().filter(|z| !z.starts_with("OK       sources/")).collect();
    assert_eq!(nicht_ok, Vec::<&String>::new());
    assert!(b.ok);
}

/// Kopiert die erste echte Quelle mit Wortlaut in einen Temp-Baum und gibt (Wurzel, Meta, Text) zurück.
fn kopie_einer_echten_quelle() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let echt = echte_wurzel().join("sources/gesetze-im-internet");
    let mut metas: Vec<PathBuf> = fs::read_dir(&echt)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".meta.yaml"))
        .collect();
    metas.sort();
    let meta = metas.first().unwrap();
    let txt = meta.with_file_name(
        fs::read_to_string(meta)
            .unwrap()
            .lines()
            .find_map(|l| l.trim().strip_prefix("datei:").map(|v| v.trim().trim_matches('"').to_owned()))
            .unwrap(),
    );
    let t = tempfile::tempdir().unwrap();
    let ziel = t.path().join("sources/gesetze-im-internet");
    fs::create_dir_all(&ziel).unwrap();
    let m = ziel.join(meta.file_name().unwrap());
    let x = ziel.join(txt.file_name().unwrap());
    fs::copy(meta, &m).unwrap();
    fs::copy(&txt, &x).unwrap();
    (t, m, x)
}

/// Der Text ohne die Zeilen, die (ohne Randleerraum) mit `praefix` beginnen.
fn ohne_zeile(text: &str, praefix: &str) -> String {
    let mut aus = String::new();
    for zeile in text.lines().filter(|l| !l.trim().starts_with(praefix)) {
        aus.push_str(zeile);
        aus.push('\n');
    }
    aus
}

#[test]
fn mutationen_an_einer_kopie_einer_echten_quelle() {
    let (t, meta, txt) = kopie_einer_echten_quelle();
    let rel = format!("sources/gesetze-im-internet/{}", txt.file_name().unwrap().to_string_lossy());
    assert_eq!(lauf(t.path()).zeilen, vec![format!("OK       {rel}")], "Kontrolle: die Kopie ist unversehrt");

    // 1. ein Byte der Quelle geändert -> MISMATCH
    let original = fs::read(&txt).unwrap();
    let mut geaendert = original.clone();
    let mitte = geaendert.len() / 2;
    geaendert[mitte] ^= 0x01;
    fs::write(&txt, &geaendert).unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec![format!("MISMATCH {rel}")]);

    // 2. auf unter 60 Zeichen Wortlaut gekürzt (sha256 der Meta nachgezogen, sonst käme MISMATCH) -> LEER
    let kurz = "--- Wortlaut nur kurz\n".to_owned();
    fs::write(&txt, &kurz).unwrap();
    let meta_text = fs::read_to_string(&meta).unwrap();
    let alt = meta_text.lines().find(|l| l.trim().starts_with("sha256:")).unwrap().to_owned();
    fs::write(&meta, meta_text.replace(&alt, &format!("  sha256: \"{}\"", sha(kurz.as_bytes())))).unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec![format!("LEER     {rel} (Wortlaut nur 8 Zeichen)")]);

    // 3. Quelle entfernt -> MISSING FILE
    fs::remove_file(&txt).unwrap();
    let z = lauf(t.path()).zeilen;
    assert_eq!(z.len(), 1);
    assert!(z[0].starts_with("MISSING FILE:  ") && z[0].contains("(referenced by sources/gesetze-im-internet/"), "{z:?}");

    // 4. `datei:` fehlt -> MISSING FIELDS
    let ohne_datei = ohne_zeile(&meta_text, "datei:");
    fs::write(&meta, ohne_datei).unwrap();
    assert_eq!(lauf(t.path()).zeilen, vec![format!("MISSING FIELDS: sources/gesetze-im-internet/{}", meta.file_name().unwrap().to_string_lossy())]);

    // 5. `sha256` fehlt -> MISSING FIELDS
    let ohne_sha = ohne_zeile(&meta_text, "sha256:");
    fs::write(&meta, ohne_sha).unwrap();
    assert!(lauf(t.path()).zeilen[0].starts_with("MISSING FIELDS: "));
}
