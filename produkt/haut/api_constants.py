"""Paket-B Haut — modul-level Konstanten (Feld-Tupel, Scheiben-Konfiguration).

Pure data: Tupel + Dict für Feld-Aggregationen, Scheiben-Routing, DBA-Methoden.
Keine Funktionen, keine Abhängigkeiten außer os/sys für Pfade.
"""
from __future__ import annotations

import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))


def _daten_wurzel() -> str:
    """Wo die Steuerdaten liegen — AUSSERHALB des Projektverzeichnisses.

    Bis 2026-08-19 lagen sie in `produkt/haut/faelle`, also mitten im Arbeitsbaum: Steuer-ID,
    Einkommen und IBAN in einem Verzeichnis, das jedes Sync- und Sicherungswerkzeug mitnimmt,
    das auf das Projekt zeigt, und das bei jedem Kopieren des Ordners mitwandert. Vor git
    geschützt waren sie (`.gitignore:36`), vor dem Rest nicht.

    Jetzt nach XDG-Konvention unter `$XDG_DATA_HOME/taxgraph` bzw. `~/.local/share/taxgraph` —
    dorthin zeigt kein Projekt-Werkzeug, und ein `rm -rf` im Projektverzeichnis trifft sie nicht
    mehr. `$TAXGRAPH_DATEN` überschreibt beides (Tests setzen stattdessen api.FAELLE direkt).

    NICHT MITGEZOGEN: produkt/auth/users.json. Das sind Zugangsdaten, keine Steuerdaten, und
    ein Umzug bräche eine bestehende Anmeldung. Eigene Entscheidung, eigener Schritt.

    KEINE VERSCHLÜSSELUNG: die Dateien liegen weiterhin im Klartext (0600). Der Umzug schliesst
    die Sicherungs- und Weitergabe-Lücke, nicht die gestohlene Platte — das war die bewusste
    Wahl (Audit verschluesselung-steuerdaten-im-klartext, Entscheidung 2026-08-19)."""
    eigen = os.environ.get("TAXGRAPH_DATEN", "").strip()
    if eigen:
        return os.path.expanduser(eigen)
    xdg = os.environ.get("XDG_DATA_HOME", "").strip()
    basis = os.path.expanduser(xdg) if xdg else os.path.join(os.path.expanduser("~"),
                                                             ".local", "share")
    return os.path.join(basis, "taxgraph")


FAELLE = os.path.join(_daten_wurzel(), "faelle")

# Der alte Ort — nur noch, damit der Umzugs-Hinweis in server.py darauf zeigen kann und ein
# Test belegt, dass wir NICHT mehr dorthin schreiben.
FAELLE_ALT = os.path.join(HERE, "faelle")

# ========== Private (führender Unterstrich) Konstanten — müssen in __all__ sein für import * ==========
_FALL_RE = re.compile(r"[A-Za-z0-9_-]{1,64}")
_ERLAUBTE_ZUSTAENDE = {"vorlaeufig", "bestaetigt"}

# ========== § 19 Einkünfte (Arbeit) ==========
EP_FELDER = ("ep_arbeitstage", "ep_entfernung_km", "ep_oepnv_kosten", "ep_eigenes_kfz")
# Formalien ohne Rechenwirkung (2026-08-19) — bewusst NICHT in EP_FELDER: dieses Tupel ist
# zugleich der Teil-Ring ("ep_werbungskosten", "abziehbarer_betrag", EP_FELDER), s. Scheibe "ep".
# Ein Formalienfeld darin wäre eine Eingabe des Teil-Rings, obwohl es keinen Betrag berührt.
# Dieselbe Trennung wie bei DHF_FORMALIEN und VV_ANLAGE_FORMALIEN.
EP_FORMALIEN = ("ep_ziel_des_weges", "ep_ziel_adresse")

# ========== an_gesamt MVP + Flags ==========
AN_GESAMT_FLAGS = ("kein_gewinn", "kein_kap", "kein_vuv", "kein_sonstige")
# Screening-Flag "Hast du Kinder?" (2026-08-15). BEWUSST NICHT in AN_GESAMT_FLAGS: das Tupel
# fliesst in RENTNER_KEGEL und in den an_gesamt-Kegel, und ein Screening-Flag im Kegel machte
# jede Zahl von seiner Beantwortung abhaengig. Es steuert nur, welche Fragen kommen.
KIND_SCREENING = ("kein_kind",)

# Screening der AUSGABENSEITE (2026-08-21). Dieselbe Bauart und dieselbe Auflage wie KIND_SCREENING
# direkt darüber: NICHT in einen Kegel — ein Screening-Flag im Kegel machte jede Zahl von seiner
# Beantwortung abhängig, obwohl es nur steuert, welche Fragen kommen.
#
# WARUM DIESE ZEILE ÜBERHAUPT NÖTIG IST, und warum sie die eigentliche Falle war: SCHEIBEN["gesamt"]
# führt eine HANDGESCHRIEBENE Feldliste. Ein Feld, das in der Bindung steht und hier nicht, wird
# nie gefragt — das Flag existiert, wird nie beantwortet, schaltet nie etwas ab. Tote Verdrahtung,
# dieselbe Klasse wie der § 35c-Teil-Ring. Gate dagegen:
# tests/test_screening_ausgabenseite.py::test_flags_haengen_in_der_scheibe_gesamt.
#
# Gemessen am 2026-08-21 im echten Nutzerlauf: ein Arbeitnehmer (einzel, Bruttolohn, alle vier
# Einkunftsarten verneint, keine Kinder) bekam 115 Fragen — darunter 17 zum Unterhalt an eine
# bedürftige Person, 12 zur energetischen Sanierung und 5 zu Versorgungsbezügen. Sachverhalte,
# die er mit je EINER Antwort ausschliessen kann.
AUSGABEN_SCREENING = ("kein_unterhalt", "keine_auslandseinkuenfte", "keine_behinderung_pflege",
                      "keine_versorgungsbezuege", "keine_energetische_sanierung",
                      # 2026-08-26: sechs Themen, die bis dahin KEINE einzige Frage nach
                      # ihrer Existenz hatten — Julius bekam sie alle ungefiltert
                      # („es war von keiner photovoltaikanlage die rede", „es war von
                      # keinem arbeitsmittel die rede", „es wird anlasslos von einem ex
                      # ehepartner ausgegangen"). 39 Fragen weniger, wenn alle verneint.
                      "keine_arbeitsmittel",
                      "kein_realsplitting",
                      "keine_spenden",
                      "keine_berufsausbildung",
                      "kein_verlustvortrag",
                      "keine_lohnersatzleistungen",
                      "keine_zweitwohnung",
                      "vpf_auswaertige_taetigkeit")

# Zaehlfelder der Instanz-Gruppen (2026-08-27). KEINE Screening-Flags, auch wenn sie daneben
# stehen: sie sind `int`, schalten nichts ab und tragen keine Umkehr — sie erheben, WIE OFT
# ein Thema vorkommt. Zwei vermietete Wohnungen, drei Handwerkerrechnungen, zwei Renten.
# Ohne sie baute die Oberflaeche je Angabe genau EIN Eingabefeld, und alles Weitere fiel
# unter den Tisch (bis 2026-08-25 galt das auch fuer Kinder).
INSTANZ_ZAEHLFELDER = (
    "vv_anzahl_objekte",
    "rentner_anzahl_renten",
    "p23_anzahl_verkaeufe",
    "hh_anzahl_handwerker",
    "hh_anzahl_dienstleistungen",
    "hh_anzahl_minijobs",
    "gwg_anzahl",
)
# Dieselben Existenzfragen fuer den PARTNER (2026-08-28). Eigene Kreuze, nicht die vorhandenen
# mitbenutzt — und das ist keine Geschmacksfrage, sondern gemessen:
#
# Neun Partner-Felder hingen an einem ICH-Kreuz. `keine_behinderung_pflege` fragt woertlich
# „Hast du selbst oder hat eines deiner Kinder eine amtlich festgestellte Behinderung…?" — der
# Partner kommt darin nicht vor. Wer nur einen behinderten PARTNER hat, antwortete
# wahrheitsgemaess „nein" und verlor dessen Pauschbetrag fuer immer. Ueber den echten Rechenweg
# gemessen (rentner_gesamt, VZ 2025, zusammen, Partner-GdB 50): 5.532 EUR gegen 5.834 EUR,
# also 302 EUR ZU VIEL STEUER.
#
# Die Bindung formuliert das Prinzip drei Zeilen ueber der Stelle, an der sie es brach: „ein Flag
# darf nur abschalten, wonach es auch gefragt hat." Sachlich dahinter § 26b EStG: die Einkuenfte
# werden zusammengerechnet, aber je Person getrennt ERMITTELT (eigene Anlage N/KAP). Die Existenz
# eines Sachverhalts ist personenbezogen; ein Kreuz, dessen Text „du" sagt, kann nicht fuer zwei
# Personen sprechen.
#
# Sie erscheinen nur bei Zusammenveranlagung (feld_bedingung auf `veranlagung`) — ein
# Alleinstehender sieht sie nie. Gemessen: 88 -> 71 Fragen in `gesamt`, 76 -> 54 in
# `rentner_gesamt`, wenn das Paar auch sie verneint.
PARTNER_SCREENING = (
    "kein_kap_partner",
    "kein_gewinn_partner",
    "kein_sonstige_partner",
    "keine_behinderung_pflege_partner",
)
# person_b_idnr NICHT hier: ERiC lehnt E0100082 amtlich ab (rc=610301106, "Eingefuegt-Kennzeichen
# J/P"), unabhaengig vom Wert — Feld wird nicht mehr deklariert (elster_kz: null in
# bindung_an_gesamt.yaml), darf also auch keinen Ehepaar-Bescheid mehr blockieren. Gemessen
# 2026-08-12, scripts/measure_person_b_idnr.py, BACKLOG person-b-idnr-wird-abgelehnt.
AN_GESAMT_PARTNER = ("bruttoarbeitslohn_partner",)

# ========== § 9 Arbeitsmittel (GWG) + § 7 Abs. 1 AfA ==========
ARBEITSMITTEL_KOSTEN = "am_anschaffungskosten"
ARBEITSMITTEL_RING = ("am_anschaffungskosten", "am_gwg_sofortabzug_gewaehlt", "arbeitsmittel_nutzungsdauer")
# § 7 Abs. 1 S. 4 Zwölftelung im Anschaffungsjahr — NUR im gefalteten gesamt-Ring. an_gesamt
# rechnet über catala_est ohne § 2-Gesamt-Scope und ist über die Oberfläche ohnehin nicht
# wählbar (index.html bietet gesamt + rentner_gesamt); dort würden die Felder nur den
# Fragenkegel aufblähen.
ARBEITSMITTEL_AFA_GESAMT = ("am_anschaffung_monat", "am_afa_ist_anschaffungsjahr")

# ========== § 36 Abs. 2 Anrechnung (LSt + Vorauszahlungen) ==========
P36_ANRECHNUNG = ("p36_lohnsteuer", "p36_vorauszahlungen")
# p36_lohnsteuer_partner NICHT in P36_ANRECHNUNG (das haengt an RENTNER_FELDER, s.u.): ein
# Rentner-Partner-Lohn-Feld ohne begleitendes bruttoarbeitslohn_partner (das rentner_gesamt
# bewusst nicht fuehrt, s. AN_GESAMT_PARTNER/GESAMT_PARTNER_19) waere dort eine Einzel-Naht.
# Eigene Tuple, nur in an_gesamt/gesamt verdrahtet (Julius-Entscheidung 2026-08-10).
P36_ANRECHNUNG_PARTNER = ("p36_lohnsteuer_partner",)

# ========== § 22 Sonstige Einkünfte + § 10 KiSt ==========
P22_NR3_EINKUENFTE = ("p22_nr3_einkuenfte",
                      # Bruttoeinnahmen (E0305101) — ohne sie weist ERiC ab. Eigenes askable
                      # Feld statt Kopie der Einkünfte: das Feld daneben fragt netto, wir kennen
                      # den Bruttowert also nicht und würden sonst Werbungskosten von null
                      # behaupten (2026-08-19).
                      "p22_nr3_einnahmen",
                      # berechnet in bescheid_deklaration: Einzelposten = Summe,
                      # Werbungskosten = Einnahmen minus Einkuenfte. ERiC rechnet die
                      # Probe nach, ein leeres WK-Feld erfuellt sie nicht.
                      "p22_nr3_einnahmen_art", "p22_nr3_einnahmen_einzelbetrag", "p22_nr3_werbungskosten")
KIST_KONFESSION_FELDER = ("kist_konfession", "kist_bundesland")
KIRCHENSTEUER_ARBEITGEBER_FELDER = ("kirchensteuer_arbeitgeber", "kirchensteuer_arbeitgeber_partner")

# ========== § 38b EStG Steuerklasse (Anlage N, E0200002) ==========
STEUERKLASSE_FELDER = ("steuerklasse", "steuerklasse_partner")

# ========== § 16 Abs. 4 Freibetrag-Gates (Rentner) ==========
P16_4_GATE_FELDER = ("rentner_alter_55_oder_berufsunfaehig", "rentner_freibetrag_erstmalig")
# Person-B (Deklaration, Task Gewinneinkünfte-Partnerseite Stufe 1): der zweite Freibetrags-Aufruf +
# die gespiegelte p16_4_gate_offen-Sperre sind Stufe 2 (Ring, api.py), noch nicht gebaut.
P16_4_GATE_FELDER_PARTNER = ("rentner_alter_55_oder_berufsunfaehig_partner", "rentner_freibetrag_erstmalig_partner")

# ========== § 9 Abs. 4a Verpflegung + Mahlzeitenkürzung ==========
VERPFLEGUNG_TAGE = ("tage_24h", "tage_an_abreise", "tage_ueber_8h_eintaegig")
VERPFLEGUNG_TAGE_NACH_FRIST = ("vpf_tage_24h_nach_drei_monaten", "vpf_tage_an_abreise_nach_drei_monaten",
                                "vpf_tage_ueber_8h_nach_drei_monaten")
VERPFLEGUNG_GUARD = ("vpf_monate_am_ort", "vpf_keine_mahlzeitengestellung")
VERPFLEGUNG_KUERZUNG = ("vpf_fruehstuecke_gestellt_anzahl", "vpf_mittagessen_gestellt_anzahl",
                        "vpf_abendessen_gestellt_anzahl", "vpf_mahlzeiten_gezahltes_entgelt",
                        "vpf_steuerfreie_erstattung_betrag",
                        # Das BERECHNETE Ergebnis (askable=false, Kz E0205508) muss mit in die
                        # Scheibe, sonst filtert _scheibe_bindung() es aus der Deklaration —
                        # der Ring kuerzt dann, das XML zeigt aber nur die vollen Tage-Kz
                        # (E0205409/E0205302/E0205201) und die Erklaerung faellt ZU HOCH aus.
                        # Gemessen 2026-08-10 auf 'gesamt': Ring rechnet 8400 Cent Kuerzung,
                        # deklariert 0. n_vor_gwg war korrekt, weil es seine Felder ueber
                        # felder_datei aus der YAML zieht statt aus diesen Tupeln — genau die
                        # Naht, an der P9.4a (826bfdf) halb fertig blieb.
                        "p9_4a_kuerzung_nach_entgelt")
VERPFLEGUNG_FRIST = ("vpf_frist_nicht_unterbrochen",)

# ========== § 10 Vorsorge ==========
VOR_FELDER = ("vor_an_anteil_rv", "vor_ag_anteil_rv", "vor_rv_ausserhalb_lstb")
VOR_PARTNER_FELDER = ("vor_an_anteil_rv_partner", "vor_ag_anteil_rv_partner",
                      "vor_rv_ausserhalb_lstb_partner")
KV_PV_FELDER = ("versicherungsart", "basis_kv", "basis_pv",
                "vorsorge_arbeitslosenversicherung", "vorsorge_erwerbsunfaehigkeit",
                "vorsorge_unfall_haftpflicht", "vorsorge_rv_alt_mit_ueberschuss",
                "vorsorge_rv_alt_ohne_ueberschuss", "mit_anspruch_auf_zuschuss")
KV_PV_PARTNER_FELDER = ("versicherungsart_partner", "basis_kv_partner", "basis_pv_partner",
                        "vorsorge_arbeitslosenversicherung_partner",
                        "vorsorge_erwerbsunfaehigkeit_partner",
                        "vorsorge_unfall_haftpflicht_partner",
                        "vorsorge_rv_alt_mit_ueberschuss_partner",
                        "vorsorge_rv_alt_ohne_ueberschuss_partner",
                        "mit_anspruch_auf_zuschuss_partner")
VORSORGE_PARTNER_FELDER = VOR_PARTNER_FELDER + KV_PV_PARTNER_FELDER + ("geburtsjahr_partner",)

# ========== § 9 Abs. 1 S. 3 Nr. 5 Doppelte Haushaltsführung ==========
DHF_KOSTEN = "dhf_unterkunftskosten_monat"
DHF_RING = ("dhf_unterkunftskosten_monat", "dhf_monate", "dhf_im_inland")
DHF_BEDINGUNGEN = ("dhf_beruflich_veranlasst", "dhf_eigener_hausstand",
                   "dhf_finanzielle_beteiligung")
# dhf_keine_pflicht_dienstwohnung stand bis 2026-08-20 in DHF_BEDINGUNGEN und war damit
# Voraussetzung des ganzen Abzugs — genau der Fehler, vor dem der Kommentar unten warnt. Die Norm
# sagt über die Dienstwohnung nur eines (§ 9 Abs. 1 S. 3 Nr. 5 S. 4 Hs. 2): "die Grenze von
# 2 000 Euro bei einer Unterkunft im AUSLAND gilt nicht, wenn eine Dienst- oder Werkswohnung
# verpflichtend und zweckgebunden genutzt werden muss". Sie HEBT eine Obergrenze AUF, ist also
# für den Steuerpflichtigen günstig, und sie betrifft ausschliesslich den Auslandsfall.
#
# Als Abzugsvoraussetzung wirkte sie doppelt falsch: sie schloss aus statt zu erweitern, und sie
# tat es auch im INLAND, wo die Norm sie gar nicht erwähnt. Gemessen 2026-08-20: wer im Inland
# eine Werkswohnung hat (1.000 EUR Miete, 12 Monate) und wahrheitsgemäss "ja" antwortete, zahlte
# 3.178,00 EUR mehr Steuer.
#
# Eigenes Tupel, weil das Feld weiterhin GEFRAGT werden muss: der Vordruck verlangt die Angabe,
# und sobald der Auslandsfall ring-fähig wird (heute Sperrgrund ausland_dhf_nicht_ring_faehig),
# steuert sie dort cap_monat_ausland. Nicht im Kegel — solange Ausland gesperrt ist, hat sie keine
# Rechenwirkung, und ein Kegel-Eintrag würde die Zahl blockieren, bis jemand sie beantwortet.
DHF_AUSLANDSGRENZE = ("dhf_keine_pflicht_dienstwohnung",)
# Formalien ohne Rechenwirkung — sie ändern keinen Betrag, aber ohne sie weist das Finanzamt
# die Erklärung zurück (gemessen 2026-08-16 und -19, zwei Schichten bis rc=0). Bewusst ein
# eigenes Tupel statt an DHF_RING/DHF_BEDINGUNGEN angehängt: DHF_RING geht in die Berechnung,
# DHF_BEDINGUNGEN gated in bescheid_zweige.py den ganzen Werbungskostenabzug. Ein Formalienfeld
# in einem dieser beiden Tupel würde entweder mitgerechnet oder zur Abzugsvoraussetzung — beides
# falsch, und Letzteres ist genau die Gate-Polaritätsfalle, die hier schon dreimal Geld gekostet
# hat (zuletzt vpf, wo der ganze Verpflegungsmehraufwand verschwand).
DHF_FORMALIEN = ("dhf_beschaeftigungsort", "dhf_grund", "dhf_begruendet_am", "dhf_bestanden_bis",
                 "dhf_hausstand_plz_ort", "dhf_hausstand_seit")

# ========== § 9 Abs. 1 S. 3 Nr. 5a Übernachtung ==========
UEBERNACHTUNG_KOSTEN = "uebernachtung_kosten_monat"
UEBERNACHTUNG_RING = ("uebernachtung_kosten_monat", "uebernachtung_monate",
                      "uebernachtung_monate_bisher", "uebernachtung_im_inland")
UEBERNACHTUNG_BEDINGUNGEN = ("uebernachtung_auswaerts", "uebernachtung_alleinnutzung",
                             "uebernachtung_keine_lange_unterbrechung")

# ========== § 21 Vermietung ==========
VV_GESAMT_FELDER = ("vv_einnahmen", "vv_gebaeude_afa", "vv_schuldzinsen",
                    "vv_erhaltungsaufwand", "vv_sonstige_wk", "vv_entgelt_quote_prozent")
# Anlage V — Objekt- und Einnahmen-Formalien (2026-08-16). BEWUSST getrennt von
# VV_GESAMT_FELDER: das Tupel fliesst in den gesamt-Kegel, und eine Adresse ist keine
# Voraussetzung dafuer, dass eine Zahl herauskommt — sie ist Voraussetzung fuers Einreichen.
VV_ANLAGE_FORMALIEN = ("vv_objekt_strasse", "vv_objekt_plz", "vv_objekt_ort",
                       "vv_wohneinheit_bezeichnung", "vv_nebenkosten_nicht_vereinbart",
                       "vv_nebenkosten_umgelegt", "vv_mieteinnahmen_summe",
                       "vv_nutzung_ferienwohnung", "vv_nutzung_an_angehoerige",
                       "vv_nutzung_kurzfristig", "vv_einnahmen_summe_gesamt",
                       "vv_summe_werbungskosten", "vv_ueberschuss", "vv_ueberschuss_person_a")
VV_ABS2_TATBESTAND = ("vv_wohnzwecke", "vv_auf_dauer")

# ========== § 20 Kapital ==========
KAP_ERTRAEGE = "kap_kapitalertraege"
KAP_TOEPFE = ("kap_gewinn_aktien", "kap_verlust_aktien", "kap_gewinn_sonstige", "kap_verlust_sonstige")
KAP_FELDER = (KAP_ERTRAEGE,) + KAP_TOEPFE   # Veranlagungsart kommt aus `veranlagung`, nicht aus einem KAP-eigenen Feld
KAP_ERTRAEGE_PARTNER = "kap_kapitalertraege_partner"
KAP_TOEPFE_PARTNER = ("kap_gewinn_aktien_partner", "kap_gewinn_sonstige_partner",
                      "kap_verlust_aktien_partner", "kap_verlust_sonstige_partner")
GESAMT_PARTNER_KAP = (KAP_ERTRAEGE_PARTNER,) + KAP_TOEPFE_PARTNER
# BERECHNETE Anlage-KAP-Antragsfelder (askable=false, § 32d Abs. 6 E1900401 + § 20 Abs. 9
# E1901401, injiziert von api._mit_ring_werten). Muessen mit in die Scheibe, sonst filtert
# _scheibe_bindung() sie aus der Deklaration -- dieselbe Naht wie VERPFLEGUNG_KUERZUNG/
# E0205508 oben: der Ring liefert den Wert, das XML wuerde ihn sonst stillschweigend verschweigen.
KAP_ANTRAG_FELDER = ("kap_antrag_guenstigerpruefung", "kap_sparer_pauschbetrag_genutzt")
# § 36 Abs. 2 S. 1 Nr. 2 EStG Anrechnung (Zeilen 37-39 Anlage KAP, E1904701/E1904901/E1904801) —
# ASKABLE (Steuerbescheinigung-Werte, anders als KAP_ANTRAG_FELDER oben, das aus dem Ring
# injiziert wird). Stufe 2 (BAU-GO team-lead 2026-08-10, spiegelt P36_ANRECHNUNG-Wiring oben).
# Nur in gesamt/rentner_gesamt verdrahtet (an_gesamt fuehrt kein KAP_FELDER, s.u.).
P36_ANRECHNUNG_KAP = ("p36_kapitalertragsteuer", "p36_kapitalertragsteuer_solz", "p36_kapitalertragsteuer_kist")
# KAP Stufe 3 (Zeile 41 Anlage KAP, E1905101, § 32d Abs. 1 S. 2/4-5): noch nicht angerechnete
# auslaendische Quellensteuer (q). ASKABLE, wie P36_ANRECHNUNG_KAP oben (Steuerbescheinigung-Wert).
P32D_Q_KAP = ("kap_q_auslaendische_steuer",)
# person_b_idnr NICHT hier: s. Kommentar bei AN_GESAMT_PARTNER oben (ERiC-Ablehnung E0100082,
# nicht mehr deklariert, darf nicht mehr sperren).
GESAMT_PARTNER_19 = ("bruttoarbeitslohn_partner",)

# ========== § 22 Renten + § 33b Pauschbeträge ==========
RENTNER_AA_ARTEN = ("gesetzliche_rente", "berufsstaendische_versorgung", "private_basisrente")
RENTNER_22 = ("rentner_renten_art", "rentner_jahresrente", "rentner_renten_beginn_jahr",
              "rentner_alter_bei_rentenbeginn")
RENTNER_33B = ("rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind", "rentner_pflegegrad",
               "rentner_gepflegter_hilflos", "rentner_hinterbliebenenbezuege")
# Die fünf Pflichtangaben der Anlage (IdNr, Personendaten, Wohnsitz, "durch wen", Helferzahl)
# gehören BEWUSST NICHT hierher: RENTNER_33B fließt in RENTNER_KEGEL, und der Kegel ist die
# Liste der Felder, ohne die es KEINE Zahl gibt. Gemessen 2026-08-15: einmal dort eingetragen,
# lieferte jeder Rentner-Fall nur noch input_kegel_nicht_bestaetigt — 74 rote Tests. Sie sind
# Deklarations-Pflicht gegenüber dem Finanzamt, nicht Rechen-Voraussetzung; sie hängen deshalb
# unten an RENTNER_FELDER (Scheibe), wie rentner_rentenfreibetrag.
RENTNER_33B_PFLEGE_ANGABEN = ("rentner_gepflegter_wohnsitz_inland", "rentner_pflege_durch",
                              "rentner_gepflegter_idnr", "rentner_gepflegter_angaben",
                              "rentner_pflege_weitere_personen")
RENTNER_PARTNER = ("rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner")
RENTNER_22_PARTNER = ("rentner_renten_art_partner", "rentner_jahresrente_partner",
                      "rentner_renten_beginn_jahr_partner", "rentner_alter_bei_rentenbeginn_partner")
GESAMT_33B = ("rentner_grad_der_behinderung", "rentner_hilflos_blind_taubblind",
              "rentner_hinterbliebenenbezuege", "rentner_pflegegrad", "rentner_gepflegter_hilflos",
              "rentner_gepflegter_wohnsitz_inland", "rentner_pflege_durch",
              "rentner_gepflegter_idnr", "rentner_gepflegter_angaben",
              "rentner_pflege_weitere_personen")
GESAMT_33B_PARTNER = ("rentner_grad_der_behinderung_partner", "rentner_hilflos_blind_taubblind_partner")

# ========== §§ 13-18 Gewinn ==========
# EUER_KOMPONENTEN bewusst OHNE Person-B-Pendant: Anlage EÜR ist Datenart E77, ein eigenes Dokument
# ohne Person-A/B-Indexfeld (kein Enum_INDEXFELD_PERSON-Treffer in E77-2025.xsd) — der Partner-Betrieb
# braeuchte ein zweites E77-Dokument, die Mechanik dafuer fehlt (BACKLOG rechenluecken, 2026-08-12).
EUER_KOMPONENTEN = ("betriebseinnahmen", "sonstige_betriebsausgaben", "afa_jahresbetrag")
GWG_FELDER = ("gwg_anschaffungskosten_netto",)
VERLUST_FELD = ("verlustvortrag_bestand",)
MITU_FELDER = ("gewinnanteil", "verguetung_taetigkeit", "verguetung_darlehen", "verguetung_ueberlassung")
# Person-B: gleiche Kz wie Person A (aufgegangen in E0800502-Instanz-B), keine Sondervergütungs-
# Trennung im Schema — s. bindung_an_gesamt.yaml.
MITU_FELDER_PARTNER = ("gewinnanteil_partner", "verguetung_taetigkeit_partner",
                       "verguetung_darlehen_partner", "verguetung_ueberlassung_partner")

# ========== § 34 Abs. 3 Ermäßigter Durchschnittssatz ==========
ABS3_FELDER = ("antrag_ermaessigter_satz", "dauernd_berufsunfaehig", "ermaessigung_einmal_genutzt")

# ========== § 35 GewSt-Anrechnung ==========
# gewst_zu_zahlen/_partner sind berechnet (Messbetrag x Hebesatz, § 16 Abs. 1 GewStG) und
# muessen trotzdem in die Scheibe: sonst filtert _scheibe_bindung() sie aus der Deklaration
# und ERiC beanstandet weiter "die zu zahlende Gewerbesteuer jedoch nicht" (2026-08-19).
GESAMT_P35 = ("gewst_hebesatz", "gewst_messbetrag", "gewst_zu_zahlen")
GESAMT_P35_PARTNER = ("gewst_hebesatz_partner", "gewst_messbetrag_partner",
                      "gewst_zu_zahlen_partner")

# ========== § 19 Abs. 2 Versorgungsfreibetrag ==========
GESAMT_VERSORGUNG = ("versorgung_jahresrente", "versorgung_bemessungsgrundlage",
                     "versorgung_beginn_jahr", "versorgung_art", "versorgung_alter_bei_beginn")

# ========== Rentner: §§ 13-18 Gewinn + Veräußerungs-Gewinn ==========
RENTNER_GEWINN = (("einkuenfte_gewinn", "gewinn_bezeichnung", "rentner_veraeusserungsgewinn",
                   "rentner_veraeusserungs_betriebsart",
                   "gewinn_betriebsart", "geburtsjahr") + EUER_KOMPONENTEN + GWG_FELDER + MITU_FELDER + ABS3_FELDER)

# ========== RENTNER_KEGEL (Pflicht-Felder im Rentner-Ring) ==========
RENTNER_KEGEL = RENTNER_22 + RENTNER_33B + ("veranlagung",) + AN_GESAMT_FLAGS + VOR_FELDER + KV_PV_FELDER

# ========== RENTNER_FELDER — ERSTE DEFINITION (Z.188 api.py) ==========
RENTNER_FELDER = (RENTNER_KEGEL + RENTNER_33B_PFLEGE_ANGABEN
                  + ("rentner_rentenfreibetrag", "rentner_rentenfreibetrag_partner")
                  + RENTNER_PARTNER + RENTNER_22_PARTNER + RENTNER_GEWINN
                  + ("gewst_hebesatz", "gewst_messbetrag") + VERLUST_FELD)

# ========== § 35a Haushaltsnahe ==========
HAUSHALT_35A_ABS23 = ("hh_dienstleistungen", "hh_handwerker_arbeitskosten")
# Einzelaufstellung (Anlass 2026-08-10, checkESt rc=610001002 ohne Einz-Kz): Instanz-Basisfelder
# der drei Töpfe (instanz_gruppe hh_minijob/hh_dienstleistung/hh_handwerker) — dieselbe Kette wie
# die Sum-Felder oben, sonst deklariert _scheibe_bindung sie auf keiner Scheibe.
HAUSHALT_35A_EINZ = ("hh_minijob_betrag", "hh_minijob_art",
                     "hh_dienstleistung_betrag", "hh_dienstleistung_art",
                     "hh_handwerker_betrag", "hh_handwerker_art")
# hh_hat_aufwendungen zuerst: die Ob-Frage vor den Beträgen und Tatbestandsmerkmalen
# (Screening-Gate, 2026-08-14 — bei "nein" schließt relevanz() die ganze § 35a-Regel aus).
HAUSHALT_35A = ("hh_hat_aufwendungen", "hh_minijob_aufwendungen") + HAUSHALT_35A_ABS23 + HAUSHALT_35A_EINZ + ("hh_in_eu_ewr", "hh_handwerker_keine_foerderung")
P35A_MITVER_ANZEIGE = ("p35a_mitveranlagung",)

# ========== § 10b Spenden + § 10 KiSt (Gesamt) ==========
AGB_KIST = ("kist_gezahlt", "kist_erstattet")

# ========== § 10 Abs.1 Nr.5 Kinderbetreuung ==========
KINDERBETREUUNG = ("kinderbetreuungskosten", "kind_unter_14_haushaltszugehoerig",
                    "kind_betreuung_dienstleister", "kind_betreuung_zeitraum",
                    "kind_betreuung_eigenanteil", "kind_betreuung_kein_gemeinsamer_haushalt_zeitraum",
                    "kind_betreuung_haushaltszugehoerigkeit_zeitraum",
                    "kind_betreuung_einzelbetrag", "kind_betreuung_eigenanteil_betrag",
                    "kind_betreuung_eigenanteil_zeitraum")

# ========== § 10 Abs.1 Nr.9 Schulgeld ==========
SCHULGELD = ("schulgeld",)

# ========== § 10 Abs.1 Nr.3 S.2 KV/PV-Beiträge des Kindes ==========
# kind_vorname (Kz E0500107) ist seit 2026-08-11 dabei: ohne ihn lehnt checkESt jede
# Kind-Instanz ab ("Tragen Sie bitte den Vornamen des Kindes ein"), unabhaengig davon,
# welche kindbezogene Position gerade erklaert wird. Er gehoert damit in denselben Kegel
# wie kind_idnr — beide identifizieren die Instanz, keiner geht in eine Rechnung ein.
KIND_KV_PV = ("kind_kv", "kind_pv", "kind_idnr", "kind_vorname",
              # Kindschaftsverhaeltnis (E0500807/808) + Zeitraum (E0500601/805): gebunden seit
              # jeher, aber bis 2026-08-11 in keinem Kegel — checkESt verlangt sie, sobald eine
              # Kind-Instanz entsteht ("Der Vorname des Kindes wurde angegeben, die Angaben zum
              # Kindschaftsverhaeltnis fehlen jedoch").
              "kind_kindschaftsverhaeltnis_a", "kind_kindschaftsverh_zeitraum_a",
              # Geburtsdatum (E0500701, umbenannt/umtypisiert aus kind_geburtsjahr) + Familienkasse
              # (E0500706) + Wohnsitz-Inland-Zeitraum (E0500703): zweite Haelfte des
              # Anlage-Kind-Blockers, gemessen 2026-08-12 — checkESt verlangt Vorname, Geburtsdatum
              # und Familienkasse gemeinsam, sowie die Aufenthaltsdauer-Angabe.
              "kind_geburtsdatum", "kind_familienkasse", "kind_wohnsitz_inland_zeitraum",
              # Kindschaftsverhaeltnis Elternteil B (E0500808/805): bildet den MITERKLAERENDEN
              # Ehegatten ab (K_Verh_B im XSD) — gehoert in den Kegel fuer Zusammenveranlagung,
              # bleibt aber bei Einzelveranlagung ungenutzt (s. naechster Kommentar).
              "kind_kindschaftsverhaeltnis_b", "kind_kindschaftsverh_zeitraum_b",
              # Anderer Elternteil als Drittperson (K_Verh_and_P/Ang_Pers, E0501103/104/106/903):
              # gemessen 2026-08-12 — bei veranlagung=einzel ist kind_kindschaftsverhaeltnis_b
              # (K_Verh_B, der MITERKLAERENDE Ehegatte) laut checkESt UNZULAESSIG ("Es handelt
              # sich um eine Einzelveranlagung, daher sind Angaben zum Kindschaftsverhaeltnis zur
              # Ehefrau nicht zulaessig"); stattdessen verlangt checkESt diese Personenangaben
              # zum tatsaechlich anderen Elternteil.
              "kind_anderer_elternteil_name", "kind_anderer_elternteil_geburtsdatum",
              "kind_anderer_elternteil_kindschaftsverhaeltnis", "kind_anderer_elternteil_zeitraum")

# ========== § 33b Abs.5 Kind-PB-Übertragung ==========
KIND_PB_UEBERTRAGUNG = ("kind_grad_der_behinderung", "kind_hilflos_blind_taubblind",
                        "kind_hinterbliebenen_uebertragung",
                        "kind_behinderten_pb_antrag", "kind_pb_nicht_selbst_genutzt")

# ========== § 33b Abs.5 S.4 agB-Ausschluss bei Kind-PB-Übertragung ==========
BEHINDERUNGSBEDINGTE_AUFWENDUNGEN = ("behinderungsbedingte_aufwendungen",)

# ========== § 33b Abs.1 S.1 Wahlrecht (eigener PB) — Stufe 2b ==========
BEHINDERUNGSBEDINGTE_AUFWENDUNGEN_WAHLRECHT = ("behinderungsbedingte_aufwendungen_wahlrecht_pb",)

# ========== § 33b Abs.1 S.1 Wahlrecht Partner — Stufe 2b-Partner ==========
# Partner-Pauschbetrag lief bisher unconditional neben agb_aufwendungen (BACKLOG
# p33b-partner-pb-doppelabzug, 1.168-1.234 EUR stiller Doppelabzug). § 33b gilt PRO PERSON
# (Abs.1 S.1 Subjekt + Abs.2/3-Anspruch individuell) -- Wahlrecht/Sperrgrund braucht daher
# eine eigene "davon"-Teilmenge fuer den Partner, auf derselben Achse (fallweit) wie das
# Obermengenfeld agb_aufwendungen, NICHT instanz_gruppe.
BEHINDERUNGSBEDINGTE_AUFWENDUNGEN_PARTNER = ("behinderungsbedingte_aufwendungen_partner",)
BEHINDERUNGSBEDINGTE_AUFWENDUNGEN_WAHLRECHT_PARTNER = ("behinderungsbedingte_aufwendungen_wahlrecht_pb_partner",)

# ========== § 33 Abs.2a Fahrtkostenpauschale (Person A) ==========
FAHRTKOSTEN_PAUSCHALE = ("fahrtkosten_pausch_gdb80_oder_70g",
                         "fahrtkosten_pausch_ag_bl_tbl_h")

# ========== Gefaltete Sonder-Abzüge (Weg ii) ==========
GESAMT_ABZUEGE = (HAUSHALT_35A + ("hh_rechnung_unbar", "spenden_betrag", "spenden_vermoegensstock",
                  "agb_aufwendungen", "fam_anzahl_kinder", "berufsausbildung_aufwendungen",
                  # Einzelaufstellung der Berufsausbildung (2026-08-19): ohne sie weist ERiC
                  # ab. _bezeichnung wird gefragt, _einzelbetrag rechnet bescheid_deklaration
                  # aus der Summe. GESAMT_ABZUEGE fließt nur in felder-Listen, nie in einen
                  # Kegel — die zwei blockieren also keine Zahl.
                  "berufsausbildung_bezeichnung", "berufsausbildung_einzelbetrag") + AGB_KIST + KINDERBETREUUNG + SCHULGELD + KIND_KV_PV + KIND_PB_UEBERTRAGUNG + BEHINDERUNGSBEDINGTE_AUFWENDUNGEN + BEHINDERUNGSBEDINGTE_AUFWENDUNGEN_WAHLRECHT + BEHINDERUNGSBEDINGTE_AUFWENDUNGEN_PARTNER + BEHINDERUNGSBEDINGTE_AUFWENDUNGEN_WAHLRECHT_PARTNER + FAHRTKOSTEN_PAUSCHALE)

# ========== RENTNER_FELDER — ZWEITE ÄNDERUNG (Z.218 api.py) ==========
RENTNER_FELDER = RENTNER_FELDER + GESAMT_ABZUEGE

# ========== § 24a/§24b Freibeträge ==========
GESAMT_FREIBETRAEGE = ("geburtsjahr", "fam_alleinstehend", "fam_monate_ohne_voraussetzung") + P35A_MITVER_ANZEIGE

# ========== § 34c DBA-Anrechnung ==========
GESAMT_DBA = ("dba_staat", "dba_methode", "dba_einkunftsart", "dba_mehrere_staaten",
              "dba_gezahlte_auslaendische_steuer", "dba_auslaendische_einkuenfte",
              "dba_abzug_statt_anrechnung")

# ========== DBA-Länder-Methoden-Mapping ==========
# Pauschale Methode je Land (Stufe 1). Gilt, wo DBA_METHOD_MAP_ART keinen Eintrag für die
# konkrete Einkunftsart hat. Kein Abkommen wendet EINE Methode auf alle Einkunftsarten an —
# diese Tabelle ist die grobe Näherung, die per-Einkunftsart-Tabelle darunter verfeinert sie.
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

# Das Feld dba_staat führt deutsche Ländernamen als Enum, die Methoden-Tabellen ISO-Codes.
# Ohne diese Brücke trifft KEIN Enum-Wert die Map und alles fällt auf den Default
# "anrechnung" — für Österreich und die USA (Freistellungs-DBA) wäre das falsch gerechnet.
# Enum-Werte ohne Eintrag (Italien, Tschechien, Kanada, Deutschland, sonstiger_staat) haben
# noch keine adjudizierte Methode und laufen bewusst auf den Anrechnungs-Default.
DBA_STAAT_ISO = {
    "oesterreich": "at",
    "österreich": "at",
    "schweiz": "ch",
    "dänemark": "dk",
    "daenemark": "dk",
    "spanien": "es",
    "frankreich": "fr",
    "grossbritannien": "gb",
    "großbritannien": "gb",
    "luxemburg": "lu",
    "niederlande": "nl",
    "polen": "pl",
    "türkei": "tr",
    "tuerkei": "tr",
    "usa": "us",
}

# Einkunftsarten für das per-Einkunftsart-Routing. Die Namen folgen den Überschriften der
# OECD-Musterabkommens-Verteilungsartikel, weil die DBA-Methodenartikel genau darauf verweisen.
DBA_EINKUNFTSARTEN = (
    "unbewegliches_vermoegen",      # Art. 6
    "unternehmensgewinne",          # Art. 7
    "dividenden",                   # Art. 10
    "zinsen",                       # Art. 11
    "lizenzgebuehren",              # Art. 12
    "veraeusserungsgewinne",        # Art. 13
    "unselbstaendige_arbeit",       # Art. 15
    "aufsichtsratsverguetungen",    # Art. 16
    "kuenstler_sportler",           # Art. 17
    "ruhegehaelter",                # Art. 18
)

# ========== DBA per-Einkunftsart (P7.1) ==========
# {(staat, einkunftsart): methode}. Ein fehlender Eintrag fällt auf DBA_METHOD_MAP zurück,
# damit die Tabelle Land für Land wachsen kann, ohne bestehendes Verhalten zu ändern.
#
# Jeder Eintrag ist am Methodenartikel des jeweiligen Abkommens belegt (Zitatanker in
# produkt/bindung/bindung_p34c_gesamt.yaml). Adjudikation gehört zu Julius/Instructor —
# hier stehen nur Einträge, deren Wortlaut eindeutig ist.
#
# STAND: nur Polen ausgearbeitet (Muster). Die übrigen zehn Länder laufen weiter über
# DBA_METHOD_MAP, bis ihr Methodenartikel einzeln adjudiziert ist.
#
# Polen — Art. 24 Abs. 1 DBA-PL 2003 (sources/dba/dba_pl_abkommen_2003.txt):
#   Buchst. a  Freistellung als Grundregel ("werden … ausgenommen"), vorbehaltlich b
#   Buchst. b  Anrechnung für aa) Dividenden ausserhalb des Schachtelprivilegs und
#              bb) Einkünfte nach Art. 11 Abs. 2, 12 Abs. 2, 13 Abs. 2, 15 Abs. 3, 16 Abs. 1, 17
#   Buchst. c  Rückfall auf Anrechnung für Art. 7 und 10 ohne Aktivitätsnachweis (§ 8 AStG)
#
# NICHT abgebildet (bewusst, je eigener Sachverhalt statt Einkunftsart):
#   - Schachtelprivileg Dividenden (a S. 2: ≥10 % Kapital, keine Personengesellschaft) →
#     hier durchgehend "anrechnung", also die für den Steuerpflichtigen ungünstigere Variante.
#     Fail-closed statt stiller Besserstellung.
#   - Aktivitätsvorbehalt Buchst. c → Art. 7/10 stehen auf der Grundregel; der Rückfall
#     braucht ein eigenes Nachweis-Feld (Stufe 2).
#   - Teil-Absätze (13 Abs. 2, 15 Abs. 3) → die Einkunftsart trägt die Absatz-Ebene nicht.
#     Beide stehen auf der jeweiligen Grundregel; die Absatz-Feinheit ist Stufe 2.
DBA_METHOD_MAP_ART = {
    ("pl", "unbewegliches_vermoegen"): "freistellung",   # Art. 24 Abs. 1 a
    ("pl", "unternehmensgewinne"): "freistellung",       # Art. 24 Abs. 1 a (c-Rückfall = Stufe 2)
    ("pl", "dividenden"): "anrechnung",                  # Art. 24 Abs. 1 b aa
    ("pl", "zinsen"): "anrechnung",                      # Art. 24 Abs. 1 b bb (Art. 11 Abs. 2)
    ("pl", "lizenzgebuehren"): "anrechnung",             # Art. 24 Abs. 1 b bb (Art. 12 Abs. 2)
    ("pl", "veraeusserungsgewinne"): "freistellung",     # Art. 24 Abs. 1 a (nur Abs. 2 → b bb)
    ("pl", "unselbstaendige_arbeit"): "freistellung",    # Art. 24 Abs. 1 a (nur Abs. 3 → b bb)
    ("pl", "aufsichtsratsverguetungen"): "anrechnung",   # Art. 24 Abs. 1 b bb (Art. 16 Abs. 1)
    ("pl", "kuenstler_sportler"): "anrechnung",          # Art. 24 Abs. 1 b bb (Art. 17)
    ("pl", "ruhegehaelter"): "freistellung",             # Art. 24 Abs. 1 a
}


def dba_staat_iso(staat: str | None) -> str:
    """Enum-Wert des Feldes dba_staat → ISO-Code der Methoden-Tabellen.

    Nimmt sowohl den deutschen Namen ("Polen") als auch den ISO-Code ("pl") entgegen,
    damit Aufrufer beides übergeben können. Unbekanntes bleibt unverändert und läuft
    damit in den Anrechnungs-Default.
    """
    if not staat:
        return ""
    s = staat.strip().lower()
    return DBA_STAAT_ISO.get(s, s)


def dba_methode_fuer(staat: str | None, einkunftsart: str | None = None) -> str:
    """Methode zur Vermeidung der Doppelbesteuerung für (Staat, Einkunftsart).

    Reihenfolge: per-Einkunftsart-Eintrag → pauschale Länder-Methode → "anrechnung".
    Der Default ist Anrechnung, weil sie ohne Abkommensgrundlage der gesetzliche
    Regelfall ist (§ 34c Abs. 1 EStG) und niemanden stillschweigend besserstellt.
    """
    s = dba_staat_iso(staat)
    if not s:
        return "anrechnung"
    if einkunftsart:
        treffer = DBA_METHOD_MAP_ART.get((s, einkunftsart.strip().lower()))
        if treffer:
            return treffer
    return DBA_METHOD_MAP.get(s) or "anrechnung"

# ========== § 23 Private Veräußerungsgeschäfte ==========
GESAMT_P23 = ("p23_veraeusserungspreis", "p23_anschaffung_herstellungskosten",
              "p23_werbungskosten", "p23_veraeusserungs_typ")

# ========== § 33a Unterhalt + Ausbildungsfreibetrag ==========
GESAMT_P33A = ("p33a_unterhalt_aufwendungen", "p33a_unterhalt_kv_pv",
               "p33a_andere_einkuenfte_bezuege", "p33a_ausbildung_anzahl_kinder",
               # Angaben zur unterstützten Person und ihrem Haushalt (2026-08-19). Fünf
               # Beanstandungen auf einmal ohne sie — die größte der acht Lücken. Sie ändern
               # keinen Betrag, erscheinen aber nur, wenn überhaupt Unterhalt erklärt wird.
               "p33a_person_name", "p33a_person_beruf_familienstand",
               "p33a_person_geburtsdatum", "p33a_haushalt_anschrift",
               "p33a_haushalt_personenzahl", "p33a_unterstuetzungszeitraum",
               "p33a_zahlungszeitraum",
               # Zweite Schicht (2026-08-19): erst sichtbar, nachdem die sieben oben
               # beantwortet waren. Fuenf Ja/Nein-Aussagen, das Verwandtschaftsverhaeltnis
               # und die IdNr der unterstuetzten Person — Letztere ist nach § 33a Abs. 1
               # S. 9 Voraussetzung fuer den Abzug, nicht Beiwerk.
               "p33a_person_hat_einkuenfte", "p33a_person_hat_vermoegen",
               "p33a_weitere_person_beteiligt", "p33a_person_im_inlaendischen_haushalt",
               "p33a_kindergeld_anspruch", "p33a_verwandtschaftsverhaeltnis",
               "p33a_person_idnr")

# ========== § 32b Progressionsvorbehalt ==========
GESAMT_P32B = ("p32b_progressionseinkuenfte",)

# ========== § 35c Energetische Sanierung ==========
GESAMT_P35C = ("p35c_sanierungsaufwendungen", "p35c_ist_uebernaechstes_foerderjahr",
                "p35c_keine_doppelfoerderung",
                # Anlage Energetische Maßnahmen — die Formalien, ohne die ERiC ablehnt
                # (gemessen 2026-08-16, sieben Beanstandungen).
                "p35c_objekt_strasse", "p35c_objekt_plz_ort",
                "p35c_gebaeude_herstellungsbeginn", "p35c_baubeginn_massnahme",
                "p35c_gesamtflaeche_qm", "p35c_eigene_wohnflaeche_qm",
                "p35c_bereits_ermaessigung_frueher",
                "p35c_foerderung_in_anspruch",
                "p35c_massnahme_art", "p35c_massnahme_einzelbetrag",
               "p35c_energieberater_aufwendungen")

# ========== § 10 Abs.1a Nr.1 Realsplitting ==========
GESAMT_REALSPLITTING = ("realsplitting_unterhaltsleistungen", "realsplitting_empfaenger_kv_pv",
                        "realsplitting_empfaenger_kv_krankengeld", "realsplitting_zustimmung")

# ========== § 21 Veräußerungs-Gewinn (Gesamt) ==========
GESAMT_VG = ("rentner_veraeusserungsgewinn", "rentner_veraeusserungs_betriebsart")
GESAMT_VG_PARTNER = ("rentner_veraeusserungsgewinn_partner", "rentner_veraeusserungs_betriebsart_partner")

# ========== § 3 Nr. 72 Photovoltaik (steuerfreie Einnahmen) ==========
GESAMT_PV = ("pv_einnahmen", "pv_bruttoleistung_kwp", "pv_anzahl_einheiten", "pv_auf_gebaeude")

# ========== § 150 Abs. 2 AO Stammdaten (ESt1A/Allg/A + BV) ==========
# stammdaten_steuernummer traegt kein Kz (Vorsatz-Block, kein E10-Element) — steht trotzdem
# hier, gleiche Stelle wie die uebrigen Stammdaten (s. produkt/bindung/bindung_an_gesamt.yaml).
STAMMDATEN_FELDER = ("stammdaten_nachname", "stammdaten_vorname", "stammdaten_geburtsdatum",
                     "stammdaten_strasse", "stammdaten_hausnummer", "stammdaten_plz", "stammdaten_wohnort",
                     "stammdaten_keine_bankverbindung", "stammdaten_iban", "stammdaten_bic",
                     "stammdaten_art_est_erklaerung", "stammdaten_steuernummer")
# ========== § 150 Abs. 2 AO Stammdaten Person B (ESt1A/Allg/B, nur Zusammenveranlagung) ==========
STAMMDATEN_FELDER_PARTNER = ("stammdaten_nachname_partner", "stammdaten_vorname_partner",
                             "stammdaten_geburtsdatum_partner", "kist_konfession_partner")

# ========== § 2 Gewinn (Gesamt) ==========
GESAMT_GEWINN = ("einkuenfte_gewinn", "gewinn_betriebsart", "gewinn_bezeichnung") + EUER_KOMPONENTEN + GWG_FELDER + GESAMT_VG + GESAMT_P35 + VERLUST_FELD + MITU_FELDER + ABS3_FELDER + GESAMT_PV
# Person-B (Task Gewinneinkünfte-Partnerseite Stufe 1, Deklaration): EÜR-Komponenten und §34 Abs.3-
# Flags bewusst NICHT dupliziert (s. EUER_KOMPONENTEN-Kommentar bzw. ABS3_FELDER; §34-Fixes laufen an
# Person A). GWG_FELDER/VERLUST_FELD sind global (nicht person-individuell), kein Pendant nötig.
GESAMT_GEWINN_PARTNER = (("einkuenfte_gewinn_partner", "gewinn_betriebsart_partner",
                          "gewinn_bezeichnung_partner")
                         + GESAMT_VG_PARTNER + GESAMT_P35_PARTNER + MITU_FELDER_PARTNER)

# ========== RENTNER_FELDER — DRITTE ÄNDERUNG (Z.282 api.py) ==========
RENTNER_FELDER = RENTNER_FELDER + GESAMT_FREIBETRAEGE + GESAMT_DBA + GESAMT_P23 + GESAMT_P33A + GESAMT_P32B + GESAMT_P35C + GESAMT_REALSPLITTING + P36_ANRECHNUNG + KIST_KONFESSION_FELDER + P22_NR3_EINKUENFTE + P16_4_GATE_FELDER + KV_PV_PARTNER_FELDER + VOR_PARTNER_FELDER + STAMMDATEN_FELDER + STAMMDATEN_FELDER_PARTNER

# ========== RENTNER_FELDER — VIERTE ÄNDERUNG (Gewinneinkünfte Person-B, Stufe 1 Deklaration) ==========
RENTNER_FELDER = RENTNER_FELDER + GESAMT_GEWINN_PARTNER + P16_4_GATE_FELDER_PARTNER

# ========== Scheiben-Konfiguration ==========
SCHEIBEN = {
    "ep": {
        "felder": EP_FELDER + EP_FORMALIEN, "felder_datei": None,
        # Expliziter Kegel, seit die Formalien dazukamen (2026-08-19). Ohne ihn faellt
        # api.py:478 auf ALLE Scheiben-Felder zurueck ("kegel" or _scheibe_felder) — und dann
        # blockieren Ziel/Zieladresse die Zahl, obwohl sie keinen Betrag beruehren. Gemessen:
        # das Intervall kam als min_cent=None/max_cent=None zurueck, der Nutzer haette auf eine
        # Adresse gewartet, um eine Entfernungspauschale zu sehen. Die anderen Scheiben hatten
        # den Fehler nicht, weil sie ihren Kegel ohnehin ausschreiben.
        "kegel": EP_FELDER,
        "gesamt_ring": "abziehbarer_betrag",
        "teil_ringe": [],
    },
    "n_vor_gwg": {
        "felder": None, "felder_datei": "bindung_n_vor_gwg.yaml",
        "gesamt_ring": None,
        # ("arbeitsmittel_afa", "am_afa_betrag", ARBEITSMITTEL_RING) gestrichen (Julius-Entscheid
        # 2026-08-14): _bescheid_fn hatte nie einen am_afa_betrag-Zweig, der Aufruf fiel ins
        # abschliessende `return None` und /stand meldete für diesen Teil-Ring dauerhaft
        # engine_unavailable. Kein Geldfehler — die § 7-AfA wird in BEIDEN echten Bescheid-Pfaden
        # gerechnet (api.py, ns_wk += am_afa_betrag). Einen Zweig zu bauen hätte eine ZWEITE
        # Rechenquelle für dieselbe AfA geschaffen; genau diese Naht hat hier schon dreimal Geld
        # gekostet. Deshalb der Eintrag weg statt eines zweiten Wegs.
        "teil_ringe": [("ep_werbungskosten", "abziehbarer_betrag", EP_FELDER)],
        "guard": False,
    },
    "an_gesamt": {
        "felder": (("bruttoarbeitslohn", "veranlagung") + EP_FELDER + EP_FORMALIEN + VOR_FELDER + KV_PV_FELDER
                   + DHF_RING + DHF_BEDINGUNGEN + DHF_AUSLANDSGRENZE + DHF_FORMALIEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_TAGE_NACH_FRIST + VERPFLEGUNG_GUARD + VERPFLEGUNG_FRIST
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
                   + EP_FELDER + EP_FORMALIEN + VOR_FELDER + KV_PV_FELDER + KAP_FELDER + KAP_ANTRAG_FELDER + P36_ANRECHNUNG_KAP + P32D_Q_KAP + AN_GESAMT_FLAGS
                   + GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP + VORSORGE_PARTNER_FELDER
                   + GESAMT_VERSORGUNG
                   + GESAMT_ABZUEGE + GESAMT_FREIBETRAEGE + GESAMT_GEWINN + GESAMT_GEWINN_PARTNER
                   + GESAMT_33B + GESAMT_33B_PARTNER + KIND_SCREENING + AUSGABEN_SCREENING + PARTNER_SCREENING + INSTANZ_ZAEHLFELDER + VV_ANLAGE_FORMALIEN
                   + GESAMT_DBA + GESAMT_P23 + P22_NR3_EINKUENFTE + GESAMT_P33A + GESAMT_P32B + GESAMT_P35C
                   + GESAMT_REALSPLITTING
                   + DHF_RING + DHF_BEDINGUNGEN + DHF_AUSLANDSGRENZE + DHF_FORMALIEN + VERPFLEGUNG_TAGE + VERPFLEGUNG_TAGE_NACH_FRIST + VERPFLEGUNG_GUARD + VERPFLEGUNG_FRIST + VERPFLEGUNG_KUERZUNG
                   + UEBERNACHTUNG_RING + UEBERNACHTUNG_BEDINGUNGEN + ARBEITSMITTEL_RING
                   + ARBEITSMITTEL_AFA_GESAMT
                   + P36_ANRECHNUNG + P36_ANRECHNUNG_PARTNER + KIST_KONFESSION_FELDER + KIRCHENSTEUER_ARBEITGEBER_FELDER + P16_4_GATE_FELDER + P16_4_GATE_FELDER_PARTNER
                   + STEUERKLASSE_FELDER
                   + STAMMDATEN_FELDER + STAMMDATEN_FELDER_PARTNER),
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
        "felder": RENTNER_FELDER + KAP_FELDER + KAP_ANTRAG_FELDER + P36_ANRECHNUNG_KAP + P32D_Q_KAP + GESAMT_PARTNER_KAP,
        "kegel": RENTNER_KEGEL,
        "felder_datei": None,
        "gesamt_ring": "festzusetzende_est_rentner",
        "teil_ringe": [],
        "guard": True,
        "gesamt_guard": True,
        "rentner": True,
        "multi_rente": "rente",
        "fremd_arten": ("kein_vuv",),
    },
}

# ========== __all__ für import * ==========
# Alle public Namen (ohne führenden _) + private (_*) Namen die für api.py kritisch sind
__all__ = [
    # Private/regex
    "_FALL_RE", "_ERLAUBTE_ZUSTAENDE",
    # Pfade
    "HERE", "FAELLE",
    # § 19 Einkünfte
    "EP_FELDER", "EP_FORMALIEN",
    # an_gesamt
    "AN_GESAMT_FLAGS", "KIND_SCREENING", "AUSGABEN_SCREENING", "PARTNER_SCREENING",
    "INSTANZ_ZAEHLFELDER", "AN_GESAMT_PARTNER",
    # Arbeitsmittel
    "ARBEITSMITTEL_KOSTEN", "ARBEITSMITTEL_RING", "ARBEITSMITTEL_AFA_GESAMT",
    # § 36/§22/§10 KiSt
    "P36_ANRECHNUNG", "P36_ANRECHNUNG_KAP", "P22_NR3_EINKUENFTE", "KIST_KONFESSION_FELDER", "P16_4_GATE_FELDER",
    # Verpflegung
    "VERPFLEGUNG_TAGE", "VERPFLEGUNG_TAGE_NACH_FRIST", "VERPFLEGUNG_GUARD", "VERPFLEGUNG_FRIST", "VERPFLEGUNG_KUERZUNG",
    # Vorsorge
    "VOR_FELDER", "VOR_PARTNER_FELDER", "KV_PV_FELDER", "KV_PV_PARTNER_FELDER", "VORSORGE_PARTNER_FELDER",
    # dHf
    "DHF_KOSTEN", "DHF_RING", "DHF_BEDINGUNGEN", "DHF_AUSLANDSGRENZE", "DHF_FORMALIEN",
    # Übernachtung
    "UEBERNACHTUNG_KOSTEN", "UEBERNACHTUNG_RING", "UEBERNACHTUNG_BEDINGUNGEN",
    # Vermietung
    "VV_GESAMT_FELDER", "VV_ANLAGE_FORMALIEN", "VV_ABS2_TATBESTAND",
    # Kapital
    "KAP_ERTRAEGE", "KAP_TOEPFE", "KAP_FELDER", "KAP_ERTRAEGE_PARTNER", "KAP_TOEPFE_PARTNER", "GESAMT_PARTNER_KAP", "GESAMT_PARTNER_19",
    # Rentner
    "RENTNER_AA_ARTEN", "RENTNER_22", "RENTNER_33B", "RENTNER_33B_PFLEGE_ANGABEN", "RENTNER_PARTNER", "RENTNER_22_PARTNER", "GESAMT_33B", "GESAMT_33B_PARTNER",
    # Gewinn
    "EUER_KOMPONENTEN", "GWG_FELDER", "VERLUST_FELD", "MITU_FELDER", "ABS3_FELDER",
    # § 35
    "GESAMT_P35",
    # § 19 Abs. 2 Versorgung
    "GESAMT_VERSORGUNG",
    # Rentner Gewinn
    "RENTNER_GEWINN", "RENTNER_KEGEL",
    # Abzüge
    "HAUSHALT_35A_ABS23", "HAUSHALT_35A", "P35A_MITVER_ANZEIGE", "AGB_KIST", "KINDERBETREUUNG", "SCHULGELD", "KIND_KV_PV", "KIND_PB_UEBERTRAGUNG", "BEHINDERUNGSBEDINGTE_AUFWENDUNGEN", "FAHRTKOSTEN_PAUSCHALE", "GESAMT_ABZUEGE",
    # Freibeträge
    "GESAMT_FREIBETRAEGE",
    # DBA
    "GESAMT_DBA", "DBA_METHOD_MAP", "DBA_METHOD_MAP_ART", "DBA_EINKUNFTSARTEN",
    "DBA_STAAT_ISO", "dba_methode_fuer", "dba_staat_iso",
    # Weitere Abzüge
    "GESAMT_P23", "GESAMT_P33A", "GESAMT_P32B", "GESAMT_P35C", "GESAMT_REALSPLITTING",
    "GESAMT_PV",
    # Stammdaten
    "STAMMDATEN_FELDER", "STAMMDATEN_FELDER_PARTNER",
    # Veräußerungs-Gewinn
    "GESAMT_VG", "GESAMT_GEWINN",
    # Rentner Felder
    "RENTNER_FELDER",
    # Scheiben
    "SCHEIBEN",
    # Anzeigetexte
    "ENUM_LABELS",
]

# ========== Anzeigetexte für enum-Werte (2026-08-14) ==========
# Bis hierher zeigte die Oberfläche den Rohwert: der Nutzer las "land_forst", "gesetzlich_an"
# oder — bei den Kindschaftsverhältnissen — schlicht "1", "2", "3" (app.js:200 setzte
# o.textContent = v). Diese Tabelle liegt bewusst NEBEN der Bindung statt darin: ein Label ist
# reine Darstellung, kein Feldwissen, und 22 YAML-Dateien dafür anzufassen hätte die Bindung
# aufgebläht, ohne dass die Rechenseite etwas davon hat.
#
# Die Texte sind KEINE Erfindung — sie stehen bereits in fragetext_laie/hilfe_kurz der jeweiligen
# Bindung (z.B. "1 = leibliches Kind/Adoptivkind, 2 = Pflegekind, 3 = Enkelkind/Stiefkind") und
# werden hier nur an die Stelle gehoben, an der der Nutzer sie braucht: ins Auswahlfeld.
#
# Vollständigkeit erzwingt tests/test_enum_labels.py — ein neuer enum_wert ohne Label wird rot.
ENUM_LABELS = {
    "dba_einkunftsart": {
        "unbewegliches_vermoegen": "Unbewegliches Vermögen (z. B. Immobilie)",
        "unternehmensgewinne": "Unternehmensgewinne",
        "dividenden": "Dividenden",
        "zinsen": "Zinsen",
        "lizenzgebuehren": "Lizenzgebühren",
        "veraeusserungsgewinne": "Veräußerungsgewinne",
        "unselbstaendige_arbeit": "Arbeitslohn aus dem Ausland",
        "aufsichtsratsverguetungen": "Aufsichtsratsvergütung",
        "kuenstler_sportler": "Auftritt als Künstler oder Sportler",
        "ruhegehaelter": "Ruhegehalt oder Pension",
    },
    "dba_methode": {
        "kein_dba": "Kein Doppelbesteuerungsabkommen mit diesem Staat",
        "dba_anrechnung": "Anrechnung — die ausländische Steuer wird angerechnet",
        "dba_freistellung": "Freistellung — die Einkünfte bleiben hier steuerfrei",
    },
    "dba_staat": {
        "Deutschland": "Deutschland", "Frankreich": "Frankreich", "Italien": "Italien",
        "Oesterreich": "Österreich", "Schweiz": "Schweiz", "Niederlande": "Niederlande",
        "Polen": "Polen", "Tschechien": "Tschechien", "Dänemark": "Dänemark",
        "Luxemburg": "Luxemburg", "Türkei": "Türkei", "Grossbritannien": "Großbritannien",
        "Spanien": "Spanien", "USA": "USA", "Kanada": "Kanada",
        "sonstiger_staat": "Anderer Staat",
    },
    "gewinn_betriebsart": {
        "gewerbe": "Gewerbebetrieb",
        "selbstaendig": "Selbständige oder freiberufliche Arbeit",
        "land_forst": "Land- und Forstwirtschaft",
    },
    # 1/2/3 stehen so im ELSTER-Feld; die Bedeutung stand bisher nur im Fragetext.
    "kind_kindschaftsverhaeltnis_a": {
        "1": "Leibliches Kind oder Adoptivkind",
        "2": "Pflegekind",
        "3": "Enkelkind oder Stiefkind",
    },
    "kind_anderer_elternteil_kindschaftsverhaeltnis": {
        "1": "Leibliches Kind oder Adoptivkind",
        "2": "Pflegekind",
    },
    # 1/2 stehen so im ELSTER-Feld E0203003. Die Schema-Doku sagt nur "Ziel des Weges" — die
    # Bedeutung der beiden Werte steht im Vordruck, nicht im XSD.
    "ep_ziel_des_weges": {
        "1": "Fester Arbeitsplatz (erste Tätigkeitsstätte)",
        "2": "Sammelpunkt oder weiträumiges Tätigkeitsgebiet",
    },
    # Ebenfalls 1/2/3 aus dem ELSTER-Feld (E0106507). Die Beschriftung stammt wörtlich aus dem
    # Schema ("Steuerpflichtige Person / Ehemann / Person A" usw.) und ist hier auf das übersetzt,
    # was der Nutzer von sich weiß — er kennt weder "Person A" noch seine Rolle im Datensatz.
    "p35c_massnahme_art": {
        "waende": "Wärmedämmung von Wänden",
        "dach": "Wärmedämmung von Dachflächen",
        "geschossdecken": "Wärmedämmung von Geschossdecken",
        "fenster_tueren": "Neue Fenster oder Außentüren",
        "sommerlicher_waermeschutz": "Sommerlicher Wärmeschutz (z. B. Rollläden, Markisen)",
        "lueftung": "Neue oder erneuerte Lüftungsanlage",
        "heizung": "Neue Heizungsanlage",
        "digital": "Digitale Systeme zur Verbrauchsoptimierung",
        "heizung_optimierung": "Optimierung einer bestehenden Heizung (älter als 2 Jahre)",
    },
    "rentner_pflege_durch": {
        "1": "Ich",
        "2": "Mein Ehe- oder Lebenspartner",
        "3": "Wir beide gemeinsam",
    },
    "kist_bundesland": {
        "baden_wuerttemberg": "Baden-Württemberg", "bayern": "Bayern", "berlin": "Berlin",
        "brandenburg": "Brandenburg", "bremen": "Bremen", "hamburg": "Hamburg",
        "hessen": "Hessen", "mecklenburg_vorpommern": "Mecklenburg-Vorpommern",
        "niedersachsen": "Niedersachsen", "nordrhein_westfalen": "Nordrhein-Westfalen",
        "rheinland_pfalz": "Rheinland-Pfalz", "saarland": "Saarland", "sachsen": "Sachsen",
        "sachsen_anhalt": "Sachsen-Anhalt", "schleswig_holstein": "Schleswig-Holstein",
        "thueringen": "Thüringen",
    },
    "kist_konfession": {
        "keine": "Keine Konfession",
        "evangelisch": "Evangelisch",
        "roemisch-katholisch": "Römisch-katholisch",
        "andere": "Andere Religionsgemeinschaft",
    },
    "p23_veraeusserungs_typ": {
        "grundstueck": "Grundstück oder Immobilie",
        "anderes_wg": "Anderes Wirtschaftsgut (z. B. Krypto, Kunst, Edelmetalle)",
    },
    "rentner_renten_art": {
        "gesetzliche_rente": "Gesetzliche Rente",
        "berufsstaendische_versorgung": "Berufsständische Versorgung (z. B. Ärzte, Anwälte)",
        "private_basisrente": "Private Basisrente (Rürup)",
        "private_leibrente": "Private Leibrente",
        "sonstige_leibrente": "Sonstige Leibrente",
    },
    # Kurz und beschreibend, ohne Anspruchsvoraussetzungen — die gehören in die Beratung,
    # nicht in ein Auswahlfeld.
    "steuerklasse": {
        "1": "I — ledig, verwitwet oder geschieden",
        "2": "II — alleinerziehend",
        "3": "III — verheiratet, Partner in Klasse V",
        "4": "IV — verheiratet, beide in Klasse IV",
        "5": "V — verheiratet, Partner in Klasse III",
        "6": "VI — weiteres Dienstverhältnis",
    },
    "veranlagung": {
        "einzel": "Einzelveranlagung — jeder für sich",
        "zusammen": "Zusammenveranlagung mit Ehe- oder Lebenspartner",
    },
    "versicherungsart": {
        "gesetzlich_an": "Gesetzlich als Arbeitnehmer",
        "gesetzlich_freiwillig": "Gesetzlich freiwillig versichert (Selbstzahler)",
        "privat": "Privat versichert",
    },
    "versorgung_art": {
        "beamtenrechtlich": "Beamtenrechtliches Ruhegehalt",
        "hinterbliebene": "Witwen- oder Waisengeld",
        "erwerbsminderung": "Rente wegen Erwerbsminderung",
        "altersgrenze_sonstige": "Betriebsrente oder Direktversicherung",
    },
}
# Partner-Felder teilen die Labels ihres Person-A-Pendants — sonst müsste jede Ergänzung
# zweimal gepflegt werden und liefe beim zweiten Mal auseinander.
for _basis in ("gewinn_betriebsart", "kist_konfession", "rentner_renten_art", "steuerklasse",
               "versicherungsart", "kind_kindschaftsverhaeltnis_a"):
    ENUM_LABELS.setdefault(f"{_basis}_partner", ENUM_LABELS[_basis])
ENUM_LABELS.setdefault("kind_kindschaftsverhaeltnis_b",
                       ENUM_LABELS["kind_kindschaftsverhaeltnis_a"])
ENUM_LABELS.setdefault("rentner_veraeusserungs_betriebsart", ENUM_LABELS["gewinn_betriebsart"])
ENUM_LABELS.setdefault("rentner_veraeusserungs_betriebsart_partner",
                       ENUM_LABELS["gewinn_betriebsart"])
