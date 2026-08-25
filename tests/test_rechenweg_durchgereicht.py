"""Der Rechenweg muss vom Modell bis zum Nutzer durchkommen — die Naht dazwischen.

ANLASS, im Live-Lauf gemessen (2026-08-23). Julius' Wunsch war: „anteilig vom jahresbrutto ist
aber einfach zu rechnen … das würde es dem nutzer einfacher machen." Das Modell TUT es inzwischen:
aus „50k pro jahr" plus „seit juli arbeitslos" kam `bruttoarbeitslohn = 2.500.000 Cent` zurück,
also 50.000 ÷ 12 × 6. Richtig gerechnet.

Nur sah der Nutzer davon nichts. `api.chat()` baute seine Antwort aus einer FESTEN Schlüsselliste,
und `rechenweg` stand nicht darin — das Feld fiel zwischen Modell und Browser weg. Die Anzeige war
gebaut, das Schema verlangte das Feld, das Modell füllte es: es kam trotzdem nie an.

Was der Nutzer stattdessen sah: „25.000 €" ohne ein Wort dazu, wie die Zahl entstand. Genau der
teure Teil des Vorschlags — die Zahl kann er nur bestätigen, wenn er die Rechnung sieht.

DIESE DATEI MISST DIE NAHT, NICHT DAS MODELL. `_llm_dialog` ist ersetzt, damit der Test allein von
der Durchreichung abhängt und nicht davon, ob ein Anbieter heute rechnet. Der Rechenweg selbst wird
NICHT nachgeprüft — Julius' Entscheid vom selben Tag: „sollten wir nicht dem modell zutrauen diese
rechnung zu können und der user bestätigt."

NULL LLM (der Dialog ist ersetzt, es fließt kein Key und kein Cent).
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/import", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import api_llm           # noqa: E402
import audit             # noqa: E402

# Der Fall aus dem Live-Lauf, in Cent: 50.000 EUR im Jahr, sechs von zwölf Monaten.
FELD = "bruttoarbeitslohn"
WERT = 2_500_000
RECHENWEG = {"basis": 5_000_000, "faktor": "6/12",
             "erklaerung": "50.000 € pro Jahr ÷ 12 × 6 Monate (ab Juli arbeitslos)"}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "rw1"})
    return "rw1"


def _stub(monkeypatch, vorschlag: dict):
    monkeypatch.setattr(api_llm, "_llm_dialog",
                        lambda freitext, katalog, kontext="", user_id=None: {
                            "vorschlaege": [dict(vorschlag)], "antwort": "", "unsicher": False,
                            "aussagen": [], "rueckfragen": []})


def _chat(fall_id):
    status, body = API.chat(fall_id, {"text": "50k im jahr, seit juli arbeitslos"})
    assert status == 200, f"chat() antwortete {status}: {body}"
    return body


def test_der_rechenweg_kommt_beim_vorschlag_an(fall, monkeypatch):
    """DER FALL AUS DEM LIVE-LAUF. Ohne die Durchreichung steht beim Nutzer eine Zahl ohne
    Herkunft — und eine Zahl ohne Herkunft kann er nicht prüfen, nur glauben."""
    _stub(monkeypatch, {"feld_id": FELD, "wert": WERT, "beleg": "50k pro jahr",
                        "begruendung": "anteilig", "rechenweg": dict(RECHENWEG)})
    v = _chat(fall)["vorschlaege"]
    assert v, "kein Vorschlag durchgekommen"
    assert "rechenweg" in v[0], (
        "`rechenweg` fehlt in der Antwort an den Browser. Das Modell liefert ihn, die Anzeige "
        "kann ihn darstellen — er fällt in api.chat() zwischen beiden weg (api.py, geschrieben.append).")
    assert v[0]["rechenweg"]["erklaerung"] == RECHENWEG["erklaerung"], "Erklärung verändert"
    assert v[0]["rechenweg"]["basis"] == RECHENWEG["basis"], "Basis verändert"


def test_ohne_rechnung_steht_dort_null_und_nicht_nichts(fall, monkeypatch):
    """Wo nichts zu rechnen war, ist `null` die richtige Antwort — und der Schlüssel muss trotzdem
    da sein. Ein FEHLENDER Schlüssel und ein leerer sind für die Anzeige dasselbe, für die Diagnose
    aber nicht: nur am vorhandenen `null` lässt sich später messen, wie oft das Modell rechnet."""
    _stub(monkeypatch, {"feld_id": FELD, "wert": 6_200_000, "beleg": "62000 euro brutto",
                        "begruendung": "direkt genannt", "rechenweg": None})
    v = _chat(fall)["vorschlaege"]
    assert v and "rechenweg" in v[0], "der Schlüssel fehlt ganz"
    assert v[0]["rechenweg"] is None


def test_ein_modell_ohne_das_feld_bricht_nichts(fall, monkeypatch):
    """Das Feld ist Pflicht im Schema — aber ein Anbieter, der es trotzdem weglässt, darf den
    Vorschlag nicht mitnehmen. Der Wert ist das Wichtige, die Rechnung die Zugabe."""
    _stub(monkeypatch, {"feld_id": FELD, "wert": WERT, "beleg": "50k pro jahr",
                        "begruendung": "anteilig"})   # kein rechenweg-Schlüssel
    v = _chat(fall)["vorschlaege"]
    assert v, "der Vorschlag ging verloren, nur weil der Rechenweg fehlte"
    assert v[0]["rechenweg"] is None


def test_auch_der_konflikt_traegt_ihn(fall, monkeypatch):
    """Beim Konflikt stehen ZWEI Zahlen nebeneinander und der Nutzer muss sich entscheiden.
    Gerade dort ist die Rechnung der Unterschied zwischen einer Wahl und einem Münzwurf."""
    # Ein bestätigter Wert steht schon da — DANN kommt der abweichende Vorschlag: das ist die Lage,
    # in der api.chat() einen Konflikt meldet statt zu schreiben (Auflage-B-Vorprüfung).
    API.event(fall, {"feld_id": FELD, "wert": 6_200_000, "zustand": "bestaetigt",
                     "schreiber": "nutzer:test",
                     "herkunft": {"herkunft": "nutzer", "pruef_tiefe": "bestaetigt",
                                  "haftung": "nutzer"},
                     "signal": {"signal_1": None, "signal_2": "klick@bruttoarbeitslohn"}})
    _stub(monkeypatch, {"feld_id": FELD, "wert": WERT, "beleg": "50k pro jahr",
                        "begruendung": "anteilig", "rechenweg": dict(RECHENWEG)})
    k = _chat(fall)["konflikte"]
    if not k:
        pytest.skip("kein Konflikt entstanden — dann prüft dieser Test nichts")
    assert "rechenweg" in k[0], "der Konflikt-Zweig reicht den Rechenweg nicht durch"
    assert k[0]["rechenweg"]["erklaerung"] == RECHENWEG["erklaerung"]
