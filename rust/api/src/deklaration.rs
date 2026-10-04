//! `api.deklaration` (`api.py:665`): der Snapshot als Kz-Tabelle der Steuererklaerung — ohne XML,
//! ohne Versand. Die Ring-Werte (Verpflegungskuerzung, Kapital-Antrag, haushaltsnahe Summen)
//! kommen vorher in den Snapshot (`bescheid::deklaration::mit_ring_werten`).
use bescheid::deklaration::{an_gesamt_sperrgrund, mit_ring_werten, sperrgrund_klartext_text};
use bescheid::Instanzquelle;
use domain::{FallId, Vz};
use elster::DeklarationsFehler;
use serde_json::{json, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::stand::{bescheid_fehler, ueberlauf_422};
use crate::zustand::Zustand;

/// Python-Klasse und Text der Ausnahme aus `est_mapping.deklariere`. Die Fehler des Jahres tragen
/// Pythons Text (`DeklarationsFehler::Jahr`, `est_mapping.py:220-229`).
///
/// ponytail: bei `Wert` und `SnapshotObjekt`/`KeinFeldGebunden` ist nur die Klasse das
/// Paritaetskriterium (`elster_paritaet`); der Text kann von Pythons abweichen. Alle drei brauchen
/// einen Store-Wert oder eine Eingabe, die `POST /event` und die Bindung nicht durchlassen.
///
/// Ausnahme `Ueberlauf`: Python wirft dort nichts, es rechnet exakt. Rust antwortet 422 wie bei
/// jedem Betrag, den die Rechnung nicht fasst ([`ueberlauf_422`]), nicht 500 mit einer Klasse, die
/// es in Python nie gab.
pub(crate) fn deklarations_fehler(e: &DeklarationsFehler) -> ApiFehler {
    if matches!(e, DeklarationsFehler::Ueberlauf { .. }) {
        return ueberlauf_422(e);
    }
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
/// 400/500 aus Scheibe und Bindung; 500 mit der Python-Klasse, wenn der Ring (nur Ueberlauf), der
/// Guard oder `elster::deklariere` scheitert (Jahr 0 oder unplausibel, kein Feld in der Bindung).
/// Meldet der Guard einen Sperrgrund, ist das keine Fehler-Variante, sondern die Antwort 409.
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
    // Ring -> Guard -> deklariere, wie `einreichung.rs` und `api.einreichen`: eine Vorschau, die Werte
    // aus einem gesperrten Fall zeigt, widerspraeche `/ergebnis` und `/einreichen`. Der Koerper ist
    // `fall_id`, `grund`, `klartext` (Python: `api.deklaration`).
    if sb.cfg.guard() {
        // Der Guard sieht Roh-Felder; `nur_bestaetigt` liest er nicht.
        let q = Instanzquelle {
            store: Some(store),
            bindung: Some(&sb.index),
            nur_bestaetigt: false,
        };
        if let Some(g) = an_gesamt_sperrgrund(&felder, Some(&sb.cfg), ring_jahr, &q)
            .map_err(|e| bescheid_fehler(&e))?
        {
            return Ok(Antwort::neu(
                409,
                json!({
                    "fall_id": fall_id.as_str(),
                    "grund": g.als_str(),
                    "klartext": sperrgrund_klartext_text(Some(g.als_str())),
                }),
            ));
        }
    }
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

#[cfg(test)]
mod tests {
    use elster::DeklarationsFehler;

    use super::deklarations_fehler;
    use crate::fehler::ApiFehler;

    /// `DeklarationsFehler::Ueberlauf` ist 422 mit dem Betrags-Text, kein 500 mit einer Klasse, die
    /// Python an dieser Stelle nie wirft. Die Stelle ist ueber HTTP nicht erreichbar (§ 23 sperrt
    /// der Guard vor `deklariere()`); darum der Test auf der Abbildung.
    #[test]
    fn ueberlauf_der_deklaration_ist_422_und_ohne_wert() {
        let e = DeklarationsFehler::Ueberlauf {
            feld_id: "p23_veraeusserung__1".to_owned(),
            was: "Differenz jenseits i64",
        };
        assert!(
            matches!(
                deklarations_fehler(&e),
                ApiFehler::Status(422, ref t)
                    if t.contains("zu groß") && t.contains("p23_veraeusserung__1")
            ),
            "{:?}",
            deklarations_fehler(&e)
        );
        // KONTROLLE: ein anderer Fehler bleibt 500 mit seiner Python-Klasse.
        assert!(
            matches!(
                deklarations_fehler(&DeklarationsFehler::KeinFeldGebunden),
                ApiFehler::Unerwartet { ref typ, .. } if typ == "ValueError"
            ),
            "{:?}",
            deklarations_fehler(&DeklarationsFehler::KeinFeldGebunden)
        );
    }
}
