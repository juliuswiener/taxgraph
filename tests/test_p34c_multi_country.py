"""§34c Multi-Country Verification — Gate-Loop Tests for dev-2. Deterministic, NULL LLM.

Verifies:
1. Single-country DBA (any country) does NOT trigger gate (returns None) when no capital conflict.
2. Multi-country (dba_mehrere_staaten=True) triggers dba_multi_country_offen gate.
3. Capital income + foreign income triggers dba_kapital_offen gate.
4. Specific countries from task list (DEU, AT, CH, USA, CAN, GBR) behave as single-country (no gate).
"""

import os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "produkt/haut"))
import api as API  # noqa: E402

# Test helper: simulate gate call with DBA fields
def _call_gate(dba_staat=None, dba_methode=None, dba_mehrere_staaten=False,
               dba_gezahlte_auslaendische_steuer=0, dba_auslaendische_einkuenfte=0,
               kap_kapitalertraege=0, kap_verlust_aktien=0, kap_verlust_sonstige=0,
               kap_gewinn_aktien=0, kap_gewinn_sonstige=0):
    """Simulate gate call with DBA and capital fields"""
    felder = {}

    # DBA fields
    if dba_staat is not None:
        felder["dba_staat"] = {"zustand": "bestaetigt", "wert": dba_staat}
    if dba_methode is not None:
        felder["dba_methode"] = {"zustand": "bestaetigt", "wert": dba_methode}
    if dba_mehrere_staaten:
        felder["dba_mehrere_staaten"] = {"zustand": "bestaetigt", "wert": True}
    if dba_gezahlte_auslaendische_steuer > 0:
        felder["dba_gezahlte_auslaendische_steuer"] = {"zustand": "bestaetigt", "wert": dba_gezahlte_auslaendische_steuer}
    if dba_auslaendische_einkuenfte > 0:
        felder["dba_auslaendische_einkuenfte"] = {"zustand": "bestaetigt", "wert": dba_auslaendische_einkuenfte}

    # Capital fields (match KAP_* definitions)
    if kap_kapitalertraege > 0:
        felder["kap_kapitalertraege"] = {"zustand": "bestaetigt", "wert": kap_kapitalertraege}
    if kap_gewinn_aktien > 0:
        felder["kap_gewinn_aktien"] = {"zustand": "bestaetigt", "wert": kap_gewinn_aktien}
    if kap_verlust_aktien > 0:
        felder["kap_verlust_aktien"] = {"zustand": "bestaetigt", "wert": kap_verlust_aktien}
    if kap_gewinn_sonstige > 0:
        felder["kap_gewinn_sonstige"] = {"zustand": "bestaetigt", "wert": kap_gewinn_sonstige}
    if kap_verlust_sonstige > 0:
        felder["kap_verlust_sonstige"] = {"zustand": "bestaetigt", "wert": kap_verlust_sonstige}

    cfg = {"gesamt_guard": True}
    return API._an_gesamt_sperrgrund(felder, cfg, 2025)

# ---- Single-Country Tests (No Gate) ----

def test_single_country_no_gate():
    """Any single country with DBA values but no multi-country flag → no gate"""
    countries = ["Deutschland", "Österreich", "Schweiz", "USA", "Kanada", "Grossbritannien"]
    for country in countries:
        result = _call_gate(
            dba_staat=country,
            dba_gezahlte_auslaendische_steuer=1000,
            dba_auslaendische_einkuenfte=50000
        )
        assert result is None, f"Country {country} should not trigger gate (got {result})"

def test_single_country_with_method_override_no_gate():
    """Single country with explicit method override still no gate (unless capital conflict)"""
    result = _call_gate(
        dba_staat="Schweiz",
        dba_methode="dba_freistellung",
        dba_gezahlte_auslaendische_steuer=2000,
        dba_auslaendische_einkuenfte=40000
    )
    assert result is None

# ---- Multi-Country Gate ----

def test_multi_country_gate():
    """Multiple countries → dba_multi_country_offen"""
    result = _call_gate(
        dba_staat="Schweiz",  # any country, flag matters
        dba_mehrere_staaten=True,
        dba_gezahlte_auslaendische_steuer=3000,
        dba_auslaendische_einkuenfte=60000
    )
    assert result == "dba_multi_country_offen"

def test_multi_country_gate_with_different_countries():
    """Multiple countries explicitly (though field doesn't store list) still triggers gate"""
    result = _call_gate(
        dba_staat="USA",
        dba_mehrere_staaten=True,
        dba_gezahlte_auslaendische_steuer=1000,
        dba_auslaendische_einkuenfte=20000
    )
    assert result == "dba_multi_country_offen"

# ---- Capital Conflict Gate ----

def test_capital_conflict_gate():
    """Capital income + foreign income → dba_kapital_offen"""
    result = _call_gate(
        dba_staat="Österreich",
        dba_auslaendische_einkuenfte=50000,
        kap_kapitalertraege=10000  # any capital income triggers the KAP_ERTRAEGE check
    )
    assert result == "dba_kapital_offen"

def test_capital_conflict_gate_with_loss():
    """Capital loss + foreign income → dba_kapital_offen (loss counts)"""
    result = _call_gate(
        dba_staat="Schweiz",
        dba_auslaendische_einkuenfte=30000,
        kap_verlust_aktien=5000  # loss field is part of KAP_TOEPFE
    )
    assert result == "dba_kapital_offen"

# ---- Edge Cases ----

def test_no_dba_fields_no_gate():
    """No DBA fields → no gate"""
    result = _call_gate()
    assert result is None

def test_only_method_no_state_no_gate():
    """Only method field, no state → no multi-country gate"""
    result = _call_gate(dba_methode="dba_anrechnung")
    assert result is None

def test_zero_values_no_gate():
    """Zero DBA values → no gate"""
    result = _call_gate(
        dba_staat="Deutschland",
        dba_gezahlte_auslaendische_steuer=0,
        dba_auslaendische_einkuenfte=0
    )
    assert result is None

# ---- Test Summary for Required Countries ----
def test_required_countries_single():
    """Test that each country from task list behaves as single-country (no gate)"""
    required_countries = ["Deutschland", "Österreich", "Schweiz", "USA", "Kanada", "Grossbritannien"]
    for country in required_countries:
        result = _call_gate(
            dba_staat=country,
            dba_gezahlte_auslaendische_steuer=1500,
            dba_auslaendische_einkuenfte=75000
        )
        assert result is None, f"Required country {country} incorrectly triggered gate: {result}"

def test_required_countries_multi():
    """Test that combining any two required countries triggers multi-country gate"""
    result = _call_gate(
        dba_staat="Deutschland",
        dba_mehrere_staaten=True,
        dba_gezahlte_auslaendische_steuer=1000,
        dba_auslaendische_einkuenfte=50000
    )
    assert result == "dba_multi_country_offen"

if __name__ == "__main__":
    test_single_country_no_gate()
    test_single_country_with_method_override_no_gate()
    test_multi_country_gate()
    test_multi_country_gate_with_different_countries()
    test_capital_conflict_gate()
    test_capital_conflict_gate_with_loss()
    test_no_dba_fields_no_gate()
    test_only_method_no_state_no_gate()
    test_zero_values_no_gate()
    test_required_countries_single()
    test_required_countries_multi()
    print("All multi-country gate tests passed!")