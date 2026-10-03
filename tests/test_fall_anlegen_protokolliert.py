"""`fall_angelegt` steht auch ohne Anmeldung im Protokoll, mit Nutzer `unbekannt`.

Vault `decisions/protokollzeile-nach-der-wirkung-vor-der-antwort` (Punkt 3), Eintrag
`backlog/taxgraph/fall-ohne-protokollzeile-im-testbetrieb`.

Vorher hing die Zeile an `if uid is not None`: wer ohne Anmeldung einen Fall anlegte (jeder Testlauf,
jeder Wartungszugang mit `TAXGRAPH_NO_AUTH=1`), bekam eine Akte ohne Spur. 15 solche Akten vom
2026-10-01 stehen in keiner Zeile von `audit.jsonl`. Der Besitzer der Akte bleibt leer (ein Fall im
Opt-out gehört bewusst niemandem, `api.py` `fall_anlegen`); nur die Spur kommt dazu.
"""
from __future__ import annotations

import json
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _s in ("produkt/haut", "produkt/store", "produkt/auth", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _s))

import api as API          # noqa: E402
import api_auth            # noqa: E402
import audit               # noqa: E402


@pytest.fixture
def ablage(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", None)
    return tmp_path / "faelle"


def _akte(ablage, fall_id: str) -> dict:
    return json.loads((ablage / f"{fall_id}.json").read_text(encoding="utf-8"))


def test_ohne_anmeldung_steht_die_zeile_mit_fall_id_und_nutzer_unbekannt(ablage, monkeypatch):
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    status, antwort = API.fall_anlegen({"fall_id": "n1", "scheibe": "ep", "veranlagungszeitraum": 2025})
    assert status == 201 and antwort["fall_id"] == "n1"

    zeilen = [z for z in audit.lies() if z["action"] == "fall_angelegt"]
    assert [(z["user_id"], z["fall_id"], z["detail"]) for z in zeilen] == [("unbekannt", "n1", "scheibe=ep")]


def test_der_fall_bleibt_ohne_anmeldung_ohne_besitzer(ablage, monkeypatch):
    """Nur die Spur kommt dazu, nicht der Besitzer: `user_id` fehlt in der Akte."""
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    API.fall_anlegen({"fall_id": "n2", "scheibe": "ep", "veranlagungszeitraum": 2025})
    assert "user_id" not in _akte(ablage, "n2")


def test_mit_anmeldung_bleibt_alles_wie_vorher(ablage, monkeypatch):
    monkeypatch.delenv("TAXGRAPH_NO_AUTH", raising=False)
    monkeypatch.setattr(api_auth, "_AUTH_USER", "alice")
    API.fall_anlegen({"fall_id": "n3", "scheibe": "ep", "veranlagungszeitraum": 2025})
    assert _akte(ablage, "n3")["user_id"] == "alice"
    zeilen = [z for z in audit.lies() if z["action"] == "fall_angelegt"]
    assert [(z["user_id"], z["fall_id"]) for z in zeilen] == [("alice", "n3")]


def test_eine_abgewiesene_anfrage_schreibt_keine_angelegt_zeile(ablage, monkeypatch):
    """Die Zeile steht nach der Wirkung: wo keine Akte entsteht (409), gibt es sie nicht."""
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    rumpf = {"fall_id": "n4", "scheibe": "ep", "veranlagungszeitraum": 2025}
    API.fall_anlegen(rumpf)
    with pytest.raises(API.ApiError) as e:
        API.fall_anlegen(rumpf)
    assert e.value.status == 409
    assert sum(z["action"] == "fall_angelegt" for z in audit.lies()) == 1
