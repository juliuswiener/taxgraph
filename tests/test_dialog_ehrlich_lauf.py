"""Der Nutzerpfad, ende zu ende: nur beantworten, was der Dialog TATSÄCHLICH fragt.

Der Unterschied zu jedem anderen Ring-Test in diesem Repo: hier wird kein einziges Feld direkt
gesetzt. Der Lauf fragt traverser.naechste_fragen(), beantwortet die vorderste Frage
wahrheitsgemäß, und wiederholt das, bis nichts mehr gefragt wird. Erst dann wird nach einer Zahl
gefragt.

Warum das eine eigene Testklasse ist (BACKLOG ratsche-misst-nicht-den-nutzerpfad, Memory
gate-misst-nicht-den-nutzerpfad): die Abgabe-Ratsche und die Ring-Tests füllen den Kegel von
Hand. Sie können deshalb grün sein, während der Dialog den Nutzer nie bis zu einer Zahl führt —
genau das war am 2026-08-12 der Fall: 218 Fragen beantwortet, danach immer noch Sperrgrund
input_kegel_nicht_bestaetigt auf einem Feld, das der Traverser gar nicht stellt.

Gemessen am 2026-08-13, nach dem Kegel/relevanz-Fix:

    an_gesamt    47 Fragen   ->  823800 ct
    gesamt      194 Fragen   ->  823800 ct

Die Zahl selbst ist hier NICHT die Aussage — sie hängt am unten gesetzten Bruttolohn und wandert
mit jedem Tarifjahr. Die Aussage ist: der Dialog endet, der Kegel ist danach leer, und es kommt
überhaupt eine Zahl heraus. Deshalb prüft der Test die Erreichbarkeit und eine grobe
Plausibilitätsspanne, nicht den Cent.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden", "produkt/unsicherheit", "produkt/store",
            "produkt/traverser", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API          # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402

BRUTTOLOHN_CENT = 6000000        # 60.000 EUR

# Julius' Fall: Arbeitnehmer, Zusammenveranlagung, VZ 2025. Keine Gewinneinkünfte, keine
# Kapitalerträge, keine Vermietung, keine sonstigen Einkünfte. Alles, was hier nicht steht,
# wird nach Typ beantwortet (s. _antwort) — bool-Gates mit False (Tatbestand liegt nicht vor),
# Beträge mit 0, enum mit dem ersten zulässigen Wert.
EXPLIZIT = {
    "veranlagung": "zusammen",
    "bruttoarbeitslohn": BRUTTOLOHN_CENT,
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": True,
}
MAX_FRAGEN = 600                 # Reißleine: der Dialog muss ENDEN, nicht nur antworten


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _antwort(bindung: dict, fid: str):
    if fid in EXPLIZIT:
        return EXPLIZIT[fid]
    b = bindung.get(fid, {})
    typ = b.get("typ")
    if typ in ("bool", "boolean"):
        return False
    if typ == "enum":
        werte = b.get("enum_werte") or []
        return werte[0] if werte else None
    if typ == "string":
        return ""
    return 0


def _durchklicken(scheibe: str):
    """Beantwortet den Dialog bis zum Ende. Returns (store, bindung, gestellte_fragen)."""
    store = ST.leerer_store(2025, fall_id=f"ehrlich_{scheibe}")
    store["scheibe"] = scheibe
    bindung = API._scheibe_bindung(store)
    gestellt: list[str] = []
    for _ in range(MAX_FRAGEN):
        fragen = TR.naechste_fragen(store, bindung)
        if not fragen:
            return store, bindung, gestellt
        fid = fragen[0]
        wert = _antwort(bindung, fid)
        assert wert is not None, (
            f"Für {fid} (typ={bindung.get(fid, {}).get('typ')!r}) lässt sich keine Antwort "
            f"bilden — der Dialog stellt eine Frage, die dieser Test nicht beantworten kann.")
        ST.append_event(store=store, feld_id=fid, wert=wert, zustand="bestaetigt",
                        herkunft={"quelle": "ehrlich_lauf"}, schreiber="ui:laie",
                        signal={"signal_1": None, "signal_2": f"ok@{fid}"},
                        ts="2026-08-13T12:00:00Z")
        gestellt.append(fid)
    raise AssertionError(
        f"Dialog endet nach {MAX_FRAGEN} Fragen nicht (Scheibe {scheibe}) — Endlosschleife "
        f"oder eine Frage, die ihre eigene Beantwortung nicht registriert.")


@pytest.mark.parametrize("scheibe", ["an_gesamt", "gesamt"])
def test_dialog_fuehrt_bis_zu_einer_zahl(scheibe):
    """Kern: wer wahrheitsgemäß antwortet, bekommt am Ende eine Steuer — ohne dass irgendwo ein
    Feld an der Frage vorbei gesetzt wurde."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    store, bindung, gestellt = _durchklicken(scheibe)
    assert gestellt, "Der Dialog hat keine einzige Frage gestellt."

    felder, _ = ST.materialisiere(store)
    cfg = API._cfg(store)
    vz = int(store["veranlagungszeitraum"])
    scheibe_felder = cfg.get("kegel") or API._scheibe_felder(store)

    sperr = (API._an_gesamt_sperrgrund(felder, cfg, vz, store, bindung)
             if cfg.get("guard") else None)
    assert sperr is None, (
        f"Nach {len(gestellt)} beantworteten Fragen sperrt der Guard mit {sperr!r} — der Nutzer "
        f"hat alles beantwortet, was ihm gestellt wurde.")

    offen = [f for f in API._relevante_kegel_felder(scheibe_felder, bindung, store)
             if f not in felder or felder[f]["zustand"] != "bestaetigt"]
    assert not offen, (
        f"Der Pflicht-Kegel verlangt nach {len(gestellt)} Fragen weiter {offen} — Felder, die "
        f"der Dialog nie gestellt hat. Genau die Lücke aus traverser-ring-kegel-relevanz-naht.")

    ergebnis = API._feste_zahl(felder, bindung, cfg, vz, scheibe_felder, store)
    assert ergebnis is not None, (
        f"Kegel vollständig und Guard frei, aber _feste_zahl liefert None (Scheibe {scheibe}).")
    zahl, _solz, _extras = ergebnis
    # Grobe Plausibilität statt Cent-Wert: die Steuer auf 60.000 EUR im Splittingtarif liegt
    # zwischen 3 % und 30 % — genug, um eine Null oder eine absurde Zahl zu fangen, und robust
    # gegen jede Tarifänderung.
    assert 0.03 * BRUTTOLOHN_CENT < zahl < 0.30 * BRUTTOLOHN_CENT, (
        f"zahl_cent={zahl} ist für {BRUTTOLOHN_CENT} ct Bruttolohn unplausibel.")


def test_beide_scheiben_kommen_auf_dieselbe_zahl():
    """an_gesamt und gesamt beschreiben denselben Fall unterschiedlich breit. Für einen reinen
    Arbeitnehmerfall ohne die Zusatz-Einkunftsarten der gesamt-Scheibe müssen sie dieselbe Steuer
    ergeben — läuft das auseinander, rechnet einer der beiden Ringe etwas mit, das der andere
    nicht sieht."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    zahlen = {}
    for scheibe in ("an_gesamt", "gesamt"):
        store, bindung, _ = _durchklicken(scheibe)
        felder, _ = ST.materialisiere(store)
        cfg = API._cfg(store)
        sf = cfg.get("kegel") or API._scheibe_felder(store)
        e = API._feste_zahl(felder, bindung, cfg, int(store["veranlagungszeitraum"]), sf, store)
        assert e is not None, f"Scheibe {scheibe} liefert keine Zahl."
        zahlen[scheibe] = e[0]
    assert zahlen["an_gesamt"] == zahlen["gesamt"], (
        f"Dieselbe Ausgangslage, zwei Steuern: {zahlen}")
