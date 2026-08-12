"""Gate für den Vorjahr-Übernahme-Writer (produkt/import/vorjahr_writer.py) + den Store-Guard
(^import:vorjahr). Deterministisch, NULL LLM.

Prüft: (1) Store-Guard fail-closed — import:vorjahr kann NIE bestaetigt schreiben; (2) Übertrag der
vorjahr-flagged Felder als vorlaeufig+herkunft=vorjahr; (3) fail-closed-default — nicht-flagged Feld wird
NICHT übertragen; (4) K2 — nur BESTÄTIGTE Vorjahres-Werte werden übernommen; (5) kein Überschreiben eines
schon aktiven Events; (6) signal_1-Vorjahr-Ref-Form; (7) Ratsche — Bestand/Zustand-Felder (Wert ändert
sich durch Verrechnung/Verbrauch im Vorjahr, z.B. verlustvortrag_bestand) tragen NIE das vorjahr-Flag,
weil die generische Übernahme (Zeile 47, `wert = vf["wert"]`) wortwörtlich kopiert.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/store", "produkt/traverser", "produkt/import"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST          # noqa: E402
import traverser as TR      # noqa: E402
import vorjahr_writer as VW  # noqa: E402

H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
VJ = {"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
TS = "2026-07-18T14:00:00+00:00"


def _bestaetigt(store, fid, wert):
    ST.append_event(store, feld_id=fid, wert=wert, zustand="bestaetigt", herkunft=H,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": f"ok@{fid}"}, ts=TS)


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


# ---- (1) Store-Guard fail-closed ---------------------------------------------

def test_guard_import_vorjahr_kann_nie_bestaetigen():
    s = ST.leerer_store(2025, fall_id="vj-guard")
    with pytest.raises(ValueError, match="import:vorjahr"):
        ST.append_event(s, feld_id="veranlagung", wert="zusammen", zustand="bestaetigt",
                        herkunft=VJ, schreiber="import:vorjahr",
                        signal={"signal_1": {"typ": "vorjahr"}, "signal_2": "erschlichen"}, ts=TS)


def test_guard_import_vorjahr_falsche_herkunft():
    s = ST.leerer_store(2025, fall_id="vj-guard2")
    with pytest.raises(ValueError, match="import:vorjahr"):
        ST.append_event(s, feld_id="veranlagung", wert="zusammen", zustand="vorlaeufig",
                        herkunft=H, schreiber="import:vorjahr",   # herkunft=laie statt vorjahr
                        signal={"signal_1": {"typ": "vorjahr"}, "signal_2": None}, ts=TS)


# ---- (2)+(6) Übertrag als vorlaeufig + signal_1-Ref --------------------------

def test_uebernahme_flagged_als_vorlaeufig(bindung):
    vj = ST.leerer_store(2024, fall_id="vj-2024")
    _bestaetigt(vj, "veranlagung", "zusammen")           # uebernehmbar
    _bestaetigt(vj, "bruttoarbeitslohn", 4000000)        # vorschlag
    vj_felder, _ = ST.materialisiere(vj)
    neu = ST.leerer_store(2025, fall_id="neu")
    n = VW.uebernehme_vorjahr(neu, vj_felder, bindung, vorjahr_vz=2024, ts=TS)
    nf, _ = ST.materialisiere(neu)
    assert n >= 2
    assert nf["veranlagung"]["zustand"] == "vorlaeufig"                    # NIE bestaetigt
    assert nf["veranlagung"]["herkunft"]["herkunft"] == "vorjahr"
    assert nf["bruttoarbeitslohn"]["wert"] == 4000000
    # signal_1-Vorjahr-Ref
    ev = ST._aktives(neu)["bruttoarbeitslohn"]
    assert ev["signal"]["signal_1"]["vz"] == 2024 and ev["signal"]["signal_1"]["quell_wert"] == 4000000
    assert ev["signal"]["signal_2"] is None


# ---- (3) fail-closed-default: nicht-flagged wird NICHT übertragen ------------

def test_nicht_flagged_feld_nicht_uebertragen(bindung):
    # vv_werbungskosten ist non-askable + trägt KEIN vorjahr-Flag -> darf nicht übertragen werden
    assert bindung.get("vv_werbungskosten", {}).get("vorjahr") is None
    vj = ST.leerer_store(2024, fall_id="vj-nf")
    _bestaetigt(vj, "vv_werbungskosten", 800000)
    vj_felder, _ = ST.materialisiere(vj)
    neu = ST.leerer_store(2025, fall_id="neu-nf")
    VW.uebernehme_vorjahr(neu, vj_felder, bindung, vorjahr_vz=2024, ts=TS)
    assert "vv_werbungskosten" not in ST._aktives(neu)


# ---- (4) K2: nur bestätigte Vorjahres-Werte -----------------------------------

def test_nur_bestaetigte_vorjahres_werte(bindung):
    vj = ST.leerer_store(2024, fall_id="vj-vorl")
    # ein NUR vorlaeufiger Vorjahres-Wert (z.B. selbst unbestätigt geblieben) -> kein Vorschlag
    ST.append_event(vj, feld_id="bruttoarbeitslohn", wert=4000000, zustand="vorlaeufig", herkunft=H,
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": None}, ts=TS)
    vj_felder, _ = ST.materialisiere(vj)
    neu = ST.leerer_store(2025, fall_id="neu-vorl")
    n = VW.uebernehme_vorjahr(neu, vj_felder, bindung, vorjahr_vz=2024, ts=TS)
    assert n == 0 and "bruttoarbeitslohn" not in ST._aktives(neu)


# ---- (5) kein Überschreiben eines aktiven Events ------------------------------

def test_kein_ueberschreiben_aktives_event(bindung):
    vj = ST.leerer_store(2024, fall_id="vj-ov")
    _bestaetigt(vj, "bruttoarbeitslohn", 4000000)
    vj_felder, _ = ST.materialisiere(vj)
    neu = ST.leerer_store(2025, fall_id="neu-ov")
    _bestaetigt(neu, "bruttoarbeitslohn", 4500000)       # Nutzer hat schon einen Wert
    VW.uebernehme_vorjahr(neu, vj_felder, bindung, vorjahr_vz=2024, ts=TS)
    nf, _ = ST.materialisiere(neu)
    assert nf["bruttoarbeitslohn"]["wert"] == 4500000 and nf["bruttoarbeitslohn"]["zustand"] == "bestaetigt"


# ---- (7) Ratsche: Bestand/Zustand-Felder duerfen NIE das vorjahr-Flag tragen -----

BESTAND_FELDER_OHNE_VORJAHR = {
    "verlustvortrag_bestand": (
        "§ 10d Abs. 4 EStG: der Bestand aendert sich durch Verrechnung im Vorjahr. Die generische "
        "Uebernahme kopiert wortwoertlich (vorjahr_writer.py:47) — mit Flag wuerde sie den "
        "unverrechneten Altwert als Vorschlag zeigen statt des tatsaechlich verbleibenden Bestands."),
    "rentner_freibetrag_erstmalig": (
        "§ 16 Abs. 4 S. 2 EStG: der Freibetrag steht nur einmal im Leben zu. Wird er dieses Jahr "
        "gewaehrt, muss die Antwort naechstes Jahr auf false kippen — eine literale Kopie zeigt ihn "
        "faelschlich weiter verfuegbar."),
    "am_afa_ist_anschaffungsjahr": (
        "§ 7 Abs. 1 EStG: die Frage ('DIESES Jahr angeschafft?') ist per Definition in jedem "
        "Folgejahr falsch, wenn kopiert — die AfA wuerde wieder anteilig statt voll fuer das Jahr "
        "berechnet, Werbungskosten dauerhaft zu niedrig."),
    "rentner_freibetrag_erstmalig_partner": (
        "§ 16 Abs. 4 S. 2 EStG, gilt PRO Steuerpflichtigem: dieselbe Begruendung wie bei "
        "rentner_freibetrag_erstmalig — der Freibetrag steht dem Partner nur einmal im Leben zu, "
        "eine literale Kopie wuerde ihn faelschlich weiter verfuegbar zeigen, nachdem er einmal "
        "gewaehrt wurde."),
}


def test_bestand_felder_tragen_kein_vorjahr_flag(bindung):
    """Ratsche gegen §10d-Sweep 2026-08-12 (BACKLOG commit-nur-defer-sweep-2026-08-10, Fund 3).

    vorjahr_writer.py kopiert den Vorjahreswert wortwoertlich (Zeile 47, `wert = vf["wert"]`). Das
    ist bei Stammdaten (Name, Geburtsjahr, Renten-Beginn-Jahr) richtig, aber falsch bei Feldern,
    deren Wert sich durch Verrechnung/Verbrauch im Vorjahr veraendert hat. Faellt dieser Test rot,
    wurde das vorjahr-Flag fuer eines der u.g. Felder wieder gesetzt — der Grund im Dict steht,
    warum das falsch waere.
    """
    for fid, grund in BESTAND_FELDER_OHNE_VORJAHR.items():
        vj = bindung.get(fid, {}).get("vorjahr")
        assert vj is None, f"{fid} traegt wieder ein vorjahr-Flag ({vj!r}). {grund}"
