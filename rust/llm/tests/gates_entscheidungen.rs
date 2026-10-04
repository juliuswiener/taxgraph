//! Entscheidungsstellen der deterministischen Gates (`api_llm.py`: `_beleg_geprueft`,
//! `_rueckfragen_gebuendelt`, `_rueckfragen_gebunden`, `_rueckfrage_verdraengt`,
//! `_felder_je_regel`, `_mit_zaehlfeldern`), am Aufrufort von `llm::gates` geprueft (N4,
//! Mutationsmessung `rust/llm`, Teil `gates.rs`). Jede Erwartung stammt aus dem Python-Aufruf mit
//! denselben Eingaben (`aem/orakel_n4_llm_gates.py`); sie steht hier als Literal, kein Test ruft
//! Python.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use llm::gates::{
    beleg_geprueft, felder_je_regel, mit_zaehlfeldern, rueckfrage_verdraengt,
    rueckfragen_gebuendelt, rueckfragen_gebunden, KatalogFeld, LoeseGrund, RUECKFRAGEN_MAX,
};
use llm::parse::{Rueckfrage, Vorschlag};
use serde_json::json;

fn vorschlag(feld: &str, beleg: &str) -> Vorschlag {
    Vorschlag {
        feld_id: feld.to_owned(),
        wert: json!(1),
        beleg: beleg.to_owned(),
        begruendung: String::new(),
        aussage: None,
        rechenweg: json!(null),
    }
}

fn rueckfrage(frage: &str, feld: &str, aussage: Option<i64>) -> Rueckfrage {
    Rueckfrage {
        frage: frage.to_owned(),
        feld_id: feld.to_owned(),
        aussage,
    }
}

fn katalog_feld(feld: &str, typ: Option<&str>, gruppe: Option<&str>) -> KatalogFeld {
    serde_json::from_value(json!({"feld_id": feld, "typ": typ, "instanz_gruppe": gruppe})).unwrap()
}

/// Python `_beleg_geprueft`: ab 3 Zeichen reicht ein Teilstring des normalisierten Freitexts,
/// darunter muss der Beleg als eigenes Wort dastehen (`(?<!\w)beleg(?!\w)`), mit Wortgrenze davor
/// und danach, auch bei Zeichen aus mehr als einem Byte.
#[test]
fn beleg_gate_nimmt_ab_drei_zeichen_den_teilstring_und_darunter_das_wort() {
    let faelle = [
        ("xabcx", "abc", true),
        ("xabx", "ab", false),
        ("xab", "ab", false),
        ("ab x", "ab", true),
        ("xäb äb", "äb", true),
        ("äb x", "äb", true),
        ("Xäb Äb", "äb", true),
    ];
    for (freitext, beleg, soll) in faelle {
        let (gefiltert, _) = llm::pii::filtere(freitext);
        let (ok, weg) = beleg_geprueft(vec![vorschlag("f", beleg)], &gefiltert);
        assert_eq!(ok.len() + weg.len(), 1);
        assert_eq!(ok.len() == 1, soll, "{freitext:?} / {beleg:?}");
    }
}

/// Python: hoechstens `RUECKFRAGEN_MAX` (8) Rueckfragen, jede Aussage hoechstens einmal. Zehn
/// verschiedene Aussagen bleiben acht, zwei fallen weg; genau acht bleiben alle.
#[test]
fn buendelung_deckelt_bei_acht() {
    assert_eq!(RUECKFRAGEN_MAX, 8);
    for (n, bleiben, weg) in [(10, 8, 2), (9, 8, 1), (8, 8, 0), (7, 7, 0)] {
        let r: Vec<_> = (0..n)
            .map(|i| rueckfrage("?", "", Some(i64::from(i))))
            .collect();
        let (behalten, zurueck) = rueckfragen_gebuendelt(r);
        assert_eq!((behalten.len(), zurueck), (bleiben, weg), "n={n}");
    }
}

/// Python `_rueckfragen_gebunden`: das Feld wird geloest (die Frage bleibt), wenn es nicht im
/// Katalog steht (`unbekannt`) oder Geld↔`int` bzw. Anzahl↔`cent` nicht passt (`zahlenart`).
/// Ein leeres `feld_id` wird nie angefasst. `eur`/`euro`/`€` und `anzahl`/`wie viele` zaehlen nur
/// als eigenes Wort.
#[test]
fn rueckfrage_bindung_loest_nach_zahlenart_und_wortgrenze() {
    let faelle: [(&str, &str, &str, Option<LoeseGrund>); 10] = [
        ("Wie viel €?", "n", "int", Some(LoeseGrund::Zahlenart)),
        ("Neuro Therapie?", "n", "int", None),
        ("Euros?", "n", "int", None),
        ("Wieviele Kinder?", "n", "cent", Some(LoeseGrund::Zahlenart)),
        ("Hauptanzahl?", "n", "cent", None),
        ("Wie viel Euro?", "n", "int", Some(LoeseGrund::Zahlenart)),
        ("Anzahl?", "n", "cent", Some(LoeseGrund::Zahlenart)),
        ("x?", "", "int", None),
        ("x?", "fehlt", "int", Some(LoeseGrund::Unbekannt)),
        ("Wie viel Euro?", "n", "cent", None),
    ];
    let kat = [katalog_feld("n", Some("int"), None)];
    for (frage, feld, typ, soll) in faelle {
        let kat = if typ == "int" {
            kat.clone()
        } else {
            [katalog_feld("n", Some(typ), None)]
        };
        let (r, geloest) = rueckfragen_gebunden(
            vec![rueckfrage(frage, feld, None)],
            &kat.iter().collect::<Vec<_>>(),
        );
        if let Some(grund) = soll {
            assert_eq!(r[0].feld_id, "", "{frage:?}");
            assert_eq!(geloest.len(), 1, "{frage:?}");
            assert_eq!(geloest[0].grund, grund, "{frage:?}");
            assert_eq!(geloest[0].feld_id, feld);
        } else {
            assert_eq!(r[0].feld_id, feld, "{frage:?}");
            assert!(geloest.is_empty(), "{frage:?}");
        }
    }
}

/// Python `{r["feld_id"] for r in rueckfragen if r.get("feld_id")}`: ein leeres `feld_id` der
/// Rueckfrage verdraengt keinen Vorschlag, auch keinen mit leerem `feld_id`.
#[test]
fn leeres_feld_id_verdraengt_keinen_vorschlag() {
    let (gefiltert, _) = llm::pii::filtere("20 km");
    let (behalten, _) = beleg_geprueft(
        vec![vorschlag("", "20 km"), vorschlag("km", "20 km")],
        &gefiltert,
    );
    assert_eq!(behalten.len(), 2);
    let r = vec![rueckfrage("?", "", None), rueckfrage("?", "km", None)];
    let rest = rueckfrage_verdraengt(behalten, &r);
    let felder: Vec<_> = rest
        .iter()
        .map(|v| v.vorschlag().feld_id.as_str())
        .collect();
    assert_eq!(felder, [""]);
}

/// Python `_felder_je_regel`: ein Feld ohne `regel_id` steht unter dem leeren Schluessel, in der
/// Reihenfolge des ersten Auftritts.
#[test]
fn felder_je_regel_nimmt_den_leeren_schluessel_fuer_felder_ohne_regel() {
    let katalog: Vec<KatalogFeld> = serde_json::from_value(json!([
        {"feld_id": "a"},
        {"feld_id": "b", "regel_id": "r1"},
        {"feld_id": "c"},
    ]))
    .unwrap();
    let je = felder_je_regel(&katalog);
    let namen: Vec<_> = je.iter().map(|(r, f)| (r.as_str(), f.len())).collect();
    assert_eq!(namen, [("", 2), ("r1", 1)]);
}

/// Python `if f.get("instanz_gruppe")`: eine leere Gruppe ist keine Gruppe und zieht kein
/// Zaehlfeld nach.
#[test]
fn leere_instanz_gruppe_zieht_kein_zaehlfeld_nach() {
    let katalog = [
        katalog_feld("a", None, Some("")),
        katalog_feld("zaehl", None, None),
    ];
    let gruppen = [(String::new(), "zaehl".to_owned())];
    let mit = mit_zaehlfeldern(vec![&katalog[0]], &katalog, &gruppen);
    assert_eq!(mit.len(), 1);
}

/// Ein Regex-Laufzeitfehler (Backtracking-Grenze von `fancy-regex`, ab etwa einer Million Zeichen
/// gemessen) loest das Feld fail-closed. Python hat diese Grenze nicht und fand dort keinen
/// Treffer: dokumentierte Abweichung (`PyRegex::sucht`, `rueckfragen_gebunden`). Gilt fuer das
/// Geldwort (Typ `int`) wie fuer das Anzahlwort (Typ `cent`).
#[test]
fn regex_laufzeitfehler_loest_das_feld_fail_closed() {
    let lang = "a".repeat(1_000_000);
    for typ in ["int", "cent"] {
        let kat = [katalog_feld("n", Some(typ), None)];
        let (r, geloest) = rueckfragen_gebunden(
            vec![rueckfrage(&lang, "n", None)],
            &kat.iter().collect::<Vec<_>>(),
        );
        assert_eq!(r[0].feld_id, "", "{typ}");
        assert_eq!(geloest.len(), 1, "{typ}");
        assert_eq!(geloest[0].grund, LoeseGrund::Zahlenart, "{typ}");
    }
}
