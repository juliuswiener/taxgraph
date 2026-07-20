"""Provider-agnostischer LLM-Client für den Chat-Berater (K1). Der LLM SCHLÄGT Feld-Werte VOR — er setzt sie
NIE: die Store-Auflage A erzwingt für jeden `llm:`-Schreiber strukturell herkunft=llm_vorschlag, zustand=
vorlaeufig, signal_2=null (nie in die Summe ohne menschlichen Hold-Confirm). stdlib-only (urllib), kein externes
Paket.

⚠ CAP-GATED, $0 bis Julius den Key gibt: der echte LLM-Call passiert NUR wenn $LLM_API_KEY gesetzt ist — sonst
LlmNichtVerfuegbar → der /chat-Handler fällt sauber auf die Erklär-Grenze zurück (501/Vertrag), nie ein Fake-Wert,
nie ein Crash. KEIN Mock-Call (Julius-Regel: „nie Mock-LLM außer explizit verlangt" — der Test injiziert eine
Fixture-Antwort, ruft aber NIE echt).

PROVIDER-AGNOSTISCH: Endpunkt/Modell kommen aus der Umgebung ($LLM_API_BASE OpenAI-kompatibel, $LLM_MODEL), kein
Anbieter hartkodiert — Julius wählt Provider/Modell/Key. Der Schlüssel kommt AUSSCHLIESSLICH aus der Umgebung
(nie im Repo, nie geloggt)."""
from __future__ import annotations

import json
import os
import urllib.request

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


def _prompt(freitext: str, katalog: list[dict]) -> list[dict]:
    """Baut die OpenAI-kompatible messages-Liste. System-Regel: die KI darf AUSSCHLIESSLICH die Felder aus dem
    übergebenen Katalog vorschlagen (askable + vorschlagbar; der Store-Katalog-Check ist die zweite Verteidigung),
    NUR als Vorschlag, mit Feld-Metadaten (fragetext/typ/bereich/enum). Antwort = striktes JSON."""
    felder = "\n".join(
        f"- {f['feld_id']}: {f.get('fragetext_laie', '')}"
        f" (Typ {f.get('typ', '')}"
        + (f", Bereich {f['bereich']}" if f.get("bereich") else "")
        + (f", Werte {f['enum_werte']}" if f.get("enum_werte") else "")
        + ")"
        for f in katalog)
    system = (
        "Du bist ein Steuer-Assistent, der aus der Freitext-Beschreibung eines Nutzers Feld-Werte VORSCHLÄGT. "
        "Du SETZT nie einen Wert und triffst keine rechtliche Entscheidung — der Mensch bestätigt jeden Vorschlag. "
        "Du darfst NUR diese Felder vorschlagen (keine anderen):\n" + felder + "\n\n"
        "Antworte AUSSCHLIESSLICH mit einem JSON-Array [{\"feld_id\":\"…\",\"wert\":…,\"begruendung\":\"kurz\"}], "
        "nur Felder für die die Beschreibung einen konkreten Wert hergibt, sonst []. Kein Fließtext.")
    return [{"role": "system", "content": system}, {"role": "user", "content": freitext}]


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


def _parse(text: str) -> list[dict]:
    """Roher LLM-Text → Liste {feld_id, wert, begruendung}. Toleriert ein Objekt-Wrapper ({\"vorschlaege\":[…]})
    oder ein nacktes Array. Nicht-Liste/kaputtes JSON → [] (kein Vorschlag ist besser als ein Müll-Vorschlag)."""
    try:
        j = json.loads(text)
    except Exception:
        return []
    if isinstance(j, dict):
        for k in ("vorschlaege", "vorschläge", "suggestions", "felder"):
            if isinstance(j.get(k), list):
                j = j[k]
                break
        else:
            j = [j] if "feld_id" in j else []
    if not isinstance(j, list):
        return []
    out = []
    for v in j:
        if isinstance(v, dict) and "feld_id" in v and "wert" in v:
            out.append({"feld_id": str(v["feld_id"]), "wert": v["wert"],
                        "begruendung": str(v.get("begruendung", ""))[:200]})
    return out


def vorschlaege(freitext: str, katalog: list[dict]) -> list[dict]:
    """Freitext + Feld-Katalog → LLM-Feld-Vorschläge [{feld_id, wert, begruendung}]. Cap-gated: kein Key/Base/
    Modell → LlmNichtVerfuegbar. Der Aufрufer (/chat-Handler) schreibt jeden Vorschlag als VORLÄUFIGES Event
    (Store-Auflage A + Katalog-Check erzwingen die Sicherheit); der Mensch bestätigt via Hold-Confirm."""
    if not (freitext or "").strip():
        return []
    return _parse(_call(_prompt(freitext, katalog)))
