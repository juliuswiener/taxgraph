//! Die Jahresliste (`vz_gueltigkeit`) jedes Kz-Felds gegen das amtliche Schema (Abnahme-Folgeauftrag 2).
//! Gegenstueck zur Ratsche `rust/bindung/tests/vz_bestand.rs`, die jede Aenderung der Liste an einen Grund bindet.
//!
//! Warum es diese Datei gibt: `elster::pruefe_bindung` prueft je Kz und gelistetem Jahr, ob das Kz genau
//! einmal im Schema des Jahres steht — aber kein Standard-Test rief es auf (gemessen 2026-10-06: nur der
//! Python-Pruefpass `xsd_verify`, der mit `produkt/` einfriert). Wer ein Jahr dazuschreibt, in dem das Kz im
//! Schema fehlt, wurde nur dort gewarnt (Beleg im Kopf von `bindung_p10_1a_realsplitting_gesamt.yaml`: der
//! Pruefpass fing die Erweiterung auf 2024).
//!
//! Was geprueft wird: jedes Kz eines Felds (und jedes Kz der Transform-Tabellen, `ernte_est_mapping_kz`) steht in
//! jedem gelisteten Jahr, zu dem ein lokales Schema da ist (2024, 2025), im Schema. Ein Jahr ohne lokales Schema
//! (2026) bleibt ungeprueft; fuer 2026 und fuer Felder ohne Kz haelt allein die Ratsche.
//!
//! Grenze: nur die Richtung "gelistet => im Schema". Dass ein Feld ein Jahr NICHT listet, obwohl sein Kz dort im
//! Schema steht, ist erlaubt (das Gesetz kann ein Jahr ausnehmen) und faengt nur die Ratsche. Ohne Schema rot,
//! ausser `TAXGRAPH_OHNE_XSD=1` (`elster::testhilfe::schemas_da`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use bindung::Bindung;
use elster::testhilfe::schemas_da;
use elster::{ernte_est_mapping_kz, finde_schema, pruefe_bindung, KzPruefling, PruefStatus};

/// Die Jahre, zu denen ein lokales Schema da sein muss; jedes andere gelistete Jahr bleibt ungeprueft.
const GEPRUEFTE_JAHRE: [i64; 2] = [2024, 2025];

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        // Die Bindung des Dienstes (`rust/bindung/daten`), nie ein fester Pfad im Test.
        let wurzel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
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

/// Die Schemas, die dieser Test braucht, sind da (E10 und E77 je Jahr). Fehlen sie, ist es rot, ausser
/// `TAXGRAPH_OHNE_XSD=1`; dann `false` und der Aufrufer ueberspringt.
fn schemas_vorhanden() -> bool {
    // `schemas_da` kennt nur E10; die E60-Kz laufen gegen E77. Ohne E10 ist es schon rot (oder uebersprungen).
    GEPRUEFTE_JAHRE.iter().all(|j| {
        schemas_da(*j) && {
            assert!(
                finde_schema(*j, "E77-{jahr}.xsd").is_some(),
                "E77-{j}.xsd fehlt: ohne sie blieben die E60-Kz ungeprueft"
            );
            true
        }
    })
}

/// Jedes Kz mit seinen gelisteten Jahren: die `elster_kz` der Felder und die Kz der Transform-Tabellen.
fn prueflinge() -> Vec<KzPruefling> {
    let mut p: Vec<KzPruefling> = bindungen()
        .iter()
        .filter_map(|b| {
            Some(KzPruefling {
                feld_id: b.feld_id.clone(),
                elster_kz: b.elster_kz.as_ref()?.as_str().to_owned(),
                vz_gueltigkeit: b.vz_gueltigkeit.clone(),
            })
        })
        .collect();
    p.extend(ernte_est_mapping_kz(index()).expect("Transform-Kz haben ein Feld in der Bindung"));
    p
}

/// Prueflinge, deren gelistete Jahre auf die geprueften Jahre eingeschraenkt sind.
fn auf_geprueft(p: &[KzPruefling]) -> Vec<KzPruefling> {
    p.iter()
        .map(|k| KzPruefling {
            vz_gueltigkeit: k
                .vz_gueltigkeit
                .iter()
                .copied()
                .filter(|j| GEPRUEFTE_JAHRE.contains(j))
                .collect(),
            ..k.clone()
        })
        .collect()
}

/// `feld (Kz): Jahr Status` fuer jedes gelistete, geprueften Jahr, in dem das Kz nicht genau einmal im Schema
/// steht (wie der Pruefpass `xsd_verify`: `exit_code == 0` nur bei lauter `Ok`). Gemessen am 2026-10-06: 2024 und
/// 2025 sind bei allen gelisteten Kz `Ok`, keines `Mehrdeutig`.
fn nicht_genau_einmal(p: &[KzPruefling]) -> Vec<String> {
    let bericht = pruefe_bindung(&auf_geprueft(p)).expect("Schema auswertbar");
    let kz: HashMap<&str, &str> = p
        .iter()
        .map(|k| (k.feld_id.as_str(), k.elster_kz.as_str()))
        .collect();
    let mut out = Vec::new();
    for (feld, f) in &bericht.felder {
        for (jahr, j) in &f.jahre {
            if j.status != PruefStatus::Ok {
                out.push(format!(
                    "{feld} ({}): {jahr} {:?}",
                    kz[feld.as_str()],
                    j.status
                ));
            }
        }
    }
    out
}

#[test]
fn jedes_gelistete_jahr_hat_das_kz_im_schema() {
    if !schemas_vorhanden() {
        return;
    }
    let p = prueflinge();
    // Die Pruefung laeuft nicht leer: viele Kz, und jedes geprueft Jahr kommt vor.
    assert!(
        p.len() >= 200,
        "nur {} Prueflinge (Erfassung: 227)",
        p.len()
    );
    for j in GEPRUEFTE_JAHRE {
        assert!(
            p.iter().any(|k| k.vz_gueltigkeit.contains(&j)),
            "kein Kz listet {j}"
        );
    }
    let fehlt = nicht_genau_einmal(&p);
    assert!(
        fehlt.is_empty(),
        "{} Kz stehen in einem gelisteten Jahr nicht genau einmal im Schema: {:#?}\nDas Jahr aus \
         `vz_gueltigkeit` nehmen (und in rust/bindung/tests/vz_bestand.yaml fuehren) oder das Kz berichtigen.",
        fehlt.len(),
        &fehlt[..fehlt.len().min(15)]
    );
}

/// Ohne diese Probe waere nicht belegt, dass der Pruefer ein fehlendes Kz sieht: im Normalfall ist er gruen.
/// `E0300717` (Realsplitting, KV/PV des Empfaengers) steht erst im Schema 2025 (gemessen am 2026-10-06:
/// nicht in E10-2023 und E10-2024).
#[test]
fn der_pruefer_sieht_ein_kz_das_im_jahr_fehlt() {
    if !schemas_vorhanden() {
        return;
    }
    let kz = |jahre: &[i64]| {
        vec![KzPruefling {
            feld_id: "probe".to_owned(),
            elster_kz: "E0300717".to_owned(),
            vz_gueltigkeit: jahre.to_vec(),
        }]
    };
    assert_eq!(nicht_genau_einmal(&kz(&[2025])).len(), 0);
    let fehlt = nicht_genau_einmal(&kz(&[2024, 2025]));
    assert_eq!(fehlt.len(), 1, "{fehlt:?}");
    assert!(
        fehlt[0].contains("2024") && fehlt[0].contains("NichtGefunden"),
        "{fehlt:?}"
    );
    // Ein Jahr ohne lokales Schema (2026) zaehlt nicht als Fehler dieses Tests: die Ratsche haelt es.
    assert_eq!(nicht_genau_einmal(&kz(&[2025, 2026])).len(), 0);
}
