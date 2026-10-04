//! Euro->Cent-Pfade der Engine-Zugriffe (`engine::zugriff::teil1`) und ihrer Ganzzahl-Guards: jede Fundstelle, an
//! der ein Euro-Wert vor der Catala-Rechnung mal 100 genommen wird (`zugriff/teil1/fehler::in_cent`), und jede
//! `checked_*`-Stelle, die die Accessorkette selbst ueberschreitet (`ok(...)`, `checked_add`, `checked_mul`,
//! `checked_sub`). Je Fundstelle meldet Rust `EngineFehler::Ueberlauf(<Marke>)` statt einer falschen Zahl — im
//! Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Die Mutanten (Bericht h8-hermetisch5, Bestand 354 passed / 0 failed, gemessen 2026-10-04 auf
//! 0aa91677): Pa1-Pf6 ersetzen je ein `in_cent(<feld>)?` durch `Ok(Cent::new(<feld>.get().wrapping_mul(100)))`
//! (Pruefung fort, Wert gewickelt); N1 trifft `fehler::in_cent` selbst; N2-N10 die Guards `ok(checked_sub)`,
//! `checked_mul(14)`, `checked_mul(satz)`, `checked_sub(a)`, `checked_add(65)`, `checked_sub(1)`; N6
//! (`checked_div_euclid(100)` -> `div_euclid`) und I6 sind nach Quelle unerreichbar (ponytail an der Stelle).
//! Alle ueberleben den Bestand; mit diesem Test ist jede Fundstelle rot, je Fall der Funktion.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jeder Python-Wert ist die Ausgabe von `produkt/engine/runner.py` auf
//! denselben Eingaben (Orakel-Skript `orakel_en5b.py`, Lauf: Anlagen zum Bericht; Wegwerf-Datenwurzel, VZ 2025;
//! die Read-Schluessel stehen wortgleich in den Docstrings der Accessoren). Die Marke ist NICHT geraten: das
//! Orakel prueft pro Fall den Cent-Ausdruck des Feldes (`wert * 100`) bzw. den Guard-Ausdruck gegen `i64` und
//! nennt die Stelle nur, wenn er aus `i64::MIN..=i64::MAX` laeuft. Rust rechnet in `i64` mit `checked_*`;
//! `EngineFehler::Ueberlauf` hat ausdruecklich kein Python-Gegenstueck (`zugriff/teil1/fehler.rs`). Je Fall
//! steht die Art im Namen: "Python-Wert ausserhalb i64" (Rust MUSS Ueberlauf melden, das stuetzt das Orakel),
//! "nur ein Zwischenprodukt ausserhalb i64" (Python liefert einen gueltigen Wert, Rust meldet Ueberlauf — die
//! dokumentierte fail-closed-Konvention, KEINE Python-Stuetze; diese Faelle sind die Mehrzahl, weil die
//! Engine vor der Rechnung in Cent umsteigt und die Deckel — 6.000 EUR, 2.800 EUR, 20 % — den Endwert wieder
//! klein machen), "passt gerade noch" (Gegenprobe: 92.233.720.368.547.758 EUR mal 100 ist noch `i64`, Rust
//! MUSS die Python-Zahl liefern; ohne diese Faelle koennte ein Guard jede grosse Zahl ablehnen).
//!
//! 108 Faelle: 8 klar, 71 Zwischenprodukt, 29 Gegenproben. Eine Zeile je Fundstelle, eine
//! Schleife, eine Sammelmeldung, die unter einer Mutation jeden roten Fall nennt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types,
    clippy::too_many_lines
)]

use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;

use bindung::Params;
use domain::{Cent, Euro, Km, Vz};
use engine::tarif::Veranlagung;
use engine::zugriff::teil1::afa::{p6_2_gwg, P62GwgEingabe};
use engine::zugriff::teil2::est::{fuenftel, est_einzel, est_zusammen, tarif_est, EstEinzelEingabe,
    EstZusammenEingabe, FuenftelEingabe, TarifEingabe};
use engine::zugriff::teil2::gesamt::{
    ermaessigter_durchschnittssatz, gesamt, p10d_2, DurchschnittssatzEingabe, GesamtfallEingabe,
    VerlustabzugEingabe,
};
use engine::zugriff::teil2::kapital::kapital_verrechnung;
use engine::zugriff::teil2::kapital::KapitalVerrechnungEingabe;
use engine::zugriff::teil2::rente::{EinkuenfteVersorgungEingabe, VersorgungsfreibetragEingabe};
use engine::zugriff::teil2::sonstige::{kfz_nutzungswert_monat_cent, KfzNutzungswertEingabe};
use engine::zugriff::teil2::solz::{solz, SolzEingabe};
use engine::zugriff::teil1::belastungen::{p33_agb, p33_zumutbar, P33AgbEingabe, P33ZumutbarEingabe};
use engine::zugriff::teil1::einkuenfte::{
    einkuenfte_nichtselbststaendig, euer_gewinn, mitunternehmer_einkuenfte, p16_4_freibetrag,
    p21_2_verbilligt, EinkuenfteNichtselbststaendigEingabe, EuerGewinnEingabe,
    MitunternehmerEinkuenfteEingabe, P164FreibetragEingabe, P212VerbilligtEingabe,
};
use engine::zugriff::teil1::ermaessigungen::{
    kist, p24a_altersentlastung, p31_familienleistung, p35a_haushaltsnahe, p36_abschlusszahlung,
    KistEingabe, P24aAltersentlastungEingabe, P31FamilienleistungEingabe, P35aHaushaltsnaheEingabe,
    P36AbschlusszahlungEingabe,
};
use engine::zugriff::teil1::fehler::EngineFehler;
use engine::zugriff::teil1::mobilitaetspraemie::{
    p101_mobilitaetspraemie, p101_mobilitaetspraemie_cent, P101Eingabe,
};
use engine::zugriff::teil1::sonderausgaben::{
    p10_1_7_berufsausbildung, p10_4b_erstattungsueberhang, p10_kv_pv, p10_kist, p10b_spenden,
    P1017BerufsausbildungEingabe, P10bSpendenEingabe, P10KistEingabe, P10KvPvEingabe,
};
use engine::zugriff::teil1::werbungskosten::{
    entfernungspauschale, raumkosten, EntfernungspauschaleEingabe, RaumkostenEingabe,
};
use rust_decimal::Decimal;

/// Eine Eingabe wie im Python-Sachverhalt: Read-Schluessel und Zahl (Euro, bei `kist`/`p36` Cent).
enum Roeh {
    Ganz(i64),
    Bool(bool),
    Text(&'static str),
}

type Eingabe = (&'static str, Roeh);

/// `(Name, Funktion, Eingaben, Python-Wert, Marke)`. `Some(marke)`: Rust MUSS `Ueberlauf(marke)` melden;
/// `None`: Rust MUSS den Python-Wert liefern.
/// Die i64-Grenze als `i128` — dieselbe Schranke, die `checked_mul(100)` und `i64::try_from` pruefen.
const GRENZE_OBEN: i128 = 9_223_372_036_854_775_807_i128;
const GRENZE_UNTEN: i128 = -9_223_372_036_854_775_808_i128;
#[allow(clippy::type_complexity)] // eine Tabelle von Faellen, keine Logik
const FAELLE: &[(&str, &str, &[Eingabe], i128, Option<&str>)] = &[
    ("GdE 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p33_zumutbar", &[
        ("anzahl_kinder", Roeh::Ganz(0)),
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(92_233_720_368_547_758)),
        ("splitting", Roeh::Bool(false)),
    ], 6_456_360_425_797_678_i128, None),

    ("GdE i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "p33_zumutbar", &[
        ("anzahl_kinder", Roeh::Ganz(0)),
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(i64::MAX)),
        ("splitting", Roeh::Bool(false)),
    ], 645_636_042_579_833_641_i128, Some("Euro->Cent")),

    ("agB-Aufwendungen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p33_agb", &[
        ("anzahl_kinder", Roeh::Ganz(0)),
        ("aussergewoehnliche_belastungen", Roeh::Ganz(i64::MAX)),
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(1000)),
        ("splitting", Roeh::Bool(false)),
    ], 9_223_372_036_854_775_757_i128, Some("Euro->Cent")),

    ("agB-Kette: GdE i64::MAX EUR im zumutbar-Zweig [nur ein Zwischenprodukt ausserhalb i64]", "p33_agb", &[
        ("anzahl_kinder", Roeh::Ganz(0)),
        ("aussergewoehnliche_belastungen", Roeh::Ganz(1000)),
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(i64::MAX)),
        ("splitting", Roeh::Bool(false)),
    ], 0_i128, Some("Euro->Cent")),

    ("GdE der 20-%-Deckelung i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p10b_spenden", &[
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(i64::MAX)),
        ("zuwendungen", Roeh::Ganz(1000)),
    ], 1000_i128, Some("Euro->Cent")),

    ("Zuwendungen 92.233.720.368.547.758 EUR bei GdE 0 EUR (x 100 passt gerade) [passt gerade noch]", "p10b_spenden", &[
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(0)),
        ("zuwendungen", Roeh::Ganz(92_233_720_368_547_758)),
    ], 0_i128, None),

    ("Zuwendungen 92.233.720.368.547.758 EUR, GdE 20 mal so viel [nur ein Zwischenprodukt ausserhalb i64]", "p10b_spenden", &[
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(i64::MAX)),
        ("zuwendungen", Roeh::Ganz(92_233_720_368_547_758)),
    ], 92_233_720_368_547_758_i128, Some("Euro->Cent")),

    ("Zuwendungen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p10b_spenden", &[
        ("gesamtbetrag_der_einkuenfte", Roeh::Ganz(1000)),
        ("zuwendungen", Roeh::Ganz(i64::MAX)),
    ], 200_i128, Some("Euro->Cent")),

    ("Erstattete KiSt i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p10_kist", &[
        ("erstattete_kirchensteuer", Roeh::Ganz(i64::MAX)),
        ("gezahlte_kirchensteuer", Roeh::Ganz(0)),
    ], 0_i128, Some("Euro->Cent")),

    ("Gezahlte KiSt 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p10_kist", &[
        ("erstattete_kirchensteuer", Roeh::Ganz(0)),
        ("gezahlte_kirchensteuer", Roeh::Ganz(92_233_720_368_547_758)),
    ], 92_233_720_368_547_758_i128, None),

    ("Gezahlte KiSt i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p10_kist", &[
        ("erstattete_kirchensteuer", Roeh::Ganz(0)),
        ("gezahlte_kirchensteuer", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("KV/PV weitere Aufwendungen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p10_kv_pv", &[
        ("basis_kv_pv", Roeh::Ganz(0)),
        ("mit_anspruch_auf_zuschuss", Roeh::Bool(false)),
        ("weitere_vorsorgeaufwendungen", Roeh::Ganz(i64::MAX)),
    ], 2800_i128, Some("Euro->Cent")),

    ("KV/PV-Basis 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p10_kv_pv", &[
        ("basis_kv_pv", Roeh::Ganz(92_233_720_368_547_758)),
        ("mit_anspruch_auf_zuschuss", Roeh::Bool(false)),
        ("weitere_vorsorgeaufwendungen", Roeh::Ganz(0)),
    ], 92_233_720_368_547_758_i128, None),

    ("KV/PV-Basis i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p10_kv_pv", &[
        ("basis_kv_pv", Roeh::Ganz(i64::MAX)),
        ("mit_anspruch_auf_zuschuss", Roeh::Bool(false)),
        ("weitere_vorsorgeaufwendungen", Roeh::Ganz(0)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("Berufsausbildung 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p10_1_7_berufsausbildung", &[
        ("berufsausbildung_aufwendungen", Roeh::Ganz(92_233_720_368_547_758)),
    ], 6000_i128, None),

    ("Berufsausbildung i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p10_1_7_berufsausbildung", &[
        ("berufsausbildung_aufwendungen", Roeh::Ganz(i64::MAX)),
    ], 6000_i128, Some("Euro->Cent")),

    ("p10_4b erstattet 500, gezahlt i64::MAX (Differenz in i64) [passt gerade noch]", "p10_4b_erstattungsueberhang", &[
        ("erstattete_kirchensteuer", Roeh::Ganz(500)),
        ("gezahlte_kirchensteuer", Roeh::Ganz(i64::MAX)),
    ], 0_i128, None),

    ("p10_4b erstattet i64::MIN minus gezahlter KiSt [nur ein Zwischenprodukt ausserhalb i64]", "p10_4b_erstattungsueberhang", &[
        ("erstattete_kirchensteuer", Roeh::Ganz(i64::MIN)),
        ("gezahlte_kirchensteuer", Roeh::Ganz(1)),
    ], 0_i128, Some("p10_4b")),

    ("p35a Dienstleistungen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p35a_haushaltsnahe", &[
        ("hh_dienstleistungen", Roeh::Ganz(i64::MAX)),
        ("hh_in_eu_ewr", Roeh::Bool(true)),
        ("hh_rechnung_unbar", Roeh::Bool(true)),
    ], 4000_i128, Some("Euro->Cent")),

    ("p35a Handwerker i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p35a_haushaltsnahe", &[
        ("hh_handwerker_arbeitskosten", Roeh::Ganz(i64::MAX)),
        ("hh_in_eu_ewr", Roeh::Bool(true)),
        ("hh_rechnung_unbar", Roeh::Bool(true)),
    ], 1200_i128, Some("Euro->Cent")),

    ("p35a Minijob 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p35a_haushaltsnahe", &[
        ("hh_in_eu_ewr", Roeh::Bool(true)),
        ("hh_minijob_aufwendungen", Roeh::Ganz(92_233_720_368_547_758)),
        ("hh_rechnung_unbar", Roeh::Bool(true)),
    ], 510_i128, None),

    ("p35a Minijob i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p35a_haushaltsnahe", &[
        ("hh_in_eu_ewr", Roeh::Bool(true)),
        ("hh_minijob_aufwendungen", Roeh::Ganz(i64::MAX)),
        ("hh_rechnung_unbar", Roeh::Bool(true)),
    ], 510_i128, Some("Euro->Cent")),

    ("kist K8 (x 8 passt gerade) [passt gerade noch]", "kist", &[
        ("bundesland", Roeh::Text("baden_wuerttemberg")),
        ("est_mit_fb", Roeh::Ganz(1_152_921_504_606_846_975)),
        ("konfession", Roeh::Text("evangelisch")),
    ], 9_223_372_036_854_775_800_i128, None),

    ("kist K9 (x 9 passt gerade) [passt gerade noch]", "kist", &[
        ("bundesland", Roeh::Text("hessen")),
        ("est_mit_fb", Roeh::Ganz(1_024_819_115_206_086_200)),
        ("konfession", Roeh::Text("evangelisch")),
    ], 9_223_372_036_854_775_800_i128, None),

    ("kist i64::MAX mal 8 % (Bayern: ebenfalls ausserhalb) [Python-Wert ausserhalb i64]", "kist", &[
        ("bundesland", Roeh::Text("bayern")),
        ("est_mit_fb", Roeh::Ganz(i64::MAX)),
        ("konfession", Roeh::Text("evangelisch")),
    ], 73_786_976_294_838_206_456_i128, Some("kist")),

    ("kist i64::MAX mal 9 % [Python-Wert ausserhalb i64]", "kist", &[
        ("bundesland", Roeh::Text("hessen")),
        ("est_mit_fb", Roeh::Ganz(i64::MAX)),
        ("konfession", Roeh::Text("evangelisch")),
    ], 83_010_348_331_692_982_263_i128, Some("kist")),

    ("p36 Festsetzung i64::MAX, keine Abzuege (passt) [passt gerade noch]", "p36_abschlusszahlung", &[
        ("festzusetzende_est_cent", Roeh::Ganz(i64::MAX)),
        ("kapitalertragsteuer_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_kist_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_solz_cent", Roeh::Ganz(0)),
        ("lohnsteuer_cent", Roeh::Ganz(0)),
        ("vorauszahlungen_cent", Roeh::Ganz(0)),
    ], 9_223_372_036_854_775_807_i128, None),

    ("p36 Festsetzung i64::MIN minus 1 Cent LSt [Python-Wert ausserhalb i64]", "p36_abschlusszahlung", &[
        ("festzusetzende_est_cent", Roeh::Ganz(i64::MIN)),
        ("kapitalertragsteuer_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_kist_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_solz_cent", Roeh::Ganz(0)),
        ("lohnsteuer_cent", Roeh::Ganz(1)),
        ("vorauszahlungen_cent", Roeh::Ganz(0)),
    ], -9_223_372_036_854_775_908_i128, Some("p36")),

    ("p36 LSt 9.223.372.036.854.775.800 Cent: aufgerundet 92.233.720.368.547.758 EUR (passt gerade) [passt gerade noch]", "p36_abschlusszahlung", &[
        ("festzusetzende_est_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_kist_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_solz_cent", Roeh::Ganz(0)),
        ("lohnsteuer_cent", Roeh::Ganz(9_223_372_036_854_775_800)),
        ("vorauszahlungen_cent", Roeh::Ganz(0)),
    ], -9_223_372_036_854_775_800_i128, None),

    ("p36 LSt i64::MAX Cent: aufgerundet 92.233.720.368.547.759 EUR, mal 100 ausserhalb [Python-Wert ausserhalb i64]", "p36_abschlusszahlung", &[
        ("festzusetzende_est_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_kist_cent", Roeh::Ganz(0)),
        ("kapitalertragsteuer_solz_cent", Roeh::Ganz(0)),
        ("lohnsteuer_cent", Roeh::Ganz(i64::MAX)),
        ("vorauszahlungen_cent", Roeh::Ganz(0)),
    ], -9_223_372_036_854_775_900_i128, Some("Euro->Cent")),

    ("p24a Arbeitslohn i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p24a_altersentlastung", &[
        ("arbeitslohn", Roeh::Ganz(i64::MAX)),
        ("geburtsjahr", Roeh::Ganz(1960)),
        ("positive_andere_einkuenfte", Roeh::Ganz(0)),
    ], 627_i128, Some("Euro->Cent")),

    ("p24a Geburtsjahr i64::MAX (plus 65 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "p24a_altersentlastung", &[
        ("arbeitslohn", Roeh::Ganz(10000)),
        ("geburtsjahr", Roeh::Ganz(i64::MAX)),
        ("positive_andere_einkuenfte", Roeh::Ganz(0)),
    ], 0_i128, Some("p24a folgejahr")),

    ("p24a Geburtsjahr i64::MAX - 65 (passt gerade) [passt gerade noch]", "p24a_altersentlastung", &[
        ("arbeitslohn", Roeh::Ganz(10000)),
        ("geburtsjahr", Roeh::Ganz(9_223_372_036_854_775_742)),
        ("positive_andere_einkuenfte", Roeh::Ganz(0)),
    ], 0_i128, None),

    ("p24a andere Einkuenfte i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p24a_altersentlastung", &[
        ("arbeitslohn", Roeh::Ganz(10000)),
        ("geburtsjahr", Roeh::Ganz(1960)),
        ("positive_andere_einkuenfte", Roeh::Ganz(i64::MAX)),
    ], 627_i128, Some("Euro->Cent")),

    ("p31 ESt mit Freibetraegen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p31_familienleistung", &[
        ("est_mit_freibetraegen", Roeh::Ganz(i64::MAX)),
        ("est_ohne_freibetraege", Roeh::Ganz(0)),
        ("kindergeld", Roeh::Ganz(0)),
    ], 0_i128, Some("Euro->Cent")),

    ("p31 ESt ohne Freibetraege i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p31_familienleistung", &[
        ("est_mit_freibetraegen", Roeh::Ganz(0)),
        ("est_ohne_freibetraege", Roeh::Ganz(i64::MAX)),
        ("kindergeld", Roeh::Ganz(0)),
    ], 0_i128, Some("Euro->Cent")),

    ("p31 Kindergeld 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p31_familienleistung", &[
        ("est_mit_freibetraegen", Roeh::Ganz(0)),
        ("est_ohne_freibetraege", Roeh::Ganz(0)),
        ("kindergeld", Roeh::Ganz(92_233_720_368_547_758)),
    ], 0_i128, None),

    ("p31 Kindergeld i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p31_familienleistung", &[
        ("est_mit_freibetraegen", Roeh::Ganz(0)),
        ("est_ohne_freibetraege", Roeh::Ganz(0)),
        ("kindergeld", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("AN-Bruttolohn 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "einkuenfte_nichtselbststaendig", &[
        ("bruttoarbeitslohn", Roeh::Ganz(92_233_720_368_547_758)),
        ("werbungskosten", Roeh::Ganz(0)),
    ], 92_233_720_368_546_528_i128, None),

    ("AN-Bruttolohn i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "einkuenfte_nichtselbststaendig", &[
        ("bruttoarbeitslohn", Roeh::Ganz(i64::MAX)),
        ("werbungskosten", Roeh::Ganz(0)),
    ], 9_223_372_036_854_774_577_i128, Some("Euro->Cent")),

    ("AN-Werbungskosten i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "einkuenfte_nichtselbststaendig", &[
        ("bruttoarbeitslohn", Roeh::Ganz(1000)),
        ("werbungskosten", Roeh::Ganz(i64::MAX)),
    ], -9_223_372_036_854_774_807_i128, Some("Euro->Cent")),

    ("p21_2 Werbungskosten i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p21_2_verbilligt", &[
        ("entgelt_quote_prozent", Roeh::Ganz(50)),
        ("werbungskosten", Roeh::Ganz(i64::MAX)),
    ], 4_611_686_018_427_387_903_i128, Some("Euro->Cent")),

    ("p16_4 Veraeusserungsgewinn i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p16_4_freibetrag", &[
        ("rentner_veraeusserungsgewinn", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("EUER Betriebsausgaben i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "euer_gewinn", &[
        ("betriebsausgaben", Roeh::Ganz(i64::MAX)),
        ("betriebseinnahmen", Roeh::Ganz(0)),
    ], -9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("EUER Betriebseinnahmen 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "euer_gewinn", &[
        ("betriebsausgaben", Roeh::Ganz(0)),
        ("betriebseinnahmen", Roeh::Ganz(92_233_720_368_547_758)),
    ], 92_233_720_368_547_758_i128, None),

    ("EUER Betriebseinnahmen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "euer_gewinn", &[
        ("betriebsausgaben", Roeh::Ganz(0)),
        ("betriebseinnahmen", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("Mitunternehmer Darlehen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "mitunternehmer_einkuenfte", &[
        ("gewinnanteil", Roeh::Ganz(0)),
        ("verguetung_darlehen", Roeh::Ganz(i64::MAX)),
        ("verguetung_taetigkeit", Roeh::Ganz(0)),
        ("verguetung_ueberlassung", Roeh::Ganz(0)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("Mitunternehmer Gewinnanteil i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "mitunternehmer_einkuenfte", &[
        ("gewinnanteil", Roeh::Ganz(i64::MAX)),
        ("verguetung_darlehen", Roeh::Ganz(0)),
        ("verguetung_taetigkeit", Roeh::Ganz(0)),
        ("verguetung_ueberlassung", Roeh::Ganz(0)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("Mitunternehmer Taetigkeit i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "mitunternehmer_einkuenfte", &[
        ("gewinnanteil", Roeh::Ganz(0)),
        ("verguetung_darlehen", Roeh::Ganz(0)),
        ("verguetung_taetigkeit", Roeh::Ganz(i64::MAX)),
        ("verguetung_ueberlassung", Roeh::Ganz(0)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("Mitunternehmer Ueberlassung i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "mitunternehmer_einkuenfte", &[
        ("gewinnanteil", Roeh::Ganz(0)),
        ("verguetung_darlehen", Roeh::Ganz(0)),
        ("verguetung_taetigkeit", Roeh::Ganz(0)),
        ("verguetung_ueberlassung", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("GWG 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p6_2_gwg", &[
        ("gwg_anschaffungskosten_netto", Roeh::Ganz(92_233_720_368_547_758)),
    ], 0_i128, None),

    ("GWG Anschaffungskosten netto i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "p6_2_gwg", &[
        ("gwg_anschaffungskosten_netto", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("EP OePNV-Kosten 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "entfernungspauschale", &[
        ("arbeitstage", Roeh::Ganz(100)),
        ("eigenes_oder_ueberlassenes_kfz", Roeh::Bool(true)),
        ("entfernung_km_roh", Roeh::Text("10")),
        ("oepnv_kosten_jahr", Roeh::Ganz(92_233_720_368_547_758)),
    ], 92_233_720_368_547_758_i128, None),

    ("EP OePNV-Kosten i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "entfernungspauschale", &[
        ("arbeitstage", Roeh::Ganz(100)),
        ("eigenes_oder_ueberlassenes_kfz", Roeh::Bool(true)),
        ("entfernung_km_roh", Roeh::Text("10")),
        ("oepnv_kosten_jahr", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("raumkosten tatsaechliche Aufwendungen i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "raumkosten", &[
        ("arbeitszimmer_vorhanden", Roeh::Bool(true)),
        ("homeoffice_tage", Roeh::Ganz(0)),
        ("ist_mittelpunkt", Roeh::Bool(true)),
        ("jahrespauschale_gewaehlt", Roeh::Bool(false)),
        ("monate_ohne_mittelpunkt", Roeh::Ganz(0)),
        ("tatsaechliche_aufwendungen", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("p101 AN: Werbungskosten i64::MIN minus Pauschbetrag [nur ein Zwischenprodukt ausserhalb i64]", "p101_mobilitaetspraemie", &[
        ("arbeitnehmer_pauschbetrag", Roeh::Ganz(1)),
        ("entfernungspauschale_ab_21km", Roeh::Ganz(1000)),
        ("grundfreibetrag", Roeh::Ganz(i64::MAX)),
        ("ist_arbeitnehmer", Roeh::Bool(true)),
        ("werbungskosten_gesamt", Roeh::Ganz(i64::MIN)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(0)),
    ], 0_i128, Some("p101 wk")),

    ("p101 EURO: Bemessungsgrundlage K9 mal 14 (passt gerade) [passt gerade noch]", "p101_mobilitaetspraemie", &[
        ("arbeitnehmer_pauschbetrag", Roeh::Ganz(0)),
        ("entfernungspauschale_ab_21km", Roeh::Ganz(658_812_288_346_769_700)),
        ("grundfreibetrag", Roeh::Ganz(i64::MAX)),
        ("ist_arbeitnehmer", Roeh::Bool(false)),
        ("werbungskosten_gesamt", Roeh::Ganz(0)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(0)),
    ], 92_233_720_368_547_758_i128, None),

    ("p101 EURO: Bemessungsgrundlage i64::MAX mal 14 [nur ein Zwischenprodukt ausserhalb i64]", "p101_mobilitaetspraemie", &[
        ("arbeitnehmer_pauschbetrag", Roeh::Ganz(0)),
        ("entfernungspauschale_ab_21km", Roeh::Ganz(i64::MAX)),
        ("grundfreibetrag", Roeh::Ganz(i64::MAX)),
        ("ist_arbeitnehmer", Roeh::Bool(false)),
        ("werbungskosten_gesamt", Roeh::Ganz(0)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(0)),
    ], 1_291_272_085_159_668_612_i128, Some("p101")),

    ("p101 EURO: Gegenprobe 1.001 EUR [passt gerade noch]", "p101_mobilitaetspraemie", &[
        ("arbeitnehmer_pauschbetrag", Roeh::Ganz(0)),
        ("entfernungspauschale_ab_21km", Roeh::Ganz(1001)),
        ("grundfreibetrag", Roeh::Ganz(i64::MAX)),
        ("ist_arbeitnehmer", Roeh::Bool(false)),
        ("werbungskosten_gesamt", Roeh::Ganz(0)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(0)),
    ], 140_i128, None),

    ("p101: GFB i64::MIN minus zvE [nur ein Zwischenprodukt ausserhalb i64]", "p101_mobilitaetspraemie", &[
        ("arbeitnehmer_pauschbetrag", Roeh::Ganz(0)),
        ("entfernungspauschale_ab_21km", Roeh::Ganz(1000)),
        ("grundfreibetrag", Roeh::Ganz(i64::MIN)),
        ("ist_arbeitnehmer", Roeh::Bool(false)),
        ("werbungskosten_gesamt", Roeh::Ganz(0)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(1)),
    ], 0_i128, Some("p101 gfb")),

    ("p101 CENT: Bemessungsgrundlage i64::MAX mal 14 [Python-Wert ausserhalb i64]", "p101_mobilitaetspraemie_cent", &[
        ("arbeitnehmer_pauschbetrag", Roeh::Ganz(0)),
        ("entfernungspauschale_ab_21km", Roeh::Ganz(i64::MAX)),
        ("grundfreibetrag", Roeh::Ganz(i64::MAX)),
        ("ist_arbeitnehmer", Roeh::Bool(false)),
        ("werbungskosten_gesamt", Roeh::Ganz(0)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(0)),
    ], 129_127_208_515_966_861_298_i128, Some("p101 cent")),

    ("p101 CENT: Gegenprobe 1.001 EUR [passt gerade noch]", "p101_mobilitaetspraemie_cent", &[
        ("arbeitnehmer_pauschbetrag", Roeh::Ganz(0)),
        ("entfernungspauschale_ab_21km", Roeh::Ganz(1001)),
        ("grundfreibetrag", Roeh::Ganz(i64::MAX)),
        ("ist_arbeitnehmer", Roeh::Bool(false)),
        ("werbungskosten_gesamt", Roeh::Ganz(0)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(0)),
    ], 14014_i128, None),

    ("Fuenftel: zvE G EUR, ao = 5 (G*100 passt gerade; Gegenprobe gegen zu breiten Guard) [passt gerade noch]", "fuenftel", &[
        ("ausserordentliche_einkuenfte", Roeh::Ganz(5)),
        ("tarif_modifiziert", Roeh::Bool(false)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(92_233_720_368_547_758)),
    ], 41_505_174_165_827_242_i128, None),

    ("Fuenftel: zvE i64::MAX, ao = i64::MIN (Zwischenwert ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "fuenftel", &[
        ("ausserordentliche_einkuenfte", Roeh::Ganz(i64::MIN)),
        ("tarif_modifiziert", Roeh::Bool(false)),
        ("zu_versteuerndes_einkommen", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_865_i128, Some("teil2 i64")),

    ("Solz: Basis 167.697.673.397.359.237 EUR (5,5 % = 9.223.372.036.854.758.035 Cent, passt gerade) [passt gerade noch]", "solz", &[
        ("bemessungsgrundlage", Roeh::Ganz(1_676_976_733_973_595_600)),
        ("kapital_steuer", Roeh::Ganz(0)),
        ("splitting", Roeh::Bool(false)),
    ], 9_223_372_036_854_775_800_i128, None),

    ("Solz: Basis i64::MAX, Kap 0 (5,5 % in Cent ausserhalb) [Python-Wert ausserhalb i64]", "solz", &[
        ("bemessungsgrundlage", Roeh::Ganz(i64::MAX)),
        ("kapital_steuer", Roeh::Ganz(0)),
        ("splitting", Roeh::Bool(false)),
    ], 50_728_546_202_701_266_938_i128, Some("teil2 i64")),

    ("Kfz 1 %-Regel: BLP 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "kfz_nutzungswert_monat_cent", &[
        ("bruchteils_teiler", Roeh::Ganz(4)),
        ("bruttolistenpreis", Roeh::Ganz(92_233_720_368_547_758)),
    ], 23_058_430_092_136_939_i128, None),

    ("Kfz 1 %-Regel: BLP i64::MAX EUR, Teiler 4 (128-Zwischenwert, Endwert passt) [passt gerade noch]", "kfz_nutzungswert_monat_cent", &[
        ("bruchteils_teiler", Roeh::Ganz(4)),
        ("bruttolistenpreis", Roeh::Ganz(i64::MAX)),
    ], 2_305_843_009_213_693_951_i128, None),

    ("Durchschnittssatz: Bemessungsgrundlage i64::MAX EUR (drittes Feld) [nur ein Zwischenprodukt ausserhalb i64]", "ermaessigter_durchschnittssatz", &[
        ("ao_einkuenfte", Roeh::Ganz(30000)),
        ("bemessungsgrundlage_durchschnitt", Roeh::Ganz(i64::MAX)),
        ("est_gesamt_zzgl_progression", Roeh::Ganz(200_000)),
    ], 4200_i128, Some("Euro->Cent")),

    ("Durchschnittssatz: ESt i64::MAX EUR, Rest klein (mittleres Feld allein) [nur ein Zwischenprodukt ausserhalb i64]", "ermaessigter_durchschnittssatz", &[
        ("ao_einkuenfte", Roeh::Ganz(30000)),
        ("bemessungsgrundlage_durchschnitt", Roeh::Ganz(1_000_000)),
        ("est_gesamt_zzgl_progression", Roeh::Ganz(i64::MAX)),
    ], 154_952_650_219_160_233_i128, Some("Euro->Cent")),

    ("Durchschnittssatz: alle drei 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "ermaessigter_durchschnittssatz", &[
        ("ao_einkuenfte", Roeh::Ganz(92_233_720_368_547_758)),
        ("bemessungsgrundlage_durchschnitt", Roeh::Ganz(92_233_720_368_547_758)),
        ("est_gesamt_zzgl_progression", Roeh::Ganz(92_233_720_368_547_758)),
    ], 2_800_000_i128, None),

    ("Durchschnittssatz: ao i64::MAX EUR (erstes Feld) [nur ein Zwischenprodukt ausserhalb i64]", "ermaessigter_durchschnittssatz", &[
        ("ao_einkuenfte", Roeh::Ganz(i64::MAX)),
        ("bemessungsgrundlage_durchschnitt", Roeh::Ganz(1_000_000)),
        ("est_gesamt_zzgl_progression", Roeh::Ganz(30000)),
    ], 700_000_i128, Some("Euro->Cent")),

    ("Verlustvortrag p10d_2: Bestand i64::MAX EUR (zweites Feld) [nur ein Zwischenprodukt ausserhalb i64]", "p10d_2", &[
        ("gesamtbetrag_einkuenfte", Roeh::Ganz(0)),
        ("verlustvortrag_bestand", Roeh::Ganz(i64::MAX)),
        ("zusammenveranlagung", Roeh::Bool(false)),
    ], 0_i128, Some("Euro->Cent")),

    ("Verlustvortrag p10d_2: GdE i64::MAX EUR, Bestand 0 (erstes Feld allein) [nur ein Zwischenprodukt ausserhalb i64]", "p10d_2", &[
        ("gesamtbetrag_einkuenfte", Roeh::Ganz(i64::MAX)),
        ("verlustvortrag_bestand", Roeh::Ganz(0)),
        ("zusammenveranlagung", Roeh::Bool(false)),
    ], 0_i128, Some("Euro->Cent")),

    ("Verlustvortrag p10d_2: GdE und Bestand 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "p10d_2", &[
        ("gesamtbetrag_einkuenfte", Roeh::Ganz(92_233_720_368_547_758)),
        ("verlustvortrag_bestand", Roeh::Ganz(92_233_720_368_547_758)),
        ("zusammenveranlagung", Roeh::Bool(false)),
    ], 64_563_604_258_283_430_i128, None),

    ("est_einzel: Bruttolohn 92.233.720.368.547.758 EUR (x 100 passt gerade) [passt gerade noch]", "est_pfad", &[
        ("bruttoarbeitslohn", Roeh::Ganz(92_233_720_368_547_758)),
    ], 41_505_174_165_826_674_i128, None),

    ("est_einzel: Bruttolohn i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_296_i128, Some("Euro->Cent")),

    ("est_einzel: Sonderausgaben i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn", Roeh::Ganz(0)),
        ("sonderausgaben", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("est_einzel: Werbungskosten i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn", Roeh::Ganz(0)),
        ("werbungskosten", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("est_zusammen: Bruttolohn A 92.233.720.368.547.758 EUR (Gegenprobe) [passt gerade noch]", "est_pfad", &[
        ("bruttoarbeitslohn_a", Roeh::Ganz(92_233_720_368_547_758)),
        ("bruttoarbeitslohn_b", Roeh::Ganz(0)),
        ("sonderausgaben_gemeinsam", Roeh::Ganz(0)),
        ("werbungskosten_a", Roeh::Ganz(0)),
        ("werbungskosten_b", Roeh::Ganz(0)),
    ], 41_505_174_165_807_410_i128, None),

    ("est_zusammen: Bruttolohn A i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn_a", Roeh::Ganz(i64::MAX)),
        ("bruttoarbeitslohn_b", Roeh::Ganz(0)),
        ("sonderausgaben_gemeinsam", Roeh::Ganz(0)),
        ("werbungskosten_a", Roeh::Ganz(0)),
        ("werbungskosten_b", Roeh::Ganz(0)),
    ], 4_150_517_416_584_610_032_i128, Some("Euro->Cent")),

    ("est_zusammen: Bruttolohn B i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn_a", Roeh::Ganz(0)),
        ("bruttoarbeitslohn_b", Roeh::Ganz(i64::MAX)),
        ("sonderausgaben_gemeinsam", Roeh::Ganz(0)),
        ("werbungskosten_a", Roeh::Ganz(0)),
        ("werbungskosten_b", Roeh::Ganz(0)),
    ], 4_150_517_416_584_610_032_i128, Some("Euro->Cent")),

    ("est_zusammen: Sonderausgaben gemeinsam i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn_a", Roeh::Ganz(0)),
        ("bruttoarbeitslohn_b", Roeh::Ganz(0)),
        ("sonderausgaben_gemeinsam", Roeh::Ganz(i64::MAX)),
        ("werbungskosten_a", Roeh::Ganz(0)),
        ("werbungskosten_b", Roeh::Ganz(0)),
    ], 0_i128, Some("Euro->Cent")),

    ("est_zusammen: Werbungskosten A i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn_a", Roeh::Ganz(0)),
        ("bruttoarbeitslohn_b", Roeh::Ganz(0)),
        ("sonderausgaben_gemeinsam", Roeh::Ganz(0)),
        ("werbungskosten_a", Roeh::Ganz(i64::MAX)),
        ("werbungskosten_b", Roeh::Ganz(0)),
    ], 0_i128, Some("Euro->Cent")),

    ("est_zusammen: Werbungskosten B i64::MAX EUR [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("bruttoarbeitslohn_a", Roeh::Ganz(0)),
        ("bruttoarbeitslohn_b", Roeh::Ganz(0)),
        ("sonderausgaben_gemeinsam", Roeh::Ganz(0)),
        ("werbungskosten_a", Roeh::Ganz(0)),
        ("werbungskosten_b", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("tarif: zvE i64::MAX EUR (x 100 ausserhalb, im Tarif-Aufruf) [nur ein Zwischenprodukt ausserhalb i64]", "est_pfad", &[
        ("zu_versteuerndes_einkommen", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_866_i128, Some("Euro->Cent")),

    ("gesamt Kapitalvermoegen 92.233.720.368.547.758 EUR (Gegenprobe) [passt gerade noch]", "gesamt_pfad", &[
        ("einkuenfte_kapitalvermoegen", Roeh::Ganz(92_233_720_368_547_758)),
    ], 41_505_174_165_827_228_i128, None),

    ("gesamt Sonderausgaben X und Vorsorge i64::MAX (Summe nur in i128, N22) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("sonderausgaben", Roeh::Ganz(92_233_720_368_547_759)),
        ("vorsorge_gesamtbeitraege_inkl_ag", Roeh::Ganz(92_233_720_368_547_759)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt Sonderausgaben i64::MAX EUR (x 100 nach 10c-Floor, N17/N22) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("sonderausgaben", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt altersentlastungsbetrag: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("altersentlastungsbetrag", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt anzurechnende_auslaendische_steuern: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("anzurechnende_auslaendische_steuern", Roeh::Ganz(i64::MAX)),
    ], -9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("gesamt aussergewoehnliche_belastungen: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("aussergewoehnliche_belastungen", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt einkuenfte_gewinn: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("einkuenfte_gewinn", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_850_i128, Some("Euro->Cent")),

    ("gesamt einkuenfte_kapitalvermoegen: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("einkuenfte_kapitalvermoegen", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_850_i128, Some("Euro->Cent")),

    ("gesamt einkuenfte_nichtselbststaendig i64::MAX EUR (N21: Summenpfad) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("einkuenfte_nichtselbststaendig", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_850_i128, Some("Euro->Cent")),

    ("gesamt einkuenfte_sonstige: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("einkuenfte_sonstige", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_850_i128, Some("Euro->Cent")),

    ("gesamt einkuenfte_vermietung: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("einkuenfte_vermietung", Roeh::Ganz(i64::MAX)),
    ], 4_150_517_416_584_629_850_i128, Some("Euro->Cent")),

    ("gesamt entlastungsbetrag_alleinerziehende: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("entlastungsbetrag_alleinerziehende", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt freibetraege_kinder: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("freibetraege_kinder", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt hinzurechnung_kindergeld i64::MAX EUR (N23: direkter Wert) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("hinzurechnung_kindergeld", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("gesamt hinzurechnung_zulage: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("hinzurechnung_zulage", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("gesamt sonstige_abzuege_vom_einkommen: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("sonstige_abzuege_vom_einkommen", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt steuer_kapital_gesondert: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("steuer_kapital_gesondert", Roeh::Ganz(i64::MAX)),
    ], 9_223_372_036_854_775_807_i128, Some("Euro->Cent")),

    ("gesamt steuerermaessigungen: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("steuerermaessigungen", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt tarifliche_est_modifiziert: i64::MAX EUR (x 100 ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("tarifliche_est_modifiziert", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("Euro->Cent")),

    ("gesamt: Sonderausgaben i64::MAX EUR plus Vorsorge (Summe im 10c-Floor ausserhalb) [nur ein Zwischenprodukt ausserhalb i64]", "gesamt_pfad", &[
        ("sonderausgaben", Roeh::Ganz(i64::MAX)),
        ("vorsorge_gesamtbeitraege_inkl_ag", Roeh::Ganz(i64::MAX)),
    ], 0_i128, Some("teil2 i64")),

    ("Kapitalverrechnung: Aktienstamm i64::MAX, Sonstige 5 EUR (Summe i64::MAX + 5) [Python-Wert ausserhalb i64]", "kapital_verrechnung", &[
        ("gewinn_aktien", Roeh::Ganz(i64::MAX)),
        ("gewinn_sonstige", Roeh::Ganz(5)),
        ("verlust_aktien", Roeh::Ganz(0)),
        ("verlust_sonstige", Roeh::Ganz(0)),
    ], 9_223_372_036_854_775_812_i128, Some("teil2 i64")),

    ("Kapitalverrechnung: Aktienstamm und Sonstige je i64::MAX EUR (Summe nur in i128) [Python-Wert ausserhalb i64]", "kapital_verrechnung", &[
        ("gewinn_aktien", Roeh::Ganz(i64::MAX)),
        ("gewinn_sonstige", Roeh::Ganz(i64::MAX)),
        ("verlust_aktien", Roeh::Ganz(0)),
        ("verlust_sonstige", Roeh::Ganz(0)),
    ], 18_446_744_073_709_551_614_i128, Some("teil2 i64")),

];

fn params() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap())
}

fn eu(w: &[Eingabe], k: &str) -> Euro {
    Euro::new(zi(w, k))
}

fn ce(w: &[Eingabe], k: &str) -> Cent {
    Cent::new(zi(w, k))
}

fn zi(w: &[Eingabe], k: &str) -> i64 {
    match w.iter().find(|(f, _)| *f == k).map(|(_, v)| v) {
        Some(Roeh::Ganz(n)) => *n,
        Some(Roeh::Bool(b)) => i64::from(*b),
        None => 0,
        Some(_) => panic!("{k}: keine Zahl"),
    }
}

fn te(w: &[Eingabe], k: &str) -> String {
    match w.iter().find(|(f, _)| *f == k).map(|(_, v)| v) {
        Some(Roeh::Text(s)) => (*s).to_owned(),
        None => String::new(),
        Some(_) => panic!("{k}: kein Text"),
    }
}

fn wa(w: &[Eingabe], k: &str) -> bool {
    matches!(w.iter().find(|(f, _)| *f == k).map(|(_, v)| v), Some(Roeh::Bool(true)))
}

/// Teil-1-Fehler als Marke: Ueberlauf als `Ueberlauf(marke)`, sonst Debug-Form.
fn teil1_marker(e: EngineFehler) -> String {
    match e {
        EngineFehler::Ueberlauf(m) => format!("Ueberlauf({m})"),
        other => format!("{other:?}"),
    }
}

/// Teil-2-Fehler: die Basis-Variante traegt den Teil-1-Fehler, die anderen ihre eigene Debug-Form.
fn teil2_marker(e: engine::zugriff::teil2::EngineFehler) -> String {
    match e {
        engine::zugriff::teil2::EngineFehler::Basis(b) => teil1_marker(b),
        other => format!("{other:?}"),
    }
}

fn t2(r: Result<Euro, engine::zugriff::teil2::EngineFehler>) -> Result<i128, String> {
    r.map(|x| i128::from(x.get())).map_err(teil2_marker)
}

fn t2c(r: Result<Cent, engine::zugriff::teil2::EngineFehler>) -> Result<i128, String> {
    r.map(|x| i128::from(x.get())).map_err(teil2_marker)
}

fn run(fnname: &str, w: &[Eingabe]) -> Result<i128, String> {
    let e = |r: Result<Euro, EngineFehler>| r.map(|x| i128::from(x.get())).map_err(teil1_marker);
    let c = |r: Result<Cent, EngineFehler>| r.map(|x| i128::from(x.get())).map_err(teil1_marker);
    match fnname {
        "p33_zumutbar" => e(p33_zumutbar(&P33ZumutbarEingabe {
            gesamtbetrag_der_einkuenfte: eu(w, "gesamtbetrag_der_einkuenfte"),
            anzahl_kinder: zi(w, "anzahl_kinder"),
            splitting: wa(w, "splitting"),
        })),
        "p33_agb" => e(p33_agb(&P33AgbEingabe {
            aussergewoehnliche_belastungen: eu(w, "aussergewoehnliche_belastungen"),
            zumutbar: P33ZumutbarEingabe {
                gesamtbetrag_der_einkuenfte: eu(w, "gesamtbetrag_der_einkuenfte"),
                anzahl_kinder: zi(w, "anzahl_kinder"),
                splitting: wa(w, "splitting"),
            },
        })),
        "p10b_spenden" => e(p10b_spenden(&P10bSpendenEingabe {
            zuwendungen: eu(w, "zuwendungen"),
            gesamtbetrag_der_einkuenfte: eu(w, "gesamtbetrag_der_einkuenfte"),
        })),
        "p10_kist" => e(p10_kist(&P10KistEingabe {
            gezahlte_kirchensteuer: eu(w, "gezahlte_kirchensteuer"),
            erstattete_kirchensteuer: eu(w, "erstattete_kirchensteuer"),
        })),
        "p10_4b_erstattungsueberhang" => e(p10_4b_erstattungsueberhang(&P10KistEingabe {
            gezahlte_kirchensteuer: eu(w, "gezahlte_kirchensteuer"),
            erstattete_kirchensteuer: eu(w, "erstattete_kirchensteuer"),
        })),
        "p10_kv_pv" => e(p10_kv_pv(&P10KvPvEingabe {
            basis_kv_pv: eu(w, "basis_kv_pv"),
            weitere_vorsorgeaufwendungen: eu(w, "weitere_vorsorgeaufwendungen"),
            mit_anspruch_auf_zuschuss: wa(w, "mit_anspruch_auf_zuschuss"),
        })),
        "p10_1_7_berufsausbildung" => e(p10_1_7_berufsausbildung(&P1017BerufsausbildungEingabe {
            berufsausbildung_aufwendungen: eu(w, "berufsausbildung_aufwendungen"),
        })),
        "p35a_haushaltsnahe" => e(p35a_haushaltsnahe(&P35aHaushaltsnaheEingabe {
            hh_minijob_aufwendungen: eu(w, "hh_minijob_aufwendungen"),
            hh_dienstleistungen: eu(w, "hh_dienstleistungen"),
            hh_handwerker_arbeitskosten: eu(w, "hh_handwerker_arbeitskosten"),
            hh_in_eu_ewr: wa(w, "hh_in_eu_ewr"),
            hh_rechnung_unbar: wa(w, "hh_rechnung_unbar"),
            hh_handwerker_gefoerdert: wa(w, "hh_handwerker_gefoerdert"),
            p35a_mitveranlagung: wa(w, "p35a_mitveranlagung"),
        })),
        "kist" => c(kist(&KistEingabe {
            konfession: te(w, "konfession"),
            bundesland: te(w, "bundesland"),
            est_mit_fb: eu(w, "est_mit_fb"),
        })),
        "p36_abschlusszahlung" => c(p36_abschlusszahlung(&P36AbschlusszahlungEingabe {
            festzusetzende_est_cent: ce(w, "festzusetzende_est_cent"),
            lohnsteuer_cent: ce(w, "lohnsteuer_cent"),
            kapitalertragsteuer_cent: ce(w, "kapitalertragsteuer_cent"),
            kapitalertragsteuer_solz_cent: ce(w, "kapitalertragsteuer_solz_cent"),
            kapitalertragsteuer_kist_cent: ce(w, "kapitalertragsteuer_kist_cent"),
            vorauszahlungen_cent: ce(w, "vorauszahlungen_cent"),
        })),
        "p24a_altersentlastung" => e(p24a_altersentlastung(&P24aAltersentlastungEingabe {
            veranlagungszeitraum: 2025,
            geburtsjahr: zi(w, "geburtsjahr"),
            arbeitslohn: eu(w, "arbeitslohn"),
            positive_andere_einkuenfte: eu(w, "positive_andere_einkuenfte"),
        }, params())),
        "p31_familienleistung" => e(p31_familienleistung(&P31FamilienleistungEingabe {
            est_ohne_freibetraege: eu(w, "est_ohne_freibetraege"),
            est_mit_freibetraegen: eu(w, "est_mit_freibetraegen"),
            kindergeld: eu(w, "kindergeld"),
        })),
        "einkuenfte_nichtselbststaendig" => e(einkuenfte_nichtselbststaendig(
            &EinkuenfteNichtselbststaendigEingabe {
                bruttoarbeitslohn: eu(w, "bruttoarbeitslohn"),
                werbungskosten: eu(w, "werbungskosten"),
                veranlagungszeitraum: Vz::Vz2025,
            })),
        "p21_2_verbilligt" => e(p21_2_verbilligt(&P212VerbilligtEingabe {
            werbungskosten: eu(w, "werbungskosten"),
            entgelt_quote_prozent: zi(w, "entgelt_quote_prozent"),
        })),
        "p16_4_freibetrag" => e(p16_4_freibetrag(&P164FreibetragEingabe {
            rentner_veraeusserungsgewinn: eu(w, "rentner_veraeusserungsgewinn"),
        })),
        "euer_gewinn" => e(euer_gewinn(&EuerGewinnEingabe {
            betriebseinnahmen: eu(w, "betriebseinnahmen"),
            betriebsausgaben: eu(w, "betriebsausgaben"),
        })),
        "mitunternehmer_einkuenfte" => e(mitunternehmer_einkuenfte(&
            MitunternehmerEinkuenfteEingabe {
                gewinnanteil: eu(w, "gewinnanteil"),
                verguetung_taetigkeit: eu(w, "verguetung_taetigkeit"),
                verguetung_darlehen: eu(w, "verguetung_darlehen"),
                verguetung_ueberlassung: eu(w, "verguetung_ueberlassung"),
            })),
        "p6_2_gwg" => e(p6_2_gwg(&P62GwgEingabe {
            gwg_anschaffungskosten_netto: eu(w, "gwg_anschaffungskosten_netto"),
        })),
        "entfernungspauschale" => e(entfernungspauschale(&EntfernungspauschaleEingabe {
            veranlagungszeitraum: Vz::Vz2025,
            entfernung_km_roh: Km::new(Decimal::from_str(&te(w, "entfernung_km_roh")).unwrap()),
            arbeitstage: zi(w, "arbeitstage"),
            eigenes_oder_ueberlassenes_kfz: wa(w, "eigenes_oder_ueberlassenes_kfz"),
            oepnv_kosten_jahr: eu(w, "oepnv_kosten_jahr"),
        }, params())),
        "raumkosten" => e(raumkosten(&RaumkostenEingabe {
            veranlagungszeitraum: Vz::Vz2025,
            arbeitszimmer_vorhanden: wa(w, "arbeitszimmer_vorhanden"),
            ist_mittelpunkt: wa(w, "ist_mittelpunkt"),
            tatsaechliche_aufwendungen: eu(w, "tatsaechliche_aufwendungen"),
            jahrespauschale_gewaehlt: wa(w, "jahrespauschale_gewaehlt"),
            monate_ohne_mittelpunkt: zi(w, "monate_ohne_mittelpunkt"),
            homeoffice_tage: zi(w, "homeoffice_tage"),
        }, params())),
        "p101_mobilitaetspraemie" => e(p101_mobilitaetspraemie(&P101Eingabe {
            entfernungspauschale_ab_21km: eu(w, "entfernungspauschale_ab_21km"),
            zu_versteuerndes_einkommen: eu(w, "zu_versteuerndes_einkommen"),
            grundfreibetrag: eu(w, "grundfreibetrag"),
            ist_arbeitnehmer: wa(w, "ist_arbeitnehmer"),
            werbungskosten_gesamt: eu(w, "werbungskosten_gesamt"),
            arbeitnehmer_pauschbetrag: eu(w, "arbeitnehmer_pauschbetrag"),
        })),
        "p101_mobilitaetspraemie_cent" => c(p101_mobilitaetspraemie_cent(&P101Eingabe {
            entfernungspauschale_ab_21km: eu(w, "entfernungspauschale_ab_21km"),
            zu_versteuerndes_einkommen: eu(w, "zu_versteuerndes_einkommen"),
            grundfreibetrag: eu(w, "grundfreibetrag"),
            ist_arbeitnehmer: wa(w, "ist_arbeitnehmer"),
            werbungskosten_gesamt: eu(w, "werbungskosten_gesamt"),
            arbeitnehmer_pauschbetrag: eu(w, "arbeitnehmer_pauschbetrag"),
        })),
        "fuenftel" => t2(fuenftel(&FuenftelEingabe {
            vz: Vz::Vz2025,
            veranlagung: Veranlagung::Einzel,
            zu_versteuerndes_einkommen: eu(w, "zu_versteuerndes_einkommen"),
            ausserordentliche_einkuenfte: eu(w, "ausserordentliche_einkuenfte"),
        })),
        "solz" => t2c(solz(&SolzEingabe {
            vz: Vz::Vz2025,
            bemessungsgrundlage: eu(w, "bemessungsgrundlage"),
            kapital_steuer: eu(w, "kapital_steuer"),
            splitting: wa(w, "splitting"),
        })),
        "kfz_nutzungswert_monat_cent" => t2c(kfz_nutzungswert_monat_cent(&KfzNutzungswertEingabe {
            bruttolistenpreis: eu(w, "bruttolistenpreis"),
            bruchteils_teiler: zi(w, "bruchteils_teiler"),
        })),
        "ermaessigter_durchschnittssatz" => t2(ermaessigter_durchschnittssatz(&DurchschnittssatzEingabe {
            ao_einkuenfte: eu(w, "ao_einkuenfte"),
            est_gesamt_zzgl_progression: eu(w, "est_gesamt_zzgl_progression"),
            bemessungsgrundlage_durchschnitt: eu(w, "bemessungsgrundlage_durchschnitt"),
        })),
        "p10d_2" => t2(p10d_2(&VerlustabzugEingabe {
            gesamtbetrag_einkuenfte: eu(w, "gesamtbetrag_einkuenfte"),
            verlustvortrag_bestand: eu(w, "verlustvortrag_bestand"),
            zusammenveranlagung: wa(w, "zusammenveranlagung"),
        })),
        "est_pfad" if w.iter().any(|(f, _)| *f == "bruttoarbeitslohn_a") => t2(est_zusammen(&EstZusammenEingabe {
            vz: Vz::Vz2025,
            bruttoarbeitslohn_a: eu(w, "bruttoarbeitslohn_a"),
            werbungskosten_a: eu(w, "werbungskosten_a"),
            bruttoarbeitslohn_b: eu(w, "bruttoarbeitslohn_b"),
            werbungskosten_b: eu(w, "werbungskosten_b"),
            sonderausgaben_gemeinsam: eu(w, "sonderausgaben_gemeinsam"),
        })),
        "est_pfad" if w.iter().any(|(f, _)| *f == "bruttoarbeitslohn") => t2(est_einzel(&EstEinzelEingabe {
            vz: Vz::Vz2025,
            bruttoarbeitslohn: eu(w, "bruttoarbeitslohn"),
            werbungskosten: eu(w, "werbungskosten"),
            sonderausgaben: eu(w, "sonderausgaben"),
        })),
        "est_pfad" => t2(tarif_est(&TarifEingabe {
            vz: Vz::Vz2025,
            veranlagung: if te(w, "veranlagung") == "zusammen" { Veranlagung::Zusammen } else { Veranlagung::Einzel },
            zu_versteuerndes_einkommen: eu(w, "zu_versteuerndes_einkommen"),
        })),
        "gesamt_pfad" => t2(gesamt(&GesamtfallEingabe {
            vz: Vz::Vz2025,
            zusammenveranlagung: wa(w, "zusammenveranlagung"),
            einkuenfte_nichtselbststaendig: eu(w, "einkuenfte_nichtselbststaendig"),
            einkuenfte_kapitalvermoegen: eu(w, "einkuenfte_kapitalvermoegen"),
            einkuenfte_vermietung: eu(w, "einkuenfte_vermietung"),
            einkuenfte_sonstige: eu(w, "einkuenfte_sonstige"),
            einkuenfte_gewinn: eu(w, "einkuenfte_gewinn"),
            altersentlastungsbetrag: eu(w, "altersentlastungsbetrag"),
            entlastungsbetrag_alleinerziehende: eu(w, "entlastungsbetrag_alleinerziehende"),
            sonderausgaben: eu(w, "sonderausgaben"),
            vorsorge_gesamtbeitraege_inkl_ag: eu(w, "vorsorge_gesamtbeitraege_inkl_ag"),
            vorsorge_ag_anteil_steuerfrei: eu(w, "vorsorge_ag_anteil_steuerfrei"),
            aussergewoehnliche_belastungen: eu(w, "aussergewoehnliche_belastungen"),
            freibetraege_kinder: eu(w, "freibetraege_kinder"),
            sonstige_abzuege_vom_einkommen: eu(w, "sonstige_abzuege_vom_einkommen"),
            anzurechnende_auslaendische_steuern: eu(w, "anzurechnende_auslaendische_steuern"),
            steuerermaessigungen: eu(w, "steuerermaessigungen"),
            steuer_kapital_gesondert: eu(w, "steuer_kapital_gesondert"),
            hinzurechnung_kindergeld: eu(w, "hinzurechnung_kindergeld"),
            kinder_ganzjaehrig: zi(w, "kinder_ganzjaehrig"),
            hinzurechnung_zulage: eu(w, "hinzurechnung_zulage"),
            tarif_modifiziert: wa(w, "tarif_modifiziert"),
            tarifliche_est_modifiziert: eu(w, "tarifliche_est_modifiziert"),
            versorgung: EinkuenfteVersorgungEingabe {
                versorgung_jahresrente: Euro::new(0),
                freibetrag: VersorgungsfreibetragEingabe { bemessungsgrundlage: Euro::new(0), beginn_jahr: 0 },
            },
        }, params())),
        "kapital_verrechnung" => t2(kapital_verrechnung(&KapitalVerrechnungEingabe {
            gewinn_aktien: eu(w, "gewinn_aktien"),
            verlust_aktien: eu(w, "verlust_aktien"),
            gewinn_sonstige: eu(w, "gewinn_sonstige"),
            verlust_sonstige: eu(w, "verlust_sonstige"),
        })),
        other => panic!("unbekannte Funktion {other}"),
    }
}

#[test]
fn euro_nach_cent_und_guards_melden_ueberlauf_je_fundstelle() {
    let mut falsch = Vec::new();
    for (name, fnname, w, py, marke) in FAELLE {
        let ist = run(fnname, w);
        let meldung = match (*marke, &ist) {
            (Some(m), Err(s)) if s == &format!("Ueberlauf({m})") => None,
            (Some(m), Err(e)) => Some(format!("Ueberlauf({m}) erwartet, gekommen {e:?}")),
            (Some(m), Ok(v)) => Some(format!("Ueberlauf({m}) erwartet, gekommen Zahl {v}")),
            (None, Err(e)) => Some(format!("Wert {py} erwartet, gekommen {e:?}")),
            (None, Ok(v)) if *v == *py => None,
            (None, Ok(v)) => Some(format!("Wert {py} erwartet, gekommen {v}")),
        };
        if let Some(m) = meldung {
            falsch.push(format!("{name}: {m}"));
        }
    }
    assert_eq!(FAELLE.len(), 108, "die Zahl der Faelle hat sich verschoben");
    assert_eq!(FAELLE.iter().filter(|f| f.4.is_none()).count(), 29, "die Zahl der Gegenproben hat sich verschoben");
    let klar = FAELLE.iter().filter(|f| f.3 > GRENZE_OBEN || f.3 < GRENZE_UNTEN).count();
    assert_eq!(klar, 8, "die Zahl der klaren Python-Ueberlaeufe hat sich verschoben");
    assert!(falsch.is_empty(), "{falsch:#?}");
}
