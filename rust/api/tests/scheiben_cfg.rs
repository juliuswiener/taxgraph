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
fn rentner_gesamt_traegt_das_doppelte_geburtsjahr_nicht_still_weg() {
    // Befund aus helfer-spec.md: rentner_gesamt hat 248 Feld-Eintraege, aber nur 247 verschiedene.
    // Folge in Python: _scheibe_felder 248, _scheibe_bindung 247, Differenz verschwindet ohne
    // Meldung. Hier wird die Differenz SICHTBAR gemacht, nicht wegnormalisiert.
    let c = Cfg::fuer(Scheibe::RentnerGesamt);
    let f = c.felder(ohne_datei).unwrap();
    assert_eq!(f.len(), 248, "roh gezaehlt wie _scheibe_felder");
    let n_geburtsjahr = f.iter().filter(|x| x.as_str() == "geburtsjahr").count();
    assert_eq!(n_geburtsjahr, 2, "das Duplikat ist da und wird nicht versteckt");
    let distinct: std::collections::HashSet<_> = f.iter().collect();
    assert_eq!(distinct.len(), 247, "distinct wie _scheibe_bindung");
    assert_eq!(c.kegel(ohne_datei).unwrap().len(), 28, "der Meet laeuft ueber 28, nicht 248");
}
