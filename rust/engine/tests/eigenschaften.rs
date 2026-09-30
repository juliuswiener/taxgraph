//! Eigenschaften der reinen Rechenlogik (`REWRITE_PLAN.md` §9): Schranken, Monotonie, Symmetrie.
//! Rein Rust gegen den echten Catala-Kern und die echten Jahresparameter, ohne Python-Orakel.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;
use std::sync::OnceLock;

use bindung::Params;
use domain::Cent;
use engine::tarif::{
    festzusetzende_est_einzel, festzusetzende_est_gesamt, festzusetzende_est_zusammen, grundtarif,
    splittingtarif, FestzusetzendeEstEinzelEingabe, FestzusetzendeEstZusammenEingabe,
    GesamtEingabe, Vz,
};
use engine::zugriff::teil1::pauschbetraege::grundfreibetrag;
use proptest::prelude::*;

fn params() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap())
}

/// Dasselbe Jahr als Catala-`Vz` und als `domain::Vz` (die Parameter-Tabellen brauchen Letzteres).
fn jahre() -> impl Strategy<Value = (Vz, domain::Vz)> {
    prop_oneof![
        Just((Vz::Vz2024, domain::Vz::Vz2024)),
        Just((Vz::Vz2025, domain::Vz::Vz2025)),
        Just((Vz::Vz2026, domain::Vz::Vz2026)),
    ]
}

/// zvE in Cent bis 20 Mio. EUR (weit ueber der Reichensteuer-Grenze); die Haelfte der Faelle
/// liegt bis 30.000 EUR, damit der Grundfreibetrag-Bereich ausreichend oft getroffen wird.
fn zve() -> impl Strategy<Value = i64> {
    prop_oneof![0_i64..3_000_000, 0_i64..2_000_000_000]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Der Tarif ist im zvE monoton nicht fallend.
    #[test]
    fn grundtarif_ist_monoton(a in zve(), d in 0_i64..500_000_000, (vz, _) in jahre()) {
        let niedrig = grundtarif(Cent::new(a), vz).unwrap();
        let hoch = grundtarif(Cent::new(a + d), vz).unwrap();
        prop_assert!(niedrig <= hoch);
    }

    /// Splitting(2x) = 2 · Grund(x).
    #[test]
    fn splitting_ist_das_doppelte_des_halben(x in zve(), (vz, _) in jahre()) {
        let gem = splittingtarif(Cent::new(2 * x), vz).unwrap();
        let halb = grundtarif(Cent::new(x), vz).unwrap();
        prop_assert_eq!(gem.get(), 2 * halb.get());
    }

    /// Schranken je VZ: nie negativ, nie mehr als der Spitzensatz 45 % des zvE, steuerfrei bis zum Grundfreibetrag.
    #[test]
    fn tarif_bleibt_in_den_schranken(z in zve(), (vz, dvz) in jahre()) {
        let est = grundtarif(Cent::new(z), vz).unwrap().get();
        prop_assert!(est >= 0);
        prop_assert!(est * 100 <= z * 45, "{est} > 45 % von {z}");
        let gfb = grundfreibetrag(dvz, params()).unwrap().get() * 100;
        if z <= gfb {
            prop_assert_eq!(est, 0);
        }
    }

    /// Kein Sprung: ein Euro mehr zvE kostet hoechstens 45 Cent plus die Euro-Rundung.
    #[test]
    fn tarif_steigt_hoechstens_mit_dem_spitzensatz(z in zve(), (vz, _) in jahre()) {
        let a = grundtarif(Cent::new(z), vz).unwrap().get();
        let b = grundtarif(Cent::new(z + 100), vz).unwrap().get();
        prop_assert!(b - a <= 100, "Anstieg {} Cent bei +1 EUR", b - a);
    }

    /// Die End-zu-End-Steuer ist nie negativ (Einzel, Zusammen, Gesamtfall).
    #[test]
    fn festzusetzende_est_ist_nie_negativ(
        brutto in 0_i64..1_000_000_000, wk in 0_i64..50_000_000, so in 0_i64..50_000_000,
        brutto_b in 0_i64..1_000_000_000, (vz, _) in jahre()
    ) {
        let einzel = festzusetzende_est_einzel(FestzusetzendeEstEinzelEingabe {
            bruttoarbeitslohn: Cent::new(brutto), werbungskosten: Cent::new(wk), sonderausgaben: Cent::new(so),
        }, vz).unwrap();
        prop_assert!(einzel >= Cent::new(0));
        let zusammen = festzusetzende_est_zusammen(FestzusetzendeEstZusammenEingabe {
            bruttoarbeitslohn_a: Cent::new(brutto), werbungskosten_a: Cent::new(wk),
            bruttoarbeitslohn_b: Cent::new(brutto_b), werbungskosten_b: Cent::new(0),
            sonderausgaben_gemeinsam: Cent::new(so),
        }, vz).unwrap();
        prop_assert!(zusammen >= Cent::new(0));
        let n = Cent::new(0);
        let gesamt = festzusetzende_est_gesamt(GesamtEingabe {
            einkuenfte_nichtselbststaendig: Cent::new(brutto), einkuenfte_kapitalvermoegen: n,
            einkuenfte_vermietung: n, einkuenfte_sonstige: n, einkuenfte_gewinn: n,
            altersentlastungsbetrag: n, entlastungsbetrag_alleinerziehende: n,
            sonderausgaben: Cent::new(so), aussergewoehnliche_belastungen: n, freibetraege_kinder: n,
            sonstige_abzuege_vom_einkommen: n, anzurechnende_auslaendische_steuern: n,
            steuerermaessigungen: n, steuer_kapital_gesondert: n, hinzurechnung_kindergeld: n,
            hinzurechnung_zulage: n, tarif_modifiziert: false, tarifliche_est_modifiziert: n,
        }, vz).unwrap();
        prop_assert!(gesamt.festzusetzende_est_cent >= 0);
    }

    /// Mehr Werbungskosten oder mehr Sonderausgaben erhoehen die Steuer nie.
    #[test]
    fn mehr_abzuege_mehr_entlastung(brutto in 0_i64..1_000_000_000, wk in 0_i64..50_000_000, dwk in 0_i64..50_000_000, (vz, _) in jahre()) {
        let est = |w: i64| festzusetzende_est_einzel(FestzusetzendeEstEinzelEingabe {
            bruttoarbeitslohn: Cent::new(brutto), werbungskosten: Cent::new(w), sonderausgaben: Cent::new(0),
        }, vz).unwrap();
        prop_assert!(est(wk + dwk) <= est(wk));
    }
}
