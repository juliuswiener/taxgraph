//! Einkuenfte: Gewinn (§§ 13-18), Kapital (§ 20), private Veraeusserungen (§ 23), auslaendische
//! Einkuenfte (DBA) und die Gewerbesteuer-Anrechnung (§ 35).
//!
//! Quelle: `produkt/bescheid/bescheid_einkuenfte.py` plus die dort importierten Konstanten und
//! `dba_methode_fuer` aus `produkt/haut/api_constants.py` (im Rust-Baum bisher nicht vorhanden).
use bindung::Params;
use domain::{Euro, PyWert, Veranlagung, Vz};
use engine::zugriff::teil1::afa::{p6_2_gwg, P62GwgEingabe};
use engine::zugriff::teil1::einkuenfte::{
    euer_gewinn, mitunternehmer_einkuenfte, p16_4_freibetrag, p3_nr72_photovoltaik,
    EuerGewinnEingabe, MitunternehmerEinkuenfteEingabe, P164FreibetragEingabe,
    P3Nr72PhotovoltaikEingabe,
};
use engine::zugriff::teil2::gesamt::{
    gesamt_tarifliche, gesamt_zve, p10d_2, GesamtfallEingabe, VerlustabzugEingabe,
};
use engine::zugriff::teil2::kapital::{
    kapital_verrechnung, sparer_pb, KapitalVerrechnungEingabe, SparerPbEingabe,
};
use engine::zugriff::teil2::p23::{
    p23_freigrenze, p23_veraeusserungsgewinn, p23_verlusttopf, VeraeusserungsgewinnEingabe,
    VerlusttopfEingabe,
};
use engine::zugriff::teil2::p33::{p33a_ausbildungsfreibetrag, p33a_unterhalt, UnterhaltEingabe};
use engine::zugriff::teil2::sonstige::{p34c_1, AuslaendischeSteuerEingabe};

use crate::{
    cent_zu_euro, euro_plus, feld_euro_oder_null, feld_int_oder_null, ist_false, ist_true,
    ist_zusammen, minus, plus, py_leerraum, summe, wert, zahl_oder_null, BescheidFehler, Felder,
    Instanzquelle,
};

// ---------------------------------------------------------------- Konstanten (api_constants.py)

/// `KAP_ERTRAEGE` (`api_constants.py:300`).
pub const KAP_ERTRAEGE: &str = "kap_kapitalertraege";
/// `KAP_TOEPFE` (`api_constants.py:301`).
pub const KAP_TOEPFE: [&str; 4] = [
    "kap_gewinn_aktien",
    "kap_verlust_aktien",
    "kap_gewinn_sonstige",
    "kap_verlust_sonstige",
];
/// `KAP_ERTRAEGE_PARTNER`.
pub const KAP_ERTRAEGE_PARTNER: &str = "kap_kapitalertraege_partner";
/// `KAP_TOEPFE_PARTNER` (Reihenfolge wie Python).
pub const KAP_TOEPFE_PARTNER: [&str; 4] = [
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
];
/// `EUER_KOMPONENTEN`.
pub const EUER_KOMPONENTEN: [&str; 3] = [
    "betriebseinnahmen",
    "sonstige_betriebsausgaben",
    "afa_jahresbetrag",
];
/// `GEWINN_QUELLEN_MENGEN` = `EUER_KOMPONENTEN + ("gwg_anschaffungskosten_netto",)` — DIE eine Menge,
/// die den EUeR-Zweig einschaltet (Umschalter hier, Waechter in `bescheid_deklaration`).
pub const GEWINN_QUELLEN_MENGEN: [&str; 4] = [
    "betriebseinnahmen",
    "sonstige_betriebsausgaben",
    "afa_jahresbetrag",
    "gwg_anschaffungskosten_netto",
];
/// `MITU_FELDER`.
pub const MITU_FELDER: [&str; 4] = [
    "gewinnanteil",
    "verguetung_taetigkeit",
    "verguetung_darlehen",
    "verguetung_ueberlassung",
];

/// `DBA_STAAT_ISO` (`api_constants.py:548`).
const DBA_STAAT_ISO: [(&str, &str); 15] = [
    ("oesterreich", "at"),
    ("österreich", "at"),
    ("schweiz", "ch"),
    ("dänemark", "dk"),
    ("daenemark", "dk"),
    ("spanien", "es"),
    ("frankreich", "fr"),
    ("grossbritannien", "gb"),
    ("großbritannien", "gb"),
    ("luxemburg", "lu"),
    ("niederlande", "nl"),
    ("polen", "pl"),
    ("türkei", "tr"),
    ("tuerkei", "tr"),
    ("usa", "us"),
];

/// `DBA_METHOD_MAP` (`api_constants.py:529`).
const DBA_METHODE: [(&str, &str); 11] = [
    ("at", "freistellung"),
    ("ch", "anrechnung"),
    ("dk", "anrechnung"),
    ("es", "anrechnung"),
    ("fr", "anrechnung"),
    ("gb", "anrechnung"),
    ("lu", "anrechnung"),
    ("nl", "anrechnung"),
    ("pl", "anrechnung"),
    ("tr", "anrechnung"),
    ("us", "freistellung"),
];

/// `DBA_METHOD_MAP_ART` ((Staat, Einkunftsart) → Methode; bisher nur Polen).
const DBA_METHODE_ART: [((&str, &str), &str); 10] = [
    (("pl", "unbewegliches_vermoegen"), "freistellung"),
    (("pl", "unternehmensgewinne"), "freistellung"),
    (("pl", "dividenden"), "anrechnung"),
    (("pl", "zinsen"), "anrechnung"),
    (("pl", "lizenzgebuehren"), "anrechnung"),
    (("pl", "veraeusserungsgewinne"), "freistellung"),
    (("pl", "unselbstaendige_arbeit"), "freistellung"),
    (("pl", "aufsichtsratsverguetungen"), "anrechnung"),
    (("pl", "kuenstler_sportler"), "anrechnung"),
    (("pl", "ruhegehaelter"), "freistellung"),
];

/// Python `x.strip().lower()` fuer einen Text-Wert; jeder andere truthy Typ ist ein
/// `AttributeError` (`.strip()` existiert nur auf `str`).
fn strip_lower(v: &PyWert) -> Result<String, BescheidFehler> {
    match v {
        PyWert::Text(s) => Ok(s.trim_matches(py_leerraum).to_lowercase()),
        _ => Err(BescheidFehler::Python {
            klasse: "AttributeError",
            was: "strip() auf Nicht-Text",
        }),
    }
}

/// `dba_staat_iso`: Enum-Wert von `dba_staat` → ISO-Code; Unbekanntes bleibt unveraendert.
fn dba_staat_iso(staat: Option<&PyWert>) -> Result<String, BescheidFehler> {
    let Some(v) = staat.filter(|v| v.truthy()) else {
        return Ok(String::new());
    };
    let s = strip_lower(v)?;
    Ok(DBA_STAAT_ISO
        .iter()
        .find(|(k, _)| *k == s)
        .map_or(s, |(_, iso)| (*iso).to_owned()))
}

/// Methode zur Vermeidung der Doppelbesteuerung fuer (Staat, Einkunftsart): per-Einkunftsart-Eintrag
/// → pauschale Laendermethode → `"anrechnung"` (`api_constants.py:633`).
///
/// # Errors
/// [`BescheidFehler::Python`] (`AttributeError`), wenn `staat`/`einkunftsart` truthy und kein Text
/// sind — Python ruft dann `.strip()` auf einem Nicht-`str`.
///
/// ```
/// use bescheid::einkuenfte::dba_methode_fuer;
/// use domain::PyWert;
/// let t = |s: &str| PyWert::Text(s.to_owned());
/// assert_eq!(dba_methode_fuer(Some(&t("Österreich")), None).unwrap(), "freistellung");
/// assert_eq!(dba_methode_fuer(Some(&t("polen")), Some(&t("Dividenden"))).unwrap(), "anrechnung");
/// assert_eq!(dba_methode_fuer(None, None).unwrap(), "anrechnung");
/// ```
pub fn dba_methode_fuer(
    staat: Option<&PyWert>,
    einkunftsart: Option<&PyWert>,
) -> Result<&'static str, BescheidFehler> {
    let s = dba_staat_iso(staat)?;
    if s.is_empty() {
        return Ok("anrechnung");
    }
    if let Some(art) = einkunftsart.filter(|v| v.truthy()) {
        let art = strip_lower(art)?;
        if let Some((_, methode)) = DBA_METHODE_ART
            .iter()
            .find(|((st, ar), _)| *st == s && *ar == art)
        {
            return Ok(methode);
        }
    }
    Ok(DBA_METHODE
        .iter()
        .find(|(k, _)| *k == s)
        .map_or("anrechnung", |(_, m)| m))
}

// ---------------------------------------------------------------- Gewinn (§§ 13-18)

/// § 6 Abs. 2 GWG-Sofortabzug fuer EIN Asset (`_abzug` in `_gwg_sofortabzug_summe`), EURO.
fn gwg_abzug(fi: &Felder) -> Result<Euro, BescheidFehler> {
    let netto = feld_int_oder_null(fi, "gwg_anschaffungskosten_netto")?;
    // Alle `Euro::new(0)` hier rechnen ein Geraet OHNE Sofortabzug. Eine festgesetzte Zahl darf diese 0 nicht still
    // zeigen: `sperre::gesamt::gwg` sperrt jede solche Instanz mit Betrag > 0 (`GwgAbschreibungOffen` /
    // `GwgMehrwertsteuerOffen`, seit 2026-10-03). Die 0 gilt nur fuer die Schaetzung (/stand). Beide Stellen muessen
    // dieselbe Instanz gleich lesen: wer hier einen Fall freigibt, gibt ihn dort frei (Folgefrage, siehe unten).
    // CENT-GUARD: die 800-EUR-Schwelle wird VOR der Euro-Rundung in Cent geprueft.
    if netto > 80_000 {
        return Ok(Euro::new(0));
    }
    // Ein verneinter Tatbestand ist eine rechenbare Antwort: das WG gehoert in die AfA. Ausnahme "netto: nein": wer die
    // Mehrwertsteuer nicht zurueckbekommt (`gwg_ohne_vorsteuerabzug`, Folgefrage), zieht den eingegebenen Bruttobetrag ab
    // (§ 9b Abs. 1 EStG; Abweichung Nr. 27). Die Grenze von 800 EUR hat oben schon auf diesem Betrag entschieden.
    let brutto_ist_abzug = ist_true(wert(fi, "gwg_ohne_vorsteuerabzug"));
    if ist_false(wert(fi, "gwg_bewegliches_selbstaendig_nutzbar"))
        || (ist_false(wert(fi, "gwg_netto_ohne_vorsteuer")) && !brutto_ist_abzug)
        || (netto > 25_000 && ist_false(wert(fi, "gwg_verzeichnis_ab_250")))
    {
        return Ok(Euro::new(0));
    }
    Ok(p6_2_gwg(&P62GwgEingabe {
        gwg_anschaffungskosten_netto: cent_zu_euro(netto),
    })?)
}

/// § 6 Abs. 2 GWG-Sofortabzug-Σ (EURO): stumpfe Σ ueber ALLE `gwg`-Instanzen; ohne Store/Bindung nur
/// das Basis-Feld aus `f`.
///
/// **Sicherheit:** bei `nur_bestaetigt` zaehlt eine vorlaeufige Instanz nicht (Zwei-Signal-Filter
/// am Instanz-Pfad, der den Store separat vom gefilterten Snapshot liest).
///
/// # Errors
/// Instanz-, Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::gwg_sofortabzug_summe;
/// use bescheid::testhilfe::{felder, store};
/// use bescheid::Instanzquelle;
/// use serde_json::json;
/// let f = felder(&store(&[("gwg_anschaffungskosten_netto", json!(50_000), true)]));
/// let q = Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
/// assert_eq!(gwg_sofortabzug_summe(&f, &q).unwrap().get(), 500); // 500 EUR netto, Sofortabzug
/// let zu_teuer = felder(&store(&[("gwg_anschaffungskosten_netto", json!(90_000), true)]));
/// assert_eq!(gwg_sofortabzug_summe(&zu_teuer, &q).unwrap().get(), 0); // > 800 EUR: § 7-AfA
/// ```
pub fn gwg_sofortabzug_summe(f: &Felder, q: &Instanzquelle<'_>) -> Result<Euro, BescheidFehler> {
    if q.beide().is_none() {
        return gwg_abzug(f);
    }
    let mut total = Euro::new(0);
    for inst in q.instanzen("gwg")? {
        if q.zaehlt(&inst) {
            total = euro_plus(total, gwg_abzug(&inst.felder)?)?;
        }
    }
    Ok(total)
}

/// Mitunternehmer-Komponente (§ 15 Abs. 1 S. 1 Nr. 2) aus den vier `MITU_FELDER` mit Suffix.
fn mitu_komponente(f: &Felder, suffix: &str) -> Result<Euro, BescheidFehler> {
    let fid = |k: &str| format!("{k}{suffix}");
    // PARITÄT: fail-open default — jedes fehlende Feld ist 0.
    let alle_null = MITU_FELDER.iter().try_fold(true, |acc, k| {
        Ok::<_, BescheidFehler>(acc && feld_int_oder_null(f, &fid(k))? == 0)
    })?;
    if alle_null {
        return Ok(Euro::new(0));
    }
    Ok(mitunternehmer_einkuenfte(
        &MitunternehmerEinkuenfteEingabe {
            gewinnanteil: feld_euro_oder_null(f, &fid("gewinnanteil"))?,
            verguetung_taetigkeit: feld_euro_oder_null(f, &fid("verguetung_taetigkeit"))?,
            verguetung_darlehen: feld_euro_oder_null(f, &fid("verguetung_darlehen"))?,
            verguetung_ueberlassung: feld_euro_oder_null(f, &fid("verguetung_ueberlassung"))?,
        },
    )?)
}

/// §§ 13-18 laufender Gewinn des EHEGATTEN (§ 26b), EURO: `(laufender_gewinn, mitu)`. Bewusst
/// schmaler als [`laufender_gewinn`]: keine EUeR-Felder, keine PV-Kuerzung.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::laufender_gewinn_partner;
/// use bescheid::testhilfe::{felder, store};
/// use serde_json::json;
/// let f = felder(&store(&[("einkuenfte_gewinn_partner", json!(5_000_000), true)]));
/// let (gewinn, mitu) = laufender_gewinn_partner(&f).unwrap();
/// assert_eq!((gewinn.get(), mitu.get()), (50_000, 0));
/// ```
pub fn laufender_gewinn_partner(f: &Felder) -> Result<(Euro, Euro), BescheidFehler> {
    let mitu = mitu_komponente(f, "_partner")?;
    Ok((
        euro_plus(feld_euro_oder_null(f, "einkuenfte_gewinn_partner")?, mitu)?,
        mitu,
    ))
}

/// §§ 13-18 laufender Gewinn (§ 15 Gewerbe / § 18 selbstaendig), EURO: `(laufender_gewinn, mitu)`.
/// EUeR-Zweig (§ 4 Abs. 3), wenn irgendeine Komponente aus [`GEWINN_QUELLEN_MENGEN`] oder ein GWG
/// vorliegt — der Gewinn KANN negativ sein; sonst der Direktwert `einkuenfte_gewinn`. Danach
/// § 3 Nr. 72 (PV-Freistellung), nie unter 0.
///
/// # Errors
/// Instanz-, Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::laufender_gewinn;
/// use bescheid::testhilfe::{felder, store};
/// use bescheid::Instanzquelle;
/// use serde_json::json;
/// let f = felder(&store(&[("einkuenfte_gewinn", json!(3_000_000), true)]));
/// let q = Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
/// let (gewinn, mitu) = laufender_gewinn(&f, &q).unwrap();
/// assert_eq!((gewinn.get(), mitu.get()), (30_000, 0));
/// ```
pub fn laufender_gewinn(f: &Felder, q: &Instanzquelle<'_>) -> Result<(Euro, Euro), BescheidFehler> {
    let gwg_summe = gwg_sofortabzug_summe(f, q)?;
    let mitu = mitu_komponente(f, "")?;
    let quelle_da = GEWINN_QUELLEN_MENGEN.iter().try_fold(false, |acc, k| {
        Ok::<_, BescheidFehler>(acc || feld_int_oder_null(f, k)? != 0)
    })?;
    let mut gewinn = if quelle_da || gwg_summe.get() > 0 {
        let ausgaben = summe(&[
            feld_int_oder_null(f, "sonstige_betriebsausgaben")?,
            feld_int_oder_null(f, "afa_jahresbetrag")?,
        ])?;
        let euer = euer_gewinn(&EuerGewinnEingabe {
            betriebseinnahmen: feld_euro_oder_null(f, "betriebseinnahmen")?,
            betriebsausgaben: euro_plus(cent_zu_euro(ausgaben), gwg_summe)?,
        })?;
        euro_plus(euer, mitu)?
    } else {
        euro_plus(feld_euro_oder_null(f, "einkuenfte_gewinn")?, mitu)?
    };
    let pv_frei = p3_nr72_photovoltaik(&P3Nr72PhotovoltaikEingabe {
        pv_einnahmen: feld_euro_oder_null(f, "pv_einnahmen")?,
        // PARITÄT: fail-open default — fehlende Leistung/Einheiten = 0 (dann keine Befreiung).
        pv_bruttoleistung_kwp: feld_int_oder_null(f, "pv_bruttoleistung_kwp")?,
        pv_anzahl_einheiten: feld_int_oder_null(f, "pv_anzahl_einheiten")?,
        pv_auf_gebaeude: ist_true(wert(f, "pv_auf_gebaeude")),
    })?;
    if pv_frei.get() > 0 {
        let abzug = pv_frei.get().min(gewinn.get().max(0));
        // Die PV-Freistellung (§ 3 Nr. 72) mindert nur einen positiven Gewinn, nie darunter.
        debug_assert!((0..=pv_frei.get()).contains(&abzug));
        gewinn = Euro::new(minus(gewinn.get(), abzug)?);
        debug_assert!(gewinn.get() >= 0 || abzug == 0);
    }
    Ok((gewinn, mitu))
}

/// Der Beitrag des Ehegatten zu `einkuenfte_gewinn` (EURO): `(laufender Gewinn + § 16-vg netto,
/// mitu, § 16-vg netto allein)`. Nur bei Zusammenveranlagung, sonst `(0, 0, 0)`; der § 16 Abs. 4-
/// Freibetrag wird je Person gerechnet und nie zum Verlust.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::gewinn_partner_anteil;
/// use bescheid::testhilfe::{felder, store};
/// use serde_json::json;
/// let einzel = felder(&store(&[("einkuenfte_gewinn_partner", json!(5_000_000), true)]));
/// assert_eq!(gewinn_partner_anteil(&einzel).unwrap().0.get(), 0); // nur bei Zusammenveranlagung
/// let zusammen = felder(&store(&[("veranlagung", json!("zusammen"), true), ("einkuenfte_gewinn_partner", json!(5_000_000), true)]));
/// assert_eq!(gewinn_partner_anteil(&zusammen).unwrap().0.get(), 50_000);
/// ```
pub fn gewinn_partner_anteil(f: &Felder) -> Result<(Euro, Euro, Euro), BescheidFehler> {
    let null = Euro::new(0);
    if !ist_zusammen(f) {
        return Ok((null, null, null));
    }
    let (laufend, mitu) = laufender_gewinn_partner(f)?;
    let netto_vg = netto_vg_partner(f)?;
    Ok((euro_plus(laufend, netto_vg)?, mitu, netto_vg))
}

/// Der § 16-Veraeusserungsgewinn des Ehegatten NACH dem § 16 Abs. 4-Freibetrag (EURO), bei 0 gefloort; nur bei
/// Zusammenveranlagung, sonst 0. Der Freibetrag gilt je Person und nur bei bestaetigtem S. 1 UND S. 2. Eine Stelle fuer
/// [`gewinn_partner_anteil`] und die Antragszeile/Sperren des § 34 Abs. 3 (`abs3_wird_gerechnet_partner`): sie lesen den
/// Gewinn, ohne den laufenden Gewinn des Partners zu rechnen.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::netto_vg_partner;
/// use bescheid::testhilfe::{felder, store};
/// use serde_json::json;
/// let zusammen = felder(&store(&[
///     ("veranlagung", json!("zusammen"), true),
///     ("rentner_veraeusserungsgewinn_partner", json!(10_000_000), true),
/// ]));
/// assert_eq!(netto_vg_partner(&zusammen).unwrap().get(), 100_000); // ohne bestaetigte S.-1/S.-2-Angaben: kein Freibetrag
/// ```
pub fn netto_vg_partner(f: &Felder) -> Result<Euro, BescheidFehler> {
    let null = Euro::new(0);
    if !ist_zusammen(f) {
        return Ok(null);
    }
    let vg_euro = feld_euro_oder_null(f, "rentner_veraeusserungsgewinn_partner")?;
    // Nur ein bestaetigtes True auf BEIDEN Bools gewaehrt den Freibetrag (S. 1 erfuellt).
    let gate_ok = ist_true(wert(f, "rentner_alter_55_oder_berufsunfaehig_partner"))
        && ist_true(wert(f, "rentner_freibetrag_erstmalig_partner"));
    let fb = if gate_ok {
        p16_4_freibetrag(&P164FreibetragEingabe {
            rentner_veraeusserungsgewinn: vg_euro,
        })?
    } else {
        null
    };
    // GEFLOORT bei 0: FB > vg darf keinen Phantom-Verlust erzeugen.
    let netto_vg = Euro::new(minus(vg_euro.get(), fb.get())?.max(0));
    // Der Freibetrag gehoert zu einem Gewinn: ohne Gewinn kein Freibetrag, nie ein Phantom-Verlust.
    debug_assert!(netto_vg.get() >= 0 && fb.get() >= 0);
    Ok(netto_vg)
}

// ---------------------------------------------------------------- Kapital (§ 20) und § 23

/// § 20 Abs. 6 Verlustverrechnung + Abs. 9 Sparer-Pauschbetrag, fuer beide Zweige, EURO.
///
/// Toepfe XOR Aggregat. Bei Zusammenveranlagung kommt das Kapital des Ehegatten ROH dazu, VOR dem
/// gemeinsamen Sparer-PB. `zusammen` kommt vom Aufrufer (Veranlagungsart ausschliesslich aus § 26).
/// Python uebergibt hier den Closure `_c`; er liest immer `f` — deshalb `f`.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::p20_kapitaleinkuenfte;
/// use bescheid::testhilfe::{felder, params, store};
/// use domain::Vz;
/// use serde_json::json;
/// let f = felder(&store(&[("kap_kapitalertraege", json!(500_000), true)]));
/// // 5.000 EUR Erträge abzüglich Sparer-Pauschbetrag 1.000 EUR
/// assert_eq!(p20_kapitaleinkuenfte(&f, false, Vz::Vz2025, params()).unwrap().get(), 4_000);
/// ```
pub fn p20_kapitaleinkuenfte(
    f: &Felder,
    zusammen: bool,
    vz: Vz,
    p: &Params,
) -> Result<Euro, BescheidFehler> {
    let toepfe_belegt = |toepfe: &[&str]| -> Result<bool, BescheidFehler> {
        toepfe
            .iter()
            .try_fold(false, |acc, t| Ok(acc || feld_int_oder_null(f, t)? != 0))
    };
    let verrechne = |suffix: &str| -> Result<Euro, BescheidFehler> {
        let fid = |k: &str| format!("{k}{suffix}");
        Ok(kapital_verrechnung(&KapitalVerrechnungEingabe {
            gewinn_aktien: feld_euro_oder_null(f, &fid("kap_gewinn_aktien"))?,
            verlust_aktien: feld_euro_oder_null(f, &fid("kap_verlust_aktien"))?,
            gewinn_sonstige: feld_euro_oder_null(f, &fid("kap_gewinn_sonstige"))?,
            verlust_sonstige: feld_euro_oder_null(f, &fid("kap_verlust_sonstige"))?,
        })?)
    };
    let mut verrechnete = if toepfe_belegt(&KAP_TOEPFE)? {
        verrechne("")?
    } else {
        feld_euro_oder_null(f, KAP_ERTRAEGE)?
    };
    if zusammen {
        let partner = if toepfe_belegt(&KAP_TOEPFE_PARTNER)? {
            verrechne("_partner")?
        } else {
            feld_euro_oder_null(f, KAP_ERTRAEGE_PARTNER)?
        };
        verrechnete = euro_plus(verrechnete, partner)?;
    }
    Ok(sparer_pb(
        &SparerPbEingabe {
            vz,
            kapitalertraege: verrechnete,
            zusammenveranlagung: zusammen,
        },
        p,
    )?)
}

/// § 23 Private Veraeusserungsgeschaefte (Stufe 1), EURO — Σ ueber ALLE `p23_veraeusserung`-
/// Instanzen: Gewinne → `gewinn_pvg`, Betraege der Verluste → `verlust_pvg`; Freigrenze 1.000 EUR
/// auf den Gesamtgewinn (§ 23 Abs. 3 S. 5), Verlusttopf `max(0, gewinn - verlust)` (S. 7).
/// Ohne Store oder ohne Bindung 0.
///
/// Python nimmt zusaetzlich `f`, liest es aber nie — der Parameter entfaellt.
///
/// **Sicherheit:** bei `nur_bestaetigt` zaehlt eine vorlaeufige Instanz nicht.
///
/// # Errors
/// Instanz-, Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::p23_ansonsten_einkuenfte;
/// use bescheid::testhilfe::{index, store};
/// use bescheid::Instanzquelle;
/// use serde_json::json;
/// let st = store(&[
///     ("p23_veraeusserungspreis", json!(500_000), true),
///     ("p23_anschaffung_herstellungskosten", json!(300_000), true),
/// ]);
/// let q = Instanzquelle { store: Some(&st), bindung: Some(index()), nur_bestaetigt: true };
/// assert_eq!(p23_ansonsten_einkuenfte(&q).unwrap().get(), 2_000); // Gewinn 2.000 EUR > Freigrenze 1.000 EUR
/// let leer = Instanzquelle { store: None, bindung: None, nur_bestaetigt: true };
/// assert_eq!(p23_ansonsten_einkuenfte(&leer).unwrap().get(), 0);
/// ```
pub fn p23_ansonsten_einkuenfte(q: &Instanzquelle<'_>) -> Result<Euro, BescheidFehler> {
    let null = Euro::new(0);
    if q.beide().is_none() {
        return Ok(null);
    }
    let (mut gewinn_pvg, mut verlust_pvg) = (null, null);
    for inst in q.instanzen("p23_veraeusserung")? {
        if !q.zaehlt(&inst) {
            continue;
        }
        // PARITÄT: fail-open default — fehlender/nicht-numerischer Betrag = 0.
        let betrag = |fid: &str| -> Result<Euro, BescheidFehler> {
            Ok(cent_zu_euro(zahl_oder_null(wert(&inst.felder, fid))?))
        };
        let gewinn = p23_veraeusserungsgewinn(&VeraeusserungsgewinnEingabe {
            veraeusserungspreis: betrag("p23_veraeusserungspreis")?,
            anschaffungs_herstellungskosten: betrag("p23_anschaffung_herstellungskosten")?,
            werbungskosten: betrag("p23_werbungskosten")?,
        })?;
        if gewinn.get() > 0 {
            gewinn_pvg = euro_plus(gewinn_pvg, gewinn)?;
        } else {
            let verlust = gewinn
                .get()
                .checked_abs()
                .ok_or(BescheidFehler::Ueberlauf("abs"))?;
            verlust_pvg = euro_plus(verlust_pvg, Euro::new(verlust))?;
        }
    }
    // Gewinne und Verlustbetraege wachsen nur (je Instanz nur eine Seite, Betrag >= 0).
    debug_assert!(gewinn_pvg.get() >= 0 && verlust_pvg.get() >= 0);
    let gesamtgewinn = Euro::new(minus(gewinn_pvg.get(), verlust_pvg.get())?);
    if p23_freigrenze(gesamtgewinn).get() <= 0 {
        return Ok(null);
    }
    Ok(p23_verlusttopf(&VerlusttopfEingabe {
        gewinn_pvg,
        verlust_pvg,
    })?)
}

// ---------------------------------------------------------------- DBA (§ 34c, § 32b)

/// Ergebnis von [`shared_dba_sonstige`]. `g` traegt `sonstige_abzuege_vom_einkommen` und
/// `anzurechnende_auslaendische_steuern`; `p32b_progressionseinkuenfte` hat in [`GesamtfallEingabe`]
/// keinen Platz (Python schreibt es nur in `g_dict`, kein Rechenweg liest es dort).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbaErgebnis {
    /// Der zurueckgegebene `dba_anrechnung` (EURO).
    pub dba_anrechnung: Euro,
    /// `Some(ausland)` nur im Freistellungs-Zweig (Progressionsvorbehalt).
    pub p32b_progressionseinkuenfte: Option<Euro>,
}

/// Die DBA-Methode der Akte: der Wert `dba_freistellung` im Feld `dba_methode` gilt, sonst entscheiden Staat und
/// Einkunftsart ([`dba_methode_fuer`]). Die Rechnung ([`shared_dba_sonstige`]) und die Sperren lesen dieselbe Stelle.
///
/// # Errors
/// Python-`AttributeError` (`dba_staat` kein Text).
pub(crate) fn dba_methode_der_akte(f: &Felder) -> Result<&'static str, BescheidFehler> {
    if matches!(wert(f, "dba_methode"), Some(PyWert::Text(s)) if s == "dba_freistellung") {
        Ok("freistellung")
    } else {
        dba_methode_fuer(wert(f, "dba_staat"), wert(f, "dba_einkunftsart"))
    }
}

/// Wahr, wenn die Akte freigestellte Auslandseinkuenfte ueber 0 Euro traegt (Methode Freistellung, § 32b Abs. 1 S. 1
/// Nr. 3 `EStG`). Genau diese Faelle setzen in [`shared_dba_sonstige`] einen Progressionsvorbehalt ueber 0.
///
/// # Errors
/// Accessor- und `AttributeError`-Fehler wie [`shared_dba_sonstige`].
pub(crate) fn dba_freistellung_aktiv(f: &Felder) -> Result<bool, BescheidFehler> {
    let ausland = feld_euro_oder_null(f, "dba_auslaendische_einkuenfte")?;
    Ok(ausland.get() > 0 && dba_methode_der_akte(f)? == "freistellung")
}

/// § 33a (Unterhalt, Ausbildungsfreibetrag) + § 10d Abs. 2 (Verlustabzug) + DBA-Anrechnung (§ 34c).
/// Setzt `g.sonstige_abzuege_vom_einkommen` und `g.anzurechnende_auslaendische_steuern`; die
/// Anrechnungs-Rechnung liest `g` nach dem ersten der beiden (Reihenfolge wie Python).
///
/// `f` ist der Feld-Snapshot der aufrufenden Quantitaet, `gde_p10d` der `GdE` fuer § 10d.
///
/// Wahl `dba_abzug_statt_anrechnung` (§ 34c Abs. 2) und Methode Freistellung schliessen sich aus: die Freistellung gewinnt,
/// kein Abzug (Abweichung Nr. 35, Test `p34c_abzug_rechnung.rs`).
///
/// # Errors
/// Accessor-, Python-`AttributeError`- (`dba_staat` kein Text) und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::shared_dba_sonstige;
/// use bescheid::testhilfe::{felder, leerer_gesamtfall, params, store};
/// use domain::{Euro, Veranlagung, Vz};
/// use serde_json::json;
/// let f = felder(&store(&[("dba_auslaendische_einkuenfte", json!(1_000_000), true), ("dba_methode", json!("dba_freistellung"), true)]));
/// let mut g = leerer_gesamtfall(Vz::Vz2025, false);
/// let erg = shared_dba_sonstige(&mut g, Euro::new(50_000), Veranlagung::Einzel, &f, Vz::Vz2025, params()).unwrap();
/// // Freistellung: keine Anrechnung, die Auslandseinkünfte gehen in den Progressionsvorbehalt
/// assert_eq!(erg.dba_anrechnung.get(), 0);
/// assert_eq!(erg.p32b_progressionseinkuenfte, Some(Euro::new(10_000)));
/// ```
pub fn shared_dba_sonstige(
    g: &mut GesamtfallEingabe,
    gde_p10d: Euro,
    veranlagung: Veranlagung,
    f: &Felder,
    vz: Vz,
    p: &Params,
) -> Result<DbaErgebnis, BescheidFehler> {
    let unt = p33a_unterhalt(&UnterhaltEingabe {
        vz,
        aufwendungen: feld_euro_oder_null(f, "p33a_unterhalt_aufwendungen")?,
        kv_pv_beitraege: feld_euro_oder_null(f, "p33a_unterhalt_kv_pv")?,
        andere_einkuenfte_bezuege: feld_euro_oder_null(f, "p33a_andere_einkuenfte_bezuege")?,
    })?;
    // PARITÄT: fail-open default — fehlende Kinderzahl = 0.
    let ausbildung =
        p33a_ausbildungsfreibetrag(feld_int_oder_null(f, "p33a_ausbildung_anzahl_kinder")?)?;
    let verlust = p10d_2(&VerlustabzugEingabe {
        gesamtbetrag_einkuenfte: gde_p10d,
        verlustvortrag_bestand: feld_euro_oder_null(f, "verlustvortrag_bestand")?,
        zusammenveranlagung: veranlagung == Veranlagung::Zusammen,
    })?;
    g.sonstige_abzuege_vom_einkommen = euro_plus(euro_plus(verlust, unt)?, ausbildung)?;

    let gezahlt = feld_euro_oder_null(f, "dba_gezahlte_auslaendische_steuer")?;
    let ausland = feld_euro_oder_null(f, "dba_auslaendische_einkuenfte")?;
    // Python berechnet die Methode IMMER (auch wenn kein Zweig sie braucht) — sie kann werfen.
    let methode = dba_methode_der_akte(f)?;
    let mut anrechnung = Euro::new(0);
    let mut progression = None;
    // ABWEICHUNG VON PYTHON (Nr. 35): bei Freistellung gibt es keinen Abzug (§ 34c Abs. 6 S. 1 und 2 EStG: Abs. 2 gilt
    // nur, wo das Abkommen die Anrechnung vorsieht). Python bucht den Abzug vor der Methodenpruefung; Rust laesst die
    // Freistellung gewinnen, dann gilt der Zweig darunter (Progressionsvorbehalt).
    if ist_true(wert(f, "dba_abzug_statt_anrechnung"))
        && gezahlt.get() > 0
        && ausland.get() > 0
        && methode != "freistellung"
    {
        g.sonstige_abzuege_vom_einkommen = euro_plus(g.sonstige_abzuege_vom_einkommen, gezahlt)?;
    } else if gezahlt.get() > 0 || ausland.get() > 0 {
        if methode == "freistellung" {
            progression = Some(ausland);
        } else {
            anrechnung = p34c_1(&AuslaendischeSteuerEingabe {
                gezahlte_auslaendische_steuer: gezahlt,
                deutsche_est_inkl_ausl: gesamt_tarifliche(g, p)?,
                zu_versteuerndes_einkommen: gesamt_zve(g, p)?,
                auslaendische_einkuenfte_staat: ausland,
            })?;
        }
    }
    g.anzurechnende_auslaendische_steuern = anrechnung;
    Ok(DbaErgebnis {
        dba_anrechnung: anrechnung,
        p32b_progressionseinkuenfte: progression,
    })
}

// ---------------------------------------------------------------- § 35 GewSt-Anrechnung

/// `(messbetrag_euro, hebesatz, gewerbliche_einkuenfte_euro)` des Ehegatten fuer § 35. Nur bei
/// Zusammenveranlagung, sonst `(0, 0, 0)`. Der Zaehler nimmt den LAUFENDEN Gewinn ohne § 16-vg
/// (§ 7 S. 2 `GewStG`); nur ein Gewerbebetrieb liefert den vollen laufenden Gewinn, sonst zaehlt
/// allein der § 15-Mitunternehmeranteil.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::p35_partner_anteile;
/// use bescheid::testhilfe::{felder, store};
/// use serde_json::json;
/// let einzel = felder(&store(&[("gewst_messbetrag_partner", json!(100_000), true)]));
/// assert_eq!(p35_partner_anteile(&einzel).unwrap().1, 0);
/// let zusammen = felder(&store(&[("veranlagung", json!("zusammen"), true), ("gewst_messbetrag_partner", json!(100_000), true), ("gewst_hebesatz_partner", json!(400), true)]));
/// let (messbetrag, hebesatz, _zaehler) = p35_partner_anteile(&zusammen).unwrap();
/// assert_eq!((messbetrag.get(), hebesatz), (1_000, 400));
/// ```
pub fn p35_partner_anteile(f: &Felder) -> Result<(Euro, i64, Euro), BescheidFehler> {
    if !ist_zusammen(f) {
        return Ok((Euro::new(0), 0, Euro::new(0)));
    }
    let (laufend, mitu) = laufender_gewinn_partner(f)?;
    let gewerbe =
        matches!(wert(f, "gewinn_betriebsart_partner"), Some(PyWert::Text(s)) if s == "gewerbe");
    let zaehler = Euro::new(if gewerbe { laufend.get() } else { mitu.get() }.max(0));
    // PARITÄT: fail-open default — fehlender Messbetrag/Hebesatz = 0.
    Ok((
        feld_euro_oder_null(f, "gewst_messbetrag_partner")?,
        feld_int_oder_null(f, "gewst_hebesatz_partner")?,
        zaehler,
    ))
}

/// `messbetrag * hebesatz // 100` (Hebesatz in Prozent, Floor wie Python).
fn gewst_je_betrieb(messbetrag: Euro, hebesatz: i64) -> Result<i64, BescheidFehler> {
    let v = i128::from(messbetrag.get()) * i128::from(hebesatz);
    i64::try_from(v.div_euclid(100)).map_err(|_| BescheidFehler::Ueberlauf("Messbetrag*Hebesatz"))
}

/// § 35 Abs. 1 S. 5: die tatsaechlich gezahlte Gewerbesteuer, je Betrieb mit dem EIGENEN Hebesatz
/// ermittelt und dann summiert. Der Deckel (`min(...)` im Zweig) laeuft ueber die Summe.
///
/// # Errors
/// [`BescheidFehler::Ueberlauf`].
///
/// ```
/// use bescheid::einkuenfte::p35_gezahlte_gewst;
/// use domain::Euro;
/// // je Betrieb Messbetrag × Hebesatz / 100, dann summiert
/// assert_eq!(p35_gezahlte_gewst(Euro::new(1_000), 400, Euro::new(500), 300).unwrap(), Euro::new(5_500));
/// ```
pub fn p35_gezahlte_gewst(
    messbetrag_a: Euro,
    hebesatz_a: i64,
    messbetrag_b: Euro,
    hebesatz_b: i64,
) -> Result<Euro, BescheidFehler> {
    plus(
        gewst_je_betrieb(messbetrag_a, hebesatz_a)?,
        gewst_je_betrieb(messbetrag_b, hebesatz_b)?,
    )
    .map(Euro::new)
}

/// `(messbetrag_ges, zaehler_ges, gezahlt)` fuer § 35 — Person A plus Ehegatte.
///
/// # Errors
/// Accessor- und Ueberlauf-Fehler.
///
/// ```
/// use bescheid::einkuenfte::p35_summen;
/// use bescheid::Felder;
/// use domain::Euro;
/// let (mb, zaehler, gezahlt) = p35_summen(&Felder::new(), Euro::new(1_000), 400, Euro::new(20_000)).unwrap();
/// assert_eq!((mb.get(), zaehler.get(), gezahlt.get()), (1_000, 20_000, 4_000));
/// ```
pub fn p35_summen(
    f: &Felder,
    messbetrag_a: Euro,
    hebesatz_a: i64,
    zaehler_a: Euro,
) -> Result<(Euro, Euro, Euro), BescheidFehler> {
    let (messbetrag_b, hebesatz_b, zaehler_b) = p35_partner_anteile(f)?;
    Ok((
        euro_plus(messbetrag_a, messbetrag_b)?,
        euro_plus(zaehler_a, zaehler_b)?,
        p35_gezahlte_gewst(messbetrag_a, hebesatz_a, messbetrag_b, hebesatz_b)?,
    ))
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::testhilfe::{felder, index, params, store};
    use domain::testhilfe::py;

    #[test]
    fn dba_methode_wie_python() {
        let m = |s: Value, a: Option<Value>| {
            let (s, a) = (py(&s), a.as_ref().map(py));
            dba_methode_fuer(Some(&s), a.as_ref())
        };
        assert_eq!(m(json!(" ÖSTERREICH "), None).unwrap(), "freistellung");
        assert_eq!(m(json!("Türkei"), None).unwrap(), "anrechnung");
        assert_eq!(
            m(json!("polen"), Some(json!("Unternehmensgewinne"))).unwrap(),
            "freistellung"
        );
        assert_eq!(
            m(json!("polen"), Some(json!("unbekannt"))).unwrap(),
            "anrechnung"
        );
        assert_eq!(m(json!(0), Some(json!(7))).unwrap(), "anrechnung"); // falsy staat: einkunftsart ungelesen
        assert!(matches!(
            m(json!(5), None),
            Err(BescheidFehler::Python {
                klasse: "AttributeError",
                ..
            })
        ));
        assert!(m(json!("pl"), Some(json!(5))).is_err());
    }

    #[test]
    fn gewst_je_betrieb_rundet_gegen_minus_unendlich() {
        // Python: -1 * 5 // 100 == -1
        assert_eq!(
            p35_gezahlte_gewst(Euro::new(-1), 5, Euro::new(0), 0).unwrap(),
            Euro::new(-1)
        );
        assert_eq!(
            p35_gezahlte_gewst(Euro::new(1000), 400, Euro::new(500), 380).unwrap(),
            Euro::new(4000 + 1900)
        );
    }

    #[test]
    fn p35_partner_nur_bei_zusammenveranlagung() {
        let einzel = felder(&store(&[("gewst_messbetrag_partner", json!(50_000), true)]));
        assert_eq!(
            p35_partner_anteile(&einzel).unwrap(),
            (Euro::new(0), 0, Euro::new(0))
        );
        let zus = felder(&store(&[
            ("veranlagung", json!("zusammen"), true),
            ("gewst_messbetrag_partner", json!(50_000), true),
            ("gewst_hebesatz_partner", json!(400), true),
            ("gewinn_betriebsart_partner", json!("gewerbe"), true),
            ("einkuenfte_gewinn_partner", json!(2_000_000), true),
        ]));
        assert_eq!(
            p35_partner_anteile(&zus).unwrap(),
            (Euro::new(500), 400, Euro::new(20_000))
        );
        let (m, z, g) = p35_summen(&zus, Euro::new(100), 300, Euro::new(1000)).unwrap();
        assert_eq!(
            (m, z, g),
            (Euro::new(600), Euro::new(21_000), Euro::new(300 + 2000))
        );
    }

    #[test]
    fn laufender_gewinn_direkt_und_euer() {
        let leer = Instanzquelle {
            store: None,
            bindung: None,
            nur_bestaetigt: true,
        };
        let direkt = felder(&store(&[("einkuenfte_gewinn", json!(1_000_000), true)]));
        assert_eq!(
            laufender_gewinn(&direkt, &leer).unwrap(),
            (Euro::new(10_000), Euro::new(0))
        );
        let euer = felder(&store(&[
            ("betriebseinnahmen", json!(5_000_000), true),
            ("sonstige_betriebsausgaben", json!(1_000_000), true),
        ]));
        assert_eq!(laufender_gewinn(&euer, &leer).unwrap().0, Euro::new(40_000));
        // Verlustjahr: Floor bei negativem Cent-Wert (-150 Cent → -2 EUR).
        let verlust = felder(&store(&[("einkuenfte_gewinn", json!(-150), true)]));
        assert_eq!(laufender_gewinn(&verlust, &leer).unwrap().0, Euro::new(-2));
    }

    #[test]
    fn p23_freigrenze_und_verlusttopf() {
        let mk = |preis2: i64| {
            store(&[
                ("p23_veraeusserungspreis", json!(250_000), true),
                ("p23_anschaffung_herstellungskosten", json!(100_000), true),
                ("p23_veraeusserungspreis__2", json!(preis2), true),
                (
                    "p23_anschaffung_herstellungskosten__2",
                    json!(150_000),
                    true,
                ),
            ])
        };
        let q = |s| Instanzquelle {
            store: Some(s),
            bindung: Some(index()),
            nur_bestaetigt: true,
        };
        // +1500 und -500 → Gesamt 1000 (Freigrenze erreicht), Topf 1000
        let s = mk(100_000);
        assert_eq!(p23_ansonsten_einkuenfte(&q(&s)).unwrap(), Euro::new(1000));
        // +1500 und -501 → Gesamt 999 → unter der Freigrenze → 0
        let s = mk(99_900);
        assert_eq!(p23_ansonsten_einkuenfte(&q(&s)).unwrap(), Euro::new(0));
        let ohne = Instanzquelle {
            store: None,
            bindung: Some(index()),
            nur_bestaetigt: true,
        };
        assert_eq!(p23_ansonsten_einkuenfte(&ohne).unwrap(), Euro::new(0));
    }

    #[test]
    fn dba_freistellung_setzt_progression_anrechnung_nicht() {
        let p = params();
        let mut g = crate::einkuenfte::tests::gesamt_null();
        let f = felder(&store(&[
            ("dba_staat", json!("Österreich"), true),
            ("dba_gezahlte_auslaendische_steuer", json!(100_000), true),
            ("dba_auslaendische_einkuenfte", json!(500_000), true),
        ]));
        let r = shared_dba_sonstige(&mut g, Euro::new(0), Veranlagung::Einzel, &f, Vz::Vz2025, p)
            .unwrap();
        assert_eq!(r.dba_anrechnung, Euro::new(0));
        assert_eq!(r.p32b_progressionseinkuenfte, Some(Euro::new(5000)));
        assert_eq!(g.anzurechnende_auslaendische_steuern, Euro::new(0));
    }

    pub(crate) fn gesamt_null() -> GesamtfallEingabe {
        use engine::zugriff::teil2::rente::{
            EinkuenfteVersorgungEingabe, VersorgungsfreibetragEingabe,
        };
        let n = Euro::new(0);
        GesamtfallEingabe {
            vz: Vz::Vz2025,
            zusammenveranlagung: false,
            einkuenfte_nichtselbststaendig: Euro::new(50_000),
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
}
