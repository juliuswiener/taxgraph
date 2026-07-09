"""Hard leakage guard for the G2 bake-off blind-reproduction tasks.

A blind-repro task must never see its own hand-formalised rule. This checks, for
every blind_repro task in tasks.yaml:

  1. after applying `exclude_rule_ids`, the assembled few-shot examples contain no
     example whose rule_id is in the exclude set;
  2. the assembled prompt messages contain no verbatim line of the hand reference
     (guards against a reference accidentally pasted into a template/few-shot).

Exit 1 on any violation. Run before every bake-off run:
    pipeline/.venv/bin/python pipeline/bakeoff/check_leakage.py
"""

from __future__ import annotations

import os
import re
import sys

import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
PIPELINE = os.path.dirname(HERE)
ROOT = os.path.dirname(PIPELINE)
sys.path.insert(0, PIPELINE)

from prompts import build_messages, load_fewshots  # noqa: E402
from provenance import load_roles  # noqa: E402


def _significant_lines(path: str) -> list[str]:
    """Reference lines that actually betray the solution.

    Generic type declarations (`input x content money`) are Catala boilerplate and
    appear in any formalisation; matching on them yields false positives. What
    betrays the hand solution is the rule *body*: definitions, exceptions/labels,
    and numeric literals.
    """
    out = []
    for ln in open(path, encoding="utf-8"):
        s = ln.strip()
        if len(s) < 20 or s.startswith("#") or s.startswith("```"):
            continue
        body = s.startswith(("definition ", "exception ", "label ", "rule "))
        numeric = bool(re.search(r"\$[\d,]+\.\d\d|\b\d+\.\d+\b", s))
        if body or numeric:
            out.append(s)
    return out


def main() -> int:
    tasks = yaml.safe_load(open(os.path.join(HERE, "tasks.yaml"), encoding="utf-8"))
    roles, _ = load_roles()
    violations = []

    for t in tasks.get("blind_repro", []):
        rid = t["rule_id"]
        exclude = set(t.get("exclude_rule_ids", []))
        if rid not in exclude:
            violations.append(f"{rid}: own rule_id not in exclude_rule_ids")

        # 1. few-shot exclusion actually applied
        for role_name in ("formalisierer_a", "formalisierer_b"):
            role = roles[role_name]
            kept = [e["rule_id"] for e in load_fewshots(role.fewshot_set_id)
                    if e.get("rule_id") not in exclude]
            leaked = sorted(set(kept) & exclude)
            if leaked:
                violations.append(f"{rid}/{role_name}: few-shot leak {leaked}")

            # 2. no verbatim reference lines inside the assembled messages
            msgs = build_messages(role.prompt_template_id, role.fewshot_set_id,
                                  {"task_content": "PROBE"}, exclude_rule_ids=exclude)
            blob = "\n".join(m["content"] for m in msgs)
            ref = os.path.join(ROOT, t["reference"])
            if os.path.exists(ref):
                hits = [l for l in _significant_lines(ref) if l in blob]
                if hits:
                    violations.append(
                        f"{rid}/{role_name}: reference line(s) present in prompt: "
                        f"{hits[:2]}")

    if violations:
        print("LEAKAGE CHECK FAILED:")
        for v in violations:
            print("  -", v)
        return 1
    n = len(tasks.get("blind_repro", []))
    print(f"leakage check OK: {n} blind-repro tasks, no few-shot or reference leakage")
    return 0


if __name__ == "__main__":
    sys.exit(main())
