"""Statischer Erreichbarkeits-Gate: Jedes askable Feld MUSS in SCHEIBEN.felder registriert
sein ODER als geplante Lücke gewhitelistet. Fängt totes Wiring (Feld in global-bindung +
Reader, aber nicht POSTbar — Bug-Klasse §51a-KiSt / §35c / §10-Realsplitting).

Kein LLM, rein statisch (Bindung + api.SCHEIBEN). Rot-Erwartung aktuell: kist_konfession
und kist_bundesland (echter Bug, dev-1 fixt). Wird GRÜN sobald dev-1 die 2 Felder in
SCHEIBEN.felder aufnimmt.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api          # noqa: E402 — SCHEIBEN, _datei_felder
import traverser as TR  # noqa: E402 — lade_bindung

# Geplante Lücken: Felder in globaler Bindung (askable=true) aber NICHT in SCHEIBEN.felder,
# weil ihr Feature nie gestartet / durch aggregierte Felder ersetzt wurde.
# Jedes Feld hier wurde auf Reader-Freiheit in produkt/ geprüft (kein .py-Reader).
GEPLANTE_LUECKEN = {
    # §33/§35a Sub-Qualifikation (Ring rechnet via Aggregat-Felder agb_aufwendungen,
    # hh_dienstleistungen, hh_handwerker_arbeitskosten):
    "agb_zwangslaeufig",
    "agb_notwendig_angemessen",
    "hh_in_eu_ewr",
    "hh_handwerker_keine_foerderung",
    # §21-WK: 4 Teil-Accessoren statt Laien-Brutto-Eingabe:
    "vv_werbungskosten",
    # Kind-Modell (§32/§31/§24b) nie gestartet (Julius-Cap):
    "fam_kinder_im_haushalt",
    "fam_kinder_beruecksichtigt",
    "kind_idnr",
    "kind_kindschaftsverhaeltnis_a",
    "kind_kindschaftsverhaeltnis_b",
    "kind_kindschaftsverh_zeitraum_a",
    "kind_kindschaftsverh_zeitraum_b",
    # §16 Abs.4 Veräußerungsfreibetrag — Geltungsbedingungen (Alter/Einmaligkeit), in
    # catala_p16_4_freibetrag pre-condition-abgefragt aber kein Ring-Laien-Input;
    # rentner_veraeusserungsgewinn selbst ist in SCHEIBEN.felder (RENTNER_GEWINN):
    "rentner_alter_55_oder_berufsunfaehig",
    "rentner_freibetrag_erstmalig",
    # §24a Altersentlastungsbetrag-Rentner — Kohorten-Schlüssel (Rentner-nicht-§24a,
    # structural 0 da §22-Leibrente §24a S.2 ausgeschlossen); gesamt-only via geburtsjahr:
    "rentner_alter_64_erfuellt",
}


def _scheiben_felder_union() -> set[str]:
    """Vereinigung aller feld_ids aus allen SCHEIBEN[x]["felder"] (explizit oder via felder_datei)."""
    union: set[str] = set()
    for cfg in api.SCHEIBEN.values():
        if cfg["felder"] is not None:
            union.update(cfg["felder"])
        else:
            union.update(api._datei_felder(cfg["felder_datei"]))
    return union


def askable_felder() -> set[str]:
    """Alle askable feld_ids aus der globalen Bindung."""
    b = TR.lade_bindung()
    return {fid for fid, e in b.items() if e.get("askable") is True}


def test_alle_askable_felder_erreichbar():
    askable = askable_felder()
    scheiben = _scheiben_felder_union()
    verstoesse = askable - scheiben - GEPLANTE_LUECKEN

    msg = (
        f"{len(verstoesse)} askable Felder weder in SCHEIBEN.felder noch gewhitelistet"
        f" → nicht POSTbar (totes Wiring, siehe §51a-KiSt / §35c):\n"
        + "\n".join(sorted(verstoesse))
        if verstoesse else ""
    )
    assert not verstoesse, msg
