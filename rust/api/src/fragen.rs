//! `GET /fall/{id}/fragen` — `api.fragen` (`api.py:342`): die Fragen-Queue der Scheibe, geordnet
//! nach dem Gewicht, das ein Feld im Ring der Steuer hat, samt Sperrgrund des Rings.
//!
//! Die Reihenfolge der Schritte ist die von Python: Scheibe und Bindung, Snapshot, Guard, Gewichte,
//! Queue, Metadaten, Mitschnitt. Ein Fehler an einer frueheren Stelle verdeckt jeden spaeteren.
use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, sperrgrund_klartext};
use bescheid::zweige::Umgebung;
use bescheid::{BescheidFehler, BindungIndex, Felder, Instanzquelle};
use bindung::{Bindung, Vorjahr};
use domain::{FallId, PyWert, Sperrgrund, Vz};
use elster::parse_instanz;
use engine::zugriff::teil2::EngineFehler;
use intervall::{AchsenBindung, Beitrag, IntervallFehler, SlotFehler};
use interview::AnkerRefSicht;
use serde_json::{json, Map, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::anzeige::enum_labels;
use crate::fehler::ApiFehler;
use crate::flow;
use crate::python::repr;
use crate::stand::{bescheid_fehler, intervall_fehler, jahr, ring, RingFehler};
use crate::zustand::{ScheibenBindung, Zustand};

/// Wie viele Fragen der Mitschnitt als Kopf der Queue nennt (`flow.kopf_der_queue`, Vorgabe).
const KOPF: usize = 6;

/// `RentenfreibetragFixierungOffen`: ein aa-Folgejahr ohne fixierten Rentenfreibetrag laesst den
/// Ring werfen (`api.py:335`).
fn fixierung_offen(e: &RingFehler) -> bool {
    matches!(
        e,
        IntervallFehler::Bescheid(SlotFehler::Slot(BescheidFehler::EngineTeil2(
            EngineFehler::RentenfreibetragFixierungOffen { .. }
        )))
    )
}

/// `{feld_id: spanne_cent}` aus den Beitraegen; bei doppelter `feld_id` gewinnt der spaetere.
fn gewichte(beitraege: &[Beitrag]) -> HashMap<String, i64> {
    beitraege
        .iter()
        .map(|b| (b.feld_id.clone(), b.spanne.get()))
        .collect()
}

/// `_gesamt_beitrag` (`api.py:310`): die Gewichte der Fragen-Reihenfolge aus dem verfuegbaren Ring,
/// Gesamt-Ring bevorzugt, sonst der erste Teil-Ring mit Accessor. `None`, wenn kein Ring rechnet
/// oder er an einem fehlenden Rentenfreibetrag scheitert: dann ordnet `naechste_fragen`
/// deterministisch, und die Liste bleibt vollstaendig.
///
/// Bewusst NICHT „gesperrt → `None`": ein gewoehnlicher Sperrgrund laesst den Ring rechnen.
fn gesamt_beitrag(
    z: &Zustand,
    sb: &ScheibenBindung<'_>,
    store: &Store,
    felder: &Felder,
    sid: &str,
) -> Result<Option<HashMap<String, i64>>, ApiFehler> {
    let params = z.params()?;
    if let Some(q) = sb.cfg.gesamt_ring() {
        let (aufbau, achsen) =
            interview::rollen(sb.cfg.kegel_roh(), &sb.sicht, Some(store), &sb.graph);
        let aufbau_achsen: Vec<AchsenBindung> = aufbau.0.iter().map(AchsenBindung::from).collect();
        let kegel_achsen: Vec<AchsenBindung> = achsen.0.iter().map(AchsenBindung::from).collect();
        let umgebung = Umgebung {
            achsen: &aufbau_achsen,
            index: &sb.index,
            params,
        };
        match ring(
            q,
            jahr(store)?,
            &umgebung,
            &kegel_achsen,
            felder,
            Some(felder),
            Some(store),
            sid,
        ) {
            Ok(Some(r)) => return Ok(Some(gewichte(&r.beitraege))),
            Ok(None) => {}
            Err(e) if fixierung_offen(&e) => return Ok(None),
            Err(e) => return Err(intervall_fehler(&e)),
        }
    }
    for (_name, q, tfelder) in sb.cfg.teil_ringe() {
        let tb: Vec<&Bindung> = tfelder
            .iter()
            .filter_map(|f| sb.index.get(*f).copied())
            .collect();
        let tb_index: BindungIndex<'_> = tb.iter().map(|b| (b.feld_id.clone(), *b)).collect();
        let tb_achsen: Vec<AchsenBindung> = tb.iter().map(|b| AchsenBindung::from(*b)).collect();
        let umgebung = Umgebung {
            achsen: &tb_achsen,
            index: &tb_index,
            params,
        };
        // Der Teil-Ring kennt weder Snapshot noch Store (`_bescheid_fn(q, vz, tb, nur_bestaetigt=False)`).
        match ring(
            q,
            jahr(store)?,
            &umgebung,
            &tb_achsen,
            felder,
            None,
            None,
            sid,
        ) {
            Ok(Some(r)) => return Ok(Some(gewichte(&r.beitraege))),
            Ok(None) => {}
            Err(e) if fixierung_offen(&e) => return Ok(None),
            Err(e) => return Err(intervall_fehler(&e)),
        }
    }
    Ok(None)
}

/// `b.get("bereich")` als Objekt; `grund` nur, wo die Bindung einen nennt (alle 42 Bereiche der
/// Bindung tun es).
fn bereich_json(b: &Bindung) -> Value {
    b.bereich.as_ref().map_or(Value::Null, |ber| {
        let mut m = Map::new();
        m.insert("min".into(), json!(ber.min));
        m.insert("max".into(), json!(ber.max));
        if let Some(g) = &ber.grund {
            m.insert("grund".into(), json!(g));
        }
        Value::Object(m)
    })
}

/// `_frage_metadaten(fid, bindung, store)` (`api.py:380`): alles, was die Oberflaeche braucht, um
/// EINE Frage zu bauen — fuer `fragen` und `frage_einzeln` aus einer Hand.
///
/// # Errors
/// 500 `KeyError`, wenn das Feld nicht in der Bindung steht (Python: `bindung[fid]`).
pub(crate) fn frage_metadaten(
    fid: &str,
    sb: &ScheibenBindung<'_>,
    store: &Store,
) -> Result<Value, ApiFehler> {
    let b = sb
        .index
        .get(fid)
        .copied()
        .ok_or_else(|| ApiFehler::unerwartet("KeyError", repr(&json!(fid))))?;
    let (anzahl, etikett) = interview::instanz_anzahl(store, &sb.sicht, &sb.graph, fid);
    Ok(json!({
        "feld_id": fid,
        "fragetext_laie": b.fragetext_laie,
        "hilfe_kurz": b.hilfe_kurz,
        "typ": b.typ.als_str(),
        "frage_invertiert": b.frage_invertiert,
        "einheit": b.einheit,
        "bereich": bereich_json(b),
        "enum_werte": b.enum_werte,
        "enum_labels": enum_labels(fid),
        "beispielwert": b.beispielwert,
        "muster": b.muster,
        "standardwert": b.standardwert,
        "screening": b.screening.unwrap_or(false),
        "instanz_anzahl": anzahl.get(),
        "instanz_etikett": etikett,
        "anker_ref": AnkerRefSicht::from(&b.anker_ref),
        "regel_id": b.quelle.regel_id,
        "vorjahr_kategorie": b.vorjahr.map(|v| match v {
            Vorjahr::Uebernehmbar => "uebernehmbar",
            Vorjahr::Vorschlag => "vorschlag",
        }),
    }))
}

/// `api.frage_einzeln(fall_id, feld_id)` (`api.py:361`) nach dem Owner-Check: die Frage zu EINEM
/// Feld, auch einem beantworteten. `feld_id__n` wird auf das Basisfeld aufgeloest.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 404, wenn das Basisfeld nicht in der Scheibe steht.
pub fn frage_einzeln(
    z: &Zustand,
    fall_id: &FallId,
    store: &Store,
    feld_id: &str,
) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let basis = parse_instanz(feld_id).map_or(feld_id, |(basis, _)| basis);
    if !sb.index.contains_key(basis) {
        return Err(ApiFehler::status(
            404,
            format!("Feld {} nicht in dieser Scheibe", repr(&json!(feld_id))),
        ));
    }
    Ok(Antwort::neu(
        200,
        json!({"fall_id": fall_id.as_str(), "frage": frage_metadaten(basis, &sb, store)?}),
    ))
}

/// `api.fragen(fall_id)` nach dem Owner-Check.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 500 mit der Python-Klasse, wenn Guard, Ring oder Intervall
/// scheitern.
pub fn fragen(z: &Zustand, fall_id: &FallId, store: &Store) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let (felder, sid) = store
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let sid = sid.to_string();
    let vz: Option<Vz> = u16::try_from(store.veranlagungszeitraum())
        .ok()
        .and_then(|j| Vz::try_from(j).ok());
    // Derselbe Guard wie in `stand`, damit der Sperrgrund auch hier in der Antwort steht — aber ohne
    // dessen Rentenbeginn-Zweig (`api.py:350`).
    let gesperrt: Option<Sperrgrund> = if sb.cfg.guard() {
        let q = Instanzquelle {
            store: Some(store),
            bindung: Some(&sb.index),
            nur_bestaetigt: false,
        };
        an_gesamt_sperrgrund(&felder, Some(&sb.cfg), vz, &q).map_err(|e| bescheid_fehler(&e))?
    } else {
        None
    };
    let beitrag = gesamt_beitrag(z, &sb, store, &felder, &sid)?;
    let queue = interview::naechste_fragen(store, &sb.sicht, &sb.graph, beitrag.as_ref());
    let out: Vec<Value> = queue
        .iter()
        .map(|fid| frage_metadaten(fid, &sb, store))
        .collect::<Result<_, _>>()?;
    let kopf: Vec<PyWert> = out.iter().take(KOPF).cloned().map(PyWert::from).collect();
    let zahl = i64::try_from(out.len()).unwrap_or(i64::MAX);
    flow::schreibe(
        &z.konfig.audit_dir,
        Some(fall_id.as_str()),
        "fragen",
        &PyWert::Objekt(vec![
            ("offen".into(), PyWert::Ganz(zahl)),
            ("kopf".into(), flow::kopf_der_queue(&kopf, KOPF)),
        ]),
    );
    Ok(Antwort::neu(
        200,
        json!({
            "fall_id": fall_id.as_str(), "snapshot_id": sid, "fragen": out,
            "ring_gesperrt": gesperrt.map(Sperrgrund::als_str),
            "ring_gesperrt_klartext": sperrgrund_klartext(gesperrt),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use domain::Cent;

    use super::*;

    /// Das Gewicht ist die Spanne (`spanne_cent`), nicht der kleinere oder groessere Wert; bei
    /// doppelter `feld_id` gewinnt der spaetere Eintrag, wie im `dict` von Python.
    #[test]
    fn gewicht_ist_die_spanne() {
        let b = |id: &str, min: i64, max: i64| Beitrag {
            feld_id: id.to_owned(),
            spanne: Cent::new(max - min),
            min: Cent::new(min),
            max: Cent::new(max),
        };
        let g = gewichte(&[b("a", 100, 105), b("b", 7, 7), b("a", 10, 40)]);
        assert_eq!(
            g,
            HashMap::from([("a".to_owned(), 30), ("b".to_owned(), 0)])
        );
    }

    /// Auftrag 8 (Mutanten H211, H212 am Fall `rentner_folgejahr_ohne_freibetrag` des Orakels): nur der
    /// fehlende Rentenfreibetrag lässt `/fragen` ohne Gewichte antworten; jeder andere Fehler des Rings
    /// ist ein 500.
    #[test]
    fn nur_die_fehlende_fixierung_ist_kein_fehler() {
        let offen = IntervallFehler::Bescheid(SlotFehler::Slot(BescheidFehler::EngineTeil2(
            EngineFehler::RentenfreibetragFixierungOffen {
                beginn: 2020,
                vz: 2025,
            },
        )));
        assert!(fixierung_offen(&offen));
        let anderer = IntervallFehler::Bescheid(SlotFehler::Slot(BescheidFehler::EngineTeil2(
            EngineFehler::VersorgungsfreibetragOffen,
        )));
        assert!(!fixierung_offen(&anderer));
        assert!(!fixierung_offen(&IntervallFehler::LeereAchse("a".into())));
        assert!(!fixierung_offen(&IntervallFehler::Bescheid(
            SlotFehler::UnbekanntesFeld("a".into())
        )));
    }

    /// Auftrag 8 (Mutant H242): `bindung[fid]` für ein Feld, das nicht in der Bindung steht, ist
    /// `KeyError` mit dem `repr` der Feld-ID. `frage_einzeln` und die Queue prüfen vorher; erreichbar ist
    /// der Zweig über HTTP nicht.
    #[test]
    fn frage_metadaten_fuer_ein_fremdes_feld_ist_key_error() {
        let tmp = tempfile::tempdir().unwrap();
        let konfig = crate::konfig::Konfig {
            wurzel: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
            faelle: tmp.path().join("faelle"),
            audit_dir: tmp.path().join("faelle"),
        };
        let auth = auth::Auth::neu(
            "testgeheimnis".into(),
            tmp.path().join("users.json"),
            Some(konfig.audit_pfad()),
        );
        let z = Zustand::neu(konfig, auth);
        let mut datei = Store::leer(2025, Some("k1".into())).into_datei();
        datei.scheibe = Some("gesamt".into());
        let store = Store::aus_datei(datei);
        let sb = z.scheibe_bindung(&store).unwrap();
        let ApiFehler::Unerwartet { typ, meldung } =
            frage_metadaten("gibt_es_nicht", &sb, &store).unwrap_err()
        else {
            panic!("kein Ausnahme-Fehler")
        };
        assert_eq!(
            (typ.as_str(), meldung.as_str()),
            ("KeyError", "'gibt_es_nicht'")
        );
        let frage = frage_metadaten("bruttoarbeitslohn", &sb, &store).unwrap();
        assert_eq!(frage["feld_id"], json!("bruttoarbeitslohn"));
    }
}
