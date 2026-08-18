"""Konzept-Grenze zwischen Haut und Rechenkern — strukturell statt als Vorsatz.

api.py sagt seit dem ersten Tag in der eigenen Moduldoku: "Keine Steuerlogik, keine zweite
Wahrheit hier." Gemessen am 2026-08-18 standen darin 3579 Zeilen, davon 1874 in Funktionen mit
`runner.catala_*`-Aufrufen — also mehr als die Hälfte des Moduls im direkten Widerspruch zu
seinem eigenen Vorspann. Ein Vorsatz, den nichts prüft, ist kein Vorsatz, sondern ein Kommentar.

Dieses Gate zieht die Grenze in drei Lagen, die verschiedene Fehler fangen:

(A) RATSCHE. Die Zahl der `runner.`-Stellen in api.py darf nie steigen. Sie ist zugleich das
    Fortschrittsmaß der Phase-3-Extraktion: jeder verschobene Block senkt sie. Die Ratsche
    verlangt AUSDRÜCKLICH, dass die Obergrenze beim Sinken nachgezogen wird — sonst verrottet
    sie nach oben und erlaubt später wieder das, was heute abgebaut wird.

(B) KEINE ZWEITE KOPIE. Keine Funktion darf gleichzeitig in api.py UND unter produkt/bescheid/
    definiert sein. Das ist die einzige Art, wie diese Extraktion echten Schaden anrichten kann,
    und die Bugklasse ist belegt: kist-bemessungsgrundlage-doppelbug.md — derselbe Paragraph
    zweimal gepflegt, einmal auf 0 EUR für JEDEN Kirchensteuerpflichtigen, einmal 1.102 EUR zu
    viel. Wer beim Verschieben die alte Fassung stehen lässt, baut genau das noch einmal.

(C) EINE WAHRHEIT ÜBER DIE NAHT. Die Funktionen, die der Rest von api.py aus dem Kern ruft,
    müssen über `api.<name>` erreichbar bleiben (23 Testdateien greifen so zu) und dabei
    DASSELBE Objekt sein wie im Kernmodul — `is`, nicht nur gleichnamig. `from x import y`
    bindet den Wert, nicht den Namen (fffd7c8-Lehre, s. test_split_naht_gate.py); eine zweite
    Bindung wäre unsichtbar, solange niemand patcht, und still falsch, sobald jemand es tut.

Was dieses Gate NICHT kann: `_an_gesamt_sperrgrund` sind 458 Zeilen Steuerlogik OHNE einen
einzigen runner-Aufruf. Lage (A) sieht sie nicht. Deshalb Lage (D): eine Namensliste der noch
ausstehenden Umzüge mit Tote-Einträge-Wächter (Muster: AUSNAHMEN in
test_zweig_duplikation_differential.py). Die Liste ist ein Arbeitsstand, kein Freibrief — sie
darf nur schrumpfen, und ein Name, der nicht mehr in api.py steht, muss raus.

NULL LLM, kein Catala nötig — reine AST-Prüfung plus ein Import.
"""
from __future__ import annotations

import ast
import os
import pathlib
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

API_PFAD = os.path.join(ROOT, "produkt", "haut", "api.py")
BESCHEID_DIR = pathlib.Path(ROOT) / "produkt" / "bescheid"


# --------------------------------------------------------------------------- (A) Ratsche

# Beim Anlegen dieses Gates am 2026-08-18 standen hier 132 (nicht 139: eine Textsuche nach
# `runner.` zählt sieben Nennungen in Kommentaren mit, die keine Abhängigkeit sind — deshalb
# AST und nicht Text, sonst schlägt die Ratsche beim Umformulieren eines Kommentars an oder
# gibt nach). Nach der Extraktion in drei Schichten sind es 0, und damit ist die Ratsche keine
# Ratsche mehr, sondern die Grenze selbst: jede einzelne neue Rechenstelle in api.py ist rot.
RUNNER_STELLEN_OBERGRENZE = 0

# Zweite Ratsche, gegen die Lücke, die die erste offenlässt: `_an_gesamt_sperrgrund` waren 458
# Zeilen Steuerlogik OHNE einen einzigen runner-Aufruf. Eine Zeilen-Obergrenze ist grob, aber
# sie ist das einzige Maß, das solche Logik überhaupt sieht — und sie kann nicht vakuum-grün
# werden. api.py hatte 3579 Zeilen und darf nicht zurückwachsen.
#
# NACHGEZOGEN am 2026-08-18, zweimal an einem Tag: 1055 → 1065 (Behandlung der
# OCR-Zeitüberschreitung am Kontoauszug-Endpunkt) → 1078 (Hinweistext für die übersprungenen
# LLM-Klassifikationen). Das ist der Zweck dieser Ratsche, nicht ihre Umgehung — sie erzwingt,
# dass jedes Wachstum begründet wird, statt es zu verbieten. Beide Male ging es um eine
# Fehler- bzw. Hinweisantwort an einem Endpunkt: das gehört in die Haut. Rechenlogik gehört es
# nicht, und dafür ist Lage (A) ohnehin schärfer. Wer diese Zahl hebt, schreibt daneben, wofür.
API_ZEILEN_OBERGRENZE = 1078


def _runner_stellen(pfad: str) -> list[int]:
    """Zeilennummern jedes Attributzugriffs auf den Namen `runner` — `runner.catala_est(...)`
    ebenso wie ein bloßes `runner.foo`. Über den AST, nicht per Textsuche: ein `runner.` im
    Kommentar oder in einem Docstring ist keine Abhängigkeit und soll die Zahl nicht bewegen."""
    baum = ast.parse(pathlib.Path(pfad).read_text(encoding="utf-8"), filename=pfad)
    return sorted(n.lineno for n in ast.walk(baum)
                  if isinstance(n, ast.Attribute) and isinstance(n.value, ast.Name)
                  and n.value.id == "runner")


def test_ratsche_runner_stellen_steigt_nie():
    """Der Rechenkern wächst in api.py nicht weiter. Neue Steuerlogik gehört nach
    produkt/bescheid/ — auch dann, wenn dort gerade noch alte steht."""
    stellen = _runner_stellen(API_PFAD)
    assert len(stellen) <= RUNNER_STELLEN_OBERGRENZE, (
        f"api.py hat jetzt {len(stellen)} `runner.`-Stellen, erlaubt sind "
        f"{RUNNER_STELLEN_OBERGRENZE}. Neue Rechenstellen gehören nach produkt/bescheid/, "
        f"nicht in die Endpunkt-Schicht.\n"
        f"Zeilen: {stellen[-10:]}")


def test_ratsche_ist_nachgezogen():
    """Ohne diesen Test verrottet die Ratsche nach oben: wer 40 Stellen auslagert und die
    Obergrenze stehen lässt, erlaubt stillschweigend, dass 40 neue nachwachsen. Der Fortschritt
    muss in der Konstante ankommen, sonst ist er nicht gesichert."""
    ist = len(_runner_stellen(API_PFAD))
    assert ist == RUNNER_STELLEN_OBERGRENZE, (
        f"api.py hat nur noch {ist} `runner.`-Stellen, die Ratsche steht aber auf "
        f"{RUNNER_STELLEN_OBERGRENZE}. RUNNER_STELLEN_OBERGRENZE auf {ist} setzen, damit der "
        f"erreichte Stand gehalten wird.")


def test_api_zeilen_ratsche():
    """Gegen Steuerlogik ohne runner-Aufruf, die Lage (A) nicht sieht. Beim Unterschreiten
    ebenso nachziehen — eine Obergrenze, die 500 Zeilen Luft lässt, ist keine."""
    ist = len(pathlib.Path(API_PFAD).read_text(encoding="utf-8").splitlines())
    assert ist <= API_ZEILEN_OBERGRENZE, (
        f"api.py ist auf {ist} Zeilen gewachsen (erlaubt: {API_ZEILEN_OBERGRENZE}). Neue Logik "
        f"gehört in den Rechenkern oder ein eigenes Modul, nicht in die Endpunkt-Schicht.")
    assert ist >= API_ZEILEN_OBERGRENZE - 40, (
        f"api.py ist auf {ist} Zeilen geschrumpft, die Ratsche steht auf "
        f"{API_ZEILEN_OBERGRENZE}. API_ZEILEN_OBERGRENZE auf {ist} setzen — sonst darf ab jetzt "
        f"wieder wachsen, was gerade abgebaut wurde.")


# ------------------------------------------------------------------ (B) keine zweite Kopie

def _def_namen(pfad: pathlib.Path) -> dict[str, int]:
    """Top-level def/class-Namen einer Datei mit Zeilennummer. Verschachtelte Closures zählen
    nicht — die sind Implementierungsdetail der umgebenden Funktion und wandern mit ihr."""
    baum = ast.parse(pfad.read_text(encoding="utf-8"), filename=str(pfad))
    return {n.name: n.lineno for n in baum.body
            if isinstance(n, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef))}


def _bescheid_dateien() -> list[pathlib.Path]:
    if not BESCHEID_DIR.is_dir():
        return []
    return sorted(p for p in BESCHEID_DIR.rglob("*.py") if "__pycache__" not in str(p))


def test_keine_funktion_ist_doppelt_definiert():
    """Der Kern der Sache. Beim Verschieben ist die gefährlichste Zwischenstellung nicht die
    halbfertige, sondern die scheinbar fertige: neue Fassung angelegt, alte nicht gelöscht.
    Beide sind dann aufrufbar, beide grün, und der nächste Fix landet in genau einer von
    beiden."""
    in_api = _def_namen(pathlib.Path(API_PFAD))
    doppelt = []
    for pfad in _bescheid_dateien():
        for name, zeile in _def_namen(pfad).items():
            if name.startswith("__"):
                continue          # __init__ u.ä. sind Modulmechanik, keine Rechenstelle
            if name in in_api:
                doppelt.append(f"{name}: api.py:{in_api[name]} UND "
                               f"{pfad.relative_to(ROOT)}:{zeile}")
    assert not doppelt, (
        "Dieselbe Funktion ist in api.py und im Rechenkern definiert — ein Fix landet künftig "
        "in genau einer der beiden Fassungen (kist-bemessungsgrundlage-doppelbug.md):\n  "
        + "\n  ".join(doppelt))


# --------------------------------------------------------------- (C) eine Wahrheit über die Naht

# Die vier Funktionen, die der Rest von api.py aus dem Rechenkern ruft — gemessen per AST am
# 2026-08-18: _feste_zahl/_gesamt_beitrag/stand/ergebnis rufen _bescheid_fn, stand/ergebnis/
# einreichen rufen _an_gesamt_sperrgrund, ergebnis ruft _abschlusszahlung_cent, deklaration/
# einreichen rufen _mit_ring_werten. Mehr Naht gibt es nicht; der Kern seinerseits ruft KEINE
# Funktion aus api.py zurück (geprüft in test_kern_ruft_nicht_in_die_haut_zurueck).
NAHT = ("_bescheid_fn", "_an_gesamt_sperrgrund", "_abschlusszahlung_cent", "_mit_ring_werten")


def test_naht_funktionen_bleiben_ueber_api_erreichbar():
    """23 Testdateien greifen auf `API._an_gesamt_sperrgrund` & Co. zu. Nach dem Umzug muss der
    Name über api weiter aufgelöst werden — sonst ist die Extraktion ein Bruch und kein
    Refactor."""
    import api as API
    fehlend = [n for n in NAHT if not callable(getattr(API, n, None))]
    assert not fehlend, (
        f"Naht-Funktion(en) über `api.` nicht mehr erreichbar: {fehlend} — nach dem Umzug im "
        f"Kernmodul re-exportieren (`from bescheid import ...`).")


def test_naht_ist_dasselbe_objekt_wie_im_kern():
    """`is`, nicht nur gleichnamig. Zwei Bindungen desselben Namens sehen im grünen Fall
    identisch aus und laufen im Fehlerfall auseinander — das ist die Bauart, die beim
    api.py-Split beinahe einen stillen Auth-Bypass gebaut hätte (fffd7c8)."""
    if not _bescheid_dateien():
        import pytest
        pytest.skip("produkt/bescheid/ existiert noch nicht — Phase 3 nicht begonnen")
    import api as API
    try:
        import bescheid as BE
    except ImportError as e:      # pragma: no cover - Diagnose, nicht Normalfall
        import pytest
        pytest.skip(f"bescheid nicht importierbar: {e}")
    for name in NAHT:
        im_kern = getattr(BE, name, None)
        if im_kern is None:
            continue              # noch nicht umgezogen — Lage (D) führt Buch darüber
        assert getattr(API, name) is im_kern, (
            f"{name}: api.{name} und bescheid.{name} sind VERSCHIEDENE Objekte. Ein Patch auf "
            f"das eine erreicht das andere nicht.")


# ------------------------------------------------------- (D) Arbeitsstand der Extraktion

# Funktionen, die inhaltlich Rechenkern sind und noch in api.py stehen. Die Liste darf nur
# SCHRUMPFEN; ein Name, der nicht mehr in api.py definiert ist, muss raus (Tote-Einträge-
# Wächter, Muster test_zweig_duplikation_differential.py). Sie ist der Teil der Grenze, den
# Lage (A) nicht sieht: _an_gesamt_sperrgrund etwa sind 458 Zeilen Steuerlogik ohne einen
# einzigen runner-Aufruf.
# Am 2026-08-18 in drei Schichten geleert: alle 26 Funktionen sind umgezogen. Die leere Menge
# ist der Grund, warum der Test darunter zwei Gesichter hat — eine Liste, die nichts mehr
# enthält, prüft nichts mehr, und genau so sieht Fortschritt aus wie eine abgeschaltete Prüfung.
UMZUG_OFFEN: set[str] = set()

# Untergrenze für die Gegenprobe, wenn UMZUG_OFFEN leer ist. Der Kern hat 26 Funktionen und
# 132 runner-Stellen; deutlich darunter hieße, dass die Rechenlogik nicht angekommen, sondern
# verschwunden ist — und die leere Umzugsliste wäre trotzdem grün.
KERN_MINDESTENS_FUNKTIONEN = 20
KERN_MINDESTENS_RUNNER_STELLEN = 100


def test_umzugsliste_ist_ehrlich():
    """Zwei Gesichter, je nach Stand:

    Solange Posten offen sind, muss jeder davon noch wirklich in api.py stehen — sonst
    behauptet die Liste Arbeit, die längst getan ist, und verdeckt, wieviel offen ist.

    Ist sie leer, prüft sie per Konstruktion nichts mehr. Dann tritt die Gegenprobe an ihre
    Stelle: der Rechenkern muss wirklich BEFÜLLT sein. Ohne sie wäre der grünste denkbare
    Zustand dieses Gates der, in dem jemand api.py und bescheid.py beide leerräumt."""
    in_api = _def_namen(pathlib.Path(API_PFAD))
    if UMZUG_OFFEN:
        tot = sorted(n for n in UMZUG_OFFEN if n not in in_api)
        assert not tot, (
            f"{tot} steht/stehen in UMZUG_OFFEN, sind aber nicht mehr in api.py definiert — "
            f"aus UMZUG_OFFEN streichen, der Umzug ist erledigt.")
        return
    dateien = _bescheid_dateien()
    assert dateien, ("UMZUG_OFFEN ist leer, aber produkt/bescheid/ enthält keine Python-Datei — "
                     "der Rechenkern ist nicht umgezogen, sondern weg.")
    fn_zahl = sum(len(_def_namen(p)) for p in dateien)
    runner_zahl = sum(len(_runner_stellen(str(p))) for p in dateien)
    assert fn_zahl >= KERN_MINDESTENS_FUNKTIONEN, (
        f"Rechenkern hat nur {fn_zahl} Funktionen (erwartet ≥ {KERN_MINDESTENS_FUNKTIONEN}) — "
        f"die leere Umzugsliste behauptet einen Umzug, der so nicht stattgefunden hat.")
    assert runner_zahl >= KERN_MINDESTENS_RUNNER_STELLEN, (
        f"Rechenkern hat nur {runner_zahl} `runner.`-Stellen (erwartet ≥ "
        f"{KERN_MINDESTENS_RUNNER_STELLEN}) — die Steuerlogik ist nicht dort angekommen.")


def test_kern_importiert_die_haut_nicht():
    """Die Richtung der Abhängigkeit, jetzt wo sie über Modulgrenzen läuft und damit wirklich
    prüfbar ist: bescheid kennt api nicht. Andernfalls entstünde ein Importzyklus, und die
    übliche Notlösung dagegen ist ein lokaler Import mitten im Rechenweg — der die Grenze
    faktisch wieder aufhebt, ohne dass es im Importkopf sichtbar wäre. Deshalb zählen auch
    Importe INNERHALB von Funktionen."""
    verboten = {"api", "api_llm", "api_auth", "server"}
    verstoss = []
    for pfad in _bescheid_dateien():
        baum = ast.parse(pfad.read_text(encoding="utf-8"), filename=str(pfad))
        for n in ast.walk(baum):
            if isinstance(n, ast.Import):
                for a in n.names:
                    if a.name.split(".")[0] in verboten:
                        verstoss.append(f"{pfad.name}:{n.lineno}: import {a.name}")
            elif isinstance(n, ast.ImportFrom) and n.module:
                if n.module.split(".")[0] in verboten:
                    verstoss.append(f"{pfad.name}:{n.lineno}: from {n.module} import ...")
    assert not verstoss, (
        "Rechenkern importiert die Endpunkt-Schicht — die Grenze zeigt dann in beide Richtungen "
        "und ist keine mehr:\n  " + "\n  ".join(verstoss))
