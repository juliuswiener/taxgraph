//! Hermetisch (Abweichung Nr. 45): ein Geraet, das keinen Sofortabzug bekommt (ueber 250 EUR ohne Verzeichnis,
//! nicht selbstaendig nutzbar, ueber 800 EUR), wird im Kaufjahr abgeschrieben (§ 7 Abs. 1 `EStG`, ueber § 4 Abs. 3 S. 3),
//! statt die Zahl zu sperren. Der Nutzer nennt Nutzungsdauer und Kaufmonat; Folgejahre traegt er weiter in
//! `afa_jahresbetrag` ein.
//!
//! Soll von Hand aus § 7 Abs. 1 S. 1 und S. 4 (`sources/gesetze-im-internet/estg_p7_2026-07-14.txt`): Jahresbetrag =
//! Anschaffungskosten / Nutzungsdauer; im Anschaffungsjahr ein Zwoelftel weniger je VOLLEM Monat VOR dem Anschaffungsmonat,
//! also `(13 - Monat) / 12` des Jahresbetrags. Die Rechnung rundet auf volle Euro ab (`p7_linear_afa`).
//! Referenz fuer die Steuer: derselbe Fall OHNE Geraet, aber mit dem Soll-Betrag im Handfeld `afa_jahresbetrag`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg, KeineZahl};
use bescheid::einkuenfte::gwg_afa_summe;
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::{Felder, Instanzquelle};
use bindung::{Bindung, Bindungspunkt};
use domain::{Feldtyp, Scheibe, Sperrgrund, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZS: [Vz; 3] = [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026];

/// `dict(paare)`: der letzte Wert je Feld gewinnt, die Reihenfolge ist die des ersten Auftretens.
fn als_dict(paare: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let mut raus: Vec<(&'static str, Value)> = Vec::new();
    for (f, w) in paare {
        match raus.iter_mut().find(|(g, _)| g == f) {
            Some(e) => e.1 = w.clone(),
            None => raus.push((f, w.clone())),
        }
    }
    raus
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

/// `tests/_kegel.py::kegel_fuer`: der volle Pflicht-Kegel; was der Test setzt, gewinnt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(&'static str, Value)]) -> Vec<(&'static str, Value)> {
    let gesetzt = als_dict(gesetzt);
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Vec<(&'static str, Value)> = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .find(|(g, _)| g == f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            (*f, w)
        })
        .collect();
    raus.extend(gesetzt.into_iter().filter(|(f, _)| !kegel.contains(f)));
    raus
}

/// Ein Fall einer Scheibe, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

/// `paare` kommen bestaetigt ueber den Kegel-Bauer; `vorlaeufig` danach als `zustand=vorlaeufig`,
/// und der Bauer belegt diese Felder nicht vor.
fn fall(
    scheibe: Scheibe,
    paare: &[(&'static str, Value)],
    vorlaeufig: &[(&'static str, Value)],
) -> Fall {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: {scheibe} liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let events: Vec<(&str, Value, bool)> = kegel_fuer(&cfg, paare)
        .into_iter()
        .filter(|(f, _)| !vorlaeufig.iter().any(|(v, _)| v == f))
        .map(|(f, w)| (f, w, true))
        .chain(vorlaeufig.iter().map(|(f, w)| (*f, w.clone(), false)))
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

/// `grund` aus `_ergebnis_roh`: bestaetigt, ein Sperrgrund des Guards oder eine Lage ohne Zahl.
#[derive(Debug, PartialEq)]
enum Grund {
    Bestaetigt,
    Sperre(Sperrgrund),
    KeineZahl(KeineZahl),
}

/// `_ergebnis_roh` (`api.py:568`) ohne HTTP: erst der K2-Guard, dann `feste_zahl`.
fn ergebnis(f: &Fall, vz: Vz) -> (Grund, Option<i64>) {
    if f.cfg.guard() {
        let q = Instanzquelle {
            store: Some(&f.store),
            bindung: Some(&f.index),
            nur_bestaetigt: false,
        };
        if let Some(g) = an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(vz), &q).unwrap() {
            return (Grund::Sperre(g), None);
        }
    }
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, vz, &kegel, &umg, Some(&f.store), None).unwrap() {
        Ok(z) => (Grund::Bestaetigt, Some(z.zahl.get())),
        Err(k) => (Grund::KeineZahl(k.grund), None),
    }
}

/// `(Name, Lohn, Einnahmen)` in Cent: die zwei Einkommen der bestehenden GWG-Tests.
const EINKOMMEN: [(&str, i64, i64); 2] = [
    ("Lohn 60.000", 6_000_000, 0),
    ("Einnahmen 50.000", 0, 5_000_000),
];

fn basis(lohn: i64, einnahmen: i64, afa: i64) -> Vec<(&'static str, Value)> {
    vec![
        ("bruttoarbeitslohn", json!(lohn)),
        ("vv_entgelt_quote_prozent", json!(100)),
        ("kein_gewinn", json!(false)),
        ("betriebseinnahmen", json!(einnahmen)),
        ("sonstige_betriebsausgaben", json!(0)),
        ("afa_jahresbetrag", json!(afa)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ]
}

#[derive(Clone, Copy)]
struct Geraet {
    betrag: i64,
    nutzbar: bool,
    netto: bool,
    ohne_abzug: Option<bool>,
    verzeichnis: bool,
    nd: Option<i64>,
    monat: Option<i64>,
}

/// 600 EUR, alle Voraussetzungen ja: Sofortabzug.
const GUT: Geraet = Geraet {
    betrag: 60_000,
    nutzbar: true,
    netto: true,
    ohne_abzug: None,
    verzeichnis: true,
    nd: None,
    monat: None,
};

/// Der Feldname in der ersten oder zweiten `gwg`-Instanz (`__2`).
fn name(zweite: bool, feld: &'static str) -> &'static str {
    if !zweite {
        return feld;
    }
    match feld {
        "gwg_anschaffungskosten_netto" => "gwg_anschaffungskosten_netto__2",
        "gwg_bewegliches_selbstaendig_nutzbar" => "gwg_bewegliches_selbstaendig_nutzbar__2",
        "gwg_netto_ohne_vorsteuer" => "gwg_netto_ohne_vorsteuer__2",
        "gwg_verzeichnis_ab_250" => "gwg_verzeichnis_ab_250__2",
        "gwg_ohne_vorsteuerabzug" => "gwg_ohne_vorsteuerabzug__2",
        "gwg_nutzungsdauer" => "gwg_nutzungsdauer__2",
        "gwg_kaufmonat" => "gwg_kaufmonat__2",
        _ => panic!("KONTROLLE: kein Name fuer die zweite Instanz: {feld}"),
    }
}

fn geraet_paare(
    mut paare: Vec<(&'static str, Value)>,
    g: Geraet,
    zweite: bool,
) -> Vec<(&'static str, Value)> {
    paare.push((name(zweite, "gwg_anschaffungskosten_netto"), json!(g.betrag)));
    paare.push((name(zweite, "gwg_bewegliches_selbstaendig_nutzbar"), json!(g.nutzbar)));
    paare.push((name(zweite, "gwg_netto_ohne_vorsteuer"), json!(g.netto)));
    paare.push((name(zweite, "gwg_verzeichnis_ab_250"), json!(g.verzeichnis)));
    if let Some(a) = g.ohne_abzug {
        paare.push((name(zweite, "gwg_ohne_vorsteuerabzug"), json!(a)));
    }
    if let Some(nd) = g.nd {
        paare.push((name(zweite, "gwg_nutzungsdauer"), json!(nd)));
    }
    if let Some(m) = g.monat {
        paare.push((name(zweite, "gwg_kaufmonat"), json!(m)));
    }
    paare
}

fn mit_geraet(lohn: i64, einnahmen: i64, g: Geraet) -> Fall {
    fall(Scheibe::Gesamt, &geraet_paare(basis(lohn, einnahmen, 0), g, false), &[])
}

/// Steuer (Cent) des Falls ohne Geraet, aber mit `afa_euro` im Handfeld `afa_jahresbetrag`.
fn referenz(vz: Vz, lohn: i64, einnahmen: i64, afa_euro: i64) -> Option<i64> {
    let f = fall(Scheibe::Gesamt, &basis(lohn, einnahmen, afa_euro * 100), &[]);
    let (grund, zahl) = ergebnis(&f, vz);
    assert_eq!(grund, Grund::Bestaetigt, "KONTROLLE: die Referenz rechnet");
    zahl
}

/// KONTROLLE: der Messaufbau ist nicht blind -- ein `AfA`-Betrag im Handfeld aendert die Steuer, in jedem Jahr und Einkommen.
#[test]
fn die_referenz_bewegt_die_steuer() {
    for vz in VZS {
        for (name, lohn, einn) in EINKOMMEN {
            let ohne = referenz(vz, lohn, einn, 0);
            let mit = referenz(vz, lohn, einn, 200);
            assert!(mit < ohne, "KONTROLLE: Messung blind {} {name}", vz.jahr());
        }
    }
}

/// `(Name, Geraet, Soll-AfA in EURO von Hand)`.
fn faelle_afa() -> Vec<(&'static str, Geraet, i64)> {
    let mk = |g: Geraet, nd: i64, monat: i64| Geraet {
        nd: Some(nd),
        monat: Some(monat),
        ..g
    };
    let t1 = Geraet { verzeichnis: false, ..GUT };
    let t2 = Geraet { nutzbar: false, ..GUT };
    let t3 = Geraet { betrag: 100_000, ..GUT };
    vec![
        ("T1 600 ohne Verzeichnis, 3 Jahre, Monat 1 (200 x 12/12)", mk(t1, 3, 1), 200),
        ("T1 600 ohne Verzeichnis, 3 Jahre, Monat 7 (200 x 6/12)", mk(t1, 3, 7), 100),
        ("T1 600 ohne Verzeichnis, 3 Jahre, Monat 12 (200 x 1/12 = 16,67 -> 16)", mk(t1, 3, 12), 16),
        ("T1 600 ohne Verzeichnis, 5 Jahre, Monat 1 (120 x 12/12)", mk(t1, 5, 1), 120),
        ("T1 600 ohne Verzeichnis, 5 Jahre, Monat 7 (120 x 6/12)", mk(t1, 5, 7), 60),
        ("T1 600 ohne Verzeichnis, 5 Jahre, Monat 12 (120 x 1/12)", mk(t1, 5, 12), 10),
        ("T2 600 nicht allein nutzbar, 3 Jahre, Monat 7", mk(t2, 3, 7), 100),
        ("T2 600 nicht allein nutzbar, 5 Jahre, Monat 1", mk(t2, 5, 1), 120),
        ("T3 1000 ueber 800, 5 Jahre, Monat 1 (200 x 12/12)", mk(t3, 5, 1), 200),
        ("T3 1000 ueber 800, 4 Jahre, Monat 10 (250 x 3/12 = 62,5 -> 62)", mk(t3, 4, 10), 62),
    ]
}

/// Jeder Fall gibt `Bestaetigt` mit der Steuer der Referenz (AK6). Vor dem Bau: Sperre `gwg_abschreibung_offen`.
#[test]
fn einzel_afa_statt_sperre() {
    let mut abweichungen = Vec::new();
    let mut gesamt = 0;
    for vz in VZS {
        for (ek, lohn, einn) in EINKOMMEN {
            for (name, g, soll_afa) in faelle_afa() {
                let ist = ergebnis(&mit_geraet(lohn, einn, g), vz);
                let soll = (Grund::Bestaetigt, referenz(vz, lohn, einn, soll_afa));
                gesamt += 1;
                if ist != soll {
                    abweichungen.push(format!("{} {ek} | {name} | ist {ist:?} | soll {soll:?}", vz.jahr()));
                }
            }
        }
    }
    assert!(
        abweichungen.is_empty(),
        "DEFEKT: {} von {gesamt} Faellen geben keine Einzel-AfA:\n{}",
        abweichungen.len(),
        abweichungen.join("\n")
    );
}

/// Kontrollen, die vor dem Bau gruen sind und danach gruen bleiben muessen.
#[test]
fn kontrollen_bleiben_gruen() {
    for vz in VZS {
        for (ek, lohn, einn) in EINKOMMEN {
            // K1: alle Voraussetzungen ja, 600 EUR: Sofortabzug = Betriebsausgabe 600, auch MIT Nutzungsdauer und
            // Monat (kein Doppelabzug).
            let ref600 = referenz(vz, lohn, einn, 600);
            let k1 = ergebnis(&mit_geraet(lohn, einn, GUT), vz);
            let k1b = ergebnis(
                &mit_geraet(lohn, einn, Geraet { nd: Some(3), monat: Some(7), ..GUT }),
                vz,
            );
            assert_eq!(k1, (Grund::Bestaetigt, ref600), "K1 Sofortabzug {} {ek}", vz.jahr());
            assert_eq!(k1b, (Grund::Bestaetigt, ref600), "K1b Sofortabzug mit Angaben {} {ek}", vz.jahr());
            // K2: Betrag 0 sperrt nie.
            let k2 = ergebnis(
                &mit_geraet(lohn, einn, Geraet { betrag: 0, verzeichnis: false, ..GUT }),
                vz,
            );
            assert_eq!(k2, (Grund::Bestaetigt, referenz(vz, lohn, einn, 0)), "K2 Betrag 0 {} {ek}", vz.jahr());
            // K3: Ausloeser ohne Nutzungsdauer und Monat: bleibt gesperrt (fail-closed).
            let k3 = ergebnis(&mit_geraet(lohn, einn, Geraet { verzeichnis: false, ..GUT }), vz);
            assert_eq!(k3, (Grund::Sperre(Sperrgrund::GwgAbschreibungOffen), None), "K3 {} {ek}", vz.jahr());
            // K4: "netto: nein" ohne Folgefrage-ja: Mehrwertsteuer-Sperre; die Angaben aendern das nicht.
            let k4 = ergebnis(
                &mit_geraet(
                    lohn,
                    einn,
                    Geraet { netto: false, ohne_abzug: Some(false), nd: Some(3), monat: Some(7), ..GUT },
                ),
                vz,
            );
            assert_eq!(k4, (Grund::Sperre(Sperrgrund::GwgMehrwertsteuerOffen), None), "K4 {} {ek}", vz.jahr());
        }
    }
}

/// AK7: fehlt Nutzungsdauer oder Kaufmonat, ist einer ausserhalb des Bereichs oder nur vorlaeufig, bleibt die Sperre.
#[test]
fn ohne_bestaetigte_angaben_bleibt_die_sperre() {
    let t1 = Geraet { verzeichnis: false, ..GUT };
    let sperre = (Grund::Sperre(Sperrgrund::GwgAbschreibungOffen), None);
    let vz = Vz::Vz2025;
    // Nur eine der zwei Angaben.
    assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, Geraet { nd: Some(3), ..t1 }), vz), sperre, "nur Nutzungsdauer");
    assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, Geraet { monat: Some(7), ..t1 }), vz), sperre, "nur Monat");
    // Ausserhalb des Bereichs (Nutzungsdauer 1 bis 30, Monat 1 bis 12).
    for (nd, monat) in [(0, 7), (31, 7), (-3, 7), (3, 0), (3, 13), (3, -1)] {
        let g = Geraet { nd: Some(nd), monat: Some(monat), ..t1 };
        assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, g), vz), sperre, "Nutzungsdauer {nd}, Monat {monat}");
    }
    // Nur ganze Zahlen zaehlen: kein Bool (Pythons `True` ist 1), keine Kommazahl, kein Text.
    for (nd, monat) in [
        (json!(3), json!(true)),
        (json!(true), json!(7)),
        (json!(3), json!(7.5)),
        (json!(3.5), json!(7)),
        (json!(3), json!("7")),
    ] {
        let mut paare = geraet_paare(basis(0, 5_000_000, 0), t1, false);
        paare.push(("gwg_nutzungsdauer", nd.clone()));
        paare.push(("gwg_kaufmonat", monat.clone()));
        assert_eq!(ergebnis(&fall(Scheibe::Gesamt, &paare, &[]), vz), sperre, "Nutzungsdauer {nd}, Monat {monat}");
    }
    // Vorlaeufig zaehlt nicht (Zwei-Signal-Regel).
    let paare = geraet_paare(basis(0, 5_000_000, 0), t1, false);
    let beide = [
        ("gwg_nutzungsdauer", json!(3)),
        ("gwg_kaufmonat", json!(7)),
    ];
    for i in 0..2 {
        let mut bestaetigt = paare.clone();
        bestaetigt.push(beide[1 - i].clone());
        let ist = ergebnis(&fall(Scheibe::Gesamt, &bestaetigt, &[beide[i].clone()]), vz);
        assert_eq!(ist, sperre, "vorlaeufig: {}", beide[i].0);
    }
}

/// Ein nicht allein nutzbares Geraet ueber 250 EUR mit Angaben beantwortet trotzdem das Verzeichnis: ist die Antwort nur
/// vorlaeufig, ist die ganze Instanz vorlaeufig, und die festgesetzte Zahl liesse das Geraet still weg. Also sperrt es.
/// Mit bestaetigter Antwort (auch "nein") rechnet es.
#[test]
fn ein_vorlaeufiges_verzeichnis_sperrt_auch_das_nicht_allein_nutzbare_geraet() {
    let vz = Vz::Vz2025;
    let t2 = Geraet { nutzbar: false, nd: Some(3), monat: Some(7), ..GUT };
    let paare = geraet_paare(basis(0, 5_000_000, 0), t2, false);
    let offen = ergebnis(&fall(Scheibe::Gesamt, &paare, &[("gwg_verzeichnis_ab_250", json!(true))]), vz);
    assert_eq!(offen, (Grund::Sperre(Sperrgrund::GwgTatbestandOffen), None), "Verzeichnis vorlaeufig");
    let soll = (Grund::Bestaetigt, referenz(vz, 0, 5_000_000, 100));
    for verzeichnis in [true, false] {
        let g = Geraet { verzeichnis, ..t2 };
        assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, g), vz), soll, "Verzeichnis bestaetigt {verzeichnis}");
    }
}

/// Ueber 800 EUR rechnet die `AfA` auf dem eingegebenen Betrag. Ob er netto oder brutto ist, sagt die Netto-Frage: ist sie
/// offen (nie beantwortet oder nur vorlaeufig), kann der Betrag brutto sein und die `AfA` zu hoch (zu wenig Steuer). Bis
/// 800 EUR sperrt eine offene Netto-Frage mit `GwgTatbestandOffen`; ueber 800 EUR muss sie es auch.
#[test]
fn ueber_800_eur_sperrt_eine_offene_netto_frage() {
    let vz = Vz::Vz2025;
    let t3 = Geraet { betrag: 100_000, nd: Some(5), monat: Some(1), ..GUT }; // 1.000 EUR, 5 Jahre, Januar: 200 EUR
    let offen = (Grund::Sperre(Sperrgrund::GwgTatbestandOffen), None);
    let alle = geraet_paare(basis(0, 5_000_000, 0), t3, false);
    let ohne_netto: Vec<(&'static str, Value)> =
        alle.iter().filter(|(f, _)| *f != "gwg_netto_ohne_vorsteuer").cloned().collect();
    // Nie beantwortet.
    assert_eq!(ergebnis(&fall(Scheibe::Gesamt, &ohne_netto, &[]), vz), offen, "Netto-Frage unbeantwortet");
    // Nur vorlaeufig beantwortet.
    let vorlaeufig = [("gwg_netto_ohne_vorsteuer", json!(true))];
    assert_eq!(ergebnis(&fall(Scheibe::Gesamt, &ohne_netto, &vorlaeufig), vz), offen, "Netto-Frage vorlaeufig");
    // Kontrolle: bestaetigt "ja" rechnet die AfA (200 EUR).
    let soll = (Grund::Bestaetigt, referenz(vz, 0, 5_000_000, 200));
    assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, t3), vz), soll, "Netto-Frage bestaetigt");
}

/// AK8: die `AfA` rechnet nur, wenn die Grundlage feststeht. "Netto: nein" ohne Folgefrage-ja sperrt auch bei einem
/// nicht allein nutzbaren Geraet mit beantworteten Angaben; der Kleinunternehmer (Folgefrage ja) schreibt den Bruttobetrag
/// ab, ueber 800 EUR gilt das nicht.
#[test]
fn die_afa_rechnet_nur_bei_feststehender_grundlage() {
    let vz = Vz::Vz2025;
    let mk = |g: Geraet| Geraet { nd: Some(3), monat: Some(7), ..g };
    let mwst = (Grund::Sperre(Sperrgrund::GwgMehrwertsteuerOffen), None);
    let t2_brutto_nein = mk(Geraet { nutzbar: false, netto: false, ohne_abzug: Some(false), ..GUT });
    assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, t2_brutto_nein), vz), mwst, "T2, brutto, Regelbesteuerer");
    let t1_brutto_nein = mk(Geraet { verzeichnis: false, netto: false, ohne_abzug: Some(false), ..GUT });
    assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, t1_brutto_nein), vz), mwst, "T1, brutto, Regelbesteuerer");
    let t3_klein = mk(Geraet { betrag: 100_000, netto: false, ohne_abzug: Some(true), ..GUT });
    assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, t3_klein), vz), mwst, "T3, ueber 800 EUR brutto");
    // Kleinunternehmer: der Bruttobetrag 600 EUR, 3 Jahre, Monat 7 gibt 100 EUR.
    let soll = (Grund::Bestaetigt, referenz(vz, 0, 5_000_000, 100));
    for g in [
        Geraet { nutzbar: false, netto: false, ohne_abzug: Some(true), ..GUT },
        Geraet { verzeichnis: false, netto: false, ohne_abzug: Some(true), ..GUT },
    ] {
        assert_eq!(ergebnis(&mit_geraet(0, 5_000_000, mk(g)), vz), soll, "Kleinunternehmer");
    }
}

/// AK9: mehrere Geraete summieren; ein Sofortabzug-Geraet daneben bleibt Sofortabzug; ein Geraet, das nur in der
/// zweiten Instanz steht, zaehlt (der Gewinn-Zweig laeuft auch dann, wenn nur die `AfA`-Summe ueber 0 liegt).
#[test]
fn mehrere_geraete_summieren_und_die_zweite_instanz_zaehlt() {
    let vz = Vz::Vz2025;
    let t1_3_7 = Geraet { verzeichnis: false, nd: Some(3), monat: Some(7), ..GUT }; // 100 EUR
    let t2_5_1 = Geraet { nutzbar: false, nd: Some(5), monat: Some(1), ..GUT }; // 120 EUR

    // Zwei AfA-Geraete: 100 + 120 = 220.
    let paare = geraet_paare(geraet_paare(basis(0, 5_000_000, 0), t1_3_7, false), t2_5_1, true);
    let ist = ergebnis(&fall(Scheibe::Gesamt, &paare, &[]), vz);
    assert_eq!(ist, (Grund::Bestaetigt, referenz(vz, 0, 5_000_000, 220)), "zwei AfA-Geraete");

    // Ein Sofortabzug-Geraet (600) und ein AfA-Geraet (100) = 700.
    let paare = geraet_paare(geraet_paare(basis(0, 5_000_000, 0), GUT, false), t1_3_7, true);
    let ist = ergebnis(&fall(Scheibe::Gesamt, &paare, &[]), vz);
    assert_eq!(ist, (Grund::Bestaetigt, referenz(vz, 0, 5_000_000, 700)), "Sofortabzug neben AfA");

    // Nur die zweite Instanz traegt ein AfA-Geraet (120): ohne Basisfelder in der ersten.
    let paare = geraet_paare(basis(0, 5_000_000, 0), t2_5_1, true);
    let ist = ergebnis(&fall(Scheibe::Gesamt, &paare, &[]), vz);
    assert_eq!(ist, (Grund::Bestaetigt, referenz(vz, 0, 5_000_000, 120)), "nur Instanz 2");
}

/// AK6 fuer die zweite Scheibe: `rentner_gesamt` gibt dieselbe Sperre, also auch dieselbe `AfA`.
#[test]
fn die_rentner_scheibe_rechnet_die_afa() {
    let vz = Vz::Vz2025;
    let rentner = |paare: Vec<(&'static str, Value)>| {
        let mut paare = paare;
        paare.push(("rentner_renten_beginn_jahr", json!(2025)));
        fall(Scheibe::RentnerGesamt, &paare, &[])
    };
    let ohne = ergebnis(&rentner(basis(0, 5_000_000, 0)), vz);
    let mit_100 = ergebnis(&rentner(basis(0, 5_000_000, 10_000)), vz);
    assert_eq!(ohne.0, Grund::Bestaetigt, "KONTROLLE: die Referenz rechnet");
    assert!(mit_100.1 < ohne.1, "KONTROLLE: Messung blind");
    let g = Geraet { verzeichnis: false, nd: Some(3), monat: Some(7), ..GUT };
    let ist = ergebnis(&rentner(geraet_paare(basis(0, 5_000_000, 0), g, false)), vz);
    assert_eq!(ist, mit_100, "Rentner-Scheibe, 600 EUR ohne Verzeichnis, 3 Jahre, Monat 7");
}

// ------------------------------------------------------------------ Ueberlebende der Nachmessung in main (38887430)

/// Die Grenzen der Nutzungsdauer gelten: 30 Jahre ist gueltig (ein Mutant `1..=29` sperrte es), ebenso 1 Jahr. Soll von Hand:
/// 1.000 EUR / 30 = 33,33 -> 33 EUR (Monat 1); 1.000 EUR / 1 Jahr = 1.000 EUR (Monat 1) und 1.000 x 1/12 = 83,33 -> 83 (Monat 12).
#[test]
fn die_grenzen_der_nutzungsdauer_rechnen() {
    let t3 = Geraet { betrag: 100_000, ..GUT };
    for vz in VZS {
        for (ek, lohn, einn) in EINKOMMEN {
            for (nd, monat, soll_afa) in [(30, 1, 33), (1, 1, 1_000), (1, 12, 83)] {
                let g = Geraet { nd: Some(nd), monat: Some(monat), ..t3 };
                let soll = (Grund::Bestaetigt, referenz(vz, lohn, einn, soll_afa));
                assert_eq!(
                    ergebnis(&mit_geraet(lohn, einn, g), vz),
                    soll,
                    "{} {ek}: Nutzungsdauer {nd}, Monat {monat}",
                    vz.jahr()
                );
            }
        }
    }
}

/// Bis 250 EUR bleibt es beim Sofortabzug, auch wenn das Verzeichnis mit "nein" beantwortet ist (die Verzeichnispflicht
/// beginnt erst ueber 250 EUR, § 6 Abs. 2 S. 4 `EStG`) und Nutzungsdauer und Monat dastehen: kein Doppelabzug. Ein Cent
/// mehr (250,01 EUR) ohne Verzeichnis gibt keinen Sofortabzug mehr, sondern die `AfA`: 250,01 / 3 Jahre = 83,34 -> 83 EUR.
#[test]
fn bis_250_eur_bleibt_der_sofortabzug_auch_ohne_verzeichnis() {
    for vz in VZS {
        for (ek, lohn, einn) in EINKOMMEN {
            let g = |betrag| Geraet { betrag, verzeichnis: false, nd: Some(3), monat: Some(1), ..GUT };
            for (betrag, soll_abzug) in [(20_000, 200), (25_000, 250), (25_001, 83)] {
                let soll = (Grund::Bestaetigt, referenz(vz, lohn, einn, soll_abzug));
                assert_eq!(
                    ergebnis(&mit_geraet(lohn, einn, g(betrag)), vz),
                    soll,
                    "{} {ek}: {betrag} Cent ohne Verzeichnis",
                    vz.jahr()
                );
            }
        }
    }
}

/// `gwg_afa_summe` ohne Store und Bindung: die Felder der ersten Instanz, wie sie die Schaetzung liest. Das ist die
/// Rechnung HINTER der Sperre (`sperre::gesamt::gwg`): in der festgesetzten Zahl faengt die Sperre "netto: nein" ab, die
/// Schaetzung rechnet trotzdem, also muss die Rechnung dieselbe Grundlage selbst verlangen.
fn afa_direkt(g: Geraet) -> i64 {
    let events: Vec<(&str, Value, bool)> =
        geraet_paare(Vec::new(), g, false).into_iter().map(|(f, w)| (f, w, true)).collect();
    let q = Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
    gwg_afa_summe(&felder(&store(&events)), &q).unwrap().get()
}

/// Die Rechnung prueft die Netto-Grundlage selbst: "Netto: nein" rechnet nur mit der Folgefrage "ja" und nur bis 800 EUR
/// (die Grenze gilt netto, den Nettobetrag kennt die Software nicht). 800,00 EUR rechnet, 800,01 EUR nicht.
/// Die erste Zeile ist die Kontrolle: die Rechnung liefert ueberhaupt etwas (600 EUR / 4 Jahre, Monat 1 = 150 EUR).
#[test]
fn die_rechnung_verlangt_die_netto_grundlage_selbst() {
    let t1 = Geraet { verzeichnis: false, nd: Some(4), monat: Some(1), ..GUT };
    assert_eq!(afa_direkt(t1), 150, "KONTROLLE: Netto ja, 600 EUR, 4 Jahre");
    for ohne_abzug in [None, Some(false)] {
        let g = Geraet { netto: false, ohne_abzug, ..t1 };
        assert_eq!(afa_direkt(g), 0, "Netto nein, Folgefrage {ohne_abzug:?}");
    }
    let klein = Geraet { netto: false, ohne_abzug: Some(true), ..t1 };
    assert_eq!(afa_direkt(klein), 150, "Netto nein, Folgefrage ja (Bruttobetrag)");
    assert_eq!(afa_direkt(Geraet { betrag: 80_000, ..klein }), 200, "genau 800 EUR brutto, 4 Jahre");
    assert_eq!(afa_direkt(Geraet { betrag: 80_001, ..klein }), 0, "800,01 EUR brutto");
    assert_eq!(afa_direkt(Geraet { betrag: 100_000, ..klein }), 0, "1.000 EUR brutto");
    assert_eq!(afa_direkt(Geraet { betrag: 80_001, ..t1 }), 200, "800,01 EUR netto: 200,0025 -> 200 EUR");
}

/// Eine vorlaeufige Instanz zaehlt in der strengen Summe nicht (Zwei-Signal-Regel, `Instanzquelle::zaehlt`), in der
/// rohen schon. Die Kontrolle ist die rohe Summe: sie zeigt, dass die Instanz rechnet (100 EUR) und der Messaufbau nicht blind ist.
#[test]
fn eine_vorlaeufige_instanz_zaehlt_nicht_in_der_afa_summe() {
    let g = Geraet { verzeichnis: false, nd: Some(3), monat: Some(7), ..GUT }; // 600 EUR, 3 Jahre, ab Juli: 100 EUR
    let paare = geraet_paare(basis(0, 5_000_000, 0), g, false);
    let summe = |f: &Fall, nur_bestaetigt: bool| {
        let q = Instanzquelle { store: Some(&f.store), bindung: Some(&f.index), nur_bestaetigt };
        gwg_afa_summe(&f.felder, &q).unwrap().get()
    };
    let vorlaeufig = fall(Scheibe::Gesamt, &paare, &[("gwg_verzeichnis_ab_250", json!(false))]);
    assert_eq!(summe(&vorlaeufig, false), 100, "KONTROLLE: roh zaehlt die Instanz");
    assert_eq!(summe(&vorlaeufig, true), 0, "streng zaehlt eine vorlaeufige Instanz nicht");
    let bestaetigt = fall(Scheibe::Gesamt, &paare, &[]);
    assert_eq!(summe(&bestaetigt, true), 100, "streng zaehlt die bestaetigte Instanz");
}

/// Ein `AfA`-Geraet allein in der zweiten Instanz, ohne Einnahmen und ohne Handfelder: dann schaltet nur die `AfA`-Summe den
/// Gewinn-Zweig ein (`GEWINN_QUELLEN_MENGEN` kennt von den Geraetefeldern nur das Basisfeld der ersten Instanz). Ohne sie
/// bliebe die `AfA` still weg. Soll: dieselbe Steuer wie ohne Geraet mit 120 EUR im Handfeld (600 EUR / 5 Jahre, Monat 1).
#[test]
fn ein_afa_geraet_allein_in_instanz_2_schaltet_den_gewinn_zweig_ein() {
    let g = Geraet { nutzbar: false, nd: Some(5), monat: Some(1), ..GUT };
    for vz in VZS {
        let soll = (Grund::Bestaetigt, referenz(vz, 6_000_000, 0, 120));
        assert!(soll.1 < referenz(vz, 6_000_000, 0, 0), "KONTROLLE: Messung blind {}", vz.jahr());
        let paare = geraet_paare(basis(6_000_000, 0, 0), g, true);
        assert_eq!(ergebnis(&fall(Scheibe::Gesamt, &paare, &[]), vz), soll, "{}", vz.jahr());
    }
}

/// Die Slots der Rechnung (`intervall::bescheid_via_slots`) tragen den Slot-Namen allein, ohne `regel_id`: zwei Felder einer
/// Scheibe mit demselben Namen ueberschreiben einander (das spaetere gewinnt). Die zwei Geraetefelder brauchen also
/// Slot-Namen, die in der Scheibe sonst niemand traegt, auch nicht ein Feld einer anderen Regel (`arbeitsmittel_nutzungsdauer`
/// bindet auf `nutzungsdauer`). Heute liest kein Rechenzweig diese Namen; der Test haelt, was der Kommentar in
/// `bindung_n_vor_gwg.yaml` verspricht.
#[test]
fn die_slot_namen_der_geraetefelder_gehoeren_in_der_scheibe_nur_ihnen() {
    let slot = |b: &Bindung| match &b.quelle.bindungspunkt {
        Bindungspunkt::SignaturSlot(s) => Some(s.clone()),
        Bindungspunkt::Geltungsbedingung(_) => None,
    };
    for scheibe in [Scheibe::Gesamt, Scheibe::RentnerGesamt] {
        let ids = Cfg::fuer(scheibe)
            .felder(|d| panic!("KONTROLLE: {scheibe} liest Felder aus {d}"))
            .unwrap();
        let teil: Vec<&Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
        for feld in ["gwg_nutzungsdauer", "gwg_kaufmonat"] {
            let eigener = slot(index()[feld]).expect("KONTROLLE: das Geraetefeld hat einen Slot");
            assert!(teil.iter().any(|b| b.feld_id == feld), "KONTROLLE: {feld} steht in {scheibe}");
            let traeger: Vec<&str> =
                teil.iter().filter(|b| slot(b).as_deref() == Some(eigener.as_str())).map(|b| b.feld_id.as_str()).collect();
            assert_eq!(traeger, [feld], "Slot {eigener} in {scheibe}");
        }
    }
}
