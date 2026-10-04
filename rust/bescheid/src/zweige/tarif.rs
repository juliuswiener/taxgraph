//! Der Tarif-Teil, den `gesamt` und `rentner` gemeinsam haben: § 34-Chooser, § 35-Anrechnung,
//! § 32d-Kapital-Guenstigerpruefung und der § 31-Rahmen samt SolZ, KiSt und Rechenweg-Kette
//! (`bescheid_zweige.py:859-1069` und `:1292-1479`). Die beiden Zweige unterscheiden sich in der
//! Reihenfolge von § 32b und Kapital; das steht in `gesamt_tarif.rs` bzw. `rentner_tarif.rs`.
use std::cell::{Cell, RefCell};

use bindung::Params;
use domain::{Cent, Euro, Konfession, PyWert, Vz};
use engine::zugriff::teil1::ermaessigungen::{
    kist, p31_familienleistung, KistEingabe, P31FamilienleistungEingabe,
};
use engine::zugriff::teil2::est::{fuenftel, tarif_est, FuenftelEingabe, TarifEingabe};
use engine::zugriff::teil2::gesamt::{
    ermaessigter_durchschnittssatz, gesamt, gesamt_tarifliche, gesamt_zve,
    DurchschnittssatzEingabe, GesamtfallEingabe,
};
use engine::zugriff::teil2::kapital::{kapital_steuer, KapitalSteuerEingabe};
use engine::zugriff::teil2::solz::{solz, SolzEingabe};

use super::ausgaben::{kette_endstand, kette_p31, kist_konfession, setze_kette, Extras};
use super::kinderfreibetrag::{auswerten, Befund};
use super::rechnen::{add, mal, mal_div, max0, sub, R};
use super::VeranlagungWert;
use crate::abzuege::abs3_eligible;
use crate::{ist_true, py_int, wert, BescheidFehler, Felder, Instanzquelle};

/// Laender mit 8 % (`runner._KIST_BY_BW`).
const KIST_8_PROZENT: [&str; 2] = ["bayern", "baden_wuerttemberg"];

/// § 35 EStG Basiswerte, freibetrag-unabhaengig (Zaehler/Nenner haengen nicht vom § 31-Zweig ab).
#[derive(Debug, Clone, Copy)]
pub(super) struct P35 {
    pub messbetrag_ges: Euro,
    pub zaehler_ges: Euro,
    pub gezahlt: Euro,
    pub nenner: Euro,
}

impl P35 {
    /// `messbetrag_ges > 0 and zaehler_ges > 0 and nenner > 0`.
    pub(super) fn aktiv(&self) -> bool {
        self.messbetrag_ges.get() > 0 && self.zaehler_ges.get() > 0 && self.nenner.get() > 0
    }
}

/// SolZ-Zwischenstand (`solz_info`): nur die drei Werte, die spaeter gelesen werden.
#[derive(Debug, Clone, Copy)]
pub(super) struct SolzInfo {
    pub est_mit_fb: Euro,
    pub kap_st: Euro,
    pub est_ohne_p35: Euro,
}

/// Endstand eines § 31-Laufs (Python `bescheid_zweige._kette_endstand`): das finale Eingabe-Dict
/// des Laufs und sein zurueckgegebener Endwert. Aus DIESEM — nicht aus dem Vor-Korrektur-Rohstand —
/// wird die Rechenweg-Kette gefuettert. `SolzInfo` reicht dafuer nicht: es traegt nur den
/// KiFB-Lauf, die Kette braucht bei Kindern auch den fb=0-Lauf (Kindergeld-Sieg).
#[derive(Debug, Clone, Copy)]
pub(super) struct Endstand {
    pub g2: GesamtfallEingabe,
    pub wert: Euro,
}

/// Was der Tarif-Teil ueber den Aufbau hinaus braucht (Python-Closure-Variablen).
pub(super) struct Lage<'a> {
    pub vz: Vz,
    pub p: &'a Params,
    pub f: &'a Felder,
    pub veranlagung: VeranlagungWert,
    pub netto_vg: Euro,
    pub netto_vg_partner: Euro,
    pub dba_anrechnung: Euro,
    pub p35: P35,
    pub pe_raw: Euro,
    pub kapitaleinkuenfte: Euro,
    pub kinder: i64,
    /// Die Instanz-Quelle der Kind-Angaben (`kind`-Gruppe), in der Sicht des Aufrufs (streng oder nicht).
    pub q: Instanzquelle<'a>,
    pub extras: Option<&'a RefCell<Extras>>,
}

impl Lage<'_> {
    pub(super) fn pe_active(&self) -> bool {
        self.pe_raw.get() > 0
    }

    /// `runner.catala_est(g)` fuer einen Gesamtfall.
    pub(super) fn est(&self, g: &GesamtfallEingabe) -> R<Euro> {
        Ok(gesamt(g, self.p)?)
    }

    /// `runner.catala_est({vz, veranlagung, zu_versteuerndes_einkommen})` — der Tarif-Zweig.
    pub(super) fn tarif(&self, veranlagung: VeranlagungWert, zve: Euro) -> R<Euro> {
        Ok(tarif_est(&TarifEingabe {
            vz: self.vz,
            veranlagung: veranlagung.tarif_strikt()?,
            zu_versteuerndes_einkommen: zve,
        })?)
    }

    /// Schreibzugriff auf `extras`, wenn Python eines uebergeben hat.
    pub(super) fn mit_extras(&self, f: impl FnOnce(&mut Extras)) {
        if let Some(e) = self.extras {
            f(&mut e.borrow_mut());
        }
    }
}

/// § 34-Chooser (XOR): Abs. 1 Fuenftel (Default) gegen Abs. 3 ermaessigter Durchschnittssatz (auf
/// Antrag). Setzt `tarif_modifiziert`/`tarifliche_est_modifiziert`, sobald ein ao-Gewinn vorliegt.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
pub(super) fn p34_chooser(l: &Lage<'_>, g2: GesamtfallEingabe) -> R<GesamtfallEingabe> {
    let ao = add(l.netto_vg, l.netto_vg_partner)?;
    if ao.get() <= 0 {
        return Ok(g2);
    }
    let zve2 = gesamt_zve(&g2, l.p)?;
    if zve2.get() <= 0 {
        return Ok(g2);
    }
    let abs3 = ist_true(wert(l.f, "antrag_ermaessigter_satz"))
        && abs3_eligible(l.f, l.vz)?
        && 0 < l.netto_vg.get()
        && l.netto_vg.get() <= 5_000_000;
    let modifiziert = if abs3 {
        // § 34 Abs. 3: Grundtarif(zvE - ao) + ermaessigter Satz * min(ao, 5 Mio).
        let est_rest = l.tarif(l.veranlagung, max0(sub(zve2, l.netto_vg)?))?;
        let est_ao = ermaessigter_durchschnittssatz(&DurchschnittssatzEingabe {
            ao_einkuenfte: l.netto_vg,
            est_gesamt_zzgl_progression: gesamt_tarifliche(&g2, l.p)?,
            bemessungsgrundlage_durchschnitt: zve2,
        })?;
        add(est_rest, est_ao)?
    } else {
        // § 34 Abs. 1 Fuenftel (Default ODER Abs.-3-Eignung fehlt — nie Abs. 3 erzwingen).
        fuenftel(&FuenftelEingabe {
            vz: l.vz,
            veranlagung: l.veranlagung.tarif_strikt()?,
            zu_versteuerndes_einkommen: zve2,
            ausserordentliche_einkuenfte: ao,
        })?
    };
    Ok(GesamtfallEingabe {
        tarif_modifiziert: true,
        tarifliche_est_modifiziert: modifiziert,
        ..g2
    })
}

/// § 35 Abs. 1: `min(4 x Messbetrag, gezahlte GewSt, Zaehler/Nenner x geminderte tarifliche ESt)`.
/// 0, wenn § 35 nicht aktiv ist.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
pub(super) fn p35_credit(l: &Lage<'_>, g2: &GesamtfallEingabe) -> R<Euro> {
    if !l.p35.aktiv() {
        return Ok(Euro::new(0));
    }
    let tarifliche_raw = gesamt_tarifliche(g2, l.p)?;
    let gemindert = max0(sub(tarifliche_raw, l.dba_anrechnung)?);
    let deckel3 = mal_div(l.p35.zaehler_ges.get(), gemindert.get(), l.p35.nenner.get())?;
    let vierfach = mal(4, l.p35.messbetrag_ges)?;
    Ok(Euro::new(
        vierfach.get().min(l.p35.gezahlt.get()).min(deckel3),
    ))
}

/// Ergebnis der § 32d-Guenstigerpruefung (Abs. 6) samt Abs.-1-Ermaessigung.
pub(super) struct Kapital {
    pub kap_st_k: Euro,
    /// Kapital-KiSt in CENT (`kist_kap_cent`).
    pub kist_kap_cent: Cent,
    pub guenstiger: bool,
}

/// `f.get("kap_q_auslaendische_steuer", {}).get("wert") or 0`, dann `int(..)`.
fn q_roh_cent(f: &Felder) -> R<i64> {
    // PARITÄT: fail-open default — ein falsy Wert ist 0.
    match wert(f, "kap_q_auslaendische_steuer") {
        Some(v) if v.truthy() => py_int(v),
        _ => Ok(0),
    }
}

/// § 32d Abs. 6 Guenstigerpruefung + Abs. 1 S. 3-5 (Kapital-KiSt-Ermaessigung); `est_raw` ist die ESt
/// OHNE Kapital, `g2` der Gesamtfall dazu.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
pub(super) fn kapital(l: &Lage<'_>, g2: &GesamtfallEingabe, est_raw: Euro) -> R<(Euro, Kapital)> {
    let est_mit = l.est(&GesamtfallEingabe {
        einkuenfte_kapitalvermoegen: l.kapitaleinkuenfte,
        ..*g2
    })?;
    let kap_st = kapital_steuer(
        &KapitalSteuerEingabe {
            vz: l.vz,
            kapitaleinkuenfte: l.kapitaleinkuenfte,
            est_regulaer_mit_kap: est_mit,
            est_regulaer_ohne_kap: est_raw,
        },
        l.p,
    )?;
    let kap = l.kapitaleinkuenfte.get();
    let satz =
        l.p.abgeltungssatz_prozent(l.vz)
            .map_err(engine::zugriff::teil1::fehler::EngineFehler::from)?;
    let abgeltung = mal_div(kap, satz, 100)?;
    let guenstiger = kap_st.get() < abgeltung;
    let mut kap_st_k = kap_st;
    let mut kist_kap_cent = 0_i64;
    if kap_st.get() == abgeltung {
        let q_eur = (q_roh_cent(l.f)?.div_euclid(100)).min(kap_st.get());
        if kist_steuererhebend(l.f) {
            let ksatz = if kist_bundesland(l.f).is_some_and(|b| KIST_8_PROZENT.contains(&b)) {
                8
            } else {
                9
            };
            let vier_q = crate::minus(
                kap,
                q_eur
                    .checked_mul(4)
                    .ok_or(BescheidFehler::Ueberlauf("4*q"))?,
            )?;
            let kap_st_k_cent = mal_div(vier_q, 10_000, 400 + ksatz)?.max(0);
            kist_kap_cent = mal_div(kap_st_k_cent, ksatz, 100)?;
            kap_st_k = Euro::new(kap_st_k_cent.div_euclid(100));
        } else {
            kap_st_k = Euro::new(crate::minus(kap_st.get(), q_eur)?.max(0));
        }
    }
    Ok((
        est_mit,
        Kapital {
            kap_st_k,
            kist_kap_cent: Cent::new(kist_kap_cent),
            guenstiger,
        },
    ))
}

/// `f.get("kist_konfession", {}).get("wert", "keine") in runner._KIST_KONFESSION_STEUERERHEBEND`.
///
/// PARITÄT: fail-open default — fehlende Konfession = "keine" (§ 32d-Ermaessigung entfaellt,
/// zu viel Steuer; Python-Docstring `_kist_konfession`, bewusst nicht geaendert). `null` und ein
/// Wert ausserhalb der `enum_werte` stehen ebenso in keiner Liste.
fn kist_steuererhebend(f: &Felder) -> bool {
    match domain::Lage::konfession(wert(f, "kist_konfession")) {
        domain::Lage::Gueltig(Konfession::Evangelisch | Konfession::RoemischKatholisch) => true,
        domain::Lage::Fehlt
        | domain::Lage::Null
        | domain::Lage::Gueltig(Konfession::Keine | Konfession::Andere)
        | domain::Lage::Abweichend(_) => false,
    }
}

/// `f.get("kist_bundesland", {}).get("wert", "")` als Text; ein Nicht-Text ist "in keiner Liste".
fn kist_bundesland(f: &Felder) -> Option<&str> {
    match wert(f, "kist_bundesland") {
        Some(PyWert::Text(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// `catala_kist`-Eingabe aus dem Snapshot (`bundesland` ohne Antwort oder Nicht-Text = "").
pub(super) fn kist_eingabe(f: &Felder, konfession: &str, est_mit_fb: Euro) -> KistEingabe {
    KistEingabe {
        konfession: konfession.to_owned(),
        bundesland: kist_bundesland(f).unwrap_or("").to_owned(),
        est_mit_fb,
    }
}

/// Wie der Rahmen die KiSt-Basis findet (die Zweige unterscheiden sich in einem Detail).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Modus {
    /// `est_ohne_p35` fehlt → Basis 0 (Python `solz_info.get("est_ohne_p35", 0)`).
    Gesamt,
    /// `est_ohne_p35` fehlt → keine KiSt (`"est_ohne_p35" in solz_info_r`).
    Rentner,
}

/// § 31 Familienleistungsausgleich-Rahmen um `festzusetzende(freibetrag, info, ende)`, danach
/// Rechenweg-Kette, SolZ und KiSt. Die Kette braucht NICHT den Basis-Gesamtfall: sie kommt aus dem
/// Endstand, den jeder Lauf ueber `ende` hinterlaesst (s. `kette_endstand`).
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler, zusaetzlich die `KeyError`-Parallele, wenn ein Lauf keinen
/// Endstand hinterlassen hat.
pub(super) fn rahmen<F>(
    l: &Lage<'_>,
    solz_out: Option<&Cell<Option<Cent>>>,
    modus: Modus,
    festzusetzende: F,
) -> R<Euro>
where
    F: Fn(Euro, &mut Option<SolzInfo>, &mut Option<Endstand>) -> R<Euro>,
{
    let mut info: Option<SolzInfo> = None;
    // Endstand je § 31-Lauf (Python `kette_end`-Dict): die Kette wird daraus gerechnet. Ein `None`
    // hier heisst: ein Lauf hat nichts hinterlassen — das ist ein Fehler, kein Grund fuer None.
    let mut ende_ohne: Option<Endstand> = None;
    let mut ende_mit: Option<Endstand> = None;
    let zusammen = l.veranlagung.zusammen();
    let kinder = l.kinder;
    let (est, kg_kind, fb_guenstiger) = if kinder > 0 {
        let je_elternteil =
            l.p.kinderfreibetrag_je_elternteil(l.vz)
                .map_err(engine::zugriff::teil1::fehler::EngineFehler::from)?;
        let kg =
            l.p.kindergeld_monatlich_je_kind(l.vz)
                .map_err(engine::zugriff::teil1::fehler::EngineFehler::from)?;
        // § 32 Abs. 6 Satz 2 und 5 je Kind: Kindschaftsverhaeltnis und Monate; Freibetrag und Kindergeld
        // folgen denselben Monaten. Der K2-Guard hat dieselben Faelle vorher gesperrt.
        let (fb_kind, kg_kind) = match auswerten(&l.q, kinder, zusammen, Some(l.vz.jahr()))? {
            Befund::Rechenbar(summen) => summen.betraege(je_elternteil, kg)?,
            Befund::Gesperrt(sperre) => {
                return Err(BescheidFehler::KindFreibetragGesperrt(sperre.grund()))
            }
        };
        let est_ohne = festzusetzende(Euro::new(0), &mut info, &mut ende_ohne)?;
        let est_mit = festzusetzende(fb_kind, &mut info, &mut ende_mit)?;
        let guenstiger = add(est_mit, kg_kind)?.get() < est_ohne.get();
        let est = p31_familienleistung(&P31FamilienleistungEingabe {
            est_ohne_freibetraege: est_ohne,
            est_mit_freibetraegen: est_mit,
            kindergeld: kg_kind,
        })?;
        (est, kg_kind, guenstiger)
    } else {
        (
            festzusetzende(Euro::new(0), &mut info, &mut ende_ohne)?,
            Euro::new(0),
            false,
        )
    };
    // P5.4 Rechenweg-Kette — auch mit Kindern, mit der § 31-Entscheidung. Gespeist aus dem
    // ENDSTAND des Laufs (s. `kette_endstand`): die Korrekturen § 34/§ 35 sitzen in dessen
    // finale Eingabe, § 32b/§ 32d in seinem Endwert. Ein fehlender Endstand ist ein Fehler, keine
    // still uebergangene Kette — sonst waere der Defekt zurueck, den p24a behebt.
    if l.extras.is_some() {
        let fehlte = |was| BescheidFehler::Python {
            klasse: "KeyError",
            was,
        };
        let ohne = kette_endstand(
            ende_ohne.as_ref().ok_or(fehlte(
                "kette_end[0] fehlt — der fb=0-Lauf hat keinen Endstand hinterlassen",
            ))?,
            l.p,
        )?;
        let kette = if kinder > 0 {
            let mit = kette_endstand(
                ende_mit.as_ref().ok_or(fehlte(
                    "kette_end[freibetrag] fehlt — der KiFB-Lauf hat keinen Endstand hinterlassen",
                ))?,
                l.p,
            )?;
            kette_p31(ohne, mit, fb_guenstiger, kg_kind)?
        } else {
            ohne
        };
        l.mit_extras(|e| setze_kette(e, kette, est));
    }
    // SolZ § 3/§ 4: Basis = KiFB-fiktive ESt minus § 32d-Kapitalsteuer.
    if let (Some(out), Some(i)) = (solz_out, info) {
        out.set(Some(solz(&SolzEingabe {
            vz: l.vz,
            bemessungsgrundlage: i.est_mit_fb,
            kapital_steuer: i.kap_st,
            splitting: zusammen,
        })?));
    }
    // KiSt § 51a: ohne beantwortete Konfession bleibt der Schluessel absent (= nicht rechenbar).
    if let (Some(extras), Some(konf)) = (l.extras, kist_konfession(l.f)) {
        let basis = match (info, modus) {
            (Some(i), _) => Some(i.est_ohne_p35),
            (None, Modus::Gesamt) => Some(Euro::new(0)),
            (None, Modus::Rentner) => None,
        };
        if let Some(basis) = basis {
            let kist_cent = kist(&kist_eingabe(l.f, konf, basis))?;
            let kap = extras.borrow().kist_kap_cent.unwrap_or(Cent::new(0));
            extras.borrow_mut().kist_cent = Some(super::rechnen::add_cent(kist_cent, kap)?);
        }
    }
    Ok(est)
}

/// Ein Gesamtfall, in dem jeder Betrag 0 ist — Python-`dict` ohne die Schluessel (`m(k)` = 0).
pub(crate) fn leerer_gesamtfall(vz: Vz, zusammen: bool) -> GesamtfallEingabe {
    use engine::zugriff::teil2::rente::{
        EinkuenfteVersorgungEingabe, VersorgungsfreibetragEingabe,
    };
    let n = Euro::new(0);
    GesamtfallEingabe {
        vz,
        zusammenveranlagung: zusammen,
        einkuenfte_nichtselbststaendig: n,
        einkuenfte_kapitalvermoegen: n,
        einkuenfte_vermietung: n,
        einkuenfte_sonstige: n,
        einkuenfte_gewinn: n,
        altersentlastungsbetrag: n,
        entlastungsbetrag_alleinerziehende: n,
        sonderausgaben: n,
        vorsorge_gesamtbeitraege_inkl_ag: n,
        vorsorge_ag_anteil_steuerfrei: n,
        aussergewoehnliche_belastungen: n,
        freibetraege_kinder: n,
        sonstige_abzuege_vom_einkommen: n,
        anzurechnende_auslaendische_steuern: n,
        steuerermaessigungen: n,
        steuer_kapital_gesondert: n,
        hinzurechnung_kindergeld: n,
        kinder_ganzjaehrig: 0,
        hinzurechnung_zulage: n,
        tarif_modifiziert: false,
        tarifliche_est_modifiziert: n,
        versorgung: EinkuenfteVersorgungEingabe {
            versorgung_jahresrente: n,
            freibetrag: VersorgungsfreibetragEingabe {
                bemessungsgrundlage: n,
                beginn_jahr: 0,
            },
        },
    }
}

/// Aequivalenz von `q_roh_cent` mit `int(v or 0)` (D15); Ausnahmeliste `crate::aequivalenz::INT`.
/// Dazu `kist_steuererhebend` gegen seine Fassung vor K7b.
#[cfg(test)]
mod aequivalenz {
    use domain::testhilfe::{ganzzahl_text, json_wert, pruefe, Ergebnis};
    use domain::{Konfession, PyWert};
    use proptest::prelude::*;
    use serde_json::{json, Value};

    use super::kist_steuererhebend;
    use crate::aequivalenz::{alt_klasse, ein_feld, enum_json, int_oder_null, int_oder_null_wie};
    use crate::vor_k2::int_oder_null_alt;
    use crate::{wert, Felder};

    /// Die Vor-K2-Fassung von `q_roh_cent`. `q_roh_cent` selbst ist seit dem Port die Produktion —
    /// dieser Helfer traegt die alte Gestalt und ist die einzige Seite, die die D-Nummern messt.
    fn alt(v: &Value) -> Ergebnis<i64> {
        alt_klasse(int_oder_null_alt(v))
    }

    /// Die Fassung vor K7b: Text gegen `KIST_STEUERERHEBEND`, fehlend = "keine", Nicht-Text = `None`.
    fn steuererhebend_alt(f: &Felder) -> bool {
        let konfession = match wert(f, "kist_konfession") {
            None => Some("keine"),
            Some(PyWert::Text(s)) => Some(s.as_str()),
            Some(_) => None,
        };
        konfession.is_some_and(|k| ["evangelisch", "roemisch-katholisch"].contains(&k))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1_000))]

        #[test]
        fn q_roh_cent_wie_pywert(v in json_wert()) {
            int_oder_null_wie(&v, &alt(&v))?;
        }

        /// K7b: `kist_steuererhebend` liest ueber `Lage`; die Fassung davor verglich den Text.
        #[test]
        fn kist_steuererhebend_wie_alt(v in enum_json(Konfession::ALLE.map(Konfession::als_str))) {
            let f = v.clone().map_or_else(Felder::new, |w| ein_feld("kist_konfession", w, true));
            pruefe(&v, &steuererhebend_alt(&f), &kist_steuererhebend(&f), Vec::new, &[])?;
        }

        #[test]
        fn q_roh_cent_text_wie_pywert(s in ganzzahl_text()) {
            let v = Value::String(s);
            int_oder_null_wie(&v, &alt(&v))?;
        }
    }

    /// D6: `int("٣" or 0)` ist in `CPython` 3.
    #[test]
    fn d6_nd_ziffer() {
        let v = json!("\u{663}");
        assert_eq!(alt(&v), Err(Some("ValueError")));
        assert_eq!(int_oder_null(&v), Ok(3));
    }

    /// D11: `int(-2.0**63 or 0)` ist in `CPython` `i64::MIN`.
    #[test]
    fn d11_minus_2_hoch_63() {
        let v = json!(-9_223_372_036_854_775_808.0);
        assert_eq!(alt(&v), Err(None));
        assert_eq!(int_oder_null(&v), Ok(i64::MIN));
    }

    /// D16: `int("\x1c42" or 0)` wirft in `CPython` `ValueError`.
    #[test]
    fn d16_steuerzeichen_am_rand() {
        let v = json!("\u{1c}42");
        assert_eq!(alt(&v), Ok(42));
        assert_eq!(int_oder_null(&v), Err(Some("ValueError")));
    }

    /// D17: `int()` mit mehr als 4300 Ziffern wirft in `CPython` `ValueError`.
    #[test]
    fn d17_mehr_als_4300_ziffern() {
        let v = json!(format!("{}5", "0".repeat(4300)));
        assert_eq!(alt(&v), Ok(5));
        assert_eq!(int_oder_null(&v), Err(Some("ValueError")));
    }
}
