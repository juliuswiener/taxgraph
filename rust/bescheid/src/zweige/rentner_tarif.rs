//! `_zweig_festzusetzende_est_rentner`, Teil 2: `_festzusetzende_r(freibetrag)`
//! (`bescheid_zweige.py:1292-1422`). Reihenfolge: § 34 → § 35 → Kapital § 32d → § 32b (anders als
//! `gesamt_tarif`: dort steht § 32b VOR Kapital).
use domain::Euro;
use engine::zugriff::teil2::est::{TarifEingabe, tarif_est};
use engine::zugriff::teil2::gesamt::{GesamtfallEingabe, gesamt_tarifliche, gesamt_zve};
use engine::zugriff::teil2::sonstige::{ProgressionsvorbehaltEingabe, p32b_1};

use super::rechnen::{R, add, max0, sub};
use super::tarif::{Endstand, Lage, SolzInfo, kapital, p34_chooser, p35_credit};

/// § 32b im Rentner-Ring: wirkt auf das Ergebnis NACH dem Kapital. Rueckgabe
/// `(result', est_ohne_p35')`; der § 35-Kredit (nach der Formel OHNE § 32b, wie Python) kommt danach ab.
fn p32b_nach_kapital(
    l: &Lage<'_>,
    g2: &GesamtfallEingabe,
    result: Euro,
    est_ohne_p35: Euro,
    credit: Euro,
) -> R<(Euro, Euro)> {
    let pre = gesamt_tarifliche(g2, l.p)?;
    let est_ohne_tarifliche = sub(result, pre)?;
    let zve32b = gesamt_zve(g2, l.p)?;
    let (mut result, mut est_ohne_p35) = (result, est_ohne_p35);
    if zve32b.get() > 0 {
        let est_erhoeht = tarif_est(&TarifEingabe {
            vz: l.vz,
            veranlagung: l.veranlagung.tarif_strikt()?,
            zu_versteuerndes_einkommen: add(zve32b, l.pe_raw)?,
        })?;
        let t_32b = p32b_1(&ProgressionsvorbehaltEingabe {
            zu_versteuerndes_einkommen: zve32b,
            progressionseinkuenfte: l.pe_raw,
            est_auf_erhoehte_bemessung: est_erhoeht,
        })?;
        result = add(t_32b, est_ohne_tarifliche)?;
        // § 51a Abs. 2 S. 1: der § 32b-Zuschlag gehoert in die KiSt-Basis (Delta zur Tarif-ESt).
        est_ohne_p35 = add(est_ohne_p35, sub(t_32b, pre)?)?;
    }
    // § 35-Deckel-3 post-wrapper: der bereits berechnete Kredit.
    if credit.get() > 0 {
        result = max0(sub(result, credit)?);
    }
    Ok((result, est_ohne_p35))
}

/// `_festzusetzende_r(freibetrag)` (EURO). `info` traegt den SolZ-Zwischenstand, `ende` den
/// Endstand dieses Laufs (1:1 gesamt-Präzedenz; Python `kette_end_r`).
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
pub(super) fn festzusetzende(
    l: &Lage<'_>,
    g: &GesamtfallEingabe,
    freibetrag: Euro,
    info: &mut Option<SolzInfo>,
    ende: &mut Option<Endstand>,
) -> R<Euro> {
    let g2 = GesamtfallEingabe {
        freibetraege_kinder: freibetrag,
        ..*g
    };
    let mut g2 = p34_chooser(l, g2)?;
    let credit = p35_credit(l, &g2)?;
    if !l.pe_active() {
        g2.steuerermaessigungen = add(g2.steuerermaessigungen, credit)?;
    }
    // § 51a Abs. 2 S. 3: KiSt-Basis ohne § 35 (NEU rechnen, nicht zurueckaddieren).
    let est_ohne_p35 = if l.pe_active() || credit.get() == 0 {
        l.est(&g2)?
    } else {
        l.est(&GesamtfallEingabe {
            steuerermaessigungen: sub(g2.steuerermaessigungen, credit)?,
            ..g2
        })?
    };
    let einzel_lauf = freibetrag.get() > 0 || l.kinder == 0;
    // § 32d Abs. 6 Guenstigerpruefung: Kapitalertraege tariflich oder Abgeltungsteuer?
    let result = if l.kapitaleinkuenfte.get() <= 0 {
        l.est(&g2)?
    } else {
        let est_raw = l.est(&g2)?;
        let (_est_mit, kap) = kapital(l, &g2, est_raw)?;
        let result = add(est_raw, kap.kap_st_k)?;
        if einzel_lauf {
            *info = Some(SolzInfo {
                est_mit_fb: result,
                kap_st: kap.kap_st_k,
                est_ohne_p35,
            });
            l.mit_extras(|e| {
                e.kist_kap_cent = Some(kap.kist_kap_cent);
                e.kap_guenstiger_gewonnen = Some(kap.guenstiger);
            });
        }
        result
    };
    let (result, est_ohne_p35) = if l.pe_active() {
        p32b_nach_kapital(l, &g2, result, est_ohne_p35, credit)?
    } else {
        (result, est_ohne_p35)
    };
    // SolZ-Tracking: est_mit_fb wird HIER final gesetzt; kap_st bleibt, wenn der Kapital-Zweig es
    // gesetzt hat (sonst 0).
    if einzel_lauf {
        *info = Some(SolzInfo {
            est_mit_fb: result,
            kap_st: info.map_or(Euro::new(0), |i| i.kap_st),
            est_ohne_p35,
        });
    }
    // Endstand NACH dem § 32b/§ 35-Wrapper: dieser `result` ist der zurueckgegebene Wert, und
    // `g2` ist das Dict, auf dem die oberen drei Stufen gerechnet werden (1:1 Python).
    *ende = Some(Endstand { g2, wert: result });
    Ok(result)
}
