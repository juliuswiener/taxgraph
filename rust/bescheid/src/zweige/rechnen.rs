//! Kleine Rechenhelfer der Zweige: Python-`int`-Arithmetik mit Ueberlauf-Fehler statt Wrap-Around.
use domain::{Cent, Euro};

use crate::BescheidFehler;

pub(super) type R<T> = Result<T, BescheidFehler>;

/// `a + b` (Euro).
pub(super) fn add(a: Euro, b: Euro) -> R<Euro> {
    crate::euro_plus(a, b)
}

/// `a - b` (Euro).
pub(super) fn sub(a: Euro, b: Euro) -> R<Euro> {
    crate::minus(a.get(), b.get()).map(Euro::new)
}

/// `n * a` (Euro mal Ganzzahl).
pub(super) fn mal(n: i64, a: Euro) -> R<Euro> {
    n.checked_mul(a.get())
        .map(Euro::new)
        .ok_or(BescheidFehler::Ueberlauf("Multiplikation"))
}

/// `max(0, a)`.
pub(super) fn max0(a: Euro) -> Euro {
    Euro::new(a.get().max(0))
}

/// Σ ueber Euro-Werte, links nach rechts.
pub(super) fn summe_euro(werte: &[Euro]) -> R<Euro> {
    werte.iter().try_fold(Euro::new(0), |acc, x| add(acc, *x))
}

/// `a * b // c` mit positivem `c` (Python-Floor), in `i128` gerechnet.
pub(super) fn mal_div(a: i64, b: i64, c: i64) -> R<i64> {
    if c == 0 {
        return Err(BescheidFehler::Python {
            klasse: "ZeroDivisionError",
            was: "Division durch null",
        });
    }
    let v = i128::from(a) * i128::from(b);
    i64::try_from(v.div_euclid(i128::from(c))).map_err(|_| BescheidFehler::Ueberlauf("a*b//c"))
}

/// `Cent -> Cent` Summe.
pub(super) fn add_cent(a: Cent, b: Cent) -> R<Cent> {
    crate::plus(a.get(), b.get()).map(Cent::new)
}
