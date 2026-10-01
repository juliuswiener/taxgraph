//! Vorwaerts = Beweis (`justification`, `trace_ergebnis`, `traverser.py:738-773`): je Feld das
//! aktive Event plus seine Stelle im Regelwerk. Regel/Slot/Feld/Event-exakt; eine Attribution je
//! Cent ist benannter Nachtrag (KONZEPT.md).
use std::collections::BTreeMap;

use bindung::{AnkerRef, Bindungspunkt};
use domain::{HerkunftVektor, PyWert, Zustand};
use serde::Serialize;
use store::{Event, EventId, Signal, Store};

use crate::antwort::Aktiv;
use crate::graph::Sicht;

/// Die Norm-Fundstelle eines Feldes in JSON-Form (`bindung.anker_ref`); `datei` nur, wenn
/// gesetzt (Python gibt das YAML-Dict roh zurueck, dort fehlt der Schluessel dann).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AnkerRefSicht<'r> {
    pub quelle: &'r str,
    pub zitatanker: &'r str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datei: Option<&'r str>,
}

impl<'r> From<&'r AnkerRef> for AnkerRefSicht<'r> {
    fn from(a: &'r AnkerRef) -> Self {
        Self {
            quelle: &a.quelle,
            zitatanker: &a.zitatanker,
            datei: a.datei.as_deref(),
        }
    }
}

/// `wert` geht beim Serialisieren ueber den Konvertierer nach JSON -- wie [`store::Event`]
/// selbst, aus demselben Grund (Auflage 1/2: die Schluesselsortierung, auf der `canonical_json`
/// und damit jede `event_id` beruht, erhaelt nur der Umweg ueber `serde_json::Value`; ein
/// `Serialize` fuer [`PyWert`] direkt braeche sie still).
///
/// ponytail: ein NaN/inf-Wert scheitert hier als Serialisierungsfehler. Im Store-Pfad kann er
/// nicht entstehen -- der Konvertierer an der Append-Grenze weist ihn vorher ab.
fn ser_wert<S: serde::Serializer>(w: &PyWert, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::Error as _;
    w.zu_json()
        .map_err(|e| S::Error::custom(e.to_string()))?
        .serialize(s)
}

/// Das Justification-Objekt eines Feldes: Store-Event + Bindung.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Justification<'s, 'r> {
    pub feld_id: &'s str,
    #[serde(serialize_with = "ser_wert")]
    pub wert: &'s PyWert,
    pub zustand: Zustand,
    /// `HerkunftVektor` statt der strengen `Herkunft`: passthrough des Store-Events, wie
    /// `traverser.py::justification`/`trace_ergebnis` `ev["herkunft"]` unveraendert ausgeben —
    /// auch fuer die 32 realen Bestandsdateien mit der Alt-Form (s. `domain::HerkunftVektor`).
    pub herkunft: &'s HerkunftVektor,
    pub event_id: EventId,
    pub signal: Option<&'s Signal>,
    pub regel_id: Option<&'r str>,
    pub signatur_slot: Option<&'r str>,
    pub geltungsbedingung: Option<&'r str>,
    pub anker_ref: Option<AnkerRefSicht<'r>>,
}

fn aus_event<'s, 'r>(ev: &'s Event, sicht: &Sicht<'r>) -> Justification<'s, 'r> {
    let b = sicht.get(&ev.feld_id);
    let (signatur_slot, geltungsbedingung) = match b.map(|b| &b.quelle.bindungspunkt) {
        Some(Bindungspunkt::SignaturSlot(s)) => (Some(s.as_str()), None),
        Some(Bindungspunkt::Geltungsbedingung(g)) => (None, Some(g.as_str())),
        None => (None, None),
    };
    Justification {
        feld_id: &ev.feld_id,
        wert: &ev.wert,
        zustand: ev.zustand,
        herkunft: &ev.herkunft,
        event_id: ev.event_id,
        signal: ev.signal.as_ref(),
        regel_id: b.map(|b| b.quelle.regel_id.as_str()),
        signatur_slot,
        geltungsbedingung,
        anker_ref: b.map(|b| AnkerRefSicht::from(&b.anker_ref)),
    }
}

/// Rekursions-Blatt: die Justification eines Feldes; `None` ohne aktives Event
/// (`justification`, `traverser.py:738-757`). Ein Feld ausserhalb der Sicht traegt keine
/// Regel-Angaben (`None`), wie in Python.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// assert!(interview::justification(&store::Store::leer(2025, None), "veranlagung", g.alle()).is_none());
/// ```
#[must_use]
pub fn justification<'s, 'r>(
    store: &'s Store,
    feld_id: &str,
    sicht: &Sicht<'r>,
) -> Option<Justification<'s, 'r>> {
    store.aktives(feld_id).map(|ev| aus_event(ev, sicht))
}

/// Vorwaerts-Trace: je beteiligter Regel die Justifications ihrer belegten Felder, nach
/// `feld_id` sortiert (`trace_ergebnis`, `traverser.py:760-773`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Trace<'s, 'r> {
    pub basis_snapshot: Option<&'s str>,
    pub regeln: BTreeMap<&'r str, Vec<Justification<'s, 'r>>>,
}

/// Baut den [`Trace`]; Felder ohne Bindung in der Sicht fallen heraus.
///
/// ```
/// let reg = interview::doctest_registry().unwrap();
/// let g = interview::Graph::aus_registry(&reg);
/// let s = store::Store::leer(2025, None);
/// let t = interview::trace_ergebnis(&s, g.alle(), Some("abc"));
/// assert!(t.regeln.is_empty());
/// assert_eq!(t.basis_snapshot, Some("abc"));
/// ```
#[must_use]
pub fn trace_ergebnis<'s, 'r>(
    store: &'s Store,
    sicht: &Sicht<'r>,
    snapshot_id: Option<&'s str>,
) -> Trace<'s, 'r> {
    let mut regeln: BTreeMap<&'r str, Vec<Justification<'s, 'r>>> = BTreeMap::new();
    for (fid, ev) in Aktiv::aus(store).iter() {
        if let Some(b) = sicht.get(fid) {
            regeln
                .entry(b.quelle.regel_id.as_str())
                .or_default()
                .push(aus_event(ev, sicht));
        }
    }
    for js in regeln.values_mut() {
        js.sort_by(|a, b| a.feld_id.cmp(b.feld_id));
    }
    Trace {
        basis_snapshot: snapshot_id,
        regeln,
    }
}
