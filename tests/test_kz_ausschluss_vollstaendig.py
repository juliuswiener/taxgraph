"""Invariante: JEDES elster_kz der Bindungstabelle ist entweder im E10-Schema
auffindbar ODER in E10_AUSSCHLUSS_DATENART benannt. Ein drittes gibt es nicht.

Fängt die Klasse (nicht den Einzelfall): hätte E6004901 gefunden, bevor ein
Szenario darauf stieß. Derselbe Ratschen-Gedanke wie das Ring-Differential.
"""
from __future__ import annotations

import os
import sys

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
for sub in ("produkt/import", "produkt/mapping", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import elster_xml as EX        # noqa: E402
import traverser as TR         # noqa: E402
import xsd_verify as XV        # noqa: E402


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


def test_jedes_kz_im_schema_oder_ausschluss(bindung):
    """Invariante: Jedes elster_kz der Bindungstabelle muss im E10-Schema
    auffindbar sein ODER in E10_AUSSCHLUSS_DATENART benannt."""
    vz = 2025
    schema = XV._find_schema(vz)
    if not schema:
        pytest.skip(f"E10-{vz}.xsd nicht gefunden — $ERIC_DIR setzen")

    schema_kz, _ = XV.walk(schema, "E10")
    schema_kz_set = set(schema_kz.keys())

    ausschluss = EX.E10_AUSSCHLUSS_DATENART

    fehlend: list[tuple[str, str]] = []  # (feld_id, kz)

    for feld_id, b in bindung.items():
        kz = b.get("elster_kz")
        if not kz:
            continue
        kz_str = str(kz)
        if kz_str in schema_kz_set:
            continue
        if kz_str in ausschluss:
            continue
        fehlend.append((feld_id, kz_str))

    assert not fehlend, (
        f"{len(fehlend)} Kz weder im E10-{vz}-Schema noch in E10_AUSSCHLUSS_DATENART:\n"
        + "\n".join(f"  – {feld_id}: {kz}" for feld_id, kz in fehlend)
        + "\n\nJedes elster_kz der Bindung braucht entweder einen E10-Schema-Pfad oder "
        "einen benannten Eintrag in E10_AUSSCHLUSS_DATENART.")


# ================================================================
# Mutations-Probe: E6004901 aus dem Ausschluss entfernen -> Fund
# ================================================================

def test_mutation_ausschluss_luecke_benennt_e6004901(bindung):
    """Mutations-Probe: wird E6004901 aus E10_AUSSCHLUSS_DATENART entfernt,
    MUSS der Test es benennen."""
    original = EX.E10_AUSSCHLUSS_DATENART.copy()

    # Mutation: E6004901 entfernen
    gemuted = dict(original)
    gemuted.pop("E6004901", None)
    # Durchreichen: die gemutete Kopie als Ausschluss verwenden
    _pruefe_ausschluss_vollstaendig(bindung, 2025, gemuted)


def _pruefe_ausschluss_vollstaendig(bindung, vz, ausschluss_dict):
    """Prüft die Invariante gegen ein gegebenes Ausschluss-Dict (echt oder mutiert)."""
    schema = XV._find_schema(vz)
    if not schema:
        pytest.skip(f"E10-{vz}.xsd nicht gefunden — $ERIC_DIR setzen")

    schema_kz, _ = XV.walk(schema, "E10")
    schema_kz_set = set(schema_kz.keys())

    fehlend: list[tuple[str, str]] = []

    for feld_id, b in bindung.items():
        kz = b.get("elster_kz")
        if not kz:
            continue
        kz_str = str(kz)
        if kz_str in schema_kz_set:
            continue
        if kz_str in ausschluss_dict:
            continue
        fehlend.append((feld_id, kz_str))

    fehlende_kz = {kz for _, kz in fehlend}
    assert "E6004901" in fehlende_kz, (
        "Mutations-Probe: E6004901 aus Ausschluss entfernt, aber Test nennt es nicht.\n"
        f"Gefundene Fehlende: {fehlende_kz}")