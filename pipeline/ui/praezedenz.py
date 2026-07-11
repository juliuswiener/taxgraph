"""Praezedenz-Ratsche (M-UI.3) - Durchsatz durch Wiederverwendung von Julius'
Entscheidungen, OHNE Fuzzy.

Praezedenzfall = eine von Julius entschiedene Triage mit Schluessel
(anker_normalisiert, kategorie, entscheidungstyp, ziel_id). Auto-Apply gilt NUR
bei EXAKTER Anker-Schluesselgleichheit eines neuen Items mit genau einem
(nicht gesperrten) Praezedenzfall - dieselbe Grenze wie `seed_mechanisch`
(item_registry.seed_mechanisch matcht auf `_key(art, anker)`). Kein
Aehnlichkeits-Schwellwert, keine Naeherung.

Zusatzbedingung fuer Auto-Apply: das Ziel muss in der Zielregel gueltig sein -
bei `bedingung_neu` muss die Bedingung in den geltungsbedingungen der Zielregel
deklariert sein; sonst Queue. Alles Uneindeutige (0 oder >1 Treffer, ungueltiges
Ziel) -> Queue.

Persistenz:
  pipeline/item_registry/praezedenz.yaml      - die Faelle
  pipeline/item_registry/praezedenz_log.yaml  - Audit (Anwendungen, Widerrufe,
                                                Chargen-Metriken)
"""

from __future__ import annotations

import json
import os
from datetime import datetime, timezone

from . import service
from .service import IR

PRAEZ_YAML = os.path.join(IR.REG_DIR, "praezedenz.yaml")
PRAEZ_LOG = os.path.join(IR.REG_DIR, "praezedenz_log.yaml")


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


# Nur diese Entscheidungstypen werden je cross-rule automatisch angewendet
# (Design-Entscheid Julius 2026-07-11). defekt_formalisierer/nicht_echt/grenzfall/
# offen_bis_neuschnitt/backlog sind kontextgebunden -> nur Praezedenz-Hinweis in
# der Queue, nie Auto-Apply in einer anderen Regel.
AUTO_APPLY_WHITELIST = {"bedingung_neu", "nicht_material"}


def _anker_degeneriert(item: dict) -> bool:
    """Degenerierter Anker (enthaelt "?"): kein tragfaehiger Praezedenz-Schluessel.
    Betrifft v.a. art:abweichung (Anker ["betrifft","?"]); ein einziger Registry-
    Eintrag je Regel, jeder kuenftige echte Fund kollidierte als "bekannt" ->
    stilles Gruen per Konstruktion. Solche Anker werden nie Praezedenz."""
    return "?" in (item.get("anker") or [])


# -- Schluessel ---------------------------------------------------------------

def item_kategorie(item: dict) -> str | None:
    if item.get("kategorie"):
        return item["kategorie"]
    anker = item.get("anker") or []
    if item.get("art") == "annahme" and len(anker) >= 3:
        return anker[2]
    return None


def ziel_id(triage: str, bedingung: str | None, konvention: str | None) -> str | None:
    if triage == "bedingung_neu":
        return bedingung
    if triage == "nicht_material":
        return konvention
    return None


def anker_schluessel(item: dict) -> str:
    """Exakter Anker-Schluessel - die Auto-Apply-Grenze (wie seed_mechanisch)."""
    return IR._key(item["art"], item["anker"])


def praez_schluessel(item: dict, triage: str, bedingung: str | None,
                     konvention: str | None) -> str:
    return json.dumps([anker_schluessel(item), item_kategorie(item), triage,
                       ziel_id(triage, bedingung, konvention)], ensure_ascii=False)


# -- Persistenz ---------------------------------------------------------------

def load_praez() -> dict:
    if os.path.exists(PRAEZ_YAML):
        d = service.load_yaml(PRAEZ_YAML) or {}
        d.setdefault("faelle", [])
        return d
    return {"version": 0, "faelle": []}


def save_praez(d: dict) -> str:
    import yaml
    os.makedirs(IR.REG_DIR, exist_ok=True)
    d["version"] = int(d.get("version", 0)) + 1
    kopf = ("# Praezedenz-Ratsche (M-UI.3). Von Julius entschiedene Triagen als\n"
            "# Praezedenzfaelle. Auto-Apply nur bei EXAKTER Anker-Gleichheit.\n"
            "# gesperrt: true -> durch Widerruf gesperrt, nie wieder Auto-Apply.\n\n")
    IR._atomic_write(PRAEZ_YAML, kopf + yaml.safe_dump(d, allow_unicode=True,
                                                       sort_keys=False))
    return PRAEZ_YAML


def load_log() -> dict:
    if os.path.exists(PRAEZ_LOG):
        d = service.load_yaml(PRAEZ_LOG) or {}
        d.setdefault("anwendungen", [])
        d.setdefault("chargen", [])
        return d
    return {"anwendungen": [], "chargen": []}


def save_log(d: dict) -> None:
    import yaml
    os.makedirs(IR.REG_DIR, exist_ok=True)
    IR._atomic_write(PRAEZ_LOG, yaml.safe_dump(d, allow_unicode=True,
                                               sort_keys=False))


# -- Fall registrieren (jede Julius-Entscheidung) -----------------------------

def record_precedent(rule_id: str, item: dict, triage: str,
                     bedingung: str | None = None,
                     konvention: str | None = None,
                     now: str | None = None) -> dict:
    """Registriert eine Julius-Entscheidung als Praezedenzfall (idempotent).

    Bereits gesperrte Faelle bleiben gesperrt (ein Widerruf reaktiviert NICHT).
    """
    now = now or _now()
    if _anker_degeneriert(item):
        # Kein Praezedenzfall aus einem degenerierten Anker (D0).
        return {"skipped": "degenerierter Anker", "anker": item.get("anker")}
    d = load_praez()
    sk = praez_schluessel(item, triage, bedingung, konvention)
    for f in d["faelle"]:
        if f["schluessel"] == sk:
            return f  # existiert schon (evtl. gesperrt) -> unveraendert
    fall = {
        "schluessel": sk,
        "anker_schluessel": anker_schluessel(item),
        "art": item["art"],
        "anker": item["anker"],
        "kategorie": item_kategorie(item),
        "entscheidungstyp": triage,
        "ziel_id": ziel_id(triage, bedingung, konvention),
        "gesperrt": False,
        "quelle_rule_id": rule_id,
        "quelle_text": (item.get("text") or item.get("formulierung") or "")[:200],
        "entschieden_am": now,
    }
    d["faelle"].append(fall)
    save_praez(d)
    return fall


# -- Treffer suchen -----------------------------------------------------------

def _kandidaten(item: dict) -> list[dict]:
    if _anker_degeneriert(item):
        return []  # D0: degenerierter Anker matcht nie
    ak = anker_schluessel(item)
    d = load_praez()
    return [f for f in d["faelle"]
            if f.get("anker_schluessel") == ak and not f.get("gesperrt")]


def _ziel_gueltig(rule: dict, fall: dict) -> bool:
    typ = fall["entscheidungstyp"]
    if typ == "bedingung_neu":
        deklariert = {b["bedingung"] for b in service.geltungsbedingungen(rule)}
        return fall["ziel_id"] in deklariert
    if typ == "nicht_material":
        return (fall["ziel_id"] is None) or (fall["ziel_id"] in service._konv_ids())
    return True  # grenzfall/nicht_echt/defekt/offen_bis_neuschnitt/backlog: kontextfrei


def treffer(rule_id: str, item: dict) -> dict | None:
    """Genau ein nicht gesperrter Praezedenzfall mit EXAKT gleichem Anker UND
    in der Zielregel gueltigem Ziel -> anwendbar. Sonst None (Queue)."""
    rule = service.get_rule(rule_id)
    if rule is None:
        return None
    kand = _kandidaten(item)
    if len(kand) != 1:
        return None  # 0 oder mehrdeutig -> Queue
    fall = kand[0]
    if not _ziel_gueltig(rule, fall):
        return None
    return fall


# -- Auto-Apply je Charge -----------------------------------------------------

def _bedingung_konv(fall: dict) -> tuple[str | None, str | None]:
    if fall["entscheidungstyp"] == "bedingung_neu":
        return fall["ziel_id"], None
    if fall["entscheidungstyp"] == "nicht_material":
        return None, fall["ziel_id"]
    return None, None


def auto_apply(rule_id: str, now: str | None = None,
               dry_run: bool = False) -> dict:
    """Wendet passende Praezedenzfaelle auf die offenen Items der Regel an.

    Jedes Auto-Apply schreibt ueber `service.submit` (== item_registry.aufnehmen,
    derselbe Schreibpfad) und wird im Audit-Log erfasst. Rueckgabe traegt die
    Auto-Apply-Quote der Charge (Durchsatz-Kennzahl).
    """
    now = now or _now()
    if service.get_rule(rule_id) is None:
        # m2: rule_id validieren, BEVOR daraus ein Report-/Draft-Pfad wird.
        raise KeyError(f"unbekannte Regel: {rule_id}")
    draft = service.open_draft(rule_id)
    items = draft.get("items", [])
    # C1(a): Schluessel, die bereits in der Registry stehen (Seeding oder Human-
    # Triage), sind vom Auto-Apply ausgenommen - sonst schriebe die Ratsche auf
    # einen fremden Eintrag, den ein Widerruf spaeter mit-loeschte.
    reg_keys = {IR._key(it["art"], it["anker"])
                for it in IR.load(rule_id).get("items", [])}
    anwendbar = []
    gesehen: set[str] = set()          # M2: Charge-interner Anker-Dedupe
    for it in items:
        if it.get("triage") != "offen":    # M2: nur offene Items
            continue
        ak = anker_schluessel(it)
        if ak in reg_keys or ak in gesehen:    # C1(a) + M2
            continue
        f = treffer(rule_id, it)
        if f is None or f["entscheidungstyp"] not in AUTO_APPLY_WHITELIST:
            continue                        # Whitelist: nur bedingung_neu/nicht_material
        gesehen.add(ak)
        anwendbar.append((it, f))

    quote = (len(anwendbar) / len(items)) if items else 0.0
    charge = {
        "rule_id": rule_id,
        "charge_groesse": len(items),
        "angewendet": len(anwendbar),
        "quote": round(quote, 4),
        "timestamp": now,
        "dry_run": dry_run,
    }
    angewendet = []
    if not dry_run:
        log = load_log()
        try:
            for it, fall in anwendbar:
                bed, konv = _bedingung_konv(fall)
                res = service.submit(
                    rule_id, it, fall["entscheidungstyp"],
                    bedingung=bed, konvention=konv, now=now,
                    entschieden_via="praezedenz-auto")
                eintrag = {
                    "id": f"{rule_id}:{res['schluessel']}",
                    "rule_id": rule_id,
                    "anker_schluessel": anker_schluessel(it),
                    "praezedenz_schluessel": fall["schluessel"],
                    "entscheidungstyp": fall["entscheidungstyp"],
                    "ziel_id": fall["ziel_id"],
                    "item_snapshot": it,
                    "timestamp": now,
                    "widerrufen": False,
                }
                log["anwendungen"].append(eintrag)
                angewendet.append(eintrag)
                save_log(log)               # M1: Audit sofort je Item, nicht am Ende
        finally:
            log["chargen"].append(charge)
            save_log(log)

    charge["angewendete"] = angewendet
    return charge


# -- Widerruf -----------------------------------------------------------------

def _registry_item_entfernen(rule_id: str, ak: str, anwendung: dict) -> str:
    """Audit-Undo einer Auto-Apply: entfernt das Item NUR, wenn es exakt der von
    dieser Anwendung angelegte frische Eintrag ist (C1). Snapshot-Match = gleiche
    Triage UND einzig die eine, von uns hinzugefuegte Formulierung. Hat Seeding
    oder eine Human-Triage den Eintrag inzwischen angefasst (andere Triage, weitere
    Formulierungen), bleibt er stehen - der Widerruf sperrt dann nur den
    Praezedenzfall. Rueckgabe: "entfernt" | "kein_match" | "nicht_gefunden".
    """
    reg = IR.load(rule_id)
    items = reg.get("items", [])
    ziel = next((it for it in items if IR._key(it["art"], it["anker"]) == ak), None)
    if ziel is None:
        return "nicht_gefunden"
    snap = anwendung.get("item_snapshot") or {}
    snap_text = snap.get("text") or snap.get("formulierung") or ""
    match = (ziel.get("triage") == anwendung.get("entscheidungstyp")
             and ziel.get("formulierungen", []) == [snap_text])
    if not match:
        return "kein_match"
    reg["items"] = [it for it in items if IR._key(it["art"], it["anker"]) != ak]
    IR.save(reg)
    return "entfernt"


def widerruf(anwendung_id: str, now: str | None = None) -> dict:
    """Widerruft eine Auto-Apply: Item zurueck auf offen UND Praezedenzfall
    gesperrt (nie wieder Auto-Apply)."""
    now = now or _now()
    log = load_log()
    treffer_ = [a for a in log["anwendungen"]
                if a["id"] == anwendung_id and not a.get("widerrufen")]
    if not treffer_:
        raise KeyError(f"keine offene Auto-Apply mit id {anwendung_id}")
    a = treffer_[0]

    item_status = _registry_item_entfernen(a["rule_id"], a["anker_schluessel"], a)
    entfernt = item_status == "entfernt"

    # Praezedenzfall IMMER sperren (auch wenn der Eintrag geschuetzt blieb): nie
    # wieder Auto-Apply dieses Schluessels.
    d = load_praez()
    gesperrt = False
    for f in d["faelle"]:
        if f["schluessel"] == a["praezedenz_schluessel"]:
            f["gesperrt"] = True
            f["gesperrt_am"] = now
            gesperrt = True
    if gesperrt:
        save_praez(d)

    a["widerrufen"] = True
    a["widerrufen_am"] = now
    log.setdefault("widerrufe", []).append({
        "anwendung_id": anwendung_id,
        "praezedenz_schluessel": a["praezedenz_schluessel"],
        "rule_id": a["rule_id"],
        "item_entfernt": entfernt,
        "item_status": item_status,      # C1: entfernt | kein_match | nicht_gefunden
        "praezedenz_gesperrt": gesperrt,
        "timestamp": now,
    })
    save_log(log)
    return {"anwendung_id": anwendung_id, "item_entfernt": entfernt,
            "item_status": item_status, "praezedenz_gesperrt": gesperrt}


# -- Anzeige ------------------------------------------------------------------

def alle_faelle() -> list[dict]:
    return load_praez().get("faelle", [])


def anwendungen(nur_offen: bool = False) -> list[dict]:
    aw = load_log().get("anwendungen", [])
    if nur_offen:
        aw = [a for a in aw if not a.get("widerrufen")]
    return aw


def chargen() -> list[dict]:
    return load_log().get("chargen", [])
