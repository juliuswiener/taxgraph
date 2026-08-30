"""§§ 13-18 Gewinn-Quelle: `luf_euer_offen` sperrt nur `land_forst`, nicht `gewerbe`/`selbstaendig`
-- drei Zweige desselben Enum-Felds `gewinn_betriebsart`, zwei ungeschuetzt.

Anlass: die Untersuchung der Frage "wer schreibt E0800302?" (Instructor-Auftrag 2026-08-30) fand,
dass `gewinn_quelle_offen` (bescheid_deklaration.py, Zeile ~837) nur feuert, wenn `einkuenfte_gewinn`
SELBST > 0 ist -- eine wahrheitsgemaess bestaetigte 0 neben positiven EUeR-Komponenten
(betriebseinnahmen/sonstige_betriebsausgaben/afa_jahresbetrag) rutscht durch, weil `_positiv()`
(dieselbe Datei) 0 identisch zu "nie beantwortet" behandelt. Der Ring
(bescheid_einkuenfte.py::_laufender_gewinn) rechnet in diesem Fall trotzdem aus den
EUeR-Komponenten -- die Deklaration (est_mapping.py::VERZWEIGUNG) schreibt weiter den rohen
Direktwert (hier: 0) nach E0800302/E0803202. Ring und Deklaration laufen auseinander.

Der eigentliche Fund (nicht gesucht, beim Pruefen des KAP-Nachbarn `kapital_semantik_offen`
gefunden): sechs Zeilen unter `gewinn_quelle_offen` steht `luf_euer_offen`
(bescheid_deklaration.py:843-845) -- die exakte Spiegelabsicherung fuer GENAU dieses Muster
("Direktwert 0 UND EUeR-Komponente positiv"), aber mit einer zusaetzlichen Bedingung:
`gewinn_betriebsart == "land_forst"`. Fuer die beiden anderen Zweige desselben Enums (`gewerbe`,
`selbstaendig`) gibt es keine solche Absicherung -- eine Reparatur, die auf halbem Weg
stehengeblieben ist, kein uebersehenes Risiko.

Gemessen (HEAD 7c0a72547a661267a55846cc5724142b077b925d, 2026-08-30), identischer EUeR-Cluster
(betriebseinnahmen 50.000, sonstige_betriebsausgaben 10.000, afa_jahresbetrag 5.000 EUR ->
35.000 EUR laufender Gewinn) in allen drei Zweigen, `einkuenfte_gewinn` je 0 bestaetigt:

  land_forst:    grund=luf_euer_offen,  zahl_cent=None            (gesperrt)
  gewerbe:       grund=bestaetigt,      zahl_cent=575700, E0800302=0
  selbstaendig:  grund=bestaetigt,      zahl_cent=575700, E0803202=0

`selbstaendig` war vorher ungemessen (Zeile 843 prueft woertlich `== "land_forst"`, was fuer sich
allein nur eine Codelesung waere) -- live verhaelt es sich IDENTISCH zu `gewerbe`, nicht zu
`land_forst`.

Ausgangsfrage (Instructor: "kommt der Nutzer wieder heraus?"): fuer den bereits existierenden
`luf_euer_offen`-Block wurde gemessen, dass `einkuenfte_gewinn` NICHT in die normale
Fragen-Warteschlange zurueckkehrt (`API.fragen()`: 197 offene Fragen, `einkuenfte_gewinn` nicht
darunter) -- `_unbeantwortet()` in traverser.py zaehlt nur `zustand in (None, "vorlaeufig")`, eine
ECHTE bestaetigte 0 gilt als beantwortet. `API.ergebnis()`'s `offen`-Liste ist im Guard-Zweig fuer
alle `_an_gesamt_sperrgrund`-Gruende hartkodiert leer (s. test_sperrgrund_klartext_im_browser.py)
-- der Nutzer bekommt (vor dessen Fix) keinen strukturellen Hinweis, welches Feld gemeint ist. Ein
Korrektur-Event auf `einkuenfte_gewinn` selbst (`ersetzt=<event_id>`, dasselbe Muster wie
test_kein_kap_partner_vorab_sperre_und_ausweg.py) wird vom Backend zwar angenommen (201) -- fuehrt
aber NICHT zu einem Bescheid: setzt man `einkuenfte_gewinn` auf die tatsaechliche Summe
(35.000 EUR), springt der Fall von `luf_euer_offen` direkt in `gewinn_quelle_offen` (Zeile 837
gilt fuer ALLE drei Zweige, unabhaengig von `gewinn_betriebsart`) -- weil der Nutzer beide
Eingabewege (Direktwert UND EUeR-Aufschluesselung) tatsaechlich befuellt hat und die Software
genau diese Koexistenz als Widerspruch behandelt, nicht als korrigierbaren Zahlenfehler. Eine
Reparatur, die `gewerbe`/`selbstaendig` einfach nach `luf_euer_offen`s Vorbild absichert, sperrt
also denselben ehrlich antwortenden Nutzer mit derselben fehlenden Ausgangsbeschreibung wie heute
`land_forst` -- eine Sperre ohne Ausgang, keine Reparatur. Fuer die Reparaturentscheidung notiert,
hier nicht mitgeprueft.

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

# Identischer EUeR-Cluster fuer alle drei Zweige: 50.000 - 10.000 - 5.000 = 35.000 EUR laufender
# Gewinn -- echte Betraege (Auftrag: "keine Nullen", sonst sieht der Defekt wie korrekte
# Unterdrueckung aus). Nur einkuenfte_gewinn selbst bleibt die 0 unter Test.
_EUER = [("betriebseinnahmen", 5_000_000), ("sonstige_betriebsausgaben", 1_000_000),
         ("afa_jahresbetrag", 500_000)]

_KZ_JE_BETRIEBSART = {"gewerbe": "E0800302", "selbstaendig": "E0803202"}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))

    def _lauf(fall_id: str, betriebsart: str) -> dict:
        st, resp = API.fall_anlegen(
            {"fall_id": fall_id, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
        assert st == 201, (st, resp)
        ereignisse = (_BASIS + _PFLICHT_ZUSATZ + [("veranlagung", "einzel")]
                      + [("gewinn_betriebsart", betriebsart), ("gewinn_bezeichnung", "Testfall")]
                      + _EUER + [("einkuenfte_gewinn", 0)])
        for fid, wert in ereignisse:
            st, resp = API.event(fall_id, _laie(fid, wert))
            assert st == 201, (fid, wert, st, resp)
        st, erg = API.ergebnis(fall_id)
        assert st == 200, (st, erg)
        st, dekl = API.deklaration(fall_id)
        assert st == 200, (st, dekl)
        kz = _KZ_JE_BETRIEBSART.get(betriebsart)
        return {
            "grund": erg.get("grund"),
            "zahl_cent": erg.get("zahl_cent"),
            "kz_wert": dekl.get("deklaration", {}).get(kz) if kz else None,
        }
    return _lauf


# ---------------------------------------------------------------- Kontrollzweig (bereits richtig)

def test_land_forst_wird_bei_doppelquelle_gesperrt(fall):
    """Positivbeleg: `luf_euer_offen` tut, was es soll -- Direktwert 0 neben positiven
    EUeR-Komponenten sperrt den Bescheid, statt ihn stillschweigend mit einer falschen Deklaration
    zu liefern. Kein xfail: das ist der funktionierende Nachbar, nicht der Defekt."""
    r = fall("kontrolle_land_forst", "land_forst")
    assert r["grund"] == "luf_euer_offen", (
        f"erwartet grund=luf_euer_offen, tatsaechlich {r['grund']!r} -- wenn dieser Kontrollzweig "
        "nicht mehr sperrt, hat sich luf_euer_offen selbst geaendert und der Vergleich unten "
        "prueft nicht mehr, was er behauptet.")
    assert r["zahl_cent"] is None


# ---------------------------------------------------------------- der Defekt

@pytest.mark.xfail(
    strict=True,
    reason="bescheid_deklaration.py:843-845 (luf_euer_offen) prueft woertlich "
           "gewinn_betriebsart == 'land_forst' -- fuer 'gewerbe' und 'selbstaendig', zwei Zweige "
           "desselben Enum-Felds, gibt es keine aequivalente Absicherung. Marker faellt am Tag "
           "des Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
@pytest.mark.parametrize("betriebsart", ["gewerbe", "selbstaendig"])
def test_gewerbeartige_zweige_liefern_bescheid_trotz_doppelquelle(fall, betriebsart):
    """Identischer Input wie test_land_forst_wird_bei_doppelquelle_gesperrt (derselbe EUeR-Cluster,
    dieselbe bestaetigte 0 in einkuenfte_gewinn) -- nur gewinn_betriebsart unterscheidet sich.
    Erwartung nach Fix: entweder sperrt es wie land_forst, oder die deklarierte Kz spiegelt die
    tatsaechlich berechnete Steuer -- in keinem Fall darf 'bestaetigt' + eine positive interne
    Steuer + eine deklarierte 0 gleichzeitig gelten."""
    kz = _KZ_JE_BETRIEBSART[betriebsart]
    r = fall(f"defekt_{betriebsart}", betriebsart)
    assert not (r["grund"] == "bestaetigt" and r["zahl_cent"] and r["kz_wert"] == 0), (
        f"{betriebsart}: grund={r['grund']!r}, zahl_cent={r['zahl_cent']} Cent "
        f"({(r['zahl_cent'] or 0) / 100:.2f} EUR intern berechnet), {kz}={r['kz_wert']} -- Ring "
        "und Deklaration laufen auseinander, exakt der Zustand, den luf_euer_offen fuer land_forst "
        "sechs Zeilen darueber schon verhindert.")
