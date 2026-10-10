//! Die Rechenzweige (Schritt 7c): `produkt/bescheid/bescheid_zweige.py` — der Dispatcher
//! [`bescheid_fn`] und die vier Rechenkerne `abziehbarer_betrag`, `festzusetzende_est` (an_gesamt),
//! `festzusetzende_est_gesamt`, `festzusetzende_est_rentner`.
//!
//! **Typestate (`REWRITE_PLAN.md` §4).** Python filtert bei `nur_bestaetigt=True` EINMAL im Kopf von
//! `_bescheid_fn` (`bescheid_zweige.py:1508`). Hier ist das der Uebergang
//! `Snapshot<Roh>` → [`snapshot::Snapshot::nur_bestaetigt`] → `Snapshot<Bestaetigt>`; jeder Zweig
//! ist ueber die [`snapshot::Marke`] generisch, der Instanz-Filter folgt aus dem Typ. Der
//! Estimate-Pfad (`nur_bestaetigt=False`) rechnet mit `Snapshot<Roh>`; ein Aufrufer kann Roh nicht in
//! den festgesetzten Pfad geben, weil [`bescheid_fn`] die Auswahl aus dem Bool trifft.
//!
//! **Out-Parameter.** Python `solz_container[0]` ist eine `Cell<Option<Cent>>`, `extras` ein
//! `RefCell<`[`ausgaben::Extras`]`>`; ein spaeterer Lauf ueberschreibt wie beim dict.
//!
//! **Umgebung.** Python liest `params/` und die Bindung ueber Modul-Globale; hier reicht der Aufrufer
//! beides in [`Umgebung`].
#![allow(clippy::many_single_char_names, clippy::doc_markdown)] // f/p/q/g/r/s: Python-Namen (1:1-Portierung); Fachbegriffe (SolZ, KiSt, EStG) in der Doku

use std::cell::{Cell, RefCell};

use bindung::Params;
use domain::{Cent, Euro, Km, Lage, PyWert, Veranlagung, Vz};
use engine::tarif::Veranlagung as TarifVeranlagung;
use engine::zugriff::teil1::werbungskosten::{entfernungspauschale, EntfernungspauschaleEingabe};
use intervall::{bescheid_via_slots, AchsenBindung, SlotFehler, Slots, Werte};
use rust_decimal::Decimal;
use store::Store;

mod an_gesamt;
pub mod ausgaben;
mod gesamt;
mod gesamt_tarif;
pub(crate) mod kinderfreibetrag;
mod rechnen;
mod rentner;
mod rentner_tarif;
pub mod snapshot;
mod tarif;
mod wk;

pub use ausgaben::{
    abschlusszahlung_cent, kette_p31, kist_konfession, setze_kette, Extras, Kette, P31Hinweis,
    P31Sieger,
};
pub use snapshot::{Bestaetigt, Marke, Roh, Snapshot};

pub(crate) use self::gesamt::{netto_vg, versorgung_ueber_lohn, versorgung_zeilen};
use self::rechnen::R;
pub(crate) use self::tarif::leerer_gesamtfall;
use crate::abzuege::oepnv_eur;
use crate::{py_int, BescheidFehler, BindungIndex, Felder, Instanzquelle};

/// Die vier Quantitaeten, die `_bescheid_fn` rechnet (Python: String-Vergleich, alles andere `None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantitaet {
    /// § 9 Entfernungspauschale.
    AbziehbarerBetrag,
    /// § 2 Gesamtsteuer, reiner AN-Fall (Scheibe `an_gesamt`).
    FestzusetzendeEst,
    /// § 21 V+V ueber `catala_gesamt`, der breiteste Rechenweg.
    FestzusetzendeEstGesamt,
    /// § 22 Renten + § 33b.
    FestzusetzendeEstRentner,
}

impl Quantitaet {
    /// Der Python-Name; unbekannt (auch die entfernten `..._haushalt`/`..._agb`) → `None`.
    ///
    /// ```
    /// use bescheid::zweige::Quantitaet;
    /// assert_eq!(Quantitaet::aus_name("festzusetzende_est_gesamt"), Some(Quantitaet::FestzusetzendeEstGesamt));
    /// assert_eq!(Quantitaet::aus_name("festzusetzende_est_haushalt"), None);
    /// ```
    #[must_use]
    pub fn aus_name(name: &str) -> Option<Self> {
        match name {
            "abziehbarer_betrag" => Some(Self::AbziehbarerBetrag),
            "festzusetzende_est" => Some(Self::FestzusetzendeEst),
            "festzusetzende_est_gesamt" => Some(Self::FestzusetzendeEstGesamt),
            "festzusetzende_est_rentner" => Some(Self::FestzusetzendeEstRentner),
            _ => None,
        }
    }
}

/// Alles, was Python ueber Modul-Globale liest: die Achsen-Sicht der Bindung (`bescheid_via_slots`),
/// den Bindungs-Index (Instanz-Enumeration) und die Jahreswerte.
#[derive(Clone, Copy)]
pub struct Umgebung<'a> {
    pub achsen: &'a [AchsenBindung],
    pub index: &'a BindungIndex<'a>,
    pub params: &'a Params,
}

/// `bescheid_fn(feld_werte) -> cent` (Naht-Einheit CENT).
pub type BescheidFn<'a> = Box<dyn Fn(&Werte) -> Result<Cent, SlotFehler<BescheidFehler>> + 'a>;

/// Python `_bescheid_fn(quantitaet, vz, bindung, felder, store, nur_bestaetigt, solz_container, extras)`.
///
/// `None`, wenn `quantitaet` unbekannt ist ("kein exponierter Accessor → ehrlich None").
/// `felder` ist der materialisierte Snapshot; `nur_bestaetigt` waehlt `Snapshot<Bestaetigt>`
/// (festgesetzte Steuer, Default in Python) oder `Snapshot<Roh>` (Estimate-Pfad).
///
/// ```
/// use bescheid::testhilfe::{index, params};
/// use bescheid::zweige::{bescheid_fn, Umgebung};
/// use domain::Vz;
/// let umg = Umgebung { achsen: &[], index: index(), params: params() };
/// assert!(bescheid_fn("kein_accessor", Vz::Vz2025, &umg, None, None, true, None, None).is_none());
/// assert!(bescheid_fn("festzusetzende_est", Vz::Vz2025, &umg, None, None, true, None, None).is_some());
/// ```
#[must_use]
#[allow(clippy::too_many_arguments)] // Python-Signatur 1:1
pub fn bescheid_fn<'a>(
    quantitaet: &str,
    vz: Vz,
    umgebung: &'a Umgebung<'a>,
    felder: Option<&Felder>,
    store: Option<&'a Store>,
    nur_bestaetigt: bool,
    solz_container: Option<&'a Cell<Option<Cent>>>,
    extras: Option<&'a RefCell<Extras>>,
) -> Option<BescheidFn<'a>> {
    let q = Quantitaet::aus_name(quantitaet)?;
    let roh = Snapshot::roh(felder.cloned().unwrap_or_default());
    let ring = Ausgang {
        vz,
        umgebung,
        store,
        solz: solz_container,
        extras,
    };
    Some(if nur_bestaetigt {
        baue(q, ring, roh.nur_bestaetigt())
    } else {
        baue(q, ring, roh)
    })
}

/// Die Parameter, die nicht vom Snapshot-Typ abhaengen.
struct Ausgang<'a> {
    vz: Vz,
    umgebung: &'a Umgebung<'a>,
    store: Option<&'a Store>,
    solz: Option<&'a Cell<Option<Cent>>>,
    extras: Option<&'a RefCell<Extras>>,
}

/// Ein Zweig-Lauf: Snapshot + Umgebung.
struct Ring<'a, Z: Marke> {
    a: Ausgang<'a>,
    snap: Snapshot<Z>,
}

impl<'a, Z: Marke> Ring<'a, Z> {
    fn vz(&self) -> Vz {
        self.a.vz
    }

    fn p(&self) -> &'a Params {
        self.a.umgebung.params
    }

    /// Python `f = felder or {}`.
    fn f(&self) -> &Felder {
        self.snap.felder()
    }

    /// `store`, `bindung`, `nur_bestaetigt` der Instanz-Summen — der Filter folgt dem Snapshot-Typ.
    fn q(&self) -> Instanzquelle<'a> {
        Instanzquelle {
            store: self.a.store,
            bindung: Some(self.a.umgebung.index),
            nur_bestaetigt: Z::NUR_BESTAETIGT,
        }
    }
}

fn baue<'a, Z: Marke + 'a>(q: Quantitaet, a: Ausgang<'a>, snap: Snapshot<Z>) -> BescheidFn<'a> {
    let achsen = a.umgebung.achsen;
    let ring = Ring { a, snap };
    match q {
        Quantitaet::AbziehbarerBetrag => Box::new(bescheid_via_slots(achsen, move |s| {
            abziehbarer_betrag(&ring, s)
        })),
        Quantitaet::FestzusetzendeEst => Box::new(bescheid_via_slots(achsen, move |s| {
            an_gesamt::festzusetzende_est(&ring, s)
        })),
        Quantitaet::FestzusetzendeEstGesamt => Box::new(bescheid_via_slots(achsen, move |s| {
            gesamt::festzusetzende_est_gesamt(&ring, s)
        })),
        Quantitaet::FestzusetzendeEstRentner => Box::new(bescheid_via_slots(achsen, move |s| {
            rentner::festzusetzende_est_rentner(&ring, s)
        })),
    }
}

/// `slots[k]` (`KeyError`, wenn der Slot fehlt).
fn slot<'s>(slots: &'s Slots, k: &str) -> R<&'s PyWert> {
    slots
        .get(k)
        .ok_or_else(|| BescheidFehler::SlotFehlt(k.to_owned()))
}

/// § 9 Entfernungspauschale (`_zweig_abziehbarer_betrag`), EURO. `entfernung_km_roh` geht als
/// `int(...)` hinein (Nachkommastellen fallen weg), anders als in den Gesamt-Zweigen.
fn abziehbarer_betrag<Z: Marke>(r: &Ring<'_, Z>, slots: &Slots) -> R<Euro> {
    let arbeitstage = py_int(slot(slots, "arbeitstage")?)?;
    let km = py_int(slot(slots, "entfernung_km_roh")?)?;
    let oepnv = oepnv_eur(slots)?;
    let kfz = slot(slots, "eigenes_oder_ueberlassenes_kfz")?.truthy();
    Ok(entfernungspauschale(
        &EntfernungspauschaleEingabe {
            veranlagungszeitraum: r.vz(),
            entfernung_km_roh: Km::new(Decimal::from(km)),
            arbeitstage,
            eigenes_oder_ueberlassenes_kfz: kfz,
            oepnv_kosten_jahr: oepnv,
        },
        r.p(),
    )?)
}

/// Der Wert eines Veranlagungs-Slots/-Felds. Python vergleicht `== "zusammen"`; nur die
/// Tarif-Zweige (`catala_est`, `catala_fuenftel`) kennen zusaetzlich `ValueError` fuer alles ausser
/// "einzel"/"zusammen".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VeranlagungWert {
    Einzel,
    Zusammen,
    /// Weder "einzel" noch "zusammen" (im Store durch den Enum-Typ der Bindung ausgeschlossen).
    Unbekannt,
}

impl VeranlagungWert {
    /// `Fehlt`, `Null` und `Abweichend` sind `Unbekannt`, wie vor K7a der `_`-Arm.
    fn aus(l: Lage<'_, Veranlagung>) -> Self {
        match l {
            Lage::Gueltig(Veranlagung::Einzel) => Self::Einzel,
            Lage::Gueltig(Veranlagung::Zusammen) => Self::Zusammen,
            Lage::Fehlt | Lage::Null | Lage::Abweichend(_) => Self::Unbekannt,
        }
    }

    /// Python `_b("veranlagung") or "einzel"`: ein falsy Wert ist "einzel".
    fn aus_oder_einzel(l: Lage<'_, Veranlagung>) -> Self {
        match l {
            Lage::Fehlt | Lage::Null => Self::Einzel,
            Lage::Abweichend(w) if !w.truthy() => Self::Einzel,
            Lage::Gueltig(_) | Lage::Abweichend(_) => Self::aus(l),
        }
    }

    pub(crate) fn zusammen(self) -> bool {
        self == Self::Zusammen
    }

    /// Nicht-strenge Sicht: alles ausser "zusammen" laeuft als Einzelveranlagung.
    pub(crate) fn domain(self) -> Veranlagung {
        if self.zusammen() {
            Veranlagung::Zusammen
        } else {
            Veranlagung::Einzel
        }
    }

    /// Strenge Sicht der Tarif-Accessoren: `ValueError("unknown veranlagung")`.
    pub(crate) fn tarif_strikt(self) -> R<TarifVeranlagung> {
        match self {
            Self::Einzel => Ok(TarifVeranlagung::Einzel),
            Self::Zusammen => Ok(TarifVeranlagung::Zusammen),
            Self::Unbekannt => Err(BescheidFehler::Python {
                klasse: "ValueError",
                was: "unknown veranlagung",
            }),
        }
    }
}

/// K7a: `VeranlagungWert` liest ueber `Lage`. Die Alt-Fassungen verglichen `PyWert` direkt und
/// fingen den Rest mit `_ =>`; `Fehlt`, `Null` und `Abweichend` muessen dasselbe liefern.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{pruefe, py};
    use domain::{Lage, PyWert};
    use proptest::prelude::*;

    use super::VeranlagungWert;
    use crate::aequivalenz::veranlagung_json;

    fn aus_alt(v: Option<&PyWert>) -> VeranlagungWert {
        match v {
            Some(PyWert::Text(s)) if s == "zusammen" => VeranlagungWert::Zusammen,
            Some(PyWert::Text(s)) if s == "einzel" => VeranlagungWert::Einzel,
            _ => VeranlagungWert::Unbekannt,
        }
    }

    fn aus_oder_einzel_alt(v: Option<&PyWert>) -> VeranlagungWert {
        match v {
            Some(w) if w.truthy() => aus_alt(v),
            _ => VeranlagungWert::Einzel,
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn veranlagung_wie_alt(v in veranlagung_json()) {
            let w = v.as_ref().map(py);
            let l = Lage::veranlagung(w.as_ref());
            pruefe(&v, &aus_alt(w.as_ref()), &VeranlagungWert::aus(l), Vec::new, &[])?;
            let (alt, neu) = (aus_oder_einzel_alt(w.as_ref()), VeranlagungWert::aus_oder_einzel(l));
            pruefe(&v, &alt, &neu, Vec::new, &[])?;
        }
    }
}
