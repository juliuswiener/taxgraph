"""Gate fuer die AfA-Kohorten-params (§ 7 Abs. 2 Fenster + Abs. 2a E-Kfz-Staffel).

Verifiziert die Norm-Werte (aus dem Freeze) + die E-Kfz-Staffel-Summe = 100 %.
"""
import os

import yaml

KOH = os.path.join(os.path.dirname(__file__), "..", "params", "kohorten")


def _load(name):
    with open(os.path.join(KOH, name), encoding="utf-8") as f:
        return yaml.safe_load(f)


def test_degressive_fenster_werte():
    d = _load("degressive_afa_fenster_p7.yaml")["fenster"]
    # Wachstumschancengesetz 2024: 2x / 20 %.
    assert d["wtchanceng_2024"]["faktor_max"] == 2.0
    assert d["wtchanceng_2024"]["prozent_cap"] == 20.0
    # Booster 2025: 3x / 30 %.
    assert d["booster_2025"]["faktor_max"] == 3.0
    assert d["booster_2025"]["prozent_cap"] == 30.0


def test_ekfz_staffel_werte_und_summe():
    s = _load("ekfz_sonderafa_staffel_p7.yaml")["staffel"]
    assert [s[j]["prozent_jahr"] for j in range(6)] == [75.0, 10.0, 5.0, 5.0, 3.0, 2.0]
    # Summe der Staffel = 100 % (voll abgeschrieben nach 6 Jahren).
    assert sum(s[j]["prozent_jahr"] for j in range(6)) == 100.0
