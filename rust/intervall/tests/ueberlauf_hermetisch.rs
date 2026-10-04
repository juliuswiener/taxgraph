//! Hermetische Ueberlauf-Faelle von `rust/intervall` (h8-hermetisch5): Summand-Addition und Euro nach Cent in
//! `bescheid_via_slots`, Spanne `max - min` und Mittelpunkt-Fixwert in `intervall`, Suchraum-Deckel.
//! Kein Korpus, kein Python zur Laufzeit; der Python-Wert steht je Fall in der Tabelle (`python`) und im Namen steht die Art:
//! - `klar`: der Python-Wert passt nicht in `i64`, Rust MUSS Ueberlauf melden (das Orakel stuetzt es);
//! - `zwischen`: der Python-Wert passt, nur die Zwischensumme nicht, Rust meldet Ueberlauf (nur die fail-closed-Konvention,
//!   `SlotFehler::Ueberlauf`, keine Python-Stuetze);
//! - `knapp`: alles passt gerade noch, Rust MUSS den Python-Wert liefern.
//!
//! Die Werte stammen aus `tools/parity/oracle_konsistenz.antwort` (`intervall.bescheid_via_slots`, `intervall.intervall`,
//! Quelle `produkt/unsicherheit/intervall.py`) mit der synthetischen Engine `_synth` (Gewicht = Summe der Namens-Bytes mod 7 - 3);
//! Erzeuger `orakel_iv5.py`. 11 Faelle `klar`, 1 `zwischen`, 15 `knapp`, dazu der Suchraum-Deckel ohne Orakel
//! (Python zaehlt dort 2^64 Punkte und wird nie fertig).
//!
//! Mutationen, die diese Datei fangen soll (Zeile je Aufrufort in `rust/intervall/src`): `slots.rs` Summe `checked_add` -> `wrapping_add`,
//! Euro->Cent-Fehlerzweig entfernt; `rechnung.rs` Spanne `checked_sub` -> `wrapping_sub`, Mittelpunkt in `i64` statt `i128`,
//! Suchraum `checked_mul` -> `wrapping_mul`.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;
use std::fmt;

use bindung::SlotBeitrag;
use domain::{Cent, Euro, Feldtyp, PyWert};
use intervall::{
    bescheid_via_slots, intervall, AchsenBindung, IntervallFehler, SlotFehler, Spanne, Werte,
};

/// Die synthetische Engine liefert eine Zahl ausserhalb `i64` (kommt in keinem Fall vor, wo Rust sie erreicht).
#[derive(Debug, PartialEq, Eq)]
struct Synth(i128);

impl fmt::Display for Synth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Synth {}", self.0)
    }
}

impl std::error::Error for Synth {}

fn gewicht(name: &str) -> i128 {
    i128::from(name.bytes().map(u64::from).sum::<u64>() % 7) - 3
}

fn zahl(v: &PyWert) -> i128 {
    match v {
        PyWert::Bool(b) => i128::from(*b),
        PyWert::Ganz(n) => i128::from(*n),
        PyWert::Text(s) => i128::try_from(s.chars().count()).unwrap(),
        _ => 0,
    }
}

/// Summe ueber `gewicht(name) * zahl(wert)` wie `_synth`, in `i128` gerechnet.
fn synth<'a>(paare: impl Iterator<Item = (&'a str, &'a PyWert)>) -> Result<i64, Synth> {
    let summe: i128 = paare.map(|(k, v)| gewicht(k) * zahl(v)).sum();
    i64::try_from(summe).map_err(|_| Synth(summe))
}

fn achse(
    id: &str,
    slot: Option<&str>,
    summand: bool,
    bereich: Option<(i64, i64)>,
) -> AchsenBindung {
    AchsenBindung {
        feld_id: id.to_owned(),
        typ: Feldtyp::Cent,
        askable: true,
        enum_werte: Vec::new(),
        bereich,
        signatur_slot: slot.map(str::to_owned),
        slot_beitrag: if summand {
            SlotBeitrag::Summand
        } else {
            SlotBeitrag::Exakt
        },
    }
}

// ------------------------------------------------------------------ bescheid_via_slots

#[derive(Debug, Clone, Copy)]
enum Erw {
    /// Der Python-Wert in Cent.
    Zahl(i64),
    /// `SlotFehler::Ueberlauf(<Slot>)`: die Summe im Slot.
    UeberlaufSlot,
    /// `SlotFehler::Ueberlauf("")`: Euro nach Cent.
    UeberlaufEuro,
}

struct SlotFall {
    name: &'static str,
    /// Quantitaet in Euro (`festzusetzende_est`) statt Cent (`gewst_cent`).
    euro: bool,
    /// (`feld_id`, Slot, Summand?)
    bindung: &'static [(&'static str, &'static str, bool)],
    werte: &'static [(&'static str, i64)],
    python: &'static str,
    erwartet: Erw,
}

/// Slot mit Gewicht +1 bzw. 0 in der synthetischen Engine.
const SLOT_GEWICHT_1: &str = "slot_p6";
const SLOT_GEWICHT_0: &str = "slot_n0";

const SLOT_FAELLE: &[SlotFall] = &[
    SlotFall {
        name: "klar: Summe MAX + 1 (Slot Gewicht 1)",
        euro: false,
        bindung: &[("sum_a", "slot_p6", true), ("sum_b", "slot_p6", true)],
        werte: &[("sum_a", i64::MAX), ("sum_b", 1)],
        python: "9223372036854775808",
        erwartet: Erw::UeberlaufSlot,
    },
    SlotFall {
        name: "knapp: Summe MAX + 0 (Slot Gewicht 1)",
        euro: false,
        bindung: &[("sum_a", "slot_p6", true), ("sum_b", "slot_p6", true)],
        werte: &[("sum_a", i64::MAX), ("sum_b", 0)],
        python: "9223372036854775807",
        erwartet: Erw::Zahl(i64::MAX),
    },
    SlotFall {
        name: "klar: Summe MIN + -1 (Slot Gewicht 1)",
        euro: false,
        bindung: &[("sum_a", "slot_p6", true), ("sum_b", "slot_p6", true)],
        werte: &[("sum_a", i64::MIN), ("sum_b", -1)],
        python: "-9223372036854775809",
        erwartet: Erw::UeberlaufSlot,
    },
    SlotFall {
        name: "knapp: Summe MIN + 0 (Slot Gewicht 1)",
        euro: false,
        bindung: &[("sum_a", "slot_p6", true), ("sum_b", "slot_p6", true)],
        werte: &[("sum_a", i64::MIN), ("sum_b", 0)],
        python: "-9223372036854775808",
        erwartet: Erw::Zahl(i64::MIN),
    },
    SlotFall {
        name: "klar: Summe MAX + MAX (Slot Gewicht 1)",
        euro: false,
        bindung: &[("sum_a", "slot_p6", true), ("sum_b", "slot_p6", true)],
        werte: &[("sum_a", i64::MAX), ("sum_b", i64::MAX)],
        python: "18446744073709551614",
        erwartet: Erw::UeberlaufSlot,
    },
    SlotFall {
        name: "knapp: Summe MAX + MIN (Slot Gewicht 1)",
        euro: false,
        bindung: &[("sum_a", "slot_p6", true), ("sum_b", "slot_p6", true)],
        werte: &[("sum_a", i64::MAX), ("sum_b", i64::MIN)],
        python: "-1",
        erwartet: Erw::Zahl(-1),
    },
    SlotFall {
        name: "zwischen: Summe MAX + 1, Slot Gewicht 0: Wert 0, Zwischensumme ausserhalb",
        euro: false,
        bindung: &[("sum_a", "slot_n0", true), ("sum_b", "slot_n0", true)],
        werte: &[("sum_a", i64::MAX), ("sum_b", 1)],
        python: "0",
        erwartet: Erw::UeberlaufSlot,
    },
    SlotFall {
        name: "knapp: Euro 92233720368547758 (x 100 passt gerade noch)",
        euro: true,
        bindung: &[("wert", "slot_p6", false)],
        werte: &[("wert", 92_233_720_368_547_758)],
        python: "9223372036854775800",
        erwartet: Erw::Zahl(9_223_372_036_854_775_800),
    },
    SlotFall {
        name: "klar: Euro 92233720368547759 (x 100 ausserhalb i64)",
        euro: true,
        bindung: &[("wert", "slot_p6", false)],
        werte: &[("wert", 92_233_720_368_547_759)],
        python: "9223372036854775900",
        erwartet: Erw::UeberlaufEuro,
    },
    SlotFall {
        name: "knapp: Euro -92233720368547758 (x 100 passt gerade noch)",
        euro: true,
        bindung: &[("wert", "slot_p6", false)],
        werte: &[("wert", -92_233_720_368_547_758)],
        python: "-9223372036854775800",
        erwartet: Erw::Zahl(-9_223_372_036_854_775_800),
    },
    SlotFall {
        name: "klar: Euro -92233720368547759 (x 100 ausserhalb i64)",
        euro: true,
        bindung: &[("wert", "slot_p6", false)],
        werte: &[("wert", -92_233_720_368_547_759)],
        python: "-9223372036854775900",
        erwartet: Erw::UeberlaufEuro,
    },
    SlotFall {
        name: "klar: Euro MAX (x 100 ausserhalb i64)",
        euro: true,
        bindung: &[("wert", "slot_p6", false)],
        werte: &[("wert", i64::MAX)],
        python: "922337203685477580700",
        erwartet: Erw::UeberlaufEuro,
    },
    SlotFall {
        name: "knapp: Cent MAX (Cent bleibt Cent)",
        euro: false,
        bindung: &[("wert", "slot_p6", false)],
        werte: &[("wert", i64::MAX)],
        python: "9223372036854775807",
        erwartet: Erw::Zahl(i64::MAX),
    },
    SlotFall {
        name: "klar: Euro-Summanden 5e16 + 5e16 (Summe 1e17, x 100 ausserhalb)",
        euro: true,
        bindung: &[("sum_a", "slot_p6", true), ("sum_b", "slot_p6", true)],
        werte: &[
            ("sum_a", 50_000_000_000_000_000),
            ("sum_b", 50_000_000_000_000_000),
        ],
        python: "10000000000000000000",
        erwartet: Erw::UeberlaufEuro,
    },
];

fn slot_ergebnis(f: &SlotFall) -> Result<Cent, SlotFehler<Synth>> {
    let bindung: Vec<AchsenBindung> = f
        .bindung
        .iter()
        .map(|(id, slot, summand)| achse(id, Some(slot), *summand, None))
        .collect();
    let mut w = Werte::neu();
    for (id, v) in f.werte {
        w.setze(id, PyWert::Ganz(*v));
    }
    let rechne = |s: &intervall::Slots| synth(s.iter().map(|(k, v)| (k.as_str(), v)));
    if f.euro {
        bescheid_via_slots(&bindung, |s| rechne(s).map(Euro::new))(&w)
    } else {
        bescheid_via_slots(&bindung, |s| rechne(s).map(Cent::new))(&w)
    }
}

#[test]
fn slots_summe_und_euro_nach_cent_melden_ueberlauf_und_rechnen_knapp_richtig() {
    assert_eq!(gewicht(SLOT_GEWICHT_1), 1);
    assert_eq!(gewicht(SLOT_GEWICHT_0), 0);
    // Sicherung gegen eine leere Schleife: beide Ausgaenge und alle drei Arten kommen vor.
    assert!(SLOT_FAELLE
        .iter()
        .any(|f| matches!(f.erwartet, Erw::UeberlaufSlot)));
    assert!(SLOT_FAELLE
        .iter()
        .any(|f| matches!(f.erwartet, Erw::UeberlaufEuro)));
    assert!(SLOT_FAELLE
        .iter()
        .any(|f| matches!(f.erwartet, Erw::Zahl(_))));
    let mut falsch = Vec::new();
    for f in SLOT_FAELLE {
        let ist = slot_ergebnis(f);
        let gut = match (f.erwartet, &ist) {
            (Erw::Zahl(z), Ok(c)) => c.get() == z,
            (Erw::UeberlaufSlot, Err(SlotFehler::Ueberlauf(m))) => !m.is_empty(),
            (Erw::UeberlaufEuro, Err(SlotFehler::Ueberlauf(m))) => m.is_empty(),
            _ => false,
        };
        if !gut {
            falsch.push(format!(
                "{}: Rust {ist:?}, Orakel {} (erwartet {:?})",
                f.name, f.python, f.erwartet
            ));
        }
    }
    assert!(
        falsch.is_empty(),
        "{} von {} Faellen weichen ab:\n{}",
        falsch.len(),
        SLOT_FAELLE.len(),
        falsch.join("\n")
    );
}

// ------------------------------------------------------------------ intervall()

#[derive(Debug)]
enum IvErw {
    /// Gesamtintervall und Beitraege (`feld_id`, Spanne, min, max) in der Reihenfolge des Ergebnisses.
    Zahl {
        min: i64,
        max: i64,
        beitraege: &'static [(&'static str, i64, i64, i64)],
    },
    /// `IntervallFehler::Ueberlauf(<feld_id>)`.
    Ueberlauf(&'static str),
}

struct IvFall {
    name: &'static str,
    /// (`feld_id`, bereich min, bereich max); alle askable, `cent`, nicht im Snapshot.
    achsen: &'static [(&'static str, i64, i64)],
    erwartet: IvErw,
}

/// Feld mit Gewicht +1 bzw. 0 in der synthetischen Engine.
const FELD_GEWICHT_1: &str = "feld_p3";
const FELD_GEWICHT_0: &str = "feld_n4";

const SPANNE_FAELLE: &[IvFall] = &[
    IvFall {
        name: "klar: Spanne 2^63 + 1",
        achsen: &[(
            "feld_p3",
            -4_611_686_018_427_387_905,
            4_611_686_018_427_387_904,
        )],
        erwartet: IvErw::Ueberlauf("feld_p3"),
    },
    IvFall {
        name: "knapp: Spanne 2^63 - 1 (= i64::MAX)",
        achsen: &[(
            "feld_p3",
            -4_611_686_018_427_387_904,
            4_611_686_018_427_387_903,
        )],
        erwartet: IvErw::Zahl {
            min: -4_611_686_018_427_387_904,
            max: 4_611_686_018_427_387_903,
            beitraege: &[(
                "feld_p3",
                i64::MAX,
                -4_611_686_018_427_387_904,
                4_611_686_018_427_387_903,
            )],
        },
    },
    IvFall {
        name: "klar: Spanne 2^63, min = i64::MIN",
        achsen: &[("feld_p3", i64::MIN, 0)],
        erwartet: IvErw::Ueberlauf("feld_p3"),
    },
    IvFall {
        name: "knapp: Spanne 2^63 - 1, min = i64::MIN + 1",
        achsen: &[("feld_p3", -9_223_372_036_854_775_807, 0)],
        erwartet: IvErw::Zahl {
            min: -9_223_372_036_854_775_807,
            max: 0,
            beitraege: &[("feld_p3", i64::MAX, -9_223_372_036_854_775_807, 0)],
        },
    },
    IvFall {
        name: "klar: Spanne 2^63, max = i64::MAX",
        achsen: &[("feld_p3", -1, i64::MAX)],
        erwartet: IvErw::Ueberlauf("feld_p3"),
    },
    IvFall {
        name: "knapp: Spanne i64::MAX, min = 0",
        achsen: &[("feld_p3", 0, i64::MAX)],
        erwartet: IvErw::Zahl {
            min: 0,
            max: i64::MAX,
            beitraege: &[("feld_p3", i64::MAX, 0, i64::MAX)],
        },
    },
    IvFall {
        name: "klar: Spanne 2^64 - 1 (MIN bis MAX)",
        achsen: &[("feld_p3", i64::MIN, i64::MAX)],
        erwartet: IvErw::Ueberlauf("feld_p3"),
    },
];

const MITTE_FAELLE: &[IvFall] = &[
    IvFall {
        name: "knapp: Mittelpunkt MAX-2..MAX (Summe ausserhalb i64)",
        achsen: &[
            ("feld_n4", 0, 0),
            ("feld_p3", 9_223_372_036_854_775_805, i64::MAX),
        ],
        erwartet: IvErw::Zahl {
            min: 9_223_372_036_854_775_805,
            max: i64::MAX,
            beitraege: &[
                ("feld_p3", 2, 9_223_372_036_854_775_805, i64::MAX),
                (
                    "feld_n4",
                    0,
                    9_223_372_036_854_775_806,
                    9_223_372_036_854_775_806,
                ),
            ],
        },
    },
    IvFall {
        name: "knapp: Mittelpunkt MAX-1..MAX (Summe ausserhalb i64, abgerundet)",
        achsen: &[
            ("feld_n4", 0, 0),
            ("feld_p3", 9_223_372_036_854_775_806, i64::MAX),
        ],
        erwartet: IvErw::Zahl {
            min: 9_223_372_036_854_775_806,
            max: i64::MAX,
            beitraege: &[
                ("feld_p3", 1, 9_223_372_036_854_775_806, i64::MAX),
                (
                    "feld_n4",
                    0,
                    9_223_372_036_854_775_806,
                    9_223_372_036_854_775_806,
                ),
            ],
        },
    },
    IvFall {
        name: "knapp: Mittelpunkt MIN..MIN+2 (Summe ausserhalb i64)",
        achsen: &[
            ("feld_n4", 0, 0),
            ("feld_p3", i64::MIN, -9_223_372_036_854_775_806),
        ],
        erwartet: IvErw::Zahl {
            min: i64::MIN,
            max: -9_223_372_036_854_775_806,
            beitraege: &[
                ("feld_p3", 2, i64::MIN, -9_223_372_036_854_775_806),
                (
                    "feld_n4",
                    0,
                    -9_223_372_036_854_775_807,
                    -9_223_372_036_854_775_807,
                ),
            ],
        },
    },
    IvFall {
        name: "knapp: Mittelpunkt MIN..MIN+1 (Summe ausserhalb i64, abgerundet)",
        achsen: &[
            ("feld_n4", 0, 0),
            ("feld_p3", i64::MIN, -9_223_372_036_854_775_807),
        ],
        erwartet: IvErw::Zahl {
            min: i64::MIN,
            max: -9_223_372_036_854_775_807,
            beitraege: &[
                ("feld_p3", 1, i64::MIN, -9_223_372_036_854_775_807),
                ("feld_n4", 0, i64::MIN, i64::MIN),
            ],
        },
    },
    IvFall {
        name: "knapp: Mittelpunkt 0..MAX (Summe passt)",
        achsen: &[("feld_n4", 0, 0), ("feld_p3", 0, i64::MAX)],
        erwartet: IvErw::Zahl {
            min: 0,
            max: i64::MAX,
            beitraege: &[
                ("feld_p3", i64::MAX, 0, i64::MAX),
                (
                    "feld_n4",
                    0,
                    4_611_686_018_427_387_903,
                    4_611_686_018_427_387_903,
                ),
            ],
        },
    },
    IvFall {
        name: "knapp: Mittelpunkt MIN+1..0 (Summe passt, abgerundet)",
        achsen: &[
            ("feld_n4", 0, 0),
            ("feld_p3", -9_223_372_036_854_775_807, 0),
        ],
        erwartet: IvErw::Zahl {
            min: -9_223_372_036_854_775_807,
            max: 0,
            beitraege: &[
                ("feld_p3", i64::MAX, -9_223_372_036_854_775_807, 0),
                (
                    "feld_n4",
                    0,
                    -4_611_686_018_427_387_904,
                    -4_611_686_018_427_387_904,
                ),
            ],
        },
    },
];

/// Gesamtmin, Gesamtmax und die Beitraege (`feld_id`, Spanne, min, max).
type IvIst = (i64, i64, Vec<(String, i64, i64, i64)>);

fn iv_ergebnis(f: &IvFall) -> Result<IvIst, IntervallFehler<Synth>> {
    let bindung: Vec<AchsenBindung> = f
        .achsen
        .iter()
        .map(|(id, lo, hi)| achse(id, None, false, Some((*lo, *hi))))
        .collect();
    let r = intervall(
        &BTreeMap::new(),
        &bindung,
        |w| synth(w.iter()).map(Cent::new),
        256,
        Some("sid"),
    )?;
    let Spanne::Zahl { min, max, .. } = r.intervall.spanne else {
        panic!("{}: Spanne NichtFixierbar", f.name)
    };
    let bt = r
        .beitraege
        .iter()
        .map(|b| (b.feld_id.clone(), b.spanne.get(), b.min.get(), b.max.get()))
        .collect();
    Ok((min.get(), max.get(), bt))
}

fn pruefe_iv(faelle: &[IvFall]) {
    assert!(faelle
        .iter()
        .any(|f| matches!(f.erwartet, IvErw::Zahl { .. })));
    let mut falsch = Vec::new();
    for f in faelle {
        let ist = iv_ergebnis(f);
        let gut = match (&f.erwartet, &ist) {
            (
                IvErw::Zahl {
                    min,
                    max,
                    beitraege,
                },
                Ok((imin, imax, ibt)),
            ) => {
                imin == min
                    && imax == max
                    && ibt.len() == beitraege.len()
                    && ibt
                        .iter()
                        .zip(*beitraege)
                        .all(|(a, b)| (a.0.as_str(), a.1, a.2, a.3) == *b)
            }
            (IvErw::Ueberlauf(id), Err(IntervallFehler::Ueberlauf(m))) => m == id,
            _ => false,
        };
        if !gut {
            falsch.push(format!(
                "{}: Rust {ist:?}, erwartet {:?}",
                f.name, f.erwartet
            ));
        }
    }
    assert!(
        falsch.is_empty(),
        "{} von {} Faellen weichen ab:\n{}",
        falsch.len(),
        faelle.len(),
        falsch.join("\n")
    );
}

#[test]
fn intervall_spanne_meldet_ueberlauf_und_rechnet_knapp_richtig() {
    assert!(SPANNE_FAELLE
        .iter()
        .any(|f| matches!(f.erwartet, IvErw::Ueberlauf(_))));
    pruefe_iv(SPANNE_FAELLE);
}

#[test]
fn intervall_fixwert_rechnet_den_mittelpunkt_in_i128() {
    // FELD_GEWICHT_0 hat Gewicht 0 und sieht in `beitraege` den Fixwert von FELD_GEWICHT_1; die Summe `lo + hi` liegt bei vier
    // Faellen ausserhalb `i64`, der Mittelpunkt immer darin.
    assert_eq!(gewicht(FELD_GEWICHT_1), 1);
    assert_eq!(gewicht(FELD_GEWICHT_0), 0);
    pruefe_iv(MITTE_FAELLE);
}

// ------------------------------------------------------------------ Suchraum-Deckel

/// Fehler der Zaehl-Engine: so viele Achsen standen beim ersten Aufruf der Stufe 2 auf ihrem ersten Wert.
#[derive(Debug, PartialEq, Eq)]
struct Gesehen(usize);

impl fmt::Display for Gesehen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} Achsen in der Stufe 2", self.0)
    }
}

impl std::error::Error for Gesehen {}

/// `n` Achsen mit je zwei Werten (0 und 3, Fixwert 1) und unbegrenztem Deckel. Die Engine meldet beim ersten Aufruf der Stufe 2
/// (nach den 2 * n Aufrufen der Stufe 1), wie viele Felder auf 0 stehen: genau die Achsen, die in den Suchraum kamen.
fn achsen_in_stufe2(n: usize) -> Result<usize, IntervallFehler<Gesehen>> {
    let ids: Vec<String> = (0..n).map(|i| format!("f{i:02}")).collect();
    let bindung: Vec<AchsenBindung> = ids
        .iter()
        .map(|id| achse(id, None, false, Some((0, 3))))
        .collect();
    let mut aufrufe = 0_usize;
    let r = intervall(
        &BTreeMap::new(),
        &bindung,
        |w| {
            aufrufe += 1;
            if aufrufe > 2 * n {
                let nullen = w.iter().filter(|(_, v)| **v == PyWert::Ganz(0)).count();
                return Err(Gesehen(nullen));
            }
            Ok(Cent::new(0))
        },
        usize::MAX,
        None,
    );
    match r {
        Err(IntervallFehler::Bescheid(Gesehen(k))) => Ok(k),
        Err(e) => Err(e),
        Ok(_) => panic!("die Zaehl-Engine haette in Stufe 2 scheitern muessen"),
    }
}

#[test]
fn suchraum_ueberlauf_zaehlt_als_ueber_dem_deckel() {
    // art: zwischen. 63 Achsen mit je 2 Werten ergeben 2^63 Punkte: das passt gerade noch in `usize` und unter den Deckel
    // `usize::MAX`, alle 63 kommen in den Suchraum.
    assert_eq!(achsen_in_stufe2(63).unwrap(), 63);
    // Die 64. Achse ergibt 2^64 Punkte, das Produkt laeuft ueber `usize`. Der Deckel gilt dann als ueberschritten, die Achse bleibt
    // draussen. Mit `wrapping_mul` wuerde das Produkt 0 und die 64. Achse kaeme hinein (64 statt 63). Python hat kein Gegenstueck:
    // `raum * n > cap` rechnet dort unbegrenzt und zaehlt die 2^64 Punkte der Stufe 2 nie zu Ende; nur die dokumentierte
    // Konvention (`intervall()`: "Ueberlauf zaehlt als ueber dem Deckel") stuetzt den Wert.
    assert_eq!(achsen_in_stufe2(64).unwrap(), 63);
}
