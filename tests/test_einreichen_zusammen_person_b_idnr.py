"""Regression 2026-08-12: person_b_idnr (E0100082) darf nach dem ERiC-Ablehnungsfix weder den
/ergebnis- noch den /einreichen-Pfad sperren -- genau der Pfad, den
scripts/measure_person_b_idnr.py gemessen hat, hier als Test. Eigene Datei (nicht in
tests/test_einreichen_durchstich.py angehaengt), bewusst UNABHAENGIG vom parallelen
Dialog-Pfad-E2E-Test eines anderen Workers -- keine Koordination, keine gemeinsame Fixtur.

Vorgeschichte (BACKLOG person-b-idnr-wird-abgelehnt)
------------------------------------------------------
ERiC lehnt E0100082 amtlich ab (rc=610301106, "Nutzdaten enthalten das Feld ... mit dem
Eingefuegt-Kennzeichen J oder P"), unabhaengig vom Wert. Das Kennzeichen kommt in
E10-2025.xsd kein einziges Mal vor -- reine ERiC-interne Laufzeitregel, dem Schema
unsichtbar. Sweep ueber alle 10 Elemente mit demselben XSD-Typ (IDNrBaseCType_RABE): nur
E0100082 betroffen, kind_idnr (E0500406) sauber. Fix (commit 1420bb6): das Feld wird nicht
mehr deklariert (elster_kz: null, askable: false in bindung_an_gesamt.yaml) und nicht mehr
verlangt (raus aus AN_GESAMT_PARTNER + GESAMT_PARTNER_19 in api_constants.py) -- vorher
sperrte genau dieser Kegel jeden Splitting-Bescheid, sobald person_b_idnr fehlte
(grund="partner_kegel_offen", api.py:_an_gesamt_sperrgrund). Das war NIE ueber
tests/test_checkest_durchstich.py sichtbar, weil dessen _fall_zusammen() den echten HTTP-
Sperrgrund-Gate-Pfad gar nicht durchlaeuft (nur Funktionsebene: est_mapping.deklariere +
erzeuge_xml + checkest_gate.validate direkt, ohne api.py:einreichen()) -- 1864 gruene Tests
sahen den Blocker nicht.

Zwei Fixturen, nicht eine (Fixtur-Fund waehrend des Baus dieses Tests)
------------------------------------------------------------------------
Der erste Anlauf versuchte EINE Fixtur fuer beide Endpunkte. Das geht nicht, ohne einen
zweiten, unrelated Fund mitzuziehen -- deshalb zwei getrennte Person-A-Kegel:

1) GESAMT_PARTNER_KAP (api_constants.py) verlangt FUENF bestaetigte KAP-Partner-Felder
   (kap_kapitalertraege_partner, kap_gewinn_aktien_partner, kap_gewinn_sonstige_partner,
   kap_verlust_aktien_partner, kap_verlust_sonstige_partner) -- tests/test_checkest_durchstich.py
   ::_BASIS_A/_BASIS_B deklarieren je nur VIER davon (kap_gewinn_sonstige[_partner] fehlt),
   unbemerkt, weil dieser Test nie durch den HTTP-Sperrgrund-Gate lief. Beide Fixturen hier
   ergaenzen das fuenfte Feld -- sonst blockt "partner_kegel_offen" aus dem FALSCHEN Grund,
   und die Messung waere verfaelscht.

2) /ergebnis prueft eine ANDERE, STRENGERE Vollstaendigkeit als /einreichen: der Catala-Ring
   verlangt fuer SCHEIBEN["gesamt"]["kegel"] (api_constants.py) zusaetzlich bestaetigte
   EP_FELDER, KV_PV_FELDER und VV_GESAMT_FELDER -- die "kein_gewinn/kein_kap/kein_vuv/
   kein_sonstige"-Opt-out-Flags reichen NUR fuer est_mapping.deklariere() (die XML-Deklaration),
   NICHT fuer die Ring-Eingabe (erste Messung: grund="input_kegel_nicht_bestaetigt", obwohl
   kein_kap/kein_vuv bereits bestaetigt True waren). ABER: sobald diese Felder (selbst mit
   Wert 0) bestaetigt sind, deklariert est_mapping sie AUCH gegenueber checkESt (Anlage V /
   Entfernungspauschale) -- und checkESt verlangt dafuer Pflichtfelder, die mit dieser Aufgabe
   nichts zu tun haben (Lage des Grundstücks E0700407, Ziel-Adresse der ersten Taetigkeitsstaette
   E0203003, Mieteinnahmen-Aufschluesselung). Zweite Messung: rc=610001002
   [plausibilitaet_fehler] mit genau diesen 6 Beanstandungen, OBWOHL person_b_idnr korrekt nicht
   mehr sperrt. Eine dieser Fixturen fuer BEIDE Endpunkte zu nutzen wuerde also entweder
   /ergebnis (fehlender Ring-Input) oder /einreichen (fremde Anlage-V/EP-Pflichtfelder, ausserhalb
   dieses Auftrags) aus einem GRUND scheitern lassen, der nichts mit person_b_idnr zu tun hat --
   _BASIS_A_RING existiert nur fuer den /ergebnis-Test, _BASIS_A nur fuer /einreichen. Person B
   ist von dieser Unterscheidung nicht betroffen: ihre VOR/KV_PV-Felder sind additiv verdrahtet,
   nicht Teil des Kegels (api.py ~2048-2050), _BASIS_B ist fuer beide Tests identisch.

Ueberspringt sauber ohne ERiC/Hersteller-ID (credential-freies CI), und die /ergebnis-
Assertion separat ohne Catala-Runner (import runner). Die Hersteller-ID wird nie geloggt.
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
for _sub in ("produkt/haut", "golden", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API                # noqa: E402
import server as SRV             # noqa: E402
import audit                      # noqa: E402
import checkest_gate as CE       # noqa: E402


def _catala_da() -> bool:
    try:
        import runner  # noqa: F401
        return True
    except Exception:
        return False


def _hid() -> str | None:
    """Hersteller-ID aus der Umgebung, sonst aus der gitignoreten .env. Nie loggen."""
    hid = os.environ.get("ELSTER_HERSTELLER_ID")
    if hid:
        return hid
    pfad = os.path.join(ROOT, ".env")
    if not os.path.exists(pfad):
        return None
    for zeile in open(pfad, encoding="utf-8"):
        if zeile.startswith(("ELSTER_HERSTELLER_ID=", "HERSTELLER_ID=")):
            return zeile.split("=", 1)[1].strip().strip('"').strip("'") or None
    return None


_HID = _hid()
_ERIC_DA = bool(_HID) and os.path.isdir(
    os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric")))

braucht_eric = pytest.mark.skipif(
    not _ERIC_DA,
    reason="ERiC oder Hersteller-ID fehlt — amtliche Pruefung nicht lauffaehig "
           "(credential-freies CI)")


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _req(base, method, path, body=None, erwarte=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, f"erwarte={erwarte}, erhalten={status} {method} {path} {body}"
    elif status >= 500:
        raise AssertionError(f"Serverfehler {status} {method} {path} {body}: {content}")
    return status, content


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": {"typ": "laie_eingabe"}, "signal_2": "laie_bestaetigt"}}


# Person A -- identisch zu tests/test_checkest_durchstich.py::_STAMM_A/_BASIS_A (dort bereits
# gegen rc=0/RESTFEHLER_ZUSAMMEN==0 bewiesen), plus das fuenfte KAP-Feld (Fund 1 oben).
_STAMM_A = (("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
            ("stammdaten_geburtsdatum", "05.05.1955"),
            ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
            ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
            ("stammdaten_keine_bankverbindung", True),
            ("stammdaten_art_est_erklaerung", True),
            ("kist_konfession", "keine"),
            ("stammdaten_steuernummer", "9181081508155"),
            ("steuerklasse", "1"), ("p36_lohnsteuer", 1200000))

_BASIS_A = (("bruttoarbeitslohn", 6000000), ("vor_an_anteil_rv", 4200000),
            ("vor_ag_anteil_rv", 1200000), ("vor_rv_ausserhalb_lstb", 0),
            ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
            ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
            ("kein_gewinn", True), ("kein_kap", True),
            ("kein_vuv", True), ("kein_sonstige", True)) + _STAMM_A

# NUR fuer den /ergebnis-Test (Fund 2 oben): dieselbe Person A, zusaetzlich der volle
# Catala-Ring-Kegel (EP_FELDER, KV_PV_FELDER, VV_GESAMT_FELDER aus api_constants.py). NICHT
# fuer /einreichen verwenden -- deklariert Anlage V + Entfernungspauschale gegenueber
# checkESt und verlangt dort fremde Pflichtfelder (Adresse etc.), die dieser Auftrag nicht
# beantwortet.
_BASIS_A_RING = _BASIS_A + (
    ("ep_arbeitstage", 220), ("ep_entfernung_km", 30), ("ep_oepnv_kosten", 0),
    ("ep_eigenes_kfz", True),
    ("versicherungsart", "gesetzlich_an"), ("basis_kv", 0), ("basis_pv", 0),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
    ("vv_einnahmen", 0), ("vv_gebaeude_afa", 0), ("vv_schuldzinsen", 0),
    ("vv_erhaltungsaufwand", 0), ("vv_sonstige_wk", 0), ("vv_entgelt_quote_prozent", 0),
)

# Person B -- STAMMDATEN_FELDER_PARTNER (api_constants.py) verlangt NUR nachname/vorname/
# geburtsdatum/kist_konfession_partner. person_b_idnr steht ABSICHTLICH nicht hier -- genau
# das ist die Regression: der Splitting-Bescheid muss auch OHNE dieses Feld stehen. Fuer
# beide Tests identisch (Person Bs VOR/KV_PV-Felder sind additiv, nicht Teil des Kegels).
_STAMM_B = (("stammdaten_nachname_partner", "Maier"),
            ("stammdaten_vorname_partner", "Carolina"),
            ("stammdaten_geburtsdatum_partner", "09.07.1988"),
            ("kist_konfession_partner", "keine"),
            ("steuerklasse_partner", "5"), ("p36_lohnsteuer_partner", 1000000))

_BASIS_B = (("bruttoarbeitslohn_partner", 5000000),
            ("vor_an_anteil_rv_partner", 3500000),
            ("vor_ag_anteil_rv_partner", 1000000),
            ("vor_rv_ausserhalb_lstb_partner", 0),
            ("kap_kapitalertraege_partner", 0), ("kap_gewinn_aktien_partner", 0),
            ("kap_gewinn_sonstige_partner", 0),
            ("kap_verlust_aktien_partner", 0),
            ("kap_verlust_sonstige_partner", 0)) + _STAMM_B


def _fall_zusammen(base, fall_id, person_a=_BASIS_A):
    st, r = _req(base, "POST", "/fall", {"fall_id": fall_id,
                                         "scheibe": "gesamt", "veranlagungszeitraum": 2025},
                 erwarte=201)
    fid = r["fall_id"]
    for fld, w in person_a + _BASIS_B:
        _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w), erwarte=201)
    _req(base, "POST", f"/fall/{fid}/event", _laie("veranlagung", "zusammen"), erwarte=201)
    return fid


def test_ergebnis_ohne_person_b_idnr_liefert_echte_zahl(base):
    """/ergebnis: ohne person_b_idnr trotzdem ein echter Splitting-Bescheid, kein
    partner_kegel_offen mehr. Ohne Catala-Runner (lokale Envs ohne OPam) nur die Gate-Aussage
    pruefbar, sonst zusaetzlich eine konkrete Zahl."""
    fid = _fall_zusammen(base, "durchstich_http_ohne_idnr_ergebnis", person_a=_BASIS_A_RING)
    st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert erg.get("grund") != "partner_kegel_offen", (
        f"person_b_idnr sperrt wieder — Fix nicht wirksam: {erg}")
    if _catala_da():
        assert isinstance(erg.get("zahl_cent"), int) and erg["grund"] == "bestaetigt", (
            f"kein echter Splitting-Bescheid ohne person_b_idnr: {erg}")


@braucht_eric
def test_einreichen_ohne_person_b_idnr_erreicht_rc_0(base, monkeypatch):
    """/einreichen: echtes XML, echtes amtliches checkESt — rc=0 erwartet (das ist der
    zweite, staerkere Beweis: nicht nur 'nicht mehr blockiert', sondern amtlich abgabefaehig
    ohne dieses Feld). Eigene, schlankere Fixtur (_BASIS_A, s. Docstring "Fund 2")."""
    monkeypatch.setenv("ELSTER_HERSTELLER_ID", _HID)
    fid = _fall_zusammen(base, "durchstich_http_ohne_idnr_einreichen", person_a=_BASIS_A)

    st, resp = _req(base, "POST", f"/fall/{fid}/einreichen", {})

    assert resp.get("grund") != "partner_kegel_offen", (
        f"person_b_idnr sperrt /einreichen wieder — Fix nicht wirksam: {resp}")
    assert resp.get("grund") != "deklaration_unvollstaendig", (
        f"Fixtur unvollstaendig, Test misst den falschen Pfad: {resp}")
    assert resp.get("grund") != "xml_nicht_baubar", f"XML-Bau gescheitert: {resp}"
    assert resp.get("grund") != "eric_nicht_verfuegbar", f"ERiC nicht ladbar: {resp}"

    rc = resp.get("rc")
    klasse = resp.get("klasse")
    assert klasse != "io_gate_nicht_geprueft", (
        f"rc={rc}: XML bricht VOR der Plausibilitaetspruefung ab (leerer Fehlerpuffer sieht "
        f"aus wie Erfolg, ist aber keiner). Antwort: {resp}")

    assert rc == CE.RC_OK and klasse == "plausibel", (
        f"rc={rc} [{klasse}] statt 0/plausibel — person_b_idnr fehlt in dieser Fixtur "
        f"absichtlich, das darf checkESt nicht mehr beanstanden. Antwort: {resp}")
    assert st == 200 and resp.get("plausibel") is True, resp
