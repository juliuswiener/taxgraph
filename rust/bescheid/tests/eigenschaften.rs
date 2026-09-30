//! Eigenschaften des Rechenrings, rein in Rust (ohne Python-Orakel) — `REWRITE_PLAN.md` §9.
//!
//! Gerechnet wird der AN-Gesamtzweig `festzusetzende_est` ueber [`bescheid_fn`], mit den echten
//! Jahresparametern und dem echten Catala-Kern.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::{bescheid_fn, Umgebung};
use bescheid::Felder;
use bindung::SlotBeitrag;
use domain::{Feldtyp, Vz};
use intervall::{AchsenBindung, Werte};
use proptest::prelude::*;
use serde_json::{json, Value};

const SLOTS: [&str; 6] = [
    "arbeitstage",
    "entfernung_km_roh",
    "oepnv_kosten_jahr",
    "eigenes_oder_ueberlassenes_kfz",
    "bruttoarbeitslohn",
    "veranlagung",
];

fn achsen() -> Vec<AchsenBindung> {
    SLOTS
        .iter()
        .map(|s| AchsenBindung {
            feld_id: (*s).to_owned(),
            typ: Feldtyp::Int,
            askable: true,
            enum_werte: Vec::new(),
            bereich: None,
            signatur_slot: Some((*s).to_owned()),
            slot_beitrag: SlotBeitrag::Exakt,
        })
        .collect()
}

/// Die Slot-Eingaben eines Laufs (Person A).
#[derive(Clone, Copy, Debug)]
struct Lauf {
    vz: Vz,
    brutto_cent: i64,
    arbeitstage: i64,
    km: i64,
}

impl Lauf {
    fn einfach(brutto_cent: i64) -> Self {
        Self {
            vz: Vz::Vz2025,
            brutto_cent,
            arbeitstage: 0,
            km: 0,
        }
    }
}

/// `festzusetzende_est` in Cent.
fn est(f: &Felder, l: Lauf, nur_bestaetigt: bool) -> i64 {
    let achsen = achsen();
    let umg = Umgebung {
        achsen: &achsen,
        index: index(),
        params: params(),
    };
    let rechne = bescheid_fn(
        "festzusetzende_est",
        l.vz,
        &umg,
        Some(f),
        None,
        nur_bestaetigt,
        None,
        None,
    )
    .expect("Quantitaet bekannt");
    let mut w = Werte::neu();
    w.setze("arbeitstage", json!(l.arbeitstage));
    w.setze("entfernung_km_roh", json!(l.km));
    w.setze("oepnv_kosten_jahr", json!(0));
    w.setze("eigenes_oder_ueberlassenes_kfz", json!(true));
    w.setze("bruttoarbeitslohn", json!(l.brutto_cent));
    w.setze("veranlagung", json!("einzel"));
    rechne(&w).expect("Zweig rechnet").get()
}

fn vz() -> impl Strategy<Value = Vz> {
    prop_oneof![Just(Vz::Vz2024), Just(Vz::Vz2025), Just(Vz::Vz2026)]
}

/// Felder, die den AN-Zweig bewegen (Vorsorge, Cent-Betraege).
const POOL: [&str; 5] = [
    "basis_kv",
    "basis_pv",
    "vor_an_anteil_rv",
    "vor_ag_anteil_rv",
    "vorsorge_arbeitslosenversicherung",
];

fn belegung() -> impl Strategy<Value = Vec<(usize, i64, bool)>> {
    prop::collection::vec(
        (0..POOL.len(), 0_i64..3_000_000, any::<bool>()),
        0..POOL.len(),
    )
}

/// Store aus einer Belegung; je Feld hoechstens ein Eintrag (spaeterer gewinnt).
fn baue(b: &[(usize, i64, bool)], vorlaeufig_ersatz: Option<i64>) -> Felder {
    let mut je_feld: Vec<Option<(i64, bool)>> = vec![None; POOL.len()];
    for (i, w, best) in b {
        je_feld[*i] = Some((*w, *best));
    }
    let events: Vec<(&str, Value, bool)> = je_feld
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            e.map(|(w, best)| {
                let wert = if best {
                    w
                } else {
                    vorlaeufig_ersatz.unwrap_or(w)
                };
                (POOL[i], json!(wert), best)
            })
        })
        .collect();
    felder(&store(&events))
}

/// Zusammenveranlagung: Person A ueber den Slot, Person B ueber das Partner-Feld.
fn est_zusammen(a_cent: i64, b_cent: i64, v: Vz) -> i64 {
    let f = felder(&store(&[
        ("veranlagung", json!("zusammen"), true),
        ("bruttoarbeitslohn_partner", json!(b_cent), true),
    ]));
    est(
        &f,
        Lauf {
            vz: v,
            ..Lauf::einfach(a_cent)
        },
        true,
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Ein vorlaeufiger Wert bewegt bei `nur_bestaetigt` die festgesetzte Steuer nie: egal, welchen
    /// Wert die vorlaeufigen Felder tragen, der Betrag bleibt.
    #[test]
    fn vorlaeufiges_bewegt_den_betrag_nie(b in belegung(), ersatz in 0_i64..5_000_000, brutto in 0_i64..20_000_000) {
        let l = Lauf::einfach(brutto);
        let a = est(&baue(&b, None), l, true);
        let c = est(&baue(&b, Some(ersatz)), l, true);
        prop_assert_eq!(a, c);
    }

    /// Ergebnis >= 0 fuer jede Eingabe, jedes Jahr.
    #[test]
    fn steuer_nie_negativ(b in belegung(), brutto in 0_i64..40_000_000, v in vz()) {
        let l = Lauf { vz: v, ..Lauf::einfach(brutto) };
        prop_assert!(est(&baue(&b, None), l, true) >= 0);
    }

    /// Mehr Werbungskosten (mehr Arbeitstage bzw. mehr Kilometer) → die Steuer steigt nie.
    #[test]
    fn mehr_werbungskosten_erhoehen_die_steuer_nie(
        brutto in 1_000_000_i64..15_000_000, t in 0_i64..250, dt in 0_i64..100, km in 0_i64..80, dkm in 0_i64..40, v in vz()
    ) {
        let f = felder(&store(&[]));
        let basis = Lauf { vz: v, brutto_cent: brutto, arbeitstage: t, km };
        let mehr = Lauf { arbeitstage: t + dt, km: km + dkm, ..basis };
        prop_assert!(est(&f, mehr, true) <= est(&f, basis, true));
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// Symmetrie: wer Person A und Person B vertauscht, bekommt dieselbe gemeinsame Steuer.
    #[test]
    fn zusammenveranlagung_ist_symmetrisch(a in 0_i64..15_000_000, b in 0_i64..15_000_000, v in vz()) {
        prop_assert_eq!(est_zusammen(a, b, v), est_zusammen(b, a, v));
    }

    /// Splittingvorteil: bei einem Alleinverdiener ist die gemeinsame Steuer nie hoeher als die Einzelsteuer.
    #[test]
    fn splitting_ist_fuer_den_alleinverdiener_nie_teurer(a in 0_i64..20_000_000, v in vz()) {
        let einzel = est(&felder(&store(&[])), Lauf { vz: v, ..Lauf::einfach(a) }, true);
        prop_assert!(est_zusammen(a, 0, v) <= einzel);
    }
}

/// Die Vorsorge-Felder muessen tatsaechlich wirken; sonst waere die Unabhaengigkeit oben leer.
#[test]
fn bestaetigte_vorsorge_mindert_die_steuer() {
    let l = Lauf::einfach(8_000_000);
    let ohne = est(&felder(&store(&[])), l, true);
    let mit = est(
        &felder(&store(&[("basis_kv", json!(600_000), true)])),
        l,
        true,
    );
    assert!(mit < ohne, "{mit} !< {ohne}");
    // dasselbe Feld vorlaeufig: bei nur_bestaetigt unsichtbar, im Schaetzpfad sichtbar
    let vorl = felder(&store(&[("basis_kv", json!(600_000), false)]));
    assert_eq!(est(&vorl, l, true), ohne);
    assert_eq!(est(&vorl, l, false), mit);
}
