//! Gesamt-Regression, Ebene "Bescheid": Faelle mit Rente, Ausland, Unfall, GWG, `AfA` und privatem Verkauf, jeder mit der
//! festzusetzenden Einkommensteuer VON HAND nach Gesetz gerechnet, drei davon als Mischfaelle. Im Standardlauf, ohne
//! `PARITY=1`, ohne Python.
//!
//! **Worum es geht.** `golden/cases` (135 Faelle) hat keinen Fall, der eine Rente aus ihren Regeln rechnet, und keinen mit
//! Auslandseinkuenften, Unfallkosten, GWG, `AfA` oder privatem Verkauf. Die Bloecke dieser Themen haben Einzeltests, aber
//! keinen Ende-zu-Ende-Fall mit bekannter richtiger Steuer. Eine Wechselwirkung (Rente und Ausland im selben Fall) bliebe
//! unbemerkt. Entscheidung `julius-entscheidet-gesamtregression-quelle-handrechnung-mit-zweiter-lesung-2026-10-16`.
//!
//! **Warum es zaehlt.** Die Zahlen unten stammen nicht aus dem Rust-Code. Wer eine Regel aendert, sieht hier, welcher Fall
//! sich um wie viel Euro bewegt. Der Test `jede_kategorie_hat_einen_fall_und_zwei_faelle_verbinden_kategorien` zaehlt die
//! Kategorien aus der Tabelle `faelle()` (jeder Fall RECHNET, die Kategorie steht im Testcode, nicht in einem
//! Beschreibungstext); er beweist nur, dass ein Fall DA ist. Dass seine Zahl stimmt, tragen die Rechenwege unten und die
//! zweite Lesung.
//!
//! ZWEITE LESUNG: Julius (offen). Die Rechenwege stehen unten; eine Leseliste mit Zahl und
//! Gesetzeszitat je Fall liegt dem Uebergabebericht bei. Ohne die Lesung gilt keine Zahl als abgenommen.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: VZ 2025, von Hand aus Gesetzestext und `params/2025/`, mit exakten Bruchzahlen
//! nachgerechnet (Rechenskript im Auftragsordner, kein Zugriff auf den Rust-Code), nie aus dem Rust-Code gelesen.
//! - Tarif § 32a Abs. 1 `EStG`: `params/2025/einkommensteuertarif_p32a.yaml` (Grundfreibetrag 12.096, Zone 3 bis 68.480 mit
//!   (176,64 z + 2.397) z + 1.015,13, z = (zvE − 17.443) / 10.000, Zone 4 bis 277.825: 0,42 x − 10.911,92). Das zvE wird auf
//!   volle Euro abgerundet, der Steuerbetrag auf den naechsten vollen Euro abgerundet (`estg_p32a_2026-07-09.txt`, Absatz 1
//!   Satz 5 und 6). Alle zvE unten sind ganze Euro.
//! - Arbeitnehmer-Pauschbetrag 1.230 Euro (`params/2025/arbeitnehmerpauschbetrag.yaml`, § 9a Satz 1 Nr. 1 Buchstabe a),
//!   Sonderausgaben-Pauschbetrag 36 Euro (`params/2025/sonderausgabenpauschbetrag.yaml`, § 10c), Werbungskosten-Pauschbetrag
//!   fuer Renten 102 Euro (`params/2025/renten_werbungskostenpauschbetrag_p9a.yaml`, § 9a Satz 1 Nr. 3). Keine weiteren
//!   Sonderausgaben und keine aussergewoehnlichen Belastungen: SA-Abzug = 36 Euro.
//! - Rente: Besteuerungsanteil bei Rentenbeginn 2025 = 83,5 % (`params/kohorten/rente_besteuerungsanteil_p22.yaml`,
//!   § 22 Nr. 1 Satz 3 Buchstabe a aa; 2025 ist das Beginnjahr, ein betragsmaessig festgeschriebener Freibetrag gibt es erst
//!   im Folgejahr).
//! - Die Fassung des Gesetzes, nach der gerechnet wurde, ist die eingefrorene "geltende Fassung 2026" unter
//!   `sources/gesetze-im-internet/`; fuer die Zahlen, die hier vorkommen, gilt dasselbe wie fuer 2025 (§ 34c Absatz 1
//!   Satz 2, § 32b Absatz 2, § 23 Absatz 3 Satz 5: 1.000-Euro-Freigrenze, `params/2025/privatverkauf_p23.yaml`).
//!
//! RECHENWEGE (Eingabe, Zwischenwerte, Zahl; gdb = Gesamtbetrag der Einkuenfte, zvE = zu versteuerndes Einkommen):
//!
//! **R1 Rente.** Zweite Lesung: Julius. Rente 30.000 Euro, Beginn 2025, einzeln. Einkuenfte = 83,5 % × 30.000 − 102 = 25.050 − 102 = 24.948 (gdb).
//! zvE = 24.948 − 36 = 24.912. Zone 3: z = 0,7469; (176,64 × 0,7469 + 2.397) × 0,7469 + 1.015,13 = 2.903,9…, abgerundet 2.903.
//!
//! **M1 Rente und Ausland (Freistellung).** Zweite Lesung: Julius. Wie R1, dazu 7.500 Euro Einkuenfte, die ein Abkommen freistellt
//! (§ 32b Abs. 1 Satz 1 Nr. 3, Abs. 2 Satz 1 Nr. 2). Das zvE bleibt 24.912. Besonderer Steuersatz = Steuer(24.912 + 7.500 =
//! 32.412) / 32.412 = 4.998 / 32.412; angewendet auf 24.912: 4.998 × 24.912 / 32.412 = 3.841,4…, abgerundet 3.841. Der Satz ist
//! nicht gerundet (§ 32b nennt keine Rundung des Satzes); auf vier Dezimalstellen abgerundet ergaebe dieselben 3.841 (die 7.500
//! sind so gewaehlt, dass die Lesart keine Rolle spielt; bei 8.000 waeren es 3.894 gegen 3.893).
//!
//! **A1a / A1b Ausland (Anrechnung, § 34c Abs. 1).** Zweite Lesung: Julius. Lohn 93.810 Euro, davon 12.000 Euro Einkuenfte aus den Niederlanden
//! (Methode Anrechnung), einzeln. gdb = 93.810 − 1.230 = 92.580; zvE = 92.580 − 36 = 92.544. Zone 4: 0,42 × 92.544 − 10.911,92 =
//! 27.956,56…, abgerundet 27.956. Auf die auslaendischen Einkuenfte entfaellt der durchschnittliche Steuersatz (§ 34c Abs. 1
//! Satz 2): 27.956 × 12.000 / 92.544 = 3.625 (ganzzahlig; die Zahlen sind so gewaehlt, dass keine Rundungsfrage entsteht).
//! A1a: gezahlt 2.000 < 3.625, angerechnet 2.000, Steuer 27.956 − 2.000 = 25.956. A1b: gezahlt 5.000 > 3.625, angerechnet
//! hoechstens 3.625, Steuer 27.956 − 3.625 = 24.331.
//!
//! **M2a Unfall, Entfernungspauschale und GWG.** Zweite Lesung: Julius. Lohn 62.000 Euro, einzeln. Entfernungspauschale 200 Tage × (20 km × 0,30 +
//! 5 km × 0,38) = 200 × 7,90 = 1.580 (§ 9 Abs. 1 Satz 3 Nr. 4, `params/2025/entfernungspauschale.yaml`; unter dem Hoechstbetrag
//! 4.500, mit Kraftwagen ohnehin ohne Grenze). Unfallkosten 2.100 (BMF-Schreiben Rz. 30, kommen zur Pauschale dazu).
//! Arbeitsmittel mit Anschaffungskosten 780 Euro (ein Arbeitnehmer hat keinen Vorsteuerabzug: es ist der Bruttopreis), Sofortabzug
//! als GWG (§ 9 Abs. 1 Satz 3 Nr. 7 Satz 2 mit § 6 Abs. 2 Satz 1 bis 3: bis 800 Euro). Werbungskosten
//! 1.580 + 2.100 + 780 = 4.460, ueber dem Arbeitnehmer-Pauschbetrag 1.230: gdb = 62.000 − 4.460 = 57.540; zvE = 57.504. Zone 3:
//! 13.452,7…, abgerundet 13.452.
//!
//! **M2b Unfall, Entfernungspauschale und `AfA`.** Zweite Lesung: Julius. Wie M2a, aber das Arbeitsmittel kostet 1.200 Euro (ueber 800 Euro: keine
//! GWG), Nutzungsdauer 3 Jahre, gekauft im Juli 2025. `AfA` nach § 7 Abs. 1: 1.200 / 3 = 400 im Jahr, im Anschaffungsjahr je VOLLEM
//! Monat vor dem Anschaffungsmonat ein Zwoelftel weniger: (13 − 7) / 12 = 6/12, also 200. Werbungskosten 1.580 + 2.100 + 200 =
//! 3.880: gdb = 58.120; zvE = 58.084. Zone 3: 13.674,0…, abgerundet 13.674.
//!
//! **P1 Privatverkauf.** Zweite Lesung: Julius. Lohn 40.000 Euro, einzeln, dazu ein Grundstueck, am 15.03.2021 fuer 100.000 Euro gekauft und am
//! 20.09.2025 fuer 145.000 Euro verkauft (innerhalb von zehn Jahren, § 23 Abs. 1 Satz 1 Nr. 1; nicht selbst bewohnt). Gewinn =
//! 145.000 − 100.000 = 45.000 (§ 23 Abs. 3 Satz 1), ueber der Freigrenze von 1.000 Euro, also steuerpflichtig (Satz 5).
//! gdb = (40.000 − 1.230) + 45.000 = 83.770; zvE = 83.734. Zone 4: 0,42 × 83.734 − 10.911,92 = 24.256,36…, abgerundet 24.256.
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

/// Die Themen der Inventur. Ein Fall nennt in `faelle()` die Themen, die er RECHNET.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kategorie {
    Rente,
    Ausland,
    Unfall,
    Gwg,
    Afa,
    Privatverkauf,
}

const ALLE: [Kategorie; 6] = [
    Kategorie::Rente,
    Kategorie::Ausland,
    Kategorie::Unfall,
    Kategorie::Gwg,
    Kategorie::Afa,
    Kategorie::Privatverkauf,
];

/// Ein Fall: Eingabe, Scheibe und die drei Zahlen von Hand (Euro): Gesamtbetrag der Einkuenfte, zu versteuerndes Einkommen,
/// festzusetzende Einkommensteuer.
struct FallDef {
    name: &'static str,
    kategorien: &'static [Kategorie],
    scheibe: Scheibe,
    eingabe: fn() -> Paare,
    gdb: i64,
    zve: i64,
    est: i64,
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

/// Ein Fall der Scheibe, mit der Scheiben-Bindung (`api._scheibe_bindung`) als Index und Achsen.
struct Fall {
    cfg: Cfg,
    index: HashMap<String, &'static Bindung>,
    achsen: Vec<AchsenBindung>,
    store: Store,
    felder: Felder,
}

fn fall(scheibe: Scheibe, paare: &[(&'static str, Value)]) -> Fall {
    let cfg = Cfg::fuer(scheibe);
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
fn lauf(scheibe: Scheibe, paare: &[(&'static str, Value)]) -> (i64, Kette) {
    let f = fall(scheibe, paare);
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

// ---------------------------------------------------------------- Eingaben

/// Rentnerin im Beginnjahr 2025 mit `euro` Euro gesetzlicher Rente, einzeln veranlagt, alle Kreuze "nein" (die Angaben des
/// Pflicht-Kegels), plus `mehr`.
fn rentner(euro: i64, mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("rentner_renten_art", json!("gesetzliche_rente")),
        ("rentner_jahresrente", json!(cent(euro))),
        ("rentner_renten_beginn_jahr", json!(2025)),
        ("rentner_alter_bei_rentenbeginn", json!(65)),
        ("rentner_rentenfreibetrag", json!(0)),
        ("veranlagung", json!("einzel")),
        ("kein_gewinn", json!(true)),
        ("kein_kap", json!(true)),
        ("kein_vuv", json!(true)),
        ("kein_sonstige", json!(false)),
        ("agb_zwangslaeufig", json!(true)),
        ("agb_notwendig_angemessen", json!(true)),
    ];
    p.extend(mehr);
    p
}

/// Arbeitnehmer mit `euro` Euro Lohn, einzeln veranlagt, plus `mehr`.
fn lohn_einzel(euro: i64, mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("bruttoarbeitslohn", json!(cent(euro))),
        ("veranlagung", json!("einzel")),
    ];
    p.extend(mehr);
    p
}

fn freistellung(euro: i64) -> Paare {
    vec![
        ("dba_auslaendische_einkuenfte", json!(cent(euro))),
        ("dba_methode", json!("dba_freistellung")),
    ]
}

/// Einkuenfte aus den Niederlanden (Methode Anrechnung), `euro` davon im Lohn enthalten, `gezahlt` Euro auslaendische Steuer.
fn anrechnung(euro: i64, gezahlt: i64) -> Paare {
    vec![
        ("dba_staat", json!("nl")),
        ("dba_auslaendische_einkuenfte", json!(cent(euro))),
        ("dba_gezahlte_auslaendische_steuer", json!(cent(gezahlt))),
    ]
}

/// 200 Arbeitstage, 25 km einfache Strecke, eigenes Kraftfahrzeug, 2.100 Euro Unfallkosten auf dem Arbeitsweg.
fn pendler_mit_unfall(mehr: Paare) -> Paare {
    let mut p: Paare = vec![
        ("ep_arbeitstage", json!(200)),
        ("ep_entfernung_km", json!(25)),
        ("ep_oepnv_kosten", json!(0)),
        ("ep_eigenes_kfz", json!(true)),
        ("ep_unfallkosten", json!(cent(2_100))),
    ];
    p.extend(mehr);
    p
}

/// Ein vollstaendiger, steuerpflichtiger privater Verkauf nach § 23: Grundstueck, gekauft `kauf`, verkauft `verkauf`
/// (TT.MM.JJJJ), Preis und Anschaffungskosten in Cent, das Kreuz `kein_p23_verkauf` auf nein.
fn privatverkauf(kauf: &'static str, verkauf: &'static str, preis: i64, kosten: i64) -> Paare {
    vec![
        ("kein_p23_verkauf", json!(false)),
        ("p23_veraeusserungs_typ", json!("grundstueck")),
        ("p23_bezeichnung", json!("Testobjekt")),
        ("p23_anschaffungsdatum", json!(kauf)),
        ("p23_veraeusserungsdatum", json!(verkauf)),
        ("p23_veraeusserungspreis", json!(preis)),
        ("p23_anschaffung_herstellungskosten", json!(kosten)),
        ("p23_werbungskosten", json!(0)),
        ("p23_afa", json!(0)),
        ("p23_eigennutzung", json!("nein")),
        ("p23_gebaeude_in_frist", json!(false)),
    ]
}

// ---------------------------------------------------------------- die Faelle

/// Arbeitsmittel als GWG mit Sofortabzug (Anschaffungskosten 780 Euro, bis 800 Euro).
fn pendler_mit_gwg() -> Paare {
    pendler_mit_unfall(vec![
        ("am_anschaffungskosten", json!(cent(780))),
        ("am_gwg_sofortabzug_gewaehlt", json!(true)),
    ])
}

/// Arbeitsmittel ueber 800 Euro: 1.200 Euro, drei Jahre Nutzungsdauer, gekauft im Juli 2025 (`AfA` im Kaufjahr).
fn pendler_mit_afa() -> Paare {
    pendler_mit_unfall(vec![
        ("am_anschaffungskosten", json!(cent(1_200))),
        ("arbeitsmittel_nutzungsdauer", json!(3)),
        ("am_afa_ist_anschaffungsjahr", json!(true)),
        ("am_anschaffung_monat", json!(7)),
    ])
}

fn faelle() -> Vec<FallDef> {
    use Kategorie::{Afa, Ausland, Gwg, Privatverkauf, Rente, Unfall};
    vec![
        FallDef {
            name: "R1 Rente 30.000, Beginn 2025",
            kategorien: &[Rente],
            scheibe: Scheibe::RentnerGesamt,
            eingabe: || rentner(30_000, vec![]),
            gdb: 24_948,
            zve: 24_912,
            est: 2_903,
        },
        FallDef {
            name: "M1 Rente 30.000 und Auslandseinkuenfte 7.500 freigestellt",
            kategorien: &[Rente, Ausland],
            scheibe: Scheibe::RentnerGesamt,
            eingabe: || rentner(30_000, freistellung(7_500)),
            gdb: 24_948,
            zve: 24_912,
            est: 3_841,
        },
        FallDef {
            name: "A1a Anrechnung, gezahlte Steuer 2.000 unter dem Hoechstbetrag 3.625",
            kategorien: &[Ausland],
            scheibe: Scheibe::Gesamt,
            eingabe: || lohn_einzel(93_810, anrechnung(12_000, 2_000)),
            gdb: 92_580,
            zve: 92_544,
            est: 25_956,
        },
        FallDef {
            name: "A1b Anrechnung, gezahlte Steuer 5.000 ueber dem Hoechstbetrag 3.625",
            kategorien: &[Ausland],
            scheibe: Scheibe::Gesamt,
            eingabe: || lohn_einzel(93_810, anrechnung(12_000, 5_000)),
            gdb: 92_580,
            zve: 92_544,
            est: 24_331,
        },
        FallDef {
            name: "M2a Unfallkosten, Entfernungspauschale und GWG-Sofortabzug",
            kategorien: &[Unfall, Gwg],
            scheibe: Scheibe::Gesamt,
            eingabe: || lohn_einzel(62_000, pendler_mit_gwg()),
            gdb: 57_540,
            zve: 57_504,
            est: 13_452,
        },
        FallDef {
            name: "M2b Unfallkosten, Entfernungspauschale und AfA im Kaufjahr",
            kategorien: &[Unfall, Afa],
            scheibe: Scheibe::Gesamt,
            eingabe: || lohn_einzel(62_000, pendler_mit_afa()),
            gdb: 58_120,
            zve: 58_084,
            est: 13_674,
        },
        FallDef {
            name: "P1 Privatverkauf eines Grundstuecks innerhalb von zehn Jahren",
            kategorien: &[Privatverkauf],
            scheibe: Scheibe::Gesamt,
            eingabe: || {
                lohn_einzel(
                    40_000,
                    privatverkauf("15.03.2021", "20.09.2025", cent(145_000), cent(100_000)),
                )
            },
            gdb: 83_770,
            zve: 83_734,
            est: 24_256,
        },
    ]
}

// ---------------------------------------------------------------- Tests

/// Jeder Fall rechnet auf die Zahlen von Hand: Gesamtbetrag der Einkuenfte, zu versteuerndes Einkommen und Steuer.
#[test]
fn jeder_fall_ergibt_die_von_hand_gerechnete_steuer() {
    let alle = faelle();
    assert!(
        !alle.is_empty(),
        "KONTROLLE: keine Faelle -- eine leere Tabelle ist kein Gruen"
    );
    // Alle Abweichungen sammeln statt beim ersten Fall abzubrechen: ein Fehler in einer Regel zeigt, welche Faelle er trifft.
    let mut abweichungen: Vec<String> = Vec::new();
    for f in &alle {
        let (est, kette) = lauf(f.scheibe, &(f.eingabe)());
        for (was, ist, soll) in [
            (
                "Gesamtbetrag der Einkuenfte",
                kette.gesamtbetrag_der_einkuenfte.get(),
                f.gdb,
            ),
            (
                "zu versteuerndes Einkommen",
                kette.zu_versteuerndes_einkommen.get(),
                f.zve,
            ),
            ("festzusetzende Einkommensteuer", est, f.est),
        ] {
            if ist != soll {
                abweichungen.push(format!("{}: {was} ist {ist}, von Hand {soll}", f.name));
            }
        }
    }
    assert!(
        abweichungen.is_empty(),
        "{} Abweichung(en):\n{}",
        abweichungen.len(),
        abweichungen.join("\n")
    );
}

/// Die Inventur: jede Kategorie hat mindestens einen Fall, und mindestens zwei Faelle verbinden zwei Kategorien. Die Tabelle
/// belegt nur, dass der Fall DA ist; der Test oben belegt, dass er rechnet.
#[test]
fn jede_kategorie_hat_einen_fall_und_zwei_faelle_verbinden_kategorien() {
    let alle = faelle();
    let leer: Vec<Kategorie> = ALLE
        .iter()
        .copied()
        .filter(|k| !alle.iter().any(|f| f.kategorien.contains(k)))
        .collect();
    assert!(
        leer.is_empty(),
        "Kategorien ohne Fall: {leer:?} (Faelle gesamt: {})",
        alle.len()
    );
    let misch = alle.iter().filter(|f| f.kategorien.len() >= 2).count();
    assert!(misch >= 2, "nur {misch} Mischfaelle, verlangt sind zwei");
}
