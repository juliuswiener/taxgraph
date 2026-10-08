//! Tabellen aus `produkt/haut/api_constants.py`, die `bescheid_deklaration.py` liest. Erzeugt aus
//! dem Python-Baum (Reihenfolge wie dort); `konstanten_gleich` in
//! `rust/parity/tests/bescheid_deklaration_paritaet.rs` vergleicht jede Tabelle gegen das Orakel.
//! Nicht von Hand pflegen: `api_constants.py` ist die Quelle.
use domain::Scheibe;

/// `ARBEITSMITTEL_KOSTEN`.
pub(super) const ARBEITSMITTEL_KOSTEN: &str = "am_anschaffungskosten";
/// `DHF_KOSTEN`.
pub(super) const DHF_KOSTEN: &str = "dhf_unterkunftskosten_monat";
/// `UEBERNACHTUNG_KOSTEN`.
pub(super) const UEBERNACHTUNG_KOSTEN: &str = "uebernachtung_kosten_monat";

/// `AGB_KIST`.
pub(super) const AGB_KIST: [&str; 2] = ["kist_gezahlt", "kist_erstattet"];
/// `AN_GESAMT_FLAGS`.
pub(super) const AN_GESAMT_FLAGS: [&str; 4] =
    ["kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige"];
/// `AN_GESAMT_PARTNER`.
pub(super) const AN_GESAMT_PARTNER: [&str; 1] = ["bruttoarbeitslohn_partner"];
/// `DHF_BEDINGUNGEN`.
pub(super) const DHF_BEDINGUNGEN: [&str; 3] = [
    "dhf_beruflich_veranlasst",
    "dhf_eigener_hausstand",
    "dhf_finanzielle_beteiligung",
];
/// `GESAMT_PARTNER_19`.
pub(super) const GESAMT_PARTNER_19: [&str; 1] = ["bruttoarbeitslohn_partner"];
/// `GESAMT_PARTNER_KAP`.
pub(super) const GESAMT_PARTNER_KAP: [&str; 5] = [
    "kap_kapitalertraege_partner",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
];
/// `RENTNER_22`.
pub(super) const RENTNER_22: [&str; 4] = [
    "rentner_renten_art",
    "rentner_jahresrente",
    "rentner_renten_beginn_jahr",
    "rentner_alter_bei_rentenbeginn",
];
/// `RENTNER_22_PARTNER`.
pub(super) const RENTNER_22_PARTNER: [&str; 4] = [
    "rentner_renten_art_partner",
    "rentner_jahresrente_partner",
    "rentner_renten_beginn_jahr_partner",
    "rentner_alter_bei_rentenbeginn_partner",
];
/// `RENTNER_AA_ARTEN`.
pub(super) const RENTNER_AA_ARTEN: [&str; 3] = [
    "gesetzliche_rente",
    "berufsstaendische_versorgung",
    "private_basisrente",
];
/// `STAMMDATEN_FELDER` (liest `api.einreichen`, nicht `bescheid_deklaration.py`).
pub(super) const STAMMDATEN_FELDER: [&str; 13] = [
    "stammdaten_nachname",
    "stammdaten_vorname",
    "stammdaten_geburtsdatum",
    "stammdaten_strasse",
    "stammdaten_hausnummer",
    "stammdaten_hausnummerzusatz",
    "stammdaten_plz",
    "stammdaten_wohnort",
    "stammdaten_keine_bankverbindung",
    "stammdaten_iban",
    "stammdaten_bic",
    "stammdaten_art_est_erklaerung",
    "stammdaten_steuernummer",
];
/// `UEBERNACHTUNG_BEDINGUNGEN`.
pub(super) const UEBERNACHTUNG_BEDINGUNGEN: [&str; 3] = [
    "uebernachtung_auswaerts",
    "uebernachtung_alleinnutzung",
    "uebernachtung_keine_lange_unterbrechung",
];
/// `VERPFLEGUNG_TAGE`.
pub(super) const VERPFLEGUNG_TAGE: [&str; 3] =
    ["tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig"];
/// `VERPFLEGUNG_TAGE_NACH_FRIST`.
pub(super) const VERPFLEGUNG_TAGE_NACH_FRIST: [&str; 3] = [
    "vpf_tage_24h_nach_drei_monaten",
    "vpf_tage_an_abreise_nach_drei_monaten",
    "vpf_tage_ueber_8h_nach_drei_monaten",
];
/// `VOR_FELDER`.
pub(super) const VOR_FELDER: [&str; 3] = [
    "vor_an_anteil_rv",
    "vor_ag_anteil_rv",
    "vor_rv_ausserhalb_lstb",
];
/// `VOR_PARTNER_FELDER`.
pub(super) const VOR_PARTNER_FELDER: [&str; 3] = [
    "vor_an_anteil_rv_partner",
    "vor_ag_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner",
];
/// `VV_GESAMT_FELDER`.
pub(super) const VV_GESAMT_FELDER: [&str; 6] = [
    "vv_einnahmen",
    "vv_gebaeude_afa",
    "vv_schuldzinsen",
    "vv_erhaltungsaufwand",
    "vv_sonstige_wk",
    "vv_entgelt_quote_prozent",
];

/// Alle Tabellen unter ihrem Python-Namen (fuer `konstanten_gleich`).
pub(super) fn alle() -> Vec<(&'static str, &'static [&'static str])> {
    vec![
        ("ARBEITSMITTEL_KOSTEN", &[ARBEITSMITTEL_KOSTEN]),
        ("DHF_KOSTEN", &[DHF_KOSTEN]),
        ("UEBERNACHTUNG_KOSTEN", &[UEBERNACHTUNG_KOSTEN]),
        ("AGB_KIST", &AGB_KIST),
        ("AN_GESAMT_FLAGS", &AN_GESAMT_FLAGS),
        ("AN_GESAMT_PARTNER", &AN_GESAMT_PARTNER),
        ("DHF_BEDINGUNGEN", &DHF_BEDINGUNGEN),
        ("GESAMT_PARTNER_19", &GESAMT_PARTNER_19),
        ("GESAMT_PARTNER_KAP", &GESAMT_PARTNER_KAP),
        ("RENTNER_22", &RENTNER_22),
        ("RENTNER_22_PARTNER", &RENTNER_22_PARTNER),
        ("RENTNER_AA_ARTEN", &RENTNER_AA_ARTEN),
        ("STAMMDATEN_FELDER", &STAMMDATEN_FELDER),
        ("UEBERNACHTUNG_BEDINGUNGEN", &UEBERNACHTUNG_BEDINGUNGEN),
        ("VERPFLEGUNG_TAGE", &VERPFLEGUNG_TAGE),
        ("VERPFLEGUNG_TAGE_NACH_FRIST", &VERPFLEGUNG_TAGE_NACH_FRIST),
        ("VOR_FELDER", &VOR_FELDER),
        ("VOR_PARTNER_FELDER", &VOR_PARTNER_FELDER),
        ("VV_GESAMT_FELDER", &VV_GESAMT_FELDER),
    ]
}

/// `RING_BETRAGSFELDER` (`api_constants.py:748`) je Scheibe geschnitten, wie
/// `_vorlaeufige_ring_betraege` es liest: Ring-Betragsfelder, die in `cfg["felder"]` stehen und
/// NICHT in `cfg["kegel"]`, in Reihenfolge von `RING_BETRAGSFELDER`.
/// ponytail: vorab geschnitten statt 119 + 350 + 248 Feldnamen zu tragen; die Schnittmenge je Scheibe
/// prueft `konstanten_gleich` gegen das Orakel.
pub(super) fn ring_kandidaten(scheibe: Scheibe) -> &'static [&'static str] {
    match scheibe {
        Scheibe::Ep | Scheibe::NVorGwg => &[],
        Scheibe::AnGesamt => &RING_AN_GESAMT,
        Scheibe::Gesamt => &RING_GESAMT,
        Scheibe::RentnerGesamt => &RING_RENTNER_GESAMT,
    }
}

/// Ring-Kandidaten der Scheibe (s. [`ring_kandidaten`]).
pub(super) const RING_AN_GESAMT: [&str; 29] = [
    "am_anschaffung_monat",
    "am_anschaffungskosten",
    "arbeitsmittel_nutzungsdauer",
    "basis_kv_partner",
    "basis_pv_partner",
    "bruttoarbeitslohn_partner",
    "ep_unfallkosten",
    "p36_lohnsteuer",
    "p36_vorauszahlungen",
    "uebernachtung_kosten_monat",
    "uebernachtung_monate",
    "uebernachtung_monate_bisher",
    "vor_ag_anteil_rv_partner",
    "vor_an_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner",
    "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit_partner",
    "vorsorge_rv_alt_mit_ueberschuss_partner",
    "vorsorge_rv_alt_ohne_ueberschuss_partner",
    "vorsorge_unfall_haftpflicht_partner",
    "vpf_abendessen_gestellt_anzahl",
    "vpf_fruehstuecke_gestellt_anzahl",
    "vpf_mahlzeiten_gezahltes_entgelt",
    "vpf_mittagessen_gestellt_anzahl",
    "vpf_monate_am_ort",
    "vpf_steuerfreie_erstattung_betrag",
    "vpf_tage_24h_nach_drei_monaten",
    "vpf_tage_an_abreise_nach_drei_monaten",
    "vpf_tage_ueber_8h_nach_drei_monaten",
];
/// Ring-Kandidaten der Scheibe (s. [`ring_kandidaten`]).
pub(super) const RING_GESAMT: [&str; 103] = [
    "afa_jahresbetrag",
    "agb_aufwendungen",
    "am_anschaffung_monat",
    "am_anschaffungskosten",
    "arbeitsmittel_nutzungsdauer",
    "basis_kv_partner",
    "basis_pv_partner",
    "behinderungsbedingte_aufwendungen",
    "behinderungsbedingte_aufwendungen_partner",
    "berufsausbildung_aufwendungen",
    "betriebseinnahmen",
    "bruttoarbeitslohn_partner",
    "dba_auslaendische_einkuenfte",
    "dba_gezahlte_auslaendische_steuer",
    "dhf_monate",
    "dhf_unterkunftskosten_monat",
    "einkuenfte_gewinn",
    "einkuenfte_gewinn_partner",
    "ep_unfallkosten",
    "fam_anzahl_kinder",
    "fam_monate_ohne_voraussetzung",
    "geburtsjahr",
    "geburtsjahr_partner",
    "gewinnanteil",
    "gewinnanteil_partner",
    "gewst_hebesatz",
    "gewst_hebesatz_partner",
    "gewst_messbetrag",
    "gewst_messbetrag_partner",
    "hh_dienstleistungen",
    "hh_handwerker_arbeitskosten",
    "hh_minijob_aufwendungen",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner",
    "kap_kapitalertraege_partner",
    "kap_q_auslaendische_steuer",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner",
    "kist_erstattet",
    "kist_gezahlt",
    "p22_nr3_einkuenfte",
    "p32b_progressionseinkuenfte",
    "p33a_andere_einkuenfte_bezuege",
    "p33a_ausbildung_anzahl_kinder",
    "p33a_unterhalt_aufwendungen",
    "p33a_unterhalt_kv_pv",
    "p35c_energieberater_aufwendungen",
    "p35c_sanierungsaufwendungen",
    "p36_kapitalertragsteuer",
    "p36_kapitalertragsteuer_kist",
    "p36_kapitalertragsteuer_solz",
    "p36_lohnsteuer",
    "p36_lohnsteuer_partner",
    "p36_vorauszahlungen",
    "parteispenden_betrag",
    "pv_anzahl_einheiten",
    "pv_bruttoleistung_kwp",
    "pv_einnahmen",
    "realsplitting_empfaenger_kv_krankengeld",
    "realsplitting_empfaenger_kv_pv",
    "realsplitting_unterhaltsleistungen",
    "rentner_grad_der_behinderung",
    "rentner_grad_der_behinderung_partner",
    "rentner_pflegegrad",
    "rentner_veraeusserungsgewinn",
    "rentner_veraeusserungsgewinn_partner",
    "sonstige_betriebsausgaben",
    "spenden_betrag",
    "tage_24h",
    "tage_an_abreise",
    "tage_ueber_8h_eintaegig",
    "uebernachtung_kosten_monat",
    "uebernachtung_monate",
    "uebernachtung_monate_bisher",
    "verguetung_darlehen",
    "verguetung_darlehen_partner",
    "verguetung_taetigkeit",
    "verguetung_taetigkeit_partner",
    "verguetung_ueberlassung",
    "verguetung_ueberlassung_partner",
    "verlustvortrag_bestand",
    "versorgung_alter_bei_beginn",
    "versorgung_beginn_jahr",
    "versorgung_bemessungsgrundlage",
    "versorgung_jahresrente",
    "vor_ag_anteil_rv_partner",
    "vor_an_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner",
    "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit_partner",
    "vorsorge_rv_alt_mit_ueberschuss_partner",
    "vorsorge_rv_alt_ohne_ueberschuss_partner",
    "vorsorge_unfall_haftpflicht_partner",
    "vpf_abendessen_gestellt_anzahl",
    "vpf_fruehstuecke_gestellt_anzahl",
    "vpf_mahlzeiten_gezahltes_entgelt",
    "vpf_mittagessen_gestellt_anzahl",
    "vpf_monate_am_ort",
    "vpf_steuerfreie_erstattung_betrag",
    "vpf_tage_24h_nach_drei_monaten",
    "vpf_tage_an_abreise_nach_drei_monaten",
    "vpf_tage_ueber_8h_nach_drei_monaten",
    "waehlervereinigungen_betrag",
];
/// Ring-Kandidaten der Scheibe (s. [`ring_kandidaten`]).
pub(super) const RING_RENTNER_GESAMT: [&str; 91] = [
    "afa_jahresbetrag",
    "agb_aufwendungen",
    "basis_kv_partner",
    "basis_pv_partner",
    "behinderungsbedingte_aufwendungen",
    "behinderungsbedingte_aufwendungen_partner",
    "berufsausbildung_aufwendungen",
    "betriebseinnahmen",
    "bruttoarbeitslohn",
    "bruttoarbeitslohn_partner",
    "dba_auslaendische_einkuenfte",
    "dba_gezahlte_auslaendische_steuer",
    "einkuenfte_gewinn",
    "einkuenfte_gewinn_partner",
    "fam_anzahl_kinder",
    "fam_monate_ohne_voraussetzung",
    "geburtsjahr",
    "geburtsjahr_partner",
    "gewinnanteil",
    "gewinnanteil_partner",
    "gewst_hebesatz",
    "gewst_hebesatz_partner",
    "gewst_messbetrag",
    "gewst_messbetrag_partner",
    "hh_dienstleistungen",
    "hh_handwerker_arbeitskosten",
    "hh_minijob_aufwendungen",
    "kap_gewinn_aktien",
    "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige",
    "kap_gewinn_sonstige_partner",
    "kap_kapitalertraege",
    "kap_kapitalertraege_partner",
    "kap_q_auslaendische_steuer",
    "kap_verlust_aktien",
    "kap_verlust_aktien_partner",
    "kap_verlust_sonstige",
    "kap_verlust_sonstige_partner",
    "kist_erstattet",
    "kist_gezahlt",
    "p22_nr3_einkuenfte",
    "p32b_progressionseinkuenfte",
    "p33a_andere_einkuenfte_bezuege",
    "p33a_ausbildung_anzahl_kinder",
    "p33a_unterhalt_aufwendungen",
    "p33a_unterhalt_kv_pv",
    "p35c_energieberater_aufwendungen",
    "p35c_sanierungsaufwendungen",
    "p36_kapitalertragsteuer",
    "p36_kapitalertragsteuer_kist",
    "p36_kapitalertragsteuer_solz",
    "p36_lohnsteuer",
    "p36_lohnsteuer_partner",
    "p36_vorauszahlungen",
    "parteispenden_betrag",
    "realsplitting_empfaenger_kv_krankengeld",
    "realsplitting_empfaenger_kv_pv",
    "realsplitting_unterhaltsleistungen",
    "rentner_alter_bei_rentenbeginn_partner",
    "rentner_grad_der_behinderung_partner",
    "rentner_jahresrente_partner",
    "rentner_renten_beginn_jahr_partner",
    "rentner_rentenfreibetrag_partner",
    "rentner_veraeusserungsgewinn",
    "rentner_veraeusserungsgewinn_partner",
    "sonstige_betriebsausgaben",
    "spenden_betrag",
    "verguetung_darlehen",
    "verguetung_darlehen_partner",
    "verguetung_taetigkeit",
    "verguetung_taetigkeit_partner",
    "verguetung_ueberlassung",
    "verguetung_ueberlassung_partner",
    "verlustvortrag_bestand",
    "versorgung_alter_bei_beginn",
    "versorgung_alter_bei_beginn_partner",
    "versorgung_beginn_jahr",
    "versorgung_beginn_jahr_partner",
    "versorgung_bemessungsgrundlage",
    "versorgung_bemessungsgrundlage_partner",
    "versorgung_jahresrente",
    "versorgung_jahresrente_partner",
    "vor_ag_anteil_rv_partner",
    "vor_an_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner",
    "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit_partner",
    "vorsorge_rv_alt_mit_ueberschuss_partner",
    "vorsorge_rv_alt_ohne_ueberschuss_partner",
    "vorsorge_unfall_haftpflicht_partner",
    "waehlervereinigungen_betrag",
];
