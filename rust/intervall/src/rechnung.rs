//! `intervall()` (`intervall.py:59-163`).
use std::collections::BTreeMap;

use domain::{Cent, Feldtyp, Zustand};
use serde_json::Value;
use store::SnapshotFeld;

use crate::{AchsenBindung, Werte};

/// Kombinatorik-Deckel: höchstens so viele Punkte im kartesischen Raum (`intervall.py:30`).
pub const CAP_DEFAULT: usize = 256;

/// Die Zahl des Intervalls. Python trägt dafür vier Schlüssel (`min_cent`, `max_cent`,
/// `min_offen`, `max_offen`), von denen nur drei Kombinationen vorkommen — die anderen sind hier
/// unrepräsentierbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spanne {
    /// Ein unbeschränktes Feld ohne Vorschlag: keine Zahl, beide Seiten offen
    /// (`min_cent = max_cent = None`, `min_offen = max_offen = True`).
    NichtFixierbar,
    /// `min ≤ max` über die Top-K-Achsen. `offen`: fixierbare, aber unbeschränkte Achsen existieren
    /// — die wahre Spanne reicht über `[min, max]` hinaus (`intervall.py:161-162`).
    Zahl { min: Cent, max: Cent, offen: bool },
}

/// Der Intervall-Teil des Ergebnisses (`intervall.py:112-114`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Intervall {
    pub spanne: Spanne,
    /// Nicht alle Achsen passten unter den Deckel.
    pub gedeckelt: bool,
    /// Anzahl Achsen, über die exakt aufgezählt wurde.
    pub exakt_bzgl_top_k: usize,
    /// Achsen außerhalb der Top-K, sortiert.
    pub rest_felder: Vec<String>,
    /// Unsichere Felder ohne Extremwerte (nicht fixierbar ODER fixierbar-unbeschränkt), sortiert.
    pub offene_achsen: Vec<String>,
    /// Unbeschränkt und ohne Vorschlag, sortiert.
    pub nicht_fixierbar: Vec<String>,
}

/// One-at-a-time-Beitrag einer Achse (`intervall.py:130`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beitrag {
    pub feld_id: String,
    /// `max − min`, nie negativ.
    pub spanne: Cent,
    pub min: Cent,
    pub max: Cent,
}

/// Ergebnis von [`intervall`] (`intervall.py:115`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervallErgebnis {
    pub basis_snapshot: Option<String>,
    pub intervall: Intervall,
    /// Absteigend nach Spanne, dann `feld_id`.
    pub beitraege: Vec<Beitrag>,
}

/// Fehler aus [`intervall`].
#[derive(Debug, thiserror::Error)]
pub enum IntervallFehler<E: std::error::Error + 'static> {
    /// Eine `enum`-Achse ohne `enum_werte`. Python: `min([])` → `ValueError`.
    #[error("Achse {0} hat keine Extremwerte (enum ohne enum_werte)")]
    LeereAchse(String),
    /// `max − min` passt nicht in `i64` (Python rechnet unbeschränkt).
    #[error("Spanne der Achse {0} läuft über")]
    Ueberlauf(String),
    #[error(transparent)]
    Bescheid(E),
}

/// Extremwerte je Typ; `None` = keine numerische Achse oder unbeschränkt (`intervall.py:59-69`).
fn extremwerte(b: &AchsenBindung) -> Option<Vec<Value>> {
    match b.typ {
        Feldtyp::Bool => Some(vec![Value::Bool(false), Value::Bool(true)]),
        Feldtyp::Enum => Some(b.enum_werte.iter().cloned().map(Value::String).collect()),
        Feldtyp::Cent | Feldtyp::Int => b.bereich.map(|(lo, hi)| {
            // `sorted({min, max})`: bei min == max genau ein Wert.
            let (a, z) = (lo.min(hi), lo.max(hi));
            if a == z {
                vec![a.into()]
            } else {
                vec![a.into(), z.into()]
            }
        }),
        Feldtyp::Datum | Feldtyp::Text => None,
    }
}

/// Fixierwert eines unsicheren Felds: Vorschlag, sonst `bereich`-Mittelpunkt, sonst `None` =
/// nicht fixierbar (`intervall.py:72-81`). Mittelpunkt `(min + max) // 2` rundet gegen −∞.
fn fixwert(feld: Option<&SnapshotFeld>, b: &AchsenBindung) -> Option<Value> {
    if let Some(f) = feld {
        return Some(f.wert.clone());
    }
    match (b.typ, b.bereich) {
        (Feldtyp::Cent | Feldtyp::Int, Some((lo, hi))) => {
            let mitte = (i128::from(lo) + i128::from(hi)).div_euclid(2);
            // Der Mittelpunkt zweier i64 liegt in i64.
            i64::try_from(mitte).ok().map(Value::from)
        }
        _ => None,
    }
}

/// Einteilung der askable Felder (`intervall.py:89-110`).
struct Einteilung {
    basis: Werte,
    achsen: BTreeMap<String, Vec<Value>>,
    offen: Vec<String>,
    nicht_fix: Vec<String>,
}

fn einteilen(felder: &BTreeMap<String, SnapshotFeld>, bindung: &[AchsenBindung]) -> Einteilung {
    let mut e = Einteilung {
        basis: Werte::neu(),
        achsen: BTreeMap::new(),
        offen: Vec::new(),
        nicht_fix: Vec::new(),
    };
    for b in bindung.iter().filter(|b| b.askable) {
        let feld = felder.get(&b.feld_id);
        if let Some(f) = feld.filter(|f| f.zustand == Zustand::Bestaetigt) {
            e.basis.setze(&b.feld_id, f.wert.clone());
            continue;
        }
        let Some(fw) = fixwert(feld, b) else {
            e.nicht_fix.push(b.feld_id.clone());
            e.offen.push(b.feld_id.clone());
            continue;
        };
        e.basis.setze(&b.feld_id, fw);
        match extremwerte(b) {
            None => e.offen.push(b.feld_id.clone()),
            Some(ex) => {
                e.achsen.insert(b.feld_id.clone(), ex);
            }
        }
    }
    e.offen.sort();
    e.nicht_fix.sort();
    e
}

/// Stufe 1: One-at-a-time-Beitrag je Achse, sortiert nach `(-spanne, feld_id)`.
fn stufe1<E, F>(e: &Einteilung, bescheid_fn: &mut F) -> Result<Vec<Beitrag>, IntervallFehler<E>>
where
    E: std::error::Error + 'static,
    F: FnMut(&Werte) -> Result<Cent, E>,
{
    let mut beitraege = Vec::with_capacity(e.achsen.len());
    for (fid, werte) in &e.achsen {
        let mut lo: Option<Cent> = None;
        let mut hi: Option<Cent> = None;
        for v in werte {
            let mut w = e.basis.clone();
            w.setze(fid, v.clone());
            let s = bescheid_fn(&w).map_err(IntervallFehler::Bescheid)?;
            lo = Some(lo.map_or(s, |x| x.min(s)));
            hi = Some(hi.map_or(s, |x| x.max(s)));
        }
        let (Some(min), Some(max)) = (lo, hi) else {
            return Err(IntervallFehler::LeereAchse(fid.clone()));
        };
        let spanne = max
            .checked_sub(min)
            .ok_or_else(|| IntervallFehler::Ueberlauf(fid.clone()))?;
        beitraege.push(Beitrag {
            feld_id: fid.clone(),
            spanne,
            min,
            max,
        });
    }
    beitraege.sort_by(|a, b| {
        b.spanne
            .cmp(&a.spanne)
            .then_with(|| a.feld_id.cmp(&b.feld_id))
    });
    Ok(beitraege)
}

/// Stufe 2: min/max über den kartesischen Raum der Top-Achsen (`itertools.product`-Reihenfolge,
/// erste Achse am langsamsten), übrige Felder auf der Basis.
fn stufe2<E, F>(
    e: &Einteilung,
    top: &[&str],
    bescheid_fn: &mut F,
) -> Result<(Cent, Cent), IntervallFehler<E>>
where
    E: std::error::Error + 'static,
    F: FnMut(&Werte) -> Result<Cent, E>,
{
    let achsen: Vec<&Vec<Value>> = top.iter().filter_map(|fid| e.achsen.get(*fid)).collect();
    let mut index = vec![0_usize; achsen.len()];
    let mut grenzen: Option<(Cent, Cent)> = None;
    loop {
        let mut w = e.basis.clone();
        for ((fid, achse), i) in top.iter().zip(&achsen).zip(&index) {
            if let Some(v) = achse.get(*i) {
                w.setze(fid, v.clone());
            }
        }
        let s = bescheid_fn(&w).map_err(IntervallFehler::Bescheid)?;
        grenzen = Some(grenzen.map_or((s, s), |(lo, hi)| (lo.min(s), hi.max(s))));
        // Zähler hochzählen, letzte Achse am schnellsten.
        let mut stelle = achsen.len();
        loop {
            let Some(s) = stelle.checked_sub(1) else {
                // Alle Stellen übergelaufen: Raum vollständig. Ohne Achsen genau ein Punkt (die
                // Basis) — Python ruft dann `bescheid_fn(base)` (`intervall.py:153-155`).
                return grenzen.ok_or_else(|| IntervallFehler::LeereAchse(String::new()));
            };
            stelle = s;
            let (Some(i), Some(achse)) = (index.get_mut(s), achsen.get(s)) else {
                continue;
            };
            *i += 1;
            if *i < achse.len() {
                break;
            }
            *i = 0;
        }
    }
}

/// [min, max]-Bescheid-Intervall über die unsicheren askable Felder (`intervall.py:84-163`).
///
/// `felder`: materialisierter Snapshot. `bindung`: askable-Kandidaten in Bindungsreihenfolge (die
/// Reihenfolge bestimmt die Einfügereihenfolge der [`Werte`]). `bescheid_fn`: rein und
/// deterministisch, Ergebnis in Cent. `cap`: Deckel für den kartesischen Raum
/// ([`CAP_DEFAULT`]); `raum * n` mit `checked_mul`, Überlauf zählt als „über dem Deckel".
///
/// # Errors
/// [`IntervallFehler`]: leere `enum`-Achse, Spannen-Überlauf, oder der Fehler von `bescheid_fn`.
///
/// ```
/// use domain::{Cent, Feldtyp};
/// use intervall::{intervall, AchsenBindung, Spanne, CAP_DEFAULT};
/// let b = AchsenBindung { feld_id: "tage".into(), typ: Feldtyp::Int, askable: true,
///     enum_werte: vec![], bereich: Some((0, 10)), signatur_slot: Some("tage".into()),
///     slot_beitrag: bindung::SlotBeitrag::Exakt };
/// let r = intervall(&Default::default(), &[b], |w| {
///     Ok::<_, std::convert::Infallible>(Cent::new(w.get("tage").and_then(|v| v.as_i64()).unwrap_or(0) * 100))
/// }, CAP_DEFAULT, None).unwrap();
/// assert_eq!(r.intervall.spanne, Spanne::Zahl { min: Cent::new(0), max: Cent::new(1000), offen: false });
/// ```
pub fn intervall<E, F>(
    felder: &BTreeMap<String, SnapshotFeld>,
    bindung: &[AchsenBindung],
    mut bescheid_fn: F,
    cap: usize,
    snapshot_id: Option<&str>,
) -> Result<IntervallErgebnis, IntervallFehler<E>>
where
    E: std::error::Error + 'static,
    F: FnMut(&Werte) -> Result<Cent, E>,
{
    let e = einteilen(felder, bindung);
    let mut iv = Intervall {
        spanne: Spanne::NichtFixierbar,
        gedeckelt: false,
        exakt_bzgl_top_k: 0,
        rest_felder: Vec::new(),
        offene_achsen: e.offen.clone(),
        nicht_fixierbar: e.nicht_fix.clone(),
    };
    let basis_snapshot = snapshot_id.map(str::to_owned);
    if !e.nicht_fix.is_empty() {
        return Ok(IntervallErgebnis {
            basis_snapshot,
            intervall: iv,
            beitraege: Vec::new(),
        });
    }
    let beitraege = stufe1(&e, &mut bescheid_fn)?;
    let mut top: Vec<&str> = Vec::new();
    let mut raum: usize = 1;
    for b in &beitraege {
        let n = e.achsen.get(&b.feld_id).map_or(0, Vec::len);
        match raum.checked_mul(n) {
            Some(neu) if neu <= cap => {
                top.push(&b.feld_id);
                raum = neu;
            }
            _ => break,
        }
    }
    let (min, max) = stufe2(&e, &top, &mut bescheid_fn)?;
    iv.spanne = Spanne::Zahl {
        min,
        max,
        offen: !e.offen.is_empty(),
    };
    iv.exakt_bzgl_top_k = top.len();
    let mut rest: Vec<String> = beitraege
        .iter()
        .map(|b| b.feld_id.clone())
        .filter(|f| !top.contains(&f.as_str()))
        .collect();
    iv.gedeckelt = !rest.is_empty();
    rest.sort();
    iv.rest_felder = rest;
    Ok(IntervallErgebnis {
        basis_snapshot,
        intervall: iv,
        beitraege,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bindung::SlotBeitrag;
    use domain::{Achsenwert, Herkunft, PruefTiefe};
    use proptest::prelude::*;
    use std::convert::Infallible;

    fn feld(wert: Value, zustand: Zustand) -> SnapshotFeld {
        let a = Achsenwert::new("t").unwrap();
        SnapshotFeld {
            wert,
            zustand,
            herkunft: Herkunft {
                herkunft: a.clone(),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a,
            }
            .into(),
        }
    }

    /// Lineare Modellsteuer: Gewicht je Feld mal Zahlwert (bool 0/1, String Länge).
    #[allow(clippy::unnecessary_wraps)] // Signatur von `intervall()` vorgegeben
    fn modell(w: &Werte) -> Result<Cent, Infallible> {
        let mut s: i64 = 0;
        for (i, (k, v)) in w.iter().enumerate() {
            let n = match v {
                Value::Bool(b) => i64::from(*b),
                Value::String(t) => i64::try_from(t.len()).unwrap(),
                _ => v.as_i64().unwrap_or(0),
            };
            let g = i64::try_from(k.len() % 5).unwrap() - 2 + i64::try_from(i % 2).unwrap();
            s += g * n;
        }
        Ok(Cent::new(s))
    }

    fn fall() -> impl Strategy<Value = (Vec<AchsenBindung>, BTreeMap<String, SnapshotFeld>)> {
        prop::collection::vec(
            (
                0_u8..4,
                -50_i64..50,
                0_i64..50,
                prop::option::of((any::<bool>(), -60_i64..60)),
            ),
            0..7,
        )
        .prop_map(|spec| {
            let mut bindung = Vec::new();
            let mut felder = BTreeMap::new();
            for (i, (art, lo, breite, snap)) in spec.into_iter().enumerate() {
                let fid = format!("f{i}{}", "x".repeat(i % 3));
                let (typ, enum_werte, bereich) = match art {
                    0 => (Feldtyp::Bool, vec![], None),
                    1 => (Feldtyp::Enum, vec!["a".into(), "bbb".into()], None),
                    2 => (Feldtyp::Int, vec![], Some((lo, lo + breite))),
                    _ => (Feldtyp::Text, vec![], None),
                };
                if let Some((bestaetigt, wert)) = snap {
                    let z = if bestaetigt {
                        Zustand::Bestaetigt
                    } else {
                        Zustand::Vorlaeufig
                    };
                    felder.insert(fid.clone(), feld(wert.into(), z));
                }
                bindung.push(AchsenBindung {
                    feld_id: fid,
                    typ,
                    askable: true,
                    enum_werte,
                    bereich,
                    signatur_slot: None,
                    slot_beitrag: SlotBeitrag::Exakt,
                });
            }
            (bindung, felder)
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(2000))]

        #[test]
        fn min_hoechstens_max((bindung, felder) in fall(), cap in 0_usize..300) {
            let r = intervall(&felder, &bindung, modell, cap, None).unwrap();
            if let Spanne::Zahl { min, max, .. } = r.intervall.spanne {
                prop_assert!(min <= max);
            }
            for b in &r.beitraege {
                prop_assert!(b.min <= b.max);
                prop_assert_eq!(b.max.checked_sub(b.min), Some(b.spanne));
            }
        }

        #[test]
        fn alle_achsen_fixiert_ergibt_punkt((bindung, _f) in fall(), wert in -60_i64..60) {
            let felder: BTreeMap<String, SnapshotFeld> =
                bindung.iter().map(|b| (b.feld_id.clone(), feld(wert.into(), Zustand::Bestaetigt))).collect();
            let r = intervall(&felder, &bindung, modell, CAP_DEFAULT, None).unwrap();
            let Spanne::Zahl { min, max, offen } = r.intervall.spanne else { panic!("keine Zahl") };
            prop_assert_eq!(min, max);
            prop_assert!(!offen);
            prop_assert!(r.beitraege.is_empty());
        }

        #[test]
        fn monoton_im_deckel((bindung, felder) in fall(), a in 0_usize..40, b in 0_usize..40) {
            let (klein, gross) = (a.min(b), a.max(b));
            let r1 = intervall(&felder, &bindung, modell, klein, None).unwrap().intervall;
            let r2 = intervall(&felder, &bindung, modell, gross, None).unwrap().intervall;
            prop_assert!(r1.exakt_bzgl_top_k <= r2.exakt_bzgl_top_k);
            prop_assert!(r2.rest_felder.len() <= r1.rest_felder.len());
            let r3 = intervall(&felder, &bindung, modell, usize::MAX, None).unwrap().intervall;
            prop_assert!(!r3.gedeckelt);
        }
    }

    #[test]
    fn nicht_fixierbar_hat_keine_zahl() {
        let b = AchsenBindung {
            feld_id: "t".into(),
            typ: Feldtyp::Text,
            askable: true,
            enum_werte: vec![],
            bereich: None,
            signatur_slot: None,
            slot_beitrag: SlotBeitrag::Exakt,
        };
        let r = intervall(&BTreeMap::new(), &[b], modell, CAP_DEFAULT, Some("sid")).unwrap();
        assert_eq!(r.intervall.spanne, Spanne::NichtFixierbar);
        assert_eq!(r.intervall.nicht_fixierbar, ["t"]);
        assert_eq!(r.basis_snapshot.as_deref(), Some("sid"));
    }

    #[test]
    fn leere_enum_achse_ist_fehler() {
        let b = AchsenBindung {
            feld_id: "e".into(),
            typ: Feldtyp::Enum,
            askable: true,
            enum_werte: vec![],
            bereich: None,
            signatur_slot: None,
            slot_beitrag: SlotBeitrag::Exakt,
        };
        let mut felder = BTreeMap::new();
        felder.insert("e".to_owned(), feld("x".into(), Zustand::Vorlaeufig));
        assert!(matches!(
            intervall(&felder, &[b], modell, CAP_DEFAULT, None),
            Err(IntervallFehler::LeereAchse(_))
        ));
    }
}
