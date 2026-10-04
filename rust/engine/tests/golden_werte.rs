//! Die 135 Golden-Erwartungen gegen die Rust-Engine -- ohne Orakel, ohne `PARITY=1`, ohne Python.
//!
//! Cutover-Audit `audits/cutover-bereitschaft-rust-port-2026-10-03.md`, Befund 6: Kein Rust-Test verglich
//! den WERT der Erwartung. `golden_kopf.rs` haelt nur die Form (Zahl, ein Feld je Fall, eindeutige IDs); die
//! Parity-Suiten rechnen gegen Python und lesen `erwartung` nie. `make golden` pinnt den Wert, braucht aber
//! Python und Catala. Dieser Test rechnet jeden `sachverhalt` der Fixture `rust/fixtures/golden_cases.json`
//! durch `engine::zugriff::teil2::est::est` (die Rust-Fassung von `runner.catala_est`) und vergleicht die
//! Zahl mit `erwartung`. Die Einheit liefert der Rueckgabetyp (`EstBetrag::Euro` oder `Cent`), sie passt
//! zum Schluessel (`*_cent` gegen Cent, sonst Euro); ein Fehler um Faktor 100 faellt daher sofort auf.
//!
//! Die Abbildung JSON-Dict -> getypte Eingabe ist ein MINIMALER Nachbau von
//! `rust/parity/tests/zugriff_teil2/adapter.rs` (`est`, `sachverhalt` und die Eingabe-Bauer der 13
//! Rechenpfade). Der Adapter bleibt unangetastet; die Python-Ausnahme-Vorhersage (`Fehl::Py`) bleibt dort.
//! Hier wird jeder Fehler zu einem roten Fall mit Name.
//!
//! ponytail: Ceiling -- das ist die Engine-Fassade, nicht der Bescheid-Weg (Snapshot, `bescheid::`-Ring);
//! was im Bescheid mit diesen Zahlen passiert, deckt der Test nicht. Die 106 Euro-Faelle (`tarifliche_est`,
//! `festzusetzende_est`, `abziehbarer_betrag`, `abzug_gesamt`) sind wie bei `make golden` nur auf den
//! vollen Euro genau (`floor`), 29 Cent-Faelle auf den Cent. Aufwertungspfad: den Bescheid-Weg ueber die
//! Snapshot-Abbildung von `bescheid_*_paritaet::golden_faelle` gegen dieselbe `erwartung` pinnen und die
//! Euro-Klassen in Cent fuehren, sobald die Fixture Cent traegt. Faellt Python weg, ist `erwartung` der
//! Anker; gemessen ist sie 2026-10-04 gleich Python (`make golden` 135/135) und gleich Rust (diese Datei).
//!
//! `GOLDEN_CASES_DATEI` lenkt den Test auf eine WEGWERF-Kopie (wie `golden_kopf.rs`), fuer die Rot-Nachweise.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::{Path, PathBuf};
use std::str::FromStr;

use bindung::Params;
use domain::{Euro, Km, Vz};
use engine::tarif::Veranlagung;
use engine::zugriff::teil1::werbungskosten::{EntfernungspauschaleEingabe, RaumkostenEingabe};
use engine::zugriff::teil2::est::{self, EstBetrag, Sachverhalt};
use engine::zugriff::teil2::gesamt::GesamtfallEingabe;
use engine::zugriff::teil2::gewerbe::{
    GewstAusgabe, GewstEingabe, Hinzurechnung, KstEingabe, Kuerzung,
};
use engine::zugriff::teil2::p35c::SanierungEingabe;
use engine::zugriff::teil2::rente::{EinkuenfteVersorgungEingabe, VersorgungsfreibetragEingabe};
use engine::zugriff::teil2::sonstige::KfzNutzungswertEingabe;
use rust_decimal::Decimal;
use serde_json::{Map, Value};

/// Die Zahl der Faelle. Gemessen 2026-10-04 auf `0aa91677`: 135 Elemente in
/// `rust/fixtures/golden_cases.json`, 135 `.yaml` unter `golden/cases/`. Gleiche Zahl wie in
/// `golden_kopf.rs`; wer einen Fall ergaenzt oder loescht, aendert beide im selben Commit.
const N_FAELLE: usize = 135;

/// Die acht Erwartungs-Schluessel, die `golden/golden_lauf.py:75-82` liest.
const ERWARTUNG: [&str; 8] = [
    "tarifliche_est",
    "festzusetzende_est",
    "abziehbarer_betrag",
    "abzug_gesamt",
    "gewst_cent",
    "nenner_b_cent",
    "sanierung_ermaessigung_cent",
    "nutzungswert_monat_cent",
];

type D = Map<String, Value>;
type R<T> = Result<T, String>;

fn wurzel() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn datei() -> PathBuf {
    std::env::var("GOLDEN_CASES_DATEI").map_or_else(
        |_| wurzel().join("rust/fixtures/golden_cases.json"),
        PathBuf::from,
    )
}

fn faelle() -> Vec<Value> {
    let text = std::fs::read_to_string(datei()).expect("golden_cases.json lesen");
    serde_json::from_str(&text).expect("golden_cases.json ist kein JSON-Feld von Faellen")
}

/// Python `int(v)` fuer int/bool.
fn int(v: &Value) -> R<i64> {
    match v {
        Value::Bool(b) => Ok(i64::from(*b)),
        _ => v
            .as_i64()
            .ok_or_else(|| format!("int() auf {v} nicht modelliert")),
    }
}

/// Python-Truthiness.
fn wahr(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

fn flag(d: &D, k: &str) -> bool {
    wahr(d.get(k))
}

/// `int(s.get(k, default))`.
fn get(d: &D, k: &str, default: i64) -> R<i64> {
    d.get(k).map_or(Ok(default), int)
}

fn eur(d: &D, k: &str) -> R<Euro> {
    Ok(Euro::new(get(d, k, 0)?))
}

/// `int(s[k])`.
fn req(d: &D, k: &str) -> R<i64> {
    d.get(k)
        .ok_or_else(|| format!("Schluessel `{k}` fehlt"))
        .and_then(int)
}

/// `s.get(k1) or s.get(k2, 0)`.
fn oder(d: &D, k1: &str, k2: &str) -> R<i64> {
    match d.get(k1) {
        Some(v) if wahr(Some(v)) => int(v),
        _ => get(d, k2, 0),
    }
}

fn vz_von(jahr: i64) -> R<Vz> {
    u16::try_from(jahr)
        .ok()
        .and_then(|j| Vz::try_from(j).ok())
        .ok_or_else(|| format!("Veranlagungszeitraum {jahr} nicht darstellbar"))
}

fn veranlagung(d: &D) -> R<Veranlagung> {
    match d.get("veranlagung").and_then(Value::as_str) {
        Some("einzel") => Ok(Veranlagung::Einzel),
        Some("zusammen") => Ok(Veranlagung::Zusammen),
        a => Err(format!("veranlagung {a:?} unbekannt")),
    }
}

fn gesamtfall(d: &D, jahr: i64) -> R<GesamtfallEingabe> {
    Ok(GesamtfallEingabe {
        vz: vz_von(jahr)?,
        zusammenveranlagung: d.get("veranlagung").and_then(Value::as_str) == Some("zusammen"),
        einkuenfte_nichtselbststaendig: eur(d, "einkuenfte_nichtselbststaendig")?,
        einkuenfte_kapitalvermoegen: eur(d, "einkuenfte_kapitalvermoegen")?,
        einkuenfte_vermietung: eur(d, "einkuenfte_vermietung")?,
        einkuenfte_sonstige: eur(d, "einkuenfte_sonstige")?,
        einkuenfte_gewinn: eur(d, "einkuenfte_gewinn")?,
        altersentlastungsbetrag: eur(d, "altersentlastungsbetrag")?,
        entlastungsbetrag_alleinerziehende: eur(d, "entlastungsbetrag_alleinerziehende")?,
        sonderausgaben: eur(d, "sonderausgaben")?,
        vorsorge_gesamtbeitraege_inkl_ag: eur(d, "vorsorge_gesamtbeitraege_inkl_ag")?,
        vorsorge_ag_anteil_steuerfrei: eur(d, "vorsorge_ag_anteil_steuerfrei")?,
        aussergewoehnliche_belastungen: eur(d, "aussergewoehnliche_belastungen")?,
        freibetraege_kinder: eur(d, "freibetraege_kinder")?,
        sonstige_abzuege_vom_einkommen: eur(d, "sonstige_abzuege_vom_einkommen")?,
        anzurechnende_auslaendische_steuern: eur(d, "anzurechnende_auslaendische_steuern")?,
        steuerermaessigungen: eur(d, "steuerermaessigungen")?,
        steuer_kapital_gesondert: eur(d, "steuer_kapital_gesondert")?,
        hinzurechnung_kindergeld: eur(d, "hinzurechnung_kindergeld")?,
        kinder_ganzjaehrig: if flag(d, "kinder_ganzjaehrig") {
            get(d, "kinder_ganzjaehrig", 0)?
        } else {
            0
        },
        hinzurechnung_zulage: eur(d, "hinzurechnung_zulage")?,
        tarif_modifiziert: flag(d, "tarif_modifiziert"),
        tarifliche_est_modifiziert: eur(d, "tarifliche_est_modifiziert")?,
        versorgung: EinkuenfteVersorgungEingabe {
            versorgung_jahresrente: eur(d, "versorgung_jahresrente")?,
            freibetrag: VersorgungsfreibetragEingabe {
                bemessungsgrundlage: Euro::new(oder(
                    d,
                    "versorgung_bemessungsgrundlage",
                    "versorgungsbezuege_bemessungsgrundlage",
                )?),
                beginn_jahr: oder(d, "versorgung_beginn_jahr", "versorgungsbeginn_jahr")?,
            },
        },
    })
}

fn gewst_eingabe(d: &D, jahr: i64) -> R<GewstEingabe> {
    let ausgabe = if d.get("gewst_output").and_then(Value::as_str) == Some("p35_anrechnung") {
        GewstAusgabe::P35Anrechnung {
            hebesatz: req(d, "gewst_hebesatz")?,
        }
    } else {
        GewstAusgabe::Messbetrag
    };
    Ok(GewstEingabe {
        vz: vz_von(jahr)?,
        ausgabe,
        gewinn_gewerbebetrieb: eur(d, "gewinn_gewerbebetrieb")?,
        hinzurechnung: Hinzurechnung {
            entgelte_schulden: eur(d, "gewst_entgelte_schulden")?,
            renten: eur(d, "gewst_renten")?,
            stille: eur(d, "gewst_stille")?,
            miet_beweglich: eur(d, "gewst_miet_beweglich")?,
            miet_unbeweglich: eur(d, "gewst_miet_unbeweglich")?,
            rechte: eur(d, "gewst_rechte")?,
        },
        kuerzung: Kuerzung {
            einheitswert: eur(d, "gewst_einheitswert")?,
            grundsteuer: eur(d, "gewst_grundsteuer")?,
            gewinnanteile_mitunternehmer: eur(d, "gewst_gewinnanteile_mitunternehmer")?,
            schachteldividenden: eur(d, "gewst_schachteldividenden")?,
        },
        fehlbetrag_bestand: eur(d, "fehlbetrag_bestand")?,
    })
}

fn kst_eingabe(d: &D) -> R<KstEingabe> {
    Ok(KstEingabe {
        gewinn_estg: eur(d, "gewinn_estg")?,
        verdeckte_gewinnausschuettung: eur(d, "verdeckte_gewinnausschuettung")?,
        verdeckte_einlage: eur(d, "verdeckte_einlage")?,
        personensteuern: eur(d, "personensteuern")?,
        geldstrafen: eur(d, "geldstrafen")?,
        dividende_bezuege: eur(d, "dividende_bezuege")?,
        beteiligung_prozent: get(d, "beteiligung_prozent", 0)?,
        veraeusserungsgewinn: eur(d, "veraeusserungsgewinn")?,
        zinsaufwand: eur(d, "zinsaufwand")?,
        zinsertrag: eur(d, "zinsertrag")?,
        abschreibungen: eur(d, "abschreibungen")?,
        zins_vortrag_bestand: eur(d, "zins_vortrag_bestand")?,
        ebitda_vortrag_bestand: eur(d, "ebitda_vortrag_bestand")?,
        keine_konzern_oder_nahestehende_b: flag(d, "keine_konzern_oder_nahestehende_b"),
        eigenkapital_escape_c: flag(d, "eigenkapital_escape_c"),
        verlustvortrag_bestand: eur(d, "verlustvortrag_bestand")?,
        schaedlicher_erwerb: flag(d, "schaedlicher_erwerb"),
        antrag_8d: flag(d, "antrag_8d"),
        fortfuehrungs_voraussetzungen: flag(d, "fortfuehrungs_voraussetzungen"),
        umsaetze: eur(d, "umsaetze")?,
        loehne_gehaelter: eur(d, "loehne_gehaelter")?,
        zuwendungen: eur(d, "zuwendungen")?,
        gewst_hebesatz: req(d, "gewst_hebesatz")?,
    })
}

/// `catala_est`: Zweigwahl in Pythons Reihenfolge (`runner.py:1723-1771`), erste passende gewinnt.
fn sachverhalt(d: &D) -> R<Sachverhalt> {
    if d.contains_key("sanierungsaufwendungen") {
        return Ok(Sachverhalt::Sanierung(SanierungEingabe {
            sanierungsaufwendungen: eur(d, "sanierungsaufwendungen")?,
            ist_uebernaechstes_foerderjahr: flag(d, "ist_uebernaechstes_foerderjahr"),
        }));
    }
    if d.contains_key("bruttolistenpreis") {
        return Ok(Sachverhalt::KfzNutzungswert(KfzNutzungswertEingabe {
            bruttolistenpreis: eur(d, "bruttolistenpreis")?,
            bruchteils_teiler: get(d, "bruchteils_teiler", 1)?,
        }));
    }
    let jahr = req(d, "veranlagungszeitraum")?;
    Ok(if flag(d, "gesamtfall") {
        Sachverhalt::Gesamtfall(gesamtfall(d, jahr)?)
    } else if flag(d, "gewerbesteuer") {
        Sachverhalt::Gewerbesteuer(gewst_eingabe(d, jahr)?)
    } else if flag(d, "koerperschaft") {
        Sachverhalt::Koerperschaft(kst_eingabe(d)?)
    } else if d.contains_key("entfernung_km_roh") {
        let km = d["entfernung_km_roh"].to_string();
        Sachverhalt::Entfernungspauschale(EntfernungspauschaleEingabe {
            veranlagungszeitraum: vz_von(jahr)?,
            entfernung_km_roh: Km::new(
                Decimal::from_str(&km).map_err(|e| format!("Decimal({km}): {e}"))?,
            ),
            arbeitstage: req(d, "arbeitstage")?,
            eigenes_oder_ueberlassenes_kfz: flag(d, "eigenes_oder_ueberlassenes_kfz"),
            oepnv_kosten_jahr: eur(d, "oepnv_kosten_jahr")?,
        })
    } else if d.contains_key("arbeitszimmer_vorhanden") {
        Sachverhalt::Arbeitszimmer(RaumkostenEingabe {
            veranlagungszeitraum: vz_von(jahr)?,
            arbeitszimmer_vorhanden: flag(d, "arbeitszimmer_vorhanden"),
            ist_mittelpunkt: flag(d, "ist_mittelpunkt"),
            tatsaechliche_aufwendungen: eur(d, "tatsaechliche_aufwendungen")?,
            jahrespauschale_gewaehlt: flag(d, "jahrespauschale_gewaehlt"),
            monate_ohne_mittelpunkt: get(d, "monate_ohne_mittelpunkt", 0)?,
            homeoffice_tage: get(d, "homeoffice_tage", 0)?,
        })
    } else if d.contains_key("bruttoarbeitslohn_a") {
        Sachverhalt::BruttoarbeitslohnZusammen(est::EstZusammenEingabe {
            vz: vz_von(jahr)?,
            bruttoarbeitslohn_a: eur(d, "bruttoarbeitslohn_a")?,
            bruttoarbeitslohn_b: eur(d, "bruttoarbeitslohn_b")?,
            werbungskosten_a: eur(d, "werbungskosten_a")?,
            werbungskosten_b: eur(d, "werbungskosten_b")?,
            sonderausgaben_gemeinsam: eur(d, "sonderausgaben_gemeinsam")?,
        })
    } else if d.contains_key("bruttoarbeitslohn") {
        Sachverhalt::Bruttoarbeitslohn(est::EstEinzelEingabe {
            vz: vz_von(jahr)?,
            bruttoarbeitslohn: Euro::new(req(d, "bruttoarbeitslohn")?),
            werbungskosten: eur(d, "werbungskosten")?,
            sonderausgaben: eur(d, "sonderausgaben")?,
        })
    } else if d.contains_key("ausserordentliche_einkuenfte") {
        Sachverhalt::Fuenftel(est::FuenftelEingabe {
            vz: vz_von(jahr)?,
            veranlagung: veranlagung(d)?,
            zu_versteuerndes_einkommen: Euro::new(req(d, "zu_versteuerndes_einkommen")?),
            ausserordentliche_einkuenfte: Euro::new(req(d, "ausserordentliche_einkuenfte")?),
        })
    } else {
        Sachverhalt::Tarif(est::TarifEingabe {
            vz: vz_von(jahr)?,
            veranlagung: veranlagung(d)?,
            zu_versteuerndes_einkommen: Euro::new(req(d, "zu_versteuerndes_einkommen")?),
        })
    })
}

/// Rechnet den Sachverhalt in der Einheit seines Zweigs (Euro oder Cent, wie `erwartung`).
fn berechne(d: &D, p: &Params) -> R<i64> {
    match est::est(&sachverhalt(d)?, p).map_err(|e| format!("{e:?}"))? {
        EstBetrag::Euro(x) => Ok(x.get()),
        EstBetrag::Cent(x) => Ok(x.get()),
    }
}

#[test]
fn fallzahl_ist_135() {
    let n = faelle().len();
    assert_eq!(
        n, N_FAELLE,
        "Golden-Korpus hat {n} Faelle statt {N_FAELLE}: ein Fall fehlt oder ist dazugekommen. \
         Zahl in dieser Datei UND in golden_kopf.rs im selben Commit nachziehen."
    );
}

#[test]
fn rust_rechnet_jeden_golden_fall_auf_die_erwartung() {
    let p = Params::lade(&wurzel()).expect("params laden");
    let alle = faelle();
    let mut abweichungen: Vec<String> = Vec::new();
    let mut gerechnet = 0_usize;
    for fall in &alle {
        let id = fall["id"].as_str().unwrap_or("<ohne id>");
        let erwartet: Vec<(&str, &Value)> = fall["erwartung"]
            .as_object()
            .map(|o| {
                ERWARTUNG
                    .iter()
                    .filter_map(|k| o.get(*k).map(|v| (*k, v)))
                    .collect()
            })
            .unwrap_or_default();
        let [(schluessel, wert)] = erwartet[..] else {
            abweichungen.push(format!(
                "{id}: {} Erwartungs-Schluessel aus den acht bekannten statt genau einem",
                erwartet.len()
            ));
            continue;
        };
        let Some(soll) = wert.as_i64() else {
            abweichungen.push(format!(
                "{id} [{schluessel}]: erwartet {wert} ist keine Ganzzahl"
            ));
            continue;
        };
        let Some(sv) = fall["sachverhalt"].as_object() else {
            abweichungen.push(format!("{id} [{schluessel}]: sachverhalt ist kein Objekt"));
            continue;
        };
        gerechnet += 1;
        match berechne(sv, &p) {
            Ok(ist) if ist == soll => {}
            Ok(ist) => {
                abweichungen.push(format!("{id} [{schluessel}]: Rust {ist}, erwartet {soll}"));
            }
            Err(e) => abweichungen.push(format!(
                "{id} [{schluessel}]: Rust kann nicht rechnen ({e}), erwartet {soll}"
            )),
        }
    }
    assert!(
        abweichungen.is_empty(),
        "{} von {} Golden-Faellen weichen ab ({gerechnet} gerechnet):\n{}",
        abweichungen.len(),
        alle.len(),
        abweichungen.join("\n")
    );
    // Ein Test, der nichts rechnet, ist gruen und beweist nichts: jeder Fall muss gerechnet worden sein.
    assert_eq!(gerechnet, alle.len(), "nicht jeder Fall wurde gerechnet");
}
