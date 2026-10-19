//! `_zweig_festzusetzende_est` (`bescheid_zweige.py:208-411`): § 2 Gesamtsteuer, reiner AN-Fall
//! (Scheibe `an_gesamt`) — Werbungskosten, KV/PV, Vorsorge, SolZ, KiSt und § 101-Praemie.
use domain::{Cent, Euro};
use engine::zugriff::teil1::einkuenfte::{
    einkuenfte_nichtselbststaendig, EinkuenfteNichtselbststaendigEingabe,
};
use engine::zugriff::teil1::mobilitaetspraemie::{p101_mobilitaetspraemie_cent, P101Eingabe};
use engine::zugriff::teil1::pauschbetraege::{arbeitnehmer_pauschbetrag, grundfreibetrag};
use engine::zugriff::teil1::sonderausgaben::{p10_kv_pv, P10KvPvEingabe};
use engine::zugriff::teil1::werbungskosten::{
    ep_ab_21km, werbungskosten_n, WerbungskostenNEingabe,
};
use engine::zugriff::teil2::est::{
    est_einzel, est_einzel_zve, est_zusammen, EstEinzelEingabe, EstZusammenEingabe,
};
use engine::zugriff::teil2::gesamt::sonderausgaben_abzug;
use engine::zugriff::teil2::solz::{solz, SolzEingabe};
use intervall::Slots;

use super::kist_anteil::{kist_cent, EinkuenfteJePerson};
use super::rechnen::{add, R};
use super::wk::{am_an_gesamt, ep_eingabe, mit_sofortabzug_im_band, mit_unfallkosten, wk_teile};
use super::{kist_konfession, slot, Marke, Ring};
use crate::abzuege::{basisbeitrag_nach_zuschuss, kind_kv_pv_summe};
use crate::{
    cent_zu_euro, feld_euro_oder_null, feld_int_oder_null, ist_true, ist_zusammen, py_int, summe,
    wert, Felder,
};

/// § 10 Abs. 1 Nr. 3/3a KV/PV-Vorsorge Person A oder B: eigener Abs.-4-Hoechstbetrag.
fn kv_pv(f: &Felder, suffix: &str, kind_cent: i64) -> R<Euro> {
    let c = |k: &str| feld_int_oder_null(f, &format!("{k}{suffix}"));
    // Der Basisbeitrag nach Abzug der steuerfreien Arbeitgeberzuschuesse (Abweichung Nr. 92): ein Rechenpfad mit `gesamt` und `rentner_gesamt`.
    let basis = basisbeitrag_nach_zuschuss(f, suffix, kind_cent)?;
    let weitere = summe(&[
        c("vorsorge_arbeitslosenversicherung")?,
        c("vorsorge_erwerbsunfaehigkeit")?,
        c("vorsorge_unfall_haftpflicht")?,
        c("vorsorge_rv_alt_mit_ueberschuss")?,
        c("vorsorge_rv_alt_ohne_ueberschuss")?,
    ])?;
    Ok(p10_kv_pv(&P10KvPvEingabe {
        basis_kv_pv: cent_zu_euro(basis),
        weitere_vorsorgeaufwendungen: cent_zu_euro(weitere),
        mit_anspruch_auf_zuschuss: ist_true(wert(f, &format!("mit_anspruch_auf_zuschuss{suffix}"))),
    })?)
}

/// § 10 Abs. 1 Nr. 2, Abs. 3 `_vorsorge_abzug`: `max(0, min(beitraege, Hoechstbeitrag) - ag)`,
/// 0 ohne Beitraege. EURO.
pub(super) fn vorsorge_abzug(
    beitraege: Euro,
    ag_steuerfrei: Euro,
    p: &bindung::Params,
    vz: domain::Vz,
) -> R<Euro> {
    if beitraege.get() == 0 {
        return Ok(Euro::new(0));
    }
    let hb = p
        .vorsorge_hoechstbeitrag(vz)
        .map_err(engine::zugriff::teil1::fehler::EngineFehler::from)?;
    Ok(Euro::new(
        crate::minus(beitraege.get().min(hb.get()), ag_steuerfrei.get())?.max(0),
    ))
}

/// Die Summe der Einkuenfte je Person fuer die Kirchensteuer bei Zusammenveranlagung (Abweichung Nr. 94): Lohn minus Werbungskosten,
/// mindestens der Pauschbetrag; Person B ohne eigene Werbungskosten, wie in `est_zusammen`. Ohne Zusammenveranlagung zaehlt sie nicht.
fn kist_einkuenfte(
    f: &Felder,
    vz: domain::Vz,
    brutto_a: Euro,
    wk_a: Euro,
    zusammen: bool,
) -> R<EinkuenfteJePerson> {
    if !zusammen {
        return Ok(EinkuenfteJePerson { a: Euro::new(0), b: Euro::new(0) });
    }
    let je = |lohn, werbungskosten| {
        einkuenfte_nichtselbststaendig(&EinkuenfteNichtselbststaendigEingabe {
            veranlagungszeitraum: vz,
            bruttoarbeitslohn: lohn,
            werbungskosten,
        })
    };
    Ok(EinkuenfteJePerson {
        a: je(brutto_a, wk_a)?,
        b: je(feld_euro_oder_null(f, "bruttoarbeitslohn_partner")?, Euro::new(0))?,
    })
}

/// Der Zweig `festzusetzende_est`, EURO. Schreibt SolZ, KiSt und die § 101-Praemie in die Ausgaben.
///
/// # Errors
/// Slot-, Accessor- und Ueberlauf-Fehler (Python-Klassen wie in [`crate::BescheidFehler`]).
pub(super) fn festzusetzende_est<Z: Marke>(r: &Ring<'_, Z>, slots: &Slots) -> R<Euro> {
    let (f, vz, p) = (r.f(), r.vz(), r.p());
    // § 9 Werbungskosten Person A: EP + dHf + Verpflegung + Uebernachtung + Arbeitsmittel, roh.
    let ep = ep_eingabe(vz, slots)?;
    let teile = wk_teile(f, vz)?;
    let wk = werbungskosten_n(
        &WerbungskostenNEingabe {
            entfernung: Some(ep),
            doppelte_haushaltsfuehrung: teile.doppelte_haushaltsfuehrung,
            verpflegung: teile.verpflegung,
            uebernachtung: teile.uebernachtung,
            am_anschaffungskosten: am_an_gesamt(f)?,
        },
        p,
    )?;
    // Unfallkosten auf dem Weg zur Arbeit: neben der Pauschale, ausserhalb ihres Hoechstbetrags (Abweichung Nr. 28).
    let wk = mit_unfallkosten(f, wk)?;
    // Arbeitsmittel mit Bruttopreis ueber 800 EUR, netto aber unter der Grenze (Abweichung Nr. 88): der Catala-Scope hat sie nicht gesehen.
    let wk = mit_sofortabzug_im_band(f, wk)?;
    // Kind-Beitraege in DENSELBEN Abs.-4-Deckel (CENT, direkt addiert).
    let kv_pv_a = kv_pv(f, "", kind_kv_pv_summe(&r.q())?.get())?;
    let zusammen = ist_zusammen(f);
    let brutto_a = cent_zu_euro(py_int(slot(slots, "bruttoarbeitslohn")?)?);
    // Abweichung Nr. 78: In dieser Scheibe sind alle Sonderausgaben Vorsorge (Nr. 2, 3, 3a); der Pauschbetrag steht daneben (§ 10c). Die
    // Catala-Scopes bilden weiter `max(Sonderausgaben; Pauschbetrag)`; der fertige Abzug enthaelt den Pauschbetrag schon, das `max`
    // aendert dann nichts.
    // ponytail: der Scope kennt keine eigene Vorsorge-Eingabe; Upgrade: Eingabe `vorsorge` im Catala-Scope und `make catala-c`.
    let (est, so_einzel) = if zusammen {
        let kv_pv_b = kv_pv(f, "_partner", 0)?;
        let est = est_zusammen(&EstZusammenEingabe {
            vz,
            bruttoarbeitslohn_a: brutto_a,
            bruttoarbeitslohn_b: feld_euro_oder_null(f, "bruttoarbeitslohn_partner")?,
            werbungskosten_a: wk,
            werbungskosten_b: Euro::new(0),
            sonderausgaben_gemeinsam: sonderausgaben_abzug(
                vz,
                true,
                Euro::new(0),
                add(kv_pv_a, kv_pv_b)?,
                p,
            )?,
        })?;
        (est, None)
    } else {
        // Altersvorsorge: die VOR-Einzelfelder DIREKT aus dem Snapshot (Kuerzung NACH dem Cap).
        let gesamt = cent_zu_euro(summe(&[
            feld_int_oder_null(f, "vor_an_anteil_rv")?,
            feld_int_oder_null(f, "vor_ag_anteil_rv")?,
            feld_int_oder_null(f, "vor_rv_ausserhalb_lstb")?,
        ])?);
        let ag = feld_euro_oder_null(f, "vor_ag_anteil_rv")?;
        let so = sonderausgaben_abzug(
            vz,
            false,
            Euro::new(0),
            add(vorsorge_abzug(gesamt, ag, p, vz)?, kv_pv_a)?,
            p,
        )?;
        // Der Slot "veranlagung" muss da sein (Python: `slots["veranlagung"]`), sein Wert zaehlt hier nicht.
        slot(slots, "veranlagung")?;
        let est = est_einzel(&EstEinzelEingabe {
            vz,
            bruttoarbeitslohn: brutto_a,
            werbungskosten: wk,
            sonderausgaben: so,
        })?;
        (est, Some(so))
    };
    // SolZ §3/§4: Basis = festzusetzende ESt (kein KiFB/§32d-Kapital im AN-Ring).
    if let Some(out) = r.a.solz {
        out.set(Some(solz(&SolzEingabe {
            vz,
            bemessungsgrundlage: est,
            kapital_steuer: Euro::new(0), // PARITÄT: fail-open default — Python setzt kein kapital_steuer
            splitting: zusammen,
        })?));
    }
    // KiSt § 51a: Massstabsteuer = festgesetzte ESt; ohne Konfession bleibt der Schluessel absent. Bei Zusammenveranlagung und
    // beantworteter Konfession des Ehegatten nach Landesrecht (Abweichung Nr. 94): die Summe der Einkuenfte je Person ist hier Lohn
    // minus Werbungskosten, mindestens der Pauschbetrag (Person B ohne eigene Werbungskosten, wie in `est_zusammen`).
    if let (Some(extras), Some(konf)) = (r.a.extras, kist_konfession(f)) {
        let einkuenfte = kist_einkuenfte(f, vz, brutto_a, wk, zusammen)?;
        let k = kist_cent(f, vz, konf, est, zusammen, einkuenfte)?;
        extras.borrow_mut().kist_cent = Some(k);
    }
    // § 101 Mobilitaetspraemie: reiner AN, einzel, mit Pendlerstrecke (Stufe 1).
    if let (Some(extras), Some(so)) = (r.a.extras, so_einzel) {
        if py_int(slot(slots, "entfernung_km_roh")?)? > 0 {
            let ab21 = ep_ab_21km(&ep, p)?;
            let zve = est_einzel_zve(&EstEinzelEingabe {
                vz,
                bruttoarbeitslohn: brutto_a,
                werbungskosten: wk,
                sonderausgaben: so,
            })?;
            let praemie: Cent = p101_mobilitaetspraemie_cent(&P101Eingabe {
                entfernungspauschale_ab_21km: ab21,
                zu_versteuerndes_einkommen: zve,
                grundfreibetrag: grundfreibetrag(vz, p)?,
                ist_arbeitnehmer: true,
                werbungskosten_gesamt: wk,
                arbeitnehmer_pauschbetrag: arbeitnehmer_pauschbetrag(vz, p)?,
            })?;
            extras.borrow_mut().mobilitaetspraemie_cent = Some(praemie);
        }
    }
    Ok(est)
}
