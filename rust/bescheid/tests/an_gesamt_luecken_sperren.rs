//! Drei Sperren des Gesamt-Guards `an_gesamt_sperrgrund`, die bis zur Autopsie 2026-10-10 nur Python-Tests hatten:
//! `kinder_gehoeren_in_gesamt`, `progression_gehoert_in_gesamt` und `partner_vor_offen`. Standardlauf, ohne `PARITY=1`,
//! ohne Python.
//!
//! **Worum es geht.** Die Scheibe `an_gesamt` rechnet ohne Kinderfreibetrag, ohne Lohnersatz-Progression und ohne die
//! Vorsorgebeitraege beider Partner. Wer solche Angaben macht, bekommt dort keine Zahl, sondern einen Grund, und die
//! Angabe gehoert in die Scheibe `gesamt`.
//!
//! **Warum es zaehlt.** Fiele eine der drei Sperren weg (Feld umbenannt, Bedingung gekippt), rechnete `an_gesamt` still
//! zu wenig Abzug (Kinder, Partner-Vorsorge) oder zu wenig Steuer (Lohnersatz erhoeht den Satz). Ihre Python-Tests
//! (`test_progression_gehoert_in_gesamt.py`, `test_paket_b_e2e_http.py`, `test_ring_regression_kampagne.py`) fallen mit
//! der Python-Loeschung (Loeschplan L2). Das Feld-Kennungs-Gate (`feld_kennung_gate.rs`) sieht die Leser `positiv` und
//! `bestaetigt` nicht, ein umbenanntes Feld bliebe also ohne diese Datei unbemerkt.
//!
//! **Wo es sitzt.** `deklaration/sperre.rs`: `an_gesamt_luecken` (Kinder, Verlustvortrag, Lohnersatz; nur Scheiben ohne
//! `gesamt_guard`) und `ohne_gesamt_guard` (Zusammenveranlagung mit Vorsorgefeldern).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Gruende und ihre Reihenfolge stehen in `_an_gesamt_sperrgrund`
//! (`bescheid_deklaration.py:1092-1101` und `:1597-1601`); die Python-Tests belegen sie fuer Kinder (`fam_anzahl_kinder=2`),
//! Lohnersatz (`120000` Cent, vorlaeufig und bestaetigt, `0` sperrt nie) und Vorsorge (`vor_an_anteil_rv=350000`). Die
//! uebrigen fuenf Vorsorgefelder und die Gegenproben auf `gesamt` und `rentner_gesamt` sind Rust-eigen. Eine Rechnung
//! gibt es nicht: DASS die Scheibe sperrt, ist Entwurf des Projekts.
//!
//! Jede Sperre hat Gegenfaelle, die KEINE Sperre liefern. Ohne sie bestuende auch ein Guard, der immer denselben Grund
//! liefert, jeden Fall.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::sync::OnceLock;

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Sperrgrund, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

const KINDER: Option<&str> = Some("kinder_gehoeren_in_gesamt");
const VERLUSTVORTRAG: Option<&str> = Some("verlustvortrag_gehoert_in_gesamt");
const PROGRESSION: Option<&str> = Some("progression_gehoert_in_gesamt");
const PARTNER_VOR: Option<&str> = Some("partner_vor_offen");
const PARTNER_KEGEL: Option<&str> = Some("partner_kegel_offen");

/// 3.500,00 EUR in Cent: ein Vorsorgebeitrag.
const VOR: i64 = 350_000;
/// 1.200,00 EUR in Cent: Lohnersatz (Elterngeld, Krankengeld).
const LOHNERSATZ: i64 = 120_000;

/// Die sechs Vorsorgefelder (`VOR_FELDER` und `VOR_PARTNER_FELDER`), absichtlich als Literale: eine Umbenennung in
/// `konstanten.rs` macht den Test rot.
const VOR_FELDER: [&str; 6] = [
    "vor_an_anteil_rv",
    "vor_ag_anteil_rv",
    "vor_rv_ausserhalb_lstb",
    "vor_an_anteil_rv_partner",
    "vor_ag_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner",
];

/// Ein bestaetigtes Zahlfeld.
fn z(fid: &'static str, wert: i64) -> Ev {
    (fid, json!(wert), true)
}

/// Ein bestaetigtes Textfeld.
fn t(fid: &'static str, wert: &'static str) -> Ev {
    (fid, json!(wert), true)
}

/// Die Bindung der Scheibe: nur ihre Feld-Ids, wie `api._scheibe_bindung`.
fn scheiben_index(scheibe: Scheibe) -> &'static BindungIndex<'static> {
    static GESAMT: OnceLock<BindungIndex<'static>> = OnceLock::new();
    static RENTNER: OnceLock<BindungIndex<'static>> = OnceLock::new();
    static AN_GESAMT: OnceLock<BindungIndex<'static>> = OnceLock::new();
    let zelle = match scheibe {
        Scheibe::RentnerGesamt => &RENTNER,
        Scheibe::AnGesamt => &AN_GESAMT,
        _ => &GESAMT,
    };
    zelle.get_or_init(|| {
        let ids = Cfg::fuer(scheibe).felder(|_| Vec::new()).unwrap();
        index()
            .iter()
            .filter(|(k, _)| ids.contains(k))
            .map(|(k, b)| (k.clone(), *b))
            .collect()
    })
}

/// Der Sperrgrund der Scheibe fuer VZ 2025 auf einem Store aus den Events (der Guard sieht Roh-Felder).
fn grund(scheibe: Scheibe, events: &[Ev]) -> Option<&'static str> {
    let st = store(events);
    let f = felder(&st);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(scheiben_index(scheibe)),
        nur_bestaetigt: false,
    };
    let cfg = Cfg::fuer(scheibe);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q)
        .unwrap()
        .map(Sperrgrund::als_str)
}

/// `events` plus `mehr`.
fn plus(mut events: Vec<Ev>, mehr: Vec<Ev>) -> Vec<Ev> {
    events.extend(mehr);
    events
}

/// Zusammenveranlagung mit dem Partner-Kegel von `an_gesamt` (`AN_GESAMT_PARTNER`: `bruttoarbeitslohn_partner`).
fn zusammen() -> Vec<Ev> {
    vec![
        t("veranlagung", "zusammen"),
        z("bruttoarbeitslohn_partner", 0),
    ]
}

/// Zusammenveranlagung mit dem Partner-Kegel von `gesamt` (Lohn und fuenf KAP-Felder, Nullen).
fn zusammen_gesamt() -> Vec<Ev> {
    plus(
        zusammen(),
        vec![
            z("kap_kapitalertraege_partner", 0),
            z("kap_gewinn_aktien_partner", 0),
            z("kap_gewinn_sonstige_partner", 0),
            z("kap_verlust_aktien_partner", 0),
            z("kap_verlust_sonstige_partner", 0),
        ],
    )
}

/// Kinder: `an_gesamt` hat keinen Kinderfreibetrag und kein Kindergeld-Vergleich. Zwei Kinder sperren dort, null Kinder
/// nie, und die Scheiben mit `gesamt_guard` (`gesamt`, `rentner_gesamt`) rechnen die Kinder selbst.
#[test]
fn kinder_sperren_die_an_scheibe_und_nur_sie() {
    let zwei = [z("fam_anzahl_kinder", 2)];
    assert_eq!(grund(Scheibe::AnGesamt, &zwei), KINDER, "zwei Kinder");
    let eins_vorlaeufig = [("fam_anzahl_kinder", json!(1), false)];
    assert_eq!(
        grund(Scheibe::AnGesamt, &eins_vorlaeufig),
        KINDER,
        "ein Kind, vorlaeufig: der Guard liest den Rohwert"
    );
    assert_eq!(
        grund(Scheibe::AnGesamt, &[z("fam_anzahl_kinder", 0)]),
        None,
        "null Kinder (Kontrolle: der Guard liefert nicht immer einen Grund)"
    );
    assert_eq!(grund(Scheibe::Gesamt, &zwei), None, "gesamt rechnet Kinder");
    assert_eq!(
        grund(Scheibe::RentnerGesamt, &zwei),
        None,
        "rentner_gesamt rechnet Kinder"
    );
}

/// Lohnersatz (§ 32b): der Guard liest den Rohwert ABSICHTLICH ohne Zustandspruefung, auch ein vorlaeufiger Wert haelt an.
/// Null ist der Normalfall und sperrt nie. Auf den Scheiben mit `gesamt_guard` gilt der Grund nicht.
#[test]
fn lohnersatz_sperrt_die_an_scheibe_auch_vorlaeufig_und_nur_sie() {
    let bestaetigt = [z("p32b_progressionseinkuenfte", LOHNERSATZ)];
    let vorlaeufig = [("p32b_progressionseinkuenfte", json!(LOHNERSATZ), false)];
    assert_eq!(
        grund(Scheibe::AnGesamt, &bestaetigt),
        PROGRESSION,
        "bestaetigt"
    );
    assert_eq!(
        grund(Scheibe::AnGesamt, &vorlaeufig),
        PROGRESSION,
        "vorlaeufig"
    );
    assert_eq!(
        grund(Scheibe::AnGesamt, &[z("p32b_progressionseinkuenfte", 0)]),
        None,
        "null Lohnersatz (Kontrolle)"
    );
    assert_eq!(
        grund(Scheibe::Gesamt, &bestaetigt),
        None,
        "gesamt rechnet die Progression"
    );
    assert_eq!(
        grund(Scheibe::RentnerGesamt, &bestaetigt),
        None,
        "rentner_gesamt rechnet die Progression"
    );
}

/// Die Reihenfolge ist Teil des Verhaltens, die erste Luecke gewinnt: Kinder vor Verlustvortrag vor Lohnersatz.
#[test]
fn die_erste_luecke_der_an_scheibe_gewinnt() {
    let kinder = z("fam_anzahl_kinder", 2);
    let verlust = z("verlustvortrag_bestand", 500_000);
    let lohnersatz = z("p32b_progressionseinkuenfte", LOHNERSATZ);
    assert_eq!(
        grund(
            Scheibe::AnGesamt,
            &[lohnersatz.clone(), verlust.clone(), kinder]
        ),
        KINDER
    );
    assert_eq!(
        grund(Scheibe::AnGesamt, &[lohnersatz, verlust]),
        VERLUSTVORTRAG
    );
}

/// Zusammenveranlagung auf `an_gesamt` rechnet keine Vorsorgebeitraege: ein positives Vorsorgefeld von Person A oder B
/// sperrt, je eines der sechs Felder allein.
#[test]
fn zusammen_mit_vorsorgebeitrag_sperrt_je_feld() {
    for feld in VOR_FELDER {
        let events = plus(zusammen(), vec![z(feld, VOR)]);
        assert_eq!(grund(Scheibe::AnGesamt, &events), PARTNER_VOR, "{feld}");
    }
}

/// Gegenfaelle zu `partner_vor_offen`: ohne Vorsorgebeitrag rechnet die Zusammenveranlagung, bei Einzelveranlagung
/// sperrt ein Vorsorgebeitrag nicht (er geht in die Rechnung), und `gesamt` rechnet die Vorsorge selbst.
#[test]
fn partner_vor_offen_hat_gegenfaelle_ohne_sperre() {
    let nullen: Vec<Ev> = VOR_FELDER.into_iter().map(|f| z(f, 0)).collect();
    assert_eq!(
        grund(Scheibe::AnGesamt, &plus(zusammen(), nullen)),
        None,
        "zusammen, alle sechs Vorsorgefelder 0 (Kontrolle)"
    );
    for feld in VOR_FELDER {
        assert_eq!(
            grund(Scheibe::AnGesamt, &[t("veranlagung", "einzel"), z(feld, VOR)]),
            None,
            "einzel mit {feld}"
        );
    }
    assert_eq!(
        grund(
            Scheibe::Gesamt,
            &plus(zusammen_gesamt(), vec![z("vor_an_anteil_rv", VOR)])
        ),
        None,
        "gesamt, zusammen mit Vorsorgebeitrag"
    );
}

/// Der Partner-Kegel steht vor den Vorsorgefeldern: fehlt `bruttoarbeitslohn_partner`, nennt der Guard den Kegel.
#[test]
fn fehlender_partner_kegel_geht_vor_dem_vorsorgegrund() {
    let events = [
        t("veranlagung", "zusammen"),
        z("vor_an_anteil_rv", VOR),
    ];
    assert_eq!(grund(Scheibe::AnGesamt, &events), PARTNER_KEGEL);
}
