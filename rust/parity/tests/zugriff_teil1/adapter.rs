//! Adapter: rohes Sachverhalt-Dict (JSON, wie das Produkt es an `runner.catala_*` uebergibt) ->
//! typisierte `engine::zugriff::teil1`-Eingabe, GENAU so, wie Python es liest: `int(...)`-Casts,
//! Wahrheitswert von `bool(...)`/`Bool(...)`, `s.get(k, default)` -> Default, `s[k]` -> `KeyError`.
//!
//! Absichtlich dumm und explizit: eine Funktion je Accessor, jede Zeile entspricht einer
//! Python-Lesestelle. Grenze: die Reihenfolge der Lesestellen ist nachgebildet, soweit sie den
//! Ausnahmetyp bestimmen kann (VZ vor Parametern vor Betraegen). Ein Typfehler in einem Feld, das
//! Python erst NACH einem Catala-Fehler liest, liefert hier den Typfehler -- solche Eingaben
//! (nicht-numerische Strings, Listen) erzeugt der Generator nicht, und der Korpus enthaelt keine.
use std::str::FromStr;

use domain::{Cent, Euro, Km, Vz};
use engine::zugriff::teil1::afa::{P62GwgEingabe, P7LinearAfaEingabe};
use engine::zugriff::teil1::belastungen::{P33AgbEingabe, P33ZumutbarEingabe};
use engine::zugriff::teil1::einkuenfte::{
    EinkuenfteNichtselbststaendigEingabe, EuerGewinnEingabe, MitunternehmerEinkuenfteEingabe,
    P164FreibetragEingabe, P212VerbilligtEingabe, P3Nr72PhotovoltaikEingabe,
    VermietungEinkuenfteEingabe,
};
use engine::zugriff::teil1::ermaessigungen::{
    KistEingabe, P24aAltersentlastungEingabe, P24bEntlastungEingabe, P31FamilienleistungEingabe,
    P35aHaushaltsnaheEingabe, P36AbschlusszahlungEingabe,
};
use engine::zugriff::teil1::mobilitaetspraemie::P101Eingabe;
use engine::zugriff::teil1::reisekosten::{DhfEingabe, UebernachtungEingabe, VerpflegungEingabe};
use engine::zugriff::teil1::sonderausgaben::{
    P1017BerufsausbildungEingabe, P10KistEingabe, P10KvPvEingabe, P10bSpendenEingabe,
};
use engine::zugriff::teil1::werbungskosten::{
    EntfernungspauschaleEingabe, RaumkostenEingabe, WerbungskostenNEingabe,
};
use rust_decimal::Decimal;
use serde_json::{Map, Value};

/// Die Python-Ausnahmeklasse, die an derselben Lesestelle entstuende. `NichtAbgebildet`: eine
/// Eingabeform, die dieser Adapter bewusst nicht nachbildet (kein Paritaetsvergleich moeglich).
pub type PyFehler = &'static str;
pub const NICHT_ABGEBILDET: PyFehler = "NichtAbgebildet";
type R<T> = Result<T, PyFehler>;
type S = Map<String, Value>;

/// Das eine Dict-Argument `s`.
pub fn dict(args: &[Value]) -> R<&S> {
    args.first()
        .and_then(Value::as_object)
        .ok_or(NICHT_ABGEBILDET)
}

/// Pythons `int(v)`.
pub fn int_von(v: &Value) -> R<i64> {
    match v {
        Value::Bool(b) => Ok(i64::from(*b)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i)
            } else {
                let f = n.as_f64().ok_or(NICHT_ABGEBILDET)?.trunc();
                // `as` ist hier die gewollte Umwandlung; der Bereich ist vorher geprueft.
                #[allow(clippy::cast_possible_truncation)]
                if f.abs() < 9.0e15 {
                    Ok(f as i64)
                } else {
                    Err(NICHT_ABGEBILDET)
                }
            }
        }
        Value::String(t) => t.trim().parse::<i64>().map_err(|_| "ValueError"),
        Value::Null | Value::Array(_) | Value::Object(_) => Err("TypeError"),
    }
}

/// Pythons Wahrheitswert.
pub fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(t) => !t.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

/// `int(s.get(k, default))`.
pub fn get_int(s: &S, k: &str, default: i64) -> R<i64> {
    s.get(k).map_or(Ok(default), int_von)
}

/// `int(s[k])`.
pub fn req_int(s: &S, k: &str) -> R<i64> {
    int_von(s.get(k).ok_or("KeyError")?)
}

fn euro(s: &S, k: &str) -> R<Euro> {
    get_int(s, k, 0).map(Euro::new)
}

fn cent(s: &S, k: &str) -> R<Cent> {
    get_int(s, k, 0).map(Cent::new)
}

/// `bool(s.get(k, default))` / `Bool(s.get(k, default))`.
pub fn get_truthy(s: &S, k: &str, default: bool) -> bool {
    s.get(k).map_or(default, truthy)
}

/// `s.get(k, {}).get("wert") is <erwartet>`.
pub fn wert_ist(s: &S, k: &str, erwartet: bool) -> R<bool> {
    match s.get(k) {
        None => Ok(false),
        Some(Value::Object(o)) => Ok(o.get("wert") == Some(&Value::Bool(erwartet))),
        Some(_) => Err("AttributeError"),
    }
}

/// `str(v)` fuer die Werte, die der Generator erzeugt.
pub fn py_str(v: &Value) -> R<String> {
    match v {
        Value::String(t) => Ok(t.clone()),
        Value::Bool(true) => Ok("True".into()),
        Value::Bool(false) => Ok("False".into()),
        Value::Null => Ok("None".into()),
        Value::Number(n) if n.is_i64() => Ok(n.to_string()),
        _ => Err(NICHT_ABGEBILDET),
    }
}

fn vz_aus_jahr(j: i64) -> Option<Vz> {
    u16::try_from(j).ok().and_then(|j| Vz::try_from(j).ok())
}

/// `s["veranlagungszeitraum"]` als Pfadbestandteil `params/<str(vz)>/...`: fehlt -> `KeyError`,
/// kein Verzeichnis -> `FileNotFoundError`.
pub fn vz_pfad(s: &S) -> R<Vz> {
    match s.get("veranlagungszeitraum").ok_or("KeyError")? {
        Value::Number(n) if n.is_i64() => {
            n.as_i64().and_then(vz_aus_jahr).ok_or("FileNotFoundError")
        }
        Value::Bool(_) | Value::Null => Err("FileNotFoundError"),
        _ => Err(NICHT_ABGEBILDET),
    }
}

/// `VZ_ENUM[s["veranlagungszeitraum"]]` (Dict mit int-Schluesseln).
pub fn vz_enum(s: &S) -> R<Vz> {
    match s.get("veranlagungszeitraum").ok_or("KeyError")? {
        Value::Number(n) if n.is_i64() => n.as_i64().and_then(vz_aus_jahr).ok_or("KeyError"),
        Value::Number(_) => Err(NICHT_ABGEBILDET),
        _ => Err("KeyError"),
    }
}

/// `Decimal(str(v))` (`catala_runtime`: `Fraction(str(v))`, exakt).
pub fn decimal_str(v: &Value) -> R<Decimal> {
    match v {
        Value::Number(n) => {
            let t = n.to_string();
            Decimal::from_str(&t)
                .or_else(|_| Decimal::from_scientific(&t))
                .map_err(|_| NICHT_ABGEBILDET)
        }
        Value::String(t) => Decimal::from_str(t.trim()).map_err(|_| "ValueError"),
        Value::Bool(_) | Value::Null => Err("ValueError"),
        _ => Err(NICHT_ABGEBILDET),
    }
}

// ---- je Accessor ---------------------------------------------------------------------------

pub fn raumkosten(args: &[Value]) -> R<RaumkostenEingabe> {
    let s = dict(args)?;
    Ok(RaumkostenEingabe {
        veranlagungszeitraum: vz_pfad(s)?,
        arbeitszimmer_vorhanden: get_truthy(s, "arbeitszimmer_vorhanden", false),
        ist_mittelpunkt: get_truthy(s, "ist_mittelpunkt", false),
        tatsaechliche_aufwendungen: euro(s, "tatsaechliche_aufwendungen")?,
        jahrespauschale_gewaehlt: get_truthy(s, "jahrespauschale_gewaehlt", false),
        monate_ohne_mittelpunkt: get_int(s, "monate_ohne_mittelpunkt", 0)?,
        homeoffice_tage: get_int(s, "homeoffice_tage", 0)?,
    })
}

/// Positionales Jahr: `params/<str(year)>/...`.
pub fn jahr(args: &[Value]) -> R<Vz> {
    let mut s = S::new();
    s.insert(
        "veranlagungszeitraum".into(),
        args.first().cloned().ok_or(NICHT_ABGEBILDET)?,
    );
    vz_pfad(&s)
}

pub fn entfernungspauschale(args: &[Value]) -> R<EntfernungspauschaleEingabe> {
    let s = dict(args)?;
    Ok(EntfernungspauschaleEingabe {
        veranlagungszeitraum: vz_pfad(s)?,
        entfernung_km_roh: Km::new(decimal_str(s.get("entfernung_km_roh").ok_or("KeyError")?)?),
        arbeitstage: req_int(s, "arbeitstage")?,
        eigenes_oder_ueberlassenes_kfz: get_truthy(s, "eigenes_oder_ueberlassenes_kfz", false),
        oepnv_kosten_jahr: euro(s, "oepnv_kosten_jahr")?,
    })
}

pub fn p101(args: &[Value]) -> R<P101Eingabe> {
    let s = dict(args)?;
    let ep = euro(s, "entfernungspauschale_ab_21km")?;
    let zve = euro(s, "zu_versteuerndes_einkommen")?;
    let gfb = euro(s, "grundfreibetrag")?;
    let an = get_truthy(s, "ist_arbeitnehmer", false);
    // Python liest die beiden nur im Arbeitnehmer-Zweig.
    let (wk, ap) = if an {
        (
            euro(s, "werbungskosten_gesamt")?,
            euro(s, "arbeitnehmer_pauschbetrag")?,
        )
    } else {
        (Euro::new(0), Euro::new(0))
    };
    Ok(P101Eingabe {
        entfernungspauschale_ab_21km: ep,
        zu_versteuerndes_einkommen: zve,
        grundfreibetrag: gfb,
        ist_arbeitnehmer: an,
        werbungskosten_gesamt: wk,
        arbeitnehmer_pauschbetrag: ap,
    })
}

fn dhf(s: &S) -> R<DhfEingabe> {
    let vz = vz_pfad(s)?;
    let im_inland = get_truthy(s, "im_inland", true);
    Ok(DhfEingabe {
        veranlagungszeitraum: vz,
        unterkunftskosten_monat: euro(s, "unterkunftskosten_monat")?,
        monate: get_int(s, "monate", 0)?,
        im_inland,
    })
}

fn verpflegung(s: &S) -> R<VerpflegungEingabe> {
    Ok(VerpflegungEingabe {
        veranlagungszeitraum: vz_pfad(s)?,
        tage_24h: get_int(s, "tage_24h", 0)?,
        tage_an_abreise: get_int(s, "tage_an_abreise", 0)?,
        tage_ueber_8h_eintaegig: get_int(s, "tage_ueber_8h_eintaegig", 0)?,
        vpf_tage_24h_nach_drei_monaten: get_int(s, "vpf_tage_24h_nach_drei_monaten", 0)?,
        vpf_tage_an_abreise_nach_drei_monaten: get_int(
            s,
            "vpf_tage_an_abreise_nach_drei_monaten",
            0,
        )?,
        vpf_tage_ueber_8h_nach_drei_monaten: get_int(s, "vpf_tage_ueber_8h_nach_drei_monaten", 0)?,
        vpf_fruehstuecke_gestellt_anzahl: get_int(s, "vpf_fruehstuecke_gestellt_anzahl", 0)?,
        vpf_mittagessen_gestellt_anzahl: get_int(s, "vpf_mittagessen_gestellt_anzahl", 0)?,
        vpf_abendessen_gestellt_anzahl: get_int(s, "vpf_abendessen_gestellt_anzahl", 0)?,
        vpf_mahlzeiten_gezahltes_entgelt: cent(s, "vpf_mahlzeiten_gezahltes_entgelt")?,
        vpf_steuerfreie_erstattung_betrag: cent(s, "vpf_steuerfreie_erstattung_betrag")?,
    })
}

fn uebernachtung(s: &S) -> R<UebernachtungEingabe> {
    // `s["veranlagungszeitraum"]` wird beim Aufruf gelesen (KeyError), der Pfad erst danach.
    s.get("veranlagungszeitraum").ok_or("KeyError")?;
    let kosten = euro(s, "uebernachtung_kosten_monat")?;
    let monate = get_int(s, "uebernachtung_monate", 0)?;
    let bisher = get_int(s, "uebernachtung_monate_bisher", 0)?;
    Ok(UebernachtungEingabe {
        veranlagungszeitraum: vz_pfad(s)?,
        uebernachtung_kosten_monat: kosten,
        uebernachtung_monate: monate,
        uebernachtung_monate_bisher: bisher,
        uebernachtung_im_inland: get_truthy(s, "uebernachtung_im_inland", true),
    })
}

pub fn werbungskosten_n(args: &[Value]) -> R<WerbungskostenNEingabe> {
    let s = dict(args)?;
    let hat = |k: &str| s.contains_key(k);
    Ok(WerbungskostenNEingabe {
        entfernung: if hat("entfernung_km_roh") {
            Some(entfernungspauschale(args)?)
        } else {
            None
        },
        doppelte_haushaltsfuehrung: if hat("unterkunftskosten_monat") {
            Some(dhf(s)?)
        } else {
            None
        },
        verpflegung: if hat("tage_24h") || hat("tage_an_abreise") || hat("tage_ueber_8h_eintaegig")
        {
            Some(verpflegung(s)?)
        } else {
            None
        },
        uebernachtung: if hat("uebernachtung_kosten_monat") {
            Some(uebernachtung(s)?)
        } else {
            None
        },
        am_anschaffungskosten: if hat("am_anschaffungskosten") {
            Some(euro(s, "am_anschaffungskosten")?)
        } else {
            None
        },
    })
}

pub fn vermietung_einkuenfte(args: &[Value]) -> R<VermietungEinkuenfteEingabe> {
    let s = dict(args)?;
    Ok(VermietungEinkuenfteEingabe {
        einnahmen: euro(s, "einnahmen")?,
        gebaeude_afa: euro(s, "gebaeude_afa")?,
        schuldzinsen: euro(s, "schuldzinsen")?,
        erhaltungsaufwand: euro(s, "erhaltungsaufwand")?,
        sonstige_werbungskosten: euro(s, "sonstige_werbungskosten")?,
    })
}

pub fn einkuenfte_nichtselbststaendig(args: &[Value]) -> R<EinkuenfteNichtselbststaendigEingabe> {
    let s = dict(args)?;
    Ok(EinkuenfteNichtselbststaendigEingabe {
        bruttoarbeitslohn: euro(s, "bruttoarbeitslohn")?,
        werbungskosten: euro(s, "werbungskosten")?,
        veranlagungszeitraum: vz_enum(s)?,
    })
}

pub fn p35a_haushaltsnahe(args: &[Value]) -> R<P35aHaushaltsnaheEingabe> {
    let s = dict(args)?;
    let minijob = euro(s, "hh_minijob_aufwendungen")?;
    let dienst = euro(s, "hh_dienstleistungen")?;
    let handwerker = euro(s, "hh_handwerker_arbeitskosten")?;
    let eu = wert_ist(s, "hh_in_eu_ewr", true)?;
    let rechnung = wert_ist(s, "hh_rechnung_unbar", true)?;
    let gefoerdert = wert_ist(s, "hh_handwerker_keine_foerderung", false)?;
    // Python liest p35a_mitveranlagung erst nach dem EU/EWR-Gate.
    let mitveranlagung = if eu {
        wert_ist(s, "p35a_mitveranlagung", true)?
    } else {
        false
    };
    Ok(P35aHaushaltsnaheEingabe {
        hh_minijob_aufwendungen: minijob,
        hh_dienstleistungen: dienst,
        hh_handwerker_arbeitskosten: handwerker,
        hh_in_eu_ewr: eu,
        hh_rechnung_unbar: rechnung,
        hh_handwerker_gefoerdert: gefoerdert,
        p35a_mitveranlagung: mitveranlagung,
    })
}

pub fn p3_nr72_photovoltaik(args: &[Value]) -> R<P3Nr72PhotovoltaikEingabe> {
    let s = dict(args)?;
    Ok(P3Nr72PhotovoltaikEingabe {
        pv_einnahmen: euro(s, "pv_einnahmen")?,
        pv_auf_gebaeude: wert_ist(s, "pv_auf_gebaeude", true)?,
        pv_bruttoleistung_kwp: get_int(s, "pv_bruttoleistung_kwp", 0)?,
        pv_anzahl_einheiten: get_int(s, "pv_anzahl_einheiten", 0)?,
    })
}

pub fn p10b_spenden(args: &[Value]) -> R<P10bSpendenEingabe> {
    let s = dict(args)?;
    Ok(P10bSpendenEingabe {
        zuwendungen: euro(s, "zuwendungen")?,
        gesamtbetrag_der_einkuenfte: euro(s, "gesamtbetrag_der_einkuenfte")?,
    })
}

fn zumutbar(s: &S) -> R<P33ZumutbarEingabe> {
    Ok(P33ZumutbarEingabe {
        gesamtbetrag_der_einkuenfte: euro(s, "gesamtbetrag_der_einkuenfte")?,
        anzahl_kinder: get_int(s, "anzahl_kinder", 0)?,
        splitting: get_truthy(s, "splitting", false),
    })
}

pub fn p33_zumutbar(args: &[Value]) -> R<P33ZumutbarEingabe> {
    zumutbar(dict(args)?)
}

pub fn p33_agb(args: &[Value]) -> R<P33AgbEingabe> {
    let s = dict(args)?;
    Ok(P33AgbEingabe {
        aussergewoehnliche_belastungen: euro(s, "aussergewoehnliche_belastungen")?,
        zumutbar: zumutbar(s)?,
    })
}

pub fn p10_kist(args: &[Value]) -> R<P10KistEingabe> {
    let s = dict(args)?;
    Ok(P10KistEingabe {
        gezahlte_kirchensteuer: euro(s, "gezahlte_kirchensteuer")?,
        erstattete_kirchensteuer: euro(s, "erstattete_kirchensteuer")?,
    })
}

pub fn kist(args: &[Value]) -> R<KistEingabe> {
    let s = dict(args)?;
    let konfession = s
        .get("konfession")
        .map_or(Ok("keine".to_string()), py_str)?;
    let bundesland = s.get("bundesland").map_or(Ok(String::new()), py_str)?;
    Ok(KistEingabe {
        konfession,
        bundesland,
        est_mit_fb: euro(s, "est_mit_fb")?,
    })
}

pub fn p36_abschlusszahlung(args: &[Value]) -> R<P36AbschlusszahlungEingabe> {
    let s = dict(args)?;
    Ok(P36AbschlusszahlungEingabe {
        lohnsteuer_cent: cent(s, "lohnsteuer_cent")?,
        kapitalertragsteuer_cent: cent(s, "kapitalertragsteuer_cent")?,
        kapitalertragsteuer_solz_cent: cent(s, "kapitalertragsteuer_solz_cent")?,
        kapitalertragsteuer_kist_cent: cent(s, "kapitalertragsteuer_kist_cent")?,
        festzusetzende_est_cent: cent(s, "festzusetzende_est_cent")?,
        vorauszahlungen_cent: cent(s, "vorauszahlungen_cent")?,
    })
}

pub fn p24a_altersentlastung(args: &[Value]) -> R<P24aAltersentlastungEingabe> {
    let s = dict(args)?;
    Ok(P24aAltersentlastungEingabe {
        veranlagungszeitraum: get_int(s, "veranlagungszeitraum", 0)?,
        geburtsjahr: get_int(s, "geburtsjahr", 0)?,
        arbeitslohn: euro(s, "arbeitslohn")?,
        positive_andere_einkuenfte: euro(s, "positive_andere_einkuenfte")?,
    })
}

pub fn p24b_entlastung(args: &[Value]) -> R<P24bEntlastungEingabe> {
    let s = dict(args)?;
    Ok(P24bEntlastungEingabe {
        alleinstehend: get_truthy(s, "alleinstehend", false),
        anzahl_kinder: get_int(s, "anzahl_kinder", 0)?,
        monate_ohne_voraussetzung: get_int(s, "monate_ohne_voraussetzung", 0)?,
    })
}

pub fn p31_familienleistung(args: &[Value]) -> R<P31FamilienleistungEingabe> {
    let s = dict(args)?;
    Ok(P31FamilienleistungEingabe {
        est_ohne_freibetraege: euro(s, "est_ohne_freibetraege")?,
        est_mit_freibetraegen: euro(s, "est_mit_freibetraegen")?,
        kindergeld: euro(s, "kindergeld")?,
    })
}

pub fn p21_2_verbilligt(args: &[Value]) -> R<P212VerbilligtEingabe> {
    let s = dict(args)?;
    Ok(P212VerbilligtEingabe {
        werbungskosten: euro(s, "werbungskosten")?,
        entgelt_quote_prozent: get_int(s, "entgelt_quote_prozent", 100)?,
    })
}

pub fn p10_kv_pv(args: &[Value]) -> R<P10KvPvEingabe> {
    let s = dict(args)?;
    Ok(P10KvPvEingabe {
        basis_kv_pv: euro(s, "basis_kv_pv")?,
        weitere_vorsorgeaufwendungen: euro(s, "weitere_vorsorgeaufwendungen")?,
        mit_anspruch_auf_zuschuss: get_truthy(s, "mit_anspruch_auf_zuschuss", false),
    })
}

pub fn p10_1_7_berufsausbildung(args: &[Value]) -> R<P1017BerufsausbildungEingabe> {
    Ok(P1017BerufsausbildungEingabe {
        berufsausbildung_aufwendungen: euro(dict(args)?, "berufsausbildung_aufwendungen")?,
    })
}

pub fn p16_4_freibetrag(args: &[Value]) -> R<P164FreibetragEingabe> {
    Ok(P164FreibetragEingabe {
        rentner_veraeusserungsgewinn: euro(dict(args)?, "rentner_veraeusserungsgewinn")?,
    })
}

pub fn euer_gewinn(args: &[Value]) -> R<EuerGewinnEingabe> {
    let s = dict(args)?;
    Ok(EuerGewinnEingabe {
        betriebseinnahmen: euro(s, "betriebseinnahmen")?,
        betriebsausgaben: euro(s, "betriebsausgaben")?,
    })
}

pub fn mitunternehmer_einkuenfte(args: &[Value]) -> R<MitunternehmerEinkuenfteEingabe> {
    let s = dict(args)?;
    Ok(MitunternehmerEinkuenfteEingabe {
        gewinnanteil: euro(s, "gewinnanteil")?,
        verguetung_taetigkeit: euro(s, "verguetung_taetigkeit")?,
        verguetung_darlehen: euro(s, "verguetung_darlehen")?,
        verguetung_ueberlassung: euro(s, "verguetung_ueberlassung")?,
    })
}

pub fn p6_2_gwg(args: &[Value]) -> R<P62GwgEingabe> {
    Ok(P62GwgEingabe {
        gwg_anschaffungskosten_netto: euro(dict(args)?, "gwg_anschaffungskosten_netto")?,
    })
}

pub fn p7_linear_afa(args: &[Value]) -> R<P7LinearAfaEingabe> {
    let s = dict(args)?;
    let ak_cent = cent(s, "anschaffungskosten_cent")?;
    // Python liest `anschaffungskosten` nur, wenn `anschaffungskosten_cent <= 0`.
    let ak = if ak_cent.get() > 0 {
        Euro::new(0)
    } else {
        euro(s, "anschaffungskosten")?
    };
    Ok(P7LinearAfaEingabe {
        anschaffungskosten_cent: ak_cent,
        anschaffungskosten: ak,
        nutzungsdauer: get_int(s, "nutzungsdauer", 0)?,
        anschaffung_monat: get_int(s, "anschaffung_monat", 0)?,
        ist_anschaffungsjahr: get_truthy(s, "ist_anschaffungsjahr", false),
    })
}
