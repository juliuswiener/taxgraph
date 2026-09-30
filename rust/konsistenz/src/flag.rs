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

use serde_json::Value;

use crate::lesung::{lies, Felder, Lesung};
use crate::zahl::ganzzahl;

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
        l if l.bestaetigt() == Some(&Value::Bool(true)) => FlagStand::Ja,
        _ => FlagStand::Nein,
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
    pub wert: Value,
    /// Klartext für den Nutzer, byte-gleich zu Python.
    pub grund: String,
}

/// Der Betragstext eines negierten Feldes, oder `None`, wenn der Wert kein Betrag > 0 ist
/// (`flag_check.py:192-195`: `isinstance(wert, (int, float)) and wert > 0`).
fn betrag_text(wert: &Value) -> Option<String> {
    // PARITÄT: Python schließt `bool` hier NICHT aus — `True` ist eine 1 > 0 und erscheint als
    // „True". Auf `cent`-Feldern verhindert Auflage T das; nachgebaut, weil billig.
    if *wert == Value::Bool(true) {
        return Some("True".to_owned());
    }
    // PARITÄT: Floats zählen hier nicht als Betrag (Wertebereich, s. Crate-Doku); Python würde
    // sie mit Float-Repr formatieren.
    let n = ganzzahl(wert).filter(|n| *n > 0)?;
    // „Wert > 1000 gilt als Cent" (`flag_check.py:194`) — ohne Tausenderpunkt, anders als `eur`.
    Some(if n > 1000 {
        format!("{} €", n.div_euclid(100))
    } else {
        n.to_string()
    })
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
    use serde_json::json;

    #[test]
    fn kein_vuv_widerspruch_und_text() {
        let s = snap(&[
            ("kein_vuv", json!(true), Zustand::Bestaetigt),
            ("vv_einnahmen", json!(1_200_000), Zustand::Bestaetigt),
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
            (json!(false), Zustand::Bestaetigt),
            (json!(true), Zustand::Vorlaeufig),
        ] {
            let s = snap(&[
                ("kein_vuv", w, z),
                ("vv_einnahmen", json!(5000), Zustand::Bestaetigt),
            ]);
            assert!(flag_widersprueche::<std::hash::RandomState>(&s, None)
                .iter()
                .all(|x| x.flag != "kein_vuv"));
        }
    }

    #[test]
    fn instanzen_werden_gesehen() {
        let s = snap(&[
            ("kein_vuv", json!(true), Zustand::Bestaetigt),
            ("vv_einnahmen__2", json!(5), Zustand::Bestaetigt),
            ("vv_einnahmen__02", json!(5), Zustand::Bestaetigt),
        ]);
        let w = flag_widersprueche::<std::hash::RandomState>(&s, None);
        assert_eq!(
            w.iter().map(|x| x.feld_id.as_str()).collect::<Vec<_>>(),
            ["vv_einnahmen__2"]
        );
    }
}
