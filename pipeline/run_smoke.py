"""Smoke test: one candidate, all four roles, one cascade run, cost report.

Default is DRY-RUN (mocked model answers from fixtures/, no key, no cost) so the
full cascade is testable now. Pass --real to actually call OpenRouter (only once
PIPELINE_DRY_RUN is unset and OPENROUTER_API_KEY is in the session).

Run:
  python pipeline/run_smoke.py            # dry-run
  python pipeline/run_smoke.py --real     # real smoke (needs key)
"""

from __future__ import annotations

import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from cascade import run_candidate, write_report  # noqa: E402
from client import mask_key  # noqa: E402

# Smoke candidate: § 20 Abs. 9 Sparer-Pauschbetrag (a simple scalar rule that is
# NOT one of the four hand-formalised rules, so the smoke never leaks a bake-off
# blind target).
SMOKE_CANDIDATE = {
    "id": "sparer_pauschbetrag_p20abs9",
    "norm_text": (
        "§ 20 Abs. 9 Satz 1 EStG: Bei der Ermittlung der Einkuenfte aus "
        "Kapitalvermoegen ist als Werbungskosten ein Betrag von 1 000 Euro "
        "abzuziehen (Sparer-Pauschbetrag); der Abzug der tatsaechlichen "
        "Werbungskosten ist ausgeschlossen."),
    "exclude_rule_ids": [],  # smoke is not a blind-repro task
    # Equivalence gate: compare this output field of A and B over the raster.
    # The rule is a scalar (no inputs), so a single empty raster point suffices.
    "output_field": "betrag",
    "input_types": {},
    "raster": [{}],
}


def main() -> int:
    real = "--real" in sys.argv
    dry_run = not real
    if real and os.environ.get("PIPELINE_DRY_RUN") == "1":
        print("PIPELINE_DRY_RUN=1 is set; unset it to run --real.")
        return 2

    res = run_candidate(SMOKE_CANDIDATE, dry_run=dry_run)
    path = write_report(res)

    print(f"=== Smoke {'(DRY-RUN)' if res.dry_run else '(REAL)'} : {res.candidate_id} ===")
    print(f"models.yaml hash : {res.models_yaml_hash}")
    print(f"module           : {res.module_name}")
    print("gates:")
    for g in res.gate_results:
        print(f"  {g.status:4}  {g.name:14}  {mask_key(g.detail)}")
    print(f"queue status     : {res.queue_status}")
    print("provenance (role / slug / provider / template / fewshots / tokens / cost):")
    for p in res.provenance:
        print(f"  {p['role']:16} {p['slug']:28} {str(p['provider']):10} "
              f"{p['prompt_template_id']:18} {p['fewshot_set_id']:22} "
              f"in={p['prompt_tokens']} out={p['completion_tokens']} "
              f"${p['cost_usd']:.6f}")
    print(f"TOTAL cost       : ${res.total_cost_usd:.6f}")
    print(f"report           : {path}")

    # dry-run success criterion: cascade wired, model gates ok, compiler gates
    # PASS or SKIP (toolchain pending), no unexpected FAIL.
    from gates import FAIL
    fails = [g for g in res.gate_results if g.status == FAIL]
    if fails:
        print(f"FAIL gates: {[g.name for g in fails]}")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
