"""Kz-Bindungen: Wert kommt WIRKLICH im XML an, an der richtigen Schema-Stelle.

Wächst mit jedem gebundenen Block (2026-08-05 ff.). Ergänzt das
Ring↔Deklaration-Differential: das prüft, ob ein Feld irgendwo verbucht ist,
dieser Test prüft die konkrete Schema-Stelle und den konkreten Wert.

Warum beides nötig ist: ein Kz kann in `deklaration` stehen (Differential grün) und
trotzdem im falschen Container landen oder in der Cent→Euro-Wandlung kippen. Genau
diese Naht war die Person-B-Lücke — Ring ODER Writer geprüft, nie die Übergabe.

Zusätzlich XSD-Validierung gegen `elster11_E10_<vz>_extern.xsd`: xs:sequence ist
ordnungsempfindlich, ein Kz an der falschen Stelle fällt nur dort auf.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/traverser", "elster/submission"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX        # noqa: E402
import est_mapping             # noqa: E402
import traverser as TR         # noqa: E402

HID = "74931"


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def _xml(felder: dict, bindung: dict) -> str:
    snap = {fid: {"wert": w, "zustand": "bestaetigt"} for fid, w in felder.items()}
    return EX.erzeuge_xml(est_mapping.deklariere(snap, bindung), vz=2025, hersteller_id=HID)


def _pfad_im_xml(xml: str, pfad: tuple[str, ...], wert: str) -> bool:
    """Steht `wert` unter genau diesem Element-Pfad? Namespace-Präfixe werden entfernt,
    damit der Test nicht an der ns0/ns1-Vergabe von ElementTree hängt (spröde)."""
    import re
    flach = re.sub(r"<(/?)[a-zA-Z0-9]+:", r"<\1", xml)
    rest = flach
    for name in pfad:
        i = rest.find(f"<{name}>")
        if i < 0:
            return False
        rest = rest[i:]
    return rest.lstrip().startswith(f"<{pfad[-1]}>{wert}<")


# ---------------------------------------------------------------- Block 1: KiSt § 10 Abs. 1 Nr. 4

def test_kist_kommt_im_xml_an(bindung):
    """kist_gezahlt -> E0107601, kist_erstattet -> E0107602, je an ihrer Schema-Stelle.

    Unterschiedliche Werte, damit eine Verwechslung der beiden Kz auffiele.
    600 EUR gezahlt / 50 EUR erstattet, Eingabe in Cent (Bindung typ=cent).
    """
    xml = _xml({"kist_gezahlt": 60000, "kist_erstattet": 5000}, bindung)

    assert _pfad_im_xml(xml, ("SA", "KiSt", "Gezahlt", "Sum", "E0107601"), "600"), (
        "E0107601 nicht unter SA/KiSt/Gezahlt/Sum mit Wert 600:\n" + xml)
    assert _pfad_im_xml(xml, ("SA", "KiSt", "Erstattet", "E0107602"), "50"), (
        "E0107602 nicht unter SA/KiSt/Erstattet mit Wert 50:\n" + xml)

    # Kein Übersprechen: der Gezahlt-Wert darf nicht im Erstattet-Zweig auftauchen
    erstattet = xml[xml.find("Erstattet"):]
    assert ">600<" not in erstattet, "gezahlter Betrag steht im Erstattet-Zweig"


def test_kist_xml_ist_schema_valide(bindung, tmp_path):
    """Das erzeugte XML validiert gegen das amtliche E10-2025-Schema.

    Fängt Ordnungsfehler (xs:sequence), die ein reiner Stringvergleich nie sieht.
    Ohne ERiC-Doku (kein $ERIC_DIR) wird übersprungen — wie in test_xsd_verify.
    """
    import validate_xsd as VX
    if not VX.find_schema("2025"):
        pytest.skip("elster11_E10_2025_extern.xsd nicht gefunden — ERIC_DIR setzen")

    ziel = tmp_path / "kist.xml"
    ziel.write_text(_xml({"kist_gezahlt": 60000, "kist_erstattet": 5000}, bindung),
                    encoding="utf-8")
    ok, meldung = VX.validate(str(ziel), "2025")
    assert ok, f"KiSt-XML nicht schema-valide: {meldung}"
