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

Zusatzangaben (`**meta`) sind auf eine POSITIVLISTE erlaubter Namen beschränkt
(`_ERLAUBTE_META`) UND auf `int`, `bool` und `None` als Werte — Anzahlen und Längen, wie in
api_llm.py:292. Ein Betrag ist zwar auch eine Zahl (`betrag_cent=4500000` wäre ein gültiges
`int`) — aber ein Name, der nicht auf der Liste steht, kommt gar nicht erst bis zur
Typprüfung. Eine Negativliste auf NAMEN würde das nächste erfundene Feld vergessen, eine
Positivliste nicht: dieselbe Bauart, die audit.py für seine Metadaten seit jeher fährt (feste,
benannte Felder statt freier `**kwargs`), dieselbe Erfahrung wie bei der conftest-Wache fürs
Audit-Log. Ein String unter erlaubtem Namen wird nicht geschrieben, sondern durch `<str>`
ersetzt: Text ist die Form, in der Nutzdaten reisen.

`fall_id` ist ein eigener Parameter, aber NICHT von der Prüfung ausgenommen: server.py
übernimmt ihn ungeprüft aus der URL (`treffer.groupdict().get("id")`) und kann damit zur
Laufzeit tragen, was dort stand. Ein Klarname scheitert dabei schon am Routing (Leerzeichen
sind in der dortigen Zeichenklasse nicht erlaubt, `api_constants._FALL_RE`) — die tatsächliche
Gefahr ist eine REIN ZIFFERNFÖRMIGE PII (Steuer-ID, IBAN ohne Trenner), die dieselbe
Zeichenklasse erfüllt wie eine echte Kennung. `pii_filter.filtere()` bleibt dafür UNANGETASTET
und sucht weiterhin Teilstrings in Fließtext (der LLM-Pfad braucht genau das) — eine Fallkennung
ist aber kein Fließtext, sondern EIN Bezeichner, und eine Teilstring-Suche sperrte damit JEDE
echte Kennung: die einzige tatsächliche Erzeugung im Produkt (`app.js:139`, `"demo-" +
Date.now()`) hängt einen 13-stelligen Zeitstempel an und enthält damit immer eine Ziffernfolge,
die `_KONTONUMMER` als Teilstring träfe. `_sicherer_fall_id` prüft deshalb VERANKERT
(`fullmatch`, nicht `subn`) gegen dieselben Muster (`pii_filter._KATEGORIEN`): nicht „steht
irgendwo eine Ziffernfolge drin", sondern „ist die Kennung ALS GANZES eine IBAN/Steuer-ID/
Kontonummer". Benannte Restlücke: eine Kennung, die einen Präfix ohne Trenner direkt mit einer
PII-Ziffernfolge verbindet (`kunde12345678901`), ist als Ganzes keine der drei Formen und käme
unerkannt durch — kein heutiges Erzeugungsschema tut das. Ist `pii_filter` nicht importierbar
(Store ohne Haut, wie bei `AUDIT_DIR`), wird `fall_id` fail-closed verworfen, aber SICHTBAR
markiert statt als `null` zu erscheinen — sonst sähe ein leeres Feld wie ein anderer Fehler aus.

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


def _lade_pii_filter():
    """Verzögert geladen, exakt wie audit._standard_dir() das mit api_constants tut:
    produkt/haut liegt nicht immer im sys.path, wenn der Store allein benutzt wird.
    Fehlschlag -> (None, None); der Aufrufer (_sicherer_fall_id) behandelt das fail-closed."""
    try:
        import sys
        _h = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "haut")
        if _h not in sys.path:
            sys.path.insert(0, _h)
        import pii_filter
        from api_constants import _FALL_RE
        return pii_filter, _FALL_RE
    except Exception:                       # noqa: BLE001 — Store ohne Haut ist ein gültiger Fall
        return None, None


_PII_FILTER, _FALL_RE = _lade_pii_filter()

# Sichtbare Platzhalter statt `null`: ein leeres fall_id-Feld ohne Hinweis sähe wie ein
# anderer Fehler aus (kein fall_id übergeben) statt wie das, was es ist (gesperrt).
_FALL_ID_KEIN_FILTER = "<gesperrt:pii_filter_fehlt>"


def _sicherer_fall_id(fall_id):
    """`fall_id` genauso wenig blind vertrauen wie `ort=` (AST-Gate, s. unten in
    tests/test_fehler_log.py). server.py übernimmt ihn ungeprüft aus der URL
    (`treffer.groupdict().get("id")`) — die Zeichenklasse, die dort für eine gültige Route
    ohnehin gelten muss (`api_constants._FALL_RE`), lässt eine REIN ZIFFERNFÖRMIGE PII
    (Steuer-ID, IBAN ohne Trenner) genauso durch wie eine echte Kennung; ein Klarname
    scheitert dagegen schon am Routing (kein Leerzeichen in der Zeichenklasse).

    NICHT `pii_filter.filtere()` (Teilstring-Suche in Fließtext, richtig für den LLM-Pfad, aber
    hier sperrte sie JEDE echte Kennung, s. Moduldocstring) — sondern VERANKERT (`fullmatch`)
    gegen dieselben Muster (`pii_filter._KATEGORIEN`): die Kennung ALS GANZES muss eine der
    Formen sein, nicht bloß eine enthalten."""
    if fall_id is None:
        return None
    if not isinstance(fall_id, str):
        return "<gesperrt:typ>"
    if _PII_FILTER is None or _FALL_RE is None:
        return _FALL_ID_KEIN_FILTER
    if not _FALL_RE.fullmatch(fall_id):
        return "<gesperrt:form>"
    treffer = sorted(kategorie for kategorie, muster in _PII_FILTER._KATEGORIEN
                     if muster.fullmatch(fall_id))
    if treffer:
        return f"<gesperrt:{','.join(treffer)}>"
    return fall_id


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


# Positivliste erlaubter `meta`-Namen — dieselbe Bauart wie audit.append (feste, benannte
# Felder statt freier **kwargs). Ein Name, der hier fehlt, kommt gar nicht erst bis zur
# Typprüfung: eine Negativliste auf NAMEN (verboten: "*betrag*", "*euro*", …) würde das
# nächste erfundene Feld vergessen.
_ERLAUBTE_META = frozenset({"anzahl", "laenge", "versuche", "geglueckt"})


def _sicher(name: str, wert):
    """Name auf der Positivliste (_ERLAUBTE_META) UND int/bool/None als Wert durchlassen,
    alles andere durch einen Platzhalter ersetzen. Ein Betrag ist zwar auch eine Zahl — aber
    ein Name ausserhalb der Liste kommt gar nicht erst bis zur Typprüfung, und Text ist die
    Form, in der Nutzdaten reisen."""
    if name not in _ERLAUBTE_META:
        return "<gesperrt>"
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
    fall_id: interne Fall-Kennung, wie in audit.append — wird selbst geprüft
             (s. _sicherer_fall_id), nicht blind übernommen.
    meta:    Anzahlen und Wahrheitswerte UNTER erlaubtem Namen (Positivliste, s. _sicher).
    """
    eintrag = {
        "ts": datetime.now(timezone.utc).isoformat(),
        "stufe": logging.getLevelName(stufe),
        "ort": ort,
        "typ": type(exc).__name__,
        "quelle": _ort_aus_traceback(exc),
        "fall_id": _sicherer_fall_id(fall_id),
    }
    for k, v in meta.items():
        eintrag[k] = _sicher(k, v)
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
