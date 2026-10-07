//! Die Scheibenlisten: welche Felder eine Scheibe traegt und welche davon der Kegel sind.
//!
//! **Von Hand gepflegte Rust-Quelle** (Weg B leicht/voll, Entscheidung 2026-10-05, Vault
//! `decisions/weg-b-leicht-und-art9-schutz-als-test.md`). Der Stand ist am 2026-10-05 aus `SCHEIBEN`
//! in `produkt/haut/api_constants.py` uebernommen; der Generator `tools/parity/gen_scheiben_tabellen.py`
//! und der Parity-Vergleich der Listen sind gestrichen. Python ist eingefroren und aendert sich nicht
//! mehr, ein Feld nur fuer Rust steht deshalb hier und in `rust/bindung/daten`.
//!
//! Die Feld-Tupel sind **nicht ableitbar**: kein `bindung_*.yaml` traegt einen
//! Scheiben-Schluessel, die Zuordnung Feld -> Scheibe steht nur hier.
//! `#[rustfmt::skip]` haelt das Layout (vier Namen je Zeile), damit ein Diff eine Zeile zeigt.
//!
//! Waechter: `rust/bescheid/tests/scheiben_tabellen_konsistenz.rs`. Wer ein Feld aufnimmt:
//!
//! 1. das Feld steht in der Registry (`rust/bindung/daten`),
//! 2. der Eintrag steht hier, ein Kegel-Feld auch in den Feldern der Scheibe und ist `askable`,
//! 3. die Laenge (und das Array hier) stimmt mit der Zahl im Konsistenztest ueberein.

/// `SCHEIBEN['ep']["felder"]` = `EP_FELDER + EP_FORMALIEN`
#[rustfmt::skip]
pub(super) const SCHEIBEN_EP_FELDER: [&str; 6] = [
    "ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz",
    "ep_ziel_des_weges", "ep_ziel_adresse",
];

/// `SCHEIBEN['ep']["kegel"]` = `EP_FELDER`
#[rustfmt::skip]
pub(super) const SCHEIBEN_EP_KEGEL: [&str; 4] = [
    "ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz",
];

// EINZIGE WAHRHEIT der vier `EP_FELDER`-Feldnamen in Rust. `EP_FELDER` selbst
// wird hier NICHT als eigener `const` gefuehrt: der Python-Name kommt in ganz
// `produkt/` nur noch in zwei Kommentaren vor (`bescheid_abzuege.py:36` Docstring,
// `bescheid_zweige.py:489` Kommentar), kein Code liest ihn. Die Feldnamen liegen
// verbatim in `SCHEIBEN_EP_KEGEL` und in `SCHEIBEN_N_VOR_GWG_TEIL_0_FELDER`.
// ponytail: zwei Kopien derselben vier Namen statt einer geteilten Konstanten.
// Deduplizieren spart vier Feldnamen und beruehrt zwei Konstanten. Ticket, nicht Bau.
// Wer die Namen aendert, aendert BEIDE Stellen.

// `SCHEIBEN['n_vor_gwg']["felder"]` ist `None` -- die Feldliste kommt zur
// Laufzeit aus `rust/bindung/daten/bindung_n_vor_gwg.yaml` (`Cfg::felder_datei`).

// `SCHEIBEN['n_vor_gwg']["kegel"]` ist `None` -- der Kegel ist der volle `felder`-Satz.

/// `SCHEIBEN['n_vor_gwg']["teil_ringe"][0]` = `ep_werbungskosten`: die Feldliste des Teil-Rings.
#[rustfmt::skip]
pub(super) const SCHEIBEN_N_VOR_GWG_TEIL_0_FELDER: [&str; 4] = [
    "ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz",
];

/// `SCHEIBEN['n_vor_gwg']["teil_ringe"]` -- `(familie, quantitaet, felder)`.
#[rustfmt::skip]
pub(super) const SCHEIBEN_N_VOR_GWG_TEIL_RINGE: [(&str, &str, &[&str]); 1] = [
    ("ep_werbungskosten", "abziehbarer_betrag", &SCHEIBEN_N_VOR_GWG_TEIL_0_FELDER),
];

/// `SCHEIBEN['an_gesamt']["felder"]` = `('bruttoarbeitslohn', 'veranlagung') + EP_FELDER + EP_FORMALIEN + VOR_FELDER + KV_PV_FELDER + DHF_RING + DHF_BEDINGUNGEN + DHF_AUSLANDSGRENZE + DHF_FORMALIEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_TAGE_NACH_FRIST + VERPFLEGUNG_GUARD + VERPFLEGUNG_FRIST + VERPFLEGUNG_KUERZUNG + UEBERNACHTUNG_RING + UEBERNACHTUNG_BEDINGUNGEN + ARBEITSMITTEL_RING + ARBEITSMITTEL_AFA_GESAMT + AN_GESAMT_FLAGS + AN_GESAMT_PARTNER + VOR_PARTNER_FELDER + KV_PV_PARTNER_FELDER + P36_ANRECHNUNG + KIST_KONFESSION_FELDER + P35A_MITVER_ANZEIGE + ('fam_anzahl_kinder', 'verlustvortrag_bestand')`
#[rustfmt::skip]
pub(super) const SCHEIBEN_AN_GESAMT_FELDER: [&str; 85] = [
    "bruttoarbeitslohn", "veranlagung", "ep_arbeitstage", "ep_entfernung_km",
    "ep_oepnv_kosten", "ep_eigenes_kfz", "ep_ziel_des_weges", "ep_ziel_adresse", "ep_unfallkosten",
    "vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb", "versicherungsart",
    "basis_kv", "basis_pv", "vorsorge_arbeitslosenversicherung", "vorsorge_erwerbsunfaehigkeit",
    "vorsorge_unfall_haftpflicht", "vorsorge_rv_alt_mit_ueberschuss", "vorsorge_rv_alt_ohne_ueberschuss", "mit_anspruch_auf_zuschuss",
    "dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland", "dhf_beruflich_veranlasst",
    "dhf_eigener_hausstand", "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung", "dhf_beschaeftigungsort",
    "dhf_grund", "dhf_begruendet_am", "dhf_bestanden_bis", "dhf_hausstand_plz_ort",
    "dhf_hausstand_seit", "tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig",
    "vpf_tage_24h_nach_drei_monaten", "vpf_tage_an_abreise_nach_drei_monaten", "vpf_tage_ueber_8h_nach_drei_monaten", "vpf_monate_am_ort",
    "vpf_keine_mahlzeitengestellung", "vpf_frist_nicht_unterbrochen", "vpf_fruehstuecke_gestellt_anzahl", "vpf_mittagessen_gestellt_anzahl",
    "vpf_abendessen_gestellt_anzahl", "vpf_mahlzeiten_gezahltes_entgelt", "vpf_steuerfreie_erstattung_betrag", "p9_4a_kuerzung_nach_entgelt",
    "uebernachtung_kosten_monat", "uebernachtung_monate", "uebernachtung_monate_bisher", "uebernachtung_im_inland",
    "uebernachtung_auswaerts", "uebernachtung_alleinnutzung", "uebernachtung_keine_lange_unterbrechung", "am_anschaffungskosten",
    "am_gwg_sofortabzug_gewaehlt", "arbeitsmittel_nutzungsdauer", "am_anschaffung_monat", "am_afa_ist_anschaffungsjahr",
    "kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige",
    "bruttoarbeitslohn_partner", "vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner", "vor_rv_ausserhalb_lstb_partner",
    "versicherungsart_partner", "basis_kv_partner", "basis_pv_partner", "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit_partner", "vorsorge_unfall_haftpflicht_partner", "vorsorge_rv_alt_mit_ueberschuss_partner", "vorsorge_rv_alt_ohne_ueberschuss_partner",
    "mit_anspruch_auf_zuschuss_partner", "p36_lohnsteuer", "p36_vorauszahlungen", "kist_konfession",
    "kist_bundesland", "p35a_mitveranlagung", "fam_anzahl_kinder", "verlustvortrag_bestand",
];

/// `SCHEIBEN['an_gesamt']["kegel"]` = `('bruttoarbeitslohn', 'veranlagung') + EP_FELDER + VOR_FELDER + KV_PV_FELDER + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + AN_GESAMT_FLAGS + ('fam_anzahl_kinder', 'verlustvortrag_bestand')`
#[rustfmt::skip]
pub(super) const SCHEIBEN_AN_GESAMT_KEGEL: [&str; 33] = [
    "bruttoarbeitslohn", "veranlagung", "ep_arbeitstage", "ep_entfernung_km",
    "ep_oepnv_kosten", "ep_eigenes_kfz", "vor_an_anteil_rv", "vor_ag_anteil_rv",
    "vor_rv_ausserhalb_lstb", "versicherungsart", "basis_kv", "basis_pv",
    "vorsorge_arbeitslosenversicherung", "vorsorge_erwerbsunfaehigkeit", "vorsorge_unfall_haftpflicht", "vorsorge_rv_alt_mit_ueberschuss",
    "vorsorge_rv_alt_ohne_ueberschuss", "mit_anspruch_auf_zuschuss", "dhf_unterkunftskosten_monat", "dhf_monate",
    "dhf_im_inland", "dhf_beruflich_veranlasst", "dhf_eigener_hausstand", "dhf_finanzielle_beteiligung",
    "tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig", "kein_gewinn",
    "kein_kap", "kein_vuv", "kein_sonstige", "fam_anzahl_kinder",
    "verlustvortrag_bestand",
];

/// `SCHEIBEN['gesamt']["felder"]` = `VV_GESAMT_FELDER + VV_ABS2_TATBESTAND + ('veranlagung', 'bruttoarbeitslohn') + EP_FELDER + EP_FORMALIEN + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + KAP_ANTRAG_FELDER + P36_ANRECHNUNG_KAP + P32D_Q_KAP + AN_GESAMT_FLAGS + GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP + VORSORGE_PARTNER_FELDER + GESAMT_VERSORGUNG + GESAMT_ABZUEGE + GESAMT_FREIBETRAEGE + GESAMT_GEWINN + GESAMT_GEWINN_PARTNER + GESAMT_33B + GESAMT_33B_PARTNER + KIND_SCREENING + AUSGABEN_SCREENING + PARTNER_SCREENING + INSTANZ_ZAEHLFELDER + VV_ANLAGE_FORMALIEN + GESAMT_DBA + GESAMT_P23 + P23_SCREENING + P22_NR3_EINKUENFTE + GESAMT_P33A + GESAMT_P32B + GESAMT_P35C + GESAMT_REALSPLITTING + DHF_RING + DHF_BEDINGUNGEN + DHF_AUSLANDSGRENZE + DHF_FORMALIEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_TAGE_NACH_FRIST + VERPFLEGUNG_GUARD + VERPFLEGUNG_FRIST + VERPFLEGUNG_KUERZUNG + VERPFLEGUNG_EINZELREISE + AGB_TATBESTAND + UEBERNACHTUNG_RING + UEBERNACHTUNG_BEDINGUNGEN + ARBEITSMITTEL_RING + ARBEITSMITTEL_AFA_GESAMT + P36_ANRECHNUNG + P36_ANRECHNUNG_PARTNER + KIST_KONFESSION_FELDER + KIRCHENSTEUER_ARBEITGEBER_FELDER + P16_4_GATE_FELDER + P16_4_GATE_FELDER_PARTNER + STEUERKLASSE_FELDER + STAMMDATEN_FELDER + STAMMDATEN_FELDER_PARTNER`
#[rustfmt::skip]
pub(super) const SCHEIBEN_GESAMT_FELDER: [&str; 364] = [
    "vv_einnahmen", "vv_gebaeude_afa", "vv_schuldzinsen", "vv_erhaltungsaufwand",
    "vv_sonstige_wk", "vv_entgelt_quote_prozent", "vv_wohnzwecke", "vv_auf_dauer",
    "veranlagung", "bruttoarbeitslohn", "ep_arbeitstage", "ep_entfernung_km",
    "ep_oepnv_kosten", "ep_eigenes_kfz", "ep_ziel_des_weges", "ep_ziel_adresse", "ep_unfallkosten",
    "vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb", "versicherungsart",
    "basis_kv", "basis_pv", "vorsorge_arbeitslosenversicherung", "vorsorge_erwerbsunfaehigkeit",
    "vorsorge_unfall_haftpflicht", "vorsorge_rv_alt_mit_ueberschuss", "vorsorge_rv_alt_ohne_ueberschuss", "mit_anspruch_auf_zuschuss",
    "kap_kapitalertraege", "kap_gewinn_aktien", "kap_verlust_aktien", "kap_gewinn_sonstige",
    "kap_verlust_sonstige", "kap_antrag_guenstigerpruefung", "kap_sparer_pauschbetrag_genutzt", "p36_kapitalertragsteuer",
    "p36_kapitalertragsteuer_solz", "p36_kapitalertragsteuer_kist", "kap_q_auslaendische_steuer", "kein_gewinn",
    "kein_kap", "kein_vuv", "kein_sonstige", "bruttoarbeitslohn_partner",
    "kap_kapitalertraege_partner", "kap_gewinn_aktien_partner", "kap_gewinn_sonstige_partner", "kap_verlust_aktien_partner",
    "kap_verlust_sonstige_partner", "vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner", "vor_rv_ausserhalb_lstb_partner",
    "versicherungsart_partner", "basis_kv_partner", "basis_pv_partner", "vorsorge_arbeitslosenversicherung_partner",
    "vorsorge_erwerbsunfaehigkeit_partner", "vorsorge_unfall_haftpflicht_partner", "vorsorge_rv_alt_mit_ueberschuss_partner", "vorsorge_rv_alt_ohne_ueberschuss_partner",
    "mit_anspruch_auf_zuschuss_partner", "geburtsjahr_partner", "versorgung_jahresrente", "versorgung_bemessungsgrundlage",
    "versorgung_beginn_jahr", "versorgung_art", "versorgung_alter_bei_beginn", "hh_hat_aufwendungen",
    "hh_minijob_aufwendungen", "hh_dienstleistungen", "hh_handwerker_arbeitskosten", "hh_minijob_betrag",
    "hh_minijob_art", "hh_dienstleistung_betrag", "hh_dienstleistung_art", "hh_handwerker_betrag",
    "hh_handwerker_art", "hh_in_eu_ewr", "hh_handwerker_keine_foerderung", "hh_rechnung_unbar",
    "spenden_betrag", "parteispenden_betrag", "spenden_vermoegensstock", "agb_aufwendungen", "fam_anzahl_kinder",
    "berufsausbildung_aufwendungen", "berufsausbildung_bezeichnung", "berufsausbildung_einzelbetrag", "kist_gezahlt",
    "kist_erstattet", "kinderbetreuungskosten", "kind_unter_14_haushaltszugehoerig", "kind_betreuung_reine_betreuung",
    "kind_betreuung_rechnung_ueberweisung", "kind_betreuung_dienstleister", "kind_betreuung_zeitraum", "kind_betreuung_eigenanteil",
    "kind_betreuung_kein_gemeinsamer_haushalt_zeitraum", "kind_betreuung_haushaltszugehoerigkeit_zeitraum", "kind_betreuung_einzelbetrag", "kind_betreuung_eigenanteil_betrag",
    "kind_betreuung_eigenanteil_zeitraum", "schulgeld", "kind_schulgeld_aufteilung_prozent",
    "kind_kv", "kind_pv", "kind_idnr", "kind_vorname", "kind_kindschaftsverhaeltnis_a", "kind_kindschaftsverh_zeitraum_a",
    "kind_geburtsdatum", "kind_familienkasse", "kind_wohnsitz_inland_zeitraum", "kind_kindschaftsverhaeltnis_b",
    "kind_kindschaftsverh_zeitraum_b", "kind_anderer_elternteil_name", "kind_anderer_elternteil_geburtsdatum", "kind_anderer_elternteil_kindschaftsverhaeltnis",
    "kind_anderer_elternteil_zeitraum", "kind_anderer_elternteil_tod_am",
    "kind_anderer_elternteil_ausland_zeitraum", "kind_grad_der_behinderung", "kind_hilflos_blind_taubblind", "kind_hinterbliebenen_uebertragung",
    "kind_behinderten_pb_antrag", "kind_pb_nicht_selbst_genutzt", "behinderungsbedingte_aufwendungen", "behinderungsbedingte_aufwendungen_wahlrecht_pb",
    "behinderungsbedingte_aufwendungen_partner", "behinderungsbedingte_aufwendungen_wahlrecht_pb_partner", "fahrtkosten_pausch_gdb80_oder_70g", "fahrtkosten_pausch_ag_bl_tbl_h",
    "geburtsjahr", "fam_alleinstehend", "fam_monate_ohne_voraussetzung", "p35a_mitveranlagung",
    "einkuenfte_gewinn", "gewinn_betriebsart", "gewinn_bezeichnung", "betriebseinnahmen",
    "sonstige_betriebsausgaben", "afa_jahresbetrag", "gwg_anschaffungskosten_netto", "gwg_bewegliches_selbstaendig_nutzbar",
    "gwg_netto_ohne_vorsteuer", "gwg_ohne_vorsteuerabzug", "gwg_verzeichnis_ab_250", "rentner_veraeusserungsgewinn", "rentner_veraeusserungs_betriebsart",
    "p34_abs3_antragsbetrag", "gewst_hebesatz", "gewst_messbetrag", "gewst_zu_zahlen",
    "verlustvortrag_bestand", "gewinnanteil", "verguetung_taetigkeit", "verguetung_darlehen",
    "verguetung_ueberlassung", "antrag_ermaessigter_satz", "dauernd_berufsunfaehig", "ermaessigung_einmal_genutzt",
    "alter_55_vor_verkauf",
    "pv_einnahmen", "pv_bruttoleistung_kwp", "pv_anzahl_einheiten", "pv_auf_gebaeude",
    "einkuenfte_gewinn_partner", "gewinn_betriebsart_partner", "gewinn_bezeichnung_partner", "rentner_veraeusserungsgewinn_partner",
    "rentner_veraeusserungs_betriebsart_partner", "gewst_hebesatz_partner", "gewst_messbetrag_partner", "gewst_zu_zahlen_partner",
    "gewinnanteil_partner", "verguetung_taetigkeit_partner", "verguetung_darlehen_partner", "verguetung_ueberlassung_partner",
    "antrag_ermaessigter_satz_partner", "dauernd_berufsunfaehig_partner", "ermaessigung_einmal_genutzt_partner", "alter_55_vor_verkauf_partner",
    "p34_abs3_antragsbetrag_partner",
    "rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind", "rentner_hinterbliebenenbezuege", "rentner_pflegegrad",
    "rentner_gepflegter_hilflos", "rentner_gepflegter_wohnsitz_inland", "rentner_pflege_durch", "rentner_gepflegter_idnr",
    "rentner_gepflegter_angaben", "rentner_pflege_weitere_personen", "rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner",
    "kein_kind", "kein_unterhalt", "keine_auslandseinkuenfte", "keine_behinderung_pflege",
    "keine_versorgungsbezuege", "keine_energetische_sanierung", "keine_arbeitsmittel", "kein_realsplitting",
    "keine_spenden", "keine_berufsausbildung", "kein_verlustvortrag", "keine_lohnersatzleistungen",
    "keine_zweitwohnung", "vpf_auswaertige_taetigkeit", "kein_kap_partner", "kein_gewinn_partner",
    "kein_sonstige_partner", "keine_behinderung_pflege_partner", "vv_anzahl_objekte", "rentner_anzahl_renten",
    "p23_anzahl_verkaeufe", "hh_anzahl_handwerker", "hh_anzahl_dienstleistungen", "hh_anzahl_minijobs",
    "gwg_anzahl", "vv_objekt_strasse", "vv_objekt_plz", "vv_objekt_ort",
    "vv_wohneinheit_bezeichnung", "vv_nebenkosten_nicht_vereinbart", "vv_nebenkosten_umgelegt", "vv_mieteinnahmen_summe",
    "vv_nutzung_ferienwohnung", "vv_nutzung_an_angehoerige", "vv_nutzung_kurzfristig", "vv_einnahmen_summe_gesamt",
    "vv_summe_werbungskosten", "vv_ueberschuss", "vv_ueberschuss_person_a", "dba_staat",
    "dba_methode", "dba_einkunftsart", "dba_mehrere_staaten", "dba_gezahlte_auslaendische_steuer",
    "dba_auslaendische_einkuenfte", "dba_abzug_statt_anrechnung", "p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten",
    "p23_werbungskosten", "p23_veraeusserungs_typ", "kein_p23_verkauf", "p22_nr3_einkuenfte",
    "p22_nr3_einnahmen", "p22_nr3_einnahmen_art", "p22_nr3_einnahmen_einzelbetrag", "p22_nr3_werbungskosten",
    "p33a_unterhalt_aufwendungen", "p33a_unterhalt_kv_pv", "p33a_andere_einkuenfte_bezuege", "p33a_ausbildung_anzahl_kinder",
    "p33a_person_name", "p33a_person_beruf_familienstand", "p33a_person_geburtsdatum", "p33a_haushalt_anschrift",
    "p33a_haushalt_personenzahl", "p33a_unterstuetzungszeitraum", "p33a_zahlungszeitraum", "p33a_person_hat_einkuenfte",
    "p33a_person_hat_vermoegen", "p33a_weitere_person_beteiligt", "p33a_person_im_inlaendischen_haushalt", "p33a_kindergeld_anspruch",
    "p33a_verwandtschaftsverhaeltnis", "p33a_person_idnr", "p32b_progressionseinkuenfte", "p35c_sanierungsaufwendungen",
    "p35c_ist_uebernaechstes_foerderjahr", "p35c_keine_doppelfoerderung", "p35c_objekt_strasse", "p35c_objekt_plz_ort",
    "p35c_gebaeude_herstellungsbeginn", "p35c_baubeginn_massnahme", "p35c_gesamtflaeche_qm", "p35c_eigene_wohnflaeche_qm",
    "p35c_bereits_ermaessigung_frueher", "p35c_foerderung_in_anspruch", "p35c_massnahme_art", "p35c_massnahme_einzelbetrag",
    "p35c_energieberater_aufwendungen", "realsplitting_unterhaltsleistungen", "realsplitting_empfaenger_kv_pv", "realsplitting_empfaenger_kv_krankengeld",
    "realsplitting_zustimmung", "dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland",
    "dhf_beruflich_veranlasst", "dhf_eigener_hausstand", "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung",
    "dhf_beschaeftigungsort", "dhf_grund", "dhf_begruendet_am", "dhf_bestanden_bis",
    "dhf_hausstand_plz_ort", "dhf_hausstand_seit", "tage_24h", "tage_an_abreise",
    "tage_ueber_8h_eintaegig", "vpf_tage_24h_nach_drei_monaten", "vpf_tage_an_abreise_nach_drei_monaten", "vpf_tage_ueber_8h_nach_drei_monaten",
    "vpf_monate_am_ort", "vpf_keine_mahlzeitengestellung", "vpf_frist_nicht_unterbrochen", "vpf_fruehstuecke_gestellt_anzahl",
    "vpf_mittagessen_gestellt_anzahl", "vpf_abendessen_gestellt_anzahl", "vpf_mahlzeiten_gezahltes_entgelt", "vpf_steuerfreie_erstattung_betrag",
    "p9_4a_kuerzung_nach_entgelt", "vpf_abwesenheit_stunden", "vpf_an_oder_abreisetag", "vpf_mit_uebernachtung",
    "agb_zwangslaeufig", "agb_notwendig_angemessen", "uebernachtung_kosten_monat", "uebernachtung_monate",
    "uebernachtung_monate_bisher", "uebernachtung_im_inland", "uebernachtung_auswaerts", "uebernachtung_alleinnutzung",
    "uebernachtung_keine_lange_unterbrechung", "am_anschaffungskosten", "am_gwg_sofortabzug_gewaehlt", "arbeitsmittel_nutzungsdauer",
    "am_anschaffung_monat", "am_afa_ist_anschaffungsjahr", "p36_lohnsteuer", "p36_vorauszahlungen",
    "p36_lohnsteuer_partner", "kist_konfession", "kist_bundesland", "kirchensteuer_arbeitgeber",
    "kirchensteuer_arbeitgeber_partner", "rentner_alter_55_oder_berufsunfaehig", "rentner_freibetrag_erstmalig", "rentner_alter_55_oder_berufsunfaehig_partner",
    "rentner_freibetrag_erstmalig_partner", "steuerklasse", "steuerklasse_partner", "stammdaten_nachname",
    "stammdaten_vorname", "stammdaten_geburtsdatum", "stammdaten_strasse", "stammdaten_hausnummer",
    "stammdaten_hausnummerzusatz", "stammdaten_plz", "stammdaten_wohnort", "stammdaten_keine_bankverbindung", "stammdaten_iban",
    "stammdaten_bic", "stammdaten_art_est_erklaerung", "stammdaten_steuernummer", "stammdaten_nachname_partner",
    "stammdaten_vorname_partner", "stammdaten_geburtsdatum_partner", "kist_konfession_partner",
];

/// `SCHEIBEN['gesamt']["kegel"]` = `VV_GESAMT_FELDER + ('veranlagung', 'bruttoarbeitslohn') + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS + AGB_TATBESTAND`
#[rustfmt::skip]
pub(super) const SCHEIBEN_GESAMT_KEGEL: [&str; 35] = [
    "vv_einnahmen", "vv_gebaeude_afa", "vv_schuldzinsen", "vv_erhaltungsaufwand",
    "vv_sonstige_wk", "vv_entgelt_quote_prozent", "veranlagung", "bruttoarbeitslohn",
    "ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz",
    "vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb", "versicherungsart",
    "basis_kv", "basis_pv", "vorsorge_arbeitslosenversicherung", "vorsorge_erwerbsunfaehigkeit",
    "vorsorge_unfall_haftpflicht", "vorsorge_rv_alt_mit_ueberschuss", "vorsorge_rv_alt_ohne_ueberschuss", "mit_anspruch_auf_zuschuss",
    "kap_kapitalertraege", "kap_gewinn_aktien", "kap_verlust_aktien", "kap_gewinn_sonstige",
    "kap_verlust_sonstige", "kein_gewinn", "kein_kap", "kein_vuv",
    "kein_sonstige", "agb_zwangslaeufig", "agb_notwendig_angemessen",
];

/// `SCHEIBEN['rentner_gesamt']["felder"]` = `RENTNER_FELDER + KAP_FELDER + KAP_ANTRAG_FELDER + P36_ANRECHNUNG_KAP + P32D_Q_KAP + GESAMT_PARTNER_KAP + AGB_TATBESTAND`
/// plus, nur in Rust (2026-10-06, Vault `rentner-ring-liest-versorgungsbezuege-vor-der-scheibe`), die sieben letzten Namen:
/// `bruttoarbeitslohn` und `steuerklasse` (die zwei Pflichtfelder der Lohnsteuer-Gruppe) und `GESAMT_VERSORGUNG`. Der
/// Rentner-Ring liest sie (`zweige/rentner.rs`); sie sind keine Kegel-Felder, die Scheibe bekommt keine neue Pflichtfrage.
/// Und die sieben allerletzten, ebenfalls nur in Rust (2026-10-07, Abweichung Nr. 33, Vault `rentner-ehegatte-lohn-und-
/// versorgung-wird-gefragt-nicht-gesperrt`): Lohn, Steuerklasse und die fuenf Versorgungsfelder des Ehegatten, bei
/// Zusammenveranlagung. Der Ring liest Lohn und Versorgung (`zweige/gesamt.rs::einkuenfte_ns_aus_lohn`, Person B); der
/// Kegel bleibt bei 28.
#[rustfmt::skip]
pub(super) const SCHEIBEN_RENTNER_GESAMT_FELDER: [&str; 274] = [
    "rentner_renten_art", "rentner_jahresrente", "rentner_renten_beginn_jahr", "rentner_alter_bei_rentenbeginn",
    "rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind", "rentner_pflegegrad", "rentner_gepflegter_hilflos",
    "rentner_hinterbliebenenbezuege", "veranlagung", "kein_gewinn", "kein_kap",
    "kein_vuv", "kein_sonstige", "vor_an_anteil_rv", "vor_ag_anteil_rv",
    "vor_rv_ausserhalb_lstb", "versicherungsart", "basis_kv", "basis_pv",
    "vorsorge_arbeitslosenversicherung", "vorsorge_erwerbsunfaehigkeit", "vorsorge_unfall_haftpflicht", "vorsorge_rv_alt_mit_ueberschuss",
    "vorsorge_rv_alt_ohne_ueberschuss", "mit_anspruch_auf_zuschuss", "kein_p23_verkauf", "rentner_gepflegter_wohnsitz_inland",
    "rentner_pflege_durch", "rentner_gepflegter_idnr", "rentner_gepflegter_angaben", "rentner_pflege_weitere_personen",
    "rentner_rentenfreibetrag", "rentner_rentenfreibetrag_partner", "rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner",
    "rentner_renten_art_partner", "rentner_jahresrente_partner", "rentner_renten_beginn_jahr_partner", "rentner_alter_bei_rentenbeginn_partner",
    "einkuenfte_gewinn", "gewinn_bezeichnung", "rentner_veraeusserungsgewinn", "rentner_veraeusserungs_betriebsart",
    "p34_abs3_antragsbetrag", "gewinn_betriebsart", "betriebseinnahmen",
    "sonstige_betriebsausgaben", "afa_jahresbetrag", "gwg_anschaffungskosten_netto", "gwg_bewegliches_selbstaendig_nutzbar",
    "gwg_netto_ohne_vorsteuer", "gwg_ohne_vorsteuerabzug", "gwg_verzeichnis_ab_250", "gewinnanteil", "verguetung_taetigkeit",
    "verguetung_darlehen", "verguetung_ueberlassung", "antrag_ermaessigter_satz", "dauernd_berufsunfaehig",
    "ermaessigung_einmal_genutzt", "alter_55_vor_verkauf", "gewst_hebesatz", "gewst_messbetrag", "gewst_zu_zahlen",
    "verlustvortrag_bestand", "hh_hat_aufwendungen", "hh_minijob_aufwendungen", "hh_dienstleistungen",
    "hh_handwerker_arbeitskosten", "hh_minijob_betrag", "hh_minijob_art", "hh_dienstleistung_betrag",
    "hh_dienstleistung_art", "hh_handwerker_betrag", "hh_handwerker_art", "hh_in_eu_ewr",
    "hh_handwerker_keine_foerderung", "hh_rechnung_unbar", "spenden_betrag", "parteispenden_betrag", "spenden_vermoegensstock",
    "agb_aufwendungen", "fam_anzahl_kinder", "berufsausbildung_aufwendungen", "berufsausbildung_bezeichnung",
    "berufsausbildung_einzelbetrag", "kist_gezahlt", "kist_erstattet", "kinderbetreuungskosten",
    "kind_unter_14_haushaltszugehoerig", "kind_betreuung_reine_betreuung", "kind_betreuung_rechnung_ueberweisung", "kind_betreuung_dienstleister",
    "kind_betreuung_zeitraum", "kind_betreuung_eigenanteil", "kind_betreuung_kein_gemeinsamer_haushalt_zeitraum", "kind_betreuung_haushaltszugehoerigkeit_zeitraum",
    "kind_betreuung_einzelbetrag", "kind_betreuung_eigenanteil_betrag", "kind_betreuung_eigenanteil_zeitraum", "schulgeld",
    "kind_schulgeld_aufteilung_prozent", "kind_kv", "kind_pv", "kind_idnr", "kind_vorname",
    "kind_kindschaftsverhaeltnis_a", "kind_kindschaftsverh_zeitraum_a", "kind_geburtsdatum", "kind_familienkasse",
    "kind_wohnsitz_inland_zeitraum", "kind_kindschaftsverhaeltnis_b", "kind_kindschaftsverh_zeitraum_b", "kind_anderer_elternteil_name",
    "kind_anderer_elternteil_geburtsdatum", "kind_anderer_elternteil_kindschaftsverhaeltnis", "kind_anderer_elternteil_zeitraum",
    "kind_anderer_elternteil_tod_am", "kind_anderer_elternteil_ausland_zeitraum", "kind_grad_der_behinderung",
    "kind_hilflos_blind_taubblind", "kind_hinterbliebenen_uebertragung", "kind_behinderten_pb_antrag", "kind_pb_nicht_selbst_genutzt",
    "behinderungsbedingte_aufwendungen", "behinderungsbedingte_aufwendungen_wahlrecht_pb", "behinderungsbedingte_aufwendungen_partner", "behinderungsbedingte_aufwendungen_wahlrecht_pb_partner",
    "fahrtkosten_pausch_gdb80_oder_70g", "fahrtkosten_pausch_ag_bl_tbl_h", "geburtsjahr", "fam_alleinstehend",
    "fam_monate_ohne_voraussetzung", "p35a_mitveranlagung", "dba_staat", "dba_methode",
    "dba_einkunftsart", "dba_mehrere_staaten", "dba_gezahlte_auslaendische_steuer", "dba_auslaendische_einkuenfte",
    "dba_abzug_statt_anrechnung", "p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten", "p23_werbungskosten",
    "p23_veraeusserungs_typ", "p33a_unterhalt_aufwendungen", "p33a_unterhalt_kv_pv", "p33a_andere_einkuenfte_bezuege",
    "p33a_ausbildung_anzahl_kinder", "p33a_person_name", "p33a_person_beruf_familienstand", "p33a_person_geburtsdatum",
    "p33a_haushalt_anschrift", "p33a_haushalt_personenzahl", "p33a_unterstuetzungszeitraum", "p33a_zahlungszeitraum",
    "p33a_person_hat_einkuenfte", "p33a_person_hat_vermoegen", "p33a_weitere_person_beteiligt", "p33a_person_im_inlaendischen_haushalt",
    "p33a_kindergeld_anspruch", "p33a_verwandtschaftsverhaeltnis", "p33a_person_idnr", "p32b_progressionseinkuenfte",
    "p35c_sanierungsaufwendungen", "p35c_ist_uebernaechstes_foerderjahr", "p35c_keine_doppelfoerderung", "p35c_objekt_strasse",
    "p35c_objekt_plz_ort", "p35c_gebaeude_herstellungsbeginn", "p35c_baubeginn_massnahme", "p35c_gesamtflaeche_qm",
    "p35c_eigene_wohnflaeche_qm", "p35c_bereits_ermaessigung_frueher", "p35c_foerderung_in_anspruch", "p35c_massnahme_art",
    "p35c_massnahme_einzelbetrag", "p35c_energieberater_aufwendungen", "realsplitting_unterhaltsleistungen", "realsplitting_empfaenger_kv_pv",
    "realsplitting_empfaenger_kv_krankengeld", "realsplitting_zustimmung", "p36_lohnsteuer", "p36_vorauszahlungen",
    "kist_konfession", "kist_bundesland", "p22_nr3_einkuenfte", "p22_nr3_einnahmen",
    "p22_nr3_einnahmen_art", "p22_nr3_einnahmen_einzelbetrag", "p22_nr3_werbungskosten", "rentner_alter_55_oder_berufsunfaehig",
    "rentner_freibetrag_erstmalig", "versicherungsart_partner", "basis_kv_partner", "basis_pv_partner",
    "vorsorge_arbeitslosenversicherung_partner", "vorsorge_erwerbsunfaehigkeit_partner", "vorsorge_unfall_haftpflicht_partner", "vorsorge_rv_alt_mit_ueberschuss_partner",
    "vorsorge_rv_alt_ohne_ueberschuss_partner", "mit_anspruch_auf_zuschuss_partner", "vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner",
    "vor_rv_ausserhalb_lstb_partner", "stammdaten_nachname", "stammdaten_vorname", "stammdaten_geburtsdatum",
    "stammdaten_strasse", "stammdaten_hausnummer", "stammdaten_hausnummerzusatz", "stammdaten_plz", "stammdaten_wohnort",
    "stammdaten_keine_bankverbindung", "stammdaten_iban", "stammdaten_bic", "stammdaten_art_est_erklaerung",
    "stammdaten_steuernummer", "stammdaten_nachname_partner", "stammdaten_vorname_partner", "stammdaten_geburtsdatum_partner",
    "kist_konfession_partner", "einkuenfte_gewinn_partner", "gewinn_betriebsart_partner", "gewinn_bezeichnung_partner",
    "rentner_veraeusserungsgewinn_partner", "rentner_veraeusserungs_betriebsart_partner", "gewst_hebesatz_partner", "gewst_messbetrag_partner",
    "gewst_zu_zahlen_partner", "gewinnanteil_partner", "verguetung_taetigkeit_partner", "verguetung_darlehen_partner",
    "verguetung_ueberlassung_partner", "antrag_ermaessigter_satz_partner", "dauernd_berufsunfaehig_partner", "ermaessigung_einmal_genutzt_partner",
    "alter_55_vor_verkauf_partner", "p34_abs3_antragsbetrag_partner", "rentner_alter_55_oder_berufsunfaehig_partner", "rentner_freibetrag_erstmalig_partner", "kap_kapitalertraege",
    "kap_gewinn_aktien", "kap_verlust_aktien", "kap_gewinn_sonstige", "kap_verlust_sonstige",
    "kap_antrag_guenstigerpruefung", "kap_sparer_pauschbetrag_genutzt", "p36_kapitalertragsteuer", "p36_kapitalertragsteuer_solz",
    "p36_kapitalertragsteuer_kist", "kap_q_auslaendische_steuer", "kap_kapitalertraege_partner", "kap_gewinn_aktien_partner",
    "kap_gewinn_sonstige_partner", "kap_verlust_aktien_partner", "kap_verlust_sonstige_partner", "agb_zwangslaeufig",
    "agb_notwendig_angemessen", "bruttoarbeitslohn", "steuerklasse", "versorgung_jahresrente",
    "versorgung_bemessungsgrundlage", "versorgung_beginn_jahr", "versorgung_art", "versorgung_alter_bei_beginn",
    "bruttoarbeitslohn_partner", "steuerklasse_partner", "versorgung_jahresrente_partner", "versorgung_bemessungsgrundlage_partner",
    "versorgung_beginn_jahr_partner", "versorgung_art_partner", "versorgung_alter_bei_beginn_partner",
];

/// `SCHEIBEN['rentner_gesamt']["kegel"]` = `RENTNER_KEGEL + AGB_TATBESTAND`
#[rustfmt::skip]
pub(super) const SCHEIBEN_RENTNER_GESAMT_KEGEL: [&str; 28] = [
    "rentner_renten_art", "rentner_jahresrente", "rentner_renten_beginn_jahr", "rentner_alter_bei_rentenbeginn",
    "rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind", "rentner_pflegegrad", "rentner_gepflegter_hilflos",
    "rentner_hinterbliebenenbezuege", "veranlagung", "kein_gewinn", "kein_kap",
    "kein_vuv", "kein_sonstige", "vor_an_anteil_rv", "vor_ag_anteil_rv",
    "vor_rv_ausserhalb_lstb", "versicherungsart", "basis_kv", "basis_pv",
    "vorsorge_arbeitslosenversicherung", "vorsorge_erwerbsunfaehigkeit", "vorsorge_unfall_haftpflicht", "vorsorge_rv_alt_mit_ueberschuss",
    "vorsorge_rv_alt_ohne_ueberschuss", "mit_anspruch_auf_zuschuss", "agb_zwangslaeufig", "agb_notwendig_angemessen",
];
