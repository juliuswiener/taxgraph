//! Konfession aus der Bindung: `kist_konfession` (`bindung_p51a_kirchensteuer.yaml:20`) und
//! `kist_konfession_partner` (`bindung_an_gesamt.yaml:1366`), beide `typ: enum`.

/// Die Laienauswahl, nicht der amtliche Religionsschluessel. „andere" hat dort bewusst keinen Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Konfession {
    Keine,
    Evangelisch,
    RoemischKatholisch,
    Andere,
}

impl Konfession {
    /// Alle Werte in der Reihenfolge der `enum_werte`.
    pub const ALLE: [Self; 4] = [
        Self::Keine,
        Self::Evangelisch,
        Self::RoemischKatholisch,
        Self::Andere,
    ];

    /// Der Bindungswert.
    ///
    /// ```
    /// assert_eq!(domain::Konfession::RoemischKatholisch.als_str(), "roemisch-katholisch");
    /// ```
    #[must_use]
    pub const fn als_str(self) -> &'static str {
        match self {
            Self::Keine => "keine",
            Self::Evangelisch => "evangelisch",
            Self::RoemischKatholisch => "roemisch-katholisch",
            Self::Andere => "andere",
        }
    }
}
