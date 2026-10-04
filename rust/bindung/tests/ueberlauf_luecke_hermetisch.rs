//! Ueberlaufstellen von `bindung::Params`, die kein Bestandstest erreicht, im Standardlauf (ohne `PARITY=1`, ohne Python). Bericht
//! h8-ueberlauf-luecke. Beide Werte kommen aus `params/` -- kein Nutzerfeld erreicht sie; erreichbar nur mit gepatchter
//! Parameterdatei (Schwaeche dieses Harness: die Datei ist ein Datensatz, kein Quelltext-Mutant. Der Test weist nach, dass die
//! Pruefung AN DIESER STELLE wirkt, nicht dass der echte params-Betrieb je dorthin kommt; die echten Werte sind 3.336, 1.464 und 20).
//!
//! * `kinderfreibetrag_je_elternteil` (`checked_add` zweier Euro-Werte, Fehler `Summe in i64`): die Mutante (`wrapping_add`) liefert
//!   die gewickelte Summe als Betrag.
//! * `ganzzahl` (`i64::try_from(d)` fuer eine Gleitkommazahl ohne Nachkommaanteil): `staffelgrenze_km: 10000000000000000000` ist ein
//!   `u64` ausserhalb `i64`; die Mutante (`d.mantissa() as i64`) liefert `-8446744073709551616` als Staffelgrenze.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use bindung::Params;
use domain::{Euro, Vz};

fn kopiere_params(from: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for eintrag in std::fs::read_dir(from).unwrap() {
        let eintrag = eintrag.unwrap();
        let ziel = nach.join(eintrag.file_name());
        if eintrag.file_type().unwrap().is_dir() {
            kopiere_params(&eintrag.path(), &ziel);
        } else {
            std::fs::copy(eintrag.path(), &ziel).unwrap();
        }
    }
}

/// `Params` aus einer Temp-Kopie von `params/`, in der EINE Datei an genau den gegebenen Stellen ersetzt ist. Jeder Anker muss in
/// der Originaldatei genau einmal treffen -- sonst ist der Test gegen eine geaenderte Dateiformatierung taub, und das soll er laut sein.
fn params_gepatcht(zelle: &str, datei: &str, stellen: &[(&str, &str)]) -> Params {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wurzel = std::env::temp_dir().join(format!(
        "taxgraph-h8-bindung-{zelle}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&wurzel);
    kopiere_params(&repo.join("params"), &wurzel.join("params"));
    let pfad = wurzel.join("params").join(datei);
    let mut text = std::fs::read_to_string(&pfad).unwrap();
    for (alt, neu) in stellen {
        assert_eq!(
            text.matches(*alt).count(),
            1,
            "Anker nicht einwandfrei in {datei}: {alt:?}"
        );
        text = text.replacen(alt, neu, 1);
    }
    std::fs::write(&pfad, text).unwrap();
    let p = Params::lade(&wurzel).unwrap();
    std::fs::remove_dir_all(&wurzel).unwrap();
    p
}

/// Freibetrag je Elternteil = Kinderfreibetrag + BEA-Freibetrag. `i64::MAX - 1464` und `1464` summieren sich gerade noch auf
/// `i64::MAX`; mit `i64::MAX - 1463` laeuft die Summe ueber.
#[test]
fn kinderfreibetrag_summe_ausserhalb_i64_ist_ein_fehler() {
    let datei = "2025/kinderfreibetrag_p32.yaml";
    let wert = |kinder: i64| {
        params_gepatcht(
            &format!("kfb{kinder}"),
            datei,
            &[(
                "kinderfreibetrag_je_elternteil:\n  wert: 3336\n",
                &format!("kinderfreibetrag_je_elternteil:\n  wert: {kinder}\n"),
            )],
        )
        .kinderfreibetrag_je_elternteil(Vz::Vz2025)
    };
    assert_eq!(
        wert(i64::MAX - 1464).unwrap(),
        Euro::new(i64::MAX),
        "i64::MAX - 1464 + 1464 passt gerade"
    );
    let r = wert(i64::MAX - 1463);
    assert!(
        r.as_ref()
            .is_err_and(|e| e.to_string().contains("Summe in i64")),
        "i64::MAX - 1463 + 1464: {r:?}"
    );
}

/// `staffelgrenze_km` (`ganzzahl`): `20.0` (Gleitkomma ohne Nachkommaanteil) liest `20`; `10000000000000000000` (u64 ausserhalb
/// `i64`) ist ein Fehler statt eines gewickelten Werts.
#[test]
fn ganzzahl_aus_gleitkomma_ausserhalb_i64_ist_ein_fehler() {
    let grenze = |zelle: &str, wert: &str| {
        params_gepatcht(
            zelle,
            "2025/entfernungspauschale.yaml",
            &[(
                "staffelgrenze_km:\n  wert: 20\n",
                &format!("staffelgrenze_km:\n  wert: {wert}\n"),
            )],
        )
        .entfernungspauschale(Vz::Vz2025)
        .map(|s| s.staffelgrenze_km)
    };
    assert_eq!(
        grenze("gk20", "20.0").unwrap(),
        20,
        "20.0 liest 20 (Gleitkomma-Pfad, `try_from` greift)"
    );
    let r = grenze("gk1e19", "10000000000000000000");
    assert!(r.is_err(), "1e19: {r:?}");
}
