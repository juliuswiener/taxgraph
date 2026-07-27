"""P1.6 Audit-Log — Append-only JSON-Lines, niemals delete/update.

Einträge: login, logout, fall_angelegt, zugriff_verweigert.
Keine PII in Detail-Feldern (user_id = system-interner Username, kein Klarname/Email).
"""
from __future__ import annotations

import json
import os
from datetime import datetime, timezone

# Default: neben den Fall-Dateien (gleiche Festplatten-Partition, kein extra Mount).
_HIER = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
AUDIT_DIR = os.environ.get("TAXGRAPH_AUDIT_DIR") or os.path.join(_HIER, "haut", "faelle")


def _audit_pfad() -> str:
    return os.path.join(AUDIT_DIR, "audit.jsonl")


def append(user_id: str, action: str, fall_id: str | None = None,
           detail: str | None = None) -> None:
    """Hängt EINEN Audit-Eintrag an (append-only, immutable).

    user_id: Username (system-intern, kein Klarname/Email).
    action: login | logout | fall_angelegt | zugriff_verweigert.
    fall_id: Optional — betroffener Fall.
    detail: Optional — z.B. Grund der Verweigerung.
    """
    entry = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "user_id": user_id,
        "action": action,
        "fall_id": fall_id,
        "detail": detail,
    }
    pfad = _audit_pfad()
    d = os.path.dirname(pfad) or "."
    os.makedirs(d, exist_ok=True)
    line = json.dumps(entry, ensure_ascii=False, sort_keys=True) + "\n"
    # O_APPEND-artig hinten anfügen — kein read/write/truncate.
    with open(pfad, "a", encoding="utf-8") as f:
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
