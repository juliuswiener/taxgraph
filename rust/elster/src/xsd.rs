//! Das amtliche E10-/E77-XSD als einzige Quelle fuer Kz-Pfade, Pflicht-Diskriminatoren und
//! Kz-Typen (`produkt/mapping/xsd_verify.py`, `elster_xml.py::kz_pfade`/`pflicht_kinder`).
//!
//! Der Walk laeuft top-down ueber LOKALE `xs:element`-Namen ab dem Start-Element; `xs:sequence`/
//! `xs:choice`/`xs:all` sind Struktur ohne eigene Pfad-Ebene. Laufzeit-Nutzer ist der XML-Writer
//! ([`crate::erzeuge_xml`]); [`pruefe_bindung`] ist das Test-/CI-Werkzeug (`xsd_verify.main`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use domain::Kz;
use roxmltree::{Document, Node};
use serde::Serialize;

use crate::deklaration::BindungIndex;
use crate::geordnet::Geordnet;
use crate::tabellen::{DOKUMENTIERT_AGGREGAT, NEGATION, PARTNER_VERZWEIGUNG, VERZWEIGUNG};

const XS: &str = "http://www.w3.org/2001/XMLSchema";

/// Rekursions-Backstop; das echte E10-Schema erreicht Tiefe 8.
pub const MAX_DEPTH: usize = 80;

/// Das XSD liess sich nicht lesen oder auswerten.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XsdFehler {
    #[error("Schema {pfad}: {nachricht}")]
    Lesen { pfad: PathBuf, nachricht: String },
    #[error("Start-Element '{start}' nicht im Schema {pfad} gefunden")]
    StartFehlt { start: String, pfad: PathBuf },
    /// ponytail: `xs:include`/`xs:import` werden nicht aufgeloest (E10/E77 2020-2025 haben keins);
    /// statt still weniger zu finden als Python, bricht der Port ab. Upgrade: Texte vorab sammeln.
    #[error("Schema {0} bindet weitere Schemata ein (xs:include/xs:import) — nicht unterstuetzt")]
    Include(PathBuf),
}

/// Python `glob(root/**/name, recursive=True)`: ohne versteckte Eintraege, Symlinks folgend.
fn glob_rekursiv(verzeichnis: &Path, name: &str, treffer: &mut Vec<PathBuf>, tiefe: usize) {
    if tiefe > 64 {
        return;
    }
    let Ok(eintraege) = std::fs::read_dir(verzeichnis) else {
        return;
    };
    for e in eintraege.flatten() {
        let dateiname = e.file_name();
        let Some(n) = dateiname.to_str() else {
            continue;
        };
        if n.starts_with('.') {
            continue;
        }
        let pfad = e.path();
        if n == name && pfad.is_file() {
            treffer.push(pfad.clone());
        }
        if pfad.is_dir() {
            glob_rekursiv(&pfad, name, treffer, tiefe + 1);
        }
    }
}

fn expand_home(p: &str) -> PathBuf {
    match (p.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ if p == "~" => std::env::var_os("HOME").map_or_else(|| PathBuf::from(p), PathBuf::from),
        _ => PathBuf::from(p),
    }
}

/// Suchwurzeln der ERiC-Auslieferung: `$ERIC_DIR`, dann `~/02_Software/eric`.
pub(crate) fn eric_wurzeln() -> Vec<PathBuf> {
    let mut wurzeln = Vec::new();
    if let Ok(d) = std::env::var("ERIC_DIR") {
        if !d.is_empty() {
            wurzeln.push(expand_home(&d));
        }
    }
    wurzeln.push(expand_home("~/02_Software/eric"));
    wurzeln
}

/// Erste Datei `name` (sortiert) unter den ERiC-Wurzeln (`xsd_verify._find_schema`).
pub(crate) fn finde_datei(name: &str) -> Option<PathBuf> {
    finde_datei_in(&eric_wurzeln(), name)
}

pub(crate) fn finde_datei_in(wurzeln: &[PathBuf], name: &str) -> Option<PathBuf> {
    for wurzel in wurzeln {
        let mut treffer = Vec::new();
        glob_rekursiv(wurzel, name, &mut treffer, 0);
        treffer.sort();
        if let Some(p) = treffer.into_iter().next() {
            return Some(p);
        }
    }
    None
}

/// Lokaler Schema-Pfad fuer ein Jahr (`_find_schema`); `muster` enthaelt `{jahr}`.
///
/// ```
/// let p = elster::finde_schema(1999, "E10-{jahr}.xsd");
/// assert!(p.is_none());
/// ```
#[must_use]
pub fn finde_schema(jahr: i64, muster: &str) -> Option<PathBuf> {
    finde_datei(&muster.replace("{jahr}", &jahr.to_string()))
}

fn ist_xs(n: Node<'_, '_>, name: &str) -> bool {
    n.is_element() && n.tag_name().name() == name && n.tag_name().namespace() == Some(XS)
}

fn kinder<'a, 'i>(n: Node<'a, 'i>) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children().filter(Node::is_element)
}

/// `^E\d{7}$`; die Regel steht in [`Kz::ist_gueltig`].
fn ist_kz(name: &str) -> bool {
    Kz::ist_gueltig(name)
}

/// Python `node.get("name") or node.get("ref")`.
fn lokaler_name<'a>(n: Node<'a, '_>) -> &'a str {
    n.attribute("name")
        .filter(|s| !s.is_empty())
        .or_else(|| n.attribute("ref"))
        .unwrap_or("")
}

/// Die drei Indizes ueber die Top-Level-Kinder des Schemas.
struct Indizes<'a, 'i> {
    typen: HashMap<&'a str, Node<'a, 'i>>,
    gruppen: HashMap<&'a str, Node<'a, 'i>>,
    elemente: HashMap<&'a str, Node<'a, 'i>>,
}

impl<'a, 'i> Indizes<'a, 'i> {
    fn neu(doc: &'a Document<'i>, pfad: &Path) -> Result<Self, XsdFehler> {
        let wurzel = doc.root_element();
        let mut ix = Self {
            typen: HashMap::new(),
            gruppen: HashMap::new(),
            elemente: HashMap::new(),
        };
        for kind in kinder(wurzel) {
            if (ist_xs(kind, "include") || ist_xs(kind, "import"))
                && kind
                    .attribute("schemaLocation")
                    .is_some_and(|loc| pfad.with_file_name(loc).exists())
            {
                return Err(XsdFehler::Include(pfad.to_path_buf()));
            }
            let Some(name) = kind.attribute("name").filter(|s| !s.is_empty()) else {
                continue;
            };
            if ist_xs(kind, "complexType") {
                ix.typen.insert(name, kind);
            } else if ist_xs(kind, "group") {
                ix.gruppen.insert(name, kind);
            } else if ist_xs(kind, "element") {
                ix.elemente.insert(name, kind);
            }
        }
        Ok(ix)
    }

    /// Inline-`complexType` oder benannter Typ; `None` = Blatt.
    fn inhalt(&self, n: Node<'a, 'i>) -> Option<Node<'a, 'i>> {
        if let Some(ct) = kinder(n).find(|k| ist_xs(*k, "complexType")) {
            return Some(ct);
        }
        n.attribute("type")
            .filter(|t| !t.is_empty())
            .and_then(|t| self.typen.get(t).copied())
    }

    fn start(&self, start: &str, pfad: &Path) -> Result<Node<'a, 'i>, XsdFehler> {
        self.elemente
            .get(start)
            .copied()
            .ok_or_else(|| XsdFehler::StartFehlt {
                start: start.to_owned(),
                pfad: pfad.to_path_buf(),
            })
    }
}

/// Rekursiv durch `xs:sequence`/`xs:choice`/`xs:all` → `xs:element`/`xs:group`.
fn flach<'a, 'i>(container: Node<'a, 'i>) -> Vec<Node<'a, 'i>> {
    let mut out = Vec::new();
    for k in kinder(container) {
        if ist_xs(k, "sequence") || ist_xs(k, "choice") || ist_xs(k, "all") {
            out.extend(flach(k));
        } else if ist_xs(k, "element") || ist_xs(k, "group") {
            out.push(k);
        }
    }
    out
}

/// Die Kind-Elemente, in die der Walk absteigt (Element-Refs aufgeloest, Gruppen expandiert).
fn abstieg<'a, 'i>(ix: &Indizes<'a, 'i>, inhalt: Node<'a, 'i>) -> Vec<Node<'a, 'i>> {
    let mut ziele = Vec::new();
    for kind in flach(inhalt) {
        if ist_xs(kind, "element") {
            match kind.attribute("ref").filter(|r| !r.is_empty()) {
                Some(r) => ziele.extend(ix.elemente.get(r).copied()),
                None => ziele.push(kind),
            }
        } else if let Some(g) = kind.attribute("ref").and_then(|r| ix.gruppen.get(r)) {
            // Wie im Original: Element-Refs INNERHALB einer Gruppe werden nicht aufgeloest.
            ziele.extend(flach(*g).into_iter().filter(|gc| ist_xs(*gc, "element")));
        }
    }
    ziele
}

fn lies(pfad: &Path) -> Result<String, XsdFehler> {
    std::fs::read_to_string(pfad).map_err(|e| XsdFehler::Lesen {
        pfad: pfad.to_path_buf(),
        nachricht: e.to_string(),
    })
}

fn parse<'i>(text: &'i str, pfad: &Path) -> Result<Document<'i>, XsdFehler> {
    let opt = roxmltree::ParsingOptions {
        allow_dtd: true,
        ..roxmltree::ParsingOptions::default()
    };
    Document::parse_with_options(text, opt).map_err(|e| XsdFehler::Lesen {
        pfad: pfad.to_path_buf(),
        nachricht: e.to_string(),
    })
}

/// Kz → alle Fundstellen (Pfad-Tupel), in Schema-Reihenfolge des ersten Auftretens.
pub type KzFundstellen = Vec<(String, Vec<Vec<String>>)>;

/// Top-Down-Walk ab `start` (`xsd_verify.walk`): jede Kz-Fundstelle einzeln, plus die Zahl der
/// `MAX_DEPTH`-Abbrueche.
///
/// # Errors
/// [`XsdFehler`], wenn das Schema nicht lesbar ist oder `start` fehlt.
///
/// ```
/// if let Some(p) = elster::finde_schema(2025, "E10-{jahr}.xsd") {
///     let (kz, abbrueche) = elster::xsd_walk(&p, "E10").unwrap();
///     assert!(kz.len() > 2000);
///     assert_eq!(abbrueche, 0);
/// }
/// ```
pub fn xsd_walk(pfad: &Path, start: &str) -> Result<(KzFundstellen, usize), XsdFehler> {
    let text = lies(pfad)?;
    let doc = parse(&text, pfad)?;
    let ix = Indizes::neu(&doc, pfad)?;
    let mut index: Geordnet<String, Vec<Vec<String>>> = Geordnet::default();
    let mut abbrueche = 0;
    let mut weg = Vec::new();
    walk_rek(
        &ix,
        ix.start(start, pfad)?,
        &mut weg,
        0,
        &mut index,
        &mut abbrueche,
    );
    Ok((index.in_reihenfolge(), abbrueche))
}

fn walk_rek<'a, 'i>(
    ix: &Indizes<'a, 'i>,
    knoten: Node<'a, 'i>,
    weg: &mut Vec<String>,
    tiefe: usize,
    index: &mut Geordnet<String, Vec<Vec<String>>>,
    abbrueche: &mut usize,
) {
    if tiefe > MAX_DEPTH {
        *abbrueche += 1;
        return;
    }
    let name = lokaler_name(knoten);
    weg.push(name.to_owned());
    if ist_kz(name) {
        index.eintrag(name.to_owned(), Vec::new).push(weg.clone());
    }
    if let Some(inhalt) = ix.inhalt(knoten) {
        for ziel in abstieg(ix, inhalt) {
            walk_rek(ix, ziel, weg, tiefe + 1, index, abbrueche);
        }
    }
    weg.pop();
}

/// Typ-Metadaten einer Kz (`_resolve_kz_meta`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KzMeta {
    pub type_name: String,
    pub enums: Vec<String>,
    pub patterns: Vec<String>,
    /// Ja-Typ-Familie (Ja1/JaX/JaNein12/Ja2): Ankreuz- bzw. Ja/Nein-Feld.
    pub is_ja: bool,
}

/// `^Ja(1|X|Nein12|2)?BaseCType(_RABE)?$` (`xsd_verify.ist_ja_typ`).
///
/// ```
/// assert!(elster::ist_ja_typ("JaNein12BaseCType_RABE"));
/// assert!(!elster::ist_ja_typ("Enum_Ja_BaseCType"));
/// ```
#[must_use]
pub fn ist_ja_typ(typ: &str) -> bool {
    let Some(rest) = typ.strip_prefix("Ja") else {
        return false;
    };
    let rest = ["1", "X", "Nein12", "2"]
        .iter()
        .find_map(|v| rest.strip_prefix(v))
        .unwrap_or(rest);
    matches!(rest, "BaseCType" | "BaseCType_RABE")
}

struct Facetten {
    enums: Vec<String>,
    patterns: Vec<String>,
}

/// Folgt der Vererbungskette (`simpleContent` → `restriction`/`extension` → `base`) und nimmt
/// die Facetten der tiefsten Stufe, die welche hat (`_resolve_type_facets`).
fn facetten(ix: &Indizes<'_, '_>, typ: &str, gesehen: &mut HashSet<String>) -> Facetten {
    let leer = Facetten {
        enums: Vec::new(),
        patterns: Vec::new(),
    };
    let Some(knoten) = ix
        .typen
        .get(typ)
        .copied()
        .filter(|_| !gesehen.contains(typ))
    else {
        return leer;
    };
    gesehen.insert(typ.to_owned());
    for sc in kinder(knoten).filter(|k| ist_xs(*k, "simpleContent")) {
        if let Some(ch) = kinder(sc).find(|k| ist_xs(*k, "restriction") || ist_xs(*k, "extension"))
        {
            let werte = |facette: &str| -> Vec<String> {
                kinder(ch)
                    .filter(|e| ist_xs(*e, facette))
                    .map(|e| e.attribute("value").unwrap_or_default().to_owned())
                    .collect()
            };
            let (enums, patterns) = (werte("enumeration"), werte("pattern"));
            let sub = facetten(ix, ch.attribute("base").unwrap_or_default(), gesehen);
            return Facetten {
                enums: if enums.is_empty() { sub.enums } else { enums },
                patterns: if patterns.is_empty() {
                    sub.patterns
                } else {
                    patterns
                },
            };
        }
    }
    leer
}

fn meta_rek<'a, 'i>(
    ix: &Indizes<'a, 'i>,
    knoten: Node<'a, 'i>,
    tiefe: usize,
    out: &mut HashMap<String, KzMeta>,
) {
    if tiefe > MAX_DEPTH {
        return;
    }
    let name = lokaler_name(knoten);
    if ist_kz(name) {
        if let Some(typ) = knoten.attribute("type").filter(|t| !t.is_empty()) {
            let f = facetten(ix, typ, &mut HashSet::new());
            out.insert(
                name.to_owned(),
                KzMeta {
                    type_name: typ.to_owned(),
                    enums: f.enums,
                    patterns: f.patterns,
                    is_ja: ist_ja_typ(typ),
                },
            );
        }
    }
    if let Some(inhalt) = ix.inhalt(knoten) {
        for ziel in abstieg(ix, inhalt) {
            meta_rek(ix, ziel, tiefe + 1, out);
        }
    }
}

/// Container-Pfad → skalare Pflicht-Kinder ohne Kz (z.B. `<Person>` in Anlage N).
pub type PflichtKinder = HashMap<Vec<String>, Vec<String>>;

fn hat_element_kinder(ix: &Indizes<'_, '_>, n: Node<'_, '_>) -> bool {
    ix.inhalt(n)
        .is_some_and(|c| flach(c).iter().any(|k| ist_xs(*k, "element")))
}

struct PflichtLauf<'x, 'a, 'i> {
    ix: &'x Indizes<'a, 'i>,
    gesehen: HashSet<Vec<String>>,
    treffer: PflichtKinder,
}

impl<'a, 'i> PflichtLauf<'_, 'a, 'i> {
    fn rek(&mut self, knoten: Node<'a, 'i>, weg: &mut Vec<String>, tiefe: usize) {
        weg.push(lokaler_name(knoten).to_owned());
        if tiefe <= MAX_DEPTH && !self.gesehen.contains(weg.as_slice()) {
            self.gesehen.insert(weg.clone());
            if let Some(inhalt) = self.ix.inhalt(knoten) {
                let mut pflicht = Vec::new();
                for kind in flach(inhalt).into_iter().filter(|k| ist_xs(*k, "element")) {
                    let r = kind.attribute("ref").filter(|r| !r.is_empty());
                    let kind_name = r.or_else(|| kind.attribute("name")).unwrap_or("");
                    let ziel = match r {
                        Some(r) => self.ix.elemente.get(r).copied(),
                        None => Some(kind),
                    };
                    let pflichtig =
                        kind.attribute("minOccurs").unwrap_or("1") != "0" && !ist_kz(kind_name);
                    if pflichtig && ziel.is_some_and(|z| !hat_element_kinder(self.ix, z)) {
                        pflicht.push(kind_name.to_owned());
                    }
                    if let Some(z) = ziel {
                        self.rek(z, weg, tiefe + 1);
                    }
                }
                if !pflicht.is_empty() {
                    self.treffer.insert(weg.clone(), pflicht);
                }
            }
        }
        weg.pop();
    }
}

/// Was der XML-Writer je VZ aus dem Schema braucht.
#[derive(Debug, Clone)]
pub struct SchemaInfo {
    /// Kz → kuerzester Pfad, in Schema-Reihenfolge (die Geschwister-Reihenfolge des Writers).
    pub pfade: Vec<(String, Vec<String>)>,
    pub pflicht: PflichtKinder,
    pub kz_meta: HashMap<String, KzMeta>,
}

/// Kz-Pfade, Pflicht-Kinder und Kz-Typen in einem Durchgang (`kz_pfade`, `pflicht_kinder`,
/// `_resolve_kz_meta`). Die Zusatz-Pflichtkinder aus `ERIC_PFLICHT_TROTZ_OPTIONAL` sind schon
/// eingemischt.
///
/// # Errors
/// [`XsdFehler`], s. [`xsd_walk`].
///
/// ```
/// if let Some(p) = elster::finde_schema(2025, "E10-{jahr}.xsd") {
///     let info = elster::schema_info(&p).unwrap();
///     assert!(info.pflicht.contains_key(&vec!["E10".to_string(), "V".to_string()]));
/// }
/// ```
pub fn schema_info(pfad: &Path) -> Result<SchemaInfo, XsdFehler> {
    let (roh, _) = xsd_walk(pfad, "E10")?;
    let pfade = roh
        .into_iter()
        .filter_map(|(kz, fundstellen)| {
            // Python `min(fundstellen, key=len)`: bei Gleichstand gewinnt die erste.
            let mut beste: Option<Vec<String>> = None;
            for f in fundstellen {
                if beste.as_ref().is_none_or(|b| f.len() < b.len()) {
                    beste = Some(f);
                }
            }
            beste.map(|p| (kz, p))
        })
        .collect();
    let text = lies(pfad)?;
    let doc = parse(&text, pfad)?;
    let ix = Indizes::neu(&doc, pfad)?;
    let start = ix.start("E10", pfad)?;
    let mut lauf = PflichtLauf {
        ix: &ix,
        gesehen: HashSet::new(),
        treffer: HashMap::new(),
    };
    lauf.rek(start, &mut Vec::new(), 0);
    let mut pflicht = lauf.treffer;
    // Gemessen 2026-08-16: ERiC verlangt Laufende_Nummer_V trotz minOccurs=0.
    let v = pflicht
        .entry(vec!["E10".to_owned(), "V".to_owned()])
        .or_default();
    if !v.iter().any(|k| k == "Laufende_Nummer_V") {
        v.push("Laufende_Nummer_V".to_owned());
    }
    let mut kz_meta = HashMap::new();
    meta_rek(&ix, start, 0, &mut kz_meta);
    Ok(SchemaInfo {
        pfade,
        pflicht,
        kz_meta,
    })
}

/// Kz-Typen ab `start` (`_resolve_kz_meta`).
///
/// # Errors
/// [`XsdFehler`], s. [`xsd_walk`].
///
/// ```
/// if let Some(p) = elster::finde_schema(2025, "E10-{jahr}.xsd") {
///     assert!(elster::kz_meta(&p, "E10").unwrap()["E0100001"].is_ja);
/// }
/// ```
pub fn kz_meta(pfad: &Path, start: &str) -> Result<HashMap<String, KzMeta>, XsdFehler> {
    let text = lies(pfad)?;
    let doc = parse(&text, pfad)?;
    let ix = Indizes::neu(&doc, pfad)?;
    let mut out = HashMap::new();
    meta_rek(&ix, ix.start(start, pfad)?, 0, &mut out);
    Ok(out)
}

// ---------------------------------------------------------------- Pruef-Werkzeug (CI-Gate)

/// Ergebnis je Kz und Jahr.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PruefStatus {
    #[serde(rename = "OK")]
    Ok,
    #[serde(rename = "AMBIGUOUS")]
    Mehrdeutig,
    #[serde(rename = "NOT_FOUND")]
    NichtGefunden,
    #[serde(rename = "SCHEMA_UNVERFUEGBAR")]
    SchemaUnverfuegbar,
}

/// Eine zu pruefende Kz mit ihren Gueltigkeitsjahren.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KzPruefling {
    pub feld_id: String,
    pub elster_kz: String,
    pub vz_gueltigkeit: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct JahrPruefung {
    pub status: PruefStatus,
    pub pfade: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FeldPruefung {
    pub status: PruefStatus,
    pub jahre: BTreeMap<i64, JahrPruefung>,
}

/// Bericht von [`pruefe_bindung`]; `exit_code == 0` genau dann, wenn alles `OK` ist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Pruefbericht {
    pub felder: BTreeMap<String, FeldPruefung>,
    pub unverfuegbare_jahre: Vec<i64>,
    pub max_depth_hits_gesamt: usize,
    pub exit_code: i32,
}

/// Die Kz-Literale der Transform-Tabellen als Prueflinge (`ernte_est_mapping_kz`): sie stehen in
/// keiner Bindung als `elster_kz` und waeren fuer den Pruefpass sonst unsichtbar.
///
/// # Errors
/// Die `feld_id` eines Wert-Felds, das in der Bindung fehlt (Python: `KeyError`).
///
/// ```
/// assert!(elster::ernte_est_mapping_kz(&std::collections::HashMap::new()).is_err());
/// ```
pub fn ernte_est_mapping_kz(bindung: &BindungIndex<'_>) -> Result<Vec<KzPruefling>, String> {
    let vz = |f: &str| {
        bindung
            .get(f)
            .map(|b| b.vz_gueltigkeit.clone())
            .ok_or_else(|| f.to_owned())
    };
    let mut out = Vec::new();
    for (feld, kz) in NEGATION {
        out.push(KzPruefling {
            feld_id: format!("negation:{feld}"),
            elster_kz: (*kz).to_owned(),
            vz_gueltigkeit: vz(feld)?,
        });
    }
    for v in VERZWEIGUNG.iter().chain(PARTNER_VERZWEIGUNG) {
        let jahre = vz(v.feld)?;
        for (art, kz) in v.kz.paare() {
            out.push(KzPruefling {
                feld_id: format!("verzweigung:{}:{art}", v.feld),
                elster_kz: kz.to_owned(),
                vz_gueltigkeit: jahre.clone(),
            });
        }
    }
    for (kz, quellen) in DOKUMENTIERT_AGGREGAT {
        let mut jahre = BTreeSet::new();
        for q in *quellen {
            jahre.extend(vz(q)?);
        }
        out.push(KzPruefling {
            feld_id: format!("aggregation:{kz}"),
            elster_kz: (*kz).to_owned(),
            vz_gueltigkeit: jahre.into_iter().collect(),
        });
    }
    Ok(out)
}

/// Datenart-Routing nach Kz-Praefix: `E60…` gegen E77 (Anlage EUeR), sonst E10.
///
/// ponytail: bleibt Python-treu bei `kz[1:3] == "60"` (`xsd_verify.py:79`), nicht
/// [`Kz::hat_e60_praefix`]. Beide Regeln sind nur fuer gueltige Kz gleich (`X6000000`: hier E77,
/// dort kein E60); `get(1..3)` zaehlt zudem Bytes, Python Zeichen. Die Bindung lehnt ungueltige Kz
/// schon beim Laden ab (`bindung_datei.rs:272`). Angleichung mit K9 (Kz an der Quelle).
fn datenart(kz: &str) -> (&'static str, &'static str) {
    if kz.get(1..3) == Some("60") {
        ("E77-{jahr}.xsd", "E77")
    } else {
        ("E10-{jahr}.xsd", "E10")
    }
}

fn rollup(jahre: &BTreeMap<i64, JahrPruefung>) -> PruefStatus {
    let hat = |s: PruefStatus| jahre.values().any(|j| j.status == s);
    if hat(PruefStatus::Mehrdeutig) {
        PruefStatus::Mehrdeutig
    } else if hat(PruefStatus::NichtGefunden) {
        PruefStatus::NichtGefunden
    } else if hat(PruefStatus::Ok) {
        PruefStatus::Ok
    } else {
        PruefStatus::SchemaUnverfuegbar
    }
}

/// Prueft je Kz und Gueltigkeitsjahr, ob die Kz genau einmal im lokalen Schema steht
/// (`xsd_verify.pruefe_bindung` mit Datenart-Routing).
///
/// # Errors
/// [`XsdFehler`], wenn ein gefundenes Schema nicht auswertbar ist.
///
/// ```
/// let b = elster::pruefe_bindung(&[]).unwrap();
/// assert_eq!(b.exit_code, 0);
/// ```
pub fn pruefe_bindung(prueflinge: &[KzPruefling]) -> Result<Pruefbericht, XsdFehler> {
    type Walk = (HashMap<String, Vec<Vec<String>>>, usize);
    let mut cache: HashMap<(PathBuf, &str), Walk> = HashMap::new();
    let mut felder = BTreeMap::new();
    let mut unverfuegbar = BTreeSet::new();
    for p in prueflinge.iter().filter(|p| !p.elster_kz.is_empty()) {
        let (muster, start) = datenart(&p.elster_kz);
        let mut jahre = BTreeMap::new();
        for jahr in &p.vz_gueltigkeit {
            let Some(schema) = finde_schema(*jahr, muster) else {
                jahre.insert(
                    *jahr,
                    JahrPruefung {
                        status: PruefStatus::SchemaUnverfuegbar,
                        pfade: Vec::new(),
                    },
                );
                unverfuegbar.insert(*jahr);
                continue;
            };
            let schluessel = (schema.clone(), start);
            if !cache.contains_key(&schluessel) {
                let (index, abbrueche) = xsd_walk(&schema, start)?;
                cache.insert(schluessel.clone(), (index.into_iter().collect(), abbrueche));
            }
            let kandidaten = cache
                .get(&schluessel)
                .and_then(|(ix, _)| ix.get(&p.elster_kz))
                .cloned()
                .unwrap_or_default();
            let status = match kandidaten.len() {
                0 => PruefStatus::NichtGefunden,
                1 => PruefStatus::Ok,
                _ => PruefStatus::Mehrdeutig,
            };
            jahre.insert(
                *jahr,
                JahrPruefung {
                    status,
                    pfade: kandidaten.iter().map(|k| k.join("/")).collect(),
                },
            );
        }
        felder.insert(
            p.feld_id.clone(),
            FeldPruefung {
                status: rollup(&jahre),
                jahre,
            },
        );
    }
    let abbrueche: usize = cache.values().map(|(_, a)| a).sum();
    let alle_ok = felder
        .values()
        .all(|f: &FeldPruefung| f.status == PruefStatus::Ok);
    Ok(Pruefbericht {
        felder,
        unverfuegbare_jahre: unverfuegbar.into_iter().collect(),
        max_depth_hits_gesamt: abbrueche,
        exit_code: i32::from(!(alle_ok && abbrueche == 0)),
    })
}
