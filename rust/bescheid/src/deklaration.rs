//! `bescheid_deklaration.py` (Schritt 7b): Sperrgruende und Ring-Werte — was einer Abgabe im Weg
//! steht und welche gerechneten Groessen in die Deklaration zurueckfliessen.
//!
//! - [`mit_ring_werten`] (`_mit_ring_werten`): berechnete Kz als fertige Events in den Snapshot,
//! - [`an_gesamt_sperrgrund`] (`_an_gesamt_sperrgrund`): der K2-Guard, liefert einen
//!   [`Sperrgrund`] oder `None`,
//! - [`sperrgrund_klartext`], [`sperrgrund_felder`], [`rentenbeginn_offen_stand`],
//!   [`vorlaeufige_ring_betraege`].
//!
//! Sperrgruende sind [`domain::Sperrgrund`]; ihr Klartext liegt dort (exhaustiver `match`).
//! Die `cfg`-Scheibe aus `api_constants.SCHEIBEN` ist [`Cfg`].
mod konstanten;
mod ring_werte;
mod sperre;

use domain::{Feldtyp, Scheibe, Sperrgrund, Zustand, UNBEKANNTER_SPERRGRUND};
use konsistenz::{partner_ohne_zusammen, PartnerWiderspruch};
use serde_json::{json, Value};

pub use ring_werte::mit_ring_werten;
pub use sperre::an_gesamt_sperrgrund;

use ring_werte::berechnet_herkunft;

use crate::{ist_positive_zahl, py_int, py_wahr, wert, BescheidFehler, BindungIndex, Felder};

/// Die Teile von `SCHEIBEN[<name>]`, die `bescheid_deklaration.py` liest.
///
/// Fuenf Scheiben existieren ([`Scheibe`]); eine neue braucht ohnehin neuen Python-Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cfg {
    scheibe: Scheibe,
    gesamt_guard: bool,
    rentner: bool,
    partner_19: bool,
    multi_objekt: Option<&'static str>,
    multi_rente: Option<&'static str>,
    fremd_arten: &'static [&'static str],
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
        };
        match scheibe {
            Scheibe::Ep | Scheibe::NVorGwg | Scheibe::AnGesamt => leer,
            Scheibe::Gesamt => Self {
                gesamt_guard: true,
                partner_19: true,
                multi_objekt: Some("vv_objekt"),
                fremd_arten: &["kein_sonstige", "kein_p23_verkauf"],
                ..leer
            },
            Scheibe::RentnerGesamt => Self {
                gesamt_guard: true,
                rentner: true,
                multi_rente: Some("rente"),
                fremd_arten: &["kein_vuv", "kein_p23_verkauf"],
                ..leer
            },
        }
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
    let beginn_ist_int = match wert(felder, "rentner_renten_beginn_jahr") {
        Some(Value::Bool(_)) => true,
        Some(Value::Number(n)) => n.is_i64() || n.is_u64(),
        _ => false,
    };
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
                    "multi_objekt": c.multi_objekt, "multi_rente": c.multi_rente, "fremd_arten": c.fremd_arten}),
            )
        })
        .collect();
    json!({"tabellen": tabellen, "ring_kandidaten": ring, "cfg": cfg})
}
