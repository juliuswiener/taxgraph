"""Die Frage zu EINEM Feld — auch zu einem beantworteten (GET /fall/<id>/feld/<fid>/frage).

DER BEFUND, gemessen am 2026-08-27: `korrigiereBestaetigt` in app.js sucht das zu korrigierende
Feld in /fragen. /fragen ist aber die Queue der UNBEANTWORTETEN Felder — bestätigt heisst
beantwortet heisst draussen. Jede Korrektur eines bestätigten Feldes endete deshalb bei

    „Diese Frage ist durch eine andere Antwort entfallen und lässt sich nicht mehr ändern."

Das stimmt nicht. Die Frage ist nicht entfallen; sie steht nur nicht in der Liste der offenen
Fragen, weil sie beantwortet ist. Der Satz schickt den Nutzer von einer Korrektur weg, die
möglich wäre — und zwar bei JEDEM bestätigten Feld, nicht nur bei Instanzfeldern.

Dass „Ändern" auf der Prüfliste trotzdem funktioniert, hat einen Grund, der die Sache verschleiert
hat: KI-Vorschläge sind VORLÄUFIG und bleiben damit in der Queue. Der Weg war also genau für die
Felder heil, an denen er zuerst gebaut wurde.

/fragen bleibt deshalb unverändert die Antwort auf „was ist noch offen". Diese Frage hier ist eine
andere, und sie bekommt einen eigenen Weg, statt die Bedeutung der Queue aufzuweichen.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/konsistenz", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API        # noqa: E402
import api_auth          # noqa: E402
import audit             # noqa: E402


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", "pruefer")
    fid = "frage-einzeln"
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    return fid


def _bestaetige(fall, feld, wert):
    API.event(fall, {"feld_id": feld, "wert": wert, "zustand": "bestaetigt",
                     "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                     "schreiber": "ui:laie",
                     "signal": {"signal_1": None, "signal_2": f"klick@{feld}"}})


def test_ein_bestaetigtes_feld_steht_nicht_mehr_in_der_queue(fall):
    """Die Vorbedingung, und zugleich der Befund. Ohne diesen Test läse sich der nächste so, als
    hätte /fragen bloss zufällig nicht geliefert."""
    _bestaetige(fall, "stammdaten_vorname", "Jonas")
    st, b = API.fragen(fall)
    assert "stammdaten_vorname" not in {q["feld_id"] for q in b["fragen"]}, (
        "Das bestätigte Feld steht noch in der Queue der OFFENEN Fragen — dann misst der Test "
        "darunter nicht mehr, was er behauptet.")


def test_die_frage_zu_einem_beantworteten_feld_ist_trotzdem_zu_haben(fall):
    """Der eigentliche Punkt: der Korrekturweg braucht die Frage, die Queue hat sie nicht mehr."""
    _bestaetige(fall, "stammdaten_vorname", "Jonas")
    st, b = API.frage_einzeln(fall, "stammdaten_vorname")
    assert st == 200 and b["frage"]["fragetext_laie"], f"Keine Frage geliefert: {b}"
    assert b["frage"]["feld_id"] == "stammdaten_vorname"


def test_die_metadaten_sind_dieselben_wie_in_der_queue(fall):
    """EINE Quelle, nicht zwei. `frage_einzeln` und `fragen` bauen ihre Felder aus derselben
    Funktion — führte jede ihre eigene Liste, fiele die Oberfläche irgendwann auf ein fehlendes
    `muster` oder `enum_labels` herein, ohne dass es jemand merkt. Genau diese Bauart ist in
    diesem Haus schon mehrfach auseinandergelaufen."""
    st, b = API.fragen(fall)
    aus_queue = b["fragen"][0]
    st, e = API.frage_einzeln(fall, aus_queue["feld_id"])
    assert sorted(e["frage"]) == sorted(aus_queue), (
        f"Verschiedene Schlüssel:\n  Queue:   {sorted(aus_queue)}\n  Einzeln: {sorted(e['frage'])}")
    assert e["frage"] == aus_queue, (
        "Gleiche Schlüssel, andere Werte — dann bauen die beiden Wege doch verschieden.")


def test_ein_instanzfeld_wird_auf_sein_basisfeld_aufgeloest(fall):
    """`kind_vorname__2` steht in KEINER Bindung — der Traverser führt nur Basisfelder, die
    Instanz ist reine Mapping-Konvention. Ohne diese Auflösung liefe der Korrekturweg für jede
    zweite Instanz ins Leere, und genau dort hat der Befund angefangen."""
    st, b = API.frage_einzeln(fall, "kind_vorname__2")
    assert st == 200
    assert b["frage"]["feld_id"] == "kind_vorname", (
        f"Nicht aufs Basisfeld aufgelöst: {b['frage']['feld_id']!r}")
    assert "instanz_anzahl" in b["frage"], "Ohne die Zahl weiss die Oberfläche nicht, wie viele "\
                                           "Zeilen sie bauen soll."


def test_ein_feld_das_es_nicht_gibt_ist_ein_404_und_keine_leere_frage(fall):
    """Fail-closed: eine leere Frage zurückzugeben hiesse, die Oberfläche baute ein Eingabefeld
    ohne Typ und schriebe irgendetwas."""
    with pytest.raises(API.ApiError) as e:
        API.frage_einzeln(fall, "gibtesnicht")
    assert e.value.status == 404


def test_die_queue_bleibt_was_sie_ist(fall):
    """Gegenprobe zur Entscheidung: /fragen wurde NICHT um beantwortete Felder erweitert. Die
    Queue beantwortet „was ist noch offen" — hätte sie beides beantwortet, wäre der Fortschritt
    nicht mehr ablesbar und jede Zählung darauf falsch."""
    st, vorher = API.fragen(fall)
    _bestaetige(fall, "stammdaten_vorname", "Jonas")
    st, nachher = API.fragen(fall)
    assert len(nachher["fragen"]) < len(vorher["fragen"]), (
        "Die Zahl der offenen Fragen sinkt nicht mehr, wenn eine beantwortet wird.")
