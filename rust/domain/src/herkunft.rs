//! Herkunfts-Vektor (`store.py:60-72`, `meet_herkunft`) und Schreiber-Klassifikation
//! (`store.py:103-118,251-330`).
use std::fmt;
use std::str::FromStr;

use super::PruefTiefe;

/// Ein Wert auf einer Herkunfts-Achse (`herkunft` oder `haftung`): "berechnet", "mensch",
/// "edaten", "kontoauszug", "`llm_vorschlag`", "vorjahr", "system", "nutzer", "amt", ... oder der
/// Meet-Sentinel `"konflikt"` (`store.py: KONFLIKT = "konflikt"`) bei Uneinigkeit.
///
/// Bewusst KEIN geschlossenes `enum`: Python erlaubt neuen Schreibern (`elster_writer.py`,
/// `kontoauszug_writer.py`, ...) jederzeit einen neuen Achsenwert einzufuehren, ohne eine
/// zentrale Liste anzufassen. Ein geschlossenes Rust-`enum` waere bei jedem neuen Python-Writer
/// lautlos hinter der Wirklichkeit zurueckgeblieben — genau das Muster, das in diesem Repo schon
/// mehrfach zu stillen Verlusten gefuehrt hat (Achsenwert bleibt ein validierter, nicht-leerer
/// `String`, keine Aufzaehlung).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Achsenwert(String);

/// `Achsenwert::new` hat einen leeren String bekommen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("ein Herkunfts-Achsenwert darf nicht leer sein")]
pub struct LeererAchsenwert;

/// Der Meet-Sentinel bei Uneinigkeit auf einer Achse (`store.py: KONFLIKT`).
pub const KONFLIKT: &str = "konflikt";

impl Achsenwert {
    /// # Errors
    /// [`LeererAchsenwert`], wenn `s` leer ist.
    pub fn new(s: impl Into<String>) -> Result<Self, LeererAchsenwert> {
        let s = s.into();
        if s.is_empty() {
            return Err(LeererAchsenwert);
        }
        Ok(Self(s))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn ist_konflikt(&self) -> bool {
        self.0 == KONFLIKT
    }
}

impl fmt::Display for Achsenwert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Herkunfts-Vektor eines Feldwerts: drei Achsen, jede fuer sich gemeinsam mit `meet_herkunft`
/// zusammengefuehrt (`store.py:60-72`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Herkunft {
    pub herkunft: Achsenwert,
    pub pruef_tiefe: PruefTiefe,
    pub haftung: Achsenwert,
}

/// Klassifikation des `schreiber`-Strings eines Events (`store.py:251-330`, Auflage A/B/K1/F2).
/// Wire-Format ist ein Praefix-Code; alles andere ist ein Mensch mit seinem Namen/seiner Id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Schreiber {
    /// `llm:<rest>`, z. B. `llm:chat`.
    Llm(String),
    /// `berechnet:<rest>`, z. B. `berechnet:maps` (Entfernungspauschale).
    Berechnet(String),
    /// `import:beleg` — ein Beleg-Import; kann strukturell nie direkt bestaetigen.
    ImportBeleg,
    /// `import:vorjahr` — eine Vorjahres-Uebernahme; bestaetigt nie direkt.
    ImportVorjahr,
    /// `import:kontoauszug` — eine Kontoauszug-Klassifikation; bestaetigt nie direkt.
    ImportKontoauszug,
    /// `import:elster` — eDaten-Import (§ 150 Abs. 7 AO); schreibt direkt `bestaetigt`.
    ImportElster,
    /// `engine` — ringintern berechneter/abgeleiteter Wert (Verpflegungskuerzung, § 35a-Summen, ...).
    Engine,
    /// `abgeleitet:<rest>`, z. B. `abgeleitet:beweist`, `abgeleitet:ableitung`.
    Abgeleitet(String),
    /// Alles andere: ein Mensch, mit dem rohen Schreiber-String als Kennung.
    Mensch(String),
}

impl Schreiber {
    /// Der Vorschlags-Katalog-Typ dieses Schreibers (`llm`/`beleg`/`kontoauszug`/`maps`), sonst
    /// `None` (`store.py:111-118`, `_vorschlag_typ`). `import:vorjahr`/`Mensch`/`import:elster`
    /// sind NICHT katalog-restringiert (eigener bestaetigter Wert bzw. echter Kanal).
    #[must_use]
    pub fn vorschlag_typ(&self) -> Option<&'static str> {
        match self {
            Self::Llm(_) => Some("llm"),
            Self::ImportBeleg => Some("beleg"),
            Self::ImportKontoauszug => Some("kontoauszug"),
            Self::Berechnet(_) => Some("maps"),
            _ => None,
        }
    }
}

impl fmt::Display for Schreiber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Llm(rest) => write!(f, "llm:{rest}"),
            Self::Berechnet(rest) => write!(f, "berechnet:{rest}"),
            Self::ImportBeleg => write!(f, "import:beleg"),
            Self::ImportVorjahr => write!(f, "import:vorjahr"),
            Self::ImportKontoauszug => write!(f, "import:kontoauszug"),
            Self::ImportElster => write!(f, "import:elster"),
            Self::Engine => write!(f, "engine"),
            Self::Abgeleitet(rest) => write!(f, "abgeleitet:{rest}"),
            Self::Mensch(roh) => write!(f, "{roh}"),
        }
    }
}

impl FromStr for Schreiber {
    type Err = std::convert::Infallible;

    /// Jeder String ist ein gueltiger Schreiber (im Zweifel `Mensch`) — parst nie fehl, damit
    /// unbekannte Praefixe fail-closed als Mensch behandelt werden statt die Deserialisierung
    /// eines ganzen Events zu verwerfen.
    ///
    /// ```
    /// use domain::Schreiber;
    /// assert_eq!("llm:chat".parse::<Schreiber>().unwrap().to_string(), "llm:chat");
    /// assert_eq!("import:beleg".parse::<Schreiber>().unwrap().to_string(), "import:beleg");
    /// assert_eq!("julius".parse::<Schreiber>().unwrap().to_string(), "julius");
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(if let Some(rest) = s.strip_prefix("llm:") {
            Self::Llm(rest.to_owned())
        } else if let Some(rest) = s.strip_prefix("berechnet:") {
            Self::Berechnet(rest.to_owned())
        } else if s == "import:beleg" {
            Self::ImportBeleg
        } else if s == "import:vorjahr" {
            Self::ImportVorjahr
        } else if s == "import:kontoauszug" {
            Self::ImportKontoauszug
        } else if s == "import:elster" {
            Self::ImportElster
        } else if s == "engine" {
            Self::Engine
        } else if let Some(rest) = s.strip_prefix("abgeleitet:") {
            Self::Abgeleitet(rest.to_owned())
        } else {
            Self::Mensch(s.to_owned())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Schreiber;

    #[test]
    fn vorschlag_typ_matcht_store_py_vorschlag_typ() {
        assert_eq!("llm:chat".parse::<Schreiber>().unwrap().vorschlag_typ(), Some("llm"));
        assert_eq!("import:beleg".parse::<Schreiber>().unwrap().vorschlag_typ(), Some("beleg"));
        assert_eq!(
            "import:kontoauszug".parse::<Schreiber>().unwrap().vorschlag_typ(),
            Some("kontoauszug")
        );
        assert_eq!("berechnet:maps".parse::<Schreiber>().unwrap().vorschlag_typ(), Some("maps"));
        assert_eq!("import:vorjahr".parse::<Schreiber>().unwrap().vorschlag_typ(), None);
        assert_eq!("import:elster".parse::<Schreiber>().unwrap().vorschlag_typ(), None);
        assert_eq!("engine".parse::<Schreiber>().unwrap().vorschlag_typ(), None);
        assert_eq!("julius".parse::<Schreiber>().unwrap().vorschlag_typ(), None);
    }

    #[test]
    fn display_from_str_roundtrip_fuer_alle_varianten() {
        for s in [
            "llm:chat",
            "berechnet:maps",
            "import:beleg",
            "import:vorjahr",
            "import:kontoauszug",
            "import:elster",
            "engine",
            "abgeleitet:beweist",
            "julius@example.com",
        ] {
            let sch: Schreiber = s.parse().unwrap();
            assert_eq!(sch.to_string(), s);
        }
    }
}
