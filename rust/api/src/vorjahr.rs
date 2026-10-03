//! `api.vorjahr` (`api.py:928`): die im Vorjahres-Fall BESTÄTIGTEN Felder mit Flag `vorjahr` werden
//! als VORLÄUFIGE Vorschläge (`import:vorjahr`) in den Fall übernommen, der schon belegte Felder
//! nicht anfasst. Übernommen wird in der Bibliothek `eingang`; diese Schicht prüft die zweite
//! Fall-Kennung des Rumpfs wie die erste, ruft in der Reihenfolge von Python auf und schreibt die Akte.
use domain::PyWert;
use eingang::vorjahr::{uebernehme_in_reihenfolge, VorjahrFeld};
use eingang::vorschlag::SchreibFehler;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use store::BindungNachschlag;

use crate::antwort::Antwort;
use crate::eigener_fall::{fall_kennung, ist_fall_id, EigenerFall};
use crate::fehler::ApiFehler;
use crate::python::{text, typname, wahr};
use crate::zustand::{Nutzer, Zustand as Dienst};

/// Ein fehlender Schlüssel des Rumpfs ist `None` (`body.get(...)`).
static NICHTS: Value = Value::Null;

/// Der Rumpf nach dem Owner-Check des Ziels: Quelle prüfen und laden, übernehmen, speichern.
///
/// # Errors
/// 400 für die Kennung der Quelle, 401/403/404 wie beim Ziel, 422 für eine Abweisung des Stores,
/// 500 mit der Python-Klasse für alles, was Python nicht fängt.
pub fn vorjahr(
    z: &Dienst,
    nutzer: &Nutzer,
    fall: &mut EigenerFall,
    body: &Value,
) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(fall.store())?;
    let Value::Object(b) = body else {
        return Err(ApiFehler::unerwartet(
            "AttributeError",
            format!("'{}' object has no attribute 'get'", typname(body)),
        ));
    };
    let quelle_roh = b.get("vorjahr_fall_id").unwrap_or(&NICHTS);
    // `not vj_id or not _FALL_RE.fullmatch(str(vj_id))`: falsch, oder ein `str` außerhalb von
    // `[A-Za-z0-9_-]{1,64}`, ist 400. Eine Ganzzahl, `true` und eine Kommazahl wie `1e-05` bestehen es.
    if !wahr(quelle_roh) || !ist_fall_id(&text(quelle_roh)) {
        return Err(ApiFehler::status(
            400,
            "vorjahr_fall_id fehlt oder ungültig",
        ));
    }
    if quelle_roh.as_str() == Some(fall.id().as_str()) {
        return Err(ApiFehler::status(
            400,
            "vorjahr_fall_id muss ein ANDERER (Vorjahres-)Fall sein",
        ));
    }
    // Ein Nicht-Text, dessen `str` passt, kommt bis `lade_fall`; `_FALL_RE.fullmatch` wirft dort
    // `TypeError` (500).
    let Value::String(vj_id) = quelle_roh else {
        return Err(ApiFehler::unerwartet(
            "TypeError",
            format!(
                "expected string or bytes-like object, got '{}'",
                typname(quelle_roh)
            ),
        ));
    };
    // Die QUELLE gehört derselben Prüfung wie das Ziel (`_fall_owner_check` + `lade_fall`): sonst
    // zöge ein Nutzer Felder aus einem fremden Fall in seinen eigenen.
    let quelle = EigenerFall::pruefe(z, nutzer, &fall_kennung(vj_id)?)?;
    let (felder, _) = quelle
        .store()
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let felder: BTreeMap<String, VorjahrFeld> = serde_json::to_value(&felder)
        .and_then(serde_json::from_value)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    // ponytail: ein Jahr außerhalb von i64 (nur eine Akte von Hand) steht als i64::MAX im Beleg
    // (`signal_1.vz`); Python hat die exakte Ganzzahl. Upgrade: `uebernehme` mit `i128`.
    let vz = i64::try_from(quelle.store().datei().veranlagungszeitraum.0).unwrap_or(i64::MAX);
    // In der Reihenfolge der Scheibe, wie Python `bindung.items()` durchgeht: so stehen die Events in
    // der Akte, und eine Abweisung bricht an derselben Stelle ab.
    let erg = uebernehme_in_reihenfolge(
        fall.store_mut(),
        &felder,
        BindungNachschlag::neu(&sb.index),
        &sb.felder,
        vz,
        None,
    )
    .map_err(|e| match e {
        // Python: jeder `ValueError` aus dem Writer ist 422 mit seinem Text, wie bei `/event`.
        SchreibFehler::Abweisung(a) => ApiFehler::status(422, a.to_string()),
        SchreibFehler::Konstante => ApiFehler::unerwartet("ValueError", e.to_string()),
    })?;
    if let Some(referenz) = erg.referenz {
        fall.store_mut()
            .setze_vorjahr_referenz(PyWert::from(referenz));
    }
    store::speichere(fall.pfad(), fall.store().datei())?;
    let mut aus = Map::new();
    aus.insert("uebernommen".into(), json!(erg.uebertragen));
    aus.insert("uebersprungen".into(), json!(erg.uebersprungen));
    aus.insert("vorjahr_fall_id".into(), json!(vj_id));
    Ok(Antwort::neu(200, Value::Object(aus)))
}
