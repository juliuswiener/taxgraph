//! Kz-Format: wie ein Store-Wert in eine ELSTER-Kennzahl geschrieben wird
//! (`est_mapping.py:31-241`, `_cent_nach_kz`, `_kz_wert`, `_schreibe_kz`, `_jahr_aus_kz_wert`).
//!
//! Rundung „zu Ihren Gunsten" (Anleitung ESt 1 A 2025, `anl_est1a_2025.txt:269-274`):
//! Einnahmen/Einkuenfte werden abgerundet, Abzuege/Aufwendungen/Verluste aufgerundet. Kz vom
//! XSD-Typ Dezimal mit zwei Nachkommastellen (`E60…` und die Liste [`KOMMA_OHNE_E60_KZ`]) werden
//! nicht gerundet, sondern exakt als `"N,NN"` geschrieben.

use std::collections::BTreeMap;

use domain::{Cent, Euro, Feldtyp, Kz};
use serde_json::Value;

use crate::py::{self, PyFehler};

/// Kz, die Abzuege/Aufwendungen/Verluste deklarieren — aufrunden (`est_mapping.py:35-102`). E0108701 (Spenden an Parteien,
/// Abweichung Nr. 31) und E0600920 (abgezogene auslaendische Steuer, § 34c Abs. 2, Abweichung Nr. 41) stehen nur in Rust.
///
/// E0703838 traegt kein Bindungsfeld mehr; es bleibt, weil es sachlich richtig klassifiziert ist.
/// Die KV/PV-Kz E2001203 … E2003202 schreibt die KV/PV-Weiche `basis_kv`/`basis_pv`.
/// Nicht aufgenommen: Kz, die einen Abzug MINDERN (E0107602, E0506604/05, E0205508, E0700501)
/// und Anrechnungsbetraege auf die Steuer (E0601901, E1905101, E0200301, E19047xx).
///
/// Die letzten vierzehn Eintraege (ab E0108002) sind Einzelposten zu Summen davor: § 35a,
/// § 35c je Massnahmenart, Berufsausbildung, Kinderbetreuung. ERiC prueft jede Summe gegen ihre
/// Posten, bei § 35c und Berufsausbildung ohne Toleranz (rc=610001002 ab 1 €). Rundete die Summe
/// auf und der Posten ab, war schon EIN Cent-Betrag uneinreichbar. Vault:
/// `decisions/aufwand-einzelposten-aufrunden-summe-aus-posten`.
pub const ABZUGS_KZ: &[&str] = &[
    "E0703838", "E2000401", "E2000801", "E2000601", "E1901301", "E1901201", "E0161804", "E0104109",
    "E0107208", "E0111215", "E2001203", "E2001505", "E2001805", "E2002105", "E2003104", "E2003202",
    "E2001403", "E2001503", "E2001803", "E2001903", "E2002003", "E0505607", "E0503110", "E0503310",
    "E0107601", "E0108202", "E0108105", "E0108701", "E0600920", "E0304601", "E0506105", "E0120103",
    "E0124401", "E0203611", "E0207611", "E0705701", "E0305201", "E0241901", "E0242001", "E0108002",
    "E0104108", "E0107207", "E0111214", "E0506104", "E0241001", "E0241101", "E0241201", "E0241301",
    "E0241302", "E0241401", "E0241501", "E0241601", "E0241701",
];

/// Kz vom XSD-Typ `DezimalzahlNichtNegOhneFuehrNull_MaxL15_MaxVK12_MinNK2_MaxNK2_CType_RABE`
/// (bzw. ohne `NichtNeg` fuer E1905101) ohne E60-Praefix (`est_mapping.py:128-142`). Ohne diese
/// Liste stuende der rohe Euro-Betrag im XML und checkESt lehnte ab („Geldbetraege muessen vom
/// Format '0,00' sein").
pub const KOMMA_OHNE_E60_KZ: &[&str] = &[
    "E0200301", "E0200501", "E1904701", "E1904901", "E1904801", "E1905101",
];

/// Kz, deren XSD-Typ eine 0 verbietet (`est_mapping.py:145-172`): `GanzzahlPos*` (Basis
/// `xs:positiveInteger`) oder eine Facette ohne `"0"` (`GdB` E0109708/E0505809: Muster
/// `20|25|…|100`; Pflegegrad E0161606: Aufzaehlung 2/3/4). ERiC lehnt eine 0 dort ab, und die
/// ganze Erklaerung ist uneinreichbar. In jeder dieser Kz heisst 0 „nichts anzugeben"
/// (`minOccurs 0`); `schreibe_kz` laesst sie weg (Vault:
/// `decisions/elster-null-in-kz-ohne-null-weglassen`).
///
/// Aus dem XSD abgeleitet ([`crate::xsd::kz_meta`] gegen `E10-2025.xsd`): jede cent-/int-Kz auf
/// einem Schreibweg von `deklariere`, deren Typ die 0 verbietet; Reihenfolge wie in Python (1:1,
/// Instanz und Person B; Spenden; die neun §35c-Kz der Art-Verzweigung). Kz mit Textwert fehlen:
/// `"0,00"` ist gueltig, ein Jahr 0 (`"01.01.0000"`) nicht (`bereich` beim Speichern ungeprueft).
///
/// JE VERANLAGUNGSJAHR, seit 2026-10-01 (`est_mapping.py:167-191`). Vorher war die Menge flach
/// fuer alle Jahre. `E10-2024.xsd` verbietet die 0 zusaetzlich in E0106603 (Anzahl weiterer
/// Pflegepersonen), waehrend `E10-2025.xsd` sie dort erlaubt und das XSD-Label sie sogar verlangt
/// (`bindung_rentner.yaml`). Beide Mengen sind aus `est_mapping.py` uebernommen, dazu ein Kz nur in Rust (E0108701,
/// Parteispenden, Abweichung Nr. 31): 2024 hat 37 Kz, 2025 hat 36 und ist Teilmenge von 2024 — der Unterschied ist genau
/// E0106603.
///
/// ponytail: eingefroren am 2026-10-01, je Jahr, damit `deklariere` ohne XSD laeuft und in jeder
/// Umgebung dieselbe Deklaration ergibt. Die Jahresliste waechst nur, wenn ein Jahres-XSD
/// dazukommt. Drift faengt `kz_mengen_aus_xsd` (`tests/eigenschaften.rs`): jede Kz beider Mengen
/// gegen das Schema ihres Jahres. Die Gegenrichtung (keine verbietende Kz fehlt) leiten
/// `null_bleibt_aus_kz_deren_xsd_typ_sie_verbietet` und fuer die Art-Verzweigung
/// `art_verzweigung_schreibt_keine_verbotene_null` (`deklaration.rs`) live aus dem XSD 2025 ab.
/// Fuer 2024 prueft sie niemand; Upgrade: beide Tests ueber die Jahresliste laufen lassen.
const NULL_UNZULAESSIG_KZ_2024: &[&str] = &[
    "E0104108", "E0104109", "E0106603", "E0107207", "E0107208", "E0108002", "E0108105", "E0108202",
    "E0108701", "E0109708", "E0111214", "E0111215", "E0161606", "E0203503", "E0203504", "E0205201",
    "E0205302", "E0205409", "E0207611", "E0240801", "E0240802", "E0241001", "E0241101", "E0241201",
    "E0241301", "E0241302", "E0241401", "E0241501", "E0241601", "E0241701", "E0241901", "E0242001",
    "E0305201", "E0505809", "E0506104", "E0506105", "E0801705",
];

/// Die Jahresmenge 2025 — siehe [`NULL_UNZULAESSIG_KZ_2024`]. Teilmenge von 2024.
const NULL_UNZULAESSIG_KZ_2025: &[&str] = &[
    "E0104108", "E0104109", "E0107207", "E0107208", "E0108002", "E0108105", "E0108202", "E0108701",
    "E0109708", "E0111214", "E0111215", "E0161606", "E0203503", "E0203504", "E0205201", "E0205302",
    "E0205409", "E0207611", "E0240801", "E0240802", "E0241001", "E0241101", "E0241201", "E0241301",
    "E0241302", "E0241401", "E0241501", "E0241601", "E0241701", "E0241901", "E0242001", "E0305201",
    "E0505809", "E0506104", "E0506105", "E0801705",
];

/// Die Vereinigung aller bekannten Jahresmengen (`est_mapping.py:193-195`). Wer sie liest, statt
/// eines Jahres, ist absichtlich zu streng: die Vereinigung sendet nie eine verbotene 0.
pub const NULL_UNZULAESSIG_KZ_VEREINIGUNG: &[&str] = NULL_UNZULAESSIG_KZ_2024;

/// Kz, in die `schreibe_kz` keine 0 schreibt, fuer dieses Veranlagungsjahr — die Regel hat vier
/// Faelle (`est_mapping.py:198-229`, `null_unzulaessig`).
///
/// 1. `vz` fehlt, ist `0` oder keine ganze Zahl → **Fehler**. `store.get("veranlagungszeitraum")
///    or 0` in `haut/api.py` liefert genau diese 0, wenn das Feld fehlt — sie darf nicht in eine
///    Jahresmenge greifen.
/// 2. `vz` ist bekannt (2024, 2025) → **seine eigene Menge**. Belegfall E0106603: 2024 verbietet
///    die 0, 2025 erlaubt sie.
/// 3. `vz` ist unbekannt aber plausibel (2026 bis 2100) → **die Vereinigung aller bekannten
///    Jahre**. Das ist KEIN stiller Rueckfall, sondern eine benannte Regel: die Vereinigung sendet
///    nie eine verbotene 0. Der Preis ist eine moeglicherweise erlaubte 0, die wir weglassen — und
///    weil 0 in diesen Kz „nichts anzugeben" heisst (`minOccurs 0`), ist der Verlust null. Das
///    Produkt rechnet VZ 2026 (`params/2026` ist vollstaendig); eine 2026er Erklaerung laeuft
///    heute bis ERiC und bekommt dort 610001042. Ein harter Fehler hier waere ein Rueckschritt auf
///    einem Pfad, der funktioniert.
/// 4. `vz` ist unplausibel (unter 2024, oder ueber 2100 wie die 10^38 eines Korpusfalls) →
///    **Fehler**. Sonst ginge 10^38 als „spaeter als 2025" durch und bekaeme die Vereinigung,
///    obwohl es kein Steuerjahr ist — eine stille Vorgabe durch die Hintertuer.
///
/// PARITÄT: Python prueft `isinstance(vz, bool)` gesondert, weil `True == 1` und `bool` ein
/// `int`-Untertyp ist. Hier traegt [`crate::deklariere`] das Jahr als `i64`, ein `bool` ist also
/// gar nicht darstellbar — die Pruefung entfaellt, statt sie zu vergessen.
///
/// # Errors
/// [`PyFehler`] mit Klasse `ValueError`, genau wo `est_mapping.py` wirft: fehlendes/0-Jahr und
/// unplausibles Jahr. Der Korpusfall `eg_huge.json` (vz 10^38−1) und `eg_neg.json` (vz −5)
/// erreichen diesen Fehler ueber `deklariere`, sobald das Jahr aus dem Fall kommt.
pub fn null_unzulaessig(vz: i64) -> Result<&'static [&'static str], PyFehler> {
    if vz == 0 {
        return Err(PyFehler {
            klasse: "ValueError",
            nachricht: format!(
                "Veranlagungsjahr fehlt oder ist 0: {vz}. deklariere() braucht das Jahr, weil die \
                 Null-Verbots-Liste je Jahr gilt. Fehlt das Feld 'veranlagungszeitraum' im Store?"
            ),
        });
    }
    match vz {
        2024 => Ok(NULL_UNZULAESSIG_KZ_2024),
        2025 => Ok(NULL_UNZULAESSIG_KZ_2025),
        2026..=2100 => Ok(NULL_UNZULAESSIG_KZ_VEREINIGUNG),
        _ => Err(PyFehler {
            klasse: "ValueError",
            nachricht: format!(
                "Veranlagungsjahr {vz} ist kein Steuerjahr (erwartet 2024..2100). deklariere() \
                 lehnt es ab, statt still eine Menge zu waehlen."
            ),
        }),
    }
}

/// Datums-Kz, in die ein Jahres-Wert als `01.01.JJJJ` geschrieben wird (`est_mapping.py:199`).
/// Aus dem XSD abgeleitet (Datums-Typ ∩ Kz-Literale des Moduls).
pub const DATUMS_KZ: &[&str] = &["E1800501", "E1801701", "E1803202"];

/// Wie ein Cent-Betrag in eine Kz geschrieben wird. Ein Kz hat genau ein Format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KzFormat {
    /// Einnahmen/Einkuenfte: auf ganze Euro abrunden (Richtung −∞, Python `//`).
    EuroAbgerundet,
    /// Abzuege/Aufwendungen/Verluste: auf ganze Euro aufrunden.
    EuroAufgerundet,
    /// Dezimal-Kz: Cent exakt als `"N,NN"`.
    KommaCent,
}

/// Das Format einer Kz (`_cent_nach_kz`-Weiche, `est_mapping.py:179-183`).
///
/// ```
/// use elster::{kz_format, KzFormat};
/// use domain::Kz;
/// let kz = |s| Kz::new(s).unwrap();
/// assert_eq!(kz_format(&kz("E6004901")), KzFormat::KommaCent);
/// assert_eq!(kz_format(&kz("E0200301")), KzFormat::KommaCent);
/// assert_eq!(kz_format(&kz("E0705701")), KzFormat::EuroAufgerundet);
/// assert_eq!(kz_format(&kz("E0200201")), KzFormat::EuroAbgerundet);
/// ```
#[must_use]
pub fn kz_format(kz: &Kz) -> KzFormat {
    if kz.hat_e60_praefix() || KOMMA_OHNE_E60_KZ.contains(&kz.as_str()) {
        KzFormat::KommaCent
    } else if ABZUGS_KZ.contains(&kz.as_str()) {
        KzFormat::EuroAufgerundet
    } else {
        KzFormat::EuroAbgerundet
    }
}

/// Ein in eine Kz geschriebener Geldbetrag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KzBetrag {
    /// Ganze Euro (XSD-Ganzzahl-Typen).
    Euro(Euro),
    /// Cent-genau, als `"N,NN"` geschrieben.
    Komma(Cent),
}

impl KzBetrag {
    /// Der Wert, wie er in der Deklaration steht (JSON-Zahl bzw. -Text).
    ///
    /// ```
    /// use domain::{Cent, Euro};
    /// use elster::KzBetrag;
    /// assert_eq!(KzBetrag::Euro(Euro::new(3)).als_json(), serde_json::json!(3));
    /// assert_eq!(KzBetrag::Komma(Cent::new(250)).als_json(), serde_json::json!("2,50"));
    /// ```
    #[must_use]
    pub fn als_json(&self) -> Value {
        match self {
            Self::Euro(e) => Value::from(e.get()),
            Self::Komma(c) => Value::String(komma_text(*c)),
        }
    }
}

/// `f"{wert // 100},{wert % 100:02d}"`.
///
/// PARITÄT: P1 — fuer negative Betraege liefert das `"-2,50"` fuer −150 Cent (Python `//` rundet
/// gegen −∞, `%` ist nicht-negativ), obwohl −1,50 € gemeint sind. Korrektur erst nach Paritaet in
/// eigenem Commit (`REWRITE_PLAN.md` §4, F1).
fn komma_text(c: Cent) -> String {
    let euro = c.floor_euro().get();
    let rest = c.get().rem_euclid(100);
    format!("{euro},{rest:02}")
}

/// Store-Cent → Kz-Betrag (`_cent_nach_kz`, `est_mapping.py:175-183`).
///
/// ```
/// use domain::{Cent, Euro, Kz};
/// use elster::{cent_nach_kz, KzBetrag};
/// let kz = |s| Kz::new(s).unwrap();
/// // Einnahme: abrunden
/// assert_eq!(cent_nach_kz(Cent::new(199), &kz("E0200201")), KzBetrag::Euro(Euro::new(1)));
/// // Abzug: aufrunden
/// assert_eq!(cent_nach_kz(Cent::new(101), &kz("E0705701")), KzBetrag::Euro(Euro::new(2)));
/// // Dezimal-Kz: exakt; P1 bei negativen Betraegen
/// assert_eq!(cent_nach_kz(Cent::new(-150), &kz("E6004901")).als_json(), serde_json::json!("-2,50"));
/// ```
#[must_use]
pub fn cent_nach_kz(cent: Cent, kz: &Kz) -> KzBetrag {
    match kz_format(kz) {
        KzFormat::KommaCent => KzBetrag::Komma(cent),
        KzFormat::EuroAufgerundet => KzBetrag::Euro(cent.ceil_euro()),
        KzFormat::EuroAbgerundet => KzBetrag::Euro(cent.floor_euro()),
    }
}

/// Store-Wert → Kz-Wert (`_kz_wert`, `est_mapping.py:216-228`): `cent` gerundet, Jahr in einer
/// Datums-Kz als `01.01.JJJJ`, alles andere unveraendert.
///
/// Der 01.01. ist eine Annahme, keine Kenntnis: der Nutzer nennt das Jahr des Rentenbeginns.
///
/// # Errors
/// [`PyFehler`] `TypeError`, wenn ein `cent`-Wert keine ganze Zahl ist. PARITÄT: Python rechnet
/// eine Gleitkommazahl dort still weiter (Ergebnis `float`); Auflage T des Stores laesst sie nicht
/// zu, der Port weist sie ab.
///
/// ```
/// use domain::{Feldtyp, Kz};
/// use elster::kz_wert;
/// use serde_json::json;
/// let kz = |s| Kz::new(s).unwrap();
/// assert_eq!(kz_wert(&json!(2015), &kz("E1800501"), Some(Feldtyp::Int)).unwrap(), json!("01.01.2015"));
/// assert_eq!(kz_wert(&json!(12345), &kz("E0200201"), Some(Feldtyp::Cent)).unwrap(), json!(123));
/// assert_eq!(kz_wert(&json!("x"), &kz("E0100201"), Some(Feldtyp::Text)).unwrap(), json!("x"));
/// ```
pub fn kz_wert(wert: &Value, kz: &Kz, typ: Option<Feldtyp>) -> Result<Value, PyFehler> {
    if typ == Some(Feldtyp::Cent) {
        let cent = match wert {
            Value::Bool(b) => i64::from(*b),
            Value::Number(n) => n
                .as_i64()
                .ok_or_else(|| PyFehler::typ("cent-Wert ist keine ganze Zahl"))?,
            _ => return Err(PyFehler::typ("unsupported operand type(s) for //")),
        };
        return Ok(cent_nach_kz(Cent::new(cent), kz).als_json());
    }
    if DATUMS_KZ.contains(&kz.as_str()) {
        if let Value::Number(n) = wert {
            if let Some(jahr) = n.as_i64() {
                return Ok(Value::String(format!("01.01.{jahr:04}")));
            }
        }
    }
    Ok(wert.clone())
}

/// Prueft eine Kz aus einer Tabelle oder aus der Deklaration, bevor sie in [`kz_wert`] geht.
/// Die Tabellen in `tabellen.rs`/`kz_format.rs` halten `&str`; diese eine Stelle macht daraus
/// eine [`Kz`]. Ein Tippfehler in einer Tabelle (`E06004901`) wird so zum Fehler statt zu `2`.
///
/// # Errors
/// [`PyFehler`] `ValueError`, wenn `kz` nicht `^E[0-9]{7}$` ist.
pub(crate) fn kz_pruefen(kz: &str) -> Result<Kz, PyFehler> {
    Kz::new(kz).map_err(|e| PyFehler {
        klasse: "ValueError",
        nachricht: e.to_string(),
    })
}

/// Die eine Schreibstelle fuer Kz-Werte in `deklariere` (`_schreibe_kz`, `est_mapping.py:288-302`):
/// `ziel[kz] = kz_wert(…)`, ausser der Kz-Wert ist 0 und die Kz verbietet die 0
/// (`null_kz`, die Menge DIESES Veranlagungsjahres). Erst umrechnen, dann pruefen: 1-99 Cent
/// werden in einer abgerundeten Kz zur 0 und entfallen ebenso.
///
/// `null_kz` kommt aus [`deklariere`](crate::deklariere) und wird durchgereicht, damit an jeder
/// Schreibstelle dieselbe Jahresmenge gilt — kein Modulzustand.
///
/// Kein `nicht_deklariert`-Eintrag: eine 0 ist kein verlorener Wert, sondern „nichts anzugeben".
/// Mit Eintrag meldete die Pruefanzeige bei jeder weggelassenen 0 „nicht alle Werte".
///
/// # Errors
/// Wie [`kz_wert`], dazu `OverflowError`/`ValueError`, wenn der Store-Wert nicht nach JSON geht
/// (NaN/+-inf; im Bestand gemessen 0 von 11294 Events).
pub(crate) fn schreibe_kz(
    ziel: &mut BTreeMap<String, Value>,
    kz: &str,
    wert: &domain::PyWert,
    typ: Option<Feldtyp>,
    null_kz: &[&str],
) -> Result<(), PyFehler> {
    // DIE Grenze Store -> Deklaration: einmal konvertieren, danach laeuft die unveraenderte
    // Value-Kette (`kz_wert`, `gleich_null`) -- die Deklaration IST JSON.
    let wert = wert.zu_json().map_err(PyFehler::from)?;
    let v = kz_wert(&wert, &kz_pruefen(kz)?, typ)?;
    if null_kz.contains(&kz) && py::gleich_null(&v) {
        return Ok(());
    }
    ziel.insert(kz.to_owned(), v);
    Ok(())
}

/// Umkehrung zu [`kz_wert`] fuer Datums-Kz (`_jahr_aus_kz_wert`, `est_mapping.py:201-212`):
/// `"TT.MM.JJJJ"` → Jahr, sonst unveraendert.
///
/// ```
/// use elster::jahr_aus_kz_wert;
/// use serde_json::json;
/// use domain::Kz;
/// let kz = |s| Kz::new(s).unwrap();
/// assert_eq!(jahr_aus_kz_wert(&json!(" 01.01.2015 "), &kz("E1800501")), json!(2015));
/// assert_eq!(jahr_aus_kz_wert(&json!("2015"), &kz("E1800501")), json!("2015"));
/// assert_eq!(jahr_aus_kz_wert(&json!("01.01.2015"), &kz("E0200201")), json!("01.01.2015"));
/// ```
#[must_use]
pub fn jahr_aus_kz_wert(wert: &Value, kz: &Kz) -> Value {
    if !DATUMS_KZ.contains(&kz.as_str()) {
        return wert.clone();
    }
    let Value::String(s) = wert else {
        return wert.clone();
    };
    // ponytail: `\d` ist in Python jede Unicode-Ziffer; hier nur ASCII (Store-Werte sind ASCII).
    match py::strip(s).as_bytes() {
        [t0, t1, b'.', m0, m1, b'.', j @ ..]
            if j.len() == 4
                && [t0, t1, m0, m1].iter().all(|b| b.is_ascii_digit())
                && j.iter().all(u8::is_ascii_digit) =>
        {
            let jahr = j
                .iter()
                .fold(0_i64, |acc, b| acc * 10 + i64::from(b - b'0'));
            Value::from(jahr)
        }
        _ => wert.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        cent_nach_kz, kz_format, kz_pruefen, schreibe_kz, KzBetrag, KzFormat, ABZUGS_KZ,
        DATUMS_KZ, KOMMA_OHNE_E60_KZ, NULL_UNZULAESSIG_KZ_2024, NULL_UNZULAESSIG_KZ_2025,
    };
    use crate::tabellen::{DOKUMENTIERT_AGGREGAT, P23_GEWINN_KZ, VERZWEIGUNG};
    use domain::{Cent, Euro, Feldtyp, Kz, PyWert};
    use proptest::prelude::*;
    use std::collections::BTreeMap;

    fn kz(s: &str) -> Kz {
        Kz::new(s).unwrap()
    }

    proptest! {
        /// Rundung „zu Ihren Gunsten": eine Einnahme wird nie hoeher, ein Abzug nie niedriger
        /// deklariert als der Cent-Betrag; beide weichen um weniger als einen Euro ab.
        #[test]
        fn rundung_zugunsten_der_steuerpflichtigen(c in -10_000_000_000_i64..10_000_000_000) {
            let KzBetrag::Euro(ein) = cent_nach_kz(Cent::new(c), &kz("E0200201")) else { panic!() };
            prop_assert!(ein.get() * 100 <= c && c - ein.get() * 100 < 100);
            let KzBetrag::Euro(ab) = cent_nach_kz(Cent::new(c), &kz("E0705701")) else { panic!() };
            prop_assert!(ab.get() * 100 >= c && ab.get() * 100 - c < 100);
        }
    }

    #[test]
    fn jedes_abzugs_kz_rundet_auf() {
        for s in ABZUGS_KZ {
            let k = kz(s);
            assert_eq!(kz_format(&k), KzFormat::EuroAufgerundet, "{s}");
            assert_eq!(cent_nach_kz(Cent::new(1), &k), KzBetrag::Euro(Euro::new(1)));
        }
    }

    /// Die Tabellen halten `&str`; `schreibe_kz` macht daraus eine `Kz`. Ein Tippfehler in einer
    /// Tabelle waere sonst ein stilles falsches Format.
    #[test]
    fn jede_tabellen_kz_ist_eine_gueltige_kz() {
        for tabelle in [
            ABZUGS_KZ,
            KOMMA_OHNE_E60_KZ,
            NULL_UNZULAESSIG_KZ_2024,
            NULL_UNZULAESSIG_KZ_2025,
            DATUMS_KZ,
        ] {
            for s in tabelle {
                assert!(Kz::ist_gueltig(s), "{s:?}");
            }
        }
        // Die Tabellen, aus denen `deklariere` eine Kz in `schreibe_kz`/`cent_nach_kz` reicht.
        for (ziel, _) in DOKUMENTIERT_AGGREGAT {
            assert!(Kz::ist_gueltig(ziel), "{ziel:?}");
        }
        for (_, s) in P23_GEWINN_KZ {
            assert!(Kz::ist_gueltig(s), "{s:?}");
        }
        for v in VERZWEIGUNG {
            for (_, s) in v.kz.paare() {
                assert!(Kz::ist_gueltig(s), "{}: {s:?}", v.feld);
            }
        }
    }

    /// Der Tippfehler aus der K9-Sonde: `E06004901` (neun Stellen) landete als Euro-Betrag `2` statt
    /// `2,50` im XML. `schreibe_kz` ist die eine Schreibstelle und lehnt ihn ab.
    #[test]
    fn schreibe_kz_lehnt_eine_ungueltige_kz_ab() {
        let mut ziel = BTreeMap::new();
        let wert = PyWert::Ganz(250);
        let fehler = schreibe_kz(&mut ziel, "E06004901", &wert, Some(Feldtyp::Cent), &[]).unwrap_err();
        assert_eq!(fehler.klasse, "ValueError");
        assert!(ziel.is_empty());
        schreibe_kz(&mut ziel, "E6004901", &wert, Some(Feldtyp::Cent), &[]).unwrap();
        assert_eq!(ziel["E6004901"], serde_json::json!("2,50"));
        assert!(kz_pruefen("E6004901").is_ok());
    }
}
