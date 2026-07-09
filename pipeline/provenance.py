"""Provenance + config loading for the formalisation pipeline (Nachtrag 3).

Every pipeline output carries: role, model slug, provider, models.yaml hash,
prompt-template version, few-shot-set version, timestamp, token counters, cost.
A run is only valid if all roles ran against one frozen models.yaml (its hash is
recorded on every stamp and checked to be identical across a run).
"""

from __future__ import annotations

import datetime
import hashlib
import os
from dataclasses import dataclass, asdict

import yaml

from client import RoleConfig, Completion, mask_key

PIPELINE_DIR = os.path.dirname(__file__)
MODELS_YAML = os.path.join(PIPELINE_DIR, "models.yaml")


def models_yaml_hash(path: str = MODELS_YAML) -> str:
    with open(path, "rb") as f:
        return "sha256:" + hashlib.sha256(f.read()).hexdigest()[:16]


def load_roles(path: str = MODELS_YAML) -> tuple[dict[str, RoleConfig], str]:
    """Return (role_name -> RoleConfig, models.yaml hash)."""
    data = yaml.safe_load(open(path, encoding="utf-8"))
    roles = {}
    for name, r in data["roles"].items():
        roles[name] = RoleConfig(
            role=name, slug=r["slug"], providers=list(r["providers"]),
            temperature=float(r.get("temperature", 0.0)),
            max_tokens=int(r.get("max_tokens", 4096)),
            prompt_template_id=r.get("prompt_template_id", ""),
            fewshot_set_id=r.get("fewshot_set_id", ""))
    return roles, models_yaml_hash(path)


def now_iso() -> str:
    return datetime.datetime.now(datetime.timezone.utc).isoformat(timespec="seconds")


@dataclass
class Provenance:
    role: str
    slug: str
    provider: str | None
    models_yaml_hash: str
    prompt_template_id: str
    fewshot_set_id: str
    timestamp: str
    prompt_tokens: int
    completion_tokens: int
    cost_usd: float

    def to_dict(self) -> dict:
        return asdict(self)


def stamp(role: RoleConfig, comp: Completion, models_hash: str) -> Provenance:
    return Provenance(
        role=comp.role, slug=comp.slug, provider=comp.provider,
        models_yaml_hash=models_hash,
        prompt_template_id=role.prompt_template_id,
        fewshot_set_id=role.fewshot_set_id,
        timestamp=now_iso(),
        prompt_tokens=comp.prompt_tokens,
        completion_tokens=comp.completion_tokens,
        cost_usd=comp.cost_usd)


def redact(obj) -> str:
    """String form with any leaked key masked (defense in depth for logs)."""
    return mask_key(str(obj))
