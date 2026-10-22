//! Buchungen des Bankimports: das Datum einer Buchung und ihr Posten im Ereignis.
//!
//! Gelesen werden vier Schreibweisen, die in den nachgebauten Bankdateien vorkommen
//! (Vault `research/bankimport-entwurf-version-1-…`, Messung M1): `dd.mm.yyyy`, `dd.mm.yy`,
//! `yyyy-mm-dd` und `yyyy-mm-dd hh:mm:ss`. Was keine dieser Formen hat, wird NICHT geraten —
//! die Uebernahme laesst die Buchung weg und zaehlt sie (`datum_unlesbar`). Ein zweistelliges
//! Jahr wird am Jahrhundertwechsel gebunden: `25` ist 2025, `99` ist 1999.
use serde_json::Value;

/// Das Datum einer Buchung: Zahlentripel ohne Uhrzeit und ohne Zeitzone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Datum {
    pub jahr: i64,
    pub monat: u32,
    pub tag: u32,
}

impl Datum {
    /// Liest die vier Formen; alles andere ist `None`, auch ein Wert, der kein Text ist.
    ///
    /// ```
    /// use eingang::bank::buchung::Datum;
    /// let d = |s: &str| serde_json::json!(s);
    /// assert_eq!(Datum::aus_json(&d("15.03.2025")), Some(Datum { jahr: 2025, monat: 3, tag: 15 }));
    /// assert_eq!(Datum::aus_json(&d("15.03.25")), Some(Datum { jahr: 2025, monat: 3, tag: 15 }));
    /// assert_eq!(Datum::aus_json(&d("2025-03-15 10:00:05")), Some(Datum { jahr: 2025, monat: 3, tag: 15 }));
    /// assert_eq!(Datum::aus_json(&d("15.03")), None, "ohne Jahr wird nicht geraten");
    /// assert_eq!(Datum::aus_json(&d("2025-13-01")), None, "Monat 13 ist kein Datum");
    /// ```
    #[must_use]
    pub fn aus_json(wert: &Value) -> Option<Self> {
        let text = wert.as_str()?.trim();
        if text.len() >= 5
            && text.as_bytes()[..4].iter().all(|b| b.is_ascii_digit())
            && text.as_bytes()[4] == b'-'
        {
            return Self::iso(text);
        }
        let teile: Vec<&str> = text.split('.').collect();
        let [tag, monat, jahr] = teile.as_slice() else {
            return None;
        };
        let kurz = ziffern(jahr, 2);
        if !ziffern(tag, 2) || !ziffern(monat, 2) || !(kurz || ziffern(jahr, 4)) {
            return None;
        }
        let jahr: i64 = jahr.parse().ok()?;
        Some(Self {
            jahr: if kurz { if jahr <= 69 { jahr + 2000 } else { jahr + 1900 } } else { jahr },
            monat: monat.parse().ok()?,
            tag: tag.parse().ok()?,
        })
        .filter(Self::gueltig)
    }

    /// `yyyy-mm-dd`, optional mit ` hh:mm:ss` danach (Revolut nennt beide Felder so).
    fn iso(text: &str) -> Option<Self> {
        let (monat, rest) = text.get(5..)?.split_once('-')?;
        let (tag, zeit) = rest.split_once(' ').unwrap_or((rest, ""));
        if !ziffern(monat, 2) || !ziffern(tag, 2) {
            return None;
        }
        if !zeit.is_empty() {
            let stunden: Vec<&str> = zeit.split(':').collect();
            if stunden.len() != 3 || !stunden.iter().all(|s| ziffern(s, 2)) {
                return None;
            }
        }
        Some(Self {
            jahr: text.get(..4)?.parse().ok()?,
            monat: monat.parse().ok()?,
            tag: tag.parse().ok()?,
        })
        .filter(Self::gueltig)
    }

    /// Monat 1 bis 12, Tag 1 bis 31. Eine Tages- oder Monatszahl ausserhalb weist der Leser ab,
    /// statt sie in ein anderes Jahr zu schieben; die echteste Pruefung am Posten ist die Summe.
    fn gueltig(&self) -> bool {
        (1..=12).contains(&self.monat) && (1..=31).contains(&self.tag)
    }

    /// Wie in der Akte: `yyyy-mm-dd`.
    ///
    /// ```
    /// use eingang::bank::buchung::Datum;
    /// assert_eq!(Datum { jahr: 2025, monat: 3, tag: 5 }.als_text(), "2025-03-05");
    /// ```
    #[must_use]
    pub fn als_text(self) -> String {
        format!("{:04}-{:02}-{:02}", self.jahr, self.monat, self.tag)
    }
}

/// Genau `breite` ASCII-Ziffern.
fn ziffern(text: &str, breite: usize) -> bool {
    text.len() == breite && text.bytes().all(|b| b.is_ascii_digit())
}

/// Ein Posten, wie er in `signal_1` des Ereignisses steht. Der Zweck ist bereits maskiert;
/// Datum, Cent-Betrag und maskierter Zweck sind die Merkmale der Dublette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posten {
    pub datum: Datum,
    pub betrag_cent: i64,
    pub zweck_maskiert: String,
}

impl Posten {
    /// Der Schluessel der Mehrmenge: derselbe Posten in zwei Auszuegen ist dieselbe Buchung.
    ///
    /// ponytail: zwei Konten mit identischen Buchungen am selben Tag zaehlen einmal.
    /// Upgrade: Kontokennung in den Schluessel nehmen, sobald ein zweites Konto imported wird.
    #[must_use]
    pub fn schluessel(&self) -> (String, i64, String) {
        (self.datum.als_text(), self.betrag_cent, self.zweck_maskiert.clone())
    }
}
