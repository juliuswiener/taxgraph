//! VaSt-Belege (ELSTER-Datenabholung) → `feld_id`-Saetze fuer [`crate::edaten`]
//! (`produkt/eingang/vast_mapping.py`). Die Euro→Cent-Umrechnung passiert an GENAU einer
//! Stelle ([`cent`]); ein 100-facher Fehler hier waere eine stille Fehlbesteuerung.
use serde::Serialize;

use llm::py;

/// Ein eDaten-Satz (`{"feld_id", "wert", "kategorie"}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EdatenSatz {
    pub feld_id: String,
    pub wert: i64,
    pub kategorie: String,
}

/// `_cent` scheiterte.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VastFehler {
    /// `ValueError("VaSt-Betrag nicht lesbar: {euro_text!r}")`.
    #[error("VaSt-Betrag nicht lesbar: {}", py::py_repr_str(.0))]
    NichtLesbar(String),
    /// Python: `OverflowError` (`Infinity`) bzw. grosse Ganzzahl ausserhalb `i64`.
    #[error("VaSt-Betrag ausserhalb des Wertebereichs: {}", py::py_repr_str(.0))]
    Ueberlauf(String),
}

/// `decimal`-Kontext: 28 signifikante Stellen.
const GENAUIGKEIT: usize = 28;

/// Zerlegt einen `Decimal`-Literaltext in (negativ, Ziffern, Exponent). `None` = kein Literal
/// ODER `NaN` (beides endet in Python als `ValueError` „nicht lesbar"); `Some(Err(()))` =
/// `Infinity` (Python: `OverflowError`).
fn zerlege(s: &str) -> Option<Result<(bool, String, i64), ()>> {
    let (neg, rest) = match s.as_bytes().first() {
        Some(b'-') => (true, s.get(1..)?),
        Some(b'+') => (false, s.get(1..)?),
        _ => (false, s),
    };
    let klein = rest.to_ascii_lowercase();
    if matches!(klein.as_str(), "inf" | "infinity") {
        return Some(Err(()));
    }
    let (mantisse, exp) = match rest.find(['e', 'E']) {
        Some(i) => (rest.get(..i)?, rest.get(i + 1..)?.parse::<i64>().ok()?),
        None => (rest, 0),
    };
    let (ganz, bruch) = mantisse.split_once('.').unwrap_or((mantisse, ""));
    let gueltig = |t: &str| !t.starts_with('_') && !t.ends_with('_') && !t.contains("__") && t.bytes().all(|b| b.is_ascii_digit() || b == b'_');
    if (ganz.is_empty() && bruch.is_empty()) || !gueltig(ganz) || !gueltig(bruch) {
        return None;
    }
    let ziffern = format!("{ganz}{bruch}").replace('_', "");
    let exp = exp.checked_sub(i64::try_from(bruch.replace('_', "").len()).ok()?)?;
    Some(Ok((neg, ziffern, exp)))
}

/// Ziffernfolge half-even auf `behalte` Stellen runden; Rueckgabe (Ziffern, Uebertrag-Stelle).
fn runde_half_even(ziffern: &str, behalte: usize) -> (String, bool) {
    let (kopf, rest) = ziffern.split_at(behalte.min(ziffern.len()));
    let mut kopf: Vec<u8> = kopf.bytes().collect();
    let erste = rest.as_bytes().first().copied().unwrap_or(b'0');
    let danach_nicht_null = rest.bytes().skip(1).any(|b| b != b'0');
    let letzte_ungerade = kopf.last().is_some_and(|b| (b - b'0') % 2 == 1);
    let auf = erste > b'5' || (erste == b'5' && (danach_nicht_null || letzte_ungerade));
    let mut ueberlauf = false;
    if auf {
        let mut i = kopf.len();
        loop {
            if i == 0 {
                kopf.insert(0, b'1');
                ueberlauf = true;
                break;
            }
            i -= 1;
            if let Some(z) = kopf.get_mut(i) {
                if *z == b'9' {
                    *z = b'0';
                } else {
                    *z += 1;
                    break;
                }
            }
        }
    }
    (String::from_utf8(kopf).unwrap_or_default(), ueberlauf)
}

/// `_cent(euro_text)`: `"45000.00"` → 4 500 000. Leer → `None` (nicht besetzt ≠ 0). Wie
/// `int((Decimal(s) * 100).to_integral_value())`: Produkt im 28-Stellen-Kontext half-even,
/// dann half-even auf ganze Cent. Dezimalziffern jeder Schrift zaehlen wie in `Decimal()`.
///
/// ```
/// use eingang::vast::cent;
/// assert_eq!(cent(Some("45000.00")).unwrap(), Some(4_500_000));
/// assert_eq!(cent(Some("0,005")).unwrap(), Some(0));
/// assert_eq!(cent(Some("0,015")).unwrap(), Some(2));
/// assert_eq!(cent(Some(" ")).unwrap(), None);
/// assert!(cent(Some("abc")).is_err());
/// ```
///
/// # Errors
/// [`VastFehler`].
pub fn cent(euro_text: Option<&str>) -> Result<Option<i64>, VastFehler> {
    let Some(roh) = euro_text else { return Ok(None) };
    let s = py::ascii_ziffern(py::strip(roh)).replace(',', ".");
    if s.is_empty() {
        return Ok(None);
    }
    let (neg, ziffern, exp) = match zerlege(&s) {
        None => return Err(VastFehler::NichtLesbar(roh.to_owned())),
        Some(Err(())) => return Err(VastFehler::Ueberlauf(roh.to_owned())),
        Some(Ok(t)) => t,
    };
    // `Decimal * 100`: Koeffizient × 100 (zwei Ziffern mehr), Exponent unveraendert; dann auf
    // 28 signifikante Stellen.
    let mut koeff = format!("{}00", ziffern.trim_start_matches('0'));
    let mut exp = exp;
    if koeff == "00" {
        return Ok(Some(0));
    }
    if koeff.len() > GENAUIGKEIT {
        let weg = koeff.len() - GENAUIGKEIT;
        let (k, ueber) = runde_half_even(&koeff, GENAUIGKEIT);
        koeff = if ueber { k.get(..GENAUIGKEIT).unwrap_or(&k).to_owned() } else { k };
        exp += i64::try_from(weg).unwrap_or(0) + i64::from(ueber);
    }
    // Mehr als 19 Vorkommastellen passen in kein `i64`; weniger als 0,1 rundet auf 0. Beide
    // Schranken halten auch die Zeichenketten klein (`1e999999`).
    if exp >= 0 && koeff.len() + usize::try_from(exp).unwrap_or(usize::MAX) > 19 {
        return Err(VastFehler::Ueberlauf(roh.to_owned()));
    }
    let betrag = if exp >= 0 {
        format!("{koeff}{}", "0".repeat(usize::try_from(exp).unwrap_or(0)))
    } else {
        let nachkomma = usize::try_from(exp.unsigned_abs()).unwrap_or(usize::MAX);
        match nachkomma.cmp(&koeff.len()) {
            std::cmp::Ordering::Greater => "0".to_owned(),
            std::cmp::Ordering::Equal => runde_half_even(&format!("0{koeff}"), 1).0,
            std::cmp::Ordering::Less => runde_half_even(&koeff, koeff.len() - nachkomma).0,
        }
    };
    let wert: i64 = betrag.parse().map_err(|_| VastFehler::Ueberlauf(roh.to_owned()))?;
    Ok(Some(if neg { -wert } else { wert }))
}

/// `LSTB`: VaSt-Element → (`feld_id`, Schema-Doku).
pub const LSTB: [(&str, &str, &str); 8] = [
    ("BruttoArbLohn", "bruttoarbeitslohn", "Bruttoarbeitslohn"),
    ("LSteuer", "p36_lohnsteuer", "einbehaltene Lohnsteuer"),
    ("ArbnKiSteuer", "kist_gezahlt", "einbehaltene Kirchensteuer des Arbeitnehmers"),
    ("LeistungenProgVorbeh", "p32b_progressionseinkuenfte", "Leistungen, die dem Progressionsvorbehalt unterliegen"),
    ("ArbnAnteilRenVers", "vor_an_anteil_rv", "Arbeitnehmeranteil zur gesetzlichen Rentenversicherung"),
    ("ArbgAnteilRenVers", "vor_ag_anteil_rv", "Arbeitgeberanteil zur gesetzlichen Rentenversicherung"),
    ("ArbnAnteilKrankVers", "basis_kv", "Arbeitnehmerbeiträge zur gesetzlichen Krankenversicherung (LStB Nr. 25)"),
    ("ArbnAnteilPflegVers", "basis_pv", "Arbeitnehmerbeiträge zur sozialen Pflegeversicherung (LStB Nr. 26)"),
];

/// `LSTB_SUMMEN`: Zielfeld ← mehrere Beleg-Elemente.
pub const LSTB_SUMMEN: [(&str, &[(&str, &str)]); 1] =
    [("vorsorge_arbeitslosenversicherung", &[("ArbnAnteilArblVers", "Arbeitnehmerbeiträge zur Arbeitslosenversicherung")])];

/// `LERSL_BETRAG_FELD`.
pub const LERSL_BETRAG_FELD: &str = "p32b_progressionseinkuenfte";

/// `aus_lstb(werte)`: Summen zuerst, dann die 1:1-Felder; unbesetzte Elemente uebersprungen.
///
/// ```
/// let w = std::collections::BTreeMap::from([("LSteuer".to_string(), "1234.56".to_string())]);
/// let s = eingang::vast::aus_lstb(&w).unwrap();
/// assert_eq!((s[0].feld_id.as_str(), s[0].wert), ("p36_lohnsteuer", 123_456));
/// ```
///
/// # Errors
/// [`VastFehler`] beim ersten nicht lesbaren Betrag.
pub fn aus_lstb(werte: &std::collections::BTreeMap<String, String>) -> Result<Vec<EdatenSatz>, VastFehler> {
    let mut raus = Vec::new();
    for (feld_id, quellen) in LSTB_SUMMEN {
        let mut besetzt = Vec::new();
        for (name, doku) in quellen {
            if let Some(c) = cent(werte.get(*name).map(String::as_str))? {
                besetzt.push((name, c, doku));
            }
        }
        if besetzt.is_empty() {
            continue;
        }
        let kat = format!("LStB: {}", besetzt.iter().map(|(n, _, d)| format!("{d} ({n})")).collect::<Vec<_>>().join(" + "));
        raus.push(EdatenSatz { feld_id: feld_id.to_owned(), wert: besetzt.iter().map(|(_, c, _)| c).sum(), kategorie: kat });
    }
    for (vast, feld_id, doku) in LSTB {
        if let Some(c) = cent(werte.get(vast).map(String::as_str))? {
            raus.push(EdatenSatz { feld_id: feld_id.to_owned(), wert: c, kategorie: format!("LStB/{vast}: {doku}") });
        }
    }
    Ok(raus)
}

/// Eine Lohnersatzleistung (`{"Betrag", "Art"}`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
pub struct Leistung {
    #[serde(rename = "Betrag", default)]
    pub betrag: Option<String>,
    #[serde(rename = "Art", default)]
    pub art: Option<String>,
}

/// `aus_lersl(leistungen)`: EIN Satz mit der Summe (§ 32b Abs. 1 Nr. 1); Summe 0 → keiner.
///
/// ```
/// use eingang::vast::{aus_lersl, Leistung};
/// let l = vec![Leistung { betrag: Some("100.00".into()), art: Some("ALG".into()) }];
/// assert_eq!(aus_lersl(&l).unwrap()[0].wert, 10_000);
/// ```
///
/// # Errors
/// [`VastFehler`].
pub fn aus_lersl(leistungen: &[Leistung]) -> Result<Vec<EdatenSatz>, VastFehler> {
    let mut summe: i64 = 0;
    let mut arten = std::collections::BTreeSet::new();
    for l in leistungen {
        let Some(c) = cent(l.betrag.as_deref())? else { continue };
        summe = summe.checked_add(c).ok_or_else(|| VastFehler::Ueberlauf(l.betrag.clone().unwrap_or_default()))?;
        let art = py::strip(l.art.as_deref().unwrap_or(""));
        if !art.is_empty() {
            arten.insert(art.to_owned());
        }
    }
    if summe == 0 {
        return Ok(Vec::new());
    }
    let mut kat = "LErsL: Lohnersatzleistungen (§ 32b Abs. 1 Nr. 1)".to_owned();
    if !arten.is_empty() {
        kat.push_str(" — ");
        kat.push_str(&arten.into_iter().collect::<Vec<_>>().join(", "));
    }
    Ok(vec![EdatenSatz { feld_id: LERSL_BETRAG_FELD.to_owned(), wert: summe, kategorie: kat }])
}

#[cfg(test)]
mod tests {
    use super::cent;

    #[test]
    fn decimal_randfaelle() {
        assert_eq!(cent(Some("-12.345")).unwrap(), Some(-1234));
        assert_eq!(cent(Some("1e2")).unwrap(), Some(10_000));
        assert_eq!(cent(Some(".5")).unwrap(), Some(50));
        assert_eq!(cent(Some("0.125")).unwrap(), Some(12));
        assert_eq!(cent(Some("0.135")).unwrap(), Some(14));
        assert!(cent(Some("NaN")).is_err());
        assert!(cent(Some("Infinity")).is_err());
    }
}
