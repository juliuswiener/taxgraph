"""Prompt templates and few-shot sets, versioned and per-task disjoint.

Templates live in pipeline/prompts/templates/<id>.txt, few-shot sets in
pipeline/prompts/fewshots/<id>.json. The `@version` suffix is part of the id and
is recorded in provenance.

Leakage protection: a few-shot set is a list of examples each tagged with a
`rule_id`. `build_messages(..., exclude_rule_ids=...)` drops examples whose
rule_id is in the exclude set, so a bake-off task that must blind-reproduce a
hand rule never sees that rule in its own few-shots.
"""

from __future__ import annotations

import json
import os

PROMPTS_DIR = os.path.dirname(__file__)
TPL_DIR = os.path.join(PROMPTS_DIR, "prompts", "templates")
FS_DIR = os.path.join(PROMPTS_DIR, "prompts", "fewshots")


def load_template(template_id: str) -> str:
    path = os.path.join(TPL_DIR, template_id + ".txt")
    return open(path, encoding="utf-8").read()


def load_fewshots(fewshot_id: str) -> list[dict]:
    path = os.path.join(FS_DIR, fewshot_id + ".json")
    if not os.path.exists(path):
        return []
    return json.load(open(path, encoding="utf-8")).get("examples", [])


def build_messages(template_id: str, fewshot_id: str, task_vars: dict,
                   exclude_rule_ids: set[str] | None = None) -> list[dict]:
    """Assemble OpenRouter chat messages: system template + few-shots + task."""
    exclude = exclude_rule_ids or set()
    tpl = load_template(template_id)
    messages = [{"role": "system", "content": tpl}]
    for ex in load_fewshots(fewshot_id):
        if ex.get("rule_id") in exclude:
            continue  # leakage protection
        messages.append({"role": "user", "content": ex["input"]})
        messages.append({"role": "assistant", "content": ex["output"]})
    # task content: template placeholders filled explicitly (temperature 0)
    task = task_vars.get("task_content", "")
    messages.append({"role": "user", "content": task})
    return messages
