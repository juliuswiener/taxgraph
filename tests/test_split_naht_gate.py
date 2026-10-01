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
(c) STATISCH: `from X import Y` auf einen Namen, den jemand zur Laufzeit umbiegt. Das ist
    die ZWEITE HÄLFTE derselben Fehlerklasse: (a) fängt `import *`, also alle Namen auf
    einmal; ein einzelner `from X import Y` war bis 2026-10-01 unsichtbar. Am 2026-10-01
    zweimal real geworden — `api_llm.py:25 from pii_filter import filtere` (ein Test, der
    `pii_filter.filtere` patcht, war STILL WIRKUNGSLOS: der PII-Filter lief ungefiltert an
    den Anbieter und der Test blieb grün) und `audit.py:37` (dieselbe Bauart, 1145 Zeilen
    ins Nutzerprotokoll). Die Regel ist in (c) gemessen, nicht geraten: siehe
    FROM_IMPORT_AUSNAHMEN und den Positiv-Test daneben.
"""

from __future__ import annotations

import ast
import os
import re
import sys
import pathlib

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/eingang", "golden", "elster"):
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


# ------------------------------------- (c) STATISCH: `from X import Y` auf laufzeit-veraenderliche Werte
#
# Die zweite Hälfte derselben Fehlerklasse. (a) fängt `import *`, also alle Namen auf einmal.
# `from X import Y` auf EINEN Namen war bis 2026-10-01 unsichtbar — und ist zweimal real
# geworden: `api_llm.py:25` (PII-Filter) und `audit.py:37` (Protokollpfad).
#
# DIE REGEL, wie sie gemessen wurde (nicht geraten):
#   Ein `from X import Y` unter produkt/ ist eine Naht, wenn
#     (i)  Y im Quellmodul X überhaupt existiert (Global ODER def/class), UND
#     (ii) irgendwo im Repo `m.Y = …` oder `setattr(m, "Y", …)` steht.
#   (ii) ist das Symptom: jemand biegt den Namen um, den er für den Wert hält, und trifft
#   nur seine eigene Kopie. Wo niemand umbiegt, ist es keine Naht — eine Konstante, die
#   niemand anfasst, kann nicht auseinanderlaufen.
#
# BREITE, gemessen am 2026-10-01 gegen HEAD (12.463 Knoten, produkt/ + tests/ + reports/):
#   ohne Bedingung (ii):  58 Meldungen  → als Tor unbrauchbar, wird abgeschaltet
#   mit  Bedingung (ii):   2 Meldungen  → davon 1 echte Falle, 1 reiner Re-Export
# Bedingung (ii) ist also das, was die Regel scharf hält — nicht (iii) „benutzt den Namen
# selbst", die nur noch 1 daraus macht. Die 2. Meldung wird deshalb NICHT weggeregelt,
# sondern benannt: siehe FROM_IMPORT_AUSNAHMEN, und die Begründung dort wird nachgeprüft.

_UMBIEGER = re.compile(r"\b([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*)\s*=(?!=)")
_UMBIEGER_SETATTR = re.compile(
    r"setattr\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*,\s*[\"']([A-Za-z_][A-Za-z0-9_]*)[\"']")

_SCAN_AUS = {"_build", ".git", "target", "__pycache__", "site-packages",
             "graphify-out", "node_modules", ".venv", "venv"}

# Benannte Ausnahmen — jeder Eintrag ist (Datei, Quellmodul, Name) und MUSS eine Begründung
# tragen. Die Begründung ist kein Kommentar: `benutzt_selbst` wird nachgeprüft, damit die
# Ausnahme nicht stillschweigend falsch wird, wenn jemand den Namen später doch aufruft.
FROM_IMPORT_AUSNAHMEN: dict[tuple[str, str, str], dict] = {
    ("produkt/bescheid/bescheid.py", "bescheid_deklaration", "_an_gesamt_sperrgrund"): {
        "benutzt_selbst": False,
        "warum": (
            "Reiner Re-Export: bescheid.py holt den Namen nur in seinen Namensraum, damit "
            "`from bescheid import …` ihn findet (kein __init__.py, bescheid.py IST die "
            "Paketfassade). Die Datei RUFT ihn nicht selbst auf — die vier echten Aufrufe "
            "stehen alle in produkt/haut/api.py:350/471/580/725, und api.py importiert den "
            "Namen SELBST aus bescheid_deklaration (api.py:62). Beide Umbieger "
            "(tests/test_kein_vuv_unbeantwortet_durchlaesst_guard.py:190, "
            "reports/repro/repro_rentner_partner_beginn_jahr_500.py:151) schreiben deshalb "
            "`api._an_gesamt_sperrgrund` — genau den Namen, den die Aufrufe lesen. Kein "
            "Pfad führt durch die Kopie in bescheid.py, also kann sie heute niemanden "
            "täuschen. Wird der Name dort später aufgerufen, wird `benutzt_selbst` True "
            "und dieser Test rot — die Ausnahme kann nicht verrotten."),
    },
}


def _modul_von(rel: str) -> str:
    return rel[:-3].replace("/", ".")


def _scan_dateien(wurzel: pathlib.Path) -> list[pathlib.Path]:
    return sorted(p for p in wurzel.rglob("*.py")
                  if not any(t in p.parts for t in _SCAN_AUS))


def _definitionen(texte: dict[str, str]):
    """(Modul-Globals, def/class) je Modul — nur auf Modulebene, denn nur dort entsteht
    eine Bindung, die ein `from X import Y` abschreibt."""
    from collections import defaultdict
    globals_: dict[str, dict[str, int]] = defaultdict(dict)
    defs: dict[str, set[str]] = defaultdict(set)
    for mod, quelle in texte.items():
        try:
            baum = ast.parse(quelle)
        except SyntaxError:
            continue
        for n in baum.body:
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                defs[mod].add(n.name)
            elif isinstance(n, ast.Assign):
                for t in n.targets:
                    if isinstance(t, ast.Name):
                        globals_[mod][t.id] = n.lineno
            elif isinstance(n, ast.AnnAssign) and isinstance(n.target, ast.Name):
                globals_[mod][n.target.id] = n.lineno
    return globals_, defs


def _umbiegende_namen(texte: dict[str, str]) -> set[str]:
    """Alle Namen, die irgendwo per `m.Y = …` oder `setattr(m, "Y", …)` umgebogen werden."""
    raus: set[str] = set()
    for quelle in texte.values():
        for rx in (_UMBIEGER, _UMBIEGER_SETATTR):
            for m in rx.finditer(quelle):
                raus.add(m.group(2))
    return raus


def _geladene_namen(quelle: str) -> set[str]:
    """Namen, die das Modul selbst LIEST.

    `from x import y` erzeugt KEINEN Name-Knoten (nur einen `alias`) — hier darf also
    nichts verworfen werden. Genau dieser Fehler hat die erste Messung auf 0 gedrückt,
    obwohl api_llm.py `pii_filter.filtere(...)` dreimal aufruft."""
    try:
        baum = ast.parse(quelle)
    except SyntaxError:
        return set()
    return {n.id for n in ast.walk(baum)
            if isinstance(n, ast.Name) and isinstance(n.ctx, ast.Load)}


def _naehte(quelltexte: dict[str, str], alle_texte: dict[str, str]) -> list[dict]:
    """Alle `from X import Y`-Nähte in `quelltexte`, gemessen gegen `alle_texte`.

    Reine Funktion über Textabbildungen, damit die Positiv-Prüfung sie mit einem
    synthetischen Baum füttern kann — ein Tor, das nie gefeuert hat, ist unbewiesen."""
    globals_, defs = _definitionen(alle_texte)
    umgebogen = _umbiegende_namen(alle_texte)
    bekannt = set(globals_) | set(defs)
    treffer = []
    for mod, quelle in quelltexte.items():
        try:
            baum = ast.parse(quelle)
        except SyntaxError:
            continue
        geladen = _geladene_namen(quelle)
        # ast.walk, NICHT baum.body: ein `from X import Y` INNERHALB einer Funktion ist
        # dieselbe Wert-Bindung, nur spaeter gebunden — sie laeuft bei JEDEM Aufruf neu und
        # bindet den dann gueltigen Wert, sodass ein Patch vor dem ersten Aufruf greift und
        # einer danach nicht. Am 2026-10-01 gemessen (HEAD 278b253): mit `baum.body` 0
        # Treffer fuer `def sende(): from pii_filter import filtere`, mit `ast.walk` 1.
        #
        # NUR HIER. `_definitionen()` bleibt auf `baum.body`: eine Bindung auf Modulebene ist
        # das, was ein `from X import Y` abschreibt. Mit `ast.walk` faenge sie lokal gebundene
        # Namen und die Regel rauschte.
        for n in ast.walk(baum):
            if not isinstance(n, ast.ImportFrom) or n.module is None:
                continue
            ziele = [k for k in bekannt if k == n.module or k.endswith("." + n.module)]
            for a in n.names:
                if a.name == "*":
                    continue
                if not any(a.name in globals_[z] or a.name in defs[z] for z in ziele):
                    continue
                if a.name not in umgebogen:
                    continue
                treffer.append({
                    "datei": mod.replace(".", "/") + ".py",
                    "zeile": n.lineno,
                    "quelle": n.module,
                    "name": a.name,
                    "lokal": a.asname or a.name,
                    "benutzt_selbst": (a.asname or a.name) in geladen,
                    "definiert_in": sorted(z for z in ziele if a.name in defs[z]) or sorted(ziele),
                })
    return treffer


def _echte_naehte() -> list[dict]:
    wurzel = pathlib.Path(ROOT)
    alle = {_modul_von(str(p.relative_to(wurzel))): p.read_text(encoding="utf-8")
            for p in _scan_dateien(wurzel)}
    quell = {m: t for m, t in alle.items() if m.startswith("produkt.")}
    return _naehte(quell, alle)


# ---- Positiv-Prüfungen: die Regel MUSS feuern, wenn die Falle da ist.

def test_regel_faengt_from_import_auf_umgebogenen_namen():
    """Synthetischer Baum mit genau der Falle vom 2026-10-01: das Quellmodul wird gepatcht,
    das importierende Modul hält eine Wert-Kopie und benutzt sie. Die Regel muss das
    melden — sonst beweist ein grüner Lauf am echten Baum nichts."""
    quell = {
        "produkt.haut.sender": (
            "from pii_filter import filtere\n\n\n"
            "def sende(text):\n"
            "    return filtere(text)[0]\n"),
    }
    alle = dict(quell)
    alle["produkt.haut.pii_filter"] = "def filtere(text):\n    return text, []\n"
    alle["tests.test_probe"] = (
        "import pii_filter\n\n\n"
        "def probe(monkeypatch):\n"
        "    monkeypatch.setattr(pii_filter, \"filtere\", lambda t: t)\n")

    gefunden = _naehte(quell, alle)

    assert [(n["datei"], n["name"], n["benutzt_selbst"]) for n in gefunden] == [
        ("produkt/haut/sender.py", "filtere", True)], (
        "die Regel hat die synthetische Falle NICHT gemeldet — dann ist sie am echten Baum "
        "unbewiesen")


def test_regel_faengt_auch_die_alias_form():
    """`from X import Y as Z` ist dieselbe Wert-Bindung, nur umbenannt. Der Umbieger trifft
    den QUELLNAMEN (`X.Y`), benutzt wird der LOKALE (`Z`) — beides muss die Regel trennen."""
    quell = {
        "produkt.haut.sender": (
            "from pii_filter import filtere as siebe\n\n\n"
            "def sende(text):\n"
            "    return siebe(text)[0]\n"),
    }
    alle = dict(quell)
    alle["produkt.haut.pii_filter"] = "def filtere(text):\n    return text, []\n"
    alle["tests.test_probe"] = "import pii_filter\npii_filter.filtere = lambda t: t\n"

    assert [(n["name"], n["lokal"], n["benutzt_selbst"]) for n in _naehte(quell, alle)] == [
        ("filtere", "siebe", True)]


def test_regel_schweigt_bei_reinem_reexport_und_bei_unberuehrtem_namen():
    """Zwei Gegenproben in einem: (1) ein reiner Re-Export (`__all__`, kein Aufruf) wird als
    `benutzt_selbst=False` gemeldet — die Ausnahme in FROM_IMPORT_AUSNAHMEN ist damit keine
    Behauptung, sondern ein Messwert. (2) Ein Name, den niemand umbiegt, ist keine Naht."""
    quell = {
        "produkt.fassade": (
            "from kern import rechner\n\n"
            "__all__ = [\"rechner\"]\n"),
        "produkt.unberuehrt": (
            "from kern import starr\n\n\n"
            "def nutze():\n"
            "    return starr()\n"),
    }
    alle = dict(quell)
    alle["produkt.kern"] = (
        "def rechner():\n    return 1\n\n\ndef starr():\n    return 2\n")
    alle["tests.test_probe"] = "import kern\nkern.rechner = lambda: 9\n"

    gefunden = _naehte(quell, alle)

    assert [(n["datei"], n["name"], n["benutzt_selbst"]) for n in gefunden] == [
        ("produkt/fassade.py", "rechner", False)], (
        "entweder wurde der Re-Export als benutzt gemeldet (dann ist die Ausnahme unten "
        "wertlos) oder `starr` fälschlich als Naht (dann meldet die Regel Rauschen)")


def test_regel_faengt_auch_den_import_in_einer_funktion():
    """`from X import Y` INNERHALB einer Funktion ist dieselbe Wert-Bindung, nur spaeter
    gebunden — und war ungemeldet, solange die Regel ueber `baum.body` lief und damit nur
    die Modulebene sah.

    Gemessen am 2026-10-01 gegen die ausgelieferte Regel (HEAD 278b253) mit diesem
    synthetischen Baum: funktionslokal **0** Treffer, modulweit **1**. Der Fix ist deshalb
    genau eine Zeile — `ast.walk(baum)` statt `baum.body` — und NUR im Import-Scan;
    `_definitionen()` bleibt auf Modulebene, sonst faengt sie lokal gebundene Namen.

    Der Funktions-Import ist der gefaehrlichere von beiden: er laeuft bei JEDEM Aufruf neu
    und bindet den dann gueltigen Wert. Ein Patch vor dem ersten Aufruf greift, einer danach
    nicht — zwei Laeufe desselben Codes verhalten sich verschieden."""
    quell = {
        "produkt.haut.spaet": (
            "def sende(text):\n"
            "    from pii_filter import filtere\n"
            "    return filtere(text)[0]\n"),
    }
    alle = dict(quell)
    alle["produkt.haut.pii_filter"] = "def filtere(text):\n    return text, []\n"
    alle["tests.test_probe"] = (
        "import pii_filter\n\n\n"
        "def probe(monkeypatch):\n"
        "    monkeypatch.setattr(pii_filter, \"filtere\", lambda t: t)\n")

    assert [(n["datei"], n["name"], n["benutzt_selbst"]) for n in _naehte(quell, alle)] == [
        ("produkt/haut/spaet.py", "filtere", True)], (
        "der Import INNERHALB der Funktion wurde nicht gemeldet — die Regel sieht nur die "
        "Modulebene (baum.body statt ast.walk)")


def test_regel_schweigt_bei_der_richtigen_bauart_import_modul():
    """Die Gegenprobe, und der Grund, warum die Regel nicht breiter sein darf:
    `import X` + `X.Y(...)` ist die KORREKTE Bauart — hier trifft ein Patch auf `X.Y` genau
    die Stelle, die gelesen wird. Eine Regel, die auch `ast.Import` mitlaese, meldete jede
    umgebogene Basis im Repo.

    Beleg, dass sie strukturell schweigt und nicht durch einen Filter: die Regel laeuft ueber
    `ast.ImportFrom`. `import X` erzeugt einen `ast.Import`-Knoten, der nie angeschaut wird.

    Gemessen am 2026-10-01 gegen HEAD 278b253 (`produkt/`): **174** `ast.Import`-Fundstellen,
    davon **30**, bei denen die Regel feuerte — alle 30 waeren Fehlalarme, denn `X.Y(...)`
    liest den Namen erst zur Aufrufzeit und folgt jedem Patch. Die gebaute Regel meldet 0."""
    quell = {
        "produkt.haut.sender": (
            "import pii_filter\n\n\n"
            "def sende(text):\n"
            "    return pii_filter.filtere(text)[0]\n"),
    }
    alle = dict(quell)
    alle["produkt.haut.pii_filter"] = "def filtere(text):\n    return text, []\n"
    alle["tests.test_probe"] = (
        "import pii_filter\n\n\n"
        "def probe(monkeypatch):\n"
        "    monkeypatch.setattr(pii_filter, \"filtere\", lambda t: t)\n")

    assert _naehte(quell, alle) == [], (
        "die Regel meldet `import pii_filter` + `pii_filter.filtere(...)` — das ist die "
        "richtige Bauart, kein Fehlalarm erlaubt")


def test_from_import_auf_umgebogene_namen_nur_benannte_ausnahmen():
    """Am ECHTEN Baum: jede gemeldete Naht muss benannt sein, und jede Ausnahme muss die
    Begründung tragen, die sie behauptet."""
    gefunden = {(n["datei"], n["quelle"], n["name"]): n for n in _echte_naehte()}
    ausnahmen = {(d, q, nm) for (d, q, nm) in FROM_IMPORT_AUSNAHMEN}

    unerlaubt = sorted(set(gefunden) - ausnahmen)
    assert not unerlaubt, (
        "neue ungeprüfte `from X import Y`-Naht auf einen umgebogenen Namen:\n  " +
        "\n  ".join(
            f"{d}:{gefunden[(d, q, nm)]['zeile']} from {q} import {nm} "
            f"(benutzt_selbst={gefunden[(d, q, nm)]['benutzt_selbst']})"
            for d, q, nm in unerlaubt) +
        "\nEntweder auf `import X` + `X.Y(...)` zur AUFRUFZEIT umbauen (das Muster von "
        "audit._ablage() und api_llm) — oder in FROM_IMPORT_AUSNAHMEN aufnehmen, NACHDEM "
        "geprüft ist, dass kein Pfad durch die Kopie führt.")

    fehlend = sorted(ausnahmen - set(gefunden))
    assert not fehlend, (
        f"Ausnahme {fehlend} wird nicht mehr gemeldet — aus FROM_IMPORT_AUSNAHMEN streichen.")

    for schluessel, eintrag in FROM_IMPORT_AUSNAHMEN.items():
        naht = gefunden[schluessel]
        assert naht["benutzt_selbst"] == eintrag["benutzt_selbst"], (
            f"{schluessel[0]} BENUTZT {schluessel[2]} jetzt selbst "
            f"(benutzt_selbst={naht['benutzt_selbst']}, Ausnahme sagt "
            f"{eintrag['benutzt_selbst']}). Die Begründung 'reiner Re-Export' stimmt nicht "
            f"mehr: die Kopie wird jetzt aufgerufen und kann einen Patch verdecken. "
            f"Umbauen oder die Ausnahme neu begründen.")
