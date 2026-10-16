//! Die Naht VaSt-Beleg -> Bindung -> Store (Python: `tests/test_vast_mapping.py`, die Naht-Tests `test_alle_ziel_felder_existieren_in_der_bindung`,
//! `test_alle_ziel_felder_sind_cent`, `test_summen_ziele_existieren_und_sind_cent`, `test_mapping_ausgabe_passt_in_den_writer`,
//! `test_gemappte_vast_elemente_existieren_im_schema`, `test_summen_quellen_existieren_im_schema`).
//!
//! **Worum es geht.** Das Mapping `eingang::vast::{LSTB, LSTB_SUMMEN, LERSL_BETRAG_FELD}` schickt Beträge der Lohnsteuerbescheinigung an
//! Felder der Bindung. Ein Ziel, das es dort nicht gibt, schreibt ins Leere: der Store lehnt den Satz ab, der Nutzer sieht nichts. Ein
//! Ziel, das kein Cent-Feld ist, mischt Euro und Cent. Beide Seiten stimmen fuer sich, das Format dazwischen prueft sonst niemand.
//! `rust/eingang/tests/orakel_werte.rs` gleicht die Werte gegen eine eingefrorene Python-Bindung ab (`fixtures/python_bindung_2dd056a6`),
//! nicht gegen die Bindung des Dienstes: ein Ziel, das in der heutigen Bindung fehlt, faellt dort nicht auf.
//!
//! Zwei Teile:
//!
//! - **Ohne Schema** (laeuft in der CI): jedes Ziel steht in der Bindung und ist `cent`; jedes Element des Mappings durchlaeuft
//!   `aus_lstb` und `edaten::uebernehme` gegen die echte Bindung und kommt als bestaetigter Cent-Wert im Store an; die beiden Lueckenlisten
//!   ([`NICHT_GEMAPPT`], [`NICHT_UNTERSUCHT`]) sind in sich stimmig (kein Eintrag doppelt, keiner zugleich im Mapping, jeder Grund mehr als ein Wort).
//! - **Mit Schema** (nur lokal): jeder VaSt-Elementname des Mappings steht als `name="..."` in `VaSt_LStB-202501.xsd`, und JEDES Beleg-Blatt
//!   des Schemas ist gemappt oder in einer der beiden Listen benannt. Ohne Schema rot, ausser `TAXGRAPH_OHNE_XSD=1`
//!   (`elster::testhilfe::schemas_da`); die CI setzt die Variable und prueft beides damit NICHT.
//!
//! Der Wächter ersetzt `test_luecken_sind_benannt` und geht darüber hinaus (Python prüfte nur, dass die Liste nicht leer war und jeder Grund
//! lang genug ist; ob sie zum Schema passt, prüfte nichts). Er schliesst den Verlust (ii) aus `REWRITE_PLAN` Nr. 69.
//!
//! Grenzen: (1) Nur die Lohnsteuerbescheinigung 2025 (`VaSt_LStB-202501.xsd`). Die anderen Belegarten im Verzeichnis (KRV, RBM, `LErsL`,
//! FSA, RIE, RUE, VWL, ...) deckt der Test nicht ab; Python nannte KRV und RBM als offen, Rust nennt sie nicht. (2) Er prüft,
//! dass ein Feld benannt ist, nicht ob der Grund stimmt; die Gründe von [`NICHT_GEMAPPT`] sind aus Python übernommen. (3) Die Einträge von
//! [`NICHT_UNTERSUCHT`] sind offene Fragen, keine Entscheidungen. (4) Ein neues Element kommt nur mit einem neuen Schema, also mit einem
//! neuen Jahr: der Test liest nur 2025.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Feldtyp, PyWert, Zustand};
use eingang::edaten::{uebernehme, Satz};
use eingang::vast::{aus_lstb, LERSL_BETRAG_FELD, LSTB, LSTB_SUMMEN};
use elster::testhilfe::schemas_da;
use roxmltree::Node;
use store::{BindungNachschlag, Store};

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

/// Alle Zielfelder des Mappings: die 1:1-Felder, die Summenfelder und das Feld der Lohnersatzleistungen.
fn ziele() -> BTreeSet<&'static str> {
    LSTB.iter()
        .map(|(_, feld, _)| *feld)
        .chain(LSTB_SUMMEN.iter().map(|(feld, _)| *feld))
        .chain([LERSL_BETRAG_FELD])
        .collect()
}

/// Jeder Elementname, den das Mapping liest.
fn vast_elemente() -> BTreeSet<&'static str> {
    LSTB.iter()
        .map(|(vast, _, _)| *vast)
        .chain(LSTB_SUMMEN.iter().flat_map(|(_, quellen)| quellen.iter().map(|(n, _)| *n)))
        .collect()
}

#[test]
fn jedes_ziel_steht_in_der_bindung_und_ist_ein_cent_feld() {
    let z = ziele();
    // KONTROLLE: die Schleife unten sieht Ziele (gemessen 2026-10-13: 9 verschiedene Felder aus 8 + 1 + 1 Eintraegen).
    assert!(z.len() >= 9, "nur {} Ziele gesehen: {z:?}", z.len());
    let fehlend: Vec<&str> = z.iter().copied().filter(|f| !index().contains_key(*f)).collect();
    assert!(fehlend.is_empty(), "Mapping zeigt auf Felder, die es in der Bindung nicht gibt: {fehlend:?}");
    let falsch: Vec<(&str, Feldtyp)> =
        z.iter().copied().filter_map(|f| Some((f, index()[f].typ)).filter(|(_, t)| *t != Feldtyp::Cent)).collect();
    assert!(falsch.is_empty(), "Ziele ohne typ=cent (VaSt liefert Euro, der Store rechnet in Cent): {falsch:?}");
}

/// Jedes Element des Mappings mit einem Betrag belegt, durch `aus_lstb` und `uebernehme` gegen die echte Bindung: jedes Ziel kommt
/// als bestaetigter Wert an (§ 150 Abs. 7 S. 2 AO), und der Wert ist der Betrag in Cent.
#[test]
fn jedes_element_des_mappings_kommt_als_bestaetigter_cent_wert_im_store_an() {
    let werte: BTreeMap<String, String> = vast_elemente().into_iter().map(|e| (e.to_owned(), "1234.56".to_owned())).collect();
    let saetze: Vec<Satz> = aus_lstb(&werte)
        .unwrap()
        .into_iter()
        .map(|s| Satz { feld_id: s.feld_id, wert: s.wert.into(), kategorie: s.kategorie.into() })
        .collect();
    // `LERSL_BETRAG_FELD` ist zugleich das Ziel von `LeistungenProgVorbeh`, also liefert `aus_lstb` einen Satz je Ziel.
    let erwartet = ziele();
    let gesehen: BTreeSet<&str> = saetze.iter().map(|s| s.feld_id.as_str()).collect();
    assert_eq!(gesehen, erwartet, "aus_lstb liefert nicht fuer jedes Ziel einen Satz");

    let mut store = Store::leer(2025, None);
    let n = uebernehme(&mut store, &saetze, Some("2026-01-01T00:00:00Z"), Some(BindungNachschlag::neu(index()))).unwrap();
    assert_eq!(n, saetze.len(), "die Bindung hat Saetze abgewiesen oder uebersprungen");
    let (felder, _) = store.materialisiere(None).unwrap();
    for feld in &erwartet {
        let f = &felder[*feld];
        assert_eq!(f.zustand, Zustand::Bestaetigt, "{feld}");
        assert_eq!(f.wert, PyWert::Ganz(123_456), "{feld}: 1234,56 EUR sind 123456 Cent");
    }
}

/// Beleg-Elemente der Lohnsteuerbescheinigung, die der Import absichtlich NICHT uebernimmt, je mit Grund (Python: `NICHT_GEMAPPT`).
/// Schluessel ist der Pfad im Schema ab der Wurzel `VaSt_LStB`. Die Gruende stammen woertlich aus Python (`produkt/eingang/vast_mapping.py`,
/// Stand 2026-10-16); die Bezeichner darin (`catala_solz`, `basis_kv_pv`, `DBA_METHOD_MAP_ART`) sind Python-Wortlaut und hier einzeln
/// nicht nachgeprueft. Eine Ausnahme: der Grund der Versorgungsbezuege ist ueberholt und steht neu (siehe [`VBEZ_GRUND`]).
const NICHT_GEMAPPT: [(&str, &str); 12] = [
    (
        "Besteuerungsgrund/Soli",
        "Solidaritätszuschlag wird vom Ring aus der festgesetzten ESt berechnet (catala_solz), nicht aus der Bescheinigung übernommen.",
    ),
    (
        "Besteuerungsgrund/EhegKiSteuer",
        "Kirchensteuer des Ehegatten gehört zur Person-B-Instanz; die Partner-Felder werden erst mit dem Zusammenveranlagungs-Abruf \
         gefüllt (eigener Vorgang, eigene Berechtigung).",
    ),
    (
        "Besteuerungsmerkmal/ELStAM/Steuerklasse/Klasse",
        "Die Lohnsteuerklasse steuert den Lohnsteuerabzug, nicht die Veranlagung. Der Ring rechnet aus `veranlagung` (§ 26).",
    ),
    (
        "Besteuerungsgrund/BeitrPrKrankVers",
        "Private Krankenversicherung: § 10 Abs. 1 Nr. 3 S. 3 zählt nur die Beitragsanteile, die den Leistungen des SGB V entsprechen — \
         abzüglich des Krankengeld-Anteils. Der Beleg liefert den Gesamtbeitrag ohne diese Aufteilung; ihn voll als Basisabsicherung zu \
         übernehmen wäre zu hoch. Privat Versicherte tragen den Wert weiter selbst ein.",
    ),
    (
        "Besteuerungsgrund/StFreiGeKrankVers",
        "Steuerfreie Arbeitgeberzuschüsse mindern die abziehbaren Beiträge (§ 10 Abs. 2 S. 1 Nr. 1), sind aber kein eigener Abzugsposten. \
         Ob unser basis_kv_pv brutto oder netto gemeint ist, klärt die Bindung nicht eindeutig — offen, statt auf Verdacht zu saldieren.",
    ),
    (
        "Besteuerungsgrund/StFreiArbLohnDBA",
        "Steuerfreier Arbeitslohn nach DBA berührt § 32b und die DBA-Methodenwahl; welches unserer Felder das trifft, hängt am Staat und \
         ist offen (siehe DBA_METHOD_MAP_ART).",
    ),
    ("Besteuerungsgrund/VBez/Wert", VBEZ_GRUND),
    ("Besteuerungsgrund/VBez/Beginn", VBEZ_GRUND),
    ("Besteuerungsgrund/VBez/Ende", VBEZ_GRUND),
    ("Besteuerungsgrund/VBez/Jahr", VBEZ_GRUND),
    ("Besteuerungsgrund/VBez/BmgFreibetrag", VBEZ_GRUND),
    ("Besteuerungsgrund/VBez/Einmversbezug", VBEZ_GRUND),
];

/// Der Python-Grund der Versorgungsbezuege war „§ 19 Abs. 2 Versorgungsfreibetrag ist im Ring nicht modelliert". Das stimmt in Rust nicht
/// mehr: der Bescheid rechnet den Versorgungsfreibetrag (Abweichungen Nr. 50, 65, 66, `rust/fixtures/README.md`).
const VBEZ_GRUND: &str = "Der Beleg liefert die Versorgungsbezüge (§ 19 Abs. 2 EStG), `aus_lstb` liest sie nicht. Der Python-Grund \
    „Versorgungsfreibetrag im Ring nicht modelliert“ ist überholt: Rust rechnet ihn. Ob der Beleg in die Felder `versorgung_*` gemappt \
    wird, ist offen und liegt bei Julius.";

/// Beleg-Elemente ohne Eintrag in `LSTB` und OHNE Grund: bis 2026-10-16 nie angesehen. Der Text ist die Doku des Schemas, KEINE
/// Begruendung. Jede Zeile hier ist eine offene Frage „soll das gemappt werden?"; wer sie beantwortet, nimmt das Element in `LSTB` auf oder
/// verschiebt es mit Grund nach [`NICHT_GEMAPPT`]. Die Liste darf nur schrumpfen.
const NICHT_UNTERSUCHT: [(&str, &str); 40] = [
    ("ArbGName", "Name des Arbeitgebers"),
    ("Eingangsdatum", "Übermittlungszeitpunkt der Bescheinigung an die Finanzverwaltung"),
    ("Allgemein/IdNr", "Identifikationsnummer"),
    ("Allgemein/Name", "Name"),
    ("Allgemein/Vorname", "Vorname"),
    ("Allgemein/Namensvorsatz", "Namensvorsatz"),
    ("Allgemein/Namenszusatz", "Namenszusatz"),
    ("Allgemein/Titel", "Titel"),
    ("Dauer/Jahr", "Beschäftigungsjahr"),
    ("Dauer/Anfang", "vom"),
    ("Dauer/Ende", "bis"),
    ("Besteuerungsmerkmal/ELStAM/GueltigAb", "Besteuerungsmerkmale gültig ab"),
    ("Besteuerungsmerkmal/ELStAM/Steuerfreibetrag", "eingetragener Steuerfreibetrag"),
    ("Besteuerungsmerkmal/ELStAM/Hinzurechnungsbetrag", "eingetragener Hinzurechnungsbetrag"),
    ("Besteuerungsmerkmal/ELStAM/Steuerklasse/Faktor", "Faktor"),
    ("Besteuerungsmerkmal/ELStAM/Kirchensteuerabzug/Konfession", "Kirchensteuermerkmal (Konfession)"),
    ("Besteuerungsmerkmal/ELStAM/Kirchensteuerabzug/Ehegkonfession", "Kirchensteuermerkmal (Konfession) des Ehegatten"),
    ("Besteuerungsgrund/AnzahlU", "Zeiträume ohne Anspruch auf Arbeitslohn (Anzahl U)"),
    ("Besteuerungsgrund/Grossbuchstaben", "bescheinigter Großbuchstabe"),
    ("Besteuerungsgrund/ErmStBetrMKalJahr", "Arbeitslohn für mehrere Kalenderjahre und Entschädigungen, z. B. Abfindungen (im Bruttoarbeitslohn enthalten)"),
    ("Besteuerungsgrund/KurzArbGeld", "davon: (Saison-)Kurzarbeitergeld"),
    ("Besteuerungsgrund/StFreiArbLohnATE", "steuerfreier Arbeitslohn nach Auslandstätigkeitserlass"),
    ("Besteuerungsgrund/ArbgAnteilBerufsVers", "Arbeitgeberanteil an berufsständische Versorgungseinrichtungen"),
    ("Besteuerungsgrund/ArbnAnteilBerufsVers", "Arbeitnehmeranteil an berufsständische Versorgungseinrichtungen"),
    ("Besteuerungsgrund/StFreiPrKrankVers", "steuerfreie Arbeitgeberzuschüsse zur privaten Krankenversicherung"),
    ("Besteuerungsgrund/StFreiGePflegVers", "steuerfreie Arbeitgeberzuschüsse zur gesetzlichen Pflegeversicherung"),
    ("Besteuerungsgrund/ArbnAnteilWBUmlage", "Arbeitnehmerbeitrag zur Winterbeschäftigungs-Umlage"),
    ("Besteuerungsgrund/ArbgAnteilZusatzVers", "steuerpflichtiger Beitrag zur Zusatzversorgung"),
    ("Besteuerungsgrund/ArbnAnteilZusatzVers", "Arbeitnehmeranteil zur Zusatzversorgung"),
    ("Besteuerungsgrund/AnzahlArbTag", "Anzahl Arbeitstage"),
    ("Besteuerungsgrund/StFreiArbgLeistg", "steuerfreie Arbeitgeberleistungen für Fahrten zwischen Wohnung und Arbeitsstätte"),
    ("Besteuerungsgrund/PauschArbgLeistg", "pauschal besteuerte Arbeitgeberleistungen für Fahrten zwischen Wohnung und Arbeitsstätte"),
    ("Besteuerungsgrund/StFreiFahrtKAusw", "steuerfreie Arbeitgeberleistungen für Fahrtkosten bei Auswärtstätigkeit"),
    ("Besteuerungsgrund/StFreiVerpfleg", "steuerfreier Arbeitgeberzuschuss für Verpflegung bei Auswärtstätigkeit"),
    ("Besteuerungsgrund/StFreiDopHaushalt", "steuerfreie Arbeitgeberleistungen bei doppelter Haushaltsführung"),
    ("Besteuerungsgrund/FreibetragDbaTuerkei", "Freibetrag DBA Türkei"),
    ("Besteuerungsgrund/ErmStVBezMKalJahr/Wert", "Versorgungsbezüge für mehrere Kalenderjahre (im Bruttoarbeitslohn enthalten)"),
    ("Besteuerungsgrund/ErmStVBezMKalJahr/Jahr", "maßgebendes Kalenderjahr des Versorgungsbeginns bei ermäßigt besteuerten Versorgungsbezügen"),
    ("Besteuerungsgrund/Kammerbeitrag/Wert", "Kammerbeitrag"),
    ("Besteuerungsgrund/Kammerbeitrag/Bundesland", "Kammerbeitrag/maßgebendes Bundesland"),
];

/// Die Kind-Elemente eines `complexType` (durch `sequence`/`choice`/`all` hindurch). Ein Konstrukt, das dieser Lauf nicht versteht
/// (`extension`, `group`, ...), bricht laut ab: ein still uebersprungener Zweig waere derselbe Verlust, den der Test finden soll.
fn kind_elemente<'a, 'i>(typ: Node<'a, 'i>, out: &mut Vec<Node<'a, 'i>>) {
    for k in typ.children().filter(Node::is_element) {
        match k.tag_name().name() {
            "element" => out.push(k),
            "sequence" | "choice" | "all" => kind_elemente(k, out),
            "annotation" | "attribute" => {}
            fremd => panic!("Schema-Konstrukt `{fremd}` unter einem complexType: der Wächter liest es nicht"),
        }
    }
}

/// Steigt von `e` durch die benannten `complexType`s ab und sammelt den Pfad jedes Blatts (`weg` beginnt mit der Wurzel).
fn abstieg<'a, 'i>(typen: &HashMap<&str, Node<'a, 'i>>, e: Node<'a, 'i>, weg: &mut Vec<String>, blaetter: &mut Vec<String>) {
    assert!(weg.len() < 12, "Schema tiefer als 12 Ebenen: {weg:?}");
    let name = e.attribute("name").unwrap_or_else(|| panic!("Element ohne `name` (ref?): {e:?}"));
    weg.push(name.to_owned());
    assert!(
        !e.children().any(|k| k.is_element() && k.tag_name().name() == "complexType"),
        "Element {weg:?} mit eingebettetem complexType: der Wächter liest es nicht"
    );
    match e.attribute("type").and_then(|t| typen.get(t)) {
        Some(typ) => {
            let mut kinder = Vec::new();
            kind_elemente(*typ, &mut kinder);
            for k in kinder {
                abstieg(typen, k, weg, blaetter);
            }
        }
        None => blaetter.push(weg[1..].join("/")),
    }
    weg.pop();
}

/// Alle Blaetter (Elemente ohne eigene Kinder) der Lohnsteuerbescheinigung als Pfad ab der Wurzel, ohne die Wurzel selbst
/// (`Besteuerungsgrund/BruttoArbLohn`). Ein Behaelter wie `Besteuerungsgrund` traegt keinen Wert; seine Blaetter zaehlen einzeln.
fn beleg_blaetter(xsd: &str) -> Vec<String> {
    let doc = roxmltree::Document::parse(xsd).expect("das LStB-Schema ist XML");
    let schema = doc.root_element();
    let ist = |n: &Node, name: &str| n.is_element() && n.tag_name().name() == name;
    let typen: HashMap<&str, Node> =
        schema.children().filter(|n| ist(n, "complexType")).filter_map(|n| Some((n.attribute("name")?, n))).collect();
    let wurzeln: Vec<Node> = schema.children().filter(|n| ist(n, "element")).collect();
    assert_eq!(wurzeln.len(), 1, "genau ein Wurzelelement erwartet: {wurzeln:?}");
    let mut blaetter = Vec::new();
    abstieg(&typen, wurzeln[0], &mut Vec::new(), &mut blaetter);
    blaetter
}

/// Ohne Schema (laeuft in der CI): die beiden Lueckenlisten sind in sich stimmig. Ein Eintrag steht nur in EINER Liste, hat keinen
/// Namen, den das Mapping schon liest, und jeder Grund ist mehr als ein Wort (Python: `len(grund) > 40`).
#[test]
fn die_lueckenlisten_sind_in_sich_stimmig() {
    // KONTROLLE: die Schleifen unten sehen Eintraege (Python-Bestand: sieben Gruende, hier zwoelf Pfade; gemessen 2026-10-16).
    assert!(NICHT_GEMAPPT.len() >= 7, "nur {} bewusste Luecken", NICHT_GEMAPPT.len());
    let gelesen = vast_elemente();
    let mut gesehen = BTreeSet::new();
    for (pfad, _) in NICHT_GEMAPPT.iter().chain(NICHT_UNTERSUCHT.iter()) {
        assert!(gesehen.insert(*pfad), "{pfad} steht doppelt in den Lueckenlisten");
        let name = pfad.rsplit('/').next().unwrap();
        assert!(!gelesen.contains(name), "{pfad}: das Mapping liest {name} schon (LSTB oder LSTB_SUMMEN), der Eintrag gehört nicht in die Liste");
    }
    for (pfad, grund) in NICHT_GEMAPPT {
        assert!(grund.chars().count() > 40, "{pfad}: Begründung zu dünn ({} Zeichen): {grund}", grund.chars().count());
    }
    for (pfad, doku) in NICHT_UNTERSUCHT {
        assert!(!doku.trim().is_empty(), "{pfad}: auch ein ungeprüftes Element trägt wenigstens die Doku des Schemas");
    }
}

/// Mit Schema (nur lokal): JEDES Beleg-Feld der Lohnsteuerbescheinigung ist gemappt (`LSTB`, `LSTB_SUMMEN`) oder steht in einer der
/// beiden Listen. Ein neues Element im Schema faellt sonst beim Import ohne Meldung weg (Abweichung Nr. 69, Verlust (ii)).
#[test]
fn jedes_beleg_feld_des_schemas_ist_gemappt_oder_benannt() {
    if !schemas_da(2025) {
        return;
    }
    let pfad = elster::finde_schema(2025, "VaSt_LStB-{jahr}01.xsd").expect("VaSt_LStB-202501.xsd liegt neben den ELSTER-Schemas");
    let blaetter = beleg_blaetter(&std::fs::read_to_string(&pfad).unwrap());
    let gelesen = vast_elemente();
    let name = |p: &str| p.rsplit('/').next().unwrap().to_owned();

    // KONTROLLE: die Suche findet Blaetter (sonst bewiese ein Null-Ergebnis nichts), kennt ein gemapptes Element und weist ein erfundenes ab.
    assert!(blaetter.len() >= 50, "nur {} Blätter im Schema gelesen: {blaetter:?}", blaetter.len());
    assert!(blaetter.iter().any(|b| b == "Besteuerungsgrund/BruttoArbLohn"), "BruttoArbLohn nicht gefunden: {blaetter:?}");
    assert!(!blaetter.iter().any(|b| b.ends_with("ArbnKiSteuerX")));

    // Ein gemappter Name gilt fuer das Blatt, an dem `aus_lstb` ihn liest: steht er an zwei Pfaden, deckt der Eintrag beide zu.
    for n in &gelesen {
        let treffer: Vec<&String> = blaetter.iter().filter(|b| name(b) == *n).collect();
        assert_eq!(treffer.len(), 1, "{n} steht im Schema an {} Stellen: {treffer:?}", treffer.len());
    }
    let benannt: BTreeSet<&str> = NICHT_GEMAPPT.iter().chain(NICHT_UNTERSUCHT.iter()).map(|(p, _)| *p).collect();
    let ohne_eintrag: Vec<&String> = blaetter.iter().filter(|b| !gelesen.contains(name(b).as_str()) && !benannt.contains(b.as_str())).collect();
    assert!(
        ohne_eintrag.is_empty(),
        "{} Beleg-Felder des Schemas ohne Eintrag in LSTB, LSTB_SUMMEN, NICHT_GEMAPPT oder NICHT_UNTERSUCHT: {ohne_eintrag:?}",
        ohne_eintrag.len()
    );
    // Gegenrichtung: ein Eintrag, den das Schema nicht mehr kennt, ist tot (umbenannt oder entfernt) und deckt ein neues Element nicht.
    let tot: Vec<&&str> = benannt.iter().filter(|p| !blaetter.iter().any(|b| b == **p)).collect();
    assert!(tot.is_empty(), "Einträge der Lückenlisten, die das Schema nicht kennt: {tot:?}");
}

#[test]
fn die_vast_elemente_des_mappings_stehen_im_lstb_schema() {
    if !schemas_da(2025) {
        return;
    }
    let pfad = elster::finde_schema(2025, "VaSt_LStB-{jahr}01.xsd").expect("VaSt_LStB-202501.xsd liegt neben den ELSTER-Schemas");
    let xsd = std::fs::read_to_string(&pfad).unwrap();
    let im_schema: BTreeSet<&str> = xsd.split("name=\"").skip(1).filter_map(|s| s.split('"').next()).collect();
    let fehlend: Vec<&str> = vast_elemente().into_iter().filter(|n| !im_schema.contains(n)).collect();
    assert!(fehlend.is_empty(), "Nicht im LStB-Schema 2025: {fehlend:?}");
    // KONTROLLE: die Suche findet Namen (sonst bewiese ein Null-Ergebnis nichts) und weist einen erfundenen ab.
    assert!(im_schema.len() > 50, "nur {} Namen im Schema gelesen", im_schema.len());
    assert!(!im_schema.contains("ArbnKiSteuerX"));
}
