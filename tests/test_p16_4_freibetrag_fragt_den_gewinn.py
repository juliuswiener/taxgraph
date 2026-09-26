"""Ein „nein" zum Freibetrag laesst die Gewinnfrage stehen (backlog
betriebsverkauf-unter-55-fragt-den-gewinn-nie, 2026-09-26).

`p16_4_freibetrag` traegt ZWEI verschiedene Dinge unter EINER regel_id: die Gewinnfrage
(`rentner_veraeusserungsgewinn`, ein signatur_slot) und die Freibetrags-Voraussetzungen
(§ 16 Abs. 4 S. 1-2 EStG, vier askable bool-Gates je Person). Ein bestaetigtes „nein" auf
eine Voraussetzung schloss ueber `relevanz()` die GANZE Regel aus — und nahm damit die
Gewinnfrage mit. Gemessen an HEAD: `alter55=False` -> 0 von 6 p16_4-Feldern in der Queue;
ein Veraeusserungsgewinn von 100.000 EUR wurde gar nicht erst gefragt (42.000 EUR Steuer).

Zweite Wirkung derselben Ursache: die Sperre `p16_4_gate_offen` wartet auf die BESTAETIGUNG
genau dieser zwei Bools. Die Ausschlussregel loeschte die Fragen, die die Sperre heben —
ein Zustand ohne Rueckweg.

Die Voraussetzungen sind Deklaration, kein Gate (Entscheidung
decisions/nein-zum-freibetrag-laesst-die-gewinnfrage-stehen.md, 2026-09-26): ein Gate
schaltet zwischen zwei RECHENWEGEN um (`antrag_ermaessigter_satz` waehlt zwischen zwei Kz).
§ 16 Abs. 4 S. 1 rechnet in beiden Faellen gleich — die Antwort entfernt nur einen Abzug,
und DIESE Entscheidung liegt in bescheid_zweige.py/`_an_gesamt_sperrgrund`, nicht in der
Ausschlusslogik des Traversers. Zweimal dieselbe Entscheidung im Code, und die zweite
Fassung frisst den Rueckweg.

Reparatur: `gate: false` auf den vier Voraussetzungen. Die Regel bleibt stehen, alle 6
Felder werden weiter gefragt, die Sperre kann greifen. `relevanz()` wird NICHT allgemein
aufgeweicht — der Rechenpfad bleibt unberuehrt und entscheidet den Freibetrag wie bisher.

Die Gegenprobe (alter55=True -> Freibetrag wird gewaehrt) steht daneben: ohne sie ist
„stehen geblieben" von „Abzug verloren" nicht zu unterscheiden.

NULL LLM, nur Speicher."""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/traverser", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, _sub))
import traverser as T  # noqa: E402
import store as ST     # noqa: E402

TS = "2026-09-26T12:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
B = T.lade_bindung()

RID = "p16_4_freibetrag"
GEWINN = "rentner_veraeusserungsgewinn"
ALTER = "rentner_alter_55_oder_berufsunfaehig"
ALTER_P = "rentner_alter_55_oder_berufsunfaehig_partner"
ERST = "rentner_freibetrag_erstmalig"
ERST_P = "rentner_freibetrag_erstmalig_partner"
VOR = (ALTER, ALTER_P, ERST, ERST_P)
P16_FELDER = sorted(f for f, b in B.items() if b["quelle"]["regel_id"] == RID)


def _store(antworten: dict):
    s = ST.leerer_store(2025)
    for fid, wert in antworten.items():
        ST.append_event(s, feld_id=fid, wert=wert, zustand="bestaetigt", herkunft=H, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"klick@{fid}"}, ts=TS, bindung=B)
    return s


def _fragen(antworten: dict) -> set:
    return set(T.naechste_fragen(_store(antworten), B))


# ---- AK1: die Gewinnfrage bleibt stehen, wenn eine Voraussetzung „nein" sagt ----------------

@pytest.mark.parametrize("vor", [ALTER, ERST], ids=["alter55", "erstmalig"])
def test_gewinnfrage_bleibt_stehen_wenn_voraussetzung_nein(vor):
    """ROT an HEAD: `alter55=False` -> p16_4_freibetrag ausgeschlossen, 0 von 6 Feldern in
    der Queue. Der Nutzer wird nach seinem 100.000-EUR-Veraeusserungsgewinn nie gefragt."""
    frage = _fragen({vor: False})
    assert T.relevanz(_store({vor: False}), B)[RID]["status"] != "ausgeschlossen"
    assert GEWINN in frage, (
        f"{vor}=False nimmt die Gewinnfrage aus der Warteschlange. § 16 Abs. 1 EStG rechnet den "
        f"Veraeusserungsgewinn in beiden Faellen gleich — die Antwort entfernt nur den Freibetrag.")


def test_alle_p16_4_felder_bleiben_in_der_queue():
    """Nicht nur die Gewinnfrage: § 16 Abs. 4 S. 1-2 verlangt die vier Angaben, und die Sperre
    `p16_4_gate_offen` kann nur greifen, solange sie gestellt werden (sonst kein Rueckweg).
    Die beantwortete Voraussetzung selbst faellt dabei korrekt aus der Queue (sie IST beantwortet)."""
    fehlend = [f for f in P16_FELDER if f != ALTER and f not in _fragen({ALTER: False})]
    assert not fehlend, f"alter55=False nimmt {fehlend} aus der Queue"


def test_voraussetzung_bleibt_fragbar():
    """`gate: false` darf das Feld nicht stillegen — der Vordruck verlangt die Angabe.

    Ohne diese Haelfte waere der bequemste Weg durch die Tests oben, das Feld auf
    `askable: false` zu setzen: die Regel bliebe stehen, die Angabe fehlte im XML, und
    checkESt fiele erst Wochen spaeter darueber (dieselbe Begruendung wie in
    tests/test_gate_polaritaet.py:test_deklarationsfeld_wird_weiter_gefragt).
    """
    for f in VOR:
        assert B[f].get("askable") is True, f"{f} wird nicht mehr gefragt."
        assert B[f].get("gate") is False, f"{f} traegt kein `gate: false` mehr — dann ist es ein Gate."
    assert set(VOR) <= _fragen({}), "die unbeantworteten Voraussetzungen fehlen in der Queue"


# ---- AK2: die Voraussetzungen des Partners loeschen die eigene Gewinnfrage nicht ------------

def test_partner_gate_loescht_die_eigene_gewinnfrage_nicht():
    """§ 26b EStG ermittelt die Einkuenfte je Ehegatte; § 16 Abs. 4 gilt PRO Person. Das
    Partner-Gate darf hoechstens den PARTNER-Freibetrag kosten."""
    frage = _fragen({ALTER_P: False, ERST_P: False})
    assert GEWINN in frage, "das Partner-Gate nimmt die eigene Gewinnfrage mit"
    assert ALTER in frage, "das Partner-Gate nimmt die eigene Voraussetzung mit"


# ---- AK4-Gegenprobe aus AK1: der Normalfall aendert sich nicht -----------------------------

def test_ja_laesst_die_regel_stehen():
    """Die Kontrolle. Ohne sie ist „stehen geblieben" von „Abzug verloren" nicht zu
    unterscheiden — und der Freibetrag wird im Rechenpfad aus dem WERT gewaehrt, nicht aus
    dem Relevanz-Status (tests/test_gate_naht_guard_liest_zustand.py)."""
    rel = T.relevanz(_store({ALTER: True, ERST: True}), B)[RID]
    assert rel["status"] != "ausgeschlossen", rel
    assert GEWINN in _fragen({ALTER: True, ERST: True})


# ---- kein Rueckfall in die verbotene allgemeine Aufweichung ---------------------------------

def test_relevanz_weicht_nicht_allgemein_auf():
    """Die Entscheidung verbietet ausdruecklich einen allgemeinen Umbau des Ausschlusses:
    ein echtes Gate muss weiter ausschliessen. Kontrollfall aus dem Nachbarmodul."""
    s = _store({"kind_unter_14_haushaltszugehoerig": False})
    assert T.relevanz(s, B)["p10_1_5_kinderbetreuung"]["status"] == "ausgeschlossen"
    assert "kinderbetreuungskosten" not in _fragen({"kind_unter_14_haushaltszugehoerig": False})
