"""Paritaets-Orakel fuer den ELSTER-Zeichensatz (`rust/parity/tests/store_zeichensatz_paritaet.rs`):
beantwortet `{"fn": "zeichensatz.<name>", ...}` ueber `produkt/store/zeichensatz.py`, die Python-Referenz
der Auflage Z (Ticket `elster-zeichensatz-strenger-als-xml`).

Zwei Fragen:
  - `erlaubt`   `von`, `bis` (Codepunkte, eingeschlossen): ein Text aus je einem Zeichen pro Codepunkt, `1` =
                `erstes_unerlaubtes_zeichen` findet nichts, `0` = es findet das Zeichen, `-` = Ersatzzeichen-
                Codepunkt (kein Unicode-Skalarwert, in Rust kein `char`). Ein Text statt einer Liste, weil
                U+0000..U+10FFFF sonst Megabytes JSON waeren.
  - `meldungen` `texte`, `feld_id`, `element`: je Text `null` (alles erlaubt) oder
                `{"feld": <Meldung der Regel>, "element": <Meldung der XML-Erzeugung>, "store": <was
                `append_event` mit der echten Bindung wirft>}`. `store` ist `{"klasse": "ZeichensatzVerletzt"
                | "Typ" | "Sonstig", "msg": <Wortlaut>}` -- der Weg, den jeder Schreiber nimmt, nicht nur
                die Hilfsfunktion.

Antwort: `{"ok": <ergebnis>}` oder `{"err": "<Ausnahmeklasse>", "msg": "<str(exc)>"}` (wie `feld_kennung_oracle`).
"""
from __future__ import annotations

import os
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
_M: dict = {}
_HERKUNFT = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _module() -> dict:
    if not _M:
        for sub in ("produkt/store", "produkt/traverser", "produkt/mapping"):
            p = os.path.join(ROOT, sub)
            if p not in sys.path:
                sys.path.insert(0, p)
        import store as ST  # noqa: E402
        import traverser as TR  # noqa: E402
        import zeichensatz as ZS  # noqa: E402
        _M.update(ST=ST, ZS=ZS, bindung=TR.lade_bindung())
    return _M


def _erlaubt(req: dict) -> str:
    ZS = _module()["ZS"]
    return "".join(
        "-" if 0xD800 <= cp <= 0xDFFF else "1" if ZS.erstes_unerlaubtes_zeichen(chr(cp)) is None else "0"
        for cp in range(req["von"], req["bis"] + 1))


def _store(feld_id: str, text: str) -> dict | None:
    """Was `append_event` mit der echten Bindung auf einen leeren Store wirft (sonst `null`)."""
    m = _module()
    s = m["ST"].leerer_store(2025, fall_id="zeichensatz-paritaet")
    try:
        m["ST"].append_event(s, feld_id=feld_id, wert=text, zustand="bestaetigt", herkunft=dict(_HERKUNFT),
                             schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"},
                             ts="2026-10-03T12:00:00+00:00", bindung=m["bindung"])
    except ValueError as exc:
        msg = str(exc)
        klasse = ("ZeichensatzVerletzt" if msg.startswith("fail-closed (Zeichensatz):")
                  else "Typ" if msg.startswith("fail-closed (Typ):") else "Sonstig")
        return {"klasse": klasse, "msg": msg}
    return None


def _meldungen(req: dict) -> list:
    ZS = _module()["ZS"]
    out = []
    for text in req["texte"]:
        z = ZS.erstes_unerlaubtes_zeichen(text)
        out.append(None if z is None else {
            "feld": ZS.feld_meldung(req["feld_id"], z),
            "element": ZS.element_meldung(req["element"], text),
            "store": _store(req["feld_id"], text),
        })
    return out


HANDLER = {"erlaubt": _erlaubt, "meldungen": _meldungen}


def handle(req: dict) -> dict:
    try:
        return {"ok": HANDLER[req["fn"][len("zeichensatz."):]](req)}
    except Exception as exc:  # noqa: BLE001 -- Fehlerparitaet braucht jeden Typ
        return {"err": type(exc).__name__, "msg": str(exc)}
