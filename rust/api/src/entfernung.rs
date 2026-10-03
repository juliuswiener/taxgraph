//! `api.entfernung` (`api.py:886`) mit der Auswertung der Karten-Antworten aus `ors_client.py`
//! (`geocode`, `_distanz_meter`, `entfernung_km`).
//!
//! Der Transport ([`llm::ors`]) liefert jede Antwort als JSON-Wert. Hier steht, was Python mit ihr
//! tut — auch mit einer Antwort in falscher Gestalt: `feats[0]`, `.get(…)`, `in` und `float(…)`
//! werfen dort `AttributeError`, `KeyError` oder `TypeError`, die als 500 mit `"<Klasse>: <Meldung>"`
//! zum Client gehen (P5). Nur `OrsNichtVerfuegbar` (kein Schlüssel, Netz, HTTP, JSON, keine
//! Koordinate, keine Distanz) ist ein 503 mit dem Fallback-Vertrag.
//!
//! # Abweichungen von Python
//! Die Meldungen sind die von Python 3.14 (`python3` dieses Rechners); ≤ 3.13 sagt bei `in` auf einer
//! Zahl nur „is not iterable“.
//!
//! `ponytail`: eine Zahl im Antwort-Text, die kein `f64` ist (`1e400`, `NaN`, `Infinity`, eine
//! Ganzzahl über `f64::MAX`), liest `serde_json` nicht; dort steht 503 statt Pythons 500 oder 200.
//! Eine Ganzzahl über `u64` liest `serde_json` als `float`; Pythons Meldung nennt dann `int`.
use domain::{Achsenwert, Herkunft, HerkunftVektor, PruefTiefe, PyWert, Zustand};
use eingang::kontoauszug::py_float;
use llm::ors::{NichtVerfuegbar, Ors};
use serde_json::{json, Value};
use store::fehler_log::{protokolliere, FallId, Meta, Stufe};
use store::{Abweisung, BindungNachschlag, NeuesEventRoh, Signal};

use crate::antwort::Antwort;
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::python::{repr, typname, wahr};
use crate::zustand::Zustand as Dienst;

/// Das Feld, das `entfernung` schreibt.
const FELD: &str = "ep_entfernung_km";

/// `ENTFERNUNG_FALLBACK["vertrag"]` (`api.py:881`).
const VERTRAG: &str = "Der Karten-Dienst ist nicht verbunden (kein Schlüssel gesetzt oder \
Netz-/Antwort-Fehler) — bitte gib die Entfernung manuell ein (kürzeste Straßenverbindung, § 9 Abs. 1 \
S. 3 Nr. 4 EStG).";

/// Ab so vielen Kilometern weist Auflage F2 den Vorschlag ab (`abs(wert) >= 10**10`).
const MAGNITUDE: f64 = 10_000_000_000.0;

static NICHTS: Value = Value::Null;

/// Wie eine Auswertung endet, wenn sie nicht gelingt.
#[derive(Debug)]
enum Ausfall {
    /// Pythons `OrsNichtVerfuegbar`: 503 mit dem Fallback und ein Eintrag im Fehlerlog.
    Nicht(NichtVerfuegbar),
    /// Jede andere Ausnahme: 500.
    Python(ApiFehler),
}

fn nicht(text: &str) -> Ausfall {
    Ausfall::Nicht(NichtVerfuegbar(text.to_owned()))
}

fn python(typ: &str, meldung: impl Into<String>) -> Ausfall {
    Ausfall::Python(ApiFehler::unerwartet(typ, meldung))
}

fn kein_get(v: &Value) -> Ausfall {
    python(
        "AttributeError",
        format!("'{}' object has no attribute 'get'", typname(v)),
    )
}

/// `(j or {}).get(schluessel)`: ein falsches `j` ist `{}`, ein wahres Nicht-Objekt hat kein `get`.
fn hole<'a>(j: &'a Value, schluessel: &str) -> Result<&'a Value, Ausfall> {
    match j {
        Value::Object(o) => Ok(o.get(schluessel).unwrap_or(&NICHTS)),
        andere if wahr(andere) => Err(kein_get(andere)),
        _ => Ok(&NICHTS),
    }
}

/// `x.get(schluessel)` ohne den Schutz durch `or {}`: nur ein Objekt hat `get`.
fn get_roh<'a>(x: &'a Value, schluessel: &str) -> Result<&'a Value, Ausfall> {
    match x {
        Value::Object(o) => Ok(o.get(schluessel).unwrap_or(&NICHTS)),
        andere => Err(kein_get(andere)),
    }
}

/// `x[0]` für ein WAHRES `x`: Liste und Text liefern das erste Element, ein Objekt hat keinen
/// Schlüssel `0`, eine Zahl lässt sich nicht indizieren.
fn erstes(x: &Value) -> Result<Value, Ausfall> {
    match x {
        Value::Array(a) => Ok(a.first().cloned().unwrap_or(Value::Null)),
        Value::String(s) => Ok(Value::String(s.chars().take(1).collect())),
        Value::Object(_) => Err(python("KeyError", "0")),
        andere => Err(python(
            "TypeError",
            format!("'{}' object is not subscriptable", typname(andere)),
        )),
    }
}

/// `schluessel in container`.
fn enthaelt(container: &Value, schluessel: &str) -> Result<bool, Ausfall> {
    match container {
        Value::Object(o) => Ok(o.contains_key(schluessel)),
        Value::Array(a) => Ok(a.iter().any(|e| e.as_str() == Some(schluessel))),
        Value::String(s) => Ok(s.contains(schluessel)),
        andere => Err(python(
            "TypeError",
            format!(
                "argument of type '{}' is not a container or iterable",
                typname(andere)
            ),
        )),
    }
}

/// `container[schluessel]`, nachdem [`enthaelt`] ihn gefunden hat: nur ein Objekt hat Textschlüssel.
fn mit_schluessel<'a>(container: &'a Value, schluessel: &str) -> Result<&'a Value, Ausfall> {
    match container {
        Value::Object(o) => Ok(o.get(schluessel).unwrap_or(&NICHTS)),
        Value::Array(_) => Err(python(
            "TypeError",
            "list indices must be integers or slices, not str",
        )),
        _ => Err(python(
            "TypeError",
            "string indices must be integers, not 'str'",
        )),
    }
}

/// `geocode` ab der Antwort (`ors_client.py:73-78`): die Koordinatenliste des ersten Treffers.
fn koordinaten(j: &Value) -> Result<Value, Ausfall> {
    let features = hole(j, "features")?;
    let element = if wahr(features) {
        Some(erstes(features)?)
    } else {
        None
    };
    let geometrie = match &element {
        Some(e) => get_roh(e, "geometry")?,
        None => &NICHTS,
    };
    let coords = if wahr(geometrie) {
        get_roh(geometrie, "coordinates")?
    } else {
        &NICHTS
    };
    match coords {
        Value::Array(c) if element.is_some() && c.len() >= 2 => Ok(coords.clone()),
        _ => Err(nicht("keine verwertbare Koordinate für Adresse gefunden")),
    }
}

/// `float(x)`.
fn als_float(x: &Value) -> Result<f64, Ausfall> {
    match x {
        Value::Number(n) => Ok(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => Ok(f64::from(u8::from(*b))),
        Value::String(s) => py_float(s).ok_or_else(|| {
            python(
                "ValueError",
                format!("could not convert string to float: {}", repr(x)),
            )
        }),
        andere => Err(python(
            "TypeError",
            format!(
                "float() argument must be a string or a real number, not '{}'",
                typname(andere)
            ),
        )),
    }
}

/// `_distanz_meter` ab der Antwort (`ors_client.py:77-80`).
fn distanz_meter(j: &Value) -> Result<f64, Ausfall> {
    let keine = || nicht("ORS-Antwort ohne verwertbare Distanz");
    let routes = hole(j, "routes")?;
    if !wahr(routes) {
        return Err(keine());
    }
    let route = erstes(routes)?;
    if !enthaelt(&route, "summary")? {
        return Err(keine());
    }
    let summary = mit_schluessel(&route, "summary")?;
    if !enthaelt(summary, "distance")? {
        return Err(keine());
    }
    als_float(mit_schluessel(summary, "distance")?)
}

/// Das Ergebnis von `max(0, round(meter / 1000))`.
#[derive(Debug, PartialEq)]
enum Km {
    Ganz(i64),
    /// Ab 10^10: die Ziffern der Ganzzahl, die Python ausgibt.
    ZuGross(String),
}

/// `max(0, round(meter / 1000))` (`ors_client.py:97`): `round` rundet gerade, `NaN` und `inf` sind
/// keine Ganzzahl.
fn km_aus(meter: f64) -> Result<Km, Ausfall> {
    let gerundet = (meter / 1000.0).round_ties_even();
    if gerundet.is_nan() {
        return Err(python("ValueError", "cannot convert float NaN to integer"));
    }
    if gerundet.is_infinite() {
        return Err(python(
            "OverflowError",
            "cannot convert float infinity to integer",
        ));
    }
    let km = gerundet.max(0.0);
    if km >= MAGNITUDE {
        // `{:.0}` druckt den vollen Wert der Gleitkommazahl, wie Pythons `int(float)`.
        return Ok(Km::ZuGross(format!("{km:.0}")));
    }
    #[allow(clippy::cast_possible_truncation)] // 0 <= km < 10^10
    Ok(Km::Ganz(km as i64))
}

/// `entfernung_km(von, nach)` (`ors_client.py:93-98`): zwei Geocodes, ein Routing.
fn km_vom_dienst(von: &str, nach: &str) -> Result<Km, Ausfall> {
    let ors = Ors::aus_env().map_err(Ausfall::Nicht)?;
    let a = koordinaten(&ors.geocode(von).map_err(Ausfall::Nicht)?)?;
    let b = koordinaten(&ors.geocode(nach).map_err(Ausfall::Nicht)?)?;
    km_aus(distanz_meter(&ors.route(&a, &b).map_err(Ausfall::Nicht)?)?)
}

/// `(body.get(schluessel) or "").strip()`: ein falscher Wert ist leer, ein wahrer Nicht-Text hat
/// kein `strip`.
fn adresse(rumpf: &serde_json::Map<String, Value>, schluessel: &str) -> Result<String, ApiFehler> {
    match rumpf.get(schluessel).unwrap_or(&NICHTS) {
        Value::String(s) => Ok(llm::py::strip(s).to_owned()),
        andere if wahr(andere) => Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'strip'", typname(andere)),
        )),
        _ => Ok(String::new()),
    }
}

/// `api.entfernung(fall_id, body)` nach dem Owner-Check. Blockiert bis zu 8 s je Dienstaufruf.
///
/// # Errors
/// 400 ohne Adressen oder ohne Arbeitsweg-Feld in der Scheibe; 422, wenn der Store den Vorschlag
/// abweist; 500 mit der Python-Klasse, wo Python eine andere Ausnahme wirft.
pub fn entfernung(
    z: &Dienst,
    fall: &mut EigenerFall,
    body: &Value,
) -> Result<Antwort, ApiFehler> {
    let Value::Object(rumpf) = body else {
        return Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", typname(body)),
        ));
    };
    let von = adresse(rumpf, "von")?;
    let nach = adresse(rumpf, "nach")?;
    if von.is_empty() || nach.is_empty() {
        return Err(ApiFehler::status(400, "von und nach (Adressen) sind Pflicht"));
    }
    let sb = z.scheibe_bindung(fall.store())?;
    if !sb.index.contains_key(FELD) {
        return Err(ApiFehler::status(
            400,
            "diese Scheibe hat kein Arbeitsweg-km-Feld",
        ));
    }
    let km = match km_vom_dienst(&von, &nach) {
        Ok(km) => km,
        Err(Ausfall::Python(f)) => return Err(f),
        Err(Ausfall::Nicht(e)) => {
            // WARNUNG, kein FEHLER: der Ausfall ist erwartet. Keine Adressen im Protokoll.
            // ponytail: die PII-Muster liefert erst `llm::pii`; bis dahin steht `<gesperrt:…>`.
            let _ = protokolliere(
                &z.konfig.fehler_pfad(),
                "api.entfernung ors",
                &e,
                Stufe::Warnung,
                Some(FallId::pruefe(fall.id().as_str(), &[])),
                Meta::default(),
            );
            return Ok(Antwort::neu(
                503,
                json!({"fehler": "unavailable", "vertrag": VERTRAG}),
            ));
        }
    };
    let km = match km {
        Km::ZuGross(wert) => {
            // Auflage F2, vor dem Store: `PyWert` trägt keine Ganzzahl über `u64`. Die Auflagen A und
            // K1 davor sind für diesen Schreiber und dieses Feld immer erfüllt.
            let grund = Abweisung::Magnitude {
                feld_id: FELD.to_owned(),
                schreiber: "berechnet:maps".to_owned(),
                wert,
            };
            return Err(ApiFehler::status(422, grund.to_string()));
        }
        Km::Ganz(km) => km,
    };
    let ersetzt = fall
        .store()
        .aktives(FELD)
        .map(|e| e.event_id.to_string());
    let achse = |t: &str| {
        Achsenwert::new(t).map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))
    };
    let neu = NeuesEventRoh {
        feld_id: FELD.to_owned(),
        wert: PyWert::Ganz(km),
        zustand: Zustand::Vorlaeufig,
        herkunft: HerkunftVektor::Voll(Herkunft {
            herkunft: achse("berechnet")?,
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: achse("nutzer")?,
        }),
        schreiber: "berechnet:maps".to_owned(),
        signal: Signal {
            signal_1: Some(Some(PyWert::Objekt(vec![
                ("typ".to_owned(), PyWert::Text("maps".to_owned())),
                (
                    "dienst".to_owned(),
                    PyWert::Text("openrouteservice".to_owned()),
                ),
            ]))),
            signal_2: None,
        },
        signal_2_fremd: None,
        ersetzt,
        ts: None,
    };
    let katalog = z.katalog()?;
    let id = fall
        .store_mut()
        .append_roh(&neu, Some(&katalog), BindungNachschlag::neu(&sb.index))
        .map_err(|e| ApiFehler::status(422, e.to_string()))?;
    store::speichere(fall.pfad(), fall.store().datei())?;
    Ok(Antwort::neu(
        200,
        json!({"km": km, "event_id": id.to_string(), "herkunft": "berechnet",
               "hinweis": "Vorschlag aus dem Karten-Dienst — bitte prüfen und bestätigen."}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fehler(a: Result<impl std::fmt::Debug, Ausfall>) -> String {
        match a.unwrap_err() {
            Ausfall::Nicht(n) => format!("503 {n}"),
            Ausfall::Python(ApiFehler::Unerwartet { typ, meldung }) => format!("{typ}: {meldung}"),
            Ausfall::Python(f) => format!("{f:?}"),
        }
    }

    /// Pythons Ausnahmen bei Antworten in falscher Gestalt, Wort für Wort aus `h8/f6b/messe_*`
    /// (Python 3.14).
    #[test]
    fn koordinaten_wie_python() {
        let ok = json!({"features": [{"geometry": {"coordinates": [11.5, 48.1]}}]});
        assert_eq!(koordinaten(&ok).unwrap(), json!([11.5, 48.1]));
        for (j, soll) in [
            (json!({"features": []}), "503 keine verwertbare Koordinate für Adresse gefunden"),
            (json!({}), "503 keine verwertbare Koordinate für Adresse gefunden"),
            (json!(null), "503 keine verwertbare Koordinate für Adresse gefunden"),
            (json!({"features": [{"geometry": {"coordinates": [11.5]}}]}), "503 keine verwertbare Koordinate für Adresse gefunden"),
            (json!({"features": ["x"]}), "AttributeError: 'str' object has no attribute 'get'"),
            (json!({"features": [null]}), "AttributeError: 'NoneType' object has no attribute 'get'"),
            (json!([1]), "AttributeError: 'list' object has no attribute 'get'"),
            (json!(5), "AttributeError: 'int' object has no attribute 'get'"),
            (json!({"features": {"a": 1}}), "KeyError: 0"),
            (json!({"features": "abc"}), "AttributeError: 'str' object has no attribute 'get'"),
            (json!({"features": 5}), "TypeError: 'int' object is not subscriptable"),
            (json!({"features": [{"geometry": "x"}]}), "AttributeError: 'str' object has no attribute 'get'"),
        ] {
            assert_eq!(fehler(koordinaten(&j)), soll, "{j}");
        }
    }

    #[test]
    fn distanz_wie_python() {
        let route = |d: Value| json!({"routes": [{"summary": {"distance": d}}]});
        assert!((distanz_meter(&route(json!(30400.0))).unwrap() - 30400.0).abs() < f64::EPSILON);
        assert!((distanz_meter(&route(json!("12000"))).unwrap() - 12000.0).abs() < f64::EPSILON);
        assert!((distanz_meter(&route(json!(true))).unwrap() - 1.0).abs() < f64::EPSILON);
        let keine = "503 ORS-Antwort ohne verwertbare Distanz";
        for (j, soll) in [
            (json!({"routes": []}), keine),
            (json!({"routes": "x"}), keine),
            (json!({"routes": [{"summary": {}}]}), keine),
            (json!({"routes": [{"summary": []}]}), keine),
            (route(json!("abc")), "ValueError: could not convert string to float: 'abc'"),
            (route(json!(null)), "TypeError: float() argument must be a string or a real number, not 'NoneType'"),
            (route(json!([1])), "TypeError: float() argument must be a string or a real number, not 'list'"),
            (json!({"routes": [5]}), "TypeError: argument of type 'int' is not a container or iterable"),
            (json!({"routes": [null]}), "TypeError: argument of type 'NoneType' is not a container or iterable"),
            (json!({"routes": [{"summary": 5}]}), "TypeError: argument of type 'int' is not a container or iterable"),
            (json!({"routes": ["summary"]}), "TypeError: string indices must be integers, not 'str'"),
            (json!({"routes": [["summary"]]}), "TypeError: list indices must be integers or slices, not str"),
            (json!({"routes": [{"summary": ["distance"]}]}), "TypeError: list indices must be integers or slices, not str"),
            (json!({"routes": {"a": 1}}), "KeyError: 0"),
            (json!([1]), "AttributeError: 'list' object has no attribute 'get'"),
        ] {
            assert_eq!(fehler(distanz_meter(&j)), soll, "{j}");
        }
    }

    /// `round` rundet gerade; die große Ganzzahl trägt alle Ziffern (`int(1e27)` in Python).
    #[test]
    fn km_rundet_gerade_und_nennt_die_ganze_zahl() {
        assert_eq!(km_aus(30_400.0).unwrap(), Km::Ganz(30));
        assert_eq!(km_aus(30_500.0).unwrap(), Km::Ganz(30));
        assert_eq!(km_aus(31_500.0).unwrap(), Km::Ganz(32));
        assert_eq!(km_aus(-5000.0).unwrap(), Km::Ganz(0));
        assert_eq!(km_aus(1e30).unwrap(), Km::ZuGross("1000000000000000013287555072".into()));
        assert_eq!(km_aus(1e13).unwrap(), Km::ZuGross("10000000000".into()));
        assert_eq!(km_aus(9.99e12).unwrap(), Km::Ganz(9_990_000_000));
        assert_eq!(fehler(km_aus(f64::NAN)), "ValueError: cannot convert float NaN to integer");
        assert_eq!(fehler(km_aus(f64::INFINITY)), "OverflowError: cannot convert float infinity to integer");
    }

    #[test]
    fn adresse_wie_python() {
        let rumpf = |v: Value| v.as_object().cloned().unwrap();
        assert_eq!(adresse(&rumpf(json!({"von": " a b "})), "von").unwrap(), "a b");
        for leer in [json!({}), json!({"von": null}), json!({"von": ""}), json!({"von": 0}), json!({"von": []}), json!({"von": false})] {
            assert_eq!(adresse(&rumpf(leer), "von").unwrap(), "");
        }
        let ApiFehler::Unerwartet { typ, meldung } =
            adresse(&rumpf(json!({"von": 5})), "von").unwrap_err()
        else {
            panic!("500 erwartet");
        };
        assert_eq!((typ.as_str(), meldung.as_str()), ("AttributeError", "'int' object has no attribute 'strip'"));
    }
}
