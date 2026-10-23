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
        s.push_str(&format!("\"{tag}\";\"{tag}\";\"Lastschrift / Belastung\";\"{text}\";\"{betrag}\";\r\n"));
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
        s.push_str(&format!(
            "\"01.0{m}.2025\";\"01.0{m}.2025\";\"{n},500\";\"Muster Fonds {n}\";\"{wkn}\";\"EUR\";\"12,34\";\"{n}1,17\";\r\n",
            m = 1 + n % 9
        ));
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
        [-480_00, -50_00, 1_234_56, -7_31],
        "Betraege der gelesenen Buchungen"
    );
    assert_eq!(verworfen, 0, "keine Zeile mit unlesbarem Betrag: die Depot-Zeilen sind keine Buchungen");
    assert_eq!(befund.kopf_nr, Some(4));
}

/// Gegenprobe zu AK1: eine Datei mit uneinheitlichen, aber gueltigen Zeilen verliert keine Buchung. Die Kopfzeile hat drei
/// Namen, die Zeilen ein leeres viertes Feld (Endsemikolon): das ist kein Ueberschuss.
#[test]
fn echtdatei_leeres_endfeld_hinter_der_kopfzeile_bleibt_buchung() {
    let text = "Buchungstag;Betrag;Verwendungszweck\n01.03.2025;-480,00;Maler;\n02.03.2025;-50,00;Spende; \n";
    let (tx, verworfen, _) = parse_csv_befund(text).unwrap();
    assert_eq!((tx.len(), verworfen), (2, 0), "leere und blanke Endfelder zaehlen nicht als Ueberschuss");
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
