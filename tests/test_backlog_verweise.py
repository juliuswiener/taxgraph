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

REF_RE = re.compile(r"BACKLOG[:,]?\s+`?([A-Za-z0-9_./-]+)`?")
SLUG_SHAPE = re.compile(r"^[a-z][a-z0-9-]*$")


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
        # Kommentarfortsetzungen ('# slug' auf der naechsten Zeile) als zusammenhaengenden
        # Whitespace-Lauf zaehlen -- fuehrendes '#' durch Leerzeichen ersetzen.
        text = re.sub(r"^(\s*)#", r"\1 ", text, flags=re.MULTILINE)
        for m in REF_RE.finditer(text):
            token = m.group(1).rstrip(".,);:\"'")
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
