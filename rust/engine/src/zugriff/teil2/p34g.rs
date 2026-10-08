//! § 34g `EStG`: Steuerermaessigung fuer Zuwendungen an politische Parteien (Abweichung Nr. 31 in
//! `rust/fixtures/README.md`). Nur Rust, kein Python-Gegenstueck und kein Catala-Scope: die Rechnung ist ein Satz mit Deckel.
//!
//! Dazu der Teil einer Spende ueber der Basis der Ermaessigung als Sonderausgabe (§ 10b Abs. 2, Abweichung Nr. 43):
//! [`p10b_parteispenden_sonderausgabe`].
//!
//! ponytail: reines Rust ohne Catala-Regel (`rules/estg/p34g` gibt es nicht). Upgrade-Pfad: ein Scope `Parteispenden` in
//! `rules/estg`, eingecheckter C-Export, Aufruf wie `p35a_haushaltsnahe`. Nicht gebaut sind Nummer 2 (unabhaengige
//! Waehlervereinigungen, Zeile 8) und die Ausschlussklausel (Partei von der staatlichen Teilfinanzierung ausgeschlossen).
use domain::{Euro, Vz};

use super::{euro, int_durch_satz, int_mal_satz, z, EngineFehler};
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

/// § 10b Abs. 2 `EStG`: der Teil einer Parteispende ueber der Basis der Ermaessigung, der als Sonderausgabe abziehbar ist,
/// EURO. `min(max(0, spenden - basis), deckel)`; `basis` = Hoechstbetrag der Ermaessigung / Satz (die Spende, bei der § 34g
/// seinen Hoechstbetrag erreicht), `deckel` = Deckel aus § 10b Abs. 2 Satz 1. Satz 2 (nur soweit § 34g nicht gewaehrt wurde)
/// ist damit der Abzug der Basis. Einzel- und Zusammen-Werte je Veranlagungsjahr aus `params/<vz>/parteispenden_p34g.yaml`.
/// Bei `spenden <= 0` liest die Funktion die Parameterdatei nie und gibt 0 (wie [`p34g_parteispenden`]); mit einem Betrag ueber
/// 0 und ohne Datei bricht sie ab. Die Lesart "zusaetzlich" stuetzt sich auf die Anleitung zur Anlage Sonderausgaben 2025, nicht
/// auf den Wortlaut allein (Vault: `backlog/taxgraph/parteispenden-ueber-dem-deckel-waehlervereinigungen-und-kontoauszug-fehlen`).
///
/// # Errors
/// [`EngineFehler::Basis`] mit einem Parameterfehler, wenn Datei oder Schluessel des Jahres fehlen; Ueberlauf jenseits von `i64`;
/// [`EngineFehler::DivisionDurchNull`] bei Satz 0.
///
/// ```
/// # use engine::zugriff::teil2::p34g::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let bis_basis = ParteispendenEingabe { vz: Vz::Vz2025, spenden: Euro::new(1650), zusammen: false };
/// assert_eq!(p10b_parteispenden_sonderausgabe(&bis_basis, &p).unwrap(), Euro::new(0));
/// let ueber = ParteispendenEingabe { vz: Vz::Vz2025, spenden: Euro::new(3000), zusammen: false };
/// assert_eq!(p10b_parteispenden_sonderausgabe(&ueber, &p).unwrap(), Euro::new(1350));
/// let ueber_deckel = ParteispendenEingabe { vz: Vz::Vz2025, spenden: Euro::new(4000), zusammen: false };
/// assert_eq!(p10b_parteispenden_sonderausgabe(&ueber_deckel, &p).unwrap(), Euro::new(1650));
/// ```
pub fn p10b_parteispenden_sonderausgabe(
    e: &ParteispendenEingabe,
    p: &Params,
) -> Result<Euro, EngineFehler> {
    if e.spenden.get() <= 0 {
        return Ok(Euro::new(0));
    }
    let s = p.parteispenden_p34g(e.vz)?;
    let (hoechstbetrag, deckel) = if e.zusammen {
        (s.hoechstbetrag_zusammen, s.sonderausgaben_hoechstbetrag_zusammen)
    } else {
        (s.hoechstbetrag_einzel, s.sonderausgaben_hoechstbetrag_einzel)
    };
    let basis = int_durch_satz(hoechstbetrag, s.satz)?;
    euro((z(e.spenden) - basis).max(0).min(z(deckel)))
}
