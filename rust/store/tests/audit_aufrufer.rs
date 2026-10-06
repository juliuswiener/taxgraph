//! Inventar der Aufrufer, die ins Protokoll schreiben: jeder Aufruf traegt ein FESTES `detail` ohne Werte.
//!
//! Gegenstueck zu `tests/test_audit.py::test_alle_audit_aufrufer_sind_bekannt` und
//! `::test_detail_felder_enthalten_nur_metadaten`. Das Protokoll haelt fest, WER WANN WAS getan hat,
//! nie WELCHE Werte (IBAN, Betrag, Name, Freitext). Ein neuer Aufrufer, der einen Nutzerwert in
//! `detail` legt, macht aus dem Protokoll eine zweite Ablage personenbezogener Daten ohne
//! Loeschpfad. Der Test liest den Quelltext aller Crates unter `rust/*/src` (ohne `parity`):
//!
//! - jeder Aufruf von `store::audit::anhaengen` (5. Argument `detail`) und jeder Aufruf der Methode
//!   `Protokoll::melde` (der Anfang des LLM-`detail`: `llm_call`) steht mit seinem Ausdruck im
//!   Inventar `ERLAUBT`; ein NEUER oder GEAENDERTER Aufruf macht den Test rot, bis jemand sein
//!   `detail` auf Werte geprueft und den Eintrag samt Begruendung nachgetragen hat,
//! - ein Eintrag, den es im Quelltext nicht mehr gibt, ist rot (keine Liste, die Pruefung vortaeuscht),
//! - kein `detail` setzt einen Ausdruck mit Namen wie `wert`, `name`, `iban`, `betrag`, `text` ein;
//!   die LAENGE eines Textes (`laenge(&text)`) ist erlaubt, sie verraet nichts.
//!
//! Der Scanner kennt Kommentare, Zeichenketten (auch rohe) und Zeichenliterale; eine Erwaehnung von
//! `anhaengen(` im Kommentar oder in einem Text zaehlt nicht. Er sieht keinen Aufruf, der erst zur
//! Laufzeit gebaut wird (Funktionszeiger, Makro); dafuer steht die Zahl der Treffer als Untergrenze.
//! `store/src/audit.rs` (die Definition mit ihren Selbsttests) und `parity` sind ausgenommen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::many_single_char_names, // der Scanner arbeitet Zeichen fuer Zeichen (z, i, k, c, n)
    clippy::similar_names
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Ein gefundener Aufruf.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Aufruf {
    datei: String,
    zeile: usize,
    funktion: String,
    /// Das `detail`-Argument, Leerraum zusammengezogen; `Some(&name)` ist zur Zuweisung
    /// `let name = ...` aufgeloest, wenn sie in derselben Datei davor steht.
    detail: String,
}

/// Quelltext ohne Kommentare (durch Leerzeichen ersetzt, Zeilenumbrueche bleiben) und fuer jedes
/// Zeichen die Angabe, ob es INNERHALB einer Zeichenkette steht.
fn bereinige(quelle: &str) -> (Vec<char>, Vec<bool>) {
    let z: Vec<char> = quelle.chars().collect();
    let mut aus = z.clone();
    let mut in_text = vec![false; z.len()];
    let leer = |aus: &mut Vec<char>, von: usize, bis: usize| {
        for c in &mut aus[von..bis] {
            if *c != '\n' {
                *c = ' ';
            }
        }
    };
    let mut i = 0;
    while i < z.len() {
        let c = z[i];
        let n = z.get(i + 1).copied();
        if c == '/' && n == Some('/') {
            let ende = (i..z.len()).find(|&k| z[k] == '\n').unwrap_or(z.len());
            leer(&mut aus, i, ende);
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
            leer(&mut aus, i, k);
            i = k;
        } else if c == 'r' && matches!(n, Some('"' | '#')) && roh_beginn(&z, i).is_some() {
            let (raten, start) = roh_beginn(&z, i).unwrap();
            let mut k = start;
            while k < z.len() {
                if z[k] == '"' && (1..=raten).all(|j| z.get(k + j) == Some(&'#')) {
                    break;
                }
                k += 1;
            }
            for t in in_text.iter_mut().take(k).skip(start) {
                *t = true;
            }
            i = (k + 1 + raten).min(z.len());
        } else if c == '"' {
            let mut k = i + 1;
            while k < z.len() && z[k] != '"' {
                k += if z[k] == '\\' { 2 } else { 1 };
            }
            let k = k.min(z.len());
            for t in in_text.iter_mut().take(k).skip(i + 1) {
                *t = true;
            }
            i = k + 1;
        } else if c == '\'' {
            // Zeichenliteral (`'x'`, `'\n'`, `'"'`) oder Lebensdauer (`'a`).
            if n == Some('\\') {
                let ende = (i + 2..z.len()).find(|&k| z[k] == '\'').unwrap_or(z.len() - 1);
                i = ende + 1;
            } else if z.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    (aus, in_text)
}

/// `r"`, `r#"`, `r##"` ...: Zahl der `#` und Index des ersten Zeichens im Text.
fn roh_beginn(z: &[char], i: usize) -> Option<(usize, usize)> {
    if i > 0 && (z[i - 1].is_alphanumeric() || z[i - 1] == '_') {
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

fn ist_kennung(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Die Argumente des Aufrufs, dessen `(` an `klammer` steht, und der Index hinter der `)`.
fn argumente(z: &[char], in_text: &[bool], klammer: usize) -> (Vec<String>, usize) {
    let mut tiefe = 0;
    let mut args: Vec<String> = Vec::new();
    let mut aktuell = String::new();
    let mut k = klammer;
    while k < z.len() {
        let c = z[k];
        let im_text = in_text[k];
        if !im_text {
            match c {
                '(' | '[' | '{' => {
                    tiefe += 1;
                    if tiefe == 1 {
                        k += 1;
                        continue;
                    }
                }
                ')' | ']' | '}' => {
                    tiefe -= 1;
                    if tiefe == 0 {
                        if !aktuell.trim().is_empty() {
                            args.push(zusammenziehen(&aktuell));
                        }
                        return (args, k + 1);
                    }
                }
                ',' if tiefe == 1 => {
                    args.push(zusammenziehen(&aktuell));
                    aktuell.clear();
                    k += 1;
                    continue;
                }
                _ => {}
            }
        }
        aktuell.push(c);
        k += 1;
    }
    (args, z.len())
}

/// Leerraum auf ein Leerzeichen, keine Leerzeichen innerhalb von Klammern am Rand.
fn zusammenziehen(s: &str) -> String {
    let mut aus = String::new();
    for wort in s.split_whitespace() {
        if !aus.is_empty() {
            aus.push(' ');
        }
        aus.push_str(wort);
    }
    aus.replace("( ", "(").replace(" )", ")")
}

/// Der Initialisierer der letzten Zuweisung `let <name> = ...;` vor `bis` (oder `None`).
fn zuweisung(z: &[char], in_text: &[bool], name: &str, bis: usize) -> Option<String> {
    let muster: Vec<char> = format!("let {name} =").chars().collect();
    let start = (0..bis.saturating_sub(muster.len()))
        .rev()
        .find(|&i| z[i..i + muster.len()] == muster[..] && !in_text[i])?;
    let von = start + muster.len();
    let mut tiefe = 0_i32;
    let mut k = von;
    while k < z.len() {
        if !in_text[k] {
            match z[k] {
                '(' | '[' | '{' => tiefe += 1,
                ')' | ']' | '}' => tiefe -= 1,
                ';' if tiefe == 0 => break,
                _ => {}
            }
        }
        k += 1;
    }
    Some(zusammenziehen(&z[von..k].iter().collect::<String>()))
}

/// Alle Aufrufe von `funktion` (bei `methode` nur die Form `.funktion(`, sonst jede ausser der
/// Definition `fn funktion`). `detail_stelle`: Index des `detail`-Arguments.
fn finde(
    datei: &str,
    quelle: &str,
    funktion: &str,
    methode: bool,
    detail_stelle: usize,
) -> Vec<Aufruf> {
    let (z, in_text) = bereinige(quelle);
    let name: Vec<char> = funktion.chars().collect();
    let mut treffer = Vec::new();
    let mut i = 0;
    while i + name.len() <= z.len() {
        let passt = z[i..i + name.len()] == name[..]
            && !in_text[i]
            && (i == 0 || !ist_kennung(z[i - 1]))
            && z.get(i + name.len()).is_none_or(|&c| !ist_kennung(c));
        if !passt {
            i += 1;
            continue;
        }
        let davor: String = z[..i].iter().collect();
        let davor = davor.trim_end();
        let nach = (i + name.len()..z.len()).find(|&k| !z[k].is_whitespace());
        let ist_aufruf = nach.is_some_and(|k| z[k] == '(');
        let ist_definition = davor.ends_with("fn");
        let ist_methode = davor.ends_with('.');
        if ist_aufruf && !ist_definition && (ist_methode || !methode) {
            let (args, ende) = argumente(&z, &in_text, nach.unwrap());
            let roh = args.get(detail_stelle).cloned().unwrap_or_default();
            let detail = aufgeloest(&z, &in_text, &roh, i);
            treffer.push(Aufruf {
                datei: datei.to_owned(),
                zeile: z[..i].iter().filter(|&&c| c == '\n').count() + 1,
                funktion: funktion.to_owned(),
                detail,
            });
            i = ende;
        } else {
            i += name.len();
        }
    }
    treffer
}

/// `Some(&name)` / `Some(name)` zeigt auf `let name = ...;` in derselben Datei: dann gilt der
/// Initialisierer. Ein Parameter (kein `let` davor) bleibt, wie er steht.
fn aufgeloest(z: &[char], in_text: &[bool], roh: &str, vor: usize) -> String {
    let inner = roh
        .strip_prefix("Some(")
        .and_then(|r| r.strip_suffix(')'))
        .map(|r| r.trim_start_matches('&'));
    match inner {
        Some(n) if !n.is_empty() && n.chars().all(ist_kennung) => zuweisung(z, in_text, n, vor)
            .map_or_else(|| roh.to_owned(), |init| format!("Some(&{init})")),
        _ => roh.to_owned(),
    }
}

/// Namen, die einen Nutzerwert tragen koennen. Gilt fuer Platzhalter im Text (`{wert}`) und fuer
/// Ausdruecke ausserhalb des Textes (`name.clone()`); `laenge(...)` und `.len()` sind ausgenommen.
const WERT_NAMEN: &[&str] = &[
    "wert", "werte", "name", "vorname", "nachname", "idnr", "iban", "betrag", "freitext", "eingabe",
    "text", "inhalt", "rumpf", "body", "nachricht", "antwort", "kontext", "beleg", "adresse",
    "strasse", "passwort", "password", "token",
];

/// Die Wert-Namen, die `detail` einsetzt (leer = in Ordnung).
fn wert_namen_in(detail: &str) -> Vec<String> {
    // `laenge(...)` und `.len()` verraten die Laenge, nicht den Inhalt.
    let mut rest = detail.to_owned();
    while let Some(p) = rest.find("laenge(") {
        let z: Vec<char> = rest.chars().collect();
        let mut tiefe = 0;
        let mut ende = z.len();
        for (k, c) in z.iter().enumerate().skip(p + "laenge".len()) {
            match c {
                '(' => tiefe += 1,
                ')' => {
                    tiefe -= 1;
                    if tiefe == 0 {
                        ende = k + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        let vorher: String = z[..p].iter().collect();
        let nachher: String = z[ende..].iter().collect();
        rest = format!("{vorher}_{nachher}");
    }
    let rest = ohne_pfad_aufruf(&ohne_pfad_aufruf(&rest, ".len()"), ".is_empty()");
    let mut gefunden = Vec::new();
    let mut wort = String::new();
    let mut im_text = false;
    let mut platzhalter = false;
    let mut vorher = ' ';
    for c in rest.chars() {
        // Zeichenketten: nur Platzhalter `{...}` zaehlen, nicht der feste Text.
        if c == '"' && vorher != '\\' {
            im_text = !im_text;
            wort.clear();
        } else if im_text {
            if c == '{' && vorher != '{' {
                platzhalter = true;
                wort.clear();
            } else if platzhalter && (ist_kennung(c) || c == '.') {
                wort.push(c);
            } else if platzhalter {
                platzhalter = false;
                pruefe_wort(&mut wort, &mut gefunden);
            }
        } else if ist_kennung(c) || c == '.' {
            wort.push(c);
        } else {
            pruefe_wort(&mut wort, &mut gefunden);
        }
        vorher = c;
    }
    pruefe_wort(&mut wort, &mut gefunden);
    gefunden
}

/// Entfernt `pfad.zu.wert.len()` ganz (Pfad und Aufruf): die Laenge oder Leere verraet den Inhalt nicht.
fn ohne_pfad_aufruf(text: &str, aufruf: &str) -> String {
    let mut z: Vec<char> = text.chars().collect();
    let a: Vec<char> = aufruf.chars().collect();
    while let Some(p) = (0..z.len().saturating_sub(a.len() - 1)).find(|&i| z[i..i + a.len()] == a[..]) {
        let mut von = p;
        while von > 0 && (ist_kennung(z[von - 1]) || z[von - 1] == '.') {
            von -= 1;
        }
        z.splice(von..p + a.len(), ['_']);
    }
    z.into_iter().collect()
}

fn pruefe_wort(wort: &mut String, gefunden: &mut Vec<String>) {
    for teil in wort.split('.') {
        if WERT_NAMEN.contains(&teil) {
            gefunden.push(teil.to_owned());
        }
    }
    wort.clear();
}

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
    }
    aus
}

/// Alle Aufrufe im echten Baum.
fn alle_aufrufe() -> Vec<Aufruf> {
    let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut aus = Vec::new();
    for p in quelldateien(&wurzel) {
        let rel = p.strip_prefix(&wurzel).unwrap().to_string_lossy().into_owned();
        if rel == "store/src/audit.rs" {
            continue;
        }
        let text = std::fs::read_to_string(&p).unwrap();
        aus.extend(finde(&rel, &text, "anhaengen", false, 4));
        aus.extend(finde(&rel, &text, "melde", true, 0));
        aus.extend(finde_kopf(&rel, &text));
    }
    aus
}

/// Das Feld `kopf: format!(...)` (der feste Anfang jedes LLM-`detail`, `llm/src/dialog.rs`): auch ein
/// Wert, der dort einzieht, landet im Protokoll, ohne dass ein `melde`-Aufruf ihn zeigt.
fn finde_kopf(datei: &str, quelle: &str) -> Vec<Aufruf> {
    let (z, in_text) = bereinige(quelle);
    let name: Vec<char> = "kopf".chars().collect();
    let mut treffer = Vec::new();
    for i in 0..z.len().saturating_sub(name.len()) {
        if z[i..i + name.len()] != name[..] || in_text[i] || (i > 0 && ist_kennung(z[i - 1])) {
            continue;
        }
        let rest: String = z[i + name.len()..].iter().take(40).collect();
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix(':') else { continue };
        if !rest.trim_start().starts_with("format!") {
            continue;
        }
        let klammer = (i..z.len()).find(|&k| z[k] == '(' && !in_text[k]).unwrap();
        let (args, _) = argumente(&z, &in_text, klammer);
        treffer.push(Aufruf {
            datei: datei.to_owned(),
            zeile: z[..i].iter().filter(|&&c| c == '\n').count() + 1,
            funktion: "kopf".to_owned(),
            detail: format!("format!({})", args.join(", ")),
        });
    }
    treffer
}

/// Der Schluessel im Inventar: bei einem `format!` die Vorlage (der feste Text mit seinen
/// Platzhaltern), sonst der Ausdruck selbst. Die Vorlage legt fest, WELCHE Angaben ins Protokoll
/// gehen; die Ausdruecke dahinter prueft `wert_namen_in`.
fn schluessel(detail: &str) -> String {
    let Some(p) = detail.find("format!(") else {
        return detail.to_owned();
    };
    let rest = &detail[p + "format!(".len()..];
    let rest = rest.trim_start();
    let Some(rest) = rest.strip_prefix('"') else {
        return detail.to_owned();
    };
    let mut vorlage = String::new();
    let mut zeichen = rest.chars();
    while let Some(c) = zeichen.next() {
        match c {
            '\\' => {
                vorlage.push(c);
                vorlage.extend(zeichen.next());
            }
            '"' => break,
            _ => vorlage.push(c),
        }
    }
    format!("format:{vorlage}")
}


/// Das Inventar: (Datei, Funktion, Schluessel, Begruendung). Wer einen Aufruf ergaenzt oder seine
/// Vorlage aendert, prueft sein `detail` auf Werte und traegt ihn hier ein; die Begruendung nennt,
/// was in `detail` steht und warum es kein Nutzerwert ist. Die Schluessel kommen von `schluessel`.
const ERLAUBT: &[(&str, &str, &str, &str)] = &[
    (
        "api/src/chat.rs",
        "melde",
        "teil",
        "reicht den Text von llm/src/dialog.rs unveraendert weiter; dort sind Kopf und Teile einzeln aufgefuehrt",
    ),
    (
        "api/src/dispatch.rs",
        "anhaengen",
        "format:status={status}",
        "nur der HTTP-Status der Antwort (eine Zahl)",
    ),
    (
        "api/src/eigener_fall.rs",
        "anhaengen",
        "format:user={uid}, owner={}",
        "Nutzername des Anfragenden und des Besitzers; beide stehen ohnehin als user_id im Protokoll, kein Freitext",
    ),
    (
        "api/src/einreichen.rs",
        "anhaengen",
        "format:vz={vz} rc=0",
        "Veranlagungszeitraum (Zahl) und der feste Rueckgabecode 0",
    ),
    (
        "api/src/routen/fall.rs",
        "anhaengen",
        "format:scheibe={scheibe}",
        "Name der Scheibe, geprueft gegen die feste Liste (scheibe_pruefen), kein Nutzertext",
    ),
    (
        "api/src/routen/fall.rs",
        "anhaengen",
        "format:scheibe={}, vz={vz}",
        "Scheibe der geloeschten Akte (aus der Liste) und Veranlagungszeitraum (Zahl)",
    ),
    (
        "auth/src/lib.rs",
        "anhaengen",
        "None",
        "Anmeldung, Abmeldung, Registrierung, Fehlschlag: ohne detail, nur Nutzer und Aktion",
    ),
    (
        "llm/src/dialog.rs",
        "anhaengen",
        "Some(teil)",
        "schreibt den Text, den Melder::melde aus Kopf und Teil baut (Eintraege darunter); nur Kategorien, Zaehlwerte, Anbieter",
    ),
    (
        "llm/src/dialog.rs",
        "kopf",
        "format:pii_kategorien={}, kontext_kategorien={}, textlaenge_vor={}, textlaenge_nach={}",
        "Namen der erkannten PII-Kategorien (nicht ihre Fundstellen) und Textlaengen; der Text selbst geht nur in laenge(...)",
    ),
    (
        "llm/src/dialog.rs",
        "melde",
        "format:{}, {teil}",
        "haengt den Teil an den Kopf; beide sind in diesem Inventar einzeln gefuehrt",
    ),
    (
        "llm/src/dialog.rs",
        "melde",
        "format:stufe={stufe}, ergebnis=kein_ergebnis, grund={grund}, versuche={}, provider={}",
        "Stufe, fester Ausfallgrund (Klasse des Fehlers), Zahl der Versuche, Name des Anbieters",
    ),
    (
        "llm/src/dialog.rs",
        "melde",
        "format:stufe=1, aussagen={}, aussagen_ohne_beleg={}, inhalt_laenge={}, provider='{}', finish='{}'",
        "Zaehlwerte, Laenge der Antwort, Name des Anbieters und Abschlussgrund",
    ),
    (
        "llm/src/dialog.rs",
        "melde",
        "format:stufe=2, aussagen={}, zugeordnet={}, regeln={}/{}, inhalt_laenge={}, provider='{}', finish='{}'",
        "Zaehlwerte, Laenge der Antwort, Name des Anbieters und Abschlussgrund",
    ),
    (
        "llm/src/dialog.rs",
        "melde",
        "format:stufe=3, katalog={}, katalog_felder={}, vorschlaege={}, ohne_beleg_verworfen={}, rueckfragen={}, rueckfragen_zurueckgestellt={zurueckgestellt}, rueckfragen_geloest={}, offen={}, antwortlaenge={}, unsicher={}, inhalt_laenge={}, provider='{}', finish='{}'",
        "Zaehlwerte und Laengen, eine Wahrheitsangabe, Name des Anbieters und Abschlussgrund",
    ),
];

fn inventar() -> BTreeMap<(String, String, String), usize> {
    let mut m = BTreeMap::new();
    for (d, f, s, _) in ERLAUBT {
        *m.entry(((*d).to_owned(), (*f).to_owned(), (*s).to_owned())).or_default() += 1;
    }
    m
}

fn gefunden(aufrufe: &[Aufruf]) -> BTreeMap<(String, String, String), usize> {
    let mut m = BTreeMap::new();
    for a in aufrufe {
        *m.entry((a.datei.clone(), a.funktion.clone(), schluessel(&a.detail))).or_default() += 1;
    }
    m
}

/// Vergleich Fund gegen Inventar; leer = gleich.
fn abweichungen(
    fund: &BTreeMap<(String, String, String), usize>,
    liste: &BTreeMap<(String, String, String), usize>,
) -> Vec<String> {
    let mut aus = Vec::new();
    for (k, n) in fund {
        if liste.get(k) != Some(n) {
            aus.push(format!(
                "NEU oder geaendert: {} {} {:?} (x{n}, im Inventar x{}) - detail auf Werte pruefen und in ERLAUBT eintragen",
                k.0, k.1, k.2, liste.get(k).copied().unwrap_or(0)
            ));
        }
    }
    for (k, n) in liste {
        if !fund.contains_key(k) {
            aus.push(format!(
                "VERALTET: {} {} {:?} (x{n}) steht im Inventar, ist im Quelltext aber nicht mehr da - Eintrag streichen",
                k.0, k.1, k.2
            ));
        }
    }
    aus
}

#[test]
fn jeder_aufruf_steht_im_inventar_und_jeder_eintrag_hat_einen_aufruf() {
    let aufrufe = alle_aufrufe();
    let diff = abweichungen(&gefunden(&aufrufe), &inventar());
    assert!(diff.is_empty(), "\n{}", diff.join("\n"));
}

/// Boden: der Scan sieht heute 7 Aufrufe von `anhaengen`, 5 von `melde` und den `kopf`. Faellt die
/// Erfassung (Verzeichnis umbenannt, Scanner kaputt), bliebe der Vergleich oben leer und gruen.
#[test]
fn die_erfassung_ist_nicht_leer() {
    let aufrufe = alle_aufrufe();
    let zahl = |f: &str| aufrufe.iter().filter(|a| a.funktion == f).count();
    assert!(zahl("anhaengen") >= 7, "nur {} Aufrufe von anhaengen", zahl("anhaengen"));
    assert!(zahl("melde") >= 5, "nur {} Aufrufe von melde", zahl("melde"));
    assert!(zahl("kopf") >= 1, "kein kopf: format!(..) gefunden");
    let dateien: std::collections::BTreeSet<&str> = aufrufe.iter().map(|a| a.datei.as_str()).collect();
    assert!(dateien.len() >= 6, "nur {} Dateien: {dateien:?}", dateien.len());
}

#[test]
fn kein_detail_traegt_einen_wert() {
    let schlecht: Vec<String> = alle_aufrufe()
        .iter()
        .filter_map(|a| {
            let namen = wert_namen_in(&a.detail);
            (!namen.is_empty()).then(|| format!("{}:{} {}: setzt {namen:?} ein: {}", a.datei, a.zeile, a.funktion, a.detail))
        })
        .collect();
    assert!(
        schlecht.is_empty(),
        "detail mit moeglichem Nutzerwert (das Protokoll haelt WER WANN WAS fest, nie WELCHE Werte):\n{}",
        schlecht.join("\n")
    );
}

#[test]
fn jeder_eintrag_ist_begruendet() {
    for (d, f, s, grund) in ERLAUBT {
        assert!(grund.chars().count() >= 20, "{d} {f} {s:?}: Begruendung zu kurz");
    }
}

// ---------------------------------------------------------------------------------------------
// Der Scanner an seinen eigenen Fehlerfaellen (ein Wachter, der nie anschlug, ist eine Behauptung).
// ---------------------------------------------------------------------------------------------

fn fund(quelle: &str) -> Vec<Aufruf> {
    let mut a = finde("x.rs", quelle, "anhaengen", false, 4);
    a.extend(finde("x.rs", quelle, "melde", true, 0));
    a
}

#[test]
fn der_scanner_findet_pfad_und_nacktaufruf_samt_verschachteltem_format() {
    let q = r#"
        fn a() {
            store::audit::anhaengen(&p, Some(u), Aktion::X, Some(f), Some(&format!("a={}, b={}", f(1, 2), [3, 4][0])))?;
            anhaengen(
                &p, None, Aktion::Y, None,
                None,
            ).unwrap();
        }
    "#;
    let a = fund(q);
    assert_eq!(a.len(), 2, "{a:?}");
    assert_eq!(schluessel(&a[0].detail), "format:a={}, b={}");
    assert_eq!(a[1].detail, "None");
    assert_eq!(a[0].zeile, 3);
}

#[test]
fn der_scanner_ueberspringt_kommentar_text_definition_und_fremde_namen() {
    let q = r##"
        // anhaengen(&p, u, a, f, Some("kommentar"))
        /* anhaengen(&p, u, a, f, Some("block")) */
        /// anhaengen(&p, u, a, f, Some("doku"))
        fn anhaengen(p: &Path, u: Option<&str>, a: A, f: Option<&str>, d: Option<&str>) {}
        fn oeffne_zum_anhaengen(p: &Path) {}
        fn t() {
            let s = "anhaengen(a, b, c, d, e)";
            let r = r#"anhaengen(a, b, c, d, e)"#;
            let z = '"'; let l: &'a str = "x";
            oeffne_zum_anhaengen(p);
            anhaengen_und_lies(p);
            self.melde(x); melde(z, f, o);
        }
    "##;
    let a = fund(q);
    // `melde(z, f, o)` ist keine Methode und zaehlt nicht; `self.melde(x)` ist der einzige Treffer.
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].funktion, "melde");
}

#[test]
fn der_scanner_loest_eine_zuweisung_auf_und_laesst_parameter_stehen() {
    let q = r#"
        fn a() {
            let detail = format!("user={uid}, owner={}", b.unwrap_or("None"));
            anhaengen(&p, u, A::Z, f, Some(&detail))?;
        }
        fn b(teil: &str) { anhaengen(&p, u, A::Z, f, Some(teil)); }
    "#;
    let a = fund(q);
    assert_eq!(a.len(), 2);
    assert_eq!(schluessel(&a[0].detail), "format:user={uid}, owner={}");
    assert_eq!(a[1].detail, "Some(teil)");
}

#[test]
fn der_scanner_findet_das_kopf_feld() {
    let q = r#"
        let m = Melder { kopf: format!("kat={}, laenge={}", py_liste(&k), crate::py::laenge(text)), protokoll: p };
    "#;
    let a = finde_kopf("x.rs", q);
    assert_eq!(a.len(), 1);
    assert_eq!(schluessel(&a[0].detail), "format:kat={}, laenge={}");
    assert!(wert_namen_in(&a[0].detail).is_empty(), "{:?}", wert_namen_in(&a[0].detail));
}

#[test]
fn die_wertpruefung_trifft_platzhalter_und_ausdruecke_aber_nicht_laengen() {
    let treffer = |s: &str| wert_namen_in(s);
    // Platzhalter im Text und Ausdruck ausserhalb.
    assert_eq!(treffer(r#"Some(&format!("a={wert}"))"#), ["wert"]);
    assert_eq!(treffer(r#"Some(&format!("a={}", betrag))"#), ["betrag"]);
    assert_eq!(treffer(r#"Some(&format!("a={}", f.iban.clone()))"#), ["iban"]);
    assert_eq!(treffer(r#"Some(&format!("a={name:?}"))"#), ["name"]);
    assert_eq!(treffer(r#"Some(&format!("a={}", a.text))"#), ["text"]);
    // Laenge und Leere verraten nichts; fester Text mit dem Wort ist kein Platzhalter.
    assert!(treffer(r#"Some(&format!("n={}", crate::py::laenge(&c1.text)))"#).is_empty());
    assert!(treffer(r#"Some(&format!("n={}", antwort.len()))"#).is_empty());
    assert!(treffer(r#"Some(&format!("n={}", a.beleg.is_empty()))"#).is_empty());
    assert!(treffer(r#"Some(&format!("name und wert sind fest, n={}", 3))"#).is_empty());
    assert!(treffer(r#"Some(&format!("a={{wert}}"))"#).is_empty());
}
