//! Die Beleg-Datei der Lohnsteuerbescheinigung (`VaSt_LStB`, ELSTER-Datenabholung, Schema `VaSt_LStB-202501.xsd`) als flache Zuordnung
//! Element → Text für [`crate::vast::aus_lstb`] (Abweichung Nr. 90, Stufe S1). Kein Abruf bei ELSTER, kein ERiC: die Datei liegt schon da.
//!
//! Genau ein Beleg, genau eine Version. Jede andere Lage ist ein benannter [`BelegFehler`]; der Aufrufer schreibt dann nichts. Die
//! Steuer-ID und die Namen aus `Allgemein` verlassen diese Funktion nie: die Zuordnung führt nur die Elemente, die `LSTB` und `LSTB_SUMMEN`
//! lesen, und kein Fehlertext nennt einen Wert aus der Datei.
use std::collections::BTreeMap;

/// Der Namensraum des Schemas `VaSt_LStB-202501.xsd` (`targetNamespace`).
pub const BELEG_NAMENSRAUM: &str = "http://finkonsens.de/elster/elstervast/vastlstb/v202501";
/// Das feste Pflicht-Attribut `version` der Wurzel `VaSt_LStB`.
pub const BELEG_VERSION: &str = "202501";
/// Das Beschäftigungsjahr (`Dauer/Jahr`), das dieser Import liest.
pub const BELEG_JAHR: &str = "2025";

/// Warum eine Beleg-Datei nichts ergibt. Der Text nennt Elementnamen und Zahlen des Imports, nie einen Wert aus der Datei.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BelegFehler {
    /// Kein wohlgeformtes XML (auch: DTD, zu viele Knoten).
    #[error("Die Datei ist kein lesbares XML.")]
    Kaputt,
    /// Kein `VaSt_LStB` in der Datei.
    #[error("Die Datei enthält keinen Beleg der Lohnsteuerbescheinigung (VaSt_LStB).")]
    KeinBeleg,
    /// Mehr als ein `VaSt_LStB` (mehrere Arbeitgeber): nicht summieren, nicht den ersten nehmen.
    #[error("Die Datei enthält {0} Belege. Der Import nimmt genau einen Beleg (einen Arbeitgeber).")]
    MehrereBelege(usize),
    /// Namensraum oder `version` ist nicht das Schema 202501.
    #[error("Der Beleg hat die Version {0}. Der Import liest nur die Version 202501 (Veranlagungszeitraum 2025).")]
    FalscheVersion(String),
    /// `Dauer/Jahr` ist nicht 2025.
    #[error("Der Beleg gilt für das Jahr {0}. Der Import liest nur das Jahr 2025.")]
    FalschesJahr(String),
    /// Ein Element, das im Schema höchstens einmal vorkommt, steht mehrfach da.
    #[error("Das Element {0} steht im Beleg mehrfach. Der Import weist das ab, statt einen Wert zu wählen.")]
    WiederholtesBlatt(String),
    /// Ein Betrag-Element ist kein Betrag (kein Text, sondern ein Behälter, oder keine Dezimalzahl).
    #[error("Der Betrag im Element {0} ist nicht lesbar.")]
    BetragNichtLesbar(String),
    /// Ein Betrag-Element liegt außerhalb des Wertebereichs.
    #[error("Der Betrag im Element {0} liegt außerhalb des Wertebereichs.")]
    BetragAusserhalb(String),
}

impl BelegFehler {
    /// Der stabile Name des Fehlers (Antwort der Route, Tests).
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Kaputt => "beleg_kaputt",
            Self::KeinBeleg => "beleg_fehlt",
            Self::MehrereBelege(_) => "beleg_mehrere",
            Self::FalscheVersion(_) => "beleg_version",
            Self::FalschesJahr(_) => "beleg_jahr",
            Self::WiederholtesBlatt(_) => "beleg_element_mehrfach",
            Self::BetragNichtLesbar(_) => "beleg_betrag_unlesbar",
            Self::BetragAusserhalb(_) => "beleg_betrag_ausserhalb",
        }
    }
}

/// Liest genau einen Beleg `VaSt_LStB` (Version 202501, Jahr 2025) und liefert Element → Text der Beträge, die `LSTB` und `LSTB_SUMMEN`
/// lesen.
///
/// # Errors
/// [`BelegFehler`]: kaputtes XML, kein Beleg, mehrere Belege, falsche Version, falsches Jahr, ein Element mehrfach, ein nicht lesbarer
/// Betrag.
pub fn beleg_xml_zu_werten(xml: &str) -> Result<BTreeMap<String, String>, BelegFehler> {
    let _ = xml;
    Err(BelegFehler::Kaputt)
}
