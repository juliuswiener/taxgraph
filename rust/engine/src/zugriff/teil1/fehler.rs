//! Fehler der Accessoren aus `teil1`. Python-Seite: jede `catala_*`-Funktion wirft entweder eine
//! Catala-Laufzeitausnahme (`catala_runtime.CatalaError` und Unterklassen) oder einen
//! Eingabefehler (`KeyError`/`ValueError`/`TypeError`/`FileNotFoundError`) beim Lesen des rohen
//! Dicts. Letztere sind hier unrepraesentierbar: die Eingabe-Structs sind typisiert, ein fehlender
//! Pflichtschluessel oder ein VZ ausserhalb 2024-2026 entsteht erst gar nicht.
use bindung::ParamsWertFehler;
use catala_sys::CatalaFehler;

use crate::altersentlastungsbetrag::AltersentlastungsbetragFehler;
use crate::dezimal::DezimalUeberlauf;
use crate::entfernungspauschale::EntfernungspauschaleFehler;
use crate::verbilligte_vermietung::VerbilligteVermietungFehler;

/// Fehler eines `teil1`-Accessors.
#[derive(Debug, thiserror::Error)]
pub enum EngineFehler {
    /// Catala-Scope verletzt eine Laufzeit-Assertion. Python: eine `CatalaError`-Unterklasse
    /// (`AssertionFailed`, `NoValue`, `Conflict`, `DivisionByZero`, ...). Der C-Shim meldet nur
    /// "fehlgeschlagen", nicht welche -- die Zuordnung ist 1:Familie.
    #[error(transparent)]
    Catala(#[from] CatalaFehler),
    /// Ein Dezimalwert passt nicht in den Zaehler/Nenner-Bereich von `catala_new_frac`. Python
    /// rechnet mit beliebig grossen Bruechen; kein Gegenstueck.
    #[error(transparent)]
    Dezimal(#[from] DezimalUeberlauf),
    /// Parameterdatei unlesbar oder Schluessel fehlt. Python: `KeyError`/`FileNotFoundError`
    /// beim Lesen von `params/`; mit typisiertem VZ nur bei beschaedigtem Repo erreichbar.
    #[error(transparent)]
    Params(#[from] ParamsWertFehler),
    /// Ganzzahl-Ueberlauf in `i64`. Python rechnet mit beliebig grossen Ganzzahlen; kein
    /// Gegenstueck, fail-closed statt Wrap-Around.
    #[error("i64-Ueberlauf in {0}")]
    Ueberlauf(&'static str),
    /// Ein Parametersatz ist nicht in ganzen Cent darstellbar (z. B. `0.305` EUR/km). Python
    /// wuerde mit `f"{x:.2f}"` still runden.
    #[error("Parametersatz {0} nicht in ganzen Cent darstellbar")]
    NichtCentGenau(rust_decimal::Decimal),
}

impl From<EntfernungspauschaleFehler> for EngineFehler {
    fn from(e: EntfernungspauschaleFehler) -> Self {
        match e {
            EntfernungspauschaleFehler::Dezimal(d) => Self::Dezimal(d),
            EntfernungspauschaleFehler::Catala(c) => Self::Catala(c),
            EntfernungspauschaleFehler::Ueberlauf(marke) => Self::Ueberlauf(marke),
        }
    }
}

impl From<AltersentlastungsbetragFehler> for EngineFehler {
    fn from(e: AltersentlastungsbetragFehler) -> Self {
        match e {
            AltersentlastungsbetragFehler::Dezimal(d) => Self::Dezimal(d),
            AltersentlastungsbetragFehler::Catala(c) => Self::Catala(c),
        }
    }
}

impl From<VerbilligteVermietungFehler> for EngineFehler {
    fn from(e: VerbilligteVermietungFehler) -> Self {
        match e {
            VerbilligteVermietungFehler::Dezimal(d) => Self::Dezimal(d),
            VerbilligteVermietungFehler::Catala(c) => Self::Catala(c),
        }
    }
}

impl EngineFehler {
    /// Name der Python-Ausnahmeklasse, die an derselben Stelle entstuende, fuer den
    /// Paritaetsvergleich. `None`: Python kennt an dieser Stelle keinen Fehler (Ueberlauf,
    /// Rundung).
    ///
    /// ```
    /// use engine::zugriff::teil1::fehler::EngineFehler;
    /// let f = EngineFehler::Catala(catala_sys::CatalaFehler::Assertion);
    /// assert_eq!(f.python_typ(), Some("CatalaError"));
    /// assert_eq!(EngineFehler::Ueberlauf("x").python_typ(), None);
    /// ```
    #[must_use]
    pub fn python_typ(&self) -> Option<&'static str> {
        match self {
            Self::Catala(_) => Some("CatalaError"),
            Self::Params(_) => Some("KeyError"),
            Self::Dezimal(_) | Self::Ueberlauf(_) | Self::NichtCentGenau(_) => None,
        }
    }
}

/// `Option` aus `checked_*` in [`EngineFehler::Ueberlauf`].
pub(crate) fn ok(wert: Option<i64>, wo: &'static str) -> Result<i64, EngineFehler> {
    wert.ok_or(EngineFehler::Ueberlauf(wo))
}

/// Euro nach Cent, Ueberlauf als [`EngineFehler::Ueberlauf`].
pub(crate) fn in_cent(betrag: domain::Euro) -> Result<domain::Cent, EngineFehler> {
    betrag
        .to_cent()
        .map_err(|_| EngineFehler::Ueberlauf("Euro->Cent"))
}
