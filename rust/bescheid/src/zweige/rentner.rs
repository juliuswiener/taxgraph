//! `_zweig_festzusetzende_est_rentner` (`bescheid_zweige.py:1072-1480`), Teil 1: § 22-Renten,
//! § 33b, Gewinn, § 24a/§ 24b und der Aufbau des Gesamtfalls. Tarif-Teil: `rentner_tarif.rs`.
use domain::{Euro, Vz};
use engine::zugriff::teil1::ermaessigungen::{p24a_altersentlastung, P24aAltersentlastungEingabe};
use engine::zugriff::teil2::gesamt::{gesamt_gde, GesamtfallEingabe};
use engine::zugriff::teil2::rente::{renten_einkuenfte, RentenEingabe, Rentenart};
use intervall::Slots;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use serde_json::Value;

use super::gesamt::{
    entlastung_24b, gde_fall, kist_ueberhang, netto_vg, nr3_euro, p35_person_a, pauschbetraege_a,
    pauschbetrag_partner, vorsorge_slots,
};
use super::rechnen::{add, max0, summe_euro, R};
use super::tarif::{leerer_gesamtfall, rahmen, Lage, Modus, P35};
use super::{rentner_tarif, Marke, Ring, VeranlagungWert};
use crate::abzuege::{p33b_kind_pauschbetraege, shared_steuer_sonder_agb};
use crate::einkuenfte::{
    gewinn_partner_anteil, laufender_gewinn, p20_kapitaleinkuenfte, p23_ansonsten_einkuenfte,
    p35_summen, shared_dba_sonstige,
};
use crate::{
    cent_zu_euro, feld_euro_oder_null, feld_int_oder_null, py_wahr, wert, zahl_dezimal, Felder,
};

/// Rentenarten mit § 22 Nr. 1 S. 3 a aa (`AA_RENTEN_ARTEN`) bzw. bb (`BB_RENTEN_ARTEN`).
const AA_RENTEN: [&str; 3] = [
    "gesetzliche_rente",
    "berufsstaendische_versorgung",
    "private_basisrente",
];
const BB_RENTEN: [&str; 2] = ["private_leibrente", "sonstige_leibrente"];

/// `rf // 100 if isinstance(rf, (int, float)) and not bool else None` (Naht-CENT → EURO).
///
/// PARITÄT: Python `//` auf einem Float ist `floor(x / 100)`. Hier: [`cent_zu_euro_dezimal`] auf dem
/// exakten `Decimal` des Floats. Grenzfall: Python teilt in `float` (Rundung nach dem Teilen),
/// `Decimal` teilt exakt; beide weichen nur ab, wenn `x / 100.0` in Float auf eine ganze Zahl
/// aufrundet, obwohl `x < 100·n` — bei 64-Bit-Floats ausgeschlossen (Abstand des Vorgaengers
/// >= 0,64 ulp der Quotienten). Store-Werte dieses Feldes sind ohnehin Ganzzahlen (Typ `cent`).
fn rentenfreibetrag_euro(rf: Option<&Value>) -> Option<Euro> {
    match rf {
        Some(Value::Number(n)) => Some(
            n.as_i64()
                .map_or_else(|| cent_zu_euro_dezimal(zahl_dezimal(n)), cent_zu_euro),
        ),
        _ => None,
    }
}

/// Rundungsfunktion § 22 Nr. 1 S. 3 a aa: Cent → volle Euro, abgerundet (`floor`, Python `// 100`).
/// Saturiert bei ±9e18 Euro (Bereichsschutz wie zuvor; Store-Werte liegen weit darunter).
fn cent_zu_euro_dezimal(cent: Decimal) -> Euro {
    let grenze = Decimal::from(9_000_000_000_000_000_000_i64);
    let euro = (cent / Decimal::ONE_HUNDRED).floor().clamp(-grenze, grenze);
    Euro::new(euro.to_i64().unwrap_or(0))
}

/// Rentenart aus `renten_art` + den Feldern, die der jeweilige Zweig liest.
fn rentenart(art: Option<&Value>, beginn: i64, alter: i64, rf: Option<Euro>) -> Rentenart {
    match art {
        Some(Value::String(s)) if BB_RENTEN.contains(&s.as_str()) => Rentenart::Bb {
            alter_bei_rentenbeginn: alter,
        },
        Some(Value::String(s)) if AA_RENTEN.contains(&s.as_str()) => Rentenart::Aa {
            renten_beginn_jahr: beginn,
            rentenfreibetrag: rf,
        },
        _ => Rentenart::NichtRingfaehig,
    }
}

/// § 22-Renten-Einkuenfte EINER Rente (Instanz oder Basis), EURO.
///
/// PARITÄT: ein unbeantwortetes `rentner_renten_beginn_jahr` (kein `int`) gibt 0 statt eines Fehlers.
/// Python `isinstance(x, int)` gilt auch fuer `bool` — hier ebenso.
fn rente_instanz(fi: &Felder, vz: Vz, p: &bindung::Params) -> R<Euro> {
    let beginn_ist_int = match wert(fi, "rentner_renten_beginn_jahr") {
        Some(Value::Number(n)) => n.is_i64() || n.is_u64(),
        Some(Value::Bool(_)) => true,
        _ => false,
    };
    if !beginn_ist_int {
        return Ok(Euro::new(0));
    }
    let art = rentenart(
        wert(fi, "rentner_renten_art"),
        feld_int_oder_null(fi, "rentner_renten_beginn_jahr")?,
        feld_int_oder_null(fi, "rentner_alter_bei_rentenbeginn")?,
        rentenfreibetrag_euro(wert(fi, "rentner_rentenfreibetrag")),
    );
    Ok(renten_einkuenfte(
        &RentenEingabe {
            vz,
            art,
            jahresrente: feld_euro_oder_null(fi, "rentner_jahresrente")?,
        },
        p,
    )?)
}

/// Person A: stumpfe Σ ueber alle `rente`-Instanzen, ohne Store nur die Basis.
///
/// PARITÄT: P8 (`REWRITE_PLAN.md` §4, `bescheid_zweige.py:1110-1127`) — Person A summiert ueber die
/// Instanz-Naht (`elster::instanzen`), Person B liest unten das FLACHE Feld `rentner_*_partner`. Eine
/// zweite Partner-Rente (`__2`) geht deshalb nie in die Summe; Rust bildet das nach.
fn renten_summe<Z: Marke>(r: &Ring<'_, Z>) -> R<Euro> {
    let (vz, p) = (r.vz(), r.p());
    let q = r.q();
    if q.store.is_none() {
        return rente_instanz(r.f(), vz, p);
    }
    let mut renten = Euro::new(0);
    for inst in q.instanzen("rente")? {
        if q.zaehlt(&inst) {
            renten = add(renten, rente_instanz(&inst.felder, vz, p)?)?;
        }
    }
    Ok(renten)
}

/// Person B (§ 26b): § 22-Rente des Ehegatten; nur bei Zusammenveranlagung und gesetzter Rentenart.
///
/// PARITÄT: P8 — Flat-Feld, keine Instanz-Σ (Gegenstueck zu [`renten_summe`]).
fn renten_partner(f: &Felder, vz: Vz, p: &bindung::Params) -> R<Euro> {
    let art = rentenart(
        wert(f, "rentner_renten_art_partner"),
        feld_int_oder_null(f, "rentner_renten_beginn_jahr_partner")?,
        feld_int_oder_null(f, "rentner_alter_bei_rentenbeginn_partner")?,
        rentenfreibetrag_euro(wert(f, "rentner_rentenfreibetrag_partner")),
    );
    Ok(renten_einkuenfte(
        &RentenEingabe {
            vz,
            art,
            jahresrente: feld_euro_oder_null(f, "rentner_jahresrente_partner")?,
        },
        p,
    )?)
}

/// Der Zweig `festzusetzende_est_rentner`, EURO. Der Slot `veranlagung` wird NICHT gelesen — die
/// Veranlagungsart kommt aus dem Feld (`_b("veranlagung") or "einzel"`).
///
/// # Errors
/// Instanz-, Accessor- und Ueberlauf-Fehler.
pub(super) fn festzusetzende_est_rentner<Z: Marke>(r: &Ring<'_, Z>, _slots: &Slots) -> R<Euro> {
    let (f, vz, p) = (r.f(), r.vz(), r.p());
    let q = r.q();
    let mut renten = renten_summe(r)?;
    let zusammen_feld = VeranlagungWert::aus(wert(f, "veranlagung")).zusammen();
    if zusammen_feld && wert(f, "rentner_renten_art_partner").is_some_and(py_wahr) {
        renten = add(renten, renten_partner(f, vz, p)?)?;
    }
    // § 33b Behinderten-/Pflege-/Hinterbliebenen-PB, Kind-Uebertragung, Ehegatte.
    let mut ausserg = add(
        pauschbetraege_a(f, vz, p)?,
        p33b_kind_pauschbetraege(&q, vz, p)?,
    )?;
    if zusammen_feld {
        ausserg = add(ausserg, pauschbetrag_partner(f, vz, p)?)?;
    }
    // § 23 und § 22 Nr. 3 additiv zu renten.
    let p23 = p23_ansonsten_einkuenfte(&q)?;
    renten = add(renten, p23)?;
    renten = add(renten, nr3_euro(f)?)?;
    // §§ 13-18 Gewinn: laufend + § 16-vg netto (+ Gewinn des Ehegatten erst danach, s. u.).
    let netto_vg = netto_vg(f)?;
    let (laufend, mitu) = laufender_gewinn(f, &q)?;
    // § 24a-Bemessung: nur Nicht-§19-Einkuenfte (Gewinn + § 23); Leibrente § 22 Nr. 1 ist ausgenommen.
    let alt = p24a_altersentlastung(
        &P24aAltersentlastungEingabe {
            veranlagungszeitraum: i64::from(vz.jahr()),
            geburtsjahr: feld_int_oder_null(f, "geburtsjahr")?,
            arbeitslohn: Euro::new(0),
            positive_andere_einkuenfte: max0(summe_euro(&[laufend, netto_vg, p23])?),
        },
        p,
    )?;
    let ent = entlastung_24b(f)?;
    let (gewinn_partner, _mitu_partner, netto_vg_partner) = gewinn_partner_anteil(f)?;
    let veranlagung = VeranlagungWert::aus_oder_einzel(wert(f, "veranlagung"));
    let zusammen = veranlagung.zusammen();
    let mut g = GesamtfallEingabe {
        einkuenfte_sonstige: renten,
        einkuenfte_gewinn: summe_euro(&[laufend, netto_vg, gewinn_partner])?,
        altersentlastungsbetrag: alt,
        entlastungsbetrag_alleinerziehende: ent,
        aussergewoehnliche_belastungen: ausserg,
        ..leerer_gesamtfall(vz, zusammen)
    };
    let mut gde = gesamt_gde(&gde_fall(&g), p)?;
    let kapitaleinkuenfte = p20_kapitaleinkuenfte(f, zusammen, vz, p)?;
    // § 10 Abs. 4b S. 3: der Ueberhang erhoeht die GdE und (mangels Slot) einkuenfte_sonstige.
    let ueberhang = kist_ueberhang(f)?;
    gde = add(gde, ueberhang)?;
    g.einkuenfte_sonstige = add(g.einkuenfte_sonstige, ueberhang)?;
    let dba = shared_dba_sonstige(&mut g, gde, veranlagung.domain(), f, vz, p)?;
    (
        g.vorsorge_gesamtbeitraege_inkl_ag,
        g.vorsorge_ag_anteil_steuerfrei,
    ) = vorsorge_slots(f, zusammen)?;
    let s = shared_steuer_sonder_agb(gde, ausserg, veranlagung.domain(), f, vz, &q, p)?;
    g.steuerermaessigungen = s.steuerermaessigungen;
    g.sonderausgaben = s.sonderausgaben;
    g.aussergewoehnliche_belastungen = s.aussergewoehnliche_belastungen;
    // § 35: Zaehler wie gesamt; Nenner = renten (§ 22 IM Nenner) + einkuenfte_gewinn, OHNE Ueberhang.
    let (mb, hs, zaehler) = p35_person_a(f, laufend, mitu)?;
    let (mb_ges, z_ges, gezahlt) = p35_summen(f, mb, hs, zaehler)?;
    let nenner = add(max0(renten), max0(g.einkuenfte_gewinn))?;
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
        pe_raw: feld_euro_oder_null(f, "p32b_progressionseinkuenfte")?,
        kapitaleinkuenfte,
        kinder: feld_int_oder_null(f, "fam_anzahl_kinder")?,
        extras: r.a.extras,
    };
    rahmen(&lage, &g, r.a.solz, Modus::Rentner, |fb, info| {
        rentner_tarif::festzusetzende(&lage, &g, fb, info)
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Float-Rentenfreibetrag: Python `//` auf Float ist `floor` — auch bei negativen Werten, und
    /// knapp unter einer ganzen Zahl (Paritaetsluecke: kein Korpusfall hat einen Float an dieser Stelle).
    #[test]
    fn rentenfreibetrag_float_rundet_wie_python_floor() {
        let euro = |v: Value| rentenfreibetrag_euro(Some(&v));
        assert_eq!(euro(json!(12345.0)), Some(Euro::new(123)));
        assert_eq!(euro(json!(-150.5)), Some(Euro::new(-2)));
        assert_eq!(euro(json!(299.999_999_999_999_94)), Some(Euro::new(2)));
        assert_eq!(euro(json!(12345)), Some(Euro::new(123)));
        assert_eq!(euro(json!("text")), None);
        assert_eq!(rentenfreibetrag_euro(None), None);
    }

    /// Jenseits des `Euro`-Bereichs saturiert die Rundung, statt zu wrappen.
    #[test]
    fn cent_zu_euro_dezimal_saturiert() {
        let riesig = Decimal::from(9_000_000_000_000_000_000_i64) * Decimal::from(1_000);
        assert_eq!(
            cent_zu_euro_dezimal(riesig),
            Euro::new(9_000_000_000_000_000_000)
        );
        assert_eq!(
            cent_zu_euro_dezimal(-riesig),
            Euro::new(-9_000_000_000_000_000_000)
        );
    }
}
