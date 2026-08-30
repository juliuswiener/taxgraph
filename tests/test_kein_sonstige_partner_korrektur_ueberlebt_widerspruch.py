"""HTTP-Messung (server.py, echter Prozess): Parallelfall zu kap_partner (s. Moduldoc
tests/test_kein_kap_partner_korrektur_ueberlebt_widerspruch.py) fuer die zweite von @main
freigegebene FLAG_NEGIERT-Familie -- kein_sonstige_partner <-> rentner_jahresrente_partner.

**Beide Tests hier sind xfail(strict=True), nicht grün.** Gemessen 2026-08-31 (HEAD 5af945c):
kein_sonstige_partner (Flag, PARTNER_SCREENING in api_constants.py) ist ausschliesslich auf
Scheibe "gesamt" postbar; rentner_jahresrente_partner (Ziel, RENTNER_22_PARTNER via
RENTNER_FELDER) ausschliesslich auf Scheibe "rentner_gesamt". api.py::event() Zeile 493 laesst
pro Fall nur Felder der EIGENEN Scheibe zu (HTTP 400 sonst) -- ein Fall ist an EINE Scheibe fuer
seine gesamte Lebensdauer gebunden, Flag und Ziel koennen also nie im selben Fall-Snapshot
koexistieren. flag_check.py:FLAG_NEGIERT["kein_sonstige_partner"] ist deshalb nicht falsch,
sondern unfeuerbar (s. Kommentar dort). Diese zwei xfails sind der Melder: bleibt der
Scheiben-Zuschnitt disjunkt, bleiben sie XFAIL; wird er es je nicht mehr, kippen sie zu XPASS und
schlagen (strict=True) -- dann ist der FLAG_NEGIERT-Eintrag scharf und muss neu geprueft werden.

Scheibe rentner_gesamt fuer beide Tests (nicht "gesamt", wo das Flag lebt): das ist die Scheibe,
auf der das Zielfeld rentner_jahresrente_partner ueberhaupt existiert -- die Tests pruefen die
Korrektur-Kette DORT, wo sie inhaltlich hingehoert, und scheitern genau an der fehlenden
Erreichbarkeit des Flags dort.

Basis-Kegel fuer Person A direkt aus tests/test_rentner_partner_kegel_guard.py::_BASIS_ZUSAMMEN
uebernommen (dort bereits gruen verifiziert fuer denselben Scheiben/Veranlagung-Fall). Die vier
RENTNER_22_PARTNER-Kernfelder werden direkt per Event gesetzt statt per /fragen-Dialogwalk --
bescheid_deklaration.py:920-925 (Vollstaendigkeits-Guard "ALLE 4 oder KEINS") verlangt sie ohnehin
alle zusammen, api.py::event() prueft bei schreiber="ui:laie" weder Katalog noch Reihenfolge noch
Geltungsbedingung, nur Scheiben-Zugehoerigkeit.

Klartext-Anzeige (r.klartext von app.js::zeigeErgebnis() ignoriert) NICHT hier nochmal
versioniert -- s. Begruendung im Moduldoc der kap-Datei (78861cc deckt denselben
grund=flag_konsistenz_offen strukturell ab, kein neuer grund-Wert) -- ohnehin unerreichbar,
solange diese beiden Tests xfail sind.

NULL LLM.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API              # noqa: E402
import audit                   # noqa: E402
import server as SRV           # noqa: E402


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _req(base_url, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base_url + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, wert, ersetzt=None):
    d = {"feld_id": fld, "wert": wert, "zustand": "bestaetigt",
         "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
         "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}
    if ersetzt:
        d["ersetzt"] = ersetzt
    return d


# Person-A-Basis-Kegel, identisch zu test_rentner_partner_kegel_guard.py::_BASIS_ZUSAMMEN.
_BASIS_ZUSAMMEN = (
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
)

# Die vier RENTNER_22_PARTNER-Kernfelder (api_constants.py) -- der Vollstaendigkeits-Guard
# (bescheid_deklaration.py:920-925) verlangt alle 4 oder keins.
_RENTE_B_VOLL = (
    ("rentner_renten_art_partner", "gesetzliche_rente"),
    ("rentner_jahresrente_partner", 1800000),
    ("rentner_renten_beginn_jahr_partner", 2015),
    ("rentner_alter_bei_rentenbeginn_partner", 0),
)


def _anlegen(base_url, fall_id):
    st, b = _req(base_url, "POST", "/fall",
                 {"scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (st, b)


def _setze_basis(base_url, fall_id):
    for fld, w in _BASIS_ZUSAMMEN:
        st, r = _req(base_url, "POST", f"/fall/{fall_id}/event", _laie(fld, w))
        assert st == 201, (fld, st, r)


# ------------------------------------------------------ Melder: Disjunktheit haelt den Fix scharf

@pytest.mark.xfail(strict=True, reason=(
    "kein_sonstige_partner ist nur auf Scheibe 'gesamt' postbar (PARTNER_SCREENING), diese Kette "
    "laeuft auf 'rentner_gesamt' -- der erste Event-Post auf kein_sonstige_partner scheitert mit "
    "HTTP 400 'feld_id nicht in dieser Scheibe'. Gemessen 2026-08-31, HEAD 5af945c. Kippt dieser "
    "Test zu XPASS, ist das Flag auf rentner_gesamt erreichbar geworden -- dann greift der "
    "FLAG_NEGIERT-Eintrag scharf und der Rest dieses Tests ist zu pruefen."))
def test_rentner_partner_korrektur_widerspruch_wird_gesperrt(base):
    """Zielverhalten, sobald die Disjunktheit (s. Moduldoc) behoben ist: Partner-Rente eintragen,
    danach kein_sonstige_partner auf True korrigieren ("mein Partner hat doch keine Rente") --
    darf den zurueckgezogenen Betrag nicht weiterzaehlen, muss auf flag_konsistenz_offen sperren
    (analog zum bereits gefixten kein_kap_partner)."""
    fall_id = "rentner_partner_defekt"
    _anlegen(base, fall_id)
    _setze_basis(base, fall_id)
    st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_sonstige_partner", False))
    assert st == 201, r
    ks_event_id = r["event_id"]
    for fld, w in _RENTE_B_VOLL:
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fld, w))
        assert st == 201, (fld, st, r)

    st, erg1 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg1.get("grund") == "bestaetigt", erg1
    baseline = erg1.get("zahl_cent")
    assert isinstance(baseline, int) and baseline > 0, (
        f"Ausgangswert ist kein positiver Cent-Betrag ({baseline!r}) -- die Vergleichszahl "
        f"unten misst dann nicht mehr denselben Fall.")

    st, r = _req(base, "POST", f"/fall/{fall_id}/event",
                 _laie("kein_sonstige_partner", True, ersetzt=ks_event_id))
    assert st == 201, ("Korrektur abgelehnt", st, r)

    st, erg2 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200, erg2
    assert erg2.get("grund") == "flag_konsistenz_offen" and erg2.get("zahl_cent") is None, (
        f"Der Widerspruch (kein_sonstige_partner=True + rentner_jahresrente_partner=18000 EUR, "
        f"beide bestaetigt) bleibt unentdeckt: zahl_cent={erg2.get('zahl_cent')} Cent, "
        f"grund={erg2.get('grund')!r} -- die Rente auf zurueckgezogene Partner-Einkuenfte zaehlt "
        f"weiter, statt wie bei Person A zu sperren.")


# ------------------------------------------------------------------- Gegenprobe, dieselbe Ursache

@pytest.mark.xfail(strict=True, reason=(
    "Selbe Ursache wie oben: kein_sonstige_partner ist auf Scheibe 'rentner_gesamt' nicht postbar "
    "(HTTP 400), schon der erste Event-Post in diesem Test scheitert. Die urspruengliche Annahme "
    "'die andere Richtung funktioniert schon richtig' (bindung_rentner.yaml feld_bedingung) war "
    "falsch verallgemeinert -- die feld_bedingung dort kann auf dieser Scheibe nie True werden, "
    "weil das Flag, auf das sie verweist, hier nie gesetzt werden kann. Gemessen 2026-08-31, HEAD "
    "5af945c."))
def test_rentner_partner_korrektur_richtung_offen_zu_gesetzt_bleibt_korrekt(base):
    """Zielverhalten, sobald die Disjunktheit (s. Moduldoc) behoben ist: erst kein_sonstige_partner
    bestaetigen (Partner-Rente-Felder muessen ausgeschlossen sein), dann zurueckkorrigieren (Felder
    muessen wieder askable werden), dann eintragen -- muss zu einem korrekten bestaetigten Ergebnis
    fuehren. Ein Fix fuer den Defekt in der Schwesterfunktion darf diese Richtung nicht brechen."""
    fall_id = "rentner_partner_kontrolle_umgekehrt"
    _anlegen(base, fall_id)
    _setze_basis(base, fall_id)

    st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_sonstige_partner", True))
    assert st == 201, ("kein_sonstige_partner", st, r)
    ks_event_id = r["event_id"]

    st, stand = _req(base, "GET", f"/fall/{fall_id}/fragen")
    assert st == 200
    offen_ids = [f["feld_id"] for f in stand["fragen"]]
    assert "rentner_jahresrente_partner" not in offen_ids, (
        "rentner_jahresrente_partner ist trotz kein_sonstige_partner=True in der Warteschlange -- "
        "die feld_bedingung greift nicht mehr, dann prueft dieser Test die falsche Ausgangslage.")

    st, r = _req(base, "POST", f"/fall/{fall_id}/event",
                 _laie("kein_sonstige_partner", False, ersetzt=ks_event_id))
    assert st == 201, ("Korrektur abgelehnt", st, r)

    st, stand2 = _req(base, "GET", f"/fall/{fall_id}/fragen")
    assert st == 200
    offen_ids2 = [f["feld_id"] for f in stand2["fragen"]]
    assert "rentner_jahresrente_partner" in offen_ids2, (
        "Nach der Korrektur ist rentner_jahresrente_partner nicht in der Warteschlange -- dann "
        "waere der gemessene Ausweg keiner.")

    for fld, w in _RENTE_B_VOLL:
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fld, w))
        assert st == 201, (fld, st, r)

    st, erg = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg.get("grund") == "bestaetigt", erg
    assert isinstance(erg.get("zahl_cent"), int) and erg["zahl_cent"] > 0, (
        f"erwartet grund=bestaetigt mit positivem Betrag (der eingetragene Wert zaehlt korrekt), "
        f"tatsaechlich grund={erg.get('grund')!r} zahl_cent={erg.get('zahl_cent')}")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
