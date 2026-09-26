"""Klasse C (Vault backlog/taxgraph/klasse-c-vorlaeufiges-einkommen-faellt-still-aus.md):
ein VORLAEUFIGER Betrag mit Ring-Wirkung faellt still aus der festgesetzten Zahl — und die Zahl
heisst trotzdem "bestaetigt".

Der Mechanismus (eine Zeile, beide Scheiben):
  produkt/bescheid/bescheid_zweige.py::_bescheid_fn filtert die flachen felder auf
  `zustand == "bestaetigt"`. Ein vorlaeufiger Betrag ist darin schlicht ABSENT, die slot_fn liest
  `_c(fid) == 0` und rechnet ohne ihn weiter. Das ist als over-tax-safe dokumentiert und fuer sich
  genommen richtig — falsch ist die Beschriftung: `_ergebnis_roh` gibt `grund="bestaetigt"` aus,
  und `/preflight` meldet GREEN.

Gemessen (5f5cbfd, Scheibe rentner_gesamt, rentner_veraeusserungsgewinn = 100.000 EUR):
  P0 ohne vg                          -> ("bestaetigt", 5917000)
  P1 vg VORLAEUFIG                    -> ("bestaetigt", 5917000)   <-- 23.100 EUR fehlen, ohne Signal
  P2 vg bestaetigt                    -> ("bestaetigt", 8227000)
P1 ist zeichengleich mit P0, obwohl der Nutzer den Betrag genannt hat.

Kein Einzelfall, sondern eine Klasse: der Guard (_an_gesamt_sperrgrund) prueft die Zustandsachse
nur an den Stellen, die er einzeln kennt (§ 16 Abs. 4 prueft die zwei Bedingungs-Bools, nicht den
Betrag selbst; § 19 Abs. 2 prueft Beginnjahr und Bemessungsgrundlage, nicht versorgung_jahresrente).
Deshalb sperrt der Fix generisch ueber die Felder, die der Ring wirklich liest.

WARUM DIESER TEST _feste_zahl RUFT UND NICHT SELBST ENTSCHEIDET
----------------------------------------------------------------
Die naheliegende Abkuerzung (Muster aus test_gate_naht_guard_liest_zustand.py: Guard, dann
Kegel-Meet, dann _bescheid_fn von Hand nachbauen) misst NICHT den Nutzerpfad: sie umgeht
_feste_zahl, also genau die Funktion, in der die Sperre sitzt. Ein solcher Test waere mit und ohne
Fix gruen — die Probe haette den Defekt nur zufaellig getroffen. Dieser Helfer ruft deshalb die
ECHTEN Entscheidungsfunktionen (API._feste_zahl fuer "darf eine Zahl erscheinen",
API._vorlaeufige_ring_betraege fuer die offen-Liste) und spiegelt nur noch die Reihenfolge der
Zweige aus _ergebnis_roh. Dass die Reihenfolge dort selbst stimmt, kann dieser Test nicht sehen —
dafuer braucht es einen Lauf ueber den echten Endpunkt (offener Punkt, s. Bericht).

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/engine", "golden", "produkt/store", "produkt/traverser",
            "produkt/mapping", "produkt/unsicherheit", "produkt/bescheid", "produkt/konsistenz",
            "produkt/eingang"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API                        # noqa: E402
import bescheid_deklaration as BD        # noqa: E402
import store as ST                       # noqa: E402
import traverser as TR                   # noqa: E402
from api_constants import SCHEIBEN       # noqa: E402

VZ = 2025
V = "vorlaeufig"

RENTNER = [
    ("veranlagung", "einzel"), ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 20000000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 65), ("rentner_rentenfreibetrag", 0),
    ("rentner_grad_der_behinderung", 0), ("rentner_hilflos_blind_taubblind", False), ("rentner_hinterbliebenenbezuege", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0), ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
    ("mit_anspruch_auf_zuschuss", False),
]

GESAMT = [
    ("veranlagung", "einzel"), ("bruttoarbeitslohn", 6000000),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0), ("vv_erhaltungsaufwand", 0),
    ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 100),
    ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0), ("ep_eigenes_kfz", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0), ("basis_kv", 0), ("basis_pv", 0),
    ("versicherungsart", "gesetzlich_an"), ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
    ("mit_anspruch_auf_zuschuss", False),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0), ("kap_verlust_aktien", 0),
    ("kap_verlust_sonstige", 0),
]


def _felder(basis: list[tuple], extra: dict) -> dict:
    """basis: [(feld, wert), ...], alle bestaetigt. extra: feld -> wert | (wert, zustand) | None (fehlt)."""
    f = {k: {"wert": v, "zustand": "bestaetigt"} for k, v in basis}
    for k, v in extra.items():
        if v is None:
            continue
        w, z = v if isinstance(v, tuple) else (v, "bestaetigt")
        f[k] = {"wert": w, "zustand": z}
    return f


def _ergebnis(scheibe: str, f: dict) -> tuple[str, str | int, list]:
    """("GESPERRT"|"bestaetigt", sperrgrund|zahl_cent, offen) -- Zweigreihenfolge wie _ergebnis_roh,
    die Entscheidungen selbst kommen aus den echten Funktionen (s. Modul-Docstring)."""
    cfg = SCHEIBEN[scheibe]
    bindung = TR.lade_bindung()
    sperr = BD._an_gesamt_sperrgrund(f, cfg, VZ, None, None)
    if sperr:
        return ("GESPERRT", sperr, [])
    r = API._feste_zahl(f, bindung, cfg, VZ, cfg["kegel"], None)
    if r is None:
        betrag_offen = sorted(API._vorlaeufige_ring_betraege(f, cfg, bindung))
        if betrag_offen:
            return ("GESPERRT", "ring_betrag_vorlaeufig", betrag_offen)
        return ("GESPERRT", "input_kegel_nicht_bestaetigt", [])
    return ("bestaetigt", r[0], [])


# ===================== Die Probe P (§ 16 Abs. 4, Scheibe rentner_gesamt) =====================
# Die zwei Bedingungs-Bools sind BESTAETIGT -- der bestehende p16_4_gate_offen-Gate feuert also
# nicht. Nur der BETRAG selbst ist vorlaeufig, und genau das sieht heute niemand an.

def test_P0_kontrolle_ohne_vg_bleibt_bestaetigt():
    """Kontrolle: ohne den Betrag ist alles wie bisher (5917000) -- der Fix darf den Normalfall
    nicht anfassen."""
    f = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                          "rentner_freibetrag_erstmalig": True})
    assert _ergebnis("rentner_gesamt", f)[:2] == ("bestaetigt", 5917000)


def test_P1_vorlaeufiger_vg_sperrt_statt_still_zu_fallen():
    """DER FEHLER: 100.000 EUR vorlaeufiger Veraeusserungsgewinn -- der Nutzer hat den Betrag
    genannt. Vor dem Fix kam zeichengleich P0 heraus (5917000), also 23.100 EUR zu wenig, mit
    grund="bestaetigt". Nach dem Fix: GESPERRT, und der Betrag steht in der Antwort."""
    f = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                          "rentner_freibetrag_erstmalig": True,
                          "rentner_veraeusserungsgewinn": (10000000, V)})
    status, grund, offen = _ergebnis("rentner_gesamt", f)
    assert status == "GESPERRT", (
        f"Ein vorlaeufiger Veraeusserungsgewinn von 100.000 EUR fiel weiter still aus der Zahl "
        f"({status}, {grund}) -- genau der Defekt aus dem Vault-Ticket.")
    # Die Sperre muss BENANNT sein, nicht nur wirksam: der Nutzer soll sehen, welcher Betrag fehlt.
    assert "rentner_veraeusserungsgewinn" in offen, (
        f"Gesperrt, aber die Antwort nennt das Feld nicht ({offen}) -- der Nutzer erfaehrt nicht, "
        "welche Angabe fehlt.")


def test_P2_bestaetigter_vg_gibt_die_echte_zahl():
    """Gegenprobe zur Sperre: DERSELBE Betrag bestaetigt ergibt eine Zahl -- und eine andere als
    P0. Ohne diesen Kontrollfall waere 'gesperrt' auch mit einer kaputten Engine gruen."""
    f = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                          "rentner_freibetrag_erstmalig": True,
                          "rentner_veraeusserungsgewinn": 10000000})
    assert _ergebnis("rentner_gesamt", f)[:2] == ("bestaetigt", 8227000)


def test_die_luecke_ist_die_differenz_die_fehlt():
    """Die Probe P quantifiziert: P2 - P0 = 23.100 EUR."""
    ohne = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                             "rentner_freibetrag_erstmalig": True})
    mit = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                            "rentner_freibetrag_erstmalig": True,
                            "rentner_veraeusserungsgewinn": 10000000})
    p0 = _ergebnis("rentner_gesamt", ohne)[1]
    p2 = _ergebnis("rentner_gesamt", mit)[1]
    assert p2 - p0 == 2310000, f"Erwartet 23.100 EUR Differenz, gemessen {p2 - p0} Cent."


# ===================== Zweiter Fall auf der Scheibe "gesamt" =====================

def test_zweiter_fall_gesamt_vorlaeufiger_gewinn_sperrt():
    """Zweite Scheibe, zweites Feld: einkuenfte_gewinn (§§ 13-18) vorlaeufig auf "gesamt"."""
    f = _felder(GESAMT, {"einkuenfte_gewinn": (5000000, V)})
    status, grund, offen = _ergebnis("gesamt", f)
    assert status == "GESPERRT"
    assert "einkuenfte_gewinn" in offen


def test_zweiter_fall_gesamt_kontrolle_bleibt_bestaetigt():
    """Kontrolle zum zweiten Fall: derselbe Fall ohne das Feld bleibt eine Zahl."""
    f = _felder(GESAMT, {})
    assert _ergebnis("gesamt", f)[0] == "bestaetigt"


# ===================== /preflight darf nicht GREEN melden =====================
# Die Sperre allein genuegt nicht: /preflight ist der Ort, an dem der Nutzer VOR dem Absenden
# sieht, was fehlt. Gemessen war er fuer P1 gruen ("hier gibt es nichts zu wissen") — genau die
# Aussage, die nicht stimmt. AMBER, nicht RED: es widerspricht sich nichts, die Erklärung ist in
# Ordnung; nur die angezeigte Zahl bildet einen genannten Betrag noch nicht ab.

def _preflight(f: dict) -> dict:
    import preflight as PF
    return PF.preflight(f, TR.lade_bindung())


def test_preflight_ist_fuer_p1_nicht_gruen():
    f = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                          "rentner_freibetrag_erstmalig": True,
                          "rentner_veraeusserungsgewinn": (10000000, V)})
    r = _preflight(f)
    assert r["status"] == "AMBER", (
        f"/preflight meldet {r['status']} fuer einen vorlaeufigen Betrag, der die Steuer bewegt.")
    assert any(e["feld_id"] == "rentner_veraeusserungsgewinn"
               for e in r["hinweise_betrag_vorlaeufig"]), (
        "AMBER, aber ohne den Betrag zu nennen -- der Nutzer erfaehrt nicht, was fehlt.")


def test_preflight_kontrollen_bleiben_gruen():
    """Ohne diese Kontrollen waere 'nicht gruen' auch mit einer Meldung gruen, die IMMER feuert."""
    ohne = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                             "rentner_freibetrag_erstmalig": True})
    mit = _felder(RENTNER, {"rentner_alter_55_oder_berufsunfaehig": True,
                            "rentner_freibetrag_erstmalig": True,
                            "rentner_veraeusserungsgewinn": 10000000})
    assert _preflight(ohne)["status"] == "GREEN"
    assert _preflight(mit)["status"] == "GREEN"
    assert _preflight(ohne)["hinweise_betrag_vorlaeufig"] == []


def test_preflight_meldet_keinen_int_wert_als_eurobetrag():
    """geburtsjahr ist typ=int und steht in RING_BETRAGSFELDER. Der Hinweissatz nennt einen
    EURO-Betrag -- ein int-Feld darf ihn nicht ausloesen ('Du hast 19 € eingetragen')."""
    f = _felder(RENTNER, {"geburtsjahr": (1960, V)})
    assert _preflight(f)["hinweise_betrag_vorlaeufig"] == []


# ===================== Die Texte behaupten keine Richtung =====================
# Klartext und Preflight-Hinweis sagen dem Nutzer, WAS noch nicht bestaetigt ist. Sie sagen NICHT,
# in welche Richtung sich die Zahl dadurch bewegt -- und sie verweisen nicht auf eine Liste.
#
# Warum die Richtung in BEIDE Richtungen falsch waere: ein vorlaeufiger agB-Abzug
# (agb_aufwendungen, 5.000 EUR) SENKT die Steuer nach der Bestaetigung, eine vorlaeufige
# Lohnsteuer-Anrechnung (p36_lohnsteuer) HEBT die Abschlusszahlung. Ein Satz, der "zu niedrig"
# behauptet, ist fuer das eine Feld richtig und fuer das andere falsch -- und der Hinweis ist fuer
# alle Felder derselbe Text.
#
# Warum "die Liste daneben" nicht stimmt: zeigeErgebnis() zeigt den Klartext genau dann, wenn
# zahl_cent === null; die Liste der offenen Angaben steht im ERFOLGS-Zweig daneben und wird in
# diesem Fall nie gezeichnet (produkt/haut/static/app.js, zwei sich ausschliessende Zweige).
#
# Der Test pinnt BEIDES: das Verbotene fehlt UND der Auftrag steht drin. Ohne den zweiten Teil
# waere "Satz gestrichen" ein gruener Fix.

VERBOTEN = ("zu niedrig", "zu hoch", "passt nicht", "nicht passt", "Liste daneben")


def test_sperrgrund_klartext_behauptet_keine_richtung():
    text = BD.sperrgrund_klartext("ring_betrag_vorlaeufig")
    for wort in VERBOTEN:
        assert wort not in text, f"Klartext behauptet eine Richtung ('{wort}'): {text}"
    assert "bestätig" in text.lower(), (
        f"Der Klartext muss sagen, was zu tun ist (bestaetigen): {text}")


@pytest.mark.parametrize("feld_id,wert", [
    ("p36_lohnsteuer", 3000000),
    ("p36_kapitalertragsteuer", 135680),
    ("agb_aufwendungen", 500000),
])
def test_preflight_hinweis_behauptet_keine_richtung(feld_id, wert):
    f = _felder(GESAMT, {feld_id: (wert, V)})
    treffer = [e for e in _preflight(f)["hinweise_betrag_vorlaeufig"] if e["feld_id"] == feld_id]
    assert treffer, (
        f"{feld_id} ist vorlaeufig, aber /preflight meldet nichts -- der Nutzer sieht nicht, "
        "dass der Betrag in der Zahl fehlt.")
    text = treffer[0]["hinweis"]
    for wort in VERBOTEN:
        assert wort not in text, f"Preflight-Hinweis behauptet eine Richtung ('{wort}'): {text}"
    assert "bestätig" in text.lower(), (
        f"Der Hinweis muss sagen, was zu tun ist (bestaetigen): {text}")
