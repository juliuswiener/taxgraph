"""G2 bake-off runner: every task through the cascade, once per pairing.

Before anything runs the leakage guard must pass. Each run writes a report.json
under pipeline/runs/bakeoff/<pairing>/<rule_id>/; blind-reproduction tasks
additionally get `blind_repro_match` from an extensional comparison against the
untouched hand reference.

    python pipeline/bakeoff/run.py --dry-run     # wiring, no key, no cost
    python pipeline/bakeoff/run.py               # real (needs OPENROUTER_API_KEY)
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import traceback

import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
PIPELINE = os.path.dirname(HERE)
ROOT = os.path.dirname(PIPELINE)
sys.path.insert(0, PIPELINE)
sys.path.insert(0, HERE)

from cascade import run_candidate  # noqa: E402
from blind import blind_repro_match  # noqa: E402
from client import mask_key, RoleCallError  # noqa: E402


def leakage_guard() -> None:
    r = subprocess.run([sys.executable, os.path.join(HERE, "check_leakage.py")],
                       capture_output=True, text=True)
    if r.returncode != 0:
        print(r.stdout + r.stderr)
        raise SystemExit("leakage guard failed; aborting bake-off")
    print(r.stdout.strip())


def _family(slug: str) -> str:
    return slug.split("/", 1)[0]


def check_pairings(pairings: list[dict]) -> None:
    """The judge must be the third model family, in every pairing.

    The soundness of the equivalence gate rests on decorrelated errors; a judge
    from the same lab as one of the formalisers is not valid redundancy. This is
    a precondition of the run, not a warning.
    """
    for p in pairings:
        fams = {_family(p["formalisierer_a"]), _family(p["formalisierer_b"]),
                _family(p["judge"])}
        if len(fams) != 3:
            raise SystemExit(
                f"pairing '{p['name']}': judge is not a third family "
                f"({sorted(fams)}); aborting - see models.yaml Judge-Rotation")
    print(f"pairing guard ok: {len(pairings)} pairings, judge always a third family")


def build_candidate(task: dict) -> dict:
    sig = task["signature"]
    return {
        "id": task["rule_id"],
        "norm_text": open(os.path.join(ROOT, task["norm_source"]), encoding="utf-8").read(),
        "exclude_rule_ids": task.get("exclude_rule_ids", []),
        "signature": sig,
        "output_field": sig["output"],
        "input_types": sig["inputs"],
        "raster": task["raster"],
        "test_seed": task.get("test_seed", "none"),
        # G2-Ausnahme: der Bake-off lief ohne Clerk-Test-Gate (keine EStH/BMF-
        # Rechenbeispiele recherchiert). In Phase 3 ist das Gate Pflicht; die
        # G2-Metriken sind deshalb "ohne Test-Gate" zu lesen.
        "test_gate_required": False,
        "output_type": "money",
    }


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--only", help="restrict to one rule_id")
    ap.add_argument("--force", action="store_true", help="recompute finished reports")
    args = ap.parse_args()

    leakage_guard()

    cfg = yaml.safe_load(open(os.path.join(PIPELINE, "models.yaml"), encoding="utf-8"))
    tasks_cfg = yaml.safe_load(open(os.path.join(HERE, "tasks.yaml"), encoding="utf-8"))
    allow = cfg["bakeoff"]["provider_allowlists"]
    pairings = cfg["bakeoff"]["pairings"]
    check_pairings(pairings)

    blind = {t["rule_id"]: t for t in tasks_cfg["blind_repro"]}
    tasks = tasks_cfg["blind_repro"] + tasks_cfg["neu"]
    if args.only:
        tasks = [t for t in tasks if t["rule_id"] == args.only]

    out_root = os.path.join(PIPELINE, "runs", "bakeoff")
    total_cost, t0 = 0.0, time.time()

    for p in pairings:
        overrides = {
            "formalisierer_a": {"slug": p["formalisierer_a"], "providers": allow[p["formalisierer_a"]]},
            "formalisierer_b": {"slug": p["formalisierer_b"], "providers": allow[p["formalisierer_b"]]},
            "judge": {"slug": p["judge"], "providers": allow[p["judge"]]},
        }
        for t in tasks:
            rid = t["rule_id"]
            d = os.path.join(out_root, p["name"], rid)
            done = os.path.join(d, "report.json")
            # Checkpoint: ein fertiger Report wird nie neu berechnet. Ein haengender
            # Call bei Lauf 17 blockiert damit keine 16 fertigen Ergebnisse und
            # erzwingt keinen Neustart von vorn.
            if os.path.exists(done) and not args.force:
                print(f"=== {p['name']} :: {rid} :: skip (checkpoint) ===")
                total_cost += json.load(open(done, encoding="utf-8")).get("total_cost_usd", 0.0)
                continue

            cand = build_candidate(t)
            print(f"\n=== {p['name']} :: {rid} ===", flush=True)
            try:
                res = run_candidate(cand, dry_run=args.dry_run, role_overrides=overrides)
            except RoleCallError as e:
                # Rolle nicht erreichbar -> Task flaggen und WEITERLAUFEN.
                os.makedirs(d, exist_ok=True)
                json.dump({"candidate_id": rid, "pairing": p["name"],
                           "queue_status": e.kind, "failed_role": e.role,
                           "failed_slug": e.slug, "providers": e.providers,
                           "reason": e.reason, "gates": [], "failed_gates": [],
                           "judge_verdict": {}, "provenance": [],
                           "total_cost_usd": 0.0},
                          open(done, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
                print(f"  {e.kind}: {e.role} ({e.slug}) -> weiter", flush=True)
                continue
            except Exception as e:  # noqa: BLE001
                # Auch ein unerwarteter Fehler bekommt einen Report: sonst ist der
                # Lauf weder resumierbar noch in der Auswertung sichtbar.
                os.makedirs(d, exist_ok=True)
                json.dump({"candidate_id": rid, "pairing": p["name"],
                           "queue_status": "run_error",
                           "reason": mask_key(f"{type(e).__name__}: {e}")[:300],
                           "traceback": mask_key(traceback.format_exc())[-800:],
                           "gates": [], "failed_gates": [], "judge_verdict": {},
                           "provenance": [], "total_cost_usd": 0.0},
                          open(done, "w", encoding="utf-8"), ensure_ascii=False, indent=2)
                print("  run_error:", mask_key(str(e))[:150], flush=True)
                continue

            report = {
                "candidate_id": rid,
                "pairing": p["name"],
                "dry_run": res.dry_run,
                "models_yaml_hash": res.models_yaml_hash,
                "queue_status": res.queue_status,
                "gates": res.gates_dict(),
                # `*_first`-Gates sind Diagnose (Erstversuch vor der Reparatur),
                # keine Eskalationsgruende.
                "failed_gates": [g.name for g in res.gate_results
                                 if g.status == "FAIL" and not g.name.endswith("_first")],
                "repaired": res.repaired,
                "judge_verdict": res.judge_verdict,
                "transport": res.transport,
                "provenance": res.provenance,
                "total_cost_usd": round(res.total_cost_usd, 6),
                # Quellen mitschreiben: der Blind-Vergleich laesst sich damit
                # offline nachrechnen, ohne die Modelle erneut zu bezahlen.
                "catala_a": res.catala_a,
                "catala_b": res.catala_b,
                "module_name": res.module_name,
            }

            if rid in blind:
                if res.catala_a:
                    match, status, detail = blind_repro_match(res.catala_a, blind[rid], ROOT)
                else:
                    match, status, detail = None, "blind_no_scope", "no candidate source"
                if match is not None:
                    report["blind_repro_match"] = match
                report["blind_repro_status"] = status
                report["blind_repro_detail"] = detail
                print(f"  blind_repro: {status} ({detail[:70]})", flush=True)

            os.makedirs(d, exist_ok=True)
            json.dump(report, open(os.path.join(d, "report.json"), "w",
                                   encoding="utf-8"), ensure_ascii=False, indent=2)

            total_cost += res.total_cost_usd
            gates = " ".join(f"{g.name}={g.status}" for g in res.gate_results)
            print(f"  {res.queue_status} | ${res.total_cost_usd:.4f}\n  {gates}")

    print(f"\nTOTAL cost ${total_cost:.4f}, wall {time.time()-t0:.0f}s")
    print(f"reports under {out_root}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
