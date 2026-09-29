//! Steuerermaessigungen, Entlastungsbetraege und Festsetzungsgroessen aus runner.py: § 35a,
//! § 51a (Kirchensteuer), § 36, § 24a, § 24b, § 31.
use bindung::Params;
use domain::{Cent, Euro};

use super::fehler::{in_cent, ok, EngineFehler};
use crate::altersentlastungsbetrag::{self, AltersentlastungsbetragEingabe};
use crate::entlastungsbetrag::{self, EntlastungsbetragEingabe};
use crate::familienleistungsausgleich::{self, FamilienleistungsausgleichEingabe};

/// Eingabe fuer [`p35a_haushaltsnahe`]. Die Flags stammen aus Feldzustaenden
/// (`s[k]["wert"]`); massgeblich ist genau der Wahrheitswert `True` bzw. `False`.
// Vier unabhaengige Feldzustaende aus dem Store, kein verkappter Zustandsautomat.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy)]
pub struct P35aHaushaltsnaheEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub hh_minijob_aufwendungen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub hh_dienstleistungen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub hh_handwerker_arbeitskosten: Euro,
    /// `hh_in_eu_ewr["wert"] is True`. PARITÄT: Python setzt fehlend = {} -> False (fail-closed)
    pub hh_in_eu_ewr: bool,
    /// `hh_rechnung_unbar["wert"] is True`. PARITÄT: Python setzt fehlend = {} -> False (fail-closed)
    pub hh_rechnung_unbar: bool,
    /// `hh_handwerker_keine_foerderung["wert"] is False`. PARITÄT: Python setzt fehlend = {} -> nicht gefoerdert (fail-open)
    pub hh_handwerker_gefoerdert: bool,
    /// `p35a_mitveranlagung["wert"] is True`. PARITÄT: Python setzt fehlend = {} -> keine Halbierung (fail-open)
    pub p35a_mitveranlagung: bool,
}

/// § 35a Abs. 1: 20 %, hoechstens 510 EUR (Minijob).
const HOECHST_MINIJOB: i64 = 510;
/// § 35a Abs. 2: 20 %, hoechstens 4.000 EUR (Dienstleistungen).
const HOECHST_DIENSTLEISTUNGEN: i64 = 4000;
/// § 35a Abs. 3: 20 %, hoechstens 1.200 EUR (Handwerker).
const HOECHST_HANDWERKER: i64 = 1200;

/// 20 % von `betrag`, abgerundet, gedeckelt; 0 fuer `betrag <= 0`.
fn zwanzig_prozent(betrag: i64, hoechst: i64) -> Result<i64, EngineFehler> {
    if betrag <= 0 {
        return Ok(0);
    }
    Ok(ok(betrag.checked_mul(20).and_then(|x| x.checked_div_euclid(100)), "p35a")?.min(hoechst))
}

/// `catala_p35a_haushaltsnahe` -- § 35a Abs. 1-5 `EStG`, EURO. EU/EWR (Abs. 4) gatet alles;
/// Rechnung + unbare Zahlung (Abs. 5 S. 3) gatet nur Abs. 2/3; oeffentlich gefoerderte
/// Massnahmen nullen nur Abs. 3 (S. 2). Bei Mitveranlagung halbiert.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::ermaessigungen::{p35a_haushaltsnahe, P35aHaushaltsnaheEingabe};
/// let e = P35aHaushaltsnaheEingabe {
///     hh_minijob_aufwendungen: Euro::new(500), hh_dienstleistungen: Euro::new(5000),
///     hh_handwerker_arbeitskosten: Euro::new(3000), hh_in_eu_ewr: true, hh_rechnung_unbar: true,
///     hh_handwerker_gefoerdert: false, p35a_mitveranlagung: false,
/// };
/// assert_eq!(p35a_haushaltsnahe(&e).unwrap(), Euro::new(1700));
/// ```
pub fn p35a_haushaltsnahe(e: &P35aHaushaltsnaheEingabe) -> Result<Euro, EngineFehler> {
    if !e.hh_in_eu_ewr {
        return Ok(Euro::new(0));
    }
    let (mut dienstleistungen, mut handwerker) = (e.hh_dienstleistungen.get(), e.hh_handwerker_arbeitskosten.get());
    if !e.hh_rechnung_unbar {
        dienstleistungen = 0;
        handwerker = 0;
    }
    if handwerker > 0 && e.hh_handwerker_gefoerdert {
        handwerker = 0;
    }
    let summe = zwanzig_prozent(e.hh_minijob_aufwendungen.get(), HOECHST_MINIJOB)?
        + zwanzig_prozent(dienstleistungen, HOECHST_DIENSTLEISTUNGEN)?
        + zwanzig_prozent(handwerker, HOECHST_HANDWERKER)?;
    Ok(Euro::new(if e.p35a_mitveranlagung { summe.div_euclid(2) } else { summe }))
}

/// Eingabe fuer [`kist`]. Konfession und Bundesland sind die Store-Werte, wie Python sie mit
/// `str(...)` liest.
#[derive(Debug, Clone)]
pub struct KistEingabe {
    /// PARITÄT: Python setzt fehlend = "keine" (Kirchensteuer 0)
    pub konfession: String,
    /// PARITÄT: Python setzt fehlend = "" (9 %)
    pub bundesland: String,
    /// § 51a-Bemessungsgrundlage, EURO. PARITÄT: Python setzt fehlend = 0 (fail-open: Kirchensteuer 0)
    pub est_mit_fb: Euro,
}

/// Laender mit 8 % Hebesatz (Landes-KiStG; Art. 8 `BayKirchStG`, `FinMin`-NRW-Erlass 18.04.2024).
const KIST_8_PROZENT: [&str; 2] = ["bayern", "baden_wuerttemberg"];
/// Steuererhebende Konfessionen.
const KIST_KONFESSION: [&str; 2] = ["evangelisch", "roemisch-katholisch"];

/// `catala_kist` -- § 51a `EStG` i.V.m. Landes-KiStG: Kirchensteuer auf die Massstabsteuer, CENT.
/// 8 % in Bayern und Baden-Wuerttemberg, sonst 9 %; 0 ohne steuererhebende Konfession. Euro x
/// ganzzahliger Prozentsatz = Cent, exakt.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use domain::{Cent, Euro};
/// use engine::zugriff::teil1::ermaessigungen::{kist, KistEingabe};
/// let e = KistEingabe { konfession: "evangelisch".into(), bundesland: "bayern".into(), est_mit_fb: Euro::new(1000) };
/// assert_eq!(kist(&e).unwrap(), Cent::new(8000));
/// ```
pub fn kist(e: &KistEingabe) -> Result<Cent, EngineFehler> {
    if !KIST_KONFESSION.contains(&e.konfession.as_str()) {
        return Ok(Cent::new(0));
    }
    let satz = if KIST_8_PROZENT.contains(&e.bundesland.as_str()) { 8 } else { 9 };
    ok(e.est_mit_fb.get().checked_mul(satz), "kist").map(Cent::new)
}

/// Eingabe fuer [`p36_abschlusszahlung`]. Alle Betraege CENT.
#[derive(Debug, Clone, Copy)]
pub struct P36AbschlusszahlungEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: Erstattung)
    pub festzusetzende_est_cent: Cent,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub lohnsteuer_cent: Cent,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub kapitalertragsteuer_cent: Cent,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub kapitalertragsteuer_solz_cent: Cent,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub kapitalertragsteuer_kist_cent: Cent,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub vorauszahlungen_cent: Cent,
}

/// Aufrunden auf volle Euro, in Cent (§ 36 Abs. 3 S. 1+2).
fn aufgerundet(c: Cent) -> Result<i64, EngineFehler> {
    Ok(in_cent(c.ceil_euro())?.get())
}

/// `catala_p36_abschlusszahlung` -- § 36 Abs. 2+4 `EStG`: Abschlusszahlung (+) oder Erstattung
/// (-), CENT. Festzusetzende `ESt` minus die anrechenbaren Abzugsteuern (`LSt`, `KapESt`, `SolZ` und
/// `KiSt` auf `KapESt`), JEDE fuer sich auf volle Euro aufgerundet (Abs. 3 S. 2), minus
/// Vorauszahlungen.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use domain::Cent;
/// use engine::zugriff::teil1::ermaessigungen::{p36_abschlusszahlung, P36AbschlusszahlungEingabe};
/// let e = P36AbschlusszahlungEingabe {
///     festzusetzende_est_cent: Cent::new(100_000), lohnsteuer_cent: Cent::new(50_001),
///     kapitalertragsteuer_cent: Cent::new(0), kapitalertragsteuer_solz_cent: Cent::new(0),
///     kapitalertragsteuer_kist_cent: Cent::new(0), vorauszahlungen_cent: Cent::new(0),
/// };
/// assert_eq!(p36_abschlusszahlung(&e).unwrap(), Cent::new(49_900));
/// ```
pub fn p36_abschlusszahlung(e: &P36AbschlusszahlungEingabe) -> Result<Cent, EngineFehler> {
    let abzug = [
        aufgerundet(e.lohnsteuer_cent)?,
        aufgerundet(e.kapitalertragsteuer_cent)?,
        aufgerundet(e.kapitalertragsteuer_solz_cent)?,
        aufgerundet(e.kapitalertragsteuer_kist_cent)?,
        e.vorauszahlungen_cent.get(),
    ];
    let mut rest = e.festzusetzende_est_cent.get();
    for a in abzug {
        rest = ok(rest.checked_sub(a), "p36")?;
    }
    Ok(Cent::new(rest))
}

/// Eingabe fuer [`p24a_altersentlastung`].
#[derive(Debug, Clone, Copy)]
pub struct P24aAltersentlastungEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: das 64+-Gate nach § 24a S. 3 entfaellt)
    pub veranlagungszeitraum: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Betrag 0)
    pub geburtsjahr: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub arbeitslohn: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub positive_andere_einkuenfte: Euro,
}

/// `catala_p24a_altersentlastung` -- § 24a `EStG` Altersentlastungsbetrag, EURO.
///
/// min(Prozentsatz x (Arbeitslohn + positive uebrige Einkuenfte); Hoechstbetrag), Kohorte nach dem
/// massgebenden Folgejahr = Geburtsjahr + 65, lebenslang fix. Geburtsjahr <= 0 -> 0 (kein
/// Phantom-Abzug). S. 3: erst ab Folgejahr <= VZ (Geburtsjahr-Naeherung, zu Lasten des
/// Steuerpflichtigen); VZ 0 -> Gate inaktiv.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler, fehlender Kohortentabelle oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::ermaessigungen::{p24a_altersentlastung, P24aAltersentlastungEingabe};
/// let p = bindung::Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap();
/// let e = P24aAltersentlastungEingabe {
///     veranlagungszeitraum: 2025, geburtsjahr: 1960, arbeitslohn: Euro::new(10_000),
///     positive_andere_einkuenfte: Euro::new(0),
/// };
/// assert_eq!(p24a_altersentlastung(&e, &p).unwrap(), Euro::new(627));
/// ```
pub fn p24a_altersentlastung(e: &P24aAltersentlastungEingabe, p: &Params) -> Result<Euro, EngineFehler> {
    let vz = e.veranlagungszeitraum;
    let folgejahr = ok(e.geburtsjahr.checked_add(65), "p24a folgejahr")?;
    if e.geburtsjahr <= 0 || (vz > 0 && folgejahr > vz) {
        return Ok(Euro::new(0));
    }
    let k = p.altersentlastung_kohorte(folgejahr)?;
    let c = altersentlastungsbetrag::berechnen(AltersentlastungsbetragEingabe {
        arbeitslohn: in_cent(e.arbeitslohn)?,
        positive_andere_einkuenfte: in_cent(e.positive_andere_einkuenfte)?,
        prozentsatz: k.prozentsatz,
        hoechstbetrag: in_cent(k.hoechstbetrag)?,
    })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`p24b_entlastung`].
#[derive(Debug, Clone, Copy)]
pub struct P24bEntlastungEingabe {
    /// PARITÄT: Python setzt fehlend = False (fail-closed: Betrag 0)
    pub alleinstehend: bool,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Betrag 0)
    pub anzahl_kinder: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine anteilige Kuerzung)
    pub monate_ohne_voraussetzung: i64,
}

/// `catala_p24b_entlastung` -- § 24b `EStG` Entlastungsbetrag fuer Alleinerziehende, EURO:
/// Grundbetrag plus Erhoehung je weiterem Kind, anteilig gekuerzt.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler.
///
/// ```
/// use engine::zugriff::teil1::ermaessigungen::{p24b_entlastung, P24bEntlastungEingabe};
/// let e = P24bEntlastungEingabe { alleinstehend: false, anzahl_kinder: 1, monate_ohne_voraussetzung: 0 };
/// assert_eq!(p24b_entlastung(&e).unwrap(), domain::Euro::new(0));
/// ```
pub fn p24b_entlastung(e: &P24bEntlastungEingabe) -> Result<Euro, EngineFehler> {
    let c = entlastungsbetrag::berechnen(EntlastungsbetragEingabe {
        alleinstehend: e.alleinstehend,
        anzahl_kinder: e.anzahl_kinder,
        monate_ohne_voraussetzung: e.monate_ohne_voraussetzung,
    })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`p31_familienleistung`]. Alle EURO.
#[derive(Debug, Clone, Copy)]
pub struct P31FamilienleistungEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub est_ohne_freibetraege: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub est_mit_freibetraegen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Hinzurechnung)
    pub kindergeld: Euro,
}

/// `catala_p31_familienleistung` -- § 31 `EStG` Guenstigerpruefung, EURO: Freibetrag guenstiger
/// -> `ESt` mit Freibetraegen plus Kindergeld (S. 4), sonst `ESt` ohne Freibetraege.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::ermaessigungen::{p31_familienleistung, P31FamilienleistungEingabe};
/// let e = P31FamilienleistungEingabe {
///     est_ohne_freibetraege: Euro::new(10_000), est_mit_freibetraegen: Euro::new(9_000), kindergeld: Euro::new(3_000),
/// };
/// assert_eq!(p31_familienleistung(&e).unwrap(), Euro::new(10_000));
/// ```
pub fn p31_familienleistung(e: &P31FamilienleistungEingabe) -> Result<Euro, EngineFehler> {
    let c = familienleistungsausgleich::berechnen(FamilienleistungsausgleichEingabe {
        est_ohne_freibetraege: in_cent(e.est_ohne_freibetraege)?,
        est_mit_freibetraegen: in_cent(e.est_mit_freibetraegen)?,
        kindergeld: in_cent(e.kindergeld)?,
    })?;
    Ok(c.floor_euro())
}
