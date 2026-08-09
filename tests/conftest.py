"""Gate: kein Test darf ins ECHTE audit.jsonl schreiben.

Grund: 32 Testdateien haben genau das versehentlich getan (Audit-Log auf 582 MB
angewachsen, s. BACKLOG.yaml "audit-jsonl-wucherung"). Ein Grep nach dem Patch-
Muster (`monkeypatch.setattr(audit, "AUDIT_DIR", ...)`) faengt die Faelle NICHT,
die den Fix noetig machten: unconditionaler audit.append-Aufruf ohne jeden Patch,
eine Schwesterklasse in derselben Datei ohne Patch, eine von zwei Fixtures in
derselben Datei ohne Patch. Deshalb misst dieses Gate den echten Schreibpfad
zur Laufzeit statt ein Namensmuster.

Fruehere Fassung mass die Dateigroesse der echten audit.jsonl vorher/nachher
(os.path.getsize). Im Parallelbetrieb (mehrere Worker/Skripte im selben
Checkout, dieselbe Datei) beschuldigt das einen unbeteiligten Test, sobald ein
FREMDER Prozess in derselben Sekunde in dieselbe echte Datei schreibt — die
Groesse ist geteilter Zustand, kein Signal ueber DIESEN Testprozess (Vorfall
2026-08-09 23:22, test_bindungstabelle.py faelschlich beschuldigt).

Jetzt: audit.append wird pro Test umwickelt, jeder Aufruf wird am tatsaechlich
aufgeloesten Pfad geprueft (kein `from audit import append` im Repo — ein
Modul-Attribut-Patch faengt jeden Aufrufer, egal aus welcher Datei/Klasse/
Fixture er kommt, exakt dieselbe Faellklasse wie beim Namensmuster-Grep oben).
Sieht nur, was DIESER Prozess in DIESEM Test tatsaechlich aufruft — kein
Datei-I/O, kein geteilter Zustand, kein Wettlauf mit fremden Schreibern.
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

_REAL_AUDIT_PFAD = os.path.abspath(os.path.join(_audit.AUDIT_DIR, "audit.jsonl"))


@pytest.fixture(autouse=True)
def _kein_schreiben_ins_echte_audit_log(request, monkeypatch):
    """Umwickelt audit.append fuer JEDEN Test und prueft bei jedem Aufruf den
    zu diesem Zeitpunkt aufgeloesten Pfad — direkt oder indirekt ueber Produktcode.
    Prozesslokal: sieht nur eigene Aufrufe, nicht das Wachstum der geteilten Datei
    (s. Docstring oben, Grund fuer den Wechsel weg von Dateigroesse).

    Ein Treffer wird GEBLOCKT statt durchgereicht (kein Aufruf von echtes_append):
    die echte Datei bleibt so auch bei einem Test, der es versucht, unangetastet.
    Der Verstoss wird erst NACH yield als assert gemeldet, nicht per raise im
    Wrapper selbst — server.py:165 faengt Exceptions aus dem Request-Dispatch
    breit ab ("nie eine nackte Exception nach aussen lecken"); ein raise dort
    wuerde als generisches 500 verschluckt statt den Test klar rot zu machen.
    """
    treffer: list[str] = []
    echtes_append = _audit.append

    def _wache(*args, **kwargs):
        pfad = os.path.abspath(_audit._audit_pfad())
        if pfad == _REAL_AUDIT_PFAD:
            treffer.append(f"pfad={pfad!r} args={args!r} kwargs={kwargs!r}")
            return None  # geblockt — echtes_append() NICHT aufgerufen, Datei unangetastet
        return echtes_append(*args, **kwargs)

    monkeypatch.setattr(_audit, "append", _wache)
    yield
    assert not treffer, (
        f"{request.node.nodeid} hat versucht, ins ECHTE audit.jsonl zu schreiben "
        f"({_REAL_AUDIT_PFAD}): " + "; ".join(treffer) + ". Schreibvorgang wurde geblockt, "
        'Datei ist unangetastet. Fix: monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path)) '
        "im Test/Fixture setzen."
    )
