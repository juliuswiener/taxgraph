#!/usr/bin/env python3
"""Audit: Verzweigung-Felder: Enum-Zweige vs. Kz-Schlüssel.

Für jedes Feld in VERZWEIGUNG/PARTNER_VERZWEIGUNG:
1. art_feld aus Bindung laden (enum_werte)
2. kz-Dict aus est_mapping laden
3. Fehlende Zweige = enum ∖ kz-keys finden
"""
import os
import sys
import yaml
import json

ROOT = os.environ.get("TAXGRAPH_ROOT", "/home/julius/00_projects/168_TaxGraph/taxgraph")
sys.path.insert(0, os.path.join(ROOT, "produkt", "mapping"))

import est_mapping

# Bindung laden — alle YAML-Dateien in produkt/bindung/
bindung_felder = {}
bindung_dir = os.path.join(ROOT, "produkt", "bindung")
for yaml_file in sorted(os.listdir(bindung_dir)):
    if not yaml_file.endswith(".yaml"):
        continue
    fpath = os.path.join(bindung_dir, yaml_file)
    with open(fpath) as f:
        bindung_doc = yaml.safe_load(f)
        for b in bindung_doc.get("bindungen", []):
            feld_id = b.get("feld_id")
            if feld_id:
                bindung_felder[feld_id] = b

# Kontrollzeilen (die Julius als fehlend gemessen hat)
KONTROLL = [
    ("einkuenfte_gewinn", "gewinn_betriebsart", 3, 2, ["land_forst"]),
    ("gewinn_bezeichnung", "gewinn_betriebsart", 3, 2, ["land_forst"]),
]

# Alle VERZWEIGUNG + PARTNER_VERZWEIGUNG Felder
all_verzw = {}
all_verzw.update(est_mapping.VERZWEIGUNG)
all_verzw.update(est_mapping.PARTNER_VERZWEIGUNG)

print("=" * 90)
print("VERZWEIGUNG-AUDIT: Enum-Zweige vs. Kz-Schlüssel")
print("=" * 90)
print()

# Kontrollzeilen zuerst (um Messmethode zu validieren)
print("KONTROLLZEILEN (Julius-gemessen):")
print("-" * 90)
for feld_id, art_feld_name, expected_enum, expected_kz, expected_fehlend in KONTROLL:
    art_feld_cfg = all_verzw.get(feld_id)
    if not art_feld_cfg:
        print(f"  {feld_id}: NICHT in VERZWEIGUNG gefunden (Fehler in Kontrollzeile)")
        continue

    # Art-Feld aus Bindung laden
    art_feld_b = bindung_felder.get(art_feld_name)
    if not art_feld_b:
        print(f"  {feld_id}  art_feld={art_feld_name}  ART-FELD NICHT IN BINDUNG")
        continue

    enum_vals = art_feld_b.get("enum_werte", [])
    enum_count = len(enum_vals)
    kz_dict = art_feld_cfg["kz"]
    kz_count = len(kz_dict)
    fehlend = set(enum_vals) - set(kz_dict.keys())

    # Validierung der Messmethode
    method_ok = enum_count == expected_enum and kz_count == expected_kz and fehlend == set(expected_fehlend)
    status = "✓ MESSMETHODE OK" if method_ok else "✗ MESSMETHODE DRIFT"

    print(f"  {feld_id:30s}  art_feld={art_feld_name:25s}  enum={enum_count}  kz-keys={kz_count}  "
          f"fehlend={list(fehlend) if fehlend else '→'}  {status}")

print()
print("=" * 90)
print("ALLE VERZWEIGUNG-FELDER (außer Kontrollen):")
print("-" * 90)

result_rows = []
fehlend_gesamt = 0
fehlend_count = 0
zwei_gesamt = 0

for feld_id, cfg in sorted(all_verzw.items()):
    # Überspringen: Kontrollzeilen
    if any(feld_id == k[0] for k in KONTROLL):
        continue

    art_feld_name = cfg.get("art_feld")
    if not art_feld_name:
        print(f"  {feld_id}: FEHLER: kein art_feld in Config")
        continue

    # Art-Feld aus Bindung laden
    art_feld_b = bindung_felder.get(art_feld_name)
    if not art_feld_b:
        print(f"  {feld_id}  art_feld={art_feld_name}  ✗ ART-FELD NICHT IN BINDUNG")
        continue

    enum_vals = art_feld_b.get("enum_werte", [])
    kz_dict = cfg["kz"]
    fehlend = set(enum_vals) - set(kz_dict.keys())

    zwei_gesamt += len(enum_vals)
    fehlend_gesamt += len(fehlend)
    if fehlend:
        fehlend_count += 1

    fehlend_str = list(fehlend) if fehlend else "→"
    row = {
        "feld_id": feld_id,
        "art_feld": art_feld_name,
        "enum": len(enum_vals),
        "kz_keys": len(kz_dict),
        "fehlend": fehlend_str,
    }
    result_rows.append(row)
    print(f"  {feld_id:35s}  art_feld={art_feld_name:30s}  enum={len(enum_vals):2d}  "
          f"kz-keys={len(kz_dict):2d}  fehlend: {fehlend_str}")

print()
print("=" * 90)
print("ZUSAMMENFASSUNG")
print("=" * 90)
print(f"Gesamtzweige (Enum-Werte aller Verzweigungen): {zwei_gesamt}")
print(f"Fehlende Zweige (Enum-Werte ohne Kz-Schlüssel): {fehlend_gesamt}")
print(f"Felder mit mindestens einem fehlenden Zweig: {fehlend_count}")
print()

# JSON für Weiterverarbeitung
output = {
    "kontrollzeilen": KONTROLL,
    "gesamt": {
        "zwei_gesamt": zwei_gesamt,
        "fehlend_gesamt": fehlend_gesamt,
        "felder_mit_luecke": fehlend_count,
    },
    "details": result_rows,
}
print(json.dumps(output, indent=2, ensure_ascii=False))
