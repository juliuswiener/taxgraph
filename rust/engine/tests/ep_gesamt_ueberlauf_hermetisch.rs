//! Entfernungspauschale: der Jahresbetrag in Cent passt nicht in `i64`, der Catala-Scope rechnet ihn aber (GMP) ohne Fehler aus
//! (Rust allein, hermetisch, ohne `PARITY=1`, ohne Python). Bericht h8-ep-fenster.
//!
//! ANLASS: `entfernungspauschale()` rief den Scope ohne Pruefung des Gesamtbetrags auf. Der Scope liefert den Betrag exakt, der
//! C-Shim (`catala-sys/csrc/shim.c`, `tg_entfernungspauschale`) liest ihn mit `mpz_get_si` -- und das gibt bei einem Wert ausserhalb
//! `long` still die unteren 63 Bit zurueck (Wert mod 2^63), keinen Fehler. Bei 366 Tagen und Kfz ist der Betrag ab
//! 663170264369776 km ausserhalb `i64`: dort lieferte Rust `Ok(Euro(102))` statt `92233720368547860`, bei 663170264369791 km
//! `Ok(Euro(2188))` statt `92233720368549946`. `ep_ab_21km` fing nur die Teilprodukte `ab21_roh` und `ep_bis20` ab; die schlagen
//! erst ab 663170264369792 km an, es blieb ein Fenster von 16 km (366 Tage), in dem die HTTP-Haut 200 mit falscher Zahl antwortete.
//!
//! ERWARTUNG: Wo der Jahresbetrag nicht in `i64` passt UND der Scope ihn ausgibt (Kfz: kein Hoechstbetrag), meldet Rust
//! `EngineFehler::Ueberlauf("ep_gesamt")`. Ohne Kfz deckelt der Scope den Betrag auf den Hoechstbetrag, bevor der Shim ihn liest;
//! das Ergebnis ist exakt (Python liefert dasselbe), also bleibt es ein Wert -- ein Fehler waere hier eine Ablehnung einer
//! richtigen Rechnung.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: `runner.catala_entfernungspauschale` / `runner.catala_ep_ab_21km` (Python, exakte Ganzzahlen),
//! VZ 2025, 366 Arbeitstage, Skript und Ausgabe `orakel_ep_k.py` / `orakel_ep_k.out` in den Anlagen zum Bericht. Die Grenzen
//! folgen aus 366 * (20 * 30 + (km - 20) * 38) Cent gegen `i64::MAX`: bei ...775 km 9223372036854772140 (passt), bei ...776 km
//! 9223372036854786048 (passt nicht).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::path::Path;

use bindung::Params;
use domain::{Euro, Km, Vz};
use engine::zugriff::teil1::werbungskosten::{
    entfernungspauschale, ep_ab_21km, werbungskosten_n, EntfernungspauschaleEingabe,
    WerbungskostenNEingabe,
};
use rust_decimal::Decimal;

const UEBERLAUF: &str = r#"Err(Ueberlauf("ep_gesamt"))"#;

fn params() -> Params {
    Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn eingabe(km: i64, kfz: bool, oepnv: i64) -> EntfernungspauschaleEingabe {
    EntfernungspauschaleEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        entfernung_km_roh: Km::new(Decimal::from(km)),
        arbeitstage: 366,
        eigenes_oder_ueberlassenes_kfz: kfz,
        oepnv_kosten_jahr: Euro::new(oepnv),
    }
}

fn wert(python: i128) -> String {
    format!("Ok(Euro({python}))")
}

/// Meldet alle abweichenden Faelle einer Tabelle auf einmal.
fn melde(abweichend: &[String], n: usize) {
    assert!(
        abweichend.is_empty(),
        "{} von {n} Faellen weichen ab:\n{}",
        abweichend.len(),
        abweichend.join("\n")
    );
}

/// `(km, Kfz, OePNV-Kosten in EUR, erwartet)`: `entfernungspauschale()`, 366 Tage.
const EP: &[(i64, bool, i64, Option<i128>)] = &[
    // Kfz: ...775 passt gerade noch (Python 92233720368547721); ab ...776 passt der Jahresbetrag nicht in i64.
    (663_170_264_369_775, true, 0, Some(92_233_720_368_547_721)),
    (663_170_264_369_776, true, 0, None),
    (663_170_264_369_791, true, 0, None),
    (663_170_264_369_792, true, 0, None),
    (1_000_000_000_000_000, true, 0, None),
    (i64::MAX, true, 0, None),
    // OePNV-Kosten (hoechstens i64::MAX / 100 EUR) ersetzen den Betrag nicht: er ist groesser, also steht er im Ergebnis.
    (663_170_264_369_776, true, 1_000_000_000_000, None),
    // ohne Kfz deckelt der Scope auf den Hoechstbetrag 4500 EUR, bevor der Shim liest: exakt, Python 4500.
    (663_170_264_369_775, false, 0, Some(4_500)),
    (663_170_264_369_776, false, 0, Some(4_500)),
    (663_170_264_369_791, false, 0, Some(4_500)),
    (663_170_264_369_792, false, 0, Some(4_500)),
    (1_000_000_000_000_000, false, 0, Some(4_500)),
    (i64::MAX, false, 0, Some(4_500)),
];

#[test]
fn entfernungspauschale_jahresbetrag_ausserhalb_i64_ist_ueberlauf_statt_wert_mod_2_63() {
    let p = params();
    let abweichend: Vec<String> = EP
        .iter()
        .filter_map(|&(km, kfz, oepnv, erwartet)| {
            let soll = erwartet.map_or_else(|| UEBERLAUF.to_string(), wert);
            let ist = format!("{:?}", entfernungspauschale(&eingabe(km, kfz, oepnv), &p));
            (ist != soll)
                .then(|| format!("km={km} kfz={kfz} oepnv={oepnv}: ist {ist}, soll {soll}"))
        })
        .collect();
    melde(&abweichend, EP.len());
}

/// `(km, Kfz, erwartet)`: `ep_ab_21km()`, 366 Tage. Das Fenster 776..791 ist der Befund; ab ...792 meldet `ab21_roh` (alte Sperre).
const AB21: &[(i64, bool, &str)] = &[
    (663_170_264_369_775, true, "Ok(Euro(92233720368545525))"),
    (663_170_264_369_776, true, UEBERLAUF),
    (663_170_264_369_791, true, UEBERLAUF),
    (663_170_264_369_792, true, r#"Err(Ueberlauf("ab21_roh"))"#),
    // ohne Kfz: Rest = 4500 - 2196 EUR = 2304 EUR (Python), im Fenster kein Fehler.
    (663_170_264_369_775, false, "Ok(Euro(2304))"),
    (663_170_264_369_776, false, "Ok(Euro(2304))"),
    (663_170_264_369_791, false, "Ok(Euro(2304))"),
];

#[test]
fn ep_ab_21km_im_fenster_776_bis_791_ist_ueberlauf_statt_still_falsch() {
    let p = params();
    let abweichend: Vec<String> = AB21
        .iter()
        .filter_map(|&(km, kfz, soll)| {
            let ist = format!("{:?}", ep_ab_21km(&eingabe(km, kfz, 0), &p));
            (ist != soll).then(|| format!("km={km} kfz={kfz}: ist {ist}, soll {soll}"))
        })
        .collect();
    melde(&abweichend, AB21.len());
}

#[test]
fn werbungskosten_n_mit_ep_ausserhalb_i64_ist_ueberlauf() {
    let p = params();
    let wk = |km: i64, kfz: bool| {
        let e = WerbungskostenNEingabe {
            entfernung: Some(eingabe(km, kfz, 0)),
            doppelte_haushaltsfuehrung: None,
            verpflegung: None,
            uebernachtung: None,
            am_anschaffungskosten: None,
        };
        format!("{:?}", werbungskosten_n(&e, &p))
    };
    let faelle = [
        (663_170_264_369_775, true, "Ok(Euro(92233720368547721))"),
        (663_170_264_369_776, true, UEBERLAUF),
        (663_170_264_369_791, true, UEBERLAUF),
        (663_170_264_369_776, false, "Ok(Euro(4500))"),
    ];
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|&(km, kfz, soll)| {
            let ist = wk(km, kfz);
            (ist != soll).then(|| format!("km={km} kfz={kfz}: ist {ist}, soll {soll}"))
        })
        .collect();
    melde(&abweichend, faelle.len());
}
