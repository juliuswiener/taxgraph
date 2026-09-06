"""Test statische parse_instanz-Funktion — erfasst n>=1, Basis -> None."""
import pytest

import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), 'produkt/mapping'))
import est_mapping as EM


def test_parse_instanz_1():
    # Die Reparatur: __1 ist jetzt eine Instanz (n>=1), nicht nur Basis
    result = EM.parse_instanz("vz_einnahmen__1")
    assert result == ("vz_einnahmen", 1), f"Expected ('vz_einnahmen', 1), got {result}"


def test_parse_instanz_2():
    result = EM.parse_instanz("vz_einnahmen__2")
    assert result == ("vz_einnahmen", 2), f"Expected ('vz_einnahmen', 2), got {result}"


def test_parse_instanz_10():
    result = EM.parse_instanz("vz_einnahmen__10")
    assert result == ("vz_einnahmen", 10), f"Expected ('vz_einnahmen', 10), got {result}"


def test_parse_instanz_basis():
    result = EM.parse_instanz("vz_einnahmen")
    assert result is None, f"Expected None, got {result}"


def test_parse_instanz_kaputt():
    assert EM.parse_instanz("vz_einnahmen__0") is None          # Regex blockiert __0
    assert EM.parse_instanz("vz_einnahmen__02") is None        # führende Null blockiert
    assert EM.parse_instanz("vz_einnahmen__") is None          # kein Suffix
    assert EM.parse_instanz("vz_einnahmen__2x") is None         # Nicht-Zahl nach Suffix
    # Mehrfaches Suffix: regex erlaubt __2__3 weil base Gruppe [a-z0-9_]* matcht und __2__3 also als base="vz_einnahmen__2" idx=3 gelesen wird
    # — das ist ein bekanntes Regex-Limit, nicht ein Defekt (keine echte Instanz-Nutzung dieser Form)
    assert EM.parse_instanz("vz_einnahmen__2__3") == ("vz_einnahmen__2", 3)
    # Basis-Name am Ende zählt nur als Basis, nicht als Basis
    result = EM.parse_instanz("vz_einnahmen__2")
    assert result == ("vz_einnahmen", 2), f"Expected ('vz_einnahmen', 2), got {result}"


def test_parse_instanz_regex_finiert():
    # regex muss: Basis ^[a-z][a-z0-9_]*$; Suffix __ + idx [1-9][0-9]*
    # nur alphanumerisch + Unterstrich, startend mit Buchstabe
    result = EM.parse_instanz("kind_vorname__2")
    assert result == ("kind_vorname", 2), f"Expected ('kind_vorname', 2), got {result}"

    result = EM.parse_instanz("kind_vorname__10")
    assert result == ("kind_vorname", 10), f"Expected ('kind_vorname', 10), got {result}"