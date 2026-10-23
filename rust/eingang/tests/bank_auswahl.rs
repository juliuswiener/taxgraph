//! Bankimport S3 (Vault `backlog/taxgraph/bankimport-s3-oberflaeche-vorschlaege-pruefen-mit-haken-und-kreuz-abwahl`): die Nutzerin
//! waehlt in der Vorschau Buchungen ab. Der Server rechnet aus derselben Datei neu; der Browser schickt nur die Kennungen.
//!
//! Die Kennung einer Buchung ist ihre Stelle in der gelesenen Liste, aus Datei und Format allein: sie haengt weder von den Haken
//! noch von den Regeln ab (`index_*`). `Ergebnis::zeilen` nennt jede Buchung, die in eine Summe eingehen kann, auch die
//! abgewaehlten; eine Buchung ohne Zeile (Nachfrage, belegtes Feld, ohne Stichwort) kann man nicht abwaehlen.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung, jeder Test mit eigenen Betraegen und eigenen Stellen. Die Buchungstexte sind erfunden.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::Cell;

use eingang::kontoauszug::{
    uebernehme_auswahl, uebernehme_voll, Ergebnis, FeldErgebnis, Folge, Haken, Klassifikator, Transaktion, Uebernahme,
};
use eingang::vorschlag::{Quelle, VorschlagEvent};
use llm::pii::Maskiert;
use llm::Kategorie;
use serde_json::{json, Value};
use store::{BindungNachschlag, Katalog, Store};

fn tx(datum: &str, betrag: i64, zweck: &str) -> Transaktion {
    Transaktion {
        datum: json!(datum),
        betrag,
        verwendungszweck: zweck.to_owned(),
    }
}

fn lauf(store: &mut Store, buchungen: &[Transaktion], abgewaehlt: &[usize], haken: Haken) -> Ergebnis {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    uebernehme_auswahl(store, buchungen, abgewaehlt, nachschlag, None, (None, None), haken).unwrap()
}

fn frisch(buchungen: &[Transaktion], abgewaehlt: &[usize], haken: Haken) -> (Store, Ergebnis) {
    let mut store = Store::leer(2025, None);
    let erg = lauf(&mut store, buchungen, abgewaehlt, haken);
    (store, erg)
}

/// Das aktive Ereignis des Felds als JSON.
fn ereignis(store: &Store, feld: &str) -> Option<Value> {
    let (_, e) = store.aktive().find(|(f, _)| *f == feld)?;
    Some(serde_json::to_value(e).unwrap())
}

fn wert(store: &Store, feld: &str) -> Option<i64> {
    ereignis(store, feld)?["wert"].as_i64()
}

fn posten_betraege(store: &Store, feld: &str) -> Vec<i64> {
    ereignis(store, feld).unwrap()["signal"]["signal_1"]["posten"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["betrag_cent"].as_i64().unwrap())
        .collect()
}

/// Die Summe aller Zaehler: jede Buchung steht in genau einem.
fn zaehler_summe(u: &Uebernahme) -> usize {
    u.uebernommen
        + u.llm_uebersprungen
        + u.einnahmen
        + u.ohne_stichwort
        + u.partei
        + u.feld_fehlt
        + u.feld_belegt
        + u.anderes_jahr
        + u.datum_unlesbar
        + u.saldo_nicht_positiv
        + u.ersetzbar
        + u.schon_erfasst
        + u.nachgefragt
        + u.art9_gesperrt
        + u.abgewaehlt
}

fn spenden(betraege: &[i64]) -> Vec<Transaktion> {
    betraege
        .iter()
        .enumerate()
        .map(|(i, b)| tx(&format!("2025-03-{:02}", i + 1), -b, &format!("Spende Tierheim Nr {i}")))
        .collect()
}

fn feld(erg: &Ergebnis, feld_id: &str) -> Option<FeldErgebnis> {
    erg.felder.iter().find(|f| f.feld_id == feld_id).cloned()
}

/// Die Kennung haengt von Datei und Format ab, nicht von den Haken: dieselbe Datei mit und ohne `art9` nennt fuer dieselben
/// Buchungen dieselbe Stelle. Mit `art9` kommt die Zeile der Krankenversicherung dazu (Stelle 2), ohne ist sie keine Zeile.
#[test]
fn bank_auswahl_index_ist_die_stelle_in_der_liste_bei_haken_an_und_aus() {
    let buchungen = [
        tx("2025-03-01", 300_000, "Gehalt Muster GmbH"),
        tx("2025-03-02", -11_00, "Spende Tierheim"),
        tx("2025-03-03", -37_00, "Krankenversicherung Muster Kasse"),
        tx("2025-03-04", -73_00, "Maler Huber"),
        tx("2025-03-05", -23_00, "Spende Unicef"),
        tx("2025-03-06", -50_000, "Miete Muster"),
    ];
    let (_, ohne) = frisch(&buchungen, &[], Haken { ersetzen: false, art9: false });
    let (_, mit) = frisch(&buchungen, &[], Haken { ersetzen: false, art9: true });
    let stellen = |e: &Ergebnis| e.zeilen.iter().map(|z| (z.index, z.feld_id)).collect::<Vec<_>>();
    assert_eq!(stellen(&ohne), [(1, "spenden_betrag"), (4, "spenden_betrag")], "{:?}", ohne.zeilen);
    assert_eq!(
        stellen(&mit),
        [(1, "spenden_betrag"), (2, "basis_kv"), (4, "spenden_betrag")],
        "{:?}",
        mit.zeilen
    );
    // Die Buchung der Art.-9-Regel traegt keinen Zweck in der Zeile, die Spende ihren maskierten.
    assert_eq!(mit.zeilen[1].zweck_maskiert, "");
    assert!(mit.zeilen[0].zweck_maskiert.contains("Tierheim"), "{:?}", mit.zeilen[0]);
}

/// Eine abgewaehlte Buchung geht nicht in die Summe ein, die uebrigen schon; die Zeile bleibt mit `abgewaehlt` stehen.
#[test]
fn bank_auswahl_abgewaehlter_posten_zaehlt_nicht_in_die_summe() {
    let (store, erg) = frisch(&spenden(&[10_000, 20_000, 40_000]), &[1], Haken::default());
    assert_eq!(wert(&store, "spenden_betrag"), Some(50_000), "{erg:?}");
    assert_eq!(posten_betraege(&store, "spenden_betrag"), [-10_000, -40_000]);
    assert_eq!((erg.uebernahme.uebernommen, erg.uebernahme.abgewaehlt), (2, 1), "{:?}", erg.uebernahme);
    let flags: Vec<(usize, bool)> = erg.zeilen.iter().map(|z| (z.index, z.abgewaehlt)).collect();
    assert_eq!(flags, [(0, false), (1, true), (2, false)]);
    let f = feld(&erg, "spenden_betrag").unwrap();
    assert_eq!((f.summe_cent, f.anzahl, f.folge), (50_000, 2, Folge::Geschrieben));
}

/// Die Raender: die erste und die letzte Stelle gelten wie jede andere (Mutant: eine Schleife, die bei 1 beginnt oder vor dem Ende
/// aufhoert).
#[test]
fn bank_auswahl_erste_und_letzte_stelle_sind_abwaehlbar() {
    let buchungen = spenden(&[10_000, 20_000, 40_000]);
    let (erste, _) = frisch(&buchungen, &[0], Haken::default());
    assert_eq!(wert(&erste, "spenden_betrag"), Some(60_000));
    let (letzte, _) = frisch(&buchungen, &[2], Haken::default());
    assert_eq!(wert(&letzte, "spenden_betrag"), Some(30_000));
    let (beide, _) = frisch(&buchungen, &[0, 2], Haken::default());
    assert_eq!(wert(&beide, "spenden_betrag"), Some(20_000));
}

/// Genau eine Buchung im Feld: ohne Abwahl ein Vorschlag, mit Abwahl keiner (die Gruppe ist leer, nichts wird geschrieben).
#[test]
fn bank_auswahl_genau_ein_posten_und_alle_abgewaehlt_schreiben_nichts() {
    let eine = spenden(&[75_00]);
    let (store, erg) = frisch(&eine, &[], Haken::default());
    assert_eq!(wert(&store, "spenden_betrag"), Some(75_00), "Kontrolle: ohne Abwahl wird geschrieben");
    assert_eq!(erg.uebernahme.uebernommen, 1);

    let (store, erg) = frisch(&eine, &[0], Haken::default());
    assert_eq!(wert(&store, "spenden_betrag"), None, "{erg:?}");
    assert_eq!((erg.uebernahme.uebernommen, erg.uebernahme.abgewaehlt), (0, 1));
    assert!(erg.felder.is_empty(), "{:?}", erg.felder);
    assert_eq!(erg.zeilen.len(), 1, "die abgewaehlte Buchung bleibt als Zeile stehen");
    assert!(erg.zeilen[0].abgewaehlt);

    let (store, erg) = frisch(&spenden(&[10_00, 20_00, 30_00]), &[0, 1, 2], Haken::default());
    assert_eq!(wert(&store, "spenden_betrag"), None);
    assert_eq!((erg.uebernahme.uebernommen, erg.uebernahme.abgewaehlt), (0, 3));
}

/// Eine Kennung ohne Zeile bleibt ohne Wirkung: eine Nachfrage (§ 35a) bleibt eine Nachfrage, eine Einnahme eine Einnahme, und eine
/// Stelle jenseits der Liste aendert nichts.
#[test]
fn bank_auswahl_kennung_ohne_zeile_bleibt_ohne_wirkung() {
    let buchungen = [
        tx("2025-03-01", -73_00, "Maler Huber"),
        tx("2025-03-02", 90_000, "Gehalt Muster GmbH"),
        tx("2025-03-03", -41_00, "Miete Muster"),
        tx("2025-03-04", -9_00, "Spende Tierheim"),
    ];
    let (store, erg) = frisch(&buchungen, &[0, 1, 2, 99], Haken::default());
    let u = erg.uebernahme;
    assert_eq!((u.nachgefragt, u.einnahmen, u.ohne_stichwort, u.abgewaehlt, u.uebernommen), (1, 1, 1, 0, 1), "{u:?}");
    assert_eq!(erg.nachfragen.len(), 1);
    assert_eq!(wert(&store, "spenden_betrag"), Some(9_00));
    assert_eq!(erg.zeilen.len(), 1, "nur die Spende ist eine Zeile");
}

/// Ein Feld mit einem Beleg ist keine Zeile (der Kontoauszug schreibt dort nichts): auch die Abwahl aendert nichts, der Abgleich
/// nennt die Summe weiter.
#[test]
fn bank_auswahl_belegtes_feld_hat_keine_zeile_und_die_abwahl_aendert_nichts() {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let katalog = Katalog::aus_bindungen(nachschlag.alle().map(|(_, b)| b));
    let mut store = Store::leer(2025, None);
    VorschlagEvent {
        quelle: Quelle::Beleg,
        feld_id: "spenden_betrag".to_owned(),
        wert: json!(25_00),
        signal_1: json!({"typ": "beleg"}),
    }
    .schreibe(&mut store, Some(&katalog), nachschlag, None)
    .unwrap();
    let erg = lauf(&mut store, &spenden(&[15_00]), &[0], Haken::default());
    assert!(erg.zeilen.is_empty(), "{:?}", erg.zeilen);
    assert_eq!((erg.uebernahme.feld_belegt, erg.uebernahme.abgewaehlt), (1, 0), "{:?}", erg.uebernahme);
    assert_eq!(erg.abgleich.len(), 1);
    assert_eq!(erg.abgleich[0].summe_cent, 15_00);
}

/// Die Folge je Feld: ersetzbar (eigener Vorschlag aus einem frueheren Auszug, ohne Haken), schon erfasst (die Abwahl macht den
/// Auszug zu einer Teilmenge des bisherigen), Saldo nicht positiv.
#[test]
fn bank_auswahl_folge_je_feld_ersetzbar_schon_erfasst_saldo() {
    let mut store = Store::leer(2025, None);
    lauf(&mut store, &spenden(&[4_00]), &[], Haken::default());
    assert_eq!(wert(&store, "spenden_betrag"), Some(4_00));
    // Ein zweiter Auszug mit derselben Buchung (Stelle 0) und einer neuen (Stelle 1).
    let zweiter = spenden(&[4_00, 5_00]);
    let erg = lauf(&mut store, &zweiter, &[], Haken::default());
    let f = feld(&erg, "spenden_betrag").unwrap();
    assert_eq!((f.summe_cent, f.anzahl, f.folge), (9_00, 2, Folge::Ersetzbar), "{erg:?}");
    assert_eq!(wert(&store, "spenden_betrag"), Some(4_00), "ohne Haken schreibt der zweite Auszug nichts");
    // Wird die neue Buchung abgewaehlt, bleibt nur die schon erfasste: keine Rueckfrage.
    let erg = lauf(&mut store, &zweiter, &[1], Haken::default());
    let f = feld(&erg, "spenden_betrag").unwrap();
    assert_eq!(f.folge, Folge::SchonErfasst, "{erg:?}");
    assert_eq!((erg.uebernahme.schon_erfasst, erg.uebernahme.ersetzbar, erg.uebernahme.abgewaehlt), (1, 0, 1));
    // Mit dem Haken `ersetzen` wird geschrieben.
    let erg = lauf(&mut store, &zweiter, &[], Haken { ersetzen: true, art9: false });
    assert_eq!(feld(&erg, "spenden_betrag").unwrap().folge, Folge::Geschrieben);
    assert_eq!(wert(&store, "spenden_betrag"), Some(9_00));

    // Eine Rueckbuchung gleicht die Spende aus: Saldo null, kein Vorschlag, beide Buchungen sind Zeilen.
    let rueck = [tx("2025-04-01", -10_00, "Spende Tierheim"), tx("2025-04-02", 10_00, "Spende Tierheim Erstattung")];
    let (store, erg) = frisch(&rueck, &[], Haken::default());
    assert_eq!(wert(&store, "spenden_betrag"), None);
    assert_eq!(feld(&erg, "spenden_betrag").unwrap().folge, Folge::SaldoNichtPositiv, "{erg:?}");
    assert_eq!(erg.zeilen.iter().map(|z| z.betrag_cent).collect::<Vec<_>>(), [-10_00, 10_00]);
}

/// Jede Buchung steht in genau einem Zaehler, auch mit der Abwahl.
#[test]
fn bank_auswahl_jede_buchung_zaehlt_in_genau_einem_grund() {
    let buchungen = [
        tx("2025-03-01", -11_00, "Spende Tierheim"),
        tx("2025-03-02", -12_00, "Spende Unicef"),
        tx("2025-03-03", 70_000, "Gehalt Muster GmbH"),
        tx("2025-03-04", -73_00, "Maler Huber"),
        tx("2025-03-05", -5_00, "Miete Muster"),
        tx("2024-03-05", -6_00, "Spende Tierheim"),
        tx("2025-03-07", -37_00, "Krankenversicherung Muster Kasse"),
        tx("2025-03-08", -13_00, "Spende Stiftung Muster"),
    ];
    for abgewaehlt in [&[][..], &[0], &[1, 7], &[0, 1, 7]] {
        let (_, erg) = frisch(&buchungen, abgewaehlt, Haken::default());
        assert_eq!(zaehler_summe(&erg.uebernahme), buchungen.len(), "abgewaehlt {abgewaehlt:?}: {:?}", erg.uebernahme);
        assert_eq!(erg.uebernahme.abgewaehlt, abgewaehlt.len(), "{:?}", erg.uebernahme);
    }
}

/// Ohne Abwahl ist `uebernehme_auswahl` dasselbe wie `uebernehme_voll`: gleiche Ereignisse, gleiche Zaehler.
#[test]
fn bank_auswahl_ohne_abwahl_ist_uebernehme_voll() {
    let buchungen = [
        tx("2025-03-01", -11_00, "Spende Tierheim"),
        tx("2025-03-02", -73_00, "Maler Huber"),
        tx("2025-03-03", -24_00, "Rentenversicherung Ruerup"),
    ];
    let (a, ea) = frisch(&buchungen, &[], Haken::default());
    let mut b = Store::leer(2025, None);
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let eb = uebernehme_voll(&mut b, &buchungen, nachschlag, None, None, None, Haken::default()).unwrap();
    assert_eq!(ea.uebernahme, eb.uebernahme);
    assert_eq!(ea.nachfragen, eb.nachfragen);
    for f in ["spenden_betrag", "vor_rv_ausserhalb_lstb"] {
        assert_eq!(wert(&a, f), wert(&b, f), "{f}");
        assert!(wert(&a, f).is_some(), "Kontrolle: {f} ist geschrieben");
    }
}

/// Eine Buchung, die der Klassifikator einordnet, ist eine Zeile mit `ueber_ki`; die Stichwort-Buchung nicht.
#[test]
fn bank_auswahl_zeile_nennt_ob_die_ki_die_buchung_einordnete() {
    let aufrufe = Cell::new(0);
    let ki = |_: &Maskiert, _: i64| {
        aufrufe.set(aufrufe.get() + 1);
        Some(Kategorie::Spende)
    };
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let mut store = Store::leer(2025, None);
    let buchungen = [tx("2025-03-01", -8_00, "Spende Tierheim"), tx("2025-03-02", -9_00, "Muster Rechnung 4711")];
    let erg = uebernehme_auswahl(
        &mut store,
        &buchungen,
        &[],
        nachschlag,
        Some(&ki as &Klassifikator<'_>),
        (None, None),
        Haken::default(),
    )
    .unwrap();
    assert_eq!(aufrufe.get(), 1, "nur die Buchung ohne Stichwort geht an den Klassifikator");
    let ueber_ki: Vec<(usize, bool)> = erg.zeilen.iter().map(|z| (z.index, z.ueber_ki)).collect();
    assert_eq!(ueber_ki, [(0, false), (1, true)]);
    // Beide Zeilen tragen die Kennung der Regel, die sie ordnete (die der KI-Kategorie ist die Regel `spende`).
    assert_eq!(erg.zeilen.iter().map(|z| z.regel_id).collect::<Vec<_>>(), ["spende", "spende"]);
}
