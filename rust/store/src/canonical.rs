//! Content-Adressierung (`store.py:20-38`): `event_id`/`snapshot_id` sind `sha256` ueber
//! `canonical_json`.
//!
//! `canonical_json` ist NICHT nachgebaut. In diesem Workspace hat `serde_json` kein
//! `preserve_order`-Feature (`rust/Cargo.lock`: kein `indexmap` unter `serde_json`) —
//! `serde_json::Map<String, Value>` ist also `BTreeMap`-gestuetzt und sortiert Schluessel bei
//! jeder Serialisierung automatisch, rekursiv. `serde_json::to_string` benutzt ausserdem
//! serienmaessig kompakte `,`/`:`-Trenner. Beides zusammen ist byte-identisch zu Pythons
//! `json.dumps(obj, sort_keys=True, ensure_ascii=False, separators=(",", ":"))` — empirisch
//! geprueft fuer Kontrollzeichen (`\n`, `\t`, `\u0001`), U+2028, Emoji, Anfuehrungszeichen,
//! Rueckwaertsschraegstrich, `\b`/`\f`, verschachtelte Objekt-Schluesselsortierung, Array-
//! Reihenfolge, negative Ganzzahlen, `null`, `bool`.
//!
//! PARITAET (Restrisiko, dokumentiert statt geloest): Gleitkommazahlen. `domain::Wert` kennt
//! keinen Float-Fall, und Auflage T weist einen Float fuer jedes GEBUNDENE `typ=cent/int`-Feld
//! zurueck (`domain::Wert::aus_pywert`). Ein Float kann store-seitig nur bei einem UNBEKANNTEN
//! `feld_id` auftreten (Auflage T laesst dann durch, s. `abweisung::pruefe_typ_konformitaet`).
//! Pythons `repr()`-Fliesskommaformatierung und `serde_json`s Float-Formatierung sind fuer diesen
//! Fall nicht Byte-fuer-Byte verglichen; ein Store mit einem Float auf einem ungebundenen Feld
//! ist ein dokumentierter Coverage-Gap, kein bekannter Fehler.
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Deterministische, sortierte, kompakte JSON-Serialisierung fuer die Content-Adressierung
/// (`store.py:22-24`: `canonical_json`).
///
/// ```
/// use store::canonical_json;
/// assert_eq!(canonical_json(&serde_json::json!({"b":1,"a":2})), r#"{"a":2,"b":1}"#);
/// ```
#[must_use]
pub fn canonical_json(value: &serde_json::Value) -> String {
    // serde_json::Value kann laut Invariante von `serde_json::Number` keinen NaN/Infinity-
    // Float tragen (`Number::from_f64` verweigert deren Konstruktion) -- der Err-Zweig ist
    // unerreichbar. Ohne unwrap/panic dokumentiert statt weggelassen.
    serde_json::to_string(value).unwrap_or_default()
}

/// sha256(text) als Hex-String (Kleinbuchstaben, wie Pythons `hexdigest()`).
///
/// ```
/// // python3 -c 'import hashlib; print(hashlib.sha256(b"abc").hexdigest())'
/// assert_eq!(
///     store::sha256_hex("abc"),
///     "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
/// );
/// ```
#[must_use]
pub fn sha256_hex(text: &str) -> String {
    use std::fmt::Write as _;
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hasher.finalize().iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

/// Ein Event- oder Snapshot-Id: `sha256(canonical_json(...))`, 32 Bytes (`schema.json`-Pattern
/// `^[a-f0-9]{64}$`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EventId([u8; 32]);

/// `EventId::parse` ist an einem Nicht-Hex-String oder einer falschen Laenge gescheitert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum EventIdFehler {
    #[error("event_id muss 64 Hex-Zeichen sein, war {0}")]
    FalscheLaenge(usize),
    #[error("event_id enthaelt ungueltige Hex-Zeichen")]
    UngueltigesHex,
}

impl EventId {
    /// ```
    /// let id = store::EventId::aus_bytes([0xab; 32]);
    /// assert_eq!(id.to_string(), "ab".repeat(32));
    /// ```
    #[must_use]
    pub fn aus_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// ```
    /// let id = store::EventId::aus_bytes([7; 32]);
    /// assert_eq!(id.als_bytes(), &[7; 32]);
    /// ```
    #[must_use]
    pub fn als_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// `sha256(canonical_json(payload))` — der EINE Content-Adressierungs-Mechanismus, den
    /// `event_id`/`snapshot_id` beide nutzen (`store.py:31-38`).
    ///
    /// ```
    /// use store::EventId;
    /// let a = EventId::von_json(&serde_json::json!({"a":1}));
    /// let b = EventId::von_json(&serde_json::json!({"a":1}));
    /// let c = EventId::von_json(&serde_json::json!({"a":2}));
    /// assert_eq!(a, b);
    /// assert_ne!(a, c);
    /// ```
    #[must_use]
    pub fn von_json(payload: &serde_json::Value) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(canonical_json(payload).as_bytes());
        Self(hasher.finalize().into())
    }

    /// # Errors
    /// [`EventIdFehler`], wenn `s` nicht aus genau 64 Hex-Zeichen besteht.
    ///
    /// ```
    /// use store::{EventId, EventIdFehler};
    /// let id = EventId::von_json(&serde_json::json!({"a": 1}));
    /// assert_eq!(EventId::parse(&id.to_string()), Ok(id));
    /// assert_eq!(EventId::parse("abc"), Err(EventIdFehler::FalscheLaenge(3)));
    /// assert_eq!(EventId::parse(&"zz".repeat(32)), Err(EventIdFehler::UngueltigesHex));
    /// ```
    pub fn parse(s: &str) -> Result<Self, EventIdFehler> {
        if s.len() != 64 {
            return Err(EventIdFehler::FalscheLaenge(s.len()));
        }
        let mut bytes = [0u8; 32];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let paar = std::str::from_utf8(chunk).map_err(|_| EventIdFehler::UngueltigesHex)?;
            let Some(byte) = bytes.get_mut(i) else {
                return Err(EventIdFehler::UngueltigesHex);
            };
            *byte = u8::from_str_radix(paar, 16).map_err(|_| EventIdFehler::UngueltigesHex)?;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for EventId {
    type Err = EventIdFehler;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Serialize for EventId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for EventId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_json, EventId};
    use serde_json::json;

    #[test]
    fn sortiert_schluessel_rekursiv_und_kompakt() {
        assert_eq!(
            canonical_json(&json!({"z":{"y":1,"x":2},"a":1})),
            r#"{"a":1,"z":{"x":2,"y":1}}"#
        );
    }

    #[test]
    fn control_chars_und_unicode_matchen_python_ensure_ascii_false() {
        // python3 -c 'import json; print(json.dumps({"s":"a\nb\tc\u0001d e😀"}, ensure_ascii=False, separators=(",",":")))'
        assert_eq!(
            canonical_json(&json!({"s": "a\nb\tc\u{1}d\u{2028}e😀"})),
            "{\"s\":\"a\\nb\\tc\\u0001d\u{2028}e😀\"}"
        );
    }

    #[test]
    fn event_id_roundtrip_hex() {
        let id = EventId::von_json(&json!({"a": 1}));
        let hex = id.to_string();
        assert_eq!(hex.len(), 64);
        assert_eq!(hex.parse::<EventId>().unwrap(), id);
    }

    #[test]
    fn event_id_ist_deterministisch_und_tamper_fest() {
        assert_eq!(
            EventId::von_json(&json!({"a": 1})),
            EventId::von_json(&json!({"a": 1}))
        );
        assert_ne!(
            EventId::von_json(&json!({"a": 1})),
            EventId::von_json(&json!({"a": 2}))
        );
    }
}
