#!/usr/bin/env python3
"""Python-Erwartung fuer die festen Funktionsfaelle in `rust/bescheid/tests/*_hermetisch.rs` und `rust/elster/tests/*_hermetisch.rs`.

WARUM ES DIESE DATEI GIBT. Die hermetischen Tests dort laufen ohne `PARITY=1` und ohne Python; ihre Erwartungswerte sind
eingefroren. Dieses Skript rechnet sie nach: es baut aus einer kurzen Ereignisliste denselben Store wie
`bescheid::testhilfe::store` und fragt `tools/parity/bescheid_oracle` (Praefix `bescheid.`, Python-Referenz
`produkt/bescheid/*.py`) oder `tools/parity/elster_oracle` (Praefix `elster.`). Es ist das Gegenstueck zu
`kette_erwartung.py`, das ganze Faelle ueber die HTTP-Endpunkte rechnet.

Aufruf (aus dem Repo-Wurzelverzeichnis), eine JSON-Anfrage je Zeile auf stdin:

    echo '{"fn":"bescheid.shared_steuer_sonder_agb","vz":2025,"events":[["agb_aufwendungen",500000,true]],
           "args":{"gde":20000,"ausserg":0,"veranlagung":"einzel"}}' | python3 tools/parity/bescheid_erwartung.py

`events`: Liste `[feld_id, wert, bestaetigt]` (wie `(feld_id, wert, bestaetigt)` im Rust-Test); alle weiteren Schluessel
(`fn`, `vz`, `args`, `nur_bestaetigt`, ...) gehen unveraendert an `handle` des gewaehlten Orakels. Ausgabe: die Antwort, eine Zeile je Anfrage.

TEMP-WURZEL: `$TAXGRAPH_DATEN` zeigt auf ein frisches Verzeichnis, das am Ende verschwindet.
SICHERHEIT: alle Werte sind ERFUNDEN (keine echten Steuerdaten).
"""
from __future__ import annotations

import atexit
import json
import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
WURZEL = tempfile.mkdtemp(prefix="bescheid-erwartung-")
atexit.register(shutil.rmtree, WURZEL, ignore_errors=True)
os.environ["TAXGRAPH_DATEN"] = WURZEL      # VOR dem Import

sys.path.insert(0, HERE)
import bescheid_oracle as BESCHEID         # noqa: E402
import elster_oracle as ELSTER             # noqa: E402


def baue_store(events: list, vz: int) -> dict:
    """Wie `bescheid::testhilfe::store`: je Ereignis ein `laie`-Eintrag, bestaetigt mit `signal_2 = ok`."""
    return {"version": 1, "veranlagungszeitraum": vz, "events": [
        {"ts": f"2026-01-01T00:00:{i:02}+00:00", "feld_id": fid, "wert": wert,
         "zustand": "bestaetigt" if best else "vorlaeufig", "event_id": f"e{i}",
         "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
         "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": "ok" if best else None}, "ersetzt": None}
        for i, (fid, wert, best) in enumerate(events)]}


def main() -> int:
    for zeile in sys.stdin:
        if not zeile.strip():
            continue
        req = json.loads(zeile)
        events = req.pop("events", None)
        if events is not None:
            req["store"] = baue_store(events, int(req.get("vz", 2025)))
        orakel = ELSTER if str(req.get("fn", "")).startswith("elster.") else BESCHEID
        print(json.dumps(orakel.handle(req), ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
