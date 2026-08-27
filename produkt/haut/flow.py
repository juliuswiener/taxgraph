"""Der ganze Fluss in EINER Datei — was gefragt wurde, was geantwortet, was die KI dazwischen tat.

ANLASS (Julius, 2026-08-27, nach einem Live-Durchgang mit neun Befunden): „ich will so ein log wo
der ganze flow nachvollziehbar ist."

Vorher gab es drei Bruchstücke und keinen Fluss:

  * `audit.jsonl` — nur Metadaten, absichtlich (tests/test_pii_filter.py erzwingt das). Man sieht,
    DASS ein Modellaufruf lief, nicht was er las.
  * `ki_debug.jsonl` — der Wortlaut der drei Modellstufen, aber nur die. Der Fragebogen, in dem
    der Nutzer die meiste Zeit verbringt, kam darin nicht vor.
  * die Ereignisse im Fall selbst — die ANTWORTEN, ohne die Fragen und ohne ihre Reihenfolge.

Genau das Fehlende steht dazwischen: WELCHE Frage lag vor, an welcher Stelle, und was hat die
Antwort davor abgeschaltet. Ohne das war jede Diagnose zur Reihenfolge Rekonstruktion aus dem
Fall — und die trägt nur, solange man die Reihenfolge schon kennt.

ABGESCHALTET, SOLANGE NICHTS GESETZT IST. Hier steht Klartext: Namen, Beträge, Kontonummern, der
Satz des Nutzers. Das ist kein Nebeneffekt, sondern der Zweck — ein Fluss ohne Werte erklärt
nichts. Deshalb: nur mit `TAXGRAPH_FLOW=1` (oder dem älteren `TAXGRAPH_KI_DEBUG=1`, das bis
2026-08-27 nur die Modellstufen schaltete), Datei mit 0600, neben dem Audit.

Die Umgebungsvariable wird bei JEDEM Aufruf gelesen, nicht beim Import: ein Test, der sie setzt
oder wegnimmt, wirkt sofort, und niemand muss wissen, wann dieses Modul zufällig geladen wurde.
"""
from __future__ import annotations

import json
import os
from datetime import datetime, timezone

import audit

DATEI = "flow.jsonl"

# Welcher Fall gerade bearbeitet wird. Gesetzt von api.chat vor dem Modellaufruf, damit die drei
# KI-Stufen im selben Strang landen wie Fragen und Antworten — `_llm_dialog` bekommt den Fall
# nicht übergeben, und ihm dafür einen Parameter anzuhängen bräche jede Test-Attrappe, die die
# Funktion ersetzt (mehrere Dateien tun das mit fester Signatur).
#
# Eine Modul-Variable ist hier sicher aus demselben Grund wie bei `api_auth._AUTH_USER`: der
# Server ist einfädig (s. make_server), es gibt zu jedem Zeitpunkt genau einen Request.
AKTUELLER_FALL: str | None = None


def an() -> bool:
    """Läuft der Mitschnitt? Zur Aufrufzeit gelesen, s. Modulkopf."""
    return (os.environ.get("TAXGRAPH_FLOW", "").strip() == "1"
            or os.environ.get("TAXGRAPH_KI_DEBUG", "").strip() == "1")


def schreibe(fall_id: str | None, art: str, inhalt) -> None:
    """Eine Zeile. Wirft nie — ein Protokoll darf den Vorgang nicht mitreissen, den es beschreibt.

    DIE SORTEN, und warum jede gebraucht wird:

    `fragen`      Was als Nächstes vorlag, und wie viele Fragen noch ausstehen. Die Köpfe
                  hintereinandergelegt ergeben genau die Reihenfolge, die der Nutzer erlebt hat —
                  bisher war die nur aus den Antworten zu erraten, und das trägt nur, wenn man
                  sie schon kennt.
    `antwort`     Was geschrieben wurde, und ÜBER WELCHEN BILDSCHIRM: `weg` ist das zweite Signal
                  (`klick@…` Fragebogen, `hold@…` bestätigter KI-Vorschlag, `rueckfrage@…`
                  Assistent, `verstanden@…` Bestätigungsliste). Genau an dieser Spalte liess sich
                  am 2026-08-27 zeigen, dass der Assistent die Instanz-Achse nicht kennt:
                  `rueckfrage@kind_vorname` schrieb EIN Ereignis, `klick@kind_geburtsdatum`
                  daneben zwei.
    `abgewiesen`  Eine fail-closed abgewiesene Antwort. Für den Nutzer ist sie ein Ereignis („da
                  stand ein Banner"), im Fall hinterlässt sie nichts — sie fehlte also genau
                  dort, wo man sie sucht.
    `nutzertext`  Der Satz, den der Nutzer in den Berater geschrieben hat.
    `ki`          Die drei Modellstufen im Wortlaut (aus api_llm).
    `ergebnis`    Grund, Zahl, und WIE VIELE offene Felder benannt wurden. Ein Grund ohne ein
                  einziges benanntes Feld ist der Zustand, in dem der Nutzer „noch offen" liest
                  und nicht erfährt, woran es liegt (Julius' neunter Befund vom 2026-08-27).

    Ablage über `audit.AUDIT_DIR`, zur AUFRUFZEIT gelesen: dieselbe Wegbeschreibung wie beim Audit,
    damit Tests, die die Ablage umlenken, diese Datei mitnehmen. Ein `from audit import AUDIT_DIR`
    bände den Wert statt des Namens und liefe an jeder Umlenkung vorbei.
    """
    if not an():
        return
    try:
        pfad = os.path.join(audit.AUDIT_DIR, DATEI)
        zeile = json.dumps({"ts": datetime.now(timezone.utc).isoformat(),
                            "fall": fall_id or AKTUELLER_FALL, "art": art, "inhalt": inhalt},
                           ensure_ascii=False, default=str)
        os.makedirs(os.path.dirname(pfad) or ".", exist_ok=True)
        # 0600 beim Anlegen; eine bestehende Datei bleibt unangetastet (dieselbe Linie wie
        # audit.py: „ein Protokoll wird nicht unterwegs umgeschrieben").
        fd = os.open(pfad, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
        with os.fdopen(fd, "a", encoding="utf-8") as f:
            f.write(zeile + "\n")
    except Exception:
        pass


# Was die OBERFLÄCHE melden darf. Eine feste Liste, kein freies Feld: der Endpunkt schreibt in
# eine Datei, und ein Client, der sich die Sorte selbst ausdenkt, könnte den Mitschnitt beliebig
# füllen. Was hier nicht steht, wird abgewiesen.
#
# ABSICHTLICH KURZ. Julius, 2026-08-27: „kannst du nicht rekonstruieren wie die checkliste aussah?
# und die nachfragen?" — er hat recht, und es ist nachgemessen: jede Antwort trägt in `signal_2`
# den Bildschirm (`screening@`, `rueckfrage@`, `verstanden@`, `hold@`, `klick@`), die Bindung
# liefert Fragetext und `frage_invertiert` (also das Kreuz hinter dem gespeicherten Gegenteil),
# und der Wortlaut der Nachfragen steht in der KI-Stufe 3. Ankreuzliste und beantwortete
# Nachfragen sind damit vollständig rekonstruierbar — dafür braucht es keine Meldung.
#
# Hier steht deshalb nur, was KEINE Antwort hinterlässt und aus dem Fall folglich nicht zu holen
# ist. Jeder Eintrag kostet eine Anfrage auf einem einfädigen Server; das ist der Preis, und er
# lohnt nur für das wirklich Unsichtbare.
UI_ARTEN = frozenset({
    "weg_gewaehlt",          # Fragebogen oder erst KI — steht nirgends im Fall
    "nachfragen_gestartet",  # welche Nachfragen wirklich kamen und welche vorher entfielen:
                             # eine entfallene und eine übersprungene sehen im Fall gleich aus
    "nachfrage_spaeter",     # „Später beantworten" — schreibt bewusst nichts (kein Merker)
    "pruefliste_aendern",    # „Ändern" statt „Stimmt": die Korrektur kommt später als `klick@`
                             # und ist dann von einer gewöhnlichen Antwort nicht zu unterscheiden
    "pruefliste_weiter",     # die Liste verlassen — mit welchen Zeilen unbestätigt
})

# Ein Client-Beitrag ist Fremdtext. Er wird gekappt, nicht geprüft: ein Mitschnitt soll zeigen,
# was ankam — aber er ist eine Datei auf der Platte und darf nicht beliebig gross werden.
MAX_ZEICHEN = 4000


def gekappt(inhalt, grenze: int = MAX_ZEICHEN):
    """Auf `grenze` Zeichen kürzen, und das SAGEN — stillschweigend gekürzt sähe im Mitschnitt aus
    wie „mehr war da nicht", und das ist die eine Aussage, die ein Protokoll nie machen darf."""
    roh = json.dumps(inhalt, ensure_ascii=False, default=str)
    if len(roh) <= grenze:
        return inhalt
    return {"gekappt_bei": grenze, "urspruengliche_zeichen": len(roh), "anfang": roh[:grenze]}


def ergebnis_notiert(fall_id: str, obj: dict) -> None:
    """Der Ausgang von /ergebnis. `offen_anzahl` ist die eigentliche Aussage: ein Grund OHNE ein
    einziges benanntes Feld ist der Zustand, in dem der Nutzer „noch offen" liest und nicht
    erfährt, woran es liegt — Julius' neunter Befund vom 2026-08-27."""
    schreibe(fall_id, "ergebnis", {"grund": obj.get("grund"), "zahl_cent": obj.get("zahl_cent"),
                                   "offen_anzahl": len(obj.get("offen") or []),
                                   "offen": (obj.get("offen") or [])[:12]})


def melde_ui(fall_id: str, body: dict) -> tuple[int, dict]:
    """Was die OBERFLÄCHE gezeigt hat, hinter POST /fall/<id>/flow.

    Julius, 2026-08-27: „du hast in dem verlauf nicht die nachfragen, ki überprüfungsfragen und
    die checkliste drin!!!" Der Grund war strukturell: das sind Bildschirme im Browser, und der
    Server sieht nur die fertigen `/event`-Aufrufe. Auf die Rückfrage „kannst du nicht
    rekonstruieren wie die checkliste aussah? und die nachfragen?" wurde nachgemessen — man kann,
    fast vollständig. Was hier ankommt, ist deshalb nur der Rest (s. UI_ARTEN).

    Wirft ValueError bei einer nicht vorgesehenen Sorte; die Hülle in api.py macht daraus 400.
    Abweisen statt still verwerfen: ein Client, der sich die Sorte selbst ausdenken dürfte, könnte
    die Datei beliebig füllen, und ein stilles Verwerfen sähe im Mitschnitt aus wie „ist nicht
    passiert".
    """
    if not an():
        return 200, {"mitgeschrieben": False}
    art = body.get("art")
    if art not in UI_ARTEN:
        raise ValueError(f"art muss eines von {sorted(UI_ARTEN)} sein")
    schreibe(fall_id, art, gekappt(body.get("inhalt")))
    return 200, {"mitgeschrieben": True}


def kopf_der_queue(fragen: list[dict], wie_viele: int = 6) -> list[dict]:
    """Was der Nutzer als Nächstes SIEHT — nicht die ganze Queue.

    Die volle Liste sind gut 300 Fragen und wird nach jeder Antwort neu geholt; sie einzeln
    mitzuschreiben ergäbe je Durchgang ein paar Megabyte, in denen der Fluss untergeht. Der Kopf
    reicht: hintereinandergelegt ergeben die Köpfe genau die Reihenfolge, die der Nutzer erlebt
    hat, und die Gesamtzahl daneben zeigt, wie der Rest schrumpft.
    """
    return [{"feld_id": q.get("feld_id"),
             "frage": (q.get("fragetext_laie") or "")[:90],
             "instanzen": q.get("instanz_anzahl")}
            for q in fragen[:wie_viele]]
