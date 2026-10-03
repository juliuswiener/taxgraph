//! Der Sachverhalts-Store: append-only Event-Log + Ableitungen + Materialisierung
//! (`produkt/store/store.py`). [`Store::append`] ist DER EINE Schreibpfad — die Ableitungen
//! [`Store::leite_ab`]/[`Store::rechne_ab`] schreiben nur ueber die private [`Store::push_neu`],
//! keine zweite Implementierung.
use std::collections::{BTreeMap, HashMap, HashSet};

use domain::{
    Achsenwert, Feldzustand, Herkunft, HerkunftVektor, PruefTiefe, PyWert, Schreiber, Zustand,
};
use serde::{Deserialize, Serialize};

use crate::ableitung;
use crate::abweisung::{self, Abweisung, AbweisungRoh};
use crate::canonical::EventId;
use crate::event::{Event, NeuesEvent, NeuesEventRoh, Signal};
use crate::katalog::Katalog;
use crate::nachschlag::BindungNachschlag;
use crate::zeit::{jetzt_iso, ts_oder_jetzt};

/// Die reine Datei-Form (`schema.json` Top-Level-Objekt), OHNE den `aktiv`-Index — das ist, was
/// `persistenz::lade`/`speichere` lesen/schreiben (`store.py:77-87`). Feldliste 1:1
/// `schema.json`s Top-Level-`properties`; nur `version`/`veranlagungszeitraum`/`events` sind dort
/// `required`, alle anderen bleiben `Option` bzw. leer-default, auch wenn sie ueber dem realen
/// 192-Dateien-Bestand haeufiger vorkommen (s. Einzeldoku je Feld).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDatei {
    pub version: u32,
    /// Bewusst `Veranlagungsjahr` (roh), nicht `domain::Vz`: `schema.json` erlaubt
    /// `minimum:2000, maximum:2100`, und Pythons `leerer_store`/`_berechne` rechnen mit
    /// uneingeschraenktem `int(...)`. `domain::Vz` deckt nur 2024..=2026 ab (Rechenkern-Bereich)
    /// — waere hier strenger als die Datei selbst sein darf. PARITAET: keine Bereichspruefung,
    /// wie im Original — s. [`Veranlagungsjahr`]-Doku.
    pub veranlagungszeitraum: Veranlagungsjahr,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fall_id: Option<String>,
    /// Fragen-Scheibe des Falls (`schema.json`: `fall_anlegen()` setzt sie, danach nie geaendert).
    /// Gemessen ueber alle 192 realen Fallakten (Zaehlung): in JEDER Datei vorhanden — trotzdem
    /// `Option`, weil `schema.json` sie NICHT in `required` fuehrt (ein Store vor `fall_anlegen()`
    /// koennte sie theoretisch nicht tragen). Bisher in `StoreDatei` gefehlt: `lade`→`speichere`
    /// haette sie fuer jede reale Datei lautlos verworfen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheibe: Option<String>,
    /// Besitzer des Falls (`schema.json`: fehlt im Einzelnutzer-Betrieb `TAXGRAPH_NO_AUTH=1`).
    /// Gemessen: 39 der 192 realen Fallakten tragen sie. Bisher in `StoreDatei` gefehlt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub events: Vec<Event>,
    #[serde(default)]
    pub snapshots: Vec<Snapshot>,
    /// Vergleichsgroessen aus einem verknuepften Vorjahres-Fall (`api.vorjahr()`,
    /// `produkt/eingang/vorjahr_writer.py`), z. B. `{"verlustvortrag_bestand": {"wert": ...}}` —
    /// `schema.json` selbst laesst die Objektform bewusst offen ("Heute nur
    /// `verlustvortrag_bestand`", weitere Vergleichsgroessen sind absehbar), deshalb roh als
    /// `PyWert` statt vorab auf die heutige eine Unterstruktur festgelegt. Gemessen:
    /// 0 der 192 realen Fallakten tragen sie (kein bisher verknuepfter Vorjahres-Fall im Bestand);
    /// Python KANN sie trotzdem schreiben — bisher in `StoreDatei` gefehlt.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "vorjahr_referenz_als_json"
    )]
    pub vorjahr_referenz: Option<PyWert>,
}

/// `vorjahr_referenz` ueber den Konvertierer `PyWert -> Value`, wie `Event` (s. dort): `PyWert`
/// hat bewusst kein `Serialize`. Ein NaN/inf wird ein Serialisierungsfehler, nie ein `null`.
#[allow(clippy::ref_option, reason = "`serialize_with` verlangt `&Feldtyp`")]
fn vorjahr_referenz_als_json<S: serde::Serializer>(
    wert: &Option<PyWert>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::Error as _;
    wert.as_ref()
        .map(PyWert::zu_json)
        .transpose()
        .map_err(|e| S::Error::custom(e.to_string()))?
        .serialize(serializer)
}

/// Das `veranlagungszeitraum`-Feld einer Store-Datei — RAW wie Python (`int(...)`, `store.py:79`),
/// keine Bereichspruefung beim Laden. Gemessen ueber alle 192 realen Fallakten unter
/// `~/.local/share/taxgraph/faelle/` (Zaehlung, kein Wert ausserhalb dieses Kommentars
/// festgehalten): neben sinnvollen Werten und Ausreissern wie `-5`/`2099` (beide passen in
/// `i64`) traegt EINE Datei einen 38-stelligen Wert (`99999999999999999999999999999999999999`),
/// der `i64` ueberlaeuft — genau der Grund, warum `store::lade` diese Datei bisher ablehnte.
///
/// Einen `veranlagungszeitraum` in Exponentialschreibweise (z. B. `2.025e3`) traegt keine Akte,
/// gemessen 2026-10-02: 0 in 192 unter `faelle/`, 0 in den 15 Akten der Quarantaene daneben
/// (`quarantaene-2026-10-01-kegel-testlauf/`). `persistenz::lade` liest mit
/// `serde_json::from_str` nur Ziffern und weist `2.025e3` als `PersistenzFehler::Format` ab;
/// `json.load` liest `2025.0`. Liefert ein Deserializer die Zahl als `f64` (z. B.
/// `serde_json::from_value` in den Parity-Tests), schneidet die [`Deserialize`]-Impl unten
/// Richtung Null ab wie Pythons `int(float)`.
///
/// ponytail: `i128` statt Pythons echtem, beliebig grossem `int` — deckt jeden gemessenen Wert
/// (auch den 38-stelligen) mit weitem Rand; ein noch groesserer Wert wird beim Laden zu einem
/// benannten Deserialisierungsfehler (serde meldet den Ueberlauf explizit), nicht zu einem
/// stillen Wrap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Veranlagungsjahr(pub i128);

impl<'de> Deserialize<'de> for Veranlagungsjahr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BesucherJahr;
        impl serde::de::Visitor<'_> for BesucherJahr {
            type Value = Veranlagungsjahr;

            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("eine Ganzzahl (optional in Exponentialschreibweise)")
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(Veranlagungsjahr(i128::from(v)))
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(Veranlagungsjahr(i128::from(v)))
            }

            fn visit_i128<E: serde::de::Error>(self, v: i128) -> Result<Self::Value, E> {
                Ok(Veranlagungsjahr(v))
            }

            fn visit_u128<E: serde::de::Error>(self, v: u128) -> Result<Self::Value, E> {
                Ok(Veranlagungsjahr(i128::try_from(v).unwrap_or(i128::MAX)))
            }

            // Python: `int(float)` schneidet Richtung Null ab. `f.trunc()` endlich; `as`
            // saettigt an den Grenzen statt zu ueberlaufen (kein Panic, kein UB) — s. Typdoku oben.
            #[allow(clippy::cast_possible_truncation)]
            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(Veranlagungsjahr(v.trunc() as i128))
            }
        }
        // Derselbe getypte Hint wie `i128::deserialize`: `serde_json::from_str` liest die
        // Ganzzahl exakt, auch ueber `u64` (`persistenz::lade`). `visit_f64` greift nur, wo der
        // Deserializer eine Kommazahl liefert, z. B. `serde_json::from_value`.
        deserializer.deserialize_i128(BesucherJahr)
    }
}

impl Veranlagungsjahr {
    /// Fuer Aufrufer, die den (bewusst engeren) `i64`-Bereich brauchen — [`Store::leer`] und die
    /// Datums-Ableitung (`ableitung::berechne`, `vz: i64`). Saettigt an den `i64`-Grenzen statt
    /// zu ueberlaufen: betrifft praktisch nur die eine gemessene 38-stellige Datei, deren Wert
    /// fuer jede reale Ableitungsregel ohnehin weit ausserhalb jeder sinnvollen Jahresspanne
    /// liegt — ein gesaettigtes `i64::MAX` fuehrt zu denselben "viel zu weit in der Zukunft"-
    /// Vergleichsergebnissen wie Pythons unbeschraenkte Arithmetik mit dem echten 38-stelligen
    /// Wert.
    ///
    /// ```
    /// use store::Veranlagungsjahr;
    /// assert_eq!(Veranlagungsjahr(-5).als_i64_saettigend(), -5); // keine Bereichspruefung
    /// assert_eq!(Veranlagungsjahr(i128::MAX).als_i64_saettigend(), i64::MAX);
    /// assert_eq!(Veranlagungsjahr(i128::MIN).als_i64_saettigend(), i64::MIN);
    /// ```
    #[must_use]
    pub fn als_i64_saettigend(self) -> i64 {
        i64::try_from(self.0).unwrap_or(if self.0.is_positive() {
            i64::MAX
        } else {
            i64::MIN
        })
    }
}

impl From<i64> for Veranlagungsjahr {
    fn from(v: i64) -> Self {
        Self(i128::from(v))
    }
}

/// Materialisierter Feldwert innerhalb eines Snapshots (`schema.json#/$defs/snapshot_feld`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SnapshotFeld {
    pub wert: domain::PyWert,
    pub zustand: Zustand,
    pub herkunft: HerkunftVektor,
}

/// `SnapshotFeld` serialisiert wie [`crate::event::Event`] ueber den Konvertierer
/// `PyWert -> Value` — NICHT ueber ein abgeleitetes `Serialize`. `snapshot_id` ist
/// `sha256(canonical_json(felder))` und erbt die Schluesselsortierung von `serde_json::Map`
/// (`BTreeMap`); ein direktes Serialisieren von `PyWert::Objekt` uebernaehme dessen
/// Einfuegereihenfolge (Auflage 1/2, s. `Event`).
impl Serialize for SnapshotFeld {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error as _;
        serde_json::json!({
            "wert": self.wert.zu_json().map_err(|e| S::Error::custom(e.to_string()))?,
            "zustand": self.zustand,
            "herkunft": self.herkunft,
        })
        .serialize(serializer)
    }
}

/// ELSTER-Pruefergebnis-Klasse (`schema.json#/$defs/eric_befund.klasse`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EricKlasse {
    Plausibel,
    PlausibilitaetFehler,
    IoGateNichtGeprueft,
    HerstellerIdGesperrt,
    DatenartversionUnbekannt,
    IoReaderUnerwarteteElemente,
    Sonstig,
}

/// Eingabe fuer [`Store::erzeuge_snapshot`]: derselbe Befund, aber OHNE `gebunden_an` — der
/// Snapshot setzt das Feld selbst (`store.py:612-614`: "bindet unweigerlich an DIESEN Zustand").
#[derive(Debug, Clone)]
pub struct EricBefundEingabe {
    pub rc: i64,
    pub klasse: EricKlasse,
    pub gekappt_verdacht: bool,
    pub fehler_anzahl: Option<u32>,
}

/// ELSTER-Pruefergebnis, gebunden an einen Snapshot-Hash (`schema.json#/$defs/eric_befund`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EricBefund {
    pub gebunden_an: EventId,
    pub rc: i64,
    pub klasse: EricKlasse,
    pub gekappt_verdacht: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fehler_anzahl: Option<u32>,
}

/// Ein Sachverhalts-Snapshot (`schema.json#/$defs/snapshot`, `store.py:606-617`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: EventId,
    pub ts: String,
    pub bis_event: EventId,
    /// `BTreeMap` statt `HashMap`: sortierte Schluessel sind Teil der Content-Adresse
    /// (`store.py:602`: "feld_id-sortiert für stabile Content-Adresse").
    pub felder: BTreeMap<String, SnapshotFeld>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eric_befund: Option<EricBefund>,
}

/// Fehler aus [`Store::materialisiere`]/[`Store::erzeuge_snapshot`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotFehler {
    /// `store.py:591`: `bis_event` steht nicht im Log.
    #[error("bis_event {0} nicht im Log")]
    BisEventUnbekannt(EventId),
    /// `store.py:610`: `store["events"][-1]` auf leerem Log ist in Python ein ungefangener
    /// `IndexError`. Hier ein expliziter Fehler statt eines Absturzes (fail-closed statt Panik).
    #[error("erzeuge_snapshot ohne bis_event auf einem leeren Log")]
    LeererLogOhneBisEvent,
    /// K2-Auflage 3: ein Feldwert geht nicht nach JSON (NaN/+-inf, auch in `Liste`/`Objekt`).
    /// KEIN stiller `null`-Fallback: der ergaebe eine `snapshot_id` ueber `null`, die zu keinem
    /// Event passt — der Fehler waere dann ein falscher Hash, kein sichtbarer Ausfall.
    #[error("Snapshot-Wert nicht darstellbar: {0}")]
    WertNichtDarstellbar(String),
}

/// Der Sachverhalts-Store. `events`/`snapshots` liegen in [`StoreDatei`]; `aktiv` ist der private
/// `feld_id -> Index`-Cache fuer das aktuell aktive Event je Feld (Auflage B).
#[derive(Debug, Clone)]
pub struct Store {
    datei: StoreDatei,
    aktiv: HashMap<String, usize>,
}

impl Store {
    /// `store.py:77-81`, `leerer_store`.
    ///
    /// ```
    /// let s = store::Store::leer(2025, Some("demo-1".to_string()));
    /// assert_eq!(s.veranlagungszeitraum(), 2025);
    /// assert_eq!(s.datei().fall_id.as_deref(), Some("demo-1"));
    /// assert!(s.events().is_empty() && s.snapshots().is_empty());
    /// ```
    #[must_use]
    pub fn leer(veranlagungszeitraum: i64, fall_id: Option<String>) -> Self {
        Self {
            datei: StoreDatei {
                version: 1,
                veranlagungszeitraum: veranlagungszeitraum.into(),
                fall_id,
                scheibe: None,
                user_id: None,
                events: Vec::new(),
                snapshots: Vec::new(),
                vorjahr_referenz: None,
            },
            aktiv: HashMap::new(),
        }
    }

    /// Baut den `aktiv`-Index aus einer geladenen [`StoreDatei`] neu auf (`store.py:92-100`,
    /// `_aktives`) — die Datei speichert den Index nicht mit.
    ///
    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let neu = |wert: i64, ersetzt: Option<store::EventId>| store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(wert).into(),
    /// #     feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt,
    /// #     ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    /// # };
    /// # let leer = std::collections::HashMap::new();
    /// # let bindung = store::BindungNachschlag::neu(&leer);
    /// let mut s = store::Store::leer(2025, None);
    /// let erste = s.append(&neu(220, None), None, bindung).unwrap();
    /// let zweite = s.append(&neu(230, Some(erste)), None, bindung).unwrap();
    /// let geladen = store::Store::aus_datei(s.into_datei());
    /// assert_eq!(geladen.aktives("ep_arbeitstage").unwrap().event_id, zweite);
    /// ```
    #[must_use]
    pub fn aus_datei(datei: StoreDatei) -> Self {
        let aktiv = baue_aktiv_index(&datei.events);
        Self { datei, aktiv }
    }

    /// ```
    /// let s = store::Store::leer(2025, None);
    /// assert_eq!(s.datei().version, 1);
    /// assert_eq!(s.datei().veranlagungszeitraum, store::Veranlagungsjahr(2025));
    /// ```
    #[must_use]
    pub fn datei(&self) -> &StoreDatei {
        &self.datei
    }

    /// ```
    /// let datei = store::Store::leer(2025, Some("demo-1".to_string())).into_datei();
    /// assert_eq!(datei.fall_id.as_deref(), Some("demo-1"));
    /// assert_eq!(store::Store::aus_datei(datei).veranlagungszeitraum(), 2025);
    /// ```
    #[must_use]
    pub fn into_datei(self) -> StoreDatei {
        self.datei
    }

    /// `store["vorjahr_referenz"] = ...` (`vorjahr_writer.uebernehme_vorjahr`): die Vergleichsgrösse aus
    /// dem Vorjahres-Fall, die `preflight` liest. Ersetzt eine frühere.
    ///
    /// ```
    /// let mut s = store::Store::leer(2025, None);
    /// assert!(s.datei().vorjahr_referenz.is_none());
    /// s.setze_vorjahr_referenz(serde_json::json!({"verlustvortrag_bestand": {"wert": 5}}).into());
    /// assert!(s.datei().vorjahr_referenz.is_some());
    /// ```
    pub fn setze_vorjahr_referenz(&mut self, referenz: PyWert) {
        self.datei.vorjahr_referenz = Some(referenz);
    }

    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let neu = |wert: i64, ersetzt: Option<store::EventId>| store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(wert).into(),
    /// #     feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt,
    /// #     ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    /// # };
    /// # let leer = std::collections::HashMap::new();
    /// # let bindung = store::BindungNachschlag::neu(&leer);
    /// let mut s = store::Store::leer(2025, None);
    /// let erste = s.append(&neu(220, None), None, bindung).unwrap();
    /// let zweite = s.append(&neu(230, Some(erste)), None, bindung).unwrap();
    /// // append-only: das ersetzte Event bleibt im Log
    /// let ids: Vec<store::EventId> = s.events().iter().map(|e| e.event_id).collect();
    /// assert_eq!(ids, [erste, zweite]);
    /// ```
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.datei.events
    }

    /// ```
    /// let mut s = store::Store::leer(2025, None);
    /// assert!(s.erzeuge_snapshot(None, None, None).is_err()); // leerer Log, kein `bis_event`
    /// assert!(s.snapshots().is_empty());
    /// ```
    #[must_use]
    pub fn snapshots(&self) -> &[Snapshot] {
        &self.datei.snapshots
    }

    /// ```
    /// let mut datei = store::Store::leer(2025, None).into_datei();
    /// assert_eq!(store::Store::aus_datei(datei.clone()).veranlagungszeitraum(), 2025);
    /// datei.veranlagungszeitraum = store::Veranlagungsjahr(i128::MAX); // gesaettigt
    /// assert_eq!(store::Store::aus_datei(datei).veranlagungszeitraum(), i64::MAX);
    /// ```
    #[must_use]
    pub fn veranlagungszeitraum(&self) -> i64 {
        self.datei.veranlagungszeitraum.als_i64_saettigend()
    }

    /// Das aktuell aktive Event fuer `feld_id`, wenn eines existiert (`store.py:92-100`,
    /// `_aktives`, hier direkt ueber den gepflegten Index statt per Voll-Scan).
    ///
    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let neu = |wert: i64, ersetzt: Option<store::EventId>| store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(wert).into(),
    /// #     feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt,
    /// #     ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    /// # };
    /// # let leer = std::collections::HashMap::new();
    /// # let bindung = store::BindungNachschlag::neu(&leer);
    /// let mut s = store::Store::leer(2025, None);
    /// assert!(s.aktives("ep_arbeitstage").is_none());
    /// let erste = s.append(&neu(220, None), None, bindung).unwrap();
    /// assert_eq!(s.aktives("ep_arbeitstage").unwrap().event_id, erste);
    /// let zweite = s.append(&neu(230, Some(erste)), None, bindung).unwrap();
    /// assert_eq!(s.aktives("ep_arbeitstage").unwrap().event_id, zweite);
    /// ```
    #[must_use]
    pub fn aktives(&self, feld_id: &str) -> Option<&Event> {
        self.aktiv
            .get(feld_id)
            .and_then(|&idx| self.datei.events.get(idx))
    }

    /// Alle aktiven Events (`feld_id`, Event), in unbestimmter Reihenfolge. Dieselbe Quelle wie
    /// [`Store::aktives`]: `traverser.py:_aktive_events` dupliziert `store.py:_aktives` wortgleich,
    /// in Rust gibt es dafuer genau diesen einen Index.
    ///
    /// ```
    /// let s = store::Store::leer(2025, None);
    /// assert_eq!(s.aktive().count(), 0);
    /// ```
    pub fn aktive(&self) -> impl Iterator<Item = (&str, &Event)> + '_ {
        self.aktiv
            .iter()
            .filter_map(|(fid, &idx)| self.datei.events.get(idx).map(|e| (fid.as_str(), e)))
    }

    /// Prueft die Auflagen A/K1/F2/T/V/W/F/B und haengt bei Erfolg ein Event an (`store.py:251-390`,
    /// `append_event`). Gibt den `event_id` des neuen Events zurueck.
    ///
    /// PARITAET: `bindung` ist in Rust ein PFLICHT-Parameter (Python: `bindung: dict | None =
    /// None`, Auflage T greift dort nur, wenn ein Aufrufer sie uebergibt — mehr als 2100
    /// Bestandstests in Python lassen sie bewusst weg). Ein unbekanntes `feld_id` laesst
    /// [`BindungNachschlag::basis_eintrag`] durch (Ok), das Verhalten fuer JEDEN Aufruf, der in
    /// Python `bindung=` mitgab, bleibt identisch.
    ///
    /// `katalog` bleibt echt optional (`Option`): nur Vorschlags-Schreiber (`llm:`/`import:beleg`/
    /// `import:kontoauszug`/`berechnet:`) brauchen ihn ueberhaupt (Auflage K1).
    ///
    /// # Errors
    /// [`Abweisung`], wenn eine der Auflagen verletzt ist.
    ///
    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let neu = |wert: i64, ersetzt: Option<store::EventId>| store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(wert).into(),
    /// #     feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt,
    /// #     ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    /// # };
    /// let leer = std::collections::HashMap::new(); // ohne Bindung lassen Auflage T/F durch
    /// let bindung = store::BindungNachschlag::neu(&leer);
    /// let mut s = store::Store::leer(2025, None);
    /// let erste = s.append(&neu(220, None), None, bindung).unwrap();
    /// // Auflage B: ein zweites aktives Event fuer dasselbe Feld braucht `ersetzt`
    /// assert_eq!(
    ///     s.append(&neu(230, None), None, bindung),
    ///     Err(store::Abweisung::AktivesEventVorhanden {
    ///         feld_id: "ep_arbeitstage".to_string(),
    ///         aktives_event: erste,
    ///     })
    /// );
    /// assert!(s.append(&neu(230, Some(erste)), None, bindung).is_ok());
    /// ```
    pub fn append(
        &mut self,
        neu: &NeuesEvent,
        katalog: Option<&Katalog>,
        bindung: BindungNachschlag<'_>,
    ) -> Result<EventId, Abweisung> {
        Self::pruefe_auflage_a(neu)?;
        Self::pruefe_wert(&neu.feld_id, &neu.wert, &neu.schreiber, katalog, bindung)?;
        self.pruefe_auflage_b(&neu.feld_id, neu.ersetzt)?;
        let wert = &neu.wert;

        let signal =
            // Schreibpfad: Schluessel ist immer da (Python `store.py`: `signal or {"signal_1":
            // None, ...}`) -- aeussere Ebene daher immer `Some(...)` (s. `Signal`-Typdoku).
            Signal { signal_1: Some(neu.signal_1.clone()), signal_2: neu.signal_2_roh().map(str::to_owned), signal_2_fehlt: false };
        let event = Event {
            event_id: EventId::aus_bytes([0; 32]),
            ts: ts_oder_jetzt(neu.ts.clone()),
            feld_id: neu.feld_id.clone(),
            wert: neu.wert.clone(),
            zustand: neu.zustand(),
            herkunft: neu.herkunft.clone().into(),
            schreiber: neu.schreiber.clone(),
            signal: Some(signal),
            ersetzt: neu.ersetzt,
        };
        let event_id = self.push_neu(event)?;

        let vz = self.datei.veranlagungszeitraum.als_i64_saettigend();
        self.leite_ab(&neu.feld_id, wert, neu.zustand(), bindung)?;
        self.rechne_ab(&neu.feld_id, wert, neu.zustand(), bindung, vz)?;
        Ok(event_id)
    }

    /// Wie [`Store::append`], aber fuer die Formen, die Pythons `append_event` roh annimmt
    /// ([`NeuesEventRoh`]): dieselben Auflagen in Pythons Reihenfolge — A, K1, F2, T/V/W/F, dann
    /// `signal_2` (Text) und `bestaetigt` (braucht `signal_2`), dann B. Der Schreiber wird wie in
    /// Python nach Praefix klassifiziert, nicht nach Gleichheit.
    ///
    /// # Errors
    /// [`AbweisungRoh`], wenn eine Auflage verletzt ist.
    pub fn append_roh(
        &mut self,
        neu: &NeuesEventRoh,
        katalog: Option<&Katalog>,
        bindung: BindungNachschlag<'_>,
    ) -> Result<EventId, AbweisungRoh> {
        let pruef = python_schreiber(&neu.schreiber);
        let ohne_signal_2 = neu.signal.signal_2.is_none() && neu.signal_2_fremd.is_none();
        Self::pruefe_auflage_a_form(
            &pruef,
            neu.herkunft.herkunft_achse().as_str(),
            neu.zustand == Zustand::Vorlaeufig && ohne_signal_2,
            neu.ersetzt.is_some(),
        )?;
        Self::pruefe_wert(&neu.feld_id, &neu.wert, &pruef, katalog, bindung)?;
        if let Some(typ) = &neu.signal_2_fremd {
            return Err(AbweisungRoh::Signal2KeinText { typ: typ.clone() });
        }
        if neu.zustand == Zustand::Bestaetigt
            && neu
                .signal
                .signal_2
                .as_deref()
                .is_none_or(|s| domain::py_strip(s).is_empty())
        {
            return Err(AbweisungRoh::BestaetigtOhneSignal2);
        }
        let ersetzt = match &neu.ersetzt {
            None => None,
            // Eine Kennung ist genau der kleingeschriebene Hex-Text, den `EventId` ausgibt; `parse`
            // nimmt nichts anderes (Grossbuchstaben, `+`, falsche Laenge: `ErsetztZielText`).
            Some(text) => Some(
                EventId::parse(text).map_err(|_| AbweisungRoh::ErsetztZielText(text.clone()))?,
            ),
        };
        self.pruefe_auflage_b(&neu.feld_id, ersetzt)?;
        let event = Event {
            event_id: EventId::aus_bytes([0; 32]),
            ts: ts_oder_jetzt(neu.ts.clone()),
            feld_id: neu.feld_id.clone(),
            wert: neu.wert.clone(),
            zustand: neu.zustand,
            herkunft: neu.herkunft.clone(),
            schreiber: neu
                .schreiber
                .parse()
                .unwrap_or_else(|u: std::convert::Infallible| match u {}),
            signal: Some(neu.signal.clone()),
            ersetzt,
        };
        let event_id = self.push_neu(event)?;
        let vz = self.datei.veranlagungszeitraum.als_i64_saettigend();
        self.leite_ab(&neu.feld_id, &neu.wert, neu.zustand, bindung)?;
        self.rechne_ab(&neu.feld_id, &neu.wert, neu.zustand, bindung, vz)?;
        Ok(event_id)
    }

    /// Auflage A (`store.py:262-324`): ein Vorschlags-Schreiber deklariert sich ehrlich
    /// (`herkunft`/`zustand`/`signal_2`) und darf (ausser `berechnet:`) nie `ersetzt` tragen.
    fn pruefe_auflage_a(neu: &NeuesEvent) -> Result<(), Abweisung> {
        // `zustand != vorlaeufig` UND `signal_2 is_some()` sind in Python zwei getrennte Checks;
        // in Rust sind sie durch `Feldzustand` strukturell gekoppelt (nur `Vorlaeufig` traegt kein
        // `Signal2`) -- ein Check deckt beide ab.
        Self::pruefe_auflage_a_form(
            &neu.schreiber,
            neu.herkunft.herkunft.as_str(),
            matches!(neu.feldzustand, Feldzustand::Vorlaeufig),
            neu.ersetzt.is_some(),
        )
    }

    /// Auflage A auf den Rohmerkmalen: `herkunft_achse` ist die `herkunft`-Achse der Herkunft,
    /// `vorlaeufig_ohne_signal_2` heisst `zustand == vorlaeufig` UND `signal_2 is None` (Pythons
    /// beide Bedingungen), `ersetzt_gesetzt` `ersetzt is not None`. Der Name des Schreibers in der
    /// Meldung ist der Praefix, den Python dort fest schreibt (`llm:-Schreiber`).
    fn pruefe_auflage_a_form(
        schreiber: &Schreiber,
        herkunft_achse: &str,
        vorlaeufig_ohne_signal_2: bool,
        ersetzt_gesetzt: bool,
    ) -> Result<(), Abweisung> {
        let Some((erwartete_herkunft, folge)) = abweisung::auflage_a_erwartung(schreiber) else {
            return Ok(());
        };
        let ehrlich = herkunft_achse == erwartete_herkunft && vorlaeufig_ohne_signal_2;
        if !ehrlich {
            return Err(Abweisung::AuflageA {
                schreiber: abweisung::auflage_a_name(schreiber),
                erwartete_herkunft,
                folge,
            });
        }
        if abweisung::ersetzt_gesperrt(schreiber) && ersetzt_gesetzt {
            return Err(Abweisung::AuflageAErsetztGuard {
                schreiber: schreiber.to_string(),
            });
        }
        Ok(())
    }

    /// K2-Auflage 3 (Wert nach JSON), K1 (Katalog), F2 (Magnitude) und T/V/W/F (Bindung), in dieser
    /// Reihenfolge (`store.py:326-366`).
    fn pruefe_wert(
        feld_id: &str,
        wert: &PyWert,
        schreiber: &Schreiber,
        katalog: Option<&Katalog>,
        bindung: BindungNachschlag<'_>,
    ) -> Result<(), Abweisung> {
        // K2-Auflage 3: der Wert muss nach JSON gehen, sonst gibt es keinen `event_id`. Geprueft
        // EINMAL hier, vor den Auflagen, die den Wert lesen: NaN/inf ist ein Fehler, kein stiller
        // Wert, und die Fehlerklasse haengt nicht davon ab, welche Auflage ihn zuerst saehe.
        wert.zu_json()
            .map_err(|e| Abweisung::WertNichtDarstellbar {
                feld_id: feld_id.to_owned(),
                grund: e.to_string(),
            })?;
        if let Some(typ) = schreiber.vorschlag_typ() {
            let katalog = katalog.ok_or_else(|| Abweisung::KatalogFehlt {
                schreiber: schreiber.to_string(),
            })?;
            if !katalog.erlaubt(typ, feld_id) {
                return Err(Abweisung::KatalogNichtFreigegeben {
                    schreiber: schreiber.to_string(),
                    feld_id: feld_id.to_owned(),
                    typ,
                });
            }
            pruefe_magnitude(feld_id, wert, schreiber)?;
        }
        pruefe_bindung(feld_id, wert, bindung)
    }

    /// Auflage B (`store.py:368-381`): hoechstens ein aktives Event je `feld_id`; Ueberschreiben
    /// nur ueber ein gueltiges `ersetzt`-Ziel.
    fn pruefe_auflage_b(&self, feld_id: &str, ersetzt: Option<EventId>) -> Result<(), Abweisung> {
        match ersetzt {
            None => {
                // Invariante: jeder Index in `aktiv` zeigt auf ein vorhandenes Event (einziger
                // Schreibpfad ist `push_geprueft`, das Index und Vec gemeinsam pflegt). Ein
                // fehlender Treffer hier ist strukturell unerreichbar; `and_then` behandelt ihn
                // fail-open als "kein aktives Event" statt zu paniken.
                if let Some(aktives_event) = self
                    .aktiv
                    .get(feld_id)
                    .and_then(|&idx| self.datei.events.get(idx))
                    .map(|e| e.event_id)
                {
                    return Err(Abweisung::AktivesEventVorhanden {
                        feld_id: feld_id.to_owned(),
                        aktives_event,
                    });
                }
                Ok(())
            }
            Some(ziel) => {
                let Some(ziel_event) = self.datei.events.iter().find(|e| e.event_id == ziel) else {
                    return Err(Abweisung::ErsetztZielUnbekannt(ziel));
                };
                if ziel_event.feld_id != feld_id {
                    return Err(Abweisung::ErsetztFeldMismatch);
                }
                if self.datei.events.iter().any(|e| e.ersetzt == Some(ziel)) {
                    return Err(Abweisung::ErsetztBereitsErsetzt);
                }
                Ok(())
            }
        }
    }

    /// `store.py:393-444`, `_leite_ab`: eine bestaetigte Angabe oberhalb `beweist.ab` beantwortet
    /// die Existenzfrage `beweist.feld_id` gleich mit (nur wenn diese noch unbeantwortet ist,
    /// keine Kette — das abgeleitete Event durchlaeuft `leite_ab` nicht erneut).
    /// # Errors
    /// [`Abweisung::WertNichtDarstellbar`], wenn der abgeleitete `wert` nicht nach JSON geht.
    fn leite_ab(
        &mut self,
        feld_id: &str,
        wert: &PyWert,
        zustand: Zustand,
        bindung: BindungNachschlag<'_>,
    ) -> Result<(), Abweisung> {
        if zustand != Zustand::Bestaetigt {
            return Ok(());
        }
        let Some(eintrag) = bindung.get(feld_id) else {
            return Ok(());
        };
        let Some(regel) = &eintrag.beweist else {
            return Ok(());
        };
        let Some(zahl) = zahl_als_f64(wert) else {
            return Ok(());
        };
        if zahl < regel.ab.unwrap_or(1.0) {
            return Ok(());
        }
        if bindung.get(&regel.feld_id).is_none() || self.aktiv.contains_key(&regel.feld_id) {
            return Ok(());
        }
        let event = Event {
            event_id: EventId::aus_bytes([0; 32]),
            ts: jetzt_iso(),
            feld_id: regel.feld_id.clone(),
            wert: regel.wert.clone().into(),
            zustand: Zustand::Bestaetigt,
            herkunft: berechnet_herkunft().into(),
            schreiber: Schreiber::Abgeleitet("beweist".to_string()),
            signal: Some(Signal {
                signal_1: Some(None),
                // Python `f"...={wert}"` ist `str(wert)`. Fuer `int` ist das derselbe Text wie
                // vorher die JSON-Darstellung; fuer `float` folgt er jetzt Python (`1e+16`, wo
                // JSON `1e16` schrieb) -- `signal_2` geht in den `event_id`.
                signal_2: Some(format!("beweist@{feld_id}={}", wert.py_str())),
                signal_2_fehlt: false,
            }),
            ersetzt: None,
        };
        self.push_neu(event)?;
        Ok(())
    }

    /// `store.py:503-572`, `_rechne_ab`: Felder mit `ableitung`-Regel, deren Quelle ODER
    /// `und_feld` gerade bestaetigt wurde, werden berechnet (ZWEI Ausloeser). Liest `aktiv`
    /// EINMAL vor der Schleife (`store.py:529-531`: "keine Kette").
    /// # Errors
    /// [`Abweisung::WertNichtDarstellbar`], wenn ein abgeleiteter `wert` nicht nach JSON geht.
    fn rechne_ab(
        &mut self,
        feld_id: &str,
        wert: &PyWert,
        zustand: Zustand,
        bindung: BindungNachschlag<'_>,
        vz: i64,
    ) -> Result<(), Abweisung> {
        if zustand != Zustand::Bestaetigt {
            return Ok(());
        }
        let aktiv_snapshot = self.aktiv.clone();
        let mut neue_events = Vec::new();
        for (ziel, eintrag) in bindung.alle() {
            let Some(regel) = &eintrag.ableitung else {
                continue;
            };
            let und = regel.und_feld.as_deref();
            if feld_id != regel.aus && Some(feld_id) != und {
                continue;
            }
            if aktiv_snapshot.contains_key(ziel) {
                continue;
            }
            let quellwert = if regel.aus == feld_id {
                wert
            } else {
                let Some(qev) = aktiv_snapshot
                    .get(&regel.aus)
                    .and_then(|&i| self.datei.events.get(i))
                else {
                    continue;
                };
                if qev.zustand != Zustand::Bestaetigt {
                    continue;
                }
                &qev.wert
            };
            if let Some(und_feld) = und {
                let Some(ev) = aktiv_snapshot
                    .get(und_feld)
                    .and_then(|&i| self.datei.events.get(i))
                else {
                    continue;
                };
                // ponytail: `store.py:577` prueft `wert in (None, "", False)` mit Pythons
                // `==` — dort zaehlen auch `0` und `0.0` als leer, hier nicht. Die Divergenz
                // ist aelter als K2 (sie stand schon auf `Value`) und wird im heutigen Bestand
                // nicht ausgeloest. Upgrade: `PyWert::py_eq` gegen `Null`/`Text("")`/`Bool(false)`,
                // nach einer Messung ueber die Nullwerte des Bestands (s. Risikokarte).
                //
                // ponytail: strukturelle Gleichheit auf `PyWert` ist Wire-Roundtrip, KEIN
                // Wertvergleich — jede Wertpruefung gehoert ueber `py_eq`. Der strukturelle
                // Match hier ist die bewusste, auftragsgemaesse Wahl (Divergenz nicht
                // reparieren), kein Vorbild fuer neue Vergleiche.
                let leer = match &ev.wert {
                    domain::PyWert::Null | domain::PyWert::Bool(false) => true,
                    domain::PyWert::Text(s) => s.is_empty(),
                    _ => false,
                };
                if ev.zustand != Zustand::Bestaetigt || leer {
                    continue;
                }
            }
            let Some(neuer_wert) = ableitung::berechne(regel, quellwert, vz) else {
                continue;
            };
            neue_events.push(Event {
                event_id: EventId::aus_bytes([0; 32]),
                ts: jetzt_iso(),
                feld_id: ziel.to_string(),
                wert: neuer_wert.into(),
                zustand: Zustand::Bestaetigt,
                herkunft: berechnet_herkunft().into(),
                schreiber: Schreiber::Abgeleitet("ableitung".to_string()),
                signal: Some(Signal {
                    signal_1: Some(None),
                    signal_2: Some(format!("ableitung@{}", regel.aus)),
                    signal_2_fehlt: false,
                }),
                ersetzt: None,
            });
        }
        for event in neue_events {
            self.push_neu(event)?;
        }
        Ok(())
    }

    /// Berechnet den `event_id` und haengt an (Ableitungen und `append` teilen sich diesen einen
    /// Pfad, s. Modul-Dokumentation).
    /// # Errors
    /// [`Abweisung::WertNichtDarstellbar`], wenn `event.wert` nicht nach JSON geht.
    fn push_neu(&mut self, mut event: Event) -> Result<EventId, Abweisung> {
        event.event_id =
            event
                .berechne_event_id()
                .map_err(|e| Abweisung::WertNichtDarstellbar {
                    feld_id: event.feld_id.clone(),
                    grund: e.to_string(),
                })?;
        Ok(self.push_geprueft(event))
    }

    /// Haengt ein bereits geprueftes Event an und pflegt den `aktiv`-Index nach. PRIVAT: NUR
    /// [`Store::push_neu`] ruft dies auf, wie in der Deliverable-Vorgabe verlangt ("die EINE
    /// Schreibstelle").
    fn push_geprueft(&mut self, event: Event) -> EventId {
        // Auflage B bzw. der `aktiv`-Check jeder Ableitung lief vorher: ohne `ersetzt` hat das Feld
        // noch kein aktives Event — sonst haette es danach zwei.
        debug_assert!(
            event.ersetzt.is_some() || !self.aktiv.contains_key(&event.feld_id),
            "{} haette zwei aktive Events",
            event.feld_id
        );
        let idx = self.datei.events.len();
        self.aktiv.insert(event.feld_id.clone(), idx);
        let id = event.event_id;
        self.datei.events.push(event);
        id
    }

    /// Faltet den Log-Praefix (bis inkl. `bis_event`, sonst alles) zu `feld_id -> SnapshotFeld` +
    /// `snapshot_id` (`store.py:577-603`, `materialisiere`). Append-only + `ersetzt`-Aufloesung:
    /// ein spaeter ersetztes Event zaehlt nicht.
    ///
    /// # Errors
    /// [`SnapshotFehler::BisEventUnbekannt`], wenn `bis_event` nicht im Log steht.
    ///
    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let neu = |wert: i64, ersetzt: Option<store::EventId>| store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(wert).into(),
    /// #     feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt,
    /// #     ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    /// # };
    /// # let leer = std::collections::HashMap::new();
    /// # let bindung = store::BindungNachschlag::neu(&leer);
    /// use domain::PyWert;
    /// let mut s = store::Store::leer(2025, None);
    /// let erste = s.append(&neu(220, None), None, bindung).unwrap();
    /// s.append(&neu(230, Some(erste)), None, bindung).unwrap();
    /// let (felder, _) = s.materialisiere(None).unwrap();
    /// assert_eq!(felder["ep_arbeitstage"].wert, PyWert::Ganz(230)); // das ersetzte zaehlt nicht
    /// let (bis_erste, _) = s.materialisiere(Some(erste)).unwrap();
    /// assert_eq!(bis_erste["ep_arbeitstage"].wert, PyWert::Ganz(220));
    /// let fremd = store::EventId::aus_bytes([0; 32]);
    /// assert_eq!(
    ///     s.materialisiere(Some(fremd)),
    ///     Err(store::SnapshotFehler::BisEventUnbekannt(fremd))
    /// );
    /// ```
    pub fn materialisiere(
        &self,
        bis_event: Option<EventId>,
    ) -> Result<(BTreeMap<String, SnapshotFeld>, EventId), SnapshotFehler> {
        let praefix: &[Event] = match bis_event {
            None => self.datei.events.as_slice(),
            Some(ziel) => {
                let ende = self
                    .datei
                    .events
                    .iter()
                    .position(|e| e.event_id == ziel)
                    .ok_or(SnapshotFehler::BisEventUnbekannt(ziel))?;
                self.datei.events.get(..=ende).unwrap_or(&[])
            }
        };
        let ersetzt_ids: HashSet<EventId> = praefix.iter().filter_map(|e| e.ersetzt).collect();
        let mut felder = BTreeMap::new();
        for e in praefix {
            if ersetzt_ids.contains(&e.event_id) {
                continue;
            }
            felder.insert(
                e.feld_id.clone(),
                SnapshotFeld {
                    wert: e.wert.clone(),
                    zustand: e.zustand,
                    herkunft: e.herkunft.clone(),
                },
            );
        }
        let sid = snapshot_id(&felder)?;
        Ok((felder, sid))
    }

    /// Materialisiert + baut einen Snapshot-Eintrag, bindet einen optionalen ERiC-Befund an den
    /// Hash (`store.py:606-617`, `erzeuge_snapshot`). Gibt den `snapshot_id` zurueck; der volle
    /// Eintrag ist danach ueber [`Store::snapshots`] erreichbar.
    ///
    /// # Errors
    /// [`SnapshotFehler`], s. [`Store::materialisiere`] und `LeererLogOhneBisEvent`.
    ///
    /// ```
    /// # use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    /// # let neu = |wert: i64, ersetzt: Option<store::EventId>| store::NeuesEvent {
    /// #     feld_id: "ep_arbeitstage".to_string(),
    /// #     wert: serde_json::json!(wert).into(),
    /// #     feldzustand: Feldzustand::Bestaetigt { signal_2: Signal2::new("klick").unwrap() },
    /// #     herkunft: Herkunft {
    /// #         herkunft: Achsenwert::new("mensch").unwrap(),
    /// #         pruef_tiefe: PruefTiefe::Ungeprueft,
    /// #         haftung: Achsenwert::new("nutzer").unwrap(),
    /// #     },
    /// #     schreiber: Schreiber::Mensch("julius".to_string()),
    /// #     signal_1: None,
    /// #     ersetzt,
    /// #     ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    /// # };
    /// # let leer = std::collections::HashMap::new();
    /// # let bindung = store::BindungNachschlag::neu(&leer);
    /// use store::{EricBefundEingabe, EricKlasse, SnapshotFehler};
    /// let mut s = store::Store::leer(2025, None);
    /// assert_eq!(
    ///     s.erzeuge_snapshot(None, None, None),
    ///     Err(SnapshotFehler::LeererLogOhneBisEvent)
    /// );
    /// let id = s.append(&neu(220, None), None, bindung).unwrap();
    /// let befund = EricBefundEingabe {
    ///     rc: 0,
    ///     klasse: EricKlasse::Plausibel,
    ///     gekappt_verdacht: false,
    ///     fehler_anzahl: None,
    /// };
    /// let sid = s.erzeuge_snapshot(None, None, Some(befund)).unwrap();
    /// let snap = &s.snapshots()[0];
    /// assert_eq!((snap.snapshot_id, snap.bis_event), (sid, id));
    /// assert_eq!(snap.eric_befund.as_ref().map(|b| b.gebunden_an), Some(sid));
    /// assert_eq!(s.materialisiere(None).unwrap().1, sid);
    /// ```
    pub fn erzeuge_snapshot(
        &mut self,
        bis_event: Option<EventId>,
        ts: Option<String>,
        eric_befund: Option<EricBefundEingabe>,
    ) -> Result<EventId, SnapshotFehler> {
        let (felder, sid) = self.materialisiere(bis_event)?;
        let letztes = match bis_event {
            Some(id) => id,
            None => self
                .datei
                .events
                .last()
                .map(|e| e.event_id)
                .ok_or(SnapshotFehler::LeererLogOhneBisEvent)?,
        };
        let eric_befund = eric_befund.map(|e| EricBefund {
            gebunden_an: sid,
            rc: e.rc,
            klasse: e.klasse,
            gekappt_verdacht: e.gekappt_verdacht,
            fehler_anzahl: e.fehler_anzahl,
        });
        let snap = Snapshot {
            snapshot_id: sid,
            ts: ts_oder_jetzt(ts),
            bis_event: letztes,
            felder,
            eric_befund,
        };
        self.datei.snapshots.push(snap);
        Ok(sid)
    }
}

fn baue_aktiv_index(events: &[Event]) -> HashMap<String, usize> {
    let ersetzt_ids: HashSet<EventId> = events.iter().filter_map(|e| e.ersetzt).collect();
    let mut aktiv = HashMap::new();
    for (idx, e) in events.iter().enumerate() {
        if ersetzt_ids.contains(&e.event_id) {
            continue;
        }
        aktiv.insert(e.feld_id.clone(), idx);
    }
    aktiv
}

/// `store.py:37-38`, `snapshot_id`: `sha256(canonical_json(felder))`.
///
/// # Errors
/// [`SnapshotFehler::WertNichtDarstellbar`], wenn ein Feldwert nicht nach JSON geht. Der
/// frueher hier stehende `unwrap_or(Value::Null)` waere nach der K2-Umstellung genau die
/// stille Konvertierung, die Auflage 3 verbietet: er ergaebe eine `snapshot_id` ueber `null`.
fn snapshot_id(felder: &BTreeMap<String, SnapshotFeld>) -> Result<EventId, SnapshotFehler> {
    let value = serde_json::to_value(felder)
        .map_err(|e| SnapshotFehler::WertNichtDarstellbar(e.to_string()))?;
    Ok(EventId::von_json(&value))
}

/// Der Schreiber, wie Python ihn fuer die Auflagen A/K1/F2 klassifiziert: nach Praefix
/// (`startswith`, `store.py:111-118` und `:262-308`), nicht nach Gleichheit wie
/// [`Schreiber::from_str`]. `import:beleg2` ist dort ein Beleg-Schreiber und faellt unter A, K1
/// und F2; als `Mensch` entkaeme er allen dreien.
fn python_schreiber(roh: &str) -> Schreiber {
    if roh.starts_with("import:beleg") {
        Schreiber::ImportBeleg
    } else if roh.starts_with("import:vorjahr") {
        Schreiber::ImportVorjahr
    } else if roh.starts_with("import:kontoauszug") {
        Schreiber::ImportKontoauszug
    } else {
        roh.parse()
            .unwrap_or_else(|u: std::convert::Infallible| match u {})
    }
}

/// Auflage T (Typ) + Auflage V (Vorzeichen) + Auflage W (Wertebereich) + Auflage F (Format), `store.py:193-248`, `_pruefe_typ_konformitaet`.
/// Unbekanntes `feld_id`: durchlassen, nicht raten (Team-Lead-Vorgabe).
///
/// Der Wert erscheint in der Meldung als `repr` wie in Python (`repr(wert)`, `{wert!r}`), nicht
/// als JSON-Text: ein Text steht in einfachen Anfuehrungszeichen, `true` als `True`.
fn pruefe_bindung(
    feld_id: &str,
    wert: &PyWert,
    bindung: BindungNachschlag<'_>,
) -> Result<(), Abweisung> {
    let Some(eintrag) = bindung.basis_eintrag(feld_id) else {
        return Ok(());
    };
    let text = match wert {
        PyWert::Text(s) => Some(s.as_str()),
        _ => None,
    };
    let zahl = match domain::Wert::aus_pywert(wert, eintrag.typ, eintrag.enum_werte.as_deref()) {
        Ok(domain::Wert::Cent(n) | domain::Wert::Int(n)) => Some(n),
        Ok(_) => None,
        Err(_) => {
            // Ein Wert mit Steuerzeichen bleibt aus der Meldung (422-detail an Nutzer und Log,
            // PII); der Ersatztext steht wortgleich in `store.py::_pruefe_typ_konformitaet`.
            let steuerzeichen = text.is_some_and(|s| !domain::nur_xml_zeichen(s));
            return Err(Abweisung::TypInkonform {
                feld_id: feld_id.to_string(),
                wert: if steuerzeichen {
                    "[Steuerzeichen im Text, Wert nicht geloggt]".to_string()
                } else {
                    wert.repr()
                },
                typ: eintrag.typ.als_str(),
            });
        }
    };
    // Auflage V (Vorzeichen): `nicht_negativ` heisst, der Schematyp des Kz kennt kein Minus
    // (Vault `decisions/geldfeld-ohne-minus-im-schema-lehnt-minus-bei-eingabe-ab`). Die 0 und
    // Positives gehen durch, Laden prueft nie.
    if let (Some(n), true) = (zahl, eintrag.nicht_negativ) {
        if n < 0 {
            return Err(Abweisung::NegativerBetrag {
                feld_id: feld_id.to_string(),
                wert: n,
            });
        }
    }
    // Auflage W (Wertebereich): nur eine Zahl AUSSERHALB von `bereich`, die nicht 0 ist. Die 0
    // heisst bei diesen Feldern "nichts anzugeben" und bleibt zulaessig, auch unter einem Minimum
    // ueber 0 (Vault `decisions/speichern-lehnt-nullwerte-nicht-ab`). Laden prueft nie.
    if let (Some(n), Some(bereich)) = (zahl, &eintrag.bereich) {
        if n != 0 && !(bereich.min..=bereich.max).contains(&n) {
            return Err(Abweisung::WertAusserhalbBereich {
                feld_id: feld_id.to_string(),
                wert: n,
                min: bereich.min,
                max: bereich.max,
            });
        }
    }
    if let (Some(muster), Some(s)) = (&eintrag.muster, text) {
        if !passt_muster(muster, s) {
            return Err(Abweisung::FormatInkonform {
                feld_id: feld_id.to_string(),
                wert: wert.repr(),
                muster: muster.clone(),
            });
        }
    }
    Ok(())
}

/// `re.fullmatch(muster, wert)` (Python): das `muster` gilt fuer den ganzen Wert, auch wenn es
/// selbst kein `$` traegt. `^(?:muster)$` bildet das in der `regex`-Crate nach (`$` = Textende).
///
/// PARITAET: ein syntaktisch ungueltiges `muster` liesse Pythons `re.fullmatch` mit einer Ausnahme
/// abbrechen (kein abgefangener Auflage-F-Fall dort). Hier fail-closed statt Absturz: ein
/// solcher Wert gilt als nicht passend, nie stillschweigend durchgelassen.
fn passt_muster(muster: &str, wert: &str) -> bool {
    regex::Regex::new(&format!("^(?:{muster})$")).is_ok_and(|re| re.is_match(wert))
}

/// Auflage F2/Magnitude (`store.py:341-358`, nur fuer Vorschlags-Schreiber): `abs(wert) >= 10^10`
/// faengt eine vermutete EUR-statt-Cent-Verwechslung. Akzeptiert Zahl ODER numerischen String
/// (LLM-Antworten liefern oft JSON-Strings); nicht-numerische Strings bleiben unangetastet.
///
/// Der Text liest [`domain::py_float`] wie Pythons `float()` (`1_0000000000`, Nicht-ASCII-Ziffern).
fn pruefe_magnitude(feld_id: &str, wert: &PyWert, schreiber: &Schreiber) -> Result<(), Abweisung> {
    let zahl = match wert {
        PyWert::Text(s) => domain::py_float(s),
        _ => zahl_als_f64(wert),
    };
    if let Some(z) = zahl {
        if z.abs() >= 10_000_000_000.0 {
            return Err(Abweisung::Magnitude {
                feld_id: feld_id.to_string(),
                schreiber: schreiber.to_string(),
                wert: wert.repr(),
            });
        }
    }
    Ok(())
}

/// Python `isinstance(wert, (int, float)) and not isinstance(wert, bool)` als `f64`: dieselbe
/// Menge, die vorher `serde_json::Value::as_f64` durchliess (`Bool`/Text/`null` sind keine Zahl).
#[allow(
    clippy::cast_precision_loss,
    reason = "wie `Value::as_f64`; die Schwellen (`beweist.ab`, 10^10) liegen weit unter 2^53"
)]
fn zahl_als_f64(wert: &PyWert) -> Option<f64> {
    match *wert {
        PyWert::Ganz(n) => Some(n as f64),
        PyWert::GrossGanz(u) => Some(u as f64),
        PyWert::Gleit(f) => Some(f),
        _ => None,
    }
}

/// `{herkunft: "berechnet", pruef_tiefe: ungeprueft, haftung: "nutzer"}` — Herkunft jeder
/// Ableitung (`store.py:438-439,563-564`: "Er hat die Zahl gesagt, nicht diesen Satz — die
/// Haftung bleibt bei ihm, die Urheberschaft nicht.").
///
/// # Panics
/// Nie: "berechnet"/"nutzer" sind nicht-leere Literale (derselbe Beweis wie
/// `domain::meet::achsenwert_bekannt_nicht_leer`).
fn berechnet_herkunft() -> Herkunft {
    #[allow(clippy::unwrap_used)]
    Herkunft {
        herkunft: Achsenwert::new("berechnet").unwrap(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: Achsenwert::new("nutzer").unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::Store;
    use crate::event::NeuesEvent;
    use crate::nachschlag::BindungNachschlag;
    use bindung::Bindung;
    use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
    use proptest::prelude::*;
    use serde_json::json;
    use std::collections::HashMap;

    fn mensch_herkunft() -> Herkunft {
        Herkunft {
            herkunft: Achsenwert::new("mensch").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        }
    }

    fn leere_bindung() -> HashMap<String, &'static Bindung> {
        HashMap::new()
    }

    #[test]
    fn zweites_event_ohne_ersetzt_wird_abgewiesen_auflage_b() {
        let mut store = Store::leer(2025, None);
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        let neu = |wert: i64| NeuesEvent {
            feld_id: "ep_arbeitstage".to_string(),
            wert: json!(wert).into(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: domain::Signal2::new("klick").unwrap(),
            },
            herkunft: mensch_herkunft(),
            schreiber: Schreiber::Mensch("julius".to_string()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        };
        assert!(store.append(&neu(220), None, bindung).is_ok());
        let fehler = store.append(&neu(230), None, bindung).unwrap_err();
        assert!(matches!(
            fehler,
            crate::Abweisung::AktivesEventVorhanden { .. }
        ));
    }

    #[test]
    fn ersetzt_macht_das_neue_event_aktiv() {
        let mut store = Store::leer(2025, None);
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        let neu = |wert: i64, ersetzt: Option<crate::EventId>| NeuesEvent {
            feld_id: "ep_arbeitstage".to_string(),
            wert: json!(wert).into(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick").unwrap(),
            },
            herkunft: mensch_herkunft(),
            schreiber: Schreiber::Mensch("julius".to_string()),
            signal_1: None,
            ersetzt,
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        };
        let erste = store.append(&neu(220, None), None, bindung).unwrap();
        let zweite = store.append(&neu(230, Some(erste)), None, bindung).unwrap();
        assert_eq!(store.aktives("ep_arbeitstage").unwrap().event_id, zweite);
    }

    #[test]
    fn llm_schreiber_ohne_katalog_wird_abgewiesen() {
        let mut store = Store::leer(2025, None);
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        let neu = NeuesEvent {
            feld_id: "ep_arbeitstage".to_string(),
            wert: json!(220).into(),
            feldzustand: Feldzustand::Vorlaeufig,
            herkunft: Herkunft {
                herkunft: Achsenwert::new("llm_vorschlag").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            },
            schreiber: Schreiber::Llm("chat".to_string()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        };
        let fehler = store.append(&neu, None, bindung).unwrap_err();
        assert!(matches!(fehler, crate::Abweisung::KatalogFehlt { .. }));
    }

    #[test]
    fn magnitude_faengt_eur_statt_cent_bei_vorschlag() {
        let mut store = Store::leer(2025, None);
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        let katalog = crate::Katalog::aus_bindungen(std::iter::empty());
        let neu = NeuesEvent {
            feld_id: "kap_zinsen".to_string(),
            wert: json!(12_000_000_000i64).into(),
            feldzustand: Feldzustand::Vorlaeufig,
            herkunft: Herkunft {
                herkunft: Achsenwert::new("berechnet").unwrap(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: Achsenwert::new("nutzer").unwrap(),
            },
            schreiber: Schreiber::Berechnet("maps".to_string()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        };
        // Katalog verweigert schon vorher (leerer Katalog) -- das ist Auflage K1, nicht F2. Der
        // Test dokumentiert die Reihenfolge: K1 kommt zuerst.
        let fehler = store.append(&neu, Some(&katalog), bindung).unwrap_err();
        assert!(matches!(
            fehler,
            crate::Abweisung::KatalogNichtFreigegeben { .. }
        ));
    }

    /// E2 (K2): F2 liest numerischen Text wie Python (`store.py:396-402`: `float(wert)`, bei
    /// `ValueError` durchgelassen). Die Inventur nahm `TypeError` an und riet, die Kulanz zu
    /// entfernen; dann liefe `"12000000000"` an F2 vorbei.
    #[test]
    fn magnitude_liest_numerischen_text_wie_python() {
        let schreiber = Schreiber::Berechnet("maps".to_string());
        let pruefe = |s: &str| {
            super::pruefe_magnitude("kap_zinsen", &domain::PyWert::Text(s.into()), &schreiber)
        };
        for zu_gross in [" 12000000000\n", "-1e10"] {
            assert!(
                matches!(pruefe(zu_gross), Err(crate::Abweisung::Magnitude { .. })),
                "{zu_gross:?}"
            );
        }
        for durch in ["9999999999", "DE89370400440532013000"] {
            assert!(pruefe(durch).is_ok(), "{durch:?}");
        }
    }

    #[test]
    fn text_mit_steuerzeichen_wird_abgewiesen_auflage_t() {
        // Ticket elster-xml-steuerzeichen-im-textwert: echte Bindung, typ=text.
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        let bindungen: Vec<Bindung> = bindung::lade_registry(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let map = crate::baue_nachschlag(&bindungen);
        let mut store = Store::leer(2025, None);
        let neu = NeuesEvent {
            feld_id: "stammdaten_nachname".to_string(),
            wert: json!("Maier\u{0}").into(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick").unwrap(),
            },
            herkunft: mensch_herkunft(),
            schreiber: Schreiber::Mensch("julius".to_string()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        };
        let fehler = store
            .append(&neu, None, BindungNachschlag::neu(&map))
            .unwrap_err();
        assert!(matches!(fehler, crate::Abweisung::TypInkonform { .. }));
        // nennt das Feld, nie den Wert (PII) und nie das Zeichen selbst
        let meldung = fehler.to_string();
        assert!(meldung.contains("stammdaten_nachname"), "{meldung}");
        assert!(
            !meldung.contains("Maier") && !meldung.contains('\u{0}'),
            "{meldung}"
        );
    }

    fn echte_bindungen() -> Vec<Bindung> {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        bindung::lade_registry(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    }

    fn mensch_bestaetigt(feld_id: &str, wert: &str) -> NeuesEvent {
        NeuesEvent {
            feld_id: feld_id.to_string(),
            wert: json!(wert).into(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick").unwrap(),
            },
            herkunft: mensch_herkunft(),
            schreiber: Schreiber::Mensch("julius".to_string()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        }
    }

    #[test]
    fn muster_prueft_den_ganzen_wert_auflage_f() {
        // Decision textfeld-format-aus-xsd-beim-speichern, Punkt 3: das `muster` gilt fuer den
        // ganzen Wert wie Pythons `re.fullmatch`, auch wenn es selbst kein `$` traegt.
        let mut idnr = echte_bindungen()
            .into_iter()
            .find(|b| b.feld_id == "kind_idnr")
            .unwrap();
        idnr.muster = Some("[0-9]{11}".to_string());
        let bindungen = [idnr];
        let map = crate::baue_nachschlag(&bindungen);
        let bindung = BindungNachschlag::neu(&map);
        let gut = mensch_bestaetigt("kind_idnr", "12345678901");
        assert!(Store::leer(2025, None).append(&gut, None, bindung).is_ok());
        for wert in ["12345678901\n", "123456789012"] {
            let neu = mensch_bestaetigt("kind_idnr", wert);
            let fehler = Store::leer(2025, None)
                .append(&neu, None, bindung)
                .unwrap_err();
            assert!(
                matches!(fehler, crate::Abweisung::FormatInkonform { .. }),
                "{wert:?}: {fehler}"
            );
        }
    }

    #[test]
    fn leerer_text_wird_abgewiesen_auflage_t() {
        // Decision textfeld-format-aus-xsd-beim-speichern, Punkt 4: jeder Text-Kz-Typ im Schema
        // verlangt mindestens ein Zeichen.
        let bindungen = echte_bindungen();
        let map = crate::baue_nachschlag(&bindungen);
        let neu = mensch_bestaetigt("stammdaten_nachname", "");
        let fehler = Store::leer(2025, None)
            .append(&neu, None, BindungNachschlag::neu(&map))
            .unwrap_err();
        assert!(
            matches!(fehler, crate::Abweisung::TypInkonform { .. }),
            "{fehler}"
        );
    }

    /// Auflage V (Vault `decisions/geldfeld-ohne-minus-im-schema-lehnt-minus-bei-eingabe-ab`): eine
    /// negative Zahl an einem Feld mit `nicht_negativ` wird abgewiesen; 0 und Positives gehen durch;
    /// ein Verlustfeld (`einkuenfte_gewinn`) nimmt das Minus weiter. Meldung wortgleich zu Python.
    #[test]
    fn negativer_betrag_wird_abgewiesen_die_null_und_verlustfelder_nicht_auflage_v() {
        let bindungen = echte_bindungen();
        let map = crate::baue_nachschlag(&bindungen);
        let bindung = BindungNachschlag::neu(&map);
        let schreibe = |feld: &str, n: i64| {
            let mut neu = mensch_bestaetigt(feld, "");
            neu.wert = json!(n).into();
            Store::leer(2025, None).append(&neu, None, bindung)
        };
        for (feld, n) in [
            ("hh_handwerker_betrag", 0),
            ("hh_handwerker_betrag", 5_000_000),
            ("hh_handwerker_betrag__2", 1),
            ("gewst_messbetrag_partner", 0),
            ("einkuenfte_gewinn", -5_000_000),
            ("einkuenfte_gewinn_partner", -1),
            ("gewinnanteil", -1),
        ] {
            assert!(
                schreibe(feld, n).is_ok(),
                "{feld}={n}: {:?}",
                schreibe(feld, n)
            );
        }
        for (feld, n) in [
            ("hh_handwerker_betrag", -5_000_000),
            ("hh_handwerker_betrag__2", -1),
            ("gewst_messbetrag_partner", -1),
            ("spenden_betrag", -1),
        ] {
            let fehler = schreibe(feld, n).unwrap_err();
            assert!(
                matches!(&fehler, crate::Abweisung::NegativerBetrag { feld_id, wert }
                    if feld_id == feld && *wert == n),
                "{feld}={n}: {fehler}"
            );
        }
        assert_eq!(
            schreibe("hh_handwerker_betrag", -5_000_000).unwrap_err().to_string(),
            "fail-closed (Vorzeichen): hh_handwerker_betrag=-5000000 darf nicht negativ sein — das \
             Feld kennt im amtlichen ELSTER-Schema kein Minus, die Erklärung würde dort abgelehnt."
        );
    }

    /// Auflage W (Vault `decisions/zahl-ausserhalb-des-bereichs-wird-beim-speichern-abgewiesen-
    /// die-null-nicht`): eine Zahl ausserhalb `bereich` wird abgewiesen, die Raender und die 0
    /// (auch unter einem Minimum ueber 0) werden angenommen. Meldung wortgleich zu Python.
    #[test]
    fn wert_ausserhalb_des_bereichs_wird_abgewiesen_die_null_nicht_auflage_w() {
        let bindungen = echte_bindungen();
        let map = crate::baue_nachschlag(&bindungen);
        let bindung = BindungNachschlag::neu(&map);
        let schreibe = |feld: &str, n: i64| {
            let mut neu = mensch_bestaetigt(feld, "");
            neu.wert = json!(n).into();
            Store::leer(2025, None).append(&neu, None, bindung)
        };
        for (feld, n) in [
            ("gewst_hebesatz", 1),
            ("gewst_hebesatz", 999),
            ("gewst_hebesatz", 0),
            ("fam_anzahl_kinder", 0),
            ("fam_anzahl_kinder", 20),
            ("geburtsjahr", 0),
            ("gewst_hebesatz__2", 999),
        ] {
            assert!(
                schreibe(feld, n).is_ok(),
                "{feld}={n}: {:?}",
                schreibe(feld, n)
            );
        }
        for (feld, n, min, max) in [
            ("gewst_hebesatz", -1, 1, 999),
            ("gewst_hebesatz", 1000, 1, 999),
            ("fam_anzahl_kinder", -1, 0, 20),
            ("fam_anzahl_kinder", 21, 0, 20),
            ("geburtsjahr", 1899, 1900, 2010),
            ("gewst_hebesatz__2", 1000, 1, 999),
        ] {
            let fehler = schreibe(feld, n).unwrap_err();
            assert!(
                matches!(&fehler, crate::Abweisung::WertAusserhalbBereich { feld_id, wert, min: a, max: b }
                    if feld_id == feld && *wert == n && *a == min && *b == max),
                "{feld}={n}: {fehler}"
            );
        }
        assert_eq!(
            schreibe("fam_anzahl_kinder", -1).unwrap_err().to_string(),
            "fail-closed (Bereich): fam_anzahl_kinder=-1 liegt ausserhalb des erlaubten Bereichs \
             0 bis 20 der Bindung."
        );
    }

    proptest! {
        /// Deliverable #8: Append-only (Laenge sinkt nie, ein akzeptiertes Event erhoeht sie um
        /// GENAU eins) + hoechstens ein aktives Event je `feld_id`.
        #[test]
        fn append_only_und_hoechstens_ein_aktives_event_je_feld(
            operationen in proptest::collection::vec(
                (prop_oneof![Just("a"), Just("b"), Just("c")], any::<i32>()),
                1..40,
            )
        ) {
            let mut store = Store::leer(2025, None);
            let map = leere_bindung();
            let bindung = BindungNachschlag::neu(&map);
            for (feld_id, wert) in operationen {
                let vor = store.events().len();
                let neu = NeuesEvent {
                    feld_id: feld_id.to_string(),
                    wert: json!(wert).into(),
                    feldzustand: Feldzustand::Vorlaeufig,
                    herkunft: mensch_herkunft(),
                    schreiber: Schreiber::Mensch("julius".to_string()),
                    signal_1: None,
                    ersetzt: None,
                    ts: Some("2026-01-01T00:00:00+00:00".to_string()),
                };
                let ergebnis = store.append(&neu, None, bindung);
                let nach = store.events().len();
                prop_assert!(nach == vor || nach == vor + 1);
                if ergebnis.is_ok() {
                    prop_assert_eq!(nach, vor + 1);
                }
                let ersetzt_ids: std::collections::HashSet<_> =
                    store.events().iter().filter_map(|e| e.ersetzt).collect();
                let mut gesehen = std::collections::HashSet::new();
                for e in store.events() {
                    if ersetzt_ids.contains(&e.event_id) {
                        continue;
                    }
                    prop_assert!(gesehen.insert(e.feld_id.clone()));
                }
            }
        }
    }

    #[test]
    fn ersetzt_ziel_unbekannt_wird_abgewiesen() {
        let mut store = Store::leer(2025, None);
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        let irgendein_id = crate::EventId::von_json(&json!({"nie": "im log"}));
        let neu = NeuesEvent {
            feld_id: "ep_arbeitstage".to_string(),
            wert: json!(220).into(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick").unwrap(),
            },
            herkunft: mensch_herkunft(),
            schreiber: Schreiber::Mensch("julius".to_string()),
            signal_1: None,
            ersetzt: Some(irgendein_id),
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        };
        let fehler = store.append(&neu, None, bindung).unwrap_err();
        assert!(matches!(fehler, crate::Abweisung::ErsetztZielUnbekannt(_)));
    }

    /// K2-Auflage 3: NaN/inf im `wert` (auch verschachtelt) und in `signal_1` weist `append` ab,
    /// statt ein stilles `null` zu schreiben; der Log bleibt leer.
    #[test]
    fn nicht_darstellbarer_wert_wird_abgewiesen_k2_auflage_3() {
        use domain::PyWert;
        let mut store = Store::leer(2025, None);
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        let neu = |wert: PyWert, signal_1: Option<PyWert>| NeuesEvent {
            feld_id: "ep_arbeitstage".to_string(),
            wert,
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick").unwrap(),
            },
            herkunft: mensch_herkunft(),
            schreiber: Schreiber::Mensch("julius".to_string()),
            signal_1,
            ersetzt: None,
            ts: Some("2026-01-01T00:00:00+00:00".to_string()),
        };
        let nan = || PyWert::Gleit(f64::NAN);
        let liste = PyWert::Liste(vec![PyWert::Ganz(1), PyWert::Gleit(f64::INFINITY)]);
        let objekt = PyWert::Objekt(vec![("quell_wert".to_string(), nan())]);
        for fall in [
            neu(nan(), None),
            neu(liste, None),
            neu(PyWert::Ganz(220), Some(objekt)),
        ] {
            let fehler = store.append(&fall, None, bindung).unwrap_err();
            assert!(
                matches!(fehler, crate::Abweisung::WertNichtDarstellbar { .. }),
                "{fehler:?}"
            );
        }
        assert!(store.events().is_empty());
    }

    /// Entscheidung leerer-zeitstempel-heisst-fehlt-und-wird-die-jetzt-zeit: ein leerer `ts` heisst "fehlt"
    /// und wird zur Jetzt-Zeit, wie Pythons `ts or _now()` (`store.py` `append_event`, `erzeuge_snapshot`).
    /// "Jetzt-ISO" heisst: RFC 3339 mit Zone, parsebar, und innerhalb einer Minute der Uhr.
    fn ist_jetzt_iso(ts: &str) -> bool {
        chrono::DateTime::parse_from_rfc3339(ts).is_ok_and(|t| {
            (chrono::Utc::now() - t.with_timezone(&chrono::Utc))
                .num_seconds()
                .abs()
                < 60
        })
    }

    fn event_mit_ts(ts: Option<String>) -> NeuesEvent {
        NeuesEvent {
            feld_id: "ep_arbeitstage".to_string(),
            wert: json!(220).into(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("klick").unwrap(),
            },
            herkunft: mensch_herkunft(),
            schreiber: Schreiber::Mensch("julius".to_string()),
            signal_1: None,
            ersetzt: None,
            ts,
        }
    }

    const FEST: &str = "2026-01-01T00:00:00+00:00";

    #[test]
    fn append_mit_leerem_zeitstempel_stempelt_die_jetzt_zeit() {
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        for ts in [Some(String::new()), None] {
            let mut store = Store::leer(2025, None);
            store
                .append(&event_mit_ts(ts.clone()), None, bindung)
                .unwrap();
            let gestempelt = &store.events()[0].ts;
            assert!(ist_jetzt_iso(gestempelt), "ts={ts:?} -> {gestempelt:?}");
        }
        // Gegenprobe: ein fester Zeitstempel bleibt, wie er ist (sonst bewiese das Gruen oben nichts).
        let mut store = Store::leer(2025, None);
        store
            .append(&event_mit_ts(Some(FEST.to_string())), None, bindung)
            .unwrap();
        assert_eq!(store.events()[0].ts, FEST);
        assert!(!ist_jetzt_iso(FEST));
    }

    #[test]
    fn erzeuge_snapshot_mit_leerem_zeitstempel_stempelt_die_jetzt_zeit() {
        let map = leere_bindung();
        let bindung = BindungNachschlag::neu(&map);
        for ts in [Some(String::new()), None] {
            let mut store = Store::leer(2025, None);
            store
                .append(&event_mit_ts(Some(FEST.to_string())), None, bindung)
                .unwrap();
            store.erzeuge_snapshot(None, ts.clone(), None).unwrap();
            let gestempelt = &store.snapshots()[0].ts;
            assert!(ist_jetzt_iso(gestempelt), "ts={ts:?} -> {gestempelt:?}");
        }
        // Gegenprobe: ein fester Zeitstempel bleibt, wie er ist.
        let mut store = Store::leer(2025, None);
        store
            .append(&event_mit_ts(Some(FEST.to_string())), None, bindung)
            .unwrap();
        store
            .erzeuge_snapshot(None, Some("2026-01-02T00:00:00+00:00".to_string()), None)
            .unwrap();
        assert_eq!(store.snapshots()[0].ts, "2026-01-02T00:00:00+00:00");
    }
}
