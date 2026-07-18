"""§ 10 Abs. 1 Nr. 3/3a + Abs. 4 EStG — Kranken-/Pflegeversicherung (Basisabsicherung + weitere Vorsorge),
Accessor-Einheitstest (module KrankenPflegeVorsorge). Prüft catala_p10_kv_pv: Höchstbetrag 2800 (ohne
Zuschussanspruch) / 1900 (mit Anspruch); Basis + weitere GETRENNT; § 10 Abs. 4 S. 4 Durchbruch (die
Basisabsicherung ist STETS voll abziehbar, auch über HB). Die Fold-seitige Einbettung in den gesamt-Ring
ist e2e getestet (test_gesamt_kv_pv_*). Toolchain-frei übersprungen."""
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "golden"))


@pytest.fixture(scope="module")
def R():
    try:
        import runner
        runner.catala_p10_kv_pv({"basis_kv_pv": 0, "weitere_vorsorgeaufwendungen": 0,
                                 "mit_anspruch_auf_zuschuss": False})
    except Exception:
        pytest.skip("Catala-Toolchain / pkg nicht verfügbar")
    return runner


@pytest.mark.parametrize("basis,weitere,zuschuss,erwartet", [
    (3200, 0, False, 3200),     # Durchbruch: Basis > HB 2800 → STETS voll (§ 10 Abs. 4 S. 4)
    (2000, 500, False, 2500),   # Basis + weitere 2500 < HB 2800 → voll
    (2000, 1000, False, 2800),  # Basis (2000) < HB, Summe 3000 > HB → auf HB 2800 gedeckelt
    (1500, 800, True, 1900),    # mit Zuschuss HB 1900; Summe 2300 > HB → auf 1900 gedeckelt
    (1900, 0, True, 1900),      # Basis = HB 1900 (mit Zuschuss) → voll
    (2200, 0, True, 2200),      # Durchbruch mit Zuschuss: Basis 2200 > HB 1900 → STETS voll
    (0, 0, False, 0),           # leer → 0
    (0, 500, False, 500),       # nur weitere Vorsorge < HB → voll 500
])
def test_p10_kv_pv_hoechstbetrag(R, basis, weitere, zuschuss, erwartet):
    assert R.catala_p10_kv_pv({"basis_kv_pv": basis, "weitere_vorsorgeaufwendungen": weitere,
                               "mit_anspruch_auf_zuschuss": zuschuss}) == erwartet


def test_p10_kv_pv_durchbruch_schlaegt_hoechstbetrag(R):
    """§ 10 Abs. 4 S. 4 Durchbruch (Über-Besteuerungs-Fix): eine reine Basisabsicherung ÜBER dem Höchstbetrag
    bleibt voll abziehbar — der gedeckelte HB-Betrag wäre zu niedrig (Nutzer würde die Pflichtbeiträge über HB
    überzahlen). Basis 3200 (> HB 2800) → 3200, nicht 2800."""
    voll = R.catala_p10_kv_pv({"basis_kv_pv": 3200, "weitere_vorsorgeaufwendungen": 0,
                               "mit_anspruch_auf_zuschuss": False})
    assert voll == 3200 and voll > 2800


def test_p10_kv_pv_weitere_gedeckelt(R):
    """Gegenprobe: weitere Vorsorge (freiwillig, kein Durchbruch) wird bei Überschreiten des HB gekürzt. Basis 2000
    (< HB) + weitere 2000 = 4000 → auf HB 2800 gedeckelt (nur die Basis genießt den Durchbruch, nicht die weitere)."""
    assert R.catala_p10_kv_pv({"basis_kv_pv": 2000, "weitere_vorsorgeaufwendungen": 2000,
                               "mit_anspruch_auf_zuschuss": False}) == 2800
