"""Durchstich-Gate: Ring -> deklariere -> erzeuge_xml -> AMTLICHES checkESt.

Warum es das gibt
-----------------
Bis 2026-08-09 wurde der Abgabepfad nirgends gegen die amtliche Plausibilitaetspruefung
gehalten. Es gab zwei Ersatzpruefungen, und beide sind blind fuer das, was ELSTER
tatsaechlich ablehnt:

  * `tests/test_elster_xml.py` und die Zone-Differentialtests validieren gegen das E10-XSD.
    Das XSD winkt Faelle durch, die ERiC abweist — belegt: die von ElementTree erzeugte
    Praefix-Deklaration am <Elster>-Root ist XSD-valide und wurde von ERiC mit
    610301200 abgewiesen (Fix cebb228).
  * `tests/test_einreichen.py` mockt `erzeuge_xml` weg
    (`monkeypatch.setattr(EX, "erzeuge_xml", lambda *a, **k: '<?xml?><Elster/>')`) und
    prueft nur die Verdrahtung des Endpunkts, nie den Inhalt des erzeugten XML.
    Der Docstring von `tests/test_checkest_gate.py` benennt das offen als "Mock-Naht".

Dieser Test schliesst die Luecke: er baut einen echten Fall ueber den echten Pfad und
laesst das amtliche Plugin urteilen.

Bauart: RATSCHE, kein Festwert
------------------------------
Solange die Erklaerung unvollstaendig ist, meldet checkESt eine Zahl von Plausibilitaets-
fehlern. Dieser Test nagelt eine OBERGRENZE fest. Er wird rot, wenn die Zahl STEIGT
(Regression) — und ebenso, wenn sie SINKT, ohne dass jemand die Konstante nachzieht.
Die zweite Richtung ist Absicht: sie zwingt dazu, jeden Fortschritt hier einzutragen,
statt ihn unbemerkt verpuffen zu lassen. Gleiche Bauart wie
`REGELN_OHNE_GROUND_TRUTH` in `tests/test_n_*`.

Ziel ist RESTFEHLER_* == 0. Dann ist der Fall abgabefaehig und der Test wird zum
harten rc==0-Gate.

Ueberspringt sauber, wenn ERiC oder die Hersteller-ID fehlen (credential-freies CI).
Die ID wird nie geloggt.
"""
from __future__ import annotations

import os
import re
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for _sub in ("produkt/import", "produkt/mapping", "produkt/store",
             "produkt/traverser", "elster"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import checkest_gate as CE   # noqa: E402
import elster_xml as EX      # noqa: E402
import est_mapping           # noqa: E402
import store as ST           # noqa: E402
import traverser as TR       # noqa: E402

# Gemessen 2026-08-09 nach Commit cebb228 (Namespace-Fix), ERiC 44.2.4.0, ESt_2025.
#
# einzel   18: Vorsatz-Block (9), Hauptvordruck ESt 1 A (2), Stammdaten Person A (4),
#              Anlage N Steuerklasse + Lohnsteuer (2), Anlage KAP (1).
# zusammen 24: dieselben 18, plus 6 fuer Person B — Person B traegt eigene Stammdaten
#              und eine eigene Steuerklasse. Die Zusammenveranlagung ist also NICHT
#              nur "einzel mit zweitem Lohn"; jeder Stammdaten-Bau muss beide Personen
#              bedienen, sonst bleibt die Haelfte stehen.
RESTFEHLER_EINZEL = 18
RESTFEHLER_ZUSAMMEN = 24

TS = "2026-08-09T22:00:00+00:00"
_H = {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


def _hid() -> str | None:
    """Hersteller-ID aus der Umgebung, sonst aus der gitignoreten .env. Nie loggen."""
    hid = os.environ.get("ELSTER_HERSTELLER_ID")
    if hid:
        return hid
    pfad = os.path.join(ROOT, ".env")
    if not os.path.exists(pfad):
        return None
    for zeile in open(pfad, encoding="utf-8"):
        if zeile.startswith(("ELSTER_HERSTELLER_ID=", "HERSTELLER_ID=")):
            return zeile.split("=", 1)[1].strip().strip('"').strip("'") or None
    return None


_HID = _hid()
_ERIC_DA = bool(_HID) and os.path.isdir(
    os.environ.get("ERIC_DIR", os.path.expanduser("~/02_Software/eric")))

braucht_eric = pytest.mark.skipif(
    not _ERIC_DA,
    reason="ERiC oder Hersteller-ID fehlt — amtliche Pruefung nicht lauffaehig "
           "(credential-freies CI)")


def _b(s, feld_id, wert):
    ST.append_event(store=s, feld_id=feld_id, wert=wert, zustand="bestaetigt",
                    herkunft=_H, schreiber="ui:laie",
                    signal={"signal_1": None, "signal_2": f"ok@{feld_id}"}, ts=TS)


_BASIS_A = (("bruttoarbeitslohn", 6000000), ("vor_an_anteil_rv", 4200000),
            ("vor_ag_anteil_rv", 1200000), ("vor_rv_ausserhalb_lstb", 0),
            ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0),
            ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0),
            ("kein_gewinn", True), ("kein_kap", True),
            ("kein_vuv", True), ("kein_sonstige", True))

_BASIS_B = (("bruttoarbeitslohn_partner", 5000000),
            ("vor_an_anteil_rv_partner", 3500000),
            ("vor_ag_anteil_rv_partner", 1000000),
            ("vor_rv_ausserhalb_lstb_partner", 0),
            ("kap_kapitalertraege_partner", 0), ("kap_gewinn_aktien_partner", 0),
            ("kap_verlust_aktien_partner", 0), ("kap_verlust_sonstige_partner", 0))


def _fall_einzel():
    s = ST.leerer_store(2025, fall_id="durchstich_einzel")
    for f, w in _BASIS_A:
        _b(s, f, w)
    _b(s, "veranlagung", "einzel")
    return s


def _fall_zusammen():
    # Eigener Store: der Store ist fail-closed gegen Ueberschreiben eines aktiven
    # Events (store.py:232), `veranlagung` darf also nur EINMAL gesetzt werden.
    s = ST.leerer_store(2025, fall_id="durchstich_zusammen")
    for f, w in _BASIS_A + _BASIS_B:
        _b(s, f, w)
    _b(s, "veranlagung", "zusammen")
    return s


def _pruefe(store) -> tuple[int, list[str]]:
    """Echter Pfad bis zum amtlichen Plugin. Gibt (rc, Fehlermeldungen) zurueck."""
    snap, _ = ST.materialisiere(store)
    xml = EX.erzeuge_xml(est_mapping.deklariere(snap, TR.lade_bindung()),
                         vz=2025, hersteller_id=_HID)
    rc, antwort = CE.validate(xml, "ESt_2025")
    texte = [" ".join(t.split())
             for t in re.findall(r"<Text>(.*?)</Text>", antwort or "", re.S)]
    return rc, texte


@braucht_eric
@pytest.mark.parametrize("name,bauer,grenze", [
    ("einzel", _fall_einzel, RESTFEHLER_EINZEL),
    ("zusammen", _fall_zusammen, RESTFEHLER_ZUSAMMEN),
])
def test_produkt_xml_erreicht_die_amtliche_pruefung(name, bauer, grenze):
    """Das erzeugte XML muss bis zur Plausibilitaet durchkommen — nicht davor abbrechen.

    Der Unterschied ist entscheidend: rc=610301200 liefert einen LEEREN Fehlerpuffer
    und sieht damit aus wie "keine Beanstandungen", ist aber ein Abbruch VOR der
    Pruefung. Genau diese Verwechslung haelt `klassifiziere_rc` auseinander.
    """
    rc, texte = _pruefe(bauer())
    klasse = CE.klassifiziere_rc(rc)
    assert klasse != "io_gate_nicht_geprueft", (
        f"[{name}] rc={rc}: das XML bricht VOR der Plausibilitaetspruefung ab. "
        f"Ein leerer Fehlerpuffer heisst hier NICHT fehlerfrei. Details in eric.log.")
    assert klasse != "hersteller_id_gesperrt", (
        f"[{name}] rc={rc}: Hersteller-ID abgelehnt — Testaufbau falsch, nicht das XML.")
    assert rc in (CE.RC_OK, CE.RC_PLAUSIBILITAET), (
        f"[{name}] unerwarteter rc={rc} [{klasse}] — weder plausibel noch "
        f"Plausibilitaetsfehler. Das ist ein neuer Fehlermodus.")


@braucht_eric
@pytest.mark.parametrize("name,bauer,grenze", [
    ("einzel", _fall_einzel, RESTFEHLER_EINZEL),
    ("zusammen", _fall_zusammen, RESTFEHLER_ZUSAMMEN),
])
def test_restfehler_ratsche(name, bauer, grenze):
    """Die Zahl amtlicher Beanstandungen darf nur sinken — und muss dann eingetragen werden."""
    rc, texte = _pruefe(bauer())
    if rc == CE.RC_OK:
        assert grenze == 0, (
            f"[{name}] checkESt meldet rc=0 (abgabefaehig!), die Ratsche steht aber "
            f"noch auf {grenze}. Setze RESTFEHLER_{name.upper()} = 0.")
        return
    assert not CE.gekappt_verdacht(""), "Puffer gekappt — Zahl waere nicht belastbar"
    assert len(texte) <= grenze, (
        f"[{name}] REGRESSION: {len(texte)} amtliche Fehler, erlaubt sind {grenze}.\n"
        + "\n".join(f"  - {t[:160]}" for t in texte))
    assert len(texte) == grenze, (
        f"[{name}] FORTSCHRITT: nur noch {len(texte)} statt {grenze} Fehler. "
        f"Trag das ein: RESTFEHLER_{name.upper()} = {len(texte)}.\n"
        f"Verbleibend:\n" + "\n".join(f"  - {t[:160]}" for t in texte))
