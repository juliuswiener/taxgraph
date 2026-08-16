#!/usr/bin/env python3
"""
Mess-Skript für veranlagung Fail-Open Richtung.
Ruft catala-Tarif-Funktionen direkt auf.
"""
import sys
import json

# Catala imports
sys.path.insert(0, '/home/julius/00_projects/168_TaxGraph/taxgraph/oracle/gettsim/_catala/rt')
sys.path.insert(0, '/home/julius/00_projects/168_TaxGraph/taxgraph/oracle/gettsim/_catala')

from pkg import Einkommensteuertarif as E
from catala_runtime import Money, Decimal, Bool, Integer

def run_festzusetzende_est_einzel(bruttoarbeitslohn_ct: int, sonderausgaben_eur: int = 0,
                                    werbungskosten_eur: int = 0, veranlagungszeitraum: int = 2025,
                                    veranlagung: str = "einzel") -> int:
    """Ruft festzusetzende_est_einzel auf."""
    inp = E.FestzusetzendeEstEinzelIn(
        bruttoarbeitslohn_in=Money(f"{bruttoarbeitslohn_ct // 100}.00"),
        sonderausgaben_in=Money(f"{sonderausgaben_eur}.00"),
        veranlagungszeitraum_in=Integer(veranlagungszeitraum),
        werbungskosten_in=Money(f"{werbungskosten_eur}.00"),
    )
    out = E.festzusetzende_est_einzel(inp)
    return int(out.tarifliche_est) * 100  # EURO -> CENT

def run_festzusetzende_est_zusammen(bruttoarbeitslohn_a_ct: int, bruttoarbeitslohn_b_ct: int,
                                     sonderausgaben_gemeinsam_eur: int = 0,
                                     werbungskosten_a_eur: int = 0, werbungskosten_b_eur: int = 0,
                                     veranlagungszeitraum: int = 2025) -> int:
    """Ruft festzusetzende_est_zusammen auf."""
    inp = E.FestzusetzendeEstZusammenIn(
        bruttoarbeitslohn_a_in=Money(f"{bruttoarbeitslohn_a_ct // 100}.00"),
        bruttoarbeitslohn_b_in=Money(f"{bruttoarbeitslohn_b_ct // 100}.00"),
        sonderausgaben_gemeinsam_in=Money(f"{sonderausgaben_gemeinsam_eur}.00"),
        veranlagungszeitraum_in=Integer(veranlagungszeitraum),
        werbungskosten_a_in=Money(f"{werbungskosten_a_eur}.00"),
        werbungskosten_b_in=Money(f"{werbungskosten_b_eur}.00"),
    )
    out = E.festzusetzende_est_zusammen(inp)
    return int(out.tarifliche_est) * 100  # EURO -> CENT

def main():
    results = []

    # Case 1: Symmetrisch - A=40k, B=30k, zusammen
    print("Case 1: Symmetrisch zusammen (A=40k, B=30k)")
    baseline = run_festzusetzende_est_zusammen(4000000, 3000000, 0, 0, 0, 2025)
    # Forced "einzel" would mean: A alone with 40k, B alone with 30k (not applicable to same household)
    # But we can test what happens if we force einzeln for both
    forced_a = run_festzusetzende_est_einzel(4000000, 0, 0, 2025)
    forced_b = run_festzusetzende_est_einzel(3000000, 0, 0, 2025)
    forced = forced_a + forced_b
    delta = forced - baseline
    direction = "under-tax" if delta < 0 else "over-tax"
    results.append(dict(case="sym_zusammen_vs_einzel",
                        baseline_ct=baseline, forced_einzel_ct=forced,
                        delta_ct=delta, direction=direction))
    print(f"  baseline (zusammen): {baseline} ct")
    print(f"  forced (einzel sum): {forced} ct")
    print(f"  delta: {delta} ct ({direction})")

    # Case 2: Asymmetrisch - A=40k, B=0, zusammen
    print("\nCase 2: Asymmetrisch zusammen (A=40k, B=0)")
    baseline2 = run_festzusetzende_est_zusammen(4000000, 0, 0, 0, 0, 2025)
    forced2 = run_festzusetzende_est_einzel(4000000, 0, 0, 2025)
    delta2 = forced2 - baseline2
    direction2 = "under-tax" if delta2 < 0 else "over-tax"
    results.append(dict(case="asym_zusammen_vs_einzel",
                        baseline_ct=baseline2, forced_einzel_ct=forced2,
                        delta_ct=delta2, direction=direction2))
    print(f"  baseline (zusammen): {baseline2} ct")
    print(f"  forced (einzel A): {forced2} ct")
    print(f"  delta: {delta2} ct ({direction2})")

    # Case 3: Einzel - A=40k (no B)
    print("\nCase 3: Einzel (A=40k)")
    baseline3 = run_festzusetzende_est_einzel(4000000, 0, 0, 2025)
    # Force same - already einzel
    delta3 = 0
    direction3 = "none"
    results.append(dict(case="einzel_only",
                        baseline_ct=baseline3, forced_einzel_ct=baseline3,
                        delta_ct=0, direction="none"))
    print(f"  baseline (einzel): {baseline3} ct")

    # Case 4: Zusammen with both 40k
    print("\nCase 4: Zusammen symmetrisch (A=40k, B=40k)")
    baseline4 = run_festzusetzende_est_zusammen(4000000, 4000000, 0, 0, 0, 2025)
    forced4_a = run_festzusetzende_est_einzel(4000000, 0, 0, 2025)
    forced4_b = run_festzusetzende_est_einzel(4000000, 0, 0, 2025)
    forced4 = forced4_a + forced4_b
    delta4 = forced4 - baseline4
    direction4 = "under-tax" if delta4 < 0 else "over-tax"
    results.append(dict(case="sym_zusammen_40_40_vs_einzel",
                        baseline_ct=baseline4, forced_einzel_ct=forced4,
                        delta_ct=delta4, direction=direction4))
    print(f"  baseline (zusammen): {baseline4} ct")
    print(f"  forced (einzel sum): {forced4} ct")
    print(f"  delta: {delta4} ct ({direction4})")

    # Write JSON
    out_path = "/home/julius/00_projects/168_TaxGraph/taxgraph/reports/meas_veranlagung.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(results, f, indent=2, ensure_ascii=False)
    print(f"\nErgebnis gespeichert: {out_path}")
    print(json.dumps(results, indent=2, ensure_ascii=False))

if __name__ == "__main__":
    main()