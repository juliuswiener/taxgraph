//! `api.deklaration` (`api.py:665`): der Snapshot als Kz-Tabelle der Steuererklaerung — ohne XML,
//! ohne Versand. Die Ring-Werte (Verpflegungskuerzung, Kapital-Antrag, haushaltsnahe Summen)
//! kommen vorher in den Snapshot (`bescheid::deklaration::mit_ring_werten`).
use bescheid::deklaration::mit_ring_werten;
use domain::{FallId, Vz};
use elster::DeklarationsFehler;
use serde_json::{json, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::stand::bescheid_fehler;
use crate::zustand::Zustand;

/// Python-Klasse und Text der Ausnahme aus `est_mapping.deklariere`. Die Fehler des Jahres tragen
/// Pythons Text (`DeklarationsFehler::Jahr`, `est_mapping.py:220-229`).
///
/// ponytail: bei `Wert` und `SnapshotObjekt`/`KeinFeldGebunden` ist nur die Klasse das
/// Paritaetskriterium (`elster_paritaet`); der Text kann von Pythons abweichen. Alle drei brauchen
/// einen Store-Wert oder eine Eingabe, die `POST /event` und die Bindung nicht durchlassen.
fn deklarations_fehler(e: &DeklarationsFehler) -> ApiFehler {
    let text = match e {
        DeklarationsFehler::Jahr(f) | DeklarationsFehler::Wert { fehler: f, .. } => {
            f.nachricht.clone()
        }
        _ => e.to_string(),
    };
    ApiFehler::unerwartet(e.python_klasse(), text)
}

/// `api.deklaration(fall_id)` nach dem Owner-Check.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 500 mit der Python-Klasse, wenn der Ring (nur Ueberlauf) oder
/// `elster::deklariere` scheitert (Jahr 0 oder unplausibel, kein Feld in der Bindung).
pub fn deklaration(z: &Zustand, fall_id: &FallId, store: &Store) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let (mut felder, sid) = store
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    // Python: `int(store.get("veranlagungszeitraum") or 0)`; ein Jahr ausserhalb von `Vz` heisst
    // fuer den Ring "kein Jahr mit Parametern" (`mit_ring_werten`, `vz: None`).
    let vz = store.veranlagungszeitraum();
    let ring_jahr: Option<Vz> = u16::try_from(vz).ok().and_then(|j| Vz::try_from(j).ok());
    mit_ring_werten(&mut felder, ring_jahr, z.params()?).map_err(|e| bescheid_fehler(&e))?;
    let d = elster::deklariere(&felder, &sb.index, vz, Some(&sid))
        .map_err(|e| deklarations_fehler(&e))?;
    let Value::Object(mut koerper) =
        serde_json::to_value(&d).map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?
    else {
        return Err(ApiFehler::unerwartet(
            "TypeError",
            "Deklaration ist kein Objekt",
        ));
    };
    koerper.insert("fall_id".to_owned(), json!(fall_id.as_str()));
    Ok(Antwort::neu(200, Value::Object(koerper)))
}
