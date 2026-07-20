"""Provider-agnostischer LLM-Client: EINE niedrig-level Wahrheit (`complete`) für jeden LLM-Call im Haut/K3-
Bereich. Task-spezifische Wrapper (Chat-Vorschläge, Kontoauszug-Klassifikation) leben NICHT hier, sondern in
der Handler-Schicht (api.py) bzw. beim jeweiligen Writer (kontoauszug_writer.llm_klassifikator_factory) — der
Client kennt keine Aufgaben, nur den rohen Call. stdlib-only (urllib), kein externes Paket.

⚠ CAP-GATED, $0 bis Julius den Key gibt: der echte LLM-Call passiert NUR wenn $LLM_API_KEY gesetzt ist — sonst
LlmNichtVerfuegbar → der Aufrufer fällt sauber auf seine Erklär-Grenze zurück (501/Vertrag bzw. stiller Skip),
nie ein Fake-Wert, nie ein Crash. KEIN Mock-Call (Julius-Regel: „nie Mock-LLM außer explizit verlangt" — Tests
injizieren eine Fixture-Antwort, rufen aber NIE echt).

PROVIDER-AGNOSTISCH: Endpunkt/Modell kommen aus der Umgebung ($LLM_API_BASE OpenAI-kompatibel, $LLM_MODEL), kein
Anbieter hartkodiert — Julius wählt Provider/Modell/Key. Der Schlüssel kommt AUSSCHLIESSLICH aus der Umgebung
(nie im Repo, nie geloggt)."""
from __future__ import annotations

import json
import os
import urllib.request
from dataclasses import dataclass

_TIMEOUT = 30


class LlmNichtVerfuegbar(Exception):
    """Kein $LLM_API_KEY gesetzt oder der Dienst antwortete nicht verwertbar — der /chat-Handler fällt auf die
    reine Erklär-Grenze zurück (die KI schlägt nichts vor, setzt erst recht nichts). Nie crashen, nie Fake."""


def _key() -> str:
    k = os.environ.get("LLM_API_KEY", "").strip()
    if not k:
        raise LlmNichtVerfuegbar("kein LLM_API_KEY in der Umgebung — Chat bleibt reine Erklär-Grenze ($0).")
    return k


def _base() -> str:
    return os.environ.get("LLM_API_BASE", "").strip().rstrip("/")


def _model() -> str:
    return os.environ.get("LLM_MODEL", "").strip()


def _call(messages: list[dict]) -> str:
    """OpenAI-kompatibler Chat-Completions-Call (provider-agnostisch). Roher Antwort-Text. Jeder Fehler
    (kein Base/Modell, Netz, HTTP, JSON) → LlmNichtVerfuegbar (Aufrufer fällt auf die Erklär-Grenze zurück)."""
    base, model = _base(), _model()
    if not base or not model:
        raise LlmNichtVerfuegbar("LLM_API_BASE/LLM_MODEL nicht gesetzt — Provider nicht konfiguriert.")
    body = json.dumps({"model": model, "messages": messages, "temperature": 0,
                       "response_format": {"type": "json_object"}}).encode("utf-8")
    req = urllib.request.Request(
        f"{base}/chat/completions", data=body, method="POST",
        headers={"Authorization": f"Bearer {_key()}", "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as r:
            j = json.loads(r.read())
        return j["choices"][0]["message"]["content"]
    except Exception as e:
        raise LlmNichtVerfuegbar(f"LLM-Aufruf fehlgeschlagen: {type(e).__name__}") from e


@dataclass
class Completion:
    """Rohe LLM-Antwort. Nur `text` — dieser Client kennt keine Aufgabe, nur den Call."""
    text: str


def complete(role: str, messages: list[dict], fixture_id: str | None = None) -> Completion:
    """Die EINE niedrig-level Wahrheit: OpenAI-kompatibler Chat-Call → Completion. Cap-gated wie `_call` (kein
    Key/Base/Modell → LlmNichtVerfuegbar, kein Netz-Zugriff). `role`/`fixture_id` sind Interface-Parität zum
    pipeline-Client (kontoauszug_writer.llm_klassifikator_factory erwartet `client.complete(role, msgs,
    fixture_id=)`) — dieser Haut-Client hat EIN Modell aus der Umgebung, kein Rollen-Routing/Fixture-Replay
    (das lebt in pipeline/client.py, eine andere Baustelle)."""
    return Completion(text=_call(messages))
