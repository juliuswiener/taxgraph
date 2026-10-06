//! § 32 Abs. 6 Satz 2 und Satz 5 `EStG` je Kind (Julius 2026-10-04, „c: A": je Kind rechnen, nur in Rust),
//! im Standardlauf (ohne `PARITY=1`, ohne Python). Vorher rechnete der Rahmen `Kinderzahl * (zusammen ? 2 : 1) *
//! Betrag` und las weder das Kindschaftsverhaeltnis noch den Zeitraum (Vault-Eintraege
//! `p32-abs6-satz-2-und-5-lesen-keine-regel`, `p32-6-kinderfreibetraege-verwaiste-regel`).
//!
//! Der Gesetzestext steht in `sources/gesetze-im-internet/estg_p32_2026-07-11.txt`, § 31 in `estg_p31_...txt`.
//! Je Kind, je Kalendermonat, je Ehegatte zaehlt EIN Zwoelftel des Betrags aus Satz 1:
//!
//! - Satz 5: „Fuer jeden Kalendermonat, in dem die Voraussetzungen ... nicht vorliegen, ermaessigen sich die dort
//!   genannten Betraege um ein Zwoelftel." Ein Monat zaehlt, wenn die Voraussetzung an mindestens einem Tag besteht.
//! - Satz 2: Bei Zusammenveranlagung verdoppelt sich der Betrag, „wenn das Kind zu beiden Ehegatten in einem
//!   Kindschaftsverhaeltnis steht". Steht es nur zu einem, haengt der Betrag an Satz 3 (anderer Elternteil
//!   verstorben, Alleinadoption) oder an einer Uebertragung (Satz 10): Angaben, nach denen die Software nicht fragt.
//!   Dann sperrt die Berechnung (`kind_freibetrag_verteilung_offen`), statt zu raten.
//! - Ein nicht lesbarer Zeitraum sperrt (`kind_zeitraum_unlesbar`), er rechnet nie das ganze Jahr.
//!
//! Das Kindergeld folgt denselben Monaten (§ 31 Satz 4: „den Anspruch auf Kindergeld fuer den gesamten
//! Veranlagungszeitraum"). Sonst stuende ein halbes Jahr Freibetrag neben zwoelf Monaten Kindergeld.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, `params/2025/kinderfreibetrag_p32.yaml` (3.336 + 1.464 = 4.800 Euro je
//! Elternteil und Kind) und `params/2025/kindergeld_p66.yaml` (255 Euro je Monat). Jede Erwartung ist von Hand aus
//! diesen Zahlen und den Monaten abgeleitet, kein Wert ist aus dem Rust-Code abgelesen. Python rechnet je Kind nicht
//! und ist hier nicht der Massstab. Die Zahl ohne Kindangaben (`basis`) ist der Normalfall von heute.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::collections::HashMap;

use bescheid::deklaration::{an_gesamt_sperrgrund, feste_zahl, Cfg};
use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::ausgaben::{Kette, P31Sieger};
use bescheid::zweige::Umgebung;
use bescheid::{Felder, Instanzquelle};
use bindung::Bindung;
use domain::{Feldtyp, Scheibe, Vz};
use intervall::AchsenBindung;
use serde_json::{json, Value};
use store::Store;

const VZ: Vz = Vz::Vz2025;
const GRUND_VERTEILUNG: &str = "kind_freibetrag_verteilung_offen";
const GRUND_ZEITRAUM: &str = "kind_zeitraum_unlesbar";

type Paare = Vec<(&'static str, Value)>;
type KindEvents = Vec<(&'static str, Value, bool)>;

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

/// Ein Fall der Scheibe, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(scheibe: Scheibe, basis: &[(&'static str, Value)], kind: &KindEvents) -> Fall {
    let cfg = Cfg::fuer(scheibe);
    let ids = cfg
        .felder(|d| panic!("KONTROLLE: Scheibe liest Felder aus {d}"))
        .unwrap();
    let teil: Vec<&'static Bindung> = ids.iter().map(|f| index()[f.as_str()]).collect();
    let mut events: Vec<(&str, Value, bool)> = kegel_fuer(&cfg, basis)
        .into_iter()
        .map(|(f, w)| (f, w, true))
        .collect();
    events.extend(kind.iter().cloned());
    let store = store(&events);
    Fall {
        cfg,
        index: teil.iter().map(|b| (b.feld_id.clone(), *b)).collect(),
        achsen: teil.iter().map(|b| AchsenBindung::from(*b)).collect(),
        felder: felder(&store),
        store,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Ausgang {
    Gesperrt(String),
    Zahl(i64, Box<Option<Kette>>),
    Anders(String),
}

/// Der Ausgang von `_ergebnis_roh` ohne HTTP: erst der K2-Guard, dann `feste_zahl` (nur bestaetigte Werte).
fn ergebnis(f: &Fall) -> Ausgang {
    if f.cfg.guard() {
        let q = Instanzquelle {
            store: Some(&f.store),
            bindung: Some(&f.index),
            nur_bestaetigt: false,
        };
        match an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q) {
            Ok(Some(g)) => return Ausgang::Gesperrt(g.als_str().to_owned()),
            Ok(None) => {}
            Err(e) => return Ausgang::Anders(format!("Guard: {e:?}")),
        }
    }
    ohne_guard(f)
}

/// `feste_zahl` allein, ohne den K2-Guard davor: so sieht der Rahmen einen Fall, den der Guard durchliess.
fn ohne_guard(f: &Fall) -> Ausgang {
    ohne_guard_im(f, VZ)
}

/// Wie [`ohne_guard`] im Veranlagungszeitraum `vz`: das Steuerjahr, das `tarif::rahmen` an `auswerten` reicht, kommt
/// aus diesem Parameter von `feste_zahl`.
fn ohne_guard_im(f: &Fall, vz: Vz) -> Ausgang {
    let kegel = f.cfg.kegel(|d| panic!("KONTROLLE: Kegel aus {d}")).unwrap();
    let kegel: Vec<&str> = kegel.iter().map(String::as_str).collect();
    let umg = Umgebung {
        achsen: &f.achsen,
        index: &f.index,
        params: params(),
    };
    match feste_zahl(&f.felder, &f.cfg, vz, &kegel, &umg, Some(&f.store), None) {
        Ok(Ok(z)) => Ausgang::Zahl(z.zahl.get(), Box::new(z.extras.kette)),
        Ok(Err(k)) => Ausgang::Anders(format!("ohne Zahl: {:?}", k.grund)),
        Err(e) => Ausgang::Anders(format!("{e:?}")),
    }
}

/// Die Kette eines Falls mit Zahl; jeder andere Ausgang ist ein Fehlschlag des Tests.
fn kette(a: Ausgang) -> Kette {
    match a {
        Ausgang::Zahl(_, k) => (*k).expect("KONTROLLE: Kette fehlt"),
        andere => panic!("KONTROLLE: erwartet eine Zahl, bekommen {andere:?}"),
    }
}

fn kindergeld(k: &Kette) -> i64 {
    k.p31
        .as_ref()
        .expect("KONTROLLE: kein p31 bei Kindern")
        .kindergeld
        .get()
}

/// Zusammenveranlagung, Einverdiener mit 300.000 Euro: die Freibetraege schlagen das Kindergeld auch bei einem
/// halben Jahr (4.800 Euro Freibetrag sparen bei diesem Satz rund 2.160 Euro, das halbe Kindergeld sind 1.530 Euro).
fn basis_zusammen(kinder: i64) -> Paare {
    vec![
        ("veranlagung", json!("zusammen")),
        ("bruttoarbeitslohn", json!(30_000_000)),
        ("bruttoarbeitslohn_partner", json!(0)),
        ("kap_kapitalertraege_partner", json!(0)),
        ("kap_gewinn_aktien_partner", json!(0)),
        ("kap_gewinn_sonstige_partner", json!(0)),
        ("kap_verlust_aktien_partner", json!(0)),
        ("kap_verlust_sonstige_partner", json!(0)),
        ("fam_anzahl_kinder", json!(kinder)),
    ]
}

fn basis_einzel(kinder: i64) -> Paare {
    vec![
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(30_000_000)),
        ("fam_anzahl_kinder", json!(kinder)),
    ]
}

/// Die Feld-ID der Instanz `n` (1 = Basis-Feld-ID, `n` >= 2 = `__n`).
fn instanz_id(n: u8, basis: &'static str) -> &'static str {
    if n == 1 {
        basis
    } else {
        // `Box::leak`: die Instanz-IDs der Test-Tabelle sind `&'static str`, wie die Basis-IDs.
        Box::leak(format!("{basis}__{n}").into_boxed_str())
    }
}

/// Die vier Kind-Felder der Instanz `n` (1 = Basis-Feld-ID, `n` >= 2 = `__n`), alle bestaetigt.
fn kind_n(n: u8, a: (&str, &str), b: (&str, &str)) -> KindEvents {
    let id = |basis: &'static str| instanz_id(n, basis);
    vec![
        (id("kind_kindschaftsverhaeltnis_a"), json!(a.0), true),
        (id("kind_kindschaftsverh_zeitraum_a"), json!(a.1), true),
        (id("kind_kindschaftsverhaeltnis_b"), json!(b.0), true),
        (id("kind_kindschaftsverh_zeitraum_b"), json!(b.1), true),
    ]
}

const GANZ: &str = "01.01-31.12";
const ZWEITE_HAELFTE: &str = "01.07-31.12";

fn rechne(scheibe: Scheibe, basis: &[(&'static str, Value)], kind: &KindEvents) -> Ausgang {
    ergebnis(&fall(scheibe, basis, kind))
}

fn zusammen(kinder: i64, kind: &KindEvents) -> Ausgang {
    rechne(Scheibe::Gesamt, &basis_zusammen(kinder), kind)
}

fn einzel(kinder: i64, kind: &KindEvents) -> Ausgang {
    rechne(Scheibe::Gesamt, &basis_einzel(kinder), kind)
}

// ---------------------------------------------------------------- Normalfall (AK4)

/// Der Normalfall von heute: ein Kind ohne Angaben. Die Freibetraege gewinnen, das Kindergeld sind 12 x 255.
#[test]
fn basis_zusammen_freibetraege_gewinnen_mit_zwoelf_monaten_kindergeld() {
    let k = kette(zusammen(1, &Vec::new()));
    assert_eq!(k.p31.as_ref().unwrap().guenstiger, P31Sieger::Freibetraege);
    assert_eq!(kindergeld(&k), 3_060);
}

/// AK4: Angaben, die den Normalfall nur ausdruecklich sagen (leiblich zu beiden, ganzes Jahr), aendern keine Zahl.
#[test]
fn ausdruecklicher_normalfall_aendert_nichts() {
    let ohne = zusammen(1, &Vec::new());
    let mit = zusammen(1, &kind_n(1, ("1", GANZ), ("1", GANZ)));
    assert_eq!(ohne, mit);
    assert!(matches!(ohne, Ausgang::Zahl(..)), "{ohne:?}");
}

/// Pflegekind zu beiden Ehegatten ist ein Kindschaftsverhaeltnis im Sinn von Satz 2 (§ 32 Abs. 1 Nr. 2).
#[test]
fn pflegekind_zu_beiden_rechnet_wie_der_normalfall() {
    let ohne = zusammen(1, &Vec::new());
    let mit = zusammen(1, &kind_n(1, ("2", GANZ), ("2", GANZ)));
    assert_eq!(ohne, mit);
}

// ---------------------------------------------------------------- Satz 5: Monate

/// Satz 5, beide Ehegatten ab Juli: sechs Monate. Der Freibetrag halbiert sich (9.600 -> 4.800 Euro), das
/// Kindergeld auch (3.060 -> 1.530 Euro).
#[test]
fn teiljahr_beide_halbiert_freibetrag_und_kindergeld() {
    let voll = kette(zusammen(1, &Vec::new()));
    let halb = kette(zusammen(
        1,
        &kind_n(1, ("1", ZWEITE_HAELFTE), ("1", ZWEITE_HAELFTE)),
    ));
    assert_eq!(
        halb.zu_versteuerndes_einkommen.get() - voll.zu_versteuerndes_einkommen.get(),
        4_800,
        "zvE mit halbem Freibetrag liegt um 4.800 Euro hoeher"
    );
    assert_eq!(kindergeld(&voll), 3_060);
    assert_eq!(kindergeld(&halb), 1_530);
}

/// Ein Monat zaehlt ab dem ersten Tag der Voraussetzung: 15.03 bis 31.12 sind zehn Monate (nicht neun).
/// Freibetrag 9.600 x 10/12 = 8.000, Kindergeld 10 x 255 = 2.550.
#[test]
fn angebrochener_monat_zaehlt_voll() {
    let voll = kette(zusammen(1, &Vec::new()));
    let teil = kette(zusammen(
        1,
        &kind_n(1, ("1", "15.03-31.12"), ("1", "15.03-31.12")),
    ));
    assert_eq!(
        teil.zu_versteuerndes_einkommen.get() - voll.zu_versteuerndes_einkommen.get(),
        1_600
    );
    assert_eq!(kindergeld(&teil), 2_550);
}

/// Der Zeitraum endet im Jahr: 01.01 bis 31.05 sind fuenf Monate. Freibetrag 9.600 x 5/12 = 4.000.
#[test]
fn zeitraum_bis_mai_sind_fuenf_monate() {
    let voll = kette(zusammen(1, &Vec::new()));
    let teil = kette(zusammen(
        1,
        &kind_n(1, ("1", "01.01-31.05"), ("1", "01.01-31.05")),
    ));
    assert_eq!(
        teil.zu_versteuerndes_einkommen.get() - voll.zu_versteuerndes_einkommen.get(),
        5_600
    );
    assert_eq!(kindergeld(&teil), 1_275);
}

/// Einzelveranlagung: Satz 5 gilt auch ohne Verdopplung. Neun Monate = 9 x 255 Kindergeld. (Bei Einzelveranlagung
/// gewinnt das Kindergeld bei einem Kind immer, der Freibetrag selbst zeigt sich dort nicht in der Kette.)
#[test]
fn einzel_teiljahr_kuerzt_das_kindergeld() {
    let k = kette(einzel(1, &kind_n(1, ("1", "01.04-31.12"), ("1", GANZ))));
    assert_eq!(kindergeld(&k), 2_295);
}

/// Einzelveranlagung: der Zeitraum von Person B ist ohne Bedeutung (Bindung: bei Einzelveranlagung unzulaessig).
#[test]
fn einzel_liest_den_zeitraum_von_person_b_nicht() {
    let ohne = einzel(1, &kind_n(1, ("1", GANZ), ("1", GANZ)));
    let mit = einzel(1, &kind_n(1, ("1", GANZ), ("3", "not a date")));
    assert_eq!(ohne, mit);
}

/// Zwei Kinder, das zweite (Instanz `__2`) nur ein halbes Jahr: Freibetrag 9.600 + 4.800, Kindergeld 3.060 + 1.530.
#[test]
fn zweites_kind_traegt_seinen_eigenen_zeitraum() {
    let beide_voll = kette(zusammen(2, &Vec::new()));
    let mut kind = kind_n(1, ("1", GANZ), ("1", GANZ));
    kind.extend(kind_n(2, ("1", ZWEITE_HAELFTE), ("1", ZWEITE_HAELFTE)));
    let k = kette(zusammen(2, &kind));
    assert_eq!(
        k.zu_versteuerndes_einkommen.get() - beide_voll.zu_versteuerndes_einkommen.get(),
        4_800
    );
    assert_eq!(kindergeld(&beide_voll), 6_120);
    assert_eq!(kindergeld(&k), 4_590);
}

/// Instanz 2 gibt es, aber `fam_anzahl_kinder` nennt nur ein Kind: die zweite Instanz zaehlt nicht und sperrt nicht.
#[test]
fn instanz_ueber_die_kinderzahl_hinaus_zaehlt_nicht() {
    let ohne = zusammen(1, &Vec::new());
    let mut kind = kind_n(1, ("1", GANZ), ("1", GANZ));
    kind.extend(kind_n(2, ("3", "kaputt"), ("3", "kaputt")));
    assert_eq!(ohne, zusammen(1, &kind));
}

// ---------------------------------------------------------------- Sperren statt falsch rechnen

/// Satz 2: Stiefkind (Verhaeltnis 3) bei Person B. Der Betrag haengt an Satz 3 oder einer Uebertragung (Satz 10).
#[test]
fn zusammen_stiefkind_bei_person_b_sperrt() {
    let a = zusammen(1, &kind_n(1, ("1", GANZ), ("3", GANZ)));
    assert_eq!(a, Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned()));
}

/// Satz 2: die Zeitraeume der beiden Ehegatten sind verschieden. Ein Monat mit nur einem Elternteil ist genau
/// der Fall von Satz 3, den die Software nicht entscheiden kann.
#[test]
fn zusammen_verschiedene_zeitraeume_sperren() {
    let a = zusammen(1, &kind_n(1, ("1", GANZ), ("1", ZWEITE_HAELFTE)));
    assert_eq!(a, Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned()));
}

/// Einzelveranlagung: ein Stiefkind hat Freibetraege nur ueber eine Uebertragung (Satz 10), nicht ueber Satz 1.
#[test]
fn einzel_stiefkind_sperrt() {
    let a = einzel(1, &kind_n(1, ("3", GANZ), ("1", GANZ)));
    assert_eq!(a, Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned()));
}

/// Ein Wert, der keine der drei Antworten ist, wird nie als Normalfall gelesen.
#[test]
fn unbekanntes_verhaeltnis_sperrt() {
    let a = einzel(1, &kind_n(1, ("4", GANZ), ("1", GANZ)));
    assert_eq!(a, Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned()));
}

/// Nicht lesbare Zeitraeume sperren, jeder aus einem eigenen Grund. Die Gegenprobe steht unten.
#[test]
fn unlesbarer_zeitraum_sperrt() {
    let faelle = [
        ("31.02-31.12", "31. Februar gibt es nicht"),
        ("30.02-31.12", "30. Februar gibt es nicht"),
        ("31.04-31.12", "31. April gibt es nicht"),
        ("29.02-31.12", "2025 ist kein Schaltjahr"),
        ("01.12-01.01", "Ende vor Anfang (kein Jahreswechsel)"),
        ("01.07-01.06", "Ende vor Anfang"),
        ("1.1-31.12", "Tag und Monat zweistellig"),
        ("01.01 - 31.12", "Leerzeichen"),
        ("01.01-31.12\n", "angehaengter Zeilenumbruch"),
        ("01.13-31.12", "Monat 13"),
        ("00.01-31.12", "Tag 0"),
        ("abc", "kein Zeitraum"),
        ("01.01.2025-31.12.2025", "Datum statt Tag und Monat"),
    ];
    let mut falsch = Vec::new();
    for (text, warum) in faelle {
        let a = einzel(1, &kind_n(1, ("1", text), ("1", GANZ)));
        if a != Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()) {
            falsch.push(format!("{text:?} ({warum}): {a:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Bei Zusammenveranlagung liest der Rahmen auch den Zeitraum von Person B; ein kaputter sperrt.
#[test]
fn unlesbarer_zeitraum_von_person_b_sperrt_bei_zusammen() {
    let a = zusammen(1, &kind_n(1, ("1", GANZ), ("1", "31.02-31.12")));
    assert_eq!(a, Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()));
}

/// Gegenprobe: die Grenzen, die gerade noch lesbar sind, sperren nicht.
#[test]
fn lesbare_zeitraeume_an_den_grenzen_sperren_nicht() {
    for text in [
        "01.01-31.12",
        "28.02-28.02",
        "31.01-31.01",
        "30.04-30.04",
        "01.12-31.12",
        "31.12-31.12",
    ] {
        let a = einzel(1, &kind_n(1, ("1", text), ("1", GANZ)));
        assert!(matches!(a, Ausgang::Zahl(..)), "{text}: {a:?}");
    }
}

/// Der Rahmen verlaesst sich nicht auf den Guard: erreicht ihn ein Fall trotzdem, meldet er die Sperre als Fehler,
/// statt den Normalfall zu rechnen. Der zweite Fall liest das Steuerjahr: 29.02. gibt es 2025 nicht.
#[test]
fn rahmen_ohne_guard_meldet_die_sperre_statt_zu_raten() {
    let verteilung = kind_n(1, ("1", GANZ), ("1", ZWEITE_HAELFTE));
    let a = ohne_guard(&fall(Scheibe::Gesamt, &basis_zusammen(1), &verteilung));
    assert_eq!(
        a,
        Ausgang::Anders("KindFreibetragGesperrt(KindFreibetragVerteilungOffen)".to_owned())
    );
    let schalttag = kind_n(1, ("1", "29.02-31.12"), ("1", GANZ));
    let a = ohne_guard(&fall(Scheibe::Gesamt, &basis_einzel(1), &schalttag));
    assert_eq!(
        a,
        Ausgang::Anders("KindFreibetragGesperrt(KindZeitraumUnlesbar)".to_owned())
    );
}

/// Das Steuerjahr, das `tarif::rahmen` durchreicht, bindet den 29.02.: ueber den Weg des Rahmens (`feste_zahl`, ohne
/// Guard) ist `29.02-31.12` im Schaltjahr 2024 rechenbar und 2025 und 2026 gesperrt. Im Schaltjahr gilt der
/// Zeitraum Februar bis Dezember: elf Monate Kindergeld, 11 x 250 Euro (`params/2024/kindergeld_p66.yaml`).
/// Faengt ein Jahr daneben (`jahr() + 1`, `jahr() - 1`, ein fester Wert) in `tarif.rs` ab: 2024 wird gesperrt oder
/// 2025 rechenbar.
#[test]
fn rahmen_bindet_den_schalttag_an_das_steuerjahr() {
    let schalttag = kind_n(1, ("1", "29.02-31.12"), ("1", GANZ));
    let im_jahr = |vz: Vz| ohne_guard_im(&fall(Scheibe::Gesamt, &basis_einzel(1), &schalttag), vz);
    assert_eq!(kindergeld(&kette(im_jahr(Vz::Vz2024))), 2_750);
    let gesperrt = Ausgang::Anders("KindFreibetragGesperrt(KindZeitraumUnlesbar)".to_owned());
    assert_eq!(im_jahr(Vz::Vz2025), gesperrt);
    assert_eq!(im_jahr(Vz::Vz2026), gesperrt);
}

/// Wie die Nachbar-Waechter (Python `store is not None and bindung is not None`): ohne Bindung liest der Guard keine
/// Kind-Instanzen und meldet weder Sperre noch Fehler. Gegenprobe: mit Bindung sperrt derselbe Fall.
#[test]
fn guard_ohne_bindung_liest_keine_kinder() {
    let f = fall(
        Scheibe::Gesamt,
        &basis_zusammen(1),
        &kind_n(1, ("3", GANZ), ("1", GANZ)),
    );
    let guard = |bindung| {
        let q = Instanzquelle {
            store: Some(&f.store),
            bindung,
            nur_bestaetigt: false,
        };
        an_gesamt_sperrgrund(&f.felder, Some(&f.cfg), Some(VZ), &q).unwrap()
    };
    assert_eq!(guard(None), None);
    assert_eq!(
        guard(Some(&f.index)).map(|g| g.als_str().to_owned()),
        Some(GRUND_VERTEILUNG.to_owned())
    );
}

/// Ein leerer Text oder `null` ist kein Zeitraum: er gilt wie keine Angabe (ganzes Jahr), er sperrt nicht.
#[test]
fn leerer_zeitraum_gilt_wie_keine_angabe() {
    let ohne = einzel(1, &Vec::new());
    let mut kind = kind_n(1, ("1", ""), ("1", ""));
    kind[1].1 = Value::Null;
    assert_eq!(ohne, einzel(1, &kind));
}

/// Das Zeitraum-Feld von Person B, das die Einzelveranlagung nie liest, sperrt dort auch nicht, wenn es kaputt ist.
#[test]
fn einzel_sperrt_nicht_wegen_person_b() {
    let a = einzel(1, &kind_n(1, ("1", GANZ), ("1", "kaputt")));
    assert!(matches!(a, Ausgang::Zahl(..)), "{a:?}");
}

// ---------------------------------------------------------------- vorlaeufige Werte

/// Ein vorlaeufiger, lesbarer Teiljahr-Zeitraum bewegt die festgesetzte Zahl nie (Zwei-Signal-Invariante): sie
/// rechnet mit dem Normalfall, der Guard sperrt nicht.
#[test]
fn vorlaeufiger_zeitraum_bewegt_die_festgesetzte_zahl_nicht() {
    let ohne = zusammen(1, &Vec::new());
    let mut kind = kind_n(1, ("1", ZWEITE_HAELFTE), ("1", ZWEITE_HAELFTE));
    for e in &mut kind {
        e.2 = false;
    }
    assert_eq!(ohne, zusammen(1, &kind));
}

/// Der Guard sieht auch vorlaeufige Werte: ein vorlaeufiges Stiefkind sperrt, weil der Schaetz-Pfad (nicht nur
/// bestaetigter Werte) es rechnen wuerde und dort ein Normalfall-Betrag falsch waere.
#[test]
fn vorlaeufiges_stiefkind_sperrt() {
    let mut kind = kind_n(1, ("1", GANZ), ("3", GANZ));
    kind[2].2 = false;
    let a = zusammen(1, &kind);
    assert_eq!(a, Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned()));
}

/// Nur die strenge Sicht ergibt einen Widerspruch: A ist vorlaeufig ab Juli, B bestaetigt ab Juli. In der rohen Sicht
/// sind beide Zeitraeume gleich, in der strengen (A = ganzes Jahr, B = ab Juli) verschieden. Der Guard prueft beide
/// Sichten; sonst bekaeme die streng rechnende Zahl einen Fehler statt einer Sperre.
#[test]
fn widerspruch_nur_in_der_strengen_sicht_sperrt() {
    let mut kind = kind_n(1, ("1", ZWEITE_HAELFTE), ("1", ZWEITE_HAELFTE));
    kind[1].2 = false; // Zeitraum A vorlaeufig
    let a = zusammen(1, &kind);
    assert_eq!(a, Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned()));
}

// ---------------------------------------------------------------- Rentner-Scheibe (gleicher Rahmen)

/// Rentner im Beginnjahr (kein Fixierungswert noetig); der Standardwert 0 des Beginnjahrs sperrte vor den Kindern.
fn basis_rentner() -> Paare {
    vec![
        ("veranlagung", json!("einzel")),
        ("fam_anzahl_kinder", json!(1)),
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_renten_beginn_jahr", json!(2025)),
    ]
}

/// Die Scheibe `rentner_gesamt` rechnet den Rahmen `tarif::rahmen` mit; ihr Guard sperrt auch.
#[test]
fn rentner_scheibe_sperrt_wie_gesamt() {
    let basis = basis_rentner();
    let a = rechne(
        Scheibe::RentnerGesamt,
        &basis,
        &kind_n(1, ("3", GANZ), ("1", GANZ)),
    );
    assert_eq!(a, Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned()));
}

/// Gegenprobe zur Rentner-Scheibe: der Normalfall rechnet, der Teiljahr-Fall kuerzt das Kindergeld.
#[test]
fn rentner_scheibe_kuerzt_das_kindergeld_nach_monaten() {
    let basis = basis_rentner();
    let voll = kette(rechne(Scheibe::RentnerGesamt, &basis, &Vec::new()));
    let teil = kette(rechne(
        Scheibe::RentnerGesamt,
        &basis,
        &kind_n(1, ("1", "01.04-31.12"), ("1", GANZ)),
    ));
    assert_eq!(kindergeld(&voll), 3_060);
    assert_eq!(kindergeld(&teil), 2_295);
}

// ---------------------------------------------------------------- `IdNr` des Kindes (Satz 12, nicht gebaut)

/// IST-ZUSTAND, Satz 12 nicht gebaut, Entscheidung offen (§ 32 Abs. 6 Satz 12 bis 14 `EStG`: die Identifizierung des Kindes
/// ueber die `IdNr` ist „Voraussetzung fuer die Beruecksichtigung des Kinderfreibetrags“, Satz 13 laesst eine andere geeignete
/// Identifizierung zu, Satz 14 wirkt rueckwirkend; Kopfkommentar von `kinderfreibetrag.rs`, Bericht `kind-vorarbeit.md`, 3.2).
/// Der Rechenweg liest `kind_idnr` am Kinderfreibetrag nie: ein Kind ohne `IdNr`, mit gueltiger, mit zu kurzer und mit einer
/// `IdNr` aus Buchstaben rechnet dieselbe Zahl, und der Freibetrag gewinnt. Wird Satz 12 gebaut (Hinweis oder Sperre), wird dieser
/// Test rot, und die Erwartung gehoert in dieselbe Aenderung. Rot ohne diese Entscheidung heisst: die `IdNr` bewegt die Zahl
/// ploetzlich, ohne dass es jemand entschieden hat.
#[test]
fn ist_zustand_der_kinderfreibetrag_haengt_nicht_an_der_idnr_des_kindes() {
    let normal = kind_n(1, ("1", GANZ), ("1", GANZ));
    let ohne = zusammen(1, &normal);
    assert!(matches!(ohne, Ausgang::Zahl(..)), "{ohne:?}");
    for (name, idnr) in [
        ("gueltige IdNr", "13579246007"),
        ("zu kurz (10 Zeichen)", "1234567890"),
        ("keine Ziffern", "abcdefghijk"),
    ] {
        let mut mit = normal.clone();
        mit.push(("kind_idnr", json!(idnr), true));
        assert_eq!(
            zusammen(1, &mit),
            ohne,
            "{name}: die IdNr bewegt die Zahl (IST-ZUSTAND: Satz 12 nicht gebaut)"
        );
    }
    let k = kette(ohne);
    assert_eq!(
        k.p31.as_ref().unwrap().guenstiger,
        P31Sieger::Freibetraege,
        "ohne IdNr gewinnt der Freibetrag (IST-ZUSTAND)"
    );
}

// ---------------------------------------------------------------- Satz 3 Nr. 1: anderer Elternteil verstorben / im Ausland (C2)
//
// § 32 Abs. 6 Satz 3 Nr. 1 EStG (`sources/gesetze-im-internet/estg_p32_2026-07-11.txt`): "Die Betraege nach Satz 2
// stehen dem Steuerpflichtigen auch dann zu, wenn 1. der andere Elternteil verstorben oder nicht unbeschraenkt
// einkommensteuerpflichtig ist". Satz 2 sind die verdoppelten Betraege. Bei Einzelveranlagung bekommt der
// Ueberlebende (oder der Elternteil im Inland) in jedem solchen Monat BEIDE Zwoelftel statt einem (Satz 5 je Monat).
// Zwei Angaben je Kind, nur bei Einzelveranlagung, Kz E0501102 (Todestag, TT.MM.JJJJ) und E0503903 (Zeitraum
// TT.MM-TT.MM, in dem der andere Elternteil im Ausland lebte).
//
// HANDRECHNUNG (VZ 2025, nur aus `params/`, kein Wert aus dem Rust-Code): Freibetrag je Elternteil 3.336 + 1.464 =
// 4.800 Euro (`kinderfreibetrag_p32.yaml`), Kindergeld 255 Euro je Monat = 3.060 Euro (`kindergeld_p66.yaml`),
// zvE = Bruttolohn - 1.230 (`arbeitnehmerpauschbetrag.yaml`) - 36 (`sonderausgabenpauschbetrag.yaml`), Tarif
// § 32a (`einkommensteuertarif_p32a.yaml`), die Steuer auf volle Euro abgerundet. Ein Personenmonat sind 400 Euro.
//
// Zone 3 (bis 68.480): y = (zvE - 17.443) / 10.000, Steuer = (176,64 y + 2.397) y + 1.015,13.
// Zone 4 (bis 277.825): 0,42 zvE - 10.911,92. Zone 5: 0,45 zvE - 19.246,67.
//
//   zvE vor Kind   ohne Satz 3 (12 PM = 4.800)   Satz 3 ganzes Jahr (24 PM = 9.600)
//   50.000         Steuer 10.691 (y 3,2557)        mit 40.400: 7.448 (y 2,2957) + 3.060 = 10.508 < 10.691
//   60.000         Steuer 14.415 (y 4,2557)        mit 50.400: 10.833 (y 3,2957) + 3.060 = 13.893 < 14.415
//   80.000         Steuer 22.688 (22.688,08)       mit 70.400: 18.656 (18.656,08) + 3.060 = 21.716 < 22.688
//
// Ohne Satz 3 gewinnt bei Einzelveranlagung das Kindergeld: 50.000 mit 4.800 abgezogen ergibt 9.029 + 3.060 = 12.089
// > 10.691; ebenso 14.415 gegen 12.583 + 3.060 = 15.643 und 22.688 gegen 20.672 + 3.060 = 23.732. Die Ersparnis durch
// Satz 3 ist 183 / 522 / 972 Euro (10.691 - 10.508, 14.415 - 13.893, 22.688 - 21.716).

const TOD: &str = "kind_anderer_elternteil_tod_am";
const AUSLAND: &str = "kind_anderer_elternteil_ausland_zeitraum";

/// Eine Zeile der Handrechnung, alles Euro: (zvE, tarifliche Steuer, festzusetzende Steuer, Sieger der Guenstigerpruefung).
type Soll = (i64, i64, i64, P31Sieger);

fn pruefe(k: &Kette, soll: &Soll, wo: &str) {
    let ist = (
        k.zu_versteuerndes_einkommen.get(),
        k.tarifliche_est.get(),
        k.festzusetzende_est.get(),
        k.p31.as_ref().expect("KONTROLLE: kein p31").guenstiger,
    );
    assert_eq!(&ist, soll, "{wo}");
}

/// Die beiden Angaben zum anderen Elternteil der Instanz `n`, bestaetigt; `None` = nicht geschrieben.
fn satz3(n: u8, tod: Option<Value>, ausland: Option<Value>) -> KindEvents {
    let mut e = Vec::new();
    if let Some(w) = tod {
        e.push((instanz_id(n, TOD), w, true));
    }
    if let Some(w) = ausland {
        e.push((instanz_id(n, AUSLAND), w, true));
    }
    e
}

fn tod(datum: &str) -> KindEvents {
    satz3(1, Some(json!(datum)), None)
}

fn ausland(zeitraum: &str) -> KindEvents {
    satz3(1, None, Some(json!(zeitraum)))
}

/// Einzelveranlagung, ein Bruttolohn in Euro, `kinder` Kinder.
fn einzel_brutto(brutto_euro: i64, kinder: i64, kind: &KindEvents) -> Ausgang {
    let basis = vec![
        ("veranlagung", json!("einzel")),
        ("bruttoarbeitslohn", json!(brutto_euro * 100)),
        ("fam_anzahl_kinder", json!(kinder)),
    ];
    rechne(Scheibe::Gesamt, &basis, kind)
}

/// Bruttolohn und Handrechnung bei zvE 50.000 / 60.000 / 80.000 Euro vor dem Kind (Brutto = zvE + 1.266):
/// `(Brutto, ohne Satz 3, Satz 3 ein ganzes Jahr)`.
fn faelle_50_60_80() -> [(i64, Soll, Soll); 3] {
    [
        (
            51_266,
            (50_000, 10_691, 10_691, P31Sieger::Kindergeld),
            (40_400, 7_448, 10_508, P31Sieger::Freibetraege),
        ),
        (
            61_266,
            (60_000, 14_415, 14_415, P31Sieger::Kindergeld),
            (50_400, 10_833, 13_893, P31Sieger::Freibetraege),
        ),
        (
            81_266,
            (80_000, 22_688, 22_688, P31Sieger::Kindergeld),
            (70_400, 18_656, 21_716, P31Sieger::Freibetraege),
        ),
    ]
}

/// Zone 5, zvE vor dem Kind 298.734 Euro (Brutto 300.000): jeder Personenmonat mehr senkt den Abzug sichtbar, weil
/// ab 18 Personenmonaten (7.200 Euro x 45 % = 3.240 > 3.060) der Freibetrag das Kindergeld schlaegt. Brutto 300.000.
const BRUTTO_ZONE_5: i64 = 300_000;
/// Ohne Satz 3: 4.800 x 0,45 = 2.160 < 3.060, das Kindergeld gewinnt; 0,45 x 298.734 - 19.246,67 = 115.183,63.
const OHNE_ZONE_5: Soll = (298_734, 115_183, 115_183, P31Sieger::Kindergeld);

/// Zone 5 mit Freibetrag `fb` Euro und Kindergeld 3.060: (298.734 - fb, Steuer darauf, Steuer + 3.060, Freibetraege).
/// Die Zahlen stehen unten je Fall mit der Rechnung; hier nur die Form.
const fn zone5(fb: i64, tarifl: i64, kindergeld: i64) -> Soll {
    (298_734 - fb, tarifl, tarifl + kindergeld, P31Sieger::Freibetraege)
}

/// Ein Elternteil im Vorjahr verstorben: der Ueberlebende hat in allen zwoelf Monaten beide Zwoelftel (24 Personenmonate,
/// 9.600 Euro), das Kindergeld bleibt 3.060. Gegenprobe im selben Test: ohne die Angabe gewinnt das Kindergeld.
#[test]
fn tod_im_vorjahr_gibt_den_vollen_betrag_bei_50_60_80_tausend() {
    for (brutto, ohne, mit) in faelle_50_60_80() {
        let k0 = kette(einzel_brutto(brutto, 1, &Vec::new()));
        pruefe(&k0, &ohne, &format!("ohne Angabe, brutto {brutto}"));
        let k = kette(einzel_brutto(brutto, 1, &tod("31.12.2024")));
        pruefe(&k, &mit, &format!("Tod 31.12.2024, brutto {brutto}"));
        assert_eq!(kindergeld(&k), 3_060, "das Kindergeld bleibt zwoelf Monate");
    }
}

/// Der Todestag liegt im Jahr 2000 oder am 01.01.1900: lesbar, lange vorbei, das ganze Jahr zaehlt.
#[test]
fn tod_lange_vorher_zaehlt_das_ganze_jahr() {
    let (brutto, _, mit) = faelle_50_60_80()[0];
    for datum in ["29.02.2000", "01.01.1900", "31.12.2024"] {
        let k = kette(einzel_brutto(brutto, 1, &tod(datum)));
        pruefe(&k, &mit, datum);
    }
}

/// Der andere Elternteil lebte das ganze Jahr im Ausland und war nicht unbeschraenkt steuerpflichtig: dieselben Zahlen.
#[test]
fn ausland_ganzes_jahr_gibt_den_vollen_betrag_bei_50_60_80_tausend() {
    for (brutto, _, mit) in faelle_50_60_80() {
        let k = kette(einzel_brutto(brutto, 1, &ausland(GANZ)));
        pruefe(&k, &mit, &format!("Ausland {GANZ}, brutto {brutto}"));
        assert_eq!(kindergeld(&k), 3_060);
    }
}

/// Beides zugleich (verstorben UND vorher im Ausland): die Monate werden vereinigt, nicht addiert. 24 Personenmonate
/// sind die Obergrenze (zwei Elternteile mal zwoelf Monate), nie 36.
#[test]
fn tod_und_ausland_ganzes_jahr_zaehlen_die_monate_einmal() {
    for (brutto, _, mit) in faelle_50_60_80() {
        let mut kind = tod("31.12.2024");
        kind.extend(ausland(GANZ));
        let k = kette(einzel_brutto(brutto, 1, &kind));
        pruefe(&k, &mit, &format!("Tod und Ausland, brutto {brutto}"));
    }
}

/// Tod im Juli (15.07.): sechs Monate, 18 Personenmonate, Freibetrag 7.200. Bei 50 / 60 / 80 Tausend reicht das nicht,
/// das Kindergeld gewinnt weiter: 8.228 + 3.060 = 11.288 > 10.691; 11.698 + 3.060 = 14.758 > 14.415;
/// 19.664 + 3.060 = 22.724 > 22.688 (mit 7.200 abgezogen: Steuer von 42.800 / 52.800 / 72.800).
#[test]
fn tod_im_juli_aendert_bei_50_60_80_tausend_nichts() {
    for (brutto, ohne, _) in faelle_50_60_80() {
        let k = kette(einzel_brutto(brutto, 1, &tod("15.07.2025")));
        pruefe(&k, &ohne, &format!("Tod 15.07.2025, brutto {brutto}"));
    }
}

/// Todesmonat und Monatsgrenze, Zone 5. Der Monat des Todes zaehlt voll (Anleitung zur Anlage Kind: Freibetraege "fuer
/// jeden angefangenen Kalendermonat"; derselbe Rechenweg wie bei Satz 5): der Tod am 30.06. gibt Juni bis Dezember =
/// 7 Monate, der Tod am 01.07. und am 31.07. gleich Juli bis Dezember = 6 Monate.
///   7 Monate: 12 + 7 = 19 PM, 19 x 400 = 7.600; zvE 291.134; 0,45 x 291.134 - 19.246,67 = 111.763,63 -> 111.763;
///             mit Kindergeld 114.823 < 115.183.
///   6 Monate: 18 PM, 7.200; zvE 291.534; 111.943,63 -> 111.943; mit Kindergeld 115.003 < 115.183.
#[test]
fn todesmonat_zaehlt_voll_und_die_monatsgrenze_verschiebt_um_einen_monat() {
    let sieben = zone5(7_600, 111_763, 3_060);
    let sechs = zone5(7_200, 111_943, 3_060);
    for (datum, soll) in [
        ("30.06.2025", &sieben),
        ("01.06.2025", &sieben),
        ("01.07.2025", &sechs),
        ("15.07.2025", &sechs),
        ("31.07.2025", &sechs),
    ] {
        let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &tod(datum)));
        pruefe(&k, soll, datum);
        assert_eq!(kindergeld(&k), 3_060, "{datum}");
    }
}

/// Tod am 01.01.2025: das ganze Jahr zaehlt (zwoelf Monate), wie ein Tod im Vorjahr. 24 PM = 9.600; zvE 289.134;
/// 0,45 x 289.134 - 19.246,67 = 110.863,63 -> 110.863; mit Kindergeld 113.923.
#[test]
fn tod_am_ersten_januar_zaehlt_das_ganze_jahr() {
    let ganz = zone5(9_600, 110_863, 3_060);
    for datum in ["01.01.2025", "31.12.2024"] {
        let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &tod(datum)));
        pruefe(&k, &ganz, datum);
    }
}

/// Der Tod nach dem Jahr oder am Jahresende wirkt nicht oder zu wenig, um etwas zu bewegen: ein Tod 2026 aendert
/// nichts (das Jahr 2025 hatte zwei Elternteile), der 31.12.2025 gibt einen Monat (13 PM = 5.200 Euro, 2.340 < 3.060:
/// das Kindergeld gewinnt weiter).
#[test]
fn tod_nach_dem_jahr_aendert_nichts_und_der_letzte_tag_gibt_einen_monat() {
    let ohne = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &Vec::new()));
    pruefe(&ohne, &OHNE_ZONE_5, "ohne Angabe");
    for datum in ["01.01.2026", "31.12.2025", "31.12.2999"] {
        let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &tod(datum)));
        pruefe(&k, &OHNE_ZONE_5, datum);
    }
}

/// Ausland-Zeitraum, Zone 5: jeder angefangene Monat zaehlt.
///   "01.04-31.10": April bis Oktober = 7 Monate, 19 PM, wie oben (111.763).
///   "30.04-31.10": ebenfalls 7 Monate (der 30.04. zaehlt als April).
///   "01.05-31.10": Mai bis Oktober = 6 Monate, 18 PM (111.943).
///   "15.03-14.10": Maerz bis Oktober = 8 Monate, 20 PM = 8.000; zvE 290.734; 111.583,63 -> 111.583; 114.643 < 115.183.
#[test]
fn ausland_zeitraum_zaehlt_die_angefangenen_monate() {
    let sieben = zone5(7_600, 111_763, 3_060);
    let sechs = zone5(7_200, 111_943, 3_060);
    let acht = zone5(8_000, 111_583, 3_060);
    for (zeitraum, soll) in [
        ("01.04-31.10", &sieben),
        ("30.04-31.10", &sieben),
        ("01.05-31.10", &sechs),
        ("15.03-14.10", &acht),
    ] {
        let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &ausland(zeitraum)));
        pruefe(&k, soll, zeitraum);
    }
}

/// Ein leeres Feld ist keine Angabe: `null` und `""` bedeuten beim Ausland-Zeitraum KEIN Monat (nicht das ganze Jahr,
/// wie beim Kindschaftszeitraum). Sonst bekaeme jeder, der das Feld leer laesst, den doppelten Freibetrag.
#[test]
fn leeres_ausland_feld_ist_kein_monat() {
    for leer in [json!(""), Value::Null] {
        let kind = satz3(1, None, Some(leer.clone()));
        let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &kind));
        pruefe(&k, &OHNE_ZONE_5, &format!("Ausland {leer:?}"));
    }
    let kind = satz3(1, Some(json!("")), None);
    let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &kind));
    pruefe(&k, &OHNE_ZONE_5, "Tod leer");
    let kind = satz3(1, Some(Value::Null), None);
    let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &kind));
    pruefe(&k, &OHNE_ZONE_5, "Tod null");
}

/// Die Monate von Tod und Ausland werden vereinigt: Tod am 01.04. (April bis Dezember, 9 Monate) und Ausland
/// "01.03-30.06" (Maerz bis Juni): zusammen Maerz bis Dezember = 10 Monate, 22 PM = 8.800; zvE 289.934;
/// 0,45 x 289.934 - 19.246,67 = 111.223,63 -> 111.223; mit Kindergeld 114.283. Eine Summe (9 + 4 = 13) gaebe 25 PM.
#[test]
fn tod_und_ausland_mit_ueberschneidung_zaehlen_die_vereinigung() {
    let mut kind = tod("01.04.2025");
    kind.extend(ausland("01.03-30.06"));
    let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &kind));
    pruefe(&k, &zone5(8_800, 111_223, 3_060), "Maerz bis Dezember");
}

/// Satz 3 gilt nur in den Monaten, in denen das Kind zum Steuerpflichtigen gehoert (Satz 5 zaehlt Monate, in denen die
/// Voraussetzungen fuer den Freibetrag vorliegen): das Kind gehoert ihm erst ab Juli (6 Monate), der andere Elternteil
/// ist seit dem 01.03. verstorben (Maerz bis Dezember). Beide Mengen zusammen: Juli bis Dezember = 6 Monate Satz 3
/// -> 6 + 6 = 12 PM = 4.800; das Kindergeld folgt dem Kind: 6 x 255 = 1.530. Zone 5: zvE 293.934;
/// 0,45 x 293.934 - 19.246,67 = 113.023,63 -> 113.023; mit Kindergeld 114.553 < 115.183. Ohne den Schnitt waeren es
/// 6 + 10 = 16 PM.
#[test]
fn satz_3_zaehlt_nur_in_den_monaten_des_kindes() {
    let mut kind = kind_n(1, ("1", ZWEITE_HAELFTE), ("1", GANZ));
    kind.extend(tod("01.03.2025"));
    let k = kette(einzel_brutto(BRUTTO_ZONE_5, 1, &kind));
    pruefe(&k, &(293_934, 113_023, 114_553, P31Sieger::Freibetraege), "ab Juli");
    assert_eq!(kindergeld(&k), 1_530);
}

/// Zwei Kinder, nur das zweite (Instanz `__2`) hat einen verstorbenen anderen Elternteil: 12 + 24 = 36 PM = 14.400;
/// das Kindergeld bleibt 2 x 3.060 = 6.120. Zone 5: zvE 284.334; 0,45 x 284.334 - 19.246,67 = 108.703,63 -> 108.703;
/// mit Kindergeld 114.823 < 115.183. Ohne Satz 3 (24 PM = 9.600): 110.863 + 6.120 = 116.983 > 115.183, das Kindergeld
/// gewinnt. Die Angabe von Kind 2 wirkt nicht auf Kind 1: stuende sie bei Kind 1, waeren es dieselben 36 PM, darum
/// steht der Gegenfall (Tod nur bei Kind 1) daneben und muss gleich sein; ein Tod bei einem Kind 3 ohne Kinderzahl
/// zaehlt nicht.
#[test]
fn zweites_kind_traegt_seinen_eigenen_todestag() {
    let normal = kind_n(1, ("1", GANZ), ("1", GANZ));
    let mut kind = normal.clone();
    kind.extend(satz3(2, Some(json!("31.12.2024")), None));
    let k = kette(einzel_brutto(BRUTTO_ZONE_5, 2, &kind));
    pruefe(
        &k,
        &(284_334, 108_703, 114_823, P31Sieger::Freibetraege),
        "Tod bei Kind 2",
    );
    assert_eq!(kindergeld(&k), 6_120);
    let mut kind1 = normal;
    kind1.extend(satz3(1, Some(json!("31.12.2024")), None));
    let k1 = kette(einzel_brutto(BRUTTO_ZONE_5, 2, &kind1));
    assert_eq!(k1, k, "Tod bei Kind 1 statt bei Kind 2: dieselbe Summe");
    let ohne = kette(einzel_brutto(BRUTTO_ZONE_5, 2, &Vec::new()));
    pruefe(
        &ohne,
        &(298_734, 115_183, 115_183, P31Sieger::Kindergeld),
        "zwei Kinder ohne Satz 3",
    );
    let mut ueber = kind_n(1, ("1", GANZ), ("1", GANZ));
    ueber.extend(satz3(3, Some(json!("31.12.2024")), None));
    assert_eq!(
        einzel_brutto(BRUTTO_ZONE_5, 2, &ueber),
        einzel_brutto(BRUTTO_ZONE_5, 2, &Vec::new()),
        "Instanz 3 gibt es bei zwei Kindern nicht"
    );
}

/// Zusammenveranlagung liest die beiden Felder nicht: beide Ehegatten haben das Kind, die Betraege sind dort schon
/// verdoppelt (Satz 2), und der Zweig `Weit_Ang` der Anlage Kind gilt nur fuer nicht zusammen veranlagte Eltern.
#[test]
fn zusammenveranlagung_liest_die_satz_3_felder_nicht() {
    let ohne = zusammen(1, &Vec::new());
    assert!(matches!(ohne, Ausgang::Zahl(..)), "{ohne:?}");
    let mut kind = tod("31.12.2024");
    kind.extend(ausland(GANZ));
    assert_eq!(zusammen(1, &kind), ohne);
    // Auch ein kaputter Wert sperrt dort nicht: der Rahmen liest ihn nie.
    let mut kaputt = tod("31.02.2025");
    kaputt.extend(ausland("kaputt"));
    assert_eq!(zusammen(1, &kaputt), ohne);
}

/// Nicht lesbare Angaben sperren, nie raten, nie "kein Tod". Das Datum ist `TT.MM.JJJJ`, streng.
#[test]
fn unlesbarer_todestag_sperrt() {
    let faelle = [
        ("31.02.2025", "31. Februar gibt es nicht"),
        ("29.02.2025", "2025 ist kein Schaltjahr"),
        ("29.02.2100", "2100 ist kein Schaltjahr"),
        ("31.04.2025", "31. April gibt es nicht"),
        ("00.03.2025", "Tag 0"),
        ("15.00.2025", "Monat 0"),
        ("15.13.2025", "Monat 13"),
        ("1.3.2025", "Tag und Monat zweistellig"),
        ("15.03.25", "Jahr vierstellig"),
        ("15.03.20255", "Jahr zu lang"),
        ("2025-03-15", "ISO-Form"),
        ("15/03/2025", "Schraegstriche"),
        (" 15.03.2025", "Leerzeichen davor"),
        ("15.03.2025\n", "angehaengter Zeilenumbruch"),
        ("15.03.999", "Jahr dreistellig"),
        ("abc", "kein Datum"),
    ];
    let mut falsch = Vec::new();
    for (text, warum) in faelle {
        let a = einzel(1, &tod(text));
        if a != Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()) {
            falsch.push(format!("{text:?} ({warum}): {a:?}"));
        }
    }
    for w in [json!(5), json!(true), json!(["15.03.2025"]), json!(2025.5)] {
        let a = einzel(1, &satz3(1, Some(w.clone()), None));
        if a != Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()) {
            falsch.push(format!("{w}: {a:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Der Ausland-Zeitraum wird wie jeder Zeitraum der Kinder gelesen (`TT.MM-TT.MM` im Steuerjahr).
#[test]
fn unlesbarer_ausland_zeitraum_sperrt() {
    let faelle = [
        ("31.02-31.12", "31. Februar gibt es nicht"),
        ("29.02-31.12", "2025 ist kein Schaltjahr"),
        ("01.12-01.01", "Ende vor Anfang"),
        ("1.1-31.12", "Tag und Monat zweistellig"),
        ("01.01 - 31.12", "Leerzeichen"),
        ("01.13-31.12", "Monat 13"),
        ("abc", "kein Zeitraum"),
        ("01.01.2025-31.12.2025", "Datum statt Tag und Monat"),
    ];
    let mut falsch = Vec::new();
    for (text, warum) in faelle {
        let a = einzel(1, &ausland(text));
        if a != Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()) {
            falsch.push(format!("{text:?} ({warum}): {a:?}"));
        }
    }
    for w in [json!(5), json!(true), json!(2025.5)] {
        let a = einzel(1, &satz3(1, None, Some(w.clone())));
        if a != Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()) {
            falsch.push(format!("{w}: {a:?}"));
        }
    }
    assert!(falsch.is_empty(), "{falsch:#?}");
}

/// Gegenprobe: was gerade noch lesbar ist, sperrt nicht. Der 29.02. eines Todestags gilt nach dem Kalender SEINES
/// Jahres (2024 und 2000 waren Schaltjahre), nicht nach dem Steuerjahr.
#[test]
fn lesbare_angaben_an_den_grenzen_sperren_nicht() {
    for datum in [
        "29.02.2024",
        "29.02.2000",
        "31.12.2025",
        "01.01.2025",
        "28.02.2025",
        "01.01.1900",
        "15.03.0000",
    ] {
        let a = einzel(1, &tod(datum));
        assert!(matches!(a, Ausgang::Zahl(..)), "{datum}: {a:?}");
    }
    for zeitraum in ["01.01-31.12", "28.02-28.02", "31.12-31.12"] {
        let a = einzel(1, &ausland(zeitraum));
        assert!(matches!(a, Ausgang::Zahl(..)), "{zeitraum}: {a:?}");
    }
}

/// Ein vorlaeufiger, lesbarer Todestag bewegt die festgesetzte Zahl nie (Zwei-Signal-Invariante), wie beim
/// Teiljahr-Zeitraum: sie rechnet mit dem Normalfall, der Guard sperrt nicht.
#[test]
fn vorlaeufiger_todestag_bewegt_die_festgesetzte_zahl_nicht() {
    let ohne = einzel_brutto(BRUTTO_ZONE_5, 1, &Vec::new());
    let mut kind = tod("31.12.2024");
    kind.extend(ausland(GANZ));
    for e in &mut kind {
        e.2 = false;
    }
    assert_eq!(einzel_brutto(BRUTTO_ZONE_5, 1, &kind), ohne);
}

/// Der Guard sieht auch vorlaeufige Werte: ein vorlaeufiger, unlesbarer Todestag oder Zeitraum sperrt, statt dass
/// die Schaetz-Zahl (die ihn liest) mit einem Fehler endet.
#[test]
fn vorlaeufig_unlesbar_sperrt() {
    for (tod_w, ausland_w) in [(Some(json!("31.02.2025")), None), (None, Some(json!("kaputt")))] {
        let mut kind = satz3(1, tod_w, ausland_w);
        for e in &mut kind {
            e.2 = false;
        }
        let a = einzel(1, &kind);
        assert_eq!(a, Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()), "{kind:?}");
    }
}

/// Der Rahmen verlaesst sich nicht auf den Guard (wie bei den Zeitraeumen): erreicht ihn ein unlesbarer Todestag,
/// meldet er die Sperre als Fehler, statt "kein Tod" zu rechnen.
#[test]
fn rahmen_ohne_guard_meldet_einen_unlesbaren_todestag() {
    let a = ohne_guard(&fall(Scheibe::Gesamt, &basis_einzel(1), &tod("31.02.2025")));
    assert_eq!(
        a,
        Ausgang::Anders("KindFreibetragGesperrt(KindZeitraumUnlesbar)".to_owned())
    );
    let a = ohne_guard(&fall(Scheibe::Gesamt, &basis_einzel(1), &ausland("kaputt")));
    assert_eq!(
        a,
        Ausgang::Anders("KindFreibetragGesperrt(KindZeitraumUnlesbar)".to_owned())
    );
}

/// Ein Kind, das zu keinem der beiden im Sinn von Abs. 1 gehoert (Stiefkind), bleibt gesperrt, auch mit Todestag:
/// Satz 3 setzt ein Kindschaftsverhaeltnis voraus.
#[test]
fn stiefkind_bleibt_auch_mit_todestag_gesperrt() {
    let mut kind = kind_n(1, ("3", GANZ), ("1", GANZ));
    kind.extend(tod("31.12.2024"));
    assert_eq!(
        einzel(1, &kind),
        Ausgang::Gesperrt(GRUND_VERTEILUNG.to_owned())
    );
}

/// Rentner-Scheibe, derselbe Rahmen: ein Todestag im Vorjahr gibt auch dort den vollen Betrag. Beginnjahr 2025
/// ohne Einkuenfte: zvE 0, der Freibetrag aendert die Steuer nicht -- darum sieht man nur, dass die Scheibe
/// rechnet (Zahl) und mit unlesbarem Wert sperrt.
#[test]
fn rentner_scheibe_liest_die_satz_3_felder() {
    let basis = basis_rentner();
    let a = rechne(Scheibe::RentnerGesamt, &basis, &tod("31.12.2024"));
    assert!(matches!(a, Ausgang::Zahl(..)), "{a:?}");
    let a = rechne(Scheibe::RentnerGesamt, &basis, &tod("31.02.2025"));
    assert_eq!(a, Ausgang::Gesperrt(GRUND_ZEITRAUM.to_owned()));
}
