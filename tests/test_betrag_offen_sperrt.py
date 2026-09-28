"""Ein eröffnetes Thema mit unbeantwortetem Betrag sperrt, statt still als 0 zu rechnen.

Entscheidung [[fehlender-betrag-sperrt-bedingt-bestaetigte-null-ist-antwort]] (2026-09-28).
Gemessen an 9fdcc92 (Scheibe gesamt, Lohn 60.000 €, einzeln, VZ 2025): für jedes Feld unten war
„unbeantwortet" == „bestätigte 0", grund 'bestaetigt' — die Zahl sah fertig aus und wich ab
(p32b 10.000 € fehlen: 1.419 € zu wenig; Verlustvortrag / § 33a je 5.000 €: 1.884 € zu viel;
KiSt gezahlt 1.000 €: 370 € zu viel; KiSt erstattet 500 €: 191 € zu wenig).

Je Thema drei Zellen: unbeantwortet sperrt · bestätigte 0 rechnet · Thema verneint rechnet.
Die letzten zwei unterscheiden den Fix von „alles sperren".

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/eingang", "produkt/store", "produkt/traverser", "produkt/mapping"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API    # noqa: E402
import audit          # noqa: E402
from _kegel import kegel_fuer  # noqa: E402

# (Thema eröffnet, Betragsfeld, Wert, erwarteter Sperrgrund, Thema verneint)
THEMEN = [
    ({"keine_lohnersatzleistungen": False}, "p32b_progressionseinkuenfte", 1000000,
     "lohnersatz_betrag_offen", {"keine_lohnersatzleistungen": True}),
    ({"kein_verlustvortrag": False}, "verlustvortrag_bestand", 500000,
     "verlustvortrag_betrag_offen", {"kein_verlustvortrag": True}),
    ({"kein_unterhalt": False}, "p33a_unterhalt_aufwendungen", 500000,
     "unterhalt_betrag_offen", {"kein_unterhalt": True}),
    ({"kist_konfession": "roemisch-katholisch", "kist_erstattet": 0}, "kist_gezahlt", 100000,
     "kirchensteuer_betrag_offen", {"kist_konfession": "keine"}),
    ({"kist_konfession": "roemisch-katholisch", "kist_gezahlt": 100000}, "kist_erstattet", 50000,
     "kirchensteuer_betrag_offen", {"kist_konfession": "keine"}),
]
IDS = [t[1] for t in THEMEN]


def _laie(fid, wert):
    return {"feld_id": fid, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fid}"}}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    n = [0]

    def _lauf(gesetzt: dict, ohne: str | None = None) -> dict:
        n[0] += 1
        fid = f"bo_{n[0]}"
        st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid})
        assert st == 201, r
        for feld_id, wert in kegel_fuer("gesamt", {"bruttoarbeitslohn": 6000000, **gesetzt}):
            if feld_id == ohne:
                continue
            st, r = API.event(fid, _laie(feld_id, wert))
            assert st == 201, f"{feld_id}={wert!r}: {r}"
        st, erg = API.ergebnis(fid)
        assert st == 200, erg
        return erg
    return _lauf


@pytest.mark.parametrize("offen,feld,wert,grund,verneint", THEMEN, ids=IDS)
def test_unbeantworteter_betrag_sperrt(fall, offen, feld, wert, grund, verneint):
    erg = fall({**offen, feld: wert}, ohne=feld)
    assert erg["grund"] == grund, f"{feld} fehlt, rechnet still: {erg['grund']!r} {erg['zahl_cent']}"
    assert erg["zahl_cent"] is None


@pytest.mark.parametrize("offen,feld,wert,grund,verneint", THEMEN, ids=IDS)
def test_bestaetigte_null_rechnet(fall, offen, feld, wert, grund, verneint):
    erg = fall({**offen, feld: 0})
    assert erg["grund"] == "bestaetigt", f"{feld}=0 ist eine Antwort: {erg['grund']!r}"


@pytest.mark.parametrize("offen,feld,wert,grund,verneint", THEMEN, ids=IDS)
def test_verneintes_thema_rechnet(fall, offen, feld, wert, grund, verneint):
    erg = fall(dict(verneint), ohne=feld)
    assert erg["grund"] == "bestaetigt", f"Thema verneint, {feld} fehlt zu Recht: {erg['grund']!r}"
