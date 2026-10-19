//! Werbungskosten Anlage N (§ 9): der Eingabe-Aufbau von `catala_werbungskosten_n`, den
//! `_zweig_festzusetzende_est` (an_gesamt) und `_zweig_festzusetzende_est_gesamt` gemeinsam haben
//! (`bescheid_zweige.py:227-297` und `:512-590`). Python baut ein `wk_input`-dict, dessen
//! Schluessel-ANWESENHEIT die Zweige schaltet; hier ist jede Anwesenheit ein `Some`.
use domain::{Cent, Euro, Km, PyWert, Vz};
use engine::zugriff::teil1::afa::{p7_linear_afa, P7LinearAfaEingabe};
use engine::zugriff::teil1::reisekosten::{DhfEingabe, UebernachtungEingabe, VerpflegungEingabe};
use engine::zugriff::teil1::werbungskosten::EntfernungspauschaleEingabe;
use intervall::Slots;
use rust_decimal::Decimal;

use super::rechnen::{add, R};
use super::slot;
use crate::abzuege::oepnv_eur;
use crate::arbeitsmittel_grenze::{
    computer_ja, computer_weg, satz_prozent, sofortabzug, ComputerWeg, Sofortabzug, GWG_GRENZE_CENT,
};
use crate::{
    cent_zu_euro, feld_euro_oder_null, feld_int_oder_null, ist_true, py_int, summe, wert,
    BescheidFehler, Felder,
};

const UNFALLKOSTEN: &str = "ep_unfallkosten";
/// Beitraege zu Berufsverbaenden je Person (Abweichung Nr. 93). Die Namen stehen als Literale: der Scanner hinter `RING_BETRAGSFELDER`
/// (`bescheid/tests/ring_scheiben.rs`) liest den Quelltext des Rings.
const BERUFSVERBAND: &str = "berufsverband_beitraege";
const BERUFSVERBAND_PARTNER: &str = "berufsverband_beitraege_partner";
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
/// Der Umsatzsteuersatz der Rechnung (Abweichung Nr. 88, [`crate::arbeitsmittel_grenze`]).
const ARBEITSMITTEL_SATZ: &str = "am_umsatzsteuersatz";
/// Die Antwort „steht auf der Liste im BMF-Schreiben vom 22.02.2022“ (Abweichung Nr. 89, [`crate::arbeitsmittel_grenze`]).
const ARBEITSMITTEL_LISTE: &str = "am_computer_software";

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
fn dezimal(v: &PyWert) -> R<Km> {
    let fehler = || BescheidFehler::Python {
        klasse: "ValueError",
        was: "Decimal(str(entfernung_km_roh))",
    };
    match v {
        PyWert::Text(t) => Decimal::from_str_exact(t.trim())
            .map(Km::new)
            .map_err(|_| fehler()),
        w => {
            let t = w.py_str();
            Decimal::from_str_exact(&t)
                .or_else(|_| Decimal::from_scientific(&t))
                .map(Km::new)
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

/// Unfallkosten auf dem Weg zur Arbeit (`ep_unfallkosten`, Abweichung Nr. 28): BMF-Schreiben vom 18.11.2021, Rz. 30 — sie
/// sind „weiterhin neben der Entfernungspauschale zu beruecksichtigen“. Sie kommen deshalb NACH `werbungskosten_n` zu den
/// Werbungskosten dazu und liegen ausserhalb des Hoechstbetrags von 4.500 EUR der Pauschale. Python kennt das Feld nicht.
///
/// Leer, 0 und negativ heissen 0: `nicht_negativ` haelt die Schreibseite, das Laden prueft nie.
///
/// ponytail: nur der Weg zwischen Wohnung und erster Taetigkeitsstaette; Familienheimfahrten und der Kilometersatz fuer
/// Menschen mit Behinderungen (selbe Rz. 30, Rz. 29) stehen nicht darin. Upgrade: je ein eigenes Feld, derselbe Aufruf.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`].
pub(super) fn mit_unfallkosten(f: &Felder, wk: Euro) -> R<Euro> {
    let unfall = feld_euro_oder_null(f, UNFALLKOSTEN)?;
    add(wk, Euro::new(unfall.get().max(0)))
}

/// Die Beitraege zu Berufsverbaenden einer Person in EURO (Abweichung Nr. 93): Gewerkschaft oder Berufsverband, § 9 Abs. 1 Satz 3 Nr. 3
/// `EStG`, Zeile 53 der Anlage N. `partner` waehlt den Ehegatten. Der Betrag zaehlt zu den uebrigen Werbungskosten DER PERSON und steht damit
/// vor dem Vergleich mit dem Arbeitnehmer-Pauschbetrag (je Person), nicht daneben. Die Erklaerung traegt ihn in vollen Euro AUFGERUNDET
/// (`E0204003`, `ABZUGS_KZ`); der Bescheid rechnet mit demselben Wert.
///
/// Leer, 0 und negativ heissen 0: `nicht_negativ` haelt die Schreibseite, das Laden prueft nie.
///
/// ponytail: Rentner und Pensionaeren (Werbungskosten gegen Versorgungsbezuege) und die Kammern (Arbeitskammer, `Kammerbeitrag/Wert`,
/// Rechtslage ungeklaert) stehen nicht darin. Upgrade: ein Betrag je Einkunftsart, derselbe Aufruf.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`] bei einem Feldwert jenseits von `i64`.
pub(super) fn berufsverband(f: &Felder, partner: bool) -> R<Euro> {
    let feld = if partner { BERUFSVERBAND_PARTNER } else { BERUFSVERBAND };
    Ok(Cent::new(feld_int_oder_null(f, feld)?.max(0)).ceil_euro())
}

/// [`berufsverband`] von Person A zu ihren Werbungskosten `wk` addiert (nach `werbungskosten_n`, wie [`mit_unfallkosten`]).
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`].
pub(super) fn mit_berufsverband(f: &Felder, wk: Euro) -> R<Euro> {
    add(wk, berufsverband(f, false)?)
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

/// Abweichung Nr. 89: ist das Arbeitsmittel ein Geraet der Liste im BMF-Schreiben vom 22.02.2022? Dann ist es kein `GWG`: weder der
/// Sofortabzug bis 800 EUR ([`am_gwg`]) noch der im Band ([`mit_sofortabzug_im_band`]) zaehlen, und die Netto-Pruefung entfaellt.
fn liste_ja(f: &Felder) -> bool {
    computer_ja(wert(f, ARBEITSMITTEL_LISTE))
}

/// Arbeitsmittel-GWG (AK ≤ 800 EUR, Wahlrecht ausgeuebt): AK in EURO. Ein Geraet der Liste ist keins (Abweichung Nr. 89).
fn am_gwg(f: &Felder) -> R<Option<Euro>> {
    let k = feld_int_oder_null(f, ARBEITSMITTEL_KOSTEN)?;
    Ok((0 < k
        && k <= GWG_GRENZE_CENT
        && !liste_ja(f)
        && ist_true(wert(f, "am_gwg_sofortabzug_gewaehlt")))
    .then(|| cent_zu_euro(k)))
}

/// Abweichung Nr. 88: Sofortabzug eines Arbeitsmittels, dessen BRUTTOPREIS ueber 800 EUR liegt, dessen Preis ohne Umsatzsteuer
/// die Grenze aber einhaelt (Satz beantwortet, Wahlrecht ausgeuebt): der volle Bruttopreis in EURO. Ohne beantworteten Satz
/// zaehlt nichts (nie ein stiller Satz; die Sperre `arbeitsmittel_afa_ueber_gwg_offen` zeigt es).
///
/// Er kommt NACH `werbungskosten_n` zu den Werbungskosten dazu, wie die Unfallkosten ([`mit_unfallkosten`]): der Catala-Scope
/// `p6_2_gwg` zaehlt den uebergebenen Preis gegen 800 EUR und lieferte fuer 900 EUR 0.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`].
pub(super) fn mit_sofortabzug_im_band(f: &Felder, wk: Euro) -> R<Euro> {
    let k = feld_int_oder_null(f, ARBEITSMITTEL_KOSTEN)?;
    let satz = satz_prozent(wert(f, ARBEITSMITTEL_SATZ));
    if k > GWG_GRENZE_CENT
        && !liste_ja(f)
        && sofortabzug(Decimal::from(k), satz) == Sofortabzug::Moeglich
        && ist_true(wert(f, "am_gwg_sofortabzug_gewaehlt"))
    {
        return add(wk, cent_zu_euro(k));
    }
    Ok(wk)
}

/// Abweichung Nr. 89: der Abzug eines Geraets der Liste im BMF-Schreiben vom 22.02.2022 mit der Nutzungsdauer von einem Jahr (Rz. 1):
/// der volle Bruttopreis in EURO, im Anschaffungsjahr (Rz. 1.4: kein Zwoelftel nach dem Kaufmonat), in jedem anderen Jahr 0. Eine eigene
/// Nutzungsdauer ueber ein Jahr (Rz. 1.2) laeuft ueber [`am_afa`] wie vor Nr. 89.
///
/// Er kommt NACH `werbungskosten_n` zu den Werbungskosten dazu, wie der Sofortabzug im Band ([`mit_sofortabzug_im_band`]): der
/// Catala-Scope `p6_2_gwg` zaehlt den uebergebenen Preis gegen 800 EUR und lieferte fuer 1.500 EUR 0. Ob das Anschaffungsjahr bestaetigt
/// ist, haelt die Sperre; ohne Antwort zaehlt hier das Folgejahr (0), wie bei jeder `AfA`.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`].
pub(super) fn mit_computer_im_kaufjahr(f: &Felder, wk: Euro) -> R<Euro> {
    let k = feld_int_oder_null(f, ARBEITSMITTEL_KOSTEN)?;
    let nd = feld_int_oder_null(f, "arbeitsmittel_nutzungsdauer")?;
    if k > 0
        && liste_ja(f)
        && computer_weg(nd) == ComputerWeg::EinJahr
        && ist_true(wert(f, "am_afa_ist_anschaffungsjahr"))
    {
        return add(wk, cent_zu_euro(k));
    }
    Ok(wk)
}

/// § 7 Abs. 1 lineare AfA fuer AK ueber der Grenze (Abweichung Nr. 88: 800 EUR netto, bei 19 % bis 952,00 EUR brutto, bei 7 % bis
/// 856,00 EUR; ohne beantworteten Satz im Band zaehlt die AfA wie bisher, die Sperre haelt das Ergebnis offen).
/// `ak_als_cent` waehlt die Schluessel-Variante: an_gesamt uebergibt `anschaffungskosten_cent`, gesamt
/// `anschaffungskosten` (EURO). `None` = kein Abzug.
///
/// Abweichung Nr. 89: ein Geraet der Liste ist kein `GWG`, die Netto-Pruefung entfaellt; mit der Nutzungsdauer von einem Jahr (keine
/// eigene Angabe oder 1) rechnet [`mit_computer_im_kaufjahr`] den Abzug, hier gibt es keinen; mit einer eigenen Nutzungsdauer ueber
/// ein Jahr (Rz. 1.2) laeuft die Verteilung wie sonst.
fn am_afa(f: &Felder, ak_als_cent: bool) -> R<Option<Euro>> {
    let k = feld_int_oder_null(f, ARBEITSMITTEL_KOSTEN)?;
    let liste = liste_ja(f);
    if !liste && sofortabzug(Decimal::from(k), satz_prozent(wert(f, ARBEITSMITTEL_SATZ))) == Sofortabzug::Moeglich {
        return Ok(None);
    }
    let nd = feld_int_oder_null(f, "arbeitsmittel_nutzungsdauer")?;
    if nd <= 0 || (liste && computer_weg(nd) == ComputerWeg::EinJahr) {
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

/// Abweichung Nr. 88 an den Aufrufstellen der Rechnung: wer den Sofortabzug im Band zaehlt und wer die `AfA`. Die Sperre und die
/// Ende-zu-Ende-Faelle stehen in `tests/sperre_scheiben_hermetisch.rs` und `tests/gesamt_mischfaelle_hermetisch.rs`; hier steht, was
/// sie nicht trennen: die Wahl des Sofortabzugs zaehlt in der Rechnung, auch wenn die Sperre sie fordert.
#[cfg(test)]
mod netto_grenze {
    use domain::Euro;
    use serde_json::{json, Value};

    use super::{am_gesamt, am_an_gesamt, mit_sofortabzug_im_band};
    use crate::testhilfe::{felder, store};

    /// Felder eines Arbeitsmittels: Preis in Cent, Satz (oder keiner), Wahl des Sofortabzugs, Nutzungsdauer (oder keine).
    fn am(preis: i64, satz: Option<&str>, sofort: bool, nd: Option<i64>) -> crate::Felder {
        let mut e: Vec<(&str, Value, bool)> = vec![
            ("am_anschaffungskosten", json!(preis), true),
            ("am_gwg_sofortabzug_gewaehlt", json!(sofort), true),
            ("am_afa_ist_anschaffungsjahr", json!(false), true),
        ];
        if let Some(s) = satz {
            e.push(("am_umsatzsteuersatz", json!(s), true));
        }
        if let Some(n) = nd {
            e.push(("arbeitsmittel_nutzungsdauer", json!(n), true));
        }
        felder(&store(&e))
    }

    /// Der Sofortabzug im Band kommt NACH dem Catala-Scope dazu: nur mit beantwortetem Satz, nur mit Wahl, nur bis zur
    /// Bruttogrenze des Satzes und nur ueber 800,00 EUR (darunter rechnet der Scope selbst).
    #[test]
    fn der_sofortabzug_im_band_zaehlt_nur_mit_satz_wahl_und_unter_der_bruttogrenze() {
        // (Name, Felder, Aufschlag in EURO auf 1.000)
        let faelle = [
            ("900 brutto, 19 %, Wahl", am(90_000, Some("satz_19"), true, None), 900),
            ("952,00 brutto, 19 %, Wahl", am(95_200, Some("satz_19"), true, None), 952),
            ("952,01 brutto, 19 %, Wahl: darueber", am(95_201, Some("satz_19"), true, Some(4)), 0),
            ("856,00 brutto, 7 %, Wahl", am(85_600, Some("satz_7"), true, None), 856),
            ("900 brutto, 7 %, Wahl: darueber", am(90_000, Some("satz_7"), true, Some(4)), 0),
            ("900 brutto, keine Umsatzsteuer: darueber", am(90_000, Some("keine_ust"), true, Some(4)), 0),
            ("900 brutto, 19 %, ohne Wahl", am(90_000, Some("satz_19"), false, None), 0),
            ("900 brutto, Satz offen: nie ein stiller Satz", am(90_000, None, true, None), 0),
            ("800,00 brutto, 19 %: darunter rechnet der Scope", am(80_000, Some("satz_19"), true, None), 0),
            ("900,50 brutto, 19 %, Wahl: volle Euro", am(90_050, Some("satz_19"), true, None), 900),
        ];
        for (name, f, soll) in faelle {
            let ist = mit_sofortabzug_im_band(&f, Euro::new(1_000)).unwrap().get() - 1_000;
            assert_eq!(ist, soll, "{name}");
        }
    }

    /// Die `AfA` gilt nur ueber der Netto-Grenze, der Sofortabzug im Band zaehlt sie nicht doppelt, und der Scope bekommt im Band
    /// keinen Preis ueber 800,00 EUR (`p6_2_gwg` lieferte dafuer 0): `(an_gesamt, gesamt)`.
    #[test]
    fn die_afa_gilt_nur_ueber_der_netto_grenze() {
        // (Name, Felder, AfA an_gesamt, GWG an gesamt, AfA gesamt)
        let faelle = [
            ("900 brutto, 19 %, Wahl, Nutzungsdauer 4: Sofortabzug, keine AfA", am(90_000, Some("satz_19"), true, Some(4)), None, None, 0),
            ("952,01 brutto, 19 %, Nutzungsdauer 4: AfA 238", am(95_201, Some("satz_19"), true, Some(4)), Some(238), None, 238),
            ("856,01 brutto, 7 %, Nutzungsdauer 4: AfA 214", am(85_601, Some("satz_7"), true, Some(4)), Some(214), None, 214),
            ("800,01 brutto, keine Umsatzsteuer, Nutzungsdauer 4: AfA 200", am(80_001, Some("keine_ust"), true, Some(4)), Some(200), None, 200),
            ("800,00 brutto, Wahl: der Scope rechnet", am(80_000, None, true, Some(4)), Some(800), Some(800), 0),
            ("900 brutto, Satz offen, Nutzungsdauer 4: die Schaetzung rechnet die AfA, die Sperre haelt sie zurueck", am(90_000, None, true, Some(4)), Some(225), None, 225),
        ];
        for (name, f, an, gwg, afa) in faelle {
            assert_eq!(am_an_gesamt(&f).unwrap().map(Euro::get), an, "{name}: an_gesamt");
            let (g, a) = am_gesamt(&f).unwrap();
            assert_eq!((g.map(Euro::get), a.get()), (gwg, afa), "{name}: gesamt");
        }
    }
}

/// Abweichung Nr. 89 an den Aufrufstellen der Rechnung: wer den Abzug eines Geraets der Liste zaehlt (nach dem Scope), wer den
/// Sofortabzug und die `AfA` auslaesst. Die Sperre und die Ende-zu-Ende-Faelle stehen in `tests/sperre_scheiben_hermetisch.rs`,
/// `tests/gesamt_mischfaelle_hermetisch.rs` und `api/tests/kette_endstand_hermetisch.rs`.
#[cfg(test)]
mod computer_liste {
    use domain::Euro;
    use serde_json::{json, Value};

    use super::{am_an_gesamt, am_gesamt, mit_computer_im_kaufjahr, mit_sofortabzug_im_band};
    use crate::testhilfe::{felder, store};

    /// Felder eines Arbeitsmittels: Preis in Cent, Antwort zur Liste (oder keine), Nutzungsdauer (oder keine), Anschaffungsjahr, Monat
    /// (oder keiner), Wahl des Sofortabzugs, Satz (oder keiner).
    fn cs(
        preis: i64,
        liste: Option<bool>,
        nd: Option<i64>,
        kaufjahr: bool,
        monat: Option<i64>,
        sofort: bool,
        satz: Option<&str>,
    ) -> crate::Felder {
        let mut e: Vec<(&str, Value, bool)> = vec![
            ("am_anschaffungskosten", json!(preis), true),
            ("am_gwg_sofortabzug_gewaehlt", json!(sofort), true),
            ("am_afa_ist_anschaffungsjahr", json!(kaufjahr), true),
        ];
        if let Some(l) = liste {
            e.push(("am_computer_software", json!(l), true));
        }
        if let Some(n) = nd {
            e.push(("arbeitsmittel_nutzungsdauer", json!(n), true));
        }
        if let Some(m) = monat {
            e.push(("am_anschaffung_monat", json!(m), true));
        }
        if let Some(s) = satz {
            e.push(("am_umsatzsteuersatz", json!(s), true));
        }
        felder(&store(&e))
    }

    /// Der Abzug im Kaufjahr kommt NACH dem Scope dazu: nur bei „ja“, nur mit einem Jahr Nutzungsdauer (keine Angabe oder 1), nur im
    /// Anschaffungsjahr, volle Euro (abgerundet), nie ein Zwoelftel.
    #[test]
    fn der_abzug_im_kaufjahr_zaehlt_nur_bei_ja_mit_einem_jahr_im_anschaffungsjahr() {
        // (Name, Felder, Aufschlag in EURO auf 1.000)
        let faelle = [
            ("ja, 1.500, keine Angabe, Kaufjahr, Oktober", cs(150_000, Some(true), None, true, Some(10), false, None), 1_500),
            ("ja, 1.500, Nutzungsdauer 1, Kaufjahr, Oktober", cs(150_000, Some(true), Some(1), true, Some(10), false, None), 1_500),
            ("ja, 900 im Band, ohne Satz und Wahl", cs(90_000, Some(true), None, true, None, false, None), 900),
            ("ja, 1.500,99: volle Euro", cs(150_099, Some(true), None, true, None, false, None), 1_500),
            ("ja, 1.500, Folgejahr: 0", cs(150_000, Some(true), None, false, Some(10), false, None), 0),
            ("ja, 1.500, Nutzungsdauer 3: die Verteilung, nicht hier", cs(150_000, Some(true), Some(3), true, Some(10), false, None), 0),
            ("nein, 1.500, Nutzungsdauer 1: wie vor Nr. 89", cs(150_000, Some(false), Some(1), true, Some(10), false, None), 0),
            ("keine Antwort, 1.500: wie vor Nr. 89", cs(150_000, None, Some(1), true, Some(10), false, None), 0),
            ("ja, Preis 0", cs(0, Some(true), None, true, None, false, None), 0),
        ];
        for (name, f, soll) in faelle {
            let ist = mit_computer_im_kaufjahr(&f, Euro::new(1_000)).unwrap().get() - 1_000;
            assert_eq!(ist, soll, "{name}");
        }
    }

    /// Ein Geraet der Liste ist kein `GWG`: der Sofortabzug im Band zaehlt nicht (sonst stuende der Preis doppelt in den Werbungskosten),
    /// der Sofortabzug bis 800 EUR und die `AfA` der Netto-Pruefung auch nicht; die eigene Nutzungsdauer ueber ein Jahr verteilt wie sonst
    /// (`(an_gesamt, gesamt)`: `AfA` in `an_gesamt`, `GWG` und `AfA` in `gesamt`).
    #[test]
    fn ein_geraet_der_liste_ist_kein_gwg_und_die_eigene_nutzungsdauer_verteilt_wie_sonst() {
        // (Name, Felder, Aufschlag im Band, AfA an_gesamt, GWG an gesamt, AfA gesamt)
        let faelle = [
            ("ja, 900, Wahl ja, Satz 19: kein Sofortabzug im Band", cs(90_000, Some(true), None, true, None, true, Some("satz_19")), 0, None, None, 0),
            ("nein, 900, Wahl ja, Satz 19: Sofortabzug im Band", cs(90_000, Some(false), None, true, None, true, Some("satz_19")), 900, None, None, 0),
            ("ja, 700, Wahl ja: kein GWG", cs(70_000, Some(true), None, true, None, true, None), 0, None, None, 0),
            ("nein, 700, Wahl ja: GWG", cs(70_000, Some(false), None, true, None, true, None), 0, Some(700), Some(700), 0),
            ("ja, 1.200, Nutzungsdauer 3, Juli: AfA 200", cs(120_000, Some(true), Some(3), true, Some(7), false, None), 0, Some(200), None, 200),
            ("ja, 700, Nutzungsdauer 3, Folgejahr: AfA 233, kein GWG", cs(70_000, Some(true), Some(3), false, None, true, None), 0, Some(233), None, 233),
            ("ja, 1.500, keine Angabe: die AfA hat nichts, der Abzug kommt danach", cs(150_000, Some(true), None, true, Some(10), false, None), 0, None, None, 0),
        ];
        for (name, f, band, an, gwg, afa) in faelle {
            let ist = mit_sofortabzug_im_band(&f, Euro::new(1_000)).unwrap().get() - 1_000;
            assert_eq!(ist, band, "{name}: Sofortabzug im Band");
            assert_eq!(am_an_gesamt(&f).unwrap().map(Euro::get), an, "{name}: an_gesamt");
            let (g, a) = am_gesamt(&f).unwrap();
            assert_eq!((g.map(Euro::get), a.get()), (gwg, afa), "{name}: gesamt");
        }
    }
}

/// Abweichung Nr. 93 an der Rechenstelle: `berufsverband` liest je Person nur ihr eigenes Feld, rundet auf volle Euro AUF und zaehlt leer,
/// 0 und einen negativen Wert (am Schreibweg vorbei im Store, das Laden prueft nie) als 0. Die Ende-zu-Ende-Faelle stehen in
/// `api/tests/berufsverband_hermetisch.rs` und `api/tests/kette_endstand_hermetisch.rs`.
#[cfg(test)]
mod berufsverband_beitraege {
    use domain::Euro;
    use serde_json::json;

    use super::{berufsverband, mit_berufsverband, BERUFSVERBAND, BERUFSVERBAND_PARTNER};
    use crate::testhilfe::{felder, store};

    #[test]
    fn der_betrag_ist_je_person_aufgerundet_und_nie_negativ() {
        // (Name, Cent von Person A, Cent von Person B, Euro von Person A, Euro von Person B)
        let faelle = [
            ("leer", None, None, 0, 0),
            ("0", Some(0), Some(0), 0, 0),
            ("1 Cent steigt auf 1 Euro", Some(1), Some(1), 1, 1),
            ("1.500,00", Some(150_000), Some(150_000), 1_500, 1_500),
            ("1.500,01 steigt auf 1.501", Some(150_001), Some(150_001), 1_501, 1_501),
            ("negativ im Store zaehlt 0", Some(-500), Some(-500), 0, 0),
            ("nur Person A", Some(17_800), None, 178, 0),
            ("nur Person B", None, Some(17_800), 0, 178),
        ];
        for (name, a, b, euro_a, euro_b) in faelle {
            let mut e = Vec::new();
            if let Some(c) = a {
                e.push((BERUFSVERBAND, json!(c), true));
            }
            if let Some(c) = b {
                e.push((BERUFSVERBAND_PARTNER, json!(c), true));
            }
            let f = felder(&store(&e));
            assert_eq!(berufsverband(&f, false).unwrap().get(), euro_a, "{name}: Person A");
            assert_eq!(berufsverband(&f, true).unwrap().get(), euro_b, "{name}: Person B");
            // Zu den Werbungskosten DER PERSON A zaehlt nur ihr Betrag, nie der des Ehegatten.
            assert_eq!(mit_berufsverband(&f, Euro::new(1_000)).unwrap().get(), 1_000 + euro_a, "{name}: mit_berufsverband");
        }
    }
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
                (alt_klasse(dezimal_alt(&v)), alt_klasse(dezimal(&py(&v)).map(domain::Km::get)));
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

/// Zwei Listen, dieselbe Fachfrage, keine Verbindung: die Bindung sagt mit `gate: false` "keine Voraussetzung", eine eigene
/// Tabelle der Bedingungen im Rechenpfad sagt weiter "Voraussetzung", und nur der Rechenpfad zaehlt fuer die Zahl. Bei der
/// Dienstwohnung war das zwei Jahre lang so (3.178 EUR Unterschied; erst das Herausnehmen aus der Tabelle brachte sie zurueck).
/// Gegenstueck zu `tests/test_gate_polaritaet.py::test_gate_false_reicht_nicht_der_rechenpfad_fuehrt_eine_zweite_liste`.
/// Dieselben Tabellen stehen in Rust zweimal: hier (Rechenpfad, `zweige/wk.rs`) und in `deklaration/konstanten.rs` (Sperrgrund-Pfad); beide haben diesen Test.
#[cfg(test)]
mod bedingungen_ohne_gate_false {
    use std::collections::BTreeSet;

    use super::{DHF_BEDINGUNGEN, UEBERNACHTUNG_BEDINGUNGEN};

    #[test]
    fn die_bedingungen_des_rechenpfads_tragen_kein_gate_false() {
        let wurzel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let reg = bindung::lade_registry_der_wurzel(&wurzel).unwrap();
        let kein_gate: BTreeSet<&str> = reg
            .dateien
            .iter()
            .flat_map(|(_, d)| d.bindungen.iter())
            .filter(|b| b.gate == Some(false))
            .map(|b| b.feld_id.as_str())
            .collect();
        assert!(kein_gate.len() >= 15, "nur {} Felder mit gate: false gelesen", kein_gate.len());
        for (name, tabelle) in [
            ("DHF_BEDINGUNGEN", &DHF_BEDINGUNGEN[..]),
            ("UEBERNACHTUNG_BEDINGUNGEN", &UEBERNACHTUNG_BEDINGUNGEN[..]),
        ] {
            assert!(!tabelle.is_empty(), "{name} ist leer");
            let doppelt: Vec<&&str> = tabelle.iter().filter(|f| kein_gate.contains(**f)).collect();
            assert!(
                doppelt.is_empty(),
                "{doppelt:?} traegt in der Bindung `gate: false`, steht aber in {name} und ist damit im Rechenpfad weiter \
                 Voraussetzung. Die Bindung allein entscheidet nicht ueber die Zahl: beide Stellen muessen dasselbe sagen"
            );
        }
    }
}
