//! `_zweig_festzusetzende_est_gesamt`, Teil 2: `_festzusetzende(freibetrag)` — der volle
//! Bescheid bei gegebenem § 32-Abs.-6-Kinderfreibetrag (`bescheid_zweige.py:859-1013`).
//! Reihenfolge: § 34 → § 35 → Est → § 32b → Kapital § 32d. (Der Rentner-Zweig macht § 32b NACH Kapital.)
use domain::{Cent, Euro};
use engine::zugriff::teil2::est::{tarif_est, TarifEingabe};
use engine::zugriff::teil2::gesamt::{gesamt_tarifliche, gesamt_zve, GesamtfallEingabe};
use engine::zugriff::teil2::sonstige::{p32b_1, ProgressionsvorbehaltEingabe};

use super::rechnen::{add, mal, mal_div, max0, sub, R};
use super::tarif::{kapital, p34_chooser, p35_credit, Lage, SolzInfo};

/// § 32b Post-Engine-Wrapper (Progressionsvorbehalt) mit nachgezogenem § 35-Deckel-3. `est_raw` ist
/// die ESt des `g2` ohne § 32b; Rueckgabe `(est_raw', est_ohne_p35)`.
fn p32b_wrapper(l: &Lage<'_>, g2: &GesamtfallEingabe, est_raw: Euro) -> R<(Euro, Euro)> {
    let pre = gesamt_tarifliche(g2, l.p)?;
    let est_ohne_tarifliche = sub(est_raw, pre)?;
    let zve32b = gesamt_zve(g2, l.p)?;
    let mut t_32b = Euro::new(0);
    let mut est = est_raw;
    if zve32b.get() > 0 {
        let est_erhoeht = tarif_est(&TarifEingabe {
            vz: l.vz,
            veranlagung: l.veranlagung.tarif_strikt()?,
            zu_versteuerndes_einkommen: add(zve32b, l.pe_raw)?,
        })?;
        t_32b = p32b_1(&ProgressionsvorbehaltEingabe {
            zu_versteuerndes_einkommen: zve32b,
            progressionseinkuenfte: l.pe_raw,
            est_auf_erhoehte_bemessung: est_erhoeht,
        })?;
        est = add(t_32b, est_ohne_tarifliche)?;
    }
    // § 51a-Basis NACH § 32b, VOR § 35.
    let est_ohne_p35 = est;
    if l.p35.aktiv() {
        let deckel3 = mal_div(
            l.p35.zaehler_ges.get(),
            max0(sub(t_32b, l.dba_anrechnung)?).get(),
            l.p35.nenner.get(),
        )?;
        let credit = Euro::new(
            mal(4, l.p35.messbetrag_ges)?
                .get()
                .min(l.p35.gezahlt.get())
                .min(deckel3),
        );
        est = max0(sub(est, credit)?);
    }
    Ok((est, est_ohne_p35))
}

/// `_festzusetzende(freibetrag)`: der volle Bescheid bei gegebenem Kinderfreibetrag (EURO).
///
/// Schreibt `info` (SolZ-Zwischenstand) und — im § 32d-Fall — `kist_kap_cent`/`kap_guenstiger_gewonnen`
/// in die Extras, jeweils nur im Lauf mit Freibetrag > 0 oder ohne Kinder.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
pub(super) fn festzusetzende(
    l: &Lage<'_>,
    g: &GesamtfallEingabe,
    freibetrag: Euro,
    info: &mut Option<SolzInfo>,
) -> R<Euro> {
    let g2 = GesamtfallEingabe {
        freibetraege_kinder: freibetrag,
        ..*g
    };
    let mut g2 = p34_chooser(l, g2)?;
    // § 35 Abs. 1: je Zweig, weil die tarifliche ESt freibetrag-abhaengig ist.
    let credit = p35_credit(l, &g2)?;
    if !l.pe_active() {
        g2.steuerermaessigungen = add(g2.steuerermaessigungen, credit)?;
    }
    let mut est_raw = l.est(&g2)?; // KEIN Kapital (est_regulaer_ohne_kap)
                                   // § 51a Abs. 2 S. 3: die KiSt-Basis traegt die § 35-Anrechnung NICHT; NEU rechnen statt zurueck-
                                   // zuaddieren (der Kredit ist nur bis zum Catala-Deckel wirksam).
    let mut est_ohne_p35 = if l.pe_active() || credit.get() == 0 {
        est_raw
    } else {
        l.est(&GesamtfallEingabe {
            steuerermaessigungen: sub(g2.steuerermaessigungen, credit)?,
            ..g2
        })?
    };
    if l.pe_active() {
        (est_raw, est_ohne_p35) = p32b_wrapper(l, &g2, est_raw)?;
    }
    let einzel_lauf = freibetrag.get() > 0 || l.kinder == 0;
    if l.kapitaleinkuenfte.get() <= 0 {
        if einzel_lauf {
            *info = Some(SolzInfo {
                est_mit_fb: est_raw,
                kap_st: Euro::new(0),
                est_ohne_p35,
            });
        }
        return Ok(est_raw);
    }
    let (_est_mit, kap) = kapital(l, &g2, est_raw)?;
    let result = add(est_raw, kap.kap_st_k)?;
    if einzel_lauf {
        *info = Some(SolzInfo {
            est_mit_fb: result,
            kap_st: kap.kap_st_k,
            est_ohne_p35,
        });
        l.mit_extras(|e| {
            e.kist_kap_cent = Some(Cent::new(kap.kist_kap_cent.get()));
            e.kap_guenstiger_gewonnen = Some(kap.guenstiger);
        });
    }
    Ok(result)
}
