"""Golden × GETTSIM Cross-Check-Gate (Paket 9, Verifikations-Haertung).

Verankert das Ergebnis von oracle/gettsim/golden_crosscheck.py deterministisch:
  1. jeder heute deckungsgleiche § 32a-Tarif-Fall MUSS deckungsgleich bleiben
     (faengt eine Catala-Tarif-Regression),
  2. jede Abweichung MUSS im Allowlist der zwei bekannten offenen GETTSIM-Bugs
     (#1209 Splitting) liegen — eine NEUE Abweichung (insb. im Grundtarif = #1210-
     Klasse oder ein echter Catala-Fehler) schlaegt rot.

GETTSIM steckt nur im venv312 (oracle/.venv312). Ohne gettsim wird der Test
uebersprungen (laeuft in `make unit` mit reinem python3 als skip; aktiv unter
`. oracle/.venv312/bin/activate && python -m pytest tests/test_gettsim_crosscheck.py`
bzw. `make gettsim-crosscheck`).
"""
from __future__ import annotations

import os
import sys

import pytest

pytest.importorskip("gettsim", reason="GETTSIM nur im venv312 verfuegbar")

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
sys.path.insert(0, os.path.join(ROOT, "oracle", "gettsim"))

import golden_crosscheck as GC  # noqa: E402

# Die heute deckungsgleichen Faelle (Catala == GETTSIM cent-exakt) — Regressions-Anker.
# Abweichungen duerfen NUR aus dieser Allowlist der offenen GETTSIM-Bugs stammen (#1209,
# Splitting-Rundung; BMF-Steuerrechner bestaetigt Catala, s. reports/s02-divergenzen.md).
#
# Alle Eintraege am 2026-08-05 gegen params/<vz>/einkommensteuertarif_p32a.yaml nachgerechnet;
# beide Rundungswege reproduzieren JEDEN der 16 Werte exakt:
#   wir      2 * abrunden(Tarif(abrunden(Z/2)))   § 32a Abs. 5 i.V.m. Abs. 1 S. 6 (Wortlaut)
#   GETTSIM  abrunden(2 * Tarif(Z/2))             rundet erst nach der Verdopplung
# Daher immer +1 EUR bei GETTSIM; bei UNGERADER zvE (g32a_2024_split_43139: 43139/2 = 21569,5)
# +2 EUR, weil dort zusaetzlich die Abrundung von Z/2 fehlt. Nur Splitting betroffen, der
# Grundtarif ist durchgaengig deckungsgleich.
# g32a_2025_split_52150 ist zusaetzlich direkt amtlich belegt: EStH 2025 H 34.2 nennt fuer
# zvE 52.150 EUR woertlich 6.430 EUR (corpus/vwv/esth2025/h-34-2.txt Z. 26) — unser Wert.
# Freigabe Julius 2026-08-05.
KNOWN_DEVIATIONS = {
    "g32a_2024_split_100000", "g32a_2024_split_23634", "g32a_2024_split_43139",
    "g32a_2025_split_24342", "g32a_2025_split_52150", "g32a_2025_split_55555",
    "g32a_2025_split_60000", "g32a_2026_split_77777",
}


@pytest.fixture(scope="module")
def rows():
    tarif, _cov, _nd = GC.collect()
    return GC.crosscheck(tarif)


def test_tarif_faelle_vorhanden(rows):
    assert len(rows) == 47, f"erwartet 47 § 32a-Tarif-Faelle, got {len(rows)}"


def test_keine_neue_abweichung(rows):
    """Jede Abweichung liegt in der Allowlist der bekannten offenen GETTSIM-Bugs."""
    dev = {r["cid"] for r in rows if r["delta_cent"] != 0}
    neu = sorted(dev - KNOWN_DEVIATIONS)
    assert not neu, (
        "NEUE Golden×GETTSIM-Abweichung ausserhalb der bekannten GETTSIM-Bugs "
        f"(#1209/#1210) — Catala-Regression oder echter Fund pruefen:\n  " + "\n  ".join(neu))


def test_deckungsgleiche_bleiben_deckungsgleich(rows):
    """Kein heute deckungsgleicher Fall darf abweichen (Catala-Tarif-Regressionswache)."""
    abgewichen = sorted(r["cid"] for r in rows
                        if r["delta_cent"] != 0 and r["cid"] not in KNOWN_DEVIATIONS)
    assert not abgewichen, f"vormals deckungsgleiche Faelle weichen jetzt ab: {abgewichen}"


def test_keine_grundtarif_abweichung(rows):
    """Grundtarif (einzel) muss cent-exakt mit GETTSIM uebereinstimmen — jede einzel-
    Abweichung waere eine #1210-Klasse-Regression bzw. ein Catala-Grundtarif-Fehler."""
    einzel_dev = sorted(r["cid"] for r in rows
                        if r["verfahren"] == "einzel" and r["delta_cent"] != 0)
    assert not einzel_dev, f"Grundtarif-Abweichung(en) gegen GETTSIM: {einzel_dev}"
