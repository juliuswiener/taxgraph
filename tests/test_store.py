"""Gate für den Sachverhalts-Store (produkt/store/, Task #11).

Deterministisch, LLM-frei. Prüft Schema, Content-Adressierung, Materialisierung, ERiC-Bindung,
feld_id-Bindung an die Bindungstabelle, Typ-Konformität + die Auflagen A–D. Plus Negativtests.
"""
from __future__ import annotations

import glob
import json
import os
import re
import sys

import pytest

yaml = pytest.importorskip("yaml")
jsonschema = pytest.importorskip("jsonschema")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
STORE_DIR = os.path.join(ROOT, "produkt", "store")
BIND_DIR = os.path.join(ROOT, "produkt", "bindung")
SCHEMA_PATH = os.path.join(STORE_DIR, "schema.json")

sys.path.insert(0, STORE_DIR)
import store as ST  # noqa: E402

TS = "2026-07-17T14:00:00+00:00"


def _schema():
    with open(SCHEMA_PATH) as f:
        return json.load(f)


def _bindung_typen():
    typen = {}
    for f in glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml")):
        d = yaml.safe_load(open(f))
        for b in d["bindungen"]:
            typen[b["feld_id"]] = (b["typ"], b.get("enum_werte"))
    return typen


def _katalog():
    """Feld-Katalog (K1) aus der echten Bindung — für die katalog-pflichtigen Vorschlags-Schreiber-Writes."""
    b = {}
    for f in glob.glob(os.path.join(BIND_DIR, "bindung_*.yaml")):
        for e in (yaml.safe_load(open(f)).get("bindungen") or []):
            b[e["feld_id"]] = e
    return ST.lade_katalog(b)


def _typ_ok(wert, typ, enum_werte) -> bool:
    if typ in ("cent", "int"):
        return isinstance(wert, int) and not isinstance(wert, bool)
    if typ == "bool":
        return isinstance(wert, bool)
    if typ == "enum":
        return wert in (enum_werte or [])
    if typ == "datum":
        # TT.MM.JJJJ — amtliches ELSTER-Format, kein ISO. Muss mit store._typ_konform
        # uebereinstimmen; test_typ_konform_spiegelt_test_store_typ_ok haelt beide synchron.
        return isinstance(wert, str) and bool(re.match(r"^\d{2}\.\d{2}\.\d{4}$", wert))
    if typ == "text":
        return isinstance(wert, str)
    return False


@pytest.fixture()
def sample():
    """Ein valider Store, gebaut über den EINEN Schreibpfad."""
    s = ST.leerer_store(2025, fall_id="fall-test")
    ST.append_event(s, feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok@ep_arbeitstage"},
                    ts=TS)
    ST.append_event(s, feld_id="agb_aufwendungen", wert=120000, zustand="vorlaeufig",
                    herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS,
                    katalog=_katalog())   # K1: agb_aufwendungen ist llm-suggestible (Feld-Katalog); events[1]=llm-Event
    ST.append_event(s, feld_id="vor_an_anteil_rv", wert=3500000, zustand="bestaetigt",
                    herkunft={"herkunft": "beleg_import", "pruef_tiefe": "plausibilisiert", "haftung": "nutzer"},
                    schreiber="import:elster", signal={"signal_1": None, "signal_2": "lstb_z23"}, ts=TS)
    ST.erzeuge_snapshot(s, ts=TS, eric_befund={"rc": 0, "klasse": "plausibel",
                                               "gekappt_verdacht": False, "fehler_anzahl": 0})
    return s


# ---- (a) Schema ---------------------------------------------------------------

def test_a_schema_valid(sample):
    errs = sorted(jsonschema.Draft202012Validator(_schema()).iter_errors(sample), key=lambda e: list(e.path))
    assert not errs, "; ".join(f"{list(e.path)}: {e.message}" for e in errs[:5])


# ---- (b) Content-Adressierung nachgerechnet -----------------------------------

def test_b_event_id_content_adressiert(sample):
    for e in sample["events"]:
        assert e["event_id"] == ST.event_id(e), f"event_id falsch für {e['feld_id']}"


def test_b_snapshot_id_content_adressiert(sample):
    for snap in sample["snapshots"]:
        assert snap["snapshot_id"] == ST.snapshot_id(snap["felder"]), "snapshot_id falsch"


# ---- (c) Materialisierung deterministisch = snapshot.felder --------------------

def test_c_materialisierung(sample):
    felder, sid = ST.materialisiere(sample)
    snap = sample["snapshots"][-1]
    assert felder == snap["felder"], "Materialisierung != snapshot.felder"
    assert sid == snap["snapshot_id"]


def test_c_ersetzt_aufloesung():
    s = ST.leerer_store(2025)
    e1 = ST.append_event(s, feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "x"}, ts=TS)
    ST.append_event(s, feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "y"},
                    ersetzt=e1["event_id"], ts=TS)
    felder, _ = ST.materialisiere(s)
    assert felder["ep_arbeitstage"]["wert"] == 220, "ersetzt-Auflösung falsch (altes Event gewinnt)"


# ---- (d) ERiC-Befund bindet an snapshot_id ------------------------------------

def test_d_eric_bindet_an_snapshot(sample):
    snap = sample["snapshots"][-1]
    assert snap["eric_befund"]["gebunden_an"] == snap["snapshot_id"]


# ---- (e) feld_id existiert in der Bindungstabelle -----------------------------

def test_e_feld_id_in_bindung(sample):
    typen = _bindung_typen()
    for e in sample["events"]:
        assert e["feld_id"] in typen, f"feld_id {e['feld_id']} nicht in Bindungstabelle"


# ---- (D) Typ-Konformität gegen die Bindungstabelle ----------------------------

def test_D_typ_konformitaet(sample):
    typen = _bindung_typen()
    for e in sample["events"]:
        typ, enum = typen[e["feld_id"]]
        assert _typ_ok(e["wert"], typ, enum), (
            f"{e['feld_id']}: Wert {e['wert']!r} passt nicht zu typ {typ}")


# ---- (A) Schreiber↔Herkunft-Kopplung (Mechanismus hart) -----------------------

def test_A_llm_schreiber_gekoppelt():
    s = ST.leerer_store(2025)
    with pytest.raises(ValueError):
        ST.append_event(s, feld_id="ep_entfernung_km", wert=30, zustand="bestaetigt",
                        herkunft={"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="llm:chat", signal={"signal_1": None, "signal_2": "gefaelscht"}, ts=TS)


def test_A_berechnet_schreiber_kann_nie_bestaetigen():
    """defense-in-depth (Instructor-Follow 2026-07-18): ein berechnet:-Schreiber (z.B. berechnet:maps =
    Karten-Entfernung) ist ein abgeleiteter VORSCHLAG — der Store erzwingt vorlaeufig+signal_2=null
    STRUKTURELL, nicht nur der disziplinierte Endpunkt. Ein berechnet:+bestaetigt-Event fällt fail-closed."""
    s = ST.leerer_store(2025)
    with pytest.raises(ValueError, match="berechnet:"):
        ST.append_event(s, feld_id="ep_entfernung_km", wert=30, zustand="bestaetigt",
                        herkunft={"herkunft": "berechnet", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="berechnet:maps",
                        signal={"signal_1": {"typ": "maps"}, "signal_2": "erschlichen"}, ts=TS)


def test_A_berechnet_schreiber_vorlaeufig_ok():
    """Gegenprobe: der disziplinierte berechnet:maps-Write (vorlaeufig, herkunft=berechnet, signal_2=null)
    geht durch — der Guard blockt nur das Bestätigen, nicht den legitimen Vorschlag (kein Über-Blocken)."""
    s = ST.leerer_store(2025)
    ev = ST.append_event(s, feld_id="ep_entfernung_km", wert=30, zustand="vorlaeufig",
                         herkunft={"herkunft": "berechnet", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="berechnet:maps",
                         signal={"signal_1": {"typ": "maps", "dienst": "openrouteservice"}, "signal_2": None},
                         ts=TS, katalog=_katalog())
    assert ev["zustand"] == "vorlaeufig" and ev["herkunft"]["herkunft"] == "berechnet"


# ---- (B) Ein aktives Event je feld_id -----------------------------------------

def test_B_zweites_event_ohne_ersetzt_faellt():
    s = ST.leerer_store(2025)
    ST.append_event(s, feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
                    herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                    schreiber="ui:laie", signal={"signal_1": None, "signal_2": "x"}, ts=TS)
    with pytest.raises(ValueError):
        ST.append_event(s, feld_id="ep_arbeitstage", wert=220, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "y"}, ts=TS)


def test_B_ersetzt_fremdes_feld_faellt():
    s = ST.leerer_store(2025)
    e1 = ST.append_event(s, feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "x"}, ts=TS)
    with pytest.raises(ValueError):
        ST.append_event(s, feld_id="ep_entfernung_km", wert=30, zustand="vorlaeufig",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": None},
                        ersetzt=e1["event_id"], ts=TS)


def test_B_veraltete_event_id_faellt():
    """Guard: Kette a→b aufbauen, dann erneut mit a["event_id"] überschreiben → ValueError.

    Eine einmal ersetzte event_id ist verbraucht. Sonst könnten zwei Korrekturen dieselbe
    Vorlage überschreiben und eine ginge verloren.
    """
    s = ST.leerer_store(2025)
    # Event a (wird ersetzt)
    e_a = ST.append_event(s, feld_id="ep_arbeitstage", wert=100, zustand="bestaetigt",
                          herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                          schreiber="ui:laie", signal={"signal_1": None, "signal_2": "a"}, ts=TS)
    # Event b (ersetzt a)
    e_b = ST.append_event(s, feld_id="ep_arbeitstage", wert=150, zustand="bestaetigt",
                          herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                          schreiber="ui:laie", signal={"signal_1": None, "signal_2": "b"},
                          ersetzt=e_a["event_id"], ts=TS)
    # Versuch: erneut mit a["event_id"] überschreiben → sollte fehlschlagen
    with pytest.raises(ValueError, match="ersetzt-Ziel ist bereits ersetzt"):
        ST.append_event(s, feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "c"},
                        ersetzt=e_a["event_id"], ts=TS)


def test_B_bestaetigt_ohne_signal_2_beim_ersetzen_faellt():
    """Guard: aktives Event vorhanden, Korrektur mit zustand=bestaetigt und signal_2=None → ValueError.

    Die Zwei-Signal-Regel darf nicht dadurch umgehbar sein, dass man den Umweg über ersetzt nimmt.
    """
    s = ST.leerer_store(2025)
    # Aktives Event
    e_aktiv = ST.append_event(s, feld_id="ep_arbeitstage", wert=100, zustand="bestaetigt",
                              herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                              schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS)
    # Versuch: ersetzen mit bestaetigt aber ohne signal_2 → sollte fehlschlagen
    with pytest.raises(ValueError, match="zustand=bestaetigt braucht ein signal_2"):
        ST.append_event(s, feld_id="ep_arbeitstage", wert=200, zustand="bestaetigt",
                        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="ui:laie", signal={"signal_1": None, "signal_2": None},
                        ersetzt=e_aktiv["event_id"], ts=TS)


# ---- (C) gekappt_verdacht + Falsch-Grün-Regel ---------------------------------

def _ist_gruen(eric: dict) -> bool:
    """Downstream-Regel (Auflage C): grün nur wenn plausibel UND NICHT gekappt-verdächtig."""
    return eric["klasse"] == "plausibel" and not eric.get("gekappt_verdacht", False)


def test_C_gekappt_verdacht_pflicht(sample):
    assert "gekappt_verdacht" in sample["snapshots"][-1]["eric_befund"]


def test_C_plausibel_aber_gekappt_ist_nicht_gruen():
    assert not _ist_gruen({"klasse": "plausibel", "gekappt_verdacht": True})
    assert _ist_gruen({"klasse": "plausibel", "gekappt_verdacht": False})


# ---- Typ-Algebra (Meet über Input-Kegel, Julius #2) ---------------------------

def test_meet_zustand_fail_closed():
    assert ST.meet_zustand(["bestaetigt", "bestaetigt"]) == "bestaetigt"
    assert ST.meet_zustand(["bestaetigt", "vorlaeufig"]) == "vorlaeufig"   # ein vorlaeufig -> Aggregat vorlaeufig


def test_meet_herkunft_konflikt_und_minimum():
    v1 = {"herkunft": "laie", "pruef_tiefe": "amtlich", "haftung": "nutzer"}
    v2 = {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}
    m = ST.meet_herkunft([v1, v2])
    assert m["pruef_tiefe"] == "ungeprueft"      # Minimum der Kette
    assert m["herkunft"] == ST.KONFLIKT          # verschiedene Kategorien -> Konflikt
    assert m["haftung"] == "nutzer"              # gleich -> bleibt


# ---- Negativtests am Schema ---------------------------------------------------

def test_neg_schema_manipuliertes_event(sample):
    """Manipulierter Wert ohne Hash-Nachzug -> Content-Adress-Check bricht."""
    s = json.loads(json.dumps(sample))
    s["events"][0]["wert"] = 999
    assert s["events"][0]["event_id"] != ST.event_id(s["events"][0]), "Tamper am Wert nicht erkannt"


def test_neg_schema_eric_falscher_hash(sample):
    s = json.loads(json.dumps(sample))
    s["snapshots"][-1]["eric_befund"]["gebunden_an"] = "b" * 64
    assert s["snapshots"][-1]["eric_befund"]["gebunden_an"] != s["snapshots"][-1]["snapshot_id"]


def test_neg_schema_bestaetigt_ohne_signal2(sample):
    s = json.loads(json.dumps(sample))
    s["events"][0]["signal"]["signal_2"] = None
    assert list(jsonschema.Draft202012Validator(_schema()).iter_errors(s)), "bestaetigt ohne signal_2 nicht abgelehnt"


def test_neg_schema_llm_bestaetigt(sample):
    s = json.loads(json.dumps(sample))
    e = s["events"][1]                 # der llm-Event
    e["zustand"] = "bestaetigt"
    e["signal"]["signal_2"] = "x"
    assert list(jsonschema.Draft202012Validator(_schema()).iter_errors(s)), "llm+bestaetigt nicht abgelehnt"


# ---- (F2) Magnitude-Bound für Vorschlags-Schreiber (EUR-statt-Cent-Verwechslung) ----

def test_F2_vorschlag_riesenwert_faellt():
    s = ST.leerer_store(2025)
    with pytest.raises(ValueError, match="Magnitude"):
        ST.append_event(s, feld_id="agb_aufwendungen", wert=10**10, zustand="vorlaeufig",
                        herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS,
                        katalog=_katalog())


def test_F2_vorschlag_normaler_centwert_ok():
    s = ST.leerer_store(2025)
    ev = ST.append_event(s, feld_id="agb_aufwendungen", wert=215600, zustand="vorlaeufig",
                         herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS,
                         katalog=_katalog())
    assert ev["wert"] == 215600


def test_F2_vorschlag_negativer_verlust_ok():
    """Symmetrischer Bound (abs) — kein 0-Floor, ein legitimer negativer Verlust-Wert bleibt zulässig."""
    s = ST.leerer_store(2025)
    ev = ST.append_event(s, feld_id="agb_aufwendungen", wert=-50000, zustand="vorlaeufig",
                         herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS,
                         katalog=_katalog())
    assert ev["wert"] == -50000


def test_F2_mensch_schreiber_ohne_bound():
    """Der Bound gilt NUR für Vorschlags-Schreiber — ein human-Write bleibt unbeschränkt (kein neuer Fail-Pfad)."""
    s = ST.leerer_store(2025)
    ev = ST.append_event(s, feld_id="agb_aufwendungen", wert=10**10, zustand="bestaetigt",
                         herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, ts=TS)
    assert ev["wert"] == 10**10


def test_F2_vorschlag_riesenwert_als_string_faellt():
    """String-Bypass (Review-Fund): chat()/generic /event reichen wert roh durch, LLMs liefern oft JSON-Strings."""
    s = ST.leerer_store(2025)
    with pytest.raises(ValueError, match="Magnitude"):
        ST.append_event(s, feld_id="agb_aufwendungen", wert="12000000000", zustand="vorlaeufig",
                        herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS,
                        katalog=_katalog())


def test_F2_vorschlag_negative_grenze_faellt():
    """abs()-Symmetrie-Mutationskiller: ohne abs() im Bound würde dieser negative Grenzwert durchrutschen."""
    s = ST.leerer_store(2025)
    with pytest.raises(ValueError, match="Magnitude"):
        ST.append_event(s, feld_id="agb_aufwendungen", wert=-(10**10), zustand="vorlaeufig",
                        herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS,
                        katalog=_katalog())


def test_F2_vorschlag_kleiner_stringwert_ok():
    """Kein False-Reject: kleiner numerischer String-Wert bleibt zulässig."""
    s = ST.leerer_store(2025)
    ev = ST.append_event(s, feld_id="agb_aufwendungen", wert="215600", zustand="vorlaeufig",
                         herkunft={"herkunft": "llm_vorschlag", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                         schreiber="llm:chat", signal={"signal_1": None, "signal_2": None}, ts=TS,
                         katalog=_katalog())
    assert ev["wert"] == "215600"
