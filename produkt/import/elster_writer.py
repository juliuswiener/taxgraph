"""eDaten-Übernahme-Writer (import:elster — vierter Store-Writer nach laie + import:beleg + import:vorjahr).
NULL LLM.

Rechtsgrund: § 150 Abs. 7 S. 2 AO — von der Finanzverwaltung elektronisch übermittelte Daten (eDaten)
„gelten als Angaben des Steuerpflichtigen, soweit er nicht abweichende Angaben macht".

Invariant: eine EIGENE (abweichende) Angabe des Nutzers hat Vorrang. Umgesetzt via One-Active-Event:
der Writer überschreibt NIE ein bereits aktives Event.

Posture (§ 150 Abs. 7 S. 2 AO — auto-bestätigt): die Finanzverwaltungs-Daten gelten als Angaben des
Steuerpflichtigen. Der Writer schreibt zustand="bestaetigt" + signal_2="edaten_uebermittelt".

signal_1 = {typ: "edaten", quell_feld_id, quell_wert, kategorie} — die Justification-Kante zeigt
„von der Finanzverwaltung übermittelt".
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "store"))
import store as ST   # noqa: E402


def uebernehme_edaten(store: dict, edaten_record: list, *, ts: str | None = None) -> int:
    """Übernimmt eDaten in den Store (bestätigt, One-Active-Event).

    edaten_record = Liste von {"feld_id": ..., "wert": ..., "kategorie": ...}
    (die von der Finanzverwaltung übermittelten Datensätze).

    Überträgt je Feld ein BESTÄTIGTES Event in den Store — aber NUR, wenn dort noch KEIN aktives
    Event für das feld_id liegt (One-Active-Event: eine abweichende Angabe des Stpfl. hat Vorrang,
    § 150 Abs. 7 S. 2 AO). Gibt die Anzahl übertragener Felder zurück.
    """
    aktiv = ST._aktives(store)
    n = 0
    for rec in edaten_record:
        fid = rec["feld_id"]
        if fid in aktiv:
            continue
        wert = rec["wert"]
        sig1 = {
            "typ": "edaten",
            "quell_feld_id": fid,
            "quell_wert": wert,
            "kategorie": rec.get("kategorie"),
        }
        ST.append_event(
            store,
            feld_id=fid,
            wert=wert,
            zustand="bestaetigt",
            herkunft={
                "herkunft": "edaten",
                "pruef_tiefe": "amtlich",
                "haftung": "amt",
            },
            schreiber="import:elster",
            signal={"signal_1": sig1, "signal_2": "edaten_uebermittelt"},
            ts=ts,
        )
        n += 1
    return n
