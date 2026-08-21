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


@functools.lru_cache(maxsize=1)
def lade_regel_bedingungen() -> dict:
    """regel_id -> Liste strukturierter Ob-Bedingungen ({regel_id, feld, wert, grund}) über alle
    bindung_*.yaml (schema.json $defs/regel_bedingung). Regel-weit, unabhängig von den eigenen
    Gate-Feldern der Regel (relevanz() prüft `feld` gegen den Store, nicht gegen die Regel selbst)."""
    yaml = _yaml()
    out = {}
    for f in glob.glob(os.path.join(PRODUKT, "bindung", "bindung_*.yaml")):
        d = yaml.safe_load(open(f))
        for rb in d.get("regel_bedingungen", []):
            out.setdefault(rb["regel_id"], []).append(rb)
    return out


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
    `annahmen_offen` geführt (nie still als erfüllt, Auflage 1-Zusatz).

    `gate: false` (Bindungs-Schema) nimmt ein askables Feld aus den Gates heraus: es ist DEKLARATION,
    keine Rechen-Voraussetzung. Der Vordruck verlangt die Angabe ("wurde als Ferienwohnung
    genutzt?"), die Regel gilt in beide Richtungen. Ohne diese Unterscheidung nahm ein "nein" des
    Normalfalls dem Vermieter die ganze Anlage V aus dem Dialog (gemessen 2026-08-16). Die
    Geltungsbedingung erscheint dann als offene Annahme — nie still als erfüllt.

    Zusätzlich `regel_bedingungen` (lade_regel_bedingungen, schema.json $defs/regel_bedingung):
    strukturierte Ob-Bedingung AUSSERHALB der eigenen Regel-Felder (z.B. p2_festzusetzung_zusammen
    gilt nur bei veranlagung=="zusammen" — ein Feld, das selbst zu einer ANDEREN Regel gehört, kann
    ein bool-Gate strukturell nie sein). Bestätigt UND abweichend -> ausgeschlossen; unbeantwortet/
    vorläufig schließt NICHT aus (fail-closed, wie die Gates)."""
    aktiv = _aktive_events(store)
    bedingungen = lade_regel_bedingungen()
    out = {}
    for rid in _regel_ids(bindung):
        gates, annahmen = [], []
        for fid, b in bindung.items():
            q = b["quelle"]
            if q["regel_id"] != rid or "geltungsbedingung" not in q:
                continue
            if b.get("askable") and b.get("gate", True):
                gates.append(fid)
            else:
                annahmen.append(q["geltungsbedingung"])
        status, offen = "relevant", []
        for cond in bedingungen.get(rid, []):
            ev = aktiv.get(cond["feld"])
            if not _unbeantwortet(ev) and ev.get("wert") != cond["wert"]:
                status = "ausgeschlossen"
        if status != "ausgeschlossen":
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


def gate_gewicht(bindung: dict) -> dict:
    """feld_id -> Zahl der askable Felder, die die Antwort auf dieses Gate abschalten kann.

    Das ist die Reihenfolge-Information, die im Graphen ohnehin steckt: ein Gate, dessen „nein"
    ganze Regeln streicht, erspart dem Nutzer mehr Fragen als eines, das nur sich selbst betrifft
    — also gehört es nach vorn. Zwei Quellen, beide aus der Bindung, keine Handliste:

      (a) eigenes bool-Gate der Regel — ein bestätigtes False schließt sie aus (relevanz(), s.o.),
          und alle ÜBRIGEN askable Felder derselben Regel entfallen mit;
      (b) das Feld steht als Ob-Bedingung einer FREMDEN Regel in regel_bedingungen — dann zählen
          deren askable Felder.

    Gemessen (Scheibe gesamt, 2026-08-14): veranlagung 38, vpf-Gates 13, hh_hat_aufwendungen 10.
    Vor dieser Sortierung standen die Gates alphabetisch, veranlagung damit auf Frage 203 von 243
    — die Antwort, die über 38 Partner-Felder entscheidet, kam fast zuletzt.

    Wächst von selbst mit: jeder neue regel_bedingungen-Eintrag (Screening-Modell) gibt seinem
    Gate automatisch Gewicht, ohne dass hier etwas nachgetragen werden muss."""
    bedingungen = lade_regel_bedingungen()
    felder_je_regel: dict[str, list[str]] = {}
    for fid, b in bindung.items():
        if b.get("askable"):
            felder_je_regel.setdefault(b["quelle"]["regel_id"], []).append(fid)

    gewicht: dict[str, int] = {}
    for fid, b in bindung.items():
        if not b.get("askable"):
            continue
        q = b["quelle"]
        n = 0
        if "geltungsbedingung" in q and b.get("typ") in ("bool", "boolean"):
            n += sum(1 for f in felder_je_regel.get(q["regel_id"], []) if f != fid)
        for rid, conds in bedingungen.items():
            if any(c["feld"] == fid for c in conds):
                n += len(felder_je_regel.get(rid, []))
        gewicht[fid] = n
    return gewicht


def naechste_fragen(store: dict, bindung: dict, beitrag: dict | None = None) -> list[str]:
    """Geordnete Interview-Queue: unbeantwortete askable Felder nicht-ausgeschlossener Regeln.
    Gating-Bedingungen zuerst (streichen ganze Regeln), dann Slots nach Unsicherheits-Beitrag
    (aus intervall.py, wenn übergeben), sonst deterministisch feld_id-sortiert.

    Die Gates untereinander stehen nach gate_gewicht() — wer viel abschaltet, kommt zuerst; bei
    gleichem Gewicht alphabetisch, damit die Reihenfolge deterministisch bleibt.

    Günstiger-sicher by construction: ALLE unbeantworteten askable Felder nicht-ausgeschlossener
    Regeln kommen in die Queue — kein Zweig wird anhand eines vorläufigen Siegers weggeschnitten."""
    rel = relevanz(store, bindung)
    aktiv = _aktive_events(store)
    kand = [fid for fid, b in bindung.items()
            if b.get("askable") and _unbeantwortet(aktiv.get(fid))
            and rel[b["quelle"]["regel_id"]]["status"] != "ausgeschlossen"]
    gw = gate_gewicht(bindung)
    # „Gate" heißt hier: die Antwort streicht andere Fragen — nicht: das Feld trägt technisch eine
    # geltungsbedingung. Beides fällt auseinander, und zwar beim wichtigsten Feld überhaupt:
    # `veranlagung` hat KEINE geltungsbedingung (es wirkt über regel_bedingungen auf
    # p2_festzusetzung_zusammen) und landete deshalb bei den Slots — alphabetisch auf Frage 203 von
    # 243, obwohl seine Antwort 38 Partner-Felder entscheidet. Wer abschaltet, kommt nach vorn.
    # BEI GLEICHEM GEWICHT ENTSCHIED BISHER DER BUCHSTABE — und das ist keine Ordnung, sondern
    # Zufall. Gemessen 2026-08-21 im echten Nutzerlauf: § 35a führt vier Gates mit je Gewicht 10;
    # `hh_handwerker_keine_foerderung` steht alphabetisch vor `hh_hat_aufwendungen`. Julius bekam
    # deshalb "Wurden die Handwerkerleistungen NICHT öffentlich gefördert?", bevor überhaupt
    # gefragt war, ob er Handwerkerkosten hatte — eine Frage nach dem Merkmal eines Sachverhalts,
    # dessen Existenz niemand erhoben hatte. Antwortet er darauf "nein", ist die Regel
    # ausgeschlossen und die Eingangsfrage kommt nie: das Ergebnis stimmt zufällig, die Begründung
    # nicht. In ZEHN Regeln entscheidet so der Feldname (tests/test_eingangsfrage_zuerst.py misst es).
    #
    # `eingangsfrage: true` in der Bindung bricht den Gleichstand: die Frage nach der EXISTENZ
    # kommt vor jeder Frage nach MERKMALEN. Deklariert statt geraten — eine Heuristik über den
    # Feldnamen ("hat_", "kein_") ist genau der Fehler, der bei der Frage-Polarität schon einmal
    # zwei Feldern das Gegenteil der Nutzerantwort entlockt hat (s. `frage_invertiert`).
    ist_gate = lambda f: "geltungsbedingung" in bindung[f]["quelle"] or gw.get(f, 0) > 0
    gates = sorted((f for f in kand if ist_gate(f)),
                   key=lambda f: (-gw.get(f, 0), not bindung[f].get("eingangsfrage"), f))
    slots = [f for f in kand if not ist_gate(f)]
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
