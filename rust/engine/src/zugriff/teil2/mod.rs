//! Accessoren aus runner.py ab `catala_sparer_pb`.
//!
//! Je `runner.catala_<name>` eine Funktion `<name>(e, p)` mit typisierter Eingabe `e` und den
//! Jahreswerten `p` ([`bindung::Params`]). Die Rueckgabe-Einheit steht im Typ (`Euro` oder `Cent`), genau
//! wie Python sie liefert.
//!
//! **Eingabe-Structs ohne `Default`.** Wo Python `s.get(k, default)` liest, ist das Feld hier
//! Pflicht; die Doku-Zeile `PARITÄT: Python setzt fehlend = <default>` benennt den stillen
//! Default als Befund. Den Default setzt nur der Paritaets-Adapter (`rust/parity/tests`).
//!
//! **Arithmetik.** Python rechnet mit unbeschraenkten `int`. Hier laeuft jede Zwischenrechnung in
//! `i128` (zwei `i64`-Faktoren plus wenige Summanden passen immer hinein); erst das Ergebnis geht
//! per `try_from` zurueck nach `i64`, sonst ein Ueberlauf-Fehler. Pythons `//` ist
//! Abrundung Richtung minus unendlich: bei positiven Konstanten `div_euclid`, bei variablen
//! Nennern [`floor_div`].

pub mod est;
pub mod gesamt;
pub mod gewerbe;
pub mod kapital;
pub mod p23;
pub mod p33;
pub mod p35c;
pub mod rente;
pub mod solz;
pub mod sonderausgaben;
pub mod sonstige;

use bindung::ParamsWertFehler;
use catala_sys::CatalaFehler;
use domain::{Cent, CentUeberlauf, Euro};

use super::teil1::fehler::EngineFehler as Basis;

/// Fehler der Teil-2-Accessoren. [`EngineFehler::Basis`] ist der Teil-1-Fehlertyp (Catala,
/// Parameter, Ueberlauf); die uebrigen Varianten sind Teil-2-eigene Python-Ausnahmen aus
/// `runner.py`. Der Paritaetstest prueft die Zuordnung.
#[derive(Debug, thiserror::Error)]
pub enum EngineFehler {
    /// Catala-Laufzeitfehler, Parameterfehler oder `i64`-Ueberlauf (Teil-1-Typ).
    #[error(transparent)]
    Basis(#[from] Basis),
    /// Python: `RentenfreibetragFixierungOffen` (§ 22 Nr. 1 S. 3 a aa, Folgejahr ohne
    /// fixierten Rentenfreibetrag).
    #[error("aa-Folgejahr {beginn} ohne fixierten Rentenfreibetrag (VZ {vz})")]
    RentenfreibetragFixierungOffen { beginn: i64, vz: u16 },
    /// Python: `VersorgungsfreibetragOffen` (§ 19 Abs. 2, Bemessungsgrundlage oder
    /// Versorgungsbeginn fehlt).
    #[error("Bemessungsgrundlage oder Versorgungsbeginn-Jahr fehlt")]
    VersorgungsfreibetragOffen,
    /// Python: `ValueError` (Rentenart ausserhalb aa/bb).
    #[error("Rentenart nicht ring-faehig (MVP: aa+bb)")]
    RentenartNichtRingfaehig,
    /// Python: `KeyError` beim Nachschlagen in einer Kohorten- oder Staffeltabelle.
    #[error("Tabelle {tabelle}: kein Eintrag fuer {schluessel}")]
    TabelleOhneEintrag { tabelle: &'static str, schluessel: i64 },
    /// Python: `ValueError` (§ 34 Abs. 1 S. 3 setzt ein positives zvE voraus).
    #[error("§ 34 Abs. 1 S. 3 setzt ein positives zvE voraus")]
    FuenftelZveNichtPositiv,
    /// Python: `ZeroDivisionError`.
    #[error("Division durch null")]
    DivisionDurchNull,
}

impl From<CatalaFehler> for EngineFehler {
    fn from(f: CatalaFehler) -> Self {
        Self::Basis(Basis::Catala(f))
    }
}

impl From<ParamsWertFehler> for EngineFehler {
    fn from(f: ParamsWertFehler) -> Self {
        Self::Basis(Basis::Params(f))
    }
}

impl From<CentUeberlauf> for EngineFehler {
    fn from(_: CentUeberlauf) -> Self {
        Self::Basis(Basis::Ueberlauf("Euro->Cent"))
    }
}

/// Kein Python-Gegenstueck: Python-`int` laeuft nie ueber.
const UEBERLAUF: EngineFehler = EngineFehler::Basis(Basis::Ueberlauf("teil2 i64"));

/// Pythons `a // b` fuer beliebige Vorzeichen (Abrundung Richtung minus unendlich).
///
/// # Errors
/// [`EngineFehler::DivisionDurchNull`] bei `b == 0`.
///
/// ```
/// use engine::zugriff::teil2::floor_div;
/// assert_eq!(floor_div(7, -2).unwrap(), -4);
/// assert_eq!(floor_div(-7, 2).unwrap(), -4);
/// assert_eq!(floor_div(7, 2).unwrap(), 3);
/// ```
pub fn floor_div(a: i128, b: i128) -> Result<i128, EngineFehler> {
    if b == 0 {
        return Err(EngineFehler::DivisionDurchNull);
    }
    let q = a / b;
    let r = if a % b != 0 && ((a < 0) != (b < 0)) {
        q - 1
    } else {
        q
    };
    // Floor-Division: bei positivem Teiler gilt r*b <= a < (r+1)*b.
    debug_assert!(b < 0 || (r * b <= a && a < (r + 1) * b));
    Ok(r)
}

/// `i128`-Zwischenwert als [`Euro`].
fn euro(v: i128) -> Result<Euro, EngineFehler> {
    i64::try_from(v).map(Euro::new).map_err(|_| UEBERLAUF)
}

/// `i128`-Zwischenwert als [`Cent`].
fn cent(v: i128) -> Result<Cent, EngineFehler> {
    i64::try_from(v).map(Cent::new).map_err(|_| UEBERLAUF)
}

/// Python `int(aufw * satz)`: Float-Produkt, dann Abschneiden Richtung 0.
///
/// PARITÄT: bewusst `f64` -- Python multipliziert `int x float` in IEEE-754 double; dieselbe
/// Operation in Rust ist bitgleich (auch `i64 -> f64` rundet in beiden Sprachen
/// round-to-nearest-even). Exakte Ganzzahlrechnung wuerde oberhalb 2^53 von Python abweichen.
/// Gemessen: fuer 1..=2.000.000 EUR und die Saetze 0.8/0.3/0.07/0.06 weicht der Float-Weg nie
/// vom exakten Prozentwert ab.
fn int_mal_float(aufw: Euro, satz: f64) -> Result<i128, EngineFehler> {
    // ponytail: Praezisionsverlust oberhalb 2^53 ist genau Pythons Verhalten.
    #[allow(clippy::cast_precision_loss)]
    let produkt = (aufw.get() as f64 * satz).trunc();
    if !produkt.is_finite() || produkt.abs() >= 9.2e18 {
        return Err(UEBERLAUF);
    }
    // ponytail: |produkt| < 2^63 und ganzzahlig, `as` schneidet nichts ab.
    #[allow(clippy::cast_possible_truncation)]
    Ok(i128::from(produkt as i64))
}

/// Rohwert eines [`Euro`] als `i128`.
fn z(e: Euro) -> i128 {
    i128::from(e.get())
}
