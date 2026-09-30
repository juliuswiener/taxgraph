//! Gemeinsamer Helfer: `rust_decimal::Decimal` -> Zaehler/Nenner, wie `catala_new_frac` sie auf
//! der C-Seite erwartet (`CATALA_DEC`-Eingabefelder: Prozentsaetze, Entfernung in km).
use rust_decimal::Decimal;

/// `Decimal` verlaesst beim Umwandeln in Zaehler/Nenner den `i64`/`u64`-Wertebereich. Bei
/// Prozentsaetzen aus `params/**/*.yaml` (0-2 Nachkommastellen, Betrag < 100) und
/// Entfernungsangaben in km praktisch unerreichbar, aber `checked_*` statt eines stillen
/// Wrap-Arounds (`REWRITE_PLAN.md` §4, "fail-closed statt fail-open").
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Dezimalwert {0} ueberschreitet den Zaehler/Nenner-Wertebereich von catala_new_frac")]
pub struct DezimalUeberlauf(pub Decimal);

/// Wandelt `d` in `(zaehler, nenner)` um, sodass `zaehler / 10^scale == d`.
///
/// # Errors
/// [`DezimalUeberlauf`], wenn Mantisse oder Nenner den `i64`/`u64`-Wertebereich verlassen.
///
/// ```
/// use engine::dezimal::zu_bruch;
/// use rust_decimal::Decimal;
/// assert_eq!(zu_bruch(Decimal::new(40, 2)).unwrap(), (40, 100));
/// ```
pub fn zu_bruch(d: Decimal) -> Result<(i64, u64), DezimalUeberlauf> {
    let num = i64::try_from(d.mantissa()).map_err(|_| DezimalUeberlauf(d))?;
    let den = 10i64.checked_pow(d.scale()).ok_or(DezimalUeberlauf(d))?;
    let den = u64::try_from(den).map_err(|_| DezimalUeberlauf(d))?;
    // Der Nenner ist eine Zehnerpotenz, nie 0 (Catala teilt durch ihn).
    debug_assert!(den >= 1);
    Ok((num, den))
}

#[cfg(test)]
mod tests {
    use super::zu_bruch;
    use rust_decimal::Decimal;

    #[test]
    fn zwanzig_prozent_wird_20_durch_100() {
        assert_eq!(zu_bruch(Decimal::new(20, 2)).unwrap(), (20, 100));
    }

    #[test]
    fn ganze_zahl_hat_nenner_eins() {
        assert_eq!(zu_bruch(Decimal::new(15, 0)).unwrap(), (15, 1));
    }
}
