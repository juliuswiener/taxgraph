"""P1.6 Audit-Log — Append-only JSON-Lines, niemals delete/update.

Einträge: login, logout, login_fehlgeschlagen, register, fall_angelegt, zugriff_verweigert.
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
def _fall_verzeichnis() -> str:
    """Das Fallverzeichnis, wie es JETZT gilt — über die Fassade `api`, nicht über
    `api_constants`.

    Der Unterschied ist die halbe Isolierung, die am 2026-10-01 1145 Zeilen ins
    Nutzerprotokoll geschrieben hat: `api` bindet `FAELLE` per `from api_constants import *`
    als EIGENEN Namen. Wer die Fallakten umlenkt, schreibt `api.FAELLE = …` — und
    `api_constants.FAELLE` bleibt stehen. Wer hier `api_constants` läse, folgte der Umlenkung
    nicht und schriebe das Protokoll weiter neben die echten Fälle, während die Akten schon
    im Tempverzeichnis liegen.
    """
    try:
        import sys
        _h = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "haut")
        if _h not in sys.path:
            sys.path.insert(0, _h)
        import api
        return api.FAELLE
    except Exception:                       # noqa: BLE001 — Store ohne Haut ist ein gültiger Fall
        # EINE Quelle statt einer zweiten Implementierung: zuerst `api_constants._daten_wurzel()`,
        # dieselbe Funktion, nach der die Akten gehen. Gemessen 2026-10-01: ohne `$TAXGRAPH_DATEN`
        # im Rueckfall lieferte dieser Zweig `~/.local/share/taxgraph/faelle` — den ECHTEN
        # Nutzerpfad, waehrend der Aufrufer sich in `/tmp` waehnte (e97f8f5 fand dasselbe).
        try:
            import api_constants
            return os.path.join(api_constants._daten_wurzel(), "faelle")
        except Exception:                   # noqa: BLE001 — auch die Haut fehlt: letzter Rueckfall
            # Nur wenn `produkt/haut` ganz fehlt, wird die Wurzel HIER gebaut — dann in derselben
            # Reihenfolge wie `_daten_wurzel()` (eigene Variable, dann XDG, dann ~). Ein Rueckfall,
            # der `TAXGRAPH_DATEN` nicht kennt, faellt genau dann auf, wenn jemand sie setzt.
            # ponytail: zweite Fassung der Reihenfolge, nur fuer den Fall ohne Haut; der Test
            # test_audit_letzter_rueckfall_kennt_taxgraph_daten haelt sie an der ersten fest.
            eigen = os.environ.get("TAXGRAPH_DATEN", "").strip()
            if eigen:
                return os.path.join(os.path.expanduser(eigen), "faelle")
            xdg = os.environ.get("XDG_DATA_HOME", "").strip()
            basis = os.path.expanduser(xdg) if xdg else os.path.join(
                os.path.expanduser("~"), ".local", "share")
            return os.path.join(basis, "taxgraph", "faelle")


def _standard_dir() -> str:
    """Die Ablage, wie sie ohne ausdrückliche Umlenkung gilt. Zur AUFRUFZEIT bestimmt."""
    return _fall_verzeichnis()


# Beim Import festgehalten, damit `_ablage()` erkennt, ob jemand `AUDIT_DIR` absichtlich
# umgebogen hat (Tests, Werkzeuge) — und nicht versehentlich den Importwert für eine Umlenkung
# hält. Zwei Namen für eine Sache waren der Fehler; zwei Werte desselben Namens wären der
# nächste, deshalb vergleicht `_ablage()` gegen genau diese eine Zahl.
_AUDIT_DIR_IMPORT = os.environ.get("TAXGRAPH_AUDIT_DIR") or _standard_dir()

# Bleibt ein gewöhnliches, beschreibbares Modul-Attribut: die Tests und `tools/parity`
# lenken die Ablage so um, und `monkeypatch.setattr` stellt den Importwert wieder her.
AUDIT_DIR = _AUDIT_DIR_IMPORT


def _ablage() -> str:
    """Das Verzeichnis, in das das Protokoll JETZT gehört.

    Reihenfolge — jede Stufe gewinnt nur, wenn die vorige nicht greift:
      1. `AUDIT_DIR` wurde ausdrücklich umgebogen (Tests, Werkzeuge). Der Vergleich gegen
         `_AUDIT_DIR_IMPORT` ist nötig, weil `monkeypatch.setattr` beim Zurückstellen genau
         diesen Wert schreibt: ein Vergleich auf „gesetzt?" wäre danach immer wahr und die
         Ablage bliebe für den Rest des Prozesses eingefroren.
      2. `$TAXGRAPH_AUDIT_DIR` — zur Aufrufzeit gelesen, nicht beim Import.
      3. Sonst: das Fallverzeichnis, wie es jetzt steht. Damit biegt EIN Griff am
         Datenverzeichnis das Protokoll mit um.
    """
    if AUDIT_DIR != _AUDIT_DIR_IMPORT:
        return AUDIT_DIR
    eigen = os.environ.get("TAXGRAPH_AUDIT_DIR", "").strip()
    if eigen:
        return os.path.expanduser(eigen)
    return _standard_dir()


def _audit_pfad() -> str:
    return os.path.join(_ablage(), "audit.jsonl")


def append(user_id: str | None, action: str, fall_id: str | None = None,
           detail: str | None = None) -> None:
    """Hängt EINEN Audit-Eintrag an (append-only, immutable).

    user_id: Username (system-intern, kein Klarname/Email). None → "unbekannt".
    action: login | logout | login_fehlgeschlagen | register | fall_angelegt | zugriff_verweigert | llm_call.
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
