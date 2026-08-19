"""P1.6 Audit-Log — Append-only JSON-Lines, niemals delete/update.

Einträge: login, logout, fall_angelegt, zugriff_verweigert.
Keine PII in Detail-Feldern (user_id = system-interner Username, kein Klarname/Email).
"""
from __future__ import annotations

import json
import os
from datetime import datetime, timezone

# Default: neben den Fall-Dateien (gleiche Festplatten-Partition, kein extra Mount) — und die
# sind am 2026-08-19 aus dem Projektverzeichnis nach $XDG_DATA_HOME/taxgraph gezogen. Der
# Gleichlauf ist Absicht und nicht bloss Bequemlichkeit: das Protokoll führt user_id, fall_id
# und Aktion, also wer wann welche Steuererklärung bearbeitet hat. Es gehört zu den Falldaten,
# nicht ins Repository — und `make backup` sichert beides in einem Zug, weil es zusammenliegt.
#
# Der Pfad wird über dieselbe Funktion bestimmt statt hier zweitgeschrieben: zwei Stellen, die
# denselben Ort meinen, laufen auseinander (die Klasse, die in diesem Projekt schon mehrfach
# Geld gekostet hat). Der Import ist lokal, weil produkt/haut nicht immer im sys.path liegt,
# wenn der Store allein benutzt wird — dann greift der ausdrückliche Rückfall.
def _standard_dir() -> str:
    try:
        import sys
        _h = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "haut")
        if _h not in sys.path:
            sys.path.insert(0, _h)
        from api_constants import FAELLE
        return FAELLE
    except Exception:                       # noqa: BLE001 — Store ohne Haut ist ein gültiger Fall
        xdg = os.environ.get("XDG_DATA_HOME", "").strip()
        basis = os.path.expanduser(xdg) if xdg else os.path.join(
            os.path.expanduser("~"), ".local", "share")
        return os.path.join(basis, "taxgraph", "faelle")


AUDIT_DIR = os.environ.get("TAXGRAPH_AUDIT_DIR") or _standard_dir()


def _audit_pfad() -> str:
    return os.path.join(AUDIT_DIR, "audit.jsonl")


def append(user_id: str | None, action: str, fall_id: str | None = None,
           detail: str | None = None) -> None:
    """Hängt EINEN Audit-Eintrag an (append-only, immutable).

    user_id: Username (system-intern, kein Klarname/Email). None → "unbekannt".
    action: login | logout | fall_angelegt | zugriff_verweigert | llm_call.
    fall_id: Optional — betroffener Fall.
    detail: Optional — z.B. Grund der Verweigerung. KEINE PII.
    """
    entry = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "user_id": user_id or "unbekannt",
        "action": action,
        "fall_id": fall_id,
        "detail": detail,
    }
    pfad = _audit_pfad()
    d = os.path.dirname(pfad) or "."
    os.makedirs(d, exist_ok=True)
    line = json.dumps(entry, ensure_ascii=False, sort_keys=True) + "\n"
    # O_APPEND — kein read/write/truncate. Über os.open statt open("a"), um den Modus bei
    # NEUANLAGE festzulegen: sonst erbt das Protokoll die umask (gemessen 0644, Audit
    # 2026-08-16 sec-users-json-world-readable nennt dieselbe Stelle). Es führt Nutzer-Kennung,
    # Fall-Kennung und Aktion — wer eine Steuererklärung bearbeitet und wann. Bei einer
    # bestehenden Datei bleibt ihr Modus unangetastet; das ist Absicht, ein Protokoll wird nicht
    # unterwegs umgeschrieben.
    fd = os.open(pfad, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
    with os.fdopen(fd, "a", encoding="utf-8") as f:
        f.write(line)
        f.flush()
        os.fsync(f.fileno())


def lies() -> list[dict]:
    """Liest alle Audit-Einträge (für Admin/Session-Overview)."""
    pfad = _audit_pfad()
    if not os.path.exists(pfad):
        return []
    with open(pfad, encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]
