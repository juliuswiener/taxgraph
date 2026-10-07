//! § 34g `EStG`: Steuerermaessigung fuer Zuwendungen an politische Parteien (Abweichung Nr. 31 in
//! `rust/fixtures/README.md`). Nur Rust, kein Python-Gegenstueck und kein Catala-Scope: die Rechnung ist ein Satz mit Deckel.
//!
//! ponytail: reines Rust ohne Catala-Regel (`rules/estg/p34g` gibt es nicht). Upgrade-Pfad: ein Scope `Parteispenden` in
//! `rules/estg`, eingecheckter C-Export, Aufruf wie `p35a_haushaltsnahe`. Nicht gebaut sind Nummer 2 (unabhaengige
//! Waehlervereinigungen, Zeile 8) und der Sonderausgabenabzug des Teils ueber dem Deckel (§ 10b Abs. 2).
use domain::{Euro, Vz};

use super::{euro, int_mal_satz, z, EngineFehler};
use bindung::Params;

/// Eingabe fuer [`p34g_parteispenden`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParteispendenEingabe {
    pub vz: Vz,
    /// Spenden und Mitgliedsbeitraege an politische Parteien, ganze Euro (das Cent-Feld, abgerundet).
    pub spenden: Euro,
    /// Zusammenveranlagung von Ehegatten: der Deckel gilt in der doppelten Hoehe (`hoechstbetrag_zusammen`).
    pub zusammen: bool,
}

/// § 34g Satz 1 Nr. 1 und Satz 2 `EStG`: die Ermaessigung der tariflichen Steuer, EURO. `min(int(spenden x satz), Deckel)`,
/// Satz und Deckel je Veranlagungsjahr aus `params/<vz>/parteispenden_p34g.yaml`. Halbe Euro fallen weg (`int()` schneidet ab).
/// Bei `spenden <= 0` liest die Funktion die Parameterdatei nie und gibt 0: ein Fall ohne Parteispende rechnet auch in einem
/// Jahr, dem die Datei fehlt. Mit einem Betrag ueber 0 und ohne Datei bricht sie ab (Parameterfehler), statt mit einem
/// geratenen Deckel zu rechnen.
///
/// # Errors
/// [`EngineFehler::Basis`] mit einem Parameterfehler, wenn Datei oder Schluessel des Jahres fehlen; Ueberlauf jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::p34g::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = ParteispendenEingabe { vz: Vz::Vz2025, spenden: Euro::new(500), zusammen: false };
/// assert_eq!(p34g_parteispenden(&e, &p).unwrap(), Euro::new(250));
/// let ueber = ParteispendenEingabe { vz: Vz::Vz2025, spenden: Euro::new(4000), zusammen: false };
/// assert_eq!(p34g_parteispenden(&ueber, &p).unwrap(), Euro::new(825));
/// ```
pub fn p34g_parteispenden(e: &ParteispendenEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    if e.spenden.get() <= 0 {
        return Ok(Euro::new(0));
    }
    let s = p.parteispenden_p34g(e.vz)?;
    let deckel = if e.zusammen {
        s.hoechstbetrag_zusammen
    } else {
        s.hoechstbetrag_einzel
    };
    euro(int_mal_satz(e.spenden, s.satz)?.min(z(deckel)))
}
