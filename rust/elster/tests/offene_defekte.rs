//! Offene Defekte der ELSTER-Seite: je Python-`xfail(strict=True)`-Fall ein Gegenstueck, das das
//! RICHTIGE Verhalten verlangt. `#[ignore]` haelt `cargo test` gruen (ein dauerhaft roter Test
//! zerstoert das Signal, `73d4245`); `cargo test -p elster --test offene_defekte -- --ignored`
//! zeigt jeden Fall rot. Der Ignore-Text nennt Grund, Python-Test und Vault-Notiz.
//!
//! Lesart des Rots: die Defekt-Zusicherung beginnt mit `DEFEKT:`. Eine Kontrolle (Messaufbau,
//! Normalfall, Werkzeug) beginnt mit `KONTROLLE:`; scheitert sie, misst der Test nichts.
//!
//! Die Felder entstehen direkt, alle bestaetigt: die Python-Tests schreiben ueber `append_event`
//! ohne `bindung=`, dort laeuft keine Ableitung (`store.py`, `_leite_ab`/`_rechne_ab`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Herkunft, PruefTiefe, Vz, Zustand};
use elster::{deklariere, erzeuge_xml, Deklaration, Felder, XmlOptionen};
use serde_json::{json, Value};
use store::SnapshotFeld;

/// Das Jahr fuer `deklariere`. Kein Fall hier traegt ein eigenes: die Python-Vorlagen laufen
/// alle mit 2025 (`vz=2025` bzw. `"veranlagungszeitraum": 2025`), und `xml` baut mit
/// `XmlOptionen::default()`, also auch fuer 2025.
const VZ: i64 = 2025;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let pfad = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        bindung::lade_registry(&pfad)
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

/// Alle Paare bestaetigt, Herkunft laie/ungeprueft/nutzer (`_b` der Python-Tests).
fn bestaetigt(paare: &[(&str, Value)]) -> Felder {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    paare
        .iter()
        .map(|(feld_id, wert)| {
            let herkunft = Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            };
            let feld = SnapshotFeld {
                wert: wert.clone().into(),
                zustand: Zustand::Bestaetigt,
                herkunft: herkunft.into(),
            };
            ((*feld_id).to_owned(), feld)
        })
        .collect()
}

fn xml(d: &Deklaration, hersteller_id: &str, snapshot: Option<&Felder>) -> String {
    let opt = XmlOptionen {
        hersteller_id: Some(hersteller_id.to_owned()),
        snapshot,
        ..XmlOptionen::default()
    };
    erzeuge_xml(d, &opt).unwrap_or_else(|e| panic!("KONTROLLE: erzeuge_xml scheitert: {e}"))
}

/// Text je Element mit dem Lokalnamen `name`, in Dokumentreihenfolge (`_kz_text`).
fn texte(xml: &str, name: &str) -> Vec<String> {
    let doc = roxmltree::Document::parse(xml).expect("KONTROLLE: XML parst");
    doc.descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == name)
        .map(|n| n.text().unwrap_or_default().to_owned())
        .collect()
}

// ---------------------------------------------------------------- A_B_LP-Summe

/// Kz, Feld Person A, Cent A, Feld Partner, Cent Partner (`_KATEGORIEN`, unterscheidbar je Seite).
const AB_LP: [(&str, &str, i64, &str, i64); 5] = [
    (
        "E2001403",
        "vorsorge_arbeitslosenversicherung",
        50_000,
        "vorsorge_arbeitslosenversicherung_partner",
        48_000,
    ),
    (
        "E2001503",
        "vorsorge_erwerbsunfaehigkeit",
        30_000,
        "vorsorge_erwerbsunfaehigkeit_partner",
        32_000,
    ),
    (
        "E2001803",
        "vorsorge_unfall_haftpflicht",
        20_000,
        "vorsorge_unfall_haftpflicht_partner",
        21_000,
    ),
    (
        "E2001903",
        "vorsorge_rv_alt_mit_ueberschuss",
        40_000,
        "vorsorge_rv_alt_mit_ueberschuss_partner",
        41_000,
    ),
    (
        "E2002003",
        "vorsorge_rv_alt_ohne_ueberschuss",
        30_000,
        "vorsorge_rv_alt_ohne_ueberschuss_partner",
        29_000,
    ),
];

/// `test_a_b_lp_summenfehler.py`: `A_B_LP` hat laut E10-2025.xsd keinen Person-Diskriminator, das
/// Sum-Kz ist die gemeinsame Summe beider Ehegatten. Kontrolle: nur Person A traegt ihren Betrag.
fn ab_lp_summe(kz: &str) {
    let (_, feld_a, cent_a, feld_b, cent_b) = *AB_LP.iter().find(|k| k.0 == kz).unwrap();
    let sum_kz = |paare: &[(&str, Value)]| {
        let mut alle = vec![("veranlagung", json!("zusammen"))];
        alle.extend_from_slice(paare);
        let d = deklariere(&bestaetigt(&alle), index(), VZ, None).unwrap();
        texte(&xml(&d, "TESTHID-NICHT-ECHT", None), kz)
    };
    let nur_a = sum_kz(&[(feld_a, json!(cent_a))]);
    assert_eq!(
        nur_a,
        [(cent_a / 100).to_string()],
        "KONTROLLE: nur Person A, {kz}"
    );
    let beide = sum_kz(&[(feld_a, json!(cent_a)), (feld_b, json!(cent_b))]);
    assert_eq!(
        beide,
        [((cent_a + cent_b) / 100).to_string()],
        "DEFEKT: {kz} traegt nicht die Summe A + Partner"
    );
}

#[test]
#[ignore = "A_B_LP-Sum-Kz traegt nur Person A, der Partner-Betrag (elster_kz: null) wird nicht addiert; laut E10-2025.xsd gemeinsame Summe beider Ehegatten. Python: test_a_b_lp_summenfehler.py::test_person_a_und_partner_werden_addiert[E2001403]. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn ab_lp_summe_e2001403() {
    ab_lp_summe("E2001403");
}

#[test]
#[ignore = "A_B_LP-Sum-Kz traegt nur Person A, der Partner-Betrag (elster_kz: null) wird nicht addiert; laut E10-2025.xsd gemeinsame Summe beider Ehegatten. Python: test_a_b_lp_summenfehler.py::test_person_a_und_partner_werden_addiert[E2001503]. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn ab_lp_summe_e2001503() {
    ab_lp_summe("E2001503");
}

#[test]
#[ignore = "A_B_LP-Sum-Kz traegt nur Person A, der Partner-Betrag (elster_kz: null) wird nicht addiert; laut E10-2025.xsd gemeinsame Summe beider Ehegatten. Python: test_a_b_lp_summenfehler.py::test_person_a_und_partner_werden_addiert[E2001803]. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn ab_lp_summe_e2001803() {
    ab_lp_summe("E2001803");
}

#[test]
#[ignore = "A_B_LP-Sum-Kz traegt nur Person A, der Partner-Betrag (elster_kz: null) wird nicht addiert; laut E10-2025.xsd gemeinsame Summe beider Ehegatten. Python: test_a_b_lp_summenfehler.py::test_person_a_und_partner_werden_addiert[E2001903]. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn ab_lp_summe_e2001903() {
    ab_lp_summe("E2001903");
}

#[test]
#[ignore = "A_B_LP-Sum-Kz traegt nur Person A, der Partner-Betrag (elster_kz: null) wird nicht addiert; laut E10-2025.xsd gemeinsame Summe beider Ehegatten. Python: test_a_b_lp_summenfehler.py::test_person_a_und_partner_werden_addiert[E2002003]. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn ab_lp_summe_e2002003() {
    ab_lp_summe("E2002003");
}

// ---------------------------------------------------------------- elf Gewinnfelder

const PERSON_A_GEWINN: [&str; 6] = [
    "betriebseinnahmen",
    "afa_jahresbetrag",
    "gewinnanteil",
    "verguetung_taetigkeit",
    "verguetung_darlehen",
    "verguetung_ueberlassung",
];
const PARTNER_GEWINN: [&str; 4] = [
    "gewinnanteil_partner",
    "verguetung_taetigkeit_partner",
    "verguetung_darlehen_partner",
    "verguetung_ueberlassung_partner",
];
const KAP_SONSTIGE: &str = "kap_gewinn_sonstige";

/// Die elf Felder in Python-Reihenfolge mit `BETRAG_CENT` = (i + 1) * 111100.
fn elf() -> Vec<(&'static str, i64)> {
    PERSON_A_GEWINN
        .iter()
        .chain(PARTNER_GEWINN.iter())
        .chain([KAP_SONSTIGE].iter())
        .zip(1_i64..)
        .map(|(f, i)| (*f, i * 111_100))
        .collect()
}

/// Kz → Texte aller `E` + 7 Ziffern-Elemente (`_kz_werte_aus_xml`).
fn kz_werte(xml: &str) -> BTreeMap<String, Vec<String>> {
    let doc = roxmltree::Document::parse(xml).expect("KONTROLLE: XML parst");
    let mut werte: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in doc.descendants().filter(roxmltree::Node::is_element) {
        let name = n.tag_name().name();
        if name.len() == 8 && name.starts_with('E') && name[1..].bytes().all(|b| b.is_ascii_digit())
        {
            let text = n.text().unwrap_or_default().to_owned();
            werte.entry(name.to_owned()).or_default().push(text);
        }
    }
    werte
}

/// Steht `eur` als `n`, `n.00` oder `n,00` in irgendeinem Kz (`_betrag_im_xml`)?
fn betrag_im_xml(werte: &BTreeMap<String, Vec<String>>, eur: i64) -> bool {
    let formen = [eur.to_string(), format!("{eur}.00"), format!("{eur},00")];
    werte
        .values()
        .flatten()
        .any(|w| formen.iter().any(|f| f == w.trim()))
}

/// Einzelbetraege und die vier Zwischensummen (`_summen`), je mit Etikett.
fn elf_betraege() -> Vec<(String, i64)> {
    let elf = elf();
    let summe = |felder: &[&str]| {
        elf.iter()
            .filter(|(f, _)| felder.contains(f))
            .map(|(_, c)| c)
            .sum::<i64>()
            / 100
    };
    let alle: Vec<&str> = elf.iter().map(|(f, _)| *f).collect();
    let zehn: Vec<&str> = alle
        .iter()
        .copied()
        .filter(|f| *f != KAP_SONSTIGE)
        .collect();
    let mut betraege: Vec<(String, i64)> = elf
        .iter()
        .map(|(f, c)| ((*f).to_owned(), c / 100))
        .collect();
    betraege.extend([
        (
            "Summe 6 Person-A-Gewinn-Komponenten".to_owned(),
            summe(&PERSON_A_GEWINN),
        ),
        ("Summe 4 _partner-Felder".to_owned(), summe(&PARTNER_GEWINN)),
        ("Summe alle elf".to_owned(), summe(&alle)),
        (
            "Summe zehn ohne kap_gewinn_sonstige".to_owned(),
            summe(&zehn),
        ),
    ]);
    betraege
}

/// `_basis`: Person A/B, Pflichtkegel, Direktwert `einkuenfte_gewinn` bewusst 0.
fn elf_lauf(mit_elf: bool) -> BTreeMap<String, Vec<String>> {
    let mut paare: Vec<(&str, Value)> = vec![
        ("bruttoarbeitslohn", json!(5_000_000)),
        ("vor_an_anteil_rv", json!(200_000)),
        ("vor_ag_anteil_rv", json!(150_000)),
        ("vor_rv_ausserhalb_lstb", json!(100_000)),
        ("kap_kapitalertraege", json!(500_000)),
        ("kap_gewinn_aktien", json!(0)),
        ("kap_verlust_aktien", json!(0)),
        ("kap_verlust_sonstige", json!(0)),
        ("vv_einnahmen", json!(0)),
        ("bruttoarbeitslohn_partner", json!(4_000_000)),
        ("vor_an_anteil_rv_partner", json!(160_000)),
        ("vor_ag_anteil_rv_partner", json!(120_000)),
        ("vor_rv_ausserhalb_lstb_partner", json!(80_000)),
        ("kap_kapitalertraege_partner", json!(300_000)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("veranlagung", json!("zusammen")),
        ("kein_gewinn", json!(false)),
        ("kein_kap", json!(false)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("gewinn_betriebsart", json!("gewerbe")),
        ("gewinn_bezeichnung", json!("Testfall")),
        ("einkuenfte_gewinn", json!(0)),
    ];
    if mit_elf {
        paare.extend(elf().into_iter().map(|(f, c)| (f, json!(c))));
    }
    let felder = bestaetigt(&paare);
    let d = deklariere(&felder, index(), VZ, None).unwrap();
    assert!(
        d.eingaben_konsistent(),
        "KONTROLLE: eingaben_konsistent (mit_elf={mit_elf}): {:?}",
        d.unvollstaendig()
    );
    kz_werte(&xml(&d, "74931", Some(&felder)))
}

/// `test_elf_gewinnfelder_geld_fehlt_im_xml.py`: bestaetigtes Geld landet in der Erklaerung.
/// Elf Betragsfelder mit `elster_kz: null` erreichen das XML weder einzeln noch als
/// Zwischensumme. Kontrollen: Leerlauf ohne die elf (kein Betrag im Skelett) und das verdrahtete
/// `bruttoarbeitslohn_partner` → E0200201 im selben Lauf.
#[test]
#[ignore = "Elf Betragsfelder (6 Gewinnkomponenten Person A, 4 _partner, kap_gewinn_sonstige) haben elster_kz: null und erreichen das XML weder einzeln noch als Zwischensumme; Reparaturrichtung offen. Python: test_elf_gewinnfelder_geld_fehlt_im_xml.py::test_elf_gewinnfelder_und_ihre_summen_erreichen_die_erklaerung. Vault: audits/ring-gegen-deklaration-ueber-und-untererklaerung.md. Rot sehen: --ignored"]
fn elf_gewinnfelder_erreichen_die_erklaerung() {
    let leer = elf_lauf(false);
    let fehltreffer: Vec<String> = elf_betraege()
        .into_iter()
        .filter(|(_, eur)| betrag_im_xml(&leer, *eur))
        .map(|(etikett, eur)| format!("{etikett}={eur}"))
        .collect();
    assert!(
        fehltreffer.is_empty(),
        "KONTROLLE: Betrag steht ohne Eingabe im XML: {fehltreffer:?}"
    );
    let haupt = elf_lauf(true);
    assert!(
        haupt
            .get("E0200201")
            .is_some_and(|w| w.iter().any(|t| t == "40000")),
        "KONTROLLE: E0200201 (bruttoarbeitslohn_partner) fehlt: {:?}",
        haupt.get("E0200201")
    );
    let fehlend: Vec<String> = elf_betraege()
        .into_iter()
        .filter(|(_, eur)| !betrag_im_xml(&haupt, *eur))
        .map(|(etikett, eur)| format!("{etikett}={eur}"))
        .collect();
    assert!(
        fehlend.is_empty(),
        "DEFEKT: {} Betraege/Summen erreichen die Erklaerung nicht: {fehlend:?}",
        fehlend.len()
    );
}

// ---------------------------------------------------------------- § 23 Partner-Verkauf

fn p23_partner_felder(mit_partner: bool) -> Felder {
    let mut paare: Vec<(&str, Value)> = vec![
        ("stammdaten_nachname", json!("Meier")),
        ("stammdaten_vorname", json!("Klaus")),
        ("stammdaten_geburtsdatum", json!("01.01.1970")),
        ("stammdaten_strasse", json!("Teststr.")),
        ("stammdaten_hausnummer", json!("1")),
        ("stammdaten_plz", json!("10115")),
        ("stammdaten_wohnort", json!("Berlin")),
        ("stammdaten_keine_bankverbindung", json!(true)),
        ("stammdaten_art_est_erklaerung", json!(true)),
        ("kist_konfession", json!("keine")),
        ("veranlagung", json!("zusammen")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("bruttoarbeitslohn", json!(0)),
        ("vor_an_anteil_rv", json!(0)),
        ("vor_ag_anteil_rv", json!(0)),
        ("vor_rv_ausserhalb_lstb", json!(0)),
        ("p23_veraeusserungspreis", json!(9_000_000)),
        ("p23_anschaffung_herstellungskosten", json!(4_500_000)),
        ("p23_werbungskosten", json!(0)),
        ("p23_veraeusserungs_typ", json!("grundstueck")),
    ];
    if mit_partner {
        // einziger heutiger Eingabeweg fuer einen zweiten Verkauf: die Zaehl-Instanz __2
        paare.extend([
            ("p23_veraeusserungspreis__2", json!(2_000_000)),
            ("p23_anschaffung_herstellungskosten__2", json!(1_200_000)),
            ("p23_werbungskosten__2", json!(0)),
            ("p23_veraeusserungs_typ__2", json!("grundstueck")),
        ]);
    }
    bestaetigt(&paare)
}

/// `_xml_bauen`: Deklaration und echtes Uebermittlungs-XML.
fn p23_partner_xml(mit_partner: bool) -> String {
    let d = deklariere(&p23_partner_felder(mit_partner), index(), VZ, None).unwrap();
    assert!(
        d.eingaben_konsistent(),
        "KONTROLLE: eingaben_konsistent (mit_partner={mit_partner}): {:?}",
        d.unvollstaendig()
    );
    xml(&d, "74931", None)
}

/// `_so_grdst_person_betrag_paare`: Anzahl `<SO>` und {(Person, E0306801)} je `<Grdst>`.
fn so_person_betrag(xml: &str) -> (usize, BTreeSet<(String, String)>) {
    let doc = roxmltree::Document::parse(xml).expect("KONTROLLE: XML parst");
    let heisst =
        |n: &roxmltree::Node<'_, '_>, name: &str| n.is_element() && n.tag_name().name() == name;
    let so: Vec<_> = doc.descendants().filter(|n| heisst(n, "SO")).collect();
    let mut paare = BTreeSet::new();
    for grdst in so
        .iter()
        .flat_map(roxmltree::Node::descendants)
        .filter(|n| heisst(n, "Grdst"))
    {
        let person = grdst
            .descendants()
            .find(|n| heisst(n, "Person"))
            .and_then(|n| n.text())
            .unwrap_or("<keine Person>")
            .to_owned();
        for betrag in grdst.descendants().filter(|n| heisst(n, "E0306801")) {
            paare.insert((person.clone(), betrag.text().unwrap_or_default().to_owned()));
        }
    }
    (so.len(), paare)
}

fn paare(liste: &[(&str, &str)]) -> BTreeSet<(String, String)> {
    liste
        .iter()
        .map(|(p, b)| ((*p).to_owned(), (*b).to_owned()))
        .collect()
}

/// Kontrolle beider § 23-Tests: ein Verkauf von Person A → ein `<SO>`, (`PersonA`, 45000).
fn p23_kontrolle_person_a() {
    let (anzahl_so, gefunden) = so_person_betrag(&p23_partner_xml(false));
    assert_eq!(anzahl_so, 1, "KONTROLLE: ein Verkauf, Anzahl <SO>");
    assert_eq!(
        gefunden,
        paare(&[("PersonA", "45000")]),
        "KONTROLLE: ein Verkauf von Person A"
    );
}

/// `test_p23_partner_verkauf_still_unter_person_a_eingereicht.py`: der Partner-Verkauf ueber die
/// Zaehl-Instanz `__2` landet still unter Person A. Erwartet: ein `<SO>` mit zwei
/// Person-unterschiedenen Verkaufsbloecken.
#[test]
#[ignore = "Zweiter Verkauf (Zaehl-Instanz __2) haengt an <Einz> derselben Grdst-Instanz, beide Betraege tragen Person=PersonA; erwartet {(PersonA,45000),(PersonB,8000)}. Python: test_p23_partner_verkauf_still_unter_person_a_eingereicht.py::test_person_a_und_partner_verkauf_zwei_instanzen_mit_unterschiedlichem_person_kennzeichen. Vault: audits/p23-personenachse-melder-verbraucht-defekt-bleibt.md. Rot sehen: --ignored"]
fn p23_partner_verkauf_traegt_person_b() {
    p23_kontrolle_person_a();
    let (anzahl_so, gefunden) = so_person_betrag(&p23_partner_xml(true));
    assert_eq!(
        anzahl_so, 1,
        "KONTROLLE: Schema-Achse (fa9453d), genau ein <SO>"
    );
    assert_eq!(
        gefunden,
        paare(&[("PersonA", "45000"), ("PersonB", "8000")]),
        "DEFEKT: Personenachse, der Partner-Verkauf steht nicht unter PersonB"
    );
}

/// Dieselbe Erwartung, zuerst die heute WAHRE Schema-Gueltigkeit als Tatsache: `<Einz>` traegt
/// kein Person-Feld, das XSD kann diesen Fehler grundsaetzlich nicht sehen.
#[test]
#[ignore = "Schema-Gueltigkeit war ein Stellvertretermerkmal: xmllint akzeptiert das Zwei-Verkaeufe-XML seit fa9453d, die Personenachse bleibt falsch (<Einz> traegt kein Person-Feld). Python: test_p23_partner_verkauf_still_unter_person_a_eingereicht.py::test_person_a_und_partner_verkauf_ist_heute_xsd_valide_aber_personenachse_falsch. Vault: audits/p23-stellvertretermerkmal-hoert-auf-zu-stellvertreten.md. Rot sehen: --ignored"]
fn p23_partner_verkauf_xsd_valide_aber_personenachse_falsch() {
    assert!(
        elster::finde_xsd_schema(Vz::Vz2025).is_some(),
        "KONTROLLE: elster11_E10_2025_extern.xsd fehlt - source_unavailable"
    );
    p23_kontrolle_person_a();
    let xml = p23_partner_xml(true);
    let (ok, meldung) = elster::validiere_xsd_text(xml.as_bytes(), Vz::Vz2025);
    assert!(
        ok,
        "KONTROLLE: xmllint akzeptiert das Dokument (fa9453d): {meldung}"
    );
    let (anzahl_so, gefunden) = so_person_betrag(&xml);
    assert_eq!(anzahl_so, 1, "KONTROLLE: genau ein <SO>");
    assert_eq!(
        gefunden,
        paare(&[("PersonA", "45000"), ("PersonB", "8000")]),
        "DEFEKT: Personenachse, der Partner-Verkauf steht nicht unter PersonB"
    );
}

// ---------------------------------------------------------------- § 23 ERiC-Gate

/// Hersteller-ID aus der Umgebung oder den `.env`-Dateien der Repo-Wurzel, sonst die gesperrte
/// Platzhalter-ID 74931 (`checkest_gate.py`, `_PLATZHALTER_HID`). Wie
/// `parity/tests/elster_paritaet.rs::hersteller_id`. Nie ausgeben.
fn hersteller_id() -> String {
    if let Ok(h) = std::env::var("ELSTER_HERSTELLER_ID") {
        if !h.trim().is_empty() {
            return h.trim().to_owned();
        }
    }
    let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for datei in [".env", ".env.elster", ".env.local"] {
        let Ok(text) = std::fs::read_to_string(wurzel.join(datei)) else {
            continue;
        };
        for zeile in text.lines() {
            let zeile = zeile.trim().trim_start_matches("export ");
            if let Some(w) = zeile.strip_prefix("ELSTER_HERSTELLER_ID=") {
                let w = w.trim().trim_matches(['"', '\'']);
                if !w.is_empty() {
                    return w.to_owned();
                }
            }
        }
    }
    "74931".to_owned()
}

/// `test_p23_mehrfachverkauf_bricht_so_maxoccurs.py`: misst nur, OB ERiC geprueft hat
/// (Klassenzugehoerigkeit, nicht ein einzelner rc), nicht WAS er sagt. Gewollte Abweichung:
/// Python setzt immer 74931 ins XML und kann deshalb nie XPASS werden; hier zaehlt die echte ID.
#[test]
#[ignore = "ERiC prueft das Zwei-Verkaeufe-XML nicht: ohne registrierte Hersteller-ID antwortet er mit einer NICHT-GEPRUEFT-Klasse (gemessen rc=610301202 ERIC_IO_TESTHERSTELLERID_GESPERRT, leerer Puffer). Kein Code-Defekt, Umgebungs-Gate. Python: test_p23_mehrfachverkauf_bricht_so_maxoccurs.py::test_eric_hat_ueberhaupt_geantwortet_zwei_verkaeufe. Vault: audits/p23-so-maxoccurs1-zwei-achsen-verwechselt.md. Rot sehen: --ignored"]
fn p23_eric_prueft_zwei_verkaeufe() {
    assert!(
        elster::find_eric_lib().is_some(),
        "KONTROLLE: ERiC fehlt - source_unavailable, nicht der Defekt"
    );
    let mut paare: Vec<(&str, Value)> = vec![
        ("bruttoarbeitslohn", json!(5_000_000)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("fam_anzahl_kinder", json!(0)),
        ("verlustvortrag_bestand", json!(0)),
        ("p23_anzahl_verkaeufe", json!(2)),
    ];
    for (suffix, preis, ak, wk) in [
        ("", 20_000_000, 15_000_000, 500_000),
        ("__2", 13_000_000, 9_000_000, 200_000),
    ] {
        for (feld, wert) in [
            ("p23_veraeusserungspreis", json!(preis)),
            ("p23_anschaffung_herstellungskosten", json!(ak)),
            ("p23_werbungskosten", json!(wk)),
            ("p23_veraeusserungs_typ", json!("grundstueck")),
        ] {
            let feld_id: &'static str = Box::leak(format!("{feld}{suffix}").into_boxed_str());
            paare.push((feld_id, wert));
        }
    }
    let felder = bestaetigt(&paare);
    let d = deklariere(&felder, index(), VZ, None).unwrap();
    let xml = xml(&d, &hersteller_id(), Some(&felder));
    let (rc, _antwort) = elster::validiere(xml.as_bytes(), "ESt_2025")
        .unwrap_or_else(|e| panic!("KONTROLLE: ERiC laedt nicht: {e:?}"));
    let klasse = elster::klassifiziere_rc(i64::from(rc));
    assert!(
        !elster::nicht_geprueft(klasse),
        "DEFEKT: ERiC antwortet mit rc={rc} ({klasse:?}), einer NICHT-GEPRUEFT-Klasse - keine echte Pruefung"
    );
}

// ---------------------------------------------------------------- Anlage L

/// `_erklaere_gewinn` ohne HTTP: in Rust ist `GET /deklaration` ein 501-Stub. Die Kz-Wahl sitzt
/// in `deklariere` (`tabellen.rs`, `GEWINN`), der Ring-Schritt (`mit_ring_werten`) beruehrt
/// keine Gewinnfelder.
fn gewinn_deklaration(betriebsart: &str, bezeichnung: &str) -> Deklaration {
    let felder = bestaetigt(&[
        ("veranlagung", json!("einzel")),
        ("gewinn_betriebsart", json!(betriebsart)),
        ("einkuenfte_gewinn", json!(3_000_000)),
        ("gewinn_bezeichnung", json!(bezeichnung)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_p23_verkauf", json!(true)),
        ("kein_gewinn", json!(false)),
    ]);
    deklariere(&felder, index(), VZ, None).unwrap()
}

/// Kz mit dem Wert 30000 (Python: `v == 30000`, int und float gleich).
fn kz_mit_30000(d: &Deklaration) -> Vec<&String> {
    d.deklaration
        .iter()
        .filter(|(_, v)| v.as_f64() == Some(30_000.0))
        .map(|(k, _)| k)
        .collect()
}

/// `test_luf_gewinn_kz_fehlt.py`: der Land-/Forstwirtschafts-Gewinn steht in keiner Zeile der
/// Deklaration. Kontrolle: `gewerbe` traegt 30000 und die Bezeichnung.
#[test]
#[ignore = "Anlage-L-Kz fuer den laufenden Gewinn fehlt bewusst (tabellen.rs GEWINN ohne land_forst): die Kz-Wahl haengt an Gewinnermittlungsart E0900407 und Wirtschaftsjahr-Lage E0900101, beide Felder fehlen im Projekt. Python: test_luf_gewinn_kz_fehlt.py::test_land_forst_gewinn_fehlt_in_deklaration. Vault: decisions/anlage-l-kein-einzelnes-kz.md. Rot sehen: --ignored"]
fn land_forst_gewinn_steht_in_der_deklaration() {
    let gewerbe = gewinn_deklaration("gewerbe", "Testbetrieb");
    assert!(
        gewerbe.eingaben_konsistent(),
        "KONTROLLE: gewerbe konsistent: {:?}",
        gewerbe.unvollstaendig()
    );
    assert!(
        !kz_mit_30000(&gewerbe).is_empty(),
        "KONTROLLE: gewerbe, kein Kz mit 30000: {:?}",
        gewerbe.deklaration
    );
    assert!(
        gewerbe.deklaration.values().any(|v| v == "Testbetrieb"),
        "KONTROLLE: gewerbe, gewinn_bezeichnung fehlt: {:?}",
        gewerbe.deklaration
    );
    let land_forst = gewinn_deklaration("land_forst", "Testhof");
    assert!(
        !kz_mit_30000(&land_forst).is_empty(),
        "DEFEKT: land_forst, kein Kz mit 30000 in der Deklaration: {:?}",
        land_forst.deklaration
    );
}
