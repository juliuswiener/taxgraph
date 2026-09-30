//! Der Chat-Berater in drei Stufen (`_llm_dialog`, `api_llm.py:959-1176`).
//!
//! Aussagen (kein Katalog) → Themen (Regel-Kennungen) → Werte (nur Felder der getroffenen
//! Regeln). Ausfall Stufe 1 → Fehler an den Aufrufer (501); Stufe 2/3 → Teilergebnis mit den
//! Aussagen. Der Nutzertext wird GENAU EINMAL gefiltert, hier.
use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use serde_json::{json, Value};

use crate::client::{Chat, LlmFehler};
use crate::gates::{self, BelegterVorschlag, KatalogFeld};
use crate::parse::{self, Aussage, AussageStatus, Rueckfrage};
use crate::pii;
use crate::prompt;
use crate::schema;

/// Metadaten-Protokoll (Audit) und optionaler Klartext-Mitschnitt (Flow). Das Audit bekommt
/// NUR Metadaten; der Mitschnitt nur, wenn der Aufrufer ihn eingeschaltet hat.
pub trait Protokoll {
    /// Ein Audit-Eintrag (`audit.append(user, "llm_call", None, teil)`).
    fn melde(&self, teil: &str);
    /// `flow.schreibe(None, "ki", {"stufe", "was", "inhalt"})`.
    fn mitschnitt(&self, stufe: u8, was: &str, inhalt: &Value);
}

/// Kein Protokoll (Tests, Paritaet).
#[derive(Debug, Clone, Copy, Default)]
pub struct KeinProtokoll;

impl Protokoll for KeinProtokoll {
    fn melde(&self, _teil: &str) {}
    fn mitschnitt(&self, _stufe: u8, _was: &str, _inhalt: &Value) {}
}

/// Audit ueber `store::audit` (`produkt/store/audit.py`), ohne Mitschnitt.
#[derive(Debug, Clone)]
pub struct AuditProtokoll {
    pub pfad: std::path::PathBuf,
    pub user_id: Option<String>,
}

impl Protokoll for AuditProtokoll {
    fn melde(&self, teil: &str) {
        // Audit ist Nebenkanal; ein Schreibfehler darf die Antwort nicht kippen.
        let _ = store::audit::anhaengen(
            &self.pfad,
            Some(self.user_id.as_deref().unwrap_or("unbekannt")),
            store::audit::AuditAktion::LlmCall,
            None,
            Some(teil),
        );
    }
    fn mitschnitt(&self, _stufe: u8, _was: &str, _inhalt: &Value) {}
}

/// Rueckgabe von `_llm_dialog`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DialogErgebnis {
    pub vorschlaege: Vec<BelegterVorschlag>,
    pub antwort: String,
    pub unsicher: bool,
    pub aussagen: Vec<Aussage>,
    pub rueckfragen: Vec<Rueckfrage>,
    pub rueckfragen_zurueckgestellt: usize,
}

impl DialogErgebnis {
    fn leer(aussagen: Vec<Aussage>) -> Self {
        Self {
            vorschlaege: Vec::new(),
            antwort: String::new(),
            unsicher: false,
            aussagen,
            rueckfragen: Vec::new(),
            rueckfragen_zurueckgestellt: 0,
        }
    }
}

/// `_teilergebnis`: die Aussagen mit ihren Regeln und einem Ausfall-Status.
fn teilergebnis(
    mut aussagen: Vec<Aussage>,
    zuordnungen: &BTreeMap<i64, Vec<String>>,
    status: AussageStatus,
) -> DialogErgebnis {
    for (i, a) in aussagen.iter_mut().enumerate() {
        a.regeln = i64::try_from(i)
            .ok()
            .and_then(|k| zuordnungen.get(&k))
            .cloned()
            .unwrap_or_default();
        a.status = status;
    }
    DialogErgebnis::leer(aussagen)
}

fn py_liste(k: &[&str]) -> String {
    format!(
        "[{}]",
        k.iter()
            .map(|s| format!("'{s}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

/// Der Dialog. `gruppen` = `(gruppe, anzahl_feld)` aus `lade_instanz_gruppen()`.
///
/// ```
/// struct Nie;
/// impl llm::Chat for Nie {
///     fn complete(&self, _: &[llm::Nachricht], _: Option<&serde_json::Value>) -> Result<llm::Completion, llm::LlmFehler> {
///         unreachable!()
///     }
/// }
/// let e = llm::dialog::llm_dialog(&Nie, "   ", &[], "", &[], &llm::dialog::KeinProtokoll).unwrap();
/// assert!(e.aussagen.is_empty());
/// ```
///
/// # Errors
/// [`LlmFehler`] nur aus Stufe 1.
pub fn llm_dialog(
    chat: &dyn Chat,
    freitext: &str,
    katalog: &[KatalogFeld],
    kontext: &str,
    gruppen: &[(String, String)],
    protokoll: &dyn Protokoll,
) -> Result<DialogErgebnis, LlmFehler> {
    if crate::py::strip(freitext).is_empty() {
        return Ok(DialogErgebnis::leer(Vec::new()));
    }
    let (gefiltert, kategorien) = pii::filtere(freitext);
    let (kontext_gefiltert, kategorien_k) = pii::filtere(kontext);
    let m = Melder {
        kopf: format!(
            "pii_kategorien={}, kontext_kategorien={}, textlaenge_vor={}, textlaenge_nach={}",
            py_liste(&kategorien),
            py_liste(&kategorien_k),
            crate::py::laenge(freitext),
            crate::py::laenge(gefiltert.as_str())
        ),
        protokoll,
    };

    let aussagen = stufe_aussagen(chat, &gefiltert, &m)?;
    let je_regel = gates::felder_je_regel(katalog);
    let Some(zuordnung) = stufe_themen(chat, &gefiltert, &aussagen, &je_regel, &m) else {
        return Ok(teilergebnis(
            aussagen,
            &BTreeMap::new(),
            AussageStatus::ThemenAusgefallen,
        ));
    };
    let eng = !zuordnung.getroffen.is_empty();
    let kat3 = gates::mit_zaehlfeldern(
        katalog_stufe3(&zuordnung, &je_regel, katalog),
        katalog,
        gruppen,
    );
    let c3 = match chat.complete(
        &prompt::dialog_prompt(&gefiltert, &kat3, &kontext_gefiltert, &aussagen),
        Some(&schema::DIALOG_SCHEMA),
    ) {
        Ok(c) => c,
        Err(e) => {
            m.gescheitert(3, &e);
            return Ok(teilergebnis(
                aussagen,
                &zuordnung.je_aussage,
                AussageStatus::WerteAusgefallen,
            ));
        }
    };
    Ok(stufe_werte(
        &c3, &gefiltert, aussagen, &zuordnung, &kat3, eng, &m,
    ))
}

/// Audit-Kopf und Ausfall-Meldung, gemeinsam fuer alle drei Stufen.
struct Melder<'a> {
    kopf: String,
    protokoll: &'a dyn Protokoll,
}

impl Melder<'_> {
    fn melde(&self, teil: &str) {
        self.protokoll.melde(&format!("{}, {teil}", self.kopf));
    }

    fn gescheitert(&self, stufe: u8, e: &LlmFehler) {
        let grund = if e.grund().is_empty() {
            "sonstiger_fehler"
        } else {
            e.grund()
        };
        self.melde(&format!(
            "stufe={stufe}, ergebnis=kein_ergebnis, grund={grund}, versuche={}, provider=''",
            e.versuche()
        ));
        self.protokoll.mitschnitt(
            stufe,
            "ausgefallen",
            &json!({"grund": grund, "versuche": e.versuche(), "provider": ""}),
        );
    }
}

/// Stufe 1: Aussagen ohne Katalog. Ein Ausfall geht als Fehler an den Aufrufer.
fn stufe_aussagen(
    chat: &dyn Chat,
    gefiltert: &pii::Gefiltert,
    m: &Melder<'_>,
) -> Result<Vec<Aussage>, LlmFehler> {
    let c1 = chat
        .complete(
            &prompt::aussagen_prompt(gefiltert),
            Some(&schema::AUSSAGEN_SCHEMA),
        )
        .inspect_err(|e| m.gescheitert(1, e))?;
    let aussagen = parse::aussagen_parse(&c1.text, gefiltert).oder_leer_wie_python();
    m.protokoll.mitschnitt(
        1,
        "aussagen",
        &serde_json::to_value(&aussagen).unwrap_or_default(),
    );
    m.melde(&format!(
        "stufe=1, aussagen={}, aussagen_ohne_beleg={}, inhalt_laenge={}, provider='{}', finish='{}'",
        aussagen.len(),
        aussagen.iter().filter(|a| a.beleg.is_empty()).count(),
        crate::py::laenge(&c1.text),
        c1.provider,
        c1.finish
    ));
    Ok(aussagen)
}

/// Stufe 2: Aussagen auf Regeln abbilden. `None` = Stufe ausgefallen (Teilergebnis).
fn stufe_themen(
    chat: &dyn Chat,
    gefiltert: &pii::Gefiltert,
    aussagen: &[Aussage],
    je_regel: &[(String, Vec<&KatalogFeld>)],
    m: &Melder<'_>,
) -> Option<parse::Zuordnung> {
    let mut verfuegbar: Vec<String> = je_regel
        .iter()
        .map(|(r, _)| r.clone())
        .filter(|r| !r.is_empty())
        .collect();
    verfuegbar.sort();
    if verfuegbar.is_empty() {
        return Some(parse::Zuordnung::default());
    }
    let nachrichten = prompt::themen_prompt(
        gefiltert,
        aussagen,
        &prompt::regel_zeilen(je_regel, &verfuegbar),
    );
    let c2 = match chat.complete(&nachrichten, Some(&schema::ZUORDNUNG_SCHEMA)) {
        Ok(c) => c,
        Err(e) => {
            m.gescheitert(2, &e);
            return None;
        }
    };
    let erlaubt: HashSet<String> = verfuegbar.iter().cloned().collect();
    let zuordnung =
        parse::zuordnung_parse(&c2.text, &erlaubt, aussagen.len()).oder_leer_wie_python();
    m.protokoll.mitschnitt(
        2,
        "zuordnungen",
        &serde_json::to_value(&zuordnung).unwrap_or_default(),
    );
    m.melde(&format!(
        "stufe=2, aussagen={}, zugeordnet={}, regeln={}/{}, inhalt_laenge={}, provider='{}', finish='{}'",
        aussagen.len(),
        zuordnung.je_aussage.len(),
        zuordnung.getroffen.len(),
        verfuegbar.len(),
        crate::py::laenge(&c2.text),
        c2.provider,
        c2.finish
    ));
    Some(zuordnung)
}

/// Katalog fuer Stufe 3: nur die Felder der getroffenen Regeln (plus regellose), sonst alle.
fn katalog_stufe3<'a>(
    zuordnung: &parse::Zuordnung,
    je_regel: &[(String, Vec<&'a KatalogFeld>)],
    katalog: &'a [KatalogFeld],
) -> Vec<&'a KatalogFeld> {
    if zuordnung.getroffen.is_empty() {
        return katalog.iter().collect();
    }
    let aus = |r: &str| {
        je_regel
            .iter()
            .find(|(k, _)| k == r)
            .map(|(_, v)| v.clone())
            .unwrap_or_default()
    };
    zuordnung
        .getroffen
        .iter()
        .flat_map(|r| aus(r))
        .chain(aus(""))
        .collect()
}

/// Stufe 3 auswerten: Vorschlaege, Rueckfragen, Antwort, Status je Aussage.
fn stufe_werte(
    c3: &crate::client::Completion,
    gefiltert: &pii::Gefiltert,
    mut aussagen: Vec<Aussage>,
    zuordnung: &parse::Zuordnung,
    kat3: &[&KatalogFeld],
    eng: bool,
    m: &Melder<'_>,
) -> DialogErgebnis {
    let (behalten, verworfen) = gates::beleg_geprueft(
        parse::chat_parse(&c3.text).oder_leer_wie_python(),
        gefiltert,
    );
    let rueckfragen = parse::rueckfragen_parse(&c3.text, kat3.len()).oder_leer_wie_python();
    let (rueckfragen, geloest) = gates::rueckfragen_gebunden(rueckfragen, kat3);
    let (rueckfragen, zurueckgestellt) = gates::rueckfragen_gebuendelt(rueckfragen);
    let behalten = gates::rueckfrage_verdraengt(behalten, &rueckfragen);
    let (antwort, unsicher) = parse::antwort_parse(&c3.text).oder_leer_wie_python();
    gates::status_setzen(
        &mut aussagen,
        &zuordnung.je_aussage,
        &behalten,
        &verworfen,
        &rueckfragen,
    );
    m.protokoll.mitschnitt(
        3,
        "ergebnis",
        &json!({"vorschlaege": behalten, "ohne_beleg_verworfen": verworfen, "rueckfragen": rueckfragen,
                "rueckfragen_geloest": geloest, "antwort": antwort, "unsicher": unsicher, "aussagen": aussagen}),
    );
    m.melde(&format!(
        "stufe=3, katalog={}, katalog_felder={}, vorschlaege={}, ohne_beleg_verworfen={}, rueckfragen={}, rueckfragen_zurueckgestellt={zurueckgestellt}, rueckfragen_geloest={}, offen={}, antwortlaenge={}, unsicher={}, inhalt_laenge={}, provider='{}', finish='{}'",
        if eng { "eng" } else { "voll" },
        kat3.len(),
        behalten.len(),
        verworfen.len(),
        rueckfragen.len(),
        geloest.len(),
        aussagen.iter().filter(|a| !matches!(a.status, AussageStatus::Vorschlag | AussageStatus::Rueckfrage)).count(),
        crate::py::laenge(&antwort),
        if unsicher { "True" } else { "False" },
        crate::py::laenge(&c3.text),
        c3.provider,
        c3.finish
    ));
    DialogErgebnis {
        vorschlaege: behalten,
        antwort,
        unsicher,
        aussagen,
        rueckfragen,
        rueckfragen_zurueckgestellt: zurueckgestellt,
    }
}
