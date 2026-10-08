//! `_zweig_festzusetzende_est_gesamt` (`bescheid_zweige.py:414-1069`), Teil 1: der Aufbau des
//! Gesamtfalls `g` aus Einkuenften, Freibetraegen und Abzuegen. Der Tarif-Teil (`_festzusetzende`,
//! § 31, SolZ, KiSt) steht in `gesamt_tarif.rs`.
use domain::{Euro, PyWert, Vz};
use engine::zugriff::teil1::einkuenfte::{
    einkuenfte_nichtselbststaendig, p16_4_freibetrag, p21_2_verbilligt, vermietung_einkuenfte,
    EinkuenfteNichtselbststaendigEingabe, P164FreibetragEingabe, P212VerbilligtEingabe,
    VermietungEinkuenfteEingabe,
};
use engine::zugriff::teil1::ermaessigungen::{
    p24a_altersentlastung, p24b_entlastung, P24aAltersentlastungEingabe, P24bEntlastungEingabe,
};
use engine::zugriff::teil1::sonderausgaben::p10_4b_erstattungsueberhang;
use engine::zugriff::teil1::sonderausgaben::P10KistEingabe;
use engine::zugriff::teil1::werbungskosten::{werbungskosten_n, WerbungskostenNEingabe};
use engine::zugriff::teil2::gesamt::{gesamt_gde, GesamtfallEingabe};
use engine::zugriff::teil2::p33::{
    behinderten_pb, hinterbliebenen_pb, pflege_pb, BehindertenPbEingabe, HinterbliebenenPbEingabe,
    PflegePbEingabe,
};
use engine::zugriff::teil2::rente::{
    einkuenfte_versorgung, EinkuenfteVersorgungEingabe, VersorgungsfreibetragEingabe,
};
use engine::zugriff::teil2::sonstige::p22_nr3_einkuenfte;
use intervall::Slots;

use super::rechnen::{add, max0, sub, summe_euro, R};
use super::tarif::{leerer_gesamtfall, rahmen, Lage, Modus, P35};
use super::wk::{am_gesamt, ep_eingabe, mit_unfallkosten, wk_teile};
use super::{gesamt_tarif, slot, Marke, Ring, VeranlagungWert};
use crate::abzuege::{p33b_kind_pauschbetraege, shared_steuer_sonder_agb};
use crate::einkuenfte::{
    gewinn_partner_anteil, laufender_gewinn, p20_kapitaleinkuenfte, p23_ansonsten_einkuenfte,
    p35_summen, progressionseinkuenfte, shared_dba_sonstige,
};
use crate::{
    cent_zu_euro, feld_euro_oder_null, feld_int_oder_null, ist_false, ist_true, py_int, summe,
    wert, zahl_int, Felder,
};

/// § 21 Ueberschuss EINES Objekts (Einnahmen − Werbungskosten), Naht-CENT → EURO. Kein Floor pro
/// Objekt (horizontaler Verlustausgleich innerhalb § 21). `fi` ist die Instanz auf die Basis-`feld_id`
/// normiert oder der ganze Snapshot (Basis-Fallback).
fn vv_objekt(fi: &Felder) -> R<Euro> {
    let ci = |k: &str| feld_euro_oder_null(fi, k);
    let vv_voll = vermietung_einkuenfte(&VermietungEinkuenfteEingabe {
        einnahmen: ci("vv_einnahmen")?,
        gebaeude_afa: ci("vv_gebaeude_afa")?,
        schuldzinsen: ci("vv_schuldzinsen")?,
        erhaltungsaufwand: ci("vv_erhaltungsaufwand")?,
        sonstige_werbungskosten: ci("vv_sonstige_wk")?,
    })?;
    // § 21 Abs. 2: verbilligte Wohnraumvermietung (Entgelt < 66 %) → WK nur anteilig.
    let quote_raw = wert(fi, "vv_entgelt_quote_prozent").and_then(PyWert::zahl_ohne_bool);
    if quote_raw.is_some_and(|n| n.py_eq(&PyWert::Ganz(0))) {
        return ci("vv_einnahmen"); // unentgeltlich → Einnahmen ohne WK, kein Verlust
    }
    // PARITÄT: fail-open default — absent = 100 (nicht verbilligt).
    let quote = match quote_raw {
        Some(n) => zahl_int(n)?,
        None => 100,
    };
    let tatbestand = !ist_false(wert(fi, "vv_wohnzwecke")) && !ist_false(wert(fi, "vv_auf_dauer"));
    if tatbestand && 0 < quote && quote < 66 {
        let wk_voll = cent_zu_euro(summe(&[
            feld_int_oder_null(fi, "vv_gebaeude_afa")?,
            feld_int_oder_null(fi, "vv_schuldzinsen")?,
            feld_int_oder_null(fi, "vv_erhaltungsaufwand")?,
            feld_int_oder_null(fi, "vv_sonstige_wk")?,
        ])?);
        let wk_abziehbar = p21_2_verbilligt(&P212VerbilligtEingabe {
            werbungskosten: wk_voll,
            entgelt_quote_prozent: quote,
        })?;
        return add(vv_voll, sub(wk_voll, wk_abziehbar)?);
    }
    Ok(vv_voll)
}

/// § 21 Ueberschuss: stumpfe Σ ueber alle `vv_objekt`-Instanzen, ohne Store nur die Basis aus `f`.
fn vv_summe<Z: Marke>(r: &Ring<'_, Z>) -> R<Euro> {
    let q = r.q();
    if q.store.is_none() {
        return vv_objekt(r.f());
    }
    let mut vv = Euro::new(0);
    for inst in q.instanzen("vv_objekt")? {
        if q.zaehlt(&inst) {
            vv = add(vv, vv_objekt(&inst.felder)?)?;
        }
    }
    Ok(vv)
}

/// § 16 Veraeusserungsgewinn NACH § 16 Abs. 4-Freibetrag, bei 0 gefloort (kein Phantom-Verlust).
/// Der Freibetrag nur bei bestaetigtem S. 1 (55+/berufsunfaehig) UND erstmalig.
pub(crate) fn netto_vg(f: &Felder) -> R<Euro> {
    let vg = feld_euro_oder_null(f, "rentner_veraeusserungsgewinn")?;
    let gate = ist_true(wert(f, "rentner_alter_55_oder_berufsunfaehig"))
        && ist_true(wert(f, "rentner_freibetrag_erstmalig"));
    let fb = if gate {
        p16_4_freibetrag(&P164FreibetragEingabe {
            rentner_veraeusserungsgewinn: vg,
        })?
    } else {
        Euro::new(0)
    };
    Ok(max0(sub(vg, fb)?))
}

/// § 33b Behinderten-/Pflege-/Hinterbliebenen-Pauschbetrag Person A (`rentner_*`-Felder), EURO.
pub(super) fn pauschbetraege_a(f: &Felder, vz: Vz, p: &bindung::Params) -> R<Euro> {
    let b = behinderten_pb(
        &BehindertenPbEingabe {
            vz,
            grad_der_behinderung: feld_int_oder_null(f, "rentner_grad_der_behinderung")?,
            ist_hilflos_blind_taubblind: ist_true(wert(f, "rentner_hilflos_blind_taubblind")),
        },
        p,
    )?;
    let pf = pflege_pb(
        &PflegePbEingabe {
            vz,
            pflegegrad: feld_int_oder_null(f, "rentner_pflegegrad")?,
            ist_hilflos: ist_true(wert(f, "rentner_gepflegter_hilflos")),
        },
        p,
    )?;
    let h = hinterbliebenen_pb(
        &HinterbliebenenPbEingabe {
            vz,
            hat_hinterbliebenenbezuege: ist_true(wert(f, "rentner_hinterbliebenenbezuege")),
        },
        p,
    )?;
    summe_euro(&[b, pf, h])
}

/// § 33b Behinderten-Pauschbetrag des Ehegatten (nur bei Zusammenveranlagung addiert).
pub(super) fn pauschbetrag_partner(f: &Felder, vz: Vz, p: &bindung::Params) -> R<Euro> {
    Ok(behinderten_pb(
        &BehindertenPbEingabe {
            vz,
            grad_der_behinderung: feld_int_oder_null(f, "rentner_grad_der_behinderung_partner")?,
            ist_hilflos_blind_taubblind: ist_true(wert(
                f,
                "rentner_hilflos_blind_taubblind_partner",
            )),
        },
        p,
    )?)
}

/// § 24a Altersentlastung Person A/B und § 24b Entlastungsbetrag: `(alt24a_a, alt24a_b, ent24b)`.
pub(super) fn entlastungen(
    f: &Felder,
    vz: Vz,
    p: &bindung::Params,
    zusammen: bool,
    positive_andere: Euro,
) -> R<(Euro, Euro, Euro)> {
    let jahr = i64::from(vz.jahr());
    let alt_a = p24a_altersentlastung(
        &P24aAltersentlastungEingabe {
            veranlagungszeitraum: jahr,
            geburtsjahr: feld_int_oder_null(f, "geburtsjahr")?,
            arbeitslohn: feld_euro_oder_null(f, "bruttoarbeitslohn")?,
            positive_andere_einkuenfte: positive_andere,
        },
        p,
    )?;
    // § 24a PER PERSON: der Ehegatte hat eigene Kohorte und Bemessung.
    let alt_b = if zusammen {
        p24a_altersentlastung(
            &P24aAltersentlastungEingabe {
                veranlagungszeitraum: jahr,
                geburtsjahr: feld_int_oder_null(f, "geburtsjahr_partner")?,
                arbeitslohn: feld_euro_oder_null(f, "bruttoarbeitslohn_partner")?,
                positive_andere_einkuenfte: Euro::new(0),
            },
            p,
        )?
    } else {
        Euro::new(0)
    };
    Ok((alt_a, alt_b, entlastung_24b(f)?))
}

/// § 24b Entlastungsbetrag Alleinerziehende (`fam_alleinstehend` ist das § 24b-Abs.-3-Flag).
pub(super) fn entlastung_24b(f: &Felder) -> R<Euro> {
    Ok(p24b_entlastung(&P24bEntlastungEingabe {
        alleinstehend: ist_true(wert(f, "fam_alleinstehend")),
        anzahl_kinder: feld_int_oder_null(f, "fam_anzahl_kinder")?,
        monate_ohne_voraussetzung: feld_int_oder_null(f, "fam_monate_ohne_voraussetzung")?,
    })?)
}

/// § 10 Abs. 4b S. 3: Erstattungsueberhang der Kirchensteuer (erhoeht den `GdE`).
pub(super) fn kist_ueberhang(f: &Felder) -> R<Euro> {
    Ok(p10_4b_erstattungsueberhang(&P10KistEingabe {
        gezahlte_kirchensteuer: feld_euro_oder_null(f, "kist_gezahlt")?,
        erstattete_kirchensteuer: feld_euro_oder_null(f, "kist_erstattet")?,
    })?)
}

/// `p22_nr3_einkuenfte` (Freigrenze 256 EUR): `nr3 // 100`-Umrechnung, nur bei `nr3 != 0`.
pub(super) fn nr3_euro(f: &Felder) -> R<Euro> {
    let nr3 = feld_int_oder_null(f, "p22_nr3_einkuenfte")?;
    if nr3 == 0 {
        return Ok(Euro::new(0));
    }
    Ok(p22_nr3_einkuenfte(domain::Cent::new(nr3)).floor_euro())
}

/// § 19-Einkuenfte Person A (+B) samt Versorgungsbezuegen (§ 19 Abs. 2) in EURO.
fn einkuenfte_ns<Z: Marke>(r: &Ring<'_, Z>, slots: &Slots, ns_wk: Euro, zusammen: bool) -> R<Euro> {
    let lohn = Euro::new(py_int(slot(slots, "bruttoarbeitslohn")?)?.div_euclid(100));
    einkuenfte_ns_aus_lohn(r.f(), r.vz(), r.p(), lohn, ns_wk, zusammen)
}

/// Die Felder der Versorgungsbezuege EINER Person (§ 19 Abs. 2 `EStG`) und das Feld des Grades der Behinderung, das IHR
/// Alters-Gate setzt. Die Namen stehen als Literale: der Scanner hinter `RING_BETRAGSFELDER`
/// (`bescheid/tests/ring_scheiben.rs`) liest den Quelltext des Rings.
struct VersorgungsFelder {
    jahresrente: &'static str,
    bemessungsgrundlage: &'static str,
    beginn_jahr: &'static str,
    art: &'static str,
    alter_bei_beginn: &'static str,
    grad_der_behinderung: &'static str,
}

/// Person A.
const VERSORGUNG_A: VersorgungsFelder = VersorgungsFelder {
    jahresrente: "versorgung_jahresrente",
    bemessungsgrundlage: "versorgung_bemessungsgrundlage",
    beginn_jahr: "versorgung_beginn_jahr",
    art: "versorgung_art",
    alter_bei_beginn: "versorgung_alter_bei_beginn",
    grad_der_behinderung: "rentner_grad_der_behinderung",
};

/// Person B (§ 26b), Abweichung Nr. 33: Python kennt diese Felder nicht, nur die Rentner-Scheibe fragt sie.
const VERSORGUNG_B: VersorgungsFelder = VersorgungsFelder {
    jahresrente: "versorgung_jahresrente_partner",
    bemessungsgrundlage: "versorgung_bemessungsgrundlage_partner",
    beginn_jahr: "versorgung_beginn_jahr_partner",
    art: "versorgung_art_partner",
    alter_bei_beginn: "versorgung_alter_bei_beginn_partner",
    grad_der_behinderung: "rentner_grad_der_behinderung_partner",
};

/// Der Versorgungsbezug einer Person, wie der Ring ihn liest.
struct Versorgungsbezug {
    jahresrente: i64,
    bemessungsgrundlage: i64,
    beginn_jahr: i64,
    /// Jahresrente, Bemessungsgrundlage und Beginnjahr alle gesetzt.
    versorgt: bool,
    /// § 19 Abs. 2 S. 2 Nr. 2 Alters-Gate erfuellt (ohne Gate: ja).
    gate_erfuellt: bool,
}

fn versorgungsbezug(f: &Felder, n: &VersorgungsFelder) -> R<Versorgungsbezug> {
    let c = |k: &str| feld_int_oder_null(f, k);
    let jahresrente = c(n.jahresrente)?;
    let bemessungsgrundlage = c(n.bemessungsgrundlage)?;
    let beginn_jahr = c(n.beginn_jahr)?;
    let alter = c(n.alter_bei_beginn)?;
    // § 19 Abs. 2 S. 2 Nr. 2 Alters-Gate: nur bei altersgrenze_sonstige (63. Lj, 60. bei GdB >= 50).
    let mut gate_erfuellt = true;
    if matches!(wert(f, n.art), Some(PyWert::Text(s)) if s == "altersgrenze_sonstige") && alter > 0
    {
        let grenze = if c(n.grad_der_behinderung)? >= 50 {
            60
        } else {
            63
        };
        gate_erfuellt = alter >= grenze;
    }
    Ok(Versorgungsbezug {
        jahresrente,
        bemessungsgrundlage,
        beginn_jahr,
        versorgt: jahresrente > 0 && bemessungsgrundlage > 0 && beginn_jahr > 0,
        gate_erfuellt,
    })
}

/// Die Einkuenfte aus einem Versorgungsbezug nach Versorgungsfreibetrag, Zuschlag und Pauschbetrag.
fn einkuenfte_aus_versorgung(v: &Versorgungsbezug, p: &bindung::Params) -> R<Euro> {
    Ok(einkuenfte_versorgung(
        &EinkuenfteVersorgungEingabe {
            versorgung_jahresrente: cent_zu_euro(v.jahresrente),
            freibetrag: VersorgungsfreibetragEingabe {
                bemessungsgrundlage: cent_zu_euro(v.bemessungsgrundlage),
                beginn_jahr: v.beginn_jahr,
            },
        },
        p,
    )?)
}

/// [`einkuenfte_ns`] mit dem Bruttoarbeitslohn Person A als Argument. Der Rentner-Ring liest keine
/// Slots und reicht den Lohn aus dem Feld herein; Versorgungsbezuege und Alters-Gate sind dieselben.
/// Bei Zusammenveranlagung (`zusammen`) kommen Lohn und Versorgung der Person B in DIESELBE Summe.
pub(super) fn einkuenfte_ns_aus_lohn(
    f: &Felder,
    vz: Vz,
    p: &bindung::Params,
    lohn: Euro,
    ns_wk: Euro,
    zusammen: bool,
) -> R<Euro> {
    let a = versorgungsbezug(f, &VERSORGUNG_A)?;
    let mut basis = lohn.get();
    if a.versorgt && !a.gate_erfuellt {
        basis = crate::plus(basis, a.jahresrente.div_euclid(100))?;
    }
    let mut ns = einkuenfte_nichtselbststaendig(&EinkuenfteNichtselbststaendigEingabe {
        veranlagungszeitraum: vz,
        bruttoarbeitslohn: Euro::new(basis),
        werbungskosten: ns_wk,
    })?;
    // Person B (§ 26b): § 19-Einkuenfte des Ehegatten in DIESELBE Summe, Person-B-WK MVP 0. Ein Versorgungsbezug vor dem
    // Alters-Gate zaehlt wie bei Person A als Arbeitslohn.
    let b = if zusammen {
        let b = versorgungsbezug(f, &VERSORGUNG_B)?;
        let mut basis_b = feld_euro_oder_null(f, "bruttoarbeitslohn_partner")?.get();
        if b.versorgt && !b.gate_erfuellt {
            basis_b = crate::plus(basis_b, b.jahresrente.div_euclid(100))?;
        }
        let einkuenfte_b = einkuenfte_nichtselbststaendig(&EinkuenfteNichtselbststaendigEingabe {
            veranlagungszeitraum: vz,
            bruttoarbeitslohn: Euro::new(basis_b),
            werbungskosten: Euro::new(0),
        })?;
        ns = add(ns, einkuenfte_b)?;
        Some(b)
    } else {
        None
    };
    if a.versorgt && a.gate_erfuellt {
        ns = add(ns, einkuenfte_aus_versorgung(&a, p)?)?;
    }
    if let Some(b) = b.filter(|b| b.versorgt && b.gate_erfuellt) {
        ns = add(ns, einkuenfte_aus_versorgung(&b, p)?)?;
    }
    Ok(ns)
}

/// § 9-Werbungskosten Person A fuer den gemischten § 19-Fall (EP, dHf, Verpflegung, Uebernachtung,
/// Arbeitsmittel-GWG) plus die mehrjaehrige AM-AfA.
fn ns_werbungskosten<Z: Marke>(r: &Ring<'_, Z>, slots: &Slots) -> R<Euro> {
    let (f, vz, p) = (r.f(), r.vz(), r.p());
    let ep = ep_eingabe(vz, slots)?;
    let teile = wk_teile(f, vz)?;
    let (gwg, afa) = am_gesamt(f)?;
    let wk = werbungskosten_n(
        &WerbungskostenNEingabe {
            entfernung: Some(ep),
            doppelte_haushaltsfuehrung: teile.doppelte_haushaltsfuehrung,
            verpflegung: teile.verpflegung,
            uebernachtung: teile.uebernachtung,
            am_anschaffungskosten: gwg,
        },
        p,
    )?;
    add(mit_unfallkosten(f, wk)?, afa)
}

/// Einkommensteuer-Slots fuer die Altersvorsorge § 10 Abs. 1 Nr. 2 (VOR): `(gesamtbeitraege, ag)`.
pub(super) fn vorsorge_slots(f: &Felder, zusammen: bool) -> R<(Euro, Euro)> {
    let c = |k: &str| feld_int_oder_null(f, k);
    let mut gesamt = cent_zu_euro(summe(&[
        c("vor_an_anteil_rv")?,
        c("vor_ag_anteil_rv")?,
        c("vor_rv_ausserhalb_lstb")?,
    ])?);
    let mut ag = feld_euro_oder_null(f, "vor_ag_anteil_rv")?;
    // Person-B-Basisvorsorge: ADDITIV in dieselben Summen-Slots (Hoechstbetrag nicht verdoppelt).
    if zusammen {
        let partner = cent_zu_euro(summe(&[
            c("vor_an_anteil_rv_partner")?,
            c("vor_ag_anteil_rv_partner")?,
            c("vor_rv_ausserhalb_lstb_partner")?,
        ])?);
        gesamt = add(gesamt, partner)?;
        ag = add(ag, feld_euro_oder_null(f, "vor_ag_anteil_rv_partner")?)?;
    }
    Ok((gesamt, ag))
}

/// § 35: `(messbetrag_a, hebesatz_a, zaehler_a)` — der Zaehler ist der laufende Gewerbe-Gewinn nur bei
/// `gewinn_betriebsart == gewerbe`, sonst der § 15-Mitunternehmeranteil.
pub(super) fn p35_person_a(f: &Felder, laufender: Euro, mitu: Euro) -> R<(Euro, i64, Euro)> {
    let gewerbe = matches!(wert(f, "gewinn_betriebsart"), Some(PyWert::Text(s)) if s == "gewerbe");
    let zaehler = max0(if gewerbe { laufender } else { mitu });
    Ok((
        feld_euro_oder_null(f, "gewst_messbetrag")?,
        feld_int_oder_null(f, "gewst_hebesatz")?,
        zaehler,
    ))
}

/// Der Zweig `festzusetzende_est_gesamt`, EURO.
///
/// # Errors
/// Slot-, Instanz-, Accessor- und Ueberlauf-Fehler.
pub(super) fn festzusetzende_est_gesamt<Z: Marke>(r: &Ring<'_, Z>, slots: &Slots) -> R<Euro> {
    let (f, vz, p) = (r.f(), r.vz(), r.p());
    let q = r.q();
    let vv = vv_summe(r)?;
    let veranlagung =
        VeranlagungWert::aus(domain::Lage::veranlagung(Some(slot(slots, "veranlagung")?)));
    let zusammen = veranlagung.zusammen();
    let mut g = GesamtfallEingabe {
        einkuenfte_vermietung: vv,
        ..leerer_gesamtfall(vz, zusammen)
    };
    let ns_wk = ns_werbungskosten(r, slots)?;
    let ns = einkuenfte_ns(r, slots, ns_wk, zusammen)?;
    g.einkuenfte_nichtselbststaendig = ns;
    // §§ 13-18 Gewinn: laufend + § 16-vg netto + Gewinn des Ehegatten (Stufe 2 der Partnerachse).
    let netto_vg = netto_vg(f)?;
    let (laufend, mitu) = laufender_gewinn(f, &q)?;
    let (gewinn_partner, _mitu_partner, netto_vg_partner) = gewinn_partner_anteil(f)?;
    g.einkuenfte_gewinn = summe_euro(&[laufend, netto_vg, gewinn_partner])?;
    // § 24a/§ 24b mindern den GdE VOR den Abzuegen.
    let positive_andere = max0(summe_euro(&[
        vv,
        g.einkuenfte_gewinn,
        g.einkuenfte_sonstige,
    ])?);
    let (alt_a, alt_b, ent24b) = entlastungen(f, vz, p, zusammen, positive_andere)?;
    g.altersentlastungsbetrag = add(alt_a, alt_b)?;
    g.entlastungsbetrag_alleinerziehende = ent24b;
    (
        g.vorsorge_gesamtbeitraege_inkl_ag,
        g.vorsorge_ag_anteil_steuerfrei,
    ) = vorsorge_slots(f, zusammen)?;
    let mut gde = gesamt_gde(&gde_fall(&g), p)?;
    let ueberhang = kist_ueberhang(f)?;
    gde = add(gde, ueberhang)?;
    // § 33b: Pauschbetraege Person A, Kind-Uebertragung (Abs. 5), Ehegatte.
    let mut ausserg = pauschbetraege_a(f, vz, p)?;
    ausserg = add(ausserg, p33b_kind_pauschbetraege(&q, vz, p)?)?;
    if zusammen {
        ausserg = add(ausserg, pauschbetrag_partner(f, vz, p)?)?;
    }
    let s = shared_steuer_sonder_agb(gde, ausserg, veranlagung.domain(), f, vz, &q, p)?;
    g.steuerermaessigungen = s.steuerermaessigungen;
    g.sonderausgaben = s.sonderausgaben;
    g.aussergewoehnliche_belastungen = s.aussergewoehnliche_belastungen;
    let kapitaleinkuenfte = p20_kapitaleinkuenfte(f, zusammen, vz, p)?;
    // § 23 + § 22 Nr. 3 in einkuenfte_sonstige; der KiSt-Ueberhang laeuft mangels Slot dort mit.
    g.einkuenfte_sonstige = add(p23_ansonsten_einkuenfte(&q)?, nr3_euro(f)?)?;
    g.einkuenfte_sonstige = add(g.einkuenfte_sonstige, ueberhang)?;
    // § 10d Abs. 2 Verlustvortrag, § 33a, DBA (§ 34c): auf der vollen GdE.
    let gde_p10d = gesamt_gde(&gde_fall(&g), p)?;
    let dba = shared_dba_sonstige(&mut g, gde_p10d, veranlagung.domain(), f, vz, p)?;
    // § 35 GewSt-Anrechnung: Basiswerte, freibetrag-unabhaengig.
    let (mb, hs, zaehler) = p35_person_a(f, laufend, mitu)?;
    let (mb_ges, z_ges, gezahlt) = p35_summen(f, mb, hs, zaehler)?;
    // Der KiSt-Ueberhang ist keine Einkunft (§ 35 Abs. 1 S. 2) und bleibt aus dem Nenner.
    let nenner = summe_euro(&[
        max0(ns),
        max0(vv),
        max0(sub(g.einkuenfte_sonstige, ueberhang)?),
        max0(g.einkuenfte_gewinn),
    ])?;
    let lage = Lage {
        vz,
        p,
        f,
        veranlagung,
        netto_vg,
        netto_vg_partner,
        dba_anrechnung: dba.dba_anrechnung,
        p35: P35 {
            messbetrag_ges: mb_ges,
            zaehler_ges: z_ges,
            gezahlt,
            nenner,
        },
        pe_raw: progressionseinkuenfte(f, &dba)?,
        kapitaleinkuenfte,
        kinder: feld_int_oder_null(f, "fam_anzahl_kinder")?,
        q: r.q(),
        extras: r.a.extras,
    };
    rahmen(&lage, r.a.solz, Modus::Gesamt, |fb, info, ende| {
        gesamt_tarif::festzusetzende(&lage, &g, fb, info, ende)
    })
}

/// Der Gesamtfall fuer die `GdE`-Zwillinge: nur die Einkunftsarten und Entlastungen (Python baut
/// dafuer ein eigenes dict — Abzuege fehlen dort, sie aendern die `GdE` nicht).
pub(super) fn gde_fall(g: &GesamtfallEingabe) -> GesamtfallEingabe {
    GesamtfallEingabe {
        einkuenfte_nichtselbststaendig: g.einkuenfte_nichtselbststaendig,
        einkuenfte_vermietung: g.einkuenfte_vermietung,
        einkuenfte_gewinn: g.einkuenfte_gewinn,
        einkuenfte_sonstige: g.einkuenfte_sonstige,
        altersentlastungsbetrag: g.altersentlastungsbetrag,
        entlastungsbetrag_alleinerziehende: g.entlastungsbetrag_alleinerziehende,
        ..leerer_gesamtfall(g.vz, g.zusammenveranlagung)
    }
}
