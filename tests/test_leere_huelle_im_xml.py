"""Leere Huelle im E10-Teil: ein Ankreuzfeld "Nein" darf keinen leeren Container hinterlassen.

Beantwortet der Nutzer ein Ankreuzfeld mit "Nein", legt der Writer den zugehoerigen Abschnitt
an und laesst das Feld dann weg. Steht sonst nichts darin, bleibt ein leerer Abschnitt stehen.
Das XSD nimmt ihn an — ERiC nicht (gemessen 2026-10-01, ERiC 44.2.4.0):

    Der Kontext '/AgB[1]/Beh[1]/Geh_Steh_Blind_Hilfl[1]' ist leer.

Warum das Gate STRUKTURELL ist und kein ERiC-Aufruf: ohne echte Hersteller-ID liefert checkESt
rc=610301200 mit LEEREM Fehlerpuffer — das sieht aus wie "fehlerfrei", ist aber ein Abbruch VOR
der Pruefung. Ein ERiC-Gate waere damit falsch-gruen und wuerde nichts pruefen. Ebenso blind ist
xmllint: es sagt "validates" zu genau dieser Huelle.

Die Definition ist an ERiC kalibriert, nicht geraten: in 8 echten Stores mit 30 Huellen werden
30 von 30 von ERiC namentlich beanstandet (der Elementname steht im Fehlertext).

Ticket: elster-leerer-container-neben-ankreuzfeld-nein
"""
from __future__ import annotations

import os
import sys
import xml.etree.ElementTree as ET

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "eingang"))

import elster_xml as EX      # noqa: E402

NS_E10 = "{http://finkonsens.de/elster/elstererklaerung/est/e10/v2025}"

# Attrappe: das Gate laeuft OHNE Hersteller-ID und ohne ERiC. Die ID wird nur verlangt,
# damit erzeuge_xml ueberhaupt serialisiert; geprueft wird die Struktur, nicht das Urteil.
HID = "74931"


def leere_huellen(xml: str) -> list[str]:
    """Elemente im E10-Teil, die keinen Inhalt tragen ausser dem Person-Diskriminator.

    Genau die Klasse, die checkESt mit "Der Kontext ... ist leer" beanstandet: kein Text und
    entweder gar kein Kind oder nur <Person>.
    """
    e10 = ET.fromstring(xml).find(f".//{NS_E10}E10")
    if e10 is None:
        return []
    raus = []
    for el in e10.iter():
        kinder, text = list(el), (el.text or "").strip()
        nur_person = bool(kinder) and all(k.tag == f"{NS_E10}Person" for k in kinder)
        if (not kinder and not text) or (nur_person and not text):
            raus.append(el.tag.replace(NS_E10, ""))
    return raus


def _basis(instanzen: dict) -> dict:
    return {
        "eingaben_konsistent": True,
        "deklaration": {"E0100201": "Muster", "E0100301": "Max"},
        "anlage_instanzen": instanzen,
    }


def _xml(instanzen: dict) -> str:
    return EX.erzeuge_xml(_basis(instanzen), vz=2025, hersteller_id=HID)


def test_ankreuzfeld_nein_hinterlaesst_keine_leere_huelle():
    """GdB 0 + hilflos 'Nein': der Beh-Container darf nicht leer stehenbleiben."""
    xml = _xml({"agb": [{"index": 1, "felder": {"E0109706": False}}]})
    assert leere_huellen(xml) == [], (
        f"Leere Huelle im XML: {leere_huellen(xml)} — checkESt beanstandet sie mit "
        f"'Der Kontext ... ist leer'. Der Writer darf den Pfad erst anlegen, wenn ein "
        f"Blatt folgt."
    )


def test_gegenprobe_feld_nicht_gestellt_erzeugt_keine_huelle():
    """Wird das Feld gar nicht gestellt, entsteht ueberhaupt kein Container."""
    assert leere_huellen(_xml({"agb": [{"index": 1, "felder": {}}]})) == []


def test_gegenprobe_ja_fuellt_die_huelle():
    """'Ja' fuellt den Container — das Gate darf hier nicht anschlagen."""
    assert leere_huellen(_xml({"agb": [{"index": 1, "felder": {"E0109706": True}}]})) == []
