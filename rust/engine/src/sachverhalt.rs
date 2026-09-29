//! Entspricht dem Verzweigungsbaum von `runner.catala_est`, aber typisiert statt roher
//! Sachverhalt-Dicts: jede Variante traegt die bereits fertig berechnete Eingabe fuer GENAU
//! einen Catala-Scope-Aufruf. Die vier rein-Python-Zweige (`sanierungsaufwendungen`,
//! `bruttolistenpreis`, `gewerbesteuer`, `koerperschaft`), die `catala_est` ebenfalls kennt,
//! rufen NIE Catala auf und sind deshalb hier nicht abgebildet.
use catala_sys::CatalaFehler;
use domain::Cent;

use crate::entfernungspauschale::{self, EntfernungspauschaleEingabe, EntfernungspauschaleFehler};
use crate::arbeitszimmer::{self, ArbeitszimmerEingabe};
use crate::tarif::{
    self, FestzusetzendeEstEinzelEingabe, FestzusetzendeEstZusammenEingabe, FuenftelEingabe,
    FuenftelFehler, GesamtEingabe, Veranlagung, Vz,
};

/// Vereinigung aller Fehlerquellen der einzelnen [`Sachverhalt`]-Zweige.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SachverhaltFehler {
    #[error(transparent)]
    Entfernungspauschale(#[from] EntfernungspauschaleFehler),
    #[error(transparent)]
    Fuenftel(#[from] FuenftelFehler),
    #[error(transparent)]
    Catala(#[from] CatalaFehler),
}

/// Ein Catala-rufender Zweig von `runner.catala_est`, mit bereits fertig berechneter Eingabe.
pub enum Sachverhalt {
    Gesamtfall {
        eingabe: GesamtEingabe,
        vz: Vz,
        zusammenveranlagung: bool,
    },
    Entfernungspauschale(EntfernungspauschaleEingabe),
    Arbeitszimmer(ArbeitszimmerEingabe),
    BruttoarbeitslohnZusammen {
        eingabe: FestzusetzendeEstZusammenEingabe,
        vz: Vz,
    },
    Bruttoarbeitslohn {
        eingabe: FestzusetzendeEstEinzelEingabe,
        vz: Vz,
    },
    AusserordentlicheEinkuenfte(FuenftelEingabe),
    Tarif {
        zve: Cent,
        vz: Vz,
        veranlagung: Veranlagung,
    },
}

impl Sachverhalt {
    /// Rechnet den Zweig aus, immer in Cent -- die Python-Vorlage `catala_est` mischt EURO/CENT
    /// zwischen Zweigen (einige Zweige geben CENT, andere `// 100`-EURO zurueck); diese Fassung
    /// normalisiert einheitlich auf Cent.
    ///
    /// # Errors
    /// Siehe [`SachverhaltFehler`].
    pub fn berechnen(self) -> Result<Cent, SachverhaltFehler> {
        match self {
            Self::Gesamtfall {
                eingabe,
                vz,
                zusammenveranlagung,
            } => {
                let ergebnis = if zusammenveranlagung {
                    tarif::festzusetzende_est_gesamt_zusammen(eingabe, vz)
                } else {
                    tarif::festzusetzende_est_gesamt(eingabe, vz)
                }?;
                Ok(Cent::new(ergebnis.festzusetzende_est_cent))
            }
            Self::Entfernungspauschale(eingabe) => {
                let ergebnis = entfernungspauschale::berechnen(eingabe)?;
                Ok(Cent::new(ergebnis.abziehbarer_betrag_cent))
            }
            Self::Arbeitszimmer(eingabe) => {
                let ergebnis = arbeitszimmer::berechnen(eingabe)?;
                Ok(Cent::new(ergebnis.abzug_gesamt_cent))
            }
            Self::BruttoarbeitslohnZusammen { eingabe, vz } => {
                Ok(tarif::festzusetzende_est_zusammen(eingabe, vz)?)
            }
            Self::Bruttoarbeitslohn { eingabe, vz } => {
                Ok(tarif::festzusetzende_est_einzel(eingabe, vz)?)
            }
            Self::AusserordentlicheEinkuenfte(eingabe) => Ok(tarif::fuenftel(eingabe)?),
            Self::Tarif {
                zve,
                vz,
                veranlagung,
            } => Ok(match veranlagung {
                Veranlagung::Einzel => tarif::grundtarif(zve, vz),
                Veranlagung::Zusammen => tarif::splittingtarif(zve, vz),
            }?),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Sachverhalt;
    use crate::tarif::{FestzusetzendeEstEinzelEingabe, Veranlagung, Vz};
    use domain::Cent;

    #[test]
    fn tarif_zweig_entspricht_direktem_grundtarif_aufruf() {
        let zve = Cent::new(5_000_000);
        let referenz = crate::tarif::grundtarif(zve, Vz::Vz2024).unwrap();
        let ergebnis = Sachverhalt::Tarif {
            zve,
            vz: Vz::Vz2024,
            veranlagung: Veranlagung::Einzel,
        }
        .berechnen()
        .unwrap();
        assert_eq!(ergebnis, referenz);
    }

    #[test]
    fn bruttoarbeitslohn_zweig_entspricht_direktem_aufruf() {
        let eingabe = FestzusetzendeEstEinzelEingabe {
            bruttoarbeitslohn: Cent::new(0),
            werbungskosten: Cent::new(0),
            sonderausgaben: Cent::new(0),
        };
        let referenz = crate::tarif::festzusetzende_est_einzel(eingabe, Vz::Vz2024).unwrap();
        let ergebnis = Sachverhalt::Bruttoarbeitslohn {
            eingabe,
            vz: Vz::Vz2024,
        }
        .berechnen()
        .unwrap();
        assert_eq!(ergebnis, referenz);
    }
}
