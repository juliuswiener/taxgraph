"""Gate: die beiden KAP-Antragsfelder müssen als Kz DEKLARIERBAR sein.

Anlass 2026-08-10: Anlage KAP war ohne Antragsgrund uneinreichbar (checkESt rc=610001002).
`api._mit_ring_werten` injiziert seither zwei berechnete Felder — `kap_antrag_guenstigerpruefung`
(§ 32d Abs. 6, Kz E1900401, IMMER gesetzt sobald Kapitalerträge erklärt werden) und
`kap_sparer_pauschbetrag_genutzt` (§ 20 Abs. 9, Kz E1901401, der in Anspruch genommene
Sparer-Pauschbetrag — Vordruck erlaubt ausdrücklich "(ggf. 0)"). Team-lead maß beide zusammen als
zwingend: E1900401 allein bleibt rc=610001002 ("...jedoch keine Angabe zum in Anspruch genommenen
Sparer-Pauschbetrag").

Dieser Test prüft NICHT den Ring (das tut test_checkest_feldmatrix.py::
test_kap_antrag_ist_inert_ohne_kapitalertraege scharf gegen checkESt), sondern die BINDUNG:
wenn beide Felder auf der Scheibe fehlen, filtert `_scheibe_bindung` sie aus der Deklaration,
egal was der Ring rechnet — dieselbe Naht wie E0205508 (siehe test_abgabescheibe_gate.py).
"""

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/import", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API              # noqa: E402
import audit                   # noqa: E402


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    """Fälle in tmp_path statt ins echte produkt/haut/faelle/ — sonst kollidieren Wiederholungsläufe."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _kz(bindung):
    return {v.get("elster_kz") for v in bindung.values() if isinstance(v, dict)}


def test_kap_antrag_kz_sind_auf_gesamt_gebunden():
    """E1900401 und E1901401 müssen auf 'gesamt' deklarierbar sein.

    Ohne diese Bindung wäre die Injektion in _mit_ring_werten wirkungslos: der Ring liefert
    den Wert, aber _scheibe_bindung filtert ihn aus dem XML — stille Differenz wie bei E0205508.
    """
    API.fall_anlegen({"fall_id": "gate_kap_antrag_kz", "scheibe": "gesamt",
                      "veranlagungszeitraum": 2025})
    kz = _kz(API._scheibe_bindung(API.lade_fall("gate_kap_antrag_kz")))
    assert "E1900401" in kz, (
        "E1900401 (§ 32d Abs. 6 Günstigerprüfung) nicht auf 'gesamt' gebunden — "
        "Anlage KAP wäre wieder uneinreichbar (rc=610001002).")
    assert "E1901401" in kz, (
        "E1901401 (§ 20 Abs. 9 in Anspruch genommener Sparer-Pauschbetrag) nicht auf 'gesamt' "
        "gebunden — Antrag allein reicht nicht, checkESt verlangt beide (gemessen 2026-08-10).")
