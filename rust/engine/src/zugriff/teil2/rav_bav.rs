//! § 22 Nr. 5 `EStG`: Leistungen aus Altersvorsorgevertraegen, Pensionsfonds, Pensionskassen und Direktversicherungen (Anlage
//! R-AV/bAV, Abweichung Nr. 55 in `rust/fixtures/README.md`). Nur Rust: Python kennt die Anlage nicht und sperrt die Rente mit
//! Leistungsmitteilung nicht einmal; ein Parity-Gegenstueck gibt es nicht.
//!
//! Stufe 1 rechnet drei Nummern der Leistungsmitteilung, gebaut nach dem eingefrorenen Wortlaut und dem BMF-Schreiben
//! (`sources/bmf/bmf_riester_foerderung_2023-10-05.txt`, Rn. 139 bis 146), nicht nach der Bindung:
//!
//! * **Nr. 1 und Nr. 4** (Zeilen 4 und 10): Leistungen auf gefoerderten oder steuerfrei gestellten Beitraegen unterliegen "in
//!   vollem Umfang der Besteuerung nach § 22 Nr. 5 Satz 1 `EStG`" (Rn. 139 bis 142, Beispiel 6.000 EUR bleiben 6.000 EUR).
//! * **Nr. 5** (Zeile 15): Leibrenten auf nicht gefoerderten Beitraegen werden mit dem Ertragsanteil besteuert, § 22 Nr. 5 Satz 2
//!   Buchst. a i. V. m. Nr. 1 Satz 3 Buchst. a Doppelbuchst. bb (Rn. 145 und 146). Der Ertragsanteil haengt vom Alter bei
//!   Rentenbeginn ab; die Tabelle ist die der Leibrenten (`params/kohorten/rente_ertragsanteil_p22.yaml`), die Rechnung
//!   [`super::rente::renten_stpfl_vor_pauschbetrag`] mit [`Rentenart::Bb`].
//!
//! Der Pauschbetrag (102 EUR, § 9a Satz 1 Nr. 3) wird nicht hier abgezogen, sondern einmal fuer alle Renten einer Person
//! ([`super::rente::renten_nach_pauschbetrag`]).
use bindung::Params;
use domain::{Euro, Vz};

use super::privatverkauf::Tag;
use super::rente::{renten_stpfl_vor_pauschbetrag, RentenEingabe, Rentenart};
use super::{euro, z, EngineFehler};

/// Was eine Leistungsmitteilung in Stufe 1 traegt: die drei Betraege (Jahresbetraege in Euro) und das Alter bei Rentenbeginn der
/// Nummer 5 (vollendetes Lebensjahr, nur noetig, wenn `nr5` ueber 0 liegt).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mitteilung {
    pub vz: Vz,
    pub nr1: Euro,
    pub nr4: Euro,
    pub nr5: Euro,
    pub alter_bei_beginn_nr5: Option<i64>,
}

/// Der steuerpflichtige Anteil EINER Mitteilung, geteilt nach Verwendung im Altersentlastungsbetrag: `(voll, ertragsanteil)`. `voll`
/// (Nr. 1 und Nr. 4) geht in die Bemessung des Altersentlastungsbetrags ein (§ 24a Satz 2 Nr. 4 gilt nur fuer Satz 11), `ertragsanteil`
/// (Nr. 5) nicht (§ 24a Satz 2 Nr. 5). Beide VOR dem Pauschbetrag.
///
/// Ein `nr5` ueber 0 ohne Alter ist ein Fehler des Aufrufers ([`EngineFehler::TabelleOhneEintrag`]), keine stille Null: der
/// Aufrufer prueft Datum und Geburtsdatum vorher und sperrt mit Klartext.
///
/// # Errors
/// [`EngineFehler`] bei einem Betrag ausserhalb von `i64` oder einem Alter ausserhalb der Ertragsanteil-Tabelle.
///
/// ```
/// # use engine::zugriff::teil2::rav_bav::*;
/// # use domain::{Euro, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let m = Mitteilung { vz: Vz::Vz2025, nr1: Euro::new(6000), nr4: Euro::new(0), nr5: Euro::new(10000), alter_bei_beginn_nr5: Some(65) };
/// assert_eq!(mitteilung_stpfl(&m, &p).unwrap(), (Euro::new(6000), Euro::new(1800)));
/// ```
pub fn mitteilung_stpfl(m: &Mitteilung, p: &Params) -> Result<(Euro, Euro), EngineFehler> {
    let voll = euro(z(m.nr1) + z(m.nr4))?;
    let ertragsanteil = if z(m.nr5) == 0 {
        Euro::new(0)
    } else {
        let alter = m.alter_bei_beginn_nr5.ok_or(EngineFehler::TabelleOhneEintrag {
            tabelle: "rente_ertragsanteil_p22",
            schluessel: -1,
        })?;
        renten_stpfl_vor_pauschbetrag(
            &RentenEingabe {
                vz: m.vz,
                art: Rentenart::Bb {
                    alter_bei_rentenbeginn: alter,
                },
                jahresrente: m.nr5,
            },
            p,
        )?
    };
    Ok((voll, ertragsanteil))
}

/// Das bei Rentenbeginn vollendete Lebensjahr (§ 22 Nr. 1 Satz 3 Buchst. a Doppelbuchst. bb: "Bei Beginn der Rente vollendetes
/// Lebensjahr des Rentenberechtigten"): die Jahresdifferenz, einen weniger, solange der Geburtstag im Beginnjahr noch nicht erreicht
/// ist. Am Geburtstag selbst ist das Lebensjahr vollendet. `None`, wenn die Rente vor der Geburt begaenne.
///
/// ```
/// # use engine::zugriff::teil2::privatverkauf::Tag;
/// # use engine::zugriff::teil2::rav_bav::alter_bei_beginn;
/// let t = |s| Tag::aus_tt_mm_jjjj(s).unwrap();
/// assert_eq!(alter_bei_beginn(t("15.06.1960"), t("14.06.2025")), Some(64));
/// assert_eq!(alter_bei_beginn(t("15.06.1960"), t("15.06.2025")), Some(65));
/// assert_eq!(alter_bei_beginn(t("15.06.1960"), t("14.06.1960")), None);
/// ```
#[must_use]
pub fn alter_bei_beginn(geburt: Tag, beginn: Tag) -> Option<i64> {
    if beginn < geburt {
        return None;
    }
    let ohne_geburtstag = (beginn.monat, beginn.tag) < (geburt.monat, geburt.tag);
    Some(i64::from(beginn.jahr - geburt.jahr) - i64::from(ohne_geburtstag))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn params() -> Params {
        Params::lade(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
    }

    fn tag(s: &str) -> Tag {
        Tag::aus_tt_mm_jjjj(s).unwrap()
    }

    fn m(nr1: i64, nr4: i64, nr5: i64, alter: Option<i64>) -> Mitteilung {
        Mitteilung {
            vz: Vz::Vz2025,
            nr1: Euro::new(nr1),
            nr4: Euro::new(nr4),
            nr5: Euro::new(nr5),
            alter_bei_beginn_nr5: alter,
        }
    }

    /// Nr. 1 und Nr. 4 sind voll steuerpflichtig (BMF Rn. 140: 6.000 EUR bleiben 6.000 EUR), Nr. 5 mit dem Ertragsanteil des
    /// Alters bei Rentenbeginn: Alter 65 = 18 %, Alter 67 = 17 % (Tabelle § 22 Nr. 1 Satz 3 Buchst. a bb, von Hand gelesen).
    #[test]
    fn rav_bav_nr1_und_nr4_voll_nr5_mit_ertragsanteil() {
        let p = params();
        assert_eq!(mitteilung_stpfl(&m(6000, 0, 0, None), &p).unwrap(), (Euro::new(6000), Euro::new(0)));
        assert_eq!(mitteilung_stpfl(&m(0, 2500, 0, None), &p).unwrap(), (Euro::new(2500), Euro::new(0)));
        assert_eq!(mitteilung_stpfl(&m(0, 0, 10000, Some(65)), &p).unwrap(), (Euro::new(0), Euro::new(1800)));
        assert_eq!(mitteilung_stpfl(&m(0, 0, 10000, Some(67)), &p).unwrap(), (Euro::new(0), Euro::new(1700)));
        // Alle drei in einer Mitteilung: die Teile addieren sich nicht ineinander, `voll` ist Nr. 1 plus Nr. 4.
        assert_eq!(
            mitteilung_stpfl(&m(6000, 1000, 10000, Some(65)), &p).unwrap(),
            (Euro::new(7000), Euro::new(1800))
        );
    }

    /// Ein Betrag in Nr. 5 ohne Alter ist ein Fehler, keine Null; ohne Betrag braucht Nr. 5 kein Alter.
    #[test]
    fn rav_bav_nr5_ohne_alter_ist_ein_fehler_ohne_betrag_nicht() {
        let p = params();
        assert!(mitteilung_stpfl(&m(0, 0, 10000, None), &p).is_err());
        assert_eq!(mitteilung_stpfl(&m(0, 0, 0, None), &p).unwrap(), (Euro::new(0), Euro::new(0)));
    }

    /// Das vollendete Lebensjahr bei Rentenbeginn, an den Grenztagen von Hand gerechnet: Geburt 15.06.1960, Rentenbeginn am Tag vor
    /// dem 65. Geburtstag = 64, am Geburtstag = 65; Schalttag-Geburt am 28.02. des Nichtschaltjahres noch nicht vollendet.
    #[test]
    fn rav_bav_alter_bei_beginn_an_den_grenztagen() {
        assert_eq!(alter_bei_beginn(tag("15.06.1960"), tag("14.06.2025")), Some(64));
        assert_eq!(alter_bei_beginn(tag("15.06.1960"), tag("15.06.2025")), Some(65));
        assert_eq!(alter_bei_beginn(tag("15.06.1960"), tag("31.12.2025")), Some(65));
        assert_eq!(alter_bei_beginn(tag("15.06.1960"), tag("01.01.2025")), Some(64));
        assert_eq!(alter_bei_beginn(tag("29.02.1960"), tag("28.02.2025")), Some(64));
        assert_eq!(alter_bei_beginn(tag("29.02.1960"), tag("01.03.2025")), Some(65));
        assert_eq!(alter_bei_beginn(tag("15.06.1960"), tag("14.06.1960")), None);
        assert_eq!(alter_bei_beginn(tag("15.06.1960"), tag("15.06.1960")), Some(0));
    }
}
