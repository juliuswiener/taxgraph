//! § 22 Nr. 1 `EStG` Renten und § 19 Abs. 2 `EStG` Versorgungsbezuege (`runner.py` Weg A).
use domain::{Euro, Vz};

use bindung::Params;
use rust_decimal::Decimal;

use super::{euro, z, EngineFehler};

/// Rentenart und die fuer sie gelesenen Felder.
///
/// Python-Zuordnung (`runner.py` `AA_RENTEN_ARTEN`/`BB_RENTEN_ARTEN`): `gesetzliche_rente`,
/// `berufsstaendische_versorgung`, `private_basisrente` -> [`Rentenart::Aa`];
/// `private_leibrente`, `sonstige_leibrente` -> [`Rentenart::Bb`]; jede andere ->
/// [`Rentenart::NichtRingfaehig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rentenart {
    /// § 22 Nr. 1 S. 3 a aa (Basisversorgung, Kohorte nach Rentenbeginn).
    Aa {
        renten_beginn_jahr: i64,
        /// Ab dem zweiten Bezugsjahr fixierter Freibetrag; `None` = nicht fixiert.
        rentenfreibetrag: Option<Euro>,
    },
    /// § 22 Nr. 1 S. 3 a bb (Leibrente, Ertragsanteil nach Alter).
    Bb { alter_bei_rentenbeginn: i64 },
    /// Jede andere Rentenart (z. B. `betriebsrente_direktzusage`): Python `ValueError`.
    NichtRingfaehig,
}

/// Eingabe fuer [`renten_einkuenfte`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RentenEingabe {
    pub vz: Vz,
    pub art: Rentenart,
    /// PARITÄT: Python setzt fehlend = 0.
    pub jahresrente: Euro,
}

/// Python `round(prozent * 10)` auf dem YAML-Float (`_renten_stpfl`,
/// `catala_p19_2_versorgungsfreibetrag`).
///
/// `bindung` liest den Float als kuerzeste Dezimaldarstellung; Zurueckparsen liefert genau den
/// `f64`, den Python haelt. `(v * 10.0).round_ties_even()` ist dieselbe IEEE-754-Operation wie
/// Pythons `round` (half-even). Danach ist alles exakte Ganzzahlarithmetik -- bitgleich zu
/// Python. Beweis ueber den ganzen Tabellen-Definitionsbereich: `kohorten_exhaustiv` in
/// `rust/parity/tests/zugriff_teil2_paritaet.rs`.
fn zehntel(prozent: Decimal) -> Result<i128, EngineFehler> {
    let v: f64 = prozent.to_string().parse().map_err(|_| super::UEBERLAUF)?;
    let r = (v * 10.0).round_ties_even();
    if !r.is_finite() || r.abs() > 1e15 {
        return Err(super::UEBERLAUF);
    }
    // ponytail: |r| <= 1e15 < 2^53 und ganzzahlig, `as` schneidet nichts ab.
    #[allow(clippy::cast_possible_truncation)]
    Ok(i128::from(r as i64))
}

/// `jahresrente * round(prozent * 10) // 1000` -- Python `_renten_stpfl`.
fn stpfl(rente: Euro, prozent: Decimal) -> Result<i128, EngineFehler> {
    Ok((z(rente) * zehntel(prozent)?).div_euclid(1000))
}

/// § 22 Nr. 1 `EStG`: steuerpflichtige Renten-Einkuenfte, EURO.
///
/// - bb: `jahresrente x Ertragsanteil(alter) - WK-PB`
/// - aa Erstjahr (Beginn == VZ): `jahresrente x Besteuerungsanteil(Beginn) - WK-PB`
/// - aa Folgejahr mit Freibetrag: `(jahresrente - rentenfreibetrag) - WK-PB`
/// - aa sonst (auch Beginn nach dem VZ): [`EngineFehler::RentenfreibetragFixierungOffen`]
///
/// Ergebnis mit Boden 0.
///
/// # Errors
/// Siehe oben; [`EngineFehler::TabelleOhneEintrag`] fuer ein Alter ausserhalb der Tabelle;
/// [`EngineFehler::RentenartNichtRingfaehig`].
///
/// ```
/// # use engine::zugriff::teil2::rente::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = RentenEingabe { vz: Vz::Vz2025, jahresrente: Euro::new(12000),
///     art: Rentenart::Aa { renten_beginn_jahr: 2025, rentenfreibetrag: None } };
/// assert_eq!(renten_einkuenfte(&e, &p).unwrap(), Euro::new(9918));
/// ```
pub fn renten_einkuenfte(e: &RentenEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let vz = i64::from(e.vz.jahr());
    let wk_pb = z(p.renten_wk_pauschbetrag(e.vz)?);
    let steuerpflichtig = match e.art {
        Rentenart::Bb {
            alter_bei_rentenbeginn,
        } => {
            let prozent = p.rente_ertragsanteil(alter_bei_rentenbeginn)?.ok_or(
                EngineFehler::TabelleOhneEintrag {
                    tabelle: "rente_ertragsanteil_p22",
                    schluessel: alter_bei_rentenbeginn,
                },
            )?;
            stpfl(e.jahresrente, prozent)?
        }
        Rentenart::Aa {
            renten_beginn_jahr,
            rentenfreibetrag,
        } => match rentenfreibetrag {
            _ if renten_beginn_jahr == vz => {
                let prozent = p.rente_besteuerungsanteil(renten_beginn_jahr)?.ok_or(
                    EngineFehler::TabelleOhneEintrag {
                        tabelle: "rente_besteuerungsanteil_p22",
                        schluessel: renten_beginn_jahr,
                    },
                )?;
                stpfl(e.jahresrente, prozent)?
            }
            Some(fb) if renten_beginn_jahr < vz => z(e.jahresrente) - z(fb),
            _ => {
                return Err(EngineFehler::RentenfreibetragFixierungOffen {
                    beginn: renten_beginn_jahr,
                    vz: e.vz.jahr(),
                })
            }
        },
        Rentenart::NichtRingfaehig => return Err(EngineFehler::RentenartNichtRingfaehig),
    };
    euro((steuerpflichtig - wk_pb).max(0))
}

/// Eingabe fuer [`p19_2_versorgungsfreibetrag`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersorgungsfreibetragEingabe {
    /// `versorgung_bemessungsgrundlage`, bei falsy-Wert ersatzweise
    /// `versorgungsbezuege_bemessungsgrundlage`. PARITÄT: Python setzt fehlend = 0, und 0
    /// gilt als fehlend.
    pub bemessungsgrundlage: Euro,
    /// `versorgung_beginn_jahr`, bei falsy-Wert ersatzweise `versorgungsbeginn_jahr`.
    /// PARITÄT: Python setzt fehlend = 0, und 0 gilt als fehlend.
    pub beginn_jahr: i64,
}

/// § 19 Abs. 2 `EStG`: Versorgungsfreibetrag + Zuschlag, EURO.
///
/// Kohorte nach Versorgungsbeginn (ausserhalb der Tabelle geklemmt); `vfb = min(BG x
/// Prozentsatz, Hoechstbetrag)`; Zuschlag hoechstens `max(0, BG - vfb)` (S. 5).
///
/// # Errors
/// [`EngineFehler::VersorgungsfreibetragOffen`], wenn BG oder Beginn 0 ist.
///
/// ```
/// # use engine::zugriff::teil2::rente::*;
/// # use domain::Euro;
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = VersorgungsfreibetragEingabe { bemessungsgrundlage: Euro::new(24000), beginn_jahr: 2025 };
/// assert_eq!(p19_2_versorgungsfreibetrag(&e, &p).unwrap(), Euro::new(990 + 297));
/// ```
pub fn p19_2_versorgungsfreibetrag(
    e: &VersorgungsfreibetragEingabe,
    p: &Params,
) -> Result<Euro, EngineFehler> {
    if e.bemessungsgrundlage.get() == 0 || e.beginn_jahr == 0 {
        return Err(EngineFehler::VersorgungsfreibetragOffen);
    }
    let k = p.versorgungsfreibetrag_kohorte(e.beginn_jahr)?;
    let bg = z(e.bemessungsgrundlage);
    let vfb = (bg * zehntel(k.prozentsatz)?)
        .div_euclid(1000)
        .min(z(k.hoechstbetrag));
    let zuschlag = z(k.zuschlag).min((bg - vfb).max(0));
    euro(vfb + zuschlag)
}

/// Eingabe fuer [`einkuenfte_versorgung`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EinkuenfteVersorgungEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (dann Einkuenfte 0).
    pub versorgung_jahresrente: Euro,
    pub freibetrag: VersorgungsfreibetragEingabe,
}

/// § 9a S. 1 Nr. 1 Buchst. b `EStG` -- Python hartkodiert 102 (`runner.py`
/// `catala_einkuenfte_versorgung`), nicht aus `params/<vz>`.
const VERSORGUNG_PAUSCHBETRAG: i128 = 102;

/// § 19 Abs. 2 `EStG`: Einkuenfte aus Versorgungsbezuegen nach Freibetrag, Zuschlag und
/// Pauschbetrag (hoechstens 102, gedeckelt auf den Rest), EURO.
///
/// PARITÄT (Befund P2): fehlt Bemessungsgrundlage oder Beginn, liefert Python 0 EUR statt
/// eines Fehlers -- `VersorgungsfreibetragOffen` wird verschluckt. Rust bildet das nach.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::rente::*;
/// # use domain::Euro;
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = EinkuenfteVersorgungEingabe { versorgung_jahresrente: Euro::new(24000),
///     freibetrag: VersorgungsfreibetragEingabe { bemessungsgrundlage: Euro::new(24000), beginn_jahr: 2025 } };
/// assert_eq!(einkuenfte_versorgung(&e, &p).unwrap(), Euro::new(24000 - 1287 - 102));
/// ```
pub fn einkuenfte_versorgung(
    e: &EinkuenfteVersorgungEingabe,
    p: &Params,
) -> Result<Euro, EngineFehler> {
    if e.versorgung_jahresrente.get() == 0 {
        return Ok(Euro::new(0));
    }
    let vfb = match p19_2_versorgungsfreibetrag(&e.freibetrag, p) {
        Ok(v) => z(v),
        // PARITÄT P2: verschluckt, s. Doku.
        Err(EngineFehler::VersorgungsfreibetragOffen) => return Ok(Euro::new(0)),
        Err(f) => return Err(f),
    };
    let rest = (z(e.versorgung_jahresrente) - vfb).max(0);
    let pausch = VERSORGUNG_PAUSCHBETRAG.min(rest);
    euro((rest - pausch).max(0))
}
