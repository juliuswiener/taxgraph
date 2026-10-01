//! Werbungskosten Anlage N (§ 9): der Eingabe-Aufbau von `catala_werbungskosten_n`, den
//! `_zweig_festzusetzende_est` (an_gesamt) und `_zweig_festzusetzende_est_gesamt` gemeinsam haben
//! (`bescheid_zweige.py:227-297` und `:512-590`). Python baut ein `wk_input`-dict, dessen
//! Schluessel-ANWESENHEIT die Zweige schaltet; hier ist jede Anwesenheit ein `Some`.
use domain::{Cent, Euro, PyWert, Vz};
use engine::zugriff::teil1::afa::{p7_linear_afa, P7LinearAfaEingabe};
use engine::zugriff::teil1::reisekosten::{DhfEingabe, UebernachtungEingabe, VerpflegungEingabe};
use engine::zugriff::teil1::werbungskosten::EntfernungspauschaleEingabe;
use intervall::Slots;
use rust_decimal::Decimal;

use super::rechnen::R;
use super::slot;
use crate::abzuege::oepnv_eur;
use crate::{
    cent_zu_euro, feld_int_oder_null, ist_true, py_int, summe, wert, BescheidFehler, Felder,
};

const DHF_KOSTEN: &str = "dhf_unterkunftskosten_monat";
const DHF_BEDINGUNGEN: [&str; 3] = [
    "dhf_beruflich_veranlasst",
    "dhf_eigener_hausstand",
    "dhf_finanzielle_beteiligung",
];
const UEBERNACHTUNG_KOSTEN: &str = "uebernachtung_kosten_monat";
const UEBERNACHTUNG_BEDINGUNGEN: [&str; 3] = [
    "uebernachtung_auswaerts",
    "uebernachtung_alleinnutzung",
    "uebernachtung_keine_lange_unterbrechung",
];
const VERPFLEGUNG_TAGE: [&str; 3] = ["tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig"];
const VERPFLEGUNG_TAGE_NACH_FRIST: [&str; 3] = [
    "vpf_tage_24h_nach_drei_monaten",
    "vpf_tage_an_abreise_nach_drei_monaten",
    "vpf_tage_ueber_8h_nach_drei_monaten",
];
pub(super) const ARBEITSMITTEL_KOSTEN: &str = "am_anschaffungskosten";
/// GWG-Grenze 800 EUR in CENT (`bescheid_zweige.py:293`).
const GWG_GRENZE_CENT: i64 = 80_000;

/// Teile des `wk_input`, die in beiden Zweigen gleich gebaut werden.
pub(super) struct WkTeile {
    pub doppelte_haushaltsfuehrung: Option<DhfEingabe>,
    pub verpflegung: Option<VerpflegungEingabe>,
    pub uebernachtung: Option<UebernachtungEingabe>,
}

/// `Decimal(str(x))` fuer `entfernung_km_roh`.
///
/// K2: `str(x)` ist [`PyWert::py_str`] — fuer Zahlen dasselbe wie `Value::Number::to_string`,
/// fuer alles andere der `repr` (und damit wie bisher ein `ValueError`).
fn dezimal(v: &PyWert) -> R<Decimal> {
    let fehler = || BescheidFehler::Python {
        klasse: "ValueError",
        was: "Decimal(str(entfernung_km_roh))",
    };
    match v {
        PyWert::Text(t) => Decimal::from_str_exact(t.trim()).map_err(|_| fehler()),
        w => {
            let t = w.py_str();
            Decimal::from_str_exact(&t)
                .or_else(|_| Decimal::from_scientific(&t))
                .map_err(|_| fehler())
        }
    }
}

/// Die vier Pflicht-Slots der Entfernungspauschale (`slots[k] for k in (...)`, KeyError bei Luecke).
///
/// # Errors
/// [`BescheidFehler::SlotFehlt`] und Konvertierungsfehler.
pub(super) fn ep_eingabe(vz: Vz, slots: &Slots) -> R<EntfernungspauschaleEingabe> {
    let arbeitstage = slot(slots, "arbeitstage")?;
    let km = slot(slots, "entfernung_km_roh")?;
    slot(slots, "oepnv_kosten_jahr")?;
    let kfz = slot(slots, "eigenes_oder_ueberlassenes_kfz")?;
    // Python-Reihenfolge der Konvertierungen: `_oepnv_eur` beim Aufbau des dicts, danach im
    // Accessor `Decimal(str(km))` und `int(arbeitstage)` — der ERSTE Fehler bestimmt die Klasse.
    let oepnv = oepnv_eur(slots)?;
    let entfernung = dezimal(km)?;
    Ok(EntfernungspauschaleEingabe {
        veranlagungszeitraum: vz,
        entfernung_km_roh: entfernung,
        arbeitstage: py_int(arbeitstage)?,
        eigenes_oder_ueberlassenes_kfz: kfz.truthy(),
        oepnv_kosten_jahr: oepnv,
    })
}

fn alle_true(f: &Felder, ids: &[&str]) -> bool {
    ids.iter().all(|b| ist_true(wert(f, b)))
}

/// Python `isinstance(v, int) and not isinstance(v, bool) and v > 3`.
///
/// ponytail: `GrossGanz` antwortet `true` statt zu melden (D3) — der Vergleich ist die einzige
/// Nutzung des Werts, und `u64` ist immer > 3. Upgrade auf `Result`, falls hier je gerechnet wird.
fn ist_int_ueber_3(v: Option<&PyWert>) -> bool {
    match v {
        Some(PyWert::Ganz(n)) => *n > 3,
        Some(PyWert::GrossGanz(_)) => true,
        _ => false,
    }
}

/// dHf, Verpflegung und Uebernachtung nach den Tatbestands-Bedingungen der Zweige.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`].
pub(super) fn wk_teile(f: &Felder, vz: Vz) -> R<WkTeile> {
    let c = |k: &str| feld_int_oder_null(f, k);
    // doppelte Haushaltsfuehrung: Kosten > 0, Inland, alle 3 Bedingungen bestaetigt-true.
    let dhf = if c(DHF_KOSTEN)? > 0
        && ist_true(wert(f, "dhf_im_inland"))
        && alle_true(f, &DHF_BEDINGUNGEN)
    {
        Some(DhfEingabe {
            veranlagungszeitraum: vz,
            unterkunftskosten_monat: cent_zu_euro(c(DHF_KOSTEN)?),
            monate: c("dhf_monate")?,
            im_inland: true,
        })
    } else {
        None
    };
    // Verpflegung: nur bei Tagen > 0; NACH_FRIST-Tage nur bei > 3 Monaten am Ort.
    let mut tage = [0_i64; 3];
    for (t, id) in tage.iter_mut().zip(VERPFLEGUNG_TAGE) {
        *t = c(id)?;
    }
    let vpf = if summe(&tage)? > 0 {
        let mut nach = [0_i64; 3];
        if ist_int_ueber_3(wert(f, "vpf_monate_am_ort")) {
            for (t, id) in nach.iter_mut().zip(VERPFLEGUNG_TAGE_NACH_FRIST) {
                *t = c(id)?;
            }
        }
        // Mahlzeiten/Entgelt/Erstattung: nur wenn > 0 gesetzt, sonst absent (= 0 im Accessor).
        let positiv = |k: &str| -> R<i64> { Ok(c(k)?.max(0)) };
        Some(VerpflegungEingabe {
            veranlagungszeitraum: vz,
            tage_24h: tage[0],
            tage_an_abreise: tage[1],
            tage_ueber_8h_eintaegig: tage[2],
            vpf_tage_24h_nach_drei_monaten: nach[0],
            vpf_tage_an_abreise_nach_drei_monaten: nach[1],
            vpf_tage_ueber_8h_nach_drei_monaten: nach[2],
            vpf_fruehstuecke_gestellt_anzahl: positiv("vpf_fruehstuecke_gestellt_anzahl")?,
            vpf_mittagessen_gestellt_anzahl: positiv("vpf_mittagessen_gestellt_anzahl")?,
            vpf_abendessen_gestellt_anzahl: positiv("vpf_abendessen_gestellt_anzahl")?,
            vpf_mahlzeiten_gezahltes_entgelt: Cent::new(positiv(
                "vpf_mahlzeiten_gezahltes_entgelt",
            )?),
            vpf_steuerfreie_erstattung_betrag: Cent::new(positiv(
                "vpf_steuerfreie_erstattung_betrag",
            )?),
        })
    } else {
        None
    };
    // Uebernachtung: Kosten > 0, Ort ist ein Bool, alle 3 Bedingungen bestaetigt-true.
    let bisher = c("uebernachtung_monate_bisher")?;
    let monate = c("uebernachtung_monate")?;
    let ueb = match wert(f, "uebernachtung_im_inland") {
        Some(PyWert::Bool(inland))
            if c(UEBERNACHTUNG_KOSTEN)? > 0 && alle_true(f, &UEBERNACHTUNG_BEDINGUNGEN) =>
        {
            Some(UebernachtungEingabe {
                veranlagungszeitraum: vz,
                uebernachtung_kosten_monat: cent_zu_euro(c(UEBERNACHTUNG_KOSTEN)?),
                uebernachtung_monate: monate,
                uebernachtung_monate_bisher: bisher,
                uebernachtung_im_inland: *inland,
            })
        }
        _ => None,
    };
    Ok(WkTeile {
        doppelte_haushaltsfuehrung: dhf,
        verpflegung: vpf,
        uebernachtung: ueb,
    })
}

/// Arbeitsmittel-GWG (AK ≤ 800 EUR, Wahlrecht ausgeuebt): AK in EURO.
fn am_gwg(f: &Felder) -> R<Option<Euro>> {
    let k = feld_int_oder_null(f, ARBEITSMITTEL_KOSTEN)?;
    Ok(
        (0 < k && k <= GWG_GRENZE_CENT && ist_true(wert(f, "am_gwg_sofortabzug_gewaehlt")))
            .then(|| cent_zu_euro(k)),
    )
}

/// § 7 Abs. 1 lineare AfA fuer AK > 800 EUR. `ak_als_cent` waehlt die Schluessel-Variante: an_gesamt
/// uebergibt `anschaffungskosten_cent`, gesamt `anschaffungskosten` (EURO). `None` = kein Abzug.
fn am_afa(f: &Felder, ak_als_cent: bool) -> R<Option<Euro>> {
    let k = feld_int_oder_null(f, ARBEITSMITTEL_KOSTEN)?;
    if k <= GWG_GRENZE_CENT {
        return Ok(None);
    }
    let nd = feld_int_oder_null(f, "arbeitsmittel_nutzungsdauer")?;
    if nd <= 0 {
        return Ok(None);
    }
    let ist_aj = ist_true(wert(f, "am_afa_ist_anschaffungsjahr"));
    let monat = if ist_aj {
        // Anschaffungsjahr: fehlt der Monat, wird NICHTS abgezogen (fail-closed).
        let m = feld_int_oder_null(f, "am_anschaffung_monat")?;
        if m <= 0 {
            return Ok(None);
        }
        m
    } else {
        0
    };
    let (ak_cent, ak_euro) = if ak_als_cent {
        (Cent::new(k), Euro::new(0))
    } else {
        (Cent::new(0), cent_zu_euro(k))
    };
    Ok(Some(p7_linear_afa(&P7LinearAfaEingabe {
        anschaffungskosten_cent: ak_cent,
        anschaffungskosten: ak_euro,
        nutzungsdauer: nd,
        anschaffung_monat: monat,
        ist_anschaffungsjahr: ist_aj,
    })?))
}

/// an_gesamt: GWG-Sofortabzug ODER (`elif`) die AfA — beides landet in `am_anschaffungskosten`.
///
/// PARITÄT: die AfA laeuft in Python durch `catala_p6_2_gwg` (wie ein GWG-Preis); ueber 800 EUR
/// liefert das defensiv 0.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
pub(super) fn am_an_gesamt(f: &Felder) -> R<Option<Euro>> {
    match am_gwg(f)? {
        Some(gwg) => Ok(Some(gwg)),
        None => am_afa(f, true),
    }
}

/// gesamt: GWG-Sofortabzug in `am_anschaffungskosten`, die AfA als eigener Summand danach.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
pub(super) fn am_gesamt(f: &Felder) -> R<(Option<Euro>, Euro)> {
    let afa = am_afa(f, false)?.unwrap_or(Euro::new(0));
    Ok((am_gwg(f)?, afa))
}

/// Aequivalenz von `ist_int_ueber_3` mit `PyWert::int_ohne_bool` (D15), je D-Nummer ein Test.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{d3, json_wert, klasse, pruefe, py, Ergebnis};
    use proptest::prelude::*;
    use rust_decimal::Decimal;
    use serde_json::json;

    use super::{dezimal, ist_int_ueber_3};
    use crate::aequivalenz::alt_klasse;
    use crate::vor_k2::{dezimal_alt, ist_int_ueber_3_alt};

    /// Ausnahmen von `ist_int_ueber_3`.
    const UEBER_3: &[&str] = &["D3"];

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        /// Die Alt-Fassung gegen `CPython` — die Messung, die D3 festhaelt (Auflage 1).
        #[test]
        fn ist_int_ueber_3_alt_wie_pywert(v in json_wert()) {
            let neu = klasse(py(&v).int_ohne_bool()).map(|i| i.is_some_and(|i| i > 3));
            pruefe(&v, &Ok(ist_int_ueber_3_alt(Some(&v))), &neu, || d3(&v, &neu), UEBER_3)?;
        }

        /// Die Produktion gegen die Alt-Fassung: beide saettigen an derselben Stelle.
        #[test]
        fn ist_int_ueber_3_wie_alt(v in json_wert()) {
            let w = py(&v);
            let (alt, neu): (Ergebnis<bool>, Ergebnis<bool>) =
                (Ok(ist_int_ueber_3_alt(Some(&v))), Ok(ist_int_ueber_3(Some(&w))));
            pruefe(&v, &alt, &neu, Vec::new, &[])?;
        }

        /// `Decimal(str(x))`: die Alt-Fassung nimmt `Value::Number::to_string`, die Produktion
        /// `PyWert::py_str`. Beide sind `str(x)` in `CPython` — kein Float-Formatierungsunterschied
        /// (dafuer traegt `repr_float` seine eigene Messung), also ohne Ausnahmen.
        #[test]
        fn dezimal_wie_alt(v in json_wert()) {
            let (alt, neu): (Ergebnis<Decimal>, Ergebnis<Decimal>) =
                (alt_klasse(dezimal_alt(&v)), alt_klasse(dezimal(&py(&v))));
            pruefe(&v, &alt, &neu, Vec::new, &[])?;
        }
    }

    /// D3: `2**64 - 1 > 3` ist in `CPython` wahr. Der Alt-Helfer antwortet `true`, `PyWert` meldet
    /// die i64-Grenze.
    #[test]
    fn d3_ueber_i64() {
        let v = json!(u64::MAX);
        assert!(ist_int_ueber_3_alt(Some(&v)));
        assert!(ist_int_ueber_3(Some(&py(&v))));
        assert_eq!(klasse(py(&v).int_ohne_bool()), Err(None));
    }
}
