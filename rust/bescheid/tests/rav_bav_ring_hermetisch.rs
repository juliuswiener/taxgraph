//! Rente mit Leistungsmitteilung (Abweichung Nr. 55, Anlage R-AV/bAV): der Ring beider Rechenwege (`rentner_gesamt`, `gesamt`)
//! rechnet die Betraege der Nummern 1, 4 und 5. Im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** Wer eine Rente aus Riester-Vertrag, Pensionskasse, Pensionsfonds oder Direktversicherung bezieht, bekommt eine
//! Leistungsmitteilung. Die Software hatte dafuer keinen Eingabeplatz; die Rente landete in einer falschen Zeile oder fehlte.
//!
//! **Warum es zaehlt.** Ohne den Ring waere die Erklaerung mit den neuen Feldern abgabefaehig und die Steuer still zu niedrig.
//!
//! **Wo es sitzt.** `rav_bav.rs::rav_bav_einkuenfte` liest die Instanzen der Gruppe `rente_av_bav`; `zweige/rentner.rs` und
//! `zweige/gesamt.rs` tragen die Summe in `einkuenfte_sonstige` und den eingeschlossenen Teil in die Bemessung des
//! Altersentlastungsbetrags. Die Tests setzen die Felder ohne Scheiben-Gate und ohne Guard in den Store: sie messen den Ring.
//! Den Guard misst `api/tests/rav_bav_waechter_hermetisch.rs`.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus Gesetz und `params/`, nie aus dem Rust-Code gelesen. Gemessen wird der
//! Unterschied des Gesamtbetrags der Einkuenfte zum Fall ohne Mitteilung, damit der Rest der Akte nicht in die Erwartung eingeht.
//! - Nr. 1 und Nr. 4 (Zeilen 4 und 10): voll steuerpflichtig (BMF-Schreiben Riester-Foerderung Rn. 139 bis 142: 6.000 bleiben 6.000).
//! - Nr. 5 (Zeile 15): Ertragsanteil nach dem bei Rentenbeginn vollendeten Lebensjahr (§ 22 Nr. 1 Satz 3 Buchst. a Doppelbuchst. bb,
//!   `params/kohorten/rente_ertragsanteil_p22.yaml`): 64 = 19 %, 65 = 18 %, 67 = 17 %.
//! - Pauschbetrag: 102 Euro "insgesamt" fuer alle Renten einer Person (§ 9a Satz 1 Nr. 3), Boden 0.
//! - Altersentlastungsbetrag (§ 24a, `estg_p24a_2026-07-13.txt`): geboren 1955, Kalenderjahr 2020: 16,0 % / 760 Euro. In die
//!   Bemessung gehen Nr. 1 und Nr. 4 ein, Nr. 5 nicht (§ 24a Satz 2).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;

use bescheid::deklaration::{feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::Umgebung;
use bescheid::Felder;
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;
const BEIDE: [Scheibe; 2] = [Scheibe::RentnerGesamt, Scheibe::Gesamt];

type Paare = Vec<(String, Value)>;

fn p(feld: &str, wert: Value) -> (String, Value) {
    (feld.to_owned(), wert)
}

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

/// Der volle Pflicht-Kegel der Scheibe; was der Test setzt, gewinnt, der Rest folgt bestaetigt.
fn kegel_fuer(cfg: &Cfg, gesetzt: &[(String, Value)]) -> Paare {
    let kegel = cfg.kegel_roh().expect("KONTROLLE: Scheibe ohne Kegel");
    let mut raus: Paare = kegel
        .iter()
        .map(|f| {
            let w = gesetzt
                .iter()
                .rev()
                .find(|(g, _)| g == *f)
                .map_or_else(|| standardwert(index()[*f]), |(_, w)| w.clone());
            ((*f).to_owned(), w)
        })
        .collect();
    raus.extend(
        gesetzt
            .iter()
            .filter(|(f, _)| !kegel.contains(&f.as_str()))
            .cloned(),
    );
    raus
}

/// Ein Fall der Scheibe, mit der Scheiben-Bindung als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

/// Ein Feld aus `vorlaeufig` steht als vorlaeufiges Ereignis im Store, alle anderen bestaetigt.
fn fall_mit(scheibe: Scheibe, paare: &[(String, Value)], vorlaeufig: &[&str]) -> Fall {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let alle = kegel_fuer(&cfg, paare);
    let events: Vec<(&str, Value, bool)> = alle
        .iter()
        .map(|(f, w)| (f.as_str(), w.clone(), !vorlaeufig.contains(&f.as_str())))
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

/// Der Gesamtbetrag der Einkuenfte in Euro; ein Fall ohne Zahl ist ein Fehlschlag des Tests.
fn gdb(scheibe: Scheibe, paare: &[(String, Value)]) -> i64 {
    gdb_mit(scheibe, paare, &[])
}

/// Wie [`gdb`]; ein Feld aus `vorlaeufig` steht als vorlaeufiges Ereignis im Store.
fn gdb_mit(scheibe: Scheibe, paare: &[(String, Value)], vorlaeufig: &[&str]) -> i64 {
    let f = fall_mit(scheibe, paare, vorlaeufig);
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, VZ, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => z
            .extras
            .kette
            .expect("KONTROLLE: Kette fehlt")
            .gesamtbetrag_der_einkuenfte
            .get(),
        Ok(Err(k)) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {:?}", k.grund),
        Err(e) => panic!("KONTROLLE: erwartet eine Zahl, bekommen {e:?}"),
    }
}

/// Die Akte ohne Mitteilung: Einzelveranlagung, geboren am 05.05.1955, bei `rentner_gesamt` ohne Anlage-R-Rente (Jahresbetrag 0,
/// damit der Pauschbetrag allein bei den Mitteilungen liegt; ihn einmal ueber beide Anlagen misst `rente_pauschbetrag_einmal.rs`).
fn akte(scheibe: Scheibe) -> Paare {
    let mut a = vec![
        p("veranlagung", json!("einzel")),
        p("stammdaten_geburtsdatum", json!("05.05.1955")),
    ];
    if scheibe == Scheibe::RentnerGesamt {
        a.extend([
            p("rentner_renten_art", json!("gesetzliche_rente")),
            p("rentner_jahresrente", json!(0)),
            p("rentner_renten_beginn_jahr", json!(2025)),
            p("rentner_alter_bei_rentenbeginn", json!(65)),
            p("rentner_rentenfreibetrag", json!(0)),
            p("kein_gewinn", json!(true)),
            p("kein_kap", json!(true)),
            p("kein_vuv", json!(true)),
            p("kein_sonstige", json!(false)),
            p("agb_zwangslaeufig", json!(true)),
            p("agb_notwendig_angemessen", json!(true)),
        ]);
    }
    a
}

/// Der Name des Feldes in der Instanz `n` (Instanz 1 traegt den Basisnamen).
fn name(basis: &str, n: u8) -> String {
    if n == 1 {
        basis.to_owned()
    } else {
        format!("{basis}__{n}")
    }
}

/// Eine vollstaendige Mitteilung der Instanz `n`: die drei Betraege in Euro, Beginn der Nr. 4 am 01.04.2022, Beginn der Nr. 5 am
/// `beginn5`, keine weiteren Nummern, keine Basisrente.
fn mitteilung(n: u8, nr1: i64, nr4: i64, nr5: i64, beginn5: &str) -> Paare {
    vec![
        p(&name("rav_bav_nr1_betrag", n), json!(cent(nr1))),
        p(&name("rav_bav_nr4_betrag", n), json!(cent(nr4))),
        p(&name("rav_bav_nr4_beginn", n), json!("01.04.2022")),
        p(&name("rav_bav_nr5_betrag", n), json!(cent(nr5))),
        p(&name("rav_bav_nr5_beginn", n), json!(beginn5)),
        p(&name("rav_bav_andere_nummern", n), json!(false)),
        p(&name("rav_bav_basisrente", n), json!(false)),
    ]
}

/// Die Akte mit dem Kreuz "ich habe eine Rente mit Leistungsmitteilung" und den gegebenen Feldern.
fn mit_rente(scheibe: Scheibe, mitteilungen: Paare) -> Paare {
    let mut a = akte(scheibe);
    a.push(p("kein_rav_bav", json!(false)));
    a.extend(mitteilungen);
    a
}

/// Der Zuwachs des Gesamtbetrags durch die Mitteilungen gegenueber der Akte ohne sie. `mehr` gilt fuer beide Faelle.
fn zuwachs_mit(scheibe: Scheibe, mitteilungen: Paare, mehr: &[(String, Value)]) -> i64 {
    let mit = [mit_rente(scheibe, mitteilungen), mehr.to_vec()].concat();
    let ohne = [akte(scheibe), mehr.to_vec()].concat();
    gdb(scheibe, &mit) - gdb(scheibe, &ohne)
}

/// Wie [`zuwachs_mit`] fuer eine Akte ohne Geburtsjahr, also ohne Altersentlastungsbetrag.
fn zuwachs(scheibe: Scheibe, mitteilungen: Paare) -> i64 {
    zuwachs_mit(scheibe, mitteilungen, &[])
}

const BEGINN_65: &str = "05.05.2020";

/// Nr. 1 (Zeile 4) und Nr. 4 (Zeile 10) sind voll steuerpflichtig: 3.000 Euro bleiben 3.000, nach 102 Euro Pauschbetrag 2.898;
/// beide in einer Mitteilung (3.000 + 2.000) nach einem Pauschbetrag 4.898.
#[test]
fn nr1_und_nr4_gehen_voll_in_den_gesamtbetrag() {
    for s in BEIDE {
        assert_eq!(
            zuwachs(s, mitteilung(1, 3_000, 0, 0, BEGINN_65)),
            2_898,
            "{s}: Nr. 1"
        );
        assert_eq!(
            zuwachs(s, mitteilung(1, 0, 3_000, 0, BEGINN_65)),
            2_898,
            "{s}: Nr. 4"
        );
        assert_eq!(
            zuwachs(s, mitteilung(1, 3_000, 2_000, 0, BEGINN_65)),
            4_898,
            "{s}: Nr. 1 und Nr. 4"
        );
    }
}

/// Nr. 5 (Zeile 15) mit dem Ertragsanteil des bei Rentenbeginn vollendeten Lebensjahrs. Geboren am 05.05.1955: Beginn am
/// 05.05.2020 = 65 (18 %), am 04.05.2020 = 64 (19 %, der Geburtstag fehlt noch), am 05.05.2022 = 67 (17 %). Je 10.000 Euro Rente
/// abzueglich 102 Euro Pauschbetrag.
#[test]
fn nr5_geht_mit_dem_ertragsanteil_des_alters_bei_rentenbeginn_ein() {
    for s in BEIDE {
        for (beginn, erwartet) in [
            (BEGINN_65, 1_800 - 102),
            ("04.05.2020", 1_900 - 102),
            ("05.05.2022", 1_700 - 102),
        ] {
            assert_eq!(
                zuwachs(s, mitteilung(1, 0, 0, 10_000, beginn)),
                erwartet,
                "{s}: Beginn {beginn}"
            );
        }
    }
}

/// Mehrere Mitteilungen addieren sich, der Pauschbetrag gilt einmal: zweimal 3.000 Euro ergeben 6.000 − 102, nicht 2 × (3.000 − 102).
#[test]
fn mehrere_mitteilungen_teilen_sich_einen_pauschbetrag() {
    for s in BEIDE {
        let beide = [
            mitteilung(1, 3_000, 0, 0, BEGINN_65),
            mitteilung(2, 3_000, 0, 0, BEGINN_65),
        ]
        .concat();
        assert_eq!(zuwachs(s, beide), 5_898, "{s}");
    }
}

/// Mehrere Mitteilungen addieren auch den Ertragsanteil der Nr. 5 (Zeile 15), jede mit dem Ertragsanteil ihres eigenen
/// Rentenbeginns: Nr. 1 3.000 und Nr. 5 10.000 ab 05.05.2020 (65 Jahre, 18 % = 1.800), dazu Nr. 4 2.000 und Nr. 5 10.000 ab
/// 05.05.2022 (67 Jahre, 17 % = 1.700). 3.000 + 2.000 + 1.800 + 1.700 − 102 = 8.398. Zaehlte nur die letzte Mitteilung ihren
/// Ertragsanteil, kaemen 6.598 heraus (Mutant N16 der Nachmessung Nr. 55).
#[test]
fn mehrere_mitteilungen_addieren_auch_den_ertragsanteil_der_nr5() {
    for s in BEIDE {
        let beide = [
            mitteilung(1, 3_000, 0, 10_000, BEGINN_65),
            mitteilung(2, 0, 2_000, 10_000, "05.05.2022"),
        ]
        .concat();
        assert_eq!(zuwachs(s, beide), 8_398, "{s}");
    }
}

/// Ein Betrag mit Cent-Rest zaehlt in vollen Euro abgerundet, wie ihn der Schreiber in die Zeile setzt (`kz_rundung.tsv`, Zeilen
/// `ab`): 3.000,99 Euro in Nr. 1 oder Nr. 4 zaehlen 3.000, nicht 3.001 (Mutant N05 der Nachmessung Nr. 55).
#[test]
fn ein_betrag_mit_cent_rest_zaehlt_abgerundet() {
    for s in BEIDE {
        for feld in ["rav_bav_nr1_betrag", "rav_bav_nr4_betrag"] {
            let m = mit(
                mitteilung(1, 0, 0, 0, BEGINN_65),
                feld,
                json!(cent(3_000) + 99),
            );
            assert_eq!(zuwachs(s, m), 2_898, "{s}: {feld}");
        }
    }
}

/// Eine vorlaeufige Mitteilung bewegt die festgesetzte Steuer nie (`Instanzquelle::zaehlt`): eine einzige vorlaeufige Angabe macht
/// die ganze Mitteilung vorlaeufig, und neben einer bestaetigten Mitteilung zaehlt nur diese, mit einem Pauschbetrag (Mutant N14
/// der Nachmessung Nr. 55). Den Nutzer erreicht der Guard davor mit `rav_bav_angaben_offen`
/// (`api/tests/rav_bav_waechter_hermetisch.rs`, `o5`); hier steht der Ring allein.
#[test]
fn eine_vorlaeufige_mitteilung_zaehlt_im_ring_nicht() {
    for s in BEIDE {
        let ohne = gdb(s, &akte(s));
        let allein = gdb_mit(
            s,
            &mit_rente(s, mitteilung(1, 3_000, 0, 0, BEGINN_65)),
            &["rav_bav_nr1_betrag"],
        );
        assert_eq!(allein - ohne, 0, "{s}: einzige Mitteilung vorlaeufig");
        let beide = [
            mitteilung(1, 3_000, 0, 0, BEGINN_65),
            mitteilung(2, 3_000, 0, 0, BEGINN_65),
        ]
        .concat();
        let zweite = gdb_mit(s, &mit_rente(s, beide), &["rav_bav_nr1_betrag__2"]);
        assert_eq!(zweite - ohne, 2_898, "{s}: zweite Mitteilung vorlaeufig");
    }
}

/// Der Pauschbetrag hat den Boden 0: 50 Euro Rente bringen keine negativen Einkuenfte, und ein Betrag 0 aendert nichts.
#[test]
fn ein_betrag_unter_dem_pauschbetrag_bringt_nichts_ein() {
    for s in BEIDE {
        assert_eq!(
            zuwachs(s, mitteilung(1, 50, 0, 0, BEGINN_65)),
            0,
            "{s}: 50 Euro"
        );
        assert_eq!(
            zuwachs(s, mitteilung(1, 0, 0, 0, BEGINN_65)),
            0,
            "{s}: 0 Euro"
        );
    }
}

/// § 24a Satz 2: Nr. 1 und Nr. 4 gehen in die Bemessung des Altersentlastungsbetrags ein, Nr. 5 nicht.
/// 5.000 Euro Nr. 1: Einkuenfte 4.898, 16 % davon waeren 783,68, der Hoechstbetrag 760 greift: 4.898 − 760 = 4.138.
/// 10.000 Euro Nr. 5 (18 %): Einkuenfte 1.698, kein Altersentlastungsbetrag.
/// Beides in einer Mitteilung: Einkuenfte 5.000 + 1.800 − 102 = 6.698 (der Pauschbetrag mindert zuerst den Ertragsanteil, der nicht in
/// die Bemessung eingeht), Bemessung 5.000, Altersentlastungsbetrag 16 % = 800, gedeckelt 760: 6.698 − 760 = 5.938.
#[test]
fn altersentlastung_bemisst_sich_nach_nr1_und_nr4_nicht_nach_nr5() {
    let alt = [p("geburtsjahr", json!(1955))];
    for s in BEIDE {
        assert_eq!(
            zuwachs_mit(s, mitteilung(1, 5_000, 0, 0, BEGINN_65), &alt),
            4_138,
            "{s}: Nr. 1"
        );
        assert_eq!(
            zuwachs_mit(s, mitteilung(1, 0, 5_000, 0, BEGINN_65), &alt),
            4_138,
            "{s}: Nr. 4"
        );
        assert_eq!(
            zuwachs_mit(s, mitteilung(1, 0, 0, 10_000, BEGINN_65), &alt),
            1_698,
            "{s}: Nr. 5"
        );
        assert_eq!(
            zuwachs_mit(s, mitteilung(1, 3_000, 2_000, 10_000, BEGINN_65), &alt),
            5_938,
            "{s}: alle drei"
        );
    }
}

/// Das Kreuz "keine Rente mit Leistungsmitteilung" nimmt die Regel der Gruppe aus der Rechnung: Betraege, die noch im Speicher
/// liegen, zaehlen dann nicht (`regel_bedingungen`).
#[test]
fn mit_dem_kreuz_keine_rente_zaehlt_keine_mitteilung() {
    for s in BEIDE {
        let mut a = akte(s);
        a.push(p("kein_rav_bav", json!(true)));
        a.extend(mitteilung(1, 3_000, 0, 0, BEGINN_65));
        assert_eq!(gdb(s, &a), gdb(s, &akte(s)), "{s}");
    }
}

/// Setzt `feld` der Instanz 1 einer Mitteilung auf `wert`.
fn mit(mut m: Paare, feld: &str, wert: Value) -> Paare {
    m.iter_mut()
        .find(|(f, _)| f == feld)
        .expect("KONTROLLE: Feld fehlt in der Mitteilung")
        .1 = wert;
    m
}

/// Eine Mitteilung, die der Ring nicht lesen kann (weitere Nummern, Basisrente, fehlender Beginn, Beginn nach dem Steuerjahr),
/// traegt nichts bei. Sie wird nicht geraten; den Nutzer erreicht der Guard davor mit Klartext.
#[test]
fn eine_nicht_lesbare_mitteilung_traegt_nichts_bei() {
    for s in BEIDE {
        let m = || mitteilung(1, 3_000, 0, 10_000, BEGINN_65);
        let faelle = [
            (
                "weitere Nummern",
                mit(m(), "rav_bav_andere_nummern", json!(true)),
            ),
            ("Basisrente", mit(m(), "rav_bav_basisrente", json!(true))),
            (
                "weitere Nummern nicht beantwortet",
                mit(m(), "rav_bav_andere_nummern", json!("")),
            ),
            (
                "Basisrente nicht beantwortet",
                mit(m(), "rav_bav_basisrente", json!("")),
            ),
            (
                "Betrag der Nr. 1 ist keine Zahl",
                mit(m(), "rav_bav_nr1_betrag", json!("x")),
            ),
            (
                "Betrag der Nr. 1 ist negativ",
                mit(m(), "rav_bav_nr1_betrag", json!(cent(-3_000))),
            ),
            (
                "Beginn der Nr. 5 fehlt",
                mit(m(), "rav_bav_nr5_beginn", json!("")),
            ),
            (
                "Beginn der Nr. 5 nach dem Steuerjahr",
                mit(m(), "rav_bav_nr5_beginn", json!("01.01.2026")),
            ),
            (
                "Beginn der Nr. 5 vor der Geburt",
                mit(m(), "rav_bav_nr5_beginn", json!("01.01.1950")),
            ),
            (
                "Beginn der Nr. 4 fehlt",
                mit(
                    mitteilung(1, 0, 3_000, 0, BEGINN_65),
                    "rav_bav_nr4_beginn",
                    json!(""),
                ),
            ),
            (
                "Beginn der Nr. 4 nach dem Steuerjahr",
                mit(
                    mitteilung(1, 0, 3_000, 0, BEGINN_65),
                    "rav_bav_nr4_beginn",
                    json!("31.12.2026"),
                ),
            ),
        ];
        for (was, mitteilung) in faelle {
            assert_eq!(zuwachs(s, mitteilung), 0, "{s}: {was}");
        }
    }
}
