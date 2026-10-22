//! Buchungen des Bankimports: das Datum einer Buchung und ihr Posten im Ereignis.
//!
//! Gelesen werden vier Schreibweisen, die in den nachgebauten Bankdateien vorkommen
//! (Vault `research/bankimport-entwurf-version-1-…`, Messung M1): `dd.mm.yyyy`, `dd.mm.yy`,
//! `yyyy-mm-dd` und `yyyy-mm-dd hh:mm:ss`. Was keine dieser Formen hat, wird NICHT geraten —
//! die Uebernahme laesst die Buchung weg und zaehlt sie (`datum_unlesbar`). Ein zweistelliges
//! Jahr ist 2000 plus die Zahl (`25` ist 2025); ein Auszug aus dem 20. Jahrhundert ist kein Fall.
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
        if text.split_once('-').is_some_and(|(jahr, _)| ziffern(jahr, 4)) {
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
            jahr: if kurz { 2000 + jahr } else { jahr },
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::Datum;

    /// Das erwartete Ergebnis des Lesers; `Option`, weil `aus_json` eine liefert.
    #[allow(clippy::unnecessary_wraps)]
    fn d(jahr: i64, monat: u32, tag: u32) -> Option<Datum> {
        Some(Datum { jahr, monat, tag })
    }

    /// Die vier gelesenen Formen und ihre Raender.
    #[test]
    fn die_vier_formen_werden_gelesen() {
        let lies = |s: &str| Datum::aus_json(&json!(s));
        assert_eq!(lies("15.03.2025"), d(2025, 3, 15));
        assert_eq!(lies("01.02.2025"), d(2025, 2, 1));
        assert_eq!(lies("15.03.25"), d(2025, 3, 15));
        assert_eq!(lies("15.03.99"), d(2099, 3, 15), "2000 plus yy, kein Jahrhundertraten");
        assert_eq!(lies("15.03.00"), d(2000, 3, 15));
        assert_eq!(lies("2025-03-15"), d(2025, 3, 15));
        assert_eq!(lies("2025-03-15 10:00:05"), d(2025, 3, 15));
        assert_eq!(lies("  2025-03-15  "), d(2025, 3, 15), "Leerraum am Rand");
        assert_eq!(lies("31.12.2025"), d(2025, 12, 31));
        assert_eq!(lies("01.01.2025"), d(2025, 1, 1));
    }

    /// Alles andere wird NICHT geraten, auch nicht, wenn es fast passt.
    #[test]
    fn alles_andere_wird_nicht_geraten() {
        let lies = |s: &str| Datum::aus_json(&json!(s));
        for roh in [
            "", " ", "15.03", "15.3.2025", "1.3.25", "15.03.202", "15.03.20255", "15/03/2025",
            "15032025", "Sa 15.03.2025", "15.03.2025 10:00", "15.03.-2025", "480,00", "2025-03",
            "2025-3-15", "2025-03-5", "25-03-15", "2025-03-15T10:00:05", "2025-03-15 10:00",
            "2025-03-15 10:00:05.123", "2025-03-15 aa:bb:cc", "2025-13-01", "2025-00-10",
            "2025-03-32", "2025-03-00", "00.01.2025", "32.01.2025", "15.13.2025", "15.00.2025",
            "+5.03.2025", "１５.03.2025",
        ] {
            assert_eq!(lies(roh), None, "{roh:?}");
        }
        // Kein Text.
        for roh in [json!(null), json!(20_250_301), json!(5.5), json!(true), json!([]), json!({})] {
            assert_eq!(Datum::aus_json(&roh), None, "{roh}");
        }
    }

    #[test]
    fn das_datum_steht_in_der_akte_mit_fuehrenden_nullen() {
        assert_eq!(Datum { jahr: 2025, monat: 3, tag: 5 }.als_text(), "2025-03-05");
        assert_eq!(Datum { jahr: 2025, monat: 12, tag: 31 }.als_text(), "2025-12-31");
        // Rundreise: was `als_text` schreibt, liest der Leser wieder.
        let tag = Datum { jahr: 2025, monat: 1, tag: 9 };
        assert_eq!(Datum::aus_json(&json!(tag.als_text())), Some(tag));
    }
}
