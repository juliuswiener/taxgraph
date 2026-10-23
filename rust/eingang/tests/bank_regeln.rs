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
