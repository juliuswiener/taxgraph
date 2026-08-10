"""Gate: Rentner-Partner-Kegel (rentner_gesamt, veranlagung=zusammen).

Anlass 2026-08-10, BACKLOG rentner-gesamt-partner-kegel-ungeschuetzt: rentner_gesamt führte
KEIN "partner_19"-Äquivalent (api_constants.py) — 28 _partner-Felder waren erreichbar, ohne dass
irgendeine Kombination geprüft wurde. Systematischer Sweep (team-lead) fand zwei konkrete Lücken:

  (a) CRASH statt Guard: rentner_renten_art_partner allein (ohne rentner_renten_beginn_jahr_partner)
      lief ungefangen bis in den Ring und crashte mit HTTP 500 ("RentenfreibetragFixierungOffen").
      Der bestehende Fixierungs-Guard griff nicht: _fixierung_offen(beginn=None) liefert False
      (kein isinstance(None, int)).
  (b) Fail-open: basis_kv_partner/basis_pv_partner bewegten Geld (-810/-180 EUR gemessen) OHNE dass
      versicherungsart_partner (die Kz-VERZWEIGUNG für ELSTER, est_mapping.PARTNER_VERZWEIGUNG)
      beantwortet war.

Fix in api.py::_an_gesamt_sperrgrund (cfg.get("rentner")-Block): (a) Renten-Gruppe RENTNER_22_PARTNER
muss ALLE 4 Kernfelder bestätigt haben oder KEINS (sonst rente_instanz_offen, dieselbe Semantik wie
die multi_rente-Instanz-Vollständigkeit für Person A) — wandelt den Crash in einen Sperrgrund um.
(b) versicherungsart_partner muss bestätigt sein, wenn basis_kv_partner/basis_pv_partner gesetzt ist
(sonst partner_kegel_offen).

Explizit NICHT gegated (A.2-Präzedenzfall, api.py-Kommentar bei partner_19): die individuellen
Vorsorge-Einzelfelder (VOR_PARTNER_FELDER, vorsorge_*_partner) bleiben unabhängig abzugsfähig — ein
Partner ohne eigenes Renten-/Kapital-Einkommen bei Zusammenveranlagung ist der gesetzliche Normalfall
(Splitting-Vorteil, § 26b), kein Sperrgrund (Gate-Polaritäts-Falle, 519199e-Präzedenz).
"""

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/import", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API              # noqa: E402
import audit                   # noqa: E402


@pytest.fixture(autouse=True)
def _isoliert(tmp_path, monkeypatch):
    """Fälle in tmp_path statt ins echte produkt/haut/faelle/ — sonst kollidieren Wiederholungsläufe."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


_BASIS_ZUSAMMEN = [
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 2000000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 0),
    ("rentner_grad_der_behinderung", 0), ("rentner_hilflos_blind_taubblind", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("rentner_hinterbliebenenbezuege", False), ("veranlagung", "zusammen"),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("versicherungsart", "gesetzlich_an"), ("basis_kv", 0), ("basis_pv", 0),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]


def _anlegen_und_fuellen(fall_id, zusatz_felder):
    st, _ = API.fall_anlegen({"fall_id": fall_id, "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025})
    assert st == 201
    for fid, wert in _BASIS_ZUSAMMEN + zusatz_felder:
        st, resp = API.event(fall_id, _laie(fid, wert))
        assert st == 201, f"{fid}: {st} {resp}"
    return API.ergebnis(fall_id)


# ---- (a) Renten-Gruppe: Teilangabe sperrt statt zu crashen ----

def test_rente_partner_teilangabe_crasht_nicht_und_wird_gesperrt():
    """rentner_renten_art_partner ALLEIN (ohne beginn_jahr_partner) — vorher HTTP 500, jetzt Sperrgrund."""
    st, erg = _anlegen_und_fuellen("teil_rente", [("rentner_renten_art_partner", "gesetzliche_rente")])
    assert st == 200, "darf nicht crashen (kein 500) — muss als reguläres Ergebnis mit Sperrgrund zurückkommen"
    assert erg["grund"] == "rente_instanz_offen", (
        f"unvollständige Partner-Renten-Gruppe muss sperren, grund={erg.get('grund')!r}")
    assert erg["zahl_cent"] is None


def test_rente_partner_vollstaendig_wird_nicht_gesperrt():
    """Gegenprobe: ALLE 4 Renten-Kernfelder bestätigt → kein Sperrgrund (sonst wäre der Guard über-scharf)."""
    st, erg = _anlegen_und_fuellen("voll_rente", [
        ("rentner_renten_art_partner", "gesetzliche_rente"),
        ("rentner_jahresrente_partner", 1500000),
        ("rentner_renten_beginn_jahr_partner", 2025),
        ("rentner_alter_bei_rentenbeginn_partner", 65),
    ])
    assert erg["grund"] == "bestaetigt", f"vollständige Partner-Rente darf nicht sperren: {erg}"
    assert isinstance(erg["zahl_cent"], int)


def test_rente_partner_komplett_leer_wird_nicht_gesperrt():
    """Gegenprobe (Gate-Polarität): Partner OHNE eigene Rente bei zusammen ist der Normalfall
    (§ 26b Splitting-Vorteil auch bei einseitigem Einkommen) — darf NICHT sperren."""
    st, erg = _anlegen_und_fuellen("leer_rente", [])
    assert erg["grund"] == "bestaetigt", f"Partner ganz ohne Rente darf nicht sperren: {erg}"
    assert isinstance(erg["zahl_cent"], int)


# ---- (b) KV/PV-Weiche: Betrag ohne Art sperrt ----

def test_kv_pv_partner_ohne_versicherungsart_wird_gesperrt():
    """basis_kv_partner OHNE versicherungsart_partner — die Kz-Weiche ist unbeantwortet."""
    st, erg = _anlegen_und_fuellen("kvpv_offen", [("basis_kv_partner", 350000)])
    assert erg["grund"] == "partner_kegel_offen", (
        f"basis_kv_partner ohne versicherungsart_partner muss sperren, grund={erg.get('grund')!r}")
    assert erg["zahl_cent"] is None


def test_kv_pv_partner_mit_versicherungsart_wird_nicht_gesperrt():
    """Gegenprobe: versicherungsart_partner mitbeantwortet → kein Sperrgrund."""
    st, erg = _anlegen_und_fuellen("kvpv_voll", [
        ("basis_kv_partner", 350000), ("versicherungsart_partner", "gesetzlich_freiwillig"),
    ])
    assert erg["grund"] == "bestaetigt", f"basis_kv_partner MIT versicherungsart_partner darf nicht sperren: {erg}"
    assert isinstance(erg["zahl_cent"], int)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
