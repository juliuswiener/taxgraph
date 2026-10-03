//! § 35 `EStG`: die Sperre `gewst_hebesatz_offen` im Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Wer Gewerbe-Messbetrag angibt, aber keinen brauchbaren Hebesatz, bekommt sonst still keine
//! Anrechnung (Vault `gewerbesteuer-anrechnung-rechnet-ohne-hebesatz-still-zu-wenig-an`: +4.000 EUR
//! bei Hebesatz 0 fuer Person A, +7.000 EUR bei fehlendem Hebesatz des Partners). Die Sperre sitzt in
//! `deklaration/sperre/einkunft.rs` (`betrag_offen`, Closure `offen`). Ihre Gegenprobe lief bisher nur in
//! `rust/parity/tests/bescheid_deklaration_paritaet.rs::gezielte_faelle` und damit nur mit `PARITY=1`;
//! die CI faehrt Parity nicht. Gemessen am 2026-10-03 auf 583d70f1: alle vier Mutationen des Eintrags
//! (R1 `z.is_zero() || z.is_sign_negative()` -> `z.is_sign_negative()`, R2 -> `z.is_zero()`, R3 Partner-
//! `offen(..)` -> `false`, R4 Person-A-`offen(..)` -> `false`) lassen `cargo test -p bescheid` gruen
//! (159 passed, 0 failed, 10 ignored). Mit diesen Tests werden alle vier rot, jede mit genau zwei der vier
//! neuen Tests (161 passed, 2 failed).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zeile der Tabelle unten ist die Ausgabe des Python-Orakels
//! `api._an_gesamt_sperrgrund(felder, SCHEIBEN["gesamt"], 2025, None, bindung)` auf denselben Events
//! (Orakel-Skript und Lauf: Anlagen zum Bericht). Kein Wert ist aus dem Rust-Code abgelesen. Die
//! Python-Tests `tests/test_gewinn_partner_ring.py::test_p35_*` stuetzen dieselben Aussagen.
//!
//! Hebesatz-Kontrollfaelle (400, 1) und Messbetrag 0 zeigen, dass die Sperre nicht alles sperrt: ohne sie
//! bestuende auch ein Guard, der immer `gewst_hebesatz_offen` liefert, jeden Fall der anderen Gruppen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};
use std::sync::OnceLock;

/// Ein handgebauter Fall: Events `(feld_id, wert, bestaetigt)` und der Grund, den das Python-Orakel liefert.
struct Fall {
    gruppe: &'static str,
    name: &'static str,
    events: Vec<(&'static str, Value, bool)>,
    erwartet: Option<&'static str>,
}

/// Die Bindung der Scheibe `gesamt`: nur ihre Feld-Ids, wie `api._scheibe_bindung`. Der volle Index kennt mehr
/// Flags und liesse `flag_widersprueche` auf Person B ansprechen.
fn scheiben_index() -> &'static BindungIndex<'static> {
    static I: OnceLock<BindungIndex<'static>> = OnceLock::new();
    I.get_or_init(|| {
        let ids = Cfg::fuer(Scheibe::Gesamt).felder(|_| Vec::new()).unwrap();
        index()
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, b)| (k.clone(), *b))
            .collect()
    })
}

/// Der Sperrgrund der Scheibe `gesamt` fuer VZ 2025 auf einem Store aus den Events.
fn grund(events: &[(&str, Value, bool)]) -> Option<Sperrgrund> {
    let st = store(events);
    let f = felder(&st);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(scheiben_index()),
        nur_bestaetigt: false,
    };
    let cfg = Cfg::fuer(Scheibe::Gesamt);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q).unwrap()
}

/// Bei Zusammenveranlagung verlangt der Partner-Kegel sechs bestaetigte Nullen (wie `partner_kegel` in
/// `bescheid_deklaration_paritaet.rs`); ohne sie kaeme `partner_kegel_offen` statt "keine Sperre".
// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn faelle() -> Vec<Fall> {
    vec![
        Fall {
            gruppe: "a_offen",
            name: "A: Messbetrag, Hebesatz 0",
            events: vec![
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(0), true),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "a_offen",
            name: "A: Messbetrag, Hebesatz -1",
            events: vec![
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(-1), true),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "a_offen",
            name: "A: Messbetrag, Hebesatz fehlt",
            events: vec![("gewst_messbetrag", json!(100_000), true)],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "a_offen",
            name: "A: Messbetrag, Hebesatz nur vorlaeufig 400",
            events: vec![
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(400), false),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "frei",
            name: "A: Messbetrag, Hebesatz 400",
            events: vec![
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(400), true),
            ],
            erwartet: None,
        },
        Fall {
            gruppe: "frei",
            name: "A: Messbetrag, Hebesatz 1",
            events: vec![
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(1), true),
            ],
            erwartet: None,
        },
        Fall {
            gruppe: "frei",
            name: "A: Messbetrag 0, Hebesatz 0",
            events: vec![
                ("gewst_messbetrag", json!(0), true),
                ("gewst_hebesatz", json!(0), true),
            ],
            erwartet: None,
        },
        Fall {
            gruppe: "frei",
            name: "A: Messbetrag 0, Hebesatz fehlt",
            events: vec![("gewst_messbetrag", json!(0), true)],
            erwartet: None,
        },
        Fall {
            gruppe: "b_offen",
            name: "B zusammen: Messbetrag, Hebesatz fehlt",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "b_offen",
            name: "B zusammen: Messbetrag, Hebesatz 0",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(0), true),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "b_offen",
            name: "B zusammen: Messbetrag, Hebesatz -1",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(-1), true),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "frei",
            name: "B zusammen: Messbetrag, Hebesatz 400",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("bruttoarbeitslohn_partner", json!(0), true),
                ("kap_kapitalertraege_partner", json!(0), true),
                ("kap_gewinn_aktien_partner", json!(0), true),
                ("kap_gewinn_sonstige_partner", json!(0), true),
                ("kap_verlust_aktien_partner", json!(0), true),
                ("kap_verlust_sonstige_partner", json!(0), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(400), true),
            ],
            erwartet: None,
        },
        Fall {
            gruppe: "frei",
            name: "B zusammen: Messbetrag 0, Hebesatz 0",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("bruttoarbeitslohn_partner", json!(0), true),
                ("kap_kapitalertraege_partner", json!(0), true),
                ("kap_gewinn_aktien_partner", json!(0), true),
                ("kap_gewinn_sonstige_partner", json!(0), true),
                ("kap_verlust_aktien_partner", json!(0), true),
                ("kap_verlust_sonstige_partner", json!(0), true),
                ("gewst_messbetrag_partner", json!(0), true),
                ("gewst_hebesatz_partner", json!(0), true),
            ],
            erwartet: None,
        },
        Fall {
            gruppe: "frei",
            name: "B einzel: Messbetrag, Hebesatz fehlt",
            events: vec![
                ("veranlagung", json!("einzel"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
            ],
            erwartet: None,
        },
        Fall {
            gruppe: "frei",
            name: "B einzel: Messbetrag, Hebesatz 0",
            events: vec![
                ("veranlagung", json!("einzel"), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(0), true),
            ],
            erwartet: None,
        },
        Fall {
            gruppe: "getrennt",
            name: "A ok, B zusammen ohne Hebesatz",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(400), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "getrennt",
            name: "A ohne Hebesatz, B zusammen ok",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(400), true),
            ],
            erwartet: Some("gewst_hebesatz_offen"),
        },
        Fall {
            gruppe: "frei",
            name: "A und B zusammen beide ok",
            events: vec![
                ("veranlagung", json!("zusammen"), true),
                ("bruttoarbeitslohn_partner", json!(0), true),
                ("kap_kapitalertraege_partner", json!(0), true),
                ("kap_gewinn_aktien_partner", json!(0), true),
                ("kap_gewinn_sonstige_partner", json!(0), true),
                ("kap_verlust_aktien_partner", json!(0), true),
                ("kap_verlust_sonstige_partner", json!(0), true),
                ("gewst_messbetrag", json!(100_000), true),
                ("gewst_hebesatz", json!(400), true),
                ("gewst_messbetrag_partner", json!(175_000), true),
                ("gewst_hebesatz_partner", json!(400), true),
            ],
            erwartet: None,
        },
    ]
}

/// Alle Faelle der Gruppe durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe(gruppe: &str) {
    let alle = faelle();
    let faelle: Vec<&Fall> = alle.iter().filter(|f| f.gruppe == gruppe).collect();
    assert!(!faelle.is_empty(), "Gruppe {gruppe} ist leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let g = grund(&f.events).map(Sperrgrund::als_str);
            (g != f.erwartet).then(|| format!("{}: {g:?}, Orakel {:?}", f.name, f.erwartet))
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// Person A: Messbetrag > 0 und Hebesatz 0, negativ, fehlend oder nur vorlaeufig sperrt (R1, R2, R4).
#[test]
fn person_a_hebesatz_null_negativ_fehlend_oder_vorlaeufig_sperrt() {
    pruefe("a_offen");
}

/// Person B bei Zusammenveranlagung, derselbe Guard fuer den Hebesatz des Partners (R1, R2, R3).
#[test]
fn person_b_bei_zusammenveranlagung_hebesatz_null_negativ_oder_fehlend_sperrt() {
    pruefe("b_offen");
}

/// Beide Halbseiten sind unabhaengig: ist A in Ordnung und B offen (oder umgekehrt), sperrt der Fall (R3, R4).
#[test]
fn person_a_und_b_werden_getrennt_geprueft() {
    pruefe("getrennt");
}

/// Kein Fehlalarm: brauchbarer Hebesatz, Messbetrag 0 und Einzelveranlagung (Partner zaehlt dort nicht)
/// sperren nicht mit diesem Grund.
#[test]
fn brauchbarer_hebesatz_messbetrag_null_und_einzelveranlagung_sperren_nicht() {
    pruefe("frei");
}
