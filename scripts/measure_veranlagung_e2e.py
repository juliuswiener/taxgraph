#!/usr/bin/env python3
"""
E2E‑Mess‑Script für veranlagung‑Fail‑Open.
Verwendet die vorhandene Test‑HTTP‑API (pytest‑fixture `client`).
"""
import json, sys
from pathlib import Path

# Wir importieren den pytest‑client aus dem Test‑modul
sys.path.append('/home/julius/00_projects/168_TaxGraph/taxgraph/tests')
from test_paket_b_e2e_http import client  # noqa: E402

# Fixtures aus test_paket_b_e2e_http.py (IDs, Eingaben)
FIXTURES = [
    # Symmetrisch – beide Partner, zusammen
    ("sym", {
        "veranlagungszeitraum": 2025,
        "bruttoarbeitslohn_a": 4000000,
        "bruttoarbeitslohn_b": 3000000,
        "veranlagung": "zusammen"
    }),
    # Asymmetrisch – Partner B ohne Einkommen, zusammen
    ("asym", {
        "veranlagungszeitraum": 2025,
        "bruttoarbeitslohn_a": 4000000,
        "bruttoarbeitslohn_b": 0,
        "veranlagung": "zusammen"
    }),
    # Einzel – nur A, veranlagung="einzel"
    ("einz", {
        "veranlagungszeitraum": 2025,
        "bruttoarbeitslohn_a": 4000000,
        "veranlagung": "einzel"
    })
]

def run_case(data: dict) -> int:
    # POST /fall (erstellt) → erhalten Fall‑ID via header Location
    resp = client.post('/fall', json=data)
    if resp.status_code != 201:
        raise RuntimeError(f"Fall anlegen fehlgeschlagen: {resp.status_code}")
    location = resp.headers['Location']
    # GET Ergebnis
    erg_resp = client.get(f"{location}/ergebnis")
    if erg_resp.status_code != 200:
        raise RuntimeError(f"Ergebnis holen fehlgeschlagen: {erg_resp.status_code}")
    erg = erg_resp.json()
    return erg.get('zahl_cent', 0)

def main():
    results = []
    for name, base_input in FIXTURES:
        # Baseline (original veranlagung)
        baseline = run_case(base_input)
        # Mutierte Variante – setze veranlagung auf "einzel" (falls existiert)
        mutated_input = dict(base_input)
        mutated_input['veranlagung'] = 'einzel'
        forced = run_case(mutated_input)
        delta = forced - baseline
        direction = 'under-tax' if delta < 0 else 'over-tax' if delta > 0 else 'none'
        results.append({
            'case': name,
            'baseline_ct': baseline,
            'forced_einzel_ct': forced,
            'delta_ct': delta,
            'direction': direction
        })
        print(f"{name}: baseline={baseline}, forced={forced}, delta={delta} ({direction})")

    out_path = Path('/home/julius/00_projects/168_TaxGraph/taxgraph/reports/meas_veranlagung_e2e.json')
    out_path.write_text(json.dumps(results, indent=2, ensure_ascii=False))
    print(f"Ergebnis gespeichert: {out_path}")

if __name__ == "__main__":
    main()
