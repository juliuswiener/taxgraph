//! Luecken in `deklaration/ring_werte.rs` (`mit_ring_werten`), die ein `cargo-mutants`-Lauf am 2026-10-24 (Basis `0cb6eccd`,
//! `arch-think`) gefunden hat: Mutanten, die auch alle Tests von `bescheid` und `api` (ohne Sweep und Browser) bestanden.
//! Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** `mit_ring_werten` legt Hilfswerte ins Formular, die ELSTER als eigene Kennzahl verlangt (Summenzeilen, die
//! Kuerzung der Verpflegung, der genutzte Sparer-Pauschbetrag). Jeder Wert entsteht nur unter einer Bedingung: bestaetigt, ueber 0,
//! aus den Toepfen statt dem Aggregat. Die Grenzen dieser Bedingungen pruefte kein Test.
//!
//! **Warum es zaehlt.** Faellt eine Bedingung, steht im Formular eine Null-Zeile, die ELSTER nicht kennt, oder ein Wert, der aus
//! einem vorlaeufigen Betrag oder dem falschen Topf stammt (genutzter Sparer-Pauschbetrag zu niedrig, Summenzeile zu hoch).
//!
//! **Wo es sitzt.** `deklaration/ring_werte.rs`: `verpflegung`, `kap_antrag`, `pb_genutzt`, `instanz_summe`, `haushaltsnah`,
//! `vermietung`, `einzelzeile`, `p22_nr3`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: aus den Regeln, die die Funktionen selbst in ihren Kommentaren nennen ("Inert: keine Summe > 0, kein
//! Eintrag", "nur BESTAETIGTE Felder", "Toepfe XOR Aggregat", "Werbungskosten NUR bei echt positiver Differenz") und aus den
//! Parametern der Vorjahre (`params/2025`). Die Cent-Werte sind von Hand gerechnet, nicht aus dem Rust-Code abgelesen.
//!
//! Jede Zeile hat eine Gegenprobe (Wert ueber 0 oder bestaetigt), die den Eintrag liefert: ohne sie bestuende auch ein Ring, der nie
//! einen Wert schreibt, jeden "kein Eintrag"-Fall.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{felder, params, store};
use bescheid::Felder;
use domain::{PyWert, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Die Felder nach `mit_ring_werten` fuer VZ 2025 (oder ohne Jahr).
fn ring(events: &[Ev], vz: Option<Vz>) -> Felder {
    let st = store(events);
    let mut f = felder(&st);
    mit_ring_werten(&mut f, vz, params()).unwrap();
    f
}

fn ring_2025(events: &[Ev]) -> Felder {
    ring(events, Some(Vz::Vz2025))
}

/// Der Wert eines Ring-Eintrags als ganze Zahl; `None` = kein Eintrag.
fn ganz(f: &Felder, fid: &str) -> Option<i64> {
    f.get(fid).map(|x| match x.wert {
        PyWert::Ganz(n) => n,
        ref w => panic!("{fid}: keine ganze Zahl: {w:?}"),
    })
}

/// Verpflegungskuerzung (§ 9 Abs. 4a): ohne gestellte Mahlzeit ist die Kuerzung 0, und eine 0 wird nicht geschrieben ("Inert").
/// Mit einem gestellten Fruehstueck (20 % von 28 EUR = 5,60 EUR) steht sie da.
#[test]
fn verpflegung_ohne_mahlzeit_schreibt_keine_null_kuerzung() {
    let tage = z("tage_24h", 5, true);
    assert_eq!(ganz(&ring_2025(std::slice::from_ref(&tage)), "p9_4a_kuerzung_nach_entgelt"), None, "keine Mahlzeit: kein Eintrag");
    assert_eq!(
        ganz(&ring_2025(&[tage, z("vpf_fruehstuecke_gestellt_anzahl", 1, true)]), "p9_4a_kuerzung_nach_entgelt"),
        Some(560),
        "ein Fruehstueck: 20 % von 2.800 Cent (Gegenprobe)"
    );
}

/// Anlage KAP: bei einem Fehler der Python-Klasse (hier: kein Jahr, `FileNotFoundError`) bleibt KEIN Eintrag stehen, der Aufruf
/// scheitert nicht (Python `except Exception: pb_genutzt_cent = None`). Mit Jahr stehen Antrag und genutzter Pauschbetrag.
#[test]
fn kap_antrag_ohne_jahr_schreibt_nichts_und_scheitert_nicht() {
    let kap = [z("kap_kapitalertraege", 100_000, true)];
    let ohne_jahr = ring(&kap, None);
    assert!(!ohne_jahr.contains_key("kap_antrag_guenstigerpruefung") && !ohne_jahr.contains_key("kap_sparer_pauschbetrag_genutzt"));
    let mit_jahr = ring_2025(&kap);
    assert_eq!(mit_jahr.get("kap_antrag_guenstigerpruefung").map(|x| x.wert.clone()), Some(PyWert::Bool(true)));
    assert_eq!(ganz(&mit_jahr, "kap_sparer_pauschbetrag_genutzt"), Some(100_000), "1.000 EUR Ertraege, Pauschbetrag 1.000 EUR: alles genutzt");
}

/// Anlage KAP: ein bestaetigter Kapitalertrag ausserhalb von `i64` ist KEIN Python-Fehler, den der Ring schluckt (Python rechnet mit
/// beliebig grossen `int` weiter und haette eine Zahl), sondern ein `Ueberlauf`: der Aufruf scheitert, und es bleibt kein halber Eintrag.
/// Nur Fehler MIT Python-Klasse fallen still weg (Test davor). Ein `cargo-mutants`-Lauf (2026-10-24) zeigte: der Zweig `Err(e) => return
/// Err(e)` hatte keinen Test, die Bedingung davor liess sich auf `true` setzen.
#[test]
fn kap_antrag_mit_ueberlauf_scheitert_statt_still_zu_fehlen() {
    let st = store(&[("kap_kapitalertraege", json!(10_000_000_000_000_000_000_u64), true)]);
    let mut f = felder(&st);
    let fehler = mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap_err();
    assert!(fehler.ist_ueberlauf(), "kein Ueberlauf: {fehler:?}");
    assert!(!f.contains_key("kap_antrag_guenstigerpruefung"), "Eintrag trotz Fehler");
}

/// Toepfe XOR Aggregat je Person: ist EIN Topf belegt, rechnet der Ring aus den Toepfen und ignoriert das Aggregat; sonst aus dem
/// Aggregat. Person B bei Zusammenveranlagung ebenso. Pauschbetrag 2025: 1.000 EUR einzeln, 2.000 EUR zusammen.
#[test]
fn sparer_pauschbetrag_genutzt_rechnet_aus_den_toepfen_wenn_einer_belegt_ist() {
    let nur_topf = [z("kap_gewinn_aktien", 500_000, true)];
    assert_eq!(
        ganz(&ring_2025(&nur_topf), "kap_sparer_pauschbetrag_genutzt"),
        Some(100_000),
        "5.000 EUR Aktiengewinn nur im Topf, Aggregat 0: die 1.000 EUR Pauschbetrag sind genutzt"
    );
    let zusammen = [
        ("veranlagung", json!("zusammen"), true),
        z("kap_kapitalertraege", 100_000, true),
        z("kap_gewinn_aktien_partner", 700_000, true),
    ];
    assert_eq!(
        ganz(&ring_2025(&zusammen), "kap_sparer_pauschbetrag_genutzt"),
        Some(200_000),
        "A 1.000 EUR im Aggregat, B 7.000 EUR im Topf: zusammen 8.000 EUR, Pauschbetrag 2.000 EUR genutzt"
    );
}

/// § 35a: die Summe je Betragsfeld zaehlt nur die BESTAETIGTEN Instanzen dieses Feldes (Instanz 1 ohne Suffix, weitere mit `__n`) und
/// steht nur bei einer Summe ueber 0.
#[test]
fn haushaltsnahe_summen_zaehlen_nur_das_eigene_feld_und_nur_bestaetigtes() {
    let f = ring_2025(&[
        z("hh_minijob_betrag", 30_000, true),
        z("hh_minijob_betrag__2", 5_000, true),
        z("hh_minijob_betrag__3", 99_999, false),
        z("hh_dienstleistung_betrag", 20_000, true),
        z("hh_handwerker_betrag", 10_000, true),
    ]);
    assert_eq!(ganz(&f, "hh_minijob_aufwendungen"), Some(35_000), "Instanz 1 und 2, die vorlaeufige 3 nicht");
    assert_eq!(ganz(&f, "hh_dienstleistungen"), Some(20_000), "nur das eigene Feld");
    assert_eq!(ganz(&f, "hh_handwerker_arbeitskosten"), Some(10_000), "nur das eigene Feld");
    let leer = ring_2025(&[z("hh_minijob_betrag", 0, true), z("hh_handwerker_betrag", 50_000, false)]);
    assert_eq!(ganz(&leer, "hh_minijob_aufwendungen"), None, "Summe 0: kein Eintrag");
    assert_eq!(ganz(&leer, "hh_handwerker_arbeitskosten"), None, "nur ein vorlaeufiger Betrag: kein Eintrag");
}

/// Anlage V: die Summenzeilen entstehen nur bei bestaetigten Einnahmen ueber 0. Die Werbungskosten zaehlen nur bestaetigte ganze
/// Zahlen (vorlaeufig und Bool zaehlen 0).
#[test]
fn vermietung_summenzeilen_nur_bei_einnahmen_ueber_null_und_bestaetigten_kosten() {
    assert_eq!(ganz(&ring_2025(&[z("vv_einnahmen", 0, true), z("vv_schuldzinsen", 5_000, true)]), "vv_ueberschuss"), None, "Einnahmen 0: keine Zeilen");
    let f = ring_2025(&[
        z("vv_einnahmen", 1_000_000, true),
        z("vv_schuldzinsen", 200_000, true),
        z("vv_sonstige_wk", 50_000, false),
        ("vv_erhaltungsaufwand", json!(true), true),
        z("vv_nebenkosten_umgelegt", 100_000, true),
    ]);
    assert_eq!(ganz(&f, "vv_summe_werbungskosten"), Some(200_000), "nur die bestaetigten Zahlen: Schuldzinsen");
    assert_eq!(ganz(&f, "vv_einnahmen_summe_gesamt"), Some(1_100_000), "Mieten und Umlagen");
    assert_eq!(ganz(&f, "vv_ueberschuss"), Some(900_000), "Summe aller Einnahmen minus Werbungskosten");
}

/// Einzelzeile: derselbe Betrag wie die Summe, sobald diese bestaetigt und ueber 0 ist; 0 schreibt keine Zeile.
#[test]
fn einzelzeile_nur_bei_bestaetigter_summe_ueber_null() {
    assert_eq!(ganz(&ring_2025(&[z("berufsausbildung_aufwendungen", 0, true)]), "berufsausbildung_einzelbetrag"), None, "Summe 0");
    assert_eq!(ganz(&ring_2025(&[z("berufsausbildung_aufwendungen", 40_000, false)]), "berufsausbildung_einzelbetrag"), None, "vorlaeufig");
    assert_eq!(ganz(&ring_2025(&[z("berufsausbildung_aufwendungen", 40_000, true)]), "berufsausbildung_einzelbetrag"), Some(40_000), "Gegenprobe");
}

/// § 22 Nr. 3: der Einzelposten entsteht bei Einnahmen ueber 0; die Werbungskosten nur bei echt positiver Differenz (negativ =
/// widerspruechliche Eingabe, bleibt sichtbar; 0 = nichts abzuziehen).
#[test]
fn p22_nr3_werbungskosten_nur_bei_positiver_differenz() {
    let fall = |einnahmen: i64, einkuenfte: i64| {
        ring_2025(&[z("p22_nr3_einnahmen", einnahmen, true), z("p22_nr3_einkuenfte", einkuenfte, true)])
    };
    let ohne = fall(0, 0);
    assert_eq!(ganz(&ohne, "p22_nr3_einnahmen_einzelbetrag"), None, "Einnahmen 0: kein Einzelposten");
    let gleich = fall(100_000, 100_000);
    assert_eq!(ganz(&gleich, "p22_nr3_einnahmen_einzelbetrag"), Some(100_000));
    assert_eq!(ganz(&gleich, "p22_nr3_werbungskosten"), None, "Differenz 0: keine Werbungskosten");
    assert_eq!(ganz(&fall(100_000, 40_000), "p22_nr3_werbungskosten"), Some(60_000), "Differenz 60.000 (Gegenprobe)");
    assert_eq!(ganz(&fall(100_000, 120_000), "p22_nr3_werbungskosten"), None, "negative Differenz: bleibt sichtbar, kein Abzug");
}
