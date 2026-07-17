"""Einheiten-Gate (Naht Kern↔Engine, Task #11). Deterministisch, NULL LLM.

Sichert die kanonische Naht-Einheit CENT gegen stille Euro/Cent-Mischung (Falsch-Grün-Klasse — der
EP-Durchstich zeigte 21,56 € statt 2156 €, weil eine EURO-Nativ-Ausgabe als Cent gelabelt wurde).

Die Einheiten-WAHRHEIT ist der golden-Erwartungswert-Key: `*_cent` -> cent, sonst euro. Das Gate:
  (A) NATIV_EINHEIT deckt genau die in Goldens vorkommenden Quantitäten ab; eine ungemappte
      Quantität wirft (kein stiller Default).
  (B) je Quantität stimmt die gemappte Einheit mit der golden-Key-Konvention überein.
  (C) die euro->cent-Normalisierung ist verlustfrei-exakt (EP 2156 -> 215600) und lässt Cent-Nativ
      unverändert (Nenner-B), geprüft gegen den echten Accessor + Golden-Erwartungswert.
"""
from __future__ import annotations

import glob
import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "unsicherheit"))

import intervall as IV   # noqa: E402


def _golden_quantitaeten() -> dict:
    """{erwartung-Key -> [(golden-id, erwartungswert), ...]} über alle golden/cases."""
    out: dict = {}
    for f in sorted(glob.glob(os.path.join(ROOT, "golden", "cases", "*.yaml"))):
        c = yaml.safe_load(open(f, encoding="utf-8"))
        for k, v in (c.get("erwartung") or {}).items():
            out.setdefault(k, []).append((c.get("id", os.path.basename(f)), v, c.get("sachverhalt") or {}))
    return out


GOLDEN_Q = _golden_quantitaeten()


# ---- (A) Vollständigkeit + fail-closed ---------------------------------------

def test_a_map_deckt_genau_die_golden_quantitaeten():
    golden_keys = set(GOLDEN_Q)
    map_keys = set(IV.NATIV_EINHEIT)
    fehlend = golden_keys - map_keys           # verdrahtete Quantität ohne Einheit -> stiller Default-Fehler
    assert not fehlend, f"golden-Quantitäten ohne NATIV_EINHEIT-Eintrag: {sorted(fehlend)}"
    ohne_golden = map_keys - golden_keys       # Auflage B: Map-Eintrag ohne Golden = benannte Lücke
    assert not ohne_golden, (f"NATIV_EINHEIT-Einträge ohne Golden (Report-Lücke + Einheiten-Begründung "
                             f"nötig): {sorted(ohne_golden)}")


def test_a_unbelegte_quantitaet_wirft():
    with pytest.raises(ValueError, match="Einheit unbelegt"):
        IV.nach_cent(1, "voellig_unbekannte_quantitaet")
    with pytest.raises(ValueError, match="Einheit unbelegt"):
        IV.bescheid_via_slots({}, lambda slots: 0, quantitaet="voellig_unbekannte_quantitaet")


# ---- (B) Map ↔ golden-Key-Konvention -----------------------------------------

def test_b_einheit_stimmt_mit_golden_key_konvention():
    for k, einheit in IV.NATIV_EINHEIT.items():
        erwartet = "cent" if k.endswith("_cent") else "euro"
        assert einheit == erwartet, f"{k}: Map sagt {einheit}, golden-Key-Konvention sagt {erwartet}"


# ---- (C) Normalisierung exakt gegen echten Accessor + Golden -----------------

def _runner():
    try:
        sys.path.insert(0, os.path.join(ROOT, "golden"))
        import runner  # noqa: F401
        return runner
    except Exception as e:  # opam-Env/_catala fehlt
        pytest.skip(f"Catala-Toolchain nicht verfügbar: {type(e).__name__}: {e}")


def test_c_euro_accessor_ep_verlustfrei_mal_100():
    runner = _runner()
    for gid, exp, s in GOLDEN_Q["abziehbarer_betrag"]:
        roh = runner.catala_entfernungspauschale(s)
        assert roh == exp, f"{gid}: EP-Accessor {roh} != Golden {exp}"          # euro-Nativ == Golden
        assert IV.nach_cent(roh, "abziehbarer_betrag") == exp * 100, f"{gid}: *100 nicht exakt"


def test_c_cent_accessor_nennerb_unveraendert():
    runner = _runner()
    for gid, exp, s in GOLDEN_Q["nenner_b_cent"]:
        roh = runner.catala_kst_nenner_b(s)
        assert roh == exp, f"{gid}: Nenner-B-Accessor {roh} != Golden {exp}"     # cent-Nativ == Golden
        assert IV.nach_cent(roh, "nenner_b_cent") == exp, f"{gid}: Cent-Nativ darf nicht skaliert werden"


def test_c_ep_konkret_2156_wird_215600():
    runner = _runner()
    treffer = [(exp, s) for _, exp, s in GOLDEN_Q["abziehbarer_betrag"] if exp == 2156]
    assert treffer, "EP-Golden mit Erwartung 2156 fehlt"
    _, s = treffer[0]
    assert IV.nach_cent(runner.catala_entfernungspauschale(s), "abziehbarer_betrag") == 215600


# ---- Negativtests (Falsch-Grün-Doktrin) --------------------------------------

def test_neg_falsche_map_faellt_bei_konvention_auf():
    """Eine verdrehte Map (Euro-Quantität als cent) MUSS die B-Konventionsprüfung brechen."""
    verdreht = {"abziehbarer_betrag": "cent"}   # falsch: kein _cent-Suffix -> müsste euro sein
    ok = all((e == ("cent" if k.endswith("_cent") else "euro")) for k, e in verdreht.items())
    assert not ok, "Konventionsprüfung ist wirkungslos (würde eine falsche Einheit durchlassen)"


def test_neg_ohne_normalisierung_waere_euro_falsch_gelabelt():
    """Beleg der Fehlerklasse: die EURO-Nativ-Ausgabe als Cent gelesen ergäbe das falsche Ergebnis."""
    euro_nativ = 2156
    assert IV.nach_cent(euro_nativ, "abziehbarer_betrag") == 215600   # richtig (2156,00 EUR)
    assert euro_nativ != 215600                                       # ohne Adapter: 21,56 EUR (der Bug)
