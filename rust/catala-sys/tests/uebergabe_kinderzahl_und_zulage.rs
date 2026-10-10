//! Zwei Felder der Uebergabe Rust -> Catala, die vor diesem Test kein Rust-Test sah
//! (Mutanten M06 und M10 der Catala-Pruefer-Messung vom 2026-10-10 ueberlebten 255 Tests in
//! `catala-sys` und `engine` und 2.547 im Workspace ohne `parity`; nur der Python-Vergleich in
//! `rust/parity` fing sie):
//!
//! 1. `anzahl_kinder` der zumutbaren Belastung (`shim.c`, `tg_zumutbare_belastung`): ab drei
//!    Kindern gilt die niedrigste Zeile von § 33 Abs. 3 `EStG`.
//! 2. `hinzurechnung_zulage` der festzusetzenden Einkommensteuer (`GesamtEingabe`,
//!    `shim.c` Einzel- und Zusammenpfad): § 2 Abs. 6 Satz 2 `EStG`.
//!
//! Die Erwartungen sind von Hand aus dem Gesetz gerechnet, nicht aus der Programmausgabe
//! abgeschrieben. Quelle: `sources/gesetze-im-internet/estg_p33_abs3_2026-07-09.txt` (Prozentsaetze)
//! und `estg_p2_2026-07-14.txt` (§ 2 Abs. 6 Satz 2).

#![allow(clippy::unwrap_used)]

use catala_sys::{
    festzusetzende_est_gesamt, festzusetzende_est_gesamt_zusammen, zumutbare_belastung,
    GesamtEingabe, Vz,
};

/// 60.000 EUR Gesamtbetrag der Einkuenfte, in Cent.
const GDE_60K: i64 = 6_000_000;
/// 30.000 EUR Gesamtbetrag der Einkuenfte, in Cent.
const GDE_30K: i64 = 3_000_000;

/// § 33 Abs. 3 `EStG`, Tabelle in `estg_p33_abs3_2026-07-09.txt`: Prozent des Gesamtbetrags der
/// Einkuenfte, je Stufe (bis 15.340 / ueber 15.340 bis 51.130 / ueber 51.130 EUR). Die Prozentsaetze
/// wirken stufenweise auf den jeweiligen Teilbetrag (so rechnet der Catala-Text; der Wortlaut in
/// `sources/` nennt nur die Saetze).
///
/// Rechnung bei 60.000 EUR (Teilbetraege 15.340, 35.790, 8.870 EUR):
/// - drei und mehr Kinder, 1 / 1 / 2 Prozent:
///   153,40 + 357,90 + 177,40 = 688,70 EUR = 68.870 Cent
/// - ein und zwei Kinder, 2 / 3 / 4 Prozent:
///   306,80 + 1.073,70 + 354,80 = 1.735,30 EUR = 173.530 Cent
/// - keine Kinder, 5 / 6 / 7 Prozent:
///   767,00 + 2.147,40 + 620,90 = 3.535,30 EUR = 353.530 Cent
///
/// Rechnung bei 30.000 EUR (Teilbetraege 15.340 und 14.660 EUR, die dritte Stufe entfaellt):
/// - drei und mehr Kinder, 1 / 1 Prozent: 153,40 + 146,60 = 300,00 EUR = 30.000 Cent
/// - zwei Kinder, 2 / 3 Prozent: 306,80 + 439,80 = 746,60 EUR = 74.660 Cent
#[test]
fn zumutbare_belastung_ab_drei_kindern_nimmt_die_niedrigste_zeile() {
    let z = |gde, kinder, splitting| zumutbare_belastung(gde, kinder, splitting).unwrap();

    // 60.000 EUR: Grenze zwischen zwei und drei Kindern, und darueber hinaus.
    assert_eq!(z(GDE_60K, 0, false), 353_530);
    assert_eq!(z(GDE_60K, 1, false), 173_530);
    assert_eq!(z(GDE_60K, 2, false), 173_530);
    assert_eq!(z(GDE_60K, 3, false), 68_870);
    assert_eq!(z(GDE_60K, 4, false), 68_870);
    assert_eq!(z(GDE_60K, 10, false), 68_870);

    // 30.000 EUR: nur die ersten zwei Stufen laufen, auch dort trennt das dritte Kind die Zeile.
    assert_eq!(z(GDE_30K, 2, false), 74_660);
    assert_eq!(z(GDE_30K, 3, false), 30_000);

    // Mit Kindern gibt es keine Splitting-Zeile (Tabelle: nur Zeile 1 trennt a) und b)).
    assert_eq!(z(GDE_60K, 3, true), 68_870);
}

fn gesamt_eingabe(zulage_cent: i64) -> GesamtEingabe {
    GesamtEingabe {
        einkuenfte_nichtselbststaendig_cent: GDE_60K,
        einkuenfte_kapitalvermoegen_cent: 0,
        einkuenfte_vermietung_cent: 0,
        einkuenfte_sonstige_cent: 0,
        einkuenfte_gewinn_cent: 0,
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
        hinzurechnung_zulage_cent: zulage_cent,
        tarif_modifiziert: false,
        tarifliche_est_modifiziert_cent: 0,
    }
}

/// § 2 Abs. 6 Satz 2 `EStG` (`estg_p2_2026-07-14.txt`): "ist fuer die Ermittlung der festzusetzenden
/// Einkommensteuer der Anspruch auf Zulage nach Abschnitt XI der tariflichen Einkommensteuer
/// hinzuzurechnen". Ohne weitere Ermaessigungen und Hinzurechnungen gilt also
/// `festzusetzende_est = tarifliche_est + zulage`, im Einzel- wie im Zusammenpfad, und die
/// tarifliche `ESt` bleibt von der Zulage unberuehrt. 175 EUR sind die Grundzulage (§ 84 `EStG`),
/// 1.234,56 EUR ein Betrag mit Cent-Rest, damit ein Abrunden der Hinzurechnung auffaellt.
#[test]
fn hinzurechnung_zulage_erhoeht_die_festzusetzende_est_um_genau_die_zulage() {
    for zulage_cent in [17_500, 123_456] {
        let ohne = festzusetzende_est_gesamt(gesamt_eingabe(0), Vz::Vz2025).unwrap();
        let mit = festzusetzende_est_gesamt(gesamt_eingabe(zulage_cent), Vz::Vz2025).unwrap();
        assert_eq!(
            ohne.festzusetzende_est_cent().unwrap(),
            ohne.tarifliche_est_cent().unwrap(),
            "Einzel ohne Zulage"
        );
        assert_eq!(
            mit.tarifliche_est_cent().unwrap(),
            ohne.tarifliche_est_cent().unwrap(),
            "Einzel: die Zulage aendert die tarifliche ESt nicht ({zulage_cent})"
        );
        assert_eq!(
            mit.festzusetzende_est_cent().unwrap(),
            mit.tarifliche_est_cent().unwrap() + zulage_cent,
            "Einzel: festzusetzende ESt = tarifliche ESt + Zulage ({zulage_cent})"
        );

        let ohne = festzusetzende_est_gesamt_zusammen(gesamt_eingabe(0), Vz::Vz2025).unwrap();
        let mit =
            festzusetzende_est_gesamt_zusammen(gesamt_eingabe(zulage_cent), Vz::Vz2025).unwrap();
        assert_eq!(
            ohne.festzusetzende_est_cent().unwrap(),
            ohne.tarifliche_est_cent().unwrap(),
            "Zusammen ohne Zulage"
        );
        assert_eq!(
            mit.tarifliche_est_cent().unwrap(),
            ohne.tarifliche_est_cent().unwrap(),
            "Zusammen: die Zulage aendert die tarifliche ESt nicht ({zulage_cent})"
        );
        assert_eq!(
            mit.festzusetzende_est_cent().unwrap(),
            mit.tarifliche_est_cent().unwrap() + zulage_cent,
            "Zusammen: festzusetzende ESt = tarifliche ESt + Zulage ({zulage_cent})"
        );
    }
}
