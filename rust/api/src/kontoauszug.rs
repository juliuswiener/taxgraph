//! `api.kontoauszug` (`api.py:956`): ein Kontoauszug (CSV, JSON oder PDF) wird gelesen, jede
//! Ausgabe mit sicherer Kategorie wird EIN vorläufiger Vorschlag (`import:kontoauszug`, nie
//! bestätigt). Gelesen und geschrieben wird in der Bibliothek `eingang`; diese Schicht prüft die
//! Form der Anfrage, ruft sie in der Reihenfolge von Python auf und schreibt die Akte.
//!
//! Eine Buchung, deren Betrag nicht lesbar ist oder ab 10^10 Cent liegt, fliegt einzeln raus und
//! zählt in `verworfen`; den Auszug ganz abzulehnen ist ausdrücklich nicht gewollt (Vault
//! `decisions/kontoauszug-betrag-cent-genau-oder-verworfen`).
use domain::py_strip;
use eingang::kontoauszug::{
    aus_json, hinweis_verworfen, parse_csv, parse_pdf_zeilen, uebernehme,
    verwirf_unlesbare_betraege, verwirf_unlesbare_betraege_json, Klassifikator, KontoauszugFehler,
    Transaktion, LLM_AUFRUFE_HOECHSTZAHL,
};
use eingang::ocr::{lies_kontoauszug_pdf, OcrFehler};
use eingang::vorschlag::SchreibFehler;
use serde_json::{json, Map, Value};
use store::BindungNachschlag;

use crate::antwort::Antwort;
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::python::{typname, wahr};
use crate::zustand::Zustand as Dienst;

/// Ein fehlender Schlüssel des Rumpfs ist `None` (`body.get(...)`).
static NICHTS: Value = Value::Null;

/// Ab dieser Confidence zählt eine PDF-Zeile (`parse_pdf_zeilen(schwelle=0.6)`).
const PDF_SCHWELLE: f64 = 0.6;

/// Ein Fehler beim Lesen des Auszugs: Pythons Klasse und Text, 500.
fn lese_fehler(e: &KontoauszugFehler) -> ApiFehler {
    match e {
        KontoauszugFehler::Csv(c) => ApiFehler::unerwartet("Error", c.to_string()),
        KontoauszugFehler::BetragUeberlauf(t) => ApiFehler::unerwartet("OverflowError", t.clone()),
        KontoauszugFehler::TransaktionUngueltig { meldung, .. } => {
            ApiFehler::unerwartet("AttributeError", meldung.clone())
        }
        // Der Store hat einen Vorschlag abgewiesen: in Python ein `ValueError` aus `append_event`,
        // das `api.kontoauszug` nicht fängt (P6: hier 500, nicht 422 wie bei `/event`).
        KontoauszugFehler::Schreiben(SchreibFehler::Abweisung(a)) => {
            ApiFehler::unerwartet("ValueError", a.to_string())
        }
        KontoauszugFehler::BetragUngueltig { .. }
        | KontoauszugFehler::Schreiben(SchreibFehler::Konstante) => {
            ApiFehler::unerwartet("ValueError", e.to_string())
        }
    }
}

/// `json.loads(text)` wie `CPython`, soweit es hier sichtbar wird: `NaN`, `Infinity` und
/// `-Infinity` sind Zahlen (Python liest sie, `serde_json` nicht), eine Kommazahl über `f64` ist
/// `inf`, und eine Ganzzahl mit mehr als 4300 Ziffern ist ein `ValueError`.
///
/// `serde_json` kennt kein `NaN`: die drei Wörter und eine überlaufende Kommazahl stehen danach als
/// Text (`"NaN"`, `"Infinity"`, `"-Infinity"`) im Wert. Das genügt dort, wo es zählt: `int()` auf
/// den Text scheitert wie auf die Zahl (`ValueError` hier, `ValueError`/`OverflowError` dort), und
/// die Buchung fliegt aus dem Auszug.
///
/// ponytail: steht ein solches Wort im Datum einer Buchung, die gebucht wird, oder im Zweck einer
/// Ausgabe, scheitert Python (500: `speichere_fall` schreibt mit `allow_nan=False`, und `.lower()`
/// kennt keine Kommazahl), Rust sieht Text und übernimmt die Buchung (dokumentierte Abweichung 5
/// in `api_http_paritaet.rs`). Ebenso steht eine Ganzzahl über `u64` als Kommazahl da (Python: exakte
/// `int`; ab 309 Ziffern Text), und ein einzelnes Surrogat-Escape (`"\ud800"`) weist `serde_json` mit 400 ab,
/// Python scheitert erst beim Schreiben der Akte (500). Upgrade: ein eigener Leser, der `PyWert` mit `NaN` liefert.
fn json_laden(text: &str) -> Result<Value, ()> {
    let mut aus = String::with_capacity(text.len());
    let (mut in_text, mut maskiert) = (false, false);
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let mut weiter = c.len_utf8();
        if in_text {
            match (maskiert, c) {
                (true, _) => maskiert = false,
                (false, '\\') => maskiert = true,
                (false, '"') => in_text = false,
                _ => {}
            }
            aus.push(c);
        } else if c == '"' {
            in_text = true;
            aus.push(c);
        } else if let Some(wort) = ["-Infinity", "Infinity", "NaN"]
            .into_iter()
            .find(|w| rest.starts_with(w) && wort_ende(rest, w.len()))
        {
            aus.push('"');
            aus.push_str(wort);
            aus.push('"');
            weiter = wort.len();
        } else if c == '-' || c.is_ascii_digit() {
            let ende = rest
                .find(|z: char| !(z.is_ascii_digit() || matches!(z, '-' | '+' | '.' | 'e' | 'E')))
                .unwrap_or(rest.len());
            let zahl = rest.get(..ende).unwrap_or_default();
            let ganzzahl = zahl.bytes().all(|z| z.is_ascii_digit() || z == b'-');
            if ganzzahl && zahl.bytes().filter(u8::is_ascii_digit).count() > 4300 {
                return Err(());
            }
            // Über `f64::MAX` (auch eine Ganzzahl mit mehr als 308 Ziffern) liest `serde_json` nichts.
            if zahl.parse::<f64>().is_ok_and(f64::is_infinite) {
                aus.push_str(if zahl.starts_with('-') {
                    "\"-Infinity\""
                } else {
                    "\"Infinity\""
                });
            } else {
                aus.push_str(zahl);
            }
            weiter = ende;
        } else {
            aus.push(c);
        }
        rest = rest.get(weiter..).unwrap_or_default();
    }
    serde_json::from_str(&aus).map_err(|_| ())
}

/// Steht nach `laenge` Bytes von `rest` ein Trenner (oder nichts)?
fn wort_ende(rest: &str, laenge: usize) -> bool {
    rest.get(laenge..)
        .and_then(|r| r.chars().next())
        .is_none_or(|c| c.is_ascii_whitespace() || matches!(c, ',' | ']' | '}'))
}

/// `base64.b64decode(text, validate=True)`: nur das Alphabet, Auffüllung `=` nur am Ende und
/// vollständig, nichts danach, kein Leerraum. Überzählige Bits der letzten Gruppe nimmt Python an.
fn base64_streng(text: &str) -> Option<Vec<u8>> {
    let (daten, auffuellung) = text.find('=').map_or((text, ""), |p| text.split_at(p));
    if !matches!(
        (daten.len() % 4, auffuellung),
        (0, "") | (2, "==") | (3, "=")
    ) {
        return None;
    }
    let sextett = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let sextette: Vec<u8> = daten.bytes().map(sextett).collect::<Option<_>>()?;
    let mut aus = Vec::with_capacity(sextette.len() / 4 * 3 + 2);
    for gruppe in sextette.chunks(4) {
        let puffer = gruppe.iter().fold(0_u32, |p, s| (p << 6) | u32::from(*s));
        // Eine Gruppe aus 4, 3 oder 2 Sextetten ergibt 3, 2 oder 1 Byte.
        let n = gruppe.len() - 1;
        let bytes = (puffer << (6 * (4 - gruppe.len()))).to_be_bytes();
        aus.extend_from_slice(bytes.get(1..=n)?);
    }
    Some(aus)
}

/// Das PDF auf die Platte (der Pfad trägt die rohen Daten, bis `lies_kontoauszug_pdf` fertig ist) und
/// lesen; die Datei verschwindet danach in jedem Fall.
fn pdf_lesen(bytes: &[u8]) -> Result<(Vec<Transaktion>, usize), ApiFehler> {
    let datei = tempfile::Builder::new()
        .suffix(".pdf")
        .tempfile()
        .map_err(|e| ApiFehler::unerwartet("OSError", e.to_string()))?;
    std::fs::write(datei.path(), bytes)
        .map_err(|e| ApiFehler::unerwartet("OSError", e.to_string()))?;
    let pfad = datei.path().to_string_lossy().into_owned();
    let (text, konfidenz) = lies_kontoauszug_pdf(&pfad).map_err(|e| match e {
        OcrFehler::Zeitlimit { .. } | OcrFehler::ZuAufwendig(_) | OcrFehler::NichtLesbar => {
            ApiFehler::status(422, format!("Kontoauszug nicht lesbar: {e}"))
        }
        OcrFehler::Start { .. } => ApiFehler::unerwartet("FileNotFoundError", e.to_string()),
        OcrFehler::KeinUtf8(_) => ApiFehler::unerwartet("UnicodeDecodeError", e.to_string()),
        OcrFehler::KeinBild => ApiFehler::unerwartet("IndexError", e.to_string()),
        OcrFehler::Tsv(_) => ApiFehler::unerwartet("Error", e.to_string()),
        OcrFehler::Io(_) => ApiFehler::unerwartet("OSError", e.to_string()),
    })?;
    Ok(parse_pdf_zeilen(&text, &konfidenz, PDF_SCHWELLE))
}

/// Der Klassifikator für Buchungen, die kein Stichwort trifft (`_kontoauszug_llm_klassifikator`):
/// ohne Provider-Angaben in der Umgebung liefert jeder Aufruf `None` (unklassifiziert) — der Deckel
/// und der Zähler `llm_uebersprungen` laufen trotzdem, wie in Python.
fn klassifikator() -> Box<Klassifikator<'static>> {
    match llm::Konfiguration::aus_env() {
        Ok(konfiguration) => {
            let chat = llm::HttpChat { konfiguration };
            Box::new(move |zweck, betrag| llm::kontoauszug::klassifiziere(&chat, zweck, betrag))
        }
        Err(_) => Box::new(|_, _| None),
    }
}

/// `(body.get("format") or "").strip().lower()`: ein falscher Wert ist leer, ein wahrer Nicht-Text
/// wirft in Python `AttributeError`.
fn format_lesen(b: &Map<String, Value>) -> Result<String, ApiFehler> {
    let roh = b.get("format").unwrap_or(&NICHTS);
    if !wahr(roh) {
        return Ok(String::new());
    }
    match roh {
        Value::String(s) => Ok(py_strip(s).to_lowercase()),
        andere => Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'strip'", typname(andere)),
        )),
    }
}

/// Der JSON-Zweig: eine Liste im Rumpf gilt, Text wird als JSON gelesen, ein falscher Wert ist
/// die leere Liste.
fn json_liste(inhalt: &Value) -> Result<Vec<Value>, ApiFehler> {
    let nicht_lesbar = || ApiFehler::status(400, "json-Inhalt nicht parsebar");
    let wert = match inhalt {
        Value::Array(_) => inhalt.clone(),
        falsch if !wahr(falsch) => json!([]),
        Value::String(text) => json_laden(text).map_err(|()| nicht_lesbar())?,
        // `json.loads(5)` ist ein `TypeError`, den `api.kontoauszug` wie einen `ValueError` fängt.
        _ => return Err(nicht_lesbar()),
    };
    match wert {
        Value::Array(liste) => Ok(liste),
        _ => Err(ApiFehler::status(
            400,
            "json muss eine Liste von Transaktionen sein",
        )),
    }
}

/// Der PDF-Zweig: Text mit Inhalt, base64.
fn pdf_bytes(inhalt: &Value) -> Result<Vec<u8>, ApiFehler> {
    let fehlt = || {
        ApiFehler::status(
            400,
            "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)",
        )
    };
    let Value::String(text) = inhalt else {
        return Err(fehlt());
    };
    if py_strip(text).is_empty() {
        return Err(fehlt());
    }
    base64_streng(text)
        .ok_or_else(|| ApiFehler::status(400, "pdf-Inhalt nicht gültig base64-kodiert"))
}

/// Der Rumpf nach dem Owner-Check: Format, Inhalt, Lesen, Verwerfen, Übernehmen, Speichern.
///
/// # Errors
/// 400 für Format und Inhalt, 422 für ein PDF, das nicht zu lesen ist, 500 mit der Python-Klasse
/// für alles, was Python nicht fängt (auch eine Store-Abweisung).
pub fn kontoauszug(z: &Dienst, fall: &mut EigenerFall, body: &Value) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(fall.store())?;
    let Value::Object(b) = body else {
        return Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", typname(body)),
        ));
    };
    let fmt = format_lesen(b)?;
    let inhalt = b.get("inhalt").unwrap_or(&NICHTS);
    let (tx, n_verworfen) = match fmt.as_str() {
        "csv" => {
            let (tx, n) = parse_csv(inhalt.as_str().unwrap_or("")).map_err(|e| lese_fehler(&e))?;
            verwirf_unlesbare_betraege(tx, n)
        }
        "json" => {
            let (ok, n) = verwirf_unlesbare_betraege_json(&json_liste(inhalt)?, 0);
            let tx = aus_json(&Value::Array(ok)).map_err(|e| lese_fehler(&e))?;
            (tx, n)
        }
        "pdf" => {
            let (tx, n) = pdf_lesen(&pdf_bytes(inhalt)?)?;
            verwirf_unlesbare_betraege(tx, n)
        }
        _ => {
            return Err(ApiFehler::status(
                400,
                "format muss csv, json oder pdf sein",
            ))
        }
    };
    let katalog = z.katalog()?;
    let klassifikator = klassifikator();
    let erg = uebernehme(
        fall.store_mut(),
        &tx,
        BindungNachschlag::neu(&sb.index),
        Some(&*klassifikator),
        None,
        Some(&katalog),
    )
    .map_err(|e| lese_fehler(&e))?;
    store::speichere(fall.pfad(), fall.store().datei())?;
    let mut aus = Map::new();
    aus.insert("uebernommen".into(), json!(erg.uebernommen));
    aus.insert("transaktionen".into(), json!(tx.len()));
    aus.insert("verworfen".into(), json!(n_verworfen));
    let mut hinweise: Vec<String> = Vec::new();
    if n_verworfen > 0 {
        hinweise.push(hinweis_verworfen(n_verworfen, &fmt));
    }
    if erg.llm_uebersprungen > 0 {
        // Ohne diesen Hinweis wäre der Deckel eine stille Kürzung.
        aus.insert("llm_uebersprungen".into(), json!(erg.llm_uebersprungen));
        hinweise.push(format!(
            "{} Buchung(en) wurden NICHT automatisch eingeordnet — die Grenze von {LLM_AUFRUFE_HOECHSTZAHL} \
             Klassifikationen je Auszug war erreicht. Bitte diese Buchungen selbst zuordnen oder den \
             Auszug in kleineren Zeiträumen hochladen.",
            erg.llm_uebersprungen
        ));
    }
    if !hinweise.is_empty() {
        aus.insert("hinweis".into(), json!(hinweise.join(" ")));
    }
    Ok(Antwort::neu(200, Value::Object(aus)))
}
