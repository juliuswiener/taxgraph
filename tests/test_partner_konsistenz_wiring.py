"""Wiring-Gate: partner_check (dev-2) im Haut-Guard _an_gesamt_sperrgrund verdrahtet.

partner_check.partner_ohne_zusammen selbst hat dev-2s Unit-Tests; hier wird nur belegt, dass die Haut
den Widerspruch als grund `partner_konsistenz_offen` surft (analog flag_check → flag_konsistenz_offen).
Der Guard ist forward-ready: aktuell führt KEINE Scheibe die rentner_*_partner-Felder, also feuert er in
Produktion (noch) nicht — der Unit-Test speist die Felder synthetisch, um die Verdrahtung festzunageln.
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API   # noqa: E402


def _snap(**felder):
    return {fid: {"wert": w, "zustand": z} for fid, (w, z) in felder.items()}


def test_partner_behinderung_ohne_zusammen_sperrt():
    """Partner-GdB gesetzt + veranlagung einzel → partner_konsistenz_offen (kein stiller Durchgriff)."""
    felder = _snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"),
                   veranlagung=("einzel", "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder) == "partner_konsistenz_offen"


def test_partner_merkzeichen_ohne_zusammen_sperrt():
    """Auch das Merkzeichen-Flag (hilflos/blind/taubblind Partner) triggert den Guard."""
    felder = _snap(rentner_hilflos_blind_taubblind_partner=(True, "bestaetigt"),
                   veranlagung=("einzel", "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder) == "partner_konsistenz_offen"


def test_partner_behinderung_mit_zusammen_kein_sperr():
    """Bei Zusammenveranlagung ist das Partner-Feld legitim → dieser Guard feuert NICHT."""
    felder = _snap(rentner_grad_der_behinderung_partner=(50, "bestaetigt"),
                   veranlagung=("zusammen", "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder) != "partner_konsistenz_offen"


def test_ohne_partner_feld_kein_sperr():
    """Kein Partner-Behinderungsfeld → der Guard bleibt still (inert für die Bestandsscheiben)."""
    felder = _snap(veranlagung=("einzel", "bestaetigt"))
    assert API._an_gesamt_sperrgrund(felder) != "partner_konsistenz_offen"
