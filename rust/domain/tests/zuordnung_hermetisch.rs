//! Zuordnungen von `domain`, die kein Bestandstest festnagelt (Mutationsmessung N4, Bericht `mutation-bindung-domain`): ein
//! vertauschter Schluessel-String oder Serde-Name bliebe im Standardlauf (ohne `PARITY=1`, ohne Python) gruen.
//!
//! * `sperrgrund.rs`: `als_str` je Variante. Der Bestandstest prueft nur `FromStr` -> `klartext`; ein vertauschtes `als_str`
//!   schriebe den falschen `grund` in die Antwort (`api.py: ergebnis()`) und in den Fehler-Log.
//! * `veranlagung.rs` (`Scheibe`) und `zustand.rs` (`PruefTiefe`): `rename_all = "snake_case"` entscheidet bei mehrteiligen
//!   Namen (`n_vor_gwg`, `orakel_bestaetigt`) zwischen dem Wire-Format und `nvorgwg`; einteilige Namen (`Veranlagung`,
//!   `Zustand`, `VorschlagTyp`, `Feldtyp`) sind bei `lowercase` und `snake_case` gleich, dort ist die Mutante gleichwertig.
//! * `wert.rs`: `Feldtyp::als_str` (steht in der Fehlermeldung `TypInkonform`) gleich dem Serde-Namen.
//! * `vz.rs`: die Zahlenwerte von `Vz` gehen als `vz as i32` ins generierte Catala-C (`catala-sys`, `tg_grundtarif(.., vz as
//!   i32, ..)`); dessen `switch` hat keinen `default`-Zweig.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::HashSet;

use domain::{
    Feldtyp, PruefTiefe, Scheibe, Sperrgrund, UnbekannterSperrgrund, Veranlagung, VorschlagTyp, Vz,
    Zustand,
};
use serde_json::Value;

const SPERRGRUND: &str = include_str!("../../fixtures/sperrgrund_klartext.json");

/// Jeder der 57 Schluessel der Python-Quelle: `parse` -> `als_str`/`Display` gibt ihn zurueck, und die 57 Varianten sind
/// verschieden. Dazu `bestaetigt` (kein Klartext) und ein unbekannter String.
#[test]
fn sperrgrund_wire_string_rundtrippt_fuer_jeden_schluessel() {
    let fixture: Value = serde_json::from_str(SPERRGRUND).unwrap();
    let klartext = fixture["klartext"].as_object().unwrap();
    assert_eq!(
        klartext.len(),
        57,
        "Fixture-Groesse: Enum und Test nachziehen"
    );
    let mut gesehen = HashSet::new();
    for schluessel in klartext.keys() {
        let grund: Sperrgrund = schluessel.parse().unwrap();
        assert_eq!(grund.als_str(), schluessel, "als_str von {grund:?}");
        assert_eq!(grund.to_string(), *schluessel, "Display von {grund:?}");
        assert!(
            gesehen.insert(grund),
            "{schluessel}: zweite Variante mit derselben Zuordnung"
        );
    }
    assert_eq!(gesehen.len(), 57);
    assert!(!gesehen.contains(&Sperrgrund::Bestaetigt));
    assert_eq!(
        "bestaetigt".parse::<Sperrgrund>(),
        Ok(Sperrgrund::Bestaetigt)
    );
    assert_eq!(Sperrgrund::Bestaetigt.als_str(), "bestaetigt");
    assert_eq!(Sperrgrund::Bestaetigt.to_string(), "bestaetigt");
    assert_eq!(
        "kein_grund".parse::<Sperrgrund>(),
        Err(UnbekannterSperrgrund("kein_grund".to_owned()))
    );
    assert_eq!(
        "".parse::<Sperrgrund>(),
        Err(UnbekannterSperrgrund(String::new()))
    );
}

fn json<T: serde::Serialize>(w: &T) -> String {
    serde_json::to_string(w).unwrap()
}

/// `Scheibe`: Serde-Name gleich `als_str` gleich Python-Schluessel; beide Richtungen.
#[test]
fn scheibe_serde_name_ist_der_wire_name() {
    for (s, name) in [
        (Scheibe::Ep, "ep"),
        (Scheibe::NVorGwg, "n_vor_gwg"),
        (Scheibe::AnGesamt, "an_gesamt"),
        (Scheibe::Gesamt, "gesamt"),
        (Scheibe::RentnerGesamt, "rentner_gesamt"),
    ] {
        assert_eq!(json(&s), format!("\"{name}\""));
        assert_eq!(s.als_str(), name);
        assert_eq!(
            serde_json::from_str::<Scheibe>(&format!("\"{name}\"")).unwrap(),
            s
        );
        if name.contains('_') {
            assert!(
                serde_json::from_str::<Scheibe>(&format!("\"{}\"", name.replace('_', ""))).is_err(),
                "{name} ohne Unterstrich"
            );
        }
    }
}

/// `PruefTiefe`: der gespeicherte Name im Event (`herkunft.pruef_tiefe`), wie `store.py: _PRUEF_ORD`.
#[test]
fn pruef_tiefe_serde_name_ist_der_gespeicherte_name() {
    for (t, name) in [
        (PruefTiefe::Ungeprueft, "ungeprueft"),
        (PruefTiefe::Plausibilisiert, "plausibilisiert"),
        (PruefTiefe::OrakelBestaetigt, "orakel_bestaetigt"),
        (PruefTiefe::Amtlich, "amtlich"),
    ] {
        assert_eq!(json(&t), format!("\"{name}\""));
        assert_eq!(
            serde_json::from_str::<PruefTiefe>(&format!("\"{name}\"")).unwrap(),
            t
        );
    }
    assert!(serde_json::from_str::<PruefTiefe>("\"orakelbestaetigt\"").is_err());
}

/// Die einteiligen Namen: Serde-Name ist der kleingeschriebene Variantenname (gilt fuer `lowercase` wie fuer `snake_case`).
#[test]
fn einteilige_serde_namen() {
    assert_eq!(json(&Veranlagung::Einzel), "\"einzel\"");
    assert_eq!(json(&Veranlagung::Zusammen), "\"zusammen\"");
    assert_eq!(json(&Zustand::Vorlaeufig), "\"vorlaeufig\"");
    assert_eq!(json(&Zustand::Bestaetigt), "\"bestaetigt\"");
    for (t, name) in [
        (VorschlagTyp::Llm, "llm"),
        (VorschlagTyp::Beleg, "beleg"),
        (VorschlagTyp::Kontoauszug, "kontoauszug"),
        (VorschlagTyp::Maps, "maps"),
    ] {
        assert_eq!(json(&t), format!("\"{name}\""));
        assert_eq!(name.parse::<VorschlagTyp>(), Ok(t));
    }
}

/// `Feldtyp::als_str` je Variante, gleich dem Serde-Namen; die Fehlermeldung `TypInkonform` nennt diesen Namen.
#[test]
fn feldtyp_als_str_ist_der_serde_name() {
    for (t, name) in [
        (Feldtyp::Cent, "cent"),
        (Feldtyp::Int, "int"),
        (Feldtyp::Bool, "bool"),
        (Feldtyp::Enum, "enum"),
        (Feldtyp::Datum, "datum"),
        (Feldtyp::Text, "text"),
    ] {
        assert_eq!(t.als_str(), name);
        assert_eq!(json(&t), format!("\"{name}\""));
        assert_eq!(
            serde_json::from_str::<Feldtyp>(&format!("\"{name}\"")).unwrap(),
            t
        );
    }
    let fehler = domain::Wert::aus_pywert(&domain::PyWert::Text("x".into()), Feldtyp::Cent, None)
        .unwrap_err();
    assert_eq!(
        fehler.to_string(),
        "Wert 'x' passt nicht zum Bindungstyp 'cent'"
    );
}

/// Die C-Zahl je Veranlagungszeitraum: 0, 1, 2 in der Reihenfolge der Jahre, und `jahr()` dazu.
#[test]
fn vz_zahlenwerte_fuer_das_catala_c() {
    assert_eq!(Vz::Vz2024 as i32, 0);
    assert_eq!(Vz::Vz2025 as i32, 1);
    assert_eq!(Vz::Vz2026 as i32, 2);
    for (vz, jahr) in [
        (Vz::Vz2024, 2024_u16),
        (Vz::Vz2025, 2025),
        (Vz::Vz2026, 2026),
    ] {
        assert_eq!(vz.jahr(), jahr);
        assert_eq!(Vz::try_from(jahr), Ok(vz));
        assert_eq!(vz.to_string(), jahr.to_string());
    }
    assert!(Vz::try_from(2023_u16).is_err());
    assert!(Vz::try_from(2027_u16).is_err());
}
