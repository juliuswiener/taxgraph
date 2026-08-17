"""Sachverhalts-Store — der Mechanismus (LLM-frei, deterministisch, fail-closed).

DER EINE Schreibpfad: `append_event()`. Es gibt keine zweite Implementierung des Event-Schreibens.
Materialisierung: `materialisiere()` faltet den append-only Log (mit `ersetzt`-Auflösung) zu
`feld_id -> {wert, zustand, herkunft}` + einem content-adressierten `snapshot_id`.

Der Typ ist Enforcement, nicht Bitte (Julius #2): `meet_zustand`/`meet_herkunft` sind die Algebra
über dem Vorlaeufig/Bestaetigt-Typ + Herkunfts-Vektor; ein Aggregat ist nur bestaetigt, wenn ALLE
Eingaben bestaetigt sind (Meet über den Input-Kegel).

Schema: produkt/store/schema.json. Persistenz atomar (temp + os.replace), nie halb geschrieben.
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import tempfile
from datetime import datetime, timezone

# ------------------------------------------------------------------ Content-Adressierung

def canonical_json(obj) -> str:
    """Deterministische Serialisierung für die Content-Adressierung (sort_keys, kompakt)."""
    return json.dumps(obj, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


def _sha(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def event_id(event: dict) -> str:
    """sha256(canonical_json(event OHNE event_id)). Tamper-fest: jede Änderung ändert den Hash."""
    payload = {k: v for k, v in event.items() if k != "event_id"}
    return _sha(canonical_json(payload))


def snapshot_id(felder: dict) -> str:
    return _sha(canonical_json(felder))


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


# ------------------------------------------------------------------ Typ-Algebra (Julius #2)

_ZUSTAND_ORD = {"vorlaeufig": 0, "bestaetigt": 1}
_PRUEF_ORD = {"ungeprueft": 0, "plausibilisiert": 1, "orakel_bestaetigt": 2, "amtlich": 3}
_PRUEF_INV = {v: k for k, v in _PRUEF_ORD.items()}
KONFLIKT = "konflikt"


def meet_zustand(zustaende) -> str:
    """Meet über den Input-Kegel: bestaetigt nur, wenn ALLE bestaetigt sind (sonst vorlaeufig)."""
    zs = list(zustaende)
    if not zs:
        return "bestaetigt"          # leeres Aggregat = neutral (kein Input-Kegel)
    return "bestaetigt" if all(z == "bestaetigt" for z in zs) else "vorlaeufig"


def meet_herkunft(vektoren) -> dict:
    """Meet pro Achse: pruef_tiefe = Minimum (Kette); herkunft/haftung = Konflikt-Markierung bei
    Uneinigkeit (nie auto-versöhnt, Auflage-1/Split-Annunciator)."""
    vs = list(vektoren)
    if not vs:
        return {"herkunft": "berechnet", "pruef_tiefe": "amtlich", "haftung": "system"}

    def _kat(achse):
        werte = {v[achse] for v in vs}
        return next(iter(werte)) if len(werte) == 1 else KONFLIKT

    tiefe = min(_PRUEF_ORD[v["pruef_tiefe"]] for v in vs)
    return {"herkunft": _kat("herkunft"), "pruef_tiefe": _PRUEF_INV[tiefe], "haftung": _kat("haftung")}


# ------------------------------------------------------------------ Persistenz (atomar)

def leerer_store(veranlagungszeitraum: int, fall_id: str | None = None) -> dict:
    s = {"version": 1, "veranlagungszeitraum": int(veranlagungszeitraum), "events": [], "snapshots": []}
    if fall_id:
        s["fall_id"] = fall_id
    return s


def lade(pfad: str) -> dict:
    import yaml
    with open(pfad, encoding="utf-8") as f:
        return yaml.safe_load(f)


# ------------------------------------------------------------------ DER Schreibpfad

def _aktives(store: dict) -> dict:
    """feld_id -> aktuell aktives Event (nicht durch ein späteres ersetzt)."""
    ersetzt_ids = {e["ersetzt"] for e in store["events"] if e.get("ersetzt")}
    aktiv = {}
    for e in store["events"]:
        if e["event_id"] in ersetzt_ids:
            continue
        aktiv[e["feld_id"]] = e
    return aktiv


_VORSCHLAG_TYP = (("llm:", "llm"), ("import:beleg", "beleg"),
                  ("import:kontoauszug", "kontoauszug"), ("berechnet:", "maps"))
# ⚠ `berechnet:`-Präfix (nicht nur berechnet:maps): fail-safe für künftige berechnet:X-Writer — jeder
# berechnet:-Vorschlag ist katalog-restringiert (aktuell nur berechnet:maps=Entfernung existiert; ein neuer
# berechnet-Dienst würde gegen den maps-Katalog geprüft = fail-closed bis ein eigener Typ-Eintrag ergänzt wird,
# NIE exempt-Bypass). Instructor-Completeness-Note 2026-07-20.


def _vorschlag_typ(schreiber: str) -> str | None:
    """Vorschlags-Schreiber-Typ (llm|beleg|kontoauszug|maps) für den Feld-Katalog-Check, sonst None.
    import:vorjahr/mensch/import:elster → None (nicht Katalog-restringiert: eigener bestätigter Wert bzw.
    echter Kanal, kein KI/Dienst-Vorschlag über fremde Felder)."""
    for praefix, typ in _VORSCHLAG_TYP:
        if schreiber.startswith(praefix):
            return typ
    return None


# Felder, die das LLM trotz der Freigabe unten NICHT vorschlagen darf. Heute leer, und das ist
# eine bewusste Entscheidung (Julius 2026-08-14), keine Nachlässigkeit: die Sicherheit liegt im
# Zwei-Signal-Mechanismus, nicht in dieser Liste. Ein Vorschlag ist immer `vorlaeufig`, bewegt
# also keine Zahl, und der Mensch bestätigt jedes Feld einzeln per Hold-Confirm.
# Wer hier etwas einträgt, sollte begründen können, warum schon der VORSCHLAG schädlich wäre —
# das Argument gibt es (eine halluzinierte IdNr oder IBAN sieht plausibel aus und wird eher
# durchgewunken als eine falsche Zahl), es hat sich gegen den Nutzen nur nicht durchgesetzt.
LLM_NICHT_VORSCHLAGBAR: frozenset[str] = frozenset()


def lade_katalog(bindung: dict) -> dict:
    """{schreiber_typ -> frozenset(feld_id)} — welcher Vorschlags-Schreiber welches Feld setzen darf.
    bindung = DATEN, dieser Store-Check = ENFORCEMENT.

    beleg/kontoauszug/maps: DEFAULT human-only (fail-closed) über das per-Feld `vorschlagbar_von`.
    Ein Feld ohne Eintrag ist in KEINER dieser Mengen. Diese Kanäle lesen fremde Dokumente bzw.
    Dienste und sollen weiterhin nur dort schreiben, wo jemand es ausdrücklich vorgesehen hat.

    llm: ALLE askable Felder minus LLM_NICHT_VORSCHLAGBAR (Julius-Entscheid 2026-08-14: "ich denke
    alles darf das llm ausfüllen, aber der mensch muss bei JEDEM feld bestätigen"). Vorher trugen
    17 von 263 Feldern ein `vorschlagbar_von: [llm]` — nie aktiv gesperrt, sondern nie freigegeben,
    weil sie einzeln nachgetragen wurden, wenn man sie brauchte. Damit konnte die KI ausgerechnet
    die häufigsten Angaben (Bruttolohn, Arbeitstage, Entfernung) nicht übersetzen, obwohl der
    Nutzer sie im selben Satz genannt hatte.
    NICHT-askable Felder bleiben ausgeschlossen: sie werden berechnet oder aus Instanz-Summen
    gefüllt, ein Vorschlag darauf wäre kein Nutzereingabe-Ersatz, sondern eine Umgehung des Rings.

    Was die Freigabe NICHT aufweicht (alles unverändert scharf):
      - Auflage A: ein Vorschlags-Schreiber schreibt ausschließlich zustand=vorlaeufig, signal_2=None.
      - Zwei-Signal: die festgesetzte Steuer liest nur bestätigte Werte (_bescheid_fn, nur_bestaetigt).
      - Auflage B: ein Feld mit aktivem Event braucht `ersetzt` — und das ist Vorschlags-Schreibern
        seit 29e923d verwehrt. Die KI kann einen bestätigten Nutzerwert also nicht überschreiben.
    """
    katalog = {"llm": set(), "beleg": set(), "kontoauszug": set(), "maps": set()}
    for fid, b in bindung.items():
        for typ in (b.get("vorschlagbar_von") or ()):
            if typ in katalog:
                katalog[typ].add(fid)
        if b.get("askable") and fid not in LLM_NICHT_VORSCHLAGBAR:
            katalog["llm"].add(fid)
    return {k: frozenset(v) for k, v in katalog.items()}


_TYP_ORD = ("cent", "int", "bool", "enum", "datum", "text")   # gesamte Bindungs-typ-Menge (bindung/SCHEMA.md)


def _typ_konform(wert, typ: str, enum_werte) -> bool:
    """Wert↔Bindungstyp-Prüfung — DIESELBE Semantik wie tests/test_store.py:_typ_ok (bewusst gespiegelt,
    nicht importiert: Store-Produktionscode zieht keine Test-Datei). `bool` ist in Python ein `int` —
    isinstance(True, int) ist True — darum der explizite `not isinstance(wert, bool)`-Ausschluss bei
    cent/int, sonst ginge ein Häkchen als 1 Cent durch. Unbekannter/unerwarteter typ-String: True
    (durchlassen) — diese Funktion wird nur mit typ aus _TYP_ORD aufgerufen (s. Aufrufer unten)."""
    if typ in ("cent", "int"):
        return isinstance(wert, int) and not isinstance(wert, bool)
    if typ == "bool":
        return isinstance(wert, bool)
    if typ == "enum":
        return wert in (enum_werte or [])
    if typ == "datum":
        # TT.MM.JJJJ, NICHT ISO. Das ist keine Geschmacksfrage: die Bindung schreibt es vor
        # (bindung_an_gesamt.yaml:985 "amtliches ELSTER-Format, kein ISO-8601 — nichts in der
        # Pipeline konvertiert Datumswerte", beispielwert "05.05.1955"), das XSD verlangt es
        # (DatumTTpMMpJJJJBekanntBaseCType_RABE zu E0100401), und die scharfen Einreichungs-
        # laeufe erreichten mit genau diesem Format rc=0. Ein erster Anlauf prueste hier auf
        # ISO und haette damit JEDE echte Geburtsdatums-Eingabe mit 422 abgewiesen — die
        # Pruefung waere fail-closed gegen den eigenen dokumentierten Standard gewesen.
        return isinstance(wert, str) and re.match(r"^\d{2}\.\d{2}\.\d{4}$", wert) is not None
    if typ == "text":
        return isinstance(wert, str)
    return True


def _pruefe_typ_konformitaet(feld_id: str, wert, bindung: dict) -> None:
    """Auflage T (Typ-Konformität, Stille-Null-Klasse): wert muss zum Bindungstyp von feld_id passen —
    fängt genau den Fall, der den Ring bisher stumm auf 0 fallen liess (String '50000' auf einem
    typ=cent-Feld wurde ohne Fehler bestaetigt; die _c/_cent/_best-Lesehelfer in api.py machen aus
    jedem Nicht-Zahl-Wert kommentarlos 0 — keine dieser >15 Lesestellen hätte je etwas gemeldet).

    Nur aktiv, wenn ein `bindung`-Dict übergeben wird (s. append_event-Signatur) — bewusst OPTIONAL,
    nicht scharf für alle 2100+ Bestands-Testaufrufe von ST.append_event(...), die kein bindung=
    übergeben (die testen andere Mechanismen mit absichtlich minimalem Setup). Real produktiv verdrahtet
    an JEDEM Schreibpfad, der wert von AUSSEN übernimmt: api.event() (HTTP), beleg_writer, kontoauszug_
    writer, elster_writer (eDaten, § 150 Abs. 7 AO — schreibt sogar DIREKT zustand=bestaetigt, ohne
    jeden Zwischenschritt) — s. Bericht an team-lead für die Naht-(a)-vs-(b)-Abwägung.

    Repeated-Instance (base__n, s. api.py Z.2658-2665): dieselbe Enumerations-Wahrheit wie überall
    (est_mapping.parse_instanz) — LOKALER Import, denn est_mapping importiert store nur lokal
    (Z.694 dort) und ein Modul-Top-Level-Import hier würde die vielen Tests brechen, die store.py
    isoliert importieren (nur produkt/store auf sys.path, ohne produkt/mapping).

    Unbekanntes feld_id (nicht in bindung) oder Bindungseintrag ohne `typ`: durchlassen, NICHT raten
    (Team-Lead-Vorgabe Schritt 3) — dokumentiert hier, nicht stillschweigend im Code versteckt."""
    basis = feld_id
    try:
        import est_mapping as _EM
        parsed = _EM.parse_instanz(feld_id)
        if parsed:
            basis = parsed[0]
    except ImportError:
        pass   # Aufrufer ohne produkt/mapping auf sys.path (reine Store-Tests) -- kein Instanz-Feld dort

    eintrag = bindung.get(basis)
    if eintrag is None:
        return   # unbekanntes feld_id: durchlassen, nicht raten
    typ = eintrag.get("typ")
    if typ not in _TYP_ORD:
        return   # kein/unerwarteter typ-Eintrag: durchlassen, nicht raten
    if not _typ_konform(wert, typ, eintrag.get("enum_werte")):
        raise ValueError(
            f"fail-closed (Typ): {feld_id}={wert!r} passt nicht zum Bindungstyp '{typ}' — "
            "der Ring läse das sonst still als 0 (Stille-Null-Klasse).")


def append_event(store: dict, *, feld_id: str, wert, zustand: str, herkunft: dict,
                 schreiber: str, signal: dict | None = None, ersetzt: str | None = None,
                 ts: str | None = None, katalog: dict | None = None,
                 bindung: dict | None = None) -> dict:
    """Baut EIN Event, prüft fail-closed (Auflagen A+B+T), setzt den content-adressierten event_id,
    hängt es an. Gibt das Event zurück. KEINE zweite Schreib-Implementierung.

    bindung: optional — wenn gesetzt, greift Auflage T (Typ-Konformität, s. _pruefe_typ_konformitaet).
    Weggelassen bleibt der Aufruf wie bisher (Rückwärtskompatibilität zu den Bestandsaufrufen)."""
    signal = signal or {"signal_1": None, "signal_2": None}

    # Auflage A: ein llm:-Schreiber muss sich ehrlich deklarieren (kein Herkunfts-Schlupfloch).
    if schreiber.startswith("llm:"):
        if herkunft.get("herkunft") != "llm_vorschlag" or zustand != "vorlaeufig" \
                or signal.get("signal_2") is not None:
            raise ValueError(
                "fail-closed (A): llm:-Schreiber muss herkunft=llm_vorschlag, zustand=vorlaeufig, "
                "signal_2=null tragen — kein Bestätigen durch die KI.")

    # Auflage A (Beleg-Writer, symmetrisch): ein import:beleg-Schreiber ist ein Vorschlag aus einem
    # Beleg — er kann strukturell NIE direkt bestätigen; die Zwei-Signal-Bestätigung bleibt der Mensch.
    if schreiber.startswith("import:beleg"):
        if herkunft.get("herkunft") != "beleg_import" or zustand != "vorlaeufig" \
                or signal.get("signal_2") is not None:
            raise ValueError(
                "fail-closed (A): import:beleg-Schreiber muss herkunft=beleg_import, zustand=vorlaeufig, "
                "signal_2=null tragen — ein Beleg-Import bestätigt nie direkt.")

    # Auflage A (Vorjahr-Writer, symmetrisch): eine Vorjahres-Übernahme ist ein Vorschlag aus dem Vorjahr —
    # sie bewegt keine Steuer-Zahl, bis der Mensch sie im neuen VZ bestätigt/aktualisiert (K2, fail-closed).
    if schreiber.startswith("import:vorjahr"):
        if herkunft.get("herkunft") != "vorjahr" or zustand != "vorlaeufig" \
                or signal.get("signal_2") is not None:
            raise ValueError(
                "fail-closed (A): import:vorjahr-Schreiber muss herkunft=vorjahr, zustand=vorlaeufig, "
                "signal_2=null tragen — eine Vorjahres-Übernahme bestätigt nie direkt.")

    # Auflage A (Kontoauszug-Writer, symmetrisch): eine Transaktions-Klassifikation (Heuristik oder LLM) ist
    # ein VORSCHLAG — sie bewegt keine Steuer-Zahl, bis der Mensch die Transaktion neben dem Auszug bestätigt
    # (K2, Chat-Berater-Grenze: das LLM schlägt vor, setzt NIE einen Wert).
    if schreiber.startswith("import:kontoauszug"):
        if herkunft.get("herkunft") != "kontoauszug" or zustand != "vorlaeufig" \
                or signal.get("signal_2") is not None:
            raise ValueError(
                "fail-closed (A): import:kontoauszug-Schreiber muss herkunft=kontoauszug, zustand=vorlaeufig, "
                "signal_2=null tragen — eine Kontoauszug-Klassifikation bestätigt nie direkt.")

    # Auflage A (Berechnet-Writer, symmetrisch): ein berechnet:-Schreiber (z.B. berechnet:maps = Entfernung
    # aus dem Karten-Dienst) liefert einen aus PII/externem Dienst ABGELEITETEN Vorschlag — er bewegt keine
    # Steuer-Zahl, bis der Mensch ihn bestätigt (K2, defense-in-depth: der Store erzwingt vorlaeufig
    # STRUKTURELL, nicht nur der disziplinierte Endpunkt). SCHREIBER-scoped: ein engine-Schreiber mit
    # herkunft=berechnet (fertiges Rechen-Ergebnis) ist NICHT betroffen — nur berechnet:*-Vorschläge.
    if schreiber.startswith("berechnet:"):
        if herkunft.get("herkunft") != "berechnet" or zustand != "vorlaeufig" \
                or signal.get("signal_2") is not None:
            raise ValueError(
                "fail-closed (A): berechnet:-Schreiber muss herkunft=berechnet, zustand=vorlaeufig, "
                "signal_2=null tragen — ein berechneter/abgeleiteter Vorschlag bestätigt nie direkt.")

    # Auflage A (Ersetzt-Guard, gemessen 2026-08-12): ein Vorschlag darf NIE ein aktives Event ersetzen. Ohne
    # diese Sperre prüft der llm:-Zweig oben nur herkunft/zustand/signal_2, nicht `ersetzt` — ein llm:-Schreiber
    # könnte also ein BESTÄTIGTES Nutzer-Event via ersetzt= inaktiv setzen und durch einen unbestätigten
    # Vorschlag ersetzen; materialisiere() nimmt das jüngste nicht-ersetzte Event, der Vorschlag würde gewinnen —
    # ein bestätigter Wert wäre still weg (fail-open genau an der Stelle, die Auflage B eigentlich sichert;
    # Beleg: tests/test_haut_chat.py Mutation 2 gegen chat(), belegt am ungehärteten Store). Gilt für
    # llm:/import:beleg/import:kontoauszug — gemessen: kein Aufrufer übergibt für diese drei je ersetzt=
    # (beleg_writer.py, kontoauszug_writer.py). NICHT für berechnet: (maps): entfernung() (api.py, ~L2874-2879)
    # übergibt dort bewusst ersetzt=<aktives Event>, weil der Nutzer selbst "berechnen" geklickt hat (explizite
    # Aktion, kein autonomer Vorschlag) — das Ergebnis bleibt trotzdem nur vorlaeufig bis Bestätigung; einziger
    # gemessene legitime Aufrufer, der Guard bleibt also berechnet:-scoped exempt.
    if schreiber.startswith(("llm:", "import:beleg", "import:kontoauszug")) and ersetzt is not None:
        raise ValueError(
            f"fail-closed (A): {schreiber} darf kein ersetzt tragen — ein Vorschlag ersetzt nie einen "
            "bestätigten Wert; die Übernahme läuft über /event mit menschlichem signal_2.")

    # Auflage K1 (Feld-Katalog): ein Vorschlags-Schreiber (llm/beleg/kontoauszug/maps) darf NUR ein Feld
    # setzen, das für seinen Typ freigegeben ist (bindung.vorschlagbar_von) — DEFAULT human-only, fail-closed.
    # Verhindert dass KI/externer Dienst ein Wahlrecht/eine Abwesenheits-Erklärung/Identität/Allokation
    # VORSCHLÄGT (der vorlaeufig-Zwang oben schützt die Steuer-Summe; dieser Katalog schützt die FRAGE selbst).
    # import:vorjahr EXEMPT (eigener bestätigter Vorjahres-Wert, kein Fremd-Feld-Vorschlag).
    typ = _vorschlag_typ(schreiber)
    if typ is not None:
        if katalog is None:
            raise ValueError(
                f"fail-closed (Katalog): Vorschlags-Schreiber {schreiber} braucht katalog=lade_katalog(bindung).")
        if feld_id not in katalog.get(typ, frozenset()):
            raise ValueError(
                f"fail-closed (Katalog): {schreiber} darf {feld_id} nicht vorschlagen "
                f"(human-only oder nicht für Typ '{typ}' freigegeben).")

        # Auflage F2 (Magnitude-Bound, Vorschlags-Schreiber-only): ein Vorschlag (LLM/Beleg/Kontoauszug/Maps)
        # ist ein GUESS — kein legitimer Steuer-Wert (Cent-Betrag, Zähler, Jahr, GdB, %) nähert sich 100 Mio €
        # (10^10 Cent). Fängt EUR-statt-Cent-Verwechslung (100×-Fehler nach oben). Symmetrisch (abs): kein
        # 0-Floor, denn negative Verlust-Felder sind legitim. HONEST LIMIT: fängt NUR Über-Skalierung (zu
        # groß) — eine Unter-Skalierung (z.B. EUR-Betrag zu klein für Cent, aber < 10^10) bleibt unerkannt;
        # dagegen wirkt die Cent-Instruktion im Chat-Prompt (_chat_prompt), nicht dieser Bound. Deckt auch
        # numerische STRING-Werte ab (chat()/generic /event reichen wert roh durch, LLMs liefern oft JSON-
        # Strings) — nicht-numerische Strings (Enums, IBAN) bleiben unangetastet.
        _num = wert
        if isinstance(wert, str):
            try:
                _num = float(wert)
            except ValueError:
                _num = None
        if isinstance(_num, (int, float)) and not isinstance(_num, bool) and abs(_num) >= 10**10:
            raise ValueError(
                f"fail-closed (F2/Magnitude): {feld_id}={wert!r} von {schreiber} — vermuteter "
                f"Einheiten-/Skalierungsfehler (EUR statt Cent).")

    # Auflage T (Typ-Konformität, Stille-Null-Klasse): s. _pruefe_typ_konformitaet-Docstring.
    if bindung is not None:
        _pruefe_typ_konformitaet(feld_id, wert, bindung)

    # Typ-Zwang: bestaetigt braucht signal_2.
    if zustand == "bestaetigt" and not (signal.get("signal_2") or "").strip():
        raise ValueError("fail-closed: zustand=bestaetigt braucht ein signal_2 (Zwei-Signal).")

    # Auflage B: höchstens ein aktives Event je feld_id; Überschreiben nur via gültiges ersetzt.
    aktiv = _aktives(store)
    if ersetzt is None:
        if feld_id in aktiv:
            raise ValueError(f"fail-closed (B): {feld_id} hat schon ein aktives Event; "
                             f"Überschreiben braucht ersetzt={aktiv[feld_id]['event_id']}.")
    else:
        ziel = next((e for e in store["events"] if e["event_id"] == ersetzt), None)
        if ziel is None:
            raise ValueError(f"fail-closed (B): ersetzt-Ziel {ersetzt} existiert nicht.")
        if ziel["feld_id"] != feld_id:
            raise ValueError("fail-closed (B): ersetzt-Ziel gehört zu anderem feld_id.")
        if ersetzt in {e.get("ersetzt") for e in store["events"]}:
            raise ValueError("fail-closed (B): ersetzt-Ziel ist bereits ersetzt.")

    event = {"event_id": "", "ts": ts or _now(), "feld_id": feld_id, "wert": wert,
             "zustand": zustand, "herkunft": herkunft, "schreiber": schreiber,
             "signal": signal, "ersetzt": ersetzt}
    event["event_id"] = event_id(event)
    store["events"].append(event)
    return event


# ------------------------------------------------------------------ Materialisierung

def materialisiere(store: dict, bis_event: str | None = None) -> tuple[dict, str]:
    """Faltet den Log-Präfix (bis inkl. bis_event, sonst alles) zu felder + snapshot_id.

    Append-only + ersetzt-Auflösung: ein Event, das später via ersetzt überschrieben wird, zählt
    nicht; das jüngste nicht-ersetzte Event je feld_id gewinnt.
    """
    if bis_event is not None:
        praefix, gefunden = [], False
        for e in store["events"]:
            praefix.append(e)
            if e["event_id"] == bis_event:
                gefunden = True
                break
        if not gefunden:
            raise ValueError(f"bis_event {bis_event} nicht im Log.")
    else:
        praefix = list(store["events"])

    ersetzt_ids = {e["ersetzt"] for e in praefix if e.get("ersetzt")}
    felder = {}
    for e in praefix:
        if e["event_id"] in ersetzt_ids:
            continue
        felder[e["feld_id"]] = {"wert": e["wert"], "zustand": e["zustand"], "herkunft": e["herkunft"]}
    # feld_id-sortiert für stabile Content-Adresse
    felder = {k: felder[k] for k in sorted(felder)}
    return felder, snapshot_id(felder)


def erzeuge_snapshot(store: dict, *, bis_event: str | None = None, ts: str | None = None,
                     eric_befund: dict | None = None) -> dict:
    """Materialisiert + baut einen Snapshot-Eintrag (bindet ERiC-Befund an den Hash)."""
    felder, sid = materialisiere(store, bis_event)
    letztes = store["events"][-1]["event_id"] if bis_event is None else bis_event
    snap = {"snapshot_id": sid, "ts": ts or _now(), "bis_event": letztes, "felder": felder}
    if eric_befund is not None:
        eric_befund = dict(eric_befund)
        eric_befund["gebunden_an"] = sid            # bindet unweigerlich an DIESEN Zustand
        snap["eric_befund"] = eric_befund
    store.setdefault("snapshots", []).append(snap)
    return snap
