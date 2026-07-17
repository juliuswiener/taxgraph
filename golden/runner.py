"""Golden-corpus runner: check the Catala § 32a formalisation against the
curated cases in golden/cases/, and verify each case's citation anchor against
the frozen source text (hard gate).

For every case:
  1. the `zitatanker` must occur (after normalisation) in the referenced
     sources/ document;
  2. the Catala-computed tarifliche ESt must equal `erwartung.tarifliche_est`.

Exit code 0 only if all cases pass. Requires the assembled Catala package
(bash oracle/gettsim/assemble_catala.sh) and PyYAML.

Run: python golden/runner.py   (or: make golden)
"""

from __future__ import annotations

import glob
import os
import re
import sys

import yaml  # noqa: F401

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)

from yamlstrict import load_str  # noqa: E402


def load_yaml_fh(fh):
    """Strikt laden: doppelte Schluessel sind ein Fehler, kein Ueberschreiben."""
    with fh:
        return load_str(fh.read(), herkunft=getattr(fh, "name", "<yaml>"))
_CAT = os.path.join(ROOT, "oracle", "gettsim", "_catala")
sys.path.insert(0, os.path.join(_CAT, "rt"))
sys.path.insert(0, _CAT)

from pkg import Einkommensteuertarif as E  # noqa: E402  (Catala-generated)
from pkg import Entfernungspauschale as EP  # noqa: E402
from pkg import Arbeitszimmer_homeoffice as AZ  # noqa: E402
from catala_runtime import Money, Decimal, Bool  # noqa: E402


def _az_params(year: int) -> dict:
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "arbeitszimmer_homeoffice.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in
            ("jahrespauschale", "tagespauschale_pro_tag", "tagespauschale_hoechstbetrag")}


def catala_raumkosten(s: dict) -> int:
    r = _az_params(s["veranlagungszeitraum"])
    out = AZ.raumkostenabzug(AZ.RaumkostenabzugIn(
        arbeitszimmer_vorhanden_in=Bool(s.get("arbeitszimmer_vorhanden", False)),
        ist_mittelpunkt_in=Bool(s.get("ist_mittelpunkt", False)),
        tatsaechliche_aufwendungen_in=Money(f"{int(s.get('tatsaechliche_aufwendungen', 0))}.00"),
        jahrespauschale_gewaehlt_in=Bool(s.get("jahrespauschale_gewaehlt", False)),
        monate_ohne_mittelpunkt_in=int(s.get("monate_ohne_mittelpunkt", 0)),
        homeoffice_tage_in=int(s.get("homeoffice_tage", 0)),
        jahrespauschale_in=Money(f"{int(r['jahrespauschale'])}.00"),
        tagespauschale_pro_tag_in=Money(f"{int(r['tagespauschale_pro_tag'])}.00"),
        tagespauschale_hoechstbetrag_in=Money(f"{int(r['tagespauschale_hoechstbetrag'])}.00")))
    return int(out.abzug_gesamt) // 100


def _ep_saetze(year: int) -> dict:
    """Read the Entfernungspauschale rates for a VZ from params/."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "entfernungspauschale.yaml"), encoding="utf-8"))
    return {k: p[k]["wert"] for k in
            ("satz_bis_20_km", "satz_ab_21_km", "staffelgrenze_km", "hoechstbetrag_ohne_kfz")}


def catala_entfernungspauschale(s: dict) -> int:
    year = s["veranlagungszeitraum"]
    r = _ep_saetze(year)
    out = EP.berechnung(EP.BerechnungIn(
        entfernung_km_roh_in=Decimal(str(s["entfernung_km_roh"])),
        arbeitstage_in=int(s["arbeitstage"]),
        eigenes_oder_ueberlassenes_kfz_in=Bool(s.get("eigenes_oder_ueberlassenes_kfz", False)),
        oepnv_kosten_jahr_in=Money(f"{int(s.get('oepnv_kosten_jahr', 0))}.00"),
        satz_bis_20_km_in=Money(f"{r['satz_bis_20_km']:.2f}"),
        satz_ab_21_km_in=Money(f"{r['satz_ab_21_km']:.2f}"),
        staffelgrenze_km_in=int(r["staffelgrenze_km"]),
        hoechstbetrag_in=Money(f"{int(r['hoechstbetrag_ohne_kfz'])}.00")))
    return int(out.abziehbarer_betrag) // 100


def catala_werbungskosten_n(s: dict) -> int:
    """§ 9 Werbungskosten Anlage N — ROH-Summe in EURO, OHNE § 9a-Arbeitnehmer-Pauschbetrag.
    Den Pauschbetrag-Guenstiger wendet der Tarif `festzusetzende_est_einzel` intern an
    (handverifiziert: ESt(WK 0)==ESt(WK 1230)); ein § 9a hier waere doppelter Abzug.

    Stufe 1: nur die Entfernungspauschale hat ein Catala-Modul. dHf (§ 9 Abs. 1 Nr. 5),
    Verpflegung (§ 9 Abs. 4a) und Arbeitsmittel (§ 9 Abs. 1 Nr. 6/7) haben (noch) keins ->
    hier NICHT aggregiert; ihr Vorhandensein sperrt in der Haut den Stufe-1-Ring
    (bestaetigte-Null-Guard, kein stiller 0-Verschluck). Erweiterungsstelle fuer Stufe 1b."""
    wk = 0
    if "entfernung_km_roh" in s:
        wk += catala_entfernungspauschale(s)
    # Stufe 1b (nach deren Catala-Modul-Bau):
    #   + catala_dhf(s) + catala_verpflegung(s) + catala_arbeitsmittel(s)
    return wk


def _kindergeld(year: int) -> int:
    """Monatliches Kindergeld je Kind aus params/<vz> (§ 66 EStG): 250/255/259."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "kindergeld_p66.yaml"), encoding="utf-8"))
    return p["kindergeld_monatlich_je_kind"]["wert"]


def _vorsorge_hb(year: int) -> int:
    """Vorsorge-Hoechstbetrag aus params/<vz> (§ 10 Abs. 3): 27566/29344/30826.
    Deckel-Eingabe von p10_1_2_altersvorsorge; ueber _vorsorge_abzug in den
    Sonderausgaben-Pfad von catala_gesamt verdrahtet."""
    p = load_yaml_fh(open(os.path.join(
        ROOT, "params", str(year), "vorsorge_hoechstbetrag_p10.yaml"), encoding="utf-8"))
    return p["hoechstbeitrag"]["wert"]


def _vorsorge_abzug(s: dict, year: int) -> int:
    """§ 10 Abs. 1 Nr. 2, Abs. 3: abziehbare Altersvorsorge (p10_1_2, MVP 100 %-
    Ansatz). Die Gesamtbeitraege (inkl. AG-Anteil) werden auf den Hoechstbeitrag zur
    knappschaftl. RV aus params/<vz> gedeckelt, dann um den steuerfreien AG-Anteil
    gekuerzt - Kuerzung NACH dem Cap (Instructor-Semantik msg 1197). Nur aktiv, wenn
    ein Vorsorge-Beitrag gesetzt ist; sonst 0 (Rueckwaertskompatibilitaet)."""
    beitraege = int(s.get("vorsorge_gesamtbeitraege_inkl_ag", 0))
    if not beitraege:
        return 0
    ag = int(s.get("vorsorge_ag_anteil_steuerfrei", 0))
    return max(0, min(beitraege, _vorsorge_hb(year)) - ag)


# -- GewSt-Kette (Paket 4, §§ 6-11/35) - Python-Andockung analog _vorsorge_abzug.
# Macht den GewSt-Steuermessbetrag (p11) + § 35-Anrechnung (p35_1) produktiv, ohne
# p35_1 strukturell zu beruehren. Rechnung in ganzen CENT (exakt); die catala-Regeln
# p7/p8/p9_*/p10a/p11 sind separat via clerk/snapshot verifiziert. Erwartungen =
# dev-2-Hand-Ketten (reports/review/2026-07-16-gewst-ketten-golden.md), unabhaengige
# Zweitrechnung.

def _gewst_hinzurechnung_p8(s: dict) -> int:
    """§ 8 Nr. 1: Viertel der um 200.000 geminderten Summe der Finanzierungsanteile
    (a/b/c 100 %, d 1/5, e 1/2, f 1/4). Ganze Euro."""
    summe = (int(s.get("gewst_entgelte_schulden", 0))
             + int(s.get("gewst_renten", 0))
             + int(s.get("gewst_stille", 0))
             + int(s.get("gewst_miet_beweglich", 0)) // 5
             + int(s.get("gewst_miet_unbeweglich", 0)) // 2
             + int(s.get("gewst_rechte", 0)) // 4)
    return max(0, summe - 200000) // 4


def _gewst_kuerzung_p9(s: dict, year: int) -> int:
    """§ 9 Nr. 1 S. 1 (VZ-Split: EZ 2024 = 1,2 % Einheitswert; EZ 2025+ =
    tatsaechliche Grundsteuer) + Nr. 2 Gewinnanteile + Nr. 2a Schachtel. Ganze Euro."""
    if year <= 2024:
        grund = int(s.get("gewst_einheitswert", 0)) * 12 // 1000  # 1,2 %
    else:
        grund = int(s.get("gewst_grundsteuer", 0))
    return (grund + int(s.get("gewst_gewinnanteile_mitunternehmer", 0))
            + int(s.get("gewst_schachteldividenden", 0)))


def _gewst_messbetrag_cent(s: dict) -> int:
    """§ 7 -> § 10a -> § 11: Steuermessbetrag in CENT. Gewerbeertrag = Gewinn +
    § 8 - § 9; optional § 10a-Verlustabzug (1-Mio-Sockel + 60 %); dann abrunden auf
    volle 100 Euro, Freibetrag 24.500 (natuerl. Person/PersG), Messzahl 3,5 %."""
    year = int(s["veranlagungszeitraum"])
    ge = (int(s.get("gewinn_gewerbebetrieb", 0)) + _gewst_hinzurechnung_p8(s)
          - _gewst_kuerzung_p9(s, year))
    fehlbetrag = int(s.get("fehlbetrag_bestand", 0))
    if fehlbetrag:
        kapazitaet = 1000000 + max(0, ge - 1000000) * 60 // 100
        ge -= min(fehlbetrag, kapazitaet)
    abgerundet = (ge // 100) * 100 if ge > 0 else 0
    nach_fb = max(0, abgerundet - 24500)
    return nach_fb * 35 // 10   # * 3,5 % in Cent (nach_fb Vielfaches von 100 -> exakt)


def _gewst_p35_anrechnung_cent(s: dict) -> int:
    """§ 35 EStG (p35_1): min(4x Messbetrag, tatsaechlich zu zahlende GewSt =
    Messbetrag x Hebesatz). Deckel 3 (Ermaessigungshoechstbetrag) = ESt-Kontext,
    ausserhalb der reinen GewSt-Kette (dev-2). Hebesatz in Prozent. CENT."""
    mb = _gewst_messbetrag_cent(s)
    vierfach = mb * 4
    gewst = mb * int(s["gewst_hebesatz"]) // 100
    return min(vierfach, gewst)


def catala_gewst(s: dict) -> int:
    """Dispatch der GewSt-Kette: Steuermessbetrag ODER § 35-Anrechnung, in CENT."""
    if s.get("gewst_output") == "p35_anrechnung":
        return _gewst_p35_anrechnung_cent(s)
    return _gewst_messbetrag_cent(s)


# --- KStG Nenner B (Kapitalgesellschaft, Paket 5, K1-K5 + § 23 + SolZ + GewSt) -----
# Alle Zwischengroessen in CENT (Klasse-5, Cent-Schnitt zuletzt, // = floor). Kette:
# einkommen_slot (§ 8 Abs.1/3) - § 8b-netto + § 10-Addback = einkommen_vor_spenden
# -> - § 9-Spenden = GdE (§ 10d-Basis) -> - § 10d-Verlust (70%) = zvE (§ 24-FB=0)
# -> KSt 15 % (§ 23) + SolZ 5,5 % + GewSt (Slot = einkommen_vor_spenden).

def _kst_8b_netto_cent(s: dict) -> int:
    """§ 8b: Bezuege (Abs.1) + Veraeusserung (Abs.2) 100 % ausser Ansatz MINUS 5 %
    Pauschale (Abs.5/3) = 95 % netto. Streubesitz < 10 % (Abs.4): Dividende voll
    steuerpflichtig (steuerfrei=0, Pauschale faellt mit). 95 % NIE als Konstante. CENT."""
    div = int(s.get("dividende_bezuege", 0)) * 100
    steuerfrei = div if int(s.get("beteiligung_prozent", 0)) >= 10 else 0
    veraeuss = int(s.get("veraeusserungsgewinn", 0)) * 100
    ausser = steuerfrei + veraeuss
    return ausser - ausser * 5 // 100


def _kst_massgebliches_einkommen_cent(s: dict) -> int:
    """§ 8 Abs.1-Slot + vGA (Abs.3 S.2) - verd. Einlage (Abs.3 S.3), - § 8b-netto,
    + § 10-Addback (Nr.2/3). = maßgebliches Einkommen § 8a Abs.1 S.2 (PRE-§ 4h/§ 9/§ 10d):
    Basis fuer die § 4h-EBITDA-Herleitung (N3) UND fuer den § 4h-Add-back. CENT."""
    slot = (int(s.get("gewinn_estg", 0)) + int(s.get("verdeckte_gewinnausschuettung", 0))
            - int(s.get("verdeckte_einlage", 0))) * 100
    addback = (int(s.get("personensteuern", 0)) + int(s.get("geldstrafen", 0))) * 100
    return slot - _kst_8b_netto_cent(s) + addback


def _kst_nichtabziehbare_zinsen_cent(s: dict) -> int:
    """§ 4h/§ 8a Zinsschranke-Add-back (N1-N3). ebitda_basis = maßgebliches Einkommen +
    Zinsaufwand + AfA - Zinsertrag (§ 8a: Einkommen statt Gewinn). Deckel = 30 % ebitda_basis
    + EBITDA-Vortrag; zinsaufwand_eff = Zinsaufwand + Zinsvortrag (nur Abzugstest). N2-Ausnahme:
    Freigrenze nettozins < 3 Mio ODER Konzern-lit-b ODER Escape-lit-c -> voller Abzug.
    ADD-BACK = gebuchter Zinsaufwand - abziehbar_final, UNGEKLEMMT (Vortrags-Verbrauch senkt
    das Einkommen, § 4h Abs.1 S.6; der Zinsvortrag war nie im gewinn_estg abgezogen -> NICHT
    im Add-back-Grundbetrag). CENT.

    S.-7-LATENZ (benannter Nachtrag, Backlog): greift eine Abs.-2-Ausnahme (Freigrenze/
    Konzern/Escape), gibt dieser Zweig zinsaufwand_eff INKL. Zinsvortrag frei -> bei
    zins_vortrag_bestand > 0 waere der Add-back negativ (Einkommen sinkt um den Vortrag),
    was § 4h Abs.1 S.7 ("Absatz 2 findet keine Anwendung, soweit Zinsaufwendungen aufgrund
    eines Zinsvortrags erhoeht wurden") anteilig ausschliesst. KEIN Golden kombiniert Ausnahme
    UND Zinsvortrag > 0 (S.-7-Anteiligkeit nicht modelliert, N2-Bedingung
    freigrenze_zinsvortrag_anteiligkeit_s7_nachtrag). Ein echter Tripwire braucht
    Negativ-zvE-Handling -> Backlog, hier NICHT gebaut."""
    zinsauf = int(s.get("zinsaufwand", 0)) * 100
    zinsert = int(s.get("zinsertrag", 0)) * 100
    afa = int(s.get("abschreibungen", 0)) * 100
    zvortrag = int(s.get("zins_vortrag_bestand", 0)) * 100
    evortrag = int(s.get("ebitda_vortrag_bestand", 0)) * 100
    ebitda_basis = _kst_massgebliches_einkommen_cent(s) + zinsauf + afa - zinsert
    verr_ebitda = ebitda_basis * 30 // 100
    zinsauf_eff = zinsauf + zvortrag
    deckel = verr_ebitda + evortrag
    abziehbar_kern = zinsert + min(zinsauf_eff - zinsert, deckel)
    nettozins = zinsauf - zinsert
    ausnahme = (nettozins < 3000000 * 100
                or bool(s.get("keine_konzern_oder_nahestehende_b"))
                or bool(s.get("eigenkapital_escape_c")))
    abziehbar_final = zinsauf_eff if ausnahme else abziehbar_kern
    return zinsauf - abziehbar_final


def _kst_verlustbestand_nach_8c_8d_cent(s: dict) -> int:
    """§ 8c (N4) / § 8d (N5) auf den Verlustbestand VOR § 10d. Schaedlicher Erwerb -> 0;
    § 8d-Suspension (schaedlich UND Antrag UND Fortfuehrungs-Voraussetzungen) -> voller
    Bestand erhalten (fortfuehrungsgebunden). CENT."""
    bestand = int(s.get("verlustvortrag_bestand", 0)) * 100
    schaedlich = bool(s.get("schaedlicher_erwerb"))
    nach_8c = 0 if schaedlich else bestand
    suspension = (schaedlich and bool(s.get("antrag_8d"))
                  and bool(s.get("fortfuehrungs_voraussetzungen")))
    return bestand if suspension else nach_8c


def _kst_einkommen_vor_spenden_cent(s: dict) -> int:
    """einkommen_vor_spenden = maßgebliches Einkommen + § 4h-Add-back (nichtabziehbare
    Zinsen erhoehen das Einkommen, N6). Zugleich GewSt-Slot (§ 7 GewStG) UND § 9-Abs.2-S.1-
    Hoechstbetragsbasis. CENT. (Ohne § 4h-Inputs = maßgebliches Einkommen unveraendert.)"""
    return _kst_massgebliches_einkommen_cent(s) + _kst_nichtabziehbare_zinsen_cent(s)


def _kst_spendenabzug_cent(s: dict) -> int:
    """§ 9 Abs.1 Nr.2: min(Zuwendungen, max(20 % Einkommen, 4 Promille (Umsatz+Loehne))).
    20/100 und 4/1000 exakt. Basis = einkommen_vor_spenden (Abs.2 S.1). CENT."""
    evs = _kst_einkommen_vor_spenden_cent(s)
    grenze_a = evs * 20 // 100
    grenze_b = (int(s.get("umsaetze", 0)) + int(s.get("loehne_gehaelter", 0))) * 100 * 4 // 1000
    return min(int(s.get("zuwendungen", 0)) * 100, max(grenze_a, grenze_b))


def _kst_zve_cent(s: dict) -> int:
    """§ 9-Spenden -> § 10d-Verlust (Sockel 1 Mio + 70 %, KapGes ohne Splitting) -> zvE.
    § 10d-Basis (GdE) = Einkommen NACH Spenden (R 7.1 KStR). § 24-Freibetrag = 0
    (KapGes-Ausschluss § 24 S.2 Nr.1). CENT."""
    evs = _kst_einkommen_vor_spenden_cent(s)
    gde = evs - _kst_spendenabzug_cent(s)
    sockel = 1000000 * 100
    hoechst = sockel + max(0, gde - sockel) * 70 // 100
    verlust = min(_kst_verlustbestand_nach_8c_8d_cent(s), hoechst)
    return max(0, gde - verlust)


def _kst_gewst_cent(s: dict) -> int:
    """GewSt KapGes: Gewerbeertrag = einkommen_vor_spenden (mit § 8b/§ 10, OHNE § 9/§ 10d;
    § 8/§ 9 GewStG-Anpassungen = 0, § 10a-Vortrag = 0). KEIN 24.500-Freibetrag (nur
    natuerl. Person/PersG, § 11). floor auf 100 Euro, Messzahl 3,5 %, x Hebesatz. CENT.
    (§ 8 Nr.9 / § 9 Nr.5 GewStG-Spendenkorrektur = benannter Nachtrag, nicht modelliert.)"""
    ge_euro = _kst_einkommen_vor_spenden_cent(s) // 100
    abger = (ge_euro // 100) * 100 if ge_euro > 0 else 0
    messbetrag_cent = abger * 35 // 10
    return messbetrag_cent * int(s["gewst_hebesatz"]) // 100


def catala_kst_nenner_b(s: dict) -> int:
    """Nenner B (KapGes) in CENT: KSt (§ 23, 15 % des zvE) + SolZ (5,5 %) + GewSt."""
    kst = _kst_zve_cent(s) * 15 // 100
    solz = kst * 55 // 1000
    return kst + solz + _kst_gewst_cent(s)


VZ_ENUM = {
    2024: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2024, None),
    2025: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2025, None),
    2026: E.Veranlagungszeitraum(E.Veranlagungszeitraum.Code.VZ2026, None),
}

_UMLAUT = str.maketrans({"ä": "ae", "ö": "oe", "ü": "ue", "ß": "ss",
                         "Ä": "ae", "Ö": "oe", "Ü": "ue"})


def normalize(text: str) -> str:
    return re.sub(r"\s+", " ", text.translate(_UMLAUT).lower()).strip()


def catala_gesamt(s: dict) -> int:
    """Verallgemeinerte § 2-Veranlagung: alle Andockstellen als money-Eingaben."""
    year = s["veranlagungszeitraum"]
    def m(k):
        return Money(f"{int(s.get(k, 0))}.00")
    # § 31 S. 4 Hinzurechnung: ist `kinder_ganzjaehrig` gesetzt, wird der Jahres-
    # Kindergeld-Betrag aus params/<vz> abgeleitet (Monatswert x 12 x Kinder),
    # sonst gilt der direkte sachverhalt-Wert. So ist die Groesse params-geankert.
    hinzu_kg = int(s.get("hinzurechnung_kindergeld", 0))
    if s.get("kinder_ganzjaehrig"):
        hinzu_kg = _kindergeld(year) * 12 * int(s["kinder_ganzjaehrig"])
    # § 10 Sonderausgaben: die abziehbare Altersvorsorge (p10_1_2, auf den params-HB
    # gedeckelt) wird zum sonderausgaben-Wert addiert. Ohne Vorsorge-Input bleibt
    # sonderausgaben unveraendert - die 62 Bestandsfaelle rechnen wie bisher.
    sonder = int(s.get("sonderausgaben", 0)) + _vorsorge_abzug(s, year)
    scope = (E.festzusetzende_est_gesamt_zusammen if s.get("veranlagung") == "zusammen"
             else E.festzusetzende_est_gesamt)
    cls = (E.FestzusetzendeEstGesamtZusammenIn if s.get("veranlagung") == "zusammen"
           else E.FestzusetzendeEstGesamtIn)
    out = scope(cls(
        einkuenfte_nichtselbststaendig_in=m("einkuenfte_nichtselbststaendig"),
        einkuenfte_kapitalvermoegen_in=m("einkuenfte_kapitalvermoegen"),
        einkuenfte_vermietung_in=m("einkuenfte_vermietung"),
        einkuenfte_sonstige_in=m("einkuenfte_sonstige"),
        einkuenfte_gewinn_in=m("einkuenfte_gewinn"),
        altersentlastungsbetrag_in=m("altersentlastungsbetrag"),
        entlastungsbetrag_alleinerziehende_in=m("entlastungsbetrag_alleinerziehende"),
        sonderausgaben_in=Money(f"{sonder}.00"),
        aussergewoehnliche_belastungen_in=m("aussergewoehnliche_belastungen"),
        freibetraege_kinder_in=m("freibetraege_kinder"),
        sonstige_abzuege_vom_einkommen_in=m("sonstige_abzuege_vom_einkommen"),
        anzurechnende_auslaendische_steuern_in=m("anzurechnende_auslaendische_steuern"),
        steuerermaessigungen_in=m("steuerermaessigungen"),
        steuer_kapital_gesondert_in=m("steuer_kapital_gesondert"),
        hinzurechnung_kindergeld_in=Money(f"{hinzu_kg}.00"),
        hinzurechnung_zulage_in=m("hinzurechnung_zulage"),
        tarif_modifiziert_in=Bool(s.get("tarif_modifiziert", False)),
        tarifliche_est_modifiziert_in=m("tarifliche_est_modifiziert"),
        veranlagungszeitraum_in=VZ_ENUM[year]))
    return int(out.festzusetzende_est) // 100


def _p35c_ermaessigung_cent(s: dict) -> int:
    """§ 35c Abs. 1 EStG energetische Sanierung: satz 7 % (6 % im uebernaechsten
    Foerderjahr), hoechst 14.000 (12.000 uebernaechst); Ermaessigung = min(satz x
    Aufwand, hoechst). CENT (Python-Accessor analog GewSt/KSt; die § 2-Verrechnung
    mit der tariflichen ESt ist NICHT diese Regel)."""
    uebernaechst = bool(s.get("ist_uebernaechstes_foerderjahr"))
    satz = 6 if uebernaechst else 7
    hoechst = (12000 if uebernaechst else 14000) * 100
    aufwand = int(s.get("sanierungsaufwendungen", 0)) * 100
    return min(aufwand * satz // 100, hoechst)


def _kfz_nutzungswert_monat_cent(s: dict) -> int:
    """§ 6 Abs. 1 Nr. 4 S. 2 EStG 1 %-Regel: nutzungswert_monat = BLP / 100 / Teiler
    (Teiler 1 voll, 2 halb, 4 viertel bei E/Hybrid). MONATSwert in CENT."""
    blp = int(s.get("bruttolistenpreis", 0)) * 100
    teiler = int(s.get("bruchteils_teiler", 1))
    return blp // 100 // teiler


def catala_est(sachverhalt: dict) -> int:
    # § 35c energetische Sanierung + § 6 Abs. 1 Nr. 4 Kfz-Nutzungswert (Paket 9, amtliche
    # Goldens): eigenstaendige Python-Accessoren, VZ-agnostisch -> VOR dem year-Read.
    if "sanierungsaufwendungen" in sachverhalt:
        return _p35c_ermaessigung_cent(sachverhalt)
    if "bruttolistenpreis" in sachverhalt:
        return _kfz_nutzungswert_monat_cent(sachverhalt)
    year = sachverhalt["veranlagungszeitraum"]
    veranlagung = sachverhalt.get("veranlagung")
    # Verallgemeinerte § 2-Veranlagung (Gesamtfall).
    if sachverhalt.get("gesamtfall"):
        return catala_gesamt(sachverhalt)
    # GewSt-Kette (§§ 6-11/35): Steuermessbetrag oder § 35-Anrechnung, in CENT.
    if sachverhalt.get("gewerbesteuer"):
        return catala_gewst(sachverhalt)
    # KStG Nenner B (Kapitalgesellschaft): KSt + SolZ + GewSt, in CENT.
    if sachverhalt.get("koerperschaft"):
        return catala_kst_nenner_b(sachverhalt)
    # Entfernungspauschale (§ 9): abziehbarer Betrag.
    if "entfernung_km_roh" in sachverhalt:
        return catala_entfernungspauschale(sachverhalt)
    # Arbeitszimmer/Homeoffice (§ 4 Abs. 5 Nr. 6b/6c): abzug_gesamt.
    if "arbeitszimmer_vorhanden" in sachverhalt:
        return catala_raumkosten(sachverhalt)
    # End-to-end Arbeitnehmerfall (Bruttolohn -> festzusetzende ESt).
    if "bruttoarbeitslohn" in sachverhalt:
        out = E.festzusetzende_est_einzel(E.FestzusetzendeEstEinzelIn(
            bruttoarbeitslohn_in=Money(f"{int(sachverhalt['bruttoarbeitslohn'])}.00"),
            werbungskosten_in=Money(f"{int(sachverhalt.get('werbungskosten', 0))}.00"),
            sonderausgaben_in=Money(f"{int(sachverhalt.get('sonderausgaben', 0))}.00"),
            veranlagungszeitraum_in=VZ_ENUM[year]))
        return int(out.festzusetzende_est) // 100
    # § 34 Abs. 1 Fuenftelregelung (ausserordentliche Einkuenfte), Kernfall verbleibendes zvE >= 0.
    if "ausserordentliche_einkuenfte" in sachverhalt:
        return catala_fuenftel(sachverhalt)
    # Tariff-level case (zvE -> tarifliche ESt).
    m = Money(f"{int(sachverhalt['zu_versteuerndes_einkommen'])}.00")
    if veranlagung == "einzel":
        out = E.grundtarif(E.GrundtarifIn(
            zu_versteuerndes_einkommen_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
    elif veranlagung == "zusammen":
        out = E.splittingtarif(E.SplittingtarifIn(
            zu_versteuerndes_einkommen_gemeinsam_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
    else:
        raise ValueError(f"unknown veranlagung: {veranlagung}")
    return int(out.tarifliche_steuer) // 100


def catala_fuenftel(s: dict) -> int:
    """§ 34 Abs. 1 Fuenftelregelung (ausserordentliche Einkuenfte), Kernfall verbleibendes zvE >= 0.

    ORCHESTRIERT bestehende Scopes (catala_gesamt-Muster, KEIN neuer Rechenpfad, keine Nachbildung
    der Tariflogik): der § 32a-Tarif kommt aus dem Catala-Scope Einkommensteuertarif, der Faktor 5
    ist die exakte Struktur-Konstante der Regel p34_fuenftel_ao_est (dort als est_ao = 5 x Differenz
    verifiziert). Glue = Subtraktion (verbleibendes zvE), 1/5-Aufteilung und Summe.

        verbleibendes_zve = zvE - ao
        est1 = Tarif(verbleibendes_zve)
        est2 = Tarif(verbleibendes_zve + ao/5)
        est_ao = 5 * (est2 - est1)                 # p34_fuenftel_ao_est
        tarifliche_est = est1 + est_ao

    NEGATIVFALL § 34 Abs. 1 S. 3 (verbleibendes zvE negativ, zvE positiv) ist modelliert: est_ao =
    5 x Tarif(zvE/5), tarifliche ESt = est_ao (Grundbetrag 0). Regel p34_1_s3_fuenftel_negativ
    (Paket 10c Block 8). H 34.2 Bsp 2 (Erwartung 12.010) als Golden verankert.
    """
    year = s["veranlagungszeitraum"]
    ver = s.get("veranlagung")
    zve = int(s["zu_versteuerndes_einkommen"])
    ao = int(s["ausserordentliche_einkuenfte"])
    verbleibendes_zve = zve - ao

    def _tarif_cent(x: int) -> int:
        m = Money(f"{int(x)}.00")
        if ver == "einzel":
            out = E.grundtarif(E.GrundtarifIn(
                zu_versteuerndes_einkommen_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
        elif ver == "zusammen":
            out = E.splittingtarif(E.SplittingtarifIn(
                zu_versteuerndes_einkommen_gemeinsam_in=m, veranlagungszeitraum_in=VZ_ENUM[year]))
        else:
            raise ValueError(f"unknown veranlagung: {ver}")
        return int(out.tarifliche_steuer)

    # § 34 Abs. 1 S. 3: verbleibendes zvE negativ UND zvE positiv -> est_ao = 5 x Tarif(zvE/5);
    # die tarifliche ESt IST das est_ao komplett (Grundbetrag 0, kein est1-Summand). Regel
    # p34_1_s3_fuenftel_negativ (Sonderpfad-Schwester, faithful-Andockung-Artefakt nicht_echt).
    if verbleibendes_zve < 0:
        if zve <= 0:
            raise ValueError("§ 34 Abs. 1 S. 3 setzt ein positives zvE voraus.")
        return (5 * _tarif_cent(zve // 5)) // 100

    est1 = _tarif_cent(verbleibendes_zve)
    est2 = _tarif_cent(verbleibendes_zve + ao // 5)
    est_ao = 5 * (est2 - est1)
    return (est1 + est_ao) // 100


def main() -> int:
    cases = sorted(glob.glob(os.path.join(ROOT, "golden", "cases", "*.yaml")))
    if not cases:
        print("no golden cases found")
        return 1

    source_cache: dict[str, str] = {}
    failures = []

    for path in cases:
        c = load_yaml_fh(open(path, encoding="utf-8"))
        cid = c["id"]
        s = c["sachverhalt"]
        erw = c["erwartung"]
        exp = erw.get("tarifliche_est",
                      erw.get("festzusetzende_est",
                              erw.get("abziehbarer_betrag",
                                      erw.get("abzug_gesamt",
                                              erw.get("gewst_cent",
                                                      erw.get("nenner_b_cent",
                                                              erw.get("sanierung_ermaessigung_cent",
                                                                      erw.get("nutzungswert_monat_cent"))))))))
        q = c["quelle"]

        # 1. citation-anchor gate
        src_path = os.path.join(ROOT, q["datei"])
        if src_path not in source_cache:
            source_cache[src_path] = normalize(open(src_path, encoding="utf-8").read())
        anchor_ok = normalize(q["zitatanker"]) in source_cache[src_path]

        # 2. value check against Catala
        got = catala_est(s)
        value_ok = got == exp

        if anchor_ok and value_ok:
            print(f"OK       {cid}  (est={got})")
        else:
            reason = []
            if not anchor_ok:
                reason.append("Zitatanker nicht im Quelltext")
            if not value_ok:
                reason.append(f"Wert Catala={got} != erwartet={exp}")
            print(f"FAIL     {cid}  -> {'; '.join(reason)}")
            failures.append(cid)

    print(f"\n{len(cases) - len(failures)}/{len(cases)} Faelle bestanden.")
    return 0 if not failures else 1


if __name__ == "__main__":
    sys.exit(main())
