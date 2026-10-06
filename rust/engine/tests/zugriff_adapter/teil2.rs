//! Sachverhalt-dict (JSON) -> typisierte Teil-2-Eingabe, genau so, wie `runner.py` das dict
//! liest: `s[k]` -> `KeyError` bei Fehlen, `s.get(k, d)` -> Default, Truthiness fuer Flags,
//! Dateizugriff je VZ -> `FileNotFoundError`, `VZ_ENUM[year]` -> `KeyError`, `assert year in`
//! -> `AssertionError`. Wo Python eine Eingabe liest, die der Rust-Typ nicht darstellen kann
//! (VZ ausserhalb 2024-2026, unbekannte Veranlagung), sagt der Adapter die Python-Ausnahme
//! voraus (`Fehl::Py`); der Test prueft diese Vorhersage gegen das Orakel.
//! Liest Python einen Wert gar nicht (z. B. VZ bei `aufwendungen <= 0`), setzt der Adapter einen
//! Platzhalter -- das ist Pythons Verhalten, kein Default.
use bindung::Params;
use domain::{Cent, Euro, Km, Vz};
use engine::tarif::Veranlagung;
use engine::zugriff::teil1::fehler::EngineFehler as Basis;
use engine::zugriff::teil1::werbungskosten::{EntfernungspauschaleEingabe, RaumkostenEingabe};
use engine::zugriff::teil2::est::{self, EstBetrag, Sachverhalt};
use engine::zugriff::teil2::gesamt::{self, GesamtfallEingabe};
use engine::zugriff::teil2::gewerbe::{
    self, GewstAusgabe, GewstEingabe, Hinzurechnung, KstEingabe, Kuerzung,
};
use engine::zugriff::teil2::rente::{
    self, EinkuenfteVersorgungEingabe, Rentenart, VersorgungsfreibetragEingabe,
};
use engine::zugriff::teil2::{
    kapital, p23, p33, p35c, solz, sonderausgaben, sonstige, EngineFehler,
};
use rust_decimal::Decimal;
use serde_json::{json, Map, Value};
use std::str::FromStr;

/// Warum kein Rust-Ergebnis vorliegt.
#[derive(Debug)]
pub enum Fehl {
    /// Der Adapter sagt diese Python-Ausnahme voraus (Eingabe in Rust nicht darstellbar).
    Py(&'static str),
    /// Die Rust-Funktion lieferte `Err`.
    Engine(EngineFehler),
}

impl From<EngineFehler> for Fehl {
    fn from(f: EngineFehler) -> Self {
        Self::Engine(f)
    }
}

pub type Roh = Result<Value, Fehl>;
type D = Map<String, Value>;

fn dict(args: &[Value]) -> &D {
    args.first()
        .and_then(Value::as_object)
        .expect("erstes Argument ist ein dict")
}

/// Python `int(v)` fuer int/bool.
fn int(v: &Value) -> i64 {
    match v {
        Value::Bool(b) => i64::from(*b),
        _ => v
            .as_i64()
            .unwrap_or_else(|| panic!("int() auf {v} nicht modelliert")),
    }
}

/// Python-Truthiness.
fn wahr(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64() != Some(0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
    }
}

fn flag(d: &D, k: &str) -> bool {
    wahr(d.get(k))
}

/// `int(s.get(k, default))`.
fn get(d: &D, k: &str, default: i64) -> i64 {
    d.get(k).map_or(default, int)
}

fn eur(d: &D, k: &str) -> Euro {
    Euro::new(get(d, k, 0))
}

/// `int(s[k])`.
fn req(d: &D, k: &str) -> Result<i64, Fehl> {
    d.get(k).map(int).ok_or(Fehl::Py("KeyError"))
}

fn req_eur(d: &D, k: &str) -> Result<Euro, Fehl> {
    req(d, k).map(Euro::new)
}

fn vz_von(jahr: i64, py_fehler: &'static str) -> Result<Vz, Fehl> {
    u16::try_from(jahr)
        .ok()
        .and_then(|j| Vz::try_from(j).ok())
        .ok_or(Fehl::Py(py_fehler))
}

/// `s["veranlagungszeitraum"]`, dann ein Zugriff, der bei fremdem Jahr mit `py_fehler` scheitert.
fn vz(d: &D, py_fehler: &'static str) -> Result<Vz, Fehl> {
    vz_von(req(d, "veranlagungszeitraum")?, py_fehler)
}

const DATEI: &str = "FileNotFoundError";

fn e(x: Euro) -> Value {
    json!(x.get())
}

fn c(x: Cent) -> Value {
    json!(x.get())
}

pub fn sparer_pb(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let e_ = kapital::SparerPbEingabe {
        vz: vz(d, DATEI)?,
        kapitalertraege: eur(d, "kapitalertraege"),
        zusammenveranlagung: flag(d, "zusammenveranlagung"),
    };
    Ok(e(kapital::sparer_pb(&e_, p)?))
}

pub fn kapital_verrechnung(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(kapital::kapital_verrechnung(
        &kapital::KapitalVerrechnungEingabe {
            gewinn_aktien: eur(d, "gewinn_aktien"),
            verlust_aktien: eur(d, "verlust_aktien"),
            gewinn_sonstige: eur(d, "gewinn_sonstige"),
            verlust_sonstige: eur(d, "verlust_sonstige"),
        },
    )?))
}

pub fn kapital_steuer(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let e_ = kapital::KapitalSteuerEingabe {
        vz: vz(d, DATEI)?,
        kapitaleinkuenfte: eur(d, "kapitaleinkuenfte"),
        est_regulaer_mit_kap: eur(d, "est_regulaer_mit_kap"),
        est_regulaer_ohne_kap: eur(d, "est_regulaer_ohne_kap"),
    };
    Ok(e(kapital::kapital_steuer(&e_, p)?))
}

pub fn renten_einkuenfte(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let jahr = req(d, "veranlagungszeitraum")?;
    let art = d
        .get("renten_art")
        .ok_or(Fehl::Py("KeyError"))?
        .as_str()
        .unwrap_or("");
    let rente = eur(d, "jahresrente");
    let v = vz_von(jahr, DATEI)?; // _renten_wk_pb(year) vor der Art-Pruefung
    let art = match art {
        "private_leibrente" | "sonstige_leibrente" => Rentenart::Bb {
            alter_bei_rentenbeginn: req(d, "alter_bei_rentenbeginn")?,
        },
        "gesetzliche_rente" | "berufsstaendische_versorgung" | "private_basisrente" => {
            Rentenart::Aa {
                renten_beginn_jahr: req(d, "renten_beginn_jahr")?,
                rentenfreibetrag: d
                    .get("rentenfreibetrag")
                    .filter(|v| !v.is_null())
                    .map(|v| Euro::new(int(v))),
            }
        }
        _ => Rentenart::NichtRingfaehig,
    };
    Ok(e(rente::renten_einkuenfte(
        &rente::RentenEingabe {
            vz: v,
            art,
            jahresrente: rente,
        },
        p,
    )?))
}

/// `s.get(k1) or s.get(k2, 0)`.
fn oder(d: &D, k1: &str, k2: &str) -> i64 {
    match d.get(k1) {
        Some(v) if wahr(Some(v)) => int(v),
        _ => get(d, k2, 0),
    }
}

fn versorgungsfreibetrag(d: &D) -> VersorgungsfreibetragEingabe {
    VersorgungsfreibetragEingabe {
        bemessungsgrundlage: Euro::new(oder(
            d,
            "versorgung_bemessungsgrundlage",
            "versorgungsbezuege_bemessungsgrundlage",
        )),
        beginn_jahr: oder(d, "versorgung_beginn_jahr", "versorgungsbeginn_jahr"),
    }
}

fn versorgung(d: &D) -> EinkuenfteVersorgungEingabe {
    EinkuenfteVersorgungEingabe {
        versorgung_jahresrente: eur(d, "versorgung_jahresrente"),
        freibetrag: versorgungsfreibetrag(d),
    }
}

pub fn p19_2_versorgungsfreibetrag(a: &[Value], p: &Params) -> Roh {
    Ok(e(rente::p19_2_versorgungsfreibetrag(
        &versorgungsfreibetrag(dict(a)),
        p,
    )?))
}

pub fn einkuenfte_versorgung(a: &[Value], p: &Params) -> Roh {
    Ok(e(rente::einkuenfte_versorgung(&versorgung(dict(a)), p)?))
}

pub fn behinderten_pb(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    Ok(e(p33::behinderten_pb(
        &p33::BehindertenPbEingabe {
            vz: vz(d, DATEI)?,
            ist_hilflos_blind_taubblind: flag(d, "ist_hilflos_blind_taubblind"),
            grad_der_behinderung: get(d, "grad_der_behinderung", 0),
        },
        p,
    )?))
}

pub fn pflege_pb(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let e_ = p33::PflegePbEingabe {
        vz: vz(d, DATEI)?,
        ist_hilflos: flag(d, "ist_hilflos"),
        pflegegrad: get(d, "pflegegrad", 0),
    };
    Ok(e(p33::pflege_pb(&e_, p)?))
}

pub fn hinterbliebenen_pb(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let e_ = p33::HinterbliebenenPbEingabe {
        vz: vz(d, DATEI)?,
        hat_hinterbliebenenbezuege: flag(d, "hat_hinterbliebenenbezuege"),
    };
    Ok(e(p33::hinterbliebenen_pb(&e_, p)?))
}

pub fn p33_2a_fahrtkostenpauschale(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let e_ = p33::FahrtkostenpauschaleEingabe {
        vz: vz(d, DATEI)?,
        hat_ag_bl_tbl_h: flag(d, "hat_ag_bl_tbl_h"),
        hat_gdb80_oder_70g: flag(d, "hat_gdb80_oder_70g"),
    };
    Ok(e(p33::p33_2a_fahrtkostenpauschale(&e_, p)?))
}

pub fn p33a_unterhalt(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    let v = vz(d, "AssertionError")?;
    Ok(e(p33::p33a_unterhalt(&p33::UnterhaltEingabe {
        vz: v,
        aufwendungen: req_eur(d, "aufwendungen")?,
        kv_pv_beitraege: eur(d, "kv_pv_beitraege"),
        andere_einkuenfte_bezuege: eur(d, "andere_einkuenfte_bezuege"),
    })?))
}

pub fn p33a_ausbildungsfreibetrag(a: &[Value], _: &Params) -> Roh {
    Ok(e(p33::p33a_ausbildungsfreibetrag(get(
        dict(a),
        "anzahl_kinder",
        0,
    ))?))
}

/// VZ fuer Kinderbetreuung/Schulgeld: nur gelesen, wenn `aufwendungen > 0`.
fn vz_bei_aufwand(d: &D) -> Result<Vz, Fehl> {
    if get(d, "aufwendungen", 0) <= 0 {
        return Ok(Vz::Vz2025); // Platzhalter: Python liest den VZ in diesem Pfad nicht
    }
    vz_von(get(d, "veranlagungszeitraum", 2025), DATEI)
}

pub fn p10_1_5_kinderbetreuung(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let e_ = sonderausgaben::KinderbetreuungEingabe {
        vz: vz_bei_aufwand(d)?,
        aufwendungen: eur(d, "aufwendungen"),
    };
    Ok(e(sonderausgaben::p10_1_5_kinderbetreuung(&e_, p)?))
}

pub fn p10_1_9_schulgeld(a: &[Value], p: &Params) -> Roh {
    let d = dict(a);
    let e_ = sonderausgaben::SchulgeldEingabe {
        vz: vz_bei_aufwand(d)?,
        aufwendungen: eur(d, "aufwendungen"),
        splitting: flag(d, "splitting"),
    };
    Ok(e(sonderausgaben::p10_1_9_schulgeld(&e_, p)?))
}

pub fn p10_1a_realsplitting(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(sonderausgaben::p10_1a_realsplitting(
        &sonderausgaben::RealsplittingEingabe {
            unterhaltsleistungen: eur(d, "unterhaltsleistungen"),
            kv_pv_beitraege: eur(d, "kv_pv_beitraege"),
            kv_krankengeld: eur(d, "kv_krankengeld"),
        },
    )?))
}

pub fn p32b_1(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(sonstige::p32b_1(
        &sonstige::ProgressionsvorbehaltEingabe {
            zu_versteuerndes_einkommen: req_eur(d, "zu_versteuerndes_einkommen")?,
            progressionseinkuenfte: req_eur(d, "progressionseinkuenfte")?,
            est_auf_erhoehte_bemessung: req_eur(d, "est_auf_erhoehte_bemessung")?,
        },
    )?))
}

fn gewst_eingabe(d: &D) -> Result<GewstEingabe, Fehl> {
    let v = vz(d, "VZ-nicht-darstellbar")?;
    let ausgabe = if d.get("gewst_output").and_then(Value::as_str) == Some("p35_anrechnung") {
        GewstAusgabe::P35Anrechnung {
            hebesatz: req(d, "gewst_hebesatz")?,
        }
    } else {
        GewstAusgabe::Messbetrag
    };
    Ok(GewstEingabe {
        vz: v,
        ausgabe,
        gewinn_gewerbebetrieb: eur(d, "gewinn_gewerbebetrieb"),
        hinzurechnung: Hinzurechnung {
            entgelte_schulden: eur(d, "gewst_entgelte_schulden"),
            renten: eur(d, "gewst_renten"),
            stille: eur(d, "gewst_stille"),
            miet_beweglich: eur(d, "gewst_miet_beweglich"),
            miet_unbeweglich: eur(d, "gewst_miet_unbeweglich"),
            rechte: eur(d, "gewst_rechte"),
        },
        kuerzung: Kuerzung {
            einheitswert: eur(d, "gewst_einheitswert"),
            grundsteuer: eur(d, "gewst_grundsteuer"),
            gewinnanteile_mitunternehmer: eur(d, "gewst_gewinnanteile_mitunternehmer"),
            schachteldividenden: eur(d, "gewst_schachteldividenden"),
        },
        fehlbetrag_bestand: eur(d, "fehlbetrag_bestand"),
    })
}

pub fn gewst(a: &[Value], _: &Params) -> Roh {
    Ok(c(gewerbe::gewst(&gewst_eingabe(dict(a))?)?))
}

fn kst_eingabe(d: &D) -> Result<KstEingabe, Fehl> {
    Ok(KstEingabe {
        gewinn_estg: eur(d, "gewinn_estg"),
        verdeckte_gewinnausschuettung: eur(d, "verdeckte_gewinnausschuettung"),
        verdeckte_einlage: eur(d, "verdeckte_einlage"),
        personensteuern: eur(d, "personensteuern"),
        geldstrafen: eur(d, "geldstrafen"),
        dividende_bezuege: eur(d, "dividende_bezuege"),
        beteiligung_prozent: get(d, "beteiligung_prozent", 0),
        veraeusserungsgewinn: eur(d, "veraeusserungsgewinn"),
        zinsaufwand: eur(d, "zinsaufwand"),
        zinsertrag: eur(d, "zinsertrag"),
        abschreibungen: eur(d, "abschreibungen"),
        zins_vortrag_bestand: eur(d, "zins_vortrag_bestand"),
        ebitda_vortrag_bestand: eur(d, "ebitda_vortrag_bestand"),
        keine_konzern_oder_nahestehende_b: flag(d, "keine_konzern_oder_nahestehende_b"),
        eigenkapital_escape_c: flag(d, "eigenkapital_escape_c"),
        verlustvortrag_bestand: eur(d, "verlustvortrag_bestand"),
        schaedlicher_erwerb: flag(d, "schaedlicher_erwerb"),
        antrag_8d: flag(d, "antrag_8d"),
        fortfuehrungs_voraussetzungen: flag(d, "fortfuehrungs_voraussetzungen"),
        umsaetze: eur(d, "umsaetze"),
        loehne_gehaelter: eur(d, "loehne_gehaelter"),
        zuwendungen: eur(d, "zuwendungen"),
        gewst_hebesatz: req(d, "gewst_hebesatz")?,
    })
}

pub fn kst_nenner_b(a: &[Value], _: &Params) -> Roh {
    Ok(c(gewerbe::kst_nenner_b(&kst_eingabe(dict(a))?)?))
}

pub fn solz(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    let jahr = req(d, "veranlagungszeitraum")?;
    let basis = req_eur(d, "bemessungsgrundlage")?;
    let kap = eur(d, "kapital_steuer");
    if !d.contains_key("splitting") {
        return Err(Fehl::Py("KeyError"));
    }
    let v = vz_von(jahr, "AssertionError")?;
    Ok(c(solz::solz(&solz::SolzEingabe {
        vz: v,
        bemessungsgrundlage: basis,
        kapital_steuer: kap,
        splitting: flag(d, "splitting"),
    })?))
}

fn gesamtfall(d: &D) -> Result<GesamtfallEingabe, Fehl> {
    Ok(GesamtfallEingabe {
        vz: vz(d, DATEI)?,
        zusammenveranlagung: d.get("veranlagung").and_then(Value::as_str) == Some("zusammen"),
        einkuenfte_nichtselbststaendig: eur(d, "einkuenfte_nichtselbststaendig"),
        einkuenfte_kapitalvermoegen: eur(d, "einkuenfte_kapitalvermoegen"),
        einkuenfte_vermietung: eur(d, "einkuenfte_vermietung"),
        einkuenfte_sonstige: eur(d, "einkuenfte_sonstige"),
        einkuenfte_gewinn: eur(d, "einkuenfte_gewinn"),
        altersentlastungsbetrag: eur(d, "altersentlastungsbetrag"),
        entlastungsbetrag_alleinerziehende: eur(d, "entlastungsbetrag_alleinerziehende"),
        sonderausgaben: eur(d, "sonderausgaben"),
        vorsorge_gesamtbeitraege_inkl_ag: eur(d, "vorsorge_gesamtbeitraege_inkl_ag"),
        vorsorge_ag_anteil_steuerfrei: eur(d, "vorsorge_ag_anteil_steuerfrei"),
        aussergewoehnliche_belastungen: eur(d, "aussergewoehnliche_belastungen"),
        freibetraege_kinder: eur(d, "freibetraege_kinder"),
        sonstige_abzuege_vom_einkommen: eur(d, "sonstige_abzuege_vom_einkommen"),
        anzurechnende_auslaendische_steuern: eur(d, "anzurechnende_auslaendische_steuern"),
        steuerermaessigungen: eur(d, "steuerermaessigungen"),
        steuer_kapital_gesondert: eur(d, "steuer_kapital_gesondert"),
        hinzurechnung_kindergeld: eur(d, "hinzurechnung_kindergeld"),
        kinder_ganzjaehrig: if flag(d, "kinder_ganzjaehrig") {
            get(d, "kinder_ganzjaehrig", 0)
        } else {
            0
        },
        hinzurechnung_zulage: eur(d, "hinzurechnung_zulage"),
        tarif_modifiziert: flag(d, "tarif_modifiziert"),
        tarifliche_est_modifiziert: eur(d, "tarifliche_est_modifiziert"),
        versorgung: versorgung(d),
    })
}

pub fn gesamt(a: &[Value], p: &Params) -> Roh {
    Ok(e(gesamt::gesamt(&gesamtfall(dict(a))?, p)?))
}

pub fn gesamt_gde(a: &[Value], p: &Params) -> Roh {
    Ok(e(gesamt::gesamt_gde(&gesamtfall(dict(a))?, p)?))
}

pub fn gesamt_tarifliche(a: &[Value], p: &Params) -> Roh {
    Ok(e(gesamt::gesamt_tarifliche(&gesamtfall(dict(a))?, p)?))
}

pub fn gesamt_zve(a: &[Value], p: &Params) -> Roh {
    Ok(e(gesamt::gesamt_zve(&gesamtfall(dict(a))?, p)?))
}

pub fn gesamt_kette(a: &[Value], p: &Params) -> Roh {
    let k = gesamt::gesamt_kette(&gesamtfall(dict(a))?, p)?;
    Ok(json!({
        "gesamtbetrag_der_einkuenfte": k.gesamtbetrag_der_einkuenfte.get(),
        "zu_versteuerndes_einkommen": k.zu_versteuerndes_einkommen.get(),
        "tarifliche_est": k.tarifliche_est.get(),
        "festzusetzende_est": k.festzusetzende_est.get(),
    }))
}

pub fn p10d_2(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(gesamt::p10d_2(&gesamt::VerlustabzugEingabe {
        gesamtbetrag_einkuenfte: eur(d, "gesamtbetrag_einkuenfte"),
        verlustvortrag_bestand: eur(d, "verlustvortrag_bestand"),
        zusammenveranlagung: flag(d, "zusammenveranlagung"),
    })?))
}

pub fn ermaessigter_durchschnittssatz(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(gesamt::ermaessigter_durchschnittssatz(
        &gesamt::DurchschnittssatzEingabe {
            ao_einkuenfte: eur(d, "ao_einkuenfte"),
            est_gesamt_zzgl_progression: eur(d, "est_gesamt_zzgl_progression"),
            bemessungsgrundlage_durchschnitt: eur(d, "bemessungsgrundlage_durchschnitt"),
        },
    )?))
}

pub fn p23_veraeusserungsgewinn(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(p23::p23_veraeusserungsgewinn(
        &p23::VeraeusserungsgewinnEingabe {
            veraeusserungspreis: req_eur(d, "veraeusserungspreis")?,
            anschaffungs_herstellungskosten: req_eur(d, "anschaffungs_herstellungskosten")?,
            werbungskosten: req_eur(d, "werbungskosten")?,
        },
    )?))
}

pub fn p23_freigrenze(a: &[Value], _: &Params) -> Roh {
    Ok(e(p23::p23_freigrenze(req_eur(dict(a), "gesamtgewinn")?)))
}

pub fn p23_verlusttopf(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(p23::p23_verlusttopf(&p23::VerlusttopfEingabe {
        gewinn_pvg: req_eur(d, "gewinn_pvg")?,
        verlust_pvg: req_eur(d, "verlust_pvg")?,
    })?))
}

pub fn p34c_1(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(sonstige::p34c_1(
        &sonstige::AuslaendischeSteuerEingabe {
            gezahlte_auslaendische_steuer: req_eur(d, "gezahlte_auslaendische_steuer")?,
            deutsche_est_inkl_ausl: req_eur(d, "deutsche_est_inkl_ausl")?,
            zu_versteuerndes_einkommen: req_eur(d, "zu_versteuerndes_einkommen")?,
            auslaendische_einkuenfte_staat: req_eur(d, "auslaendische_einkuenfte_staat")?,
        },
    )?))
}

fn veranlagung(d: &D) -> Result<Veranlagung, Fehl> {
    match d.get("veranlagung").and_then(Value::as_str) {
        Some("einzel") => Ok(Veranlagung::Einzel),
        Some("zusammen") => Ok(Veranlagung::Zusammen),
        _ => Err(Fehl::Py("ValueError")),
    }
}

fn fuenftel_eingabe(d: &D) -> Result<est::FuenftelEingabe, Fehl> {
    let jahr = req(d, "veranlagungszeitraum")?;
    let zve = req(d, "zu_versteuerndes_einkommen")?;
    let ao = req(d, "ausserordentliche_einkuenfte")?;
    let (veranlagung, vz) = if zve - ao < 0 && zve <= 0 {
        (Veranlagung::Einzel, Vz::Vz2025) // Platzhalter: ValueError vor jedem Tarif-Aufruf
    } else {
        (veranlagung(d)?, vz_von(jahr, "KeyError")?)
    };
    Ok(est::FuenftelEingabe {
        vz,
        veranlagung,
        zu_versteuerndes_einkommen: Euro::new(zve),
        ausserordentliche_einkuenfte: Euro::new(ao),
    })
}

pub fn fuenftel(a: &[Value], _: &Params) -> Roh {
    Ok(e(est::fuenftel(&fuenftel_eingabe(dict(a))?)?))
}

fn est_einzel_eingabe(d: &D) -> Result<est::EstEinzelEingabe, Fehl> {
    let jahr = req(d, "veranlagungszeitraum")?;
    let brutto = req_eur(d, "bruttoarbeitslohn")?;
    Ok(est::EstEinzelEingabe {
        vz: vz_von(jahr, "KeyError")?,
        bruttoarbeitslohn: brutto,
        werbungskosten: eur(d, "werbungskosten"),
        sonderausgaben: eur(d, "sonderausgaben"),
    })
}

pub fn est_einzel_zve(a: &[Value], _: &Params) -> Roh {
    Ok(e(est::est_einzel_zve(&est_einzel_eingabe(dict(a))?)?))
}

fn est_zusammen_eingabe(d: &D) -> Result<est::EstZusammenEingabe, Fehl> {
    Ok(est::EstZusammenEingabe {
        vz: vz(d, "KeyError")?,
        bruttoarbeitslohn_a: eur(d, "bruttoarbeitslohn_a"),
        bruttoarbeitslohn_b: eur(d, "bruttoarbeitslohn_b"),
        werbungskosten_a: eur(d, "werbungskosten_a"),
        werbungskosten_b: eur(d, "werbungskosten_b"),
        sonderausgaben_gemeinsam: eur(d, "sonderausgaben_gemeinsam"),
    })
}

pub fn est_zusammen(a: &[Value], _: &Params) -> Roh {
    Ok(e(est::est_zusammen(&est_zusammen_eingabe(dict(a))?)?))
}

fn sanierung(d: &D) -> p35c::SanierungEingabe {
    p35c::SanierungEingabe {
        sanierungsaufwendungen: eur(d, "sanierungsaufwendungen"),
        ist_uebernaechstes_foerderjahr: flag(d, "ist_uebernaechstes_foerderjahr"),
    }
}

/// `catala_est`: Zweigwahl in Pythons Reihenfolge.
fn sachverhalt(d: &D) -> Result<Sachverhalt, Fehl> {
    if d.contains_key("sanierungsaufwendungen") {
        return Ok(Sachverhalt::Sanierung(sanierung(d)));
    }
    if d.contains_key("bruttolistenpreis") {
        return Ok(Sachverhalt::KfzNutzungswert(
            sonstige::KfzNutzungswertEingabe {
                bruttolistenpreis: eur(d, "bruttolistenpreis"),
                bruchteils_teiler: get(d, "bruchteils_teiler", 1),
            },
        ));
    }
    let jahr = req(d, "veranlagungszeitraum")?;
    Ok(if flag(d, "gesamtfall") {
        Sachverhalt::Gesamtfall(gesamtfall(d)?)
    } else if flag(d, "gewerbesteuer") {
        Sachverhalt::Gewerbesteuer(gewst_eingabe(d)?)
    } else if flag(d, "koerperschaft") {
        Sachverhalt::Koerperschaft(kst_eingabe(d)?)
    } else if d.contains_key("entfernung_km_roh") {
        let v = vz_von(jahr, DATEI)?;
        let km = d.get("entfernung_km_roh").ok_or(Fehl::Py("KeyError"))?;
        Sachverhalt::Entfernungspauschale(EntfernungspauschaleEingabe {
            veranlagungszeitraum: v,
            entfernung_km_roh: Km::new(
                Decimal::from_str(&km.to_string()).expect("Decimal(str(km))"),
            ),
            arbeitstage: req(d, "arbeitstage")?,
            eigenes_oder_ueberlassenes_kfz: flag(d, "eigenes_oder_ueberlassenes_kfz"),
            oepnv_kosten_jahr: eur(d, "oepnv_kosten_jahr"),
        })
    } else if d.contains_key("arbeitszimmer_vorhanden") {
        Sachverhalt::Arbeitszimmer(RaumkostenEingabe {
            veranlagungszeitraum: vz_von(jahr, DATEI)?,
            arbeitszimmer_vorhanden: flag(d, "arbeitszimmer_vorhanden"),
            ist_mittelpunkt: flag(d, "ist_mittelpunkt"),
            tatsaechliche_aufwendungen: eur(d, "tatsaechliche_aufwendungen"),
            jahrespauschale_gewaehlt: flag(d, "jahrespauschale_gewaehlt"),
            monate_ohne_mittelpunkt: get(d, "monate_ohne_mittelpunkt", 0),
            homeoffice_tage: get(d, "homeoffice_tage", 0),
        })
    } else if d.contains_key("bruttoarbeitslohn_a") {
        Sachverhalt::BruttoarbeitslohnZusammen(est_zusammen_eingabe(d)?)
    } else if d.contains_key("bruttoarbeitslohn") {
        Sachverhalt::Bruttoarbeitslohn(est_einzel_eingabe(d)?)
    } else if d.contains_key("ausserordentliche_einkuenfte") {
        Sachverhalt::Fuenftel(fuenftel_eingabe(d)?)
    } else {
        let zve = req_eur(d, "zu_versteuerndes_einkommen")?;
        let veranlagung = veranlagung(d)?;
        Sachverhalt::Tarif(est::TarifEingabe {
            vz: vz_von(jahr, "KeyError")?,
            veranlagung,
            zu_versteuerndes_einkommen: zve,
        })
    })
}

pub fn est(a: &[Value], p: &Params) -> Roh {
    Ok(match est::est(&sachverhalt(dict(a))?, p)? {
        EstBetrag::Euro(x) => e(x),
        EstBetrag::Cent(x) => c(x),
    })
}

pub fn p35c_sanierung(a: &[Value], _: &Params) -> Roh {
    Ok(e(p35c::p35c_sanierung(&sanierung(dict(a)))?))
}

pub fn p35c_energieberater(a: &[Value], _: &Params) -> Roh {
    Ok(e(p35c::p35c_energieberater(eur(
        dict(a),
        "energieberater_aufwendungen",
    ))?))
}

pub fn p35c_jahresdeckel(a: &[Value], _: &Params) -> Roh {
    let d = dict(a);
    Ok(e(p35c::p35c_jahresdeckel(&p35c::JahresdeckelEingabe {
        sanierung_ermaessigung: eur(d, "sanierung_ermaessigung"),
        energieberater_ermaessigung: eur(d, "energieberater_ermaessigung"),
        ist_uebernaechstes_foerderjahr: flag(d, "ist_uebernaechstes_foerderjahr"),
    })?))
}

#[allow(clippy::unnecessary_wraps)] // Signatur = `Adapter`
pub fn p22_nr3_einkuenfte(a: &[Value], _: &Params) -> Roh {
    let betrag = a.first().map(int).expect("ein Argument");
    Ok(c(sonstige::p22_nr3_einkuenfte(Cent::new(betrag))))
}

pub type Adapter = fn(&[Value], &Params) -> Roh;

/// Alle Teil-2-Funktionen, Tarif-/Gesamt-Familie zuerst.
pub const FUNKTIONEN: &[(&str, Adapter)] = &[
    ("gesamt", gesamt),
    ("gesamt_gde", gesamt_gde),
    ("gesamt_tarifliche", gesamt_tarifliche),
    ("gesamt_zve", gesamt_zve),
    ("gesamt_kette", gesamt_kette),
    ("est", est),
    ("est_einzel_zve", est_einzel_zve),
    ("est_zusammen", est_zusammen),
    ("fuenftel", fuenftel),
    ("solz", solz),
    ("sparer_pb", sparer_pb),
    ("kapital_verrechnung", kapital_verrechnung),
    ("kapital_steuer", kapital_steuer),
    ("renten_einkuenfte", renten_einkuenfte),
    ("p19_2_versorgungsfreibetrag", p19_2_versorgungsfreibetrag),
    ("einkuenfte_versorgung", einkuenfte_versorgung),
    ("behinderten_pb", behinderten_pb),
    ("pflege_pb", pflege_pb),
    ("hinterbliebenen_pb", hinterbliebenen_pb),
    ("p33_2a_fahrtkostenpauschale", p33_2a_fahrtkostenpauschale),
    ("p33a_unterhalt", p33a_unterhalt),
    ("p33a_ausbildungsfreibetrag", p33a_ausbildungsfreibetrag),
    ("p10_1_5_kinderbetreuung", p10_1_5_kinderbetreuung),
    ("p10_1_9_schulgeld", p10_1_9_schulgeld),
    ("p10_1a_realsplitting", p10_1a_realsplitting),
    ("p32b_1", p32b_1),
    ("gewst", gewst),
    ("kst_nenner_b", kst_nenner_b),
    ("p10d_2", p10d_2),
    ("p23_veraeusserungsgewinn", p23_veraeusserungsgewinn),
    ("p23_freigrenze", p23_freigrenze),
    ("p23_verlusttopf", p23_verlusttopf),
    ("p34c_1", p34c_1),
    (
        "ermaessigter_durchschnittssatz",
        ermaessigter_durchschnittssatz,
    ),
    ("p35c_sanierung", p35c_sanierung),
    ("p35c_energieberater", p35c_energieberater),
    ("p35c_jahresdeckel", p35c_jahresdeckel),
    ("p22_nr3_einkuenfte", p22_nr3_einkuenfte),
];

/// Python-Ausnahmeklasse fuer eine Rust-Fehlervariante (`None` = Catala-Laufzeitfehler).
pub fn python_typ(f: &EngineFehler) -> Option<&'static str> {
    Some(match f {
        EngineFehler::Basis(Basis::Catala(_)) => return None,
        EngineFehler::Basis(_) => "Basis(kein Python-Gegenstueck)",
        EngineFehler::RentenfreibetragFixierungOffen { .. } => "RentenfreibetragFixierungOffen",
        EngineFehler::VersorgungsfreibetragOffen => "VersorgungsfreibetragOffen",
        EngineFehler::RentenartNichtRingfaehig | EngineFehler::FuenftelZveNichtPositiv => {
            "ValueError"
        }
        EngineFehler::TabelleOhneEintrag { .. } => "KeyError",
        EngineFehler::DivisionDurchNull => "ZeroDivisionError",
    })
}
