//! Rueckwaerts, Teil 2: die geordnete Interview-Queue (`naechste_fragen`,
//! `traverser.py:380-733`).
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

use bindung::{Bindung, Bindungspunkt, Vorjahr};
use domain::PyWert;
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
    let vz = store.datei().veranlagungszeitraum.als_i64_saettigend();
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
                && !feld_ausgeschlossen(b, &aktiv, sicht, graph, vz)
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
///
/// Rust-eigen (Abweichung Nr. 32): `alter_im_vz` meint ein Geburtsjahr in `feld`. Das Feld bleibt nur im Jahr, in dem
/// Veranlagungsjahr `vz` minus Geburtsjahr genau diese Zahl ergibt; jedes BESTAETIGTE andere Geburtsjahr schliesst es aus,
/// ein fehlendes oder vorlaeufiges nicht (fail-closed wie `wert`). Ein bestaetigter Wert, der kein Ganzzahl-Geburtsjahr ist,
/// schliesst ebenfalls nicht aus: die Frage bleibt.
fn feld_ausgeschlossen(
    b: &Bindung,
    aktiv: &Aktiv<'_>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
    vz: i64,
) -> bool {
    let Some(bed) = &b.feld_bedingung else {
        return false;
    };
    let stand = if let Some(alter) = bed.alter_im_vz {
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| {
            matches!(w, PyWert::Ganz(gj) if vz.checked_sub(*gj) != Some(alter))
        })
    } else if let Some(nicht) = &bed.wert_nicht {
        let nicht = PyWert::from(nicht.clone());
        bedingung_je_instanz(aktiv, sicht, graph, &bed.feld, |w| w.py_eq(&nicht))
    } else {
        let soll = bed.wert.clone().map_or(PyWert::Null, PyWert::from);
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
        *gruppe = nach_ausloesern(nach_vordruck(&geordnet, gw, gewicht_aktiv));
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

/// Ein Feld mit `ableitung` steht hinter seinen Ausloesern (`aus`, `und_feld`), auch im selben
/// Thema (`_nach_ausloesern`, `traverser.py`; Vault: `decisions/ableitung-feuert-je-instanz-und-
/// frage-nach-beiden-ausloesern`, Punkt 2). `themen_folge` ordnet nur THEMEN; innerhalb eines
/// Themas stand das Ziel `kind_unter_14_haushaltszugehoerig` (Gate) vor seinem `und_feld` (Slot),
/// und wer der Queue folgte, beantwortete es, bevor die Ableitung feuern konnte.
///
/// Das Ziel bleibt eine Frage; nur seine STELLE aendert sich: direkt hinter den letzten Ausloeser,
/// der noch weiter hinten steht. Mehrere Nachzuegler hinter demselben Ausloeser behalten ihre
/// Reihenfolge, die Menge bleibt gleich. Ein Ausloeser mit eigener `ableitung` zaehlt nicht (keine
/// Ketten; er koennte selbst nachruecken und seinen Nachzuegler verlieren), ebenso wenig einer, der
/// nicht in der Gruppe steht (anderes Thema, schon beantwortet).
// ponytail: nur Ausloeser derselben Gruppe. Liegt ein `und_feld` in einem anderen Thema, ordnet
// `themen_folge` es nicht vor (dort gilt nur `aus`); heute gibt es keinen solchen Fall.
fn nach_ausloesern(gruppe: Vec<&Bindung>) -> Vec<&Bindung> {
    let ort: HashMap<&str, usize> = gruppe
        .iter()
        .enumerate()
        .map(|(i, b)| (b.feld_id.as_str(), i))
        .collect();
    // Index des Ausloesers -> Indizes seiner Nachzuegler, in Gruppenordnung.
    let mut hinter: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, b) in gruppe.iter().enumerate() {
        let Some(regel) = &b.ableitung else {
            continue;
        };
        let weiter_hinten = [Some(regel.aus.as_str()), regel.und_feld.as_deref()]
            .into_iter()
            .flatten()
            .filter_map(|x| ort.get(x).copied())
            .filter(|&j| j > i && gruppe.get(j).is_some_and(|a| a.ableitung.is_none()))
            .max();
        if let Some(j) = weiter_hinten {
            hinter.entry(j).or_default().push(i);
        }
    }
    if hinter.is_empty() {
        return gruppe;
    }
    let nachgezogen: HashSet<usize> = hinter.values().flatten().copied().collect();
    let mut aus = Vec::with_capacity(gruppe.len());
    for (i, &b) in gruppe.iter().enumerate() {
        if nachgezogen.contains(&i) {
            continue;
        }
        aus.push(b);
        for k in hinter.get(&i).into_iter().flatten() {
            aus.extend(gruppe.get(*k).copied());
        }
    }
    aus
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
            .filter(|b| b.elster_kz.is_some() && klasse(b) == Some(k))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Eine Bindung der echten Registry, umbenannt; `ableitung` je nach `regel` gesetzt oder leer.
    fn feld(id: &str, regel: Option<(&str, Option<&str>)>) -> Bindung {
        let reg = crate::doctest_registry().expect("registry");
        let vorlage = reg
            .dateien
            .iter()
            .flat_map(|(_, d)| &d.bindungen)
            .find(|b| b.feld_id == "kind_unter_14_haushaltszugehoerig")
            .expect("Vorlage")
            .clone();
        let mut b = vorlage.clone();
        b.feld_id = id.to_owned();
        b.ableitung = regel.map(|(aus, und)| {
            let mut a = vorlage
                .ableitung
                .clone()
                .expect("Vorlage traegt eine ableitung");
            a.aus = aus.to_owned();
            a.und_feld = und.map(str::to_owned);
            a
        });
        b
    }

    fn ids(v: &[&Bindung]) -> Vec<String> {
        v.iter().map(|b| b.feld_id.clone()).collect()
    }

    /// Gegenstueck zu `test_nachzuegler_behalten_ihre_reihenfolge_und_gehen_nicht_verloren`.
    #[test]
    fn nachzuegler_behalten_ihre_reihenfolge_und_gehen_nicht_verloren() {
        let regel = Some(("q", Some("u")));
        let (z1, z2) = (feld("z1", regel), feld("z2", regel));
        let (v, q, u, w) = (
            feld("v", None),
            feld("q", None),
            feld("u", None),
            feld("w", None),
        );
        let ordnung = |gruppe: Vec<&Bindung>| ids(&nach_ausloesern(gruppe));
        assert_eq!(
            ordnung(vec![&z1, &z2, &v, &q, &u, &w]),
            ["v", "q", "u", "z1", "z2", "w"]
        );
        // Steht schon alles richtig, aendert sich nichts.
        assert_eq!(
            ordnung(vec![&q, &u, &z1, &z2, &v, &w]),
            ["q", "u", "z1", "z2", "v", "w"]
        );
        // Fehlt ein Ausloeser in der Gruppe, bleibt das Ziel stehen oder folgt dem anderen.
        assert_eq!(ordnung(vec![&z1, &v, &q]), ["v", "q", "z1"]);
        assert_eq!(ordnung(vec![&z1, &v, &w]), ["z1", "v", "w"]);
    }

    /// Ein Ausloeser mit eigener `ableitung` zaehlt nicht: keine Kette, nichts geht verloren.
    #[test]
    fn ausloeser_mit_eigener_ableitung_zaehlt_nicht() {
        let (a, b) = (feld("a", Some(("b", None))), feld("b", Some(("a", None))));
        assert_eq!(ids(&nach_ausloesern(vec![&a, &b])), ["a", "b"]);
    }

    /// Ein Ereignis fuer `feld_id` mit dem rohen Wert `wert`, am Store vorbei geschrieben (die Bindung prueft hier nichts).
    fn ereignis(feld_id: &str, wert: PyWert, bestaetigt: bool) -> store::Event {
        use domain::{Achsenwert, Herkunft, PruefTiefe, Schreiber, Zustand};
        let mut e = store::Event {
            event_id: store::EventId::aus_bytes([0; 32]),
            ts: "2026-10-07T10:00:00Z".to_owned(),
            feld_id: feld_id.to_owned(),
            wert,
            zustand: if bestaetigt { Zustand::Bestaetigt } else { Zustand::Vorlaeufig },
            herkunft: Herkunft {
                herkunft: Achsenwert::new("laie").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            }
            .into(),
            schreiber: Schreiber::Mensch("julius".to_owned()),
            signal: None,
            ersetzt: None,
        };
        e.event_id = e.berechne_event_id().unwrap();
        e
    }

    /// Steht die Frage `feld` in der Queue eines Falls (VZ 2025), der nur `geburtsjahr` kennt?
    fn frage_steht(feld: &str, geburtsjahr: Option<(PyWert, bool)>) -> bool {
        let reg = crate::doctest_registry().expect("registry");
        let g = Graph::aus_registry(&reg);
        let events = geburtsjahr
            .into_iter()
            .map(|(w, b)| ereignis("geburtsjahr", w, b))
            .collect();
        let s = Store::aus_datei(store::StoreDatei {
            version: 1,
            veranlagungszeitraum: store::Veranlagungsjahr(2025),
            fall_id: None,
            scheibe: None,
            user_id: None,
            events,
            snapshots: Vec::new(),
            vorjahr_referenz: None,
        });
        let ohne_beitrag: Option<&HashMap<String, i64>> = None;
        naechste_fragen(&s, g.alle(), &g, ohne_beitrag).contains(&feld)
    }

    /// `feld_bedingung.alter_im_vz` (Abweichung Nr. 32): die Frage nach dem 55. Geburtstag steht nur, wenn das BESTAETIGTE
    /// Geburtsjahr im Veranlagungsjahr genau 55 Jahre ergibt (2025 − 1970). Jedes andere bestaetigte Ganzzahl-Geburtsjahr
    /// schliesst sie aus, auch am Rand von `i64` (kein Ueberlauf). Fail-closed: kein, ein vorlaeufiges oder ein bestaetigtes
    /// Geburtsjahr, das keine Ganzzahl ist, schliesst sie nicht aus.
    #[test]
    fn die_altersbedingung_schliesst_nur_ein_bestaetigtes_anderes_geburtsjahr_aus() {
        let frage = |gj| frage_steht("alter_55_vor_verkauf", gj);
        let bestaetigt = |w: PyWert| Some((w, true));
        let vorlaeufig = |w: PyWert| Some((w, false));
        assert!(frage(bestaetigt(PyWert::Ganz(1970))), "2025 minus 1970 ist 55");
        for anders in [1969, 1971, 1900, 2010, 0, -1] {
            assert!(!frage(bestaetigt(PyWert::Ganz(anders))), "geboren {anders}");
        }
        for rand in [i64::MIN, i64::MAX] {
            assert!(!frage(bestaetigt(PyWert::Ganz(rand))), "geboren {rand}: kein Ueberlauf, kein 55. Jahr");
        }
        assert!(frage(None), "ohne Geburtsjahr bleibt die Frage");
        assert!(frage(vorlaeufig(PyWert::Ganz(1971))), "ein vorlaeufiges Geburtsjahr schliesst nicht aus");
        assert!(frage(vorlaeufig(PyWert::Ganz(1970))), "ein vorlaeufiges Geburtsjahr im Jahr 55 laesst die Frage stehen");
        for kein_jahr in [PyWert::Text("1971".to_owned()), PyWert::Gleit(1971.0), PyWert::Bool(false), PyWert::Null] {
            assert!(
                frage(bestaetigt(kein_jahr.clone())),
                "ein bestaetigter Wert, der keine Ganzzahl ist ({kein_jahr:?}), schliesst die Frage nicht aus"
            );
        }
    }
}
