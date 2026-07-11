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
    with open(PRAEZ_YAML, "w", encoding="utf-8") as f:
        f.write(kopf + yaml.safe_dump(d, allow_unicode=True, sort_keys=False))
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
    with open(PRAEZ_LOG, "w", encoding="utf-8") as f:
        f.write(yaml.safe_dump(d, allow_unicode=True, sort_keys=False))


# -- Fall registrieren (jede Julius-Entscheidung) -----------------------------

def record_precedent(rule_id: str, item: dict, triage: str,
                     bedingung: str | None = None,
                     konvention: str | None = None,
                     now: str | None = None) -> dict:
    """Registriert eine Julius-Entscheidung als Praezedenzfall (idempotent).

    Bereits gesperrte Faelle bleiben gesperrt (ein Widerruf reaktiviert NICHT).
    """
    now = now or _now()
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
    draft = service.open_draft(rule_id)
    items = draft.get("items", [])
    anwendbar = []
    for it in items:
        f = treffer(rule_id, it)
        if f is not None:
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
    if not dry_run and anwendbar:
        log = load_log()
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
        log["chargen"].append(charge)
        save_log(log)
    elif not dry_run:
        log = load_log()
        log["chargen"].append(charge)
        save_log(log)

    charge["angewendete"] = angewendet
    return charge


# -- Widerruf -----------------------------------------------------------------

def _registry_item_entfernen(rule_id: str, ak: str) -> bool:
    """Audit-Undo einer Auto-Apply: entfernt genau das per Praezedenz gesetzte
    Item aus der Registry (der einzige nicht-aufnehmen-Schreibzugriff, eng auf
    den Widerruf begrenzt und protokolliert). Auto-applied Items sind stets
    frische Eintraege - Entfernen setzt sie auf `offen` (discover_draft findet
    sie erneut) und der gesperrte Praezedenzfall verhindert Re-Apply.
    """
    reg = IR.load(rule_id)
    vorher = len(reg.get("items", []))
    reg["items"] = [it for it in reg.get("items", [])
                    if IR._key(it["art"], it["anker"]) != ak]
    if len(reg["items"]) != vorher:
        IR.save(reg)
        return True
    return False


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

    entfernt = _registry_item_entfernen(a["rule_id"], a["anker_schluessel"])

    # Praezedenzfall sperren
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
        "praezedenz_gesperrt": gesperrt,
        "timestamp": now,
    })
    save_log(log)
    return {"anwendung_id": anwendung_id, "item_entfernt": entfernt,
            "praezedenz_gesperrt": gesperrt}


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
