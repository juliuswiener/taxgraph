"""Jede Bindungsdatei muss sich lesen lassen — als BENANNTER roter Test, nicht als Sammelabbruch.

ANLASS, 2026-08-28, und der Verursacher war ich: in `bindung_an_gesamt.yaml` stand ein `grund:`
innerhalb einer Flow-Klammer `{...}`, dessen Text mit dem deutschen OEFFNENDEN Anfuehrungszeichen
begann (U+201E „) und mit einem ASCII-`"` endete. Zwei verschiedene Zeichen, kein Paar: das
ASCII-Zeichen beendete den Skalar, die Klammer blieb offen, die Datei war unlesbar.

WAS DANN PASSIERTE, ist der Grund fuer diese Datei. Fast jedes Testmodul im Haus ruft
`lade_bindung()` schon beim IMPORT (`BINDUNG = TR.lade_bindung()` auf Modulebene). Eine einzige
kaputte YAML nimmt damit die ganze Sammlung mit — die Rueckmeldung lautet dann „pytest sammelt
nicht", ueber Dutzende Module verteilt, und niemand sieht, WELCHE Datei es ist. Zwei Agenten
standen still und suchten.

DIESE DATEI LIEST DIE YAMLS DIREKT, NIE UEBER `lade_bindung()`. Das ist keine Stilfrage, sondern
die ganze Wirkung: ginge sie ueber den Loader, haenge sie an derselben Kette und fiele mit ihr.
Deshalb hier KEIN `import traverser` und kein Import irgendeines Produktmoduls — nur `glob`,
`os` und `yaml`. Die Dateiliste entsteht ueber `glob` zur SAMMELZEIT (ohne Parsen), sodass jede
Datei ihren eigenen benannten Testfall bekommt.

Die Alternative, die der Team-Lead vorgeschlagen hatte — „nach jeder YAML-Aenderung einmal
`python -c 'import yaml…'` laufen lassen" — ist richtig, aber sie haengt daran, dass jeder sie
tippt. Der Beweis, dass das nicht traegt, bin ich von heute Vormittag. Ein Ein-Zeiler, den man
tippen MUSS, ist dieselbe Bauart wie ein Kommentar, der etwas zusagt: er bindet niemanden.

KEIN LLM.
"""
from __future__ import annotations

import glob
import os

import pytest
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
BINDUNG_DIR = os.path.join(ROOT, "produkt", "bindung")

# Zur SAMMELZEIT nur die Dateinamen einsammeln — kein Parsen. Waere hier schon `yaml.safe_load`,
# risse eine kaputte Datei die Sammlung dieses Moduls mit, und der Test waere genau das, wogegen
# er gebaut ist.
YAML_DATEIEN = sorted(glob.glob(os.path.join(BINDUNG_DIR, "*.yaml")))


def _kurz(pfad: str) -> str:
    return os.path.relpath(pfad, ROOT)


def test_es_gibt_ueberhaupt_bindungsdateien():
    """Positivkontrolle. Ohne sie waere die Datei unten vakuum-gruen: fasst `glob` ins Leere,
    laeuft NULL parametrisierte Faelle, und die Suite meldet gruen, ohne eine Zeile YAML gelesen
    zu haben. Dieselbe Klasse wie ein Waechter, der gruen ist, WEIL er nichts findet."""
    assert len(YAML_DATEIEN) >= 20, (
        f"Nur {len(YAML_DATEIEN)} YAML in {_kurz(BINDUNG_DIR)} gefunden — erwartet werden gut "
        f"zwei Dutzend. Entweder stimmt der Pfad nicht, dann prueft dieses Modul nichts.")


@pytest.mark.parametrize("pfad", YAML_DATEIEN, ids=lambda p: os.path.basename(p))
def test_bindungsdatei_laedt(pfad):
    """Der eigentliche Waechter: ein benannter roter Test je Datei, mit Zeile und Spalte.

    `yaml.safe_load` statt `full_load`: hier wird fremder Text gelesen, und die Bindung braucht
    keine Python-Objekte.
    """
    try:
        with open(pfad, encoding="utf-8") as fh:
            inhalt = yaml.safe_load(fh)
    except yaml.YAMLError as e:
        mark = getattr(e, "problem_mark", None)
        ort = f" — Zeile {mark.line + 1}, Spalte {mark.column + 1}" if mark else ""
        pytest.fail(
            f"{_kurz(pfad)} laesst sich nicht lesen{ort}: {getattr(e, 'problem', e)}.\n"
            f"Haeufigste Ursache im Haus: ein `grund:`-Text INNERHALB einer Flow-Klammer `{{...}}`, "
            f"der ein deutsches oeffnendes Anfuehrungszeichen („, U+201E) mit einem ASCII-\" "
            f"schliesst. Das sind zwei verschiedene Zeichen — das ASCII-Zeichen beendet den Skalar "
            f"und die Klammer bleibt offen. Entweder mit “ (U+201C) schliessen oder die Bedingung "
            f"als Block schreiben statt als Flow-Mapping.")
    assert isinstance(inhalt, dict), (
        f"{_kurz(pfad)} laedt, ist aber kein Mapping, sondern {type(inhalt).__name__} — eine "
        f"Bindungsdatei muss auf oberster Ebene Schluessel tragen (version, scheibe, bindungen).")


@pytest.mark.parametrize("pfad", [p for p in YAML_DATEIEN
                                  if os.path.basename(p).startswith("bindung_")],
                         ids=lambda p: os.path.basename(p))
def test_bindungsdatei_hat_eine_bindungsliste(pfad):
    """`lade_bindung()` liest ausschliesslich `d.get("bindungen", [])`. Fehlt der Schluessel oder
    ist er kein Liste, ist die Datei zwar lesbar, aber ihr Inhalt unsichtbar — tote Verdrahtung,
    die kein Ladefehler meldet."""
    with open(pfad, encoding="utf-8") as fh:
        inhalt = yaml.safe_load(fh)
    assert "bindungen" in inhalt, (
        f"{_kurz(pfad)} hat keinen Schluessel `bindungen` — lade_bindung() liest die Datei dann "
        f"folgenlos ein, jedes Feld darin waere unerreichbar.")
    assert isinstance(inhalt["bindungen"], list), (
        f"{_kurz(pfad)}: `bindungen` ist {type(inhalt['bindungen']).__name__}, erwartet wird eine "
        f"Liste.")


def test_keine_feld_id_doppelt_ueber_alle_dateien():
    """`lade_bindung()` schreibt alle Dateien in EIN dict — bei doppelter feld_id gewinnt
    stillschweigend die zuletzt eingelesene Datei, und `glob` gibt keine zugesagte Reihenfolge.

    Das ist kein Ladefehler und faellt sonst nirgends auf: die Bindung waere je nach
    Dateisystem-Reihenfolge eine andere. Deshalb hier mitgeprueft, wo die Dateien ohnehin schon
    einzeln geoeffnet werden.
    """
    herkunft: dict[str, str] = {}
    doppelt: list[str] = []
    for pfad in YAML_DATEIEN:
        with open(pfad, encoding="utf-8") as fh:
            inhalt = yaml.safe_load(fh) or {}
        for eintrag in inhalt.get("bindungen") or []:
            fid = eintrag.get("feld_id")
            if fid is None:
                continue
            if fid in herkunft:
                doppelt.append(f"{fid} (in {herkunft[fid]} und {_kurz(pfad)})")
            else:
                herkunft[fid] = _kurz(pfad)
    assert not doppelt, (
        "Diese feld_id stehen in mehr als einer Bindungsdatei — welche gilt, entscheidet die "
        "Reihenfolge von glob(): " + "; ".join(sorted(doppelt)))
