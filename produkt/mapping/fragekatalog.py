"""Fragekatalog aus Bindungs-YAMLs — extrahiert fragetext_laie + hilfe_kurz für UI. NULL LLM.

lade_fragekatalog() -> dict {feld_id: {fragetext_laie, hilfe_kurz, typ, einheit, ...}}
export_fragekatalog(path) -> YAML-Datei
"""
from __future__ import annotations

import os
from functools import lru_cache

import yaml

_BINDUNG_DIR = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "bindung")


@lru_cache(maxsize=1)
def lade_fragekatalog() -> dict[str, dict]:
    """Alle Bindungs-YAMLs einlesen -> {feld_id: {eintrag}} mit fragekatalog-relevanten Feldern.

    Jeder Eintrag enthält:
      - fragetext_laie
      - hilfe_kurz
      - typ, einheit
      - beispielwert, bereich (falls vorhanden)
      - scheibe (aus der YAML-Kopfzeile)
      - quelle_regel_id (aus quelle.regel_id)

    Nur askable=true-Felder mit fragetext_laie werden aufgenommen (Lücken und
    Nicht-Laien-Felder wie interne Signatur-Slots haben keinen Fragebedarf).
    """
    if not os.path.isdir(_BINDUNG_DIR):
        return {}

    katalog: dict[str, dict] = {}
    for fname in sorted(os.listdir(_BINDUNG_DIR)):
        if not fname.endswith(".yaml"):
            continue
        fpath = os.path.join(_BINDUNG_DIR, fname)
        try:
            with open(fpath, encoding="utf-8") as f:
                data = yaml.safe_load(f)
        except Exception:
            continue
        if not isinstance(data, dict):
            continue
        scheibe = data.get("scheibe", "")
        bindungen = data.get("bindungen")
        if not isinstance(bindungen, list):
            continue
        for e in bindungen:
            if not isinstance(e, dict):
                continue
            fid = e.get("feld_id")
            if not fid or not isinstance(fid, str):
                continue
            ft = e.get("fragetext_laie")
            hk = e.get("hilfe_kurz")
            if not ft or not hk:        # nur Laien-askable-Felder mit Hilfetext
                continue
            eintrag = {
                "fragetext_laie": ft,
                "hilfe_kurz": hk,
                "typ": e.get("typ"),
                "einheit": e.get("einheit"),
                "beispielwert": e.get("beispielwert"),
                "bereich": e.get("bereich"),
                "scheibe": scheibe,
                "quelle_regel_id": _regel_id(e.get("quelle")),
            }
            katalog[fid] = eintrag
    return katalog


def _regel_id(quelle) -> str | None:
    if isinstance(quelle, dict):
        return quelle.get("regel_id")
    return None


def export_fragekatalog(path: str) -> None:
    """Fragekatalog als YAML exportieren (sortiert nach feld_id)."""
    katalog = lade_fragekatalog()
    # YAML-Blöcke pro feld_id, ohne überflüssige Typ-Annotationen
    with open(path, "w", encoding="utf-8") as f:
        yaml.dump(
            {fid: e for fid, e in sorted(katalog.items())},
            f,
            allow_unicode=True,
            default_flow_style=False,
            width=120,
            sort_keys=True,
        )
