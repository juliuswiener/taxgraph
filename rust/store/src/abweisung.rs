//! Die Auflagen A/K1/F2/T/B (`store.py:251-391`) als ein Fehler-Enum statt einzelner
//! `ValueError`-Texte. Jede Variante entspricht genau einem `raise ValueError(...)` im Original;
//! die Nachrichten (`Display`) uebernehmen den Wortlaut, damit ein Fehlertext, der z.B. in einem
//! HTTP-Body landet, sich fuer Julius nicht unangekuendigt aendert.
//!
//! NICHT vertreten: "`zustand=bestaetigt` braucht ein `signal_2`" (`store.py:365f`, Auflage
//! Zwei-Signal). [`domain::Feldzustand::Bestaetigt`] traegt `signal_2` zwingend mit — der
//! Zustand, den diese Pruefung in Python abfaengt, ist in Rust gar nicht erst konstruierbar
//! (s. `crate::event::NeuesEvent`). Der zugehoerige Python-Test
//! (`test_B_bestaetigt_ohne_signal_2_beim_ersetzen`) hat deshalb keine Rust-Entsprechung.
//!
//! EBENFALLS OHNE ENTSPRECHUNG: [`Abweisung::WertNichtDarstellbar`] (K2-Auflage 3, s. dort).
//! `store.py` kennt keine solche Pruefung — sie ist eine Verschaerfung, keine Nachbildung.
use domain::Schreiber;

use crate::canonical::EventId;

/// Eine der Auflagen A/K1/F2/T/B hat ein `append` abgewiesen.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Abweisung {
    /// Auflage A: ein Vorschlags-Schreiber (`llm:`/`import:beleg`/`import:vorjahr`/
    /// `import:kontoauszug`/`berechnet:`) hat sich nicht ehrlich deklariert (falsche
    /// `herkunft`/`zustand`/`signal_2`, `store.py:262-308`).
    #[error(
        "fail-closed (A): {schreiber}-Schreiber muss herkunft={erwartete_herkunft}, \
         zustand=vorlaeufig, signal_2=null tragen — {folge}"
    )]
    AuflageA {
        schreiber: String,
        erwartete_herkunft: &'static str,
        folge: &'static str,
    },

    /// Auflage A (Ersetzt-Guard, `store.py:321-324`): `llm:`/`import:beleg`/`import:kontoauszug`
    /// duerfen nie `ersetzt` tragen.
    #[error(
        "fail-closed (A): {schreiber} darf kein ersetzt tragen — ein Vorschlag ersetzt nie einen \
         bestätigten Wert; die Übernahme läuft über /event mit menschlichem signal_2."
    )]
    AuflageAErsetztGuard { schreiber: String },

    /// Auflage K1 (`store.py:333-335`): ein Vorschlags-Schreiber ohne Katalog.
    #[error("fail-closed (Katalog): Vorschlags-Schreiber {schreiber} braucht katalog=lade_katalog(bindung).")]
    KatalogFehlt { schreiber: String },

    /// Auflage K1 (`store.py:336-339`): `feld_id` ist fuer diesen Vorschlags-Typ nicht
    /// freigegeben.
    #[error(
        "fail-closed (Katalog): {schreiber} darf {feld_id} nicht vorschlagen (human-only oder \
         nicht für Typ '{typ}' freigegeben)."
    )]
    KatalogNichtFreigegeben {
        schreiber: String,
        feld_id: String,
        typ: &'static str,
    },

    /// Auflage F2/Magnitude (`store.py:355-358`): `abs(wert) >= 10^10` bei einem
    /// Vorschlags-Schreiber.
    #[error(
        "fail-closed (F2/Magnitude): {feld_id}={wert} von {schreiber} — vermuteter \
         Einheiten-/Skalierungsfehler (EUR statt Cent)."
    )]
    Magnitude {
        feld_id: String,
        schreiber: String,
        wert: String,
    },

    /// K2-Auflage 3: `wert` oder `signal_1` geht nicht nach JSON — NaN/+-inf, auch innerhalb einer
    /// `Liste`/eines `Objekt`. KEINE Entsprechung in `store.py`; Python haelt NaN an der Tuer auf
    /// (`server.py`, 400) und beim Schreiben (`speichere_fall` mit `allow_nan=False`,
    /// `api.py:159`; die Akte bleibt alt, B4). Hier fail-closed statt eines stillen `null`. Kommt
    /// im Bestand nicht vor (gemessen: 11294/11294 sind bool/int/str), aber eine stille
    /// Konvertierung waere genau die Fehlerklasse, die den `event_id` kostet.
    #[error("fail-closed (Wert): {feld_id}={grund}")]
    WertNichtDarstellbar { feld_id: String, grund: String },

    /// Auflage T (`store.py:228-231`, Stille-Null-Klasse): `wert` passt nicht zum Bindungstyp.
    #[error(
        "fail-closed (Typ): {feld_id}={wert} passt nicht zum Bindungstyp '{typ}' — der Ring läse \
         das sonst still als 0 (Stille-Null-Klasse)."
    )]
    TypInkonform {
        feld_id: String,
        wert: String,
        typ: &'static str,
    },

    /// Auflage F/Format (`store.py:245-248`): `wert` passt nicht auf `bindung.muster`.
    #[error(
        "fail-closed (Format): {feld_id}={wert} passt nicht zum Muster '{muster}' der Bindung — \
         ein formal falscher Wert wird spätestens beim Finanzamt abgelehnt."
    )]
    FormatInkonform {
        feld_id: String,
        wert: String,
        muster: String,
    },

    /// Auflage W/Wertebereich (`store.py::_pruefe_typ_konformitaet`, Vault
    /// `decisions/zahl-ausserhalb-des-bereichs-wird-beim-speichern-abgewiesen-die-null-nicht`):
    /// eine Zahl ausserhalb von `bindung.bereich` `min..=max`, die nicht 0 ist. Die 0 bleibt
    /// zulaessig, auch unter einem Minimum ueber 0 (sie heisst "nichts anzugeben"). Die Zahl steht
    /// in der Meldung (kein PII, anders als ein Text).
    #[error(
        "fail-closed (Bereich): {feld_id}={wert} liegt ausserhalb des erlaubten Bereichs \
         {min} bis {max} der Bindung."
    )]
    WertAusserhalbBereich {
        feld_id: String,
        wert: i64,
        min: i64,
        max: i64,
    },

    /// Auflage B (`store.py:371-373`): `feld_id` hat schon ein aktives Event, `ersetzt` fehlt.
    #[error("fail-closed (B): {feld_id} hat schon ein aktives Event; Überschreiben braucht ersetzt={aktives_event}.")]
    AktivesEventVorhanden {
        feld_id: String,
        aktives_event: EventId,
    },

    /// Auflage B (`store.py:377`): das `ersetzt`-Ziel existiert nicht im Log.
    #[error("fail-closed (B): ersetzt-Ziel {0} existiert nicht.")]
    ErsetztZielUnbekannt(EventId),

    /// Auflage B (`store.py:378-379`): das `ersetzt`-Ziel gehoert zu einem anderen `feld_id`.
    #[error("fail-closed (B): ersetzt-Ziel gehört zu anderem feld_id.")]
    ErsetztFeldMismatch,

    /// Auflage B (`store.py:380-381`): das `ersetzt`-Ziel ist bereits selbst ersetzt.
    #[error("fail-closed (B): ersetzt-Ziel ist bereits ersetzt.")]
    ErsetztBereitsErsetzt,
}

/// Die erwartete `herkunft`+Erklaerungssatz je Vorschlags-Schreiber (Auflage A,
/// `store.py:262-308`). `None`, wenn dieser Schreiber nicht Auflage-A-pflichtig ist
/// (`import:elster`, `engine`, `abgeleitet:*`, ein Mensch).
#[must_use]
pub fn auflage_a_erwartung(schreiber: &Schreiber) -> Option<(&'static str, &'static str)> {
    match schreiber {
        Schreiber::Llm(_) => Some(("llm_vorschlag", "kein Bestätigen durch die KI.")),
        Schreiber::ImportBeleg => Some(("beleg_import", "ein Beleg-Import bestätigt nie direkt.")),
        Schreiber::ImportVorjahr => {
            Some(("vorjahr", "eine Vorjahres-Übernahme bestätigt nie direkt."))
        }
        Schreiber::ImportKontoauszug => Some((
            "kontoauszug",
            "eine Kontoauszug-Klassifikation bestätigt nie direkt.",
        )),
        Schreiber::Berechnet(_) => Some((
            "berechnet",
            "ein berechneter/abgeleiteter Vorschlag bestätigt nie direkt.",
        )),
        _ => None,
    }
}

/// Auflage A, Ersetzt-Guard (`store.py:321`): `llm:`/`import:beleg`/`import:kontoauszug` duerfen
/// nie `ersetzt` tragen. `berechnet:` ist EXEMPT (`api.py entfernung()` uebergibt dort bewusst
/// `ersetzt=<aktives Event>`, s. Kommentar in `store.py:317-320`).
#[must_use]
pub fn ersetzt_gesperrt(schreiber: &Schreiber) -> bool {
    matches!(
        schreiber,
        Schreiber::Llm(_) | Schreiber::ImportBeleg | Schreiber::ImportKontoauszug
    )
}

#[cfg(test)]
mod tests {
    use super::{auflage_a_erwartung, ersetzt_gesperrt};

    #[test]
    fn auflage_a_erwartung_matcht_store_py() {
        assert_eq!(
            auflage_a_erwartung(&"llm:chat".parse().unwrap()).map(|(h, _)| h),
            Some("llm_vorschlag")
        );
        assert_eq!(
            auflage_a_erwartung(&"berechnet:maps".parse().unwrap()).map(|(h, _)| h),
            Some("berechnet")
        );
        assert_eq!(auflage_a_erwartung(&"import:elster".parse().unwrap()), None);
        assert_eq!(auflage_a_erwartung(&"engine".parse().unwrap()), None);
        assert_eq!(auflage_a_erwartung(&"julius".parse().unwrap()), None);
    }

    #[test]
    fn ersetzt_guard_ist_berechnet_exempt() {
        assert!(ersetzt_gesperrt(&"llm:chat".parse().unwrap()));
        assert!(ersetzt_gesperrt(&"import:beleg".parse().unwrap()));
        assert!(ersetzt_gesperrt(&"import:kontoauszug".parse().unwrap()));
        assert!(!ersetzt_gesperrt(&"berechnet:maps".parse().unwrap()));
        assert!(!ersetzt_gesperrt(&"import:vorjahr".parse().unwrap()));
    }
}
