"""Der Vordruck-Anlass vom 2026-08-27 bleibt geschuetzt, auch wenn die Gewichte gewinnen.

ANLASS: `_nach_vordruck` sortiert die Felder mit ELSTER-Kennzahl nach der Reihenfolge des
amtlichen Formulars. GEMESSEN 2026-08-27 im Kinderfreibetrags-Block, wie Julius ihn im
Durchgang sah: die ersten vier Fragen galten dem ANDEREN Elternteil (Geburtsdatum, Verhaeltnis,
Name, Zeitraum), der Vorname des eigenen Kindes stand auf Platz 12 von 13. Das war der Grund,
warum ueberhaupt nach dem Vordruck sortiert wird.

2026-09-26 hat `_nach_vordruck` eine zweite Regel bekommen: liegt ein Unsicherheits-Beitrag aus
dem Rechenkern vor, bleibt die Klasse "wert" in der Gewichtsordnung stehen (Entscheidung
decisions/fragebogen-reihenfolge-gewichte-gegen-vordruck.md). Damit koennte der alte Befund
theoretisch wieder aufreissen — WENN die betroffenen Felder der Klasse "wert" angehoerten.

WARUM DIESER TEST TROTZDEM STEHT: dass es heute nicht passiert, ist eine Messung von heute,
kein Schutz fuer morgen. Der Test pinnt beide Haelften:
  H1 STRUKTUR: die vier Anlass-Felder tragen die Klasse "formal", die von der Gewichtsregel
     NICHT beruehrt wird — und ueber Klassengrenzen wird nie getauscht.
  H2 WIRKUNG: ihre Position aendert sich auch dann nicht, wenn ein beitrag vorliegt.
Faellt H1, ist die Begruendung weg; faellt H2, ist der Befund zurueck.

Der Kegel ist die VOLLBINDUNG mit leerem Store — dieselbe Naht wie
tests/test_fragen_gewichte_nach_relevanz.py:115, nur ohne API/catala.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/traverser", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402

BINDUNG = TR.lade_bindung()
GW = TR.gate_gewicht(BINDUNG)

# Der Vordruck-Anlass 2026-08-27, namentlich — damit ein Rueckbau auffaellt und nicht bloss
# der Sweep gruen bleibt.
ANLASS = {
    "kind_vorname": "E0500107 Vorname eigenes Kind",
    "kind_anderer_elternteil_name": "E0501103 Name anderer Elternteil",
    "kind_anderer_elternteil_geburtsdatum": "E0501104 Geburtsdatum anderer Elternteil",
    "kind_anderer_elternteil_kindschaftsverhaeltnis": "E0501106 Verhaeltnis anderer Elternteil",
}

# Gemessene Werte 2026-09-26 (die Sonde hinter der Entscheidung). Sie stehen hier als Zahlen,
# damit eine Aenderung der Ordnung auffaellt statt still durchzugehen.
BEITRAG = {"ep_arbeitstage": 507900, "ep_eigenes_kfz": 147400}


def _klasse(fid: str) -> str | None:
    if BINDUNG[fid].get("eingangsfrage"):
        return None
    if GW.get(fid, 0) > 0:
        return None
    return "formal" if "geltungsbedingung" in (BINDUNG[fid].get("quelle") or {}) else "wert"


def _thema(fid: str) -> str:
    return (BINDUNG[fid].get("quelle") or {}).get("regel_id") or ""


def _queue(beitrag: dict | None) -> list[str]:
    """naechste_fragen mit leerem Store — dieselben Stufen, ohne API und ohne catala."""
    s = ST.leerer_store(2025, fall_id="vordruck-ordnung")
    return TR.naechste_fragen(s, BINDUNG, beitrag)


def _gruppe(thema: str, beitrag: dict | None) -> list[str]:
    return [f for f in _queue(beitrag) if _thema(f) == thema]


# ---- H1: Struktur — die Anlass-Felder gehoeren der Klasse "formal" --------------------------

def test_anlass_felder_sind_formal_und_nicht_wert():
    """Die Begruendung der Gewichtsregel steht und faellt damit, dass diese vier NICHT "wert"
    sind. Waeren sie "wert", wuerde der Beitrag sie umsortieren und der 2026-08-27-Befund
    koennte zurueckkommen."""
    for fid, label in ANLASS.items():
        assert fid in BINDUNG, f"{label} ({fid}) ist gar nicht mehr gebunden."
        assert _klasse(fid) == "formal", (
            f"{label} ({fid}) traegt die Klasse {_klasse(fid)!r} statt 'formal'. Die "
            f"Gewichtsregel setzt nur die Klasse 'wert' aus — gehoert dieses Feld zu 'wert', "
            f"kann der Vordruck-Anlass vom 2026-08-27 wieder aufreissen.")


def test_anlass_felder_tragen_eine_kennzahl():
    """Ohne Kennzahl gaebe es nichts zu sortieren — dann waere der ganze Vordruck-Anlass
    gegenstandslos und dieser Test bewachte nichts."""
    for fid, label in ANLASS.items():
        assert BINDUNG[fid].get("elster_kz"), f"{label} ({fid}) traegt keine ELSTER-Kennzahl."


# ---- H2: Wirkung — die Position haelt gegen die Gewichte -----------------------------------

def test_vordruck_anlass_haelt_gegen_die_gewichte():
    """Der Kern: dieselbe Position mit und ohne beitrag.

    Faellt dieser Test, hat die Gewichtsregel den Vordruck-Anlass ueberschrieben — dann ist
    entweder die Regel falsch gebaut oder die Klassenzuordnung hat sich geaendert."""
    themen = {_thema(f) for f in ANLASS}
    assert len(themen) == 1, f"die vier Anlass-Felder liegen in {len(themen)} Themen: {themen}"
    thema = themen.pop()

    ohne = _gruppe(thema, None)
    mit = _gruppe(thema, BEITRAG)

    assert len(ohne) == len(mit), (
        f"Thema {thema}: {len(ohne)} Fragen ohne beitrag, {len(mit)} mit beitrag — die "
        f"Gewichtsregel darf die Zahl der Fragen nicht aendern.")

    for fid, label in ANLASS.items():
        if fid not in ohne:
            continue
        p_ohne, p_mit = ohne.index(fid), mit.index(fid)
        assert p_ohne == p_mit, (
            f"{label} ({fid}) steht ohne beitrag auf Platz {p_ohne} und mit beitrag auf "
            f"{p_mit}. Der Vordruck-Anlass vom 2026-08-27 ist damit zurueck: die Daten des "
            f"anderen Elternteils duerfen nicht vor den Vornamen des eigenen Kindes rutschen.")


def test_vorname_vor_anderem_elternteil():
    """Der Befund namentlich, in der Ordnung, die Julius im Durchgang erwartet hat.

    Ohne diesen Test wuerde der Sweep oben auch dann gruen bleiben, wenn ALLE vier Felder
    gemeinsam ans Ende rutschten — die Reihenfolge untereinander waere ja erhalten."""
    thema = _thema("kind_vorname")
    for beitrag in (None, BEITRAG):
        q = _gruppe(thema, beitrag)
        if "kind_vorname" not in q:
            continue
        p_vorname = q.index("kind_vorname")
        spaeter = [f for f in ANLASS
                   if f != "kind_vorname" and f in q and q.index(f) < p_vorname]
        assert not spaeter, (
            f"mit beitrag={beitrag is not None}: {spaeter} stehen VOR kind_vorname "
            f"(Platz {p_vorname}). Genau das war der Befund vom 2026-08-27.")
