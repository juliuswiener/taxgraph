//! § 22 Nr. 1 `EStG` Renten und § 19 Abs. 2 `EStG` Versorgungsbezuege (`runner.py` Weg A).
use domain::{Euro, Satz, Vz};

use bindung::Params;
use rust_decimal::{Decimal, RoundingStrategy};

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

/// Python `round(prozent * 10)` (`_renten_stpfl`, `catala_p19_2_versorgungsfreibetrag`), exakt:
/// Zehntelprozent, half-even gerundet wie Pythons `round`.
///
/// Rechtsgrundlage: Die Tabellen in § 22 Nr. 1 S. 3 Buchst. a Doppelbuchst. aa S. 3
/// (Besteuerungsanteil) und § 19 Abs. 2 S. 3 `EStG` (Versorgungsfreibetrag) nennen Prozent mit
/// einer Nachkommastelle, Doppelbuchst. bb S. 4 (Ertragsanteil) ganze Prozent. Als Zehntelprozent
/// ist jeder Tabellenwert exakt, die Rundung aendert keinen.
///
/// PARITÄT, bewusst: Python rundet das Float-Produkt `prozent * 10`. Trifft das genau ,5, der
/// Dezimalwert aber nicht, weicht Python ab (Test `zehntel_rundet_den_dezimalwert`). Kein
/// Tabellenwert liegt so: `kohorten_exhaustiv` in `rust/parity/tests/zugriff_teil2_paritaet.rs`
/// prueft den ganzen Tabellen-Definitionsbereich.
fn zehntel(prozent: Satz) -> Result<i128, EngineFehler> {
    prozent
        .get()
        .checked_mul(Decimal::TEN)
        .map(|v| v.round_dp_with_strategy(0, RoundingStrategy::MidpointNearestEven))
        .and_then(|v| i64::try_from(v).ok())
        .map(i128::from)
        .ok_or(super::UEBERLAUF)
}

/// `jahresrente * round(prozent * 10) // 1000` -- Python `_renten_stpfl`.
fn stpfl(rente: Euro, prozent: Satz) -> Result<i128, EngineFehler> {
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
    let wk_pb = z(p.renten_wk_pauschbetrag(e.vz)?);
    euro((steuerpflichtig_vor_pauschbetrag(e, p)? - wk_pb).max(0))
}

/// Der steuerpflichtige Anteil EINER Rente VOR dem Werbungskosten-Pauschbetrag, EURO (Abweichung Nr. 55). Die Arme sind die von
/// [`renten_einkuenfte`]; der Pauschbetrag gilt nach § 9a Satz 1 Nr. 3 `EStG` "insgesamt" fuer alle Renten einer Person und wird
/// einmal von der Summe abgezogen ([`renten_nach_pauschbetrag`]), nicht je Rente.
///
/// # Errors
/// Wie [`renten_einkuenfte`].
pub fn renten_stpfl_vor_pauschbetrag(
    e: &RentenEingabe,
    p: &Params,
) -> Result<Euro, EngineFehler> {
    euro(steuerpflichtig_vor_pauschbetrag(e, p)?)
}

/// § 9a Satz 1 Nr. 3 `EStG`: der Pauschbetrag von 102 EUR gilt fuer Einnahmen aus § 22 Nr. 1, 1a und 5 `EStG` zusammen (Anleitung
/// zur Anlage R-AV/bAV, Zeilen 27 bis 33: "insgesamt"). Er wird EINMAL von der Summe der steuerpflichtigen Anteile aller Renten
/// einer Person abgezogen; das Ergebnis hat den Boden 0. Abweichung Nr. 55: vorher zog `renten_summe` ihn je Rente ab.
///
/// `ausgeschlossen` ist der Teil, der in die Bemessung des Altersentlastungsbetrags NICHT eingeht (Leibrenten nach § 22 Nr. 1 Satz 3
/// Buchst. a und Leistungen nach § 22 Nr. 5 Satz 2 Buchst. a, `EStG` § 24a Satz 2 Nr. 2 und 5), `eingeschlossen` der Teil, der
/// eingeht (Leistungen nach § 22 Nr. 5 Satz 1 ausser Satz 11, § 24a Satz 2 Nr. 4). Wie der Pauschbetrag auf beide Teile fiele,
/// sagt das Gesetz nicht.
// ponytail: der Pauschbetrag mindert zuerst den ausgeschlossenen Teil; der Unterschied zur anderen Wahl ist hoechstens der
// Altersentlastungsprozentsatz mal 102 EUR. Upgrade: eine Quelle, die die Aufteilung nennt.
///
/// # Errors
/// [`EngineFehler`] bei einem Betrag ausserhalb von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::rente::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = renten_nach_pauschbetrag(Euro::new(60), Euro::new(60), Vz::Vz2025, &p).unwrap();
/// assert_eq!((e.gesamt, e.eingeschlossen), (Euro::new(18), Euro::new(18)));
/// ```
pub fn renten_nach_pauschbetrag(
    ausgeschlossen: Euro,
    eingeschlossen: Euro,
    vz: Vz,
    p: &Params,
) -> Result<RentenEinkuenfte, EngineFehler> {
    let pauschbetrag = z(p.renten_wk_pauschbetrag(vz)?);
    let (aus, ein) = (z(ausgeschlossen), z(eingeschlossen));
    let rest = (pauschbetrag - aus).max(0);
    let (aus_nach, ein_nach) = ((aus - pauschbetrag).max(0), (ein - rest).max(0));
    Ok(RentenEinkuenfte {
        gesamt: euro(aus_nach + ein_nach)?,
        eingeschlossen: euro(ein_nach)?,
    })
}

/// Ergebnis von [`renten_nach_pauschbetrag`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RentenEinkuenfte {
    /// Einkuenfte aus allen Renten nach dem Pauschbetrag, Boden 0.
    pub gesamt: Euro,
    /// Davon der Teil, der in die Bemessung des Altersentlastungsbetrags eingeht.
    pub eingeschlossen: Euro,
}

fn steuerpflichtig_vor_pauschbetrag(e: &RentenEingabe, p: &Params) -> Result<i128, EngineFehler> {
    let vz = i64::from(e.vz.jahr());
    let steuerpflichtig = match e.art {
        Rentenart::Bb {
            alter_bei_rentenbeginn,
        } => {
            // Die letzte Tabellenzeile gilt nach oben offen: "… 94 bis 96 2 ab 97 1"
            // (sources/gesetze-im-internet/estg_p22_2026-07-13.txt:23). Nur der Schluessel wird gedeckelt.
            let prozent = p.rente_ertragsanteil(alter_bei_rentenbeginn.min(97))?.ok_or(
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
    Ok(steuerpflichtig)
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

#[cfg(test)]
mod tests {
    use super::{
        renten_einkuenfte, renten_nach_pauschbetrag, renten_stpfl_vor_pauschbetrag, zehntel,
        RentenEingabe, Rentenart,
    };
    use domain::{Euro, Satz, Vz};
    use rust_decimal::Decimal;

    fn params() -> bindung::Params {
        bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
            .unwrap()
    }

    /// Abweichung Nr. 55, § 9a Satz 1 Nr. 3 `EStG`: der Pauschbetrag von 102 EUR gilt "insgesamt" fuer alle Renten einer Person. Zwei
    /// Renten mit je 60 EUR bringen 120 - 102 = 18 EUR (vorher: je Rente auf 0 gedeckelt, zusammen 0). Von Hand gerechnet.
    #[test]
    fn rav_bav_pauschbetrag_gilt_einmal_fuer_alle_renten() {
        let p = params();
        let e = renten_nach_pauschbetrag(Euro::new(60), Euro::new(60), Vz::Vz2025, &p).unwrap();
        assert_eq!(e.gesamt, Euro::new(18));
        // Je Rente einzeln: 0 + 0; die Summe nach einmaligem Abzug ist hoeher.
        let je_rente = |betrag| {
            renten_einkuenfte(
                &RentenEingabe {
                    vz: Vz::Vz2025,
                    art: Rentenart::Bb { alter_bei_rentenbeginn: 65 },
                    jahresrente: Euro::new(betrag),
                },
                &p,
            )
            .unwrap()
        };
        assert_eq!(je_rente(333), Euro::new(0)); // 333 * 18 % = 59 EUR, unter dem Pauschbetrag
        // Boden 0 und die Grenze: unter 102 EUR bleibt nichts, ueber 102 EUR der Rest.
        for (summe, soll) in [(0, 0), (101, 0), (102, 0), (103, 1), (5000, 4898)] {
            let e = renten_nach_pauschbetrag(Euro::new(summe), Euro::new(0), Vz::Vz2025, &p).unwrap();
            assert_eq!(e.gesamt, Euro::new(soll), "Summe {summe}");
        }
    }

    /// Der Pauschbetrag mindert zuerst den Teil, der nicht in die Bemessung des Altersentlastungsbetrags eingeht (Annahme, mit
    /// `ponytail:` an der Funktion). Ausgeschlossen 80, eingeschlossen 100: 22 EUR Pauschbetrag bleiben, eingeschlossen 78.
    #[test]
    fn rav_bav_pauschbetrag_mindert_zuerst_den_ausgeschlossenen_teil() {
        let p = params();
        let e = renten_nach_pauschbetrag(Euro::new(80), Euro::new(100), Vz::Vz2025, &p).unwrap();
        assert_eq!((e.gesamt, e.eingeschlossen), (Euro::new(78), Euro::new(78)));
        let e = renten_nach_pauschbetrag(Euro::new(500), Euro::new(100), Vz::Vz2025, &p).unwrap();
        assert_eq!((e.gesamt, e.eingeschlossen), (Euro::new(498), Euro::new(100)));
    }

    /// `renten_einkuenfte` bleibt eine Rente nach Pauschbetrag (Parity-Gegenstueck zu Catala): die Zerlegung aendert nichts.
    #[test]
    fn rav_bav_zerlegung_laesst_renten_einkuenfte_unveraendert() {
        let p = params();
        for jahresrente in [0, 50, 102, 103, 12_000] {
            let e = RentenEingabe {
                vz: Vz::Vz2025,
                art: Rentenart::Bb { alter_bei_rentenbeginn: 65 },
                jahresrente: Euro::new(jahresrente),
            };
            let vor = renten_stpfl_vor_pauschbetrag(&e, &p).unwrap();
            let nach = renten_nach_pauschbetrag(vor, Euro::new(0), Vz::Vz2025, &p).unwrap();
            assert_eq!(nach.gesamt, renten_einkuenfte(&e, &p).unwrap(), "Rente {jahresrente}");
        }
    }

    /// PARITÄT, bewusst: Python `round(0.8500000000000001 * 10)` rundet das Float-Produkt 8.5
    /// half-even auf 8; exakt ist 8.500000000000001 -> 9. Kein Tabellenwert liegt so
    /// (`kohorten_exhaustiv`).
    #[test]
    fn zehntel_rundet_den_dezimalwert() {
        assert_eq!(
            zehntel(Satz::new(Decimal::new(8_500_000_000_000_001, 16))).unwrap(),
            9
        );
    }
}
