//! Kein Unterprozess ohne Zeitlimit: jedes `Command::new` unter `rust/*/src` haelt sein Kind an einer
//! Grenze fest, oder es steht mit Grund in `AUSNAHMEN`.
//!
//! Gegenstueck zu `tests/test_unterprozess_zeitlimit.py` (`kein subprocess.run ohne timeout`). Der Dienst
//! bearbeitet Anfragen hintereinander (`Zustand.sperre`); ein `tesseract`, das auf einer hochgeladenen
//! Datei nicht zurueckkehrt, haelt dann nicht eine Anfrage auf, sondern alles. Die Grenze pro Aufruf
//! allein reicht nicht (500 Seiten x 60 s); der Seitendeckel dazu steht in `ocr_pfade.rs`. Hier geht es
//! nur um die Grenze: ein neuer Aufruf ohne sie macht diesen Test rot.
//!
//! Ein Aufruf HAT ein Zeitlimit, wenn die umschliessende Funktion `.wait_timeout(` UND `.kill(`
//! enthaelt (`wait_timeout::ChildExt`, danach das Kind beenden; ein Limit ohne Kill laesst das Kind
//! weiterlaufen). `.output()`, `.status()` und `wait_with_output()` warten ohne Grenze und zaehlen nicht.
//! Die Hoehe des Limits prueft dieser Test nicht: `eingang::ocr::lauf` nimmt es als `Duration`
//! (kein `Option`), die Hoehen pinnen `ocr_zeitlimit.rs` und `ocr_kill.rs` (beide `#[ignore]`).
//!
//! - Ein neues `Command::new` ohne Limit ist rot, bis es eines bekommt oder mit Grund in `AUSNAHMEN` steht.
//! - Eine Ausnahme ohne Aufruf ist rot (die Liste darf nur schrumpfen), ebenso eine, deren Funktion
//!   inzwischen ein Limit hat, und eine mit anderer Anzahl von Aufrufen.
//! - Jede Ausnahme gilt nur, solange kein Dienst-Pfad die Funktion ruft (`kein_aufrufer`): ausserhalb
//!   von `#[cfg(test)]` darf der Name nur als Definition oder als `pub use` vorkommen.
//! - Eine Verwendung von `Command` ausserhalb von `Command::new(` (Umbenennung mit `as`, Rueckgabetyp)
//!   sieht der Scanner nicht als Aufruf; sie ist deshalb selbst rot.
//!
//! Gescannt wird `src/**/*.rs` und `build.rs` jedes Crates unter `rust/`, ohne `parity`: das ist die
//! Test-Werkstatt (kein Crate haengt davon ab) und startet nur `python3` als Orakel; ihr Stillstand ist
//! ein haengender Testlauf, kein haengender Dienst. Die Aufrufe unter `rust/*/tests/` (`cc`, `make`,
//! `cargo`) sind ebenfalls nicht erfasst; dort setzt der CI-Job die Grenze.
//!
//! ponytail: Der Scanner kennt Kommentare, Zeichenketten (auch rohe) und Zeichenliterale, aber keine
//! Makros (`command!(..)`), keinen Funktionszeiger auf `Command::new` und keine anderen Wege zu einem
//! Kindprozess (`libc::fork`, `posix_spawn`, `tokio::process` unter anderem Namen). Das Crate-Verzeichnis
//! ist die Grenze der Suche; ein Crate mit anderem Pfad sieht er nicht. Upgrade: ein Lint ueber
//! `disallowed-methods` in `clippy.toml`, sobald der Workspace eines fuehrt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::many_single_char_names, // der Scanner arbeitet Zeichen fuer Zeichen (z, i, k, c, n)
    clippy::similar_names,
    clippy::too_many_lines
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Ein gefundenes `Command::new`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Fund {
    datei: String,
    funktion: String,
    zeile: usize,
    /// Die umschliessende Funktion enthaelt `.wait_timeout(` und `.kill(`.
    hat_limit: bool,
}

/// Ein `Command::new` ohne Zeitlimit, das es bleiben darf.
struct Ausnahme {
    /// Pfad relativ zu `rust/`.
    datei: &'static str,
    funktion: &'static str,
    /// Genau so viele `Command::new` stehen in dieser Funktion.
    anzahl: usize,
    grund: &'static str,
    /// Namen, die ausserhalb von `#[cfg(test)]` weder aufgerufen noch importiert werden duerfen.
    kein_aufrufer: &'static [&'static str],
}

const AUSNAHMEN: &[Ausnahme] = &[
    Ausnahme {
        datei: "elster/src/xmllint.rs",
        funktion: "validiere",
        anzahl: 1,
        grund: "`validiere_xsd` und `validiere_xsd_text` pruefen ein XML offline gegen das ERiC-Schema. \
                Nur Tests rufen sie (elster/tests, bescheid/tests/einreichung_e2e.rs, parity/tests); \
                kein Dienst-Pfad. Haengt `xmllint`, haengt der Test. Ruft ein Dienst-Pfad sie, braucht \
                diese Funktion `wait_timeout` und `kill` wie `eingang::ocr::lauf`.",
        kein_aufrufer: &["validiere_xsd", "validiere_xsd_text"],
    },
    Ausnahme {
        datei: "catala-sys/src/lib.rs",
        funktion: "recompute_source_hash",
        anzahl: 1,
        grund: "`sh -c 'find rules | sort | xargs cat | sha256sum'` ueber die eigenen `rules/*.catala_en`: \
                lokaler Lesezugriff, keine Nutzerdaten, kein Netz. Nur der Selbsttest von catala-sys und \
                ein Doctest rufen sie (`generated/SOURCE_HASH` gegen die Regeln); kein Dienst-Pfad. Ruft \
                ein Dienst-Pfad sie, braucht sie `wait_timeout` und `kill`.",
        kein_aufrufer: &["recompute_source_hash"],
    },
];

fn ist_kennung(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Ersetzt `von..bis` durch Leerzeichen; Zeilenumbrueche bleiben, damit Zeilennummern stimmen.
fn leere(aus: &mut [char], von: usize, bis: usize) {
    for c in &mut aus[von..bis] {
        if *c != '\n' {
            *c = ' ';
        }
    }
}

/// `r"`, `r#"`, `br##"` ...: Zahl der `#` und Index des ersten Zeichens im Text.
fn roh_beginn(z: &[char], i: usize) -> Option<(usize, usize)> {
    if i > 0 && ist_kennung(z[i - 1]) && !(z[i - 1] == 'b' && (i < 2 || !ist_kennung(z[i - 2]))) {
        return None;
    }
    let mut k = i + 1;
    let mut raten = 0;
    while z.get(k) == Some(&'#') {
        raten += 1;
        k += 1;
    }
    (z.get(k) == Some(&'"')).then_some((raten, k + 1))
}

/// Quelltext mit Leerzeichen statt Kommentaren, Zeichenketten-Inhalt und Zeichenliteralen. Laenge und
/// Zeilenumbrueche bleiben, damit ein Index im Ergebnis auch im Original stimmt.
fn maskiere(quelle: &str) -> Vec<char> {
    let z: Vec<char> = quelle.chars().collect();
    let mut aus = z.clone();
    let mut i = 0;
    while i < z.len() {
        let c = z[i];
        let n = z.get(i + 1).copied();
        let roh = if c == 'r' { roh_beginn(&z, i) } else { None };
        if c == '/' && n == Some('/') {
            let ende = (i..z.len()).find(|&k| z[k] == '\n').unwrap_or(z.len());
            leere(&mut aus, i, ende);
            i = ende;
        } else if c == '/' && n == Some('*') {
            let mut tiefe = 1;
            let mut k = i + 2;
            while k < z.len() && tiefe > 0 {
                if z[k] == '/' && z.get(k + 1) == Some(&'*') {
                    tiefe += 1;
                    k += 2;
                } else if z[k] == '*' && z.get(k + 1) == Some(&'/') {
                    tiefe -= 1;
                    k += 2;
                } else {
                    k += 1;
                }
            }
            leere(&mut aus, i, k.min(z.len()));
            i = k;
        } else if let Some((raten, start)) = roh {
            let mut k = start;
            while k < z.len() && !(z[k] == '"' && (1..=raten).all(|j| z.get(k + j) == Some(&'#'))) {
                k += 1;
            }
            leere(&mut aus, start, k.min(z.len()));
            i = k + 1 + raten;
        } else if c == '"' {
            let mut k = i + 1;
            while k < z.len() && z[k] != '"' {
                k += if z[k] == '\\' { 2 } else { 1 };
            }
            let k = k.min(z.len());
            leere(&mut aus, i + 1, k);
            i = k + 1;
        } else if c == '\'' {
            // Zeichenliteral (`'x'`, `'\n'`, `'"'`, `'\''`) oder Lebensdauer (`'a`).
            if n == Some('\\') {
                let ende = (i + 3..z.len()).find(|&k| z[k] == '\'').unwrap_or(z.len() - 1);
                leere(&mut aus, i + 1, ende);
                i = ende + 1;
            } else if z.get(i + 2) == Some(&'\'') {
                leere(&mut aus, i + 1, i + 2);
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    aus
}

/// Anfangsindizes von `wort` als ganzem Bezeichner.
fn wort_stellen(z: &[char], wort: &str) -> Vec<usize> {
    let w: Vec<char> = wort.chars().collect();
    (0..z.len())
        .filter(|&i| {
            z[i..].starts_with(&w)
                && (i == 0 || !ist_kennung(z[i - 1]))
                && !z.get(i + w.len()).is_some_and(|&c| ist_kennung(c))
        })
        .collect()
}

fn folge_stelle(z: &[char], folge: &str) -> Vec<usize> {
    let f: Vec<char> = folge.chars().collect();
    (0..z.len()).filter(|&i| z[i..].starts_with(&f)).collect()
}

/// Ab `von`: Index des ersten `{` oder `;` ausserhalb von Klammern und bei `{` der Index der passenden
/// `}`. `None`, wenn nichts davon kommt.
fn item_ab(z: &[char], von: usize) -> Option<(usize, usize)> {
    let mut klammer = 0i32;
    let mut k = von;
    while let Some(&c) = z.get(k) {
        match c {
            '(' | '[' => klammer += 1,
            ')' | ']' => klammer -= 1,
            ';' if klammer == 0 => return Some((k, k)),
            '{' if klammer == 0 => {
                let mut tiefe = 0i32;
                for (j, &d) in z.iter().enumerate().skip(k) {
                    match d {
                        '{' => tiefe += 1,
                        '}' => {
                            tiefe -= 1;
                            if tiefe == 0 {
                                return Some((k, j));
                            }
                        }
                        _ => {}
                    }
                }
                return None;
            }
            _ => {}
        }
        k += 1;
    }
    None
}

/// Name, Beginn `{` und Ende `}` jeder Funktion mit Rumpf.
fn fn_rumpfe(z: &[char]) -> Vec<(String, usize, usize)> {
    let mut aus = Vec::new();
    for i in wort_stellen(z, "fn") {
        let mut k = i + 2;
        if !z.get(k).is_some_and(|c| c.is_whitespace()) {
            continue; // `fn(i32) -> i32` als Typ
        }
        while z.get(k).is_some_and(|c| c.is_whitespace()) {
            k += 1;
        }
        let von = k;
        while z.get(k).is_some_and(|&c| ist_kennung(c)) {
            k += 1;
        }
        if k == von {
            continue;
        }
        let name: String = z[von..k].iter().collect();
        if let Some((term, ende)) = item_ab(z, k) {
            if z[term] == '{' {
                aus.push((name, term, ende));
            }
        }
    }
    aus
}

/// `(von, bis, pub)` jedes `use`-Satzes; `pub` heisst Wiedereinfuehrung, kein Aufruf.
fn use_bereiche(z: &[char]) -> Vec<(usize, usize, bool)> {
    wort_stellen(z, "use")
        .into_iter()
        .map(|i| {
            let ende = (i..z.len()).find(|&k| z[k] == ';').unwrap_or(z.len());
            let vor: String = z[i.saturating_sub(12)..i].iter().collect();
            let vor = vor.trim_end();
            (i, ende, vor.ends_with("pub") || vor.ends_with(')'))
        })
        .collect()
}

fn zeile_von(z: &[char], i: usize) -> usize {
    z[..i].iter().filter(|&&c| c == '\n').count() + 1
}

/// Jedes `Command::new` einer Datei samt Zeitlimit-Urteil, dazu jede Verwendung von `Command`, die der
/// Scanner nicht als `Command::new(` erkennt.
fn scanne(datei: &str, quelle: &str) -> (Vec<Fund>, Vec<String>) {
    let z = maskiere(quelle);
    let rumpfe = fn_rumpfe(&z);
    let uses = use_bereiche(&z);
    let (mut funde, mut unklar) = (Vec::new(), Vec::new());
    for i in wort_stellen(&z, "Command") {
        let ort = format!("{datei}:{}", zeile_von(&z, i));
        let mut k = i + "Command".len();
        while z.get(k).is_some_and(|c| c.is_whitespace()) {
            k += 1;
        }
        if uses.iter().any(|&(von, bis, _)| von < i && i < bis) {
            if z[k..].starts_with(&['a', 's']) && !z.get(k + 2).is_some_and(|&c| ist_kennung(c)) {
                unklar.push(format!("{ort}: `Command as ...` benennt den Typ um; der Scanner sucht `Command::new`"));
            }
            continue;
        }
        let nach: String = z[i + "Command".len()..].iter().take(80).filter(|c| !c.is_whitespace()).collect();
        if !nach.starts_with("::new(") {
            unklar.push(format!("{ort}: `Command` ohne `::new(`; der Scanner sieht den Start des Kindes nicht"));
            continue;
        }
        let Some((name, von, bis)) = rumpfe.iter().filter(|(_, von, bis)| *von < i && i < *bis).max_by_key(|(_, von, _)| *von)
        else {
            unklar.push(format!("{ort}: `Command::new` steht in keiner Funktion"));
            continue;
        };
        let rumpf: String = z[*von..=*bis].iter().collect();
        funde.push(Fund {
            datei: datei.to_owned(),
            funktion: name.clone(),
            zeile: zeile_von(&z, i),
            hat_limit: rumpf.contains(".wait_timeout(") && rumpf.contains(".kill("),
        });
    }
    (funde, unklar)
}

/// Aufrufe der Namen aus `namen` ausserhalb von `#[cfg(test)]`; erlaubt sind Definition und `pub use`.
fn verbotene_aufrufer(datei: &str, quelle: &str, namen: &[&str]) -> Vec<String> {
    let z = maskiere(quelle);
    let test: Vec<(usize, usize)> = folge_stelle(&z, "#[cfg(test)]")
        .into_iter()
        .filter_map(|i| item_ab(&z, i + "#[cfg(test)]".len()).map(|(_, ende)| (i, ende)))
        .collect();
    let uses = use_bereiche(&z);
    let mut aus = Vec::new();
    for name in namen {
        for i in wort_stellen(&z, name) {
            if test.iter().any(|&(von, bis)| von < i && i <= bis) {
                continue;
            }
            let mut j = i;
            while j > 0 && z[j - 1].is_whitespace() {
                j -= 1;
            }
            let definition = j >= 2 && z[j - 2..j] == ['f', 'n'] && (j < 3 || !ist_kennung(z[j - 3]));
            let wieder_ein = uses.iter().any(|&(von, bis, oeffentlich)| oeffentlich && von < i && i < bis);
            if !definition && !wieder_ein {
                aus.push(format!("{datei}:{}: `{name}` wird ausserhalb von #[cfg(test)] benutzt", zeile_von(&z, i)));
            }
        }
    }
    aus
}

/// Abweichungen zwischen Funden und `ausnahmen`, je eine Zeile.
fn abweichungen(funde: &[Fund], ausnahmen: &[Ausnahme]) -> Vec<String> {
    let mut je_funktion: BTreeMap<(&str, &str), Vec<&Fund>> = BTreeMap::new();
    for f in funde {
        je_funktion.entry((f.datei.as_str(), f.funktion.as_str())).or_default().push(f);
    }
    let mut aus = Vec::new();
    for (&(datei, funktion), liste) in &je_funktion {
        let ausnahme = ausnahmen.iter().find(|a| a.datei == datei && a.funktion == funktion);
        let ort = format!("{datei}::{funktion} (Zeile {})", liste[0].zeile);
        match ausnahme {
            Some(a) if liste.len() != a.anzahl => {
                aus.push(format!("{ort}: {} Aufrufe, die Ausnahme nennt {}", liste.len(), a.anzahl));
            }
            Some(_) if liste.iter().any(|f| f.hat_limit) => {
                aus.push(format!("{ort}: hat ein Zeitlimit; die Ausnahme ist ueberfluessig, streichen"));
            }
            None if liste.iter().any(|f| !f.hat_limit) => {
                aus.push(format!(
                    "{ort}: `Command::new` ohne Zeitlimit (`.wait_timeout(` und `.kill(` in derselben Funktion) und ohne Eintrag in AUSNAHMEN"
                ));
            }
            _ => {}
        }
    }
    for a in ausnahmen {
        if !je_funktion.contains_key(&(a.datei, a.funktion)) {
            aus.push(format!("{}::{}: Ausnahme ohne `Command::new`; streichen oder Name nachziehen", a.datei, a.funktion));
        }
    }
    aus
}

fn rust_wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// `src/**/*.rs` und `build.rs` jedes Crates unter `wurzel`, ohne `parity`.
fn quelldateien(wurzel: &Path) -> Vec<PathBuf> {
    fn sammle(dir: &Path, aus: &mut Vec<PathBuf>) {
        let mut eintraege: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
        eintraege.sort();
        for p in eintraege {
            if p.is_dir() {
                sammle(&p, aus);
            } else if p.extension().is_some_and(|e| e == "rs") {
                aus.push(p);
            }
        }
    }
    let mut aus = Vec::new();
    let mut crates: Vec<_> = std::fs::read_dir(wurzel)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("src").is_dir() && p.join("Cargo.toml").is_file())
        .collect();
    crates.sort();
    for c in crates {
        if c.file_name().is_some_and(|n| n == "parity") {
            continue;
        }
        sammle(&c.join("src"), &mut aus);
        if c.join("build.rs").is_file() {
            aus.push(c.join("build.rs"));
        }
    }
    aus
}

fn relativ(wurzel: &Path, p: &Path) -> String {
    p.strip_prefix(wurzel).unwrap().to_string_lossy().replace('\\', "/")
}

/// `(Dateien, Funde, unklare Verwendungen, verbotene Aufrufer)` ueber den echten Quelltext.
fn echt() -> (usize, Vec<Fund>, Vec<String>, Vec<String>) {
    let wurzel = rust_wurzel();
    let dateien = quelldateien(&wurzel);
    let namen: Vec<&str> = AUSNAHMEN.iter().flat_map(|a| a.kein_aufrufer.iter().copied()).collect();
    let (mut funde, mut unklar, mut aufrufer) = (Vec::new(), Vec::new(), Vec::new());
    for p in &dateien {
        let rel = relativ(&wurzel, p);
        let text = std::fs::read_to_string(p).unwrap();
        let (f, u) = scanne(&rel, &text);
        funde.extend(f);
        unklar.extend(u);
        aufrufer.extend(verbotene_aufrufer(&rel, &text, &namen));
    }
    (dateien.len(), funde, unklar, aufrufer)
}

#[test]
fn jedes_command_new_hat_ein_zeitlimit_oder_eine_begruendete_ausnahme() {
    let (_, funde, unklar, _) = echt();
    let mut probleme = abweichungen(&funde, AUSNAHMEN);
    probleme.extend(unklar);
    assert!(probleme.is_empty(), "Unterprozess ohne Zeitlimit:\n{}", probleme.join("\n"));
}

#[test]
fn kein_dienst_pfad_ruft_eine_ausgenommene_funktion() {
    let (_, _, _, aufrufer) = echt();
    assert!(
        aufrufer.is_empty(),
        "Eine Ausnahme gilt nur ohne Dienst-Pfad als Aufrufer; entweder die Funktion bekommt ein Zeitlimit \
         (und die Ausnahme faellt weg) oder der Aufruf entfaellt:\n{}",
        aufrufer.join("\n")
    );
}

#[test]
fn die_erfassung_ist_nicht_leer() {
    // Sonst ist ein leerer Scan (falscher Pfad, Scanner blind) ein gruener Lauf.
    let (dateien, funde, _, _) = echt();
    assert!(dateien > 50, "nur {dateien} Quelldateien gelesen; ein Crate-Verzeichnis fehlt");
    let mit_limit: Vec<_> = funde.iter().filter(|f| f.hat_limit).collect();
    assert!(
        mit_limit.iter().any(|f| f.datei == "eingang/src/ocr.rs" && f.funktion == "lauf"),
        "eingang::ocr::lauf muss als Aufruf mit Zeitlimit erkannt sein: {funde:?}"
    );
}

#[test]
fn jede_ausnahme_ist_begruendet() {
    for a in AUSNAHMEN {
        assert!(a.anzahl >= 1, "{}::{}: Anzahl 0 ist keine Ausnahme", a.datei, a.funktion);
        assert!(a.grund.len() > 80, "{}::{}: Grund zu knapp", a.datei, a.funktion);
        assert!(!a.kein_aufrufer.is_empty(), "{}::{}: ohne `kein_aufrufer` ist der Grund nicht pruefbar", a.datei, a.funktion);
        assert!(rust_wurzel().join(a.datei).is_file(), "{}: Datei fehlt", a.datei);
    }
}

fn fund(quelle: &str) -> (Vec<Fund>, Vec<String>) {
    scanne("x/src/y.rs", quelle)
}

#[test]
fn der_scanner_findet_pfad_und_nacktaufruf_und_ordnet_den_innersten_fn_zu() {
    let (f, u) = fund(
        "use std::process::Command;\n\
         fn aussen() {\n    let _ = std::process::Command::new(\"a\").output();\n    fn innen() {\n        let _ = Command :: new (\"b\");\n    }\n}\n",
    );
    assert!(u.is_empty(), "{u:?}");
    assert_eq!(f.len(), 2, "{f:?}");
    assert_eq!((f[0].funktion.as_str(), f[0].zeile), ("aussen", 3));
    assert_eq!((f[1].funktion.as_str(), f[1].zeile), ("innen", 5));
}

#[test]
fn der_scanner_ueberspringt_kommentar_text_und_zeichenliteral() {
    let (f, u) = fund(
        "// Command::new(\"x\")\n/* Command::new(\"y\") /* verschachtelt Command::new */ */\n\
         fn f() {\n    let _ = \"Command::new(\\\"z\\\")\";\n    let _ = r#\"Command::new(\"r\")\"#;\n    let _ = br\"Command::new\";\n    let _ = '\"'; let _ = '\\''; let _: &'static str = \"{\";\n}\n",
    );
    assert!(f.is_empty() && u.is_empty(), "{f:?} {u:?}");
}

#[test]
fn ein_limit_zaehlt_nur_im_selben_fn_und_nur_als_code() {
    let (f, _) = fund(
        "fn mit() { let mut k = Command::new(\"a\").spawn().unwrap(); k.wait_timeout(t); k.kill(); }\n\
         fn ohne() { let _ = Command::new(\"b\").output(); }\n\
         fn nur_kommentar() { let _ = Command::new(\"c\").output(); /* k.wait_timeout(t); k.kill(); */ }\n\
         fn nur_text() { let _ = Command::new(\"d\").output(); let _ = \".wait_timeout( .kill(\"; }\n\
         fn ohne_kill() { let mut k = Command::new(\"e\").spawn().unwrap(); k.wait_timeout(t); }\n\
         fn nebenan() { k.wait_timeout(t); k.kill(); }\n",
    );
    let urteil: Vec<(&str, bool)> = f.iter().map(|x| (x.funktion.as_str(), x.hat_limit)).collect();
    assert_eq!(
        urteil,
        [("mit", true), ("ohne", false), ("nur_kommentar", false), ("nur_text", false), ("ohne_kill", false)]
    );
}

#[test]
fn eine_verwendung_ausserhalb_von_command_new_ist_rot() {
    let (f, u) = fund("use std::process::Command as Kind;\nfn f() -> Command { todo!() }\nfn g(c: &Command) {}\n");
    assert!(f.is_empty(), "{f:?}");
    assert_eq!(u.len(), 3, "{u:?}");
    assert!(u[0].contains("benennt den Typ um"), "{u:?}");
    // Eine Liste in `use` ohne Umbenennung ist kein Befund.
    let (f, u) = fund("use std::process::{Child, Command, Stdio};\nfn f() {}\n");
    assert!(f.is_empty() && u.is_empty(), "{f:?} {u:?}");
}

#[test]
fn abweichungen_nennen_neuen_aufruf_ueberfluessige_und_verwaiste_ausnahme() {
    let ohne = |d: &str, fk: &str| Fund { datei: d.into(), funktion: fk.into(), zeile: 7, hat_limit: false };
    let mit = |d: &str, fk: &str| Fund { datei: d.into(), funktion: fk.into(), zeile: 9, hat_limit: true };
    let a = |d, fk, n| Ausnahme { datei: d, funktion: fk, anzahl: n, grund: "", kein_aufrufer: &[] };
    // sauber: ein Limit ist ok, eine eingetragene Ausnahme ist ok
    assert!(abweichungen(&[mit("a.rs", "f"), ohne("b.rs", "g")], &[a("b.rs", "g", 1)]).is_empty());
    // neuer Aufruf ohne Limit
    let r = abweichungen(&[ohne("a.rs", "f")], &[]);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].contains("a.rs::f") && r[0].contains("ohne Zeitlimit"), "{r:?}");
    // Ausnahme ohne Aufruf (Funktion umbenannt oder Aufruf entfernt)
    let r = abweichungen(&[], &[a("b.rs", "g", 1)]);
    assert!(r.len() == 1 && r[0].contains("Ausnahme ohne `Command::new`"), "{r:?}");
    // Ausnahme, die inzwischen ein Limit hat
    let r = abweichungen(&[mit("b.rs", "g")], &[a("b.rs", "g", 1)]);
    assert!(r.len() == 1 && r[0].contains("ueberfluessig"), "{r:?}");
    // zweiter Aufruf in einer ausgenommenen Funktion
    let r = abweichungen(&[ohne("b.rs", "g"), ohne("b.rs", "g")], &[a("b.rs", "g", 1)]);
    assert!(r.len() == 1 && r[0].contains("2 Aufrufe"), "{r:?}");
}

#[test]
fn der_aufrufer_scan_laesst_definition_reexport_und_tests_durch() {
    let namen = ["rechne"];
    let quelle = "pub use m::{rechne, anders};\n\
                  /// `rechne` im Kommentar\n\
                  pub fn rechne() {}\n\
                  fn dienst() { rechne(); }\n\
                  #[cfg(test)]\nmod tests {\n    fn t() { rechne(); }\n}\n\
                  fn danach() { let _ = \"rechne\"; rechne_nicht(); }\n";
    let r = verbotene_aufrufer("x.rs", quelle, &namen);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].starts_with("x.rs:4:"), "{r:?}");
    // ein privater Import in einem Dienst-Crate ist ein Aufrufer
    let r = verbotene_aufrufer("y.rs", "use elster::rechne;\n", &namen);
    assert_eq!(r.len(), 1, "{r:?}");
    // `#[cfg(test)]` an einem einzelnen Item nimmt nur dieses Item aus, nicht den Rest der Datei
    let r = verbotene_aufrufer("z.rs", "#[cfg(test)]\nfn t() { rechne(); }\nfn p() { rechne(); }\n", &namen);
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].starts_with("z.rs:3:"), "{r:?}");
}
