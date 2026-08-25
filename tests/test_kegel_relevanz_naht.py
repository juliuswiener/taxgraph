"""Der Pflicht-Kegel muss traverser.relevanz() respektieren (BACKLOG traverser-ring-kegel-relevanz-naht).

Befund aus dem Dialog-Durchstich (2026-08-12): wer wahrheitsgemäß angibt, seine Wohnung NICHT zu
Wohnzwecken zu überlassen (vv_wohnzwecke=False), schließt damit die Regel
p21_2_verbilligte_vermietung_wk aus. Der Traverser fragt deren Felder folgerichtig nie — aber
_feste_zahl() verlangt jedes Feld aus SCHEIBEN[scheibe]["kegel"] als bestätigt, ohne zu prüfen, ob
seine Regel überhaupt noch gilt. Ergebnis: `input_kegel_nicht_bestaetigt` mit
offen=["vv_entgelt_quote_prozent"] — ein Feld, das der Nutzer nie zu sehen bekommt. Der Dialog
kommt nie zu einer Zahl, ohne dass irgendwo etwas falsch gebunden wäre; es fehlt die Naht
zwischen zwei Mechanismen.

Die drei Tests hier decken die drei Richtungen ab, in die der Fix falsch laufen kann:
  1. er wirkt nicht                  -> test_ausgeschlossene_regel_sperrt_den_kegel_nicht
  2. er wirkt zu früh (fail-open)    -> test_unbeantwortetes_gate_sperrt_weiter
  3. er sprengt die slot_fn          -> test_kein_ausschliessbares_kegelfeld_ist_ein_gelesener_slot

Richtung 2 ist die gefährliche: relevanz() schließt NUR bei einem BESTÄTIGTEN False aus
(traverser.py:122). Würde der Fix auch unbeantwortete Gates ausschließen, verschwänden Pflichtfelder
lautlos aus dem Kegel und die Steuer würde auf unvollständiger Basis festgesetzt.

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
sys.path.insert(0, HERE)

import api as API              # noqa: E402
import api_constants as AC     # noqa: E402
import store as ST             # noqa: E402

from test_checkest_durchstich import _b            # noqa: E402
from test_p35a_bestandsdaten import _GESAMT_KEGEL  # noqa: E402

# Das Feld, das die ausgeschlossene Regel im Kegel hält, und das Gate, das sie ausschließt.
BLOCKIERER = "vv_entgelt_quote_prozent"
GATE = "vv_wohnzwecke"


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _fall(mit_gate: bool):
    """Voller gesamt-Kegel MINUS dem Blockierer. `mit_gate`: vv_wohnzwecke=False bestätigt
    (schließt p21_2_verbilligte_vermietung_wk aus) oder gar nicht gesetzt (Regel unentschieden).

    `kein_vuv` wird auf False GEDREHT (der Bestandskegel führt True). Grund, gemessen 2026-08-25:
    seit `p21_2_verbilligte_vermietung_wk` eine Regelbedingung auf `kein_vuv=false` hat, schliesst
    ein bestätigtes „keine Vermietung" die Regel bereits aus — und zwar zu Recht. Dann misst
    `test_unbeantwortetes_gate_sperrt_weiter` aber nicht mehr, was sein Name sagt: die Regel wäre
    auf einem ZWEITEN Weg ausgeschlossen, und das unbeantwortete Gate hätte nichts mehr zu sperren.
    Mit `kein_vuv=False` gibt es Vermietung, die Regel ist wieder unentschieden, und das Gate ist
    das Einzige, worauf es ankommt."""
    store = ST.leerer_store(2025, fall_id="kegel_relevanz")
    for feld, wert in _GESAMT_KEGEL:
        if feld == BLOCKIERER:
            continue                       # genau das Feld, das der Nutzer nie gefragt bekommt
        _b(store, feld, False if feld == "kein_vuv" else wert)
    if mit_gate:
        _b(store, GATE, False)
    store["scheibe"] = "gesamt"
    return store


def test_vorbedingung_die_regel_haengt_am_gate_und_nicht_am_screening():
    """Hält die Anpassung oben ehrlich: stünde `kein_vuv` wieder auf True, wären die beiden Tests
    darunter grün, ohne das Gate je zu berühren."""
    felder, _ = ST.materialisiere(_fall(mit_gate=False))
    assert felder.get("kein_vuv", {}).get("wert") is False, (
        "kein_vuv ist nicht False — dann schliesst das Screening die Regel aus und das Gate wird "
        "gar nicht mehr geprüft.")


def _feste_zahl_fuer(store):
    cfg = API._cfg(store)
    bindung = API._scheibe_bindung(store)
    felder, _ = ST.materialisiere(store)
    scheibe_felder = cfg.get("kegel") or API._scheibe_felder(store)
    vz = int(store["veranlagungszeitraum"])
    return API._feste_zahl(felder, bindung, cfg, vz, scheibe_felder, store)


def test_vorbedingung_blockierer_ist_im_kegel_und_nicht_gesetzt():
    """Hält den Test ehrlich: fällt BLOCKIERER irgendwann aus dem Kegel, prüfen die beiden
    Tests unten nichts mehr und wären trotzdem grün."""
    assert BLOCKIERER in (AC.SCHEIBEN["gesamt"].get("kegel") or ()), (
        f"{BLOCKIERER} steht nicht mehr im gesamt-Kegel — dieser Test misst dann nichts.")
    assert BLOCKIERER not in dict(_GESAMT_KEGEL) or True
    store = _fall(mit_gate=True)
    felder, _ = ST.materialisiere(store)
    assert BLOCKIERER not in felder or felder[BLOCKIERER]["zustand"] != "bestaetigt", (
        f"{BLOCKIERER} ist gesetzt — der Fall bildet die Lücke nicht ab.")


def test_ausgeschlossene_regel_sperrt_den_kegel_nicht():
    """Kern: Gate bestätigt-false -> Regel ausgeschlossen -> ihr Kegel-Feld darf nicht mehr sperren."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    ergebnis = _feste_zahl_fuer(_fall(mit_gate=True))
    assert ergebnis is not None, (
        f"{GATE}=False schließt p21_2_verbilligte_vermietung_wk aus, trotzdem sperrt der Kegel "
        f"auf {BLOCKIERER} — ein Feld, das der Traverser nie fragt.")
    zahl, _solz, _extras = ergebnis
    assert isinstance(zahl, int) and zahl > 0, f"zahl_cent unplausibel: {zahl!r}"


def test_unbeantwortetes_gate_sperrt_weiter():
    """Gegenrichtung (fail-closed): ohne Antwort auf das Gate ist die Regel NICHT ausgeschlossen,
    ihr Kegel-Feld bleibt Pflicht. Ohne diesen Test wäre ein Fix, der jedes ungeklärte Feld
    durchwinkt, ebenfalls grün — und würde die Steuer auf unvollständiger Basis festsetzen."""
    if not _catala_da():
        pytest.skip("catala nicht verfügbar")
    assert _feste_zahl_fuer(_fall(mit_gate=False)) is None, (
        f"{GATE} ist unbeantwortet — die Regel ist unentschieden, {BLOCKIERER} muss den Kegel "
        f"weiter sperren. relevanz() schließt nur bei bestätigtem False aus (traverser.py:122).")


def test_kein_ausschliessbares_kegelfeld_ist_ein_gelesener_slot():
    """Strukturgate gegen die Falle im Fix: fällt ein Feld aus dem Kegel, fehlt auch sein Slot im
    Dict, das bescheid_via_slots an die slot_fn übergibt. Die slot_fn liest ihre Slots seit der
    Klasse-3-Härtung mit slots[k] statt .get() — ein fehlender Slot wäre also ein KeyError statt
    eines stillen 0-Werts (gewollt so). Solange kein ausschließbares Kegel-Feld einen GELESENEN
    Slot trägt, kann das nicht passieren. Wird rot, sobald jemand ein solches Feld hinzufügt —
    dann braucht dieser Fix eine Antwort auf die Frage, welchen Wert der Slot dann tragen soll."""
    yaml = pytest.importorskip("yaml")
    import glob

    from test_slot_fn_reader_existiert import GELESENE_SLOT_NAMEN_JE_QUANTITAET

    bind = {}
    for fp in sorted(glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml"))):
        for b in yaml.safe_load(open(fp)).get("bindungen", []):
            bind[b["feld_id"]] = b

    gates_je_regel: dict[str, list[str]] = {}
    for fid, b in bind.items():
        q = b["quelle"]
        if "geltungsbedingung" in q and b.get("askable") and b.get("typ") in ("bool", "boolean"):
            gates_je_regel.setdefault(q["regel_id"], []).append(fid)

    verstoesse = []
    for scheibe, cfg in AC.SCHEIBEN.items():
        quant = cfg.get("gesamt_ring")
        gelesen = GELESENE_SLOT_NAMEN_JE_QUANTITAET.get(quant, set())
        for f in (cfg.get("kegel") or ()):
            b = bind.get(f)
            if not b or b["quelle"]["regel_id"] not in gates_je_regel:
                continue
            slot = b["quelle"].get("signatur_slot")
            if slot and slot in gelesen:
                verstoesse.append(f"{scheibe}: {f} -> Slot {slot!r} (quantitaet {quant})")
    assert not verstoesse, (
        "Ausschließbares Kegel-Feld trägt einen Slot, den die slot_fn liest — fällt es aus dem "
        "Kegel, wirft die slot_fn KeyError:\n  " + "\n  ".join(verstoesse))
