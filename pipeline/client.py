"""OpenRouter client with provider pinning, key masking and provenance.

Key handling (binding):
- The API key is read ONLY from os.environ["OPENROUTER_API_KEY"].
- The key is never read from files, never printed, never logged. All logging
  goes through mask_key(). If the key is missing the client aborts cleanly with
  a hint; it never prompts for the key.
- Provider pinning is mandatory: every request sends the role's provider
  allowlist with allow_fallbacks=false. No :floor/:nitro shortcuts, no `latest`
  aliases (slugs are pinned in pipeline/models.yaml).

Dry-run: if PIPELINE_DRY_RUN=1 (or dry_run=True) the client returns a mocked
response from pipeline/fixtures/ instead of calling OpenRouter, so the whole
gate cascade runs without a key or cost.
"""

from __future__ import annotations

import json
import os
import re
import sys
import time
from dataclasses import dataclass, field
from typing import Any

import requests

OPENROUTER_URL = "https://openrouter.ai/api/v1/chat/completions"
OPENROUTER_MODELS_URL = "https://openrouter.ai/api/v1/models"
_ENV_KEY = "OPENROUTER_API_KEY"


def mask_key(text: str) -> str:
    """Replace any OpenRouter key (sk-or-...) in text with sk-or-***."""
    return re.sub(r"sk-or-[A-Za-z0-9._-]+", "sk-or-***", str(text))


def _require_key() -> str:
    key = os.environ.get(_ENV_KEY)
    if not key:
        sys.stderr.write(
            f"[pipeline] {_ENV_KEY} not set in the environment. Load it before "
            "starting the session (Julius: source ~/.config/taxgraph/openrouter.env). "
            "Aborting; the client never prompts for or reads the key from files.\n")
        raise SystemExit(2)
    return key


def is_dry_run(dry_run: bool | None) -> bool:
    if dry_run is not None:
        return dry_run
    return os.environ.get("PIPELINE_DRY_RUN", "") == "1"


@dataclass
class RoleConfig:
    role: str
    slug: str
    providers: list[str]
    temperature: float = 0.0
    max_tokens: int = 4096
    prompt_template_id: str = ""
    fewshot_set_id: str = ""
    repair_template_id: str = ""


class RoleCallError(RuntimeError):
    """A role call failed after all retries. Carries what is needed to flag the
    task and keep the bake-off running instead of hanging."""

    def __init__(self, role: str, slug: str, providers: list[str], reason: str,
                 kind: str = "role_error"):
        super().__init__(mask_key(f"{role}/{slug}: {reason}"))
        self.role, self.slug, self.providers = role, slug, list(providers)
        self.reason, self.kind = mask_key(reason), kind


@dataclass
class Completion:
    text: str
    role: str
    slug: str
    provider: str | None
    prompt_tokens: int
    completion_tokens: int
    cost_usd: float
    raw: dict = field(default_factory=dict, repr=False)


class OpenRouterClient:
    def __init__(self, dry_run: bool | None = None,
                 timeout: tuple[int, int] = (10, 180),
                 max_retries: int = 2, fixtures_dir: str | None = None):
        """timeout = (connect, read) seconds. A slow-trickling response cannot
        stall a run forever: the read timeout fires between chunks, and the whole
        call is bounded by max_retries attempts."""
        self.dry_run = is_dry_run(dry_run)
        self.timeout = timeout
        self.max_retries = max_retries
        self.fixtures_dir = fixtures_dir or os.path.join(
            os.path.dirname(__file__), "fixtures")
        self._key = None if self.dry_run else _require_key()
        # Provider-Flakiness getrennt von Modellqualitaet messbar machen.
        self.transport: list[dict] = []

    def _event(self, role: str, slug: str, provider: str, kind: str, detail: str = ""):
        self.transport.append({"role": role, "slug": slug, "provider": provider,
                               "kind": kind, "detail": mask_key(detail)[:160]})

    def transport_summary(self) -> dict:
        s: dict = {"retries": 0, "timeouts": 0, "rate_limits": 0, "errors": 0,
                   "by_provider": {}}
        for e in self.transport:
            k = e["kind"]
            if k in s:
                s[k] += 1
            p = e["provider"] or "unknown"
            s["by_provider"].setdefault(p, {"retries": 0, "timeouts": 0,
                                            "rate_limits": 0, "errors": 0})
            if k in s["by_provider"][p]:
                s["by_provider"][p][k] += 1
        return s

    # -- model listing (free GET, used to resolve/verify slugs) ---------------
    def list_models(self) -> list[dict]:
        if self.dry_run:
            return []
        r = requests.get(OPENROUTER_MODELS_URL, timeout=self.timeout,
                         headers={"Authorization": f"Bearer {self._key}"})
        r.raise_for_status()
        return r.json().get("data", [])

    # -- chat completion ------------------------------------------------------
    def complete(self, role: RoleConfig, messages: list[dict],
                 fixture_id: str | None = None) -> Completion:
        if self.dry_run:
            return self._fixture_completion(role, messages, fixture_id)

        body = {
            "model": role.slug,
            "messages": messages,
            "temperature": role.temperature,
            "max_tokens": role.max_tokens,
            "provider": {"only": role.providers, "allow_fallbacks": False},
            "usage": {"include": True},
        }
        headers = {
            "Authorization": f"Bearer {self._key}",
            "Content-Type": "application/json",
            "X-Title": "TaxGraph-Pipeline",
        }
        # Bounded retry on transient upstream errors (timeout / 429 / 5xx) against
        # the SAME pinned provider set. NOT a provider fallback (allow_fallbacks
        # stays false); after the last attempt the call raises RoleCallError so the
        # caller can flag the task and keep going instead of hanging.
        prov = role.providers[0] if role.providers else "unknown"
        r = None
        for attempt in range(self.max_retries + 1):
            try:
                r = requests.post(OPENROUTER_URL, headers=headers, json=body,
                                  timeout=self.timeout)
            except requests.Timeout as e:
                self._event(role.role, role.slug, prov, "timeouts", str(e))
                if attempt < self.max_retries:
                    self._event(role.role, role.slug, prov, "retries", "after timeout")
                    time.sleep(2 ** attempt)
                    continue
                raise RoleCallError(role.role, role.slug, role.providers,
                                    f"timeout after {self.max_retries + 1} attempts",
                                    kind="role_timeout") from None
            except requests.RequestException as e:
                self._event(role.role, role.slug, prov, "errors", str(e))
                if attempt < self.max_retries:
                    self._event(role.role, role.slug, prov, "retries", "after neterr")
                    time.sleep(2 ** attempt)
                    continue
                raise RoleCallError(role.role, role.slug, role.providers,
                                    f"request failed: {e}") from None

            if r.status_code in (429, 500, 502, 503):
                kind = "rate_limits" if r.status_code == 429 else "errors"
                self._event(role.role, role.slug, prov, kind, f"HTTP {r.status_code}")
                if attempt < self.max_retries:
                    self._event(role.role, role.slug, prov, "retries",
                                f"after {r.status_code}")
                    time.sleep(2 ** attempt)
                    continue
            break

        if r.status_code >= 400:
            raise RoleCallError(
                role.role, role.slug, role.providers,
                f"OpenRouter {r.status_code}: {r.text[:300]}",
                kind="role_timeout" if r.status_code == 429 else "role_error")
        data = r.json()
        msg = data["choices"][0].get("message", {}) or {}
        # OpenRouter liefert content=null z.B. bei Refusal oder reinen
        # Reasoning-Antworten. Das darf die Kaskade nicht zum Absturz bringen:
        # leerer Text -> die Gates schlagen sauber fehl.
        choice = msg.get("content") or ""
        if not choice:
            fr = data["choices"][0].get("finish_reason", "?")
            self._event(role.role, role.slug, prov, "errors",
                        f"empty content (finish_reason={fr})")
        usage = data.get("usage", {}) or {}
        provider = data.get("provider") or (data.get("choices", [{}])[0].get("provider"))
        return Completion(
            text=choice, role=role.role, slug=role.slug, provider=provider,
            prompt_tokens=int(usage.get("prompt_tokens", 0)),
            completion_tokens=int(usage.get("completion_tokens", 0)),
            cost_usd=float(usage.get("cost", 0.0) or 0.0),
            raw=data)

    def _fixture_completion(self, role: RoleConfig, messages: list[dict],
                            fixture_id: str | None = None) -> Completion:
        key = fixture_id or role.role
        path = os.path.join(self.fixtures_dir, f"{key}.json")
        if not os.path.exists(path):
            raise FileNotFoundError(
                f"dry-run fixture missing for role '{role.role}': {path}")
        fx = json.load(open(path, encoding="utf-8"))
        return Completion(
            text=fx["text"], role=role.role, slug=role.slug + " (dry-run)",
            provider="dry-run",
            prompt_tokens=fx.get("prompt_tokens", 0),
            completion_tokens=fx.get("completion_tokens", 0),
            cost_usd=0.0, raw={"dry_run": True})
