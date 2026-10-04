//! § 20 Abs. 6 `EStG`: wann entscheiden die Toepfe, wann zaehlt der Aggregat-Betrag (`p20_kapitaleinkuenfte`), im
//! Standardlauf (ohne `PARITY=1`, ohne Python). Ein Topf ist "belegt", wenn sein Wert `!= 0` ist, auch wenn er negativ ist
//! (Python `any(_c(t) != 0 for t in KAP_TOEPFE)`).
//!
//! WARUM ES DIESE DATEI GIBT. Die Mutationsmessung `bescheid-elster-mutation` (2026-10-04, Kennung E11) liess
//! `einkuenfte::p20_kapitaleinkuenfte` mit `!= 0` -> `> 0` in allen Standard-Tests gruen; nur `PARITY=1` faengt es, mit
//! Zufallsfaellen, die negative Toepfe enthalten. Ueber die HTTP-Schnittstelle ist die Stelle NICHT erreichbar:
//! `POST /event` weist einen negativen Topf mit 422 ("fail-closed (Vorzeichen)") ab (gemessen am Python-Server fuer
//! `kap_gewinn_aktien` und `kap_verlust_aktien`). Darum sitzt der Fall hier auf Funktionsebene, nicht in
//! `api/tests/kette_endstand_hermetisch.rs`. Ein Fall aus dem Store, der den Wertebereich umgeht, bleibt die einzige Quelle
//! eines negativen Topfes.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: die Ausgabe des Python-Orakels, gemessen 2026-10-04 auf main 2e4cc91d:
//! `bescheid_einkuenfte._p20_kapitaleinkuenfte(lambda k: werte.get(k, 0), zusammen, 2025)` mit den Cent-Werten dieser
//! Tabelle (`import bescheid_einkuenfte` ueber `tools/parity/korpus_faelle`). Kein Wert ist aus dem Rust-Code abgelesen.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::einkuenfte::p20_kapitaleinkuenfte;
use bescheid::testhilfe::{felder, params, store};
use domain::Vz;
use serde_json::json;

/// `(Felder in Cent, zusammen, Python-Wert in EURO)`.
type Fall<'a> = (&'a [(&'a str, i64)], bool, i64);

/// Kapitaleinkuenfte in EURO nach Sparer-Pauschbetrag fuer die Felder `werte` (Name, Cent).
fn kap(werte: &[(&str, i64)], zusammen: bool) -> i64 {
    let paare: Vec<_> = werte
        .iter()
        .map(|(feld, cent)| (*feld, json!(*cent), true))
        .collect();
    let f = felder(&store(&paare));
    p20_kapitaleinkuenfte(&f, zusammen, Vz::Vz2025, params())
        .unwrap()
        .get()
}

/// Gegenprobe: ohne Topf zaehlt der Aggregat-Betrag (20.000 EUR minus Sparer-Pauschbetrag 1.000 EUR).
#[test]
fn ohne_topf_zaehlt_der_aggregat_betrag() {
    assert_eq!(kap(&[("kap_kapitalertraege", 2_000_000)], false), 19_000);
    assert_eq!(
        kap(
            &[
                ("kap_kapitalertraege", 2_000_000),
                ("kap_kapitalertraege_partner", 1_000_000)
            ],
            true
        ),
        28_000
    );
}

/// Ein NEGATIVER Topf ist belegt: die Verrechnung entscheidet, der Aggregat-Betrag zaehlt nicht.
#[test]
fn negativer_topf_ist_belegt_und_schaltet_auf_die_verrechnung() {
    let faelle: [Fall; 3] = [
        (
            &[
                ("kap_gewinn_aktien", -300_000),
                ("kap_kapitalertraege", 2_000_000),
            ],
            false,
            0,
        ),
        (
            &[
                ("kap_verlust_aktien", -300_000),
                ("kap_kapitalertraege", 2_000_000),
            ],
            false,
            2_000,
        ),
        (
            &[
                ("kap_kapitalertraege", 2_000_000),
                ("kap_gewinn_aktien_partner", -300_000),
                ("kap_kapitalertraege_partner", 1_000_000),
            ],
            true,
            18_000,
        ),
    ];
    for (werte, zusammen, python) in faelle {
        assert_eq!(
            kap(werte, zusammen),
            python,
            "{werte:?}, zusammen {zusammen}"
        );
    }
}
