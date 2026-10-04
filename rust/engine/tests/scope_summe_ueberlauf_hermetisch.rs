//! Vier Catala-Scopes, die Cent-Betraege exakt (GMP) addieren oder subtrahieren, deren Ergebnis der C-Shim aber mit `mpz_get_si`
//! liest (Rust allein, hermetisch, ohne `PARITY=1`, ohne Python). Bericht h8-ep-fenster, Schritt 6 (Nachbarn der Entfernungspauschale).
//!
//! ANLASS: `mpz_get_si` gibt bei einem Wert ausserhalb `long` still dessen untere 63 Bit zurueck, keinen Fehler. Jeder Eingabebetrag
//! passt einzeln in `i64` (`in_cent` prueft ihn), Summe oder Differenz im Scope nicht: `EuerGewinn` (Einnahmen minus Ausgaben),
//! `MitunternehmerEinkuenfte` (vier Summanden), `Kirchensteuerabzug` (gezahlt minus erstattet), `AgbAbzug` (Aufwendungen minus
//! zumutbare Belastung). Vor dem Fix lieferte Rust dort `Ok` mit einer falschen Zahl; ueber HTTP (`gesamt`, `kein_gewinn` falsch)
//! antwortete Rust 200 mit `zahl_cent` 4150517416584808200 statt Pythons 8301034833169457400 (Mitunternehmer: `gewinnanteil` und
//! `verguetung_taetigkeit` je 9223372036854775800 ct).
//!
//! ERWARTUNG: `EngineFehler::Ueberlauf(<Marke>)`, wenn das ERGEBNIS nicht in `i64` passt. Nicht, wenn nur eine Teilsumme nicht passt:
//! der Scope rechnet exakt, `M + M - M - M` ist 0 (Python 0) und bleibt `Ok(0)`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: Python `runner.catala_euer_gewinn`, `catala_mitunternehmer_einkuenfte`, `catala_p10_kist`,
//! `catala_p33_agb` (exakte Ganzzahlen, VZ-unabhaengig), Skript und Ausgabe `orakel_nachbarn.py` / `orakel_nachbarn.out` in den
//! Anlagen zum Bericht. `M` ist der groesste Euro-Betrag, dessen Cent-Wert noch in `i64` passt (92233720368547758).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use domain::Euro;
use engine::zugriff::teil1::belastungen::{p33_agb, P33AgbEingabe, P33ZumutbarEingabe};
use engine::zugriff::teil1::einkuenfte::{
    euer_gewinn, mitunternehmer_einkuenfte, EuerGewinnEingabe, MitunternehmerEinkuenfteEingabe,
};
use engine::zugriff::teil1::sonderausgaben::{p10_kist, P10KistEingabe};

const M: i64 = i64::MAX / 100;
/// `2 * H = M`.
const H: i64 = 46_116_860_184_273_879;
/// `4 * V + 2 = M`.
const V: i64 = 23_058_430_092_136_939;

/// `Some(python)`: Rust MUSS `Ok(Euro(python))` liefern. `None`: Rust MUSS `Err(Ueberlauf(marke))` melden.
type Soll = Option<i128>;

fn pruefe(marke: &str, faelle: &[(&str, Soll, String)]) {
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|(name, soll, ist)| {
            let soll = soll.map_or_else(
                || format!(r#"Err(Ueberlauf("{marke}"))"#),
                |v| format!("Ok(Euro({v}))"),
            );
            (ist != &soll).then(|| format!("{name}: ist {ist}, soll {soll}"))
        })
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
fn euer_gewinn_differenz_ausserhalb_i64_ist_ueberlauf() {
    let f = |a: i64, b: i64| {
        format!(
            "{:?}",
            euer_gewinn(&EuerGewinnEingabe {
                betriebseinnahmen: Euro::new(a),
                betriebsausgaben: Euro::new(b),
            })
        )
    };
    pruefe(
        "euer gewinn",
        &[
            ("M - (-M)", None, f(M, -M)),
            ("-M - M", None, f(-M, M)),
            ("M - 0 (Kontrolle)", Some(92_233_720_368_547_758), f(M, 0)),
            (
                "H - (-H): passt gerade noch",
                Some(92_233_720_368_547_758),
                f(H, -H),
            ),
            ("(H+1) - (-H): ein Euro zu viel", None, f(H + 1, -H)),
            (
                "-H - H: passt gerade noch",
                Some(-92_233_720_368_547_758),
                f(-H, H),
            ),
            ("(-H-1) - H: ein Euro zu viel", None, f(-H - 1, H)),
        ],
    );
}

#[test]
fn mitunternehmer_summe_ausserhalb_i64_ist_ueberlauf_aber_nicht_bei_grosser_teilsumme() {
    let f = |a: i64, b: i64, c: i64, d: i64| {
        format!(
            "{:?}",
            mitunternehmer_einkuenfte(&MitunternehmerEinkuenfteEingabe {
                gewinnanteil: Euro::new(a),
                verguetung_taetigkeit: Euro::new(b),
                verguetung_darlehen: Euro::new(c),
                verguetung_ueberlassung: Euro::new(d),
            })
        )
    };
    pruefe(
        "mitunternehmer summe",
        &[
            ("M+M+M+M", None, f(M, M, M, M)),
            ("M+M", None, f(M, M, 0, 0)),
            ("0+M+M+0 (nur Verguetungen)", None, f(0, M, M, 0)),
            (
                "M+0+0+0 (Kontrolle)",
                Some(92_233_720_368_547_758),
                f(M, 0, 0, 0),
            ),
            (
                "4V+2 = M: passt gerade noch",
                Some(92_233_720_368_547_758),
                f(V, V, V, V + 2),
            ),
            ("4V+3 = M+1: ein Euro zu viel", None, f(V, V, V, V + 3)),
            // Teilsumme M+M liegt ausserhalb i64, das Ergebnis 0 nicht: der Scope rechnet exakt, Python 0.
            (
                "M+M-M-M: grosse Teilsumme, Ergebnis 0",
                Some(0),
                f(M, M, -M, -M),
            ),
        ],
    );
}

#[test]
fn kirchensteuerabzug_differenz_ausserhalb_i64_ist_ueberlauf() {
    let f = |gezahlt: i64, erstattet: i64| {
        format!(
            "{:?}",
            p10_kist(&P10KistEingabe {
                gezahlte_kirchensteuer: Euro::new(gezahlt),
                erstattete_kirchensteuer: Euro::new(erstattet),
            })
        )
    };
    pruefe(
        "kist abzug",
        &[
            ("M - (-M)", None, f(M, -M)),
            ("M - 0 (Kontrolle)", Some(92_233_720_368_547_758), f(M, 0)),
            (
                "H - (-H): passt gerade noch",
                Some(92_233_720_368_547_758),
                f(H, -H),
            ),
            ("(H+1) - (-H): ein Euro zu viel", None, f(H + 1, -H)),
            // gezahlt <= erstattet: der Scope gibt 0 aus, die Differenz -2M erreicht den Shim nie.
            ("-M - M: Erstattung ueberwiegt, Abzug 0", Some(0), f(-M, M)),
        ],
    );
}

#[test]
fn agb_abzug_differenz_ausserhalb_i64_ist_ueberlauf() {
    // Negativer Gesamtbetrag der Einkuenfte macht die zumutbare Belastung negativ (5 % von -M = -461168601842738790 ct);
    // Aufwendungen minus zumutbare Belastung ist dann eine Summe.
    let f = |agb: i64, gde: i64| {
        format!(
            "{:?}",
            p33_agb(&P33AgbEingabe {
                aussergewoehnliche_belastungen: Euro::new(agb),
                zumutbar: P33ZumutbarEingabe {
                    gesamtbetrag_der_einkuenfte: Euro::new(gde),
                    anzahl_kinder: 0,
                    splitting: false,
                },
            })
        )
    };
    pruefe(
        "agb abzug",
        &[
            ("M bei GdE -M", None, f(M, -M)),
            (
                "M bei GdE M (Kontrolle)",
                Some(85_777_359_942_750_079),
                f(M, M),
            ),
            (
                "87622034350120370 bei GdE -M: passt gerade noch",
                Some(92_233_720_368_547_757),
                f(87_622_034_350_120_370, -M),
            ),
            (
                "87622034350120371 bei GdE -M: ein Euro zu viel",
                None,
                f(87_622_034_350_120_371, -M),
            ),
            // agb < zumutbar: der Scope gibt 0 aus, die Differenz (unter i64::MIN) erreicht den Shim nie.
            (
                "-M bei GdE M: Abzug 0, Differenz unter i64::MIN",
                Some(0),
                f(-M, M),
            ),
        ],
    );
}
