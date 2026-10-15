//! Abweichung Nr. 78: der Sonderausgaben-Pauschbetrag (36 EUR, bei Zusammenveranlagung 72 EUR) steht NEBEN der Vorsorge. Je Scheibe, die
//! ihn anwendet (`RentnerGesamt`, `Gesamt`, `AnGesamt` einzeln und zusammen), und in der Engine unmittelbar. Im Standardlauf, ohne
//! `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Der Pauschbetrag ersetzt die Sonderausgaben, die jemand nicht einzeln nachweist. § 10c Satz 1 `EStG` nennt dafuer nur
//! § 10 Abs. 1 Nr. 4, 5, 7 und 9, Abs. 1a und § 10b (`sources/gesetze-im-internet/estg_p10c_2026-07-09.txt`). Die Vorsorge (Nr. 2
//! Altersvorsorge, Nr. 3 und 3a Kranken-, Pflege- und weitere Versicherungen) steht nicht darin: sie wird zusaetzlich abgezogen. Der Abzug ist
//! also `max(uebrige Sonderausgaben; Pauschbetrag) + Vorsorge`. Bis zur Abweichung Nr. 78 rechnete Rust (wie Python) `max(Sonderausgaben +
//! Vorsorge; Pauschbetrag)`: ab 36 EUR Vorsorge fiel der Pauschbetrag weg, darunter ging die Vorsorge ganz im Pauschbetrag auf.
//!
//! **Warum es zaehlt.** Das zu versteuernde Einkommen war bei jedem Fall mit Vorsorge um bis zu 36 EUR (zusammen 72 EUR) zu hoch, die Steuer
//! um bis zu rund 16 EUR (zusammen rund 32 EUR). Entdeckt hat es die BMF-Uebersicht zur Rentenbesteuerung (`rente_bmf_uebersicht_hermetisch.rs`).
//!
//! **Wo es sitzt.** Die Engine bildet den Abzug einmal (`engine::zugriff::teil2::gesamt::sonderausgaben_abzug`); `Gesamt` und `RentnerGesamt`
//! gehen ueber `gesamt_out`, `AnGesamt` uebergibt den fertigen Abzug an die Catala-Scopes `FestzusetzendeEstEinzel`/`-Zusammen`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Handrechnung aus § 10c `EStG` (Abzug), § 2 Abs. 4 `EStG` (zvE = `GdE` minus Abzug) und § 32a `EStG` (Tarif;
//! Koeffizienten aus `params/2025/einkommensteuertarif_p32a.yaml`), mit einem eigenen Rechenskript ausserhalb des Repos. Das Skript gibt vor
//! dem Einsatz an 12 bekannten Werten (6 Golden-Faelle `gesamt_*_vorsorge_*`, 6 Tarifpunkte `g32a_*`) jeweils die Golden-Zahl wieder. Keine Zahl
//! ist dem neuen Lauf entnommen. Jede Zeile nennt die Rechnung und, zur Abgrenzung, das Ergebnis der alten Lesart (Python).
//!
//! Grenzen: nur VZ 2025 (die Golden-Faelle in `golden_werte.rs` decken 2024 und 2026); `AnGesamt` gibt kein zvE heraus, dort gilt die Steuer;
//! die Catala-Scopes `FestzusetzendeEstEinzel`/`-Zusammen` bilden weiter `max(Sonderausgaben; Pauschbetrag)` und werden von `AnGesamt` mit dem
//! fertigen Abzug gespeist (der Pauschbetrag ist darin schon enthalten, das `max` aendert nichts).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;

use bescheid::deklaration::{feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::Felder;
use bindung::Bindung;
use domain::{Euro, Feldtyp, Scheibe, Vz};
use engine::zugriff::teil2::gesamt::{beispiel_gesamtfall, gesamt, gesamt_zve, GesamtfallEingabe};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;

type Paare = Vec<(String, Value)>;

fn p(feld: &str, wert: Value) -> (String, Value) {
    (feld.to_owned(), wert)
}

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// Der Abwesenheitswert, nie der illustrative Beispielwert (wie in `rente_bmf_uebersicht_hermetisch.rs`).
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

/// Der volle Pflicht-Kegel der Scheibe; was der Test setzt, gewinnt, der Rest folgt bestaetigt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(String, Value)]) -> Paare {
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .rev()
                .find(|(g, _)| g == *f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            ((*f).to_owned(), w)
        })
        .collect();
    raus.extend(
        gesetzt
            .iter()
            .filter(|(f, _)| !kegel.contains(&f.as_str()))
            .cloned(),
    );
    raus
}

/// Was die Scheibe herausgibt, in ganzen Euro: die festzusetzende Einkommensteuer und, wo die Scheibe eine Kette fuehrt, das zvE.
#[derive(Debug, PartialEq, Eq)]
struct Ergebnis {
    zve: Option<i64>,
    est: i64,
}

/// Die Scheibe `scheibe` im Veranlagungsjahr 2025 fuer eine Akte; ein Fall ohne Zahl ist ein Fehlschlag des Tests.
fn rechne(scheibe: Scheibe, paare: &[(String, Value)]) -> Ergebnis {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let alle = kegel_fuer(&cfg, paare);
    let events: Vec<(&str, Value, bool)> = alle
        .iter()
        .map(|(f, w)| (f.as_str(), w.clone(), true))
        .collect();
    let store: Store = store(&events);
    let felder: Felder = felder(&store);
    let achsen: Vec<AchsenBindung> = teil.iter().map(|b| AchsenBindung::from(*b)).collect();
    let index: HashMap<String, &'static Bindung> =
        teil.iter().map(|b| (b.feld_id.clone(), *b)).collect();
    let kegel = cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &achsen,
        index: &index,
        params: params(),
    };
    match feste_zahl(&felder, &cfg, VZ, &kegel, &umg, Some(&store), None) {
        Ok(Ok(z)) => {
            let est = z.zahl.get() / 100;
            let zve = z.extras.kette.map(|k| {
                // Kontrolle: die Kette endet bei der ausgegebenen Steuer, `zahl` ist also die festzusetzende ESt in Cent.
                assert_eq!(k.festzusetzende_est.get(), est, "KONTROLLE: Kette gegen zahl");
                k.zu_versteuerndes_einkommen.get()
            });
            Ergebnis { zve, est }
        }
        Ok(Err(k)) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {:?}", k.grund),
        Err(e) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {e:?}"),
    }
}

/// Eine Zeile: Akte, Soll und die Rechnung dazu.
struct Fall {
    name: &'static str,
    scheibe: Scheibe,
    paare: Paare,
    /// zvE nach der Handrechnung; `None` bei `AnGesamt` (gibt kein zvE heraus).
    zve: Option<i64>,
    est: i64,
}

/// Alle Zeilen durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jede rote Zeile.
fn melde(faelle: &[Fall]) {
    let ab: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let ist = rechne(f.scheibe, &f.paare);
            let soll = Ergebnis {
                zve: f.zve,
                est: f.est,
            };
            (ist != soll).then(|| format!("{}: Ist {ist:?}, Soll {soll:?}", f.name))
        })
        .collect();
    assert!(
        ab.is_empty(),
        "{} von {} Zeilen weichen ab: {ab:#?}",
        ab.len(),
        faelle.len()
    );
}

// ---------------------------------------------------------------------------------------------------------------------------------
// AnGesamt: reiner Arbeitnehmer. Brutto 61.230 EUR - Arbeitnehmer-Pauschbetrag 1.230 EUR (§ 9a Satz 1 Nr. 1 EStG) = 60.000 EUR Einkuenfte.

/// Akte `AnGesamt`: Brutto 61.230 EUR, Einzelveranlagung, dazu die Vorsorgefelder.
fn an_einzel(vorsorge: &[(&str, Value)]) -> Paare {
    let mut v = vec![
        p("veranlagung", json!("einzel")),
        p("bruttoarbeitslohn", json!(cent(61_230))),
    ];
    v.extend(vorsorge.iter().map(|(f, w)| p(f, w.clone())));
    v
}

/// Akte `AnGesamt` zusammen: A 41.230 EUR und B 21.230 EUR brutto = 40.000 + 20.000 EUR Einkuenfte.
fn an_zusammen(vorsorge: &[(&str, Value)]) -> Paare {
    let mut v = vec![
        p("veranlagung", json!("zusammen")),
        p("bruttoarbeitslohn", json!(cent(41_230))),
        p("bruttoarbeitslohn_partner", json!(cent(21_230))),
    ];
    v.extend(vorsorge.iter().map(|(f, w)| p(f, w.clone())));
    v
}

/// Einzelveranlagung. `GdE` 60.000 EUR, zvE = 60.000 - 36 - Vorsorge (§ 10c Satz 1 + § 10 Abs. 1 Nr. 2, 3).
///  - RV 10.000 EUR (12.000 Beitraege inkl. AG, AG-Anteil 2.000, Hoechstbetrag 29.344): zvE 49.964, Steuer 10.678 (alt 50.000, 10.691)
///  - RV 20 EUR: zvE 59.944, Steuer 14.393 (alt: max(20; 36) = 36, 59.964, 14.401)
///  - RV 36 EUR: zvE 59.928, Steuer 14.387 (alt: 36, 59.964, 14.401)
///  - Basis-KV 3.000 EUR: zvE 56.964, Steuer 13.247 (alt 57.000, 13.260)
///  - ohne Vorsorge (Kontrolle): zvE 59.964, Steuer 14.401 (alt gleich)
#[test]
fn an_gesamt_einzel_pauschbetrag_steht_neben_der_vorsorge() {
    let rv = |an: i64, ag: i64| {
        an_einzel(&[
            ("vor_an_anteil_rv", json!(cent(an))),
            ("vor_ag_anteil_rv", json!(cent(ag))),
        ])
    };
    let fall = |name, paare, est| Fall {
        name,
        scheibe: Scheibe::AnGesamt,
        paare,
        zve: None,
        est,
    };
    melde(&[
        fall("RV 10.000 (10.000 + AG 2.000)", rv(10_000, 2_000), 10_678),
        fall("RV 20 (unter dem Pauschbetrag)", rv(20, 0), 14_393),
        fall("RV 36 (gleich dem Pauschbetrag)", rv(36, 0), 14_387),
        fall(
            "Basis-KV 3.000 (Nr. 3)",
            an_einzel(&[("basis_kv", json!(cent(3_000)))]),
            13_247,
        ),
        fall("ohne Vorsorge (Kontrolle)", an_einzel(&[]), 14_401),
    ]);
}

/// Zusammenveranlagung: der Pauschbetrag verdoppelt sich (§ 10c Satz 2). `GdE` 60.000 EUR, Splitting (§ 32a Abs. 5).
///  - Basis-KV 3.000 EUR (A) + 2.000 EUR (B): zvE 60.000 - 72 - 5.000 = 54.928, Steuer 7.188 (alt 55.000, 7.208)
///  - Basis-KV 20 EUR (A): zvE 60.000 - 72 - 20 = 59.908, Steuer 8.580 (alt: max(20; 72) = 72, 59.928, 8.586)
#[test]
fn an_gesamt_zusammen_pauschbetrag_steht_neben_der_vorsorge() {
    let fall = |name, paare, est| Fall {
        name,
        scheibe: Scheibe::AnGesamt,
        paare,
        zve: None,
        est,
    };
    melde(&[
        fall(
            "Basis-KV 3.000 + 2.000",
            an_zusammen(&[
                ("basis_kv", json!(cent(3_000))),
                ("basis_kv_partner", json!(cent(2_000))),
            ]),
            7_188,
        ),
        fall(
            "Basis-KV 20 (unter 72)",
            an_zusammen(&[("basis_kv", json!(cent(20)))]),
            8_580,
        ),
    ]);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Gesamt: alle Einkunftsarten. Dieselbe Akte (61.230 EUR brutto, 60.000 EUR Einkuenfte), dazu Vorsorge ueber beide Wege: Altersvorsorge
// (`vorsorge_abzug`) und Kranken-/Pflegeversicherung (`SteuerSonderAgb::vorsorge_kv_pv`), und uebrige Sonderausgaben (Kirchensteuer, Nr. 4).

fn gesamt_akte(zusatz: &[(&str, Value)]) -> Paare {
    let mut v = vec![
        p("veranlagung", json!("einzel")),
        p("bruttoarbeitslohn", json!(cent(61_230))),
    ];
    v.extend(zusatz.iter().map(|(f, w)| p(f, w.clone())));
    v
}

/// `GdE` 60.000 EUR.
///  - RV 10.000 EUR: zvE 49.964, Steuer 10.678 (alt 50.000, 10.691)
///  - RV 20 EUR: zvE 59.944, Steuer 14.393 (alt 59.964, 14.401)
///  - Basis-KV 3.000 EUR: zvE 56.964, Steuer 13.247 (alt 57.000, 13.260)
///  - Kirchensteuer 20 EUR (unter dem Pauschbetrag) + RV 10.000 EUR: Abzug max(20; 36) + 10.000 = 10.036, zvE 49.964, Steuer 10.678 (alt:
///    20 + 10.000, 49.980, 10.684)
///  - Kirchensteuer 500 EUR (ueber dem Pauschbetrag) + RV 10.000 EUR: Abzug 500 + 10.000, zvE 49.500, Steuer 10.514 (alt gleich; Kontrolle:
///    wo der Pauschbetrag nicht bindet, aendert sich nichts)
#[test]
fn gesamt_pauschbetrag_steht_neben_der_vorsorge() {
    let rv = |an: i64, ag: i64| {
        vec![
            ("vor_an_anteil_rv", json!(cent(an))),
            ("vor_ag_anteil_rv", json!(cent(ag))),
        ]
    };
    let fall = |name, zusatz: &[(&str, Value)], zve, est| Fall {
        name,
        scheibe: Scheibe::Gesamt,
        paare: gesamt_akte(zusatz),
        zve: Some(zve),
        est,
    };
    let mit_kist = |kist: i64| {
        let mut v = rv(10_000, 2_000);
        v.push(("kist_gezahlt", json!(cent(kist))));
        v
    };
    melde(&[
        fall("RV 10.000", &rv(10_000, 2_000), 49_964, 10_678),
        fall("RV 20 (unter dem Pauschbetrag)", &rv(20, 0), 59_944, 14_393),
        fall(
            "Basis-KV 3.000 (Nr. 3)",
            &[("basis_kv", json!(cent(3_000)))],
            56_964,
            13_247,
        ),
        fall(
            "Kirchensteuer 20 + RV 10.000",
            &mit_kist(20),
            49_964,
            10_678,
        ),
        fall(
            "Kirchensteuer 500 + RV 10.000 (Kontrolle)",
            &mit_kist(500),
            49_500,
            10_514,
        ),
    ]);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// RentnerGesamt: gesetzliche Rente 40.000 EUR brutto, Rentenbeginn 2005, steuerfreier Teil 20.000 EUR (Besteuerungsanteil 50 %),
// Werbungskosten-Pauschbetrag 102 EUR: GdE 40.000 - 20.000 - 102 = 19.898 EUR. Die Vorsorge ist die Basis-Kranken-/Pflegeversicherung.

fn rentner_akte(basis_kv_euro: i64) -> Paare {
    vec![
        p("veranlagung", json!("einzel")),
        p("stammdaten_geburtsdatum", json!("05.05.1955")),
        p("kein_gewinn", json!(true)),
        p("kein_kap", json!(true)),
        p("kein_vuv", json!(true)),
        p("kein_sonstige", json!(false)),
        p("agb_zwangslaeufig", json!(true)),
        p("agb_notwendig_angemessen", json!(true)),
        p("rentner_renten_art", json!("gesetzliche_rente")),
        p("rentner_jahresrente", json!(cent(40_000))),
        p("rentner_renten_beginn_jahr", json!(2005)),
        p("rentner_alter_bei_rentenbeginn", json!(65)),
        p("rentner_rentenfreibetrag", json!(cent(20_000))),
        p("versicherungsart", json!("gesetzlich_freiwillig")),
        p("basis_kv", json!(cent(basis_kv_euro))),
        p("basis_pv", json!(0)),
        p("mit_anspruch_auf_zuschuss", json!(false)),
    ]
}

/// `GdE` 19.898 EUR, zvE = 19.898 - 36 - Vorsorge.
///  - Basis-KV 20 EUR: zvE 19.842, Steuer 1.600 (alt: max(20; 36) = 36, 19.862, 1.605)
///  - Basis-KV 36 EUR: zvE 19.826, Steuer 1.596 (alt 19.862, 1.605)
///  - Basis-KV 3.000 EUR: zvE 16.862, Steuer 879 (alt 16.898, 887)
///  - ohne Vorsorge (Kontrolle): zvE 19.862, Steuer 1.605 (alt gleich)
#[test]
fn rentner_pauschbetrag_steht_neben_der_vorsorge() {
    let fall = |name, kv, zve, est| Fall {
        name,
        scheibe: Scheibe::RentnerGesamt,
        paare: rentner_akte(kv),
        zve: Some(zve),
        est,
    };
    melde(&[
        fall("Basis-KV 20 (unter dem Pauschbetrag)", 20, 19_842, 1_600),
        fall("Basis-KV 36 (gleich dem Pauschbetrag)", 36, 19_826, 1_596),
        fall("Basis-KV 3.000", 3_000, 16_862, 879),
        fall("ohne Vorsorge (Kontrolle)", 0, 19_862, 1_605),
    ]);
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Engine unmittelbar (`gesamt_zve`, `gesamt`): Zusammenveranlagung und Grenzen, ohne den Umweg ueber Felder.

fn euro(v: i64) -> Euro {
    Euro::new(v)
}

/// 60.000 EUR Einkuenfte, VZ 2025, Einzelveranlagung (`beispiel_gesamtfall` mit anderem Betrag).
fn basis() -> GesamtfallEingabe {
    GesamtfallEingabe {
        einkuenfte_nichtselbststaendig: euro(60_000),
        ..beispiel_gesamtfall()
    }
}

/// Zusammenveranlagung: der Pauschbetrag verdoppelt sich. RV 10.000 EUR: zvE 60.000 - 72 - 10.000 = 49.928, Splitting 2 x Tarif(24.964) =
/// 5.834 (alt: 50.000, 5.854). Kontrolle derselben Rechnung ohne Vorsorge: 60.000 - 72 = 59.928.
#[test]
fn engine_zusammen_verdoppelt_den_pauschbetrag_neben_der_vorsorge() {
    let e = GesamtfallEingabe {
        zusammenveranlagung: true,
        vorsorge_gesamtbeitraege_inkl_ag: euro(12_000),
        vorsorge_ag_anteil_steuerfrei: euro(2_000),
        ..basis()
    };
    assert_eq!(gesamt_zve(&e, params()).unwrap(), euro(49_928));
    assert_eq!(gesamt(&e, params()).unwrap(), euro(5_834));
    let ohne = GesamtfallEingabe {
        zusammenveranlagung: true,
        ..basis()
    };
    assert_eq!(gesamt_zve(&ohne, params()).unwrap(), euro(59_928));
}

/// Das Feld `vorsorge_kv_pv` (Kranken- und Pflegeversicherung, Nr. 3 und 3a, schon mit der Grenze aus Abs. 4) steht wie die Rentenbeitraege
/// neben dem Pauschbetrag, nicht in der Konkurrenz mit ihm: 3.000 EUR, Einzel: zvE 60.000 - 36 - 3.000 = 56.964, Tarif 13.247 (alt: 57.000,
/// 13.260); zusammen: 60.000 - 72 - 3.000 = 56.928, Splitting 7.742 (alt: 57.000, 7.762). Mit 5.000 EUR uebrigen Sonderausgaben (mehr als
/// der Pauschbetrag) bleibt es bei 60.000 - 5.000 - 3.000 = 52.000, Tarif 11.407, wie nach der alten Lesart.
#[test]
fn engine_kv_pv_steht_neben_dem_pauschbetrag() {
    let einzel = GesamtfallEingabe {
        vorsorge_kv_pv: euro(3_000),
        ..basis()
    };
    assert_eq!(gesamt_zve(&einzel, params()).unwrap(), euro(56_964));
    assert_eq!(gesamt(&einzel, params()).unwrap(), euro(13_247));
    let zusammen = GesamtfallEingabe {
        zusammenveranlagung: true,
        ..einzel
    };
    assert_eq!(gesamt_zve(&zusammen, params()).unwrap(), euro(56_928));
    assert_eq!(gesamt(&zusammen, params()).unwrap(), euro(7_742));
    let uebrige = GesamtfallEingabe {
        sonderausgaben: euro(5_000),
        vorsorge_kv_pv: euro(3_000),
        ..basis()
    };
    assert_eq!(gesamt_zve(&uebrige, params()).unwrap(), euro(52_000));
    assert_eq!(gesamt(&uebrige, params()).unwrap(), euro(11_407));
}

/// Wo der Pauschbetrag gegen uebrige Sonderausgaben konkurriert, gilt das Maximum: uebrige 36 EUR = Pauschbetrag: 59.964 (Steuer 14.401);
/// uebrige 100 EUR und Vorsorge (RV) 100 EUR: 60.000 - 100 - 100 = 59.800 (Steuer 14.337). Beides aendert sich gegen die alte Lesart nicht.
#[test]
fn engine_uebrige_sonderausgaben_konkurrieren_mit_dem_pauschbetrag() {
    let gleich = GesamtfallEingabe {
        sonderausgaben: euro(36),
        ..basis()
    };
    assert_eq!(gesamt_zve(&gleich, params()).unwrap(), euro(59_964));
    assert_eq!(gesamt(&gleich, params()).unwrap(), euro(14_401));
    let beides = GesamtfallEingabe {
        sonderausgaben: euro(100),
        vorsorge_gesamtbeitraege_inkl_ag: euro(100),
        ..basis()
    };
    assert_eq!(gesamt_zve(&beides, params()).unwrap(), euro(59_800));
    assert_eq!(gesamt(&beides, params()).unwrap(), euro(14_337));
}
