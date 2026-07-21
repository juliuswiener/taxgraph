"""§23 EStG Private Veräußerungsgeschäfte — Accessor-Fidelity (3 Snapshots). NULL LLM.

3 promoted+inert Snapshots (p23_veraeusserungsgewinn, p23_freigrenze, p23_3_verlusttopf):
Jede Regel muss JEDEN test_seed EXAKT reproduzieren. EURO (integer).
"""

import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402


# -- p23_veraeusserungsgewinn (3 seeds: 200000-150000-5000=45000 / 100000-120000-3000=-23000 / 50000-50000-0=0)

def test_gewinn_seed_0():
    r = R.catala_p23_veraeusserungsgewinn({"veraeusserungspreis": 200000, "anschaffungs_herstellungskosten": 150000,
                                            "werbungskosten": 5000})
    assert r == 45000


def test_gewinn_seed_1_verlust():
    r = R.catala_p23_veraeusserungsgewinn({"veraeusserungspreis": 100000, "anschaffungs_herstellungskosten": 120000,
                                            "werbungskosten": 3000})
    assert r == -23000


def test_gewinn_seed_2_null():
    r = R.catala_p23_veraeusserungsgewinn({"veraeusserungspreis": 50000, "anschaffungs_herstellungskosten": 50000,
                                            "werbungskosten": 0})
    assert r == 0


# -- p23_freigrenze (4 seeds: 5000→5000 / 999→0 / 1000→1000 / 0→0)

def test_freigrenze_seed_0():
    assert R.catala_p23_freigrenze({"gesamtgewinn": 5000}) == 5000


def test_freigrenze_seed_1_unter():
    assert R.catala_p23_freigrenze({"gesamtgewinn": 999}) == 0


def test_freigrenze_seed_2_schwelle():
    """Wächter: 1000 ist die Grenze — AB 1000 fällt der VOLLE Gewinn an."""
    assert R.catala_p23_freigrenze({"gesamtgewinn": 1000}) == 1000


def test_freigrenze_seed_3_null():
    assert R.catala_p23_freigrenze({"gesamtgewinn": 0}) == 0


# -- p23_3_verlusttopf (4 seeds: 2000-500=1500 / 500-800=0 / 1000-1000=0 / 0-0=0)

def test_verlusttopf_seed_0():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 2000, "verlust_pvg": 500}) == 1500


def test_verlusttopf_seed_1_verlust_uebersteigt():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 500, "verlust_pvg": 800}) == 0


def test_verlusttopf_seed_2_gleich():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 1000, "verlust_pvg": 1000}) == 0


def test_verlusttopf_seed_3_beide_null():
    assert R.catala_p23_verlusttopf({"gewinn_pvg": 0, "verlust_pvg": 0}) == 0
