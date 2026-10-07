//! § 34c Abs. 2 `EStG`, Abzug statt Anrechnung (Abweichung Nr. 29): die Rechnung. Im Standardlauf, ohne `PARITY=1`, ohne
//! Python. Der Zweig `dba_abzug_statt_anrechnung` in `einkuenfte::shared_dba_sonstige` hatte in Rust bis dahin keinen Test
//! (im Vergleichstest `bescheid_blatt_paritaet.rs` steht das Feld nur mit dem Wert `false`).
//!
//! **Worum es geht.** Wer im Ausland Steuer gezahlt hat, laesst sie anrechnen (Abs. 1) oder, nur auf Antrag, bei der
//! Ermittlung der Einkuenfte abziehen (Abs. 2). Die Rechnung kennt beide Wege und waehlt nie von selbst den guenstigeren.
//!
//! **Warum es zaehlt.** Der Abzugszweig ist die Gegenseite der Sperre in `p34c_abzug_einreichung_hermetisch.rs`: die
//! Sperre haelt die Abgabe an, WEIL die Rechnung den Abzug rechnet. Rechnete der Zweig nichts, wuerde die Sperre ohne Grund
//! sperren. Dieses Modul haelt fest, was der Zweig tut, und dass die Anrechnung ohne Wahl unveraendert bleibt.
//!
//! **Bekannte Abweichung vom Gesetz, hier nur festgehalten, nicht geaendert.** Das Gesetz zieht die Steuer "bei der
//! Ermittlung der Einkuenfte" ab (§ 34c Abs. 2); die Rechnung zieht sie vom Einkommen ab (`sonstige_abzuege_vom_einkommen`),
//! wie Python. Der Unterschied bei den Einkuenften (Gesamtbetrag, Altersentlastung u. a.) ist nicht gemessen. Die Tests
//! unten pinnen das Verhalten des Zweigs, nicht seine Richtigkeit gegen das Gesetz.
//!
//! **Abweichung Nr. 35 (Freistellung schlaegt die Wahl).** Bei einem Abkommen mit Freistellung gibt es keinen Abzug
//! (§ 34c Abs. 6 S. 1 und 2 `EStG`). Die drei Tests `bei_freistellung_*` und `bei_anrechnungsabkommen_*` halten das fest;
//! Python bucht dort den Abzug (Vault: `p34c-bei-dba-freistellung-rechnet-der-bescheid-keinen-abzug`).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand. Lohn 50.000 Euro, Sonderausgaben-Pauschbetrag 36 Euro (§ 10c,
//! `params/2025/`), also zvE ohne Abzug 49.964 Euro (`engine::zugriff::teil2::gesamt`, Doctest `gesamt_zve`). Gezahlte
//! Steuer 700 Euro, Auslandseinkuenfte 5.000 Euro. Hoechstbetrag der Anrechnung (Abs. 1 S. 2): tarifliche Steuer x
//! 5.000 / 49.964, bei einer tariflichen Steuer um 9.400 Euro rund 940 Euro und damit ueber den 700 Euro gezahlter Steuer;
//! die Anrechnung betraegt darum die vollen 700 Euro.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::einkuenfte::{shared_dba_sonstige, DbaErgebnis};
use bescheid::testhilfe::{felder, leerer_gesamtfall, params, store};
use domain::{Euro, Veranlagung, Vz};
use engine::zugriff::teil2::gesamt::{gesamt_zve, GesamtfallEingabe};
use serde_json::{json, Value};

const VZ: Vz = Vz::Vz2025;
const WAHL: &str = "dba_abzug_statt_anrechnung";
const STEUER: &str = "dba_gezahlte_auslaendische_steuer";
const EINKUENFTE: &str = "dba_auslaendische_einkuenfte";

/// Cent aus Euro.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// Der Gesamtfall mit 50.000 Euro Lohn.
fn fall() -> GesamtfallEingabe {
    let mut g = leerer_gesamtfall(VZ, false);
    g.einkuenfte_nichtselbststaendig = Euro::new(50_000);
    g
}

/// Ein Lauf von `shared_dba_sonstige` mit den gegebenen Feldern (alle bestaetigt); liefert Gesamtfall und Ergebnis.
fn lauf(paare: &[(&str, Value)]) -> (GesamtfallEingabe, DbaErgebnis) {
    let events: Vec<(&str, Value, bool)> =
        paare.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
    let f = felder(&store(&events));
    let mut g = fall();
    let erg = shared_dba_sonstige(
        &mut g,
        Euro::new(50_000),
        Veranlagung::Einzel,
        &f,
        VZ,
        params(),
    )
    .unwrap();
    (g, erg)
}

fn zve(g: &GesamtfallEingabe) -> i64 {
    gesamt_zve(g, params()).unwrap().get()
}

/// KONTROLLE zuerst: ohne jede Auslandsangabe bleibt alles bei 0 und das zvE bei 49.964 Euro. Sonst belegten die Zahlen
/// unten nichts: ein Fall, der schon ohne den Zweig abweicht, liesse jeden Test gruen.
#[test]
fn kontrolle_ohne_auslandsangaben_aendert_sich_nichts() {
    let (g, erg) = lauf(&[]);
    assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 0);
    assert_eq!(g.anzurechnende_auslaendische_steuern.get(), 0);
    assert_eq!(erg.dba_anrechnung.get(), 0);
    assert_eq!(erg.p32b_progressionseinkuenfte, None);
    assert_eq!(zve(&g), 49_964);
}

/// Die Anrechnung ohne Wahl (Kontrollfall): 700 Euro gezahlt, Hoechstbetrag rund 940 Euro, also 700 Euro auf die Steuer
/// angerechnet; das zvE bleibt unberuehrt.
#[test]
fn ohne_wahl_rechnet_die_anrechnung_und_laesst_das_einkommen_stehen() {
    for wahl in [None, Some(false)] {
        let mut paare = vec![(STEUER, json!(cent(700))), (EINKUENFTE, json!(cent(5_000)))];
        if let Some(w) = wahl {
            paare.push((WAHL, json!(w)));
        }
        let (g, erg) = lauf(&paare);
        assert_eq!(erg.dba_anrechnung.get(), 700, "Wahl {wahl:?}");
        assert_eq!(
            g.anzurechnende_auslaendische_steuern.get(),
            700,
            "Wahl {wahl:?}"
        );
        assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 0, "Wahl {wahl:?}");
        assert_eq!(zve(&g), 49_964, "Wahl {wahl:?}");
    }
}

/// Der Abzug (Wahl `true`, Steuer und Einkuenfte ueber 0): die 700 Euro mindern das Einkommen, es wird NICHTS angerechnet.
/// zvE 49.964 − 700 = 49.264 Euro.
#[test]
fn abzug_gewaehlt_zieht_die_steuer_vom_einkommen_ab_und_rechnet_keine_anrechnung() {
    let (g, erg) = lauf(&[
        (STEUER, json!(cent(700))),
        (EINKUENFTE, json!(cent(5_000))),
        (WAHL, json!(true)),
    ]);
    assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 700);
    assert_eq!(erg.dba_anrechnung.get(), 0, "der Abzug rechnet nichts an");
    assert_eq!(g.anzurechnende_auslaendische_steuern.get(), 0);
    assert_eq!(erg.p32b_progressionseinkuenfte, None);
    assert_eq!(zve(&g), 49_264);
}

/// Der Abzug haengt an BEIDEN Betraegen: ohne Auslandseinkuenfte oder ohne gezahlte Steuer gibt es nichts abzuziehen. Mit
/// der Wahl `true` bleibt das Einkommen dann unberuehrt, und es wird auch nicht angerechnet (der Hoechstbetrag ist 0 bzw.
/// die gezahlte Steuer 0).
#[test]
fn abzug_braucht_steuer_und_auslandseinkuenfte() {
    let ohne_einkuenfte = lauf(&[(STEUER, json!(cent(700))), (WAHL, json!(true))]);
    let ohne_steuer = lauf(&[(EINKUENFTE, json!(cent(5_000))), (WAHL, json!(true))]);
    for (name, (g, erg)) in [
        ("ohne Einkuenfte", ohne_einkuenfte),
        ("ohne Steuer", ohne_steuer),
    ] {
        assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 0, "{name}");
        assert_eq!(erg.dba_anrechnung.get(), 0, "{name}");
        assert_eq!(zve(&g), 49_964, "{name}");
    }
}

/// Ohne gezahlte Steuer gibt es nichts abzuziehen: die Wahl `true` aendert dann nichts an der Freistellung. Die 5.000 Euro
/// gehen in den Progressionsvorbehalt (`dba_methode` = Freistellung), genau wie ohne die Wahl. Mit Steuer ueber 0 gilt seit
/// Nr. 35 dasselbe (Test `bei_freistellung_ueber_dba_methode_gibt_es_keinen_abzug`); Python buchte dort den Abzug.
#[test]
fn ohne_steuer_bleibt_die_freistellung_im_progressionsvorbehalt() {
    for wahl in [true, false] {
        let (g, erg) = lauf(&[
            (EINKUENFTE, json!(cent(5_000))),
            ("dba_methode", json!("dba_freistellung")),
            (WAHL, json!(wahl)),
        ]);
        assert_eq!(
            erg.p32b_progressionseinkuenfte,
            Some(Euro::new(5_000)),
            "Wahl {wahl}"
        );
        assert_eq!(erg.dba_anrechnung.get(), 0, "Wahl {wahl}");
        assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 0, "Wahl {wahl}");
    }
}

/// Abweichung Nr. 35 (§ 34c Abs. 6 S. 1 und 2 `EStG`): bei Freistellung gibt es keinen Abzug. Das Abkommen stellt die
/// Auslandseinkuenfte steuerfrei; Abs. 2 gilt nur dort, wo das Abkommen die Anrechnung vorsieht. Die Wahl `true` aendert
/// dann nichts: weder Abzug noch Anrechnung, die 5.000 Euro gehen in den Progressionsvorbehalt, das zvE bleibt bei
/// 49.964 Euro. Der Lauf mit Wahl `false` muss dasselbe zeigen.
#[test]
fn bei_freistellung_ueber_dba_methode_gibt_es_keinen_abzug() {
    for wahl in [true, false] {
        let (g, erg) = lauf(&[
            (STEUER, json!(cent(700))),
            (EINKUENFTE, json!(cent(5_000))),
            ("dba_methode", json!("dba_freistellung")),
            (WAHL, json!(wahl)),
        ]);
        assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 0, "Wahl {wahl}");
        assert_eq!(erg.dba_anrechnung.get(), 0, "Wahl {wahl}");
        assert_eq!(
            g.anzurechnende_auslaendische_steuern.get(),
            0,
            "Wahl {wahl}"
        );
        assert_eq!(
            erg.p32b_progressionseinkuenfte,
            Some(Euro::new(5_000)),
            "Wahl {wahl}"
        );
        assert_eq!(zve(&g), 49_964, "Wahl {wahl}");
    }
}

/// Dasselbe, wenn die Freistellung aus Staat und Einkunftsart folgt (`dba_methode_fuer`, nicht aus `dba_methode`): USA und
/// Oesterreich je pauschal, Polen nur fuer Ruhegehaelter. Beide Wege zur Methode muessen den Abzug ausschliessen.
#[test]
fn bei_freistellung_ueber_staat_und_einkunftsart_gibt_es_keinen_abzug() {
    let faelle: [(&str, Option<&str>); 3] =
        [("us", None), ("at", None), ("pl", Some("ruhegehaelter"))];
    for (staat, art) in faelle {
        let mut paare = vec![
            (STEUER, json!(cent(700))),
            (EINKUENFTE, json!(cent(5_000))),
            ("dba_staat", json!(staat)),
            (WAHL, json!(true)),
        ];
        if let Some(a) = art {
            paare.push(("dba_einkunftsart", json!(a)));
        }
        let (g, erg) = lauf(&paare);
        let fall = format!("{staat} {art:?}");
        assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 0, "{fall}");
        assert_eq!(erg.dba_anrechnung.get(), 0, "{fall}");
        assert_eq!(
            erg.p32b_progressionseinkuenfte,
            Some(Euro::new(5_000)),
            "{fall}"
        );
        assert_eq!(zve(&g), 49_964, "{fall}");
    }
}

/// KONTROLLE zu Nr. 35: sieht das Abkommen die Anrechnung vor, bleibt der Abzug erlaubt (§ 34c Abs. 6 S. 2). Niederlande
/// pauschal, Polen fuer Dividenden: die 700 Euro mindern das Einkommen wie ohne Abkommen, nichts wird angerechnet, kein
/// Progressionsvorbehalt. Ohne diese Kontrolle bestuende der Fix auch, wenn er den Abzug ueberall abschaltete.
#[test]
fn bei_anrechnungsabkommen_bleibt_der_abzug_erlaubt() {
    let faelle: [(Option<&str>, Option<&str>); 3] = [
        (None, None),
        (Some("nl"), None),
        (Some("pl"), Some("dividenden")),
    ];
    for (staat, art) in faelle {
        let mut paare = vec![
            (STEUER, json!(cent(700))),
            (EINKUENFTE, json!(cent(5_000))),
            (WAHL, json!(true)),
        ];
        if let Some(s) = staat {
            paare.push(("dba_staat", json!(s)));
        }
        if let Some(a) = art {
            paare.push(("dba_einkunftsart", json!(a)));
        }
        let (g, erg) = lauf(&paare);
        let fall = format!("{staat:?} {art:?}");
        assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 700, "{fall}");
        assert_eq!(erg.dba_anrechnung.get(), 0, "{fall}");
        assert_eq!(erg.p32b_progressionseinkuenfte, None, "{fall}");
        assert_eq!(zve(&g), 49_264, "{fall}");
    }
}

/// Nur der Wert `true` waehlt den Abzug: ein Text, eine Zahl oder `false` bleiben bei der Anrechnung (`ist_true`).
#[test]
fn nur_ein_ausdruecklich_wahres_feld_waehlt_den_abzug() {
    for wahl in [json!(false), json!("ja"), json!(1), Value::Null] {
        let (g, erg) = lauf(&[
            (STEUER, json!(cent(700))),
            (EINKUENFTE, json!(cent(5_000))),
            (WAHL, wahl.clone()),
        ]);
        assert_eq!(g.sonstige_abzuege_vom_einkommen.get(), 0, "Wahl {wahl}");
        assert_eq!(erg.dba_anrechnung.get(), 700, "Wahl {wahl}");
    }
}
