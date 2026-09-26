"""Eine Bedingung auf einem Feld mit Instanzen wird je Instanz ausgewertet (backlog
feld-bedingung-liest-nur-instanz-1, 2026-09-26).

Bis zum 2026-09-26 las jede Nachschlagestelle des Traversers — Gate in relevanz(), regel_bedingung,
feld_bedingung — nur Instanz 1 (`feld`), nie `feld__2`. Gemessen (`tri-sperren`): zwei Kinder,
Kind 1 „unter 14" nein, Kind 2 ja → die ganze Kinderbetreuung ausgeschlossen, 0 Betreuungsfragen
für Kind 2. Getauscht (Kind 1 ja, Kind 2 nein) kamen sie — die REIHENFOLGE der Kinder entschied
über bis zu 4.800 EUR Abzug (§ 10 Abs. 1 Nr. 5 EStG, „höchstens 4 800 Euro je Kind").

Reparatur einmal an der Nachschlagestelle (decisions/instanz-schleife-an-der-nachschlagestelle.md),
nicht je Feld: ausgeschlossen erst, wenn JEDE Instanz bestätigt abweicht; eine angekündigte, noch
leere Instanz ist offen und schließt nie aus (fail-closed).

Die Kontrollfälle (alle Kinder „nein" → ausgeschlossen) stehen daneben, weil eine Reparatur, die
nie mehr ausschließt, dieselben Tests grün machte.

NULL LLM, nur Speicher."""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/traverser", "produkt/store", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, _sub))
import traverser as T  # noqa: E402
import store as ST     # noqa: E402

TS = "2026-09-26T12:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
B = T.lade_bindung()

KIND = ("fam_anzahl_kinder", "kind_unter_14_haushaltszugehoerig", "p10_1_5_kinderbetreuung")
GWG = ("gwg_anzahl", "gwg_bewegliches_selbstaendig_nutzbar", "p6_2_gwg_sofortabzug")


def _store(antworten: dict):
    s = ST.leerer_store(2025)
    for fid, wert in antworten.items():
        ST.append_event(s, feld_id=fid, wert=wert, zustand="bestaetigt", herkunft=H, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"ok@{fid}"}, ts=TS, bindung=B)
    return s


def _ev(wert):
    return {"wert": wert, "zustand": "bestaetigt"}


def _regel_fragen(s, rid: str) -> list:
    return [f for f in T.naechste_fragen(s, B) if B[f]["quelle"]["regel_id"] == rid]


# ---- feld_bedingung: die vier Fälle des Tickets, direkt an der Funktion ----------------------

@pytest.mark.parametrize("aktiv, soll", [
    ({"b": _ev(True)}, True),                          # I1 ja, keine I2: ausgeschlossen
    ({"b": _ev(True), "b__2": _ev(False)}, False),     # I1 ja, I2 nein: Instanz 2 braucht die Frage
    ({"b": _ev(False)}, False),                        # I1 nein
    ({"b__2": _ev(False)}, False),                     # I1 offen, I2 nein
])
def test_feld_bedingung_liest_jede_instanz(aktiv, soll):
    eintrag = {"feld_bedingung": {"feld": "b", "wert": False}}
    assert T._feld_ausgeschlossen(eintrag, aktiv) is soll


# ---- Gate in relevanz(): echte Bindung, beide Reihenfolgen ------------------------------------

@pytest.mark.parametrize("anzahl, gate, rid", [KIND, GWG], ids=["kind", "gwg"])
def test_nein_bei_instanz_1_nimmt_instanz_2_die_fragen_nicht(anzahl, gate, rid):
    nein_ja = _store({anzahl: 2, gate: False, f"{gate}__2": True})
    ja_nein = _store({anzahl: 2, gate: True, f"{gate}__2": False})
    assert T.relevanz(nein_ja, B)[rid]["status"] != "ausgeschlossen"
    fragen = _regel_fragen(nein_ja, rid)
    assert fragen, f"{rid}: Instanz 2 qualifiziert, aber keine einzige Frage der Regel in der Queue"
    assert fragen == _regel_fragen(ja_nein, rid), "die Reihenfolge der Instanzen entscheidet"


def test_kinderbetreuung_fragt_den_betrag_fuer_kind_2():
    s = _store({"fam_anzahl_kinder": 2, "kind_unter_14_haushaltszugehoerig": False,
                "kind_unter_14_haushaltszugehoerig__2": True})
    assert "kinderbetreuungskosten" in T.naechste_fragen(s, B)


def test_angekuendigte_leere_instanz_schliesst_nicht_aus():
    """Kind 1 „nein", Kind 2 noch leer: die Frage für Kind 2 muss wiederkommen. Schlösse das
    „nein" die Regel aus, fiele auch die Gate-Frage selbst aus der Queue — Kind 2 bekäme sie nie."""
    s = _store({"fam_anzahl_kinder": 2, "kind_unter_14_haushaltszugehoerig": False})
    assert T.relevanz(s, B)["p10_1_5_kinderbetreuung"]["status"] != "ausgeschlossen"
    assert "kind_unter_14_haushaltszugehoerig" in T.naechste_fragen(s, B)


@pytest.mark.parametrize("antworten", [
    {"fam_anzahl_kinder": 1, "kind_unter_14_haushaltszugehoerig": False},
    {"fam_anzahl_kinder": 2, "kind_unter_14_haushaltszugehoerig": False,
     "kind_unter_14_haushaltszugehoerig__2": False},
], ids=["ein-kind-nein", "zwei-kinder-nein"])
def test_kontrolle_jede_instanz_nein_schliesst_aus(antworten):
    s = _store(antworten)
    assert T.relevanz(s, B)["p10_1_5_kinderbetreuung"]["status"] == "ausgeschlossen"
    assert _regel_fragen(s, "p10_1_5_kinderbetreuung") == []


# ---- regel_bedingung: dieselbe Nachschlagestelle (heute kein Instanz-Feld, latent) ------------

def test_regel_bedingung_liest_jede_instanz(monkeypatch):
    monkeypatch.setattr(T, "lade_regel_bedingungen", lambda: {"r": [{"regel_id": "r", "feld": "b", "wert": True}]})
    bindung = {"x": {"feld_id": "x", "quelle": {"regel_id": "r"}, "askable": True}}
    s = {"events": [{"event_id": f"e-{f}", "feld_id": f, "wert": w, "zustand": "bestaetigt", "ersetzt": None}
                    for f, w in (("b", False), ("b__2", True))]}
    assert T.relevanz(s, bindung)["r"]["status"] != "ausgeschlossen"
    s["events"].pop()
    assert T.relevanz(s, bindung)["r"]["status"] == "ausgeschlossen"
