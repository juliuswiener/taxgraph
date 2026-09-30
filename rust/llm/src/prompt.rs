//! Die drei Prompts (`_aussagen_prompt`, `_themen_prompt`, `_dialog_prompt` mit
//! `_aussagen_block`, dazu `_regel_zeilen`; `api_llm.py:37-221, 650-672, 766-784, 841-860`).
//! Die festen Textteile stehen generiert in [`crate::texte`].
use std::fmt::Write as _;

use crate::client::Nachricht;
use crate::gates::KatalogFeld;
use crate::parse::Aussage;
use crate::pii::Gefiltert;
use crate::py;
use crate::texte;

/// `_REGEL_FRAGEN`, `_REGEL_FRAGE_ZEICHEN`.
const REGEL_FRAGEN: usize = 3;
const REGEL_FRAGE_ZEICHEN: usize = 44;

/// Stufe 1.
///
/// ```
/// let (g, _) = llm::pii::filtere("ich bin ledig");
/// let m = llm::prompt::aussagen_prompt(&g);
/// assert_eq!(m[1].inhalt(), "ich bin ledig");
/// ```
#[must_use]
pub fn aussagen_prompt(freitext: &Gefiltert) -> Vec<Nachricht> {
    vec![
        Nachricht::system(texte::AUSSAGEN_SYSTEM.to_owned()),
        Nachricht::nutzer(freitext),
    ]
}

/// `_regel_zeilen(je_regel, regeln)`: je Regel die ersten drei Feldfragen, an der Wortgrenze
/// auf 44 Zeichen gekuerzt.
///
/// ```
/// let kat: Vec<llm::gates::KatalogFeld> = serde_json::from_str(
///     r#"[{"feld_id": "a", "fragetext_laie": "Wie heisst du?", "regel_id": "r"}]"#).unwrap();
/// let je = llm::gates::felder_je_regel(&kat);
/// assert_eq!(llm::prompt::regel_zeilen(&je, &["r".into()]), "- r: Wie heisst du");
/// ```
#[must_use]
pub fn regel_zeilen(je_regel: &[(String, Vec<&KatalogFeld>)], regeln: &[String]) -> String {
    regeln
        .iter()
        .map(|r| {
            let felder = je_regel
                .iter()
                .find(|(k, _)| k == r)
                .map(|(_, v)| v.as_slice())
                .unwrap_or_default();
            let fragen: Vec<String> = felder
                .iter()
                .take(REGEL_FRAGEN)
                .filter_map(|f| {
                    let roh = f
                        .fragetext_laie
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .unwrap_or(&f.feld_id);
                    let mut t = py::strip(roh.trim_end_matches(['?', ' '])).to_owned();
                    if py::laenge(&t) > REGEL_FRAGE_ZEICHEN {
                        let kopf = py::vorne(&t, REGEL_FRAGE_ZEICHEN);
                        t = format!("{}…", kopf.rsplit_once(' ').map_or(kopf, |(a, _)| a));
                    }
                    (!t.is_empty()).then_some(t)
                })
                .collect();
            if fragen.is_empty() {
                format!("- {r}")
            } else {
                format!("- {r}: {}", fragen.join("; "))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Stufe 2.
///
/// ```
/// let (g, _) = llm::pii::filtere("Frage");
/// let m = llm::prompt::themen_prompt(&g, &[], "- r");
/// assert!(m[0].inhalt().contains("AUSSAGEN:\n(keine)\n\nREGELN:\n- r"));
/// ```
#[must_use]
pub fn themen_prompt(
    freitext: &Gefiltert,
    aussagen: &[Aussage],
    regel_zeilen: &str,
) -> Vec<Nachricht> {
    let nummeriert = nummeriert(aussagen);
    let nummeriert = if nummeriert.is_empty() {
        "(keine)".to_owned()
    } else {
        nummeriert
    };
    let system = format!(
        "{}{nummeriert}{}{regel_zeilen}{}",
        texte::THEMEN_KOPF,
        texte::THEMEN_MITTE,
        texte::THEMEN_ENDE
    );
    vec![Nachricht::system(system), Nachricht::nutzer(freitext)]
}

fn nummeriert(aussagen: &[Aussage]) -> String {
    aussagen
        .iter()
        .enumerate()
        .map(|(i, a)| format!("[{i}] {}", a.text))
        .collect::<Vec<_>>()
        .join("\n")
}

fn opt(s: Option<&str>) -> &str {
    s.unwrap_or("None")
}

/// Eine Feldzeile des Stufe-3-Katalogs.
fn feld_zeile(f: &KatalogFeld) -> String {
    let mut z = format!(
        "- {}: {} (Typ {}",
        f.feld_id,
        opt(f.fragetext_laie.as_deref()),
        opt(f.typ.as_deref())
    );
    if let Some(b) = f.bereich.as_ref().filter(|b| !b.0.is_empty()) {
        let _ = write!(z, ", Bereich {}", b.py_repr());
    }
    if let Some(w) = f.enum_werte.as_ref().filter(|w| !w.is_empty()) {
        let liste = w
            .iter()
            .map(|s| py::py_repr_str(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(z, ", Werte [{liste}]");
    }
    z.push(')');
    if let Some(h) = f.hilfe_kurz.as_deref().filter(|h| !h.is_empty()) {
        let _ = write!(z, "\n    dazu gehört: {h}");
    }
    z
}

/// Stufe 3 (`_dialog_prompt`), `kontext` bereits gefiltert (leer = kein Kontext).
///
/// ```
/// let (g, _) = llm::pii::filtere("x");
/// let (k, _) = llm::pii::filtere("");
/// let m = llm::prompt::dialog_prompt(&g, &[], &k, &[]);
/// assert!(m[0].inhalt().ends_with("Kein Fließtext außerhalb des JSON."));
/// ```
#[must_use]
pub fn dialog_prompt(
    freitext: &Gefiltert,
    katalog: &[&KatalogFeld],
    kontext: &Gefiltert,
    aussagen: &[Aussage],
) -> Vec<Nachricht> {
    let felder = katalog
        .iter()
        .map(|f| feld_zeile(f))
        .collect::<Vec<_>>()
        .join("\n");
    let mut system = format!("{}{felder}{}", texte::DIALOG_KOPF, texte::DIALOG_REGELN);
    if !aussagen.is_empty() {
        system.push_str(texte::AUSSAGEN_BLOCK_KOPF);
        system.push_str(&nummeriert(aussagen));
        system.push_str(texte::AUSSAGEN_BLOCK_ENDE);
    }
    if !kontext.as_str().is_empty() {
        system.push_str(kontext.as_str());
        system.push_str("\n\n");
    }
    system.push_str(texte::DIALOG_ENDE);
    vec![Nachricht::system(system), Nachricht::nutzer(freitext)]
}
