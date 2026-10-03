//! § 34 Abs. 3 fuer Person A, Veraeusserungsgewinn beim Ehegatten: der Guard sperrt mit
//! `abs3_partner_gewinn_offen`, statt eine Zahl mit ungeglaettetem Partner-Gewinn zu liefern.
//!
//! Der Test braucht weder Python noch `PARITY=1`: die Parity-Suite `bescheid_deklaration_paritaet`
//! haelt dieselben Faelle gegen Python fest (`p34_partner_gewinn_sperre`), laeuft aber nur mit
//! `PARITY=1` und damit nicht in der CI. Ohne diesen Test bliebe der Aufruf `abs3_partner_gewinn` in
//! `deklaration/sperre.rs` ohne Gegenprobe im Standardlauf.
//!
//! Trigger (Entscheid main 2026-10-03): ROHER Partner-Gewinn > 0 (nicht `netto_vg_partner`), UND
//! `zusammen`, UND der Chooser nimmt Abs. 3 (Antrag, Berechtigung, 0 < `netto_vg` <= 5 Mio). Nur
//! bestaetigte Felder urteilen; ein Betrag <= 0 sperrt nie. Der Grund wird als Text verglichen, damit
//! der Test auch ohne die Enum-Variante uebersetzt und rot wird statt nicht zu uebersetzen.
//!
//! Abweichungsfall roh/netto: 40.000 EUR beim Partner liegen unter dem Freibetrag (netto 0); der rohe
//! Trigger sperrt, ein Netto-Trigger sperrte nicht.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::deklaration::{an_gesamt_sperrgrund, Cfg};
use bescheid::testhilfe::{felder, index, store};
use bescheid::{BindungIndex, Instanzquelle};
use domain::{Scheibe, Vz};
use serde_json::{json, Value};

const GRUND: &str = "abs3_partner_gewinn_offen";
/// 500.000 EUR (Person A, ueber dem Freibetrag, unter 5 Mio).
const VG_A: i64 = 50_000_000;
/// 300.000 EUR (Partner, Freibetrag 0).
const VG_PARTNER: i64 = 30_000_000;

type Ereignisse = Vec<(&'static str, Value, bool)>;

/// Die Bindung der Scheibe: nur ihre Feld-Ids, wie `api._scheibe_bindung`. Der volle Index kennt mehr
/// Flags und liesse `flag_widersprueche` auf Felder ansprechen, die der Fall nicht fuehrt.
fn scheiben_index(scheibe: Scheibe) -> BindungIndex<'static> {
    let ids = Cfg::fuer(scheibe).felder(|_| Vec::new()).unwrap();
    index()
        .iter()
        .filter(|(k, _)| ids.contains(k))
        .map(|(k, b)| (k.clone(), *b))
        .collect()
}

/// Ein berechtigter Fall: zusammen, A beantragt Abs. 3 mit 500.000 EUR, Partner 300.000 EUR. `abw`
/// ersetzt Felder; ein Wert `Null` entfernt das Feld.
fn fall(abw: &[(&'static str, Value, bool)]) -> Ereignisse {
    let mut e: Ereignisse = vec![
        ("veranlagung", json!("zusammen"), true),
        ("antrag_ermaessigter_satz", json!(true), true),
        ("geburtsjahr", json!(1960), true),
        ("dauernd_berufsunfaehig", json!(false), true),
        ("ermaessigung_einmal_genutzt", json!(false), true),
        ("rentner_alter_55_oder_berufsunfaehig", json!(true), true),
        ("rentner_freibetrag_erstmalig", json!(true), true),
        ("rentner_veraeusserungsgewinn", json!(VG_A), true),
        ("rentner_veraeusserungs_betriebsart", json!("gewerbe"), true),
        (
            "rentner_veraeusserungsgewinn_partner",
            json!(VG_PARTNER),
            true,
        ),
        (
            "rentner_alter_55_oder_berufsunfaehig_partner",
            json!(true),
            true,
        ),
        ("rentner_freibetrag_erstmalig_partner", json!(true), true),
        // ohne diese zwei Gruppen sperrt der Guard frueher: `flag_konsistenz_offen` (Gewinn ohne
        // kein_gewinn = nein), `partner_kegel_offen` (zusammen ohne die Pflichtfelder von Person B)
        ("kein_gewinn", json!(false), true),
        ("bruttoarbeitslohn_partner", json!(0), true),
        ("kap_kapitalertraege_partner", json!(0), true),
        ("kap_gewinn_aktien_partner", json!(0), true),
        ("kap_gewinn_sonstige_partner", json!(0), true),
        ("kap_verlust_aktien_partner", json!(0), true),
        ("kap_verlust_sonstige_partner", json!(0), true),
    ];
    for (f, w, b) in abw {
        e.retain(|(g, _, _)| g != f);
        if !w.is_null() {
            e.push((f, w.clone(), *b));
        }
    }
    e
}

/// Der Sperrgrund (als Text) der Scheibe fuer VZ 2025.
fn grund(scheibe: Scheibe, events: &Ereignisse) -> Option<&'static str> {
    let st = store(events);
    let f = felder(&st);
    let idx = scheiben_index(scheibe);
    let q = Instanzquelle {
        store: Some(&st),
        bindung: Some(&idx),
        nur_bestaetigt: false,
    };
    let cfg = Cfg::fuer(scheibe);
    an_gesamt_sperrgrund(&f, Some(&cfg), Some(Vz::Vz2025), &q)
        .unwrap()
        .map(domain::Sperrgrund::als_str)
}

const SCHEIBEN: [Scheibe; 2] = [Scheibe::Gesamt, Scheibe::RentnerGesamt];

#[test]
fn der_volle_fall_sperrt_in_beiden_scheiben() {
    for s in SCHEIBEN {
        assert_eq!(grund(s, &fall(&[])), Some(GRUND), "{s:?}");
    }
}

/// Jede Bedingung einzeln: fehlt eine, sperrt der Guard nicht (und keine fremde Sperre springt ein).
#[test]
fn ohne_eine_der_bedingungen_sperrt_der_guard_nicht() {
    let gewinn = "rentner_veraeusserungsgewinn_partner";
    let faelle: Vec<(&str, Ereignisse)> = vec![
        // Betrag: nur ein Gewinn > 0 sperrt
        ("Partner-VG 0", fall(&[(gewinn, json!(0), true)])),
        ("Partner-VG negativ", fall(&[(gewinn, json!(-1), true)])),
        ("Partner-VG fehlt", fall(&[(gewinn, Value::Null, true)])),
        // nur bestaetigte Felder urteilen
        (
            "Partner-VG vorlaeufig",
            fall(&[(gewinn, json!(VG_PARTNER), false)]),
        ),
        (
            "Antrag vorlaeufig Ja",
            fall(&[("antrag_ermaessigter_satz", json!(true), false)]),
        ),
        (
            "Antrag vorlaeufig Nein",
            fall(&[("antrag_ermaessigter_satz", json!(false), false)]),
        ),
        (
            "Veranlagung vorlaeufig",
            fall(&[("veranlagung", json!("zusammen"), false)]),
        ),
        (
            "Geburtsjahr vorlaeufig",
            fall(&[("geburtsjahr", json!(1960), false)]),
        ),
        (
            "Gewinn A vorlaeufig",
            fall(&[("rentner_veraeusserungsgewinn", json!(VG_A), false)]),
        ),
        // der Chooser nimmt Abs. 3 nicht: keine falsche Zahl, also keine Sperre
        (
            "Antrag Nein",
            fall(&[("antrag_ermaessigter_satz", json!(false), true)]),
        ),
        (
            "Antrag fehlt",
            fall(&[("antrag_ermaessigter_satz", Value::Null, true)]),
        ),
        (
            "Einzelveranlagung",
            fall(&[("veranlagung", json!("einzel"), true)]),
        ),
        ("A zu jung", fall(&[("geburtsjahr", json!(1990), true)])),
        (
            "A Abs. 3 schon genutzt",
            fall(&[("ermaessigung_einmal_genutzt", json!(true), true)]),
        ),
        (
            "A netto 0 (40.000 EUR unter dem Freibetrag)",
            fall(&[("rentner_veraeusserungsgewinn", json!(4_000_000), true)]),
        ),
        (
            "A ohne Gewinn",
            fall(&[("rentner_veraeusserungsgewinn", Value::Null, true)]),
        ),
    ];
    for s in SCHEIBEN {
        for (name, e) in &faelle {
            assert_eq!(grund(s, e), None, "{s:?}: {name}");
        }
    }
}

/// Abweichungsfall roh/netto: Partner-Gewinn unter dem Freibetrag sperrt, 1 Cent auch.
#[test]
fn der_rohe_partner_gewinn_entscheidet_nicht_der_netto_gewinn() {
    for vg in [1, 4_000_000, VG_PARTNER] {
        let e = fall(&[("rentner_veraeusserungsgewinn_partner", json!(vg), true)]);
        for s in SCHEIBEN {
            assert_eq!(grund(s, &e), Some(GRUND), "{s:?}: Partner-VG {vg}");
        }
    }
}

/// Ueber 5 Mio steht die eigene, genauere Sperre davor.
#[test]
fn ueber_fuenf_millionen_behaelt_den_eigenen_grund() {
    let e = fall(&[("rentner_veraeusserungsgewinn", json!(600_000_000), true)]);
    assert_eq!(grund(Scheibe::Gesamt, &e), Some("abs3_ueber_5mio_offen"));
}

/// Der Klartext steht im Enum (Python `SPERRGRUND_KLARTEXT`, byte-gleich ueber die Fixture in `domain`).
#[test]
fn der_grund_hat_einen_klartext() {
    let g: domain::Sperrgrund = GRUND.parse().unwrap();
    assert!(g.klartext().is_some_and(|k| k.contains("Ehepartner")));
}
