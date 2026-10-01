//! Store-Snapshot → ELSTER-Deklaration (`est_mapping.py::deklariere`, `:635-950`).
//!
//! Fail-closed (Auflage 3/C): nur BESTAETIGTE Werte werden deklariert; jedes vorlaeufige Feld
//! landet in `unvollstaendig`, und [`Deklaration::eingaben_konsistent`] ist per Konstruktion genau
//! dann wahr, wenn diese Liste leer ist — das Feld ist privat, kein Aufrufer kann es setzen.
//! Auflage A: die dokumentierte Aggregation steht in `dokumentiert`, nie in `deklaration`.
//! Auflage C: was bewusst nicht deklariert wird, steht mit Grund in `nicht_deklariert`.

use std::collections::{BTreeMap, HashMap};

use bindung::Bindung;
use domain::{Cent, Feldtyp, Zustand};
use serde::ser::SerializeMap;
use serde::Serialize;
use serde_json::Value;
use store::SnapshotFeld;

use crate::geordnet::Geordnet;
use crate::instanz::parse_instanz;
use crate::kz_format::{cent_nach_kz, schreibe_kz, KzBetrag};
use crate::py::{self, PyFehler};
use crate::tabellen::{
    suche, PflichtBedingung, Verzweigung, DOKUMENTIERT_AGGREGAT, KAP_FELDER_A, KAP_FELDER_B,
    KAP_NULL_GRUND, KONSTANTE_KZ, MULTIPLIKATION, NEGATION, P23_ART_FELD, P23_BETRAGSFELDER,
    P23_GEWINN_KZ, PARTNER_INSTANZ, PARTNER_VERZWEIGUNG, PFLICHTFELDER, VERZWEIGUNG,
    WERTEKODIERUNG,
};

/// Die materialisierte Felder-Ebene eines Snapshots (`feld_id -> {wert, zustand, herkunft}`).
pub type Felder = BTreeMap<String, SnapshotFeld>;

/// Bindungstabelle `feld_id -> Bindung` (s. [`store::baue_nachschlag`]).
pub type BindungIndex<'a> = HashMap<String, &'a Bindung>;

/// Ein Feld mit Grund (nicht deklariert, unvollstaendig oder Pflichtluecke).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Eintrag {
    pub feld_id: String,
    pub grund: String,
    /// Handlungsanweisung fuer den Nutzer (nur Klasse i, Wertekodierung ohne Code).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hinweis: Option<String>,
}

impl Eintrag {
    fn neu(feld_id: impl Into<String>, grund: impl Into<String>) -> Self {
        Self {
            feld_id: feld_id.into(),
            grund: grund.into(),
            hinweis: None,
        }
    }
}

/// Dokumentierte, nicht deklarierte Summe (Auflage A): verlustbehaftet, nur die Summe ist
/// rekonstruierbar; die Detail-Felder bleiben im Store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Aggregat {
    pub summe: i64,
    pub quell_felder: Vec<String>,
}

/// Eine Anlage-Kind-Instanz (Klasse e, nur die Anzahl).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KindAnlage {
    pub index: i64,
}

/// Eine Instanz einer wiederholbaren Anlage (Kz-Reuse der Basis).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AnlageInstanz {
    pub index: u64,
    pub felder: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub dokumentiert: BTreeMap<String, Aggregat>,
}

/// Ergebnis von [`deklariere`].
#[derive(Debug, Clone, PartialEq)]
pub struct Deklaration {
    pub basis_snapshot: Option<String>,
    /// Kz → Wert, Person A und Basis-Instanzen.
    pub deklaration: BTreeMap<String, Value>,
    pub kind_anlagen: Vec<KindAnlage>,
    /// Anlage-Instanz B (Zusammenveranlagung): dieselben Kz wie Person A.
    pub person_b: BTreeMap<String, Value>,
    /// Gruppe → Instanzen, in der Reihenfolge des ersten Auftretens der Gruppe.
    pub anlage_instanzen: Vec<(String, Vec<AnlageInstanz>)>,
    pub dokumentiert: BTreeMap<String, Aggregat>,
    pub nicht_deklariert: Vec<Eintrag>,
    unvollstaendig: Vec<Eintrag>,
    pflichtfelder_luecken: Vec<Eintrag>,
}

impl Deklaration {
    /// Vorlaeufige oder widerspruechliche Eingaben (fail-closed).
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), None).unwrap();
    /// assert!(d.unvollstaendig().is_empty()); // nichts vorlaeufig, nichts widerspruechlich
    /// ```
    #[must_use]
    pub fn unvollstaendig(&self) -> &[Eintrag] {
        &self.unvollstaendig
    }

    /// Wahr genau dann, wenn nichts vorlaeufig oder widerspruechlich ist.
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), None).unwrap();
    /// assert!(d.eingaben_konsistent());
    /// assert_eq!(d.eingaben_konsistent(), d.unvollstaendig().is_empty());
    /// ```
    #[must_use]
    pub fn eingaben_konsistent(&self) -> bool {
        self.unvollstaendig.is_empty()
    }

    /// Fehlende Pflichtfelder (eigene Aussage, bewusst NICHT in `unvollstaendig`).
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), None).unwrap();
    /// // Pflichtluecken sind eine eigene Aussage und machen die Eingaben nicht inkonsistent.
    /// assert!(d.eingaben_konsistent());
    /// assert!(!d.pflichtfelder_luecken().is_empty()); // ohne jede Angabe fehlen Pflichtfelder
    /// ```
    #[must_use]
    pub fn pflichtfelder_luecken(&self) -> &[Eintrag] {
        &self.pflichtfelder_luecken
    }

    /// Instanzen einer Gruppe.
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use elster::{deklariere, Felder};
    /// let d = deklariere(&Felder::new(), &HashMap::new(), None).unwrap();
    /// assert!(d.instanzen_der_gruppe("kind").is_empty());
    /// ```
    #[must_use]
    pub fn instanzen_der_gruppe(&self, gruppe: &str) -> &[AnlageInstanz] {
        self.anlage_instanzen
            .iter()
            .find(|(g, _)| g == gruppe)
            .map_or(&[], |(_, v)| v.as_slice())
    }
}

impl Serialize for Deklaration {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        struct Gruppen<'a>(&'a [(String, Vec<AnlageInstanz>)]);
        impl Serialize for Gruppen<'_> {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                let mut m = s.serialize_map(Some(self.0.len()))?;
                for (g, v) in self.0 {
                    m.serialize_entry(g, v)?;
                }
                m.end()
            }
        }
        let mut m = s.serialize_map(Some(12))?;
        m.serialize_entry("basis_snapshot", &self.basis_snapshot)?;
        m.serialize_entry("deklaration", &self.deklaration)?;
        m.serialize_entry("kind_anlagen", &self.kind_anlagen)?;
        m.serialize_entry("person_b", &self.person_b)?;
        m.serialize_entry("anlage_instanzen", &Gruppen(&self.anlage_instanzen))?;
        m.serialize_entry("dokumentiert", &self.dokumentiert)?;
        m.serialize_entry("nicht_deklariert", &self.nicht_deklariert)?;
        m.serialize_entry("unvollstaendig", &self.unvollstaendig)?;
        m.serialize_entry("vollstaendig", &self.eingaben_konsistent())?;
        m.serialize_entry("eingaben_konsistent", &self.eingaben_konsistent())?;
        m.serialize_entry("pflichtfelder_luecken", &self.pflichtfelder_luecken)?;
        m.serialize_entry(
            "pflichtfelder_vollstaendig",
            &self.pflichtfelder_luecken.is_empty(),
        )?;
        m.end()
    }
}

/// Warum [`deklariere`] kein Ergebnis liefert.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeklarationsFehler {
    /// Nachauflage D: das Snapshot-OBJEKT statt der Felder-Ebene uebergeben.
    ///
    /// PARITÄT: der Typ [`Felder`] schliesst die Verwechslung aus; Python prueft nur, ob ein
    /// Schluessel `felder`/`snapshot_id` heisst — dieselbe Pruefung steht hier, damit ein Feld
    /// dieses Namens in beiden Welten gleich abgewiesen wird.
    #[error(
        "deklariere() erwartet die felder-Ebene (feld_id -> {{wert, zustand, ...}}), NICHT das \
         Snapshot-Objekt — übergib materialisiere()[0] bzw. snapshot['felder']."
    )]
    SnapshotObjekt,
    /// Nachauflage D: nicht-leere Eingabe, aber kein Feld in der Bindungstabelle.
    #[error(
        "kein Eingabe-Feld in der Bindungstabelle gefunden — vermutlich falsche Eingabe-Ebene/-Struktur; \
         deklariere() liefert kein stilles Leer-Ergebnis."
    )]
    KeinFeldGebunden,
    /// Ein Store-Wert passt nicht zu der Operation, die das Original auf ihm ausfuehrt.
    #[error("Feld {feld_id}: {fehler}")]
    Wert { feld_id: String, fehler: PyFehler },
}

impl DeklarationsFehler {
    /// Die Python-Ausnahmeklasse, die `est_mapping.py` an derselben Stelle wirft.
    ///
    /// ```
    /// assert_eq!(elster::DeklarationsFehler::KeinFeldGebunden.python_klasse(), "ValueError");
    /// ```
    #[must_use]
    pub fn python_klasse(&self) -> &'static str {
        match self {
            Self::SnapshotObjekt | Self::KeinFeldGebunden => "ValueError",
            Self::Wert { fehler, .. } => fehler.klasse,
        }
    }
}

#[derive(Default)]
struct InstBau {
    felder: BTreeMap<String, Value>,
    dokumentiert: BTreeMap<String, Aggregat>,
    rohdaten: BTreeMap<&'static str, i64>,
}

struct Bau<'a> {
    snapshot: &'a Felder,
    bindung: &'a BindungIndex<'a>,
    deklaration: BTreeMap<String, Value>,
    kind_anlagen: Vec<KindAnlage>,
    person_b: BTreeMap<String, Value>,
    /// Einfuege-Reihenfolge von Gruppen UND Instanz-Indizes ist beobachtbar (§ 23-Nachlauf).
    instanzen: Geordnet<String, Geordnet<u64, InstBau>>,
    nicht_deklariert: Vec<Eintrag>,
    unvollstaendig: Vec<Eintrag>,
    agg_akku: Vec<(&'static str, Vec<(String, i64)>)>,
}

type Ergebnis<T> = Result<T, DeklarationsFehler>;

fn wert_fehler(feld_id: &str) -> impl Fn(PyFehler) -> DeklarationsFehler + '_ {
    move |fehler| DeklarationsFehler::Wert {
        feld_id: feld_id.to_owned(),
        fehler,
    }
}

/// Nicht-leerer `elster_kz` (Python: `b.get("elster_kz")` ist truthy).
fn kz_von(b: &Bindung) -> Option<&str> {
    b.elster_kz.as_deref().filter(|k| !k.is_empty())
}

fn gruppe_von(b: &Bindung) -> Option<&str> {
    b.instanz_gruppe.as_deref().filter(|g| !g.is_empty())
}

fn zustand_text(z: Zustand) -> &'static str {
    match z {
        Zustand::Vorlaeufig => "vorlaeufig",
        Zustand::Bestaetigt => "bestaetigt",
    }
}

/// Python `dict.get(wert)` auf einer Tabelle mit Text-Schluesseln: Listen/Objekte sind nicht
/// hashbar (`TypeError`), jeder andere Nicht-Text trifft keinen Schluessel.
fn nachschlagen<'t>(
    tabelle: &'t [(&'t str, &'t str)],
    wert: &Value,
) -> Result<Option<&'t str>, PyFehler> {
    match wert {
        Value::String(s) => Ok(suche(tabelle, s)),
        Value::Array(_) | Value::Object(_) => Err(PyFehler::typ("unhashable type")),
        _ => Ok(None),
    }
}

/// `_cent_nach_kz(int(wert), ziel) if typ == "cent" else int(wert)` (Klasse a).
fn aggregat_beitrag(wert: &Value, ziel: &str, typ: Feldtyp) -> Result<i64, PyFehler> {
    let n = py::int(wert)?;
    if typ != Feldtyp::Cent {
        return Ok(n);
    }
    match cent_nach_kz(Cent::new(n), ziel) {
        KzBetrag::Euro(e) => Ok(e.get()),
        KzBetrag::Komma(_) => Err(PyFehler::typ(
            "unsupported operand type(s) for +: 'int' and 'str'",
        )),
    }
}

impl Bau<'_> {
    fn instanz(&mut self, gruppe: &str, idx: u64) -> &mut InstBau {
        self.instanzen
            .eintrag(gruppe.to_owned(), Geordnet::default)
            .eintrag(idx, InstBau::default)
    }

    fn nicht(&mut self, feld_id: &str, grund: impl Into<String>) {
        self.nicht_deklariert.push(Eintrag::neu(feld_id, grund));
    }

    fn offen(&mut self, feld_id: &str, grund: impl Into<String>) {
        self.unvollstaendig.push(Eintrag::neu(feld_id, grund));
    }

    /// `_deklariere_instanz` (`est_mapping.py:590-632`).
    fn instanz_feld(
        &mut self,
        basis: &str,
        idx: u64,
        feld_id: &str,
        sfeld: &SnapshotFeld,
        b: &Bindung,
    ) -> Ergebnis<()> {
        let fehler = wert_fehler(feld_id);
        if sfeld.zustand != Zustand::Bestaetigt {
            let grund = format!(
                "Instanz-Wert {} — Pflicht-Bestätigung (Zwei-Signal) fehlt",
                zustand_text(sfeld.zustand)
            );
            self.offen(feld_id, grund);
            return Ok(());
        }
        let gruppe = gruppe_von(b).unwrap_or_default();
        let wert = &sfeld.wert;
        self.instanz(gruppe, idx);
        if let Some((ziel, _)) = DOKUMENTIERT_AGGREGAT
            .iter()
            .find(|(_, q)| q.contains(&basis))
        {
            let beitrag = aggregat_beitrag(wert, ziel, b.typ).map_err(&fehler)?;
            let agg = self
                .instanz(gruppe, idx)
                .dokumentiert
                .entry((*ziel).to_owned())
                .or_insert_with(|| Aggregat {
                    summe: 0,
                    quell_felder: Vec::new(),
                });
            agg.summe = agg
                .summe
                .checked_add(beitrag)
                .ok_or_else(|| fehler(PyFehler::ueberlauf("Summe jenseits i64")))?;
            agg.quell_felder.push(feld_id.to_owned());
        } else if let Some(cfg) = VERZWEIGUNG.iter().find(|v| v.feld == basis) {
            let art_feld_inst = format!("{}__{idx}", cfg.art_feld);
            let art = self.snapshot.get(&art_feld_inst);
            match art.filter(|a| a.zustand == Zustand::Bestaetigt) {
                None => self.offen(
                    feld_id,
                    format!("Instanz-Art ({art_feld_inst}) unbestätigt — Kz-Zweig offen"),
                ),
                Some(art) => {
                    if let Some(kz) = nachschlagen(cfg.kz, &art.wert).map_err(&fehler)? {
                        let felder = &mut self.instanz(gruppe, idx).felder;
                        schreibe_kz(felder, kz, wert, Some(b.typ)).map_err(&fehler)?;
                    } else {
                        let grund =
                            format!("Instanz-Art '{}' ohne Kz-Zweig", py::str_von(&art.wert));
                        self.nicht(feld_id, grund);
                    }
                }
            }
        } else if let Some(p23) = P23_BETRAGSFELDER.iter().find(|f| **f == basis) {
            let n = py::int(wert).map_err(&fehler)?;
            self.instanz(gruppe, idx).rohdaten.insert(p23, n);
        } else if let Some(kz) = kz_von(b) {
            let felder = &mut self.instanz(gruppe, idx).felder;
            schreibe_kz(felder, kz, wert, Some(b.typ)).map_err(&fehler)?;
        } else {
            self.nicht(
                feld_id,
                format!("Instanz-Basis '{basis}' ohne elster_kz/Aggregat-Ziel"),
            );
        }
        Ok(())
    }

    /// Klasse f (Person A) bzw. g×f (Person B): Kz haengt am bestaetigten Art-Feld.
    fn verzweigung(
        &mut self,
        feld_id: &str,
        wert: &Value,
        b: &Bindung,
        cfg: &Verzweigung,
        partner: bool,
    ) -> Ergebnis<()> {
        let fehler = wert_fehler(feld_id);
        let art = self
            .snapshot
            .get(cfg.art_feld)
            .filter(|a| a.zustand == Zustand::Bestaetigt);
        let Some(art) = art else {
            let grund = if partner {
                format!(
                    "Partner-Renten-Art ({}) unbestätigt — Kz-Zweig offen",
                    cfg.art_feld
                )
            } else {
                format!("Art ({}) unbestätigt — Kz-Zweig offen", cfg.art_feld)
            };
            self.offen(feld_id, grund);
            return Ok(());
        };
        let art_text = py::str_von(&art.wert);
        match nachschlagen(cfg.kz, &art.wert).map_err(&fehler)? {
            Some(kz) => {
                let ziel = if partner {
                    &mut self.person_b
                } else {
                    &mut self.deklaration
                };
                schreibe_kz(ziel, kz, wert, Some(b.typ)).map_err(&fehler)?;
            }
            None if partner => {
                self.nicht(
                    feld_id,
                    format!("Partner-Renten-Art '{art_text}' ohne Kz-Zweig"),
                );
                self.offen(
                    feld_id,
                    format!(
                        "Eure Angabe zu '{art_text}' ({}) ist vollständig, nichts von euch fehlt oder \
                         widerspricht sich. Diese Einkunftsart des Partners ist bei uns noch nicht abgebbar \
                         — das liegt nicht an eurer Eingabe, ihr müsst hier nichts nachtragen.",
                        cfg.art_feld
                    ),
                );
            }
            None => {
                self.nicht(
                    feld_id,
                    format!("Art '{art_text}' ({}) ohne Kz-Zweig", cfg.art_feld),
                );
                self.offen(
                    feld_id,
                    format!(
                        "Für '{art_text}' ({}) ist diese Einkunftsart bei uns noch nicht abgebbar — keine \
                         Eingabe von dir fehlt oder widerspricht sich.",
                        cfg.art_feld
                    ),
                );
            }
        }
        Ok(())
    }

    /// Klasse j — IBAN: Format, Pruefziffer, Laender-Weiche.
    fn iban(&mut self, feld_id: &str, wert: &Value) {
        let norm: String = py::str_von(wert)
            .chars()
            .filter(|c| !py::ist_leerraum(*c))
            .collect::<String>()
            .to_uppercase();
        if !iban_muster(&norm) {
            self.offen(
                feld_id,
                "IBAN entspricht nicht dem amtlichen Muster (2 Buchstaben Laendercode + 2 Pruefziffern + 1-30 \
                 alphanumerische Zeichen, String_MinL5_MaxL34_Musterm188614856_CType, E10-2025.xsd) -- Wert \
                 nicht geloggt.",
            );
        } else if !iban_pruefziffer_gueltig(&norm) {
            self.offen(
                feld_id,
                "IBAN-Pruefziffer (ISO 13616, Modulo 97) ungueltig -- Wert nicht geloggt.",
            );
        } else {
            let kz = if norm.starts_with("DE") {
                "E0102102"
            } else {
                "E0102603"
            };
            self.deklaration.insert(kz.to_owned(), Value::String(norm));
        }
    }

    /// Ein Feld der Hauptschleife (`est_mapping.py:671-782`).
    fn feld(&mut self, feld_id: &str, sfeld: &SnapshotFeld, b: &Bindung) -> Ergebnis<()> {
        let fehler = wert_fehler(feld_id);
        if sfeld.zustand != Zustand::Bestaetigt {
            let grund = format!(
                "Wert {} — Pflicht-Bestätigung (Zwei-Signal) fehlt",
                zustand_text(sfeld.zustand)
            );
            self.offen(feld_id, grund);
            return Ok(());
        }
        let wert = &sfeld.wert;
        if let Some(kz) = suche(NEGATION, feld_id) {
            self.deklaration
                .insert(kz.to_owned(), Value::Bool(!py::truthy(wert)));
        } else if MULTIPLIKATION.contains(&feld_id) {
            let n = py::int(wert).map_err(&fehler)?;
            self.kind_anlagen = (1..=n.max(0)).map(|index| KindAnlage { index }).collect();
            let grund = b
                .elster_kz_grund
                .as_deref()
                .filter(|g| !g.is_empty())
                .unwrap_or("Multiplikation -> kind_anlagen (Kinderzahl implizit)");
            self.nicht(feld_id, grund);
        } else if let Some((ziel, _)) = DOKUMENTIERT_AGGREGAT
            .iter()
            .find(|(_, q)| q.contains(&feld_id))
        {
            let beitrag = aggregat_beitrag(wert, ziel, b.typ).map_err(&fehler)?;
            if let Some((_, akku)) = self.agg_akku.iter_mut().find(|(z, _)| z == ziel) {
                akku.push((feld_id.to_owned(), beitrag));
            }
        } else if let Some(cfg) = VERZWEIGUNG.iter().find(|v| v.feld == feld_id) {
            self.verzweigung(feld_id, wert, b, cfg, false)?;
        } else if let Some(cfg) = PARTNER_VERZWEIGUNG.iter().find(|v| v.feld == feld_id) {
            self.verzweigung(feld_id, wert, b, cfg, true)?;
        } else if let Some(kz) = suche(PARTNER_INSTANZ, feld_id) {
            schreibe_kz(&mut self.person_b, kz, wert, Some(b.typ)).map_err(&fehler)?;
        } else if let Some(cfg) = WERTEKODIERUNG.iter().find(|w| w.feld == feld_id) {
            match nachschlagen(cfg.code, wert).map_err(&fehler)? {
                Some(code) => {
                    self.deklaration
                        .insert(cfg.kz.to_owned(), Value::String(code.to_owned()));
                }
                None => self.nicht_deklariert.push(Eintrag {
                    feld_id: feld_id.to_owned(),
                    grund: format!(
                        "Wert '{}' ohne XSD-Code-Zuordnung ({})",
                        py::str_von(wert),
                        cfg.kz
                    ),
                    hinweis: Some(cfg.hinweis_unbekannt.to_owned()),
                }),
            }
        } else if feld_id == "stammdaten_iban" {
            self.iban(feld_id, wert);
        } else if let Some(kz) = kz_von(b) {
            schreibe_kz(&mut self.deklaration, kz, wert, Some(b.typ)).map_err(&fehler)?;
        } else if let Some(p23) = P23_BETRAGSFELDER.iter().find(|f| **f == feld_id) {
            let n = py::int(wert).map_err(&fehler)?;
            self.instanz("p23_veraeusserung", 1).rohdaten.insert(p23, n);
        } else {
            let grund = b
                .elster_kz_grund
                .clone()
                .unwrap_or_else(|| "kein elster_kz".to_owned());
            self.nicht(feld_id, grund);
        }
        Ok(())
    }
}

/// `^[A-Z]{2}[0-9]{2}[0-9A-Z]{1,30}$` (`String_MinL5_MaxL34_Musterm188614856_CType`).
fn iban_muster(s: &str) -> bool {
    let b = s.as_bytes();
    (5..=34).contains(&b.len())
        && b.iter().take(2).all(u8::is_ascii_uppercase)
        && b.iter().skip(2).take(2).all(u8::is_ascii_digit)
        && b.iter()
            .skip(4)
            .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase())
}

/// ISO 13616 Pruefziffer (Modulo 97): erste vier Zeichen ans Ende, Buchstaben A=10..Z=35, Rest 1.
fn iban_pruefziffer_gueltig(iban: &str) -> bool {
    let (kopf, rumpf) = iban.split_at(4.min(iban.len()));
    let mut rest: u32 = 0;
    for c in rumpf.chars().chain(kopf.chars()) {
        let Some(d) = c.to_digit(36) else {
            return false;
        };
        for ziffer in d.to_string().chars().filter_map(|z| z.to_digit(10)) {
            rest = (rest * 10 + ziffer) % 97;
        }
    }
    rest == 1
}

/// `_pflichtfelder_luecken` (`est_mapping.py:496-513`): sieht Feld-ABWESENHEIT, die die
/// Hauptschleife per Konstruktion nicht sehen kann.
fn pflichtfelder_luecken(snapshot: &Felder) -> Vec<Eintrag> {
    let mut luecken = Vec::new();
    for (bedingung, version, felder) in PFLICHTFELDER {
        if matches!(bedingung, PflichtBedingung::AlleOderKeins)
            && !felder.iter().any(|f| snapshot.contains_key(*f))
        {
            continue;
        }
        for f in felder.iter().filter(|f| !snapshot.contains_key(**f)) {
            luecken.push(Eintrag::neu(
                *f,
                format!(
                    "Pflichtfeld fehlt (ERiC {version}, Bedingung '{}') -- kein Wert im Snapshot",
                    bedingung.als_str()
                ),
            ));
        }
    }
    luecken
}

/// `_kap_alle_null`: jedes Feld fehlt oder ist bestaetigt 0.
fn kap_alle_null(snapshot: &Felder, felder: &[&str]) -> Ergebnis<bool> {
    for f in felder {
        let Some(sfeld) = snapshot.get(*f) else {
            continue;
        };
        if sfeld.zustand != Zustand::Bestaetigt
            || py::int(&sfeld.wert).map_err(wert_fehler(f))? != 0
        {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Snapshot (Felder-Ebene) → Deklaration (`est_mapping.py::deklariere`).
///
/// Nur bestaetigte Felder werden deklariert; der Rest steht mit Grund in
/// [`Deklaration::unvollstaendig`] bzw. `nicht_deklariert`.
///
/// # Errors
/// [`DeklarationsFehler`], wenn die Eingabe-Ebene falsch ist oder ein Store-Wert nicht zu der
/// Operation passt, die das Original auf ihm ausfuehrt (Python-Ausnahme an derselben Stelle).
///
/// ```
/// use std::collections::HashMap;
/// use elster::{deklariere, Felder};
/// let leer = Felder::new();
/// let d = deklariere(&leer, &HashMap::new(), None).unwrap();
/// assert!(d.eingaben_konsistent());
/// assert_eq!(d.deklaration.get("E0100001"), Some(&serde_json::json!(true)));
/// ```
pub fn deklariere(
    snapshot: &Felder,
    bindung: &BindungIndex<'_>,
    snapshot_id: Option<&str>,
) -> Ergebnis<Deklaration> {
    if snapshot.contains_key("felder") || snapshot.contains_key("snapshot_id") {
        return Err(DeklarationsFehler::SnapshotObjekt);
    }
    let mut bau = Bau {
        snapshot,
        bindung,
        deklaration: KONSTANTE_KZ
            .iter()
            .map(|k| ((*k).to_owned(), Value::Bool(true)))
            .collect(),
        kind_anlagen: Vec::new(),
        person_b: BTreeMap::new(),
        instanzen: Geordnet::default(),
        nicht_deklariert: Vec::new(),
        unvollstaendig: Vec::new(),
        agg_akku: DOKUMENTIERT_AGGREGAT
            .iter()
            .map(|(z, _)| (*z, Vec::new()))
            .collect(),
    };
    let mut getroffen = 0_usize;
    for (feld_id, sfeld) in snapshot {
        if let Some((basis, idx)) = parse_instanz(feld_id) {
            if let Some(b) = bindung.get(basis).filter(|b| gruppe_von(b).is_some()) {
                getroffen += 1;
                bau.instanz_feld(basis, idx, feld_id, sfeld, b)?;
                continue;
            }
        }
        let Some(b) = bindung.get(feld_id.as_str()) else {
            bau.nicht(feld_id, "nicht in der Bindungstabelle");
            bau.offen(feld_id, "Feld nicht in der Bindungstabelle");
            continue;
        };
        getroffen += 1;
        bau.feld(feld_id, sfeld, b)?;
    }
    if !snapshot.is_empty() && getroffen == 0 {
        return Err(DeklarationsFehler::KeinFeldGebunden);
    }
    let pflichtfelder_luecken = pflichtfelder_luecken(snapshot);
    bau.bankverbindung();
    bau.kap_nulldeklaration()?;
    bau.antrag_person_b();
    let dokumentiert = bau.dokumentiert();
    bau.p23_gewinn()?;
    let anlage_instanzen = bau.instanzen_ausgabe();
    Ok(Deklaration {
        basis_snapshot: snapshot_id.map(str::to_owned),
        deklaration: bau.deklaration,
        kind_anlagen: bau.kind_anlagen,
        person_b: bau.person_b,
        anlage_instanzen,
        dokumentiert,
        nicht_deklariert: bau.nicht_deklariert,
        unvollstaendig: bau.unvollstaendig,
        pflichtfelder_luecken,
    })
}

impl Bau<'_> {
    /// Bankverbindung: Exklusivitaet IBAN ↔ „keine Bankverbindung" (gemessen rc=610001002),
    /// Kontoinhaber E0101601 automatisch bei bestaetigter IBAN.
    fn bankverbindung(&mut self) {
        let iban =
            self.deklaration.contains_key("E0102102") || self.deklaration.contains_key("E0102603");
        let keine = self.deklaration.get("E0102002") == Some(&Value::Bool(true));
        if iban && keine {
            self.offen(
                "stammdaten_iban",
                "IBAN UND 'keine Bankverbindung' gleichzeitig bestaetigt -- checkESt lehnt den Widerspruch ab \
                 (rc=610001002). Genau eines von beidem darf gelten.",
            );
        } else if iban {
            self.deklaration
                .insert("E0101601".to_owned(), Value::Bool(true));
        }
    }

    /// Option A: KAP-Nullen beider Personen atomar unterdruecken.
    fn kap_nulldeklaration(&mut self) -> Ergebnis<()> {
        if !(kap_alle_null(self.snapshot, KAP_FELDER_A)?
            && kap_alle_null(self.snapshot, KAP_FELDER_B)?)
        {
            return Ok(());
        }
        for f in KAP_FELDER_A {
            let Some(kz) = self.bindung.get(*f).and_then(|b| kz_von(b)) else {
                continue;
            };
            // Python: `deklaration.pop(kz, None) is not None` — ein gespeichertes `None` zaehlt nicht.
            if self.deklaration.remove(kz).is_some_and(|v| !v.is_null()) {
                self.nicht(f, KAP_NULL_GRUND);
            }
        }
        for f in KAP_FELDER_B {
            let Some(kz) = suche(PARTNER_INSTANZ, f) else {
                continue;
            };
            if self.person_b.remove(kz).is_some_and(|v| !v.is_null()) {
                self.nicht(f, KAP_NULL_GRUND);
            }
        }
        Ok(())
    }

    /// § 32d Abs. 6 S. 4 EStG: der Guenstigerpruefungs-Antrag gilt fuer beide Ehegatten und muss
    /// im XML in beiden Person-Containern stehen.
    fn antrag_person_b(&mut self) {
        let zusammen = self
            .snapshot
            .get("veranlagung")
            .is_some_and(|v| v.zustand == Zustand::Bestaetigt && v.wert == "zusammen");
        let antrag_kz = self
            .bindung
            .get("kap_antrag_guenstigerpruefung")
            .and_then(|b| kz_von(b));
        if let (true, Some(kz)) = (zusammen, antrag_kz) {
            if let Some(w) = self.deklaration.get(kz).cloned() {
                self.person_b.insert(kz.to_owned(), w);
            }
        }
    }

    fn dokumentiert(&self) -> BTreeMap<String, Aggregat> {
        let mut out = BTreeMap::new();
        for (ziel, akku) in &self.agg_akku {
            if akku.is_empty() {
                continue;
            }
            let mut quell: Vec<String> = akku.iter().map(|(f, _)| f.clone()).collect();
            quell.sort();
            let summe = akku.iter().map(|(_, w)| *w).fold(0_i64, i64::wrapping_add);
            out.insert(
                (*ziel).to_owned(),
                Aggregat {
                    summe,
                    quell_felder: quell,
                },
            );
        }
        out
    }

    /// § 23 Gewinn je Instanz = Preis − AK/HK − WK, dann an die Kz des Veraeusserungstyps.
    fn p23_gewinn(&mut self) -> Ergebnis<()> {
        const GRUPPE: &str = "p23_veraeusserung";
        let Some(gruppe) = self.instanzen.get(&GRUPPE.to_owned()) else {
            return Ok(());
        };
        let gewinne: Vec<(u64, i64)> = gruppe
            .schluessel()
            .iter()
            .filter_map(|idx| gruppe.get(idx).map(|inst| (*idx, inst)))
            .map(|(idx, inst)| {
                let roh = |k: &str| inst.rohdaten.get(k).copied().unwrap_or(0);
                let gewinn = roh("p23_veraeusserungspreis")
                    .wrapping_sub(roh("p23_anschaffung_herstellungskosten"))
                    .wrapping_sub(roh("p23_werbungskosten"));
                (idx, gewinn)
            })
            .collect();
        for (idx, gewinn) in gewinne {
            if gewinn == 0 {
                continue;
            }
            let art_feld = if idx == 1 {
                P23_ART_FELD.to_owned()
            } else {
                format!("{P23_ART_FELD}__{idx}")
            };
            let feld_id = format!("p23_veraeusserung__{idx}");
            let Some(art) = self
                .snapshot
                .get(&art_feld)
                .filter(|a| a.zustand == Zustand::Bestaetigt)
            else {
                self.offen(&feld_id, format!("{art_feld} unbestätigt — Kz-Zweig offen"));
                continue;
            };
            if let Some(kz) =
                nachschlagen(P23_GEWINN_KZ, &art.wert).map_err(wert_fehler(&feld_id))?
            {
                let v = cent_nach_kz(Cent::new(gewinn), kz).als_json();
                self.instanz(GRUPPE, idx).felder.insert(kz.to_owned(), v);
            } else {
                let grund = format!("p23-Typ '{}' ohne Kz-Zweig", py::str_von(&art.wert));
                self.nicht(&feld_id, grund);
            }
        }
        Ok(())
    }

    fn instanzen_ausgabe(&mut self) -> Vec<(String, Vec<AnlageInstanz>)> {
        let mut out = Vec::new();
        for (gruppe, instanzen) in std::mem::take(&mut self.instanzen).in_reihenfolge() {
            let mut sortiert = instanzen.in_reihenfolge();
            sortiert.sort_by_key(|(i, _)| *i);
            let eintraege: Vec<AnlageInstanz> = sortiert
                .into_iter()
                .filter(|(_, inst)| !inst.felder.is_empty() || !inst.dokumentiert.is_empty())
                .map(|(index, inst)| AnlageInstanz {
                    index,
                    felder: inst.felder,
                    dokumentiert: inst
                        .dokumentiert
                        .into_iter()
                        .map(|(z, mut a)| {
                            a.quell_felder.sort();
                            (z, a)
                        })
                        .collect(),
                })
                .collect();
            if !eintraege.is_empty() {
                out.push((gruppe, eintraege));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use domain::{Achsenwert, Herkunft, PruefTiefe};
    use serde_json::json;

    use super::{
        deklariere, gruppe_von, iban_muster, iban_pruefziffer_gueltig, Felder, Feldtyp,
        SnapshotFeld, Value, Zustand, PARTNER_VERZWEIGUNG, VERZWEIGUNG,
    };

    #[test]
    fn iban_pruefziffer() {
        assert!(iban_muster("DE89370400440532013000"));
        assert!(iban_pruefziffer_gueltig("DE89370400440532013000"));
        assert!(!iban_pruefziffer_gueltig("DE88370400440532013000"));
        assert!(!iban_muster("DE8937"[..3].to_string().as_str()));
    }

    /// P9 (Vault: `decisions/elster-null-in-kz-ohne-null-weglassen`): auch die Art-Verzweigung
    /// (Klasse f, g×f) schreibt keine 0 in eine Kz, deren XSD-Typ sie verbietet, auch nicht aus
    /// 50 Cent abgerundet. Zwilling von `null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet`
    /// (`tests/eigenschaften.rs`), der die `pub(crate)`-Tabellen nicht sieht. Die Kz-Menge kommt
    /// live aus dem XSD 2025, nicht aus `NULL_UNZULAESSIG_KZ`.
    #[test]
    fn art_verzweigung_schreibt_keine_verbotene_null() {
        let Some(xsd) = crate::finde_schema(2025, "E10-{jahr}.xsd") else {
            println!("E10-2025.xsd fehlt — source_unavailable");
            return;
        };
        let meta = crate::kz_meta(&xsd, "E10").unwrap();
        let verbietet_null = |kz: &str| {
            meta.get(kz).is_some_and(|m| {
                m.type_name.starts_with("GanzzahlPos")
                    || (!m.enums.is_empty() && !m.enums.iter().any(|e| e == "0"))
                    || (!m.patterns.is_empty()
                        && !m.patterns.iter().any(|p| {
                            regex::Regex::new(&format!("^(?:{p})$"))
                                .unwrap()
                                .is_match("0")
                        }))
            })
        };
        let pfad = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../produkt/bindung");
        let bindungen: Vec<bindung::Bindung> = bindung::lade_registry(&pfad)
            .unwrap()
            .dateien
            .into_iter()
            .flat_map(|(_, d)| d.bindungen)
            .collect();
        let index = store::baue_nachschlag(&bindungen);
        let a = |s: &str| Achsenwert::new(s.to_owned()).unwrap();
        let feld = |wert: Value| SnapshotFeld {
            wert,
            zustand: Zustand::Bestaetigt,
            herkunft: Herkunft {
                herkunft: a("laie"),
                pruef_tiefe: PruefTiefe::Ungeprueft,
                haftung: a("nutzer"),
            }
            .into(),
        };
        let mut durch = Vec::new();
        for cfg in VERZWEIGUNG.iter().chain(PARTNER_VERZWEIGUNG) {
            let b = index[cfg.feld];
            let werte: &[i64] = if b.typ == Feldtyp::Cent {
                &[0, 50]
            } else {
                &[0]
            };
            let suffixe: &[&str] = if gruppe_von(b).is_some() {
                &["", "__2"]
            } else {
                &[""]
            };
            for i in suffixe {
                for (art, _) in cfg.kz {
                    for w in werte {
                        let snapshot = Felder::from([
                            (format!("{}{i}", cfg.feld), feld(json!(w))),
                            (format!("{}{i}", cfg.art_feld), feld(json!(art))),
                        ]);
                        let d = deklariere(&snapshot, &index, None).unwrap();
                        let instanzen = d.anlage_instanzen.iter().flat_map(|(_, ii)| ii);
                        durch.extend(
                            [&d.deklaration, &d.person_b]
                                .into_iter()
                                .chain(instanzen.map(|inst| &inst.felder))
                                .flatten()
                                .filter(|(kz, v)| **v == json!(0) && verbietet_null(kz))
                                .map(|(kz, _)| format!("{kz} {}{i}={w} Art {art}", cfg.feld)),
                        );
                    }
                }
            }
        }
        assert!(
            durch.is_empty(),
            "0 in {} Faellen durchgelassen, obwohl der XSD-Typ sie verbietet: {durch:?}",
            durch.len()
        );
    }
}
