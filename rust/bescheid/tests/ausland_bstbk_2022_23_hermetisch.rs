//! Auslandsfall aus dem Loesungsvorschlag der Bundessteuerberaterkammer (`BStBK`) zur Steuerberaterpruefung 2022/23, Teil I,
//! Sachverhalt 1, Veranlagungszeitraum 2024. Reiner Pruefstein, keine Abweichung vom Vorbild. Im Standardlauf, ohne `PARITY=1`,
//! ohne Python.
//!
//! **Worum es geht.** Eine Unternehmerin (geschieden, einzeln veranlagt, wohnt in Duesseldorf, hat auch einen Wohnsitz in
//! Italien) hat Auslandsbezug aus zwei Richtungen. Italien stellt nach dem Abkommen (DBA) Einkuenfte steuerfrei; sie heben aber
//! den Steuersatz fuer das uebrige Einkommen (Progressionsvorbehalt, § 32b `EStG`). Aus Brasilien (kein Abkommen) kommt eine
//! Dividende mit einbehaltener Quellensteuer, die auf die deutsche Steuer angerechnet wird (§ 34c Abs. 1 `EStG`). Die Kammer
//! rechnet den Fall bis zur festzusetzenden Einkommensteuer von 51.980,00 Euro durch.
//!
//! **Warum es zaehlt.** Die Zahl stammt nicht aus unserem Code, sondern von Fachleuten, und sie prueft zwei Auslandsregeln in
//! einer Rechnung. Das Backlog-Ticket `ausland-bstbk-sachverhalt-1-vz-2024-als-bescheid-test` (Vault) verlangt sie als Bescheid-Test.
//!
//! **Das ist ein TEILFALL, und das beweist er NICHT.**
//! 1. Er beweist nicht das Zusammenwirken von Freistellung und Anrechnung in EINER Akte. Der Bescheid-Weg rechnet je Akte eine
//!    Methode und einen Staat (`shared_dba_sonstige`: Freistellung ODER Anrechnung). Der Test fuehrt deshalb die zwei Haelften
//!    getrennt durch den Weg: Italien als Akte mit Freistellung (Test 3), Brasilien als Akte mit Anrechnung (Test 5).
//! 2. Die festzusetzende Einkommensteuer des Vollfalls entsteht hier nur durch eine Subtraktion IM TEST (Steuer der
//!    Italien-Haelfte minus Anrechnung der Brasilien-Haelfte, Test 6), nicht im Bescheid. Kein Bescheid-Aufruf liefert sie.
//! 3. Der Vollfall in einer Akte wird heute abgewiesen: Wer `dba_mehrere_staaten` bestaetigt, bekommt die Sperre
//!    `dba_multi_country_offen` statt einer Zahl (Test 7; Stelle `deklaration/sperre/einkunft.rs::dba_p32b_p16`). Das ist
//!    eine bekannte Grenze (Hilfetext des Feldes: "Die DBA-Anrechnung ist in Stufe-1 auf EINEN Staat beschraenkt"), kein Defekt.
//!    Der Befehl, der die Abweisung zeigt:
//!    `cd rust && cargo nextest run -p bescheid --test it ausland_bstbk_2022_23_hermetisch::bekannte_grenze_der_vollfall`
//!
//! **BEFUND: Die Zahlen der Loesung und der Bescheid-Weg weichen um 25 Euro ab, und die Ursache ist die Tarifkonstante.**
//! Die Loesung rechnet "0,42 x X - 10.602,13" (PDF-Seite 7). Diese Konstante gehoert zum Grundfreibetrag 11.604 Euro, das ist
//! der Tarif des Jahres 2024 VOR dem rueckwirkenden Gesetz vom Dezember 2024 (abgeleitet: `params/derive_coefficients.py`
//! mit Grundfreibetrag 11.604 ergibt 10.602,13; mit 11.784 ergibt es 10.636,31). Der Bescheid-Weg rechnet mit der Catala-Regel
//! `rules/estg/p32a/einkommensteuertarif.catala_en` (eingecheckt als `rust/catala-sys/generated/Einkommensteuertarif.c`), Block
//! VZ2024: Grundfreibetrag 11.784, Konstante 10.636,31; dieselben Zahlen stehen in `params/2024/einkommensteuertarif_p32a.yaml`
//! ("Endfassung ist massgeblich", `BGBl` 2024 I Nr. 386). Die Loesung nennt sich "Rechtsstand 2025"; ob sie die Aenderung uebersehen
//! //! hat oder der Tarif des Repos falsch waere, entschied der Gesetzestext der Fassung fuer 2024. NACHGEPRUEFT durch main am
//! 2026-10-15: Die Fassung des § 32a EStG fuer den Veranlagungszeitraum 2024 (gueltig bis 31.12.2024, geaendert erst durch
//! `BGBl` 2024 I Nr. 449) nennt Grundfreibetrag 11.784 und "0,42 x X - 10.636,31", wie das Repo; gelesen auf der Fassung bei
//! buzer.de (Textspiegel eines Dritten, nicht das `BGBl` selbst; der Wortlaut liegt weiter nicht in `sources/`). Die Loesung
//! rechnet also mit dem Tarif vor der rueckwirkenden Anhebung. Die Regel nennt ihre 2024-Werte "aus den GETTSIM-Zonenparametern
//! abgeleitet, Methode gegen die VZ-2026-Literalfassung validiert". Folge fuer diesen Test: Die
//! Zahlen der Loesung stehen unveraendert als Soll in der Rechnung mit IHRER Konstante (Test 1, gruen), der Bescheid-Weg wird
//! gegen dieselbe Rechnung mit der Konstante der Regel gehalten (Tests 2 bis 6), und der Abstand zur Zahl der Loesung steht
//! als Zahl im Test (25 Euro bei der festzusetzenden Steuer). Die Erwartung ist nirgends an den Rust-Code angepasst. Beleg, dass
//! allein die Konstante den Abstand macht: setzt man im C-Tarif 10.602,13 statt 10.636,31 ein, liefert der Bescheid-Weg die
//! 66.980 der Loesung (Mutant M8 der Nachmessung, `audits/messbelege/nm-ausland-bstbk`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Loesungsvorschlag der `BStBK` zur Klausur "Einkommensteuer- und Ertragsteuerrecht", Steuerberaterpruefung
//! 2022/2023, Fundstelle Bundessteuerblatt `BStBl` I 2025 Seite 1200 vom 30. Mai 2025 (Rechtsstand 2025, gerechnet fuer VZ 2024),
//! Teil I Sachverhalt 1, PDF-Seiten 2 bis 7 (Seitenzaehlung der Datei: "Seite 1 von 26" bis "Seite 6 von 26"). Das PDF liegt nicht im
//! Repo (Rechte bei der `BStBK`; Julius 2026-10-15: die Lizenz ist vorerst egal, der Sachverhalt steht trotzdem in eigenen Worten).
//! SHA256 der abgerufenen Datei `6d26297a11260eb711b8606c30cfa3edf9eb5fcb8f26ab0dea33174b0d38f407`, abgerufen 2026-10-15 von
//! bstbk.de. Die Kammer nennt ihren Vorschlag selbst "unverbindlich" und "nicht die offizielle Loesung"; der Aufgabentext
//! (Bundessteuerblatt) lag nicht vor, der Sachverhalt ist aus der Loesung rekonstruiert.
//!
//! Eingabe -> Fundstelle (PDF-Seite):
//! | Eingabe | Wert | Feld | Seite |
//! |---|---|---|---|
//! | Gewinn aus Gewerbebetrieb im Inland, "vorlaeufig ermittelt laut Sachverhalt" | 140.000 | (Teil von `einkuenfte_gewinn`) | 3 |
//! | Gewinnausschuettung der brasilianischen Gesellschaft, voll steuerpflichtig (kein Teileinkuenfteverfahren) | 50.000 | (Teil von `einkuenfte_gewinn`) | 4 |
//! | Summe der Einkuenfte (140.000 + 50.000) | 190.000 | `einkuenfte_gewinn` | 6 |
//! | Unterhalt an den geschiedenen Ehemann, 12 x 750 Euro, Besteuerung beim Empfaenger nachgewiesen | 9.000 | `realsplitting_unterhaltsleistungen`, `realsplitting_zustimmung` | 6 |
//! | Vorsorgeaufwendungen "schlicht genannt" | 2.500 | `basis_kv` | 6 |
//! | Betriebsstaette Italien (Gewinn 16.000 + 60 % der Dividende 4.000), freigestellt, Progressionsvorbehalt | 18.400 | (Teil von `dba_auslaendische_einkuenfte`) | 4 |
//! | Veraeusserungsgewinn Wohnung Italien (§ 23 `EStG`), freigestellt, Progressionsvorbehalt | 40.000 | (Teil von `dba_auslaendische_einkuenfte`) | 5 |
//! | Progressionseinkuenfte Italien zusammen | 58.400 | `dba_auslaendische_einkuenfte` | 7 |
//! | Einbehaltene brasilianische Quellensteuer, 30 % von 50.000 | 15.000 | `dba_gezahlte_auslaendische_steuer` | 7 |
//! | Vermietung Italien 5.000 (EU, § 32b Abs. 1 Satz 2 Nr. 3: weder besteuert noch im Vorbehalt) | 0 | nicht eingegeben | 4 |
//! | Verlust Ferienwohnung Brasilien -5.000 (§ 2a `EStG`: nicht ausgleichbar) | 0 | nicht eingegeben | 4 |
//!
//! Zahlen der Loesung (PDF-Seite 6 und 7): Summe der Einkuenfte 190.000; zu versteuerndes Einkommen 178.500; Steuer auf Einkommen
//! plus Progressionseinkuenfte (236.900): 0,42 x 236.900 - 10.602,13 = 88.895,87, abgerundet 88.895; tarifliche Einkommensteuer
//! auf 178.500 mit diesem Satz 88.895 / 236.900 x 178.500 = 66.980,00; Hoechstbetrag § 34c 66.980 x 50.000 / 178.500 = 18.761,00;
//! angerechnet 15.000; festzusetzende Einkommensteuer 51.980,00. Die Loesung rundet jeden Zwischenschritt auf volle Euro ab;
//! die Handrechnung unten tut dasselbe.
//!
//! Annahmen, die NICHT aus der Loesung stammen: Die Art der 2.500 Euro Vorsorge nennt die Loesung nicht; der Test setzt sie als
//! Beitrag zur Basis-Krankenversicherung (voll abziehbar, unter dem Hoechstbetrag), das Ergebnis haengt nicht von der Art ab. Die
//! Zustimmung des Empfaengers zum Unterhaltsabzug setzt die Loesung voraus ("Besteuerung beim Empfaenger nachgewiesen"); das Feld
//! `realsplitting_zustimmung` steht deshalb auf ja. Die Loesung rechnet keine Gewerbesteuer-Ermaessigung (§ 35 `EStG`), keinen
//! Solidaritaetszuschlag und keine Kirchensteuer; der Test auch nicht (festzusetzende Einkommensteuer).
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
use domain::{Euro, Feldtyp, Scheibe, Sperrgrund, Vz};
use engine::zugriff::teil2::sonstige::{p34c_1, AuslaendischeSteuerEingabe};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2024;

/// Die Params-Datei des Tarifs 2024 (Quelle der Handrechnungen der Tests im Repo) und die Catala-Regel, aus der der eingecheckte
/// C-Tarif des Bescheid-Wegs entsteht; der Test haelt die Konstante der Handrechnung gegen beide.
const TARIF_2024_YAML: &str = include_str!("../../../params/2024/einkommensteuertarif_p32a.yaml");
const TARIF_2024_CATALA: &str = include_str!("../../../rules/estg/p32a/einkommensteuertarif.catala_en");

// Zahlen der Loesung (PDF-Seite 6 und 7), in vollen Euro.
const SUMME_DER_EINKUENFTE: i64 = 190_000;
const ZVE: i64 = 178_500;
const SUMME_MIT_VORBEHALT: i64 = 236_900;
const L_STEUER_AUF_SUMME: i64 = 88_895;
const L_TARIFLICHE_EST: i64 = 66_980;
const L_HOECHSTBETRAG_34C: i64 = 18_761;
const L_ANGERECHNET: i64 = 15_000;
const L_FESTZUSETZENDE_EST: i64 = 51_980;

/// Zone-4-Konstante in Cent: "0,42 x X - K". Loesung: 10.602,13 Euro; Catala-Regel und Params-Datei 2024: 10.636,31 Euro.
const KONSTANTE_LOESUNG: i64 = 1_060_213;
const KONSTANTE_PARAMS_2024: i64 = 1_063_631;

const PROGRESSION_ITALIEN: i64 = 58_400;
const AUSLAND_BRASILIEN: i64 = 50_000;
const QUELLENSTEUER_BRASILIEN: i64 = 15_000;

type Paare = Vec<(&'static str, Value)>;

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// `dict(paare)`: der letzte Wert je Feld gewinnt, die Reihenfolge ist die des ersten Auftretens.
fn als_dict(paare: &[(&'static str, Value)]) -> Paare {
    let mut raus: Paare = Vec::new();
    for (f, w) in paare {
        match raus.iter_mut().find(|(g, _)| g == f) {
            Some(e) => e.1 = w.clone(),
            None => raus.push((f, w.clone())),
        }
    }
    raus
}

/// Der Abwesenheitswert, nie der illustrative Beispielwert (wie in `dba_freistellung_progression.rs`).
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
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(&'static str, Value)]) -> Paare {
    let gesetzt = als_dict(gesetzt);
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .find(|(g, _)| g == f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            (*f, w)
        })
        .collect();
    raus.extend(gesetzt.into_iter().filter(|(f, _)| !kegel.contains(f)));
    raus
}

/// Ein Fall der Scheibe, mit der Scheiben-Bindung als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(paare: &[(&'static str, Value)]) -> Fall {
    let cfg = Cfg::fuer(Scheibe::Gesamt);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe gesamt liest Felder aus {d}"))
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

/// Ausgang eines Laufs: erst der Guard, dann `feste_zahl`.
#[derive(Debug)]
enum Lauf {
    Zahl { steuer_euro: i64, kette: Kette },
    Sperre(Sperrgrund),
    Anderes(String),
}

fn lauf(paare: &[(&'static str, Value)]) -> Lauf {
    let f = fall(paare);
    if f.cfg.guard() {
        let q = Instanzquelle {
            store: Some(&f.store),
            bindung: Some(&f.index),
            nur_bestaetigt: false,
        };
        if let Some(g) = an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q).unwrap() {
            return Lauf::Sperre(g);
        }
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None).unwrap() {
        Ok(z) => {
            assert_eq!(z.zahl.get() % 100, 0, "Cent-Rest in der Steuer: {z:?}");
            match z.extras.kette {
                Some(kette) => Lauf::Zahl {
                    steuer_euro: z.zahl.get() / 100,
                    kette,
                },
                None => Lauf::Anderes("Zahl ohne Rechenweg-Kette".to_owned()),
            }
        }
        Err(k) => Lauf::Anderes(format!("keine Zahl: {:?}", k.grund)),
    }
}

/// Steuer in vollen Euro und Rechenweg-Kette, oder ein Fehlschlag des Tests mit dem Grund.
fn zahl(paare: &[(&'static str, Value)]) -> (i64, Kette) {
    match lauf(paare) {
        Lauf::Zahl { steuer_euro, kette } => (steuer_euro, kette),
        Lauf::Sperre(g) => panic!("KONTROLLE: erwartet eine Zahl, bekommen die Sperre {g}"),
        Lauf::Anderes(was) => panic!("KONTROLLE: erwartet eine Zahl, bekommen: {was}"),
    }
}

/// Inlaendischer Teil der Akte: Gewinn 190.000 (140.000 + 50.000), Unterhalt 9.000, Vorsorge 2.500, einzeln veranlagt.
fn ohne_ausland() -> Paare {
    vec![
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(false)),
        ("gewinn_betriebsart", json!("gewerbe")),
        ("einkuenfte_gewinn", json!(cent(SUMME_DER_EINKUENFTE))),
        ("realsplitting_zustimmung", json!(true)),
        ("realsplitting_unterhaltsleistungen", json!(cent(9_000))),
        ("basis_kv", json!(cent(2_500))),
    ]
}

/// Erste Haelfte: Italien stellt frei, 58.400 Euro heben den Satz.
fn mit_italien() -> Paare {
    let mut p = ohne_ausland();
    p.extend([
        ("dba_staat", json!("Italien")),
        ("dba_methode", json!("dba_freistellung")),
        ("dba_auslaendische_einkuenfte", json!(cent(PROGRESSION_ITALIEN))),
    ]);
    p
}

/// Zweite Haelfte: Brasilien, kein Abkommen, 50.000 Euro Dividende (Teil des Gewinns), 15.000 Euro Quellensteuer, Anrechnung.
fn mit_brasilien() -> Paare {
    let mut p = ohne_ausland();
    p.extend([
        ("dba_staat", json!("sonstiger_staat")),
        ("dba_methode", json!("kein_dba")),
        ("dba_auslaendische_einkuenfte", json!(cent(AUSLAND_BRASILIEN))),
        (
            "dba_gezahlte_auslaendische_steuer",
            json!(cent(QUELLENSTEUER_BRASILIEN)),
        ),
    ]);
    p
}

/// Die Rechnung der Loesung in ihren Schritten, von Hand und in vollen Euro abgerundet wie die Loesung, mit der Zone-4-Konstante
/// `konstante_cent` ("0,42 x X - K", K in Cent). Zone 4 gilt von 66.761 bis 277.825 Euro (VZ 2024); in Cent: 0,42 x X Euro = 42 x X Cent.
#[derive(Debug, PartialEq, Eq)]
struct Rechnung {
    /// Tarif auf das zu versteuernde Einkommen allein (Akte ohne Vorbehalt).
    tarif_auf_zve: i64,
    /// Tarif auf zvE plus Progressionseinkuenfte (236.900).
    tarif_auf_summe: i64,
    /// Italien-Haelfte: `tarif_auf_summe` x zvE / Summe, die tarifliche Steuer mit Vorbehalt.
    tarif_mit_vorbehalt: i64,
    /// Hoechstbetrag § 34c fuer Brasilien auf Basis von `tarif_mit_vorbehalt`.
    hoechstbetrag: i64,
    angerechnet: i64,
    /// `tarif_mit_vorbehalt` minus `angerechnet`: die festzusetzende Steuer des Vollfalls.
    festzusetzende_est: i64,
    /// Brasilien-Haelfte allein (ohne Italien): Hoechstbetrag auf Basis von `tarif_auf_zve`.
    brasilien_hoechstbetrag: i64,
    brasilien_festzusetzende_est: i64,
}

fn rechnung(konstante_cent: i64) -> Rechnung {
    let zone4 = |x: i64| {
        assert!((66_761..=277_825).contains(&x), "Zone 4 gilt hier nicht");
        (42 * x - konstante_cent) / 100
    };
    let tarif_auf_zve = zone4(ZVE);
    let tarif_auf_summe = zone4(SUMME_MIT_VORBEHALT);
    let tarif_mit_vorbehalt = tarif_auf_summe * ZVE / SUMME_MIT_VORBEHALT;
    let hoechstbetrag = tarif_mit_vorbehalt * AUSLAND_BRASILIEN / ZVE;
    let angerechnet = QUELLENSTEUER_BRASILIEN.min(hoechstbetrag);
    let brasilien_hoechstbetrag = tarif_auf_zve * AUSLAND_BRASILIEN / ZVE;
    Rechnung {
        tarif_auf_zve,
        tarif_auf_summe,
        tarif_mit_vorbehalt,
        hoechstbetrag,
        angerechnet,
        festzusetzende_est: tarif_mit_vorbehalt - angerechnet,
        brasilien_hoechstbetrag,
        brasilien_festzusetzende_est: tarif_auf_zve
            - QUELLENSTEUER_BRASILIEN.min(brasilien_hoechstbetrag),
    }
}

/// Der Baustein § 34c Abs. 1 fuer die brasilianische Seite, gespeist mit `tariflich` als deutscher Steuer auf das Einkommen.
fn anrechnung(gezahlt: i64, tariflich: i64) -> i64 {
    p34c_1(&AuslaendischeSteuerEingabe {
        gezahlte_auslaendische_steuer: Euro::new(gezahlt),
        deutsche_est_inkl_ausl: Euro::new(tariflich),
        zu_versteuerndes_einkommen: Euro::new(ZVE),
        auslaendische_einkuenfte_staat: Euro::new(AUSLAND_BRASILIEN),
    })
    .unwrap()
    .get()
}

// ------------------------------------------------------------------------------------------------ Test 1: die Loesung selbst

/// Test 1: Die Handrechnung mit der Konstante der Loesung gibt jede gedruckte Zahl der Loesung wieder (88.895 / 66.980 / 18.761 /
/// 15.000 / 51.980). So liest der Test die Zahlen richtig, und die Soll-Werte stehen unveraendert da. Dazu die Summen der
/// Loesung: 190.000 - 9.000 - 2.500 = 178.500; 178.500 + 58.400 = 236.900; 18.400 + 40.000 = 58.400; 30 % von 50.000 = 15.000.
#[test]
fn die_rechenschritte_der_loesung_sind_mit_ihrer_konstante_nachgerechnet_stimmig() {
    assert_eq!(SUMME_DER_EINKUENFTE - 9_000 - 2_500, ZVE);
    assert_eq!(ZVE + PROGRESSION_ITALIEN, SUMME_MIT_VORBEHALT);
    assert_eq!(18_400 + 40_000, PROGRESSION_ITALIEN);
    assert_eq!(QUELLENSTEUER_BRASILIEN, AUSLAND_BRASILIEN * 30 / 100);
    let r = rechnung(KONSTANTE_LOESUNG);
    assert_eq!(r.tarif_auf_summe, L_STEUER_AUF_SUMME);
    assert_eq!(r.tarif_mit_vorbehalt, L_TARIFLICHE_EST);
    assert_eq!(r.hoechstbetrag, L_HOECHSTBETRAG_34C);
    assert_eq!(r.angerechnet, L_ANGERECHNET);
    assert_eq!(r.festzusetzende_est, L_FESTZUSETZENDE_EST);
}

/// Die Konstante der Handrechnung `KONSTANTE_PARAMS_2024` (10.636,31) steht in der Params-Datei UND im VZ2024-Block der
/// Catala-Regel, aus der der C-Tarif des Bescheid-Weges entsteht, jeweils mit dem Grundfreibetrag 11.784 (der Tarif der Loesung
/// gehoert dagegen zu 11.604, siehe Kopf). KONTROLLE: ohne diese Bindung koennte die Handrechnung eine Konstante tragen, die der
/// Weg gar nicht benutzt. Dass der Weg SELBST diese Konstante rechnet, messen die Tests 2 bis 6 an fuenf Punkten.
#[test]
fn die_konstante_der_handrechnung_steht_in_der_params_datei_und_der_catala_regel_2024() {
    let zeile = TARIF_2024_YAML
        .lines()
        .find(|z| z.starts_with("zone4_abzug:"))
        .expect("KONTROLLE: Zeile zone4_abzug fehlt in der Params-Datei");
    assert!(
        zeile.contains("wert: 10636.31,"),
        "KONTROLLE: Konstante der Params-Datei: {zeile}"
    );
    assert!(
        TARIF_2024_YAML.contains("wert: 11784"),
        "KONTROLLE: Grundfreibetrag 2024 in der Params-Datei"
    );
    let anfang = TARIF_2024_CATALA
        .find("-- VZ2024: Tarifparameter {")
        .expect("KONTROLLE: Block VZ2024 fehlt in der Catala-Regel");
    let ende = TARIF_2024_CATALA[anfang..]
        .find("-- VZ2025:")
        .expect("KONTROLLE: Block VZ2025 fehlt in der Catala-Regel");
    let block = &TARIF_2024_CATALA[anfang..anfang + ende];
    assert!(block.contains("-- zone4_abzug: 10636.31\n"), "KONTROLLE: Block VZ2024: {block}");
    assert!(block.contains("-- grundfreibetrag: 11784.0\n"), "KONTROLLE: Block VZ2024: {block}");
    assert_eq!(KONSTANTE_PARAMS_2024, 1_063_631);
}

// ------------------------------------------------------------------------------------------------ Tests 2 bis 6: der Weg

/// Test 2: KONTROLLE zuerst: ohne Auslandsposten kommt der Bescheid-Weg auf die Summe der Einkuenfte (190.000) und das zu versteuernde
/// Einkommen (178.500) der Loesung, beide EXAKT. Die Steuer ist der Tarif auf 178.500 mit der Konstante der Params-Datei (64.333);
/// mit der Konstante der Loesung waeren es 64.367. Sonst belegten die Zahlen unten nichts: ein Fall, der schon im Inland
/// abweicht, liesse jeden Auslandstest aus dem falschen Grund gruen oder rot.
#[test]
fn kontrolle_ohne_auslandsposten_sind_summe_und_zve_die_der_loesung() {
    let (steuer, k) = zahl(&ohne_ausland());
    assert_eq!(k.gesamtbetrag_der_einkuenfte.get(), SUMME_DER_EINKUENFTE);
    assert_eq!(k.zu_versteuerndes_einkommen.get(), ZVE);
    let params2024 = rechnung(KONSTANTE_PARAMS_2024);
    let loesung = rechnung(KONSTANTE_LOESUNG);
    assert_eq!((params2024.tarif_auf_zve, loesung.tarif_auf_zve), (64_333, 64_367));
    assert_eq!(steuer, params2024.tarif_auf_zve);
    assert_eq!(k.tarifliche_est.get(), params2024.tarif_auf_zve);
}

/// Test 3: Italien-Haelfte: 58.400 Euro freigestellte Einkuenfte heben den Satz. Das zu versteuernde Einkommen bleibt 178.500 (Zahl der
/// Loesung, exakt). Die Steuer ist die Rechnung der Loesung mit der Konstante der Params-Datei: Tarif auf 236.900 = 88.861, davon
/// der Satz auf 178.500 = 66.955. Die Zahl der Loesung ist 66.980; der Abstand von 25 Euro kommt aus der Tarifkonstante (BEFUND im
/// Kopf). Diese Haelfte hat keine Anrechnung; `kette.tarifliche_est` ist der Tarif OHNE Vorbehalt, die Steuer der Akte steht in
/// `festzusetzende_est`.
#[test]
fn italien_haelfte_rechnet_den_vorbehalt_wie_die_loesung_mit_der_konstante_der_params() {
    let (steuer, k) = zahl(&mit_italien());
    let r = rechnung(KONSTANTE_PARAMS_2024);
    assert_eq!(k.gesamtbetrag_der_einkuenfte.get(), SUMME_DER_EINKUENFTE);
    assert_eq!(k.zu_versteuerndes_einkommen.get(), ZVE, "der Vorbehalt aendert das zvE nicht");
    assert_eq!((r.tarif_auf_summe, r.tarif_mit_vorbehalt), (88_861, 66_955));
    assert_eq!(k.festzusetzende_est.get(), r.tarif_mit_vorbehalt);
    assert_eq!(steuer, r.tarif_mit_vorbehalt);
    assert_eq!(L_TARIFLICHE_EST - steuer, 25, "Abstand zur Zahl der Loesung: nur die Tarifkonstante");
}

/// Test 4: Brasilien-Haelfte als Baustein § 34c Abs. 1 mit den Zahlen der Loesung: bei der tariflichen Steuer 66.980 der Loesung ist der
/// Hoechstbetrag 18.761, die 15.000 Euro Quellensteuer liegen darunter und werden voll angerechnet. KONTROLLE: bei einer hoeheren
/// gezahlten Steuer (30.000) deckelt der Hoechstbetrag auf genau 18.761; sonst bewiese die Anrechnung von 15.000 nur, dass der
/// Baustein die gezahlte Steuer durchreicht. Dieser Test ist von der Tarifkonstante unabhaengig: er nimmt die Steuer als Eingabe.
#[test]
fn brasilien_haelfte_rechnet_hoechstbetrag_und_anrechnung_der_loesung() {
    assert_eq!(anrechnung(QUELLENSTEUER_BRASILIEN, L_TARIFLICHE_EST), L_ANGERECHNET);
    assert_eq!(anrechnung(30_000, L_TARIFLICHE_EST), L_HOECHSTBETRAG_34C);
    assert_eq!(anrechnung(0, L_TARIFLICHE_EST), 0);
}

/// Test 5: Brasilien-Haelfte ueber den ganzen Bescheid-Weg (nicht nur den Baustein): die Akte hat NUR die brasilianische Dividende mit
/// 15.000 Euro Quellensteuer, ohne Italien. Die Zahlen sind NICHT die der Loesung (ihr Satz enthaelt den Vorbehalt), sondern die
/// Handrechnung mit der Konstante der Params-Datei: Tarif auf 178.500 = 64.333, Hoechstbetrag 64.333 x 50.000 / 178.500 = 18.020,
/// die 15.000 Euro liegen darunter, festzusetzende Einkommensteuer 64.333 - 15.000 = 49.333.
#[test]
fn brasilien_haelfte_ueber_den_ganzen_weg_rechnet_die_anrechnung() {
    let r = rechnung(KONSTANTE_PARAMS_2024);
    assert_eq!((r.brasilien_hoechstbetrag, r.brasilien_festzusetzende_est), (18_020, 49_333));
    let (steuer, k) = zahl(&mit_brasilien());
    assert_eq!(k.zu_versteuerndes_einkommen.get(), ZVE);
    assert_eq!(k.tarifliche_est.get(), r.tarif_auf_zve);
    assert_eq!(k.festzusetzende_est.get(), r.brasilien_festzusetzende_est);
    assert_eq!(steuer, r.brasilien_festzusetzende_est);
}

/// Test 6: Die zwei Haelften zusammen: Steuer der Italien-Haelfte (vom Bescheid-Weg) minus Anrechnung der Brasilien-Haelfte (Baustein,
/// gespeist mit dieser Steuer). Das ist die Rechnung der Loesung mit der Konstante der Params-Datei: 66.955 - 15.000 = 51.955; die
/// Loesung hat 51.980 (Abstand 25). Die Subtraktion geschieht HIER IM TEST, nicht im Bescheid: kein Bescheid-Aufruf liefert diese Zahl.
#[test]
fn die_haelften_zusammen_ergeben_die_rechnung_der_loesung_mit_der_konstante_der_params() {
    let (steuer_italien, _) = zahl(&mit_italien());
    let angerechnet = anrechnung(QUELLENSTEUER_BRASILIEN, steuer_italien);
    let r = rechnung(KONSTANTE_PARAMS_2024);
    assert_eq!(angerechnet, r.angerechnet);
    assert_eq!(angerechnet, L_ANGERECHNET, "voll angerechnet, wie in der Loesung");
    assert_eq!(steuer_italien - angerechnet, r.festzusetzende_est);
    assert_eq!(r.festzusetzende_est, 51_955);
    assert_eq!(L_FESTZUSETZENDE_EST - r.festzusetzende_est, 25, "Abstand zur Zahl der Loesung");
}

// ------------------------------------------------------------------------------------------------ Test 7: die Grenze

/// Test 7: BEKANNTE GRENZE, hier festgehalten und nicht gutgeheissen: der Fall der Loesung in EINER Akte (Freistellung Italien UND
/// Anrechnung Brasilien) laeuft nicht durch den Bescheid-Weg. Wer bestaetigt, Einkuenfte aus mehr als einem Staat zu haben
/// (`dba_mehrere_staaten`), bekommt die Sperre `dba_multi_country_offen` statt einer Zahl. Faellt die Sperre einmal, muss dieser
/// Test eine Zahl verlangen: 51.955 nach dem Tarif der Params-Datei (51.980 nach der Konstante der Loesung, siehe BEFUND im Kopf).
/// ponytail: ein Staat und eine Methode je Akte; Upgrade-Pfad: Methode und Betrag je Staat (Julius entscheidet die Klasse der
/// nachgebauten Python-Ausnahmen spaeter, Vault `eine-zahl-als-staat-der-auslandseinkuenfte-bleibt-ein-fehler-bis-die-ganze-klasse-
/// entschieden-wird`).
#[test]
fn bekannte_grenze_der_vollfall_in_einer_akte_ist_gesperrt_statt_falsch_gerechnet() {
    let mut p = mit_italien();
    p.push((
        "dba_gezahlte_auslaendische_steuer",
        json!(cent(QUELLENSTEUER_BRASILIEN)),
    ));
    // KONTROLLE: ohne das Feld `dba_mehrere_staaten` rechnet der Weg eine Zahl (die Sperre kommt vom Feld, nicht von der Akte).
    assert!(
        matches!(lauf(&p), Lauf::Zahl { .. }),
        "KONTROLLE: ohne dba_mehrere_staaten erwartet eine Zahl"
    );
    p.push(("dba_mehrere_staaten", json!(true)));
    match lauf(&p) {
        Lauf::Sperre(g) => assert_eq!(g, Sperrgrund::DbaMultiCountryOffen),
        Lauf::Zahl { steuer_euro, .. } => {
            panic!("KONTROLLE: erwartet die Sperre dba_multi_country_offen, bekommen die Zahl {steuer_euro}")
        }
        Lauf::Anderes(was) => {
            panic!("KONTROLLE: erwartet die Sperre dba_multi_country_offen, bekommen: {was}")
        }
    }
}
