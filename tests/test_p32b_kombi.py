"""§32b-Koinzidenz-Guard (K2, fail-closed): §32b Progressionsvorbehalt Post-Engine NACH
§34/§35/§34c. Bei Co-Präsenz (Lohnersatz + GewSt/DBA) nutzten die §35-Deckel-3- und
§34c-Höchstbeträge die PRE-§32b-tarifliche = silent Over-tax-Edge → fail-closed
(p32b_kombi_offen). Regression-Golden zum Inline-Fix (Commit 3f6e00f). Stufe-2: korrekte
post-§32b-Höchstbeträge. NULL LLM.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "produkt", "haut"))
import api as API  # noqa: E402

CFG = {"gesamt_guard": True, "guard": True}


def _snap(**felder):
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


def test_p32b_plus_p35_gewerbesteuer_sperrt():
    """pe>0 + gewst_messbetrag>0 → p32b_kombi_offen (§35-Deckel-3 nutzte pre-§32b-Tarif)."""
    felder = _snap(p32b_progressionseinkuenfte=(500000, "bestaetigt"),
                   gewst_messbetrag=(100000, "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder, CFG) == "p32b_kombi_offen"


def test_p32b_plus_p34c_dba_sperrt():
    """pe>0 + dba_auslaendische_einkuenfte>0 → p32b_kombi_offen (§34c-HB nutzte pre-§32b-Tarif)."""
    felder = _snap(p32b_progressionseinkuenfte=(500000, "bestaetigt"),
                   dba_auslaendische_einkuenfte=(2000000, "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder, CFG) == "p32b_kombi_offen"


def test_p32b_allein_kein_kombi_sperr():
    """pe>0 ALLEIN (kein §34/§35/§34c) → kein Kombi-Sperr, §32b rechnet normal."""
    felder = _snap(p32b_progressionseinkuenfte=(500000, "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder, CFG) != "p32b_kombi_offen"


def test_ohne_pe_kein_kombi_sperr():
    """gewst_messbetrag>0 ohne pe → kein §32b-Kombi-Sperr (nur §35 allein, rechenbar)."""
    felder = _snap(gewst_messbetrag=(100000, "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder, CFG) != "p32b_kombi_offen"
