//! Entscheidungen von `llm::pii` gegen die Python-Fassung (`produkt/haut/pii_filter.py` und
//! `produkt/eingang/kontoauszug_writer.py:maskiere`): jede Grenze der Muster einmal von beiden Seiten.
//!
//! Die Erwartungen sind NICHT aus Rust abgeleitet. Jede Zeile ist ein Lauf der Python-Funktionen
//! (`filtere` -> gefiltert + Kategorien, `maskiere` -> maskiert; `ist_besondere_kategorie`); die
//! Eingaben stehen hier, die Ausgaben stammen aus Python. Wer ein Muster in `pii.rs` aendert,
//! erzeugt die Tabellen neu (Eingabe-Listen und Orakel-Aufruf liegen ausserhalb des Baums).
//! Die Fehlerpfade (Backtracking-Grenze) hat Python nicht: dort gilt die Zusage der Moduldoku.
use llm::pii::{filtere, ist_besondere_kategorie, maskiere};

/// (Text, gefiltert, Kategorien, maskiert)
type Fall = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static str,
);

fn pruefe(gruppe: &str, faelle: &[Fall]) {
    for (text, gefiltert, kategorien, maskiert) in faelle {
        let (g, k) = filtere(text);
        assert_eq!(g.as_str(), *gefiltert, "{gruppe}: filtere({text:?})");
        assert_eq!(k, *kategorien, "{gruppe}: Kategorien von {text:?}");
        assert_eq!(
            maskiere(text).as_str(),
            *maskiert,
            "{gruppe}: maskiere({text:?})"
        );
    }
}

/// IBAN, zusammengeschrieben: Laenge des Rumpfs, Laenderkuerzel, Pruefziffern.
#[rustfmt::skip]
#[test]
fn iban_kompakt_wie_python() {
    pruefe(
        "iban_kompakt",
        &[
            ("IBAN DE89A4A4A4A4 Miete", "IBAN DE89A4A4A4A4 Miete", &[], "IBAN DE89A4A4A4A4 Miete"),
            ("IBAN DE89A4A4A4A4A Miete", "IBAN DE89A4A4A4A4A Miete", &[], "IBAN DE89A4A4A4A4A Miete"),
            ("IBAN DE89A4A4A4A4A4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89A4A4A4A4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4A Miete", "IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4A Miete", &[], "IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4A Miete"),
            ("IBAN de89A4A4A4A4A4 x", "IBAN [PII] x", &["iban"], "IBAN de89**** x"),
            ("IBAN De89A4A4A4A4A4 x", "IBAN [PII] x", &["iban"], "IBAN De89**** x"),
            ("IBAN dE89A4A4A4A4A4 x", "IBAN [PII] x", &["iban"], "IBAN dE89**** x"),
            ("NL91ABNA0417164300", "[PII]", &["iban"], "NL91****"),
            ("IBAN DE89370400440532013000 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE8A4A4A4A4A4A x", "IBAN DE8A4A4A4A4A4A x", &[], "IBAN DE8A4A4A4A4A4A x"),
            ("IBAN D89A4A4A4A4A4A x", "IBAN D89A4A4A4A4A4A x", &[], "IBAN D89A4A4A4A4A4A x"),
            ("IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4x", "IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4x", &[], "IBAN DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4x"),
            ("DE89A4A4A4A4A4A4A4A4A4A4A4A4A4A4A4", "[PII]", &["iban"], "DE89****"),
            ("XX00A4A4A4A4A4", "[PII]", &["iban"], "XX00****"),
        ],
    );
}

/// IBAN in Gruppen: Gruppenzahl, Restgruppe, Trenner.
#[rustfmt::skip]
#[test]
fn iban_gruppen_wie_python() {
    pruefe(
        "iban_gruppen",
        &[
            ("IBAN DE89 4A4A Miete", "IBAN DE89 4A4A Miete", &[], "IBAN DE89 4A4A Miete"),
            ("IBAN DE89 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A Miete", "IBAN [PII] 4A4A Miete", &["iban"], "IBAN DE89**** 4A4A Miete"),
            ("IBAN DE89 4A4A 4A4A 4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A4 Miete", "IBAN [PII] 4A4A4 Miete", &["iban"], "IBAN DE89**** 4A4A4 Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4 Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A 4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89-4A4A-4A4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89 4A4A-4A4A 4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN DE89/4A4A/4A4A Miete", "IBAN DE89/4A4A/4A4A Miete", &[], "IBAN DE89/4A4A/4A4A Miete"),
            ("IBAN DE89-4A4A-4A4A-4A Miete", "IBAN [PII] Miete", &["iban"], "IBAN DE89**** Miete"),
            ("IBAN de89 4A4A 4A4A x", "IBAN [PII]", &["iban"], "IBAN de89****"),
            ("IBAN DE89  4A4A 4A4A x", "IBAN DE89  4A4A 4A4A x", &[], "IBAN DE89  4A4A 4A4A x"),
            ("DE89 3704 0044 0532 0130 00 Miete", "[PII] Miete", &["iban"], "DE89**** Miete"),
            ("de89 3704 0044 0532 0130 00 Miete", "[PII] Miete", &["iban"], "de89**** Miete"),
            ("DE89-3704-0044-0532-0130-00 Miete", "[PII] Miete", &["iban"], "DE89**** Miete"),
        ],
    );
}

/// Steuernummer/IdNr (11 bis 13 Ziffern, Trenner) und Kontonummer (ab 8 Ziffern).
#[rustfmt::skip]
#[test]
fn steuer_id_und_konto_wie_python() {
    pruefe(
        "steuer_id_und_konto",
        &[
            ("StNr 1234567 ok", "StNr 1234567 ok", &[], "StNr 1234567 ok"),
            ("StNr 12345678 ok", "StNr [PII] ok", &["kontonummer"], "StNr 12**** ok"),
            ("StNr 123456789 ok", "StNr [PII] ok", &["kontonummer"], "StNr 12**** ok"),
            ("StNr 1234567890 ok", "StNr [PII] ok", &["kontonummer"], "StNr 12**** ok"),
            ("StNr 12345678901 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 123456789012 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 1234567890123 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 12345678901234 ok", "StNr [PII] ok", &["kontonummer"], "StNr 12**** ok"),
            ("StNr 1234567890123456 ok", "StNr [PII] ok", &["kontonummer"], "StNr 12**** ok"),
            ("StNr 12 345 678 901 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 181/815/08155 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 181/815/081 55 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 21/815/08150/3 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 12  345 678 901 ok", "StNr 12  345 678 901 ok", &[], "StNr 12  345 678 901 ok"),
            ("StNr 123 4567 8901 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 12/345 678/901 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 1 2 3 4 5 6 7 8 9 0 1 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 1 2 3 4 5 6 7 8 9 0 1 2 3 ok", "StNr [PII] ok", &["steuer_id"], "StNr **** ok"),
            ("StNr 1 2 3 4 5 6 7 8 9 0 ok", "StNr 1 2 3 4 5 6 7 8 9 0 ok", &[], "StNr 1 2 3 4 5 6 7 8 9 0 ok"),
            ("A12345678 B", "A12345678 B", &[], "A12345678 B"),
            ("12345678.5", "[PII].5", &["kontonummer"], "12****.5"),
            ("12345678,50 Euro", "[PII],50 Euro", &["kontonummer"], "12****,50 Euro"),
            ("kto 123456789 und 1234567", "kto [PII] und 1234567", &["kontonummer"], "kto 12**** und 1234567"),
            ("Betrag 1.234,56 Euro", "Betrag 1.234,56 Euro", &[], "Betrag 1.234,56 Euro"),
            ("Betrag 1234567,89 Euro", "Betrag 1234567,89 Euro", &[], "Betrag 1234567,89 Euro"),
        ],
    );
}

/// Datum TT.MM.JJJJ: Tag, Monat, Jahr.
#[rustfmt::skip]
#[test]
fn datum_wie_python() {
    pruefe(
        "datum",
        &[
            ("am 31.01.2024 ok", "am [PII] ok", &["datum"], "am 31.01.2024 ok"),
            ("am 30.04.2024 ok", "am [PII] ok", &["datum"], "am 30.04.2024 ok"),
            ("am 15.12.2024 ok", "am [PII] ok", &["datum"], "am 15.12.2024 ok"),
            ("am 15.10.2024 ok", "am [PII] ok", &["datum"], "am 15.10.2024 ok"),
            ("am 15.11.2024 ok", "am [PII] ok", &["datum"], "am 15.11.2024 ok"),
            ("am 01.09.2024 ok", "am [PII] ok", &["datum"], "am 01.09.2024 ok"),
            ("am 29.02.2024 ok", "am [PII] ok", &["datum"], "am 29.02.2024 ok"),
            ("am 10.01.2024 ok", "am [PII] ok", &["datum"], "am 10.01.2024 ok"),
            ("am 20.01.2024 ok", "am [PII] ok", &["datum"], "am 20.01.2024 ok"),
            ("am 00.01.2024 ok", "am 00.01.2024 ok", &[], "am 00.01.2024 ok"),
            ("am 32.01.2024 ok", "am 32.01.2024 ok", &[], "am 32.01.2024 ok"),
            ("am 15.13.2024 ok", "am 15.13.2024 ok", &[], "am 15.13.2024 ok"),
            ("am 15.00.2024 ok", "am 15.00.2024 ok", &[], "am 15.00.2024 ok"),
            ("am 1.2.2024 ok", "am 1.2.2024 ok", &[], "am 1.2.2024 ok"),
            ("am 01.02.24 ok", "am 01.02.24 ok", &[], "am 01.02.24 ok"),
            ("am 01.02.20245 ok", "am 01.02.20245 ok", &[], "am 01.02.20245 ok"),
            ("am 01.02.202 ok", "am 01.02.202 ok", &[], "am 01.02.202 ok"),
            ("am 01.2.2024 ok", "am 01.2.2024 ok", &[], "am 01.2.2024 ok"),
            ("am 15.12.2024. ok", "am [PII]. ok", &["datum"], "am 15.12.2024. ok"),
            ("am 19.08.2026 und 20260819 ok", "am [PII] und [PII] ok", &["datum", "kontonummer"], "am 19.08.2026 und 20**** ok"),
            ("am 31.12.1999 ok", "am [PII] ok", &["datum"], "am 31.12.1999 ok"),
            ("am 01.01.0001 ok", "am [PII] ok", &["datum"], "am 01.01.0001 ok"),
            ("am 39.01.2024 ok", "am 39.01.2024 ok", &[], "am 39.01.2024 ok"),
            ("am 40.01.2024 ok", "am 40.01.2024 ok", &[], "am 40.01.2024 ok"),
            ("am 15.19.2024 ok", "am 15.19.2024 ok", &[], "am 15.19.2024 ok"),
        ],
    );
}

/// PLZ + Ort: Stellenzahl, Leerraum, Ausnahmeliste.
#[rustfmt::skip]
#[test]
fn plz_ort_wie_python() {
    pruefe(
        "plz_ort",
        &[
            ("in 12345 Berlin ok", "in [PII] ok", &["plz_ort"], "in 12345 Berlin ok"),
            ("in 1234 Berlin ok", "in 1234 Berlin ok", &[], "in 1234 Berlin ok"),
            ("in 123456 Berlin ok", "in 123456 Berlin ok", &[], "in 123456 Berlin ok"),
            ("in 12345Berlin ok", "in 12345Berlin ok", &[], "in 12345Berlin ok"),
            ("in 12345 Ei ok", "in [PII] ok", &["plz_ort"], "in 12345 Ei ok"),
            ("in 12345 Ulm ok", "in [PII] ok", &["plz_ort"], "in 12345 Ulm ok"),
            ("in 12345 Übach ok", "in [PII] ok", &["plz_ort"], "in 12345 Übach ok"),
            ("in 12345 Ärmel ok", "in [PII] ok", &["plz_ort"], "in 12345 Ärmel ok"),
            ("in 12345 Österreich ok", "in [PII] ok", &["plz_ort"], "in 12345 Österreich ok"),
            ("in 12345 berlin ok", "in 12345 berlin ok", &[], "in 12345 berlin ok"),
            ("in 12345  Berlin ok", "in [PII] ok", &["plz_ort"], "in 12345  Berlin ok"),
            ("in 12345\tBerlin ok", "in [PII] ok", &["plz_ort"], "in 12345\tBerlin ok"),
            ("in 12345 B ok", "in 12345 B ok", &[], "in 12345 B ok"),
            ("in 12345 BERLIN ok", "in 12345 BERLIN ok", &[], "in 12345 BERLIN ok"),
            ("in 12345 Straße ok", "in [PII] ok", &["plz_ort"], "in 12345 Straße ok"),
            ("in 12345 Bad Homburg ok", "in [PII] Homburg ok", &["plz_ort"], "in 12345 Bad Homburg ok"),
            ("12345 Euro ok", "12345 Euro ok", &[], "12345 Euro ok"),
            ("12345 Kilometer ok", "12345 Kilometer ok", &[], "12345 Kilometer ok"),
            ("12345 Meter ok", "12345 Meter ok", &[], "12345 Meter ok"),
            ("12345 Stück ok", "12345 Stück ok", &[], "12345 Stück ok"),
            ("12345 Tonnen ok", "12345 Tonnen ok", &[], "12345 Tonnen ok"),
            ("12345 Liter ok", "12345 Liter ok", &[], "12345 Liter ok"),
            ("12345 Jahre ok", "12345 Jahre ok", &[], "12345 Jahre ok"),
            ("12345 Tage ok", "12345 Tage ok", &[], "12345 Tage ok"),
            ("12345 Stunden ok", "12345 Stunden ok", &[], "12345 Stunden ok"),
            ("12345 Mitglieder ok", "12345 Mitglieder ok", &[], "12345 Mitglieder ok"),
            ("12345 Mitarbeiter ok", "12345 Mitarbeiter ok", &[], "12345 Mitarbeiter ok"),
            ("12345 Einwohner ok", "12345 Einwohner ok", &[], "12345 Einwohner ok"),
            ("12345 Jahr ok", "12345 Jahr ok", &[], "12345 Jahr ok"),
            ("12345 Tag ok", "12345 Tag ok", &[], "12345 Tag ok"),
            ("12345 Stunde ok", "12345 Stunde ok", &[], "12345 Stunde ok"),
            ("12345 EUR ok", "12345 EUR ok", &[], "12345 EUR ok"),
            ("12345 € ok", "12345 € ok", &[], "12345 € ok"),
            ("12345 km ok", "12345 km ok", &[], "12345 km ok"),
            ("12345 m ok", "12345 m ok", &[], "12345 m ok"),
            ("12345 kg ok", "12345 kg ok", &[], "12345 kg ok"),
            ("12345 g ok", "12345 g ok", &[], "12345 g ok"),
            ("12345 Eurox ok", "[PII] ok", &["plz_ort"], "12345 Eurox ok"),
            ("12345 Kilometerx ok", "[PII] ok", &["plz_ort"], "12345 Kilometerx ok"),
            ("12345 Meterx ok", "[PII] ok", &["plz_ort"], "12345 Meterx ok"),
            ("12345 Stückx ok", "[PII] ok", &["plz_ort"], "12345 Stückx ok"),
            ("12345 Tonnenx ok", "[PII] ok", &["plz_ort"], "12345 Tonnenx ok"),
            ("12345 Literx ok", "[PII] ok", &["plz_ort"], "12345 Literx ok"),
            ("12345 Jahrex ok", "[PII] ok", &["plz_ort"], "12345 Jahrex ok"),
            ("12345 Tagex ok", "[PII] ok", &["plz_ort"], "12345 Tagex ok"),
            ("12345 Stundenx ok", "[PII] ok", &["plz_ort"], "12345 Stundenx ok"),
            ("12345 Mitgliederx ok", "[PII] ok", &["plz_ort"], "12345 Mitgliederx ok"),
            ("12345 Mitarbeiterx ok", "[PII] ok", &["plz_ort"], "12345 Mitarbeiterx ok"),
            ("12345 Einwohnerx ok", "[PII] ok", &["plz_ort"], "12345 Einwohnerx ok"),
            ("12345 Jahrx ok", "[PII] ok", &["plz_ort"], "12345 Jahrx ok"),
            ("12345 Tagx ok", "[PII] ok", &["plz_ort"], "12345 Tagx ok"),
            ("12345 Stundex ok", "[PII] ok", &["plz_ort"], "12345 Stundex ok"),
            ("12345 EURx ok", "12345 EURx ok", &[], "12345 EURx ok"),
            ("12345 Eurostar", "[PII]", &["plz_ort"], "12345 Eurostar"),
            ("12345 Tagen", "[PII]", &["plz_ort"], "12345 Tagen"),
            ("12345 Jahren", "[PII]", &["plz_ort"], "12345 Jahren"),
            ("12345 Stunden", "12345 Stunden", &[], "12345 Stunden"),
            ("12345 Meters", "[PII]", &["plz_ort"], "12345 Meters"),
            ("12345 Literatur", "[PII]", &["plz_ort"], "12345 Literatur"),
        ],
    );
}

/// Strasse + Hausnummer: Endungen, Hausnummer Pflicht/optional, Stellen.
#[rustfmt::skip]
#[test]
fn strasse_wie_python() {
    pruefe(
        "strasse",
        &[
            ("Hauptstraße 5", "[PII]", &["strasse"], "Hauptstraße 5"),
            ("Hauptstrasse 5", "[PII]", &["strasse"], "Hauptstrasse 5"),
            ("Hauptstr. 5", "[PII]", &["strasse"], "Hauptstr. 5"),
            ("Hauptstraße", "[PII]", &["strasse"], "Hauptstraße"),
            ("Hauptstrasse", "[PII]", &["strasse"], "Hauptstrasse"),
            ("Hauptstr.", "[PII]", &["strasse"], "Hauptstr."),
            ("Hauptstr", "Hauptstr", &[], "Hauptstr"),
            ("Hauptstrasseplatz", "[PII]platz", &["strasse"], "Hauptstrasseplatz"),
            ("Straße 5", "Straße 5", &[], "Straße 5"),
            ("Str. 5", "Str. 5", &[], "Str. 5"),
            ("hauptstraße 5", "hauptstraße 5", &[], "hauptstraße 5"),
            ("Lindenweg 3", "[PII]", &["strasse"], "Lindenweg 3"),
            ("Kastanienallee 3", "[PII]", &["strasse"], "Kastanienallee 3"),
            ("Marktplatz 3", "[PII]", &["strasse"], "Marktplatz 3"),
            ("Seegasse 3", "[PII]", &["strasse"], "Seegasse 3"),
            ("Mühlendamm 3", "[PII]", &["strasse"], "Mühlendamm 3"),
            ("Stadtring 3", "[PII]", &["strasse"], "Stadtring 3"),
            ("Potsdamerchaussee 3", "[PII]", &["strasse"], "Potsdamerchaussee 3"),
            ("Weg 3", "Weg 3", &[], "Weg 3"),
            ("Allee 3", "Allee 3", &[], "Allee 3"),
            ("Ring 3", "Ring 3", &[], "Ring 3"),
            ("Lindenweg", "Lindenweg", &[], "Lindenweg"),
            ("Kastanienallee", "Kastanienallee", &[], "Kastanienallee"),
            ("Marktplatz", "Marktplatz", &[], "Marktplatz"),
            ("Seegasse", "Seegasse", &[], "Seegasse"),
            ("Mühlendamm", "Mühlendamm", &[], "Mühlendamm"),
            ("Stadtring", "Stadtring", &[], "Stadtring"),
            ("Potsdamerchaussee", "Potsdamerchaussee", &[], "Potsdamerchaussee"),
            ("Arbeitsplatz", "Arbeitsplatz", &[], "Arbeitsplatz"),
            ("Ehering", "Ehering", &[], "Ehering"),
            ("Studienplatz", "Studienplatz", &[], "Studienplatz"),
            ("Hauptstraße 1234", "[PII]", &["strasse"], "Hauptstraße 1234"),
            ("Hauptstraße 12345", "[PII]5", &["strasse"], "Hauptstraße 12345"),
            ("Hauptstraße 5a", "[PII]", &["strasse"], "Hauptstraße 5a"),
            ("Hauptstraße 5ab", "[PII]b", &["strasse"], "Hauptstraße 5ab"),
            ("Hauptstraße 5A", "[PII]A", &["strasse"], "Hauptstraße 5A"),
            ("Hauptstraße  5", "[PII]", &["strasse"], "Hauptstraße  5"),
            ("Lindenweg 1234", "[PII]", &["strasse"], "Lindenweg 1234"),
            ("Lindenweg 12345", "[PII]5", &["strasse"], "Lindenweg 12345"),
            ("Lindenweg 5a", "[PII]", &["strasse"], "Lindenweg 5a"),
            ("Lindenweg 12b", "[PII]", &["strasse"], "Lindenweg 12b"),
            ("Lindenweg 5A", "[PII]A", &["strasse"], "Lindenweg 5A"),
            ("Lindenweg  5", "[PII]", &["strasse"], "Lindenweg  5"),
            ("Hauptstraße 5, Lindenweg 3", "[PII], [PII]", &["strasse"], "Hauptstraße 5, Lindenweg 3"),
            ("Äpfelstraße 1", "[PII]", &["strasse"], "Äpfelstraße 1"),
            ("Östraße 2", "Östraße 2", &[], "Östraße 2"),
            ("Üweg 4", "Üweg 4", &[], "Üweg 4"),
            ("Hauptstraße 0", "[PII]", &["strasse"], "Hauptstraße 0"),
            ("Hauptstr.5", "[PII]5", &["strasse"], "Hauptstr.5"),
        ],
    );
}

/// Anrede Herr/Frau + Name.
#[rustfmt::skip]
#[test]
fn anrede_wie_python() {
    pruefe(
        "anrede",
        &[
            ("Herr Maier", "[PII]", &["anrede_name"], "Herr Maier"),
            ("Frau Meier", "[PII]", &["anrede_name"], "Frau Meier"),
            ("HerrMaier", "HerrMaier", &[], "HerrMaier"),
            ("FrauMeier", "FrauMeier", &[], "FrauMeier"),
            ("Herr maier", "Herr maier", &[], "Herr maier"),
            ("Herr  Maier", "[PII]", &["anrede_name"], "Herr  Maier"),
            ("Herr\tMaier", "[PII]", &["anrede_name"], "Herr\tMaier"),
            ("Hausfrau Meier", "Hausfrau Meier", &[], "Hausfrau Meier"),
            ("Frau Müller-Lüdenscheidt", "[PII]-Lüdenscheidt", &["anrede_name"], "Frau Müller-Lüdenscheidt"),
            ("Herr Ärmel", "[PII]", &["anrede_name"], "Herr Ärmel"),
            ("Frau Öz", "[PII]", &["anrede_name"], "Frau Öz"),
            ("Herr M", "Herr M", &[], "Herr M"),
            ("Herren Maier", "Herren Maier", &[], "Herren Maier"),
            ("Frau Dr. Maier", "[PII]. Maier", &["anrede_name"], "Frau Dr. Maier"),
            ("Herr Maier und Frau Meier", "[PII] und [PII]", &["anrede_name"], "Herr Maier und Frau Meier"),
            ("herr Maier", "herr Maier", &[], "herr Maier"),
            ("Herr MAIER", "Herr MAIER", &[], "Herr MAIER"),
            ("Frauen Meier", "Frauen Meier", &[], "Frauen Meier"),
        ],
    );
}

/// Reihenfolge der Muster und Leereingaben.
#[rustfmt::skip]
#[test]
fn reihenfolge_wie_python() {
    pruefe(
        "reihenfolge",
        &[
            ("12345 Hauptstraße 5", "[PII] 5", &["plz_ort"], "12345 Hauptstraße 5"),
            ("Herr Maierstraße 5", "Herr [PII]", &["strasse"], "Herr Maierstraße 5"),
            ("Frau Lindenweg 3", "Frau [PII]", &["strasse"], "Frau Lindenweg 3"),
            ("12345 Lindenweg 3", "[PII] 3", &["plz_ort"], "12345 Lindenweg 3"),
            ("Herr 12345 Berlin", "Herr [PII]", &["plz_ort"], "Herr 12345 Berlin"),
            ("01.02.2024 12345 Berlin", "[PII] [PII]", &["datum", "plz_ort"], "01.02.2024 12345 Berlin"),
            ("Herr Maier wohnt in 12345 Berlin, Lindenweg 3, am 01.02.2024, IBAN DE89 3704 0044 0532 0130 00, StNr 181/815/08155, Konto 12345678.", "[PII] wohnt in [PII], [PII], am [PII], IBAN [PII], StNr [PII], Konto [PII].", &["anrede_name", "datum", "iban", "kontonummer", "plz_ort", "steuer_id", "strasse"], "Herr Maier wohnt in 12345 Berlin, Lindenweg 3, am 01.02.2024, IBAN DE89****, StNr ****, Konto 12****."),
            ("StNr 12345678901", "StNr [PII]", &["steuer_id"], "StNr ****"),
            ("StNr 123456789012 und 1234567890123", "StNr [PII] und [PII]", &["steuer_id"], "StNr **** und ****"),
            ("IBAN DE89 3704 0044 0532 0130 00", "IBAN [PII]", &["iban"], "IBAN DE89****"),
            ("DE89 3704 0044 0532 0130 00 und 12345678901", "[PII] und [PII]", &["iban", "steuer_id"], "DE89**** und ****"),
            ("de89 3704 0044 0532 0130 00", "[PII]", &["iban"], "de89****"),
            ("12345678 DE89 3704 0044 0532 0130 00", "[PII] [PII]", &["iban", "kontonummer"], "12**** DE89****"),
            ("Kto 12345678 12345678901 DE89370400440532013000", "Kto [PII] [PII] [PII]", &["iban", "kontonummer", "steuer_id"], "Kto 12**** **** DE89****"),
            ("[PII] 12345678", "[PII] [PII]", &["kontonummer"], "[PII] 12****"),
            ("", "", &[], ""),
            (" ", " ", &[], " "),
            ("ohne Treffer", "ohne Treffer", &[], "ohne Treffer"),
        ],
    );
}

/// (Feld-ID, Fragetext, Art. 9?) je Alternative des Musters, ueber `feld_id` und ueber den Fragetext, Gross-/Kleinschreibung.
#[rustfmt::skip]
const ART9: &[(&str, &str, bool)] = &[
    ("konfession", "", true),
    ("x", "konfession", true),
    ("KONFESSION", "", true),
    ("x", "Frage Konfession?", true),
    ("kirche", "", true),
    ("x", "kirche", true),
    ("KIRCHE", "", true),
    ("x", "Frage Kirche?", true),
    ("grad_der_behinderung", "", true),
    ("x", "grad_der_behinderung", true),
    ("GRAD_DER_BEHINDERUNG", "", true),
    ("x", "Frage Grad_der_behinderung?", true),
    ("schwerbehind", "", true),
    ("x", "schwerbehind", true),
    ("SCHWERBEHIND", "", true),
    ("x", "Frage Schwerbehind?", true),
    ("gehbehind", "", true),
    ("x", "gehbehind", true),
    ("GEHBEHIND", "", true),
    ("x", "Frage Gehbehind?", true),
    ("behinderungsbedingte", "", true),
    ("x", "behinderungsbedingte", true),
    ("BEHINDERUNGSBEDINGTE", "", true),
    ("x", "Frage Behinderungsbedingte?", true),
    ("behinderten_pb", "", true),
    ("x", "behinderten_pb", true),
    ("BEHINDERTEN_PB", "", true),
    ("x", "Frage Behinderten_pb?", true),
    ("behinderten-pausch", "", true),
    ("x", "behinderten-pausch", true),
    ("BEHINDERTEN-PAUSCH", "", true),
    ("x", "Frage Behinderten-pausch?", true),
    ("behinderten pausch", "", true),
    ("x", "behinderten pausch", true),
    ("BEHINDERTEN PAUSCH", "", true),
    ("x", "Frage Behinderten pausch?", true),
    ("behindertenpausch", "", true),
    ("x", "behindertenpausch", true),
    ("BEHINDERTENPAUSCH", "", true),
    ("x", "Frage Behindertenpausch?", true),
    ("behindertepausch", "", false),
    ("x", "behindertepausch", false),
    ("BEHINDERTEPAUSCH", "", false),
    ("x", "Frage Behindertepausch?", false),
    ("behindertenpausch", "", true),
    ("x", "behindertenpausch", true),
    ("BEHINDERTENPAUSCH", "", true),
    ("x", "Frage Behindertenpausch?", true),
    ("pflegegrad", "", true),
    ("x", "pflegegrad", true),
    ("PFLEGEGRAD", "", true),
    ("x", "Frage Pflegegrad?", true),
    ("pflegebeduerftig", "", true),
    ("x", "pflegebeduerftig", true),
    ("PFLEGEBEDUERFTIG", "", true),
    ("x", "Frage Pflegebeduerftig?", true),
    ("gepflegter", "", true),
    ("x", "gepflegter", true),
    ("GEPFLEGTER", "", true),
    ("x", "Frage Gepflegter?", true),
    ("pflegst", "", true),
    ("x", "pflegst", true),
    ("PFLEGST", "", true),
    ("x", "Frage Pflegst?", true),
    ("pflege_durch", "", true),
    ("x", "pflege_durch", true),
    ("PFLEGE_DURCH", "", true),
    ("x", "Frage Pflege_durch?", true),
    ("pflegt die person", "", true),
    ("x", "pflegt die person", true),
    ("PFLEGT DIE PERSON", "", true),
    ("x", "Frage Pflegt die person?", true),
    ("hilflos", "", true),
    ("x", "hilflos", true),
    ("HILFLOS", "", true),
    ("x", "Frage Hilflos?", true),
    ("blind", "", true),
    ("x", "blind", true),
    ("BLIND", "", true),
    ("x", "Frage Blind?", true),
    ("taubblind", "", true),
    ("x", "taubblind", true),
    ("TAUBBLIND", "", true),
    ("x", "Frage Taubblind?", true),
    ("merkzeichen", "", true),
    ("x", "merkzeichen", true),
    ("MERKZEICHEN", "", true),
    ("x", "Frage Merkzeichen?", true),
    ("berufsunfaehig", "", true),
    ("x", "berufsunfaehig", true),
    ("BERUFSUNFAEHIG", "", true),
    ("x", "Frage Berufsunfaehig?", true),
    ("erwerbsunfaehig", "", true),
    ("x", "erwerbsunfaehig", true),
    ("ERWERBSUNFAEHIG", "", true),
    ("x", "Frage Erwerbsunfaehig?", true),
    ("krankheitskosten", "", true),
    ("x", "krankheitskosten", true),
    ("KRANKHEITSKOSTEN", "", true),
    ("x", "Frage Krankheitskosten?", true),
    ("heilbehandlung", "", true),
    ("x", "heilbehandlung", true),
    ("HEILBEHANDLUNG", "", true),
    ("x", "Frage Heilbehandlung?", true),
    ("", "", false),
    ("bruttoarbeitslohn", "Brutto?", false),
    ("pflege", "", false),
    ("pflegt", "", false),
    ("behinderten", "", false),
    ("behinderten_", "", false),
    ("kirchensteuer", "", true),
    ("Konfession", "", true),
    ("", "Konfession", true),
    ("KIRCHE", "", true),
    ("x", "Wen pflegst du?", true),
    ("rentner_gepflegter_angaben", "Wer ist die Person?", true),
    ("x", "Wer ist die Person, die du pflegst?", true),
    ("grad_der_behinderung", "x", true),
    ("x", "x", false),
    ("pflegt die person", "", true),
    ("pflegt  die person", "", false),
    ("behinder", "", false),
    ("schwer", "", false),
    ("geh", "", false),
    ("heil", "", false),
    ("hilf", "", false),
    ("krank", "", false),
    ("berufs", "", false),
    ("erwerbs", "", false),
];

/// Art. 9: jede Alternative des Musters ueber `feld_id` und ueber den Fragetext, Gross-/Kleinschreibung.
#[test]
fn art9_wie_python() {
    for (feld, frage, erwartet) in ART9 {
        assert_eq!(
            ist_besondere_kategorie(feld, frage),
            *erwartet,
            "Art. 9: ({feld:?}, {frage:?})"
        );
    }
}

/// Laufzeitfehler eines Musters (Backtracking-Grenze bei sehr langem Text) sperrt FAIL-CLOSED den GANZEN Text.
/// Python kennt diese Grenze nicht und gaebe den Text unveraendert zurueck; hier gilt die Zusage der Moduldoku.
/// 1 Mio. Zeichen liegen gemessen weit ueber der Grenze (ab 250 000 bis 500 000 je nach Zeichen), 10 000 weit darunter.
#[test]
fn laufzeitfehler_der_muster_sperrt_den_ganzen_text() {
    let lang = "a".repeat(1_000_000);
    let (g, k) = filtere(&lang);
    assert_eq!(g.as_str(), "[PII]", "filtere gibt keinen Rohtext frei");
    assert_eq!(k, vec!["filter_abgebrochen"]);
    assert_eq!(
        maskiere(&lang).as_str(),
        "****",
        "maskiere gibt keinen Rohtext frei"
    );
    let kurz = "a".repeat(10_000);
    let (g, k) = filtere(&kurz);
    assert_eq!((g.as_str(), k.len()), (kurz.as_str(), 0));
    assert_eq!(maskiere(&kurz).as_str(), kurz);
}

/// Die Ausgabetypen zeigen ihren Text auch ueber `Display`.
#[test]
fn display_zeigt_den_text_der_typen() {
    let (g, _) = filtere("IBAN DE89370400440532013000");
    assert_eq!(g.to_string(), "IBAN [PII]");
    assert_eq!(g.as_str(), "IBAN [PII]");
    let m = maskiere("Kto 12345678");
    assert_eq!(m.to_string(), "Kto 12****");
    assert_eq!(m.as_str(), "Kto 12****");
}
