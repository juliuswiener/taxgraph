//! `_mit_ring_werten` (`bescheid_deklaration.py:61`): haengt berechnete Ring-Werte als fertige Events
//! in den Snapshot ein, damit die Deklaration sie als Kz schreiben kann.
//!
//! Jeder Eintrag ist `zustand=bestaetigt`, `herkunft=berechnet/amtlich/system`.
//!
//! PARITÄT: Pythons Event traegt zusaetzlich `schreiber="engine"` und `signal={signal_1: None,
//! signal_2: None}`. [`SnapshotFeld`] kennt beides nicht (Snapshot-Sicht = `wert`, `zustand`,
//! `herkunft`); beide Werte sind konstant und der Writer liest sie nicht.
use bindung::Params;
use domain::{Achsenwert, Euro, Herkunft, HerkunftVektor, PruefTiefe, PyWert, Vz, Zustand};
use elster::parse_instanz;
use engine::zugriff::teil2::kapital::{
    kapital_verrechnung, sparer_pb, KapitalVerrechnungEingabe, SparerPbEingabe,
};
use store::SnapshotFeld;

use super::c2;
use super::konstanten::VERPFLEGUNG_TAGE;
use crate::abzuege::abs3_eligible;
use crate::einkuenfte::{KAP_ERTRAEGE, KAP_ERTRAEGE_PARTNER, KAP_TOEPFE, KAP_TOEPFE_PARTNER};
use crate::zweige::netto_vg;
use crate::{
    cent_zu_euro, euro_plus, feld_int_oder_null, ist_positive_zahl, ist_true, ist_zusammen, minus,
    plus, py_int, wert, zahl_oder_null, BescheidFehler, Felder,
};

type R<T> = Result<T, BescheidFehler>;

/// Die Herkunft jedes injizierten Werts: `berechnet` / `amtlich` / `system` (Haftung System).
///
/// # Errors
/// [`BescheidFehler::Python`] nur, wenn eine Achse leer waere (kann nicht eintreten).
pub(crate) fn berechnet_herkunft() -> R<HerkunftVektor> {
    let achse = |s: &str| {
        Achsenwert::new(s).map_err(|_| BescheidFehler::Python {
            klasse: "ValueError",
            was: "leere Herkunftsachse",
        })
    };
    Ok(HerkunftVektor::Voll(Herkunft {
        herkunft: achse("berechnet")?,
        pruef_tiefe: PruefTiefe::Amtlich,
        haftung: achse("system")?,
    }))
}

fn setze(f: &mut Felder, fid: &str, w: PyWert, h: &HerkunftVektor) {
    f.insert(
        fid.to_owned(),
        SnapshotFeld {
            wert: w,
            zustand: Zustand::Bestaetigt,
            herkunft: h.clone(),
        },
    );
}

/// Hängt die berechneten Ring-Werte in `felder` ein (Python liefert dasselbe Dict zurueck).
///
/// `vz`: `None` = ein Jahr ohne Parameter (Python: `FileNotFoundError` im `try`, siehe (1) und
/// (2)/(3)); alle anderen Abschnitte sind vom Jahr unabhaengig.
///
/// # Errors
/// Nur i64-Ueberlauf ([`BescheidFehler::Ueberlauf`]); jede Python-Ausnahme, die Python im `try`
/// schluckt, schluckt Rust hier ebenso.
///
/// ```
/// use bescheid::deklaration::mit_ring_werten;
/// use bescheid::testhilfe::params;
/// use bescheid::Felder;
/// use domain::Vz;
/// let mut f = Felder::new();
/// mit_ring_werten(&mut f, Some(Vz::Vz2025), params()).unwrap();
/// assert!(f.is_empty()); // ohne Verpflegungs-/Kapital-Felder entsteht kein Ring-Wert
/// ```
pub fn mit_ring_werten(felder: &mut Felder, vz: Option<Vz>, p: &Params) -> R<()> {
    let h = berechnet_herkunft()?;
    verpflegung(felder, vz, p, &h)?;
    kap_antrag(felder, vz, p, &h)?;
    haushaltsnah(felder, &h)?;
    vermietung(felder, &h)?;
    einzelzeilen(felder, &h)?;
    p34_antrag(felder, vz, &h)
}

// ---------------------------------------------------------------- (1) Verpflegungskuerzung

/// (1) E0205508: Kuerzungsbetrag wegen Mahlzeitengestellung in CENT. Inert: ohne BESTAETIGTES
/// Tage-Feld kein Eintrag (auch kein Wert 0). Ein vorlaeufiger Wert ist so gut wie nicht da.
fn verpflegung(f: &mut Felder, vz: Option<Vz>, p: &Params, h: &HerkunftVektor) -> R<()> {
    if !VERPFLEGUNG_TAGE.iter().any(|t| bestaetigt(f, t).is_some()) {
        return Ok(());
    }
    let kuerzung = match kuerzung_cent(f, vz, p) {
        Ok(c) => c,
        // PARITÄT: P3 — `except Exception: kuerzung_cent = 0`. Jede Ausnahme im Runner (nicht
        // lesbarer Wert, Jahr ohne Params) wird zu "keine Kuerzung"; der Nutzer erfaehrt es nicht.
        Err(e) if e.python_klasse().is_some() => 0,
        Err(e) => return Err(e),
    };
    if kuerzung > 0 {
        setze(f, "p9_4a_kuerzung_nach_entgelt", PyWert::Ganz(kuerzung), h);
    }
    Ok(())
}

/// `runner._verpflegung_kuerzung_cent(s, vz)`: Kuerzung wegen Mahlzeiten nach Entgelt, in Cent
/// (§ 9 Abs. 4a S. 8-10). Gelesen werden nur BESTAETIGTE Felder von `s`; ein vorlaeufiges zaehlt wie ein
/// fehlendes (0).
fn kuerzung_cent(s: &Felder, vz: Option<Vz>, p: &Params) -> R<i64> {
    let vz = vz.ok_or(BescheidFehler::Python {
        klasse: "FileNotFoundError",
        was: "params/<vz>/verpflegung_p9_4a.yaml",
    })?;
    let sa = p
        .verpflegung(vz)
        .map_err(|e| BescheidFehler::Engine(e.into()))?;
    // PARITÄT: `int(s.get(k, 0))` — fehlend ist 0 (fail-open default), ein `null` ist `TypeError`.
    let ganz = |k: &str| bestaetigt(s, k).map_or(Ok(0), py_int);
    let mal = |a: i64, b: i64| {
        a.checked_mul(b)
            .ok_or(BescheidFehler::Ueberlauf("Verpflegung"))
    };
    let in_frist = |tage: i64, nach: i64| -> R<i64> { Ok(minus(tage, nach.max(0))?.max(0)) };
    let t24 = ganz("tage_24h")?;
    let tar = ganz("tage_an_abreise")?;
    let t8 = ganz("tage_ueber_8h_eintaegig")?;
    let (n24, nar, n8) = (
        ganz("vpf_tage_24h_nach_drei_monaten")?,
        ganz("vpf_tage_an_abreise_nach_drei_monaten")?,
        ganz("vpf_tage_ueber_8h_nach_drei_monaten")?,
    );
    let s24 = mal(mal(in_frist(t24, n24)?, sa.pauschale_24h.get())?, 100)?;
    let s_ar = mal(
        mal(in_frist(tar, nar)?, sa.pauschale_an_abreise.get())?,
        100,
    )?;
    let s_8 = mal(mal(in_frist(t8, n8)?, sa.pauschale_ab_8h.get())?, 100)?;
    let fruehstuecke = ganz("vpf_fruehstuecke_gestellt_anzahl")?;
    let mittag = ganz("vpf_mittagessen_gestellt_anzahl")?;
    let abend = ganz("vpf_abendessen_gestellt_anzahl")?;
    let p24_cent = mal(sa.pauschale_24h.get(), 100)?;
    let je_fruehstueck = mal(p24_cent, sa.kuerzung_fruehstueck_prozent)?.div_euclid(100);
    let je_mittag_abend = mal(p24_cent, sa.kuerzung_mittag_abend_prozent)?.div_euclid(100);
    let brutto28 = plus(
        plus(
            mal(fruehstuecke, je_fruehstueck)?,
            mal(mittag, je_mittag_abend)?,
        )?,
        mal(abend, je_mittag_abend)?,
    )?;
    // Mahlzeiten zuerst den 28-EUR-Tagen, den Rest den 14-EUR-Tagen zuordnen.
    let k28 = brutto28.min(s24);
    let rest = minus(brutto28, k28)?.max(0);
    let k14 = rest.min(plus(s_ar, s_8)?);
    let brutto = plus(k28, k14)?;
    let entgelt = ganz("vpf_mahlzeiten_gezahltes_entgelt")?;
    // Python liest auch die steuerfreie Erstattung (`int(...)`): ein `null` dort ist `TypeError`
    // und macht die ganze Kuerzung zu 0.
    ganz("vpf_steuerfreie_erstattung_betrag")?;
    Ok(minus(brutto, entgelt)?.max(0))
}

// ---------------------------------------------------------------- (2)+(3) Anlage KAP

/// Der Wert eines BESTAETIGTEN Feldes (Python `_kap_wert`, im Verpflegungs-Block der Filter auf `s`). Ein
/// vorlaeufiger Wert ist so gut wie nicht da: er loest weder den KAP-Antrag noch die Verpflegungskuerzung aus
/// und geht in keine Summe ein.
fn bestaetigt<'a>(f: &'a Felder, fid: &str) -> Option<&'a PyWert> {
    f.get(fid)
        .filter(|x| x.zustand == Zustand::Bestaetigt)
        .map(|x| &x.wert)
}

/// Python `_c2` des Antrag-Blocks: [`c2`], aber ein unbestaetigtes Feld zaehlt 0.
///
/// Nicht `c2` selbst filtern: `sperre.rs` liest damit `rentner_veraeusserungsgewinn` fuer den Guard.
fn c2_bestaetigt(f: &Felder, fid: &str) -> R<i64> {
    if bestaetigt(f, fid).is_some() {
        c2(f, fid)
    } else {
        Ok(0)
    }
}

/// (2)+(3) E1900401 (Antrag Guenstigerpruefung) und E1901401 (genutzter Sparer-Pauschbetrag): beide
/// zusammen NACH erfolgreicher Berechnung, oder keins von beiden. Es zaehlen nur bestaetigte Werte.
fn kap_antrag(f: &mut Felder, vz: Option<Vz>, p: &Params, h: &HerkunftVektor) -> R<()> {
    let zusammen = ist_zusammen(f);
    let positiv = |fid: &str| ist_positive_zahl(bestaetigt(f, fid));
    let erklaert = KAP_TOEPFE.iter().any(|t| positiv(t))
        || positiv(KAP_ERTRAEGE)
        || (zusammen
            && (KAP_TOEPFE_PARTNER.iter().any(|t| positiv(t)) || positiv(KAP_ERTRAEGE_PARTNER)));
    if !erklaert {
        return Ok(());
    }
    let pb_genutzt_cent = match pb_genutzt(f, zusammen, vz, p) {
        Ok(c) => c,
        // PARITÄT: `except Exception: pb_genutzt_cent = None` — bei Ausfall bleibt KEIN Eintrag stehen.
        Err(e) if e.python_klasse().is_some() => return Ok(()),
        Err(e) => return Err(e),
    };
    setze(f, "kap_antrag_guenstigerpruefung", PyWert::Bool(true), h);
    setze(
        f,
        "kap_sparer_pauschbetrag_genutzt",
        PyWert::Ganz(pb_genutzt_cent),
        h,
    );
    Ok(())
}

/// Toepfe XOR Aggregat je Person, Sparer-Pauschbetrag; Ergebnis `max(0, verrechnete - danach) * 100`.
fn pb_genutzt(f: &Felder, zusammen: bool, vz: Option<Vz>, p: &Params) -> R<i64> {
    let toepfe_belegt = |toepfe: &[&str]| -> R<bool> {
        toepfe
            .iter()
            .try_fold(false, |acc, t| Ok(acc || c2_bestaetigt(f, t)? != 0))
    };
    let verrechne = |suffix: &str| -> R<Euro> {
        let euro = |k: &str| c2_bestaetigt(f, &format!("{k}{suffix}")).map(cent_zu_euro);
        Ok(kapital_verrechnung(&KapitalVerrechnungEingabe {
            gewinn_aktien: euro("kap_gewinn_aktien")?,
            verlust_aktien: euro("kap_verlust_aktien")?,
            gewinn_sonstige: euro("kap_gewinn_sonstige")?,
            verlust_sonstige: euro("kap_verlust_sonstige")?,
        })?)
    };
    let mut verrechnete = if toepfe_belegt(&KAP_TOEPFE)? {
        verrechne("")?
    } else {
        c2_bestaetigt(f, KAP_ERTRAEGE).map(cent_zu_euro)?
    };
    if zusammen {
        let partner = if toepfe_belegt(&KAP_TOEPFE_PARTNER)? {
            verrechne("_partner")?
        } else {
            c2_bestaetigt(f, KAP_ERTRAEGE_PARTNER).map(cent_zu_euro)?
        };
        verrechnete = euro_plus(verrechnete, partner)?;
    }
    let vz = vz.ok_or(BescheidFehler::Python {
        klasse: "FileNotFoundError",
        was: "params/<vz>/sparer_pauschbetrag",
    })?;
    let nach_pb = sparer_pb(
        &SparerPbEingabe {
            vz,
            kapitalertraege: verrechnete,
            zusammenveranlagung: zusammen,
        },
        p,
    )?;
    let genutzt_euro = minus(verrechnete.get(), nach_pb.get())?.max(0);
    genutzt_euro
        .checked_mul(100)
        .ok_or(BescheidFehler::Ueberlauf("Sparer-Pauschbetrag genutzt"))
}

// ---------------------------------------------------------------- (4) § 35a Summen-Kz

/// Σ der bestaetigten Einz-Instanzen von `basis` (Basis-`feld_id` ohne Suffix = Instanz 1), in Cent.
///
/// PARITÄT: fail-open default — ein Nicht-Zahlwert zaehlt 0.
fn instanz_summe(f: &Felder, basis: &str) -> R<i64> {
    let mut total = 0_i64;
    for (fid, ev) in f {
        let b = parse_instanz(fid).map_or(fid.as_str(), |(b, _)| b);
        if b != basis || ev.zustand != Zustand::Bestaetigt {
            continue;
        }
        total = plus(total, zahl_oder_null(Some(&ev.wert))?)?;
    }
    Ok(total)
}

/// (4) `hh_minijob_aufwendungen`/`hh_dienstleistungen`/`hh_handwerker_arbeitskosten` aus den
/// Einz-Instanzen. Inert: keine Σ > 0, kein Eintrag.
fn haushaltsnah(f: &mut Felder, h: &HerkunftVektor) -> R<()> {
    for (summe_fid, betrag_fid) in [
        ("hh_minijob_aufwendungen", "hh_minijob_betrag"),
        ("hh_dienstleistungen", "hh_dienstleistung_betrag"),
        ("hh_handwerker_arbeitskosten", "hh_handwerker_betrag"),
    ] {
        let summe = instanz_summe(f, betrag_fid)?;
        if summe > 0 {
            setze(f, summe_fid, PyWert::Ganz(summe), h);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- (7) Anlage V

/// `isinstance(v, int)` — Pythons `bool` ist ein `int` (0/1).
///
/// Ein `u64` ueber `i64::MAX` meldet wie bisher [`BescheidFehler::Ueberlauf`] (`int_mit_bool`
/// traegt dieselbe Grenze als `I64Grenze`).
fn py_int_typ(v: &PyWert) -> R<Option<i64>> {
    match v {
        PyWert::Bool(b) => Ok(Some(i64::from(*b))),
        PyWert::Ganz(n) => Ok(Some(*n)),
        PyWert::GrossGanz(_) => Err(BescheidFehler::Ueberlauf("int")),
        // `Gleit` ist kein `int` (`isinstance(2.0, int)` ist in CPython falsch), Text auch nicht.
        _ => Ok(None),
    }
}

/// Bestaetigtes Feld mit ganzzahligem Wert (Bool zaehlt): `(Wert als i64, Wert unveraendert)`.
fn bestaetigt_int(f: &Felder, fid: &str) -> R<Option<(i64, PyWert)>> {
    let Some(x) = f.get(fid).filter(|x| x.zustand == Zustand::Bestaetigt) else {
        return Ok(None);
    };
    Ok(py_int_typ(&x.wert)?.map(|i| (i, x.wert.clone())))
}

/// `_w if isinstance(_w, int) and not isinstance(_w, bool) else 0` fuer ein bestaetigtes Feld.
///
/// PARITÄT: fail-open default — ein unbestaetigtes oder nicht ganzzahliges Feld zaehlt 0.
fn bestaetigt_ganz_oder_null(f: &Felder, fid: &str) -> R<i64> {
    let Some(x) = f.get(fid).filter(|x| x.zustand == Zustand::Bestaetigt) else {
        return Ok(0);
    };
    match &x.wert {
        PyWert::Bool(_) => Ok(0),
        v => Ok(py_int_typ(v)?.unwrap_or(0)),
    }
}

/// (7) Summenzeilen der Wohnungs-Mieteinnahmen, Werbungskosten und Ueberschuss (checkESt verlangt
/// Einzelbetrag UND Summe). Bei Alleineigentum voll auf Person A.
fn vermietung(f: &mut Felder, h: &HerkunftVektor) -> R<()> {
    let Some((einnahmen, einnahmen_wert)) =
        bestaetigt_int(f, "vv_einnahmen")?.filter(|(e, _)| *e > 0)
    else {
        return Ok(());
    };
    let mut wk = 0_i64;
    for fid in [
        "vv_gebaeude_afa",
        "vv_schuldzinsen",
        "vv_erhaltungsaufwand",
        "vv_sonstige_wk",
    ] {
        wk = plus(wk, bestaetigt_ganz_oder_null(f, fid)?)?;
    }
    let umlage = bestaetigt_ganz_oder_null(f, "vv_nebenkosten_umgelegt")?;
    // E0701401 = Summe ALLER Einnahmen (Mieten + Umlagen), E0700206 nur die der Wohnungs-Mieten.
    let gesamt = plus(einnahmen, umlage)?;
    setze(f, "vv_einnahmen_summe_gesamt", PyWert::Ganz(gesamt), h);
    let ueberschuss = minus(gesamt, wk)?;
    setze(f, "vv_summe_werbungskosten", PyWert::Ganz(wk), h);
    setze(f, "vv_ueberschuss", PyWert::Ganz(ueberschuss), h);
    setze(f, "vv_ueberschuss_person_a", PyWert::Ganz(ueberschuss), h);
    setze(f, "vv_mieteinnahmen_summe", einnahmen_wert, h);
    Ok(())
}

// ---------------------------------------------------------------- (5)(6) und weitere Einzelzeilen

/// Einzelzeile `ziel` mit demselben Betrag wie `summe`, sobald diese bestaetigt und > 0 ist.
fn einzelzeile(f: &mut Felder, summe: &str, ziel: &str, h: &HerkunftVektor) -> R<()> {
    if let Some((_, w)) = bestaetigt_int(f, summe)?.filter(|(i, _)| *i > 0) {
        setze(f, ziel, w, h);
    }
    Ok(())
}

/// (8) § 34 Abs. 3 Antragszeile (`E0801602` G / `E0805003` S / `E0901704` L, je nach
/// `rentner_veraeusserungs_betriebsart`, `est_mapping.VERZWEIGUNG`): der Gewinn der Basiszeile, fuer
/// den der ermaessigte Satz beantragt wird. Geschrieben nur, was der Chooser auch rechnet
/// (`tarif::p34_chooser`: Antrag UND `abs3_eligible` UND 0 < `netto_vg` <= 5 Mio); nur bestaetigte
/// Felder zaehlen. Ueber 5 Mio sperrt `abs3_ueber_5mio_offen` vorher.
///
/// ponytail: nur Person A (AK2 des Eintrags p34-antrag-ohne-kennzahl-erreicht-elster-nicht);
/// `vz = None` (Jahr ohne Parameter) schreibt nichts, Python rechnet hier mit jedem int.
fn p34_antrag(f: &mut Felder, vz: Option<Vz>, h: &HerkunftVektor) -> R<()> {
    let Some(vz) = vz else {
        return Ok(());
    };
    let fb: Felder = f
        .iter()
        .filter(|(_, x)| x.zustand == Zustand::Bestaetigt)
        .map(|(k, x)| (k.clone(), x.clone()))
        .collect();
    let netto = netto_vg(&fb)?.get();
    if ist_true(wert(&fb, "antrag_ermaessigter_satz"))
        && abs3_eligible(&fb, vz)?
        && 0 < netto
        && netto <= 5_000_000
    {
        let vg_cent = feld_int_oder_null(&fb, "rentner_veraeusserungsgewinn")?;
        setze(f, "p34_abs3_antragsbetrag", PyWert::Ganz(vg_cent), h);
    }
    Ok(())
}

/// (6) § 35c-Einzelzeile, GewSt-Kz, § 22 Nr. 3, § 10 Abs. 1 Nr. 7, (5) § 35c-Umkehrung.
fn einzelzeilen(f: &mut Felder, h: &HerkunftVektor) -> R<()> {
    einzelzeile(
        f,
        "p35c_sanierungsaufwendungen",
        "p35c_massnahme_einzelbetrag",
        h,
    )?;
    gewerbesteuer(f, h)?;
    p22_nr3(f, h)?;
    einzelzeile(
        f,
        "berufsausbildung_aufwendungen",
        "berufsausbildung_einzelbetrag",
        h,
    )?;
    // E0240902 fragt UMGEKEHRT zu unserem Gate p35c_keine_doppelfoerderung.
    if let Some(x) = f
        .get("p35c_keine_doppelfoerderung")
        .filter(|x| x.zustand == Zustand::Bestaetigt)
    {
        if let PyWert::Bool(b) = x.wert {
            setze(f, "p35c_foerderung_in_anspruch", PyWert::Bool(!b), h);
        }
    }
    Ok(())
}

/// § 35 / § 16 Abs. 1 `GewStG`: zu zahlende Gewerbesteuer = auf volle Euro ABGERUNDETER Messbetrag
/// mal Hebesatz (ELSTER-Regel 100800013).
fn gewerbesteuer(f: &mut Felder, h: &HerkunftVektor) -> R<()> {
    for (messbetrag, hebesatz, ziel) in [
        ("gewst_messbetrag", "gewst_hebesatz", "gewst_zu_zahlen"),
        (
            "gewst_messbetrag_partner",
            "gewst_hebesatz_partner",
            "gewst_zu_zahlen_partner",
        ),
    ] {
        let (Some((m, _)), Some((s, _))) =
            (bestaetigt_int(f, messbetrag)?, bestaetigt_int(f, hebesatz)?)
        else {
            continue;
        };
        if m > 0 && s > 0 {
            let zu_zahlen = m
                .div_euclid(100)
                .checked_mul(s)
                .ok_or(BescheidFehler::Ueberlauf("gewst_zu_zahlen"))?;
            setze(f, ziel, PyWert::Ganz(zu_zahlen), h);
        }
    }
    Ok(())
}

/// § 22 Nr. 3: aus Einnahmen und Einkuenften folgen Einzelposten und Werbungskosten. Die
/// Werbungskosten NUR bei echt positiver Differenz (negativ = widerspruechliche Eingabe, bleibt sichtbar).
fn p22_nr3(f: &mut Felder, h: &HerkunftVektor) -> R<()> {
    let (Some((einnahmen, einnahmen_wert)), Some((einkuenfte, _))) = (
        bestaetigt_int(f, "p22_nr3_einnahmen")?,
        bestaetigt_int(f, "p22_nr3_einkuenfte")?,
    ) else {
        return Ok(());
    };
    if einnahmen <= 0 {
        return Ok(());
    }
    setze(f, "p22_nr3_einnahmen_einzelbetrag", einnahmen_wert, h);
    let werbungskosten = minus(einnahmen, einkuenfte)?;
    if werbungskosten > 0 {
        setze(f, "p22_nr3_werbungskosten", PyWert::Ganz(werbungskosten), h);
    }
    Ok(())
}

/// Aequivalenz von `py_int_typ` mit `PyWert::int_mit_bool` (D15), ohne Ausnahme.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{json_wert, klasse, pruefe, py};
    use proptest::prelude::*;

    use super::py_int_typ;
    use crate::aequivalenz::alt_klasse;
    use crate::vor_k2::py_int_typ_alt;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        /// Die Alt-Fassung gegen `CPython` — hier entstehen keine D-Nummern: `GrossGanz` meldet
        /// in beiden Fassungen `OverflowError`, und `Gleit`/Text sind in beiden kein `int`.
        #[test]
        fn py_int_typ_alt_wie_pywert(v in json_wert()) {
            let neu = klasse(py(&v).int_mit_bool());
            pruefe(&v, &alt_klasse(py_int_typ_alt(&v)), &neu, Vec::new, &[])?;
        }

        /// Die Produktion gegen die Alt-Fassung.
        #[test]
        fn py_int_typ_wie_alt(v in json_wert()) {
            pruefe(&v, &alt_klasse(py_int_typ_alt(&v)), &alt_klasse(py_int_typ(&py(&v))), Vec::new, &[])?;
        }
    }
}

/// § 34 Abs. 3 Antragszeile (Vault `p34-antrag-ohne-kennzahl-erreicht-elster-nicht`, AK1/AK3): der
/// Ring-Wert entsteht nur, wenn der Chooser Abs. 3 auch rechnet, und landet im Container der Basiszeile.
#[cfg(test)]
mod p34_antrag_tests {
    use domain::{PyWert, Vz};
    use serde_json::{json, Value};

    use super::mit_ring_werten;
    use crate::testhilfe::{felder, index, params, store};
    use crate::Felder;

    /// (Betriebsart, Basis-Kz, Antrags-Kz) — aus dem amtlichen E10-2025.xsd, wie im Python-Test.
    const ARTEN: [(&str, &str, &str); 3] = [
        ("gewerbe", "E0801301", "E0801602"),
        ("selbstaendig", "E0804501", "E0805003"),
        ("land_forst", "E0901201", "E0901704"),
    ];
    const ALLE_ANTRAGS_KZ: [&str; 6] = [
        "E0801602", "E0805003", "E0901704", "E0801903", "E0805305", "E0902002",
    ];
    /// 500.000 EUR: ueber dem Freibetrag, unter 5 Mio.
    const VG: i64 = 50_000_000;

    /// Berechtigter Antragsfall; `abweichend` ersetzt oder ergaenzt Felder (Wert, bestaetigt).
    fn fall(art: &str, abweichend: &[(&str, Value, bool)]) -> Vec<(String, Value, bool)> {
        let mut felder: Vec<(String, Value, bool)> = [
            ("rentner_veraeusserungsgewinn", json!(VG)),
            ("rentner_veraeusserungs_betriebsart", json!(art)),
            ("rentner_alter_55_oder_berufsunfaehig", json!(true)),
            ("rentner_freibetrag_erstmalig", json!(true)),
            ("geburtsjahr", json!(1955)),
            ("antrag_ermaessigter_satz", json!(true)),
            ("dauernd_berufsunfaehig", json!(false)),
            ("ermaessigung_einmal_genutzt", json!(false)),
        ]
        .into_iter()
        .map(|(f, w)| (f.to_owned(), w, true))
        .collect();
        for (f, w, b) in abweichend {
            felder.retain(|(g, _, _)| g != f);
            felder.push(((*f).to_owned(), w.clone(), *b));
        }
        felder
    }

    /// Ring-Werte und Deklaration (VZ 2025) zu einem Fall; `vz` steuert nur `mit_ring_werten`.
    fn lauf(fall: &[(String, Value, bool)], vz: Option<Vz>) -> (Felder, elster::Deklaration) {
        let paare: Vec<(&str, Value, bool)> = fall
            .iter()
            .map(|(f, w, b)| (f.as_str(), w.clone(), *b))
            .collect();
        let mut f = felder(&store(&paare));
        mit_ring_werten(&mut f, vz, params()).unwrap();
        let d = elster::deklariere(&f, index(), 2025, None).unwrap();
        (f, d)
    }

    fn antrags_kz(d: &elster::Deklaration) -> Vec<&str> {
        ALLE_ANTRAGS_KZ
            .into_iter()
            .filter(|k| d.deklaration.contains_key(*k))
            .collect()
    }

    #[test]
    fn antrag_schreibt_die_antrags_kz_der_anlage() {
        for (art, basis, antrag) in ARTEN {
            let (f, d) = lauf(&fall(art, &[]), Some(Vz::Vz2025));
            assert_eq!(
                f.get("p34_abs3_antragsbetrag").map(|x| x.wert.clone()),
                Some(PyWert::Ganz(VG)),
                "{art}: kein Ring-Wert"
            );
            assert_eq!(
                d.deklaration.get(basis).and_then(Value::as_f64),
                Some(500_000.0),
                "{art}: Basis"
            );
            assert_eq!(
                d.deklaration.get(antrag).and_then(Value::as_f64),
                Some(500_000.0),
                "{art}: {antrag}"
            );
            assert_eq!(
                antrags_kz(&d),
                vec![antrag],
                "{art}: nur die eigene Antrags-Kz darf stehen"
            );
        }
    }

    #[test]
    fn ohne_antrag_oder_berechtigung_steht_keine_antrags_kz() {
        let faelle: [(&str, Vec<(&str, Value, bool)>); 8] = [
            (
                "antrag nein",
                vec![("antrag_ermaessigter_satz", json!(false), true)],
            ),
            (
                "antrag unbestaetigt",
                vec![("antrag_ermaessigter_satz", json!(true), false)],
            ),
            (
                "Geburtsjahr unbestaetigt",
                vec![("geburtsjahr", json!(1960), false)],
            ),
            (
                "Gewinn unbestaetigt",
                vec![("rentner_veraeusserungsgewinn", json!(VG), false)],
            ),
            ("zu jung", vec![("geburtsjahr", json!(1990), true)]),
            (
                "schon genutzt",
                vec![("ermaessigung_einmal_genutzt", json!(true), true)],
            ),
            // 40.000 EUR liegen unter dem Freibetrag (45.000): netto_vg 0
            (
                "netto null",
                vec![("rentner_veraeusserungsgewinn", json!(4_000_000), true)],
            ),
            (
                "Alter vorlaeufig",
                vec![
                    ("geburtsjahr", json!(1955), false),
                    ("dauernd_berufsunfaehig", json!(false), true),
                ],
            ),
        ];
        for (name, abweichung) in faelle {
            let (f, d) = lauf(&fall("gewerbe", &abweichung), Some(Vz::Vz2025));
            assert!(
                !f.contains_key("p34_abs3_antragsbetrag"),
                "{name}: Ring-Wert entstand"
            );
            assert_eq!(
                antrags_kz(&d),
                Vec::<&str>::new(),
                "{name}: Antrags-Kz geschrieben"
            );
            // Kontrolle: die Basiszeile steht, wo der Gewinn bestaetigt ist (sonst misst der Fall nichts)
            let gewinn_bestaetigt = abweichung
                .iter()
                .all(|(f, _, b)| *f != "rentner_veraeusserungsgewinn" || *b);
            assert!(
                !gewinn_bestaetigt || d.deklaration.contains_key("E0801301"),
                "{name}: der Fall misst nichts"
            );
        }
    }

    /// Die Grenzen des Netto-Gewinns sind cent genau: 5.000.000,00 EUR (Freibetrag 0) schreiben,
    /// 5.000.001,00 EUR nicht (die Route sperrt dort vorher mit `abs3_ueber_5mio_offen`); 45.001 EUR
    /// lassen 1 EUR netto, 45.000 EUR keinen.
    #[test]
    fn die_netto_grenzen_gelten_cent_genau() {
        let faelle = [
            (500_000_000, true),
            (500_000_100, false),
            (13_600_000, true),
            (4_500_100, true),
            (4_500_000, false),
        ];
        for (vg, schreibt) in faelle {
            let abw = [("rentner_veraeusserungsgewinn", json!(vg), true)];
            let (f, d) = lauf(&fall("gewerbe", &abw), Some(Vz::Vz2025));
            assert_eq!(
                f.contains_key("p34_abs3_antragsbetrag"),
                schreibt,
                "vg={vg}"
            );
            assert_eq!(antrags_kz(&d).len(), usize::from(schreibt), "vg={vg}");
        }
    }

    #[test]
    fn ohne_veranlagungsjahr_steht_keine_antrags_kz() {
        let (f, _) = lauf(&fall("gewerbe", &[]), None);
        assert!(!f.contains_key("p34_abs3_antragsbetrag"));
    }
}
