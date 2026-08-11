"""Gegenprobe KAP Stufe 3 (Zeile 41, Kz E1905101, § 32d Abs. 1 S. 2/5): kein Kz ohne
erklaerte auslaendische Quellensteuer.

`kap_q_auslaendische_steuer` traegt KEINE Sonderbehandlung wie KAP_FELDER_A (kein
Null-Unterdrueckungsfilter, s. est_mapping.py Z.214-223) — es folgt der Standardregel: nur
bestaetigte Felder werden deklariert. Ohne Antwort steht das Feld nicht im Snapshot, also auch
kein E1905101 im XML. Mit Antwort (auch 0) erscheint der Kz. Dieser Test prueft GENAU diese
Naht (Ring -> est_mapping.deklariere), nicht die Ring-Rechnung selbst (s. dazu
test_p51a_kist_bemessung.py::test_kist_mit_kapital_und_q)."""
from __future__ import annotations

import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/mapping", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import est_mapping as EM   # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402

TS = "2026-08-10T10:00:00+00:00"
H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _b(s, feld_id, wert, zustand="bestaetigt"):
    sig = {"signal_1": None, "signal_2": f"ok@{feld_id}"} if zustand == "bestaetigt" else {"signal_1": None, "signal_2": None}
    ST.append_event(s, feld_id=feld_id, wert=wert, zustand=zustand, herkunft=H, schreiber="ui:laie",
                    signal=sig, ts=TS)


def _store_mit(felder: dict, vz=2025):
    s = ST.leerer_store(vz, fall_id="kap-q")
    for fid, wert in felder.items():
        _b(s, fid, wert)
    return s


def test_q_unbeantwortet_kein_kz(bindung):
    """Kapitalertraege erklaert, q NICHT beantwortet -> E1905101 fehlt in der Deklaration."""
    felder = {"kap_kapitalertraege": 500000, "kap_gewinn_aktien": 0, "kap_verlust_aktien": 0,
              "kap_gewinn_sonstige": 0, "kap_verlust_sonstige": 0}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert "E1905101" not in r["deklaration"], (
        "E1905101 (Zeile 41, anrechenbare auslaendische Steuer) deklariert, obwohl q nie "
        "beantwortet wurde — Kz ohne erklaerten Sachverhalt.")


def test_q_beantwortet_kz_erscheint(bindung):
    """q=100 EUR beantwortet -> E1905101 deklariert als "100,00".

    Erwartung korrigiert 2026-08-11: hier stand `== 100` (roher Integer), analog zu E1900701.
    Das war falsch — die beiden Kz haben verschiedene XSD-Typen:

        E1900701  Zeile 7   GanzzahlNichtNegOhneFuehrNull...       -> "5000"
        E1905101  Zeile 41  Dezimalzahl...MinNK2_MaxNK2...         -> "100,00"

    Der Vordruck zeigt es auch: Seite 3 fuehrt bei den Steuerabzugsbetraegen die Spalten
    "EUR | Ct", Zeile 7 nur "EUR". checkESt lehnte den Integer ab:
    "Geldbetraege muessen vom Format '0,00' sein". Das Kz gehoert deshalb in
    est_mapping._KOMMA_OHNE_E60_KZ, wo die drei Geschwister aus Stufe 2 schon standen.
    Format-Gate: tests/test_kap_dezimalformat.py.
    """
    felder = {"kap_kapitalertraege": 500000, "kap_gewinn_aktien": 0, "kap_verlust_aktien": 0,
              "kap_gewinn_sonstige": 0, "kap_verlust_sonstige": 0,
              "kap_q_auslaendische_steuer": 10000}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"].get("E1905101") == "100,00", (
        f"E1905101 fehlt oder falsches Format: {r['deklaration'].get('E1905101')!r} != '100,00'. "
        f"deklaration={r['deklaration']}")


def test_q_null_beantwortet_kz_bleibt(bindung):
    """q=0 EXPLIZIT beantwortet (nicht: unbeantwortet) -> Kz bleibt deklariert (echte Null, keine
    Sonderbehandlung wie bei KAP_FELDER_A — der Vordruck erlaubt 0, s. Bindung-Kommentar).

    Auch die Null traegt das Dezimalformat: "0,00", nicht 0. Siehe
    test_q_beantwortet_kz_erscheint.
    """
    felder = {"kap_kapitalertraege": 500000, "kap_gewinn_aktien": 0, "kap_verlust_aktien": 0,
              "kap_gewinn_sonstige": 0, "kap_verlust_sonstige": 0,
              "kap_q_auslaendische_steuer": 0}
    snap, _ = ST.materialisiere(_store_mit(felder))
    r = EM.deklariere(snap, bindung)
    assert r["deklaration"].get("E1905101") == "0,00", (
        f"E1905101 sollte als echte Null im Dezimalformat bleiben: "
        f"{r['deklaration'].get('E1905101')!r} != '0,00'")
