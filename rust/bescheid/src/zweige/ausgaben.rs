//! Die Out-Parameter der Zweige (Python: `solz_container: list`, `extras: dict`) als typisierte
//! Felder, dazu die vier kleinen Funktionen um sie: `_kist_konfession`, `_abschlusszahlung_cent`,
//! `_setze_kette`, `_kette_p31` (`bescheid_zweige.py:72-182`).
//!
//! Python-`dict`-Semantik bleibt erhalten: ein Schluessel ist `None`, solange kein Lauf ihn gesetzt hat
//! (`Schluessel absent = nicht rechenbar`, `_feste_zahl`), und spaetere Laeufe ueberschreiben.
use domain::{Cent, Euro};
use engine::zugriff::teil1::ermaessigungen::{p36_abschlusszahlung, P36AbschlusszahlungEingabe};
use engine::zugriff::teil2::gesamt::GesamtKette;
use serde_json::Value;

use super::rechnen::R;
use crate::{zahl_int, Felder};
use domain::Zustand;

/// Wer die Guenstigerpruefung § 31 gewonnen hat (`kette["p31"]["guenstiger"]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum P31Sieger {
    Freibetraege,
    Kindergeld,
}

/// Die `p31`-Erklaerung der Rechenweg-Kette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct P31Hinweis {
    pub guenstiger: P31Sieger,
    /// Kindergeld fuer das Jahr, EURO.
    pub kindergeld: Euro,
    pub text: String,
}

/// Rechenweg-Kette (`catala_gesamt_kette`, alle Werte EURO), im § 31-Fall mit `p31`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kette {
    pub gesamtbetrag_der_einkuenfte: Euro,
    pub zu_versteuerndes_einkommen: Euro,
    pub tarifliche_est: Euro,
    pub festzusetzende_est: Euro,
    pub p31: Option<P31Hinweis>,
}

impl From<GesamtKette> for Kette {
    fn from(k: GesamtKette) -> Self {
        Self {
            gesamtbetrag_der_einkuenfte: k.gesamtbetrag_der_einkuenfte,
            zu_versteuerndes_einkommen: k.zu_versteuerndes_einkommen,
            tarifliche_est: k.tarifliche_est,
            festzusetzende_est: k.festzusetzende_est,
            p31: None,
        }
    }
}

/// Python-`extras`-dict: Post-Engine-Zuschlaege und Praemien der `/ergebnis`-Antwort.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Extras {
    /// § 51a Kirchensteuer, CENT. `None` = Schluessel absent (Konfession unbeantwortet).
    pub kist_cent: Option<Cent>,
    /// § 32d Abs. 1 S. 3-5 Kapital-KiSt, CENT (nur Zwischenwert fuer `kist_cent`).
    pub kist_kap_cent: Option<Cent>,
    /// § 32d Abs. 6: hat der tarifliche Zweig (strikt `<`) gewonnen? Speist E1900401.
    pub kap_guenstiger_gewonnen: Option<bool>,
    /// § 101 Mobilitaetspraemie, CENT.
    pub mobilitaetspraemie_cent: Option<Cent>,
    /// P5.4 Rechenweg-Kette; nur gesetzt, wenn ihre letzte Stufe die ausgegebene Steuer trifft.
    pub kette: Option<Kette>,
}

/// Die Konfession — oder `None`, wenn sie niemand angegeben hat.
///
/// Python: `wert if isinstance(wert, str) and wert else None`. Ein Feld ohne Eintrag, mit `null`,
/// mit Zahl oder leerem Text ist "nicht angegeben" (kein "keine" als Vorgabe, s. Python-Docstring).
///
/// ```
/// use bescheid::testhilfe::{felder, store};
/// use bescheid::zweige::kist_konfession;
/// use serde_json::json;
/// assert_eq!(kist_konfession(&felder(&store(&[("kist_konfession", json!("rk"), true)]))), Some("rk"));
/// assert_eq!(kist_konfession(&felder(&store(&[("kist_konfession", json!(""), true)]))), None);
/// ```
#[must_use]
pub fn kist_konfession(felder: &Felder) -> Option<&str> {
    match felder.get("kist_konfession").map(|e| &e.wert) {
        Some(Value::String(s)) if !s.is_empty() => Some(s.as_str()),
        _ => None,
    }
}

/// Bestaetigter numerischer Wert (`isinstance(w, (int, float)) and not bool`), sonst `None`.
fn best_zahl<'a>(felder: &'a Felder, fid: &str) -> Option<&'a serde_json::Number> {
    let e = felder.get(fid)?;
    if e.zustand != Zustand::Bestaetigt {
        return None;
    }
    match &e.wert {
        Value::Number(n) => Some(n),
        _ => None,
    }
}

/// § 36 Abs. 2+4 `EStG`: Abschlusszahlung (+) / Erstattung (−) in CENT auf der festgesetzten `ESt`
/// `zahl_cent`. `None`, wenn kein einziges Anrechnungsfeld BESTAETIGT vorliegt.
///
/// PARITÄT: fail-open default — ein fehlendes Anrechnungsfeld zaehlt 0 (`int(x or 0)`), solange
/// mindestens eines der fuenf da ist.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`], Accessor-Fehler.
///
/// ```
/// use bescheid::testhilfe::{felder, store};
/// use bescheid::zweige::abschlusszahlung_cent;
/// use domain::Cent;
/// use serde_json::json;
/// let f = felder(&store(&[("p36_lohnsteuer", json!(100_000), true)]));
/// // festgesetzte ESt 2.500 EUR, angerechnete Lohnsteuer 1.000 EUR → Abschlusszahlung 1.500 EUR
/// assert_eq!(abschlusszahlung_cent(&f, Cent::new(250_000)).unwrap(), Some(Cent::new(150_000)));
/// // ohne ein bestaetigtes Anrechnungsfeld gibt es keine Zahl
/// let leer = felder(&store(&[("p36_lohnsteuer", json!(100_000), false)]));
/// assert_eq!(abschlusszahlung_cent(&leer, Cent::new(250_000)).unwrap(), None);
/// ```
pub fn abschlusszahlung_cent(felder: &Felder, zahl_cent: Cent) -> R<Option<Cent>> {
    let ids = [
        "p36_lohnsteuer",
        "p36_vorauszahlungen",
        "p36_kapitalertragsteuer",
        "p36_kapitalertragsteuer_solz",
        "p36_kapitalertragsteuer_kist",
    ];
    let werte = ids.map(|i| best_zahl(felder, i));
    if werte.iter().all(Option::is_none) {
        return Ok(None);
    }
    let cent = |n: Option<&serde_json::Number>| -> R<Cent> {
        // PARITÄT: fail-open default — absent = 0.
        n.map_or(Ok(Cent::new(0)), |n| zahl_int(n).map(Cent::new))
    };
    let [lst, vor, kapest, solz, kist] = werte;
    Ok(Some(p36_abschlusszahlung(&P36AbschlusszahlungEingabe {
        festzusetzende_est_cent: zahl_cent,
        lohnsteuer_cent: cent(lst)?,
        kapitalertragsteuer_cent: cent(kapest)?,
        kapitalertragsteuer_solz_cent: cent(solz)?,
        kapitalertragsteuer_kist_cent: cent(kist)?,
        vorauszahlungen_cent: cent(vor)?,
    })?))
}

/// Rechenweg-Kette nur, wenn ihre letzte Stufe exakt die ausgegebene Steuer `est` (EURO) ist.
///
/// ```
/// use bescheid::zweige::{setze_kette, Extras};
/// use bescheid::zweige::Kette;
/// use domain::Euro;
/// let kette = |est: i64| Kette {
///     gesamtbetrag_der_einkuenfte: Euro::new(50_000),
///     zu_versteuerndes_einkommen: Euro::new(45_000),
///     tarifliche_est: Euro::new(est),
///     festzusetzende_est: Euro::new(est),
///     p31: None,
/// };
/// let mut extras = Extras::default();
/// setze_kette(&mut extras, kette(9_000), Euro::new(1));
/// assert!(extras.kette.is_none()); // letzte Stufe trifft die ausgegebene Steuer nicht
/// setze_kette(&mut extras, kette(9_000), Euro::new(9_000));
/// assert!(extras.kette.is_some());
/// ```
// ponytail: prueft nur die letzte Stufe (Python-Befund) — heben sich zwei Korrekturen auf den Euro
// genau auf, stimmen die Zwischenstufen nicht. Upgrade: die Kette aus dem Endstand speisen.
pub fn setze_kette(extras: &mut Extras, kette: Kette, est: Euro) {
    if kette.festzusetzende_est == est {
        extras.kette = Some(kette);
    }
}

/// `f"{n:,}".replace(",", ".")` — Tausenderpunkte wie Python.
fn tausender(n: i64) -> String {
    let ziffern = n.unsigned_abs().to_string();
    let mut aus = String::new();
    for (i, c) in ziffern.chars().enumerate() {
        if i > 0 && (ziffern.len() - i).is_multiple_of(3) {
            aus.push('.');
        }
        aus.push(c);
    }
    if n < 0 {
        aus.insert(0, '-');
    }
    aus
}

/// Die Kette im § 31-Fall: der Lauf, der die Steuer bestimmt hat, plus die Guenstigerpruefung als
/// benannte Entscheidung. `kindergeld` in EURO.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`] bei `festzusetzende_est + kindergeld`.
///
/// ```
/// use bescheid::zweige::{kette_p31, P31Sieger};
/// use bescheid::zweige::Kette;
/// use domain::Euro;
/// let kette = |est: i64| Kette {
///     gesamtbetrag_der_einkuenfte: Euro::new(50_000),
///     zu_versteuerndes_einkommen: Euro::new(45_000),
///     tarifliche_est: Euro::new(est),
///     festzusetzende_est: Euro::new(est),
///     p31: None,
/// };
/// let k = kette_p31(kette(9_000), kette(7_000), true, Euro::new(3_000)).unwrap();
/// assert_eq!(k.festzusetzende_est, Euro::new(10_000)); // tarifliche Steuer + Kindergeld (§ 31 S. 4)
/// assert_eq!(k.p31.unwrap().guenstiger, P31Sieger::Freibetraege);
/// let k = kette_p31(kette(9_000), kette(7_000), false, Euro::new(3_000)).unwrap();
/// assert_eq!(k.festzusetzende_est, Euro::new(9_000));
/// ```
pub fn kette_p31(
    kette_ohne: Kette,
    kette_mit: Kette,
    freibetraege_guenstiger: bool,
    kindergeld: Euro,
) -> R<Kette> {
    // Kindergeld kommt aus den Jahresparametern; ein negativer Wert drehte die Gegenueberstellung.
    debug_assert!(kindergeld.get() >= 0);
    let kg = tausender(kindergeld.get());
    if freibetraege_guenstiger {
        let mut kette = kette_mit;
        kette.festzusetzende_est = super::rechnen::add(kette.festzusetzende_est, kindergeld)?;
        kette.p31 = Some(P31Hinweis {
            guenstiger: P31Sieger::Freibetraege,
            kindergeld,
            text: format!(
                "Höher als die tarifliche Steuer, weil die {kg} € Kindergeld für das Jahr \
                 hinzugerechnet sind (§ 31 S. 4 EStG): Der Kinderfreibetrag ist in den Stufen \
                 darüber schon abgezogen."
            ),
        });
        Ok(kette)
    } else {
        let mut kette = kette_ohne;
        kette.p31 = Some(P31Hinweis {
            guenstiger: P31Sieger::Kindergeld,
            kindergeld,
            text: format!(
                "Hier bleibt es beim Kindergeld von {kg} €; der Kinderfreibetrag wurde \
                 geprüft und mindert die Steuer nicht (§ 31 EStG)."
            ),
        });
        Ok(kette)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tausenderpunkte_wie_python() {
        assert_eq!(tausender(0), "0");
        assert_eq!(tausender(999), "999");
        assert_eq!(tausender(1000), "1.000");
        assert_eq!(tausender(3_060), "3.060");
        assert_eq!(tausender(-1_234_567), "-1.234.567");
    }
}

/// Aequivalenz von `best_zahl` mit `PyWert::zahl_ohne_bool` (D15), ohne Ausnahme.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{json_wert, pruefe, py};
    use proptest::prelude::*;

    use super::best_zahl;
    use crate::aequivalenz::ein_feld;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn best_zahl_wie_pywert(v in json_wert(), bestaetigt in any::<bool>()) {
            let alt = best_zahl(&ein_feld("x", v.clone(), bestaetigt), "x").is_some();
            let neu = bestaetigt && py(&v).zahl_ohne_bool().is_some();
            pruefe(&v, &alt, &neu, Vec::new, &[])?;
        }
    }
}
