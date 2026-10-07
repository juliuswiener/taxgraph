//! Unfallkosten auf dem Arbeitsweg neben der Entfernungspauschale (Abweichung Nr. 28). Im Standardlauf, ohne `PARITY=1`,
//! ohne Python.
//!
//! **Worum es geht.** Wer auf dem Weg zur Arbeit einen Unfall hatte, darf die Kosten zusaetzlich zur Entfernungspauschale
//! abziehen. `TaxGraph` fragte sie nicht, die Rechnung kannte sie nicht: die Steuer fiel zu hoch aus.
//!
//! **Warum es zaehlt.** 1.500 Euro Unfallkosten senken den Gesamtbetrag der Einkuenfte um 1.500 Euro. Bei 50.000 Euro Lohn
//! und 30 km Arbeitsweg sind das 517 Euro weniger Steuer (9.922 statt 9.405 Euro, Fall A unten). Die Pauschale allein deckt
//! den Unfall nie ab.
//!
//! **Wo es sitzt.** `zweige/wk.rs::mit_unfallkosten`; beide Ringe, die Werbungskosten rechnen (`zweige/an_gesamt.rs`,
//! `zweige/gesamt.rs`), rufen sie nach `werbungskosten_n`. Die Tests setzen das Feld ohne Scheiben-Gate in den Store, sie
//! messen die Ringe; die Fragen- und Abgabe-Seite misst `api/tests/unfallkosten_hermetisch.rs`.
//!
//! RECHTSLAGE (Entscheidung `unfallkosten-folgen-der-bmf-verwaltungssicht`): Wortlaut `sources/bmf/
//! bmf_entfernungspauschalen_wortlaut_2021-11-18.txt`, Rz. 30: Unfallkosten sind „weiterhin neben der Entfernungspauschale
//! zu beruecksichtigen“; dazu zaehlen fahrzeug- und wegstreckenbezogene Aufwendungen (entgegen BFH VI R 8/18) und Aufwendungen
//! fuer Koerperschaeden.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus Gesetz und `params/`, nie aus dem Rust-Code gelesen.
//! - Entfernungspauschale: Arbeitstage x (20 km x 0,30 + restliche km x 0,38) (`sources/gesetze-im-internet/
//!   estg_p9_abs1nr4_abs2_2026-07-09.txt`, Rz. 9 des BMF-Schreibens); 30 km, 220 Tage: 220 x (6,00 + 3,80) = 2.156 Euro.
//!   60 km, 220 Tage: 220 x (6,00 + 15,20) = 4.664 Euro, ohne Kraftwagen auf 4.500 Euro begrenzt (Rz. 10, 11).
//! - Arbeitnehmer-Pauschbetrag 1.230 Euro (`params/2025/arbeitnehmerpauschbetrag.yaml`, § 9a Satz 1 Nr. 1a): die
//!   Werbungskosten zaehlen erst, wenn sie ihn uebersteigen.
//! - Gesamtbetrag der Einkuenfte = Lohn − max(Werbungskosten, 1.230).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types,
    clippy::too_many_lines
)]

use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg, FesteZahl};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::{Felder, Instanzquelle};
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;
const FELD: &str = "ep_unfallkosten";

type Paare = Vec<(&'static str, Value)>;

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// `tests/_kegel.py::standardwert`: der Abwesenheitswert, nie der illustrative Beispielwert.
fn standardwert(b: &Bindung) -> Value {
    if let Some(w) = &b.abwesenheitswert {
        return w.clone();
    }
    match b.typ {
        Feldtyp::Bool => json!(b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_")),
        Feldtyp::Cent | Feldtyp::Int => json!(0),
        Feldtyp::Enum => b.beispielwert.clone(),
        _ => panic!(
            "KONTROLLE: Kegel-Feld {} hat keinen Abwesenheitswert",
            b.feld_id
        ),
    }
}

/// `tests/_kegel.py::kegel_fuer`: der volle Pflicht-Kegel; was der Test setzt, gewinnt, der Rest folgt bestaetigt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(&'static str, Value)]) -> Paare {
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .rev()
                .find(|(g, _)| g == f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            (*f, w)
        })
        .collect();
    raus.extend(gesetzt.iter().filter(|(f, _)| !kegel.contains(f)).cloned());
    raus
}

/// Ein Fall der Scheibe, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(scheibe: Scheibe, paare: &[(&'static str, Value)]) -> Fall {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let events: Vec<(&str, Value, bool)> = kegel_fuer(&cfg, paare)
        .into_iter()
        .map(|(f, w)| (f, w, true))
        .collect();
    let store = store(&events);
    Fall {
        cfg,
        index: teil.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        achsen: teil.iter().map(|b| AchsenBindung::from(*b)).collect(),
        felder: felder(&store),
        store,
    }
}

/// Der Ausgang von `_ergebnis_roh` ohne HTTP: erst der K2-Guard, dann `feste_zahl` (nur bestaetigte Werte).
fn ergebnis(f: &Fall) -> Result<FesteZahl, String> {
    let q = Instanzquelle {
        store: Some(&f.store),
        bindung: Some(&f.index),
        nur_bestaetigt: false,
    };
    match an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q) {
        Ok(Some(g)) => return Err(format!("Sperre {}", g.als_str())),
        Ok(None) => {}
        Err(e) => return Err(format!("Guard: {e:?}")),
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => Ok(z),
        Ok(Err(k)) => Err(format!("ohne Zahl: {:?}", k.grund)),
        Err(e) => Err(format!("{e:?}")),
    }
}

fn zahl(scheibe: Scheibe, paare: &[(&'static str, Value)]) -> FesteZahl {
    ergebnis(&fall(scheibe, paare)).unwrap_or_else(|was| panic!("KONTROLLE: erwartet eine Zahl, bekommen {was}"))
}

/// Gesamtbetrag der Einkuenfte in Euro (nur der Gesamt-Ring legt die Kette offen).
fn gdb(paare: &[(&'static str, Value)]) -> i64 {
    zahl(Scheibe::Gesamt, paare)
        .extras
        .kette
        .expect("KONTROLLE: Kette fehlt")
        .gesamtbetrag_der_einkuenfte
        .get()
}

/// Ein Arbeitnehmer mit 50.000 Euro Lohn, einzeln veranlagt, `tage` Arbeitstage und `km` einfache Strecke, plus `mehr`.
fn pendler(km: i64, tage: i64, kfz: bool, mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(cent(50_000))),
        ("ep_arbeitstage", json!(tage)),
        ("ep_entfernung_km", json!(km)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(kfz)),
    ];
    for (f, w) in mehr {
        match p.iter_mut().find(|(g, _)| *g == f) {
            Some(e) => e.1 = w,
            None => p.push((f, w)),
        }
    }
    p
}

fn unfall(euro: i64) -> Paare {
    vec![(FELD, json!(cent(euro)))]
}

/// § 32a Abs. 1 `EStG` fuer VZ 2025, von Hand aus `params/2025/einkommensteuertarif_p32a.yaml` (Euro, abgerundet).
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]
fn tarif_2025(zve: i64) -> i64 {
    let z = zve as f64;
    let wert = if zve <= 12_096 {
        0.0
    } else if zve <= 17_443 {
        let y = (z - 12_096.0) / 10_000.0;
        (932.30 * y + 1_400.0) * y
    } else if zve <= 68_480 {
        let y = (z - 17_443.0) / 10_000.0;
        (176.64 * y + 2_397.0) * y + 1_015.13
    } else if zve <= 277_825 {
        0.42 * z - 10_911.92
    } else {
        0.45 * z - 19_246.67
    };
    wert.floor() as i64
}

/// Sonderausgaben-Pauschbetrag 2025 (§ 10c `EStG`), der einzige Sonderausgabenabzug dieser Faelle.
const SA_PAUSCHBETRAG: i64 = 36;

/// Beide Ringe, die Werbungskosten rechnen.
const RINGE: [Scheibe; 2] = [Scheibe::Gesamt, Scheibe::AnGesamt];

/// Die Faelle der Kontrolle, mit den Zahlen, die der Stand VOR dem Bau gab (HEAD `93bab088`, gemessen 2026-10-07 mit
/// diesem Fall ohne das Feld): (Name, km, Tage, Kraftwagen, Steuer in Cent, Gesamtbetrag der Einkuenfte in Euro).
const KONTROLLE: [(&str, i64, i64, bool, i64, i64); 3] = [
    ("A 30 km, 220 Tage, Kraftwagen", 30, 220, true, 992_200, 47_844),
    ("B 60 km, 220 Tage, ohne Kraftwagen, Pauschale ueber dem Hoechstbetrag", 60, 220, false, 911_800, 45_500),
    ("C 5 km, 100 Tage, Kraftwagen, Pauschale unter dem Pauschbetrag", 5, 100, true, 1_024_500, 48_770),
];

// ---------------------------------------------------------------- Scheiben

/// Das Feld steht in den Scheiben, deren Ring Werbungskosten rechnet. Die Rentner-Scheibe fragt keine Entfernungspauschale
/// und ihr Ring rechnet ausser dem Pauschbetrag keine Werbungskosten: eine Frage dort liesse sich nie auswirken.
#[test]
fn die_ringe_mit_werbungskosten_fuehren_das_feld_die_rentner_scheibe_nicht() {
    for s in RINGE {
        let ids = Cfg::fuer(s).felder(|d| panic!("Scheibe liest Felder aus {d}")).unwrap();
        assert!(ids.iter().any(|f| f == FELD), "{s:?} fuehrt {FELD} nicht");
        assert!(
            !Cfg::fuer(s).kegel_roh().unwrap().contains(&FELD),
            "{FELD} ist eine freiwillige Angabe, kein Pflicht-Kegel-Feld ({s:?})"
        );
    }
    let rentner = Cfg::fuer(Scheibe::RentnerGesamt)
        .felder(|d| panic!("Scheibe liest Felder aus {d}"))
        .unwrap();
    assert!(!rentner.iter().any(|f| f == FELD), "rentner_gesamt fragt {FELD}, ihr Ring liest es nie");
    assert!(
        !rentner.iter().any(|f| f == "ep_arbeitstage"),
        "rentner_gesamt fragt jetzt eine Entfernungspauschale: dann braucht {FELD} dort eine Entscheidung"
    );
}

// ---------------------------------------------------------------- Betrag

/// 30 km, 220 Tage, Kraftwagen: Pauschale 2.156 Euro. 1.500 Euro Unfallkosten kommen DAZU: Werbungskosten 3.656 Euro,
/// Gesamtbetrag 46.344 Euro, die Steuer folgt dem Tarif auf das zvE. Beide Ringe geben dieselbe Zahl.
#[test]
fn unfallkosten_kommen_zur_entfernungspauschale_dazu() {
    assert_eq!(gdb(&pendler(30, 220, true, vec![])), 50_000 - 2_156, "Kontrolle ohne Unfallkosten");
    let mit = gdb(&pendler(30, 220, true, unfall(1_500)));
    assert_eq!(mit, 50_000 - (2_156 + 1_500), "Gesamtbetrag der Einkuenfte mit 1.500 Euro Unfallkosten");
    let kette = zahl(Scheibe::Gesamt, &pendler(30, 220, true, unfall(1_500))).extras.kette.unwrap();
    assert_eq!(kette.zu_versteuerndes_einkommen.get(), mit - SA_PAUSCHBETRAG);
    assert_eq!(kette.tarifliche_est.get(), tarif_2025(mit - SA_PAUSCHBETRAG));
    for s in RINGE {
        let ohne = zahl(s, &pendler(30, 220, true, vec![])).zahl.get();
        let mit = zahl(s, &pendler(30, 220, true, unfall(1_500))).zahl.get();
        assert_eq!(
            mit,
            cent(tarif_2025(50_000 - 3_656 - SA_PAUSCHBETRAG)),
            "{s:?}: Steuer mit Unfallkosten folgt dem Tarif auf 46.308 Euro zvE"
        );
        assert!(mit < ohne, "{s:?}: Unfallkosten senken die Steuer ({mit} gegen {ohne})");
        assert_eq!(ohne - mit, cent(517), "{s:?}: 9.922 Euro ohne, 9.405 Euro mit Unfallkosten");
    }
}

/// 60 km, 220 Tage, ohne Kraftwagen: Pauschale 4.664 Euro, begrenzt auf 4.500 Euro. Die Grenze gilt fuer die Pauschale, nicht
/// fuer die Unfallkosten: 1.000 Euro kommen auf die 4.500 Euro oben drauf (Werbungskosten 5.500 Euro).
#[test]
fn der_hoechstbetrag_der_pauschale_gilt_nicht_fuer_unfallkosten() {
    assert_eq!(gdb(&pendler(60, 220, false, vec![])), 50_000 - 4_500, "Kontrolle: Pauschale auf 4.500 Euro begrenzt");
    assert_eq!(
        gdb(&pendler(60, 220, false, unfall(1_000))),
        50_000 - (4_500 + 1_000),
        "die Unfallkosten liegen ueber der Grenze"
    );
    for s in RINGE {
        assert_eq!(
            zahl(s, &pendler(60, 220, false, unfall(1_000))).zahl.get(),
            cent(tarif_2025(50_000 - 5_500 - SA_PAUSCHBETRAG)),
            "{s:?}"
        );
    }
}

/// 5 km, 100 Tage: Pauschale 150 Euro, der Arbeitnehmer-Pauschbetrag (1.230 Euro) gilt. 500 Euro Unfallkosten (Werbungskosten
/// 650 Euro) aendern nichts, 2.000 Euro (2.150 Euro) uebersteigen ihn.
#[test]
fn unfallkosten_wirken_erst_oberhalb_des_arbeitnehmer_pauschbetrags() {
    assert_eq!(gdb(&pendler(5, 100, true, vec![])), 50_000 - 1_230);
    assert_eq!(gdb(&pendler(5, 100, true, unfall(500))), 50_000 - 1_230, "650 Euro Werbungskosten unter dem Pauschbetrag");
    assert_eq!(gdb(&pendler(5, 100, true, unfall(2_000))), 50_000 - 2_150, "2.150 Euro Werbungskosten ueber dem Pauschbetrag");
    for s in RINGE {
        assert_eq!(
            zahl(s, &pendler(5, 100, true, unfall(500))).zahl,
            zahl(s, &pendler(5, 100, true, vec![])).zahl,
            "{s:?}: 500 Euro aendern die Steuer nicht"
        );
        assert!(
            zahl(s, &pendler(5, 100, true, unfall(2_000))).zahl < zahl(s, &pendler(5, 100, true, vec![])).zahl,
            "{s:?}: 2.000 Euro senken sie"
        );
    }
}

/// Zusammenveranlagung: die Unfallkosten sind die der Person A (Person B hat keine Entfernungspauschale im Modell). Person B
/// verdient 30.000 Euro (Pauschbetrag 1.230 Euro). Gesamtbetrag ohne Unfallkosten 50.000 − 2.156 + 30.000 − 1.230 = 76.614
/// Euro, mit 1.500 Euro Unfallkosten 75.114 Euro; zvE nach 2 x 36 Euro Pauschbetrag; Splitting: Steuer = 2 x Tarif(zvE / 2).
#[test]
fn unfallkosten_zaehlen_auch_bei_zusammenveranlagung() {
    let zusammen = |mehr: Paare| {
        // Der Gesamt-Guard verlangt bei Zusammenveranlagung den bestaetigten Person-B-Kegel (Lohn und Kapital).
        let mut m: Paare = vec![
            ("veranlagung", json!("zusammen")),
            ("bruttoarbeitslohn_partner", json!(cent(30_000))),
            ("kap_kapitalertraege_partner", json!(0)),
            ("kap_gewinn_aktien_partner", json!(0)),
            ("kap_gewinn_sonstige_partner", json!(0)),
            ("kap_verlust_aktien_partner", json!(0)),
            ("kap_verlust_sonstige_partner", json!(0)),
        ];
        m.extend(mehr);
        pendler(30, 220, true, m)
    };
    let splitting = |gesamtbetrag: i64| cent(2 * tarif_2025((gesamtbetrag - 2 * SA_PAUSCHBETRAG) / 2));
    for s in RINGE {
        assert_eq!(zahl(s, &zusammen(vec![])).zahl.get(), splitting(76_614), "{s:?}: Kontrolle ohne Unfallkosten");
        assert_eq!(zahl(s, &zusammen(unfall(1_500))).zahl.get(), splitting(75_114), "{s:?}: mit 1.500 Euro Unfallkosten");
    }
}

/// Ein negativer Wert, der am Schreibweg vorbei im Store steht (Laden prueft nie), erhoeht die Steuer nicht.
#[test]
fn ein_negativer_wert_im_store_zaehlt_als_null() {
    let negativ = vec![(FELD, json!(-cent(1_500)))];
    assert_eq!(gdb(&pendler(30, 220, true, negativ.clone())), 50_000 - 2_156);
    for s in RINGE {
        assert_eq!(zahl(s, &pendler(30, 220, true, negativ.clone())).zahl, zahl(s, &pendler(30, 220, true, vec![])).zahl);
    }
}

// ---------------------------------------------------------------- Kontrolle

/// Ohne das Feld und mit dem Wert 0 bleibt jede Zahl, wie sie vor dem Bau war (`KONTROLLE`, gemessen auf HEAD
/// `93bab088`). Gleich bleibt auch der Gesamtbetrag.
#[test]
fn leer_oder_null_aendert_keine_zahl_gegen_den_stand_vor_dem_bau() {
    for (name, km, tage, kfz, steuer_cent, gdb_euro) in KONTROLLE {
        for mehr in [vec![], unfall(0)] {
            assert_eq!(gdb(&pendler(km, tage, kfz, mehr.clone())), gdb_euro, "{name}: Gesamtbetrag");
            for s in RINGE {
                assert_eq!(
                    zahl(s, &pendler(km, tage, kfz, mehr.clone())).zahl.get(),
                    steuer_cent,
                    "{name}, {s:?}, Feld {mehr:?}: Steuer"
                );
            }
        }
    }
}

// ---------------------------------------------------------------- vorlaeufig

/// Ein vorlaeufiger Betrag faellt still aus der Zahl, weil der Ring auf bestaetigt filtert: er steht deshalb in den
/// Ring-Betragsfeldern der beiden Scheiben und macht die Zahl vorlaeufig statt "bestaetigt".
#[test]
fn ein_vorlaeufiger_betrag_wird_gemeldet() {
    use bescheid::deklaration::vorlaeufige_ring_betraege;
    let f = felder(&store(&[(FELD, json!(cent(1_500)), false)]));
    for s in RINGE {
        assert!(
            vorlaeufige_ring_betraege(&f, &Cfg::fuer(s), index()).contains(&FELD),
            "{s:?}: ein vorlaeufiger {FELD} bleibt unbemerkt"
        );
    }
    let f = felder(&store(&[(FELD, json!(cent(1_500)), true)]));
    for s in RINGE {
        assert!(
            !vorlaeufige_ring_betraege(&f, &Cfg::fuer(s), index()).contains(&FELD),
            "{s:?}: ein bestaetigter {FELD} wird als vorlaeufig gemeldet"
        );
    }
}
