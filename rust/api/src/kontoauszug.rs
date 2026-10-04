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
    aus_json, hinweis_verworfen, parse_csv, parse_pdf_zeilen, pruefe_buchungsfelder, uebernehme,
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
/// `inf`, eine Ganzzahl ausserhalb von `i64` bleibt exakt, und eine Ganzzahl mit mehr als 4300 Ziffern
/// ist ein `ValueError`.
///
/// `serde_json` kennt kein `NaN` und hält keine Ganzzahl über `u64` exakt. Die drei Wörter, eine
/// überlaufende Kommazahl und eine Ganzzahl ausserhalb von `i64` stehen darum als Text im Wert, der
/// mit der Kennzeichnung (zweiter Rückgabewert) beginnt und auf `float` oder `int` endet. Eine solche
/// Zahl ist als Betrag keine tragbare Zahl (`int()` auf den Text scheitert wie auf die Zahl, die
/// Buchung fliegt aus dem Auszug); im Datum einer Buchung und im Zweck einer Ausgabe weist
/// `eingang::kontoauszug::pruefe_buchungsfelder` sie ab (422).
///
/// ponytail: Die Kennzeichnung ist ein Text, der im Auszug nicht roh vorkommt. Ein Auszug, der sie als
/// `\u`-Escape schriebe, sähe aus wie eine solche Zahl (422 statt 200); das gibt es nur mit Absicht.
/// Ein einzelnes Surrogat-Escape (`"\ud800"`) weist `serde_json` mit 400 ab, Python scheitert erst beim
/// Schreiben der Akte (500, dokumentierte Abweichung 5c in `api_http_paritaet.rs`). Upgrade: ein eigener
/// Leser, der `PyWert` mit `NaN` liefert.
fn json_laden(text: &str) -> Result<(Value, String), ()> {
    let marke = marke_fuer(text);
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
            ersetze(&mut aus, &marke, "float");
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
            if json_ganzzahl(zahl) && zahl.parse::<i64>().is_err() {
                ersetze(&mut aus, &marke, "int");
            // Über `f64::MAX` liest `serde_json` nichts.
            } else if zahl.parse::<f64>().is_ok_and(f64::is_infinite) {
                ersetze(&mut aus, &marke, "float");
            } else {
                aus.push_str(zahl);
            }
            weiter = ende;
        } else {
            aus.push(c);
        }
        rest = rest.get(weiter..).unwrap_or_default();
    }
    serde_json::from_str(&aus)
        .map(|wert| (wert, marke))
        .map_err(|_| ())
}

/// Setzt den Text für eine Zahl, die `serde_json` nicht hält: `"<marke><typ>"`.
fn ersetze(aus: &mut String, marke: &str, typ: &str) {
    aus.push('"');
    aus.push_str(marke);
    aus.push_str(typ);
    aus.push('"');
}

/// Die Kennzeichnung der Zahlen, die `json_laden` durch Text ersetzt: sie kommt im Auszug nicht vor.
fn marke_fuer(text: &str) -> String {
    let mut marke = String::from("§nicht_tragbar:");
    while text.contains(&marke) {
        marke.push('_');
    }
    marke
}

/// Eine Ganzzahl in der Schreibweise von JSON (`-?(0|[1-9][0-9]*)`); `-`, `--5` und `007` sind es nicht.
fn json_ganzzahl(zahl: &str) -> bool {
    let ziffern = zahl.strip_prefix('-').unwrap_or(zahl);
    !ziffern.is_empty()
        && ziffern.bytes().all(|b| b.is_ascii_digit())
        && (ziffern.len() == 1 || !ziffern.starts_with('0'))
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
        // `BildUmwandlung`: pdftoppm scheitert an der Datei, die pdftotext angenommen hat — der Nutzer kann
        // eine andere hochladen.
        OcrFehler::Zeitlimit { .. }
        | OcrFehler::ZuAufwendig(_)
        | OcrFehler::NichtLesbar
        | OcrFehler::BildUmwandlung => {
            ApiFehler::status(422, format!("Kontoauszug nicht lesbar: {e}"))
        }
        // Ein fehlendes Hilfsprogramm ist ein Betriebsproblem, nicht die Datei des Nutzers: 503 wie in
        // Python (`api.kontoauszug`, `except FileNotFoundError`), Text wortgleich, ohne Ausnahme-Typ.
        // `OcrNichtVerfuegbar`: tesseract endet mit Fehlercode (etwa ohne `deu`-Daten) — ebenfalls 503.
        OcrFehler::Start { .. } | OcrFehler::OcrNichtVerfuegbar => {
            ApiFehler::status(503, format!("PDF-Auslesen ist gerade nicht möglich: {e}"))
        }
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
/// die leere Liste. Zweiter Wert: die Kennzeichnung der Zahlen, die `json_laden` ersetzt hat (nur
/// beim Text).
fn json_liste(inhalt: &Value) -> Result<(Vec<Value>, Option<String>), ApiFehler> {
    let nicht_lesbar = || ApiFehler::status(400, "json-Inhalt nicht parsebar");
    let (wert, marke) = match inhalt {
        Value::Array(_) => (inhalt.clone(), None),
        falsch if !wahr(falsch) => (json!([]), None),
        Value::String(text) => {
            let (wert, marke) = json_laden(text).map_err(|()| nicht_lesbar())?;
            (wert, Some(marke))
        }
        // `json.loads(5)` ist ein `TypeError`, den `api.kontoauszug` wie einen `ValueError` fängt.
        _ => return Err(nicht_lesbar()),
    };
    match wert {
        Value::Array(liste) => Ok((liste, marke)),
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
            let (liste, marke) = json_liste(inhalt)?;
            let (ok, n) = verwirf_unlesbare_betraege_json(&liste, 0);
            // Datum und Zweck, die die Akte nicht hält: 422 (`api.kontoauszug`, vorher 500 beim Schreiben).
            pruefe_buchungsfelder(&ok, marke.as_deref()).map_err(|text| {
                ApiFehler::status(422, format!("Kontoauszug nicht lesbar: {text}"))
            })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const MARKE: &str = "§nicht_tragbar:";

    /// Die Zahl, die `json_laden` durch Text ersetzt hat.
    fn ersatz(typ: &str) -> Value {
        json!(format!("{MARKE}{typ}"))
    }

    /// `json_laden` in einem eigenen Thread mit Frist: eine Schleife, die nicht endet, macht den Test
    /// rot (Auftrag 8, Mutanten D185/D187), statt den Lauf einzufrieren.
    fn laden(text: &str) -> Option<Value> {
        let (tx, rx) = std::sync::mpsc::channel();
        let eingabe = text.to_owned();
        std::thread::spawn(move || {
            let _ = tx.send(json_laden(&eingabe).ok().map(|(wert, _)| wert));
        });
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap_or_else(|_| {
                let anfang: String = text.chars().take(40).collect();
                panic!("json_laden endet nicht: {anfang:?}")
            })
    }

    /// `base64.b64decode(text, validate=True)` aus `CPython` 3.14: gleiche Eingaben, gleiche Ausgänge
    /// (Fehlerfall `None`). Die Werte stammen aus einem Lauf von Python, nicht aus dem Code hier.
    #[test]
    fn base64_streng_wie_python_validate() {
        let faelle: [(&str, Option<&[u8]>); 45] = [
            // Länge 1 mod 4 trägt nie Daten, auch nicht mit Auffüllung (Mutant H102).
            ("Q=", None),
            ("QUJDR=", None),
            ("QUJDR==", None),
            ("QUJDQUJD=", None),
            ("", Some(&[])),
            ("QUJD", Some(&[65, 66, 67])),
            ("QUI=", Some(&[65, 66])),
            ("QQ==", Some(&[65])),
            ("QR==", Some(&[65])),
            ("QQ=", None),
            ("QQ", None),
            ("Q", None),
            ("QUJDRA==", Some(&[65, 66, 67, 68])),
            ("QUJDRA=", None),
            ("QUJDRA", None),
            ("QUJDREU=", Some(&[65, 66, 67, 68, 69])),
            ("QUJDREVG", Some(&[65, 66, 67, 68, 69, 70])),
            ("QQ==QQ==", None),
            ("=QUJ", None),
            ("QU JD", None),
            ("QUJD\n", None),
            ("QU-D", None),
            ("QU_D", None),
            ("+/+/", Some(&[251, 255, 191])),
            ("////", Some(&[255, 255, 255])),
            ("++++", Some(&[251, 239, 190])),
            ("AAAA", Some(&[0, 0, 0])),
            ("Zm9v", Some(&[102, 111, 111])),
            ("Zm9vYg==", Some(&[102, 111, 111, 98])),
            ("Zm9vYmE=", Some(&[102, 111, 111, 98, 97])),
            ("Zm9vYmFy", Some(&[102, 111, 111, 98, 97, 114])),
            ("Zm9vYg=", None),
            ("Zm9vYg===", None),
            ("Zm9=", Some(&[102, 111])),
            ("Zm==", Some(&[102])),
            ("=", None),
            ("==", None),
            ("A===", None),
            ("AA==", Some(&[0])),
            ("AAA=", Some(&[0, 0])),
            ("AAA", None),
            ("ab/+", Some(&[105, 191, 254])),
            ("ä", None),
            ("QUJDä=", None),
            ("QQ==\n", None),
        ];
        for (text, soll) in faelle {
            assert_eq!(base64_streng(text).as_deref(), soll, "{text:?}");
        }
    }

    /// `json.loads` aus `CPython` 3.14: was Python liest, liest `json_laden` mit denselben Werten
    /// (`NaN`, `Infinity`, Überlauf und große Ganzzahlen als Ersatztext); was Python ablehnt,
    /// lehnt es auch ab.
    #[test]
    fn json_laden_wie_python() {
        let float = || ersatz("float");
        let ganz = || ersatz("int");
        let faelle: Vec<(&str, Option<Value>)> = vec![
            (r#"[1, 2.5, "a"]"#, Some(json!([1, 2.5, "a"]))),
            ("[NaN]", Some(json!([float()]))),
            ("[Infinity, -Infinity]", Some(json!([float(), float()]))),
            ("NaN", Some(float())),
            ("-Infinity", Some(float())),
            (r#"["NaN"]"#, Some(json!(["NaN"]))),
            ("[NaNa]", None),
            ("[Infinityx]", None),
            ("[-Infinityx]", None),
            ("[NaN,1]", Some(json!([float(), 1]))),
            (r#"{"a":NaN}"#, Some(json!({"a": float()}))),
            ("[NaN ]", Some(json!([float()]))),
            ("[NaN\n]", Some(json!([float()]))),
            ("[ NaN , NaN ]", Some(json!([float(), float()]))),
            ("[5 NaN]", None),
            // Auftrag k9-2 (Mutant K04): ein Wort, dem ein Doppelpunkt folgt, ist kein Wort am Ende. Mit dem
            // Ersatztext wäre `{"…float": 1}` ein gültiges Objekt; Python lehnt `{NaN: 1}` ab (Schlüssel ohne
            // Anführungszeichen).
            ("{NaN: 1}", None),
            ("{Infinity: 1}", None),
            ("{-Infinity: 1}", None),
            (r#"{"a": 1, NaN: 2}"#, None),
            ("[12345678901234567890]", Some(json!([ganz()]))),
            (
                "[9223372036854775807]",
                Some(json!([9_223_372_036_854_775_807_i64])),
            ),
            ("[-9223372036854775808]", Some(json!([i64::MIN]))),
            ("[-9223372036854775809]", Some(json!([ganz()]))),
            ("[-12345678901234567890]", Some(json!([ganz()]))),
            ("[1e999]", Some(json!([float()]))),
            ("[-1e999]", Some(json!([float()]))),
            ("[1E999]", Some(json!([float()]))),
            ("[10e400]", Some(json!([float()]))),
            ("[2e308]", Some(json!([float()]))),
            ("[1e+999]", Some(json!([float()]))),
            ("[1.7976931348623159e308]", Some(json!([float()]))),
            (
                "[1.7976931348623157e308]",
                Some(json!([1.797_693_134_862_315_7e308_f64])),
            ),
            ("[1e5]", Some(json!([100_000.0]))),
            ("[1E5]", Some(json!([100_000.0]))),
            ("[1.5e+3]", Some(json!([1500.0]))),
            ("[1e-5]", Some(json!([1e-5]))),
            ("[-]", None),
            ("[--5]", None),
            ("[01]", None),
            ("[-01]", None),
            ("[0123456789012345678901234567890]", None),
            ("[1-2]", None),
            ("[1.5.5]", None),
            ("[1e]", None),
            ("[1e5e5]", None),
            ("", None),
            ("[", None),
            ("[1,]", None),
            (r#"{"a": [1, 2]}"#, Some(json!({"a": [1, 2]}))),
            (
                r#"["NaN 1e999 12345678901234567890"]"#,
                Some(json!(["NaN 1e999 12345678901234567890"])),
            ),
            (r#"{"1e999": 1}"#, Some(json!({"1e999": 1}))),
            (r#"["a\\", NaN]"#, Some(json!(["a\\", float()]))),
            (r#"["\"", NaN]"#, Some(json!(["\"", float()]))),
            (r#"["\" NaN"]"#, Some(json!(["\" NaN"]))),
            (r#"["ä", NaN]"#, Some(json!(["ä", float()]))),
            (r#"["ä", 12345678901234567890]"#, Some(json!(["ä", ganz()]))),
        ];
        for (text, soll) in faelle {
            assert_eq!(laden(text), soll, "{text:?}");
        }
    }

    /// `CPython` liest eine Ganzzahl bis 4300 Ziffern und wirft ab 4301 `ValueError`; eine Kommazahl
    /// mit mehr Ziffern liest es.
    #[test]
    fn json_laden_vierttausenddreihundert_ziffern() {
        let ziffern = |n: usize| format!("1{}", "0".repeat(n - 1));
        for n in [4299, 4300] {
            for vorzeichen in ["", "-"] {
                let text = format!("[{vorzeichen}{}]", ziffern(n));
                assert_eq!(
                    laden(&text),
                    Some(json!([ersatz("int")])),
                    "{vorzeichen}{n} Ziffern"
                );
            }
        }
        for n in [4301, 4302] {
            for vorzeichen in ["", "-"] {
                let text = format!("[{vorzeichen}{}]", ziffern(n));
                assert_eq!(laden(&text), None, "{vorzeichen}{n} Ziffern");
            }
        }
        let lang = format!("[0.{}]", "1".repeat(5000));
        assert!(laden(&lang).is_some(), "Kommazahl mit 5000 Ziffern");
    }

    /// Die Kennzeichnung kommt im Auszug nicht roh vor: steht sie schon darin, hängt `marke_fuer`
    /// Unterstriche an, bis sie neu ist; der Auszug behält seinen Text.
    #[test]
    fn json_laden_waehlt_eine_marke_die_im_auszug_nicht_vorkommt() {
        assert_eq!(marke_fuer("[]"), MARKE);
        let (wert, marke) = json_laden(r#"["§nicht_tragbar:float", NaN]"#).unwrap();
        assert_eq!(marke, "§nicht_tragbar:_");
        assert_eq!(
            wert,
            json!(["§nicht_tragbar:float", "§nicht_tragbar:_float"])
        );
        let (wert, marke) = json_laden(r#"["§nicht_tragbar:_", "§nicht_tragbar:", NaN]"#).unwrap();
        assert_eq!(marke, "§nicht_tragbar:__");
        assert_eq!(wert[2], json!("§nicht_tragbar:__float"));
        assert_eq!(json_laden("[]").unwrap().1, MARKE);
    }

    /// Eine Ganzzahl in der Schreibweise von JSON: `-?(0|[1-9][0-9]*)`.
    #[test]
    fn json_ganzzahl_ist_die_json_schreibweise() {
        for ja in ["0", "-0", "7", "-7", "10", "-10", "9223372036854775808"] {
            assert!(json_ganzzahl(ja), "{ja}");
        }
        for nein in [
            "", "-", "--5", "007", "-07", "00", "1e5", "1.5", "+5", "1-2", "a", "5a",
        ] {
            assert!(!json_ganzzahl(nein), "{nein}");
        }
    }

    /// Ein Wort endet vor Leerraum, `,`, `]`, `}` oder am Ende des Texts.
    #[test]
    fn wort_ende_kennt_die_trenner() {
        for ja in [
            "NaN", "NaN ", "NaN\t", "NaN\n", "NaN,", "NaN]", "NaN}", "NaN ,",
        ] {
            assert!(wort_ende(ja, 3), "{ja:?}");
        }
        for nein in ["NaNa", "NaN1", "NaN:", "NaN\"", "NaN)", "NaN_"] {
            assert!(!wort_ende(nein, 3), "{nein:?}");
        }
        assert!(wort_ende("-Infinity", 9));
        assert!(!wort_ende("-Infinityx", 9));
    }

    fn status_und_text(f: ApiFehler) -> (u16, String) {
        match f {
            ApiFehler::Status(s, m) => (s, m),
            ApiFehler::Unerwartet { typ, meldung } => (500, format!("{typ}: {meldung}")),
        }
    }

    /// `(body.get("format") or "").strip().lower()`: ein falscher Wert ist leer, ein wahrer
    /// Nicht-Text ist ein `AttributeError` (500).
    #[test]
    fn format_lesen_wie_python() {
        let lies = |b: Value| -> Result<String, (u16, String)> {
            let Value::Object(m) = b else { unreachable!() };
            format_lesen(&m).map_err(status_und_text)
        };
        assert_eq!(lies(json!({"format": " CSV "})), Ok("csv".into()));
        assert_eq!(lies(json!({"format": "Json"})), Ok("json".into()));
        assert_eq!(lies(json!({"format": "PDF"})), Ok("pdf".into()));
        assert_eq!(lies(json!({"format": "   "})), Ok(String::new()));
        for leer in [
            json!({}),
            json!({"format": null}),
            json!({"format": false}),
            json!({"format": 0}),
            json!({"format": ""}),
            json!({"format": []}),
            json!({"format": {}}),
        ] {
            assert_eq!(lies(leer.clone()), Ok(String::new()), "{leer}");
        }
        for (wahr, typ) in [
            (json!({"format": 5}), "int"),
            (json!({"format": true}), "bool"),
            (json!({"format": [1]}), "list"),
            (json!({"format": {"a": 1}}), "dict"),
            (json!({"format": 1.5}), "float"),
        ] {
            assert_eq!(
                lies(wahr.clone()),
                Err((
                    500,
                    format!("AttributeError: '{typ}' object has no attribute 'strip'")
                )),
                "{wahr}"
            );
        }
    }

    /// Der JSON-Zweig: eine Liste im Rumpf gilt (ohne Kennzeichnung), Text wird gelesen (mit), ein
    /// falscher Wert ist die leere Liste; alles andere ist 400.
    #[test]
    fn json_liste_wie_python() {
        let lies = |v: Value| json_liste(&v).map_err(status_und_text);
        assert_eq!(
            lies(json!([1, "a"])),
            Ok((vec![json!(1), json!("a")], None))
        );
        for leer in [
            json!(null),
            json!(false),
            json!(0),
            json!(""),
            json!({}),
            json!(0.0),
        ] {
            assert_eq!(lies(leer.clone()), Ok((vec![], None)), "{leer}");
        }
        assert_eq!(
            lies(json!("[1, NaN]")),
            Ok((
                vec![json!(1), json!(format!("{MARKE}float"))],
                Some(MARKE.to_owned())
            ))
        );
        assert_eq!(lies(json!("[]")), Ok((vec![], Some(MARKE.to_owned()))));
        let nicht_lesbar = Err((400, "json-Inhalt nicht parsebar".to_owned()));
        for kaputt in [
            json!("nope"),
            json!("["),
            json!(5),
            json!(true),
            json!({"a": 1}),
            json!(1.5),
        ] {
            assert_eq!(lies(kaputt.clone()), nicht_lesbar, "{kaputt}");
        }
        for keine_liste in [
            json!("5"),
            json!("{}"),
            json!(r#""x""#),
            json!("null"),
            json!("NaN"),
        ] {
            assert_eq!(
                lies(keine_liste.clone()),
                Err((
                    400,
                    "json muss eine Liste von Transaktionen sein".to_owned()
                )),
                "{keine_liste}"
            );
        }
    }

    /// Der PDF-Zweig: Text mit Inhalt, base64 mit `validate=True`.
    #[test]
    fn pdf_bytes_wie_python() {
        let lies = |v: Value| pdf_bytes(&v).map_err(status_und_text);
        assert_eq!(lies(json!("QUJD")), Ok(vec![65, 66, 67]));
        let fehlt = Err((
            400,
            "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)".to_owned(),
        ));
        for leer in [
            json!(null),
            json!(""),
            json!("   \n"),
            json!(5),
            json!(["QUJD"]),
            json!({}),
            json!(false),
        ] {
            assert_eq!(lies(leer.clone()), fehlt, "{leer}");
        }
        let ungueltig = Err((400, "pdf-Inhalt nicht gültig base64-kodiert".to_owned()));
        for kaputt in [
            json!("QQ="),
            json!("QU JD"),
            json!(" QUJD"),
            json!("QUJD\n"),
            json!("ä"),
        ] {
            assert_eq!(lies(kaputt.clone()), ungueltig, "{kaputt}");
        }
    }

    /// Die Schwelle, mit der `pdf_lesen` liest, ist die aus Python (`parse_pdf_zeilen(..., schwelle=0.6)`,
    /// `kontoauszug_writer.py:289`): eine Zeile mit Konfidenz 0,55 fällt heraus, eine mit 0,6 bleibt
    /// (Auftrag 8, Mutant H112). `pdf_lesen` selbst braucht Tesseract; die Schwelle prüft der Test an
    /// der Funktion, der sie übergeben wird.
    #[test]
    fn pdf_schwelle_ist_die_aus_python() {
        let zeile = "01.03.2025 Maler Huber -480,00 EUR";
        let lies = |konfidenz: f64| {
            let conf = std::collections::BTreeMap::from([(0_usize, konfidenz)]);
            let (tx, verworfen) = parse_pdf_zeilen(zeile, &conf, PDF_SCHWELLE);
            (tx.len(), verworfen)
        };
        assert_eq!(lies(0.55), (0, 1));
        assert_eq!(lies(0.59), (0, 1));
        assert_eq!(lies(0.6), (1, 0));
        assert_eq!(lies(0.95), (1, 0));
    }

    /// Ein Inhalt, der kein Text ist, liest das CSV wie der leere Text: nichts, kein Fehler. Der Ersatz
    /// `""` ist von jedem anderen Text ohne Datenzeile nicht zu unterscheiden (Mutant H129, gleichwertig).
    #[test]
    fn csv_ohne_text_ist_wie_csv_ohne_zeilen() {
        let ergebnis = |t: &str| {
            parse_csv(t)
                .map(|(tx, n)| (tx.len(), n))
                .map_err(|e| e.to_string())
        };
        assert_eq!(ergebnis(""), Ok((0, 0)));
        assert_eq!(ergebnis("x"), ergebnis(""));
        assert_eq!(ergebnis("datum;betrag;verwendungszweck\n"), Ok((0, 0)));
    }
}
