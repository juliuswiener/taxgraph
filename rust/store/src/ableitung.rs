//! Datums-Ableitungen fuer `_rechne_ab` (`store.py:447-500`, `_jahr`/`_monat_tag`/`_berechne`).
//! Reine Funktionen, keine Store-Abhaengigkeit — [`crate::store::Store`] ruft sie auf.
use bindung::{Ableitung, AbleitungArt};

/// Jahreszahl aus einem Datum — ISO (`JJJJ-MM-TT`) oder deutsch (`TT.MM.JJJJ`), sonst `None`
/// (`store.py:447-455`, `_jahr`). Formatpruefung nur ueber Ziffern-an-der-richtigen-Stelle, wie
/// im Original (kein `regex`, byte-Muster wie `domain::wert::ist_tt_mm_jjjj`).
#[must_use]
pub fn jahr_aus_datum(wert: &str) -> Option<i64> {
    let w = wert.trim();
    match w.as_bytes() {
        [j0, j1, j2, j3, b'-', m0, m1, b'-', t0, t1]
            if [j0, j1, j2, j3, m0, m1, t0, t1]
                .iter()
                .all(|b| b.is_ascii_digit()) =>
        {
            vierstellig(*j0, *j1, *j2, *j3)
        }
        [t0, t1, b'.', m0, m1, b'.', j0, j1, j2, j3]
            if [t0, t1, m0, m1, j0, j1, j2, j3]
                .iter()
                .all(|b| b.is_ascii_digit()) =>
        {
            vierstellig(*j0, *j1, *j2, *j3)
        }
        _ => None,
    }
}

/// (Monat, Tag) aus demselben Datum (`store.py:458-465`, `_monat_tag`).
#[must_use]
pub fn monat_tag_aus_datum(wert: &str) -> Option<(u32, u32)> {
    let w = wert.trim();
    match w.as_bytes() {
        [_, _, _, _, b'-', m0, m1, b'-', t0, t1]
            if [m0, m1, t0, t1].iter().all(|b| b.is_ascii_digit()) =>
        {
            Some((zweistellig(*m0, *m1)?, zweistellig(*t0, *t1)?))
        }
        [t0, t1, b'.', m0, m1, b'.', _, _, _, _]
            if [t0, t1, m0, m1].iter().all(|b| b.is_ascii_digit()) =>
        {
            Some((zweistellig(*m0, *m1)?, zweistellig(*t0, *t1)?))
        }
        _ => None,
    }
}

fn vierstellig(a: u8, b: u8, c: u8, d: u8) -> Option<i64> {
    std::str::from_utf8(&[a, b, c, d]).ok()?.parse().ok()
}

fn zweistellig(a: u8, b: u8) -> Option<u32> {
    std::str::from_utf8(&[a, b]).ok()?.parse().ok()
}

/// Der abgeleitete Wert, oder `None`, wenn er sich nicht sicher bestimmen laesst (`store.py:468-500`,
/// `_berechne`). `wert` muss ein String sein (Datum) — jeder andere JSON-Typ liefert `None`, wie
/// Pythons `_jahr`/`_monat_tag` (`isinstance(wert, str)`-Wache).
#[must_use]
pub fn berechne(regel: &Ableitung, wert: &serde_json::Value, vz: i64) -> Option<serde_json::Value> {
    let wert_str = wert.as_str()?;
    match regel.art {
        AbleitungArt::AlterUnterAmJahresende => {
            let jahr = jahr_aus_datum(wert_str)?;
            #[allow(clippy::cast_possible_truncation)]
            let schwelle = regel.schwelle? as i64;
            if (vz - jahr) < schwelle {
                Some(serde_json::Value::Bool(true))
            } else {
                None
            }
        }
        AbleitungArt::JahrAusDatum => Some(serde_json::json!(jahr_aus_datum(wert_str)?)),
        AbleitungArt::AlterAmJahresbeginnErreicht => {
            let jahr = jahr_aus_datum(wert_str)?;
            let monat_tag = monat_tag_aus_datum(wert_str)?;
            #[allow(clippy::cast_possible_truncation)]
            let schwelle = regel.schwelle? as i64;
            let vollendet_im_jahr = jahr + schwelle;
            let ok = vollendet_im_jahr < vz || (vollendet_im_jahr == vz && monat_tag == (1, 1));
            Some(serde_json::Value::Bool(ok))
        }
        // PARITAET: `store.py::_berechne` hat fuer `art == "uebernahme"` KEINEN eigenen Zweig —
        // der Kontrollfluss faellt durch `jahr_aus_datum`/beide `if art == ...`-Zweige bis zum
        // finalen `return None`. Das Ergebnis ist in Python IMMER `None`, unabhaengig vom Wert.
        // Dokumentierte Luecke (keine bekannte Bindung nutzt diese Art heute), keine erfundene
        // Semantik.
        AbleitungArt::Uebernahme => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{berechne, jahr_aus_datum, monat_tag_aus_datum};
    use bindung::{Ableitung, AbleitungArt};
    use serde_json::json;

    #[test]
    fn jahr_aus_iso_und_deutschem_datum() {
        assert_eq!(jahr_aus_datum("1955-05-05"), Some(1955));
        assert_eq!(jahr_aus_datum("05.05.1955"), Some(1955));
        assert_eq!(jahr_aus_datum("nicht-datum"), None);
    }

    #[test]
    fn monat_tag_reihenfolge_ist_unabhaengig_vom_eingabeformat() {
        assert_eq!(monat_tag_aus_datum("1955-05-09"), Some((5, 9)));
        assert_eq!(monat_tag_aus_datum("09.05.1955"), Some((5, 9)));
    }

    fn regel(art: AbleitungArt, schwelle: Option<f64>) -> Ableitung {
        Ableitung {
            aus: "geburtsdatum".to_string(),
            art,
            schwelle,
            grund: "test".to_string(),
            und_feld: None,
        }
    }

    #[test]
    fn alter_am_jahresbeginn_grenzfall_erster_januar() {
        // "vor Beginn des Kalenderjahres das 64. Lebensjahr vollendet" (§ 24a S. 3 EStG):
        // wer am 01.01.1961 geboren ist, vollendet das 64. Lebensjahr mit Ablauf des 31.12.2024.
        let r = regel(AbleitungArt::AlterAmJahresbeginnErreicht, Some(64.0));
        assert_eq!(berechne(&r, &json!("01.01.1961"), 2025), Some(json!(true)));
        assert_eq!(berechne(&r, &json!("02.01.1961"), 2025), Some(json!(false)));
    }

    #[test]
    fn uebernahme_ist_immer_none() {
        let r = regel(AbleitungArt::Uebernahme, None);
        assert_eq!(berechne(&r, &json!("05.05.1955"), 2025), None);
    }

    #[test]
    fn nicht_string_wert_liefert_none() {
        let r = regel(AbleitungArt::JahrAusDatum, None);
        assert_eq!(berechne(&r, &json!(1955), 2025), None);
    }
}
