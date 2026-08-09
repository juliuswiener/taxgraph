"""Gate: kein Test darf ins ECHTE audit.jsonl schreiben.

Grund: 32 Testdateien haben genau das versehentlich getan (Audit-Log auf 582 MB
angewachsen, s. BACKLOG.yaml "audit-jsonl-wucherung"). Ein Grep nach dem Patch-
Muster (`monkeypatch.setattr(audit, "AUDIT_DIR", ...)`) faengt die Faelle NICHT,
die den Fix noetig machten: unconditionaler audit.append-Aufruf ohne jeden Patch,
eine Schwesterklasse in derselben Datei ohne Patch, eine von zwei Fixtures in
derselben Datei ohne Patch. Deshalb misst dieses Gate den echten Schreibpfad
(Dateigroesse der realen Datei) statt ein Namensmuster.

os.path.getsize statt Zeilenzahl: O(1) stat() statt die 582-MB-Datei pro Test
einzulesen (1658 Tests x wc -l waere ein Vielfaches der eigentlichen Suite-Laufzeit).
"""
from __future__ import annotations

import os
import sys

import pytest

_HERE = os.path.dirname(os.path.abspath(__file__))
_ROOT = os.path.dirname(_HERE)
_STORE = os.path.join(_ROOT, "produkt", "store")
if _STORE not in sys.path:
    sys.path.insert(0, _STORE)
_HAUT = os.path.join(_ROOT, "produkt", "haut")
if _HAUT not in sys.path:
    sys.path.insert(0, _HAUT)

import audit as _audit  # echtes Modul-Attribut, gelesen VOR jedem Test-Monkeypatch
import server as _server  # noqa: E402 — teilt server._lade_env_dateien mit dem Server-Start

# .env-Naht (s. tests/test_env_loader.py): pytest rief server._lade_env_dateien() bisher NIE auf
# (nur server.main() tat das) — dadurch blieb z.B. $ELSTER_HERSTELLER_ID aus einer lokalen `.env`
# fuer die gesamte Suite unsichtbar. Reuse der bestehenden Funktion, kein neuer Mechanismus.
# Bestehendes Prozess-Env gewinnt IMMER (kein Override, s. Doku dort); fehlende .env = no-op.
_server._lade_env_dateien(_ROOT)

_REAL_AUDIT_PFAD = os.path.join(_audit.AUDIT_DIR, "audit.jsonl")


@pytest.fixture(autouse=True)
def _kein_schreiben_ins_echte_audit_log(request):
    """Miss die Groesse der echten Datei vor/nach JEDEM Test. Wenn sie waechst,
    hat der Test — direkt oder indirekt — ins echte AUDIT_DIR geschrieben,
    statt es auf tmp_path zu patchen."""
    vorher = os.path.getsize(_REAL_AUDIT_PFAD) if os.path.exists(_REAL_AUDIT_PFAD) else 0
    yield
    nachher = os.path.getsize(_REAL_AUDIT_PFAD) if os.path.exists(_REAL_AUDIT_PFAD) else 0
    assert nachher == vorher, (
        f"{request.node.nodeid} hat ins ECHTE audit.jsonl geschrieben "
        f"({_REAL_AUDIT_PFAD}): {vorher} -> {nachher} Bytes. "
        'Fix: monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path)) im Test/Fixture setzen.'
    )
