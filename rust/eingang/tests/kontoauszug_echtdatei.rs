//! Zwei Fehler, die eine echte Bankdatei im Kontoauszug-Import zeigte (Vault `backlog/taxgraph/bankimport-echte-bankdatei-
//! zeigt-depotblock-deckelhinweis-und-umlaut-fehler`, AK1 und AK2): die Depot-Tabelle hinter den Buchungen zaehlt als Buchungen,
//! und der Deckel der KI-Aufrufe meldet sich auch dort, wo keine KI laeuft.
//!
//! HERKUNFT DER DATEIEN: ERFUNDEN. Die echte Datei bleibt auf dem Rechner ihres Besitzers. Nachgebaut ist nur ihre Gestalt:
//! Vorspann, Kopfzeile mit fuenf Namen und einem leeren sechsten Feld, Buchungen mit sechs Feldern, Fusszeilen mit zwei bis
//! drei Feldern, danach eine zweite Tabelle mit neun Feldern, deren fuenftes Feld (die Betragsspalte der ersten) eine
//! Wertpapierkennnummer traegt, eine davon rein numerisch. Alle Namen, Betraege und Kennnummern sind frei erfunden.
#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use std::cell::Cell;
use std::fmt::Write as _;

use eingang::kontoauszug::{
    parse_csv_befund, uebernehme, Transaktion, LLM_AUFRUFE_HOECHSTZAHL,
};
use store::{BindungNachschlag, Store};

/// Die Datei, wie die Bank sie liefert, in der Schreibweise des Tests (UTF-8; die Zeichensatzfrage ist AK3).
fn girokonto_mit_depot() -> String {
    let mut s = String::new();
    s.push_str(";\r\n");
    s.push_str("\"Umsätze Girokonto\";\"Zeitraum: 180 Tage\";\r\n");
    s.push_str("\"Neuer Kontostand\";\"1.111,11 EUR\";\r\n");
    s.push_str("\r\n");
    s.push_str("\"Buchungstag\";\"Wertstellung (Valuta)\";\"Vorgang\";\"Buchungstext\";\"Umsatz in EUR\";\r\n");
    for (tag, text, betrag) in [
        ("15.03.2025", "Auftraggeber: Maler Müller GmbH Buchungstext: Malerarbeiten", "-480,00"),
        ("20.03.2025", "Auftraggeber: Rotes Kreuz Buchungstext: Spende", "-50,00"),
        ("21.03.2025", "Auftraggeber: Muster AG Buchungstext: Gehalt", "1.234,56"),
        ("22.03.2025", "Auftraggeber: Muster Markt Buchungstext: Einkauf", "-7,31"),
    ] {
        write!(s, "\"{tag}\";\"{tag}\";\"Lastschrift / Belastung\";\"{text}\";\"{betrag}\";\r\n").unwrap();
    }
    s.push_str("\r\n");
    s.push_str("\"Summe Ausgaben\";\"-537,31 EUR\";\r\n");
    s.push_str("\r\n");
    s.push_str("\"Anzahl Buchungen\";\"Zeitraum: 4 Tage\";\r\n");
    s.push_str("\r\n");
    s.push_str("\"Muster Depot\";\"Stand: 22.03.2025\";\r\n");
    s.push_str("\r\n");
    s.push_str("\"Kaufdatum\";\"Valuta\";\"Stück\";\"Bezeichnung\";\"WKN\";\"Währung\";\"Kurs\";\"Wert\";\r\n");
    // Zwoelf Zeilen; die Kennnummer in Feld 5 liegt auf der Betragsspalte der ersten Tabelle. Eine ist rein numerisch.
    for n in 0..12 {
        let wkn = if n == 11 { "123456".to_owned() } else { format!("A{n}B{n}C{n}") };
        write!(
            s,
            "\"01.0{m}.2025\";\"01.0{m}.2025\";\"{n},500\";\"Muster Fonds {n}\";\"{wkn}\";\"EUR\";\"12,34\";\"{n}1,17\";\r\n",
            m = 1 + n % 9
        )
        .unwrap();
    }
    s
}

#[test]
fn echtdatei_zweite_tabelle_zaehlt_nicht_als_buchung() {
    let (tx, verworfen, befund) = parse_csv_befund(&girokonto_mit_depot()).unwrap();
    // Die vier Buchungen der ersten Tabelle, und nichts sonst: weder die Kennnummer 123456 (als Betrag 123456,00 EUR)
    // noch die elf Kennnummern mit Buchstaben als „unlesbarer Betrag“.
    assert_eq!(
        tx.iter().map(|t| t.betrag).collect::<Vec<_>>(),
        [-48_000, -5_000, 123_456, -731],
        "Betraege der gelesenen Buchungen"
    );
    assert_eq!(verworfen, 0, "keine Zeile mit unlesbarem Betrag: die Depot-Zeilen sind keine Buchungen");
    assert_eq!(befund.kopf_nr, Some(4));
    // Die zweite Tabelle wird gezaehlt, nicht still verschluckt: ihre Kopfzeile und die zwoelf Zeilen. Die Fusszeilen mit
    // weniger Feldern sind keine fremde Tabelle (sie hatten und haben keinen Betrag).
    assert_eq!(befund.fremde_zeilen, 13);
    assert_eq!(befund.ohne_betrag, 3, "Summe Ausgaben, Anzahl Buchungen und die Depot-Ueberschrift ohne Betragsfeld");
}

/// Eine breite Zeile MITTEN in der Tabelle bleibt eine Buchung wie in Python (`DictReader` wirft das Ueberschussfeld weg).
#[test]
fn echtdatei_breiter_datensatz_mitten_im_abschnitt_bleibt_buchung() {
    // Direkt unter der Kopfzeile ...
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler;extra\n02.03.2025;-50,00;Spende\n";
    let (tx, verworfen, befund) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen, befund.fremde_zeilen), (2, 0, 0));
    // ... und nach einer gewoehnlichen Zeile, ohne Leerzeile davor.
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n02.03.2025;-50,00;Spende;extra\n03.03.2025;-7,00;Reinigung\n";
    let (tx, verworfen, befund) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen, befund.fremde_zeilen), (3, 0, 0));
}

/// Eine Leerzeile unter der Kopfzeile oder mitten in der Tabelle trennt keine Tabelle ab: die Zeilen davor und danach bleiben
/// Buchungen. Eine breite Zeile direkt NACH der ersten Leerzeile unter der Kopfzeile ist ebenfalls noch Buchung (kein
/// Abschnitt war zu Ende).
#[test]
fn echtdatei_leerzeile_trennt_nur_nach_einer_zeile_unter_der_kopfzeile() {
    let direkt = "Buchungstag;Betrag;Verwendungszweck\n\n01.03.2025;-480,00;Maler;extra\n02.03.2025;-50,00;Spende\n";
    let (tx, _, befund) = parse_csv_befund(direkt).unwrap();
    assert_eq!((tx.len(), befund.fremde_zeilen), (2, 0));
    let mitten = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n\n02.03.2025;-50,00;Spende\n";
    let (tx, _, befund) = parse_csv_befund(mitten).unwrap();
    assert_eq!((tx.len(), befund.fremde_zeilen), (2, 0));
}

/// Der fremde Abschnitt endet an der naechsten Leerzeile: eine weitere Buchungstabelle danach wird wieder gelesen.
#[test]
fn echtdatei_fremder_abschnitt_endet_an_der_leerzeile() {
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n\n\
                Kaufdatum;Valuta;Stueck;Bezeichnung\n01.02.2025;01.02.2025;1,000;Muster Fonds\n\n\
                02.03.2025;-50,00;Spende\n";
    let (tx, verworfen, befund) = parse_csv_befund(text).unwrap();
    assert_eq!(tx.iter().map(|t| t.betrag).collect::<Vec<_>>(), [-48_000, -5_000]);
    assert_eq!((verworfen, befund.fremde_zeilen), (0, 2));
}

/// Gegenprobe zu AK1: eine Datei mit uneinheitlichen, aber gueltigen Zeilen verliert keine Buchung. Die Kopfzeile hat drei
/// Namen, die Zeilen ein leeres viertes Feld (Endsemikolon): das ist kein Ueberschuss.
#[test]
fn echtdatei_leeres_endfeld_hinter_der_kopfzeile_bleibt_buchung() {
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler;\n02.03.2025;-50,00;Spende; \n";
    let (tx, verworfen, _) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen), (2, 0), "leere und blanke Endfelder zaehlen nicht als Ueberschuss");
}

/// Eine Zeile aus Leerzeichen ist eine Leerzeile und trennt den Abschnitt (die Bank-Datei kennt beides). Ohne `strip` bliebe sie
/// ein Datensatz, und die zweite Tabelle danach wuerde als Buchungszeilen gelesen (Mutante E01 von main ueberlebte).
#[test]
fn echtdatei_zeile_aus_leerzeichen_trennt_wie_eine_leerzeile() {
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n   \n\
                Kaufdatum;Valuta;Stueck;Bezeichnung\n01.02.2025;01.02.2025;1,000;Muster Fonds\n";
    let (tx, verworfen, befund) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen, befund.fremde_zeilen), (1, 0, 2));
}

/// Ein blankes Feld jenseits der Kopfzeile, am Anfang eines Abschnitts, ist kein Ueberschuss: die Zeile und ihr Abschnitt bleiben
/// Buchungen (Mutante E02 von main ueberlebte; die Gegenprobe davor prueft das Endfeld nur ohne Leerzeile davor).
#[test]
fn echtdatei_blankes_endfeld_am_abschnittsanfang_ist_kein_ueberschuss() {
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler\n\n\
                02.03.2025;-50,00;Spende; \n03.03.2025;-7,00;Reinigung\n";
    let (tx, verworfen, befund) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen, befund.fremde_zeilen), (3, 0, 0));
}

/// Die Breite der Kopfzeile zaehlt auch ihr leeres letztes Feld (Endsemikolon, so liefert es die Bank): ein nicht leeres Feld an
/// dieser Stelle liegt NICHT jenseits der Kopfzeile (Mutante E03 von main ueberlebte).
#[test]
fn echtdatei_leeres_letztes_feld_der_kopfzeile_zaehlt_zur_breite() {
    let text = "Buchungstag;Betrag;Verwendungszweck;\n01.03.2025;-480,00;Maler;\n\n02.03.2025;-50,00;Spende;Bemerkung\n";
    let (tx, verworfen, befund) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen, befund.fremde_zeilen), (2, 0, 0));
}

/// Gegenprobe zu AK1: eine Zeile mit WENIGER Feldern bleibt, was sie war: ohne Betragsfeld „ohne Betrag“, mit Betrag eine Buchung.
#[test]
fn echtdatei_zeile_mit_weniger_feldern_bleibt_wie_zuvor() {
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00\nSumme\n";
    let (tx, verworfen, befund) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen, befund.ohne_betrag), (1, 0, 1));
}

fn ausgaben_ohne_stichwort(n: i64) -> Vec<Transaktion> {
    (0..n)
        .map(|i| Transaktion {
            datum: serde_json::json!("01.03.2025"),
            betrag: -100 - i,
            verwendungszweck: format!("Einkauf {i}"),
        })
        .collect()
}

/// AK2, die Seite des Crates: der Deckel der Aufrufe (50) gilt, wenn ein Klassifikator uebergeben ist, und nur dann. Ohne
/// Klassifikator ist jede Buchung ohne Stichwort „ohne Stichwort“ und nie „uebersprungen“. Die Route uebergibt ohne
/// KI-Konfiguration keinen Klassifikator (Test `echtdatei_ohne_ki_kein_deckelhinweis` in `kontoauszug_naht.rs`).
#[test]
fn echtdatei_deckel_gilt_nur_mit_klassifikator() {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let buchungen = ausgaben_ohne_stichwort(60);

    let aufrufe = Cell::new(0_usize);
    let kein_treffer = |_: &llm::pii::Maskiert, _: i64| {
        aufrufe.set(aufrufe.get() + 1);
        None
    };
    let mut store = Store::leer(2025, None);
    let mit = uebernehme(&mut store, &buchungen, nachschlag, Some(&kein_treffer), None, None).unwrap();
    assert_eq!(LLM_AUFRUFE_HOECHSTZAHL, 50);
    assert_eq!(aufrufe.get(), 50, "der Klassifikator wird genau 50-mal gefragt");
    assert_eq!((mit.ohne_stichwort, mit.llm_uebersprungen), (50, 10));

    let mut store = Store::leer(2025, None);
    let ohne = uebernehme(&mut store, &buchungen, nachschlag, None, None, None).unwrap();
    assert_eq!((ohne.ohne_stichwort, ohne.llm_uebersprungen), (60, 0));
}
