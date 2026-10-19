//! Gesamtlauf (`gesamt`), Zusammenveranlagung: der Ring rechnet den Versorgungsbezug des Ehegatten (§ 19 Abs. 2 `EStG`) mit
//! Versorgungsfreibetrag, Zuschlag und Pauschbetrag. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Die Scheibe `gesamt` nahm die fuenf Versorgungsfelder des Ehegatten nicht an (Abweichung Nr. 87). Der Ring
//! (`zweige/gesamt.rs::einkuenfte_ns_aus_lohn`, Person B) rechnet sie seit Abweichung Nr. 33 fuer die Rentner-Scheibe; diese Datei
//! haelt fest, dass er es auch im Gesamtlauf mit den Zahlen tut, die ein Mensch nach Gesetz erwartet. Die Scheibe misst
//! `api/tests/gesamt_ehegatte_versorgung_hermetisch.rs` auf dem echten Weg `POST /event`.
//!
//! **Warum es zaehlt.** Ohne die Felder zaehlte ein Bezug des Ehegatten als voller Arbeitslohn: Arbeitnehmer-Pauschbetrag statt
//! Freibetrag, Zuschlag und Versorgungs-Pauschbetrag, die Steuer war zu hoch (Richtung aus dem Code abgeleitet).
//!
//! **Wo es sitzt.** `zweige/gesamt.rs` (`VERSORGUNG_B`, `versorgungsbezug`, `einkuenfte_aus_versorgung`), Alters-Gate des Ehegatten
//! mit `rentner_grad_der_behinderung_partner` (Nr. 84, verneinte Behinderung gilt als Grad 0, gilt NUR fuer Person A).
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand gerechnet aus `params/` und dem Gesetzestext, nie aus dem Rust-Code gelesen
//! (Skript `handrechnung.py` im Auftragsordner, es liest `params/` per Regex und ruft keinen Rust-Code auf). Rechenweg:
//! - Versorgungseinkuenfte = Bezug − Versorgungsfreibetrag − Zuschlag − 102 Euro (§ 9a Satz 1 Nr. 1 Buchstabe b `EStG`). Freibetrag
//!   = min(Prozentsatz × Bemessungsgrundlage, Hoechstbetrag), Zuschlag je Kohorte des Versorgungsbeginns
//!   (`params/kohorten/versorgungsfreibetrag_p19_2.yaml`, `estg_p19_2026-07-17.txt`): 2025: 13,2 % / 990 / 297, 2005: 40,0 % /
//!   3.000 / 900.
//! - Der Lohn (Nr. 3 der Lohnsteuerbescheinigung) enthaelt den Bezug (Nr. 8): hier Lohn gleich Bezug, also Arbeitslohn 0, es
//!   bleibt die Versorgung (Abweichung Nr. 50).
//! - zvE = Gesamtbetrag der Einkuenfte − 72 Euro (Sonderausgaben-Pauschbetrag 36, bei Zusammenveranlagung verdoppelt, § 10c
//!   `EStG`, `params/2025/sonderausgabenpauschbetrag.yaml`); Splitting: die Steuer ist das Doppelte der Steuer auf die Haelfte
//!   des zvE (§ 32a Abs. 5). Die zvE unten sind gerade, die Haelfte stellt keine Rundungsfrage. Tarif § 32a Abs. 1 aus
//!   `params/2025/einkommensteuertarif_p32a.yaml`, auf volle Euro abgerundet.
//! - H1 Beginn 2025, 30.001 Euro: 30.001 − 990 − 297 − 102 = 28.612; zvE 28.540; Haelfte 14.270, Zone 2: y = 0,2174,
//!   (932,30 y + 1.400) y = 348,4…, abgerundet 348; Steuer 696.
//! - H2 Beginn 2025, 60.001 Euro: 60.001 − 1.389 = 58.612; zvE 58.540; Haelfte 29.270, Zone 3: 8.194.
//! - H3 Beginn 2005, 30.000 Euro: 30.000 − 3.900 − 102 = 25.998; zvE 25.926; Steuer 256.
//! - H4 Beginn 2025, Bemessungsgrundlage 5.000, 30.001 Euro: Freibetrag 660, Zuschlag 297: 30.001 − 957 − 102 = 28.942; zvE
//!   28.870; Steuer 756.
//! - H5 / H6 A 30.001 und B 60.001 (und vertauscht): 28.612 + 58.612 = 87.224; zvE 87.152; Steuer 16.970.
//! - Alters-Gate (§ 19 Abs. 2 Satz 2 Nr. 2): Beginn mit 62 gilt als Arbeitslohn (ohne Freibetrag, mit Arbeitnehmer-Pauschbetrag
//!   1.230): 30.000 − 1.230 = 28.770; Beginn mit 63 (60 bei Grad der Behinderung ab 50) als Versorgungsbezug: 28.611.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types,
    clippy::too_many_lines
)]

use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::ausgaben::Kette;
use bescheid::zweige::Umgebung;
use bescheid::{Felder, Instanzquelle};
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;

type Paare = Vec<(&'static str, Value)>;

/// Euro in Cent.
const fn cent(euro: i64) -> i64 {
    euro * 100
}

/// `tests/_kegel.py::standardwert`: der Abwesenheitswert, nie der illustrative Beispielwert.
fn standardwert(b: &Bindung) -> Value {
    if let Some(w) = &b.abwesenheitswert {
        return w.clone();
    }
    match b.typ {
        Feldtyp::Bool => json!(b.feld_id.starts_with("kein_") || b.feld_id.starts_with("keine_")),
        Feldtyp::Cent | Feldtyp::Int => json!(0),
        Feldtyp::Enum => b.beispielwert.clone(),
        _ => panic!(
            "KONTROLLE: Kegel-Feld {} hat keinen Abwesenheitswert",
            b.feld_id
        ),
    }
}

/// `tests/_kegel.py::kegel_fuer`: der volle Pflicht-Kegel; was der Test setzt, gewinnt, der Rest folgt bestaetigt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(&'static str, Value)]) -> Paare {
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .rev()
                .find(|(g, _)| g == f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            (*f, w)
        })
        .collect();
    raus.extend(gesetzt.iter().filter(|(f, _)| !kegel.contains(f)).cloned());
    raus
}

/// Ein Fall der Scheibe `gesamt`, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(paare: &[(&'static str, Value)]) -> Fall {
    let cfg = Cfg::fuer(Scheibe::Gesamt);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let events: Vec<(&str, Value, bool)> = kegel_fuer(&cfg, paare)
        .into_iter()
        .map(|(f, w)| (f, w, true))
        .collect();
    let store = store(&events);
    Fall {
        cfg,
        index: teil.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        achsen: teil.iter().map(|b| AchsenBindung::from(*b)).collect(),
        felder: felder(&store),
        store,
    }
}

/// Die festzusetzende Einkommensteuer in vollen Euro und die Rechenweg-Kette. Erst der K2-Guard, dann `feste_zahl`; jeder
/// andere Ausgang ist ein Fehlschlag des Tests.
fn lauf(paare: &[(&'static str, Value)]) -> (i64, Kette) {
    let f = fall(paare);
    let q = Instanzquelle {
        store: Some(&f.store),
        bindung: Some(&f.index),
        nur_bestaetigt: false,
    };
    match an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q) {
        Ok(None) => {}
        Ok(Some(g)) => panic!("KONTROLLE: erwartet eine Zahl, bekommen die Sperre {g}"),
        Err(e) => panic!("KONTROLLE: Guard: {e:?}"),
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => {
            assert_eq!(z.zahl.get() % 100, 0, "Cent-Rest in der Steuer: {z:?}");
            (
                z.zahl.get() / 100,
                z.extras.kette.expect("KONTROLLE: Kette fehlt"),
            )
        }
        Ok(Err(k)) => panic!("KONTROLLE: ohne Zahl: {:?}", k.grund),
        Err(e) => panic!("KONTROLLE: {e:?}"),
    }
}

/// Gesamtbetrag der Einkuenfte in Euro.
fn gdb(paare: &[(&'static str, Value)]) -> i64 {
    lauf(paare).1.gesamtbetrag_der_einkuenfte.get()
}

/// Ein Paar, ZUSAMMEN veranlagt, mit dem Lohn des Ehegatten `lohn_b` und dem Lohn von Person A `lohn_a` (Euro), alle Kreuze
/// "nein"/"kein" (der Pflicht-Kegel), der Partner-Kegel (Versicherungsart, fuenf Kapital-Felder) auf "keine Angabe", plus `mehr`.
fn paar(lohn_a: i64, lohn_b: i64, mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(cent(lohn_a))),
        ("bruttoarbeitslohn_partner", json!(cent(lohn_b))),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(true)),
        ("versicherungsart_partner", json!("gesetzlich_an")),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
    ];
    for (f, w) in mehr {
        match p.iter_mut().find(|(g, _)| *g == f) {
            Some(e) => e.1 = w,
            None => p.push((f, w)),
        }
    }
    p
}

/// Die vier Versorgungsfelder des Ehegatten fuer einen beamtenrechtlichen Bezug: `euro`, Bemessungsgrundlage `bemessung`, Beginn.
fn versorgung_b(euro: i64, bemessung: i64, beginn: i64) -> Paare {
    vec![
        ("versorgung_jahresrente_partner", json!(cent(euro))),
        ("versorgung_bemessungsgrundlage_partner", json!(cent(bemessung))),
        ("versorgung_beginn_jahr_partner", json!(beginn)),
        ("versorgung_art_partner", json!("beamtenrechtlich")),
    ]
}

/// Dieselben vier Felder fuer Person A.
fn versorgung_a(euro: i64, bemessung: i64, beginn: i64) -> Paare {
    vec![
        ("versorgung_jahresrente", json!(cent(euro))),
        ("versorgung_bemessungsgrundlage", json!(cent(bemessung))),
        ("versorgung_beginn_jahr", json!(beginn)),
        ("versorgung_art", json!("beamtenrechtlich")),
    ]
}

/// Prueft (Gesamtbetrag der Einkuenfte, zu versteuerndes Einkommen, festzusetzende Einkommensteuer) eines Falls.
fn pruefe(was: &str, paare: &[(&'static str, Value)], gdb: i64, zve: i64, est: i64) {
    let (steuer, kette) = lauf(paare);
    assert_eq!(
        (kette.gesamtbetrag_der_einkuenfte.get(), kette.zu_versteuerndes_einkommen.get(), steuer),
        (gdb, zve, est),
        "{was}: (Gesamtbetrag, zvE, Steuer)"
    );
}

// ---------------------------------------------------------------- Bezug des Ehegatten

/// H1 und H2: 30.001 und 60.001 Euro Bezug des Ehegatten, Beginn 2025. Heute (vor Nr. 87): der Bezug ist im Gesamtlauf nicht
/// eintragbar.
#[test]
fn der_bezug_des_ehegatten_geht_nach_freibetrag_zuschlag_und_pauschbetrag_in_die_steuer() {
    // Ohne jede Einkunft: Gesamtbetrag 0 und Steuer 0 (das zvE zeigt die Kette dort als −72, es ist nicht auf 0 geklemmt; die Steuer ist 0).
    let (steuer, kette) = lauf(&paar(0, 0, vec![]));
    assert_eq!((kette.gesamtbetrag_der_einkuenfte.get(), steuer), (0, 0), "ohne Bezug");
    pruefe("H1 30.001", &paar(0, 30_001, versorgung_b(30_001, 30_001, 2025)), 28_612, 28_540, 696);
    pruefe("H2 60.001", &paar(0, 60_001, versorgung_b(60_001, 60_001, 2025)), 58_612, 58_540, 8_194);
}

/// H3 und H4: der Freibetrag haengt am Beginnjahr und an der Bemessungsgrundlage des EHEGATTEN.
#[test]
fn der_freibetrag_folgt_beginnjahr_und_bemessung_des_ehegatten() {
    pruefe("H3 Beginn 2005", &paar(0, 30_000, versorgung_b(30_000, 30_000, 2005)), 25_998, 25_926, 256);
    pruefe("H4 Bemessung 5.000", &paar(0, 30_001, versorgung_b(30_001, 5_000, 2025)), 28_942, 28_870, 756);
}

/// H5 bis H7: Person A und der Ehegatte haben je eine eigene Versorgung. Nur A und nur B rechnen dieselbe Zahl (H7, H1); beide
/// zusammen die Summe, vertauscht dasselbe.
#[test]
fn die_versorgung_von_person_a_und_ehegatte_zaehlt_je_fuer_sich() {
    pruefe("H7 nur A 30.001", &paar(30_001, 0, versorgung_a(30_001, 30_001, 2025)), 28_612, 28_540, 696);
    pruefe("H1 nur B 30.001", &paar(0, 30_001, versorgung_b(30_001, 30_001, 2025)), 28_612, 28_540, 696);
    let a_klein = [versorgung_a(30_001, 30_001, 2025), versorgung_b(60_001, 60_001, 2025)].concat();
    let a_gross = [versorgung_a(60_001, 60_001, 2025), versorgung_b(30_001, 30_001, 2025)].concat();
    pruefe("H5 A 30.001, B 60.001", &paar(30_001, 60_001, a_klein), 87_224, 87_152, 16_970);
    pruefe("H6 A 60.001, B 30.001", &paar(60_001, 30_001, a_gross), 87_224, 87_152, 16_970);
}

/// Bei Einzelveranlagung zaehlt der Ehegatte nicht: Lohn und Bezug im Store aendern nichts.
#[test]
fn bei_einzelveranlagung_zaehlt_der_bezug_des_ehegatten_nicht() {
    let einzel = |mehr: Paare| paar(0, 0, [vec![("veranlagung", json!("einzel"))], mehr].concat());
    let ohne = gdb(&einzel(vec![]));
    let mit = gdb(&einzel([vec![("bruttoarbeitslohn_partner", json!(cent(30_001)))], versorgung_b(30_001, 30_001, 2025)].concat()));
    assert_eq!(ohne, 0);
    assert_eq!(mit, ohne, "Einzelveranlagung: der Ehegatte zaehlt nicht");
}

// ---------------------------------------------------------------- Alters-Gate des Ehegatten

/// Alters-Gate des Ehegatten (§ 19 Abs. 2 Satz 2 Nr. 2): Beginn mit 62 gilt als Arbeitslohn (28.770), mit 63 als Versorgungsbezug
/// (28.611). Ab Grad der Behinderung 50 DES EHEGATTEN liegt die Grenze bei 60. Der Grad von Person A (`rentner_grad_der_behinderung`
/// und `versorgung_grad_der_behinderung`) und die Verneinung von Person A (`keine_behinderung_pflege`, Nr. 84) zaehlen fuer den
/// Ehegatten nicht.
#[test]
fn das_alters_gate_gilt_fuer_den_ehegatten_mit_seinem_eigenen_grad() {
    let altersgrenze = |alter: i64, grad_b: i64, mehr_a: Paare| {
        let mut p = versorgung_b(30_000, 30_000, 2025);
        p.push(("versorgung_art_partner", json!("altersgrenze_sonstige")));
        p.push(("versorgung_alter_bei_beginn_partner", json!(alter)));
        p.push(("rentner_grad_der_behinderung_partner", json!(grad_b)));
        p.extend(mehr_a);
        gdb(&paar(0, 30_000, p))
    };
    assert_eq!(altersgrenze(62, 0, vec![]), 28_770, "62 Jahre: Arbeitslohn");
    assert_eq!(altersgrenze(63, 0, vec![]), 28_611, "63 Jahre: Versorgungsbezug");
    assert_eq!(altersgrenze(59, 50, vec![]), 28_770, "59 Jahre, Grad 50: Arbeitslohn");
    assert_eq!(altersgrenze(60, 50, vec![]), 28_611, "60 Jahre, Grad 50: Versorgungsbezug");
    assert_eq!(
        altersgrenze(60, 0, vec![("rentner_grad_der_behinderung", json!(50)), ("versorgung_grad_der_behinderung", json!(50))]),
        28_770,
        "60 Jahre, Grad 50 nur bei Person A: der Grad des Ehegatten entscheidet, also Arbeitslohn"
    );
    assert_eq!(
        altersgrenze(60, 0, vec![("keine_behinderung_pflege", json!(true))]),
        28_770,
        "60 Jahre, Person A verneint eine Behinderung (Nr. 84): fuer den Ehegatten ohne Wirkung, die Grenze bleibt 63"
    );
}
