//! Rueckwaerts, Teil 2: die geordnete Interview-Queue (`naechste_fragen`,
//! `traverser.py:380-733`).
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use bindung::{Bindung, Bindungspunkt, Vorjahr};
use domain::PyWert;
use serde_json::Value;
use store::{Event, Store};

use crate::antwort::{Aktiv, Antwort};
use crate::graph::{Graph, Sicht};
use crate::instanz::instanz_unvollstaendig;
use crate::relevanz::{
    bedingung_je_instanz, gate_gewicht, relevanz_mit, Bedingungsstand, Regelstatus,
};

/// Geordnete Interview-Queue: unbeantwortete askable Felder nicht-ausgeschlossener Regeln
/// (`naechste_fragen`, `traverser.py:395-444`).
///
/// Ordnung: Gates zuerst — nach [`gate_gewicht`] absteigend, bei Gleichstand die
/// `eingangsfrage` vor den Merkmalsfragen (§ 35a, 2026-08-21: "NICHT oeffentlich gefoerdert?"
/// kam vor "hattest du Handwerkerkosten?"), dann alphabetisch. Dann die Slots nach
/// Unsicherheits-`beitrag` (Cent, aus dem Intervall), ohne Beitrag alphabetisch. Danach Themen
/// am Stueck (`_nach_themen`), Vordruck-Ordnung innerhalb des Themas, Themen hinter ihre
/// Voraussetzung.
///
/// Guenstiger-sicher by construction: ALLE offenen askable Felder nicht-ausgeschlossener Regeln
/// stehen in der Queue — kein Zweig faellt wegen eines vorlaeufigen Siegers weg. Ein leerer
/// `beitrag` gilt wie keiner (Python: `if beitrag:`).
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let ohne_beitrag: Option<&std::collections::HashMap<String, i64>> = None;
/// let q = interview::naechste_fragen(&store::Store::leer(2025, None), g.alle(), &g, ohne_beitrag);
/// assert!(q.len() > 100);
/// assert!(q.iter().all(|f| g.alle().get(f).unwrap().askable));
/// ```
#[must_use]
pub fn naechste_fragen<'r, S: std::hash::BuildHasher>(
    store: &Store,
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
    beitrag: Option<&HashMap<String, i64, S>>,
) -> Vec<&'r str> {
    let aktiv = Aktiv::aus(store);
    let rel = relevanz_mit(&aktiv, sicht, graph);
    let kand: Vec<&'r Bindung> = sicht
        .iter()
        .filter(|b| {
            b.askable
                && (aktiv.antwort(&b.feld_id).ist_offen()
                    || instanz_unvollstaendig(&aktiv, sicht, graph, &b.feld_id))
                && !vorjahr_uebernommen(b, aktiv.get(&b.feld_id))
                && rel
                    .get(b.quelle.regel_id.as_str())
                    .is_none_or(|r| r.status != Regelstatus::Ausgeschlossen)
                && !feld_ausgeschlossen(b, &aktiv, sicht, graph)
        })
        .collect();
    let gw = gate_gewicht(sicht, graph);
    let gewicht = |b: &Bindung| gw.get(b.feld_id.as_str()).copied().unwrap_or(0);
    // "Gate" heisst hier: die Antwort streicht andere Fragen. `veranlagung` hat KEINE
    // geltungsbedingung, wirkt aber ueber regel_bedingungen auf 38 Partner-Felder.
    let ist_gate = |b: &Bindung| {
        matches!(b.quelle.bindungspunkt, Bindungspunkt::Geltungsbedingung(_)) || gewicht(b) > 0
    };
    let (mut gates, mut slots): (Vec<&'r Bindung>, Vec<&'r Bindung>) =
        kand.into_iter().partition(|b| ist_gate(b));
    gates.sort_by(|a, b| {
        (Reverse(gewicht(a)), !a.eingangsfrage, a.feld_id.as_str()).cmp(&(
            Reverse(gewicht(b)),
            !b.eingangsfrage,
            b.feld_id.as_str(),
        ))
    });
    let beitrag = beitrag.filter(|m| !m.is_empty());
    match beitrag {
        Some(m) => slots.sort_by_key(|b| {
            (
                Reverse(m.get(&b.feld_id).copied().unwrap_or(0)),
                b.feld_id.as_str(),
            )
        }),
        None => slots.sort_by_key(|b| b.feld_id.as_str()),
    }
    gates.extend(slots);
    let angefangen = angefangene_themen(store, sicht);
    nach_themen(&gates, sicht, graph, &gw, &angefangen, beitrag.is_some())
}

/// `vorjahr: uebernehmbar` mit Vorjahres-Wert faellt aus der Queue, bleibt aber korrigierbar
/// (`_vorjahr_uebernommen`, `traverser.py:380-392`). `vorschlag` bleibt eine Frage.
fn vorjahr_uebernommen(b: &Bindung, ev: Option<&Event>) -> bool {
    b.vorjahr == Some(Vorjahr::Uebernehmbar)
        && ev.is_some_and(|e| e.herkunft.herkunft_achse().as_str() == "vorjahr")
}

/// Faellt DIESES Feld weg, obwohl seine Regel gilt (`_feld_ausgeschlossen`,
/// `traverser.py:523-558`)? Nur bei BESTAETIGT abweichender Antwort jeder Instanz; `wert_nicht`
/// fuer Auswahlfelder (`kist_konfession == "keine"`).
///
/// PARITAET: Python unterscheidet "Schluessel `wert_nicht` vorhanden" von "fehlt"; die Bindung
/// kennt nur `Option`. Ein ausdrueckliches `wert_nicht: null` hiesse in Python "gleich None";
/// in keiner `bindung_*.yaml` belegt (gemessen: 42× `wert: false`, 8× `"zusammen"`, 5×
/// `wert_nicht: "keine"`, 4× `wert_nicht: 0`, 3× `wert: true`).
fn feld_ausgeschlossen(
    b: &Bindung,
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
) -> bool {
    let Some(bed) = &b.feld_bedingung else {
        return false;
    };
    let stand = if let Some(nicht) = &bed.wert_nicht {
        let nicht = PyWert::from(nicht.clone());
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| w.py_eq(&nicht))
    } else {
        let soll = PyWert::from(bed.wert.clone().unwrap_or(Value::Null));
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| !w.py_eq(&soll))
    };
    stand == Bedingungsstand::Ausgeschlossen
}

/// Themen mit mindestens einer BESTAETIGTEN Antwort, das zuletzt angefasste zuerst
/// (`_angefangene_themen`, `traverser.py:447-464`). Liest ALLE Events (auch ersetzte) — die
/// Log-Reihenfolge ist die Antwort-Reihenfolge.
fn angefangene_themen<'r>(store: &Store, sicht: &Sicht<'r>) -> Vec<&'r str> {
    let mut zuletzt: HashMap<&'r str, usize> = HashMap::new();
    for (i, e) in store.events().iter().enumerate() {
        if Antwort::aus(Some(e)).ist_offen() {
            continue;
        }
        if let Some(b) = sicht.get(&e.feld_id) {
            if !b.quelle.regel_id.is_empty() {
                zuletzt.insert(b.quelle.regel_id.as_str(), i);
            }
        }
    }
    let mut themen: Vec<(&'r str, usize)> = zuletzt.into_iter().collect();
    themen.sort_by_key(|&(_, i)| Reverse(i));
    themen.into_iter().map(|(t, _)| t).collect()
}

/// Themen am Stueck, ohne die Rangfolge umzuwerfen (`_nach_themen`, `traverser.py:467-520`):
/// ein Thema steht dort, wo sein BESTES Feld stand; innerhalb gilt die bisherige Ordnung, die
/// Eingangsfrage zuerst. Gemessen vorher: 175 Themenwechsel bei 62 Themen.
fn nach_themen<'r>(
    felder: &[&'r Bindung],
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
    gw: &HashMap<&'r str, usize>,
    angefangen: &[&'r str],
    gewicht_aktiv: bool,
) -> Vec<&'r str> {
    let mut themen: Vec<(&'r str, Vec<&'r Bindung>)> = Vec::new();
    let mut pos: HashMap<&'r str, usize> = HashMap::new();
    for &b in felder {
        let t = b.quelle.regel_id.as_str();
        let i = *pos.entry(t).or_insert_with(|| {
            themen.push((t, Vec::new()));
            themen.len() - 1
        });
        if let Some((_, gruppe)) = themen.get_mut(i) {
            gruppe.push(b);
        }
    }
    for (_, gruppe) in &mut themen {
        let (eingang, rest): (Vec<&Bindung>, Vec<&Bindung>) =
            gruppe.iter().partition(|b| b.eingangsfrage);
        let geordnet: Vec<&Bindung> = eingang.into_iter().chain(rest).collect();
        *gruppe = nach_vordruck(&geordnet, gw, gewicht_aktiv);
    }
    let folge = themen_folge(&themen, sicht, graph, angefangen);
    let mut out = Vec::with_capacity(felder.len());
    for t in folge {
        if let Some((_, gruppe)) = pos.get(t).and_then(|&i| themen.get(i)) {
            out.extend(gruppe.iter().map(|b| b.feld_id.as_str()));
        }
    }
    // Guenstiger-sicher: jedes Feld genau einmal, kein Thema verloren oder doppelt gesetzt.
    debug_assert_eq!(
        out.len(),
        felder.len(),
        "Themenfolge verliert oder verdoppelt Felder"
    );
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Klasse {
    Formal,
    Wert,
}

/// Innerhalb eines Themas: Felder mit ELSTER-Kennzahl in Vordruck-Reihenfolge, nur innerhalb
/// ihrer Klasse (`_nach_vordruck`, `traverser.py:561-623`). Eingangsfragen und echte Gates
/// (Gewicht > 0) bleiben stehen; "formal" = traegt eine Geltungsbedingung, streicht nichts.
/// Mit Unsicherheits-Beitrag bleibt die Klasse "wert" in Beitrags-Ordnung.
fn nach_vordruck<'r>(
    gruppe: &[&'r Bindung],
    gw: &HashMap<&'r str, usize>,
    gewicht_aktiv: bool,
) -> Vec<&'r Bindung> {
    let klasse = |b: &Bindung| -> Option<Klasse> {
        if b.eingangsfrage || gw.get(b.feld_id.as_str()).copied().unwrap_or(0) > 0 {
            return None;
        }
        Some(match b.quelle.bindungspunkt {
            Bindungspunkt::Geltungsbedingung(_) => Klasse::Formal,
            Bindungspunkt::SignaturSlot(_) => Klasse::Wert,
        })
    };
    let mut aus = gruppe.to_vec();
    for k in [Klasse::Formal, Klasse::Wert] {
        if k == Klasse::Wert && gewicht_aktiv {
            continue;
        }
        let mit: Vec<&Bindung> = gruppe
            .iter()
            .copied()
            .filter(|b| {
                b.elster_kz.as_deref().is_some_and(|kz| !kz.is_empty()) && klasse(b) == Some(k)
            })
            .collect();
        if mit.len() < 2 {
            continue;
        }
        let kandidaten: HashSet<&str> = mit.iter().map(|b| b.feld_id.as_str()).collect();
        let mut sortiert = mit.clone();
        sortiert.sort_by(|a, b| a.elster_kz.cmp(&b.elster_kz));
        let mut naechster = sortiert.into_iter();
        for platz in &mut aus {
            if kandidaten.contains(platz.feld_id.as_str()) {
                if let Some(b) = naechster.next() {
                    *platz = b;
                }
            }
        }
    }
    aus
}

/// Ein Thema folgt ALLEN Themen, die seine Voraussetzung erheben (`_themen_folge`,
/// `traverser.py:626-733`): `regel_bedingungen`-Feld, `ableitung.aus`, Zaehlfeld der
/// Instanz-Gruppe. Einstieg fest (`themen_zuerst`), dann angefangene Themen, dann der Rest.
/// KEINE Topologie-Sortierung: runden-weise, bei einem Ring bleibt der Rest in
/// Wunsch-Reihenfolge — nie Endlosschleife, nie ein verlorenes Thema.
fn themen_folge<'r>(
    themen: &[(&'r str, Vec<&'r Bindung>)],
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
    angefangen: &[&'r str],
) -> Vec<&'r str> {
    let vorhanden: HashSet<&str> = themen.iter().map(|(t, _)| *t).collect();
    let mut quelle: HashMap<&'r str, HashSet<&'r str>> = HashMap::new();
    for (thema, felder) in themen {
        let mut merke = |feld: &str| {
            if let Some(qt) = sicht.get(feld).map(|b| b.quelle.regel_id.as_str()) {
                if !qt.is_empty() && qt != *thema && vorhanden.contains(qt) {
                    quelle.entry(thema).or_default().insert(qt);
                }
            }
        };
        for c in graph.regel_bedingungen(thema) {
            merke(&c.feld);
        }
        for b in felder {
            if let Some(abl) = &b.ableitung {
                merke(&abl.aus);
            }
            let zaehl = b
                .instanz_gruppe
                .as_deref()
                .filter(|g| !g.is_empty())
                .and_then(|g| graph.instanz_gruppe(g))
                .map(|g| g.anzahl_feld.as_str())
                .filter(|z| !z.is_empty());
            if let Some(z) = zaehl {
                if !felder.iter().any(|f| f.feld_id == z) {
                    merke(z);
                }
            }
        }
    }
    let vorne: Vec<&'r str> = graph
        .themen_zuerst()
        .iter()
        .copied()
        .filter(|t| vorhanden.contains(t))
        .collect();
    let laufend: Vec<&'r str> = angefangen
        .iter()
        .copied()
        .filter(|t| vorhanden.contains(t) && !vorne.contains(t))
        .collect();
    let wunsch: Vec<&'r str> = vorne
        .iter()
        .chain(&laufend)
        .copied()
        .chain(
            themen
                .iter()
                .map(|(t, _)| *t)
                .filter(|t| !vorne.contains(t) && !laufend.contains(t)),
        )
        .collect();
    let mut folge: Vec<&'r str> = Vec::new();
    let mut gesetzt_menge: HashSet<&'r str> = HashSet::new();
    let mut offen = wunsch.clone();
    for _runde in 0..=wunsch.len() {
        if offen.is_empty() {
            break;
        }
        let gesetzt: Vec<&'r str> = offen
            .iter()
            .copied()
            .filter(|t| quelle.get(t).is_none_or(|q| q.is_subset(&gesetzt_menge)))
            .collect();
        if gesetzt.is_empty() {
            break;
        }
        gesetzt_menge.extend(gesetzt.iter().copied());
        folge.extend(gesetzt.iter().copied());
        offen.retain(|t| !gesetzt.contains(t));
    }
    folge.extend(offen);
    folge
}
