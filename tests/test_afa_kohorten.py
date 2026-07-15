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


def test_ekfz_blp_grenze_kohorten():
    k = _load("ekfz_blp_grenze_p6.yaml")["kohorten"]
    # Freeze-verankerte Grenz-Werte je Anschaffungszeitpunkt-Kohorte.
    assert k["vor_wtchanceng"]["bruttolistenpreis_grenze"] == 60000
    assert k["wtchanceng_2024"]["bruttolistenpreis_grenze"] == 70000
    assert k["stinvsofortpg_2025"]["bruttolistenpreis_grenze"] == 100000
    # Monoton steigend (jede Kohorte hebt die Grenze an, kein 80k dazwischen).
    werte = [k["vor_wtchanceng"]["bruttolistenpreis_grenze"],
             k["wtchanceng_2024"]["bruttolistenpreis_grenze"],
             k["stinvsofortpg_2025"]["bruttolistenpreis_grenze"]]
    assert werte == sorted(werte) and len(set(werte)) == 3
    # Fenster schliessen luecken-/ueberlappungsfrei an: bis(70k) = Tag vor ab(100k).
    assert k["wtchanceng_2024"]["ab"] == "2024-01-01"
    assert k["wtchanceng_2024"]["bis"] == "2025-06-30"
    assert k["stinvsofortpg_2025"]["ab"] == "2025-07-01"
    # 60k-Kohorte: untere Grenze offen (Einfuehrung liegt vor beiden Freezes).
    assert k["vor_wtchanceng"]["ab"] is None
    assert k["vor_wtchanceng"]["bis"] == "2023-12-31"
