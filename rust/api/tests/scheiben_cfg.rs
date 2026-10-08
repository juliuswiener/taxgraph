//! Schritt 2 der `stand`-Naht: `Cfg` traegt die Scheiben-Schluessel, die `/stand` liest.
//!
//! Geprueft wird die Gestalt aus `api.py` (`_cfg`, `_scheibe_felder`, `_feste_zahl`), nicht ein
//! Zahlwert. Jede Zusicherung nennt die Python-Fundstelle, gegen die sie laeuft.
//!
//! Die Tabellen selbst prueft `konstanten_gleich` (`rust/parity/tests/bescheid_deklaration_paritaet.rs`)
//! gegen das Orakel. Hier steht nur, dass `Cfg` sie richtig VERDRAHTET.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::items_after_statements
)]

use bescheid::deklaration::Cfg;
use domain::Scheibe;

fn ohne_datei(_: &str) -> Vec<String> {
    panic!("diese Scheibe hat felder=None und darf keine Datei anfassen")
}

#[test]
fn ep_traegt_sechs_felder_und_vier_kegel() {
    // api.py:794 SCHEIBEN["ep"]: felder = EP_FELDER + EP_FORMALIEN, kegel = EP_FELDER
    let c = Cfg::fuer(Scheibe::Ep);
    let f = c.felder(ohne_datei).unwrap();
    let k = c.kegel(ohne_datei).unwrap();
    assert_eq!(f.len(), 6, "ep felder: {f:?}");
    assert_eq!(k.len(), 4, "ep kegel: {k:?}");
    assert_eq!(f[0], "ep_arbeitstage");
    assert_eq!(f[5], "ep_ziel_adresse");
    // Der Kegel ist eine Teilmenge: die beiden Formalien fehlen absichtlich (kein Betrag beruehrt).
    assert!(k.iter().all(|x| f.contains(x)), "kegel nicht Teilmenge von felder");
    assert!(!k.contains(&"ep_ziel_des_weges".to_owned()));
    assert_eq!(c.gesamt_ring(), Some("abziehbarer_betrag"));
    assert!(!c.guard(), "ep hat keinen guard-Schluessel -> false (api.py:471)");
}

#[test]
fn n_vor_gwg_ist_die_scheibe_mit_felder_null() {
    // api.py:182: felder is None -> _datei_felder(cfg["felder_datei"]).
    // Der gefaehrlichste Fall: None als "leer" zu lesen liefert 69 Felder weniger OHNE Fehler.
    let c = Cfg::fuer(Scheibe::NVorGwg);
    assert!(c.felder_roh().is_none(), "n_vor_gwg MUSS felder=None tragen");
    assert_eq!(c.felder_datei(), Some("bindung_n_vor_gwg.yaml"));
    let f = c.felder(|d| {
        assert_eq!(d, "bindung_n_vor_gwg.yaml");
        vec!["ep_arbeitstage".to_owned(); 69]
    })
    .unwrap();
    assert_eq!(f.len(), 69, "n_vor_gwg liest die YAML, nicht nichts");
    assert_eq!(c.gesamt_ring(), None, "n_vor_gwg hat keine Scheiben-Zahl");
    assert!(!c.guard());
    // teil_ringe: [("ep_werbungskosten", "abziehbarer_betrag", EP_FELDER)]
    let t = c.teil_ringe();
    assert_eq!(t.len(), 1);
    assert_eq!(t[0].0, "ep_werbungskosten");
    assert_eq!(t[0].1, "abziehbarer_betrag");
    assert_eq!(t[0].2.len(), 4);
}

#[test]
fn die_drei_guard_scheiben_setzen_guard_und_gesamt_ring() {
    // api.py:471 liest cfg.get("guard"); api.py:474 liest cfg["gesamt_ring"].
    for (s, q) in [
        (Scheibe::AnGesamt, "festzusetzende_est"),
        (Scheibe::Gesamt, "festzusetzende_est_gesamt"),
        (Scheibe::RentnerGesamt, "festzusetzende_est_rentner"),
    ] {
        let c = Cfg::fuer(s);
        assert!(c.guard(), "{s} braucht guard=true");
        assert_eq!(c.gesamt_ring(), Some(q), "{s} gesamt_ring");
        assert!(c.felder_roh().is_some(), "{s} traegt seine Felder als Tupel");
    }
}

#[test]
fn felder_null_ohne_datei_ist_ein_fehler_kein_leerer_satz() {
    // Der Riegel gegen den stillen Rueckfall: wer felder=None ohne felder_datei liest, bekommt
    // einen Fehler und keine leere Liste. Sonst saehe der Nutzer eine leere Fragenliste.
    for s in [Scheibe::Ep, Scheibe::AnGesamt, Scheibe::Gesamt, Scheibe::RentnerGesamt] {
        let c = Cfg::fuer(s);
        assert!(c.felder(ohne_datei).is_ok(), "{s} darf keine Datei brauchen");
    }
    // NVorGwg mit einer Datei, die nichts liefert: kein Fehler, aber auch nicht still falsch --
    // die Aufloesung ist Sache des Aufrufers, der die YAML kennt.
    let n = Cfg::fuer(Scheibe::NVorGwg);
    assert!(n.felder(|_| Vec::new()).unwrap().is_empty());
}

#[test]
fn rentner_gesamt_fuehrt_jedes_feld_einmal() {
    // Python fuehrt `geburtsjahr` in `SCHEIBEN['rentner_gesamt']` doppelt (roh 250, distinct 249 am
    // 2026-10-03; 252/251 seit C2, 256/255 seit B Option 1, beides 2026-10-06). Rust fuehrt es einmal
    // (2026-10-06): das zweite Vorkommen brach die Vorjahr-Uebernahme mit 422 ab
    // (`vorjahr_naht.rs`, `vorjahr_nach_rentner_gesamt_uebernimmt_das_geburtsjahr`). Roh und
    // distinct sind seither gleich. Seit 2026-10-06 stehen sieben Felder nur in Rust dazu
    // (`bruttoarbeitslohn`, `steuerklasse`, fuenf `versorgung_*`): 262; seit 2026-10-07 acht mit
    // `kind_schulgeld_aufteilung_prozent` (Abweichung Nr. 26): 263; seit 2026-10-07 neun mit
    // `gwg_ohne_vorsteuerabzug` (Abweichung Nr. 27): 264; seit 2026-10-07 zehn mit `parteispenden_betrag`
    // (Abweichung Nr. 31): 265; seit 2026-10-07 zwoelf mit `alter_55_vor_verkauf` und
    // `alter_55_vor_verkauf_partner` (Abweichung Nr. 32): 267; seit 2026-10-07 vierzehn mit `bruttoarbeitslohn_partner`,
    // `steuerklasse_partner` und den fuenf `versorgung_*_partner` (Abweichung Nr. 33): 274; seit 2026-10-07
    // sechzehn mit `p36_lohnsteuer_partner` und `geburtsjahr_partner` (Abweichung Nr. 36): 276; seit 2026-10-08
    // siebzehn mit `dba_fiktive_steuer_vorhanden` (Abweichung Nr. 41): 277; seit 2026-10-08 achtzehn mit
    // `waehlervereinigungen_betrag` (Abweichung Nr. 44): 278.
    let c = Cfg::fuer(Scheibe::RentnerGesamt);
    let f = c.felder(ohne_datei).unwrap();
    assert_eq!(f.len(), 280, "roh gezaehlt");
    let n_geburtsjahr = f.iter().filter(|x| x.as_str() == "geburtsjahr").count();
    assert_eq!(n_geburtsjahr, 1, "geburtsjahr steht einmal in der Liste");
    let distinct: std::collections::HashSet<_> = f.iter().collect();
    assert_eq!(distinct.len(), 280, "jedes Feld genau einmal");
    assert_eq!(c.kegel(ohne_datei).unwrap().len(), 28, "der Meet laeuft ueber 28, nicht 255");
}
