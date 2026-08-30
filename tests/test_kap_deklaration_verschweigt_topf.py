"""`kapital_semantik_offen` (bescheid_deklaration.py:827) prueft `_positiv(KAP_ERTRAEGE) and
any(_positiv(t) for t in KAP_TOEPFE)` -- feuert nur, wenn BEIDE Seiten > 0 sind. Ein Nutzer, der
`kap_kapitalertraege` wahrheitsgemaess bei 0 belaesst (keine Zinsen/Dividenden) und NUR
`kap_gewinn_sonstige` befuellt (Fonds-/Zertifikate-Gewinn, eigener Topf ohne eigenes Kz), erzeugt
KEINE Ambiguitaet -- der Guard bleibt zurecht still, und `_p20_kapitaleinkuenfte`
(bescheid_einkuenfte.py:231) rechnet korrekt aus dem Topf.

Der Fund liegt eine Ebene tiefer, in `est_mapping.py`s Kz-Verzweigung: E1900701 (kap_kapitalertraege)
wird 1:1 aus dem ROHEN Feldwert deklariert, nicht aus dem effektiven, verrechneten Kapitalertrag.
`test_kap_nulldeklaration.py::test_nur_ein_feld_ungleich_null_reicht_zum_erhalt` haelt genau dieses
Verhalten bereits als SOLL-Zustand fest (E1900701 == 0 bei kap_gewinn_sonstige=12300) -- die
Deklaration ist damit nicht bloss unvollstaendig, sie behauptet aktiv "Kapitalertraege: 0 EUR",
waehrend derselbe Lauf im selben Bescheid einen positiven, versteuerten Betrag ausweist.

Gemessen ueber den ECHTEN Nutzerpfad (API.ergebnis() -> API._ergebnis_roh(), nicht API._bescheid_fn
direkt -- dort feuert `kapital_semantik_offen` vorher und der Zustand ist fuer den Nutzer gar nicht
erreichbar, das war der zurueckgezogene Befund der Nachbarinstanz). HEAD wird beim Testlauf notiert,
siehe Commit-Message.

Suchmuster (Instructor-Auftrag "greppe, wie viele Waechter auf positiv pruefen, wo sie auf gesetzt
pruefen muessten"): `_positiv\\(` in bescheid_deklaration.py, 31 Fundstellen. `_positiv()` selbst
unterscheidet `False` korrekt von `0` (isinstance-Check) -- die Luecke ist nicht Bool-vs-Zahl,
sondern echte-Null-vs-unbeantwortet, und NUR dort ein Fall, wo (a) ein Nutzer die 0 wahrheitsgemaess
meinen kann UND (b) genau deshalb eine reale Ring/Deklaration-Divergenz durchrutscht. Von 31
Fundstellen erfuellen das zwei Familien: KAP (hier, Zeile 827/831, neu) und Gewinn/EUeR
(gewinn_quelle_offen Zeile 837 + luf_euer_offen Zeile 843-845, bereits xfail(strict=True) in
tests/test_luf_euer_offen_asymmetrie.py, Commit 95b8783). Die uebrigen 29 sind Ein-Bedingungs-
Relevanzsperren (0 bedeutet dort "Sachverhalt entfaellt", keine Ambiguitaet moeglich) oder
zustand-Vollstaendigkeitschecks -- kein Fall nach der Prueffrage.

NULL LLM."""
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
    ("bruttoarbeitslohn", 3_000_000), ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0),
    ("vor_rv_ausserhalb_lstb", 0),
    ("kein_gewinn", True), ("kein_vuv", True), ("kein_sonstige", True), ("kein_kap", False),
] + _STAMM + _PFLICHT_ZUSATZ + [("veranlagung", "einzel")]

# Unterscheidbare Betraege (Instructor-Auftrag: kein Wert, der fuer beide Seiten zufaellig passt):
# 30.000 EUR Bruttolohn als Basis, damit die KAP-Verrechnung ueberhaupt sichtbar Steuer aendert.
_KAP_NULL = [("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0),
             ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0), ("kap_gewinn_sonstige", 0)]
_KAP_NUR_SONSTIGE = [("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0),
                     ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
                     ("kap_gewinn_sonstige", 175_000)]   # 1.750,00 EUR, NUR im Topf


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))

    def _lauf(fall_id: str, kap_events: list) -> dict:
        st, resp = API.fall_anlegen(
            {"fall_id": fall_id, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
        assert st == 201, (st, resp)
        for fid, wert in _BASIS + kap_events:
            st, resp = API.event(fall_id, _laie(fid, wert))
            assert st == 201, (fid, wert, st, resp)
        st, erg = API.ergebnis(fall_id)          # der ECHTE Nutzerpfad, nicht _bescheid_fn direkt
        assert st == 200, (st, erg)
        st, dekl = API.deklaration(fall_id)
        assert st == 200, (st, dekl)
        return {
            "grund": erg.get("grund"),
            "zahl_cent": erg.get("zahl_cent"),
            "e1900701": dekl.get("deklaration", {}).get("E1900701"),
        }
    return _lauf


def test_guard_bleibt_still_bei_reiner_topf_angabe(fall):
    """Positivbeleg: `kapital_semantik_offen` tut hier zu Recht nichts -- Topf-only ist ein
    ehrlicher, unambiger Input (Instructor: 'gibt es einen Nutzer, fuer den 0 die wahre Antwort
    ist?' -- ja, wer nur Fondsgewinne hat). Kein xfail: der Guard selbst ist nicht der Defekt."""
    r = fall("kontrolle_topf_only", _KAP_NUR_SONSTIGE)
    assert r["grund"] == "bestaetigt", (
        f"erwartet grund=bestaetigt (Guard bleibt still), tatsaechlich {r['grund']!r} -- wenn der "
        "Guard hier doch feuert, hat er sich geaendert und der Vergleich unten prueft nichts mehr.")
    assert r["zahl_cent"] is not None


@pytest.mark.xfail(
    strict=True,
    reason="est_mapping.py deklariert E1900701 1:1 aus dem rohen Feld kap_kapitalertraege, nicht "
           "aus dem effektiv verrechneten Kapitalertrag. Bei reiner Topf-Angabe (kap_gewinn_sonstige "
           "> 0, kap_kapitalertraege == 0, wahrheitsgemaess) versteuert der Ring den Topf-Betrag "
           "korrekt, aber die Anlage KAP behauptet E1900701=0 -- eine geschriebene Tatsachenbehauptung, "
           "kein fehlender Wert. Marker faellt am Tag des Fixes (XPASS) und zwingt dazu, ihn zu entfernen.")
def test_topf_wird_versteuert_aber_als_null_deklariert(fall):
    """Unterscheidbare Betraege: Baseline (alle KAP-Felder 0) gegen Topf-only (1.750 EUR NUR in
    kap_gewinn_sonstige). Steuer muss steigen -- die Deklaration darf das nicht verschweigen."""
    basis = fall("basis_kap_null", _KAP_NULL)
    topf = fall("topf_nur_sonstige", _KAP_NUR_SONSTIGE)
    assert basis["zahl_cent"] is not None and topf["zahl_cent"] is not None
    delta = topf["zahl_cent"] - basis["zahl_cent"]
    assert delta > 0, (
        f"1.750 EUR kap_gewinn_sonstige muss die Steuer erhoehen -- Delta {delta} Cent "
        f"(Basis {basis['zahl_cent']}, mit Topf {topf['zahl_cent']})")
    assert topf["e1900701"] != 0 or basis["e1900701"] == topf["e1900701"], (
        f"E1900701 bleibt bei 0 stehen (Basis {basis['e1900701']}, mit Topf {topf['e1900701']}), "
        f"waehrend die Steuer um {delta} Cent ({delta / 100:.2f} EUR) gestiegen ist -- die Anlage "
        "KAP behauptet 'keine Kapitalertraege', der Bescheid versteuert sie trotzdem.")
