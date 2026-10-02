//! Persistenz der Store-Datei: [`lade`] liest JSON wie der Python-Server
//! (`produkt/haut/api.py:138-143`, `lade_fall` -> `json.load`), [`speichere`] schreibt JSON
//! atomar mit Modus 0600 (`api.py:146-162`, `speichere_fall`). `produkt/store/store.py:84-87`
//! (`lade`, YAML) ruft der Server nicht auf; YAML wies DEL/C1 ab und aenderte NEL und Leerzeichen
//! an U+2028/U+2029 still (Vault `decisions/rust-liest-fallakte-als-json-wie-der-server`).
//!
//! PARITAET: `json.load` liest `NaN`, `Infinity`, `1e400` und Ganzzahlen ausserhalb von
//! `i64`/`u64` still als Zahl. [`lade`] weist sie unter `events` mit Zeile, Spalte und
//! [`Sperrform`] ab, statt eine falsche Zahl zu laden (Vault
//! `decisions/fallakte-mit-nan-oder-ueberlauf-sperrt-mit-namen`).
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::store::StoreDatei;

/// Fehler beim Laden/Speichern der Store-Datei.
#[derive(Debug, thiserror::Error)]
pub enum PersistenzFehler {
    #[error("store-datei {0} konnte nicht gelesen werden: {1}")]
    Lesen(PathBuf, std::io::Error),
    #[error("store-datei {0} {}", format_grund(.1))]
    Format(PathBuf, serde_json::Error),
    #[error("store-datei {pfad} enthaelt {form:?} in Zeile {zeile}, Spalte {spalte}")]
    Sperrform {
        pfad: PathBuf,
        zeile: usize,
        spalte: usize,
        form: Sperrform,
    },
    #[error("store-datei konnte nicht geschrieben werden: {0}")]
    Schreiben(#[from] std::io::Error),
    #[error("store-datei konnte nicht serialisiert werden: {0}")]
    Serialisieren(#[from] serde_json::Error),
}

/// Was an der Datei wirklich falsch ist. `serde_json` meldet mit demselben Fehlertyp ein
/// Syntaxproblem (kein JSON), einen abgeschnittenen Text und gueltiges JSON mit falscher Form
/// (fehlendes oder doppeltes Feld, falscher Typ); nur die ersten beiden sind „kein gueltiges JSON".
/// Zeile und Spalte stehen in `{e}`.
fn format_grund(e: &serde_json::Error) -> String {
    use serde_json::error::Category;
    match e.classify() {
        Category::Syntax => format!("ist kein gueltiges JSON: {e}"),
        Category::Eof => format!("ist kein gueltiges JSON, der Text endet zu frueh: {e}"),
        Category::Data => {
            format!("ist gueltiges JSON, hat aber nicht die Form einer Fallakte: {e}")
        }
        Category::Io => format!("konnte nicht als JSON gelesen werden: {e}"),
    }
}

/// Zahlform, die `json.load` still als Zahl liest und [`lade`] unter `events` abweist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sperrform {
    /// `NaN`.
    NaN,
    /// `Infinity` oder `-Infinity`.
    Infinity,
    /// Kommazahl ausserhalb von `f64`, z. B. `1e400`.
    KommazahlUeberlauf,
    /// Ganzzahl ausserhalb `i64::MIN..=u64::MAX` in einem Event; `serde_json` laese sie still als
    /// gerundetes `f64`.
    GanzzahlUeberlauf,
}

/// Laedt eine Store-Datei als JSON, KEINE Schema-Pruefung — eine strukturell falsche Datei liefert
/// hier einen Fehler (Serde braucht die Pflichtfelder), eine inhaltlich falsche (z.B. unbekanntes
/// `feld_id`) laedt anstandslos durch, genau wie in Python.
///
/// # Errors
/// [`PersistenzFehler::Lesen`]/[`PersistenzFehler::Format`]/[`PersistenzFehler::Sperrform`].
///
/// ```
/// let dir = std::env::temp_dir().join(format!("taxgraph-doctest-lade-{}", std::process::id()));
/// let pfad = dir.join("fall.json");
/// let datei = store::Store::leer(2025, Some("demo-1".to_string())).into_datei();
/// store::speichere(&pfad, &datei).unwrap();
/// assert_eq!(store::lade(&pfad).unwrap().fall_id.as_deref(), Some("demo-1"));
/// assert!(store::lade(&dir.join("fehlt.json")).is_err());
/// std::fs::remove_dir_all(&dir).ok();
/// ```
pub fn lade(pfad: &Path) -> Result<StoreDatei, PersistenzFehler> {
    let text = std::fs::read_to_string(pfad)
        .map_err(|e| PersistenzFehler::Lesen(pfad.to_path_buf(), e))?;
    if let Some((stelle, form)) = finde_sperrform(&text) {
        let vorher = text.get(..stelle).unwrap_or_default();
        let zeilenanfang = vorher.rsplit('\n').next().unwrap_or_default();
        return Err(PersistenzFehler::Sperrform {
            pfad: pfad.to_path_buf(),
            zeile: vorher.matches('\n').count() + 1,
            spalte: zeilenanfang.chars().count() + 1,
            form,
        });
    }
    serde_json::from_str(&text).map_err(|e| PersistenzFehler::Format(pfad.to_path_buf(), e))
}

/// Erste [`Sperrform`] unter `events`, ausserhalb von Strings, mit ihrem Byte-Versatz.
///
/// ponytail: eigener Vorab-Scan statt `serde_json` mit `arbitrary_precision` (D1,
/// `domain/src/py_wert.rs:508`). Er kennt nur Strings, Klammertiefe und den obersten Schluessel
/// und prueft darum jedes Token unter `events`, auch in `signal` und in unbekannten Schluesseln.
/// Ausserhalb entscheidet `serde_json` allein: `veranlagungszeitraum` laedt exakt als `i128`, eine
/// Ganzzahl ueber `u64` in `vorjahr_referenz` gerundet als `Gleit`. Upgrade: voller Pfad je
/// Fundstelle, wenn eine weitere Stelle sperren soll.
fn finde_sperrform(text: &str) -> Option<(usize, Sperrform)> {
    let mut zeichen = text.char_indices().peekable();
    let mut tiefe = 0_usize;
    let mut in_events = false;
    while let Some((start, c)) = zeichen.next() {
        match c {
            '"' => {
                let mut ende = text.len();
                while let Some((i, c)) = zeichen.next() {
                    match c {
                        '\\' => {
                            zeichen.next();
                        }
                        '"' => {
                            ende = i;
                            break;
                        }
                        _ => {}
                    }
                }
                let danach = text.get(ende + 1..).unwrap_or_default();
                if tiefe == 1 && danach.trim_start().starts_with(':') {
                    let schluessel = text
                        .get(start..=ende)
                        .and_then(|s| serde_json::from_str::<String>(s).ok());
                    in_events = schluessel.as_deref() == Some("events");
                }
            }
            '{' | '[' => tiefe += 1,
            '}' | ']' => tiefe = tiefe.saturating_sub(1),
            'N' | 'I' | '-' | '0'..='9' if in_events => {
                let mut ende = text.len();
                while let Some(&(i, c)) = zeichen.peek() {
                    if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
                        ende = i;
                        break;
                    }
                    zeichen.next();
                }
                let token = text.get(start..ende).unwrap_or_default();
                if let Some(form) = sperrform(token) {
                    return Some((start, form));
                }
            }
            _ => {}
        }
    }
    None
}

/// Die [`Sperrform`] eines Zahl- oder Wort-Tokens.
fn sperrform(token: &str) -> Option<Sperrform> {
    match token {
        "NaN" => Some(Sperrform::NaN),
        "Infinity" | "-Infinity" => Some(Sperrform::Infinity),
        _ if token.contains(['.', 'e', 'E']) => token
            .parse::<f64>()
            .is_ok_and(f64::is_infinite)
            .then_some(Sperrform::KommazahlUeberlauf),
        _ => {
            let ziffern = token.strip_prefix('-').unwrap_or(token);
            let ganzzahl = !ziffern.is_empty() && ziffern.bytes().all(|b| b.is_ascii_digit());
            let ausserhalb = token.parse::<i64>().is_err() && token.parse::<u64>().is_err();
            (ganzzahl && ausserhalb).then_some(Sperrform::GanzzahlUeberlauf)
        }
    }
}

/// Schreibt die Store-Datei atomar (`produkt/haut/api.py:146-166`, `speichere_fall`): Tempfile im
/// selben Verzeichnis, fsync, `rename` — ein Leser sieht nie einen halb geschriebenen Zustand.
/// Scheitert ein Schritt nach dem Anlegen, kommt das Tempfile wieder weg. Modus 0600 ab
/// Neuanlage: hier stehen Steuer-ID, Einkommen und IBAN.
///
/// # Errors
/// [`PersistenzFehler::Schreiben`]/[`PersistenzFehler::Serialisieren`], wenn Tempfile, Schreiben
/// oder `rename` scheitern.
///
/// ```
/// use std::os::unix::fs::PermissionsExt;
/// let dir = std::env::temp_dir().join(format!("taxgraph-doctest-speichere-{}", std::process::id()));
/// let pfad = dir.join("fall.json");
/// store::speichere(&pfad, &store::Store::leer(2025, None).into_datei()).unwrap();
/// let modus = std::fs::metadata(&pfad).unwrap().permissions().mode() & 0o777;
/// assert_eq!(modus, 0o600);
/// std::fs::remove_dir_all(&dir).ok();
/// ```
pub fn speichere(pfad: &Path, datei: &StoreDatei) -> Result<(), PersistenzFehler> {
    let verzeichnis = pfad
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(verzeichnis)?;
    let dateiname = pfad.file_name().and_then(|n| n.to_str()).unwrap_or("store");
    let temp_pfad = verzeichnis.join(format!(".{dateiname}.{}.tmp", std::process::id()));
    let tmp = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp_pfad)?;
    fuelle_und_ersetze(tmp, &temp_pfad, pfad, datei).inspect_err(|_| {
        // Die eigene Teil-Datei, nie die Akte. Scheitert auch das, zaehlt der erste Fehler.
        let _ = std::fs::remove_file(&temp_pfad);
    })
}

/// Rest von [`speichere`] ab dem eigenen Tempfile: schreiben, fsync, schliessen, `rename`.
fn fuelle_und_ersetze(
    mut tmp: std::fs::File,
    temp_pfad: &Path,
    pfad: &Path,
    datei: &StoreDatei,
) -> Result<(), PersistenzFehler> {
    serde_json::to_writer(&mut tmp, datei)?;
    tmp.sync_all()?;
    drop(tmp);
    std::fs::rename(temp_pfad, pfad)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{lade, speichere, PersistenzFehler, Sperrform};
    use crate::store::{StoreDatei, Veranlagungsjahr};

    fn testdatei() -> StoreDatei {
        StoreDatei {
            version: 1,
            veranlagungszeitraum: Veranlagungsjahr(2025),
            fall_id: Some("demo-1".to_string()),
            scheibe: None,
            user_id: None,
            events: Vec::new(),
            snapshots: Vec::new(),
            vorjahr_referenz: None,
        }
    }

    #[test]
    fn speichere_und_lade_roundtrip_mit_modus_0600() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("fall.json");
        let original = testdatei();
        speichere(&pfad, &original).unwrap();
        let modus = std::fs::metadata(&pfad).unwrap().permissions().mode() & 0o777;
        assert_eq!(modus, 0o600);
        let geladen = lade(&pfad).unwrap();
        assert_eq!(geladen.veranlagungszeitraum, Veranlagungsjahr(2025));
        assert_eq!(geladen.fall_id.as_deref(), Some("demo-1"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn lade_liest_json_ohne_optionale_felder() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-json-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("roh.json");
        std::fs::write(
            &pfad,
            r#"{"version":1,"veranlagungszeitraum":2026,"events":[],"snapshots":[]}"#,
        )
        .unwrap();
        let geladen = lade(&pfad).unwrap();
        assert_eq!(geladen.veranlagungszeitraum, Veranlagungsjahr(2026));
        std::fs::remove_dir_all(&dir).ok();
    }

    /// `vorjahr_referenz` ist `PyWert` (K2): der Rueckweg ist verlustfrei, NaN wird ein Fehler,
    /// nie ein stilles `null`.
    #[test]
    fn vorjahr_referenz_roundtrip_und_nan() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-vorjahr-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("fall.json");
        let mut datei = testdatei();
        let referenz = serde_json::json!({"verlustvortrag_bestand": {"wert": 150_000}});
        datei.vorjahr_referenz = Some(referenz.into());
        speichere(&pfad, &datei).unwrap();
        assert_eq!(
            lade(&pfad).unwrap().vorjahr_referenz,
            datei.vorjahr_referenz
        );
        datei.vorjahr_referenz = Some(domain::PyWert::Gleit(f64::NAN));
        assert!(serde_json::to_value(&datei).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    /// N2: scheitert das Schreiben nach dem Anlegen des Tempfiles (hier NaN, wie
    /// `allow_nan=False` in `speichere_fall`), kommt die eigene Teil-Datei weg. Die Akte bleibt
    /// byte-gleich.
    #[test]
    fn gescheitertes_speichern_laesst_keine_teil_datei() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-teil-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("fall.json");
        let mut datei = testdatei();
        speichere(&pfad, &datei).unwrap();
        let vorher = std::fs::read(&pfad).unwrap();
        datei.vorjahr_referenz = Some(domain::PyWert::Gleit(f64::NAN));
        assert!(matches!(
            speichere(&pfad, &datei),
            Err(PersistenzFehler::Serialisieren(_))
        ));
        assert_eq!(std::fs::read(&pfad).unwrap(), vorher);
        let namen: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(namen, ["fall.json"]);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Eine Akte mit einem Event; `wert` steht roh in Zeile 3 ab Spalte 50, hinter einem `ä`.
    fn akte_mit(vz: &str, wert: &str) -> String {
        concat!(
            r#"{"version":1,"veranlagungszeitraum":VZ,"snapshots":[],"events":["#,
            "\n",
            r#"{"event_id":"ID","ts":"2026-01-01T00:00:00+00:00","feld_id":"ep_arbeitstage","#,
            "\n",
            r#""signal":{"signal_1":null,"signal_2":"ä"},"wert":WERT,"zustand":"bestaetigt","#,
            r#""herkunft":{"herkunft":"mensch","pruef_tiefe":"ungeprueft","haftung":"nutzer"},"#,
            r#""schreiber":"julius","ersetzt":null}]}"#
        )
        .replace("VZ", vz)
        .replace("ID", &"0".repeat(64))
        .replace("WERT", wert)
    }

    /// B4 (Vault `decisions/fallakte-mit-nan-oder-ueberlauf-sperrt-mit-namen`): `CPython` liest
    /// diese Formen mit `json.load` still als Zahl (`api.py:143`). `lade` weist sie in einem
    /// Event-Wert mit Zeile, Spalte in Zeichen und Form ab. Ohne den Scan sperrt `serde_json` die
    /// Akte ohne Namen und liest die Ganzzahlen als gerundetes `f64`.
    #[test]
    fn b4_nan_infinity_und_ueberlauf_sperren_mit_namen() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-b4-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("b4.json");
        let falsch: Vec<String> = [
            ("NaN", Sperrform::NaN),
            ("Infinity", Sperrform::Infinity),
            ("-Infinity", Sperrform::Infinity),
            ("1e400", Sperrform::KommazahlUeberlauf),
            ("-1e400", Sperrform::KommazahlUeberlauf),
            ("18446744073709551616", Sperrform::GanzzahlUeberlauf),
            ("-9223372036854775809", Sperrform::GanzzahlUeberlauf),
        ]
        .into_iter()
        .filter_map(|(token, form)| {
            std::fs::write(&pfad, akte_mit("2025", token)).unwrap();
            match lade(&pfad) {
                Err(PersistenzFehler::Sperrform {
                    zeile: 3,
                    spalte: 50,
                    form: ist,
                    ..
                }) if ist == form => None,
                anders => Some(format!("{token}: {anders:?}")),
            }
        })
        .collect();
        let meldung = lade(&pfad).err().map(|e| e.to_string());
        std::fs::remove_dir_all(&dir).ok();
        assert!(falsch.is_empty(), "{falsch:#?}");
        let ende = "enthaelt GanzzahlUeberlauf in Zeile 3, Spalte 50";
        assert!(
            meldung.as_deref().is_some_and(|m| m.ends_with(ende)),
            "{meldung:?}"
        );
    }

    /// Grenze des Scans: Zahlformen in einem String (auch hinter `\"`) und eine Ganzzahl ueber
    /// `u64` ausserhalb von `events` laden wie bisher, auch hinter `events` (`vorjahr_referenz`,
    /// gerundet als `Gleit`). Eine reale Akte traegt einen 38-stelligen `veranlagungszeitraum`.
    #[test]
    fn zahlform_im_text_und_grosser_vz_laden() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-grenze-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("grenze.json");
        let vz = "99999999999999999999999999999999999999";
        let text = r#""NaN" -Infinity 1e400 18446744073709551616"#;
        let akte = akte_mit(vz, &format!("{text:?}"))
            .replace("]}", r#"],"vorjahr_referenz":18446744073709551617}"#);
        std::fs::write(&pfad, akte).unwrap();
        let datei = lade(&pfad).unwrap();
        std::fs::remove_dir_all(&dir).ok();
        assert_eq!(
            datei.veranlagungszeitraum,
            Veranlagungsjahr(vz.parse().unwrap())
        );
        assert_eq!(datei.events[0].wert, domain::PyWert::Text(text.to_string()));
        assert_eq!(
            datei.vorjahr_referenz,
            Some(domain::PyWert::Gleit(18_446_744_073_709_551_616.0))
        );
    }

    /// Laden prueft nie (Vault `decisions/geldfeld-ohne-minus-im-schema-lehnt-minus-bei-eingabe-ab`
    /// Punkt 3, `decisions/zahl-ausserhalb-des-bereichs-wird-beim-speichern-abgewiesen-die-null-
    /// nicht` Punkt 3): eine Akte mit einem Minus in einem `nicht_negativ`-Feld oder einer Zahl
    /// ausserhalb von `bereich` laedt wie bisher.
    #[test]
    fn akte_mit_minus_und_wert_ausserhalb_bereich_laedt_weiter() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-minus-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("minus.json");
        for (feld, wert) in [
            ("hh_handwerker_betrag", "-5000000"),
            ("fam_anzahl_kinder", "99"),
        ] {
            let akte = akte_mit("2025", wert).replace("ep_arbeitstage", feld);
            std::fs::write(&pfad, akte).unwrap();
            let datei = lade(&pfad).unwrap();
            assert_eq!(datei.events[0].feld_id, feld);
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Wortlaut von `PersistenzFehler::Format`: „kein gueltiges JSON" nur fuer Syntax und
    /// abgeschnittenen Text; gueltiges JSON mit falscher Form (fehlendes Feld, falscher Typ,
    /// doppeltes Feld) sagt das so. Jede Meldung nennt Zeile und Spalte.
    #[test]
    fn format_meldung_unterscheidet_syntax_abbruch_und_form() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-wortlaut-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("wortlaut.json");
        let falsch: Vec<String> = [
            ("{\"version\":1,}", "ist kein gueltiges JSON: ", true),
            (
                "{\"version\":1,",
                "kein gueltiges JSON, der Text endet zu frueh: ",
                true,
            ),
            (
                "{\"foo\":1}",
                "ist gueltiges JSON, hat aber nicht die Form einer Fallakte: ",
                false,
            ),
            (
                "[]",
                "ist gueltiges JSON, hat aber nicht die Form einer Fallakte: ",
                false,
            ),
            (
                r#"{"version":"eins","veranlagungszeitraum":2025,"snapshots":[],"events":[]}"#,
                "ist gueltiges JSON, hat aber nicht die Form einer Fallakte: ",
                false,
            ),
        ]
        .into_iter()
        .filter_map(|(text, erwartet, ist_kein_json)| {
            std::fs::write(&pfad, text).unwrap();
            let meldung = lade(&pfad).err().map(|e| e.to_string()).unwrap_or_default();
            let ok = meldung.contains(erwartet)
                && meldung.contains("line ")
                && meldung.contains("column ")
                && meldung.contains("kein gueltiges JSON") == ist_kein_json;
            (!ok).then(|| format!("{text}: {meldung}"))
        })
        .collect();
        std::fs::remove_dir_all(&dir).ok();
        assert!(falsch.is_empty(), "{falsch:#?}");
    }

    /// Grenze: ein Struct-Feld zweimal, oben (`veranlagungszeitraum`) oder im Event (`wert`),
    /// sperrt die ganze Akte als `PersistenzFehler::Format` mit dem Feldnamen. `json.load` laedt
    /// sie, der letzte Wert gewinnt. Gemessen 2026-10-02: 0 von 192 realen Akten tragen einen
    /// doppelten Schluessel. Ein doppelter Schluessel in einem `wert`-Objekt laedt wie in Python
    /// (D8, `domain/src/py_wert.rs`).
    #[test]
    fn doppeltes_struct_feld_sperrt_mit_namen() {
        let dir = std::env::temp_dir().join(format!(
            "taxgraph-store-test-persistenz-doppelt-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let pfad = dir.join("doppelt.json");
        let falsch: Vec<String> = [
            ("wert", akte_mit("2025", r#"1,"wert":2"#)),
            (
                "veranlagungszeitraum",
                akte_mit(r#"2025,"veranlagungszeitraum":2026"#, "1"),
            ),
        ]
        .into_iter()
        .filter_map(|(feld, akte)| {
            std::fs::write(&pfad, akte).unwrap();
            match lade(&pfad) {
                Err(PersistenzFehler::Format(_, e))
                    if e.to_string()
                        .starts_with(&format!("duplicate field `{feld}`")) =>
                {
                    None
                }
                anders => Some(format!("{feld}: {anders:?}")),
            }
        })
        .collect();
        std::fs::remove_dir_all(&dir).ok();
        assert!(falsch.is_empty(), "{falsch:#?}");
    }
}
