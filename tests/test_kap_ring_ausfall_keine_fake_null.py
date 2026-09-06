"""Regressionstest fuer den KAP-Ring-Ausfall-Fix in _mit_ring_werten (bescheid_deklaration.py).

Auftrag main 2026-08-31, HEAD 2729b0d: try/except Exception um den KAP-Block ((2)+(3),
Antrag Guenstigerpruefung E1900401 + genutzter Sparer-Pauschbetrag E1901401) liess bis zu
diesem Fix E1900401=True unbedingt VOR dem try stehen, waehrend E1901401 bei einer Exception
im try auf 0 fiel -- BEIDE Felder mit "zustand": "bestaetigt" hartkodiert, ununterscheidbar
von einem echten Ergebnis "Antrag gestellt, 0 EUR Pauschbetrag genutzt".

Live gemessen (Kontrastpaar, identische Eingabe kap_gewinn_sonstige=175000 Cent):
- runner.catala_sparer_pb funktioniert normal -> E1901401 = 1000 EUR (Kontrolle)
- runner.catala_sparer_pb wirft (hier ueber Monkeypatch simuliert, s.u.) -> E1901401 = 0 EUR,
  VOR dem Fix als "bestaetigt" injiziert und live bis ins abgesendete XML durchgereicht
  (<E1901401>0</E1901401> statt <E1901401>1000</E1901401>) -- kein Waechter zwischen
  _mit_ring_werten und erzeuge_xml() sah den Unterschied, weil beide Faelle "eingaben_
  konsistent": true meldeten (der Waechter prueft nur zustand, und der log log log immer
  "bestaetigt").

ERREICHBARKEIT bei einem GUELTIGEN Veranlagungszeitraum (vz=2025, params/2025/*.yaml alle
vorhanden und valide): NICHT gefunden. Gepruefte Kandidaten:
  - String/Non-Int auf einem typ=cent-Feld -> vom Store-Typwaechter (_pruefe_typ_konformitaet,
    store.py) am Schreibpfad bereits abgelehnt (400), erreicht _mit_ring_werten nie.
  - Extreme int-Werte -> catala_kapital_verrechnung/catala_sparer_pb sind reine max(0, int-int)-
    Arithmetik, kein Overflow in Python moeglich.
  - params/2025/sparer_pauschbetrag_p20_9.yaml und abgeltungssatz_p32d.yaml: beide vorhanden,
    beide mit gueltigem numerischen wert.wert.
Der try/except bleibt trotzdem FALSCH GEBAUT: er faengt jede kuenftige Exception in diesem
Block (Registry-Aenderung, kaputte YAML, Tippfehler in einem Refactor) zur stillen Fake-Null.
Dieser Test erzwingt die Bauform direkt per Monkeypatch, ohne einen echten Auslöser zu
brauchen -- die Bauform ist der Defekt, nicht ein bestimmter vz.

Was dieser Test NICHT behauptet: dass es heute einen erreichbaren Weg gibt, den Ausfall
ueber echte Nutzereingaben bei einem gueltigen vz auszuloesen.
"""
from __future__ import annotations

import os
import sys

ROOT = os.environ.get("TAXGRAPH_ROOT", "/home/julius/00_projects/168_TaxGraph/taxgraph")
for _sub in ("produkt/bescheid", "produkt/haut", "produkt/store", "produkt/traverser",
             "produkt/unsicherheit", "produkt/mapping", "produkt/konsistenz",
             "produkt/eingang", "produkt/engine", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import bescheid_deklaration as BD  # noqa: E402
import runner  # noqa: E402


def _bestaetigt(fid, wert):
    return {"wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fid}"}}


def _basis_felder():
    return {
        "veranlagung": _bestaetigt("veranlagung", "einzel"),
        "kein_kap": _bestaetigt("kein_kap", False),
        "kap_kapitalertraege": _bestaetigt("kap_kapitalertraege", 0),
        "kap_gewinn_aktien": _bestaetigt("kap_gewinn_aktien", 0),
        "kap_verlust_aktien": _bestaetigt("kap_verlust_aktien", 0),
        "kap_verlust_sonstige": _bestaetigt("kap_verlust_sonstige", 0),
        "kap_gewinn_sonstige": _bestaetigt("kap_gewinn_sonstige", 175000),
    }


def test_kontrolle_ring_funktioniert_liefert_echten_betrag():
    """Gegentest (Auflage main): normaler Ring-Lauf bleibt UNVERAENDERT bei 1.000 EUR."""
    felder = BD._mit_ring_werten(_basis_felder(), 2025)
    assert felder["kap_antrag_guenstigerpruefung"]["wert"] is True
    assert felder["kap_antrag_guenstigerpruefung"]["zustand"] == "bestaetigt"
    assert felder["kap_sparer_pauschbetrag_genutzt"]["wert"] == 100000  # CENT = 1.000 EUR
    assert felder["kap_sparer_pauschbetrag_genutzt"]["zustand"] == "bestaetigt"


def test_ring_ausfall_injiziert_keine_fake_null(monkeypatch):
    """Rot vor dem Fix (E1900401=True + E1901401=0, beide 'bestaetigt' -- die stille Null),
    gruen danach (kein Eintrag fuer beide Felder, statt eine erfundene Null zu behaupten)."""
    def _wirft(*a, **kw):
        raise RuntimeError("simulierter Ausfall im KAP-Sparer-PB-Pfad")
    monkeypatch.setattr(runner, "catala_sparer_pb", _wirft)

    felder = BD._mit_ring_werten(_basis_felder(), 2025)

    assert "kap_antrag_guenstigerpruefung" not in felder, (
        "E1900401 (Antrag Guenstigerpruefung) steht trotz Ring-Ausfall als 'bestaetigt' -- "
        "die alte Fake-Null-Bauart ist zurueck")
    assert "kap_sparer_pauschbetrag_genutzt" not in felder, (
        "E1901401 (genutzter Sparer-Pauschbetrag) steht trotz Ring-Ausfall als 'bestaetigt' 0 -- "
        "genau die stille Null, die 1.000 EUR unbelegt verschwinden liess")


def test_ring_ausfall_nur_im_zweiten_aufruf_injiziert_ebenfalls_nichts(monkeypatch):
    """Haerterer Fall: catala_kapital_verrechnung (erster Ring-Aufruf im Block) funktioniert,
    erst catala_sparer_pb (zweiter Aufruf) wirft -- derselbe try umschliesst beide, ein
    Teilerfolg darf trotzdem keinen der beiden Betraege als 'bestaetigt' hinterlassen."""
    orig_verrechnung = runner.catala_kapital_verrechnung
    def _wirft(*a, **kw):
        raise RuntimeError("simulierter Ausfall nur im zweiten Ring-Aufruf")
    monkeypatch.setattr(runner, "catala_sparer_pb", _wirft)

    felder = BD._mit_ring_werten(_basis_felder(), 2025)

    assert "kap_antrag_guenstigerpruefung" not in felder
    assert "kap_sparer_pauschbetrag_genutzt" not in felder
    # Kontrolle: der erste Aufruf lief tatsaechlich durch dieselbe felder-Eingabe erfolgreich
    # (kein Aufruf-Fehler VOR dem eigentlichen Patch-Ziel) -- sonst waere der Test wirkungslos.
    assert orig_verrechnung({"gewinn_aktien": 0, "verlust_aktien": 0,
                              "gewinn_sonstige": 1750, "verlust_sonstige": 0}) == 1750
