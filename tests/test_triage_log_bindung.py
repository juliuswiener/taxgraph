"""Triage-Log-Bindungs-Gate (Registry-Ratsche, Zusatz-Order Instructor 2026-07-16).

Jede Item-Triage in pipeline/item_registry/*.yaml, deren Klasse menschliche
Autorisierung verlangt (nicht_echt / nicht_material / bedingung_neu), MUSS entweder
  (a) eine gueltige Log-Bindung haben = ein pipeline/item_registry/entscheidungs_log.yaml
      -Eintrag fuer dasselbe (rule_id, schluessel) mit NICHT-LEEREM entschieden_via, ODER
  (b) in der Grandfather-Baseline (log_bindung_baseline.tsv) stehen.

Hintergrund: Historische Triage loggte nur abweichung-Items; 369 annahme/norm_teil-
Items blieben ohne maschinenlesbare Autorisierung. Entdeckt im N1-Nachlauf 2026-07-16
(Paket 7): eine Triage ueber `item_registry.py aufnehmen` (log-los) statt
`ui.service.submit` (schreibt den Log) band die Adjudikations-ID nicht. Ohne dieses
Gate faellt so ein Loch nur durch manuellen Nachlauf-grep auf.

Die Baseline dokumentiert den Altbestand ehrlich und darf NUR SCHRUMPFEN: ein Eintrag,
dessen Item inzwischen gebunden ist oder nicht mehr existiert, ist zu entfernen (Test 2
erzwingt das). Ein NEUES ungebundenes auth-Item ist NICHT in der Baseline -> Test 1
schlaegt fehl. Baseline-Wachstum ist nur ueber einen sichtbaren Commit-Diff moeglich
und im Review zu begruenden (die Datei ist eingecheckt).
"""
from __future__ import annotations

import glob
import os
import sys

import yaml

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
REGDIR = os.path.join(ROOT, "pipeline", "item_registry")
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import item_registry as IR  # noqa: E402

AUTH = {"nicht_echt", "nicht_material", "bedingung_neu"}
BASELINE = os.path.join(REGDIR, "log_bindung_baseline.tsv")


def _bound_keys() -> set[tuple[str, str]]:
    """(rule_id, schluessel) mit mindestens einem Log-Eintrag mit nicht-leerem entschieden_via."""
    log = yaml.safe_load(open(os.path.join(REGDIR, "entscheidungs_log.yaml"), encoding="utf-8"))
    out = set()
    for e in (log.get("entscheidungen") or []):
        if (e.get("entschieden_via") or "").strip():
            out.add((e["rule_id"], e["schluessel"]))
    return out


def _auth_items() -> set[tuple[str, str]]:
    """(rule_id, schluessel) aller auth-pflichtigen Registry-Items."""
    out = set()
    for f in sorted(glob.glob(os.path.join(REGDIR, "*.yaml"))):
        b = os.path.basename(f)
        if b in ("entscheidungs_log.yaml", "praezedenz.yaml"):
            continue
        reg = yaml.safe_load(open(f, encoding="utf-8"))
        if not isinstance(reg, dict) or "items" not in reg:
            continue
        rid = reg["rule_id"]
        for it in reg.get("items", []):
            if it.get("triage") in AUTH:
                out.add((rid, IR._key(it["art"], it["anker"])))
    return out


def _baseline() -> set[tuple[str, str]]:
    if not os.path.exists(BASELINE):
        return set()
    out = set()
    for line in open(BASELINE, encoding="utf-8"):
        line = line.rstrip("\n")
        if not line or line.startswith("#"):
            continue
        rid, _, k = line.partition("\t")
        assert k, f"Baseline-Zeile ohne Tab-Trenner: {line!r}"
        out.add((rid, k))
    return out


def test_alle_auth_items_gebunden_oder_grandfathered():
    """KERNZWECK: jedes auth-Item ist gebunden ODER in der Baseline. Ein NEUES
    ungebundenes auth-Item (N1-Klasse) schlaegt hier mit Item-Pfad fehl."""
    bound = _bound_keys()
    baseline = _baseline()
    verstoesse = sorted(
        (rid, k) for rid, k in _auth_items()
        if (rid, k) not in bound and (rid, k) not in baseline
    )
    assert not verstoesse, (
        f"{len(verstoesse)} auth-Item(s) ohne Log-Bindung und nicht in der Baseline "
        f"(via ui.service.submit binden oder — nur mit Begruendung — grandfathern):\n"
        + "\n".join(f"  {rid} :: {k}" for rid, k in verstoesse[:40])
    )


def test_baseline_schrumpft_nur():
    """SCHRUMPF-ERZWINGUNG: ein Baseline-Eintrag, dessen Item inzwischen echt gebunden
    ist ODER nicht mehr als auth-Item existiert, ist zu entfernen (Hygiene)."""
    bound = _bound_keys()
    auth = _auth_items()
    baseline = _baseline()
    inzwischen_gebunden = sorted(e for e in baseline if e in bound)
    verwaist = sorted(e for e in baseline if e not in auth)
    fehler = []
    if inzwischen_gebunden:
        fehler.append(
            f"{len(inzwischen_gebunden)} Baseline-Eintrag/-Eintraege inzwischen echt gebunden "
            f"-> aus Baseline entfernen:\n"
            + "\n".join(f"  {rid} :: {k}" for rid, k in inzwischen_gebunden[:40])
        )
    if verwaist:
        fehler.append(
            f"{len(verwaist)} Baseline-Eintrag/-Eintraege ohne existierendes auth-Item "
            f"(stale) -> aus Baseline entfernen:\n"
            + "\n".join(f"  {rid} :: {k}" for rid, k in verwaist[:40])
        )
    assert not fehler, "\n".join(fehler)


def test_baseline_datei_vorhanden_und_parsebar():
    """Format-Wache: Baseline existiert und jede Datenzeile hat einen Tab-Trenner."""
    assert os.path.exists(BASELINE), f"Baseline fehlt: {BASELINE}"
    eintraege = _baseline()
    assert eintraege, "Baseline ist leer — erwartet den grandfathered Altbestand"
