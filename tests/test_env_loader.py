"""server._lade_env_dateien: turnkey-Loader für gitignored .env.maps/.env.llm (K3-Live-Schaltung, dev-1).

Kein Netz, kein Port, kein echter Key. Prüft: (1) ein unsetzter Schlüssel wird geladen, (2) das echte
Prozess-Env GEWINNT (kein Override — Sicherheits-Invariant: eine gesetzte Umgebung sticht die Datei),
(3) Kommentare/Anführungszeichen/Leerzeilen sauber, (4) fehlende Datei = still no-op (kein Crash).
"""
from __future__ import annotations

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

import server as SRV   # noqa: E402


def test_laedt_unsetzten_schluessel(tmp_path, monkeypatch):
    (tmp_path / ".env.maps").write_text(
        '# Kommentar\nTG_ORS_TEST="xyz123"\nLEER=\n', encoding="utf-8")
    monkeypatch.delenv("TG_ORS_TEST", raising=False)   # registriert für Teardown-Restore (kein Leak)
    SRV._lade_env_dateien(str(tmp_path))
    assert os.environ.get("TG_ORS_TEST") == "xyz123"   # Anführungszeichen getrimmt


def test_prozess_env_gewinnt_kein_override(tmp_path, monkeypatch):
    """Sicherheits-Invariant: ein bereits gesetzter Schlüssel wird von der Datei NIE überschrieben."""
    (tmp_path / ".env.maps").write_text("TG_ORS_TEST=aus_datei\n", encoding="utf-8")
    monkeypatch.setenv("TG_ORS_TEST", "aus_prozess")
    SRV._lade_env_dateien(str(tmp_path))
    assert os.environ["TG_ORS_TEST"] == "aus_prozess"  # Prozess-Env sticht die Datei


def test_fehlende_datei_ist_noop(tmp_path):
    SRV._lade_env_dateien(str(tmp_path))               # keine .env.* vorhanden → kein Crash, kein Effekt


def test_llm_datei_auch_geladen(tmp_path, monkeypatch):
    (tmp_path / ".env.llm").write_text("TG_LLM_TEST=abc\n", encoding="utf-8")
    monkeypatch.delenv("TG_LLM_TEST", raising=False)
    SRV._lade_env_dateien(str(tmp_path))
    assert os.environ.get("TG_LLM_TEST") == "abc"


def test_env_datei_auch_geladen(tmp_path, monkeypatch):
    """Schlichte `.env` (z.B. $ELSTER_HERSTELLER_ID) — sonst liegt die Datei wirkungslos herum."""
    (tmp_path / ".env").write_text("TG_ENV_TEST=hid42\n", encoding="utf-8")
    monkeypatch.delenv("TG_ENV_TEST", raising=False)
    SRV._lade_env_dateien(str(tmp_path))
    assert os.environ.get("TG_ENV_TEST") == "hid42"


def test_env_datei_ueberschreibt_prozess_env_nicht(tmp_path, monkeypatch):
    """Dieselbe Sicherheits-Invariante wie fuer .env.maps — auch fuer die neue `.env`."""
    (tmp_path / ".env").write_text("TG_ENV_TEST=aus_datei\n", encoding="utf-8")
    monkeypatch.setenv("TG_ENV_TEST", "aus_prozess")
    SRV._lade_env_dateien(str(tmp_path))
    assert os.environ["TG_ENV_TEST"] == "aus_prozess"
