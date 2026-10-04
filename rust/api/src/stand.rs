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

/// 422 fuer einen Betrag, den die Rechnung nicht fasst.
///
/// GEWOLLTE ABWEICHUNG (Korrektheit vor Paritaet, `REWRITE_PLAN.md` §4): Python rechnet mit beliebig
/// grossen `int` weiter und antwortet 200 mit einer Zahl; Rust rechnet in `i64`. Ein Betrag, der dort
/// ueberlaeuft (z. B. `bruttoarbeitslohn` = `i64::MIN` ueber `POST /event`, das ihn annimmt), ist eine
/// Eingabe des Nutzers und kein Programmfehler: 422 mit einer Meldung, die sagt, dass ein Betrag zu gross
/// ist, statt eines 500 mit einer Python-Klasse, die es dort nie gab. Der Text traegt keinen Wert aus dem Store.
fn ueberlauf_422(e: &dyn std::fmt::Display) -> ApiFehler {
    ApiFehler::status(
        422,
        format!(
            "Ein eingegebener Betrag ist zu groß für die Berechnung — bitte prüfe die Beträge. ({e})"
        ),
    )
}

/// Die Python-Klasse der Ausnahme, die `_dispatch` an `{"fehler": "Klasse: Text"}` baut.
///
/// Ein `i64`-Ueberlauf kommt hier nie an: [`intervall_fehler`] und [`bescheid_fehler`] fangen ihn
/// vorher als 422 ab ([`ueberlauf_422`]). Was bleibt, hat eine Python-Klasse oder keine
/// (`Dezimal`, `NichtCentGenau`) und gilt als `OverflowError`.
fn slot_klasse(e: &SlotFehler<BescheidFehler>) -> &'static str {
    match e {
        SlotFehler::Slot(b) => b.python_klasse().unwrap_or("OverflowError"),
        SlotFehler::UnbekanntesFeld(_) => "KeyError",
        SlotFehler::SummandNichtGanzzahl(_) => "TypeError",
        SlotFehler::Ueberlauf(_) => "OverflowError",
    }
}

pub(crate) fn intervall_fehler(e: &IntervallFehler<SlotFehler<BescheidFehler>>) -> ApiFehler {
    match e {
        IntervallFehler::LeereAchse(_) => ApiFehler::unerwartet("ValueError", e.to_string()),
        IntervallFehler::Ueberlauf(_) | IntervallFehler::Bescheid(SlotFehler::Ueberlauf(_)) => {
            ueberlauf_422(e)
        }
        IntervallFehler::Bescheid(SlotFehler::Slot(b)) if b.ist_ueberlauf() => ueberlauf_422(e),
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

/// Der Fehler von [`ring`]: Intervall-Rechnung ueber einen Accessor.
pub(crate) type RingFehler = IntervallFehler<SlotFehler<BescheidFehler>>;

/// `IV.intervall(felder, bindung, bescheid_fn, snapshot_id=sid)` fuer eine Quantitaet.
///
/// `bf_felder` ist das `felder`-Argument von `_bescheid_fn` (der Snapshot fuer Einzelfelder), nicht
/// das von `IV.intervall`: `/stand` reicht ihn dem Teil-Ring durch, `GET /fragen` nicht
/// (`api.py:332`).
///
/// `None`, wenn es zu ihr keinen Accessor gibt (`_bescheid_fn` liefert dort `None`).
#[allow(clippy::too_many_arguments)] // Python-Signatur 1:1: `_bescheid_fn` plus `IV.intervall`
pub(crate) fn ring(
    quantitaet: &str,
    vz: Vz,
    umgebung: &Umgebung<'_>,
    achsen: &[AchsenBindung],
    felder: &Felder,
    bf_felder: Option<&Felder>,
    store: Option<&Store>,
    sid: &str,
) -> Result<Option<IntervallErgebnis>, RingFehler> {
    // Estimate-Pfad: ein vorlaeufiger Wert zeigt seine Wirkung im Range (`nur_bestaetigt=False`).
    let Some(bf) = bescheid_fn(
        quantitaet, vz, umgebung, bf_felder, store, false, None, None,
    ) else {
        return Ok(None);
    };
    intervall::intervall(felder, achsen, |w| bf(w), intervall::CAP_DEFAULT, Some(sid)).map(Some)
}

/// Der `intervall`-Teil von [`ring`] als JSON, fuer `/stand`.
fn spanne(
    quantitaet: &str,
    vz: Vz,
    umgebung: &Umgebung<'_>,
    achsen: &[AchsenBindung],
    felder: &Felder,
    store: Option<&Store>,
    sid: &str,
) -> Result<Option<Value>, ApiFehler> {
    let r = ring(
        quantitaet,
        vz,
        umgebung,
        achsen,
        felder,
        Some(felder),
        store,
        sid,
    )
    .map_err(|e| intervall_fehler(&e))?;
    Ok(r.map(|r| intervall_json(&r.intervall)))
}

/// Das Jahr als [`Vz`], oder der Fehler, den Python spaeter in der Rechnung wirft.
///
/// PARITÄT: Python rechnet mit jedem `int` weiter und scheitert erst im Ring (`params/<vz>`);
/// [`Vz`] kennt nur 2024–2026. Ein Fall mit anderem Jahr kann nur von Hand angelegt werden
/// (`POST /fall` prueft `params/`). Gewollte Abweichung, Eintraege 1b (Jahr 10^38, `stand`) und 1h
/// (Jahr 2099, `stand`/`fragen`/`ergebnis`; vollstaendige Akte: beide 500) in
/// `rust/parity/tests/api_http_paritaet.rs`, `dokumentierte_abweichungen`; die gesaettigte Zahl im
/// Text von `deklaration` steht dort als 1i.
pub(crate) fn jahr(store: &Store) -> Result<Vz, ApiFehler> {
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

pub(crate) fn bescheid_fehler(e: &BescheidFehler) -> ApiFehler {
    if e.ist_ueberlauf() {
        return ueberlauf_422(e);
    }
    ApiFehler::unerwartet(e.python_klasse().unwrap_or("OverflowError"), e.to_string())
}
