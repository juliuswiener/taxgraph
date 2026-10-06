//! § 32 Abs. 6 EStG je Kind: wie viele Monate und wie viele Elternteile den Kinderfreibetrag tragen.
//!
//! Vor diesem Modul rechnete der Rahmen `Kinderzahl * (zusammen ? 2 : 1) * Betrag` und las weder das
//! Kindschaftsverhaeltnis noch den Zeitraum je Kind. Julius hat am 2026-10-03 entschieden, den Freibetrag je
//! Kind zu rechnen, und am 2026-10-04 den Bau freigegeben ("c: A"); nur in Rust, Python bleibt unveraendert
//! und rechnet weiter Kinderzahl mal Betrag.
//!
//! Das Gesetz (`sources/gesetze-im-internet/estg_p32_2026-07-11.txt`, Abs. 6) ist Quelle jeder Regel:
//!
//! - Satz 1: Der Betrag steht "fuer jedes zu beruecksichtigende Kind des Steuerpflichtigen" zu.
//! - Satz 2: "Bei Ehegatten, die nach den §§ 26, 26b zusammen zur Einkommensteuer veranlagt werden, verdoppeln
//!   sich die Betraege nach Satz 1, wenn das Kind zu beiden Ehegatten in einem Kindschaftsverhaeltnis steht."
//! - Satz 3 Nr. 1 (gebaut 2026-10-06, C2, Julius: "bauen"): "Die Betraege nach Satz 2 stehen dem Steuerpflichtigen
//!   auch dann zu, wenn 1. der andere Elternteil verstorben oder nicht unbeschraenkt einkommensteuerpflichtig ist".
//!   Bei Einzelveranlagung zaehlt in den betroffenen Monaten das zweite Zwoelftel des anderen Elternteils mit
//!   ([`satz_3_monate`]); das Kindergeld bleibt bei den Monaten des Kindes (Anleitung zur Anlage Kind 2025, Z. 28-33:
//!   wer den vollen Freibetrag hat, hat auch den vollen Anspruch auf Kindergeld -- und der bemiessen sich nicht nach
//!   Satz 3).
//! - Satz 5: "Fuer jeden Kalendermonat, in dem die Voraussetzungen fuer einen Freibetrag nach den Saetzen 1 bis 4
//!   nicht vorliegen, ermaessigen sich die dort genannten Betraege um ein Zwoelftel."
//!
//! Gerechnet wird in Personenmonaten: je Kind, je Kalendermonat, je Ehegatte mit Kindschaftsverhaeltnis EIN Zwoelftel
//! des Betrags aus Satz 1. Der Normalfall (ganzes Jahr, leiblich zu beiden) sind 24 Personenmonate bei
//! Zusammenveranlagung und 12 sonst, also genau der alte Rechenweg.
//!
//! Das Kindergeld folgt denselben Monaten (§ 31 Satz 4: "den Anspruch auf Kindergeld fuer den gesamten
//! Veranlagungszeitraum"). Ohne das stuende ein halbes Jahr Freibetrag neben zwoelf Monaten Kindergeld.
//!
//! Was dieser Rahmen NICHT entscheiden kann, sperrt die Berechnung ([`KindSperre`]), statt zu raten:
//! - Ein Zeitraum, der sich nicht als `TT.MM-TT.MM` im Steuerjahr lesen laesst, und ein Todestag, der kein
//!   `TT.MM.JJJJ` ist.
//! - Stiefkind oder Enkelkind (Verhaeltnis 3): ein Kindschaftsverhaeltnis im Sinn von Abs. 1 ist es nicht; Freibetraege
//!   gibt es nur durch eine Uebertragung (Satz 10), die Antrag und Haushalt voraussetzt. Nach beidem fragt die Software nicht.
//!   Auch ein Todestag aendert daran nichts: Satz 3 setzt ein Kindschaftsverhaeltnis nach Satz 1 voraus.
//! - Zusammenveranlagung, aber verschiedene Zeitraeume der beiden Ehegatten: in den Monaten mit nur einem Elternteil
//!   haengt der Betrag an Satz 3 Nr. 2 (Alleinadoption, Pflegekind nur zu einem), wonach die Software nicht fragt.
//!   Die beiden Felder des Satzes 3 Nr. 1 (Tod, Ausland) werden bei Zusammenveranlagung nicht gelesen: beide
//!   Ehegatten haben die vollen Betraege schon nach Satz 2, und das Kennzeichenfeld der amtlichen Anlage Kind
//!   (Weit_Ang) gilt nur fuer nicht zusammen veranlagte Eltern.
//!
//! Von Satz 3 nur Nr. 1, nicht Nr. 2 (allein angenommen, Pflegekind nur zu einem Ehegatten): die amtliche XSD hat
//! dafuer kein Kennzeichen, und `kind_kindschaftsverhaeltnis_a` (1 = leibliches Kind oder Adoptivkind) sagt nicht, ob
//! der andere Elternteil das Kind mitgetragen hat. Auch nicht gebaut: E0501513 ("Wohnsitz oder Aufenthalt des anderen
//! Elternteils ist nicht zu ermitteln oder der Vater ist amtlich nicht feststellbar") -- die Anleitung zaehlt es zu den
//! vollen Freibetraegen (Z. 112-115), § 32 Abs. 6 Satz 3 nennt es nicht; ohne Frage nach dem Grund der Unknown-Situation
//! waere die Antwort ausserdem ein Ratespiel.
//!
//! Nicht gebaut, bewusst: Satz 4 (Kind ohne Inlandswohnsitz, Verhaeltnisse des Wohnsitzstaates), Saetze 6 bis 11
//! (Uebertragung auf den anderen Elternteil, Stief- und Grosseltern), Saetze 12 bis 14 (Identifizierung des Kindes ueber
//! die IdNr, Feld `kind_idnr`: die neun Bedingungsfelder `kind_durch_idnr_identifiziert` bewegen die Zahl weiter nicht),
//! Abs. 3 bis 5 (Alter; das nennt der Nutzer ueber `fam_anzahl_kinder`: "zaehle nur Kinder, fuer die du Anspruch hattest").
//!
//! ponytail: ein Monat zaehlt, wenn die Voraussetzung an mindestens einem Tag besteht (Satz 5 sagt "in dem die
//! Voraussetzungen ... nicht vorliegen"; ein Monat mit einem Tag ist keiner, in dem sie fehlen). Die Zwoelftel
//! werden auf volle Euro abgerundet (das Gesetz nennt keine Rundung; Abrunden kostet hoechstens 1 Euro Freibetrag).
//! Beides ist eine Lesart, kein Wortlaut.
use std::collections::BTreeMap;

use domain::{Euro, PyWert, Sperrgrund, Zustand};
use store::SnapshotFeld;

use super::rechnen::{mal_div, R};
use crate::{BescheidFehler, Instanzquelle};

/// Monate eines Kalenderjahres als Bitmaske: Bit 0 = Januar, Bit 11 = Dezember.
type Monate = u16;
const GANZES_JAHR: Monate = 0x0FFF;

const VERHAELTNIS_A: &str = "kind_kindschaftsverhaeltnis_a";
const VERHAELTNIS_B: &str = "kind_kindschaftsverhaeltnis_b";
const ZEITRAUM_A: &str = "kind_kindschaftsverh_zeitraum_a";
const ZEITRAUM_B: &str = "kind_kindschaftsverh_zeitraum_b";
/// Kz E0501102: Tag, an dem das Kindschaftsverhaeltnis zum anderen Elternteil durch dessen Tod erloschen ist.
const TOD_AM: &str = "kind_anderer_elternteil_tod_am";
/// Kz E0503903: Zeitraum, in dem der andere Elternteil im Ausland lebte. Die amtliche Anlage fragt nach dem
/// Auslandsaufenthalt, das Gesetz (§ 32 Abs. 6 Satz 3 Nr. 1) nach der fehlenden unbeschraenkten Steuerpflicht: die
/// Anleitung der Anlage Kind 2025 paart beide in einem Satz (Z. 110-112).
const AUSLAND_ZEITRAUM: &str = "kind_anderer_elternteil_ausland_zeitraum";

/// Warum die Berechnung des Kinderfreibetrags sperrt (Sperrgrund siehe [`KindSperre::grund`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KindSperre {
    /// Ein Zeitraum ist kein `TT.MM-TT.MM` im Steuerjahr, oder ein Todestag ist kein `TT.MM.JJJJ`.
    ZeitraumUnlesbar,
    /// Der Betrag haengt an Angaben, nach denen die Software nicht fragt (Satz 3, Satz 10).
    VerteilungOffen,
}

impl KindSperre {
    pub(crate) const fn grund(self) -> Sperrgrund {
        match self {
            Self::ZeitraumUnlesbar => Sperrgrund::KindZeitraumUnlesbar,
            Self::VerteilungOffen => Sperrgrund::KindFreibetragVerteilungOffen,
        }
    }
}

/// Die Summe ueber alle Kinder: Personenmonate (Freibetrag) und Kindergeld-Monate.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Summen {
    pub(crate) personenmonate: i64,
    pub(crate) kindergeld_monate: i64,
}

impl Summen {
    /// Der Normalfall eines Kindes: ganzes Jahr, leiblich zu allen Beteiligten.
    const fn normal(zusammen: bool) -> Self {
        Self {
            personenmonate: if zusammen { 24 } else { 12 },
            kindergeld_monate: 12,
        }
    }

    fn plus(self, andere: Self) -> R<Self> {
        let zu = |a: i64, b: i64| {
            a.checked_add(b)
                .ok_or(BescheidFehler::Ueberlauf("Kinder-Monate"))
        };
        Ok(Self {
            personenmonate: zu(self.personenmonate, andere.personenmonate)?,
            kindergeld_monate: zu(self.kindergeld_monate, andere.kindergeld_monate)?,
        })
    }

    fn mal(self, n: i64) -> R<Self> {
        let mal = |a: i64| {
            a.checked_mul(n)
                .ok_or(BescheidFehler::Ueberlauf("Kinder-Monate"))
        };
        Ok(Self {
            personenmonate: mal(self.personenmonate)?,
            kindergeld_monate: mal(self.kindergeld_monate)?,
        })
    }

    /// `(Freibetrag, Kindergeld)` in Euro: der Freibetrag je Elternteil mal Personenmonate durch zwoelf, auf volle
    /// Euro abgerundet; das Kindergeld je Monat mal Kindergeld-Monate.
    ///
    /// # Errors
    /// [`BescheidFehler::Ueberlauf`] jenseits von `i64`.
    pub(crate) fn betraege(self, je_elternteil: Euro, kindergeld_monat: Euro) -> R<(Euro, Euro)> {
        let freibetrag = mal_div(je_elternteil.get(), self.personenmonate, 12)?;
        let kindergeld = kindergeld_monat
            .get()
            .checked_mul(self.kindergeld_monate)
            .ok_or(BescheidFehler::Ueberlauf("Kindergeld-Monate"))?;
        Ok((Euro::new(freibetrag), Euro::new(kindergeld)))
    }
}

/// Ergebnis von [`auswerten`]: die Summen oder der Grund, nicht zu rechnen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Befund {
    Rechenbar(Summen),
    Gesperrt(KindSperre),
}

/// Die Kinder `1..=kinder` des Falls: Instanz `n` der Gruppe `kind` gehoert zum n-ten Kind; ein Kind ohne Instanz
/// und ein Kind ohne diese vier Angaben gilt als Normalfall (ganzes Jahr, leiblich zu allen Beteiligten) wie bisher.
/// Eine Instanz jenseits der Kinderzahl zaehlt nicht.
///
/// `q.nur_bestaetigt` waehlt die Sicht: streng liest nur bestaetigte Felder (festgesetzte Zahl), sonst alle.
///
/// PARITÄT: fail-open default — fehlende Angaben sind der Normalfall. Das ist der Rechenweg von heute und der
/// von Python; kein Eingabefeld ist Pflicht, und die 26 Akten mit Kindern tragen alle den Normalfall.
///
/// # Errors
/// [`BescheidFehler::BindungFehlt`], [`BescheidFehler::Snapshot`], [`BescheidFehler::Ueberlauf`].
pub(crate) fn auswerten(
    q: &Instanzquelle<'_>,
    kinder: i64,
    zusammen: bool,
    jahr: Option<u16>,
) -> R<Befund> {
    if kinder <= 0 {
        return Ok(Befund::Rechenbar(Summen::default()));
    }
    let mut summe = Summen::default();
    let mut mit_angaben = 0_i64;
    for inst in q.instanzen("kind")? {
        let Ok(index) = i64::try_from(inst.index) else {
            continue;
        };
        if index < 1 || index > kinder {
            continue;
        }
        match kind(&inst.felder, q.nur_bestaetigt, zusammen, jahr) {
            Ok(k) => summe = summe.plus(k)?,
            Err(sperre) => return Ok(Befund::Gesperrt(sperre)),
        }
        mit_angaben += 1;
    }
    let ohne_angaben = Summen::normal(zusammen).mal(kinder - mit_angaben)?;
    Ok(Befund::Rechenbar(summe.plus(ohne_angaben)?))
}

/// Ein Kind aus den Feldern seiner Instanz.
fn kind(
    felder: &BTreeMap<String, SnapshotFeld>,
    streng: bool,
    zusammen: bool,
    jahr: Option<u16>,
) -> Result<Summen, KindSperre> {
    let lies = |id: &str| {
        felder
            .get(id)
            .filter(|f| !streng || f.zustand == Zustand::Bestaetigt)
            .map(|f| &f.wert)
    };
    verhaeltnis(lies(VERHAELTNIS_A))?;
    let a = zeitraum(lies(ZEITRAUM_A), jahr)?;
    if !zusammen {
        // Satz 3 Nr. 1: je Monat, in dem der andere Elternteil verstorben ist oder im Ausland lebte und nicht
        // unbeschraenkt steuerpflichtig war, zaehlt auch sein Zwoelftel. Nur in den Monaten des Kindes (Satz 5).
        let zusatz = monate(a & satz_3_monate(lies(TOD_AM), lies(AUSLAND_ZEITRAUM), jahr)?);
        return Ok(Summen {
            personenmonate: monate(a) + zusatz,
            kindergeld_monate: monate(a),
        });
    }
    verhaeltnis(lies(VERHAELTNIS_B))?;
    let b = zeitraum(lies(ZEITRAUM_B), jahr)?;
    if a != b {
        return Err(KindSperre::VerteilungOffen);
    }
    Ok(Summen {
        personenmonate: 2 * monate(a),
        kindergeld_monate: monate(a),
    })
}

/// Zahl der gesetzten Monate.
fn monate(m: Monate) -> i64 {
    i64::from(m.count_ones())
}

/// Die Monate des § 32 Abs. 6 Satz 3 Nr. 1: der andere Elternteil ist verstorben ([`ab_todestag`]) oder er lebte im
/// Ausland und war nicht unbeschraenkt steuerpflichtig ([`ausland_monate`]). Beides zusammen ist die VEREINIGUNG,
/// nicht die Summe: ein Elternteil, der im Ausland verstorben ist, gibt einen Monat, nicht zwei.
fn satz_3_monate(
    tod: Option<&PyWert>,
    ausland: Option<&PyWert>,
    jahr: Option<u16>,
) -> Result<Monate, KindSperre> {
    Ok(ab_todestag(tod, jahr)? | ausland_monate(ausland, jahr)?)
}

/// Der Todestag `TT.MM.JJJJ` als Monatsmaske: ab dem Monat des Todes bis Dezember. Das Jahr des Datums entscheidet:
/// vor dem Steuerjahr das ganze Jahr, nach dem Steuerjahr kein Monat. Der Todesmonat zaehlt (derselbe angefangene
/// Monat wie bei Satz 5, ponytail oben). Der 29.02. gilt nur, wenn das JAHR DES DATUMS ein Schaltjahr ist -- nicht
/// das Steuerjahr. Kein Wert, `null` und leerer Text: kein Tod, also kein Monat.
fn ab_todestag(w: Option<&PyWert>, jahr: Option<u16>) -> Result<Monate, KindSperre> {
    let text = match w {
        None | Some(PyWert::Null) => return Ok(0),
        Some(PyWert::Text(t)) if t.is_empty() => return Ok(0),
        Some(PyWert::Text(t)) => t,
        Some(_) => return Err(KindSperre::ZeitraumUnlesbar),
    };
    // Genau drei Gruppen: das Muster des Feldes ist `TT.MM.JJJJ`, und der Store laesst nur das durch.
    let teile: Vec<&str> = text.split('.').collect();
    let [tag, monat, jahreszahl] = teile.as_slice() else {
        return Err(KindSperre::ZeitraumUnlesbar);
    };
    let (tag, monat) = (zwei_ziffern(tag)?, zwei_ziffern(monat)?);
    let totungsjahr = vier_ziffern(jahreszahl)?;
    if !(1..=12).contains(&monat) || tag < 1 || tag > tage_im_monat(Some(totungsjahr), monat) {
        return Err(KindSperre::ZeitraumUnlesbar);
    }
    let steuerjahr = jahr.ok_or(KindSperre::ZeitraumUnlesbar)?;
    if totungsjahr > steuerjahr {
        return Ok(0);
    }
    let ab = if totungsjahr < steuerjahr { 1 } else { monat };
    Ok(GANZES_JAHR & !((1 << (ab - 1)) - 1))
}

/// Der Ausland-Zeitraum `TT.MM-TT.MM` (derselbe Strengkalauf wie beim Kindschaftszeitraum). Kein Wert, `null` und
/// leerer Text sind keine Angabe und ergeben KEINEN Monat -- bewusst anders als [`zeitraum`], das leer als ganzes Jahr
/// liest: ein leeres Satz-3-Feld duerfte die Personenmonate nie still bis auf 24 erhoehen.
fn ausland_monate(w: Option<&PyWert>, jahr: Option<u16>) -> Result<Monate, KindSperre> {
    if w.is_none_or(|wert| matches!(wert, PyWert::Null) || matches!(wert, PyWert::Text(t) if t.is_empty())) {
        return Ok(0);
    }
    zeitraum(w, jahr)
}

/// Genau vier ASCII-Ziffern (das Jahr eines Todestags).
fn vier_ziffern(text: &str) -> Result<u16, KindSperre> {
    let b = text.as_bytes();
    if b.len() != 4 || !b.iter().all(u8::is_ascii_digit) {
        return Err(KindSperre::ZeitraumUnlesbar);
    }
    Ok(b.iter().fold(0_u16, |a, c| a * 10 + u16::from(c - b'0')))
}

/// Kindschaftsverhaeltnis im Sinn von § 32 Abs. 1: 1 = leiblich oder angenommen (Nr. 1), 2 = Pflegekind (Nr. 2).
/// Fehlt die Angabe, gilt der Normalfall. 3 (Enkel, Stief) und jeder andere Wert ist keines: der Betrag haengt dann
/// an einer Uebertragung (Satz 10).
fn verhaeltnis(w: Option<&PyWert>) -> Result<(), KindSperre> {
    let kindschaft = match w {
        None | Some(PyWert::Null) => true,
        Some(PyWert::Text(t)) => t.is_empty() || t == "1" || t == "2",
        Some(PyWert::Ganz(n)) => matches!(n, 1 | 2),
        Some(_) => false,
    };
    if kindschaft {
        Ok(())
    } else {
        Err(KindSperre::VerteilungOffen)
    }
}

/// Zeitraum `TT.MM-TT.MM` innerhalb des Steuerjahres als Monatsmaske. Kein Text, `null` und `""` sind keine Angabe
/// (ganzes Jahr). Alles andere, das sich nicht lesen laesst, ist [`KindSperre::ZeitraumUnlesbar`]: nie das ganze Jahr.
///
/// Gelesen wird streng: genau zwei Ziffern je Teil, ASCII, kein Leerraum. Ein Tag muss im Monat des Steuerjahres
/// vorkommen (29.02. nur im Schaltjahr); der Anfang liegt nicht nach dem Ende (kein Jahreswechsel).
fn zeitraum(w: Option<&PyWert>, jahr: Option<u16>) -> Result<Monate, KindSperre> {
    let text = match w {
        None | Some(PyWert::Null) => return Ok(GANZES_JAHR),
        Some(PyWert::Text(t)) if t.is_empty() => return Ok(GANZES_JAHR),
        Some(PyWert::Text(t)) => t,
        Some(_) => return Err(KindSperre::ZeitraumUnlesbar),
    };
    let (von, bis) = text.split_once('-').ok_or(KindSperre::ZeitraumUnlesbar)?;
    let (von, bis) = (tag_und_monat(von, jahr)?, tag_und_monat(bis, jahr)?);
    if von > bis {
        return Err(KindSperre::ZeitraumUnlesbar);
    }
    // Die Tupel sind (Monat, Tag): der Vergleich oben ordnet erst nach Monat, dann nach Tag.
    Ok((von.0..=bis.0).fold(0, |maske: Monate, m| maske | (1 << (m - 1))))
}

/// `TT.MM` als `(Monat, Tag)`; der Tag muss im Monat des Jahres vorkommen.
fn tag_und_monat(text: &str, jahr: Option<u16>) -> Result<(u8, u8), KindSperre> {
    let (tag, monat) = text.split_once('.').ok_or(KindSperre::ZeitraumUnlesbar)?;
    let (tag, monat) = (zwei_ziffern(tag)?, zwei_ziffern(monat)?);
    if !(1..=12).contains(&monat) || tag < 1 || tag > tage_im_monat(jahr, monat) {
        return Err(KindSperre::ZeitraumUnlesbar);
    }
    Ok((monat, tag))
}

/// Genau zwei ASCII-Ziffern.
fn zwei_ziffern(text: &str) -> Result<u8, KindSperre> {
    match text.as_bytes() {
        [a @ b'0'..=b'9', b @ b'0'..=b'9'] => Ok((a - b'0') * 10 + (b - b'0')),
        _ => Err(KindSperre::ZeitraumUnlesbar),
    }
}

/// Tage im Monat; ohne Jahr (`None`, Alt-Aufrufer ohne Veranlagungszeitraum) zaehlt der Februar als Schaltmonat.
fn tage_im_monat(jahr: Option<u16>, monat: u8) -> u8 {
    match monat {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if jahr.is_none_or(|j| j % 4 == 0 && (j % 100 != 0 || j % 400 == 0)) {
                29
            } else {
                28
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::testhilfe::{index, store};

    const J: Option<u16> = Some(2025);

    fn text(s: &str) -> PyWert {
        PyWert::Text(s.to_owned())
    }

    /// Die Felder der ersten Kind-Instanz aus einem echten Store; `(feld_id, wert, bestaetigt)`.
    fn instanz(events: &[(&str, Value, bool)]) -> BTreeMap<String, SnapshotFeld> {
        let st = store(events);
        let q = Instanzquelle {
            store: Some(&st),
            bindung: Some(index()),
            nur_bestaetigt: false,
        };
        q.instanzen("kind")
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .felder
    }

    fn je_zeitraum(a: &str, b: &str, bestaetigt: bool) -> BTreeMap<String, SnapshotFeld> {
        instanz(&[
            (ZEITRAUM_A, json!(a), bestaetigt),
            (ZEITRAUM_B, json!(b), bestaetigt),
        ])
    }

    /// Satz 5: jede Maske ist die Menge der Monate, in denen der Zeitraum mindestens einen Tag hat.
    #[test]
    fn zeitraum_liest_monatsmasken() {
        let z = |s: &str| zeitraum(Some(&text(s)), J);
        assert_eq!(z("01.01-31.12"), Ok(GANZES_JAHR));
        assert_eq!(z("01.07-31.12"), Ok(0b1111_1100_0000));
        assert_eq!(z("01.01-31.05"), Ok(0b1_1111));
        assert_eq!(
            z("15.03-31.03"),
            Ok(0b100),
            "ein angebrochener Monat zaehlt"
        );
        assert_eq!(
            z("31.01-01.02"),
            Ok(0b11),
            "zwei Tage in zwei Monaten sind zwei Monate"
        );
        assert_eq!(z("01.01-01.01"), Ok(1));
        assert_eq!(z("15.12-31.12"), Ok(1 << 11));
    }

    /// Keine Angabe ist das ganze Jahr: fehlend, `null` und leerer Text (die UI schreibt `""`).
    #[test]
    fn zeitraum_ohne_angabe_ist_das_ganze_jahr() {
        assert_eq!(zeitraum(None, J), Ok(GANZES_JAHR));
        assert_eq!(zeitraum(Some(&PyWert::Null), J), Ok(GANZES_JAHR));
        assert_eq!(zeitraum(Some(&text("")), J), Ok(GANZES_JAHR));
    }

    /// Was sich nicht eindeutig lesen laesst, sperrt: nie das ganze Jahr, nie geraten.
    #[test]
    fn zeitraum_unlesbares_sperrt() {
        let aus = Err(KindSperre::ZeitraumUnlesbar);
        for s in [
            "31.12-01.01",        // Anfang nach Ende: kein Jahreswechsel
            "02.01-01.01",        // gleicher Monat, Tag rueckwaerts
            "01.01",              // kein Bindestrich
            "01.01-",             // Ende fehlt
            "-31.12",             // Anfang fehlt
            "01.01-31.12-",       // Rest hinter dem Ende
            "1.1-31.12",          // einstellige Teile
            "001.01-31.12",       // dreistellig
            "01.01\u{2013}31.12", // Gedankenstrich statt Bindestrich
            " 01.01-31.12",       // Leerraum
            "01.01 -31.12",
            "01-01-31.12", // Punkt fehlt
            "+1.01-31.12", // Vorzeichen ist keine Ziffer
            "00.01-31.12", // Tag 0
            "32.01-31.12", // Tag 32
            "01.00-31.12", // Monat 0
            "01.13-31.12", // Monat 13
            "01.01-31.13",
            "31.04-30.04", // 31. April
            "31.06-30.06",
            "30.02-30.02", // 30. Februar, in keinem Jahr
            "0١.01-31.12", // Nicht-ASCII-Ziffer
            "0:.01-31.12", // ':' folgt '9' im ASCII und gaebe als Ziffer 10 den Tag 10
            "01.0:-31.12", // dasselbe im Monat (10)
        ] {
            assert_eq!(zeitraum(Some(&text(s)), J), aus, "{s:?}");
        }
        for w in [
            PyWert::Ganz(5),
            PyWert::Bool(true),
            PyWert::Gleit(1.5),
            PyWert::Liste(Vec::new()),
        ] {
            assert_eq!(zeitraum(Some(&w), J), aus, "{w:?}");
        }
    }

    /// Der 29.02. gilt nur im Schaltjahr; ohne Jahr (Alt-Aufrufer) zaehlt er.
    #[test]
    fn zeitraum_neunundzwanzigster_februar_nur_im_schaltjahr() {
        let z = |j: Option<u16>| zeitraum(Some(&text("29.02-29.02")), j);
        assert_eq!(z(Some(2024)), Ok(0b10));
        assert_eq!(z(Some(2000)), Ok(0b10), "durch 400 teilbar: Schaltjahr");
        assert_eq!(z(None), Ok(0b10));
        assert_eq!(z(Some(2025)), Err(KindSperre::ZeitraumUnlesbar));
        assert_eq!(
            z(Some(2100)),
            Err(KindSperre::ZeitraumUnlesbar),
            "durch 100, nicht durch 400"
        );
        assert_eq!(tage_im_monat(Some(2025), 2), 28);
        assert_eq!(tage_im_monat(Some(2024), 2), 29);
    }

    /// Die Monatslaengen: 31 Tage in sieben, 30 in vier Monaten.
    #[test]
    fn tage_im_monat_kennt_alle_zwoelf_monate() {
        let tage: Vec<u8> = (1..=12).map(|m| tage_im_monat(J, m)).collect();
        assert_eq!(tage, [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]);
        assert_eq!(tage.iter().map(|t| u32::from(*t)).sum::<u32>(), 365);
    }

    /// Verhaeltnis 1 und 2 (Abs. 1 Nr. 1 und 2) und jede fehlende Angabe tragen den Freibetrag; 3 und Unbekanntes nicht.
    #[test]
    fn verhaeltnis_nur_eins_und_zwei_tragen() {
        for ok in [
            None,
            Some(PyWert::Null),
            Some(text("")),
            Some(text("1")),
            Some(text("2")),
            Some(PyWert::Ganz(1)),
            Some(PyWert::Ganz(2)),
        ] {
            assert_eq!(verhaeltnis(ok.as_ref()), Ok(()), "{ok:?}");
        }
        for offen in [
            text("3"),
            text("0"),
            text("4"),
            text("leiblich"),
            text(" 1"),
            PyWert::Ganz(3),
            PyWert::Ganz(0),
            PyWert::Bool(true),
            PyWert::Gleit(1.0),
        ] {
            assert_eq!(
                verhaeltnis(Some(&offen)),
                Err(KindSperre::VerteilungOffen),
                "{offen:?}"
            );
        }
    }

    /// Einzelveranlagung: ein Elternteil, A bestimmt die Monate, B wird nicht gelesen.
    #[test]
    fn kind_einzel_liest_nur_person_a() {
        let f = je_zeitraum("01.04-31.12", "garbage", true);
        assert_eq!(
            kind(&f, false, false, J),
            Ok(Summen {
                personenmonate: 9,
                kindergeld_monate: 9
            })
        );
    }

    /// Zusammenveranlagung, gleiche Zeitraeume: zwei Elternteile je Monat, das Kindergeld einmal je Monat.
    #[test]
    fn kind_zusammen_gleicher_zeitraum_verdoppelt_nur_den_freibetrag() {
        let f = je_zeitraum("01.07-31.12", "01.07-31.12", true);
        assert_eq!(
            kind(&f, false, true, J),
            Ok(Summen {
                personenmonate: 12,
                kindergeld_monate: 6
            })
        );
    }

    /// Zusammenveranlagung, verschiedene Zeitraeume: Satz 3 / Satz 10, danach fragt die Software nicht.
    #[test]
    fn kind_zusammen_verschiedene_zeitraeume_sperren() {
        let f = je_zeitraum("01.01-31.12", "01.07-31.12", true);
        assert_eq!(kind(&f, false, true, J), Err(KindSperre::VerteilungOffen));
        // Dieselben Zeitraeume gelten bei Einzelveranlagung nichts: dort zaehlt nur A.
        assert!(kind(&f, false, false, J).is_ok());
    }

    /// Ein unlesbarer Zeitraum von B sperrt nur bei Zusammenveranlagung, einer von A immer.
    #[test]
    fn kind_unlesbarer_zeitraum_sperrt_je_nach_person() {
        let b_kaputt = je_zeitraum("01.01-31.12", "31.12-01.01", true);
        assert_eq!(
            kind(&b_kaputt, false, true, J),
            Err(KindSperre::ZeitraumUnlesbar)
        );
        assert!(kind(&b_kaputt, false, false, J).is_ok());
        let a_kaputt = je_zeitraum("31.12-01.01", "01.01-31.12", true);
        assert_eq!(
            kind(&a_kaputt, false, false, J),
            Err(KindSperre::ZeitraumUnlesbar)
        );
        assert_eq!(
            kind(&a_kaputt, false, true, J),
            Err(KindSperre::ZeitraumUnlesbar)
        );
    }

    /// Stiefkind bei A sperrt immer, bei B nur bei Zusammenveranlagung.
    #[test]
    fn kind_stiefkind_sperrt_je_nach_person() {
        let b3 = instanz(&[
            (VERHAELTNIS_A, json!("1"), true),
            (VERHAELTNIS_B, json!("3"), true),
        ]);
        assert_eq!(kind(&b3, false, true, J), Err(KindSperre::VerteilungOffen));
        assert!(kind(&b3, false, false, J).is_ok());
        let a3 = instanz(&[
            (VERHAELTNIS_A, json!("3"), true),
            (VERHAELTNIS_B, json!("1"), true),
        ]);
        assert_eq!(kind(&a3, false, false, J), Err(KindSperre::VerteilungOffen));
        assert_eq!(kind(&a3, false, true, J), Err(KindSperre::VerteilungOffen));
    }

    /// Die strenge Sicht sieht vorlaeufige Werte nicht (Zwei-Signal-Invariante): ein vorlaeufiger
    /// Teiljahr-Zeitraum bewegt die festgesetzte Zahl nicht, die rohe Sicht liest ihn.
    #[test]
    fn kind_strenge_sicht_ignoriert_vorlaeufiges() {
        let f = je_zeitraum("01.07-31.12", "01.07-31.12", false);
        assert_eq!(
            kind(&f, true, true, J),
            Ok(Summen {
                personenmonate: 24,
                kindergeld_monate: 12
            })
        );
        assert_eq!(
            kind(&f, false, true, J),
            Ok(Summen {
                personenmonate: 12,
                kindergeld_monate: 6
            })
        );
        let stief = instanz(&[(VERHAELTNIS_A, json!("3"), false)]);
        assert!(kind(&stief, true, false, J).is_ok());
        assert_eq!(
            kind(&stief, false, false, J),
            Err(KindSperre::VerteilungOffen)
        );
    }

    /// Die Zwoelftel runden auf volle Euro ab: 4.878 Euro (gerade fuer den Test gewaehlt, nicht im Gesetz) mal
    /// 7/12 sind 2.845,50 und werden 2.845, nicht 2.846. Das Kindergeld ist ganzzahlig.
    #[test]
    fn betraege_runden_den_freibetrag_ab() {
        let s = Summen {
            personenmonate: 7,
            kindergeld_monate: 7,
        };
        let rechne = |s: Summen, fb: i64| s.betraege(Euro::new(fb), Euro::new(255)).unwrap();
        assert_eq!(rechne(s, 4_878), (Euro::new(2_845), Euro::new(1_785)));
        // Normalfall: 24 Personenmonate sind zwei volle Betraege, 12 Kindergeldmonate zwoelf volle.
        assert_eq!(
            rechne(Summen::normal(true), 4_800),
            (Euro::new(9_600), Euro::new(3_060))
        );
        assert_eq!(
            rechne(Summen::normal(false), 4_800),
            (Euro::new(4_800), Euro::new(3_060))
        );
        assert_eq!(
            rechne(Summen::default(), 4_800),
            (Euro::new(0), Euro::new(0))
        );
    }

    /// Mehrere Kinder summieren sich; ein Ueberlauf wird ein Fehler, kein Wrap.
    #[test]
    fn summen_addieren_und_laufen_nicht_still_ueber() {
        let zwei = Summen::normal(true).mal(2).unwrap();
        assert_eq!(
            zwei,
            Summen {
                personenmonate: 48,
                kindergeld_monate: 24
            }
        );
        let drei = zwei.plus(Summen::normal(true)).unwrap();
        assert_eq!(
            drei,
            Summen {
                personenmonate: 72,
                kindergeld_monate: 36
            }
        );
        let riesig = Summen {
            personenmonate: i64::MAX,
            kindergeld_monate: 1,
        };
        assert!(matches!(
            riesig.plus(Summen::normal(false)),
            Err(BescheidFehler::Ueberlauf(_))
        ));
        assert!(matches!(riesig.mal(2), Err(BescheidFehler::Ueberlauf(_))));
        let teuer = Summen {
            personenmonate: 1,
            kindergeld_monate: i64::MAX,
        };
        assert!(matches!(
            teuer.betraege(Euro::new(1), Euro::new(2)),
            Err(BescheidFehler::Ueberlauf(_))
        ));
    }

    /// Sperrgrund je Fall: die zwei Gruende sind verschieden und tragen die Schluessel, die die Klartexte kennen.
    #[test]
    fn sperre_nennt_ihren_grund() {
        assert_eq!(
            KindSperre::ZeitraumUnlesbar.grund().als_str(),
            "kind_zeitraum_unlesbar"
        );
        assert_eq!(
            KindSperre::VerteilungOffen.grund().als_str(),
            "kind_freibetrag_verteilung_offen"
        );
    }

    /// Ohne Kinder zaehlt keine Instanz und keine Summe; ohne Angaben ist jedes Kind der Normalfall.
    #[test]
    fn auswerten_null_kinder_und_normalfall() {
        let st = store(&[]);
        let q = Instanzquelle {
            store: Some(&st),
            bindung: Some(index()),
            nur_bestaetigt: false,
        };
        assert_eq!(
            auswerten(&q, 0, true, J).unwrap(),
            Befund::Rechenbar(Summen::default())
        );
        assert_eq!(
            auswerten(&q, -3, true, J).unwrap(),
            Befund::Rechenbar(Summen::default())
        );
        assert_eq!(
            auswerten(&q, 3, true, J).unwrap(),
            Befund::Rechenbar(Summen {
                personenmonate: 72,
                kindergeld_monate: 36
            })
        );
        assert_eq!(
            auswerten(&q, 3, false, J).unwrap(),
            Befund::Rechenbar(Summen {
                personenmonate: 36,
                kindergeld_monate: 36
            })
        );
    }


    /// Die beiden Felder einer Kind-Instanz fuer Satz 3: Todestag und Ausland-Zeitraum.
    fn je_satz3(tod: Value, ausland: Value) -> BTreeMap<String, SnapshotFeld> {
        instanz(&[
            (TOD_AM, tod, true),
            (AUSLAND_ZEITRAUM, ausland, true),
        ])
    }

    /// Der Todestag als Maske: ab dem Monat des Todes bis Dezember, nach dem Kalender SEINES Jahres.
    #[test]
    fn todesmonat_und_satz_3_monate() {
        let ab = |s: &str| ab_todestag(Some(&text(s)), J);
        assert_eq!(ab("15.07.2025"), Ok(0b1111_1100_0000), "Tod im Juli: Juli bis Dezember");
        assert_eq!(ab("01.07.2025"), Ok(0b1111_1100_0000), "der erste Tag des Monats");
        assert_eq!(ab("31.07.2025"), Ok(0b1111_1100_0000), "der letzte Tag des Monats");
        assert_eq!(ab("30.06.2025"), Ok(0b1111_1110_0000), "Tod im Juni: Juni bis Dezember");
        assert_eq!(ab("01.01.2025"), Ok(GANZES_JAHR), "Tod am ersten Tag: das ganze Jahr");
        assert_eq!(ab("31.12.2025"), Ok(1 << 11), "Tod im Dezember: nur der Dezember");
        assert_eq!(ab("31.12.2024"), Ok(GANZES_JAHR), "Vorjahr: das ganze Steuerjahr");
        assert_eq!(ab("01.01.1900"), Ok(GANZES_JAHR), "weit in der Vergangenheit");
        assert_eq!(ab("01.01.2026"), Ok(0), "Tod nach dem Steuerjahr: kein Monat");
        assert_eq!(ab("31.12.2999"), Ok(0));
        assert_eq!(ab("15.03.0000"), Ok(GANZES_JAHR), "Jahr 0000 liegt vor dem Steuerjahr");
        // Der 29.02. gilt im Schaltjahr DES DATUMS, nicht des Steuerjahres (2025 hat keinen 29.02.).
        assert_eq!(
            ab_todestag(Some(&text("29.02.2024")), Some(2024)),
            Ok(0b1111_1111_1110),
            "Schaltjahr 2024 im Steuerjahr 2024: Februar bis Dezember"
        );
        assert_eq!(ab("29.02.2024"), Ok(GANZES_JAHR), "Tod 2024, Steuerjahr 2025: das ganze Jahr");
        assert_eq!(ab("29.02.2100"), Err(KindSperre::ZeitraumUnlesbar), "2100 ist kein Schaltjahr");
        assert_eq!(
            ab_todestag(Some(&text("29.02.2000")), Some(2000)),
            Ok(0b1111_1111_1110),
            "durch 400 teilbar: Schaltjahr"
        );
        for kaputt in [
            "31.02.2025", "31.04.2025", "00.03.2025", "15.00.2025", "15.13.2025", "1.3.2025", "15.03.25",
            "15.03.20255", "2025-03-15", "15/03/2025", " 15.03.2025", "15.03.2025 ", "15.03.2025\t",
            "15.03.", ".15.03.2025", "15.03.2025.1", "abc", "15.03.999", "1١.03.2025", "15.0١.2025",
            "15.03.20a5", "15.03.-205",
        ] {
            assert_eq!(ab(kaputt), Err(KindSperre::ZeitraumUnlesbar), "{kaputt:?}");
        }
        for w in [PyWert::Ganz(5), PyWert::Bool(true), PyWert::Gleit(1.5), PyWert::Liste(Vec::new())] {
            assert_eq!(ab_todestag(Some(&w), J), Err(KindSperre::ZeitraumUnlesbar), "{w:?}");
        }
        // Kein Wert, null, leerer Text: keine Angabe, kein Monat -- NIE das ganze Jahr.
        assert_eq!(ab_todestag(None, J), Ok(0));
        assert_eq!(ab_todestag(Some(&PyWert::Null), J), Ok(0));
        assert_eq!(ab_todestag(Some(&text("")), J), Ok(0));
        // Ohne Steuerjahr (Alt-Aufrufer mit `None`) ist der Fall nicht entscheidbar: Sperre, nicht geraten.
        assert_eq!(ab_todestag(Some(&text("15.07.2025")), None), Err(KindSperre::ZeitraumUnlesbar));
    }

    /// Der Ausland-Zeitraum: derselbe Leser wie beim Kindschaftszeitraum, aber leer ist KEIN Monat (nicht das
    /// ganze Jahr) -- sonst erhoebe ein leeres Feld still die Personenmonate bis auf die Obergrenze.
    #[test]
    fn ausland_monate_lesen_wie_der_kindschaftszeitraum_nur_leer_anders() {
        let a = |s: &str| ausland_monate(Some(&text(s)), J);
        assert_eq!(a("01.07-31.12"), Ok(0b1111_1100_0000));
        assert_eq!(a("15.03-31.03"), Ok(0b100), "ein angebrochener Monat zaehlt");
        assert_eq!(a("01.04-31.10"), Ok(0b11_1111_1000), "April bis Oktober sind sieben Monate");
        assert_eq!(a("01.12-31.12"), Ok(1 << 11));
        assert_eq!(a("01.01-01.01"), Ok(1));
        assert_eq!(ausland_monate(None, J), Ok(0), "fehlend ist kein Monat");
        assert_eq!(ausland_monate(Some(&PyWert::Null), J), Ok(0));
        assert_eq!(ausland_monate(Some(&text("")), J), Ok(0), "LEER ist kein Monat, nicht das ganze Jahr");
        assert_eq!(zeitraum(Some(&text("")), J), Ok(GANZES_JAHR), "beim Kindschaftszeitraum ist leer das ganze Jahr");
        for kaputt in ["31.02-31.12", "29.02-31.12", "01.12-01.01", "1.1-31.12", "01.13-31.12", "abc", "01.01"] {
            assert_eq!(a(kaputt), Err(KindSperre::ZeitraumUnlesbar), "{kaputt:?}");
        }
        assert_eq!(
            ausland_monate(Some(&PyWert::Ganz(2025)), J),
            Err(KindSperre::ZeitraumUnlesbar)
        );
    }

    /// Satz 3 Nr. 1: Tod und Ausland werden VEREINIGT, nicht addiert; ein Elternteil, der im Ausland verstorben
    /// ist, gibt einen Monat, nicht zwei.
    #[test]
    fn satz_3_vereinigt_statt_addiert() {
        let leer = Value::Null;
        let m = |tod: &str, ausland: &str| {
            let f = je_satz3(json!(tod), json!(ausland));
            satz_3_monate(
                f.get(TOD_AM).map(|x| &x.wert),
                f.get(AUSLAND_ZEITRAUM).map(|x| &x.wert),
                J,
            )
        };
        assert_eq!(m("15.07.2025", "01.01-31.12"), Ok(GANZES_JAHR), "Ausland ganzes Jahr, Tod im Juli");
        assert_eq!(m("01.04.2025", "01.03-30.06"), Ok(0b1111_1111_1100), "Maerz bis Dezember, 10 Monate");
        assert_eq!(
            m("01.09.2025", "01.03-31.05"),
            Ok(0b1111_0001_1100),
            "Maerz bis Mai und September bis Dezember"
        );
        assert_eq!(m("31.12.2024", "01.01-31.12"), Ok(GANZES_JAHR), "beides ganzes Jahr bleibt ein Jahr");
        assert_eq!(m("31.12.2025", "01.01-31.12"), Ok(GANZES_JAHR), "ein voller Ausland-Zeitraum deckt den Todesmonat ab");
        assert_eq!(m("15.07.2025", "01.07-31.12"), Ok(0b1111_1100_0000));
        let f = je_satz3(leer.clone(), leer.clone());
        assert_eq!(
            satz_3_monate(
                f.get(TOD_AM).map(|x| &x.wert),
                f.get(AUSLAND_ZEITRAUM).map(|x| &x.wert),
                J
            ),
            Ok(0),
            "zwei leere Felder sind zwei Antworten ohne Auswirkung"
        );
    }

    /// Einzelveranlagung mit Satz 3: der Ueberlebende traegt in den betroffenen Monaten beide Zwoelftel (Hoechstzahl
    /// 24 je Kind), das Kindergeld bleibt bei den Monaten des Kindes.
    #[test]
    fn kind_einzel_satz_3_gibt_beide_zwoelftel() {
        let f = |a: &str, tod: Value, ausland: Value| {
            let mut e = vec![
                (ZEITRAUM_A, json!(a), true),
                (TOD_AM, tod, true),
                (AUSLAND_ZEITRAUM, ausland, true),
            ];
            e.push((ZEITRAUM_B, json!("01.01-31.12"), true));
            let f = instanz(&e);
            kind(&f, false, false, J)
        };
        assert_eq!(
            f("01.01-31.12", json!("31.12.2024"), Value::Null),
            Ok(Summen {
                personenmonate: 24,
                kindergeld_monate: 12
            }),
            "Tod im Vorjahr: zwei Zwoelftel je Monat, Kindergeld einmal"
        );
        assert_eq!(
            f("01.01-31.12", Value::Null, json!("01.01-31.12")),
            Ok(Summen {
                personenmonate: 24,
                kindergeld_monate: 12
            }),
            "Ausland im ganzen Jahr: dasselbe"
        );
        assert_eq!(
            f("01.07-31.12", json!("01.03.2025"), Value::Null),
            Ok(Summen {
                personenmonate: 12,
                kindergeld_monate: 6
            }),
            "Kind ab Juli, Tod seit Maerz: nur die Monate des Kindes doppelt, nie 16"
        );
        assert_eq!(
            f("01.01-31.12", json!("15.07.2025"), Value::Null),
            Ok(Summen {
                personenmonate: 18,
                kindergeld_monate: 12
            }),
            "Tod im Juli: sechs Monate mit beiden Zwoelfteln"
        );
        assert_eq!(
            f("01.01-31.12", json!("01.01.2026"), Value::Null),
            Ok(Summen {
                personenmonate: 12,
                kindergeld_monate: 12
            }),
            "Tod nach dem Steuerjahr aendert nichts"
        );
        assert_eq!(
            f("01.01-31.12", Value::Null, json!("")),
            Ok(Summen {
                personenmonate: 12,
                kindergeld_monate: 12
            }),
            "leerer Ausland-Zeitraum ist kein Monat"
        );
    }

    /// Zusammenveranlagung liest die beiden Felder nicht (Satz 2 gibt die vollen Betraege schon, und das
    /// Kennzeichenfeld der Anlage Kind gilt nur fuer nicht zusammen veranlagte Eltern). Ein kaputter Wert sperrt
    /// dort deshalb auch nicht.
    #[test]
    fn kind_zusammen_liest_satz_3_nicht() {
        let f = instanz(&[
            (ZEITRAUM_A, json!("01.01-31.12"), true),
            (ZEITRAUM_B, json!("01.01-31.12"), true),
            (TOD_AM, json!("31.02.2025"), true),
            (AUSLAND_ZEITRAUM, json!("kaputt"), true),
        ]);
        assert_eq!(
            kind(&f, false, true, J),
            Ok(Summen {
                personenmonate: 24,
                kindergeld_monate: 12
            })
        );
    }

    /// Ein Stiefkind bleibt gesperrt, auch mit Todestag: Satz 3 setzt ein Kindschaftsverhaeltnis nach Satz 1 voraus.
    /// Ein unlesbarer Todestag oder Ausland-Zeitraum sperrt bei Einzel je nach Person (A-Zweig), nie mit "kein Tod".
    #[test]
    fn satz_3_sperrt_statt_zu_raten() {
        let f = instanz(&[
            (VERHAELTNIS_A, json!("3"), true),
            (TOD_AM, json!("31.12.2024"), true),
        ]);
        assert_eq!(kind(&f, false, false, J), Err(KindSperre::VerteilungOffen));
        for (id, wert) in [(TOD_AM, json!("31.02.2025")), (AUSLAND_ZEITRAUM, json!("01.13-31.12"))] {
            let f = instanz(&[(ZEITRAUM_A, json!("01.01-31.12"), true), (id, wert.clone(), true)]);
            assert_eq!(
                kind(&f, false, false, J),
                Err(KindSperre::ZeitraumUnlesbar),
                "{id}"
            );
            // Die strenge Sicht (festgesetzte Zahl) laesst den vorlaeufigen Wert weg: Normalfall, keine Sperre.
            let f = instanz(&[
                (ZEITRAUM_A, json!("01.01-31.12"), true),
                (id, wert, false),
            ]);
            assert_eq!(
                kind(&f, true, false, J),
                Ok(Summen {
                    personenmonate: 12,
                    kindergeld_monate: 12
                }),
                "{id} vorlaeufig"
            );
        }
    }

    /// `auswerten` ueber mehrere Instanzen: nur das Kind mit Angabe bekommt die Zusatzmonate, ein Kind ohne
    /// Angabe bleibt beim Normalfall von heute (12 bei Einzel), und eine Instanz jenseits der Kinderzahl zaehlt
    /// weder mit noch gegen die Zahl.
    #[test]
    fn auswerten_satz_3_je_instanz() {
        let mit_tod = &[
            (TOD_AM, json!("31.12.2024"), true),
            ("kind_anderer_elternteil_tod_am__2", json!("01.01.2026"), true),
        ];
        let q = Instanzquelle {
            store: Some(&store(mit_tod)),
            bindung: Some(index()),
            nur_bestaetigt: false,
        };
        assert_eq!(
            auswerten(&q, 2, false, J).unwrap(),
            Befund::Rechenbar(Summen {
                personenmonate: 36,
                kindergeld_monate: 24
            }),
            "Kind 1 zwei Zwoelftel, Kind 2 Tod erst 2026, Kind 3 ohne Angaben: 24 + 12"
        );
        assert_eq!(
            auswerten(&q, 1, false, J).unwrap(),
            Befund::Rechenbar(Summen {
                personenmonate: 24,
                kindergeld_monate: 12
            }),
            "nur Kind 1 gezaehlt: die Instanz 2 jenseits der Kinderzahl wirkt nicht"
        );
    }
}
