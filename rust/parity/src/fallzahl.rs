//! Die Fallzahl der Zufalls- und Generator-Laeufe der Parity-Suiten: EINE Stelle, EIN Env-Knopf.
//!
//! `PARITY_N` setzt die Zahl der erzeugten Faelle. Ohne die Variable gilt der Standard der Suite
//! (die Zahl, die vorher im Quelltext stand), und der Lauf ist bitgleich zu vorher. Ein Wert, der
//! keine ganze Zahl ab 1 ist, bricht mit Klartext ab: nie ein stilles Standard, denn ein Lauf mit
//! `PARITY_N=10 000` (Leerzeichen, Tippfehler) wuerde sonst mit 1000 Faellen gruen melden.
//!
//! Zwei Dinge aendern sich, sobald `PARITY_N` gesetzt ist:
//! - Die Suite meldet die Zahl auf stderr (einmal je Stelle). Das geht am Capture von `cargo test`
//!   vorbei, damit eine Gate-Zeile die Zahl auch ohne `--nocapture` zeigt.
//! - Abdeckungs-Waechter, die an die Standardzahl gebunden sind (Pins bekannt leerer Zeilen,
//!   Untergrenzen wie `n >= 1000`), laufen nur bei `n == standard` ([`wache_gilt`]). Bei kleinerem
//!   N sind sie falsch rot, bei groesserem koennen gepinnte leere Zeilen "wieder rechnen". Die
//!   Pruefung "keine Abweichung" gilt bei jedem N. Bei groesserem N gilt die Abdeckung des Standards
//!   weiter: die Folgen sind eine deterministische Fortsetzung, die ersten `standard` Faelle sind
//!   dieselben.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::io::Write as _;
use std::sync::Mutex;

/// Der Name des Env-Knopfs.
pub const ENV: &str = "PARITY_N";

static GEMELDET: Mutex<BTreeSet<String>> = Mutex::new(BTreeSet::new());

/// Der Wert von `PARITY_N` (`None` = nicht gesetzt) gegen den Standard der Suite.
fn lies(roh: Option<&OsStr>, standard: usize) -> Result<usize, String> {
    let Some(roh) = roh else {
        return Ok(standard);
    };
    match roh.to_str().map(str::parse::<usize>) {
        Some(Ok(n)) if n >= 1 => Ok(n),
        _ => Err(format!(
            "{ENV}=\"{}\" ist keine Fallzahl: erwartet wird eine ganze Zahl ab 1 ohne Leerzeichen \
             und Trennzeichen, z. B. {ENV}=10000. Ohne {ENV} laeuft die Suite mit ihrem Standard \
             ({standard}).",
            roh.to_string_lossy()
        )),
    }
}

/// Schreibt `zeile` einmal je `schluessel` und Prozess auf stderr, am Capture von `cargo test` vorbei.
fn melde_einmal(schluessel: &str, zeile: &str) {
    let mut gemeldet = GEMELDET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if gemeldet.insert(schluessel.to_owned()) {
        // Absichtlich nicht `eprintln!`: libtest faengt dessen Ausgabe bei bestandenen Tests ab.
        let _ = writeln!(std::io::stderr(), "{zeile}");
    }
}

/// Die Fallzahl fuer `wo` (Suite und Stelle, z. B. `"elster generierte_stores"`): `PARITY_N`, sonst
/// `standard`.
///
/// # Panics
///
/// Panikt mit Klartext, wenn `PARITY_N` gesetzt, aber keine ganze Zahl ab 1 ist.
#[must_use]
pub fn holen(wo: &str, standard: usize) -> usize {
    let roh = std::env::var_os(ENV);
    let gelesen = lies(roh.as_deref(), standard);
    assert!(
        gelesen.is_ok(),
        "{wo}: {}",
        gelesen.as_ref().err().map_or("", String::as_str)
    );
    let n = gelesen.unwrap_or(standard);
    if roh.is_some() {
        melde_einmal(
            &format!("n {wo}"),
            &format!("{ENV}={n}: {wo} laeuft mit {n} Faellen (Standard {standard})"),
        );
    }
    n
}

/// Wie [`holen`], fuer `proptest` (`Config::cases` ist `u32`).
///
/// # Panics
///
/// Panikt wie [`holen`], und wenn die Zahl nicht in ein `u32` passt.
#[must_use]
pub fn holen_u32(wo: &str, standard: u32) -> u32 {
    let n = holen(wo, usize::try_from(standard).unwrap_or(usize::MAX));
    let klein = u32::try_from(n);
    assert!(
        klein.is_ok(),
        "{wo}: {ENV}={n} ist groesser als {}",
        u32::MAX
    );
    klein.unwrap_or(u32::MAX)
}

/// Gilt der Abdeckungs-Waechter `wo` bei dieser Zahl? Nur bei `n == standard` (Begruendung im
/// Moduldoc). Sonst meldet die Funktion das einmal und gibt `false` zurueck.
#[must_use]
pub fn wache_gilt(wo: &str, n: usize, standard: usize) -> bool {
    if n == standard {
        return true;
    }
    melde_einmal(
        &format!("wache {wo}"),
        &format!(
            "{ENV}={n}: Abdeckungs-Waechter {wo} uebersprungen (gelten nur bei dem Standard {standard}); \
             'keine Abweichung' gilt weiter"
        ),
    );
    false
}

/// Wie [`wache_gilt`], wenn mehrere Stellen EINEN Pool speisen: `stellen` = (n, Standard) je Stelle. Der
/// Waechter gilt nur, wenn jede Stelle ihre Standardzahl hat.
#[must_use]
pub fn wache_gilt_pool(wo: &str, stellen: &[(usize, usize)]) -> bool {
    match stellen.iter().find(|(n, standard)| n != standard) {
        None => true,
        Some(&(n, standard)) => wache_gilt(wo, n, standard),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn os(s: &str) -> std::ffi::OsString {
        s.into()
    }

    #[test]
    fn ohne_variable_gilt_der_standard() {
        assert_eq!(lies(None, 1200), Ok(1200));
    }

    #[test]
    fn eine_ganze_zahl_ab_eins_gilt() {
        assert_eq!(lies(Some(&os("7")), 1000), Ok(7));
        assert_eq!(lies(Some(&os("1")), 1000), Ok(1));
        assert_eq!(lies(Some(&os("10000")), 1000), Ok(10_000));
    }

    #[test]
    fn jeder_andere_wert_ist_ein_fehler_mit_klartext() {
        for roh in [
            "", "0", "-3", "abc", "7.5", "1e4", "10 000", "10_000", " 7", "7 ", "10,5",
        ] {
            let e = lies(Some(&os(roh)), 1000).expect_err(roh);
            assert!(
                e.contains("PARITY_N") && e.contains("keine Fallzahl"),
                "{roh:?}: {e}"
            );
            assert!(e.contains("1000"), "der Standard steht in der Meldung: {e}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn ein_wert_der_kein_unicode_ist_ist_ein_fehler() {
        use std::os::unix::ffi::OsStringExt;
        let roh = std::ffi::OsString::from_vec(vec![0xff, 0xfe]);
        assert!(lies(Some(&roh), 1000).is_err());
    }

    #[test]
    fn wache_gilt_nur_beim_standard() {
        assert!(wache_gilt("test", 1000, 1000));
        assert!(!wache_gilt("test", 7, 1000));
        assert!(!wache_gilt("test", 10_000, 1000));
    }

    #[test]
    fn wache_gilt_pool_nur_wenn_jede_stelle_ihren_standard_hat() {
        assert!(wache_gilt_pool("test", &[(1500, 1500), (1000, 1000)]));
        assert!(wache_gilt_pool("test", &[]));
        // PARITY_N=1500: die erste Stelle steht auf ihrem Standard, die zweite nicht.
        assert!(!wache_gilt_pool("test", &[(1500, 1500), (1500, 1000)]));
        assert!(!wache_gilt_pool("test", &[(7, 1500), (7, 1000)]));
    }
}
