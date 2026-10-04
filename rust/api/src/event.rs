//! `api.event` (`api.py:495`): DER Schreib-Endpunkt. Eine dünne Hülle über den Store: sie prüft die
//! Form der Anfrage, legt sie roh ab ([`store::Store::append_roh`]) und schreibt die Akte. Die
//! Garantien (`llm` bleibt vorläufig, `bestaetigt` braucht `signal_2`, ein aktives Event je Feld)
//! erzwingt der Store, nicht diese Schicht.
use domain::{Achsenwert, Herkunft, HerkunftAlt, HerkunftVektor, PruefTiefe, PyWert, Zustand};
use serde_json::{json, Map, Value};
use store::{AbweisungRoh, BindungNachschlag, NeuesEventRoh, Signal};

use crate::antwort::Antwort;
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::flow;
use crate::python::{repr, text, typname, unhashbar};
use crate::zustand::Zustand as Dienst;

/// Die Stufen der Prüftiefe in der Reihenfolge von `_PRUEF_ORD` (`store.py:47`).
const PRUEF_STUFEN: [&str; 4] = [
    "ungeprueft",
    "plausibilisiert",
    "orakel_bestaetigt",
    "amtlich",
];

/// Vorschlags-Schreiber, für die der Katalog gilt (`api.py:523`).
const VORSCHLAG_PRAEFIXE: [&str; 4] = ["llm:", "berechnet:", "import:beleg", "import:kontoauszug"];

/// `body.get(schluessel)`: ein fehlender Schlüssel ist `None`.
fn hole<'a>(body: &'a Map<String, Value>, schluessel: &str) -> &'a Value {
    static NICHTS: Value = Value::Null;
    body.get(schluessel).unwrap_or(&NICHTS)
}

/// Ein Fehler der Form (`_pruefe_begleitfelder`, `store.py:325`) oder des Stores: `ValueError`
/// in Python, bei `/event` ein 422.
struct Abgewiesen(String);

/// Was Python an dieser Stelle als etwas anderes als `ValueError` wirft (ein 500).
enum Abbruch {
    Abgewiesen(Abgewiesen),
    Unerwartet(ApiFehler),
}

impl From<Abgewiesen> for Abbruch {
    fn from(a: Abgewiesen) -> Self {
        Self::Abgewiesen(a)
    }
}

fn form(text: impl Into<String>) -> Abgewiesen {
    Abgewiesen(format!("fail-closed (Form): {}", text.into()))
}

/// `_pruefe_begleitfelder(ts, herkunft, signal)` (`store.py:325-362`): `ts`, `herkunft` und
/// `signal` tragen genau die Form, die der Rust-Leser der Fallakte kennt. Gemeinsame Tabelle der
/// Formen: `rust/fixtures/begleitfelder_formen.json`. `herkunft` ist hier schon ein Objekt
/// (`api.event` prüft das vorher).
fn pruefe_begleitfelder(
    ts: &Value,
    herkunft: &Map<String, Value>,
    signal: &Value,
) -> Result<(), Abbruch> {
    if !matches!(ts, Value::Null | Value::String(_)) {
        return Err(form(format!(
            "ts muss Text sein oder fehlen, nicht {}.",
            typname(ts)
        ))
        .into());
    }
    let schluessel: Vec<&str> = herkunft.keys().map(String::as_str).collect();
    let voll = schluessel.len() == 3
        && ["herkunft", "pruef_tiefe", "haftung"]
            .iter()
            .all(|k| herkunft.contains_key(*k));
    if !voll && schluessel != ["herkunft"] {
        return Err(form(
            "herkunft muss genau die Schlüssel herkunft, pruef_tiefe und haftung tragen (oder nur \
             herkunft) — jeder andere Schlüsselsatz macht die Akte für die Rust-Fassung unlesbar.",
        )
        .into());
    }
    for achse in ["herkunft", "haftung"] {
        if let Some(v) = herkunft.get(achse) {
            if v.as_str().is_none_or(str::is_empty) {
                return Err(
                    form(format!("herkunft.{achse} muss ein nicht leerer Text sein.")).into(),
                );
            }
        }
    }
    if let Some(stufe) = herkunft.get("pruef_tiefe") {
        // Python: `stufe not in _PRUEF_ORD` auf einem `dict`; eine Liste oder ein Objekt ist dort
        // nicht hashbar, ein `TypeError` und damit ein 500, kein 422.
        if matches!(stufe, Value::Array(_) | Value::Object(_)) {
            return Err(Abbruch::Unerwartet(ApiFehler::unerwartet(
                "TypeError",
                unhashbar(stufe, "dict key"),
            )));
        }
        if !stufe.as_str().is_some_and(|s| PRUEF_STUFEN.contains(&s)) {
            return Err(form(format!(
                "herkunft.pruef_tiefe muss eine der Stufen {} sein.",
                PRUEF_STUFEN.join(", ")
            ))
            .into());
        }
    }
    match signal {
        Value::Null => {}
        Value::Object(o) => {
            if o.keys().any(|k| k != "signal_1" && k != "signal_2") {
                return Err(
                    form("signal darf nur die Schlüssel signal_1 und signal_2 tragen.").into(),
                );
            }
        }
        andere => {
            return Err(form(format!(
                "signal muss ein Objekt sein oder fehlen, nicht {}.",
                typname(andere)
            ))
            .into());
        }
    }
    Ok(())
}

fn prueftiefe(s: &str) -> Option<PruefTiefe> {
    Some(match s {
        "ungeprueft" => PruefTiefe::Ungeprueft,
        "plausibilisiert" => PruefTiefe::Plausibilisiert,
        "orakel_bestaetigt" => PruefTiefe::OrakelBestaetigt,
        "amtlich" => PruefTiefe::Amtlich,
        _ => return None,
    })
}

/// Die Herkunft, nachdem [`pruefe_begleitfelder`] sie freigegeben hat: voll oder nur `herkunft`.
fn herkunft_vektor(herkunft: &Map<String, Value>) -> Option<HerkunftVektor> {
    let achse = |k: &str| Achsenwert::new(herkunft.get(k)?.as_str()?).ok();
    let kopf = achse("herkunft")?;
    if herkunft.len() == 1 {
        return Some(HerkunftVektor::Alt(HerkunftAlt { herkunft: kopf }));
    }
    Some(HerkunftVektor::Voll(Herkunft {
        herkunft: kopf,
        pruef_tiefe: prueftiefe(herkunft.get("pruef_tiefe")?.as_str()?)?,
        haftung: achse("haftung")?,
    }))
}

/// `signal or {"signal_1": None, "signal_2": None}`, so abgelegt wie geschickt: ein Schlüssel, der
/// fehlt, fehlt auch in der Akte. Zweiter Wert: der Typname eines `signal_2`, das kein Text ist.
fn signal_ablage(signal: &Value) -> (Signal, Option<String>) {
    let standard = || {
        (
            Signal {
                signal_1: Some(None),
                signal_2: None,
                signal_2_fehlt: false,
            },
            None,
        )
    };
    let Value::Object(o) = signal else {
        return standard();
    };
    if o.is_empty() {
        return standard();
    }
    let signal_1 = o.get("signal_1").map(|w| match w {
        Value::Null => None,
        w => Some(PyWert::from(w.clone())),
    });
    let (signal_2, fremd) = match o.get("signal_2") {
        None | Some(Value::Null) => (None, None),
        Some(Value::String(s)) => (Some(s.clone()), None),
        Some(w) => (None, Some(typname(w).to_owned())),
    };
    let signal_2_fehlt = !o.contains_key("signal_2");
    (
        Signal {
            signal_1,
            signal_2,
            signal_2_fehlt,
        },
        fremd,
    )
}

/// Die ersten 200 Zeichen (`str(e)[:200]`).
fn kopf200(s: &str) -> String {
    s.chars().take(200).collect()
}

/// Die Felder des Rumpfs, nachdem `api.event` sie an der Tür geprüft hat (`api.py:501-518`).
struct Tuer<'a> {
    rumpf: &'a Map<String, Value>,
    feld_id: String,
    zustand: Zustand,
    herkunft: &'a Map<String, Value>,
    schreiber: &'a str,
}

/// `api.event` bis zum Aufruf des Stores (`api.py:501-518`), in Pythons Reihenfolge: Rumpf,
/// `feld_id`, `zustand`, `herkunft`, `schreiber`.
fn tuer<'a>(
    sb: &crate::zustand::ScheibenBindung<'_>,
    body: &'a Value,
) -> Result<Tuer<'a>, ApiFehler> {
    let Value::Object(b) = body else {
        return Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", typname(body)),
        ));
    };
    let fid = hole(b, "feld_id");
    // Pythons `fid not in bindung` ist ein Wörterbuch-Zugriff: Liste und Objekt sind nicht hashbar.
    if matches!(fid, Value::Array(_) | Value::Object(_)) {
        return Err(ApiFehler::unerwartet(
            "TypeError",
            unhashbar(fid, "dict key"),
        ));
    }
    let feld_id = fid.as_str().unwrap_or_default().to_owned();
    let in_scheibe = fid.as_str().is_some() && sb.index.contains_key(&feld_id);
    if !in_scheibe {
        // Wiederholte Instanz (`base__n`): die Basis muss eine `instanz_gruppe` tragen.
        let basis = fid
            .as_str()
            .and_then(elster::parse_instanz)
            .map(|(basis, _)| basis);
        let instanzfaehig = basis.is_some_and(|b| {
            sb.index
                .get(b)
                .is_some_and(|x| x.instanz_gruppe.as_deref().is_some_and(|g| !g.is_empty()))
        });
        if !instanzfaehig {
            return Err(ApiFehler::status(
                400,
                format!("feld_id {} nicht in dieser Scheibe", repr(fid)),
            ));
        }
    }
    let zustand = hole(b, "zustand");
    let zustand_enum = match zustand.as_str() {
        Some("vorlaeufig") => Some(Zustand::Vorlaeufig),
        Some("bestaetigt") => Some(Zustand::Bestaetigt),
        _ => None,
    };
    let Some(zustand_enum) = zustand_enum else {
        // Pythons `zustand not in {…}`: ein Set; Liste und Objekt sind nicht hashbar.
        if matches!(zustand, Value::Array(_) | Value::Object(_)) {
            return Err(ApiFehler::unerwartet(
                "TypeError",
                unhashbar(zustand, "set element"),
            ));
        }
        // ponytail: Python druckt das Set in der Reihenfolge seines Hashs, die mit
        // `PYTHONHASHSEED` wechselt; hier steht die häufigste.
        return Err(ApiFehler::status(
            400,
            "zustand muss {'bestaetigt', 'vorlaeufig'} sein",
        ));
    };
    let Some(herkunft) = hole(b, "herkunft")
        .as_object()
        .filter(|h| h.contains_key("herkunft"))
    else {
        return Err(ApiFehler::status(
            400,
            "herkunft-Objekt (mit Schlüssel 'herkunft') ist Pflicht",
        ));
    };
    let Some(schreiber) = hole(b, "schreiber").as_str().filter(|s| !s.is_empty()) else {
        return Err(ApiFehler::status(400, "schreiber ist Pflicht"));
    };
    Ok(Tuer {
        rumpf: b,
        feld_id,
        zustand: zustand_enum,
        herkunft,
        schreiber,
    })
}

/// `api.event(fall_id, body)` nach dem Owner-Check.
///
/// `roh` ist der Rumpf als Bytes: der Mitschnitt (`flow.jsonl`) hält die Schlüssel in der
/// Reihenfolge des Clients, ein `serde_json::Value` sortiert sie.
///
/// # Errors
/// 400 für Feld, Zustand, Herkunft und Schreiber; 422 für jede Abweisung des Stores; 500 mit der
/// Python-Klasse, wo Python eine andere Ausnahme wirft.
pub fn event(
    z: &Dienst,
    fall: &mut EigenerFall,
    body: &Value,
    roh: &[u8],
) -> Result<Antwort, ApiFehler> {
    let fall_id = fall.id().clone();
    let sb = z.scheibe_bindung(fall.store())?;
    let Tuer {
        rumpf: b,
        feld_id,
        zustand: zustand_enum,
        herkunft,
        schreiber,
    } = tuer(&sb, body)?;

    // Ab hier `ST.append_event`: jedes `ValueError` ist ein 422 samt Mitschnitt, jede andere
    // Ausnahme ein 500.
    let ordnung: PyWert =
        serde_json::from_slice(roh).unwrap_or_else(|_| PyWert::from(body.clone()));
    let wert_roh = flow::hole(&ordnung, "wert")
        .cloned()
        .unwrap_or(PyWert::Null);
    let abgelegt = ablegen(
        z,
        fall,
        &sb,
        b,
        herkunft,
        &feld_id,
        zustand_enum,
        schreiber,
        &wert_roh,
    );
    let ablage = &z.konfig.audit_dir;
    let id = match abgelegt {
        Ok(id) => id,
        Err(Abbruch::Unerwartet(f)) => return Err(f),
        Err(Abbruch::Abgewiesen(Abgewiesen(grund))) => {
            flow::schreibe(
                ablage,
                Some(fall_id.as_str()),
                "abgewiesen",
                &PyWert::Objekt(vec![
                    ("feld_id".into(), PyWert::Text(feld_id)),
                    ("wert".into(), wert_roh),
                    ("grund".into(), PyWert::Text(kopf200(&grund))),
                ]),
            );
            return Err(ApiFehler::status(422, grund));
        }
    };
    store::speichere(fall.pfad(), fall.store().datei())?;
    let weg = match hole(b, "signal").as_object().map(|o| hole(o, "signal_2")) {
        Some(Value::String(s)) => PyWert::Text(s.clone()),
        _ => PyWert::Null,
    };
    flow::schreibe(
        ablage,
        Some(fall_id.as_str()),
        "antwort",
        &PyWert::Objekt(vec![
            ("feld_id".into(), PyWert::Text(feld_id.clone())),
            ("wert".into(), wert_roh),
            (
                "zustand".into(),
                PyWert::Text(zustand_name(zustand_enum).into()),
            ),
            ("weg".into(), weg),
            (
                "ersetzt".into(),
                PyWert::Bool(crate::python::wahr(hole(b, "ersetzt"))),
            ),
            ("schreiber".into(), PyWert::Text(schreiber.to_owned())),
        ]),
    );
    Ok(Antwort::neu(
        201,
        json!({"event_id": id.to_string(), "feld_id": feld_id, "zustand": zustand_name(zustand_enum)}),
    ))
}

fn zustand_name(z: Zustand) -> &'static str {
    match z {
        Zustand::Vorlaeufig => "vorlaeufig",
        Zustand::Bestaetigt => "bestaetigt",
    }
}

/// `ST.append_event(...)` samt dem Katalog, den `api.event` für Vorschlags-Schreiber lädt.
#[allow(clippy::too_many_arguments)] // die Argumente von `append_event`, wie in Python
fn ablegen(
    z: &Dienst,
    fall: &mut EigenerFall,
    sb: &crate::zustand::ScheibenBindung<'_>,
    b: &Map<String, Value>,
    herkunft: &Map<String, Value>,
    feld_id: &str,
    zustand: Zustand,
    schreiber: &str,
    wert: &PyWert,
) -> Result<store::EventId, Abbruch> {
    let (ts, signal) = (hole(b, "ts"), hole(b, "signal"));
    pruefe_begleitfelder(ts, herkunft, signal)?;
    let Some(herkunft) = herkunft_vektor(herkunft) else {
        return Err(Abbruch::Unerwartet(ApiFehler::unerwartet(
            "ValueError",
            "herkunft nach der Prüfung nicht lesbar",
        )));
    };
    let (signal, signal_2_fremd) = signal_ablage(signal);
    let neu = NeuesEventRoh {
        feld_id: feld_id.to_owned(),
        wert: wert.clone(),
        zustand,
        herkunft,
        schreiber: schreiber.to_owned(),
        signal,
        signal_2_fremd,
        ersetzt: match hole(b, "ersetzt") {
            Value::Null => None,
            w => Some(text(w)),
        },
        ts: ts.as_str().map(str::to_owned),
    };
    let katalog = if VORSCHLAG_PRAEFIXE.iter().any(|p| schreiber.starts_with(p)) {
        Some(z.katalog().map_err(Abbruch::Unerwartet)?)
    } else {
        None
    };
    fall.store_mut()
        .append_roh(&neu, katalog.as_ref(), BindungNachschlag::neu(&sb.index))
        .map_err(|e: AbweisungRoh| Abbruch::Abgewiesen(Abgewiesen(e.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Auftrag 6, Mutant X11: `str(e)[:200]` kürzt auf 200 Zeichen, nicht auf Bytes und nicht auf 199.
    #[test]
    fn kopf200_kuerzt_auf_zweihundert_zeichen() {
        let lang = "ä".repeat(250);
        assert_eq!(kopf200(&lang), "ä".repeat(200));
        let genau = "x".repeat(200);
        assert_eq!(kopf200(&genau), genau);
        assert_eq!(kopf200("kurz"), "kurz");
        assert_eq!(kopf200(""), "");
    }
}
