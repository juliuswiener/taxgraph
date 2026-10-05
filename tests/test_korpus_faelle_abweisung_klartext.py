"""Gate: `tools/parity/korpus_faelle.lege_an` nennt bei einer Abweisung den Grund im Klartext.

Ticket `korpus-faelle-liest-ein-feld-das-apierror-nicht-hat`: beide `except API.ApiError`-Zweige
lasen `e.detail`. `ApiError` (produkt/haut/api.py) traegt nur `status` und die Meldung (`str(e)`).
Jede Abweisung beim Anlegen oder bei einem Event warf deshalb einen AttributeError im Werkzeug,
statt "ABWEISUNG: <status> <grund>" zu liefern; `korpus_faelle.main` (und früher der E2E-Erzeuger, in Stufe 2
gelöscht) zeigt dem Menschen genau diese Zeile.

Kein Netzwerk, kein Server, kein Schreiben: `API.fall_anlegen` und `API.event` sind ersetzt, die
Abweisung ist ein echtes `API.ApiError`. Zwei Faelle, weil `lege_an` zwei `except`-Zweige hat und
ein Fall nur den Zweig deckt, den er nimmt.
"""
from __future__ import annotations

import importlib.util
import os

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
SKRIPT = os.path.join(ROOT, "tools", "parity", "korpus_faelle.py")

SCHEIBE = "gesamt"
GRUND = "Testgrund: der Wert ist kein Betrag"


def _lade(monkeypatch, tmp_path):
    """Das Skript laden. Es beendet sich beim Import, solange TAXGRAPH_DATEN fehlt.

    Der Wert ist nur der Schluessel durch diese Tuer: `api_constants.FAELLE` ist durch conftest.py
    laengst gebunden, und beide Endpunkte sind im Test ersetzt, es wird nichts geschrieben.
    """
    monkeypatch.setenv("TAXGRAPH_DATEN", str(tmp_path))
    spec = importlib.util.spec_from_file_location("korpus_faelle_unter_test", SKRIPT)
    modul = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(modul)
    return modul


@pytest.mark.parametrize("anlegen_wirft", [True, False], ids=["anlegen", "event"])
def test_abweisung_nennt_status_und_grund_im_klartext(monkeypatch, tmp_path, anlegen_wirft):
    kf = _lade(monkeypatch, tmp_path)
    abweisung = kf.API.ApiError(422, GRUND)

    def anlegen(body):
        if anlegen_wirft:
            raise abweisung
        return 201, {}

    def event(fall_id, body):
        raise abweisung

    monkeypatch.setattr(kf.API, "fall_anlegen", anlegen)
    monkeypatch.setattr(kf.API, "event", event)

    ergebnis = kf.lege_an("abweisung-klartext", SCHEIBE, {})

    # Der Event-Fall scheitert am ersten Feld des Kegels; der Anlegen-Fall braucht den Kegel nicht.
    wo = "ABWEISUNG" if anlegen_wirft else f"ABWEISUNG bei {kf.kegel_fuer(SCHEIBE, {})[0][0]}"
    assert ergebnis == f"{wo}: 422 {GRUND}"
