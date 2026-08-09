"""Gate: Bindungs-`typ` vs XSD-Kz-Typ.

Jedes gebundene Kz muss in GENAU EINEN Prüfzweig fallen — ein Feld, das durch
alle Zweige fällt (unbekannter typ), ist ein FEHLER, kein Skip:

  typ=bool   -> XSD MUSS ein Ja-Typ sein (Ja1 / JaX / JaNein12 / Ja2)
  typ=cent/int -> XSD darf KEIN Ja-Typ sein
  typ=text   -> XSD-Facetten: xs:enumeration -> beispielwert MUSS ein
                Enum-Wert sein; xs:pattern -> beispielwert MUSS alle
                Patterns matchen; sonst Freitext ok

E60xx-Kz (E77/EÜR, andere Datenart) werden separat mit E77-Schema geprüft,
wenn verfügbar. Kz ohne lokales Schema werden in einer SICHTBAREN Liste
gemeldet, nie still übersprungen.

Fängt aktuell (gemessen, HEAD d9107c9):
  - E0205508 (bool an Ganzzahl gebunden)
  - E0500807/E0500808 (text "leibliches Kind" an Enum {1,2,3} gebunden)
  - E0500601/E0500805 (text "01.01.2025 - 31.12.2025" an TT.MM-TT.MM-Pattern)
"""
from __future__ import annotations

import os
import re
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "traverser"))

import est_mapping as EM  # noqa: E402
import xsd_verify as X  # noqa: E402
import traverser as TR  # noqa: E402

pytest.importorskip("yaml")

_SCHEMA_2025 = X._find_schema(2025)
requires_real_schema = pytest.mark.skipif(
    _SCHEMA_2025 is None,
    reason="lokales ERiC-E10-2025.xsd nicht gefunden ($ERIC_DIR/~/02_Software/eric)")

_E77_SCHEMA_2025 = X._find_schema(2025, "E77-{jahr}.xsd")


def test_ist_ja_typ_erkennung():
    """Unit-Test: die 4 Ja-Typen + Nicht-Ja-Typen."""
    assert X.ist_ja_typ("Ja1BaseCType")
    assert X.ist_ja_typ("Ja1BaseCType_RABE")
    assert X.ist_ja_typ("JaXBaseCType")
    assert X.ist_ja_typ("JaXBaseCType_RABE")
    assert X.ist_ja_typ("JaNein12BaseCType")
    assert X.ist_ja_typ("JaNein12BaseCType_RABE")
    assert X.ist_ja_typ("Ja2BaseCType")
    assert X.ist_ja_typ("Ja2BaseCType_RABE")
    assert not X.ist_ja_typ("GanzzahlNichtNegOhneFuehrNull_MaxVK12_CType")
    assert not X.ist_ja_typ("GanzzahlNichtNegOhneFuehrNull_MaxVK12_CType_RABE")
    assert not X.ist_ja_typ("StringBaseWithAliasCType")
    assert not X.ist_ja_typ("Enum_Kind_K_Verh_K_Verh_A_E0500807_CType")
    assert not X.ist_ja_typ("xs:string")


@requires_real_schema
def test_bindungs_typ_vs_xsd_typ():
    """JEDES gebundene Kz in genau einem Prüfzweig — kein stiller Durchfall."""
    bindung = TR.lade_bindung()

    # E10- und E77-Typ-Facetten (E77 optional)
    e10_meta = X._resolve_kz_meta(_SCHEMA_2025, "E10")
    e77_meta: dict[str, dict] = {}
    if _E77_SCHEMA_2025 is not None:
        e77_meta = X._resolve_kz_meta(_E77_SCHEMA_2025, "E77")

    def _schema_fuer_kz(kz: str) -> dict[str, dict]:
        return e77_meta if kz[1:3] == "60" else e10_meta

    mismatches: list[str] = []
    ungeprueft: list[str] = []

    for feld_id, b in sorted(bindung.items()):
        kz = b.get("elster_kz")
        if not kz:
            continue
        typ = b.get("typ")

        meta = _schema_fuer_kz(kz).get(kz)
        if meta is None:
            ungeprueft.append(f"{feld_id} (Kz {kz}): Kz nicht im Schema")
            continue

        # GENAU EIN Zweig — ein neuer typ fällt durch und wird gemeldet
        if typ == "bool":
            if not meta["is_ja"]:
                mismatches.append(
                    f"{feld_id}: typ=bool, Kz {kz}, XSD type={meta['type_name']} "
                    f"(kein Ja-Typ)")
        elif typ in ("cent", "int"):
            if meta["is_ja"]:
                mismatches.append(
                    f"{feld_id}: typ={typ}, Kz {kz}, XSD type={meta['type_name']} "
                    f"(Ja-Typ, aber Betrag)")
        elif typ == "enum" and feld_id in EM.WERTEKODIERUNG:
            # Klasse i (est_mapping.WERTEKODIERUNG): enum_werte sind Laien-Vokabular, KEIN
            # 1:1-Passthrough — geprüft wird die ÜBERSETZUNG (die amtlichen Codes), nicht die
            # Laien-Werte selbst gegen die XSD-enum.
            if not meta["enums"]:
                mismatches.append(
                    f"{feld_id}: typ=enum (Wertekodierung), Kz {kz}, XSD keine enumeration")
            else:
                for laie, code in EM.WERTEKODIERUNG[feld_id]["code"].items():
                    if code not in meta["enums"]:
                        mismatches.append(
                            f"{feld_id}: Wertekodierung {laie!r} -> {code!r}, Kz {kz}, "
                            f"XSD erlaubt nur {meta['enums']}")
        elif typ == "enum":
            enum_werte = b.get("enum_werte", [])
            if not meta["enums"]:
                mismatches.append(
                    f"{feld_id}: typ=enum, Kz {kz}, XSD keine enumeration — "
                    f"enum_werte {enum_werte} können nicht gemappt werden")
            else:
                for ev in enum_werte:
                    if str(ev) not in meta["enums"]:
                        mismatches.append(
                            f"{feld_id}: typ=enum, Kz {kz}, enum_werte enthalten {ev!r}, "
                            f"XSD erlaubt nur {meta['enums']}")
        elif typ == "datum":
            beispiel = b.get("beispielwert")
            if meta["patterns"]:
                if beispiel is None:
                    ungeprueft.append(f"{feld_id} (Kz {kz}): datum an Pattern "
                                      f"{meta['patterns']} gebunden, aber kein beispielwert")
                elif not all(re.fullmatch(p, str(beispiel)) for p in meta["patterns"]):
                    mismatches.append(
                        f"{feld_id}: typ=datum, Kz {kz}, XSD pattern={meta['patterns']}, "
                        f"beispielwert {beispiel!r} matcht nicht")
            # kein Pattern -> Datumsfeld ohne Format-Facette, ok
        elif typ == "text":
            beispiel = b.get("beispielwert")
            if meta["enums"]:
                if beispiel is None:
                    ungeprueft.append(f"{feld_id} (Kz {kz}): text an Enum "
                                      f"{meta['enums']} gebunden, aber kein beispielwert")
                elif str(beispiel) not in meta["enums"]:
                    mismatches.append(
                        f"{feld_id}: typ=text, Kz {kz}, XSD enum={meta['enums']}, "
                        f"beispielwert {beispiel!r} nicht in enum")
            elif meta["patterns"]:
                if beispiel is None:
                    ungeprueft.append(f"{feld_id} (Kz {kz}): text an Pattern "
                                      f"{meta['patterns']} gebunden, aber kein beispielwert")
                elif not all(re.fullmatch(p, str(beispiel)) for p in meta["patterns"]):
                    mismatches.append(
                        f"{feld_id}: typ=text, Kz {kz}, XSD pattern={meta['patterns']}, "
                        f"beispielwert {beispiel!r} matcht nicht")
            # weder enum noch pattern -> Freitext, ok
        else:
            mismatches.append(f"{feld_id}: unbekannter typ {typ!r} — "
                              f"fällt durch alle Prüfzweige")

    if ungeprueft:
        print("--- Nicht prüfbar (sichtbar, kein stiller Skip) ---")
        for u in ungeprueft:
            print(f"  {u}")

    assert not mismatches, (
        "Bindungs-Typ ↔ XSD-Typ Mismatches:\n" + "\n".join(mismatches))