"""§ 24a: Bei offenem Geburtsdatum fragt die Bindung das GEBURTSJAHR — nicht ein Feld, das niemand setzen kann.

Vault: decisions/die-rentner-erklaerung-nennt-bei-offenem-alter-das-geburtsjahr.md,
backlog/taxgraph/p24a-rueckfall-nennt-falsches-feld.md (Julius 2026-10-03, „5: 1", Form B).

Die Bindung deklarierte `rentner_alter_64_erfuellt` (aus dem Geburtsdatum abgeleitet) als Rückfall,
gefragt wird aber `geburtsjahr`. Das abgeleitete Feld steht in keiner Scheibe: ein POST darauf endet
mit 400. Form B: das Feld bleibt als abgeleitete Größe, ist nicht mehr `askable` und keine
`eingangsfrage`; seine Erklärung und die der Regelbedingung nennen `geburtsjahr`.

Zwei Ebenen, weil sie Verschiedenes sagen:
  * API-Ebene (Scheibe `gesamt`, VZ 2025, KEGEL_BASIS + beide § 33-Tatbestandsmerkmale): die Zahlen der
    Sonde vom 2026-09-26. Sie MÜSSEN vor und nach dem Bau gleich sein; am Stand vor dem Bau (620f5a4)
    gemessen. Die Scheiben-Bindung enthält das abgeleitete Feld nicht, die Zahl hängt nur an `geburtsjahr`.
  * Voller Bindungsbestand (so laufen die Unit-Tests und der Traverser ohne Scheibe): dort feuert die
    Ableitung und dort stand das Feld als nie beantwortbares Gate in der Queue. Vor dem Bau stand die Regel
    nach einer Antwort nur auf `geburtsjahr` weiter auf „unentschieden" (offenes Gate, nie beantwortbar).

NULL LLM.
"""
import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/traverser", "golden", "tests"):
    sys.path.insert(0, os.path.join(ROOT, _sub))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))

import api as API        # noqa: E402
import audit             # noqa: E402
import server as SRV     # noqa: E402
import store as ST       # noqa: E402
import traverser as TR   # noqa: E402
from test_p33b_abs5_s4_ring import KEGEL_BASIS  # noqa: E402 — dieselbe Basis wie die Sonde

FELD = "rentner_alter_64_erfuellt"
REGEL = "p24a_altersentlastungsbetrag"
BINDUNG = TR.lade_bindung()
KATALOG = ST.lade_katalog(BINDUNG)


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


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


def _post(base, pfad, body):
    req = urllib.request.Request(base + pfad, data=json.dumps(body).encode(), method="POST",
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _get(base, pfad):
    with urllib.request.urlopen(base + pfad, timeout=60) as r:
        return json.loads(r.read())


def _lauf(base, fid, *, geburtsdatum=None, geburtsjahr=None):
    """Legt den Fall an, schreibt die Basis plus (optional) Geburtsdatum / Geburtsjahr.

    Liefert (zahl_cent, feld_ids der Fragequeue).
    """
    st, _ = _post(base, "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    assert st == 201
    felder = list(KEGEL_BASIS) + [("agb_zwangslaeufig", True), ("agb_notwendig_angemessen", True)]
    if geburtsdatum is not None:
        felder.append(("stammdaten_geburtsdatum", geburtsdatum))
    if geburtsjahr is not None:
        felder.append(("geburtsjahr", geburtsjahr))
    for f, w in felder:
        st, antwort = _post(base, f"/fall/{fid}/event", _laie(f, w))
        assert st == 201, (f, w, antwort)
    e = _get(base, f"/fall/{fid}/ergebnis")
    assert e["grund"] == "bestaetigt", e
    fragen = [f.get("feld_id") or f.get("feld") for f in _get(base, f"/fall/{fid}/fragen")["fragen"]]
    return e["zahl_cent"], fragen


def _store(*paare):
    """Store im VOLLEN Bindungsbestand (wie test_ableitung_reihenfolge.py): hier feuert die Ableitung."""
    s = ST.leerer_store(2025, fall_id="p24a-voll")
    for feld, wert in paare:
        ST.append_event(s, feld_id=feld, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"klick@{feld}"},
                        ts="2026-08-27T12:00:00+00:00", bindung=BINDUNG, katalog=KATALOG)
    return s


def _rel(store):
    return TR.relevanz(store, BINDUNG)[REGEL]


# ---- AK1: die Zahl und die Fragequeue (API) bewegen sich nicht -------------------

def test_ohne_geburtsdatum_bleibt_geburtsjahr_in_der_queue(base):
    zahl, fragen = _lauf(base, "p24a-ohne")
    assert zahl == 15356200
    assert len(fragen) == 218
    assert "geburtsjahr" in fragen
    assert FELD not in fragen          # nie gefragt: das Feld steht in keiner Scheibe


def test_mit_geburtsdatum_1958_fragt_geburtsjahr_nicht_und_senkt_die_zahl(base):
    zahl, fragen = _lauf(base, "p24a-1958", geburtsdatum="01.01.1958")
    assert zahl == 15325100                         # 311 EUR weniger als ohne (Δ 31.100 ct)
    assert len(fragen) == 216
    assert "geburtsjahr" not in fragen


def test_geburtsjahr_allein_traegt_den_rueckfall_und_bewegt_die_zahl(base):
    """Ohne Geburtsdatum bewegt die Antwort auf `geburtsjahr` die Zahl genauso (die Prämisse der Entscheidung)."""
    zahl, fragen = _lauf(base, "p24a-jahr", geburtsjahr=1958)
    assert zahl == 15325100
    assert "geburtsjahr" not in fragen


def test_das_abgeleitete_feld_laesst_sich_nicht_setzen(base):
    st, _ = _post(base, "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "p24a-post"})
    assert st == 201
    st, antwort = _post(base, "/fall/p24a-post/event", _laie(FELD, True))
    assert st == 400, antwort
    assert "nicht in dieser Scheibe" in json.dumps(antwort, ensure_ascii=False)


# ---- AK1/AK2: voller Bindungsbestand — die Ableitung feuert, das Rückfallfeld wird nicht gefragt ----

def test_ableitung_feuert_obwohl_das_feld_nicht_askable_ist():
    for geburtsdatum, erwartet in (("01.01.1958", True), ("01.01.1975", False)):
        felder, _ = ST.materialisiere(_store(("stammdaten_geburtsdatum", geburtsdatum)))
        assert felder[FELD]["wert"] is erwartet, geburtsdatum
        assert felder[FELD]["herkunft"]["herkunft"] == "berechnet"
        assert felder["geburtsjahr"]["wert"] == int(geburtsdatum[-4:])


def test_jahrgang_unter_64_schliesst_die_regel_aus_und_1958_laesst_sie_gelten():
    jung = _rel(_store(("stammdaten_geburtsdatum", "01.01.1975")))
    assert jung["status"] == "ausgeschlossen"
    alt = _rel(_store(("stammdaten_geburtsdatum", "01.01.1958")))
    assert alt["status"] == "relevant"
    assert alt["gates_offen"] == []


def test_ohne_angabe_ist_nur_geburtsjahr_ein_offenes_gate_und_das_feld_nicht_in_der_queue():
    s = _store()
    rel = _rel(s)
    assert rel["status"] == "unentschieden"
    assert rel["gates_offen"] == ["geburtsjahr"]      # vorher: ["geburtsjahr", FELD] — Gate ohne Antwortweg
    queue = TR.naechste_fragen(s, BINDUNG)
    assert "geburtsjahr" in queue
    assert FELD not in queue


@pytest.mark.parametrize("jahr", [1958, 1990])
def test_antwort_auf_geburtsjahr_allein_entscheidet_die_regel(jahr):
    """Der Rückfall wirkt: vorher blieb die Regel nach der Antwort auf `geburtsjahr` „unentschieden",
    weil das nie beantwortbare Gate offen blieb, und das tote Feld stand weiter in der Queue."""
    s = _store(("geburtsjahr", jahr))
    rel = _rel(s)
    assert rel["status"] == "relevant"
    assert rel["gates_offen"] == []
    assert FELD not in TR.naechste_fragen(s, BINDUNG)
    assert FELD not in ST.materialisiere(s)[0]        # kein Geburtsdatum -> keine Ableitung


# ---- AK2: die Bindung nennt kein unsetzbares Rückfallfeld mehr ------------------

def test_rueckfallfeld_ist_abgeleitet_nicht_gefragt_und_die_erklaerung_nennt_geburtsjahr():
    b = BINDUNG[FELD]
    assert not b.get("askable"), "das Feld lässt sich in keiner Scheibe setzen: nicht askable"
    assert not b.get("eingangsfrage")
    assert b["ableitung"]["aus"] == "stammdaten_geburtsdatum"   # die Ableitung bleibt
    bedingung = [c for c in TR.lade_regel_bedingungen()[REGEL] if c["feld"] == FELD]
    assert len(bedingung) == 1
    # Beide Erklärungen verweisen auf den Rückfall, der wirklich gefragt wird.
    assert "geburtsjahr" in b["ableitung"]["grund"]
    assert "geburtsjahr" in bedingung[0]["grund"]
    assert BINDUNG["geburtsjahr"]["askable"]                   # der Rückfall ist fragbar
