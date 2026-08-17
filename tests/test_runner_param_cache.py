"""Parameter-YAML wird pro Prozess genau einmal von Platte gelesen.

Befund (team-lead-Messung): golden/runner.py laed dieselbe Parameter-YAML in JEDEM
api.fragen()-Request neu -- 3051 load_yaml_fh()-Aufrufe, 88 % der Laufzeit in
yaml.load (6444ms -> 790ms mit Cache, Faktor 8,2). Eigene Nachmessung auf kleinerem,
aber realistischem Fall (Scheibe "gesamt", 188 beantwortete Felder): 140
load_yaml_fh()-Aufrufe, ebenfalls 88 % der Laufzeit (siehe golden/runner.py::
_load_yaml_path-Docstring). Parameter-YAML ist statischer Repo-Content je VZ und
aendert sich nie zur Laufzeit -- ein Prozess-Cache ist fachlich korrekt, gleiches
Muster wie produkt/traverser/traverser.py::lade_bindung().

Was hier geprueft wird:
  1. Derselbe Pfad wird ueber viele Aufrufe hinweg nur EINMAL von Platte gelesen
     (cache_info().misses bleibt bei 1, hits waechst mit).
  2. Der Cache liefert inhaltlich dasselbe wie ein direkter, ungecachter Read --
     kein stiller Drift durch das Caching.
  3. Ein realistischer API.fragen()-Lauf auf demselben Fall liest beim zweiten Mal
     KEINE einzige neue Parameter-Datei (misses waechst zwischen den beiden
     Laeufen nicht) -- der eigentliche Nutzerpfad.

Mutationsprobe (2026-08-17, manuell durchgefuehrt, nicht Teil dieser Datei):
Decorator aus _load_yaml_path entfernt, volle Suite gelaufen. Ergebnis im Bericht
an team-lead."""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import runner as RUNNER  # noqa: E402
import store as ST       # noqa: E402
import traverser as TR   # noqa: E402

PFAD_P32A_2024 = os.path.join(ROOT, "params", "2024", "einkommensteuertarif_p32a.yaml")


def test_gleicher_pfad_wird_nur_einmal_von_platte_gelesen():
    RUNNER._load_yaml_path.cache_clear()
    for _ in range(50):
        RUNNER._load_yaml_path(PFAD_P32A_2024)
    info = RUNNER._load_yaml_path.cache_info()
    assert info.misses == 1, f"{info.misses} Platten-Reads statt 1 -- Cache greift nicht."
    assert info.hits == 49, f"nur {info.hits} Cache-Treffer statt 49."


def test_cache_liefert_denselben_inhalt_wie_ungecachter_read():
    """Kein stiller Drift: Cache-Ergebnis == direkter load_yaml_fh()-Read derselben Datei."""
    RUNNER._load_yaml_path.cache_clear()
    gecacht = RUNNER._load_yaml_path(PFAD_P32A_2024)
    roh = RUNNER.load_yaml_fh(open(PFAD_P32A_2024, encoding="utf-8"))
    assert gecacht == roh


def test_zwei_fragen_laeufe_lesen_die_datei_nur_beim_ersten_mal():
    """Der eigentliche Nutzerpfad: zwei API.fragen()-Aufrufe auf demselben Fall duerfen in der
    zweiten Runde keine einzige neue Parameter-Datei von Platte laden (misses bleibt konstant)."""
    RUNNER._load_yaml_path.cache_clear()
    fall_id = "test_runner_param_cache_fragen"
    pfad = API._fall_pfad(fall_id)
    if os.path.exists(pfad):
        os.remove(pfad)
    try:
        store = ST.leerer_store(2025, fall_id=fall_id)
        store["scheibe"] = "gesamt"
        bindung = API._scheibe_bindung(store)
        stamm = {"veranlagung": "zusammen", "bruttoarbeitslohn": 6000000,
                 "geburtsjahr": 1970, "kein_gewinn": False, "kein_kap": False,
                 "kein_vuv": False, "kein_sonstige": False, "kein_kind": False}
        for _ in range(250):
            offen = TR.naechste_fragen(store, bindung)
            if not offen:
                break
            fid = offen[0]
            b = bindung.get(fid, {})
            if fid in stamm:
                wert = stamm[fid]
            elif b.get("typ") in ("bool", "boolean"):
                wert = False
            elif b.get("typ") == "enum":
                werte = b.get("enum_werte") or []
                wert = werte[0] if werte else "x"
            elif b.get("typ") == "string":
                wert = ""
            else:
                wert = 1000
            ST.append_event(store=store, feld_id=fid, wert=wert, zustand="bestaetigt",
                            herkunft={"quelle": "test_runner_param_cache"}, schreiber="ui:laie",
                            signal={"signal_1": None, "signal_2": f"ok@{fid}"},
                            ts="2026-08-17T12:00:00Z")
        API.speichere_fall(fall_id, store)

        API.fragen(fall_id)
        misses_nach_erstem = RUNNER._load_yaml_path.cache_info().misses
        assert misses_nach_erstem > 0, "kein einziger load ausgeloest -- Fall zu leer fuer den Test."

        API.fragen(fall_id)
        misses_nach_zweitem = RUNNER._load_yaml_path.cache_info().misses
        assert misses_nach_zweitem == misses_nach_erstem, (
            f"zweiter api.fragen()-Lauf hat {misses_nach_zweitem - misses_nach_erstem} "
            f"Parameter-Dateien neu von Platte gelesen -- Cache greift nicht ueber Requests hinweg.")
    finally:
        if os.path.exists(pfad):
            os.remove(pfad)
