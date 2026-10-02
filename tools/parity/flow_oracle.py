"""Paritaets-Orakel fuer `rust/api/src/flow.rs` (Schritt 9c/0e, Suite 18).

Beantwortet `{"fn": "flow.<name>", ...}` ueber `produkt/haut/flow.py`. Jede Anfrage bekommt
- ein FRISCHES Ablageverzeichnis (`audit.AUDIT_DIR = <tmp>`, dieselbe Umlenkung wie
  `schritt8_oracle.py`), damit keine Zeile einer Nachbaranfrage mitgelesen wird,
- den Schalter exakt so, wie die Anfrage ihn nennt (`TAXGRAPH_FLOW`, `TAXGRAPH_KI_DEBUG`; `null`
  heisst: nicht gesetzt). `flow.an()` liest zur Aufrufzeit, also wirkt das sofort.

Eingaben mit Struktur (`inhalt`, `body`, `fragen`) kommen als JSON-TEXT und werden hier mit
`json.loads` gelesen, nie als bereits geparstes Objekt: die Rust-Seite parst denselben Text mit
`serde_json` in `PyWert`, und nur so landet dieselbe Einfuegereihenfolge auf beiden Seiten.

`ts` ist die einzige Zeile, die nicht gleich sein kann (Uhrzeit). Zurueck kommt deshalb
`{"ts_ok": <Form stimmt>, "rest": <Zeile ab dem Komma nach ts>}`.

Antwort: `{"ok": ...}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}` (nur fuer Fehler
ausserhalb von `melde_ui`; dessen ValueError ist Teil des Ergebnisses).
"""
from __future__ import annotations

import json
import os
import re
import shutil
import sys
import tempfile

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_M: dict = {}

# `YYYY-MM-DDTHH:MM:SS` + optional `.ffffff` + `+00:00`
_TS = re.compile(r'\{"ts": "(\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d{6})?\+00:00)", ')


def _module() -> dict:
    if not _M:
        tmp = tempfile.mkdtemp(prefix="flow-orakel-")
        os.environ["TAXGRAPH_AUDIT_DIR"] = tmp
        for sub in ("produkt/store", "produkt/haut"):
            p = os.path.join(ROOT, sub)
            if p not in sys.path:
                sys.path.insert(0, p)
        import audit  # noqa: E402
        import flow  # noqa: E402
        _M.update(AU=audit, FL=flow)
    return _M


def _schalter(req: dict) -> None:
    for name, schluessel in (("TAXGRAPH_FLOW", "flow"), ("TAXGRAPH_KI_DEBUG", "debug")):
        wert = req.get(schluessel)
        if wert is None:
            os.environ.pop(name, None)
        else:
            os.environ[name] = wert


def _ablage(req: dict) -> str:
    """Setzt Schalter und eine frische Ablage; `req["datei_vorher"]` legt eine Datei vorab an."""
    m = _module()
    _schalter(req)
    tmp = tempfile.mkdtemp(prefix="flow-lauf-")
    m["AU"].AUDIT_DIR = tmp
    if req.get("datei_vorher") is not None:
        pfad = os.path.join(tmp, m["FL"].DATEI)
        with open(pfad, "w", encoding="utf-8") as f:
            f.write(req["datei_vorher"])
        os.chmod(pfad, req["modus_vorher"])
    return tmp


def _lies(tmp: str) -> dict | None:
    """Die Datei der Anfrage: `ts` jeder Zeile abgetrennt und auf seine Form geprueft, dazu der
    Modus. `None`, wenn es keine Datei gibt."""
    pfad = os.path.join(tmp, _module()["FL"].DATEI)
    if not os.path.exists(pfad):
        return None
    modus = os.stat(pfad).st_mode & 0o777
    with open(pfad, encoding="utf-8", newline="") as f:
        text = f.read()
    zeilen, ts_ok = [], True
    for z in text.split("\n")[:-1]:
        t = _TS.match(z)
        if t is None:
            ts_ok = False
            zeilen.append(z)
        else:
            zeilen.append(z[t.end() - 2:])  # ab dem Komma nach ts
    return {"ts_ok": ts_ok, "zeilen": zeilen, "endet_mit_zeilenende": text.endswith("\n"),
            "modus": modus}


def _aufraeumen(tmp: str) -> None:
    shutil.rmtree(tmp, ignore_errors=True)


def _an(req: dict) -> bool:
    _module()
    _schalter(req)
    return _M["FL"].an()


def _schreibe(req: dict) -> dict:
    """`flow.schreibe(fall, art, inhalt)` -> die entstandene Datei."""
    tmp = _ablage(req)
    try:
        _M["FL"].schreibe(req.get("fall"), req["art"], json.loads(req["inhalt"]))
        return {"datei": _lies(tmp)}
    finally:
        _aufraeumen(tmp)


def _gekappt(req: dict) -> str:
    _module()
    return json.dumps(_M["FL"].gekappt(json.loads(req["inhalt"]), req["grenze"]),
                      ensure_ascii=False)


def _melde_ui(req: dict) -> dict:
    tmp = _ablage(req)
    try:
        try:
            status, body = _M["FL"].melde_ui(req["fall"], json.loads(req["body"]))
            antwort = {"status": status, "body": body}
        except ValueError as exc:
            antwort = {"err": "ValueError", "msg": str(exc)}
        return {"antwort": antwort, "datei": _lies(tmp)}
    finally:
        _aufraeumen(tmp)


def _ergebnis_notiert(req: dict) -> dict:
    tmp = _ablage(req)
    try:
        _M["FL"].ergebnis_notiert(req["fall"], json.loads(req["obj"]))
        return {"datei": _lies(tmp)}
    finally:
        _aufraeumen(tmp)


def _kopf(req: dict) -> str:
    _module()
    return json.dumps(_M["FL"].kopf_der_queue(json.loads(req["fragen"]), req["wie_viele"]),
                      ensure_ascii=False)


def _ui_arten(_req: dict) -> list:
    return sorted(_module()["FL"].UI_ARTEN)


def _konstanten(_req: dict) -> dict:
    return {"datei": _module()["FL"].DATEI, "max_zeichen": _module()["FL"].MAX_ZEICHEN}


HANDLER = {
    "an": _an,
    "schreibe": _schreibe,
    "gekappt": _gekappt,
    "melde_ui": _melde_ui,
    "ergebnis_notiert": _ergebnis_notiert,
    "kopf": _kopf,
    "ui_arten": _ui_arten,
    "konstanten": _konstanten,
}


def handle(req: dict) -> dict:
    name = req["fn"].split(".", 1)[1]
    try:
        return {"ok": HANDLER[name](req)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}
