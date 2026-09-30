//! Die zwei Schreibwege der Importer, als Typen getrennt.
//!
//! [`VorschlagEvent`] (Kontoauszug, Beleg, Vorjahr) hat KEINEN Zustands-Parameter: es wird
//! immer `vorlaeufig` ohne `signal_2` — ein Import bestaetigt nie (Auflage A, `store.py:262-308`).
//! [`EdatenEvent`] ist der eine Importer, der bestaetigt schreibt: eDaten der Finanzverwaltung
//! „gelten als Angaben des Steuerpflichtigen" (§ 150 Abs. 7 S. 2 AO), mit festem `signal_2`
//! `edaten_uebermittelt` und Herkunft `edaten`/`amtlich`/`amt`. Beide gehen durch
//! [`store::Store::append`], den einzigen Schreibpfad.
use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, Schreiber, Signal2};
use serde_json::Value;
use store::{Abweisung, BindungNachschlag, EventId, Katalog, NeuesEvent, Store};

/// Wer vorschlaegt — legt Herkunft und Schreiber fest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quelle {
    Kontoauszug,
    Beleg,
    Vorjahr,
}

impl Quelle {
    fn herkunft_wort(self) -> &'static str {
        match self {
            Self::Kontoauszug => "kontoauszug",
            Self::Beleg => "beleg_import",
            Self::Vorjahr => "vorjahr",
        }
    }

    fn schreiber(self) -> Schreiber {
        match self {
            Self::Kontoauszug => Schreiber::ImportKontoauszug,
            Self::Beleg => Schreiber::ImportBeleg,
            Self::Vorjahr => Schreiber::ImportVorjahr,
        }
    }
}

/// Fehler beim Schreiben eines Import-Events.
#[derive(Debug, thiserror::Error)]
pub enum SchreibFehler {
    /// Der Store hat das Event abgewiesen (Python: `ValueError` aus `append_event`).
    #[error(transparent)]
    Abweisung(#[from] Abweisung),
    /// Unerreichbar: eine der festen Achsen/Signale war leer.
    #[error("interne Konstante leer")]
    Konstante,
}

fn achse(s: &str) -> Result<Achsenwert, SchreibFehler> {
    Achsenwert::new(s).map_err(|_| SchreibFehler::Konstante)
}

/// Ein vorlaeufiger Import-Vorschlag.
#[derive(Debug, Clone, PartialEq)]
pub struct VorschlagEvent {
    pub quelle: Quelle,
    pub feld_id: String,
    pub wert: Value,
    pub signal_1: Value,
}

impl VorschlagEvent {
    /// Ueber den Store schreiben: `vorlaeufig`, Herkunft `<quelle>`/`ungeprueft`/`nutzer`.
    ///
    /// # Errors
    /// [`SchreibFehler::Abweisung`], wenn eine Store-Auflage greift.
    ///
    /// ```
    /// use eingang::vorschlag::{Quelle, VorschlagEvent};
    /// use store::{BindungNachschlag, Store};
    /// let leer = std::collections::HashMap::new();
    /// let mut store = Store::leer(2025, None);
    /// let ev = VorschlagEvent { quelle: Quelle::Vorjahr, feld_id: "unbekannt".into(), wert: serde_json::json!(1), signal_1: serde_json::json!({"typ": "vorjahr"}) };
    /// ev.schreibe(&mut store, None, BindungNachschlag::neu(&leer), None).unwrap(); // landet als vorlaeufiges Event
    /// assert_eq!(store.aktive().count(), 1);
    /// ```
    pub fn schreibe(
        &self,
        store: &mut Store,
        katalog: Option<&Katalog>,
        bindung: BindungNachschlag<'_>,
        ts: Option<&str>,
    ) -> Result<EventId, SchreibFehler> {
        let herkunft = Herkunft {
            herkunft: achse(self.quelle.herkunft_wort())?,
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: achse("nutzer")?,
        };
        let neu = NeuesEvent {
            feld_id: self.feld_id.clone(),
            wert: self.wert.clone(),
            feldzustand: Feldzustand::Vorlaeufig,
            // `.into()` bewusst: `NeuesEvent::herkunft` kann auf `HerkunftVektor` wechseln
            // (`From<Herkunft>`), dann bleibt diese Stelle unveraendert gueltig.
            #[allow(clippy::useless_conversion)]
            herkunft: herkunft.into(),
            schreiber: self.quelle.schreiber(),
            signal_1: Some(self.signal_1.clone()),
            ersetzt: None,
            ts: ts.map(str::to_owned),
        };
        Ok(store.append(&neu, katalog, bindung)?)
    }
}

/// Ein bestaetigtes eDaten-Event (§ 150 Abs. 7 S. 2 AO).
#[derive(Debug, Clone, PartialEq)]
pub struct EdatenEvent {
    pub feld_id: String,
    pub wert: Value,
    pub signal_1: Value,
}

impl EdatenEvent {
    /// Ueber den Store schreiben: `bestaetigt`, `signal_2 = "edaten_uebermittelt"`, Herkunft
    /// `edaten`/`amtlich`/`amt`, Schreiber `import:elster`.
    ///
    /// # Errors
    /// [`SchreibFehler::Abweisung`].
    ///
    /// ```
    /// use eingang::vorschlag::EdatenEvent;
    /// use store::{BindungNachschlag, Store};
    /// let leer = std::collections::HashMap::new();
    /// let mut store = Store::leer(2025, None);
    /// let ev = EdatenEvent { feld_id: "unbekannt".into(), wert: serde_json::json!(1), signal_1: serde_json::json!({"typ": "edaten"}) };
    /// // Ohne Bindung ist die Typpruefung offen; der Store nimmt das bestaetigte Event an.
    /// assert!(ev.schreibe(&mut store, BindungNachschlag::neu(&leer), None).is_ok());
    /// ```
    pub fn schreibe(
        &self,
        store: &mut Store,
        bindung: BindungNachschlag<'_>,
        ts: Option<&str>,
    ) -> Result<EventId, SchreibFehler> {
        let herkunft = Herkunft {
            herkunft: achse("edaten")?,
            pruef_tiefe: PruefTiefe::Amtlich,
            haftung: achse("amt")?,
        };
        let neu = NeuesEvent {
            feld_id: self.feld_id.clone(),
            wert: self.wert.clone(),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: Signal2::new("edaten_uebermittelt").map_err(|_| SchreibFehler::Konstante)?,
            },
            // `.into()` bewusst: `NeuesEvent::herkunft` kann auf `HerkunftVektor` wechseln
            // (`From<Herkunft>`), dann bleibt diese Stelle unveraendert gueltig.
            #[allow(clippy::useless_conversion)]
            herkunft: herkunft.into(),
            schreiber: Schreiber::ImportElster,
            signal_1: Some(self.signal_1.clone()),
            ersetzt: None,
            ts: ts.map(str::to_owned),
        };
        Ok(store.append(&neu, None, bindung)?)
    }
}
