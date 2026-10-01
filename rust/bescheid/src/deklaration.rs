//! `bescheid_deklaration.py` (Schritt 7b): Sperrgruende und Ring-Werte — was einer Abgabe im Weg
//! steht und welche gerechneten Groessen in die Deklaration zurueckfliessen.
//!
//! - [`mit_ring_werten`] (`_mit_ring_werten`): berechnete Kz als fertige Events in den Snapshot,
//! - [`an_gesamt_sperrgrund`] (`_an_gesamt_sperrgrund`): der K2-Guard, liefert einen
//!   [`Sperrgrund`] oder `None`,
//! - [`sperrgrund_klartext`], [`sperrgrund_felder`], [`rentenbeginn_offen_stand`],
//!   [`vorlaeufige_ring_betraege`],
//! - [`einreichungs_xml`] (`api.einreichen` bis zum XML): Ring-Werte, Guard, `deklariere` und
//!   `erzeuge_xml` in Pythons Reihenfolge.
//!
//! Sperrgruende sind [`domain::Sperrgrund`]; ihr Klartext liegt dort (exhaustiver `match`).
//! Die `cfg`-Scheibe aus `api_constants.SCHEIBEN` ist [`Cfg`].
mod einreichung;
mod feste_zahl;
mod konstanten;
mod ring_werte;
mod scheiben_tabellen;

use scheiben_tabellen as tab;
mod sperre;

use domain::{Feldtyp, PyWert, Scheibe, Sperrgrund, Zustand, UNBEKANNTER_SPERRGRUND};
use konsistenz::{partner_ohne_zusammen, PartnerWiderspruch};
use serde_json::{json, Value};

pub use einreichung::{einreichungs_xml, EinreichFehler, Einreichung};
pub use feste_zahl::{feste_zahl, FesteZahl, KeineZahl, KeineZahlGrund};
pub use ring_werte::mit_ring_werten;
pub use sperre::an_gesamt_sperrgrund;

use ring_werte::berechnet_herkunft;

use crate::{ist_positive_zahl, py_int, py_wahr, wert, BescheidFehler, BindungIndex, Felder};

/// Die Teile von `SCHEIBEN[<name>]`, die `bescheid_deklaration.py` liest.
///
/// Fuenf Scheiben existieren ([`Scheibe`]); eine neue braucht ohnehin neuen Python-Code.
// Vier unabhaengige Scheiben-Schluessel aus `SCHEIBEN`, kein verkappter Zustandsautomat.
// `guard` und `gesamt_guard` sind NICHT dasselbe: `an_gesamt` traegt `guard=true`,
// `gesamt_guard=false` (gemessen 2026-10-01) -- zusammenlegen waere eine falsche Auskunft.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cfg {
    scheibe: Scheibe,
    gesamt_guard: bool,
    rentner: bool,
    partner_19: bool,
    multi_objekt: Option<&'static str>,
    multi_rente: Option<&'static str>,
    fremd_arten: &'static [&'static str],
    /// `SCHEIBEN[<name>]["felder"]`: die Feld-Ids der Scheibe. `None` heisst **nicht** "keine
    /// Felder", sondern "lies [`Cfg::felder_datei`]" (`n_vor_gwg` ist die einzige solche Scheibe).
    /// Ein Port, der `None` als leer liest, liefert 69 Felder weniger ohne Fehler.
    felder: Option<&'static [&'static str]>,
    /// `SCHEIBEN[<name>]["felder_datei"]`: die YAML, aus der `felder` kommt, wenn es `None` ist.
    felder_datei: Option<&'static str>,
    /// `SCHEIBEN[<name>]["kegel"]`: die Pflicht-Spannen-Achsen. `None` heisst "der volle
    /// `felder`-Satz" (`_scheibe_felder`), nicht "kein Kegel".
    kegel: Option<&'static [&'static str]>,
    /// `SCHEIBEN[<name>]["gesamt_ring"]`: die Quantitaet des Scheiben-Gesamtbescheids, oder `None`
    /// ("diese Scheibe hat keine Scheiben-Zahl"). `_feste_zahl` liest genau diesen Schluessel.
    gesamt_ring: Option<&'static str>,
    /// `SCHEIBEN[<name>].get("guard")`: ob der K2-Guard [`an_gesamt_sperrgrund`] ueberhaupt laeuft.
    /// Fehlt der Schluessel, ist er `false` -- genau wie `cfg.get("guard")` in Python.
    guard: bool,
    /// `SCHEIBEN[<name>]["teil_ringe"]`: `(familie, quantitaet, felder)` je Teil-Ring.
    teil_ringe: &'static [(&'static str, &'static str, &'static [&'static str])],
}

impl Cfg {
    /// `SCHEIBEN[scheibe]` (`api_constants.py:794`).
    ///
    /// ```
    /// use bescheid::deklaration::Cfg;
    /// use domain::Scheibe;
    /// assert_eq!(Cfg::fuer(Scheibe::Ep), Cfg::fuer(Scheibe::Ep));
    /// assert_ne!(Cfg::fuer(Scheibe::Gesamt), Cfg::fuer(Scheibe::RentnerGesamt));
    /// ```
    #[must_use]
    pub const fn fuer(scheibe: Scheibe) -> Self {
        let leer = Self {
            scheibe,
            gesamt_guard: false,
            rentner: false,
            partner_19: false,
            multi_objekt: None,
            multi_rente: None,
            fremd_arten: &[],
            felder: None,
            felder_datei: None,
            kegel: None,
            gesamt_ring: None,
            guard: false,
            teil_ringe: &[],
        };
        match scheibe {
            Scheibe::Ep => Self {
                felder: Some(&tab::SCHEIBEN_EP_FELDER),
                kegel: Some(&tab::SCHEIBEN_EP_KEGEL),
                gesamt_ring: Some("abziehbarer_betrag"),
                ..leer
            },
            // Die einzige Scheibe mit `felder = None`: die Feldliste kommt aus der YAML.
            Scheibe::NVorGwg => Self {
                felder_datei: Some("bindung_n_vor_gwg.yaml"),
                teil_ringe: &tab::SCHEIBEN_N_VOR_GWG_TEIL_RINGE,
                ..leer
            },
            Scheibe::AnGesamt => Self {
                felder: Some(&tab::SCHEIBEN_AN_GESAMT_FELDER),
                kegel: Some(&tab::SCHEIBEN_AN_GESAMT_KEGEL),
                gesamt_ring: Some("festzusetzende_est"),
                guard: true,
                ..leer
            },
            Scheibe::Gesamt => Self {
                felder: Some(&tab::SCHEIBEN_GESAMT_FELDER),
                kegel: Some(&tab::SCHEIBEN_GESAMT_KEGEL),
                gesamt_ring: Some("festzusetzende_est_gesamt"),
                guard: true,
                gesamt_guard: true,
                partner_19: true,
                multi_objekt: Some("vv_objekt"),
                fremd_arten: &["kein_sonstige", "kein_p23_verkauf"],
                ..leer
            },
            Scheibe::RentnerGesamt => Self {
                felder: Some(&tab::SCHEIBEN_RENTNER_GESAMT_FELDER),
                kegel: Some(&tab::SCHEIBEN_RENTNER_GESAMT_KEGEL),
                gesamt_ring: Some("festzusetzende_est_rentner"),
                guard: true,
                gesamt_guard: true,
                rentner: true,
                multi_rente: Some("rente"),
                fremd_arten: &["kein_vuv", "kein_p23_verkauf"],
                ..leer
            },
        }
    }
}

/// Ein Fehler beim Aufloesen der Scheiben-Feldliste: die Scheibe hat `felder = None`, aber keine
/// `felder_datei`, unter der sie nachzuschlagen waere. Ein stiller Rueckfall auf "leer" waere die
/// Auskunft "diese Scheibe hat keine Felder" -- falsch und ohne Signal.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("Scheibe {scheibe} hat felder=None, aber keine felder_datei")]
pub struct FeldlisteFehler {
    pub scheibe: Scheibe,
}

impl Cfg {
    /// Die Feld-Ids der Scheibe, wie `api._scheibe_felder`: `cfg["felder"]`, sonst die `feld_id`s
    /// aus `cfg["felder_datei"]` in Dateireihenfolge.
    ///
    /// `bindung` liefert die Feld-Ids der Datei; welche Datei, sagt [`Cfg::felder_datei`]. Ein
    /// Aufrufer, der `felder()` ohne Bindung braucht, hat hier nichts zu suchen -- die Scheiben
    /// `ep`, `an_gesamt`, `gesamt`, `rentner_gesamt` tragen ihre Liste selbst.
    ///
    /// # Errors
    /// [`FeldlisteFehler`], wenn `felder` und `felder_datei` beide `None` sind.
    pub fn felder(
        &self,
        datei_felder: impl FnOnce(&str) -> Vec<String>,
    ) -> Result<Vec<String>, FeldlisteFehler> {
        match (self.felder, self.felder_datei) {
            (Some(f), _) => Ok(f.iter().map(|s| (*s).to_owned()).collect()),
            (None, Some(d)) => Ok(datei_felder(d)),
            (None, None) => Err(FeldlisteFehler {
                scheibe: self.scheibe,
            }),
        }
    }

    /// `cfg["kegel"]`, oder der volle `felder`-Satz, wenn der Schluessel fehlt (`_scheibe_felder`
    /// in `api.py:576`: `cfg.get("kegel") or _scheibe_felder(store)`).
    ///
    /// # Errors
    /// Wie [`Cfg::felder`].
    pub fn kegel(
        &self,
        datei_felder: impl FnOnce(&str) -> Vec<String>,
    ) -> Result<Vec<String>, FeldlisteFehler> {
        match self.kegel {
            Some(k) => Ok(k.iter().map(|s| (*s).to_owned()).collect()),
            None => self.felder(datei_felder),
        }
    }

    /// `cfg["gesamt_ring"]`: die Quantitaet des Scheiben-Gesamtbescheids, `None` fuer `n_vor_gwg`.
    #[must_use]
    pub const fn gesamt_ring(&self) -> Option<&'static str> {
        self.gesamt_ring
    }

    /// `cfg.get("guard")`: ob der K2-Guard laeuft. Fehlender Schluessel ist `false`.
    #[must_use]
    pub const fn guard(&self) -> bool {
        self.guard
    }

    /// `cfg["teil_ringe"]`: `(familie, quantitaet, felder)` je Teil-Ring.
    #[must_use]
    pub const fn teil_ringe(
        &self,
    ) -> &'static [(&'static str, &'static str, &'static [&'static str])] {
        self.teil_ringe
    }

    /// `cfg["felder_datei"]`: die YAML, aus der [`Cfg::felder`] liest, wenn `felder` `None` ist.
    #[must_use]
    pub const fn felder_datei(&self) -> Option<&'static str> {
        self.felder_datei
    }

    /// `cfg["felder"]` als `Option`, ohne YAML-Aufloesung: `None` heisst "lies die Datei", nicht
    /// "keine Felder". Wer die fertige Liste will, nimmt [`Cfg::felder`].
    #[must_use]
    pub const fn felder_roh(&self) -> Option<&'static [&'static str]> {
        self.felder
    }

    /// `cfg["kegel"]` als `Option`, ohne Rueckfall auf `felder`: `None` heisst "der volle Satz".
    #[must_use]
    pub const fn kegel_roh(&self) -> Option<&'static [&'static str]> {
        self.kegel
    }

    /// Die Scheibe, zu der diese `Cfg` gehoert.
    #[must_use]
    pub const fn scheibe(&self) -> Scheibe {
        self.scheibe
    }
}

/// Python `int(felder.get(fid, {}).get("wert") or 0)`.
///
/// PARITÄT: fail-open default — fehlend, `None`, `False`, `0`, `""` sind 0; ein nicht-numerischer
/// Text ist ein `ValueError`, eine Liste ein `TypeError` (Python-Ausnahme, kein Default).
fn c2(f: &Felder, fid: &str) -> Result<i64, BescheidFehler> {
    match wert(f, fid) {
        Some(v) if py_wahr(v) => py_int(v),
        _ => Ok(0),
    }
}

/// Der Satz zu einem Sperrgrund, den ein Laie lesen kann. `None` (keine Sperre) bleibt `None`; jeder
/// andere Wert liefert einen Satz, nie die rohe Kennung.
///
/// PARITÄT: `Bestaetigt` ist kein Sperrgrund und hat in `SPERRGRUND_KLARTEXT` keinen Eintrag —
/// Python liefert dafuer [`UNBEKANNTER_SPERRGRUND`], hier ebenso.
///
/// ```
/// use bescheid::deklaration::sperrgrund_klartext;
/// use domain::Sperrgrund;
/// assert_eq!(sperrgrund_klartext(None), None);
/// assert!(sperrgrund_klartext(Some(Sperrgrund::PartnerKonsistenzOffen)).is_some());
/// ```
#[must_use]
pub fn sperrgrund_klartext(grund: Option<Sperrgrund>) -> Option<&'static str> {
    grund.map(|g| g.klartext().unwrap_or(UNBEKANNTER_SPERRGRUND))
}

/// Wie [`sperrgrund_klartext`] fuer eine rohe Kennung (`grund: str` aus einer anderen Quelle als
/// `an_gesamt_sperrgrund`): eine unbekannte Kennung liefert [`UNBEKANNTER_SPERRGRUND`].
///
/// ```
/// use bescheid::deklaration::sperrgrund_klartext_text;
/// assert_eq!(sperrgrund_klartext_text(None), None);
/// assert_eq!(sperrgrund_klartext_text(Some("gibt_es_nicht")), Some(domain::UNBEKANNTER_SPERRGRUND));
/// ```
#[must_use]
pub fn sperrgrund_klartext_text(grund: Option<&str>) -> Option<&'static str> {
    grund.map(|s| {
        s.parse::<Sperrgrund>()
            .ok()
            .and_then(Sperrgrund::klartext)
            .unwrap_or(UNBEKANNTER_SPERRGRUND)
    })
}

/// Die Angaben, die einen Widerspruchs-Sperrgrund ausloesen — leer bei allen anderen. Gegenstueck zu
/// [`sperrgrund_klartext`]: wer den Grund liefert, sagt auch, WORAN er haengt.
///
/// ```
/// use bescheid::deklaration::sperrgrund_felder;
/// use bescheid::Felder;
/// use domain::Sperrgrund;
/// assert!(sperrgrund_felder(Some(Sperrgrund::DhfTatbestandOffen), &Felder::new()).is_empty());
/// assert!(sperrgrund_felder(None, &Felder::new()).is_empty());
/// ```
#[must_use]
pub fn sperrgrund_felder(grund: Option<Sperrgrund>, felder: &Felder) -> Vec<PartnerWiderspruch> {
    if grund == Some(Sperrgrund::PartnerKonsistenzOffen) {
        partner_ohne_zusammen(felder)
    } else {
        Vec::new()
    }
}

/// § 22 aa Rentenfreibetrag (`/stand`-spezifisch): eine Jahresrente > 0 ohne ganzzahliges
/// Beginnjahr sperrt mit `rentenbeginn_offen`. NUR von `stand()` gerufen, nicht von
/// [`an_gesamt_sperrgrund`].
///
/// PARITÄT: Pythons `isinstance(x, int)` laesst `True`/`False` als Beginnjahr durch.
///
/// ```
/// use bescheid::deklaration::{rentenbeginn_offen_stand, Cfg};
/// use bescheid::testhilfe::{felder, store};
/// use domain::{Scheibe, Sperrgrund};
/// use serde_json::json;
/// let f = felder(&store(&[("rentner_jahresrente", json!(1_200_000), true)]));
/// let cfg = Cfg::fuer(Scheibe::RentnerGesamt);
/// assert_eq!(rentenbeginn_offen_stand(&f, Some(&cfg)), Some(Sperrgrund::RentenbeginnOffen));
/// assert_eq!(rentenbeginn_offen_stand(&f, None), None);
/// ```
#[must_use]
pub fn rentenbeginn_offen_stand(felder: &Felder, cfg: Option<&Cfg>) -> Option<Sperrgrund> {
    if !cfg.is_some_and(|c| c.rentner) {
        return None;
    }
    // `isinstance(beginn, int)`: `bool` zaehlt in `CPython` als `int`. `Gleit` nicht, Text nicht.
    let beginn_ist_int = matches!(
        wert(felder, "rentner_renten_beginn_jahr"),
        Some(PyWert::Bool(_) | PyWert::Ganz(_) | PyWert::GrossGanz(_))
    );
    (ist_positive_zahl(wert(felder, "rentner_jahresrente")) && !beginn_ist_int)
        .then_some(Sperrgrund::RentenbeginnOffen)
}

/// Die vorlaeufigen Betragsfelder DIESER Scheibe, die der Ring liest (Klasse C). Der Ring filtert auf
/// `bestaetigt`: ein vorlaeufiger Betrag faellt still aus der Zahl, die Zahl hiesse trotzdem
/// "bestaetigt". Ausgenommen sind Kegel-Felder (Pflicht-Seite, deckt der Kegel-Meet) und Felder, die
/// keine Betraege sind (`typ` nicht `cent`/`int`).
///
/// `bindung`: die Bindung der Scheibe (`_scheibe_bindung`).
///
/// ```
/// use bescheid::deklaration::{vorlaeufige_ring_betraege, Cfg};
/// use bescheid::testhilfe::{felder, index, store};
/// use domain::Scheibe;
/// use serde_json::json;
/// let f = felder(&store(&[("p36_lohnsteuer", json!(500_000), false)]));
/// let vorl = vorlaeufige_ring_betraege(&f, &Cfg::fuer(Scheibe::AnGesamt), index());
/// assert!(vorl.contains(&"p36_lohnsteuer"));
/// ```
#[must_use]
pub fn vorlaeufige_ring_betraege(
    felder: &Felder,
    cfg: &Cfg,
    bindung: &BindungIndex<'_>,
) -> Vec<&'static str> {
    konstanten::ring_kandidaten(cfg.scheibe)
        .iter()
        .copied()
        .filter(|fid| {
            bindung
                .get(*fid)
                .is_some_and(|b| matches!(b.typ, Feldtyp::Cent | Feldtyp::Int))
                && felder
                    .get(*fid)
                    .is_some_and(|ev| ev.zustand != Zustand::Bestaetigt)
        })
        .collect()
}

/// Die Tabellen, wie Rust sie traegt, in der Form von `_deklaration_konstanten` im Orakel
/// (`tools/parity/bescheid_oracle.py`): Grundlage von `konstanten_gleich` im Parity-Test.
///
/// ```
/// use bescheid::deklaration::konstanten_json;
/// assert!(konstanten_json()["tabellen"].is_object());
/// ```
#[doc(hidden)]
#[must_use]
pub fn konstanten_json() -> Value {
    let tabellen: serde_json::Map<String, Value> = konstanten::alle()
        .into_iter()
        .map(|(n, t)| (n.to_owned(), json!(t)))
        .collect();
    let scheiben = [
        Scheibe::Ep,
        Scheibe::NVorGwg,
        Scheibe::AnGesamt,
        Scheibe::Gesamt,
        Scheibe::RentnerGesamt,
    ];
    let ring: serde_json::Map<String, Value> = scheiben
        .iter()
        .map(|s| {
            (
                s.als_str().to_owned(),
                json!(konstanten::ring_kandidaten(*s)),
            )
        })
        .collect();
    let cfg: serde_json::Map<String, Value> = scheiben
        .iter()
        .map(|s| {
            let c = Cfg::fuer(*s);
            (
                s.als_str().to_owned(),
                json!({"gesamt_guard": c.gesamt_guard, "rentner": c.rentner, "partner_19": c.partner_19,
                    "multi_objekt": c.multi_objekt, "multi_rente": c.multi_rente, "fremd_arten": c.fremd_arten,
                    "felder": c.felder, "kegel": c.kegel, "gesamt_ring": c.gesamt_ring, "guard": c.guard,
                    "felder_datei": c.felder_datei,
                    "teil_ringe": c.teil_ringe.iter().map(|(f, q, fs)| json!([f, q, fs])).collect::<Vec<_>>()}),
            )
        })
        .collect();
    json!({"tabellen": tabellen, "ring_kandidaten": ring, "cfg": cfg})
}

/// Aequivalenz von `c2` mit `int(v or 0)` (D15); Ausnahmeliste `crate::aequivalenz::INT`.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{ganzzahl_text, json_wert, Ergebnis};
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use crate::aequivalenz::{alt_klasse, int_oder_null, int_oder_null_wie};
    use crate::vor_k2::int_oder_null_alt;

    /// Die Vor-K2-Fassung von `c2`. `c2` selbst ist seit dem Port die Produktion — dieser Helfer
    /// traegt die alte Gestalt und ist die einzige Seite, die die D-Nummern messen darf.
    fn alt(v: &Value) -> Ergebnis<i64> {
        alt_klasse(int_oder_null_alt(v))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn c2_wie_pywert(v in json_wert()) {
            int_oder_null_wie(&v, &alt(&v))?;
        }

        #[test]
        fn c2_text_wie_pywert(s in ganzzahl_text()) {
            let v = Value::String(s);
            int_oder_null_wie(&v, &alt(&v))?;
        }
    }

    /// D6: `int("٣" or 0)` ist in `CPython` 3.
    #[test]
    fn d6_nd_ziffer() {
        let v = json!("\u{663}");
        assert_eq!(alt(&v), Err(Some("ValueError")));
        assert_eq!(int_oder_null(&v), Ok(3));
    }

    /// D11: `int(-2.0**63 or 0)` ist in `CPython` `i64::MIN`.
    #[test]
    fn d11_minus_2_hoch_63() {
        let v = json!(-9_223_372_036_854_775_808.0);
        assert_eq!(alt(&v), Err(None));
        assert_eq!(int_oder_null(&v), Ok(i64::MIN));
    }

    /// D16: `int("\x1c42" or 0)` wirft in `CPython` `ValueError`.
    #[test]
    fn d16_steuerzeichen_am_rand() {
        let v = json!("\u{1c}42");
        assert_eq!(alt(&v), Ok(42));
        assert_eq!(int_oder_null(&v), Err(Some("ValueError")));
    }

    /// D17: `int()` mit mehr als 4300 Ziffern wirft in `CPython` `ValueError`.
    #[test]
    fn d17_mehr_als_4300_ziffern() {
        let v = json!(format!("{}5", "0".repeat(4300)));
        assert_eq!(alt(&v), Ok(5));
        assert_eq!(int_oder_null(&v), Err(Some("ValueError")));
    }
}
