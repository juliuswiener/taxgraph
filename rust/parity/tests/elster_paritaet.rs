//! Paritaet `rust/elster` gegen die Python-Referenz (`est_mapping.py`, `elster_xml.py`,
//! `xsd_verify.py`, `checkest_gate.py`) ueber `tools/parity/elster_oracle.py` — EIN Orakel-Prozess
//! fuer das ganze Test-Binary.
//!
//! - `abzugs_kz_mengengleich`: Python-`_ABZUGS_KZ` und Rust-`ABZUGS_KZ` sind als Menge gleich.
//! - `kz_format_sweep`: `_cent_nach_kz` dicht um 0 und duenn darueber hinaus, jedes Kz-Format,
//!   die Abzugs-Kz aus beiden Listen; `_kz_wert`, `_jahr_aus_kz_wert`, `parse_instanz` ueber
//!   systematische Eingaben.
//! - `schema_und_werkzeug`: `kz_pfade`/`pflicht_kinder`/`_resolve_kz_meta` (2024, 2025),
//!   `klassifiziere_rc`, `xsd_verify.pruefe_bindung` (Bindung + Ernte).
//! - `reale_faelle`: jede Fall-Datei unter `faelle_verzeichnis()` — deklariere, zuruecklesen,
//!   instanzen (alle Gruppen), XML (ohne/mit Vorsatz), je einmal mit allen Feldern und einmal
//!   nur mit den bestaetigten. Jedes Rust-XML zusaetzlich gegen das XSD (xmllint).
//! - `generierte_stores`: 1000 proptest-Stores ueber `Store::append`, dieselben Vergleiche.
//! - `checkest_stichprobe`: `ERiC` (nur `ERIC_VALIDIERE`) auf beiden Seiten, rc-Vergleich.
//! - `negativkontrolle`: ein gestoertes Ergebnis muss als Abweichung erkannt werden.
//!
//! SICHERHEIT: reale Fall-Dateien sind echte Steuerdaten — nur lokal lesen, nie kopieren; in die
//! Ausgabe gehen nur Zaehlwerte und JSON-Pfade, nie Werte. Die Hersteller-ID kommt aus der
//! Umgebung bzw. `.env` und geht nur ueber die Pipe ans Orakel, nie in Ausgabe oder Datei.
//!
//!   `PARITY`=1 `cargo` test -p parity --test `elster_paritaet` -- --nocapture
#![allow(
    clippy::too_many_lines,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Mutex, MutexGuard, OnceLock};

use bindung::Bindung;
use domain::{
    Achsenwert, Feldtyp, Feldzustand, Herkunft, PruefTiefe, PyWert, Schreiber, Signal2, Zustand,
};
use elster::{Felder, XmlOptionen};
use parity::Oracle;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, TestRng, TestRunner};
use serde_json::{json, Value};
use store::{BindungNachschlag, NeuesEvent, Store, StoreDatei};

const HID_TEST: &str = "74931";
const GRUPPEN: &[&str] = &[
    "gwg",
    "hh_dienstleistung",
    "hh_handwerker",
    "hh_minijob",
    "kind",
    "p23_veraeusserung",
    "rente",
    "vv_objekt",
];

fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn skip_ohne_parity_env() -> bool {
    std::env::var("PARITY").as_deref() != Ok("1")
}

fn home() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
}

/// s. `store_paritaet.rs::faelle_verzeichnis` (Datei-lokales Idiom, bewusst dupliziert).
fn faelle_verzeichnis() -> std::path::PathBuf {
    let eigen = std::env::var("TAXGRAPH_DATEN").unwrap_or_default();
    if !eigen.trim().is_empty() {
        let e = eigen.trim();
        return e
            .strip_prefix("~/")
            .map_or_else(|| std::path::PathBuf::from(e), |r| home().join(r))
            .join("faelle");
    }
    let xdg = std::env::var("XDG_DATA_HOME").unwrap_or_default();
    let basis = if xdg.trim().is_empty() {
        home().join(".local/share")
    } else {
        xdg.trim().into()
    };
    basis.join("taxgraph").join("faelle")
}

fn walk_json(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for e in read.flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(walk_json(&p));
        } else if p.extension().is_some_and(|x| x == "json") {
            out.push(p);
        }
    }
    out.sort();
    out
}

fn bindungen() -> &'static [Bindung] {
    static CELL: OnceLock<Vec<Bindung>> = OnceLock::new();
    CELL.get_or_init(|| {
        let reg =
            bindung::lade_registry(&repo_root().join("produkt/bindung")).expect("bindung laedt");
        reg.dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect()
    })
}

fn index() -> &'static HashMap<String, &'static Bindung> {
    static CELL: OnceLock<HashMap<String, &'static Bindung>> = OnceLock::new();
    CELL.get_or_init(|| store::baue_nachschlag(bindungen()))
}

fn oracle() -> MutexGuard<'static, Oracle> {
    static CELL: OnceLock<Mutex<Oracle>> = OnceLock::new();
    CELL.get_or_init(|| Mutex::new(Oracle::spawn(&repo_root()).expect("oracle.py startet")))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Ein Orakel-Aufruf; die aeussere Huelle `{"ok": ..}` wird entfernt (ein `err` dort ist ein
/// Harness-Fehler, keine Paritaetsaussage).
fn frage(req: &Value) -> Value {
    let antwort = oracle().call_json(req).expect("Orakel antwortet");
    match antwort.get("ok") {
        Some(v) => v.clone(),
        None => panic!("Orakel-Harness-Fehler fuer {}: {antwort}", req["fn"]),
    }
}

/// Erster Unterschied als JSON-Pfad (ohne Werte — reale Daten bleiben aus der Ausgabe).
fn unterschied(links: &Value, rechts: &Value, pfad: &str) -> Option<String> {
    match (links, rechts) {
        (Value::Object(l), Value::Object(r)) => {
            for k in l.keys().chain(r.keys()) {
                match (l.get(k), r.get(k)) {
                    (Some(p), Some(q)) => {
                        if let Some(d) = unterschied(p, q, &format!("{pfad}/{k}")) {
                            return Some(d);
                        }
                    }
                    _ => return Some(format!("{pfad}/{k} (nur auf einer Seite)")),
                }
            }
            None
        }
        (Value::Array(l), Value::Array(r)) => {
            if l.len() != r.len() {
                return Some(format!("{pfad} (Laenge {} vs {})", l.len(), r.len()));
            }
            l.iter()
                .zip(r)
                .enumerate()
                .find_map(|(i, (p, q))| unterschied(p, q, &format!("{pfad}/{i}")))
        }
        _ if links == rechts => None,
        _ => Some(pfad.to_owned()),
    }
}

/// Ort der ersten Byte-Abweichung zweier XML-Texte (Zeile, Spalte) — ohne Inhalt.
fn xml_unterschied(a: &str, b: &str) -> Option<String> {
    if a == b {
        return None;
    }
    let pos = a
        .bytes()
        .zip(b.bytes())
        .position(|(x, y)| x != y)
        .unwrap_or(a.len().min(b.len()));
    let zeile = String::from_utf8_lossy(&a.as_bytes()[..pos])
        .matches('\n')
        .count()
        + 1;
    Some(format!(
        "XML weicht ab ab Byte {pos} (Zeile {zeile}), Laengen {}/{}",
        a.len(),
        b.len()
    ))
}

// ---------------------------------------------------------------- Rust-Seite eines Falls

#[derive(Default)]
struct Zaehler {
    faelle: usize,
    dekl_ok: usize,
    dekl_err: usize,
    zurueck: usize,
    xml_ok: usize,
    xml_err: usize,
    instanzen: usize,
    xsd_valide: usize,
    xsd_invalide: usize,
    abweichungen: Vec<String>,
    /// Erreichte Zweige (aus dem Python-Ergebnis gezaehlt) — belegt, was die Eingaben abdecken.
    zweige: BTreeMap<String, usize>,
}

fn zaehle_zweige(z: &mut Zaehler, d: &Value, xml: &Value) {
    let mut hit = |k: &str, ja: bool| {
        if ja {
            *z.zweige.entry(k.to_owned()).or_default() += 1;
        }
    };
    let leer = |v: &Value| {
        v.as_object().is_none_or(serde_json::Map::is_empty)
            && v.as_array().is_none_or(Vec::is_empty)
    };
    hit("konsistent", d["eingaben_konsistent"] == json!(true));
    hit("person_b", !leer(&d["person_b"]));
    hit("anlage_instanzen", !leer(&d["anlage_instanzen"]));
    hit("dokumentiert", !leer(&d["dokumentiert"]));
    hit("kind_anlagen", !leer(&d["kind_anlagen"]));
    hit("iban_de", d["deklaration"].get("E0102102").is_some());
    hit("iban_ausland", d["deklaration"].get("E0102603").is_some());
    hit("pflichtluecken", !leer(&d["pflichtfelder_luecken"]));
    hit("xml_basis_ok", xml["basis"].get("ok").is_some());
    hit("xml_abgabe_ok (Vorsatz)", xml["abgabe"].get("ok").is_some());
    for liste in ["nicht_deklariert", "unvollstaendig"] {
        for e in d[liste].as_array().into_iter().flatten() {
            let g = e["grund"].as_str().unwrap_or_default();
            let kurz: String = g
                .split(['(', '\'', ':'])
                .next()
                .unwrap_or_default()
                .chars()
                .take(28)
                .collect();
            hit(&format!("{liste}: {kurz}"), true);
        }
    }
    for e in xml.as_object().into_iter().flat_map(|m| m.values()) {
        if let Some(m) = e["msg"].as_str() {
            let kurz: String = m.chars().take(40).collect();
            hit(&format!("XmlFehler: {kurz}"), true);
        }
    }
}

fn vz_fuer_xml(vz: i64) -> i64 {
    if (2024..=2026).contains(&vz) {
        vz
    } else {
        2025
    }
}

fn xml_varianten(vz: i64) -> Value {
    json!({
        "basis": {"vz": vz, "hersteller_id": HID_TEST},
        "abgabe": {"vz": vz, "hersteller_id": HID_TEST, "abgabefaehig": true, "mit_snapshot": true},
    })
}

/// Der XML-Teil (`rust_xml`, xmllint) braucht die Schemas 2024 und 2025; generierte Stores tragen
/// nur diese Jahre. Ohne Schema scheitert `erzeuge_xml` auf BEIDEN Seiten mit demselben Text, und
/// der Vergleich ist gruen, ohne ein Byte XML geprueft zu haben. Rot, ausser `TAXGRAPH_OHNE_XSD=1`;
/// dann laufen die Vergleiche ohne XML weiter.
fn xml_braucht_schemas() {
    for vz in [2024, 2025] {
        let _ = elster::testhilfe::schemas_da(vz);
    }
}

fn rust_xml(
    d: &elster::Deklaration,
    felder: &Felder,
    vz: i64,
    abgabe: bool,
    hid: &str,
) -> Result<String, String> {
    let opt = XmlOptionen {
        vz,
        hersteller_id: Some(hid.to_owned()),
        abgabefaehig: abgabe,
        snapshot: abgabe.then_some(felder),
        ..XmlOptionen::default()
    };
    elster::erzeuge_xml(d, &opt).map_err(|e| e.0)
}

/// Vergleicht einen ganzen Fall. `zeige_werte`: nur fuer generierte Daten.
fn vergleiche_fall(
    datei: &StoreDatei,
    py_store: &Value,
    nur_bestaetigt: bool,
    z: &mut Zaehler,
    name: &str,
    zeige_werte: bool,
) {
    let vz = vz_fuer_xml(datei.veranlagungszeitraum.als_i64_saettigend());
    let py = frage(&json!({
        "fn": "elster.fall", "store": py_store, "nur_bestaetigt": nur_bestaetigt,
        "xml": xml_varianten(vz), "gruppen": GRUPPEN,
    }));
    z.faelle += 1;
    let store = Store::aus_datei(datei.clone());
    let (mut felder, sid) = store.materialisiere(None).expect("materialisiere");
    if nur_bestaetigt {
        felder.retain(|_, f| f.zustand == Zustand::Bestaetigt);
    }
    let mut neu: Vec<String> = Vec::new();
    let mut abw = |was: &str, wo: Option<String>| {
        if let Some(w) = wo {
            neu.push(format!("{name}: {was} {w}"));
        }
    };
    abw(
        "snapshot_id",
        (py["snapshot_id"] != json!(sid.to_string())).then(String::new),
    );
    // Das Jahr kommt aus dem FALL, nie aus `vz_fuer_xml`: der Helfer ersetzt jedes Jahr
    // ausserhalb 2024–2026 durch 2025 und verdeckte genau die Abweichung, die
    // `null_unzulaessig` sichtbar macht.
    let rust = elster::deklariere(
        &felder,
        index(),
        datei.veranlagungszeitraum.als_i64_saettigend(),
        Some(&sid),
    );
    match (&rust, py["deklariere"].get("ok")) {
        (Ok(d), Some(p)) => {
            z.dekl_ok += 1;
            if zeige_werte {
                zaehle_zweige(z, p, &py["xml"]);
            }
            let r = serde_json::to_value(d).unwrap();
            let wo = unterschied(&r, p, "");
            let wo = wo.map(|w| {
                if zeige_werte {
                    format!("{w}\n  rust={r}\n  py={p}")
                } else {
                    w
                }
            });
            abw("deklariere", wo);
            z.zurueck += 1;
            let rz = serde_json::to_value(elster::zuruecklesen(d, index())).unwrap();
            abw(
                "zuruecklesen",
                unterschied(&rz, &py["zuruecklesen"]["ok"], ""),
            );
            for (variante, abgabe) in [("basis", false), ("abgabe", true)] {
                vergleiche_xml(
                    &rust_xml(d, &felder, vz, abgabe, HID_TEST),
                    &py["xml"][variante],
                    vz,
                    z,
                    name,
                    zeige_werte,
                );
            }
        }
        (Err(e), None) => {
            z.dekl_err += 1;
            let klasse = py["deklariere"]["err"].as_str().unwrap_or_default();
            abw(
                "deklariere-Fehlerklasse",
                (e.python_klasse() != klasse).then(|| format!("{} vs {klasse}", e.python_klasse())),
            );
        }
        (r, _) => abw(
            "deklariere ok/err",
            Some(format!(
                "rust_ok={} py={}",
                r.is_ok(),
                py["deklariere"].get("err").is_none()
            )),
        ),
    }
    for g in GRUPPEN {
        z.instanzen += 1;
        let r = serde_json::to_value(elster::instanzen(&store, index(), g).expect("instanzen"))
            .unwrap();
        abw(
            &format!("instanzen[{g}]"),
            unterschied(&r, &py["instanzen"][g]["ok"], ""),
        );
    }
    z.abweichungen.extend(neu);
}

fn vergleiche_xml(
    rust: &Result<String, String>,
    py: &Value,
    vz: i64,
    z: &mut Zaehler,
    name: &str,
    zeige: bool,
) {
    let mut abw = |w: String| z.abweichungen.push(format!("{name}: xml {w}"));
    match (rust, py.get("ok").and_then(Value::as_str)) {
        (Ok(r), Some(p)) => {
            z.xml_ok += 1;
            if let Some(w) = xml_unterschied(r, p) {
                abw(if zeige {
                    format!("{w}\n--- rust\n{r}\n--- py\n{p}")
                } else {
                    w
                });
            }
            let schema_vz = u16::try_from(vz)
                .ok()
                .and_then(|j| domain::Vz::try_from(j).ok())
                .expect("vz_fuer_xml liefert 2024..=2026");
            let (ok, meldung) = elster::validiere_xsd_text(r.as_bytes(), schema_vz);
            if ok {
                z.xsd_valide += 1;
            } else {
                z.xsd_invalide += 1;
                if zeige && std::env::var("ZEIGE_XSD").is_ok() {
                    eprintln!("[xsd] {name}: {meldung}");
                }
            }
        }
        (Err(r), None) => {
            z.xml_err += 1;
            let klasse = py["err"].as_str().unwrap_or_default();
            let msg = py["msg"].as_str().unwrap_or_default();
            if klasse != "XmlFehler" || msg != r {
                abw(if zeige {
                    format!("Fehler {klasse}\n  rust={r}\n  py={msg}")
                } else {
                    format!("Fehlertext/Klasse {klasse}")
                });
            }
        }
        (r, _) => abw(format!(
            "ok/err rust_ok={} py_ok={}",
            r.is_ok(),
            py.get("ok").is_some()
        )),
    }
}

fn bericht(titel: &str, z: &Zaehler) {
    println!(
        "[{titel}] Faelle={} deklariere ok={} err={} | zuruecklesen={} | erzeuge_xml ok={} err={} | instanzen={} | \
         xmllint valide={} invalide={} | Abweichungen={}",
        z.faelle, z.dekl_ok, z.dekl_err, z.zurueck, z.xml_ok, z.xml_err, z.instanzen, z.xsd_valide, z.xsd_invalide,
        z.abweichungen.len()
    );
    for (k, v) in &z.zweige {
        println!("  Zweig {v:>5}  {k}");
    }
    for a in z.abweichungen.iter().take(10) {
        println!("  ABWEICHUNG {a}");
    }
}

// ---------------------------------------------------------------- Tests

/// Pythons `_ABZUGS_KZ` (`est_mapping.py`), sortiert.
fn py_abzugs_kz() -> Vec<String> {
    serde_json::from_value(frage(&json!({"fn": "elster.abzugs_kz"}))).expect("Liste von Kz")
}

/// Beide Seiten fuehren dieselben Abzugs-Kz. Fehlt eines auf einer Seite, rundet sie dort ab statt
/// auf (Vault `decisions/elster-testluecken-mit-eigener-probe-schliessen`, Punkt 1).
#[test]
fn abzugs_kz_mengengleich() {
    if skip_ohne_parity_env() {
        return;
    }
    let py_liste = py_abzugs_kz();
    let py: BTreeSet<&str> = py_liste.iter().map(String::as_str).collect();
    let rust: BTreeSet<&str> = elster::ABZUGS_KZ.iter().copied().collect();
    let nur_py: Vec<_> = py.difference(&rust).collect();
    let nur_rust: Vec<_> = rust.difference(&py).collect();
    println!(
        "[ABZUGS_KZ] py={} rust={} nur py={nur_py:?} nur rust={nur_rust:?}",
        py.len(),
        rust.len()
    );
    assert!(nur_py.is_empty() && nur_rust.is_empty());
}

#[test]
fn kz_format_sweep() {
    if skip_ohne_parity_env() {
        return;
    }
    let formate = ["E0200201", "E0705701", "E0200301", "E6004901"];
    // Vereinigung beider Abzugslisten: ein Kz, das nur Python aufrundet, fiele sonst aus dem Sweep.
    let py_abzug = py_abzugs_kz();
    let abzug: BTreeSet<&str> = elster::ABZUGS_KZ
        .iter()
        .copied()
        .chain(py_abzug.iter().map(String::as_str))
        .collect();
    let mut alle: Vec<&str> = abzug
        .into_iter()
        .chain(elster::KOMMA_OHNE_E60_KZ.iter().copied())
        .collect();
    alle.extend(["E6002301", "E1900701", "E1800501", "E0100001"]);
    let mut laeufe: Vec<(&str, i64, i64, usize)> = Vec::new();
    for kz in formate {
        laeufe.push((kz, -100_000, 100_000, 1));
        laeufe.push((kz, -1_000_000_000_000, 1_000_000_000_000, 99_999_989));
        laeufe.push((kz, i64::MAX / 4 - 1000, i64::MAX / 4, 1));
        laeufe.push((kz, i64::MIN / 4, i64::MIN / 4 + 1000, 1));
    }
    for kz in &alle {
        laeufe.push((kz, -1000, 1000, 1));
    }
    let mut n = 0_usize;
    let mut diffs = 0_usize;
    for (kz, von, bis, schritt) in laeufe {
        let py = frage(
            &json!({"fn": "elster.cent_sweep", "kz": kz, "von": von, "bis": bis, "schritt": schritt}),
        );
        let py = py.as_array().expect("Liste");
        let kz_typ = domain::Kz::new(kz).unwrap();
        let rust: Vec<Value> = (von..=bis)
            .step_by(schritt)
            .map(|c| elster::cent_nach_kz(domain::Cent::new(c), &kz_typ).als_json())
            .collect();
        assert_eq!(rust.len(), py.len(), "{kz}: Laenge");
        n += rust.len();
        for (i, (r, p)) in rust.iter().zip(py).enumerate() {
            if r != p {
                diffs += 1;
                if diffs < 5 {
                    println!(
                        "  ABWEICHUNG _cent_nach_kz({}, {kz}): rust={r} py={p}",
                        von + i64::try_from(i * schritt).unwrap()
                    );
                }
            }
        }
    }
    println!("[_cent_nach_kz] Werte={n} Abweichungen={diffs}");
    assert_eq!(diffs, 0);
    kz_wert_sweep();
    parse_instanz_sweep();
}

fn kz_wert_sweep() {
    let werte = [
        json!(0),
        json!(1),
        json!(-150),
        json!(2015),
        json!(99),
        json!(12345),
        json!(true),
        json!(false),
        json!("x"),
        json!("01.01.2015"),
        json!(null),
        json!(1.5),
        json!(-7),
        json!(10000),
    ];
    let kzs = [
        "E1800501", "E1801701", "E0200201", "E0705701", "E0200301", "E6004901", "E0100201",
    ];
    let typen = [
        Some("cent"),
        Some("int"),
        Some("bool"),
        Some("enum"),
        Some("datum"),
        Some("text"),
        None,
    ];
    let mut faelle = Vec::new();
    for w in &werte {
        for kz in kzs {
            for t in typen {
                // PARITÄT: float in einem cent-Feld (Python rechnet still in float) ist ausgenommen —
                // Auflage T laesst ihn im Store nicht zu, der Port weist ihn ab (kz_wert-Doku).
                if t == Some("cent") && w.is_f64() {
                    continue;
                }
                faelle.push(json!([w, kz, t]));
            }
        }
    }
    let py = frage(&json!({"fn": "elster.kz_wert", "faelle": faelle}));
    let mut diffs = 0;
    for (f, p) in faelle.iter().zip(py.as_array().unwrap()) {
        let typ = f[2]
            .as_str()
            .map(|t| serde_json::from_value::<Feldtyp>(json!(t)).unwrap());
        let kz = domain::Kz::new(f[1].as_str().unwrap()).unwrap();
        let r = match elster::kz_wert(&f[0], &kz, typ) {
            Ok(v) => json!({"ok": v}),
            Err(e) => json!({"err": e.klasse}),
        };
        let gleich = r
            .get("ok")
            .map_or_else(|| p.get("err") == r.get("err"), |v| p.get("ok") == Some(v));
        if !gleich {
            diffs += 1;
            println!("  ABWEICHUNG _kz_wert{f}: rust={r} py={p}");
        }
    }
    let jahr_faelle: Vec<Value> = [
        json!("01.01.2015"),
        json!(" 31.12.1999\n"),
        json!("1.1.2015"),
        json!("01.01.15"),
        json!(2015),
        json!("x"),
        json!("01.01.2015x"),
        json!("\u{a0}01.02.2003\u{2003}"),
        json!(null),
        json!(true),
    ]
    .iter()
    .flat_map(|w| ["E1800501", "E1803202", "E0200201"].map(|kz| json!([w, kz])))
    .collect();
    let pj = frage(&json!({"fn": "elster.jahr_aus_kz_wert", "faelle": jahr_faelle}));
    for (f, p) in jahr_faelle.iter().zip(pj.as_array().unwrap()) {
        let kz = domain::Kz::new(f[1].as_str().unwrap()).unwrap();
        let r = elster::jahr_aus_kz_wert(&f[0], &kz);
        if &r != p {
            diffs += 1;
            println!("  ABWEICHUNG _jahr_aus_kz_wert{f}: rust={r} py={p}");
        }
    }
    println!(
        "[_kz_wert] Faelle={} [_jahr_aus_kz_wert] Faelle={} Abweichungen={diffs}",
        faelle.len(),
        jahr_faelle.len()
    );
    assert_eq!(diffs, 0);
}

/// Die Zaehlerregel `[2-9]|[1-9][0-9]+` (Entscheidung 2026-10-03, vault
/// `die-schreib-route-weist-eine-kennung-mit-instanz-eins-ab`): `x__1` ist keine Instanz, Instanz 1 ist
/// die Basis. Feste Antworten, auf BEIDEN Seiten geprueft -- das Orakel soll nicht nur "keine" sagen.
const INSTANZ_ANKER: [(&str, Option<(&str, u64)>); 16] = [
    ("a", None),
    ("a__1", None),
    ("a__2", Some(("a", 2))),
    ("a__3", Some(("a", 3))),
    ("a__9", Some(("a", 9))),
    ("a__10", Some(("a", 10))),
    ("a__11", Some(("a", 11))),
    ("a__19", Some(("a", 19))),
    ("a__99", Some(("a", 99))),
    ("a__100", Some(("a", 100))),
    ("a__0", None),
    ("a__01", None),
    ("a__02", None),
    ("a__001", None),
    ("a__", None),
    ("a__x", None),
];

fn parse_instanz_sweep() {
    let mut ids: Vec<String> = [
        "a",
        "a__1",
        "a__2",
        "a__3",
        "a__9",
        "a__10",
        "a__11",
        "a__19",
        "a__99",
        "a__100",
        "a__01",
        "a__001",
        "a__x",
        "a__11\n",
        "a__1\n",
        "a__10\n",
        "a__02",
        "a__0",
        "a___1",
        "a__1__2",
        "a__2\n",
        "a__2\n\n",
        "A__2",
        "_a__2",
        "1a__2",
        "a__",
        "__2",
        "a_b__10",
        "vv_einnahmen__99",
        "a__2 ",
        " a__2",
        "ä__2",
        "a__99999999999999999",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    // Jeder Zaehler von 0 bis 120, glatt, mit fuehrender Null und mit abschliessendem Zeilenumbruch: die
    // Zaehlerregel `[2-9]|[1-9][0-9]+` an ihren Kanten (1, 9/10, 99/100) statt nur durch Zufall.
    for n in 0..=120 {
        for suffix in ["", "\n"] {
            ids.push(format!("a__{n}{suffix}"));
            ids.push(format!("a__0{n}{suffix}"));
        }
    }
    ids.sort_unstable();
    ids.dedup();
    let mut rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
    let zeichen = ['a', 'b', 'z', '_', '_', '_', '0', '1', '2', '9', 'A', '\n'];
    for _ in 0..20_000 {
        let len = 1 + (rng.next_u32() % 12) as usize;
        ids.push(
            (0..len)
                .map(|_| zeichen[(rng.next_u32() as usize) % zeichen.len()])
                .collect(),
        );
    }
    let py = frage(&json!({"fn": "elster.parse_instanz", "ids": ids}));
    let py = py.as_array().unwrap();
    let mut diffs = 0;
    // `store::instanz_basis` (die Basis allein): Python ist die Referenz, bei jeder Eingabe -- auch bei einem
    // abschliessenden `\n` (Pythons `$`), das `instanz_basis` seit `feld_kennung_paritaet` ebenso liest.
    // `zeilenende` zaehlt die `\n`-Eingaben, die Python als Instanz liest: sie muessen darunter sein.
    let (mut basis_diffs, mut zeilenende, mut instanz_ja, mut instanz_nein) = (0, 0, 0, 0);
    for (id, p) in ids.iter().zip(py) {
        let r = elster::parse_instanz(id).map_or(Value::Null, |(b, i)| json!([b, i]));
        if &r != p {
            diffs += 1;
            println!("  ABWEICHUNG parse_instanz({id:?}): rust={r} py={p}");
        }
        let py_basis = p.get(0).and_then(Value::as_str);
        instanz_ja += usize::from(py_basis.is_some());
        instanz_nein += usize::from(py_basis.is_none());
        zeilenende += usize::from(py_basis.is_some() && id.ends_with('\n'));
        let basis = store::instanz_basis(id);
        if basis != py_basis {
            basis_diffs += 1;
            println!("  ABWEICHUNG instanz_basis({id:?}): rust={basis:?} py={py_basis:?}");
        }
    }
    println!(
        "[parse_instanz] Eingaben={} Abweichungen={diffs}; [instanz_basis] Abweichungen={basis_diffs}, \
         davon Zeilenende gelesen {zeilenende}, Python Instanz {instanz_ja} / keine {instanz_nein}",
        ids.len()
    );
    assert_eq!(diffs, 0);
    assert_eq!(basis_diffs, 0);
    assert!(
        zeilenende >= 100 && instanz_ja > 200 && instanz_nein > 500,
        "{zeilenende} {instanz_ja} {instanz_nein}"
    );
    // Die Anker: Python, `parse_instanz` und `instanz_basis` sagen dasselbe, und es ist nicht "immer keine".
    for (id, soll) in INSTANZ_ANKER {
        let i = ids.iter().position(|x| x == id).unwrap_or_else(|| {
            panic!("{id:?} fehlt im Sweep");
        });
        let soll_py = soll.map_or(Value::Null, |(b, n)| json!([b, n]));
        assert_eq!(py[i], soll_py, "CPython {id:?}");
        assert_eq!(elster::parse_instanz(id), soll, "parse_instanz {id:?}");
        assert_eq!(
            store::instanz_basis(id),
            soll.map(|(b, _)| b),
            "instanz_basis {id:?}"
        );
    }
}

#[test]
fn schema_und_werkzeug() {
    if skip_ohne_parity_env() {
        return;
    }
    let mut diffs = 0;
    for vz in [2024_i64, 2025] {
        if !elster::testhilfe::schemas_da(vz) {
            continue;
        }
        let pfad = elster::finde_schema(vz, "E10-{jahr}.xsd").unwrap();
        let info = elster::schema_info(&pfad).unwrap();
        let py = frage(&json!({"fn": "elster.schema", "vz": vz}));
        let pfade = json!(info
            .pfade
            .iter()
            .map(|(k, p)| json!([k, p]))
            .collect::<Vec<_>>());
        let mut pflicht: Vec<Value> = info.pflicht.iter().map(|(p, k)| json!([p, k])).collect();
        pflicht.sort_by_key(ToString::to_string);
        let mut py_pflicht = py["pflicht"].as_array().unwrap().clone();
        py_pflicht.sort_by_key(ToString::to_string);
        let meta = serde_json::to_value(&info.kz_meta).unwrap();
        for (was, r, p) in [
            ("kz_pfade", &pfade, &py["pfade"]),
            ("pflicht_kinder", &json!(pflicht), &json!(py_pflicht)),
            ("kz_meta", &meta, &py["kz_meta"]),
        ] {
            if let Some(w) = unterschied(r, p, "") {
                diffs += 1;
                println!("  ABWEICHUNG {was}({vz}) {w}");
            }
        }
        println!(
            "[schema {vz}] kz_pfade={} pflicht_container={} kz_meta={}",
            info.pfade.len(),
            info.pflicht.len(),
            info.kz_meta.len()
        );
    }
    let rcs: Vec<i64> = vec![
        0,
        610_001_002,
        610_301_200,
        610_301_202,
        610_001_042,
        610_301_106,
        1,
        -1,
        610_001_861,
        42,
    ];
    let py = frage(&json!({"fn": "elster.klassifiziere_rc", "rcs": rcs}));
    for (rc, p) in rcs.iter().zip(py.as_array().unwrap()) {
        let k = elster::klassifiziere_rc(*rc);
        let r = json!([k, elster::nicht_geprueft(k)]);
        if &r != p {
            diffs += 1;
            println!("  ABWEICHUNG klassifiziere_rc({rc}): rust={r} py={p}");
        }
    }
    println!("[klassifiziere_rc] rcs={}", rcs.len());
    let mut prueflinge: Vec<elster::KzPruefling> = bindungen()
        .iter()
        .filter_map(|b| {
            b.elster_kz.as_ref().map(|kz| elster::KzPruefling {
                feld_id: b.feld_id.clone(),
                elster_kz: kz.to_string(),
                vz_gueltigkeit: b.vz_gueltigkeit.clone(),
            })
        })
        .collect();
    prueflinge.extend(elster::ernte_est_mapping_kz(index()).unwrap());
    let bericht = serde_json::to_value(elster::pruefe_bindung(&prueflinge).unwrap()).unwrap();
    let py = frage(&json!({"fn": "elster.pruefe_bindung"}));
    if let Some(w) = unterschied(&bericht, &py, "") {
        diffs += 1;
        println!("  ABWEICHUNG pruefe_bindung {w}");
    }
    println!(
        "[pruefe_bindung] Prueflinge={} exit_code={}",
        prueflinge.len(),
        bericht["exit_code"]
    );
    assert_eq!(diffs, 0);
}

#[test]
fn reale_faelle() {
    if skip_ohne_parity_env() {
        return;
    }
    xml_braucht_schemas();
    let dateien = walk_json(&faelle_verzeichnis());
    let mut z = Zaehler::default();
    for (i, pfad) in dateien.iter().enumerate() {
        // P10: store::lade laedt inzwischen ALLE realen Faelle (legacy Herkunft, unbegrenzte VZ).
        // Ein Ladefehler ist kein stiller Skip mehr, sondern ein harter Testabbruch.
        let datei =
            store::lade(pfad).unwrap_or_else(|e| panic!("store::lade({}): {e}", pfad.display()));
        // Python bekommt die ORIGINAL-Datei, nicht Rusts Re-Serialisierung.
        let original: Value =
            serde_json::from_str(&std::fs::read_to_string(pfad).unwrap()).unwrap();
        for nur in [false, true] {
            vergleiche_fall(
                &datei,
                &original,
                nur,
                &mut z,
                &format!("fall#{i}{}", if nur { "/bestaetigt" } else { "" }),
                false,
            );
        }
    }
    println!("[reale Faelle] Dateien={}", dateien.len());
    bericht("reale Faelle", &z);
    assert!(
        z.faelle > 0,
        "keine realen Faelle gefunden unter {}",
        faelle_verzeichnis().display()
    );
    assert!(z.abweichungen.is_empty());
}

// ---------------------------------------------------------------- generierte Stores

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Cursor<'_> {
    fn byte(&mut self) -> u8 {
        let b = self
            .bytes
            .get(self.pos % self.bytes.len().max(1))
            .copied()
            .unwrap_or(0);
        self.pos += 1;
        b.wrapping_add(u8::try_from(self.pos / self.bytes.len().max(1) % 251).unwrap())
    }
    fn range(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (usize::from(self.byte()) << 8 | usize::from(self.byte())) % n
        }
    }
    fn chance(&mut self, prozent: usize) -> bool {
        self.range(100) < prozent
    }
    fn waehle<'p, T>(&mut self, pool: &'p [T]) -> Option<&'p T> {
        (!pool.is_empty()).then(|| &pool[self.range(pool.len())])
    }
}

const IBANS: &[&str] = &[
    "DE89370400440532013000",
    "de89 3704 0044 0532 0130 00",
    "GB82WEST12345698765432",
    "AT611904300234573201",
    "DE88370400440532013000",
    "DE89",
    "XX00",
    "1234567890",
];
const CENTS: &[i64] = &[
    0,
    1,
    49,
    50,
    99,
    100,
    101,
    150,
    199,
    12_345,
    99_999,
    1_000_000,
    123_456_789,
    -1,
    -99,
    -150,
    -10_001,
];

fn text_wert(c: &mut Cursor, b: &Bindung) -> Value {
    match b.feld_id.as_str() {
        "stammdaten_iban" => json!(if c.chance(60) {
            IBANS[c.range(4)]
        } else {
            c.waehle(IBANS).unwrap()
        }),
        "stammdaten_steuernummer" => match c.range(4) {
            0 => json!("9181012345678"),
            1 => json!("1234012345678"),
            2 => json!("91811234567"),
            _ => json!(format!("91810{:08}", c.range(100_000_000))),
        },
        _ => match c.range(4) {
            0 => json!("Text <&> \"ü\" 'x'"),
            1 => json!(format!("Wert{}", c.range(1000))),
            _ => b
                .standardwert
                .clone()
                .filter(Value::is_string)
                .unwrap_or_else(|| b.beispielwert.clone()),
        },
    }
}

fn wert_fuer(c: &mut Cursor, b: &Bindung) -> Value {
    match b.typ {
        Feldtyp::Cent => {
            json!(c.waehle(CENTS).copied().unwrap_or(0) * if c.chance(50) { 1 } else { 7 })
        }
        Feldtyp::Int => {
            if b.feld_id.contains("jahr") {
                json!(1950 + c.range(80))
            } else {
                json!(c.range(6))
            }
        }
        Feldtyp::Bool => json!(c.chance(50)),
        Feldtyp::Enum => b
            .enum_werte
            .as_ref()
            .and_then(|w| c.waehle(w).cloned())
            .map_or(Value::Null, Value::String),
        Feldtyp::Datum => json!(format!(
            "{:02}.{:02}.{}",
            1 + c.range(28),
            1 + c.range(12),
            1940 + c.range(80)
        )),
        Feldtyp::Text => text_wert(c, b),
    }
}

fn herkunft_mensch() -> Herkunft {
    Herkunft {
        herkunft: Achsenwert::new("laie".to_owned()).unwrap(),
        pruef_tiefe: PruefTiefe::Ungeprueft,
        haftung: Achsenwert::new("nutzer".to_owned()).unwrap(),
    }
}

/// Ein Store aus zufaelligen, typ-korrekten Eingaben; abgewiesene Appends werden uebersprungen.
fn generiere_store(c: &mut Cursor) -> StoreDatei {
    let alle = bindungen();
    let instanzfaehig: Vec<&Bindung> = alle.iter().filter(|b| b.instanz_gruppe.is_some()).collect();
    let stammdaten: Vec<&Bindung> = alle
        .iter()
        .filter(|b| b.feld_id.starts_with("stammdaten_"))
        .collect();
    // "sauber": alles bestaetigt, Art-Weichen gesetzt, gueltige Bankverbindung — erreicht XML/Vorsatz.
    let sauber = c.chance(40);
    let alle_bestaetigt = sauber || c.chance(50);
    let mut kandidaten: Vec<(String, &Bindung)> = Vec::new();
    if sauber || c.chance(30) {
        let bank = if c.chance(50) {
            "stammdaten_iban"
        } else {
            "stammdaten_keine_bankverbindung"
        };
        kandidaten.extend(
            stammdaten
                .iter()
                .filter(|b| {
                    !sauber
                        || !b.feld_id.contains("bank") && b.feld_id != "stammdaten_iban"
                        || b.feld_id == bank
                })
                .map(|b| (b.feld_id.clone(), *b)),
        );
    }
    for _ in 0..(3 + c.range(40)) {
        if c.chance(25) {
            let b = *c.waehle(&instanzfaehig).unwrap();
            let n = 1 + c.range(4);
            let id = if n == 1 && c.chance(50) {
                b.feld_id.clone()
            } else {
                format!("{}__{n}", b.feld_id)
            };
            kandidaten.push((id, b));
        } else {
            let b = c.waehle(alle).unwrap();
            kandidaten.push((b.feld_id.clone(), b));
        }
    }
    if sauber {
        let weichen: Vec<&Bindung> = alle
            .iter()
            .filter(|b| b.typ == Feldtyp::Enum)
            .filter(|b| {
                b.feld_id.ends_with("art")
                    || b.feld_id.ends_with("art_partner")
                    || b.feld_id.ends_with("_typ")
            })
            .collect();
        let mut dazu = Vec::new();
        for (id, b) in &kandidaten {
            let suffix = id.strip_prefix(b.feld_id.as_str()).unwrap_or("").to_owned();
            for w in &weichen {
                if w.instanz_gruppe == b.instanz_gruppe {
                    let wid = if w.instanz_gruppe.is_some() {
                        format!("{}{suffix}", w.feld_id)
                    } else {
                        w.feld_id.clone()
                    };
                    dazu.push((wid, *w));
                }
            }
        }
        kandidaten.extend(dazu);
    }
    let mut store = Store::leer(if c.chance(80) { 2025 } else { 2024 }, None);
    let nachschlag = BindungNachschlag::neu(index());
    let signal = Signal2::new("ui:bestaetigt").unwrap();
    for (feld_id, b) in kandidaten {
        let bestaetigt = alle_bestaetigt || c.chance(80);
        let neu = NeuesEvent {
            feld_id,
            wert: PyWert::from(wert_fuer(c, b)),
            feldzustand: if bestaetigt {
                Feldzustand::Bestaetigt {
                    signal_2: signal.clone(),
                }
            } else {
                Feldzustand::Vorlaeufig
            },
            herkunft: herkunft_mensch(),
            schreiber: Schreiber::Mensch("julius".to_owned()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-09-29T00:00:00+00:00".to_owned()),
        };
        let _ = store.append(&neu, None, nachschlag);
    }
    if !sauber && c.chance(10) {
        let neu = NeuesEvent {
            feld_id: "kein_bindungsfeld_x".to_owned(),
            wert: PyWert::Ganz(1),
            feldzustand: Feldzustand::Bestaetigt { signal_2: signal },
            herkunft: herkunft_mensch(),
            schreiber: Schreiber::Mensch("julius".to_owned()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-09-29T00:00:00+00:00".to_owned()),
        };
        let _ = store.append(&neu, None, nachschlag);
    }
    store.into_datei()
}

#[test]
fn generierte_stores() {
    if skip_ohne_parity_env() {
        return;
    }
    xml_braucht_schemas();
    let z = std::cell::RefCell::new(Zaehler::default());
    let n = std::cell::Cell::new(0_usize);
    let cfg = Config {
        cases: 1000,
        failure_persistence: None,
        max_shrink_iters: 0,
        ..Config::default()
    };
    let mut runner =
        TestRunner::new_with_rng(cfg, TestRng::deterministic_rng(RngAlgorithm::ChaCha));
    let ergebnis = runner.run(&prop::collection::vec(any::<u8>(), 64..256), |bytes| {
        let mut c = Cursor {
            bytes: &bytes,
            pos: 0,
        };
        let datei = generiere_store(&mut c);
        n.set(n.get() + 1);
        let vorher = z.borrow().abweichungen.len();
        vergleiche_fall(
            &datei,
            &serde_json::to_value(&datei).unwrap(),
            false,
            &mut z.borrow_mut(),
            &format!("gen#{}", n.get()),
            true,
        );
        prop_assert_eq!(
            z.borrow().abweichungen.len(),
            vorher,
            "{:?}",
            z.borrow().abweichungen.last()
        );
        Ok(())
    });
    bericht("generierte Stores", &z.borrow());
    ergebnis.unwrap();
}

// ---------------------------------------------------------------- § 35c: je Massnahmenart ein Fall

/// (Massnahmenart, Container unter `EM_35c/Obj/Aufw/Massn`, Kz) — wie `P35C_ARTEN` in
/// `tests/test_kz_bindung_durchgang.py`. Die Liste dient NUR der Nicht-Leer-Probe; ob die Zuordnung
/// Art -> Zeile stimmt, entscheidet der Vergleich mit Python in `vergleiche_fall`. Ein Dreher in
/// dieser Liste UND in `rust/elster/src/tabellen.rs` faellt dort auf (Python steht dazwischen).
const P35C_ARTEN: [(&str, &str, &str); 9] = [
    ("waende", "Waende", "E0241001"),
    ("dach", "Dach", "E0241101"),
    ("geschossdecken", "Geschossd", "E0241201"),
    ("fenster_tueren", "Fenst_Tuer", "E0241301"),
    ("sommerlicher_waermeschutz", "Somm_Waerm", "E0241302"),
    ("lueftung", "Lueftung", "E0241401"),
    ("heizung", "Heizung", "E0241501"),
    ("digital", "Digital", "E0241601"),
    ("heizung_optimierung", "Heizung_alt", "E0241701"),
];

/// 12.345,67 Euro: der Cent-Anteil ist der Grund, warum die Kz-Rundung (auf/ab) mitgeprueft wird.
const P35C_BETRAG: i64 = 1_234_567;

/// Ein Store mit Art (falls gegeben) UND Einzelbetrag, beide unmittelbar bestaetigt. Der Orakelweg
/// ruft `_mit_ring_werten` nicht auf (`tools/parity/elster_oracle.py:46-52`): der Zwilling
/// `p35c_massnahme_einzelbetrag` entsteht dort nicht aus der Summe, darum traegt der Store ihn selbst.
fn p35c_store(art: Option<&str>) -> StoreDatei {
    let mut store = Store::leer(2025, None);
    let nachschlag = BindungNachschlag::neu(index());
    let signal = Signal2::new("ui:bestaetigt").unwrap();
    let felder = art
        .map(|a| ("p35c_massnahme_art", json!(a)))
        .into_iter()
        .chain([("p35c_massnahme_einzelbetrag", json!(P35C_BETRAG))]);
    for (feld_id, wert) in felder {
        let neu = NeuesEvent {
            feld_id: feld_id.to_owned(),
            wert: PyWert::from(wert),
            feldzustand: Feldzustand::Bestaetigt {
                signal_2: signal.clone(),
            },
            herkunft: herkunft_mensch(),
            schreiber: Schreiber::Mensch("julius".to_owned()),
            signal_1: None,
            ersetzt: None,
            ts: Some("2026-09-29T00:00:00+00:00".to_owned()),
        };
        store.append(&neu, None, nachschlag).expect("p35c-Feld");
    }
    store.into_datei()
}

/// Das Rust-XML (Variante `basis`) zu einem Store.
fn p35c_rust_xml(datei: &StoreDatei) -> Result<String, String> {
    let (felder, sid) = Store::aus_datei(datei.clone())
        .materialisiere(None)
        .unwrap();
    let vz = datei.veranlagungszeitraum.als_i64_saettigend();
    let d = elster::deklariere(&felder, index(), vz, Some(&sid)).map_err(|e| format!("{e:?}"))?;
    rust_xml(&d, &felder, vz, false, HID_TEST)
}

/// `xml` ohne Namespace-Praefixe in den Tags (`<ns0:E1>` -> `<E1>`), wie `_pfad_im_xml` in
/// `tests/test_kz_bindung_durchgang.py`: der Test haengt so nicht an der Praefix-Vergabe.
fn ohne_praefix(xml: &str) -> String {
    let mut aus = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(i) = rest.find('<') {
        aus.push_str(&rest[..=i]);
        rest = &rest[i + 1..];
        if let Some(r) = rest.strip_prefix('/') {
            aus.push('/');
            rest = r;
        }
        let ende = rest
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(rest.len());
        if rest[ende..].starts_with(':') {
            rest = &rest[ende + 1..];
        }
    }
    aus.push_str(rest);
    aus
}

/// Der Text unter `EM_35c/Obj/Aufw/Massn/<container>/<kz>`, wenn der Pfad im XML steht.
fn p35c_wert(xml: &str, container: &str, kz: &str) -> Option<String> {
    let flach = ohne_praefix(xml);
    let mut rest = flach.as_str();
    for name in ["EM_35c", "Obj", "Aufw", "Massn", container] {
        let i = rest.find(&format!("<{name}>"))?;
        rest = &rest[i..];
    }
    let i = rest.find(&format!("<{kz}>"))?;
    let inhalt = &rest[i + kz.len() + 2..];
    Some(inhalt[..inhalt.find('<')?].to_owned())
}

/// Die Nicht-Leer-Probe zu einer Art: das XML ist `Ok`, der Pfad steht darin mit einer Zahl, und
/// keines der anderen acht Kz steht darin. Rueckgabe: was nicht stimmt.
fn p35c_probe(art: &str, xml: &Result<String, String>) -> Vec<String> {
    let Some((_, container, kz)) = P35C_ARTEN.iter().find(|(a, _, _)| *a == art) else {
        return vec![format!("unbekannte Art {art}")];
    };
    let xml = match xml {
        Ok(x) => x,
        Err(e) => return vec![format!("Rust-XML ist Err: {e}")],
    };
    let mut fehler = Vec::new();
    match p35c_wert(xml, container, kz) {
        None => fehler.push(format!("{container}/{kz} fehlt im XML")),
        Some(w) if w.is_empty() || !w.bytes().all(|b| b.is_ascii_digit()) => {
            fehler.push(format!("{container}/{kz} traegt keine Zahl: {w:?}"));
        }
        Some(_) => {}
    }
    for (_, _, anderes) in P35C_ARTEN.iter().filter(|(_, _, k)| k != kz) {
        if xml.contains(anderes) {
            fehler.push(format!("{anderes} steht auch im XML (fremde Zeile)"));
        }
    }
    fehler
}

/// Je Massnahmenart des § 35c ein handgebauter Fall mit Cent-Anteil im Betrag: Rust gleich Python
/// (Deklaration, Zuruecklesen, XML), dazu die Nicht-Leer-Probe. Der Zufallsgenerator
/// (`generierte_stores`) trifft vier der neun Arten nie (Vault `decisions/je-sanierungsart-ein-
/// handgebauter-store-im-paritaetstest`); ein Dreher in der Tabelle `tabellen.rs` (`p35c_massnahme_art`
/// -> Kz) bleibt dort unsichtbar. Gemessen: `schema_und_werkzeug` (`pruefe_bindung`) faengt ihn heute
/// auch, braucht dafuer aber die XSD-Schemas; dieser Test faengt ihn ueber die Deklaration und, mit
/// Schemas, ueber das XML. Der Generator bleibt unveraendert.
#[test]
fn p35c_je_massnahmenart_ein_fall() {
    if skip_ohne_parity_env() {
        return;
    }
    // Ohne Schemas (`TAXGRAPH_OHNE_XSD=1`) laeuft nur der Vergleich der Deklaration, wie in den
    // anderen Tests dieser Datei; sonst ist ein fehlendes Schema rot.
    let mit_xml = elster::testhilfe::schemas_da(2025);
    let mut z = Zaehler::default();
    for (art, container, kz) in P35C_ARTEN {
        let datei = p35c_store(Some(art));
        let name = format!("p35c/{art}");
        vergleiche_fall(
            &datei,
            &serde_json::to_value(&datei).unwrap(),
            false,
            &mut z,
            &name,
            true,
        );
        if mit_xml {
            let xml = p35c_rust_xml(&datei);
            let wert = xml.as_ref().ok().and_then(|x| p35c_wert(x, container, kz));
            println!("  {art:<26} {container}/{kz} = {wert:?}");
            let probe = p35c_probe(art, &xml);
            z.abweichungen
                .extend(probe.into_iter().map(|p| format!("{name}: {p}")));
        }
    }
    bericht("p35c je Massnahmenart", &z);
    assert_eq!(z.faelle, 9, "neun Arten, neun Faelle");
    assert_eq!(z.dekl_ok, 9, "deklariere lieferte nicht in jedem Fall Ok");
    assert!(
        !mit_xml || z.xml_ok >= 9,
        "weniger als neun Fall-XML auf beiden Seiten Ok: {}",
        z.xml_ok
    );
    assert!(z.abweichungen.is_empty(), "{:#?}", z.abweichungen);
    // Gegenprobe: ohne Art im Speicher steht der Betrag in keiner der neun Zeilen; die Probe muss
    // das sehen, sonst waere ein leeres XML auf beiden Seiten gruen.
    if mit_xml {
        let leer = p35c_rust_xml(&p35c_store(None));
        for (art, _, _) in P35C_ARTEN {
            assert!(
                !p35c_probe(art, &leer).is_empty(),
                "Gegenprobe: die Probe sieht den fehlenden Betrag bei {art} nicht"
            );
        }
    }
    println!("[p35c] neun Arten, je Fall Rust gleich Python; mit XML: Pfad im XML und Gegenprobe (leere Art) erkannt");
}

// ---------------------------------------------------------------- ERiC

/// Hersteller-ID aus der Umgebung oder den `.env`-Dateien der Repo-Wurzel. Nie ausgeben.
fn hersteller_id() -> Option<String> {
    if let Ok(h) = std::env::var("ELSTER_HERSTELLER_ID") {
        if !h.trim().is_empty() {
            return Some(h.trim().to_owned());
        }
    }
    for datei in [".env", ".env.elster", ".env.local"] {
        let Ok(text) = std::fs::read_to_string(repo_root().join(datei)) else {
            continue;
        };
        for zeile in text.lines() {
            let zeile = zeile.trim().trim_start_matches("export ");
            if let Some(w) = zeile.strip_prefix("ELSTER_HERSTELLER_ID=") {
                let w = w.trim().trim_matches(['"', '\'']);
                if !w.is_empty() {
                    return Some(w.to_owned());
                }
            }
        }
    }
    None
}

fn eric_vergleich(xml_rust: &str, xml_py: &str, datenart: &str) -> (i32, Value) {
    let (rc, antwort) = elster::validiere(xml_rust.as_bytes(), datenart).expect("ERiC laedt");
    let fehler = antwort.matches("<FehlerRegelpruefung>").count();
    let py = frage(&json!({"fn": "elster.checkest", "xml": xml_py, "datenart": datenart}));
    (rc, json!([py, [rc, fehler]]))
}

#[test]
fn checkest_stichprobe() {
    if skip_ohne_parity_env() {
        return;
    }
    if elster::find_eric_lib().is_none() {
        println!("[checkESt] ERiC nicht gefunden — source_unavailable, kein rc-Vergleich");
        return;
    }
    xml_braucht_schemas();
    let hid = hersteller_id();
    println!(
        "[checkESt] Hersteller-ID {}",
        if hid.is_some() {
            "gesetzt (Wert nicht ausgegeben)"
        } else {
            "FEHLT"
        }
    );
    let hid = hid.unwrap_or_else(|| HID_TEST.to_owned());
    let mut vergleiche = 0;
    let mut diffs = 0;
    let mut klassen: BTreeMap<String, usize> = BTreeMap::new();
    // Amtliches Beispiel (Harness-Beweis wie checkest_gate --prove), plausibel und verfaelscht.
    let beispiel = std::fs::read_to_string(
        repo_root().join("elster/testdaten/est_2020_amtliches_beispiel.xml"),
    )
    .unwrap_or_default();
    let beispiel = beispiel.replace("74931", &hid);
    let kaputt = beispiel.replace(
        "<E0100401>05.05.1955</E0100401>",
        "<E0100401>99.99.9999</E0100401>",
    );
    let mut proben: Vec<(String, String, String)> = vec![
        (beispiel.clone(), beispiel, "ESt_2020".to_owned()),
        (kaputt.clone(), kaputt, "ESt_2020".to_owned()),
    ];
    let mut rng = TestRng::deterministic_rng(RngAlgorithm::ChaCha);
    let mut versuche = 0;
    while proben.len() < 10 && versuche < 400 {
        versuche += 1;
        let bytes: Vec<u8> = (0..200)
            .map(|_| u8::try_from(rng.next_u32() % 256).unwrap())
            .collect();
        let datei = generiere_store(&mut Cursor {
            bytes: &bytes,
            pos: 0,
        });
        let store = Store::aus_datei(datei.clone());
        let (felder, sid) = store.materialisiere(None).unwrap();
        let vz = datei.veranlagungszeitraum.als_i64_saettigend();
        let Ok(d) = elster::deklariere(&felder, index(), vz, Some(&sid)) else {
            continue;
        };
        let abgabe = proben.len().is_multiple_of(2);
        let Ok(r) = rust_xml(&d, &felder, vz, abgabe, &hid) else {
            continue;
        };
        let mut kw = xml_varianten(vz)[if abgabe { "abgabe" } else { "basis" }].clone();
        kw["hersteller_id"] = json!(hid);
        let py = frage(
            &json!({"fn": "elster.fall", "store": serde_json::to_value(&datei).unwrap(), "xml": {"x": kw}, "gruppen": []}),
        );
        let Some(p) = py["xml"]["x"]["ok"].as_str() else {
            continue;
        };
        proben.push((r, p.to_owned(), format!("ESt_{vz}")));
    }
    for (r, p, datenart) in &proben {
        vergleiche += 1;
        if r != p {
            diffs += 1;
        }
        let (rc, beide) = eric_vergleich(r, p, datenart);
        *klassen
            .entry(format!("{:?}", elster::klassifiziere_rc(i64::from(rc))))
            .or_default() += 1;
        if beide[0] != beide[1] {
            diffs += 1;
            println!("  ABWEICHUNG checkESt {datenart}: py/rust = {beide}");
        }
    }
    println!("[checkESt] Proben={vergleiche} (2 amtlich ESt_2020 + {} generiert) rc-Klassen={klassen:?} Abweichungen={diffs}", vergleiche - 2);
    assert_eq!(diffs, 0);
}

#[test]
fn negativkontrolle() {
    if skip_ohne_parity_env() {
        return;
    }
    // (1) Cent-Sweep: ein Wert um 1 gestoert.
    let py = frage(
        &json!({"fn": "elster.cent_sweep", "kz": "E0705701", "von": -500, "bis": 500, "schritt": 1}),
    );
    let mut rust: Vec<Value> = (-500..=500)
        .map(|c| {
            elster::cent_nach_kz(domain::Cent::new(c), &domain::Kz::new("E0705701").unwrap())
                .als_json()
        })
        .collect();
    assert_eq!(json!(rust), py, "Ausgangslage gleich");
    rust[600] = json!(rust[600].as_i64().unwrap() + 1);
    assert_ne!(
        json!(rust),
        py,
        "Negativkontrolle Cent: Stoerung nicht erkannt"
    );
    // (2) Deklaration: ein Kz-Wert gestoert.
    let mut z = Zaehler::default();
    let mut c = Cursor {
        bytes: &[7, 3, 9, 200, 17, 88, 1, 2, 3, 4, 5, 6, 250, 11],
        pos: 0,
    };
    let datei = generiere_store(&mut c);
    vergleiche_fall(
        &datei,
        &serde_json::to_value(&datei).unwrap(),
        false,
        &mut z,
        "neg",
        true,
    );
    assert!(
        z.abweichungen.is_empty(),
        "Ausgangslage muss gleich sein: {:?}",
        z.abweichungen
    );
    let (felder, sid) = Store::aus_datei(datei.clone())
        .materialisiere(None)
        .unwrap();
    // Dieselbe Jahresquelle wie `vergleiche_fall` und der Oracle (`store.veranlagungszeitraum`).
    let vz = datei.veranlagungszeitraum.als_i64_saettigend();
    let mut d = serde_json::to_value(
        elster::deklariere(&felder, index(), vz, Some(&sid)).unwrap(),
    )
    .unwrap();
    d["deklaration"]["E0100001"] = json!(false);
    let py = frage(
        &json!({"fn": "elster.fall", "store": serde_json::to_value(&datei).unwrap(), "gruppen": []}),
    );
    assert!(
        unterschied(&d, &py["deklariere"]["ok"], "").is_some(),
        "Negativkontrolle Deklaration nicht erkannt"
    );
    // (3) XML: ein Byte gestoert.
    assert!(xml_unterschied("<a>1</a>", "<a>2</a>").is_some());
    println!("[Negativkontrolle] Cent, Deklaration, XML: jede Stoerung erkannt");
}
