//! Der C-Shim liest Scope-Ausgaben mit `mpz_get_si`; das gibt bei einem Wert ausserhalb `long` still dessen untere 63 Bit zurueck
//! (Rust allein, hermetisch, ohne `PARITY=1`, ohne Python). Bericht h8-shim-guard.
//!
//! ERWARTUNG: Eine GELESENE Ausgabe, die nicht in `i64` passt, ist `Err(CatalaFehler::Ueberlauf(<Ausgabe>))` -- an einer Stelle, dem
//! Makro `TG_AUS` im Shim, nicht je Scope. Passt der Wert, bleibt es `Ok` mit dem Orakelwert.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Python-Orakel `orakel_guard.py` -> `orakel_guard.out` (Catala-Python-Runtime direkt, exakte Ganzzahlen,
//! dieselben ganzen Euro als Eingabe). `C` = 9223372036854775800 ct (92233720368547758 EUR), der groesste ganze Euro-Betrag in `i64`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use catala_sys::{
    agb_abzug, altersentlastungsbetrag, entfernungspauschale, entlastungsbetrag,
    ermaessigter_durchschnittssatz, euer_gewinn, familienleistungsausgleich,
    festzusetzende_est_einzel_voll, festzusetzende_est_gesamt, festzusetzende_est_gesamt_zusammen,
    kirchensteuerabzug, mitunternehmer_einkuenfte, raumkostenabzug, verbilligte_vermietung_wk,
    CatalaFehler, EntfernungspauschaleEingabe, FestzusetzendeEstErgebnis, GesamtEingabe,
    RaumkostenabzugEingabe, Vz,
};

const C: i64 = 9_223_372_036_854_775_800;

/// Ergebnis einer gelesenen Ausgabe.
type Erg = Result<i64, CatalaFehler>;

fn ueberlauf(ausgabe: &'static str) -> Erg {
    Err(CatalaFehler::Ueberlauf(ausgabe))
}

/// Meldet alle abweichenden Faelle einer Tabelle auf einmal.
fn melde(faelle: &[(&str, Erg, Erg)]) {
    let abweichend: Vec<String> = faelle
        .iter()
        .filter(|(_, ist, soll)| ist != soll)
        .map(|(name, ist, soll)| format!("{name}: ist {ist:?}, soll {soll:?}"))
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen ab:\n{}",
        abweichend.len(),
        faelle.len(),
        abweichend.join("\n")
    );
}

#[test]
fn euer_gewinn_ausserhalb_i64_ist_ueberlauf() {
    // Orakel: C - (-C) = 18446744073709551600 (nicht in i64); C - 0 = 9223372036854775800.
    melde(&[
        (
            "C - (-C)",
            euer_gewinn(C, -C),
            ueberlauf("EuerGewinn__gewinn"),
        ),
        (
            "-C - C",
            euer_gewinn(-C, C),
            ueberlauf("EuerGewinn__gewinn"),
        ),
        ("C - 0 passt", euer_gewinn(C, 0), Ok(C)),
    ]);
}

/// Genau an der Grenze (Cent, nicht auf ganze Euro gerundet): `i64::MAX` und `i64::MIN` passen, eins darueber oder darunter nicht.
/// Orakel `orakel_guard.out`, Zeilen `grenze_*`: (MAX, 0) -> 9223372036854775807; (MIN, 0) -> -9223372036854775808; (MAX, -1) ->
/// 9223372036854775808 (ausserhalb); (MIN, 1) -> -9223372036854775809 (ausserhalb); (0, MIN) -> 9223372036854775808 (ausserhalb);
/// (-1, MIN) -> 9223372036854775807; (0, MAX) -> -9223372036854775807; (-1, MAX) -> -9223372036854775808.
#[test]
fn euer_gewinn_genau_an_der_i64_grenze() {
    let m = "EuerGewinn__gewinn";
    melde(&[
        ("MAX - 0 passt", euer_gewinn(i64::MAX, 0), Ok(i64::MAX)),
        ("MIN - 0 passt", euer_gewinn(i64::MIN, 0), Ok(i64::MIN)),
        (
            "MAX - (-1) = MAX + 1",
            euer_gewinn(i64::MAX, -1),
            ueberlauf(m),
        ),
        ("MIN - 1 = MIN - 1", euer_gewinn(i64::MIN, 1), ueberlauf(m)),
        ("0 - MIN = MAX + 1", euer_gewinn(0, i64::MIN), ueberlauf(m)),
        (
            "-1 - MIN = MAX passt",
            euer_gewinn(-1, i64::MIN),
            Ok(i64::MAX),
        ),
        ("0 - MAX passt", euer_gewinn(0, i64::MAX), Ok(-i64::MAX)),
        (
            "-1 - MAX = MIN passt",
            euer_gewinn(-1, i64::MAX),
            Ok(i64::MIN),
        ),
    ]);
}

#[test]
fn mitunternehmer_einkuenfte_ausserhalb_i64_ist_ueberlauf() {
    // Orakel: C + C = 18446744073709551600; C allein 9223372036854775800.
    melde(&[
        (
            "C + C",
            mitunternehmer_einkuenfte(C, C, 0, 0),
            ueberlauf("MitunternehmerEinkuenfte__einkuenfte_mitunternehmer"),
        ),
        (
            "C allein passt",
            mitunternehmer_einkuenfte(C, 0, 0, 0),
            Ok(C),
        ),
        (
            "C + C - C - C = 0 passt (Teilsumme ausserhalb, Ergebnis nicht)",
            mitunternehmer_einkuenfte(C, C, -C, -C),
            Ok(0),
        ),
    ]);
}

#[test]
fn kirchensteuerabzug_ausserhalb_i64_ist_ueberlauf() {
    // Orakel: C - (-C) = 18446744073709551600; C - 0 = 9223372036854775800.
    melde(&[
        (
            "gezahlt C, erstattet -C",
            kirchensteuerabzug(C, -C),
            ueberlauf("Kirchensteuerabzug__abziehbare_kirchensteuer"),
        ),
        (
            "gezahlt C, erstattet 0 passt",
            kirchensteuerabzug(C, 0),
            Ok(C),
        ),
    ]);
}

#[test]
fn agb_abzug_ausserhalb_i64_ist_ueberlauf() {
    // Orakel: C - (-C) = 18446744073709551600; C - 0 = 9223372036854775800.
    melde(&[
        (
            "Aufwendungen C, zumutbar -C",
            agb_abzug(C, -C),
            ueberlauf("AgbAbzug__abzug_agb"),
        ),
        ("Aufwendungen C, zumutbar 0 passt", agb_abzug(C, 0), Ok(C)),
    ]);
}

#[test]
fn entlastungsbetrag_ausserhalb_i64_ist_ueberlauf() {
    // Orakel: 400000000000000 Kinder -> 9600000000000402000; 2 Kinder -> 450000.
    melde(&[
        (
            "400000000000000 Kinder",
            entlastungsbetrag(true, 400_000_000_000_000, 0),
            ueberlauf("Entlastungsbetrag__entlastungsbetrag"),
        ),
        ("2 Kinder passt", entlastungsbetrag(true, 2, 0), Ok(450_000)),
    ]);
}

#[test]
fn familienleistungsausgleich_ausserhalb_i64_ist_ueberlauf() {
    // Orakel: (C, -C, -C) -> -18446744073709551600; (C, 0, 0) -> 0.
    melde(&[
        (
            "est_mit -C, Kindergeld -C",
            familienleistungsausgleich(C, -C, -C),
            ueberlauf("Familienleistungsausgleich__est_nach_familienausgleich"),
        ),
        (
            "(C, 0, 0) passt",
            familienleistungsausgleich(C, 0, 0),
            Ok(0),
        ),
    ]);
}

#[test]
fn verbilligte_vermietung_ausserhalb_i64_ist_ueberlauf() {
    // Orakel: WK C, Quote -1000 % -> -92233720368547758000; Quote 50 % -> 4611686018427387900.
    melde(&[
        (
            "Quote -1000 %",
            verbilligte_vermietung_wk(C, -1000, 1),
            ueberlauf("VerbilligteVermietungWk__abziehbare_werbungskosten"),
        ),
        (
            "Quote 50 % passt",
            verbilligte_vermietung_wk(C, 50, 1),
            Ok(4_611_686_018_427_387_900),
        ),
    ]);
}

#[test]
fn ermaessigter_durchschnittssatz_ausserhalb_i64_ist_ueberlauf() {
    // Orakel (Eingaben in Cent): (ao 500000000, est C, bmg 100) -> 25825441703193372240000000; Kontrolle (10000000, 2000000, 10000000) -> 1400000.
    melde(&[
        (
            "bmg klein",
            ermaessigter_durchschnittssatz(500_000_000, C, 100),
            ueberlauf("ErmaessigterDurchschnittssatz__est_ao"),
        ),
        (
            "Kontrolle passt",
            ermaessigter_durchschnittssatz(10_000_000, 2_000_000, 10_000_000),
            Ok(1_400_000),
        ),
    ]);
}

fn ep(
    km: i64,
    tage: i64,
    kfz: bool,
    satz_bis: i64,
) -> (Result<i64, CatalaFehler>, Result<i64, CatalaFehler>) {
    let e = entfernungspauschale(EntfernungspauschaleEingabe {
        entfernung_km_roh_num: km,
        entfernung_km_roh_den: 1,
        arbeitstage: tage,
        eigenes_oder_ueberlassenes_kfz: kfz,
        oepnv_kosten_jahr_cent: 0,
        satz_bis_20_km_cent: satz_bis,
        satz_ab_21_km_cent: 38,
        staffelgrenze_km: 20,
        hoechstbetrag_cent: 450_000,
    })
    .unwrap();
    (e.entfernungspauschale_cent(), e.abziehbarer_betrag_cent())
}

/// Zwei Ausgaben eines Scopes: jede wird fuer sich geprueft, erst beim Lesen (lazy). Nur die zweite ist (je nach Fall) ausserhalb `i64`.
/// Orakel (`orakel_guard.out`, `ep_*`): Kfz, 366 Tage, 663170264369776 km -> beide 9223372036854786048 (ausserhalb); ...775 km -> beide
/// 9223372036854772140; ohne Kfz -> beide 450000; negative Tage (10 km, Kfz oder nicht) -> `entfernungspauschale` -2767011611056432742100
/// (ausserhalb), `abziehbarer_betrag` 0; Satz bis 20 km -30, Tage `i64::MAX`: ohne Kfz beide 450000, mit Kfz beide 22505027769925652969080.
#[test]
fn entfernungspauschale_prueft_jede_ausgabe_fuer_sich() {
    let (e, a) = (
        "Entfernungspauschale__entfernungspauschale",
        "Entfernungspauschale__abziehbarer_betrag",
    );
    let neg = -i64::MAX;
    let (e776, a776) = ep(663_170_264_369_776, 366, true, 30);
    let (e775, a775) = ep(663_170_264_369_775, 366, true, 30);
    let (eo, ao) = ep(663_170_264_369_776, 366, false, 30);
    let (en, an) = ep(10, neg, true, 30);
    let (eno, ano) = ep(10, neg, false, 30);
    let (es, as_) = ep(100, i64::MAX, false, -30);
    let (esk, ask) = ep(100, i64::MAX, true, -30);
    melde(&[
        ("Kfz ...776 km, entfernungspauschale", e776, ueberlauf(e)),
        ("Kfz ...776 km, abziehbarer_betrag", a776, ueberlauf(a)),
        (
            "Kfz ...775 km, entfernungspauschale",
            e775,
            Ok(9_223_372_036_854_772_140),
        ),
        (
            "Kfz ...775 km, abziehbarer_betrag",
            a775,
            Ok(9_223_372_036_854_772_140),
        ),
        ("ohne Kfz ...776 km, entfernungspauschale", eo, Ok(450_000)),
        ("ohne Kfz ...776 km, abziehbarer_betrag", ao, Ok(450_000)),
        (
            "negative Tage, Kfz: entfernungspauschale (ungelesen ausserhalb)",
            en,
            ueberlauf(e),
        ),
        (
            "negative Tage, Kfz: abziehbarer_betrag (lazy: passt)",
            an,
            Ok(0),
        ),
        (
            "negative Tage, ohne Kfz: entfernungspauschale",
            eno,
            ueberlauf(e),
        ),
        (
            "negative Tage, ohne Kfz: abziehbarer_betrag (lazy: passt)",
            ano,
            Ok(0),
        ),
        (
            "Satz bis 20 km -30, ohne Kfz: entfernungspauschale",
            es,
            Ok(450_000),
        ),
        (
            "Satz bis 20 km -30, ohne Kfz: abziehbarer_betrag",
            as_,
            Ok(450_000),
        ),
        (
            "Satz bis 20 km -30, Kfz: entfernungspauschale",
            esk,
            ueberlauf(e),
        ),
        (
            "Satz bis 20 km -30, Kfz: abziehbarer_betrag",
            ask,
            ueberlauf(a),
        ),
    ]);
}

/// Die sechs Ausgaben der Einkommensteuertarif-Scopes in der Reihenfolge des Orakels: Summe der Einkuenfte, Gesamtbetrag, Einkommen,
/// zvE, tarifliche `ESt`, festzusetzende `ESt`.
fn est_sechs(e: &FestzusetzendeEstErgebnis) -> [Erg; 6] {
    [
        e.summe_der_einkuenfte_cent(),
        e.gesamtbetrag_der_einkuenfte_cent(),
        e.einkommen_cent(),
        e.zu_versteuerndes_einkommen_cent(),
        e.tarifliche_est_cent(),
        e.festzusetzende_est_cent(),
    ]
}

const EST_MARKEN: [&str; 6] = [
    "Einkommensteuertarif__summe_der_einkuenfte",
    "Einkommensteuertarif__gesamtbetrag_der_einkuenfte",
    "Einkommensteuertarif__einkommen",
    "Einkommensteuertarif__zu_versteuerndes_einkommen",
    "Einkommensteuertarif__tarifliche_est",
    "Einkommensteuertarif__festzusetzende_est",
];

/// `Some(x)`: die Ausgabe passt und ist `x`; `None`: sie liegt ausserhalb `i64` (Ueberlauf mit der Marke dieser Ausgabe).
fn est_soll(soll: [Option<i64>; 6]) -> [Erg; 6] {
    let mut aus = EST_MARKEN.map(ueberlauf);
    for (a, s) in aus.iter_mut().zip(soll) {
        if let Some(x) = s {
            *a = Ok(x);
        }
    }
    aus
}

fn melde_est(name: &str, ist: &FestzusetzendeEstErgebnis, soll: [Option<i64>; 6]) {
    let namen = [
        "Summe",
        "Gesamtbetrag",
        "Einkommen",
        "zvE",
        "tarifliche",
        "festzusetzende",
    ];
    let faelle: Vec<(String, Erg, Erg)> = namen
        .iter()
        .zip(est_sechs(ist))
        .zip(est_soll(soll))
        .map(|((n, i), s)| (format!("{name}, {n}"), i, s))
        .collect();
    let faelle: Vec<(&str, Erg, Erg)> = faelle
        .iter()
        .map(|(n, i, s)| (n.as_str(), *i, *s))
        .collect();
    melde(&faelle);
}

/// Einzel, volle Ausgabe: Bruttoarbeitslohn -C und Werbungskosten C. Summe, Gesamtbetrag, Einkommen und zvE sind -18446744073709551600 bzw.
/// -18446744073709555200 (ausserhalb `i64`); tarifliche und festzusetzende `ESt` sind 0 und werden OHNE Fehler gelesen (lazy). Kontrolle:
/// Bruttoarbeitslohn C allein, alles passt. Orakel `orakel_guard.out`: `einzel_brutto_negC_wk_C`, `einzel_brutto_C`.
#[test]
fn est_einzel_voll_prueft_jede_ausgabe_fuer_sich() {
    melde_est(
        "Brutto -C, WK C",
        &festzusetzende_est_einzel_voll(-C, C, 0, Vz::Vz2025).unwrap(),
        [None, None, None, None, Some(0), Some(0)],
    );
    melde_est(
        "Brutto C",
        &festzusetzende_est_einzel_voll(C, 0, 0, Vz::Vz2025).unwrap(),
        [
            Some(9_223_372_036_854_652_800),
            Some(9_223_372_036_854_652_800),
            Some(9_223_372_036_854_649_200),
            Some(9_223_372_036_854_649_200),
            Some(4_150_517_416_582_667_400),
            Some(4_150_517_416_582_667_400),
        ],
    );
}

fn gesamt(
    nichtselbst: i64,
    gewinn: i64,
    sonstige: i64,
    vermietung: i64,
    kapital: i64,
) -> GesamtEingabe {
    GesamtEingabe {
        einkuenfte_nichtselbststaendig_cent: nichtselbst,
        einkuenfte_kapitalvermoegen_cent: kapital,
        einkuenfte_vermietung_cent: vermietung,
        einkuenfte_sonstige_cent: sonstige,
        einkuenfte_gewinn_cent: gewinn,
        altersentlastungsbetrag_cent: 0,
        entlastungsbetrag_alleinerziehende_cent: 0,
        sonderausgaben_cent: 0,
        aussergewoehnliche_belastungen_cent: 0,
        freibetraege_kinder_cent: 0,
        sonstige_abzuege_vom_einkommen_cent: 0,
        anzurechnende_auslaendische_steuern_cent: 0,
        steuerermaessigungen_cent: 0,
        steuer_kapital_gesondert_cent: 0,
        hinzurechnung_kindergeld_cent: 0,
        hinzurechnung_zulage_cent: 0,
        tarif_modifiziert: false,
        tarifliche_est_modifiziert_cent: 0,
    }
}

/// Gesamt-Scopes (Einzel und Zusammen): zwei Einkuenfte zu je C -> Summe, Gesamtbetrag, Einkommen und zvE sind 18446744073709551600
/// (ausserhalb `i64`), tarifliche und festzusetzende `ESt` (8301034833167373500 einzel, 8301034833165448800 zusammen) passen und werden
/// OHNE Fehler gelesen. Fuenf Einkuenfte zu je C: auch die Steuer (20752587082921320800 einzel) liegt ausserhalb. Kontrolle: eine Einkunftsart
/// zu C, alles passt. Orakel `orakel_guard.out`: `gesamt_zwei`, `gesamt_zwei_zusammen`, `gesamt_fuenf`, `gesamt_kontrolle`.
#[test]
fn est_gesamt_prueft_jede_ausgabe_fuer_sich() {
    melde_est(
        "Gesamt zwei",
        &festzusetzende_est_gesamt(gesamt(C, C, 0, 0, 0), Vz::Vz2025).unwrap(),
        [
            None,
            None,
            None,
            None,
            Some(8_301_034_833_167_373_500),
            Some(8_301_034_833_167_373_500),
        ],
    );
    melde_est(
        "Gesamt zusammen zwei",
        &festzusetzende_est_gesamt_zusammen(gesamt(C, C, 0, 0, 0), Vz::Vz2025).unwrap(),
        [
            None,
            None,
            None,
            None,
            Some(8_301_034_833_165_448_800),
            Some(8_301_034_833_165_448_800),
        ],
    );
    melde_est(
        "Gesamt fuenf",
        &festzusetzende_est_gesamt(gesamt(C, C, C, C, C), Vz::Vz2025).unwrap(),
        [None, None, None, None, None, None],
    );
    melde_est(
        "Gesamt Kontrolle",
        &festzusetzende_est_gesamt(gesamt(C, 0, 0, 0, 0), Vz::Vz2025).unwrap(),
        [
            Some(C),
            Some(C),
            Some(C),
            Some(C),
            Some(4_150_517_416_582_724_400),
            Some(4_150_517_416_582_724_400),
        ],
    );
}

/// Altersentlastungsbetrag: `min(Prozentsatz / 100 * (Arbeitslohn + positive andere Einkuenfte), Hoechstbetrag)`. Bei einer negativen
/// Bemessungsgrundlage ausserhalb `i64` bleibt der Rohbetrag unter dem Hoechstbetrag und kommt ungedeckelt heraus.
/// Orakel `orakel_guard.out`: `ae_negC_negC_100` (-C, -C, 100 %, Hoechstbetrag 76000 ct) -> -18446744073709551600; `ae_kontrolle`
/// (200000, 100000, 20 %, 76000) -> 60000.
#[test]
fn altersentlastungsbetrag_ausserhalb_i64_ist_ueberlauf() {
    melde(&[
        (
            "Bemessung -2C",
            altersentlastungsbetrag(-C, -C, 100, 1, 76_000),
            ueberlauf("Altersentlastungsbetrag__altersentlastungsbetrag"),
        ),
        (
            "Kontrolle passt",
            altersentlastungsbetrag(200_000, 100_000, 20, 1, 76_000),
            Ok(60_000),
        ),
    ]);
}

fn raum(
    vorhanden: bool,
    mittelpunkt: bool,
    aufwendungen: i64,
    pauschale_gewaehlt: bool,
    monate: i64,
    tage: i64,
) -> [Result<i64, CatalaFehler>; 3] {
    let r = raumkostenabzug(RaumkostenabzugEingabe {
        arbeitszimmer_vorhanden: vorhanden,
        ist_mittelpunkt: mittelpunkt,
        tatsaechliche_aufwendungen_cent: aufwendungen,
        jahrespauschale_gewaehlt: pauschale_gewaehlt,
        monate_ohne_mittelpunkt: monate,
        homeoffice_tage: tage,
        jahrespauschale_cent: 126_000,
        tagespauschale_pro_tag_cent: 600,
        tagespauschale_hoechstbetrag_cent: 126_000,
    })
    .unwrap();
    [
        r.abzug_arbeitszimmer_cent(),
        r.abzug_homeoffice_cent(),
        r.abzug_gesamt_cent(),
    ]
}

/// Raumkostenabzug, drei Ausgaben, jede fuer sich. Jahrespauschale 1260 EUR, je Monat ohne Mittelpunkt um 1/12 gekuerzt: bei 10^15
/// Monaten (ausserhalb der 0..12 der Regel) wird `126000 - 126000 * 10^15 / 12` zu -10499999999999874000, das Homeoffice bleibt 0 und
/// passt (lazy). Homeoffice: -10^17 Tage zu 600 ct ergeben -60000000000000000000, das Arbeitszimmer bleibt 0. Orakel `orakel_guard.out`:
/// `rk_jahrespauschale_monate_1e15`, `rk_homeoffice_tage_neg1e17`, `rk_kontrolle` (Mittelpunkt, 500000 ct, 100 Tage -> 500000 / 0 / 500000).
#[test]
fn raumkostenabzug_prueft_jede_ausgabe_fuer_sich() {
    let (az, ho, ge) = (
        "ArbeitszimmerHomeoffice__abzug_arbeitszimmer",
        "ArbeitszimmerHomeoffice__abzug_homeoffice",
        "ArbeitszimmerHomeoffice__abzug_gesamt",
    );
    let [a1, h1, g1] = raum(true, false, 0, true, 1_000_000_000_000_000, 0);
    let [a2, h2, g2] = raum(false, false, 0, false, 0, -100_000_000_000_000_000);
    let [a3, h3, g3] = raum(true, true, 500_000, false, 0, 100);
    melde(&[
        ("Pauschale 10^15 Monate: Arbeitszimmer", a1, ueberlauf(az)),
        (
            "Pauschale 10^15 Monate: Homeoffice (lazy: passt)",
            h1,
            Ok(0),
        ),
        ("Pauschale 10^15 Monate: Gesamt", g1, ueberlauf(ge)),
        ("Tage -10^17: Arbeitszimmer (lazy: passt)", a2, Ok(0)),
        ("Tage -10^17: Homeoffice", h2, ueberlauf(ho)),
        ("Tage -10^17: Gesamt", g2, ueberlauf(ge)),
        ("Kontrolle: Arbeitszimmer", a3, Ok(500_000)),
        ("Kontrolle: Homeoffice", h3, Ok(0)),
        ("Kontrolle: Gesamt", g3, Ok(500_000)),
    ]);
}
