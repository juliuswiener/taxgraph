"""§33b im gesamt-Ring — Accessor-Komposition (Person A + Person B, additiv zu §33-agB).
1:1 gespiegelt vom Rentner-Ring (api.py:1002-1018). Deterministisch, NULL LLM.
"""

import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "golden"))
import runner as R  # noqa: E402

VZ = 2025


# -- §33b Behinderten-PB Person A (GdB-Staffel) ----------------------------------

def test_behinderten_gdb50():
    """GdB 50 → 1.140€ (§33b Abs.3 S.2 Staffel)."""
    pb = R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 50})
    assert pb == 1140


def test_behinderten_hilflos_blind():
    """Merkzeichen H/Bl/TBl → 7.400€ (ersetzt GdB-Staffel)."""
    pb = R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 30,
                                   "ist_hilflos_blind_taubblind": True})
    assert pb == 7400


def test_behinderten_unter_20_null():
    """GdB < 20 → kein Pauschbetrag."""
    assert R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 10}) == 0


# -- §33b Pflege-PB Person A -----------------------------------------------

def test_pflege_pg3():
    """Pflegegrad 3 → 1.100€ (§33b Abs.6 Staffel)."""
    assert R.catala_pflege_pb({"veranlagungszeitraum": VZ, "pflegegrad": 3}) == 1100


def test_pflege_hilflos_vorrang():
    """Hilflos → 1.800€ (Vorrang vor Pflegegrad-Staffel)."""
    assert R.catala_pflege_pb({"veranlagungszeitraum": VZ, "pflegegrad": 1,
                                "ist_hilflos": True}) == 1800


def test_pflege_ohne_pflegegrad_null():
    """Kein Pflegegrad → 0€."""
    assert R.catala_pflege_pb({"veranlagungszeitraum": VZ}) == 0


# -- §33b Hinterbliebenen-PB --------------------------------------------------

def test_hinterbliebenen():
    """Hinterbliebenenbezüge → 370€ (§33b Abs.4)."""
    assert R.catala_hinterbliebenen_pb({"veranlagungszeitraum": VZ,
                                         "hat_hinterbliebenenbezuege": True}) == 370


def test_keine_hinterbliebenen_null():
    """Keine Hinterbliebenenbezüge → 0€."""
    assert R.catala_hinterbliebenen_pb({"veranlagungszeitraum": VZ,
                                         "hat_hinterbliebenenbezuege": False}) == 0


# -- Person-A-Komposition (1:1 gespiegelt api.py gesamt slot_fn) ---------------

def test_komposition_behinderten_pflege_additiv():
    """GdB 50 (1140€) + Pflegegrad 3 (1100€) = 2.240€ additiv."""
    ausserg = (R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 50})
               + R.catala_pflege_pb({"veranlagungszeitraum": VZ, "pflegegrad": 3}))
    assert ausserg == 2240


def test_komposition_alle_drei_additiv():
    """GdB 50 (1140€) + Pflegegrad 3 (1100€) + Hinterbliebenen (370€) = 2.610€."""
    ausserg = (R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 50})
               + R.catala_pflege_pb({"veranlagungszeitraum": VZ, "pflegegrad": 3})
               + R.catala_hinterbliebenen_pb({"veranlagungszeitraum": VZ,
                                              "hat_hinterbliebenenbezuege": True}))
    assert ausserg == 2610


# -- Person-B (Ehegatte, nur Behinderung) -------------------------------------

def test_ehegatte_zusammen_additiv():
    """Person A GdB 50 (1140€) + Person B GdB 60 (1440€) = 2.580€ additiv."""
    ausserg = (R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 50})
               + R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 60}))
    assert ausserg == 2580


def test_ehegatte_hilflos():
    """Person A GdB 50 (1140€) + Person B hilflos/blind (7400€) = 8.540€."""
    ausserg = (R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 50})
               + R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 30,
                                          "ist_hilflos_blind_taubblind": True}))
    assert ausserg == 8540


# -- Additiv zu §33-agB -------------------------------------------------------

def test_mit_agb_additiv():
    """§33b-GdB50 (1140€) + §33-agB (500€ Einzelnachweis) = 1.640€ aussergewoehnliche_belastungen.
    agB-Wert hier direkt als Platzhalter — der echte catala_p33_agb-Aufruf ist separat getestet."""
    ausserg = R.catala_behinderten_pb({"veranlagungszeitraum": VZ, "grad_der_behinderung": 50})
    agb_anteil = 500  # Platzhalter — catala_p33_agb testet test_agb_kist_p33_p10.py
    assert ausserg + agb_anteil == 1640
