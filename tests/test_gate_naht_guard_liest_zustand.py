"""Naht-Fix gate-naht-guard-liest-zustand: Guard (_an_gesamt_sperrgrund) und Ring (_bescheid_fn)
lasen bisher unterschiedliche Dinge -- der Guard den Rohwert, der Ring nur `zustand=bestaetigt`
(bei nur_bestaetigt=True, dem /ergebnis-Pfad). Drei Fehlerklassen, je einmal reproduziert:

  A (under-tax, P1 § 16 Abs. 4 Freibetrag): ein VORLAEUFIGES True bestand den Guard (der nur den
    Rohwert prüfte) und der Ring gewährte den FB unconditional, sobald der Fall durchkam -- ein
    unbestätigter Gate-Bool kostete real Geld. Gegenprobe: ein BESTÄTIGTES False wurde vorher
    trotzdem gesperrt (der Guard prüfte den AUSGANG der Entscheidung statt, OB sie getroffen
    wurde) -- eine ehrliche, abschließende Absage verdient ein echtes Ergebnis, keine Dauer-Sperre.

  B (silent over-tax, P6 doppelte Haushaltsführung Inland): ein VORLAEUFIGES True bestand den
    Guard, aber der Ring filtert auf bestätigt und ließ den Werbungskostenabzug klanglos fallen --
    dieselbe Erklärung kostete plötzlich mehr Steuer, ohne dass irgendetwas das meldete.

  C (fail-open, P2 Verpflegung 3-Monats-Frist): fehlte vpf_monate_am_ort komplett, prüfte der
    Guard es überhaupt nicht (kein isinstance-Check auf None) -- der Ring rechnete mit der
    impliziten Annahme "≤ 3 Monate" weiter, ohne dass die je bestätigt war.

Direkter Aufruf wie in tests/test_gewinn_partner_ring.py: BD._an_gesamt_sperrgrund +
API._bescheid_fn auf einem synthetischen felder-Dict, ohne HTTP/Store -- schnell, exakt.

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
            "produkt/import"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API                        # noqa: E402
import bescheid_deklaration as BD        # noqa: E402
import store as ST                       # noqa: E402
import traverser as TR                   # noqa: E402
from api_constants import SCHEIBEN       # noqa: E402

VZ = 2025
V = "vorlaeufig"


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _felder(basis: list[tuple], extra: dict) -> dict:
    """basis: [(feld, wert), ...], alle bestätigt. extra: feld -> wert | (wert, zustand) | None (fehlt)."""
    f = {k: {"wert": v, "zustand": "bestaetigt"} for k, v in basis}
    for k, v in extra.items():
        if v is None:
            continue
        w, z = v if isinstance(v, tuple) else (v, "bestaetigt")
        f[k] = {"wert": w, "zustand": z}
    return f


def _ergebnis(scheibe: str, f: dict) -> tuple[str, str | int]:
    """("GESPERRT", sperrgrund) oder ("bestaetigt", zahl_cent) -- 1:1 zu api._ergebnis_roh."""
    cfg = SCHEIBEN[scheibe]
    kegel = cfg["kegel"]
    sperr = BD._an_gesamt_sperrgrund(f, cfg, VZ, None, None)
    if sperr:
        return ("GESPERRT", sperr)
    zust = [f[k]["zustand"] for k in kegel if k in f]
    if len(zust) < len(kegel) or ST.meet_zustand(zust) != "bestaetigt":
        return ("GESPERRT", "input_kegel_nicht_bestaetigt")
    bindung = TR.lade_bindung()
    bf = API._bescheid_fn(cfg["gesamt_ring"], VZ, bindung, f, store=None, nur_bestaetigt=True)
    return ("bestaetigt", bf({k: f[k]["wert"] for k in kegel}))


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
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0), ("kap_verlust_aktien", 0),
    ("kap_verlust_sonstige", 0),
]


# ---- Klasse A: under-tax -- § 16 Abs. 4 Freibetrag (P1, rentner_gesamt) -----------------------

_VG = {"rentner_veraeusserungsgewinn": 10000000}   # 100.000 EUR, FB waere 45.000 EUR


def test_p16_4_vorlaeufige_bools_sperren_statt_freibetrag_zu_gewaehren():
    """VORHER: ein vorläufiges True bestand den rohen `is True`-Check, der Ring gewährte den FB
    unconditional -- 82.270 EUR, obwohl KEINE Bedingung bestätigt war (under-tax). NACHHER:
    p16_4_gate_offen, kein stiller Betrag."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _felder(RENTNER, {**_VG, "rentner_alter_55_oder_berufsunfaehig": (True, V),
                           "rentner_freibetrag_erstmalig": (True, V)})
    ergebnis = _ergebnis("rentner_gesamt", f)
    assert ergebnis == ("GESPERRT", "p16_4_gate_offen"), ergebnis


def test_p16_4_bestaetigtes_nein_liefert_echte_zahl_statt_dauersperre():
    """Kehrseite desselben Fixes: ein BESTÄTIGTES False ist eine abschließende Antwort (kein FB),
    keine offene Frage -- der Guard darf hier nicht mehr sperren. 101.170 EUR = voller vg
    (100.000 EUR) ohne FB, mehr als die 82.270 EUR MIT FB (s. Referenztest unten)."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _felder(RENTNER, {**_VG, "rentner_alter_55_oder_berufsunfaehig": False,
                           "rentner_freibetrag_erstmalig": False})
    grund, cent = _ergebnis("rentner_gesamt", f)
    assert grund == "bestaetigt", (grund, cent)
    assert cent == 10117000, f"erwartet 101.170,00 EUR ohne FB, tatsächlich {cent / 100:,.2f} EUR"


def test_p16_4_bestaetigtes_ja_liefert_weiterhin_den_freibetrag():
    """Gegenprobe (kein Over-Fix): beide Bools bestätigt True liefert weiterhin 82.270 EUR MIT FB."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _felder(RENTNER, {**_VG, "rentner_alter_55_oder_berufsunfaehig": True,
                           "rentner_freibetrag_erstmalig": True})
    grund, cent = _ergebnis("rentner_gesamt", f)
    assert grund == "bestaetigt", (grund, cent)
    assert cent == 8227000, f"erwartet 82.270,00 EUR mit FB, tatsächlich {cent / 100:,.2f} EUR"


# ---- Klasse B: silent over-tax -- doppelte Haushaltsführung Inland (P6, gesamt) ---------------

_DHF = {"dhf_unterkunftskosten_monat": 100000, "dhf_monate": 12, "dhf_beruflich_veranlasst": True,
        "dhf_eigener_hausstand": True, "dhf_finanzielle_beteiligung": True}


def test_dhf_im_inland_vorlaeufig_sperrt_statt_abzug_zu_verschlucken():
    """VORHER: dhf_im_inland VORLAEUFIG True bestand den Guard (Rohwert True), aber der Ring
    filtert auf bestätigt und sah das Feld dann als fehlend -- 13.924 EUR statt der 9.976 EUR MIT
    Abzug, Differenz 3.948 EUR ohne jede Meldung (silent over-tax). NACHHER: dhf_tatbestand_offen."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _felder(GESAMT, {**_DHF, "dhf_im_inland": (True, V)})
    ergebnis = _ergebnis("gesamt", f)
    assert ergebnis == ("GESPERRT", "dhf_tatbestand_offen"), ergebnis


def test_dhf_im_inland_bestaetigt_liefert_den_abzug():
    """Gegenprobe (kein Over-Fix): bestätigtes True liefert weiterhin 9.976 EUR MIT dHf-Abzug."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _felder(GESAMT, {**_DHF, "dhf_im_inland": True})
    grund, cent = _ergebnis("gesamt", f)
    assert grund == "bestaetigt", (grund, cent)
    assert cent == 997600, f"erwartet 9.976,00 EUR mit dHf-Abzug, tatsächlich {cent / 100:,.2f} EUR"


# ---- Klasse C: fail-open -- Verpflegung 3-Monats-Frist (P2, gesamt) ----------------------------

_VPF = {"tage_24h": 200, "tage_an_abreise": 0, "tage_ueber_8h_eintaegig": 0,
        "vpf_keine_mahlzeitengestellung": True}


def test_vpf_monate_am_ort_fehlt_sperrt_statt_stillschweigend_volle_pauschale():
    """VORHER: fehlte vpf_monate_am_ort komplett, prüfte der Guard das gar nicht (kein
    isinstance-Check auf None) -- 12.272 EUR, obwohl die implizite Annahme "≤ 3 Monate" nie
    bestätigt war (fail-open). NACHHER: verpflegung_dreimonatsfrist_aufteilung_offen."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _felder(GESAMT, dict(_VPF))
    ergebnis = _ergebnis("gesamt", f)
    assert ergebnis == ("GESPERRT", "verpflegung_dreimonatsfrist_aufteilung_offen"), ergebnis


def test_vpf_monate_am_ort_bestaetigt_liefert_die_pauschale():
    """Gegenprobe (kein Over-Fix): bestätigte 2 Monate (≤ 3) liefern weiterhin die volle
    Pauschale, 12.272 EUR."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    f = _felder(GESAMT, {**_VPF, "vpf_monate_am_ort": 2})
    grund, cent = _ergebnis("gesamt", f)
    assert grund == "bestaetigt", (grund, cent)
    assert cent == 1227200, f"erwartet 12.272,00 EUR volle Pauschale, tatsächlich {cent / 100:,.2f} EUR"
