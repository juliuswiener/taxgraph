"""Der Fluss-Mitschnitt (produkt/haut/flow.py) — Julius, 2026-08-27: „ich will so ein log wo der
ganze flow nachvollziehbar ist."

Anlass war ein Live-Durchgang mit neun Befunden, von denen KEINER aus einem Protokoll kam: die
Reihenfolge der Fragen, die Doppelungen und das blockierte Ende musste ich aus den Ereignissen im
Fall rekonstruieren. Das trägt nur, solange man die Reihenfolge schon kennt — und genau die war
die Frage.

Geprüft wird deshalb nicht „es entsteht eine Datei", sondern dass der Fluss danach WIEDERHOLBAR
ist: welche Frage lag vor, was kam zurück, über welchen Bildschirm, und was sagte das Ergebnis.

  1. Ohne Schalter passiert NICHTS. Hier steht Klartext (Namen, Beträge, Kontonummern); ein
     Mitschnitt, der von allein mitläuft, wäre ein zweiter Datenspeicher ohne Zweckbindung.
  2. Der Weg steht drin. `klick@` / `hold@` / `rueckfrage@` / `verstanden@` unterscheidet den
     Fragebogen vom Assistenten — an genau dieser Spalte liess sich zeigen, dass der Assistent
     die Instanz-Achse nicht kennt.
  3. Eine ABGEWIESENE Antwort steht auch drin. Für den Nutzer ist sie ein Ereignis („da stand ein
     Banner"), im Fall hinterlässt sie nichts — sie fehlte also genau dort, wo man sie sucht.
  4. Das Ergebnis steht drin, samt der Zahl der benannten offenen Felder. Ein Grund ohne ein
     einziges benanntes Feld ist der Zustand, in dem der Nutzer „noch offen" liest und nicht
     weiterkommt; das muss ein Protokoll zeigen können.
  5. Ein Schreibfehler im Mitschnitt reisst den Vorgang NICHT mit. Ein Protokoll darf nicht
     kaputtmachen, was es beschreibt.
"""
from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import api_auth          # noqa: E402
import audit             # noqa: E402
import flow              # noqa: E402


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", "pruefer")
    monkeypatch.setenv("TAXGRAPH_FLOW", "1")
    fid = "flow-test"
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
    return fid, tmp_path / "faelle" / "flow.jsonl"


def _zeilen(pfad):
    if not os.path.exists(pfad):
        return []
    return [json.loads(z) for z in open(pfad, encoding="utf-8") if z.strip()]


def _schreibe(fid, feld, wert, weg):
    API.event(fid, {"feld_id": feld, "wert": wert, "zustand": "bestaetigt",
                    "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                 "haftung": "nutzer"},
                    "schreiber": "ui:laie",
                    "signal": {"signal_1": None, "signal_2": f"{weg}@{feld}"}})


def test_ohne_schalter_entsteht_kein_mitschnitt(tmp_path, monkeypatch):
    """Punkt 1. Der Schalter wird zur AUFRUFZEIT gelesen — deshalb reicht es, ihn hier
    wegzunehmen, ohne zu wissen, wann das Modul geladen wurde."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", "pruefer")
    monkeypatch.delenv("TAXGRAPH_FLOW", raising=False)
    monkeypatch.delenv("TAXGRAPH_KI_DEBUG", raising=False)
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "ohne"})
    API.fragen("ohne")
    _schreibe("ohne", "kein_kap", True, "klick")
    assert not os.path.exists(tmp_path / "faelle" / "flow.jsonl"), (
        "Der Mitschnitt lief ohne Schalter — hier steht Klartext, das darf nicht von allein "
        "passieren.")


def test_der_fluss_ist_aus_dem_mitschnitt_wiederherstellbar(fall):
    """Punkte 2 und 4, der eigentliche Zweck: Frage → Antwort → Ergebnis, in dieser Reihenfolge,
    mit dem Bildschirm daneben, über den die Antwort kam."""
    fid, pfad = fall
    API.fragen(fid)
    _schreibe(fid, "kein_kap", True, "klick")
    _schreibe(fid, "veranlagung", "einzel", "verstanden")
    API.fragen(fid)
    API.ergebnis(fid)

    z = _zeilen(pfad)
    arten = [e["art"] for e in z]
    assert arten.count("fragen") >= 2 and "antwort" in arten and "ergebnis" in arten, (
        f"Der Strang trägt nicht alle drei Sorten: {arten}")
    assert all(e["fall"] == fid for e in z), f"Einträge ohne Fallbezug: {z}"

    antworten = [e["inhalt"] for e in z if e["art"] == "antwort"]
    assert [a["feld_id"] for a in antworten] == ["kein_kap", "veranlagung"], (
        f"Die Reihenfolge der Antworten stimmt nicht: {antworten}")
    assert [a["weg"] for a in antworten] == ["klick@kein_kap", "verstanden@veranlagung"], (
        "Der WEG fehlt oder stimmt nicht — ohne ihn ist eine Antwort aus dem Fragebogen von "
        f"einer aus dem Assistenten nicht zu unterscheiden: {antworten}")

    kopf = [e["inhalt"] for e in z if e["art"] == "fragen"]
    assert kopf[0]["offen"] > kopf[-1]["offen"], (
        f"Die Zahl der offenen Fragen sinkt nicht — dann zeigt der Mitschnitt keinen Fortschritt: "
        f"{kopf[0]['offen']} -> {kopf[-1]['offen']}")
    assert kopf[0]["kopf"] and all("frage" in q and "feld_id" in q for q in kopf[0]["kopf"]), (
        f"Im Kopf der Queue fehlt der Fragetext: {kopf[0]}")


def test_eine_abgewiesene_antwort_steht_im_mitschnitt(fall):
    """Punkt 3. Der Store weist fail-closed ab (hier: ein Text in einem bool-Feld). Im Fall bleibt
    davon nichts zurück — für den Nutzer war es trotzdem ein Ereignis."""
    fid, pfad = fall
    with pytest.raises(API.ApiError) as e:
        _schreibe(fid, "kein_kap", "vielleicht", "klick")
    assert e.value.status == 422

    abgewiesen = [x["inhalt"] for x in _zeilen(pfad) if x["art"] == "abgewiesen"]
    assert len(abgewiesen) == 1, f"Die Abweisung fehlt im Mitschnitt: {_zeilen(pfad)}"
    assert abgewiesen[0]["feld_id"] == "kein_kap" and abgewiesen[0]["grund"], (
        f"Abweisung ohne Feld oder ohne Grund: {abgewiesen[0]}")


def test_das_ergebnis_meldet_wie_viele_offene_felder_es_benennt(fall):
    """Punkt 4 scharf gestellt. Genau hier lag Julius' neunter Befund: `/ergebnis` sagte „noch
    offen" und lieferte die Begründungsliste LEER — der Nutzer las den Zustand und erfuhr nicht,
    woran es liegt. Der Mitschnitt muss diesen Fall benennen können, sonst taugt er nicht als
    Diagnose."""
    fid, pfad = fall
    st, obj = API.ergebnis(fid)
    e = [x["inhalt"] for x in _zeilen(pfad) if x["art"] == "ergebnis"]
    assert len(e) == 1, f"Kein Ergebnis-Eintrag: {_zeilen(pfad)}"
    assert e[0]["grund"] == obj["grund"], "Der Mitschnitt nennt einen anderen Grund als die API."
    assert e[0]["offen_anzahl"] == len(obj.get("offen") or []), (
        "Die Zahl der benannten offenen Felder stimmt nicht mit der Antwort überein — genau diese "
        "Zahl unterscheidet „es fehlt X“ von „es fehlt etwas“.")


def test_die_oberflaeche_meldet_nur_was_sonst_niemand_sieht(fall):
    """Der Meldeweg für die Bildschirme (POST /fall/<id>/flow).

    Die Liste der erlaubten Sorten ist ABSICHTLICH kurz. Nachgemessen 2026-08-27: Ankreuzliste und
    beantwortete Nachfragen sind aus dem Fall vollständig rekonstruierbar — jede Antwort trägt in
    `signal_2` ihren Bildschirm, die Bindung liefert Fragetext und `frage_invertiert`. Gemeldet
    wird nur, was KEINE Antwort hinterlässt: eine übersprungene Nachfrage, eine vorher entfallene,
    ein „Ändern", eine unbestätigt verlassene Liste, die Wegwahl.

    Ein Client, der sich die Sorte selbst ausdenken dürfte, könnte die Datei beliebig füllen —
    deshalb eine feste Liste und eine Abweisung, kein stilles Verwerfen."""
    fid, pfad = fall
    st, _ = API.flow_melden(fid, {"art": "nachfrage_spaeter",
                                  "inhalt": {"feld_id": "bruttoarbeitslohn"}})
    assert st == 200
    gemeldet = [e for e in _zeilen(pfad) if e["art"] == "nachfrage_spaeter"]
    assert len(gemeldet) == 1 and gemeldet[0]["fall"] == fid, (
        f"Die Meldung steht nicht im Strang: {_zeilen(pfad)}")

    with pytest.raises(API.ApiError) as e:
        API.flow_melden(fid, {"art": "checkliste_beantwortet", "inhalt": {}})
    assert e.value.status == 400, (
        "Eine nicht vorgesehene Sorte wurde angenommen — dann kann ein Client den Mitschnitt "
        "mit Beliebigem füllen.")


def test_eine_zu_grosse_meldung_wird_gekappt_und_sagt_es(fall):
    """Ein Client-Beitrag ist Fremdtext und darf die Datei nicht sprengen. Gekappt wird er
    deshalb — aber NICHT stillschweigend: stillschweigend gekürzt sähe im Mitschnitt aus wie
    „mehr war da nicht", und das ist die eine Aussage, die ein Protokoll nie machen darf."""
    fid, pfad = fall
    API.flow_melden(fid, {"art": "nachfragen_gestartet",
                          "inhalt": {"gestellt": ["x" * 200 for _ in range(80)]}})
    i = [e["inhalt"] for e in _zeilen(pfad) if e["art"] == "nachfragen_gestartet"][0]
    assert i.get("gekappt_bei") == flow.MAX_ZEICHEN, f"Nicht gekappt: {str(i)[:200]}"
    assert i.get("urspruengliche_zeichen") > flow.MAX_ZEICHEN, (
        f"Die ursprüngliche Grösse fehlt — dann ist nicht zu sehen, wie viel weg ist: {i}")


def test_health_sagt_der_oberflaeche_ob_mitgeschrieben_wird(fall, monkeypatch):
    """Ohne diese Auskunft müsste die Seite ihre Bildschirme IMMER melden, auch wenn niemand
    mitschreibt — auf einem einfädigen Server sind das Anfragen, die echte blockieren."""
    assert API.health()[1]["flow"] is True
    monkeypatch.delenv("TAXGRAPH_FLOW", raising=False)
    monkeypatch.delenv("TAXGRAPH_KI_DEBUG", raising=False)
    assert API.health()[1]["flow"] is False


def test_ein_kaputter_mitschnitt_reisst_den_vorgang_nicht_mit(fall, monkeypatch):
    """Punkt 5. Ein Protokoll darf nicht kaputtmachen, was es beschreibt."""
    fid, _ = fall
    monkeypatch.setattr(audit, "AUDIT_DIR", "/proc/gibtesnicht/und/geht/nicht")
    st, b = API.fragen(fid)
    assert st == 200 and b["fragen"], "Der Endpunkt ist am Mitschnitt gescheitert."
