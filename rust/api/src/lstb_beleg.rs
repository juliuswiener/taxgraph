//! `POST /fall/{id}/beleg` (Abweichung Nr. 90, Stufe S1, Rust-eigen): die Beleg-Datei der Lohnsteuerbescheinigung (`VaSt_LStB`, Schema 202501)
//! kommt als Text im JSON-Feld `beleg_xml`; die Beträge der Person A gehen über `eingang::edaten::uebernehme` als BESTÄTIGT in den Fall
//! (§ 150 Abs. 7 S. 2 AO: Daten der Finanzverwaltung gelten als Angaben des Steuerpflichtigen).
//!
//! Alles oder nichts: ein Fehler beim Lesen des Belegs, ein falsches Jahr des Falls oder eine Abweisung des Stores lässt die Akte unberührt,
//! denn gespeichert wird erst nach dem letzten Satz. Der Dienst legt den Beleg nicht ab; Steuer-ID und Namen aus dem Beleg erreichen weder
//! die Akte noch ein Log noch eine Antwort (der Leser gibt sie nie heraus, [`eingang::vast_beleg`]).
//!
//! Grenzen: nur Person A, nur Veranlagungszeitraum 2025, genau ein Beleg, kein Abruf bei ELSTER. Ein Feld, das schon einen Wert trägt (eigene
//! Angabe, auch ein vorläufiger Vorschlag), und ein Feld außerhalb der Scheibe des Falls stehen unter `uebersprungen`.
use std::collections::BTreeSet;

use eingang::edaten::{uebernehme, Satz};
use eingang::vast::aus_lstb;
use eingang::vast_beleg::{beleg_xml_zu_werten, BelegFehler, BELEG_JAHR, BELEG_VERSION};
use eingang::vorschlag::SchreibFehler;
use serde_json::{json, Value};
use store::BindungNachschlag;

use crate::antwort::Antwort;
use crate::eigener_fall::EigenerFall;
use crate::fehler::ApiFehler;
use crate::zustand::Zustand as Dienst;

/// Ein Fehlertext mit dem stabilen Namen vorn: `beleg_mehrere: Die Datei enthält 2 Belege. …`.
fn benannt(code: &str, text: impl std::fmt::Display) -> String {
    format!("{code}: {text}")
}

fn beleg_fehler(e: &BelegFehler) -> ApiFehler {
    ApiFehler::status(422, benannt(e.code(), e))
}

/// Der Rumpf nach dem Owner-Check: lesen, abbilden, übernehmen, speichern.
///
/// # Errors
/// 400 ohne `beleg_xml` als Text; 422 für einen Beleg, der nicht genau einer in Version 202501 ist (Name des Fehlers vor dem Text), für einen
/// Fall außerhalb von 2025 und für eine Abweisung des Stores; 401/403/404 wie bei jedem Fall; 500 nur für Fehler des Dienstes selbst.
pub fn beleg(z: &Dienst, fall: &mut EigenerFall, body: &Value) -> Result<Antwort, ApiFehler> {
    let Some(xml) = body.get("beleg_xml").and_then(Value::as_str) else {
        return Err(ApiFehler::status(
            400,
            benannt("beleg_xml_fehlt", "Der Rumpf braucht das Feld beleg_xml mit dem Text der Beleg-Datei."),
        ));
    };
    let vz = fall.store().datei().veranlagungszeitraum.0;
    if vz.to_string() != BELEG_JAHR {
        return Err(ApiFehler::status(
            422,
            benannt(
                "beleg_fall_jahr",
                format!("Der Fall gilt für das Jahr {vz}. Der Import liest nur Belege für {BELEG_JAHR} (Version {BELEG_VERSION})."),
            ),
        ));
    }
    let werte = beleg_xml_zu_werten(xml).map_err(|e| beleg_fehler(&e))?;
    let saetze: Vec<Satz> = aus_lstb(&werte)
        .map_err(|e| ApiFehler::status(422, benannt("beleg_betrag_unlesbar", e)))?
        .into_iter()
        .map(|s| Satz {
            feld_id: s.feld_id,
            wert: Value::from(s.wert),
            kategorie: Value::from(s.kategorie),
        })
        .collect();

    let sb = z.scheibe_bindung(fall.store())?;
    let vorher: BTreeSet<String> = fall.store().aktive().map(|(f, _)| f.to_owned()).collect();
    uebernehme(
        fall.store_mut(),
        &saetze,
        None,
        Some(BindungNachschlag::neu(&sb.index)),
    )
    .map_err(|e| match e {
        SchreibFehler::Abweisung(a) => ApiFehler::status(422, benannt("beleg_abgewiesen", a)),
        SchreibFehler::Konstante => ApiFehler::unerwartet("ValueError", e.to_string()),
    })?;
    let nachher: BTreeSet<String> = fall.store().aktive().map(|(f, _)| f.to_owned()).collect();
    let (mut uebernommen, mut uebersprungen) = (Vec::new(), Vec::new());
    for s in &saetze {
        let neu = !vorher.contains(&s.feld_id) && nachher.contains(&s.feld_id);
        if neu { &mut uebernommen } else { &mut uebersprungen }.push(s.feld_id.clone());
    }
    store::speichere(fall.pfad(), fall.store().datei())?;
    Ok(Antwort::neu(
        200,
        json!({
            "uebernommen": uebernommen,
            "uebersprungen": uebersprungen,
            "beleg_version": BELEG_VERSION,
        }),
    ))
}
