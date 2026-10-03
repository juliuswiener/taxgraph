//! Sonderausgaben, Kranken- und Pflegeversicherung, aussergewoehnliche Belastungen, eigener Pauschbetrag und
//! Kinderbetreuung: die Abbildung Feld -> Eingabe in `abzuege.rs` (`shared_steuer_sonder_agb`) im Standardlauf (ohne
//! `PARITY=1`, ohne Python).
//!
//! Die Rechenkerne (`engine`) rechnen richtig, wenn man ihnen die richtigen Werte gibt. Dass die Bescheid-Schicht ihnen die
//! richtigen Werte gibt, pruefte fuer diese Funktionen bisher nur Parity (`bescheid_blatt_paritaet.rs`, nur mit `PARITY=1`;
//! die CI faehrt Parity nicht). Gemessen am 2026-10-03 auf 57944f48 (Bericht h8-hermetisch3): 13 Mutationen am Aufrufort in
//! `abzuege.rs` (`sonderausgaben` 3, `kv_pv_sonderausgaben` 2, `aussergewoehnliche_belastungen` 3, `eigener_pb` 2,
//! `kinderbetreuung_summe` 3). 11 lassen `cargo test -p bescheid` gruen (181 passed, 0 failed, 10 ignored): Spenden ohne
//! Gesamtbetrag der Einkuenfte, Partner-KV/PV nur bei Einzelveranlagung, Kirchensteuer gezahlt/erstattet vertauscht,
//! Partner liest die Felder von Person A (KV/PV und eigener Pauschbetrag), Wahlrecht Einzelnachweis ohne Abzug des
//! Pauschbetrags, Partner-Kette auch bei Einzelveranlagung, `notwendig und angemessen` = nein sperrt nicht, `hilflos`
//! ignoriert, Kinderbetreuung ohne Cent-zu-Euro und nur das letzte Kind. Eine (`mit_anspruch_auf_zuschuss` immer false) wird
//! nur zufaellig rot: sie loescht die einzige Lesestelle eines Feld-Kennung-Literals, und die Mindestzahl in
//! `feld_kennung_gate.rs` (141 < 142) schlaegt an, nicht das Verhalten. Eine (das Gate `kind_unter_14_haushaltszugehoerig`
//! entfaellt) faengt schon ein Bestandstest. Mit diesen Tests werden alle 13 rot, jede mit genau einem der vier neuen Tests.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl ist die Ausgabe des Python-Orakels
//! `bescheid_abzuege._shared_steuer_sonder_agb` (ueber `tools/parity/bescheid_oracle` mit leerer Wegwerf-Datenwurzel) auf
//! denselben Events. 20 der 25 Faelle stimmen ausserdem mit einer Rechnung aus den Quellen im Baum ueberein (Spenden 20 %
//! des Gesamtbetrags der Einkuenfte `estg_p10b_2026-07-13.txt`; Kirchensteuer gezahlt minus erstattet, KV/PV-Hoechstbetraege
//! 1.900 und 2.800 EUR und Kinderbetreuung 80 % bis 4.800 EUR je Kind `estg_p10_2026-07-11.txt`; Pauschbetraege 1.140 EUR
//! bei `GdB` 50 und 7.400 EUR bei Merkzeichen H `estg_p33b_2026-07-13.txt`). Fuenf Faelle tragen nur das Python-Orakel (der
//! Abzug nach der zumutbaren Belastung, § 33 Abs. 3, und das Wahlrecht Pauschbetrag mit behinderungsbedingten Kosten): sie
//! sind im Code mit einem Kommentar markiert. Kein Wert ist aus dem Rust-Code abgelesen.
//!
//! Zahlen in den Events sind Cent (die Felder `spenden_betrag`, `kinderbetreuungskosten`, `basis_kv` usw. sind Cent-Felder;
//! `rentner_grad_der_behinderung` ist der Grad); `gde`, `ausserg` und `erwartet` sind EUR.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::abzuege::shared_steuer_sonder_agb;
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::Instanzquelle;
use domain::{Euro, Veranlagung, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Ein bestaetigtes Ja/Nein-Feld.
fn b(fid: &'static str, wert: bool) -> Ev {
    (fid, json!(wert), true)
}

/// Ein Zahlfeld (Cent, bzw. Grad der Behinderung).
fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein handgebauter Fall; `erwartet` in EUR ist die Ausgabe des Python-Orakels.
struct Fall {
    gruppe: &'static str,
    name: &'static str,
    events: Vec<Ev>,
    /// Gesamtbetrag der Einkuenfte, EUR.
    gde: i64,
    /// Aussergewoehnliche Belastungen vor dem eigenen Pauschbetrag, EUR.
    ausserg: i64,
    zusammen: bool,
    nur_bestaetigt: bool,
    /// `[Steuerermaessigungen, Sonderausgaben, aussergewoehnliche Belastungen]`, EUR.
    erwartet: [i64; 3],
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn faelle() -> Vec<Fall> {
    vec![
        Fall {
            gruppe: "sonderausgaben",
            name: "Spenden 1.000 EUR bei Gesamtbetrag der Einkuenfte 50.000 EUR",
            events: vec![z("spenden_betrag", 100_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 1000, 0],
        },
        Fall {
            gruppe: "sonderausgaben",
            name: "Spenden 15.000 EUR: Grenze 20 % von 50.000 = 10.000 EUR",
            events: vec![z("spenden_betrag", 1_500_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 10_000, 0],
        },
        Fall {
            gruppe: "sonderausgaben",
            name: "Kirchensteuer gezahlt 800, erstattet 200 EUR: 600",
            events: vec![z("kist_gezahlt", 80_000, true), z("kist_erstattet", 20_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 600, 0],
        },
        Fall {
            gruppe: "sonderausgaben",
            name: "Kirchensteuer erstattet 800 > gezahlt 200 EUR: nie negativ, 0",
            events: vec![z("kist_gezahlt", 20_000, true), z("kist_erstattet", 80_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 0],
        },
        Fall {
            gruppe: "kv_pv",
            name: "Partner-KV 3.000 EUR bei Zusammenveranlagung: zaehlt",
            events: vec![z("basis_kv_partner", 300_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: true,
            nur_bestaetigt: true,
            erwartet: [0, 3000, 0],
        },
        Fall {
            gruppe: "kv_pv",
            name: "Partner-KV 3.000 EUR bei Einzelveranlagung: zaehlt nicht",
            events: vec![z("basis_kv_partner", 300_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 0],
        },
        Fall {
            gruppe: "kv_pv",
            name: "KV Person A 2.000 + Partner 3.000 EUR (zusammen): 5.000",
            events: vec![z("basis_kv", 200_000, true), z("basis_kv_partner", 300_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: true,
            nur_bestaetigt: true,
            erwartet: [0, 5000, 0],
        },
        Fall {
            gruppe: "kv_pv",
            name: "Basis 1.000 + Haftpflicht 3.000 EUR, Anspruch auf Zuschuss: Hoechstbetrag 1.900",
            events: vec![z("basis_kv", 100_000, true), z("vorsorge_unfall_haftpflicht", 300_000, true), b("mit_anspruch_auf_zuschuss", true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 1900, 0],
        },
        Fall {
            gruppe: "kv_pv",
            name: "dasselbe, ausdruecklich ohne Anspruch auf Zuschuss: Hoechstbetrag 2.800",
            events: vec![z("basis_kv", 100_000, true), z("vorsorge_unfall_haftpflicht", 300_000, true), b("mit_anspruch_auf_zuschuss", false)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 2800, 0],
        },
        Fall {
            gruppe: "kv_pv",
            name: "Partner: Basis 1.000 + Haftpflicht 3.000 EUR, Partner mit Anspruch auf Zuschuss: 1.900",
            events: vec![z("basis_kv_partner", 100_000, true), z("vorsorge_unfall_haftpflicht_partner", 300_000, true), b("mit_anspruch_auf_zuschuss_partner", true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: true,
            nur_bestaetigt: true,
            erwartet: [0, 1900, 0],
        },
        Fall {
            gruppe: "agb",
            name: "Wahlrecht Einzelnachweis (nein), GdB 50: eigener Pauschbetrag 1.140 EUR geht von der agB-Basis 3.000 EUR ab",
            events: vec![z("rentner_grad_der_behinderung", 50, true), b("behinderungsbedingte_aufwendungen_wahlrecht_pb", false)],
            gde: 50_000,
            ausserg: 3000,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 1860],
        },
        // Nur das Python-Orakel stuetzt diesen Fall (zumutbare Belastung, § 33 Abs. 3): keine eigene Rechnung aus den Quellen.
        Fall {
            gruppe: "agb",
            name: "Wahlrecht Pauschbetrag (ja), GdB 50: Basis 3.000 EUR bleibt, agB-Aufwand wird um behinderungsbedingte Kosten gekuerzt",
            events: vec![z("rentner_grad_der_behinderung", 50, true), b("behinderungsbedingte_aufwendungen_wahlrecht_pb", true), z("agb_aufwendungen", 500_000, true), z("behinderungsbedingte_aufwendungen", 100_000, true)],
            gde: 20_000,
            ausserg: 3000,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 5953],
        },
        Fall {
            gruppe: "agb",
            name: "Partner GdB 50 mit Wahlrecht Einzelnachweis bei Einzelveranlagung: Partner zaehlt nicht",
            events: vec![z("rentner_grad_der_behinderung_partner", 50, true), b("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", false)],
            gde: 50_000,
            ausserg: 3000,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 3000],
        },
        Fall {
            gruppe: "agb",
            name: "dasselbe bei Zusammenveranlagung: Partner-Pauschbetrag 1.140 EUR geht ab",
            events: vec![z("rentner_grad_der_behinderung_partner", 50, true), b("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", false)],
            gde: 50_000,
            ausserg: 3000,
            zusammen: true,
            nur_bestaetigt: true,
            erwartet: [0, 0, 1860],
        },
        Fall {
            gruppe: "agb",
            name: "Merkzeichen H (hilflos) ohne GdB, Wahlrecht Einzelnachweis: Pauschbetrag 7.400 EUR geht von 10.000 EUR ab",
            events: vec![b("rentner_hilflos_blind_taubblind", true), b("behinderungsbedingte_aufwendungen_wahlrecht_pb", false)],
            gde: 50_000,
            ausserg: 10_000,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 2600],
        },
        // Nur das Python-Orakel stuetzt diesen Fall (zumutbare Belastung, § 33 Abs. 3): keine eigene Rechnung aus den Quellen.
        Fall {
            gruppe: "agb",
            name: "agB 5.000 EUR, 'notwendig und angemessen' ausdruecklich nein: kein Abzug",
            events: vec![z("agb_aufwendungen", 500_000, true), b("agb_notwendig_angemessen", false)],
            gde: 20_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 0],
        },
        // Nur das Python-Orakel stuetzt diesen Fall (zumutbare Belastung, § 33 Abs. 3): keine eigene Rechnung aus den Quellen.
        Fall {
            gruppe: "agb",
            name: "agB 5.000 EUR, 'zwangslaeufig' ausdruecklich nein: kein Abzug",
            events: vec![z("agb_aufwendungen", 500_000, true), b("agb_zwangslaeufig", false)],
            gde: 20_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 0],
        },
        // Nur das Python-Orakel stuetzt diesen Fall (zumutbare Belastung, § 33 Abs. 3): keine eigene Rechnung aus den Quellen.
        Fall {
            gruppe: "agb",
            name: "agB 5.000 EUR, beide Merkmale ausdruecklich ja: Abzug nach zumutbarer Belastung",
            events: vec![z("agb_aufwendungen", 500_000, true), b("agb_notwendig_angemessen", true), b("agb_zwangslaeufig", true)],
            gde: 20_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 3953],
        },
        // Nur das Python-Orakel stuetzt diesen Fall (zumutbare Belastung, § 33 Abs. 3): keine eigene Rechnung aus den Quellen.
        Fall {
            gruppe: "agb",
            name: "agB 5.000 EUR, Merkmale unbeantwortet: Abzug wie bei ja",
            events: vec![z("agb_aufwendungen", 500_000, true)],
            gde: 20_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 3953],
        },
        Fall {
            gruppe: "kinderbetreuung",
            name: "ein Kind unter 14 (bestaetigt), 3.000 EUR: 80 % = 2.400",
            events: vec![b("kind_unter_14_haushaltszugehoerig", true), z("kinderbetreuungskosten", 300_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 2400, 0],
        },
        Fall {
            gruppe: "kinderbetreuung",
            name: "zwei Kinder, je 3.000 EUR: 4.800",
            events: vec![b("kind_unter_14_haushaltszugehoerig", true), z("kinderbetreuungskosten", 300_000, true), b("kind_unter_14_haushaltszugehoerig__2", true), z("kinderbetreuungskosten__2", 300_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 4800, 0],
        },
        Fall {
            gruppe: "kinderbetreuung",
            name: "ein Kind, 8.000 EUR: Hoechstbetrag 4.800",
            events: vec![b("kind_unter_14_haushaltszugehoerig", true), z("kinderbetreuungskosten", 800_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 4800, 0],
        },
        Fall {
            gruppe: "kinderbetreuung",
            name: "Kind unter 14 ausdruecklich nein: 0",
            events: vec![b("kind_unter_14_haushaltszugehoerig", false), z("kinderbetreuungskosten", 300_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 0],
        },
        Fall {
            gruppe: "kinderbetreuung",
            name: "Kind ohne Antwort zu 'unter 14': 0",
            events: vec![z("kinderbetreuungskosten", 300_000, true)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 0, 0],
        },
        Fall {
            gruppe: "kinderbetreuung",
            name: "zweites Kind nur vorlaeufig bei nur_bestaetigt: zaehlt nicht",
            events: vec![b("kind_unter_14_haushaltszugehoerig", true), z("kinderbetreuungskosten", 300_000, true), ("kind_unter_14_haushaltszugehoerig__2", json!(true), false), z("kinderbetreuungskosten__2", 300_000, false)],
            gde: 50_000,
            ausserg: 0,
            zusammen: false,
            nur_bestaetigt: true,
            erwartet: [0, 2400, 0],
        },
    ]
}

fn rechne(fall: &Fall) -> [i64; 3] {
    let st = store(&fall.events);
    // Bei `nur_bestaetigt` ist `f` schon auf bestaetigte Felder gefiltert (`bescheid_zweige.py:1487`,
    // `bescheid_oracle._ctx`); der Store selbst behaelt alle Events und liefert die Instanzen.
    let f = if fall.nur_bestaetigt {
        let bestaetigt: Vec<Ev> = fall.events.iter().filter(|e| e.2).cloned().collect();
        felder(&store(&bestaetigt))
    } else {
        felder(&st)
    };
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(index()),
        nur_bestaetigt: fall.nur_bestaetigt,
    };
    let veranlagung = if fall.zusammen {
        Veranlagung::Zusammen
    } else {
        Veranlagung::Einzel
    };
    let s = shared_steuer_sonder_agb(
        Euro::new(fall.gde),
        Euro::new(fall.ausserg),
        veranlagung,
        &f,
        Vz::Vz2025,
        &q,
        params(),
    )
    .unwrap();
    [
        s.steuerermaessigungen.get(),
        s.sonderausgaben.get(),
        s.aussergewoehnliche_belastungen.get(),
    ]
}

/// Alle Faelle der Gruppe durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe(gruppe: &str) {
    let faelle: Vec<Fall> = faelle()
        .into_iter()
        .filter(|f| f.gruppe == gruppe)
        .collect();
    assert!(!faelle.is_empty(), "Gruppe {gruppe} ist leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let got = rechne(f);
            (got != f.erwartet)
                .then(|| format!("{}: Rust {got:?}, Orakel {:?}", f.name, f.erwartet))
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// Sonderausgaben: Spenden bis 20 % des Gesamtbetrags der Einkuenfte (§ 10b) und Kirchensteuer gezahlt minus
/// erstattet, nie negativ (§ 10 Abs. 1 Nr. 4).
#[test]
fn spenden_und_kirchensteuer() {
    pruefe("sonderausgaben");
}

/// Kranken- und Pflegeversicherung (§ 10 Abs. 1 Nr. 3 und Abs. 4): Person A und Partner (Partner nur bei
/// Zusammenveranlagung, eigenes Feld-Suffix `_partner`), Hoechstbetrag 1.900 EUR mit, 2.800 EUR ohne Anspruch
/// auf Zuschuss; ein ausdruecklich bestaetigtes Nein zum Zuschuss gilt als Nein.
#[test]
fn kv_pv_person_partner_und_zuschuss() {
    pruefe("kv_pv");
}

/// Aussergewoehnliche Belastungen (§ 33, § 33b): der eigene Pauschbetrag geht von der Basis ab (`GdB`, Merkzeichen H),
/// der Partner nur bei Zusammenveranlagung (eigenes Feld-Suffix); das Wahlrecht Pauschbetrag/Einzelnachweis; die
/// Merkmale `notwendig und angemessen` und `zwangslaeufig` sperren den Abzug nur bei ausdruecklichem Nein.
#[test]
fn aussergewoehnliche_belastungen_und_eigener_pauschbetrag() {
    pruefe("agb");
}

/// Kinderbetreuung (§ 10 Abs. 1 Nr. 5): 80 % der Kosten je Kind unter 14, Hoechstbetrag 4.800 EUR je Kind,
/// Summe ueber alle Kinder-Instanzen, nur bei bestaetigtem `kind_unter_14_haushaltszugehoerig`, Cent zu ganzen Euro.
#[test]
fn kinderbetreuung_summe_ueber_kinder() {
    pruefe("kinderbetreuung");
}
