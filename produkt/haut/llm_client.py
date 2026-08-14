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
import urllib.error      # explizit: urllib.request zieht es zwar intern nach, aber darauf
import urllib.request    # zu bauen ist Zufall, kein Vertrag — und _call fängt HTTPError
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


def _call(messages: list[dict], schema: dict | None = None) -> str:
    """OpenAI-kompatibler Chat-Completions-Call (provider-agnostisch). Roher Antwort-Text. Jeder Fehler
    (kein Base/Modell, Netz, HTTP, JSON) → LlmNichtVerfuegbar (Aufrufer fällt auf die Erklär-Grenze zurück).

    `schema`: optionales JSON-Schema (Form {"name": …, "strict": True, "schema": {…}}). Ist es gesetzt,
    geht response_format={"type":"json_schema", …} raus statt des schwächeren json_object — der Provider
    erzwingt dann die Struktur, statt sie nur zu erbitten. Ohne Schema bleibt es beim Objekt-Modus.
    Nicht jedes Modell/Endpoint kann das (OpenRouter führt es als `structured_outputs` je Endpoint);
    deepseek/deepseek-v4-pro kann es, gemessen 2026-08-14."""
    base, model = _base(), _model()
    if not base or not model:
        raise LlmNichtVerfuegbar("LLM_API_BASE/LLM_MODEL nicht gesetzt — Provider nicht konfiguriert.")
    schluessel = _key()
    format_teil = ({"type": "json_schema", "json_schema": schema} if schema
                   else {"type": "json_object"})
    body = json.dumps({"model": model, "messages": messages, "temperature": 0,
                       "response_format": format_teil}).encode("utf-8")
    req = urllib.request.Request(
        f"{base}/chat/completions", data=body, method="POST",
        headers={"Authorization": f"Bearer {schluessel}", "Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=_TIMEOUT) as r:
            j = json.loads(r.read())
        return j["choices"][0]["message"]["content"]
    except urllib.error.HTTPError as e:
        # Statuscode UND Provider-Meldung mitnehmen (gekürzt). Vorher stand hier nur
        # type(e).__name__, also "HTTPError" — und damit sah eine erschöpfte Budget-Grenze
        # (403 "Budget limit exceeded") exakt aus wie ein falscher Key, ein totes Modell oder
        # ein Tippfehler in der Base-URL. Gemessen 2026-08-14: die Ursache war nur über einen
        # Direktaufruf am Client vorbei zu finden.
        # KEIN Leak-Risiko: der Key steht im REQUEST-Header, nicht in der Antwort, und diese
        # Meldung erreicht den Nutzer ohnehin nie — der /chat-Handler antwortet mit der festen
        # CHAT_501-Konstante. Sie ist für das Server-Log und die Diagnose da.
        try:
            detail = e.read().decode("utf-8", "replace")[:300]
        except Exception:                                    # noqa: BLE001
            detail = ""
        # Der Schlüssel steht im Request-Header — manche Provider spiegeln ihn in der
        # Fehlerantwort zurück ("invalid key sk-…"). Ohne diese Maskierung landete er über die
        # Ausnahme im Log. Gefunden vom eigenen Test dieser Änderung, nicht im Betrieb.
        if schluessel and schluessel in detail:
            detail = detail.replace(schluessel, "<KEY>")
        raise LlmNichtVerfuegbar(f"LLM-Aufruf fehlgeschlagen: HTTP {e.code} {detail}") from e
    except Exception as e:
        raise LlmNichtVerfuegbar(f"LLM-Aufruf fehlgeschlagen: {type(e).__name__}") from e


@dataclass
class Completion:
    """Rohe LLM-Antwort. Nur `text` — dieser Client kennt keine Aufgabe, nur den Call."""
    text: str


def complete(role: str, messages: list[dict], fixture_id: str | None = None,
             schema: dict | None = None) -> Completion:
    """Die EINE niedrig-level Wahrheit: OpenAI-kompatibler Chat-Call → Completion. Cap-gated wie `_call` (kein
    Key/Base/Modell → LlmNichtVerfuegbar, kein Netz-Zugriff). `role`/`fixture_id` sind Interface-Parität zum
    pipeline-Client (kontoauszug_writer.llm_klassifikator_factory erwartet `client.complete(role, msgs,
    fixture_id=)`) — dieser Haut-Client hat EIN Modell aus der Umgebung, kein Rollen-Routing/Fixture-Replay
    (das lebt in pipeline/client.py, eine andere Baustelle).

    `schema`: optionales JSON-Schema, das der Provider erzwingt (s. _call). Der Kontoauszug-Pfad
    ruft weiter ohne — dessen Fixture-Replay-Signatur bleibt unberührt."""
    return Completion(text=_call(messages, schema=schema))
