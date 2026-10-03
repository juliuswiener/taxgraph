//! Kind-Abzuege in `abzuege.rs` mit vorlaeufigen Instanzen (`nur_bestaetigt = false`) und ohne Store im Standardlauf (ohne
//! `PARITY=1`, ohne Python): `kind_kv_pv_summe`, `kinderbetreuung_summe`, `p10_1_5_gate_fehlend`, `schulgeld_summe`,
//! `kind_behinderten_pb_daten`, `p33b_kind_pauschbetraege` und, im Ganzen, `shared_steuer_sonder_agb`.
//!
//! Die Instanzquelle hat zwei Schalter, die die bisherigen Tests kaum beruehren: `nur_bestaetigt` (bei `true` bewegt eine
//! vorlaeufige Instanz die festgesetzte Steuer nie; bei `false`, dem Schaetzpfad, zaehlt sie) und der Store selbst (Alt-Aufrufer
//! ohne Store rechnen 0 bzw. leer). Ihre Gegenprobe gegen Python lief bisher nur in
//! `rust/parity/tests/bescheid_blatt_paritaet.rs` und damit nur mit `PARITY=1`; die CI faehrt Parity nicht. Gemessen am
//! 2026-10-03 auf 4a2f6ea4 (Bericht h8-hermetisch4): 14 Mutationen am Aufrufort in `abzuege.rs` (N1-N14). Zehn lassen
//! `cargo test -p bescheid` gruen (188 passed, 0 failed, 10 ignored): N2 Kinderbetreuung und N3 Schulgeld und N4 Kind-
//! Pauschbetrag zaehlen vorlaeufige Kinder nicht, N5 ein Kind mit `IdNr` unter 11 Zeichen uebertraegt den Pauschbetrag trotzdem,
//! N6 das Gate `kind_unter_14_haushaltszugehoerig` wird auch bei Kosten 0 gemeldet, N8 Schulgeld ohne Zusammenveranlagung, N9 der
//! Hinterbliebenen-Pauschbetrag entfaellt, N10 das Merkzeichen H/Bl/TBl des Kindes wird nicht erkannt, N13 ein Kind mit
//! Pauschbetrag kuerzt die Einzelnachweise der agB nicht, N14 die Kind-Beitraege fliessen nicht in die Sonderausgaben. N10 wird
//! auf dem Bestand nur zufaellig rot (`lesestellen_des_rechenkerns_sind_in_der_bindung`: die Mutation loescht die einzige
//! Lesestelle des Feldes `kind_hilflos_blind_taubblind`, 141 < Mindestzahl 142 Literale; kein Verhaltenstest). Drei fangen schon Bestandstests (N1
//! `kind_kv_pv_verlangt_idnr_und_bestaetigung`, N7 und N12 `betraege_instanzen_und_flat_rueckfall`). Eine (N11: `hh_summe` prueft
//! `store.is_none()` statt `beide().is_none()`) ist am oeffentlichen Weg nicht beobachtbar: `shared_steuer_sonder_agb` ruft
//! danach `kind_kv_pv_summe`, und die meldet bei Store ohne Bindung ohnehin `BindungFehlt` (Test
//! `store_ohne_bindung_ist_ein_fehler`); sie zaehlt nicht als gefangen. Mit diesen Tests werden die uebrigen 13 rot, jede mit dem
//! Test, der ihre Stelle prueft (Tabelle im Bericht).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: jede Zahl der Tabellen unten ist die Ausgabe des Python-Orakels
//! `bescheid_abzuege._kind_kv_pv_summe`, `_kinderbetreuung_summe`, `_p10_1_5_gate_fehlend`, `_schulgeld_summe`,
//! `_kind_behinderten_pb_daten`, `_p33b_kind_pauschbetraege` und `_shared_steuer_sonder_agb` (ueber `tools/parity/bescheid_oracle`,
//! VZ 2025, `store_uebergeben` ja/nein, Wegwerf-Datenwurzel) auf denselben Events; Orakel-Skript und Lauf: Anlagen zum Bericht.
//! Kein Wert ist aus dem Rust-Code abgelesen. 40 der 54 Faelle stimmen ausserdem mit einer eigenen Rechnung aus den Quellen im
//! Baum ueberein: Kinderbetreuung 80 % der Kosten bis 4.800 EUR je Kind (§ 10 Abs. 1 Nr. 5 S. 1, `estg_p10_2026-07-11.txt`),
//! Schulgeld 30 % des Entgelts bis 5.000 EUR (§ 10 Abs. 1 Nr. 9 S. 1, ebenda), Kind-KV/PV als Cent-Summe unter dem Hoechstbetrag
//! 2.800 EUR (§ 10 Abs. 1 Nr. 3 S. 2, Abs. 4), Pauschbetraege 1.140 EUR (`GdB` 50), 2.120 EUR (`GdB` 80), 2.840 EUR (`GdB` 100),
//! 7.400 EUR (Merkzeichen H/Bl/TBl) und 370 EUR (Hinterbliebene) (§ 33b Abs. 3 und 4 `EStG`, `estg_p33b_2026-07-13.txt`). Nur das
//! Python-Orakel stuetzt: die acht Faelle zu `p10_1_5_gate_fehlend` (DASS eine unbeantwortete Frage gemeldet wird, ist Entwurf
//! des Projekts), die fuenf agB-Faelle (Abzug nach der zumutbaren Belastung, § 33 Abs. 3 `EStG`, und die Kuerzung nach § 33b
//! Abs. 5 S. 4) und der Fall "Einzel, ein Kind mit 10.000 EUR Schulgeld": das Orakel liefert 2.500 EUR (30 % = 3.000 EUR,
//! gedeckelt). Das ist die Auslegung des Projekts (`params/2025/schulgeld_p10.yaml`: halber Hoechstbetrag je Elternteil bei
//! Einzelveranlagung, Satz 5 "je Elternpaar nur einmal"); der Wortlaut von Satz 1 nennt 5.000 EUR. Der Test haelt die Auslegung
//! fest, er entscheidet sie nicht.
//!
//! Zahlen in den Events sind Cent (`kind_kv`, `kind_pv`, `kinderbetreuungskosten`, `schulgeld`, `agb_aufwendungen`, ...);
//! `kind_grad_der_behinderung` ist der Grad; `erwartet` ist Cent bei `kind_kv_pv_summe`, sonst EUR.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::fmt::Debug;

use bescheid::abzuege::{
    kind_behinderten_pb_daten, kind_kv_pv_summe, kinderbetreuung_summe, p10_1_5_gate_fehlend,
    p33b_kind_pauschbetraege, schulgeld_summe, shared_steuer_sonder_agb,
};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::{BescheidFehler, Felder, Instanzquelle};
use domain::{Euro, Veranlagung, Vz};
use serde_json::{json, Value};
use store::Store;

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Kind-Pauschbetrag-Daten je Kind als `(Grad, Merkzeichen, Hinterbliebene)` und die Summe in EUR.
type PbErgebnis = (Vec<(i64, bool, bool)>, i64);

/// Gesamtbetrag der Einkuenfte (EUR) der `shared_steuer_sonder_agb`-Faelle.
const GDE: i64 = 50_000;

/// Ein Ja/Nein-Feld; `bestaetigt = false` ist ein vorlaeufiger Wert.
fn b(fid: &'static str, wert: bool, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Zahlfeld (Cent, Grad der Behinderung); `bestaetigt = false` ist ein vorlaeufiger Wert.
fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Textfeld.
fn t(fid: &'static str, wert: &'static str, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein handgebauter Fall; `erwartet` ist die Ausgabe des Python-Orakels auf denselben Events.
struct Fall<E> {
    name: &'static str,
    events: Vec<Ev>,
    nur_bestaetigt: bool,
    /// `false`: Alt-Aufrufer ohne Store (`store_uebergeben = false` im Orakel).
    mit_store: bool,
    erwartet: E,
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn kv_pv_faelle() -> Vec<Fall<i64>> {
    vec![
        Fall {
            name: "Kind 1 bestaetigt: 1.000 + 500 EUR",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 100_000, true),
                z("kind_pv", 50_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 150_000,
        },
        Fall {
            name: "Kind 1 bestaetigt, Kind 2 vorlaeufig, nur_bestaetigt = false: beide zaehlen",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 100_000, true),
                z("kind_pv", 50_000, true),
                t("kind_idnr__2", "12345678901", true),
                z("kind_kv__2", 70_000, false),
                z("kind_pv__2", 30_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 250_000,
        },
        Fall {
            name: "dieselben Events, nur_bestaetigt = true: das vorlaeufige Kind zaehlt nicht",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 100_000, true),
                z("kind_pv", 50_000, true),
                t("kind_idnr__2", "12345678901", true),
                z("kind_kv__2", 70_000, false),
                z("kind_pv__2", 30_000, true),
            ],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: 150_000,
        },
        Fall {
            name: "Kind 2 vorlaeufig mit IdNr aus 10 Zeichen: zaehlt nicht",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 100_000, true),
                z("kind_pv", 50_000, true),
                t("kind_idnr__2", "1234567890", true),
                z("kind_kv__2", 70_000, false),
                z("kind_pv__2", 30_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 150_000,
        },
        Fall {
            name: "Kind 2 vorlaeufig ohne IdNr: zaehlt nicht",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 100_000, true),
                z("kind_pv", 50_000, true),
                z("kind_kv__2", 70_000, false),
                z("kind_pv__2", 30_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 150_000,
        },
        Fall {
            name: "nur Kind 1, beide Felder vorlaeufig, nur_bestaetigt = false",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 100_000, false),
                z("kind_pv", 50_000, false),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 150_000,
        },
        Fall {
            name: "Kind 1 mit KV negativ und PV 0: nur positive Betraege",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", -500, true),
                z("kind_pv", 0, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 0,
        },
        Fall {
            name: "ohne Store (Alt-Aufrufer): 0",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 100_000, true),
                z("kind_pv", 50_000, true),
                t("kind_idnr__2", "12345678901", true),
                z("kind_kv__2", 70_000, false),
                z("kind_pv__2", 30_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: false,
            erwartet: 0,
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn betreuung_faelle() -> Vec<Fall<i64>> {
    vec![
        Fall {
            name: "Kind 1 bestaetigt, 3.000 EUR: 80 % = 2.400",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 2400,
        },
        Fall {
            name:
                "Kind 2 vorlaeufig, 5.000 EUR: 80 % = 4.000; nur_bestaetigt = false: beide zaehlen",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_unter_14_haushaltszugehoerig__2", true, false),
                z("kinderbetreuungskosten__2", 500_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 6400,
        },
        Fall {
            name: "dieselben Events, nur_bestaetigt = true",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_unter_14_haushaltszugehoerig__2", true, false),
                z("kinderbetreuungskosten__2", 500_000, true),
            ],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: 2400,
        },
        Fall {
            name: "Kind 2 vorlaeufig, 7.000 EUR: Hoechstbetrag 4.800",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_unter_14_haushaltszugehoerig__2", true, false),
                z("kinderbetreuungskosten__2", 700_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 7200,
        },
        Fall {
            name: "Kind 2 vorlaeufig, aber ueber 14 (nein): zaehlt nicht",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_unter_14_haushaltszugehoerig__2", false, false),
                z("kinderbetreuungskosten__2", 500_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 2400,
        },
        Fall {
            name: "Kind 2 vorlaeufig ohne Antwort zu 'unter 14': zaehlt nicht",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                z("kinderbetreuungskosten__2", 500_000, false),
            ],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 2400,
        },
        Fall {
            name: "ohne Store (Alt-Aufrufer): 0",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_unter_14_haushaltszugehoerig__2", true, false),
                z("kinderbetreuungskosten__2", 500_000, true),
            ],
            nur_bestaetigt: false,
            mit_store: false,
            erwartet: 0,
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn gate_faelle() -> Vec<Fall<Vec<&'static str>>> {
    vec![
        Fall {
            name: "Kosten 3.000 EUR, Gate nicht im Snapshot",
            events: vec![z("kinderbetreuungskosten", 300_000, true)],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: vec!["kind_unter_14_haushaltszugehoerig"],
        },
        Fall {
            name: "Kosten 0, Gate nicht im Snapshot: keine Frage offen",
            events: vec![z("kinderbetreuungskosten", 0, true)],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: vec![],
        },
        Fall {
            name: "Kosten negativ, Gate nicht im Snapshot: keine Frage offen",
            events: vec![z("kinderbetreuungskosten", -100, true)],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: vec![],
        },
        Fall {
            name: "Kosten 3.000 EUR, Gate nur vorlaeufig im Snapshot: gilt als beantwortet",
            events: vec![
                z("kinderbetreuungskosten", 300_000, true),
                b("kind_unter_14_haushaltszugehoerig", true, false),
            ],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: vec![],
        },
        Fall {
            name: "Kind 1 mit Gate und Kosten, Kind 2 vorlaeufig mit Kosten ohne Gate",
            events: vec![
                b("kind_unter_14_haushaltszugehoerig", true, true),
                z("kinderbetreuungskosten", 300_000, true),
                z("kinderbetreuungskosten__2", 500_000, false),
            ],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: vec!["kind_unter_14_haushaltszugehoerig"],
        },
        Fall {
            name: "Kind 1 Kosten 0 ohne Gate, Kind 2 vorlaeufig Kosten 0 ohne Gate",
            events: vec![
                z("kinderbetreuungskosten", 0, true),
                z("kinderbetreuungskosten__2", 0, false),
            ],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: vec![],
        },
        Fall {
            name: "nur_bestaetigt = false, Kosten 3.000 EUR ohne Gate: dieselbe Antwort",
            events: vec![z("kinderbetreuungskosten", 300_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: vec!["kind_unter_14_haushaltszugehoerig"],
        },
        Fall {
            name: "ohne Store (Alt-Aufrufer): leer",
            events: vec![z("kinderbetreuungskosten", 300_000, true)],
            nur_bestaetigt: true,
            mit_store: false,
            erwartet: vec![],
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn schulgeld_faelle() -> Vec<Fall<i64>> {
    vec![
        Fall {
            name: "Einzel, Kind 1 bestaetigt 4.000 EUR: 30 % = 1.200",
            events: vec![t("veranlagung", "einzel", true), z("schulgeld", 400_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 1200,
        },
        Fall {
            name: "Einzel, Kind 2 vorlaeufig 8.000 EUR: 30 % = 2.400; nur_bestaetigt = false: beide zaehlen",
            events: vec![t("veranlagung", "einzel", true), z("schulgeld", 400_000, true), z("schulgeld__2", 800_000, false)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 3600,
        },
        Fall {
            name: "dieselben Events, nur_bestaetigt = true",
            events: vec![t("veranlagung", "einzel", true), z("schulgeld", 400_000, true), z("schulgeld__2", 800_000, false)],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: 1200,
        },
        Fall {
            name: "Einzel, ein Kind mit 10.000 EUR: Deckel des Produkts (Auslegung, nur Orakel)",
            events: vec![t("veranlagung", "einzel", true), z("schulgeld", 1_000_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 2500,
        },
        Fall {
            name: "zusammen, ein Kind mit 10.000 EUR: 30 % = 3.000 (Deckel verdoppelt)",
            events: vec![t("veranlagung", "zusammen", true), z("schulgeld", 1_000_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 3000,
        },
        Fall {
            name: "zusammen, ein Kind mit 20.000 EUR: 30 % = 6.000, gedeckelt auf 5.000 (Hoechstbetrag Satz 1)",
            events: vec![t("veranlagung", "zusammen", true), z("schulgeld", 2_000_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 5000,
        },
        Fall {
            name: "zusammen, Kind 1 und vorlaeufiges Kind 2 mit je 10.000 EUR",
            events: vec![t("veranlagung", "zusammen", true), z("schulgeld", 1_000_000, true), z("schulgeld__2", 1_000_000, false)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 6000,
        },
        Fall {
            name: "Schulgeld 0 und negativ: kein Abzug",
            events: vec![t("veranlagung", "einzel", true), z("schulgeld", 0, true), z("schulgeld__2", -100, false)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: 0,
        },
        Fall {
            name: "ohne Store (Alt-Aufrufer): 0",
            events: vec![t("veranlagung", "einzel", true), z("schulgeld", 400_000, true), z("schulgeld__2", 800_000, false)],
            nur_bestaetigt: false,
            mit_store: false,
            erwartet: 0,
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn pb_faelle() -> Vec<Fall<PbErgebnis>> {
    vec![
        Fall {
            name: "Kind 1 bestaetigt, GdB 50: 1.140 EUR",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(50, false, false)], 1140),
        },
        Fall {
            name: "Kind 1 GdB 50, Kind 2 vorlaeufig GdB 80, nur_bestaetigt = false: beide zaehlen (1.140 + 2.120)",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), t("kind_idnr__2", "12345678901", true), b("kind_behinderten_pb_antrag__2", true, true), b("kind_pb_nicht_selbst_genutzt__2", true, true), z("kind_grad_der_behinderung__2", 80, true), b("kind_hilflos_blind_taubblind__2", false, false)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(50, false, false), (80, false, false)], 3260),
        },
        Fall {
            name: "dieselben Events, nur_bestaetigt = true",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), t("kind_idnr__2", "12345678901", true), b("kind_behinderten_pb_antrag__2", true, true), b("kind_pb_nicht_selbst_genutzt__2", true, true), z("kind_grad_der_behinderung__2", 80, true), b("kind_hilflos_blind_taubblind__2", false, false)],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: (vec![(50, false, false)], 1140),
        },
        Fall {
            name: "Kind ohne IdNr-Laenge (10 Zeichen): keine Uebertragung",
            events: vec![t("kind_idnr", "1234567890", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![], 0),
        },
        Fall {
            name: "Kind mit 10-Zeichen-IdNr neben einem gueltigen Kind",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), t("kind_idnr__2", "1234567890", true), b("kind_behinderten_pb_antrag__2", true, true), b("kind_pb_nicht_selbst_genutzt__2", true, true), z("kind_grad_der_behinderung__2", 80, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(50, false, false)], 1140),
        },
        Fall {
            name: "Kind ohne Antrag: keine Uebertragung",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![], 0),
        },
        Fall {
            name: "Kind mit Antrag, aber Pauschbetrag selbst genutzt: keine Uebertragung",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", false, true), z("kind_grad_der_behinderung", 50, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![], 0),
        },
        Fall {
            name: "Kind mit Merkzeichen H (ohne GdB): 7.400 EUR",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), b("kind_hilflos_blind_taubblind", true, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(0, true, false)], 7400),
        },
        Fall {
            name: "Kind mit GdB 50 und Merkzeichen H: nur 7.400 EUR, nicht zusaetzlich 1.140",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), b("kind_hilflos_blind_taubblind", true, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(50, true, false)], 7400),
        },
        Fall {
            name: "Kind mit GdB 50 und Hinterbliebenenbezuegen: 1.140 + 370",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), b("kind_hinterbliebenen_uebertragung", true, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(50, false, true)], 1510),
        },
        Fall {
            name: "Kind mit GdB 19: unter der Schwelle, 0 EUR",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 19, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(19, false, false)], 0),
        },
        Fall {
            name: "Kind mit Hinterbliebenenbezuegen ohne GdB: 370 EUR",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), b("kind_hinterbliebenen_uebertragung", true, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(0, false, true)], 370),
        },
        Fall {
            name: "Kind 2 vorlaeufig mit Hinterbliebenenbezuegen, GdB 100: 2.840 + 370",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), t("kind_idnr__2", "12345678901", true), b("kind_behinderten_pb_antrag__2", true, true), b("kind_pb_nicht_selbst_genutzt__2", true, true), z("kind_grad_der_behinderung__2", 100, true), b("kind_hinterbliebenen_uebertragung__2", true, false)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: (vec![(50, false, false), (100, false, true)], 4350),
        },
        Fall {
            name: "ohne Store (Alt-Aufrufer): leer, 0 EUR",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), t("kind_idnr__2", "12345678901", true), b("kind_behinderten_pb_antrag__2", true, true), b("kind_pb_nicht_selbst_genutzt__2", true, true), z("kind_grad_der_behinderung__2", 80, true), b("kind_hilflos_blind_taubblind__2", false, false)],
            nur_bestaetigt: false,
            mit_store: false,
            erwartet: (vec![], 0),
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn sa_faelle() -> Vec<Fall<[i64; 3]>> {
    vec![
        Fall {
            name: "Kind-KV/PV 1.500 EUR fliessen in die Sonderausgaben (§ 10 Abs. 1 Nr. 3 S. 2)",
            events: vec![t("kind_idnr", "12345678901", true), z("kind_kv", 100_000, true), z("kind_pv", 50_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: [0, 1500, 0],
        },
        Fall {
            name: "Kind-KV/PV 1.500 EUR plus Kind 2 vorlaeufig 1.000 EUR, nur_bestaetigt = false",
            events: vec![t("kind_idnr", "12345678901", true), z("kind_kv", 100_000, true), z("kind_pv", 50_000, true), t("kind_idnr__2", "12345678901", true), z("kind_kv__2", 70_000, false), z("kind_pv__2", 30_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: [0, 2500, 0],
        },
        Fall {
            name: "Kind-KV/PV ohne Store: nicht in den Sonderausgaben",
            events: vec![t("kind_idnr", "12345678901", true), z("kind_kv", 100_000, true), z("kind_pv", 50_000, true)],
            nur_bestaetigt: false,
            mit_store: false,
            erwartet: [0, 0, 0],
        },
        Fall {
            name: "agB 20.000 EUR, ohne Kind-Pauschbetrag: voller Betrag, Abzug nach zumutbarer Belastung",
            events: vec![z("agb_aufwendungen", 2_000_000, true), z("behinderungsbedingte_aufwendungen", 500_000, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: [0, 0, 17_153],
        },
        Fall {
            name: "agB 20.000 EUR, Kind mit Pauschbetrag: § 33b Abs. 5 S. 4 kuerzt um die behinderungsbedingten 5.000 EUR",
            events: vec![z("agb_aufwendungen", 2_000_000, true), z("behinderungsbedingte_aufwendungen", 500_000, true), t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: [0, 0, 12_153],
        },
        Fall {
            name: "dieselben Events, Kind-Pauschbetrag nur vorlaeufig, nur_bestaetigt = false: kuerzt ebenfalls",
            events: vec![z("agb_aufwendungen", 2_000_000, true), z("behinderungsbedingte_aufwendungen", 500_000, true), t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), b("kind_hilflos_blind_taubblind", false, false)],
            nur_bestaetigt: false,
            mit_store: true,
            erwartet: [0, 0, 12_153],
        },
        Fall {
            name: "Kind-Pauschbetrag nur vorlaeufig, nur_bestaetigt = true: kuerzt nicht",
            events: vec![z("agb_aufwendungen", 2_000_000, true), z("behinderungsbedingte_aufwendungen", 500_000, true), t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), b("kind_hilflos_blind_taubblind", false, false)],
            nur_bestaetigt: true,
            mit_store: true,
            erwartet: [0, 0, 17_153],
        },
        Fall {
            name: "Kind-Pauschbetrag ohne Store: keine Kuerzung",
            events: vec![z("agb_aufwendungen", 2_000_000, true), z("behinderungsbedingte_aufwendungen", 500_000, true), t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true)],
            nur_bestaetigt: false,
            mit_store: false,
            erwartet: [0, 0, 17_153],
        },
    ]
}

/// Die Instanzquelle eines Falls; die Bindung ist immer da, der Store nur bei `mit_store`.
fn quelle<'a, E>(st: &'a Store, fall: &Fall<E>) -> Instanzquelle<'a> {
    Instanzquelle {
        store: fall.mit_store.then_some(st),
        bindung: Some(index()),
        nur_bestaetigt: fall.nur_bestaetigt,
    }
}

/// Der Snapshot `f`: bei `nur_bestaetigt` schon auf bestaetigte Felder gefiltert (`bescheid_zweige.py:1487`,
/// `bescheid_oracle._ctx`); der Store selbst behaelt alle Events und liefert die Instanzen.
fn snapshot<E>(fall: &Fall<E>, st: &Store) -> Felder {
    if fall.nur_bestaetigt {
        let bestaetigt: Vec<Ev> = fall.events.iter().filter(|e| e.2).cloned().collect();
        felder(&store(&bestaetigt))
    } else {
        felder(st)
    }
}

/// Kind-Pauschbetrag-Daten je Kind und Summe der uebertragenen Pauschbetraege.
fn pb_ergebnis(q: &Instanzquelle<'_>) -> PbErgebnis {
    let daten = kind_behinderten_pb_daten(q)
        .unwrap()
        .into_iter()
        .map(|d| {
            (
                d.grad_der_behinderung,
                d.ist_hilflos_blind_taubblind,
                d.hat_hinterbliebenenbezuege,
            )
        })
        .collect();
    let summe = p33b_kind_pauschbetraege(q, Vz::Vz2025, params())
        .unwrap()
        .get();
    (daten, summe)
}

/// `[Steuerermaessigungen, Sonderausgaben, agB]` in EUR, Einzelveranlagung, `GDE`, keine agB-Basis.
fn sa_ergebnis<E>(fall: &Fall<E>, st: &Store) -> [i64; 3] {
    let s = shared_steuer_sonder_agb(
        Euro::new(GDE),
        Euro::new(0),
        Veranlagung::Einzel,
        &snapshot(fall, st),
        Vz::Vz2025,
        &quelle(st, fall),
        params(),
    )
    .unwrap();
    [
        s.steuerermaessigungen.get(),
        s.sonderausgaben.get(),
        s.aussergewoehnliche_belastungen.get(),
    ]
}

/// Alle Faelle durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe<E: PartialEq + Debug>(faelle: &[Fall<E>], rechne: impl Fn(&Fall<E>, &Store) -> E) {
    assert!(!faelle.is_empty(), "Tabelle ist leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let got = rechne(f, &store(&f.events));
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

/// § 10 Abs. 1 Nr. 3 S. 2: Kind-KV/PV in Cent, Kind nur mit `IdNr` aus mindestens 11 Zeichen. Bei `nur_bestaetigt = false` zaehlt auch ein vorlaeufiges Kind, bei `true` nicht; ohne Store ist die Summe 0.
#[test]
fn kind_kv_pv_vorlaeufige_kinder_zaehlen_nur_ohne_bestaetigungszwang() {
    pruefe(&kv_pv_faelle(), |fall, st| {
        kind_kv_pv_summe(&quelle(st, fall)).unwrap().get()
    });
}

/// § 10 Abs. 1 Nr. 5: 80 % der Kosten bis 4.800 EUR je Kind unter 14. Bei `nur_bestaetigt = false` zaehlt auch ein vorlaeufiges Kind; ohne Store ist die Summe 0.
#[test]
fn kinderbetreuung_vorlaeufige_kinder_zaehlen_nur_ohne_bestaetigungszwang() {
    pruefe(&betreuung_faelle(), |fall, st| {
        kinderbetreuung_summe(&quelle(st, fall), Vz::Vz2025, params())
            .unwrap()
            .get()
    });
}

/// Die offene Frage `kind_unter_14_haushaltszugehoerig` wird nur gemeldet, wenn ein Kind Kosten > 0 hat und das Gate GAR NICHT im Snapshot steht (auch ein vorlaeufiges Gate beantwortet es); `nur_bestaetigt` wird nicht gelesen; ohne Store ist die Antwort leer.
#[test]
fn gate_fehlend_nur_bei_positiven_kosten_ohne_gate_im_snapshot() {
    pruefe(&gate_faelle(), |fall, st| {
        p10_1_5_gate_fehlend(&quelle(st, fall))
            .unwrap()
            .into_iter()
            .collect()
    });
}

/// § 10 Abs. 1 Nr. 9: 30 % des Entgelts je Kind; der Deckel haengt an der Veranlagungsart (Einzel / zusammen). Bei `nur_bestaetigt = false` zaehlt auch ein vorlaeufiges Kind; ohne Store ist die Summe 0.
#[test]
fn schulgeld_vorlaeufige_kinder_zaehlen_und_zusammenveranlagung_gilt() {
    pruefe(&schulgeld_faelle(), |fall, st| {
        schulgeld_summe(&quelle(st, fall), Vz::Vz2025, &snapshot(fall, st), params())
            .unwrap()
            .get()
    });
}

/// § 33b Abs. 5: ein Kind uebertraegt seinen Pauschbetrag nur mit `IdNr` (11 Zeichen), Antrag und `nicht selbst genutzt`; `GdB`, Merkzeichen H/Bl/TBl (7.400 EUR statt Staffel) und Hinterbliebenenbezuege (370 EUR zusaetzlich) gehen je Kind in die Summe. Bei `nur_bestaetigt = false` zaehlt auch ein vorlaeufiges Kind; ohne Store ist alles leer.
#[test]
fn kind_pauschbetrag_vorlaeufige_kinder_idnr_merkzeichen_und_hinterbliebene() {
    pruefe(&pb_faelle(), |fall, st| pb_ergebnis(&quelle(st, fall)));
}

/// Im Gesamtlauf `shared_steuer_sonder_agb`: Kind-KV/PV erhoehen die Sonderausgaben (§ 10 Abs. 1 Nr. 3 S. 2); ein Kind mit uebertragenem Pauschbetrag kuerzt die behinderungsbedingten Einzelnachweise der agB (§ 33b Abs. 5 S. 4) -- nur wenn das Kind zaehlt (vorlaeufig: nur bei `nur_bestaetigt = false`) und ein Store da ist.
#[test]
fn kind_beitraege_und_kind_pauschbetrag_in_sonderausgaben_und_agb() {
    pruefe(&sa_faelle(), sa_ergebnis);
}

/// Ein Store ohne Bindung ist ein Fehler, kein stiller Wert: alle sieben Funktionen melden `BindungFehlt`. Das Python-Orakel
/// wirft in allen sieben Faellen einen `AttributeError` (`bindung_uebergeben = false`); es gibt dort keinen Flat-Rueckfall.
#[test]
fn store_ohne_bindung_ist_ein_fehler() {
    let st = store(&[
        z("kind_kv", 100_000, true),
        z("kinderbetreuungskosten", 300_000, true),
        z("schulgeld", 100_000, true),
    ]);
    let f = felder(&st);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: None,
        nur_bestaetigt: false,
    };
    let ist_fehler = |r: Result<(), BescheidFehler>| matches!(r, Err(BescheidFehler::BindungFehlt));
    assert!(ist_fehler(kind_kv_pv_summe(&q).map(|_| ())));
    assert!(ist_fehler(
        kinderbetreuung_summe(&q, Vz::Vz2025, params()).map(|_| ())
    ));
    assert!(ist_fehler(p10_1_5_gate_fehlend(&q).map(|_| ())));
    assert!(ist_fehler(
        schulgeld_summe(&q, Vz::Vz2025, &f, params()).map(|_| ())
    ));
    assert!(ist_fehler(kind_behinderten_pb_daten(&q).map(|_| ())));
    assert!(ist_fehler(
        p33b_kind_pauschbetraege(&q, Vz::Vz2025, params()).map(|_| ())
    ));
    assert!(ist_fehler(
        shared_steuer_sonder_agb(
            Euro::new(GDE),
            Euro::new(0),
            Veranlagung::Einzel,
            &f,
            Vz::Vz2025,
            &q,
            params()
        )
        .map(|_| ())
    ));
}
