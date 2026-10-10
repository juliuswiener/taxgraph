//! Store-Snapshot → ELSTER-Deklaration (`est_mapping.py::deklariere`, `:635-950`).
//!
//! Fail-closed (Auflage 3/C): nur BESTAETIGTE Werte werden deklariert; jedes vorlaeufige Feld
//! landet in `unvollstaendig`, und [`Deklaration::eingaben_konsistent`] ist per Konstruktion genau
//! dann wahr, wenn diese Liste leer ist — das Feld ist privat, kein Aufrufer kann es setzen.
//! Auflage A: die dokumentierte Aggregation steht in `dokumentiert`, nie in `deklaration`.
//! Auflage C: was bewusst nicht deklariert wird, steht mit Grund in `nicht_deklariert`.

use std::collections::{BTreeMap, HashMap};

use bindung::Bindung;
use domain::{Cent, Feldtyp, Lage, PyWert, Veranlagung, Zustand};
use serde::ser::SerializeMap;
use serde::Serialize;
use serde_json::Value;
use store::{EventId, SnapshotFeld};

use crate::geordnet::Geordnet;
use crate::instanz::parse_instanz;
use crate::kz_format::{cent_nach_kz, kz_pruefen, null_unzulaessig, schreibe_kz, KzBetrag};
use crate::py::{self, PyFehler};
use crate::tabellen::{
    suche, ArtKz, PflichtBedingung, Verzweigung, DOKUMENTIERT_AGGREGAT, KAP_FELDER_A, KAP_FELDER_B,
    KAP_NULL_GRUND, KONSTANTE_KZ, MULTIPLIKATION, NEGATION, P23_ART_FELD, P23_BETRAGSFELDER,
    P23_GEWINN_KZ, PARTNER_INSTANZ, PARTNER_VERZWEIGUNG, PFLEGE_KZ, PFLICHTFELDER, VERZWEIGUNG,
    WERTEKODIERUNG,
};

/// Nur Rust (Abweichung Nr. 26): der Anteil am Schulgeld-Hoechstbetrag je Kind (Kz `E0504603`).
const SCHULGELD_ANTEIL: &str = "kind_schulgeld_aufteilung_prozent";

/// Der Anteil, der "je zur Haelfte" heisst; er steht nicht im XML.
const SCHULGELD_ANTEIL_HAELFTE: i64 = 50;

/// Nur Rust (Abweichung Nr. 28): Unfallkosten auf dem Weg zur Arbeit, zusaetzlich zur Entfernungspauschale.
pub(crate) const UNFALLKOSTEN: &str = "ep_unfallkosten";

/// Die Zeile "Sonstiges" der Anlage N, in die die Unfallkosten gehoeren (Abweichung Nr. 48, `N/Wk/Weitere_Wk/Sonst`,
/// `E10-2025.xsd:18231-18274`; Anleitung zur Anlage N "Zeile 62 bis 64"): Bezeichnung, Betrag in volle Euro (aufgerundet,
/// `ABZUGS_KZ`) und die Summe der weiteren Werbungskosten. Dass es die Zeilen 62 und 63 sind, folgt aus Schema und Vordruck.
const UNFALLKOSTEN_TEXT_KZ: &str = "E0205405";
const UNFALLKOSTEN_BETRAG_KZ: &str = "E0205406";
const WEITERE_WK_SUMME_KZ: &str = "E0204803";

/// Die drei Kz der Zeile "Sonstiges", die `Bau::unfallkosten` aus EINEM Feld ableitet (kein Bindungs-Kz, wie bei der IBAN):
/// die Abdeckungstests (`abdeckung.rs`) kennen sie als Transform-Ziele.
#[cfg(test)]
pub(crate) const UNFALLKOSTEN_ZIEL_KZ: [&str; 3] = [
    UNFALLKOSTEN_TEXT_KZ,
    UNFALLKOSTEN_BETRAG_KZ,
    WEITERE_WK_SUMME_KZ,
];

/// Die Bezeichnung in `E0205405`. Das Schema (`NAEnum_BEWERBUNGSKOSTEN_3`) kennt keine Aufzaehlung; ob ERiC den Text gegen die
/// Namensliste der Dokumentation prueft, ist ohne `checkESt` offen (deshalb bleibt die Sperre).
const UNFALLKOSTEN_TEXT: &str = "Unfallkosten Arbeitsweg";

/// Der Grund der Abgabe-Sperre bei Unfallkosten ueber 0: das XML traegt den Betrag seit Abweichung Nr. 48 unter
/// [`UNFALLKOSTEN_BETRAG_KZ`], aber kein `checkESt`-Lauf hat die Zeile je angenommen. Die Sperre faellt mit dem ersten Lauf.
const UNFALLKOSTEN_SPERRE: &str = "Unfallkosten über 0 Euro: Für diesen Betrag gibt es noch kein geprüftes ELSTER-Kennzeichen. Die Abgabe ist deshalb gesperrt. Setze den Betrag auf 0 oder lösche ihn, wenn du ohne diesen Abzug abgeben willst.";

/// Nur Rust (Abweichung Nr. 49): der Ring-Wert zur zweiten Zeile "Sonstiges" der Anlage N, die gezahlte auslaendische Steuer in
/// CENT, wenn der BESCHEID den Abzug nach § 34c Abs. 2 `EStG` rechnet (`bescheid::deklaration::mit_ring_werten`). Das Feld hat kein
/// Kz; [`Bau::dba_abzug_zeile`] leitet daraus dieselben zwei Kz ab wie die Unfallkosten ([`UNFALLKOSTEN_TEXT_KZ`],
/// [`UNFALLKOSTEN_BETRAG_KZ`]), in der Gruppe [`WEITERE_WK_GRUPPE`] als Instanz [`DBA_ABZUG_ZEILE_INDEX`].
const DBA_ABZUG_ZEILE: &str = "dba_abzug_zeile_cent";

/// Die Bezeichnung der Abzugszeile in `E0205405`. Wie bei den Unfallkosten kennt das Schema keine Aufzaehlung; ob ERiC den Text
/// gegen die Namensliste prueft, ist ohne `checkESt` offen (deshalb bleibt die Sperre). Der Text steht im ELSTER-Zeichensatz.
const DBA_ABZUG_ZEILE_TEXT: &str = "Abzug ausländische Steuer nach § 34c Abs. 2 EStG";

/// Die Gruppe in `anlage_instanzen`, die die zweite Zeile "Sonstiges" traegt, und ihr Index. Instanz 1 ist die Zeile der
/// Unfallkosten in `deklaration`; der Schreiber legt die Instanz 2 in dasselbe `Weitere_Wk` (`INSTANZ_CONTAINER_TIEFER` in
/// `xml.rs`) und zaehlt den Rang dicht, auch wenn Instanz 1 fehlt.
const WEITERE_WK_GRUPPE: &str = "weitere_wk";
const DBA_ABZUG_ZEILE_INDEX: u64 = 2;

/// Nur Rust (Abweichung Nr. 29): die Wahl "Abzug statt Anrechnung" der auslaendischen Steuer (§ 34c Abs. 2 EStG).
const DBA_ABZUG: &str = "dba_abzug_statt_anrechnung";

/// Die gezahlte auslaendische Steuer; die Wahl sperrt nur zusammen mit einem Betrag ueber 0 in diesem Feld.
const DBA_STEUER: &str = "dba_gezahlte_auslaendische_steuer";

/// Das Kz der abgezogenen auslaendischen Steuer (§ 34c Abs. 2 EStG, Anlage AUS Zeile 10, `Staat_Spez_InvFonds`). Bei gewaehltem
/// Abzug und gezahlter Steuer ueber 0 steht die Steuer hier und NICHT unter `E0601901` (Abweichung Nr. 41); aufgerundet
/// (`ABZUGS_KZ`).
const DBA_ABZUG_KZ: &str = "E0600920";

/// Der Grund der Abgabe-Sperre bei gewaehltem Abzug und gezahlter Steuer ueber 0 (`dba_abzug_statt_anrechnung` ohne Kz):
/// der Bescheid kuerzt die Einkuenfte der Anlage N um die Steuer (bei DBA-Freistellung seit Abweichung Nr. 35 nicht mehr: dort
/// gibt es keinen Abzug). Die Erklaerung traegt die Steuer unter `E0600920` und, seit Abweichung Nr. 49, als zweite Zeile
/// "Sonstiges" der Anlage N (`Weitere_Wk/Sonst`, [`DBA_ABZUG_ZEILE`]). Kein `checkESt`-Lauf hat diese Zeile je angenommen: die
/// Sperre faellt mit dem ersten Lauf. Die Sperre nutzt die WEITE Bedingung ([`Bau::abzug_gewaehlt`]), die Zeile die enge des
/// Bescheids; in den zwei Faellen dazwischen (keine Auslandseinkuenfte, Freistellung) steht keine Zeile, der Text sagt "soweit".
const DBA_ABZUG_SPERRE: &str = "Abzug der ausländischen Steuer gewählt: Die Erklärung trägt den Betrag in der Anlage AUS und, soweit der Bescheid deine Einkünfte kürzt, in der Zeile „Sonstiges“ der Anlage N. Ein checkESt-Lauf hat diese Zeile noch nie angenommen. Die Abgabe ist deshalb gesperrt. Antworte „nein“ (Anrechnung), wenn du abgeben willst, oder trage den Abzug im Formular selbst ein.";

/// Nur Rust (Abweichung Nr. 51): der Staat der Auslandseinkuenfte (Anlage AUS, Zeile "1. Staat"), die Einkuenfte aus diesem
/// Staat (Kz kommt aus der Bindung, Anlage AUS Zeile 7) und die Tabelle der Listentexte. Das Feld hat kein Kz in der Bindung:
/// der Wert ist ein Bindungswert, das Schema will den Namen aus seiner Laenderliste ([`Bau::dba_staat`]).
pub(crate) const DBA_STAAT: &str = "dba_staat";
const DBA_EINKUENFTE: &str = "dba_auslaendische_einkuenfte";

/// Das Kz "aus dem Staat / Spezial-Investmentfonds" (Anlage AUS, `Staat_Spez_InvFonds/E0600301`). Der Typ ist Klartext aus der
/// Laenderliste des Schemas (197 Eintraege, nur in der Dokumentation, keine Aufzaehlung). `checkESt` prueft den Text nicht
/// (26 Texte gemessen, alle rc=0): die Gegenprobe gegen die Liste steht in `bescheid/tests/dba_staat_kz.rs`.
pub(crate) const STAAT_KZ: &str = "E0600301";

/// Die 15 benannten Bindungswerte von `dba_staat` und ihr Text in der Laenderliste. 12 sind gleich, `Oesterreich`, `Tschechien` und
/// `Grossbritannien` weichen ab. `sonstiger_staat` hat keinen Eintrag und sperrt ([`STAAT_SPERRE`]).
const DBA_STAAT_LISTENTEXT: &[(&str, &str)] = &[
    ("Deutschland", "Deutschland"),
    ("Frankreich", "Frankreich"),
    ("Italien", "Italien"),
    ("Oesterreich", "Österreich"),
    ("Schweiz", "Schweiz"),
    ("Niederlande", "Niederlande"),
    ("Polen", "Polen"),
    ("Tschechien", "Tschechische Republik"),
    ("Dänemark", "Dänemark"),
    ("Luxemburg", "Luxemburg"),
    ("Türkei", "Türkei"),
    ("Grossbritannien", "Vereinigtes Königreich"),
    ("Spanien", "Spanien"),
    ("USA", "USA"),
    ("Kanada", "Kanada"),
];

/// Der Grund der Abgabe-Sperre bei Auslandseinkuenften ohne benennbaren Staat: `sonstiger_staat`, ein fehlender oder ein
/// unbekannter Wert. Die Erklaerung braucht den Namen aus der amtlichen Laenderliste; ohne ihn lehnt `checkESt` die Akte ab.
const STAAT_SPERRE: &str = "Auslandseinkünfte ohne Staat: Die Erklärung braucht den Staat, aus dem die Einkünfte stammen, mit seinem Namen aus der amtlichen Länderliste. Für „sonstiger Staat“ und für eine fehlende oder unbekannte Angabe gibt es diesen Namen noch nicht. Die Abgabe ist deshalb gesperrt. Wähle den Staat in der Liste, wenn er dort steht, oder trage die Anlage AUS im amtlichen Formular selbst ein.";

/// Der Grund der Abgabe-Sperre bei auslaendischer Steuer (Anrechnung oder Abzug) ohne Einkuenfte aus dem Staat: die Anlage AUS
/// verlangt zur Steuer die Einkuenfte, auch mit dem Wert 0; `checkESt` lehnt die Akte sonst mit rc=610001002 ab.
const STEUER_OHNE_EINKUENFTE_SPERRE: &str = "Ausländische Steuer ohne Einkünfte: Die Anlage AUS verlangt zur Steuer die Einkünfte aus dem Staat, auch mit dem Wert 0. Die Abgabe ist deshalb gesperrt. Trage die Einkünfte aus diesem Staat ein, oder lösche die Steuer, wenn du ohne Auslandseinkünfte abgibst.";

/// Nur Rust (Abweichung Nr. 42): der Jahresbetrag der Versorgungsbezuege von Person A und, bei Zusammenveranlagung, des
/// Ehegatten (Abweichung Nr. 33). Beide Felder tragen kein Kz.
const VERSORGUNG: &str = "versorgung_jahresrente";
const VERSORGUNG_PARTNER: &str = "versorgung_jahresrente_partner";

/// Der Grund der Abgabe-Sperre bei einem bestaetigten Versorgungsbezug ueber 0 (`VERSORGUNG`, `VERSORGUNG_PARTNER` ohne Kz):
/// der Bescheid rechnet den Bezug ein (Versorgungsfreibetrag, Zuschlag, Pauschbetrag). Seit Abweichung Nr. 50 traegt das XML
/// ihn in den Zeilen 11 bis 13 der Anlage N, soweit der Bescheid ihn als Versorgungsbezug rechnet ([`VERSORGUNG_ZEILE`]); ein
/// `checkESt`-Lauf hat diese Zeilen mit Nr. 50 offline angenommen (rc=0), die Sperre bleibt trotzdem, bis Julius sie aufhebt
/// (Abweichung Nr. 42). Seit Nr. 53 nimmt der Text die Rente mit Leistungsmitteilung aus (§ 22 Nr. 5 `EStG`, kein Versorgungsbezug).
const VERSORGUNG_SPERRE: &str = "Versorgungsbezüge über 0 Euro: Die Erklärung trägt sie in den Zeilen 11 bis 13 der Anlage N, soweit der Bescheid sie als Versorgungsbezug rechnet. Die Abgabe ist dafür noch nicht freigegeben und deshalb gesperrt. Eine Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung mit Leistungsmitteilung gehört nicht in diese Zeilen. Trage die Versorgungsbezüge im amtlichen Formular selbst ein.";

/// Nur Rust (Abweichung Nr. 50): die Ring-Werte zu den Zeilen 11 bis 13 der Anlage N, je Person. `bescheid::deklaration::
/// mit_ring_werten` setzt sie auf `true`, wenn der Bescheid den Bezug als steuerbeguenstigten Versorgungsbezug rechnet (alle
/// drei Angaben bestaetigt, Alters-Gate erfuellt): das Gate kennt `elster` nicht. Beide Felder tragen kein Kz.
/// [`Bau::versorgung`] und [`Bau::versorgung_angabe`] schreiben daraus Betrag, Bemessungsgrundlage und Beginnjahr der Person
/// (Person B in die zweite Anlage N, `person_b`).
const VERSORGUNG_ZEILE: &str = "versorgung_zeile";
const VERSORGUNG_ZEILE_PARTNER: &str = "versorgung_zeile_partner";

/// Bemessungsgrundlage und Beginnjahr des Versorgungsbezugs (Person A und B). Auch sie tragen kein Kz in der Bindung (Weg 2
/// der Qualifikation: ein Kz dort kippte die Reihenfolge der Fragen); sie stehen nur im XML, wenn der Ring-Wert es verlangt.
const VERSORGUNG_BMG: &str = "versorgung_bemessungsgrundlage";
const VERSORGUNG_BMG_PARTNER: &str = "versorgung_bemessungsgrundlage_partner";
const VERSORGUNG_BEGINN: &str = "versorgung_beginn_jahr";
const VERSORGUNG_BEGINN_PARTNER: &str = "versorgung_beginn_jahr_partner";

/// Die Zeilen 11 bis 13 der Anlage N (`N/ArbL/VBez/Einz`): Betrag laut Nr. 8 der Lohnsteuerbescheinigung, Bemessungsgrundlage
/// laut Nr. 29, Beginnjahr laut Nr. 30. Fundstelle: `E10-2025.xsd` ab Zeile 17626, Vordruck `anlage_n_2025.txt:49-67`. Zeile 5
/// (`E0200201`, der Bruttoarbeitslohn) bleibt, wie eingegeben: sie ENTHAELT den Bezug.
const VERSORGUNG_BETRAG_KZ: &str = "E0200801";
const VERSORGUNG_BMG_KZ: &str = "E0200902";
const VERSORGUNG_BEGINN_KZ: &str = "E0201307";

/// Die materialisierte Felder-Ebene eines Snapshots (`feld_id -> {wert, zustand, herkunft}`).
pub type Felder = BTreeMap<String, SnapshotFeld>;

/// Bindungstabelle `feld_id -> Bindung` (s. [`store::baue_nachschlag`]).
pub type BindungIndex<'a> = HashMap<String, &'a Bindung>;

/// Ein Feld mit Grund (nicht deklariert, unvollstaendig oder Pflichtluecke).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Eintrag {
    pub feld_id: String,
    pub grund: String,
    /// Handlungsanweisung fuer den Nutzer (nur Klasse i, Wertekodierung ohne Code).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hinweis: Option<String>,
}

impl Eintrag {
    fn neu(feld_id: impl Into<String>, grund: impl Into<String>) -> Self {
        Self {
            feld_id: feld_id.into(),
            grund: grund.into(),
            hinweis: None,
        }
    }
}

/// Dokumentierte, nicht deklarierte Summe (Auflage A): verlustbehaftet, nur die Summe ist
/// rekonstruierbar; die Detail-Felder bleiben im Store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Aggregat {
    pub summe: i64,
    pub quell_felder: Vec<String>,
}

/// Eine Anlage-Kind-Instanz (Klasse e, nur die Anzahl).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindAnlage {
    pub index: i64,
}

/// Eine Instanz einer wiederholbaren Anlage (Kz-Reuse der Basis).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnlageInstanz {
    pub index: u64,
    pub felder: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dokumentiert: BTreeMap<String, Aggregat>,
}

/// Ergebnis von [`deklariere`].
#[derive(Debug, Clone, PartialEq)]
pub struct Deklaration {
    pub basis_snapshot: Option<String>,
    /// Kz → Wert, Person A und Basis-Instanzen.
    pub deklaration: BTreeMap<String, Value>,
    pub kind_anlagen: Vec<KindAnlage>,
    /// Anlage-Instanz B (Zusammenveranlagung): dieselben Kz wie Person A.
    pub person_b: BTreeMap<String, Value>,
    /// Gruppe → Instanzen, in der Reihenfolge des ersten Auftretens der Gruppe.
    pub anlage_instanzen: Vec<(String, Vec<AnlageInstanz>)>,
    pub dokumentiert: BTreeMap<String, Aggregat>,
    pub nicht_deklariert: Vec<Eintrag>,
    unvollstaendig: Vec<Eintrag>,
    pflichtfelder_luecken: Vec<Eintrag>,
}

impl Deklaration {
    /// Vorlaeufige oder widerspruechliche Eingaben (fail-closed).
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), 2025, None).unwrap();
    /// assert!(d.unvollstaendig().is_empty()); // nichts vorlaeufig, nichts widerspruechlich
    /// ```
    #[must_use]
    pub fn unvollstaendig(&self) -> &[Eintrag] {
        &self.unvollstaendig
    }

    /// Wahr genau dann, wenn nichts vorlaeufig oder widerspruechlich ist.
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), 2025, None).unwrap();
    /// assert!(d.eingaben_konsistent());
    /// assert_eq!(d.eingaben_konsistent(), d.unvollstaendig().is_empty());
    /// ```
    #[must_use]
    pub fn eingaben_konsistent(&self) -> bool {
        self.unvollstaendig.is_empty()
    }

    /// Fehlende Pflichtfelder (eigene Aussage, bewusst NICHT in `unvollstaendig`).
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), 2025, None).unwrap();
    /// // Pflichtluecken sind eine eigene Aussage und machen die Eingaben nicht inkonsistent.
    /// assert!(d.eingaben_konsistent());
    /// assert!(!d.pflichtfelder_luecken().is_empty()); // ohne jede Angabe fehlen Pflichtfelder
    /// ```
    #[must_use]
    pub fn pflichtfelder_luecken(&self) -> &[Eintrag] {
        &self.pflichtfelder_luecken
    }

    /// Instanzen einer Gruppe.
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), 2025, None).unwrap();
    /// assert!(d.instanzen_der_gruppe("kind").is_empty());
    /// ```
    #[must_use]
    pub fn instanzen_der_gruppe(&self, gruppe: &str) -> &[AnlageInstanz] {
        self.anlage_instanzen
            .iter()
            .find(|(g, _)| g == gruppe)
            .map_or(&[], |(_, v)| v.as_slice())
    }
}

impl Serialize for Deklaration {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        struct Gruppen<'a>(&'a [(String, Vec<AnlageInstanz>)]);
        impl Serialize for Gruppen<'_> {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let mut m = s.serialize_map(Some(self.0.len()))?;
                for (g, v) in self.0 {
                    m.serialize_entry(g, v)?;
                }
                m.end()
            }
        }
        let mut m = s.serialize_map(Some(12))?;
        m.serialize_entry("basis_snapshot", &self.basis_snapshot)?;
        m.serialize_entry("deklaration", &self.deklaration)?;
        m.serialize_entry("kind_anlagen", &self.kind_anlagen)?;
        m.serialize_entry("person_b", &self.person_b)?;
        m.serialize_entry("anlage_instanzen", &Gruppen(&self.anlage_instanzen))?;
        m.serialize_entry("dokumentiert", &self.dokumentiert)?;
        m.serialize_entry("nicht_deklariert", &self.nicht_deklariert)?;
        m.serialize_entry("unvollstaendig", &self.unvollstaendig)?;
        m.serialize_entry("vollstaendig", &self.eingaben_konsistent())?;
        m.serialize_entry("eingaben_konsistent", &self.eingaben_konsistent())?;
        m.serialize_entry("pflichtfelder_luecken", &self.pflichtfelder_luecken)?;
        m.serialize_entry(
            "pflichtfelder_vollstaendig",
            &self.pflichtfelder_luecken.is_empty(),
        )?;
        m.end()
    }
}

/// Warum [`deklariere`] kein Ergebnis liefert.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeklarationsFehler {
    /// Nachauflage D: das Snapshot-OBJEKT statt der Felder-Ebene uebergeben.
    ///
    /// PARITÄT: der Typ [`Felder`] schliesst die Verwechslung aus; Python prueft nur, ob ein
    /// Schluessel `felder`/`snapshot_id` heisst — dieselbe Pruefung steht hier, damit ein Feld
    /// dieses Namens in beiden Welten gleich abgewiesen wird.
    #[error(
        "deklariere() erwartet die felder-Ebene (feld_id -> {{wert, zustand, ...}}), NICHT das \
         Snapshot-Objekt — übergib materialisiere()[0] bzw. snapshot['felder']."
    )]
    SnapshotObjekt,
    /// Nachauflage D: nicht-leere Eingabe, aber kein Feld in der Bindungstabelle.
    #[error(
        "kein Eingabe-Feld in der Bindungstabelle gefunden — vermutlich falsche Eingabe-Ebene/-Struktur; \
         deklariere() liefert kein stilles Leer-Ergebnis."
    )]
    KeinFeldGebunden,
    /// Das Veranlagungsjahr fehlt, ist 0 oder unplausibel (`est_mapping.py:220-229`,
    /// [`null_unzulaessig`]).
    ///
    /// Eigene Variante, damit die Python-Klasse `ValueError` an der richtigen Stelle steht —
    /// `Wert { feld_id, .. }` verlangt ein Feld, hier gibt es keines.
    #[error("{0}")]
    Jahr(PyFehler),
    /// Ein Store-Wert passt nicht zu der Operation, die das Original auf ihm ausfuehrt.
    #[error("Feld {feld_id}: {fehler}")]
    Wert { feld_id: String, fehler: PyFehler },
    /// Eine Rechnung der Deklaration verlaesst `i64`; Python rechnet dort exakt weiter.
    ///
    /// GEWOLLTE ABWEICHUNG (Korrektheit vor Paritaet): Rust meldet den Fehler (HTTP 422, s.
    /// `api::deklaration::deklarations_fehler`), statt still zu rechnen. Der Text nennt nur das
    /// Feld und die Stelle, nie einen Wert aus dem Store.
    #[error("Feld {feld_id}: {was}")]
    Ueberlauf { feld_id: String, was: &'static str },
}

impl DeklarationsFehler {
    /// Die Python-Ausnahmeklasse, die `est_mapping.py` an derselben Stelle wirft.
    ///
    /// ```
    /// assert_eq!(elster::DeklarationsFehler::KeinFeldGebunden.python_klasse(), "ValueError");
    /// ```
    #[must_use]
    pub fn python_klasse(&self) -> &'static str {
        match self {
            Self::SnapshotObjekt | Self::KeinFeldGebunden => "ValueError",
            Self::Jahr(fehler) | Self::Wert { fehler, .. } => fehler.klasse,
            // Keine Python-Klasse: Python wirft hier nichts (s. [`Self::Ueberlauf`]). "OverflowError"
            // ist die Klasse der Rust-Grenze, wie bei `domain::PyFehler::I64Grenze` (`py.rs`).
            Self::Ueberlauf { .. } => "OverflowError",
        }
    }
}

#[derive(Default)]
struct InstBau {
    felder: BTreeMap<String, Value>,
    dokumentiert: BTreeMap<String, Aggregat>,
    rohdaten: BTreeMap<&'static str, i64>,
}

struct Bau<'a> {
    snapshot: &'a Felder,
    bindung: &'a BindungIndex<'a>,
    /// Die Null-Verbots-Menge DIESES Veranlagungsjahres ([`null_unzulaessig`]), durchgereicht an
    /// jede Schreibstelle (`_schreibe_kz(…, null_kz)`, `est_mapping.py:288-302`).
    null_kz: &'static [&'static str],
    deklaration: BTreeMap<String, Value>,
    kind_anlagen: Vec<KindAnlage>,
    person_b: BTreeMap<String, Value>,
    /// Einfuege-Reihenfolge von Gruppen UND Instanz-Indizes ist beobachtbar (§ 23-Nachlauf).
    instanzen: Geordnet<String, Geordnet<u64, InstBau>>,
    nicht_deklariert: Vec<Eintrag>,
    unvollstaendig: Vec<Eintrag>,
    agg_akku: Vec<(&'static str, Vec<(String, i64)>)>,
}

type Ergebnis<T> = Result<T, DeklarationsFehler>;

fn wert_fehler(feld_id: &str) -> impl Fn(PyFehler) -> DeklarationsFehler + '_ {
    move |fehler| DeklarationsFehler::Wert {
        feld_id: feld_id.to_owned(),
        fehler,
    }
}

/// `elster_kz` der Bindung (Python: `b.get("elster_kz")` ist truthy; ein leerer Text ist als `Kz`
/// nicht darstellbar).
fn kz_von(b: &Bindung) -> Option<&str> {
    b.elster_kz.as_ref().map(domain::Kz::as_str)
}

fn gruppe_von(b: &Bindung) -> Option<&str> {
    b.instanz_gruppe.as_deref().filter(|g| !g.is_empty())
}

fn zustand_text(z: Zustand) -> &'static str {
    match z {
        Zustand::Vorlaeufig => "vorlaeufig",
        Zustand::Bestaetigt => "bestaetigt",
    }
}

/// Python `dict.get(wert)` hasht den Schluessel vor der Suche: Listen/Objekte sind nicht hashbar
/// (`TypeError`), jeder andere Wert ist es.
fn hashbar(wert: &PyWert) -> Result<(), PyFehler> {
    match wert {
        PyWert::Liste(_) | PyWert::Objekt(_) => Err(PyFehler::typ("unhashable type")),
        _ => Ok(()),
    }
}

/// Python `dict.get(wert)` auf einer Tabelle mit Text-Schluesseln: jeder Nicht-Text trifft keinen
/// Schluessel.
fn nachschlagen<'t>(
    tabelle: &'t [(&'t str, &'t str)],
    wert: &PyWert,
) -> Result<Option<&'t str>, PyFehler> {
    hashbar(wert)?;
    Ok(match wert {
        PyWert::Text(s) => suche(tabelle, s),
        _ => None,
    })
}

/// Python `dict.get(wert)` auf einer Tabelle, deren Schluessel genau die Werte des Enums sind.
/// `Lage::Abweichend` fasst Text ohne Schluessel, Null/Bool/Zahl und Liste/Objekt zusammen; nur
/// Liste/Objekt bleibt `TypeError`.
fn nachschlagen_enum<T>(lage: Lage<'_, T>) -> Result<Option<T>, PyFehler> {
    match lage {
        Lage::Gueltig(t) => Ok(Some(t)),
        Lage::Abweichend(w) => hashbar(w).map(|()| None),
        Lage::Fehlt | Lage::Null => Ok(None),
    }
}

/// Die Kz zum Art-Wert einer [`Verzweigung`].
fn art_kz(kz: ArtKz, art: &PyWert) -> Result<Option<&'static str>, PyFehler> {
    match kz {
        ArtKz::Rente(tabelle) => Ok(nachschlagen_enum(Lage::rentenart(Some(art)))?.map(tabelle)),
        ArtKz::Text(tabelle) => nachschlagen(tabelle, art),
    }
}

/// `_cent_nach_kz(int(wert), ziel) if typ == "cent" else int(wert)` (Klasse a).
fn aggregat_beitrag(wert: &PyWert, ziel: &str, typ: Feldtyp) -> Result<i64, PyFehler> {
    let n = py::int(wert)?;
    if typ != Feldtyp::Cent {
        return Ok(n);
    }
    match cent_nach_kz(Cent::new(n), &kz_pruefen(ziel)?) {
        KzBetrag::Euro(e) => Ok(e.get()),
        KzBetrag::Komma(_) => Err(PyFehler::typ(
            "unsupported operand type(s) for +: 'int' and 'str'",
        )),
    }
}

impl Bau<'_> {
    fn instanz(&mut self, gruppe: &str, idx: u64) -> &mut InstBau {
        self.instanzen
            .eintrag(gruppe.to_owned(), Geordnet::default)
            .eintrag(idx, InstBau::default)
    }

    fn nicht(&mut self, feld_id: &str, grund: impl Into<String>) {
        self.nicht_deklariert.push(Eintrag::neu(feld_id, grund));
    }

    fn offen(&mut self, feld_id: &str, grund: impl Into<String>) {
        self.unvollstaendig.push(Eintrag::neu(feld_id, grund));
    }

    /// Gehoert der Anteil am Schulgeld-Hoechstbetrag (`E0504603`, Abweichung Nr. 26) eines Kindes ins XML? Nur bei
    /// bestaetigter Einzelveranlagung und einem Anteil ungleich 50. Das Kz sagt: "laut gemeinsamem Antrag NICHT je zur
    /// Haelfte aufzuteilen, mein Anteil ist ..."; bei 50 gibt es keinen solchen Antrag. Der Abschnitt `Elt_k_ZV` gilt im
    /// Schema nur fuer Eltern ohne Zusammenveranlagung, und die Rechnung liest den Anteil ebenso nur bei Einzelveranlagung
    /// (`bescheid::abzuege::schulgeld_summe`). Sonst bleibt die Angabe draussen, und der Grund steht in `nicht_deklariert`.
    fn schulgeld_anteil_gehoert_ins_xml(&mut self, feld_id: &str, wert: &PyWert) -> Ergebnis<bool> {
        let einzel = self.snapshot.get("veranlagung").is_some_and(|v| {
            v.zustand == Zustand::Bestaetigt
                && matches!(
                    Lage::veranlagung(Some(&v.wert)),
                    Lage::Gueltig(Veranlagung::Einzel)
                )
        });
        let anteil = py::int(wert).map_err(wert_fehler(feld_id))?;
        if einzel && anteil != SCHULGELD_ANTEIL_HAELFTE {
            return Ok(true);
        }
        let grund = if einzel {
            "Anteil 50 = je zur Hälfte, kein gesonderter Antrag der Eltern"
        } else {
            "Der Anteil am Schulgeld-Höchstbetrag gilt nur bei bestätigter Einzelveranlagung"
        };
        self.nicht(feld_id, grund);
        Ok(false)
    }

    /// `ep_unfallkosten` OHNE geprueftes Kz (Abweichung Nr. 28, Nr. 48). Ueber 0 steht der Betrag in der Zeile "Sonstiges" der
    /// Anlage N ([`UNFALLKOSTEN_BETRAG_KZ`], aufgerundet auf volle Euro; Bezeichnung [`UNFALLKOSTEN_TEXT_KZ`]; die Summe
    /// [`WEITERE_WK_SUMME_KZ`] bildet [`weitere_wk_summe`] aus allen Zeilen), und die Abgabe bleibt gesperrt
    /// ([`UNFALLKOSTEN_SPERRE`], 409 `deklaration_unvollstaendig`), bis `checkESt` die Zeile einmal angenommen hat: die Sperre
    /// haelt auch `erzeuge_xml` zurueck. Bei 0 und leer steht nichts im XML und der Grund in `nicht_deklariert`.
    /// Fuer die Sperre genuegt der Wegfall des `offen`-Aufrufs; die Zeile bleibt. Die zweite Zeile, der Abzug nach § 34c Abs. 2
    /// `EStG`, schreibt [`Bau::dba_abzug_zeile`] (Abweichung Nr. 49).
    fn unfallkosten(&mut self, feld_id: &str, wert: &PyWert, b: &Bindung) -> Ergebnis<()> {
        let betrag = py::int(wert).map_err(wert_fehler(feld_id))?;
        if betrag > 0 {
            schreibe_kz(
                &mut self.deklaration,
                UNFALLKOSTEN_BETRAG_KZ,
                wert,
                Some(b.typ),
                self.null_kz,
            )
            .map_err(wert_fehler(feld_id))?;
            self.deklaration.insert(
                UNFALLKOSTEN_TEXT_KZ.to_owned(),
                Value::String(UNFALLKOSTEN_TEXT.to_owned()),
            );
            self.offen(feld_id, UNFALLKOSTEN_SPERRE);
        } else {
            let grund = b
                .elster_kz_grund
                .clone()
                .unwrap_or_else(|| "kein elster_kz".to_owned());
            self.nicht(feld_id, grund);
        }
        Ok(())
    }

    /// `dba_abzug_zeile_cent` OHNE Kz (Abweichung Nr. 49): der Ring setzt den Wert nur, wenn der Bescheid den Abzug nach § 34c
    /// Abs. 2 `EStG` rechnet und nur aus bestaetigten Feldern. Ueber 0 steht der Betrag, aufgerundet auf volle Euro
    /// ([`UNFALLKOSTEN_BETRAG_KZ`]), mit der Bezeichnung [`DBA_ABZUG_ZEILE_TEXT`] als zweite Zeile "Sonstiges" in der Gruppe
    /// [`WEITERE_WK_GRUPPE`], Instanz [`DBA_ABZUG_ZEILE_INDEX`]. Die Sperre setzt dieser Zweig nicht: sie haengt am Marker
    /// `dba_abzug_statt_anrechnung` ([`Bau::dba_abzug`], weite Bedingung). Die Summe bildet [`weitere_wk_summe`].
    fn dba_abzug_zeile(&mut self, feld_id: &str, wert: &PyWert, b: &Bindung) -> Ergebnis<()> {
        let betrag = py::int(wert).map_err(wert_fehler(feld_id))?;
        if betrag > 0 {
            let null_kz = self.null_kz;
            let zeile = &mut self
                .instanz(WEITERE_WK_GRUPPE, DBA_ABZUG_ZEILE_INDEX)
                .felder;
            schreibe_kz(zeile, UNFALLKOSTEN_BETRAG_KZ, wert, Some(b.typ), null_kz)
                .map_err(wert_fehler(feld_id))?;
            zeile.insert(
                UNFALLKOSTEN_TEXT_KZ.to_owned(),
                Value::String(DBA_ABZUG_ZEILE_TEXT.to_owned()),
            );
        } else {
            let grund = b
                .elster_kz_grund
                .clone()
                .unwrap_or_else(|| "kein elster_kz".to_owned());
            self.nicht(feld_id, grund);
        }
        Ok(())
    }

    /// Hat der Nutzer die Zusammenveranlagung bestaetigt? Nur dann zaehlt der Ehegatte im Bescheid (Abweichung Nr. 33).
    fn zusammen_bestaetigt(&self) -> bool {
        self.snapshot.get("veranlagung").is_some_and(|v| {
            v.zustand == Zustand::Bestaetigt
                && matches!(
                    Lage::veranlagung(Some(&v.wert)),
                    Lage::Gueltig(Veranlagung::Zusammen)
                )
        })
    }

    /// Hat der Ring den Bezug der Person in die Zeilen 11 bis 13 der Anlage N verwiesen (Abweichung Nr. 50)? Der Ring-Wert
    /// [`VERSORGUNG_ZEILE`] bzw. [`VERSORGUNG_ZEILE_PARTNER`] muss bestaetigt `true` sein.
    fn versorgung_zeile_gilt(&self, partner: bool) -> bool {
        let flag = if partner {
            VERSORGUNG_ZEILE_PARTNER
        } else {
            VERSORGUNG_ZEILE
        };
        self.snapshot
            .get(flag)
            .is_some_and(|s| s.zustand == Zustand::Bestaetigt && matches!(s.wert, PyWert::Bool(true)))
    }

    /// Schreibt `wert` unter `kz` in die Anlage N der Person: Person A in die Deklaration, Person B in die zweite Instanz
    /// (`person_b`, wie die Kz aus `PARTNER_INSTANZ`).
    fn schreibe_versorgung(
        &mut self,
        partner: bool,
        kz: &str,
        feld_id: &str,
        wert: &PyWert,
        b: &Bindung,
    ) -> Ergebnis<()> {
        let ziel = if partner {
            &mut self.person_b
        } else {
            &mut self.deklaration
        };
        schreibe_kz(ziel, kz, wert, Some(b.typ), self.null_kz).map_err(wert_fehler(feld_id))
    }

    /// `versorgung_jahresrente` und `versorgung_jahresrente_partner` OHNE Kz (Abweichung Nr. 42, Nr. 50). Steht der Ring-Wert
    /// ([`Bau::versorgung_zeile_gilt`]) auf `true`, steht der Betrag in Zeile 11 der Anlage N ([`VERSORGUNG_BETRAG_KZ`], auf
    /// volle Euro abgerundet wie jede Einnahme), sonst mit Grund in `nicht_deklariert` (Bezug vor dem Alters-Gate: er steckt als
    /// Arbeitslohn in Zeile 5; Bezug unter 1 Euro). Ein bestaetigter Betrag ueber 0 sperrt die Abgabe in beiden Faellen
    /// ([`VERSORGUNG_SPERRE`], 409 `deklaration_unvollstaendig`), bis `checkESt` die Zeilen einmal angenommen hat; die Sperre
    /// haelt auch `erzeuge_xml` zurueck. Der Betrag des Ehegatten sperrt nur bei bestaetigter Zusammenveranlagung: bei
    /// Einzelveranlagung zaehlt er im Bescheid nicht. Ein vorlaeufiger Betrag erreicht diesen Zweig nicht; er behaelt den
    /// Grund "Pflicht-Bestaetigung fehlt" (`feld`). Ein Wert, aus dem sich keine ganze Zahl lesen laesst (`None`), zaehlt wie
    /// im Bescheid als 0 und wirft keinen Fehler (wie [`Bau::abzug_gewaehlt`]).
    fn versorgung(&mut self, feld_id: &str, wert: &PyWert, b: &Bindung) -> Ergebnis<()> {
        let partner = feld_id == VERSORGUNG_PARTNER;
        if self.versorgung_zeile_gilt(partner) {
            self.schreibe_versorgung(partner, VERSORGUNG_BETRAG_KZ, feld_id, wert, b)?;
        } else {
            let grund = b
                .elster_kz_grund
                .clone()
                .unwrap_or_else(|| "kein elster_kz".to_owned());
            self.nicht(feld_id, grund);
        }
        let zaehlt = feld_id == VERSORGUNG || self.zusammen_bestaetigt();
        if zaehlt && py::int(wert).is_ok_and(|cent| cent > 0) {
            self.offen(feld_id, VERSORGUNG_SPERRE);
        }
        Ok(())
    }

    /// Bemessungsgrundlage und Beginnjahr des Versorgungsbezugs OHNE Kz (Abweichung Nr. 50): Zeile 12 ([`VERSORGUNG_BMG_KZ`],
    /// Euro abgerundet) und Zeile 13 ([`VERSORGUNG_BEGINN_KZ`], das Jahr). Sie stehen nur im XML, wenn der Ring-Wert es
    /// verlangt ([`Bau::versorgung_zeile_gilt`]): ERiC lehnt Zeile 12 oder 13 ohne Zeile 11 ab (`Arbeitslohn_100200010`,
    /// `Arbeitslohn_ab08_9`). Sonst steht das Feld mit Grund in `nicht_deklariert`.
    fn versorgung_angabe(&mut self, feld_id: &str, wert: &PyWert, b: &Bindung) -> Ergebnis<()> {
        let partner = feld_id == VERSORGUNG_BMG_PARTNER || feld_id == VERSORGUNG_BEGINN_PARTNER;
        let kz = if feld_id == VERSORGUNG_BMG || feld_id == VERSORGUNG_BMG_PARTNER {
            VERSORGUNG_BMG_KZ
        } else {
            VERSORGUNG_BEGINN_KZ
        };
        if self.versorgung_zeile_gilt(partner) {
            self.schreibe_versorgung(partner, kz, feld_id, wert, b)
        } else {
            let grund = b
                .elster_kz_grund
                .clone()
                .unwrap_or_else(|| "kein elster_kz".to_owned());
            self.nicht(feld_id, grund);
            Ok(())
        }
    }

    /// Wahr, wenn der Nutzer den Abzug statt der Anrechnung bestaetigt gewaehlt hat (Wahl `true`) und die gezahlte Steuer
    /// bestaetigt ueber 0 liegt. Dann steht die Steuer unter [`DBA_ABZUG_KZ`] (Abweichung Nr. 41, `feld`) und die Abgabe ist
    /// gesperrt ([`Bau::dba_abzug`]). Beide Zweige lesen diese eine Bedingung: sonst stuende die Steuer an der falschen
    /// Stelle oder die Sperre griffe nicht. Gezaehlt wird in Cent, wie bei den Unfallkosten: ein Cent genuegt.
    fn abzug_gewaehlt(&self) -> bool {
        let bestaetigt = |id: &str| {
            self.snapshot
                .get(id)
                .filter(|s| s.zustand == Zustand::Bestaetigt)
        };
        bestaetigt(DBA_ABZUG).is_some_and(|w| matches!(w.wert, PyWert::Bool(true)))
            && bestaetigt(DBA_STEUER).is_some_and(|s| py::int(&s.wert).is_ok_and(|cent| cent > 0))
    }

    /// `dba_abzug_statt_anrechnung` OHNE Kz (Abweichung Nr. 29): der Marker steht mit Grund in `nicht_deklariert`. Ist die
    /// Wahl `true` und die gezahlte Steuer (bestaetigt) ueber 0, sperrt er die Abgabe ([`DBA_ABZUG_SPERRE`], 409
    /// `deklaration_unvollstaendig`): der Bescheid kuerzt die Werbungskosten der Anlage N (`bescheid::einkuenfte::
    /// dba_abzug_werbungskosten`, Abweichung Nr. 41), diese Zeile schreibt die Erklaerung nicht. Die Steuer selbst steht dann
    /// unter `E0600920` ([`Bau::abzug_gewaehlt`]). Die Sperre braucht die Auslandseinkuenfte NICHT: die Rechnung kuerzt nur bei
    /// Einkuenften ueber 0, die Erklaerung meldet aber auch bei 0 den Abzug, den der Nutzer gewaehlt hat.
    fn dba_abzug(&mut self, feld_id: &str, b: &Bindung) {
        let grund = b
            .elster_kz_grund
            .clone()
            .unwrap_or_else(|| "kein elster_kz".to_owned());
        self.nicht(feld_id, grund);
        if self.abzug_gewaehlt() {
            self.offen(feld_id, DBA_ABZUG_SPERRE);
        }
    }

    /// Steht das Kz des Felds (aus der Bindung, kein Literal) in der Deklaration von Person A?
    fn kz_des_felds_steht(&self, feld_id: &str) -> bool {
        self.bindung
            .get(feld_id)
            .and_then(|b| kz_von(b))
            .is_some_and(|kz| self.deklaration.contains_key(kz))
    }

    /// Der Staat der Auslandseinkuenfte (Abweichung Nr. 51), NACH der Feldschleife, denn er haengt daran, ob die Einkuenfte
    /// ([`DBA_EINKUENFTE`], `E0601401`) und die Steuer (`E0601901` oder, bei gewaehltem Abzug, [`DBA_ABZUG_KZ`]) schon stehen.
    ///
    /// 1. Einkuenfte stehen und der Wert ist ein benannter Staat: [`STAAT_KZ`] traegt den Listentext ([`DBA_STAAT_LISTENTEXT`]).
    /// 2. Einkuenfte stehen, der Staat fehlt, ist `sonstiger_staat` oder unbekannt: nichts im XML, Sperre ([`STAAT_SPERRE`]).
    /// 3. Einkuenfte fehlen, die Steuer steht: Sperre ([`STEUER_OHNE_EINKUENFTE_SPERRE`]), auch ohne Staat. `checkESt` lehnt
    ///    Steuer ohne Einkuenfte mit und ohne Staat ab (15 Kombinationen gemessen).
    /// 4. Einkuenfte und Steuer fehlen: kein Staat im XML (`checkESt` lehnt "Staat ohne Einkuenfte" ab), keine Sperre.
    ///
    /// Ein vorlaeufiger Staat hat seinen Eintrag ("Pflicht-Bestaetigung fehlt") schon aus [`Bau::feld`].
    fn dba_staat(&mut self) {
        let snapshot = self.snapshot;
        let staat = snapshot.get(DBA_STAAT);
        if staat.is_some_and(|s| s.zustand != Zustand::Bestaetigt) {
            return;
        }
        let hat_einkuenfte = self.kz_des_felds_steht(DBA_EINKUENFTE);
        let hat_steuer =
            self.kz_des_felds_steht(DBA_STEUER) || self.deklaration.contains_key(DBA_ABZUG_KZ);
        if !hat_einkuenfte {
            if hat_steuer {
                self.offen(DBA_STEUER, STEUER_OHNE_EINKUENFTE_SPERRE);
            }
            if staat.is_some() {
                self.nicht(DBA_STAAT, "Staat ohne Einkünfte aus diesem Staat: die Anlage AUS nennt den Staat nur mit Einkünften");
            }
            return;
        }
        let text = staat.and_then(|s| match &s.wert {
            PyWert::Text(t) => suche(DBA_STAAT_LISTENTEXT, t),
            _ => None,
        });
        if let Some(text) = text {
            self.deklaration
                .insert(STAAT_KZ.to_owned(), Value::String(text.to_owned()));
        } else {
            if staat.is_some() {
                self.nicht(DBA_STAAT, "Staat ohne Listentext: sonstiger Staat oder unbekannte Angabe");
            }
            self.offen(DBA_STAAT, STAAT_SPERRE);
        }
    }

    /// `_deklariere_instanz` (`est_mapping.py:590-632`).
    fn instanz_feld(
        &mut self,
        basis: &str,
        idx: u64,
        feld_id: &str,
        sfeld: &SnapshotFeld,
        b: &Bindung,
    ) -> Ergebnis<()> {
        let fehler = wert_fehler(feld_id);
        if sfeld.zustand != Zustand::Bestaetigt {
            let grund = format!(
                "Instanz-Wert {} — Pflicht-Bestätigung (Zwei-Signal) fehlt",
                zustand_text(sfeld.zustand)
            );
            self.offen(feld_id, grund);
            return Ok(());
        }
        let gruppe = gruppe_von(b).unwrap_or_default();
        let wert = &sfeld.wert;
        self.instanz(gruppe, idx);
        if let Some((ziel, _)) = DOKUMENTIERT_AGGREGAT
            .iter()
            .find(|(_, q)| q.contains(&basis))
        {
            let beitrag = aggregat_beitrag(wert, ziel, b.typ).map_err(&fehler)?;
            let agg = self
                .instanz(gruppe, idx)
                .dokumentiert
                .entry((*ziel).to_owned())
                .or_insert_with(|| Aggregat {
                    summe: 0,
                    quell_felder: Vec::new(),
                });
            agg.summe = agg
                .summe
                .checked_add(beitrag)
                .ok_or_else(|| fehler(PyFehler::ueberlauf("Summe jenseits i64")))?;
            agg.quell_felder.push(feld_id.to_owned());
        } else if let Some(cfg) = VERZWEIGUNG.iter().find(|v| v.feld == basis) {
            let art_feld_inst = format!("{}__{idx}", cfg.art_feld);
            let art = self.snapshot.get(&art_feld_inst);
            match art.filter(|a| a.zustand == Zustand::Bestaetigt) {
                None => self.offen(
                    feld_id,
                    format!("Instanz-Art ({art_feld_inst}) unbestätigt — Kz-Zweig offen"),
                ),
                Some(art) => {
                    if let Some(kz) = art_kz(cfg.kz, &art.wert).map_err(&fehler)? {
                        // Vor dem mutablen Borrow kopieren: `null_kz` ist `&'static`.
                        let null_kz = self.null_kz;
                        let felder = &mut self.instanz(gruppe, idx).felder;
                        schreibe_kz(felder, kz, wert, Some(b.typ), null_kz).map_err(&fehler)?;
                    } else {
                        let grund = format!("Instanz-Art '{}' ohne Kz-Zweig", art.wert.py_str());
                        self.nicht(feld_id, grund);
                    }
                }
            }
        } else if let Some(p23) = P23_BETRAGSFELDER.iter().find(|f| **f == basis) {
            let n = py::int(wert).map_err(&fehler)?;
            self.instanz(gruppe, idx).rohdaten.insert(p23, n);
        } else if let (SCHULGELD_ANTEIL, Some(kz)) = (basis, kz_von(b)) {
            if self.schulgeld_anteil_gehoert_ins_xml(feld_id, wert)? {
                let null_kz = self.null_kz;
                let felder = &mut self.instanz(gruppe, idx).felder;
                schreibe_kz(felder, kz, wert, Some(b.typ), null_kz).map_err(&fehler)?;
            }
        } else if let Some(kz) = kz_von(b) {
            let null_kz = self.null_kz;
            let felder = &mut self.instanz(gruppe, idx).felder;
            schreibe_kz(felder, kz, wert, Some(b.typ), null_kz).map_err(&fehler)?;
        } else {
            self.nicht(
                feld_id,
                format!("Instanz-Basis '{basis}' ohne elster_kz/Aggregat-Ziel"),
            );
        }
        Ok(())
    }

    /// Klasse f (Person A) bzw. g×f (Person B): Kz haengt am bestaetigten Art-Feld.
    fn verzweigung(
        &mut self,
        feld_id: &str,
        wert: &PyWert,
        b: &Bindung,
        cfg: &Verzweigung,
        partner: bool,
    ) -> Ergebnis<()> {
        let fehler = wert_fehler(feld_id);
        let art = self
            .snapshot
            .get(cfg.art_feld)
            .filter(|a| a.zustand == Zustand::Bestaetigt);
        let Some(art) = art else {
            let grund = if partner {
                format!(
                    "Partner-Renten-Art ({}) unbestätigt — Kz-Zweig offen",
                    cfg.art_feld
                )
            } else {
                format!("Art ({}) unbestätigt — Kz-Zweig offen", cfg.art_feld)
            };
            self.offen(feld_id, grund);
            return Ok(());
        };
        let art_text = art.wert.py_str();
        match art_kz(cfg.kz, &art.wert).map_err(&fehler)? {
            Some(kz) => {
                let ziel = if partner {
                    &mut self.person_b
                } else {
                    &mut self.deklaration
                };
                schreibe_kz(ziel, kz, wert, Some(b.typ), self.null_kz).map_err(&fehler)?;
            }
            None if partner => {
                self.nicht(
                    feld_id,
                    format!("Partner-Renten-Art '{art_text}' ohne Kz-Zweig"),
                );
                self.offen(
                    feld_id,
                    format!(
                        "Eure Angabe zu '{art_text}' ({}) ist vollständig, nichts von euch fehlt oder \
                         widerspricht sich. Diese Einkunftsart des Partners ist bei uns noch nicht abgebbar \
                         — das liegt nicht an eurer Eingabe, ihr müsst hier nichts nachtragen.",
                        cfg.art_feld
                    ),
                );
            }
            None => {
                self.nicht(
                    feld_id,
                    format!("Art '{art_text}' ({}) ohne Kz-Zweig", cfg.art_feld),
                );
                self.offen(
                    feld_id,
                    format!(
                        "Für '{art_text}' ({}) ist diese Einkunftsart bei uns noch nicht abgebbar — keine \
                         Eingabe von dir fehlt oder widerspricht sich.",
                        cfg.art_feld
                    ),
                );
            }
        }
        Ok(())
    }

    /// Klasse j — IBAN: Format, Pruefziffer, Laender-Weiche.
    fn iban(&mut self, feld_id: &str, wert: &PyWert) {
        let norm: String = wert
            .py_str()
            .chars()
            .filter(|c| !py::ist_leerraum(*c))
            .collect::<String>()
            .to_uppercase();
        if !iban_muster(&norm) {
            self.offen(
                feld_id,
                "IBAN entspricht nicht dem amtlichen Muster (2 Buchstaben Laendercode + 2 Pruefziffern + 1-30 \
                 alphanumerische Zeichen, String_MinL5_MaxL34_Musterm188614856_CType, E10-2025.xsd) -- Wert \
                 nicht geloggt.",
            );
        } else if !iban_pruefziffer_gueltig(&norm) {
            self.offen(
                feld_id,
                "IBAN-Pruefziffer (ISO 13616, Modulo 97) ungueltig -- Wert nicht geloggt.",
            );
        } else {
            let kz = if norm.starts_with("DE") {
                "E0102102"
            } else {
                "E0102603"
            };
            self.deklaration.insert(kz.to_owned(), Value::String(norm));
        }
    }

    /// Ein Feld der Hauptschleife (`est_mapping.py:671-782`).
    fn feld(&mut self, feld_id: &str, sfeld: &SnapshotFeld, b: &Bindung) -> Ergebnis<()> {
        let fehler = wert_fehler(feld_id);
        if sfeld.zustand != Zustand::Bestaetigt {
            let grund = format!(
                "Wert {} — Pflicht-Bestätigung (Zwei-Signal) fehlt",
                zustand_text(sfeld.zustand)
            );
            self.offen(feld_id, grund);
            return Ok(());
        }
        let wert = &sfeld.wert;
        if let Some(kz) = suche(NEGATION, feld_id) {
            self.deklaration
                .insert(kz.to_owned(), Value::Bool(!wert.truthy()));
        } else if MULTIPLIKATION.contains(&feld_id) {
            let n = py::int(wert).map_err(&fehler)?;
            self.kind_anlagen = (1..=n.max(0)).map(|index| KindAnlage { index }).collect();
            let grund = b
                .elster_kz_grund
                .as_deref()
                .filter(|g| !g.is_empty())
                .unwrap_or("Multiplikation -> kind_anlagen (Kinderzahl implizit)");
            self.nicht(feld_id, grund);
        } else if let Some((ziel, _)) = DOKUMENTIERT_AGGREGAT
            .iter()
            .find(|(_, q)| q.contains(&feld_id))
        {
            let beitrag = aggregat_beitrag(wert, ziel, b.typ).map_err(&fehler)?;
            if let Some((_, akku)) = self.agg_akku.iter_mut().find(|(z, _)| z == ziel) {
                akku.push((feld_id.to_owned(), beitrag));
            }
        } else if let Some(cfg) = VERZWEIGUNG.iter().find(|v| v.feld == feld_id) {
            self.verzweigung(feld_id, wert, b, cfg, false)?;
        } else if let Some(cfg) = PARTNER_VERZWEIGUNG.iter().find(|v| v.feld == feld_id) {
            self.verzweigung(feld_id, wert, b, cfg, true)?;
        } else if let Some(kz) = suche(PARTNER_INSTANZ, feld_id) {
            schreibe_kz(&mut self.person_b, kz, wert, Some(b.typ), self.null_kz)
                .map_err(&fehler)?;
        } else if let Some(cfg) = WERTEKODIERUNG.iter().find(|w| w.feld == feld_id) {
            let konfession = nachschlagen_enum(Lage::konfession(Some(wert))).map_err(&fehler)?;
            match konfession.and_then(cfg.code) {
                Some(code) => {
                    self.deklaration
                        .insert(cfg.kz.to_owned(), Value::String(code.to_owned()));
                }
                None => self.nicht_deklariert.push(Eintrag {
                    feld_id: feld_id.to_owned(),
                    grund: format!(
                        "Wert '{}' ohne XSD-Code-Zuordnung ({})",
                        wert.py_str(),
                        cfg.kz
                    ),
                    hinweis: Some(cfg.hinweis_unbekannt.to_owned()),
                }),
            }
        } else if feld_id == DBA_STAAT {
            // Abweichung Nr. 51: der Staat steht nach der Feldschleife (`Bau::dba_staat`), nicht hier.
        } else if feld_id == "stammdaten_iban" {
            self.iban(feld_id, wert);
        } else if let (SCHULGELD_ANTEIL, Some(kz)) = (feld_id, kz_von(b)) {
            // Kind 1 (ohne `__N`) steht auf oberster Ebene, Kind 2.. in den Instanzen (`instanz_feld`).
            if self.schulgeld_anteil_gehoert_ins_xml(feld_id, wert)? {
                schreibe_kz(&mut self.deklaration, kz, wert, Some(b.typ), self.null_kz)
                    .map_err(&fehler)?;
            }
        } else if let (UNFALLKOSTEN, None) = (feld_id, kz_von(b)) {
            self.unfallkosten(feld_id, wert, b)?;
        } else if let (VERSORGUNG | VERSORGUNG_PARTNER, None) = (feld_id, kz_von(b)) {
            self.versorgung(feld_id, wert, b)?;
        } else if let (
            VERSORGUNG_BMG | VERSORGUNG_BMG_PARTNER | VERSORGUNG_BEGINN | VERSORGUNG_BEGINN_PARTNER,
            None,
        ) = (feld_id, kz_von(b))
        {
            self.versorgung_angabe(feld_id, wert, b)?;
        } else if let (VERSORGUNG_ZEILE | VERSORGUNG_ZEILE_PARTNER, None) = (feld_id, kz_von(b)) {
            // Ring-Wert (Abweichung Nr. 50): kein Feld der Erklaerung. `versorgung` und `versorgung_angabe` lesen ihn.
        } else if let (DBA_ABZUG, None) = (feld_id, kz_von(b)) {
            self.dba_abzug(feld_id, b);
        } else if let (DBA_ABZUG_ZEILE, None) = (feld_id, kz_von(b)) {
            self.dba_abzug_zeile(feld_id, wert, b)?;
        } else if feld_id == DBA_STEUER && self.abzug_gewaehlt() {
            // Abweichung Nr. 41: bei gewaehltem Abzug steht die Steuer unter `E0600920`, nicht als Anrechnung unter `E0601901`.
            schreibe_kz(
                &mut self.deklaration,
                DBA_ABZUG_KZ,
                wert,
                Some(b.typ),
                self.null_kz,
            )
            .map_err(&fehler)?;
        } else if let Some(kz) = kz_von(b) {
            schreibe_kz(&mut self.deklaration, kz, wert, Some(b.typ), self.null_kz)
                .map_err(&fehler)?;
        } else if let Some(p23) = P23_BETRAGSFELDER.iter().find(|f| **f == feld_id) {
            let n = py::int(wert).map_err(&fehler)?;
            self.instanz("p23_veraeusserung", 1).rohdaten.insert(p23, n);
        } else {
            let grund = b
                .elster_kz_grund
                .clone()
                .unwrap_or_else(|| "kein elster_kz".to_owned());
            self.nicht(feld_id, grund);
        }
        Ok(())
    }
}

/// `^[A-Z]{2}[0-9]{2}[0-9A-Z]{1,30}$` (`String_MinL5_MaxL34_Musterm188614856_CType`).
pub(crate) fn iban_muster(s: &str) -> bool {
    let b = s.as_bytes();
    (5..=34).contains(&b.len())
        && b.iter().take(2).all(u8::is_ascii_uppercase)
        && b.iter().skip(2).take(2).all(u8::is_ascii_digit)
        && b.iter()
            .skip(4)
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
}

/// ISO 13616 Pruefziffer (Modulo 97): erste vier Zeichen ans Ende, Buchstaben A=10..Z=35, Rest 1.
fn iban_pruefziffer_gueltig(iban: &str) -> bool {
    let (kopf, rumpf) = iban.split_at(4.min(iban.len()));
    let mut rest: u32 = 0;
    for c in rumpf.chars().chain(kopf.chars()) {
        let Some(d) = c.to_digit(36) else {
            return false;
        };
        for ziffer in d.to_string().chars().filter_map(|z| z.to_digit(10)) {
            rest = (rest * 10 + ziffer) % 97;
        }
    }
    rest == 1
}

/// `_pflichtfelder_luecken` (`est_mapping.py:496-513`): sieht Feld-ABWESENHEIT, die die
/// Hauptschleife per Konstruktion nicht sehen kann. Oeffentlich seit Abweichung Nr. 47: `GET /preflight` nennt dieselben
/// Luecken wie `GET /deklaration` (`pflichtfelder_vollstaendig`), statt eine zweite Liste zu fuehren.
#[must_use]
pub fn pflichtfelder_luecken(snapshot: &Felder) -> Vec<Eintrag> {
    let mut luecken = Vec::new();
    for (bedingung, version, felder) in PFLICHTFELDER {
        if matches!(bedingung, PflichtBedingung::AlleOderKeins)
            && !felder.iter().any(|f| snapshot.contains_key(*f))
        {
            continue;
        }
        for f in felder.iter().filter(|f| !snapshot.contains_key(**f)) {
            luecken.push(Eintrag::neu(
                *f,
                format!(
                    "Pflichtfeld fehlt (ERiC {version}, Bedingung '{}') -- kein Wert im Snapshot",
                    bedingung.als_str()
                ),
            ));
        }
    }
    luecken
}

/// `_kap_alle_null`: jedes Feld fehlt oder ist bestaetigt 0.
fn kap_alle_null(snapshot: &Felder, felder: &[&str]) -> Ergebnis<bool> {
    for f in felder {
        let Some(sfeld) = snapshot.get(*f) else {
            continue;
        };
        if sfeld.zustand != Zustand::Bestaetigt
            || py::int(&sfeld.wert).map_err(wert_fehler(f))? != 0
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Snapshot (Felder-Ebene) → Deklaration (`est_mapping.py::deklariere`).
///
/// Nur bestaetigte Felder werden deklariert; der Rest steht mit Grund in
/// [`Deklaration::unvollstaendig`] bzw. `nicht_deklariert`.
///
/// `vz` ist das Veranlagungsjahr, PFLICHT. Es entscheidet, welche Kz eine 0 verbieten duerfen
/// ([`null_unzulaessig`]) — E0106603 verbietet sie 2024 und erlaubt sie 2025. Es gibt keinen
/// Vorgabewert, kein `None` und keinen Rueckfall: ein fehlendes oder unplausibles Jahr ist ein
/// Fehler, weil eine still gewaehlte Menge genau die Fehlerklasse waere, die diese Pruefung
/// abstellt (Julius-Entscheidung 2026-10-01, festgehalten in `est_mapping.py::null_unzulaessig`).
///
/// # Errors
/// [`DeklarationsFehler`], wenn die Eingabe-Ebene falsch ist, das Jahr fehlt/0/unplausibel ist,
/// oder ein Store-Wert nicht zu der Operation passt, die das Original auf ihm ausfuehrt
/// (Python-Ausnahme an derselben Stelle).
///
/// ```
/// use std::collections::HashMap;
/// use elster::{deklariere, Felder};
/// let leer = Felder::new();
/// let d = deklariere(&leer, &HashMap::new(), 2025, None).unwrap();
/// assert!(d.eingaben_konsistent());
/// assert_eq!(d.deklaration.get("E0100001"), Some(&serde_json::json!(true)));
/// assert!(deklariere(&leer, &HashMap::new(), 0, None).is_err());
/// let sid = store::EventId::aus_bytes([7; 32]);
/// let mit = deklariere(&leer, &HashMap::new(), 2025, Some(&sid)).unwrap();
/// assert_eq!(mit.basis_snapshot.as_deref(), Some(sid.to_string().as_str()));
/// ```
pub fn deklariere(
    snapshot: &Felder,
    bindung: &BindungIndex<'_>,
    vz: i64,
    snapshot_id: Option<&EventId>,
) -> Ergebnis<Deklaration> {
    if snapshot.contains_key("felder") || snapshot.contains_key("snapshot_id") {
        return Err(DeklarationsFehler::SnapshotObjekt);
    }
    // fail-closed, VOR jeder Schreibstelle (`est_mapping.py:763`).
    let null_kz = null_unzulaessig(vz).map_err(DeklarationsFehler::Jahr)?;
    let mut bau = Bau {
        snapshot,
        bindung,
        null_kz,
        deklaration: KONSTANTE_KZ
            .iter()
            .map(|k| ((*k).to_owned(), Value::Bool(true)))
            .collect(),
        kind_anlagen: Vec::new(),
        person_b: BTreeMap::new(),
        instanzen: Geordnet::default(),
        nicht_deklariert: Vec::new(),
        unvollstaendig: Vec::new(),
        agg_akku: DOKUMENTIERT_AGGREGAT
            .iter()
            .map(|(z, _)| (*z, Vec::new()))
            .collect(),
    };
    let mut getroffen = 0_usize;
    for (feld_id, sfeld) in snapshot {
        if let Some((basis, idx)) = parse_instanz(feld_id) {
            if let Some(b) = bindung.get(basis).filter(|b| gruppe_von(b).is_some()) {
                getroffen += 1;
                bau.instanz_feld(basis, idx, feld_id, sfeld, b)?;
                continue;
            }
        }
        let Some(b) = bindung.get(feld_id.as_str()) else {
            bau.nicht(feld_id, "nicht in der Bindungstabelle");
            bau.offen(feld_id, "Feld nicht in der Bindungstabelle");
            continue;
        };
        getroffen += 1;
        bau.feld(feld_id, sfeld, b)?;
    }
    if !snapshot.is_empty() && getroffen == 0 {
        return Err(DeklarationsFehler::KeinFeldGebunden);
    }
    let pflichtfelder_luecken = pflichtfelder_luecken(snapshot);
    bau.bankverbindung();
    bau.kap_nulldeklaration()?;
    bau.pflegeblock();
    bau.dba_staat();
    bau.antrag_person_b();
    let dokumentiert = bau.dokumentiert();
    bau.p23_gewinn()?;
    let anlage_instanzen = bau.instanzen_ausgabe();
    p35a_summe_aus_posten(&mut bau.deklaration, &anlage_instanzen);
    weitere_wk_summe(&mut bau.deklaration, &anlage_instanzen);
    Ok(Deklaration {
        basis_snapshot: snapshot_id.map(ToString::to_string),
        deklaration: bau.deklaration,
        kind_anlagen: bau.kind_anlagen,
        person_b: bau.person_b,
        anlage_instanzen,
        dokumentiert,
        nicht_deklariert: bau.nicht_deklariert,
        unvollstaendig: bau.unvollstaendig,
        pflichtfelder_luecken,
    })
}

/// § 35a: (Summen-Kz, Posten-Kz). Die Summe ist die Summe der GERUNDETEN Posten (Vordruck Zeile 9:
/// „+ … ="), nicht die aufgerundete Rohsumme aus dem Ring. Sonst standen bei 4 × 100,01 € vier
/// Posten zu 101 gegen eine Summe von 401: Differenz 3, ERiC toleriert 2 (rc=610001002).
/// `_P35A_SUMME_AUS_POSTEN` in `est_mapping.py`.
// ponytail: feste Liste, dieselben drei Toepfe wie `haushaltsnah` in
// `rust/bescheid/src/deklaration/ring_werte.rs`. Ein vierter Topf, dessen Summe die Engine aus
// Posten bildet, braucht hier und in `est_mapping.py` einen Eintrag; ab dann lohnt es, die Paare
// aus der Bindung abzuleiten.
const P35A_SUMME_AUS_POSTEN: [(&str, &str); 3] = [
    ("E0104109", "E0104108"),
    ("E0107208", "E0107207"),
    ("E0111215", "E0111214"),
];

/// Summen-Kz = Summe der gerundeten Posten-Kz (Instanz 1 in `deklaration`, 2..N in den
/// Instanzen). Ersetzt wird nur eine vorhandene Summe und nur durch einen Betrag > 0, wie im
/// Ring. Ohne Posten bleibt ein Bestandswert stehen (`tests/test_p35a_bestandsdaten.py`).
fn p35a_summe_aus_posten(
    deklaration: &mut BTreeMap<String, Value>,
    instanzen: &[(String, Vec<AnlageInstanz>)],
) {
    for (summe_kz, posten_kz) in P35A_SUMME_AUS_POSTEN {
        let summe: i64 = deklaration
            .get(posten_kz)
            .into_iter()
            .chain(
                instanzen
                    .iter()
                    .flat_map(|(_, ii)| ii)
                    .filter_map(|i| i.felder.get(posten_kz)),
            )
            .filter_map(Value::as_i64)
            .sum();
        if summe > 0 && deklaration.contains_key(summe_kz) {
            deklaration.insert(summe_kz.to_owned(), Value::from(summe));
        }
    }
}

/// [`WEITERE_WK_SUMME_KZ`] = Summe der GERUNDETEN Zeilen "Sonstiges" der Anlage N (Aufwand: Vault
/// `aufwand-einzelposten-aufrunden-summe-aus-posten`; `ERiC` weist ab 1 Euro Differenz ab): die Zeile in `deklaration`
/// (Unfallkosten, Instanz 1, Abweichung Nr. 48) und die der Gruppe [`WEITERE_WK_GRUPPE`] (Abzug nach § 34c Abs. 2 `EStG`,
/// Abweichung Nr. 49). Ohne Zeile keine Summe.
fn weitere_wk_summe(
    deklaration: &mut BTreeMap<String, Value>,
    instanzen: &[(String, Vec<AnlageInstanz>)],
) {
    let zeilen: Vec<i64> = deklaration
        .get(UNFALLKOSTEN_BETRAG_KZ)
        .into_iter()
        .chain(
            instanzen
                .iter()
                .filter(|(gruppe, _)| gruppe == WEITERE_WK_GRUPPE)
                .flat_map(|(_, ii)| ii)
                .filter_map(|i| i.felder.get(UNFALLKOSTEN_BETRAG_KZ)),
        )
        .filter_map(Value::as_i64)
        .collect();
    if !zeilen.is_empty() {
        deklaration.insert(
            WEITERE_WK_SUMME_KZ.to_owned(),
            Value::from(zeilen.iter().sum::<i64>()),
        );
    }
}

impl Bau<'_> {
    /// Bankverbindung: Exklusivitaet IBAN ↔ „keine Bankverbindung" (gemessen rc=610001002),
    /// Kontoinhaber E0101601 automatisch bei bestaetigter IBAN.
    fn bankverbindung(&mut self) {
        let iban =
            self.deklaration.contains_key("E0102102") || self.deklaration.contains_key("E0102603");
        let keine = self.deklaration.get("E0102002") == Some(&Value::Bool(true));
        if iban && keine {
            self.offen(
                "stammdaten_iban",
                "IBAN UND 'keine Bankverbindung' gleichzeitig bestaetigt -- checkESt lehnt den Widerspruch ab \
                 (rc=610001002). Genau eines von beidem darf gelten.",
            );
        } else if iban {
            self.deklaration
                .insert("E0101601".to_owned(), Value::Bool(true));
        }
    }

    /// Option A: KAP-Nullen beider Personen atomar unterdruecken.
    fn kap_nulldeklaration(&mut self) -> Ergebnis<()> {
        if !(kap_alle_null(self.snapshot, KAP_FELDER_A)?
            && kap_alle_null(self.snapshot, KAP_FELDER_B)?)
        {
            return Ok(());
        }
        for f in KAP_FELDER_A {
            let Some(kz) = self.bindung.get(*f).and_then(|b| kz_von(b)) else {
                continue;
            };
            // Python: `deklaration.pop(kz, None) is not None` — ein gespeichertes `None` zaehlt nicht.
            if self.deklaration.remove(kz).is_some_and(|v| !v.is_null()) {
                self.nicht(f, KAP_NULL_GRUND);
            }
        }
        for f in KAP_FELDER_B {
            let Some(kz) = suche(PARTNER_INSTANZ, f) else {
                continue;
            };
            if self.person_b.remove(kz).is_some_and(|v| !v.is_null()) {
                self.nicht(f, KAP_NULL_GRUND);
            }
        }
        Ok(())
    }

    /// Pflege-Pauschbetrag § 33b Abs. 6 EStG: Pflegegrad ausserhalb des XSD-Enums aufloesen bzw.
    /// den Block fallen lassen (`_pflegeblock`, `est_mapping.py`). Vault:
    /// `decisions/pflegegrad-ausserhalb-des-schemas-abbilden-oder-weglassen`, Punkte 1-3.
    ///
    /// E0161606 kennt laut XSD nur „2", „3" und „4" („4" = Pflegegrad 4 ODER 5); die Bindung nimmt
    /// 1..5 an, weil der Dialog auch Grad 0 und 1 annehmen muss. Drei Regeln:
    ///
    /// 1. Grad 5 → 4. Schema und Gesetz (§ 33b Abs. 6 S. 3 EStG: „Pflegegrad 4 oder 5" = 1.800 EUR)
    ///    fassen beide zusammen.
    /// 2. Grad nicht in {2, 3, 4} und kein Merkzeichen H → der ganze Block entfaellt. Es gibt
    ///    keinen Pauschbetrag, und ein Rest-Block ohne E0161606/E0161808 verletzt die ERiC-Regel
    ///    101100086 (`FelderNichtGemeinsamAngegeben`) — die ganze Erklaerung waere uneinreichbar.
    /// 3. Grad nicht in {2, 3, 4} MIT Merkzeichen H → nur E0161606 entfaellt; E0161808 traegt den
    ///    Anspruch (§ 33b Abs. 6 S. 4 EStG: 1.800 EUR ohne Grad).
    ///
    /// Die H-Pruefung ist `is True`, NICHT die Abwesenheit des Schluessels: ein bestaetigtes „Nein"
    /// steht als `Bool(false)` in der Deklaration. Wer die Abwesenheit prueft, haelt ein „Nein"
    /// fuer ein „Ja" und laesst bei Grad 1 den Rest-Block stehen — genau der Fall, den ERiC abweist.
    ///
    /// Kein `nicht_deklariert`-Eintrag fuer die entfallenden Kz: unter Grad 2 ohne H gibt es keinen
    /// Pauschbetrag, mit H traegt E0161808 ihn. Die Pruefanzeige meldete sonst in jedem der 26
    /// echten Grad-0-Faelle „nicht alle Werte stehen in der Erklaerung". Dieselbe Abwaegung wie bei
    /// der weggelassenen 0 (P9, Vault: `decisions/elster-null-in-kz-ohne-null-weglassen`, Punkt 3).
    fn pflegeblock(&mut self) {
        let grad_kz = "E0161606";
        let h_kz = "E0161808";
        if self.deklaration.get(grad_kz) == Some(&Value::Number(5.into())) {
            self.deklaration
                .insert(grad_kz.to_owned(), Value::Number(4.into()));
        }
        if matches!(
            self.deklaration.get(grad_kz),
            Some(Value::Number(n)) if [2, 3, 4].iter().any(|g| n.as_i64() == Some(*g))
        ) {
            return;
        }
        if self.deklaration.get(h_kz) == Some(&Value::Bool(true)) {
            self.deklaration.remove(grad_kz);
            return;
        }
        for kz in PFLEGE_KZ {
            self.deklaration.remove(*kz);
        }
    }

    /// § 32d Abs. 6 S. 4 EStG: der Guenstigerpruefungs-Antrag gilt fuer beide Ehegatten und muss
    /// im XML in beiden Person-Containern stehen.
    fn antrag_person_b(&mut self) {
        // `est_mapping.py:1010`: `veranl.get("wert") == "zusammen"`. „Ist das zusammen?" entscheidet
        // `Lage::veranlagung`; ein abweichender Wert (`"Zusammen"`, eine Liste) ist es auch in
        // Python nicht, deshalb zaehlt nur `Lage::Gueltig`.
        let zusammen = self.snapshot.get("veranlagung").is_some_and(|v| {
            v.zustand == Zustand::Bestaetigt
                && matches!(
                    Lage::veranlagung(Some(&v.wert)),
                    Lage::Gueltig(Veranlagung::Zusammen)
                )
        });
        let antrag_kz = self
            .bindung
            .get("kap_antrag_guenstigerpruefung")
            .and_then(|b| kz_von(b));
        if let (true, Some(kz)) = (zusammen, antrag_kz) {
            if let Some(w) = self.deklaration.get(kz).cloned() {
                self.person_b.insert(kz.to_owned(), w);
            }
        }
    }

    fn dokumentiert(&self) -> BTreeMap<String, Aggregat> {
        let mut out = BTreeMap::new();
        for (ziel, akku) in &self.agg_akku {
            if akku.is_empty() {
                continue;
            }
            let mut quell: Vec<String> = akku.iter().map(|(f, _)| f.clone()).collect();
            quell.sort();
            // ponytail: `wrapping_add` darf hier nie umbrechen. Quellen sind die vier `cent`-Felder
            // von `DOKUMENTIERT_AGGREGAT`, je als Euro (`aggregat_beitrag`, <= i64::MAX / 100);
            // die Summe liegt bei <= 3,7e17 und damit weit unter i64::MAX. Wird eines der Felder ein
            // `int` oder kommt ein fuenftes hinzu, gilt die Grenze nicht mehr: dann `checked_add`
            // und `DeklarationsFehler::Ueberlauf`, wie in `p23_gewinn`.
            let summe = akku.iter().map(|(_, w)| *w).fold(0_i64, i64::wrapping_add);
            out.insert(
                (*ziel).to_owned(),
                Aggregat {
                    summe,
                    quell_felder: quell,
                },
            );
        }
        out
    }

    /// § 23 Gewinn je Instanz = Preis − AK/HK − WK, dann an die Kz des Veraeusserungstyps.
    fn p23_gewinn(&mut self) -> Ergebnis<()> {
        const GRUPPE: &str = "p23_veraeusserung";
        let Some(gruppe) = self.instanzen.get(&GRUPPE.to_owned()) else {
            return Ok(());
        };
        // GEWOLLTE ABWEICHUNG von Python (Korrektheit vor Paritaet): Python rechnet die Differenz
        // exakt (Preis 0, AK = WK = 9e18 ct ergibt -1,8e19 ct). Ausserhalb von `i64` meldet Rust
        // `Ueberlauf` (HTTP 422) statt zu rechnen; `wrapping_sub` kippte dort das Vorzeichen
        // (+446744073709551616 ct statt eines Verlusts). Ueber HTTP heute nicht erreichbar: der
        // Guard sperrt jede § 23-Eingabe vor `deklariere()` mit 409. Darum kein Parity-Eintrag.
        let gewinne = gruppe
            .schluessel()
            .iter()
            .filter_map(|idx| gruppe.get(idx).map(|inst| (*idx, inst)))
            .map(|(idx, inst)| {
                let roh = |k: &str| inst.rohdaten.get(k).copied().unwrap_or(0);
                let gewinn = roh("p23_veraeusserungspreis")
                    .checked_sub(roh("p23_anschaffung_herstellungskosten"))
                    .and_then(|d| d.checked_sub(roh("p23_werbungskosten")))
                    .ok_or_else(|| DeklarationsFehler::Ueberlauf {
                        feld_id: format!("p23_veraeusserung__{idx}"),
                        was: "Differenz jenseits i64",
                    })?;
                Ok((idx, gewinn))
            })
            .collect::<Ergebnis<Vec<(u64, i64)>>>()?;
        for (idx, gewinn) in gewinne {
            if gewinn == 0 {
                continue;
            }
            let art_feld = if idx == 1 {
                P23_ART_FELD.to_owned()
            } else {
                format!("{P23_ART_FELD}__{idx}")
            };
            let feld_id = format!("p23_veraeusserung__{idx}");
            let Some(art) = self
                .snapshot
                .get(&art_feld)
                .filter(|a| a.zustand == Zustand::Bestaetigt)
            else {
                self.offen(&feld_id, format!("{art_feld} unbestätigt — Kz-Zweig offen"));
                continue;
            };
            if let Some(kz) =
                nachschlagen(P23_GEWINN_KZ, &art.wert).map_err(wert_fehler(&feld_id))?
            {
                let v = cent_nach_kz(
                    Cent::new(gewinn),
                    &kz_pruefen(kz).map_err(wert_fehler(&feld_id))?,
                )
                .als_json();
                self.instanz(GRUPPE, idx).felder.insert(kz.to_owned(), v);
            } else {
                let grund = format!("p23-Typ '{}' ohne Kz-Zweig", art.wert.py_str());
                self.nicht(&feld_id, grund);
            }
        }
        Ok(())
    }

    fn instanzen_ausgabe(&mut self) -> Vec<(String, Vec<AnlageInstanz>)> {
        let mut out = Vec::new();
        for (gruppe, instanzen) in std::mem::take(&mut self.instanzen).in_reihenfolge() {
            let mut sortiert = instanzen.in_reihenfolge();
            sortiert.sort_by_key(|(i, _)| *i);
            let eintraege: Vec<AnlageInstanz> = sortiert
                .into_iter()
                .filter(|(_, inst)| !inst.felder.is_empty() || !inst.dokumentiert.is_empty())
                .map(|(index, inst)| AnlageInstanz {
                    index,
                    felder: inst.felder,
                    dokumentiert: inst
                        .dokumentiert
                        .into_iter()
                        .map(|(z, mut a)| {
                            a.quell_felder.sort();
                            (z, a)
                        })
                        .collect(),
                })
                .collect();
            if !eintraege.is_empty() {
                out.push((gruppe, eintraege));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashMap};
    use std::path::Path;

    use domain::{Achsenwert, Herkunft, Konfession, PruefTiefe, Rentenart};
    use serde_json::json;

    use super::{
        deklariere, gruppe_von, iban_muster, iban_pruefziffer_gueltig, Felder, Feldtyp,
        SnapshotFeld, Value, Zustand, PARTNER_VERZWEIGUNG, VERZWEIGUNG,
    };
    use crate::tabellen::PFLEGE_KZ;
    use crate::tabellen::WERTEKODIERUNG;

    #[test]
    fn iban_pruefziffer() {
        assert!(iban_muster("DE89370400440532013000"));
        assert!(iban_pruefziffer_gueltig("DE89370400440532013000"));
        assert!(!iban_pruefziffer_gueltig("DE88370400440532013000"));
        assert!(!iban_muster("DE8937"[..3].to_string().as_str()));
    }

    /// Die Laengengrenzen 5 bis 34 Zeichen, gegen Pythons `est_mapping._IBAN_PATTERN`
    /// (`re.compile(r"^[A-Z]{2}[0-9]{2}[0-9A-Z]{1,30}$").match`, `est_mapping.py:688`). Gemessen 2026-10-04 am
    /// Python-Muster mit `"DE89" + "0" * (n - 4)`: n = 4 Nein, 5 Ja, 33 Ja, 34 Ja, 35 Nein. Die Mutationsmessung
    /// (`bescheid-elster-mutation`, D01) liess `5..=34` -> `5..34` in jedem Lauf gruen, auch mit `PARITY=1`: keine IBAN
    /// mit 34 Zeichen kam vor. Die Pruefziffer ist hier nicht Gegenstand (`iban_pruefziffer_gueltig` prueft sie getrennt).
    #[test]
    fn iban_muster_laengengrenzen_wie_python() {
        let iban = |n: usize| format!("DE89{}", "0".repeat(n - 4));
        for (n, python) in [(4, false), (5, true), (33, true), (34, true), (35, false)] {
            assert_eq!(iban_muster(&iban(n)), python, "Laenge {n}");
        }
    }

    /// Die Jahresregel selbst (`null_unzulaessig`, `est_mapping.py:198-229`). Ohne diesen Test
    /// bliebe die Regel unbelegt: `deklariere` allein prueft nur, dass sie ueberhaupt greift.
    ///
    /// Die zwei Korpusfaelle sind die Gegenprobe zur stillen Vorgabe — `eg_huge.json` traegt
    /// vz 10^38−1, `eg_neg.json` vz −5; beide muessen `ValueError` geben. Sonst ginge 10^38 als
    /// „spaeter als 2025" durch und bekaeme die Vereinigungsmenge.
    #[test]
    fn null_verbot_gilt_je_veranlagungsjahr() {
        // 1. Fehlendes Jahr: die 0 aus `store.get(...) or 0`. Fail-closed, KEINE Menge.
        let fehlt = deklariere(&Felder::new(), &HashMap::new(), 0, None).unwrap_err();
        assert_eq!(fehlt.python_klasse(), "ValueError", "{fehlt}");
        assert!(fehlt.to_string().contains("fehlt oder ist 0"), "{fehlt}");

        // 2. Unplausible Jahre: unter 2024 und die 10^38 des Korpusfalls.
        //
        // `eg_huge.json` traegt 99999999999999999999999999999999999999; auf dem Weg hierher
        // saettigt `Veranlagungsjahr::als_i64_saettigend` das auf `i64::MAX` — derselbe Zweig.
        for vz in [-5_i64, 2023, i64::MAX, i64::MIN] {
            let e = deklariere(&Felder::new(), &HashMap::new(), vz, None).unwrap_err();
            assert_eq!(e.python_klasse(), "ValueError", "vz {vz}: {e}");
            assert!(e.to_string().contains("kein Steuerjahr"), "vz {vz}: {e}");
        }

        // 3. Der Belegfall der Jahresspanne: E0106603 verbietet die 0 nur 2024.
        //
        // `rentner_pflege_weitere_personen` traegt XSD-Label „ist eine 0 einzutragen" und
        // `beispielwert: 0`; 2024 zwingt das XSD die 0 dennoch in einen Typ ohne 0. Beide Welten
        // lassen die Kz darum 2024 weg und schreiben sie 2025.
        //
        // Der Pflege-Block raeumt E0106603 ohne Pflegegrad weg (`pflegeblock`); die Begleit-Kz
        // halten ihn zusammen, wie in `pflegeblock_folgt_dem_xsd_enum`.
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        let mut felder = Felder::new();
        for (f, w) in [
            ("rentner_gepflegter_wohnsitz_inland", json!(true)),
            ("rentner_pflege_weitere_personen", json!(0)),
            ("rentner_gepflegter_idnr", json!("12345678911")),
            ("rentner_gepflegter_angaben", json!("Muster")),
            ("rentner_pflege_durch", json!("1")),
            ("rentner_pflegegrad", json!(3)),
        ] {
            felder.insert(f.to_owned(), feld(w));
        }
        let mit_null = |vz: i64| {
            deklariere(&felder, &index, vz, None)
                .unwrap()
                .deklaration
                .get("E0106603")
                .cloned()
        };
        assert_eq!(mit_null(2024), None, "2024 verbietet die 0 in E0106603");
        assert_eq!(mit_null(2025), Some(json!(0)), "2025 verlangt sie sogar");

        // 4. Unbekanntes aber plausibles Jahr: die Vereinigung — zu streng, nie zu lax.
        let vereinigung = crate::null_unzulaessig(2026).unwrap();
        for jahr in [2024_i64, 2025] {
            for kz in crate::null_unzulaessig(jahr).unwrap() {
                assert!(
                    vereinigung.contains(kz),
                    "{kz} aus {jahr} fehlt in der Vereinigung"
                );
            }
        }
        assert_eq!(mit_null(2026), None, "2026 erbt die 0-Sperre von 2024");
    }

    /// Vault: `decisions/pflegegrad-ausserhalb-des-schemas-abbilden-oder-weglassen`, Punkte 1-3.
    /// Der Pflege-Block ueber die echte Bindung: fuenf Begleit-Kz, die Regel greift ueber mehrere
    /// Kz gleichzeitig. Zwilling von `tests/test_pflegegrad_kodierung_elster.py` (Python).
    ///
    /// Die Faelle sind absichtlich vollstaendig: ein Mutant, der Regel 2 zu Regel 3 verkuerzt (nur
    /// E0161606 weg), laesst die Kz-Menge „E0161606 fehlt" erfuellt — deshalb prueft der Test die
    /// GANZE Menge des Blocks, nicht das Fehlen eines einzelnen Kz.
    #[test]
    fn pflegeblock_folgt_dem_xsd_enum() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        let begleit: [(&str, Value); 5] = [
            ("rentner_gepflegter_wohnsitz_inland", json!(true)),
            ("rentner_pflege_weitere_personen", json!(0)),
            ("rentner_gepflegter_idnr", json!("12345678911")),
            ("rentner_gepflegter_angaben", json!("Muster")),
            ("rentner_pflege_durch", json!("1")),
        ];
        let lauf = |grad: Option<i64>, h: Option<bool>| {
            let mut felder = Felder::new();
            for (f, w) in &begleit {
                felder.insert((*f).to_owned(), feld(w.clone()));
            }
            if let Some(g) = grad {
                felder.insert("rentner_pflegegrad".to_owned(), feld(json!(g)));
            }
            if let Some(h) = h {
                felder.insert("rentner_gepflegter_hilflos".to_owned(), feld(json!(h)));
            }
            // VZ 2025: bis auf E0106603 tragen die Kz dieses Blocks (E0161606, E0161808,
            // PFLEGE_KZ) in beiden Jahresmengen dieselbe 0-Sperre. E0106603 ist der Belegfall der
            // Jahresspanne und steht in `null_verbot_gilt_je_veranlagungsjahr`.
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            let da: Vec<&str> = PFLEGE_KZ
                .iter()
                .copied()
                .filter(|kz| d.deklaration.contains_key(*kz))
                .collect();
            (d.deklaration.get("E0161606").cloned(), da)
        };

        // Regel 1: Grad 5 wird auf „4" abgebildet, der Block bleibt vollstaendig.
        // E0161808 zaehlt nur mit, wenn die Probe das Merkzeichen-Feld ueberhaupt setzt.
        for grad in [2_i64, 3, 4] {
            let (kz, block) = lauf(Some(grad), None);
            assert_eq!(kz, Some(json!(grad)), "Grad {grad}");
            assert_eq!(block.len(), PFLEGE_KZ.len() - 1, "Grad {grad}: {block:?}");
            assert!(!block.contains(&"E0161808"), "Grad {grad}: {block:?}");
        }
        let (kz, block) = lauf(Some(5), None);
        assert_eq!(kz, Some(json!(4)), "Grad 5 → 4");
        assert_eq!(block.len(), PFLEGE_KZ.len() - 1);

        // Regel 2: Grad ausserhalb {2,3,4} ohne Merkzeichen H → der GANZE Block entfaellt.
        // `h: None` (Feld nie beantwortet) und `h: Some(false)` (bestaetigtes Nein) sind derselbe
        // Fall — ein bestaetigtes Nein steht als Bool(false) in der Deklaration.
        for (grad, h) in [
            (Some(1), None),
            (Some(0), None),
            (Some(1), Some(false)),
            (Some(0), Some(false)),
            (None, Some(false)),
        ] {
            let (kz, block) = lauf(grad, h);
            assert_eq!(kz, None, "Grad {grad:?}, H {h:?}");
            assert!(block.is_empty(), "Grad {grad:?}, H {h:?}: {block:?}");
        }

        // Regel 3: Grad ausserhalb {2,3,4} MIT Merkzeichen H → nur E0161606 entfaellt, der Rest
        // des Blocks bleibt (er beschreibt dieselbe gepflegte Person).
        for grad in [Some(1_i64), Some(0)] {
            let (kz, block) = lauf(grad, Some(true));
            assert_eq!(kz, None, "Grad {grad:?}, H true");
            assert_eq!(block.len(), PFLEGE_KZ.len() - 1, "Grad {grad:?}: {block:?}");
            assert!(block.contains(&"E0161808"), "Grad {grad:?}: {block:?}");
        }
    }

    /// P9 (Vault: `decisions/elster-null-in-kz-ohne-null-weglassen`): auch die Art-Verzweigung
    /// (Klasse f, g×f) schreibt keine 0 in eine Kz, deren XSD-Typ sie verbietet, auch nicht aus
    /// 50 Cent abgerundet. Zwilling von `null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet`
    /// (`tests/eigenschaften.rs`), der die `pub(crate)`-Tabellen nicht sieht. Die Kz-Menge kommt
    /// live aus dem XSD 2025, nicht aus `null_unzulaessig`.
    #[test]
    fn art_verzweigung_schreibt_keine_verbotene_null() {
        if !crate::testhilfe::schemas_da(2025) {
            return;
        }
        let xsd = crate::finde_schema(2025, "E10-{jahr}.xsd").unwrap();
        let meta = crate::kz_meta(&xsd, "E10").unwrap();
        let verbietet_null = |kz: &str| {
            meta.get(kz).is_some_and(|m| {
                m.type_name.starts_with("GanzzahlPos")
                    || (!m.enums.is_empty() && !m.enums.iter().any(|e| e == "0"))
                    || (!m.patterns.is_empty()
                        && !m.patterns.iter().any(|p| {
                            regex::Regex::new(&format!("^(?:{p})$"))
                                .unwrap()
                                .is_match("0")
                        }))
            })
        };
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        let mut durch = Vec::new();
        let mut geprueft = BTreeSet::new();
        for cfg in VERZWEIGUNG.iter().chain(PARTNER_VERZWEIGUNG) {
            let b = index[cfg.feld];
            let werte: &[i64] = if b.typ == Feldtyp::Cent {
                &[0, 50]
            } else {
                &[0]
            };
            let suffixe: &[&str] = if gruppe_von(b).is_some() {
                &["", "__2"]
            } else {
                &[""]
            };
            for i in suffixe {
                for (art, ziel) in cfg.kz.paare() {
                    if verbietet_null(ziel) {
                        geprueft.insert(ziel);
                    }
                    for w in werte {
                        let snapshot = Felder::from([
                            (format!("{}{i}", cfg.feld), feld(json!(w))),
                            (format!("{}{i}", cfg.art_feld), feld(json!(art))),
                        ]);
                        // VZ 2025: die neun §35c-Kz der Art-Verzweigung (E0241001…) tragen in
                        // beiden Jahresmengen dieselbe 0-Sperre.
                        let d = deklariere(&snapshot, &index, 2025, None).unwrap();
                        let instanzen = d.anlage_instanzen.iter().flat_map(|(_, ii)| ii);
                        durch.extend(
                            [&d.deklaration, &d.person_b]
                                .into_iter()
                                .chain(instanzen.map(|inst| &inst.felder))
                                .flatten()
                                .filter(|(kz, v)| **v == json!(0) && verbietet_null(kz))
                                .map(|(kz, _)| format!("{kz} {}{i}={w} Art {art}", cfg.feld)),
                        );
                    }
                }
            }
        }
        // Nicht-Leer-Probe (Vault: `decisions/elster-testluecken-mit-eigener-probe-schliessen`):
        // findet das XSD keine Kz mit 0-Sperre, bestuende `durch.is_empty()` leer.
        assert!(
            !geprueft.is_empty(),
            "keine Kz der Art-Verzweigung verbietet laut XSD 2025 die 0: der Test prueft nichts"
        );
        assert!(
            durch.is_empty(),
            "0 in {} Faellen durchgelassen, obwohl der XSD-Typ sie verbietet: {durch:?}",
            durch.len()
        );
    }

    /// § 32d Abs. 6 S. 4 EStG (`est_mapping.py:1008-1012`): der Guenstigerpruefungs-Antrag steht
    /// nur bei bestaetigter Zusammenveranlagung auch in `person_b`. Python vergleicht
    /// `veranl.get("wert") == "zusammen"`: nur genau dieser Text, kein anderer Typ.
    #[test]
    fn antrag_spiegelt_nur_bei_bestaetigter_zusammenveranlagung() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let kz = index["kap_antrag_guenstigerpruefung"]
            .elster_kz
            .as_ref()
            .unwrap()
            .as_str();
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value, zustand: Zustand| SnapshotFeld {
            wert: wert.into(),
            zustand,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        let gespiegelt = |veranlagung: Option<(Value, Zustand)>| {
            let mut felder = Felder::from([(
                "kap_antrag_guenstigerpruefung".to_owned(),
                feld(json!(true), Zustand::Bestaetigt),
            )]);
            if let Some((w, z)) = veranlagung {
                felder.insert("veranlagung".to_owned(), feld(w, z));
            }
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            assert!(d.deklaration.contains_key(kz), "{:?}", d.deklaration);
            match d.person_b.get(kz) {
                Some(w) => {
                    assert_eq!(Some(w), d.deklaration.get(kz));
                    true
                }
                None => false,
            }
        };
        assert!(gespiegelt(Some((json!("zusammen"), Zustand::Bestaetigt))));
        for (w, z) in [
            (json!("zusammen"), Zustand::Vorlaeufig),
            (json!("einzel"), Zustand::Bestaetigt),
            (json!("Zusammen"), Zustand::Bestaetigt),
            (json!("zusammen "), Zustand::Bestaetigt),
            (Value::Null, Zustand::Bestaetigt),
            (json!(["zusammen"]), Zustand::Bestaetigt),
            (json!({"wert": "zusammen"}), Zustand::Bestaetigt),
        ] {
            assert!(!gespiegelt(Some((w.clone(), z))), "{w} {z:?}");
        }
        assert!(!gespiegelt(None));
    }

    /// Abweichung Nr. 41: bei bestaetigter Wahl `true` und gezahlter Steuer ueber 0 steht die Steuer aufgerundet unter dem
    /// Abzugs-Kz `E0600920` und NICHT unter `E0601901`; in jedem anderen Fall (Wahl `false`, Wahl nur vorlaeufig, Steuer 0)
    /// bleibt es bei der Anrechnung, abgerundet. Das Regal (`regal.rs`) fuehrt dieses Literal als `Verhalten` mit diesem Test.
    #[test]
    fn abzug_traegt_die_steuer_unter_dem_abzugs_kz_und_nicht_als_anrechnung() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value, zustand: Zustand| SnapshotFeld {
            wert: wert.into(),
            zustand,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        // (Wahl, Zustand der Wahl, Steuer in Cent) -> (Abzug E0600920, Anrechnung E0601901)
        let faelle = [
            (true, Zustand::Bestaetigt, 70_001, Some(701), None),
            (true, Zustand::Bestaetigt, 1, Some(1), None),
            (false, Zustand::Bestaetigt, 70_001, None, Some(700)),
            (true, Zustand::Vorlaeufig, 70_001, None, Some(700)),
            (true, Zustand::Bestaetigt, 0, None, Some(0)),
        ];
        for (wahl, zustand, cent, abzug, anrechnung) in faelle {
            let felder = Felder::from([
                ("dba_abzug_statt_anrechnung".to_owned(), feld(json!(wahl), zustand)),
                (
                    "dba_gezahlte_auslaendische_steuer".to_owned(),
                    feld(json!(cent), Zustand::Bestaetigt),
                ),
            ]);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            assert_eq!(
                d.deklaration.get("E0600920"),
                abzug.map(|e| json!(e)).as_ref(),
                "Wahl {wahl} {zustand:?}, {cent} Cent: Abzug"
            );
            assert_eq!(
                d.deklaration.get("E0601901"),
                anrechnung.map(|e| json!(e)).as_ref(),
                "Wahl {wahl} {zustand:?}, {cent} Cent: Anrechnung"
            );
        }
    }

    /// Abweichung Nr. 48: ein bestaetigter Betrag ueber 0 in `ep_unfallkosten` steht aufgerundet unter `E0205406`, mit der
    /// Bezeichnung `E0205405` und der Summe `E0204803`, UND sperrt die Abgabe; bei 0 und bei einem negativen Wert steht nichts
    /// in der Deklaration, der Betrag bleibt in `nicht_deklariert` und nichts sperrt. Das Regal (`regal.rs`) fuehrt die drei Literale als `Verhalten` mit diesem Test.
    #[test]
    fn unfallkosten_stehen_ueber_null_in_der_zeile_sonstiges() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        // Cent -> (Betrag E0205406, ob die Zeile steht)
        // -500: ein negativer Wert im Store zaehlt im Bescheid als 0 (`ein_negativer_wert_im_store_zaehlt_als_null`), also keine Zeile, keine Sperre.
        for (cent, euro) in [
            (150_000, Some(1500)),
            (150_001, Some(1501)),
            (1, Some(1)),
            (0, None),
            (-500, None),
        ] {
            let felder = Felder::from([("ep_unfallkosten".to_owned(), feld(json!(cent)))]);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            let zeile = ["E0205405", "E0205406", "E0204803"].map(|kz| d.deklaration.get(kz));
            if let Some(e) = euro {
                assert_eq!(zeile[1], Some(&json!(e)), "{cent} Cent: Betrag");
                assert_eq!(zeile[2], Some(&json!(e)), "{cent} Cent: Summe");
                assert!(
                    zeile[0].and_then(Value::as_str).is_some_and(|t| t.contains("Unfall")),
                    "{cent} Cent: Bezeichnung {:?}",
                    zeile[0]
                );
                assert!(
                    d.unvollstaendig().iter().any(|e| e.feld_id == "ep_unfallkosten"),
                    "{cent} Cent: die Sperre ist weg"
                );
                assert!(d.nicht_deklariert.iter().all(|e| e.feld_id != "ep_unfallkosten"));
            } else {
                assert!(zeile.iter().all(Option::is_none), "{cent} Cent: {zeile:?}");
                assert!(d.unvollstaendig().is_empty(), "{cent} Cent: {:?}", d.unvollstaendig());
                assert!(d.nicht_deklariert.iter().any(|e| e.feld_id == "ep_unfallkosten"));
            }
        }
    }

    /// Abweichung Nr. 51: der Staat der Auslandseinkuenfte steht als Listentext unter `E0600301`, genau dann, wenn die
    /// Einkuenfte `E0601401` stehen. Drei Bindungswerte weichen vom Listentext ab. `sonstiger_staat`, ein fehlender und ein
    /// unbekannter Wert schreiben nichts und sperren (`dba_staat`); Steuer ohne Einkuenfte sperrt (`dba_gezahlte_auslaendische_steuer`);
    /// ein Staat ohne Einkuenfte und ohne Steuer steht nicht und sperrt nicht. Das Regal fuehrt das Literal mit diesem Test.
    /// Die ganze Akte und die Gegenprobe gegen die Laenderliste des Schemas stehen in `bescheid/tests/dba_staat_kz.rs`.
    #[test]
    fn der_staat_steht_als_listentext_in_e0600301_genau_mit_den_einkuenften() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        // (Staat, Einkuenfte vorhanden, Steuer vorhanden) -> (Text unter E0600301, Sperre zum Staat, Sperre zur Steuer)
        let faelle = [
            (Some("Frankreich"), true, false, Some("Frankreich"), false, false),
            (Some("Oesterreich"), true, false, Some("Österreich"), false, false),
            (Some("Tschechien"), true, true, Some("Tschechische Republik"), false, false),
            (Some("Grossbritannien"), true, false, Some("Vereinigtes Königreich"), false, false),
            (Some("sonstiger_staat"), true, false, None, true, false),
            (Some("Atlantis"), true, false, None, true, false),
            (None, true, false, None, true, false),
            (Some("Frankreich"), false, false, None, false, false),
            (Some("Frankreich"), false, true, None, false, true),
            (None, false, true, None, false, true),
        ];
        for (staat, einkuenfte, steuer, text, sperre_staat, sperre_steuer) in faelle {
            let mut felder = Felder::new();
            if let Some(s) = staat {
                felder.insert("dba_staat".to_owned(), feld(json!(s)));
            }
            if einkuenfte {
                felder.insert("dba_auslaendische_einkuenfte".to_owned(), feld(json!(500_000)));
            }
            if steuer {
                felder.insert("dba_gezahlte_auslaendische_steuer".to_owned(), feld(json!(70_000)));
            }
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            let name = format!("{staat:?}, Einkünfte {einkuenfte}, Steuer {steuer}");
            assert_eq!(
                d.deklaration.get("E0600301"),
                text.map(|t| json!(t)).as_ref(),
                "{name}: Text"
            );
            let sperre = |id: &str| d.unvollstaendig().iter().any(|e| e.feld_id == id);
            // Ein geschriebener Staat steht NICHT in `nicht_deklariert`, ein beantworteter, aber nicht geschriebener schon.
            let in_nicht = d.nicht_deklariert.iter().any(|e| e.feld_id == "dba_staat");
            assert_eq!(in_nicht, staat.is_some() && text.is_none(), "{name}: nicht_deklariert");
            assert_eq!(sperre("dba_staat"), sperre_staat, "{name}: Sperre zum Staat");
            assert_eq!(sperre("dba_gezahlte_auslaendische_steuer"), sperre_steuer, "{name}: Sperre zur Steuer");
        }
        // Ein vorlaeufiger Staat steht nicht im Kz: der Eintrag "Pflicht-Bestaetigung fehlt" kommt aus `feld`, genau einmal.
        let vorlaeufig = Felder::from([
            (
                "dba_staat".to_owned(),
                SnapshotFeld { zustand: Zustand::Vorlaeufig, ..feld(json!("Frankreich")) },
            ),
            ("dba_auslaendische_einkuenfte".to_owned(), feld(json!(500_000))),
        ]);
        let d = deklariere(&vorlaeufig, &index, 2025, None).unwrap();
        assert_eq!(d.deklaration.get("E0600301"), None, "vorläufiger Staat steht im Kz");
        let gruende: Vec<&str> =
            d.unvollstaendig().iter().filter(|e| e.feld_id == "dba_staat").map(|e| e.grund.as_str()).collect();
        assert_eq!(gruende.len(), 1, "{gruende:?}");
        assert!(gruende[0].contains("Bestätigung"), "{gruende:?}");
    }

    /// Abweichung Nr. 49: der Ring-Wert `dba_abzug_zeile_cent` steht als ZWEITE Zeile "Sonstiges" in der Gruppe `weitere_wk`,
    /// Instanz 2, aufgerundet, mit einer Bezeichnung, die § 34c nennt; `E0204803` ist die Summe der GERUNDETEN Zeilen
    /// (Unfallkosten in `deklaration`, Abzug in der Instanz). Der Wert allein sperrt nichts (die Sperre haengt am Marker
    /// `dba_abzug_statt_anrechnung`); bei 0 steht keine Zeile. Das Regal fuehrt die Literale mit diesem Test.
    #[test]
    fn abzug_steht_als_zweite_zeile_sonstiges_und_die_summe_bildet_beide_zeilen() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("berechnet"),
                pruef_tiefe: PruefTiefe::Amtlich,
                haftung: a("system"),
            }
            .into(),
        };
        assert_eq!(domain::zeichensatz::erstes_unerlaubtes_zeichen(super::DBA_ABZUG_ZEILE_TEXT), None);
        // (Unfallkosten in Cent, Abzug in Cent) -> (Zeile Unfallkosten, Zeile Abzug, Summe), volle Euro
        // -500: ein negativer Wert zaehlt wie 0 (Mutant W3 der Nachmessung in main: `betrag != 0` schriebe eine Zeile).
        let faelle = [
            (None, Some(70_001), None, Some(701), Some(701)),
            (Some(150_001), Some(70_001), Some(1501), Some(701), Some(2202)),
            (Some(150_001), None, Some(1501), None, Some(1501)),
            (Some(150_001), Some(0), Some(1501), None, Some(1501)),
            (None, Some(0), None, None, None),
            (Some(150_001), Some(-500), Some(1501), None, Some(1501)),
            (None, Some(-500), None, None, None),
        ];
        for (unfall, abzug, z_unfall, z_abzug, summe) in faelle {
            let mut felder = Felder::new();
            if let Some(c) = unfall {
                felder.insert("ep_unfallkosten".to_owned(), feld(json!(c)));
            }
            if let Some(c) = abzug {
                felder.insert("dba_abzug_zeile_cent".to_owned(), feld(json!(c)));
            }
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            let name = format!("Unfall {unfall:?}, Abzug {abzug:?}");
            assert_eq!(d.deklaration.get("E0205406"), z_unfall.map(|e| json!(e)).as_ref(), "{name}: Zeile Unfallkosten");
            assert_eq!(d.deklaration.get("E0204803"), summe.map(|e| json!(e)).as_ref(), "{name}: Summe");
            let instanzen: Vec<_> = d.anlage_instanzen.iter().filter(|(g, _)| g == "weitere_wk").collect();
            if let Some(e) = z_abzug {
                assert_eq!(instanzen.len(), 1, "{name}: genau eine Gruppe weitere_wk");
                let zeilen = &instanzen[0].1;
                assert_eq!(zeilen.len(), 1, "{name}: genau eine Zeile");
                assert_eq!(zeilen[0].index, 2, "{name}: Instanz 2");
                assert_eq!(zeilen[0].felder.get("E0205406"), Some(&json!(e)), "{name}: Betrag");
                let text = zeilen[0].felder.get("E0205405").and_then(Value::as_str).unwrap_or_default();
                assert!(text.contains("34c") && !text.contains("Unfall"), "{name}: Bezeichnung `{text}`");
                assert!(d.nicht_deklariert.iter().all(|e| e.feld_id != "dba_abzug_zeile_cent"), "{name}");
            } else {
                assert!(instanzen.is_empty(), "{name}: keine Gruppe weitere_wk");
            }
            // Bei 0 und negativ steht der Wert mit Grund in `nicht_deklariert` (Mutant W6: der Eintrag fehlte); bei einer Zeile nicht.
            let eintraege = d.nicht_deklariert.iter().filter(|e| e.feld_id == "dba_abzug_zeile_cent").count();
            assert_eq!(eintraege, usize::from(abzug.is_some_and(|c| c <= 0)), "{name}: Eintrag in nicht_deklariert");
            let sperre_unfall = unfall.is_some();
            assert_eq!(
                d.unvollstaendig().iter().any(|e| e.feld_id == "ep_unfallkosten"),
                sperre_unfall,
                "{name}: nur die Unfallkosten sperren"
            );
            assert!(d.unvollstaendig().iter().all(|e| e.feld_id != "dba_abzug_zeile_cent"), "{name}: der Ring-Wert sperrt nichts");
        }
    }

    /// Abweichung Nr. 50: Betrag, Bemessungsgrundlage und Beginnjahr des Versorgungsbezugs stehen NUR bei gesetztem Ring-Wert
    /// (`versorgung_zeile`, `versorgung_zeile_partner`) in den Zeilen 11 bis 13 der Anlage N: Person A in der Deklaration, Person B
    /// in `person_b`. Der Betrag und die Bemessungsgrundlage stehen abgerundet auf volle Euro. Ohne Ring-Wert (Bezug vor dem
    /// Alters-Gate, kein Wert) steht kein Kz, und die drei Felder bleiben mit Grund in `nicht_deklariert`. Die Sperre
    /// ([`VERSORGUNG_SPERRE`]) steht in beiden Faellen; der Ring-Wert selbst steht in keiner Liste. Das Regal (`regal.rs`) fuehrt
    /// die drei Literale als `Verhalten` mit diesem Test.
    #[test]
    fn versorgung_steht_nur_mit_dem_ring_wert_in_den_zeilen_11_bis_13() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        let kz = ["E0200801", "E0200902", "E0201307"];
        // (Person B?, Ring-Wert) -> die Felder der Person: Bezug 30.000,50 Euro, Bemessungsgrundlage 25.000,99 Euro, Beginn 2020.
        for partner in [false, true] {
            let s = if partner { "_partner" } else { "" };
            for ring in [Some(true), Some(false), None] {
                let mut felder = Felder::from([
                    (format!("versorgung_jahresrente{s}"), feld(json!(3_000_050))),
                    (format!("versorgung_bemessungsgrundlage{s}"), feld(json!(2_500_099))),
                    (format!("versorgung_beginn_jahr{s}"), feld(json!(2020))),
                    (format!("versorgung_art{s}"), feld(json!("beamtenrechtlich"))),
                    ("veranlagung".to_owned(), feld(json!("zusammen"))),
                ]);
                if let Some(wahr) = ring {
                    felder.insert(format!("versorgung_zeile{s}"), feld(json!(wahr)));
                }
                let d = deklariere(&felder, &index, 2025, None).unwrap();
                let name = format!("Person {}, Ring-Wert {ring:?}", if partner { "B" } else { "A" });
                let (da, db) = if partner { (&d.person_b, &d.deklaration) } else { (&d.deklaration, &d.person_b) };
                let steht = ring == Some(true);
                if steht {
                    assert_eq!(da.get(kz[0]), Some(&json!(30_000)), "{name}: Betrag, abgerundet");
                    assert_eq!(da.get(kz[1]), Some(&json!(25_000)), "{name}: Bemessungsgrundlage, abgerundet");
                    assert_eq!(da.get(kz[2]), Some(&json!(2020)), "{name}: Beginnjahr");
                } else {
                    assert!(kz.iter().all(|k| da.get(*k).is_none()), "{name}: ohne Ring-Wert kein Kz: {da:?}");
                }
                assert!(kz.iter().all(|k| db.get(*k).is_none()), "{name}: nichts bei der anderen Person: {db:?}");
                // Die drei Felder stehen genau dann in `nicht_deklariert`, wenn keine Zeile steht; die Art (Weiche) immer.
                let nicht = |f: &str| d.nicht_deklariert.iter().filter(|e| e.feld_id == format!("{f}{s}")).count();
                for f in ["versorgung_jahresrente", "versorgung_bemessungsgrundlage", "versorgung_beginn_jahr"] {
                    assert_eq!(nicht(f), usize::from(!steht), "{name}: {f} in nicht_deklariert");
                }
                assert_eq!(nicht("versorgung_art"), 1, "{name}: die Art ist kein Feld der Erklaerung");
                // Der Ring-Wert ist kein Feld der Erklaerung: weder nicht_deklariert noch unvollstaendig.
                let flag = format!("versorgung_zeile{s}");
                assert!(d.nicht_deklariert.iter().all(|e| e.feld_id != flag), "{name}");
                assert!(d.unvollstaendig().iter().all(|e| e.feld_id != flag), "{name}");
                // Die Sperre bleibt in jedem Fall mit Bezug (Ehegatte: Zusammenveranlagung ist bestaetigt).
                let gesperrt = d.unvollstaendig().iter().filter(|e| e.feld_id == format!("versorgung_jahresrente{s}")).count();
                assert_eq!(gesperrt, 1, "{name}: die Sperre bleibt");
            }
            // Ein nur VORLAEUFIGER Ring-Wert gibt keine Zeile: der Ring setzt ihn nur aus bestaetigten Feldern.
            let mut felder = Felder::from([
                (format!("versorgung_jahresrente{s}"), feld(json!(3_000_050))),
                (format!("versorgung_bemessungsgrundlage{s}"), feld(json!(2_500_099))),
                (format!("versorgung_beginn_jahr{s}"), feld(json!(2020))),
                ("veranlagung".to_owned(), feld(json!("zusammen"))),
            ]);
            let mut flag = feld(json!(true));
            flag.zustand = Zustand::Vorlaeufig;
            felder.insert(format!("versorgung_zeile{s}"), flag);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            let da = if partner { &d.person_b } else { &d.deklaration };
            assert!(kz.iter().all(|k| da.get(*k).is_none()), "Person {}: vorlaeufiger Ring-Wert: {da:?}", if partner { "B" } else { "A" });
        }
    }

    /// Abweichung Nr. 53: der Grund der Abgabe-Sperre bei Versorgungsbezuegen sagt nicht mehr, ein `checkESt`-Lauf habe die
    /// Zeilen 11 bis 13 "noch nie angenommen" (Nr. 50 hat sie offline angenommen, rc=0), und nimmt die Rente mit Leistungsmitteilung
    /// aus. Die Sperre selbst bleibt (Abweichung Nr. 42): der Text nennt sie und das amtliche Formular.
    #[test]
    fn die_versorgung_sperre_sagt_nicht_noch_nie_angenommen_und_nimmt_die_leistungsmitteilung_aus() {
        let text = super::VERSORGUNG_SPERRE;
        assert!(!text.contains("noch nie angenommen"), "{text}");
        assert!(
            text.contains("Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung mit Leistungsmitteilung gehört nicht in diese Zeilen"),
            "{text}"
        );
        assert!(text.starts_with("Versorgungsbezüge über 0 Euro") && text.contains("gesperrt") && text.contains("Formular"), "{text}");
        assert!(text.contains("noch nicht freigegeben und deshalb gesperrt"), "{text}");
        assert!(!text.contains("nicht gesperrt"), "{text}");
    }

    /// Python `dict.get(wert)` an allen drei Aufrufstellen der Enum-Felder: Konfession (`feld`),
    /// Rentenart als Instanz (`instanz_feld`) und beim Partner (`verzweigung`). Drei
    /// Nicht-Treffer bleiben getrennt: Text ohne Schluessel und Null/Bool/Zahl geben keinen Code,
    /// Liste und Objekt sind `TypeError` (nicht hashbar).
    #[test]
    fn enum_felder_schlagen_nach_wie_python_dict_get() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: &Value| SnapshotFeld {
            wert: wert.clone().into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        let lauf = |paare: &[(&str, Value)]| {
            let felder: Felder = paare
                .iter()
                .map(|(f, w)| ((*f).to_owned(), feld(w)))
                .collect();
            deklariere(&felder, &index, 2025, None)
        };
        // Der Instanzpfad (`instanz_feld`) beginnt bei `__2`: Instanz 1 ist die Basis-`feld_id` ohne
        // Suffix und geht den Feldpfad; `x__1` ist keine Instanz (Entscheidung 2026-10-03).
        let rente = |art: &Value| {
            [
                ("rentner_jahresrente__2", json!(1_200_000)),
                ("rentner_renten_art__2", art.clone()),
            ]
        };
        let partner = |art: &Value| {
            [
                ("rentner_jahresrente_partner", json!(1_200_000)),
                ("rentner_renten_art_partner", art.clone()),
            ]
        };

        for (wert, code) in [
            ("keine", Some("11")),
            ("evangelisch", Some("02")),
            ("roemisch-katholisch", Some("03")),
            ("andere", None),
            ("Evangelisch", None),
        ] {
            let d = lauf(&[("kist_konfession", json!(wert))]).unwrap();
            assert_eq!(
                d.deklaration.get("E0100402"),
                code.map(|c| json!(c)).as_ref(),
                "{wert}"
            );
        }
        for (art, kz) in [
            ("gesetzliche_rente", Some("E1800301")),
            ("berufsstaendische_versorgung", Some("E1800301")),
            ("private_basisrente", Some("E1800301")),
            ("private_leibrente", Some("E1801601")),
            ("sonstige_leibrente", Some("E1803102")),
            ("andere", None),
        ] {
            let kz: Vec<&str> = kz.into_iter().collect();
            let d = lauf(&rente(&json!(art))).unwrap();
            let inst: Vec<&String> = d
                .anlage_instanzen
                .iter()
                .flat_map(|(_, ii)| ii)
                .flat_map(|i| i.felder.keys())
                .collect();
            assert_eq!(inst, kz, "Instanz {art}");
            let d = lauf(&partner(&json!(art))).unwrap();
            assert_eq!(d.person_b.keys().collect::<Vec<_>>(), kz, "Partner {art}");
        }
        for (wert, typ_fehler) in [
            (Value::Null, false),
            (json!(true), false),
            (json!(3), false),
            (json!(["keine"]), true),
            (json!({"gesetzliche_rente": 1}), true),
        ] {
            for paare in [
                vec![("kist_konfession", wert.clone())],
                rente(&wert).to_vec(),
                partner(&wert).to_vec(),
            ] {
                let feld_id = paare[0].0;
                match lauf(&paare) {
                    Err(e) => assert!(
                        typ_fehler && e.python_klasse() == "TypeError",
                        "{feld_id} {wert}: {e}"
                    ),
                    Ok(d) => {
                        assert!(!typ_fehler, "{feld_id} {wert}: kein TypeError");
                        assert!(
                            d.nicht_deklariert.iter().any(|e| e.feld_id == feld_id),
                            "{feld_id} {wert}"
                        );
                    }
                }
            }
        }
    }

    /// Die Domain-Enums sind genau die `enum_werte` der Bindung, in deren Reihenfolge, fuer
    /// Person A und den Partner. `ArtKz::paare` und damit die Kz-Prueflinge folgen dieser Folge.
    #[test]
    fn enum_typen_sind_die_bindungswerte() {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let werte = |f: &str| index[f].enum_werte.clone().unwrap();
        for f in ["kist_konfession", "kist_konfession_partner"] {
            assert_eq!(werte(f), Konfession::ALLE.map(Konfession::als_str), "{f}");
        }
        for f in ["rentner_renten_art", "rentner_renten_art_partner"] {
            assert_eq!(werte(f), Rentenart::ALLE.map(Rentenart::als_str), "{f}");
        }
    }

    /// Klasse i: jeder Code, den `WERTEKODIERUNG` schreiben kann, steht in der Werteliste seiner Kz
    /// im Schema. Der Zweig sitzt im Crate, weil die Tabelle `pub(crate)` ist (Vault:
    /// `decisions/typpruefung-bindung-gegen-schema-bekommt-in-rust-einen-test-kein-build-skript`);
    /// `tests/bindungs_typ_vs_xsd_typ.rs` prueft denselben Weg ueber `deklariere`. Ohne Schema rot,
    /// ausser `TAXGRAPH_OHNE_XSD=1`.
    #[test]
    fn wertekodierung_codes_stehen_in_der_werteliste() {
        if !crate::testhilfe::schemas_da(2025) {
            return;
        }
        let pfad = crate::finde_schema(2025, "E10-{jahr}.xsd").unwrap();
        let meta = crate::kz_meta(&pfad, "E10").unwrap();
        let mut codes = 0;
        for w in WERTEKODIERUNG {
            let enums = &meta
                .get(w.kz)
                .unwrap_or_else(|| panic!("{}: Kz {} nicht im Schema", w.feld, w.kz))
                .enums;
            assert!(!enums.is_empty(), "{}: Kz {} ohne Werteliste", w.feld, w.kz);
            for k in Konfession::ALLE {
                if let Some(code) = (w.code)(k) {
                    codes += 1;
                    assert!(
                        enums.iter().any(|e| e == code),
                        "{}: {k:?} -> {code:?}, Kz {} erlaubt nur {enums:?}",
                        w.feld,
                        w.kz
                    );
                }
            }
        }
        // KONTROLLE: zwei Felder mit je drei Codes (keine, evangelisch, roemisch-katholisch);
        // „andere" hat keinen. Ein Lauf ohne Code haette nichts geprueft.
        assert_eq!(codes, 6, "gepruefte Codes");
    }

    /// § 23-Gewinn = Preis − AK/HK − WK ausserhalb von `i64`: Fehler statt Umbruch.
    ///
    /// GEWOLLTE ABWEICHUNG von Python, das exakt rechnet. Mit `wrapping_sub` ergab Preis 0 bei
    /// AK = WK = 9e18 ct **+446744073709551616** ct statt eines Verlusts von 1,8e19 ct (das
    /// Vorzeichen kippte). Ueber HTTP ist die Stelle nicht erreichbar (der Guard sperrt § 23 mit
    /// 409, bericht h8-casts), darum der Test auf Funktionsebene.
    #[test]
    fn p23_gewinn_ausserhalb_i64_ist_ein_fehler_statt_umbruch() {
        use super::DeklarationsFehler;

        const NEUN_E18: i64 = 9_000_000_000_000_000_000;
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert: wert.into(),
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        // Je Verkauf `(Suffix, Preis, AK/HK, WK)`; Suffix "" ist Instanz 1, "__2" Instanz 2.
        let rechne = |verkaeufe: &[(&str, i64, i64, i64)]| {
            let mut felder = Felder::new();
            for (suffix, preis, ak, wk) in verkaeufe {
                for (f, w) in [
                    ("p23_veraeusserungs_typ", json!("grundstueck")),
                    ("p23_veraeusserungspreis", json!(preis)),
                    ("p23_anschaffung_herstellungskosten", json!(ak)),
                    ("p23_werbungskosten", json!(wk)),
                ] {
                    felder.insert(format!("{f}{suffix}"), feld(w));
                }
            }
            deklariere(&felder, &index, 2025, None)
        };
        let ueberlauf = |verkaeufe: &[(&str, i64, i64, i64)], erwartet_feld: &str| {
            let e = rechne(verkaeufe).unwrap_err();
            assert_eq!(
                e,
                DeklarationsFehler::Ueberlauf {
                    feld_id: erwartet_feld.to_owned(),
                    was: "Differenz jenseits i64",
                },
                "{verkaeufe:?}"
            );
            assert_eq!(e.python_klasse(), "OverflowError");
        };

        // 1. KONTROLLE: der Normalfall kommt an, sonst bewiese der Fehler unten nichts.
        //    20 000 000 − 10 000 000 − 100 000 ct = 9 900 000 ct = 99 000 EUR, Kz E0306801.
        let d = rechne(&[("", 20_000_000, 10_000_000, 100_000)]).unwrap();
        let (gruppe, instanzen) = &d.anlage_instanzen[0];
        assert_eq!(gruppe, "p23_veraeusserung");
        assert_eq!(instanzen[0].felder.get("E0306801"), Some(&json!(99_000)));

        // 2. Der Fall des Berichts: Preis 0, AK = WK = 9e18 ct (exakt −1,8e19 ct).
        ueberlauf(&[("", 0, NEUN_E18, NEUN_E18)], "p23_veraeusserung__1");
        // 3. Nach oben: Preis i64::MAX, AK −1 (Preis − AK = i64::MAX + 1).
        ueberlauf(&[("", i64::MAX, -1, 0)], "p23_veraeusserung__1");
        // 4. Erst die ZWEITE Subtraktion laeuft ueber: 0 − i64::MAX − 2 liegt unter i64::MIN.
        ueberlauf(&[("", 0, i64::MAX, 2)], "p23_veraeusserung__1");
        // 5. Die Instanz im Fehler ist die, die ueberlaeuft (Instanz 1 ist in Ordnung).
        ueberlauf(
            &[
                ("", 20_000_000, 10_000_000, 100_000),
                ("__2", 0, NEUN_E18, NEUN_E18),
            ],
            "p23_veraeusserung__2",
        );
        // 6. GRENZE: 0 − i64::MAX − 1 = i64::MIN passt noch; kein Fehler, keine Entscheidung ueber
        //    den Wert (die Grenze selbst ist der Beleg, dass `checked_sub` nicht zu eng ist).
        assert!(rechne(&[("", 0, i64::MAX, 1)]).is_ok());
    }
}
