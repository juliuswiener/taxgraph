//! Bankimport: die Bausteine, die eine Buchung zu einem Vorschlag machen.
//!
//! [`buchung`] liest das Datum einer Buchung und beschreibt ihren Posten, [`saldo`] vereinigt
//! Posten aus zwei Auszuegen und bildet den Saldo eines Felds. Die Uebernahme selbst bleibt in
//! [`crate::kontoauszug`], weil dort die Kategorien und die Zielfelder sitzen.
pub mod buchung;
pub mod saldo;
