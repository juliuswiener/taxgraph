//! Die vier Lagen, in denen `_feste_zahl` (`api.py:194`) `None` liefert -- je eine mit
//! eigenem Grund. Python wirft sie in einen Topf und `_ergebnis_roh` (`api.py:587-619`)
//! rechnet sie durch Nachfragen wieder auseinander; hier traegt das Ergebnis den Grund.
//!
//! Reihenfolge ist Semantik: Lage 1 vor Lage 3. `_ergebnis_roh` prueft sie zuerst.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use bescheid::deklaration::{feste_zahl, Cfg, KeineZahl};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use domain::{Scheibe, Vz};
use serde_json::json;

fn umgebung() -> Umgebung<'static> {
    Umgebung {
        achsen: &[],
        index: index(),
        params: params(),
    }
}

/// Lage 1: `cfg["gesamt_ring"] is None`. Nur `n_vor_gwg`. Python-Grund
/// `kein_scheiben_gesamtbescheid`.
#[test]
fn lage_1_scheibe_ohne_gesamt_accessor() {
    let leer = felder(&store(&[]));
    let r = feste_zahl(
        &leer,
        &Cfg::fuer(Scheibe::NVorGwg),
        Vz::Vz2025,
        &[],
        &umgebung(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(r.unwrap_err().grund, KeineZahl::KeinScheibenGesamtbescheid);
}

/// Lage 2: der Kegel ist unvollstaendig. `ep` hat vier Kegel-Felder, `felder` ist leer.
/// Python-Grund `input_kegel_nicht_bestaetigt`.
#[test]
fn lage_2_kegel_unvollstaendig() {
    let leer = felder(&store(&[]));
    let kegel = ["ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz"];
    let r = feste_zahl(
        &leer,
        &Cfg::fuer(Scheibe::Ep),
        Vz::Vz2025,
        &kegel,
        &umgebung(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(r.unwrap_err().grund, KeineZahl::InputKegelNichtBestaetigt);
}

/// Lage 2, zweite Haelfte: der Kegel ist VOLLSTAENDIG, aber ein Feld ist vorlaeufig.
/// Ohne diesen Fall waere `meet_zustand` allein der Waechter -- und der ist es nicht.
#[test]
fn lage_2_kegel_vollstaendig_aber_vorlaeufig() {
    let f = felder(&store(&[
        ("ep_arbeitstage", json!(220), true),
        ("ep_entfernung_km", json!(30), false),
        ("ep_oepnv_kosten", json!(0), true),
        ("ep_eigenes_kfz", json!(true), true),
    ]));
    let kegel = ["ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz"];
    let r = feste_zahl(
        &f,
        &Cfg::fuer(Scheibe::Ep),
        Vz::Vz2025,
        &kegel,
        &umgebung(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(r.unwrap_err().grund, KeineZahl::InputKegelNichtBestaetigt);
}

/// Lage 3: der Kegel ist durchgehend bestaetigt, aber ein Ring-Betrag AUSSERHALB des
/// Kegels ist nur genannt (Klasse C). Python-Grund `ring_betrag_vorlaeufig` -- und die
/// betroffene Feld-Id steht im Grund, sonst zeigt die Oberflaeche "noch offen" mit
/// leerer Liste.
#[test]
fn lage_3_ring_betrag_vorlaeufig_nennt_das_feld() {
    // Der VOLLE an_gesamt-Kegel (33 Felder), alle bestaetigt. `p36_lohnsteuer` steht NICHT
    // darin und ist vorlaeufig -- genau die Klasse-C-Lage.
    let cfg = Cfg::fuer(Scheibe::AnGesamt);
    let mut paare: Vec<(&str, serde_json::Value, bool)> = cfg
        .kegel_roh()
        .unwrap()
        .iter()
        .map(|f| (*f, json!(1), true))
        .collect();
    paare.push(("p36_lohnsteuer", json!(500_000), false));
    let f = felder(&store(&paare));
    let kegel: Vec<&str> = cfg.kegel_roh().unwrap().to_vec();
    let r = feste_zahl(&f, &cfg, Vz::Vz2025, &kegel, &umgebung(), None, None).unwrap();
    let g = r.unwrap_err();
    assert!(
        matches!(g.grund, KeineZahl::RingBetragVorlaeufig { .. }),
        "erwartet RingBetragVorlaeufig, war {:?}",
        g.grund
    );
    assert!(
        g.felder.contains(&"p36_lohnsteuer"),
        "das sperrende Feld fehlt im Grund: {:?}",
        g.felder
    );
}

/// Lage 4 in Rust **unerreichbar** -- gemessen an der Quelle, nicht am leeren Kegel.
///
/// `cfg["gesamt_ring"]` kommt aus [`Cfg::fuer`], und dessen fuenf Arme tragen genau die vier
/// Quantitaeten, die `Quantitaet::aus_name` kennt (`zweige.rs:74`). `bescheid_fn` gibt also
/// nie `None` zurueck -- die Lage kann nicht entstehen.
///
/// In Python ist sie erreichbar, weil `cfg` ein freies `dict` ist: dort ist `cfg["gesamt_ring"]`
/// ein beliebiger String. Die Aussage wird deshalb am Accessor geprueft, nicht am leeren Kegel:
/// ein leerer Kegel trifft Lage 2 gar nicht (`0 < 0` ist falsch) und liefe an der Frage vorbei.
///
/// Der Arm bleibt stehen: er ist die ehrliche Antwort auf die Python-Zeile `if bf is None:
/// return None`. Traegt jemand einen sechsten Namen in `SCHEIBEN`, den `Quantitaet::aus_name`
/// nicht kennt, faellt er hier auf, statt still als [`KeineZahl::EngineUnavailable`] durchzugehen.
#[test]
fn lage_4_ist_in_rust_unerreichbar() {
    use bescheid::zweige::Quantitaet;

    let alle = [
        Scheibe::Ep,
        Scheibe::NVorGwg,
        Scheibe::AnGesamt,
        Scheibe::Gesamt,
        Scheibe::RentnerGesamt,
    ];
    let mut mit_accessor = 0;
    for s in alle {
        let Some(q) = Cfg::fuer(s).gesamt_ring() else {
            continue; // Lage 1: kein Accessor, kein Gesamtbescheid
        };
        mit_accessor += 1;
        assert!(
            Quantitaet::aus_name(q).is_some(),
            "{s}: Accessor {q:?} ist Quantitaet::aus_name unbekannt -- Lage 4 waere erreichbar"
        );
    }
    // Vier der fuenf Scheiben tragen einen Gesamt-Accessor; nur `n_vor_gwg` nicht.
    assert_eq!(mit_accessor, 4, "Zahl der Scheiben mit Gesamt-Accessor");
}

/// Die Gegenprobe zu [`lage_4_ist_in_rust_unerreichbar`]: hier wird die Lage WIRKLICH
/// erreicht -- mit einem Namen, den `aus_name` nicht kennt, so wie Python es kann.
/// Ohne diesen Test waere die Unerreichbarkeit nur eine Behauptung ueber den Code.
#[test]
fn lage_4_waere_erreichbar_wenn_der_name_unbekannt_waere() {
    let u = umgebung();
    assert!(
        bescheid::zweige::bescheid_fn("erfunden", Vz::Vz2025, &u, None, None, true, None, None)
            .is_none(),
        "ein unbekannter Name muss None geben -- sonst ist Lage 4 keine Lage"
    );
    assert!(
        bescheid::zweige::bescheid_fn(
            "festzusetzende_est",
            Vz::Vz2025,
            &u,
            None,
            None,
            true,
            None,
            None
        )
        .is_some(),
        "ein bekannter Name muss einen Accessor geben -- sonst prueft der Test nichts"
    );
}

// Der leere Kegel hat eine eigene Datei: `znull_kegel.rs`. Er ist ein **Pin** auf einen
// gemessenen, im Betrieb nicht erreichbaren Sonderfall -- und gehoert damit nicht in die
// Datei, die die vier Lagen von `_feste_zahl` prueft.
