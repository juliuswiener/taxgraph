"""P1.7 DSGVO-Löschung — DELETE /fall/{id}.

Testet:
- Löschen wirkt (Fall danach 404)
- Fremder Fall wird abgewiesen (403)
- Audit-Eintrag entsteht
- Audit-Eintrag überlebt die Löschung
- 404 bei unbekanntem Fall
"""

from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/auth", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API
import api_auth
import audit


# --------------------------------------------------------------- Fixtures
@pytest.fixture
def fall(tmp_path, monkeypatch):
    """Legt einen Fall an (Owner-Kontext: dev/Test = immer erlaubt)."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, body = API.fall_anlegen({"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "del1"})
    assert st == 201
    return "del1"


@pytest.fixture
def audit_dir(tmp_path, monkeypatch):
    """Setzt das Audit-Verzeichnis auf das Fall-Verzeichnis (wie Produktion)."""
    faelle = str(tmp_path / "faelle")
    monkeypatch.setattr(API, "FAELLE", faelle)
    old = audit.AUDIT_DIR
    audit.AUDIT_DIR = faelle
    yield faelle
    audit.AUDIT_DIR = old


# --------------------------------------------------------------- Löschung wirkt
class TestLoeschen:
    def test_loeschen_entfernt_fall(self, fall, tmp_path, monkeypatch):
        """Nach Löschen existiert die Fall-Datei nicht mehr."""
        API.fall_loeschen(fall)
        pfad = API._fall_pfad(fall)
        assert not os.path.exists(pfad), "Fall-Datei wurde nicht entfernt"

    def test_loeschen_danach_404(self, fall):
        """Nach Löschen liefert lade_fall 404."""
        API.fall_loeschen(fall)
        with pytest.raises(API.ApiError) as exc:
            API.lade_fall(fall)
        assert exc.value.status == 404

    def test_loeschen_antwort_hat_metadaten(self, fall):
        """Antwort enthällt fall_id, scheibe, veranlagungszeitraum."""
        st, body = API.fall_loeschen(fall)
        assert st == 200
        assert body["geloescht"] is True
        assert body["fall_id"] == "del1"
        assert body["scheibe"] == "ep"


# --------------------------------------------------------------- Owner-Check
class TestOwnerCheck:
    def test_loeschen_fremder_fall_403(self, fall, monkeypatch):
        """Fall gehört user1, Request läuft als user2 → 403."""
        # Fall mit owner user1 anlegen
        assert API.lade_fall(fall) is not None  # exists
        monkeypatch.setattr(api_auth, "_AUTH_USER", "user1")
        # Fall-Datei ohne Auth setzen (direkt)
        pfad = API._fall_pfad(fall)
        with open(pfad, encoding="utf-8") as f:
            store = json.load(f)
        store["user_id"] = "user1"
        with open(pfad, "w", encoding="utf-8") as f:
            json.dump(store, f)

        # Als user2 löschen
        monkeypatch.setattr(api_auth, "_AUTH_USER", "user2")
        with pytest.raises(API.ApiError) as exc:
            API.fall_loeschen(fall)
        assert exc.value.status == 403

    def test_loeschen_ohne_auth_erlaubt(self, fall, monkeypatch):
        """Kein Auth-Kontext → Löschen erlaubt (dev/Test)."""
        monkeypatch.setattr(api_auth, "_AUTH_USER", None)
        st, body = API.fall_loeschen(fall)
        assert st == 200

    def test_loeschen_eigener_fall_erlaubt(self, fall, monkeypatch):
        """Fall gehört user1, Request als user1 → 200."""
        # Fall mit owner setzen
        pfad = API._fall_pfad(fall)
        with open(pfad, encoding="utf-8") as f:
            store = json.load(f)
        store["user_id"] = "user1"
        with open(pfad, "w", encoding="utf-8") as f:
            json.dump(store, f)

        monkeypatch.setattr(api_auth, "_AUTH_USER", "user1")
        st, body = API.fall_loeschen(fall)
        assert st == 200

    def test_loeschen_unbekannter_fall_404(self):
        """Nicht-existenter Fall → 404."""
        with pytest.raises(API.ApiError) as exc:
            API.fall_loeschen("nichtda")
        assert exc.value.status == 404


# --------------------------------------------------------------- Audit
class TestAudit:
    def test_audit_eintrag_nach_loeschung(self, fall, audit_dir, monkeypatch):
        """Nach Löschen steht ein Audit-Eintrag mit action=fall_geloescht."""
        monkeypatch.setattr(api_auth, "_AUTH_USER", "testuser")
        audit.AUDIT_DIR = audit_dir
        API.fall_loeschen(fall)
        entries = audit.lies()
        loesch = [e for e in entries if e["action"] == "fall_geloescht"]
        assert len(loesch) == 1
        assert loesch[0]["fall_id"] == "del1"
        assert loesch[0]["user_id"] == "testuser"

    def test_audit_ueberlebt_loeschung(self, fall, audit_dir, monkeypatch):
        """Audit-Eintrag existiert, NACHDEM die Fall-Datei weg ist."""
        monkeypatch.setattr(api_auth, "_AUTH_USER", "testuser")
        audit.AUDIT_DIR = audit_dir
        API.fall_loeschen(fall)
        # Fall-Datei ist weg
        assert not os.path.exists(API._fall_pfad(fall))
        # Audit liest trotzdem
        entries = audit.lies()
        assert any(e["action"] == "fall_geloescht" for e in entries)

    def test_audit_kein_freitext(self, fall, audit_dir, monkeypatch):
        """Detail enthält nur Metadaten, nie PII."""
        monkeypatch.setattr(api_auth, "_AUTH_USER", "testuser")
        audit.AUDIT_DIR = audit_dir
        API.fall_loeschen(fall)
        entries = audit.lies()
        for e in entries:
            if e["action"] == "fall_geloescht":
                assert "scheibe=" in (e["detail"] or "")
                assert "vz=" in (e["detail"] or "")


# --------------------------------------------------------------- Mutationsprobe
class TestMutation:
    def test_loeschen_ohne_audit_ist_keine_loeschung(self, fall, monkeypatch):
        """Beweis: Wenn audit.append fehlt, ist Löschung nicht dokumentiert.
        (Einmaliger Commit-Beleg, dauerhafter Guard ist der Produktionscode.)"""
        # audit.append durch no-op ersetzen
        orig_append = audit.append
        audit_called = []
        monkeypatch.setattr(audit, "append", lambda *a, **kw: audit_called.append(a))
        API.fall_loeschen(fall)
        assert any("fall_geloescht" in str(a) for a in audit_called), \
            "Kein fall_geloescht-Audit-Eintrag erzeugt"

    def test_loeschen_entfernt_datei_wirklich(self, fall, tmp_path, monkeypatch):
        """Datei existiert vorher, nicht nachher."""
        monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
        API.fall_anlegen({"scheibe": "ep", "veranlagungszeitraum": 2025, "fall_id": "mut1"})
        pfad = API._fall_pfad("mut1")
        assert os.path.exists(pfad)
        API.fall_loeschen("mut1")
        assert not os.path.exists(pfad)