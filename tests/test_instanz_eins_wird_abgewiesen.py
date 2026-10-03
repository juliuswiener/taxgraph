"""`x__1` ist keine Instanz: die Schreib-Route weist es ab, jeder Leser sieht es gleich.

Entscheidung Julius, 2026-10-03 (vault `die-schreib-route-weist-eine-kennung-mit-instanz-eins-ab`):
Die erste Instanz eines Feldes trägt den Basisnamen (`x`), ab der zweiten steht ein Zähler (`x__2`).
Die Oberfläche erzeugt `x__1` nie. Der Server nahm es trotzdem an, wenn `x` zu einer Instanz-Gruppe
gehört, und die Leser verstanden es verschieden: Deklaration und Ring-Instanz-Leser legten `x__1` in
dasselbe Fach wie `x`, der Traverser fragte für Instanz 1 nur nach `x`. Schickte ein Aufrufer beide,
überschrieb eines das andere ohne Meldung.

Die Regel für den Zähler ist `[2-9]|[1-9][0-9]+`. Sie steht an zwei Stellen in Python
(`est_mapping._INSTANZ_RE`, `flag_check._INSTANZ_SUFFIX_RE`) und gilt in Rust gleich
(`elster::parse_instanz`, `store::instanz_basis`, `konsistenz::instanz_feld_ids`); der Paritätstest
`elster_paritaet` hält die beiden Sprachen zusammen.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "produkt/konsistenz", "produkt/eingang", "produkt/bescheid",
             "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API                  # noqa: E402
import audit                       # noqa: E402
import est_mapping as EM           # noqa: E402
import flag_check as FC            # noqa: E402
import traverser as TR             # noqa: E402

# Eine Basis mit instanz_gruppe in der Scheibe `gesamt` (Vermietung, Gruppe `vv_objekt`).
BASIS = "vv_einnahmen"
WERT = 1_500_000

ZAEHLER_JA = ["2", "3", "9", "10", "11", "19", "20", "99", "100", "101", "1000", "12345678901234567890"]
# `1` ist die Basis selbst, `0` und führende Nullen gibt es nicht, der Rest ist keine Zahl.
ZAEHLER_NEIN = ["1", "0", "00", "01", "02", "001", "010", "", "x", "2x", "x2", "-2", "+2", "2.0", " 2", "2 ",
                "٢"]  # arabisch-indische Ziffer zwei: `[0-9]` kennt sie nicht


@pytest.mark.parametrize("zaehler", ZAEHLER_JA)
def test_parse_instanz_nimmt_zaehler_ab_zwei(zaehler):
    assert EM.parse_instanz(f"{BASIS}__{zaehler}") == (BASIS, int(zaehler))


@pytest.mark.parametrize("zaehler", ZAEHLER_NEIN)
def test_parse_instanz_lehnt_eins_null_und_unsinn_ab(zaehler):
    assert EM.parse_instanz(f"{BASIS}__{zaehler}") is None


def test_die_basis_ist_instanz_eins_und_hat_keinen_zaehler():
    assert EM.parse_instanz(BASIS) is None


def _rumpf(feld_id: str, wert: int) -> dict:
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"klick@{feld_id}"}}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    _st, r = API.fall_anlegen({"fall_id": "instanz-eins", "scheibe": "gesamt", "veranlagungszeitraum": 2025})
    return r["fall_id"]


def _feld_ids(fall_id: str) -> set:
    return set(API.ST._aktives(API.lade_fall(fall_id)))


def test_vorbedingung_die_basis_hat_eine_instanz_gruppe():
    """Ohne sie misst der Test darunter nichts: die Abweisung käme dann von der Gruppe, nicht vom Zähler."""
    assert TR.lade_bindung()[BASIS].get("instanz_gruppe")


def test_schreib_route_weist_instanz_eins_ab_und_laesst_den_rest_durch(fall):
    for ok in (BASIS, f"{BASIS}__2", f"{BASIS}__10"):
        st, _ = API.event(fall, _rumpf(ok, WERT))
        assert st == 201, f"{ok}: {st}"
    with pytest.raises(API.ApiError) as e:
        API.event(fall, _rumpf(f"{BASIS}__1", WERT))
    assert e.value.status == 400, f"Status {e.value.status} statt 400"
    assert "nicht in dieser Scheibe" in str(e.value)
    # Abgewiesen heißt: nichts steht im Store, und `x` ist nicht überschrieben worden.
    assert _feld_ids(fall) == {BASIS, f"{BASIS}__2", f"{BASIS}__10"}


@pytest.mark.parametrize("kennung", [f"{BASIS}__0", f"{BASIS}__01", f"{BASIS}__02", f"{BASIS}__"])
def test_schreib_route_weist_auch_null_und_fuehrende_null_ab(fall, kennung):
    with pytest.raises(API.ApiError) as e:
        API.event(fall, _rumpf(kennung, WERT))
    assert e.value.status == 400


def test_traverser_und_parse_instanz_stimmen_ueberein():
    """Der Traverser erzeugt `x` für Instanz 1 und `x__i` ab 2, und `parse_instanz` liest genau das:
    für jede Erzeugung ist die Rückrichtung eindeutig, und `x__1` kommt in keiner der beiden vor."""
    for i in range(1, 30):
        kennung = TR.instanz_feld_id(BASIS, i)
        assert EM.parse_instanz(kennung) == (None if i == 1 else (BASIS, i)), kennung
    assert EM.parse_instanz(f"{BASIS}__1") is None


def test_flag_check_zaehlt_instanz_eins_nur_als_basis():
    """`flag_check` führt eine eigene Fassung der Regel (es importiert `est_mapping` bewusst nicht). Sie darf
    `x__1` nicht als zweites Exemplar von Instanz 1 zählen."""
    snapshot = {BASIS: 1, f"{BASIS}__1": 1, f"{BASIS}__2": 1, f"{BASIS}__10": 1, f"{BASIS}__01": 1, f"{BASIS}__x": 1}
    assert FC._instanz_feld_ids(snapshot, BASIS) == [BASIS, f"{BASIS}__2", f"{BASIS}__10"]
