"""Anker-Gate-Tail: golden/cases-Loader ins deckt_ab-Freeze-Gate.

Bis hierher war der Zitatanker-Freeze-Check der golden-Faelle NUR in
`golden/runner.py` (Schritt 1) verdrahtet - und der laeuft erst nach dem
schweren Catala-Assembly (`from pkg import ...` beim Modul-Import). Ein
Quell-Umbau, der einen golden-Anker bricht, wurde vom billigen Dauergate
`make unit` (pytest, kein Catala) NICHT gefangen; er fiel erst im vollen
`make golden` auf. Dieses Modul zieht die golden/cases-Anker in dieselbe
billige, Catala-freie Freeze-Pruefung wie das deckt_ab-Gate
(tests/test_deckt_ab_freeze.py) - dieselbe `_normalize`-Mechanik.

`gates._normalize` ist zeichengleich mit `golden/runner.py:normalize`
(beide: _UMLAUT-Transliteration + lower + Whitespace-Kollaps), das Verdikt
deckt sich also exakt mit dem Runner-Schritt-1. Kein Catala-Import hier -
laeuft in `make unit` (< 1 s).
"""

from __future__ import annotations

import glob
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, ROOT)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

from gates import _normalize   # noqa: E402  (identisch zu golden/runner.normalize)
from yamlstrict import load_str   # noqa: E402

CASES_GLOB = os.path.join(ROOT, "golden", "cases", "*.yaml")


def _cases():
    for path in sorted(glob.glob(CASES_GLOB)):
        with open(path, encoding="utf-8") as fh:
            yield path, load_str(fh.read(), herkunft=path)


def golden_anker_verletzungen(case: dict, root: str, cache: dict | None = None):
    """Spiegelt golden/runner.py Schritt 1 (Anker-in-Freeze), Catala-frei.

    Rueckgabe: Liste (case_id, grund). Leer = ok. Faelle ohne `quelle`
    (Schema erlaubt es theoretisch nicht, defensiv) werden uebersprungen.
    """
    cache = cache if cache is not None else {}
    q = case.get("quelle")
    if not q:
        return []
    cid = case.get("id", "<ohne-id>")
    datei = q.get("datei")
    anker = q.get("zitatanker")
    if not datei or not anker:
        return [(cid, f"quelle ohne datei/zitatanker (datei={datei!r})")]
    src_path = os.path.join(root, datei)
    if src_path not in cache:
        cache[src_path] = (_normalize(open(src_path, encoding="utf-8").read())
                           if os.path.exists(src_path) else None)
    norm_src = cache[src_path]
    if norm_src is None:
        return [(cid, f"Quelldatei fehlt: {datei}")]
    if _normalize(anker) not in norm_src:
        return [(cid, f"Zitatanker nicht im Freeze ({datei}): {anker[:60]!r}")]
    return []


# -- Tail-Gate: jeder golden-Anker steht woertlich im Freeze -------------------

def test_alle_golden_anker_im_freeze():
    cache: dict = {}
    verletzungen = []
    for _path, case in _cases():
        verletzungen += golden_anker_verletzungen(case, ROOT, cache)
    assert not verletzungen, (
        f"{len(verletzungen)} golden-Anker nicht im Freeze; erstes: "
        f"{verletzungen[0]}")


# -- Selbst-Konsistenz: jeder Fall traegt genau einen quelle-Anker ------------

def test_jeder_fall_hat_quelle_anker():
    ohne = []
    n = 0
    for path, case in _cases():
        n += 1
        q = case.get("quelle") or {}
        if not q.get("datei") or not q.get("zitatanker"):
            ohne.append(os.path.basename(path))
    assert not ohne, f"{len(ohne)} golden-Faelle ohne datei/zitatanker: {ohne}"
    assert n >= 78, f"unerwartet wenige golden-Faelle: {n}"


# -- Negativtest: synthetischer manipulierter Anker MUSS fallen ---------------

def test_negativtest_synthetischer_bad_anker(tmp_path):
    (tmp_path / "f.txt").write_text("Der Freeze enthaelt genau diesen Satz.\n",
                                    encoding="utf-8")
    gut = {"id": "gut", "quelle": {"datei": "f.txt",
                                   "zitatanker": "genau diesen Satz"}}
    boese = {"id": "boese", "quelle": {"datei": "f.txt",
                                       "zitatanker": "DIESER SATZ FEHLT"}}
    assert golden_anker_verletzungen(gut, str(tmp_path)) == []
    viol = golden_anker_verletzungen(boese, str(tmp_path))
    assert [v[0] for v in viol] == ["boese"], viol


# -- Negativtest: realer golden-Anker + Fremdtext MUSS fallen -----------------

def test_negativtest_realer_anker_manipuliert():
    cache: dict = {}
    # ersten Fall mit quelle nehmen, Anker mit Fremdtext verbiegen -> Verletzung
    real = next(case for _p, case in _cases() if (case.get("quelle") or {}).get("zitatanker"))
    kaputt = {"id": real["id"],
              "quelle": {"datei": real["quelle"]["datei"],
                         "zitatanker": real["quelle"]["zitatanker"] + " ZZZ_NICHT_IM_FREEZE"}}
    viol = golden_anker_verletzungen(kaputt, ROOT, cache)
    assert any(v[0] == real["id"] for v in viol), viol
