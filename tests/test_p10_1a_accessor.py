"""Unit test: §10 Abs.1a Nr.1 EStG Realsplitting accessor."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R


def test_seed_1_under_cap():
    """Unterhalt 10.000, kein KV/PV. Deckel 13.805. min(10.000; 13.805) = 10.000."""
    assert R.catala_p10_1a_realsplitting({
        "unterhaltsleistungen": 10000,
        "kv_pv_beitraege": 0
    }) == 10000


def test_seed_2_cap_no_kv():
    """WÄCHTER Deckelung: Unterhalt 15.000 > 13.805 -> 13.805."""
    assert R.catala_p10_1a_realsplitting({
        "unterhaltsleistungen": 15000,
        "kv_pv_beitraege": 0
    }) == 13805


def test_seed_3_cap_with_kv():
    """WÄCHTER KV/PV-Erhöhung: Unterhalt 15.000, KV/PV 2.000. Deckel 13.805 + 2.000 = 15.805. min(15.000; 15.805) = 15.000."""
    assert R.catala_p10_1a_realsplitting({
        "unterhaltsleistungen": 15000,
        "kv_pv_beitraege": 2000
    }) == 15000


def test_seed_4_high_cap():
    """WÄCHTER hohe Deckelung: Unterhalt 20.000, kein KV/PV -> 13.805."""
    assert R.catala_p10_1a_realsplitting({
        "unterhaltsleistungen": 20000,
        "kv_pv_beitraege": 0
    }) == 13805


if __name__ == "__main__":
    test_seed_1_under_cap()
    test_seed_2_cap_no_kv()
    test_seed_3_cap_with_kv()
    test_seed_4_high_cap()
    print("✓ All accessor tests passed")
