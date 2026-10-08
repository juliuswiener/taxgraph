//! DBA-Freistellung (Abweichung Nr. 40, § 32b Abs. 1 S. 1 Nr. 3 `EStG`): die Rechnung wendet den Progressionsvorbehalt an,
//! in den Scheiben `gesamt` und `rentner_gesamt`. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Einkuenfte, die ein Abkommen in Deutschland freistellt, sind hier steuerfrei, heben aber den
//! Steuersatz fuer das uebrige Einkommen: der Satz ergibt sich aus Einkommen PLUS Auslandseinkuenfte (§ 32b Abs. 2 Nr. 2),
//! angewendet wird er auf das Einkommen ohne sie. Die Rechnung ermittelte den Betrag schon (`shared_dba_sonstige`), gab ihn
//! aber an keine Stelle weiter, die den Satz bestimmt; Python hat dieselbe Luecke.
//!
//! **Warum es zaehlt.** Bei 50.000 Euro zvE und 20.000 Euro freigestellten Einkuenften zeigte der Bescheid 10.691 statt
//! 13.205 Euro: 2.514 Euro zu wenig Steuer, ohne Hinweis (Vault `dba-freistellung-progressionsvorbehalt-wird-berechnet-aber-
//! nie-angewendet`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus dem Gesetzestext (`estg_p32b_2026-07-13.txt`) und `params/2025/
//! einkommensteuertarif_p32a.yaml`, nie aus dem Rust-Code gelesen.
//! - zvE 50.000 Euro = Lohn − Arbeitnehmer-Pauschbetrag 1.230 − Sonderausgaben-Pauschbetrag 36 (Lohn 51.266 Euro).
//! - Tarif 50.000: zone 3, z = (50.000 − 17.443) / 10.000 = 3,2557; (176,64 z + 2.397) z + 1.015,13 = 10.691,36, abgerundet
//!   10.691. Tarif 70.000: zone 4, 0,42 × 70.000 − 10.911,92 = 18.488,08, abgerundet 18.488.
//! - Besonderer Satz = 18.488 / 70.000; angewendet auf 50.000: 18.488 × 50.000 / 70.000 = 13.205,71, abgerundet 13.205
//!   (Rust rechnet in vollen Euro, wie der bestehende Weg fuer Lohnersatz: `p32b_1`).
//! - Die Catala-Kette `Familie3_dba_freistellung` rechnet dieselbe Formel in Cent (Satz exakt, Rundung beim Betrag auf den
//!   Cent): 13.205,71 Euro. Der Unterschied ist die Cent-Stelle, die Rust abschneidet (ein Euro-Betrag je Schritt, wie
//!   ueberall im Rechner).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types,
    clippy::too_many_lines
)]

use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::ausgaben::Kette;
use bescheid::zweige::Umgebung;
use bescheid::{Felder, Instanzquelle};
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";
const METHODE: &str = "dba_methode";

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

/// Die festzusetzende Einkommensteuer in vollen Euro und die Rechenweg-Kette (falls vorhanden). Erst der K2-Guard, dann
/// `feste_zahl`; jeder andere Ausgang ist ein Fehlschlag des Tests.
fn lauf(scheibe: Scheibe, paare: &[(&'static str, Value)]) -> (i64, Option<Kette>) {
    let f = fall(scheibe, paare);
    let q = Instanzquelle {
        store: Some(&f.store),
        bindung: Some(&f.index),
        nur_bestaetigt: false,
    };
    match an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q) {
        Ok(None) => {}
        Ok(Some(g)) => panic!("KONTROLLE: erwartet eine Zahl, bekommen die Sperre {g}"),
        Err(e) => panic!("KONTROLLE: Guard: {e:?}"),
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => {
            assert_eq!(z.zahl.get() % 100, 0, "Cent-Rest in der Steuer: {z:?}");
            (z.zahl.get() / 100, z.extras.kette)
        }
        Ok(Err(k)) => panic!("KONTROLLE: ohne Zahl: {:?}", k.grund),
        Err(e) => panic!("KONTROLLE: {e:?}"),
    }
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

/// § 32b Abs. 2: Steuer auf (zvE + Vorbehalt) geteilt durch (zvE + Vorbehalt), angewendet auf das zvE, in vollen Euro
/// abgerundet.
fn mit_vorbehalt(zve: i64, vorbehalt: i64) -> i64 {
    let erhoeht = zve + vorbehalt;
    i64::try_from(i128::from(tarif_2025(erhoeht)) * i128::from(zve) / i128::from(erhoeht)).unwrap()
}

// ---------------------------------------------------------------- Scheibe gesamt

/// Lohn 51.266 Euro, einzeln veranlagt: zvE 50.000 Euro (Pauschbetraege siehe Kopf).
fn gesamt(mehr: &[(&'static str, Value)]) -> (i64, Option<Kette>) {
    let mut p: Paare = vec![
        ("bruttoarbeitslohn", json!(cent(51_266))),
        ("veranlagung", json!("einzel")),
    ];
    p.extend(mehr.iter().cloned());
    lauf(Scheibe::Gesamt, &p)
}

/// KONTROLLE zuerst: ohne Auslandseinkuenfte ist das zvE 50.000 Euro und die Steuer der Tarif auf 50.000 (10.691 Euro).
/// Sonst belegten die Zahlen unten nichts: ein Fall, der schon ohne den Vorbehalt abweicht, liesse jeden Test gruen.
#[test]
fn kontrolle_ohne_auslandseinkuenfte_ist_das_zve_50_000_und_die_steuer_10_691() {
    let (steuer, kette) = gesamt(&[]);
    assert_eq!(tarif_2025(50_000), 10_691, "Handrechnung des Tarifs");
    assert_eq!(steuer, 10_691);
    if let Some(k) = kette {
        assert_eq!(k.zu_versteuerndes_einkommen.get(), 50_000);
    }
}

/// Das Beispiel aus dem Ticket: 20.000 Euro freigestellte Einkuenfte heben den Satz von 21,38 % auf 26,41 %. Die Steuer
/// steigt von 10.691 auf 13.205 Euro (2.514 Euro mehr).
#[test]
fn freistellung_hebt_den_satz_auf_den_der_summe() {
    assert_eq!(tarif_2025(70_000), 18_488, "Handrechnung des Tarifs");
    assert_eq!(mit_vorbehalt(50_000, 20_000), 13_205, "Handrechnung");
    let (steuer, _) = gesamt(&[
        (EINKUENFTE, json!(cent(20_000))),
        (METHODE, json!("dba_freistellung")),
    ]);
    assert_eq!(steuer, 13_205);
}

/// Dieselbe Freistellung ueber Staat und Einkunftsart (ohne Antwort auf `dba_methode`): USA und Oesterreich pauschal,
/// Polen nur fuer Ruhegehaelter.
#[test]
fn freistellung_ueber_staat_und_einkunftsart_rechnet_ebenso() {
    let faelle: [(&'static str, Option<&'static str>); 3] =
        [("us", None), ("at", None), ("pl", Some("ruhegehaelter"))];
    for (staat, art) in faelle {
        let mut mehr: Paare = vec![
            (EINKUENFTE, json!(cent(20_000))),
            ("dba_staat", json!(staat)),
        ];
        if let Some(a) = art {
            mehr.push(("dba_einkunftsart", json!(a)));
        }
        let (steuer, _) = gesamt(&mehr);
        assert_eq!(steuer, 13_205, "{staat} {art:?}");
    }
}

/// KONTROLLE zur Methode: bei Anrechnung oder ohne Abkommen heben dieselben Betraege den Satz NICHT (sie sind dort schon
/// im Einkommen enthalten und werden angerechnet). Ohne diese Kontrolle bestuende der Fix auch, wenn er jeden
/// Auslandsbetrag in den Vorbehalt schickte.
#[test]
fn bei_anrechnung_und_ohne_abkommen_bleibt_der_satz() {
    let (ohne, _) = gesamt(&[]);
    for (name, mehr) in [
        (
            "Methode Anrechnung",
            vec![
                (EINKUENFTE, json!(cent(20_000))),
                (METHODE, json!("dba_anrechnung")),
            ],
        ),
        ("kein Abkommen", vec![(EINKUENFTE, json!(cent(20_000)))]),
        (
            "Niederlande pauschal",
            vec![
                (EINKUENFTE, json!(cent(20_000))),
                ("dba_staat", json!("nl")),
            ],
        ),
    ] {
        let (steuer, _) = gesamt(&mehr);
        assert_eq!(
            steuer, ohne,
            "{name}: ohne gezahlte Steuer ist nichts anzurechnen"
        );
    }
}

/// Der Vorbehalt waechst mit dem Betrag, und die Rechnung folgt der Formel fuer jeden Betrag: Satz der Summe, angewendet
/// auf das Einkommen. Null Euro Auslandseinkuenfte aendern nichts.
#[test]
fn die_steuer_folgt_der_formel_fuer_jeden_betrag() {
    let mut vorher = gesamt(&[]).0;
    for euro in [0, 1_000, 5_000, 20_000, 60_000, 250_000] {
        let (steuer, _) = gesamt(&[
            (EINKUENFTE, json!(cent(euro))),
            (METHODE, json!("dba_freistellung")),
        ]);
        assert_eq!(steuer, mit_vorbehalt(50_000, euro), "{euro} Euro");
        assert!(steuer >= vorher, "{euro} Euro: die Steuer faellt nicht");
        vorher = steuer;
    }
}

/// BEKANNTE GRENZE, hier festgehalten und nicht gutgeheissen: erfasst jemand die 20.000 Euro auch im Lohn (zvE 70.000 statt
/// 50.000) UND als Freistellung, zaehlen sie doppelt. Die Steuer ist dann 20.912 Euro (Satz der Summe 90.000, angewendet auf
/// 70.000) statt der 13.205 Euro des richtigen Falls. Der Rechner kann die Doppelzaehlung nicht erkennen; schuetzen kann nur
/// der Fragetext (Test oben). ponytail: keine Plausibilitaetspruefung gegen den Lohn; Upgrade-Pfad: Lohn- und Auslandsbetrag
/// vergleichen und rueckfragen, sobald es einen Beleg gibt, wie oft das vorkommt.
#[test]
fn bekannte_grenze_wer_den_betrag_auch_im_lohn_erfasst_zaehlt_ihn_doppelt() {
    assert_eq!(mit_vorbehalt(70_000, 20_000), 20_912, "Handrechnung");
    let (steuer, _) = lauf(
        Scheibe::Gesamt,
        &[
            ("bruttoarbeitslohn", json!(cent(71_266))),
            ("veranlagung", json!("einzel")),
            (EINKUENFTE, json!(cent(20_000))),
            (METHODE, json!("dba_freistellung")),
        ],
    );
    assert_eq!(steuer, 20_912);
}

// ---------------------------------------------------------------- Scheibe rentner_gesamt

/// Rentnerin im Beginnjahr 2025 mit 20.000 Euro gesetzlicher Rente, einzeln veranlagt, alle Kreuze "nein" (die Angaben
/// des Pflicht-Kegels), plus `mehr`.
fn rentner(mehr: &[(&'static str, Value)]) -> (i64, Option<Kette>) {
    let mut p: Paare = vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(cent(20_000))),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_rentenfreibetrag", json!(0)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ];
    p.extend(mehr.iter().cloned());
    lauf(Scheibe::RentnerGesamt, &p)
}

/// Die Rentner-Scheibe rechnet denselben vollen Vorbehalt: die Steuer folgt der Formel mit dem zvE des Falls ohne
/// Auslandseinkuenfte. Das zvE aendert der Vorbehalt nicht.
#[test]
fn rentner_scheibe_rechnet_den_vorbehalt_ebenso() {
    let (ohne, kette) = rentner(&[]);
    let zve = kette
        .expect("KONTROLLE: Kette fehlt")
        .zu_versteuerndes_einkommen
        .get();
    assert!(
        zve > 12_096,
        "KONTROLLE: das zvE {zve} liegt im Tarifbereich"
    );
    assert_eq!(
        ohne,
        tarif_2025(zve),
        "KONTROLLE: Steuer ohne Auslandseinkuenfte"
    );
    for euro in [5_000, 20_000] {
        let (steuer, _) = rentner(&[
            (EINKUENFTE, json!(cent(euro))),
            (METHODE, json!("dba_freistellung")),
        ]);
        assert_eq!(steuer, mit_vorbehalt(zve, euro), "{euro} Euro, zvE {zve}");
        assert!(steuer > ohne, "{euro} Euro: der Vorbehalt hebt die Steuer");
    }
}
