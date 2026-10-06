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

/// Die vier Kind-Felder der Instanz `n` (1 = Basis-Feld-ID, `n` >= 2 = `__n`), alle bestaetigt.
fn kind_n(n: u8, a: (&str, &str), b: (&str, &str)) -> KindEvents {
    // `Box::leak`: die Instanz-IDs der Test-Tabelle sind `&'static str`, wie die Basis-IDs.
    let id = |basis: &'static str| -> &'static str {
        if n == 1 {
            basis
        } else {
            Box::leak(format!("{basis}__{n}").into_boxed_str())
        }
    };
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
