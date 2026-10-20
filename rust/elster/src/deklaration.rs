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

/// Die Bezeichnung in `E0205405`. Das Schema (`NAEnum_BEWERBUNGSKOSTEN_3`) kennt keine Aufzaehlung; `checkESt` nimmt diesen Text an
/// (VZ 2025, `bescheid/tests/checkest_blockmatrix.rs::zeilen_nr48_nr49_gegen_echtes_checkest`, rc=0).
const UNFALLKOSTEN_TEXT: &str = "Unfallkosten Arbeitsweg";

/// Nur Rust (Abweichung Nr. 93): die Beitraege zu Berufsverbaenden (§ 9 Abs. 1 S. 3 Nr. 3 `EStG`, Zeile 53 der Anlage N,
/// `N/Wk/Berufsverb`, `E10-2025.xsd`). Das Feld traegt nur den Betrag ([`BERUFSVERBAND_BETRAG_KZ`], aufgerundet auf volle Euro,
/// `ABZUGS_KZ`; Person A ueber die Bindung, Person B ueber `PARTNER_INSTANZ`). Bezeichnung und Summe leitet
/// [`berufsverband_zeile`] daraus ab, fuer beide Personen.
const BERUFSVERBAND_TEXT_KZ: &str = "E0204001";
const BERUFSVERBAND_SUMME_KZ: &str = "E0204002";
const BERUFSVERBAND_BETRAG_KZ: &str = "E0204003";

/// Die zwei Kz, die [`berufsverband_zeile`] aus dem Betrag ableitet (kein Bindungs-Kz): die Abdeckungstests (`abdeckung.rs`)
/// kennen sie als Transform-Ziele.
#[cfg(test)]
pub(crate) const BERUFSVERBAND_ZIEL_KZ: [&str; 2] = [BERUFSVERBAND_TEXT_KZ, BERUFSVERBAND_SUMME_KZ];

/// Die Bezeichnung in `E0204001`. Das Schema kennt dort nur `String_MinL1_MaxL999`; der Text sagt, was die Zeile ist, nicht welchen
/// Verband der Nutzer meint (das Feld fragt den Namen nicht, er waere ein Merkmal nach Art. 9 DSGVO).
const BERUFSVERBAND_TEXT: &str = "Beiträge zu Berufsverbänden";

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

/// Der Grund der Abgabe-Sperre bei gewaehltem Abzug und gezahlter Steuer ueber 0 (`dba_abzug_statt_anrechnung` ohne Kz), wenn der
/// Bescheid den Abzug NICHT rechnet (bei DBA-Freistellung seit Abweichung Nr. 35 gibt es keinen Abzug; ohne Auslandseinkuenfte
/// ueber 0 auch nicht). Die Erklaerung trug die Steuer unter `E0600920` und, seit Abweichung Nr. 49, als zweite Zeile "Sonstiges"
/// der Anlage N (`Weitere_Wk/Sonst`, [`DBA_ABZUG_ZEILE`]); sie meldete den Abzug dann ohne Zeile. `checkESt` hat Zeile und
/// Anlage AUS angenommen, die Sperre fiel am 2026-10-10 fuer den getragenen Fall ([`Bau::abzug_zeile_getragen`]). Sie nutzt die
/// WEITE Bedingung ([`Bau::abzug_gewaehlt`]) abzueglich des getragenen Falls.
const DBA_ABZUG_SPERRE: &str = "Abzug der ausländischen Steuer gewählt: In deinem Fall rechnet der Bescheid keinen Abzug, zum Beispiel weil ein Abkommen die Einkünfte freistellt oder keine Auslandseinkünfte über 0 stehen. Die Erklärung würde den Abzug trotzdem melden. Die Abgabe ist deshalb gesperrt. Antworte „nein“ (Anrechnung), wenn du abgeben willst, oder trage den Abzug im Formular selbst ein.";

/// Nur Rust (Abweichung Nr. 51): der Staat der Auslandseinkuenfte (Anlage AUS, Zeile "1. Staat"), die Einkuenfte aus diesem
/// Staat (Kz kommt aus der Bindung, Anlage AUS Zeile 7) und die Tabelle der Listentexte. Das Feld hat kein Kz in der Bindung:
/// der Wert ist ein Bindungswert, das Schema will den Namen aus seiner Laenderliste ([`Bau::dba_staat`]).
pub(crate) const DBA_STAAT: &str = "dba_staat";
const DBA_EINKUENFTE: &str = "dba_auslaendische_einkuenfte";

/// Nur Rust (Abweichung Nr. 58): die Folgefrage zu `sonstiger_staat`. Ihr Wert ist selbst der Listentext (182 Eintraege der
/// Laenderliste, die 15 benannten stehen bei [`DBA_STAAT`]); [`Bau::dba_staat`] schreibt ihn nach [`STAAT_KZ`], wenn der Wert
/// in den `enum_werte` der Bindung steht. Die Gegenprobe gegen die Liste des Schemas steht in `bescheid/tests/dba_staat_kz.rs`.
pub(crate) const DBA_STAAT_SONSTIGER: &str = "dba_staat_sonstiger";
const SONSTIGER_STAAT: &str = "sonstiger_staat";

/// Das Kz "aus dem Staat / Spezial-Investmentfonds" (Anlage AUS, `Staat_Spez_InvFonds/E0600301`). Der Typ ist Klartext aus der
/// Laenderliste des Schemas (197 Eintraege, nur in der Dokumentation, keine Aufzaehlung). `checkESt` prueft den Text nicht
/// (26 Texte gemessen, alle rc=0): die Gegenprobe gegen die Liste steht in `bescheid/tests/dba_staat_kz.rs`.
pub(crate) const STAAT_KZ: &str = "E0600301";

/// Abweichung Nr. 54: Instanz-Felder des Privatverkaufs, deren Kz vom Wert des Art-Felds ([`P23_ART_FELD`]) abhaengt.
/// Gemessen am `checkESt`-Spike AK0: beim Grundstueck sind Bezeichnung, beide Daten und der Gewinn Pflicht, beim anderen
/// Wirtschaftsgut nur die Art (E0307101) und der Gewinn; die Daten E0307201/E0307202 sind dort freiwillig, `TaxGraph` schreibt sie.
pub(crate) const P23_ARTFELDER: &[(&str, &[(&str, &str)])] = &[
    (
        "p23_bezeichnung",
        &[("grundstueck", "E0306111"), ("anderes_wg", "E0307101")],
    ),
    (
        "p23_anschaffungsdatum",
        &[("grundstueck", "E0306201"), ("anderes_wg", "E0307201")],
    ),
    (
        "p23_veraeusserungsdatum",
        &[("grundstueck", "E0306202"), ("anderes_wg", "E0307202")],
    ),
];

/// Abweichung Nr. 54 (Anlage SO, Zeile 36): die Kz eines selbst bewohnten Grundstuecks innerhalb der Frist: der Haken "zu eigenen
/// Wohnzwecken", der Zeitraum (`TT.MM.JJJJ-TT.MM.JJJJ`) und die Flaeche in m². Der Gewinn daneben ist 0 und steht ohne Einzelbetraege
/// (`checkESt`-Fall Gx7, rc=0; `bescheid/tests/checkest_blockmatrix.rs`).
pub(crate) const P23_EIGENNUTZUNG_KZ: [&str; 3] = ["E0306301", "E0306302", "E0306303"];

/// Abweichung Nr. 54: was die Nutzung eines Grundstuecks zu eigenen Wohnzwecken fuer die Erklaerung bedeutet ([`Bau::p23_nutzung`]).
enum Nutzung {
    /// Keine Eigennutzung bestaetigt, oder kein Grundstueck: Gewinn und Kz wie bei jedem Verkauf.
    Keine,
    /// Eigennutzung, aber dieses Feld fehlt oder ist unbestaetigt: die Zeile 36 ist offen.
    Fehlt(&'static str),
    /// Eigennutzung mit Wohnflaeche und Zeitraum: die Kz aus [`P23_EIGENNUTZUNG_KZ`] mit ihren Werten.
    Kz([(&'static str, Value); 3]),
}

/// Abweichung Nr. 54: Rechenfelder des Privatverkaufs ohne eigenes Kz (Eigennutzung samt Wohnflaeche und Zeitraum, taeglicher Gebrauch,
/// Einkunftsquelle, Gebaeude der letzten zehn Jahre). Sie entscheiden, ob ein Verkauf steuerpflichtig ist (`bescheid::privatverkauf`
/// nimmt die Verkaeufe ausserhalb der Erklaerung vorher aus dem Snapshot); `p23_eigennutzung` und die drei Felder dahinter liest
/// ausserdem [`Bau::p23_gewinn`] fuer die Kz in [`P23_EIGENNUTZUNG_KZ`].
pub(crate) const P23_RECHENFELDER: &[&str] = &[
    "p23_eigennutzung",
    "p23_wohnflaeche",
    "p23_eigennutzung_von",
    "p23_eigennutzung_bis",
    "p23_taeglicher_gebrauch",
    "p23_einkunftsquelle",
    "p23_gebaeude_in_frist",
];

/// Abweichung Nr. 55: die Gruppe der Rente mit Leistungsmitteilung (Anlage R-AV/bAV), ihre drei Betragsfelder (Zeilen 4, 10, 15), die
/// zwei Beginn-Felder mit dem Betragsfeld, zu dem sie gehoeren (Zeilen 12, 16), und die zwei Rechenfelder ohne Kz. `ERiC` verlangt
/// Betrag und Beginn gemeinsam (Spike Nr. 55, Fall B und K1): beide stehen nur, wenn der Betrag mindestens einen vollen Euro traegt.
pub(crate) const RAV_BAV_GRUPPE: &str = "rente_av_bav";
pub(crate) const RAV_BAV_BETRAGSFELDER: &[&str] = &[
    "rav_bav_nr1_betrag",
    "rav_bav_nr4_betrag",
    "rav_bav_nr5_betrag",
];
pub(crate) const RAV_BAV_BEGINNFELDER: &[(&str, &str)] = &[
    ("rav_bav_nr4_beginn", "rav_bav_nr4_betrag"),
    ("rav_bav_nr5_beginn", "rav_bav_nr5_betrag"),
];
pub(crate) const RAV_BAV_RECHENFELDER: &[&str] = &["rav_bav_andere_nummern", "rav_bav_basisrente"];

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

/// Der Grund der Abgabe-Sperre bei Auslandseinkuenften ohne benennbaren Staat: ein fehlender oder ein unbekannter Wert von
/// `dba_staat`. Die Erklaerung braucht den Namen aus der amtlichen Laenderliste; ohne ihn lehnt `checkESt` die Akte ab.
const STAAT_SPERRE: &str = "Auslandseinkünfte ohne Staat: Die Erklärung braucht den Staat, aus dem die Einkünfte stammen, mit seinem Namen aus der amtlichen Länderliste. Für eine fehlende oder unbekannte Angabe gibt es diesen Namen nicht. Die Abgabe ist deshalb gesperrt. Wähle den Staat in der Liste, oder wähle „Anderer Staat“ und nenne ihn bei der Folgefrage.";

/// Der Grund der Abgabe-Sperre bei `sonstiger_staat` ohne gueltige Folgefrage (Abweichung Nr. 58): die Erklaerung braucht den Namen des
/// Staates, und nur der Nutzer kennt ihn. Das Feld der Sperre ist die Folgefrage, nicht `dba_staat`: dort steht die Antwort schon.
const STAAT_SONSTIGER_SPERRE: &str = "Auslandseinkünfte aus „Anderer Staat“: Die Erklärung braucht den Namen des Staates aus der amtlichen Länderliste. Die Abgabe ist deshalb gesperrt, bis du ihn nennst. Beantworte die Frage, aus welchem Staat die Einkünfte stammen. Steht der Staat nicht in der Liste, trage die Anlage AUS im amtlichen Formular selbst ein.";

/// Der Grund der Abgabe-Sperre bei auslaendischer Steuer (Anrechnung oder Abzug) ohne Einkuenfte aus dem Staat: die Anlage AUS
/// verlangt zur Steuer die Einkuenfte, auch mit dem Wert 0; `checkESt` lehnt die Akte sonst mit rc=610001002 ab.
const STEUER_OHNE_EINKUENFTE_SPERRE: &str = "Ausländische Steuer ohne Einkünfte: Die Anlage AUS verlangt zur Steuer die Einkünfte aus dem Staat, auch mit dem Wert 0. Die Abgabe ist deshalb gesperrt. Trage die Einkünfte aus diesem Staat ein, oder lösche die Steuer, wenn du ohne Auslandseinkünfte abgibst.";

/// Abweichung Nr. 61: die fuenf Vorsorge-Felder des Ehegatten und die Summen-Kz, die sie mit Person A teilen. Die Zeilen 44 bis 48
/// der Anlage Vorsorgeaufwand tragen die Beitraege der Eheleute zusammen (Anleitung, Beispiel: "Die Summe dieser Betraege ... tragen
/// sie in Zeile 46 ein"); das Schema kennt dort keine Person (`A_B_LP`, nur "Summe"). Die Felder des Ehegatten tragen kein eigenes Kz.
const VORSORGE_PARTNER: [(&str, &str); 5] = [
    ("vorsorge_arbeitslosenversicherung_partner", "E2001403"),
    ("vorsorge_erwerbsunfaehigkeit_partner", "E2001503"),
    ("vorsorge_unfall_haftpflicht_partner", "E2001803"),
    ("vorsorge_rv_alt_mit_ueberschuss_partner", "E2001903"),
    ("vorsorge_rv_alt_ohne_ueberschuss_partner", "E2002003"),
];

/// Abweichung Nr. 61: Felder, die der Bescheid in den Gewinn oder in die Kapitaleinkuenfte rechnet und die kein Kz tragen. Die
/// Erklaerung nannte den Betrag bisher 0 (`E0800302`/`E0803202`, `E1900701`) und sperrte nicht. Jetzt sperrt die Abgabe mit dem
/// Betrag im Klartext (Vault `die-abgabe-sperrt-wo-die-steuerrechnung-einen-wert-rechnet-den-die-erklaerung-nicht-traegt`).
/// Bauen liesse sich keiner: die Anleitungen zu Anlage G, S, EUeR und KAP liegen nicht im Quellenkorpus, und die Beteiligungszeile
/// verlangt Finanzamt und Steuernummer der Gesellschaft (`bindung_an_gesamt.yaml`, `einkuenfte_gewinn`).
/// Die Betriebseinnahmen-Seite: der Bescheid rechnet daraus den Gewinn, sobald eines davon ungleich 0 ist (`laufender_gewinn`).
const GEWINN_EUER: [(&str, &str); 4] = [
    ("betriebseinnahmen", "Betriebseinnahmen"),
    ("sonstige_betriebsausgaben", "sonstige Betriebsausgaben"),
    ("afa_jahresbetrag", "Abschreibungen (AfA)"),
    ("gwg_anschaffungskosten_netto", "Anschaffungskosten eines geringwertigen Wirtschaftsguts"),
];
/// Gewinnanteil und Verguetungen aus einer Beteiligung (§ 15 Abs. 1 S. 1 Nr. 2 `EStG`), Person A.
const GEWINN_MITU: [(&str, &str); 4] = [
    ("gewinnanteil", "Gewinnanteil aus einer Beteiligung"),
    ("verguetung_taetigkeit", "Vergütung für Tätigkeit in einer Beteiligung"),
    ("verguetung_darlehen", "Vergütung für Darlehen an eine Beteiligung"),
    ("verguetung_ueberlassung", "Vergütung für Überlassung an eine Beteiligung"),
];
/// Dieselben Felder des Ehegatten (nur bei Zusammenveranlagung im Bescheid).
const GEWINN_MITU_PARTNER: [(&str, &str); 4] = [
    ("gewinnanteil_partner", "Gewinnanteil des Ehegatten aus einer Beteiligung"),
    ("verguetung_taetigkeit_partner", "Vergütung des Ehegatten für Tätigkeit in einer Beteiligung"),
    ("verguetung_darlehen_partner", "Vergütung des Ehegatten für Darlehen an eine Beteiligung"),
    ("verguetung_ueberlassung_partner", "Vergütung des Ehegatten für Überlassung an eine Beteiligung"),
];
/// Das Feld des fertigen Gewinns (Direktwert); seine Kz je Art stehen in der Verzweigung (`GEWINN` in `tabellen.rs`).
const GEWINN_DIREKTWERT: &str = "einkuenfte_gewinn";
/// Sonstige Kapitalgewinne (Topf ohne Kz), Person A und Ehegatte.
const KAP_SONSTIGE: [(&str, &str, bool); 2] = [
    ("kap_gewinn_sonstige", "Sonstige Kapitalgewinne", false),
    ("kap_gewinn_sonstige_partner", "Sonstige Kapitalgewinne des Ehegatten", true),
];

/// Nur Rust (Abweichung Nr. 42): der Jahresbetrag der Versorgungsbezuege von Person A und, bei Zusammenveranlagung, des
/// Ehegatten (Abweichung Nr. 33). Beide Felder tragen kein Kz.
const VERSORGUNG: &str = "versorgung_jahresrente";
const VERSORGUNG_PARTNER: &str = "versorgung_jahresrente_partner";

/// Der Grund der Abgabe-Sperre bei einem bestaetigten Versorgungsbezug ueber 0, den die Erklaerung NICHT in den Zeilen 11 bis 13
/// der Anlage N traegt (`VERSORGUNG`, `VERSORGUNG_PARTNER` ohne Kz; der Ring-Wert [`VERSORGUNG_ZEILE`] steht nicht auf `true`):
/// die Altersgrenze zu Beginn des Bezugs ist nicht erreicht (der Bezug steckt dann nur als Arbeitslohn in Zeile 5; seit Abweichung
/// Nr. 65 sperrt das nur noch, wenn der Ring-Wert [`VERSORGUNG_ALTERSGRENZE`] fehlt, etwa ohne Veranlagungsjahr), oder Betrag
/// oder Bemessungsgrundlage liegen unter 1 Euro (das Schema nimmt die Zeilen 11 und 12 erst ab 1 Euro). Bis Abweichung Nr. 64
/// hielt die Sperre (Abweichung Nr. 42) JEDEN Bezug ueber 0 zurueck; Julius hat sie am 2026-10-12 fuer die Faelle gehoben, die die
/// Zeilen 11 bis 13 voll tragen. Seit Nr. 53 nimmt der Text die Rente mit Leistungsmitteilung aus (§ 22 Nr. 5 `EStG`, kein
/// Versorgungsbezug).
const VERSORGUNG_SPERRE: &str = "Versorgungsbezüge über 0 Euro: Die Erklärung kann diesen Bezug nicht in die Zeilen 11 bis 13 der Anlage N eintragen. Das ist der Fall, wenn die Altersgrenze zu Beginn des Bezugs nicht erreicht ist oder wenn Betrag oder Bemessungsgrundlage unter 1 Euro liegen. Die Abgabe ist dafür gesperrt. Eine Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung mit Leistungsmitteilung gehört nicht in diese Zeilen. Trage die Versorgungsbezüge im amtlichen Formular selbst ein.";

/// Der Grund der Abgabe-Sperre (Abweichung Nr. 65) bei einem Versorgungsbezug, dessen Alters-Gate (Alter bei Beginn) nicht erfuellt
/// ist, dessen Person die Altersgrenze (63 Jahre, bei Schwerbehinderung 60) aber im Veranlagungsjahr oder davor erreicht hat
/// (Ring-Wert [`VERSORGUNG_ALTERSGRENZE`] steht auf [`ALTERSGRENZE_DARUEBER`]). Ab dem Zeitpunkt der Vollendung ist der Bezug ein
/// Versorgungsbezug (§ 19 Abs. 2 S. 2 Nr. 2 `EStG`), dem Zeile 11 und der Versorgungsfreibetrag in der Erklaerung fehlen. Das Feld
/// des Geburtsjahres, das die Grenze entscheidet, kann auch unlesbar sein; auch dann sperrt die Abgabe. Seit Abweichung Nr. 83 nennt
/// der Text die Frage nach dem Grad der Behinderung (ein Grad unter 50 gibt die Grenze 63), bedingt, denn die Frage steht nicht immer
/// in der Liste.
const VERSORGUNG_SPERRE_ALTERSGRENZE: &str = "Versorgungsbezüge über 0 Euro: Der Bezug begann vor der Altersgrenze (63 Jahre, bei Schwerbehinderung 60 Jahre), aber die Person hat die Altersgrenze im Veranlagungsjahr oder davor erreicht. Von da an ist der Bezug ein Versorgungsbezug, und Zeile 11 sowie der Versorgungsfreibetrag fehlen in der Erklärung. Die Abgabe ist dafür gesperrt. Steht die Frage nach dem Grad der Behinderung noch offen, beantworte sie: Mit einem Grad unter 50 gilt die Grenze 63, und die Sperre entfällt, solange du 63 noch nicht erreicht hast. Sonst trage die Versorgungsbezüge im amtlichen Formular selbst ein.";

/// Nur Rust (Abweichung Nr. 65): die Ring-Werte der Altersgrenze, je Person, Text. `bescheid::deklaration::mit_ring_werten` setzt sie
/// nur, wenn das Alters-Gate (Alter bei Beginn) nicht erfuellt ist: [`ALTERSGRENZE_LOHN`], wenn die Person die Altersgrenze erst
/// nach dem Veranlagungsjahr erreicht (der Bezug ist das ganze Jahr Arbeitslohn in Zeile 5 und steht in keiner Zeile 11 bis 13),
/// [`ALTERSGRENZE_DARUEBER`], wenn sie die Grenze im oder vor dem Jahr erreicht hat. Beide Felder tragen kein Kz.
const VERSORGUNG_ALTERSGRENZE: &str = "versorgung_altersgrenze";
const VERSORGUNG_ALTERSGRENZE_PARTNER: &str = "versorgung_altersgrenze_partner";
const ALTERSGRENZE_LOHN: &str = "lohn";
const ALTERSGRENZE_DARUEBER: &str = "darueber";

/// Die vier Ring-Werte des Versorgungsbezugs ohne Kz (Zeilen und Altersgrenze, je Person): kein Feld der Erklaerung.
fn ist_versorgung_ring_wert(feld_id: &str) -> bool {
    matches!(
        feld_id,
        VERSORGUNG_ZEILE | VERSORGUNG_ZEILE_PARTNER | VERSORGUNG_ALTERSGRENZE | VERSORGUNG_ALTERSGRENZE_PARTNER
    )
}

/// Der Grund der Abgabe-Sperre (Abweichung Nr. 64) bei einem Versorgungsbezug, der im Veranlagungsjahr oder danach beginnt. Im
/// Beginnjahr kann der Bezug unterjaehrig sein; fuer diesen Fall verlangt der Vordruck in Zeile 14 den ersten und den
/// letzten Zahlungsmonat (Nr. 31 der Lohnsteuerbescheinigung), und die Erklaerung traegt diese Zeile nicht. Ein Beginn nach dem
/// Veranlagungsjahr lehnt `ERiC` ab (Regel `Arbeitslohn_ab08_5`); der Bescheid sperrt ihn schon vorher (`VersorgungsbeginnNachVz`).
const VERSORGUNG_SPERRE_BEGINN: &str = "Versorgungsbezüge über 0 Euro: Der Bezug beginnt im Veranlagungsjahr oder später. Bei einem Beginn im Veranlagungsjahr verlangt der Vordruck in Zeile 14 den ersten und den letzten Zahlungsmonat, und die Erklärung trägt diese Zeile nicht. Ein Beginn nach dem Veranlagungsjahr lehnt ELSTER ab. Die Abgabe ist dafür gesperrt. Trage die Versorgungsbezüge im amtlichen Formular selbst ein.";

/// Die Gruende der Abgabe-Sperre (Abweichung Nr. 66), wenn der Nutzer eine der vier Rueckfragen zum Versorgungsbezug mit Ja beantwortet
/// ([`VERSORGUNG_RUECKFRAGEN`]). Die Eingaben verraten diese Faelle nicht, und die Erklaerung traegt die Zeilen dafuer nicht: ein
/// zweiter Bezug (zweite Spalte der Zeilen 11 bis 16, § 19 Abs. 2 S. 6 `EStG`), Sterbegeld und Verwandtes (Zeile 15, Nr. 32 der
/// Lohnsteuerbescheinigung), Bezuege fuer mehrere Kalenderjahre (Zeile 16, Nr. 9; ohne die Zeile fehlt die Fuenftelregelung, § 34
/// Abs. 1 `EStG`) und ein Bezug, der nicht das ganze Jahr lief (Zeile 14, Nr. 31; § 19 Abs. 2 S. 12 `EStG`). Jeder Text nennt sein
/// eigenes Merkmal, die Sperre und das amtliche Formular.
const VERSORGUNG_SPERRE_ZWEITER_BEZUG: &str = "Versorgungsbezüge über 0 Euro: Angegeben sind zwei verschiedene Versorgungsbezüge. Der Vordruck hat dafür eine zweite Spalte in den Zeilen 11 bis 16, und die Erklärung trägt nur den ersten Bezug. Die Abgabe ist dafür gesperrt. Trage die Versorgungsbezüge im amtlichen Formular selbst ein.";
const VERSORGUNG_SPERRE_STERBEGELD: &str = "Versorgungsbezüge über 0 Euro: Angegeben sind Sterbegeld, eine Kapitalauszahlung, eine Abfindung oder eine Nachzahlung von Versorgungsbezügen (Nummer 32 der Lohnsteuerbescheinigung). Der Vordruck verlangt dafür Zeile 15, und die Erklärung trägt diese Zeile nicht. Die Abgabe ist dafür gesperrt. Trage die Versorgungsbezüge im amtlichen Formular selbst ein.";
const VERSORGUNG_SPERRE_MEHRERE_JAHRE: &str = "Versorgungsbezüge über 0 Euro: Angegeben sind Versorgungsbezüge für mehrere Kalenderjahre (Nummer 9 der Lohnsteuerbescheinigung). Der Vordruck verlangt dafür Zeile 16, und die Erklärung trägt diese Zeile nicht; ohne sie fehlt die ermäßigte Besteuerung (Fünftelregelung). Die Abgabe ist dafür gesperrt. Trage die Versorgungsbezüge im amtlichen Formular selbst ein.";
const VERSORGUNG_SPERRE_NICHT_GANZJAEHRIG: &str = "Versorgungsbezüge über 0 Euro: Angegeben ist, dass die Versorgungsbezüge nicht in allen zwölf Monaten gezahlt wurden (Nummer 31 der Lohnsteuerbescheinigung). Der Vordruck verlangt dafür in Zeile 14 den ersten und den letzten Zahlungsmonat, und die Erklärung trägt diese Zeile nicht. Die Abgabe ist dafür gesperrt. Trage die Versorgungsbezüge im amtlichen Formular selbst ein.";

/// Eine Rueckfrage zum Versorgungsbezug (Abweichung Nr. 66) oder zum Lohn (Abweichung Nr. 85, 86): die Felder von Person A und Ehegatte
/// (Ja/Nein, kein Kz), der Grund der Sperre bei Ja und das Merkmal des Falls fuer den Grund der fehlenden Antwort (Abweichung Nr. 79).
struct Rueckfrage {
    person_a: &'static str,
    partner: &'static str,
    text: &'static str,
    /// Ein Nebensatz nach "ob": steht im Grund einer unbeantworteten Frage ([`rueckfrage_unbeantwortet_grund`]).
    merkmal: &'static str,
}

/// Der Grund der Abgabe-Sperre (Abweichung Nr. 79), wenn der Nutzer eine der vier Rueckfragen zum Versorgungsbezug nicht mit Ja oder
/// Nein beantwortet hat. Die Eingaben verraten den Fall nicht, die Erklaerung traegt die Zeilen dafuer nicht: ohne Antwort wuesste
/// niemand, ob die Erklaerung vollstaendig ist. Der Text nennt die fehlende Antwort, das Merkmal des Falls und die Sperre, und er ist nie der
/// Text des Ja: ein Nutzer, der "Ja" nie gesagt hat, soll nicht lesen, er habe es gesagt.
fn rueckfrage_unbeantwortet_grund(merkmal: &str) -> String {
    format!(
        "Versorgungsbezüge über 0 Euro: Die Frage, ob {merkmal}, ist nicht mit Ja oder Nein beantwortet. Die Eingaben zeigen diesen Fall nicht, und die Erklärung trägt die Zeilen dafür nicht. Die Abgabe ist gesperrt, bis du die Frage beantwortest. Antwortest du Ja, bleibt die Abgabe gesperrt, und du trägst die Versorgungsbezüge im amtlichen Formular selbst ein."
    )
}

/// Die vier Rueckfragen, in der Reihenfolge, in der ihre Gruende in `unvollstaendig` stehen. Normalfall ist Nein: ein bestaetigtes Nein
/// sperrt nicht. Ein bestaetigtes Ja sperrt mit dem Text des Falls, eine fehlende, vorlaeufige oder nicht boolesche Antwort mit dem
/// Text der fehlenden Antwort (Abweichung Nr. 79, vorher zaehlte sie wie Nein; [`Bau::versorgung_rueckfragen`]).
const VERSORGUNG_RUECKFRAGEN: [Rueckfrage; 4] = [
    Rueckfrage {
        person_a: "versorgung_zweiter_bezug",
        partner: "versorgung_zweiter_bezug_partner",
        text: VERSORGUNG_SPERRE_ZWEITER_BEZUG,
        merkmal: "zwei verschiedene Versorgungsbezüge gezahlt wurden",
    },
    Rueckfrage {
        person_a: "versorgung_sterbegeld",
        partner: "versorgung_sterbegeld_partner",
        text: VERSORGUNG_SPERRE_STERBEGELD,
        merkmal: "Sterbegeld, eine Kapitalauszahlung, eine Abfindung oder eine Nachzahlung von Versorgungsbezügen gezahlt wurde",
    },
    Rueckfrage {
        person_a: "versorgung_mehrere_jahre",
        partner: "versorgung_mehrere_jahre_partner",
        text: VERSORGUNG_SPERRE_MEHRERE_JAHRE,
        merkmal: "die Versorgungsbezüge Zahlungen für mehrere Kalenderjahre enthalten",
    },
    Rueckfrage {
        person_a: "versorgung_nicht_ganzjaehrig",
        partner: "versorgung_nicht_ganzjaehrig_partner",
        text: VERSORGUNG_SPERRE_NICHT_GANZJAEHRIG,
        merkmal: "die Versorgungsbezüge nicht in allen zwölf Monaten gezahlt wurden",
    },
];

/// Die Gruende der Abgabe-Sperre (Abweichung Nr. 85), wenn der Nutzer eine der Rueckfragen zum Lohn mit Ja beantwortet
/// ([`LOHN_RUECKFRAGEN`]). Die Eingaben verraten diese Faelle nicht (sie stecken im Bruttoarbeitslohn oder fehlen ganz), und die
/// Erklaerung traegt die Zeilen dafuer nicht: Arbeitslohn fuer mehrere Jahre und Entschaedigungen, z. B. Abfindungen (Zeile 17 der
/// Anlage N, Nr. 10 der Lohnsteuerbescheinigung; die Fuenftelregelung wendet die Veranlagung an), und steuerfreier Arbeitslohn nach dem
/// Auslandstaetigkeitserlass (Zeile 22, je Staat und Person eine Anlage N-AUS; der Progressionsvorbehalt wirkt auf den Steuersatz).
/// Quelle: `sources/bfinv/anlage_n_2025.txt` Zeilen 17 und 22, `sources/bfinv/anleitungen/anl_n_2025.txt` (Zeile 4 bis 17, Zeile 21 bis
/// 24). Jeder Text nennt sein eigenes Merkmal, die Sperre und das amtliche Formular.
const LOHN_SPERRE_MEHRERE_JAHRE: &str = "Arbeitslohn: Angegeben ist Arbeitslohn für mehrere Jahre oder eine Entschädigung, zum Beispiel eine Abfindung (Nummer 10 der Lohnsteuerbescheinigung). Der Vordruck verlangt dafür Zeile 17, und die Erklärung trägt diese Zeile nicht; ohne sie fehlt die ermäßigte Besteuerung (Fünftelregelung). Die Abgabe ist dafür gesperrt. Trage den Arbeitslohn im amtlichen Formular selbst ein.";
const LOHN_SPERRE_AUSLANDSTAETIGKEIT: &str = "Arbeitslohn: Angegeben ist steuerfreier Arbeitslohn nach dem Auslandstätigkeitserlass. Der Vordruck verlangt dafür Zeile 22 und je Staat und Person eine Anlage N-AUS, und die Erklärung trägt beides nicht. Der steuerfreie Arbeitslohn wirkt sich auf den persönlichen Steuersatz aus (Progressionsvorbehalt). Die Abgabe ist dafür gesperrt. Trage den Arbeitslohn im amtlichen Formular selbst ein.";

/// Der Grund der Abgabe-Sperre (Abweichung Nr. 86), wenn der Nutzer die Rueckfrage zu Leistungen des Arbeitgebers mit Ja beantwortet
/// ([`LOHN_RUECKFRAGEN`]): steuerfreie und pauschal besteuerte Leistungen fuer Fahrten zwischen Wohnung und Arbeitsstaette (Zeile 51 der
/// Anlage N, Nr. 17 und 18 der Lohnsteuerbescheinigung), steuerfreie Leistungen fuer Fahrtkosten bei Auswaertstaetigkeit (Zeile 71), bei
/// doppelter Haushaltsfuehrung (Zeile 34 der Anlage N-DHF) und die steuerfreie Sammelbefoerderung (Grossbuchstabe F, Zeilen 32, 40 und
/// 48). Die Erklaerung traegt keine dieser Angaben, und die Anrechnung auf Werbungskosten rechnet sie nicht (nur Verpflegung ist
/// belegt, § 9 Abs. 4a Satz 11). Quelle: `sources/bfinv/anlage_n_2025.txt`, `sources/bfinv/dhf_2025.txt`,
/// `sources/bfinv/anleitungen/anl_n_2025.txt`, `sources/bfinv/anleitungen/anl_ndhf_2025.txt`.
const LOHN_SPERRE_ERSTATTUNGEN: &str = "Arbeitslohn: Angegeben sind Leistungen des Arbeitgebers für Fahrten, für eine doppelte Haushaltsführung oder für eine Sammelbeförderung (Nummer 17 und 18 der Lohnsteuerbescheinigung, Großbuchstabe F). Der Vordruck verlangt dafür die Zeile 51 und die Zeile 71 der Anlage N, die Zeile 34 der Anlage N-DHF und die Angabe zur Sammelbeförderung (Zeilen 32, 40 und 48), und die Erklärung trägt diese Angaben nicht. Die Abgabe ist dafür gesperrt. Trage diese Angaben im amtlichen Formular selbst ein.";

/// Der Grund der Abgabe-Sperre (Abweichung Nr. 85, 86), wenn der Nutzer eine der Rueckfragen zum Lohn nicht mit Ja oder Nein beantwortet
/// hat; das Gegenstueck zu [`rueckfrage_unbeantwortet_grund`] (Nr. 79) fuer den Lohn. Er nennt die fehlende Antwort, das Merkmal des
/// Falls und die Sperre, und er ist nie der Text des Ja. Der Schluss sagt "die Angaben dazu", nicht "den Arbeitslohn": die Leistungen des
/// Arbeitgebers sind kein Arbeitslohn.
fn lohn_rueckfrage_unbeantwortet_grund(merkmal: &str) -> String {
    format!(
        "Arbeitslohn: Die Frage, ob {merkmal}, ist nicht mit Ja oder Nein beantwortet. Die Eingaben zeigen diesen Fall nicht, und die Erklärung trägt die Zeilen dafür nicht. Die Abgabe ist gesperrt, bis du die Frage beantwortest. Antwortest du Ja, bleibt die Abgabe gesperrt, und du trägst die Angaben dazu im amtlichen Formular selbst ein."
    )
}

/// Die Rueckfragen zum Lohn (Abweichung Nr. 85, 86), in der Reihenfolge, in der ihre Gruende in `unvollstaendig` stehen. Sie gelten je
/// Person, deren bestaetigter Bruttoarbeitslohn ueber 0 liegt ([`Bau::lohn_rueckfragen`]); genau dann steht die Frage in der Fragenliste
/// (Bindung `feld_bedingung`). Wie bei den Rueckfragen zum Versorgungsbezug ([`VERSORGUNG_RUECKFRAGEN`]) sperrt ein bestaetigtes Ja mit
/// dem Text des Falls und jede andere Lage ausser einem bestaetigten Nein mit dem Text der fehlenden Antwort.
const LOHN_RUECKFRAGEN: [Rueckfrage; 3] = [
    Rueckfrage {
        person_a: "lohn_mehrere_jahre",
        partner: "lohn_mehrere_jahre_partner",
        text: LOHN_SPERRE_MEHRERE_JAHRE,
        merkmal: "der Arbeitslohn Zahlungen für mehrere Jahre oder eine Entschädigung enthält, zum Beispiel eine Abfindung",
    },
    Rueckfrage {
        person_a: "lohn_auslandstaetigkeit",
        partner: "lohn_auslandstaetigkeit_partner",
        text: LOHN_SPERRE_AUSLANDSTAETIGKEIT,
        merkmal: "steuerfreier Arbeitslohn nach dem Auslandstätigkeitserlass gezahlt wurde",
    },
    Rueckfrage {
        person_a: "lohn_erstattungen",
        partner: "lohn_erstattungen_partner",
        text: LOHN_SPERRE_ERSTATTUNGEN,
        merkmal: "der Arbeitgeber Fahrtkosten oder Kosten einer doppelten Haushaltsführung erstattet oder bezuschusst oder dich zur Arbeit befördert hat",
    },
];

/// Der Bruttoarbeitslohn von Person A und des Ehegatten (Kz E0200201, Person B in der zweiten Anlage N): bei einem bestaetigten Betrag
/// ueber 0 gelten die Rueckfragen [`LOHN_RUECKFRAGEN`].
const BRUTTOARBEITSLOHN: &str = "bruttoarbeitslohn";
const BRUTTOARBEITSLOHN_PARTNER: &str = "bruttoarbeitslohn_partner";

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

/// Ein Cent-Betrag als Euro mit Komma, fuer die Gruende der Abgabe-Sperre (`1234,50`, `-5,00`).
fn euro_text(cent: i64) -> String {
    let betrag = cent.unsigned_abs();
    let vorzeichen = if cent < 0 { "-" } else { "" };
    format!("{vorzeichen}{},{:02}", betrag / 100, betrag % 100)
}

/// Der Grund der Abgabe-Sperre bei einem Betrag aus [`GEWINN_EUER`] ohne Gewinn als fertigen Betrag (Abweichung Nr. 61).
fn gewinn_euer_grund(label: &str, cent: i64) -> String {
    format!(
        "Gewinn aus Einnahmen und Ausgaben: Du hast {label} mit {} Euro angegeben, aber keinen Gewinn als fertigen Betrag. Die \
         Steuerrechnung rechnet den Gewinn aus diesen Angaben, die Erklärung würde 0 Euro nennen. Die Abgabe ist deshalb \
         gesperrt. Trage den Gewinn als fertigen Betrag ein, oder trage die Anlage EÜR im amtlichen Formular selbst ein.",
        euro_text(cent)
    )
}

/// Der Grund der Abgabe-Sperre bei Gewinnanteil oder Verguetung aus einer Beteiligung (Abweichung Nr. 61).
fn gewinn_mitu_grund(label: &str, cent: i64) -> String {
    format!(
        "Beteiligung an einer Gesellschaft: Du hast {label} mit {} Euro angegeben. Die Steuerrechnung zählt den Betrag, die \
         Erklärung hat dafür keine Zeile und würde ihn weglassen; sie bräuchte Finanzamt und Steuernummer der Gesellschaft. Die \
         Abgabe ist deshalb gesperrt. Trage die Beteiligung in Anlage G oder Anlage S im amtlichen Formular selbst ein.",
        euro_text(cent)
    )
}

/// Der Grund der Abgabe-Sperre bei sonstigen Kapitalgewinnen im Einzeltopf (Abweichung Nr. 61).
fn kap_topf_grund(label: &str, cent: i64) -> String {
    format!(
        "{label}: Du hast {} Euro angegeben. Die Steuerrechnung versteuert den Betrag, die Anlage KAP trägt in ihrer Zeile für \
         Kapitalerträge aber nur den Gesamtbetrag und würde 0 Euro nennen. Die Abgabe ist deshalb gesperrt. Trage die \
         Kapitalerträge im amtlichen Formular selbst ein.",
        euro_text(cent)
    )
}

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
    /// Das Veranlagungsjahr, wie `deklariere` es bekam (Abweichung Nr. 64: ein Versorgungsbezug mit Beginn im Veranlagungsjahr
    /// oder danach sperrt, [`Bau::versorgung_sperre`]).
    vz: i64,
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

    /// `ep_unfallkosten` OHNE Einzel-Kz (Abweichung Nr. 28, Nr. 48). Ueber 0 steht der Betrag in der Zeile "Sonstiges" der
    /// Anlage N ([`UNFALLKOSTEN_BETRAG_KZ`], aufgerundet auf volle Euro; Bezeichnung [`UNFALLKOSTEN_TEXT_KZ`]; die Summe
    /// [`WEITERE_WK_SUMME_KZ`] bildet [`weitere_wk_summe`] aus allen Zeilen). `checkESt` hat die Zeile angenommen (VZ 2025, Julius
    /// 2026-10-10 gab die Sperre frei), die Abgabe ist nicht gesperrt. Bei 0 und leer steht nichts im XML und der Grund in
    /// `nicht_deklariert`. Die zweite Zeile, der Abzug nach § 34c Abs. 2 `EStG`, schreibt [`Bau::dba_abzug_zeile`] (Abweichung Nr. 49).
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

    /// Abweichung Nr. 61: ein Vorsorge-Feld des Ehegatten ([`VORSORGE_PARTNER`])? `true`, wenn ja. Bei bestaetigter
    /// Zusammenveranlagung schreibt [`Bau::vorsorge_partner`] seinen Betrag nach der Feldschleife in die gemeinsame Summe; sonst
    /// bleibt er draussen, wie im Bescheid (der Ehegatte zaehlt nur bei Zusammenveranlagung), und der Grund steht in `nicht_deklariert`.
    fn vorsorge_partner_feld(&mut self, feld_id: &str) -> bool {
        if !VORSORGE_PARTNER.iter().any(|(f, _)| *f == feld_id) {
            return false;
        }
        if !self.zusammen_bestaetigt() {
            self.nicht(
                feld_id,
                "Der Betrag des Ehegatten zaehlt nur bei bestaetigter Zusammenveranlagung; sonst steht er nicht in der Erklaerung",
            );
        }
        true
    }

    /// Abweichung Nr. 61: die Summe der Zeilen 44 bis 48 der Anlage Vorsorgeaufwand ist die Summe BEIDER Ehegatten. Person A hat
    /// ihren gerundeten Betrag in der Schleife geschrieben; hier kommt der gerundete Betrag des Ehegatten dazu (Aufwand:
    /// Einzelposten aufrunden, Summe aus den Posten, Vault `aufwand-einzelposten-aufrunden-summe-aus-posten`). Nur bei bestaetigter
    /// Zusammenveranlagung und bestaetigtem Betrag; ein vorlaeufiger Betrag steht schon in `unvollstaendig` (`feld`).
    fn vorsorge_partner(&mut self) -> Ergebnis<()> {
        if !self.zusammen_bestaetigt() {
            return Ok(());
        }
        for (feld_id, kz) in VORSORGE_PARTNER {
            let (Some(sfeld), Some(b)) = (self.snapshot.get(feld_id), self.bindung.get(feld_id)) else {
                continue;
            };
            if sfeld.zustand != Zustand::Bestaetigt {
                continue;
            }
            let mut zeile = BTreeMap::new();
            schreibe_kz(&mut zeile, kz, &sfeld.wert, Some(b.typ), self.null_kz)
                .map_err(wert_fehler(feld_id))?;
            let Some(partner) = zeile.get(kz).and_then(Value::as_i64) else {
                continue;
            };
            let eigen = self.deklaration.get(kz).and_then(Value::as_i64).unwrap_or(0);
            // Beide Betraege sind gerundete Euro aus i64-Cent (hoechstens i64::MAX / 100): die Summe laeuft nicht ueber.
            self.deklaration
                .insert(kz.to_owned(), Value::from(eigen.saturating_add(partner)));
        }
        Ok(())
    }

    /// Alle bestaetigten Werte ungleich 0 zu `basis` und seinen Instanzen (`basis__2` ...), in Cent: `(feld_id, cent)`.
    /// Ein vorlaeufiger Wert steht schon in `unvollstaendig` (`feld`); ein Wert, der keine Ganzzahl ist, kommt nicht vor (der Store
    /// weist ihn bei einem Cent-Feld beim Speichern ab).
    fn betraege_ungleich_null(&self, basis: &str) -> Vec<(String, i64)> {
        self.snapshot
            .iter()
            .filter(|(feld_id, _)| {
                feld_id.as_str() == basis || parse_instanz(feld_id).is_some_and(|(b, _)| b == basis)
            })
            .filter(|(_, s)| s.zustand == Zustand::Bestaetigt)
            .filter_map(|(feld_id, s)| match py::int(&s.wert) {
                Ok(n) if n != 0 => Some((feld_id.clone(), n)),
                _ => None,
            })
            .collect()
    }

    /// Abweichung Nr. 61: die Abgabe sperrt, wo der Bescheid Gewinn oder Kapitaleinkuenfte aus Feldern rechnet, die kein Kz tragen
    /// ([`GEWINN_EUER`], [`GEWINN_MITU`], [`GEWINN_MITU_PARTNER`], [`KAP_SONSTIGE`]). Die Erklaerung nennt den Betrag sonst als 0.
    ///
    /// 1. Betriebseinnahmen-Seite ohne Gewinn als fertigen Betrag: der Bescheid rechnet den Gewinn aus ihr, das Kz traegt 0. Steht
    ///    der Gewinn als fertiger Betrag (ungleich 0), gilt er; ob er zur Aufschluesselung passt, prueft der Bescheid
    ///    (`gewinn_quelle_offen`).
    /// 2. Gewinnanteil und Verguetungen aus einer Beteiligung: der Bescheid rechnet sie in den Gewinn, die Erklaerung hat dafuer
    ///    keine Zeile. Immer, auch neben einem fertigen Gewinn.
    /// 3. Sonstige Kapitalgewinne: der Bescheid versteuert den Topf, `E1900701` traegt nur den Aggregat-Wert.
    ///
    /// Der Ehegatte zaehlt wie im Bescheid nur bei bestaetigter Zusammenveranlagung. ponytail: die Betriebseinnahmen-Seite wird an
    /// den Eingabefeldern erkannt, nicht am Rechenergebnis des Bescheids; ein Gewinn von genau 0 aus ihr sperrt darum auch, und
    /// der Nutzer traegt dann den Gewinn 0 als fertigen Betrag ein. Upgrade-Pfad: Ring-Wert des Bescheid-Gewinns und Vergleich hier.
    fn gewinn_sperren(&mut self) {
        let gewinn_steht = VERZWEIGUNG
            .iter()
            .filter(|v| v.feld == GEWINN_DIREKTWERT)
            .filter_map(|v| match v.kz {
                ArtKz::Text(tabelle) => Some(tabelle),
                ArtKz::Rente(_) => None,
            })
            .flat_map(|tabelle| tabelle.iter().map(|(_, kz)| *kz))
            .any(|kz| {
                self.deklaration
                    .get(kz)
                    .is_some_and(|v| v.as_f64().is_none_or(|x| x != 0.0))
            });
        let zusammen = self.zusammen_bestaetigt();
        let mut offen: Vec<(String, String)> = Vec::new();
        if !gewinn_steht {
            for (basis, label) in GEWINN_EUER {
                for (feld_id, cent) in self.betraege_ungleich_null(basis) {
                    offen.push((feld_id, gewinn_euer_grund(label, cent)));
                }
            }
        }
        let mitu = GEWINN_MITU
            .iter()
            .chain(GEWINN_MITU_PARTNER.iter().filter(|_| zusammen));
        for (basis, label) in mitu {
            for (feld_id, cent) in self.betraege_ungleich_null(basis) {
                offen.push((feld_id, gewinn_mitu_grund(label, cent)));
            }
        }
        for (basis, label, partner) in KAP_SONSTIGE {
            if partner && !zusammen {
                continue;
            }
            for (feld_id, cent) in self.betraege_ungleich_null(basis) {
                offen.push((feld_id, kap_topf_grund(label, cent)));
            }
        }
        for (feld_id, grund) in offen {
            self.offen(&feld_id, grund);
        }
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

    /// Was der Ring zur Altersgrenze der Person sagt (Abweichung Nr. 65): der bestaetigte Text des Ring-Werts
    /// [`VERSORGUNG_ALTERSGRENZE`] bzw. [`VERSORGUNG_ALTERSGRENZE_PARTNER`], `None`, wenn er fehlt, nicht bestaetigt oder kein Text ist.
    fn altersgrenze(&self, partner: bool) -> Option<&str> {
        let flag = if partner {
            VERSORGUNG_ALTERSGRENZE_PARTNER
        } else {
            VERSORGUNG_ALTERSGRENZE
        };
        self.snapshot
            .get(flag)
            .filter(|s| s.zustand == Zustand::Bestaetigt)
            .and_then(|s| match &s.wert {
                PyWert::Text(t) => Some(t.as_str()),
                _ => None,
            })
    }

    /// Hat der Nutzer die Rueckfrage `feld` mit Ja beantwortet (Abweichung Nr. 66)? Nur ein BESTAETIGTES `true` zaehlt.
    fn rueckfrage_ja(&self, feld: &str) -> bool {
        self.snapshot
            .get(feld)
            .is_some_and(|s| s.zustand == Zustand::Bestaetigt && matches!(s.wert, PyWert::Bool(true)))
    }

    /// Hat der Nutzer die Rueckfrage `feld` mit Nein beantwortet (Abweichung Nr. 79)? Nur ein BESTAETIGTES `false` zaehlt: Nein ist der
    /// Normalfall und die einzige Antwort, die nicht sperrt. Eine fehlende, vorlaeufige oder nicht boolesche Antwort ist keine Antwort.
    fn rueckfrage_nein(&self, feld: &str) -> bool {
        self.snapshot
            .get(feld)
            .is_some_and(|s| s.zustand == Zustand::Bestaetigt && matches!(s.wert, PyWert::Bool(false)))
    }

    /// Die Rueckfragen der Person, die den Bezug nicht frei geben (Abweichung Nr. 66, Nr. 79): je Frage hoechstens ein Grund in
    /// `unvollstaendig`, unter dem Feld der Rueckfrage (so findet der Nutzer die Antwort, die er aendern kann). Ein bestaetigtes Ja
    /// gibt den Text des Falls, ein bestaetigtes Nein keinen Grund, jede andere Lage (fehlend, vorlaeufig, nicht boolesch) den Text der
    /// fehlenden Antwort: bis zur Antwort ist die Erklaerung nicht vollstaendig. Sie gelten unabhaengig davon, ob die Zeilen 11 bis 13
    /// den Bezug tragen: auch ein sonst getragener Bezug braucht bei Ja die Zeilen 14 bis 16 oder die zweite Spalte, die die Erklaerung
    /// nicht hat. Der Aufrufer ruft das nur fuer einen bestaetigten Bezug ueber 0 einer Person, die zaehlt ([`Bau::versorgung`]); genau
    /// dann steht die Frage in der Fragenliste (Bindung `feld_bedingung`), die Sperre verlangt also nie eine Antwort auf eine verborgene Frage.
    fn versorgung_rueckfragen(&mut self, partner: bool) {
        self.rueckfragen_pruefen(&VERSORGUNG_RUECKFRAGEN, partner, rueckfrage_unbeantwortet_grund);
    }

    /// Die Rueckfragen `liste` der Person (Abweichung Nr. 66, Nr. 79, Nr. 85, Nr. 86): ein bestaetigtes Ja gibt den Text des Falls unter dem Feld
    /// der Frage, ein bestaetigtes Nein keinen Grund, jede andere Lage (fehlend, vorlaeufig, nicht boolesch) den Text, den `unbeantwortet`
    /// aus dem Merkmal der Frage baut. Die Rueckfragen zum Versorgungsbezug und zum Lohn teilen diese eine Pruefung.
    fn rueckfragen_pruefen(&mut self, liste: &[Rueckfrage], partner: bool, unbeantwortet: fn(&str) -> String) {
        for r in liste {
            let feld = if partner { r.partner } else { r.person_a };
            if self.rueckfrage_ja(feld) {
                self.offen(feld, r.text);
            } else if !self.rueckfrage_nein(feld) {
                self.offen(feld, unbeantwortet(r.merkmal));
            }
        }
    }

    /// Die Rueckfragen zum Lohn (Abweichung Nr. 85, 86): je Person, deren bestaetigter Bruttoarbeitslohn ueber 0 liegt, die Fragen aus
    /// [`LOHN_RUECKFRAGEN`]. Der Ehegatte zaehlt nur bei bestaetigter Zusammenveranlagung, wie im Bescheid. Genau dann steht die Frage in
    /// der Fragenliste (Bindung `feld_bedingung` am Bruttoarbeitslohn), die Sperre verlangt also nie eine Antwort auf eine verborgene Frage.
    /// Der Aufruf steht nach der Feldschleife, weil der Lohn kein eigener Zweig der Schleife ist.
    fn lohn_rueckfragen(&mut self) {
        for partner in [false, true] {
            let lohn = if partner { BRUTTOARBEITSLOHN_PARTNER } else { BRUTTOARBEITSLOHN };
            let hat_lohn = self
                .snapshot
                .get(lohn)
                .is_some_and(|s| s.zustand == Zustand::Bestaetigt && py::int(&s.wert).is_ok_and(|cent| cent > 0));
            if hat_lohn && (!partner || self.zusammen_bestaetigt()) {
                self.rueckfragen_pruefen(&LOHN_RUECKFRAGEN, partner, lohn_rueckfrage_unbeantwortet_grund);
            }
        }
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

    /// Der Grund, aus dem ein Versorgungsbezug ueber 0 die Abgabe sperrt, oder `None`, wenn die Erklaerung ihn voll traegt
    /// (Abweichung Nr. 64). Voll getragen heisst: der Ring-Wert ([`Bau::versorgung_zeile_gilt`]) steht auf `true` (Zeilen 11 bis
    /// 13 stehen im XML) UND der Bezug beginnt vor dem Veranlagungsjahr. Ohne diesen Ring-Wert gilt: [`ALTERSGRENZE_LOHN`] (Alters-
    /// Gate nicht erfuellt, die Person erreicht die Altersgrenze erst nach dem Veranlagungsjahr, Abweichung Nr. 65) ist frei, wie
    /// ein getragener Bezug, aber ohne Zeilen; [`ALTERSGRENZE_DARUEBER`] gibt [`VERSORGUNG_SPERRE_ALTERSGRENZE`]; sonst
    /// [`VERSORGUNG_SPERRE`]. Bei einem Beginn im Veranlagungsjahr oder danach (oder einem Beginnjahr, das sich nicht als
    /// bestaetigte ganze Zahl lesen laesst) gilt in jedem freien Fall [`VERSORGUNG_SPERRE_BEGINN`]. ponytail: ein Beginn im
    /// Veranlagungsjahr gilt als unterjaehrig (Zeile 14), auch wenn der
    /// Bezug am 1. Januar begann. Ein Bezug, der im Veranlagungsjahr ENDET (Zeile 14, Sterbegeld Zeile 15, mehrere
    /// Kalenderjahre Zeile 16, zweiter Bezug je Person), ist aus den Eingaben nicht erkennbar: seit Abweichung Nr. 66 fragt die
    /// Erklaerung danach ([`Bau::versorgung_rueckfragen`], getrennt von dieser Sperre), und ein Ja sperrt. Upgrade-Pfad: die Zeilen 14
    /// bis 16 und die zweite Spalte bauen; dann entfaellt die Frage zum jeweiligen Fall.
    fn versorgung_sperre(&self, partner: bool) -> Option<&'static str> {
        if !self.versorgung_zeile_gilt(partner) {
            // Kein Ring-Wert fuer die Zeilen: drei Gruende. Die Person erreicht die Altersgrenze erst nach dem Veranlagungsjahr
            // (der Bezug ist Arbeitslohn und frei, sofern der Beginn passt), sie hat sie im oder vor dem Jahr erreicht (eigener
            // Text), oder der Bezug liegt unter 1 Euro bzw. fehlt etwas (alter Text).
            match self.altersgrenze(partner) {
                Some(ALTERSGRENZE_LOHN) => {}
                Some(ALTERSGRENZE_DARUEBER) => return Some(VERSORGUNG_SPERRE_ALTERSGRENZE),
                _ => return Some(VERSORGUNG_SPERRE),
            }
        }
        let beginn = if partner {
            VERSORGUNG_BEGINN_PARTNER
        } else {
            VERSORGUNG_BEGINN
        };
        let jahr = self
            .snapshot
            .get(beginn)
            .filter(|s| s.zustand == Zustand::Bestaetigt)
            .and_then(|s| py::int(&s.wert).ok());
        match jahr {
            Some(jahr) if jahr < self.vz => None,
            _ => Some(VERSORGUNG_SPERRE_BEGINN),
        }
    }

    /// `versorgung_jahresrente` und `versorgung_jahresrente_partner` OHNE Kz (Abweichung Nr. 42, Nr. 50, Nr. 64). Steht der
    /// Ring-Wert ([`Bau::versorgung_zeile_gilt`]) auf `true`, steht der Betrag in Zeile 11 der Anlage N
    /// ([`VERSORGUNG_BETRAG_KZ`], auf volle Euro abgerundet wie jede Einnahme), sonst mit Grund in `nicht_deklariert` (Bezug vor
    /// dem Alters-Gate: er steckt als Arbeitslohn in Zeile 5; Bezug unter 1 Euro). Ein bestaetigter Betrag ueber 0 sperrt die
    /// Abgabe nur, wo die Erklaerung ihn nicht voll traegt ([`Bau::versorgung_sperre`], 409 `deklaration_unvollstaendig`); die
    /// Sperre haelt auch `erzeuge_xml` zurueck. Der Betrag des Ehegatten zaehlt nur bei bestaetigter Zusammenveranlagung: bei
    /// Einzelveranlagung zaehlt er im Bescheid nicht und sperrt nicht. Ein vorlaeufiger Betrag erreicht diesen Zweig nicht; er
    /// behaelt den Grund "Pflicht-Bestaetigung fehlt" (`feld`). Ein Wert, aus dem sich keine ganze Zahl lesen laesst (`None`),
    /// zaehlt wie im Bescheid als 0 und wirft keinen Fehler (wie [`Bau::abzug_gewaehlt`]).
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
            if let Some(grund) = self.versorgung_sperre(partner) {
                self.offen(feld_id, grund);
            }
            self.versorgung_rueckfragen(partner);
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

    /// Wahr, wenn der Bescheid den Abzug rechnet und die Erklaerung ihn als zweite Zeile "Sonstiges" der Anlage N traegt: der
    /// Ring-Wert [`DBA_ABZUG_ZEILE`] steht bestaetigt und ueber 0 im Snapshot (`bescheid::deklaration::mit_ring_werten`, die enge
    /// Bedingung des Bescheids: Wahl, Steuer und Auslandseinkuenfte ueber 0, keine Freistellung, Arbeitslohn, einzeln, keine
    /// fiktive Steuer). `deklariere` kann diese Bedingung nicht selbst auswerten; ohne Ring-Wert gilt der Abzug als nicht getragen.
    fn abzug_zeile_getragen(&self) -> bool {
        self.snapshot.get(DBA_ABZUG_ZEILE).is_some_and(|s| {
            s.zustand == Zustand::Bestaetigt && py::int(&s.wert).is_ok_and(|cent| cent > 0)
        })
    }

    /// `dba_abzug_statt_anrechnung` OHNE Kz (Abweichung Nr. 29): der Marker steht mit Grund in `nicht_deklariert`. Ist die
    /// Wahl `true` und die gezahlte Steuer (bestaetigt) ueber 0, steht die Steuer unter `E0600920` ([`Bau::abzug_gewaehlt`]).
    /// Die Abgabe sperrt ([`DBA_ABZUG_SPERRE`], 409 `deklaration_unvollstaendig`) nur, wenn der Bescheid den Abzug nicht rechnet
    /// ([`Bau::abzug_zeile_getragen`] falsch): dann meldete die Erklaerung `E0600920` ohne die Zeile der Anlage N und ohne
    /// gekuerzte Einkuenfte. Im getragenen Fall fiel die Sperre am 2026-10-10 (`checkESt` hat Zeile und Anlage AUS angenommen).
    /// Die Sperre braucht die Auslandseinkuenfte NICHT: die Rechnung kuerzt nur bei Einkuenften ueber 0, die Erklaerung meldet
    /// aber auch bei 0 den Abzug, den der Nutzer gewaehlt hat.
    fn dba_abzug(&mut self, feld_id: &str, b: &Bindung) {
        let grund = b
            .elster_kz_grund
            .clone()
            .unwrap_or_else(|| "kein elster_kz".to_owned());
        self.nicht(feld_id, grund);
        if self.abzug_gewaehlt() && !self.abzug_zeile_getragen() {
            self.offen(feld_id, DBA_ABZUG_SPERRE);
        }
    }

    /// Anlage AUS Zeile 7 NACH dem Abzug (Anleitung zur Anlage AUS, Zeile 10: "bei der Ermittlung der Einkuenfte abziehen und die
    /// abgezogene Steuer in Zeile 10 eintragen"; Schema: `E0600920` ist "in Zeile `E0601401.Vordruckzeile` abgezogene
    /// auslaendische Steuern"). Ist die Zeile "Sonstiges" getragen ([`Bau::abzug_zeile_getragen`]), steht `E0601401` um die
    /// aufgerundete Steuer unter `E0600920` gekuerzt; das Ergebnis darf negativ sein (die Anleitung nennt das, das Schema
    /// erlaubt es). Die Anrechnung (Wahl `false`) laesst Zeile 7 unberuehrt. NACH der Feldschleife, denn beide Kz stehen erst dann.
    fn dba_einkuenfte_nach_abzug(&mut self) {
        if !self.abzug_gewaehlt() || !self.abzug_zeile_getragen() {
            return;
        }
        let Some(kz) = self.bindung.get(DBA_EINKUENFTE).and_then(|b| kz_von(b)) else {
            return;
        };
        let (Some(einkuenfte), Some(abzug)) = (
            self.deklaration.get(kz).and_then(Value::as_i64),
            self.deklaration.get(DBA_ABZUG_KZ).and_then(Value::as_i64),
        ) else {
            return;
        };
        self.deklaration
            .insert(kz.to_owned(), Value::from(einkuenfte.saturating_sub(abzug)));
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
            PyWert::Text(t) => suche(DBA_STAAT_LISTENTEXT, t)
                .map(str::to_owned)
                .or_else(|| self.sonstiger_staat_text(t)),
            _ => None,
        });
        if let Some(text) = text {
            self.deklaration
                .insert(STAAT_KZ.to_owned(), Value::String(text));
        } else if self.staat_ist_sonstiger() {
            // Eine vorlaeufige Folgefrage hat ihren Eintrag schon aus `Bau::feld`; sonst fehlt sie oder steht nicht in der Liste.
            if snapshot.get(DBA_STAAT_SONSTIGER).is_none_or(|f| f.zustand == Zustand::Bestaetigt) {
                self.nicht(DBA_STAAT, "Staat ohne Listentext: sonstiger Staat ohne gueltigen Namen aus der Folgefrage");
                self.offen(DBA_STAAT_SONSTIGER, STAAT_SONSTIGER_SPERRE);
            }
        } else {
            if staat.is_some() {
                self.nicht(DBA_STAAT, "Staat ohne Listentext: unbekannte Angabe");
            }
            self.offen(DBA_STAAT, STAAT_SPERRE);
        }
    }

    /// Steht `dba_staat` bestaetigt auf `sonstiger_staat`? Nur dann zaehlt die Folgefrage [`DBA_STAAT_SONSTIGER`].
    fn staat_ist_sonstiger(&self) -> bool {
        self.snapshot.get(DBA_STAAT).is_some_and(|s| {
            s.zustand == Zustand::Bestaetigt && matches!(&s.wert, PyWert::Text(t) if t == SONSTIGER_STAAT)
        })
    }

    /// Der Listentext aus der Folgefrage, wenn `staatwert` `sonstiger_staat` ist: bestaetigt, ein Text und einer der `enum_werte`
    /// der Bindung. Jeder andere Wert (fehlt, vorlaeufig, kein Text, nicht in der Liste) liefert `None` und sperrt.
    fn sonstiger_staat_text(&self, staatwert: &str) -> Option<String> {
        if staatwert != SONSTIGER_STAAT {
            return None;
        }
        let folge = self.snapshot.get(DBA_STAAT_SONSTIGER)?;
        let PyWert::Text(t) = &folge.wert else {
            return None;
        };
        let erlaubt = self.bindung.get(DBA_STAAT_SONSTIGER)?.enum_werte.as_ref()?;
        (folge.zustand == Zustand::Bestaetigt && erlaubt.contains(t)).then(|| t.clone())
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
        } else if self.p23_feld(idx, basis, feld_id, wert, b)? {
            // Abweichung Nr. 54: Feld eines Privatverkaufs, `p23_feld` hat es geschrieben oder bewusst uebergangen.
        } else if self.rav_bav_feld(idx, basis, feld_id, wert, b)? {
            // Abweichung Nr. 55: Feld einer weiteren Mitteilung, `rav_bav_feld` hat es geschrieben oder bewusst uebergangen.
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
        } else if feld_id == DBA_STAAT || feld_id == DBA_STAAT_SONSTIGER {
            // Abweichung Nr. 51 und Nr. 58: der Staat steht nach der Feldschleife (`Bau::dba_staat`), nicht hier.
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
        } else if ist_versorgung_ring_wert(feld_id) && kz_von(b).is_none() {
            // Ring-Werte (Abweichung Nr. 50 und Nr. 65): kein Feld der Erklaerung. `versorgung`, `versorgung_angabe` und
            // `versorgung_sperre` lesen sie.
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
        } else if self.vorsorge_partner_feld(feld_id) {
            // Abweichung Nr. 61: Vorsorge-Feld des Ehegatten, `vorsorge_partner_feld` hat es uebergangen, die Summe steht nach der Schleife.
        } else if self.rav_bav_feld(1, feld_id, feld_id, wert, b)? {
            // Abweichung Nr. 55: Feld der ersten Mitteilung, `rav_bav_feld` hat es geschrieben oder bewusst uebergangen.
        } else if let Some(kz) = kz_von(b) {
            schreibe_kz(&mut self.deklaration, kz, wert, Some(b.typ), self.null_kz)
                .map_err(&fehler)?;
        } else if self.p23_feld(1, feld_id, feld_id, wert, b)? {
            // Abweichung Nr. 54: Feld des ersten Privatverkaufs, `p23_feld` hat es geschrieben oder bewusst uebergangen.
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
        vz,
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
    bau.dba_einkuenfte_nach_abzug();
    bau.dba_staat();
    bau.vorsorge_partner()?;
    bau.lohn_rueckfragen();
    bau.gewinn_sperren();
    bau.antrag_person_b();
    let dokumentiert = bau.dokumentiert();
    bau.p23_gewinn()?;
    let anlage_instanzen = bau.instanzen_ausgabe();
    p35a_summe_aus_posten(&mut bau.deklaration, &anlage_instanzen);
    weitere_wk_summe(&mut bau.deklaration, &anlage_instanzen);
    berufsverband_zeile(&mut bau.deklaration);
    berufsverband_zeile(&mut bau.person_b);
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

/// Zeile 53 der Anlage N einer Person (Abweichung Nr. 93): ueber 0 stehen Bezeichnung [`BERUFSVERBAND_TEXT_KZ`] und Summe
/// [`BERUFSVERBAND_SUMME_KZ`] (= der eine Betrag, es gibt nur eine Zeile je Person) neben dem Betrag [`BERUFSVERBAND_BETRAG_KZ`].
/// Bei 0 oder einem negativen Wert faellt der Betrag weg: eine Zeile ohne Beitrag meldet nichts, und das Schema verlangt in `Einz`
/// Bezeichnung und Betrag zusammen. Wie jede weggelassene 0 bekommt sie keinen Eintrag in `nicht_deklariert`
/// (Vault: `decisions/elster-null-in-kz-ohne-null-weglassen`, Punkt 3).
fn berufsverband_zeile(zeile: &mut BTreeMap<String, Value>) {
    match zeile.get(BERUFSVERBAND_BETRAG_KZ).and_then(Value::as_i64) {
        Some(euro) if euro > 0 => {
            zeile.insert(
                BERUFSVERBAND_TEXT_KZ.to_owned(),
                Value::String(BERUFSVERBAND_TEXT.to_owned()),
            );
            zeile.insert(BERUFSVERBAND_SUMME_KZ.to_owned(), Value::from(euro));
        }
        _ => {
            zeile.remove(BERUFSVERBAND_BETRAG_KZ);
        }
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

    /// Abweichung Nr. 54: ein Feld der Privatverkaufs-Instanz `idx` (Betrag, Bezeichnung, Datum oder Rechenfeld). `false`: `basis` gehoert
    /// nicht dazu. Betraege gehen in die `rohdaten` (der Gewinn entsteht in [`Bau::p23_gewinn`]), Bezeichnung und Daten an das Kz der Art,
    /// die Rechenfelder (Eigennutzung, taeglicher Gebrauch, Einkunftsquelle) schreiben hier nichts: `bescheid::privatverkauf` liest sie, und
    /// die Eigennutzung samt Wohnflaeche und Zeitraum setzt erst [`Bau::p23_gewinn`] in die Kz der Zeile 36.
    fn p23_feld(
        &mut self,
        idx: u64,
        basis: &str,
        feld_id: &str,
        wert: &PyWert,
        b: &Bindung,
    ) -> Ergebnis<bool> {
        if let Some(p23) = P23_BETRAGSFELDER.iter().find(|f| **f == basis) {
            let n = py::int(wert).map_err(wert_fehler(feld_id))?;
            self.instanz("p23_veraeusserung", idx).rohdaten.insert(p23, n);
        } else if let Some((_, tabelle)) = P23_ARTFELDER.iter().find(|(f, _)| *f == basis) {
            self.p23_artfeld(idx, feld_id, wert, b, tabelle)?;
        } else {
            return Ok(P23_RECHENFELDER.contains(&basis));
        }
        Ok(true)
    }

    /// Abweichung Nr. 55: ein Feld der Rente mit Leistungsmitteilung, Instanz `idx`. `false`: `basis` gehoert nicht dazu. Ein Betrag
    /// unter einem vollen Euro entfaellt, ebenso ein Beginn ohne Betrag derselben Instanz (ohne `nicht_deklariert`: nichts geht verloren,
    /// der Bescheid rechnet beides mit 0). Die Rechenfelder schreiben nichts: `bescheid::rav_bav` liest sie. Instanz 1 steht in
    /// `deklaration`, die uebrigen in den Instanzen, wie bei jedem Kz-Feld.
    fn rav_bav_feld(
        &mut self,
        idx: u64,
        basis: &str,
        feld_id: &str,
        wert: &PyWert,
        b: &Bindung,
    ) -> Ergebnis<bool> {
        if RAV_BAV_RECHENFELDER.contains(&basis) {
            return Ok(true);
        }
        let betrag_feld = if RAV_BAV_BETRAGSFELDER.contains(&basis) {
            basis
        } else if let Some((_, betrag)) = RAV_BAV_BEGINNFELDER.iter().find(|(f, _)| *f == basis) {
            *betrag
        } else {
            return Ok(false);
        };
        let Some(kz) = kz_von(b) else {
            return Ok(false);
        };
        let fehler = wert_fehler(feld_id);
        // Der Betrag der eigenen Instanz: bei einem Betragsfeld der eigene Wert, bei einem Beginn das bestaetigte Betragsfeld daneben.
        let betrag = if betrag_feld == basis {
            Some(py::int(wert).map_err(&fehler)?)
        } else {
            let betrag_id = if idx == 1 {
                betrag_feld.to_owned()
            } else {
                format!("{betrag_feld}__{idx}")
            };
            self.snapshot
                .get(&betrag_id)
                .filter(|a| a.zustand == Zustand::Bestaetigt)
                .map(|a| py::int(&a.wert).map_err(&fehler))
                .transpose()?
        };
        if betrag.is_some_and(|c| c >= 100) {
            let null_kz = self.null_kz;
            let ziel = if idx == 1 {
                &mut self.deklaration
            } else {
                &mut self.instanz(RAV_BAV_GRUPPE, idx).felder
            };
            schreibe_kz(ziel, kz, wert, Some(b.typ), null_kz).map_err(&fehler)?;
        }
        Ok(true)
    }

    /// Abweichung Nr. 54: ein Textfeld oder Datum des Privatverkaufs an das Kz seiner Art (Grundstueck, anderes Wirtschaftsgut).
    /// Ohne bestaetigte Art bleibt der Kz-Zweig offen; Kryptowert und Beteiligung haben keinen Zweig (die Berechnung sperrt vorher).
    fn p23_artfeld(
        &mut self,
        idx: u64,
        feld_id: &str,
        wert: &PyWert,
        b: &Bindung,
        tabelle: &[(&str, &str)],
    ) -> Ergebnis<()> {
        let fehler = wert_fehler(feld_id);
        let art_feld = if idx == 1 {
            P23_ART_FELD.to_owned()
        } else {
            format!("{P23_ART_FELD}__{idx}")
        };
        let Some(art) = self
            .snapshot
            .get(&art_feld)
            .filter(|a| a.zustand == Zustand::Bestaetigt)
        else {
            self.offen(feld_id, format!("{art_feld} unbestätigt — Kz-Zweig offen"));
            return Ok(());
        };
        if let Some(kz) = nachschlagen(tabelle, &art.wert).map_err(&fehler)? {
            let null_kz = self.null_kz;
            let felder = &mut self.instanz("p23_veraeusserung", idx).felder;
            schreibe_kz(felder, kz, wert, Some(b.typ), null_kz).map_err(&fehler)?;
        } else {
            let grund = format!("p23-Typ '{}' ohne Kz-Zweig", art.wert.py_str());
            self.nicht(feld_id, grund);
        }
        Ok(())
    }

    /// Abweichung Nr. 54 (Anlage SO, Zeile 36): traegt das Grundstueck der Instanz `idx` Haken, Zeitraum und Flaeche? Das gilt, wenn die
    /// Eigennutzung bestaetigt `ausschliesslich` oder `letzte_drei_jahre` ist. Teilnutzung (`teilweise`) und `nein` tragen nichts: die
    /// Teilnutzung sperrt vorher (`bescheid::privatverkauf`). Das Feld, das fehlt, nennt [`Nutzung::Fehlt`]; die Reihenfolge ist Flaeche,
    /// Beginn, Ende. `feld_id` ist die Instanz fuer eine Fehlermeldung.
    fn p23_nutzung(&self, idx: u64, grundstueck: bool, feld_id: &str) -> Ergebnis<Nutzung> {
        let bestaetigt = |basis: &str| {
            let name = if idx == 1 { basis.to_owned() } else { format!("{basis}__{idx}") };
            self.snapshot.get(&name).filter(|f| f.zustand == Zustand::Bestaetigt)
        };
        let eigen = bestaetigt("p23_eigennutzung")
            .is_some_and(|f| matches!(&f.wert, PyWert::Text(t) if t == "ausschliesslich" || t == "letzte_drei_jahre"));
        if !grundstueck || !eigen {
            return Ok(Nutzung::Keine);
        }
        let Some(flaeche) = bestaetigt("p23_wohnflaeche") else {
            return Ok(Nutzung::Fehlt("p23_wohnflaeche"));
        };
        let (Some(PyWert::Text(von)), Some(PyWert::Text(bis))) = (
            bestaetigt("p23_eigennutzung_von").map(|f| &f.wert),
            bestaetigt("p23_eigennutzung_bis").map(|f| &f.wert),
        ) else {
            return Ok(Nutzung::Fehlt("p23_eigennutzung_von und p23_eigennutzung_bis"));
        };
        let [haken, zeitraum, qm] = P23_EIGENNUTZUNG_KZ;
        let flaeche = py::int(&flaeche.wert).map_err(wert_fehler(feld_id))?;
        Ok(Nutzung::Kz([
            (haken, Value::Bool(true)),
            (zeitraum, Value::String(format!("{von}-{bis}"))),
            (qm, Value::from(flaeche)),
        ]))
    }

    /// § 23 Gewinn je Instanz = Preis − (AK/HK − `AfA`) − WK (Abs. 3 S. 1 und 4), dann an die Kz des Veraeusserungstyps. Ein selbst
    /// bewohntes Grundstueck traegt stattdessen den Gewinn 0 und die Kz der Zeile 36 ([`Bau::p23_nutzung`]).
    fn p23_gewinn(&mut self) -> Ergebnis<()> {
        const GRUPPE: &str = "p23_veraeusserung";
        let Some(gruppe) = self.instanzen.get(&GRUPPE.to_owned()) else {
            return Ok(());
        };
        // GEWOLLTE ABWEICHUNG von Python (Korrektheit vor Paritaet): Python rechnet die Differenz
        // exakt (Preis 0, AK = WK = 9e18 ct ergibt -1,8e19 ct). Ausserhalb von `i64` meldet Rust
        // `Ueberlauf` (HTTP 422) statt zu rechnen; `wrapping_sub` kippte dort das Vorzeichen
        // (+446744073709551616 ct statt eines Verlusts). Ueber HTTP seit Abweichung Nr. 54 hoechstens
        // nach dem Guard erreichbar: `bescheid::privatverkauf` rechnet denselben Gewinn vorher in Cent
        // mit `i128` und meldet einen Ueberlauf als Fehler. Darum kein Parity-Eintrag.
        let gewinne = gruppe
            .schluessel()
            .iter()
            .filter_map(|idx| gruppe.get(idx).map(|inst| (*idx, inst)))
            .map(|(idx, inst)| {
                let roh = |k: &str| inst.rohdaten.get(k).copied().unwrap_or(0);
                let gewinn = roh("p23_anschaffung_herstellungskosten")
                    .checked_sub(roh("p23_afa"))
                    .and_then(|kosten| roh("p23_veraeusserungspreis").checked_sub(kosten))
                    .and_then(|d| d.checked_sub(roh("p23_werbungskosten")))
                    .ok_or_else(|| DeklarationsFehler::Ueberlauf {
                        feld_id: format!("p23_veraeusserung__{idx}"),
                        was: "Differenz jenseits i64",
                    })?;
                Ok((idx, gewinn))
            })
            .collect::<Ergebnis<Vec<(u64, i64)>>>()?;
        for (idx, gewinn) in gewinne {
            // Abweichung Nr. 54: auch ein Gewinn von 0 steht in der Erklaerung. Bezeichnung und beide Daten sind jetzt Teil der Zeile,
            // und `checkESt` verlangt beim Grundstueck dann den Gewinn (Spike AK0, Pflicht-Kz E0306801); ohne die Zeile fehlte er.
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
            let grundstueck = matches!(&art.wert, PyWert::Text(t) if t == "grundstueck");
            if let Some(kz) =
                nachschlagen(P23_GEWINN_KZ, &art.wert).map_err(wert_fehler(&feld_id))?
            {
                // Ein selbst bewohntes Grundstueck traegt Haken, Zeitraum und Flaeche; sein Gewinn ist steuerfrei und steht als 0.
                let (gewinn, nutzung) = match self.p23_nutzung(idx, grundstueck, &feld_id)? {
                    Nutzung::Keine => (gewinn, Vec::new()),
                    Nutzung::Fehlt(feld) => {
                        self.offen(&feld_id, format!("{feld} fehlt oder ist unbestätigt — Zeile 36 der Anlage SO offen"));
                        continue;
                    }
                    Nutzung::Kz(zeile_36) => (0, Vec::from(zeile_36)),
                };
                let v = cent_nach_kz(
                    Cent::new(gewinn),
                    &kz_pruefen(kz).map_err(wert_fehler(&feld_id))?,
                )
                .als_json();
                let felder = &mut self.instanz(GRUPPE, idx).felder;
                felder.insert(kz.to_owned(), v);
                for (nutzung_kz, wert) in nutzung {
                    felder.insert(nutzung_kz.to_owned(), wert);
                }
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
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::path::Path;

    use domain::{Achsenwert, Herkunft, Konfession, PruefTiefe, Rentenart};
    use serde_json::json;

    use super::{
        deklariere, gruppe_von, iban_muster, iban_pruefziffer_gueltig, Deklaration, Felder, Feldtyp,
        SnapshotFeld, Value, Zustand, P23_EIGENNUTZUNG_KZ, PARTNER_VERZWEIGUNG, VERZWEIGUNG,
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

    /// Der Abzug nach § 34c Abs. 2 steht in der Anlage AUS so, wie die Anleitung (Zeile 10) und das Schema es verlangen:
    /// `E0600920` heisst "In Zeile 7 (`E0601401`) abgezogene auslaendische Steuern", Zeile 7 steht also NACH dem Abzug. Ist die
    /// Zeile "Sonstiges" getragen (Ring-Wert `dba_abzug_zeile_cent` bestaetigt und ueber 0: der Bescheid rechnet den Abzug), steht
    /// `E0601401` um die aufgerundete Steuer gekuerzt, auch negativ; die Abgabe sperrt nicht. Rechnet der Bescheid keinen Abzug
    /// (Ring-Wert fehlt oder nur vorlaeufig, z. B. bei Freistellung), bleibt die Sperre am Marker und Zeile 7 unveraendert; die
    /// Anrechnung (Wahl `false`) laesst Zeile 7 unberuehrt. Das Regal (`regal.rs`) fuehrt `E0600920` mit dem Test der Abweichung Nr. 41.
    #[test]
    fn abzug_getragen_kuerzt_zeile_7_und_sperrt_nicht_sonst_bleibt_die_sperre() {
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
        let b = Zustand::Bestaetigt;
        // (Name, Wahl, Ring-Wert in Cent mit Zustand, Einkuenfte in Cent) -> (E0601401, E0600920, E0601901, Sperre am Marker)
        let faelle = [
            ("getragen", true, Some((70_001, b)), 500_000, Some(4299), Some(701), None, false),
            ("getragen, Einkuenfte unter der Steuer", true, Some((70_001, b)), 70_000, Some(-1), Some(701), None, false),
            ("Bescheid rechnet keinen Abzug", true, None, 500_000, Some(5000), Some(701), None, true),
            ("Ring-Wert nur vorlaeufig", true, Some((70_001, Zustand::Vorlaeufig)), 500_000, Some(5000), Some(701), None, true),
            ("Ring-Wert 0", true, Some((0, b)), 500_000, Some(5000), Some(701), None, true),
            ("Anrechnung", false, None, 500_000, Some(5000), None, Some(700), false),
        ];
        for (name, wahl, ring, einkuenfte, z7, z10, anrechnung, sperre) in faelle {
            let mut felder = Felder::from([
                ("dba_abzug_statt_anrechnung".to_owned(), feld(json!(wahl), b)),
                ("dba_gezahlte_auslaendische_steuer".to_owned(), feld(json!(70_001), b)),
                ("dba_auslaendische_einkuenfte".to_owned(), feld(json!(einkuenfte), b)),
                ("dba_staat".to_owned(), feld(json!("Frankreich"), b)),
            ]);
            if let Some((cent, zustand)) = ring {
                felder.insert("dba_abzug_zeile_cent".to_owned(), feld(json!(cent), zustand));
            }
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            let kz = |k: &str| d.deklaration.get(k).cloned();
            assert_eq!(kz("E0601401"), z7.map(|e| json!(e)), "{name}: Zeile 7");
            assert_eq!(kz("E0600920"), z10.map(|e| json!(e)), "{name}: Zeile 10");
            assert_eq!(kz("E0601901"), anrechnung.map(|e| json!(e)), "{name}: Anrechnung");
            let gesperrt: Vec<_> = d.unvollstaendig().iter().map(|e| e.feld_id.clone()).collect();
            // Ein nur vorlaeufiger Ring-Wert steht zusaetzlich mit seinem eigenen Grund ("Pflicht-Bestaetigung fehlt") in der Liste.
            let soll: Vec<&str> = match (sperre, ring) {
                (false, _) => vec![],
                (true, Some((_, Zustand::Vorlaeufig))) => vec!["dba_abzug_statt_anrechnung", "dba_abzug_zeile_cent"],
                (true, _) => vec!["dba_abzug_statt_anrechnung"],
            };
            assert_eq!(gesperrt, soll, "{name}: Sperre");
        }
    }

    /// Abweichung Nr. 48: ein bestaetigter Betrag ueber 0 in `ep_unfallkosten` steht aufgerundet unter `E0205406`, mit der
    /// Bezeichnung `E0205405` und der Summe `E0204803` und sperrt die Abgabe NICHT (`checkESt` hat die Zeile angenommen, die
    /// Sperre fiel am 2026-10-10); bei 0 und bei einem negativen Wert steht nichts in der Deklaration, der Betrag bleibt in
    /// `nicht_deklariert` und nichts sperrt. Das Regal (`regal.rs`) fuehrt die drei Literale als `Verhalten` mit diesem Test.
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
                    d.unvollstaendig().is_empty(),
                    "{cent} Cent: die Sperre steht noch: {:?}",
                    d.unvollstaendig()
                );
                assert!(d.nicht_deklariert.iter().all(|e| e.feld_id != "ep_unfallkosten"));
            } else {
                assert!(zeile.iter().all(Option::is_none), "{cent} Cent: {zeile:?}");
                assert!(d.unvollstaendig().is_empty(), "{cent} Cent: {:?}", d.unvollstaendig());
                assert!(d.nicht_deklariert.iter().any(|e| e.feld_id == "ep_unfallkosten"));
            }
        }
    }

    /// Abweichung Nr. 93: ein bestaetigter Beitrag ueber 0 in `berufsverband_beitraege` steht aufgerundet unter `E0204003` in der
    /// Deklaration, der des Ehegatten unter demselben Kz in `person_b`; dazu kommen Bezeichnung `E0204001` und Summe `E0204002`.
    /// Bei 0 und bei einem negativen Wert steht keines der drei Kz da. Das Regal (`regal.rs`) fuehrt die Literale als `Verhalten`
    /// mit diesem Test.
    #[test]
    fn berufsverband_steht_ueber_null_in_zeile_53() {
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
        // Cent -> Betrag in Euro, wenn die Zeile steht. -500: ein negativer Wert zaehlt im Bescheid als 0, also keine Zeile.
        for (cent, euro) in [
            (150_000, Some(1500)),
            (150_001, Some(1501)),
            (1, Some(1)),
            (0, None),
            (-500, None),
        ] {
            let felder = Felder::from([
                ("berufsverband_beitraege".to_owned(), feld(json!(cent))),
                ("berufsverband_beitraege_partner".to_owned(), feld(json!(cent))),
            ]);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            for (name, zeile) in [("Person A", &d.deklaration), ("Person B", &d.person_b)] {
                let kz = ["E0204001", "E0204002", "E0204003"].map(|k| zeile.get(k));
                if let Some(e) = euro {
                    assert_eq!(kz[2], Some(&json!(e)), "{name}, {cent} Cent: Betrag");
                    assert_eq!(kz[1], Some(&json!(e)), "{name}, {cent} Cent: Summe");
                    assert!(
                        kz[0].and_then(Value::as_str).is_some_and(|t| t.contains("Berufsverb")),
                        "{name}, {cent} Cent: Bezeichnung {:?}",
                        kz[0]
                    );
                } else {
                    assert!(kz.iter().all(Option::is_none), "{name}, {cent} Cent: {kz:?}");
                }
            }
            assert!(d.unvollstaendig().is_empty(), "{cent} Cent: {:?}", d.unvollstaendig());
        }
    }

    /// Abweichung Nr. 61: die fuenf Summen der Zeilen 44 bis 48 (Anlage Vorsorgeaufwand) tragen die Beitraege BEIDER Ehegatten, jeder
    /// Betrag einzeln aufgerundet, dann addiert (`aufwand-einzelposten-aufrunden-summe-aus-posten`: 500,01 + 480,01 steht als
    /// 501 + 481 = 982, nicht als 981). Der Ehegatte zaehlt nur bei bestaetigter Zusammenveranlagung; ein vorlaeufiger Betrag steht
    /// in `unvollstaendig` und aendert die Summe nicht. Das Regal (`regal.rs`) fuehrt die fuenf Literale als `Verhalten` mit diesem Test.
    #[test]
    fn die_vorsorge_summe_traegt_beide_ehegatten_aus_gerundeten_posten() {
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
        let b = Zustand::Bestaetigt;
        // Je Kz Person A und Ehegatte, mit anderen Cent-Werten je Kz: ein vertauschtes Kz faellt auf.
        let paare = [
            ("E2001403", "vorsorge_arbeitslosenversicherung", 50_001, 48_001, 982),
            ("E2001503", "vorsorge_erwerbsunfaehigkeit", 30_001, 32_001, 622),
            ("E2001803", "vorsorge_unfall_haftpflicht", 20_001, 21_001, 412),
            ("E2001903", "vorsorge_rv_alt_mit_ueberschuss", 40_001, 41_001, 812),
            ("E2002003", "vorsorge_rv_alt_ohne_ueberschuss", 30_001, 29_001, 592),
        ];
        let mut zusammen = Felder::from([("veranlagung".to_owned(), feld(json!("zusammen"), b))]);
        let mut einzel = Felder::from([("veranlagung".to_owned(), feld(json!("einzel"), b))]);
        for (_, eigen, cent_a, cent_b, _) in paare {
            for felder in [&mut zusammen, &mut einzel] {
                felder.insert(eigen.to_owned(), feld(json!(cent_a), b));
                felder.insert(format!("{eigen}_partner"), feld(json!(cent_b), b));
            }
        }
        let d = deklariere(&zusammen, &index, 2025, None).unwrap();
        for (kz, _, _, _, summe) in paare {
            assert_eq!(d.deklaration.get(kz), Some(&json!(summe)), "zusammen, {kz}: Summe beider Ehegatten");
        }
        assert!(d.unvollstaendig().is_empty(), "{:?}", d.unvollstaendig());
        // Einzelveranlagung: nur Person A (aufgerundet); der Betrag des Ehegatten steht mit Grund in `nicht_deklariert`.
        let d = deklariere(&einzel, &index, 2025, None).unwrap();
        for (kz, eigen, cent_a, _, _) in paare {
            assert_eq!(d.deklaration.get(kz), Some(&json!((cent_a + 99) / 100)), "einzel, {kz}: nur Person A");
            assert!(d.nicht_deklariert.iter().any(|e| e.feld_id == format!("{eigen}_partner")), "einzel, {eigen}_partner");
        }
        // Nur der Ehegatte traegt einen Betrag: die Summe ist sein gerundeter Betrag.
        let mut nur_partner = zusammen.clone();
        nur_partner.remove("vorsorge_arbeitslosenversicherung");
        let d = deklariere(&nur_partner, &index, 2025, None).unwrap();
        assert_eq!(d.deklaration.get("E2001403"), Some(&json!(481)), "nur der Ehegatte");
        // Ein vorlaeufiger Betrag des Ehegatten sperrt und zaehlt nicht in die Summe.
        let mut vorlaeufig = zusammen.clone();
        vorlaeufig.insert(
            "vorsorge_arbeitslosenversicherung_partner".to_owned(),
            feld(json!(48_001), Zustand::Vorlaeufig),
        );
        let d = deklariere(&vorlaeufig, &index, 2025, None).unwrap();
        assert_eq!(d.deklaration.get("E2001403"), Some(&json!(501)), "vorlaeufiger Betrag zaehlt nicht");
        let gesperrt: Vec<_> = d.unvollstaendig().iter().map(|e| e.feld_id.as_str()).collect();
        assert_eq!(gesperrt, ["vorsorge_arbeitslosenversicherung_partner"]);
    }

    /// Abweichung Nr. 61: die Abgabe sperrt, wo der Bescheid Gewinn oder Kapitaleinkuenfte aus Feldern ohne Kz rechnet, mit dem
    /// Betrag im Klartext. (1) Betriebseinnahmen-Seite ohne Gewinn als fertigen Betrag (ungleich 0) sperrt, auch in einer
    /// Instanz; steht der Gewinn, sperrt sie nicht. (2) Gewinnanteil und Verguetungen aus einer Beteiligung sperren immer, die
    /// des Ehegatten nur bei Zusammenveranlagung. (3) Sonstige Kapitalgewinne sperren, die des Ehegatten nur bei
    /// Zusammenveranlagung. Ein Betrag von 0 und ein vorlaeufiger Betrag erzeugen keinen dieser Gruende.
    #[test]
    #[allow(clippy::too_many_lines, reason = "eine Fall-Tabelle mit der Zeile je Eingabe, geteilt wuerde die Setup-Zeilen doppeln")]
    fn die_abgabe_sperrt_wo_der_bescheid_gewinn_oder_kapital_ohne_kz_rechnet() {
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
        let b = Zustand::Bestaetigt;
        // Gesperrte Felder des Falls (mit Text-Bruchstueck des Grundes), ohne die Sperren, die andere Regeln setzen.
        let sperren = |paare: &[(&str, Value, Zustand)]| -> Vec<(String, String)> {
            let felder: Felder = paare
                .iter()
                .map(|(f, w, z)| ((*f).to_owned(), feld(w.clone(), *z)))
                .collect();
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            d.unvollstaendig()
                .iter()
                .map(|e| (e.feld_id.clone(), e.grund.clone()))
                .collect()
        };
        let art = ("gewinn_betriebsart", json!("gewerbe"), b);
        let zusammen = ("veranlagung", json!("zusammen"), b);
        let einzel = ("veranlagung", json!("einzel"), b);
        // (1) Betriebseinnahmen-Seite.
        let euer = sperren(&[("betriebseinnahmen", json!(5_000_000), b)]);
        assert_eq!(euer.len(), 1, "{euer:?}");
        assert_eq!(euer[0].0, "betriebseinnahmen");
        assert!(euer[0].1.contains("Betriebseinnahmen mit 50000,00 Euro"), "{}", euer[0].1);
        assert!(euer[0].1.contains("würde 0 Euro nennen"), "{}", euer[0].1);
        for basis in ["sonstige_betriebsausgaben", "afa_jahresbetrag", "gwg_anschaffungskosten_netto"] {
            let s = sperren(&[(basis, json!(-123_456), b)]);
            assert_eq!(s.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), [basis]);
            assert!(s[0].1.contains("-1234,56 Euro"), "{basis}: {}", s[0].1);
        }
        let instanz = sperren(&[("gwg_anschaffungskosten_netto__2", json!(60_000), b)]);
        assert!(
            instanz.iter().any(|(f, g)| f == "gwg_anschaffungskosten_netto__2" && g.contains("Gewinn aus Einnahmen und Ausgaben")),
            "{instanz:?}"
        );
        // Mit einem fertigen Gewinn ungleich 0 sperrt die Betriebseinnahmen-Seite nicht; mit 0 oder ohne bestaetigte Art schon.
        let mit_gewinn = sperren(&[art.clone(), ("einkuenfte_gewinn", json!(3_500_000), b), ("betriebseinnahmen", json!(5_000_000), b)]);
        assert!(mit_gewinn.is_empty(), "{mit_gewinn:?}");
        let gewinn_null = sperren(&[art.clone(), ("einkuenfte_gewinn", json!(0), b), ("betriebseinnahmen", json!(5_000_000), b)]);
        assert_eq!(gewinn_null.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), ["betriebseinnahmen"]);
        // Ein Betrag von 0 und ein vorlaeufiger Betrag erzeugen den Gewinn-Grund nicht (der vorlaeufige hat seinen eigenen).
        assert_eq!(sperren(&[("betriebseinnahmen", json!(0), b)]).len(), 0);
        let vorlaeufig = sperren(&[("betriebseinnahmen", json!(5_000_000), Zustand::Vorlaeufig)]);
        assert!(vorlaeufig.iter().all(|(_, g)| !g.contains("Gewinn aus Einnahmen")), "{vorlaeufig:?}");
        // (2) Beteiligung: immer, auch neben einem fertigen Gewinn; der Ehegatte nur bei Zusammenveranlagung.
        for basis in ["gewinnanteil", "verguetung_taetigkeit", "verguetung_darlehen", "verguetung_ueberlassung"] {
            let s = sperren(&[art.clone(), ("einkuenfte_gewinn", json!(3_500_000), b), (basis, json!(111_100), b)]);
            assert_eq!(s.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), [basis], "{basis}");
            assert!(s[0].1.contains("1111,00 Euro"), "{basis}: {}", s[0].1);
            assert!(s[0].1.contains("Finanzamt und Steuernummer der Gesellschaft"), "{}", s[0].1);
            let partner = format!("{basis}_partner");
            let ehegatte = |veranlagung: &(&str, Value, Zustand)| {
                sperren(&[veranlagung.clone(), (partner.as_str(), json!(222_200), b)])
            };
            let mit = ehegatte(&zusammen);
            assert_eq!(mit.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), [partner.as_str()], "{partner} zusammen");
            assert!(mit[0].1.contains("2222,00 Euro"), "{}", mit[0].1);
            assert!(ehegatte(&einzel).is_empty(), "{partner} einzel");
        }
        // (3) Sonstige Kapitalgewinne.
        let kap = sperren(&[("kap_gewinn_sonstige", json!(175_000), b)]);
        assert_eq!(kap.iter().map(|(f, _)| f.as_str()).collect::<Vec<_>>(), ["kap_gewinn_sonstige"]);
        assert!(kap[0].1.contains("1750,00 Euro"), "{}", kap[0].1);
        assert_eq!(sperren(&[("kap_gewinn_sonstige", json!(0), b)]).len(), 0);
        let kap_ehegatte = |veranlagung: &(&str, Value, Zustand)| {
            sperren(&[veranlagung.clone(), ("kap_gewinn_sonstige_partner", json!(175_000), b)])
        };
        assert_eq!(kap_ehegatte(&zusammen).len(), 1);
        assert_eq!(kap_ehegatte(&einzel).len(), 0);
    }

    /// Abweichung Nr. 51: der Staat der Auslandseinkuenfte steht als Listentext unter `E0600301`, genau dann, wenn die
    /// Einkuenfte `E0601401` stehen. Drei Bindungswerte weichen vom Listentext ab. `sonstiger_staat`, ein fehlender und ein
    /// unbekannter Wert schreiben nichts und sperren (`dba_staat`); Steuer ohne Einkuenfte sperrt (`dba_gezahlte_auslaendische_steuer`);
    /// ein Staat ohne Einkuenfte und ohne Steuer steht nicht und sperrt nicht. Das Regal fuehrt das Literal mit diesem Test.
    /// Die ganze Akte und die Gegenprobe gegen die Laenderliste des Schemas stehen in `bescheid/tests/dba_staat_kz.rs`.
    /// Abweichung Nr. 58 haengt die Folgefrage `dba_staat_sonstiger` (Name zu `sonstiger_staat`) an dieselbe Tabelle.
    #[test]
    #[allow(clippy::too_many_lines, reason = "eine Fall-Tabelle mit der Zeile je Eingabe, geteilt wuerde die Setup-Zeilen doppeln")]
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
        // (Staat, Folgefrage, Einkuenfte vorhanden, Steuer vorhanden)
        //   -> (Text unter E0600301, Feld der Sperre zum Staat, Sperre zur Steuer)
        // Die Folgefrage (Abweichung Nr. 58) zaehlt nur bei bestaetigtem `sonstiger_staat`; ihr Wert muss in ihren `enum_werte` stehen.
        let faelle = [
            (Some("Frankreich"), None, true, false, Some("Frankreich"), None, false),
            (Some("Oesterreich"), None, true, false, Some("Österreich"), None, false),
            (Some("Tschechien"), None, true, true, Some("Tschechische Republik"), None, false),
            (Some("Grossbritannien"), None, true, false, Some("Vereinigtes Königreich"), None, false),
            (Some("sonstiger_staat"), None, true, false, None, Some("dba_staat_sonstiger"), false),
            (Some("sonstiger_staat"), Some("Portugal"), true, false, Some("Portugal"), None, false),
            (Some("sonstiger_staat"), Some("Korea, Republik"), true, true, Some("Korea, Republik"), None, false),
            (Some("sonstiger_staat"), Some("Cote d'Ivoire"), true, false, Some("Cote d'Ivoire"), None, false),
            (Some("sonstiger_staat"), Some("Atlantis"), true, false, None, Some("dba_staat_sonstiger"), false),
            // Ein benannter Staat steht nicht in der Folgefrage (die 15 bleiben bei `dba_staat`): der Wert wird abgelehnt.
            (Some("sonstiger_staat"), Some("Frankreich"), true, false, None, Some("dba_staat_sonstiger"), false),
            (Some("sonstiger_staat"), Some("portugal"), true, false, None, Some("dba_staat_sonstiger"), false),
            // Sonstiger Staat ohne Einkuenfte: nichts zu erklaeren, der Staat steht nicht und sperrt nicht.
            (Some("sonstiger_staat"), Some("Portugal"), false, false, None, None, false),
            (Some("sonstiger_staat"), None, false, true, None, None, true),
            // Eine Antwort, die ein spaeter gewaehlter benannter Staat ueberholt hat, aendert nichts.
            (Some("Frankreich"), Some("Portugal"), true, false, Some("Frankreich"), None, false),
            (Some("Atlantis"), Some("Portugal"), true, false, None, Some("dba_staat"), false),
            (None, Some("Portugal"), true, false, None, Some("dba_staat"), false),
            (Some("Atlantis"), None, true, false, None, Some("dba_staat"), false),
            (None, None, true, false, None, Some("dba_staat"), false),
            (Some("Frankreich"), None, false, false, None, None, false),
            (Some("Frankreich"), None, false, true, None, None, true),
            (None, None, false, true, None, None, true),
        ];
        for (staat, folge, einkuenfte, steuer, text, sperre_staat, sperre_steuer) in faelle {
            let mut felder = Felder::new();
            if let Some(s) = staat {
                felder.insert("dba_staat".to_owned(), feld(json!(s)));
            }
            if let Some(f) = folge {
                felder.insert("dba_staat_sonstiger".to_owned(), feld(json!(f)));
            }
            if einkuenfte {
                felder.insert("dba_auslaendische_einkuenfte".to_owned(), feld(json!(500_000)));
            }
            if steuer {
                felder.insert("dba_gezahlte_auslaendische_steuer".to_owned(), feld(json!(70_000)));
            }
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            let name = format!("{staat:?}, Folgefrage {folge:?}, Einkünfte {einkuenfte}, Steuer {steuer}");
            assert_eq!(
                d.deklaration.get("E0600301"),
                text.map(|t| json!(t)).as_ref(),
                "{name}: Text"
            );
            let sperre = |id: &str| d.unvollstaendig().iter().any(|e| e.feld_id == id);
            // Ein geschriebener Staat steht NICHT in `nicht_deklariert`, ein beantworteter, aber nicht geschriebener schon.
            let in_nicht = d.nicht_deklariert.iter().any(|e| e.feld_id == "dba_staat");
            assert_eq!(in_nicht, staat.is_some() && text.is_none(), "{name}: nicht_deklariert");
            // Die Folgefrage ist Quelle von `E0600301` oder ueberholt; ein Eintrag steht nie bei ihr, sondern bei `dba_staat`.
            let folge_in_nicht = d.nicht_deklariert.iter().any(|e| e.feld_id == "dba_staat_sonstiger");
            assert!(!folge_in_nicht, "{name}: die Folgefrage steht in nicht_deklariert");
            for feld_der_sperre in ["dba_staat", "dba_staat_sonstiger"] {
                assert_eq!(
                    sperre(feld_der_sperre),
                    sperre_staat == Some(feld_der_sperre),
                    "{name}: Sperre zu {feld_der_sperre}"
                );
            }
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

        // Abweichung Nr. 58: eine vorlaeufige Folgefrage steht nicht im Kz und sperrt genau einmal (der Eintrag aus `feld`, bei
        // `sonstiger_staat` kein zweiter aus `dba_staat`). Das gilt auch bei einem benannten Staat: wie jedes vorlaeufige Feld sperrt sie
        // (Eigenschaft `vorlaeufig_deklariert_nie`), obwohl die Oberflaeche sie dann nicht mehr fragt. ponytail: eine ueberholte
        // vorlaeufige Folgefrage bleibt gesperrt, bis der Nutzer erneut "Anderer Staat" waehlt oder sie bestaetigt; selten, fail-closed.
        for (staat, sperrt) in [("sonstiger_staat", true), ("Frankreich", true)] {
            let folge_vorlaeufig = Felder::from([
                ("dba_staat".to_owned(), feld(json!(staat))),
                (
                    "dba_staat_sonstiger".to_owned(),
                    SnapshotFeld { zustand: Zustand::Vorlaeufig, ..feld(json!("Portugal")) },
                ),
                ("dba_auslaendische_einkuenfte".to_owned(), feld(json!(500_000))),
            ]);
            let d = deklariere(&folge_vorlaeufig, &index, 2025, None).unwrap();
            assert_ne!(d.deklaration.get("E0600301"), Some(&json!("Portugal")), "{staat}: vorläufiger Name steht im Kz");
            let gruende: Vec<&str> = d
                .unvollstaendig()
                .iter()
                .filter(|e| e.feld_id == "dba_staat_sonstiger")
                .map(|e| e.grund.as_str())
                .collect();
            assert_eq!(gruende.len(), usize::from(sperrt), "{staat}: {gruende:?}");
            if sperrt {
                assert!(gruende[0].contains("Bestätigung"), "{gruende:?}");
            }
        }
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
            // Die Sperre der Unfallkosten ist am 2026-10-10 gefallen (`checkESt` hat die Zeile angenommen): nichts sperrt.
            assert!(d.unvollstaendig().is_empty(), "{name}: es sperrt etwas: {:?}", d.unvollstaendig());
        }
    }

    /// Abweichung Nr. 50: Betrag, Bemessungsgrundlage und Beginnjahr des Versorgungsbezugs stehen NUR bei gesetztem Ring-Wert
    /// (`versorgung_zeile`, `versorgung_zeile_partner`) in den Zeilen 11 bis 13 der Anlage N: Person A in der Deklaration, Person B
    /// in `person_b`. Der Betrag und die Bemessungsgrundlage stehen abgerundet auf volle Euro. Ohne Ring-Wert (Bezug vor dem
    /// Alters-Gate, kein Wert) steht kein Kz, und die drei Felder bleiben mit Grund in `nicht_deklariert`. Die Sperre
    /// ([`VERSORGUNG_SPERRE`]) steht genau dann, wenn keine Zeile steht (Abweichung Nr. 64); der Ring-Wert selbst steht in keiner
    /// Liste. Das Regal (`regal.rs`) fuehrt die drei Literale als `Verhalten` mit diesem Test.
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
                // Abweichung Nr. 64: die Sperre steht nur, wo keine Zeile steht (Ehegatte: Zusammenveranlagung ist bestaetigt);
                // der Bezug beginnt 2020, also vor dem Veranlagungsjahr.
                let sperren: Vec<&str> = d
                    .unvollstaendig()
                    .iter()
                    .filter(|e| e.feld_id == format!("versorgung_jahresrente{s}"))
                    .map(|e| e.grund.as_str())
                    .collect();
                let soll: &[&str] = if steht { &[] } else { &[super::VERSORGUNG_SPERRE] };
                assert_eq!(sperren, soll, "{name}: Sperre");
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

    /// Abweichung Nr. 83: der Text der Altersgrenze nennt die Frage nach dem Grad der Behinderung und sagt, dass ein Grad unter 50 die
    /// Grenze 63 gibt (die Sperre faellt, solange die Person 63 noch nicht erreicht hat). Er sagt es bedingt ("Steht die Frage ... noch
    /// offen"), denn die Frage steht nicht immer in der Liste. Die beiden anderen Texte des Bezugs nennen sie nicht: nur diese Sperre
    /// haengt am Grad.
    #[test]
    fn der_sperrtext_der_altersgrenze_nennt_die_frage_nach_dem_grad_der_behinderung() {
        let text = super::VERSORGUNG_SPERRE_ALTERSGRENZE;
        assert!(
            text.contains("Steht die Frage nach dem Grad der Behinderung noch offen")
                && text.contains("Grad unter 50")
                && text.contains("Grenze 63"),
            "{text}"
        );
        for fremd in [super::VERSORGUNG_SPERRE, super::VERSORGUNG_SPERRE_BEGINN] {
            assert!(!fremd.contains("Grad der Behinderung"), "{fremd}");
        }
    }

    /// Abweichung Nr. 53 und Nr. 64: die Gruende der Abgabe-Sperre bei Versorgungsbezuegen sagen nicht mehr, ein `checkESt`-Lauf habe
    /// die Zeilen 11 bis 13 "noch nie angenommen" (Nr. 50 hat sie offline angenommen, rc=0) oder die Abgabe sei "noch nicht
    /// freigegeben" (Julius hat die Sperre am 2026-10-12 fuer die getragenen Faelle gehoben). Beide nennen die Sperre und das
    /// amtliche Formular; der erste nimmt die Rente mit Leistungsmitteilung aus und sagt, dass die Zeilen den Bezug nicht tragen,
    /// der zweite nennt die Zeile 14 und den Beginn im Veranlagungsjahr.
    #[test]
    fn die_versorgung_sperren_nennen_den_grund_die_ausnahme_und_das_formular() {
        for text in [super::VERSORGUNG_SPERRE, super::VERSORGUNG_SPERRE_BEGINN, super::VERSORGUNG_SPERRE_ALTERSGRENZE] {
            assert!(!text.contains("noch nie angenommen") && !text.contains("noch nicht freigegeben"), "{text}");
            assert!(text.starts_with("Versorgungsbezüge über 0 Euro") && text.contains("gesperrt") && text.contains("Formular"), "{text}");
            assert!(!text.contains("nicht gesperrt"), "{text}");
        }
        let text = super::VERSORGUNG_SPERRE;
        assert!(
            text.contains("Rente aus Pensionskasse, Pensionsfonds oder Direktversicherung mit Leistungsmitteilung gehört nicht in diese Zeilen"),
            "{text}"
        );
        assert!(text.contains("nicht in die Zeilen 11 bis 13") && text.contains("Altersgrenze") && !text.contains("Zeile 14"), "{text}");
        let text = super::VERSORGUNG_SPERRE_BEGINN;
        assert!(text.contains("Zeile 14") && text.contains("im Veranlagungsjahr oder später") && !text.contains("nicht in die Zeilen 11 bis 13"), "{text}");
        // Abweichung Nr. 65: der Text der Altersgrenze sagt, dass die Person die Grenze im Veranlagungsjahr oder davor erreicht hat
        // und dass Zeile 11 sowie der Freibetrag fehlen; er verwechselt sich weder mit dem Text der Zeilen noch mit dem des Beginns.
        let text = super::VERSORGUNG_SPERRE_ALTERSGRENZE;
        assert!(
            text.contains("im Veranlagungsjahr oder davor erreicht")
                && text.contains("Zeile 11")
                && text.contains("Versorgungsfreibetrag")
                && !text.contains("Zeile 14")
                && !text.contains("nicht in die Zeilen 11 bis 13"),
            "{text}"
        );
        // Abweichung Nr. 66: jeder der vier Texte der Rueckfragen nennt die Sperre und das Formular, die Zeile des Vordrucks (oder die zweite
        // Spalte) und sein EIGENES Merkmal; keiner traegt das Merkmal eines anderen Falls (sonst verwechselte sich die Tabelle der Tests).
        let faelle = [
            (super::VERSORGUNG_SPERRE_ZWEITER_BEZUG, "zwei verschiedene Versorgungsbezüge", "zweite Spalte"),
            (super::VERSORGUNG_SPERRE_STERBEGELD, "Sterbegeld", "Zeile 15"),
            (super::VERSORGUNG_SPERRE_MEHRERE_JAHRE, "mehrere Kalenderjahre", "Zeile 16"),
            (super::VERSORGUNG_SPERRE_NICHT_GANZJAEHRIG, "nicht in allen zwölf Monaten", "Zeile 14"),
        ];
        for (text, merkmal, zeile) in faelle {
            assert!(text.starts_with("Versorgungsbezüge über 0 Euro") && text.contains("gesperrt") && text.contains("Formular"), "{text}");
            assert!(text.contains(merkmal) && text.contains(zeile), "{text}");
            for (anderer, _, _) in faelle {
                assert_eq!(anderer == text, anderer.contains(merkmal), "der Text trägt ein fremdes Merkmal {merkmal}: {anderer}");
            }
            for fremd in [
                super::VERSORGUNG_SPERRE,
                super::VERSORGUNG_SPERRE_BEGINN,
                super::VERSORGUNG_SPERRE_ALTERSGRENZE,
            ] {
                assert!(!fremd.contains(merkmal), "der Text eines anderen Grundes trägt das Merkmal {merkmal}: {fremd}");
            }
        }
    }

    /// Abweichung Nr. 64: steht der Ring-Wert auf `true` (die Zeilen 11 bis 13 stehen im XML), sperrt ein Bezug NUR, wenn er im
    /// Veranlagungsjahr oder danach beginnt ([`VERSORGUNG_SPERRE_BEGINN`]). Das Jahr ist das von `deklariere` (hier 2024 und
    /// 2025), keine Konstante. Ein Beginnjahr, das nur vorlaeufig dasteht, gilt als nicht gelesen und sperrt ebenso. Bei
    /// Einzelveranlagung zaehlt der Bezug des Ehegatten nicht und sperrt nie. Die drei Zeilen stehen in jedem Fall.
    #[test]
    fn ein_bezug_ab_dem_veranlagungsjahr_sperrt_mit_eigenem_grund_je_veranlagungsjahr() {
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
        for partner in [false, true] {
            let s = if partner { "_partner" } else { "" };
            for vz in [2024_i64, 2025] {
                // (Beginnjahr, Zustand des Beginnjahres, Veranlagung, erwarteter Grund)
                let faelle = [
                    (vz - 1, Zustand::Bestaetigt, "zusammen", None),
                    (vz - 30, Zustand::Bestaetigt, "zusammen", None),
                    (vz, Zustand::Bestaetigt, "zusammen", Some(super::VERSORGUNG_SPERRE_BEGINN)),
                    (vz + 1, Zustand::Bestaetigt, "zusammen", Some(super::VERSORGUNG_SPERRE_BEGINN)),
                    (vz - 1, Zustand::Vorlaeufig, "zusammen", Some(super::VERSORGUNG_SPERRE_BEGINN)),
                    (vz, Zustand::Bestaetigt, "einzel", if partner { None } else { Some(super::VERSORGUNG_SPERRE_BEGINN) }),
                ];
                for (beginn, zustand, veranlagung, soll) in faelle {
                    let felder = Felder::from([
                        (format!("versorgung_jahresrente{s}"), feld(json!(3_000_000), Zustand::Bestaetigt)),
                        (format!("versorgung_bemessungsgrundlage{s}"), feld(json!(2_500_000), Zustand::Bestaetigt)),
                        (format!("versorgung_beginn_jahr{s}"), feld(json!(beginn), zustand)),
                        (format!("versorgung_art{s}"), feld(json!("beamtenrechtlich"), Zustand::Bestaetigt)),
                        (format!("versorgung_zeile{s}"), feld(json!(true), Zustand::Bestaetigt)),
                        ("veranlagung".to_owned(), feld(json!(veranlagung), Zustand::Bestaetigt)),
                    ]);
                    let d = deklariere(&felder, &index, vz, None).unwrap();
                    let name = format!("Person {}, VZ {vz}, Beginn {beginn} {zustand:?}, {veranlagung}", if partner { "B" } else { "A" });
                    let sperren: Vec<&str> = d
                        .unvollstaendig()
                        .iter()
                        .filter(|e| e.feld_id == format!("versorgung_jahresrente{s}"))
                        .map(|e| e.grund.as_str())
                        .collect();
                    let soll: Vec<&str> = soll.into_iter().collect();
                    assert_eq!(sperren, soll, "{name}: Sperre");
                    let da = if partner { &d.person_b } else { &d.deklaration };
                    if veranlagung == "zusammen" || !partner {
                        assert_eq!(da.get("E0200801"), Some(&json!(30_000)), "{name}: Zeile 11 steht");
                    }
                }
            }
        }
    }

    /// Abweichung Nr. 65: ohne Ring-Wert fuer die Zeilen waehlt der Ring-Wert der Altersgrenze (`versorgung_altersgrenze`, Text) den
    /// Grund. `lohn` (die Person erreicht die Grenze erst nach dem Veranlagungsjahr) ist frei: keine Sperre, kein Kz 11 bis 13, die
    /// drei Felder mit Grund in `nicht_deklariert`; auch dann sperrt ein Beginn im Veranlagungsjahr oder danach. `darueber` gibt
    /// [`VERSORGUNG_SPERRE_ALTERSGRENZE`], auch vor dem Beginn-Grund. Fehlt der Wert, ist er vorlaeufig oder ist er kein bekannter
    /// Text, gilt [`VERSORGUNG_SPERRE`]. Bei Einzelveranlagung zaehlt der Wert des Ehegatten nicht.
    #[test]
    fn der_altersgrenzen_wert_waehlt_den_grund_der_sperre_ohne_zeilen() {
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
        let kz = ["E0200801", "E0200902", "E0201307"];
        for partner in [false, true] {
            let s = if partner { "_partner" } else { "" };
            // (Wert des Ring-Werts, Zustand, Beginnjahr, Veranlagung, erwarteter Grund)
            let b = Zustand::Bestaetigt;
            let faelle = [
                (None, b, 2020, "zusammen", Some(super::VERSORGUNG_SPERRE)),
                (Some(json!("lohn")), b, 2020, "zusammen", None),
                (Some(json!("lohn")), b, 2025, "zusammen", Some(super::VERSORGUNG_SPERRE_BEGINN)),
                (Some(json!("lohn")), b, 2026, "zusammen", Some(super::VERSORGUNG_SPERRE_BEGINN)),
                (Some(json!("lohn")), Zustand::Vorlaeufig, 2020, "zusammen", Some(super::VERSORGUNG_SPERRE)),
                (Some(json!("darueber")), b, 2020, "zusammen", Some(super::VERSORGUNG_SPERRE_ALTERSGRENZE)),
                (Some(json!("darueber")), b, 2025, "zusammen", Some(super::VERSORGUNG_SPERRE_ALTERSGRENZE)),
                (Some(json!("darueber")), Zustand::Vorlaeufig, 2020, "zusammen", Some(super::VERSORGUNG_SPERRE)),
                (Some(json!("frei")), b, 2020, "zusammen", Some(super::VERSORGUNG_SPERRE)),
                (Some(json!(true)), b, 2020, "zusammen", Some(super::VERSORGUNG_SPERRE)),
                (Some(json!("darueber")), b, 2020, "einzel", if partner { None } else { Some(super::VERSORGUNG_SPERRE_ALTERSGRENZE) }),
            ];
            for (ring, zustand, beginn, veranlagung, soll) in faelle {
                let mut felder = Felder::from([
                    (format!("versorgung_jahresrente{s}"), feld(json!(3_000_000), b)),
                    (format!("versorgung_bemessungsgrundlage{s}"), feld(json!(2_500_000), b)),
                    (format!("versorgung_beginn_jahr{s}"), feld(json!(beginn), b)),
                    (format!("versorgung_art{s}"), feld(json!("altersgrenze_sonstige"), b)),
                    ("veranlagung".to_owned(), feld(json!(veranlagung), b)),
                ]);
                if let Some(wert) = &ring {
                    felder.insert(format!("versorgung_altersgrenze{s}"), feld(wert.clone(), zustand));
                }
                let d = deklariere(&felder, &index, 2025, None).unwrap();
                let name = format!("Person {}, Altersgrenze {ring:?} {zustand:?}, Beginn {beginn}, {veranlagung}", if partner { "B" } else { "A" });
                let sperren: Vec<&str> = d
                    .unvollstaendig()
                    .iter()
                    .filter(|e| e.feld_id == format!("versorgung_jahresrente{s}"))
                    .map(|e| e.grund.as_str())
                    .collect();
                assert_eq!(sperren, soll.into_iter().collect::<Vec<_>>(), "{name}: Sperre");
                // Nie eine Zeile 11 bis 13 ohne den Ring-Wert der Zeilen; der Ring-Wert der Altersgrenze ist kein Feld der Erklaerung.
                let da = if partner { &d.person_b } else { &d.deklaration };
                assert!(kz.iter().all(|k| da.get(*k).is_none()), "{name}: kein Kz: {da:?}");
                let flag = format!("versorgung_altersgrenze{s}");
                assert!(d.nicht_deklariert.iter().all(|e| e.feld_id != flag), "{name}: nicht_deklariert");
                // Ein vorlaeufiger Wert steht wie jedes Feld ohne Bestaetigung in `unvollstaendig` (Zwei-Signal), ein bestaetigter nie.
                let offen = d.unvollstaendig().iter().any(|e| e.feld_id == flag);
                assert_eq!(offen, zustand != Zustand::Bestaetigt, "{name}: unvollstaendig");
                let nicht = |f: &str| d.nicht_deklariert.iter().filter(|e| e.feld_id == format!("{f}{s}")).count();
                assert_eq!(nicht("versorgung_jahresrente"), 1, "{name}: der Betrag steht mit Grund in nicht_deklariert");
            }
        }
    }

    /// Abweichung Nr. 66: ein bestaetigtes Ja auf eine der vier Rueckfragen zum Versorgungsbezug sperrt mit dem Text des Falls, unter dem
    /// Feld der Rueckfrage. Nur dort: bei einem bestaetigten Bezug ueber 0 der Person, die zaehlt (Ehegatte nur bei bestaetigter
    /// Zusammenveranlagung). Abweichung Nr. 79: Nein sperrt nicht; eine fehlende, eine vorlaeufige und eine nicht boolesche Antwort sperren
    /// mit eigenem Text ("nicht mit Ja oder Nein beantwortet", `eine_unbeantwortete_rueckfrage_sperrt_mit_eigenem_text`). Die Antwort
    /// der einen Person gilt nie fuer die andere.
    #[test]
    #[allow(clippy::too_many_lines, reason = "eine Fall-Tabelle mit der Zeile je Eingabe, geteilt wuerde die Setup-Zeilen doppeln")]
    fn ein_ja_auf_eine_rueckfrage_sperrt_mit_dem_text_des_falls() {
        // Abweichung Nr. 79: der Text einer unbeantworteten Rueckfrage nennt, dass keine Ja/Nein-Antwort da ist, das Merkmal des Falls
        // und die Sperre. Er ist nie der Text des Ja.
        const UNBEANTWORTET: &str = "nicht mit Ja oder Nein beantwortet";
        const SPERRE_BIS_ANTWORT: &str = "Die Abgabe ist gesperrt, bis du die Frage beantwortest";
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
        let b = Zustand::Bestaetigt;
        // Die Nummer der Lohnsteuerbescheinigung steht im Text: der Nutzer sucht sie auf seinem Ausdruck.
        for (text, nummer) in [
            (super::VERSORGUNG_SPERRE_STERBEGELD, "(Nummer 32 der Lohnsteuerbescheinigung)"),
            (super::VERSORGUNG_SPERRE_MEHRERE_JAHRE, "(Nummer 9 der Lohnsteuerbescheinigung)"),
            (super::VERSORGUNG_SPERRE_NICHT_GANZJAEHRIG, "(Nummer 31 der Lohnsteuerbescheinigung)"),
        ] {
            assert!(text.contains(nummer), "der Text nennt {nummer} nicht: {text}");
        }
        let faelle = [
            ("versorgung_zweiter_bezug", super::VERSORGUNG_SPERRE_ZWEITER_BEZUG, "zwei verschiedene Versorgungsbezüge"),
            ("versorgung_sterbegeld", super::VERSORGUNG_SPERRE_STERBEGELD, "Sterbegeld"),
            ("versorgung_mehrere_jahre", super::VERSORGUNG_SPERRE_MEHRERE_JAHRE, "mehrere Kalenderjahre"),
            ("versorgung_nicht_ganzjaehrig", super::VERSORGUNG_SPERRE_NICHT_GANZJAEHRIG, "zwölf Monaten"),
        ];
        for partner in [false, true] {
            let s = if partner { "_partner" } else { "" };
            let fremd = if partner { "" } else { "_partner" };
            for (name, text, merkmal) in faelle {
                let eigen = format!("{name}{s}");
                // (Antwort, Zustand, Bezug in CENT, Veranlagung, Wirkung: "ja" = Text des Falls, "offen" = Text der unbeantworteten
                // Frage, "nichts" = kein Grund unter dem Feld)
                let reihen = [
                    (Some(json!(true)), b, 3_000_000, "zusammen", "ja"),
                    (Some(json!(false)), b, 3_000_000, "zusammen", "nichts"),
                    (None, b, 3_000_000, "zusammen", "offen"),
                    (Some(json!(true)), Zustand::Vorlaeufig, 3_000_000, "zusammen", "offen"),
                    (Some(json!(false)), Zustand::Vorlaeufig, 3_000_000, "zusammen", "offen"),
                    (Some(json!("ja")), b, 3_000_000, "zusammen", "offen"),
                    (Some(json!(1)), b, 3_000_000, "zusammen", "offen"),
                    // Ohne Bezug ueber 0 ist die Frage verborgen: weder ein gespeichertes Ja noch Schweigen sperrt.
                    (Some(json!(true)), b, 0, "zusammen", "nichts"),
                    (None, b, 0, "zusammen", "nichts"),
                    // Der Ehegatte zaehlt nur bei bestaetigter Zusammenveranlagung, auch mit Schweigen.
                    (Some(json!(true)), b, 3_000_000, "einzel", if partner { "nichts" } else { "ja" }),
                    (None, b, 3_000_000, "einzel", if partner { "nichts" } else { "offen" }),
                ];
                for (antwort, zustand, bezug, veranlagung, soll) in reihen {
                    let mut felder = Felder::from([
                        (format!("versorgung_jahresrente{s}"), feld(json!(bezug), b)),
                        (format!("versorgung_bemessungsgrundlage{s}"), feld(json!(2_500_000), b)),
                        (format!("versorgung_beginn_jahr{s}"), feld(json!(2020), b)),
                        (format!("versorgung_art{s}"), feld(json!("beamtenrechtlich"), b)),
                        (format!("versorgung_zeile{s}"), feld(json!(true), b)),
                        ("veranlagung".to_owned(), feld(json!(veranlagung), b)),
                    ]);
                    if let Some(w) = &antwort {
                        felder.insert(eigen.clone(), feld(w.clone(), zustand));
                    }
                    let d = deklariere(&felder, &index, 2025, None).unwrap();
                    let wo = format!("Person {}, {eigen} = {antwort:?} {zustand:?}, Bezug {bezug}, {veranlagung}", if partner { "B" } else { "A" });
                    // Nur die Gruende unter dem Feld DIESER Frage: die drei anderen Fragen der Person sind hier unbeantwortet.
                    let gesperrt: Vec<&str> = d
                        .unvollstaendig()
                        .iter()
                        .filter(|e| e.feld_id == eigen && e.grund.starts_with("Versorgungsbezüge über 0 Euro"))
                        .map(|e| e.grund.as_str())
                        .collect();
                    match soll {
                        "ja" => assert_eq!(gesperrt, [text], "{wo}"),
                        "offen" => {
                            assert_eq!(gesperrt.len(), 1, "{wo}: {gesperrt:?}");
                            let grund = gesperrt[0];
                            assert!(grund.contains(UNBEANTWORTET), "{wo}: der Text nennt die fehlende Antwort nicht: {grund}");
                            assert!(grund.contains(merkmal), "{wo}: der Text nennt das Merkmal {merkmal} nicht: {grund}");
                            assert!(grund.contains(SPERRE_BIS_ANTWORT), "{wo}: der Text nennt die Sperre bis zur Antwort nicht: {grund}");
                            assert_ne!(grund, text, "{wo}: der Text ist der des Ja");
                        }
                        _ => assert!(gesperrt.is_empty(), "{wo}: {gesperrt:?}"),
                    }
                    // Die Rueckfrage ist kein Feld der Erklaerung: mit Grund in `nicht_deklariert`, ein bestaetigtes Ja steht in keinem Kz.
                    if antwort.is_some() && zustand == b {
                        assert_eq!(d.nicht_deklariert.iter().filter(|e| e.feld_id == eigen).count(), 1, "{wo}: nicht_deklariert");
                    }
                }
                // Die Antwort der anderen Person gilt nicht fuer diese: das Ja der anderen Person sperrt bei dieser nichts.
                let felder = Felder::from([
                    (format!("versorgung_jahresrente{s}"), feld(json!(3_000_000), b)),
                    (format!("versorgung_bemessungsgrundlage{s}"), feld(json!(2_500_000), b)),
                    (format!("versorgung_beginn_jahr{s}"), feld(json!(2020), b)),
                    (format!("versorgung_art{s}"), feld(json!("beamtenrechtlich"), b)),
                    (format!("versorgung_zeile{s}"), feld(json!(true), b)),
                    ("veranlagung".to_owned(), feld(json!("zusammen"), b)),
                    (format!("{name}{fremd}"), feld(json!(true), b)),
                ]);
                let d = deklariere(&felder, &index, 2025, None).unwrap();
                // Unter dem Feld der anderen Person steht nichts, und der Text des Ja steht nirgends (die vier Fragen DIESER Person sind
                // hier unbeantwortet und sperren mit dem Text der fehlenden Antwort).
                assert!(
                    d.unvollstaendig().iter().all(|e| e.feld_id != format!("{name}{fremd}") && e.grund != text),
                    "Person {}: das Ja der anderen Person sperrt: {:?}",
                    if partner { "B" } else { "A" },
                    d.unvollstaendig()
                );
            }
        }
    }

    /// Abweichung Nr. 79 (vorher Nr. 66, Grenze 3: "eine unbeantwortete Rueckfrage sperrt nicht"): eine unbeantwortete Rueckfrage sperrt
    /// mit EIGENEM Text unter ihrem Feld, ohne die Erklaerung zu aendern (kein stilles Anderes). Vier Nein sperren nichts (Nein ist der
    /// Normalfall); ohne jede Antwort stehen vier Gruende der fehlenden Antwort, vier Ja geben vier Gruende des Falls, beide in der
    /// Reihenfolge der Faelle; neben einem Grund des Bezugs selbst (unter 1 Euro) bleibt dieser stehen.
    #[test]
    fn eine_unbeantwortete_rueckfrage_sperrt_mit_eigenem_text() {
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
        let basis = |bezug: i64, zeile: bool| {
            Felder::from([
                ("versorgung_jahresrente".to_owned(), feld(json!(bezug))),
                ("versorgung_bemessungsgrundlage".to_owned(), feld(json!(2_500_000))),
                ("versorgung_beginn_jahr".to_owned(), feld(json!(2020))),
                ("versorgung_art".to_owned(), feld(json!("beamtenrechtlich"))),
                ("versorgung_zeile".to_owned(), feld(json!(zeile))),
            ])
        };
        let namen = ["versorgung_zweiter_bezug", "versorgung_sterbegeld", "versorgung_mehrere_jahre", "versorgung_nicht_ganzjaehrig"];
        let merkmale = ["zwei verschiedene Versorgungsbezüge", "Sterbegeld", "mehrere Kalenderjahre", "zwölf Monaten"];
        // Ohne jede Antwort: vier Gruende der fehlenden Antwort, jeder unter seinem Feld, jeder mit Merkmal und Sperre.
        let ohne = deklariere(&basis(3_000_000, true), &index, 2025, None).unwrap();
        let ids: Vec<&str> = ohne.unvollstaendig().iter().map(|e| e.feld_id.as_str()).collect();
        assert_eq!(ids, namen, "ohne Antwort: Reihenfolge und Felder der Gruende");
        for (e, merkmal) in ohne.unvollstaendig().iter().zip(merkmale) {
            assert!(
                e.grund.starts_with("Versorgungsbezüge über 0 Euro")
                    && e.grund.contains("nicht mit Ja oder Nein beantwortet")
                    && e.grund.contains(merkmal)
                    && e.grund.contains("Die Abgabe ist gesperrt, bis du die Frage beantwortest"),
                "{}: der Text der fehlenden Antwort ist unvollstaendig: {}",
                e.feld_id,
                e.grund
            );
        }
        // Vier Nein: nichts sperrt, und die Erklaerung ist dieselbe wie ohne Antwort (die Sperre aendert keine Zeile).
        let mut mit_nein = basis(3_000_000, true);
        for n in namen {
            mit_nein.insert(n.to_owned(), feld(json!(false)));
        }
        let nein = deklariere(&mit_nein, &index, 2025, None).unwrap();
        assert!(nein.unvollstaendig().is_empty(), "vier Nein sperren: {:?}", nein.unvollstaendig());
        assert_eq!(nein.deklaration, ohne.deklaration, "vier Nein aendern die Erklaerung");
        // Drei Nein und eine fehlende Antwort: genau die fehlende sperrt.
        for (i, n) in namen.iter().enumerate() {
            let mut f = mit_nein.clone();
            f.remove(*n);
            let d = deklariere(&f, &index, 2025, None).unwrap();
            let ids: Vec<&str> = d.unvollstaendig().iter().map(|e| e.feld_id.as_str()).collect();
            assert_eq!(ids, [*n], "nur {n} fehlt");
            assert!(d.unvollstaendig()[0].grund.contains(merkmale[i]), "{n}: {}", d.unvollstaendig()[0].grund);
        }
        // Vier Ja: vier Gruende, in der Reihenfolge der Faelle, jeder unter seinem Feld.
        let mut mit_ja = basis(3_000_000, true);
        for n in namen {
            mit_ja.insert(n.to_owned(), feld(json!(true)));
        }
        let ja = deklariere(&mit_ja, &index, 2025, None).unwrap();
        let ids: Vec<&str> = ja.unvollstaendig().iter().map(|e| e.feld_id.as_str()).collect();
        assert_eq!(ids, namen, "Reihenfolge und Felder der Gruende");
        assert_eq!(ja.deklaration, ohne.deklaration, "ein Ja aendert die Zeilen der Erklaerung nicht, es sperrt nur");
        // Ein Bezug unter 1 Euro (keine Zeile) UND ein Ja: der Grund des Bezugs steht neben dem der Rueckfrage.
        let mut klein = basis(99, false);
        for n in namen {
            klein.insert(n.to_owned(), feld(json!(false)));
        }
        klein.insert("versorgung_sterbegeld".to_owned(), feld(json!(true)));
        let d = deklariere(&klein, &index, 2025, None).unwrap();
        let gruende: Vec<(&str, &str)> = d.unvollstaendig().iter().map(|e| (e.feld_id.as_str(), e.grund.as_str())).collect();
        assert_eq!(
            gruende,
            [
                ("versorgung_jahresrente", super::VERSORGUNG_SPERRE),
                ("versorgung_sterbegeld", super::VERSORGUNG_SPERRE_STERBEGELD),
            ]
        );
    }

    /// Abweichung Nr. 85, 86: die Rueckfragen zum Lohn (mehrere Jahre/Entschaedigung, Auslandstaetigkeitserlass, Leistungen des
    /// Arbeitgebers) je Person. Ein bestaetigtes Ja
    /// gibt den Text des Falls unter dem Feld der Frage, ein bestaetigtes Nein keinen Grund, jede andere Lage (fehlend, vorlaeufig, nicht
    /// boolesch) den Text der fehlenden Antwort. Die Fragen gelten nur bei einem BESTAETIGTEN Bruttoarbeitslohn der Person ueber 0, die des
    /// Ehegatten nur bei bestaetigter Zusammenveranlagung.
    #[test]
    fn die_rueckfragen_zum_lohn_sperren_bei_ja_und_ohne_antwort_je_person() {
        const UNBEANTWORTET: &str = "nicht mit Ja oder Nein beantwortet";
        const SPERRE_BIS_ANTWORT: &str = "Die Abgabe ist gesperrt, bis du die Frage beantwortest";
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
        let b = Zustand::Bestaetigt;
        // Die Nummer der Lohnsteuerbescheinigung und die Zeile des Vordrucks stehen im Text: der Nutzer sucht sie auf seinem Ausdruck.
        assert!(super::LOHN_SPERRE_MEHRERE_JAHRE.contains("Nummer 10") && super::LOHN_SPERRE_MEHRERE_JAHRE.contains("Zeile 17"));
        assert!(super::LOHN_SPERRE_AUSLANDSTAETIGKEIT.contains("Zeile 22") && super::LOHN_SPERRE_AUSLANDSTAETIGKEIT.contains("Anlage N-AUS"));
        // Abweichung Nr. 86: die Zeilen der drei Stellen, die Nummern der Bescheinigung und die Sammelbefoerderung stehen im Text.
        let erst = super::LOHN_SPERRE_ERSTATTUNGEN;
        assert!(["Zeile 51", "Zeile 71", "Zeile 34 der Anlage N-DHF", "Sammelbeförderung", "Nummer 17 und 18"].iter().all(|m| erst.contains(m)), "{erst}");
        let faelle = [
            ("lohn_mehrere_jahre", super::LOHN_SPERRE_MEHRERE_JAHRE, "Abfindung"),
            ("lohn_auslandstaetigkeit", super::LOHN_SPERRE_AUSLANDSTAETIGKEIT, "Auslandstätigkeitserlass"),
            ("lohn_erstattungen", super::LOHN_SPERRE_ERSTATTUNGEN, "doppelten Haushaltsführung"),
        ];
        for partner in [false, true] {
            let s = if partner { "_partner" } else { "" };
            for (name, text, merkmal) in faelle {
                let eigen = format!("{name}{s}");
                // (Antwort, Zustand der Antwort, Lohn in CENT, Zustand des Lohns, Veranlagung, Wirkung: "ja" = Text des Falls, "offen" =
                // Text der unbeantworteten Frage, "nichts" = kein Grund unter dem Feld)
                let reihen = [
                    (Some(json!(true)), b, 4_000_000, b, "zusammen", "ja"),
                    (Some(json!(false)), b, 4_000_000, b, "zusammen", "nichts"),
                    (None, b, 4_000_000, b, "zusammen", "offen"),
                    (Some(json!(true)), Zustand::Vorlaeufig, 4_000_000, b, "zusammen", "offen"),
                    (Some(json!(false)), Zustand::Vorlaeufig, 4_000_000, b, "zusammen", "offen"),
                    (Some(json!("ja")), b, 4_000_000, b, "zusammen", "offen"),
                    (Some(json!(1)), b, 4_000_000, b, "zusammen", "offen"),
                    // Ohne Lohn ueber 0 (oder mit einem nur vorlaeufigen Lohn) ist die Frage verborgen: weder ein gespeichertes Ja noch
                    // Schweigen sperrt.
                    (Some(json!(true)), b, 0, b, "zusammen", "nichts"),
                    (None, b, 0, b, "zusammen", "nichts"),
                    (None, b, 4_000_000, Zustand::Vorlaeufig, "zusammen", "nichts"),
                    (Some(json!(true)), b, 4_000_000, Zustand::Vorlaeufig, "zusammen", "nichts"),
                    // Der Ehegatte zaehlt nur bei bestaetigter Zusammenveranlagung, auch mit Schweigen.
                    (Some(json!(true)), b, 4_000_000, b, "einzel", if partner { "nichts" } else { "ja" }),
                    (None, b, 4_000_000, b, "einzel", if partner { "nichts" } else { "offen" }),
                ];
                for (antwort, a_zustand, lohn, l_zustand, veranlagung, soll) in reihen {
                    let mut felder = Felder::from([
                        (format!("bruttoarbeitslohn{s}"), feld(json!(lohn), l_zustand)),
                        ("veranlagung".to_owned(), feld(json!(veranlagung), b)),
                    ]);
                    if let Some(w) = &antwort {
                        felder.insert(eigen.clone(), feld(w.clone(), a_zustand));
                    }
                    let d = deklariere(&felder, &index, 2025, None).unwrap();
                    let wo = format!("Person {}, {eigen} = {antwort:?} {a_zustand:?}, Lohn {lohn} {l_zustand:?}, {veranlagung}", if partner { "B" } else { "A" });
                    // Nur die Gruende unter dem Feld DIESER Frage: die andere Frage der Person ist hier unbeantwortet.
                    let gesperrt: Vec<&str> = d
                        .unvollstaendig()
                        .iter()
                        .filter(|e| e.feld_id == eigen && e.grund.starts_with("Arbeitslohn:"))
                        .map(|e| e.grund.as_str())
                        .collect();
                    match soll {
                        "ja" => assert_eq!(gesperrt, [text], "{wo}"),
                        "offen" => {
                            assert_eq!(gesperrt.len(), 1, "{wo}: {gesperrt:?}");
                            let grund = gesperrt[0];
                            assert!(grund.contains(UNBEANTWORTET), "{wo}: der Text nennt die fehlende Antwort nicht: {grund}");
                            assert!(grund.contains(merkmal), "{wo}: der Text nennt das Merkmal {merkmal} nicht: {grund}");
                            assert!(grund.contains(SPERRE_BIS_ANTWORT), "{wo}: der Text nennt die Sperre bis zur Antwort nicht: {grund}");
                            assert_ne!(grund, text, "{wo}: der Text ist der des Ja");
                        }
                        _ => assert!(gesperrt.is_empty(), "{wo}: {gesperrt:?}"),
                    }
                    // Die Rueckfrage ist kein Feld der Erklaerung: mit Grund in `nicht_deklariert`, ein bestaetigtes Ja steht in keinem Kz.
                    if antwort.is_some() && a_zustand == b {
                        assert_eq!(d.nicht_deklariert.iter().filter(|e| e.feld_id == eigen).count(), 1, "{wo}: nicht_deklariert");
                    }
                }
            }
        }
        // Die Antwort der anderen Person gilt nicht fuer diese: das Ja des Ehegatten sperrt Person A nicht und umgekehrt.
        let felder = Felder::from([
            ("bruttoarbeitslohn".to_owned(), feld(json!(4_000_000), b)),
            ("bruttoarbeitslohn_partner".to_owned(), feld(json!(3_000_000), b)),
            ("veranlagung".to_owned(), feld(json!("zusammen"), b)),
            ("lohn_mehrere_jahre_partner".to_owned(), feld(json!(true), b)),
            ("lohn_mehrere_jahre".to_owned(), feld(json!(false), b)),
            ("lohn_auslandstaetigkeit".to_owned(), feld(json!(false), b)),
            ("lohn_auslandstaetigkeit_partner".to_owned(), feld(json!(false), b)),
            ("lohn_erstattungen".to_owned(), feld(json!(false), b)),
            ("lohn_erstattungen_partner".to_owned(), feld(json!(false), b)),
        ]);
        let d = deklariere(&felder, &index, 2025, None).unwrap();
        let ids: Vec<&str> = d.unvollstaendig().iter().map(|e| e.feld_id.as_str()).collect();
        assert_eq!(ids, ["lohn_mehrere_jahre_partner"], "nur das Ja des Ehegatten sperrt, unter seinem Feld");
    }

    /// Abweichung Nr. 85, 86: ohne jede Antwort stehen die Gruende der fehlenden Antwort je Person in der Reihenfolge der Fragen, Nein
    /// sperrt nichts und aendert die Erklaerung nicht, Ja gibt je Frage den Grund des Falls. Neben einem Grund anderer Herkunft bleibt
    /// dieser stehen (die Pruefung ersetzt keinen Grund).
    #[test]
    fn eine_unbeantwortete_lohnfrage_sperrt_mit_eigenem_text_ohne_die_erklaerung_zu_aendern() {
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
        let basis = |zusammen: bool| {
            let mut f = Felder::from([("bruttoarbeitslohn".to_owned(), feld(json!(4_000_000)))]);
            if zusammen {
                f.insert("veranlagung".to_owned(), feld(json!("zusammen")));
                f.insert("bruttoarbeitslohn_partner".to_owned(), feld(json!(3_000_000)));
            }
            f
        };
        let a_namen = ["lohn_mehrere_jahre", "lohn_auslandstaetigkeit", "lohn_erstattungen"];
        let b_namen = ["lohn_mehrere_jahre_partner", "lohn_auslandstaetigkeit_partner", "lohn_erstattungen_partner"];
        let ids = |d: &Deklaration| d.unvollstaendig().iter().map(|e| e.feld_id.clone()).collect::<Vec<String>>();
        // Ohne jede Antwort: Person A zuerst, dann der Ehegatte, in der Reihenfolge der Fragen; jeder Grund nennt Antwort und Sperre.
        let ohne = deklariere(&basis(true), &index, 2025, None).unwrap();
        assert_eq!(ids(&ohne), [a_namen, b_namen].concat(), "ohne Antwort: Reihenfolge und Felder der Gruende");
        for e in ohne.unvollstaendig() {
            assert!(
                e.grund.starts_with("Arbeitslohn:")
                    && e.grund.contains("nicht mit Ja oder Nein beantwortet")
                    && e.grund.contains("Die Abgabe ist gesperrt, bis du die Frage beantwortest"),
                "{}: der Text der fehlenden Antwort ist unvollstaendig: {}",
                e.feld_id,
                e.grund
            );
        }
        // Nur Person A mit Lohn (Einzelveranlagung): drei Gruende, keiner des Ehegatten.
        let einzel = deklariere(&basis(false), &index, 2025, None).unwrap();
        assert_eq!(ids(&einzel), a_namen, "Einzelveranlagung: nur Person A");
        // Sechsmal Nein: nichts sperrt, und die Erklaerung ist dieselbe wie ohne Antwort (die Sperre aendert keine Zeile).
        let mut nein = basis(true);
        for n in a_namen.iter().chain(&b_namen) {
            nein.insert((*n).to_owned(), feld(json!(false)));
        }
        let mit_nein = deklariere(&nein, &index, 2025, None).unwrap();
        assert!(mit_nein.unvollstaendig().is_empty(), "sechsmal Nein sperren: {:?}", mit_nein.unvollstaendig());
        assert_eq!(mit_nein.deklaration, ohne.deklaration, "Nein aendert die Erklaerung");
        assert_eq!(mit_nein.person_b, ohne.person_b, "Nein aendert die zweite Anlage N");
        // Fuenf Nein und eine fehlende Antwort: genau die fehlende sperrt.
        for n in a_namen.iter().chain(&b_namen) {
            let mut f = nein.clone();
            f.remove(*n);
            let d = deklariere(&f, &index, 2025, None).unwrap();
            assert_eq!(ids(&d), [(*n).to_owned()], "nur {n} fehlt");
        }
        // Sechs Ja: sechs Gruende des Falls in der Reihenfolge der Fragen, die Erklaerung bleibt gleich.
        let mut ja = basis(true);
        for n in a_namen.iter().chain(&b_namen) {
            ja.insert((*n).to_owned(), feld(json!(true)));
        }
        let mit_ja = deklariere(&ja, &index, 2025, None).unwrap();
        assert_eq!(ids(&mit_ja), [a_namen, b_namen].concat(), "sechs Ja: Reihenfolge und Felder der Gruende");
        let texte: Vec<&str> = mit_ja.unvollstaendig().iter().map(|e| e.grund.as_str()).collect();
        let je_person = [super::LOHN_SPERRE_MEHRERE_JAHRE, super::LOHN_SPERRE_AUSLANDSTAETIGKEIT, super::LOHN_SPERRE_ERSTATTUNGEN];
        assert_eq!(texte, [je_person, je_person].concat());
        assert_eq!(mit_ja.deklaration, ohne.deklaration, "ein Ja aendert die Zeilen der Erklaerung nicht, es sperrt nur");
        // Neben einem Grund anderer Herkunft bleibt dieser stehen: die Rueckfrage zum Versorgungsbezug und die zum Lohn sperren getrennt.
        let mut beides = nein.clone();
        beides.insert("versorgung_jahresrente".to_owned(), feld(json!(99)));
        beides.insert("lohn_auslandstaetigkeit".to_owned(), feld(json!(true)));
        let d = deklariere(&beides, &index, 2025, None).unwrap();
        let gruende: Vec<(&str, &str)> = d.unvollstaendig().iter().map(|e| (e.feld_id.as_str(), e.grund.as_str())).collect();
        assert!(
            gruende.contains(&("versorgung_jahresrente", super::VERSORGUNG_SPERRE))
                && gruende.contains(&("lohn_auslandstaetigkeit", super::LOHN_SPERRE_AUSLANDSTAETIGKEIT)),
            "{gruende:?}"
        );
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

    /// Abweichung Nr. 54 (Anlage SO, Zeile 36): ein selbst bewohntes Grundstueck traegt Haken, Zeitraum und Flaeche und den Gewinn 0,
    /// jedes der drei Kz-Literale von `P23_EIGENNUTZUNG_KZ` steht hier mit seinem Wert (Regal: `Verhalten`). Vier Gegenproben in einer
    /// Akte, damit sie sich nicht gegenseitig verdecken: Instanz 2 ist "nein" (Gewinn voll, kein Kz der Zeile 36), Instanz 3 ein anderes
    /// Wirtschaftsgut mit einer liegengebliebenen Eigennutzung, Instanz 4 `letzte_drei_jahre` (Suffix `__4`), Instanz 5 `teilweise`
    /// (die Teilnutzung traegt keine Zeile 36; sie sperrt schon vorher in `bescheid`). Fehlt der Zeitraum, steht die Zeile 36 offen und der
    /// Gewinn nicht in der Erklaerung.
    #[test]
    fn selbst_bewohntes_grundstueck_traegt_haken_zeitraum_und_flaeche_und_den_gewinn_null() {
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
        // (Suffix, Art, Eigennutzung); Preis 200.000, Kosten 150.000 EUR: der volle Gewinn waere 50.000 EUR.
        let faelle = [
            ("", "grundstueck", "ausschliesslich"),
            ("__2", "grundstueck", "nein"),
            ("__3", "anderes_wg", "ausschliesslich"),
            ("__4", "grundstueck", "letzte_drei_jahre"),
            ("__5", "grundstueck", "teilweise"),
        ];
        let mut felder = Felder::new();
        for (suffix, typ, eigen) in faelle {
            for (f, w) in [
                ("p23_veraeusserungs_typ", json!(typ)),
                ("p23_bezeichnung", json!("Haus")),
                ("p23_anschaffungsdatum", json!("15.06.2020")),
                ("p23_veraeusserungsdatum", json!("10.03.2025")),
                ("p23_veraeusserungspreis", json!(20_000_000)),
                ("p23_anschaffung_herstellungskosten", json!(15_000_000)),
                ("p23_eigennutzung", json!(eigen)),
                ("p23_wohnflaeche", json!(120)),
                ("p23_eigennutzung_von", json!("01.03.2020")),
                ("p23_eigennutzung_bis", json!("09.03.2025")),
            ] {
                felder.insert(format!("{f}{suffix}"), feld(w));
            }
        }
        let d = deklariere(&felder, &index, 2025, None).unwrap();
        let (_, instanzen) = &d.anlage_instanzen[0];
        let kz = |i: usize| -> BTreeMap<&str, Value> { instanzen[i].felder.iter().map(|(k, v)| (k.as_str(), v.clone())).collect() };
        assert_eq!(instanzen.len(), 5);
        assert_eq!(
            kz(0),
            BTreeMap::from([
                ("E0306111", json!("Haus")),
                ("E0306201", json!("15.06.2020")),
                ("E0306202", json!("10.03.2025")),
                ("E0306301", json!(true)),
                ("E0306302", json!("01.03.2020-09.03.2025")),
                ("E0306303", json!(120)),
                ("E0306801", json!(0)),
            ])
        );
        let voll = BTreeMap::from([
            ("E0306111", json!("Haus")),
            ("E0306201", json!("15.06.2020")),
            ("E0306202", json!("10.03.2025")),
            ("E0306801", json!(50_000)),
        ]);
        assert_eq!(kz(1), voll, "Eigennutzung nein: voller Gewinn, keine Zeile 36");
        assert_eq!(
            kz(2),
            BTreeMap::from([
                ("E0307101", json!("Haus")),
                ("E0307201", json!("15.06.2020")),
                ("E0307202", json!("10.03.2025")),
                ("E0307701", json!(50_000)),
            ]),
            "anderes Wirtschaftsgut: die liegengebliebene Eigennutzung zaehlt nicht"
        );
        assert_eq!(kz(3).get("E0306302"), Some(&json!("01.03.2020-09.03.2025")), "Instanz 4 traegt ihren eigenen Zeitraum");
        assert_eq!(kz(3).get("E0306801"), Some(&json!(0)));
        assert_eq!(kz(4), voll, "Teilnutzung: keine Zeile 36, der Gewinn bleibt voll");
        assert!(d.eingaben_konsistent(), "{:?}", d.unvollstaendig());
        // Wohnflaeche und Zeitraum sind bekannte Felder: sie stehen nicht unter "nicht deklariert" (nur das Art-Feld selbst steht dort).
        let fremd: Vec<&str> = d
            .nicht_deklariert
            .iter()
            .map(|e| e.feld_id.as_str())
            .filter(|f| f.starts_with("p23_") && !f.starts_with("p23_veraeusserungs_typ"))
            .collect();
        assert!(fremd.is_empty(), "{fremd:?}");

        // Ohne Zeitraum ist die Zeile 36 offen: ein Eintrag in `unvollstaendig`, und der Gewinn steht nicht in der Erklaerung.
        felder.remove("p23_eigennutzung_bis");
        let d = deklariere(&felder, &index, 2025, None).unwrap();
        assert!(!d.eingaben_konsistent());
        let offen: Vec<&str> = d.unvollstaendig().iter().map(|e| e.feld_id.as_str()).collect();
        assert_eq!(offen, ["p23_veraeusserung__1"], "{offen:?}");
        let erste = d.anlage_instanzen[0].1.iter().find(|i| i.index == 1);
        assert!(erste.is_none_or(|i| !i.felder.contains_key("E0306801")), "Gewinn trotz offener Zeile 36");
    }

    /// Abweichung Nr. 54, Nachmessung `a2c47914` (Mutanten E07 und E11): Zeile 36 liest nur bestaetigte Angaben, und eine fehlende
    /// Wohnflaeche laesst sie offen, statt den vollen Gewinn ohne Zeile 36 zu schreiben. Je Feld und Zustand ein Fall, damit keiner den
    /// anderen verdeckt. Die Eigennutzung selbst, nur vorlaeufig, traegt keinen Haken.
    #[test]
    fn zeile_36_liest_nur_bestaetigte_angaben_und_eine_fehlende_flaeche_laesst_sie_offen() {
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
        // Ein selbst bewohntes Grundstueck, alle Angaben bestaetigt; `aendere` setzt ein Feld auf `Zustand` oder (None) nimmt es weg.
        let deklaration = |aendere: Option<(&str, Option<Zustand>)>| {
            let mut felder = Felder::new();
            for (f, w) in [
                ("p23_veraeusserungs_typ", json!("grundstueck")),
                ("p23_bezeichnung", json!("Haus")),
                ("p23_anschaffungsdatum", json!("15.06.2020")),
                ("p23_veraeusserungsdatum", json!("10.03.2025")),
                ("p23_veraeusserungspreis", json!(20_000_000)),
                ("p23_anschaffung_herstellungskosten", json!(15_000_000)),
                ("p23_eigennutzung", json!("ausschliesslich")),
                ("p23_wohnflaeche", json!(120)),
                ("p23_eigennutzung_von", json!("01.03.2020")),
                ("p23_eigennutzung_bis", json!("09.03.2025")),
            ] {
                felder.insert(f.to_owned(), feld(w, Zustand::Bestaetigt));
            }
            if let Some((f, zustand)) = aendere {
                match zustand {
                    Some(z) => felder.get_mut(f).unwrap().zustand = z,
                    None => {
                        felder.remove(f);
                    }
                }
            }
            deklariere(&felder, &index, 2025, None).unwrap()
        };
        let kz_der_ersten = |d: &Deklaration| -> Vec<String> {
            d.anlage_instanzen[0]
                .1
                .iter()
                .find(|i| i.index == 1)
                .map(|i| i.felder.keys().cloned().collect())
                .unwrap_or_default()
        };

        // KONTROLLE: alles bestaetigt, dann steht Zeile 36 mit dem Gewinn 0 und nichts ist offen.
        let ganz = deklaration(None);
        assert!(ganz.eingaben_konsistent(), "{:?}", ganz.unvollstaendig());
        assert!(kz_der_ersten(&ganz).contains(&"E0306301".to_owned()), "{:?}", kz_der_ersten(&ganz));

        for f in ["p23_wohnflaeche", "p23_eigennutzung_von", "p23_eigennutzung_bis"] {
            for zustand in [None, Some(Zustand::Vorlaeufig)] {
                let d = deklaration(Some((f, zustand)));
                // Ein vorlaeufiges Feld steht zusaetzlich unter seinem eigenen Namen offen; die Instanz steht je Fall genau einmal da.
                let offen: Vec<&str> = d
                    .unvollstaendig()
                    .iter()
                    .filter(|e| e.feld_id == "p23_veraeusserung__1")
                    .map(|e| e.grund.as_str())
                    .collect();
                assert_eq!(offen.len(), 1, "{f} {zustand:?}: {:?}", d.unvollstaendig());
                assert!(offen[0].contains(f), "{f} {zustand:?}: Grund nennt das Feld nicht: {}", offen[0]);
                let kz = kz_der_ersten(&d);
                assert!(!kz.contains(&"E0306801".to_owned()), "{f} {zustand:?}: Gewinn trotz offener Zeile 36: {kz:?}");
                assert!(!kz.contains(&"E0306301".to_owned()), "{f} {zustand:?}: Haken trotz offener Zeile 36: {kz:?}");
            }
        }

        // Nur vorlaeufig bestaetigt die Eigennutzung nichts: kein Haken, kein Zeitraum, keine Flaeche in der Erklaerung.
        let kz = kz_der_ersten(&deklaration(Some(("p23_eigennutzung", Some(Zustand::Vorlaeufig)))));
        for nutzung_kz in P23_EIGENNUTZUNG_KZ {
            assert!(!kz.contains(&nutzung_kz.to_owned()), "vorlaeufige Eigennutzung schreibt {nutzung_kz}: {kz:?}");
        }
    }

    /// Abweichung Nr. 54: Bezeichnung und beide Daten eines Privatverkaufs stehen unter dem Kz seiner Art, der Gewinn nach AfA-Minderung
    /// der Kosten unter dem Gewinn-Kz; die Rechenfelder (Eigennutzung, taeglicher Gebrauch, Einkunftsquelle) schreiben nichts. Jedes
    /// der sechs Kz-Literale von `P23_ARTFELDER` steht hier mit seinem Wert: ein anderes Literal macht den Test rot (Regal: `Verhalten`).
    /// Gemessen am `checkESt`-Spike AK0 (`bescheid/tests/checkest_privatverkauf_spike.rs`).
    #[test]
    fn privatverkauf_schreibt_bezeichnung_und_daten_an_das_kz_der_art() {
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
        // Zwei Verkaeufe: Instanz 1 ein Grundstueck (ohne Suffix), Instanz 2 ein anderes Wirtschaftsgut (`__2`).
        let mut felder = Felder::new();
        for (suffix, typ, name, kauf, verkauf) in [
            ("", "grundstueck", "Wohnung Musterstrasse 1", "15.06.2020", "10.03.2025"),
            ("__2", "anderes_wg", "Goldbarren", "01.02.2025", "20.08.2025"),
        ] {
            for (f, w) in [
                ("p23_veraeusserungs_typ", json!(typ)),
                ("p23_bezeichnung", json!(name)),
                ("p23_anschaffungsdatum", json!(kauf)),
                ("p23_veraeusserungsdatum", json!(verkauf)),
                ("p23_veraeusserungspreis", json!(20_000_000)),
                ("p23_anschaffung_herstellungskosten", json!(15_000_000)),
                ("p23_afa", json!(1_000_000)),
                ("p23_werbungskosten", json!(500_000)),
                ("p23_eigennutzung", json!("nein")),
                ("p23_taeglicher_gebrauch", json!(false)),
                ("p23_einkunftsquelle", json!(false)),
                ("p23_gebaeude_in_frist", json!(false)),
            ] {
                felder.insert(format!("{f}{suffix}"), feld(w));
            }
        }
        let d = deklariere(&felder, &index, 2025, None).unwrap();
        let (gruppe, instanzen) = &d.anlage_instanzen[0];
        assert_eq!(gruppe, "p23_veraeusserung");
        assert_eq!(instanzen.len(), 2);
        // 200.000 - (150.000 - 10.000) - 5.000 = 55.000 EUR in beiden Verkaeufen.
        let grundstueck: BTreeMap<_, _> = instanzen[0].felder.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        assert_eq!(
            grundstueck,
            BTreeMap::from([
                ("E0306111", json!("Wohnung Musterstrasse 1")),
                ("E0306201", json!("15.06.2020")),
                ("E0306202", json!("10.03.2025")),
                ("E0306801", json!(55_000)),
            ])
        );
        let wirtschaftsgut: BTreeMap<_, _> = instanzen[1].felder.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        assert_eq!(
            wirtschaftsgut,
            BTreeMap::from([
                ("E0307101", json!("Goldbarren")),
                ("E0307201", json!("01.02.2025")),
                ("E0307202", json!("20.08.2025")),
                ("E0307701", json!(55_000)),
            ])
        );
        // Die Rechenfelder erzeugen weder eine Luecke noch einen Eintrag "nicht deklariert"; nur das Art-Feld selbst steht dort (wie vorher).
        assert!(d.eingaben_konsistent(), "{:?}", d.unvollstaendig());
        let fremd: Vec<&str> = d
            .nicht_deklariert
            .iter()
            .map(|e| e.feld_id.as_str())
            .filter(|f| f.starts_with("p23_") && !f.starts_with("p23_veraeusserungs_typ"))
            .collect();
        assert!(fremd.is_empty(), "{fremd:?}");
    }

    /// Abweichung Nr. 55: eine Leistungsmitteilung (Anlage R-AV/bAV) steht je Instanz mit den Kz ihrer Zeilen 4, 10 + 12 und 15 + 16.
    /// Instanz 1 steht in `deklaration`, Instanz 2 in den Anlage-Instanzen. Ein Betrag unter einem Euro und ein Betrag 0 stehen nicht in
    /// der Erklaerung, und der Beginn steht nur zu einem Betrag ueber 0: `ERiC` verlangt beide gemeinsam (Spike Nr. 55, Faelle B und K1).
    /// Die zwei Rechenfelder (weitere Nummern, Basisrente) schreiben nichts. Instanz 3 traegt nichts, ihre Beginn-Felder stehen trotzdem
    /// im Speicher.
    #[test]
    fn rav_bav_schreibt_betraege_und_beginn_nur_zu_einem_betrag_ueber_null() {
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
        // Instanz 1 traegt alle drei Nummern (Cent), Instanz 2 nur Nummer 1 und Nullen mit Beginn, Instanz 3 Betraege unter einem Euro
        // mit Beginn.
        let mut felder = Felder::new();
        for (suffix, nr1, nr4, nr5) in [("", 600_000, 300_000, 1_000_000), ("__2", 250_000, 0, 0), ("__3", 50, 99, 1)] {
            for (f, w) in [
                ("rav_bav_nr1_betrag", json!(nr1)),
                ("rav_bav_nr4_betrag", json!(nr4)),
                ("rav_bav_nr4_beginn", json!("01.04.2022")),
                ("rav_bav_nr5_betrag", json!(nr5)),
                ("rav_bav_nr5_beginn", json!("05.05.2020")),
                ("rav_bav_andere_nummern", json!(false)),
                ("rav_bav_basisrente", json!(false)),
            ] {
                felder.insert(format!("{f}{suffix}"), feld(w));
            }
        }
        let d = deklariere(&felder, &index, 2025, None).unwrap();
        let erste: BTreeMap<&str, Value> = ["E1803101", "E1803301", "E1803401", "E1803501", "E1803601"]
            .into_iter()
            .filter_map(|kz| d.deklaration.get(kz).map(|v| (kz, v.clone())))
            .collect();
        assert_eq!(
            erste,
            BTreeMap::from([
                ("E1803101", json!(6_000)),
                ("E1803301", json!(3_000)),
                ("E1803401", json!("01.04.2022")),
                ("E1803501", json!(10_000)),
                ("E1803601", json!("05.05.2020")),
            ])
        );
        let (gruppe, instanzen) = &d.anlage_instanzen[0];
        assert_eq!(gruppe, "rente_av_bav");
        assert_eq!(d.anlage_instanzen.len(), 1);
        assert_eq!(instanzen.len(), 1, "Instanz 3 ohne Kz faellt weg: {instanzen:?}");
        assert_eq!(instanzen[0].index, 2);
        let zweite: BTreeMap<&str, Value> = instanzen[0].felder.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
        assert_eq!(zweite, BTreeMap::from([("E1803101", json!(2_500))]));
        // Die Rechenfelder erzeugen weder eine Luecke noch einen Eintrag "nicht deklariert".
        assert!(d.eingaben_konsistent(), "{:?}", d.unvollstaendig());
        let fremd: Vec<&str> = d
            .nicht_deklariert
            .iter()
            .map(|e| e.feld_id.as_str())
            .filter(|f| f.starts_with("rav_bav_"))
            .collect();
        assert!(fremd.is_empty(), "{fremd:?}");
    }

    /// § 23-Gewinn = Preis − AK/HK − WK ausserhalb von `i64`: Fehler statt Umbruch.
    ///
    /// GEWOLLTE ABWEICHUNG von Python, das exakt rechnet. Mit `wrapping_sub` ergab Preis 0 bei
    /// AK = WK = 9e18 ct **+446744073709551616** ct statt eines Verlusts von 1,8e19 ct (das
    /// Vorzeichen kippte). Ueber HTTP ist die Stelle nicht erreichbar (bericht h8-casts: damals
    /// sperrte der Guard § 23 mit 409; seit Abweichung Nr. 54 rechnet `bescheid::privatverkauf`
    /// den Gewinn vorher in `i128` und meldet den Ueberlauf), darum der Test auf Funktionsebene.
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

    // ---- Mutationsmessung von `deklaration.rs` (Entscheidung d-45): 18 Ueberlebende ----
    //
    // `cargo-mutants` (20.10.2026, Basis `7675fcc3`) liess an diesen Stellen 18 Aenderungen durch, die auch
    // `cargo test --workspace` nicht bemerkte. Die Tests unten nennen je Mutant die Zeile bei Basis `7675fcc3` und die
    // Aenderung; die Nummern sind die des Vault-Audits `cargo-mutants-deklaration-rs-18-ueberlebende-2026-10-20`.
    // Die Erwartungstexte stehen als eigene Literale da, nie ueber dieselbe Funktion berechnet.

    fn bindungen_der_wurzel() -> Vec<bindung::Bindung> {
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    }

    fn eingabe(wert: Value, zustand: Zustand) -> SnapshotFeld {
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        SnapshotFeld {
            wert: wert.into(),
            zustand,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        }
    }

    fn bestaetigt(wert: Value) -> SnapshotFeld {
        eingabe(wert, Zustand::Bestaetigt)
    }

    fn felder_aus<const N: usize>(eintraege: [(&str, SnapshotFeld); N]) -> Felder {
        eintraege
            .into_iter()
            .map(|(feld_id, sfeld)| (feld_id.to_owned(), sfeld))
            .collect()
    }

    /// Die Gruende aller Eintraege zu `feld_id`, in der Reihenfolge der Liste.
    fn gruende<'a>(liste: &'a [super::Eintrag], feld_id: &str) -> Vec<&'a str> {
        liste
            .iter()
            .filter(|e| e.feld_id == feld_id)
            .map(|e| e.grund.as_str())
            .collect()
    }

    const IBAN_DE: &str = "DE89370400440532013000";
    const IBAN_GB: &str = "GB82WEST12345698765432";

    /// Mutanten 1, 2, 3 (`Bau::iban`, Zeilen 1539, 1542, 1549: je ein `!` geloescht) und 7 (`Bau::bankverbindung`,
    /// Zeile 1943: `||` zu `&&`). Eine gueltige IBAN steht ohne Leerraum und in Grossbuchstaben unter dem Kz ihres Landes
    /// (`E0102102` fuer DE, `E0102603` sonst), und der Kontoinhaber `E0101601` folgt ihr. Vor dem Test hatte `Bau::iban` keinen
    /// Test: `iban_muster` und `iban_pruefziffer_gueltig` einzeln waren gedeckt, ihre Verdrahtung nicht.
    #[test]
    fn eine_gueltige_iban_steht_ohne_leerraum_unter_dem_kz_ihres_landes() {
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        for (getippt, kz, anderes_kz, norm) in [
            (IBAN_DE, "E0102102", "E0102603", IBAN_DE),
            ("de89 3704 0044 0532 0130 00", "E0102102", "E0102603", IBAN_DE),
            ("  DE89\t3704\u{a0}0044 0532 0130 00\n", "E0102102", "E0102603", IBAN_DE),
            (IBAN_GB, "E0102603", "E0102102", IBAN_GB),
            ("gb82 west 1234 5698 7654 32", "E0102603", "E0102102", IBAN_GB),
        ] {
            let felder = felder_aus([("stammdaten_iban", bestaetigt(json!(getippt)))]);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            assert_eq!(d.deklaration.get(kz), Some(&json!(norm)), "{getippt:?}");
            assert!(!d.deklaration.contains_key(anderes_kz), "{getippt:?}: {:?}", d.deklaration);
            assert_eq!(d.deklaration.get("E0101601"), Some(&json!(true)), "{getippt:?}: Kontoinhaber");
            assert!(d.unvollstaendig().is_empty(), "{getippt:?}: {:?}", d.unvollstaendig());
        }
    }

    /// Mutanten 1, 2, 3 von der anderen Seite: eine IBAN mit falschem Muster oder falscher Pruefziffer kommt NIE in die
    /// Erklaerung (kein `E0102102`, kein `E0102603`, kein Kontoinhaber) und sperrt mit dem Grund der Pruefung, der den Wert
    /// nicht nennt. Die Pruefziffer-Faelle haben das richtige Muster: nur die Modulo-97-Pruefung weist sie ab.
    #[test]
    fn eine_ungueltige_iban_bleibt_draussen_und_sperrt_mit_dem_grund_der_pruefung() {
        const MUSTER: &str = "IBAN entspricht nicht dem amtlichen Muster (2 Buchstaben Laendercode + 2 Pruefziffern + 1-30 alphanumerische Zeichen, String_MinL5_MaxL34_Musterm188614856_CType, E10-2025.xsd) -- Wert nicht geloggt.";
        const PRUEFZIFFER: &str = "IBAN-Pruefziffer (ISO 13616, Modulo 97) ungueltig -- Wert nicht geloggt.";
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        for (getippt, grund) in [
            ("DE8", MUSTER),
            ("12345678", MUSTER),
            ("DE89-3704-0044-0532-0130-00", MUSTER),
            ("DE88370400440532013000", PRUEFZIFFER),
            ("de88 3704 0044 0532 0130 00", PRUEFZIFFER),
            ("GB83WEST12345698765432", PRUEFZIFFER),
        ] {
            let felder = felder_aus([("stammdaten_iban", bestaetigt(json!(getippt)))]);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            assert_eq!(d.unvollstaendig(), [super::Eintrag::neu("stammdaten_iban", grund)], "{getippt:?}");
            for kz in ["E0102102", "E0102603", "E0101601"] {
                assert!(!d.deklaration.contains_key(kz), "{getippt:?}: {kz} in {:?}", d.deklaration);
            }
        }
    }

    /// Mutanten 7 und 8 (`Bau::bankverbindung`, Zeilen 1943 und 1944: `||` zu `&&`, `==` zu `!=`). IBAN und
    /// "keine Bankverbindung" zugleich sind ein Widerspruch, den `checkESt` mit rc=610001002 ablehnt; die IBAN allein oder
    /// "keine Bankverbindung" allein sind keiner. Der Kontoinhaber `E0101601` steht nur bei einer IBAN ohne Widerspruch.
    #[test]
    fn iban_und_keine_bankverbindung_zugleich_sind_ein_widerspruch_der_sperrt() {
        const WIDERSPRUCH: &str = "IBAN UND 'keine Bankverbindung' gleichzeitig bestaetigt -- checkESt lehnt den Widerspruch ab (rc=610001002). Genau eines von beidem darf gelten.";
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        let lauf = |iban: Option<&str>, keine: Option<bool>| {
            let mut felder = Felder::new();
            if let Some(i) = iban {
                felder.insert("stammdaten_iban".to_owned(), bestaetigt(json!(i)));
            }
            if let Some(k) = keine {
                felder.insert("stammdaten_keine_bankverbindung".to_owned(), bestaetigt(json!(k)));
            }
            deklariere(&felder, &index, 2025, None).unwrap()
        };
        // Nur die IBAN (beide Laender): Kontoinhaber, kein Widerspruch.
        for iban in [IBAN_DE, IBAN_GB] {
            for keine in [None, Some(false)] {
                let d = lauf(Some(iban), keine);
                assert_eq!(d.deklaration.get("E0101601"), Some(&json!(true)), "{iban} {keine:?}");
                assert!(d.unvollstaendig().is_empty(), "{iban} {keine:?}: {:?}", d.unvollstaendig());
            }
        }
        // Nur "keine Bankverbindung": das Kz steht, kein Kontoinhaber, kein Widerspruch.
        let d = lauf(None, Some(true));
        assert_eq!(d.deklaration.get("E0102002"), Some(&json!(true)));
        assert!(!d.deklaration.contains_key("E0101601"));
        assert!(d.unvollstaendig().is_empty(), "{:?}", d.unvollstaendig());
        // Weder noch: nichts.
        let d = lauf(None, None);
        assert!(!d.deklaration.contains_key("E0101601"));
        assert!(d.unvollstaendig().is_empty(), "{:?}", d.unvollstaendig());
        // Beides: der Widerspruch sperrt unter `stammdaten_iban`, und der Kontoinhaber bleibt weg.
        for iban in [IBAN_DE, IBAN_GB] {
            let d = lauf(Some(iban), Some(true));
            assert_eq!(d.unvollstaendig(), [super::Eintrag::neu("stammdaten_iban", WIDERSPRUCH)], "{iban}");
            assert!(!d.deklaration.contains_key("E0101601"), "{iban}: {:?}", d.deklaration);
        }
    }

    /// Mutant 4 (`Bau::feld`, Zeile 1578: `!` vor `wert.truthy()` geloescht). Ein Feld in `NEGATION` schreibt das GEGENTEIL
    /// seiner Antwort: der Vordruck fragt nach der schaedlichen Haushaltsgemeinschaft (`E0503701`), das Feld
    /// `fam_alleinstehend` nach dem Alleinsein. Ohne die Umkehr stuende der Haken verkehrt herum in der Erklaerung.
    #[test]
    fn ein_negations_feld_schreibt_das_gegenteil_seiner_antwort() {
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        for (antwort, erwartet) in [(true, false), (false, true)] {
            let felder = felder_aus([("fam_alleinstehend", bestaetigt(json!(antwort)))]);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            assert_eq!(d.deklaration.get("E0503701"), Some(&json!(erwartet)), "fam_alleinstehend = {antwort}");
        }
    }

    /// Mutanten 5 und 6 (`Bau::feld`, Zeile 1593: `==` zu `!=` bei der Suche nach dem Ziel; `Bau::dokumentiert`, Zeile
    /// 2054: Rueckgabe durch eine leere Tabelle ersetzt). Die vier Vermietungs-Betraege ohne eigenes Kz stehen NICHT in der
    /// Erklaerung, sondern als dokumentierte Summe unter `E0703838`, mit den Quellfeldern in sortierter Reihenfolge.
    #[test]
    fn vermietungs_betraege_ohne_kz_stehen_als_dokumentierte_summe_nicht_in_der_erklaerung() {
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        let lauf = |betraege: &[(&str, i64)]| {
            let felder: Felder = betraege
                .iter()
                .map(|(f, cent)| ((*f).to_owned(), bestaetigt(json!(cent))))
                .collect();
            deklariere(&felder, &index, 2025, None).unwrap()
        };
        let aggregat = |summe: i64, quellen: &[&str]| {
            BTreeMap::from([(
                "E0703838".to_owned(),
                super::Aggregat {
                    summe,
                    quell_felder: quellen.iter().map(|q| (*q).to_owned()).collect(),
                },
            )])
        };
        // Alle vier, in unsortierter Reihenfolge der Eingabe: 3000 + 1200 + 500 + 400 EUR.
        let d = lauf(&[
            ("vv_sonstige_wk", 40_000),
            ("vv_schuldzinsen", 120_000),
            ("vv_gebaeude_afa", 300_000),
            ("vv_erhaltungsaufwand", 50_000),
        ]);
        assert_eq!(
            d.dokumentiert,
            aggregat(5_100, &["vv_erhaltungsaufwand", "vv_gebaeude_afa", "vv_schuldzinsen", "vv_sonstige_wk"])
        );
        assert!(!d.deklaration.contains_key("E0703838"), "dokumentiert, nicht deklariert: {:?}", d.deklaration);
        // Nur zwei: die Summe zaehlt genau die vorhandenen.
        let d = lauf(&[("vv_schuldzinsen", 120_000), ("vv_erhaltungsaufwand", 50_000)]);
        assert_eq!(d.dokumentiert, aggregat(1_700, &["vv_erhaltungsaufwand", "vv_schuldzinsen"]));
        // Keiner: keine Summe, auch keine 0.
        assert!(lauf(&[]).dokumentiert.is_empty());
    }

    /// Mutant 9 (`Bau::vorsorge_partner_feld`, Zeile 927: Rueckgabe immer `false`). Die fuenf Vorsorge-Felder des Ehegatten
    /// uebernimmt `vorsorge_partner_feld` selbst: ohne bestaetigte Zusammenveranlagung steht der Grund "zaehlt nur bei
    /// bestaetigter Zusammenveranlagung" in `nicht_deklariert`; mit ihr steht gar kein Eintrag da, denn der Betrag geht in die
    /// gemeinsame Summe. Mit dem Mutanten landeten die Felder in der letzten Verzweigung von `feld` und trugen dort den Grund
    /// "kein eigenes Kz" aus der Bindung, auch bei Zusammenveranlagung.
    #[test]
    fn die_vorsorge_felder_des_ehegatten_nennen_ihren_grund_nur_ohne_bestaetigte_zusammenveranlagung() {
        const GRUND: &str = "Der Betrag des Ehegatten zaehlt nur bei bestaetigter Zusammenveranlagung; sonst steht er nicht in der Erklaerung";
        const FELDER: [&str; 5] = [
            "vorsorge_arbeitslosenversicherung_partner",
            "vorsorge_erwerbsunfaehigkeit_partner",
            "vorsorge_unfall_haftpflicht_partner",
            "vorsorge_rv_alt_mit_ueberschuss_partner",
            "vorsorge_rv_alt_ohne_ueberschuss_partner",
        ];
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        let lauf = |veranlagung: Option<(&str, Zustand)>| {
            let mut felder: Felder = FELDER
                .iter()
                .map(|f| ((*f).to_owned(), bestaetigt(json!(30_001))))
                .collect();
            if let Some((wert, zustand)) = veranlagung {
                felder.insert("veranlagung".to_owned(), eingabe(json!(wert), zustand));
            }
            deklariere(&felder, &index, 2025, None).unwrap()
        };
        let d = lauf(Some(("zusammen", Zustand::Bestaetigt)));
        for f in FELDER {
            assert!(
                gruende(&d.nicht_deklariert, f).is_empty(),
                "zusammen, {f}: {:?}",
                gruende(&d.nicht_deklariert, f)
            );
        }
        // Einzel, nur vorlaeufig zusammen und gar keine Angabe: der Ehegatte zaehlt nicht.
        for veranlagung in [
            Some(("einzel", Zustand::Bestaetigt)),
            Some(("zusammen", Zustand::Vorlaeufig)),
            None,
        ] {
            let d = lauf(veranlagung);
            for f in FELDER {
                assert_eq!(gruende(&d.nicht_deklariert, f), [GRUND], "{veranlagung:?}, {f}");
            }
        }
    }

    /// Mutant 10 (`deklariere`, Zeile 1777: `||` zu `&&`). Wer das Snapshot-OBJEKT statt der Felder-Ebene uebergibt, wird
    /// abgewiesen, sobald EINER der Schluessel `felder` oder `snapshot_id` darin steht (Python: `est_mapping.py`, dieselbe
    /// Pruefung). Ohne einen der beiden ist die Eingabe in Ordnung.
    #[test]
    fn ein_snapshot_objekt_wird_schon_bei_einem_der_beiden_schluessel_abgewiesen() {
        use super::DeklarationsFehler;

        let index = HashMap::new();
        for schluessel in [&["felder"][..], &["snapshot_id"], &["felder", "snapshot_id"]] {
            let felder: Felder = schluessel
                .iter()
                .map(|s| ((*s).to_owned(), bestaetigt(json!({}))))
                .collect();
            let e = deklariere(&felder, &index, 2025, None).unwrap_err();
            assert_eq!(e, DeklarationsFehler::SnapshotObjekt, "{schluessel:?}");
            assert_eq!(e.python_klasse(), "ValueError", "{schluessel:?}");
        }
        assert!(deklariere(&Felder::new(), &index, 2025, None).is_ok());
    }

    /// Mutant 11 (`euro_text`, Zeile 458: `<` zu `<=`). Ein Betrag von 0 bekommt kein Minuszeichen; Cent unter einem Euro
    /// behalten die zwei Nachkommastellen, negative Betraege das Minus vor dem Betrag.
    #[test]
    fn euro_text_setzt_das_minus_nur_vor_negative_betraege() {
        for (cent, text) in [
            (0, "0,00"),
            (5, "0,05"),
            (-5, "-0,05"),
            (500, "5,00"),
            (-500, "-5,00"),
            (123_450, "1234,50"),
            (-123_450, "-1234,50"),
        ] {
            assert_eq!(super::euro_text(cent), text, "{cent} ct");
        }
    }

    /// Mutanten 12 und 13 (`zustand_text`, Zeile 752: Rueckgabe `""` und `"xyzzy"`). Beide Werte des Zustands haben ihr Wort,
    /// und das Wort steht im Grund der Sperre eines vorlaeufigen Felds.
    #[test]
    fn der_zustand_steht_mit_seinem_wort_im_grund_der_sperre_eines_vorlaeufigen_felds() {
        assert_eq!(super::zustand_text(Zustand::Vorlaeufig), "vorlaeufig");
        assert_eq!(super::zustand_text(Zustand::Bestaetigt), "bestaetigt");

        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        let felder = felder_aus([("stammdaten_iban", eingabe(json!(IBAN_DE), Zustand::Vorlaeufig))]);
        let d = deklariere(&felder, &index, 2025, None).unwrap();
        assert_eq!(
            d.unvollstaendig(),
            [super::Eintrag::neu(
                "stammdaten_iban",
                "Wert vorlaeufig — Pflicht-Bestätigung (Zwei-Signal) fehlt"
            )]
        );
        assert!(!d.deklaration.contains_key("E0102102"), "{:?}", d.deklaration);
    }

    /// Mutanten 14 und 15 (`Bau::verzweigung`, Zeile 1501: der Guard `partner` wird `true` und `false`). Hat die bestaetigte
    /// Art keinen Kz-Zweig, sagen die Gruende es mit dem Wortlaut der Person: bei der Hauptperson "Art ... ohne Kz-Zweig", beim
    /// Partner "Partner-Renten-Art ... ohne Kz-Zweig" und die laengere Rueckmeldung an "euch" statt an "dich".
    #[test]
    fn eine_art_ohne_kz_zweig_nennt_ihren_grund_im_wortlaut_der_person() {
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        let felder = felder_aus([
            ("rentner_renten_art", bestaetigt(json!("unbekannt"))),
            ("rentner_jahresrente", bestaetigt(json!(1_200_000))),
            ("rentner_renten_art_partner", bestaetigt(json!("unbekannt"))),
            ("rentner_jahresrente_partner", bestaetigt(json!(1_200_000))),
        ]);
        let d = deklariere(&felder, &index, 2025, None).unwrap();
        assert_eq!(
            gruende(&d.nicht_deklariert, "rentner_jahresrente"),
            ["Art 'unbekannt' (rentner_renten_art) ohne Kz-Zweig"]
        );
        assert_eq!(
            gruende(d.unvollstaendig(), "rentner_jahresrente"),
            ["Für 'unbekannt' (rentner_renten_art) ist diese Einkunftsart bei uns noch nicht abgebbar — keine Eingabe von dir fehlt oder widerspricht sich."]
        );
        assert_eq!(
            gruende(&d.nicht_deklariert, "rentner_jahresrente_partner"),
            ["Partner-Renten-Art 'unbekannt' ohne Kz-Zweig"]
        );
        assert_eq!(
            gruende(d.unvollstaendig(), "rentner_jahresrente_partner"),
            ["Eure Angabe zu 'unbekannt' (rentner_renten_art_partner) ist vollständig, nichts von euch fehlt oder widerspricht sich. Diese Einkunftsart des Partners ist bei uns noch nicht abgebbar — das liegt nicht an eurer Eingabe, ihr müsst hier nichts nachtragen."]
        );
    }

    /// Mutant 16 (`Bau::feld`, Zeile 1585: `!` im Filter des Grunds der Multiplikation geloescht). Das Zaehlfeld der Kinder
    /// legt die Anlagen Kind an und steht mit dem Grund der Bindung in `nicht_deklariert`; fehlt der Grund oder ist er leer,
    /// gilt der feste Text "Multiplikation -> `kind_anlagen` (Kinderzahl implizit)".
    #[test]
    fn das_zaehlfeld_der_kinder_nennt_den_grund_der_bindung_oder_den_festen_text() {
        const VORGABE: &str = "Multiplikation -> kind_anlagen (Kinderzahl implizit)";
        let bindungen = bindungen_der_wurzel();
        let mut index = store::baue_nachschlag(&bindungen);
        let echt = index["fam_anzahl_kinder"];
        let grund_der_bindung = echt.elster_kz_grund.clone().unwrap();
        assert!(!grund_der_bindung.is_empty(), "die Bindung braucht fuer diesen Test einen Grund");
        let ohne_grund = bindung::Bindung { elster_kz_grund: None, ..echt.clone() };
        let leerer_grund = bindung::Bindung { elster_kz_grund: Some(String::new()), ..echt.clone() };
        let felder = felder_aus([("fam_anzahl_kinder", bestaetigt(json!(2)))]);

        let d = deklariere(&felder, &index, 2025, None).unwrap();
        assert_eq!(d.kind_anlagen.len(), 2);
        assert_eq!(gruende(&d.nicht_deklariert, "fam_anzahl_kinder"), [grund_der_bindung.as_str()]);

        for b in [&ohne_grund, &leerer_grund] {
            index.insert("fam_anzahl_kinder".to_owned(), b);
            let d = deklariere(&felder, &index, 2025, None).unwrap();
            assert_eq!(
                gruende(&d.nicht_deklariert, "fam_anzahl_kinder"),
                [VORGABE],
                "Grund {:?}",
                b.elster_kz_grund
            );
        }
    }

    /// Mutanten 17 und 18 (`Bau::kap_nulldeklaration`, Zeilen 1969 und 1977: `!` in `is_some_and(|v| !v.is_null())`
    /// geloescht). Sind alle zehn Kapital-Felder beider Personen bestaetigt 0, laesst die Erklaerung die Kz weg
    /// (Option A, Julius-Entscheidung 2026-08-10), und jedes Feld MIT Kz nennt den Grund dafuer in `nicht_deklariert`.
    /// Die zwei Felder ohne Kz (`kap_gewinn_sonstige` und sein Partnerfeld) tragen ihren eigenen Grund und keinen dieses Texts.
    /// Ist EIN Feld ungleich 0, bleibt alles stehen und kein Feld nennt den Grund.
    #[test]
    fn nullen_im_kapital_fallen_weg_und_jedes_feld_mit_kz_nennt_den_grund() {
        const OPTION_A: &str = "Option A (Julius-Entscheidung 2026-08-10): alle KAP-Felder beider Personen bestaetigt 0 -- als Kz deklariert wuerde ELSTER 'Kapitalertraege erklaert' lesen und einen Angabegrund verlangen, den wir nicht liefern.";
        const MIT_KZ_A: [(&str, &str); 4] = [
            ("kap_kapitalertraege", "E1900701"),
            ("kap_gewinn_aktien", "E1900901"),
            ("kap_verlust_aktien", "E1901301"),
            ("kap_verlust_sonstige", "E1901201"),
        ];
        const MIT_KZ_B: [&str; 4] = [
            "kap_kapitalertraege_partner",
            "kap_gewinn_aktien_partner",
            "kap_verlust_aktien_partner",
            "kap_verlust_sonstige_partner",
        ];
        const OHNE_KZ: [&str; 2] = ["kap_gewinn_sonstige", "kap_gewinn_sonstige_partner"];
        let bindungen = bindungen_der_wurzel();
        let index = store::baue_nachschlag(&bindungen);
        let lauf = |ausnahme: Option<(&str, i64)>| {
            let mut felder = Felder::new();
            for f in MIT_KZ_A.map(|(f, _)| f).into_iter().chain(MIT_KZ_B).chain(OHNE_KZ) {
                felder.insert(f.to_owned(), bestaetigt(json!(0)));
            }
            if let Some((f, cent)) = ausnahme {
                felder.insert(f.to_owned(), bestaetigt(json!(cent)));
            }
            deklariere(&felder, &index, 2025, None).unwrap()
        };

        let d = lauf(None);
        for (f, kz) in MIT_KZ_A {
            assert!(!d.deklaration.contains_key(kz), "{f}: {kz} in {:?}", d.deklaration);
            assert_eq!(gruende(&d.nicht_deklariert, f), [OPTION_A], "{f}");
        }
        assert!(d.person_b.is_empty(), "{:?}", d.person_b);
        for f in MIT_KZ_B {
            assert_eq!(gruende(&d.nicht_deklariert, f), [OPTION_A], "{f}");
        }
        for f in OHNE_KZ {
            assert!(!gruende(&d.nicht_deklariert, f).contains(&OPTION_A), "{f}");
        }

        // Ein Betrag bei Person A ODER bei Person B haelt die Nullen beider Personen in der Erklaerung.
        for f in ["kap_kapitalertraege", "kap_verlust_aktien_partner"] {
            let d = lauf(Some((f, 500)));
            assert_eq!(d.deklaration.get("E1900901"), Some(&json!(0)), "{f}: Null von Person A bleibt");
            assert_eq!(d.person_b.get("E1900901"), Some(&json!(0)), "{f}: Null von Person B bleibt");
            for g in MIT_KZ_A.map(|(g, _)| g).into_iter().chain(MIT_KZ_B) {
                assert!(!gruende(&d.nicht_deklariert, g).contains(&OPTION_A), "{f}: {g}");
            }
        }
    }
}
