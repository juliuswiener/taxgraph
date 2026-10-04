//! Fehlerpfade und selten gewaehlte Zweige von `domain`, die kein Bestandstest festnagelt (Mutationsmessung N4f, Bericht
//! `mutation-bindung-domain`, Abschnitt N4f). Jeder Test steht fuer Mutanten, die im Standardlauf (ohne `PARITY=1`, ohne Python)
//! gruen blieben: ein Fehler wird mit `.unwrap_or_default()` verschluckt (`?` ersetzt), ein `match`-Zweig faellt aus oder liefert
//! den Fehler statt des Werts.
//!
//! * `Deserialize` von `Achsenwert`, `Kz` und `Schreiber`: ein Nicht-Text ist ein Fehler des Deserializers, kein leerer Text.
//!   `Schreiber` parst jeden Text; ein verschluckter Fehler machte aus `5` ein `Mensch("")`.
//! * `Deserialize` von `PyWert`: ein Fehler mitten in einer Liste oder einem Objekt bricht ab. Ein eigener Deserializer, weil
//!   `serde_json` nach einem verschluckten Fehler selbst noch einmal scheitert (mit anderem Text).
//! * `PyWert::int_dezimal` und `PyWert::dezimal` fuer `Ganz`; `PyWert::py_eq` fuer zwei Gleitkommazahlen.
//! * `Wert::aus_pywert`: ein Nicht-Text bei `Feldtyp::Enum` und `Feldtyp::Datum` ist `TypInkonform`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::fmt;

use domain::{Achsenwert, Feldtyp, Kz, PyWert, Schreiber, Wert, WertFehler};
use rust_decimal::Decimal;
use serde::de::value::{Error as DeFehler, I64Deserializer, StrDeserializer};
use serde::de::{
    self, DeserializeOwned, DeserializeSeed, Deserializer, IntoDeserializer, MapAccess, SeqAccess,
    Visitor,
};
use serde::Deserialize;

/// Fehlertext von `serde_json`, wenn `text` nicht als `T` gelesen werden kann.
fn json_fehler<T: DeserializeOwned>(text: &str) -> String {
    match serde_json::from_str::<T>(text) {
        Ok(_) => panic!("{text} wurde als {} gelesen", std::any::type_name::<T>()),
        Err(e) => e.to_string(),
    }
}

#[test]
fn deserialize_eines_nicht_textes_meldet_den_typfehler_des_deserializers() {
    for (json, was) in [
        ("5", "integer `5`"),
        ("null", "null"),
        ("true", "boolean `true`"),
        ("[]", "sequence"),
    ] {
        let soll = format!("invalid type: {was}, expected a string");
        for (typ, fehler) in [
            ("Achsenwert", json_fehler::<Achsenwert>(json)),
            ("Kz", json_fehler::<Kz>(json)),
            ("Schreiber", json_fehler::<Schreiber>(json)),
        ] {
            assert!(
                fehler.starts_with(&soll),
                "{typ} aus {json}: {fehler:?} soll mit {soll:?} beginnen"
            );
        }
    }
    // Gegenprobe: ein Text wird gelesen, nur der Nicht-Text scheitert.
    assert_eq!(
        serde_json::from_str::<Schreiber>("\"engine\"").unwrap(),
        Schreiber::Engine
    );
    assert_eq!(
        serde_json::from_str::<Kz>("\"E0100401\"").unwrap().as_str(),
        "E0100401"
    );
}

/// Liefert ein Element (`1`) bzw. einen Eintrag (`"a": 1`) und danach, je nach `ende`, das Ende oder einen Fehler.
struct Test {
    objekt: bool,
    ende: Ende,
}

#[derive(Clone, Copy)]
enum Ende {
    Ordentlich,
    Fehler,
}

impl<'de> Deserializer<'de> for Test {
    type Error = DeFehler;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, DeFehler> {
        if self.objekt {
            visitor.visit_map(Zaehler(0, self.ende))
        } else {
            visitor.visit_seq(Zaehler(0, self.ende))
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

struct Zaehler(u8, Ende);

impl Zaehler {
    fn weiter(&mut self) -> Result<bool, DeFehler> {
        self.0 += 1;
        match (self.0, self.1) {
            (1, _) => Ok(true),
            (_, Ende::Ordentlich) => Ok(false),
            (_, Ende::Fehler) => Err(de::Error::custom("kaputt mitten im Inhalt")),
        }
    }
}

impl<'de> SeqAccess<'de> for Zaehler {
    type Error = DeFehler;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, DeFehler> {
        if self.weiter()? {
            let d: I64Deserializer<DeFehler> = 1_i64.into_deserializer();
            seed.deserialize(d).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<'de> MapAccess<'de> for Zaehler {
    type Error = DeFehler;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, DeFehler> {
        if self.weiter()? {
            let d: StrDeserializer<DeFehler> = "a".into_deserializer();
            seed.deserialize(d).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, DeFehler> {
        let d: I64Deserializer<DeFehler> = 1_i64.into_deserializer();
        seed.deserialize(d)
    }
}

impl fmt::Debug for Test {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Test")
    }
}

#[test]
fn ein_fehler_mitten_in_liste_oder_objekt_bricht_das_lesen_ab() {
    // Gegenprobe: ohne Fehler liest derselbe Deserializer ein Element bzw. einen Eintrag.
    let ok = |objekt| {
        PyWert::deserialize(Test {
            objekt,
            ende: Ende::Ordentlich,
        })
        .unwrap()
    };
    assert_eq!(ok(false), PyWert::Liste(vec![PyWert::Ganz(1)]));
    assert_eq!(
        ok(true),
        PyWert::Objekt(vec![("a".to_owned(), PyWert::Ganz(1))])
    );
    // Mit Fehler nach dem ersten Inhalt: Fehler, kein gekuerztes Ergebnis.
    for objekt in [false, true] {
        let r = PyWert::deserialize(Test {
            objekt,
            ende: Ende::Fehler,
        });
        assert_eq!(
            r.unwrap_err().to_string(),
            "kaputt mitten im Inhalt",
            "objekt={objekt}"
        );
    }
}

#[test]
fn int_dezimal_und_dezimal_einer_ganzzahl() {
    for n in [0, 1, -1, 42, -42, i64::MIN, i64::MAX] {
        assert_eq!(PyWert::Ganz(n).int_dezimal(), Ok(n.to_string()), "{n}");
        assert_eq!(PyWert::Ganz(n).dezimal(), Ok(Decimal::from(n)), "{n}");
    }
}

/// `CPython`: `1.5 == 1.5` ist `True`, `0.0 == -0.0` ist `True`, `nan == nan` ist `False`, `inf == inf` ist `True`.
#[test]
fn py_eq_zweier_gleitkommazahlen() {
    let g = PyWert::Gleit;
    assert!(g(1.5).py_eq(&g(1.5)));
    assert!(!g(1.5).py_eq(&g(2.5)));
    assert!(g(0.0).py_eq(&g(-0.0)));
    assert!(!g(f64::NAN).py_eq(&g(f64::NAN)));
    assert!(g(f64::INFINITY).py_eq(&g(f64::INFINITY)));
    assert!(!g(f64::INFINITY).py_eq(&g(f64::NEG_INFINITY)));
}

#[test]
fn nicht_text_bei_enum_und_datum_ist_typinkonform() {
    // Leerer Text ist als Enum-Wert zugelassen: ein Nicht-Text darf nicht zu "" werden.
    let werte = vec![String::new(), "1".to_owned(), "01.01.2025".to_owned()];
    for (wert, repr) in [
        (PyWert::Ganz(1), "1"),
        (PyWert::Bool(true), "True"),
        (PyWert::Null, "None"),
        (PyWert::Gleit(1.5), "1.5"),
    ] {
        for (typ, name, enum_werte) in [
            (Feldtyp::Enum, "enum", Some(werte.as_slice())),
            (Feldtyp::Datum, "datum", None),
        ] {
            assert_eq!(
                Wert::aus_pywert(&wert, typ, enum_werte),
                Err(WertFehler::TypInkonform {
                    wert: repr.to_owned(),
                    typ: name
                }),
                "{repr} als {name}"
            );
        }
    }
}
