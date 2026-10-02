//! `GET /fall/{id}/ergebnis` — `api.ergebnis` (`api.py:558`) um `_ergebnis_roh` (`api.py:569`): die
//! festgesetzte Steuer, oder der benannte Grund, warum es keine gibt.
//!
//! Fail-closed wie in Python: eine Zahl nur, wenn der Guard nicht sperrt, der Kegel vollstaendig
//! bestaetigt ist und kein Ring-Betrag aussteht. Die Reihenfolge der Schritte ist die von Python;
//! ein Fehler an einer frueheren Stelle verdeckt jeden spaeteren.
use bescheid::deklaration::{
    an_gesamt_sperrgrund, feste_zahl, sperrgrund_felder, sperrgrund_klartext_text,
    vorlaeufige_ring_betraege,
};
use bescheid::zweige::{abschlusszahlung_cent, bescheid_fn, Kette, P31Sieger, Umgebung};
use bescheid::{Felder, Instanzquelle};
use domain::{Cent, FallId, PyWert, Sperrgrund, Vz, Zustand as Feldzustand};
use intervall::AchsenBindung;
use serde_json::{json, Map, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::flow;
use crate::stand::{bescheid_fehler, jahr};
use crate::zustand::{ScheibenBindung, Zustand};

/// Die Antwort ohne Zahl: alle Betraege `null`, der Grund und die offenen Felder.
fn ohne_zahl(fall_id: &FallId, sid: &str, grund: &str, offen: &[&str]) -> Map<String, Value> {
    let mut m = Map::new();
    for (schluessel, wert) in [
        ("fall_id", json!(fall_id.as_str())),
        ("snapshot_id", json!(sid)),
        ("zahl_cent", Value::Null),
        ("solz_cent", Value::Null),
        ("kist_cent", Value::Null),
        ("mobilitaetspraemie_cent", Value::Null),
        ("abschlusszahlung_cent", Value::Null),
        ("grund", json!(grund)),
        ("offen", json!(offen)),
        ("trace", Value::Null),
    ] {
        m.insert(schluessel.into(), wert);
    }
    m
}

/// Die Rechenweg-Kette (`catala_gesamt_kette`, alle Werte EURO), im § 31-Fall mit `p31`.
fn kette_json(k: &Kette) -> Value {
    let mut m = Map::new();
    m.insert(
        "gesamtbetrag_der_einkuenfte".into(),
        json!(k.gesamtbetrag_der_einkuenfte.get()),
    );
    m.insert(
        "zu_versteuerndes_einkommen".into(),
        json!(k.zu_versteuerndes_einkommen.get()),
    );
    m.insert("tarifliche_est".into(), json!(k.tarifliche_est.get()));
    m.insert(
        "festzusetzende_est".into(),
        json!(k.festzusetzende_est.get()),
    );
    if let Some(p) = &k.p31 {
        let sieger = match p.guenstiger {
            P31Sieger::Freibetraege => "freibetraege",
            P31Sieger::Kindergeld => "kindergeld",
        };
        m.insert(
            "p31".into(),
            json!({"guenstiger": sieger, "kindergeld": p.kindergeld.get(), "text": p.text}),
        );
    }
    Value::Object(m)
}

/// Die Huelle `api.ergebnis`: bei einem Grund ausser `bestaetigt` kommt der Klartext dazu, und der
/// Ausgang geht in den Fluss (`flow.ergebnis_notiert`).
fn melde(z: &Zustand, fall_id: &FallId, mut obj: Map<String, Value>) -> Antwort {
    let grund = obj
        .get("grund")
        .and_then(Value::as_str)
        .filter(|g| *g != "bestaetigt")
        .map(str::to_owned);
    if let Some(g) = grund {
        obj.insert("klartext".into(), json!(sperrgrund_klartext_text(Some(&g))));
    }
    // `ergebnis_notiert` liest nur diese drei Schluessel.
    let fluss = PyWert::Objekt(
        ["grund", "zahl_cent", "offen"]
            .into_iter()
            .map(|k| {
                let v = obj.get(k).cloned().map_or(PyWert::Null, PyWert::from);
                (k.to_owned(), v)
            })
            .collect(),
    );
    flow::ergebnis_notiert(&z.konfig.audit_dir, fall_id.as_str(), &fluss);
    Antwort::neu(200, Value::Object(obj))
}

/// Die Basis-Felder der Instanzen von `gwg`, `kind` und `p23_veraeusserung`, die nicht bestaetigt
/// sind, dazu das offen gelassene Gate von § 10 Abs. 1 Nr. 5, sortiert (`api.py:628`). Die Zahl
/// bleibt die gefilterte; das hier ist der Hinweis darauf, was sie nicht kennt.
fn offene_instanzfelder(
    store: &Store,
    index: &bescheid::BindungIndex<'_>,
) -> Result<Vec<String>, ApiFehler> {
    let q = Instanzquelle {
        store: Some(store),
        bindung: Some(index),
        nur_bestaetigt: false,
    };
    let mut offen = std::collections::BTreeSet::new();
    for gruppe in ["gwg", "kind", "p23_veraeusserung"] {
        for inst in q.instanzen(gruppe).map_err(|e| bescheid_fehler(&e))? {
            offen.extend(
                inst.felder
                    .iter()
                    .filter(|(_, fw)| fw.zustand != Feldzustand::Bestaetigt)
                    .map(|(fid, _)| fid.clone()),
            );
        }
    }
    offen.extend(
        bescheid::abzuege::p10_1_5_gate_fehlend(&q)
            .map_err(|e| bescheid_fehler(&e))?
            .into_iter()
            .map(str::to_owned),
    );
    Ok(offen.into_iter().collect())
}

/// Die Antwort bei einem Sperrgrund des Guards: keine Zahl, dazu die Angaben, die ihn ausloesen
/// (`sperr_felder`, leer ausser beim Partner-Widerspruch).
fn gesperrt(
    fall_id: &FallId,
    sid: &str,
    grund: Sperrgrund,
    felder: &Felder,
) -> Result<Map<String, Value>, ApiFehler> {
    let zu = |v: &PyWert| {
        v.zu_json()
            .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))
    };
    let mut felder_json = Vec::new();
    for w in sperrgrund_felder(Some(grund), felder) {
        felder_json.push(json!({
            "feld_id": w.feld_id, "wert": zu(&w.wert)?,
            "veranlagung": zu(&w.veranlagung)?, "grund": w.grund,
        }));
    }
    let mut obj = ohne_zahl(fall_id, sid, grund.als_str(), &[]);
    obj.insert("sperr_felder".into(), Value::Array(felder_json));
    Ok(obj)
}

/// Der Grund, warum `feste_zahl` keine Zahl liefert, und die offenen Felder dazu.
///
/// Python reicht dort nur `None` weiter und baut den Grund neu (`api.py:595-616`): dieselbe
/// Relevanz-Sicht wie `_feste_zahl` (ein abbestelltes Feld ist nicht "offen"), die Liste der
/// vorlaeufigen Betraege geht der des Kegels vor, und `engine_unavailable` gilt nur bei vollem Kegel.
fn grund_ohne_zahl<'k>(
    sb: &ScheibenBindung<'_>,
    store: &Store,
    felder: &Felder,
    quantitaet: &str,
    vz: Vz,
    umgebung: &Umgebung<'_>,
    scheibe_felder: &[&'k str],
) -> (&'static str, Vec<&'k str>) {
    let mut offen: Vec<&str> =
        interview::relevante_kegel_felder(scheibe_felder, &sb.sicht, Some(store), &sb.graph)
            .into_iter()
            .filter(|f| {
                felder
                    .get(*f)
                    .is_none_or(|v| v.zustand != Feldzustand::Bestaetigt)
            })
            .collect();
    let mut betrag_offen = vorlaeufige_ring_betraege(felder, &sb.cfg, &sb.index);
    betrag_offen.sort_unstable();
    if !betrag_offen.is_empty() {
        offen.clone_from(&betrag_offen);
    }
    let ohne_accessor = bescheid_fn(
        quantitaet,
        vz,
        umgebung,
        Some(felder),
        None,
        true,
        None,
        None,
    )
    .is_none();
    let grund = if !betrag_offen.is_empty() {
        "ring_betrag_vorlaeufig"
    } else if ohne_accessor && offen.is_empty() {
        "engine_unavailable"
    } else {
        "input_kegel_nicht_bestaetigt"
    };
    offen.sort_unstable();
    (grund, offen)
}

/// `api.ergebnis(fall_id)` nach dem Owner-Check.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 500 mit der Python-Klasse, wenn Guard, Ring oder Trace
/// scheitern.
pub fn ergebnis(z: &Zustand, fall_id: &FallId, store: &Store) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let (felder, sid) = store
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let sid = sid.to_string();
    // Pflicht-Kegel: die Einzel-Basis (`cfg["kegel"]`); Partner-Pflichtfelder gehoeren nur bei
    // `zusammen` dazu, und die prueft der Guard.
    let scheibe_felder: Vec<&str> = match sb.cfg.kegel_roh() {
        Some(k) if !k.is_empty() => k.to_vec(),
        _ => sb.felder.iter().map(String::as_str).collect(),
    };
    let vz_opt: Option<Vz> = u16::try_from(store.veranlagungszeitraum())
        .ok()
        .and_then(|j| Vz::try_from(j).ok());
    if sb.cfg.guard() {
        // K2: ein nicht-ringfaehiger Abzug oder eine Einkunftsart sperrt den Ring VOR jeder Zahl.
        let q = Instanzquelle {
            store: Some(store),
            bindung: Some(&sb.index),
            nur_bestaetigt: false,
        };
        let sperr = an_gesamt_sperrgrund(&felder, Some(&sb.cfg), vz_opt, &q)
            .map_err(|e| bescheid_fehler(&e))?;
        if let Some(g) = sperr {
            return Ok(melde(z, fall_id, gesperrt(fall_id, &sid, g, &felder)?));
        }
    }
    // Multi-Regel-Scheibe ohne ehrlichen Gesamt-Accessor: bewusst KEINE Scheiben-Zahl.
    let Some(quantitaet) = sb.cfg.gesamt_ring() else {
        return Ok(melde(
            z,
            fall_id,
            ohne_zahl(fall_id, &sid, "kein_scheiben_gesamtbescheid", &[]),
        ));
    };
    let vz = jahr(store)?;
    let params = z.params()?;
    let achsen: Vec<AchsenBindung> = sb.sicht.iter().map(AchsenBindung::from).collect();
    let umgebung = Umgebung {
        achsen: &achsen,
        index: &sb.index,
        params,
    };
    let fest = feste_zahl(
        &felder,
        &sb.cfg,
        vz,
        &scheibe_felder,
        &umgebung,
        Some(store),
        Some(&sb.graph),
    )
    .map_err(|e| bescheid_fehler(&e))?;
    let Ok(fest) = fest else {
        let (grund, offen) = grund_ohne_zahl(
            &sb,
            store,
            &felder,
            quantitaet,
            vz,
            &umgebung,
            &scheibe_felder,
        );
        return Ok(melde(z, fall_id, ohne_zahl(fall_id, &sid, grund, &offen)));
    };
    let trace = interview::trace_ergebnis(store, &sb.sicht, Some(&sid));
    let trace = serde_json::to_value(&trace)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let offen = offene_instanzfelder(store, &sb.index)?;
    let abschluss = abschlusszahlung_cent(&felder, fest.zahl).map_err(|e| bescheid_fehler(&e))?;
    let cent = |c: Option<Cent>| json!(c.map(Cent::get));
    let mut obj = ohne_zahl(fall_id, &sid, "bestaetigt", &[]);
    for (schluessel, wert) in [
        ("zahl_cent", json!(fest.zahl.get())),
        ("solz_cent", cent(fest.solz_cent)),
        ("kist_cent", cent(fest.extras.kist_cent)),
        (
            "mobilitaetspraemie_cent",
            cent(fest.extras.mobilitaetspraemie_cent),
        ),
        ("abschlusszahlung_cent", cent(abschluss)),
        ("offen", json!(offen)),
        ("trace", trace),
        (
            "kette",
            fest.extras.kette.as_ref().map_or(Value::Null, kette_json),
        ),
    ] {
        obj.insert(schluessel.into(), wert);
    }
    Ok(melde(z, fall_id, obj))
}
