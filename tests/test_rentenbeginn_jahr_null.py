"""§ 22 Anlage R: ein Rentenbeginn-Jahr <= 0 sperrt mit eigenem Grund `rentenbeginn_jahr_ungueltig`.

Anlass: Backlog rentenbeginn-jahr-null-wird-datum-0000 (Entscheid rentenbeginn-jahr-null-sperrt-mit-eigenem-grund).
`rentner_renten_beginn_jahr` ist ein int, `est_mapping._kz_wert` macht daraus fuer E1800501 / E1801701 /
E1803202 den Text "01.01.JJJJ" — aus Jahr 0 also "01.01.0000". Das Muster des XSD verlangt vier Ziffern mit
mindestens einer ≠ 0 (E10-2025.xsd:108), ERiC lehnt das Datum ab (rc=610001002), die Erklaerung ist nicht
einreichbar. Die 0 nimmt der Speicher an (sie heisst bei Bereichsfeldern „nichts anzugeben",
test_bindung_bereich_serverseitig.py::test_null_unter_minimum_wird_angenommen); Werte unter 0 weist er ab,
sie erreichen den Guard nur ueber den Direktweg (eDaten-Import, Skript) — dafuer der Direkt-Test unten.

Vorher: aa ohne Freibetrag meldete `rentenfreibetrag_fixierung_offen` (der Nutzer sollte einen Freibetrag zu
einem Rentenbeginn nachtragen, der gar kein Jahr ist), aa mit Freibetrag und beide Leibrenten liefen durch den
Guard bis zu ERiC. Jetzt steht die Jahr-Bedingung in `_beginn_grund` VOR der Freibetrag-Bedingung, fuer Person A,
Person B (zusammen) und jede weitere Rente-Instanz (`__n`).

Die Kontrollen (Jahr 2015, aa mit Freibetrag) beweisen, dass derselbe Aufbau ohne Jahr 0 NICHT sperrt."""
from __future__ import annotations

import pytest

import api as API  # noqa: E402  (conftest legt produkt/haut auf sys.path)
import bescheid_deklaration as BD  # noqa: E402
from test_paket_b_e2e_http import (  # noqa: F401 — base ist eine Fixture
    _catala_da, _rente_instanz_anlegen, _rentner_anlegen, _rentner_kegel, _req, base)

GRUND = "rentenbeginn_jahr_ungueltig"
FREIBETRAG = 600000     # Cent, nur aa-Folgejahr

# (Art, Freibetrag, Alter bei Rentenbeginn): gesetzliche Rente mit und ohne Freibetrag, beide Leibrenten.
RENTEN = [
    pytest.param("gesetzliche_rente", None, 0, id="aa-ohne-freibetrag"),
    pytest.param("gesetzliche_rente", FREIBETRAG, 0, id="aa-mit-freibetrag"),
    pytest.param("private_leibrente", None, 65, id="privat"),
    pytest.param("sonstige_leibrente", None, 65, id="sonstige"),
]
ORTE = ["A", "B", "__2"]   # Person A, Person B (zusammen), zweite Rente von Person A (Instanz)


def _anlegen(base, fid, ort, art, rf, alter, beginn):
    """Legt einen rentner_gesamt-Fall an, dessen Rente am Ort `ort` (art, beginn, rf) traegt; alles andere gueltig."""
    if ort == "A":
        kegel = _rentner_kegel(renten_art=art, beginn=beginn, alter=alter, rentenfreibetrag=rf)
    elif ort == "B":
        kegel = _rentner_kegel(veranlagung="zusammen", renten_art_partner=art, jahresrente_partner=1200000,
                               beginn_partner=beginn, alter_partner=alter, rentenfreibetrag_partner=rf)
    else:
        kegel = _rentner_kegel()                       # Rente 1 = Basis, gueltig (Erstjahr 2025)
    _rentner_anlegen(base, fid, kegel)
    if ort == "__2":
        _rente_instanz_anlegen(base, fid, 2, art, 900000, beginn=beginn, alter=alter, rentenfreibetrag=rf)


def _fid(praefix, ort, art, rf):
    return f"{praefix}-{ort.strip('_') or 'a'}-{art[:4]}-{'rf' if rf else 'x'}".replace("_", "-")


@pytest.mark.parametrize("ort", ORTE)
@pytest.mark.parametrize("art,rf,alter", RENTEN)
def test_jahr_null_sperrt_mit_eigenem_grund(base, ort, art, rf, alter):
    """AK1 + AK2: /ergebnis, /deklaration, /stand und /einreichen nennen denselben Grund, der Klartext das Jahr
    des Rentenbeginns — auch bei aa ohne Freibetrag (dort stand vorher der Freibetrag)."""
    fid = _fid("j0", ort, art, rf)
    _anlegen(base, fid, ort, art, rf, alter, beginn=0)
    _, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert (erg["zahl_cent"], erg["grund"]) == (None, GRUND), erg
    assert "Jahr des Rentenbeginns" in erg["klartext"], erg
    _, dekl = _req(base, "GET", f"/fall/{fid}/deklaration", erwarte=409)
    assert dekl["grund"] == GRUND, dekl
    _, stand = _req(base, "GET", f"/fall/{fid}/stand")
    assert stand["ring_gesperrt"] == GRUND, stand
    _, einr = _req(base, "POST", f"/fall/{fid}/einreichen", {}, erwarte=409)
    assert einr["grund"] == GRUND and einr["eingereicht"] is False, einr


@pytest.mark.parametrize("ort", ORTE)
@pytest.mark.parametrize("art,rf,alter", RENTEN)
def test_gueltiges_jahr_sperrt_nicht(base, ort, art, rf, alter):
    """Kontrolle (Gate-Polaritaet): derselbe Aufbau mit Jahr 2015 — aa bekommt seinen Freibetrag — sperrt nicht."""
    fid = _fid("j15", ort, art, rf)
    _anlegen(base, fid, ort, art, rf or (FREIBETRAG if art == "gesetzliche_rente" else None), alter, beginn=2015)
    _, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
    assert erg["grund"] != GRUND, erg
    if _catala_da():
        assert (erg["grund"], isinstance(erg["zahl_cent"], int)) == ("bestaetigt", True), erg
    _, dekl = _req(base, "GET", f"/fall/{fid}/deklaration")
    assert dekl.get("grund") != GRUND, dekl


@pytest.mark.parametrize("art", ["gesetzliche_rente", "private_leibrente", "sonstige_leibrente"])
@pytest.mark.parametrize("beginn,erwartet", [
    (0, GRUND), (-1, GRUND), (-2025, GRUND),        # die Werte unter 0 weist der Speicher ab; hier der Direktweg
    (2015, None), (1, None),                         # 1 ("01.01.0001") ist fuer das XSD-Muster ein gueltiges Jahr
])
def test_guard_direkt_bei_jahr_bis_null(art, beginn, erwartet):
    """Der Guard selbst, ohne Speicher: `beginn <= 0` sperrt, Jahr 1 und 2015 mit Freibetrag nicht."""
    cfg = API.SCHEIBEN["rentner_gesamt"]
    felder = {"rentner_renten_art": {"wert": art, "zustand": "bestaetigt"},
              "rentner_renten_beginn_jahr": {"wert": beginn, "zustand": "bestaetigt"},
              "rentner_rentenfreibetrag": {"wert": FREIBETRAG, "zustand": "bestaetigt"}}
    assert BD._an_gesamt_sperrgrund(felder, cfg, 2025) == erwartet
