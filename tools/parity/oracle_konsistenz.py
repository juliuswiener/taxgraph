"""Orakel-Einträge für `rust/konsistenz` und `rust/intervall` (Schritt 5b).

Eingebunden von `oracle.py` über den Präfix `konsistenz.` / `intervall.`. Antwortform wie
`oracle.py::_runner`: `{"ok": <Ergebnis>}` oder `{"err": "<Ausnahmeklasse>"}`.

Synthetische Engine (beide Seiten kennen sie, `intervall_paritaet.rs`): `_synth` gewichtet jeden
Wert mit `sum(name.encode()) % 7 - 3`; Zahlwert = int, bool 0/1, str Länge (Codepoints), sonst 0.
Die echte Bescheid-Funktion kommt mit der Crate `bescheid`.
"""
from __future__ import annotations

import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
for _d in ("konsistenz", "unsicherheit", "traverser", "haut"):
    sys.path.insert(0, os.path.join(ROOT, "produkt", _d))

import check_nicht_gerechnet as _CNG  # noqa: E402
import check_pauschalen as _CP  # noqa: E402
import flag_check as _FC  # noqa: E402
import partner_check as _PC  # noqa: E402
import preflight as _PF  # noqa: E402
import intervall as _IV  # noqa: E402
import traverser as _TR  # noqa: E402
from api_constants import RING_BETRAGSFELDER as _RING  # noqa: E402


def _num(v) -> int:
    if isinstance(v, bool):
        return int(v)
    if isinstance(v, int):
        return v
    if isinstance(v, str):
        return len(v)
    return 0


def _gewicht(name: str) -> int:
    return sum(name.encode()) % 7 - 3


def _synth(werte: dict) -> int:
    return sum(_gewicht(k) * _num(v) for k, v in werte.items())


def _bindung(req: dict) -> dict:
    """`bindung_ids`: Liste echter feld_ids (Reihenfolge = dict-Reihenfolge) aus TR.lade_bindung();
    `bindung`: Liste synthetischer Einträge `{feld_id, ...}` in Python-Form."""
    if "bindung_ids" in req:
        alle = _TR.lade_bindung()
        return {f: alle[f] for f in req["bindung_ids"]}
    return {e["feld_id"]: e for e in req["bindung"]}


def _sicht(b: dict) -> dict:
    """Was intervall.py aus einem Bindungseintrag liest — Gegenprobe zu `AchsenBindung::from`."""
    ber = b.get("bereich")
    return {"feld_id": b["feld_id"], "typ": b["typ"], "askable": bool(b.get("askable")),
            "enum_werte": list(b.get("enum_werte") or []),
            "bereich": [ber["min"], ber["max"]] if ber else None,
            "signatur_slot": b["quelle"].get("signatur_slot"),
            "summand": b.get("slot_beitrag") == "summand"}


def _konstanten(_req: dict) -> dict:
    return {"flag_negiert": [[k, v] for k, v in _FC.FLAG_NEGIERT.items()],
            "partner_felder": list(_PC.PARTNER_FELDER),
            "ring_betragsfelder": list(_RING),
            "nicht_gerechnet": [[k, v] for k, v in _CNG.NICHT_GERECHNET.items()],
            "pauschal_checks": [{k: (list(v) if isinstance(v, tuple) else v) for k, v in c.items()}
                                for c in _CP.PAUSCHAL_CHECKS]}


def _preflight(req: dict) -> dict:
    snap = req["snapshot"]
    bindung = None if req.get("bindung") is None else {f: {} for f in req["bindung"]}
    luecken = _TR.fehlende_instanzen(snap, _TR.lade_bindung())
    return {"ergebnis": _PF.preflight(snap, bindung, vorjahr_referenz=req.get("vorjahr_referenz")),
            "luecken": luecken}


def _intervall(req: dict) -> dict:
    return _IV.intervall(req["snapshot"], _bindung(req), _synth, cap=req["cap"],
                         snapshot_id=req.get("snapshot_id"))


def _via_slots(req: dict) -> int:
    fn = _IV.bescheid_via_slots(_bindung(req), _synth, quantitaet=req["quantitaet"])
    return fn(dict(req["feld_werte"]))


DISPATCH = {
    "konsistenz.konstanten": _konstanten,
    "konsistenz.flag_widersprueche": lambda r: _FC.flag_widersprueche(
        r["snapshot"], None if r.get("bindung") is None else {f: {} for f in r["bindung"]}),
    "konsistenz.partner_ohne_zusammen": lambda r: _PC.partner_ohne_zusammen(r["snapshot"]),
    "konsistenz.alleinerziehend_mit_zusammen": lambda r: _PC.alleinerziehend_mit_zusammen(r["snapshot"]),
    "konsistenz.pauschal_hinweise": lambda r: _CP.pauschal_hinweise(r["snapshot"]),
    "konsistenz.nicht_gerechnete_angaben": lambda r: _CNG.nicht_gerechnete_angaben(r["snapshot"]),
    "konsistenz.preflight": _preflight,
    "intervall.sicht": lambda r: [_sicht(b) for b in _bindung(r).values()],
    "intervall.intervall": _intervall,
    "intervall.bescheid_via_slots": _via_slots,
}


def antwort(req: dict) -> dict:
    try:
        return {"ok": DISPATCH[req["fn"]](req)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparität braucht jeden Typ
        return {"err": type(exc).__name__}
