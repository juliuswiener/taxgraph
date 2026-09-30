//! Guard-Abschnitte des `gesamt_guard`-Zweigs, die Einkunftsarten und fehlende Betraege betreffen
//! (`_an_gesamt_sperrgrund`, DBA bis `gewst_hebesatz_offen`).
use std::collections::HashSet;

use domain::Sperrgrund;
use konsistenz::flag_widersprueche;
use serde_json::Value;

use super::{bestaetigt, oder_null_positiv, positiv, Grund, K};
use crate::deklaration::konstanten::AGB_KIST;
use crate::einkuenfte::{
    EUER_KOMPONENTEN, GEWINN_QUELLEN_MENGEN, KAP_ERTRAEGE, KAP_ERTRAEGE_PARTNER, KAP_TOEPFE,
    KAP_TOEPFE_PARTNER,
};
use crate::{ist_false, ist_true, ist_zusammen, wert, Felder};

/// § 34c DBA (multi-country, § 32d-Kapital), § 32b-Koinzidenz, § 16 Abs. 4 Freibetrag.
pub(super) fn dba_p32b_p16(k: &K<'_>) -> Grund {
    let f = k.f;
    if ist_true(wert(f, "dba_mehrere_staaten")) {
        return Ok(Some(Sperrgrund::DbaMultiCountryOffen));
    }
    // Kapital gesetzt (Toepfe oder Aggregat) UND auslaendische Einkuenfte: keine stille 0-Anrechnung.
    let kapital = KAP_TOEPFE.iter().any(|t| positiv(f, t)) || oder_null_positiv(f, KAP_ERTRAEGE)?;
    if kapital && positiv(f, "dba_auslaendische_einkuenfte") {
        return Ok(Some(Sperrgrund::DbaKapitalOffen));
    }
    if positiv(f, "p32b_progressionseinkuenfte") && p32b_koinzidenz(f) {
        return Ok(Some(Sperrgrund::P32bKombiOffen));
    }
    Ok(p16_4(f))
}

/// § 32b Post-Engine NACH § 34/§ 35/§ 34c: Co-Praesenz ist in Stufe 1 unaufgeloest.
fn p32b_koinzidenz(f: &Felder) -> bool {
    // 1. § 34 ao-Gewinn / ermaessigter Satz
    positiv(f, "rentner_veraeusserungsgewinn")
        || ist_true(wert(f, "antrag_ermaessigter_satz"))
        // 1b. ao-Gewinn des Ehegatten (die Fuenftelung umfasst beide)
        || (ist_zusammen(f) && positiv(f, "rentner_veraeusserungsgewinn_partner"))
        // 2. § 35 Gewerbesteuer
        || positiv(f, "gewst_messbetrag")
        // 3. § 34c DBA-Anrechnung
        || positiv(f, "dba_gezahlte_auslaendische_steuer")
        || positiv(f, "dba_auslaendische_einkuenfte")
}

/// § 16 Abs. 4: Veraeusserungsgewinn > 0 verlangt beide Bedingungs-Bools BESTAETIGT (egal welcher
/// Wert); fuer den Ehegatten nur bei Zusammenveranlagung.
fn p16_4(f: &Felder) -> Option<Sperrgrund> {
    let offen = |gewinn: &str, alter: &str, erstmalig: &str| {
        positiv(f, gewinn) && !(bestaetigt(f, alter) && bestaetigt(f, erstmalig))
    };
    let eigen = offen(
        "rentner_veraeusserungsgewinn",
        "rentner_alter_55_oder_berufsunfaehig",
        "rentner_freibetrag_erstmalig",
    );
    let partner = ist_zusammen(f)
        && offen(
            "rentner_veraeusserungsgewinn_partner",
            "rentner_alter_55_oder_berufsunfaehig_partner",
            "rentner_freibetrag_erstmalig_partner",
        );
    (eigen || partner).then_some(Sperrgrund::P164GateOffen)
}

/// Flag ↔ Einkunftsart, Kapital-Semantik, Gewinn-Quelle, EUeR-Vollstaendigkeit.
pub(super) fn flag_kapital_gewinn(k: &K<'_>) -> Option<Sperrgrund> {
    let f = k.f;
    // `bindung is not None`: die Feldmenge der Scheibe; `None` = Alt-Verhalten.
    let scheibe: Option<HashSet<String>> = k.q.bindung.map(|b| b.keys().cloned().collect());
    if !flag_widersprueche(f, scheibe.as_ref()).is_empty() {
        return Some(Sperrgrund::FlagKonsistenzOffen);
    }
    // E1900701-Aggregat UND Verlust-Toepfe beide gesetzt: additiv-vs-subset ungeklaert.
    let semantik_eigen = positiv(f, KAP_ERTRAEGE) && KAP_TOEPFE.iter().any(|t| positiv(f, t));
    let semantik_partner = ist_zusammen(f)
        && positiv(f, KAP_ERTRAEGE_PARTNER)
        && KAP_TOEPFE_PARTNER.iter().any(|t| positiv(f, t));
    if semantik_eigen || semantik_partner {
        return Some(Sperrgrund::KapitalSemantikOffen);
    }
    gewinn(f)
}

/// §§ 13-18: Direktwert UND EUeR-Komponenten (Doppelquelle), Land-/Forstwirtschaft mit `EUeR`,
/// begonnener EUeR-Weg mit unbeantworteter Angabe. `GEWINN_QUELLEN_MENGEN` ist DIESELBE Menge wie
/// die des Umschalters in `laufender_gewinn`.
fn gewinn(f: &Felder) -> Option<Sperrgrund> {
    let quelle_positiv = GEWINN_QUELLEN_MENGEN.iter().any(|q| positiv(f, q));
    let direktwert = positiv(f, "einkuenfte_gewinn");
    if direktwert && quelle_positiv {
        return Some(Sperrgrund::GewinnQuelleOffen);
    }
    let land_forst =
        matches!(wert(f, "gewinn_betriebsart"), Some(Value::String(s)) if s == "land_forst");
    if land_forst && quelle_positiv && !direktwert {
        return Some(Sperrgrund::LufEuerOffen);
    }
    // Eine bestaetigte 0 ist eine Antwort; NICHT schon bei kein_gewinn = nein.
    let euer_begonnen = GEWINN_QUELLEN_MENGEN.iter().any(|q| bestaetigt(f, q));
    (euer_begonnen && !direktwert && EUER_KOMPONENTEN.iter().any(|q| !bestaetigt(f, q)))
        .then_some(Sperrgrund::GewinnAngabenOffen)
}

/// Betraege mit genau einem Screening-Flag und ohne Ersatzquelle: Thema eroeffnet, Betrag unbestaetigt.
pub(super) fn betrag_offen(k: &K<'_>) -> Option<Sperrgrund> {
    let f = k.f;
    let verneint = |flag: &str| ist_false(wert(f, flag));
    if verneint("keine_lohnersatzleistungen") && !bestaetigt(f, "p32b_progressionseinkuenfte") {
        return Some(Sperrgrund::LohnersatzBetragOffen);
    }
    if verneint("kein_verlustvortrag") && !bestaetigt(f, "verlustvortrag_bestand") {
        return Some(Sperrgrund::VerlustvortragBetragOffen);
    }
    if verneint("kein_unterhalt") && !bestaetigt(f, "p33a_unterhalt_aufwendungen") {
        return Some(Sperrgrund::UnterhaltBetragOffen);
    }
    // `wert not in (None, "keine")`
    let konfession = !matches!(wert(f, "kist_konfession"), None | Some(Value::Null))
        && !matches!(wert(f, "kist_konfession"), Some(Value::String(s)) if s == "keine");
    if konfession && AGB_KIST.iter().any(|a| !bestaetigt(f, a)) {
        return Some(Sperrgrund::KirchensteuerBetragOffen);
    }
    realsplitting_fahrtkosten(f).or_else(|| {
        (positiv(f, "gewst_messbetrag") && !bestaetigt(f, "gewst_hebesatz"))
            .then_some(Sperrgrund::GewstHebesatzOffen)
    })
}

/// § 10 Abs. 1a Nr. 1 (Zustimmung des Empfaengers) und § 33 Abs. 2a S. 2 (zwei ODER-Wege).
fn realsplitting_fahrtkosten(f: &Felder) -> Option<Sperrgrund> {
    // Eine verneinte Zustimmung schliesst die Regel aus; dann wird auch der Betrag nicht verlangt.
    let zustimmung_verneint =
        bestaetigt(f, "realsplitting_zustimmung") && ist_false(wert(f, "realsplitting_zustimmung"));
    if ist_false(wert(f, "kein_realsplitting"))
        && !zustimmung_verneint
        && (!bestaetigt(f, "realsplitting_zustimmung")
            || !bestaetigt(f, "realsplitting_unterhaltsleistungen"))
    {
        return Some(Sperrgrund::RealsplittingAngabenOffen);
    }
    // aG/Bl/TBl/H = ja entscheidet allein; sonst zaehlt die 900-EUR-Frage mit.
    (ist_false(wert(f, "keine_behinderung_pflege"))
        && !ist_true(wert(f, "fahrtkosten_pausch_ag_bl_tbl_h"))
        && (!bestaetigt(f, "fahrtkosten_pausch_ag_bl_tbl_h")
            || !bestaetigt(f, "fahrtkosten_pausch_gdb80_oder_70g")))
    .then_some(Sperrgrund::FahrtkostenpauschaleOffen)
}
