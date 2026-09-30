//! Der `gesamt_guard`-Zweig von `_an_gesamt_sperrgrund` (Scheiben `gesamt`, `rentner_gesamt`):
//! Partner, Instanz-Vollstaendigkeit, Rente, Versorgung, § 33b, § 35a/§ 35c, GWG, Kinderbetreuung.
use domain::Sperrgrund;
use rust_decimal::Decimal;
use serde_json::Value;
use store::SnapshotFeld;

use super::einkunft::{betrag_offen, dba_p32b_p16, flag_kapital_gewinn};
use super::{bestaetigt, positiv, py_int_wert, werbungskosten, zahl_wert, Grund, K};
use crate::deklaration::konstanten::{
    GESAMT_PARTNER_19, GESAMT_PARTNER_KAP, RENTNER_22, RENTNER_22_PARTNER, RENTNER_AA_ARTEN,
    VV_GESAMT_FELDER,
};
use crate::deklaration::Cfg;
use crate::{ist_true, ist_zusammen, wert, BescheidFehler, Felder};

/// Alle Pruefungen des `gesamt_guard`-Zweigs in Python-Reihenfolge; endet IMMER (mit `None` oder Grund).
pub(super) fn gesamt_guard(k: &K<'_>, cfg: &Cfg) -> Grund {
    sperre!(dba_p32b_p16(k));
    sperre_o!(flag_kapital_gewinn(k));
    sperre_o!(betrag_offen(k));
    // Person B (#4): bei Zusammenveranlagung der vollstaendig BESTAETIGTE Person-B-Kegel.
    if cfg.partner_19
        && ist_zusammen(k.f)
        && GESAMT_PARTNER_19
            .iter()
            .chain(&GESAMT_PARTNER_KAP)
            .any(|pf| !bestaetigt(k.f, pf))
    {
        return Ok(Some(Sperrgrund::PartnerKegelOffen));
    }
    sperre!(multi_objekt(k, cfg));
    if cfg.rentner {
        sperre!(rente(k, cfg));
    }
    sperre_o!(versorgung(k.f));
    sperre!(behinderung_wahlrecht(k));
    sperre!(p35a_p35c(k));
    sperre!(gwg(k));
    sperre!(kinderbetreuung(k));
    if cfg
        .fremd_arten
        .iter()
        .any(|fl| crate::ist_false(wert(k.f, fl)))
    {
        return Ok(Some(Sperrgrund::EinkunftsartNichtRingFaehig));
    }
    // dHf/Verpflegung sind auch im gesamt/rentner-WK-Pfad verdrahtet.
    werbungskosten::dhf_vpf_grund(k)
}

/// Multi-Objekt § 21 (#5): jede WEITERE `vv_objekt`-Instanz (Index ≥ 2) vollstaendig und bestaetigt.
fn multi_objekt(k: &K<'_>, cfg: &Cfg) -> Grund {
    let (Some(gruppe), Some(_)) = (cfg.multi_objekt, k.q.beide()) else {
        return Ok(None);
    };
    for inst in k.q.instanzen(gruppe)? {
        // Subset-Check (nicht ==): optionale Tatbestand-Felder duerfen zusaetzlich da sein.
        let vollstaendig = VV_GESAMT_FELDER
            .iter()
            .all(|p| inst.felder.contains_key(*p));
        if inst.index >= 2 && (!vollstaendig || inst.zustand != domain::Zustand::Bestaetigt) {
            return Ok(Some(Sperrgrund::VvInstanzOffen));
        }
    }
    Ok(None)
}

/// § 22 aa: `art in RENTNER_AA_ARTEN`, `beginn` ganzzahlig (Python-`bool` zaehlt!), vor dem VZ, und
/// kein Freibetrag (Zahl, kein Bool) → die Euro-Fixierung fehlt.
fn fixierung_offen(
    k: &K<'_>,
    art: Option<&Value>,
    beginn: Option<&Value>,
    rf: Option<&Value>,
) -> bool {
    let art_aa = matches!(art, Some(Value::String(s)) if RENTNER_AA_ARTEN.contains(&s.as_str()));
    let (Some(beginn), Some(vz)) = (py_int_wert(beginn), k.vz) else {
        return false;
    };
    art_aa && beginn < i64::from(vz.jahr()) && zahl_wert(rf).is_none()
}

/// Rentner-Scheibe: Fixierung und Vollstaendigkeit je Rente-Instanz, Person-B-Rente.
fn rente(k: &K<'_>, cfg: &Cfg) -> Grund {
    let f = k.f;
    if let (Some(gruppe), Some(_)) = (cfg.multi_rente, k.q.beide()) {
        // Multi-Rente (#6): Fixierung + Vollstaendigkeit JE Rente-Instanz der Person A.
        for inst in k.q.instanzen(gruppe)? {
            let w = |id: &str| inst.felder.get(id).map(|x: &SnapshotFeld| &x.wert);
            let kern = RENTNER_22.iter().all(|p| inst.felder.contains_key(*p));
            if inst.index >= 2 && (!kern || inst.zustand != domain::Zustand::Bestaetigt) {
                return Ok(Some(Sperrgrund::RenteInstanzOffen));
            }
            if fixierung_offen(
                k,
                w("rentner_renten_art"),
                w("rentner_renten_beginn_jahr"),
                w("rentner_rentenfreibetrag"),
            ) {
                return Ok(Some(Sperrgrund::RentenfreibetragFixierungOffen));
            }
        }
    } else if fixierung_offen(
        k,
        wert(f, "rentner_renten_art"),
        wert(f, "rentner_renten_beginn_jahr"),
        wert(f, "rentner_rentenfreibetrag"),
    ) {
        return Ok(Some(Sperrgrund::RentenfreibetragFixierungOffen));
    }
    if ist_zusammen(f) {
        sperre_o!(rente_partner(k));
    }
    Ok(None)
}

/// Person B: alle vier Kernfelder der Rente bestaetigt oder keins; KV/PV-Weiche braucht
/// `versicherungsart_partner`; aa-Folgejahr-Fixierung fuer die Ehegatten-Rente.
fn rente_partner(k: &K<'_>) -> Option<Sperrgrund> {
    let f = k.f;
    let da = RENTNER_22_PARTNER
        .iter()
        .filter(|p| bestaetigt(f, p))
        .count();
    if da > 0 && da != RENTNER_22_PARTNER.len() {
        return Some(Sperrgrund::RenteInstanzOffen);
    }
    if (positiv(f, "basis_kv_partner") || positiv(f, "basis_pv_partner"))
        && !bestaetigt(f, "versicherungsart_partner")
    {
        return Some(Sperrgrund::PartnerKegelOffen);
    }
    fixierung_offen(
        k,
        wert(f, "rentner_renten_art_partner"),
        wert(f, "rentner_renten_beginn_jahr_partner"),
        wert(f, "rentner_rentenfreibetrag_partner"),
    )
    .then_some(Sperrgrund::RentenfreibetragFixierungOffen)
}

/// § 19 Abs. 2 Versorgungsfreibetrag: Beginnjahr und Bemessungsgrundlage BESTAETIGT gesetzt.
fn versorgung(f: &Felder) -> Option<Sperrgrund> {
    if !positiv(f, "versorgung_jahresrente") {
        return None;
    }
    // `isinstance(x, int) and x > 0` — Pythons bool zaehlt als int (True > 0).
    let beginn_ok = py_int_wert(wert(f, "versorgung_beginn_jahr")).is_some_and(|b| b > 0)
        && bestaetigt(f, "versorgung_beginn_jahr");
    let bmg_ok = zahl_wert(wert(f, "versorgung_bemessungsgrundlage"))
        .is_some_and(|b| b > Decimal::ZERO)
        && bestaetigt(f, "versorgung_bemessungsgrundlage");
    (!(beginn_ok && bmg_ok)).then_some(Sperrgrund::VersorgungsfreibetragOffen)
}

/// Kind-PB-Uebertragung (§ 33b Abs. 5): Kind mit `IdNr` (≥ 11 Zeichen), Antrag und "nicht selbst genutzt".
fn kind_pb_uebertragen(k: &K<'_>) -> Result<bool, BescheidFehler> {
    if k.q.beide().is_none() {
        return Ok(false);
    }
    for inst in k.q.instanzen("kind")? {
        if inst.zustand != domain::Zustand::Bestaetigt {
            continue;
        }
        let w = |id: &str| inst.felder.get(id).map(|x| &x.wert);
        let idnr_lang = matches!(w("kind_idnr"), Some(Value::String(s)) if s.chars().count() >= 11);
        if idnr_lang
            && ist_true(w("kind_behinderten_pb_antrag"))
            && ist_true(w("kind_pb_nicht_selbst_genutzt"))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// § 33b Abs. 1 S. 1 Wahlrecht (Person A und Person B): ein PB vorhanden, Aufwendungen > 0 und das
/// Wahlrecht unbeantwortet sperrt. Kein over-tax-sicherer Default moeglich.
fn behinderung_wahlrecht(k: &K<'_>) -> Grund {
    let f = k.f;
    // GdB >= 20 nach bit-identischer Regel zu `catala_behinderten_pb`; kein Zahlwert = 0.
    let pb = |gdb: &str, hilflos: &str| {
        // PARITÄT: fail-open default — `_gdb_num = _gdb if Zahl else 0`: kein Zahlwert = GdB 0.
        zahl_wert(wert(f, gdb)).unwrap_or(Decimal::ZERO) >= Decimal::from(20)
            || ist_true(wert(f, hilflos))
    };
    if pb(
        "rentner_grad_der_behinderung",
        "rentner_hilflos_blind_taubblind",
    ) && positiv(f, "behinderungsbedingte_aufwendungen")
        && !kind_pb_uebertragen(k)?
        && !bestaetigt(f, "behinderungsbedingte_aufwendungen_wahlrecht_pb")
    {
        return Ok(Some(
            Sperrgrund::BehinderungsbedingteAufwendungenWahlrechtOffen,
        ));
    }
    // Partner-Spiegel: selbstaendiges Wahlrecht, nur bei Zusammenveranlagung.
    Ok((ist_zusammen(f)
        && pb(
            "rentner_grad_der_behinderung_partner",
            "rentner_hilflos_blind_taubblind_partner",
        )
        && positiv(f, "behinderungsbedingte_aufwendungen_partner")
        && !bestaetigt(f, "behinderungsbedingte_aufwendungen_wahlrecht_pb_partner"))
    .then_some(Sperrgrund::BehinderungsbedingteAufwendungenWahlrechtPartnerOffen))
}

/// `_hh_instanz_positiv`: irgendeine Instanz mit Betrag > 0 (jeder Zustand); ohne Store/Bindung der
/// Flat-Wert des Betragsfeldes, ohne Instanz-Treffer der Flat-Wert der Summe (Bestandsdaten).
fn hh_positiv(k: &K<'_>, gruppe: &str, betrag: &str, summe: &str) -> Result<bool, BescheidFehler> {
    if k.q.beide().is_none() {
        return Ok(positiv(k.f, betrag));
    }
    for inst in k.q.instanzen(gruppe)? {
        if crate::ist_positive_zahl(inst.felder.get(betrag).map(|x| &x.wert)) {
            return Ok(true);
        }
    }
    Ok(positiv(k.f, summe))
}

/// § 35a Abs. 3/5 und Abs. 4 (EU/EWR), § 35c Abs. 3 S. 2.
fn p35a_p35c(k: &K<'_>) -> Grund {
    let f = k.f;
    let dienst = |k: &K<'_>| {
        hh_positiv(
            k,
            "hh_dienstleistung",
            "hh_dienstleistung_betrag",
            "hh_dienstleistungen",
        )
    };
    let handwerker = |k: &K<'_>| {
        hh_positiv(
            k,
            "hh_handwerker",
            "hh_handwerker_betrag",
            "hh_handwerker_arbeitskosten",
        )
    };
    // Abs. 5 S. 3 rechnung_unbar: nur Dienstleistung ODER Handwerker (Minijob verlangt keine unbare Zahlung).
    if (dienst(k)? || handwerker(k)?) && !bestaetigt(f, "hh_rechnung_unbar") {
        return Ok(Some(Sperrgrund::RechnungUnbarOffen));
    }
    // Abs. 3 S. 2: gefoerderte Handwerkermassnahmen.
    if handwerker(k)? && !bestaetigt(f, "hh_handwerker_keine_foerderung") {
        return Ok(Some(Sperrgrund::HandwerkerFoerderungOffen));
    }
    // Abs. 4 S. 1: EU/EWR gatet ALLE drei Toepfe; ein explizites false ist eine ANTWORT.
    let minijob = hh_positiv(
        k,
        "hh_minijob",
        "hh_minijob_betrag",
        "hh_minijob_aufwendungen",
    )?;
    if (minijob || dienst(k)? || handwerker(k)?) && !bestaetigt(f, "hh_in_eu_ewr") {
        return Ok(Some(Sperrgrund::HaushaltEuEwrOffen));
    }
    // § 35c Abs. 3 S. 2: die Ermaessigung entfaellt bei Doppelfoerderung — unbeantwortet sperrt.
    let p35c =
        positiv(f, "p35c_sanierungsaufwendungen") || positiv(f, "p35c_energieberater_aufwendungen");
    Ok((p35c && !bestaetigt(f, "p35c_keine_doppelfoerderung"))
        .then_some(Sperrgrund::P35cDoppelfoerderungOffen))
}

/// § 6 Abs. 2 GWG-Sofortabzug S. 1-5: je `gwg`-Instanz mit Betrag in (0, 800 EUR] die drei
/// Voraussetzungen; die Verzeichnis-Frage nur ueber 250 EUR. Explizites false ist eine Antwort.
fn gwg(k: &K<'_>) -> Grund {
    if k.q.beide().is_none() {
        return Ok(None);
    }
    for inst in k.q.instanzen("gwg")? {
        // PARITÄT: fail-open default — `_netto_i = _netto_v if Zahl else 0`: kein Zahlwert = 0.
        let betrag = zahl_wert(
            inst.felder
                .get("gwg_anschaffungskosten_netto")
                .map(|x| &x.wert),
        )
        .unwrap_or(Decimal::ZERO);
        // Ueber 800 EUR netto ist der Sofortabzug ausgeschlossen (Schwelle in Cent).
        if betrag <= Decimal::ZERO || betrag > Decimal::from(80_000) {
            continue;
        }
        let offen = |id: &str| {
            inst.felder
                .get(id)
                .is_none_or(|x| x.zustand != domain::Zustand::Bestaetigt)
        };
        if offen("gwg_bewegliches_selbstaendig_nutzbar")
            || offen("gwg_netto_ohne_vorsteuer")
            || (betrag > Decimal::from(25_000) && offen("gwg_verzeichnis_ab_250"))
        {
            return Ok(Some(Sperrgrund::GwgTatbestandOffen));
        }
    }
    Ok(None)
}

/// § 10 Abs. 1 Nr. 5 S. 2/S. 4 Kinderbetreuung je Kind-Instanz mit Betrag > 0 und bestaetigter
/// Qualifikation aus S. 1. Auch ein BESTAETIGTES "nein" sperrt: der Betrag wird nicht zerlegt.
fn kinderbetreuung(k: &K<'_>) -> Grund {
    if k.q.beide().is_none() {
        return Ok(None);
    }
    for inst in k.q.instanzen("kind")? {
        let w = |id: &str| inst.felder.get(id).map(|x| &x.wert);
        // PARITÄT: fail-open default — kein Zahlwert = 0 (`_aufw_i`), die Instanz zaehlt nicht.
        if zahl_wert(w("kinderbetreuungskosten")).unwrap_or(Decimal::ZERO) <= Decimal::ZERO {
            continue;
        }
        // Nur ein bestaetigt qualifiziertes Kind erreicht den Ring ueberhaupt.
        if !ist_true(w("kind_unter_14_haushaltszugehoerig")) {
            continue;
        }
        let ja_und_bestaetigt = |id: &str| {
            inst.felder.get(id).is_some_and(|x| {
                x.zustand == domain::Zustand::Bestaetigt && x.wert == Value::Bool(true)
            })
        };
        if !ja_und_bestaetigt("kind_betreuung_reine_betreuung") {
            return Ok(Some(Sperrgrund::KinderbetreuungReineBetreuungOffen));
        }
        if !ja_und_bestaetigt("kind_betreuung_rechnung_ueberweisung") {
            return Ok(Some(Sperrgrund::KinderbetreuungZahlungOffen));
        }
    }
    Ok(None)
}
