"""§§ 13-18 Gewinn-Quelle: `gewinn_quelle_offen` sperrt JEDE Betriebsart, sobald EUeR-Komponenten
UND die dazu passende Summe beide gesetzt sind -- unabhaengig von `gewinn_betriebsart`. Der
vollstaendige, ehrliche Nutzer (Summe UND Aufschluesselung angegeben) bekommt keinen Bescheid;
der unvollstaendigere Weg (nur die Summe, EUeR-Felder unangetastet) kommt durch.

Anlass: lage-messung mass gegen einen aelteren Stand (7c0a725), dass dieser Weg betriebsart-
agnostisch `gewinn_quelle_offen` liefert -- anders als der bereits committete `luf_euer_offen`-
Befund (test_luf_euer_offen_asymmetrie.py), der NUR `land_forst` betrifft (Direktwert=0 neben
EUeR-Komponenten). Hier ist der Direktwert NICHT 0, sondern die zur EUeR passende Summe selbst.

Bei HEAD 95b8783 nachgemessen (identischer EUeR-Cluster wie im Nachbartest: betriebseinnahmen
50.000, sonstige_betriebsausgaben 10.000, afa_jahresbetrag 5.000 EUR -> 35.000 EUR Nettosumme,
`einkuenfte_gewinn` auf genau diese 35.000 EUR gesetzt), Befund bestaetigt sich unveraendert
fuer alle drei Zweige des Enums `gewinn_betriebsart`:

  EUeR + Summe=35.000 EUR (Weg 2): grund=gewinn_quelle_offen, zahl_cent=None -- in ALLEN DREI
  nur Summe=35.000 EUR, keine EUeR (Weg 3): grund=bestaetigt, zahl_cent=575700 -- in ALLEN DREI

`gewinn_quelle_offen` (bescheid_deklaration.py:837-838) prueft
`_positiv("einkuenfte_gewinn") and any(_positiv(k) for k in EUER_KOMPONENTEN)`. `_positiv()`
(Zeile 612-615) verlangt `w > 0` fuer Int/Float. Diese Bedingung enthaelt -- anders als
`luf_euer_offen` sechs Zeilen darunter (:843-845, prueft woertlich `== "land_forst"`) -- keine
Abfrage von `gewinn_betriebsart`: sie feuert unabhaengig davon, welcher der drei Zweige gerade
laeuft.

Kein Reparaturweg wird hier vorweggenommen -- ob die Loesung "Rueckfrage statt Sperre",
"Sperre bleibt, aber mit korrigierbarem Ausweg" oder etwas drittes heisst, ist nicht Gegenstand
dieses Tests. Er bindet sich nur an das beobachtbare Symptom: vollstaendige Eingabe liefert
keinen Bescheid, waehrend eine unvollstaendigere denselben Fall durchlaesst.

NULL LLM.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/traverser", "produkt/mapping"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API   # noqa: E402
import audit         # noqa: E402


def _laie(feld_id: str, wert) -> dict:
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"}}


_STAMM = [
    ("stammdaten_nachname", "Meier"), ("stammdaten_vorname", "Klaus"),
    ("stammdaten_geburtsdatum", "01.01.1970"),
    ("stammdaten_strasse", "Teststr."), ("stammdaten_hausnummer", "1"),
    ("stammdaten_plz", "10115"), ("stammdaten_wohnort", "Berlin"),
    ("stammdaten_keine_bankverbindung", True),
    ("stammdaten_art_est_erklaerung", True),
    ("kist_konfession", "keine"),
]

_PFLICHT_ZUSATZ = [
    ("basis_kv", 0), ("basis_pv", 0), ("versicherungsart", "gesetzlich_an"),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("mit_anspruch_auf_zuschuss", False),
    ("ep_arbeitstage", 0), ("ep_eigenes_kfz", False), ("ep_entfernung_km", 0),
    ("ep_oepnv_kosten", 0),
]

_BASIS = [
    ("bruttoarbeitslohn", 0), ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0),
    ("vor_rv_ausserhalb_lstb", 0),
    ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0), ("kap_gewinn_sonstige", 0),
    ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
    ("kein_gewinn", False), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
] + _STAMM

# Identischer EUeR-Cluster wie test_luf_euer_offen_asymmetrie.py: 50.000 - 10.000 - 5.000 =
# 35.000 EUR laufender Gewinn -- echte Betraege, keine Nullen. `einkuenfte_gewinn` traegt in
# diesem Test dieselbe Summe (35.000 EUR = 3.500.000 Cent), nicht 0 -- das unterscheidet ihn
# von luf_euer_offen (dort war der Direktwert 0).
_EUER = [("betriebseinnahmen", 5_000_000), ("sonstige_betriebsausgaben", 1_000_000),
         ("afa_jahresbetrag", 500_000)]
_SUMME = 3_500_000  # 35.000 EUR, exakt die EUeR-Nettosumme


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))

    def _lauf(fall_id: str, betriebsart: str, mit_euer: bool) -> dict:
        st, resp = API.fall_anlegen(
            {"fall_id": fall_id, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
        assert st == 201, (st, resp)
        ereignisse = (_BASIS + _PFLICHT_ZUSATZ + [("veranlagung", "einzel")]
                      + [("gewinn_betriebsart", betriebsart), ("gewinn_bezeichnung", "Testfall")]
                      + (_EUER if mit_euer else [])
                      + [("einkuenfte_gewinn", _SUMME)])
        for fid, wert in ereignisse:
            st, resp = API.event(fall_id, _laie(fid, wert))
            assert st == 201, (fid, wert, st, resp)
        st, erg = API.ergebnis(fall_id)
        assert st == 200, (st, erg)
        return {"grund": erg.get("grund"), "zahl_cent": erg.get("zahl_cent")}
    return _lauf


# ---------------------------------------------------------------- Kontrolle (Unterscheidungskraft)

@pytest.mark.parametrize("betriebsart", ["land_forst", "gewerbe", "selbstaendig"])
def test_nur_summe_liefert_bescheid(fall, betriebsart):
    """Positivkontrolle: dieselbe Summe (35.000 EUR), aber OHNE EUeR-Komponenten, liefert einen
    echten Bescheid. Ohne diese Kontrolle koennte ein kaputter Messaufbau (z.B. eine falsch
    verdrahtete Pflichtfeld-Kette) denselben xfail unten erzeugen, ohne dass gewinn_quelle_offen
    ueberhaupt beteiligt ist -- diese Zeile beweist, dass der Aufbau selbst funktioniert."""
    r = fall(f"kontrolle_{betriebsart}", betriebsart, mit_euer=False)
    assert r["grund"] == "bestaetigt", (
        f"{betriebsart}: erwartet grund=bestaetigt bei reiner Summenangabe, tatsaechlich "
        f"{r['grund']!r} -- wenn das nicht mehr durchkommt, hat sich der Messaufbau selbst "
        "veraendert und der xfail unten prueft nicht mehr, was er behauptet.")
    assert r["zahl_cent"] == 575700


# ---------------------------------------------------------------- der Defekt

@pytest.mark.xfail(
    strict=True,
    reason="bescheid_deklaration.py:837-838 (gewinn_quelle_offen) prueft "
           "_positiv('einkuenfte_gewinn') and any(_positiv(k) for k in EUER_KOMPONENTEN) ohne "
           "Abfrage von gewinn_betriebsart -- die Sperre trifft alle drei Enum-Zweige, sobald "
           "EUeR-Komponenten UND die dazu passende Summe beide gesetzt sind. Marker faellt am "
           "Tag des Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
@pytest.mark.parametrize("betriebsart", ["land_forst", "gewerbe", "selbstaendig"])
def test_euer_und_passende_summe_liefert_keinen_bescheid(fall, betriebsart):
    """Identischer Cluster wie test_nur_summe_liefert_bescheid, zusaetzlich die EUeR-Komponenten
    gesetzt -- der vollstaendigere, ehrlichere Weg. Erwartung nach Fix: ein Bescheid mit Zahl,
    unabhaengig davon, ob die Reparatur die Sperre umbaut oder den Widerspruch anders aufloest."""
    r = fall(f"defekt_{betriebsart}", betriebsart, mit_euer=True)
    assert r["zahl_cent"] is not None and r["grund"] != "gewinn_quelle_offen", (
        f"{betriebsart}: vollstaendige Eingabe (EUeR-Komponenten + passende Summe) liefert "
        f"keinen Bescheid (grund={r['grund']!r}, zahl_cent={r['zahl_cent']!r}) -- der "
        "unvollstaendigere Weg (nur Summe, siehe Kontrolle oben) kommt durch, der ehrliche "
        "vollstaendige nicht.")
