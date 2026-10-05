//! Ueberlaufgrenzen der Fallakte: die Zahlformen, die `rust/store` beim Laden sperrt, und die Jahre,
//! die sie in `i128` haelt (`produkt/haut/server.py::_ganzzahl_im_i64`, `decisions/
//! tuer-und-speicher-weisen-ab-was-die-fallakte-nicht-exakt-halten-kann`). Standardlauf, ohne Python.
//!
//! Die Mutanten (Bericht h8-hermetisch5, gemessen 2026-10-04 auf 0aa91677, `cargo test -p store
//! -p api -p bescheid -p eingang -p intervall --lib --tests`, 403 passed / 0 failed): S1
//! `sperrform`: `KommazahlUeberlauf` nie (`is_ok_and(f64::is_infinite)` -> `false`), S3 `sperrform`:
//! `GanzzahlUeberlauf` nie (`(ganzzahl && ausserhalb)` -> `false`), S4 `visit_u128`: Wert ueber
//! `i128::MAX` wird 0 statt `i128::MAX`, S5 `als_i64_saettigend`: obere Grenze saettigt nach
//! `i64::MIN` (`is_positive` -> `is_negative`), S7 `visit_f64`: `v.trunc() as i128` -> `i128::from(v.trunc() as
//! i64)` (die Saettigung nach `i64` zieht in den Rohwert vor; erreichbar ueber `serde_json::from_value`, wo ein
//! Deserializer eine Kommazahl liefert — `ein_jahr_als_kommazahl_...` unten). S2 (`parse::<u64>().is_err()` zu
//! `||`) faengt der Bestand (`b4_ganzzahl_ueberlauf_sperrt_mit_namen`). S6 (`finde_sperrform`, Klammertiefe
//! `saturating_sub` -> `wrapping_sub`): auf jeder Datei, die Python als JSON liest, faellt die Tiefe nie unter 0 und beide rechnen
//! gleich; erst ein Text mit ueberzaehliger schliessender Klammer (kein JSON, Python: `JSONDecodeError`) unterscheidet sie: das
//! Original meldet einen Fehler, die Mutante PANIKT (`attempt to add with overflow` an `tiefe += 1`, gemessen: Sonde im Messbaum,
//! `s6_original.out` / `s6_mutant.out`) -- `eine_ueberzaehlige_schliessende_klammer_ist_ein_fehler_und_keine_panik` unten.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Rumpf-Grenze ist Python nachgerechnet — `server._ganzzahl_im_i64`
//! (im Orakel-Lauf `tools/parity/schritt8_oracle.py` haelt dieselben Zeichenketten) laesst
//! `i64::MIN..=i64::MAX` durch und weist `9223372036854775808`, `18446744073709551615`,
//! `-9223372036854775809` und jede 39-stellige Zahl mit `ValueError` ab. Die Rust-Akte sperrt
//! innerhalb von `events` schon eine Ganzzahl bis `u64::MAX` nicht (`Sperrform::GanzzahlUeberlauf`
//! = "ausserhalb `i64::MIN..=u64::MAX`"), das ist die dokumentierte Grenze der Doku — sie ist hier
//! als Konvention markschiert, nicht als Orakel-Wert. Ein Jahr ueber `i64` haelt Rust als `i128`
//! (Store-Doku, ponytail) und gibt es nach `i64` gesaettigt zurück; Python`json.load` haelt die
//! exakte Ganzzahl, die Ziffernliste unten ist die 10^38 des gemessenen Korpusfalls.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::Path;

use domain::{Achsenwert, Feldzustand, Herkunft, PruefTiefe, PyWert, Schreiber, Signal2};
use serde_json::json;
use store::{
    Abweisung, BindungNachschlag, NeuesEvent, PersistenzFehler, Sperrform, Store, StoreDatei,
    Veranlagungsjahr,
};

fn nachschlag() -> BindungNachschlag<'static> {
    static B: std::sync::OnceLock<Vec<bindung::Bindung>> = std::sync::OnceLock::new();
    static M: std::sync::OnceLock<std::collections::HashMap<String, &'static bindung::Bindung>> =
        std::sync::OnceLock::new();
    BindungNachschlag::neu(M.get_or_init(|| {
        store::baue_nachschlag(B.get_or_init(|| {
            let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
            bindung::lade_registry_der_wurzel(&pfad)
                .unwrap()
                .dateien
                .into_iter()
                .flat_map(|(_, d)| d.bindungen)
                .collect()
        }))
    }))
}

fn pfad(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("taxgraph-h5-akten-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join(format!("{name}.json"))
}

fn schreibe(name: &str, text: &str) -> std::path::PathBuf {
    let p = pfad(name);
    std::fs::write(&p, text).unwrap();
    p
}

/// Eine Akte mit genau einem Event; `wert` steht roh in Zeile 3, Spalte 50 (Laenge des
/// Zeilenanfangs `{"signal":{"signal_1":null,"signal_2":"a"},"wert":` = 49 Zeichen).
fn akte(vz: &str, wert: &str) -> String {
    concat!(
        r#"{"version":1,"veranlagungszeitraum":VZ,"snapshots":[],"events":["#,
        "\n",
        r#"{"event_id":"ID","ts":"2026-01-01T00:00:00+00:00","feld_id":"ep_arbeitstage","#,
        "\n",
        r#""signal":{"signal_1":null,"signal_2":"a"},"wert":WERT,"zustand":"bestaetigt","#,
        r#""herkunft":{"herkunft":"mensch","pruef_tiefe":"ungeprueft","haftung":"nutzer"},"#,
        r#""schreiber":"m","ersetzt":null}]}"#
    )
    .replace("VZ", vz)
    .replace("ID", &"0".repeat(64))
    .replace("WERT", wert)
}

/// `(Wert-Text, erwartete Sperrform oder None, Art)`. `None` = die Akte laedt.
const GRENZEN: &[(&str, Option<Sperrform>, &str)] = &[
    // Grenze der Rust-Sperre in events: i64::MAX und u64::MAX gehen durch, eine Zahl ueber u64::MAX sperrt.
    ("9223372036854775807", None, "i64::MAX laedt [konvention]"),
    (
        "18446744073709551615",
        None,
        "u64::MAX laedt als GrossGanz [konvention]",
    ),
    (
        "18446744073709551616",
        Some(Sperrform::GanzzahlUeberlauf),
        "u64::MAX + 1 sperrt [klar: Python-Wert ausserhalb i64]",
    ),
    (
        "-9223372036854775809",
        Some(Sperrform::GanzzahlUeberlauf),
        "i64::MIN - 1 sperrt [klar: Python-Wert ausserhalb i64]",
    ),
    (
        "-9223372036854775808",
        None,
        "i64::MIN laedt [konvention: Python weist sie an der Rumpftuer ab]",
    ),
    (
        "1e400",
        Some(Sperrform::KommazahlUeberlauf),
        "Kommazahl ueber f64 sperrt [klar]",
    ),
    (
        "1e308",
        None,
        "Kommazahl innerhalb f64 laedt [knapp: 1e308 ist noch kein inf]",
    ),
];

#[test]
fn die_akte_sperrt_die_zahlformen_ihrer_grenze_und_laedt_darunter() {
    let mut falsch = Vec::new();
    for (wert, form, art) in GRENZEN {
        let p = schreibe(&format!("grenze-{}", art.len()), &akte("2025", wert));
        let ist = store::lade(&p);
        let meldung = match (form, &ist) {
            (
                Some(f),
                Err(PersistenzFehler::Sperrform {
                    zeile,
                    spalte,
                    form: ist,
                    ..
                }),
            ) => {
                if ist == f && *zeile == 3 && *spalte == 50 {
                    None
                } else {
                    Some(format!("Sperrform {f:?} in Zeile 3/Spalte 50 erwartet, gekommen {ist:?} ({zeile}/{spalte})"))
                }
            }
            (Some(f), Err(e)) => Some(format!("Sperrform {f:?} erwartet, gekommen {e:?}")),
            (Some(f), Ok(_)) => Some(format!("Sperrform {f:?} erwartet, gekommen Ok")),
            (None, Err(e)) => Some(format!("Laden erwartet, gekommen {e:?}")),
            (None, Ok(_)) => None,
        };
        if let Some(m) = meldung {
            falsch.push(format!("{wert} [{art}]: {m}"));
        }
        std::fs::remove_file(p).ok();
    }
    assert_eq!(
        GRENZEN.len(),
        7,
        "die Zahl der Grenzfaelle hat sich verschoben"
    );
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Die Kehrseite der Sperrform: kein Weg bringt eine Ganzzahl ausserhalb `i64` in die Akte. Python
/// weist sie schon an der Rumpftuer ab (`server._ganzzahl_im_i64`, Doku oben); Rust sperrt dieselben
/// Zahlen beim `append` (Achsen-Pruefung: Wert passt nicht zum Bindungstyp) und beim Laden (Sperrform).
/// Erwartung hier ist also nur: `append` lehnt ab — welcher Zweig zuerst greift, ist Rust-Interne.
fn event(feld: &str, wert: serde_json::Value) -> NeuesEvent {
    NeuesEvent {
        feld_id: feld.to_string(),
        wert: PyWert::from(wert),
        feldzustand: Feldzustand::Bestaetigt {
            signal_2: Signal2::new("klick").unwrap(),
        },
        herkunft: Herkunft {
            herkunft: Achsenwert::new("mensch").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("nutzer").unwrap(),
        },
        schreiber: Schreiber::Mensch("julius".to_string()),
        signal_1: None,
        ersetzt: None,
        ts: Some("2026-01-01T00:00:00+00:00".to_string()),
    }
}

#[test]
fn append_weist_dieselben_zahlen_ab_wie_die_sperrform_der_akten_datei() {
    let gross = [
        ("u64::MAX", json!(u64::MAX)),
        ("2^63 (i64::MAX + 1)", json!("9223372036854775808")),
        ("i64::MIN als Text", json!("-9223372036854775808")),
    ];
    let n = gross.len();
    let mut falsch = Vec::new();
    for (name, wert) in gross {
        let mut s = Store::leer(2025, None);
        match s.append(&event("ep_arbeitstage", wert), None, nachschlag()) {
            Err(Abweisung::TypInkonform { .. } | Abweisung::Magnitude { .. }) => {}
            Err(e) => falsch.push(format!(
                "{name}: Abweisung (Typ/Magnitude) erwartet, gekommen {e:?}"
            )),
            Ok(_) => falsch.push(format!("{name}: append haelt die Zahl durch")),
        }
    }
    assert_eq!(n, 3, "die Zahl der Append-Faelle hat sich verschoben");
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Das Jahr: 10^38 (der gemessene Korpusfall) haelt die Akte als `i128`, die `i64`-Sicht saettigt
/// oben — nie ein Wrap auf ein plausibles Jahr. Ein Jahr ueber `i64::MAX`, aber unter `u64::MAX`
/// (67561) muss die `u16`-Schranke der Routes es ausschliessen, sonst laest es als 2025.
#[test]
fn ein_jahr_ausserhalb_i64_saettigt_und_wird_nie_ein_steuerjahr() {
    let (zwei_63, max_u16_ueber, zehn_38) = (
        "9223372036854775808",
        "67561",
        "99999999999999999999999999999999999999",
    );
    let mut falsch = Vec::new();
    // 1) 10^38 laedt, die i64-Sicht ist i64::MAX (nicht 0, nicht gewrappt).
    let p = schreibe("vz-zehn-38", &akte(zehn_38, "1"));
    let datei = store::lade(&p).unwrap_or_else(|e| panic!("10^38: {e:?}"));
    let s = Store::aus_datei(datei);
    if s.veranlagungszeitraum() != i64::MAX {
        falsch.push(format!(
            "10^38: i64::MAX erwartet, gekommen {}",
            s.veranlagungszeitraum()
        ));
    }
    // 2) 2^63 laedt und saettigt ebenfalls nach oben (i128, nicht u16-Rueckfall).
    let p2 = schreibe("vz-zwei-63", &akte(zwei_63, "1"));
    let s2 = Store::aus_datei(store::lade(&p2).unwrap_or_else(|e| panic!("2^63: {e:?}")));
    if s2.veranlagungszeitraum() != i64::MAX {
        falsch.push(format!(
            "2^63: i64::MAX erwartet, gekommen {}",
            s2.veranlagungszeitraum()
        ));
    }
    // 3) 67561 = 2025 mod 2^16: in `u16` passt es, in `Vz` nicht. Ein `as u16`-Rueckfall machte
    //    daraus 2025 — die Route darf es nicht als Jahr annehmen.
    let p3 = schreibe("vz-67561", &akte(zwei_63, "1"));
    let d3: StoreDatei = store::lade(&p3).unwrap();
    if u16::try_from(i64::try_from(d3.veranlagungszeitraum.0).unwrap_or(i64::MAX))
        .is_ok_and(|j| j == 2025)
    {
        falsch.push(
            "i64::MAX als u16 = 2025 (Wrap): die Schranke muss die i64-Zahl pruefen".to_owned(),
        );
    }
    let _ = std::fs::remove_file(p);
    let _ = std::fs::remove_file(p2);
    let _ = std::fs::remove_file(p3);
    let _ = max_u16_ueber;
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// S7: ein Jahr, das ein Deserializer als Kommazahl liefert (`serde_json::from_value`, wie die
/// Parity-Tests; `persistenz::lade` liest Ziffern und kommt hier nicht vorbei), wird wie Pythons
/// `int(float)` Richtung Null abgeschnitten und bleibt bis `i128` exakt. Die Saettigung auf `i64`
/// gehoert allein in `als_i64_saettigend`: der Rohwert `Veranlagungsjahr.0` ist das, was die Akte
/// beim Speichern wieder schreibt.
///
/// Python-Folien: `int(1e30) = 1000000000000000019884624838656`, `int(1e19) = 10000000000000000000`,
/// `int(9e18) = 9000000000000000000`, `int(2025.9) = 2025`, `int(-2025.9) = -2025` (gemessen,
/// Python 3.14). Die Faelle ueber 9,2e18 (alle vier ersten) trennen `v.trunc() as i128` von
/// `i128::from(v.trunc() as i64)`; 9e18 und die Jahreszahlen sind die Gegenproben darunter.
#[test]
fn ein_jahr_als_kommazahl_schneidet_ab_und_haelt_die_i128_breite() {
    let faelle: [(f64, i128); 8] = [
        (1e30, 1_000_000_000_000_000_019_884_624_838_656),
        (-1e30, -1_000_000_000_000_000_019_884_624_838_656),
        (1e19, 10_000_000_000_000_000_000),
        (-1e19, -10_000_000_000_000_000_000),
        (9.0e18, 9_000_000_000_000_000_000),
        (-9.0e18, -9_000_000_000_000_000_000),
        (2025.9, 2025),
        (-2025.9, -2025),
    ];
    let mut falsch = Vec::new();
    for (f, erwartet) in faelle {
        let ist: Veranlagungsjahr = serde_json::from_value(json!(f)).unwrap();
        if ist.0 != erwartet {
            falsch.push(format!("{f:e}: erwartet {erwartet}, gekommen {}", ist.0));
        }
    }
    // S4-Beleg: `serde_json::from_str` liest ein Jahr ueber `i128::MAX` nie als `u128` (das
    // wuerde `visit_u128` rufen), sondern meldet `NumberOutOfRange` — `visit_u128` ist ueber diese
    // Naht unerreichbar. 10^39 ist rund das 5,9-fache von `i128::MAX` (1,7e38); Python haelt die exakte Zahl.
    let zu_gross = format!("1{}", "0".repeat(39));
    let fehler = serde_json::from_str::<Veranlagungsjahr>(&zu_gross)
        .unwrap_err()
        .to_string();
    assert!(fehler.contains("out of range"), "10^39 als Jahr: {fehler}");
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// S6: Text mit ueberzaehliger schliessender Klammer vor dem Objekt ist kein JSON; Python `json.loads` wirft `JSONDecodeError`
/// (gemessen fuer alle drei Texte). Rust meldet einen Fehler und panikt nie: `}` bei Tiefe 0 bleibt bei 0 (`saturating_sub`). Die
/// Fehlerart ist Rust-eigen (`Sperrform` fuer einen Text, dessen `events` nach dem Objektanfang ein `1e999` tragen, sonst
/// `Format`), keine Python-Stuetze; die Messgroesse ist die fehlende Panik.
#[test]
fn eine_ueberzaehlige_schliessende_klammer_ist_ein_fehler_und_keine_panik() {
    let kopf = r#"{"version":1,"veranlagungszeitraum":2025,"events":"#;
    let mut falsch = Vec::new();
    for (name, text, sperrform) in [
        (
            "klammer_inf",
            format!("}}{kopf}[{{\"wert\":1e999}}]}}"),
            true,
        ),
        ("klammer_gueltig", format!("}}{kopf}[]}}"), false),
        (
            "zwei_klammern_inf",
            format!("]}}{kopf}[{{\"wert\":1e999}}]}}"),
            true,
        ),
    ] {
        let r = store::lade(&schreibe(name, &text));
        let ok = matches!(
            (&r, sperrform),
            (Err(PersistenzFehler::Sperrform { .. }), true)
                | (Err(PersistenzFehler::Format(..)), false)
        );
        if !ok {
            falsch.push(format!("{name}: {:?}", r.map(|_| ())));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}
