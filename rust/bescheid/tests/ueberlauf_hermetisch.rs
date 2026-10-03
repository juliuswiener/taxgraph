//! Ueberlaufpfade der Bescheid-Schicht: `kind_kv_pv_summe` (Addition), `shared_steuer_sonder_agb` (Subtraktion) und
//! `mit_ring_werten` (Verpflegungskuerzung, Gewerbesteuer) melden `BescheidFehler::Ueberlauf(<Stelle>)` statt eine falsche Zahl
//! zu liefern, im Standardlauf (ohne `PARITY=1`, ohne Python).
//!
//! Python rechnet mit beliebig grossen Ganzzahlen; Rust rechnet in `i64` mit `checked_*` und meldet `Ueberlauf` (fail-closed
//! statt Wrap-Around, Modul-Doku in `bescheid/src/lib.rs`; `python_klasse() == None`). Ein Wrap-Around lieferte eine falsche
//! Zahl in den Bescheid, keinen Fehler. Kein Bestandstest prueft einen dieser Pfade. Gemessen am 2026-10-03 auf 4a2f6ea4 (Bericht
//! h8-hermetisch4): sechs Mutationen am Aufrufort, je ein `checked_*` durch `wrapping_*` ersetzt: U10 Verpflegungskuerzung
//! (`mal`), U11 Gewerbesteuer (Messbetrag mal Hebesatz), U12 genutzter Sparer-Pauschbetrag mal 100, U13 Betrag des Verlusts bei
//! § 23 (`checked_abs`), U14 `plus`, U15 `minus`. Alle sechs lassen `cargo test -p bescheid` gruen (188 passed, 0 failed, 10
//! ignored). Mit diesen Tests werden vier rot (U10, U11, U14, U15), jede mit dem Test, der ihre Stelle prueft. Zwei bleiben auch
//! mit diesen Tests gruen, weil KEINE Eingabe sie erreicht (aequivalente Mutanten, zaehlen nicht als gefangen): U12 -- der genutzte
//! Sparer-Pauschbetrag ist hoechstens der Pauschbetrag (1.000 oder 2.000 EUR), mal 100 also hoechstens 200.000 Cent, auch bei
//! Kapitalertraegen von 2^62 Cent je Person (letzter Fall der Ring-Tabelle: Rust = Python = 200.000) -- und U13 -- ein Verlust
//! aus Preis, Anschaffungskosten und Werbungskosten (je Cent durch 100) liegt weit ueber `i64::MIN`. Die Fehlerart ist Teil der
//! Erwartung: der Test verlangt `Ueberlauf` MIT der Marke der Stelle, nicht irgendeinen Fehler und keine Panik.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: der Python-Wert jedes Falls ist die Ausgabe des Python-Orakels (`bescheid_abzuege._kind_kv_pv_summe`,
//! `_shared_steuer_sonder_agb`, `bescheid_deklaration._mit_ring_werten`, ueber `tools/parity/bescheid_oracle`, VZ 2025,
//! `nur_bestaetigt = false`, Wegwerf-Datenwurzel); Orakel-Skript und Lauf: Anlagen zum Bericht. Jeder Fall traegt eine von drei
//! Marken im Namen: "Python-Wert ausserhalb i64": es gibt keinen `i64`-Wert, Rust MUSS `Ueberlauf` melden; das Orakel stuetzt das.
//! "nur ein Zwischenprodukt ausserhalb i64": Python liefert einen gueltigen Wert (z. B. agB 92.233.720.368.544.911 EUR bei
//! `agb_aufwendungen = i64::MAX` Cent und `behinderungsbedingte_aufwendungen = -1`), Rust meldet `Ueberlauf("Subtraktion")`, weil
//! die Cent-Differenz nicht passt. Das ist die dokumentierte fail-closed-Konvention von Rust und KEINE Python-Stuetze: Rust ist hier
//! strenger als Python (Eingaben ab etwa 92 Billiarden EUR, kein realer Fall, nie eine falsche Zahl). "passt gerade noch": alles
//! passt, Rust MUSS den Python-Wert liefern; diese Gegenproben zeigen, dass nicht jede grosse Zahl fehlschlaegt.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::fmt::Debug;

use bescheid::abzuege::{kind_kv_pv_summe, shared_steuer_sonder_agb};
use bescheid::deklaration::mit_ring_werten;
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::{BescheidFehler, Instanzquelle};
use domain::{Cent, Euro, PyWert, Veranlagung, Vz};
use serde_json::{json, Value};

/// Ein Event `(feld_id, wert, bestaetigt)`.
type Ev = (&'static str, Value, bool);

/// Ein Ja/Nein-Feld.
fn b(fid: &'static str, wert: bool, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Zahlfeld (Cent oder Grad der Behinderung).
fn z(fid: &'static str, wert: i64, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein Textfeld.
fn t(fid: &'static str, wert: &'static str, bestaetigt: bool) -> Ev {
    (fid, json!(wert), bestaetigt)
}

/// Ein handgebauter Fall. `python` ist der Wert, den Python liefert (beliebig grosse Ganzzahl, nur fuer die Meldung);
/// `erwartet` ist `Ok(Wert)` (Rust MUSS Python treffen) oder `Err(marke)` (Rust MUSS `Ueberlauf(marke)` melden).
struct Fall<E> {
    name: &'static str,
    events: Vec<Ev>,
    python: &'static str,
    erwartet: Result<E, &'static str>,
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn kv_pv_faelle() -> Vec<Fall<i64>> {
    vec![
        Fall {
            name: "kind_kv 9223372036854775807, kind_pv 1 [Python-Wert ausserhalb i64]",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", i64::MAX, true),
                z("kind_pv", 1, true),
            ],
            python: "9223372036854775808",
            erwartet: Err("Addition"),
        },
        Fall {
            name: "kind_kv 9223372036854775806, kind_pv 1 [passt gerade noch]",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", 9_223_372_036_854_775_806, true),
                z("kind_pv", 1, true),
            ],
            python: "9223372036854775807",
            erwartet: Ok(i64::MAX),
        },
        Fall {
            name: "kind_kv 9223372036854775807, kind_pv 0 [passt gerade noch]",
            events: vec![
                t("kind_idnr", "12345678901", true),
                z("kind_kv", i64::MAX, true),
                z("kind_pv", 0, true),
            ],
            python: "9223372036854775807",
            erwartet: Ok(i64::MAX),
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn agb_faelle() -> Vec<Fall<[i64; 3]>> {
    vec![
        Fall {
            name: "agb_aufwendungen 9223372036854775807, behinderungsbedingte_aufwendungen -1, Kind mit Pauschbetrag [nur ein Zwischenprodukt ausserhalb i64]",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), z("agb_aufwendungen", i64::MAX, true), z("behinderungsbedingte_aufwendungen", -1, true)],
            python: "[0, 0, 92233720368544911]",
            erwartet: Err("Subtraktion"),
        },
        Fall {
            name: "agb_aufwendungen 9223372036854775806, behinderungsbedingte_aufwendungen -1, Kind mit Pauschbetrag [passt gerade noch]",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), z("agb_aufwendungen", 9_223_372_036_854_775_806, true), z("behinderungsbedingte_aufwendungen", -1, true)],
            python: "[0, 0, 92233720368544911]",
            erwartet: Ok([0, 0, 92_233_720_368_544_911]),
        },
        Fall {
            name: "agb_aufwendungen -9223372036854775808, behinderungsbedingte_aufwendungen 1, Kind mit Pauschbetrag [nur ein Zwischenprodukt ausserhalb i64]",
            events: vec![t("kind_idnr", "12345678901", true), b("kind_behinderten_pb_antrag", true, true), b("kind_pb_nicht_selbst_genutzt", true, true), z("kind_grad_der_behinderung", 50, true), z("agb_aufwendungen", i64::MIN, true), z("behinderungsbedingte_aufwendungen", 1, true)],
            python: "[0, 0, 0]",
            erwartet: Err("Subtraktion"),
        },
    ]
}

// Eine Tabelle von Faellen, keine Logik: die Laenge ist die Zahl der Faelle.
#[allow(clippy::too_many_lines)]
fn ring_faelle() -> Vec<Fall<Vec<(String, PyWert)>>> {
    vec![
        Fall {
            name: "24-h-Tage und Fruehstuecke = i64::MAX [Python-Wert ausserhalb i64]",
            events: vec![z("tage_24h", i64::MAX, true), z("vpf_fruehstuecke_gestellt_anzahl", i64::MAX, true)],
            python: "{p9_4a_kuerzung_nach_entgelt: 5165088340638674451920}",
            erwartet: Err("Verpflegung"),
        },
        Fall {
            name: "24-h-Tage = i64::MAX allein [nur ein Zwischenprodukt ausserhalb i64]",
            events: vec![z("tage_24h", i64::MAX, true)],
            python: "{}",
            erwartet: Err("Verpflegung"),
        },
        Fall {
            name: "10 24-h-Tage, 3 Fruehstuecke [passt gerade noch]",
            events: vec![z("tage_24h", 10, true), z("vpf_fruehstuecke_gestellt_anzahl", 3, true)],
            python: "{p9_4a_kuerzung_nach_entgelt: 1680}",
            erwartet: Ok(vec![("p9_4a_kuerzung_nach_entgelt".to_owned(), PyWert::Ganz(1680))]),
        },
        Fall {
            name: "Messbetrag i64::MAX, Hebesatz 400 [Python-Wert ausserhalb i64]",
            events: vec![z("gewst_messbetrag", i64::MAX, true), z("gewst_hebesatz", 400, true)],
            python: "{gewst_zu_zahlen: 36893488147419103200}",
            erwartet: Err("gewst_zu_zahlen"),
        },
        Fall {
            name: "Messbetrag 2.305.843.009.213.693.900 ct, Hebesatz 400: passt gerade [passt gerade noch]",
            events: vec![z("gewst_messbetrag", 2_305_843_009_213_693_900, true), z("gewst_hebesatz", 400, true)],
            python: "{gewst_zu_zahlen: 9223372036854775600}",
            erwartet: Ok(vec![("gewst_zu_zahlen".to_owned(), PyWert::Ganz(9_223_372_036_854_775_600))]),
        },
        Fall {
            name: "Partner: Messbetrag i64::MAX, Hebesatz 400 [Python-Wert ausserhalb i64]",
            events: vec![z("gewst_messbetrag_partner", i64::MAX, true), z("gewst_hebesatz_partner", 400, true)],
            python: "{gewst_zu_zahlen_partner: 36893488147419103200}",
            erwartet: Err("gewst_zu_zahlen"),
        },
        Fall {
            name: "zusammen, Kapitalertraege je 2^62 ct: der genutzte Sparer-Pauschbetrag bleibt 2.000 EUR [passt gerade noch]",
            events: vec![t("veranlagung", "zusammen", true), z("kap_kapitalertraege", 4_611_686_018_427_387_904, true), z("kap_kapitalertraege_partner", 4_611_686_018_427_387_904, true)],
            python: "{kap_antrag_guenstigerpruefung: True, kap_sparer_pauschbetrag_genutzt: 200000}",
            erwartet: Ok(vec![("kap_antrag_guenstigerpruefung".to_owned(), PyWert::Bool(true)), ("kap_sparer_pauschbetrag_genutzt".to_owned(), PyWert::Ganz(200_000))]),
        },
    ]
}

/// Gleich, wenn beide Wert liefern und die Werte gleich sind, oder beide `Ueberlauf` mit derselben Marke melden -- weder einen
/// Wert (Wrap-Around) noch eine andere Fehlerart.
fn gleich<E: PartialEq>(
    got: &Result<E, BescheidFehler>,
    erwartet: &Result<E, &'static str>,
) -> bool {
    match (got, erwartet) {
        (Ok(a), Ok(b)) => a == b,
        (Err(BescheidFehler::Ueberlauf(m)), Err(soll)) => m == soll,
        _ => false,
    }
}

/// Alle Faelle durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn pruefe<E: PartialEq + Debug>(
    faelle: &[Fall<E>],
    rechne: impl Fn(&Fall<E>) -> Result<E, BescheidFehler>,
) {
    assert!(!faelle.is_empty(), "Tabelle ist leer");
    let abweichend: Vec<String> = faelle
        .iter()
        .filter_map(|f| {
            let got = rechne(f);
            (!gleich(&got, &f.erwartet)).then(|| {
                format!(
                    "{}: Rust {got:?}, Python {}, erwartet {:?}",
                    f.name, f.python, f.erwartet
                )
            })
        })
        .collect();
    assert!(
        abweichend.is_empty(),
        "{} von {} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len(),
        faelle.len()
    );
}

/// § 10 Abs. 1 Nr. 3 S. 2: `kind_kv + kind_pv` in Cent; die Summe, die nicht mehr in `i64` passt, meldet `Ueberlauf("Addition")`.
#[test]
fn kind_kv_pv_addition_meldet_ueberlauf() {
    pruefe(&kv_pv_faelle(), |fall| {
        let st = store(&fall.events);
        let q = Instanzquelle {
            store: Some(&st),
            bindung: Some(index()),
            nur_bestaetigt: false,
        };
        kind_kv_pv_summe(&q).map(Cent::get)
    });
}

/// § 33b Abs. 5 S. 4: die behinderungsbedingten Aufwendungen kuerzen die Einzelnachweise der agB (Cent); die Differenz, die
/// nicht mehr in `i64` passt, meldet `Ueberlauf("Subtraktion")`, auch wenn der Euro-Betrag danach noch passen wuerde.
#[test]
fn agb_subtraktion_meldet_ueberlauf() {
    pruefe(&agb_faelle(), |fall| {
        let st = store(&fall.events);
        let q = Instanzquelle {
            store: Some(&st),
            bindung: Some(index()),
            nur_bestaetigt: false,
        };
        shared_steuer_sonder_agb(
            Euro::new(50_000),
            Euro::new(0),
            Veranlagung::Einzel,
            &felder(&st),
            Vz::Vz2025,
            &q,
            params(),
        )
        .map(|s| {
            [
                s.steuerermaessigungen.get(),
                s.sonderausgaben.get(),
                s.aussergewoehnliche_belastungen.get(),
            ]
        })
    });
}

/// `mit_ring_werten` (Verpflegungskuerzung, Gewerbesteuer, Sparer-Pauschbetrag): ein Ring-Wert, der nicht mehr in `i64` passt,
/// meldet `Ueberlauf` mit der Marke der Stelle; ein Wert, der gerade noch passt, wird gesetzt.
#[test]
fn ring_werte_melden_ueberlauf_und_rechnen_knapp_richtig() {
    pruefe(&ring_faelle(), |fall| {
        let vorher = felder(&store(&fall.events));
        let mut nachher = vorher.clone();
        mit_ring_werten(&mut nachher, Some(Vz::Vz2025), params())?;
        let mut diff: Vec<(String, PyWert)> = nachher
            .iter()
            .filter(|(k, v)| vorher.get(*k).map(|x| &x.wert) != Some(&v.wert))
            .map(|(k, v)| (k.clone(), v.wert.clone()))
            .collect();
        diff.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(diff)
    });
}
