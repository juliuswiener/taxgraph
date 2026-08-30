"""HTTP-Messung (server.py, echter Prozess, kein Fixture-Kurzschluss): wird `kein_kap_partner`
per Event auf True gesetzt, BEVOR die fuenf KAP-Partner-Detailfelder (bescheid_deklaration.py,
GESAMT_PARTNER_KAP-Pflichtkegel fuer die gesamt-Scheibe) je gefragt wurden, sperrt der
`partner_kegel_offen`-Guard, und die normale Fragen-Warteschlange leert sich, OHNE dass eines der
fuenf Felder je angeboten wurde — der Traverser nimmt sie per `feld_bedingung: {feld:
kein_kap_partner, wert: false}` (traverser.py, `_feld_ausgeschlossen`) aus der Queue, sobald das
Kreuz bestaetigt ist. Ueber die Warteschlange kommt der Nutzer an dieser Stelle nicht weiter.

Ein Korrektur-Event auf GENAU dieses Kreuz (`ersetzt=<event_id>`, Auflage B in store.py:
`append_event`) kehrt die `feld_bedingung` um und stellt die fuenf Felder sofort zurueck in die
Warteschlange — danach ist der Sperrgrund nach ihrer Beantwortung weg. Getestet wird deshalb
BEIDES: der Stillstand UND der Ausweg. Ein Test, der nur den Stillstand pruefte, waere nicht
falsch, aber schaerfer als der Code hergibt, sobald man ihn "Sackgasse" nennt — es gibt einen
Weg zurueck, nur nicht ueber die normale Reihenfolge.

Gemessen bei HEAD c745e596b5bb10a6c6120925db87c54ffeb92468 (2026-08-30), per echter HTTP-API
gegen server.py (POST /fall, /event; GET /fragen, /ergebnis) — kein Mock, kein direkter
Store-Zugriff. Die Reihenfolge, in der der Fragebogen `kein_kap_partner` VOR den Detailfeldern
sieht, entsteht hier ueber ein direktes Event (wie es z.B. ein per LLM-Vorschlag im "Verstanden"-
Panel bestaetigtes Feld taete: `api.py::event()` prueft bei `schreiber="ui:laie"` weder Katalog
noch Reihenfolge) — nicht ueber die Klick-Reihenfolge des Fragebogens selbst.

NULL LLM.
"""
from __future__ import annotations

import json
import os
import re._constants as _src  # noqa: E402
import re._parser as _srep    # noqa: E402
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
import api_constants as APC    # noqa: E402
import audit                   # noqa: E402
import server as SRV           # noqa: E402

KAP_PARTNER_FELDER = APC.GESAMT_PARTNER_KAP   # die fuenf Pflichtfelder, live aus dem Produktcode

EXPLIZIT = {
    "veranlagung": "zusammen",
    "bruttoarbeitslohn": 6000000,     # 60.000 EUR, Cent
    "kein_gewinn": True, "kein_kap": True, "kein_vuv": True, "kein_sonstige": True,
}
MAX_FRAGEN = 600   # Reissleine: der Dialog muss ENDEN, nicht nur antworten


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


# -- Antwort-Generator: baut fuer JEDE Frage im Dialog einen bindungstyp-konformen Wert (Auflage T,
# store.py:_pruefe_typ_konformitaet, greift real ueber /event). "text"-Felder mit einem `muster`-
# Regex (z.B. Datumsbereiche) brauchen einen zum Muster passenden Wert, nicht irgendeine Zeichen-
# kette -- deshalb der kleine Regex-Sample-Generator darunter (deckt Literale, Zeichenklassen,
# \d/\w/\s, Wiederholung, Gruppen, erste Alternative -- keine Allgemeinheits-Garantie, reicht aber
# fuer die im Fragebogen vorkommenden Muster).
def _gen_category(cat):
    name = str(cat)
    if "DIGIT" in name and "NOT" not in name:
        return "1"
    if "WORD" in name and "NOT" not in name:
        return "a"
    if "SPACE" in name and "NOT" not in name:
        return " "
    return "x"


def _gen_in(av):
    negate = av and av[0][0] is _src.NEGATE
    items = av[1:] if negate else av
    if negate:
        return "z"
    for op, val in items:
        if op is _src.LITERAL:
            return chr(val)
        if op is _src.RANGE:
            return chr(val[0])
        if op is _src.CATEGORY:
            return _gen_category(val)
    return "x"


def _gen_parsed(parsed):
    out = []
    for op, av in parsed:
        if op is _src.LITERAL:
            out.append(chr(av))
        elif op is _src.NOT_LITERAL:
            out.append("_")
        elif op is _src.ANY:
            out.append("x")
        elif op is _src.IN:
            out.append(_gen_in(av))
        elif op is _src.CATEGORY:
            out.append(_gen_category(av))
        elif op in (_src.MAX_REPEAT, _src.MIN_REPEAT):
            mn, _mx, item = av
            for _ in range(mn if mn > 0 else 1):
                out.append(_gen_parsed(item))
        elif op is _src.SUBPATTERN:
            out.append(_gen_parsed(av[3]))
        elif op is _src.BRANCH:
            out.append(_gen_parsed(av[1][0]))
        # AT (Anker), ASSERT/ASSERT_NOT: ignoriert, kein Zeichen im Ergebnis.
    return "".join(out)


def _antwort_fuer(frage: dict):
    fid = frage["feld_id"]
    if fid in EXPLIZIT:
        return EXPLIZIT[fid]
    typ = frage.get("typ")
    if typ == "bool":
        return False
    if typ == "enum":
        werte = frage.get("enum_werte") or []
        return werte[0] if werte else None
    if typ == "text":
        muster = frage.get("muster")
        if muster:
            return _gen_parsed(_srep.parse(muster))
        return "x"
    if typ == "datum":
        return "01.01.2000"
    return 0   # cent/int


def _durchklicken(base_url, fall_id):
    """Beantwortet den Dialog ab dem aktuellen Fall-Stand bis /fragen leer ist. Gibt die gestellten
    Feld-IDs zurueck (vorab per Event bestaetigte Felder tauchen hier nicht auf)."""
    gestellt = []
    for _ in range(MAX_FRAGEN):
        st, b = _req(base_url, "GET", f"/fall/{fall_id}/fragen")
        assert st == 200, (st, b)
        fragen = b["fragen"]
        if not fragen:
            return gestellt
        frage = fragen[0]
        fid = frage["feld_id"]
        wert = _antwort_fuer(frage)
        assert wert is not None, f"keine Antwort fuer {fid} (typ={frage.get('typ')!r})"
        st, r = _req(base_url, "POST", f"/fall/{fall_id}/event", _laie(fid, wert))
        assert st == 201, (fid, st, r)
        gestellt.append(fid)
    raise AssertionError(f"Dialog endet nach {MAX_FRAGEN} Fragen nicht.")


def test_kein_kap_partner_vorab_sperrt_bis_zur_korrektur(base):
    fall_id = "sackgasse_gesamt"
    st, b = _req(base, "POST", "/fall",
                 {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (st, b)

    for fld, w in EXPLIZIT.items():
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fld, w))
        assert st == 201, (fld, st, r)

    # Das Kreuz VOR den Detailfeldern bestaetigen -- ausserhalb der Warteschlangen-Reihenfolge,
    # so wie es z.B. die Bestaetigung eines LLM-Vorschlags im "Verstanden"-Panel taete.
    st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_kap_partner", True))
    assert st == 201, ("kein_kap_partner", st, r)
    kreuz_event_id = r["event_id"]

    gestellt = _durchklicken(base, fall_id)
    assert gestellt, "Der Dialog hat nach der Vorab-Bestaetigung keine einzige Frage gestellt."
    assert "kein_kap_partner" not in gestellt, (
        "kein_kap_partner wurde erneut gefragt, obwohl es vorab bestaetigt wurde.")
    fehlend_gestellt = [f for f in KAP_PARTNER_FELDER if f in gestellt]
    assert not fehlend_gestellt, (
        f"Diese Felder wurden doch gefragt: {fehlend_gestellt} -- dann waere der Kegel ueber die "
        f"Warteschlange erfuellbar und der Test misst den falschen Zustand.")

    st, stand = _req(base, "GET", f"/fall/{fall_id}/fragen")
    assert st == 200 and not stand["fragen"], (
        "Warteschlange nach Durchklicken nicht leer -- unvollstaendiger Lauf, kein Stillstands-Beweis.")

    st, erg = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg.get("grund") == "partner_kegel_offen", (
        f"Erwartet grund=partner_kegel_offen (die fuenf KAP-Partner-Felder wurden nie gefragt). "
        f"Tatsaechlich: {erg.get('grund')!r}. Wenn das nicht mehr partner_kegel_offen ist, ist "
        f"dieser Guard-Zustand behoben -- dann darf dieser Test nicht mehr rot sein.")

    # -- DER AUSWEG: ein Korrektur-Event auf das Kreuz selbst (ersetzt=<event_id>) oeffnet die
    # fuenf Felder sofort wieder.
    st, r = _req(base, "POST", f"/fall/{fall_id}/event",
                 _laie("kein_kap_partner", False, ersetzt=kreuz_event_id))
    assert st == 201, ("Korrektur-Event abgelehnt", st, r)

    st, stand2 = _req(base, "GET", f"/fall/{fall_id}/fragen")
    offen_ids = [f["feld_id"] for f in stand2["fragen"]]
    fehlt_nach_korrektur = [f for f in KAP_PARTNER_FELDER if f not in offen_ids]
    assert not fehlt_nach_korrektur, (
        f"Nach der Korrektur fehlen diese Felder in der Warteschlange: {fehlt_nach_korrektur} -- "
        f"dann waere der gemessene Ausweg keiner.")

    for fid in KAP_PARTNER_FELDER:
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fid, 0))
        assert st == 201, (fid, st, r)

    st, erg2 = _req(base, "GET", f"/fall/{fall_id}/ergebnis")
    assert st == 200 and erg2.get("grund") != "partner_kegel_offen", (
        f"Nach Korrektur+Beantwortung haengt der Sperrgrund immer noch an partner_kegel_offen "
        f"({erg2.get('grund')!r}) -- dann waere der gemessene Ausweg keiner.")
