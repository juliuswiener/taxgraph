"""Traverser — der Regel-Graph in zwei Leserichtungen (K1, Task #11). READ-ONLY, NULL LLM.

Rückwärts = Interview (`relevanz`, `naechste_fragen`), vorwärts = Beweis (`justification`,
`trace_ergebnis`). Reine Ableitung über Bindungstabelle + rules.yaml + Store; keine Catala-
Introspektion (Grenze, s. KONZEPT.md). Die EINZIGE Sicht, die Paket B (Haut) liest; geschrieben
wird ausschließlich über `store.append_event` (API.md).
"""
from __future__ import annotations

import functools
import glob
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
PRODUKT = os.path.dirname(HERE)
ROOT = os.path.dirname(PRODUKT)


# ---------------------------------------------------------------- Loader

def _yaml():
    import yaml
    return yaml


@functools.lru_cache(maxsize=1)
def lade_bindung() -> dict:
    """feld_id -> Bindungs-Eintrag (über alle bindung_*.yaml). Pro Prozess gecacht (statischer
    Repo-Content, ändert sich nie zur Laufzeit) — war ungecacht ~161ms/Call, Hotpath in JEDEM
    api.py-Handler."""
    yaml = _yaml()
    out = {}
    for f in glob.glob(os.path.join(PRODUKT, "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f))
        for b in d.get("bindungen", []):
            out[b["feld_id"]] = b
    return out


def lade_rules() -> dict:
    yaml = _yaml()
    doc = yaml.safe_load(open(os.path.join(ROOT, "pipeline", "produktion", "rules.yaml")))
    return {r["rule_id"]: r for r in doc["regeln"]}


def lade_guenstiger() -> dict:
    yaml = _yaml()
    return yaml.safe_load(open(os.path.join(HERE, "guenstiger_liste.yaml")))


def _aktive_events(store: dict) -> dict:
    """feld_id -> aktuell aktives Event (nicht durch ein späteres ersetzt)."""
    ersetzt = {e["ersetzt"] for e in store.get("events", []) if e.get("ersetzt")}
    aktiv = {}
    for e in store.get("events", []):
        if e["event_id"] in ersetzt:
            continue
        aktiv[e["feld_id"]] = e
    return aktiv


def _unbeantwortet(ev) -> bool:
    return ev is None or ev.get("zustand") == "vorlaeufig"


# ---------------------------------------------------------------- (a) RÜCKWÄRTS

def _regel_ids(bindung: dict) -> set:
    return {b["quelle"]["regel_id"] for b in bindung.values()}


def relevanz(store: dict, bindung: dict) -> dict:
    """Je Regel: status (ausgeschlossen|relevant|unentschieden) + offene Gates + offene Annahmen.

    Gate = askable bool-Geltungsbedingung. `false` (bestätigt) -> ausgeschlossen; offen/vorlaeufig ->
    unentschieden. Nicht-askable (berechnete) Geltungsbedingungen sind KEIN Gate, werden aber als
    `annahmen_offen` geführt (nie still als erfüllt, Auflage 1-Zusatz)."""
    aktiv = _aktive_events(store)
    out = {}
    for rid in _regel_ids(bindung):
        gates, annahmen = [], []
        for fid, b in bindung.items():
            q = b["quelle"]
            if q["regel_id"] != rid or "geltungsbedingung" not in q:
                continue
            if b.get("askable"):
                gates.append(fid)
            else:
                annahmen.append(q["geltungsbedingung"])
        status, offen = "relevant", []
        for fid in gates:
            ev = aktiv.get(fid)
            if _unbeantwortet(ev):
                offen.append(fid)
            elif ev.get("wert") is False:
                status = "ausgeschlossen"
                break
        if status != "ausgeschlossen":
            status = "unentschieden" if offen else "relevant"
        out[rid] = {"status": status, "gates_offen": sorted(offen),
                    "annahmen_offen": sorted(annahmen)}
    return out


def naechste_fragen(store: dict, bindung: dict, beitrag: dict | None = None) -> list[str]:
    """Geordnete Interview-Queue: unbeantwortete askable Felder nicht-ausgeschlossener Regeln.
    Gating-Bedingungen zuerst (streichen ganze Regeln), dann Slots nach Unsicherheits-Beitrag
    (aus intervall.py, wenn übergeben), sonst deterministisch feld_id-sortiert.

    Günstiger-sicher by construction: ALLE unbeantworteten askable Felder nicht-ausgeschlossener
    Regeln kommen in die Queue — kein Zweig wird anhand eines vorläufigen Siegers weggeschnitten."""
    rel = relevanz(store, bindung)
    aktiv = _aktive_events(store)
    kand = [fid for fid, b in bindung.items()
            if b.get("askable") and _unbeantwortet(aktiv.get(fid))
            and rel[b["quelle"]["regel_id"]]["status"] != "ausgeschlossen"]
    gates = sorted(f for f in kand if "geltungsbedingung" in bindung[f]["quelle"])
    slots = [f for f in kand if "geltungsbedingung" not in bindung[f]["quelle"]]
    if beitrag:
        slots.sort(key=lambda f: (-beitrag.get(f, 0), f))
    else:
        slots.sort()
    return gates + slots


# ---------------------------------------------------------------- (b) VORWÄRTS

def justification(store: dict, feld_id: str, bindung: dict) -> dict | None:
    """Rekursions-Blatt: das Justification-Objekt eines Felds aus Store-Event + Bindungstabelle.
    None, wenn das Feld (noch) kein Event hat."""
    ev = _aktive_events(store).get(feld_id)
    if ev is None:
        return None
    b = bindung.get(feld_id, {})
    q = b.get("quelle", {})
    return {
        "feld_id": feld_id,
        "wert": ev["wert"],
        "zustand": ev["zustand"],
        "herkunft": ev["herkunft"],
        "event_id": ev["event_id"],
        "signal": ev.get("signal"),
        "regel_id": q.get("regel_id"),
        "signatur_slot": q.get("signatur_slot"),
        "geltungsbedingung": q.get("geltungsbedingung"),
        "anker_ref": b.get("anker_ref"),
    }


def trace_ergebnis(store: dict, bindung: dict, snapshot_id: str | None = None) -> dict:
    """Vorwärts-Trace: je beteiligter Regel (deren Felder belegt sind) die Justifications ihrer
    Felder. Regel/Slot/Feld/Event-EXAKT; per-Cent-Attribution ist benannter Nachtrag (KONZEPT.md)."""
    aktiv = _aktive_events(store)
    out = {"basis_snapshot": snapshot_id, "regeln": {}}
    for fid in aktiv:
        b = bindung.get(fid)
        if not b:
            continue
        rid = b["quelle"]["regel_id"]
        out["regeln"].setdefault(rid, []).append(justification(store, fid, bindung))
    for rid in out["regeln"]:
        out["regeln"][rid].sort(key=lambda j: j["feld_id"])
    return out
