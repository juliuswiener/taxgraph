"""Ein Geldfeld, das im amtlichen Schema kein Minus kennt, lehnt ein Minus schon bei der Eingabe ab.

Vault decisions/geldfeld-ohne-minus-im-schema-lehnt-minus-bei-eingabe-ab (Instructor, 2026-10-02),
Backlog negativer-aufwand-umgeht-pflichtfrage. Gemessen (Worker authfix, 2026-10-02): POST /event mit
-50000 Cent auf hh_handwerker_betrag gab 201, die Pflichtfrage "Rechnung und Überweisung?" prüft nur
Beträge > 0 und entfiel, ELSTER lehnte die Erklärung erst bei der Abgabe ab ("Unzulässiges Vorzeichen").

`nicht_negativ: true` steht an 42 Feldern, deren Schematyp kein Minus kennt (NichtNeg/Pos, E10-2025.xsd),
und an 8 Partner-Spiegeln ohne eigenes Kz. Der Store weist ein Minus dort beim Schreiben ab (422, Auflage V),
0 und Positives gehen durch, Laden prüft nie. Die vier Verlustfelder, zwei Differenzfelder und 74 unklare
Felder tragen es nicht.

ponytail: die 74 unklaren Felder (23 mit XSD-Minus ohne Differenz im Bindungstext, 51 ohne Kz) bekommen das
Attribut erst bei eigenem Beleg; `test_schema_kennt_minus_nicht_aber_bindung_sperrt_es` hält nur die
Gegenrichtung fest (kein Attribut ohne Schemagrund). Obergrenze: die Heuristik liest den Typnamen
(NichtNeg/Pos), nicht das Muster — das Muster steckt im Basistyp. Upgrade: Muster über die Vererbungskette
auflösen, wenn ein Typ ohne diese Namen auftaucht."""
from __future__ import annotations

import functools
import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/store", "produkt/mapping", "produkt/traverser"):
    if os.path.join(ROOT, _sub) not in sys.path:
        sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API  # noqa: E402
import store as ST  # noqa: E402
import traverser as TR  # noqa: E402
import xsd_verify as X  # noqa: E402
from test_paket_b_e2e_http import _laie, _req, base  # noqa: E402,F401 — Fixture

BINDUNG = TR.lade_bindung()
MIT_ATTRIBUT = sorted(f for f, b in BINDUNG.items() if b.get("nicht_negativ"))
VERLUSTFELDER = ["einkuenfte_gewinn", "einkuenfte_gewinn_partner", "gewinnanteil", "gewinnanteil_partner"]
JAHR = 2025


@functools.lru_cache(maxsize=1)
def _kz_meta() -> dict:
    meta = {}
    for muster, start in (("E10-{jahr}.xsd", "E10"), ("E77-{jahr}.xsd", "E77")):
        pfad = X._find_schema(JAHR, muster)
        if pfad is not None:
            meta[start] = X._resolve_kz_meta(pfad, start)
    return meta


def _schema_kennt_kein_minus(feld_id: str):
    """True/False, wenn das Kz des Cent-Felds im lokalen Schema liegt; None ohne Kz oder ohne Schemaeintrag."""
    kz = BINDUNG[feld_id].get("elster_kz")
    if not isinstance(kz, str) or not kz.startswith("E"):
        return None
    m = _kz_meta().get(X._datenart_fuer_kz(kz)[1], {}).get(kz)
    if m is None:
        return None
    return "NichtNeg" in m["type_name"] or "Pos" in m["type_name"]


def _schreibe(feld_id: str, wert, bindung=BINDUNG):
    st = ST.leerer_store(2025, fall_id="nicht-negativ")
    return ST.append_event(
        st, feld_id=feld_id, wert=wert, zustand="bestaetigt",
        herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"}, bindung=bindung)


# ------------------------------------------------------------------ AK4: Bindung <-> lokales Schema

def test_cent_feld_mit_schematyp_ohne_minus_traegt_nicht_negativ(braucht_echtes_xsd):
    """Fehlt das Schema, ist der Test LAUT ROT (Skip nur mit TAXGRAPH_OHNE_XSD=1, tests/conftest.py)."""
    assert _kz_meta(), "E10-2025.xsd nicht gefunden"
    ohne_minus = [f for f, b in sorted(BINDUNG.items()) if b.get("typ") == "cent" and _schema_kennt_kein_minus(f)]
    assert len(ohne_minus) == 42, f"gemessen 2026-10-02: 42 Cent-Felder mit Schematyp ohne Minus, jetzt {len(ohne_minus)}"
    fehlend = [f for f in ohne_minus if not BINDUNG[f].get("nicht_negativ")]
    assert fehlend == [], (
        f"{len(fehlend)} Cent-Felder, deren Schematyp kein Minus kennt, tragen nicht_negativ nicht — ein Minus "
        f"landet dort ungefragt in der Erklärung und scheitert erst bei ELSTER: {fehlend}")


def test_schema_kennt_minus_nicht_aber_bindung_sperrt_es(braucht_echtes_xsd):
    """Die Gegenrichtung: nicht_negativ ohne Schemagrund würde ein erlaubtes Minus abweisen."""
    assert _kz_meta(), "E10-2025.xsd nicht gefunden"
    zuviel = [f for f in MIT_ATTRIBUT if _schema_kennt_kein_minus(f) is False]
    assert zuviel == [], f"nicht_negativ an Feldern, deren Schematyp ein Minus erlaubt: {zuviel}"


def test_partner_spiegel_ohne_kz_folgt_dem_grundfeld():
    """Die 8 Partner-Felder ohne eigenes Kz tragen das Attribut genau dann, wenn ihr Grundfeld es trägt."""
    abweichend = []
    for f, b in sorted(BINDUNG.items()):
        grund = f.removesuffix("_partner")
        if f == grund or b.get("typ") != "cent" or b.get("elster_kz"):
            continue
        if grund in BINDUNG and bool(b.get("nicht_negativ")) != bool(BINDUNG[grund].get("nicht_negativ")):
            abweichend.append(f)
    assert abweichend == [], f"Partner-Spiegel und Grundfeld weichen bei nicht_negativ ab: {abweichend}"


def test_attribut_steht_nur_an_zahlfeldern_und_die_verlustfelder_tragen_es_nicht():
    assert len(MIT_ATTRIBUT) == 50, f"42 Schematypen ohne Minus + 8 Partner-Spiegel, gezählt {len(MIT_ATTRIBUT)}"
    assert [f for f in MIT_ATTRIBUT if BINDUNG[f].get("typ") not in ("cent", "int")] == []
    assert [f for f in VERLUSTFELDER if f in MIT_ATTRIBUT] == []


# ------------------------------------------------------------------ AK1 / AK2: Schreibweg

@pytest.mark.parametrize("feld_id", MIT_ATTRIBUT)
def test_minus_wird_abgewiesen_null_und_positives_gehen_durch(feld_id):
    with pytest.raises(ValueError, match=rf"^fail-closed \(Vorzeichen\): {feld_id}=-1 darf nicht negativ"):
        _schreibe(feld_id, -1)
    assert _schreibe(feld_id, 0)["wert"] == 0
    assert _schreibe(feld_id, 50000)["wert"] == 50000


@pytest.mark.parametrize("feld_id", VERLUSTFELDER)
def test_verlustfelder_nehmen_weiter_ein_minus(feld_id):
    assert _schreibe(feld_id, -5000000)["wert"] == -5000000


def test_instanzfeld_prueft_die_grenze_der_basis():
    with pytest.raises(ValueError, match=r"^fail-closed \(Vorzeichen\): hh_handwerker_betrag__2=-1"):
        _schreibe("hh_handwerker_betrag__2", -1)
    assert _schreibe("hh_handwerker_betrag__2", 1)["wert"] == 1


def test_ohne_bindung_prueft_der_store_nichts():
    """Die Prüfung hängt wie Auflage T an `bindung=`: Bestandsaufrufe ohne sie bleiben unberührt."""
    assert _schreibe("hh_handwerker_betrag", -1, bindung=None)["wert"] == -1


def test_text_bleibt_ein_typfehler():
    with pytest.raises(ValueError, match=r"^fail-closed \(Typ\)"):
        _schreibe("hh_handwerker_betrag", "-5")


def test_roter_befehl_aus_dem_backlog_minus_ueber_http_ist_422(base):
    """`hh_handwerker_betrag = -5000000` (Cent) über POST /fall/{id}/event: vorher 201, jetzt 422 ohne Event."""
    _req(base, "POST", "/fall", {"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "minus-http"},
         erwarte=201)
    st, body = _req(base, "POST", "/fall/minus-http/event", _laie("hh_handwerker_betrag", -5000000), erwarte=422)
    assert "fail-closed (Vorzeichen)" in body["fehler"] and "hh_handwerker_betrag=-5000000" in body["fehler"]
    _req(base, "POST", "/fall/minus-http/event", _laie("hh_handwerker_betrag", 5000000), erwarte=201)


# ------------------------------------------------------------------ AK3: Laden prüft nie

def test_akte_mit_gespeichertem_minus_laedt_weiter(tmp_path, monkeypatch):
    """Eine Akte, die schon ein Minus trägt (vor der Prüfung geschrieben), lädt und liest weiter; die
    Rechnung setzt es wie bisher auf 0. Das Event wird ohne `bindung=` geschrieben wie ein Altbestand."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    store = ST.leerer_store(2025, fall_id="alt-minus")
    store["scheibe"] = "gesamt"
    _ev = ST.append_event(store, feld_id="hh_handwerker_betrag", wert=-5000000, zustand="bestaetigt",
                          herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                          schreiber="ui:laie", signal={"signal_1": None, "signal_2": "ok"})
    API.speichere_fall("alt-minus", store)
    geladen = API.lade_fall("alt-minus")
    felder, _ = ST.materialisiere(geladen)
    assert felder["hh_handwerker_betrag"]["wert"] == -5000000
