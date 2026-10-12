//! § 24a Satz 4 `EStG`: bei Zusammenveranlagung wird der Altersentlastungsbetrag fuer jeden Ehegatten gesondert aus
//! dessen EIGENEN Einkuenften bemessen (Abweichung Nr. 62, Bezug Nr. 36). Beide Zweige (`festzusetzende_est_gesamt`
//! und `festzusetzende_est_rentner`), im Standardlauf, ohne `PARITY=1`, ohne Python.
//!
//! WARUM ES DIESE DATEI GIBT. Vorher floss der Gewinn des Ehegatten in die Bemessung von Person A, und Person B bekam
//! nie etwas (Gesamt-Zweig); der Rentner-Zweig liess B ebenfalls leer ausgehen. Gemessen am 2026-10-12 (HEAD `7128eb9e`,
//! Spiegelprobe): 242 EUR zu hoch oder zu niedrig je Fall. Python rechnete § 24a nur fuer Person A (Nr. 36), also gibt es
//! keinen Python-Wert; die Erwartung kommt aus dem Gesetz und von Hand.
//!
//! METHODE (Spiegelprobe). Bei Zusammenveranlagung haengt die festzusetzende `ESt` nur von der Summe der Einkuenfte ab,
//! nicht davon, wer sie verdient. Vertauscht man A und B samt Gewinn, muss dieselbe Steuer herauskommen. Dazu kommen die
//! Absolutwerte, von Hand nach § 32a `EStG` (`params/2025/einkommensteuertarif_p32a.yaml`) gerechnet:
//!
//! Alle Faelle: VZ 2025, Gewinn 80.000 EUR (8.000.000 Cent) bei einer Person, kein Lohn, keine Kinder, keine Konfession.
//! Altersentlastung: Jahrgang 1955 faellt in die Zeile 2020 der Tabelle (16,0 %, Hoechstbetrag 760 EUR; `sources/gesetze-im-
//! internet/estg_p24a_2026-07-13.txt`), 16,0 % von 80.000 EUR sind 12.800 EUR, der Deckel gilt: 760 EUR. Jahrgang 1990 bekommt
//! nichts. Sonderausgaben-Pauschbetrag 36 EUR je Person (`params/2025/sonderausgabenpauschbetrag.yaml`), zusammen 72 EUR.
//!
//! - OHNE Altersentlastung: zvE = 80.000 − 72 = 79.928 EUR; Splitting, die Haelfte 39.964 EUR. Zone 3 (17.443 < zvE <= 68.480):
//!   z = (39.964 − 17.443) / 10.000 = 2,2521; (176,64 · z + 2.397) · z + 1.015,13 = 7.309,32 → 7.309 EUR (abgerundet), mal 2 =
//!   14.618 EUR = **1.461.800 Cent**.
//! - MIT 760 EUR: zvE = 80.000 − 760 − 72 = 79.168 EUR; die Haelfte 39.584 EUR; z = 2,2141; Steuer = 7.188,26 → 7.188 EUR, mal 2 =
//!   14.376 EUR = **1.437.600 Cent**.
//! - MIT 2 · 760 EUR (beide verdienen je 40.000 EUR, beide 1955): zvE = 80.000 − 1.520 − 72 = 78.408 EUR; die Haelfte 39.204 EUR;
//!   z = 2,1761; Steuer = 7.067,70 → 7.067 EUR, mal 2 = 14.134 EUR = **1.413.400 Cent**.
//! - EINZELVERANLAGUNG, Jahrgang 1955, Gewinn 80.000 EUR: zvE = 80.000 − 760 − 36 = 79.204 EUR (Zone 4, ueber 68.480 EUR):
//!   0,42 · 79.204 − 10.911,92 = 22.353,76 → 22.353 EUR = **2.235.300 Cent**; der Gewinn des Ehegatten zaehlt hier nirgends.
//!
//! Die Hoehe von `alt_a` und `alt_b` einzeln (320 + 480 EUR und weitere) prueft `zweige/gesamt.rs::tests`, weil
//! `entlastungen` nicht oeffentlich ist.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_types
)]

use std::cell::RefCell;

use bescheid::testhilfe::{felder, index, params, store};
use bescheid::zweige::{bescheid_fn, Extras, Umgebung};
use bindung::SlotBeitrag;
use domain::{Feldtyp, PyWert, Vz};
use intervall::{AchsenBindung, Werte};
use serde_json::{json, Value};

type Paare = Vec<(&'static str, Value)>;

/// Slot der Zweige -> Feld, das ihn speist (`rust/bindung/daten`, Scheibe `gesamt`).
const SLOTS: [(&str, &str); 6] = [
    ("arbeitstage", "ep_arbeitstage"),
    ("entfernung_km_roh", "ep_entfernung_km"),
    ("oepnv_kosten_jahr", "ep_oepnv_kosten"),
    ("eigenes_oder_ueberlassenes_kfz", "ep_eigenes_kfz"),
    ("bruttoarbeitslohn", "bruttoarbeitslohn"),
    ("veranlagung", "veranlagung"),
];

const QUANTITAETEN: [&str; 2] = ["festzusetzende_est_gesamt", "festzusetzende_est_rentner"];

/// 80.000 EUR in Cent.
const G: i64 = 8_000_000;
/// 40.000 EUR in Cent.
const HALB: i64 = 4_000_000;
/// 500.000 EUR in Cent (Veraeusserungsgewinn).
const VERKAUF: i64 = 50_000_000;

/// Festzusetzende `ESt` in Cent, siehe Kopf: ohne Altersentlastung / mit 760 EUR / mit 2 · 760 EUR.
const OHNE: i64 = 1_461_800;
const MIT_760: i64 = 1_437_600;
const MIT_2X760: i64 = 1_413_400;
/// Einzelveranlagung, Jahrgang 1955, Gewinn 80.000 EUR.
const EINZEL_1955: i64 = 2_235_300;

fn basis(veranlagung: &'static str, geb_a: i64, geb_b: i64, gewinn_a: i64, gewinn_b: i64) -> Paare {
    vec![
        ("veranlagung", json!(veranlagung)),
        ("bruttoarbeitslohn", json!(0)),
        ("ep_arbeitstage", json!(0)),
        ("ep_entfernung_km", json!(0)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(false)),
        ("kist_konfession", json!("keine")),
        ("geburtsjahr", json!(geb_a)),
        ("geburtsjahr_partner", json!(geb_b)),
        ("einkuenfte_gewinn", json!(gewinn_a)),
        ("einkuenfte_gewinn_partner", json!(gewinn_b)),
    ]
}

fn py(v: &Value) -> PyWert {
    match v {
        Value::Bool(b) => PyWert::Bool(*b),
        Value::String(s) => PyWert::Text(s.clone()),
        Value::Number(n) => PyWert::Ganz(n.as_i64().unwrap()),
        andere => panic!("Slot-Wert {andere}"),
    }
}

/// Die festzusetzende `ESt` in Cent der Quantitaet `quantitaet` fuer die Felder `paare`.
fn est(quantitaet: &str, paare: &Paare) -> i64 {
    let events: Vec<_> = paare.iter().map(|(f, w)| (*f, w.clone(), true)).collect();
    let f = felder(&store(&events));
    let achsen: Vec<AchsenBindung> = SLOTS
        .iter()
        .map(|(slot, _)| AchsenBindung {
            feld_id: (*slot).to_owned(),
            typ: Feldtyp::Int,
            askable: true,
            enum_werte: Vec::new(),
            bereich: None,
            signatur_slot: Some((*slot).to_owned()),
            slot_beitrag: SlotBeitrag::Exakt,
        })
        .collect();
    let umg = Umgebung {
        achsen: &achsen,
        index: index(),
        params: params(),
    };
    let extras = RefCell::new(Extras::default());
    let rechne = bescheid_fn(
        quantitaet,
        Vz::Vz2025,
        &umg,
        Some(&f),
        None,
        true,
        None,
        Some(&extras),
    )
    .expect("Quantitaet bekannt");
    let mut w = Werte::neu();
    for (slot, feld) in SLOTS {
        let wert = paare.iter().find(|(f, _)| *f == feld).unwrap().1.clone();
        w.setze(slot, py(&wert));
    }
    rechne(&w).expect("Zweig rechnet").get()
}

/// Ein Fall der Spiegelprobe: `(Name, Jahrgang A, Jahrgang B, Gewinn A, Gewinn B, erwartete ESt in Cent)`.
type Fall = (&'static str, i64, i64, i64, i64, i64);

fn pruefe(faelle: &[Fall]) {
    for q in QUANTITAETEN {
        for (name, ga, gb, wa, wb, soll) in faelle {
            let ist = est(q, &basis("zusammen", *ga, *gb, *wa, *wb));
            assert_eq!(ist, *soll, "{q}: {name}");
        }
    }
}

/// Kontrolle: beide 1990, nirgends Altersentlastung. Der Gewinn von A oder von B ergibt dieselbe Steuer (Spiegel). Haelt die
/// Probe selbst dicht: die Steuer ohne Entlastung muss in beiden Zweigen gleich bleiben.
#[test]
fn ohne_berechtigten_ehegatten_bleibt_die_steuer_ohne_entlastung() {
    pruefe(&[
        ("N: beide 1990, Gewinn A", 1990, 1990, G, 0, OHNE),
        ("N': beide 1990, Gewinn B", 1990, 1990, 0, G, OHNE),
    ]);
}

/// Der Ehegatte (1955) hat den Gewinn, Person A (1990) ist zu jung: B bekommt 760 EUR. Vorher bekam niemand etwas (Gesamt und
/// Rentner: 242 EUR zu hoch). Spiegel: A (1955) mit dem Gewinn gibt dieselbe Steuer.
#[test]
fn der_aeltere_ehegatte_mit_eigenem_gewinn_bekommt_seine_entlastung() {
    pruefe(&[
        ("X: A 1990, B 1955, Gewinn B", 1990, 1955, 0, G, MIT_760),
        ("X': A 1955, B 1990, Gewinn A", 1955, 1990, G, 0, MIT_760),
    ]);
}

/// Person A (1955) hat KEINEN Gewinn, der Ehegatte (1990) schon: A bekommt nichts, weil der Gewinn nicht ihr gehoert. Vorher
/// bekam A im Gesamt-Zweig 760 EUR (242 EUR zu niedrig). Spiegel: A (1990) mit dem Gewinn, B (1955) ohne.
#[test]
fn der_gewinn_des_jungen_ehegatten_entlastet_den_alten_nicht() {
    pruefe(&[
        ("Y: A 1955, B 1990, Gewinn B", 1955, 1990, 0, G, OHNE),
        ("Y': A 1990, B 1955, Gewinn A", 1990, 1955, G, 0, OHNE),
    ]);
}

/// Beide 1955, der Gewinn liegt bei B: B bekommt die 760 EUR (nicht A). Die Summe war vorher schon richtig, nur die Person
/// falsch; die Steuer ist hier fuer A und B gleich, der Fall haelt also die Spiegelgleichheit, nicht die Person.
#[test]
fn beide_aelter_der_gewinn_liegt_bei_einem() {
    pruefe(&[
        ("Z: beide 1955, Gewinn B", 1955, 1955, 0, G, MIT_760),
        ("Z': beide 1955, Gewinn A", 1955, 1955, G, 0, MIT_760),
    ]);
}

/// Beide 1955 und beide mit eigenem Gewinn (je 40.000 EUR): jeder Ehegatte bekommt seine 760 EUR, zusammen 1.520 EUR. Eine
/// gemeinsame Bemessung (alles bei A) gaebe nur 760 EUR (= 1.437.600 Cent).
#[test]
fn beide_mit_eigenem_gewinn_bekommen_je_ihre_entlastung() {
    pruefe(&[("V: beide 1955, je 40.000 EUR Gewinn", 1955, 1955, HALB, HALB, MIT_2X760)]);
}

/// Die uebrigen Einkuenfte von Person A bleiben in IHRER Bemessung: 80.000 EUR Vermietung (Gesamt-Zweig; der Rentner-Zweig liest
/// keine Vermietung). A 1955 bekommt 760 EUR (`MIT_760`), A 1990 nichts (`OHNE`); dieselben Zahlen wie beim Gewinn, weil beide
/// Einkunftsarten gleich versteuert werden (Summe 80.000 EUR, Splitting).
#[test]
fn die_vermietung_von_a_zaehlt_in_deren_bemessung() {
    let mit_vermietung = |geb_a: i64| {
        let mut p = basis("zusammen", geb_a, 1990, 0, 0);
        p.push(("vv_einnahmen", json!(G)));
        est("festzusetzende_est_gesamt", &p)
    };
    assert_eq!(mit_vermietung(1955), MIT_760, "A 1955");
    assert_eq!(mit_vermietung(1990), OHNE, "A 1990");
}

/// Der § 16-Veraeusserungsgewinn von Person A (500.000 EUR, ohne Freibetrag) bleibt in IHRER Bemessung. Die Steuer darauf folgt
/// der Fuenftelregel; die Handrechnung dazu steht hier nicht, die Probe ist qualitativ: A 1955 zahlt WENIGER als A 1990 bei
/// gleichem Gewinn, weil sie 760 EUR Altersentlastung bekommt (0,16 · 500.000 = 80.000, gedeckelt). Bei 80.000 EUR waere die
/// Probe blind: die Fuenftelregel gibt dort in beiden Faellen 0 EUR Steuer.
#[test]
fn der_veraeusserungsgewinn_von_a_zaehlt_in_deren_bemessung() {
    for q in QUANTITAETEN {
        let mit_verkauf = |geb_a: i64| {
            let mut p = basis("zusammen", geb_a, 1990, 0, 0);
            p.push(("rentner_veraeusserungsgewinn", json!(VERKAUF)));
            p.push(("rentner_veraeusserungs_betriebsart", json!("gewerbe")));
            p.push(("kein_gewinn", json!(false)));
            est(q, &p)
        };
        let (alt, jung) = (mit_verkauf(1955), mit_verkauf(1990));
        assert!(alt < jung, "{q}: A 1955 muss weniger zahlen als A 1990 ({alt} gegen {jung} Cent)");
    }
}

/// Einzelveranlagung bleibt unveraendert: der Gewinn des Ehegatten und sein Jahrgang zaehlen nirgends, auch nicht fuer A.
#[test]
fn einzelveranlagung_kennt_den_ehegatten_nicht() {
    for q in QUANTITAETEN {
        let ohne_partner = basis("einzel", 1955, 1955, G, 0);
        let mit_partner = basis("einzel", 1955, 1955, G, 5_000_000);
        assert_eq!(est(q, &ohne_partner), EINZEL_1955, "{q}: ohne Partnerwerte");
        assert_eq!(est(q, &mit_partner), EINZEL_1955, "{q}: mit Gewinn und Jahrgang des Partners");
    }
}
