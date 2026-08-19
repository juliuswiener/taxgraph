"""Split-Naht-Gate — Voraussetzung 1 aus dem Backlog-Eintrag api-py-datei-split (Vault).

`from api import _AUTH_USER` bindet den WERT beim Import, nicht den Namen (fffd7c8-Lehre).
Ein `from <modul> import *` ist dieselbe Bauart für ALLE exportierten Namen auf einmal,
nur unsichtbar — genau das tut api.py:43 (`from api_constants import *`). Wird FAELLE
(kommt aus api_constants) danach per `setattr(api, "FAELLE", ...)` gepatcht, hält api.py
seine EIGENE Kopie fest; ein ausgelagertes Modul mit derselben Star-Import-Bauart würde
die Mutation nicht sehen — still falscher Schreibpfad, Roundtrip-Assert bleibt trotzdem
grün (siehe (b)).

(a) STATISCH: kein Modul unter produkt/ darf `from X import *` machen, ausser den benannten
    Ausnahmen (analog REGELN_OHNE_GROUND_TRUTH in test_bindungstabelle.py).
(b) DYNAMISCH: FAELLE per setattr umbiegen MUSS die Schreibfunktion wirklich umlenken —
    nicht nur den Lesepfad, der bei einer Wert-Kopie ebenso falsch, aber intern konsistent
    wäre und den Roundtrip grün ließe.
"""

from __future__ import annotations

import ast
import os
import sys
import pathlib

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API  # noqa: E402
import api_auth  # noqa: E402


# ------------------------------------------------------------------ (a) STATISCH: Star-Imports

# Benannte Ausnahme (analog REGELN_OHNE_GROUND_TRUTH): jeder Eintrag ist ein (Datei, Zielmodul)-
# Paar, das WEISS eine `import *`-Wert-Bindung macht. api_constants exportiert nur reine
# Konstanten-Tupel/Dicts, von denen NUR FAELLE per setattr gepatcht wird — dokumentiert und
# geprüft in (b). Ein NEUER Star-Import ist kein Freifahrtschein; er muss hier explizit
# aufgenommen werden, sonst wird der Test rot.
STAR_IMPORT_AUSNAHMEN = {
    ("produkt/haut/api.py", "api_constants"),
}


def _produkt_py_dateien() -> list[pathlib.Path]:
    produkt = pathlib.Path(ROOT) / "produkt"
    return sorted(p for p in produkt.rglob("*.py") if "__pycache__" not in str(p))


def _star_imports() -> list[tuple[str, str, int]]:
    """(relativer_pfad, ziel_modul, zeile) für jedes `from <modul> import *` unter produkt/."""
    treffer = []
    for pfad in _produkt_py_dateien():
        rel = str(pfad.relative_to(ROOT))
        baum = ast.parse(pfad.read_text(encoding="utf-8"))
        for node in baum.body:
            if isinstance(node, ast.ImportFrom):
                for alias in node.names:
                    if alias.name == "*":
                        treffer.append((rel, node.module, node.lineno))
    return treffer


def test_star_imports_nur_benannte_ausnahmen():
    """Jeder `from X import *` unter produkt/ muss in STAR_IMPORT_AUSNAHMEN stehen.

    Blindspot sichtbar machen (wie test_bindungstabelle.REGELN_OHNE_GROUND_TRUTH): neue
    Star-Imports MUESSEN hier auftauchen, sonst ist die Wert-Bindungs-Gefahr unsichtbar."""
    gefunden = {(datei, modul) for datei, modul, _zeile in _star_imports()}
    unerlaubt = gefunden - STAR_IMPORT_AUSNAHMEN
    assert not unerlaubt, (
        f"neuer ungeprüfter Star-Import: {sorted(unerlaubt)} — entweder Star-Import entfernen "
        "(from X import <konkrete Namen>) oder bewusst in STAR_IMPORT_AUSNAHMEN aufnehmen, "
        "NACHDEM geprüft ist, dass keiner der exportierten Namen per setattr gepatcht wird.")
    fehlend = STAR_IMPORT_AUSNAHMEN - gefunden
    assert not fehlend, (
        f"Ausnahme {sorted(fehlend)} existiert nicht mehr — aus STAR_IMPORT_AUSNAHMEN streichen.")


# ------------------------------------------------------------------ (b) DYNAMISCH: FAELLE-Naht

def test_faelle_setattr_erreicht_speichere_fall(tmp_path, monkeypatch):
    """setattr(API, "FAELLE", tmp) MUSS speichere_fall wirklich in tmp schreiben lassen —
    nicht nur den Lesepfad umbiegen. Prüft die Schreibfunktion direkt (kein API-Umweg über
    fall_anlegen, damit Audit/Auth die Naht nicht verdecken)."""
    alter_pfad = API.FAELLE
    ziel = str(tmp_path / "faelle")
    monkeypatch.setattr(API, "FAELLE", ziel)

    API.speichere_fall("naht_probe", {"veranlagungszeitraum": 2025, "fall_id": "naht_probe"})

    erwartete_datei = tmp_path / "faelle" / "naht_probe.json"
    assert erwartete_datei.exists(), (
        f"speichere_fall hat NICHT in {ziel} geschrieben — FAELLE-Mutation kam nicht an "
        "(Wert-Bindung statt Attribut-Zugriff)")
    # Der echte FAELLE-Pfad (api_constants.FAELLE via alter Wert) darf NICHT verändert worden
    # sein — sonst schreibt der Test in den echten 289-Fälle-Store statt in tmp_path.
    assert not (pathlib.Path(alter_pfad) / "naht_probe.json").exists(), (
        f"speichere_fall hat in den ECHTEN FAELLE-Pfad {alter_pfad} geschrieben — "
        "setattr-Naht wirkungslos, Schreibzugriff ging am gepatchten Pfad vorbei")


def test_faelle_setattr_erreicht_fall_anlegen(tmp_path, monkeypatch):
    """Wie oben, aber über den echten Endpunkt fall_anlegen — Regressionsdeckung für den
    Pfad, den die 32 bestehenden FAELLE-Testdateien tatsächlich benutzen."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", None)

    status, body = API.fall_anlegen({"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "naht_probe2"})

    assert status == 201
    erwartete_datei = tmp_path / "faelle" / "naht_probe2.json"
    assert erwartete_datei.exists(), (
        f"fall_anlegen hat NICHT in {tmp_path / 'faelle'} geschrieben — FAELLE-Mutation kam "
        "nicht an")
