"""Test: client.py — mask_key, is_dry_run, _json_or_none, RoleCallError, transport_summary.

mask_key zuerst und gruendlich: es ist die Stelle, an der ein API-Schluessel NICHT
in ein Log geraten darf. Die Behauptung "die Maskierung greift an 12 Stellen" sollte
belegt sein, nicht nur plausibel klingen.

Interessant sind die Faelle, in denen Maskierung STILL VERSAGT: Schluessel in einer
verschachtelten Struktur, in einer Exception-Message, mehrere Schluessel in einem
String, Schluessel als Teilstring eines laengeren Tokens, leerer/None-Key,
Key kuerzer als die Maskierungslaenge, Unicode-Zeichen im Key.

GRENZE: mask_key ist OpenRouter-spezifisch (nur `sk-or-...`). sk-ant-/sk-proj-Keys
laufen ungefiltert durch — dokumentiert als Charakterisierungstests, kein Fix.
Durch pipeline/client.py laufen nur OpenRouter-Keys; die anderen Clients
(ors_client, llm_client) nutzen mask_key nicht und muessen es nicht (sie geben nur
den Exception-Typnamen weiter, nie Key/Body/Header).

NICHT getestet (bewusst):
  - complete(): braucht HTTP-Mock oder Live-OpenRouter; ohne Eingriff in Produktivcode
    nicht testbar. Wird indirekt durch test_judge_mehrheit.py (FakeClient) abgedeckt.
  - __init__() / _require_key(): brauchen env OPENROUTER_API_KEY; dry-run init
    (self._key = None) ist implizit durch alle dry_run-Tests getestet.
"""
import sys
import os
import json

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "pipeline"))

import client as CL  # noqa: E402


# =========================================================================
# mask_key — Security-Funktion, gruendlich
# =========================================================================

def test_mask_key_normal():
    """Normaler Key -> vollstaendig maskiert."""
    assert CL.mask_key("sk-or-abcdef0123456789") == "sk-or-***"


def test_mask_key_in_dict_string():
    """Key in str(dict) -> maskiert (der typische Leak-Pfad)."""
    d = {"api": "sk-or-abcdef0123456789"}
    result = CL.mask_key(str(d))
    assert "sk-or-***" in result
    assert "sk-or-abcdef0123456789" not in result


def test_mask_key_in_exception():
    """Key in Exception-Message -> maskiert durch RoleCallError."""
    e = CL.RoleCallError("worker", "some/model", ["p1"],
                         "sk-or-abcdef0123456789", kind="role_error")
    msg = str(e)
    assert "sk-or-***" in msg
    assert "sk-or-abcdef0123456789" not in msg


def test_mask_key_mehrere():
    """Mehrere Keys in einem String -> alle maskiert."""
    s = "key1=sk-or-abc key2=sk-or-def"
    assert CL.mask_key(s) == "key1=sk-or-*** key2=sk-or-***"


def test_mask_key_als_substring():
    """Key als Teilstring -> maskiert.

    ACHTUNG (gierige Regex): `[A-Za-z0-9._-]+` frisst auch folgendes `_`/`-`/`.`
    mit, weil sie in der Klasse sind. Bei 'prefix_sk-or-abcdef123_suffix' wird also
    '_suffix' mitmaskiert -> 'prefix_sk-or-***'. Das ist UEBER-Maskierung (kein
    Leck, Maskieren ist sicher), aber die Umgebung bleibt NICHT erhalten.
    """
    s = "prefix_sk-or-abcdef123_suffix"
    result = CL.mask_key(s)
    assert "sk-or-***" in result
    assert "sk-or-abcdef123" not in result
    assert result.startswith("prefix_")


def test_mask_key_leer():
    """Leerer String -> leer."""
    assert CL.mask_key("") == ""


def test_mask_key_none():
    """None -> str(None) = 'None', kein Key, kein Crash."""
    assert CL.mask_key(None) == "None"


def test_mask_key_kuerzer_als_ersatz():
    """Key kuerzer als 'sk-or-***' -> maskiert (das Ersatz ist laenger, aber das ist OK)."""
    assert CL.mask_key("sk-or-a") == "sk-or-***"


def test_mask_key_mit_punkt():
    """Key mit Punkt (OpenRouter-Key erlaubt Punkt) -> maskiert."""
    assert CL.mask_key("sk-or-abc.def") == "sk-or-***"


def test_mask_key_mit_bindestrich():
    """Key mit Bindestrich -> maskiert."""
    assert CL.mask_key("sk-or-abc-def") == "sk-or-***"


def test_mask_key_mit_unterstrich():
    """Key mit Unterstrich -> maskiert."""
    assert CL.mask_key("sk-or-abc_def") == "sk-or-***"


def test_mask_key_schon_maskiert():
    """Bereits maskierter 'sk-or-***' -> kein zweites Maskieren (bleibt gleich)."""
    # 'sk-or-***' matcht nicht: '***' ist nicht in [A-Za-z0-9._-]
    assert CL.mask_key("sk-or-***") == "sk-or-***"


def test_mask_key_ohne_suffix():
    """'sk-or-' ohne folgende Zeichen -> kein Match (regex + braucht min 1 Zeichen)."""
    assert CL.mask_key("sk-or-") == "sk-or-"


def test_mask_key_nur_prefix():
    """'sk-or-' allein (kein Key) -> unveraendert."""
    assert CL.mask_key("sk-or-") == "sk-or-"


def test_mask_key_unicode_im_key():
    """CHARAKTERISIERUNG: Unicode im Key maskiert nur den ASCII-Praefix.

    Restrisiko, bewusst NICHT gefixt: `[A-Za-z0-9._-]` matcht kein Nicht-ASCII,
    also maskiert der ASCII-Praefix, der Unicode-Suffix bleibt sichtbar.
    Warum nicht fixt: Ersatz durch `\\S` wuerde das Overmasking verschaerfen
    (Leerzeichen-getrennte Tokens fressen). OpenRouter-Keys sind hex-artig und
    enthalten nie Unicode — rein theoretisch, kein reales Leck.
    """
    result = CL.mask_key("sk-or-abc\xe9def")
    assert "sk-or-***" in result  # abc ist maskiert
    assert "\xe9def" in result  # Unicode-Suffix bleibt sichtbar


# -----------------------------------------------------------------------------
# GRENZE: mask_key ist OpenRouter-spezifisch, KEINE allgemeine Key-Maskierung.
# Andere Provider-Keys (sk-ant, sk-proj) laufen ungefiltert durch. Das ist der
# Ist-Zustand, kein Fix: durch pipeline/client.py laufen nur OpenRouter-Keys
# (sk-or-...); die anderen Clients (ors_client, llm_client) nutzen mask_key nicht
# und muessen es auch nicht (sie geben nur den Exception-Typnamen weiter, nie Key/
# Body/Header). Falls spaeter ein zweiter Provider in pipeline/client.py einbaut
# wird, bricht dieser Test sichtbar statt still.
# -----------------------------------------------------------------------------

def test_mask_key_ignoriert_sk_ant():
    """sk-ant-... (Anthropic) wird NICHT maskiert — OpenRouter-spezifisch."""
    assert CL.mask_key("key=sk-ant-api03-abc123") == "key=sk-ant-api03-abc123"


def test_mask_key_ignoriert_sk_proj():
    """sk-proj-... (OpenAI) wird NICHT maskiert — OpenRouter-spezifisch."""
    assert CL.mask_key("key=sk-proj-abc123") == "key=sk-proj-abc123"


def test_mask_key_ignoriert_sk_ant_bearer():
    """Authorization-Header mit sk-ant-Key bleibt unveraendert."""
    s = "Authorization: Bearer sk-ant-api03-xyz"
    assert CL.mask_key(s) == s


def test_mask_key_nicht_key():
    """sk-or-aehnlich aber kein Schluessel -> unveraendert."""
    assert CL.mask_key("sk-orchestra") == "sk-orchestra"


# =========================================================================
# is_dry_run — bool + env
# =========================================================================

def test_is_dry_run_true():
    assert CL.is_dry_run(True) is True


def test_is_dry_run_false():
    assert CL.is_dry_run(False) is False


def test_is_dry_run_none_mit_env(monkeypatch):
    monkeypatch.setenv("PIPELINE_DRY_RUN", "1")
    assert CL.is_dry_run(None) is True


def test_is_dry_run_none_ohne_env(monkeypatch):
    monkeypatch.delenv("PIPELINE_DRY_RUN", raising=False)
    assert CL.is_dry_run(None) is False


def test_is_dry_run_false_schlaegt_env(monkeypatch):
    """False ueberschreibt env (dry_run-Parameter hat Vorrang)."""
    monkeypatch.setenv("PIPELINE_DRY_RUN", "1")
    assert CL.is_dry_run(False) is False


# =========================================================================
# _json_or_none — Gateway-HTML, leere Antworten
# =========================================================================

def test_json_or_none_valid_dict():
    class R:
        @staticmethod
        def json():
            return {"ok": True}
    assert CL._json_or_none(R) == {"ok": True}


def test_json_or_none_valid_list():
    """Liste ist JSON, aber kein dict -> None (fail-closed)."""
    class R:
        @staticmethod
        def json():
            return [1, 2]
    assert CL._json_or_none(R) is None


def test_json_or_none_invalid():
    class R:
        @staticmethod
        def json():
            raise ValueError("no json")
    assert CL._json_or_none(R) is None


def test_json_or_none_empty():
    class R:
        @staticmethod
        def json():
            return {}
    assert CL._json_or_none(R) == {}


# =========================================================================
# RoleCallError — Key-Maskierung im Error
# =========================================================================

def test_role_call_error_maskt_key():
    e = CL.RoleCallError("worker", "m/s", ["p1"],
                         "got sk-or-abc123", kind="role_error")
    assert "sk-or-***" in str(e)
    assert "sk-or-abc123" not in str(e)


def test_role_call_error_attributes():
    e = CL.RoleCallError("worker", "m/s", ["p1", "p2"],
                         "boom", kind="role_timeout")
    assert e.role == "worker"
    assert e.slug == "m/s"
    assert e.providers == ["p1", "p2"]
    assert e.kind == "role_timeout"


def test_role_call_error_string_representation():
    """str() enthaelt role/slug und masked reason."""
    e = CL.RoleCallError("worker", "m/s", ["p1"], "boom")
    s = str(e)
    assert "worker" in s
    assert "m/s" in s
    assert "boom" in s


# =========================================================================
# transport_summary — Aggregation
# =========================================================================

def test_transport_summary_empty():
    c = CL.OpenRouterClient(dry_run=True)
    s = c.transport_summary()
    assert s["retries"] == 0
    assert s["timeouts"] == 0
    assert s["errors"] == 0
    assert s["rate_limits"] == 0


def test_transport_summary_zaehlt():
    c = CL.OpenRouterClient(dry_run=True)
    c._event("r", "s", "p1", "retries", "after timeout")
    c._event("r", "s", "p1", "retries", "again")
    c._event("r", "s", "p1", "errors", "timeout")
    c._event("r", "s", "p1", "rate_limits", "429")
    c._event("r", "s", "p2", "timeouts", "slow")
    s = c.transport_summary()
    assert s["retries"] == 2
    assert s["errors"] == 1
    assert s["rate_limits"] == 1
    assert s["timeouts"] == 1


def test_transport_summary_by_provider():
    c = CL.OpenRouterClient(dry_run=True)
    c._event("r", "s", "p1", "retries", "x")
    c._event("r", "s", "p1", "errors", "y")
    c._event("r", "s", "p2", "retries", "z")
    s = c.transport_summary()
    assert s["by_provider"]["p1"]["retries"] == 1
    assert s["by_provider"]["p1"]["errors"] == 1
    assert s["by_provider"]["p2"]["retries"] == 1
    assert "p1" not in s["by_provider"].get("p2", {})


def test_transport_summary_unbekannter_kind_ignoriert():
    """Unbekannter event kind -> nicht in Summary (nur bekannte werden gezaehlt)."""
    c = CL.OpenRouterClient(dry_run=True)
    c._event("r", "s", "p1", "unknown_kind", "x")
    s = c.transport_summary()
    assert s["retries"] == 0
    assert s["errors"] == 0
    assert s["rate_limits"] == 0
    assert s["timeouts"] == 0


def test_transport_summary_maskiert_detail():
    """_event ruft mask_key auf detail auf -> Key im Detail maskiert."""
    c = CL.OpenRouterClient(dry_run=True)
    c._event("r", "s", "p1", "errors", "key=sk-or-abc123")
    assert len(c.transport) == 1
    assert "sk-or-***" in c.transport[0]["detail"]
    assert "sk-or-abc123" not in c.transport[0]["detail"]


# =========================================================================
# _fixture_completion — Dry-Run-Pfad
# =========================================================================

def test_fixture_completion_missing(tmp_path):
    """Fehlende Fixture -> FileNotFoundError (fail-closed, kein stilles None)."""
    c = CL.OpenRouterClient(dry_run=True, fixtures_dir=str(tmp_path))
    role = CL.RoleConfig(role="nixda", slug="m/s", providers=["p1"])
    try:
        c._fixture_completion(role, [])
        assert False, "fehlende Fixture muss failen"
    except FileNotFoundError:
        pass


def test_fixture_completion_ok(tmp_path):
    fx = {"text": "ok", "prompt_tokens": 10, "completion_tokens": 5}
    p = tmp_path / "worker.json"
    p.write_text(json.dumps(fx))
    c = CL.OpenRouterClient(dry_run=True, fixtures_dir=str(tmp_path))
    role = CL.RoleConfig(role="worker", slug="m/s", providers=["p1"])
    comp = c._fixture_completion(role, [])
    assert comp.text == "ok"
    assert comp.role == "worker"
    assert comp.slug == "m/s (dry-run)"
    assert comp.provider == "dry-run"
    assert comp.prompt_tokens == 10
    assert comp.completion_tokens == 5
    assert comp.cost_usd == 0.0
    assert comp.raw == {"dry_run": True}


def test_fixture_completion_mit_fixture_id(tmp_path):
    """fixture_id ueberschreibt role.role als Dateiname."""
    fx = {"text": "override"}
    p = tmp_path / "custom.json"
    p.write_text(json.dumps(fx))
    c = CL.OpenRouterClient(dry_run=True, fixtures_dir=str(tmp_path))
    role = CL.RoleConfig(role="worker", slug="m/s", providers=["p1"])
    comp = c._fixture_completion(role, [], fixture_id="custom")
    assert comp.text == "override"


# =========================================================================
# list_models — trockener Pfad (dry_run=True)
# =========================================================================

def test_list_models_dry_run():
    c = CL.OpenRouterClient(dry_run=True)
    assert c.list_models() == []