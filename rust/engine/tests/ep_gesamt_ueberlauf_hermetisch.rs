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
//! Das Fenster ist kein Sonderfall von 366 Tagen: es gibt es bei jeder Tageszahl, stets etwa 16 km breit (`TAGE`: 1 und 2 Tage).
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
use domain::{Cent, Euro, Km, Vz};
use engine::entfernungspauschale::{
    berechnen as ep_berechnen, EntfernungspauschaleEingabe as ScopeEingabe,
};
use engine::sachverhalt::Sachverhalt;
use engine::zugriff::teil1::werbungskosten::{
    entfernungspauschale, ep_ab_21km, werbungskosten_n, EntfernungspauschaleEingabe,
    WerbungskostenNEingabe,
};
use rust_decimal::Decimal;

const UEBERLAUF: &str = r#"Err(Ueberlauf("ep_gesamt"))"#;
const SACHVERHALT_UEBERLAUF: &str = r#"Err(Entfernungspauschale(Ueberlauf("ep_gesamt")))"#;
/// Die alte Sperre der Teilprodukte in `ep_ab_21km` (schlaegt erst am Ende des Fensters an).
const AB21_ROH: &str = r#"Err(Ueberlauf("ab21_roh"))"#;

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
    (663_170_264_369_792, true, AB21_ROH),
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

/// Das Fenster gibt es bei jeder Tageszahl, stets etwa 16 km breit (Anteil bis 20 km zu Satz ab 21 km: 600 zu 38). Kfz.
/// `(Tage, km, entfernungspauschale(), ep_ab_21km())`. Bei 1 Tag laeuft die SUMME der beiden Teile ueber, bei 2 Tagen erst die
/// Multiplikation mit den Tagen: zwei verschiedene Stellen der Vorab-Pruefung.
const TAGE: &[(i64, i64, &str, &str)] = &[
    (
        1,
        242_720_316_759_336_209,
        "Ok(Euro(92233720368547757))",
        "Ok(Euro(92233720368547751))",
    ),
    (1, 242_720_316_759_336_210, UEBERLAUF, UEBERLAUF),
    (1, 242_720_316_759_336_225, UEBERLAUF, UEBERLAUF),
    (1, 242_720_316_759_336_226, UEBERLAUF, AB21_ROH),
    (
        2,
        121_360_158_379_668_106,
        "Ok(Euro(92233720368547757))",
        "Ok(Euro(92233720368547745))",
    ),
    (2, 121_360_158_379_668_107, UEBERLAUF, UEBERLAUF),
    (2, 121_360_158_379_668_122, UEBERLAUF, UEBERLAUF),
    (2, 121_360_158_379_668_123, UEBERLAUF, AB21_ROH),
];

#[test]
fn fenster_gibt_es_bei_jeder_tageszahl() {
    let p = params();
    let abweichend: Vec<String> = TAGE
        .iter()
        .flat_map(|&(tage, km, ep, ab21)| {
            let e = EntfernungspauschaleEingabe {
                arbeitstage: tage,
                ..eingabe(km, true, 0)
            };
            [
                (
                    "entfernungspauschale",
                    format!("{:?}", entfernungspauschale(&e, &p)),
                    ep,
                ),
                ("ep_ab_21km", format!("{:?}", ep_ab_21km(&e, &p)), ab21),
            ]
            .into_iter()
            .filter(|(_, ist, soll)| ist != soll)
            .map(move |(was, ist, soll)| {
                format!("{tage} Tage, km={km}, {was}: ist {ist}, soll {soll}")
            })
        })
        .collect();
    melde(&abweichend, 2 * TAGE.len());
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

/// Zweiter Zugang zum Scope: `Sachverhalt::Entfernungspauschale(..).berechnen()` ruft `entfernungspauschale::berechnen` ohne
/// `werbungskosten::entfernungspauschale`. Die Vorab-Pruefung sitzt in `berechnen` selbst, damit beide Zugaenge sie haben.
/// Vorher (Wegwerf-Test, Anlage `sachverhalt_ep.out`): `Ok(Cent(10240))` bei ...776 und `Ok(Cent(218860))` bei ...791 km.
#[test]
fn sachverhalt_entfernungspauschale_ist_ueberlauf_statt_wert_mod_2_63() {
    let berechne = |km: i64, kfz: bool| {
        format!(
            "{:?}",
            Sachverhalt::Entfernungspauschale(ScopeEingabe {
                entfernung_km_roh: Km::new(Decimal::from(km)),
                arbeitstage: 366,
                eigenes_oder_ueberlassenes_kfz: kfz,
                oepnv_kosten_jahr: Cent::new(0),
                satz_bis_20_km: Cent::new(30),
                satz_ab_21_km: Cent::new(38),
                staffelgrenze_km: 20,
                hoechstbetrag: Cent::new(450_000),
            })
            .berechnen()
        )
    };
    let faelle = [
        (663_170_264_369_775, true, "Ok(Cent(9223372036854772140))"),
        (663_170_264_369_776, true, SACHVERHALT_UEBERLAUF),
        (663_170_264_369_791, true, SACHVERHALT_UEBERLAUF),
        (663_170_264_369_792, true, SACHVERHALT_UEBERLAUF),
        (663_170_264_369_776, false, "Ok(Cent(450000))"),
    ];
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|&(km, kfz, soll)| {
            let ist = berechne(km, kfz);
            (ist != soll).then(|| format!("km={km} kfz={kfz}: ist {ist}, soll {soll}"))
        })
        .collect();
    melde(&abweichend, faelle.len());
}

/// Mutant V1 (`.all(|x| *x >= 0)` -> `> 0` in `gesamt_pruefen`, Messung von main): ohne Kfz ist ein Faktor 0 (km <= Grenze, Satz 0, Grenze 0)
/// KEIN Grund fuer einen Fehler. Laeuft der Jahresbetrag nur durch `arbeitstage` (ein freies `i64`) ueber `i64`, deckelt der Scope ihn
/// ohne Kfz auf den Hoechstbetrag, bevor der Shim liest: Ok mit 450000 ct, Python gleich. Mit Kfz gibt der Scope den Betrag aus: Ueberlauf.
/// `(Name, km, Tage, Kfz, Satz bis 20 km, Satz ab 21 km, Staffelgrenze, erwartet)`; Satz in Cent. Aufruf von `entfernungspauschale::berechnen`
/// mit eigenen Saetzen (`werbungskosten::entfernungspauschale` nimmt sie aus den Parametern, dort ist keiner 0).
/// HERKUNFT: Python-Orakel `orakel_v1.py` -> `orakel_v1.out` (Catala-Scope `EP.berechnung` direkt, exakte Ganzzahlen): ohne Kfz in allen vier
/// Faellen `entfernungspauschale_ct` = `abziehbarer_ct` = 450000; mit Kfz 2767011611056432742100, 5534023222112865484200,
/// 28039050992038518453280, 35048813740048148066600 (alle ausserhalb `i64`).
#[allow(clippy::type_complexity)]
const NULLFAKTOR: &[(&str, i64, i64, bool, i64, i64, i64, &str)] = &[
    (
        "A: ueber = 0 (km 10 <= Grenze)",
        10,
        i64::MAX,
        false,
        30,
        38,
        20,
        "Ok((450000, 450000))",
    ),
    (
        "B: Satz ab 21 km = 0",
        100,
        i64::MAX,
        false,
        30,
        0,
        20,
        "Ok((450000, 450000))",
    ),
    (
        "C: Satz bis 20 km = 0",
        100,
        i64::MAX,
        false,
        0,
        38,
        20,
        "Ok((450000, 450000))",
    ),
    (
        "D: Staffelgrenze 0 (bis = 0)",
        100,
        i64::MAX,
        false,
        30,
        38,
        0,
        "Ok((450000, 450000))",
    ),
    // mit Kfz derselbe Fall: der Scope gibt den Betrag ungedeckelt aus, er passt nicht in i64
    ("A mit Kfz", 10, i64::MAX, true, 30, 38, 20, ENG_UEBERLAUF),
    ("B mit Kfz", 100, i64::MAX, true, 30, 0, 20, ENG_UEBERLAUF),
    ("C mit Kfz", 100, i64::MAX, true, 0, 38, 20, ENG_UEBERLAUF),
    ("D mit Kfz", 100, i64::MAX, true, 30, 38, 0, ENG_UEBERLAUF),
    // Kontrollen ohne Ueberlauf: unveraendert Ok (Python 60000 und 120000 ct)
    (
        "A, 200 Tage",
        10,
        200,
        false,
        30,
        38,
        20,
        "Ok((60000, 60000))",
    ),
    (
        "B, 200 Tage",
        100,
        200,
        false,
        30,
        0,
        20,
        "Ok((120000, 120000))",
    ),
];
const ENG_UEBERLAUF: &str = r#"Err(Ueberlauf("ep_gesamt"))"#;

#[test]
fn ohne_kfz_ist_ein_nullfaktor_kein_grund_fuer_einen_fehler() {
    let abweichend: Vec<String> = NULLFAKTOR
        .iter()
        .filter_map(|&(name, km, tage, kfz, satz_bis, satz_ab, grenze, soll)| {
            let ist = format!(
                "{:?}",
                ep_berechnen(ScopeEingabe {
                    entfernung_km_roh: Km::new(Decimal::from(km)),
                    arbeitstage: tage,
                    eigenes_oder_ueberlassenes_kfz: kfz,
                    oepnv_kosten_jahr: Cent::new(0),
                    satz_bis_20_km: Cent::new(satz_bis),
                    satz_ab_21_km: Cent::new(satz_ab),
                    staffelgrenze_km: grenze,
                    hoechstbetrag: Cent::new(450_000),
                })
                .map(|e| (e.entfernungspauschale_cent, e.abziehbarer_betrag_cent))
            );
            (ist != soll).then(|| format!("{name}: ist {ist}, soll {soll}"))
        })
        .collect();
    melde(&abweichend, NULLFAKTOR.len());
}
