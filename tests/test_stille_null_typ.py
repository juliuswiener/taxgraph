"""Stille-Null-Klasse (Variante Typ-Konformität) — team-lead-Auftrag 2026-08-16.

Zwei Befunde, ein Muster: ein Wert, der nie richtig ankam, wurde trotzdem als `bestaetigt`
gespeichert — der Ring liest ihn dann still als 0 (kein Fehler, kein Absturz, falsche Steuer).

Befund A (Backend): `store.append_event` prüfte `wert` NIE gegen den Bindungstyp (typ=cent/int/
bool/enum/datum/text aus produkt/bindung/*.yaml) — für KEINEN Schreiber, auch nicht für den
menschlichen (ui:laie). Live reproduziert (Scratchpad, siehe Bericht): ein Gehalt als String
'50000' auf bruttoarbeitslohn (typ=cent) wurde mit zustand=bestaetigt akzeptiert; die _c/_cent/
_best-Lesehelfer in api.py (>15 Stellen) machen aus jedem Nicht-Zahl-Wert kommentarlos 0.

Fix: neue Auflage T in produkt/store/store.py (append_event bekommt ein optionales `bindung`-
Argument; wird es übergeben, muss `wert` zum Bindungstyp passen, sonst ValueError). Bewusst
OPTIONAL statt scharf für ALLE Aufrufer — die >2100 Bestandstests von ST.append_event(...) rufen
ohne bindung= und bleiben unberührt (sie testen andere Mechanismen mit absichtlich minimalem
Setup). Real verdrahtet an api.event() (HTTP, hier getestet), api.entfernung(), dem KI-Chat-
Vorschlags-Schreiber, beleg_writer und kontoauszug_writer (s. Bericht an team-lead für die
vollständige Naht-(a)-vs-(b)-Abwägung — insbesondere: import:elster/eDaten schreibt DIREKT
bestaetigt und hat noch KEINEN Produktions-Aufrufer, darum dort nur vorbereitet, nicht verdrahtet).

Befund B (Frontend, produkt/haut/static/app.js:leseWert) ist NICHT hier, sondern in
tests/test_ui_leerwert_stille_null.py (Playwright, wie test_ui_wahl_buttons.py).

NULL LLM.
"""
from __future__ import annotations

import glob
import os
import sys

import pytest

yaml = pytest.importorskip("yaml")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
             "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API     # noqa: E402
import audit          # noqa: E402
import store as ST    # noqa: E402

TS = "2026-08-16T12:00:00+00:00"


def _bindung() -> dict:
    b = {}
    for f in glob.glob(os.path.join(ROOT, "produkt", "bindung", "bindung_*.yaml")):
        for e in (yaml.safe_load(open(f, encoding="utf-8")).get("bindungen") or []):
            b[e["feld_id"]] = e
    return b


BINDUNG = _bindung()


def _laie(fld, w, zustand="bestaetigt"):
    signal2 = f"ok@{fld}" if zustand == "bestaetigt" else None
    return {"feld_id": fld, "wert": w, "zustand": zustand,
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie",
            "signal": {"signal_1": None, "signal_2": signal2}}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, r = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "sn-typ"})
    assert st == 201, r
    return "sn-typ"


# ---------------------------------------------------------------- HTTP-Ebene (api.event, Befund A)

def test_string_auf_cent_feld_wird_abgelehnt(fall):
    """Der genaue Live-Repro aus dem Auftrag: Gehalt als String statt Cent-Integer.
    api.event() fängt die ValueError des Stores und wirft ApiError(422, ...) — kein Tupel-Return
    (anders als z.B. einreichen()), darum pytest.raises statt st==422 (Muster aus
    test_fall_loeschen.py/test_kontoauszug_pdf_endpoint.py)."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("bruttoarbeitslohn", "50000"))
    assert exc.value.status == 422
    assert "Typ" in str(exc.value)


def test_bool_auf_cent_feld_wird_abgelehnt(fall):
    """Python-Falle: bool ist ein int-Subtyp (isinstance(True, int) is True). Ohne den expliziten
    Ausschluss ginge ein Häkchen als 1 Cent durch — genau der in produkt/store/store.py
    dokumentierte Grund für `not isinstance(wert, bool)`."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("bruttoarbeitslohn", True))
    assert exc.value.status == 422


def test_float_auf_cent_feld_wird_abgelehnt(fall):
    """cent ist ein Integer-Betrag — ein Bruchteil-Cent (z.B. aus falscher Euro→Cent-Umrechnung
    ohne round()) ist kein gültiger Speicherwert."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("bruttoarbeitslohn", 50000.5))
    assert exc.value.status == 422


def test_valider_cent_wert_wird_akzeptiert(fall):
    st, r = API.event(fall, _laie("bruttoarbeitslohn", 6000000))
    assert st == 201, r


def test_explizite_null_bleibt_gueltig(fall):
    """Der Kern der Team-Lead-Vorgabe: eine ECHTE, absichtlich eingegebene 0 darf NICHT durch die
    neue Typ-Prüfung verworfen werden — sie ist ein gültiger Cent-Betrag, keine Nicht-Antwort."""
    st, r = API.event(fall, _laie("bruttoarbeitslohn", 0))
    assert st == 201, r


def test_falscher_enum_wert_wird_abgelehnt(fall):
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("veranlagung", "gemeinsam"))   # gültig wären einzel/zusammen
    assert exc.value.status == 422


def test_gueltiger_enum_wert_wird_akzeptiert(fall):
    st, r = API.event(fall, _laie("veranlagung", "einzel"))
    assert st == 201, r


def test_string_auf_datum_feld_wird_abgelehnt(fall):
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_geburtsdatum", "kein Datum"))
    assert exc.value.status == 422


def test_iso_datum_wird_abgelehnt(fall):
    """Die Gegenrichtung, und sie ist der eigentliche Punkt: ISO ist hier FALSCH. Der erste
    Anlauf dieser Prüfung verlangte ISO und hätte damit jede echte Geburtsdatums-Eingabe mit
    422 abgewiesen — fail-closed gegen den eigenen dokumentierten Standard. Wer den Regex
    künftig „modernisiert", macht diesen Test rot und liest den Grund."""
    with pytest.raises(API.ApiError) as exc:
        API.event(fall, _laie("stammdaten_geburtsdatum", "1985-04-12"))
    assert exc.value.status == 422


def test_valides_datum_wird_akzeptiert(fall):
    """TT.MM.JJJJ — amtliches ELSTER-Format (XSD DatumTTpMMpJJJJBekanntBaseCType_RABE zu
    E0100401), so auch der beispielwert der Bindung. Nichts in der Pipeline konvertiert."""
    st, r = API.event(fall, _laie("stammdaten_geburtsdatum", "12.04.1985"))
    assert st == 201, r


# ---------------------------------------------------------------- Store-Ebene (append_event, direkt)

def test_ohne_bindung_bleibt_rueckwaertskompatibel():
    """Die >2100 Bestandsaufrufe von ST.append_event(...) übergeben KEIN bindung= — sie dürfen
    durch die neue Auflage T nicht brechen. Ohne bindung greift die Prüfung nicht, exakt wie vor
    diesem Fix (das ist der Grund, warum die Suite ohne Massen-Retrofit grün bleiben kann)."""
    s = ST.leerer_store(2025, fall_id="sn-typ-kompat")
    ev = ST.append_event(s, feld_id="bruttoarbeitslohn", wert="50000", zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS)
    assert ev["wert"] == "50000"   # unverändert akzeptiert -- kein bindung= übergeben


def test_mit_bindung_greift_direkt_am_store():
    s = ST.leerer_store(2025, fall_id="sn-typ-direkt")
    with pytest.raises(ValueError, match="fail-closed \\(Typ\\)"):
        ST.append_event(s, feld_id="bruttoarbeitslohn", wert="50000", zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS,
                        bindung=BINDUNG)


def test_unbekanntes_feld_id_durchlaesst():
    """Team-Lead-Vorgabe Schritt 3: unbekanntes feld_id -> durchlassen, nicht raten."""
    s = ST.leerer_store(2025, fall_id="sn-typ-unbekannt")
    ev = ST.append_event(s, feld_id="dieses_feld_gibt_es_nicht", wert="irgendwas", zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS,
                         bindung=BINDUNG)
    assert ev["wert"] == "irgendwas"


def test_typ_konform_spiegelt_test_store_typ_ok():
    """produkt/store/store.py:_typ_konform ist ABSICHTLICH eine gespiegelte Kopie von
    tests/test_store.py:_typ_ok (Produktionscode importiert keine Testdatei) — dieser Test hält
    beide synchron: driftet die eine Semantik von der anderen, schlägt er an, statt dass es erst
    an einem falsch akzeptierten/abgelehnten Produktionswert auffällt."""
    from test_store import _typ_ok
    faelle = [
        (50000, "cent", None, True), ("50000", "cent", None, False), (True, "cent", None, False),
        (50000.5, "cent", None, False), (0, "cent", None, True),
        (True, "bool", None, True), (1, "bool", None, False), ("true", "bool", None, False),
        ("einzel", "enum", ["einzel", "zusammen"], True), ("x", "enum", ["einzel", "zusammen"], False),
        ("12.04.1985", "datum", None, True), ("1985-04-12", "datum", None, False),
        ("kein Datum", "datum", None, False),
        ("Text", "text", None, True), (5, "text", None, False),
    ]
    for wert, typ, enum_werte, erwartet in faelle:
        assert ST._typ_konform(wert, typ, enum_werte) == erwartet == _typ_ok(wert, typ, enum_werte), (
            f"Drift zwischen store._typ_konform und test_store._typ_ok bei wert={wert!r} typ={typ!r}")
