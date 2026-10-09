//! `api.preflight_check` (`api.py:641`): die Konsistenz-Pruefungen und die vergessenen Pauschalen
//! als Liste, die die Oberflaeche zeigt. Kein LLM, kein Schreibpfad.
//!
//! Abweichung Nr. 47: dazu ein Hinweis `pflichtfelder`, wenn `elster::pflichtfelder_luecken` (dieselbe Liste wie
//! `/deklaration`) etwas nennt, das die Scheibe des Falls fragt. Python hat die Liste nicht.
use std::collections::HashSet;

use domain::{FallId, PyWert};
use interview::Graph;
use konsistenz::Ampel;
use serde_json::{json, Value};
use store::Store;

use crate::antwort::Antwort;
use crate::fehler::ApiFehler;
use crate::zustand::Zustand;

/// `(store.get("vorjahr_referenz") or {}).get("verlustvortrag_bestand")` und davon `"wert"`, nur
/// als Ganzzahl (`preflight.py:349-353`).
///
/// ponytail: Python nimmt auch einen `float`, Rust nur eine Ganzzahl bis `i64`, und ein
/// `vorjahr_referenz`, das kein Objekt ist, zaehlt als fehlend (Python wirft `AttributeError`). Die
/// Referenz schreibt nur `api.vorjahr` als Objekt mit Ganzzahl-Betraegen; ein groesserer Bestand
/// als `i64` (92 Billiarden Euro) kommt nicht vor. Upgrade: Gleitkomma-Zweig in
/// `konsistenz::plausibilitaets_widersprueche`.
fn vorjahr_verlustvortrag(store: &Store) -> Option<i64> {
    let feld = |obj: &PyWert, name: &str| match obj {
        PyWert::Objekt(paare) => paare
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone()),
        _ => None,
    };
    let referenz = store.datei().vorjahr_referenz.as_ref()?;
    match feld(&feld(referenz, "verlustvortrag_bestand")?, "wert")? {
        PyWert::Ganz(n) => Some(n),
        _ => None,
    }
}

/// Das Kreuz der Rentner-Scheibe (Abweichung Nr. 46): "nein" heisst weder Lohn noch Pension.
const KREUZ_KEIN_LOHN: &str = "kein_lohn_pension";

/// Die Pflichtfelder der Lohn-Gruppe (`elster::PFLICHTFELDER`), die das Kreuz bei "nein" aus der Fragenliste verbirgt. Ein
/// Test (`preflight_pflichtfelder.rs`) haelt fest, dass sie dort wirklich fehlen.
const HINTER_DEM_KREUZ: [&str; 3] = ["bruttoarbeitslohn", "steuerklasse", "p36_lohnsteuer"];

/// Der Fragetext bis einschliesslich des ersten "?" (wie `konsistenz::preflight`, dort privat). Fehlt er, steht die
/// Kennung da; die Texte der Pflichtfelder pinnt `preflight_pflichtfelder.rs`.
fn frage_kurz(feld_id: &str, graph: &Graph<'_>) -> String {
    let text = graph
        .alle()
        .get(feld_id)
        .and_then(|b| b.fragetext_laie.as_deref())
        .unwrap_or(feld_id);
    match text.split_once('?') {
        Some((kopf, _)) => format!("{kopf}?"),
        None => text.to_owned(),
    }
}

/// "a", "a und b", "a, b und c".
fn aufzaehlung(teile: &[String]) -> String {
    match teile.split_last() {
        None => String::new(),
        Some((letztes, [])) => letztes.clone(),
        Some((letztes, davor)) => format!("{} und {letztes}", davor.join(", ")),
    }
}

/// Der Satz zu den offenen Pflichtfeldern, oder `None`, wenn die Scheibe nichts offen fragt.
///
/// Genannt wird nur, was die Scheibe des Falls fragt: Die Stammdaten stehen in `ep` nicht, der Nutzer koennte sie dort
/// nicht beantworten. Hat er das Kreuz [`KREUZ_KEIN_LOHN`] auf "nein" gesetzt und liegt in der Lohn-Gruppe trotzdem ein
/// Wert, sind Lohn und Steuerklasse verborgen: Dann steht die Eingangsfrage im Satz, nie eine Frage, die er nicht mehr
/// sieht.
///
/// ponytail: `pflichtfelder_luecken` kennt 7 Stammdaten und zwei "alle oder keins"-Gruppen (Lohn, Rentenversicherungs-
/// anteile), und nur von Person A; `ERiC` verlangt mehr. Ein fehlender Satz heisst deshalb NICHT "abgabefaehig". Upgrade:
/// die Liste in `elster::PFLICHTFELDER` erweitern, dieser Satz folgt ihr von selbst.
fn pflichtfelder_satz(
    felder: &konsistenz::Felder,
    scheibe: &HashSet<String>,
    graph: &Graph<'_>,
) -> Option<String> {
    let kreuz_nein =
        konsistenz::lies(felder, KREUZ_KEIN_LOHN).bestaetigt() == Some(&PyWert::Bool(true));
    let mut fragen: Vec<String> = Vec::new();
    let mut verborgen = false;
    for luecke in elster::pflichtfelder_luecken(felder) {
        let id = luecke.feld_id.as_str();
        if !scheibe.contains(id) {
            continue;
        }
        if kreuz_nein && HINTER_DEM_KREUZ.contains(&id) {
            verborgen = true;
        } else {
            fragen.push(format!("»{}«", frage_kurz(id, graph)));
        }
    }
    let mut saetze: Vec<String> = Vec::new();
    if !fragen.is_empty() {
        saetze.push(format!(
            "Für die Abgabe fehlen noch Angaben. Bitte beantworte: {}.",
            aufzaehlung(&fragen)
        ));
    }
    if verborgen {
        saetze.push(format!(
            "Bei der Frage »{}« hast du „nein“ angekreuzt, aber Angaben zu Lohn oder Lohnsteuer \
             gespeichert. Bitte prüfe diese Antwort.",
            frage_kurz(KREUZ_KEIN_LOHN, graph)
        ));
    }
    (!saetze.is_empty()).then(|| saetze.join(" "))
}

/// `api.preflight_check(fall_id)` nach dem Owner-Check.
///
/// # Errors
/// 400/500 aus Scheibe und Bindung; 500 `ValueError`, wenn der Snapshot nicht entsteht.
pub fn preflight(z: &Zustand, fall_id: &FallId, store: &Store) -> Result<Antwort, ApiFehler> {
    let sb = z.scheibe_bindung(store)?;
    let (felder, _) = store
        .materialisiere(None)
        .map_err(|e| ApiFehler::unerwartet("ValueError", e.to_string()))?;
    let scheibe: HashSet<String> = sb.index.keys().cloned().collect();
    // Erschoepfend, ohne `..`: eine achte Liste in `PreflightErgebnis` bricht hier den Bau, statt still
    // nicht ausgeliefert zu werden (Ampel rot ohne Grund, `preflight.py` liefert alle Listen).
    let konsistenz::PreflightErgebnis {
        widersprueche_flag,
        widersprueche_partner,
        widersprueche_alleinerziehend,
        widersprueche_plausibilitaet,
        hinweise_pauschalen,
        hinweise_nicht_gerechnet,
        hinweise_betrag_vorlaeufig,
        status,
    } = konsistenz::preflight(
        &felder,
        Some(&scheibe),
        vorjahr_verlustvortrag(store),
        &sb.graph,
    );
    // Nur ausliefern, was etwas zu sagen hat; die Reihenfolge der sieben Listen ist die von Python.
    // `nicht_gerechnet` ist ein eigener Bereich und laeuft NICHT unter `pauschale` mit.
    let mut items: Vec<Value> = Vec::new();
    let mut nimm = |typ: &str, bereich: &str, texte: Vec<&str>| {
        items.extend(
            texte
                .into_iter()
                .map(|text| json!({"typ": typ, "bereich": bereich, "text": text})),
        );
    };
    nimm(
        "widerspruch",
        "flag",
        widersprueche_flag
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "widerspruch",
        "partner",
        widersprueche_partner
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "widerspruch",
        "alleinerziehend",
        widersprueche_alleinerziehend
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "widerspruch",
        "plausibilitaet",
        widersprueche_plausibilitaet
            .iter()
            .map(|w| w.grund.as_str())
            .collect(),
    );
    nimm(
        "hinweis",
        "pauschale",
        hinweise_pauschalen.iter().map(|h| h.hinweis).collect(),
    );
    nimm(
        "hinweis",
        "nicht_gerechnet",
        hinweise_nicht_gerechnet.iter().map(|h| h.hinweis).collect(),
    );
    nimm(
        "hinweis",
        "betrag_vorlaeufig",
        hinweise_betrag_vorlaeufig
            .iter()
            .map(|h| h.hinweis.as_str())
            .collect(),
    );
    // Hinten, hinter den sieben Listen von Python. Die Ampel steigt hoechstens von GREEN auf AMBER: eine offene
    // Angabe ist kein Widerspruch, und RED bleibt RED.
    let satz = pflichtfelder_satz(&felder, &scheibe, &sb.graph);
    if let Some(satz) = &satz {
        nimm("hinweis", "pflichtfelder", vec![satz.as_str()]);
    }
    let status = if satz.is_some() && status == Ampel::Gruen {
        Ampel::Gelb
    } else {
        status
    };
    Ok(Antwort::neu(
        200,
        json!({"fall_id": fall_id.as_str(), "status": status.als_str(), "items": items}),
    ))
}
