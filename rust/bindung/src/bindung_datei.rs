//! `bindung_*.yaml`: eine Bindungstabelle (`produkt/bindung/schema.json`, "Bindungstabelle
//! (UI-Kern, Task #11)"). Struktur und Feldnamen folgen dem Schema 1:1; `deny_unknown_fields`
//! ueberall, weil das Schema selbst `additionalProperties: false` ist.
//!
//! `Quelle` (`signatur_slot` XOR `geltungsbedingung`) hat eine handgeschriebene `Deserialize`-Impl,
//! damit der ungueltige Zustand "beides oder keins" im Typ gar nicht erst entsteht. Die
//! restlichen Cross-Feld-Regeln des Schemas (`allOf`: askable->fragetext_laie,
//! elster_kz=null->elster_kz_grund, typ=enum->enum_werte, frage_invertiert->typ=bool+askable)
//! sind als [`Bindung::validieren`] nachgeschaltet -- eine volle State-Machine-`Deserialize`
//! fuer jede dieser vier Regeln haette denselben Effekt bei deutlich mehr Code gehabt.
use std::path::{Path, PathBuf};

use domain::{Feldtyp, Kz};
use serde::Deserialize;
use serde_json::Value;

/// `Bindung`-Parse/Validierungsfehler.
#[derive(Debug, thiserror::Error)]
pub enum BindungFehler {
    #[error("konnte {pfad} nicht lesen: {nachricht}")]
    Io { pfad: PathBuf, nachricht: String },
    #[error("YAML-Fehler in {pfad}: {nachricht}")]
    Yaml { pfad: PathBuf, nachricht: String },
    #[error("{feld_id}: askable=true braucht fragetext_laie (min. 5 Zeichen)")]
    AskableOhneFragetext { feld_id: String },
    #[error("{feld_id}: typ=enum braucht enum_werte")]
    EnumOhneWerte { feld_id: String },
    #[error("{feld_id}: elster_kz=null braucht elster_kz_grund")]
    KzNullOhneGrund { feld_id: String },
    #[error("{feld_id}: elster_kz {kz:?} passt nicht auf ^E[0-9]{{7}}$")]
    KzFormat { feld_id: String, kz: String },
    #[error("{feld_id}: frage_invertiert=true braucht typ=bool und askable=true")]
    InvertiertOhneBoolAskable { feld_id: String },
    #[error("ungueltige feld_id {0:?} (erwartet ^[a-z][a-z0-9_]*$)")]
    UngueltigeFeldId(String),
    #[error("{feld_id}: vz_gueltigkeit darf nicht leer sein")]
    LeereVzGueltigkeit { feld_id: String },
}

/// Prueft `^[a-z][a-z0-9_]*$` ohne Regex-Abhaengigkeit (nur ASCII, wie das Schema selbst).
#[must_use]
pub fn ist_gueltige_feld_id(s: &str) -> bool {
    let mut bytes = s.bytes();
    matches!(bytes.next(), Some(b) if b.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Bindungspunkt einer [`Quelle`]: entweder ein Signatur-Slot einer Catala-Regel oder eine
/// benannte Geltungsbedingung -- nie beides, nie keins (Schema: `quelle.oneOf`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bindungspunkt {
    SignaturSlot(String),
    Geltungsbedingung(String),
}

/// Woran eine Bindung/Luecke andockt: eine Regel-Id plus genau ein [`Bindungspunkt`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quelle {
    pub regel_id: String,
    pub bindungspunkt: Bindungspunkt,
}

impl<'de> Deserialize<'de> for Quelle {
    /// `signatur_slot` UND `geltungsbedingung` gleichzeitig, oder keins von beiden, ist ein
    /// Deserialisierungsfehler -- die ungueltige `Quelle` kann gar nicht erst entstehen.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Roh {
            regel_id: String,
            signatur_slot: Option<String>,
            geltungsbedingung: Option<String>,
        }
        let roh = Roh::deserialize(deserializer)?;
        let bindungspunkt = match (roh.signatur_slot, roh.geltungsbedingung) {
            (Some(s), None) => Bindungspunkt::SignaturSlot(s),
            (None, Some(g)) => Bindungspunkt::Geltungsbedingung(g),
            _ => {
                return Err(serde::de::Error::custom(
                    "quelle braucht genau eines von signatur_slot/geltungsbedingung",
                ))
            }
        };
        Ok(Self {
            regel_id: roh.regel_id,
            bindungspunkt,
        })
    }
}

/// Norm-Fundstelle + woertliches Zitat (`$defs/anker_ref`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnkerRef {
    pub quelle: String,
    pub zitatanker: String,
    pub datei: Option<String>,
}

/// `exakt` = das Feld IST der Slot-Wert; `summand` = mehrere Felder addieren sich auf denselben
/// Slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlotBeitrag {
    Exakt,
    Summand,
}

/// Vorjahres-Uebernahme-Flag. Fail-closed-Default: fehlt das Feld, wird NICHT uebernommen
/// (siehe [`Bindung::vorjahr`]: `Option<Vorjahr>`, kein Default-Wert dieses Enums).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Vorjahr {
    Uebernehmbar,
    Vorschlag,
}

/// Welche Vorschlags-Schreiber ein Feld setzen duerfen (K1, Sicherheit). Leer/absent = DEFAULT
/// human-only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VorschlagsSchreiber {
    Kontoauszug,
    Beleg,
    Maps,
    Llm,
}

/// Einordnung einer fehlenden `elster_kz`: `Endgueltig` = amtlich belegt, dass es keine gibt;
/// `Offen` = Bindungsarbeit steht noch aus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KzStatus {
    Offen,
    Endgueltig,
}

/// Optionale Wertebereichsgrenzen (nur cent/int).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bereich {
    pub min: i64,
    pub max: i64,
    pub grund: Option<String>,
}

/// Dieses Feld beweist die Antwort auf eine Existenzfrage (`$defs` innerhalb `bindung`,
/// Property `beweist`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Beweist {
    pub feld_id: String,
    pub wert: Value,
    pub ab: Option<f64>,
}

/// Dieses Feld entfaellt, wenn `feld` einen anderen Wert als `wert` traegt (oder GENAU
/// `wert_nicht`, wo eine Existenzfrage ein Auswahlfeld ist).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeldBedingung {
    pub feld: String,
    /// Genau eines von `wert`/`wert_nicht` ist gesetzt (Schema: `oneOf`, Rust prueft es nicht
    /// nach) -- ausgeschrieben statt XOR-Typ, weil hier (anders als bei `Quelle`) keine
    /// nachgeschaltete Regel darauf angewiesen ist, dass genau eines gesetzt ist.
    pub wert: Option<Value>,
    pub grund: String,
    pub wert_nicht: Option<Value>,
}

/// Berechnungsart einer [`Ableitung`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AbleitungArt {
    JahrAusDatum,
    AlterAmJahresbeginnErreicht,
    Uebernahme,
    AlterUnterAmJahresende,
}

/// Dieses Feld wird berechnet, sobald `aus` bestaetigt vorliegt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ableitung {
    pub aus: String,
    pub art: AbleitungArt,
    pub schwelle: Option<f64>,
    pub grund: String,
    pub und_feld: Option<String>,
}

/// Eine einzelne Feld-Bindung (`$defs/bindung`). Feldnamen und Optionalitaet 1:1 aus
/// `schema.json`; die vier `allOf`-Regeln pruefen [`Bindung::validieren`].
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bindung {
    pub feld_id: String,
    pub quelle: Quelle,
    pub typ: Feldtyp,
    #[serde(default)]
    pub slot_beitrag: Option<SlotBeitrag>,
    pub einheit: Option<String>,
    pub askable: bool,
    pub gate: Option<bool>,
    #[serde(default)]
    pub frage_invertiert: bool,
    #[serde(default)]
    pub eingangsfrage: bool,
    pub fragetext_laie: Option<String>,
    pub hilfe_kurz: String,
    pub beispielwert: Value,
    pub muster: Option<String>,
    pub standardwert: Option<Value>,
    pub abwesenheitswert: Option<Value>,
    pub herkunft_slots: Option<Vec<String>>,
    pub vorjahr: Option<Vorjahr>,
    #[serde(default)]
    pub vorschlagbar_von: Vec<VorschlagsSchreiber>,
    pub instanz_gruppe: Option<String>,
    pub elster_kz: Option<String>,
    pub elster_kz_grund: Option<String>,
    pub kz_status: Option<KzStatus>,
    pub vz_gueltigkeit: Vec<i64>,
    pub anker_ref: AnkerRef,
    pub enum_werte: Option<Vec<String>>,
    pub bereich: Option<Bereich>,
    pub screening: Option<bool>,
    pub beweist: Option<Beweist>,
    pub feld_bedingung: Option<FeldBedingung>,
    pub ableitung: Option<Ableitung>,
}

impl Bindung {
    /// Die vier `allOf`-Regeln aus `schema.json` plus `feld_id`-Zeichensatz und
    /// nicht-leere `vz_gueltigkeit`, die `serde` allein nicht ausdruecken kann.
    ///
    /// # Errors
    /// [`BindungFehler`], wenn eine der Regeln verletzt ist.
    pub fn validieren(&self) -> Result<(), BindungFehler> {
        if !ist_gueltige_feld_id(&self.feld_id) {
            return Err(BindungFehler::UngueltigeFeldId(self.feld_id.clone()));
        }
        if self.vz_gueltigkeit.is_empty() {
            return Err(BindungFehler::LeereVzGueltigkeit {
                feld_id: self.feld_id.clone(),
            });
        }
        if self.askable
            && self
                .fragetext_laie
                .as_ref()
                .is_none_or(|s| s.chars().count() < 5)
        {
            return Err(BindungFehler::AskableOhneFragetext {
                feld_id: self.feld_id.clone(),
            });
        }
        if matches!(self.typ, Feldtyp::Enum) && self.enum_werte.is_none() {
            return Err(BindungFehler::EnumOhneWerte {
                feld_id: self.feld_id.clone(),
            });
        }
        match &self.elster_kz {
            None if self.elster_kz_grund.is_none() => {
                return Err(BindungFehler::KzNullOhneGrund {
                    feld_id: self.feld_id.clone(),
                })
            }
            Some(kz) if !ist_gueltige_elster_kz(kz) => {
                return Err(BindungFehler::KzFormat {
                    feld_id: self.feld_id.clone(),
                    kz: kz.clone(),
                })
            }
            _ => {}
        }
        if self.frage_invertiert && !(matches!(self.typ, Feldtyp::Bool) && self.askable) {
            return Err(BindungFehler::InvertiertOhneBoolAskable {
                feld_id: self.feld_id.clone(),
            });
        }
        Ok(())
    }
}

/// `^E[0-9]{7}$`; die Regel steht in [`Kz::ist_gueltig`].
fn ist_gueltige_elster_kz(s: &str) -> bool {
    Kz::ist_gueltig(s)
}

/// Slot/Geltungsbedingung einer Scheiben-Regel ohne Bindung, mit Grund (`$defs/luecke`).
/// Dieselbe XOR-Regel wie bei [`Quelle`] (`luecke.oneOf`), deshalb dieselbe Technik: der
/// ungueltige Zustand "beides oder keins" entsteht gar nicht erst.
#[derive(Debug, Clone)]
pub struct Luecke {
    pub regel_id: String,
    pub bindungspunkt: Bindungspunkt,
    pub grund: String,
}

impl<'de> Deserialize<'de> for Luecke {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Roh {
            regel_id: String,
            signatur_slot: Option<String>,
            geltungsbedingung: Option<String>,
            grund: String,
        }
        let roh = Roh::deserialize(deserializer)?;
        let bindungspunkt = match (roh.signatur_slot, roh.geltungsbedingung) {
            (Some(s), None) => Bindungspunkt::SignaturSlot(s),
            (None, Some(g)) => Bindungspunkt::Geltungsbedingung(g),
            _ => {
                return Err(serde::de::Error::custom(
                    "luecke braucht genau eines von signatur_slot/geltungsbedingung",
                ))
            }
        };
        Ok(Self {
            regel_id: roh.regel_id,
            bindungspunkt,
            grund: roh.grund,
        })
    }
}

/// Eine Regel ist nur relevant, wenn `feld` bestaetigt den Wert `wert` traegt
/// (`$defs/regel_bedingung`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegelBedingung {
    pub regel_id: String,
    pub feld: String,
    pub wert: Value,
    pub grund: String,
}

/// Woher die Instanz-Anzahl einer Instanz-Gruppe kommt (`$defs/instanz_gruppe`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstanzGruppe {
    pub gruppe: String,
    pub anzahl_feld: String,
    pub etikett: String,
    pub max: u32,
    pub grund: String,
}

/// Eine Regel, die den Fragebogen eroeffnet (`$defs/thema_zuerst`).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemaZuerst {
    pub regel_id: String,
    pub grund: String,
}

/// Eine vollstaendige `bindung_*.yaml`-Datei.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BindungDatei {
    pub version: u32,
    pub scheibe: String,
    pub bindungen: Vec<Bindung>,
    #[serde(default)]
    pub luecken: Vec<Luecke>,
    #[serde(default)]
    pub regel_bedingungen: Vec<RegelBedingung>,
    #[serde(default)]
    pub instanz_gruppen: Vec<InstanzGruppe>,
    #[serde(default)]
    pub themen_zuerst: Vec<ThemaZuerst>,
}

/// Laedt und validiert eine `bindung_*.yaml`-Datei (jede [`Bindung`] einzeln ueber
/// [`Bindung::validieren`]).
///
/// # Errors
/// [`BindungFehler`] bei I/O-, YAML- oder Validierungsfehlern.
pub fn lade_bindung(pfad: &Path) -> Result<BindungDatei, BindungFehler> {
    let text = std::fs::read_to_string(pfad).map_err(|e| BindungFehler::Io {
        pfad: pfad.to_path_buf(),
        nachricht: e.to_string(),
    })?;
    let datei: BindungDatei = serde_yaml_ng::from_str(&text).map_err(|e| BindungFehler::Yaml {
        pfad: pfad.to_path_buf(),
        nachricht: e.to_string(),
    })?;
    for b in &datei.bindungen {
        b.validieren()?;
    }
    Ok(datei)
}

#[cfg(test)]
mod tests {
    use super::{ist_gueltige_elster_kz, ist_gueltige_feld_id};

    #[test]
    fn feld_id_zeichensatz() {
        assert!(ist_gueltige_feld_id("vv_einnahmen"));
        assert!(ist_gueltige_feld_id("vv_einnahmen__2"));
        assert!(!ist_gueltige_feld_id("Vv_einnahmen"));
        assert!(!ist_gueltige_feld_id("2vv"));
        assert!(!ist_gueltige_feld_id(""));
    }

    #[test]
    fn elster_kz_format() {
        assert!(ist_gueltige_elster_kz("E0123456"));
        assert!(!ist_gueltige_elster_kz("E012345"));
        assert!(!ist_gueltige_elster_kz("e0123456"));
    }

    /// Grund fuer die Wahl von `serde_yaml_ng` (statt z.B. `serde_yaml`, unmaintained seit
    /// 2024): ein doppelter Schluessel in derselben Mapping-Ebene ist ein YAML-Parse-Fehler,
    /// keine stille Letzter-gewinnt-Ueberschreibung. Eine kopierte Bindung mit vergessenem
    /// `feld_id`-Update waere sonst ein zweiter, unbemerkt verschwundener Eintrag.
    #[test]
    fn doppelte_yaml_schluessel_werden_abgewiesen() {
        let yaml = "a: 1\na: 2\n";
        let ergebnis: Result<serde_yaml_ng::Value, _> = serde_yaml_ng::from_str(yaml);
        assert!(
            ergebnis.is_err(),
            "serde_yaml_ng haette doppelte Schluessel abweisen muessen"
        );
    }
}
