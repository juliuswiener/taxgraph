"""Paritaets-Orakel fuer `domain::PyWert` (rust/parity/tests/wert_paritaet.rs): beantwortet
`{"fn": "wert.<name>", ...}` mit echtem CPython, ohne ein Produktmodul zu laden.

- `wert.py_eq`: `{"paare": [[a, b], ...]}`, jede Seite im Draht-Format von `wert()`. Antwort
  `{"ok": [bool, ...]}`, je Paar `a == b`.
- `wert.py_eq_json`: `{"paare": [[text, text], ...]}`; jede Seite geht durch `json.loads`, wie
  der Lader der Produktpfade. Antwort wie oben, `{"err": "<Klasse>"}` je Paar, wenn ein Text nicht laedt.
- `wert.int`: `{"werte": [wert, ...]}`. Antwort `{"ok": [<Antwort>, ...]}`, je Wert `{"ok": "<dezimal>"}`
  fuer `int(x)` oder `{"err": "<Klasse>", "msg": "<str(exc)>"}`.
- `wert.int_text_sweep`: `int(c + "7" + c)` fuer jeden Skalarwert `c`, kompakt (siehe `_int_text_sweep`).

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
import unicodedata


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


def _je_wert(req: dict, f) -> list:
    """Eine Antwort je Wert: `{"ok": f(x)}` oder die Ausnahme mit Klasse und `str(exc)`."""
    out = []
    for k in req["werte"]:
        try:
            out.append({"ok": f(wert(k))})
        except Exception as exc:  # noqa: BLE001 -- die Klasse und der Text gehoeren zur Antwort
            out.append({"err": type(exc).__name__, "msg": str(exc)})
    return out


def _int(req: dict) -> list:
    return _je_wert(req, lambda x: str(int(x)))


def _int_text_sweep(req: dict) -> dict:
    """`int(c + "7" + c)` fuer jeden Skalarwert `c` (ohne Surrogate), ohne 1,1 Mio. Antworten zu senden.

    - `ok`: `[[codepunkt, "<dezimal>"], ...]` fuer jeden Text, den `int` annimmt.
    - `nicht_druckbar`: Bereiche `[von, bis]` der `c`, fuer die `c.isprintable()` falsch ist. Dort
      traegt die Fehlermeldung ein `repr`, das CPython escapet (Cf, Co, Cn).
    - `vorlage_ausnahmen`: `[[codepunkt, "<str(exc)>"], ...]` fuer jeden Fehler eines druckbaren `c`,
      dessen Text nicht `invalid literal for int() with base 10: 'c7c'` ist (`'` und `\\` im `repr`).
    - `klassen`: Zahl der Fehler je Ausnahmeklasse.
    """
    ok, klassen, bereiche, ausnahmen = [], {}, [], []
    for cp in range(0x110000):
        if 0xD800 <= cp < 0xE000:
            continue
        c = chr(cp)
        if not c.isprintable():
            if bereiche and bereiche[-1][1] == cp - 1:
                bereiche[-1][1] = cp
            else:
                bereiche.append([cp, cp])
        try:
            ok.append([cp, str(int(c + "7" + c))])
        except Exception as exc:  # noqa: BLE001
            klassen[type(exc).__name__] = klassen.get(type(exc).__name__, 0) + 1
            if c.isprintable() and str(exc) != f"invalid literal for int() with base 10: '{c}7{c}'":
                ausnahmen.append([cp, str(exc)])
    return {"unicode": unicodedata.unidata_version, "ok": ok, "nicht_druckbar": bereiche,
            "vorlage_ausnahmen": ausnahmen, "klassen": klassen}


HANDLER = {"py_eq": _py_eq, "py_eq_json": _py_eq_json, "int": _int, "int_text_sweep": _int_text_sweep}


def handle(req: dict) -> dict:
    try:
        return {"ok": HANDLER[req["fn"][len("wert."):]](req)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}
