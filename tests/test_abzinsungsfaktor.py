"""Gate fuer die Abzinsungsfaktor-Tabelle (§ 6 Abs. 1 Nr. 3a e EStG).

(1) Stuetzwerte gegen BMF v. 26.05.2005 (BStBl I S. 699, Tabelle 2).
(2) Die eingecheckte params-Datei ist byte-identisch zur deterministischen Regeneration
    (kein manueller Drift).
"""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "params"))
import generate_abzinsungsfaktor as gen  # noqa: E402

PARAMS = os.path.join(os.path.dirname(__file__), "..", "params", "kohorten",
                      "abzinsungsfaktor_5komma5_p6.yaml")


def test_bmf_stuetzwerte():
    # BMF v. 26.05.2005, Tabelle 2 (Rueckstellungen 5,5 %).
    assert str(gen.faktor(1)) == "0.948"
    assert str(gen.faktor(10)) == "0.585"
    assert str(gen.faktor(19)) == "0.362"


def test_faktor_monoton_fallend_und_kleiner_eins():
    vals = [gen.faktor(n) for n in range(1, gen.MAX_RESTLAUFZEIT + 1)]
    assert all(v < 1 for v in vals)
    assert all(vals[i] > vals[i + 1] for i in range(len(vals) - 1))


def test_params_datei_ist_regenerierbar():
    # Eingecheckte Tabelle == deterministische Regeneration (kein Drift).
    with open(PARAMS, encoding="utf-8") as f:
        eingecheckt = f.read()
    assert eingecheckt == gen.build_yaml()
