//! § 23 Abs. 1 und 3 `EStG`: wann ein privater Verkauf steuerpflichtig ist und was er an Gewinn bringt (Abweichung Nr. 54 in
//! `rust/fixtures/README.md`). Nur Rust: Python rechnet die Befreiungen nicht, sondern sperrt jeden Verkauf. Die Rechenregeln
//! `p23_veraeusserungsgewinn`, `p23_freigrenze` und `p23_verlusttopf` stehen in [`super::p23`] und bleiben unveraendert, weil
//! `rust/parity` sie gegen Python haelt.
//!
//! Gebaut nach dem eingefrorenen Wortlaut (`sources/gesetze-im-internet/estg_p23_*`), nicht nach der Bindung:
//!
//! * **Frist** (Abs. 1 Nr. 1 und 2): "nicht mehr als zehn Jahre" bei Grundstuecken, "nicht mehr als ein Jahr" bei anderen
//!   Wirtschaftsguetern, zehn Jahre, wenn aus dem Gut als Einkunftsquelle mindestens in einem Kalenderjahr Einkuenfte erzielt
//!   werden. Das Fristende folgt `§ 188 Abs. 2 und 3 BGB` (ABGELEITET, nicht im Korpus): der Tag, der durch seine Benennung dem
//!   Anschaffungstag entspricht, fehlt der Monat dieses Tags im letzten Jahr (29. Februar), endet die Frist am letzten Tag des
//!   Monats.
//! * **Eigennutzung** (Abs. 1 Nr. 1 Satz 3, nur Grundstuecke): ausschliesslich zu eigenen Wohnzwecken zwischen Anschaffung und
//!   Veraeusserung, oder im Jahr der Veraeusserung und in den beiden Vorjahren. Teilnutzung rechnet dieses Modul nicht
//!   (der Aufrufer sperrt vorher).
//! * **Gegenstand des taeglichen Gebrauchs** (Abs. 1 Nr. 2 Satz 2, nur andere Wirtschaftsgueter): steuerfrei.
//! * **Unentgeltlicher Erwerb** (Abs. 1 Satz 3 nach Nr. 3 der Fassung): dem Einzelrechtsnachfolger ist die Anschaffung des
//!   Rechtsvorgaengers zuzurechnen. Das ist keine Befreiung, sondern ein Eingabewert: der Aufrufer reicht das Datum des
//!   Rechtsvorgaengers als Anschaffungsdatum durch.
//! * **`AfA`** (Abs. 3 Satz 4): die Anschaffungs- oder Herstellungskosten mindern sich um Absetzungen fuer Abnutzung.
use bindung::Params;
use domain::{Cent, Vz};

use super::{cent, EngineFehler};

/// Ein Kalendertag, nur fuer die Fristrechnung. Die Ordnung ist die zeitliche (Jahr, Monat, Tag).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag {
    pub jahr: i32,
    pub monat: u32,
    pub tag: u32,
}

const fn schaltjahr(jahr: i32) -> bool {
    (jahr % 4 == 0 && jahr % 100 != 0) || jahr % 400 == 0
}

const fn tage_im_monat(jahr: i32, monat: u32) -> u32 {
    match monat {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if schaltjahr(jahr) => 29,
        _ => 28,
    }
}

impl Tag {
    /// Liest `TT.MM.JJJJ` (das amtliche ELSTER-Format, genau zehn Zeichen) als echten Kalendertag. Jahre vor 1900 und nach 2100
    /// sind kein Verkaufsdatum. `None` heisst: kein lesbarer Tag (`31.04.2025`, `29.02.2025`, `1.1.2025`, ISO-Format).
    ///
    /// ```
    /// # use engine::zugriff::teil2::privatverkauf::Tag;
    /// assert_eq!(Tag::aus_tt_mm_jjjj("29.02.2024"), Some(Tag { jahr: 2024, monat: 2, tag: 29 }));
    /// assert_eq!(Tag::aus_tt_mm_jjjj("29.02.2025"), None);
    /// ```
    #[must_use]
    pub fn aus_tt_mm_jjjj(text: &str) -> Option<Self> {
        let b = text.as_bytes();
        if b.len() != 10 || b.get(2) != Some(&b'.') || b.get(5) != Some(&b'.') {
            return None;
        }
        let ziffern = |von: usize, bis: usize| -> Option<u32> {
            let teil = text.get(von..bis)?;
            if teil.bytes().all(|c| c.is_ascii_digit()) {
                teil.parse().ok()
            } else {
                None
            }
        };
        let (tag, monat, jahr) = (ziffern(0, 2)?, ziffern(3, 5)?, i32::try_from(ziffern(6, 10)?).ok()?);
        if !(1900..=2100).contains(&jahr) || !(1..=12).contains(&monat) {
            return None;
        }
        (1..=tage_im_monat(jahr, monat))
            .contains(&tag)
            .then_some(Self { jahr, monat, tag })
    }

    /// Das Ende einer Frist von `n` Jahren, die am Tag nach `self` beginnt (`§ 188 Abs. 2 und 3 BGB`): derselbe Tag und Monat
    /// `n` Jahre spaeter, beim 29. Februar ohne Schaltjahr der 28. Februar. Ein Verkauf an diesem Tag liegt noch in der Frist.
    #[must_use]
    pub const fn frist_ende(self, n: i32) -> Self {
        let jahr = self.jahr + n;
        let tag = if self.tag > tage_im_monat(jahr, self.monat) {
            tage_im_monat(jahr, self.monat)
        } else {
            self.tag
        };
        Self {
            jahr,
            monat: self.monat,
            tag,
        }
    }
}

/// Die Art des verkauften Gutes, soweit § 23 sie unterscheidet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art {
    /// Grundstuecke und Rechte, die den Vorschriften ueber Grundstuecke unterliegen (Abs. 1 Nr. 1): zehn Jahre.
    Grundstueck,
    /// Andere Wirtschaftsgueter (Abs. 1 Nr. 2): ein Jahr.
    AnderesWg,
}

/// Wie lange ein Grundstueck zu eigenen Wohnzwecken genutzt wurde (Abs. 1 Nr. 1 Satz 3). Teilnutzung gehoert nicht hierher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eigennutzung {
    /// Nicht oder nicht in einer Weise, die Satz 3 ausnimmt.
    Nein,
    /// Ausschliesslich zu eigenen Wohnzwecken zwischen Anschaffung und Veraeusserung.
    Ausschliesslich,
    /// Im Jahr der Veraeusserung und in den beiden vorangegangenen Jahren zu eigenen Wohnzwecken.
    LetzteDreiJahre,
}

/// Eingabe fuer [`p23_befreiung`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerkaufEingabe {
    pub art: Art,
    /// Anschaffung; bei unentgeltlichem Erwerb die des Rechtsvorgaengers.
    pub anschaffung: Tag,
    pub veraeusserung: Tag,
    /// Nur bei [`Art::Grundstueck`] gelesen.
    pub eigennutzung: Eigennutzung,
    /// Nur bei [`Art::AnderesWg`] gelesen: Gegenstand des taeglichen Gebrauchs.
    pub taeglicher_gebrauch: bool,
    /// Nur bei [`Art::AnderesWg`] gelesen: aus dem Gut wurden in mindestens einem Kalenderjahr Einkuenfte erzielt.
    pub einkunftsquelle: bool,
}

/// Ob ein Verkauf steuerpflichtig ist, und wenn nicht, warum nicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Befreiung {
    /// Innerhalb der Frist und nicht ausgenommen: der Gewinn zaehlt (vorbehaltlich der Freigrenze).
    Steuerpflichtig,
    /// Die Frist ist abgelaufen (Zahl liegt vor, der Gewinn zaehlt nicht).
    FristAbgelaufen,
    /// Grundstueck zu eigenen Wohnzwecken genutzt (Abs. 1 Nr. 1 Satz 3).
    Eigennutzung,
    /// Gegenstand des taeglichen Gebrauchs (Abs. 1 Nr. 2 Satz 2).
    TaeglicherGebrauch,
}

/// Abs. 1 Nr. 1 und 2: ist dieser Verkauf ein steuerpflichtiges privates Veraeusserungsgeschaeft?
///
/// Die Reihenfolge ist: Ausnahmen der Art zuerst (Eigennutzung beim Grundstueck, taeglicher Gebrauch beim anderen Gut), dann die
/// Frist. Liegt die Veraeusserung VOR der Anschaffung (Abs. 1 Nr. 4, Leerverkauf), ist die Rechnung nicht definiert; der
/// Aufrufer sperrt vorher.
///
/// ```
/// # use engine::zugriff::teil2::privatverkauf::*;
/// let kauf = Tag::aus_tt_mm_jjjj("15.06.2015").unwrap();
/// let mut e = VerkaufEingabe { art: Art::Grundstueck, anschaffung: kauf,
///     veraeusserung: Tag::aus_tt_mm_jjjj("15.06.2025").unwrap(), eigennutzung: Eigennutzung::Nein,
///     taeglicher_gebrauch: false, einkunftsquelle: false };
/// assert_eq!(p23_befreiung(&e), Befreiung::Steuerpflichtig);
/// e.veraeusserung = Tag::aus_tt_mm_jjjj("16.06.2025").unwrap();
/// assert_eq!(p23_befreiung(&e), Befreiung::FristAbgelaufen);
/// ```
#[must_use]
pub fn p23_befreiung(e: &VerkaufEingabe) -> Befreiung {
    let jahre = match e.art {
        Art::Grundstueck => {
            if e.eigennutzung != Eigennutzung::Nein {
                return Befreiung::Eigennutzung;
            }
            10
        }
        Art::AnderesWg => {
            if e.taeglicher_gebrauch {
                return Befreiung::TaeglicherGebrauch;
            }
            if e.einkunftsquelle {
                10
            } else {
                1
            }
        }
    };
    if e.veraeusserung <= e.anschaffung.frist_ende(jahre) {
        Befreiung::Steuerpflichtig
    } else {
        Befreiung::FristAbgelaufen
    }
}

/// § 23 Abs. 3 S. 1 und S. 4: Gewinn oder Verlust eines Verkaufs, CENT. Die Anschaffungs- oder Herstellungskosten mindern sich um
/// die abgezogene `AfA`, `Preis - (Kosten - AfA) - Werbungskosten`. Darf negativ sein. Cent statt Euro, weil die Freigrenze
/// ([`p23_freigrenze_je_vz`]) bei 999,99 und 1.000,00 Euro kippt: ein Abrunden jedes Postens auf volle Euro verschoebe den Gewinn.
///
/// # Errors
/// [`EngineFehler::Basis`] bei Ueberlauf jenseits von `i64`.
///
/// ```
/// # use engine::zugriff::teil2::privatverkauf::p23_gewinn_nach_afa;
/// # use domain::Cent;
/// // 200.000,00 - (150.000,00 - 10.000,00) - 5.000,00 Euro
/// let g = p23_gewinn_nach_afa(Cent::new(20_000_000), Cent::new(15_000_000), Cent::new(1_000_000), Cent::new(500_000)).unwrap();
/// assert_eq!(g, Cent::new(5_500_000));
/// ```
pub fn p23_gewinn_nach_afa(
    preis: Cent,
    anschaffungskosten: Cent,
    afa: Cent,
    werbungskosten: Cent,
) -> Result<Cent, EngineFehler> {
    let z = |c: Cent| i128::from(c.get());
    cent(z(preis) - (z(anschaffungskosten) - z(afa)) - z(werbungskosten))
}

/// § 23 Abs. 3 S. 5: Freigrenze je Veranlagungsjahr aus `params/<vz>/privatverkauf_p23.yaml`. Ein Gesamtgewinn unter der Grenze
/// bleibt steuerfrei (0); ab der Grenze zaehlt der GANZE Gesamtgewinn. CENT: 999,99 Euro sind steuerfrei, 1.000,00 Euro nicht.
///
/// # Errors
/// [`EngineFehler::Basis`], wenn das Jahr keine Parameterdatei hat, oder bei Ueberlauf.
///
/// ```
/// # use engine::zugriff::teil2::privatverkauf::p23_freigrenze_je_vz;
/// # use domain::{Cent, Vz};
/// # let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// assert_eq!(p23_freigrenze_je_vz(Cent::new(99_999), Vz::Vz2025, &p).unwrap(), Cent::new(0));
/// assert_eq!(p23_freigrenze_je_vz(Cent::new(100_000), Vz::Vz2025, &p).unwrap(), Cent::new(100_000));
/// ```
pub fn p23_freigrenze_je_vz(
    gesamtgewinn: Cent,
    vz: Vz,
    p: &Params,
) -> Result<Cent, EngineFehler> {
    let grenze = p.privatverkauf_freigrenze(vz)?.to_cent().map_err(EngineFehler::from)?;
    Ok(if gesamtgewinn >= grenze {
        gesamtgewinn
    } else {
        Cent::new(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(s: &str) -> Tag {
        Tag::aus_tt_mm_jjjj(s).unwrap_or_else(|| panic!("{s} ist kein Tag"))
    }

    fn grundstueck(kauf: &str, verkauf: &str) -> VerkaufEingabe {
        VerkaufEingabe {
            art: Art::Grundstueck,
            anschaffung: tag(kauf),
            veraeusserung: tag(verkauf),
            eigennutzung: Eigennutzung::Nein,
            taeglicher_gebrauch: false,
            einkunftsquelle: false,
        }
    }

    fn wg(kauf: &str, verkauf: &str) -> VerkaufEingabe {
        VerkaufEingabe {
            art: Art::AnderesWg,
            ..grundstueck(kauf, verkauf)
        }
    }

    /// Hand gerechnet aus "nicht mehr als zehn Jahre": Kauf 15.06.2015, die Frist endet mit Ablauf des 15.06.2025.
    #[test]
    fn grundstueck_zehn_jahre_grenztag() {
        assert_eq!(p23_befreiung(&grundstueck("15.06.2015", "15.06.2025")), Befreiung::Steuerpflichtig);
        assert_eq!(p23_befreiung(&grundstueck("15.06.2015", "14.06.2025")), Befreiung::Steuerpflichtig);
        assert_eq!(p23_befreiung(&grundstueck("15.06.2015", "16.06.2025")), Befreiung::FristAbgelaufen);
    }

    /// Kauf am Schalttag 29.02.2016: im Zieljahr 2026 gibt es keinen 29. Februar, die Frist endet am 28.02.2026.
    #[test]
    fn grundstueck_schalttag_endet_am_letzten_februartag() {
        assert_eq!(p23_befreiung(&grundstueck("29.02.2016", "28.02.2026")), Befreiung::Steuerpflichtig);
        assert_eq!(p23_befreiung(&grundstueck("29.02.2016", "01.03.2026")), Befreiung::FristAbgelaufen);
        // Zieljahr mit Schalttag: 29.02.2016 + 4 Jahre waere der 29.02.2020 (hier ueber frist_ende geprueft).
        assert_eq!(tag("29.02.2016").frist_ende(4), tag("29.02.2020"));
        // Kauf am 28.02. eines Nicht-Schaltjahrs: Fristende 28.02., der 01.03. liegt ausserhalb.
        assert_eq!(p23_befreiung(&grundstueck("28.02.2015", "28.02.2025")), Befreiung::Steuerpflichtig);
        assert_eq!(p23_befreiung(&grundstueck("28.02.2015", "01.03.2025")), Befreiung::FristAbgelaufen);
    }

    #[test]
    fn anderes_wg_ein_jahr_und_einkunftsquelle_zehn_jahre() {
        assert_eq!(p23_befreiung(&wg("10.01.2025", "10.01.2026")), Befreiung::Steuerpflichtig);
        assert_eq!(p23_befreiung(&wg("10.01.2025", "11.01.2026")), Befreiung::FristAbgelaufen);
        assert_eq!(p23_befreiung(&wg("29.02.2020", "28.02.2021")), Befreiung::Steuerpflichtig);
        assert_eq!(p23_befreiung(&wg("29.02.2020", "01.03.2021")), Befreiung::FristAbgelaufen);
        let mut quelle = wg("10.01.2025", "11.01.2026");
        quelle.einkunftsquelle = true;
        assert_eq!(p23_befreiung(&quelle), Befreiung::Steuerpflichtig);
        quelle.veraeusserung = tag("10.01.2035");
        assert_eq!(p23_befreiung(&quelle), Befreiung::Steuerpflichtig);
        quelle.veraeusserung = tag("11.01.2035");
        assert_eq!(p23_befreiung(&quelle), Befreiung::FristAbgelaufen);
    }

    #[test]
    fn taeglicher_gebrauch_befreit_nur_andere_wirtschaftsgueter() {
        let mut auto = wg("10.01.2025", "01.03.2025");
        auto.taeglicher_gebrauch = true;
        assert_eq!(p23_befreiung(&auto), Befreiung::TaeglicherGebrauch);
        // Beim Grundstueck gibt es diese Ausnahme nicht.
        let mut haus = grundstueck("10.01.2025", "01.03.2025");
        haus.taeglicher_gebrauch = true;
        assert_eq!(p23_befreiung(&haus), Befreiung::Steuerpflichtig);
    }

    #[test]
    fn eigennutzung_befreit_nur_grundstuecke_beide_varianten() {
        for variante in [Eigennutzung::Ausschliesslich, Eigennutzung::LetzteDreiJahre] {
            let mut haus = grundstueck("10.01.2025", "01.03.2025");
            haus.eigennutzung = variante;
            assert_eq!(p23_befreiung(&haus), Befreiung::Eigennutzung, "{variante:?}");
            // Beim anderen Wirtschaftsgut wird die Angabe nicht gelesen.
            let mut ding = wg("10.01.2025", "01.03.2025");
            ding.eigennutzung = variante;
            assert_eq!(p23_befreiung(&ding), Befreiung::Steuerpflichtig, "{variante:?}");
        }
    }

    /// Unentgeltlicher Erwerb: der Erblasser kaufte 2010, der Erbe verkauft im Maerz 2025 (nach etwa drei Jahren Besitz). Mit dem Datum
    /// des Erblassers ist die Frist laengst abgelaufen; mit dem eigenen Erbdatum waere der Verkauf faelschlich steuerpflichtig.
    #[test]
    fn unentgeltlicher_erwerb_laeuft_ab_dem_datum_des_rechtsvorgaengers() {
        assert_eq!(p23_befreiung(&grundstueck("01.09.2010", "20.03.2025")), Befreiung::FristAbgelaufen);
        assert_eq!(p23_befreiung(&grundstueck("15.03.2022", "20.03.2025")), Befreiung::Steuerpflichtig);
    }

    #[test]
    fn tag_liest_nur_echte_kalendertage() {
        assert_eq!(Tag::aus_tt_mm_jjjj("29.02.2024"), Some(Tag { jahr: 2024, monat: 2, tag: 29 }));
        for schlecht in ["29.02.2025", "31.04.2025", "00.01.2025", "01.13.2025", "1.1.2025", "2025-06-15", "15.06.25", "15/06/2025", "15.06.1899", "15.06.2101", "ab.06.2025", ""] {
            assert_eq!(Tag::aus_tt_mm_jjjj(schlecht), None, "{schlecht:?}");
        }
        // 1900 ist kein Schaltjahr, 2000 schon.
        assert_eq!(Tag::aus_tt_mm_jjjj("29.02.1900"), None);
        assert!(Tag::aus_tt_mm_jjjj("29.02.2000").is_some());
    }

    #[test]
    fn gewinn_mindert_die_kosten_um_die_afa() {
        let e = |preis, kosten, afa, wk| p23_gewinn_nach_afa(Cent::new(preis), Cent::new(kosten), Cent::new(afa), Cent::new(wk)).unwrap();
        assert_eq!(e(20_000_000, 15_000_000, 0, 500_000), Cent::new(4_500_000));
        assert_eq!(e(20_000_000, 15_000_000, 1_000_000, 500_000), Cent::new(5_500_000));
        assert_eq!(e(100_000, 120_000, 0, 5_000), Cent::new(-25_000));
        // Cent-genau: 100.000,00 - 99.000,01 = 999,99 Euro (nicht 1.000 durch Abrunden der Kosten auf 99.000)
        assert_eq!(e(10_000_000, 9_900_001, 0, 0), Cent::new(99_999));
    }

    #[test]
    fn freigrenze_kommt_aus_dem_jahr_und_ist_eine_grenze() {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let p = Params::lade(&wurzel).unwrap();
        for vz in [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026] {
            let f = |c: i64| p23_freigrenze_je_vz(Cent::new(c), vz, &p).unwrap();
            assert_eq!(f(99_999), Cent::new(0), "{vz:?}: 999,99 Euro");
            assert_eq!(f(100_000), Cent::new(100_000), "{vz:?}: 1.000,00 Euro");
            assert_eq!(f(100_001), Cent::new(100_001), "{vz:?}: 1.000,01 Euro");
            assert_eq!(f(-500_000), Cent::new(0), "{vz:?}: Verlust");
        }
    }
}
