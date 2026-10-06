//! `bindung_*.yaml`: eine Bindungstabelle (`rust/bindung/daten/schema.json`, "Bindungstabelle
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
    #[error("{feld_id}: frage_invertiert=true braucht typ=bool und askable=true")]
    InvertiertOhneBoolAskable { feld_id: String },
    #[error("ungueltige feld_id {0:?} (erwartet ^[a-z][a-z0-9_]*$)")]
    UngueltigeFeldId(String),
    #[error("{feld_id}: vz_gueltigkeit darf nicht leer sein")]
    LeereVzGueltigkeit { feld_id: String },
    #[error("{feld_id}: bereich gibt es nur bei typ cent/int, nicht bei typ {typ}")]
    BereichBeiFremdemTyp { feld_id: String, typ: String },
    #[error("{feld_id}: bereich min {min} liegt ueber max {max}, jede Zahl ausser 0 waere abgewiesen")]
    BereichVerdreht { feld_id: String, min: i64, max: i64 },
    #[error("{feld_id}: negativer cent-Bereich (min {min}) braucht bereich.grund, die Verlust-Begruendung")]
    NegativerCentBereichOhneGrund { feld_id: String, min: i64 },
    #[error("{feld_id}: beispielwert {beispielwert} liegt ausserhalb des eigenen bereich [{min}, {max}]")]
    BeispielwertAusserhalbBereich {
        feld_id: String,
        beispielwert: String,
        min: i64,
        max: i64,
    },
    #[error("{feld_id}: fragetext_laie nennt ein Gesetzeskuerzel oder eine Fundstelle ({fragetext:?}), der Laie liest Klartext")]
    FragetextMitGesetzeskuerzel { feld_id: String, fragetext: String },
    #[error("{wo}: {was} braucht mindestens {min} Zeichen")]
    ZuKurz {
        wo: String,
        was: &'static str,
        min: usize,
    },
    #[error("{feld_id}: enum_werte darf nicht leer sein")]
    EnumWerteLeer { feld_id: String },
    #[error("{feld_id}: feld_bedingung braucht genau eines von wert/wert_nicht")]
    FeldBedingungNichtGenauEins { feld_id: String },
    #[error("{feld_id}: ungueltige instanz_gruppe {gruppe:?} (erwartet ^[a-z][a-z0-9_]*$)")]
    UngueltigeInstanzGruppe { feld_id: String, gruppe: String },
    #[error("instanz_gruppen {gruppe}: max {max} liegt ausserhalb von {}..={}", MIN_INSTANZ_MAX, MAX_INSTANZ_MAX)]
    InstanzGruppeMaxAusserhalb { gruppe: String, max: u32 },
    #[error("regel_bedingungen {regel_id}: ungueltiges feld {feld:?} (erwartet ^[a-z][a-z0-9_]*$)")]
    UngueltigesRegelBedingungFeld { regel_id: String, feld: String },
    #[error("version {0} ist zu klein, mindestens 1")]
    VersionZuKlein(u32),
    #[error("{feld_id}: muster {muster:?} ist keine gueltige Regex der regex-Crate ({nachricht}); der Store wiese jeden Wert des Feldes ab")]
    UngueltigesMuster {
        feld_id: String,
        muster: String,
        nachricht: String,
    },
    #[error("{feld_id}: beispielwert {beispielwert:?} passt nicht zum eigenen muster {muster:?}")]
    BeispielwertPasstNichtZumMuster {
        feld_id: String,
        beispielwert: String,
        muster: String,
    },
}

/// Mindestlaenge von `hilfe_kurz` und `anker_ref.zitatanker` (Schema: `minLength: 3`).
const MIN_KURZTEXT: usize = 3;
/// Mindestlaenge einer Begruendung, die kurz sein darf (`regel_bedingung.grund`, `luecke.grund`).
const MIN_GRUND_KURZ: usize = 5;
/// Mindestlaenge einer Begruendung, die eine Entscheidung erklaert (`feld_bedingung.grund`,
/// `ableitung.grund`, `instanz_gruppe.grund`, `thema_zuerst.grund`).
const MIN_GRUND_LANG: usize = 40;
/// Grenzen von `instanz_gruppe.max` (Schema: `minimum 1`, `maximum 20`).
const MIN_INSTANZ_MAX: u32 = 1;
const MAX_INSTANZ_MAX: u32 = 20;

/// Weniger als `min` Zeichen, gezaehlt nach Zeichen und nicht nach Bytes (wie `minLength`).
fn zu_kurz(text: &str, min: usize) -> bool {
    text.chars().count() < min
}

/// Ein Baustein der kleinen Mustersuche in [`nennt_gesetzeskuerzel`]: ein festes Zeichen, ein
/// optionales Leerzeichen (`\s?`) oder eine Ziffer (`[0-9]`).
enum Baustein {
    Zeichen(char),
    Leerraum,
    Ziffer,
}

/// Passt `muster` am Anfang von `zeichen`?
fn passt_am_anfang(zeichen: &[char], muster: &[Baustein]) -> bool {
    let mut i = 0;
    for baustein in muster {
        match baustein {
            Baustein::Zeichen(soll) => {
                if zeichen.get(i) != Some(soll) {
                    return false;
                }
                i += 1;
            }
            Baustein::Leerraum => {
                if zeichen.get(i).is_some_and(|c| c.is_whitespace()) {
                    i += 1;
                }
            }
            Baustein::Ziffer => {
                if !zeichen.get(i).is_some_and(char::is_ascii_digit) {
                    return false;
                }
                i += 1;
            }
        }
    }
    true
}

/// Steht `muster` irgendwo in `text`?
fn enthaelt_muster(text: &str, muster: &[Baustein]) -> bool {
    let zeichen: Vec<char> = text.chars().collect();
    (0..zeichen.len()).any(|anfang| {
        zeichen
            .get(anfang..)
            .is_some_and(|rest| passt_am_anfang(rest, muster))
    })
}

/// Das Verbot aus `schema.json` fuer `fragetext_laie`: der Laie liest Klartext, kein Gesetz. Das
/// Schema verbietet (Regex-Suche) `§|EStG|GewStG|KStG|Abs\.|i\.\s?S\.\s?d\.|Satz\s?[0-9]|Aufwendungen i`.
/// Ohne Regex-Abhaengigkeit nachgebaut; `\s` ist hier `char::is_whitespace`.
fn nennt_gesetzeskuerzel(text: &str) -> bool {
    use Baustein::{Leerraum, Zeichen, Ziffer};
    const TEILE: [&str; 6] = ["§", "EStG", "GewStG", "KStG", "Abs.", "Aufwendungen i"];
    // i. S. d.
    let i_s_d = [
        Zeichen('i'),
        Zeichen('.'),
        Leerraum,
        Zeichen('S'),
        Zeichen('.'),
        Leerraum,
        Zeichen('d'),
        Zeichen('.'),
    ];
    // Satz 3
    let satz_ziffer = [
        Zeichen('S'),
        Zeichen('a'),
        Zeichen('t'),
        Zeichen('z'),
        Leerraum,
        Ziffer,
    ];
    TEILE.iter().any(|teil| text.contains(teil))
        || enthaelt_muster(text, &i_s_d)
        || enthaelt_muster(text, &satz_ziffer)
}

/// `^[a-z][a-z0-9_]*$` ohne Regex-Abhaengigkeit (nur ASCII, wie das Schema selbst). Die eine Regel
/// steht in `domain`; `BasisId::new` ruft dieselbe Funktion.
pub use domain::ist_gueltige_feld_id;

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
// Vier unabhaengige Schalter der Bindungsdatei (`askable`, `frage_invertiert`, `eingangsfrage`,
// `nicht_negativ`), wie sie `schema.json` fuehrt, kein verkappter Zustandsautomat.
#[allow(clippy::struct_excessive_bools)]
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
    pub elster_kz: Option<Kz>,
    pub elster_kz_grund: Option<String>,
    pub kz_status: Option<KzStatus>,
    pub vz_gueltigkeit: Vec<i64>,
    pub anker_ref: AnkerRef,
    pub enum_werte: Option<Vec<String>>,
    pub bereich: Option<Bereich>,
    /// Das Feld kennt im amtlichen Schema kein Minus (`nicht_negativ: true`); der Store weist
    /// eine negative Zahl beim Schreiben ab.
    #[serde(default)]
    pub nicht_negativ: bool,
    pub screening: Option<bool>,
    pub beweist: Option<Beweist>,
    pub feld_bedingung: Option<FeldBedingung>,
    pub ableitung: Option<Ableitung>,
}

impl Bindung {
    /// Die vier `allOf`-Regeln aus `schema.json` plus `feld_id`-Zeichensatz,
    /// nicht-leere `vz_gueltigkeit` und die Bereichs-Regeln, die `serde` allein nicht
    /// ausdruecken kann.
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
        if self.elster_kz.is_none() && self.elster_kz_grund.is_none() {
            return Err(BindungFehler::KzNullOhneGrund {
                feld_id: self.feld_id.clone(),
            });
        }
        if self.frage_invertiert && !(matches!(self.typ, Feldtyp::Bool) && self.askable) {
            return Err(BindungFehler::InvertiertOhneBoolAskable {
                feld_id: self.feld_id.clone(),
            });
        }
        if let Some(bereich) = &self.bereich {
            self.pruefe_bereich(bereich)?;
        }
        self.pruefe_muster()?;
        self.pruefe_schema_reste()
    }

    /// `muster` muss eine Regex sein, die die `regex`-Crate uebersetzt, und der `beispielwert` muss
    /// dazu passen. Der Store faengt ein ungueltiges Muster fail-closed ab (`store::passt_muster`:
    /// "nicht passend"), ein Feld mit kaputtem Muster nimmt also keinen einzigen Wert mehr an, ohne
    /// dass der Dienst beim Start etwas meldet. Auch ein Muster, das Pythons `re` kennt und die
    /// `regex`-Crate nicht (Lookahead, Rueckverweis), fiele so still aus.
    fn pruefe_muster(&self) -> Result<(), BindungFehler> {
        let Some(muster) = &self.muster else {
            return Ok(());
        };
        // Derselbe Rahmen wie `store::passt_muster` (`re.fullmatch`): das Muster gilt fuer den
        // ganzen Wert. Aendert sich der Rahmen dort, muss er hier mit.
        let regex = regex::Regex::new(&format!("^(?:{muster})$")).map_err(|e| {
            BindungFehler::UngueltigesMuster {
                feld_id: self.feld_id.clone(),
                muster: muster.clone(),
                nachricht: e.to_string().lines().last().unwrap_or_default().to_string(),
            }
        })?;
        if let Value::String(beispiel) = &self.beispielwert {
            if !regex.is_match(beispiel) {
                return Err(BindungFehler::BeispielwertPasstNichtZumMuster {
                    feld_id: self.feld_id.clone(),
                    beispielwert: beispiel.clone(),
                    muster: muster.clone(),
                });
            }
        }
        Ok(())
    }

    /// Die Regeln aus `schema.json`, die `serde` nicht kennt: Mindestlaengen, das Verbot von
    /// Gesetzeskuerzeln im Fragetext, nicht leere `enum_werte`, genau eine Bedingung in
    /// `feld_bedingung`, Muster von `instanz_gruppe`. Ohne sie gilt eine Bindung als gueltig, die der
    /// Laie nicht lesen kann (Fragetext mit Paragraf) oder deren Bedingung zwei Antworten hat und
    /// still eine davon gewinnen laesst.
    fn pruefe_schema_reste(&self) -> Result<(), BindungFehler> {
        let kurz = |was: &'static str, min: usize| BindungFehler::ZuKurz {
            wo: self.feld_id.clone(),
            was,
            min,
        };
        if zu_kurz(&self.hilfe_kurz, MIN_KURZTEXT) {
            return Err(kurz("hilfe_kurz", MIN_KURZTEXT));
        }
        if zu_kurz(&self.anker_ref.zitatanker, MIN_KURZTEXT) {
            return Err(kurz("anker_ref.zitatanker", MIN_KURZTEXT));
        }
        if let Some(fragetext) = &self.fragetext_laie {
            if nennt_gesetzeskuerzel(fragetext) {
                return Err(BindungFehler::FragetextMitGesetzeskuerzel {
                    feld_id: self.feld_id.clone(),
                    fragetext: fragetext.clone(),
                });
            }
        }
        if self.enum_werte.as_ref().is_some_and(Vec::is_empty) {
            return Err(BindungFehler::EnumWerteLeer {
                feld_id: self.feld_id.clone(),
            });
        }
        if let Some(gruppe) = &self.instanz_gruppe {
            if !ist_gueltige_feld_id(gruppe) {
                return Err(BindungFehler::UngueltigeInstanzGruppe {
                    feld_id: self.feld_id.clone(),
                    gruppe: gruppe.clone(),
                });
            }
        }
        if let Some(bedingung) = &self.feld_bedingung {
            if bedingung.wert.is_some() == bedingung.wert_nicht.is_some() {
                return Err(BindungFehler::FeldBedingungNichtGenauEins {
                    feld_id: self.feld_id.clone(),
                });
            }
            if zu_kurz(&bedingung.grund, MIN_GRUND_LANG) {
                return Err(BindungFehler::ZuKurz {
                    wo: format!("{}.feld_bedingung", self.feld_id),
                    was: "grund",
                    min: MIN_GRUND_LANG,
                });
            }
        }
        if let Some(ableitung) = &self.ableitung {
            if zu_kurz(&ableitung.grund, MIN_GRUND_LANG) {
                return Err(BindungFehler::ZuKurz {
                    wo: format!("{}.ableitung", self.feld_id),
                    was: "grund",
                    min: MIN_GRUND_LANG,
                });
            }
        }
        Ok(())
    }

    /// Die Bereichs-Regeln, die `Bereich` als reiner Datentyp nicht kennt. Der Store weist eine
    /// Zahl ausserhalb von `min..=max` ab (ausser 0); ein falscher Bereich macht das Feld also
    /// unbenutzbar, ohne dass der Dienst es meldet.
    // ponytail: ein Gleitkomma-`beispielwert` wird in f64 gegen die Grenzen verglichen. Die Grenzen
    // sind kleine Ganzzahlen (Jahre, Tage, Cent bis ein paar Milliarden); ab 2^53 waere der
    // Vergleich um eine Einheit ungenau. Upgrade: Decimal-Vergleich, falls ein Bereich so gross wird.
    #[allow(clippy::cast_precision_loss)]
    fn pruefe_bereich(&self, bereich: &Bereich) -> Result<(), BindungFehler> {
        if !matches!(self.typ, Feldtyp::Cent | Feldtyp::Int) {
            return Err(BindungFehler::BereichBeiFremdemTyp {
                feld_id: self.feld_id.clone(),
                typ: format!("{:?}", self.typ),
            });
        }
        if bereich.min > bereich.max {
            return Err(BindungFehler::BereichVerdreht {
                feld_id: self.feld_id.clone(),
                min: bereich.min,
                max: bereich.max,
            });
        }
        if matches!(self.typ, Feldtyp::Cent)
            && bereich.min < 0
            && bereich.grund.as_deref().is_none_or(str::is_empty)
        {
            return Err(BindungFehler::NegativerCentBereichOhneGrund {
                feld_id: self.feld_id.clone(),
                min: bereich.min,
            });
        }
        if let Value::Number(zahl) = &self.beispielwert {
            let innerhalb = match zahl.as_i64() {
                Some(i) => bereich.min <= i && i <= bereich.max,
                // Gleitkomma oder ueber i64: Vergleich in f64, die Grenzen sind klein.
                None => zahl
                    .as_f64()
                    .is_some_and(|f| bereich.min as f64 <= f && f <= bereich.max as f64),
            };
            if !innerhalb {
                return Err(BindungFehler::BeispielwertAusserhalbBereich {
                    feld_id: self.feld_id.clone(),
                    beispielwert: zahl.to_string(),
                    min: bereich.min,
                    max: bereich.max,
                });
            }
        }
        Ok(())
    }
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

impl BindungDatei {
    /// Jede [`Bindung`] einzeln ueber [`Bindung::validieren`], dazu die Regeln der Abschnitte
    /// `luecken`, `regel_bedingungen`, `instanz_gruppen` und `themen_zuerst` aus `schema.json`,
    /// die `serde` nicht kennt (Mindestlaengen der Begruendungen, `instanz_gruppe.max` von 1 bis 20,
    /// Muster von `regel_bedingung.feld`) und `version` mindestens 1.
    ///
    /// # Errors
    /// [`BindungFehler`] bei der ersten verletzten Regel.
    pub fn validieren(&self) -> Result<(), BindungFehler> {
        if self.version < 1 {
            return Err(BindungFehler::VersionZuKlein(self.version));
        }
        for b in &self.bindungen {
            b.validieren()?;
        }
        for luecke in &self.luecken {
            if zu_kurz(&luecke.grund, MIN_GRUND_KURZ) {
                return Err(BindungFehler::ZuKurz {
                    wo: format!("luecken {}", luecke.regel_id),
                    was: "grund",
                    min: MIN_GRUND_KURZ,
                });
            }
        }
        for bedingung in &self.regel_bedingungen {
            if !ist_gueltige_feld_id(&bedingung.feld) {
                return Err(BindungFehler::UngueltigesRegelBedingungFeld {
                    regel_id: bedingung.regel_id.clone(),
                    feld: bedingung.feld.clone(),
                });
            }
            if zu_kurz(&bedingung.grund, MIN_GRUND_KURZ) {
                return Err(BindungFehler::ZuKurz {
                    wo: format!("regel_bedingungen {}", bedingung.regel_id),
                    was: "grund",
                    min: MIN_GRUND_KURZ,
                });
            }
        }
        for gruppe in &self.instanz_gruppen {
            if !(MIN_INSTANZ_MAX..=MAX_INSTANZ_MAX).contains(&gruppe.max) {
                return Err(BindungFehler::InstanzGruppeMaxAusserhalb {
                    gruppe: gruppe.gruppe.clone(),
                    max: gruppe.max,
                });
            }
            if zu_kurz(&gruppe.grund, MIN_GRUND_LANG) {
                return Err(BindungFehler::ZuKurz {
                    wo: format!("instanz_gruppen {}", gruppe.gruppe),
                    was: "grund",
                    min: MIN_GRUND_LANG,
                });
            }
        }
        for thema in &self.themen_zuerst {
            if zu_kurz(&thema.grund, MIN_GRUND_LANG) {
                return Err(BindungFehler::ZuKurz {
                    wo: format!("themen_zuerst {}", thema.regel_id),
                    was: "grund",
                    min: MIN_GRUND_LANG,
                });
            }
        }
        Ok(())
    }
}

/// Laedt und validiert eine `bindung_*.yaml`-Datei ([`BindungDatei::validieren`]).
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
    datei.validieren()?;
    Ok(datei)
}

#[cfg(test)]
mod tests {
    use super::{ist_gueltige_feld_id, BindungDatei};

    #[test]
    fn feld_id_zeichensatz() {
        assert!(ist_gueltige_feld_id("vv_einnahmen"));
        assert!(ist_gueltige_feld_id("vv_einnahmen__2"));
        assert!(!ist_gueltige_feld_id("Vv_einnahmen"));
        assert!(!ist_gueltige_feld_id("2vv"));
        assert!(!ist_gueltige_feld_id(""));
    }

    /// Die Kz-Regel `^E[0-9]{7}$` wirkt beim Laden: eine ungueltige `elster_kz` scheitert in
    /// `serde` und nennt den Text, statt als `String` bis in `elster` zu laufen.
    #[test]
    fn ungueltige_elster_kz_scheitert_beim_laden() {
        let yaml = |kz: &str| {
            format!(
                "version: 1\nscheibe: test\nbindungen:\n  - feld_id: testfeld\n    \
                 quelle: {{regel_id: r, signatur_slot: s}}\n    typ: bool\n    askable: false\n    \
                 hilfe_kurz: Tipp\n    beispielwert: true\n    elster_kz: \"{kz}\"\n    \
                 vz_gueltigkeit: [2025]\n    anker_ref: {{quelle: Q, zitatanker: Zit}}\n"
            )
        };
        assert!(serde_yaml_ng::from_str::<BindungDatei>(&yaml("E0123456")).is_ok());
        for kz in ["E012345", "e0123456", "E06004901", ""] {
            let fehler = serde_yaml_ng::from_str::<BindungDatei>(&yaml(kz)).unwrap_err();
            assert!(
                fehler.to_string().contains(&format!("ungueltige Kz {kz:?}")),
                "{kz:?}: {fehler}"
            );
        }
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
