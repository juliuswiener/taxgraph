"""Verursacher-Attrappe fuer tests/test_ablage_endet_an_der_testdatei.py -- KEINE Testdatei.

Der Name beginnt mit `_`, darum sammelt `pytest tests/` sie nicht. Gesammelt wird sie nur,
wenn der Waechter sie ausdruecklich auf die Befehlszeile setzt.

Sie tut genau das, was die fuenf Verursacher vom 2026-10-01 taten: eine Modul-Fixture lenkt
api.FAELLE und audit.AUDIT_DIR per Zuweisung um und setzt beide nie zurueck.
"""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/haut", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api                     # noqa: E402
import audit                   # noqa: E402

SENTINEL = "/leck-probe/faelle"  # kein echtes Verzeichnis -- die Attrappe schreibt nichts


@pytest.fixture(scope="module")
def umgelenkt():
    api.FAELLE = SENTINEL
    audit.AUDIT_DIR = SENTINEL


def test_umlenkung_greift(umgelenkt):
    assert (audit.AUDIT_DIR, api.FAELLE) == (SENTINEL, SENTINEL)
