"""Ein fehlgeschlagener LLM-Aufruf muss sagen, WARUM — ohne den Key zu verraten.

Anlass 2026-08-14: nach dem Anlegen von .env.llm antwortete /chat weiter mit 501. Der Client
meldete intern nur `LLM-Aufruf fehlgeschlagen: HTTPError` — also exakt dasselbe für einen
falschen Key, ein nicht existierendes Modell, eine vertippte Base-URL und den tatsächlichen
Grund: HTTP 403 "Budget limit exceeded (monthly limit)". Die Ursache war nur zu finden, indem
man den Client umging und den Request von Hand stellte.

Seither reicht _call bei HTTPError den Statuscode und die (gekürzte) Provider-Meldung durch.

Die Meldung erreicht den Nutzer NICHT — der /chat-Handler antwortet mit der festen
CHAT_501-Konstante; sie ist fürs Server-Log und die Diagnose. Trotzdem prüft der zweite Test,
dass der Schlüssel nicht darin landet: er steht im Request-Header, und eine Provider-Antwort,
die ihn zurückspiegelt, wäre kein theoretischer Fall.

NULL LLM — kein echter Call, urlopen wird ersetzt.
"""
from __future__ import annotations

import io
import os
import sys
import urllib.error

import pytest

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

import llm_client as LC  # noqa: E402

KEY = "sk-testkey-UNVERWECHSELBAR-4711"


@pytest.fixture
def konfiguriert(monkeypatch):
    monkeypatch.setenv("LLM_API_KEY", KEY)
    monkeypatch.setenv("LLM_API_BASE", "https://example.test/v1")
    monkeypatch.setenv("LLM_MODEL", "test/modell")
    # Seit der Client vorübergehende Störungen wiederholt (2026-08-18, Audit
    # res-product-clients-no-retry), wartet er zwischen den Versuchen wirklich. Für die
    # Diagnose-Prüfungen hier ist das reine Wartezeit: sie prüfen den TEXT der Meldung, nicht
    # die Wiederholung selbst (die hat ihre eigene Datei, test_llm_deckel_und_wiederholung.py).
    monkeypatch.setattr(LC.time, "sleep", lambda s: None)


def _antworte_mit(status: int, body: bytes):
    def fake(req, timeout=None):
        raise urllib.error.HTTPError(req.full_url, status, "Fehler", {}, io.BytesIO(body))
    return fake


def test_http_status_und_ursache_kommen_durch(konfiguriert, monkeypatch):
    monkeypatch.setattr(
        LC.urllib.request, "urlopen",
        _antworte_mit(403, b'{"error":{"message":"Budget limit exceeded (monthly limit)"}}'))
    with pytest.raises(LC.LlmNichtVerfuegbar) as e:
        LC._call([{"role": "user", "content": "hi"}])
    meldung = str(e.value)
    assert "403" in meldung, f"Statuscode fehlt: {meldung}"
    assert "Budget limit exceeded" in meldung, f"Provider-Ursache fehlt: {meldung}"


def test_der_schluessel_steht_nie_in_der_meldung(konfiguriert, monkeypatch):
    """Auch dann nicht, wenn der Provider ihn in der Fehlerantwort zurückspiegelt."""
    monkeypatch.setattr(
        LC.urllib.request, "urlopen",
        _antworte_mit(401, f'{{"error":"invalid key {KEY}"}}'.encode()))
    with pytest.raises(LC.LlmNichtVerfuegbar) as e:
        LC._call([{"role": "user", "content": "hi"}])
    assert KEY not in str(e.value), "Der API-Key steht in der Fehlermeldung."


def test_nicht_http_fehler_bleiben_knapp(konfiguriert, monkeypatch):
    """Netz-/Parse-Fehler haben keine Provider-Meldung — dort bleibt es beim Ausnahmetyp,
    statt eine leere Klammer anzuhängen."""
    def kaputt(req, timeout=None):
        raise TimeoutError("zu langsam")
    monkeypatch.setattr(LC.urllib.request, "urlopen", kaputt)
    with pytest.raises(LC.LlmNichtVerfuegbar) as e:
        LC._call([{"role": "user", "content": "hi"}])
    assert "TimeoutError" in str(e.value)


def test_ohne_konfiguration_kein_netzzugriff(monkeypatch):
    """Cap-Gate: ohne Key/Base/Modell darf gar kein Request entstehen ($0-Garantie)."""
    monkeypatch.delenv("LLM_API_KEY", raising=False)
    monkeypatch.delenv("LLM_API_BASE", raising=False)
    monkeypatch.delenv("LLM_MODEL", raising=False)

    def darf_nicht(req, timeout=None):
        raise AssertionError("Es wurde ein HTTP-Request gebaut, obwohl nichts konfiguriert ist.")
    monkeypatch.setattr(LC.urllib.request, "urlopen", darf_nicht)
    with pytest.raises(LC.LlmNichtVerfuegbar):
        LC._call([{"role": "user", "content": "hi"}])
