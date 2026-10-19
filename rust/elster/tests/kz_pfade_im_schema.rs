//! Der PFAD, an dem ein Kz im amtlichen Schema steht, nicht nur dass es dort genau einmal steht. Gegenstueck zu den Pfad-Tests in
//! `tests/test_xsd_verify.py` (Sweep Cluster A/B/C, Task #13/#15, E77-Weiche).
//!
//! **Worum es geht.** `vz_gueltigkeit_schema.rs` prueft je Kz, dass es im Schema eines gelisteten Jahres GENAU EINMAL steht. Ein Kz,
//! das in den falschen Abschnitt wandert (die haushaltsnahen Aufwendungen in "Krankheitskosten" statt in `HA_35a`), steht danach immer
//! noch genau einmal im Schema und bleibt dort gruen; ERiC lehnt die Erklaerung erst ab, wenn der Wert im falschen Abschnitt landet
//! (Sweep 2026-07-20, `reports/review/2026-07-20-xsd-kz-section-sweep-findings.md`). Diese Tabelle haelt die gemessenen Abschnitte fest.
//! Gegenversuch Nr. 69 `X2`: ein Kz gleichen Typs aus einem anderen Abschnitt blieb in jedem Rust-Test gruen.
//!
//! **Das Schema liegt nur lokal** (`$ERIC_DIR`, sonst `~/02_Software/eric`). Fehlt es, ist der Test rot; nur `TAXGRAPH_OHNE_XSD=1` laesst
//! ihn aus (`elster::testhilfe::schemas_da`), und das steht auf stderr. Die CI setzt die Variable und prueft die Pfade damit NICHT.
//!
//! Grenze: die Tabelle ist eine Abschrift der gemessenen Abschnitte (Stand E10-2025 / E77-2025). Ein neues Kz ohne Zeile bleibt
//! ungeprueft; die Eindeutigkeit aller Kz haelt `vz_gueltigkeit_schema.rs`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::sync::OnceLock;

use bindung::Bindung;
use domain::{Achsenwert, Herkunft, PruefTiefe, Zustand};
use elster::testhilfe::schemas_da;
use elster::{deklariere, ernte_est_mapping_kz, finde_schema, xsd_walk, Felder, KzPruefling};
use serde_json::{json, Value};
use store::SnapshotFeld;

const VZ: i64 = 2025;

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        bindung::lade_registry_der_wurzel(&wurzel)
            .expect("Bindung laedt")
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

/// Kz -> alle Pfade (`E10/.../Kz`), aus dem Schema `muster` des Jahres `VZ`.
fn pfade(muster: &str, start: &str) -> HashMap<String, Vec<String>> {
    let schema = finde_schema(VZ, muster).unwrap_or_else(|| panic!("{muster} fehlt"));
    let (fundstellen, abbrueche) = xsd_walk(&schema, start).unwrap();
    assert_eq!(abbrueche, 0, "{muster}: der Walk brach an der Tiefe ab");
    fundstellen
        .into_iter()
        .map(|(kz, wege)| (kz, wege.iter().map(|w| w.join("/")).collect()))
        .collect()
}

fn pfade_e10() -> &'static HashMap<String, Vec<String>> {
    static CELL: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();
    CELL.get_or_init(|| pfade("E10-{jahr}.xsd", "E10"))
}

fn ernte() -> &'static [KzPruefling] {
    static CELL: OnceLock<Vec<KzPruefling>> = OnceLock::new();
    CELL.get_or_init(|| ernte_est_mapping_kz(index()).expect("Tabellen-Kz haben ein Feld in der Bindung"))
}

/// (`feld_id` der Bindung, Kz, Pfad im E10-Schema 2025) -- Sweep Cluster A (§ 35a, § 33), B (Kapital), C (Behinderung Person A), dazu seit
/// Abweichung Nr. 92 die drei steuerfreien Arbeitgeberzuschuesse zur KV/PV (Anlage Vorsorgeaufwand, `Stfr_AG_Zusch`, Nr. 24 a, b, c).
const BINDUNG_PFADE: [(&str, &str, &str); 10] = [
    ("zuschuss_ag_kv_gesetzlich", "E2003705", "E10/VOR/Stfr_AG_Zusch/E2003705"),
    ("zuschuss_ag_kv_privat", "E2003807", "E10/VOR/Stfr_AG_Zusch/E2003807"),
    ("zuschuss_ag_pv", "E2003907", "E10/VOR/Stfr_AG_Zusch/E2003907"),
    ("hh_minijob_aufwendungen", "E0104109", "E10/HA_35a/St_Erm/Minijobs/Sum/E0104109"),
    ("hh_dienstleistungen", "E0107208", "E10/HA_35a/St_Erm/Hhn_BV_DL/Sum/E0107208"),
    ("hh_handwerker_arbeitskosten", "E0111215", "E10/HA_35a/St_Erm/Handw_L/Sum/E0111215"),
    // AgB/And_Aufw hat drei Unterbuckets (Pflege, Beh_Aufw, Sonst); der Fragetext passt auf den Sonst-Catch-all (Lead-Entscheid).
    ("agb_aufwendungen", "E0161804", "E10/AgB/And_Aufw/Sonst/Sum/E0161804"),
    // § 20 Abs. 9: die Anlage-KAP-Elternzeile, NICHT die Unterhaltsleistungs-Zusatzsektion des Hauptvordrucks (ESt1A_U/.../KapV/E0121709).
    ("kap_kapitalertraege", "E1900701", "E10/KAP/KapErt_inl_StAbz/Betr_lt_StBesch/E1900701"),
    ("rentner_grad_der_behinderung", "E0109708", "E10/AgB/Beh/Ausw_Rentb_Besch/E0109708"),
    ("rentner_hilflos_blind_taubblind", "E0109706", "E10/AgB/Beh/Geh_Steh_Blind_Hilfl/E0109706"),
];

#[test]
fn die_kz_der_bindung_liegen_in_ihrem_abschnitt() {
    if !schemas_da(VZ) {
        return;
    }
    for (feld, kz, pfad) in BINDUNG_PFADE {
        let b = index()[feld];
        assert_eq!(b.elster_kz.as_ref().map(domain::Kz::as_str), Some(kz), "{feld}: das Kz der Bindung");
        assert_eq!(pfade_e10().get(kz).map(Vec::as_slice), Some(&[pfad.to_owned()][..]), "{feld} ({kz})");
    }
}

/// (Kennung aus `ernte_est_mapping_kz`, Kz, Pfad) -- Task #13 (Rente), Task #15 (Veraeusserungsgewinn: alle drei Anlagen).
const ERNTE_PFADE: [(&str, &str, &str); 4] = [
    ("verzweigung:rentner_jahresrente:gesetzliche_rente", "E1800301", "E10/R/Leibr_gesetzl/Einz/E1800301"),
    (
        "verzweigung:rentner_veraeusserungsgewinn:gewerbe",
        "E0801301",
        "E10/G/VAe_G_v_FB/Betr_TBetr_MUAnt/VAe_G_FB_Antr/E0801301",
    ),
    // 'selbstaendig' MUSS auf Anlage S zeigen (E0804501), nicht auf Anlage L (E0901201).
    ("verzweigung:rentner_veraeusserungsgewinn:selbstaendig", "E0804501", "E10/S/VAe_Gew/Vor_FB/E0804501"),
    ("verzweigung:rentner_veraeusserungsgewinn:land_forst", "E0901201", "E10/L/VAe_G_v_FB/VAe_G_FB_Antr/E0901201"),
];

#[test]
fn die_kz_der_art_verzweigung_liegen_in_ihrer_anlage() {
    if !schemas_da(VZ) {
        return;
    }
    for (id, kz, pfad) in ERNTE_PFADE {
        let k = ernte().iter().find(|k| k.feld_id == id).unwrap_or_else(|| panic!("{id} fehlt in der Ernte"));
        assert_eq!(k.elster_kz, kz, "{id}: das Kz der Tabelle");
        assert_eq!(pfade_e10().get(kz).map(Vec::as_slice), Some(&[pfad.to_owned()][..]), "{id} ({kz})");
    }
}

/// Python: `test_real_schema_k_verh_a_b_disambiguierung`, die goldene Probe des Walkers und Anlass von `xsd_verify` ueberhaupt: zwei Kz
/// gleichen Typs in Geschwister-Elementen (`K_Verh_A`, `K_Verh_B`) behalten je ihren eigenen Pfad.
#[test]
fn k_verh_a_und_k_verh_b_liegen_in_getrennten_zweigen() {
    if !schemas_da(VZ) {
        return;
    }
    for (kz, pfad) in [
        ("E0500807", "E10/Kind/K_Verh/K_Verh_A/E0500807"),
        ("E0500808", "E10/Kind/K_Verh/K_Verh_B/E0500808"),
    ] {
        assert_eq!(pfade_e10().get(kz).map(Vec::as_slice), Some(&[pfad.to_owned()][..]), "{kz}");
    }
}

#[test]
fn die_e60_kz_der_euer_stehen_im_e77_schema() {
    if !schemas_da(VZ) {
        return;
    }
    assert!(finde_schema(VZ, "E77-{jahr}.xsd").is_some(), "E77-{VZ}.xsd fehlt: die E60-Kz blieben ungeprueft");
    let e77 = pfade("E77-{jahr}.xsd", "E77");
    for feld in ["gwg_anschaffungskosten_netto", "sonstige_betriebsausgaben"] {
        let kz = index()[feld].elster_kz.as_ref().unwrap().as_str();
        assert!(kz.starts_with("E60"), "{feld}: {kz} ist kein E60-Kz");
        assert_eq!(e77.get(kz).map(Vec::len), Some(1), "{feld} ({kz}) steht nicht genau einmal in E77-{VZ}");
        // Und NICHT im E10-Schema: dort ist es ein Fehler, den der Writer ueber `E10_AUSSCHLUSS_DATENART` kennt.
        assert!(!pfade_e10().contains_key(kz), "{feld} ({kz}) steht auch im E10-Schema");
    }
}

// ---------------------------------------------------------------------------------------------------------------------------------
// Person B teilt die Kz von Person A (Klasse g, `PARTNER_INSTANZ`): kein eigenes Kz in der Bindung, aber derselbe Wert im Bucket B.

fn laie() -> Herkunft {
    let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
    Herkunft { herkunft: a("laie"), pruef_tiefe: PruefTiefe::Ungeprueft, haftung: a("nutzer") }
}

fn bestaetigt(paare: &[(&str, Value)]) -> Felder {
    paare
        .iter()
        .map(|(f, w)| {
            (
                (*f).to_owned(),
                SnapshotFeld { wert: w.clone().into(), zustand: Zustand::Bestaetigt, herkunft: laie().into() },
            )
        })
        .collect()
}

#[test]
fn die_partner_spiegel_haben_kein_eigenes_kz_und_schreiben_in_das_kz_von_person_a() {
    // (Partner-Feld, Wert, das Kz von Person A, in das Person B schreibt). Kein Schema noetig: die Tabelle `PARTNER_INSTANZ`.
    for (feld, wert, kz) in [
        ("kap_kapitalertraege_partner", json!(100_000), "E1900701"),
        ("rentner_grad_der_behinderung_partner", json!(50), "E0109708"),
        ("rentner_hilflos_blind_taubblind_partner", json!(true), "E0109706"),
        ("zuschuss_ag_kv_gesetzlich_partner", json!(150_000), "E2003705"),
        ("zuschuss_ag_kv_privat_partner", json!(110_000), "E2003807"),
        ("zuschuss_ag_pv_partner", json!(9_000), "E2003907"),
        ("berufsverband_beitraege_partner", json!(17_800), "E0204003"),
    ] {
        assert!(index()[feld].elster_kz.is_none(), "{feld}: ein Partner-Spiegel traegt kein eigenes Kz (Klasse g)");
        let d = deklariere(&bestaetigt(&[("veranlagung", json!("zusammen")), (feld, wert)]), index(), VZ, None).unwrap();
        assert!(d.person_b.contains_key(kz), "{feld}: Person B schreibt nicht nach {kz}: {:?}", d.person_b.keys().collect::<Vec<_>>());
        assert!(!d.deklaration.contains_key(kz), "{feld}: der Wert von Person B landete im Bucket von Person A");
    }
}
