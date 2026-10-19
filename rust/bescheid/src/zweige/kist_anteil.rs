//! KiSt § 51a bei Zusammenveranlagung nach Landesrecht (Abweichung Nr. 94, Rust-eigen).
//!
//! Die Landes-Kirchensteuergesetze erheben die Kirchensteuer bei Zusammenveranlagung, wenn nur EIN Ehegatte Mitglied einer steuererhebenden
//! Kirche ist (glaubensverschiedene Ehe), nur von dem Teil der gemeinsamen Einkommensteuer, der auf das Mitglied entfaellt (NRW § 7 Abs. 2,
//! BY Art. 9 Abs. 2 Nr. 2, BW § 19 Abs. 4; in allen 16 Laendern gleich). Der Teil folgt dem Verhaeltnis der Betraege, die sich bei Anwendung
//! des Grundtarifs (§ 32a Abs. 1 `EStG`) auf die Summe der Einkuenfte jeder Person ergeben wuerden. Gesondert besteuerte Kapitaleinkuenfte
//! (§ 32d) zaehlen in der Verhaeltnisrechnung nicht mit (NRW § 7 Abs. 2 Satz 3); sie stehen nicht in [`EinkuenfteJePerson`]. Vor Nr. 94
//! rechnete die App Satz mal VOLLE gemeinsame ESt, wenn Person A Mitglied war, und 0, wenn nur Person B es war.
//!
//! Sind beide Mitglied (gleiche oder verschiedene Kirche), bleibt die Summe Satz mal volle ESt: bei konfessionsverschiedener Ehe teilen die
//! Laender die ESt je Kirche (14 Laender zur Haelfte, Bayern nach den Einkuenften), und der Satz ist je Land fuer beide Kirchen gleich.
//! Die Verteilung auf die zwei Kirchen gibt die App nicht aus.
use domain::{Cent, Euro, Vz};
use engine::tarif::Veranlagung;
use engine::zugriff::teil1::ermaessigungen::kist;
use engine::zugriff::teil2::est::{tarif_est, TarifEingabe};

use super::ausgaben::kist_konfession_partner;
use super::rechnen::{add, mal_div, max0, R};
use super::tarif::kist_eingabe;
use crate::Felder;

/// Die Summe der Einkuenfte (§ 2 Abs. 3 `EStG`, VOR Altersentlastungs- und Entlastungsbetrag) je Person, EURO, ohne die gesondert
/// besteuerten Kapitaleinkuenfte (§ 32d). Person A traegt auch, was das Modell keiner Person zuordnet: Vermietung (ohne Miteigentum),
/// § 22 Nr. 3 und § 23.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EinkuenfteJePerson {
    pub a: Euro,
    pub b: Euro,
}

/// Kirchensteuer in CENT auf die Massstabsteuer `est` (§ 51a, EURO) nach der Konfession von Person A; bei Zusammenveranlagung mit
/// beantworteter Konfession von Person B nach Landesrecht (siehe Moduldoku).
///
/// Ist die Konfession von Person B nicht beantwortet, bleibt es beim bisherigen Weg (Satz mal volle ESt nach Person A): die Antwort
/// steht in `gesamt` und `rentner_gesamt` als offene Frage, und die Zahl ist bis dahin eine Schaetzung.
///
/// # Errors
/// Ueberlauf- und Tarif-Fehler.
pub(super) fn kist_cent(
    f: &Felder,
    vz: Vz,
    konfession_a: &str,
    est: Euro,
    zusammen: bool,
    einkuenfte: EinkuenfteJePerson,
) -> R<Cent> {
    let voll = kist(&kist_eingabe(f, konfession_a, est))?;
    if !zusammen {
        return Ok(voll);
    }
    let Some(konfession_b) = kist_konfession_partner(f) else {
        return Ok(voll);
    };
    match (
        ist_mitglied(f, konfession_a)?,
        ist_mitglied(f, konfession_b)?,
    ) {
        (true, false) => anteil(voll, einkuenfte.a, einkuenfte.b, vz),
        (false, true) => {
            let voll_b = kist(&kist_eingabe(f, konfession_b, est))?;
            anteil(voll_b, einkuenfte.b, einkuenfte.a, vz)
        }
        // Beide Mitglied: die Summe bleibt. Keines Mitglied: `voll` ist 0.
        _ => Ok(voll),
    }
}

/// Steuererhebende Konfession? Die Liste steht nur in `kist` (`ermaessigungen.rs`): eine Konfession ist Mitglied genau dann, wenn
/// ein Euro Massstabsteuer eine Kirchensteuer ergibt.
fn ist_mitglied(f: &Felder, konfession: &str) -> R<bool> {
    Ok(kist(&kist_eingabe(f, konfession, Euro::new(1)))?.get() > 0)
}

/// Der Teil von `voll` (CENT), der auf `mein` entfaellt: Grundtarif auf `mein` im Verhaeltnis zur Summe der Grundtarif-Betraege beider.
/// Auf den Cent abgerundet (die Landestexte regeln die Rundung nicht). Negative Einkuenfte zaehlen 0.
///
/// ponytail: sind beide Grundtarif-Betraege 0 (jede Summe der Einkuenfte unter dem Grundfreibetrag, die Steuer entsteht nur ueber
/// § 32b), ist das Verhaeltnis nicht definiert; die Rechnung bleibt dann bei `voll`. Upgrade: Entscheidung, wie die Laender diesen Fall
/// halten.
fn anteil(voll: Cent, mein: Euro, anderer: Euro, vz: Vz) -> R<Cent> {
    let grund = |e: Euro| -> R<Euro> {
        Ok(tarif_est(&TarifEingabe {
            vz,
            veranlagung: Veranlagung::Einzel,
            zu_versteuerndes_einkommen: max0(e),
        })?)
    };
    let (t_mein, t_anderer) = (grund(mein)?, grund(anderer)?);
    let summe = add(t_mein, t_anderer)?;
    if summe.get() == 0 {
        return Ok(voll);
    }
    Ok(Cent::new(mal_div(voll.get(), t_mein.get(), summe.get())?))
}

/// Die Zahlen stammen aus der Handrechnung des Auftrags (`handrechnung.py`, Tarif und Pauschbetraege aus `params/2025/`): Einkuenfte
/// 58.770 (A) und 18.770 EUR (B), Grundtarif 13.938 und 1.336 EUR, gemeinsame ESt 13.838 EUR.
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::testhilfe::{felder, store};

    const E: EinkuenfteJePerson = EinkuenfteJePerson {
        a: Euro::new(58_770),
        b: Euro::new(18_770),
    };
    const EST: Euro = Euro::new(13_838);
    const VZ: Vz = Vz::Vz2025;

    fn fall(land: &str, konf_b: Option<&str>) -> Felder {
        let mut ev = vec![("kist_bundesland", json!(land), true)];
        if let Some(k) = konf_b {
            ev.push(("kist_konfession_partner", json!(k), true));
        }
        felder(&store(&ev))
    }

    fn cent(f: &Felder, a: &str, zusammen: bool, e: EinkuenfteJePerson) -> i64 {
        kist_cent(f, VZ, a, EST, zusammen, e).unwrap().get()
    }

    #[test]
    fn nur_ein_ehegatte_mitglied_zahlt_seinen_anteil() {
        let f = fall("nordrhein_westfalen", Some("keine"));
        assert_eq!(cent(&f, "evangelisch", true, E), 113_648, "A: 124.542 x 13.938 / 15.274");
        let f = fall("nordrhein_westfalen", Some("evangelisch"));
        assert_eq!(cent(&f, "keine", true, E), 10_893, "B: 124.542 x 1.336 / 15.274");
        let f = fall("bayern", Some("evangelisch"));
        assert_eq!(cent(&f, "keine", true, E), 9_683, "B in Bayern (8 %): 110.704 x 1.336 / 15.274");
    }

    #[test]
    fn beide_mitglied_oder_ehegatte_unbekannt_oder_einzel_bleibt_satz_mal_volle_est() {
        let voll = 124_542;
        for (konf_b, zusammen) in [
            (Some("evangelisch"), true),
            (Some("roemisch-katholisch"), true),
            (None, true),
            (Some("keine"), false),
        ] {
            let f = fall("nordrhein_westfalen", konf_b);
            assert_eq!(cent(&f, "evangelisch", zusammen, E), voll, "{konf_b:?} zusammen={zusammen}");
        }
        // Keiner Mitglied, auch "andere" ist keine steuererhebende Konfession der App: 0.
        let f = fall("nordrhein_westfalen", Some("andere"));
        assert_eq!(cent(&f, "keine", true, E), 0);
        assert_eq!(cent(&f, "andere", true, E), 0);
    }

    #[test]
    fn negative_einkuenfte_zaehlen_null_und_ohne_grundtarif_bleibt_es_bei_der_vollen_est() {
        let f = fall("nordrhein_westfalen", Some("keine"));
        let verlust_b = EinkuenfteJePerson { a: Euro::new(58_770), b: Euro::new(-5_000) };
        assert_eq!(cent(&f, "evangelisch", true, verlust_b), 124_542, "T(B) = 0: A traegt alles");
        // Ein Verlust, dessen Cent-Betrag nicht mehr in i64 passt, rechnet der Tarif nicht (Ueberlauf): der Boden 0 faengt ihn vorher ab.
        let riesig = EinkuenfteJePerson { a: Euro::new(58_770), b: Euro::new(-92_233_720_368_547_759) };
        assert_eq!(cent(&f, "evangelisch", true, riesig), 124_542, "Verlust ausserhalb i64 in Cent: wie 0");
        let verlust_a = EinkuenfteJePerson { a: Euro::new(-5_000), b: Euro::new(58_770) };
        assert_eq!(cent(&f, "evangelisch", true, verlust_a), 0, "T(A) = 0: nichts entfaellt auf A");
        let unter_freibetrag = EinkuenfteJePerson { a: Euro::new(10_000), b: Euro::new(12_096) };
        assert_eq!(cent(&f, "evangelisch", true, unter_freibetrag), 124_542, "beide Grundtarif-Betraege 0: nicht definiert, voll");
    }
}
