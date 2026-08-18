"""Dateien mit Geheimnissen gehören dem Eigentümer allein — geprüft, nicht gehofft.

DER FUND (Audit 2026-08-16, sec-users-json-world-readable), am 2026-08-18 nachgemessen:
produkt/auth/users.json lag mit 0644 auf der Platte. Darin stehen bcrypt-Hashes. Jeder Nutzer
des Rechners konnte sie mitlesen und offline gegen sie rechnen — bcrypt macht das teuer, nicht
unmöglich, und ein Hash, den niemand lesen kann, muss gar nicht erst standhalten. Dieselbe
Ursache im Prüfprotokoll (produkt/store/audit.py), das führt, wer wann welche Steuererklärung
bearbeitet hat.

WAS DIESEN FUND INTERESSANT MACHT, ist die Gegenseite: die FALL-Dateien — mit Steuer-ID,
Einkommen und IBAN — hatten längst 0600. Aber nicht, weil es jemand entschieden hätte, sondern
weil api.speichere_fall zufällig tempfile.NamedTemporaryFile benutzt, das so anlegt. Zwei
Schreibpfade, dieselbe Sorte Daten, gegensätzliche Ergebnisse, und in KEINEM stand, welcher
Modus gewollt ist. Der eine war aus Versehen richtig, der andere aus Versehen falsch.

Deshalb prüft diese Datei alle drei Pfade gemeinsam. Ein Test, der nur users.json abdeckt,
hielte den Zufall bei den Fall-Dateien für eine Zusage — und die nächste Umstellung von
tempfile auf ein gewöhnliches open() fiele niemandem auf.

NICHT PRÜFBAR und deshalb hier nur benannt: .env selbst (mode 0644 gemessen, enthält die
ELSTER-Zertifikats-PIN). Die Datei ist gitignored, existiert in CI nicht und wird von keinem
Code geschrieben — ein Test darüber wäre entweder ständig übersprungen oder ständig rot. Sie
ist von Hand auf 0600 gesetzt worden; der Code-Teil dieses Fundes
(sec-elster-pin-in-every-test-process) war bereits behoben, conftest.py:64 entfernt die PIN aus
der Umgebung der Testprozesse.

NULL LLM.
"""
from __future__ import annotations

import json
import os
import stat
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/auth", "produkt/traverser",
             "produkt/unsicherheit", "produkt/mapping", "produkt/konsistenz", "produkt/import",
             "produkt/bescheid", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API      # noqa: E402
import audit           # noqa: E402
import auth            # noqa: E402

NUR_EIGENTUEMER = 0o600


def _modus(pfad) -> int:
    return stat.S_IMODE(os.stat(pfad).st_mode)


def _lesbar_fuer_andere(pfad) -> bool:
    return bool(_modus(pfad) & (stat.S_IRGRP | stat.S_IROTH))


def test_users_json_nur_fuer_den_eigentuemer(tmp_path, monkeypatch):
    """Der eigentliche Fund. bcrypt-Hashes gehören niemandem sonst."""
    ziel = tmp_path / "users.json"
    monkeypatch.setattr(auth, "USER_STORE", str(ziel))
    auth._speichere_users({"users": {"a@b.c": {"pw": "$2b$12$nichtecht"}}})
    assert ziel.exists()
    assert not _lesbar_fuer_andere(ziel), (
        f"users.json hat Modus {oct(_modus(ziel))} — die Passwort-Hashes sind für andere "
        f"Nutzer des Rechners lesbar.")
    assert _modus(ziel) == NUR_EIGENTUEMER, f"erwartet 0600, ist {oct(_modus(ziel))}"


def test_users_json_bleibt_dicht_beim_zweiten_schreiben(tmp_path, monkeypatch):
    """Zweimal schreiben ist der Normalfall (jede Registrierung). Ein Fix, der nur beim ersten
    Mal greift, wäre in der Praxis wirkungslos — und ein Test, der nur einmal schreibt, sähe
    das nicht."""
    ziel = tmp_path / "users.json"
    monkeypatch.setattr(auth, "USER_STORE", str(ziel))
    auth._speichere_users({"users": {}})
    auth._speichere_users({"users": {"a@b.c": {"pw": "$2b$12$nichtecht"}}})
    assert _modus(ziel) == NUR_EIGENTUEMER, f"nach dem zweiten Schreiben {oct(_modus(ziel))}"


def test_users_json_ueberlebt_eine_liegengebliebene_tmp_datei(tmp_path, monkeypatch):
    """Bricht ein Lauf zwischen Anlegen und os.replace ab, bleibt eine .tmp liegen — mit
    unbekanntem Modus. Wird sie einfach weiterverwendet, erbt die Nutzerdatenbank ihn."""
    ziel = tmp_path / "users.json"
    monkeypatch.setattr(auth, "USER_STORE", str(ziel))
    rest = tmp_path / "users.json.tmp"
    rest.write_text("{}", encoding="utf-8")
    os.chmod(rest, 0o644)
    auth._speichere_users({"users": {"a@b.c": {"pw": "$2b$12$nichtecht"}}})
    assert _modus(ziel) == NUR_EIGENTUEMER, (
        f"Modus {oct(_modus(ziel))} von einer liegengebliebenen tmp-Datei geerbt")
    assert json.loads(ziel.read_text(encoding="utf-8"))["users"], "Inhalt nicht geschrieben"


def test_audit_protokoll_nur_fuer_den_eigentuemer(tmp_path, monkeypatch):
    """Das Protokoll führt Nutzer-Kennung, Fall-Kennung und Aktion — wer wann welche
    Steuererklärung bearbeitet hat."""
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path))
    audit.append("nutzer@example.test", "fall_create", "probe", "status=201")
    dateien = [p for p in tmp_path.rglob("*") if p.is_file()]
    assert dateien, "kein Protokoll geschrieben — der Test prüft nichts"
    for p in dateien:
        assert not _lesbar_fuer_andere(p), (
            f"{p.name} hat Modus {oct(_modus(p))} — das Prüfprotokoll ist für andere lesbar.")


def test_fall_datei_nur_fuer_den_eigentuemer(tmp_path, monkeypatch):
    """Die Gegenseite des Fundes: hier war es schon richtig, aber aus Versehen (tempfile legt
    mit 0600 an). Ohne diesen Test bliebe der Zufall ungeprüft, und die nächste Umstellung auf
    ein gewöhnliches open() machte Steuer-ID, Einkommen und IBAN für alle lesbar."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    API.speichere_fall("rechte_probe", {"veranlagungszeitraum": 2025, "fall_id": "rechte_probe"})
    ziel = tmp_path / "faelle" / "rechte_probe.json"
    assert ziel.exists()
    assert _modus(ziel) == NUR_EIGENTUEMER, (
        f"Fall-Datei hat Modus {oct(_modus(ziel))} — darin stehen Steuer-ID, Einkommen und IBAN.")


def test_die_pruefung_erkennt_ihren_eigenen_fehlerfall(tmp_path):
    """Negativprobe: ohne sie wäre nicht belegt, dass _lesbar_fuer_andere überhaupt anschlägt —
    ein Modus-Test, der immer grün ist, sieht von aussen aus wie ein bestandener."""
    offen = tmp_path / "offen"
    offen.write_text("x", encoding="utf-8")
    os.chmod(offen, 0o644)
    assert _lesbar_fuer_andere(offen)
    os.chmod(offen, 0o600)
    assert not _lesbar_fuer_andere(offen)


@pytest.mark.parametrize("umaske", [0o000, 0o022])
def test_modus_haengt_nicht_an_der_umask(tmp_path, monkeypatch, umaske):
    """Der Kern des Fundes war eine geerbte umask. Mit umask 000 legt ein gewöhnliches open()
    mit 0666 an — genau der Fall, den die alte Fassung nicht überlebte. Beide Werte geprüft,
    damit der Test nicht zufällig von der Umgebung des Entwicklers profitiert."""
    ziel = tmp_path / "users.json"
    monkeypatch.setattr(auth, "USER_STORE", str(ziel))
    alt = os.umask(umaske)
    try:
        auth._speichere_users({"users": {"a@b.c": {"pw": "$2b$12$nichtecht"}}})
    finally:
        os.umask(alt)
    assert _modus(ziel) == NUR_EIGENTUEMER, (
        f"bei umask {oct(umaske)} entstand Modus {oct(_modus(ziel))} — der Modus wird nicht "
        f"explizit gesetzt, sondern von der Umgebung geerbt.")
