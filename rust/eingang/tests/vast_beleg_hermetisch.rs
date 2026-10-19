//! Der Leser der Beleg-Datei (`eingang::vast_beleg::beleg_xml_zu_werten`, Abweichung Nr. 90, Stufe S1): eine Beleg-XML der Lohnsteuerbescheinigung
//! (Schema `VaSt_LStB-202501.xsd`) ergibt die flache Zuordnung Element → Text für `aus_lstb`. Hermetisch: jede Fixtur ist erfunden (Firma, Name,
//! Beträge; die Steuer-ID `00000000000` beginnt mit 0 und kann nie vergeben sein), nichts kommt aus einem echten Beleg, kein `ERiC`, kein ELSTER.
//!
//! Aufbau der Fixtur nach dem Schema: Wurzel `VaSt_LStB` mit Namensraum und `version="202501"`; darin `Allgemein` (Steuer-ID, Name), `Dauer`
//! (`Jahr`), `Besteuerungsmerkmal` (`ELStAM`, bis zu 12-mal) und `Besteuerungsgrund` (die Beträge, je Element höchstens einmal).
//!
//! Fehler: zweiter Beleg, wiederholtes Blatt, falsche Version, kaputtes XML und nicht lesbarer Betrag ergeben je einen benannten Fehler. Kein
//! Fehlertext nennt die Steuer-ID, den Namen oder den Wert eines Betrags.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use eingang::vast::aus_lstb;
use eingang::vast_beleg::{beleg_xml_zu_werten, BelegFehler, BELEG_NAMENSRAUM};

const IDNR: &str = "00000000000";
const NAME: &str = "Mustermann";
const VORNAME: &str = "Erika";

/// (Element, Text, Zielfeld, Cent): erfundene, unterschiedliche Beträge für alle 15 Elemente, die `LSTB` und `LSTB_SUMMEN` lesen.
/// Die zwei Summenziele tragen je zwei Elemente.
const BETRAEGE: [(&str, &str, &str, i64); 15] = [
    ("BruttoArbLohn", "61234.56", "bruttoarbeitslohn", 6_123_456),
    ("LSteuer", "9876.54", "p36_lohnsteuer", 987_654),
    ("ArbnKiSteuer", "612.34", "kist_gezahlt", 61_234),
    ("LeistungenProgVorbeh", "1500.10", "p32b_progressionseinkuenfte", 150_010),
    ("ArbnAnteilKrankVers", "3210.98", "basis_kv", 321_098),
    ("ArbnAnteilPflegVers", "654.32", "basis_pv", 65_432),
    ("StFreiVerpfleg", "120.00", "vpf_steuerfreie_erstattung_betrag", 12_000),
    ("StFreiGeKrankVers", "1111.11", "zuschuss_ag_kv_gesetzlich", 111_111),
    ("StFreiPrKrankVers", "2222.22", "zuschuss_ag_kv_privat", 222_222),
    ("StFreiGePflegVers", "333.33", "zuschuss_ag_pv", 33_333),
    ("ArbnAnteilArblVers", "745.67", "vorsorge_arbeitslosenversicherung", 74_567),
    // Summenziel `vor_an_anteil_rv`: 5678,90 + 100,01 = 5778,91 EUR.
    ("ArbnAnteilRenVers", "5678.90", "vor_an_anteil_rv", 577_891),
    ("ArbnAnteilBerufsVers", "100.01", "vor_an_anteil_rv", 577_891),
    // Summenziel `vor_ag_anteil_rv`: 5678,91 + 200,02 = 5878,93 EUR.
    ("ArbgAnteilRenVers", "5678.91", "vor_ag_anteil_rv", 587_893),
    ("ArbgAnteilBerufsVers", "200.02", "vor_ag_anteil_rv", 587_893),
];

/// Die Beträge als `<Element>Text</Element>`-Zeilen, eingerückt wie ein Beleg aus ELSTER.
fn grund_alle() -> String {
    BETRAEGE.iter().map(|(e, t, _, _)| format!("      <{e}>{t}</{e}>\n")).collect::<Vec<_>>().concat()
}

/// Ein Beleg: Namensraum, `version`-Attribut (`None` = fehlt), `Dauer/Jahr`, Zahl der `ELStAM` und der Inhalt von `Besteuerungsgrund`.
fn beleg_mit(ns: &str, version: Option<&str>, jahr: &str, elstam: usize, grund: &str) -> String {
    let version = version.map_or_else(String::new, |v| format!(" version=\"{v}\""));
    let elstam: String = (0..elstam)
        .map(|i| {
            format!(
                "      <ELStAM>\n        <GueltigAb>01{:02}</GueltigAb>\n        <Steuerklasse><Klasse>1</Klasse></Steuerklasse>\n      </ELStAM>\n",
                i % 12 + 1
            )
        })
        .collect::<Vec<_>>()
        .concat();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<VaSt_LStB xmlns=\"{ns}\"{version}>\n  <ArbGName>Erfundene Firma GmbH</ArbGName>\n  \
         <Eingangsdatum>28.02.2026 10:00:00</Eingangsdatum>\n  <Allgemein>\n    <IdNr>{IDNR}</IdNr>\n    <Name>{NAME}</Name>\n    \
         <Vorname>{VORNAME}</Vorname>\n  </Allgemein>\n  <Dauer>\n    <Jahr>{jahr}</Jahr>\n    <Anfang>0101</Anfang>\n    <Ende>3112</Ende>\n  </Dauer>\n  \
         <Besteuerungsmerkmal>\n{elstam}  </Besteuerungsmerkmal>\n  <Besteuerungsgrund>\n{grund}  </Besteuerungsgrund>\n</VaSt_LStB>\n"
    )
}

/// Der gültige Beleg: Schema 202501, Jahr 2025, zwei `ELStAM`, alle 15 Beträge.
fn beleg_gut() -> String {
    beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 2, &grund_alle())
}

/// Ein Beleg in einem Behälter wie `Nutzdaten`.
fn im_behaelter(belege: &[String]) -> String {
    let innen: String = belege.iter().map(|b| b.replace("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n", "")).collect();
    format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Nutzdaten>\n{innen}</Nutzdaten>\n")
}

fn fehler(xml: &str) -> BelegFehler {
    beleg_xml_zu_werten(xml).expect_err("die Datei muss abgelehnt werden")
}

#[test]
fn ein_beleg_ergibt_die_flache_zuordnung_element_zu_text() {
    let werte = beleg_xml_zu_werten(&beleg_gut()).unwrap();
    let erwartet: BTreeMap<String, String> = BETRAEGE.iter().map(|(e, t, _, _)| ((*e).to_owned(), (*t).to_owned())).collect();
    assert_eq!(werte, erwartet, "genau die 15 Beträge, als Text, ohne Steuer-ID und Namen");
    assert_eq!(werte.len(), 15);
}

#[test]
fn die_zuordnung_des_belegs_kommt_als_cent_je_zielfeld_aus_aus_lstb() {
    let werte = beleg_xml_zu_werten(&beleg_gut()).unwrap();
    let saetze = aus_lstb(&werte).unwrap();
    // 13 Ziele: 10 aus `LSTB`, 3 aus `LSTB_SUMMEN`.
    assert_eq!(saetze.len(), 13, "{saetze:?}");
    let je_feld: BTreeMap<&str, i64> = saetze.iter().map(|s| (s.feld_id.as_str(), s.wert)).collect();
    for (element, _, feld, cent) in BETRAEGE {
        assert_eq!(je_feld.get(feld), Some(&cent), "{element} -> {feld}");
    }
}

#[test]
fn der_beleg_traegt_weder_steuer_id_noch_namen_in_die_zuordnung() {
    let werte = beleg_xml_zu_werten(&beleg_gut()).unwrap();
    for (schluessel, wert) in &werte {
        for geheim in [IDNR, NAME, VORNAME, "Erfundene Firma"] {
            assert!(!schluessel.contains(geheim) && !wert.contains(geheim), "{schluessel}={wert} nennt {geheim}");
        }
    }
    for element in ["IdNr", "Name", "Vorname", "ArbGName", "Eingangsdatum", "Jahr", "Klasse", "GueltigAb"] {
        assert!(!werte.contains_key(element), "{element} gehört nicht in die Zuordnung");
    }
}

#[test]
fn ein_beleg_mit_zwoelf_elstam_und_den_anderen_wiederholbaren_behaeltern_wird_gelesen() {
    // `ELStAM` (12), `VBez` (12), `ErmStVBezMKalJahr` (12), `Kammerbeitrag` (2) sind im Schema wiederholbar; ihre Kinder heißen je Wiederholung gleich.
    let mut grund = grund_alle();
    for _ in 0..2 {
        grund.push_str("      <VBez><Art>1</Art></VBez>\n      <Kammerbeitrag><Wert>10.00</Wert></Kammerbeitrag>\n");
    }
    for _ in 0..12 {
        grund.push_str("      <ErmStVBezMKalJahr><Wert>5.00</Wert><Jahr>2025</Jahr></ErmStVBezMKalJahr>\n");
    }
    let xml = beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 12, &grund);
    assert_eq!(xml.matches("<ELStAM>").count(), 12, "KONTROLLE: die Fixtur trägt zwölf ELStAM");
    let werte = beleg_xml_zu_werten(&xml).unwrap();
    assert_eq!(werte, beleg_xml_zu_werten(&beleg_gut()).unwrap(), "wiederholte Behälter ändern die Zuordnung nicht");
}

#[test]
fn ein_beleg_in_einem_nutzdaten_behaelter_wird_gelesen() {
    let werte = beleg_xml_zu_werten(&im_behaelter(&[beleg_gut()])).unwrap();
    assert_eq!(werte.len(), 15);
}

#[test]
fn ein_beleg_ohne_betraege_ergibt_eine_leere_zuordnung() {
    let werte = beleg_xml_zu_werten(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, "")).unwrap();
    assert_eq!(werte.len(), 0, "ein Beleg ohne Betrag ist gültig und liefert nichts");
}

#[test]
fn ein_element_das_der_import_nicht_liest_bleibt_draussen() {
    // `Soli`, `AnzahlU` und `Grossbuchstaben` stehen im Schema, der Import liest sie nicht (`NICHT_GEMAPPT`): sie gehören nicht in die Zuordnung.
    let grund = format!("{}      <Soli>55.55</Soli>\n      <AnzahlU>3</AnzahlU>\n      <Grossbuchstaben>B</Grossbuchstaben>\n", grund_alle());
    let werte = beleg_xml_zu_werten(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &grund)).unwrap();
    assert_eq!(werte, beleg_xml_zu_werten(&beleg_gut()).unwrap());
    for fremd in ["Soli", "AnzahlU", "Grossbuchstaben"] {
        assert!(!werte.contains_key(fremd), "{fremd}");
    }
}

#[test]
fn ein_kommentar_mitten_im_betrag_laesst_die_zahl_ganz() {
    // Der Kommentar teilt den Text in zwei Knoten; wer nur den ersten liest, bekäme 98 statt 9876,54.
    let grund = grund_alle().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer>98<!-- Notiz -->76.54</LSteuer>");
    let werte = beleg_xml_zu_werten(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &grund)).unwrap();
    assert_eq!(werte["LSteuer"], "9876.54");
}

#[test]
fn ein_leeres_betragselement_ist_nicht_besetzt_und_keine_null() {
    let grund = grund_alle().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer></LSteuer>");
    let werte = beleg_xml_zu_werten(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &grund)).unwrap();
    assert_eq!(werte.len(), 14);
    assert!(!werte.contains_key("LSteuer"));
}

#[test]
fn ein_zweiter_beleg_wird_abgelehnt_nicht_summiert_und_nicht_der_erste_genommen() {
    let zwei = im_behaelter(&[beleg_gut(), beleg_gut()]);
    assert_eq!(fehler(&zwei), BelegFehler::MehrereBelege(2));
    let drei = im_behaelter(&[beleg_gut(), beleg_gut(), beleg_gut()]);
    assert_eq!(fehler(&drei), BelegFehler::MehrereBelege(3));
    // Auch ein zweiter Beleg mit anderer Version zählt: die Zahl der Belege kommt vor der Prüfung der Version.
    let gemischt = im_behaelter(&[beleg_gut(), beleg_mit("http://finkonsens.de/elster/elstervast/vastlstb/v202401", Some("202401"), "2024", 1, "")]);
    assert_eq!(fehler(&gemischt), BelegFehler::MehrereBelege(2));
}

#[test]
fn eine_datei_ohne_beleg_wird_abgelehnt() {
    assert_eq!(fehler("<?xml version=\"1.0\"?><Nutzdaten/>"), BelegFehler::KeinBeleg);
    assert_eq!(fehler("<Nutzdaten><Anderes/></Nutzdaten>"), BelegFehler::KeinBeleg);
}

#[test]
fn ein_wiederholtes_blatt_wird_abgelehnt_auch_bei_gleichem_wert() {
    // Das Schema erlaubt `LSteuer` einmal. Zwei Zeilen: ein flaches Wörterbuch hielte stumm die letzte.
    let zweimal = format!("{}      <LSteuer>1.00</LSteuer>\n", grund_alle());
    let xml = beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &zweimal);
    assert_eq!(fehler(&xml), BelegFehler::WiederholtesBlatt("LSteuer".into()));
    let gleich = format!("{}      <LSteuer>9876.54</LSteuer>\n", grund_alle());
    assert_eq!(fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &gleich)), BelegFehler::WiederholtesBlatt("LSteuer".into()));
    // Ein Element, das der Import nicht liest, gehört zum selben Schema: auch das ist ein ungültiger Beleg.
    let anzahl = format!("{}      <AnzahlU>1</AnzahlU>\n      <AnzahlU>2</AnzahlU>\n", grund_alle());
    assert_eq!(fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &anzahl)), BelegFehler::WiederholtesBlatt("AnzahlU".into()));
}

#[test]
fn ein_zweiter_besteuerungsgrund_wird_abgelehnt() {
    let xml = beleg_gut().replace("</VaSt_LStB>", "  <Besteuerungsgrund>\n      <LSteuer>1.00</LSteuer>\n  </Besteuerungsgrund>\n</VaSt_LStB>");
    assert_eq!(fehler(&xml), BelegFehler::WiederholtesBlatt("Besteuerungsgrund".into()));
}

#[test]
fn eine_falsche_version_wird_abgelehnt() {
    let alt_ns = "http://finkonsens.de/elster/elstervast/vastlstb/v202401";
    // Schema 2024 durchgehend.
    assert_eq!(fehler(&beleg_mit(alt_ns, Some("202401"), "2024", 1, &grund_alle())), BelegFehler::FalscheVersion("202401".into()));
    // Namensraum 2025, Attribut 2024.
    assert_eq!(fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202401"), "2025", 1, &grund_alle())), BelegFehler::FalscheVersion("202401".into()));
    // Attribut 2025, Namensraum 2024: der Fehler nennt den Namensraum, nicht das (richtige) Attribut.
    assert_eq!(fehler(&beleg_mit(alt_ns, Some("202501"), "2025", 1, &grund_alle())), BelegFehler::FalscheVersion("v202401".into()));
    // Attribut fehlt (im Schema Pflicht).
    assert_eq!(fehler(&beleg_mit(BELEG_NAMENSRAUM, None, "2025", 1, &grund_alle())), BelegFehler::FalscheVersion("-".into()));
    // Ohne Namensraum.
    assert_eq!(fehler(&beleg_mit("", Some("202501"), "2025", 1, &grund_alle())), BelegFehler::FalscheVersion("-".into()));
}

#[test]
fn ein_beleg_ganz_ohne_namensraum_wird_abgelehnt() {
    // `xmlns=""` (oben) und gar kein `xmlns` sind zwei Lagen: der Leser sieht für das eine einen leeren, für das andere keinen Namensraum.
    let xml = beleg_gut().replace(&format!(" xmlns=\"{BELEG_NAMENSRAUM}\""), "");
    assert!(!xml.contains("xmlns"), "KONTROLLE: die Fixtur trägt keinen Namensraum mehr");
    assert_eq!(fehler(&xml), BelegFehler::FalscheVersion("-".into()));
}

#[test]
fn ein_falsches_jahr_wird_abgelehnt() {
    assert_eq!(fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2024", 1, &grund_alle())), BelegFehler::FalschesJahr("2024".into()));
    assert_eq!(fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2026", 1, &grund_alle())), BelegFehler::FalschesJahr("2026".into()));
}

#[test]
fn kaputtes_xml_wird_abgelehnt() {
    let gut = beleg_gut();
    let abgeschnitten = &gut[..gut.len() / 2];
    for roh in ["", "kein XML", "<VaSt_LStB", "<a><b></a></b>", abgeschnitten] {
        assert_eq!(fehler(roh), BelegFehler::Kaputt, "{roh:.40}");
    }
    // Eine DTD mit Entität (Angriff über den Rumpf) wird abgewiesen, nicht aufgelöst.
    let dtd = format!("<?xml version=\"1.0\"?>\n<!DOCTYPE VaSt_LStB [<!ENTITY a \"1\">]>\n{}", beleg_gut().split_once("?>\n").unwrap().1);
    assert_eq!(fehler(&dtd), BelegFehler::Kaputt);
}

#[test]
fn ein_nicht_lesbarer_betrag_nennt_das_element_und_nicht_den_wert() {
    let text = grund_alle().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer>77zz</LSteuer>");
    let e = fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &text));
    assert_eq!(e, BelegFehler::BetragNichtLesbar("LSteuer".into()));
    assert!(!e.to_string().contains("77zz"), "{e}");
    // Ein Behälter statt eines Betrags.
    let behaelter = grund_alle().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer><Wert>9876.54</Wert></LSteuer>");
    assert_eq!(
        fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &behaelter)),
        BelegFehler::BetragNichtLesbar("LSteuer".into())
    );
    // Außerhalb von i64 in Cent.
    let gross = grund_alle().replace("<BruttoArbLohn>61234.56</BruttoArbLohn>", "<BruttoArbLohn>99999999999999999999.99</BruttoArbLohn>");
    assert_eq!(
        fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &gross)),
        BelegFehler::BetragAusserhalb("BruttoArbLohn".into())
    );
}

#[test]
fn kein_fehlertext_nennt_steuer_id_namen_oder_einen_wert_der_datei() {
    let alt_ns = "http://finkonsens.de/elster/elstervast/vastlstb/v202401";
    let zweimal = format!("{}      <LSteuer>4711.08</LSteuer>\n", grund_alle());
    let proben = [
        im_behaelter(&[beleg_gut(), beleg_gut()]),
        beleg_mit(alt_ns, Some("202401"), "2025", 1, &grund_alle()),
        beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2024", 1, &grund_alle()),
        beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &zweimal),
        beleg_gut().replace("<LSteuer>9876.54</LSteuer>", "<LSteuer>Mustermann</LSteuer>"),
        beleg_gut()[..200].to_owned(),
    ];
    for xml in &proben {
        let e = fehler(xml);
        let text = format!("{e} {e:?}");
        for geheim in [IDNR, NAME, VORNAME, "Erfundene Firma", "4711.08", "9876.54"] {
            assert!(!text.contains(geheim), "der Fehler {} nennt {geheim}: {text}", e.code());
        }
    }
}

#[test]
fn ein_langer_oder_fremder_name_im_fehler_wird_gekuerzt() {
    // Ein Angreifer wählt den Namen des wiederholten Elements; der Fehler gibt höchstens einen kurzen, harmlosen Namen zurück.
    let lang = "A".repeat(500);
    let grund = format!("      <{lang}>1</{lang}>\n      <{lang}>2</{lang}>\n");
    let e = fehler(&beleg_mit(BELEG_NAMENSRAUM, Some("202501"), "2025", 1, &grund));
    assert!(matches!(e, BelegFehler::WiederholtesBlatt(_)), "{e:?}");
    assert!(e.to_string().len() < 300, "{} Zeichen", e.to_string().len());
    let version = beleg_mit(BELEG_NAMENSRAUM, Some(&"9".repeat(500)), "2025", 1, "");
    assert!(fehler(&version).to_string().len() < 300);
}

#[test]
fn jeder_fehler_hat_einen_eigenen_stabilen_namen() {
    let alle = [
        BelegFehler::Kaputt,
        BelegFehler::KeinBeleg,
        BelegFehler::MehrereBelege(2),
        BelegFehler::FalscheVersion("x".into()),
        BelegFehler::FalschesJahr("x".into()),
        BelegFehler::WiederholtesBlatt("x".into()),
        BelegFehler::BetragNichtLesbar("x".into()),
        BelegFehler::BetragAusserhalb("x".into()),
    ];
    let namen: std::collections::BTreeSet<&str> = alle.iter().map(BelegFehler::code).collect();
    assert_eq!(namen.len(), alle.len(), "{namen:?}");
    assert!(namen.iter().all(|n| n.starts_with("beleg_")), "{namen:?}");
}
