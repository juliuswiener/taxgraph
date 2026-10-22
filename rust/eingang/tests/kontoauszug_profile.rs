//! Bankimport S0: Der CSV-Leser findet die Kopfzeile hinter dem Vorspann, erkennt den Trenner und kennt die
//! Spaltennamen der Banken, die vorher still 0 Buchungen lieferten.
//!
//! HERKUNFT DER DATEIEN: Alle Dateien sind ERFUNDEN (IBAN `DE89 3704 0044 0532 0130 00` ist ein Standardbeispiel).
//! Kopf- und Vorspannzeilen folgen den Fremdparsern und Testdateien aus Vault `research/bankimport-konzept-recherche-2026-10-22`
//! (Fork `bank-csv`, Abschnitt 2) -- KEIN Bankdokument bestaetigt sie. Eine echte Datei kann anders heissen; ein
//! gruener Test hier belegt nur, dass der Leser diese nachgebauten Formen liest, nicht dass er eine Bank liest.
//! Jede Datei hat zwei Ausgaben: 480,00 an einen Maler und 50,00 Spende.
//! Gegenprobe: Sparkasse, Commerzbank und Volksbank neu lasen schon vor S0 2 von 2 und tun es weiter.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use eingang::kontoauszug::{parse_csv, Transaktion};

const MALER: &str = "Maler Müller GmbH Rechnung 123 Malerarbeiten";
const SPENDE: &str = "Spende Rotes Kreuz";

fn dkb_neu_ohne_vorspann() -> String {
    format!(
        "\"Buchungsdatum\";\"Wertstellung\";\"Status\";\"Zahlungspflichtige*r\";\"Zahlungsempfänger*in\";\"Verwendungszweck\";\"Umsatztyp\";\"IBAN\";\"Betrag (€)\";\"Gläubiger-ID\";\"Mandatsreferenz\";\"Kundenreferenz\"\n\
\"15.03.25\";\"15.03.25\";\"Gebucht\";\"Max Muster\";\"Maler Müller GmbH\";\"{MALER}\";\"Ausgang\";\"DE02120300000000202051\";\"-480,00\";\"\";\"\";\"\"\n\
\"20.03.25\";\"20.03.25\";\"Gebucht\";\"Max Muster\";\"Rotes Kreuz\";\"{SPENDE}\";\"Ausgang\";\"DE02120300000000202051\";\"-50,00\";\"\";\"\";\"\"\n"
    )
}

fn dkb_neu() -> String {
    format!(
        "\"Girokonto\";\"DE89370400440532013000\"\n\n\"Kontostand vom 31.12.2025:\";\"5.000,01 EUR\"\n\n{}",
        dkb_neu_ohne_vorspann()
    )
}

fn dkb_alt() -> String {
    format!(
        "\"Kontonummer:\";\"DE89370400440532013000 / Girokonto\";\n\"Von:\";\"01.01.2025\";\n\"Bis:\";\"31.12.2025\";\n\"Kontostand vom 31.12.2025:\";\"5.000,01 EUR\";\n\n\
\"Buchungstag\";\"Wertstellung\";\"Buchungstext\";\"Auftraggeber / Begünstigter\";\"Verwendungszweck\";\"Kontonummer\";\"BLZ\";\"Betrag (EUR)\";\"Gläubiger-ID\";\"Mandatsreferenz\";\"Kundenreferenz\";\n\
\"15.03.2025\";\"15.03.2025\";\"Lastschrift\";\"Maler Müller GmbH\";\"{MALER}\";\"DE02120300000000202051\";\"12030000\";\"-480,00\";\"\";\"\";\"\";\n\
\"20.03.2025\";\"20.03.2025\";\"Lastschrift\";\"Rotes Kreuz\";\"{SPENDE}\";\"DE02120300000000202051\";\"12030000\";\"-50,00\";\"\";\"\";\"\";\n"
    )
}

fn ing_ohne_vorspann() -> String {
    format!(
        "Buchung;Valuta;Auftraggeber/Empfänger;Buchungstext;Verwendungszweck;Saldo;Währung;Betrag;Währung\n\
15.03.2025;15.03.2025;Maler Müller GmbH;Lastschrift;{MALER};4.519,99;EUR;-480,00;EUR\n\
20.03.2025;20.03.2025;Rotes Kreuz;Lastschrift;{SPENDE};4.469,99;EUR;-50,00;EUR\n"
    )
}

fn ing() -> String {
    format!(
        "Umsatzanzeige;Datei erstellt am: 22.10.2026 10:00\n;Letztes Update: aktuell\n\nIBAN;DE89 3704 0044 0532 0130 00\nKontoname;Girokonto\nBank;ING\nKunde;Max Muster\nZeitraum;01.01.2025 - 31.12.2025\nSaldo;5.000,01;EUR\n\nSortierung;Datum absteigend\n\nIn der CSV-Datei finden Sie alle bereits gebuchten Umsätze.\n\n{}",
        ing_ohne_vorspann()
    )
}

fn comdirect() -> String {
    format!(
        "\"Umsätze Girokonto\";\"Zeitraum: 30 Tage\";\n\"Neuer Kontostand\";\"1.111,11 EUR\";\n\n\
\"Buchungstag\";\"Wertstellung (Valuta)\";\"Vorgang\";\"Buchungstext\";\"Umsatz in EUR\";\n\
\"15.03.2025\";\"15.03.2025\";\"Lastschrift / Belastung\";\"Auftraggeber: Maler Müller GmbH Buchungstext: {MALER} Ref. XYZ\";\"-480,00\";\n\
\"20.03.2025\";\"20.03.2025\";\"Lastschrift / Belastung\";\"Auftraggeber: Rotes Kreuz Buchungstext: {SPENDE} Ref. ABC\";\"-50,00\";\n"
    )
}

fn n26() -> String {
    format!(
        "\"Datum\",\"Empfänger\",\"Kontonummer\",\"Transaktionstyp\",\"Verwendungszweck\",\"Kategorie\",\"Betrag (EUR)\",\"Betrag (Fremdwährung)\",\"Fremdwährung\",\"Wechselkurs\"\n\
\"2025-03-15\",\"Maler Müller GmbH\",\"DE02120300000000202051\",\"Ausgehende Überweisung\",\"{MALER}\",\"\",\"-480.00\",\"\",\"\",\"\"\n\
\"2025-03-20\",\"Rotes Kreuz\",\"DE02120300000000202051\",\"Ausgehende Überweisung\",\"{SPENDE}\",\"\",\"-50.00\",\"\",\"\",\"\"\n"
    )
}

fn revolut() -> String {
    "Type,Product,Started Date,Completed Date,Description,Amount,Fee,Currency,State,Balance\n\
CARD_PAYMENT,Current,2025-03-15 10:00:00,2025-03-15 10:00:05,Maler Müller GmbH,-480.00,0.00,EUR,COMPLETED,4519.99\n\
CARD_PAYMENT,Current,2025-03-20 10:00:00,2025-03-20 10:00:05,Spende Rotes Kreuz,-50.00,0.00,EUR,COMPLETED,4469.99\n"
        .to_owned()
}

fn sparkasse() -> String {
    format!(
        "\"Auftragskonto\";\"Buchungstag\";\"Valutadatum\";\"Buchungstext\";\"Verwendungszweck\";\"Glaeubiger ID\";\"Mandatsreferenz\";\"Kundenreferenz (End-to-End)\";\"Sammlerreferenz\";\"Lastschrift Ursprungsbetrag\";\"Auslagenersatz Ruecklastschrift\";\"Beguenstigter/Zahlungspflichtiger\";\"Kontonummer/IBAN\";\"BIC (SWIFT-Code)\";\"Betrag\";\"Waehrung\";\"Info\"\n\
\"DE89370400440532013000\";\"15.03.25\";\"15.03.25\";\"FOLGELASTSCHRIFT\";\"{MALER}\";\"\";\"\";\"\";\"\";\"\";\"\";\"Maler Müller GmbH\";\"DE02120300000000202051\";\"COBADEFFXXX\";\"-480,00\";\"EUR\";\"Umsatz gebucht\"\n\
\"DE89370400440532013000\";\"20.03.25\";\"20.03.25\";\"ONLINE-UEBERWEISUNG\";\"{SPENDE}\";\"\";\"\";\"\";\"\";\"\";\"\";\"Rotes Kreuz\";\"DE02120300000000202051\";\"COBADEFFXXX\";\"-50,00\";\"EUR\";\"Umsatz gebucht\"\n"
    )
}

fn commerzbank() -> String {
    format!(
        "Buchungstag;Wertstellung;Umsatzart;Buchungstext;Betrag;Währung;Auftraggeberkonto;Bankleitzahl Auftraggeberkonto;IBAN Auftraggeberkonto;Kategorie\n\
15.03.2025;15.03.2025;Lastschrift;{MALER};-480,00;EUR;;;DE02120300000000202051;Sonstiges\n\
20.03.2025;20.03.2025;Überweisung;{SPENDE};-50,00;EUR;;;DE02120300000000202051;Sonstiges\n"
    )
}

fn volksbank_neu() -> String {
    format!(
        "Bezeichnung Auftragskonto;IBAN Auftragskonto;BIC Auftragskonto;Bankname Auftragskonto;Buchungstag;Valutadatum;Name Zahlungsbeteiligter;IBAN Zahlungsbeteiligter;BIC (SWIFT-Code) Zahlungsbeteiligter;Buchungstext;Verwendungszweck;Betrag;Waehrung;Saldo nach Buchung;Bemerkung;Kategorie;Steuerrelevant;Glaeubiger ID;Mandatsreferenz\n\
Giro;DE89370400440532013000;GENODEF1XXX;Testbank;15.03.2025;15.03.2025;Maler Müller GmbH;DE02120300000000202051;GENODEF1YYY;Lastschrift;{MALER};-480,00;EUR;4.519,99;;;;;\n\
Giro;DE89370400440532013000;GENODEF1XXX;Testbank;20.03.2025;20.03.2025;Rotes Kreuz;DE02120300000000202051;GENODEF1YYY;Überweisung;{SPENDE};-50,00;EUR;4.469,99;;;;;\n"
    )
}

fn volksbank_alt() -> String {
    format!(
        "Buchungstag;Valuta;Auftraggeber/Zahlungsempfänger;Umsatz;Soll/Haben;Vorgang/Verwendungszweck\n\
15.03.2025;15.03.2025;Maler Müller GmbH;480,00;S;{MALER}\n\
20.03.2025;20.03.2025;Rotes Kreuz;50,00;S;{SPENDE}\n"
    )
}

fn lies(name: &str, text: &str) -> Vec<Transaktion> {
    match parse_csv(text) {
        Ok((tx, verworfen)) => {
            assert_eq!(verworfen, 0, "{name}: verworfen");
            tx
        }
        Err(e) => panic!("{name}: {e}"),
    }
}

/// AK1: jedes Profil liefert 2 von 2 Buchungen, mit dem Betrag, dem Datum und dem Zweck der Zeile
/// (nicht 2 leere Huellen). Die Liste der Profile, die schon vor S0 lasen, ist die Gegenprobe: ein Leser,
/// der nichts liest, faellt an ihnen nicht auf, ein Test, der nur zaehlt, auch nicht.
#[test]
fn jedes_profil_liest_zwei_von_zwei_buchungen_mit_datum_und_zweck() {
    let profile = [
        ("DKB neu", dkb_neu()),
        ("DKB neu ohne Vorspann", dkb_neu_ohne_vorspann()),
        ("DKB alt", dkb_alt()),
        ("ING", ing()),
        ("ING ohne Vorspann", ing_ohne_vorspann()),
        ("comdirect", comdirect()),
        ("N26", n26()),
        ("Revolut", revolut()),
        ("Sparkasse (Gegenprobe)", sparkasse()),
        ("Commerzbank (Gegenprobe)", commerzbank()),
        ("Volksbank neu (Gegenprobe)", volksbank_neu()),
    ];
    let mut falsch = Vec::new();
    for (name, text) in profile {
        let tx = lies(name, &text);
        let betraege: Vec<i64> = tx.iter().map(|t| t.betrag).collect();
        let datum_leer = tx.iter().filter(|t| t.datum.as_str() == Some("")).count();
        let zwecke_ok = tx.len() == 2
            && tx[0].verwendungszweck.contains("Maler")
            && tx[1].verwendungszweck.contains("Spende");
        if betraege != [-48_000, -5_000] || datum_leer != 0 || !zwecke_ok {
            falsch.push(format!(
                "{name}: Betraege {betraege:?}, Datum leer {datum_leer}, Zwecke ok {zwecke_ok}"
            ));
        }
    }
    assert!(falsch.is_empty(), "S0 ROT: {falsch:#?}");
}

/// AK2, Alias `vorgang/verwendungszweck`: Die alte Volksbank-Datei (Betrag ohne Vorzeichen, Soll/Haben) liest den Zweck.
/// Die Beträge bleiben ohne Minus, also Einnahmen -- das ist die dokumentierte Grenze (die Route sagt es im Hinweis).
#[test]
fn volksbank_alt_liest_den_zweck_und_zaehlt_keine_ausgabe() {
    let tx = lies("Volksbank alt", &volksbank_alt());
    assert_eq!(tx.len(), 2);
    assert!(tx[0].verwendungszweck.contains("Maler"), "{tx:?}");
    assert!(tx.iter().all(|t| t.betrag > 0), "{tx:?}");
}

/// Ein BOM am Dateianfang gehoert nicht zum ersten Spaltennamen: sonst hiesse die Datumsspalte
/// `\u{feff}"buchungsdatum"` und das Datum bliebe leer.
#[test]
fn ein_bom_am_anfang_stoert_die_erste_spalte_nicht() {
    let tx = lies(
        "DKB neu mit BOM",
        &format!("\u{feff}{}", dkb_neu_ohne_vorspann()),
    );
    assert_eq!(tx.len(), 2);
    assert_eq!(tx[0].datum.as_str(), Some("15.03.25"));
}

/// Der Trenner Tab wird erkannt (wie `;` und `,`).
#[test]
fn der_trenner_tab_wird_erkannt() {
    let tx = lies(
        "Tab",
        "Buchungstag\tBetrag\tVerwendungszweck\n15.03.2025\t-480,00\tMaler Huber\n",
    );
    assert_eq!(
        tx.iter()
            .map(|t| (t.betrag, t.verwendungszweck.as_str()))
            .collect::<Vec<_>>(),
        [(-48_000, "Maler Huber")]
    );
}

/// Die Kopfzeile steht in den ersten 30 Datensaetzen: der 30. (Nr. 29) gilt, der 31. (Nr. 30) nicht mehr.
#[test]
fn die_kopfzeile_wird_in_den_ersten_30_zeilen_gesucht() {
    let datei = |vorspann: usize| {
        let mut t = "Vorspann;x\n".repeat(vorspann);
        t.push_str("Buchungstag;Betrag;Verwendungszweck\n15.03.2025;-480,00;Maler Huber\n");
        t
    };
    assert_eq!(lies("29 Vorspannzeilen", &datei(29)).len(), 1);
    assert_eq!(lies("30 Vorspannzeilen", &datei(30)).len(), 0);
}

/// Eine Datei ohne Betragsspalte liest weiter 0 Buchungen und bricht nicht ab (die Route meldet es dann).
#[test]
fn ohne_betragsspalte_bleibt_es_bei_null_buchungen() {
    let tx = lies(
        "ohne Betrag",
        "Buchungstag;Wert;Verwendungszweck\n15.03.2025;5;Maler Huber\n",
    );
    assert!(tx.is_empty(), "{tx:?}");
}
