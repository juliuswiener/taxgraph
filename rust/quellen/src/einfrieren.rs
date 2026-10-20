//! `scripts/freeze_source.py`: eine Rechtsquelle abrufen, entstrippen, prüfen, einfrieren.
//!
//! Warum ein Werkzeug und kein Einzeiler: beim Einfrieren von § 9 und § 6 hat ein falsches Start-Muster zwei LEERE Quellen
//! erzeugt, und `make sources-check` war trotzdem grün, denn eine leere Datei passt zu ihrem `sha256`. Darum laufen die
//! Prüfungen VOR dem Schreiben: der Text ist mindestens `--min-laenge` Zeichen lang (Vorgabe 500), und jede mit `--erwarte`
//! genannte Passage kommt wörtlich vor. Ohne beides wird nichts geschrieben.
//!
//! Der Abruf ist von der Bearbeitung getrennt: [`hole`] und [`text_aus_antwort`] liefern den Text, [`friere_ein`] arbeitet
//! auf Text. So braucht kein Test das Netz.

use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use regex::Regex;
use sha2::{Digest, Sha256};

use crate::unterprozess::lauf;
use crate::{entity, py, Fehler};

/// Gesamtzeit für einen Abruf mit `curl` (`--max-time` darin, dazu das Zeitlimit des Prozesses).
const CURL_ZEIT: u64 = 180;
const GZIP_LIMIT: Duration = Duration::from_secs(60);

/// Einfrier-Ebene: ganzer Paragraph oder ganzes BMF-Schreiben.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ebene {
    Paragraph,
    Schreiben,
}

/// Alles, was `freeze_source.py` aus der Kommandozeile liest.
#[derive(Debug, Clone)]
pub struct Auftrag {
    pub url: String,
    pub name: String,
    pub norm: String,
    pub start: String,
    pub ende: Option<String>,
    pub erwarte: Vec<String>,
    pub min_laenge: i64,
    pub fassung: String,
    pub abrufdatum: String,
    pub verwendet_in: String,
    pub out: PathBuf,
    pub authority: String,
    pub ebene: Ebene,
    pub norm_uri: String,
}

/// Was nach dem Schreiben bekannt ist.
#[derive(Debug, PartialEq, Eq)]
pub struct Gefroren {
    pub zeichen: usize,
    pub sha256: String,
    /// Die Zeile, die das Python-Skript druckt.
    pub meldung: String,
}

/// Die fremden Programme, die der Abruf braucht (austauschbar für Tests).
#[derive(Debug, Clone)]
pub struct Werkzeuge {
    pub curl: String,
    pub gzip: String,
    pub pdftotext: String,
}

impl Default for Werkzeuge {
    fn default() -> Self {
        Self { curl: "curl".into(), gzip: "gzip".into(), pdftotext: "pdftotext".into() }
    }
}

/// Die Antwort des Servers als Text: rohes gzip (Magic `1f 8b`, auch ohne `Content-Encoding`, so liefern Archivkopien) wird
/// mit `gzip -dc` entpackt; ungültiges UTF-8 wird zu Ersatzzeichen (`decode("utf-8", "replace")`).
///
/// ```
/// let w = quellen::einfrieren::Werkzeuge::default();
/// assert_eq!(quellen::einfrieren::text_aus_antwort("Grüße".as_bytes().to_vec(), &w).unwrap(), "Grüße");
/// assert_eq!(quellen::einfrieren::text_aus_antwort(vec![b'a', 0xff], &w).unwrap(), "a\u{fffd}");
/// ```
///
/// # Errors
/// [`Fehler::Unterprozess`]: `gzip` fehlt oder hängt; [`Fehler::Entpacken`]: `gzip` lehnt die Daten ab.
pub fn text_aus_antwort(roh: Vec<u8>, werkzeuge: &Werkzeuge) -> Result<String, Fehler> {
    let bytes = if roh.starts_with(&[0x1f, 0x8b]) { entpacke(roh, &werkzeuge.gzip)? } else { roh };
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn entpacke(roh: Vec<u8>, gzip: &str) -> Result<Vec<u8>, Fehler> {
    let aus = lauf(gzip, &[OsStr::new("-dc")], Some(roh), GZIP_LIMIT)?;
    if aus.status.success() {
        Ok(aus.stdout)
    } else {
        Err(Fehler::Entpacken { grund: String::from_utf8_lossy(&aus.stderr).trim().to_owned() })
    }
}

/// Ruft die URL ab: `file://…` liest die Datei (die Naht für Tests; `urllib` im Python-Skript kennt sie auch), `http://` und
/// `https://` gehen über `curl` (folgt Weiterleitungen, Fehlerstatus ab 400 ist ein Fehler, Kennung `Mozilla/5.0`).
///
/// # Errors
/// [`Fehler::Abruf`]: Schema nicht erlaubt, Datei fehlt, `curl` meldet einen Fehler; [`Fehler::Unterprozess`]: `curl` fehlt
/// oder hängt.
pub fn hole(url: &str, werkzeuge: &Werkzeuge) -> Result<Vec<u8>, Fehler> {
    if let Some(pfad) = url.strip_prefix("file://") {
        let pfad = pfad.strip_prefix("localhost").unwrap_or(pfad);
        let abruf = |grund: String| Fehler::Abruf { url: url.to_owned(), grund };
        if !pfad.starts_with('/') {
            return Err(abruf("file:// braucht einen absoluten Pfad".into()));
        }
        let lokal = PathBuf::from(prozent_dekodiere(pfad));
        return fs::read(&lokal).map_err(|e| abruf(format!("{}: {e}", lokal.display())));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(Fehler::Abruf { url: url.to_owned(), grund: "nur http://, https:// und file:// sind erlaubt".into() });
    }
    let max_zeit = CURL_ZEIT.to_string();
    let teile = ["-sS", "-f", "-L", "--proto", "=http,https", "--max-time", &max_zeit, "-A", "Mozilla/5.0", "--url", url];
    let args: Vec<&OsStr> = teile.iter().map(OsStr::new).collect();
    let aus = lauf(&werkzeuge.curl, &args, None, Duration::from_secs(CURL_ZEIT + 20))?;
    if aus.status.success() {
        Ok(aus.stdout)
    } else {
        Err(Fehler::Abruf { url: url.to_owned(), grund: String::from_utf8_lossy(&aus.stderr).trim().to_owned() })
    }
}

/// `%XX` im Pfad einer `file://`-URL, wie `urllib` es auflöst (ungültige Folgen bleiben stehen).
fn prozent_dekodiere(pfad: &str) -> String {
    let b = pfad.as_bytes();
    let mut aus = Vec::with_capacity(b.len());
    let mut i = 0;
    while let Some(&c) = b.get(i) {
        let ziffer = |k: usize| b.get(i + k).and_then(|&x| char::from(x).to_digit(16));
        if let (b'%', Some(h), Some(l)) = (c, ziffer(1), ziffer(2)) {
            aus.push(u8::try_from(h * 16 + l).unwrap_or(b'?'));
            i += 3;
        } else {
            aus.push(c);
            i += 1;
        }
    }
    String::from_utf8_lossy(&aus).into_owned()
}

/// Die vier festen Muster der Entstrippung (dieselben wie `re.sub` im Python-Skript).
struct Muster {
    script: Regex,
    absatz: Regex,
    tag: Regex,
    leer: Regex,
}

impl Muster {
    fn neu() -> Result<Self, Fehler> {
        let m = |p: &str| Regex::new(p).map_err(|e| Fehler::Muster(e.to_string()));
        Ok(Self { script: m(r"(?s)<script.*?</script>")?, absatz: m(r"</p>|<br\s*/?>")?, tag: m(r"<[^>]+>")?, leer: m(r"[ \t]+")? })
    }
}

/// HTML entstrippen und die Passage von `start` bis `ende` ausschneiden.
///
/// Reihenfolge wie im Python-Skript: `<script>…</script>` weg, `</p>` und `<br>` zu Zeilenumbruch, jedes andere Tag zu einem
/// Leerzeichen, Entities dekodieren, geschützte Leerzeichen (U+00A0) zu normalen, Tabs und Leerzeichenfolgen zu einem
/// Leerzeichen. Dann ab `start`; das Ende wird erst ab 10 Zeichen hinter dem Start gesucht (Zeichen, nicht Bytes); danach
/// jede Zeile gestrippt und Leerzeilen weg.
///
/// ```
/// let html = "<p>Vor</p><p>A&nbsp;&sect;&nbsp;9 Zeichen</p><script>x</script><p>Ende hier</p>";
/// assert_eq!(quellen::einfrieren::entstrippen(html, "A", Some("Ende")).unwrap(), "A § 9 Zeichen");
/// // "Ende" steht weniger als 10 Zeichen hinter dem Start: dort wird nicht gesucht, der Rest bleibt
/// let nah = "<p>A § 9</p><p>Ende hier</p>";
/// assert_eq!(quellen::einfrieren::entstrippen(nah, "A", Some("Ende")).unwrap(), "A § 9\nEnde hier");
/// ```
///
/// # Errors
/// [`Fehler::StartFehlt`] und die Fehler von [`entity::dekodiere`].
pub fn entstrippen(roh: &str, start: &str, ende: Option<&str>) -> Result<String, Fehler> {
    let m = Muster::neu()?;
    let t = m.script.replace_all(roh, "");
    let t = m.absatz.replace_all(&t, "\n");
    let t = m.tag.replace_all(&t, " ");
    let t = entity::dekodiere(&t)?.replace('\u{a0}', " ");
    let t = m.leer.replace_all(&t, " ");
    let i = t.find(start).ok_or_else(|| Fehler::StartFehlt { start: start.to_owned() })?;
    let ab_start = t.get(i..).unwrap_or_default();
    // `t.find(ende, i + 10)`: ab dem 10. Zeichen HINTER dem Start (i + 10 zählt Zeichen); weniger Zeichen übrig: nichts.
    let bis = ende
        .filter(|e| !e.is_empty())
        .and_then(|e| {
            let (versatz, _) = ab_start.char_indices().nth(10)?;
            ab_start.get(versatz..)?.find(e).map(|k| versatz + k)
        })
        .and_then(|j| ab_start.get(..j))
        .unwrap_or(ab_start);
    Ok(bis.split('\n').map(py::strip).filter(|l| !l.is_empty()).collect::<Vec<_>>().join("\n"))
}

/// Prüft den entstrippten Text und schreibt `<name>.txt` und `<name>.meta.yaml` nach `auftrag.out`. Alle Prüfungen laufen VOR
/// dem ersten Schreiben.
///
/// # Errors
/// [`Fehler::StartFehlt`], [`Fehler::ZuKurz`], [`Fehler::ErwarteFehlt`], [`Fehler::UnbekannteEntity`], [`Fehler::Datei`].
pub fn friere_ein(auftrag: &Auftrag, roh: &str) -> Result<Gefroren, Fehler> {
    let body = entstrippen(roh, &auftrag.start, auftrag.ende.as_deref())?;
    let laenge = py::laenge(&body);
    if i64::try_from(laenge).unwrap_or(i64::MAX) < auftrag.min_laenge {
        return Err(Fehler::ZuKurz { laenge, minimum: auftrag.min_laenge });
    }
    let fehlend: Vec<String> = auftrag.erwarte.iter().filter(|e| !body.contains(e.as_str())).cloned().collect();
    if !fehlend.is_empty() {
        return Err(Fehler::ErwarteFehlt { fehlend });
    }
    let ebene_text = match auftrag.ebene {
        Ebene::Paragraph => "ganzer Paragraph (Konvention, siehe sources/README.md)",
        Ebene::Schreiben => "ganzes BMF-Schreiben (Randziffern, Abschnitte und Beispiele wie abgerufen)",
    };
    let kopf = format!(
        "Quelle: {}\nAbgerufen: {}\nNorm: {}\nFassung: {}\nEinfrier-Ebene: {ebene_text}.\n\
         Hinweis: geschuetzte Leerzeichen (U+00A0) durch normale ersetzt.\n\n--- Wortlaut (abgerufener Ausschnitt) ---\n\n",
        auftrag.url, auftrag.abrufdatum, auftrag.norm, auftrag.fassung
    );
    let text = format!("{kopf}{body}\n");
    let sha = format!("{:x}", Sha256::digest(text.as_bytes()));
    let txt_pfad = auftrag.out.join(format!("{}.txt", auftrag.name));
    fs::write(&txt_pfad, &text).map_err(Fehler::datei(&txt_pfad))?;
    let meta_pfad = auftrag.out.join(format!("{}.meta.yaml", auftrag.name));
    fs::write(&meta_pfad, meta(auftrag, &sha)).map_err(Fehler::datei(&meta_pfad))?;
    let meldung = format!(
        "{}: {laenge} Zeichen, {} Passage(n) verifiziert, sha256 {}",
        auftrag.name,
        auftrag.erwarte.len(),
        sha.get(..16).unwrap_or(&sha)
    );
    Ok(Gefroren { zeichen: laenge, sha256: sha, meldung })
}

/// Der Inhalt von `<name>.meta.yaml`, Zeile für Zeile wie im Python-Skript (die Werte stehen ungeschützt zwischen den
/// Anführungszeichen: ein `"` in `--norm` ergäbe dort wie hier ungültiges YAML).
fn meta(a: &Auftrag, sha: &str) -> String {
    let verwendet = if a.verwendet_in.is_empty() { "[]".to_owned() } else { format!("[\"{}\"]", a.verwendet_in) };
    let ebene = match a.ebene {
        Ebene::Paragraph => "paragraph",
        Ebene::Schreiben => "schreiben",
    };
    format!(
        "dokument:\n  norm_uri: \"{}\"\n  norm: \"{}\"\n  fassung: \"{}\"\n  quelle_url: \"{}\"\n  abrufdatum: \"{}\"\n  datei: \"{}.txt\"\n  \
         sha256: \"{sha}\"\n  authority: {}\n  redistributable: true\n  einfrier_ebene: {ebene}\n  verwendet_in: {verwendet}\n",
        a.norm_uri, a.norm, a.fassung, a.url, a.abrufdatum, a.name, a.authority
    )
}
