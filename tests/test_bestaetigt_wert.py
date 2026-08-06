"""Test: _bestaetigt_wert — die Regel "vorlaeufig zählt nicht als Beleg" ist scharf.

Ein Feld mit zustand=vorlaeufig darf NICHT als Beleg zählen (None zurück).
Wenn jemand die Regel entschärft (vorlaeufig durchlässt), wird dieser Test rot.
"""
import sys
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "konsistenz"))

from _helpers import _bestaetigt_wert  # noqa: E402


def _snap(**felder):
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


def test_bestaetigt_gibt_wert():
    """Bestaetigtes Feld -> Wert."""
    s = _snap(bruttoarbeitslohn=(5000000, "bestaetigt"))
    assert _bestaetigt_wert(s, "bruttoarbeitslohn") == 5000000


def test_vorlaeufig_gibt_none():
    """Vorlaeufiges Feld -> None (kein Beleg)."""
    s = _snap(bruttoarbeitslohn=(5000000, "vorlaeufig"))
    assert _bestaetigt_wert(s, "bruttoarbeitslohn") is None


def test_fehlt_gibt_none():
    """Fehlendes Feld -> None."""
    assert _bestaetigt_wert({}, "bruttoarbeitslohn") is None


def test_bool_wird_durchgelassen():
    """Bool-Wert (True/False) wird nicht als 0/None gefiltert."""
    s = _snap(kein_gewinn=(True, "bestaetigt"))
    assert _bestaetigt_wert(s, "kein_gewinn") is True
    s2 = _snap(kein_gewinn=(False, "bestaetigt"))
    assert _bestaetigt_wert(s2, "kein_gewinn") is False


def test_string_wird_durchgelassen():
    """String-Wert wird durchgelassen."""
    s = _snap(veranlagung=("zusammen", "bestaetigt"))
    assert _bestaetigt_wert(s, "veranlagung") == "zusammen"