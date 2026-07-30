"""Paket-B Haut — modul-level Konstanten (Feld-Tupel, Scheiben-Konfiguration).

Pure data: Tupel + Dict für Feld-Aggregationen, Scheiben-Routing, DBA-Methoden.
Keine Funktionen, keine Abhängigkeiten außer os/sys für Pfade.
"""
from __future__ import annotations

import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
FAELLE = os.path.join(HERE, "faelle")

# ========== Private (führender Unterstrich) Konstanten — müssen in __all__ sein für import * ==========
_FALL_RE = re.compile(r"[A-Za-z0-9_-]{1,64}")
_AUTH_USER: str | None = None
_ERLAUBTE_ZUSTAENDE = {"vorlaeufig", "bestaetigt"}

# ========== § 19 Einkünfte (Arbeit) ==========
EP_FELDER = ("ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz")

# ========== an_gesamt MVP + Flags ==========
AN_GESAMT_FLAGS = ("kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige")
AN_GESAMT_PARTNER = ("bruttoarbeitslohn_partner", "person_b_idnr")

# ========== § 9 Arbeitsmittel (GWG) ==========
ARBEITSMITTEL_KOSTEN = "am_anschaffungskosten"
ARBEITSMITTEL_RING = ("am_anschaffungskosten", "am_gwg_sofortabzug_gewaehlt", "arbeitsmittel_nutzungsdauer")

# ========== § 36 Abs. 2 Anrechnung (LSt + Vorauszahlungen) ==========
P36_ANRECHNUNG = ("p36_lohnsteuer", "p36_vorauszahlungen")

# ========== § 22 Sonstige Einkünfte + § 10 KiSt ==========
P22_NR3_EINKUENFTE = ("p22_nr3_einkuenfte",)
KIST_KONFESSION_FELDER = ("kist_konfession", "kist_bundesland")

# ========== § 16 Abs. 4 Freibetrag-Gates (Rentner) ==========
P16_4_GATE_FELDER = ("rentner_alter_55_oder_berufsunfaehig", "rentner_freibetrag_erstmalig")

# ========== § 9 Abs. 4a Verpflegung ==========
VERPFLEGUNG_TAGE = ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig")
VERPFLEGUNG_GUARD = ("vpf_monate_am_ort", "vpf_keine_mahlzeitengestellung")

# ========== § 10 Vorsorge ==========
VOR_FELDER = ("vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb")
VOR_PARTNER_FELDER = ("vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner",
                      "vor_rv_ausserhalb_lstb_partner")
KV_PV_FELDER = ("basis_kv_pv", "weitere_vorsorgeaufwendungen", "mit_anspruch_auf_zuschuss")
KV_PV_PARTNER_FELDER = ("basis_kv_pv_partner", "weitere_vorsorgeaufwendungen_partner",
                        "mit_anspruch_auf_zuschuss_partner")
VORSORGE_PARTNER_FELDER = VOR_PARTNER_FELDER + KV_PV_PARTNER_FELDER + ("geburtsjahr_partner",)

# ========== § 9 Abs. 1 S. 3 Nr. 5 Doppelte Haushaltsführung ==========
DHF_KOSTEN = "dhf_unterkunftskosten_monat"
DHF_RING = ("dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland")
DHF_BEDINGUNGEN = ("dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
                   "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung")

# ========== § 9 Abs. 1 S. 3 Nr. 5a Übernachtung ==========
UEBERNACHTUNG_KOSTEN = "uebernachtung_kosten_monat"
UEBERNACHTUNG_RING = ("uebernachtung_kosten_monat", "uebernachtung_monate",
                      "uebernachtung_monate_bisher", "uebernachtung_im_inland")
UEBERNACHTUNG_BEDINGUNGEN = ("uebernachtung_auswaerts", "uebernachtung_alleinnutzung",
                             "uebernachtung_keine_lange_unterbrechung")

# ========== § 21 Vermietung ==========
VV_GESAMT_FELDER = ("vv_einnahmen", "vv_gebaeude_afa", "vv_schuldzinsen",
                    "vv_erhaltungsaufwand", "vv_sonstige_wk", "vv_entgelt_quote_prozent")
VV_ABS2_TATBESTAND = ("vv_wohnzwecke", "vv_auf_dauer")

# ========== § 20 Kapital ==========
KAP_ERTRAEGE = "kap_kapitalertraege"
KAP_TOEPFE = ("kap_gewinn_aktien", "kap_verlust_aktien", "kap_gewinn_sonstige", "kap_verlust_sonstige")
KAP_FELDER = (KAP_ERTRAEGE,) + KAP_TOEPFE + ("kap_zusammenveranlagung",)
KAP_ERTRAEGE_PARTNER = "kap_kapitalertraege_partner"
KAP_TOEPFE_PARTNER = ("kap_gewinn_aktien_partner", "kap_gewinn_sonstige_partner",
                      "kap_verlust_aktien_partner", "kap_verlust_sonstige_partner")
GESAMT_PARTNER_KAP = (KAP_ERTRAEGE_PARTNER,) + KAP_TOEPFE_PARTNER
GESAMT_PARTNER_19 = ("bruttoarbeitslohn_partner", "person_b_idnr")

# ========== § 22 Renten + § 33b Pauschbeträge ==========
RENTNER_AA_ARTEN = ("gesetzliche_rente", "berufsstaendische_versorgung", "private_basisrente")
RENTNER_22 = ("rentner_renten_art", "rentner_jahresrente", "rentner_renten_beginn_jahr",
              "rentner_alter_bei_rentenbeginn")
RENTNER_33B = ("rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind", "rentner_pflegegrad",
               "rentner_gepflegter_hilflos", "rentner_hinterbliebenenbezuege")
RENTNER_PARTNER = ("rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner")
RENTNER_22_PARTNER = ("rentner_renten_art_partner", "rentner_jahresrente_partner",
                      "rentner_renten_beginn_jahr_partner", "rentner_alter_bei_rentenbeginn_partner")
GESAMT_33B = ("rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind",
              "rentner_hinterbliebenenbezuege", "rentner_pflegegrad", "rentner_gepflegter_hilflos")
GESAMT_33B_PARTNER = ("rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner")

# ========== §§ 13-18 Gewinn ==========
EUER_KOMPONENTEN = ("betriebseinnahmen", "sonstige_betriebsausgaben", "afa_jahresbetrag")
GWG_FELDER = ("gwg_anschaffungskosten_netto",)
VERLUST_FELD = ("verlustvortrag_bestand",)
MITU_FELDER = ("gewinnanteil", "verguetung_taetigkeit", "verguetung_darlehen", "verguetung_ueberlassung")

# ========== § 34 Abs. 3 Ermäßigter Durchschnittssatz ==========
ABS3_FELDER = ("antrag_ermaessigter_satz", "dauernd_berufsunfaehig", "ermaessigung_einmal_genutzt")

# ========== § 35 GewSt-Anrechnung ==========
GESAMT_P35 = ("gewst_hebesatz", "gewst_messbetrag")

# ========== Rentner: §§ 13-18 Gewinn + Veräußerungs-Gewinn ==========
RENTNER_GEWINN = (("einkuenfte_gewinn", "rentner_veraeusserungsgewinn", "rentner_veraeusserungs_betriebsart",
                   "gewinn_betriebsart", "geburtsjahr") + EUER_KOMPONENTEN + GWG_FELDER + MITU_FELDER + ABS3_FELDER)

# ========== RENTNER_KEGEL (Pflicht-Felder im Rentner-Ring) ==========
RENTNER_KEGEL = RENTNER_22 + RENTNER_33B + ("veranlagung",) + AN_GESAMT_FLAGS + VOR_FELDER + KV_PV_FELDER

# ========== RENTNER_FELDER — ERSTE DEFINITION (Z.188 api.py) ==========
RENTNER_FELDER = (RENTNER_KEGEL + ("rentner_rentenfreibetrag", "rentner_rentenfreibetrag_partner")
                  + RENTNER_PARTNER + RENTNER_22_PARTNER + RENTNER_GEWINN
                  + ("gewst_hebesatz", "gewst_messbetrag") + VERLUST_FELD)

# ========== § 35a Haushaltsnahe ==========
HAUSHALT_35A_ABS23 = ("hh_dienstleistungen", "hh_handwerker_arbeitskosten")
HAUSHALT_35A = ("hh_minijob_aufwendungen",) + HAUSHALT_35A_ABS23 + ("hh_in_eu_ewr",)
P35A_MITVER_ANZEIGE = ("p35a_mitveranlagung",)

# ========== § 10b Spenden + § 10 KiSt (Gesamt) ==========
AGB_KIST = ("kist_gezahlt", "kist_erstattet")

# ========== § 10 Abs.1 Nr.5 Kinderbetreuung ==========
KINDERBETREUUNG = ("kinderbetreuungskosten", "kinderbetreuung_anzahl_kinder")

# ========== Gefaltete Sonder-Abzüge (Weg ii) ==========
GESAMT_ABZUEGE = (HAUSHALT_35A + ("hh_rechnung_unbar", "spenden_betrag",
                  "agb_aufwendungen", "fam_anzahl_kinder", "berufsausbildung_aufwendungen") + AGB_KIST + KINDERBETREUUNG)

# ========== RENTNER_FELDER — ZWEITE ÄNDERUNG (Z.218 api.py) ==========
RENTNER_FELDER = RENTNER_FELDER + GESAMT_ABZUEGE

# ========== § 24a/§24b Freibeträge ==========
GESAMT_FREIBETRAEGE = ("geburtsjahr", "fam_alleinstehend", "fam_monate_ohne_voraussetzung") + P35A_MITVER_ANZEIGE

# ========== § 34c DBA-Anrechnung ==========
GESAMT_DBA = ("dba_staat", "dba_methode", "dba_mehrere_staaten",
              "dba_gezahlte_auslaendische_steuer", "dba_auslaendische_einkuenfte",
              "dba_abzug_statt_anrechnung")

# ========== DBA-Länder-Methoden-Mapping ==========
DBA_METHOD_MAP = {
    "at": "freistellung",
    "ch": "anrechnung",
    "dk": "anrechnung",
    "es": "anrechnung",
    "fr": "anrechnung",
    "gb": "anrechnung",
    "lu": "anrechnung",
    "nl": "anrechnung",
    "pl": "anrechnung",
    "tr": "anrechnung",
    "us": "freistellung",
}

# ========== § 23 Private Veräußerungsgeschäfte ==========
GESAMT_P23 = ("p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten",
              "p23_werbungskosten", "p23_veraeusserungs_typ")

# ========== § 33a Unterhalt + Ausbildungsfreibetrag ==========
GESAMT_P33A = ("p33a_unterhalt_aufwendungen", "p33a_unterhalt_kv_pv",
               "p33a_andere_einkuenfte_bezuege", "p33a_ausbildung_anzahl_kinder")

# ========== § 32b Progressionsvorbehalt ==========
GESAMT_P32B = ("p32b_progressionseinkuenfte",)

# ========== § 35c Energetische Sanierung ==========
GESAMT_P35C = ("p35c_sanierungsaufwendungen", "p35c_ist_uebernaechstes_foerderjahr",
               "p35c_energieberater_aufwendungen")

# ========== § 10 Abs.1a Nr.1 Realsplitting ==========
GESAMT_REALSPLITTING = ("realsplitting_unterhaltsleistungen", "realsplitting_empfaenger_kv_pv",
                        "realsplitting_zustimmung")

# ========== § 21 Veräußerungs-Gewinn (Gesamt) ==========
GESAMT_VG = ("rentner_veraeusserungsgewinn", "rentner_veraeusserungs_betriebsart")

# ========== § 3 Nr. 72 Photovoltaik (steuerfreie Einnahmen) ==========
GESAMT_PV = ("pv_einnahmen", "pv_bruttoleistung_kwp", "pv_anzahl_einheiten", "pv_auf_gebaeude")

# ========== § 2 Gewinn (Gesamt) ==========
GESAMT_GEWINN = ("einkuenfte_gewinn", "gewinn_betriebsart") + EUER_KOMPONENTEN + GWG_FELDER + GESAMT_VG + GESAMT_P35 + VERLUST_FELD + MITU_FELDER + ABS3_FELDER + GESAMT_PV

# ========== RENTNER_FELDER — DRITTE ÄNDERUNG (Z.282 api.py) ==========
RENTNER_FELDER = RENTNER_FELDER + GESAMT_FREIBETRAEGE + GESAMT_DBA + GESAMT_P23 + GESAMT_P33A + GESAMT_P32B + GESAMT_P35C + GESAMT_REALSPLITTING + P36_ANRECHNUNG + KIST_KONFESSION_FELDER + P22_NR3_EINKUENFTE + P16_4_GATE_FELDER + KV_PV_PARTNER_FELDER + VOR_PARTNER_FELDER

# ========== Scheiben-Konfiguration ==========
SCHEIBEN = {
    "ep": {
        "felder": EP_FELDER, "felder_datei": None,
        "gesamt_ring": "abziehbarer_betrag",
        "teil_ringe": [],
    },
    "n_vor_gwg": {
        "felder": None, "felder_datei": "bindung_n_vor_gwg.yaml",
        "gesamt_ring": None,
        "teil_ringe": [("ep_werbungskosten", "abziehbarer_betrag", EP_FELDER)],
        "guard": False,
    },
    "an_gesamt": {
        "felder": (("bruttoarbeitslohn", "veranlagung") + EP_FELDER + VOR_FELDER + KV_PV_FELDER
                   + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_GUARD
                   + UEBERNACHTUNG_RING + UEBERNACHTUNG_BEDINGUNGEN + ARBEITSMITTEL_RING
                   + AN_GESAMT_FLAGS + AN_GESAMT_PARTNER + VOR_PARTNER_FELDER + KV_PV_PARTNER_FELDER
                   + P36_ANRECHNUNG
                   + KIST_KONFESSION_FELDER
                   + P35A_MITVER_ANZEIGE
                   + ("fam_anzahl_kinder", "verlustvortrag_bestand")),
        "kegel": (("bruttoarbeitslohn", "veranlagung") + EP_FELDER + VOR_FELDER + KV_PV_FELDER
                  + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + AN_GESAMT_FLAGS
                  + ("fam_anzahl_kinder", "verlustvortrag_bestand")),
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est",
        "teil_ringe": [],
        "guard": True,
    },
    "gesamt": {
        "felder": (VV_GESAMT_FELDER + VV_ABS2_TATBESTAND + ("veranlagung", "bruttoarbeitslohn")
                   + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS
                   + GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP + VORSORGE_PARTNER_FELDER
                   + GESAMT_ABZUEGE + GESAMT_FREIBETRAEGE + GESAMT_GEWINN
                   + GESAMT_33B + GESAMT_33B_PARTNER
                   + GESAMT_DBA + GESAMT_P23 + P22_NR3_EINKUENFTE + GESAMT_P33A + GESAMT_P32B + GESAMT_P35C
                   + GESAMT_REALSPLITTING
                   + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_GUARD
                   + UEBERNACHTUNG_RING + UEBERNACHTUNG_BEDINGUNGEN + ARBEITSMITTEL_RING
                   + P36_ANRECHNUNG + KIST_KONFESSION_FELDER + P16_4_GATE_FELDER),
        "kegel": (VV_GESAMT_FELDER + ("veranlagung", "bruttoarbeitslohn")
                  + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS),
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est_gesamt",
        "teil_ringe": [],
        "guard": True,
        "gesamt_guard": True,
        "fremd_arten": ("kein_sonstige",),
        "partner_19": True,
        "multi_objekt": "vv_objekt",
    },
    "rentner_gesamt": {
        "felder": RENTNER_FELDER,
        "kegel": RENTNER_KEGEL,
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est_rentner",
        "teil_ringe": [],
        "guard": True,
        "gesamt_guard": True,
        "rentner": True,
        "multi_rente": "rente",
        "fremd_arten": ("kein_kap", "kein_vuv"),
    },
}

# ========== __all__ für import * ==========
# Alle public Namen (ohne führenden _) + private (_*) Namen die für api.py kritisch sind
__all__ = [
    # Private/regex
    "_FALL_RE", "_AUTH_USER", "_ERLAUBTE_ZUSTAENDE",
    # Pfade
    "HERE", "FAELLE",
    # § 19 Einkünfte
    "EP_FELDER",
    # an_gesamt
    "AN_GESAMT_FLAGS", "AN_GESAMT_PARTNER",
    # Arbeitsmittel
    "ARBEITSMITTEL_KOSTEN", "ARBEITSMITTEL_RING",
    # § 36/§22/§10 KiSt
    "P36_ANRECHNUNG", "P22_NR3_EINKUENFTE", "KIST_KONFESSION_FELDER", "P16_4_GATE_FELDER",
    # Verpflegung
    "VERPFLEGUNG_TAGE", "VERPFLEGUNG_GUARD",
    # Vorsorge
    "VOR_FELDER", "VOR_PARTNER_FELDER", "KV_PV_FELDER", "KV_PV_PARTNER_FELDER", "VORSORGE_PARTNER_FELDER",
    # dHf
    "DHF_KOSTEN", "DHF_RING", "DHF_BEDINGUNGEN",
    # Übernachtung
    "UEBERNACHTUNG_KOSTEN", "UEBERNACHTUNG_RING", "UEBERNACHTUNG_BEDINGUNGEN",
    # Vermietung
    "VV_GESAMT_FELDER", "VV_ABS2_TATBESTAND",
    # Kapital
    "KAP_ERTRAEGE", "KAP_TOEPFE", "KAP_FELDER", "KAP_ERTRAEGE_PARTNER", "KAP_TOEPFE_PARTNER", "GESAMT_PARTNER_KAP", "GESAMT_PARTNER_19",
    # Rentner
    "RENTNER_AA_ARTEN", "RENTNER_22", "RENTNER_33B", "RENTNER_PARTNER", "RENTNER_22_PARTNER", "GESAMT_33B", "GESAMT_33B_PARTNER",
    # Gewinn
    "EUER_KOMPONENTEN", "GWG_FELDER", "VERLUST_FELD", "MITU_FELDER", "ABS3_FELDER",
    # § 35
    "GESAMT_P35",
    # Rentner Gewinn
    "RENTNER_GEWINN", "RENTNER_KEGEL",
    # Abzüge
    "HAUSHALT_35A_ABS23", "HAUSHALT_35A", "P35A_MITVER_ANZEIGE", "AGB_KIST", "KINDERBETREUUNG", "GESAMT_ABZUEGE",
    # Freibeträge
    "GESAMT_FREIBETRAEGE",
    # DBA
    "GESAMT_DBA", "DBA_METHOD_MAP",
    # Weitere Abzüge
    "GESAMT_P23", "GESAMT_P33A", "GESAMT_P32B", "GESAMT_P35C", "GESAMT_REALSPLITTING",
    "GESAMT_PV",
    # Veräußerungs-Gewinn
    "GESAMT_VG", "GESAMT_GEWINN",
    # Rentner Felder
    "RENTNER_FELDER",
    # Scheiben
    "SCHEIBEN",
]
