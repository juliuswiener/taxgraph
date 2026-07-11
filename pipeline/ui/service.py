"""Service-Schicht der Triage-UI - reine Logik, kein FastAPI, kein LLM.

Der EINE Schreibpfad: `submit()` baut ein Draft-Fragment und ruft
`item_registry.aufnehmen(draft)`. Es gibt hier KEINE zweite Implementierung des
Registry-Schreibens. Entscheidungs-Metadaten (timestamp, item_snapshot,
entschieden_via) liegen daneben im Entscheidungs-Log - die Registry-Datei selbst
bleibt im von den Gates konsumierten Schema.
"""

from __future__ import annotations

import os
import sys
from datetime import datetime, timezone

HERE = os.path.dirname(os.path.abspath(__file__))
PIPELINE = os.path.dirname(HERE)
ROOT = os.path.dirname(PIPELINE)
for _p in (ROOT, PIPELINE):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import item_registry as IR          # noqa: E402
import quellen                       # noqa: E402
from yamlstrict import load_yaml     # noqa: E402

RULES_YAML = os.path.join(PIPELINE, "produktion", "rules.yaml")
KONV_YAML = os.path.join(PIPELINE, "signatur_konventionen.yaml")
DISCOVERY_DIR = os.path.join(IR.REG_DIR, "discovery")
ENTSCHEIDUNGS_LOG = os.path.join(IR.REG_DIR, "entscheidungs_log.yaml")

# Nur diese Triage-Status sind ueber die UI setzbar; sie sind die Teilmenge von
# IR.TRIAGE, die eine menschliche Entscheidung ausdrueckt.
UI_TRIAGE = ("bedingung_neu", "grenzfall", "nicht_material", "nicht_echt",
             "defekt_formalisierer", "offen_bis_neuschnitt", "nicht_material_backlog")


def _now() -> str:
    return datetime.now(timezone.utc).isoformat()


# -- Manifest / Konventionen --------------------------------------------------

def _rules_list() -> list[dict]:
    d = load_yaml(RULES_YAML) or {}
    return list(d.get("regeln", []))


def alle_regeln() -> list[dict]:
    return _rules_list()


def get_rule(rule_id: str) -> dict | None:
    for r in _rules_list():
        if r.get("rule_id") == rule_id:
            return r
    return None


def konventionen() -> list[dict]:
    d = load_yaml(KONV_YAML) or {}
    return list(d.get("konventionen", []))


def _konv_ids() -> set[str]:
    return {f"konv:{k['id']}" for k in konventionen()} | {k["id"] for k in konventionen()}


def geltungsbedingungen(rule: dict) -> list[dict]:
    return list(rule.get("geltungsbedingungen") or [])


# -- Offene Items je Regel ----------------------------------------------------

def _static_draft(rule_id: str) -> dict | None:
    p = os.path.join(DISCOVERY_DIR, f"{rule_id}.yaml")
    if os.path.exists(p):
        return load_yaml(p)
    return None


def open_draft(rule_id: str) -> dict:
    """Frischer Triage-Entwurf der noch NICHT registrierten Items.

    Bevorzugt den Live-Lauf (`discover_draft`, rechnet gegen den aktuellen
    Registry-Stand). Faellt auf den eingefrorenen Discovery-Draft zurueck, falls
    kein Produktions-Report existiert.
    """
    try:
        return IR.discover_draft(rule_id)
    except SystemExit:
        st = _static_draft(rule_id)
        if st is None:
            return {"rule_id": rule_id, "items": []}
        return st


def _report_ts(rule_id: str) -> str | None:
    p = os.path.join(PIPELINE, "runs", "produktion", rule_id, "report.json")
    if not os.path.exists(p):
        return None
    import json
    try:
        d = json.load(open(p, encoding="utf-8"))
    except Exception:
        return None
    return d.get("judge_timestamp") or (d.get("judge_verdict") or {}).get("timestamp")


def queue_overview() -> list[dict]:
    """Je Regel: Anzahl offener Items und Alter (Report-Zeitstempel)."""
    out = []
    for r in _rules_list():
        rid = r["rule_id"]
        draft = open_draft(rid)
        items = draft.get("items", [])
        offen = [it for it in items if it.get("triage") == "offen"]
        vorschlag = [it for it in items if it.get("triage") not in ("offen", None)]
        out.append({
            "rule_id": rid,
            "norm": r.get("norm"),
            "offen": len(offen),
            "mit_vorschlag": len(vorschlag),
            "gesamt": len(items),
            "stand": _report_ts(rid),
        })
    return out


def norm_excerpt(rule: dict, quote: str | None, window: int = 320) -> dict:
    """Eingefrorener Normtext-Ausschnitt um den Anker.

    Quelle ist der Dokumentstore (build_norm_text ueber die eingefrorenen
    sources/-Dateien), NICHT der Judge-Output. Der Anker ist die woertliche
    Formulierung des Items; wir suchen sie im Normtext und geben die Umgebung.
    Kein Fuzzy-Schwellwert: exakter Substring, sonst Praefix der ersten Worte,
    sonst Kopf des Normtextes.
    """
    try:
        text, meta = quellen.build_norm_text(rule, ROOT)
    except Exception as e:  # QuellenFehler o.ae. -> Anzeige statt Absturz
        return {"gefunden": False, "fehler": str(e), "ausschnitt": "", "quellen": []}
    if not quote:
        return {"gefunden": False, "ausschnitt": text[:window * 2], "quellen": meta}

    def _find(hay: str, needle: str) -> int:
        i = hay.find(needle)
        if i >= 0:
            return i
        i = hay.lower().find(needle.lower())
        if i >= 0:
            return i
        # Praefix der ersten Worte (kein Aehnlichkeits-Score, nur ein Anker)
        worte = needle.split()
        for n in (10, 8, 6, 4):
            if len(worte) >= n:
                pref = " ".join(worte[:n])
                j = hay.lower().find(pref.lower())
                if j >= 0:
                    return j
        return -1

    idx = _find(text, quote.strip())
    if idx < 0:
        return {"gefunden": False, "ausschnitt": text[:window * 2], "quellen": meta}
    start = max(0, idx - window)
    end = min(len(text), idx + len(quote) + window)
    return {
        "gefunden": True,
        "ausschnitt": ("…" if start > 0 else "") + text[start:end] + ("…" if end < len(text) else ""),
        "treffer": quote.strip(),
        "quellen": meta,
    }


def rule_context(rule_id: str) -> dict:
    rule = get_rule(rule_id)
    if rule is None:
        raise KeyError(rule_id)
    draft = open_draft(rule_id)
    return {
        "rule_id": rule_id,
        "norm": rule.get("norm"),
        "geltungsbedingungen": geltungsbedingungen(rule),
        "konventionen": konventionen(),
        "items": draft.get("items", []),
        "hinweis": draft.get("hinweis"),
    }


def item_context(rule_id: str, quote: str) -> dict:
    """Einzel-Item-Kontext: Anker-Umgebung im eingefrorenen Normtext."""
    rule = get_rule(rule_id)
    if rule is None:
        raise KeyError(rule_id)
    return {
        "rule_id": rule_id,
        "geltungsbedingungen": geltungsbedingungen(rule),
        "konventionen": konventionen(),
        "normtext": norm_excerpt(rule, quote),
    }


# -- Entscheidungs-Log (Audit neben der Registry) -----------------------------

def _load_log(path: str, key: str) -> dict:
    if os.path.exists(path):
        return load_yaml(path) or {key: []}
    return {key: []}


def _save_log(path: str, data: dict) -> None:
    import yaml
    os.makedirs(os.path.dirname(path), exist_ok=True)
    IR._atomic_write(path, yaml.safe_dump(data, allow_unicode=True, sort_keys=False))


def entscheidungs_log() -> list[dict]:
    return _load_log(ENTSCHEIDUNGS_LOG, "entscheidungen").get("entscheidungen", [])


def _log_entscheidung(eintrag: dict) -> None:
    data = _load_log(ENTSCHEIDUNGS_LOG, "entscheidungen")
    data.setdefault("entscheidungen", []).append(eintrag)
    _save_log(ENTSCHEIDUNGS_LOG, data)


# -- DER Schreibpfad: submit -> aufnehmen -------------------------------------

def build_draft_item(item: dict, triage: str,
                     bedingung: str | None = None,
                     konvention: str | None = None) -> dict:
    """Baut EIN Draft-Item im Format, das `aufnehmen` erwartet.

    `item` ist das Snapshot des zu triagierenden Items (aus open_draft): traegt
    art, anker, text/formulierung und optional klasse.
    """
    art = item.get("art")
    anker = item.get("anker")
    if not art or not anker:
        raise ValueError("item braucht art und anker")
    text = item.get("text") or item.get("formulierung") or ""
    d = {"art": art, "anker": anker, "text": text, "triage": triage}
    if item.get("klasse"):
        d["klasse"] = item["klasse"]
    if triage == "bedingung_neu":
        if not bedingung:
            raise ValueError("bedingung_neu braucht eine bedingung-ID")
        d["bedingung"] = bedingung
    if triage == "nicht_material" and konvention:
        d["konvention"] = konvention  # von aufnehmen ignoriert, im Log erfasst
    return d


def submit(rule_id: str, item: dict, triage: str,
           bedingung: str | None = None,
           weitere_bedingungen: list[str] | None = None,
           konvention: str | None = None,
           neue_bedingung: dict | None = None,
           now: str | None = None,
           entschieden_via: str = "ui") -> dict:
    """Nimmt eine Triage-Entscheidung entgegen und schreibt sie ueber
    `item_registry.aufnehmen`.

    bedingung_neu braucht eine bedingung-ID (aufnehmen erzwingt das). Ob die
    Bedingung schon im Manifest deklariert ist, ist KEINE Vorbedingung des
    Schreibens: ein bedingung_neu-Item mit noch nicht deklarierter Bedingung ist
    der Normalfall, der den geltungsbereich_gate im Review haelt, bis Julius die
    Bedingung in rules.yaml ergaenzt. Das Ergebnis traegt daher nur ein
    Warn-Flag `bedingung_deklariert`. nicht_material -> Konvention muss existieren
    (geschlossene Menge, echter Tippfehler-Schutz).

    Rueckgabe: {"pfad", "aufgenommen", "gemerged", "version", "bedingung_deklariert"}.
    """
    if triage not in UI_TRIAGE:
        raise ValueError(f"unbekannter Triage-Status: {triage}")
    rule = get_rule(rule_id)
    if rule is None:
        raise KeyError(rule_id)
    now = now or _now()

    bedingung_deklariert = None
    if triage == "bedingung_neu":
        if not bedingung:
            raise ValueError("bedingung_neu braucht eine bedingung-ID")
        deklariert = {b["bedingung"] for b in geltungsbedingungen(rule)}
        bedingung_deklariert = bedingung in deklariert
    if triage == "nicht_material" and konvention and konvention not in _konv_ids():
        raise ValueError(f"unbekannte Konvention: {konvention}")

    draft_item = build_draft_item(item, triage, bedingung=bedingung,
                                  konvention=konvention)

    # Merge-Erkennung: existiert der Schluessel bereits mit anderer Formulierung?
    reg_vorher = IR.load(rule_id)
    idx = {IR._key(it["art"], it["anker"]): it for it in reg_vorher.get("items", [])}
    k = IR._key(draft_item["art"], draft_item["anker"])
    vorher = idx.get(k)
    gemerged = bool(vorher and draft_item["text"]
                    and draft_item["text"] not in vorher.get("formulierungen", []))

    pfad, n = IR.aufnehmen({"rule_id": rule_id, "items": [draft_item]})
    version = IR.load(rule_id).get("version")

    _log_entscheidung({
        "rule_id": rule_id,
        "schluessel": k,
        "triage": triage,
        "bedingung": bedingung,
        "weitere_bedingungen": weitere_bedingungen or [],
        "konvention": konvention,
        "neue_bedingung": neue_bedingung,
        "bedingung_deklariert": bedingung_deklariert,
        "gemerged": gemerged,
        "item_snapshot": item,
        "entschieden_via": entschieden_via,
        "timestamp": now,
    })
    return {"pfad": pfad, "aufgenommen": n, "gemerged": gemerged, "version": version,
            "schluessel": k, "bedingung_deklariert": bedingung_deklariert}


# -- Merge-Log ----------------------------------------------------------------

def merge_log() -> list[dict]:
    """Gemergte Items ueber alle Regeln: Registry-Eintraege mit >1 Formulierung.
    Zeigt beide (alle) Originalformulierungen."""
    out = []
    if not os.path.isdir(IR.REG_DIR):
        return out
    for fn in sorted(os.listdir(IR.REG_DIR)):
        if not fn.endswith(".yaml") or fn == "praezedenz.yaml":
            continue
        rid = fn[:-5]
        reg = IR.load(rid)
        for it in reg.get("items", []):
            forms = it.get("formulierungen", [])
            if len(forms) > 1:
                out.append({
                    "rule_id": rid,
                    "art": it.get("art"),
                    "anker": it.get("anker"),
                    "triage": it.get("triage"),
                    "bedingung": it.get("bedingung"),
                    "formulierungen": forms,
                })
    return out
