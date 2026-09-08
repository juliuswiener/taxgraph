"""Gate gegen tote 'BACKLOG <slug>'-Verweise im Produktivcode (ausserhalb reports/).

Jeder `BACKLOG <slug>`-Kommentar ist die Bindung zwischen Code und Regal (Vault). Ein
Verweis, der ins Leere zeigt, ist eine Luecke, die niemand mehr findet — s.
~/00_projects/vault/backlog/taxgraph/tote-backlog-verweise-im-code.md.

Adressraum: eine Datei backlog/taxgraph/<slug>.md ODER eine `### `slug`'-Ueberschrift in
backlog/taxgraph.md (deckt auch die nach tickets/ ausgelagerten Slugs ab, die nur noch als
Ueberschrift-Stub adressierbar sind, s. [[ein-eintrag-ist-eine-datei]]).

Vault-Pfad per TAXGRAPH_VAULT_ROOT ueberschreibbar; SKIP (nicht failen), wenn der Vault
fehlt -- CI hat ihn nicht.
"""
from __future__ import annotations

import os
import re

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

VAULT_ROOT = os.environ.get(
    "TAXGRAPH_VAULT_ROOT", os.path.expanduser("~/00_projects/vault")
)
BACKLOG_DIR = os.path.join(VAULT_ROOT, "backlog", "taxgraph")
TAXGRAPH_MD = os.path.join(VAULT_ROOT, "backlog", "taxgraph.md")

# Trenner zwischen 'BACKLOG' und dem Slug ist ENTWEDER Whitespace auf derselben Zeile ODER
# ein Zeilenumbruch, gefolgt von einer '#'-Kommentarzeile, die den Slug fortsetzt (Form 1:
# 'BACKLOG' steht am Zeilenende, der Slug faengt erst auf der naechsten Kommentarzeile an,
# z.B. api_constants.py:442f). Das '#' im zweiten Zweig ist bewusst PFLICHT, nicht optional
# -- sonst wuerde jede beliebige Folgezeile (auch Nicht-Kommentar-Code) als Fortsetzung
# gelesen. `m.start()` bleibt in beiden Zweigen die 'BACKLOG'-Zeile, `line_no` also exakt.
REF_RE = re.compile(r"BACKLOG[:,]?(?:[ \t]+|[ \t]*\r?\n[ \t]*#[ \t]*)`?([A-Za-z0-9_./-]+)`?")
SLUG_SHAPE = re.compile(r"^[a-z][a-z0-9-]*$")

# Bindestrich-Zeilenumbruch (Form 2): ein SLUG, der am Zeilenende mit '-' abbricht und auf
# der naechsten Kommentarzeile ('# ...') weitergeht, ist EIN Slug -- anders als Form 1 oben
# bricht hier nicht 'BACKLOG', sondern der Slug selbst mitten im Wort um (z.B.
# test_checkest_durchstich.py:85f). Bewusst KEINE Erweiterung der Zeichenklasse in REF_RE
# um \s -- das wuerde beliebige Folgewoerter mit in den Slug ziehen. Stattdessen wird nur
# bei einem Token, das auf '-' endet, gezielt auf der naechsten '#'-Zeile weitergesucht
# (_find_backlog_refs).
_CONT_RE = re.compile(r"[ \t]*\r?\n[ \t]*#[ \t]*")
_SLUG_CHARS_RE = re.compile(r"[A-Za-z0-9_./-]+")


# Nur versionierter Inhalt zaehlt (wie git grep/git archive im Vault-Eintrag) --
# .audit/ und graphify-out/ sind gitignorierte, generierte Verzeichnisse mit Prosa-
# bzw. Report-Text, in dem 'BACKLOG' vorkommt, ohne ein Kommentar-Verweis zu sein.
EXCLUDED_TOP_DIRS = {"reports", ".git", ".audit", "graphify-out"}


def _files_outside_reports(root):
    for dirpath, dirnames, filenames in os.walk(root):
        rel = os.path.relpath(dirpath, root)
        top = rel.split(os.sep, 1)[0]
        if top in EXCLUDED_TOP_DIRS:
            dirnames[:] = []
            continue
        # __pycache__ liegt fast nie direkt unter root (immer verschachtelt, z.B.
        # tests/__pycache__) -- der Top-Level-Check oben trifft das nie. Deshalb hier,
        # auf jeder Ebene des Walks, unabhaengig von der Tiefe.
        dirnames[:] = [d for d in dirnames if d != "__pycache__"]
        for fn in filenames:
            yield os.path.join(dirpath, fn)


def _known_headings():
    with open(TAXGRAPH_MD, encoding="utf-8") as fh:
        text = fh.read()
    return set(re.findall(r"^### `([a-z0-9-]+)`", text, re.MULTILINE))


def _slug_resolves(slug, known_headings):
    if os.path.isfile(os.path.join(BACKLOG_DIR, f"{slug}.md")):
        return True
    return slug in known_headings


def _find_backlog_refs(root):
    refs = []
    for path in _files_outside_reports(root):
        if os.path.abspath(path) == os.path.abspath(__file__):
            continue
        try:
            with open(path, encoding="utf-8", errors="ignore") as fh:
                text = fh.read()
        except OSError:
            continue
        if "BACKLOG" not in text:
            continue
        for m in REF_RE.finditer(text):
            token = m.group(1)
            end = m.end(1)
            # Umbruch-Join lokal am Token, NICHT in `text` selbst -- eine Aenderung von
            # `text` wuerde Zeichen entfernen und `line_no` (unten, ueber `text.count`)
            # fuer JEDEN nachfolgenden Treffer verschieben. `m.start()` zeigt dadurch immer
            # noch exakt auf die BACKLOG-Zeile, auch bei einem zusammengefuehrten Slug.
            while token.endswith("-"):
                cont = _CONT_RE.match(text, end)
                if not cont:
                    break
                nxt = _SLUG_CHARS_RE.match(text, cont.end())
                if not nxt:
                    break
                token += nxt.group(0)
                end = nxt.end()
            token = token.rstrip(".,);:\"'")
            first = token.split("/")[0]
            if not SLUG_SHAPE.match(first):
                continue
            line_no = text.count("\n", 0, m.start()) + 1
            rel = os.path.relpath(path, root)
            refs.append((rel, line_no, token, first))
    return refs


@pytest.mark.skipif(
    not os.path.isdir(VAULT_ROOT), reason="Vault nicht vorhanden (CI hat ihn nicht)"
)
def test_backlog_verweise_loesen_auf():
    known_headings = _known_headings()
    dead = [
        (rel, line_no, token)
        for rel, line_no, token, first in _find_backlog_refs(ROOT)
        if not _slug_resolves(first, known_headings)
    ]
    assert not dead, "Tote BACKLOG-Verweise (loesen nicht auf):\n" + "\n".join(
        f"  {rel}:{line_no}: BACKLOG {token}" for rel, line_no, token in dead
    )


def test_zeilenumbruch_slug_wird_zusammengefuehrt_woerter_nicht(tmp_path):
    """(Auftrag 2026-09-08) Ein Slug, der am Zeilenende mit '-' abbricht und auf der naechsten
    Kommentarzeile ('# ...') weitergeht, ist EIN Slug -- mit der Zeilennummer der BACKLOG-Zeile,
    nicht der Fortsetzungszeile (s. Kommentar bei `_find_backlog_refs`: der Join passiert lokal
    am Token, `text` bleibt fuer `line_no` unangetastet). Direkt daneben zwei EIGENSTAENDIGE
    Kommentare, deren zweiter zufaellig auch slugfoermig aussieht -- die duerfen NICHT
    zusammengezogen werden, sonst waere aus dem blinden Pruefer ein erfindender geworden."""
    quelle = tmp_path / "quelle.py"
    quelle.write_text(
        "eins\n"                                                        # Zeile 1
        "zwei\n"                                                        # Zeile 2
        "# Kommentar (BACKLOG umbruch-test-\n"                          # Zeile 3 -- Umbruch-Anfang
        "# teil-zwei) Rest des Satzes.\n"                                # Zeile 4 -- Fortsetzung
        "# Eigener Gedanke (BACKLOG woerter-schon-fertig) hier.\n"       # Zeile 5 -- vollstaendig
        "# naechster-satz-faengt-zufaellig-slugartig-an, unabhaengig.\n",  # Zeile 6
        encoding="utf-8",
    )
    refs = {(token, line_no) for _, line_no, token, _ in _find_backlog_refs(str(tmp_path))}
    assert ("umbruch-test-teil-zwei", 3) in refs, (
        "Umbruch-Slug loest nicht zu EINEM Token mit der BACKLOG-Zeilennummer auf.")
    assert ("woerter-schon-fertig", 5) in refs, "vollstaendiger Slug auf einer Zeile geht verloren."
    assert not any(t.startswith("woerter-schon-fertig-naechster") for t, _ in refs), (
        "zwei unabhaengige Kommentarzeilen wurden zusammengezogen.")
    assert len(refs) == 2, f"unerwartete zusaetzliche/fehlende Treffer: {refs}"


def test_backlog_am_zeilenende_slug_auf_naechster_kommentarzeile(tmp_path):
    """(Auftrag 2026-09-08, 24-B) Form 1 des Zeilenumbruchs: nicht der SLUG bricht mitten im
    Wort um (das deckt schon test_zeilenumbruch_slug_...), sondern 'BACKLOG' selbst steht am
    Zeilenende und der Slug faengt erst auf der naechsten '#'-Kommentarzeile an -- reale Faelle
    u.a. api_constants.py:442, est_mapping.py:430, test_bindungstabelle.py:499,
    test_checkest_blockmatrix.py:243. `line_no` muss weiterhin die 'BACKLOG'-Zeile sein, nicht
    die Fortsetzungszeile. Gegenfall: eine Zeile, die auf 'BACKLOG' endet, gefolgt von einer
    Zeile OHNE '#' (also keinem Kommentar, sondern z.B. Code) -- die darf NICHT als Fortsetzung
    gelesen werden, sonst wuerde jede beliebige Folgezeile zum Slug-Rater."""
    quelle = tmp_path / "quelle.py"
    quelle.write_text(
        "eins\n"                                                          # Zeile 1
        "# Text vor dem Umbruch, siehe unten (BACKLOG\n"                  # Zeile 2 -- Umbruch-Anfang
        "# eol-umbruch-form-eins) Rest des Satzes.\n"                     # Zeile 3 -- Fortsetzung
        "# Kaputter Fall, endet auch auf BACKLOG\n"                       # Zeile 4 -- OHNE Fortsetzungs-#
        "kein_kommentar = 'eol-ohne-hash-darf-nicht-gefunden-werden'\n",   # Zeile 5 -- Code, kein '#'
        encoding="utf-8",
    )
    refs = {(token, line_no) for _, line_no, token, _ in _find_backlog_refs(str(tmp_path))}
    assert ("eol-umbruch-form-eins", 2) in refs, (
        "BACKLOG-am-Zeilenende loest nicht zu einem Token mit der BACKLOG-Zeilennummer auf.")
    assert not any(t.startswith("eol-ohne-hash") for t, _ in refs), (
        "eine Nicht-Kommentarzeile (kein fuehrendes '#') wurde faelschlich als Fortsetzung gelesen.")
    assert len(refs) == 1, f"unerwartete zusaetzliche/fehlende Treffer: {refs}"


def test_pycache_wird_nicht_gescannt_auf_jeder_ebene(tmp_path):
    """(Auftrag 2026-09-08) `_files_outside_reports` prueft nur das ERSTE Pfadsegment relativ zu
    root gegen EXCLUDED_TOP_DIRS -- ein verschachtelter `__pycache__` (wie ueberall im Repo,
    z.B. `tests/__pycache__`) wurde davon nie erfasst, und es gibt keine Endungsfilterung, also
    landete kompilierter Bytecode als Text im Scan. `__pycache__` muss deshalb bei JEDEM Besuch
    aus `dirnames` gestrichen werden, unabhaengig von der Tiefe."""
    normal = tmp_path / "unterordner" / "datei.py"
    normal.parent.mkdir(parents=True)
    normal.write_text("# BACKLOG ausserhalb-pycache -- muss gefunden werden\n", encoding="utf-8")

    versteckt = tmp_path / "unterordner" / "__pycache__" / "datei.cpython-314.pyc"
    versteckt.parent.mkdir(parents=True)
    versteckt.write_bytes(b"BACKLOG sollte-nie-auftauchen\x00\x01\x02irgendein Bytecode-Muell")

    gefundene_pfade = set(_files_outside_reports(str(tmp_path)))
    assert str(normal) in gefundene_pfade, "normale Datei ausserhalb __pycache__ fehlt."
    assert not any("__pycache__" in p for p in gefundene_pfade), (
        "__pycache__ wurde trotz verschachtelter Lage durchlaufen.")

    refs = [(rel, line_no, token) for rel, line_no, token, _ in _find_backlog_refs(str(tmp_path))]
    assert any(t == "ausserhalb-pycache" for _, _, t in refs)
    assert not any(t == "sollte-nie-auftauchen" for _, _, t in refs), (
        "Bytecode aus __pycache__ wurde als BACKLOG-Verweis gelesen.")
