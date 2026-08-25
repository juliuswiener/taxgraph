"""Eine Frage nach einem MERKMAL darf erst kommen, wenn die EXISTENZ bejaht ist.

ANLASS, Julius im echten Durchgang am 2026-08-25 — vier Meldungen kurz hintereinander, jedes Mal
derselbe Satz („frage hat keine daseinsberechtigung"):

    „War der Übernachtungsort ein auswärtiger Arbeitsort …?"   — nie auswärts übernachtet
    „Warst du beim Verkauf des Betriebs mindestens 55 …?"      — nie einen Betrieb verkauft
    „War dein Kind während der Betreuung unter 14 …?"          — Geburtsdatum längst angegeben
    „Hat dein Ex-Ehepartner … zugestimmt?"                     — nie Unterhalt an einen Ex gezahlt

NACHGEMESSEN, und das ist der eigentliche Befund: die Existenzfrage stand jeweils FRÜH in der
Queue (`keine_behinderung_pflege` auf Position 8 von 321, `kein_gewinn` auf 26). Sie zu
beantworten nahm die Detailfrage trotzdem nicht weg — zwischen den beiden Regeln bestand keine
Verbindung. `naechste_fragen` nimmt jedes unbeantwortete askable Feld jeder nicht ausgeschlossenen
Regel auf; eine Regel wird nur durch ihre EIGENEN Gates oder durch eine `regel_bedingung`
ausgeschlossen.

Der Mechanismus dafür existiert seit dem 2026-08-14 (Screening-Gruppe B) und war für 25 Regeln
gepflegt. Diese Datei misst die fünf, die am 2026-08-25 dazukamen — und, wichtiger, die REGEL
dahinter: eine bejahte Abwesenheit muss die Folgefragen wegnehmen.

KEIN BROWSER, KEIN LLM: gemessen wird die Fragen-Queue selbst.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/traverser", "produkt/store", "produkt/mapping"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _queue(bindung, gesetzt=None):
    store = ST.leerer_store(veranlagungszeitraum=2025, fall_id="existenzprobe")
    for fid, wert in (gesetzt or {}).items():
        ST.append_event(store, feld_id=fid, wert=wert, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                  "haftung": "nutzer"},
                        schreiber="test", signal={"signal_1": None, "signal_2": "t"},
                        bindung=bindung)
    return TR.naechste_fragen(store, bindung)


# (Existenzfrage, Antwort, Detailfrage, Julius' Wortlaut)
FAELLE = [
    ("kein_gewinn", True, "rentner_alter_55_oder_berufsunfaehig",
     "Warst du beim Verkauf des Betriebs mindestens 55 Jahre alt?"),
    ("kein_gewinn", True, "antrag_ermaessigter_satz",
     "Möchtest du für deinen Veräusserungsgewinn den ermässigten Satz?"),
    ("kein_kind", True, "kind_unter_14_haushaltszugehoerig",
     "War dein Kind während der Betreuung unter 14 Jahre alt?"),
    ("kein_vuv", True, "vv_wohnzwecke",
     "Wird die Wohnung zum Wohnen vermietet?"),
    ("vpf_auswaertige_taetigkeit", False, "uebernachtung_auswaerts",
     "War der Übernachtungsort ein auswärtiger Arbeitsort?"),
]


@pytest.mark.parametrize("gate,antwort,detail,wortlaut", FAELLE,
                         ids=[f[2] for f in FAELLE])
def test_die_detailfrage_faellt_weg(bindung, gate, antwort, detail, wortlaut):
    """Vorher steht die Detailfrage in der Queue, nachher nicht mehr."""
    vorher = _queue(bindung)
    assert detail in vorher, (
        f"Vorbedingung: {detail} muss ohne Antwort in der Queue stehen — sonst misst der Test "
        f"nichts.")
    nachher = _queue(bindung, {gate: antwort})
    assert detail not in nachher, (
        f"„{wortlaut}“ wird immer noch gestellt, obwohl {gate}={antwort} bestätigt ist.")


@pytest.mark.parametrize("gate,antwort,detail,wortlaut", FAELLE,
                         ids=[f[2] for f in FAELLE])
def test_ohne_antwort_bleibt_die_frage_stehen(bindung, gate, antwort, detail, wortlaut):
    """DIE GEGENRICHTUNG, und sie ist hier die wichtigere: eine Bedingung, die zu viel wegnimmt,
    verschluckt Fragen, die der Nutzer beantworten müsste — und ein nicht gestellter Abzug ist ein
    stiller Geldverlust.

    Deshalb: die entgegengesetzte Antwort darf die Frage NICHT wegnehmen."""
    nachher = _queue(bindung, {gate: not antwort})
    assert detail in nachher, (
        f"„{wortlaut}“ verschwindet auch bei {gate}={not antwort} — die Bedingung nimmt zu viel "
        f"weg, und der Nutzer verliert einen Abzug, ohne es zu merken.")


def test_unbeantwortet_schliesst_nie_aus(bindung):
    """Fail-closed, wie überall in diesem Haus: solange die Existenzfrage offen ist, stehen die
    Detailfragen in der Queue. Sonst hinge ein Abzug daran, dass jemand eine Frage übersieht."""
    q = _queue(bindung)
    for _, _, detail, wortlaut in FAELLE:
        assert detail in q, f"„{wortlaut}“ fehlt schon ohne jede Antwort: {detail}"


def test_jede_bedingung_nennt_ihre_norm(bindung):
    """Eine Regelbedingung nimmt dem Nutzer Fragen weg — das ist eine steuerliche Aussage und
    braucht denselben Beleg wie jede andere. Die 25 Bestands-Einträge halten das durchgehend ein;
    diese Prüfung hält es fest, damit der nächste Eintrag es nicht unterläuft."""
    rb = TR.lade_regel_bedingungen()
    assert len(rb) >= 30, f"Nur {len(rb)} Regelbedingungen — Sammler prüfen."
    ohne = []
    for regel, bedingungen in rb.items():
        for c in bedingungen:
            grund = c.get("grund") or ""
            if "§" not in grund or len(grund) < 80:
                ohne.append(f"{regel}: {grund[:60]!r}")
    assert not ohne, ("Regelbedingungen ohne Normbeleg:\n  " + "\n  ".join(ohne))


def test_die_bedingung_zeigt_auf_ein_feld_das_es_gibt(bindung):
    """Ein Tippfehler im `feld` einer Bedingung wäre lautlos: relevanz() findet kein Event, die
    Bedingung greift nie, und die Regel bleibt für immer relevant. Genau die Bauart von stillem
    Fehlschlag, gegen die dieses Haus sonst überall fail-closed steht."""
    rb = TR.lade_regel_bedingungen()
    fehlend = [f"{r}: {c['feld']}" for r, cs in rb.items() for c in cs
               if c["feld"] not in bindung]
    assert not fehlend, ("Regelbedingungen zeigen auf Felder, die es nicht gibt:\n  "
                         + "\n  ".join(fehlend))
