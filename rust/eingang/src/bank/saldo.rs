//! Die Postenmenge eines Felds: vereinigen und den Saldo bilden.
//!
//! Eine Menge ist eine Liste von Posten; ein Posten, der n-mal vorkommt, steht n-mal darin. So
//! steht sie auch in `signal_1` des Ereignisses. Zwei Auszuege koennen denselben Zeitraum
//! decken: derselbe Posten darf dann nicht zweimal zaehlen. Vereinigt wird darum je Schluessel
//! (Datum, Cent-Betrag, maskierter Zweck) mit dem MAXIMUM der Haeufigkeit, nicht mit der Summe.
//! Eine Buchung, die zweimal am selben Tag mit demselben Text und Betrag in EINEM Auszug steht,
//! sind zwei Buchungen und zaehlen zweimal.
use std::collections::BTreeMap;

use super::buchung::Posten;

/// Die vereinigte Menge: je Schluessel das Maximum der Haeufigkeit aus `alt` und `neu`. Die
/// Reihenfolge bleibt erhalten: erst alle Posten aus `alt`, dann die aus `neu`, die `alt` nicht
/// schon in dieser Haeufigkeit hatte.
///
/// ```
/// use eingang::bank::{buchung::{Datum, Posten}, saldo::{saldo, vereinige}};
/// let p = |tag: u32, betrag: i64| Posten {
///     datum: Datum { jahr: 2025, monat: 3, tag },
///     betrag_cent: betrag,
///     zweck_maskiert: "Spende".to_owned(),
/// };
/// let alt = vec![p(1, -400), p(2, -700)];
/// let neu = vec![p(1, -400), p(3, -500)];
/// let zusammen = vereinige(&alt, &neu);
/// assert_eq!(zusammen.len(), 3, "der gemeinsame Posten zaehlt einmal");
/// assert_eq!(saldo(&zusammen), Some(1_600));
/// // Zwei gleiche Posten in EINEM Auszug sind zwei Buchungen.
/// let doppelt = vec![p(1, -400), p(1, -400)];
/// assert_eq!(vereinige(&doppelt, &doppelt).len(), 2);
/// ```
#[must_use]
pub fn vereinige(alt: &[Posten], neu: &[Posten]) -> Vec<Posten> {
    let mut vorhanden: BTreeMap<(String, i64, String), usize> = BTreeMap::new();
    for posten in alt {
        *vorhanden.entry(posten.schluessel()).or_insert(0) += 1;
    }
    let mut ergebnis = alt.to_vec();
    let mut aus_neu: BTreeMap<(String, i64, String), usize> = BTreeMap::new();
    for posten in neu {
        let schluessel = posten.schluessel();
        let n = aus_neu.entry(schluessel.clone()).or_insert(0);
        *n += 1;
        if *n > vorhanden.get(&schluessel).copied().unwrap_or(0) {
            ergebnis.push(posten.clone());
        }
    }
    ergebnis
}

/// Der Saldo in Cent, als Ausgabe positiv: `-Σ betrag_cent`. Eine Rueckbuchung (Betrag > 0)
/// mindert ihn. Ein Ueberlauf ist `None`; die Aufruferin meldet ihn als Fehler, nie als 0.
///
/// ```
/// use eingang::bank::{buchung::{Datum, Posten}, saldo::saldo};
/// let p = |betrag: i64| Posten { datum: Datum { jahr: 2025, monat: 3, tag: 1 }, betrag_cent: betrag, zweck_maskiert: String::new() };
/// assert_eq!(saldo(&[p(-10_000), p(-25_000), p(5_000)]), Some(30_000));
/// assert_eq!(saldo(&[p(i64::MIN)]), None, "i64::MIN hat kein positives Gegenstueck");
/// assert_eq!(saldo(&[p(-i64::MAX), p(-1)]), None);
/// ```
#[must_use]
pub fn saldo(posten: &[Posten]) -> Option<i64> {
    let mut summe: i64 = 0;
    for posten in posten {
        summe = summe.checked_add(posten.betrag_cent)?;
    }
    summe.checked_neg()
}
