"""Paritaets-Orakel fuer `domain::PyWert` (rust/parity/tests/wert_paritaet.rs): beantwortet
`{"fn": "wert.<name>", ...}` mit echtem CPython, ohne ein Produktmodul zu laden.

- `wert.py_eq`: `{"paare": [[a, b], ...]}`, jede Seite im Draht-Format von `wert()`. Antwort
  `{"ok": [bool, ...]}`, je Paar `a == b`.
- `wert.py_eq_json`: `{"paare": [[text, text], ...]}`; jede Seite geht durch `json.loads`, wie
  der Lader der Produktpfade. Antwort wie oben, `{"err": "<Klasse>"}` je Paar, wenn ein Text nicht laedt.

Draht-Format (kein Wert reist als JSON-Zahl, denn `json.loads` kennt weder NaN noch `u64`-exakte
Floats im Rust-Sinn): `null`, `true`/`false`, Text und Liste wie JSON; `{"i": "<dezimal>"}` eine
`int`; `{"f": "<16 Hex>"}` die IEEE-754-Bits einer `float` (NaN, -0.0, inf); `{"o": [[schluessel,
wert], ...]}` ein `dict`. Jeder Aufruf baut seine Objekte neu: zwei NaN sind nie dasselbe Objekt, die
Identitaetsabkuerzung von `list.__eq__` greift also nicht (das ponytail an `PyWert::py_eq`).

Antwort: `{"ok": <ergebnis>}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}`.
"""
from __future__ import annotations

import json
import struct


def wert(k):
    if k is None or isinstance(k, (bool, str)):
        return k
    if isinstance(k, list):
        return [wert(x) for x in k]
    if isinstance(k, dict) and len(k) == 1:
        (tag, inhalt), = k.items()
        if tag == "i":
            return int(inhalt)
        if tag == "f":
            return struct.unpack(">d", bytes.fromhex(inhalt))[0]
        if tag == "o":
            return {schluessel: wert(w) for schluessel, w in inhalt}
    raise ValueError(f"kein Draht-Wert: {k!r}")


def _py_eq(req: dict) -> list:
    return [wert(a) == wert(b) for a, b in req["paare"]]


def _py_eq_json(req: dict) -> list:
    def laedt(text):
        try:
            return json.loads(text), None
        except Exception as exc:  # noqa: BLE001 -- die Klasse gehoert zur Antwort
            return None, type(exc).__name__
    out = []
    for a, b in req["paare"]:
        (x, fa), (y, fb) = laedt(a), laedt(b)
        out.append({"err": fa or fb} if fa or fb else x == y)
    return out


HANDLER = {"py_eq": _py_eq, "py_eq_json": _py_eq_json}


def handle(req: dict) -> dict:
    try:
        return {"ok": HANDLER[req["fn"][len("wert."):]](req)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}
