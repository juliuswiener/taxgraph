#!/usr/bin/env python3
"""Identifiziere die 21 'Spezialfälle' aus der früheren Gruppierung.

Alle 59 Cent-Felder ohne elster_kz. Filter nach Grund:
- ENDGUELTIG: 16
- Kein eigenes Kz: 5
- Anlage-Instanzen: 11
- VERZWEIGUNG: 6 (jetzt 14 nach neuer Zählung)
- Spezialfälle: 21 (Rest)
"""
import os
import sys
import yaml

ROOT = os.environ.get("TAXGRAPH_ROOT", "/home/julius/00_projects/168_TaxGraph/taxgraph")

# Bindung laden
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

# Filterkriterien
KATEGORIEN = {
    "VERZWEIGUNG": [
        "rentner_jahresrente", "rentner_renten_beginn_jahr",
        "rentner_veraeusserungsgewinn", "einkuenfte_gewinn", "gewinn_bezeichnung",
        "basis_kv", "basis_pv",
        # Neu hinzugekommen in dieser Session
        "basis_kv_partner", "basis_pv_partner", "einkuenfte_gewinn_partner", "gewinn_bezeichnung_partner",
        "rentner_jahresrente_partner", "rentner_renten_beginn_jahr_partner", "rentner_veraeusserungsgewinn_partner",
        "p35c_massnahme_einzelbetrag",
    ],
    "ENDGUELTIG": [],  # werden unten gefüllt: grep nach "ENDGUELTIG:" im grund
    "Kein eigenes Kz": [],
    "Anlage-Instanzen": [],
}

# Felder mit elster_kz=null sammeln
felder_ohne_kz = []
for feld_id, b in bindung_felder.items():
    if b.get("typ") == "cent" and b.get("elster_kz") is None and b.get("askable"):
        grund = b.get("elster_kz_grund", "")
        felder_ohne_kz.append((feld_id, grund))

print("="*90)
print(f"FELDER OHNE KZ (askable, cent, elster_kz=null): {len(felder_ohne_kz)}")
print("="*90)
print()

# Klassifizierung
kategorisiert = {
    "VERZWEIGUNG": [],
    "ENDGUELTIG": [],
    "Kein eigenes Kz": [],
    "Anlage-Instanzen": [],
    "Spezialfälle": [],
}

for feld_id, grund in felder_ohne_kz:
    if feld_id in KATEGORIEN["VERZWEIGUNG"]:
        kategorisiert["VERZWEIGUNG"].append((feld_id, grund))
    elif "ENDGUELTIG" in grund:
        kategorisiert["ENDGUELTIG"].append((feld_id, grund))
    elif "Kein eigenes Kz" in grund or "elster_kz: null" in grund:
        kategorisiert["Kein eigenes Kz"].append((feld_id, grund))
    elif ("Anlage" in grund and "Instanz" in grund) or "INSTANZ" in grund or "_instanzen" in feld_id:
        kategorisiert["Anlage-Instanzen"].append((feld_id, grund))
    else:
        kategorisiert["Spezialfälle"].append((feld_id, grund))

# Ausgabe
for cat in ["ENDGUELTIG", "Kein eigenes Kz", "Anlage-Instanzen", "VERZWEIGUNG", "Spezialfälle"]:
    items = kategorisiert[cat]
    print(f"{cat}: {len(items)}")
    for feld_id, grund in sorted(items):
        grund_kurz = grund[:60] + ("..." if len(grund) > 60 else "")
        print(f"  {feld_id:40s}  {grund_kurz}")
    print()

print("="*90)
print("SPEZIALFÄLLE (die 21) — zur HTTP-Messung:")
print("="*90)
spezial = kategorisiert["Spezialfälle"]
print(f"\nAnzahl: {len(spezial)}")
print("\nFelder:")
for feld_id, grund in sorted(spezial):
    print(f"  {feld_id}")
