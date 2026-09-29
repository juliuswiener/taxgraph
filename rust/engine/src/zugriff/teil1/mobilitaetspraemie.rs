//! § 101 `EStG` Mobilitaetspraemie (`catala_p101_mobilitaetspraemie`, `_cent`), Hand-Python.
use domain::{Cent, Euro};

use super::fehler::{ok, EngineFehler};

/// Eingabe fuer [`p101_mobilitaetspraemie`] und [`p101_mobilitaetspraemie_cent`]. Alle Betraege
/// EURO. Bei Zusammenveranlagung uebergibt der Aufrufer gemeinsames zvE und VERDOPPELTEN
/// Grundfreibetrag (§ 101 S. 2 Hs. 2).
#[derive(Debug, Clone, Copy)]
pub struct P101Eingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Praemie 0)
    pub entfernungspauschale_ab_21km: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: zvE 0 maximiert die Unterschreitung)
    pub zu_versteuerndes_einkommen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: keine Unterschreitung)
    pub grundfreibetrag: Euro,
    /// PARITÄT: Python setzt fehlend = False (fail-open: § 101 S. 3 entfaellt)
    pub ist_arbeitnehmer: bool,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed). Nur gelesen, wenn `ist_arbeitnehmer`.
    pub werbungskosten_gesamt: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: kein Pauschbetrag abgezogen). Nur gelesen, wenn `ist_arbeitnehmer`.
    pub arbeitnehmer_pauschbetrag: Euro,
}

/// § 101 S. 1-3 `EStG` -- Bemessungsgrundlage, EURO: ab-21-km-EP; bei Arbeitnehmern nur soweit
/// sie mit den uebrigen WK den Pauschbetrag uebersteigt (S. 3); begrenzt auf die
/// Unterschreitung des Grundfreibetrags durch das zvE (S. 2).
fn bemessungsgrundlage(e: &P101Eingabe) -> Result<i64, EngineFehler> {
    let mut ep_ab_21 = e.entfernungspauschale_ab_21km.get();
    if e.ist_arbeitnehmer {
        let ueber = ok(
            e.werbungskosten_gesamt.get().checked_sub(e.arbeitnehmer_pauschbetrag.get()),
            "p101 wk",
        )?;
        ep_ab_21 = ep_ab_21.min(ueber.max(0));
    }
    let unterschreitung = ok(e.grundfreibetrag.get().checked_sub(e.zu_versteuerndes_einkommen.get()), "p101 gfb")?;
    Ok(ep_ab_21.min(unterschreitung.max(0)))
}

/// `catala_p101_mobilitaetspraemie` -- § 101 S. 4 `EStG`: 14 % der Bemessungsgrundlage, EURO
/// (abgerundet).
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::mobilitaetspraemie::{p101_mobilitaetspraemie, P101Eingabe};
/// let e = P101Eingabe {
///     entfernungspauschale_ab_21km: Euro::new(1000), zu_versteuerndes_einkommen: Euro::new(10_000),
///     grundfreibetrag: Euro::new(12_096), ist_arbeitnehmer: false,
///     werbungskosten_gesamt: Euro::new(0), arbeitnehmer_pauschbetrag: Euro::new(0),
/// };
/// assert_eq!(p101_mobilitaetspraemie(&e).unwrap(), Euro::new(140));
/// ```
pub fn p101_mobilitaetspraemie(e: &P101Eingabe) -> Result<Euro, EngineFehler> {
    let bg = bemessungsgrundlage(e)?;
    ok(bg.checked_mul(14).and_then(|x| x.checked_div_euclid(100)), "p101").map(Euro::new)
}

/// `catala_p101_mobilitaetspraemie_cent` -- § 101 S. 4 `EStG`: 14 % der Bemessungsgrundlage,
/// CENT-exakt (Euro x 14 = Cent, kein Rundungsschnitt; Auszahlung ohne Unter-Ansatz).
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use domain::{Cent, Euro};
/// use engine::zugriff::teil1::mobilitaetspraemie::{p101_mobilitaetspraemie_cent, P101Eingabe};
/// let e = P101Eingabe {
///     entfernungspauschale_ab_21km: Euro::new(1001), zu_versteuerndes_einkommen: Euro::new(10_000),
///     grundfreibetrag: Euro::new(12_096), ist_arbeitnehmer: false,
///     werbungskosten_gesamt: Euro::new(0), arbeitnehmer_pauschbetrag: Euro::new(0),
/// };
/// assert_eq!(p101_mobilitaetspraemie_cent(&e).unwrap(), Cent::new(14_014));
/// ```
pub fn p101_mobilitaetspraemie_cent(e: &P101Eingabe) -> Result<Cent, EngineFehler> {
    ok(bemessungsgrundlage(e)?.checked_mul(14), "p101 cent").map(Cent::new)
}
