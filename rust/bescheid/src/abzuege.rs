//! Abzuege: Kinder (§§ 10 Abs. 1 Nr. 5/9, 33b), Sonderausgaben und aussergewoehnliche Belastungen,
//! dazu zwei kleine geteilte Helfer (OePNV-Kosten, § 34 Abs. 3-Eignung).
//!
//! Quelle: `produkt/bescheid/bescheid_abzuege.py`. Python fuellt hier `g_dict` per Seiteneffekt;
//! Rust liefert die drei betroffenen Werte als [`SteuerSonderAgb`] zurueck, der Zweig setzt sie.
use std::collections::BTreeSet;

use bindung::Params;
use domain::{Cent, Euro, PyWert, Veranlagung, Vz};
use engine::zugriff::teil1::belastungen::{p33_agb, P33AgbEingabe, P33ZumutbarEingabe};
use engine::zugriff::teil1::ermaessigungen::{p35a_haushaltsnahe, P35aHaushaltsnaheEingabe};
use engine::zugriff::teil1::sonderausgaben::{
    p10_1_7_berufsausbildung, p10_kist, p10_kv_pv, p10b_spenden, P1017BerufsausbildungEingabe,
    P10KistEingabe, P10KvPvEingabe, P10bSpendenEingabe,
};
use engine::zugriff::teil2::p33::{
    behinderten_pb, hinterbliebenen_pb, p33_2a_fahrtkostenpauschale, BehindertenPbEingabe,
    FahrtkostenpauschaleEingabe, HinterbliebenenPbEingabe,
};
use engine::zugriff::teil2::p35c::{
    p35c_energieberater, p35c_jahresdeckel, p35c_sanierung, JahresdeckelEingabe, SanierungEingabe,
};
use engine::zugriff::teil2::sonderausgaben::{
    p10_1_5_kinderbetreuung, p10_1_9_schulgeld, p10_1a_realsplitting, KinderbetreuungEingabe,
    RealsplittingEingabe, SchulgeldEingabe,
};
use intervall::Slots;

use crate::{
    cent_zu_euro, euro_plus, feld_euro_oder_null, feld_int_oder_null, ist_false, ist_positive_zahl,
    ist_true, ist_zusammen, minus, plus, positive_zahl, py_int, summe, wert, zahl_oder_null,
    BescheidFehler, Felder, Instanzquelle,
};

/// § 34 Abs. 3 S. 1: (Alter ≥ 55 [aus `geburtsjahr`] ODER dauernd berufsunfaehig) UND § 34 Abs. 3
/// S. 4 nicht schon einmal genutzt. Geteilt zwischen Chooser und Guard; `antrag_ermaessigter_satz`
/// prueft der Aufrufer separat.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`] bei einem `geburtsjahr` jenseits von `i64`.
///
/// ```
/// use bescheid::{abzuege::abs3_eligible, Felder};
/// assert!(!abs3_eligible(&Felder::new(), domain::Vz::Vz2025).unwrap());
/// ```
pub fn abs3_eligible(f: &Felder, vz: Vz) -> Result<bool, BescheidFehler> {
    // PARITÄT: fail-open default — fehlendes geburtsjahr = 0 = "kein Alter bekannt" (kein Fehler).
    let gj = feld_int_oder_null(f, "geburtsjahr")?;
    let alter_ge_55 = gj > 0 && i64::from(vz.jahr()) - gj >= 55;
    let berufsunfaehig = ist_true(wert(f, "dauernd_berufsunfaehig"));
    let einmal_genutzt = ist_true(wert(f, "ermaessigung_einmal_genutzt"));
    Ok((alter_ge_55 || berufsunfaehig) && !einmal_genutzt)
}

/// `oepnv_kosten_jahr` Naht-CENT → EURO (Store liefert Cent, der Runner-Accessor erwartet Euro).
///
/// # Errors
/// [`BescheidFehler::SlotFehlt`] (Python `KeyError`), [`BescheidFehler::Python`] bei einem Wert, den
/// `int()` ablehnt.
///
/// ```
/// use bescheid::abzuege::oepnv_eur;
/// use domain::PyWert;
/// use intervall::Slots;
/// let s: Slots = [("oepnv_kosten_jahr".to_owned(), PyWert::Ganz(-150))].into();
/// assert_eq!(oepnv_eur(&s).unwrap().get(), -2); // Cent → Euro rundet gegen −∞
/// ```
pub fn oepnv_eur(slots: &Slots) -> Result<Euro, BescheidFehler> {
    let v = slots
        .get("oepnv_kosten_jahr")
        .ok_or_else(|| BescheidFehler::SlotFehlt("oepnv_kosten_jahr".into()))?;
    Ok(cent_zu_euro(py_int(v)?))
}

/// `kind_idnr` ist ein Text mit mindestens 11 Zeichen (`not idnr or not str or len(idnr) < 11`).
fn kind_idnr_ok(felder: &Felder) -> bool {
    matches!(wert(felder, "kind_idnr"), Some(PyWert::Text(s)) if s.chars().count() >= 11)
}

/// § 10 Abs. 1 Nr. 3 S. 2: KV/PV-Beitraege des Kindes, `kind_kv + kind_pv` je Kind-Instanz, in CENT
/// (direkt zu `basis_kv`/`basis_pv` addierbar). Ohne `kind_idnr` (S. 2) zaehlt das Kind nicht.
///
/// # Errors
/// [`BescheidFehler::BindungFehlt`], [`BescheidFehler::Snapshot`], [`BescheidFehler::Ueberlauf`].
///
/// ```
/// use bescheid::abzuege::kind_kv_pv_summe;
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::Instanzquelle;
/// use domain::{Cent, Euro, Vz};
/// use serde_json::json;
/// let st = store(&[("kind_idnr", json!("12345678901"), true), ("kind_kv", json!(1000), true), ("kind_pv", json!(500), true)]);
/// let q = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
/// assert_eq!(kind_kv_pv_summe(&q).unwrap().get(), 1500);
/// ```
pub fn kind_kv_pv_summe(q: &Instanzquelle<'_>) -> Result<Cent, BescheidFehler> {
    let mut total = 0_i64;
    for inst in q.instanzen("kind")? {
        if !q.zaehlt(&inst) || !kind_idnr_ok(&inst.felder) {
            continue;
        }
        for fid in ["kind_kv", "kind_pv"] {
            if let Some(betrag) = positive_zahl(wert(&inst.felder, fid))? {
                total = plus(total, betrag)?;
            }
        }
    }
    Ok(Cent::new(total))
}

/// Per-Kind-Summe § 10 Abs. 1 Nr. 5 (Kinderbetreuung). Ein Kind zaehlt nur mit bestaetigtem
/// `kind_unter_14_haushaltszugehoerig` (S. 1).
///
/// # Errors
/// Wie [`kind_kv_pv_summe`], dazu Accessor-Fehler.
///
/// ```
/// use bescheid::abzuege::kinderbetreuung_summe;
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::Instanzquelle;
/// use domain::{Cent, Euro, Vz};
/// use serde_json::json;
/// let st = store(&[("kind_unter_14_haushaltszugehoerig", json!(true), true), ("kinderbetreuungskosten", json!(400_000), true)]);
/// let q = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
/// assert!(kinderbetreuung_summe(&q, Vz::Vz2025, params()).unwrap().get() > 0);
/// ```
pub fn kinderbetreuung_summe(
    q: &Instanzquelle<'_>,
    vz: Vz,
    p: &Params,
) -> Result<Euro, BescheidFehler> {
    let mut total = Euro::new(0);
    for inst in q.instanzen("kind")? {
        if !q.zaehlt(&inst) || !ist_true(wert(&inst.felder, "kind_unter_14_haushaltszugehoerig")) {
            continue;
        }
        if let Some(aufw) = positive_zahl(wert(&inst.felder, "kinderbetreuungskosten"))? {
            let e = KinderbetreuungEingabe {
                vz,
                aufwendungen: cent_zu_euro(aufw),
            };
            total = euro_plus(total, p10_1_5_kinderbetreuung(&e, p)?)?;
        }
    }
    Ok(total)
}

/// § 10 Abs. 1 Nr. 5 S. 1: Kind-Instanzen mit Betrag > 0, deren Geltungsbedingung
/// `kind_unter_14_haushaltszugehoerig` GAR NICHT im Snapshot steht — die offen gelassene Frage.
/// Liefert nur den Feldnamen fuer `offen`, keinen Sperrgrund. `q.nur_bestaetigt` wird nicht gelesen.
///
/// # Errors
/// [`BescheidFehler::BindungFehlt`], [`BescheidFehler::Snapshot`].
///
/// ```
/// use bescheid::abzuege::p10_1_5_gate_fehlend;
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::Instanzquelle;
/// use domain::{Cent, Euro, Vz};
/// use serde_json::json;
/// let st = store(&[("kinderbetreuungskosten", json!(1), true)]);
/// let q = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
/// assert!(p10_1_5_gate_fehlend(&q).unwrap().contains("kind_unter_14_haushaltszugehoerig"));
/// ```
pub fn p10_1_5_gate_fehlend(
    q: &Instanzquelle<'_>,
) -> Result<BTreeSet<&'static str>, BescheidFehler> {
    let mut treffer = BTreeSet::new();
    for inst in q.instanzen("kind")? {
        if inst
            .felder
            .contains_key("kind_unter_14_haushaltszugehoerig")
        {
            continue;
        }
        // Python prueft hier nur `aufw > 0`, ohne `int()` — kein Ueberlauf-Pfad.
        if ist_positive_zahl(wert(&inst.felder, "kinderbetreuungskosten")) {
            treffer.insert("kind_unter_14_haushaltszugehoerig");
        }
    }
    Ok(treffer)
}

/// Per-Kind-Summe § 10 Abs. 1 Nr. 9 (Schulgeld); `f` liefert nur die Veranlagungsart.
///
/// # Errors
/// Wie [`kinderbetreuung_summe`].
///
/// ```
/// use bescheid::abzuege::schulgeld_summe;
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::Instanzquelle;
/// use domain::{Cent, Euro, Vz};
/// use serde_json::json;
/// let st = store(&[("schulgeld", json!(500_000), true)]);
/// let q = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
/// let f = felder(&st);
/// assert!(schulgeld_summe(&q, Vz::Vz2025, &f, params()).unwrap().get() > 0);
/// ```
pub fn schulgeld_summe(
    q: &Instanzquelle<'_>,
    vz: Vz,
    f: &Felder,
    p: &Params,
) -> Result<Euro, BescheidFehler> {
    let splitting = ist_zusammen(f);
    let mut total = Euro::new(0);
    for inst in q.instanzen("kind")? {
        if !q.zaehlt(&inst) {
            continue;
        }
        if let Some(aufw) = positive_zahl(wert(&inst.felder, "schulgeld"))? {
            let e = SchulgeldEingabe {
                vz,
                aufwendungen: cent_zu_euro(aufw),
                splitting,
            };
            total = euro_plus(total, p10_1_9_schulgeld(&e, p)?)?;
        }
    }
    Ok(total)
}

/// Ein Kind mit uebertragenem Pauschbetrag (§ 33b Abs. 5), Eingabe fuer `behinderten_pb` /
/// `hinterbliebenen_pb`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindPbDaten {
    pub grad_der_behinderung: i64,
    pub ist_hilflos_blind_taubblind: bool,
    pub hat_hinterbliebenenbezuege: bool,
}

/// § 33b Abs. 5 Kind-PB-Uebertragung, je Kind. S. 1 "auf Antrag ... wenn ihn das Kind nicht in
/// Anspruch nimmt" (kumulativ), S. 5 `kind_idnr` als Voraussetzung.
///
/// # Errors
/// [`BescheidFehler::BindungFehlt`], [`BescheidFehler::Snapshot`], [`BescheidFehler::Ueberlauf`].
///
/// ```
/// use bescheid::abzuege::kind_behinderten_pb_daten;
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::Instanzquelle;
/// use domain::{Cent, Euro, Vz};
/// use serde_json::json;
/// let st = store(&[("kind_idnr", json!("12345678901"), true), ("kind_behinderten_pb_antrag", json!(true), true), ("kind_pb_nicht_selbst_genutzt", json!(true), true), ("kind_grad_der_behinderung", json!(50), true)]);
/// let q = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
/// let daten = kind_behinderten_pb_daten(&q).unwrap();
/// assert_eq!(daten.len(), 1);
/// assert_eq!(daten[0].grad_der_behinderung, 50);
/// ```
pub fn kind_behinderten_pb_daten(
    q: &Instanzquelle<'_>,
) -> Result<Vec<KindPbDaten>, BescheidFehler> {
    let mut daten = Vec::new();
    for inst in q.instanzen("kind")? {
        if !q.zaehlt(&inst) || !kind_idnr_ok(&inst.felder) {
            continue;
        }
        let f = &inst.felder;
        if !(ist_true(wert(f, "kind_behinderten_pb_antrag"))
            && ist_true(wert(f, "kind_pb_nicht_selbst_genutzt")))
        {
            continue;
        }
        daten.push(KindPbDaten {
            // PARITÄT: fail-open default — fehlender GdB = 0 (dann liefert der Accessor 0 EUR).
            grad_der_behinderung: zahl_oder_null(wert(f, "kind_grad_der_behinderung"))?,
            ist_hilflos_blind_taubblind: ist_true(wert(f, "kind_hilflos_blind_taubblind")),
            hat_hinterbliebenenbezuege: ist_true(wert(f, "kind_hinterbliebenen_uebertragung")),
        });
    }
    Ok(daten)
}

/// Summe der auf die Eltern uebertragenen Kind-Pauschbetraege (§ 33b Abs. 5), EURO — additiv zum
/// eigenen PB.
///
/// # Errors
/// Wie [`kind_behinderten_pb_daten`], dazu Accessor-Fehler.
///
/// ```
/// use bescheid::abzuege::p33b_kind_pauschbetraege;
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::Instanzquelle;
/// use domain::{Cent, Euro, Vz};
/// use serde_json::json;
/// let st = store(&[("kind_idnr", json!("12345678901"), true), ("kind_behinderten_pb_antrag", json!(true), true), ("kind_pb_nicht_selbst_genutzt", json!(true), true), ("kind_grad_der_behinderung", json!(50), true)]);
/// let q = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
/// assert!(p33b_kind_pauschbetraege(&q, Vz::Vz2025, params()).unwrap().get() > 0);
/// ```
pub fn p33b_kind_pauschbetraege(
    q: &Instanzquelle<'_>,
    vz: Vz,
    p: &Params,
) -> Result<Euro, BescheidFehler> {
    let mut summe = Euro::new(0);
    for kd in kind_behinderten_pb_daten(q)? {
        let e = BehindertenPbEingabe {
            vz,
            ist_hilflos_blind_taubblind: kd.ist_hilflos_blind_taubblind,
            grad_der_behinderung: kd.grad_der_behinderung,
        };
        summe = euro_plus(summe, behinderten_pb(&e, p)?)?;
        if kd.hat_hinterbliebenenbezuege {
            let h = HinterbliebenenPbEingabe {
                vz,
                hat_hinterbliebenenbezuege: true,
            };
            summe = euro_plus(summe, hinterbliebenen_pb(&h, p)?)?;
        }
    }
    Ok(summe)
}

/// Die drei Werte, die `_shared_steuer_sonder_agb` in `g_dict` schreibt (alle EURO).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SteuerSonderAgb {
    pub steuerermaessigungen: Euro,
    pub sonderausgaben: Euro,
    pub aussergewoehnliche_belastungen: Euro,
}

/// § 35a-Einzelaufstellung: stumpfe Σ ueber alle `<gruppe>`-Instanzen (CENT). Ohne Store oder ohne
/// Bindung nur die Instanz-1-Basis; ohne Instanz-Summe > 0 der alte Flat-Wert `sum_fid`.
fn hh_summe(
    f: &Felder,
    q: &Instanzquelle<'_>,
    betrag_fid: &str,
    gruppe: &str,
    sum_fid: &str,
) -> Result<i64, BescheidFehler> {
    if q.beide().is_none() {
        return feld_int_oder_null(f, betrag_fid);
    }
    let mut total = 0_i64;
    for inst in q.instanzen(gruppe)? {
        if !q.zaehlt(&inst) {
            continue;
        }
        total = plus(total, zahl_oder_null(wert(&inst.felder, betrag_fid))?)?;
    }
    if total > 0 {
        return Ok(total);
    }
    // Bestandsdaten-Fallback: `f` ist bei nur_bestaetigt schon auf bestaetigt gefiltert.
    feld_int_oder_null(f, sum_fid)
}

/// § 35a (haushaltsnahe) + § 35c (Sanierung, Energieberater, Jahresdeckel), EURO.
fn steuerermaessigungen(f: &Felder, q: &Instanzquelle<'_>) -> Result<Euro, BescheidFehler> {
    let base = p35a_haushaltsnahe(&P35aHaushaltsnaheEingabe {
        hh_minijob_aufwendungen: cent_zu_euro(hh_summe(
            f,
            q,
            "hh_minijob_betrag",
            "hh_minijob",
            "hh_minijob_aufwendungen",
        )?),
        hh_dienstleistungen: cent_zu_euro(hh_summe(
            f,
            q,
            "hh_dienstleistung_betrag",
            "hh_dienstleistung",
            "hh_dienstleistungen",
        )?),
        hh_handwerker_arbeitskosten: cent_zu_euro(hh_summe(
            f,
            q,
            "hh_handwerker_betrag",
            "hh_handwerker",
            "hh_handwerker_arbeitskosten",
        )?),
        hh_in_eu_ewr: ist_true(wert(f, "hh_in_eu_ewr")),
        hh_rechnung_unbar: ist_true(wert(f, "hh_rechnung_unbar")),
        // Abs. 3 S. 2: nur ein explizites False heisst "gefoerdert".
        hh_handwerker_gefoerdert: ist_false(wert(f, "hh_handwerker_keine_foerderung")),
        p35a_mitveranlagung: ist_true(wert(f, "p35a_mitveranlagung")),
    })?;
    // § 35c Abs. 3 S. 2: bestaetigt-False heisst "es gibt eine Doppelfoerderung" -> beide Toepfe 0.
    let doppelt = ist_false(wert(f, "p35c_keine_doppelfoerderung"));
    let uebernaechstes = ist_true(wert(f, "p35c_ist_uebernaechstes_foerderjahr"));
    let sanierung = if doppelt {
        Euro::new(0)
    } else {
        p35c_sanierung(&SanierungEingabe {
            sanierungsaufwendungen: feld_euro_oder_null(f, "p35c_sanierungsaufwendungen")?,
            ist_uebernaechstes_foerderjahr: uebernaechstes,
        })?
    };
    let energieberater = if doppelt {
        Euro::new(0)
    } else {
        p35c_energieberater(feld_euro_oder_null(f, "p35c_energieberater_aufwendungen")?)?
    };
    let deckel = p35c_jahresdeckel(&JahresdeckelEingabe {
        sanierung_ermaessigung: sanierung,
        energieberater_ermaessigung: energieberater,
        ist_uebernaechstes_foerderjahr: uebernaechstes,
    })?;
    euro_plus(base, deckel)
}

/// KV/PV-Sonderausgaben einer Person; `partner` waehlt die `_partner`-Felder (ohne Kind-Beitraege).
fn kv_pv_sonderausgaben(
    f: &Felder,
    partner: bool,
    kind_kv_pv: i64,
) -> Result<Euro, BescheidFehler> {
    let s = if partner { "_partner" } else { "" };
    let fid = |name: &str| format!("{name}{s}");
    let basis = summe(&[
        feld_int_oder_null(f, &fid("basis_kv"))?,
        feld_int_oder_null(f, &fid("basis_pv"))?,
        kind_kv_pv,
    ])?;
    let weitere = summe(&[
        feld_int_oder_null(f, &fid("vorsorge_arbeitslosenversicherung"))?,
        feld_int_oder_null(f, &fid("vorsorge_erwerbsunfaehigkeit"))?,
        feld_int_oder_null(f, &fid("vorsorge_unfall_haftpflicht"))?,
        feld_int_oder_null(f, &fid("vorsorge_rv_alt_mit_ueberschuss"))?,
        feld_int_oder_null(f, &fid("vorsorge_rv_alt_ohne_ueberschuss"))?,
    ])?;
    Ok(p10_kv_pv(&P10KvPvEingabe {
        basis_kv_pv: cent_zu_euro(basis),
        weitere_vorsorgeaufwendungen: cent_zu_euro(weitere),
        mit_anspruch_auf_zuschuss: ist_true(wert(f, &fid("mit_anspruch_auf_zuschuss"))),
    })?)
}

/// Sonderausgaben (§ 10, § 10b, § 10 Abs. 1a), EURO.
fn sonderausgaben(
    gde: Euro,
    veranlagung: Veranlagung,
    f: &Felder,
    vz: Vz,
    q: &Instanzquelle<'_>,
    p: &Params,
) -> Result<Euro, BescheidFehler> {
    let mut sa = p10b_spenden(&P10bSpendenEingabe {
        zuwendungen: feld_euro_oder_null(f, "spenden_betrag")?,
        gesamtbetrag_der_einkuenfte: gde,
    })?;
    sa = euro_plus(
        sa,
        p10_kist(&P10KistEingabe {
            gezahlte_kirchensteuer: feld_euro_oder_null(f, "kist_gezahlt")?,
            erstattete_kirchensteuer: feld_euro_oder_null(f, "kist_erstattet")?,
        })?,
    )?;
    sa = euro_plus(
        sa,
        kv_pv_sonderausgaben(f, false, kind_kv_pv_summe(q)?.get())?,
    )?;
    sa = euro_plus(sa, kinderbetreuung_summe(q, vz, p)?)?;
    sa = euro_plus(sa, schulgeld_summe(q, vz, f, p)?)?;
    if ist_true(wert(f, "realsplitting_zustimmung")) {
        // kv_krankengeld ist TEILMENGE von kv_pv_beitraege (96-%-Kuerzung im Accessor).
        let e = RealsplittingEingabe {
            unterhaltsleistungen: feld_euro_oder_null(f, "realsplitting_unterhaltsleistungen")?,
            kv_pv_beitraege: feld_euro_oder_null(f, "realsplitting_empfaenger_kv_pv")?,
            kv_krankengeld: feld_euro_oder_null(f, "realsplitting_empfaenger_kv_krankengeld")?,
        };
        sa = euro_plus(sa, p10_1a_realsplitting(&e)?)?;
    }
    if veranlagung == Veranlagung::Zusammen {
        sa = euro_plus(sa, kv_pv_sonderausgaben(f, true, 0)?)?;
    }
    let ausb = P1017BerufsausbildungEingabe {
        berufsausbildung_aufwendungen: feld_euro_oder_null(f, "berufsausbildung_aufwendungen")?,
    };
    euro_plus(sa, p10_1_7_berufsausbildung(&ausb)?)
}

/// Eigener oder Partner-Pauschbetrag (§ 33b Abs. 1) nach den Feldern `rentner_grad_der_behinderung*`.
fn eigener_pb(f: &Felder, vz: Vz, p: &Params, partner: bool) -> Result<Euro, BescheidFehler> {
    let s = if partner { "_partner" } else { "" };
    // PARITÄT: fail-open default — fehlender GdB = 0.
    let gdb = feld_int_oder_null(f, &format!("rentner_grad_der_behinderung{s}"))?;
    Ok(behinderten_pb(
        &BehindertenPbEingabe {
            vz,
            grad_der_behinderung: gdb,
            ist_hilflos_blind_taubblind: ist_true(wert(
                f,
                &format!("rentner_hilflos_blind_taubblind{s}"),
            )),
        },
        p,
    )?)
}

/// § 33 Abs. 1/2 (agB) mit § 33b-Wahlrecht, § 33b Abs. 5 S. 4 und dem Tatbestands-Gate, EURO.
fn aussergewoehnliche_belastungen(
    gde: Euro,
    ausserg: Euro,
    veranlagung: Veranlagung,
    f: &Felder,
    vz: Vz,
    q: &Instanzquelle<'_>,
    p: &Params,
) -> Result<Euro, BescheidFehler> {
    let zusammen = veranlagung == Veranlagung::Zusammen;
    let mut ausserg = ausserg;
    let mut agb_cent = feld_int_oder_null(f, "agb_aufwendungen")?;
    let eigener_pb_eur = eigener_pb(f, vz, p, false)?;
    let wahlrecht_pb = wert(f, "behinderungsbedingte_aufwendungen_wahlrecht_pb");
    // Beide Summanden in CENT, genau eine Division am Ende.
    let gekuerzt = |agb: i64, fid: &str| -> Result<i64, BescheidFehler> {
        Ok(minus(agb, feld_int_oder_null(f, fid)?)?.max(0))
    };
    if !kind_behinderten_pb_daten(q)?.is_empty()
        || (eigener_pb_eur.get() > 0 && ist_true(wahlrecht_pb))
    {
        agb_cent = gekuerzt(agb_cent, "behinderungsbedingte_aufwendungen")?;
    } else if eigener_pb_eur.get() > 0 && ist_false(wahlrecht_pb) {
        ausserg = Euro::new(minus(ausserg.get(), eigener_pb_eur.get())?.max(0));
    }
    // Partner-Spiegel: EIGENSTAENDIGE Kette (beide Wahlrechte sind unabhaengig).
    if zusammen {
        let partner_pb_eur = eigener_pb(f, vz, p, true)?;
        let wahlrecht_partner = wert(f, "behinderungsbedingte_aufwendungen_wahlrecht_pb_partner");
        if partner_pb_eur.get() > 0 && ist_true(wahlrecht_partner) {
            agb_cent = gekuerzt(agb_cent, "behinderungsbedingte_aufwendungen_partner")?;
        } else if partner_pb_eur.get() > 0 && ist_false(wahlrecht_partner) {
            ausserg = Euro::new(minus(ausserg.get(), partner_pb_eur.get())?.max(0));
        }
    }
    // § 33 Abs. 1/2 S. 1: nur ein bestaetigtes NEIN nullt den Abzug (`is False`, nicht `is not True`).
    let tatbestand_verneint =
        ist_false(wert(f, "agb_zwangslaeufig")) || ist_false(wert(f, "agb_notwendig_angemessen"));
    if tatbestand_verneint {
        return Ok(ausserg);
    }
    let fahrt = p33_2a_fahrtkostenpauschale(
        &FahrtkostenpauschaleEingabe {
            vz,
            hat_gdb80_oder_70g: ist_true(wert(f, "fahrtkosten_pausch_gdb80_oder_70g")),
            hat_ag_bl_tbl_h: ist_true(wert(f, "fahrtkosten_pausch_ag_bl_tbl_h")),
        },
        p,
    )?;
    let abzug = p33_agb(&P33AgbEingabe {
        aussergewoehnliche_belastungen: euro_plus(cent_zu_euro(agb_cent), fahrt)?,
        zumutbar: P33ZumutbarEingabe {
            gesamtbetrag_der_einkuenfte: gde,
            // PARITÄT: fail-open default — fehlende Kinderzahl = 0 (hoechster Satz).
            anzahl_kinder: feld_int_oder_null(f, "fam_anzahl_kinder")?,
            splitting: zusammen,
        },
    })?;
    euro_plus(ausserg, abzug)
}

/// Steuerermaessigungen (§ 35a/§ 35c), Sonderausgaben und agB in EINEM Lauf.
///
/// `gde` ist der Gesamtbetrag der Einkuenfte, `ausserg` die bereits aufgebaute agB-Basis (z. B.
/// Kind-Pauschbetraege); die Funktion kuerzt sie nur (§ 33b-Wahlrecht) und addiert den § 33-Abzug.
///
/// # Errors
/// Accessor-, Instanz- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::abzuege::shared_steuer_sonder_agb;
/// use bescheid::testhilfe::{felder, index, params, store};
/// use bescheid::Instanzquelle;
/// use domain::{Cent, Euro, Vz};
/// use serde_json::json;
/// use domain::Veranlagung;
/// let f = felder(&store(&[("agb_aufwendungen", json!(1_000_000), true)]));
/// let leer = Instanzquelle { store: None, bindung: Some(index()), nur_bestaetigt: true };
/// let s = shared_steuer_sonder_agb(Euro::new(20_000), Euro::new(0), Veranlagung::Einzel, &f, Vz::Vz2025, &leer, params()).unwrap();
/// assert!(s.aussergewoehnliche_belastungen.get() > 0);
/// ```
pub fn shared_steuer_sonder_agb(
    gde: Euro,
    ausserg: Euro,
    veranlagung: Veranlagung,
    f: &Felder,
    vz: Vz,
    q: &Instanzquelle<'_>,
    p: &Params,
) -> Result<SteuerSonderAgb, BescheidFehler> {
    Ok(SteuerSonderAgb {
        steuerermaessigungen: steuerermaessigungen(f, q)?,
        sonderausgaben: sonderausgaben(gde, veranlagung, f, vz, q, p)?,
        aussergewoehnliche_belastungen: aussergewoehnliche_belastungen(
            gde,
            ausserg,
            veranlagung,
            f,
            vz,
            q,
            p,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::testhilfe::{felder, index, params, store};

    fn q(s: Option<&store::Store>, nur_bestaetigt: bool) -> Instanzquelle<'_> {
        Instanzquelle {
            store: s,
            bindung: Some(index()),
            nur_bestaetigt,
        }
    }

    #[test]
    fn abs3_alter_berufsunfaehig_und_einmal_genutzt() {
        let f = felder(&store(&[("geburtsjahr", json!(1960), true)]));
        assert!(abs3_eligible(&f, Vz::Vz2025).unwrap());
        let f = felder(&store(&[
            ("geburtsjahr", json!(1960), true),
            ("ermaessigung_einmal_genutzt", json!(true), true),
        ]));
        assert!(!abs3_eligible(&f, Vz::Vz2025).unwrap());
        let f = felder(&store(&[
            ("geburtsjahr", json!(1990), true),
            ("dauernd_berufsunfaehig", json!(true), true),
        ]));
        assert!(abs3_eligible(&f, Vz::Vz2025).unwrap());
        let f = felder(&store(&[("geburtsjahr", json!(1990), true)]));
        assert!(!abs3_eligible(&f, Vz::Vz2025).unwrap());
    }

    #[test]
    fn oepnv_rundet_gegen_minus_unendlich() {
        let s: Slots = [("oepnv_kosten_jahr".to_owned(), PyWert::Ganz(-150))].into();
        assert_eq!(oepnv_eur(&s).unwrap(), Euro::new(-2));
        assert!(matches!(
            oepnv_eur(&Slots::new()),
            Err(BescheidFehler::SlotFehlt(_))
        ));
    }

    /// Ein bestaetigtes Kind mit `kind_idnr`, KV 1.000 und PV 500 Cent; das zweite ohne `IdNr` zaehlt nicht.
    #[test]
    fn kind_kv_pv_verlangt_idnr_und_bestaetigung() {
        let st = store(&[
            ("kind_idnr", json!("12345678901"), true),
            ("kind_kv", json!(1000), true),
            ("kind_pv", json!(500), true),
            ("kind_idnr__2", json!("kurz"), true),
            ("kind_kv__2", json!(9999), true),
        ]);
        assert_eq!(
            kind_kv_pv_summe(&q(Some(&st), true)).unwrap(),
            Cent::new(1500)
        );
        assert_eq!(kind_kv_pv_summe(&q(None, true)).unwrap(), Cent::new(0));
        // Vorlaeufiges KV: die Instanz ist nicht bestaetigt → bei nur_bestaetigt zaehlt sie nicht.
        let st = store(&[
            ("kind_idnr", json!("12345678901"), true),
            ("kind_kv", json!(1000), false),
        ]);
        assert_eq!(kind_kv_pv_summe(&q(Some(&st), true)).unwrap(), Cent::new(0));
        assert_eq!(
            kind_kv_pv_summe(&q(Some(&st), false)).unwrap(),
            Cent::new(1000)
        );
    }

    #[test]
    fn store_ohne_bindung_ist_ein_fehler_aber_ohne_store_nicht() {
        let st = store(&[("kind_kv", json!(1), true)]);
        let ohne = Instanzquelle {
            store: Some(&st),
            bindung: None,
            nur_bestaetigt: true,
        };
        assert!(matches!(
            kind_kv_pv_summe(&ohne),
            Err(BescheidFehler::BindungFehlt)
        ));
        let leer = Instanzquelle {
            store: None,
            bindung: None,
            nur_bestaetigt: true,
        };
        assert_eq!(kind_kv_pv_summe(&leer).unwrap(), Cent::new(0));
    }

    #[test]
    fn kinderbetreuung_braucht_bestaetigtes_gate() {
        let mit = store(&[
            ("kind_unter_14_haushaltszugehoerig", json!(true), true),
            ("kinderbetreuungskosten", json!(400_000), true),
        ]);
        let ohne = store(&[
            ("kind_unter_14_haushaltszugehoerig", json!(false), true),
            ("kinderbetreuungskosten", json!(400_000), true),
        ]);
        let p = params();
        assert!(
            kinderbetreuung_summe(&q(Some(&mit), true), Vz::Vz2025, p)
                .unwrap()
                .get()
                > 0
        );
        assert_eq!(
            kinderbetreuung_summe(&q(Some(&ohne), true), Vz::Vz2025, p).unwrap(),
            Euro::new(0)
        );
        assert!(p10_1_5_gate_fehlend(&q(Some(&mit), true))
            .unwrap()
            .is_empty());
        let fehlt = store(&[("kinderbetreuungskosten", json!(1), true)]);
        assert_eq!(
            p10_1_5_gate_fehlend(&q(Some(&fehlt), true)).unwrap().len(),
            1
        );
    }

    #[test]
    fn shared_agb_tatbestand_nein_nullt_den_abzug() {
        let p = params();
        let basis = [("agb_aufwendungen", json!(1_000_000), true)];
        let mit = felder(&store(&basis));
        let leer = q(None, true);
        let ja = shared_steuer_sonder_agb(
            Euro::new(20_000),
            Euro::new(0),
            Veranlagung::Einzel,
            &mit,
            Vz::Vz2025,
            &leer,
            p,
        )
        .unwrap();
        assert!(ja.aussergewoehnliche_belastungen.get() > 0);
        let nein = felder(&store(&[
            basis[0].clone(),
            ("agb_zwangslaeufig", json!(false), true),
        ]));
        let n = shared_steuer_sonder_agb(
            Euro::new(20_000),
            Euro::new(7),
            Veranlagung::Einzel,
            &nein,
            Vz::Vz2025,
            &leer,
            p,
        )
        .unwrap();
        assert_eq!(n.aussergewoehnliche_belastungen, Euro::new(7)); // nur die Basis `ausserg`
    }
}
