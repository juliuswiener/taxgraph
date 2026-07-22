"""Paket-B Haut — reine Endpunkt-Logik über die Paket-A-Naht (traverser/API.md). LLM-frei.

Jede Funktion ist eine reine `(fall/eingabe) -> (http_status, obj)`-Hülle über GENAU eine Naht-
Funktion — kein Transport, kein Socket (den setzt server.py drum). So bleibt der Upgrade-Pfad
(stdlib -> uvicorn) ein reiner Austausch der Transport-Schicht.

Naht-Grenze (API.md): LESEN über traverser/intervall/est_mapping/bindung; SCHREIBEN
ausschliesslich `store.append_event`. Keine Steuerlogik, keine zweite Wahrheit hier.

Bescheid-Ehrlichkeit (K2): ein numerischer [min,max]-Ring erscheint NUR, wo ein exponierter
golden-Accessor die Größe wirklich rechnet. Eine Scheibe mit Gesamt-Accessor (EP) trägt einen
`intervall` (Scheiben-Bescheid); eine Multi-Regel-Scheibe ohne ehrlichen Gesamt-Accessor
(N+VOR+GWG) trägt KEINEN Gesamt-Bescheid — nur ring-fähige Teilfamilien (EP-Abzug) als
`teil_ringe`, der Rest ist ehrlich engine=unavailable. Kein erfundener Betrag.
"""
from __future__ import annotations

import base64
import json
import os
import re
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)
for _sub in ("produkt/store", "produkt/traverser", "produkt/unsicherheit", "produkt/mapping",
             "produkt/konsistenz", "produkt/import", "golden"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402
import intervall as IV      # noqa: E402
import est_mapping as EM    # noqa: E402
import flag_check as FC     # noqa: E402  (Flag↔Einkunftsart-Widersprüche, dev-2)
import partner_check as PC  # noqa: E402  (Partner-Behinderungsfeld↔Zusammenveranlagung, dev-2)
import vorjahr_writer as VW  # noqa: E402  (Vorjahres-Übernahme, dev-2 Store-Writer)

FAELLE = os.path.join(HERE, "faelle")

EP_FELDER = ("ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz")

# an_gesamt MVP (reiner Arbeitnehmerfall). Die 4 Abwesenheits-Flags sind die Anwendbarkeits-
# Voraussetzung (bestätigte Null je Einkunftsart); Invertierung der positiven Laienfrage macht die Haut.
AN_GESAMT_FLAGS = ("kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige")
# Sperr-Felder (K2-Guard): ein aktiver Wert > 0 macht den Ring UNMÖGLICH (die Engine kann diese
# Werbungskosten mangels Modell nicht rechnen) → Ring bleibt unavailable, NIE still auf 0.
# Stufe 1b: dHf + Verpflegung sind RING-FÄHIG (raus); nur Arbeitsmittel (kein Modell) bleibt hart im Guard.
GUARD_WERBUNGSKOSTEN = ("am_anschaffungskosten",)
# Verpflegung (§ 9 Abs. 4a): 3 Tage-Ring-Inputs + 2 Reduktions-Guard-Felder. FAIL-CLOSED-ON-UNSET:
# bei Tagen > 0 ist der Ring nur fähig, wenn beide Guard-Felder EXPLIZIT sicher sind (monate ≤ 3
# gesetzt UND keine_mahlzeitengestellung=true gesetzt); sonst (inkl. UNSET) verpflegung_reduktion_offen.
VERPFLEGUNG_TAGE = ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig")
VERPFLEGUNG_GUARD = ("vpf_monate_am_ort", "vpf_keine_mahlzeitengestellung")
# Stufe 1a: die VOR-Felder (§ 10 Altersvorsorge) sind RING-FÄHIG — echte Rechnung via
# _vorsorge_abzug über den Store-Einzelfeld-Zugriff (kein Guard mehr, aber Teil des Kegels).
VOR_FELDER = ("vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb")
# § 10 Abs. 1 Nr. 3/3a KV/PV-Vorsorge (§10-Stufe 2): PFLICHT-Kegel (betrifft jeden; mit_anspruch absent→false→
# HB 2800 statt 1900 = zu hoher Abzug = Unter-tax für Zuschuss-Nutzer → mandatory + explizit gefragt).
KV_PV_FELDER = ("basis_kv_pv", "weitere_vorsorgeaufwendungen", "mit_anspruch_auf_zuschuss")
# Stufe 1b — doppelte Haushaltsführung (§ 9 Abs. 1 S. 3 Nr. 5): 3 Ring-Inputs + 4 Tatbestands-
# Bedingungen. dHf-Abzug greift NUR wenn alle 4 bestätigt-true UND Inland; Kosten > 0 mit offener
# Bedingung → dhf_tatbestand_offen, mit Ausland → ausland_dhf_nicht_ring_faehig (kein Fake).
DHF_KOSTEN = "dhf_unterkunftskosten_monat"
DHF_RING = ("dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland")
DHF_BEDINGUNGEN = ("dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
                   "dhf_finanzielle_beteiligung", "dhf_keine_pflicht_dienstwohnung")
# Front 2 — Zusammenveranlagung (§ 26b, Splitting). Partner-Pflichtfelder + Partner-VOR. Nur bei
# veranlagung=zusammen relevant: der Ring rechnet dann catala_est_zusammen (§9a je Person + Splitting
# im Scope). MVP: Person B ohne gesonderte WK (wk_b=0), ohne VOR (Partner-VOR sperrt via Guard).
AN_GESAMT_PARTNER = ("bruttoarbeitslohn_partner", "person_b_idnr")
VOR_PARTNER_FELDER = ("vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner",
                      "vor_rv_ausserhalb_lstb_partner")
# § 10 Person-B-Vorsorge (A.2, Zusammenveranlagung): KV/PV-Partner (§ 10 Abs. 1 Nr. 3/3a, eigener HB je Person) +
# geburtsjahr_partner (§ 24a-B-Kohorte). vor_*_rv_partner (Basisvorsorge-B) stehen schon in VOR_PARTNER_FELDER.
# OPTIONAL im gesamt-Ring (absent → 0, over-tax-safe), NICHT Pflicht-Kegel (Person-B-Vorsorge absent = Paar ohne
# B-Vorsorge → kein stiller Under-tax). Ersetzt den A.1-partner_vorsorge_offen-Guard (Person B jetzt rechenbar).
KV_PV_PARTNER_FELDER = ("basis_kv_pv_partner", "weitere_vorsorgeaufwendungen_partner",
                        "mit_anspruch_auf_zuschuss_partner")
VORSORGE_PARTNER_FELDER = VOR_PARTNER_FELDER + KV_PV_PARTNER_FELDER + ("geburtsjahr_partner",)
# Front V+V (§ 21): Überschuss-Rechnung Einnahmen − Werbungskosten (Scheibe 3, referenziert). vv_entgelt_quote_
# prozent (§ 21 Abs. 2) ist PFLICHT-Kegel (per Objekt): jeder Vermieter MUSS die Entgelt-Quote beantworten
# (100 % wenn nicht verbilligt), sonst bliebe der verbilligt-Vermieter stumm bei voller WK = Unter-tax (K2).
VV_GESAMT_FELDER = ("vv_einnahmen", "vv_gebaeude_afa", "vv_schuldzinsen",
                    "vv_erhaltungsaufwand", "vv_sonstige_wk", "vv_entgelt_quote_prozent")
# § 21 Abs. 2 Tatbestand (Wohnzwecke + auf Dauer): OPTIONAL (nicht Pflicht-Kegel), per Objekt. Absent → Tatbestand
# hält → Kürzung greift bei quote < 66 (fail-safe über-tax). Explizit False (gewerblich/nicht-dauer) → volle WK.
VV_ABS2_TATBESTAND = ("vv_wohnzwecke", "vv_auf_dauer")
# Front Kapital (§ 20 / § 32d): E1900701-Aggregat ODER die zwei Verlust-Töpfe (Aktien/sonstige) — MVP
# SINGLE-SOURCE (Instructor-Entscheid Q1): beide gesetzt → kapital_semantik_offen (Vordruck-Semantik
# additiv-vs-subset = benannter GAP, nicht geraten). kap_zusammenveranlagung verdoppelt den Sparer-PB.
KAP_ERTRAEGE = "kap_kapitalertraege"
KAP_TOEPFE = ("kap_gewinn_aktien", "kap_verlust_aktien", "kap_gewinn_sonstige", "kap_verlust_sonstige")
KAP_FELDER = (KAP_ERTRAEGE,) + KAP_TOEPFE + ("kap_zusammenveranlagung",)
# Front Rentner (§ 22 Leibrente + § 33b Pauschbeträge). § 22 → einkuenfte_sonstige (4-Zweig-Accessor,
# aa Rentenfreibetrag-Fixierung), § 33b → aussergewoehnliche_belastungen (behinderten+pflege+hinterbliebenen
# additiv). alter_bei_rentenbeginn ist DERIVED (immer da); rentenfreibetrag nur aa-Folgejahr (nicht im Kegel,
# Guard erzwingt ihn). Partner-Behinderung nur bei Zusammenveranlagung (partner_check).
RENTNER_AA_ARTEN = ("gesetzliche_rente", "berufsstaendische_versorgung", "private_basisrente")
RENTNER_22 = ("rentner_renten_art", "rentner_jahresrente", "rentner_renten_beginn_jahr",
              "rentner_alter_bei_rentenbeginn")
RENTNER_33B = ("rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind", "rentner_pflegegrad",
               "rentner_gepflegter_hilflos", "rentner_hinterbliebenenbezuege")
RENTNER_PARTNER = ("rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner")
# Weg-ii-Fix (K2, Over-tax): VOR_FELDER (§ 10 Abs. 1 Nr. 2 Basisvorsorge) + KV_PV_FELDER (§ 10 Abs. 1 Nr. 3/3a
# KV/PV) mandatory im Kegel — 1:1 gesamt-Präzedenz (Z. 254f). KV_PV zwingend: mit_anspruch_auf_zuschuss absent
# → false → HB 2800 statt 1900 = Unter-tax bei Zuschuss-Nutzern (quasi-universell, betrifft jeden Rentner mit
# KV/PV). VOR over-tax-safe (absent → 0), mandatory nur für Uniformität mit gesamt.
RENTNER_KEGEL = RENTNER_22 + RENTNER_33B + ("veranlagung",) + AN_GESAMT_FLAGS + VOR_FELDER + KV_PV_FELDER
# RENTNER_22_PARTNER: § 22-Rente des Ehegatten (Zusammenveranlagung, #4b), analog RENTNER_22; +
# rentner_rentenfreibetrag_partner (aa-Folgejahr B). Nur bei zusammen relevant → nicht im Pflicht-Kegel.
RENTNER_22_PARTNER = ("rentner_renten_art_partner", "rentner_jahresrente_partner",
                      "rentner_renten_beginn_jahr_partner", "rentner_alter_bei_rentenbeginn_partner")
# §§ 13-18 Gewinn im Rentner-Ring (2-I), OPTIONAL (NICHT Pflicht-Kegel → absent → 0, over-tax-safe): einkuenfte_gewinn
# = laufender §15/§18-Gewinn (direkter Betrag, wie gesamt-Ring) + rentner_veraeusserungsgewinn = §16-Veräußerungsgewinn
# (nach § 16 Abs. 4-Freibetrag, catala_p16_4_freibetrag) — beide summieren ADDITIV in einkuenfte_gewinn (§16 Abs.1
# „gehören auch": Veräußerungs- + laufender Gewinn = dieselbe §2-Einkunftsart). rentner_veraeusserungs_betriebsart =
# Kz-Weiche (Anlage G/S, Klasse-f), Ring liest sie nicht. Alle global gebunden (lade_bindung globt alle yamls).
# § 4 Abs. 3 EÜR-Komponenten (Stufe 2a): der laufende Gewinn = betriebseinnahmen − (sonstige_betriebsausgaben +
# afa_jahresbetrag) via catala_euer_gewinn (shared _laufender_gewinn). OPTIONAL — irgendeine present → EÜR-Pfad,
# sonst der direkte einkuenfte_gewinn-Wert (Stufe 1). In gesamt.felder UND RENTNER_FELDER (Scope-A: EÜR auch im
# Rentner-Pfad, ein Rentner mit laufendem § 15/§ 18-Betrieb). gewinn_betriebsart auch in RENTNER_FELDER = die
# land_forst-Weiche für den luf_euer_offen-Guard (getrennt von rentner_veraeusserungs_betriebsart = § 16-vg-Kz).
EUER_KOMPONENTEN = ("betriebseinnahmen", "sonstige_betriebsausgaben", "afa_jahresbetrag")
# § 6 Abs. 2 GWG-Sofortabzug (Stufe 2b): gwg_anschaffungskosten_netto = instanz_gruppe:gwg (mehrere Assets je
# Fall, je ≤ 800 netto sofort abziehbar), STUMPFE Σ (catala_p6_2_gwg) in den EÜR-betriebsausgaben-Term von
# _laufender_gewinn. In gesamt.felder UND RENTNER_FELDER. Feld war schon in n_vor_gwg gebunden (inert) → hier
# reused (kein neuer regel_id-Knoten). Weitere Assets = gwg_anschaffungskosten_netto__2..N (instanzen-Naht).
GWG_FELDER = ("gwg_anschaffungskosten_netto",)
# § 35 GewSt-Anrechnung (S1, gesamt-Ring): gewst_messbetrag (GewSt-Steuermessbetrag aus dem Messbescheid, INPUT,
# enthält § 8/§9 schon, cent, OPTIONAL/opt-in) + gewst_hebesatz (%, int) → Anrechnung auf die tarifliche ESt.
# NUR gesamt.felder (nicht rentner, nicht kegel). Kein gewst_messbetrag → kein § 35 (over-tax-safe opt-out);
# Messbetrag ohne Hebesatz sperrt gewst_hebesatz_offen. Kz null-MVP.
GESAMT_P35 = ("gewst_hebesatz", "gewst_messbetrag")
# § 10d Abs. 2 Verlustvortrag (gesamt + rentner): verlustvortrag_bestand = der festgestellte verbleibende Verlust-
# vortrag (Feststellungsbescheid, cent, opt-in). catala_p10d_2 mindert den GdE „vorrangig vor Sonderausgaben/agB/
# sonstigen Abzugsbeträgen" (§ 10d Abs. 2) → Fold in sonstige_abzuege_vom_einkommen (linear, kein Floor → wertgleich).
# gesamtbetrag_einkuenfte + zusammenveranlagung sind DERIVED (GdE-Zwilling + veranlagung) → kein Feld-Add. absent → 0.
VERLUST_FELD = ("verlustvortrag_bestand",)
# § 15 Abs. 1 S. 1 Nr. 2 Mitunternehmer (#2-Front): gewinnanteil (§ 15a-ausgleichsfähiger Anteil, KANN NEGATIV) +
# 3 Sondervergütungen (Tätigkeit/Darlehen/Überlassung, ≥ 0) → catala_mitunternehmer_einkuenfte-Summe ADDITIV in
# einkuenfte_gewinn (§ 15 gewerblich, Anlage G) via _laufender_gewinn (→ symmetrisch in §35-Zähler+Nenner). In
# gesamt UND rentner (Rentner-mit-Mitunternehmer). § 35-Abs.2-anteiliger-Messbetrag = eigene Folge-Naht (a2). Kz
# null-MVP, alle optional (absent → 0). kein_gewinn-Negation für die 4 = dev-2 flag_check.
MITU_FELDER = ("gewinnanteil", "verguetung_taetigkeit", "verguetung_darlehen", "verguetung_ueberlassung")
# § 34 Abs. 3 ermäßigter Durchschnittssatz (Stufe-2a): 3 Flags steuern den Abs.1-vs-Abs.3-Chooser (dev-1-Naht).
# antrag_ermaessigter_satz (S.1 „auf Antrag", opt-in — ohne → Abs.1-Default) + dauernd_berufsunfaehig (S.1 Alt. zu
# Alter≥55, letzterer DERIVE aus geburtsjahr) + ermaessigung_einmal_genutzt (S.4 „einmal im Leben", FA-tracked Selbst-
# Bestätigung). ao = REUSE rentner_veraeusserungsgewinn (§16-vg). Alle optional (absent → False → Abs.1). Kz null-MVP.
ABS3_FELDER = ("antrag_ermaessigter_satz", "dauernd_berufsunfaehig", "ermaessigung_einmal_genutzt")
RENTNER_GEWINN = (("einkuenfte_gewinn", "rentner_veraeusserungsgewinn", "rentner_veraeusserungs_betriebsart",
                   "gewinn_betriebsart", "geburtsjahr") + EUER_KOMPONENTEN + GWG_FELDER + MITU_FELDER + ABS3_FELDER)
# ⚠ geburtsjahr in RENTNER_GEWINN (b): behebt (i) § 24a-rentner-Kohorte/64+-Gate + (ii) den § 34-Abs.3-rentner-AGE-Pfad
# (_abs3_eligible las f["geburtsjahr"] → im rentner-Ring 0 → alter≥55=False → abs3-via-Alter fiel auf Abs.1 = Over-tax).
# global-glob gebunden (bindung_an_gesamt), optional (absent → 0 → kein § 24a, over-tax-safe).
# § 35 GewSt-Anrechnung auch im Rentner-Ring (Rentner-mit-Gewerbe): gewst_hebesatz + gewst_messbetrag (schon
# global gebunden, s. GESAMT_P35). Der Deckel-3-Nenner ist hier VOLLSTÄNDIG renten(§22) + einkuenfte_gewinn —
# die rentner-Scheibe hat KEIN § 19/§ 21 (ein Rentner-mit-Minijob/Miete ist scheiben-strukturell nicht modellierbar,
# pre-existing Grenze, nicht § 35-spezifisch). Guard gewst_hebesatz_offen greift shared (gesamt_guard).
# VOR_FELDER/KV_PV_FELDER kommen schon über RENTNER_KEGEL (jetzt mandatory, s.o.). GESAMT_ABZUEGE (§35a/§10b/
# §33/§10-KiSt/§10-1-7) wird UNTEN nachgetragen (Weg-ii-Fix) — Tupel steht erst nach seiner Definition zur
# Verfügung, s. Anhang bei GESAMT_ABZUEGE.
RENTNER_FELDER = (RENTNER_KEGEL + ("rentner_rentenfreibetrag", "rentner_rentenfreibetrag_partner")
                  + RENTNER_PARTNER + RENTNER_22_PARTNER + RENTNER_GEWINN
                  + ("gewst_hebesatz", "gewst_messbetrag") + VERLUST_FELD)
# Person B (Zusammenveranlagung, dev-2s Person-B-Deklaration): die §19-Einkünfte des Ehegatten in den
# Gesamt-Ring. Nur bei veranlagung=zusammen relevant → NICHT im Pflicht-Kegel (der Guard erzwingt den
# Person-B-Kegel bei zusammen). Person-B-Kapital/§22 = getrennte Folge-Nachträge (#4-Fortsetzung).
GESAMT_PARTNER_19 = ("bruttoarbeitslohn_partner", "person_b_idnr")
# Person B (#4b): Kapital (§ 20) + Rente (§ 22) des Ehegatten in den jeweiligen zusammen-Ring. Single-
# source wie Basis (Aggregat XOR Töpfe). kap_gewinn_sonstige_partner: Register-B-K2-Fix 2026-07-19 —
# symmetrisch zu Person A deklariert (vorher hart 0 = stiller Under-tax des Ehegatten-Gewinns).
KAP_ERTRAEGE_PARTNER = "kap_kapitalertraege_partner"
KAP_TOEPFE_PARTNER = ("kap_gewinn_aktien_partner", "kap_gewinn_sonstige_partner",
                      "kap_verlust_aktien_partner", "kap_verlust_sonstige_partner")
GESAMT_PARTNER_KAP = (KAP_ERTRAEGE_PARTNER,) + KAP_TOEPFE_PARTNER
# § 35a-Töpfe (charge29): Abs. 2/3 (Dienstleistung/Handwerker) verlangen rechnung_unbar (Abs. 5 S. 3), Abs. 1
# Minijob nicht. Bausteine der gefalteten Sonder-Abzüge (die Standalone-haushalt/agb-Scheiben sind deprecated).
HAUSHALT_35A_ABS23 = ("hh_dienstleistungen", "hh_handwerker_arbeitskosten")   # Abs. 2/3 (rechnung_unbar-Pflicht)
HAUSHALT_35A = ("hh_minijob_aufwendungen",) + HAUSHALT_35A_ABS23              # + Abs. 1 Minijob
AGB_KIST = ("kist_gezahlt", "kist_erstattet")                                # § 10 KiSt gezahlt/erstattet
KINDERBETREUUNG = ("kinderbetreuungskosten", "kinderbetreuung_anzahl_kinder")  # § 10 Abs.1 Nr.5 Kinderbetreuung
# Gefaltete Sonder-Abzüge (Weg ii): §35a + §10b + §33 + §10-KiSt + §10 Abs.1 Nr.5 Kinderbetreuung +
# §10 Abs.1 Nr.7 Berufsausbildung als OPTIONALE Felder im gesamt-Ring (NICHT im Pflicht-Kegel — absent → Abzug 0,
# fail-SAFE). Der gesamt-slot_fn rechnet sie additiv auf JEDE Einkunfts-Kombi; die K2-Sperren
# (rechnung_unbar/erstattungsueberhang) fängt der Guard.
GESAMT_ABZUEGE = (HAUSHALT_35A + ("hh_rechnung_unbar", "spenden_betrag",
                  "agb_aufwendungen", "fam_anzahl_kinder", "berufsausbildung_aufwendungen") + AGB_KIST + KINDERBETREUUNG)
# Weg-ii-Fix (K2, Over-tax): GESAMT_ABZUEGE OPTIONAL auch im Rentner-Ring nachgetragen (NICHT im Kegel —
# absent → 0, fail-safe, wie im gesamt-Ring). Ohne diese Deklaration wären die Felder für rentner_gesamt
# nicht mal POSTbar (_scheibe_bindung filtert global-bindung auf cfg["felder"]).
RENTNER_FELDER = RENTNER_FELDER + GESAMT_ABZUEGE
# § 24a/§ 24b Freibeträge (Weg ii Stage 2), OPTIONAL im gesamt-Ring (absent → 0). geburtsjahr = §24a-Kohorten-
# Schlüssel (gesamt-only); fam_alleinstehend = §24b-Abs.3-Flag (quelle p24b/alleinstehend, fragetext „ohne
# anderen Erwachsenen im Haushalt" — IST die Abs.3-Bedingung, kein Extra-Feld nötig); fam_monate = §24b-Kürzung.
# fam_anzahl_kinder steht schon in GESAMT_ABZUEGE (§33-zumutbar + §24b geteilt).
GESAMT_FREIBETRAEGE = ("geburtsjahr", "fam_alleinstehend", "fam_monate_ohne_voraussetzung")
# §33b Behinderten-/Pflege-/Hinterbliebenen-Pauschbetrag — NUR über rentner_-Felder geführt
# (globale Bindung, Kz E0109708/etc.). Im gesamt-Ring optional (NICHT im Kegel, absent→0→over-tax-safe).
# rentner_-Präfix = Namensschuld (wie rentner_veraeusserungsgewinn), selbe Felder wie rentner-Scheibe.
GESAMT_33B = ("rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind",
              "rentner_hinterbliebenenbezuege", "rentner_pflegegrad", "rentner_gepflegter_hilflos")
GESAMT_33B_PARTNER = ("rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner")
# §34c Abs.1 DBA-Anrechnung (Stufe-1, single-country, fail-closed): 5 Felder (Staat/Methode/
# mehrere/Betrag/Einkünfte). OPTIONAL (absent→0→safe). Teilmenge-Invariant: dba_auslaendische_
# einkuenfte ⊆ Welteinkommen, NIE additiv zur GdE, nur Accessor-Zähler. an_gesamt out-of-scope.
GESAMT_DBA = ("dba_staat", "dba_methode", "dba_mehrere_staaten",
              "dba_gezahlte_auslaendische_steuer", "dba_auslaendische_einkuenfte")
# §23 Private Veräußerungsgeschäfte (Stufe-1): 4 Felder pro Veräußerung (Multi-Instanz,
# instanz_gruppe "p23_veraeusserung" wie §21). OPTIONAL (absent→0→safe). Σ über Instanzen
# im Ring → Freigrenze+VTOP → ADDITIV in einkuenfte_sonstige (neben §22-Rente).
GESAMT_P23 = ("p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten",
              "p23_werbungskosten", "p23_veraeusserungs_typ")
# §33a EStG (Unterhalt Abs.1 + Ausbildungsfreibetrag Abs.2): 4 Felder, OPTIONAL
# (absent→0→over-tax-safe). Keine zumutbare Eigenbelastung (§33) → sonstige_abzuege_
# vom_einkommen (neben §10d). Teilmengen-Invariant: andere_einkuenfte_bezuege = nur
# Einkünfte des Unterhaltsempfängers, NICHT die des Steuerpflichtigen.
GESAMT_P33A = ("p33a_unterhalt_aufwendungen", "p33a_unterhalt_kv_pv",
               "p33a_andere_einkuenfte_bezuege", "p33a_ausbildung_anzahl_kinder")
# §32b Progressionsvorbehalt (Abs.1 Nr.1 Lohnersatz): 1 Aggregat-Feld, OPTIONAL
# (absent→0→safe). Post-Engine-Wrapper (NICHT tarif_modifiziert, Scheibe-Isolation).
GESAMT_P32B = ("p32b_progressionseinkuenfte",)
# Weg-ii-Parität-Fix (K2, Over-tax, ring-b-Fund #4): GESAMT_FREIBETRAEGE auch im Rentner-Ring nachgetragen —
# ohne fam_alleinstehend/fam_monate_ohne_voraussetzung postbar war § 24b im Rentner-Ring nicht erreichbar
# (geburtsjahr/fam_anzahl_kinder stehen schon in RENTNER_GEWINN/GESAMT_ABZUEGE, Duplikat harmlos).
RENTNER_FELDER = RENTNER_FELDER + GESAMT_FREIBETRAEGE + GESAMT_DBA + GESAMT_P23 + GESAMT_P33A + GESAMT_P32B
# §§ 13-18 Gewinneinkünfte (Stufe 1), OPTIONAL im gesamt-Ring (NICHT Pflicht-Kegel → absent → 0, over-tax-safe).
# einkuenfte_gewinn (CENT) = der vorberechnete Gewinn-Betrag → einkuenfte_gewinn-Slot der slot_fn (§ 2-Summand).
# gewinn_betriebsart (Enum gewerbe/selbstaendig/land_forst) = NUR gespeichert — Kz-Weiche für est_mapping/
# Deklaration (Anlage G/S/L), der RING liest sie NICHT (wie veranlagung ein Enum-Feld ohne Rechen-Effekt).
# In felder (askable, POST /event braucht fid∈bindung), symmetrisch zu dev-2s Bindung → kein Orphan-askable.
# § 16 Veräußerungsgewinn im gesamt-Ring (Non-Rentner-§16-vg): REUSE des generellen §16-vg-Felds
# rentner_veraeusserungsgewinn (+ -betriebsart) — trotz „rentner_"-Namens ist es NICHT rentner-spezifisch (Kz
# E0801301 Anlage G / E0804501 Anlage S / E0901201 Anlage L, regel_id p16_4, flag_check deckt es seit 2-I). Global gebunden →
# hier nur zusätzlich in gesamt.felder, Fold spiegelt den rentner-2-I-Fold (netto_vg = max(0, vg − §16-Abs.4-FB),
# ADDITIV in einkuenfte_gewinn). NAMING-DEBT: „rentner_"-Präfix in gesamt irreführend → künftiger Rename
# (rentner_veraeusserungsgewinn → veraeusserungsgewinn), eigener Cleanup. Routing bleibt Scheibe-fix (nicht
# feld-getriggert) → EIN Fall = EINE Scheibe = EIN slot_fn, kein Doppel-Pfad.
GESAMT_VG = ("rentner_veraeusserungsgewinn", "rentner_veraeusserungs_betriebsart")
GESAMT_GEWINN = ("einkuenfte_gewinn", "gewinn_betriebsart") + EUER_KOMPONENTEN + GWG_FELDER + GESAMT_VG + GESAMT_P35 + VERLUST_FELD + MITU_FELDER + ABS3_FELDER

# Scheiben-Konfiguration.
#   felder      : feste feld_id-Menge (None -> aus felder_datei laden).
#   felder_datei: bindung_*.yaml, aus der ALLE feld_ids der Scheibe gezogen werden.
#   gesamt_ring : quantitaet-Key, wenn EIN Accessor die GANZE Scheibe als Bescheid bedient
#                 (-> /stand.intervall + /ergebnis feste Zahl). None = kein ehrlicher Gesamt-Bescheid.
#   teil_ringe  : [(name, quantitaet, felder)] ring-fähige Teilfamilien für Scheiben OHNE Gesamt-Ring
#                 (ehrlicher Teil-Ring, ausdrücklich KEIN Scheiben-Bescheid).
SCHEIBEN = {
    "ep": {
        "felder": EP_FELDER, "felder_datei": None,
        "gesamt_ring": "abziehbarer_betrag",
        "teil_ringe": [],
    },
    "n_vor_gwg": {
        "felder": None, "felder_datei": "bindung_n_vor_gwg.yaml",
        "gesamt_ring": None,      # Gesamtsteuer via catala_gesamt = eigenes Integrations-Paket
        "teil_ringe": [("ep_werbungskosten", "abziehbarer_betrag", EP_FELDER)],
        "guard": False,
    },
    # Gesamtsteuer-Ring MVP: technischer Durchstich — echte festzusetzende ESt (§2) für den REINEN
    # Arbeitnehmerfall (Bruttolohn + Entfernungspauschale, keine gesondert erfassten Sonderausgaben,
    # keine anderen Einkunftsarten). NICHT „fertig für Angestellte": VOR/dHf/… sperren via Guard.
    "an_gesamt": {
        "felder": (("bruttoarbeitslohn", "veranlagung") + EP_FELDER + VOR_FELDER + KV_PV_FELDER
                   + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_GUARD
                   + AN_GESAMT_FLAGS + AN_GESAMT_PARTNER + VOR_PARTNER_FELDER + KV_PV_PARTNER_FELDER
                   + ("fam_anzahl_kinder", "verlustvortrag_bestand")),
        # Pflicht-Kegel = einzel-Basis (inkl. Verpflegungs-TAGE; die Reduktions-Guard-Felder prüft
        # der Guard nur bei Tagen > 0). Partner-Pflichtfelder prüft der Guard nur bei zusammen. KV_PV_FELDER
        # (§10 Abs.1 Nr.3/3a) PFLICHT wie in gesamt/rentner_gesamt — betrifft jeden (Gesamt-Parität, Over-tax-
        # Fix). KV_PV_PARTNER_FELDER NICHT im Kegel (optional wie in gesamt, absent → 0, over-tax-safe).
        # fam_anzahl_kinder + verlustvortrag_bestand = MANDATORY im Kegel (K2-Gap-A/B-Fix): an_gesamt
        # kann Kinder/Verlustvortrag NUR via Sperr-Grund verarbeiten (gehoeren_in_gesamt).
        "kegel": (("bruttoarbeitslohn", "veranlagung") + EP_FELDER + VOR_FELDER + KV_PV_FELDER
                  + DHF_RING + DHF_BEDINGUNGEN + VERPFLEGUNG_TAGE + AN_GESAMT_FLAGS
                  + ("fam_anzahl_kinder", "verlustvortrag_bestand")),
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est",
        "teil_ringe": [],
        "guard": True,
    },
    # DER §-2-GESAMT-RING (EIN catala_gesamt-Ring): dieselbe Scheibe rechnet reinen §21 (Bruttolohn =
    # bestätigte Null), Job+Vermietung UND Kapital (§ 20/§ 32d, Günstigerprüfung über zwei gesamt-Läufe).
    # §19 → einkuenfte_nichtselbststaendig (§9a-bereinigt), §21 → einkuenfte_vermietung, §20 → Kapital-
    # Steuer (est_ohne + min(Abgeltung, Günstiger-delta)). §21-Verlust mindert §19-Lohn (Loss-Offset, K2),
    # §10c einmal. Der frühere Name „vv_gesamt" war nach der §19/§20-Integration ein Fehlname — die
    # Named-Architektur-Schuld ist mit dem Rename auf „gesamt" eingelöst (an_gesamt bleibt der schmale
    # AN-only-MVP über catala_est). NAMED GAPS: § 10d Verlustvortrag; Kapital-Co-Okkurrenz E1900701+Töpfe
    # (kapital_semantik_offen); zusammen+§19 (Person-B); §22-Rente = weitere Summanden.
    "gesamt": {
        "felder": (VV_GESAMT_FELDER + VV_ABS2_TATBESTAND + ("veranlagung", "bruttoarbeitslohn")
                   + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS
                   + GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP + VORSORGE_PARTNER_FELDER
                   + GESAMT_ABZUEGE + GESAMT_FREIBETRAEGE + GESAMT_GEWINN
                   + GESAMT_33B + GESAMT_33B_PARTNER
                   + GESAMT_DBA + GESAMT_P23 + GESAMT_P33A + GESAMT_P32B),  # Weg ii: Abzüge + §24a/§24b + §21-Abs.2 + Vorsorge + §§13-18-Gewinn + §34c + §23 + §33a + §32b OPTIONAL
        # Pflicht-Kegel = einzel-Basis (ohne Person-B-Felder UND ohne die optionalen Abzugs-Felder); der Guard
        # erzwingt den Person-B-Kegel nur bei zusammen. Abzüge sind fail-safe optional (absent → 0). VOR_FELDER
        # (§ 10 Altersvorsorge) + KV_PV_FELDER (§ 10 KV/PV) im Kegel (mandatory) → kein stiller Über-/Unter-tax.
        "kegel": (VV_GESAMT_FELDER + ("veranlagung", "bruttoarbeitslohn")
                  + EP_FELDER + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + AN_GESAMT_FLAGS),
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est_gesamt",
        "teil_ringe": [],
        "guard": True,
        "gesamt_guard": True,   # aktiviert flag_check- + Kapital-Semantik-Guards (Einkunftsart-Konsistenz)
        # fremd_arten = Einkunftsarten, die DIESE Scheibe NICHT rechnet -> müssen abwesend bestätigt sein
        # (kein_sonstige §22). §19/§21/§20 UND §§13-18-Gewinn (Stufe 1, einkuenfte_gewinn-Slot) rechnet sie
        # -> deren Flags NICHT hier. kein_gewinn ist ab Stufe 1 KEINE fremd_art mehr; der flag_check-Guard
        # (kein_gewinn → [einkuenfte_gewinn]) fängt den Konsistenz-Widerspruch (kein_gewinn=True ∧ Gewinn>0).
        "fremd_arten": ("kein_sonstige",),
        "partner_19": True,     # § 19-Einkünfte des Ehegatten in den Ring (Zusammenveranlagung, #4)
        "multi_objekt": "vv_objekt",  # Multi-Objekt-§21-Σ (#5): der Ring summiert ALLE vv_objekt-Instanzen
    },
    # Rentner-Ring (§ 22 Leibrente + § 33b): eigene Scheibe (Feld-Ergonomie — Renten-Felder blähen den
    # AN/gesamt-Kegel nicht). Rechnet über DENSELBEN catala_gesamt-Kern (einkuenfte_sonstige = § 22-Renten-
    # Einkünfte, aussergewoehnliche_belastungen = § 33b). fremd_arten = {kein_kap, kein_vuv} — NICHT kein_sonstige
    # (die Rente IST § 22-sonstige, kein_sonstige=False ist hier korrekt) und ab 2-I NICHT kein_gewinn (der Ring
    # rechnet jetzt § 16-Veräußerungsgewinn + laufenden § 15/§ 18-Gewinn → RENTNER_GEWINN/slot_fn; kein_gewinn=False
    # ist hier korrekt statt gesperrt, exakt wie in „gesamt" ab Stufe 1). Die Konsistenz (kein_gewinn=True + vg/
    # gewinn > 0) fängt der flag_check-Guard (kein_gewinn → [einkuenfte_gewinn, rentner_veraeusserungsgewinn]).
    # § 24a=0 (Leibrenten nach § 24a S. 2 ausgeschlossen, Renten-only-MVP). partner_check LIVE (Ehegatte-Behinderung).
    "rentner_gesamt": {
        "felder": RENTNER_FELDER,
        "kegel": RENTNER_KEGEL,     # rentenfreibetrag + Partner-Behinderung nur bedingt (Guard/Accessor)
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est_rentner",
        "teil_ringe": [],
        "guard": True,
        "gesamt_guard": True,
        "rentner": True,           # aktiviert die § 22-Rentenfreibetrag-Fixierungs-Prüfung (K2)
        "multi_rente": "rente",    # Multi-Rente-§22-Σ (#6): der Ring summiert ALLE rente-Instanzen (aa/bb je Rente)
        "fremd_arten": ("kein_kap", "kein_vuv"),
    },
    # DEPRECATED (Weg ii, Stage 1b): die Standalone-Scheiben haushalt_gesamt (§35a+§10b) + agb_gesamt (§33+§10-
    # KiSt) sind ENTFERNT — ihre Abzüge sind in den EINEN gesamt-Ring gefaltet (GESAMT_ABZUEGE, additiv auf JEDE
    # Einkunfts-Kombi, ECHTE GdE via catala_gesamt_gde). Die Accessoren (catala_p35a/p10b/p33/p10_kist/zumutbar)
    # BLEIBEN — der gesamt-slot_fn ruft sie. Grund: sonst zwei GdE-Wahrheiten (Standalone §19-only vs Fold echt).
}

_FALL_RE = re.compile(r"[A-Za-z0-9_-]{1,64}")


class ApiError(ValueError):
    """Trägt einen HTTP-Status mit (fail-closed-Antwort statt 500)."""
    def __init__(self, status: int, msg: str):
        super().__init__(msg)
        self.status = status


# ----------------------------------------------------------------- Fall-Persistenz (atomar JSON)

def _fall_pfad(fall_id: str) -> str:
    if not _FALL_RE.fullmatch(fall_id):
        raise ApiError(400, f"ungültige fall_id (nur [A-Za-z0-9_-]{{1,64}}): {fall_id!r}")
    return os.path.join(FAELLE, f"{fall_id}.json")


def lade_fall(fall_id: str) -> dict:
    p = _fall_pfad(fall_id)
    if not os.path.exists(p):
        raise ApiError(404, f"Fall {fall_id!r} existiert nicht")
    with open(p, encoding="utf-8") as f:
        return json.load(f)


def speichere_fall(fall_id: str, store: dict) -> None:
    p = _fall_pfad(fall_id)
    os.makedirs(FAELLE, exist_ok=True)
    tmp = tempfile.NamedTemporaryFile("w", dir=FAELLE, delete=False, encoding="utf-8", suffix=".tmp")
    try:
        json.dump(store, tmp, ensure_ascii=False)
        tmp.flush()
        os.fsync(tmp.fileno())
    finally:
        tmp.close()
    os.replace(tmp.name, p)


# ----------------------------------------------------------------- Scheibe -> Bindung/Engine

def _cfg(store: dict) -> dict:
    sch = store.get("scheibe")
    if sch not in SCHEIBEN:
        raise ApiError(400, f"unbekannte Scheibe {sch!r}")
    return SCHEIBEN[sch]


def _datei_felder(dateiname: str) -> tuple:
    import yaml
    d = yaml.safe_load(open(os.path.join(PRODUKT, "bindung", dateiname), encoding="utf-8"))
    return tuple(b["feld_id"] for b in d.get("bindungen", []))


def _scheibe_felder(store: dict) -> tuple:
    cfg = _cfg(store)
    return cfg["felder"] if cfg["felder"] is not None else _datei_felder(cfg["felder_datei"])


def _scheibe_bindung(store: dict) -> dict:
    felder = _scheibe_felder(store)
    b = TR.lade_bindung()
    fehlend = [f for f in felder if f not in b]
    if fehlend:
        raise ApiError(500, f"Bindungstabelle unvollständig für Scheibe: {fehlend}")
    return {f: b[f] for f in felder}


def _abs3_eligible(f: dict, vz: int) -> bool:
    """§ 34 Abs. 3 S. 1: (Alter ≥ 55 [DERIVE aus geburtsjahr] ODER dauernd berufsunfähig) UND § 34 Abs. 3 S. 4 nicht
    schon einmal genutzt. SHARED zwischen Chooser (_festzusetzende) und Guard (_an_gesamt_sperrgrund) — bit-identisch,
    sonst Guard/Chooser-Drift. antrag_ermaessigter_satz (S. 1 „auf Antrag") wird vom Aufrufer separat geprüft."""
    gj = f.get("geburtsjahr", {}).get("wert")
    gj = int(gj) if isinstance(gj, (int, float)) and not isinstance(gj, bool) else 0
    alter_ge_55 = (vz - gj) >= 55 if gj > 0 else False
    berufsunfaehig = f.get("dauernd_berufsunfaehig", {}).get("wert") is True
    einmal_genutzt = f.get("ermaessigung_einmal_genutzt", {}).get("wert") is True
    return (alter_ge_55 or berufsunfaehig) and not einmal_genutzt


def _gwg_sofortabzug_summe(f: dict, store: dict | None, bindung: dict | None,
                           nur_bestaetigt: bool = True) -> int:
    """§ 6 Abs. 2 GWG-Sofortabzug-Σ (EURO, Stufe 2b) — STUMPFE Σ über ALLE gwg-Instanzen: je Asset
    catala_p6_2_gwg (≤ 800 netto → Sofortabzug, sonst 0 = kein GWG, gehört in die AfA). instanzen-Naht wie
    Multi-Objekt-§21/Multi-Rente-§22 (EM.instanzen(gwg); Basis-Feld = Instanz 1). Ohne store (Alt-Aufrufer) nur
    das Basis-gwg_anschaffungskosten_netto aus f. Naht-CENT → EURO (Accessor nimmt EURO). Ein > 800-„GWG" gibt 0
    (over-tax-safe); der User-Hinweis (hilfe_kurz) lenkt auf die AfA."""
    import runner
    def _abzug(fi: dict) -> int:
        v = fi.get("gwg_anschaffungskosten_netto", {}).get("wert")
        netto = int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
        # ⭐ CENT-GUARD (§ 6 Abs. 2, 800€-Schwelle): netto//100 FLOORT Cent vor dem catala-≤800-Vergleich
        # → 800,01-800,99€ (80001-80099 Cent) würde auf 800 abgerundet fälschlich als GWG durchgehen
        # (under-tax, Sofortabzug statt AfA). Schwelle VOR der Euro-Rundung in Cent prüfen.
        if netto > 80000:
            return 0
        return runner.catala_p6_2_gwg({"gwg_anschaffungskosten_netto": netto // 100})
    if store is not None and bindung is not None:
        # ⭐ SECURITY (Zwei-Signal am Ring, INSTANZ-Pfad): EM.instanzen liest den STORE separat vom bestätigt-
        # gefilterten _bescheid_fn-Snapshot → eine VORLÄUFIGE gwg-Instanz bewegte sonst den Sofortabzug OHNE
        # Confirm (dev-2-Repro: 600€ am Ring). gwg ist OPTIONAL → KEIN Kegel-/Sperr-Gate (anders als vv/rente) →
        # bei nur_bestaetigt=True (festgesetzt) ist der Filter PFLICHT. nur_bestaetigt=False (Estimate /stand) zeigt
        # die vorläufige Wirkung im Range (Parität zum agB-Skalar). inst["zustand"] = per-Instanz-meet.
        return sum(_abzug(inst["felder"]) for inst in EM.instanzen(store, bindung, "gwg")
                   if not nur_bestaetigt or inst["zustand"] == "bestaetigt")
    return _abzug(f)


def _laufender_gewinn(f: dict, store: dict | None = None, bindung: dict | None = None,
                      nur_bestaetigt: bool = True):
    """§§ 13-18 laufender Gewinn (§ 15 Gewerbe / § 18 selbständig), EURO — die EINE Quelle für den laufenden
    (Nicht-Veräußerungs-)Gewinn, geteilt von gesamt- und rentner-Ring (Scope A). § 4 Abs. 3 EÜR (Stufe 2a/2b) wenn
    IRGENDEINE EÜR-Komponente (betriebseinnahmen/sonstige_betriebsausgaben/afa_jahresbetrag) ODER ein GWG vorliegt:
    gewinn = betriebseinnahmen − (sonstige_BA + AfA + GWG-Σ) via catala_euer_gewinn — KANN NEGATIV sein (Verlustjahr
    → § 2 Abs. 3-Ausgleich mindert andere Einkünfte); sonst der direkte einkuenfte_gewinn-Wert (Stufe 1). Der
    GWG-Sofortabzug (§ 6 Abs. 2, Stufe 2b) mindert als Betriebsausgabe den Gewinn (Σ über alle Assets). Naht-CENT
    → EURO. BEIDE Quellen (Direktwert + EÜR) sperrt gewinn_quelle_offen VORHER; land_forst + EÜR sperrt luf_euer_offen.

    Returns (laufender_gewinn, mitu): laufender_gewinn = kompletter §2-Gewinn (inkl. mitu-Summand);
    mitu = §15-Mitunternehmer-Komponente (immer gewerbesteuerpflichtig, §35-Zähler)."""
    import runner
    def _c(fid):
        v = f.get(fid, {}).get("wert")
        return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
    gwg_summe = _gwg_sofortabzug_summe(f, store, bindung, nur_bestaetigt)
    # § 15 Abs. 1 S. 1 Nr. 2 Mitunternehmer (#2): SEPARATER §15-gewerblicher Summand (Beteiligung an PersG, additiv
    # zum eigenen Gewerbe/EÜR — KEIN gewinn_quelle_offen-Konflikt, eigene Felder). gewinnanteil = §15a-ausgleichs-
    # fähiger Anteil (kann NEGATIV, roh summiert). Hier IN _laufender_gewinn → symmetrisch in §35-Zähler+Nenner.
    # BOUNDED ASSUMPTION: mitu = ausschl. §15 gewerblich (Anlage G). §18-freiberufl-Mitunternehmerschaft wäre
    # NICHT gewerbesteuerpflichtig → bräuchte eigenes Flag (out-of-scope, Backlog). Instructor-Ruling §35-Mitu.
    mitu = runner.catala_mitunternehmer_einkuenfte({
        "gewinnanteil": _c("gewinnanteil") // 100,
        "verguetung_taetigkeit": _c("verguetung_taetigkeit") // 100,
        "verguetung_darlehen": _c("verguetung_darlehen") // 100,
        "verguetung_ueberlassung": _c("verguetung_ueberlassung") // 100,
    }) if any(_c(k) for k in MITU_FELDER) else 0
    if any(_c(k) for k in EUER_KOMPONENTEN) or gwg_summe > 0:
        gewinn = runner.catala_euer_gewinn({
            "betriebseinnahmen": _c("betriebseinnahmen") // 100,
            "betriebsausgaben": (_c("sonstige_betriebsausgaben") + _c("afa_jahresbetrag")) // 100 + gwg_summe}) + mitu
    else:
        gewinn = _c("einkuenfte_gewinn") // 100 + mitu
    return gewinn, mitu


def _p23_ansonsten_einkuenfte(f: dict, store: dict | None, bindung: dict | None,
                                nur_bestaetigt: bool = True) -> int:
    """§23 Private Veräußerungsgeschäfte (Stufe-1), EURO — Σ über ALLE p23_veraeusserung-Instanzen:
    je Veräußerung: gewinn = veraeusserungspreis − AK/HK − WK (cent→euro); Σ pos → gewinn_pvg,
    Σ neg-Beträge → verlust_pvg; Gesamtgewinn → Freigrenze 1000€ (§23 Abs.3 S.5, 999→0/1000→1000);
    Verlusttopf max(0, gewinn_pvg−verlust_pvg) (§23 Abs.3 S.7) = anzusetzende_einkuenfte → ADDITIV
    in einkuenfte_sonstige. Mehrjahr-Verlustvor-/rücktrag (S.8) = Stufe-2-Backlog. Absent→0 (safe)."""
    import runner
    if store is None or bindung is None:
        return 0
    from produkt.mapping import est_mapping as EM
    instanzen = EM.instanzen(store, bindung, "p23_veraeusserung")
    gewinn_pvg = 0
    verlust_pvg = 0
    for inst in instanzen:
        # norm: inst["felder"] nutzt Basis-feld_ids (OHNE __n-Suffix)
        preis = int(inst["felder"].get("p23_veraeusserungspreis", 0)) // 100
        ak = int(inst["felder"].get("p23_anschaffung_herstellungskosten", 0)) // 100
        wk = int(inst["felder"].get("p23_werbungskosten", 0)) // 100
        gewinn = runner.catala_p23_veraeusserungsgewinn({
            "veraeusserungspreis": preis, "anschaffungs_herstellungskosten": ak, "werbungskosten": wk})
        if gewinn > 0:
            gewinn_pvg += gewinn
        else:
            verlust_pvg += abs(gewinn)
    # Freigrenze auf Gesamtsumme positive+negative = gesamtgewinn
    gesamtgewinn = gewinn_pvg - verlust_pvg
    steuerpflichtig = runner.catala_p23_freigrenze({"gesamtgewinn": gesamtgewinn})
    if steuerpflichtig <= 0:
        return 0
    # Verlusttopf (same-year) auf die FREIGRENZEN-gerechneten Einzelkomponenten
    return runner.catala_p23_verlusttopf({"gewinn_pvg": gewinn_pvg, "verlust_pvg": verlust_pvg})


def _oepnv_eur(slots: dict) -> int:
    """oepnv_kosten_jahr Naht-CENT -> EURO (Store liefert Cent, EP_FELDER-Runner-Accessor erwartet Euro)."""
    return int(slots.get("oepnv_kosten_jahr", 0)) // 100


def _bescheid_fn(quantitaet: str, vz: int, bindung: dict, felder: dict | None = None,
                 store: dict | None = None, nur_bestaetigt: bool = True, solz_container=None,
                 extras: dict | None = None):
    """bescheid_fn(feld_werte)->cent für eine ring-fähige Familie (Naht-Einheit CENT via
    intervall.bescheid_via_slots). None, wenn die Catala-Toolchain oder ein Accessor fehlt —
    dann bleibt der Ring ehrlich leer, nie ein erfundener Betrag. `felder` (materialisierter
    Store-Snapshot) erlaubt den Zugriff auf Einzelfelder, die die Summen-Slots verdecken (VOR-AG).
    `store` (optional) erlaubt die Multi-Objekt-Instanz-Enumeration (est_mapping.instanzen, #5) — ohne
    store rechnet der §21-Ring nur die Basis-Instanz (Alt-Aufrufer/Teil-Ringe).

    `nur_bestaetigt` (DEFAULT True = fail-safe): filtert den felder-Snapshot (+ die Instanz-Σ) auf bestätigt-
    only → für die FESTGESETZTE Steuer (/ergebnis via _feste_zahl). False NUR in den Estimate-Pfaden (/stand +
    fragen), die die [min,max]-Spanne bauen und NIE die festgesetzte Steuer emittieren — dort SOLL die vorläufige
    Wirkung sichtbar sein (Steuer-at-Risk-Range, Instructor-Vertrag). Default True hält jeden neuen Caller sicher."""
    # ⭐ SECURITY — Zwei-Signal-Invariant AM RING: bei nur_bestaetigt=True liest der Bescheid AUSSCHLIESSLICH
    # bestätigte Werte. Ein VORLÄUFIGER Vorschlag (llm:chat / import:beleg / kontoauszug / vorjahr / berechnet:maps)
    # für ein OPTIONALES Feld (agB §33, Spenden §10b, Berufsausbildung §10, Mitunternehmer §15, §16-vg … — NICHT im
    # Pflicht-Kegel, also NICHT vom Meet-Gate in _feste_zahl erfasst) darf die festgesetzte Steuer NIE bewegen,
    # bevor der Mensch signal_2 gesetzt hat ("ein Vorschlag bewegt die Summe nie ohne Confirm"). Ohne den Filter
    # las _c/_cent/_b den Roh-Aktiv-Wert (zustand-blind) → ein vorläufiger llm:chat-agB senkte /ergebnis (604k
    # statt 691k) OHNE Confirm. vorläufig → hier absent → 0 → kein Abzug bis zum Confirm (over-tax-safe). Kegel-
    # Felder sind ohnehin alle bestätigt (Gate in _feste_zahl). Der Sperr-Guard (_an_gesamt_sperrgrund) sieht die
    # Roh-felder SEPARAT weiter (ein vorläufiges nicht-ring-fähiges Feld muss den Ring weiter sperren). Die store-
    # basierte Instanz-Σ (est_mapping.instanzen) liest den store separat → nur_bestaetigt wird DURCHgefädelt
    # (_gwg_sofortabzug_summe/_laufender_gewinn + vv/rente-Σ inline), sonst zeigte /stand die gwg-Instanz-Wirkung nicht.
    if nur_bestaetigt and felder:
        felder = {fid: ev for fid, ev in felder.items() if ev.get("zustand") == "bestaetigt"}
    if quantitaet == "abziehbarer_betrag":          # § 9 Entfernungspauschale
        try:
            import runner  # noqa: F401
        except Exception:
            return None

        def slot_fn(slots: dict) -> int:
            s = {"veranlagungszeitraum": int(vz),
                 "arbeitstage": int(slots.get("arbeitstage", 0)),
                 "entfernung_km_roh": int(slots.get("entfernung_km_roh", 0)),
                 "oepnv_kosten_jahr": _oepnv_eur(slots),
                 "eigenes_oder_ueberlassenes_kfz": bool(slots.get("eigenes_oder_ueberlassenes_kfz", False))}
            return runner.catala_entfernungspauschale(s)

        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="abziehbarer_betrag")

    if quantitaet == "festzusetzende_est":          # § 2 Gesamtsteuer MVP (reiner AN-Fall)
        try:
            import runner  # noqa: F401
        except Exception:
            return None

        f = felder or {}

        def _cent(fid):
            v = f.get(fid, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

        def slot_fn(slots: dict) -> int:
            wk_input = {"veranlagungszeitraum": vz,
                        **{k: slots[k] for k in
                           ("arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr", "eigenes_oder_ueberlassenes_kfz")
                           if k in slots}}
            if "oepnv_kosten_jahr" in wk_input:
                wk_input["oepnv_kosten_jahr"] = _oepnv_eur(wk_input)   # Naht-CENT -> EURO
            # doppelte Haushaltsführung (Stufe 1b): dHf-Abzug NUR bei erfülltem Tatbestand —
            # Kosten > 0, Inland, alle 4 Geltungsbedingungen bestätigt-true. Sonst legitim 0
            # (Bedingung bestätigt-false = kein dHf); offene Bedingung/Ausland sperrt der Guard.
            if (_cent(DHF_KOSTEN) > 0 and f.get("dhf_im_inland", {}).get("wert") is True
                    and all(f.get(b, {}).get("wert") is True for b in DHF_BEDINGUNGEN)):
                wk_input["unterkunftskosten_monat"] = _cent(DHF_KOSTEN) // 100    # cent -> euro
                wk_input["monate"] = _cent("dhf_monate")
                wk_input["im_inland"] = True
            # Verpflegung (Stufe 1b): Tage je Kategorie in den Roh-WK, NUR wenn Reduktion explizit
            # safe (≤ 3 Monate + keine Mahlzeitengestellung); fail-closed bei UNSET (Guard sperrt
            # den Ring dann sowieso, hier doppelt sicher gegen Über-Abzug). Tage sind Anzahl (kein cent).
            _mon = f.get("vpf_monate_am_ort", {}).get("wert")
            if (sum(_cent(t) for t in VERPFLEGUNG_TAGE) > 0
                    and isinstance(_mon, int) and not isinstance(_mon, bool) and _mon <= 3
                    and f.get("vpf_keine_mahlzeitengestellung", {}).get("wert") is True):
                for t in VERPFLEGUNG_TAGE:
                    wk_input[t] = _cent(t)
            wk = runner.catala_werbungskosten_n(wk_input)   # Person A: EP + dHf + Verpflegung, roh
            # § 10 Abs. 1 Nr. 3/3a KV/PV-Vorsorge (Pflicht-Kegel Person A, Gesamt-Parität, Over-tax-Fix):
            # eigener Abs.4-Höchstbetrag (1900/2800), additiv, GETRENNT von der VOR-Basisvorsorge unten.
            kv_pv_a = runner.catala_p10_kv_pv({
                "basis_kv_pv": _cent("basis_kv_pv") // 100,
                "weitere_vorsorgeaufwendungen": _cent("weitere_vorsorgeaufwendungen") // 100,
                "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss", {}).get("wert") is True})
            # Zusammenveranlagung (§ 26b): Roh-Bruttolohn + Roh-WK pro Person -> catala_est_zusammen
            # (Pauschbetrag je Ehegatte + Splitting IM Scope). MVP: Person B ohne gesonderte WK (0),
            # ohne VOR (Partner-VOR sperrt der Guard). Person-B-KV/PV optional (absent -> 0, eigener
            # Höchstbetrag je Person, additiv wie in gesamt) — Person-B-WK/VOR bleiben Folge-Nachträge.
            zusammen = f.get("veranlagung", {}).get("wert") == "zusammen"
            if zusammen:
                kv_pv_b = runner.catala_p10_kv_pv({
                    "basis_kv_pv": _cent("basis_kv_pv_partner") // 100,
                    "weitere_vorsorgeaufwendungen": _cent("weitere_vorsorgeaufwendungen_partner") // 100,
                    "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss_partner", {}).get("wert") is True})
                est = runner.catala_est_zusammen({
                    "veranlagungszeitraum": vz,
                    "bruttoarbeitslohn_a": int(slots.get("bruttoarbeitslohn", 0)) // 100,
                    "bruttoarbeitslohn_b": _cent("bruttoarbeitslohn_partner") // 100,
                    "werbungskosten_a": wk, "werbungskosten_b": 0,
                    "sonderausgaben_gemeinsam": kv_pv_a + kv_pv_b})
            else:
                # § 10 Altersvorsorge (Stufe 1a): die VOR-Einzelfelder DIREKT aus dem Store greifen —
                # der Summen-Slot gesamtbeitraege_inkl_ag würde den AG-Anteil verschmelzen und die
                # Kürzung (nach dem Cap) unmöglich machen. gesamtbeitraege = AN + AG + außerhalb; der
                # steuerfreie AG-Anteil getrennt. Naht-CENT -> EURO für _vorsorge_abzug.
                gesamt = (_cent("vor_an_anteil_rv") + _cent("vor_ag_anteil_rv")
                          + _cent("vor_rv_ausserhalb_lstb")) // 100
                ag = _cent("vor_ag_anteil_rv") // 100
                so = runner._vorsorge_abzug({"vorsorge_gesamtbeitraege_inkl_ag": gesamt,
                                             "vorsorge_ag_anteil_steuerfrei": ag}, vz) + kv_pv_a
                est = runner.catala_est({
                    "veranlagungszeitraum": vz,
                    "veranlagung": slots.get("veranlagung", "einzel"),
                    # bruttoarbeitslohn ist Naht-CENT (Bindung typ:cent) -> catala_est erwartet EURO.
                    "bruttoarbeitslohn": int(slots.get("bruttoarbeitslohn", 0)) // 100,
                    "werbungskosten": wk,
                    "sonderausgaben": so})
            # SolZ §3, §4 SolzG: Basis = festzusetzende ESt (kein KiFB/§32d-Kapital im AN-Ring)
            if solz_container is not None:
                solz_container[0] = runner.catala_solz({
                    "veranlagungszeitraum": vz,
                    "bemessungsgrundlage": est,
                    "splitting": zusammen})
            # KiSt § 51a: dieselbe Maßstabsteuer wie SolZ (reiner AN-Fall: kein KiFB/§32d)
            if extras is not None:
                extras["kist_cent"] = runner.catala_kist({
                    "est_mit_fb": est,
                    "konfession": f.get("kist_konfession", {}).get("wert", "keine"),
                    "bundesland": f.get("kist_bundesland", {}).get("wert", "")})
            # § 101 Mobilitätsprämie (Post-Engine-Prämie, KEIN ESt-Impact; extras-Naht wie KiSt).
            # Stufe-1: reiner-AN einzel mit Pendlerstrecke (ab-21km-EP). zusammen = Stufe-2 (per-
            # Ehegatte-S.3 + doppelter GFB, S.2 Hs.2). Ohne Entfernung → kein § 101 (Feld absent →
            # mobilitaetspraemie_cent bleibt None). Prämie zahlt nur bei zvE < GFB (Accessor: 0 sonst).
            if (extras is not None and not zusammen
                    and int(slots.get("entfernung_km_roh", 0)) > 0):
                ep_ab_21 = runner.catala_ep_ab_21km({
                    "veranlagungszeitraum": vz,
                    "arbeitstage": int(slots.get("arbeitstage", 0)),
                    "entfernung_km_roh": int(slots.get("entfernung_km_roh", 0)),
                    "eigenes_oder_ueberlassenes_kfz": bool(slots.get("eigenes_oder_ueberlassenes_kfz", False)),
                    "oepnv_kosten_jahr": _oepnv_eur(slots)})
                extras["mobilitaetspraemie_cent"] = runner.catala_p101_mobilitaetspraemie_cent({
                    "entfernungspauschale_ab_21km": ep_ab_21,
                    "zu_versteuerndes_einkommen": runner.catala_est_einzel_zve({
                        "veranlagungszeitraum": vz,
                        "bruttoarbeitslohn": int(slots.get("bruttoarbeitslohn", 0)) // 100,
                        "werbungskosten": wk, "sonderausgaben": so}),
                    "grundfreibetrag": runner.catala_grundfreibetrag(vz),
                    "ist_arbeitnehmer": True,                 # § 101 S. 3 (AN-Pauschbetrag-soweit)
                    "werbungskosten_gesamt": wk,              # roh AN-WK inkl. voller EP
                    "arbeitnehmer_pauschbetrag": runner.catala_arbeitnehmer_pauschbetrag(vz)})
            return est
        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")

    if quantitaet == "festzusetzende_est_gesamt":   # § 21 V+V via catala_gesamt (reiner §21-MVP)
        try:
            import runner  # noqa: F401
        except Exception:
            return None
        f = felder or {}

        def _c(fid):
            v = f.get(fid, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

        def _vv_objekt(fi: dict) -> int:
            # § 21 Überschuss EINES Objekts (Einnahmen − Werbungskosten), Naht-CENT -> EURO. KEIN per-
            # Objekt-Floor (catala_vermietung_einkuenfte gibt Verluste durch → horizontaler Verlustausgleich
            # innerhalb § 21; der Floor kommt erst im gesamt-Scope, § 2 Abs. 3). fi = das je-Instanz auf die
            # Basis-feld_id normierte Feld-Dict (instanzen-Naht) ODER die felder-closure f (Basis-Fallback).
            def _ci(k):
                v = fi.get(k, {}).get("wert")
                return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
            def _bi(k):
                return fi.get(k, {}).get("wert")
            vv_voll = runner.catala_vermietung_einkuenfte({
                "einnahmen": _ci("vv_einnahmen") // 100,
                "gebaeude_afa": _ci("vv_gebaeude_afa") // 100,
                "schuldzinsen": _ci("vv_schuldzinsen") // 100,
                "erhaltungsaufwand": _ci("vv_erhaltungsaufwand") // 100,
                "sonstige_werbungskosten": _ci("vv_sonstige_wk") // 100})
            # § 21 Abs. 2: verbilligte Wohnraumvermietung (Entgelt < 66 % ortsüblich) → WK nur anteilig (× quote/
            # 100), das erhöht die § 21-Einkünfte (mehr Steuer, K2-sichere Richtung). NUR bei Tatbestand Wohnzwecke
            # + auf Dauer (§ 21 Abs. 2 S. 1); gewerblich/nicht-dauer (Feld = False) → volle WK. quote 100 (default)
            # ≥ 66 → keine Kürzung. PER OBJEKT (jede Instanz eigene Quote/Tatbestand). Umgesetzt als Add-back der
            # nicht abziehbaren WK auf den Voll-Überschuss (behält catala_vermietung_einkuenfte + dessen Konsistenz-
            # Gate im Pfad). quote im Pflicht-Kegel → immer beantwortet (kein stiller Unter-Abzug).
            # C-Fix (K2, Under-tax): quote=0 = UNENTGELTLICHE Überlassung (keine Einkünfteerzielungsabsicht, § 21
            # greift nicht) → dieses Objekt trägt seine Einnahmen OHNE WK-Abzug bei = kein WK-Verlust der die § 19
            # senkt. Vorher kollabierte `_ci or 100` die 0 auf 100 → voller WK-Verlust = Under-tax. Robust ggü.
            # Extrem-verbilligt (Mini-Miete, quote ganzzahlig auf 0 gerundet, einnahmen > 0): Einnahmen STATT hart 0
            # (kein neuer Under-tax; bei einnahmen 0 = die reine unentgeltliche Überlassung → 0). absent (nur Alt-
            # Aufrufer/Teil-Ring, im echten Ring nie: Pflicht-Kegel) → 100 (nicht verbilligt).
            quote_raw = _bi("vv_entgelt_quote_prozent")
            quote_present = isinstance(quote_raw, (int, float)) and not isinstance(quote_raw, bool)
            if quote_present and quote_raw == 0:
                return _ci("vv_einnahmen") // 100   # unentgeltlich → Einnahmen ohne WK, kein Verlust
            quote = int(quote_raw) if quote_present else 100
            tatbestand = (_bi("vv_wohnzwecke") is not False) and (_bi("vv_auf_dauer") is not False)
            if tatbestand and 0 < quote < 66:
                wk_voll = (_ci("vv_gebaeude_afa") + _ci("vv_schuldzinsen")
                           + _ci("vv_erhaltungsaufwand") + _ci("vv_sonstige_wk")) // 100
                wk_abziehbar = runner.catala_p21_2_verbilligt({
                    "werbungskosten": wk_voll, "entgelt_quote_prozent": quote})
                return vv_voll + (wk_voll - wk_abziehbar)   # nicht abziehbare WK zurück → höhere Einkünfte
            return vv_voll

        def slot_fn(slots: dict) -> int:
            # § 21 Überschuss je Objekt, dann STUMPFE Σ über ALLE vv_objekt-Instanzen (Multi-Objekt, #5):
            # est_mapping.instanzen (dev-2s Ring-Naht, EINE Enumerations-Wahrheit — index==1 = Basis-vv-Felder,
            # __n = weitere Objekte, je Instanz auf die Basis-feld_id normiert). Ohne store (Teil-Ring/Alt-
            # Aufrufer) nur die Basis aus f. Unvollständige Instanzen fängt der Guard VOR diesem Aufruf ab.
            if store is not None:
                # Zwei-Signal-Filter (Instanz-Pfad, nur_bestaetigt=True): vorläufige Objekt-Instanz nie in die Σ.
                # Defense-in-depth — der vv_instanz_offen-Guard (_an_gesamt_sperrgrund) sperrt index≥2-vorläufig schon
                # VOR diesem Aufruf, die Basis liegt im Kegel-Meet — aber konsistent zum gwg-Σ (refactor-sicher).
                # nur_bestaetigt=False (/stand) zeigt die vorläufige Wirkung im Range.
                vv = sum(_vv_objekt(inst["felder"])
                         for inst in EM.instanzen(store, bindung, "vv_objekt")
                         if not nur_bestaetigt or inst["zustand"] == "bestaetigt")
            else:
                vv = _vv_objekt(f)
            g = {"gesamtfall": True, "veranlagungszeitraum": vz,
                 "veranlagung": slots.get("veranlagung", "einzel"),
                 "einkuenfte_vermietung": vv}
            # kombiniert §19+§21: Bruttolohn im Kegel -> §19-Einkünfte (§9a-bereinigt, § 2 Abs. 2 Nr. 2)
            # als einkuenfte_nichtselbststaendig in die §-2-Summe; der §21-Verlust mindert dann den
            # §19-Lohn (§ 2 Abs. 3). Bruttolohn 0 (reiner Vermieter) -> einkuenfte_ns 0, kein Effekt.
            # §19-WK = Entfernungspauschale (roh, § 9a-Günstiger im einzel-Tarif); dHf/Verpflegung/AM
            # sind hier NICHT modelliert (Folge-Nachtrag) — es gibt keine solchen Slots in der Scheibe.
            gesamt_wk_input = {"veranlagungszeitraum": vz,
                **{k: slots[k] for k in
                   ("arbeitstage", "entfernung_km_roh", "oepnv_kosten_jahr", "eigenes_oder_ueberlassenes_kfz")
                   if k in slots}}
            if "oepnv_kosten_jahr" in gesamt_wk_input:
                gesamt_wk_input["oepnv_kosten_jahr"] = _oepnv_eur(gesamt_wk_input)   # Naht-CENT -> EURO
            ns_wk = runner.catala_werbungskosten_n(gesamt_wk_input)
            ns = runner.catala_einkuenfte_nichtselbststaendig({
                "veranlagungszeitraum": vz,
                "bruttoarbeitslohn": int(slots.get("bruttoarbeitslohn", 0)) // 100,   # Naht-CENT -> EURO
                "werbungskosten": ns_wk})
            # Person B (§ 26b Zusammenveranlagung, #4): die § 19-Einkünfte des Ehegatten (§9a-bereinigt JE
            # PERSON) in DIESELBE einkuenfte_nichtselbststaendig-Summe — kein _a/_b-Split, der Gesamt-Scope
            # rechnet Splitting + doppelten § 10c aus veranlagung=zusammen (handverifiziert: gesamt(zusammen,
            # ns_A+ns_B) == catala_est_zusammen(brutto_A, brutto_B)). Person-B-WK MVP 0 (Folge-Nachtrag).
            if g["veranlagung"] == "zusammen":
                ns += runner.catala_einkuenfte_nichtselbststaendig({
                    "veranlagungszeitraum": vz,
                    "bruttoarbeitslohn": _c("bruttoarbeitslohn_partner") // 100, "werbungskosten": 0})
            g["einkuenfte_nichtselbststaendig"] = ns
            # §§ 13-18 Gewinneinkünfte (Stufe 1 + 2a): der laufende Gewinn als einkuenfte_gewinn-Summand in die
            # § 2-Summe (Engine-Slot einkuenfte_gewinn_in LIVE, runner.py _gesamt_out). _laufender_gewinn wählt
            # Stufe 2a (EÜR § 4 Abs. 3, betriebseinnahmen − BA komponentenweise) wenn eine EÜR-Komponente vorliegt,
            # sonst den direkten Stufe-1-Betrag. OPTIONAL (absent → 0, over-tax-safe). Doppelquelle/land_forst +
            # Einkunftsart-Konsistenz (kein_gewinn=True) fangen gewinn_quelle_offen/luf_euer_offen/flag_check vorher.
            # § 16 Veräußerungsgewinn (Non-Rentner-§16-vg, REUSE des generellen §16-vg-Felds): netto nach § 16 Abs. 4-
            # Freibetrag (catala_p16_4_freibetrag + rentner-2-I-Fold gespiegelt; der „rentner_"-Feldname ist NICHT
            # rentner-spezifisch, Kz Anlage G/S). GEFLOORT bei 0 (FB > vg → kein Phantom-Verlust) + ADDITIV in
            # einkuenfte_gewinn (§ 16 Abs. 1: Veräußerungs- + laufender Gewinn = dieselbe § 2-Einkunftsart). Absent → 0.
            vg_euro = _c("rentner_veraeusserungsgewinn") // 100
            netto_vg = max(0, vg_euro - runner.catala_p16_4_freibetrag({"rentner_veraeusserungsgewinn": vg_euro}))
            laufender_gewinn, mitu = _laufender_gewinn(f, store, bindung, nur_bestaetigt)   # § 15/§ 18 laufend (für § 35-Zähler, OHNE § 16-vg)
            g["einkuenfte_gewinn"] = laufender_gewinn + netto_vg
            # § 24a/§ 24b Freibeträge (Weg ii Stage 2, § 2 Abs. 3 — MINDERN den GdE VOR den Abzügen): § 24a
            # Altersentlastungsbetrag (§24a S.1: Arbeitslohn BRUTTO + max(0, positive Summe der Nicht-§19-Einkünfte =
            # V+V + §§13-18-Gewinn; Leibrenten/Versorgungsbez. raus S.2; Kohorten-Satz/-Deckel aus geburtsjahr + 65;
            # §20-tarifl.-Kapital-Günstiger = seltener Nachtrag) + § 24b Entlastungsbetrag Alleinerziehende (fam_alleinstehend IST das §24b-Abs.3-Flag — quelle
            # p24b/alleinstehend, fragetext „ohne anderen Erwachsenen im Haushalt"; anzahl_kinder + monate). Absent
            # → 0 (fail-safe). Beide fließen in den GdE-Zwilling (echte GdE post § 24a/§24b für die §10b/§33-
            # Deckelung) UND in g (est).
            alt24a = runner.catala_p24a_altersentlastung({
                "veranlagungszeitraum": vz,   # § 24a S. 3 64+-Gate (geburtsjahr+65 ≤ VZ)
                "geburtsjahr": _c("geburtsjahr"),
                "arbeitslohn": _c("bruttoarbeitslohn") // 100,
                # §23 (§22Nr.2) gehört IN die §24a-Bemessung (S.2-Ausschluss nennt nur §22Nr.1/4/5)
                "positive_andere_einkuenfte": max(0, vv + g["einkuenfte_gewinn"]
                                                   + g.get("einkuenfte_sonstige", 0))})
            # § 24a PER PERSON (§ 24a S. 1 „der Steuerpflichtige", A.2): bei Zusammenveranlagung hat der Ehegatte
            # eine EIGENE Kohorte (geburtsjahr_partner + 65) + eigene Bemessung (bruttoarbeitslohn_partner; positive
            # andere Einkünfte-B = 0, da vv/Kapital im Ring nicht owner-getrennt = konservativ/over-tax-safe, mit
            # dev-2 abgestimmt). Absent (geburtsjahr_partner ≤ 0) → 0 (fail-safe). Additiv zu Person A.
            alt24a_b = runner.catala_p24a_altersentlastung({
                "veranlagungszeitraum": vz,   # § 24a S. 3 64+-Gate (Person B: geburtsjahr_partner+65 ≤ VZ)
                "geburtsjahr": _c("geburtsjahr_partner"),
                "arbeitslohn": _c("bruttoarbeitslohn_partner") // 100,
                "positive_andere_einkuenfte": 0}) if g["veranlagung"] == "zusammen" else 0
            ent24b = runner.catala_p24b_entlastung({
                "alleinstehend": f.get("fam_alleinstehend", {}).get("wert") is True,
                "anzahl_kinder": _c("fam_anzahl_kinder"),
                "monate_ohne_voraussetzung": _c("fam_monate_ohne_voraussetzung")})
            g["altersentlastungsbetrag"] = alt24a + alt24a_b
            g["entlastungsbetrag_alleinerziehende"] = ent24b
            # § 10 Abs. 1 Nr. 2/Abs. 3 Altersvorsorge (Basisvorsorge RV): die 3 VOR-Felder DIREKT aus dem Store —
            # gesamtbeitraege = AN + AG + außerhalb LStB, der steuerfreie AG-Anteil getrennt (Kürzung NACH dem
            # knappschaft-Höchstbetrag-Cap). catala_gesamt ruft _vorsorge_abzug(s) SCHON intern (runner.py Z.759,
            # addiert auf sonderausgaben nach _sonderausgaben_final) → hier nur die Slots setzen, kein Doppelzählen.
            # Naht-CENT → EURO. Absent → 0 (im Pflicht-Kegel, also immer gefragt: kein stiller Über-tax). Person-B-
            # VOR (vor_*_partner) + zusammen-VOR = Nachtrag wie an_gesamt (MVP Person-A-Altersvorsorge).
            g["vorsorge_gesamtbeitraege_inkl_ag"] = (_c("vor_an_anteil_rv") + _c("vor_ag_anteil_rv")
                                                     + _c("vor_rv_ausserhalb_lstb")) // 100
            g["vorsorge_ag_anteil_steuerfrei"] = _c("vor_ag_anteil_rv") // 100
            # Person-B-Basisvorsorge (§ 10 Abs. 3, A.2): bei Zusammenveranlagung die vor_*_rv_partner ADDITIV in
            # dieselben Summen-Slots (catala_gesamt/_vorsorge_abzug deckelt EINMAL, kürzt den AG-Anteil). ⚠ RESIDUAL
            # (gemeldet): der Höchstbetrag (_vorsorge_hb) wird bei zusammen NICHT verdoppelt (§ 10 Abs. 3 S. 3) →
            # nur Hoch-RV-Paare mit kombinierten Beiträgen > ~27566 € leicht unter-abgezogen (over-tax-safe, selten).
            if g["veranlagung"] == "zusammen":
                g["vorsorge_gesamtbeitraege_inkl_ag"] += (_c("vor_an_anteil_rv_partner")
                    + _c("vor_ag_anteil_rv_partner") + _c("vor_rv_ausserhalb_lstb_partner")) // 100
                g["vorsorge_ag_anteil_steuerfrei"] += _c("vor_ag_anteil_rv_partner") // 100
            # Sonder-Abzüge (Weg ii, Faltung): §35a → steuerermaessigungen, §10b + §10-KiSt → sonderausgaben,
            # §33-agB → aussergewoehnliche_belastungen — ADDITIV auf JEDE Einkunfts-Kombi (§19+§21+§20 zusammen
            # MIT §35a/§10b/§33 in EINEM Bescheid). GdE (§2 Abs.3 = ns+vv+gewinn+sonstige − §24a − §24b = ALLE tarifl. Arten, VOR den Abzügen fest
            # §2 Abs.3-vor-Abs.4 → kein Zirkel) = Basis der §10b-20%-Deckelung + §33-zumutbar-Staffel (Korrektheit
            # vs. §19-only der Sonder-Scheiben). Absente Abzugs-Felder → 0 (fail-SAFE: über-, nie unterbesteuert).
            # rechnung_unbar=false nullt §35a Abs.2/3 (Minijob unberührt); fam_anzahl_kinder/splitting → zumutbar.
            # einkuenfte_gewinn (§§13-18) + einkuenfte_sonstige (§22) SIND jetzt in der §10b/§33-GdE (§33-K2-Fix:
            # §2 Abs.3 SdE = alle tarifl. Arten). NUR §32d-Kapital bewusst RAUS (§2 Abs.5b Abgeltung). Guard fängt K2.
            gde = runner.catala_gesamt_gde({"veranlagungszeitraum": vz, "veranlagung": g["veranlagung"],
                                            "einkuenfte_nichtselbststaendig": ns, "einkuenfte_vermietung": vv,
                                            "einkuenfte_gewinn": g["einkuenfte_gewinn"],
                                            "einkuenfte_sonstige": g.get("einkuenfte_sonstige", 0),
                                            "altersentlastungsbetrag": alt24a + alt24a_b,
                                            "entlastungsbetrag_alleinerziehende": ent24b})
            abs23_aus = f.get("hh_rechnung_unbar", {}).get("wert") is False
            g["steuerermaessigungen"] = runner.catala_p35a_haushaltsnahe({
                "minijob_aufwendungen": _c("hh_minijob_aufwendungen") // 100,
                "haushaltsnahe_dienstleistungen": 0 if abs23_aus else _c("hh_dienstleistungen") // 100,
                "handwerker_arbeitskosten": 0 if abs23_aus else _c("hh_handwerker_arbeitskosten") // 100})
            # § 35c EStG energetische Sanierungsmassnahmen + Energieberater-Sondersatz.
            # Zwei Teilregeln (Sanierung 7%/6%, Energieberater 50%) werden im Jahresdeckel
            # kombiniert (14k/12k). Accessor nimmt EUROS (Cent→EUR via //100).
            p35c_sanierung_rohbetrag = runner.catala_p35c_sanierung({
                "sanierungsaufwendungen": _c("p35c_sanierungsaufwendungen") // 100,
                "ist_uebernaechstes_foerderjahr": f.get("p35c_ist_uebernaechstes_foerderjahr", {}).get("wert") is True})
            p35c_energieberater_rohbetrag = runner.catala_p35c_energieberater({
                "energieberater_aufwendungen": _c("p35c_energieberater_aufwendungen") // 100})
            # Jahresdeckel-Kombination (P35cJahresdeckel aus Catala)
            p35c_gesamt_deckel = runner.catala_p35c_jahresdeckel({
                "sanierung_ermaessigung": p35c_sanierung_rohbetrag,
                "energieberater_ermaessigung": p35c_energieberater_rohbetrag,
                "ist_uebernaechstes_foerderjahr": f.get("p35c_ist_uebernaechstes_foerderjahr", {}).get("wert") is True})
            g["steuerermaessigungen"] += p35c_gesamt_deckel
            # sonderausgaben = § 10b Spenden + § 10 KiSt + § 10 Abs.1 Nr.3/3a KV/PV-Vorsorge +
            # § 10 Abs.1 Nr.5 Kinderbetreuung (§10-Stufe 2, additiv;
            # KV/PV hat EIGENEN Abs.4-Höchstbetrag 1900/2800 + Basis-Durchbruch, getrennt von der Abs.3-Basisvorsorge
            # die catala_gesamt intern via _vorsorge_abzug addiert). PLAIN Read-Keys (1:1 mit dev-2s Binding). Die 3
            # KV/PV-Felder sind Pflicht-Kegel → immer beantwortet (kein stiller Über/Unter-tax; mit_anspruch steuert HB).
            # Kinderbetreuung: pro-Kind-Deckel 4800€; anzahl_kinder Multiplikator; aufwendungen Summe/Person.
            g["sonderausgaben"] = (runner.catala_p10b_spenden({
                    "zuwendungen": _c("spenden_betrag") // 100, "gesamtbetrag_der_einkuenfte": gde})
                + runner.catala_p10_kist({
                    "gezahlte_kirchensteuer": _c("kist_gezahlt") // 100,
                    "erstattete_kirchensteuer": _c("kist_erstattet") // 100})
                + runner.catala_p10_kv_pv({
                    "basis_kv_pv": _c("basis_kv_pv") // 100,
                    "weitere_vorsorgeaufwendungen": _c("weitere_vorsorgeaufwendungen") // 100,
                    "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss", {}).get("wert") is True})
                + runner.catala_p10_1_5_kinderbetreuung({
                    "aufwendungen": _c("kinderbetreuungskosten") // 100,
                    "anzahl_kinder": f.get("kinderbetreuung_anzahl_kinder", {}).get("wert", 0) or 0})
                # § 10 Abs. 1a Nr. 1 Realsplitting (Unterhalt Ex-Ehegatte, Tier-1): min(unterhaltsleistungen,
                # 13.805 + kv_pv_beitraege). Gate: realsplitting_zustimmung==true → sonst 0 (over-tax-safe).
                + (runner.catala_p10_1a_realsplitting({
                    "unterhaltsleistungen": _c("realsplitting_unterhaltsleistungen") // 100,
                    "kv_pv_beitraege": _c("realsplitting_empfaenger_kv_pv") // 100})
                   if f.get("realsplitting_zustimmung", {}).get("wert") is True else 0)
                # Person-B-KV/PV (§ 10 Abs. 4, A.2): eigener Höchstbetrag JE PERSON → separater Accessor-Aufruf,
                # additiv (kein gemeinsamer Deckel, kein Doppelzählen — B liest die _partner-Read-Keys).
                + (runner.catala_p10_kv_pv({
                    "basis_kv_pv": _c("basis_kv_pv_partner") // 100,
                    "weitere_vorsorgeaufwendungen": _c("weitere_vorsorgeaufwendungen_partner") // 100,
                    "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss_partner", {}).get("wert") is True})
                   if g["veranlagung"] == "zusammen" else 0)
                # § 10 Abs. 1 Nr. 7 Aufwendungen eigene Berufsausbildung (Tier-1): min(aufwendungen, 6000), Person-A
                # (Satz 2 je-Person Ehegatten = Nachtrag wie A.2). Additiv wie § 10b/KV-PV/KiSt (eigener Höchstbetrag,
                # kein Doppelzählen — eigenes Feld berufsausbildung_aufwendungen). OPTIONAL: absent → 0 (over-tax-safe).
                + runner.catala_p10_1_7_berufsausbildung({
                    "berufsausbildung_aufwendungen": _c("berufsausbildung_aufwendungen") // 100}))
            # §33b Behinderten-/Pflege-/Hinterbliebenen-Pauschbetrag Person A (additiv zu §33-agB).
            # 1:1 gespiegelt vom rentner-Ring (api.py:1002-1010). Felder = rentner_-globale IDs
            # (Kz E0109708/etc. über rentner-Bindung, selbe Felder wie rentner-Scheibe). Absent→0 (safe).
            ausserg = (runner.catala_behinderten_pb({
                           "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung"),
                           "ist_hilflos_blind_taubblind": f.get("rentner_hilflos_blind_taubblind", {}).get("wert") is True})
                       + runner.catala_pflege_pb({
                           "veranlagungszeitraum": vz, "pflegegrad": _c("rentner_pflegegrad"),
                           "ist_hilflos": f.get("rentner_gepflegter_hilflos", {}).get("wert") is True})
                       + runner.catala_hinterbliebenen_pb({
                           "veranlagungszeitraum": vz,
                           "hat_hinterbliebenenbezuege": f.get("rentner_hinterbliebenenbezuege", {}).get("wert") is True}))
            # Person-B-§33b: eigener Behinderten-Pauschbetrag des Ehegatten additiv (1:1 Rentner-Präzedenz
            # api.py:1015-1018). Nur Zusammenveranlagung. Pflege-/Hinterbliebenen-PB für Person B nicht
            # modelliert (wie rentner). Felder = rentner_*-globale IDs.
            if g["veranlagung"] == "zusammen":
                ausserg += runner.catala_behinderten_pb({
                    "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung_partner"),
                    "ist_hilflos_blind_taubblind": f.get("rentner_hilflos_blind_taubblind_partner", {}).get("wert") is True})
            g["aussergewoehnliche_belastungen"] = ausserg + runner.catala_p33_agb({
                "aussergewoehnliche_belastungen": _c("agb_aufwendungen") // 100,
                "gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": _c("fam_anzahl_kinder"),
                "splitting": g["veranlagung"] == "zusammen"})
            # Kapital § 20/§ 32d: SINGLE-SOURCE (Instructor-Q1) — E1900701-Aggregat XOR Verlust-Töpfe;
            # Co-Okkurrenz sperrt der Guard (kapital_semantik_offen). Töpfe (§ 20 Abs. 6, per-Topf-Floor)
            # → verrechnete; sonst das Aggregat. Dann Sparer-PB (§ 20 Abs. 9). kapitaleinkuenfte ist UNABHÄNGIG
            # vom § 31-Kinderfreibetrag (§ 2 Abs. 5b/Abs. 6) → EINMAL vorab, vor der § 31-Verzweigung.
            if any(_c(t) != 0 for t in KAP_TOEPFE):
                verrechnete = runner.catala_kapital_verrechnung({
                    "gewinn_aktien": _c("kap_gewinn_aktien") // 100,
                    "verlust_aktien": _c("kap_verlust_aktien") // 100,
                    "gewinn_sonstige": _c("kap_gewinn_sonstige") // 100,
                    "verlust_sonstige": _c("kap_verlust_sonstige") // 100})
            else:
                verrechnete = _c(KAP_ERTRAEGE) // 100
            zusammen = (g["veranlagung"] == "zusammen"
                        or f.get("kap_zusammenveranlagung", {}).get("wert") is True)
            # Person B (§ 26b, #4b): das Kapital des Ehegatten single-source (Aggregat XOR Töpfe) ROH
            # addieren VOR dem gemeinsamen Sparer-PB (§ 20 Abs. 9 S. 3, ×2 über zusammenveranlagung). Nur
            # bei Zusammenveranlagung; Co-Okkurrenz B sperrt der Guard (kapital_semantik_offen).
            if g["veranlagung"] == "zusammen":
                if any(_c(t) != 0 for t in KAP_TOEPFE_PARTNER):
                    verrechnete += runner.catala_kapital_verrechnung({
                        "gewinn_aktien": _c("kap_gewinn_aktien_partner") // 100,
                        "verlust_aktien": _c("kap_verlust_aktien_partner") // 100,
                        "gewinn_sonstige": _c("kap_gewinn_sonstige_partner") // 100,  # Register-B-K2-Fix: symmetrisch statt hart 0
                        "verlust_sonstige": _c("kap_verlust_sonstige_partner") // 100})
                else:
                    verrechnete += _c(KAP_ERTRAEGE_PARTNER) // 100
            kapitaleinkuenfte = runner.catala_sparer_pb({
                "veranlagungszeitraum": vz, "kapitalertraege": verrechnete, "zusammenveranlagung": zusammen})

            # §23 Private Veräußerungsgeschäfte (Stufe-1): Σ über Instanzen → ADDITIV in einkuenfte_sonstige
            g["einkuenfte_sonstige"] = _p23_ansonsten_einkuenfte(f, store, bindung, nur_bestaetigt)

            # § 10d Abs. 2 Verlustvortrag (opt-in via verlustvortrag_bestand): der festgestellte verbleibende Verlust-
            # vortrag mindert den GdE „VORRANGIG vor Sonderausgaben, agB, sonstigen Abzugsbeträgen" (§ 10d Abs. 2 S. 1)
            # → Fold in sonstige_abzuege_vom_einkommen (§ 2 Abs. 5-Rest-Slot). Die zvE-Kette ist rein linear OHNE Floor
            # (einkommensteuertarif:494-514) → die Reihenfolge ist wertgleich zu „vorrangig vor". Der Höchstbetrag
            # (catala_p10d_2, min(GdE, Sockel+70%-Überstieg), gefixt sha 294cdd6a) braucht die VOLLE GdE (alle
            # tariflichen Einkunftsarten INKL. §§ 13-18-Gewinn; § 32d-Kapital § 2 Abs. 5b-exkl.) — jetzt IDENTISCH zur
            # §10b/§33-gde oben (§33-K2-Fix: beide voll ns+vv+gewinn+sonstige; der frühere Gap ist geschlossen). absent → 0
            # (over-tax-safe). Steht VOR dem § 35 (der § 35-Deckel-3 nutzt die post-§10d geminderte tarifliche ESt).
            gde_p10d = runner.catala_gesamt_gde({
                "veranlagungszeitraum": vz, "veranlagung": g["veranlagung"],
                "einkuenfte_nichtselbststaendig": ns, "einkuenfte_vermietung": vv,
                "einkuenfte_gewinn": g["einkuenfte_gewinn"],
                "einkuenfte_sonstige": g.get("einkuenfte_sonstige", 0),   # K2-Sweep-Konsistenz: §22-Loch-Vorsorge (heute ≡0 im gesamt, kein künftiger §10d-Under-tax)
                "altersentlastungsbetrag": alt24a + alt24a_b,
                "entlastungsbetrag_alleinerziehende": ent24b})
            # §33a Unterhalt + Ausbildungsfreibetrag: ADDITIV zu §10d (beide GdE-Minderung "vom GdE abgezogen")
            p33a_unt = runner.catala_p33a_unterhalt({
                "veranlagungszeitraum": vz,
                "aufwendungen": _c("p33a_unterhalt_aufwendungen") // 100,
                "kv_pv_beitraege": _c("p33a_unterhalt_kv_pv") // 100,
                "andere_einkuenfte_bezuege": _c("p33a_andere_einkuenfte_bezuege") // 100})
            p33a_ausb = runner.catala_p33a_ausbildungsfreibetrag({
                "anzahl_kinder": _c("p33a_ausbildung_anzahl_kinder")})
            g["sonstige_abzuege_vom_einkommen"] = runner.catala_p10d_2({
                "gesamtbetrag_einkuenfte": gde_p10d,
                "verlustvortrag_bestand": _c("verlustvortrag_bestand") // 100,
                "zusammenveranlagung": g["veranlagung"] == "zusammen"}) + p33a_unt + p33a_ausb
            # §34c Abs.1 DBA-Anrechnung (Stufe-1, single-country, fail-closed): anrechnung = min(gezahlt, HB).
            # HB = tarifliche_est * dba_auslaendische_einkuenfte / zvE. §34c VOR §35 (§35 Abs.1 S.4: geminderte
            # tarifliche Steuer = tarifliche NACH §34c). anrechnung in g → catala_gesamt-Slot, mindert
            # festzusetzende_est. dba_auslaendische_einkuenfte [cent] → EURO (wie alle Geld-Felder).
            # PRE-§34c tarifliche = catala_gesamt_tarifliche(g) — invariant gegen anrechnung (empirisch bewiesen).
            # Fail-closed: Freistellung, multi-country, §32d-Kapital → Guard sperrt (kein silent-zero).
            dba_anrechnung = 0
            dba_gezahlt = _c("dba_gezahlte_auslaendische_steuer") // 100
            dba_ausl = _c("dba_auslaendische_einkuenfte") // 100
            if dba_gezahlt > 0 or dba_ausl > 0:
                dba_anrechnung = runner.catala_p34c_1({
                    "gezahlte_auslaendische_steuer": dba_gezahlt,
                    "deutsche_est_inkl_ausl": runner.catala_gesamt_tarifliche(g),
                    "zu_versteuerndes_einkommen": runner.catala_gesamt_zve(g),
                    "auslaendische_einkuenfte_staat": dba_ausl})
            g["anzurechnende_auslaendische_steuern"] = dba_anrechnung
            # § 35 GewSt-Anrechnung (S1, opt-in via gewst_messbetrag): der GewSt-Steuermessbetrag (INPUT aus dem
            # GewSt-Messbescheid, enthält § 8-Hinzurechnung/§ 9-Kürzung schon) + Hebesatz → Anrechnung auf die
            # tarifliche ESt. Zähler des Ermäßigungshöchstbetrags (§ 35 Abs. 1 S. 2) = gewerbliche Einkünfte (S. 3
            # „der Gewerbesteuer unterliegenden Gewinne") = laufender § 15-Gewerbe-Gewinn — NUR betriebsart=gewerbe
            # (§ 18-selbständig/§ 13-LuF nicht gewerbesteuerpflichtig), § 16-vg-netto RAUS (§ 7 S. 2 GewStG: Ver-
            # äußerungsgewinn natürl. Person nicht im Gewerbeertrag). Nenner = Σ positive tarifliche Einkünfte
            # (Kapital = § 32d-Abgeltung, § 2 Abs. 5b-separat, NICHT im tariflichen zvE → exkl.). Kein gewst_messbetrag
            # → kein § 35 (opt-out, over-tax-safe). Der gewst_hebesatz_offen-Guard sperrt Messbetrag-ohne-Hebesatz.
            p35_messbetrag = _c("gewst_messbetrag") // 100
            p35_hebesatz = _c("gewst_hebesatz")
            p35_zaehler = max(0, laufender_gewinn) if f.get("gewinn_betriebsart", {}).get("wert") == "gewerbe" else max(0, mitu)
            # Nenner (§ 35 Abs. 1 S. 2 „Summe aller positiven Einkünfte") = Σ positive TARIFLICHE Einkunftsarten:
            # § 19 (ns) + § 21 (vv) + § 22 (sonstige) + §§ 13-18 (gewinn, inkl. § 16-vg = § 2-Einkunft). Das
            # § 32d-Abgeltung-Kapital ist NICHT einzubeziehen (§ 2 Abs. 5b EStG: „Kapitalerträge nach § 32d Absatz 1
            # und § 43 Absatz 5 nicht einzubeziehen" — es ist nicht im tariflichen zvE, das tarifliche_est skaliert).
            # einkuenfte_sonstige im gesamt-Ring = §23 (§22Nr.2) + künftig eventuell §22Nr.5 — §22Nr.1
            # (Leibrente) lebt in der rentner-Scheibe. §23-Nenner-Integration: §23-Einkünfte sind
            # §2-tariflich, gehören also in den §35-Nenner (alle positiven tariflichen Einkünfte) —
            # der Term dokumentiert die korrekte Formel + ist robust, falls § 22 je in den gesamt-Ring kommt.
            p35_nenner = (max(0, ns) + max(0, vv) + max(0, g.get("einkuenfte_sonstige", 0))
                          + max(0, g["einkuenfte_gewinn"]))

            # §3 Abs.2 SolzG: SolZ-Basis = KiFB-fiktive ESt (immer mit §32 Abs.6-Freibetraegen,
            # unabhaengig vom §31-Ergebnis) minus §32d-Kapitalsteuer. solz_info wird von
            # _festzusetzende je Lauf befuellt; der letzte Lauf (KiFB>0) ueberschreibt.
            solz_info = {}

            # §32b Progressionsvorbehalt (Stufe-1, Lohnersatz, Post-Engine-Wrapper)
            pe_raw = _c("p32b_progressionseinkuenfte") // 100
            pe_active = pe_raw > 0
            p35_active = p35_messbetrag > 0 and p35_zaehler > 0 and p35_nenner > 0
            # §32b×§34-Koinzidenz-Guard: Post-Engine §32b NACH §34 (tarif_modifiziert).
            # Bewegt: Guard in _an_gesamt_sperrgrund sperrt p32b_p34_kombi_offen bei Co-Präsenz.

            def _festzusetzende(freibetrag: int) -> int:
                # Der volle festzusetzende ESt-Bescheid (§ 19+§21+alle Abzüge, PLUS § 20-Kapital-Günstiger § 32d
                # Abs. 6) bei GEGEBENEM § 32-Abs.6-Kinderfreibetrag. Kapital-Günstiger: est_ohne_kap vs est_mit_kap
                # (Grundtarif) → min(Abgeltung, Delta). freibetrag=0 → kein Kinderfreibetrag.
                g2 = g if freibetrag == 0 else dict(g, freibetraege_kinder=freibetrag)
                # § 34 CHOOSER (XOR — nie beide auf denselben vg): Abs. 1 Fünftel (Default, von Amts wegen) vs Abs. 3
                # ermäßigter Durchschnittssatz (AUF ANTRAG, 55+/dauernd-berufsunfähig, einmal im Leben, ao ≤ 5 Mio). Der
                # § 16-vg (netto_vg, außerordentlich § 34 Abs. 2 Nr. 1) wird geglättet statt voll progressiv. Engine-vor-
                # verdrahtet: tarif_modifiziert setzt tarifliche_est = tarifliche_est_modifiziert (einkommensteuertarif
                # Z.483/518). PER §31-Zweig (zve2 je Zweig — Kinderfreibetrag senkt zvE → eigener Tarif). Guard zve2>0.
                # ao = netto_vg NUR (laufender §15/§18-Gewinn progressiv). §35-Deckel-3 liest die post-§34-tarifliche unten.
                if netto_vg > 0:
                    zve2 = runner.catala_gesamt_zve(g2)
                    if zve2 > 0:
                        if f.get("antrag_ermaessigter_satz", {}).get("wert") is True \
                                and _abs3_eligible(f, vz) and netto_vg <= 5_000_000:
                            # § 34 Abs. 3: est = plain grundtarif(verbleibendes zvE = zvE−ao, S.3 „allgemeine Tarif-
                            # vorschriften") + ermäßigter_satz × min(ao,5Mio). est_gesamt = grundtarif(VOLLES zvE) OHNE
                            # §32b-Progressionszuschlag (nicht im Ring). catala_est nur-zvE → plain §32a, KEIN Fünftel.
                            est_rest = runner.catala_est({"veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                                                          "zu_versteuerndes_einkommen": max(0, zve2 - netto_vg)})
                            est_ao = runner.catala_ermaessigter_durchschnittssatz({
                                "ao_einkuenfte": netto_vg,
                                "est_gesamt_zzgl_progression": runner.catala_gesamt_tarifliche(g2),
                                "bemessungsgrundlage_durchschnitt": zve2})
                            g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=est_rest + est_ao)
                        else:
                            # § 34 Abs. 1 Fünftel (Default ODER Abs.3-Eligibility-fail-closed — nie Abs.3 erzwingen):
                            # 5×[Tarif(zvE_rest+ao/5)−Tarif(zvE_rest)], S.3-Negativ via catala_fuenftel.
                            g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=runner.catala_fuenftel({
                                "veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                                "zu_versteuerndes_einkommen": zve2, "ausserordentliche_einkuenfte": netto_vg}))
                # § 35 Abs. 1: min(4×Messbetrag [S. 1 „das Vierfache"], Messbetrag×Hebesatz [S. 5 „tatsächlich zu
                # zahlende Gewerbesteuer"], Ermäßigungshöchstbetrag [S. 2: Zähler/Nenner × geminderte tarifliche
                # Steuer]). ADDITIV in steuerermaessigungen DIESES Freibetrag-Zweigs — tarifliche_est ist freibetrag-
                # abhängig (Kinderfreibetrag senkt zvE), daher Deckel-3 JE ZWEIG (global-einmal würde den Kinder-
                # freibetrag-Zweig über-crediten = stille Under-tax). §35 Abs.1 S.4: geminderte tarifliche Steuer
                # = tarifliche_est NACH §34c-Anrechnung (PRE-§35, NEVER umgekehrt — sonst §35-Über-Kredit).
                # §32b Progressionsvorbehalt: §35-Deckel-3 braucht POST-§32b tarifliche wenn pe_active.
                # Stufe-1: §35 NORMAL in g2 wenn ¬pe_active; POST-WRAPPER-APPLY wenn pe_active.
                p35_credit = 0
                if p35_messbetrag > 0 and p35_zaehler > 0 and p35_nenner > 0:
                    tarifliche_raw = runner.catala_gesamt_tarifliche(g2)
                    tarifliche_gemindert = max(0, tarifliche_raw - dba_anrechnung)
                    p35_credit = min(4 * p35_messbetrag,
                                     p35_messbetrag * p35_hebesatz // 100,
                                     p35_zaehler * tarifliche_gemindert // p35_nenner)
                    if not pe_active:
                        g2 = dict(g2, steuerermaessigungen=g2.get("steuerermaessigungen", 0) + p35_credit)
                est_raw = runner.catala_est(g2)     # KEIN Kapital (est_regulaer_ohne_kap)
                # §32b Post-Engine-Wrapper (NACH catala_est, VOR §35-Apply-if-pe_active)
                if pe_active:
                    tarifliche_pre32b = runner.catala_gesamt_tarifliche(g2)
                    est_without_tarifliche = est_raw - tarifliche_pre32b
                    zve32b = runner.catala_gesamt_zve(g2)
                    t_32b = 0
                    if zve32b > 0:
                        est_erhoeht = runner.catala_est({"veranlagungszeitraum": vz,
                                                          "veranlagung": g2.get("veranlagung", "einzel"),
                                                          "zu_versteuerndes_einkommen": zve32b + pe_raw})
                        t_32b = runner.catala_p32b_1({
                            "zu_versteuerndes_einkommen": zve32b,
                            "progressionseinkuenfte": pe_raw,
                            "est_auf_erhoehte_bemessung": est_erhoeht})
                        est_raw = t_32b + est_without_tarifliche
                    # §35-Deckel-3 apply post-wrapper mit t_32b (§35 Abs.1 S.4 geminderte tarifliche = Post-§32b)
                    if p35_messbetrag > 0 and p35_zaehler > 0 and p35_nenner > 0:
                        deckel3_32b = p35_zaehler * max(0, t_32b - dba_anrechnung) // p35_nenner
                        p35_credit_pe = min(4 * p35_messbetrag, p35_messbetrag * p35_hebesatz // 100, deckel3_32b)
                        est_raw = max(0, est_raw - p35_credit_pe)
                # End §32b wrapper
                if kapitaleinkuenfte <= 0:
                    if freibetrag > 0 or kinder == 0:
                        solz_info["est_mit_fb"] = est_raw
                        solz_info["kap_st"] = 0
                    return est_raw
                est_mit = runner.catala_est(dict(g2, einkuenfte_kapitalvermoegen=kapitaleinkuenfte))
                kap_st = runner.catala_kapital_steuer({
                    "veranlagungszeitraum": vz, "kapitaleinkuenfte": kapitaleinkuenfte,
                    "est_regulaer_mit_kap": est_mit, "est_regulaer_ohne_kap": est_raw})
                result = est_raw + kap_st
                if freibetrag > 0 or kinder == 0:
                    solz_info["est_mit_fb"] = result
                    solz_info["kap_st"] = kap_st
                return result

            # § 31 Familienleistungsausgleich (Günstigerprüfung Kindergeld vs Kinderfreibetrag § 32 Abs. 6): bei
            # Kindern den vollen Bescheid EINMAL OHNE + einmal MIT Kinderfreibetrag rechnen; FL wählt das für den
            # Steuerpflichtigen Günstigere (Kindergeld-besser → est_ohne_fb, Kindergeld bleibt; Freibetrag-besser
            # → est_mit_fb + Kindergeld-Hinzurechnung § 31 S. 4). Der Kinderfreibetrag mindert das zvE (§ 2 Abs. 5),
            # NICHT die GdE (§ 2 Abs. 3) → die §10b/§33-Deckel (auf gde) bleiben unberührt. Ohne Kinder kein § 31.
            kinder = _c("fam_anzahl_kinder")
            if kinder > 0:
                est = runner.catala_p31_familienleistung({
                    "est_ohne_freibetraege": _festzusetzende(0),
                    "est_mit_freibetraegen": _festzusetzende(
                        kinder * runner._kinderfreibetrag(vz, g["veranlagung"])),
                    "kindergeld": kinder * runner._kindergeld(vz) * 12})
            else:
                est = _festzusetzende(0)
            # SolZ §3, §4 SolzG: Basis = KiFB-fiktive ESt (§3 Abs.2) minus §32d-Kapitalsteuer (§3 Abs.3 S.1);
            # §32d-Kapital-SolZ 5,5% ohne Freigrenze (§3 Abs.3 S.2) wird von catala_solz separat addiert.
            if solz_container is not None and "est_mit_fb" in solz_info:
                solz_container[0] = runner.catala_solz({
                    "veranlagungszeitraum": vz,
                    "bemessungsgrundlage": solz_info["est_mit_fb"],
                    "kapital_steuer": solz_info.get("kap_st", 0),
                    "splitting": g["veranlagung"] == "zusammen"})
            # KiSt § 51a: Basis = Maßstabsteuer ohne §32d-Abgeltung-Kapital (= SolZ-basis_main;
            # die Abgeltung-KiSt e/(4+k) ist ein eigener Nachtrag → hier NICHT auf kap_st).
            if extras is not None and "est_mit_fb" in solz_info:
                extras["kist_cent"] = runner.catala_kist({
                    "est_mit_fb": solz_info["est_mit_fb"] - solz_info.get("kap_st", 0),
                    "konfession": f.get("kist_konfession", {}).get("wert", "keine"),
                    "bundesland": f.get("kist_bundesland", {}).get("wert", "")})
            return est
        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")

    if quantitaet == "festzusetzende_est_rentner":   # § 22 Renten + § 33b via catala_gesamt
        try:
            import runner  # noqa: F401
        except Exception:
            return None
        f = felder or {}

        def _c(fid):
            v = f.get(fid, {}).get("wert")
            return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0

        def _b(fid):
            return f.get(fid, {}).get("wert")

        def _rente_instanz(fi: dict) -> int:
            # § 22 Renten-Einkünfte EINER Rente (aa/bb je Instanz-Art) → einkuenfte_sonstige-Summand. Naht-CENT
            # → EURO (jahresrente/rentenfreibetrag); renten_beginn_jahr/alter sind int (kein cent). fi = das je-
            # Instanz auf die Basis-feld_id normierte Feld-Dict (instanzen-Naht) ODER die felder-closure f (Basis).
            def _ci(k):
                v = fi.get(k, {}).get("wert")
                return int(v) if isinstance(v, (int, float)) and not isinstance(v, bool) else 0
            rf = fi.get("rentner_rentenfreibetrag", {}).get("wert")
            return runner.catala_renten_einkuenfte({
                "veranlagungszeitraum": vz,
                "renten_art": fi.get("rentner_renten_art", {}).get("wert"),
                "jahresrente": _ci("rentner_jahresrente") // 100,
                "renten_beginn_jahr": _ci("rentner_renten_beginn_jahr"),
                "alter_bei_rentenbeginn": _ci("rentner_alter_bei_rentenbeginn"),
                "rentenfreibetrag": (rf // 100 if isinstance(rf, (int, float))
                                     and not isinstance(rf, bool) else None)})

        def slot_fn(slots: dict) -> int:
            # § 22 Renten-Einkünfte → einkuenfte_sonstige, als STUMPFE Σ über ALLE rente-Instanzen der Person A
            # (Multi-Rente, #6: gesetzl. + Betriebs- + Leibrente je eigene aa/bb-Behandlung, § 22-Anteil JE
            # RENTE, dann summiert — DIESELBE instanzen-Naht wie Multi-Objekt-§21). index==1 = Basis-rentner_*-
            # Felder, __n = weitere Renten. Ohne store (Alt-Aufrufer) nur die Basis. Die aa-Folgejahr-ohne-RF-
            # Sperre fängt der Guard je Instanz VORHER (rentenfreibetrag_fixierung_offen); hier kommt nur der
            # rechenbare Fall an. Unvollständige Renten-Instanz sperrt der Guard (rente_instanz_offen).
            if store is not None:
                # Zwei-Signal-Filter (Instanz-Pfad, nur_bestaetigt=True): vorläufige Renten-Instanz nie in die Σ.
                # Defense-in-depth — rente_instanz_offen-Guard sperrt index≥2-vorläufig schon vorher, Basis im Kegel-
                # Meet — konsistent zum gwg/vv-Σ. nur_bestaetigt=False (/stand) zeigt die vorläufige Wirkung im Range.
                renten = sum(_rente_instanz(inst["felder"])
                             for inst in EM.instanzen(store, bindung, "rente")
                             if not nur_bestaetigt or inst["zustand"] == "bestaetigt")
            else:
                renten = _rente_instanz(f)
            # Person B (§ 26b, #4b): die § 22-Rente des Ehegatten in DIESELBE einkuenfte_sonstige-Summe
            # (§9a-/Ertragsanteil JE PERSON, dann summiert). Die aa-Folgejahr-ohne-RF-Sperre für B fängt der
            # Guard vorab. Nur bei Zusammenveranlagung; Person-B ohne Rente → jahresrente 0 → renten 0.
            if (_b("veranlagung") == "zusammen") and _b("rentner_renten_art_partner"):
                rfp = _b("rentner_rentenfreibetrag_partner")
                renten += runner.catala_renten_einkuenfte({
                    "veranlagungszeitraum": vz,
                    "renten_art": _b("rentner_renten_art_partner"),
                    "jahresrente": _c("rentner_jahresrente_partner") // 100,
                    "renten_beginn_jahr": _c("rentner_renten_beginn_jahr_partner"),
                    "alter_bei_rentenbeginn": _c("rentner_alter_bei_rentenbeginn_partner"),
                    "rentenfreibetrag": (rfp // 100 if isinstance(rfp, (int, float))
                                         and not isinstance(rfp, bool) else None)})
            # § 33b Behinderten- + Pflege- + Hinterbliebenen-Pauschbetrag (additiv) → aussergewoehnliche_belastungen.
            ausserg = (runner.catala_behinderten_pb({
                           "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung"),
                           "ist_hilflos_blind_taubblind": _b("rentner_hilflos_blind_taubblind") is True})
                       + runner.catala_pflege_pb({
                           "veranlagungszeitraum": vz, "pflegegrad": _c("rentner_pflegegrad"),
                           "ist_hilflos": _b("rentner_gepflegter_hilflos") is True})
                       + runner.catala_hinterbliebenen_pb({
                           "veranlagungszeitraum": vz,
                           "hat_hinterbliebenenbezuege": _b("rentner_hinterbliebenenbezuege") is True}))
            # Partner-§33b (§ 26b, #4b, Wiring-Fix): eigener Behinderten-Pauschbetrag des Ehegatten additiv zur
            # gemeinsamen ausserg-Summe — nur Zusammenveranlagung (RENTNER_PARTNER hat nur GdB/hilflos, kein
            # Pflegegrad/Hinterbliebenenbezüge für Person B). Vorher: Felder standen nur im Gate-Tuple, nie
            # tatsächlich addiert → stiller Über-tax (250 € je betroffenem Fall).
            if _b("veranlagung") == "zusammen":
                ausserg += runner.catala_behinderten_pb({
                    "veranlagungszeitraum": vz, "grad_der_behinderung": _c("rentner_grad_der_behinderung_partner"),
                    "ist_hilflos_blind_taubblind": _b("rentner_hilflos_blind_taubblind_partner") is True})
            # §23 Private Veräußerungsgeschäfte (Stufe-1): Σ über Instanzen → ADDITIV zu renten
            p23_eink = _p23_ansonsten_einkuenfte(f, store, bindung, nur_bestaetigt)
            renten += p23_eink
            # §§ 13-18 Gewinn (2-I + 2a): laufender § 15/§ 18-Gewinn (aus _laufender_gewinn — Stufe 2a EÜR ODER
            # Stufe-1-Direktwert, Scope A geteilt mit dem gesamt-Ring) + § 16-Ver-
            # äußerungsgewinn NACH § 16 Abs. 4-Freibetrag. FB (roh) via catala_p16_4_freibetrag; der steuerbare Rest
            # ist bei 0 GEFLOORT (§ 16 Abs. 4 „soweit nicht übersteigt" — FB > vg erzeugt KEINEN Verlust, sonst
            # stiller Under-tax gegen die Rente). ADDITIV (§ 16 Abs. 1: Veräußerungs- + laufender Gewinn = dieselbe
            # § 2-Einkunftsart Gewerbebetrieb → EIN einkuenfte_gewinn-Summand). Absent → 0 (over-tax-safe). CENT→EURO.
            # Naht-CENT → EURO: der Accessor nimmt EUROS (wie catala_p10_1_7_berufsausbildung — die //100-
            # Umrechnung liegt im slot_fn, nicht im Accessor). vg_euro EINMAL, an Freibetrag + Subtraktion.
            vg_euro = _c("rentner_veraeusserungsgewinn") // 100
            netto_vg = max(0, vg_euro - runner.catala_p16_4_freibetrag(
                {"rentner_veraeusserungsgewinn": vg_euro}))
            laufender_gewinn, mitu = _laufender_gewinn(f, store, bindung, nur_bestaetigt)   # § 15/§ 18 laufend (§ 35-Zähler, OHNE § 16-vg)
            # § 24a Altersentlastungsbetrag im Rentner-Ring (b): Bemessung = positive Nicht-§19-Einkünfte = §§13-18-Gewinn
            # (laufender + § 16-vg-netto); LEIBRENTE § 22 Nr. 1 (renten) + Versorgungsbezüge § 19 Abs. 2 sind KEINE Bemessung
            # (§ 24a S. 2-Ausschlüsse). §23 private Veräußerungsgeschäfte (§22 Nr.2) gehören IN die
            # §24a-Bemessung (S.2 erwähnt nur §19Abs.2/§22Nr.1/§22Nr.4/5 — kein §22Nr.2-Ausschluss).
            # p23_eink additiv zu laufender_gewinn + netto_vg (K2-konservativ: floor auf kombinierte Summe).
            # Kein § 19-Mini-Job-Arbeitslohn im rentner-Ring (MVP-Lücke, over-tax-safe → 0). MIT
            # dem 64+-Gate (§ 24a S. 3, geerbt vom Accessor via veranlagungszeitraum). Pure Leibrente (kein Gewinn) → 0.
            alt24a_r = runner.catala_p24a_altersentlastung({
                "veranlagungszeitraum": vz, "geburtsjahr": _c("geburtsjahr"), "arbeitslohn": 0,
                "positive_andere_einkuenfte": max(0, laufender_gewinn + netto_vg + p23_eink)})
            # § 24b Entlastungsbetrag Alleinerziehende (Weg-ii-Parität-Fix, K2, Over-tax): fehlte im Rentner-Ring
            # komplett (GESAMT_FREIBETRAEGE nie an RENTNER_FELDER, s.o.) — Rentner-Witwe/-Witwer mit Kindern kriegt
            # sonst den 4260€+240€/Kind-Freibetrag nicht. 1:1 gesamt-Präzedenz Z. 671-674 (ungegatet, fam_anzahl_
            # kinder-Kinder-Gate ist Fund D/separater Scope). alleinerziehend_mit_zusammen-Guard (Z. 1110) fängt
            # den Widerspruch automatisch, sobald fam_alleinstehend gebunden ist — kein Zusatzcode nötig.
            ent24b_r = runner.catala_p24b_entlastung({
                "alleinstehend": f.get("fam_alleinstehend", {}).get("wert") is True,
                "anzahl_kinder": _c("fam_anzahl_kinder"),
                "monate_ohne_voraussetzung": _c("fam_monate_ohne_voraussetzung")})
            rentner_g = {
                "gesamtfall": True, "veranlagungszeitraum": vz,
                "veranlagung": _b("veranlagung") or "einzel",
                "einkuenfte_sonstige": renten,
                "einkuenfte_gewinn": laufender_gewinn + netto_vg,
                "altersentlastungsbetrag": alt24a_r,
                "entlastungsbetrag_alleinerziehende": ent24b_r,
                "aussergewoehnliche_belastungen": ausserg}
            # § 10d Abs. 2 Verlustvortrag (Rentner-Ring): mindert die volle rentner-GdE (renten § 22 + einkuenfte_
            # gewinn) „vorrangig vor …" (§ 10d Abs. 2 S. 1) → sonstige_abzuege_vom_einkommen, VOR dem § 35 (dessen
            # Deckel-3 die post-§10d geminderte tarifliche ESt nutzt). catala_p10d_2-Höchstbetrag = min(GdE, …), absent
            # → 0. Der GdE-Zwilling nimmt die rentner-Einkunftsarten (kein § 19/§ 21 hier), § 32d-Kapital gibt's nicht.
            # DIESELBE gde speist auch die Weg-ii-Abzüge unten (§10b-20%-Deckel + §33-zumutbar-Staffel) — EIN Aufruf.
            gde = runner.catala_gesamt_gde({
                "veranlagungszeitraum": vz, "veranlagung": rentner_g["veranlagung"],
                "einkuenfte_sonstige": renten, "einkuenfte_gewinn": rentner_g["einkuenfte_gewinn"],
                "altersentlastungsbetrag": alt24a_r, "entlastungsbetrag_alleinerziehende": ent24b_r})
            # §33a Unterhalt + Ausbildungsfreibetrag: ADDITIV zu §10d (beide GdE-Minderung)
            p33a_unt = runner.catala_p33a_unterhalt({
                "veranlagungszeitraum": vz,
                "aufwendungen": _c("p33a_unterhalt_aufwendungen") // 100,
                "kv_pv_beitraege": _c("p33a_unterhalt_kv_pv") // 100,
                "andere_einkuenfte_bezuege": _c("p33a_andere_einkuenfte_bezuege") // 100})
            p33a_ausb = runner.catala_p33a_ausbildungsfreibetrag({
                "anzahl_kinder": _c("p33a_ausbildung_anzahl_kinder")})
            rentner_g["sonstige_abzuege_vom_einkommen"] = runner.catala_p10d_2({
                "gesamtbetrag_einkuenfte": gde,
                "verlustvortrag_bestand": _c("verlustvortrag_bestand") // 100,
                "zusammenveranlagung": rentner_g["veranlagung"] == "zusammen"}) + p33a_unt + p33a_ausb
            # §34c Abs.1 DBA-Anrechnung (1:1 gesamt-Präzedenz, §34c VOR §35).
            dba_anrechnung = 0
            dba_gezahlt = _c("dba_gezahlte_auslaendische_steuer") // 100
            dba_ausl = _c("dba_auslaendische_einkuenfte") // 100
            if dba_gezahlt > 0 or dba_ausl > 0:
                dba_anrechnung = runner.catala_p34c_1({
                    "gezahlte_auslaendische_steuer": dba_gezahlt,
                    "deutsche_est_inkl_ausl": runner.catala_gesamt_tarifliche(rentner_g),
                    "zu_versteuerndes_einkommen": runner.catala_gesamt_zve(rentner_g),
                    "auslaendische_einkuenfte_staat": dba_ausl})
            rentner_g["anzurechnende_auslaendische_steuern"] = dba_anrechnung
            # Weg-ii-Fix (K2, Over-tax): § 10 Abs. 1 Nr. 2 Basisvorsorge (VOR_FELDER jetzt Pflicht-Kegel) —
            # catala_est ruft _vorsorge_abzug intern (runner.py), kein Doppelzählen, nur die Slots setzen.
            rentner_g["vorsorge_gesamtbeitraege_inkl_ag"] = (_c("vor_an_anteil_rv") + _c("vor_ag_anteil_rv")
                                                              + _c("vor_rv_ausserhalb_lstb")) // 100
            rentner_g["vorsorge_ag_anteil_steuerfrei"] = _c("vor_ag_anteil_rv") // 100
            # § 35a Haushaltsnahe (Weg-ii-Fix) → steuerermaessigungen. rechnung_unbar=false nullt Abs.2/3 (Minijob
            # unberührt) — 1:1 gesamt-Präzedenz (Z. 708-712).
            abs23_aus = f.get("hh_rechnung_unbar", {}).get("wert") is False
            rentner_g["steuerermaessigungen"] = runner.catala_p35a_haushaltsnahe({
                "minijob_aufwendungen": _c("hh_minijob_aufwendungen") // 100,
                "haushaltsnahe_dienstleistungen": 0 if abs23_aus else _c("hh_dienstleistungen") // 100,
                "handwerker_arbeitskosten": 0 if abs23_aus else _c("hh_handwerker_arbeitskosten") // 100})
            # § 35c EStG energetische Sanierungsmassnahmen + Energieberater (1:1 gesamt-Präzedenz).
            p35c_sanierung_r = runner.catala_p35c_sanierung({
                "sanierungsaufwendungen": _c("p35c_sanierungsaufwendungen") // 100,
                "ist_uebernaechstes_foerderjahr": f.get("p35c_ist_uebernaechstes_foerderjahr", {}).get("wert") is True})
            p35c_energieberater_r = runner.catala_p35c_energieberater({
                "energieberater_aufwendungen": _c("p35c_energieberater_aufwendungen") // 100})
            p35c_gesamt_deckel_r = runner.catala_p35c_jahresdeckel({
                "sanierung_ermaessigung": p35c_sanierung_r,
                "energieberater_ermaessigung": p35c_energieberater_r,
                "ist_uebernaechstes_foerderjahr": f.get("p35c_ist_uebernaechstes_foerderjahr", {}).get("wert") is True})
            rentner_g["steuerermaessigungen"] += p35c_gesamt_deckel_r
            # § 10b Spenden (gde-Deckel) + § 10 KiSt + § 10 Abs. 1 Nr. 3/3a KV/PV + § 10 Abs. 1 Nr. 5 Kinderbetreuung
            # + § 10 Abs. 1 Nr. 7 Berufsausbildung
            # (Weg-ii-Fix, additiv → sonderausgaben; 1:1 gesamt-Präzedenz Z. 717-737). Person-B-KV/PV DEFER (benannte
            # Lücke, Folge-Ticket — Rentner-Ring hat noch keine Partner-KV/PV-Felder).
            rentner_g["sonderausgaben"] = (runner.catala_p10b_spenden({
                    "zuwendungen": _c("spenden_betrag") // 100, "gesamtbetrag_der_einkuenfte": gde})
                + runner.catala_p10_kist({
                    "gezahlte_kirchensteuer": _c("kist_gezahlt") // 100,
                    "erstattete_kirchensteuer": _c("kist_erstattet") // 100})
                + runner.catala_p10_kv_pv({
                    "basis_kv_pv": _c("basis_kv_pv") // 100,
                    "weitere_vorsorgeaufwendungen": _c("weitere_vorsorgeaufwendungen") // 100,
                    "mit_anspruch_auf_zuschuss": f.get("mit_anspruch_auf_zuschuss", {}).get("wert") is True})
                + runner.catala_p10_1_5_kinderbetreuung({
                    "aufwendungen": _c("kinderbetreuungskosten") // 100,
                    "anzahl_kinder": f.get("kinderbetreuung_anzahl_kinder", {}).get("wert", 0) or 0})
                # § 10 Abs. 1a Nr. 1 Realsplitting (Unterhalt Ex-Ehegatte, Tier-1): 1:1 gesamt-Präzedenz.
                # Gate: realsplitting_zustimmung==true → sonst 0 (over-tax-safe).
                + (runner.catala_p10_1a_realsplitting({
                    "unterhaltsleistungen": _c("realsplitting_unterhaltsleistungen") // 100,
                    "kv_pv_beitraege": _c("realsplitting_empfaenger_kv_pv") // 100})
                   if f.get("realsplitting_zustimmung", {}).get("wert") is True else 0)
                + runner.catala_p10_1_7_berufsausbildung({
                    "berufsausbildung_aufwendungen": _c("berufsausbildung_aufwendungen") // 100}))
            # § 33-agB (Weg-ii-Fix) ADDITIV zu § 33b (ausserg, oben) — beide Absätze koexistieren (Pauschbetrag +
            # Einzelnachweis sind unterschiedliche Aufwands-Arten). gde-Basis wie § 10b (§2 Abs.3-K2-Fix).
            rentner_g["aussergewoehnliche_belastungen"] = ausserg + runner.catala_p33_agb({
                "aussergewoehnliche_belastungen": _c("agb_aufwendungen") // 100,
                "gesamtbetrag_der_einkuenfte": gde, "anzahl_kinder": _c("fam_anzahl_kinder"),
                "splitting": rentner_g["veranlagung"] == "zusammen"})
            # § 35 GewSt-Anrechnung Basiswerte (freibetrag-unabhängig — Zähler/Nenner hängen nicht vom § 31-Zweig
            # ab, nur die tarifliche_est im Deckel-3 unten). Zähler = laufender Gewerbe-Gewinn (NUR betriebsart=
            # gewerbe, § 16-vg-netto RAUS § 7 S. 2 GewStG). Nenner = renten (§ 22 IM Nenner — echt hier, anders als
            # gesamt wo sonstige=0) + einkuenfte_gewinn (VOLLSTÄNDIG: die rentner-Scheibe hat kein § 19/§ 21). Opt-
            # in via gewst_messbetrag; gewst_hebesatz_offen-Guard (shared) fängt Messbetrag-ohne-Hebesatz.
            p35_messbetrag = _c("gewst_messbetrag") // 100
            p35_hebesatz = _c("gewst_hebesatz")
            p35_zaehler = max(0, laufender_gewinn) if _b("gewinn_betriebsart") == "gewerbe" else max(0, mitu)
            p35_nenner = max(0, renten) + max(0, rentner_g["einkuenfte_gewinn"])

            # §3 Abs.2 SolzG: SolZ-Basis = KiFB-fiktive ESt (immer mit §32 Abs.6-Freibetraegen;
            # kein §32d-Kapital im Rentner-Ring → kap_st=0 immer). solz_info_r ueberschrieben vom KiFB-Lauf.
            solz_info_r = {}

            # §32b Progressionsvorbehalt (Rentner-Ring, 1:1 gesamt-Präzedenz)
            pe_raw = _c("p32b_progressionseinkuenfte") // 100
            pe_active = pe_raw > 0

            def _festzusetzende_r(freibetrag: int) -> int:
                # § 31 Familienleistungsausgleich (Fund D, Rentner-Ring-Fix): PER §31-Zweig neu gerechnet (g2 statt
                # rentner_g direkt) — Kinderfreibetrag senkt zvE (§ 2 Abs. 5) → eigener Tarif + eigene § 35-Deckel-3-
                # Anrechnung je Zweig. 1:1 gesamt-Naht-Präzedenz (_festzusetzende Z. 842-892).
                g2 = dict(rentner_g, freibetraege_kinder=freibetrag) if freibetrag else dict(rentner_g)
                # § 34 CHOOSER im Rentner-Ring (Abs. 1 Fünftel Default vs Abs. 3 ermäßigter Satz auf Antrag):
                # identisch zur gesamt-Naht. Guard zve2>0. ao = netto_vg (laufender Gewinn progressiv).
                if netto_vg > 0:
                    zve2 = runner.catala_gesamt_zve(g2)
                    if zve2 > 0:
                        if f.get("antrag_ermaessigter_satz", {}).get("wert") is True \
                                and _abs3_eligible(f, vz) and netto_vg <= 5_000_000:
                            # § 34 Abs. 3: plain grundtarif(verbleibendes zvE, S.3) + ermäßigter_satz × min(ao,5Mio).
                            est_rest = runner.catala_est({"veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                                                          "zu_versteuerndes_einkommen": max(0, zve2 - netto_vg)})
                            est_ao = runner.catala_ermaessigter_durchschnittssatz({
                                "ao_einkuenfte": netto_vg,
                                "est_gesamt_zzgl_progression": runner.catala_gesamt_tarifliche(g2),
                                "bemessungsgrundlage_durchschnitt": zve2})
                            g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=est_rest + est_ao)
                        else:
                            g2 = dict(g2, tarif_modifiziert=True, tarifliche_est_modifiziert=runner.catala_fuenftel({
                                "veranlagungszeitraum": vz, "veranlagung": g2["veranlagung"],
                                "zu_versteuerndes_einkommen": zve2, "ausserordentliche_einkuenfte": netto_vg}))
                # § 35 GewSt-Anrechnung Deckel-3 (JE §31-Zweig — tarifliche_est ist freibetrag-abhängig, global-
                # einmal würde den Kinderfreibetrag-Zweig über-crediten = stille Under-tax, 1:1 gesamt-Präzedenz).
                p35_credit_r = 0
                if p35_messbetrag > 0 and p35_zaehler > 0 and p35_nenner > 0:
                    tarifliche_raw = runner.catala_gesamt_tarifliche(g2)
                    tarifliche_gemindert = max(0, tarifliche_raw - dba_anrechnung)
                    # Weg-ii-Fix (K2, PFLICHT): ADDITIV statt hart überschreiben — sonst löscht § 35 GewSt-
                    # Anrechnung das § 35a-Ergebnis (steuerermaessigungen) still.
                    p35_credit_r = min(4 * p35_messbetrag, p35_messbetrag * p35_hebesatz // 100,
                                       p35_zaehler * tarifliche_gemindert // p35_nenner)
                    if not pe_active:
                        g2 = dict(g2, steuerermaessigungen=g2.get("steuerermaessigungen", 0) + p35_credit_r)
                result = runner.catala_est(g2)
                # §32b Post-Engine-Wrapper (Rentner-Ring, 1:1 gesamt-Präzedenz)
                if pe_active:
                    tarifliche_pre32b = runner.catala_gesamt_tarifliche(g2)
                    est_without_tarifliche = result - tarifliche_pre32b
                    zve32b = runner.catala_gesamt_zve(g2)
                    if zve32b > 0:
                        est_erhoeht = runner.catala_est({"veranlagungszeitraum": vz,
                                                          "veranlagung": g2.get("veranlagung", "einzel"),
                                                          "zu_versteuerndes_einkommen": zve32b + pe_raw})
                        tarifliche_32b = runner.catala_p32b_1({
                            "zu_versteuerndes_einkommen": zve32b,
                            "progressionseinkuenfte": pe_raw,
                            "est_auf_erhoehte_bemessung": est_erhoeht})
                        result = tarifliche_32b + est_without_tarifliche
                    # §35-Deckel-3 post-wrapper mit tarifliche_32b
                    if p35_credit_r > 0:
                        result = max(0, result - p35_credit_r)
                # SolZ-Tracking
                if freibetrag > 0 or kinder == 0:
                    solz_info_r["est_mit_fb"] = result
                return result

            # § 31 Familienleistungsausgleich (Günstigerprüfung Kindergeld vs Kinderfreibetrag § 32 Abs. 6, Fund D):
            # fehlte komplett im Rentner-Ring — Rentner mit Kindern verlor die Günstiger-Freibetrag-Anrechnung
            # (Over-tax). 1:1 gesamt-Naht-Präzedenz Z. 894-906. Ohne Kinder kein § 31.
            kinder = _c("fam_anzahl_kinder")
            if kinder > 0:
                est = runner.catala_p31_familienleistung({
                    "est_ohne_freibetraege": _festzusetzende_r(0),
                    "est_mit_freibetraegen": _festzusetzende_r(
                        kinder * runner._kinderfreibetrag(vz, rentner_g["veranlagung"])),
                    "kindergeld": kinder * runner._kindergeld(vz) * 12})
            else:
                est = _festzusetzende_r(0)
            # SolZ §3, §4 SolzG: Rentner-Ring hat kein §32d-Kapital → Basis = KiFB-fiktive ESt direkt
            if solz_container is not None and "est_mit_fb" in solz_info_r:
                solz_container[0] = runner.catala_solz({
                    "veranlagungszeitraum": vz,
                    "bemessungsgrundlage": solz_info_r["est_mit_fb"],
                    "splitting": rentner_g["veranlagung"] == "zusammen"})
            # KiSt § 51a: Rentner-Ring hat kein §32d-Kapital → Basis = KiFB-fiktive ESt direkt (= SolZ-Basis)
            if extras is not None and "est_mit_fb" in solz_info_r:
                extras["kist_cent"] = runner.catala_kist({
                    "est_mit_fb": solz_info_r["est_mit_fb"],
                    "konfession": f.get("kist_konfession", {}).get("wert", "keine"),
                    "bundesland": f.get("kist_bundesland", {}).get("wert", "")})
            return est
        return IV.bescheid_via_slots(bindung, slot_fn, quantitaet="festzusetzende_est")

    # festzusetzende_est_haushalt (§35a+§10b) + festzusetzende_est_agb (§33+§10-KiSt) ENTFERNT (Weg ii, Stage 1b):
    # ihre Abzüge sind in den gesamt-Ring gefaltet (siehe festzusetzende_est_gesamt slot_fn — GESAMT_ABZUEGE).
    return None     # kein exponierter Accessor -> ehrlich None (dHf/Verpflegung/AM/VOR/GWG)


def _feste_zahl(felder: dict, bindung: dict, cfg: dict, vz: int, scheibe_felder: tuple,
                store: dict | None = None):
    """Fail-closed: die festzusetzende Zahl NUR bei Scheiben-Gesamt-Accessor UND vollständig
    bestätigtem Input-Kegel (Meet). Ohne Gesamt-Accessor gibt es KEINE Scheiben-Zahl (ehrlich).
    `store` erlaubt dem §21-Ring die Multi-Objekt-Instanz-Σ (#5).
    Returns (zahl_euro, solz_cent, extras) — extras = Post-Engine-Zuschlag-/Prämien-Dict
    (kist_cent § 51a, mobilitaetspraemie_cent § 101; Schlüssel absent = nicht rechenbar)."""
    q = cfg["gesamt_ring"]
    if q is None:
        return None
    zustaende = [felder[f]["zustand"] for f in scheibe_felder if f in felder]
    if len(zustaende) < len(scheibe_felder) or ST.meet_zustand(zustaende) != "bestaetigt":
        return None
    solz_out = [None]   # mutable container — slot_fn schreibt SolZ hinein
    extras = {}         # slot_fn schreibt kist_cent (§51a) + mobilitaetspraemie_cent (§101) hinein
    bf = _bescheid_fn(q, vz, bindung, felder, store, solz_container=solz_out, extras=extras)
    if bf is None:
        return None
    zahl = bf({f: felder[f]["wert"] for f in scheibe_felder})
    return zahl, solz_out[0], extras


def _an_gesamt_sperrgrund(felder: dict, cfg: dict | None = None, vz: int | None = None,
                          store: dict | None = None, bindung: dict | None = None):
    """K2-Guard: nicht-ring-fähige Werbungskosten/Einkunftsarten sperren den Ring GANZ (nie Fake-0).
    Ein dHf-/Verpflegung-/AM-Feld mit Wert > 0 (vorläufig ODER bestätigt) sperrt (kein Catala-Modul);
    ein fremd_arten-Flag = false (Nutzer HAT eine NICHT von dieser Scheibe gerechnete Art) macht den
    §2-Ring unzulässig. VOR (§ 10) ist seit Stufe 1a ring-fähig — kein Guard mehr."""
    def _positiv(f):
        v = felder.get(f)
        w = v and v.get("wert")
        return isinstance(w, (int, float)) and not isinstance(w, bool) and w > 0
    # Partner-Behinderungsfeld (§ 33b Person B) ohne Zusammenveranlagung: benannte Inkonsistenz
    # (dev-2s partner_check, Spiegel zu partner_kegel_offen). Universell VOR der Scheiben-Verzweigung —
    # feuert live, sobald eine Scheibe (rentner_gesamt) die rentner_*_partner-Felder führt.
    if PC.partner_ohne_zusammen(felder):
        return "partner_konsistenz_offen"
    # § 24b Alleinerziehend ↔ Zusammenveranlagung (D-Fix, K2, Under-tax): fam_alleinstehend bestätigt True UND
    # veranlagung=zusammen → Widerspruch (§ 24b Abs. 1/3 verlangt „allein stehend", nicht zusammenveranlagt). Der
    # § 24b-Entlastungsbetrag würde sonst still gewährt = Unter-Besteuerung → fail-closed (dev-2s partner_check).
    # Universell vor der Scheiben-Verzweigung — feuert für jede Scheibe mit fam_alleinstehend + veranlagung (gesamt).
    if PC.alleinerziehend_mit_zusammen(felder):
        return "alleinerziehend_konsistenz_offen"
    # § 34 Abs. 3 >5Mio-Excess (Guard-A, fail-closed, beide Ringe): antrag_ermaessigter_satz ∧ eligible (55+/berufs-
    # unfähig ∧ ¬einmal) ∧ VÄ-Gewinn > 5 Mio → der ermäßigte Satz gilt nur bis 5 Mio (§ 34 Abs. 3 S. 1); der Excess
    # (Stufe-2b) ist unaufgelöst → fail-closed statt still auf Abs.1 fallen (das verweigerte den Abs.3-Benefit auf die
    # ersten 5 Mio = Over-tax). ¬eligible → KEIN Guard (Abs.1-Fünftel auf ganzes ao, kein 5Mio-Cap). _abs3_eligible =
    # bit-identisch zum Chooser. Schwelle auf raw VÄ-Gewinn = äquiv. netto_vg (§16-Abs.4-FB = 0 ab vg>181000 ≪ 5Mio).
    if vz is not None and felder.get("antrag_ermaessigter_satz", {}).get("wert") is True and _abs3_eligible(felder, vz) \
            and int(felder.get("rentner_veraeusserungsgewinn", {}).get("wert") or 0) // 100 > 5_000_000:
        return "abs3_ueber_5mio_offen"
    # an_gesamt Gap-A (K2, Over-tax): Kinder → §31/§32-KiFB-Rechnung NICHT in dieser Scheibe.
    # an_gesamt nutzt catala_est (kein §2-Gesamt-Scope, kein freibetraege_kinder); Kinder-Fälle
    # gehören in Scheibe "gesamt", die den vollen §31-Günstiger-§2-Lauf macht. Der Guard feuert
    # NUR für Scheiben OHNE gesamt_guard (d.h. an_gesamt; gesamt/rentner_gesamt haben gesamt_guard
    # = §31-fähig). Feuert bei bestätigtem fam_anzahl_kinder > 0 — kein stiller §31-loser Bescheid.
    # fam_anzahl_kinder == 0 (bestätigt kinderlose) → normale AN-Berechnung. Pflichtfeld im Kegel
    # (K2-safe: ohne Antwort keine festzusetzende Steuer). UI-Screening vor Scheibe-Wahl = Backlog.
    if cfg and not cfg.get("gesamt_guard") and _positiv("fam_anzahl_kinder"):
        return "kinder_gehoeren_in_gesamt"
    # an_gesamt Gap-B (K2, Over-tax): Verlustvortrag (§10d Abs.2) nicht in catala_est rechenbar
    # (kein §2-Gesamt-Scope, kein sonstige_abzuege_vom_einkommen-Slot). Gleiches Muster wie Gap-A.
    if cfg and not cfg.get("gesamt_guard") and _positiv("verlustvortrag_bestand"):
        return "verlustvortrag_gehoert_in_gesamt"
    # an_gesamt §32b (K2, Over-tax): Lohnersatz (Elterngeld/Krankengeld) ist bei Angestellten
    # HAEUFIG. an_gesamts catala_est hat KEIN extrahierbares zvE → GATE statt Under-tax.
    if cfg and not cfg.get("gesamt_guard") and _positiv("p32b_progressionseinkuenfte"):
        return "progression_gehoert_in_gesamt"
    if cfg and cfg.get("gesamt_guard"):
        # §34c DBA-Anrechnung (Stufe-1, K2): fail-closed bei Freistellungs-DBA, multi-country,
        # §32d-Kapital. Ohne diese Gates wäre die Anrechnung still 0 (gezahlt=0/ausl=0=absent) —
        # was bei vorhandenem (aber nicht gestütztem) DBA-Sachverhalt legitim = kein silent Over-tax.
        # Die GATES treffen nur die Fälle, wo der Nutzer aktiv DBA-Werte gesetzt hat, die diese
        # Scheibe nicht rechenbar macht → fail-closed (= keine stille 0-Anrechnung).
        dba_methode = (felder.get("dba_methode") or {}).get("wert")
        dba_mehrere = (felder.get("dba_mehrere_staaten") or {}).get("wert")
        if dba_methode == "dba_freistellung":
            return "dba_freistellung_offen"
        if dba_mehrere is True:
            return "dba_multi_country_offen"
        if any(_positiv(k) for k in KAP_TOEPFE) or (felder.get(KAP_ERTRAEGE, {}).get("wert") or 0) > 0:
            if _positiv("dba_auslaendische_einkuenfte"):
                return "dba_kapital_offen"
        # §32b-Koinzidenz (K2, fail-closed): §32b Post-Engine NACH §34/§35/§34c.
        # Co-Präsenz (Lohnersatz + ao-Gewinn/GewSt/DBA) unaufgelöst in Stufe-1 → fail-closed
        # (kein silent-wrong-Basis-Deckel). Stufe-2: korrekte Post-§32b-Höchstbeträge.
        p32b_pe = (felder.get("p32b_progressionseinkuenfte") or {}).get("wert")
        p32b_has_pe = isinstance(p32b_pe, (int, float)) and not isinstance(p32b_pe, bool) and p32b_pe > 0
        if p32b_has_pe:
            # 1. §34 ao-Gewinn / ermäßigter Satz
            ao_gewinn = (felder.get("rentner_veraeusserungsgewinn") or {}).get("wert")
            if (isinstance(ao_gewinn, (int, float)) and not isinstance(ao_gewinn, bool) and ao_gewinn > 0) \
                    or (felder.get("antrag_ermaessigter_satz", {}).get("wert") is True):
                return "p32b_kombi_offen"
            # 2. §35 Gewerbesteuer
            if _positiv("gewst_messbetrag"):
                return "p32b_kombi_offen"
            # 3. §34c DBA-Anrechnung
            if _positiv("dba_gezahlte_auslaendische_steuer") or _positiv("dba_auslaendische_einkuenfte"):
                return "p32b_kombi_offen"
        # Gesamt-Ring: Flag↔Einkunftsart-Widerspruch (kein_X=true + echtes Feld > 0 bestätigt) surfacen —
        # K2, keine still übergangene Einkunftsart (dev-2s flag_check).
        if FC.flag_widersprueche(felder):
            return "flag_konsistenz_offen"
        # Kapital-Semantik (Instructor-Q1, fail-closed): E1900701-Aggregat UND Verlust-Töpfe beide gesetzt
        # → additiv-vs-subset ungeklärt (benannter GAP) → kein Rate-Bescheid (die slot_fn nähme sonst still
        # nur die Töpfe und verschluckte das Aggregat).
        if _positiv(KAP_ERTRAEGE) and any(_positiv(t) for t in KAP_TOEPFE):
            return "kapital_semantik_offen"
        # Person B (#4b): dieselbe Single-source-Konsistenz für das Ehegatten-Kapital.
        if (felder.get("veranlagung", {}).get("wert") == "zusammen"
                and _positiv(KAP_ERTRAEGE_PARTNER) and any(_positiv(t) for t in KAP_TOEPFE_PARTNER)):
            return "kapital_semantik_offen"
        # §§ 13-18 Gewinn-Quelle SINGLE-SOURCE (Stufe 2a, fail-closed): direkter einkuenfte_gewinn UND eine
        # EÜR-Komponente (betriebseinnahmen/sonstige_BA/AfA) beide gesetzt → welcher laufende Gewinn gilt? Doppel-
        # quelle → kein Rate-Bescheid (_laufender_gewinn nähme sonst still die EÜR und verschluckte den Direktwert).
        # Spiegel kapital_semantik_offen. Entweder den Betrag DIREKT ODER komponentenweise, nicht beides.
        if _positiv("einkuenfte_gewinn") and any(_positiv(k) for k in EUER_KOMPONENTEN):
            return "gewinn_quelle_offen"
        # § 13 Land-/Forstwirtschaft ist NICHT EÜR-materialisiert (EuerGewinn-Bedingungen § 15 Abs. 2/§ 18 Abs. 1,
        # nicht § 13): gewinn_betriebsart=land_forst MIT EÜR-Komponente (und OHNE Direktwert) → luf_euer_offen,
        # fail-closed (NIE silent 0 — die EÜR gilt für LuF steuerlich anders, § 13a Durchschnittssätze etc.).
        # land_forst + Direktwert bleibt erlaubt (Stufe-1-Direktwert ist einkunftsart-agnostisch, keine EÜR-Rechnung).
        if (felder.get("gewinn_betriebsart", {}).get("wert") == "land_forst"
                and any(_positiv(k) for k in EUER_KOMPONENTEN) and not _positiv("einkuenfte_gewinn")):
            return "luf_euer_offen"
        # § 35 GewSt-Anrechnung (S1, fail-closed): der Steuermessbetrag ist da (opt-in), aber der Hebesatz fehlt →
        # die Anrechnung min(4×MB, MB×Hebesatz, …) ist ohne Hebesatz nicht rechenbar. KEIN 4×MB-Default (der
        # über-creditete bei Hebesatz < 400 % = Under-tax) → gewst_hebesatz_offen. Kein gewst_messbetrag = kein § 35
        # (over-tax-safe opt-out, feuert NICHT). Feld-präsenz-getrieben; Scheiben ohne die Felder → _positiv=False.
        if _positiv("gewst_messbetrag") and (felder.get("gewst_hebesatz") or {}).get("zustand") != "bestaetigt":
            return "gewst_hebesatz_offen"
        # Person B (#4): bei Zusammenveranlagung braucht der Ring den vollständig BESTÄTIGTEN Person-B-
        # Kegel (Bruttolohn + IdNr) — sonst kein halber Ehepaar-Bescheid (K2). Bei einzel irrelevant.
        if cfg.get("partner_19") and felder.get("veranlagung", {}).get("wert") == "zusammen":
            if any((felder.get(pf) or {}).get("zustand") != "bestaetigt"
                   for pf in GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP):
                return "partner_kegel_offen"
        # (A.2: der frühere partner_vorsorge_offen-Guard ist ENTFERNT — die Person-B-Vorsorge (VOR + KV/PV + § 24a)
        # ist jetzt im gesamt-slot_fn additiv verdrahtet, ein zusammen-Bescheid rechnet beider Ehegatten-Vorsorge
        # korrekt statt zu sperren. partner_vorsorge_offen bleibt im Schema-Enum als Alt-Grund erhalten, feuert nicht.)
        # Multi-Objekt § 21 (#5): jede WEITERE vv_objekt-Instanz (index ≥ 2) muss VOLLSTÄNDIG bestätigt sein —
        # alle 5 Basis-vv-Felder present UND per-Instanz-meet == bestaetigt (instanzen-Naht). Sonst kein Σ (K2:
        # eine halbe/vorläufige Objekt-Instanz erzeugte sonst ein still zu niedriges §21-Σ). Instanz 1 = der
        # Basis-Kegel, den _feste_zahl separat prüft (input_kegel_nicht_bestaetigt) — hier nur die Zusatzobjekte.
        gruppe = cfg.get("multi_objekt")
        if gruppe and store is not None and bindung is not None:
            pflicht = frozenset(VV_GESAMT_FELDER)   # 6 Pflicht-vv-Felder je Objekt (inkl. § 21-Abs.2-Entgelt-Quote)
            for inst in EM.instanzen(store, bindung, gruppe):
                # Subset-Check (nicht ==): die optionalen §21-Abs.2-Tatbestand-Felder (wohnzwecke/auf_dauer, auch
                # vv_objekt-getaggt) dürfen zusätzlich present sein → ALLE Pflicht-Felder present genügt.
                if inst["index"] >= 2 and (
                        not pflicht <= set(inst["felder"]) or inst["zustand"] != "bestaetigt"):
                    return "vv_instanz_offen"
        # § 22 aa Rentenfreibetrag-Fixierung (K2): ab dem 2. Jahr ist der Freibetrag in EURO fix; fehlt er
        # (aa-Folgejahr, renten_beginn < VZ, kein rentenfreibetrag) → fail-closed, kein %×erhöhte-Rente.
        if cfg.get("rentner"):
            def _fixierung_offen(art, beginn, rf):
                return (art in RENTNER_AA_ARTEN and isinstance(beginn, int) and vz is not None
                        and beginn < vz and not (isinstance(rf, (int, float)) and not isinstance(rf, bool)))
            # Multi-Rente (#6): Fixierung + Vollständigkeit JE Rente-Instanz der Person A (instanzen-Naht). Eine
            # Zusatz-Rente (index≥2) braucht die 4 Kern-Felder present + per-Instanz-meet bestaetigt (rentenfrei-
            # betrag optional = nur aa-Folgejahr) — sonst rente_instanz_offen (kein still zu niedriges §22-Σ, K2).
            # index 1 = Basis-Kegel (prüft _feste_zahl). Ohne store → nur die Basis-Fixierung (Alt-Aufrufer).
            if cfg.get("multi_rente") and store is not None and bindung is not None:
                kern = frozenset(RENTNER_22)
                for inst in EM.instanzen(store, bindung, cfg["multi_rente"]):
                    fi = inst["felder"]
                    if inst["index"] >= 2 and (not kern <= set(fi) or inst["zustand"] != "bestaetigt"):
                        return "rente_instanz_offen"
                    if _fixierung_offen(fi.get("rentner_renten_art", {}).get("wert"),
                                        fi.get("rentner_renten_beginn_jahr", {}).get("wert"),
                                        fi.get("rentner_rentenfreibetrag", {}).get("wert")):
                        return "rentenfreibetrag_fixierung_offen"
            elif _fixierung_offen(felder.get("rentner_renten_art", {}).get("wert"),
                                  felder.get("rentner_renten_beginn_jahr", {}).get("wert"),
                                  felder.get("rentner_rentenfreibetrag", {}).get("wert")):
                return "rentenfreibetrag_fixierung_offen"
            # Person B (#4b): dieselbe aa-Folgejahr-Fixierungs-Sperre für die Ehegatten-Rente bei zusammen.
            if felder.get("veranlagung", {}).get("wert") == "zusammen" and _fixierung_offen(
                    felder.get("rentner_renten_art_partner", {}).get("wert"),
                    felder.get("rentner_renten_beginn_jahr_partner", {}).get("wert"),
                    felder.get("rentner_rentenfreibetrag_partner", {}).get("wert")):
                return "rentenfreibetrag_fixierung_offen"
        # § 35a Abs. 5 S. 3 rechnung_unbar = CONDITIONAL-MANDATORY (K2, charge29): NUR wenn Dienstleistung
        # ODER Handwerker (Abs. 2/3) > 0 — Minijob (Abs. 1) verlangt keine unbare Zahlung. Unbeantwortet
        # (nicht bestätigt) → rechnung_unbar_offen (kein Abs2/3-Abzug ohne Beleg-/Überweisungsnachweis);
        # explizit false ist ANTWORT (Ring rechenbar, die slot_fn nullt Abs. 2/3), nur UNSET sperrt.
        # Feld-präsenz-getrieben (gilt für JEDE gesamt_guard-Scheibe, die diese Felder führt — haushalt/agb UND
        # der gefaltete gesamt-Ring, Weg ii). Scheiben ohne die Felder: _positiv/_num liefern absent→False/0.
        if _positiv("hh_dienstleistungen") or _positiv("hh_handwerker_arbeitskosten"):
            if (felder.get("hh_rechnung_unbar") or {}).get("zustand") != "bestaetigt":
                return "rechnung_unbar_offen"
        # § 10 Abs. 4b KiSt-Erstattungsüberhang (K2, #8): erstattete > gezahlte Kirchensteuer → fail-closed. Der
        # abziehbare Teil wäre 0, ABER die Überhang-Hinzurechnung zum GdE (§ 10 Abs. 4b S. 3) ist NICHT
        # materialisiert → ein stiller Abzug 0 würde unterbesteuern. Benannter Nachtrag → erstattungsueberhang_offen.
        def _num(fid):
            w = (felder.get(fid) or {}).get("wert")
            return w if isinstance(w, (int, float)) and not isinstance(w, bool) else 0
        if _num("kist_erstattet") > _num("kist_gezahlt"):
            return "erstattungsueberhang_offen"
        # fremd_arten = Arten, die DIESE Scheibe NICHT rechnet → bestätigt-false (Nutzer HAT die Art) sperrt
        # (Stufe 2). Die von der Scheibe GERECHNETEN Arten stehen NICHT in fremd_arten (kein Fehl-Sperr).
        if any(felder.get(fl, {}).get("wert") is False for fl in cfg.get("fremd_arten", ())):
            return "einkunftsart_nicht_ring_faehig"
        return None
    if any(_positiv(f) for f in GUARD_WERBUNGSKOSTEN):
        return "werbungskosten_nicht_ring_faehig"
    # dHf-Tatbestand: Kosten > 0, aber Auslandsunterkunft ODER eine der 4 Geltungsbedingungen offen
    # (nicht bestätigt) → kein Ring (K2: kein dHf-Abzug ohne bestätigten Tatbestand). Eine Bedingung
    # bestätigt-FALSE ist NICHT offen — dann greift dHf legitim nicht (Abzug 0), Ring bleibt gültig.
    if _positiv(DHF_KOSTEN):
        if felder.get("dhf_im_inland", {}).get("wert") is False:
            return "ausland_dhf_nicht_ring_faehig"
        if any((felder.get(b) or {}).get("zustand") != "bestaetigt" for b in DHF_BEDINGUNGEN):
            return "dhf_tatbestand_offen"
    # Verpflegung: Tage > 0 → Ring nur fähig, wenn BEIDE Reduktions-Guard-Felder EXPLIZIT sicher
    # sind (§ 9 Abs. 4a S. 6 3-Monats-Frist + S. 8 Mahlzeitenkürzung). FAIL-CLOSED bei UNSET: wer
    # Reisetage einträgt, aber die Reduktions-Fragen nicht beantwortet, bekommt keinen Über-Abzug.
    if sum((felder.get(t, {}).get("wert") or 0) for t in VERPFLEGUNG_TAGE) > 0:
        mon = felder.get("vpf_monate_am_ort", {}).get("wert")
        safe = (isinstance(mon, int) and not isinstance(mon, bool) and mon <= 3
                and felder.get("vpf_keine_mahlzeitengestellung", {}).get("wert") is True)
        if not safe:
            return "verpflegung_reduktion_offen"
    # Zusammenveranlagung: der Splitting-Ring braucht den vollständigen Kegel BEIDER Personen.
    if felder.get("veranlagung", {}).get("wert") == "zusammen":
        if any((felder.get(pf) or {}).get("zustand") != "bestaetigt" for pf in AN_GESAMT_PARTNER):
            return "partner_kegel_offen"        # Person-B-Pflichtfeld offen → kein halber Bescheid
        if any(_positiv(vf) for vf in VOR_FELDER + VOR_PARTNER_FELDER):
            return "partner_vor_offen"           # MVP-zusammen ohne VOR; VOR-Feld (A/B) gesetzt sperrt
    if any(felder.get(f, {}).get("wert") is False for f in AN_GESAMT_FLAGS):
        return "einkunftsart_nicht_ring_faehig"
    return None


# ----------------------------------------------------------------- Endpunkte (reine Logik)

def fall_anlegen(body: dict) -> tuple[int, dict]:
    scheibe = body.get("scheibe", "ep")
    if scheibe not in SCHEIBEN:
        raise ApiError(400, f"unbekannte Scheibe {scheibe!r}")
    vz = int(body.get("veranlagungszeitraum", 2025))
    fall_id = body.get("fall_id")
    if not fall_id or not _FALL_RE.fullmatch(str(fall_id)):
        raise ApiError(400, "fall_id fehlt oder ungültig (nur [A-Za-z0-9_-]{1,64})")
    if os.path.exists(_fall_pfad(fall_id)):
        raise ApiError(409, f"Fall {fall_id!r} existiert bereits")
    store = ST.leerer_store(vz, fall_id=fall_id)
    store["scheibe"] = scheibe
    speichere_fall(fall_id, store)
    return 201, {"fall_id": fall_id, "scheibe": scheibe, "veranlagungszeitraum": vz}


def _badge(herkunft: dict) -> str:
    """Herkunfts-Kategorie je Wert (UI-Lab Dim 1, „Herkunft zum Anfassen"): die 6 Store-Herkünfte
    direkt als anzeigbare Badge-Klasse (statt binär solide/schimmernd). Die Haut stylt jede Kategorie
    (laie=selbst · beleg_import=Beleg · vorjahr/berechnet/orakel=abgeleitet · llm_vorschlag=KI-schimmernd)."""
    return herkunft.get("herkunft", "laie")


def _ring_bindung(cfg: dict, bindung: dict) -> dict:
    """Bindung für die Spannen-/intervall-Rechnung: nur die Pflicht-Kegel-Felder. Sonst zögen die
    (bei einzel ungesetzten) Partner-Felder als unbounded-ohne-Wert das Intervall auf nicht_fixierbar."""
    kegel = cfg.get("kegel")
    return {f: bindung[f] for f in kegel if f in bindung} if kegel else bindung


def _gesamt_beitrag(store: dict, cfg: dict, bindung: dict, felder: dict, sid: str, vz: int):
    """Frage-Reihenfolge-Gewichte aus dem verfügbaren Ring (Gesamt bevorzugt, sonst erster Teil)."""
    if cfg["gesamt_ring"]:
        rb = _ring_bindung(cfg, bindung)
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, rb, felder, store, nur_bestaetigt=False)  # Estimate-Pfad: vorläufig zeigt Wirkung im Range
        if bf is not None:
            return {b["feld_id"]: b["spanne_cent"]
                    for b in IV.intervall(felder, rb, bf, snapshot_id=sid)["beitraege"]}
    for _name, q, tfelder in cfg["teil_ringe"]:
        tb = {f: bindung[f] for f in tfelder if f in bindung}
        bf = _bescheid_fn(q, vz, tb, nur_bestaetigt=False)   # Estimate-Pfad (fragen-Gewichte)
        if bf is not None:
            return {b["feld_id"]: b["spanne_cent"]
                    for b in IV.intervall(felder, tb, bf, snapshot_id=sid)["beitraege"]}
    return None


def fragen(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    beitrag = _gesamt_beitrag(store, cfg, bindung, felder, sid, int(store["veranlagungszeitraum"]))
    queue = TR.naechste_fragen(store, bindung, beitrag)
    out = []
    for fid in queue:
        b = bindung[fid]
        out.append({
            "feld_id": fid,
            "fragetext_laie": b.get("fragetext_laie"),
            "hilfe_kurz": b.get("hilfe_kurz"),
            "typ": b["typ"],
            "einheit": b.get("einheit"),
            "bereich": b.get("bereich"),
            "enum_werte": b.get("enum_werte"),
            "beispielwert": b.get("beispielwert"),
            "anker_ref": b.get("anker_ref"),
        })
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "fragen": out}


def stand(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    rel = TR.relevanz(store, bindung)
    vz = int(store["veranlagungszeitraum"])
    felder_out = {
        fid: {"wert": v["wert"], "zustand": v["zustand"], "herkunft": v["herkunft"],
              "herkunft_badge": _badge(v["herkunft"])}
        for fid, v in felder.items()
    }

    gesamt_iv, engine, teil = None, "unavailable", []
    gesperrt = _an_gesamt_sperrgrund(felder, cfg, vz, store, bindung) if cfg.get("guard") else None
    if gesperrt:
        engine = "gesperrt"          # nicht-ring-fähiger Abzug/Einkunftsart -> kein Ring (K2)
    elif cfg["gesamt_ring"]:
        rb = _ring_bindung(cfg, bindung)
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, rb, felder, store, nur_bestaetigt=False)  # Estimate-Pfad: vorläufig zeigt Wirkung im Range
        if bf is not None:
            gesamt_iv = IV.intervall(felder, rb, bf, snapshot_id=sid)["intervall"]
            engine = "catala"
    else:
        for name, q, tfelder in cfg["teil_ringe"]:
            tb = {f: bindung[f] for f in tfelder if f in bindung}
            bf = _bescheid_fn(q, vz, tb, felder, nur_bestaetigt=False)   # Estimate-Pfad (/stand-Teil-Range)
            if bf is not None:
                tiv = IV.intervall(felder, tb, bf, snapshot_id=sid)["intervall"]
                teil.append({"familie": name, "quantitaet": q, "intervall": tiv})
        engine = "catala_teilweise" if teil else "unavailable"

    return 200, {"fall_id": fall_id, "snapshot_id": sid, "engine": engine,
                 "felder": felder_out, "relevanz": rel, "intervall": gesamt_iv,
                 "teil_ringe": teil, "ring_gesperrt": gesperrt}


_ERLAUBTE_ZUSTAENDE = {"vorlaeufig", "bestaetigt"}


def event(fall_id: str, body: dict) -> tuple[int, dict]:
    """DER einzige Schreib-Endpunkt — dünne Hülle über store.append_event. Die fail-closed-Garantien
    (llm->vorlaeufig, bestaetigt->signal_2, ein aktives Event/feld) erzwingt der Store, nicht die Haut."""
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    fid = body.get("feld_id")
    if fid not in bindung:
        # Repeated-Instance (#5): base__n einer instanz-fähigen Basis-Bindung ist ein gültiges Instanz-Feld
        # (Instanz 2..N eines Multi-Objekt-/Multi-Rente-Konsumenten). Die Instanz ist reine est_mapping-
        # Konvention — der Store lernt sie nicht, aber der Schreibpfad muss sie durchlassen (parse_instanz =
        # DIESELBE Enumerations-Wahrheit wie instanzen/deklariere, kein zweites Regex).
        parsed = EM.parse_instanz(fid) if isinstance(fid, str) else None
        basis = parsed[0] if parsed else None
        if not (basis and bindung.get(basis, {}).get("instanz_gruppe")):
            raise ApiError(400, f"feld_id {fid!r} nicht in dieser Scheibe")
    zustand = body.get("zustand")
    if zustand not in _ERLAUBTE_ZUSTAENDE:
        raise ApiError(400, f"zustand muss {_ERLAUBTE_ZUSTAENDE} sein")
    herkunft = body.get("herkunft")
    if not isinstance(herkunft, dict) or "herkunft" not in herkunft:
        raise ApiError(400, "herkunft-Objekt (mit Schlüssel 'herkunft') ist Pflicht")
    schreiber = body.get("schreiber")
    if not isinstance(schreiber, str) or not schreiber:
        raise ApiError(400, "schreiber ist Pflicht")
    # dev-2-Kontrakt: der Katalog-Check gilt für Vorschlags-Schreiber (llm:/berechnet:/import:beleg/kontoauszug) —
    # ein Client-gesetzter llm:-Schreiber über /event darf die human-only-Felder NICHT umgehen. mensch (ui:/import:
    # vorjahr/import:elster) ist nicht betroffen → kein Katalog (Confirm braucht keins).
    _vorschlag = isinstance(schreiber, str) and (
        schreiber.startswith(("llm:", "berechnet:", "import:beleg", "import:kontoauszug")))
    try:
        ev = ST.append_event(
            store, feld_id=fid, wert=body.get("wert"), zustand=zustand, herkunft=herkunft,
            schreiber=schreiber, signal=body.get("signal"), ersetzt=body.get("ersetzt"),
            ts=body.get("ts"),
            # GLOBALER Katalog (TR.lade_bindung()), nicht per-Scheibe: die Autorisierung hängt am Feld, dev-2-Kontrakt.
            katalog=(ST.lade_katalog(TR.lade_bindung()) if _vorschlag else None))
    except ValueError as e:
        # fail-closed-Abweisung des Stores -> 422 (nicht 500): die Haut hat korrekt weitergereicht.
        raise ApiError(422, str(e))
    speichere_fall(fall_id, store)
    return 201, {"event_id": ev["event_id"], "feld_id": fid, "zustand": zustand}


def warum(fall_id: str, feld_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    j = TR.justification(store, feld_id, bindung)
    if j is None:
        raise ApiError(404, f"Feld {feld_id!r} hat (noch) kein Event")
    return 200, {"fall_id": fall_id, "justification": j}


def ergebnis(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    cfg = _cfg(store)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    # Pflicht-Kegel: einzel-Basis (cfg["kegel"]); die Partner-Pflichtfelder gehören nur bei
    # veranlagung=zusammen dazu und werden dort vom Guard geprüft.
    scheibe_felder = cfg.get("kegel") or _scheibe_felder(store)
    vz = int(store["veranlagungszeitraum"])
    if cfg.get("guard"):
        # K2: ein nicht-ring-fähiger Abzug/Einkunftsart sperrt den Ring VOR jeder Zahl — nie Fake-Bescheid.
        sperr = _an_gesamt_sperrgrund(felder, cfg, vz, store, bindung)
        if sperr:
            return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                         "solz_cent": None, "kist_cent": None, "mobilitaetspraemie_cent": None,
                         "grund": sperr, "offen": [], "trace": None}
    result = _feste_zahl(felder, bindung, cfg, vz, scheibe_felder, store)
    if result is None:
        if cfg["gesamt_ring"] is None:
            # Multi-Regel-Scheibe ohne ehrlichen Gesamt-Accessor: bewusst KEINE Scheiben-Zahl.
            return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                         "solz_cent": None, "kist_cent": None, "mobilitaetspraemie_cent": None,
                         "grund": "kein_scheiben_gesamtbescheid", "offen": [], "trace": None}
        offen = [f for f in scheibe_felder
                 if f not in felder or felder[f]["zustand"] != "bestaetigt"]
        bf = _bescheid_fn(cfg["gesamt_ring"], vz, bindung, felder)
        grund = "engine_unavailable" if (bf is None and not offen) else "input_kegel_nicht_bestaetigt"
        return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": None,
                     "solz_cent": None, "kist_cent": None, "mobilitaetspraemie_cent": None,
                     "grund": grund, "offen": sorted(offen), "trace": None}
    zahl, solz, extras = result
    trace = TR.trace_ergebnis(store, bindung, snapshot_id=sid)
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "zahl_cent": zahl,
                 "solz_cent": solz, "kist_cent": extras.get("kist_cent"),
                 "mobilitaetspraemie_cent": extras.get("mobilitaetspraemie_cent"),
                 "grund": "bestaetigt", "offen": [], "trace": trace}


def deklaration(fall_id: str) -> tuple[int, dict]:
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    result = EM.deklariere(felder, bindung, snapshot_id=sid)
    return 200, {"fall_id": fall_id, **result}


def graph(fall_id: str) -> tuple[int, dict]:
    """Read-only Abhängigkeits-Übersicht (Desktop): Knoten = Regeln der Scheibe mit ihrem
    Relevanz-Status (aus traverser.relevanz), Kanten = Feld→Regel (welches Abfrage-Feld welche Regel
    speist, mit Feld-Zustand). Reine Ableitung, EIN Traverser-Aufruf, kein Bescheid, kein Schreibpfad."""
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    felder, sid = ST.materialisiere(store)
    rel = TR.relevanz(store, bindung)
    knoten = [{"regel_id": rid, "status": s["status"],
               "gates_offen": s["gates_offen"], "annahmen_offen": s["annahmen_offen"]}
              for rid, s in sorted(rel.items())]
    kanten = []
    for fid in sorted(bindung):
        q = bindung[fid]["quelle"]
        ev = felder.get(fid)
        kanten.append({
            "feld_id": fid,
            "regel_id": q["regel_id"],
            "rolle": "slot" if "signatur_slot" in q else "gate",
            "zustand": ev["zustand"] if ev else "offen",
            "fragetext_laie": bindung[fid].get("fragetext_laie"),
        })
    return 200, {"fall_id": fall_id, "snapshot_id": sid, "knoten": knoten, "kanten": kanten}


# POST /chat und POST /elster-ampel: bewusst KEINE 200-Antwort in dieser Stufe.
CHAT_501 = {
    "fehler": "not_implemented",
    "vertrag": ("LLM-Chat schreibt qua Store-Auflage A ausschliesslich vorlaeufig-Events "
                "(schreiber='llm:…', herkunft.herkunft='llm_vorschlag', signal_2=null); "
                "Bestätigung bleibt der menschliche Zwei-Signal-Klick."),
    "stufe": "spätere Stufe mit eigenem Julius-Cap — kein LLM-Call in dieser Stufe.",
}
AMPEL_503 = {
    "fehler": "unavailable",
    "grund": ("ELSTER-Ampel (warmer checkESt-Daemon) ist für diese Scheibe noch nicht verdrahtet "
              "— ein gültiger ESt-Fall entsteht erst mit der Gesamtsteuer-Integration. Kein Fake-Grün."),
    "regel": "gekappt_verdacht=true ist nie grün (API.md-Garantie 5).",
}
# Arbeitsweg-Entfernung über Karten-Dienst (Julius-Feature): der Geocoding+Routing-Aufruf ist LIVE
# (ors_client, echter Call) — eine AUSGEHENDE Integration mit PII (Adressen verlassen das Gerät), daher
# nur env-key-gated ($ORS_API_KEY, .env.maps). Kein Key / Netz-/Antwort-Fehler → sauberer Fallback auf
# die manuelle km-Eingabe (ENTFERNUNG_FALLBACK unten), nie Crash, nie Fake-km.
ENTFERNUNG_FALLBACK = {
    "fehler": "unavailable",
    "vertrag": ("Der Karten-Dienst ist nicht verbunden (kein Schlüssel gesetzt oder Netz-/Antwort-Fehler) "
                "— bitte gib die Entfernung manuell ein (kürzeste Straßenverbindung, § 9 Abs. 1 S. 3 Nr. 4 EStG)."),
}


def entfernung(fall_id: str, body: dict) -> tuple[int, dict]:
    """Arbeitsweg-km über den Karten-Dienst (Julius-Feature). AUSGEHENDE PII-Integration: die Adressen
    gehen an OpenRouteService (nur auf Nutzer-Klick). Das Ergebnis ist ein VORSCHLAG — die Haut prefillt
    das km-Feld, der Nutzer bestätigt/überschreibt (Zwei-Signal, § 9 kürzeste Straßenverbindung; eine
    längere ist bei regelmäßiger Nutzung zulässig). Kein Key / Netzfehler → 503-Fallback (manuell), nie
    Crash, nie still gesetzt. Der API-Key kommt nur aus $ORS_API_KEY (nie im Repo)."""
    store = lade_fall(fall_id)                            # 404, wenn der Fall nicht existiert
    von = (body.get("von") or "").strip()
    nach = (body.get("nach") or "").strip()
    if not von or not nach:
        raise ApiError(400, "von und nach (Adressen) sind Pflicht")
    bindung = _scheibe_bindung(store)
    if "ep_entfernung_km" not in bindung:
        raise ApiError(400, "diese Scheibe hat kein Arbeitsweg-km-Feld")
    import ors_client
    try:
        km = ors_client.entfernung_km(von, nach)
    except (ors_client.OrsNichtVerfuegbar, ImportError):  # Cap-Gate/Netzfehler/Import → Erklär-Grenze;
        return 503, ENTFERNUNG_FALLBACK                   # ein echter Logik-Bug propagiert (K2, konsistent zu chat()/kontoauszug)
    # PROVENIENZ (K2, „Herkunft je Wert"): der km-Wert kommt aus dem Karten-Dienst → als VORLÄUFIGES
    # Event mit herkunft=berechnet ins Store (Badge zeigt „berechnet/maps", NICHT „selbst"). Der Nutzer
    # bestätigt/überschreibt (Zwei-Signal). Ein aktives Event des Felds wird ersetzt (Nutzer hat „berechnen"
    # geklickt = er will den Karten-Vorschlag). signal_1 trägt die Provenienz (KEINE Adressen — PII-sparsam).
    aktiv_ev = ST._aktives(store).get("ep_entfernung_km")
    try:
        ev = ST.append_event(
            store, feld_id="ep_entfernung_km", wert=km, zustand="vorlaeufig",
            herkunft={"herkunft": "berechnet", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            schreiber="berechnet:maps",
            signal={"signal_1": {"typ": "maps", "dienst": "openrouteservice"}, "signal_2": None},
            ersetzt=(aktiv_ev["event_id"] if aktiv_ev else None),
            katalog=ST.lade_katalog(TR.lade_bindung()))  # berechnet:-Schreiber → GLOBALER Katalog-Check (dev-2-Kontrakt)
    except ValueError as e:
        raise ApiError(422, str(e))
    speichere_fall(fall_id, store)
    return 200, {"km": km, "event_id": ev["event_id"], "herkunft": "berechnet",
                 "hinweis": "Vorschlag aus dem Karten-Dienst — bitte prüfen und bestätigen."}


def vorjahr(fall_id: str, body: dict) -> tuple[int, dict]:
    """Vorjahr-Übernahme (dev-2s vorjahr_writer): überträgt die vorjahr-flagged, im Vorjahres-Fall
    BESTÄTIGTEN Felder als VORLÄUFIGE Vorschläge (herkunft=vorjahr) in den aktuellen Fall — der Nutzer
    bestätigt/überschreibt (Zwei-Signal). Der Store-Guard ^import:vorjahr erzwingt vorläufig strukturell;
    schon belegte Felder bleiben unangetastet (One-Active-Event)."""
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    vj_id = body.get("vorjahr_fall_id")
    if not vj_id or not _FALL_RE.fullmatch(str(vj_id)):
        raise ApiError(400, "vorjahr_fall_id fehlt oder ungültig")
    if vj_id == fall_id:
        raise ApiError(400, "vorjahr_fall_id muss ein ANDERER (Vorjahres-)Fall sein")
    vj_store = lade_fall(vj_id)                           # 404, wenn der Vorjahres-Fall fehlt
    vj_felder, _ = ST.materialisiere(vj_store)
    n = VW.uebernehme_vorjahr(store, vj_felder, bindung,
                              vorjahr_vz=int(vj_store.get("veranlagungszeitraum", 0)))
    speichere_fall(fall_id, store)
    return 200, {"uebernommen": n, "vorjahr_fall_id": vj_id}


def _kontoauszug_llm_klassifikator():
    """Baut den Kontoauszug-LLM-Fallback-Klassifikator (dev-2s kontoauszug_writer.llm_klassifikator_factory,
    llm_client-MODUL als `client` — hat `.complete`, kein Klassen-Bau nötig). Cap-gated wie /chat: JEDER Aufruf
    fängt NUR LlmNichtVerfuegbar (Cap-Gate/Netzfehler — die Factory selbst fängt nichts) und liefert None (=
    unklassifiziert, wie bisher llm_klassifikator=None) statt den GESAMTEN Upload bei der ERSTEN mehrdeutigen
    Transaktion abstürzen zu lassen (Regression ggü. det-only). Ein Logik-/Parse-Bug ist KEIN erwarteter
    Cap-Gate-Fall — der propagiert bewusst (K2: silent-swallow eines echten Bugs ist selbst ein Risiko)."""
    import llm_client
    import kontoauszug_writer as KW
    roh = KW.llm_klassifikator_factory(llm_client, "kontoauszug_klassifikation")

    def klassifikator(zweck, betrag):
        try:
            return roh(zweck, betrag)
        except llm_client.LlmNichtVerfuegbar:
            return None
    return klassifikator


def kontoauszug(fall_id: str, body: dict) -> tuple[int, dict]:
    """Kontoauszug-Upload (dev-2s kontoauszug_writer): parst den Auszug und schreibt je AUSGABEN-Transaktion
    mit eindeutiger deterministischer Kategorie + Ziel-Feld in DIESER Scheibe einen VORLÄUFIGEN Vorschlag
    (herkunft=kontoauszug, Store-Guard ^import:kontoauszug erzwingt vorläufig). Der Nutzer bestätigt neben
    dem Auszug (K2). Deterministik-first: der LLM-Klassifikator-Fallback (mehrdeutige Zwecke) ist verdrahtet,
    aber selbst cap-gated (kein $LLM_API_KEY → jede Transaktion fällt still auf "unklassifiziert" zurück, kein
    Crash, kein Mock-Call). IBAN/Kontonummern werden vom Writer maskiert (PII). Kein Überschreiben aktiver
    Felder. § 35a-Kategorien greifen nur, wenn die Scheibe die Ziel-Felder führt (sonst 0 Vorschläge)."""
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    fmt = (body.get("format") or "").strip().lower()
    inhalt = body.get("inhalt")
    import kontoauszug_writer as KW
    n_verworfen = 0
    if fmt == "csv":
        tx = KW.parse_csv(inhalt if isinstance(inhalt, str) else "")
    elif fmt == "json":
        try:
            tx = inhalt if isinstance(inhalt, list) else json.loads(inhalt or "[]")
        except (ValueError, TypeError):
            raise ApiError(400, "json-Inhalt nicht parsebar")
        if not isinstance(tx, list):
            raise ApiError(400, "json muss eine Liste von Transaktionen sein")
    elif fmt == "pdf":
        # PDF kommt als base64 im JSON-Body (server.py macht NUR json.loads, kein Multipart) — auf
        # Disk entpackt, weil der OCR/Layout-Pfad (wie beleg_writer.lies_beleg_text) pfad-basiert ist,
        # nicht bytes-basiert. tmp-Datei trägt den ROHEN Bank-Auszug (PII/IBAN vor Writer-Maskierung)
        # → finally: os.unlink, UNBEDINGT auch bei Exception im OCR/Parse-Pfad (kein Disk-Leck).
        if not isinstance(inhalt, str) or not inhalt.strip():
            raise ApiError(400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)")
        try:
            pdf_bytes = base64.b64decode(inhalt, validate=True)
        except ValueError:              # binascii.Error ist eine ValueError-Unterklasse (verifiziert)
            raise ApiError(400, "pdf-Inhalt nicht gültig base64-kodiert")
        fd, pfad = tempfile.mkstemp(suffix=".pdf")
        try:
            with os.fdopen(fd, "wb") as fh:
                fh.write(pdf_bytes)
            text, conf_map = KW.lies_kontoauszug_pdf(pfad)
            tx, n_verworfen = KW.parse_pdf_zeilen(text, conf_map)
        finally:
            os.unlink(pfad)
    else:
        raise ApiError(400, "format muss csv, json oder pdf sein")
    # katalog GLOBAL (dev-2-Kontrakt): Enforcement decoupled vom per-Scheibe-Targeting.
    n = KW.uebernehme_kontoauszug(store, tx, bindung, llm_klassifikator=_kontoauszug_llm_klassifikator(),
                                  katalog=ST.lade_katalog(TR.lade_bindung()))
    speichere_fall(fall_id, store)
    out = {"uebernommen": n, "transaktionen": len(tx), "verworfen": n_verworfen}
    if n_verworfen > 0:
        out["hinweis"] = f"{n_verworfen} Zeile(n) unsicher erkannt (Confidence < 60%) — bitte manuell prüfen/nachtragen."
    return 200, out


def _chat_prompt(freitext: str, katalog: list[dict]) -> list[dict]:
    """Baut die OpenAI-kompatible messages-Liste für den Chat-Vorschlags-Task. System-Regel: die KI darf
    AUSSCHLIESSLICH die Felder aus dem übergebenen Katalog vorschlagen (askable + vorschlagbar; der Store-
    Katalog-Check ist die zweite Verteidigung), NUR als Vorschlag, mit Feld-Metadaten (fragetext/typ/bereich/
    enum). Antwort = striktes JSON. Task-Wrapper (Handler-Schicht) — der Client (llm_client) kennt diesen
    Prompt nicht."""
    felder = "\n".join(
        f"- {f['feld_id']}: {f.get('fragetext_laie', '')}"
        f" (Typ {f.get('typ', '')}"
        + (f", Bereich {f['bereich']}" if f.get("bereich") else "")
        + (f", Werte {f['enum_werte']}" if f.get("enum_werte") else "")
        + ")"
        for f in katalog)
    system = (
        "Du bist ein Steuer-Assistent, der aus der Freitext-Beschreibung eines Nutzers Feld-Werte VORSCHLÄGT. "
        "Du SETZT nie einen Wert und triffst keine rechtliche Entscheidung — der Mensch bestätigt jeden Vorschlag. "
        "Du darfst NUR diese Felder vorschlagen (keine anderen):\n" + felder + "\n\n"
        "Geld-Beträge MUSST du als GANZZAHL in CENT angeben (EUR × 100), z.B. 2156,50 € → 215650. "
        "Niemals als EUR-Kommazahl oder EUR-Ganzzahl.\n"
        "Antworte AUSSCHLIESSLICH mit einem JSON-Array [{\"feld_id\":\"…\",\"wert\":…,\"begruendung\":\"kurz\"}], "
        "nur Felder für die die Beschreibung einen konkreten Wert hergibt, sonst []. Kein Fließtext.")
    return [{"role": "system", "content": system}, {"role": "user", "content": freitext}]


def _chat_parse(text: str) -> list[dict]:
    """Roher LLM-Text → Liste {feld_id, wert, begruendung}. Toleriert ein Objekt-Wrapper ({\"vorschlaege\":[…]})
    oder ein nacktes Array. Nicht-Liste/kaputtes JSON → [] (kein Vorschlag ist besser als ein Müll-Vorschlag)."""
    try:
        j = json.loads(text)
    except Exception:
        return []
    if isinstance(j, dict):
        for k in ("vorschlaege", "vorschläge", "suggestions", "felder"):
            if isinstance(j.get(k), list):
                j = j[k]
                break
        else:
            j = [j] if "feld_id" in j else []
    if not isinstance(j, list):
        return []
    out = []
    for v in j:
        if isinstance(v, dict) and "feld_id" in v and "wert" in v:
            out.append({"feld_id": str(v["feld_id"]), "wert": v["wert"],
                        "begruendung": str(v.get("begruendung", ""))[:200]})
    return out


def _llm_vorschlaege(freitext: str, katalog: list[dict]) -> list[dict]:
    """Chat-Task-Wrapper (Handler-Schicht) ÜBER llm_client.complete (der einen niedrig-level Wahrheit). Cap-
    gated: kein Key/Base/Modell → LlmNichtVerfuegbar propagiert (der /chat-Handler fängt sie → 501). Der
    Aufrufer schreibt jeden Vorschlag als VORLÄUFIGES Event (Store-Auflage A + Katalog-Check erzwingen die
    Sicherheit); der Mensch bestätigt via Hold-Confirm."""
    if not (freitext or "").strip():
        return []
    import llm_client
    comp = llm_client.complete("chat", _chat_prompt(freitext, katalog))
    return _chat_parse(comp.text)


def chat(fall_id: str, body: dict) -> tuple[int, dict]:
    """Chat-Berater (K1): der Nutzer beschreibt seine Situation in Freitext → das LLM SCHLÄGT Feld-Werte VOR →
    jeder Vorschlag wird als VORLÄUFIGES Event geschrieben (schreiber='llm:chat'). Store-Auflage A + der Katalog-
    Check (katalog=lade_katalog) erzwingen strukturell: herkunft=llm_vorschlag, zustand=vorlaeufig, signal_2=null
    (nie in die Summe ohne menschlichen Hold-Confirm) UND nur Felder die der Katalog als LLM-vorschlagbar führt
    (identitäts-/rechtskritische Felder lehnt der Check ab). CAP-GATED: kein LLM-Key/Provider → 501 + Erklär-
    Vertrag ($0, kein Mock-Call). Ein einzelner abgelehnter Vorschlag (Katalog/Auflage A) überspringt still —
    der Rest bleibt gültig, nie ein Crash, nie ein Fake-Wert. Die KI setzt NIE einen Wert."""
    store = lade_fall(fall_id)
    bindung = _scheibe_bindung(store)
    freitext = (body.get("text") or "").strip()
    # ZWEI Kataloge (dev-2-Kontrakt, msg 4365 — NICHT verwechseln):
    #  (1) PROMPT-Katalog (Haut-Zone): die LLM-vorschlagbaren Felder DIESER Scheibe als Metadaten-LISTE
    #      [{feld_id, fragetext_laie, typ, bereich, enum_werte}] — nur Kontext für die KI, welche Felder sie
    #      überhaupt vorschlagen darf. Wird an _llm_vorschlaege übergeben (dessen _chat_prompt eine Liste will).
    #  (2) CHECK-Katalog (Store-Enforcement): GLOBAL via TR.lade_bindung(), Form {typ→frozenset(feld_id)} — die
    #      un-bypassbare Untergrenze in append_event. GLOBAL, NICHT per-Scheibe: die Autorisierung eines Felds
    #      hängt an seinem `vorschlagbar_von`, nicht an der offenen Scheibe (ein per-Scheibe-Check-Katalog würde
    #      Vorschlags-Schreiber auf global-autorisierte Nicht-Scheibe-Felder fälschlich abweisen — fail-OPEN wäre
    #      es NIE, aber falsch-abweisen bricht legitime beleg/kontoauszug-Writes).
    prompt_katalog = [
        {"feld_id": fid, "fragetext_laie": b.get("fragetext_laie", ""), "typ": b.get("typ"),
         "bereich": b.get("bereich"), "enum_werte": b.get("enum_werte")}
        for fid, b in bindung.items() if "llm" in (b.get("vorschlagbar_von") or [])]
    check_katalog = ST.lade_katalog(TR.lade_bindung())
    import llm_client
    try:
        vorschlaege = _llm_vorschlaege(freitext, prompt_katalog)
    except (llm_client.LlmNichtVerfuegbar, ImportError):   # Cap-Gate/Import → reine Erklär-Grenze (kein Key, $0);
        return 501, CHAT_501                               # echte Logik-/Parse-Bugs propagieren (konsistent zu kontoauszug)
    geschrieben, abgelehnt = [], []
    for v in vorschlaege:
        try:
            ev = ST.append_event(
                store, feld_id=v["feld_id"], wert=v["wert"], zustand="vorlaeufig",
                herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                schreiber="llm:chat",
                signal={"signal_1": {"typ": "llm", "begruendung": v.get("begruendung", "")}, "signal_2": None},
                katalog=check_katalog)               # dev-2s GLOBALER Katalog-Check lehnt human-only-Felder fail-closed ab
            geschrieben.append({"feld_id": v["feld_id"], "event_id": ev["event_id"], "wert": v["wert"]})
        except (ValueError, KeyError):
            abgelehnt.append(v.get("feld_id"))       # Katalog/Auflage-A-Abweisung → still überspringen, Rest gilt
    speichere_fall(fall_id, store)
    _abg = [a for a in abgelehnt if a]
    if _abg:                                         # Security-Observability (feld_ids, KEIN Wert/Freitext = PII-frei):
        sys.stderr.write(f"[haut.chat] LLM-Vorschläge außerhalb Katalog abgelehnt: {sorted(set(_abg))}\n")
    return 200, {"vorschlaege": geschrieben, "abgelehnt": _abg,
                 "hinweis": "Vorschläge erfasst — bitte im Fluss neben jedem Wert bestätigen (die KI setzt nichts)."}
