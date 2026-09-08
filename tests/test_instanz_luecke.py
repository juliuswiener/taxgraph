"""Befund 2026-09-07 (Diagnose), repariert 2026-09-07: `naechste_fragen()` kannte nur die
BASIS-`feld_id` einer Instanz-Gruppe (`instanz_gruppen` in bindung_regel_bedingungen.yaml).
Sobald Instanz 1 beantwortet war, fiel das Basisfeld für immer aus der Fragen-Warteschlange —
unabhängig davon, wie viele Instanzen das Zählfeld (`anzahl_feld`) noch verlangte. Ein Nutzer,
der 2 Kinder angibt und nur eins ausfüllt, bekam nie wieder eine Frage nach Kind 2; `/fragen`
zeigte es einfach nicht mehr an. Preflight (`fehlende_instanzen()`) SAH die Lücke und benannte
sie im Text, aber der Dialog selbst führte nirgendwo zurück.

Kein Kinder-Problem: dieselbe Struktur trifft ALLE acht `instanz_gruppen` (vv_objekt, rente,
p23_veraeusserung, hh_handwerker, hh_dienstleistung, hh_minijob, gwg, kind) — dieser Test prüft
zwei davon (`kind`, `vv_objekt`), damit die Wache nicht nur für Kinder gilt.

Gemessen über vollen HTTP-Weg (register→fall→event→fragen ist hier nicht nötig — TAXGRAPH_NO_AUTH
kommt aus tests/conftest.py, das ihn für die gesamte Suite setzt), eigene `base`-Fixture wie
tests/test_paket_b_e2e_http.py (Vorbild tests/test_rentenfreibetrag_vorlaeufig_bricht_fragen.py:
eigene Helfer statt Import aus der fremden Datei — die steht unter fremdem WIP).

Grüne Kriterium ist NICHT ein `__2`-suffixierter Feld-Name in `/fragen` — der Traverser bietet
per Design nur je EINE Frage pro Basis-`feld_id` an (die Oberfläche rendert daraus N Eingabefelder
auf einer Karte, s. app.js `baueInstanzEingaben`/`schreibeInstanzen`). Grün heißt: die Basis-
`feld_id` erscheint wieder in `/fragen`, mit `instanz_anzahl` == der bestätigten Zahl.

Regressions-Wache (zweite Hälfte): eine Karte, die in EINEM Zug vollständig ausgefüllt wird (alle
Instanzen auf einmal, wie `schreibeInstanzen()` es tut), darf NICHT danach nochmal auftauchen —
sonst schiebt ein fertig ausgefülltes Formular plötzlich Fragen nach, und die Reparatur wäre
schlimmer als der Defekt, den sie behebt.

NULL LLM."""
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
for sub in ("produkt/haut", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit              # noqa: E402


def _req(base: str, method: str, path: str, body: dict | None = None, erwarte: int | None = None):
    """Wie test_paket_b_e2e_http.py: 5xx -> AssertionError (nie unterdrückbar), es sei denn
    `erwarte` sagt es ausdrücklich voraus."""
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, f"erwarte={erwarte}, erhalten={status} {method} {path} {body}"
    elif status >= 500:
        raise AssertionError(f"Serverfehler {status} {method} {path} {body}: {content}")
    elif status >= 400:
        raise AssertionError(f"Fehler {status} {method} {path} {body}: {content}")
    return status, content


def _laie(feld_id, wert):
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"}}


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


def _anlegen(base, fall_id):
    _req(base, "POST", "/fall",
         {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fall_id}, erwarte=201)


def _fragen_ids(base, fall_id):
    _, b = _req(base, "GET", f"/fall/{fall_id}/fragen", erwarte=200)
    return {f["feld_id"]: f for f in b["fragen"]}


def test_kind_instanz2_bleibt_erfragbar(base):
    """2 Kinder angegeben, nur Kind 1 ausgefüllt -> `kind_vorname` muss weiter/wieder in
    `/fragen` stehen, mit instanz_anzahl=2 (Kind 2 fehlt noch)."""
    fall_id = "kind1"
    _anlegen(base, fall_id)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_kind", False), erwarte=201)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("fam_anzahl_kinder", 2), erwarte=201)

    fragen = _fragen_ids(base, fall_id)
    assert "kind_vorname" in fragen, "vor jeder Antwort muss die Frage stehen"
    assert fragen["kind_vorname"]["instanz_anzahl"] == 2

    _req(base, "POST", f"/fall/{fall_id}/event", _laie("kind_vorname", "Anna"), erwarte=201)

    fragen = _fragen_ids(base, fall_id)
    assert "kind_vorname" in fragen, (
        "Kind 2 fehlt noch -- kind_vorname muss wieder erfragbar sein, nicht dauerhaft "
        "aus der Warteschlange fallen (Befund 2026-09-07)")
    assert fragen["kind_vorname"]["instanz_anzahl"] == 2


def test_vv_objekt_instanz2_bleibt_erfragbar(base):
    """2 Vermietungsobjekte angegeben, nur Objekt 1 ausgefüllt -> `vv_einnahmen` muss
    weiter/wieder in `/fragen` stehen, mit instanz_anzahl=2 (Objekt 2 fehlt noch)."""
    fall_id = "vv1"
    _anlegen(base, fall_id)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_vuv", False), erwarte=201)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("vv_anzahl_objekte", 2), erwarte=201)

    fragen = _fragen_ids(base, fall_id)
    assert "vv_einnahmen" in fragen, "vor jeder Antwort muss die Frage stehen"
    assert fragen["vv_einnahmen"]["instanz_anzahl"] == 2

    _req(base, "POST", f"/fall/{fall_id}/event", _laie("vv_einnahmen", 100000), erwarte=201)

    fragen = _fragen_ids(base, fall_id)
    assert "vv_einnahmen" in fragen, (
        "Objekt 2 fehlt noch -- vv_einnahmen muss wieder erfragbar sein, nicht dauerhaft "
        "aus der Warteschlange fallen (Befund 2026-09-07)")
    assert fragen["vv_einnahmen"]["instanz_anzahl"] == 2


def test_vollstaendig_ausgefuellte_karte_fragt_nicht_nach(base):
    """Regressions-Wache: werden BEIDE Instanzen in einem Zug geschrieben (wie
    `schreibeInstanzen()` in app.js es beim Absenden der Karte tut -- Basis-`feld_id` +
    `__2`), darf die Frage danach NICHT wieder auftauchen. Sonst schiebt ein fertig
    ausgefülltes Formular Fragen nach, und die Reparatur der Instanz-Lücke wäre selbst ein
    neuer Defekt (Instructor: "Kaputtmachen, was heute funktioniert, ist schlimmer als der
    Defekt")."""
    fall_id = "vv2"
    _anlegen(base, fall_id)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("kein_vuv", False), erwarte=201)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("vv_anzahl_objekte", 2), erwarte=201)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("vv_einnahmen", 100000), erwarte=201)
    _req(base, "POST", f"/fall/{fall_id}/event", _laie("vv_einnahmen__2", 50000), erwarte=201)

    fragen = _fragen_ids(base, fall_id)
    assert "vv_einnahmen" not in fragen, "beide Instanzen sind da -- keine Nachfrage"
