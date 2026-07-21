"""§34c Abs.1 DBA-Anrechnung — Accessor-Fidelity + Seed-Gate. Deterministisch, NULL LLM.

4 pipeline-verified test_seeds (p34c_1_anrechnung_hoechstbetrag, rules.yaml:5206).
EURO (integer). catala_p34c_1 Accessor muss JEDEN Seed exakt reproduzieren.
"""

import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


def test_seed_0_gezahlte_unter_hoechstbetrag():
    """3000 gezahlt, HB=20000*40000/100000=8000 → anrechnung=3000 (gezahlt<HB)."""
    r = R.catala_p34c_1({"gezahlte_auslaendische_steuer": 3000,
                           "deutsche_est_inkl_ausl": 20000,
                           "zu_versteuerndes_einkommen": 100000,
                           "auslaendische_einkuenfte_staat": 40000})
    assert r == 3000


def test_seed_1_hoechstbetrag_deckelt():
    """10000 gezahlt > HB=8000 → anrechnung=8000 (gedeckelt)."""
    r = R.catala_p34c_1({"gezahlte_auslaendische_steuer": 10000,
                           "deutsche_est_inkl_ausl": 20000,
                           "zu_versteuerndes_einkommen": 100000,
                           "auslaendische_einkuenfte_staat": 40000})
    assert r == 8000


def test_seed_2_ausl_null_ergibt_null():
    """auslaendische_einkuenfte=0 → anrechnung=0 (kein Höchstbetrag rechenbar)."""
    r = R.catala_p34c_1({"gezahlte_auslaendische_steuer": 3000,
                           "deutsche_est_inkl_ausl": 20000,
                           "zu_versteuerndes_einkommen": 100000,
                           "auslaendische_einkuenfte_staat": 0})
    assert r == 0


def test_seed_3_zve_60000_ausl_30000():
    """HB=30000*30000/60000=15000, gezahlt=5000 → anrechnung=5000."""
    r = R.catala_p34c_1({"gezahlte_auslaendische_steuer": 5000,
                           "deutsche_est_inkl_ausl": 30000,
                           "zu_versteuerndes_einkommen": 60000,
                           "auslaendische_einkuenfte_staat": 30000})
    assert r == 5000


def test_zve_null_abort():
    """zve=0 → 0 (Division nicht möglich)."""
    assert R.catala_p34c_1({"gezahlte_auslaendische_steuer": 5000,
                              "deutsche_est_inkl_ausl": 20000,
                              "zu_versteuerndes_einkommen": 0,
                              "auslaendische_einkuenfte_staat": 40000}) == 0


def test_zve_negativ_abort():
    """zve=-1000 (Verlustfall) → 0."""
    assert R.catala_p34c_1({"gezahlte_auslaendische_steuer": 5000,
                              "deutsche_est_inkl_ausl": 20000,
                              "zu_versteuerndes_einkommen": -1000,
                              "auslaendische_einkuenfte_staat": 40000}) == 0
