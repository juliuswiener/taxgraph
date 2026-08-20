"""Fehler-Protokoll — was ein except-Block fängt, bleibt danach rekonstruierbar.

Bis 2026-08-20 gab es in produkt/ keine einzige Protokollierung (`git grep -c 'logging\\|logger'
-- produkt/` fand nichts) bei 47 except-Blöcken. Solange nur Entwicklungsfälle im Store liegen,
ist das eine Unbequemlichkeit; sobald ein echter Steuerfall darin liegt, ist es die Frage
"warum hat der Nutzer keine Erklärung bekommen", die niemand mehr beantworten kann.

WAS HIER NICHT HINEINGEHT — und warum das die eigentliche Schwierigkeit ist:
Dieses Produkt verarbeitet Beträge, IBAN, Steuer-ID, Namen und Gesundheitsdaten (Art. 9 DSGVO).
Ein Protokoll, das eine Ausnahme mitsamt ihrer Nutzdaten schreibt, ist ein NEUER Datenabfluss
und kein Fortschritt. Der Verdacht ist hier nicht theoretisch, sondern gemessen: store.py:232
und store.py:342 werfen `ValueError(f"... {feld_id}={wert!r} ...")` — der abgewiesene Betrag
steht IM Ausnahmetext —, und server.py reicht ihn mit `f"{type(e).__name__}: {e}"` weiter.
Ein `logger.exception(e)` an einer dieser Stellen schriebe den Steuerbetrag auf die Platte.

Deshalb nimmt `protokolliere()` die Ausnahme als OBJEKT und liest daraus ausschliesslich:
  * `type(exc).__name__`      — ein Klassenname aus dem Quelltext, kein Nutzdatum
  * Datei / Zeile / Funktion  — Code-Metadaten aus dem Traceback
Es gibt in diesem Modul keinen Pfad, auf dem `str(exc)`, `exc.args` oder der Traceback-TEXT in
die Zeile gelangen. `traceback.extract_tb` liefert je Rahmen auch `.line`, den Quelltext — der
wird bewusst nicht gelesen. Das ist der Unterschied zu `logging.exception`, dessen letzte Zeile
immer `str(exc)` ist; dieselbe Disziplin wie in produkt/store/audit.py, das seit jeher nur
Kategorien, Längen und Anzahlen führt.

Zusatzangaben (`**meta`) sind auf `int`, `bool` und `None` beschränkt — Anzahlen und Längen,
wie in api_llm.py:292. Ein String wird nicht geschrieben, sondern durch `<str>` ersetzt: Text
ist die Form, in der Nutzdaten reisen, und eine Regel, die von der Sorgfalt des nächsten
Aufrufers abhängt, hält nicht (dieselbe Erfahrung wie bei der conftest-Wache fürs Audit-Log).
`fall_id` ist davon ausgenommen und ein eigener Parameter — eine interne Kennung, die
audit.py aus demselben Grund führt.

Ablage: neben den Falldaten, über audit.AUDIT_DIR statt einer zweiten Wegbeschreibung. Zwei
Stellen, die denselben Ort meinen, laufen auseinander — die Klasse, die in diesem Projekt
schon mehrfach Geld gekostet hat. Der Zugriff erfolgt zur AUFRUFZEIT (`audit.AUDIT_DIR`, kein
`from audit import AUDIT_DIR`): ein from-Import bindet den Wert, nicht den Namen, und liefe an
jedem Test vorbei, der die Ablage umlenkt.
"""
from __future__ import annotations

import json
import logging
import os
import traceback
from datetime import datetime, timezone

import audit

# Weitergereicht, damit ein Aufrufer die Stufe benennen kann, ohne selbst `logging` zu
# importieren — der Struktur-Test (tests/test_fehler_log.py) verbietet genau das, weil ein
# direkter logging-Aufruf an der PII-Schranke dieses Moduls vorbeiginge.
FEHLER, WARNUNG, DEBUG = logging.ERROR, logging.WARNING, logging.DEBUG

_LOGGER = logging.getLogger("taxgraph.fehler")
_LOGGER.setLevel(logging.DEBUG)
_LOGGER.propagate = False       # nicht zusätzlich in eine fremde Root-Konfiguration schreiben

_ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def _pfad() -> str:
    return os.path.join(audit.AUDIT_DIR, "fehler.log")


def _handler_fuer(pfad: str) -> logging.Handler:
    """Ein FileHandler je Ziel. Die Ablage wird in Tests umgelenkt (audit.AUDIT_DIR), und ein
    einmal beim Import gebundener Handler schriebe weiter in die echte Datei des Nutzers."""
    vorhanden = [h for h in _LOGGER.handlers
                 if getattr(h, "baseFilename", None) == os.path.abspath(pfad)]
    if vorhanden:
        return vorhanden[0]
    for alt in list(_LOGGER.handlers):      # Ziel gewechselt: alten Handler schliessen
        _LOGGER.removeHandler(alt)
        alt.close()
    os.makedirs(os.path.dirname(pfad) or ".", exist_ok=True)
    # 0o600 wie beim Audit-Log: das Protokoll führt Fall-Kennung und Ausfallzeitpunkt. Ohne
    # vorheriges Anlegen erbt die Datei die umask (gemessen 0644, Audit sec-users-json-world-readable).
    if not os.path.exists(pfad):
        os.close(os.open(pfad, os.O_WRONLY | os.O_CREAT, 0o600))
    h = logging.FileHandler(pfad, encoding="utf-8")
    h.setFormatter(logging.Formatter("%(message)s"))    # die Zeile ist fertiges JSON
    _LOGGER.addHandler(h)
    return h


def _ort_aus_traceback(exc: BaseException) -> str:
    """Ursprungsort als `datei:zeile:funktion` — der INNERSTE Rahmen, dort ist der Fehler
    entstanden. Nur Code-Metadaten: `FrameSummary.line` (der Quelltext) wird nicht gelesen.
    Der Dateipfad wird auf den Repo-Anteil gekürzt, damit nicht der Benutzername mitläuft."""
    spuren = traceback.extract_tb(exc.__traceback__)
    if not spuren:
        return "unbekannt"
    f = spuren[-1]
    datei = os.path.relpath(f.filename, _ROOT) if f.filename.startswith(_ROOT) else \
        os.path.basename(f.filename)
    return f"{datei}:{f.lineno}:{f.name}"


def _sicher(wert):
    """int/bool/None durchlassen, alles Übrige durch seinen Typnamen ersetzen. Ein Betrag ist
    zwar auch eine Zahl — aber Anzahlen und Längen sind die Form, in der Metadaten hier seit
    audit.py geführt werden, und Text ist die Form, in der Nutzdaten reisen."""
    if wert is None or isinstance(wert, (int, bool)):
        return wert
    return f"<{type(wert).__name__}>"


def protokolliere(ort: str, exc: BaseException, *, stufe: int = logging.ERROR,
                  fall_id: str | None = None, **meta) -> None:
    """Schreibt EINEN Fehlereintrag (JSON-Lines, wie audit.jsonl).

    ort:     fester Bezeichner der Fangstelle aus dem Quelltext, z.B. "server.dispatch".
             Ein Literal, kein zusammengesetzter Text — sonst reist hier Nutzereingabe mit.
    exc:     die gefangene Ausnahme. Es wird NUR ihr Typ und ihr Ursprungsort gelesen,
             nie `str(exc)`.
    stufe:   logging.ERROR für einen verschluckten echten Fehler, logging.WARNING für einen
             erwarteten Ausfall, dessen GRUND sonst verloren ginge, logging.DEBUG für
             Kontrollfluss.
    fall_id: interne Fall-Kennung (kein Klarname), wie in audit.append.
    meta:    Anzahlen und Wahrheitswerte. Strings werden nicht geschrieben (s. _sicher).
    """
    eintrag = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "stufe": logging.getLevelName(stufe),
        "ort": ort,
        "typ": type(exc).__name__,
        "quelle": _ort_aus_traceback(exc),
        "fall_id": fall_id,
    }
    for k, v in meta.items():
        eintrag[k] = _sicher(v)
    pfad = _pfad()
    _handler_fuer(pfad)
    _LOGGER.log(stufe, json.dumps(eintrag, ensure_ascii=False, sort_keys=True))


def lies() -> list[dict]:
    """Alle Einträge (für Diagnose und für das Gate in tests/test_fehler_log.py)."""
    pfad = _pfad()
    if not os.path.exists(pfad):
        return []
    with open(pfad, encoding="utf-8") as f:
        return [json.loads(z) for z in f if z.strip()]
