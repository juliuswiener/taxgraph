//! Bankimport S2 (Vault `backlog/taxgraph/bankimport-s2-regelkatalog-mit-stufen-35a-ohne-betrag-und-art9-waechter`):
//! § 35a schreibt keinen Betrag ins Feld (d-61), nur `spenden_betrag`, `vor_rv_ausserhalb_lstb`, `basis_kv` und `basis_pv`
//! nehmen einen Kontoauszug an, und ein Beleg schlaegt den Kontoauszug.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung, jeder Test mit eigenen Betraegen. BMF-Schreiben vom 2016-11-09 Rn. 40
//! (`sources/bmf/bmf_p35a_2016-11-09.txt`, Zeilen 510 bis 513): der Anteil der Arbeitskosten folgt aus der Rechnung, eine Schaetzung
//! durch den Steuerpflichtigen ist nicht zulaessig; der Rechnungsbetrag auf dem Konto ist darum kein Wert fuer das Feld.
//! Die Buchungstexte sind erfunden.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::Cell;

use eingang::kontoauszug::{uebernehme, Klassifikator, Transaktion};
use eingang::vorschlag::{Quelle, VorschlagEvent};
use llm::pii::Maskiert;
use llm::Kategorie;
use serde_json::{json, Value};
use store::{BindungNachschlag, Katalog, Store};

const HH: [&str; 3] = ["hh_handwerker_betrag", "hh_dienstleistung_betrag", "hh_minijob_betrag"];

fn tx(datum: &str, betrag: i64, zweck: &str) -> Transaktion {
    Transaktion {
        datum: json!(datum),
        betrag,
        verwendungszweck: zweck.to_owned(),
    }
}

fn wert(store: &Store, feld: &str) -> Option<i64> {
    let (_, e) = store.aktive().find(|(f, _)| *f == feld)?;
    serde_json::to_value(e).unwrap()["wert"].as_i64()
}

/// Die drei § 35a-Kategorien bekommen keinen Betrag ins Feld; eine Spende im selben Auszug schon (Kontrolle: ohne sie waere
/// ein Test gruen, der nie etwas schreibt).
#[test]
fn bank_regeln_35a_schreibt_keinen_betrag_ins_feld() {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let mut store = Store::leer(2025, None);
    let buchungen = [
        tx("2025-03-01", -73_00, "Maler Huber"),
        tx("2025-03-02", -41_00, "Reinigung Treppenhaus"),
        tx("2025-03-03", -29_00, "Minijob-Zentrale"),
        tx("2025-03-04", -11_00, "Spende Tierheim"),
    ];
    let erg = uebernehme(&mut store, &buchungen, nachschlag, None, None, None).unwrap();
    for feld in HH {
        assert_eq!(wert(&store, feld), None, "{feld} traegt einen Betrag aus dem Kontoauszug");
    }
    assert_eq!(wert(&store, "spenden_betrag"), Some(11_00), "Kontrolle: die Spende wird weiter vorgeschlagen");
    assert_eq!(erg.uebernommen, 1, "{erg:?}");
}

/// Auch ein Treffer der KI fuer eine § 35a-Kategorie schreibt nichts: der Weg zur Kategorie aendert nichts am Rechnungsbetrag.
#[test]
fn bank_regeln_35a_gilt_auch_fuer_einen_ki_treffer() {
    let nachschlag = BindungNachschlag::neu(eingang::doctest_bindung().unwrap());
    let aufrufe = Cell::new(0);
    let ki = |_: &Maskiert, _: i64| {
        aufrufe.set(aufrufe.get() + 1);
        Some(Kategorie::Handwerker)
    };
    let mut store = Store::leer(2025, None);
    let buchungen = [tx("2025-03-05", -58_00, "Muster Rechnung 4711")];
    uebernehme(&mut store, &buchungen, nachschlag, Some(&ki as &Klassifikator<'_>), None, None).unwrap();
    assert_eq!(aufrufe.get(), 1, "der Klassifikator muss den Fall sehen");
    assert_eq!(wert(&store, "hh_handwerker_betrag"), None);
}

/// Freigabe des Kontoauszugs (Messung A des Forks `bankimport-s2-messung`): `basis_kv` und `basis_pv` sind Kegel-Felder, ein
/// vorlaeufiger Betrag aendert die Zahl nicht. `kist_*` (Ring, Julius entscheidet) und `hh_*` (der vorlaeufige Betrag faellt
/// still aus der Zahl) bleiben zu. Jedes zugemachte Feld muss in der Registry stehen, sonst waere "nicht erlaubt" leer wahr.
#[test]
fn bank_freigabe_kontoauszug_nur_diese_felder() {
    let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let alle: Vec<bindung::Bindung> = bindung::lade_registry_der_wurzel(&wurzel)
        .unwrap()
        .dateien
        .into_iter()
        .flat_map(|(_, d)| d.bindungen)
        .collect();
    let katalog = Katalog::aus_bindungen(&alle);
    let mut falsch = Vec::new();
    for feld in ["spenden_betrag", "vor_rv_ausserhalb_lstb", "basis_kv", "basis_pv"] {
        if !katalog.erlaubt("kontoauszug", feld) {
            falsch.push(format!("{feld}: muss den Kontoauszug annehmen"));
        }
    }
    for feld in HH.into_iter().chain(["kist_gezahlt", "kist_erstattet"]) {
        if !alle.iter().any(|b| b.feld_id == feld) {
            falsch.push(format!("{feld}: steht nicht in der Registry"));
        }
        if katalog.erlaubt("kontoauszug", feld) {
            falsch.push(format!("{feld}: darf den Kontoauszug nicht annehmen"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Beleg schlaegt Konto, Schreib-Seite: steht auf dem Feld ein Ereignis des Belegs, schreibt der Kontoauszug nichts und aendert
/// nichts. Die Seite der Antwort (der Abgleich) prueft `rust/api/tests/kontoauszug_naht.rs`.
#[test]
fn bank_beleg_schlaegt_konto_der_kontoauszug_schreibt_nichts() {
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
    let erg = uebernehme(
        &mut store,
        &[tx("2025-03-06", -15_00, "Spende Tierheim")],
        nachschlag,
        None,
        None,
        None,
    )
    .unwrap();
    assert_eq!((erg.uebernommen, erg.feld_belegt), (0, 1), "{erg:?}");
    assert_eq!(wert(&store, "spenden_betrag"), Some(25_00), "der Beleg gilt weiter");
    let schreiber = store.events().iter().map(|e| serde_json::to_value(e).unwrap()["schreiber"].clone()).collect::<Vec<Value>>();
    assert_eq!(schreiber, [json!("import:beleg")], "kein Ereignis des Kontoauszugs dazugekommen");
}

// ---------------------------------------------------------------------------
// Antwort des § 35a, Haken `art9` und Posten ohne Zweck, Abgleich (Bankimport S2). Jeder Test nimmt eigene Betraege.
// ---------------------------------------------------------------------------

use eingang::kontoauszug::{bestehende_posten, uebernehme_voll, Abgleich, Haken, Nachfrage};

const MIT_ART9: Haken = Haken { ersetzen: false, art9: true };

fn nachschlag() -> BindungNachschlag<'static> {
    BindungNachschlag::neu(eingang::doctest_bindung().unwrap())
}

/// Die Nachfrage nennt Feld, ISO-Datum, Betrag mit dem Vorzeichen der Buchung und den maskierten Zweck; eine Buchung
/// ausserhalb des Veranlagungsjahrs und ein Datum ohne Jahr fragen nicht nach; eine Rueckbuchung steht mit positivem Betrag.
#[test]
fn bank_regeln_35a_nachfrage_nennt_feld_datum_betrag_und_zweck() {
    let mut store = Store::leer(2025, None);
    let buchungen = [
        tx("2025-03-01", -64_00, "Maler Kiefer Rechnung 4711"),
        tx("2025-03-05", 12_00, "Rueckbuchung Maler Kiefer"),
        tx("2024-12-30", -33_00, "Reinigung Vorjahr"),
        tx("15.03", -18_00, "Minijob ohne Jahr"),
        tx("2025-04-02", -27_00, "Hausmeister Muster, Kto 12345678901"),
    ];
    let e = uebernehme_voll(&mut store, &buchungen, nachschlag(), None, None, None, Haken::default()).unwrap();
    assert_eq!(
        e.nachfragen,
        [
            Nachfrage {
                feld_id: "hh_handwerker_betrag",
                datum: "2025-03-01".to_owned(),
                betrag_cent: -64_00,
                zweck_maskiert: "Maler Kiefer Rechnung 4711".to_owned(),
            },
            Nachfrage {
                feld_id: "hh_handwerker_betrag",
                datum: "2025-03-05".to_owned(),
                betrag_cent: 12_00,
                zweck_maskiert: "Rueckbuchung Maler Kiefer".to_owned(),
            },
            Nachfrage {
                feld_id: "hh_dienstleistung_betrag",
                datum: "2025-04-02".to_owned(),
                betrag_cent: -27_00,
                zweck_maskiert: "Hausmeister Muster, Kto ****".to_owned(),
            },
        ]
    );
    let u = e.uebernahme;
    assert_eq!((u.nachgefragt, u.anderes_jahr, u.datum_unlesbar, u.uebernommen), (3, 1, 1, 0), "{u:?}");
    assert_eq!(store.events().len(), 0);
}

/// Ein Fall ohne das Feld (Bindung der Scheibe fuehrt es nicht) fragt nicht nach: die Pruefung des Zielfelds steht VOR der Nachfrage.
#[test]
fn bank_regeln_35a_fragt_nicht_nach_wo_der_fall_das_feld_nicht_fuehrt() {
    let mut karte = eingang::doctest_bindung().unwrap().clone();
    karte.remove("hh_minijob_betrag");
    let mut store = Store::leer(2025, None);
    let e = uebernehme_voll(
        &mut store,
        &[tx("2025-03-03", -21_00, "Minijob-Zentrale"), tx("2025-03-04", -22_00, "Heizung Wartung")],
        BindungNachschlag::neu(&karte),
        None,
        None,
        None,
        Haken::default(),
    )
    .unwrap();
    assert_eq!((e.uebernahme.feld_fehlt, e.uebernahme.nachgefragt), (1, 1), "{:?}", e.uebernahme);
    assert_eq!(e.nachfragen.iter().map(|n| n.feld_id).collect::<Vec<_>>(), ["hh_handwerker_betrag"]);
}

/// Haken `art9`, Zustand AUS: weder Krankenversicherung noch Pflegeversicherung schreiben ein Ereignis; die Buchungen zaehlen als
/// nicht ausgewertet (`art9_gesperrt`) und gehen nie an den Klassifikator.
#[test]
fn bank_art9_ohne_haken_schlagen_die_regeln_nie_an_und_die_ki_sieht_nichts() {
    let aufrufe = Cell::new(0);
    let ki = |_: &Maskiert, _: i64| {
        aufrufe.set(aufrufe.get() + 1);
        Some(Kategorie::Spende)
    };
    let mut store = Store::leer(2025, None);
    let buchungen = [
        tx("2025-03-01", -251_00, "Techniker Krankenversicherung Beitrag"),
        tx("2025-03-02", -52_00, "Pflegeversicherung Beitrag"),
        tx("2025-03-03", -41_17, "Zahnarzt Dr. Beispiel"),
        tx("2025-03-04", -9_99, "Muster Einkauf 4711"),
    ];
    let e = uebernehme_voll(&mut store, &buchungen, nachschlag(), Some(&ki as &Klassifikator<'_>), None, None, Haken::default())
        .unwrap();
    assert_eq!(aufrufe.get(), 1, "nur die harmlose Buchung geht an den Klassifikator");
    let u = e.uebernahme;
    assert_eq!((u.art9_gesperrt, u.uebernommen), (3, 1), "{u:?}");
    assert_eq!(wert(&store, "basis_kv"), None);
    assert_eq!(wert(&store, "basis_pv"), None);
    // Die KI ordnete die harmlose Buchung der Spende zu: das Ereignis traegt nur diese eine Buchung.
    assert_eq!(wert(&store, "spenden_betrag"), Some(9_99));
}

/// Haken `art9`, Zustand AN: die zwei Regeln schreiben vorlaeufige Summen; der Posten traegt KEINEN Zweck (leer), das Ereignis die
/// Kennung der Regel, die Antwort den Satz der Regel. Ein Text aus der besonderen Kategorie ohne Regel (Zahnarzt) bleibt ohne
/// Vorschlag und geht auch jetzt nie an die KI.
#[test]
fn bank_art9_mit_haken_schreiben_die_regeln_ohne_zweck() {
    let aufrufe = Cell::new(0);
    let ki = |_: &Maskiert, _: i64| {
        aufrufe.set(aufrufe.get() + 1);
        None
    };
    let mut store = Store::leer(2025, None);
    let buchungen = [
        tx("2025-03-01", -251_00, "Techniker Krankenversicherung Beitrag"),
        tx("2025-03-02", -52_00, "Pflegeversicherung Beitrag"),
        tx("2025-03-03", -41_17, "Zahnarzt Dr. Beispiel"),
        tx("2025-03-04", -9_99, "Muster Einkauf 4711"),
    ];
    let e = uebernehme_voll(&mut store, &buchungen, nachschlag(), Some(&ki as &Klassifikator<'_>), None, None, MIT_ART9).unwrap();
    assert_eq!(aufrufe.get(), 1, "nur die harmlose Buchung geht an den Klassifikator");
    let u = e.uebernahme;
    assert_eq!((u.uebernommen, u.art9_gesperrt, u.ohne_stichwort), (2, 0, 2), "{u:?}");
    assert_eq!(wert(&store, "basis_kv"), Some(251_00));
    assert_eq!(wert(&store, "basis_pv"), Some(52_00));
    let ereignisse = serde_json::to_string(&store.events()).unwrap();
    for text in ["Techniker", "Krankenversicherung", "Pflegeversicherung", "Zahnarzt", "Beispiel"] {
        assert!(!ereignisse.contains(text), "{text:?} steht in der Akte");
    }
    let (_, ev) = store.aktive().find(|(f, _)| *f == "basis_kv").unwrap();
    let s1 = &serde_json::to_value(ev).unwrap()["signal"]["signal_1"];
    assert_eq!(s1["regel_id"], "krankenversicherung");
    assert_eq!(s1["kategorie"], "krankenversicherung", "Regeln ohne Kategorie tragen die Kennung");
    assert_eq!(s1["posten"], json!([{"datum": "2025-03-01", "betrag_cent": -251_00, "zweck_maskiert": ""}]));
    assert_eq!(e.regelhinweise.len(), 2, "{:?}", e.regelhinweise);
    assert!(e.regelhinweise[0].starts_with("basis_kv: ") && e.regelhinweise[0].contains("Beitragsbescheinigung"));
}

/// Bei beiden Woertern gewinnt die erste Regel des Katalogs (`krankenversicherung` vor `pflegeversicherung`).
#[test]
fn bank_art9_ein_text_mit_beiden_woertern_geht_in_die_erste_regel() {
    let mut store = Store::leer(2025, None);
    let b = [tx("2025-03-06", -88_00, "Krankenversicherung und Pflegeversicherung")];
    let e = uebernehme_voll(&mut store, &b, nachschlag(), None, None, None, MIT_ART9).unwrap();
    assert_eq!(e.uebernahme.uebernommen, 1);
    assert_eq!((wert(&store, "basis_kv"), wert(&store, "basis_pv")), (Some(88_00), None));
}

/// Zwei Art.-9-Buchungen am selben Tag mit gleichem Betrag und verschiedenem Text haben denselben Posten-Schluessel (der Zweck
/// ist leer), zaehlen in EINEM Auszug aber beide in der Summe. Derselbe Auszug noch einmal verdoppelt die Summe nicht.
#[test]
fn bank_art9_gleiche_posten_zaehlen_im_auszug_beide_und_der_zweite_import_verdoppelt_nicht() {
    let b = [
        tx("2025-03-07", -75_00, "Krankenkasse Muster Eins"),
        tx("2025-03-07", -75_00, "Krankenkasse Muster Zwei"),
    ];
    let mut store = Store::leer(2025, None);
    let e = uebernehme_voll(&mut store, &b, nachschlag(), None, None, None, MIT_ART9).unwrap();
    assert_eq!((e.uebernahme.uebernommen, wert(&store, "basis_kv")), (2, Some(150_00)));
    // Derselbe Auszug noch einmal, auch mit dem Haken zum Ersetzen: dieselbe Menge, nichts Neues.
    for ersetzen in [false, true] {
        let e = uebernehme_voll(&mut store, &b, nachschlag(), None, None, None, Haken { ersetzen, art9: true }).unwrap();
        assert_eq!((e.uebernahme.schon_erfasst, e.uebernahme.uebernommen, e.uebernahme.ersetzbar), (2, 0, 0), "{:?}", e.uebernahme);
        assert_eq!(store.events().len(), 1, "kein neues Ereignis");
        assert_eq!(wert(&store, "basis_kv"), Some(150_00), "die Summe bleibt");
    }
}

/// GRENZE (`ponytail` an der Stelle in `uebernehme_voll`): ein zweiter, ueberlappender Auszug mit einer ANDEREN Buchung gleichen Tags
/// und Betrags (anderer Text) bringt sie nicht dazu: ohne Zweck im Schluessel sind beide dieselbe Buchung. Der Test haelt die
/// Grenze fest; wer den Schluessel erweitert, aendert ihn.
#[test]
fn bank_art9_grenze_zwei_verschiedene_buchungen_gleichen_tags_und_betrags_aus_zwei_auszuegen_zaehlen_einmal() {
    let mut store = Store::leer(2025, None);
    let erster = [tx("2025-03-08", -60_00, "Krankenkasse Muster Eins")];
    let zweiter = [tx("2025-03-08", -60_00, "Krankenkasse Muster Zwei")];
    uebernehme_voll(&mut store, &erster, nachschlag(), None, None, None, MIT_ART9).unwrap();
    let e = uebernehme_voll(&mut store, &zweiter, nachschlag(), None, None, None, Haken { ersetzen: true, art9: true }).unwrap();
    assert_eq!(e.uebernahme.schon_erfasst, 1);
    assert_eq!(wert(&store, "basis_kv"), Some(60_00), "die zweite Buchung zaehlt nicht mit (Grenze)");
}

/// Der Lesepfad der Ersetzung kennt Ereignisse mit unbekannter Kategorie und leerem Zweck: `bestehende_posten` liest sie, und ein
/// Auszug mit dem Haken ersetzt sie durch die Vereinigung.
#[test]
fn bank_art9_lesepfad_vertraegt_unbekannte_kategorie_und_leeren_zweck() {
    let nachschlag = nachschlag();
    let katalog = Katalog::aus_bindungen(nachschlag.alle().map(|(_, b)| b));
    let mut store = Store::leer(2025, None);
    VorschlagEvent {
        quelle: Quelle::Kontoauszug,
        feld_id: "basis_pv".to_owned(),
        wert: json!(31_00),
        signal_1: json!({"typ": "kontoauszug", "kategorie": "frei_erfunden", "regel_id": "irgendwas", "jahr": 2025,
            "summe_cent": 31_00, "quelle": "heuristik",
            "posten": [{"datum": "2025-03-09", "betrag_cent": -31_00, "zweck_maskiert": ""}]}),
    }
    .schreibe(&mut store, Some(&katalog), nachschlag, None)
    .unwrap();
    assert_eq!(bestehende_posten(&store, "basis_pv").map(|p| p.len()), Some(1));
    let b = [tx("2025-03-09", -31_00, "Pflegeversicherung Beitrag"), tx("2025-03-19", -14_00, "Pflegeversicherung Nachzahlung")];
    let e = uebernehme_voll(&mut store, &b, nachschlag, None, None, None, Haken { ersetzen: true, art9: true }).unwrap();
    assert_eq!(e.ersetzungen.len(), 1, "{:?}", e.ersetzungen);
    assert_eq!((e.ersetzungen[0].bisher_cent, e.ersetzungen[0].neu_cent), (31_00, 45_00));
    assert_eq!(wert(&store, "basis_pv"), Some(45_00));
}

/// Abgleich: ein Beleg (oder ein bestaetigter Wert) auf dem Feld bleibt, der Kontoauszug nennt seine Summe in Cent, Ausgabe positiv;
/// eine Rueckbuchung mindert sie; eine Summe null oder darunter wird nicht genannt.
#[test]
fn bank_beleg_schlaegt_konto_abgleich_nennt_die_summe_des_kontos() {
    let nachschlag = nachschlag();
    let katalog = Katalog::aus_bindungen(nachschlag.alle().map(|(_, b)| b));
    let mut store = Store::leer(2025, None);
    for feld in ["spenden_betrag", "vor_rv_ausserhalb_lstb"] {
        VorschlagEvent { quelle: Quelle::Beleg, feld_id: feld.to_owned(), wert: json!(5_00), signal_1: json!({"typ": "beleg"}) }
            .schreibe(&mut store, Some(&katalog), nachschlag, None)
            .unwrap();
    }
    let b = [
        tx("2025-03-10", -15_00, "Spende Tierheim"),
        tx("2025-03-11", -20_00, "Spende Unicef"),
        tx("2025-03-12", 6_00, "Rueckbuchung Spende Unicef"),
        // Ausgleich: Summe null, nicht genannt.
        tx("2025-03-13", -40_00, "Rürup Beitrag"),
        tx("2025-03-14", 40_00, "Rueckbuchung Rürup Beitrag"),
        // Ausserhalb des Jahres: gehoert nicht zum Abgleich.
        tx("2024-12-31", -99_00, "Spende Vorjahr"),
    ];
    let e = uebernehme_voll(&mut store, &b, nachschlag, None, None, None, Haken::default()).unwrap();
    assert_eq!(e.abgleich, [Abgleich { feld_id: "spenden_betrag", summe_cent: 29_00, anzahl: 3 }]);
    assert_eq!((e.uebernahme.uebernommen, e.uebernahme.feld_belegt, e.uebernahme.anderes_jahr), (0, 5, 1), "{:?}", e.uebernahme);
    assert_eq!(store.events().len(), 2, "kein Ereignis des Kontoauszugs");
}
