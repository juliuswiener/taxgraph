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


def speichere(pfad: str, store: dict) -> None:
    import yaml
    d = os.path.dirname(pfad) or "."
    os.makedirs(d, exist_ok=True)
    tmp = tempfile.NamedTemporaryFile("w", dir=d, delete=False, encoding="utf-8", suffix=".tmp")
    try:
        yaml.safe_dump(store, tmp, allow_unicode=True, sort_keys=False)
        tmp.flush()
        os.fsync(tmp.fileno())
    finally:
        tmp.close()
    os.replace(tmp.name, pfad)


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


def append_event(store: dict, *, feld_id: str, wert, zustand: str, herkunft: dict,
                 schreiber: str, signal: dict | None = None, ersetzt: str | None = None,
                 ts: str | None = None) -> dict:
    """Baut EIN Event, prüft fail-closed (Auflagen A+B), setzt den content-adressierten event_id,
    hängt es an. Gibt das Event zurück. KEINE zweite Schreib-Implementierung."""
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
