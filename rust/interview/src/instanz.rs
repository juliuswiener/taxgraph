//! Instanz-Achse (`traverser.py:106-265`): wie viele Instanzen ein Feld hat, wie eine heisst,
//! welche fehlen.
use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU16;

use domain::{BasisId, FeldId};
use serde::Serialize;
use store::{SnapshotFeld, Store};

use crate::antwort::{Aktiv, Antwort, InstanzAnzahl};
use crate::graph::{Graph, Sicht};

/// Instanz `i` eines Feldes: `basis` fuer die erste, `basis__i` ab der zweiten
/// (`instanz_feld_id`, `traverser.py:144-152`). Die Regel selbst ist `domain::FeldId`.
///
/// ```
/// use std::num::NonZeroU16;
/// use domain::BasisId;
/// let b = BasisId::new("kind_vorname").unwrap();
/// assert_eq!(interview::instanz_feld_id(&b, NonZeroU16::MIN).to_string(), "kind_vorname");
/// assert_eq!(interview::instanz_feld_id(&b, NonZeroU16::new(3).unwrap()).to_string(), "kind_vorname__3");
/// ```
#[must_use]
pub fn instanz_feld_id(basis: &BasisId, i: NonZeroU16) -> FeldId {
    FeldId::instanz(basis.clone(), i).unwrap_or_else(|_| FeldId::erste_instanz(basis.clone()))
}

/// Der Schluessel von Instanz `i` eines beliebigen Feld-Strings. `None`, wenn `feld` keine
/// gueltige Basis ist und `i >= 2` — ein solches Feld hat keine Instanzen.
fn schluessel(feld: &str, i: NonZeroU16) -> Option<String> {
    if i == NonZeroU16::MIN {
        return Some(feld.to_owned());
    }
    BasisId::new(feld).ok().map(|b| instanz_feld_id(&b, i).to_string())
}

/// Die Antworten ALLER Instanzen von `feld`, nach Nummer (`_instanz_antworten`,
/// `traverser.py:252-265`): Instanzen 1..=n aus dem bestaetigten Zaehlfeld (eine angekuendigte,
/// leere Instanz gilt als offen) plus jede Instanz, die im Store steht.
pub(crate) fn instanz_antworten<'s>(
    aktiv: &Aktiv<'s>,
    sicht: &Sicht<'_>,
    graph: &Graph<'_>,
    feld: &str,
) -> Vec<Antwort<'s>> {
    let n = graph
        .gruppe_von(sicht, feld)
        .map_or(InstanzAnzahl::EINS, |g| InstanzAnzahl::aus_zaehlfeld(aktiv.antwort(&g.anzahl_feld), g));
    let mut nummern: BTreeSet<NonZeroU16> = (1..=n.get()).filter_map(NonZeroU16::new).collect();
    nummern.extend(aktiv.instanzen_von(feld));
    nummern
        .into_iter()
        .map(|i| schluessel(feld, i).map_or(Antwort::Offen, |k| aktiv.antwort(&k)))
        .collect()
}

/// `true`, wenn `feld_id` zu einer Gruppe gehoert, deren bestaetigte Zahl mehr Instanzen
/// verlangt, als bestaetigt sind (`_instanz_unvollstaendig`, `traverser.py:155-188`). Haelt das
/// Basisfeld in der Queue, sonst fiele `kind_vorname__2` nach Instanz 1 fuer immer heraus.
pub(crate) fn instanz_unvollstaendig(aktiv: &Aktiv<'_>, sicht: &Sicht<'_>, graph: &Graph<'_>, feld_id: &str) -> bool {
    let Some(g) = graph.gruppe_von(sicht, feld_id) else { return false };
    if feld_id == g.anzahl_feld {
        return false;
    }
    let anzahl = InstanzAnzahl::aus_zaehlfeld(aktiv.antwort(&g.anzahl_feld), g);
    if anzahl.get() < 2 {
        return false;
    }
    (1..=anzahl.get())
        .filter_map(NonZeroU16::new)
        .any(|i| schluessel(feld_id, i).is_none_or(|k| aktiv.antwort(&k).ist_offen()))
}

/// Wie viele Eingabefelder braucht `feld_id` — und wie heisst eine Instanz
/// (`instanz_anzahl`, `traverser.py:106-119`)? `(1, "")` ohne Instanz-Achse oder ohne gepflegte
/// Gruppe, sonst die BESTAETIGTE Zahl aus dem Zaehlfeld, gekappt auf `max`.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let s = store::Store::leer(2025, None);
/// let (n, etikett) = interview::instanz_anzahl(&s, g.alle(), &g, "veranlagung");
/// assert_eq!((n.get(), etikett), (1, ""));
/// ```
#[must_use]
pub fn instanz_anzahl<'r>(store: &Store, sicht: &Sicht<'r>, graph: &Graph<'r>, feld_id: &str) -> (InstanzAnzahl, &'r str) {
    match graph.gruppe_von(sicht, feld_id) {
        None => (InstanzAnzahl::EINS, ""),
        Some(g) => {
            let aktiv = Aktiv::aus(store);
            (InstanzAnzahl::aus_zaehlfeld(aktiv.antwort(&g.anzahl_feld), g), g.etikett.as_str())
        }
    }
}

/// Eine begonnene, aber luecke Instanz-Reihe (`fehlende_instanzen`, Rueckgabe-Eintrag).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FehlendeInstanz<'r> {
    pub feld_id: &'r str,
    pub gruppe: &'r str,
    pub etikett: &'r str,
    pub anzahl: u16,
    pub vorhanden: Vec<u16>,
    pub fehlend: Vec<u16>,
}

/// Welche angekuendigten Instanzen hat der Nutzer NICHT ausgefuellt
/// (`fehlende_instanzen`, `traverser.py:191-234`)? Nur Reihen, die schon eine bestaetigte
/// Instanz tragen; arbeitet auf dem materialisierten Snapshot, nach `feld_id` sortiert.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let felder = std::collections::BTreeMap::new();
/// assert!(interview::fehlende_instanzen(&felder, g.alle(), &g).is_empty());
/// ```
#[must_use]
pub fn fehlende_instanzen<'r>(
    felder: &BTreeMap<String, SnapshotFeld>,
    sicht: &Sicht<'r>,
    graph: &Graph<'r>,
) -> Vec<FehlendeInstanz<'r>> {
    let mut basen: Vec<&'r str> = sicht.feld_ids().collect();
    basen.sort_unstable();
    let mut out = Vec::new();
    for basis in basen {
        let Some(g) = graph.gruppe_von(sicht, basis) else { continue };
        if basis == g.anzahl_feld {
            continue;
        }
        let anzahl = InstanzAnzahl::aus_zaehlfeld(Antwort::aus(felder.get(&g.anzahl_feld)), g);
        if anzahl.get() < 2 {
            continue;
        }
        let (mut vorhanden, mut fehlend) = (Vec::new(), Vec::new());
        for i in (1..=anzahl.get()).filter_map(NonZeroU16::new) {
            let da = schluessel(basis, i).is_some_and(|k| !Antwort::aus(felder.get(&k)).ist_offen());
            if da { vorhanden.push(i.get()) } else { fehlend.push(i.get()) }
        }
        if !vorhanden.is_empty() && !fehlend.is_empty() {
            out.push(FehlendeInstanz {
                feld_id: basis,
                gruppe: g.gruppe.as_str(),
                etikett: g.etikett.as_str(),
                anzahl: anzahl.get(),
                vorhanden,
                fehlend,
            });
        }
    }
    out
}
