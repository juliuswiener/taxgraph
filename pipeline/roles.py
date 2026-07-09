"""Role calls: assemble messages from templates+few-shots and call the client.

Each returns (text, Provenance). In dry-run the client returns a fixture keyed by
`fixture_id`; templates and few-shots are still assembled so the wiring is real.
"""

from __future__ import annotations

from client import OpenRouterClient, RoleConfig
from prompts import build_messages
from provenance import stamp, Provenance


def _call(client: OpenRouterClient, role: RoleConfig, task_content: str,
          fixture_id: str, models_hash: str, exclude: set[str] | None):
    messages = build_messages(role.prompt_template_id, role.fewshot_set_id,
                              {"task_content": task_content},
                              exclude_rule_ids=exclude)
    comp = client.complete(role, messages, fixture_id=fixture_id)
    return comp.text, stamp(role, comp, models_hash)


def extract(client, role: RoleConfig, norm_text: str, models_hash: str,
            exclude=None) -> tuple[str, Provenance]:
    task = f"Norm-Ausschnitt:\n{norm_text}\n\nExtrahiere die Claims als JSON."
    return _call(client, role, task, "worker", models_hash, exclude)


def _signature_block(sig: dict | None) -> str:
    """Prescribe the scope interface so A, B and the hand reference are
    extensionally comparable. Blind stays the semantics, not the naming."""
    if not sig:
        return ""
    ins = "\n".join(f"  input {k} content {v}" for k, v in sig["inputs"].items())
    return (f"\n\nVerbindliche Scope-Signatur (Namen und Typen exakt so verwenden):\n"
            f"declaration scope {sig['scope']}:\n{ins}\n"
            f"  output {sig['output']} content money\n")


def formalize(client, role: RoleConfig, norm_text: str, claims_text: str,
              models_hash: str, exclude=None, signature: dict | None = None
              ) -> tuple[str, Provenance]:
    task = (f"Norm-Ausschnitt:\n{norm_text}\n\nExtrahierte Claims:\n{claims_text}"
            f"{_signature_block(signature)}\n\nFormalisiere nach Catala.")
    return _call(client, role, task, role.role, models_hash, exclude)


def roundtrip(client, role: RoleConfig, norm_text: str, catala_src: str,
              models_hash: str, exclude=None) -> tuple[str, Provenance]:
    task = (f"Original-Norm:\n{norm_text}\n\nCatala-Formalisierung:\n{catala_src}\n\n"
            f"Rueckuebersetzen und vergleichen.")
    return _call(client, role, task, "judge", models_hash, exclude)
