//! Reisekosten-Bausteine von `catala_werbungskosten_n` (runner.py `_dhf_abzug`,
//! `_verpflegung_abzug`, `_uebernachtung_abzug`): Transkriptionen der Registry-Rechenwege
//! `p9_1_3_nr5`, `p9_4a`, `p9_1_3_nr5a_*`. KOPPLUNG: bei Aenderung der Registry-Regeln nachziehen.
use bindung::Params;
use domain::{Cent, Euro, Vz};

use super::fehler::{ok, EngineFehler};

/// Doppelte Haushaltsfuehrung (§ 9 Abs. 1 S. 3 Nr. 5 `EStG`).
#[derive(Debug, Clone, Copy)]
pub struct DhfEingabe {
    /// `s["veranlagungszeitraum"]` (Pflicht, sobald der Zweig laeuft).
    pub veranlagungszeitraum: Vz,
    /// `unterkunftskosten_monat` (sein Vorhandensein schaltet den Zweig ein).
    pub unterkunftskosten_monat: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub monate: i64,
    /// PARITÄT: Python setzt fehlend = True (fail-open: Inlandsgrenze statt Auslandsgrenze)
    pub im_inland: bool,
}

/// § 9 Abs. 1 S. 3 Nr. 5 `EStG` -- abziehbare Unterkunftskosten, EURO: Monatsmiete gekappt je
/// Monat (Inland 1.000, Ausland 2.000 aus `params/<vz>`), mal Monate. Fuehrt `params/<vz>` keine
/// Auslandsgrenze (VZ 2024/2025: sie gilt erst ab VZ 2026, `StÄndG` 2025), bleibt die Auslandsmiete
/// ungekappt. PARITÄT: Python `_dhf_abzug`, `None` = keine Grenze.
pub(crate) fn dhf_abzug(e: &DhfEingabe, p: &Params) -> Result<i64, EngineFehler> {
    let cap = p.dhf(e.veranlagungszeitraum)?;
    let grenze = if e.im_inland {
        Some(cap.cap_monat_inland)
    } else {
        cap.cap_monat_ausland
    };
    let miete = e.unterkunftskosten_monat.get();
    ok(
        grenze
            .map_or(miete, |g| miete.min(g.get()))
            .checked_mul(e.monate),
        "dhf",
    )
}

/// Verpflegungsmehraufwand (§ 9 Abs. 4a `EStG`). Tage und Mahlzeiten als Anzahl, Entgelt und
/// Erstattung in CENT (runner.py liest sie roh als Cent).
#[derive(Debug, Clone, Copy)]
pub struct VerpflegungEingabe {
    /// `s["veranlagungszeitraum"]` (Pflicht, sobald der Zweig laeuft).
    pub veranlagungszeitraum: Vz,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub tage_24h: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub tage_an_abreise: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub tage_ueber_8h_eintaegig: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Dreimonatsfrist)
    pub vpf_tage_24h_nach_drei_monaten: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Dreimonatsfrist)
    pub vpf_tage_an_abreise_nach_drei_monaten: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Dreimonatsfrist)
    pub vpf_tage_ueber_8h_nach_drei_monaten: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Mahlzeitenkuerzung)
    pub vpf_fruehstuecke_gestellt_anzahl: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Mahlzeitenkuerzung)
    pub vpf_mittagessen_gestellt_anzahl: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Mahlzeitenkuerzung)
    pub vpf_abendessen_gestellt_anzahl: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-closed: mindert die Kuerzung nicht)
    pub vpf_mahlzeiten_gezahltes_entgelt: Cent,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: keine Erstattung abgezogen)
    pub vpf_steuerfreie_erstattung_betrag: Cent,
}

fn mul3(a: i64, b: i64, c: i64, wo: &'static str) -> Result<i64, EngineFehler> {
    ok(a.checked_mul(b).and_then(|x| x.checked_mul(c)), wo)
}

/// § 9 Abs. 4a `EStG` -- Jahres-Verpflegungspauschale, EURO.
///
/// S. 3 Jahressumme je Kategorie; S. 6 Tage nach der Dreimonatsfrist fallen weg (negativ -> 0);
/// S. 8 Mahlzeitenkuerzung, Bezugsgroesse IMMER die 24-h-Pauschale, Mahlzeiten zuerst den
/// 24-h-Tagen, Rest den 14-EUR-Tagen zugeordnet, je Kategorie auf die Pauschalensumme gedeckelt;
/// S. 10 gezahltes Entgelt mindert die Kuerzung (Floor 0); S. 11 steuerfreie Erstattung mindert
/// den Abzug (Floor 0). Rechnung in Cent, am Ende abgerundet auf Euro.
pub(crate) fn verpflegung_abzug(e: &VerpflegungEingabe, p: &Params) -> Result<i64, EngineFehler> {
    let s = p.verpflegung(e.veranlagungszeitraum)?;
    let in_frist =
        |tage: i64, nach: i64| ok(tage.checked_sub(nach.max(0)), "vpf tage").map(|t| t.max(0));
    let s24 = mul3(
        in_frist(e.tage_24h, e.vpf_tage_24h_nach_drei_monaten)?,
        s.pauschale_24h.get(),
        100,
        "vpf s24",
    )?;
    let sa = mul3(
        in_frist(e.tage_an_abreise, e.vpf_tage_an_abreise_nach_drei_monaten)?,
        s.pauschale_an_abreise.get(),
        100,
        "vpf sa",
    )?;
    let s8 = mul3(
        in_frist(
            e.tage_ueber_8h_eintaegig,
            e.vpf_tage_ueber_8h_nach_drei_monaten,
        )?,
        s.pauschale_ab_8h.get(),
        100,
        "vpf s8",
    )?;
    let pauschale_gesamt = ok(
        s24.checked_add(sa).and_then(|x| x.checked_add(s8)),
        "vpf gesamt",
    )?;

    let p24_cent = ok(s.pauschale_24h.get().checked_mul(100), "vpf p24")?;
    let je_fruehstueck = ok(
        p24_cent
            .checked_mul(s.kuerzung_fruehstueck_prozent)
            .and_then(|x| x.checked_div_euclid(100)),
        "vpf kf",
    )?;
    let je_mittag_abend = ok(
        p24_cent
            .checked_mul(s.kuerzung_mittag_abend_prozent)
            .and_then(|x| x.checked_div_euclid(100)),
        "vpf km",
    )?;
    let k28_brutto = ok(
        e.vpf_fruehstuecke_gestellt_anzahl
            .checked_mul(je_fruehstueck)
            .zip(
                e.vpf_mittagessen_gestellt_anzahl
                    .checked_mul(je_mittag_abend),
            )
            .zip(
                e.vpf_abendessen_gestellt_anzahl
                    .checked_mul(je_mittag_abend),
            )
            .and_then(|((f, m), a)| f.checked_add(m)?.checked_add(a)),
        "vpf k28",
    )?;
    let k28 = k28_brutto.min(s24);
    let rest = ok(k28_brutto.checked_sub(k28), "vpf rest")?.max(0);
    let k14 = rest.min(ok(sa.checked_add(s8), "vpf s14")?);
    let kuerzung = ok(k28.checked_add(k14), "vpf k")?;
    let nach_entgelt = ok(
        kuerzung.checked_sub(e.vpf_mahlzeiten_gezahltes_entgelt.get()),
        "vpf entgelt",
    )?
    .max(0);
    let ergebnis = ok(
        pauschale_gesamt
            .checked_sub(nach_entgelt)
            .and_then(|x| x.checked_sub(e.vpf_steuerfreie_erstattung_betrag.get())),
        "vpf ergebnis",
    )?
    .max(0);
    Ok(Cent::new(ergebnis).floor_euro().get())
}

/// Uebernachtung bei Auswaertstaetigkeit (§ 9 Abs. 1 S. 3 Nr. 5a `EStG`).
#[derive(Debug, Clone, Copy)]
pub struct UebernachtungEingabe {
    /// `s["veranlagungszeitraum"]` (Pflicht, sobald der Zweig laeuft).
    pub veranlagungszeitraum: Vz,
    /// `uebernachtung_kosten_monat` (sein Vorhandensein schaltet den Zweig ein).
    pub uebernachtung_kosten_monat: Euro,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open)
    pub uebernachtung_monate: i64,
    /// PARITÄT: Python setzt fehlend = 0 (fail-open: alle Monate vor der 48-Monats-Schwelle, ungekappt)
    pub uebernachtung_monate_bisher: i64,
    /// PARITÄT: Python setzt fehlend = True (fail-open)
    pub uebernachtung_im_inland: bool,
}

/// § 9 Abs. 1 S. 3 Nr. 5a `EStG` -- Uebernachtungskosten, EURO. Die ersten 48 Monate am selben
/// Ort ungekappt; danach auf die Grenze nach Nr. 5 gekappt (S. 4). Ein Zeitraum ueber die
/// Schwelle wird monatsweise geteilt (BMF v. 25.11.2020, Rz. 126). Die Grenze kommt aus denselben
/// Params wie bei der dHf, ohne eigenen VZ-Schalter: die 2.000-EUR-Auslandsgrenze gilt erst ab
/// VZ 2026 (`StÄndG` 2025), davor fuehrt `params/<vz>` keine (BMF-Reisekosten v. 25.11.2020,
/// Rz. 124).
pub(crate) fn uebernachtung_abzug(
    e: &UebernachtungEingabe,
    p: &Params,
) -> Result<i64, EngineFehler> {
    let kosten = e.uebernachtung_kosten_monat.get();
    let bis_schwelle = ok(48i64.checked_sub(e.uebernachtung_monate_bisher), "uen 48")?;
    let vor_48 = e.uebernachtung_monate.min(bis_schwelle).max(0);
    let nach_48 = ok(e.uebernachtung_monate.checked_sub(vor_48), "uen nach")?;
    let cap = p.dhf(e.veranlagungszeitraum)?;
    let grenze = if e.uebernachtung_im_inland {
        Some(cap.cap_monat_inland)
    } else {
        cap.cap_monat_ausland
    };
    let gekappt = grenze.map_or(kosten, |g| kosten.min(g.get()));
    ok(
        kosten
            .checked_mul(vor_48)
            .zip(gekappt.checked_mul(nach_48))
            .and_then(|(a, b)| a.checked_add(b)),
        "uebernachtung",
    )
}

#[cfg(test)]
mod tests {
    use super::{dhf_abzug, uebernachtung_abzug, DhfEingabe, UebernachtungEingabe};
    use bindung::Params;
    use domain::{Euro, Vz};

    fn params() -> Params {
        Params::lade(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
    }

    fn dhf(vz: Vz, miete: i64, im_inland: bool) -> i64 {
        let e = DhfEingabe {
            veranlagungszeitraum: vz,
            unterkunftskosten_monat: Euro::new(miete),
            monate: 12,
            im_inland,
        };
        dhf_abzug(&e, &params()).unwrap()
    }

    fn uebernachtung(vz: Vz, im_inland: bool) -> i64 {
        let e = UebernachtungEingabe {
            veranlagungszeitraum: vz,
            uebernachtung_kosten_monat: Euro::new(2500),
            uebernachtung_monate: 12,
            uebernachtung_monate_bisher: 48,
            uebernachtung_im_inland: im_inland,
        };
        uebernachtung_abzug(&e, &params()).unwrap()
    }

    /// Die 2.000-EUR-Auslandsgrenze gilt erst ab VZ 2026 (`StÄndG` 2025, `BGBl`. 2025 I Nr. 363).
    /// In `params/2024` und `params/2025` steht sie nicht; 2.500 EUR x 12 bleiben dort ungekappt.
    /// Erwartungswerte = Python-Orakel auf 604022c8 mit den neuen Params:
    /// `runner._dhf_abzug({"unterkunftskosten_monat": 2500, "monate": 12, "im_inland": False}, vz)`
    /// liefert 30000 / 30000 / 24000.
    #[test]
    fn dhf_ausland_ist_vor_vz2026_ungekappt() {
        assert_eq!(dhf(Vz::Vz2024, 2500, false), 30_000);
        assert_eq!(dhf(Vz::Vz2025, 2500, false), 30_000);
        assert_eq!(dhf(Vz::Vz2026, 2500, false), 24_000);
    }

    /// Kontrollfall: die Inlandsgrenze 1.000 gilt in jedem der drei VZ (1.400 x 12 -> 12.000).
    #[test]
    fn dhf_inland_bleibt_in_jedem_vz_auf_1000_gekappt() {
        for vz in [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026] {
            assert_eq!(dhf(vz, 1400, true), 12_000);
        }
    }

    /// Nr. 5a verweist fuer die Hoehe nach 48 Monaten auf Nr. 5: dieselbe Grenze aus denselben
    /// Params, kein zweiter VZ-Schalter. Python-Orakel `runner._uebernachtung_abzug` (bisher=48,
    /// monate=12, 2.500 EUR): Ausland 30000 / 30000 / 24000, Inland 12000 in allen drei.
    #[test]
    fn uebernachtung_nach_48_monaten_folgt_derselben_grenze() {
        assert_eq!(uebernachtung(Vz::Vz2024, false), 30_000);
        assert_eq!(uebernachtung(Vz::Vz2025, false), 30_000);
        assert_eq!(uebernachtung(Vz::Vz2026, false), 24_000);
        for vz in [Vz::Vz2024, Vz::Vz2025, Vz::Vz2026] {
            assert_eq!(uebernachtung(vz, true), 12_000);
        }
    }
}
