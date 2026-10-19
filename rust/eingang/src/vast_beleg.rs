//! Die Beleg-Datei der Lohnsteuerbescheinigung (`VaSt_LStB`, ELSTER-Datenabholung, Schema `VaSt_LStB-202501.xsd`) als flache Zuordnung
//! Element → Text für [`crate::vast::aus_lstb`] (Abweichung Nr. 90, Stufe S1). Kein Abruf bei ELSTER, kein ERiC: die Datei liegt schon da.
//!
//! Genau ein Beleg, genau eine Version. Jede andere Lage ist ein benannter [`BelegFehler`]; der Aufrufer schreibt dann nichts. Die
//! Steuer-ID und die Namen aus `Allgemein` verlassen diese Funktion nie: die Zuordnung führt nur die Elemente, die `LSTB` und `LSTB_SUMMEN`
//! lesen, und kein Fehlertext nennt einen Wert aus der Datei.
use std::collections::{BTreeMap, BTreeSet};

use roxmltree::{Document, Node, ParsingOptions};

use crate::vast::{cent, VastFehler, LSTB, LSTB_SUMMEN};

/// Die Wurzel des Belegs.
const WURZEL: &str = "VaSt_LStB";
/// Obergrenze der Knoten einer Beleg-Datei (ein echter Beleg hat unter 500).
const MAX_KNOTEN: u32 = 100_000;
/// Die Behälter unter `Besteuerungsgrund`, die das Schema mehrfach erlaubt (`VBez` 12, `ErmStVBezMKalJahr` 12, `Kammerbeitrag` 2). Ihre
/// Kinder liest der Import nicht. `ELStAM` (12) steht unter `Besteuerungsmerkmal`, nicht hier.
const WIEDERHOLBAR: [&str; 3] = ["VBez", "ErmStVBezMKalJahr", "Kammerbeitrag"];

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
    // Ohne DTD (kein Entitäten-Angriff) und mit Knotengrenze: die Datei kommt aus einer Anfrage.
    let optionen = ParsingOptions {
        allow_dtd: false,
        nodes_limit: MAX_KNOTEN,
        ..ParsingOptions::default()
    };
    let dok = Document::parse_with_options(xml, optionen).map_err(|_| BelegFehler::Kaputt)?;
    let belege: Vec<Node> = dok
        .descendants()
        .filter(|n| ist_element(n, WURZEL))
        .collect();
    let beleg = match belege.as_slice() {
        [] => return Err(BelegFehler::KeinBeleg),
        [eins] => *eins,
        viele => return Err(BelegFehler::MehrereBelege(viele.len())),
    };
    pruefe_version(beleg)?;
    pruefe_jahr(beleg)?;
    let mut gruende = kinder(beleg, "Besteuerungsgrund");
    let Some(grund) = gruende.next() else {
        return Ok(BTreeMap::new());
    };
    if gruende.next().is_some() {
        return Err(BelegFehler::WiederholtesBlatt("Besteuerungsgrund".into()));
    }
    betraege(grund)
}

/// Namensraum und `version` der Wurzel: nur das Schema 202501.
fn pruefe_version(beleg: Node) -> Result<(), BelegFehler> {
    let version = beleg.attribute("version");
    if version != Some(BELEG_VERSION) {
        return Err(BelegFehler::FalscheVersion(kurz(version.unwrap_or("-"))));
    }
    match beleg.tag_name().namespace() {
        Some(BELEG_NAMENSRAUM) => Ok(()),
        // `…/vastlstb/v202401`: der letzte Pfadteil nennt die Version, die das Schema meint.
        Some(fremd) => Err(BelegFehler::FalscheVersion(kurz(
            fremd.rsplit('/').find(|t| !t.is_empty()).unwrap_or("-"),
        ))),
        None => Err(BelegFehler::FalscheVersion("-".into())),
    }
}

/// `Dauer/Jahr`, falls vorhanden, ist 2025. Steht es mehrfach da, muss jedes 2025 sein.
fn pruefe_jahr(beleg: Node) -> Result<(), BelegFehler> {
    for dauer in kinder(beleg, "Dauer") {
        for jahr in kinder(dauer, "Jahr") {
            let text = text_von(jahr);
            if text.trim() != BELEG_JAHR {
                return Err(BelegFehler::FalschesJahr(kurz(text.trim())));
            }
        }
    }
    Ok(())
}

/// Die Beträge unter `Besteuerungsgrund`: jedes Kind höchstens einmal (außer den wiederholbaren Behältern), und von den Kindern nur die, die
/// `LSTB` und `LSTB_SUMMEN` lesen. Der Text wird auf seine Lesbarkeit als Betrag geprüft, damit der Fehler das Element nennt.
fn betraege(grund: Node) -> Result<BTreeMap<String, String>, BelegFehler> {
    let mut gesehen = BTreeSet::new();
    let mut werte = BTreeMap::new();
    for kind in grund.children().filter(Node::is_element) {
        let name = kind.tag_name().name();
        if !WIEDERHOLBAR.contains(&name) && !gesehen.insert(name) {
            return Err(BelegFehler::WiederholtesBlatt(kurz(name)));
        }
        if !gelesene_elemente().any(|n| n == name) {
            continue;
        }
        if kind.children().any(|k| k.is_element()) {
            return Err(BelegFehler::BetragNichtLesbar(name.to_owned()));
        }
        let text = text_von(kind);
        match cent(Some(&text)) {
            Ok(None) => {}
            Ok(Some(_)) => {
                werte.insert(name.to_owned(), text.trim().to_owned());
            }
            Err(VastFehler::NichtLesbar(_)) => {
                return Err(BelegFehler::BetragNichtLesbar(name.to_owned()))
            }
            Err(VastFehler::Ueberlauf(_)) => {
                return Err(BelegFehler::BetragAusserhalb(name.to_owned()))
            }
        }
    }
    Ok(werte)
}

/// Jeder Elementname, den `aus_lstb` liest.
fn gelesene_elemente() -> impl Iterator<Item = &'static str> {
    LSTB.iter()
        .map(|(vast, _, _)| *vast)
        .chain(LSTB_SUMMEN.iter().flat_map(|(_, q)| q.iter().map(|(n, _)| *n)))
}

fn ist_element(n: &Node, name: &str) -> bool {
    n.is_element() && n.tag_name().name() == name
}

fn kinder<'a, 'i>(n: Node<'a, 'i>, name: &'static str) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(move |k| ist_element(k, name))
}

/// Der Text eines Elements: alle Textknoten hintereinander (ein Kommentar mitten in der Zahl teilt sie in zwei Knoten).
fn text_von(n: Node) -> String {
    n.children()
        .filter(Node::is_text)
        .filter_map(|t| t.text())
        .collect()
}

/// Ein Wert aus der Datei, der in einen Fehlertext darf: kurz und aus Buchstaben, Ziffern, `_`, `-`, `.`; sonst `?`.
fn kurz(s: &str) -> String {
    let brauchbar = !s.is_empty()
        && s.chars().count() <= 32
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'));
    if brauchbar { s.to_owned() } else { "?".into() }
}
