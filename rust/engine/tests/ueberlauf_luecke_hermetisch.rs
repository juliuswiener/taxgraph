//! Ueberlaufstellen der Rechenkern-Zugriffe (`engine::zugriff::teil1`, `engine::dezimal`, `engine::zugriff::teil2::rente`), die
//! `zugriff_ueberlauf_hermetisch.rs` und `params_caps_ueberlauf_hermetisch.rs` NICHT erreichen, im Standardlauf (ohne `PARITY=1`,
//! ohne Python). Bericht h8-ueberlauf-luecke.
//!
//! ANLASS: zwei Mutanten von `main`, je ein `checked_*` am Aufrufort durch `wrapping_*` ersetzt, blieben unter `cargo test -p engine`
//! gruen: M1 `reisekosten.rs` (`pauschale_24h * 100`, Marke `vpf p24`) und M2 `werbungskosten.rs` (`wk + gwg`, Marke `wk am`). Die 127
//! Mutanten aus h8-hermetisch5 waren eine Auswahl; ein Schema (alle neuen Fundstellen, je Lauf eine per `MUTSITE` angeschaltet) fand
//! in diesem Crate weitere ueberlebende Stellen. Jede Stelle unten steht mit ihrer Schema-Kennung im Bericht; ein Fall dieser Datei
//! tragt die Stelle im Namen nur ueber die erwartete Marke (`Ueberlauf(<Marke>)`), nicht ueber die Kennung.
//!
//! Die Fehlerart ist Teil der Erwartung: `Ueberlauf` MIT der Marke der Stelle, kein Wert (Wrap-Around), keine andere Fehlerart. Zwei
//! Gruppen stuetzen sich nur auf die Marke: (a) Stellen, deren Wrap-Around am Ende ohnehin eine andere Sperre ausloest (`vpf kf` und
//! `vpf km`: dieselbe Basis mal 20 und mal 40) -- dort ist der Wert, den die Mutante liefert, `Ok(0)` (die Fallbeschreibung nennt ihn);
//! (b) `Satz->Cent` aus der Params-Datei.
//!
//! HERKUNFT DER ERWARTUNGSWERTE: der Python-Wert jedes Falls ist die Ausgabe des Python-Orakels (`runner._verpflegung_abzug`,
//! `_uebernachtung_abzug`, `catala_werbungskosten_n`, `catala_ep_ab_21km`, `catala_vermietung_einkuenfte`,
//! `catala_p3_nr72_photovoltaik`, VZ 2025, leere Wegwerf-Datenwurzel; Params-Faelle patchen `runner._load_yaml_path` im Prozess);
//! Orakel-Skript und Lauf: Anlagen zum Bericht. Jeder Fall traegt eine von drei Marken im Namen: "Python-Wert ausserhalb i64" (es gibt
//! keinen `i64`-Wert, Rust MUSS `Ueberlauf` melden), "Zwischenprodukt ausserhalb i64" (Python liefert einen gueltigen Wert, Rust
//! meldet `Ueberlauf`, weil ein Zwischenschritt nicht passt: die dokumentierte fail-closed-Konvention, KEINE Python-Stuetze) und
//! "passt gerade noch" (alles passt, Rust MUSS den Python-Wert liefern).
//!
//! ERREICHBARKEIT: diese Stellen sind ueber die `pub`-Schnittstelle des Crates erreichbar. Ueber die HTTP-Haut erreicht sie kein
//! Fall, ausser wo der Bericht es nennt (Eingangsgrenzen: `bindung_n_vor_gwg.yaml` -- Tage und Monate haben `bereich`, die Haut
//! baut die doppelte Haushaltsfuehrung nur mit `im_inland: true`, also gekappt, und klemmt Mahlzeiten, Entgelt und Erstattung auf
//! mindestens 0). Params-Faelle sind erreichbar nur mit gepatchter Parameterdatei (Schwaeche des Harness: die Datei ist ein Datensatz, kein
//! Quelltext-Mutant -- der Test weist nach, dass die Pruefung AN DIESER STELLE wirkt, nicht dass der echte params-Betrieb je dorthin
//! kommt).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::disallowed_types
)]

use std::path::Path;

use bindung::Params;
use domain::{Cent, Euro, Km, Vz};
use engine::dezimal::zu_bruch;
use engine::zugriff::teil1::einkuenfte::{
    p3_nr72_photovoltaik, vermietung_einkuenfte, P3Nr72PhotovoltaikEingabe,
    VermietungEinkuenfteEingabe,
};
use engine::zugriff::teil1::fehler::EngineFehler;
use engine::zugriff::teil1::reisekosten::{DhfEingabe, UebernachtungEingabe, VerpflegungEingabe};
use engine::zugriff::teil1::werbungskosten::{
    ep_ab_21km, werbungskosten_n, EntfernungspauschaleEingabe, WerbungskostenNEingabe,
};
use engine::zugriff::teil2::rente::{renten_einkuenfte, RentenEingabe, Rentenart};
use engine::zugriff::teil2::EngineFehler as Teil2Fehler;
use rust_decimal::Decimal;

/// `Some(marke)`: Rust MUSS `EngineFehler::Ueberlauf(marke)` melden. `None`: Rust MUSS den Wert `python` liefern.
type Marke = Option<&'static str>;

const MAX: i64 = i64::MAX;
const MIN: i64 = i64::MIN;

fn params() -> Params {
    Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn kopiere_params(from: &Path, nach: &Path) {
    std::fs::create_dir_all(nach).unwrap();
    for eintrag in std::fs::read_dir(from).unwrap() {
        let eintrag = eintrag.unwrap();
        let ziel = nach.join(eintrag.file_name());
        if eintrag.file_type().unwrap().is_dir() {
            kopiere_params(&eintrag.path(), &ziel);
        } else {
            std::fs::copy(eintrag.path(), &ziel).unwrap();
        }
    }
}

/// `Params` aus einer Temp-Kopie von `params/`, in der EINE Datei an genau den gegebenen Stellen ersetzt ist. Jeder Anker muss in
/// der Originaldatei genau einmal treffen -- sonst ist der Test gegen eine geaenderte Dateiformatierung taub, und das soll er laut sein.
fn params_gepatcht(zelle: &str, datei: &str, stellen: &[(&str, &str)]) -> Params {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let wurzel =
        std::env::temp_dir().join(format!("taxgraph-h8-luecke-{zelle}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&wurzel);
    kopiere_params(&repo.join("params"), &wurzel.join("params"));
    let pfad = wurzel.join("params").join(datei);
    let mut text = std::fs::read_to_string(&pfad).unwrap();
    for (alt, neu) in stellen {
        assert_eq!(
            text.matches(*alt).count(),
            1,
            "Anker nicht einwandfrei in {datei}: {alt:?}"
        );
        text = text.replacen(alt, neu, 1);
    }
    std::fs::write(&pfad, text).unwrap();
    let p = Params::lade(&wurzel).unwrap();
    std::fs::remove_dir_all(&wurzel).unwrap();
    p
}

/// Nur ein Zweig von `werbungskosten_n`; alle anderen Schluessel fehlen.
fn nur() -> WerbungskostenNEingabe {
    WerbungskostenNEingabe {
        entfernung: None,
        doppelte_haushaltsfuehrung: None,
        verpflegung: None,
        uebernachtung: None,
        am_anschaffungskosten: None,
    }
}

/// `[24-h-Tage, An-/Abreisetage, Tage ueber 8 h, je Kategorie nach der Dreimonatsfrist (3x), Fruehstuecke, Mittagessen, Abendessen]`,
/// Entgelt, Erstattung (Cent).
fn vpf(t: [i64; 9], entgelt: i64, erstattung: i64) -> VerpflegungEingabe {
    VerpflegungEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        tage_24h: t[0],
        tage_an_abreise: t[1],
        tage_ueber_8h_eintaegig: t[2],
        vpf_tage_24h_nach_drei_monaten: t[3],
        vpf_tage_an_abreise_nach_drei_monaten: t[4],
        vpf_tage_ueber_8h_nach_drei_monaten: t[5],
        vpf_fruehstuecke_gestellt_anzahl: t[6],
        vpf_mittagessen_gestellt_anzahl: t[7],
        vpf_abendessen_gestellt_anzahl: t[8],
        vpf_mahlzeiten_gezahltes_entgelt: Cent::new(entgelt),
        vpf_steuerfreie_erstattung_betrag: Cent::new(erstattung),
    }
}

/// Meldet den Fall, wenn Rust nicht tut, was das Orakel verlangt: bei `None` genau den Python-Wert, bei `Some(marke)` genau
/// `Ueberlauf(marke)` -- weder einen Wert (Wrap-Around) noch eine andere Fehlerart.
fn abweichung(
    name: &str,
    got: &Result<Euro, EngineFehler>,
    python: i128,
    marke: Marke,
) -> Option<String> {
    let passt = match (got, marke) {
        (Ok(e), None) => i128::from(e.get()) == python,
        (Err(EngineFehler::Ueberlauf(m)), Some(soll)) => *m == soll,
        _ => false,
    };
    (!passt).then(|| {
        let soll = marke.map_or_else(
            || "den Python-Wert".to_owned(),
            |m| format!("Ueberlauf({m:?})"),
        );
        format!("{name}: Rust {got:?}, Python {python}, erwartet {soll}")
    })
}

/// Alle Faelle durchlaufen, dann melden: unter einer Mutation zeigt die Meldung jeden roten Fall.
fn melde(abweichend: &[String], faelle: usize) {
    assert!(
        abweichend.is_empty(),
        "{} von {faelle} Faellen weichen vom Python-Orakel ab: {abweichend:#?}",
        abweichend.len()
    );
}

/// `(Name, Eingabe, Python-Wert, Marke)`.
type VpfFall = (&'static str, [i64; 9], i64, i64, i128, Marke);

/// Verpflegung am Aufrufort, echte Params (28 / 14 / 14 EUR; 560 ct je Fruehstueck, 1120 ct je Mittag-/Abendessen).
/// Spalten: Name, Tage und Mahlzeiten, Entgelt, Erstattung, Python, Marke.
const VPF: &[VpfFall] = &[
    ("24-h-Tage = i64::MAX/28 (x2800 passt nicht) [Zwischenprodukt ausserhalb i64]", [329_406_144_173_384_850, 0, 0, 0, 0, 0, 0, 0, 0], 0, 0, 9_223_372_036_854_775_800, Some("vpf s24")),
    ("24-h-Tage 3294061441733848 + An-/Abreisetage 1 [passt gerade noch]", [3_294_061_441_733_848, 1, 0, 0, 0, 0, 0, 0, 0], 0, 0, 92_233_720_368_547_758, None),
    ("24-h-Tage 3294061441733848 + An-/Abreisetage 2 [Zwischenprodukt ausserhalb i64]", [3_294_061_441_733_848, 2, 0, 0, 0, 0, 0, 0, 0], 0, 0, 92_233_720_368_547_772, Some("vpf gesamt")),
    ("24-h-Tage 3294061441733848 + An-/Abreisetage 1 + Tage ueber 8 h 1 [Zwischenprodukt ausserhalb i64]", [3_294_061_441_733_848, 1, 1, 0, 0, 0, 0, 0, 0], 0, 0, 92_233_720_368_547_772, Some("vpf gesamt")),
    ("Fruehstuecke 16470307208669242 (x560 passt) [passt gerade noch]", [0, 0, 0, 0, 0, 0, 16_470_307_208_669_242, 0, 0], 0, 0, 0, None),
    ("Fruehstuecke 16470307208669242 + Mittagessen 1 [Zwischenprodukt ausserhalb i64]", [0, 0, 0, 0, 0, 0, 16_470_307_208_669_242, 1, 0], 0, 0, 0, Some("vpf k28")),
    ("Fruehstuecke 16470307208669242 + Mittagessen 0 [passt gerade noch]", [0, 0, 0, 0, 0, 0, 16_470_307_208_669_242, 0, 0], 0, 0, 0, None),
    ("Fruehstuecke 16470307208669242 + Abendessen 1 [Zwischenprodukt ausserhalb i64]", [0, 0, 0, 0, 0, 0, 16_470_307_208_669_242, 0, 1], 0, 0, 0, Some("vpf k28")),
    ("1 Tag 24 h, Entgelt i64::MIN [Zwischenprodukt ausserhalb i64]", [1, 0, 0, 0, 0, 0, 0, 0, 0], MIN, 0, 0, Some("vpf entgelt")),
    ("1 Tag 24 h, Erstattung i64::MIN [Zwischenprodukt ausserhalb i64]", [1, 0, 0, 0, 0, 0, 0, 0, 0], 0, MIN, 92_233_720_368_547_786, Some("vpf ergebnis")),
    ("1 Tag 24 h, Erstattung -1 [passt gerade noch]", [1, 0, 0, 0, 0, 0, 0, 0, 0], 0, -1, 28, None),
];

/// Die Mahlzeiten-Summe und die Summe der Pauschalen: `f + m + a` (zwei Stellen) und `s24 + sa + s8` (zwei Stellen); dazu die
/// Stellen, die nur ueber die negative Cent-Seite (`Cent` ist vorzeichenbehaftet; die Haut klemmt auf >= 0) zu erreichen sind.
#[test]
fn verpflegung_summen_und_cent_seite_melden_ueberlauf() {
    let p = params();
    let abweichend: Vec<String> = VPF
        .iter()
        .filter_map(|&(name, t, entgelt, erstattung, python, marke)| {
            let e = WerbungskostenNEingabe {
                verpflegung: Some(vpf(t, entgelt, erstattung)),
                ..nur()
            };
            abweichung(name, &werbungskosten_n(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, VPF.len());
}

/// Fall mit gepatchter Verpflegungs-Params: `(Name, Zelle, Stellen, Tage und Mahlzeiten, Python, Marke)`.
type VpfParamsFall = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
    [i64; 9],
    i128,
    Marke,
);

const PAUSCHALE_ANKER: &str = "pauschale_24h:\n  wert: 28\n";
const MITTAG_ANKER: &str = "kuerzung_mittag_abend_prozent:\n  wert: 40\n";

/// M1 (`pauschale_24h * 100`, Marke `vpf p24`) und die beiden Kuerzungsstellen (`p24_cent * 20` bzw. `* 40`). Die Pauschale kommt aus
/// `params/2025/verpflegung_p9_4a.yaml` (28 EUR): ohne gepatchte Datei bleibt `* 100` bei 2.800 und der Wert erreicht keine der Stellen.
/// Bei Tagen > 0 faengt `mul3` (`Tage * Pauschale * 100`) den Riesenwert zuerst -- M1 sitzt erst bei Tagen = 0 allein.
///
/// M1 mit Wert: `pauschale_24h` = 184467440737095517 EUR, `* 100` = 2^64 + 84. Gewrappt bleiben 84 ct; mit einem Fruehstueck und
/// einem An-/Abreisetag rechnet die Mutante die Kuerzung 84 * 20 / 100 = 16 ct statt der Sperre: Abzug 1400 - 16 = 1384 ct = 13 EUR,
/// Python rechnet die Kuerzung auf 1400 ct (Deckel) und liefert 0 EUR.
const VPF_PARAMS: &[VpfParamsFall] = &[
    ("M1: pauschale_24h 184467440737095517 EUR (x100 = 2^64 + 84), 1 Fruehstueck, 1 An-/Abreisetag, Mutante liefert 13 [Zwischenprodukt ausserhalb i64]", "m1a", &[(PAUSCHALE_ANKER, "pauschale_24h:\n  wert: 184467440737095517\n")], [0, 1, 0, 0, 0, 0, 1, 0, 0], 0, Some("vpf p24")),
    ("M1: pauschale_24h 184467440737095517 EUR, keine Tage [Zwischenprodukt ausserhalb i64]", "m1b", &[(PAUSCHALE_ANKER, "pauschale_24h:\n  wert: 184467440737095517\n")], [0, 0, 0, 0, 0, 0, 0, 0, 0], 0, Some("vpf p24")),
    ("pauschale_24h 2305843009213693 EUR (x100 x40 passt gerade) [passt gerade noch]", "m1c", &[(PAUSCHALE_ANKER, "pauschale_24h:\n  wert: 2305843009213693\n")], [0, 0, 0, 0, 0, 0, 0, 0, 0], 0, None),
    ("Fruehstueck-Kuerzung: pauschale_24h 4611686018427388 EUR (x100 x20 passt nicht), Mittag-/Abend-Kuerzung 1 %, Mutante liefert Ok(0) [Zwischenprodukt ausserhalb i64]", "kf", &[(PAUSCHALE_ANKER, "pauschale_24h:\n  wert: 4611686018427388\n"), (MITTAG_ANKER, "kuerzung_mittag_abend_prozent:\n  wert: 1\n")], [0, 0, 0, 0, 0, 0, 0, 0, 0], 0, Some("vpf kf")),
    ("Mittag-/Abend-Kuerzung: pauschale_24h 2305843009213694 EUR (x100 x20 passt, x40 nicht), Mutante liefert Ok(0) [Zwischenprodukt ausserhalb i64]", "km", &[(PAUSCHALE_ANKER, "pauschale_24h:\n  wert: 2305843009213694\n")], [0, 0, 0, 0, 0, 0, 0, 0, 0], 0, Some("vpf km")),
];

#[test]
fn verpflegung_pauschale_aus_params_meldet_ueberlauf() {
    let abweichend: Vec<String> = VPF_PARAMS
        .iter()
        .filter_map(|&(name, zelle, stellen, t, python, marke)| {
            let p = params_gepatcht(zelle, "2025/verpflegung_p9_4a.yaml", stellen);
            let e = WerbungskostenNEingabe {
                verpflegung: Some(vpf(t, 0, 0)),
                ..nur()
            };
            abweichung(name, &werbungskosten_n(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, VPF_PARAMS.len());
}

/// `(Name, Kosten je Monat EUR, Monate, Monate bisher, Python-Wert, Marke)`, VZ 2025, Ausland (ohne Grenze: `params/2025` fuehrt
/// keine Auslandsgrenze).
const UEN: &[(&str, i64, i64, i64, i128, Marke)] = &[
    ("Kosten i64::MAX, 2 Monate, bisher 48 (vor der Schwelle 0 Monate) [Python-Wert ausserhalb i64]", MAX, 2, 48, 18_446_744_073_709_551_614, Some("uebernachtung")),
    ("Kosten 200000000000000000, 48 Monate, bisher 24 (je 24 Monate vor und nach der Schwelle) [Python-Wert ausserhalb i64]", 200_000_000_000_000_000, 48, 24, 9_600_000_000_000_000_000, Some("uebernachtung")),
    ("Kosten 192153584101141162, 48 Monate, bisher 24 [passt gerade noch]", 192_153_584_101_141_162, 48, 24, 9_223_372_036_854_775_776, None),
];

/// § 9 Abs. 1 S. 3 Nr. 5a: der gekappte Teil nach der 48-Monats-Schwelle (`gekappt * nach_48`) und die Summe beider Teile.
#[test]
fn uebernachtung_nach_der_schwelle_meldet_ueberlauf() {
    let p = params();
    let abweichend: Vec<String> = UEN
        .iter()
        .filter_map(|&(name, kosten, monate, bisher, python, marke)| {
            let e = WerbungskostenNEingabe {
                uebernachtung: Some(UebernachtungEingabe {
                    veranlagungszeitraum: Vz::Vz2025,
                    uebernachtung_kosten_monat: Euro::new(kosten),
                    uebernachtung_monate: monate,
                    uebernachtung_monate_bisher: bisher,
                    uebernachtung_im_inland: false,
                }),
                ..nur()
            };
            abweichung(name, &werbungskosten_n(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, UEN.len());
}

/// Die dHf mit 768614336404564650 EUR * 12 Monate = `i64::MAX - 7` (Ausland, VZ 2025, ohne Grenze) als Grundlast jedes Summenfalls.
fn dhf_fast_max() -> DhfEingabe {
    DhfEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        unterkunftskosten_monat: Euro::new(768_614_336_404_564_650),
        monate: 12,
        im_inland: false,
    }
}

/// Zweig, der zur Grundlast hinzukommt.
#[derive(Clone, Copy)]
enum Zweig {
    Keiner,
    Entfernung,
    Verpflegung24h,
    Arbeitsmittel(i64),
}

/// M2 (`wk + gwg`, Marke `wk am`) und die beiden Nachbarn `wk + dhf` (Marke `wk dhf`) und `wk + vpf` (Marke `wk vpf`). Die Grundlast
/// `i64::MAX - 7` ist erreichbar nur ueber die `pub`-Schnittstelle: die Haut baut die dHf mit `im_inland: true` (Kappung 1.000 EUR je
/// Monat, 12.000 EUR im Jahr), und jede andere Summe (Entfernung, Verpflegung, Uebernachtung) ist als Euro-Betrag hoechstens
/// `i64::MAX / 100 * 12` -- die Summe aller Zweige bleibt ueber HTTP weit unter `i64::MAX - 800` (Beleg im Bericht).
const WK: &[(&str, Zweig, i128, Marke)] = &[
    ("dHf i64::MAX-7 allein [passt gerade noch]", Zweig::Keiner, 9_223_372_036_854_775_800, None),
    ("dHf i64::MAX-7 + Entfernungspauschale 30 km, 220 Tage [Python-Wert ausserhalb i64]", Zweig::Entfernung, 9_223_372_036_854_777_956, Some("wk dhf")),
    ("dHf i64::MAX-7 + Verpflegung 1 Tag 24 h [Python-Wert ausserhalb i64]", Zweig::Verpflegung24h, 9_223_372_036_854_775_828, Some("wk vpf")),
    ("M2: dHf i64::MAX-7 + Arbeitsmittel 8 EUR, Mutante liefert i64::MIN [Python-Wert ausserhalb i64]", Zweig::Arbeitsmittel(8), 9_223_372_036_854_775_808, Some("wk am")),
    ("dHf i64::MAX-7 + Arbeitsmittel 7 EUR [passt gerade noch]", Zweig::Arbeitsmittel(7), 9_223_372_036_854_775_807, None),
];

#[test]
fn werbungskosten_summe_meldet_ueberlauf_je_zweig() {
    let p = params();
    let abweichend: Vec<String> = WK
        .iter()
        .filter_map(|&(name, zweig, python, marke)| {
            let mut e = WerbungskostenNEingabe {
                doppelte_haushaltsfuehrung: Some(dhf_fast_max()),
                ..nur()
            };
            match zweig {
                Zweig::Keiner => {}
                Zweig::Entfernung => {
                    e.entfernung = Some(EntfernungspauschaleEingabe {
                        veranlagungszeitraum: Vz::Vz2025,
                        entfernung_km_roh: Km::new(Decimal::from(30)),
                        arbeitstage: 220,
                        eigenes_oder_ueberlassenes_kfz: false,
                        oepnv_kosten_jahr: Euro::new(0),
                    });
                }
                Zweig::Verpflegung24h => {
                    e.verpflegung = Some(vpf([1, 0, 0, 0, 0, 0, 0, 0, 0], 0, 0));
                }
                Zweig::Arbeitsmittel(euro) => e.am_anschaffungskosten = Some(Euro::new(euro)),
            }
            abweichung(name, &werbungskosten_n(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, WK.len());
}

/// `(Name, km, Arbeitstage, Python-Wert, Marke)`, VZ 2025, ohne Kfz, ohne `OePNV`.
const AB21: &[(&str, i64, i64, i128, Marke)] = &[
    ("30 km, 30000000000000000 Arbeitstage (x10 km x38 ct passt nicht) [Zwischenprodukt ausserhalb i64]", 30, 30_000_000_000_000_000, 0, Some("ab21_roh")),
    ("20 km, 922337203685477581 Arbeitstage (x20 = 2^64 + 4, Mutante liefert Ok(0)) [Zwischenprodukt ausserhalb i64]", 20, 922_337_203_685_477_581, 0, Some("ep_bis20")),
    ("20 km, 16000000000000000 Arbeitstage (x20 passt, x30 ct nicht) [Zwischenprodukt ausserhalb i64]", 20, 16_000_000_000_000_000, 0, Some("ep_bis20")),
    ("20 km, 15372286728091293 Arbeitstage (x20 x30 ct passt gerade) [passt gerade noch]", 20, 15_372_286_728_091_293, 0, None),
    ("km -9223372036854775807, 1 Arbeitstag (km - 20 passt nicht) [Zwischenprodukt ausserhalb i64]", -9_223_372_036_854_775_807, 1, 0, Some("km - grenze")),
];

/// § 9 Abs. 1 S. 3 Nr. 4: `ep_ab_21km`, die drei Zwischenprodukte, die `ep_ab_21km_meldet_ueberlauf` (30 km, Tage = `i64::MAX`) nicht
/// trifft, und `km - grenze` bei einer negativen Entfernung (die Haut verlangt `nicht_negativ`).
#[test]
fn ep_ab_21km_meldet_weitere_ueberlaeufe() {
    let p = params();
    let abweichend: Vec<String> = AB21
        .iter()
        .filter_map(|&(name, km, tage, python, marke)| {
            let e = EntfernungspauschaleEingabe {
                veranlagungszeitraum: Vz::Vz2025,
                entfernung_km_roh: Km::new(Decimal::from(km)),
                arbeitstage: tage,
                eigenes_oder_ueberlassenes_kfz: false,
                oepnv_kosten_jahr: Euro::new(0),
            };
            abweichung(name, &ep_ab_21km(&e, &p), python, marke)
        })
        .collect();
    melde(&abweichend, AB21.len());
}

/// `Satz->Cent` aus der Params-Datei (`satz_cent_abgeschnitten`, nur in `ep_ab_21km`): `satz_ab_21_km` = 1e17 EUR je km. Die Sperre
/// steht VOR jeder Rechnung; die Mutante (`as i64`) rechnet mit dem gewrappten Satz weiter und stoppt erst bei `ab21_roh` -- der
/// Test verlangt die Marke `Satz->Cent` (kein Wert-Unterschied, nur die Marke; ohne Tage bliebe die Marke gleich). Python: 220 Tage,
/// 30 km: `fl(220 * 10 * 1e17)` ist gross, aber `min(.., rest)` liefert 0, ein gueltiger Wert.
#[test]
fn ep_ab_21km_satz_aus_params_meldet_ueberlauf() {
    let p = params_gepatcht(
        "satz",
        "2025/entfernungspauschale.yaml",
        &[(
            "satz_ab_21_km:\n  wert: 0.38\n",
            "satz_ab_21_km:\n  wert: 100000000000000000\n",
        )],
    );
    let e = EntfernungspauschaleEingabe {
        veranlagungszeitraum: Vz::Vz2025,
        entfernung_km_roh: Km::new(Decimal::from(30)),
        arbeitstage: 220,
        eigenes_oder_ueberlassenes_kfz: false,
        oepnv_kosten_jahr: Euro::new(0),
    };
    assert_eq!(
        ep_ab_21km(&e, &p).map_err(|e| format!("{e:?}")),
        Err(format!("{:?}", EngineFehler::Ueberlauf("Satz->Cent")))
    );
}

/// `(Name, Einnahmen, AfA, Schuldzinsen, Erhaltungsaufwand, sonstige, Python-Wert, Marke)`.
type VuvFall = (&'static str, i64, i64, i64, i64, i64, i128, Marke);

/// Die vier Stellen: drei Additionen der Werbungskosten, eine Subtraktion. Mit negativen Einnahmen (Verlust-Fall) schlaegt die
/// Mutante nicht in die Sperre `vuv` um, sondern liefert eine Zahl: `i64::MAX` statt der Python-Zahl -9223372036854775809.
const VUV: &[VuvFall] = &[
    ("Einnahmen -1, AfA i64::MAX, Schuldzinsen 1 [Python-Wert ausserhalb i64]", -1, MAX, 1, 0, 0, -9_223_372_036_854_775_809, Some("vuv")),
    ("Einnahmen -1, AfA i64::MAX-1, Schuldzinsen 1 [passt gerade noch]", -1, MAX - 1, 1, 0, 0, -9_223_372_036_854_775_808, None),
    ("Einnahmen -1, AfA i64::MAX-1, Schuldzinsen 1, Erhaltungsaufwand 1 [Python-Wert ausserhalb i64]", -1, MAX - 1, 1, 1, 0, -9_223_372_036_854_775_809, Some("vuv")),
    ("Einnahmen -1, AfA i64::MAX-1, Schuldzinsen 1, sonstige 1 [Python-Wert ausserhalb i64]", -1, MAX - 1, 1, 0, 1, -9_223_372_036_854_775_809, Some("vuv")),
    ("Einnahmen -2, AfA i64::MAX [Python-Wert ausserhalb i64]", -2, MAX, 0, 0, 0, -9_223_372_036_854_775_809, Some("vuv")),
    ("Einnahmen -1, AfA i64::MAX [passt gerade noch]", -1, MAX, 0, 0, 0, -9_223_372_036_854_775_808, None),
];

/// § 21: Einkuenfte aus Vermietung und Verpachtung, Summe der Werbungskosten und Differenz zu den Einnahmen.
#[test]
fn vermietung_summe_und_differenz_melden_ueberlauf() {
    let abweichend: Vec<String> = VUV
        .iter()
        .filter_map(
            |&(name, einnahmen, afa, zinsen, erhaltung, sonstige, python, marke)| {
                let e = VermietungEinkuenfteEingabe {
                    einnahmen: Euro::new(einnahmen),
                    gebaeude_afa: Euro::new(afa),
                    schuldzinsen: Euro::new(zinsen),
                    erhaltungsaufwand: Euro::new(erhaltung),
                    sonstige_werbungskosten: Euro::new(sonstige),
                };
                abweichung(name, &vermietung_einkuenfte(&e), python, marke)
            },
        )
        .collect();
    melde(&abweichend, VUV.len());
}

/// § 3 Nr. 72 (Photovoltaik): `Einheiten * 30`. Python liefert die steuerfreien 900 EUR (30 * Einheiten ist unbegrenzt); die Mutante
/// rechnet mit dem gewrappten Produkt (negativ), `1 kWp > negativ` und liefert 0 -- Rust meldet die Sperre.
#[test]
fn photovoltaik_einheiten_mal_30_meldet_ueberlauf() {
    let rechne = |einheiten: i64| {
        p3_nr72_photovoltaik(&P3Nr72PhotovoltaikEingabe {
            pv_einnahmen: Euro::new(900),
            pv_auf_gebaeude: true,
            pv_bruttoleistung_kwp: 1,
            pv_anzahl_einheiten: einheiten,
        })
    };
    // 1 kWp, 307445734561825861 Einheiten (x30 > i64::MAX): Python 900 [Zwischenprodukt ausserhalb i64]
    let n = MAX / 30 + 1;
    let sperre = rechne(n);
    assert!(
        matches!(sperre, Err(EngineFehler::Ueberlauf("pv"))),
        "{n} Einheiten: {sperre:?}"
    );
    // Gegenprobe: 307445734561825860 Einheiten (x30 passt gerade): Python 900 [passt gerade noch]
    let passt = rechne(MAX / 30);
    assert!(
        matches!(passt, Ok(e) if e == Euro::new(900)),
        "{} Einheiten: {passt:?}",
        MAX / 30
    );
}

/// `zu_bruch` (`catala_new_frac`-Eingabe): Mantisse ausserhalb `i64`, Zehnerpotenz ausserhalb `i64`. Der Wert 10^20 ist der Fall,
/// den die Mutante nicht anderweitig faengt: `10^20 mod 2^64` ist 7766279631452241920 und passt als positives `i64`; ein Exponent 19
/// (10^19 mod 2^64 negativ) fiele der folgenden `u64::try_from`-Stelle auf. Ein Python-Orakel gibt es nicht (`catala_new_frac` ist
/// C-seitig); der Erwartungswert ist die dokumentierte Sperre (`DezimalUeberlauf`).
#[test]
fn zu_bruch_meldet_mantisse_und_nenner_ausserhalb_i64() {
    let gross = Decimal::from_i128_with_scale(i128::from(MAX) + 1, 0);
    assert!(zu_bruch(gross).is_err(), "Mantisse i64::MAX + 1");
    let klein = Decimal::from_i128_with_scale(i128::from(MIN) - 1, 0);
    assert!(zu_bruch(klein).is_err(), "Mantisse i64::MIN - 1");
    assert_eq!(
        zu_bruch(Decimal::from_i128_with_scale(i128::from(MAX), 0)).unwrap(),
        (MAX, 1),
        "Mantisse i64::MAX passt gerade"
    );
    assert!(zu_bruch(Decimal::new(1, 19)).is_err(), "Nenner 10^19");
    let zwanzig = Decimal::new(1, 20);
    assert!(
        zu_bruch(zwanzig).is_err(),
        "Nenner 10^20 (gewrappt 7766279631452241920)"
    );
    assert_eq!(
        zu_bruch(Decimal::new(1, 18)).unwrap(),
        (1, 1_000_000_000_000_000_000),
        "Nenner 10^18 passt gerade"
    );
}

/// `zehntel` in `teil2/rente.rs`: `prozent * 10` ausserhalb `Decimal` (Satz 1e28 -> 1e29 > 7,9e28). Die Marke ist dieselbe wie bei der
/// `i64`-Stelle (`teil2 i64`); die Mutante (`*` statt `checked_mul`) PANIKT an der Stelle -- rot ist hier ein Absturz statt der Sperre.
#[test]
fn rente_prozent_mal_zehn_ausserhalb_decimal_meldet_ueberlauf() {
    let p = params_gepatcht(
        "rente",
        "kohorten/rente_ertragsanteil_p22.yaml",
        &[(
            "  0: {ertragsanteil_prozent: 59.0}\n",
            "  0: {ertragsanteil_prozent: 10000000000000000000000000000.0}\n",
        )],
    );
    let e = RentenEingabe {
        vz: Vz::Vz2025,
        art: Rentenart::Bb {
            alter_bei_rentenbeginn: 0,
        },
        jahresrente: Euro::new(1000),
    };
    let r = renten_einkuenfte(&e, &p);
    assert!(
        matches!(
            r,
            Err(Teil2Fehler::Basis(EngineFehler::Ueberlauf("teil2 i64")))
        ),
        "{r:?}"
    );
}
