"""Paritaets-Orakel fuer die Feld-Kennungs-Regel (`rust/parity/tests/feld_kennung_paritaet.rs`):
beantwortet `{"fn": "feld_kennung.<name>", ...}` ueber die Python-Referenz.

Vier Regeln, je eine Antwort:
  - `echte`       alle Kennungen der Bindung (`traverser.lade_bindung`).
  - `schema`      die Schema-Regel, wie `jsonschema` sie anwendet (`pattern` per `re.search`): das `pattern` von
                  `feld_id` aus `produkt/store/schema.json` UND `produkt/bindung/schema.json`, beide gelesen,
                  nicht nachgeschrieben. Pythons `$` passt auch vor einem abschliessenden `\\n`.
  - `parse_instanz`  `est_mapping.parse_instanz`: `[basis, "<n>"]` oder `null`. Der Zaehler steht als Text, weil
                  Python ihn unbeschraenkt fuehrt und JSON in Rust bei 2^53 rundet.
  - `feld_id`     die Konvention des Traversers (`traverser.instanz_feld_id`): Instanz 1 ist die Basis, n >= 2
                  traegt `basis__n`. Was `instanz_feld_id` nicht zurueck erzeugt, ist keine Instanz
                  (`_instanz_antworten`, Rundreise); eine Kennung ohne Instanz-Suffix gilt, wenn sie die
                  Schema-Regel erfuellt. Antwort `[basis, "<n>"]` oder `null`.

Antwort: `{"ok": <ergebnis>}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}` (wie `elster_oracle`).
"""
from __future__ import annotations

import json
import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_M: dict = {}


def _muster(schema: object) -> list:
    """Alle `pattern` unter einer Eigenschaft `feld_id` (egal wie tief das Schema sie verschachtelt)."""
    gefunden = []
    if isinstance(schema, dict):
        eig = schema.get("feld_id")
        if isinstance(eig, dict) and "pattern" in eig:
            gefunden.append(eig["pattern"])
        for v in schema.values():
            gefunden += _muster(v)
    elif isinstance(schema, list):
        for v in schema:
            gefunden += _muster(v)
    return gefunden


def _module() -> dict:
    if not _M:
        for sub in ("produkt/store", "produkt/traverser", "produkt/mapping"):
            p = os.path.join(ROOT, sub)
            if p not in sys.path:
                sys.path.insert(0, p)
        import jsonschema  # noqa: E402
        import est_mapping as EM  # noqa: E402
        import traverser as TR  # noqa: E402
        pruefer = []
        for pfad in ("produkt/store/schema.json", "produkt/bindung/schema.json"):
            with open(os.path.join(ROOT, pfad), encoding="utf-8") as f:
                schema = json.load(f)
            muster = _muster(schema)
            assert len(set(muster)) == 1, f"{pfad}: feld_id-pattern nicht eindeutig: {muster}"
            klasse = jsonschema.validators.validator_for(schema)
            pruefer.append(klasse({"type": "string", "pattern": muster[0]}))
        _M.update(EM=EM, TR=TR, pruefer=pruefer, bindung=TR.lade_bindung())
    return _M


def _fang(f, *a):
    try:
        return {"ok": f(*a)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}


def _schema_ok(s: str) -> bool:
    """Beide Schemas, und sie muessen sich einig sein (sonst gaebe es keine EINE Schema-Regel)."""
    store, bindung = (p.is_valid(s) for p in _module()["pruefer"])
    assert store == bindung, f"Schemas uneinig bei {s!r}: store={store} bindung={bindung}"
    return store


def _echte(_req: dict) -> list:
    return sorted(_module()["bindung"])


def _schema(req: dict) -> list:
    return [_schema_ok(s) for s in req["ids"]]


def _parse_instanz(req: dict) -> list:
    EM = _module()["EM"]
    out = []
    for s in req["ids"]:
        p = EM.parse_instanz(s)
        out.append(None if p is None else [p[0], str(p[1])])
    return out


def _feld_id(req: dict) -> list:
    EM, TR = _module()["EM"], _module()["TR"]
    out = []
    for s in req["ids"]:
        p = EM.parse_instanz(s)
        if p is not None:
            basis, n = p
            out.append([basis, str(n)] if TR.instanz_feld_id(basis, n) == s else None)
        else:
            out.append([s, "1"] if _schema_ok(s) else None)
    return out


HANDLER = {"echte": _echte, "schema": _schema, "parse_instanz": _parse_instanz, "feld_id": _feld_id}


def handle(req: dict) -> dict:
    return _fang(HANDLER[req["fn"][len("feld_kennung."):]], req)
