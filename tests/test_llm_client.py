"""K1/K3 LLM-Client — reiner Client-Test (produkt/haut/llm_client.py) + Interface-Reconcile-Beweis mit dem
Kontoauszug-LLM-Fallback (produkt/import/kontoauszug_writer.llm_klassifikator_factory).

Der Client kennt keine Aufgabe, nur den rohen Call (`complete(role, msgs, fixture_id)`). Task-Wrapper
(Chat-Vorschläge) sitzen in api.py (siehe test_haut_chat.py), der Kontoauszug-Klassifikator-Wrapper in
kontoauszug_writer.py (dev-2-Zone, unverändert) — hier wird nur bewiesen, dass BEIDE korrekt über `complete`
zusammenlaufen. KEIN echter LLM-Call, KEIN Mock: plain Fixture-Funktionen ersetzen den Netz-Call.

Deckt:
  - test_complete_cap_gate_kein_key ....... kein Key/Base/Modell → LlmNichtVerfuegbar, kein Netz.
  - test_complete_wraps_call .............. complete() liefert Completion(text=...) über den bestehenden
                                             Cap-gated-HTTP-Call (_call monkeypatcht, kein echter Call).
  - test_kontoauszug_klassifikator_via_llm_client_modul ... KW.llm_klassifikator_factory nimmt das
                                             llm_client-MODUL als `client` (Interface-Reconcile-Beweis).
  - test_api_kontoauszug_llm_fallback_capgate_no_crash .... ohne Key: mehrdeutige Transaktion → stiller Skip,
                                             KEIN Crash (Regressions-Beweis für die neue Verdrahtung).
  - test_api_kontoauszug_llm_fallback_klassifiziert ....... mit Fixture-complete: mehrdeutige Transaktion wird
                                             über den LLM-Fallback klassifiziert und als Vorschlag geschrieben.
"""
from __future__ import annotations

import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API                    # noqa: E402
import llm_client as LC              # noqa: E402
import kontoauszug_writer as KW      # noqa: E402
import audit                # noqa: E402


@pytest.fixture
def fall(tmp_path, monkeypatch):
    """Ein leerer gesamt-Fall in einem tmp-FAELLE-Verzeichnis (kein Zugriff auf echte Fälle)."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    st, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "ka1"})
    assert st == 201
    return "ka1"


def _clean_env(monkeypatch):
    for k in ("LLM_API_KEY", "LLM_API_BASE", "LLM_MODEL"):
        monkeypatch.delenv(k, raising=False)


# --------------------------------------------------------------- Client: Cap-Gate + Completion-Wrapping
def test_complete_cap_gate_kein_key(monkeypatch):
    """Kein Key/Base/Modell in der Umgebung → complete() wirft LlmNichtVerfuegbar VOR jedem Netz-Zugriff
    (ECHTER llm_client, kein Monkeypatch von complete/_call selbst)."""
    _clean_env(monkeypatch)
    with pytest.raises(LC.LlmNichtVerfuegbar):
        LC.complete("chat", [{"role": "user", "content": "x"}])


def test_complete_wraps_call(monkeypatch):
    """complete() liefert eine Completion mit `.text` — der rohe HTTP-Call (_call) ist die einzige Stelle mit
    Netz-Zugriff; hier ersetzt (kein echter Call, kein Mock — plain Fixture-Funktion)."""
    monkeypatch.setattr(LC, "_call", lambda messages, schema=None: '{"kategorie": "spende"}')
    comp = LC.complete("kontoauszug_klassifikation", [{"role": "user", "content": "x"}], fixture_id="egal")
    assert isinstance(comp, LC.Completion)
    assert comp.text == '{"kategorie": "spende"}'


# --------------------------------------------------------------- Interface-Reconcile: llm_client-Modul als client
def test_kontoauszug_klassifikator_via_llm_client_modul(monkeypatch):
    """KW.llm_klassifikator_factory erwartet `client.complete(role, msgs, fixture_id=)` — das llm_client-MODUL
    selbst erfüllt das (Modul-Attribut-Zugriff), kein Klassen-Bau nötig. Beweist den Interface-Reconcile."""
    monkeypatch.setattr(LC, "complete",
                        lambda role, messages, fixture_id=None: LC.Completion(text='{"kategorie": "spende"}'))
    klass = KW.llm_klassifikator_factory(LC, "kontoauszug_klassifikation")
    assert klass("kryptischer Zweck", -5000) == "spende"


# --------------------------------------------------------------- api.kontoauszug()-Verdrahtung
def test_api_kontoauszug_llm_fallback_capgate_no_crash(fall, monkeypatch):
    """Ohne $LLM_API_KEY: eine mehrdeutige Transaktion geht durch den LLM-Fallback, der cap-gated ist —
    KEIN Crash (der Upload bleibt 200), die Transaktion wird still übersprungen (wie vorher llm_klassifikator=
    None). Regressions-Beweis für die neue Verdrahtung in api.kontoauszug()."""
    _clean_env(monkeypatch)
    csv = "datum;betrag;verwendungszweck\n15.03.2025;-1200,00;Zahlung Rechnung Nr 4711\n"
    st, body = API.kontoauszug(fall, {"format": "csv", "inhalt": csv})
    assert st == 200
    assert body["uebernommen"] == 0                 # mehrdeutig + kein Key -> kein Vorschlag, kein Crash


def test_api_kontoauszug_llm_fallback_klassifiziert(fall, monkeypatch):
    """Mit Fixture-complete: eine mehrdeutige Transaktion wird über den LLM-Fallback klassifiziert und als
    VORLÄUFIGER Vorschlag geschrieben (quelle=llm) — über den ECHTEN api.py-Verdrahtungspunkt."""
    monkeypatch.setattr(LC, "complete",
                        lambda role, messages, fixture_id=None: LC.Completion(text='{"kategorie": "spende"}'))
    csv = "datum;betrag;verwendungszweck\n15.03.2025;-1200,00;Zahlung Rechnung Nr 4711\n"
    st, body = API.kontoauszug(fall, {"format": "csv", "inhalt": csv})
    assert st == 200
    assert body["uebernommen"] == 1
    import store as ST
    aktiv = ST._aktives(API.lade_fall(fall))
    assert aktiv["spenden_betrag"]["wert"] == 120000
    assert aktiv["spenden_betrag"]["signal"]["signal_1"]["quelle"] == "llm"
