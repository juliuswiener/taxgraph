//! GENERIERT von `tools/parity/gen_enum_labels.py` aus `produkt/haut/api_constants.py`.
//! NICHT von Hand pflegen. Neu erzeugen:
//!
//! ```text
//! python3 tools/parity/gen_enum_labels.py
//! ```
//!
//! `ENUM_LABELS` (`api_constants.py:951`): je Enum-Feld die Anzeigetexte seiner Werte, in
//! Pythons Einfuegereihenfolge, samt der abgeleiteten Schluessel am Ende des Moduls.
//! `#[rustfmt::skip]` haelt das Generator-Layout, damit ein erneuter Lauf byte-stabil bleibt.
//!
//! Drift faengt `enum_labels_gleich` (`rust/parity/tests/api_http_paritaet.rs`).

/// `feld_id -> [(enum-wert, anzeigetext)]`.
#[rustfmt::skip]
pub const ENUM_LABELS: [(&str, &[(&str, &str)]); 26] = [
    ("dba_einkunftsart", &[
        ("unbewegliches_vermoegen", "Unbewegliches Vermögen (z. B. Immobilie)"),
        ("unternehmensgewinne", "Unternehmensgewinne"),
        ("dividenden", "Dividenden"),
        ("zinsen", "Zinsen"),
        ("lizenzgebuehren", "Lizenzgebühren"),
        ("veraeusserungsgewinne", "Veräußerungsgewinne"),
        ("unselbstaendige_arbeit", "Arbeitslohn aus dem Ausland"),
        ("aufsichtsratsverguetungen", "Aufsichtsratsvergütung"),
        ("kuenstler_sportler", "Auftritt als Künstler oder Sportler"),
        ("ruhegehaelter", "Ruhegehalt oder Pension"),
    ]),
    ("dba_methode", &[
        ("kein_dba", "Kein Doppelbesteuerungsabkommen mit diesem Staat"),
        ("dba_anrechnung", "Anrechnung — die ausländische Steuer wird angerechnet"),
        ("dba_freistellung", "Freistellung — die Einkünfte bleiben hier steuerfrei"),
    ]),
    ("dba_staat", &[
        ("Deutschland", "Deutschland"),
        ("Frankreich", "Frankreich"),
        ("Italien", "Italien"),
        ("Oesterreich", "Österreich"),
        ("Schweiz", "Schweiz"),
        ("Niederlande", "Niederlande"),
        ("Polen", "Polen"),
        ("Tschechien", "Tschechien"),
        ("Dänemark", "Dänemark"),
        ("Luxemburg", "Luxemburg"),
        ("Türkei", "Türkei"),
        ("Grossbritannien", "Großbritannien"),
        ("Spanien", "Spanien"),
        ("USA", "USA"),
        ("Kanada", "Kanada"),
        ("sonstiger_staat", "Anderer Staat"),
    ]),
    ("gewinn_betriebsart", &[
        ("gewerbe", "Gewerbebetrieb"),
        ("selbstaendig", "Selbständige oder freiberufliche Arbeit"),
        ("land_forst", "Land- und Forstwirtschaft"),
    ]),
    ("kind_kindschaftsverhaeltnis_a", &[
        ("1", "Leibliches Kind oder Adoptivkind"),
        ("2", "Pflegekind"),
        ("3", "Enkelkind oder Stiefkind"),
    ]),
    ("kind_anderer_elternteil_kindschaftsverhaeltnis", &[
        ("1", "Leibliches Kind oder Adoptivkind"),
        ("2", "Pflegekind"),
    ]),
    ("ep_ziel_des_weges", &[
        ("1", "Fester Arbeitsplatz (erste Tätigkeitsstätte)"),
        ("2", "Sammelpunkt oder weiträumiges Tätigkeitsgebiet"),
    ]),
    ("p35c_massnahme_art", &[
        ("waende", "Wärmedämmung von Wänden"),
        ("dach", "Wärmedämmung von Dachflächen"),
        ("geschossdecken", "Wärmedämmung von Geschossdecken"),
        ("fenster_tueren", "Neue Fenster oder Außentüren"),
        ("sommerlicher_waermeschutz", "Sommerlicher Wärmeschutz (z. B. Rollläden, Markisen)"),
        ("lueftung", "Neue oder erneuerte Lüftungsanlage"),
        ("heizung", "Neue Heizungsanlage"),
        ("digital", "Digitale Systeme zur Verbrauchsoptimierung"),
        ("heizung_optimierung", "Optimierung einer bestehenden Heizung (älter als 2 Jahre)"),
    ]),
    ("rentner_pflege_durch", &[
        ("1", "Ich"),
        ("2", "Mein Ehe- oder Lebenspartner"),
        ("3", "Wir beide gemeinsam"),
    ]),
    ("kist_bundesland", &[
        ("baden_wuerttemberg", "Baden-Württemberg"),
        ("bayern", "Bayern"),
        ("berlin", "Berlin"),
        ("brandenburg", "Brandenburg"),
        ("bremen", "Bremen"),
        ("hamburg", "Hamburg"),
        ("hessen", "Hessen"),
        ("mecklenburg_vorpommern", "Mecklenburg-Vorpommern"),
        ("niedersachsen", "Niedersachsen"),
        ("nordrhein_westfalen", "Nordrhein-Westfalen"),
        ("rheinland_pfalz", "Rheinland-Pfalz"),
        ("saarland", "Saarland"),
        ("sachsen", "Sachsen"),
        ("sachsen_anhalt", "Sachsen-Anhalt"),
        ("schleswig_holstein", "Schleswig-Holstein"),
        ("thueringen", "Thüringen"),
    ]),
    ("kist_konfession", &[
        ("keine", "Keine Konfession"),
        ("evangelisch", "Evangelisch"),
        ("roemisch-katholisch", "Römisch-katholisch"),
        ("andere", "Andere Religionsgemeinschaft"),
    ]),
    ("p23_veraeusserungs_typ", &[
        ("grundstueck", "Grundstück oder Immobilie"),
        ("anderes_wg", "Anderes Wirtschaftsgut (z. B. Krypto, Kunst, Edelmetalle)"),
    ]),
    ("rentner_renten_art", &[
        ("gesetzliche_rente", "Gesetzliche Rente"),
        ("berufsstaendische_versorgung", "Berufsständische Versorgung (z. B. Ärzte, Anwälte)"),
        ("private_basisrente", "Private Basisrente (Rürup)"),
        ("private_leibrente", "Private Leibrente"),
        ("sonstige_leibrente", "Sonstige Leibrente"),
    ]),
    ("steuerklasse", &[
        ("1", "I — ledig, verwitwet oder geschieden"),
        ("2", "II — alleinerziehend"),
        ("3", "III — verheiratet, Partner in Klasse V"),
        ("4", "IV — verheiratet, beide in Klasse IV"),
        ("5", "V — verheiratet, Partner in Klasse III"),
        ("6", "VI — weiteres Dienstverhältnis"),
    ]),
    ("veranlagung", &[
        ("einzel", "Einzelveranlagung — jeder für sich"),
        ("zusammen", "Zusammenveranlagung mit Ehe- oder Lebenspartner"),
    ]),
    ("versicherungsart", &[
        ("gesetzlich_an", "Gesetzlich als Arbeitnehmer"),
        ("gesetzlich_freiwillig", "Gesetzlich freiwillig versichert (Selbstzahler)"),
        ("privat", "Privat versichert"),
    ]),
    ("versorgung_art", &[
        ("beamtenrechtlich", "Beamtenrechtliches Ruhegehalt"),
        ("hinterbliebene", "Witwen- oder Waisengeld"),
        ("erwerbsminderung", "Rente wegen Erwerbsminderung"),
        ("altersgrenze_sonstige", "Betriebsrente oder Direktversicherung"),
    ]),
    ("gewinn_betriebsart_partner", &[
        ("gewerbe", "Gewerbebetrieb"),
        ("selbstaendig", "Selbständige oder freiberufliche Arbeit"),
        ("land_forst", "Land- und Forstwirtschaft"),
    ]),
    ("kist_konfession_partner", &[
        ("keine", "Keine Konfession"),
        ("evangelisch", "Evangelisch"),
        ("roemisch-katholisch", "Römisch-katholisch"),
        ("andere", "Andere Religionsgemeinschaft"),
    ]),
    ("rentner_renten_art_partner", &[
        ("gesetzliche_rente", "Gesetzliche Rente"),
        ("berufsstaendische_versorgung", "Berufsständische Versorgung (z. B. Ärzte, Anwälte)"),
        ("private_basisrente", "Private Basisrente (Rürup)"),
        ("private_leibrente", "Private Leibrente"),
        ("sonstige_leibrente", "Sonstige Leibrente"),
    ]),
    ("steuerklasse_partner", &[
        ("1", "I — ledig, verwitwet oder geschieden"),
        ("2", "II — alleinerziehend"),
        ("3", "III — verheiratet, Partner in Klasse V"),
        ("4", "IV — verheiratet, beide in Klasse IV"),
        ("5", "V — verheiratet, Partner in Klasse III"),
        ("6", "VI — weiteres Dienstverhältnis"),
    ]),
    ("versicherungsart_partner", &[
        ("gesetzlich_an", "Gesetzlich als Arbeitnehmer"),
        ("gesetzlich_freiwillig", "Gesetzlich freiwillig versichert (Selbstzahler)"),
        ("privat", "Privat versichert"),
    ]),
    ("kind_kindschaftsverhaeltnis_a_partner", &[
        ("1", "Leibliches Kind oder Adoptivkind"),
        ("2", "Pflegekind"),
        ("3", "Enkelkind oder Stiefkind"),
    ]),
    ("kind_kindschaftsverhaeltnis_b", &[
        ("1", "Leibliches Kind oder Adoptivkind"),
        ("2", "Pflegekind"),
        ("3", "Enkelkind oder Stiefkind"),
    ]),
    ("rentner_veraeusserungs_betriebsart", &[
        ("gewerbe", "Gewerbebetrieb"),
        ("selbstaendig", "Selbständige oder freiberufliche Arbeit"),
        ("land_forst", "Land- und Forstwirtschaft"),
    ]),
    ("rentner_veraeusserungs_betriebsart_partner", &[
        ("gewerbe", "Gewerbebetrieb"),
        ("selbstaendig", "Selbständige oder freiberufliche Arbeit"),
        ("land_forst", "Land- und Forstwirtschaft"),
    ]),
];
