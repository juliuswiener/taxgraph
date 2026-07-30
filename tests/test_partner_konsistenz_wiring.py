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


# ===== § 20 Abs. 9 — Veranlagungsart hat genau EINE Quelle ============
#
# Bis 2026-07-30 gab es neben `veranlagung` ein zweites Feld kap_zusammenveranlagung, das
# dieselbe Frage stellte. Standen beide im Widerspruch (veranlagung=einzel + Flag=true),
# verdoppelte sich der Sparer-Pauschbetrag, ohne dass Partner-Kapital addiert wurde:
# 250 € zu wenig Steuer bei 4.000 € Kapital. Das Feld ist entfernt — § 26 EStG kennt keine
# von der allgemeinen Veranlagungsart getrennte Kapital-Veranlagung.

def test_kap_zusammenveranlagung_ist_entfernt():
    """Das Feld darf nicht zurückkommen — sonst ist der Widerspruch wieder möglich."""
    import glob
    import yaml
    treffer = []
    for pfad in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(pfad, encoding="utf-8")) or {}
        for b in (d.get("bindungen") or []):
            if b["feld_id"] == "kap_zusammenveranlagung":
                treffer.append(os.path.basename(pfad))
    assert not treffer, (
        f"kap_zusammenveranlagung ist wieder gebunden ({treffer}) — die Veranlagungsart "
        f"hat genau eine Quelle: das Feld `veranlagung`.")


def test_veranlagung_bleibt_die_einzige_quelle():
    """Der Ring liest die Veranlagungsart nur aus `veranlagung`.

    Geprüft wird der ausgeführte Code, nicht die Kommentare — der Historien-Hinweis auf das
    entfernte Feld soll stehen bleiben dürfen.
    """
    pfad = os.path.join(ROOT, "produkt", "haut", "api.py")
    code = [z for z in open(pfad, encoding="utf-8")
            if "kap_zusammenveranlagung" in z and not z.lstrip().startswith("#")]
    assert not code, (
        "api.py liest wieder ein KAP-eigenes Veranlagungs-Flag:\n" + "".join(code))
