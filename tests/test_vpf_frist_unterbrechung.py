"""
Tests für vpf_frist_nicht_unterbrochen Sperrgrund-Logik.

§ 9 Abs. 4a S. 6-7: Verpflegungspauschale auf 3 Monate beschränkt, außer wenn
Unterbrechung ≥4 Wochen Frist neu setzt.

Sperrgrund verpflegung_dreimonatsfrist_unterbrechung_offen:
- Bedingung: monate > 3 + Tage_gesamt > 0 + Tage_nach_frist_gesamt == 0 + Frage unbeantwortet
- Effekt: _an_gesamt_sperrgrund() gibt Sperrgrund-Name zurück (not None)
"""
from __future__ import annotations

import sys
import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt/haut"))

import api


def test_vpf_frist_unterbrechung_unbeantwortet_sperrgrund():
    """
    Fall 1: monate=5, tage_24h=60, NACH_FRIST alle=0 bestaetigt, vpf_frist_nicht_unterbrochen NICHT gesetzt
    -> grund == "verpflegung_dreimonatsfrist_unterbrechung_offen"
    """
    felder = {
        "tage_24h": {"wert": 60, "zustand": "bestaetigt"},
        "tage_an_abreise": {"wert": 0, "zustand": "bestaetigt"},
        "tage_ueber_8h_eintaegig": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_monate_am_ort": {"wert": 5, "zustand": "bestaetigt"},
        "vpf_keine_mahlzeitengestellung": {"wert": True, "zustand": "bestaetigt"},
        "vpf_tage_24h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_an_abreise_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_ueber_8h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        # vpf_frist_nicht_unterbrochen: NICHT gesetzt (zustand != "bestaetigt")
    }
    scheibe = {"guard": True}
    vz = 2025

    grund = api._an_gesamt_sperrgrund(felder, scheibe, vz, None, None)

    assert grund == "verpflegung_dreimonatsfrist_unterbrechung_offen", \
        f"Expected Sperrgrund, got grund={grund}"


def test_vpf_frist_unterbrechung_bestaetigt_rechnet():
    """
    Fall 2: monate=5, tage_24h=60, NACH_FRIST alle=0 bestaetigt, vpf_frist_nicht_unterbrochen bestaetigt=True
    -> grund is None (Ring rechnet)
    """
    felder = {
        "tage_24h": {"wert": 60, "zustand": "bestaetigt"},
        "tage_an_abreise": {"wert": 0, "zustand": "bestaetigt"},
        "tage_ueber_8h_eintaegig": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_monate_am_ort": {"wert": 5, "zustand": "bestaetigt"},
        "vpf_keine_mahlzeitengestellung": {"wert": True, "zustand": "bestaetigt"},
        "vpf_tage_24h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_an_abreise_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_ueber_8h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        # vpf_frist_nicht_unterbrochen: BESTAETIGT
        "vpf_frist_nicht_unterbrochen": {"wert": True, "zustand": "bestaetigt"},
    }
    scheibe = {"guard": True}
    vz = 2025

    grund = api._an_gesamt_sperrgrund(felder, scheibe, vz, None, None)

    assert grund is None, \
        f"Expected no Sperrgrund (rechnet), got grund={grund}"


def test_vpf_normal_path_mit_nach_frist_nicht_blockiert():
    """
    Fall 3 (Regression): monate=5, tage_24h=60, vpf_tage_24h_nach_drei_monaten=15 bestaetigt
    -> grund is None (Sperrgrund darf NICHT feuern, normaler Weg nicht blockiert)

    Wichtigste Regression: die neue Sperrgrund-Bedingung "tage_nach_frist_gesamt == 0"
    darf NICHT feuern, wenn Nutzer die Aufteilung korrekt angegeben hat.
    """
    felder = {
        "tage_24h": {"wert": 60, "zustand": "bestaetigt"},
        "tage_an_abreise": {"wert": 0, "zustand": "bestaetigt"},
        "tage_ueber_8h_eintaegig": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_monate_am_ort": {"wert": 5, "zustand": "bestaetigt"},
        "vpf_keine_mahlzeitengestellung": {"wert": True, "zustand": "bestaetigt"},
        # NACH_FRIST: 15 Tage (nicht 0) → Sperrgrund darf NICHT feuern
        "vpf_tage_24h_nach_drei_monaten": {"wert": 15, "zustand": "bestaetigt"},
        "vpf_tage_an_abreise_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        "vpf_tage_ueber_8h_nach_drei_monaten": {"wert": 0, "zustand": "bestaetigt"},
        # vpf_frist_nicht_unterbrochen: NICHT gesetzt (nicht nötig, Aufteilung ist korrekt)
    }
    scheibe = {"guard": True}
    vz = 2025

    grund = api._an_gesamt_sperrgrund(felder, scheibe, vz, None, None)

    assert grund is None, \
        f"Expected no Sperrgrund (normal path), got grund={grund}. " \
        "Sperrgrund darf NICHT feuern wenn nach_frist > 0"


# ---- Traverser-Seite: Polaritaet des Gates ---------------------------------------

def _bindung():
    import glob
    import yaml
    bind = {}
    for f in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
        for b in yaml.safe_load(open(f))["bindungen"]:
            bind[b["feld_id"]] = b
    return bind


def _relevanz_mit(wert):
    """p9_4a-Status, wenn die uebrigen Gates bejaht sind und das Fristfeld `wert` hat."""
    sys.path.insert(0, ROOT)
    from produkt.traverser import traverser as T
    events = [{"event_id": "e0", "feld_id": "vpf_auswaertige_taetigkeit",
               "wert": True, "zustand": "bestaetigt"},
              {"event_id": "e1", "feld_id": "vpf_keine_mahlzeitengestellung",
               "wert": True, "zustand": "bestaetigt"}]
    if wert is not None:
        events.append({"event_id": "e2", "feld_id": "vpf_frist_nicht_unterbrochen",
                       "wert": wert, "zustand": "bestaetigt"})
    store = {"events": events}
    bind = _bindung()
    rel = T.relevanz(store, bind)["p9_4a_verpflegungsmehraufwand"]
    fragen = T.naechste_fragen(store, bind)
    return rel, [f for f in fragen if f.startswith(("vpf", "tage"))]


def test_vpf_frist_normalfall_schliesst_nicht_aus():
    """Der NORMALFALL — durchgehend am selben Ort — darf p9_4a nicht ausschliessen.

    Das Feld ist ein Gate: traverser.py:96 setzt `ausgeschlossen`, sobald ein askable
    bool-Geltungsbedingungsfeld `false` ist. Es kommt also darauf an, welche Antwort der
    Normalfall gibt. Bis 2026-08-07 hiess das Feld vpf_frist_unterbrochen und fragte "War
    die Taetigkeit unterbrochen?" — der Normalfall antwortete wahrheitsgemaess `false` und
    verlor damit den gesamten Verpflegungsmehraufwand (gemessen: status ausgeschlossen,
    0 statt 16 offene vpf-Fragen). § 9 Abs. 4a S. 7 ist aber eine Ausnahme ZUGUNSTEN des
    Steuerpflichtigen; ohne Unterbrechung laeuft die Frist einfach weiter und die Regel gilt.

    Jetzt fragt vpf_frist_nicht_unterbrochen nach dem Nicht-Unterbrochensein: Normalfall
    antwortet `true`, Gate bleibt offen.
    """
    rel, _ = _relevanz_mit(True)
    assert rel["status"] == "relevant", (
        f"Normalfall (durchgehend, vpf_frist_nicht_unterbrochen=True) ergibt "
        f"status={rel['status']} statt relevant — Polaritaet des Gates verdreht.")
    rel_offen, _ = _relevanz_mit(None)
    assert rel_offen["status"] != "ausgeschlossen", (
        f"unbeantwortet darf nicht ausschliessen, status={rel_offen['status']}")


def test_vpf_frist_normalfall_streicht_keine_fragen():
    """Der Normalfall darf die Verpflegungs-Fragen nicht abschneiden.

    Gegenstueck zum Status-Test: ein Gate auf `false` leert nicht nur den Status, es raeumt
    auch die Interview-Queue der Regel — der Nutzer wurde nie wieder nach seinen Reisetagen
    gefragt. Mit richtiger Polaritaet passiert das nur noch im Sonderfall (unterbrochen),
    und dort ist es richtig: dann laeuft die Frist neu und S. 6 greift nicht.
    """
    _, fragen_ohne = _relevanz_mit(None)
    assert fragen_ohne, "Vorbedingung kaputt: ohne Antwort muss es offene vpf-Fragen geben"
    _, fragen_normal = _relevanz_mit(True)
    assert len(fragen_normal) >= len(fragen_ohne) - 1, (
        f"Normalfall streicht vpf-Fragen: {len(fragen_ohne)} -> {len(fragen_normal)}")


def test_vpf_frist_feld_bleibt_askable():
    """askable: false waere ein Deadlock — der Guard sperrt auf ein Feld, das nie gefragt wird.

    Gemessen 2026-08-07: mit askable: false liefert _an_gesamt_sperrgrund weiterhin
    verpflegung_dreimonatsfrist_unterbrechung_offen (api.py:1721 liest das Feld direkt),
    waehrend naechste_fragen es nicht mehr anbietet. Der Fall ist dann unaufloesbar.
    """
    b = _bindung()["vpf_frist_nicht_unterbrochen"]
    assert b.get("askable") is True, (
        "vpf_frist_nicht_unterbrochen muss askable bleiben, sonst sperrt der Guard in "
        "_dhf_vpf_grund auf eine Frage, die das Interview nicht mehr stellt (Deadlock).")
