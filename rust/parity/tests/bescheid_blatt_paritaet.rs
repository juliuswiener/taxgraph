//! Parität `rust/bescheid` (Schritt 7a: `abzuege`, `einkuenfte`) gegen `produkt/bescheid/*.py` über
//! `tools/parity/bescheid_oracle.py` (Präfix `bescheid.` in `oracle.py`). EIN Orakel-Prozess je Binary.
//!
//! Vier Eingabequellen, je mit Vergleichszahl beider Seiten (Zeilen = Funktionsaufrufe):
//! - `reale_faelle`: jede Fall-Datei unter `faelle_verzeichnis()`, je `nur_bestaetigt` ∈ {true,false} ×
//!   zwei Argument-Varianten × (Store übergeben / nicht übergeben).
//! - `golden_faelle`: `rust/fixtures/golden_cases.json`, Sachverhalt als Snapshot gelesen (ohne Store;
//!   je einmal roh und einmal ×100 für plausible Cent-Größen).
//! - `generierte_faelle`: ≥ 1.000 proptest-Fälle (Store aus Events, negative Beträge, vorläufige und
//!   fehlende Felder, falsch typisierte Werte), jeder Fall durch ALLE Funktionen.
//! - `dba_methode_generiert`: ≥ 1.000 Werte-Paare direkt in `dba_methode_fuer`.
//!
//! Fehler-Parität: Python-Ausnahme ↔ Rust-`Err` je Aufruf; ist die Rust-Klasse bekannt und nicht die
//! Sammelklasse `CatalaError`, muss sie gleich heißen.
//! Negativkontrolle: `negativkontrolle_*` stört ein Rust-Ergebnis um 1 und verlangt genau eine
//! Abweichung; `PARITY_STOERUNG=1` schaltet dieselbe Störung in `reale_faelle` (Lauf wird rot).
//!
//! SICHERHEIT: reale Fälle sind echte Steuerdaten. Nur lokal lesen, nur Zählwerte ausgeben.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `bescheid_blatt_paritaet` -- --nocapture
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]
#![allow(
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::{Mutex, OnceLock};

use bescheid::abzuege::{self, KindPbDaten};
use bescheid::einkuenfte as ek;
use bescheid::{BescheidFehler, Felder, Instanzquelle};
use bindung::{Bindung, Params};
use domain::{Euro, PyWert, Veranlagung, Vz};
use engine::zugriff::teil2::gesamt::GesamtfallEingabe;
use engine::zugriff::teil2::rente::{EinkuenfteVersorgungEingabe, VersorgungsfreibetragEingabe};
use parity::Oracle;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use serde_json::{json, Value};
use store::{EventId, Store, StoreDatei};

/// Bekannt leere Zeilen im Block `golden_faelle` -- Korpus `golden_cases.json`, nicht der
/// Fall-Korpus, darum an [`parity::pin::KORPUS`] gebunden wie alle Listen hier.
///
/// Grund fuer alle 14: die Vorlage traegt Aggregat-Schluessel
/// (`zu_versteuerndes_einkommen`, `gesamtfall`), das Blatt erwartet Feld-IDs. Die Zeile ist
/// richtig gebaut und wird von ihrem Block nicht erreicht -- dieselben 14 rechnen im Block
/// `generierte_faelle` derselben Suite (249/250/113/49/129/189/1064/982/247/677/243/48/242/378).
const LEER_GOLDEN: &[(&str, &str)] = &[
    ("abs3_eligible", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("gewinn_partner_anteil", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("gwg_sofortabzug_summe", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("kind_behinderten_pb_daten", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("kind_kv_pv_summe", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("kinderbetreuung_summe", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("laufender_gewinn", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("laufender_gewinn_partner", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("p10_1_5_gate_fehlend", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("p20_kapitaleinkuenfte", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("p23_ansonsten_einkuenfte", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("p33b_kind_pauschbetraege", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("p35_partner_anteile", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
    ("schulgeld_summe", "Vorlage traegt Aggregat-Schluessel, keine Feld-ID"),
];

/// Bekannt leere Zeilen im Block `reale_faelle`. Alle acht rechnen im Block
/// `generierte_faelle` derselben Suite (Zahlen im Kommentar je Eintrag) -- es ist also
/// KEINE tote Zeile, sondern eine, die der reale Korpus nicht ausloest.
///
/// Massstab ist der WERT, nicht die Feldliste. Zwei Felder (`kind_kv`,
/// `rentner_rentenfreibetrag`) stehen nicht im Kegel und bewegen die Vergleichszahl
/// trotzdem -- in allen sechs geprueften Faellen (gemessen von `gdb-bau`). Nach einer
/// Feldliste gaelten 20 von 36 Varianten als blind, nach dem Wert 0 von 36. Wer hier
/// eine Feldliste als Kriterium einbaut, meldet falschen Alarm.
const LEER_REALE: &[(&str, &str)] = &[
    ("gewinn_partner_anteil", "realer Korpus trifft den Zweig nicht; generierte: 250"),
    ("kind_behinderten_pb_daten", "realer Korpus trifft den Zweig nicht; generierte: 49"),
    ("kinderbetreuung_summe", "realer Korpus trifft den Zweig nicht; generierte: 189"),
    ("laufender_gewinn_partner", "realer Korpus trifft den Zweig nicht; generierte: 982"),
    ("p10_1_5_gate_fehlend", "realer Korpus trifft den Zweig nicht; generierte: 247"),
    ("p23_ansonsten_einkuenfte", "realer Korpus trifft den Zweig nicht; generierte: 243"),
    ("p33b_kind_pauschbetraege", "realer Korpus trifft den Zweig nicht; generierte: 48"),
    ("shared_dba_sonstige", "realer Korpus trifft den Zweig nicht; generierte: 716"),
];

const FUNKTIONEN: &[&str] = &[
    "abs3_eligible",
    "oepnv_eur",
    "kind_kv_pv_summe",
    "kinderbetreuung_summe",
    "p10_1_5_gate_fehlend",
    "schulgeld_summe",
    "kind_behinderten_pb_daten",
    "p33b_kind_pauschbetraege",
    "shared_steuer_sonder_agb",
    "gwg_sofortabzug_summe",
    "laufender_gewinn_partner",
    "laufender_gewinn",
    "gewinn_partner_anteil",
    "p20_kapitaleinkuenfte",
    "p23_ansonsten_einkuenfte",
    "p35_partner_anteile",
    "p35_gezahlte_gewst",
    "p35_summen",
    "shared_dba_sonstige",
    "dba_methode_fuer",
];

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn stoerung_an() -> bool {
    std::env::var("PARITY_STOERUNG").as_deref() == Ok("1")
}

fn faelle_verzeichnis() -> std::path::PathBuf {
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
    match std::env::var("TAXGRAPH_DATEN") {
        Ok(p) if !p.trim().is_empty() => std::path::PathBuf::from(p.trim()).join("faelle"),
        _ => match std::env::var("XDG_DATA_HOME") {
            Ok(x) if !x.trim().is_empty() => {
                std::path::PathBuf::from(x.trim()).join("taxgraph/faelle")
            }
            _ => home.join(".local/share/taxgraph/faelle"),
        },
    }
}


fn params() -> &'static Params {
    static P: OnceLock<Params> = OnceLock::new();
    P.get_or_init(|| Params::lade(&repo_root()).expect("params laden"))
}

fn bindungen() -> &'static [Bindung] {
    static B: OnceLock<Vec<Bindung>> = OnceLock::new();
    B.get_or_init(|| {
        let reg = bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("registry");
        reg.dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static I: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    I.get_or_init(|| store::baue_nachschlag(bindungen()))
}

fn oracle() -> std::sync::MutexGuard<'static, Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn frage(req: &Value) -> Value {
    let antwort = oracle().call_json(req).expect("Orakel antwortet");
    match antwort.get("ok") {
        Some(v) => v.clone(),
        None => panic!("Orakel-Harness-Fehler für {}: {antwort}", req["fn"]),
    }
}

// ---------------------------------------------------------------- Eingaben

/// Ein Vergleichsfall: derselbe Kontext geht an Python (JSON) und an Rust.
#[derive(Clone)]
struct Fall {
    store: Option<Value>,
    felder: Option<Value>,
    vz: u16,
    nur: bool,
    store_uebergeben: bool,
    bindung_uebergeben: bool,
    args: Value,
}

impl Fall {
    fn request(&self, funktionen: &[&str]) -> Value {
        json!({"fn": "bescheid.fall", "funktionen": funktionen.iter().map(|n| format!("bescheid.{n}")).collect::<Vec<_>>(),
            "store": self.store, "felder": self.felder, "vz": self.vz, "nur_bestaetigt": self.nur,
            "store_uebergeben": self.store_uebergeben, "bindung_uebergeben": self.bindung_uebergeben,
            "args": self.args})
    }
}

/// Rust-Kontext eines Falls (Snapshot wie in `_bescheid_fn`: materialisiert, bei `nur` gefiltert).
struct Ctx {
    f: Felder,
    store: Option<Store>,
    bindung: bool,
    vz: Vz,
    nur: bool,
    args: Value,
}

fn baue_ctx(fall: &Fall) -> Ctx {
    let (mut f, store) = match (&fall.store, &fall.felder) {
        (Some(s), _) => {
            let datei: StoreDatei =
                serde_json::from_value(s.clone()).expect("Store-Datei deserialisiert");
            let st = Store::aus_datei(datei);
            (st.materialisiere(None).expect("materialisiere").0, Some(st))
        }
        (None, Some(fe)) => (serde_json::from_value(fe.clone()).expect("felder"), None),
        (None, None) => (Felder::new(), None),
    };
    if fall.nur && !f.is_empty() {
        f.retain(|_, v| v.zustand == domain::Zustand::Bestaetigt);
    }
    Ctx {
        f,
        store: if fall.store_uebergeben { store } else { None },
        bindung: fall.bindung_uebergeben,
        vz: Vz::try_from(fall.vz).expect("vz 2024..2026"),
        nur: fall.nur,
        args: fall.args.clone(),
    }
}

fn a_i64(c: &Ctx, k: &str) -> i64 {
    c.args[k].as_i64().unwrap_or(0)
}

fn a_euro(c: &Ctx, k: &str) -> Euro {
    Euro::new(a_i64(c, k))
}

fn veranlagung(c: &Ctx) -> Veranlagung {
    if c.args["veranlagung"] == "zusammen" {
        Veranlagung::Zusammen
    } else {
        Veranlagung::Einzel
    }
}

fn gesamtfall(c: &Ctx) -> GesamtfallEingabe {
    let g = &c.args["g"];
    let e = |k: &str| Euro::new(g[k].as_i64().unwrap_or(0));
    GesamtfallEingabe {
        vz: c.vz,
        zusammenveranlagung: g["veranlagung"] == "zusammen",
        einkuenfte_nichtselbststaendig: e("einkuenfte_nichtselbststaendig"),
        einkuenfte_kapitalvermoegen: e("einkuenfte_kapitalvermoegen"),
        einkuenfte_vermietung: e("einkuenfte_vermietung"),
        einkuenfte_sonstige: e("einkuenfte_sonstige"),
        einkuenfte_gewinn: e("einkuenfte_gewinn"),
        altersentlastungsbetrag: e("altersentlastungsbetrag"),
        entlastungsbetrag_alleinerziehende: e("entlastungsbetrag_alleinerziehende"),
        sonderausgaben: e("sonderausgaben"),
        vorsorge_gesamtbeitraege_inkl_ag: e("vorsorge_gesamtbeitraege_inkl_ag"),
        vorsorge_ag_anteil_steuerfrei: e("vorsorge_ag_anteil_steuerfrei"),
        aussergewoehnliche_belastungen: e("aussergewoehnliche_belastungen"),
        freibetraege_kinder: e("freibetraege_kinder"),
        sonstige_abzuege_vom_einkommen: e("sonstige_abzuege_vom_einkommen"),
        anzurechnende_auslaendische_steuern: e("anzurechnende_auslaendische_steuern"),
        steuerermaessigungen: e("steuerermaessigungen"),
        steuer_kapital_gesondert: e("steuer_kapital_gesondert"),
        hinzurechnung_kindergeld: e("hinzurechnung_kindergeld"),
        kinder_ganzjaehrig: 0,
        hinzurechnung_zulage: e("hinzurechnung_zulage"),
        tarif_modifiziert: false,
        tarifliche_est_modifiziert: e("tarifliche_est_modifiziert"),
        // Python: fehlende Versorgungsfelder → `VersorgungsfreibetragOffen` verschluckt, 0 EUR (Befund P2).
        versorgung: EinkuenfteVersorgungEingabe {
            versorgung_jahresrente: Euro::new(0),
            freibetrag: VersorgungsfreibetragEingabe {
                bemessungsgrundlage: Euro::new(0),
                beginn_jahr: 0,
            },
        },
    }
}

fn eu(e: Euro) -> Value {
    json!(e.get())
}

fn tupel(e: &[Euro]) -> Value {
    Value::Array(e.iter().map(|x| eu(*x)).collect())
}

/// Ruft die Rust-Funktion `name`; das Ergebnis hat dieselbe JSON-Form wie die Orakel-Antwort.
fn rust_run(c: &Ctx, name: &str) -> Result<Value, BescheidFehler> {
    let p = params();
    let q = Instanzquelle {
        store: c.store.as_ref(),
        bindung: c.bindung.then(index),
        nur_bestaetigt: c.nur,
    };
    Ok(match name {
        "abs3_eligible" => json!(abzuege::abs3_eligible(&c.f, c.vz)?),
        "oepnv_eur" => {
            // K2: `Slots` traegt `PyWert`; die Orakel-Seite liefert JSON, also konvertiert
            // `PyWert::from` (total) hier EINMAL an der Grenze.
            let slots = c.args["slots"]
                .as_object()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| (k.clone(), PyWert::from(v.clone())))
                        .collect()
                })
                .unwrap_or_default();
            eu(abzuege::oepnv_eur(&slots)?)
        }
        "kind_kv_pv_summe" => json!(abzuege::kind_kv_pv_summe(&q)?.get()),
        "kinderbetreuung_summe" => eu(abzuege::kinderbetreuung_summe(&q, c.vz, p)?),
        "p10_1_5_gate_fehlend" => json!(abzuege::p10_1_5_gate_fehlend(&q)?
            .into_iter()
            .collect::<Vec<_>>()),
        "schulgeld_summe" => eu(abzuege::schulgeld_summe(&q, c.vz, &c.f, p)?),
        "kind_behinderten_pb_daten" => Value::Array(
            abzuege::kind_behinderten_pb_daten(&q)?
                .into_iter()
                .map(
                    |KindPbDaten {
                         grad_der_behinderung,
                         ist_hilflos_blind_taubblind,
                         hat_hinterbliebenenbezuege,
                     }| {
                        json!({"grad_der_behinderung": grad_der_behinderung,
                        "ist_hilflos_blind_taubblind": ist_hilflos_blind_taubblind,
                        "hat_hinterbliebenenbezuege": hat_hinterbliebenenbezuege})
                    },
                )
                .collect(),
        ),
        "p33b_kind_pauschbetraege" => eu(abzuege::p33b_kind_pauschbetraege(&q, c.vz, p)?),
        "shared_steuer_sonder_agb" => {
            let r = abzuege::shared_steuer_sonder_agb(
                a_euro(c, "gde"),
                a_euro(c, "ausserg"),
                veranlagung(c),
                &c.f,
                c.vz,
                &q,
                p,
            )?;
            json!({"steuerermaessigungen": r.steuerermaessigungen.get(), "sonderausgaben": r.sonderausgaben.get(),
                "aussergewoehnliche_belastungen": r.aussergewoehnliche_belastungen.get()})
        }
        "gwg_sofortabzug_summe" => eu(ek::gwg_sofortabzug_summe(&c.f, &q)?),
        "laufender_gewinn_partner" => {
            let (a, b) = ek::laufender_gewinn_partner(&c.f)?;
            tupel(&[a, b])
        }
        "laufender_gewinn" => {
            let (a, b) = ek::laufender_gewinn(&c.f, &q)?;
            tupel(&[a, b])
        }
        "gewinn_partner_anteil" => {
            let (a, b, d) = ek::gewinn_partner_anteil(&c.f)?;
            tupel(&[a, b, d])
        }
        "p20_kapitaleinkuenfte" => eu(ek::p20_kapitaleinkuenfte(
            &c.f,
            c.args["zusammen"] == true,
            c.vz,
            p,
        )?),
        "p23_ansonsten_einkuenfte" => eu(ek::p23_ansonsten_einkuenfte(&q)?),
        "p35_partner_anteile" => {
            let (m, h, z) = ek::p35_partner_anteile(&c.f)?;
            json!([m.get(), h, z.get()])
        }
        "p35_gezahlte_gewst" => eu(ek::p35_gezahlte_gewst(
            a_euro(c, "messbetrag_a"),
            a_i64(c, "hebesatz_a"),
            a_euro(c, "messbetrag_b"),
            a_i64(c, "hebesatz_b"),
        )?),
        "p35_summen" => {
            let (m, z, g) = ek::p35_summen(
                &c.f,
                a_euro(c, "messbetrag_a"),
                a_i64(c, "hebesatz_a"),
                a_euro(c, "zaehler_a"),
            )?;
            tupel(&[m, z, g])
        }
        "shared_dba_sonstige" => {
            let mut g = gesamtfall(c);
            let r = ek::shared_dba_sonstige(
                &mut g,
                a_euro(c, "gde_p10d"),
                veranlagung(c),
                &c.f,
                c.vz,
                p,
            )?;
            json!({"ret": r.dba_anrechnung.get(),
                "sonstige_abzuege_vom_einkommen": g.sonstige_abzuege_vom_einkommen.get(),
                "anzurechnende_auslaendische_steuern": g.anzurechnende_auslaendische_steuern.get(),
                "p32b_progressionseinkuenfte": r.p32b_progressionseinkuenfte.map(Euro::get)})
        }
        "dba_methode_fuer" => {
            // K2: die Argumente gehen als Text in die Bindung; `null` bleibt `None`.
            let t = |k: &str| c.args.get(k).map(|v| PyWert::from(v.clone()));
            json!(ek::dba_methode_fuer(
                t("staat").as_ref(),
                t("einkunftsart").as_ref()
            )?)
        }
        other => panic!("unbekannte Funktion {other}"),
    })
}

// ---------------------------------------------------------------- Vergleich und Zählung

#[derive(Default, Clone)]
struct Zeile {
    python: u64,
    rust: u64,
    ok_gleich: u64,
    err_gleich: u64,
    nicht_leer: u64,
    abw: u64,
}

#[derive(Default)]
struct Bilanz {
    zeilen: BTreeMap<&'static str, Zeile>,
    abweichungen: Vec<String>,
    /// Faelle, in denen die absichtliche Abweichung Nr. 35 (Freistellung schlaegt die Abzugswahl) den Vergleich
    /// getragen hat; siehe [`nr35_python_ohne_wahl`].
    nr35: u64,
}

fn nicht_leer(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => a.iter().any(nicht_leer),
        Value::Object(o) => o.values().any(nicht_leer),
    }
}

/// Erhöht die erste Ganzzahl im Ergebnis um 1 (Störung der Negativkontrolle).
fn stoere(v: &mut Value) -> bool {
    match v {
        Value::Number(n) if n.is_i64() => {
            *v = json!(n.as_i64().unwrap() + 1);
            true
        }
        Value::Array(a) => a.iter_mut().any(stoere),
        Value::Object(o) => o.values_mut().any(stoere),
        _ => false,
    }
}

impl Bilanz {
    fn vergleiche(
        &mut self,
        name: &'static str,
        py: &Value,
        rust: Result<Value, BescheidFehler>,
        ort: &str,
        werte: bool,
    ) {
        let z = self.zeilen.entry(name).or_default();
        z.python += 1;
        z.rust += 1;
        let melde = |abw: &mut Vec<String>, was: String| abw.push(format!("{ort} {name}: {was}"));
        match (py.get("ok"), rust) {
            (Some(p), Ok(r)) => {
                if nicht_leer(p) {
                    z.nicht_leer += 1;
                }
                if *p == r {
                    z.ok_gleich += 1;
                } else {
                    z.abw += 1;
                    melde(
                        &mut self.abweichungen,
                        if werte {
                            format!("py={p} rust={r}")
                        } else {
                            "Wert weicht ab".into()
                        },
                    );
                }
            }
            (None, Err(e)) => {
                let py_klasse = py["err"].as_str().unwrap_or("?");
                match e.python_klasse() {
                    Some(k) if k != "CatalaError" && k != py_klasse => {
                        z.abw += 1;
                        melde(
                            &mut self.abweichungen,
                            format!("Fehlerklasse rust={k} py={py_klasse}"),
                        );
                    }
                    _ => z.err_gleich += 1,
                }
            }
            (Some(p), Err(e)) => {
                z.abw += 1;
                melde(
                    &mut self.abweichungen,
                    if werte {
                        format!("py ok={p}, rust Err={e}")
                    } else {
                        "py ok, rust Err".into()
                    },
                );
            }
            (None, Ok(r)) => {
                z.abw += 1;
                melde(
                    &mut self.abweichungen,
                    if werte {
                        format!("py Err={}, rust ok={r}", py["err"])
                    } else {
                        "py Err, rust ok".into()
                    },
                );
            }
        }
    }

    fn drucke(&self, titel: &str, faelle: usize) {
        eprintln!(
            "== {titel}: {faelle} Fälle × {} Funktionen ==",
            FUNKTIONEN.len()
        );
        eprintln!(
            "{:<30} {:>7} {:>7} {:>8} {:>8} {:>9} {:>5}",
            "funktion", "python", "rust", "ok=", "err=", "nicht-leer", "abw"
        );
        for (n, z) in &self.zeilen {
            eprintln!(
                "{n:<30} {:>7} {:>7} {:>8} {:>8} {:>9} {:>5}",
                z.python, z.rust, z.ok_gleich, z.err_gleich, z.nicht_leer, z.abw
            );
        }
        let gesamt: u64 = self.zeilen.values().map(|z| z.abw).sum();
        eprintln!("Abweichungen gesamt: {gesamt}");
        for a in self.abweichungen.iter().take(8) {
            eprintln!("  ! {a}");
        }
    }

    fn abweichungen(&self) -> u64 {
        self.zeilen.values().map(|z| z.abw).sum()
    }

    /// Die `nicht_leer`-Zaehler aller Zeilen als [`parity::pin::Gesehen`].
    fn gesehen(&self) -> parity::pin::Gesehen {
        self.zeilen
            .iter()
            .map(|(n, z)| (*n, z.nicht_leer))
            .collect()
    }

    /// Waechter gegen einen gruenen Lauf, der nichts belegt: eine Vergleichszeile, die nie
    /// einen Wert GESEHEN hat (`nicht-leer == 0`), kann nicht "0 Abweichungen" beweisen --
    /// gruen und leer sehen identisch aus.
    ///
    /// Ticket `parity-lauf-gruen-ohne-dass-die-zeile-rechnet`. Der Waechter greift JE BLOCK:
    /// eine Zeile, die nur in `golden_faelle` rechnet, deckt die Luecke in `reale_faelle` nicht.
    /// Gepinnte, heute nachweislich leere Zeilen stehen in der `LEER`-Liste des Blocks -- mit
    /// Grund und Korpus. Beide Richtungen sind streng (s. [`parity::pin::pruefe`]).
    fn wache_rechnet(&self, block: &str, liste: &[(&str, &str)]) {
        parity::pin::pruefe(block, parity::pin::KORPUS, liste, &self.gesehen());
    }
}

/// Das Feld der Abzugswahl (§ 34c Abs. 2 `EStG`), das Rust bei Freistellung nicht mehr liest (Abweichung Nr. 35).
const ABZUGSWAHL: &str = "dba_abzug_statt_anrechnung";

/// Derselbe Fall mit der Abzugswahl auf `false` (nur die Store-Form; die Fall-Vorlagen des Golden-Masters tragen das
/// Feld nicht). Die Events werden neu gehasht wie in [`event`]; ein spaeteres Event auf denselben Schluessel gewinnt, darum
/// werden alle umgeschrieben.
fn ohne_abzugswahl(fall: &Fall) -> Fall {
    let mut f = fall.clone();
    let events = f
        .store
        .as_mut()
        .and_then(|s| s.get_mut("events"))
        .and_then(Value::as_array_mut);
    for e in events
        .into_iter()
        .flatten()
        .filter(|e| e["feld_id"] == ABZUGSWAHL)
    {
        e["wert"] = json!(false);
        if let Some(o) = e.as_object_mut() {
            o.remove("event_id");
        }
        e["event_id"] = json!(EventId::von_json(e).to_string());
    }
    f
}

/// Abweichung Nr. 35 (`rust/fixtures/README.md`): bei Freistellung rechnet Rust keinen Abzug nach § 34c Abs. 2, Python
/// bucht ihn vor der Methodenpruefung (`bescheid_einkuenfte.py`, Zweig `dba_abzug_statt_anrechnung`). Liefert dann Pythons
/// Antwort auf denselben Fall OHNE die Wahl, und nur dann: Python und Rust muessen sich unterscheiden, Rust muss den
/// Freistellungs-Zweig genommen haben (`p32b_progressionseinkuenfte` nicht leer), und Rusts Antwort muss genau Pythons
/// Antwort ohne Wahl sein. Jede andere Abweichung bleibt eine.
fn nr35_python_ohne_wahl(
    fall: &Fall,
    py: &Value,
    rust: &Result<Value, BescheidFehler>,
) -> Option<Value> {
    let (Some(p), Ok(r)) = (py.get("ok"), rust) else {
        return None;
    };
    if p == r || r["p32b_progressionseinkuenfte"].is_null() {
        return None;
    }
    let ohne = frage(&ohne_abzugswahl(fall).request(&["shared_dba_sonstige"]))
        ["bescheid.shared_dba_sonstige"]
        .clone();
    (ohne.get("ok") == Some(r)).then_some(ohne)
}

/// Ein Fall gegen Python und Rust; `stoere_erste` verfälscht das erste Rust-Ergebnis (Negativkontrolle).
fn vergleiche_fall(b: &mut Bilanz, fall: &Fall, ort: &str, werte: bool, stoere_erste: bool) {
    let py = frage(&fall.request(FUNKTIONEN));
    let c = baue_ctx(fall);
    let mut gestoert = !stoere_erste;
    for n in FUNKTIONEN {
        let mut r = rust_run(&c, n);
        if !gestoert {
            if let Ok(v) = r.as_mut() {
                gestoert = stoere(v);
            }
        }
        let mut py_n = py[format!("bescheid.{n}")].clone();
        if *n == "shared_dba_sonstige" {
            if let Some(ohne) = nr35_python_ohne_wahl(fall, &py_n, &r) {
                py_n = ohne;
                b.nr35 += 1;
            }
        }
        b.vergleiche(n, &py_n, r, ort, werte);
    }
}

// ---------------------------------------------------------------- Konstanten

#[test]
fn konstanten_gleich() {
    if skip() {
        return;
    }
    let py = frage(&json!({"fn": "bescheid.konstanten"}));
    let liste = |a: &[&str]| json!(a);
    assert_eq!(py["kap_ertraege"], ek::KAP_ERTRAEGE);
    assert_eq!(py["kap_toepfe"], liste(&ek::KAP_TOEPFE));
    assert_eq!(py["kap_ertraege_partner"], ek::KAP_ERTRAEGE_PARTNER);
    assert_eq!(py["kap_toepfe_partner"], liste(&ek::KAP_TOEPFE_PARTNER));
    assert_eq!(py["euer_komponenten"], liste(&ek::EUER_KOMPONENTEN));
    assert_eq!(
        py["gewinn_quellen_mengen"],
        liste(&ek::GEWINN_QUELLEN_MENGEN)
    );
    assert_eq!(py["mitu_felder"], liste(&ek::MITU_FELDER));
    // DBA-Tabellen indirekt: jeder Python-Schlüssel muss dieselbe Methode liefern.
    let mut n = 0;
    for (staat, iso) in py["dba_staat_iso"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p[0].as_str().unwrap(), p[1].as_str().unwrap()))
    {
        let methode = py["dba_methode"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m[0] == iso)
            .map_or("anrechnung", |m| m[1].as_str().unwrap());
        assert_eq!(
            ek::dba_methode_fuer(Some(&PyWert::Text(staat.to_owned())), None).unwrap(),
            methode,
            "staat {staat}"
        );
        n += 1;
    }
    for p in py["dba_methode_art"].as_array().unwrap() {
        let (staat, art, m) = (
            p[0][0].as_str().unwrap(),
            p[0][1].as_str().unwrap(),
            p[1].as_str().unwrap(),
        );
        assert_eq!(
            ek::dba_methode_fuer(
                Some(&PyWert::Text(staat.to_owned())),
                Some(&PyWert::Text(art.to_owned()))
            )
            .unwrap(),
            m
        );
        n += 1;
    }
    eprintln!("konstanten_gleich: 7 Listen + {n} DBA-Einträge, 0 Abweichungen");
}

// ---------------------------------------------------------------- reale Fälle

fn args_real(f_zusammen: bool, variante: usize) -> Value {
    let (gde, ausserg, zusammen) = if variante == 0 {
        (0, 0, f_zusammen)
    } else {
        (60_000, 2_500, !f_zusammen)
    };
    json!({"gde": gde, "ausserg": ausserg, "veranlagung": if f_zusammen { "zusammen" } else { "einzel" },
        "zusammen": zusammen, "slots": {"oepnv_kosten_jahr": 123_456}, "messbetrag_a": 1500, "hebesatz_a": 400,
        "messbetrag_b": 700, "hebesatz_b": 380, "zaehler_a": 21_000, "gde_p10d": gde,
        "g": {"veranlagungszeitraum": 2025, "veranlagung": if f_zusammen { "zusammen" } else { "einzel" },
            "einkuenfte_nichtselbststaendig": 60_000 - gde / 2, "sonderausgaben": 1_000}})
}

#[test]
fn reale_faelle() {
    if skip() {
        return;
    }
    // EIN gemeinsamer Leser, der ZAEHLT: gefunden, gelesen, uebersprungen. Die frueher hier
    // gestandene stille `continue` liess einen Lauf auf 44 von 192 Dateien gruen melden und
    // dabei "192 Dateien" behaupten -- gemessen am 2026-10-01.
    let korpus = parity::korpus::Korpus::lies(&faelle_verzeichnis());
    korpus.pflicht(&faelle_verzeichnis(), "reale_faelle");
    let mut b = Bilanz::default();
    let (mut faelle, mut kein_store, mut vz_ersatz) = (0usize, 0usize, 0usize);
    let mut stoerung_offen = stoerung_an();
    for (pfad, roh) in &korpus.gelesen {
        let roh = roh.clone();
        let _ = pfad;
        if roh.get("events").is_none() {
            kein_store += 1;
            continue;
        }
        let vz = roh["veranlagungszeitraum"]
            .as_u64()
            .filter(|v| (2024..=2026).contains(v))
            .unwrap_or_else(|| {
                vz_ersatz += 1;
                2025
            }) as u16;
        let datei: StoreDatei =
            serde_json::from_value(roh.clone()).expect("Store-Datei deserialisiert");
        let (felder, _) = Store::aus_datei(datei).materialisiere(None).unwrap();
        let f_zusammen = felder
            .get("veranlagung")
            .is_some_and(|f| f.wert == PyWert::Text("zusammen".to_owned()));
        for nur in [true, false] {
            for variante in 0..2 {
                let fall = Fall {
                    store: Some(roh.clone()),
                    felder: None,
                    vz,
                    nur,
                    store_uebergeben: variante == 0,
                    bindung_uebergeben: true,
                    args: args_real(f_zusammen, variante),
                };
                vergleiche_fall(
                    &mut b,
                    &fall,
                    "real",
                    false,
                    std::mem::take(&mut stoerung_offen),
                );
                faelle += 1;
            }
        }
    }
    b.drucke("reale_faelle", faelle);
    b.wache_rechnet("reale_faelle", LEER_REALE);
    eprintln!(
        "reale_faelle: {} von {} Dateien GELESEN, {kein_store} ohne Store übersprungen, \
         {vz_ersatz} mit VZ außerhalb 2024–2026 (Ersatz 2025), {} unlesbar",
        korpus.gelesen_zahl(),
        korpus.gefunden.len(),
        korpus.uebersprungen.len()
    );
    assert!(faelle > 0);
    assert_eq!(
        b.abweichungen(),
        0,
        "Abweichungen (Anzahl s. o., keine Werte ausgegeben)"
    );
}

// ---------------------------------------------------------------- Golden

#[test]
fn golden_faelle() {
    if skip() {
        return;
    }
    let text = std::fs::read_to_string(repo_root().join("rust/fixtures/golden_cases.json"))
        .expect("golden_cases.json");
    let faelle: Vec<Value> = serde_json::from_str(&text).unwrap();
    let herkunft = json!({"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"});
    let mut b = Bilanz::default();
    let mut n = 0usize;
    for fall in &faelle {
        let sv = fall["sachverhalt"].as_object().unwrap();
        let vz = sv
            .get("veranlagungszeitraum")
            .and_then(Value::as_u64)
            .filter(|v| (2024..=2026).contains(v))
            .unwrap_or(2025) as u16;
        for faktor in [1_i64, 100] {
            let felder: serde_json::Map<String, Value> = sv
                .iter()
                .filter(|(_, v)| v.is_number() || v.is_boolean() || v.is_string())
                .map(|(k, v)| {
                    let wert = match (v.as_i64(), k.as_str()) {
                        (Some(i), k) if k != "veranlagungszeitraum" && k != "anzahl_kinder" => {
                            json!(i * faktor)
                        }
                        _ => v.clone(),
                    };
                    (
                        k.clone(),
                        json!({"wert": wert, "zustand": "bestaetigt", "herkunft": herkunft}),
                    )
                })
                .collect();
            let zusammen = sv.get("veranlagung").is_some_and(|v| v == "zusammen");
            let f = Fall {
                store: None,
                felder: Some(Value::Object(felder)),
                vz,
                nur: true,
                store_uebergeben: true,
                bindung_uebergeben: true,
                args: args_real(zusammen, usize::from(faktor == 100)),
            };
            vergleiche_fall(&mut b, &f, "golden", true, false);
            n += 1;
        }
    }
    b.drucke("golden_faelle", n);
    b.wache_rechnet("golden_faelle", LEER_GOLDEN);
    assert_eq!(n, faelle.len() * 2);
    assert_eq!(b.abweichungen(), 0);
}

// ---------------------------------------------------------------- Generator

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn byte(&mut self) -> u64 {
        let b = self
            .bytes
            .get(self.pos % self.bytes.len().max(1))
            .copied()
            .unwrap_or(0);
        self.pos += 1;
        u64::from(b)
    }
    fn range(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        ((self.byte() << 16) | (self.byte() << 8) | self.byte()) % n
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.range(100) < pct
    }
    fn waehle<'b, T>(&mut self, xs: &'b [T]) -> &'b T {
        &xs[self.range(xs.len() as u64) as usize]
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Cent,
    Bool,
    Gdb,
    Jahr,
    Anz,
    Kwp,
    Hebesatz,
    Idnr,
    Veranl,
    DbaStaat,
    DbaArt,
    DbaMethode,
    Betriebsart,
}
use Kind::{Anz, Bool, Cent, DbaArt, DbaMethode, DbaStaat, Gdb, Hebesatz, Idnr, Jahr, Kwp, Veranl};

/// `(feld_id, Art, Instanzgruppe?)`. Instanzfelder erscheinen als `x`, `x__2`, `x__3`.
const FELDER: &[(&str, Kind, bool)] = &[
    ("veranlagung", Veranl, false),
    ("geburtsjahr", Jahr, false),
    ("dauernd_berufsunfaehig", Bool, false),
    ("ermaessigung_einmal_genutzt", Bool, false),
    // kind
    ("kind_idnr", Idnr, true),
    ("kind_kv", Cent, true),
    ("kind_pv", Cent, true),
    ("kind_unter_14_haushaltszugehoerig", Bool, true),
    ("kinderbetreuungskosten", Cent, true),
    ("schulgeld", Cent, true),
    ("kind_behinderten_pb_antrag", Bool, true),
    ("kind_pb_nicht_selbst_genutzt", Bool, true),
    ("kind_grad_der_behinderung", Gdb, true),
    ("kind_hilflos_blind_taubblind", Bool, true),
    ("kind_hinterbliebenen_uebertragung", Bool, true),
    // § 35a
    ("hh_minijob_betrag", Cent, true),
    ("hh_dienstleistung_betrag", Cent, true),
    ("hh_handwerker_betrag", Cent, true),
    ("hh_minijob_aufwendungen", Cent, false),
    ("hh_dienstleistungen", Cent, false),
    ("hh_handwerker_arbeitskosten", Cent, false),
    ("hh_in_eu_ewr", Bool, false),
    ("hh_rechnung_unbar", Bool, false),
    ("hh_handwerker_keine_foerderung", Bool, false),
    ("p35a_mitveranlagung", Bool, false),
    // § 35c
    ("p35c_keine_doppelfoerderung", Bool, false),
    ("p35c_ist_uebernaechstes_foerderjahr", Bool, false),
    ("p35c_sanierungsaufwendungen", Cent, false),
    ("p35c_energieberater_aufwendungen", Cent, false),
    // Sonderausgaben
    ("spenden_betrag", Cent, false),
    ("kist_gezahlt", Cent, false),
    ("kist_erstattet", Cent, false),
    ("basis_kv", Cent, false),
    ("basis_pv", Cent, false),
    ("vorsorge_arbeitslosenversicherung", Cent, false),
    ("vorsorge_erwerbsunfaehigkeit", Cent, false),
    ("vorsorge_unfall_haftpflicht", Cent, false),
    ("vorsorge_rv_alt_mit_ueberschuss", Cent, false),
    ("vorsorge_rv_alt_ohne_ueberschuss", Cent, false),
    ("mit_anspruch_auf_zuschuss", Bool, false),
    ("basis_kv_partner", Cent, false),
    ("basis_pv_partner", Cent, false),
    ("vorsorge_arbeitslosenversicherung_partner", Cent, false),
    ("vorsorge_erwerbsunfaehigkeit_partner", Cent, false),
    ("vorsorge_unfall_haftpflicht_partner", Cent, false),
    ("vorsorge_rv_alt_mit_ueberschuss_partner", Cent, false),
    ("vorsorge_rv_alt_ohne_ueberschuss_partner", Cent, false),
    ("mit_anspruch_auf_zuschuss_partner", Bool, false),
    ("realsplitting_zustimmung", Bool, false),
    ("realsplitting_unterhaltsleistungen", Cent, false),
    ("realsplitting_empfaenger_kv_pv", Cent, false),
    ("realsplitting_empfaenger_kv_krankengeld", Cent, false),
    ("berufsausbildung_aufwendungen", Cent, false),
    // agB
    ("agb_aufwendungen", Cent, false),
    ("agb_zwangslaeufig", Bool, false),
    ("agb_notwendig_angemessen", Bool, false),
    ("behinderungsbedingte_aufwendungen", Cent, false),
    ("behinderungsbedingte_aufwendungen_partner", Cent, false),
    (
        "behinderungsbedingte_aufwendungen_wahlrecht_pb",
        Bool,
        false,
    ),
    (
        "behinderungsbedingte_aufwendungen_wahlrecht_pb_partner",
        Bool,
        false,
    ),
    ("rentner_grad_der_behinderung", Gdb, false),
    ("rentner_grad_der_behinderung_partner", Gdb, false),
    ("rentner_hilflos_blind_taubblind", Bool, false),
    ("rentner_hilflos_blind_taubblind_partner", Bool, false),
    ("fahrtkosten_pausch_gdb80_oder_70g", Bool, false),
    ("fahrtkosten_pausch_ag_bl_tbl_h", Bool, false),
    ("fam_anzahl_kinder", Anz, false),
    // DBA / § 33a / § 10d
    ("p33a_unterhalt_aufwendungen", Cent, false),
    ("p33a_unterhalt_kv_pv", Cent, false),
    ("p33a_andere_einkuenfte_bezuege", Cent, false),
    ("p33a_ausbildung_anzahl_kinder", Anz, false),
    ("verlustvortrag_bestand", Cent, false),
    ("dba_gezahlte_auslaendische_steuer", Cent, false),
    ("dba_auslaendische_einkuenfte", Cent, false),
    ("dba_staat", DbaStaat, false),
    ("dba_einkunftsart", DbaArt, false),
    ("dba_methode", DbaMethode, false),
    ("dba_abzug_statt_anrechnung", Bool, false),
    // Gewinn
    ("einkuenfte_gewinn", Cent, false),
    ("betriebseinnahmen", Cent, false),
    ("sonstige_betriebsausgaben", Cent, false),
    ("afa_jahresbetrag", Cent, false),
    ("gewinnanteil", Cent, false),
    ("verguetung_taetigkeit", Cent, false),
    ("verguetung_darlehen", Cent, false),
    ("verguetung_ueberlassung", Cent, false),
    ("pv_einnahmen", Cent, false),
    ("pv_bruttoleistung_kwp", Kwp, false),
    ("pv_anzahl_einheiten", Anz, false),
    ("pv_auf_gebaeude", Bool, false),
    ("gwg_anschaffungskosten_netto", Cent, true),
    ("gwg_bewegliches_selbstaendig_nutzbar", Bool, true),
    ("gwg_netto_ohne_vorsteuer", Bool, true),
    ("gwg_verzeichnis_ab_250", Bool, true),
    // Partner
    ("einkuenfte_gewinn_partner", Cent, false),
    ("gewinnanteil_partner", Cent, false),
    ("verguetung_taetigkeit_partner", Cent, false),
    ("verguetung_darlehen_partner", Cent, false),
    ("verguetung_ueberlassung_partner", Cent, false),
    ("rentner_veraeusserungsgewinn_partner", Cent, false),
    ("rentner_alter_55_oder_berufsunfaehig_partner", Bool, false),
    ("rentner_freibetrag_erstmalig_partner", Bool, false),
    ("gewst_messbetrag_partner", Cent, false),
    ("gewst_hebesatz_partner", Hebesatz, false),
    ("gewinn_betriebsart_partner", Kind::Betriebsart, false),
    // Kapital
    ("kap_kapitalertraege", Cent, false),
    ("kap_gewinn_aktien", Cent, false),
    ("kap_verlust_aktien", Cent, false),
    ("kap_gewinn_sonstige", Cent, false),
    ("kap_verlust_sonstige", Cent, false),
    ("kap_kapitalertraege_partner", Cent, false),
    ("kap_gewinn_aktien_partner", Cent, false),
    ("kap_verlust_aktien_partner", Cent, false),
    ("kap_gewinn_sonstige_partner", Cent, false),
    ("kap_verlust_sonstige_partner", Cent, false),
    // § 23
    ("p23_veraeusserungspreis", Cent, true),
    ("p23_anschaffung_herstellungskosten", Cent, true),
    ("p23_werbungskosten", Cent, true),
];

const GRENZEN: &[i64] = &[
    0, 1, 99, 100, 101, 25_000, 25_001, 80_000, 80_001, 99_900, 100_000, 250_000, 1_000_000,
    5_000_000, 20_000_000,
];
const STAATEN: &[&str] = &[
    "Österreich",
    "oesterreich",
    "USA",
    "polen",
    "Polen",
    "Türkei",
    "schweiz",
    "Italien",
    "sonstiger_staat",
    "  Frankreich ",
    "ÖSTERREICH",
    "",
];
const ARTEN: &[&str] = &[
    "dividenden",
    "Unternehmensgewinne",
    "zinsen",
    "ruhegehaelter",
    " Zinsen",
    "unbekannt",
    "",
];

fn wert(c: &mut Cursor, k: Kind, wahr_pct: u64) -> Value {
    match k {
        Cent => match c.range(24) {
            0 => Value::Null,
            1 => json!(true),
            2 => json!("12"),
            3 => json!(c.range(2_000_000) as i64 as f64 + 0.5),
            4 => json!(-(c.range(600_000) as i64)),
            5 | 6 => json!(*c.waehle(GRENZEN)),
            7 => json!(-150),
            8 => json!(-*c.waehle(GRENZEN)),
            9 => json!(c.range(30_000_000) as i64),
            _ => json!(c.range(3_000_000) as i64),
        },
        Bool => match c.range(100) {
            0..=3 => Value::Null,
            4 => json!("ja"),
            5 => json!(1),
            _ => json!(c.chance(wahr_pct)),
        },
        Gdb => match c.range(8) {
            0 => json!(0),
            1 => Value::Null,
            2 => json!(19.5),
            _ => json!(20 + c.range(90) as i64),
        },
        Jahr => match c.range(8) {
            0 => Value::Null,
            1 => json!(0),
            2 => json!(-5),
            _ => json!(1930 + c.range(90) as i64),
        },
        Anz => json!(c.range(6) as i64 - i64::from(c.chance(5))),
        Kwp => json!(c.range(150) as i64 - i64::from(c.chance(5))),
        Hebesatz => json!(*c.waehle(&[0, 200, 380, 400, 490, 900, -100])),
        Idnr => match c.range(6) {
            0 => json!("kurz"),
            1 => Value::Null,
            2 => json!(12_345_678_901_i64),
            3 => json!(""),
            _ => json!(format!("{:011}", c.range(99_999_999_999))),
        },
        Veranl => json!(*c.waehle(&["einzel", "zusammen", "zusammen", "getrennt", "", "Zusammen"])),
        DbaStaat => match c.range(12) {
            0 => json!(5),
            1 => json!([1]),
            2 => Value::Null,
            3 => json!(true),
            _ => json!(*c.waehle(STAATEN)),
        },
        DbaArt => match c.range(10) {
            0 => json!(7),
            1 => Value::Null,
            _ => json!(*c.waehle(ARTEN)),
        },
        DbaMethode => json!(*c.waehle(&["dba_freistellung", "dba_anrechnung", "", "x"])),
        Kind::Betriebsart => {
            json!(*c.waehle(&["gewerbe", "gewerbe", "selbstaendig", "land_forst", ""]))
        }
    }
}

fn event(i: usize, fid: &str, wert: &Value, bestaetigt: bool) -> Value {
    let mut e = json!({
        "ts": format!("2026-01-01T00:{:02}:{:02}+00:00", i / 60, i % 60), "feld_id": fid, "wert": wert,
        "zustand": if bestaetigt { "bestaetigt" } else { "vorlaeufig" },
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": null, "signal_2": if bestaetigt { json!("ok") } else { Value::Null }},
        "ersetzt": null,
    });
    e["event_id"] = json!(EventId::von_json(&e).to_string());
    e
}

/// Feld-Präfixe je Fokusgruppe: im Fokus liegt die Feld-Dichte bei 95 %.
const FOKUS: &[&[&str]] = &[
    &[],
    &[
        "kind_",
        "kinderbetreuungskosten",
        "schulgeld",
        "veranlagung",
    ],
    &["hh_", "p35a", "p35c"],
    &[
        "gwg_",
        "betriebseinnahmen",
        "sonstige_betriebsausgaben",
        "afa_",
        "pv_",
        "einkuenfte_gewinn",
    ],
    &["p23_"],
    &["dba_", "p33a", "verlustvortrag", "veranlagung"],
    &[
        "agb_",
        "behinderungsbedingte",
        "rentner_",
        "fahrtkosten",
        "fam_",
        "veranlagung",
    ],
    &[
        "gewinnanteil",
        "verguetung_",
        "gewst_",
        "gewinn_betriebsart",
        "einkuenfte_gewinn_partner",
        "veranlagung",
    ],
    &["kap_", "veranlagung"],
    &[
        "basis_",
        "vorsorge_",
        "spenden",
        "kist_",
        "realsplitting",
        "berufsausbildung",
        "mit_anspruch",
        "veranlagung",
    ],
];

fn generiere_fall(c: &mut Cursor) -> Fall {
    let dichte = *c.waehle(&[15, 35, 60, 90]);
    let bestaetigt_pct = *c.waehle(&[50, 80, 100]);
    let wahr_pct = *c.waehle(&[40, 85, 95]);
    let fokus = FOKUS[c.range(FOKUS.len() as u64) as usize];
    let mut events: Vec<(String, Value, bool)> = Vec::new();
    // Zustand je Instanz einheitlich (die Instanz ist nur bestaetigt, wenn ALLE ihre Felder es sind);
    // einzelne Felder weichen mit 4 % ab und machen die Instanz vorlaeufig.
    let inst_zustand = [
        c.chance(bestaetigt_pct),
        c.chance(bestaetigt_pct),
        c.chance(bestaetigt_pct),
    ];
    for (fid, kind, gruppe) in FELDER {
        let dichte_hier = if fokus.iter().any(|p| fid.contains(p)) {
            95
        } else {
            dichte
        };
        if !c.chance(dichte_hier) {
            continue;
        }
        let instanzen: &[u64] = if *gruppe {
            &[1, 2, 3][..=(c.range(3) as usize)]
        } else {
            &[1]
        };
        for n in instanzen {
            let id = match n {
                1 if *gruppe && c.chance(5) => format!("{fid}__1"),
                1 => (*fid).to_owned(),
                n => format!("{fid}__{n}"),
            };
            let z = if *gruppe {
                inst_zustand[(*n - 1) as usize] != c.chance(4)
            } else {
                c.chance(bestaetigt_pct)
            };
            events.push((id, wert(c, *kind, wahr_pct), z));
        }
    }
    // Überschreiben: ein späteres Event auf denselben Schlüssel gewinnt (kein `ersetzt`).
    if !events.is_empty() && c.chance(15) {
        let (fid, _, _) = events[c.range(events.len() as u64) as usize].clone();
        events.push((fid, wert(c, Cent, 50), c.chance(50)));
    }
    let evs: Vec<Value> = events
        .iter()
        .enumerate()
        .map(|(i, (f, w, b))| event(i, f, w, *b))
        .collect();
    let vz = *c.waehle(&[2024_u16, 2025, 2026]);
    let store = json!({"version": 1, "veranlagungszeitraum": vz, "events": evs});
    let zusammen = c.chance(50);
    let veranl = if zusammen {
        "zusammen"
    } else {
        *c.waehle(&["einzel", "getrennt"])
    };
    let euro = |c: &mut Cursor, max: u64| -> i64 {
        c.range(max) as i64 * if c.chance(6) { -1 } else { 1 }
    };
    let slot = match c.range(8) {
        0 => Value::Null,
        1 => json!("12"),
        2 => json!(1234.9),
        3 => json!(true),
        4 => json!(-150),
        _ => json!(euro(c, 900_000)),
    };
    let slots = if c.chance(10) {
        json!({})
    } else {
        json!({"oepnv_kosten_jahr": slot})
    };
    let hebe = |c: &mut Cursor| -> i64 { *c.waehle(&[0, 200, 380, 400, 490, 900, -100]) };
    let args = json!({
        "gde": euro(c, 200_000), "ausserg": euro(c, 8_000), "veranlagung": veranl, "zusammen": c.chance(50),
        "slots": slots,
        "messbetrag_a": euro(c, 20_000), "hebesatz_a": hebe(c), "messbetrag_b": euro(c, 20_000), "hebesatz_b": hebe(c),
        "zaehler_a": euro(c, 300_000), "gde_p10d": euro(c, 200_000),
        "g": {"veranlagungszeitraum": vz, "veranlagung": veranl,
            "einkuenfte_nichtselbststaendig": euro(c, 200_000), "einkuenfte_gewinn": euro(c, 80_000),
            "einkuenfte_sonstige": euro(c, 5_000), "einkuenfte_kapitalvermoegen": euro(c, 5_000),
            "einkuenfte_vermietung": euro(c, 5_000), "sonderausgaben": euro(c, 20_000),
            "aussergewoehnliche_belastungen": euro(c, 5_000), "steuerermaessigungen": euro(c, 2_000),
            "anzurechnende_auslaendische_steuern": euro(c, 2_000)},
        "staat": null, "einkunftsart": null,
    });
    Fall {
        store: Some(store),
        felder: None,
        vz,
        nur: c.chance(60),
        store_uebergeben: c.chance(85),
        bindung_uebergeben: c.chance(95),
        args,
    }
}

fn runner(cases: u32) -> TestRunner {
    let cfg = Config {
        cases,
        failure_persistence: None,
        max_shrink_iters: 0,
        ..Config::default()
    };
    TestRunner::new_with_rng(cfg, TestRng::deterministic_rng(RngAlgorithm::ChaCha))
}

#[test]
fn generierte_faelle() {
    if skip() {
        return;
    }
    let b = std::cell::RefCell::new(Bilanz::default());
    let n = std::cell::Cell::new(0usize);
    let n_faelle = parity::fallzahl::holen_u32("bescheid_blatt_paritaet generierte_faelle", 1200);
    let ergebnis = runner(n_faelle).run(&prop::collection::vec(any::<u8>(), 1024..4096), |bytes| {
        let mut c = Cursor {
            bytes: &bytes,
            pos: 0,
        };
        let fall = generiere_fall(&mut c);
        n.set(n.get() + 1);
        let vorher = b.borrow().abweichungen();
        vergleiche_fall(
            &mut b.borrow_mut(),
            &fall,
            &format!("gen#{}", n.get()),
            true,
            false,
        );
        prop_assert_eq!(
            b.borrow().abweichungen(),
            vorher,
            "{:?}",
            b.borrow().abweichungen.last()
        );
        Ok(())
    });
    b.borrow().drucke("generierte_faelle", n.get());
    // Der Zaehler traegt seinen Wert: ohne diese Zeile druckte der Lauf "Abweichungen
    // gesamt: N" und meldete bei N>0 dasselbe wie bei N=0. Der Delta-Check im Closure
    // darueber faengt nur eine AENDERUNG, keine konstante Verschiebung.
    assert_eq!(
        b.borrow().abweichungen(),
        0,
        "generierte_faelle: Abweichungen (Anzahl s. o.)"
    );
    // Pin und Untergrenze gelten beim Standard; sonst muss der Lauf mindestens die verlangte Zahl
    // rechnen.
    let wachen = parity::fallzahl::wache_gilt(
        "bescheid_blatt_paritaet generierte_faelle",
        n_faelle as usize,
        1200,
    );
    if wachen {
        b.borrow().wache_rechnet("generierte_faelle", &[]);
        // Wachposten gegen eine tote Maskierung (Abweichung Nr. 35): trifft kein Fall die Freistellung mit gewaehltem
        // Abzug, belegt "0 Abweichungen" den Zweig nicht mehr (Generator geaendert, oder Rust bucht den Abzug wieder).
        assert!(
            b.borrow().nr35 > 0,
            "generierte_faelle: kein Fall traegt Abweichung Nr. 35 (Freistellung gewinnt gegen die Abzugswahl)"
        );
    }
    eprintln!(
        "generierte_faelle: Abweichung Nr. 35 trug den Vergleich in {} Faellen",
        b.borrow().nr35
    );
    ergebnis.unwrap();
    assert!(n.get() >= if wachen { 1000 } else { n_faelle as usize });
}

#[test]
fn dba_methode_generiert() {
    if skip() {
        return;
    }
    let mut b = Bilanz::default();
    let n = std::cell::Cell::new(0usize);
    let strategie = prop::collection::vec(any::<u8>(), 64..128);
    let b_ref = std::cell::RefCell::new(&mut b);
    let n_faelle =
        parity::fallzahl::holen_u32("bescheid_blatt_paritaet dba_methode_generiert", 1500);
    runner(n_faelle)
        .run(&strategie, |bytes| {
            let mut c = Cursor {
                bytes: &bytes,
                pos: 0,
            };
            let zufall_text = |c: &mut Cursor| -> Value {
                match c.range(8) {
                    0 => json!(c.range(5)),
                    1 => Value::Null,
                    2 => json!([c.range(2)]),
                    3 => json!(format!(
                        "{}{}",
                        c.waehle(STAATEN),
                        if c.chance(30) { "\u{1c} " } else { "" }
                    )),
                    4 => json!(c
                        .waehle(&["ÖSTERREICH", "USA ", "\tpolen", "İSTANBUL", "ǅ"])
                        .to_string()),
                    5 => json!(c.waehle(ARTEN).to_uppercase()),
                    _ => {
                        let pool = if c.chance(50) { STAATEN } else { ARTEN };
                        json!(*c.waehle(pool))
                    }
                }
            };
            let (staat, art) = (zufall_text(&mut c), zufall_text(&mut c));
            let fall = Fall {
                store: None,
                felder: None,
                vz: 2025,
                nur: true,
                store_uebergeben: true,
                bindung_uebergeben: true,
                args: json!({"staat": staat, "einkunftsart": art}),
            };
            let py = frage(&fall.request(&["dba_methode_fuer"]));
            let ctx = baue_ctx(&fall);
            n.set(n.get() + 1);
            let vorher = b_ref.borrow().abweichungen();
            b_ref.borrow_mut().vergleiche(
                "dba_methode_fuer",
                &py["bescheid.dba_methode_fuer"],
                rust_run(&ctx, "dba_methode_fuer"),
                "dba",
                true,
            );
            prop_assert_eq!(
                b_ref.borrow().abweichungen(),
                vorher,
                "{:?}",
                b_ref.borrow().abweichungen.last()
            );
            Ok(())
        })
        .unwrap();
    b.drucke("dba_methode_generiert", n.get());
    assert_eq!(
        b.abweichungen(),
        0,
        "dba_methode_generiert: Abweichungen (Anzahl s. o.)"
    );
    let wachen = parity::fallzahl::wache_gilt(
        "bescheid_blatt_paritaet dba_methode_generiert",
        n_faelle as usize,
        1500,
    );
    if wachen {
        b.wache_rechnet("dba_methode_generiert", &[]);
    }
    assert!(n.get() >= if wachen { 1000 } else { n_faelle as usize });
    let z = &b.zeilen["dba_methode_fuer"];
    if wachen {
        assert!(
            z.ok_gleich > 0 && z.err_gleich > 0,
            "beide Pfade (ok und Fehler) müssen vorkommen"
        );
    }
}

// ---------------------------------------------------------------- Negativkontrolle

/// Ein Rust-Ergebnis um 1 gestört → GENAU eine Abweichung. Ohne diesen Test könnte `vergleiche`
/// kaputt sein (immer 0) und die Läufe oben trotzdem grün.
#[test]
fn negativkontrolle_erkennt_genau_eine_abweichung() {
    if skip() {
        return;
    }
    let f = |wert: Value| json!({"wert": wert, "zustand": "bestaetigt", "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}});
    let fall = Fall {
        store: None,
        // Nicht-leere Ergebnisse: Alter 65 → abs3 = true; EÜR-Gewinn; Kapital; GdB-PB.
        felder: Some(
            json!({"geburtsjahr": f(json!(1960)), "betriebseinnahmen": f(json!(5_000_000)),
            "sonstige_betriebsausgaben": f(json!(1_000_000)), "kap_kapitalertraege": f(json!(300_000)),
            "agb_aufwendungen": f(json!(400_000))}),
        ),
        vz: 2025,
        nur: true,
        store_uebergeben: true,
        bindung_uebergeben: true,
        args: args_real(false, 1),
    };
    let mut sauber = Bilanz::default();
    vergleiche_fall(&mut sauber, &fall, "neg", true, false);
    let mut gestoert = Bilanz::default();
    vergleiche_fall(&mut gestoert, &fall, "neg", true, true);
    let nicht_leer_zahl: u64 = sauber.zeilen.values().map(|z| z.nicht_leer).sum();
    gestoert.drucke("negativkontrolle (gestört)", 1);
    eprintln!("negativkontrolle: ungestört {} Abweichungen, gestört {} (erwartet 0 und 1), nicht-leere Ergebnisse: {nicht_leer_zahl}", sauber.abweichungen(), gestoert.abweichungen());
    assert_eq!(sauber.abweichungen(), 0);
    assert_eq!(
        gestoert.abweichungen(),
        1,
        "die Störung muss GENAU eine Abweichung erzeugen"
    );
    assert!(
        nicht_leer_zahl >= 4,
        "der Kontrollfall muss nicht-leere Ergebnisse tragen"
    );
}
