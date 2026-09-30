//! Einkunftsgroessen aus runner.py: § 21 V+V, § 19, § 3 Nr. 72, § 21 Abs. 2, § 16 Abs. 4,
//! § 4 Abs. 3, § 15 Abs. 1 S. 1 Nr. 2. Alle EURO rein, EURO raus.
use domain::{Cent, Euro, Vz};
use rust_decimal::Decimal;

use super::fehler::{in_cent, ok, EngineFehler};
use crate::betriebsfreibetrag::{self, BetriebsFreibetragEingabe};
use crate::euer::{self, EuerEingabe};
use crate::mitunternehmer::{self, MitunternehmerEingabe};
use crate::tarif::{self, FestzusetzendeEstEinzelEingabe};
use crate::verbilligte_vermietung::{self, VerbilligteVermietungEingabe};

/// Eingabe fuer [`vermietung_einkuenfte`].
#[derive(Debug, Clone, Copy)]
pub struct VermietungEinkuenfteEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Einnahmen 0)
    pub einnahmen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub gebaeude_afa: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub schuldzinsen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub erhaltungsaufwand: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub sonstige_werbungskosten: Euro,
}

/// `catala_vermietung_einkuenfte` -- § 21 Abs. 1 `EStG`: Einnahmen minus Werbungskosten
/// (Gebaeude-AfA, Schuldzinsen, Erhaltungsaufwand, sonstige), EURO. Negativ moeglich.
/// Transkription des Registry-Rechenwegs `p21_vermietung_einkuenfte`.
///
/// # Errors
/// [`EngineFehler::Ueberlauf`] bei `i64`-Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::einkuenfte::{vermietung_einkuenfte, VermietungEinkuenfteEingabe};
/// let e = VermietungEinkuenfteEingabe {
///     einnahmen: Euro::new(1000), gebaeude_afa: Euro::new(300), schuldzinsen: Euro::new(900),
///     erhaltungsaufwand: Euro::new(0), sonstige_werbungskosten: Euro::new(0),
/// };
/// assert_eq!(vermietung_einkuenfte(&e).unwrap(), Euro::new(-200));
/// ```
pub fn vermietung_einkuenfte(e: &VermietungEinkuenfteEingabe) -> Result<Euro, EngineFehler> {
    let wk = e
        .gebaeude_afa
        .get()
        .checked_add(e.schuldzinsen.get())
        .and_then(|x| x.checked_add(e.erhaltungsaufwand.get()))
        .and_then(|x| x.checked_add(e.sonstige_werbungskosten.get()));
    ok(wk.and_then(|w| e.einnahmen.get().checked_sub(w)), "vuv").map(Euro::new)
}

/// Eingabe fuer [`einkuenfte_nichtselbststaendig`].
#[derive(Debug, Clone, Copy)]
pub struct EinkuenfteNichtselbststaendigEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub bruttoarbeitslohn: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Pauschbetrag greift)
    pub werbungskosten: Euro,
    /// `s["veranlagungszeitraum"]` (Pflicht).
    pub veranlagungszeitraum: Vz,
}

/// `catala_einkuenfte_nichtselbststaendig` -- § 19 i.V.m. § 9a S. 1 Nr. 1 `EStG`: Bruttolohn
/// minus Werbungskosten, mindestens Arbeitnehmer-Pauschbetrag (`summe_der_einkuenfte` des
/// Einzel-Tarifs, § 9a-Guenstiger im Scope), EURO. Sonderausgaben 0: der § 10c-Floor gilt einmal
/// je Person im Gesamt-Scope.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::{Euro, Vz};
/// use engine::zugriff::teil1::einkuenfte::{einkuenfte_nichtselbststaendig, EinkuenfteNichtselbststaendigEingabe};
/// let e = EinkuenfteNichtselbststaendigEingabe {
///     bruttoarbeitslohn: Euro::new(40_000), werbungskosten: Euro::new(0), veranlagungszeitraum: Vz::Vz2025,
/// };
/// assert_eq!(einkuenfte_nichtselbststaendig(&e).unwrap(), Euro::new(38_770));
/// ```
pub fn einkuenfte_nichtselbststaendig(
    e: &EinkuenfteNichtselbststaendigEingabe,
) -> Result<Euro, EngineFehler> {
    let out = tarif::festzusetzende_est_einzel_voll(
        FestzusetzendeEstEinzelEingabe {
            bruttoarbeitslohn: in_cent(e.bruttoarbeitslohn)?,
            werbungskosten: in_cent(e.werbungskosten)?,
            sonderausgaben: Cent::new(0),
        },
        e.veranlagungszeitraum,
    )?;
    Ok(Cent::new(out.summe_der_einkuenfte_cent).floor_euro())
}

/// Eingabe fuer [`p3_nr72_photovoltaik`].
#[derive(Debug, Clone, Copy)]
pub struct P3Nr72PhotovoltaikEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: keine Befreiung)
    pub pv_einnahmen: Euro,
    /// `pv_auf_gebaeude["wert"] is True`. PARITÄT: Python setzt fehlend = {} -> False (fail-closed)
    pub pv_auf_gebaeude: bool,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: keine Befreiung)
    pub pv_bruttoleistung_kwp: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: keine Befreiung)
    pub pv_anzahl_einheiten: i64,
}

/// Leistungsgrenze je Wohn- oder Gewerbeeinheit, kWp (§ 3 Nr. 72 S. 1).
const PV_KWP_JE_EINHEIT: i64 = 30;
/// Leistungsgrenze insgesamt je Steuerpflichtigem, kWp (§ 3 Nr. 72 S. 1).
const PV_KWP_GESAMT: i64 = 100;

/// `catala_p3_nr72_photovoltaik` -- § 3 Nr. 72 `EStG`: steuerfreie Einnahmen aus
/// Gebaeude-Photovoltaik, EURO (der von den Gewinneinkuenften abzuziehende Betrag).
///
/// Zwei kumulative, einschliessende Grenzen: 30 kWp je Einheit und hoechstens 100 kWp insgesamt.
/// Freigrenze, kein Freibetrag: wird eine Grenze gerissen, sind die Einnahmen VOLL steuerpflichtig.
/// Nur Anlagen auf, an oder in Gebaeuden; ohne Leistungsangabe keine Befreiung.
///
/// # Errors
/// Keine; `Result` fuer die einheitliche Accessor-Signatur.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::einkuenfte::{p3_nr72_photovoltaik, P3Nr72PhotovoltaikEingabe};
/// let e = P3Nr72PhotovoltaikEingabe {
///     pv_einnahmen: Euro::new(900), pv_auf_gebaeude: true, pv_bruttoleistung_kwp: 30, pv_anzahl_einheiten: 1,
/// };
/// assert_eq!(p3_nr72_photovoltaik(&e).unwrap(), Euro::new(900));
/// ```
pub fn p3_nr72_photovoltaik(e: &P3Nr72PhotovoltaikEingabe) -> Result<Euro, EngineFehler> {
    let null = Euro::new(0);
    if e.pv_einnahmen.get() <= 0 || !e.pv_auf_gebaeude {
        return Ok(null);
    }
    let (leistung, einheiten) = (e.pv_bruttoleistung_kwp, e.pv_anzahl_einheiten);
    if leistung <= 0 || einheiten <= 0 {
        return Ok(null);
    }
    let grenze_einheiten = ok(einheiten.checked_mul(PV_KWP_JE_EINHEIT), "pv")?;
    if leistung > grenze_einheiten || leistung > PV_KWP_GESAMT {
        return Ok(null);
    }
    Ok(e.pv_einnahmen)
}

/// Eingabe fuer [`p21_2_verbilligt`].
#[derive(Debug, Clone, Copy)]
pub struct P212VerbilligtEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub werbungskosten: Euro,
    /// Prozent der ortsueblichen Marktmiete, ganzzahlig. PARITÄT: Python setzt fehlend = 100 (fail-open: volle WK)
    pub entgelt_quote_prozent: i64,
}

/// `catala_p21_2_verbilligt` -- § 21 Abs. 2 `EStG`: abziehbare Werbungskosten bei verbilligter
/// Wohnraumvermietung, EURO. Quote >= 66 % volle WK, darunter anteilig (quote/100). Der
/// 50-66-%-Korridor wird konservativ anteilig gekuerzt (ueber-, nie unterbesteuert).
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::einkuenfte::{p21_2_verbilligt, P212VerbilligtEingabe};
/// let e = P212VerbilligtEingabe { werbungskosten: Euro::new(1000), entgelt_quote_prozent: 50 };
/// assert_eq!(p21_2_verbilligt(&e).unwrap(), Euro::new(500));
/// ```
pub fn p21_2_verbilligt(e: &P212VerbilligtEingabe) -> Result<Euro, EngineFehler> {
    let c = verbilligte_vermietung::berechnen(VerbilligteVermietungEingabe {
        werbungskosten: in_cent(e.werbungskosten)?,
        entgelt_quote_prozent: Decimal::from(e.entgelt_quote_prozent),
    })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`p16_4_freibetrag`].
#[derive(Debug, Clone, Copy)]
pub struct P164FreibetragEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: Freibetrag 0)
    pub rentner_veraeusserungsgewinn: Euro,
}

/// `catala_p16_4_freibetrag` -- § 16 Abs. 4 `EStG`: ROHER Freibetrag fuer den
/// Veraeusserungsgewinn (45.000 minus Ueberschreitung von 136.000; 0 bei Gewinn <= 0), EURO.
/// Der Aufrufer floort den steuerbaren Rest; das Alters-/Einmal-Gate sitzt in der Haut.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::einkuenfte::{p16_4_freibetrag, P164FreibetragEingabe};
/// let e = P164FreibetragEingabe { rentner_veraeusserungsgewinn: Euro::new(140_000) };
/// assert_eq!(p16_4_freibetrag(&e).unwrap(), Euro::new(41_000));
/// ```
pub fn p16_4_freibetrag(e: &P164FreibetragEingabe) -> Result<Euro, EngineFehler> {
    let c = betriebsfreibetrag::berechnen(BetriebsFreibetragEingabe {
        veraeusserungsgewinn: in_cent(e.rentner_veraeusserungsgewinn)?,
    })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`euer_gewinn`].
#[derive(Debug, Clone, Copy)]
pub struct EuerGewinnEingabe {
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub betriebseinnahmen: Euro,
    /// Aggregat (sonstige + `AfA` + GWG). PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub betriebsausgaben: Euro,
}

/// `catala_euer_gewinn` -- § 4 Abs. 3 `EStG`: Betriebseinnahmen minus Betriebsausgaben, EURO.
/// Kann negativ sein (Verlustjahr).
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::einkuenfte::{euer_gewinn, EuerGewinnEingabe};
/// let e = EuerGewinnEingabe { betriebseinnahmen: Euro::new(100), betriebsausgaben: Euro::new(250) };
/// assert_eq!(euer_gewinn(&e).unwrap(), Euro::new(-150));
/// ```
pub fn euer_gewinn(e: &EuerGewinnEingabe) -> Result<Euro, EngineFehler> {
    let c = euer::berechnen(EuerEingabe {
        betriebseinnahmen: in_cent(e.betriebseinnahmen)?,
        betriebsausgaben: in_cent(e.betriebsausgaben)?,
    })?;
    Ok(c.floor_euro())
}

/// Eingabe fuer [`mitunternehmer_einkuenfte`].
#[derive(Debug, Clone, Copy)]
pub struct MitunternehmerEinkuenfteEingabe {
    /// § 15a-ausgleichsfaehiger Anteil, kann negativ sein. PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub gewinnanteil: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub verguetung_taetigkeit: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub verguetung_darlehen: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed)
    pub verguetung_ueberlassung: Euro,
}

/// `catala_mitunternehmer_einkuenfte` -- § 15 Abs. 1 S. 1 Nr. 2 `EStG`: Gewinnanteil plus drei
/// Sonderverguetungen, EURO.
///
/// # Errors
/// [`EngineFehler`] bei Catala-Laufzeitfehler oder Ueberlauf.
///
/// ```
/// use domain::Euro;
/// use engine::zugriff::teil1::einkuenfte::{mitunternehmer_einkuenfte, MitunternehmerEinkuenfteEingabe};
/// let e = MitunternehmerEinkuenfteEingabe {
///     gewinnanteil: Euro::new(-500), verguetung_taetigkeit: Euro::new(1000),
///     verguetung_darlehen: Euro::new(0), verguetung_ueberlassung: Euro::new(0),
/// };
/// assert_eq!(mitunternehmer_einkuenfte(&e).unwrap(), Euro::new(500));
/// ```
pub fn mitunternehmer_einkuenfte(
    e: &MitunternehmerEinkuenfteEingabe,
) -> Result<Euro, EngineFehler> {
    let c = mitunternehmer::berechnen(MitunternehmerEingabe {
        gewinnanteil: in_cent(e.gewinnanteil)?,
        verguetung_taetigkeit: in_cent(e.verguetung_taetigkeit)?,
        verguetung_darlehen: in_cent(e.verguetung_darlehen)?,
        verguetung_ueberlassung: in_cent(e.verguetung_ueberlassung)?,
    })?;
    Ok(c.floor_euro())
}
