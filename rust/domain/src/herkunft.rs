//! Herkunfts-Vektor (`store.py:60-72`, `meet_herkunft`) und Schreiber-Klassifikation
//! (`store.py:103-118,251-330`).
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

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

/// Wie im Wire-Format (`schema.json`): ein blanker String, keine geschachtelte Struktur.
impl Serialize for Achsenwert {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// Ein leerer Achsenwert ist beim Laden ein Deserialisierungsfehler, nicht erst beim ersten
/// `meet_herkunft` — dieselbe fail-closed-Disziplin wie [`Achsenwert::new`].
impl<'de> Deserialize<'de> for Achsenwert {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(s).map_err(serde::de::Error::custom)
    }
}

/// Herkunfts-Vektor eines Feldwerts: drei Achsen, jede fuer sich gemeinsam mit `meet_herkunft`
/// zusammengefuehrt (`store.py:60-72`). Feldnamen 1:1 `schema.json#/$defs/herkunft_vektor`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Herkunft {
    pub herkunft: Achsenwert,
    pub pruef_tiefe: PruefTiefe,
    pub haftung: Achsenwert,
}

/// Der `herkunft`-Vektor, wie ein Event ihn tatsaechlich TRAEGT — nicht nur, wie ein neuer
/// geschrieben wird ([`Herkunft`] bleibt dafuer die strenge Form). Gemessen ueber alle 192
/// realen Fallakten unter `~/.local/share/taxgraph/faelle/` (Zaehlung, keine Werte extrahiert):
/// 10.304 Events tragen alle drei Schluessel (`Voll`); 990 Events in 32 Dateien tragen exakt
/// `{"herkunft": "<wert>"}` ohne `pruef_tiefe`/`haftung` (`Alt`, eine fruehere Store-Schema-
/// Version) — keine dritte oder gemischte Form kommt vor.
///
/// `store.py::lade` (Python) hat auf dem Lesepfad keinen Schema-Validator und akzeptiert beide
/// Formen stillschweigend; `#[serde(untagged)]` bildet das nach — `Voll` zuerst versucht, `Alt`
/// nur, wenn `Voll` an einem fehlenden Feld scheitert. `Alt` traegt `#[serde(deny_unknown_fields)]`,
/// damit ein dritter, noch unbekannter Schluesselsatz explizit als Ladefehler auffaellt statt
/// lautlos als `Alt` fehlgedeutet zu werden.
///
/// `Voll` serialisiert ueber [`Herkunft`]s eigenes `Serialize` (alle drei Felder); `Alt`
/// serialisiert als struct-Variante unter `untagged` zu exakt `{"herkunft":"<wert>"}` — byte-
/// identisch zur eingelesenen Form, damit `event_id` (`sha256(canonical_json(payload))`) fuer
/// unveraenderte Events stabil bleibt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HerkunftVektor {
    Voll(Herkunft),
    Alt(HerkunftAlt),
}

/// Die Alt-Form von [`HerkunftVektor`]: nur die `herkunft`-Achse, ohne `pruef_tiefe`/`haftung`
/// (s. dortige Doku fuer die gemessenen Zahlen). Eigener Typ statt einer struct-Variante, weil
/// `#[serde(deny_unknown_fields)]` nur auf einem Container sitzen darf, nicht auf einer
/// Enum-Variante direkt.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HerkunftAlt {
    pub herkunft: Achsenwert,
}

impl HerkunftVektor {
    /// Die `herkunft`-Achse — die einzige, die BEIDE Formen tragen (`traverser.py` liest ueber
    /// `ev["herkunft"]["herkunft"]`, unabhaengig von der Form).
    #[must_use]
    pub fn herkunft_achse(&self) -> &Achsenwert {
        match self {
            Self::Voll(h) => &h.herkunft,
            Self::Alt(a) => &a.herkunft,
        }
    }

    /// `Some`, wenn der Vektor die volle Form traegt (alle drei Achsen) — `None` fuer `Alt`, dem
    /// `pruef_tiefe`/`haftung` fehlen.
    #[must_use]
    pub fn als_voll(&self) -> Option<&Herkunft> {
        match self {
            Self::Voll(h) => Some(h),
            Self::Alt(_) => None,
        }
    }
}

impl From<Herkunft> for HerkunftVektor {
    fn from(h: Herkunft) -> Self {
        Self::Voll(h)
    }
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
            Self::ImportVorjahr
            | Self::ImportElster
            | Self::Engine
            | Self::Abgeleitet(_)
            | Self::Mensch(_) => None,
        }
    }
}

/// Wire-Format ist der rohe `schreiber`-String (`schema.json`: `"type": "string"`), ueber
/// [`Display`](fmt::Display) serialisiert.
impl Serialize for Schreiber {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

/// `FromStr::Err` ist [`std::convert::Infallible`] — jeder String parst (im Zweifel als
/// `Mensch`), das Laden eines Events kann an dieser Stelle nie scheitern.
impl<'de> Deserialize<'de> for Schreiber {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(s.parse::<Self>()
            .unwrap_or_else(|unmoeglich: std::convert::Infallible| match unmoeglich {}))
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
        assert_eq!(
            "llm:chat".parse::<Schreiber>().unwrap().vorschlag_typ(),
            Some("llm")
        );
        assert_eq!(
            "import:beleg".parse::<Schreiber>().unwrap().vorschlag_typ(),
            Some("beleg")
        );
        assert_eq!(
            "import:kontoauszug"
                .parse::<Schreiber>()
                .unwrap()
                .vorschlag_typ(),
            Some("kontoauszug")
        );
        assert_eq!(
            "berechnet:maps"
                .parse::<Schreiber>()
                .unwrap()
                .vorschlag_typ(),
            Some("maps")
        );
        assert_eq!(
            "import:vorjahr"
                .parse::<Schreiber>()
                .unwrap()
                .vorschlag_typ(),
            None
        );
        assert_eq!(
            "import:elster"
                .parse::<Schreiber>()
                .unwrap()
                .vorschlag_typ(),
            None
        );
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

    #[test]
    fn schreiber_und_herkunft_serialisieren_als_wire_format() {
        use super::{Achsenwert, Herkunft};
        use crate::PruefTiefe;

        let sch: Schreiber = "llm:chat".parse().unwrap();
        assert_eq!(serde_json::to_string(&sch).unwrap(), "\"llm:chat\"");
        let zurueck: Schreiber = serde_json::from_str("\"llm:chat\"").unwrap();
        assert_eq!(zurueck, sch);

        let h = Herkunft {
            herkunft: Achsenwert::new("llm_vorschlag").unwrap(),
            pruef_tiefe: PruefTiefe::Ungeprueft,
            haftung: Achsenwert::new("system").unwrap(),
        };
        let json = serde_json::to_value(&h).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"herkunft":"llm_vorschlag","pruef_tiefe":"ungeprueft","haftung":"system"})
        );
        let zurueck: Herkunft = serde_json::from_value(json).unwrap();
        assert_eq!(zurueck, h);

        assert!(serde_json::from_str::<Achsenwert>("\"\"").is_err());
    }
}
