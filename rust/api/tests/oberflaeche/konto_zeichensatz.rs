//! Zeichensatz der Kontoauszug-Datei (Bankimport, echte Bankdatei, AK3; Vault `backlog/taxgraph/bankimport-echte-bankdatei-zeigt-
//! depotblock-deckelhinweis-und-umlaut-fehler`). Eine Bank liefert ihre CSV in `windows-1252` (cp1252): ein „ä“ ist das eine Byte
//! `0xE4`. Las die Seite die Datei mit `datei.text()`, dekodierte der Browser sie als UTF-8, und jedes solche Byte wurde zu U+FFFD
//! („Sanit�r“). Vier Stichwoerter des Dienstes tragen einen Umlaut (`sanitär`, `gebäudereinig`, `rürup`, `rürup-rente`): die
//! Buchung bekam keinen Vorschlag, ohne dass die Seite es sagte.
//!
//! Hier steht, was die Seite aus den Bytes macht: eine cp1252-Datei liefert den Vorschlag, eine UTF-8-Datei (mit und ohne BOM)
//! bleibt unveraendert, und ein Byte, das in cp1252 nicht belegt ist, bricht die Seite nicht.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung. Eine Buchung, -50,00 EUR, Stichwort „sanitär“ (Handwerker): Saldo 50,00 EUR = 5_000 Cent
//! in `hh_handwerker_betrag`. Die Datei ist erfunden.
//!
//! Grenzen: Chromium allein. Die Datei kommt per `DataTransfer` ins Eingabefeld (wie die Dateiwahl sie liefert); der
//! Dateiwahl-Dialog des Betriebssystems laeuft nicht.

use serde_json::{json, Value};

use crate::dienst::Start;
use crate::konto_ersetzen::{warte_status, zum_kontoauszug};
use crate::seite::Umgebung;

const FELD: &str = "hh_handwerker_betrag";
const ZWECK: &str = "Sanitär Meier Rechnung";

/// Die Bytes, die eine Bank im Zeichensatz `windows-1252` schreibt. Nur die Zeichen, die diese Tests brauchen.
fn cp1252(text: &str) -> Vec<u8> {
    text.chars()
        .map(|c| match c {
            'ä' => 0xE4,
            'ö' => 0xF6,
            'ü' => 0xFC,
            'ß' => 0xDF,
            '€' => 0x80,
            c if c.is_ascii() => c as u8,
            c => panic!("{c:?} fehlt in cp1252()"),
        })
        .collect()
}

fn auszug(zweck_bytes: &[u8]) -> Vec<u8> {
    let mut b = b"datum;betrag;verwendungszweck\n01.03.2025;-50,00;".to_vec();
    b.extend_from_slice(zweck_bytes);
    b.push(b'\n');
    b
}

/// Der Nutzer waehlt eine Datei `name` mit diesen Bytes. Das Feld ist waehrend eines Uploads gesperrt; der Nutzer wartet.
fn waehle_bytes(u: &Umgebung, name: &str, bytes: &[u8]) {
    u.s.warte_bis("!document.getElementById('konto-file').disabled", 15, "das Eingabefeld des Kontoauszugs ist frei");
    u.s.skript(
        "const dt = new DataTransfer(); dt.items.add(new File([new Uint8Array(arguments[1])], arguments[0])); \
         const e = document.getElementById('konto-file'); e.files = dt.files; e.dispatchEvent(new Event('change', {bubbles: true}));",
        &[json!(name), json!(bytes)],
    );
}

/// Laedt `bytes` als CSV-Datei in einen neuen Fall und liefert (Wert des Vorschlags in Cent, gespeicherter Zweck).
fn lade(u: &Umgebung, bytes: &[u8]) -> (Value, Value) {
    lade_datei(u, "auszug.csv", bytes)
}

/// Wie `lade`, mit dem Dateinamen des Nutzers (die Endung waehlt das Format).
fn lade_datei(u: &Umgebung, name: &str, bytes: &[u8]) -> (Value, Value) {
    let fall = zum_kontoauszug(u);
    waehle_bytes(u, name, bytes);
    let status = warte_status(u, "1 von 1 Buchung(en) als Vorschlag erfasst");
    let ev = u.dienst.ereignisse(&fall, FELD);
    assert_eq!(ev.len(), 1, "ein Vorschlag im Feld {FELD}; Status {status:?}");
    (ev[0]["wert"].clone(), ev[0]["signal"]["signal_1"]["posten"][0]["zweck_maskiert"].clone())
}

#[test]
fn konto_zeichensatz_cp1252_datei_liefert_den_vorschlag() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let (wert, zweck) = lade(&u, &auszug(&cp1252(ZWECK)));
    assert_eq!(wert, json!(5_000), "Saldo 50,00 EUR");
    assert_eq!(zweck, json!(ZWECK), "das ä kommt als ä an, nicht als U+FFFD");
}

#[test]
fn konto_zeichensatz_utf8_datei_bleibt_unveraendert() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let (wert, zweck) = lade(&u, &auszug(ZWECK.as_bytes()));
    assert_eq!(wert, json!(5_000), "Saldo 50,00 EUR");
    assert_eq!(zweck, json!(ZWECK));
}

#[test]
fn konto_zeichensatz_utf8_datei_mit_bom_bleibt_unveraendert() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend(auszug(ZWECK.as_bytes()));
    let (wert, zweck) = lade(&u, &bytes);
    assert_eq!(wert, json!(5_000), "Saldo 50,00 EUR; der BOM steht nicht im Spaltennamen");
    assert_eq!(zweck, json!(ZWECK));
}

/// Das Eurozeichen ist in cp1252 das Byte `0x80`; in ISO-8859-15 und ISO-8859-1 (den Zeichensaetzen, die man fuer „westlich“
/// haelt) steht dort ein Steuerzeichen. Der Rueckfall ist also genau `windows-1252` (Mutante U02 von main ueberlebte: die
/// Umlaute allein trennen die Zeichensaetze nicht).
#[test]
fn konto_zeichensatz_eurozeichen_kommt_als_euro_an() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let (wert, zweck) = lade(&u, &auszug(&cp1252("Maler Huber € Anzahlung")));
    assert_eq!(wert, json!(5_000), "Saldo 50,00 EUR");
    assert_eq!(zweck, json!("Maler Huber € Anzahlung"), "0x80 ist in cp1252 das Eurozeichen");
}

/// Die JSON-Datei geht denselben Weg wie die CSV-Datei (`_dateiAlsText`): eine JSON-Datei in cp1252 liefert den Vorschlag
/// (Mutante U03 von main ueberlebte: `datei.text()` fuer JSON haette ein U+FFFD im Zweck gelassen). Betrag in Cent.
#[test]
fn konto_zeichensatz_json_datei_in_cp1252_liefert_den_vorschlag() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let json_text = format!(r#"[{{"datum": "01.03.2025", "betrag": -5000, "verwendungszweck": "{ZWECK}"}}]"#);
    let (wert, zweck) = lade_datei(&u, "auszug.json", &cp1252(&json_text));
    assert_eq!(wert, json!(5_000), "Saldo 50,00 EUR");
    assert_eq!(zweck, json!(ZWECK), "das ä kommt als ä an, nicht als U+FFFD");
}

/// Das Byte `0x81` ist in cp1252 nicht belegt und kein UTF-8. Die Seite darf daran nicht scheitern: `TextDecoder("windows-1252")`
/// macht daraus das Steuerzeichen U+0081 (WHATWG-Index; hier im Browser gemessen), nie einen Fehler. Der Vorschlag entsteht, weil
/// „maler“ davor steht.
#[test]
fn konto_zeichensatz_unbelegtes_byte_bricht_die_seite_nicht() {
    let _sperre = crate::browser::seriell();
    let u = umgebung_oder_ende!(Start::default(), 1280, 900);
    let mut zweck = b"Maler Huber ".to_vec();
    zweck.push(0x81);
    let (wert, gespeichert) = lade(&u, &auszug(&zweck));
    assert_eq!(wert, json!(5_000), "Saldo 50,00 EUR");
    assert_eq!(gespeichert, json!("Maler Huber \u{81}"), "das unbelegte Byte kommt als U+0081 an");
}
