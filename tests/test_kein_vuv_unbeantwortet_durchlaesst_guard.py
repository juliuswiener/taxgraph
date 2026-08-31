"""K2-Guard-Lücke: `kein_vuv` nie beantwortet lässt `_an_gesamt_sperrgrund` durch, obwohl
`vv_einnahmen` bestätigt daneben steht.

Fund (Aufgabe E/F, 2026-08-31): `/ergebnis` sperrt diesen Zustand korrekt über einen ZWEITEN,
separaten Mechanismus (api.py::_ergebnis_roh, `_feste_zahl` liefert None -> grund=
"input_kegel_nicht_bestaetigt"). `/einreichen` kennt diesen zweiten Mechanismus nicht — es ruft
nur den GETEILTEN Guard `_an_gesamt_sperrgrund` (bescheid_deklaration.py, im Zweig
`if cfg and cfg.get("gesamt_guard"):`, Aufruf `FC.flag_widersprueche(felder)`). Dort steht
(produkt/konsistenz/flag_check.py, in `flag_widersprueche()`):

    if _bestaetigt_wert(snapshot, flag) is not True:
        continue                                    # Flag nicht bestätigt-true -> keine Behauptung

(Zeilennummer bewusst weggelassen — stand am 31.08. um 22 Zeilen verschoben, als ein anderer
Commit zwei Partner-Einträge vor diese Zeile einfügte, ohne die Zeile selbst zu berühren.)

`flag_widersprueche` prüft AUSSCHLIESSLICH den Fall „Flag EXPLIZIT auf true bestätigt UND ein
negiertes Feld > 0 bestätigt" (grund="flag_konsistenz_offen"). Den Fall „Flag NIE beantwortet UND
ein negiertes Feld > 0 bestätigt" prüft es nicht — das Flag ist dann `not True`, also `continue`,
ohne dass irgendein anderer Zweig in `_an_gesamt_sperrgrund` diesen Zustand sperrt.

Gemessen an ZWEI unabhängigen Wegen auf dieselbe Aufrufstelle (api.py::einreichen(), `if
cfg.get("guard"): sperr = _an_gesamt_sperrgrund(felder, cfg, vz, store, bindung)`):
  (a) direkter In-Process-Aufruf mit demselben Zustand, den `einreichen()` übergibt,
  (b) ein Pass-Through-Beobachter, der während eines ECHTEN `POST /fall/{id}/einreichen`
      (echtes HTTP, echtes ERiC/checkESt danach) unverändert mitschreibt, was `einreichen()`
      selbst an dieser Stelle berechnet.
Beide lieferten `None` für den Verdachtsfall und übereinstimmend `flag_konsistenz_offen` für die
Kontrollzeile (expliziter Widerspruch) — die Kontrollzeile bestätigt, dass an der richtigen Stelle
gemessen wurde. Unten stehen BEIDE Formen: die isolierten dict-Tests (schnell, kein HTTP) UND
dieselbe (a)-vs-(b)-Gegenprobe über den echten `/einreichen`-Endpunkt (Abgabepfad, Aufgabe F-Nachtrag
31.08.) — Letztere zeigt, dass der Befund nicht nur an der isolierten Funktion gilt, sondern am
tatsächlich vom Nutzer durchlaufenen Pfad.

WAS DIESE TESTS NICHT BEHAUPTEN: dass eine Abgabe tatsächlich rausginge (ERIC_ENCRYPT_AND_SEND ist
nicht verdrahtet, `/einreichen` sendet nie) und nicht, dass eine vollständige, ERiC-plausible
Erklärung mit diesem Zustand tatsächlich rc=0 erreichen würde — checkESt kann bei einer
vollständigeren Erklärung an anderer Stelle noch anschlagen, das wurde hier nicht gemessen. Der
Rückgabecode NACH dem Guard (rc, HTTP-Status der `/einreichen`-Antwort) ist absichtlich NICHT die
Aussage dieser Tests — er hängt an einer minimalen, nicht-abgabefertigen Fixture und wäre Rauschen.
Diese Tests behaupten ausschließlich: UNSER EIGENER Guard, der `einreichen()` VOR jeder ERiC-Runde
befragt, hält diesen Zustand für nicht sperrenswert — auf der isolierten Funktion UND am echten
Abgabepfad gemessen.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/haut", "produkt/import", "golden", "produkt/store", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import pytest

import api
import audit
import server as SRV


def test_kontrollzeile_expliziter_widerspruch_wird_gesperrt():
    """Kontrollzeile: kein_vuv EXPLIZIT bestätigt=True + vv_einnahmen bestätigt>0
    -> grund == "flag_konsistenz_offen". Bestätigt, dass der Guard an dieser Stelle überhaupt
    greifen KANN — ohne diese grüne Zeile würde die xfail-Zeile unten nichts beweisen."""
    felder = {
        "vv_einnahmen": {"wert": 2000000, "zustand": "bestaetigt"},
        "kein_vuv": {"wert": True, "zustand": "bestaetigt"},
    }
    # gesamt_guard: FC.flag_widersprueche() haengt in _an_gesamt_sperrgrund unter
    # `if cfg and cfg.get("gesamt_guard"):` (bescheid_deklaration.py:771) -- ohne dieses Flag wird
    # der Zweig nie erreicht (erste Fassung dieses Tests hatte nur "guard": True und schlug HIER
    # fehl, nicht bei der xfail-Zeile -- genau die Verwechslung, vor der die Kontrollzeile schuetzt).
    scheibe = {"guard": True, "gesamt_guard": True}

    grund = api._an_gesamt_sperrgrund(felder, scheibe, 2025, None, None)

    assert grund == "flag_konsistenz_offen", f"Kontrollzeile lief nicht — grund={grund!r}"


@pytest.mark.xfail(
    strict=True,
    reason="_an_gesamt_sperrgrund prueft nur explizite Widersprueche (Flag=true + Feld>0), "
           "nicht 'Flag nie beantwortet + Feld>0'. Der Guard sollte diesen Zustand ebenso "
           "sperren wie /ergebnis es (ueber einen ANDEREN Mechanismus) tut.",
)
def test_kein_vuv_nie_beantwortet_sollte_gesperrt_werden():
    """Verdachtsfall: kein_vuv fehlt ganz (nie beantwortet), vv_einnahmen bestätigt=20.000 EUR.
    Erwuenscht waere ein Sperrgrund (analog zu /ergebnis's 'input_kegel_nicht_bestaetigt') —
    _an_gesamt_sperrgrund liefert stattdessen None, weil flag_widersprueche() nur bestaetigt=true
    prueft, nie 'unbeantwortet'. /einreichen erreicht dadurch die ERiC-Pruefung mit einem nie
    bestaetigten Betrag im Vordruck (E0700201), ungesperrt durch unsere eigene Software."""
    felder = {
        "vv_einnahmen": {"wert": 2000000, "zustand": "bestaetigt"},
        # kein_vuv: absichtlich NICHT im Snapshot -- nie beantwortet
    }
    scheibe = {"guard": True, "gesamt_guard": True}

    grund = api._an_gesamt_sperrgrund(felder, scheibe, 2025, None, None)

    assert grund is not None, "erwarteter Sperrgrund fehlt -- Guard laesst unbeantwortetes kein_vuv durch"


# --------------------------------------------------------------------- Abgabepfad (echter HTTP-Endpunkt)
#
# Die zwei Tests oben pruefen die isolierte Funktion mit handgebauten Snapshots. Die zwei Tests
# unten fahren denselben Vergleich -- direkter Aufruf vs. Beobachtung -- am ECHTEN Abgabepfad:
# POST /fall/{id}/einreichen, derselbe Code, den ein Nutzer tatsaechlich durchlaeuft. Kein ERiC
# noetig (der Pass-Through-Beobachter greift VOR dem ERiC-Import ab, s. api.py::einreichen()) --
# beide Tests bleiben credential-frei gruen/xfail.

_HTTP_BASIS = (("stammdaten_iban", "DE02120300000000202051"),
               ("stammdaten_steuernummer", "9181081508155"))


@pytest.fixture
def _server(tmp_path, monkeypatch):
    """Eigener HTTP-Server pro Test, isoliertes FAELLE-Verzeichnis -- Muster aus
    tests/test_einreichen_durchstich.py::base."""
    monkeypatch.setattr(api, "FAELLE", str(tmp_path / "faelle"))
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


def _req(base, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, wert):
    return {"feld_id": fld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _direkter_abgriff(fall_id):
    """Dieselben Vorbereitungsschritte wie einreichen(), bis zur Guard-Aufrufstelle -- dann
    _an_gesamt_sperrgrund selbst aufrufen. Braucht KEINE vollstaendige Deklaration: der Guard
    sitzt in einreichen() VOR EM.deklariere()/EX.erzeuge_xml(), eine minimale Fixture reicht."""
    store = api.lade_fall(fall_id)
    bindung = api._scheibe_bindung(store)
    felder, _sid = api.ST.materialisiere(store)
    cfg = api._cfg(store)
    vz = int(store.get("veranlagungszeitraum") or 0)
    felder = api._mit_ring_werten(felder, vz)
    assert cfg.get("guard"), "cfg.get('guard') falsy -- Scheibe ruft den Guard gar nicht auf"
    return api._an_gesamt_sperrgrund(felder, cfg, vz, store, bindung)


def _fall_bauen(base, fall_id, mit_widerspruch_nachtrag):
    st, r = _req(base, "POST", "/fall",
                 {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id})
    assert st == 201, (st, r)
    for fld, wert in _HTTP_BASIS + (("veranlagung", "einzel"), ("vv_einnahmen", 2000000)):
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie(fld, wert))
        assert st == 201, (fld, st, r)
    if mit_widerspruch_nachtrag:
        # kein_vuv NACH vv_einnahmen setzen (Reihenfolge aus Aufgabe E beibehalten, s. Docstring
        # oben -- hier zwar unkritisch, weil wir /event direkt statt /fragen ansprechen, aber zur
        # Konsistenz mit der Konstruktion in Aufgabe E/F).
        st, r = _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_vuv", True))
        assert st == 201, (st, r)


def test_abgabepfad_kontrollzeile_direkt_und_beobachtet_stimmen_ueberein(_server, monkeypatch):
    """Kontrollzeile am echten Endpunkt: derselbe Widerspruch wie oben, aber durch POST
    /fall/{id}/einreichen gefahren statt per Hand-Dict. direkter und beobachteter Abgriff
    muessen uebereinstimmen UND 'flag_konsistenz_offen' zeigen -- sonst ist die Stelle falsch."""
    beobachtet = {}
    original = api._an_gesamt_sperrgrund

    def _pass_through(felder, cfg=None, vz=None, store=None, bindung=None):
        r = original(felder, cfg, vz, store, bindung)
        beobachtet["wert"] = r
        return r

    monkeypatch.setattr(api, "_an_gesamt_sperrgrund", _pass_through)

    _fall_bauen(_server, "abgabepfad_kontrolle", mit_widerspruch_nachtrag=True)
    direkt = _direkter_abgriff("abgabepfad_kontrolle")

    st, resp = _req(_server, "POST", "/fall/abgabepfad_kontrolle/einreichen", {})

    assert beobachtet.get("wert") == "flag_konsistenz_offen", (
        f"Beobachteter Guard-Wert am echten Endpunkt={beobachtet.get('wert')!r} -- "
        f"erwartet 'flag_konsistenz_offen'")
    assert direkt == beobachtet["wert"], (
        f"direkt={direkt!r} != beobachtet={beobachtet['wert']!r} -- Abgriff misst nicht dieselbe Stelle")
    # Zur Einordnung, NICHT die Kernaussage: der HTTP-Status/grund an der Oberflaeche zeigt
    # denselben Sperrgrund, weil der Guard hier VOR jeder anderen Pruefung 409 zurueckgibt.
    assert st == 409 and resp.get("grund") == "flag_konsistenz_offen", resp


@pytest.mark.xfail(
    strict=True,
    reason="Wie test_kein_vuv_nie_beantwortet_sollte_gesperrt_werden, aber am echten "
           "/einreichen-Endpunkt statt an der isolierten Funktion: der Guard laesst den "
           "Verdachtsfall auch im echten Abgabepfad unbestraft durch.",
)
def test_abgabepfad_verdachtsfall_direkt_und_beobachtet_stimmen_ueberein(_server, monkeypatch):
    """Verdachtsfall am echten Endpunkt: kein_vuv nie beantwortet, vv_einnahmen bestaetigt. Erwartet
    (fuer eine SPERRE): beobachteter Guard-Wert waere nicht None. Gemessen: er ist None -- der
    Guard haelt den Zustand VOR jeder ERiC-Runde fuer nicht sperrenswert. Was diese Messung nicht
    zeigt: den Rueckgabecode NACH dem Guard -- der ist Rauschen aus einer nicht-abgabefertigen
    Fixture (fehlende Pflichtfelder fuehren spaeter zu 'deklaration_unvollstaendig' -- ein ANDERES
    Gate, nicht dieses hier) und wird deshalb bewusst nicht geprueft."""
    beobachtet = {}
    original = api._an_gesamt_sperrgrund

    def _pass_through(felder, cfg=None, vz=None, store=None, bindung=None):
        r = original(felder, cfg, vz, store, bindung)
        beobachtet["wert"] = r
        return r

    monkeypatch.setattr(api, "_an_gesamt_sperrgrund", _pass_through)

    _fall_bauen(_server, "abgabepfad_verdacht", mit_widerspruch_nachtrag=False)
    direkt = _direkter_abgriff("abgabepfad_verdacht")

    _req(_server, "POST", "/fall/abgabepfad_verdacht/einreichen", {})

    assert direkt == beobachtet.get("wert"), (
        f"direkt={direkt!r} != beobachtet={beobachtet.get('wert')!r} -- Abgriff misst nicht dieselbe Stelle")
    assert beobachtet.get("wert") is not None, (
        "erwarteter Sperrgrund fehlt -- Guard laesst unbeantwortetes kein_vuv auch am echten "
        "Abgabepfad durch")
