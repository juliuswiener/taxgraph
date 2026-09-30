//! Der Sachverhalts-Store: append-only Event-Log + Ableitungen + Materialisierung
//! (`produkt/store/store.py`). [`Store::append`] ist DER EINE Schreibpfad — die Ableitungen
//! [`Store::leite_ab`]/[`Store::rechne_ab`] schreiben nur ueber die private [`Store::push_neu`],
//! keine zweite Implementierung.
use std::collections::{BTreeMap, HashMap, HashSet};

use domain::{Achsenwert, Feldzustand, Herkunft, HerkunftVektor, PruefTiefe, Schreiber, Zustand};
use serde::{Deserialize, Serialize};

use crate::ableitung;
use crate::abweisung::{self, Abweisung};
use crate::canonical::EventId;
use crate::event::{Event, NeuesEvent, Signal};
use crate::katalog::Katalog;
use crate::nachschlag::BindungNachschlag;
use crate::zeit::jetzt_iso;

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
    /// `serde_json::Value` statt vorab auf die heutige eine Unterstruktur festgelegt. Gemessen:
    /// 0 der 192 realen Fallakten tragen sie (kein bisher verknuepfter Vorjahres-Fall im Bestand);
    /// Python KANN sie trotzdem schreiben — bisher in `StoreDatei` gefehlt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vorjahr_referenz: Option<serde_json::Value>,
}

/// Das `veranlagungszeitraum`-Feld einer Store-Datei — RAW wie Python (`int(...)`, `store.py:79`),
/// keine Bereichspruefung beim Laden. Gemessen ueber alle 192 realen Fallakten unter
/// `~/.local/share/taxgraph/faelle/` (Zaehlung, kein Wert ausserhalb dieses Kommentars
/// festgehalten): neben sinnvollen Werten und Ausreissern wie `-5`/`2099` (beide passen in
/// `i64`) traegt EINE Datei einen 40-stelligen Wert (`99999999999999999999999999999999999999`),
/// der `i64` ueberlaeuft — genau der Grund, warum `store::lade` diese Datei bisher ablehnte.
///
/// Eine ZWEITE Datei traegt den Wert als JSON-Zahl in Exponentialschreibweise (Mantisse +
/// `e`/`E`-Exponent, kein Wert hier festgehalten). `serde_yaml_ng` (der Parser von
/// `persistenz::lade`, s. o.) loest das schon als
/// Ganzzahl auf — `store::lade` laedt diese Datei bereits fehlerfrei. `serde_json` (von
/// Parity-Tests fuer den rohen Python-Vergleich genutzt) liest denselben Token dagegen als
/// `f64` und scheiterte bisher an `i128`s Standard-`Deserialize` (kein `visit_f64`). Die
/// [`Deserialize`]-Impl unten faengt beide Zahl-Formen ab (`i128`-Varianten direkt, `f64` durch
/// Abschneiden Richtung Null wie Pythons `int(float)`) — deserializer-unabhaengig, wie Pythons
/// eigene `int(...)`-Koerzion an den Verwendungsstellen.
///
/// ponytail: `i128` statt Pythons echtem, beliebig grossem `int` — deckt jeden gemessenen Wert
/// (auch den 40-stelligen) mit weitem Rand; ein noch groesserer Wert wird beim Laden zu einem
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
        // Derselbe getypte Hint wie die vorherige derive-Deserialize (i128::deserialize ruft
        // intern deserialize_i128 auf) — die bereits bewiesene YAML-Ladung bleibt unveraendert,
        // nur `visit_f64` kommt als Fallback hinzu.
        deserializer.deserialize_i128(BesucherJahr)
    }
}

impl Veranlagungsjahr {
    /// Fuer Aufrufer, die den (bewusst engeren) `i64`-Bereich brauchen — [`Store::leer`] und die
    /// Datums-Ableitung ([`ableitung::berechne`], `vz: i64`). Saettigt an den `i64`-Grenzen statt
    /// zu ueberlaufen: betrifft praktisch nur die eine gemessene 40-stellige Datei, deren Wert
    /// fuer jede reale Ableitungsregel ohnehin weit ausserhalb jeder sinnvollen Jahresspanne
    /// liegt — ein gesaettigtes `i64::MAX` fuehrt zu denselben "viel zu weit in der Zukunft"-
    /// Vergleichsergebnissen wie Pythons unbeschraenkte Arithmetik mit dem echten 40-stelligen
    /// Wert.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotFeld {
    pub wert: serde_json::Value,
    pub zustand: Zustand,
    pub herkunft: HerkunftVektor,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SnapshotFehler {
    /// `store.py:591`: `bis_event` steht nicht im Log.
    #[error("bis_event {0} nicht im Log")]
    BisEventUnbekannt(EventId),
    /// `store.py:610`: `store["events"][-1]` auf leerem Log ist in Python ein ungefangener
    /// `IndexError`. Hier ein expliziter Fehler statt eines Absturzes (fail-closed statt Panik).
    #[error("erzeuge_snapshot ohne bis_event auf einem leeren Log")]
    LeererLogOhneBisEvent,
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
    #[must_use]
    pub fn aus_datei(datei: StoreDatei) -> Self {
        let aktiv = baue_aktiv_index(&datei.events);
        Self { datei, aktiv }
    }

    #[must_use]
    pub fn datei(&self) -> &StoreDatei {
        &self.datei
    }

    #[must_use]
    pub fn into_datei(self) -> StoreDatei {
        self.datei
    }

    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.datei.events
    }

    #[must_use]
    pub fn snapshots(&self) -> &[Snapshot] {
        &self.datei.snapshots
    }

    #[must_use]
    pub fn veranlagungszeitraum(&self) -> i64 {
        self.datei.veranlagungszeitraum.als_i64_saettigend()
    }

    /// Das aktuell aktive Event fuer `feld_id`, wenn eines existiert (`store.py:92-100`,
    /// `_aktives`, hier direkt ueber den gepflegten Index statt per Voll-Scan).
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

    /// Prueft die Auflagen A/K1/F2/T/F/B und haengt bei Erfolg ein Event an (`store.py:251-390`,
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
    pub fn append(
        &mut self,
        neu: &NeuesEvent,
        katalog: Option<&Katalog>,
        bindung: BindungNachschlag<'_>,
    ) -> Result<EventId, Abweisung> {
        Self::pruefe_auflage_a(neu)?;
        if let Some(typ) = neu.schreiber.vorschlag_typ() {
            let katalog = katalog.ok_or_else(|| Abweisung::KatalogFehlt {
                schreiber: neu.schreiber.to_string(),
            })?;
            if !katalog.erlaubt(typ, &neu.feld_id) {
                return Err(Abweisung::KatalogNichtFreigegeben {
                    schreiber: neu.schreiber.to_string(),
                    feld_id: neu.feld_id.clone(),
                    typ,
                });
            }
            pruefe_magnitude(&neu.feld_id, &neu.wert, &neu.schreiber)?;
        }
        pruefe_bindung(&neu.feld_id, &neu.wert, bindung)?;
        self.pruefe_auflage_b(neu)?;

        let signal =
            // Schreibpfad: Schluessel ist immer da (Python `store.py`: `signal or {"signal_1":
            // None, ...}`) -- aeussere Ebene daher immer `Some(...)` (s. `Signal`-Typdoku).
            Signal { signal_1: Some(neu.signal_1.clone()), signal_2: neu.signal_2_roh().map(str::to_owned) };
        let event = Event {
            event_id: EventId::aus_bytes([0; 32]),
            ts: neu.ts.clone().unwrap_or_else(jetzt_iso),
            feld_id: neu.feld_id.clone(),
            wert: neu.wert.clone(),
            zustand: neu.zustand(),
            herkunft: neu.herkunft.clone().into(),
            schreiber: neu.schreiber.clone(),
            signal: Some(signal),
            ersetzt: neu.ersetzt,
        };
        let event_id = self.push_neu(event);

        let vz = self.datei.veranlagungszeitraum.als_i64_saettigend();
        self.leite_ab(&neu.feld_id, &neu.wert, neu.zustand(), bindung);
        self.rechne_ab(&neu.feld_id, &neu.wert, neu.zustand(), bindung, vz);
        Ok(event_id)
    }

    /// Auflage A (`store.py:262-324`): ein Vorschlags-Schreiber deklariert sich ehrlich
    /// (`herkunft`/`zustand`/`signal_2`) und darf (ausser `berechnet:`) nie `ersetzt` tragen.
    fn pruefe_auflage_a(neu: &NeuesEvent) -> Result<(), Abweisung> {
        let Some((erwartete_herkunft, folge)) = abweisung::auflage_a_erwartung(&neu.schreiber)
        else {
            return Ok(());
        };
        // `zustand != vorlaeufig` UND `signal_2 is_some()` sind in Python zwei getrennte Checks;
        // in Rust sind sie durch `Feldzustand` strukturell gekoppelt (nur `Vorlaeufig` traegt kein
        // `Signal2`) -- ein Check deckt beide ab.
        let ehrlich = neu.herkunft.herkunft.as_str() == erwartete_herkunft
            && matches!(neu.feldzustand, Feldzustand::Vorlaeufig);
        if !ehrlich {
            return Err(Abweisung::AuflageA {
                schreiber: neu.schreiber.to_string(),
                erwartete_herkunft,
                folge,
            });
        }
        if abweisung::ersetzt_gesperrt(&neu.schreiber) && neu.ersetzt.is_some() {
            return Err(Abweisung::AuflageAErsetztGuard {
                schreiber: neu.schreiber.to_string(),
            });
        }
        Ok(())
    }

    /// Auflage B (`store.py:368-381`): hoechstens ein aktives Event je `feld_id`; Ueberschreiben
    /// nur ueber ein gueltiges `ersetzt`-Ziel.
    fn pruefe_auflage_b(&self, neu: &NeuesEvent) -> Result<(), Abweisung> {
        match neu.ersetzt {
            None => {
                // Invariante: jeder Index in `aktiv` zeigt auf ein vorhandenes Event (einziger
                // Schreibpfad ist `push_geprueft`, das Index und Vec gemeinsam pflegt). Ein
                // fehlender Treffer hier ist strukturell unerreichbar; `and_then` behandelt ihn
                // fail-open als "kein aktives Event" statt zu paniken.
                if let Some(aktives_event) = self
                    .aktiv
                    .get(&neu.feld_id)
                    .and_then(|&idx| self.datei.events.get(idx))
                    .map(|e| e.event_id)
                {
                    return Err(Abweisung::AktivesEventVorhanden {
                        feld_id: neu.feld_id.clone(),
                        aktives_event,
                    });
                }
                Ok(())
            }
            Some(ziel) => {
                let Some(ziel_event) = self.datei.events.iter().find(|e| e.event_id == ziel) else {
                    return Err(Abweisung::ErsetztZielUnbekannt(ziel));
                };
                if ziel_event.feld_id != neu.feld_id {
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
    fn leite_ab(
        &mut self,
        feld_id: &str,
        wert: &serde_json::Value,
        zustand: Zustand,
        bindung: BindungNachschlag<'_>,
    ) {
        if zustand != Zustand::Bestaetigt {
            return;
        }
        let Some(eintrag) = bindung.get(feld_id) else {
            return;
        };
        let Some(regel) = &eintrag.beweist else {
            return;
        };
        let Some(zahl) = wert.as_f64() else { return };
        if zahl < regel.ab.unwrap_or(1.0) {
            return;
        }
        if bindung.get(&regel.feld_id).is_none() || self.aktiv.contains_key(&regel.feld_id) {
            return;
        }
        let event = Event {
            event_id: EventId::aus_bytes([0; 32]),
            ts: jetzt_iso(),
            feld_id: regel.feld_id.clone(),
            wert: regel.wert.clone(),
            zustand: Zustand::Bestaetigt,
            herkunft: berechnet_herkunft().into(),
            schreiber: Schreiber::Abgeleitet("beweist".to_string()),
            signal: Some(Signal {
                signal_1: Some(None),
                signal_2: Some(format!("beweist@{feld_id}={wert}")),
            }),
            ersetzt: None,
        };
        self.push_neu(event);
    }

    /// `store.py:503-572`, `_rechne_ab`: Felder mit `ableitung`-Regel, deren Quelle ODER
    /// `und_feld` gerade bestaetigt wurde, werden berechnet (ZWEI Ausloeser). Liest `aktiv`
    /// EINMAL vor der Schleife (`store.py:529-531`: "keine Kette").
    fn rechne_ab(
        &mut self,
        feld_id: &str,
        wert: &serde_json::Value,
        zustand: Zustand,
        bindung: BindungNachschlag<'_>,
        vz: i64,
    ) {
        if zustand != Zustand::Bestaetigt {
            return;
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
            let quellwert: serde_json::Value = if regel.aus == feld_id {
                wert.clone()
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
                qev.wert.clone()
            };
            if let Some(und_feld) = und {
                let Some(ev) = aktiv_snapshot
                    .get(und_feld)
                    .and_then(|&i| self.datei.events.get(i))
                else {
                    continue;
                };
                let leer = ev.wert.is_null()
                    || ev.wert == serde_json::json!("")
                    || ev.wert == serde_json::json!(false);
                if ev.zustand != Zustand::Bestaetigt || leer {
                    continue;
                }
            }
            let Some(neuer_wert) = ableitung::berechne(regel, &quellwert, vz) else {
                continue;
            };
            neue_events.push(Event {
                event_id: EventId::aus_bytes([0; 32]),
                ts: jetzt_iso(),
                feld_id: ziel.to_string(),
                wert: neuer_wert,
                zustand: Zustand::Bestaetigt,
                herkunft: berechnet_herkunft().into(),
                schreiber: Schreiber::Abgeleitet("ableitung".to_string()),
                signal: Some(Signal {
                    signal_1: Some(None),
                    signal_2: Some(format!("ableitung@{}", regel.aus)),
                }),
                ersetzt: None,
            });
        }
        for event in neue_events {
            self.push_neu(event);
        }
    }

    /// Berechnet den `event_id` und haengt an (Ableitungen und `append` teilen sich diesen einen
    /// Pfad, s. Modul-Dokumentation).
    fn push_neu(&mut self, mut event: Event) -> EventId {
        event.event_id = event.berechne_event_id();
        self.push_geprueft(event)
    }

    /// Haengt ein bereits geprueftes Event an und pflegt den `aktiv`-Index nach. PRIVAT: NUR
    /// [`Store::push_neu`] ruft dies auf, wie in der Deliverable-Vorgabe verlangt ("die EINE
    /// Schreibstelle").
    fn push_geprueft(&mut self, event: Event) -> EventId {
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
        let sid = snapshot_id(&felder);
        Ok((felder, sid))
    }

    /// Materialisiert + baut einen Snapshot-Eintrag, bindet einen optionalen ERiC-Befund an den
    /// Hash (`store.py:606-617`, `erzeuge_snapshot`). Gibt den `snapshot_id` zurueck; der volle
    /// Eintrag ist danach ueber [`Store::snapshots`] erreichbar.
    ///
    /// # Errors
    /// [`SnapshotFehler`], s. [`Store::materialisiere`] und `LeererLogOhneBisEvent`.
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
            ts: ts.unwrap_or_else(jetzt_iso),
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
fn snapshot_id(felder: &BTreeMap<String, SnapshotFeld>) -> EventId {
    // Kann nur an derselben (unerreichbaren) NaN-Float-Invariante scheitern wie `canonical_json`
    // (s. `canonical.rs`-Moduldoku) -- `Value::Null` als total harmloser, nie erreichter Fallback.
    let value = serde_json::to_value(felder).unwrap_or(serde_json::Value::Null);
    EventId::von_json(&value)
}

/// Auflage T (Typ) + Auflage F (Format), `store.py:193-248`, `_pruefe_typ_konformitaet`.
/// Unbekanntes `feld_id`: durchlassen, nicht raten (Team-Lead-Vorgabe).
fn pruefe_bindung(
    feld_id: &str,
    wert: &serde_json::Value,
    bindung: BindungNachschlag<'_>,
) -> Result<(), Abweisung> {
    let Some(eintrag) = bindung.basis_eintrag(feld_id) else {
        return Ok(());
    };
    if domain::Wert::aus_json(wert, eintrag.typ, eintrag.enum_werte.as_deref()).is_err() {
        // Ein Wert mit Steuerzeichen bleibt aus der Meldung (422-detail an Nutzer und Log, PII);
        // der Ersatztext steht wortgleich in `store.py::_pruefe_typ_konformitaet`.
        let steuerzeichen = wert.as_str().is_some_and(|s| !domain::nur_xml_zeichen(s));
        return Err(Abweisung::TypInkonform {
            feld_id: feld_id.to_string(),
            wert: if steuerzeichen {
                "[Steuerzeichen im Text, Wert nicht geloggt]".to_string()
            } else {
                wert.to_string()
            },
            typ: eintrag.typ.als_str(),
        });
    }
    if let Some(muster) = &eintrag.muster {
        if let Some(s) = wert.as_str() {
            if !passt_muster(muster, s) {
                return Err(Abweisung::FormatInkonform {
                    feld_id: feld_id.to_string(),
                    wert: wert.to_string(),
                    muster: muster.clone(),
                });
            }
        }
    }
    Ok(())
}

/// `re.match(muster, wert)` (Python) matcht ab Position 0, nicht zwingend bis Stringende —
/// `^(?:muster)` bildet das in der `regex`-Crate nach. Alle echten `muster`-Werte im Repo sind
/// bereits selbst mit `^` verankert (`grep muster: produkt/bindung/*.yaml`, Stand 2026-09-29).
///
/// PARITAET: ein syntaktisch ungueltiges `muster` liesse Pythons `re.match` mit einer Ausnahme
/// abbrechen (kein abgefangener Auflage-F-Fall dort). Hier fail-closed statt Absturz: ein
/// solcher Wert gilt als nicht passend, nie stillschweigend durchgelassen.
fn passt_muster(muster: &str, wert: &str) -> bool {
    regex::Regex::new(&format!("^(?:{muster})")).is_ok_and(|re| re.is_match(wert))
}

/// Auflage F2/Magnitude (`store.py:341-358`, nur fuer Vorschlags-Schreiber): `abs(wert) >= 10^10`
/// faengt eine vermutete EUR-statt-Cent-Verwechslung. Akzeptiert Zahl ODER numerischen String
/// (LLM-Antworten liefern oft JSON-Strings); nicht-numerische Strings bleiben unangetastet.
fn pruefe_magnitude(
    feld_id: &str,
    wert: &serde_json::Value,
    schreiber: &Schreiber,
) -> Result<(), Abweisung> {
    let zahl = match wert {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    };
    if let Some(z) = zahl {
        if z.abs() >= 10_000_000_000.0 {
            return Err(Abweisung::Magnitude {
                feld_id: feld_id.to_string(),
                schreiber: schreiber.to_string(),
                wert: wert.to_string(),
            });
        }
    }
    Ok(())
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
            wert: json!(wert),
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
            wert: json!(wert),
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
            wert: json!(220),
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
            wert: json!(12_000_000_000i64),
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
            wert: json!("Maier\u{0}"),
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
                    wert: json!(wert),
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
            wert: json!(220),
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
}
