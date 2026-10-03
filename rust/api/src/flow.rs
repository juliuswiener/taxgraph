//! `produkt/haut/flow.py` — der Fluss-Mitschnitt: was gefragt wurde, was geantwortet, was die KI
//! dazwischen tat. Eine JSON-Zeile je Ereignis in `<audit_dir>/flow.jsonl`, nur mit
//! `TAXGRAPH_FLOW=1` (oder dem aelteren `TAXGRAPH_KI_DEBUG=1`), Datei mit 0600.
//!
//! Der Schalter wird bei JEDEM Aufruf gelesen, nicht beim Start: [`an`] ist [`crate::konfig::flow_an`].
//!
//! Zeilen und `inhalt` sind [`PyWert`], nie `serde_json::Value`: ohne `preserve_order` sortierte
//! `Value` die Schluessel, Python behaelt die Einfuegereihenfolge. [`dumps`] ist
//! `json.dumps(x, ensure_ascii=False)` ueber `PyWert`, Byte fuer Byte.
//!
//! PARITÄT (festgehalten, nicht angeglichen):
//! - `AKTUELLER_FALL` gibt es nicht (F1): Python setzt ihn pro Anfrage in einer Modulvariable, weil
//!   der Server einfaedig ist. Der Rust-Server ist mehrfaedig; der Aufrufer reicht den Fall durch.
//! - `ablage` ist der beim Start aufgeloeste `audit_dir` ([`crate::konfig::Konfig`]). Python liest
//!   `audit._ablage()` bei jedem Aufruf; eine Umlenkung nach dem Start wirkt hier nicht.
//! - Eine Ganzzahl ausserhalb von `i64::MIN..=u64::MAX` kommt als `float` an ([`PyWert`]-Grenze,
//!   D1) und steht dann als `1e+30` in der Zeile, Python schreibt die Ziffern.
use std::fs::OpenOptions;
use std::io::Write as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::Path;

use domain::PyWert;
use serde_json::json;

pub use crate::konfig::flow_an as an;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;

/// Dateiname im Ablageverzeichnis (`flow.py:34`).
pub const DATEI: &str = "flow.jsonl";

/// Was die OBERFLAECHE melden darf, sortiert wie `sorted(UI_ARTEN)` (`flow.py:113`). Eine feste
/// Liste, kein freies Feld: ein Client, der sich die Sorte ausdenkt, fuellte sonst die Datei.
pub const UI_ARTEN: [&str; 5] = [
    "nachfrage_spaeter",
    "nachfragen_gestartet",
    "pruefliste_aendern",
    "pruefliste_weiter",
    "weg_gewaehlt",
];

/// Ein Client-Beitrag wird gekappt, nicht geprueft (`flow.py:125`), in Zeichen (Codepunkten).
pub const MAX_ZEICHEN: usize = 4000;

/// `json.dumps(x, ensure_ascii=False)`: Trenner `", "` und `": "`, Text mit Pythons Escapes,
/// `NaN`/`Infinity` fuer die Gleitkomma-Sonderwerte (kein gueltiges JSON, aber Pythons Vorgabe).
///
/// ```
/// use api::flow::dumps;
/// use domain::PyWert;
/// let w: PyWert = serde_json::from_str(r#"{"b": [1, 2.5, null], "a": "ä\n"}"#).unwrap();
/// assert_eq!(dumps(&w), "{\"b\": [1, 2.5, null], \"a\": \"ä\\n\"}");
/// ```
#[must_use]
pub fn dumps(wert: &PyWert) -> String {
    match wert {
        PyWert::Null => "null".to_owned(),
        PyWert::Bool(b) => b.to_string(),
        PyWert::Ganz(n) => n.to_string(),
        PyWert::GrossGanz(u) => u.to_string(),
        PyWert::Gleit(f) if f.is_nan() => "NaN".to_owned(),
        PyWert::Gleit(f) if f.is_infinite() => {
            if *f > 0.0 { "Infinity" } else { "-Infinity" }.to_owned()
        }
        PyWert::Gleit(_) => wert.repr(),
        PyWert::Text(s) => text(s),
        PyWert::Liste(l) => {
            let teile: Vec<String> = l.iter().map(dumps).collect();
            format!("[{}]", teile.join(", "))
        }
        PyWert::Objekt(o) => {
            let teile: Vec<String> = o
                .iter()
                .map(|(k, w)| format!("{}: {}", text(k), dumps(w)))
                .collect();
            format!("{{{}}}", teile.join(", "))
        }
    }
}

/// Ein JSON-Text-Literal. `serde_json` escaped wie Pythons `ensure_ascii=False`: `"`, `\`, die
/// Steuerzeichen unter U+0020 (`\b \f \n \r \t`, sonst `\u00xx`), alles andere roh.
fn text(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| String::from("\"\""))
}

/// `gekappt(inhalt, grenze)`: bis `grenze` Zeichen der JSON-Fassung bleibt `inhalt`, sonst steht
/// da, WIE VIEL gekappt wurde — stillschweigend gekuerzt sah wie „mehr war da nicht" aus.
///
/// ```
/// use api::flow::gekappt;
/// use domain::PyWert;
/// let kurz = PyWert::Text("abc".into());
/// assert_eq!(gekappt(&kurz, 5), kurz); // "\"abc\"" hat 5 Zeichen
/// let lang = gekappt(&kurz, 4);
/// assert_eq!(lang.repr(), "{'gekappt_bei': 4, 'urspruengliche_zeichen': 5, 'anfang': '\"abc'}");
/// ```
#[must_use]
pub fn gekappt(inhalt: &PyWert, grenze: usize) -> PyWert {
    let roh = dumps(inhalt);
    let laenge = roh.chars().count();
    if laenge <= grenze {
        return inhalt.clone();
    }
    let zahl = |n: usize| PyWert::Ganz(i64::try_from(n).unwrap_or(i64::MAX));
    PyWert::Objekt(vec![
        ("gekappt_bei".into(), zahl(grenze)),
        ("urspruengliche_zeichen".into(), zahl(laenge)),
        (
            "anfang".into(),
            PyWert::Text(roh.chars().take(grenze).collect()),
        ),
    ])
}

/// `datetime.now(timezone.utc).isoformat()`: Mikrosekunden nur, wenn nicht 0.
fn iso_jetzt() -> String {
    let t = chrono::Utc::now();
    if t.timestamp_subsec_micros() == 0 {
        t.format("%Y-%m-%dT%H:%M:%S+00:00").to_string()
    } else {
        t.format("%Y-%m-%dT%H:%M:%S%.6f+00:00").to_string()
    }
}

/// Die Zeile samt Zeilenende (`flow.py:86-94`): `ts`, `fall`, `art`, `inhalt` in dieser Reihenfolge.
/// `fall`: `fall_id or AKTUELLER_FALL` — ein leerer Text zaehlt wie ein fehlender und wird `null`.
///
/// ```
/// use api::flow::zeile;
/// use domain::PyWert;
/// let z = zeile("2026-01-01T00:00:00+00:00", Some("f1"), "antwort", &PyWert::Ganz(7));
/// assert_eq!(z, "{\"ts\": \"2026-01-01T00:00:00+00:00\", \"fall\": \"f1\", \"art\": \"antwort\", \"inhalt\": 7}\n");
/// assert!(zeile("t", Some(""), "a", &PyWert::Null).contains("\"fall\": null"));
/// ```
#[must_use]
pub fn zeile(ts: &str, fall: Option<&str>, art: &str, inhalt: &PyWert) -> String {
    let fall = fall
        .filter(|f| !f.is_empty())
        .map_or_else(|| "null".to_owned(), text);
    format!(
        "{{\"ts\": {}, \"fall\": {fall}, \"art\": {}, \"inhalt\": {}}}\n",
        text(ts),
        text(art),
        dumps(inhalt)
    )
}

/// `makedirs`, dann 0600 beim Anlegen anhaengen; eine bestehende Datei behaelt ihre Rechte
/// (`flow.py:89-94`). Kein `fsync`, wie in Python.
fn haenge_an(ablage: &Path, zeile: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(ablage)?;
    let mut f = OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .open(ablage.join(DATEI))?;
    f.write_all(zeile.as_bytes())
}

/// Eine Zeile. Wirft nie — ein Protokoll darf den Vorgang nicht mitreissen, den es beschreibt
/// (`except Exception: pass`, `flow.py:95`). Ohne Schalter ([`an`]) geschieht nichts.
///
/// `fall`: der Fall dieser Anfrage; der Aufrufer reicht ihn durch (F1, s. Moduldoku).
pub fn schreibe(ablage: &Path, fall: Option<&str>, art: &str, inhalt: &PyWert) {
    if !an() {
        return;
    }
    // ponytail: ein Fehler beim Schreiben bleibt unsichtbar (wie in Python); upgrade waere ein
    // Zaehler fuer verlorene Zeilen, sobald jemand den Mitschnitt fuer mehr als Diagnose nutzt.
    let _ = haenge_an(ablage, &zeile(&iso_jetzt(), fall, art, inhalt));
}

/// `obj.get(schluessel)` eines `dict`; alles andere (und ein fehlender Schluessel) ist `None`.
pub(crate) fn hole<'a>(obj: &'a PyWert, schluessel: &str) -> Option<&'a PyWert> {
    match obj {
        PyWert::Objekt(paare) => paare.iter().find(|(k, _)| k == schluessel).map(|(_, v)| v),
        _ => None,
    }
}

/// Der `inhalt` von `ergebnis_notiert` (`flow.py:141-143`): `grund`, `zahl_cent`, wie viele
/// offene Felder benannt wurden, und die ersten 12.
///
/// ponytail (A2): ein `offen`, das keine Liste ist, zaehlt wie leer. Python wirft dort `TypeError`
/// oder schneidet einen Text; der einzige Aufrufer (`api.ergebnis`, `api.py:568`) liefert immer
/// eine Liste.
#[must_use]
pub fn ergebnis_inhalt(obj: &PyWert) -> PyWert {
    let offen: &[PyWert] = match hole(obj, "offen") {
        Some(PyWert::Liste(l)) => l,
        _ => &[],
    };
    let zahl = i64::try_from(offen.len()).unwrap_or(i64::MAX);
    PyWert::Objekt(vec![
        (
            "grund".into(),
            hole(obj, "grund").cloned().unwrap_or(PyWert::Null),
        ),
        (
            "zahl_cent".into(),
            hole(obj, "zahl_cent").cloned().unwrap_or(PyWert::Null),
        ),
        ("offen_anzahl".into(), PyWert::Ganz(zahl)),
        (
            "offen".into(),
            PyWert::Liste(offen.iter().take(12).cloned().collect()),
        ),
    ])
}

/// Der Ausgang von `/ergebnis`. `offen_anzahl` ist die eigentliche Aussage: ein Grund OHNE ein
/// einziges benanntes Feld ist der Zustand, in dem der Nutzer „noch offen" liest und nicht
/// erfaehrt, woran es liegt.
pub fn ergebnis_notiert(ablage: &Path, fall_id: &str, obj: &PyWert) {
    schreibe(ablage, Some(fall_id), "ergebnis", &ergebnis_inhalt(obj));
}

/// `POST /fall/<id>/flow` (`flow.melde_ui`): was die OBERFLAECHE gezeigt hat. Ohne Schalter
/// `200 {"mitgeschrieben": false}` und keine Pruefung; sonst ein Rumpf ohne Objekt oder eine `art`
/// ohne Text oder ausserhalb [`UI_ARTEN`] ist 400 (Pythons `ValueError`, die Huelle macht daraus
/// 400), alles andere wird gekappt mitgeschrieben.
///
/// # Errors
/// 400 mit Pythons Wortlaut.
pub fn melde_ui(ablage: &Path, fall_id: &str, body: &PyWert) -> Result<Antwort, ApiFehler> {
    if !an() {
        return Ok(Antwort::neu(200, json!({"mitgeschrieben": false})));
    }
    if !matches!(body, PyWert::Objekt(_)) {
        return Err(ApiFehler::status(400, "Rumpf muss ein Objekt sein"));
    }
    let art = match hole(body, "art") {
        Some(PyWert::Text(a)) if UI_ARTEN.contains(&a.as_str()) => a,
        _ => {
            let liste = UI_ARTEN.map(|a| format!("'{a}'")).join(", ");
            return Err(ApiFehler::status(
                400,
                format!("art muss eines von [{liste}] sein"),
            ));
        }
    };
    let inhalt = hole(body, "inhalt").unwrap_or(&PyWert::Null);
    schreibe(ablage, Some(fall_id), art, &gekappt(inhalt, MAX_ZEICHEN));
    Ok(Antwort::neu(200, json!({"mitgeschrieben": true})))
}

/// `kopf_der_queue(fragen, wie_viele)`: was der Nutzer als Naechstes SIEHT, nicht die ganze Queue —
/// je Frage `feld_id`, die ersten 90 Zeichen des Fragetexts und die Instanzzahl.
///
/// ponytail (A2): ein `fragetext_laie`, das kein Text ist, zaehlt wie leer; eine Frage, die kein
/// `dict` ist, liefert drei `null`. Python wirft bzw. schneidet dort; `api.fragen` (`api.py:358`)
/// reicht nur Metadaten-`dict`s mit Text oder `None`.
#[must_use]
pub fn kopf_der_queue(fragen: &[PyWert], wie_viele: usize) -> PyWert {
    let kopf = |q: &PyWert| {
        let frage: String = match hole(q, "fragetext_laie") {
            Some(PyWert::Text(t)) => t.chars().take(90).collect(),
            _ => String::new(),
        };
        PyWert::Objekt(vec![
            (
                "feld_id".into(),
                hole(q, "feld_id").cloned().unwrap_or(PyWert::Null),
            ),
            ("frage".into(), PyWert::Text(frage)),
            (
                "instanzen".into(),
                hole(q, "instanz_anzahl").cloned().unwrap_or(PyWert::Null),
            ),
        ])
    };
    PyWert::Liste(fragen.iter().take(wie_viele).map(kopf).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pw(text: &str) -> PyWert {
        serde_json::from_str(text).unwrap()
    }

    /// Einfuegereihenfolge, Trenner, Escapes und Sonderwerte: die Zeichen, an denen `serde_json`
    /// und Pythons `json.dumps` auseinanderlaufen koennten.
    #[test]
    fn dumps_wie_python() {
        assert_eq!(
            dumps(&pw(r#"{"z": 1, "a": [true, false, null]}"#)),
            r#"{"z": 1, "a": [true, false, null]}"#
        );
        assert_eq!(
            dumps(&PyWert::Text("a\"b\\c\n\t\u{1}\u{7f}ä€\u{2028}".into())),
            "\"a\\\"b\\\\c\\n\\t\\u0001\u{7f}ä€\u{2028}\""
        );
        assert_eq!(dumps(&PyWert::Gleit(1e16)), "1e+16");
        assert_eq!(dumps(&PyWert::Gleit(-0.0)), "-0.0");
        assert_eq!(dumps(&PyWert::Gleit(f64::NAN)), "NaN");
        assert_eq!(dumps(&PyWert::Gleit(f64::INFINITY)), "Infinity");
        assert_eq!(dumps(&PyWert::Gleit(f64::NEG_INFINITY)), "-Infinity");
        assert_eq!(dumps(&pw("18446744073709551615")), "18446744073709551615");
        assert_eq!(dumps(&pw("[]")), "[]");
        assert_eq!(dumps(&pw("{}")), "{}");
    }

    /// Die Grenze zaehlt Zeichen, nicht Bytes: 4000 Umlaute sind 8000 Bytes.
    #[test]
    fn kappung_zaehlt_zeichen() {
        let genau = PyWert::Text("ä".repeat(MAX_ZEICHEN - 2)); // + zwei Anfuehrungszeichen
        assert_eq!(gekappt(&genau, MAX_ZEICHEN), genau);
        let zu_lang = PyWert::Text("ä".repeat(MAX_ZEICHEN - 1));
        let PyWert::Objekt(o) = gekappt(&zu_lang, MAX_ZEICHEN) else {
            panic!("nicht gekappt")
        };
        assert_eq!(o[0], ("gekappt_bei".into(), PyWert::Ganz(4000)));
        assert_eq!(o[1], ("urspruengliche_zeichen".into(), PyWert::Ganz(4001)));
        let PyWert::Text(anfang) = &o[2].1 else {
            panic!("anfang")
        };
        assert_eq!(anfang.chars().count(), MAX_ZEICHEN);
    }

    #[test]
    fn ergebnis_inhalt_kappt_offen_auf_zwoelf() {
        let offen: Vec<String> = (0..15).map(|i| format!("f{i}")).collect();
        let obj = pw(
            &serde_json::json!({"grund": "offen", "zahl_cent": null, "offen": offen}).to_string(),
        );
        let PyWert::Objekt(o) = ergebnis_inhalt(&obj) else {
            panic!("kein Objekt")
        };
        let namen: Vec<&str> = o.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(namen, ["grund", "zahl_cent", "offen_anzahl", "offen"]);
        assert_eq!(o[2].1, PyWert::Ganz(15));
        let PyWert::Liste(l) = &o[3].1 else {
            panic!("offen")
        };
        assert_eq!(l.len(), 12);
        // A2: kein dict, kein offen -> null, null, 0, [].
        assert_eq!(
            ergebnis_inhalt(&PyWert::Null).repr(),
            "{'grund': None, 'zahl_cent': None, 'offen_anzahl': 0, 'offen': []}"
        );
    }

    #[test]
    fn kopf_nimmt_sechs_und_kappt_den_text_bei_neunzig_zeichen() {
        let fragen: Vec<PyWert> = (0..8)
            .map(|i| pw(&serde_json::json!({"feld_id": format!("f{i}"), "fragetext_laie": "ä".repeat(100), "instanz_anzahl": 1}).to_string()))
            .collect();
        let PyWert::Liste(k) = kopf_der_queue(&fragen, 6) else {
            panic!("keine Liste")
        };
        assert_eq!(k.len(), 6);
        let PyWert::Objekt(erste) = &k[0] else {
            panic!("kein Objekt")
        };
        assert_eq!(erste[1].1, PyWert::Text("ä".repeat(90)));
        assert_eq!(
            kopf_der_queue(&[pw(r#"{"feld_id": "x", "fragetext_laie": null}"#)], 6).repr(),
            "[{'feld_id': 'x', 'frage': '', 'instanzen': None}]"
        );
    }
}
