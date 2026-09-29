//! Accessoren aus runner.py bis einschließlich `catala_p7_linear_afa`.
//!
//! Je `catala_<name>(s: dict)` eine Funktion `<name>(e: &<Name>Eingabe[, p: &Params])`. Die
//! Eingabe-Structs haben kein `Default`: jeder Wert, den Python mit `s.get(k, default)` still
//! ergaenzt, ist hier ein Pflichtfeld mit `PARITÄT:`-Vermerk. Rueckgabe in der Einheit, die
//! Python liefert (`Euro` oder `Cent`). `Params` nur, wo Python `params/` liest.

pub mod afa;
pub mod belastungen;
pub mod einkuenfte;
pub mod ermaessigungen;
pub mod fehler;
pub mod mobilitaetspraemie;
pub mod pauschbetraege;
pub mod reisekosten;
pub mod sonderausgaben;
pub mod werbungskosten;
