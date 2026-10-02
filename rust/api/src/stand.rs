//! `GET /fall/{id}/stand` — `api.stand` (`api.py:447-492`): die beantworteten Felder mit
//! Anzeige-Metadaten, die Relevanz der Regeln, der Sperrgrund und die Spanne der festzusetzenden
//! Steuer.
//!
//! Die Reihenfolge der Schritte ist die von Python: erst Scheibe und Bindung, dann der Snapshot,
//! dann der Guard, zuletzt der Ring. Ein Fehler an einer frueheren Stelle verdeckt jeden spaeteren.
use bescheid::deklaration::{an_gesamt_sperrgrund, rentenbeginn_offen_stand, sperrgrund_klartext};
use bescheid::zweige::{bescheid_fn, Umgebung};
use bescheid::{BescheidFehler, BindungIndex, Felder, Instanzquelle};
use domain::{FallId, Sperrgrund, Vz};
use intervall::{AchsenBindung, Intervall, IntervallErgebnis, IntervallFehler, SlotFehler, Spanne};
use serde_json::{json, Map, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::anzeige::{anzeige_metadaten, badge};
use crate::fehler::ApiFehler;
use crate::zustand::Zustand;

/// Die Python-Klasse der Ausnahme, die `_dispatch` an `{"fehler": "Klasse: Text"}` baut.
///
/// PARITÄT: wo Python mit beliebig grossen `int` weiterrechnet und Rust ueberlaeuft, gibt es hier
/// `OverflowError` (500) statt einer Zahl. Das ist ehrlicher als ein stilles Wrap.
fn slot_klasse(e: &SlotFehler<BescheidFehler>) -> &'static str {
    match e {
        SlotFehler::Slot(b) => b.python_klasse().unwrap_or("OverflowError"),
        SlotFehler::UnbekanntesFeld(_) => "KeyError",
        SlotFehler::SummandNichtGanzzahl(_) => "TypeError",
        SlotFehler::Ueberlauf(_) => "OverflowError",
    }
}

fn intervall_fehler(e: &IntervallFehler<SlotFehler<BescheidFehler>>) -> ApiFehler {
    match e {
        IntervallFehler::LeereAchse(_) => ApiFehler::unerwartet("ValueError", e.to_string()),
        IntervallFehler::Ueberlauf(_) => ApiFehler::unerwartet("OverflowError", e.to_string()),
        IntervallFehler::Bescheid(s) => ApiFehler::unerwartet(slot_klasse(s), e.to_string()),
    }
}

/// Der `intervall`-Teil des Ergebnisses (`intervall.py:112-114`) als JSON.
#[must_use]
pub(crate) fn intervall_json(iv: &Intervall) -> Value {
    let (min, max, offen) = match iv.spanne {
        Spanne::NichtFixierbar => (Value::Null, Value::Null, true),
        Spanne::Zahl { min, max, offen } => (min.get().into(), max.get().into(), offen),
    };
    json!({
        "min_cent": min, "max_cent": max, "min_offen": offen, "max_offen": offen,
        "gedeckelt": iv.gedeckelt, "exakt_bzgl_top_k": iv.exakt_bzgl_top_k,
        "rest_felder": iv.rest_felder, "offene_achsen": iv.offene_achsen,
        "nicht_fixierbar": iv.nicht_fixierbar,
    })
}

/// `IV.intervall(felder, bindung, bescheid_fn, snapshot_id=sid)["intervall"]` fuer eine Quantitaet.
///
/// `None`, wenn es zu ihr keinen Accessor gibt (`_bescheid_fn` liefert dort `None`).
fn spanne(
    quantitaet: &str,
    vz: Vz,
    umgebung: &Umgebung<'_>,
    achsen: &[AchsenBindung],
    felder: &Felder,
    store: Option<&Store>,
    sid: &str,
) -> Result<Option<Value>, ApiFehler> {
    // Estimate-Pfad: ein vorlaeufiger Wert zeigt seine Wirkung im Range (`nur_bestaetigt=False`).
    let Some(bf) = bescheid_fn(
        quantitaet,
        vz,
        umgebung,
        Some(felder),
        store,
        false,
        None,
        None,
    ) else {
        return Ok(None);
    };
    let r: IntervallErgebnis =
        intervall::intervall(felder, achsen, |w| bf(w), intervall::CAP_DEFAULT, Some(sid))
            .map_err(|e| intervall_fehler(&e))?;
    Ok(Some(intervall_json(&r.intervall)))
}

/// Das Jahr als [`Vz`], oder der Fehler, den Python spaeter in der Rechnung wirft.
///
/// PARITÄT: Python rechnet mit jedem `int` weiter und scheitert erst im Ring (`params/<vz>`);
/// [`Vz`] kennt nur 2024–2026. Ein Fall mit anderem Jahr kann nur von Hand angelegt werden
/// (`POST /fall` prueft `params/`).
fn jahr(store: &Store) -> Result<Vz, ApiFehler> {
    let j = store.veranlagungszeitraum();
    u16::try_from(j)
        .ok()
        .and_then(|j| Vz::try_from(j).ok())
        .ok_or_else(|| {
            ApiFehler::unerwartet(
                "ValueError",
                format!("kein unterstuetzter Veranlagungszeitraum: {j}"),
            )
        })
}

/// `api.stand(fall_id)` nach dem Owner-Check.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 500 mit der Python-Klasse, wenn Guard, Ring oder Intervall
/// scheitern.
pub fn stand(z: &Zustand, fall_id: &FallId, store: &Store) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let cfg = sb.cfg;
    let (felder, sid) = store
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let sid = sid.to_string();
    let rel = interview::relevanz(store, &sb.sicht, &sb.graph);

    let mut felder_out = Map::new();
    for (fid, v) in &felder {
        let Ok(Value::Object(mut o)) = serde_json::to_value(v) else {
            return Err(ApiFehler::unerwartet(
                "ValueError",
                format!("Feld {fid} nicht darstellbar"),
            ));
        };
        let herkunft = o.get("herkunft").map_or_else(|| badge(&Value::Null), badge);
        o.insert("herkunft_badge".into(), herkunft);
        o.insert(
            "event_id".into(),
            json!(store.aktives(fid).map(|e| e.event_id.to_string())),
        );
        anzeige_metadaten(fid, &sb.index, &mut o);
        felder_out.insert(fid.clone(), Value::Object(o));
    }

    let vz_opt = u16::try_from(store.veranlagungszeitraum())
        .ok()
        .and_then(|j| Vz::try_from(j).ok());
    let gesperrt: Option<Sperrgrund> = if cfg.guard() {
        let q = Instanzquelle {
            store: Some(store),
            bindung: Some(&sb.index),
            nur_bestaetigt: false,
        };
        an_gesamt_sperrgrund(&felder, Some(&cfg), vz_opt, &q).map_err(|e| bescheid_fehler(&e))?
    } else {
        None
    }
    .or_else(|| rentenbeginn_offen_stand(&felder, Some(&cfg)));

    let (mut engine, mut gesamt_iv, mut teil) = ("unavailable", Value::Null, Vec::new());
    if gesperrt.is_some() {
        engine = "gesperrt"; // nicht-ring-faehiger Abzug/Einkunftsart -> kein Ring (K2)
    } else if let Some(q) = cfg.gesamt_ring() {
        let params = z.params()?;
        let (aufbau, achsen) =
            interview::rollen(cfg.kegel_roh(), &sb.sicht, Some(store), &sb.graph);
        let aufbau_achsen: Vec<AchsenBindung> = aufbau.0.iter().map(AchsenBindung::from).collect();
        let kegel_achsen: Vec<AchsenBindung> = achsen.0.iter().map(AchsenBindung::from).collect();
        let umgebung = Umgebung {
            achsen: &aufbau_achsen,
            index: &sb.index,
            params,
        };
        if let Some(iv) = spanne(
            q,
            jahr(store)?,
            &umgebung,
            &kegel_achsen,
            &felder,
            Some(store),
            &sid,
        )? {
            gesamt_iv = iv;
            engine = "catala";
        }
    } else {
        let params = z.params()?;
        for (name, q, tfelder) in cfg.teil_ringe() {
            let tb: Vec<&bindung::Bindung> = tfelder
                .iter()
                .filter_map(|f| sb.index.get(*f).copied())
                .collect();
            let tb_index: BindungIndex<'_> = tb.iter().map(|b| (b.feld_id.clone(), *b)).collect();
            let tb_achsen: Vec<AchsenBindung> =
                tb.iter().map(|b| AchsenBindung::from(*b)).collect();
            let umgebung = Umgebung {
                achsen: &tb_achsen,
                index: &tb_index,
                params,
            };
            // Der Teil-Ring kennt den Store nicht (`_bescheid_fn(q, vz, tb, felder, nur_bestaetigt=False)`).
            if let Some(iv) = spanne(q, jahr(store)?, &umgebung, &tb_achsen, &felder, None, &sid)? {
                teil.push(json!({"familie": name, "quantitaet": q, "intervall": iv}));
            }
        }
        engine = if teil.is_empty() {
            "unavailable"
        } else {
            "catala_teilweise"
        };
    }

    Ok(Antwort::neu(
        200,
        json!({
            "fall_id": fall_id.as_str(), "snapshot_id": sid, "engine": engine,
            "felder": felder_out, "relevanz": rel, "intervall": gesamt_iv,
            "teil_ringe": teil, "ring_gesperrt": gesperrt.map(Sperrgrund::als_str),
            "ring_gesperrt_klartext": sperrgrund_klartext(gesperrt),
        }),
    ))
}

fn bescheid_fehler(e: &BescheidFehler) -> ApiFehler {
    ApiFehler::unerwartet(e.python_klasse().unwrap_or("OverflowError"), e.to_string())
}
