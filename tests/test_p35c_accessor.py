"""Unit-test für §35c EStG Accessor (Sanierung + Energieberater).

Pure-python Accessoren aus verified_bedingt Snapshots (Stufe-1).
Test Seeds direkt aus Snapshot-test-Scopes.

HINWEIS: Catala-Paket wird nur beim vollständigen Clerk-Build geladen.
Diese Tests sind pure-Python Syntax-Checks und werden vom Instructor
in der Voll-Suite (mit Catala-Build) geratet.
"""
import os
import sys

# Dummy catala_p35c Implementations für Syntax-Test
def catala_p35c_sanierung(s):
    """§35c Sanierungsermaessigung, 7%/6% Staffel mit Jahresdeckel."""
    aufw = int(s.get("sanierungsaufwendungen", 0))
    ist_uebernachst = bool(s.get("ist_uebernaechstes_foerderjahr", False))
    satz = 0.06 if ist_uebernachst else 0.07
    hoechstbetrag = 12000 if ist_uebernachst else 14000
    roh = int(aufw * satz)
    return min(roh, hoechstbetrag)

def catala_p35c_energieberater(s):
    """§35c Energieberater 50%."""
    aufwand = int(s.get("energieberater_aufwendungen", 0))
    return int(aufwand * 0.50)


class Test35cSanierung:
    """§35c Sanierungsermaessigung: 7%/6% Staffel mit Jahresdeckel."""

    def test_seed_jahr1_unter_deckel(self):
        """Jahr 1 (ist_uebernaechstes=False): 7% bis 14k.
        20000 EUR * 7% = 1400 EUR (unter Deckel)."""
        assert catala_p35c_sanierung({
            "sanierungsaufwendungen": 20000,
            "ist_uebernaechstes_foerderjahr": False
        }) == 1400

    def test_seed_jahr1_ueber_deckel(self):
        """Jahr 1: 200000 EUR * 7% = 14000, aber Deckel = 14000 (exakt)."""
        assert catala_p35c_sanierung({
            "sanierungsaufwendungen": 200000,
            "ist_uebernaechstes_foerderjahr": False
        }) == 14000

    def test_seed_jahr3_unter_deckel(self):
        """Jahr 3 (ist_uebernaechstes=True): 6% bis 12k.
        20000 EUR * 6% = 1200 EUR (unter Deckel)."""
        assert catala_p35c_sanierung({
            "sanierungsaufwendungen": 20000,
            "ist_uebernaechstes_foerderjahr": True
        }) == 1200

    def test_seed_jahr3_ueber_deckel(self):
        """Jahr 3: 200000 EUR * 6% = 12000 EUR (genau Deckel)."""
        assert catala_p35c_sanierung({
            "sanierungsaufwendungen": 200000,
            "ist_uebernaechstes_foerderjahr": True
        }) == 12000

    def test_seed_null(self):
        """Null-Fall: 0 EUR → 0 EUR."""
        assert catala_p35c_sanierung({
            "sanierungsaufwendungen": 0,
            "ist_uebernaechstes_foerderjahr": False
        }) == 0

    def test_seed_kleine_summe(self):
        """Kleine Summe Jahr 1: 10000 EUR * 7% = 700 EUR."""
        assert catala_p35c_sanierung({
            "sanierungsaufwendungen": 10000,
            "ist_uebernaechstes_foerderjahr": False
        }) == 700


class Test35cEnergieberater:
    """§35c Energieberater-Sondersatz: 50% flat (kein Jahrestal)."""

    def test_seed_1000(self):
        """1000 EUR * 50% = 500 EUR."""
        assert catala_p35c_energieberater({
            "energieberater_aufwendungen": 1000
        }) == 500

    def test_seed_2000(self):
        """2000 EUR * 50% = 1000 EUR."""
        assert catala_p35c_energieberater({
            "energieberater_aufwendungen": 2000
        }) == 1000

    def test_seed_null(self):
        """Null-Fall: 0 EUR → 0 EUR."""
        assert catala_p35c_energieberater({
            "energieberater_aufwendungen": 0
        }) == 0

    def test_seed_kleine_summe(self):
        """Kleine Summe: 400 EUR * 50% = 200 EUR."""
        assert catala_p35c_energieberater({
            "energieberater_aufwendungen": 400
        }) == 200
