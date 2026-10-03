"""Paritaets-Orakel fuer `domain::PyWert` (rust/parity/tests/wert_paritaet.rs): beantwortet
`{"fn": "wert.<name>", ...}` mit echtem CPython, ohne ein Produktmodul zu laden.

- `wert.py_eq`: `{"paare": [[a, b], ...]}`, jede Seite im Draht-Format von `wert()`. Antwort
  `{"ok": [bool, ...]}`, je Paar `a == b`.
- `wert.py_eq_json`: `{"paare": [[text, text], ...]}`; jede Seite geht durch `json.loads`, wie
  der Lader der Produktpfade. Antwort wie oben, `{"err": "<Klasse>"}` je Paar, wenn ein Text nicht laedt.
- `wert.int`: `{"werte": [wert, ...]}`. Antwort `{"ok": [<Antwort>, ...]}`, je Wert `{"ok": "<dezimal>"}`
  fuer `int(x)` oder `{"err": "<Klasse>", "msg": "<str(exc)>"}`.
- `wert.int_text_sweep`: `int(c + "7" + c)` fuer jeden Skalarwert `c`, kompakt (siehe `_int_text_sweep`).
- `wert.truthy`, `typname`, `gt_null`, `int_mit_bool`, `int_ohne_bool`, `zahl_ohne_bool`, `oder_null`:
  `{"werte": [wert, ...]}` wie `wert.int`; je Wert `bool(x)`, `type(x).__name__`, `x > 0`, `x if
  isinstance(x, int) else None`, `... and not isinstance(x, bool)`, `isinstance(x, (int, float)) and not
  isinstance(x, bool)` (als Wahrheitswert) und `x or 0` (im Draht-Format, siehe `kodiere`).
- `wert.repr`, `wert.py_str`: `{"werte": [wert, ...]}` wie `wert.int`; je Wert `repr(x)` und `str(x)`
  als Text.
- `wert.repr_text_sweep`: `repr(c)` fuer jeden Skalarwert `c`, kompakt (siehe `_repr_text_sweep`).

Draht-Format (kein Wert reist als JSON-Zahl, denn `json.loads` kennt weder NaN noch `u64`-exakte
Floats im Rust-Sinn): `null`, `true`/`false`, Text und Liste wie JSON; `{"i": "<dezimal>"}` eine
`int`; `{"f": "<16 Hex>"}` die IEEE-754-Bits einer `float` (NaN, -0.0, inf); `{"o": [[schluessel,
wert], ...]}` ein `dict`. Jeder Aufruf baut seine Objekte neu: zwei NaN sind nie dasselbe Objekt, die
Identitaetsabkuerzung von `list.__eq__` greift also nicht (das ponytail an `PyWert::py_eq`).

Antwort: `{"ok": <ergebnis>}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}`.
"""
from __future__ import annotations

import json
import re
import struct
import unicodedata
from pathlib import Path


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


def kodiere(x):
    """Umkehr von `wert()`: ein Python-Wert im Draht-Format."""
    if x is None or isinstance(x, (bool, str)):
        return x
    if isinstance(x, int):
        return {"i": str(x)}
    if isinstance(x, float):
        return {"f": struct.pack(">d", x).hex()}
    if isinstance(x, list):
        return [kodiere(v) for v in x]
    return {"o": [[k, kodiere(v)] for k, v in x.items()]}


def _truthy(req: dict) -> list:
    return _je_wert(req, bool)


def _typname(req: dict) -> list:
    return _je_wert(req, lambda x: type(x).__name__)


def _gt_null(req: dict) -> list:
    return _je_wert(req, lambda x: x > 0)


def _int_mit_bool(req: dict) -> list:
    return _je_wert(req, lambda x: str(int(x)) if isinstance(x, int) else None)


def _int_ohne_bool(req: dict) -> list:
    return _je_wert(req, lambda x: str(int(x)) if isinstance(x, int) and not isinstance(x, bool) else None)


def _zahl_ohne_bool(req: dict) -> list:
    return _je_wert(req, lambda x: isinstance(x, (int, float)) and not isinstance(x, bool))


def _oder_null(req: dict) -> list:
    return _je_wert(req, lambda x: kodiere(x or 0))


def _repr(req: dict) -> list:
    return _je_wert(req, repr)


def _py_str(req: dict) -> list:
    return _je_wert(req, str)


def _repr_text_sweep(req: dict) -> dict:
    """`repr(c)` fuer jeden Skalarwert `c` (ohne Surrogate), ohne 1,1 Mio. Antworten zu senden.

    - `nicht_druckbar`: Laeufe `[von, bis, kategorie]` der `c` mit `c.isprintable() == False`, nach
      `unicodedata.category`. Genau diese `c` escapet `repr`.
    - `nicht_escaped`: die `c`, die nicht druckbar sind und trotzdem unveraendert in `repr(c)` stehen
      (leer: `repr` escapet jedes nicht druckbare Zeichen).
    - `ausnahmen`: `[[codepunkt, repr(c)]]` fuer jedes druckbare `c`, dessen `repr` nicht `'c'` ist
      (`'` waehlt die Anfuehrungszeichen `"`, `\\` wird verdoppelt).
    """
    laeufe, nicht_escaped, ausnahmen = [], [], []
    for cp in range(0x110000):
        if 0xD800 <= cp < 0xE000:
            continue
        c = chr(cp)
        r = repr(c)
        if c.isprintable():
            if r != f"'{c}'":
                ausnahmen.append([cp, r])
            continue
        if r == f"'{c}'":
            nicht_escaped.append(cp)
        kat = unicodedata.category(c)
        if laeufe and laeufe[-1][1] == cp - 1 and laeufe[-1][2] == kat:
            laeufe[-1][1] = cp
        else:
            laeufe.append([cp, cp, kat])
    return {"unicode": unicodedata.unidata_version, "nicht_druckbar": laeufe,
            "nicht_escaped": nicht_escaped, "ausnahmen": ausnahmen}


def _repr_stabil(x) -> str:
    """`repr(x)`, aber ein `set` in sortierter Reihenfolge: dessen Reihenfolge haengt vom Hash der Texte ab
    und wechselt von Lauf zu Lauf (PYTHONHASHSEED). Rekursive Werte fallen auf `repr` zurueck."""
    t = type(x)
    if t is set:
        return "{" + ", ".join(sorted(_repr_stabil(e) for e in x)) + "}" if x else "set()"
    if t is list:
        return "[" + ", ".join(map(_repr_stabil, x)) + "]"
    if t is dict:
        return "{" + ", ".join(f"{_repr_stabil(k)}: {_repr_stabil(v)}" for k, v in x.items()) + "}"
    return repr(x)


def _text_ok(s: str) -> bool:
    """Ein Text ohne einzelnes Surrogat: nur er reist als JSON-Text (und existiert als Rust-`String`)."""
    return re.search("[\ud800-\udfff]", s) is None


def _draht_laden(x, schluessel_text=False):
    """Wie `kodiere`, aber ein Wert ohne Draht-Form -- `dict` mit Nicht-Text-Schluessel, `date`, `bytes`,
    `set`, `tuple`, Text mit einzelnem Surrogat -- wird `{"?": repr(x)}`. Rust kann ihn nie treffen: das
    ist die Abweichung. Mit `schluessel_text` werden Schluessel vom Typ `int` zu ihrem Dezimaltext, wie
    `PyWert` sie ablegt (`{20: 1}` wird `{"20": 1}`)."""
    t = type(x)
    if x is None or t is bool or (t is str and _text_ok(x)):
        return x
    if t is int:
        return {"i": str(x)}
    if t is float:
        return {"f": struct.pack(">d", x).hex()}
    if t is list:
        return [_draht_laden(v, schluessel_text) for v in x]
    if t is dict:
        def schluessel(k):
            return str(k) if schluessel_text and type(k) is int else k
        if all(type(schluessel(k)) is str and _text_ok(schluessel(k)) for k in x):
            return {"o": [[schluessel(k), _draht_laden(v, schluessel_text)] for k, v in x.items()]}
    return {"?": _repr_stabil(x)}


def _lader(art: str):
    """Der Lader der Produktpfade: `json.loads` oder `yaml.safe_load` (PyYAML, reiner Python-Lader)."""
    if art == "json":
        return json.loads
    import yaml
    return yaml.safe_load


def _geladen(f, text: str, schluessel_text=False) -> dict:
    """`{"ok": {"w": <Draht>, "r": repr(x)}}` oder die Ausnahme mit Klasse und (gekuerzter) Meldung."""
    try:
        x = f(text)
        try:
            w = _draht_laden(x, schluessel_text)
        except RecursionError:
            w = {"?": "rekursiv"}
        try:
            r = _repr_stabil(x)
        except RecursionError:
            r = repr(x)
        return {"ok": {"w": w, "r": r}}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)[:200]}


def _laden(req: dict) -> list:
    """`{"art": "json"|"yaml", "texte": [text, ...]}`: je Text `json.loads(text)` / `yaml.safe_load(text)`."""
    f = _lader(req["art"])
    return [_geladen(f, t) for t in req["texte"]]


def _laden_dateien(req: dict) -> list:
    """Wie `laden`, aber `{"pfade": [...]}` relativ zur Wurzel des Repos; der Text kommt aus der Datei.
    `"schluessel_text": true` legt `int`-Schluessel als Text ab (siehe `_draht_laden`)."""
    f = _lader(req["art"])
    wurzel = Path(__file__).resolve().parents[2]
    return [_geladen(f, (wurzel / p).read_text(encoding="utf-8"), req.get("schluessel_text", False))
            for p in req["pfade"]]


def _laden_tiefe(req: dict) -> dict:
    """Die groesste Schachtelungstiefe `[[...]]`, die der Lader noch annimmt (bis `req["bis"]`)."""
    f = _lader(req["art"])
    tiefste = 0
    for d in range(1, req["bis"] + 1):
        if "ok" not in _geladen(f, "[" * d + "]" * d):
            break
        tiefste = d
    return {"tiefste": tiefste}


def _gleit_texte(req: dict) -> list:
    """Dezimaltexte `d.ddde+XX`, bei denen die Rundung zur naechsten `double` heikel ist; gueltig fuer
    `json.loads` UND `yaml.safe_load` (PyYAML verlangt Punkt und Vorzeichen im Exponenten). Fest gesaet.

    - `kurz`: kuerzester Text von Zufalls-Bitmustern (er muss genau dieselben Bits ergeben).
    - `lang`: 16 bis 30 Ziffern, Exponent -325..308.
    - `mitte`: der exakte Mittelpunkt zweier Nachbarn, ein Stueck darueber und darunter: dort entscheidet
      die gerade Mantisse (Gleichstand) und jede Ungenauigkeit der Rundung sichtbar.
    - `fest`: bekannte Randfaelle (2.2250738585072012e-308, halbe kleinste Subnormale, `f64::MAX` + Hauch).
    """
    import random
    from decimal import Decimal, getcontext
    getcontext().prec = 1500
    rnd = random.Random(20261002)

    def text(ziffern: str, exp10: int) -> str:
        """`ziffern` mit Punkt nach der ersten Stelle; `exp10` ist der Exponent der ersten Stelle."""
        ziffern = ziffern.rstrip("0") or "0"
        return f"{ziffern[0]}.{ziffern[1:] or '0'}e{exp10:+d}"

    def aus_decimal(d: Decimal) -> str:
        sign, ziffern, exp = d.as_tuple()
        z = "".join(map(str, ziffern))
        return text(z, exp + len(z) - 1)

    kurz, lang, mitte = [], [], []
    while len(kurz) < 3000:
        f = struct.unpack(">d", struct.pack(">Q", rnd.getrandbits(63)))[0]
        if f == f and abs(f) != float("inf"):
            kurz.append(aus_decimal(Decimal(repr(f))))
    for _ in range(3000):
        n = rnd.randint(16, 30)
        rest = "".join(rnd.choice("0123456789") for _ in range(n - 1))
        lang.append(text(rnd.choice("123456789") + rest, rnd.randint(-325, 308)))
    while len(mitte) < 1200:
        f = rnd.uniform(1.0, 2.0) * 2.0 ** rnd.randint(-40, 40)
        nach = struct.unpack(">d", struct.pack(">Q", struct.unpack(">Q", struct.pack(">d", f))[0] + 1))[0]
        m = (Decimal(f) + Decimal(nach)) / 2
        ulp = Decimal(nach) - Decimal(f)
        mitte += [aus_decimal(m), aus_decimal(m + ulp / 10 ** 6), aus_decimal(m - ulp / 10 ** 6)]
    fest = ["2.2250738585072011e-308", "2.2250738585072012e-308", "2.2250738585072014e-308",
            "4.9406564584124654e-324", "2.4703282292062327e-324", "2.4703282292062328e-324",
            "2.4703282292062330e-324", "1.7976931348623157e+308", "1.7976931348623158e+308",
            "1.7976931348623159e+308", "1.0e+23", "9.999999999999999e+22", "8.41e+21", "1.0e+22",
            "1.0e-22", "7.0e-10", "3.0e-1", "9.007199254740993e+15", "9.007199254740992e+15",
            "9.007199254740994e+15", "1.8446744073709552e+19", "9.223372036854775807e+18",
            "9.223372036854775808e+18", "5.0e-324", "0.0e+0", "-0.0e+0", "1.0e+0"]
    return kurz + lang + mitte + fest


HANDLER = {
    "laden": _laden,
    "laden_dateien": _laden_dateien,
    "laden_tiefe": _laden_tiefe,
    "laden_gleit_texte": _gleit_texte,
    "py_eq": _py_eq,
    "py_eq_json": _py_eq_json,
    "int": _int,
    "int_text_sweep": _int_text_sweep,
    "truthy": _truthy,
    "typname": _typname,
    "gt_null": _gt_null,
    "int_mit_bool": _int_mit_bool,
    "int_ohne_bool": _int_ohne_bool,
    "zahl_ohne_bool": _zahl_ohne_bool,
    "oder_null": _oder_null,
    "repr": _repr,
    "py_str": _py_str,
    "repr_text_sweep": _repr_text_sweep,
}


def handle(req: dict) -> dict:
    try:
        return {"ok": HANDLER[req["fn"][len("wert."):]](req)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}
