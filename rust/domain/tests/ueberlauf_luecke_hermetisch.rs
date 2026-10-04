//! Ueberlaufstellen von `domain`, die kein Bestandstest erreicht, im Standardlauf (ohne `PARITY=1`, ohne Python). Bericht
//! h8-ueberlauf-luecke. Beide Stellen sind ueber die `pub`-Schnittstelle erreichbar:
//!
//! * `money.rs`, `Km::volle_km` (`i64::try_from(self.0.trunc())`): ein Kilometerwert ausserhalb `i64` ergibt `None` -- die Aufrufer
//!   (`ep_ab_21km`) melden daraus `Ueberlauf("km")`. Die Mutante (`d.mantissa() as i64`) liefert den gewickelten Wert.
//! * `py_wert.rs`, `visit_u128` (`i128::try_from(n)`): YAML liefert Ganzzahlen bis `u128`; ab `i128::MAX + 1` wird der Wert ein
//!   `Gleit` (ponytail an `Deserialize`: ausserhalb von `i128` nur noch als f64). Die Mutante (`n as i128`) wickelt `u128::MAX`
//!   auf `-1` und liefert `Ganz(-1)`. Python haelt die Zahl exakt (`int`): der Test pinnt die dokumentierte f64-Saettigung, kein
//!   Python-Orakel stuetzt den f64-Wert.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use domain::{Km, PyWert};
use rust_decimal::Decimal;

/// `Km::volle_km`: `i64::MAX` passt, `i64::MAX + 1` und `i64::MIN - 1` nicht; ein Bruchteil wird abgeschnitten, nicht gerundet.
#[test]
fn volle_km_ausserhalb_i64_ist_none_statt_gewickelt() {
    let km = |m: i128, skala: u32| Km::new(Decimal::from_i128_with_scale(m, skala)).volle_km();
    let max = i128::from(i64::MAX);
    let min = i128::from(i64::MIN);
    assert_eq!(km(max, 0), Some(i64::MAX), "i64::MAX passt gerade");
    assert_eq!(km(max + 1, 0), None, "i64::MAX + 1");
    assert_eq!(km(min, 0), Some(i64::MIN), "i64::MIN passt gerade");
    assert_eq!(km(min - 1, 0), None, "i64::MIN - 1");
    // 9223372036854775807,9 und 9223372036854775808,5 (Mantisse mal 10, Skala 1): trunc, dann Bereichspruefung
    assert_eq!(
        km(max * 10 + 9, 1),
        Some(i64::MAX),
        "i64::MAX,9 schneidet auf i64::MAX ab"
    );
    assert_eq!(km((max + 1) * 10 + 5, 1), None, "i64::MAX + 1,5");
}

/// `PyWert` aus YAML, `u128` ausserhalb `i128`: ein `Gleit`, nie ein gewickelter `Ganz`. Gegenproben: `i128::MAX` (als `u128`)
/// bleibt ein `Gleit` (ausserhalb `u64`), `u64::MAX` ein `GrossGanz`.
#[test]
fn yaml_u128_ausserhalb_i128_wird_ein_gleit_statt_gewickelt() {
    let yaml = |t: &str| serde_yaml_ng::from_str::<PyWert>(t).unwrap();
    let nach_f64 = |n: u128| {
        #[allow(clippy::cast_precision_loss)]
        let f = n as f64;
        f
    };
    // u128::MAX = 340282366920938463463374607431768211455 und i128::MAX + 1 (die kleinste Zahl, die `i128::try_from` ablehnt)
    assert_eq!(
        yaml("340282366920938463463374607431768211455"),
        PyWert::Gleit(nach_f64(u128::MAX))
    );
    let knapp_drueber = u128::try_from(i128::MAX).unwrap() + 1;
    assert_eq!(
        yaml(&knapp_drueber.to_string()),
        PyWert::Gleit(nach_f64(knapp_drueber))
    );
    // Gegenproben
    assert_eq!(
        yaml(&i128::MAX.to_string()),
        PyWert::Gleit(nach_f64(u128::try_from(i128::MAX).unwrap()))
    );
    assert_eq!(yaml(&u64::MAX.to_string()), PyWert::GrossGanz(u64::MAX));
}
