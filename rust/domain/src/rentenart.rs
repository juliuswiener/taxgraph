//! Rentenart aus der Bindung: `rentner_renten_art` (`bindung_rentner.yaml:60`) und
//! `rentner_renten_art_partner` (`:126`), beide `typ: enum`.
//!
//! Ein anderer Begriff als `engine::zugriff::teil2::rente::Rentenart`: dort steht die
//! Besteuerungsklasse (`Aa`, `Bb`, nicht ring-faehig), auf die mehrere dieser Werte fallen.

/// Die Art einer Rente, wie der Nutzer sie angibt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rentenart {
    GesetzlicheRente,
    BerufsstaendischeVersorgung,
    PrivateBasisrente,
    PrivateLeibrente,
    SonstigeLeibrente,
}

impl Rentenart {
    /// Alle Werte in der Reihenfolge der `enum_werte`.
    pub const ALLE: [Self; 5] = [
        Self::GesetzlicheRente,
        Self::BerufsstaendischeVersorgung,
        Self::PrivateBasisrente,
        Self::PrivateLeibrente,
        Self::SonstigeLeibrente,
    ];

    /// Der Bindungswert.
    ///
    /// ```
    /// assert_eq!(domain::Rentenart::PrivateLeibrente.als_str(), "private_leibrente");
    /// ```
    #[must_use]
    pub const fn als_str(self) -> &'static str {
        match self {
            Self::GesetzlicheRente => "gesetzliche_rente",
            Self::BerufsstaendischeVersorgung => "berufsstaendische_versorgung",
            Self::PrivateBasisrente => "private_basisrente",
            Self::PrivateLeibrente => "private_leibrente",
            Self::SonstigeLeibrente => "sonstige_leibrente",
        }
    }
}
