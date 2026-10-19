//! `_zweig_festzusetzende_est_gesamt` (`bescheid_zweige.py:414-1069`), Teil 1: der Aufbau des
//! Gesamtfalls `g` aus Einkuenften, Freibetraegen und Abzuegen. Der Tarif-Teil (`_festzusetzende`,
//! § 31, SolZ, KiSt) steht in `gesamt_tarif.rs`.
use domain::{Euro, PyWert, Sperrgrund, Vz};
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
    einkuenfte_versorgung, renten_nach_pauschbetrag, EinkuenfteVersorgungEingabe,
    VersorgungsfreibetragEingabe,
};
use engine::zugriff::teil2::sonstige::p22_nr3_einkuenfte;
use intervall::Slots;

use super::rechnen::{add, max0, sub, summe_euro, R};
use super::tarif::{leerer_gesamtfall, rahmen, Lage, Modus, P35};
use super::wk::{am_gesamt, ep_eingabe, mit_sofortabzug_im_band, mit_unfallkosten, wk_teile};
use super::{gesamt_tarif, slot, Marke, Ring, VeranlagungWert};
use crate::abzuege::{p33b_kind_pauschbetraege, shared_steuer_sonder_agb};
use crate::einkuenfte::{
    dba_abzug_werbungskosten, gewinn_partner_anteil, laufender_gewinn, p20_kapitaleinkuenfte,
    p35_summen, progressionseinkuenfte, shared_dba_sonstige,
};
use crate::privatverkauf::p23_privatverkauf;
use crate::rav_bav::rav_bav_einkuenfte;
use crate::{
    cent_zu_euro, feld_euro_oder_null, feld_int_oder_null, ist_false, ist_true, py_int, summe,
    wert, zahl_int, BescheidFehler, Felder,
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
///
/// Die Bemessung des § 24a steht NUR hier und gilt je Person (Satz 4; Abweichung Nr. 62, Bezug Nr. 36): Arbeitslohn ohne
/// Versorgungsbezug der Person plus `max(0, uebrige Einkuenfte der Person)`. `uebrige_a` sind die uebrigen Einkuenfte von Person A
/// (der Aufrufer kennt sie: Vermietung, eigener Gewinn, Sonstige, Leistungen mit Leistungsmitteilung), NICHT gefloort. Die uebrigen
/// Einkuenfte von Person B sind ihr laufender und ihr § 16-Gewinn nach Freibetrag (`gewinn_partner_anteil`); sonst kennt das Modell
/// fuer B nichts. Der Gewinn des Ehegatten gehoert nie in die Bemessung von Person A.
pub(super) fn entlastungen(
    f: &Felder,
    vz: Vz,
    p: &bindung::Params,
    zusammen: bool,
    uebrige_a: Euro,
) -> R<(Euro, Euro, Euro)> {
    let jahr = i64::from(vz.jahr());
    // § 24a Satz 2 Nr. 1: Versorgungsbezuege (§ 19 Abs. 2) bleiben ausser Betracht. Der Lohn enthaelt den Bezug (Abweichung
    // Nr. 50), also geht er hier vom Lohn ab, soweit der Bescheid ihn als Versorgungsbezug rechnet.
    let alt_a = p24a_altersentlastung(
        &P24aAltersentlastungEingabe {
            veranlagungszeitraum: jahr,
            geburtsjahr: feld_int_oder_null(f, "geburtsjahr")?,
            arbeitslohn: Euro::new(arbeitslohn_ohne_bezug(
                feld_euro_oder_null(f, "bruttoarbeitslohn")?.get(),
                &versorgungsbezug(f, &VERSORGUNG_A)?,
            )?),
            positive_andere_einkuenfte: max0(uebrige_a),
        },
        p,
    )?;
    // § 24a PER PERSON (Satz 4): der Ehegatte hat eigene Kohorte und eigene Bemessung, auch aus seinem eigenen Gewinn.
    let alt_b = if zusammen {
        p24a_altersentlastung(
            &P24aAltersentlastungEingabe {
                veranlagungszeitraum: jahr,
                geburtsjahr: feld_int_oder_null(f, "geburtsjahr_partner")?,
                arbeitslohn: Euro::new(arbeitslohn_ohne_bezug(
                    feld_euro_oder_null(f, "bruttoarbeitslohn_partner")?.get(),
                    &versorgungsbezug(f, &VERSORGUNG_B)?,
                )?),
                positive_andere_einkuenfte: max0(gewinn_partner_anteil(f)?.0),
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

/// Die Felder der Versorgungsbezuege EINER Person (§ 19 Abs. 2 `EStG`) und die zwei Felder des Grades der Behinderung, der IHR
/// Alters-Gate setzt. Die Namen stehen als Literale: der Scanner hinter `RING_BETRAGSFELDER`
/// (`bescheid/tests/ring_scheiben.rs`) liest den Quelltext des Rings.
struct VersorgungsFelder {
    jahresrente: &'static str,
    bemessungsgrundlage: &'static str,
    beginn_jahr: &'static str,
    art: &'static str,
    alter_bei_beginn: &'static str,
    /// Der Grad, den die Rentner-Scheibe fragt (§ 33b-Block, immer gefragt).
    grad_der_behinderung: &'static str,
    /// Der Grad, den der Gesamtlauf fragt (Abweichung Nr. 80), nur bei einem Bezug der Art `altersgrenze_sonstige`. Der Ehegatte hat
    /// keinen: `gesamt` fuehrt keinen Bezug des Ehegatten, nur die Rentner-Scheibe, und die fragt seinen Grad immer.
    grad_gesamtfrage: Option<&'static str>,
    geburtsjahr: &'static str,
}

/// Person A.
const VERSORGUNG_A: VersorgungsFelder = VersorgungsFelder {
    jahresrente: "versorgung_jahresrente",
    bemessungsgrundlage: "versorgung_bemessungsgrundlage",
    beginn_jahr: "versorgung_beginn_jahr",
    art: "versorgung_art",
    alter_bei_beginn: "versorgung_alter_bei_beginn",
    grad_der_behinderung: "rentner_grad_der_behinderung",
    grad_gesamtfrage: Some("versorgung_grad_der_behinderung"),
    geburtsjahr: "geburtsjahr",
};

/// Person B (§ 26b), Abweichung Nr. 33: Python kennt diese Felder nicht, nur die Rentner-Scheibe fragt sie.
const VERSORGUNG_B: VersorgungsFelder = VersorgungsFelder {
    jahresrente: "versorgung_jahresrente_partner",
    bemessungsgrundlage: "versorgung_bemessungsgrundlage_partner",
    beginn_jahr: "versorgung_beginn_jahr_partner",
    art: "versorgung_art_partner",
    alter_bei_beginn: "versorgung_alter_bei_beginn_partner",
    grad_der_behinderung: "rentner_grad_der_behinderung_partner",
    grad_gesamtfrage: None,
    geburtsjahr: "geburtsjahr_partner",
};

/// § 19 Abs. 2 S. 2 Nr. 2 `EStG`: Bezuege wegen Erreichens einer Altersgrenze sind erst Versorgungsbezuege, wenn die Person das
/// 63. Lebensjahr vollendet hat, bei Schwerbehinderung (Grad der Behinderung ab 50) das 60.
fn altersgrenze_fuer_grad(grad_der_behinderung: i64) -> i64 {
    if grad_der_behinderung >= 50 {
        60
    } else {
        63
    }
}

/// Das Feld, das den Grad der Behinderung der Person tragen soll (Abweichung Nr. 80): das der Rentner-Scheibe, wenn es in der Akte
/// steht, sonst das, das der Gesamtlauf fragt (nur Person A). Steht keines da, ist es das der Rentner-Scheibe; der Aufrufer liest es
/// dann als fehlend.
fn grad_feld(f: &Felder, n: &VersorgungsFelder) -> &'static str {
    match n.grad_gesamtfrage {
        Some(gesamtfrage) if wert(f, n.grad_der_behinderung).is_none() => gesamtfrage,
        _ => n.grad_der_behinderung,
    }
}

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
        gate_erfuellt = alter >= altersgrenze_fuer_grad(c(grad_feld(f, n))?);
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

/// Der Arbeitslohn EINER Person ohne ihren Versorgungsbezug (Abweichung Nr. 50). Nr. 3 der Lohnsteuerbescheinigung, der
/// Bruttoarbeitslohn (`lohn`), ENTHAELT den Bezug aus Nr. 8 (Vordruck Anlage N Zeile 11: "im Bruttoarbeitslohn laut Zeile 5
/// enthalten"). Ein steuerbeguenstigter Bezug (Alters-Gate erfuellt) kommt als Versorgungseinkunft dazu und geht deshalb
/// vom Lohn ab. Ein Bezug vor dem Alters-Gate ist normaler Arbeitslohn und steckt schon in `lohn`: er zaehlt einmal. Der
/// Bezug kann nicht groesser sein als der Lohn; der Guard sperrt das vorher ([`Sperrgrund::VersorgungUeberLohn`]), kommt der
/// Fehler an, hat ein Aufrufer ohne Guard gerechnet. Python addiert beide Betraege.
fn arbeitslohn_ohne_bezug(lohn: i64, v: &Versorgungsbezug) -> R<i64> {
    let bezug = v.jahresrente.div_euclid(100);
    if bezug > lohn {
        return Err(BescheidFehler::VersorgungGesperrt(
            Sperrgrund::VersorgungUeberLohn,
        ));
    }
    if v.versorgt && v.gate_erfuellt {
        crate::minus(lohn, bezug)
    } else {
        Ok(lohn)
    }
}

/// Ist der Bezug einer Person groesser als ihr Bruttoarbeitslohn? `partner` waehlt Person B (`*_partner`). Der Guard
/// ([`Sperrgrund::VersorgungUeberLohn`]) und der Ring ([`arbeitslohn_ohne_bezug`]) rechnen mit derselben Zahl, in Euro.
pub(crate) fn versorgung_ueber_lohn(f: &Felder, partner: bool) -> R<bool> {
    let (lohn, bezug) = if partner {
        ("bruttoarbeitslohn_partner", &VERSORGUNG_B)
    } else {
        ("bruttoarbeitslohn", &VERSORGUNG_A)
    };
    let bezug = feld_int_oder_null(f, bezug.jahresrente)?.div_euclid(100);
    Ok(bezug > feld_euro_oder_null(f, lohn)?.get())
}

/// Wie die Erklaerung den Bezug einer Person liest (Abweichung Nr. 50 und Nr. 65). `mit_ring_werten` setzt daraus die Ring-Werte
/// `versorgung_zeile` und `versorgung_altersgrenze`; `elster` kennt weder das Gate noch das Geburtsjahr.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersorgungLage {
    /// Die Zeilen 11 bis 13 der Anlage N tragen den Bezug: der Bescheid rechnet ihn als Versorgungsbezug.
    Zeilen,
    /// Das Alters-Gate (Alter bei Beginn) ist nicht erfuellt, und die Person erreicht die Altersgrenze erst NACH dem
    /// Veranlagungsjahr: der Bezug ist das ganze Jahr normaler Arbeitslohn (Zeile 5) und braucht keine Zeile 11 bis 13.
    Lohn,
    /// Das Alters-Gate ist nicht erfuellt, aber die Person hat die Altersgrenze im oder vor dem Veranlagungsjahr erreicht (oder
    /// das Geburtsjahr ist unlesbar): ab da ist der Bezug ein Versorgungsbezug, dem Zeile 11 und der Freibetrag fehlen.
    UeberDerGrenze,
    /// Kein Bezug, Angaben fehlen oder ein Betrag liegt unter 1 Euro.
    Keine,
}

/// Der Grenzfall zu [`versorgung_lage`]: die Altersgrenze der Person, gegen die das Veranlagungsjahr geprueft wird. Ist der Grad der
/// Behinderung nicht bestaetigt (die Rentner-Scheibe fragt ihn immer, der Gesamtlauf bei einem Bezug der Art `altersgrenze_sonstige`,
/// Abweichung Nr. 80), gilt vorsichtig die niedrigere Grenze: eine zu fruehe Grenze sperrt, eine zu spaete liesse einen
/// Versorgungsbezug als Arbeitslohn durch.
fn altersgrenze_im_jahr(f: &Felder, n: &VersorgungsFelder) -> R<i64> {
    let grad = grad_feld(f, n);
    if wert(f, grad).is_none() {
        return Ok(altersgrenze_fuer_grad(50));
    }
    Ok(altersgrenze_fuer_grad(feld_int_oder_null(f, grad)?))
}

/// Gehoert der Bezug einer Person in die Zeilen 11 bis 13 der Anlage N (Abweichung Nr. 50)? Ja, wenn der Bescheid ihn als
/// Versorgungsbezug rechnet (Jahresrente, Bemessungsgrundlage und Beginnjahr gesetzt, Alters-Gate erfuellt) und beide Betraege
/// auf volle Euro mindestens 1 ergeben: `E0200801` ist im Schema ein Betrag "nicht null" (`ponytail:` ein Bezug unter 1 Euro
/// bekommt keine Zeile). Ist das Alters-Gate (Alter BEI BEGINN) nicht erfuellt, entscheidet das Alter im Veranlagungsjahr
/// (Abweichung Nr. 65, § 19 Abs. 2 S. 2 Nr. 2: Bezuege wegen einer Altersgrenze gelten ERST als Versorgungsbezuege, wenn die Person
/// die Grenze vollendet hat): erreicht sie die Grenze erst nach dem Jahr (Geburtsjahr plus Grenze groesser Veranlagungsjahr), ist
/// der Bezug [`VersorgungLage::Lohn`], sonst [`VersorgungLage::UeberDerGrenze`]. Ohne Veranlagungsjahr (`vz` ist `None`) gibt es
/// nur [`VersorgungLage::Keine`]. `ponytail:` das Geburtsjahr genuegt statt des Geburtstags; im Jahr der Vollendung ist der Bezug
/// schon teils Versorgungsbezug, deshalb sperrt dieses Jahr. Upgrade-Pfad: Beginn des Versorgungsbezugs ab dem Geburtstag bauen.
pub(crate) fn versorgung_lage(f: &Felder, partner: bool, vz: Option<Vz>) -> R<VersorgungLage> {
    let n = if partner { &VERSORGUNG_B } else { &VERSORGUNG_A };
    let v = versorgungsbezug(f, n)?;
    if !v.versorgt || v.jahresrente < 100 || v.bemessungsgrundlage < 100 {
        return Ok(VersorgungLage::Keine);
    }
    if v.gate_erfuellt {
        return Ok(VersorgungLage::Zeilen);
    }
    let Some(vz) = vz else {
        return Ok(VersorgungLage::Keine);
    };
    let jahr = i64::from(vz.jahr());
    let geburtsjahr = feld_int_oder_null(f, n.geburtsjahr)?;
    // Ein Geburtsjahr nach dem Veranlagungsjahr ist unlesbar; 0 (Feld fehlt) und negative Jahre scheitern schon an der Summe.
    let grenze_nach_jahr = geburtsjahr <= jahr && geburtsjahr + altersgrenze_im_jahr(f, n)? > jahr;
    Ok(if grenze_nach_jahr {
        VersorgungLage::Lohn
    } else {
        VersorgungLage::UeberDerGrenze
    })
}

/// [`einkuenfte_ns`] mit dem Bruttoarbeitslohn Person A als Argument. Der Rentner-Ring liest keine
/// Slots und reicht den Lohn aus dem Feld herein; Versorgungsbezuege und Alters-Gate sind dieselben.
/// Bei Zusammenveranlagung (`zusammen`) kommen Lohn und Versorgung der Person B in DIESELBE Summe.
/// Der Lohn enthaelt den Bezug ([`arbeitslohn_ohne_bezug`], Abweichung Nr. 50).
pub(super) fn einkuenfte_ns_aus_lohn(
    f: &Felder,
    vz: Vz,
    p: &bindung::Params,
    lohn: Euro,
    ns_wk: Euro,
    zusammen: bool,
) -> R<Euro> {
    let a = versorgungsbezug(f, &VERSORGUNG_A)?;
    let basis = arbeitslohn_ohne_bezug(lohn.get(), &a)?;
    let mut ns = einkuenfte_nichtselbststaendig(&EinkuenfteNichtselbststaendigEingabe {
        veranlagungszeitraum: vz,
        bruttoarbeitslohn: Euro::new(basis),
        werbungskosten: ns_wk,
    })?;
    // Person B (§ 26b): § 19-Einkuenfte des Ehegatten in DIESELBE Summe, Person-B-WK MVP 0. Sein Lohn enthaelt wie bei
    // Person A den Bezug.
    let b = if zusammen {
        let b = versorgungsbezug(f, &VERSORGUNG_B)?;
        let basis_b = arbeitslohn_ohne_bezug(feld_euro_oder_null(f, "bruttoarbeitslohn_partner")?.get(), &b)?;
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
    // § 34c Abs. 2 (Abweichung Nr. 41): der gewaehlte Abzug der Auslandssteuer zaehlt wie Werbungskosten mit, VOR dem
    // Vergleich mit dem Arbeitnehmer-Pauschbetrag in `einkuenfte_nichtselbststaendig`.
    // Arbeitsmittel mit Bruttopreis ueber 800 EUR, netto aber unter der Grenze (Abweichung Nr. 88): der Catala-Scope hat sie nicht gesehen.
    let wk = mit_sofortabzug_im_band(f, mit_unfallkosten(f, wk)?)?;
    add(add(wk, afa)?, dba_abzug_werbungskosten(f)?)
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
    // Leistungen mit Leistungsmitteilung (Abweichung Nr. 55): Pauschbetrag einmal fuer alle Mitteilungen; ihr `voll`-Teil (Nr. 1 und
    // Nr. 4) geht in die Bemessung des Altersentlastungsbetrags ein (§ 24a Satz 2).
    let rav = rav_bav_einkuenfte(f, &q, vz, p)?;
    let rav_rente = renten_nach_pauschbetrag(rav.ertragsanteil, rav.voll, vz, p)?;
    // § 24a/§ 24b mindern den GdE VOR den Abzuegen. Die uebrigen Einkuenfte von Person A sind NUR ihre eigenen: der Gewinn des
    // Ehegatten (`gewinn_partner`) steht in `g.einkuenfte_gewinn`, aber nicht hier; `entlastungen` bemisst ihn bei Person B.
    // § 24a Satz 2 nimmt nur Versorgungsbezuege, Leibrenten (Nr. 2) und Teile des § 22 Nr. 4 und 5 heraus: der private Verkauf (§ 23)
    // und die sonstigen Leistungen (§ 22 Nr. 3) zaehlen mit (Abweichung Nr. 63; Python liest hier `einkuenfte_sonstige`, das erst
    // spaeter entsteht, `bescheid_zweige.py` Zeile 671/770). Die Leibrente (`rav_rente.gesamt`) und der KiSt-Ueberhang gehen nicht hinein.
    let p23 = p23_privatverkauf(&q, vz, p)?;
    let nr3 = nr3_euro(f)?;
    let uebrige_a = summe_euro(&[vv, laufend, netto_vg, p23, nr3, rav_rente.eingeschlossen])?;
    let (alt_a, alt_b, ent24b) = entlastungen(f, vz, p, zusammen, uebrige_a)?;
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
    // Abweichung Nr. 78: die KV/PV-Vorsorge steht neben dem Pauschbetrag, die uebrigen Sonderausgaben konkurrieren mit ihm.
    g.sonderausgaben = s.uebrige_sonderausgaben()?;
    g.vorsorge_kv_pv = s.vorsorge_kv_pv;
    g.aussergewoehnliche_belastungen = s.aussergewoehnliche_belastungen;
    let kapitaleinkuenfte = p20_kapitaleinkuenfte(f, zusammen, vz, p)?;
    // § 23 + § 22 Nr. 3 + § 22 Nr. 5 in einkuenfte_sonstige; der KiSt-Ueberhang laeuft mangels Slot dort mit.
    g.einkuenfte_sonstige = summe_euro(&[p23, nr3, rav_rente.gesamt])?;
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

/// Die Hoehe von `alt24a_a` und `alt24a_b` einzeln (Abweichung Nr. 62); die Steuerwirkung und die beiden Zweige pruefen
/// `tests/altersentlastung_je_ehegatte_hermetisch.rs`. Alle Faelle VZ 2025, Jahrgang 1955: Zeile 2020 der Tabelle in § 24a,
/// 16,0 %, Hoechstbetrag 760 EUR (`sources/gesetze-im-internet/estg_p24a_2026-07-13.txt`).
#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::testhilfe::{felder, params, store};

    /// `(alt24a_a, alt24a_b)` in EUR. `uebrige_a_euro` sind die uebrigen Einkuenfte von Person A, wie der Aufrufer sie bildet.
    fn alt(veranlagung: &str, felder_cent: &[(&str, Value)], uebrige_a_euro: i64) -> (i64, i64) {
        let mut ereignisse = vec![("veranlagung", json!(veranlagung), true)];
        ereignisse.extend(felder_cent.iter().map(|(f, w)| (*f, w.clone(), true)));
        let f = felder(&store(&ereignisse));
        let zusammen = veranlagung == "zusammen";
        let (a, b, _ent) = entlastungen(&f, Vz::Vz2025, params(), zusammen, Euro::new(uebrige_a_euro)).unwrap();
        (a.get(), b.get())
    }

    /// AK3: beide 1955 mit eigenem Gewinn, A 2.000 EUR, B 3.000 EUR. Je Person 16,0 %: A 0,16 · 2.000 = 320, B 0,16 · 3.000 = 480.
    /// Der Deckel von 760 EUR gilt je Person und wird nicht erreicht. Eine gemeinsame Bemessung gaebe 0,16 · 5.000 = 800, gedeckelt
    /// 760 EUR, und ein Betrag mehr bei der Person mit dem kleineren Gewinn.
    #[test]
    fn beide_ehegatten_bekommen_16_prozent_ihres_eigenen_gewinns() {
        let ehegatten = [
            ("geburtsjahr", json!(1955)),
            ("geburtsjahr_partner", json!(1955)),
            ("einkuenfte_gewinn_partner", json!(300_000)),
        ];
        assert_eq!(alt("zusammen", &ehegatten, 2_000), (320, 480));
    }

    /// Der Ehegatte mit Lohn UND Gewinn: Bemessung = Lohn 2.000 EUR + Gewinn 1.000 EUR = 3.000 EUR, 0,16 · 3.000 = 480 EUR. Nur der
    /// Lohn gaebe 320, nur der Gewinn 160. Person A hat weder Lohn noch uebrige Einkuenfte: 0.
    #[test]
    fn der_ehegatte_mit_lohn_und_gewinn_bemisst_beides() {
        let ehegatten = [
            ("geburtsjahr", json!(1955)),
            ("geburtsjahr_partner", json!(1955)),
            ("bruttoarbeitslohn_partner", json!(200_000)),
            ("einkuenfte_gewinn_partner", json!(100_000)),
        ];
        assert_eq!(alt("zusammen", &ehegatten, 0), (0, 480));
    }

    /// § 24a Satz 1: nur die POSITIVE Summe der uebrigen Einkuenfte. Ein Verlust beim Gewinn mindert den Lohn nicht: B hat Lohn
    /// 2.000 EUR und Gewinn −5.000 EUR, Bemessung 2.000 + max(0, −5.000) = 2.000, 0,16 · 2.000 = 320. A hat Lohn 2.000 EUR und uebrige
    /// Einkuenfte −1.000 EUR: ebenso 320.
    #[test]
    fn ein_verlust_der_uebrigen_einkuenfte_mindert_den_lohn_nicht() {
        let ehegatten = [
            ("geburtsjahr", json!(1955)),
            ("geburtsjahr_partner", json!(1955)),
            ("bruttoarbeitslohn", json!(200_000)),
            ("bruttoarbeitslohn_partner", json!(200_000)),
            ("einkuenfte_gewinn_partner", json!(-500_000)),
        ];
        assert_eq!(alt("zusammen", &ehegatten, -1_000), (320, 320));
    }

    /// Der Gewinn des Ehegatten (80.000 EUR) gehoert nicht in die Bemessung von Person A: A hat nichts (0), B ist 1990 und
    /// bekommt nichts (0). Vorher bekam A hier 760 EUR.
    #[test]
    fn der_gewinn_des_ehegatten_entlastet_person_a_nicht() {
        let ehegatten = [
            ("geburtsjahr", json!(1955)),
            ("geburtsjahr_partner", json!(1990)),
            ("einkuenfte_gewinn_partner", json!(8_000_000)),
        ];
        assert_eq!(alt("zusammen", &ehegatten, 0), (0, 0));
    }

    /// Person B 1955 mit Gewinn 80.000 EUR, Person A 1990: B bekommt den Deckel, 0,16 · 80.000 = 12.800, gedeckelt 760 EUR.
    /// Vorher bekam B nie etwas.
    #[test]
    fn der_aeltere_ehegatte_mit_eigenem_gewinn_bekommt_den_deckel() {
        let ehegatten = [
            ("geburtsjahr", json!(1990)),
            ("geburtsjahr_partner", json!(1955)),
            ("einkuenfte_gewinn_partner", json!(8_000_000)),
        ];
        assert_eq!(alt("zusammen", &ehegatten, 0), (0, 760));
    }

    /// Einzelveranlagung: `alt24a_b` bleibt 0, auch mit Jahrgang, Lohn (2.000 EUR) und Gewinn des Partners in den Feldern; A
    /// bekommt aus 20.000 EUR uebrigen Einkuenften 0,16 · 20.000 = 3.200, gedeckelt 760 EUR.
    #[test]
    fn einzelveranlagung_rechnet_nur_person_a() {
        let felder_partner = [
            ("geburtsjahr", json!(1955)),
            ("geburtsjahr_partner", json!(1955)),
            ("bruttoarbeitslohn_partner", json!(200_000)),
            ("einkuenfte_gewinn_partner", json!(5_000_000)),
        ];
        assert_eq!(alt("einzel", &felder_partner, 20_000), (760, 0));
    }

    /// Der § 16-Gewinn des Ehegatten (1.000 EUR, ohne Freibetrag, weil die zwei Angaben zu § 16 Abs. 4 fehlen) geht in dessen
    /// Bemessung ein: 0,16 · 1.000 = 160 EUR. Ohne ihn ergaebe sich 0 (der laufende Gewinn ist 0).
    #[test]
    fn der_veraeusserungsgewinn_des_ehegatten_geht_in_dessen_bemessung() {
        let ehegatten = [
            ("geburtsjahr", json!(1990)),
            ("geburtsjahr_partner", json!(1955)),
            ("rentner_veraeusserungsgewinn_partner", json!(100_000)),
        ];
        assert_eq!(alt("zusammen", &ehegatten, 0), (0, 160));
    }

    /// Der Freibetrag nach § 16 Abs. 4 mindert den Gewinn des Ehegatten VOR der Bemessung: 30.000 EUR Gewinn mit beiden Angaben
    /// (ab 55 Jahren, erstmals) liegen unter dem Freibetrag von 45.000 EUR, netto 0, also `alt24a_b` = 0. Mit dem rohen Gewinn
    /// ergaebe sich 0,16 · 30.000 = 4.800, gedeckelt 760 EUR.
    #[test]
    fn der_freibetrag_nach_p16_4_mindert_den_gewinn_vor_der_bemessung() {
        let ehegatten = [
            ("geburtsjahr", json!(1990)),
            ("geburtsjahr_partner", json!(1955)),
            ("rentner_veraeusserungsgewinn_partner", json!(3_000_000)),
            ("rentner_alter_55_oder_berufsunfaehig_partner", json!(true)),
            ("rentner_freibetrag_erstmalig_partner", json!(true)),
        ];
        assert_eq!(alt("zusammen", &ehegatten, 0), (0, 0));
    }
}
