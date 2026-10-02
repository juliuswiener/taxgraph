//! Abwesenheits-Flag ↔ echte Einkunftsfelder (`produkt/konsistenz/flag_check.py`).
//!
//! Ein Flag `kein_X = true` behauptet, dass die Einkunftsart X fehlt (§ 2 Abs. 1 `EStG`). Ist
//! trotzdem ein Betragsfeld dieser Art bestätigt und > 0, ist das ein Widerspruch: der reine
//! AN-Ring würde die Einkunftsart still übergehen.
//!
//! Asymmetrie A/B (`flag_check.py:14-18`): `kein_gewinn_partner` und
//! `keine_behinderung_pflege_partner` fehlen bewusst — das Flag ist nur auf Scheibe „gesamt"
//! fragbar, die Zielfelder auch auf „`rentner_gesamt`". Vor einem Eintrag erst die
//! Szenen-Fragbarkeit prüfen.
use std::collections::HashSet;
use std::hash::BuildHasher;

use domain::PyWert;
#[cfg(test)]
use serde_json::Value;

use crate::lesung::{lies, Felder, Lesung};
#[cfg(test)]
use crate::zahl::ganzzahl_alt;

/// Flag → die echten Einkunfts-Betragsfelder derselben Art (`flag_check.py:20-74`), in
/// Python-Reihenfolge. Die Reihenfolge bestimmt die Reihenfolge der Widersprüche.
///
/// - `kein_kap`: § 2 Abs. 1 Nr. 5; `kein_vuv`: Nr. 6; `kein_sonstige`: Nr. 7 (Renten).
/// - `kein_kap_partner`: Partner-Spiegel der fünf Kap-Felder (§ 26b, Einkünfte je Ehegatte).
/// - `kein_sonstige_partner`: § 22 Nr. 1 Partner-Rente. Flag nur auf „gesamt", Zielfeld nur auf
///   „`rentner_gesamt`" — die Scheiben-Fragbarkeit ([`FlagStand::NichtFragbar`]) schließt die Lücke.
/// - `kein_p23_verkauf`: § 23 Abs. 1/3, private Veräußerungsgeschäfte.
/// - `kein_gewinn`: § 2 Abs. 1 Nr. 1-3 (§§ 13-18), jeder Betriebsindikator inkl. Verlustjahr,
///   GWG und Mitunternehmer (§ 15 Abs. 1 Nr. 2).
pub const FLAG_NEGIERT: [(&str, &[&str]); 7] = [
    (
        "kein_kap",
        &[
            "kap_kapitalertraege",
            "kap_gewinn_aktien",
            "kap_verlust_aktien",
            "kap_gewinn_sonstige",
            "kap_verlust_sonstige",
        ],
    ),
    ("kein_vuv", &["vv_einnahmen"]),
    ("kein_sonstige", &["rentner_jahresrente"]),
    (
        "kein_kap_partner",
        &[
            "kap_kapitalertraege_partner",
            "kap_gewinn_aktien_partner",
            "kap_verlust_aktien_partner",
            "kap_gewinn_sonstige_partner",
            "kap_verlust_sonstige_partner",
        ],
    ),
    ("kein_sonstige_partner", &["rentner_jahresrente_partner"]),
    (
        "kein_p23_verkauf",
        &[
            "p23_veraeusserungspreis",
            "p23_anschaffung_herstellungskosten",
            "p23_werbungskosten",
        ],
    ),
    (
        "kein_gewinn",
        &[
            "einkuenfte_gewinn",
            "rentner_veraeusserungsgewinn",
            "betriebseinnahmen",
            "sonstige_betriebsausgaben",
            "afa_jahresbetrag",
            "gwg_anschaffungskosten_netto",
            "gewinnanteil",
            "verguetung_taetigkeit",
            "verguetung_darlehen",
            "verguetung_ueberlassung",
        ],
    ),
];

/// Lesbarer Name eines Flags (`flag_check.py:137-145`); unbekannt → die Id selbst.
fn flag_name(flag: &str) -> &str {
    match flag {
        "kein_gewinn" => "Gewinneinkünfte (Gewerbe, Landwirtschaft, selbständige Arbeit)",
        "kein_kap" => "Kapitalerträge (Zinsen, Dividenden, Kursgewinne)",
        "kein_vuv" => "Einnahmen aus Vermietung oder Verpachtung",
        "kein_sonstige" => "sonstige Einkünfte (z. B. Renten, private Verkäufe)",
        "kein_kap_partner" => "Kapitalerträge deines Partners (Zinsen, Dividenden, Kursgewinne)",
        "kein_sonstige_partner" => "sonstige Einkünfte deines Partners (z. B. Renten)",
        "kein_p23_verkauf" => "private Verkäufe (z. B. Grundstück, Wertpapiere außerhalb §20)",
        andere => andere,
    }
}

/// Lesbarer Name eines negierten Feldes (`flag_check.py:147-173`); unbekannt → die Id selbst.
fn feld_name(basis: &str) -> &str {
    match basis {
        "kap_kapitalertraege" => "Kapitaleinkünfte",
        "kap_gewinn_aktien" => "Aktiengewinne",
        "kap_verlust_aktien" => "Aktienverluste",
        "kap_gewinn_sonstige" => "sonstige Kapitalgewinne",
        "kap_verlust_sonstige" => "sonstige Kapitalverluste",
        "vv_einnahmen" => "Einnahmen aus Vermietung",
        "rentner_jahresrente" => "Renteneinkünfte",
        "kap_kapitalertraege_partner" => "Kapitaleinkünfte des Partners",
        "kap_gewinn_aktien_partner" => "Aktiengewinne des Partners",
        "kap_verlust_aktien_partner" => "Aktienverluste des Partners",
        "kap_gewinn_sonstige_partner" => "sonstige Kapitalgewinne des Partners",
        "kap_verlust_sonstige_partner" => "sonstige Kapitalverluste des Partners",
        "rentner_jahresrente_partner" => "Renteneinkünfte des Partners",
        "einkuenfte_gewinn" => "Gewinneinkünfte",
        "rentner_veraeusserungsgewinn" => "Veräußerungsgewinne",
        "betriebseinnahmen" => "Betriebseinnahmen",
        "sonstige_betriebsausgaben" => "sonstige Betriebsausgaben",
        "afa_jahresbetrag" => "Abschreibungen (AfA)",
        "gwg_anschaffungskosten_netto" => "GWG-Anschaffungen",
        "gewinnanteil" => "Mitunternehmer-Gewinnanteil",
        "verguetung_taetigkeit" => "Mitunternehmer-Vergütung (Tätigkeit)",
        "verguetung_darlehen" => "Mitunternehmer-Vergütung (Darlehen)",
        "verguetung_ueberlassung" => "Mitunternehmer-Vergütung (Überlassung)",
        "p23_veraeusserungspreis" => "Veräußerungspreis (privater Verkauf)",
        "p23_anschaffung_herstellungskosten" => {
            "Anschaffungs-/Herstellungskosten (privater Verkauf)"
        }
        "p23_werbungskosten" => "Werbungskosten (privater Verkauf)",
        andere => andere,
    }
}

/// Was ein Flag im Snapshot aussagt (`flag_check.py:177-187`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagStand {
    /// Nicht im Snapshot, auf dieser Scheibe aber fragbar (oder keine Scheibe bekannt). „Nie
    /// gefragt" ist NICHT „verneint": ein Betrag neben einem nie gestellten Kreuz ist derselbe
    /// stille Widerspruch — wird wie [`FlagStand::Ja`] geprüft.
    Unbeantwortet,
    /// Nicht im Snapshot und auf dieser Scheibe strukturell unfragbar — kein Widerspruch möglich.
    NichtFragbar,
    /// Bestätigt `true`: Abwesenheit behauptet.
    Ja,
    /// Bestätigt ≠ `true` oder nur vorläufig: keine Behauptung.
    Nein,
}

/// Stand des Flags `flag`. `scheibe_felder` ist die Feldmenge der aktuellen Scheibe
/// (`api.py::_scheibe_bindung`); `None` = Alt-Verhalten ohne Scheiben-Kontext.
///
/// ```
/// use konsistenz::{flag_stand, Felder, FlagStand};
/// let leer = Felder::new();
/// assert_eq!(flag_stand::<std::hash::RandomState>(&leer, "kein_vuv", None), FlagStand::Unbeantwortet);
/// let scheibe = std::collections::HashSet::new();
/// assert_eq!(flag_stand(&leer, "kein_vuv", Some(&scheibe)), FlagStand::NichtFragbar);
/// ```
#[must_use]
pub fn flag_stand<S: BuildHasher>(
    felder: &Felder,
    flag: &str,
    scheibe_felder: Option<&HashSet<String, S>>,
) -> FlagStand {
    match lies(felder, flag) {
        Lesung::Fehlt => match scheibe_felder {
            Some(s) if !s.contains(flag) => FlagStand::NichtFragbar,
            _ => FlagStand::Unbeantwortet,
        },
        // PARITAET: `flag_check.py:186` prueft `is not True` -- IDENTITAET, nicht Pythons `==`.
        // Das Muster ist hier die richtige Uebersetzung (`PyWert::Ganz(1)` faellt korrekt
        // durch); `py_eq` waere falsch, weil es `1 == True` als wahr ansaehe.
        Lesung::Bestaetigt(PyWert::Bool(true)) => FlagStand::Ja,
        Lesung::Vorlaeufig(_) | Lesung::Bestaetigt(_) => FlagStand::Nein,
    }
}

/// `^[1-9][0-9]*$` über dem Suffix nach `basis__` (`flag_check.py:109`).
///
/// PARITÄT: Pythons `$` passt auch vor einem abschließenden `\n` — nachgebaut. Das Store-Schema
/// (`^[a-z][a-z0-9_]*$`) schließt solche Schlüssel ohnehin aus.
fn ist_instanz_suffix(rest: &str) -> bool {
    let rest = rest.strip_suffix('\n').unwrap_or(rest);
    let mut b = rest.bytes();
    matches!(b.next(), Some(b'1'..=b'9')) && b.all(|c| c.is_ascii_digit())
}

/// Alle im Snapshot vorhandenen Schlüssel für `basis`: die Basis selbst (Instanz 1), dann jedes
/// `basis__<n>` (n ≥ 1) in Snapshot-Reihenfolge (`flag_check.py:112-121`). Keine Zählung aus einem
/// Zählfeld: was da ist, ist da.
///
/// ```
/// use konsistenz::{instanz_feld_ids, Felder};
/// assert!(instanz_feld_ids(&Felder::new(), "vv_einnahmen").is_empty());
/// ```
#[must_use]
pub fn instanz_feld_ids(felder: &Felder, basis: &str) -> Vec<String> {
    let mut treffer: Vec<String> = if felder.contains_key(basis) {
        vec![basis.to_owned()]
    } else {
        Vec::new()
    };
    let praefix = format!("{basis}__");
    treffer.extend(
        felder
            .keys()
            .filter(|k| k.strip_prefix(&praefix).is_some_and(ist_instanz_suffix))
            .cloned(),
    );
    treffer
}

/// Ein Flag↔Einkunftsart-Widerspruch.
#[derive(Debug, Clone, PartialEq)]
pub struct FlagWiderspruch {
    pub flag: &'static str,
    pub feld_id: String,
    /// Der bestätigte Wert, wie im Snapshot.
    pub wert: PyWert,
    /// Klartext für den Nutzer, byte-gleich zu Python.
    pub grund: String,
}

/// Der Betragstext eines negierten Feldes, oder `None`, wenn der Wert kein Betrag > 0 ist
/// (`flag_check.py:192-195`: `isinstance(wert, (int, float)) and wert > 0`).
///
/// Diese Fassung IST das `CPython`-Modell: `gt_null` bildet die `isinstance`-Pruefung des Quells
/// nach (es nimmt `bool`, `int` und `float` an und wirft fuer alles andere `TypeError`), der
/// Resttext ist `str(wert)`, also [`PyWert::py_str`]. `mod aequivalenz` misst sie deshalb direkt
/// gegen die Vor-K2-Fassung [`betrag_text_alt`]; die D-Nummern D3 und D10 stehen dort als Liste.
fn betrag_text(wert: &PyWert) -> Option<String> {
    // PARITÄT: Python schließt `bool` hier NICHT aus — `True` ist eine 1 > 0 und erscheint als
    // „True". `gt_null` nimmt `Bool` an, `int_dezimal` schreibt es als „1" bis „0"; der Text
    // bleibt deshalb Pythons `str(wert)`.
    if wert.gt_null() != Ok(true) {
        return None;
    }
    match wert {
        // „Wert > 1000 gilt als Cent" (`flag_check.py:194`) — ohne Tausenderpunkt, anders als `eur`.
        PyWert::Ganz(n) if *n > 1000 => Some(format!("{} €", n.div_euclid(100))),
        PyWert::GrossGanz(n) if *n > 1000 => Some(format!("{} €", n / 100)),
        PyWert::Gleit(f) if *f > 1000.0 => Some(format!("{} €", gleit_floordiv(*f))),
        w => Some(w.py_str()),
    }
}

/// Die Fassung vor dem K2-Port: `Value` trennt `bool` von Zahlen, Floats fallen an `ganzzahl`
/// durch.
#[cfg(test)]
fn betrag_text_alt(wert: &Value) -> Option<String> {
    // PARITÄT: Python schließt `bool` hier NICHT aus — `True` ist eine 1 > 0 und erscheint als
    // „True". Auf `cent`-Feldern verhindert Auflage T das; nachgebaut, weil billig.
    if *wert == Value::Bool(true) {
        return Some("True".to_owned());
    }
    // PARITÄT: Floats zählen hier nicht als Betrag (Wertebereich, s. Crate-Doku); Python würde
    // sie mit Float-Repr formatieren.
    let n = ganzzahl_alt(wert).filter(|n| *n > 0)?;
    Some(if n > 1000 {
        format!("{} €", n.div_euclid(100))
    } else {
        n.to_string()
    })
}

/// `flag_check.py:194`: `f"{wert // 100} €"` auf einem Float — `CPythons` `float_divmod`
/// (`Objects/floatobject.c`), damit auch die Randfaelle der `//`-Division stimmen:
/// `fmod`, dann `(f - mod) / 100` und die `0.5`-Korrektur. Gemessen an 23 Floats bis 1e300
/// gegen 3.12.9 und 3.14.7: kein Unterschied (die naive Fassung `floor(f / 100.0)` trifft
/// dieselben 23).
///
/// PARITÄT: bleibt `f64` wie Python, denn das Ergebnis ist nur Anzeige-Text, kein Betrag
/// (Vault `decisions/rust-port-geld-cent-saetze-decimal`, Nachtrag 2026-10-02).
///
/// ponytail: `inf` ergaebe in `CPython` `nan` (gemessen), hier bliebe `floor` bei `inf`. Beides
/// ist im Store nicht darstellbar — `serde_json` weist `1e999` beim Laden als `NumberOutOfRange`
/// ab, `store::Store::append` NaN/inf an der Append-Grenze (Auflage 3). Upgrade: ein `inf`-Zweig,
/// falls je ein Pfad ohne diese beiden Waechter entsteht.
fn gleit_floordiv(f: f64) -> String {
    let rest = f % 100.0;
    let div = (f - rest) / 100.0;
    let mut floor = div.floor();
    if div - floor > 0.5 {
        floor += 1.0;
    }
    PyWert::Gleit(floor).py_str()
}

/// Snapshot → Flag↔Einkunftsart-Widersprüche (`flag_check.py:124-201`).
///
/// ```
/// use konsistenz::{flag_widersprueche, Felder};
/// assert!(flag_widersprueche::<std::hash::RandomState>(&Felder::new(), None).is_empty());
/// ```
#[must_use]
pub fn flag_widersprueche<S: BuildHasher>(
    felder: &Felder,
    scheibe_felder: Option<&HashSet<String, S>>,
) -> Vec<FlagWiderspruch> {
    let mut widersprueche = Vec::new();
    for (flag, basen) in FLAG_NEGIERT {
        match flag_stand(felder, flag, scheibe_felder) {
            FlagStand::Ja | FlagStand::Unbeantwortet => {}
            FlagStand::Nein | FlagStand::NichtFragbar => continue,
        }
        let flag_titel = flag_name(flag);
        for basis in basen {
            for feld_id in instanz_feld_ids(felder, basis) {
                let Some(wert) = lies(felder, &feld_id).bestaetigt() else {
                    continue;
                };
                let Some(betrag) = betrag_text(wert) else {
                    continue;
                };
                let feld_titel = feld_name(basis);
                widersprueche.push(FlagWiderspruch {
                    flag,
                    grund: format!(
                        "Du hast angegeben, keine {flag_titel} zu haben — bei den {feld_titel} \
                         wurden aber {betrag} erfasst. Bitte prüfe, welche der beiden Angaben stimmt."
                    ),
                    wert: wert.clone(),
                    feld_id,
                });
            }
        }
    }
    widersprueche
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lesung::test_snap as snap;
    use domain::Zustand;

    #[test]
    fn kein_vuv_widerspruch_und_text() {
        let s = snap(&[
            ("kein_vuv", PyWert::Bool(true), Zustand::Bestaetigt),
            ("vv_einnahmen", PyWert::Ganz(1_200_000), Zustand::Bestaetigt),
        ]);
        let w = flag_widersprueche::<std::hash::RandomState>(&s, None);
        assert_eq!(w.len(), 1);
        assert_eq!(
            w[0].grund,
            "Du hast angegeben, keine Einnahmen aus Vermietung oder Verpachtung zu haben — bei den \
             Einnahmen aus Vermietung wurden aber 12000 € erfasst. Bitte prüfe, welche der beiden \
             Angaben stimmt."
        );
    }

    #[test]
    fn bestaetigt_false_und_vorlaeufig_ueberspringen() {
        for (w, z) in [
            (PyWert::Bool(false), Zustand::Bestaetigt),
            (PyWert::Bool(true), Zustand::Vorlaeufig),
        ] {
            let s = snap(&[
                ("kein_vuv", w, z),
                ("vv_einnahmen", PyWert::Ganz(5000), Zustand::Bestaetigt),
            ]);
            assert!(flag_widersprueche::<std::hash::RandomState>(&s, None)
                .iter()
                .all(|x| x.flag != "kein_vuv"));
        }
    }

    #[test]
    fn instanzen_werden_gesehen() {
        let s = snap(&[
            ("kein_vuv", PyWert::Bool(true), Zustand::Bestaetigt),
            ("vv_einnahmen__2", PyWert::Ganz(5), Zustand::Bestaetigt),
            ("vv_einnahmen__02", PyWert::Ganz(5), Zustand::Bestaetigt),
        ]);
        let w = flag_widersprueche::<std::hash::RandomState>(&s, None);
        assert_eq!(
            w.iter().map(|x| x.feld_id.as_str()).collect::<Vec<_>>(),
            ["vv_einnahmen__2"]
        );
    }
}

/// Aequivalenz mit `domain::PyWert` (D15): Abweichungen nur mit D-Nummer aus der Liste des
/// Helfers, je D-Nummer ein Test mit dem `CPython`-Verhalten.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{d3_d10, json_wert, pruefe, py};
    use proptest::prelude::*;
    use serde_json::json;

    use super::{betrag_text, betrag_text_alt};

    /// Ausnahmen der Vor-K2-Fassung `betrag_text_alt` gegen `CPython` — Auflage 1: die Liste
    /// bleibt, auch wenn kein Fall im Bestand sie trifft. Sie ist die Falle fuer den ersten, der
    /// es tut.
    const BETRAG: &[&str] = &["D3", "D10"];

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        /// Die Produktion gegen die Vor-K2-Fassung, mit der D-Liste als einziger Erlaubnis.
        ///
        /// Der Test hat eine Annahme widerlegt, die ich beim Umbau hatte: die Produktion ist NICHT
        /// abweichungsfrei gegen die Alt-Fassung. Bei `2^63` liefert sie
        /// `Some("92233720368547758 €")` wie `CPython`, die Alt-Fassung `None`. Sie erbt die
        /// D-Nummern also, statt sie zu tilgen — genau das ist der Zweck des Ports.
        #[test]
        fn betrag_text_wie_pywert(v in json_wert()) {
            let (alt, neu) = (betrag_text_alt(&v), betrag_text(&py(&v)));
            pruefe(&v, &alt, &neu, || d3_d10(&v), BETRAG)?;
        }
    }

    /// D3: `2**64 - 1` ist in `CPython` ein `int` und wird als Betrag formatiert. Die Vor-K2-
    /// Fassung liefert `None` (`ganzzahl_alt` endet an `i64`), der Widerspruch entfiel damit still.
    /// Die Produktion ueber `PyWert::int_dezimal` trifft `CPython`.
    #[test]
    fn d3_ueber_i64() {
        let v = json!(u64::MAX);
        assert_eq!(betrag_text_alt(&v), None);
        assert_eq!(
            betrag_text(&py(&v)),
            Some("184467440737095516 €".to_owned())
        );
    }

    /// D10: `float` zaehlt in `CPython` als Zahl (`isinstance(wert, (int, float))`). Die Vor-K2-
    /// Fassung liefert `None` (`ganzzahl_alt` nimmt keine Floats), der Widerspruch entfiel damit
    /// still. 2.5 und 1500.0 sind der `isinstance`-Zweig und der Format-Zweig, gemessen an 3.12.9
    /// und 3.14.7 (`str(2.5)` = „2.5", `f"{1500.0 // 100} €"` = „15.0 €"); 1234.5 zeigt, dass die
    /// `//`-Division des Quells abrundet und den Bruchteil verwirft.
    #[test]
    fn d10_float_ist_betrag() {
        for (v, text) in [
            (json!(2.5), "2.5"),
            (json!(1500.0), "15.0 €"),
            (json!(1234.5), "12.0 €"),
        ] {
            assert_eq!(betrag_text_alt(&v), None);
            assert_eq!(betrag_text(&py(&v)), Some(text.to_owned()));
        }
    }

    /// Das Praedikat und die Produktion selbst: jede Zahl > 0 gilt, auch ein Float unter 1000 und
    /// `True` (Python: `isinstance(True, int)`).
    #[test]
    fn d3_d10_predikat() {
        assert_eq!(d3_d10(&json!(u64::MAX)), ["D3"]);
        assert_eq!(d3_d10(&json!(1500.0)), ["D10"]);
        assert!(d3_d10(&json!(1500)).is_empty());
        assert_eq!(betrag_text(&py(&json!(0.5))), Some("0.5".to_owned()));
        assert_eq!(betrag_text(&py(&json!(true))), Some("True".to_owned()));
        assert_eq!(betrag_text(&py(&json!(null))), None);
        assert_eq!(betrag_text(&py(&json!("1500"))), None);
        assert_eq!(betrag_text(&py(&json!(-1))), None);
    }
}
