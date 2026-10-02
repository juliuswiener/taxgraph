//! `api.einreichen` bis zum XML (`produkt/haut/api.py:675-748`): Scheibe, Stammdaten-Pruefung,
//! Ring-Werte, K2-Guard, Deklaration, ELSTER-XML. Die ERiC-Pruefung und die Snapshot-Bindung
//! danach gehoeren zum Handler.
use bindung::Params;
use domain::{Scheibe, Sperrgrund, Vz};
use elster::{
    deklariere, erzeuge_xml, Deklaration, DeklarationsFehler, Eintrag, XmlFehler, XmlOptionen,
    TESTMERKER_ERIC,
};
use store::{SnapshotFehler, Store};

use super::konstanten::STAMMDATEN_FELDER;
use super::{an_gesamt_sperrgrund, mit_ring_werten, scheibe_bindung, Cfg, ScheibenFehler};
use crate::{BescheidFehler, BindungIndex, Instanzquelle};

/// Was `api.einreichen` vor der ERiC-Pruefung in der Hand haelt.
#[derive(Debug, Clone)]
pub struct Einreichung {
    /// Das Jahr des Falls (`ESt_<vz>`, XSD).
    pub vz: Vz,
    /// `result`; `basis_snapshot` ist die `sid` aus `materialisiere`.
    pub deklaration: Deklaration,
    /// Das ELSTER-XML, abgabefaehig, mit [`TESTMERKER_ERIC`].
    pub xml: String,
}

/// Warum `api.einreichen` kein XML erreicht; je Variante die Python-Antwort an derselben Stelle.
#[derive(Debug, thiserror::Error)]
pub enum EinreichFehler {
    /// `_cfg` (400) oder `_scheibe_bindung` (500).
    #[error(transparent)]
    Scheibe(#[from] ScheibenFehler),
    /// `ST.materialisiere`: `ValueError`, 500.
    #[error(transparent)]
    Snapshot(#[from] SnapshotFehler),
    /// 409 `scheibe_nicht_abgabefaehig` mit `fehlende_stammdatenfelder`.
    #[error("Scheibe {scheibe} traegt keine Erklaerung, es fehlen {fehlende_stammdatenfelder:?}")]
    ScheibeNichtAbgabefaehig {
        scheibe: Scheibe,
        fehlende_stammdatenfelder: Vec<&'static str>,
    },
    /// Ein Jahr ausserhalb 2024–2026.
    ///
    /// PARITÄT: Python rechnet mit jedem `int` weiter und scheitert erst in `deklariere`
    /// (`null_unzulaessig`: 0, unter 2024, ueber 2100 → `ValueError`, 500); 2027–2100 laufen dort
    /// mit der Vereinigungsmenge weiter. [`Vz`] kennt nur 2024–2026.
    #[error("kein unterstuetzter Veranlagungszeitraum: {0}")]
    Veranlagungszeitraum(i64),
    /// Ring-Werte oder Guard: die Python-Ausnahme an derselben Stelle, 500.
    #[error(transparent)]
    Bescheid(#[from] BescheidFehler),
    /// 409, `grund` ist der Sperrgrund.
    #[error("Sperrgrund {0}")]
    Gesperrt(Sperrgrund),
    /// `EM.deklariere` wirft, 500.
    #[error(transparent)]
    Deklaration(#[from] DeklarationsFehler),
    /// 409 `deklaration_unvollstaendig` mit `unvollstaendig`.
    #[error("Deklaration unvollstaendig: {} Eintraege", .0.len())]
    DeklarationUnvollstaendig(Vec<Eintrag>),
    /// 422 `xml_nicht_baubar`, `detail` ist der Text.
    #[error(transparent)]
    XmlNichtBaubar(#[from] XmlFehler),
}

/// `api.einreichen` bis zum XML, in Pythons Reihenfolge:
///
/// 1. Scheibe (`_cfg`) und ihre Bindung (`_scheibe_bindung`), dann `materialisiere`,
/// 2. Stammdaten: nur eine Scheibe, deren `felder` alle `STAMMDATEN_FELDER` fuehren, traegt eine
///    Erklaerung,
/// 3. [`mit_ring_werten`], dann der K2-Guard [`an_gesamt_sperrgrund`], wenn `cfg.guard()`,
/// 4. `deklariere`; vorlaeufige oder widerspruechliche Eingaben halten an,
/// 5. `erzeuge_xml` abgabefaehig, mit [`TESTMERKER_ERIC`] und den Feldern fuer die Steuernummer.
///
/// `empfaenger_land`: `body["empfaenger_land"] or "BY"` liefert der Handler. `hersteller_id`:
/// `None` liest `$ELSTER_HERSTELLER_ID` wie Python.
///
/// # Errors
/// [`EinreichFehler`], je Variante mit der Python-Antwort.
///
/// ```
/// use bescheid::deklaration::{einreichungs_xml, EinreichFehler};
/// use bescheid::testhilfe::{index, params};
/// let datei = serde_json::from_value(serde_json::json!(
///     {"version": 1, "veranlagungszeitraum": 2025, "scheibe": "an_gesamt", "events": []}
/// )).unwrap();
/// let st = store::Store::aus_datei(datei);
/// // an_gesamt ist eine Teilrechnung: kein Stammdatenfeld liegt in ihrem Kegel.
/// match einreichungs_xml(&st, index(), params(), "BY", None) {
///     Err(EinreichFehler::ScheibeNichtAbgabefaehig { fehlende_stammdatenfelder, .. }) => {
///         assert_eq!(fehlende_stammdatenfelder.len(), 12);
///     }
///     anders => panic!("{anders:?}"),
/// }
/// ```
pub fn einreichungs_xml(
    store: &Store,
    index: &BindungIndex<'_>,
    params: &Params,
    empfaenger_land: &str,
    hersteller_id: Option<String>,
) -> Result<Einreichung, EinreichFehler> {
    let cfg = Cfg::der_akte(store)?;
    // PARITÄT: `n_vor_gwg` traegt `felder=None`; Python liest die Liste aus der YAML, hier bleibt
    // die Bindung leer. Die Stammdaten-Pruefung weist die Scheibe in beiden Welten ab
    // (`cfg.get("felder") or ()`), bevor jemand die Bindung liest.
    let scheiben_felder = cfg.felder_roh().unwrap_or_default();
    let bindung = scheibe_bindung(scheiben_felder, index)?;
    let (mut felder, sid) = store.materialisiere(None)?;
    let fehlende_stammdatenfelder: Vec<&'static str> = STAMMDATEN_FELDER
        .iter()
        .copied()
        .filter(|f| !scheiben_felder.contains(f))
        .collect();
    if !fehlende_stammdatenfelder.is_empty() {
        return Err(EinreichFehler::ScheibeNichtAbgabefaehig {
            scheibe: cfg.scheibe(),
            fehlende_stammdatenfelder,
        });
    }
    let jahr = store.veranlagungszeitraum();
    let vz = u16::try_from(jahr)
        .ok()
        .and_then(|j| Vz::try_from(j).ok())
        .ok_or(EinreichFehler::Veranlagungszeitraum(jahr))?;
    mit_ring_werten(&mut felder, Some(vz), params)?;
    if cfg.guard() {
        // Der Guard sieht Roh-Felder; `nur_bestaetigt` liest er nicht.
        let q = Instanzquelle {
            store: Some(store),
            bindung: Some(&bindung),
            nur_bestaetigt: false,
        };
        if let Some(grund) = an_gesamt_sperrgrund(&felder, Some(&cfg), Some(vz), &q)? {
            return Err(EinreichFehler::Gesperrt(grund));
        }
    }
    // Das Jahr kommt aus dem FALL, nie als stiller Vorgabewert: `store.veranlagungszeitraum()`.
    // `deklariere` prueft es erneut (`null_unzulaessig`, `est_mapping.py:198-229`) und weist es
    // ab, wenn es fehlt, 0 oder unplausibel ist — dieselbe Stelle wie `EM.deklariere(vz=...)`.
    let deklaration = deklariere(
        &felder,
        &bindung,
        i64::from(vz.jahr()),
        Some(&sid.to_string()),
    )?;
    if !deklaration.eingaben_konsistent() {
        return Err(EinreichFehler::DeklarationUnvollstaendig(
            deklaration.unvollstaendig().to_vec(),
        ));
    }
    let xml = erzeuge_xml(
        &deklaration,
        &XmlOptionen {
            vz: i64::from(vz.jahr()),
            empfaenger_land: empfaenger_land.to_owned(),
            hersteller_id,
            testmerker: Some(TESTMERKER_ERIC.to_owned()),
            abgabefaehig: true,
            snapshot: Some(&felder),
            ..XmlOptionen::default()
        },
    )?;
    Ok(Einreichung {
        vz,
        deklaration,
        xml,
    })
}
