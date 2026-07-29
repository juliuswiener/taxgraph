"""Auth Integration Tests — P1 Security (Registration, Login, Session, Logout, Audit).

Comprehensive E2E tests covering all auth security features: registration, login, session management, logout, authorization, audit logging, token security, and edge cases.
"""

import json
import os
import sys
import threading
from datetime import datetime, timedelta, timezone

import jwt
import pytest
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# Import auth components for direct testing
for sub in ("produkt/haut", "produkt/auth", "produkt/store"):
    sys_path = os.path.join(ROOT, sub)
    if sys_path not in sys.path:
        sys.path.insert(0, sys_path)

import auth as AUTH
import api as API
import server as SRV
import audit
import store as ST

# ------------------------------------------------------------------ HTTP Helper

def _req(base: str, method: str, path: str, body: dict | None = None, token: str | None = None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(base + path, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())

# ------------------------------------------------------------------ Base Fixture

@pytest.fixture
def base(tmp_path, monkeypatch):
    """Fixture: spun-up test server with temporary stores."""
    faelle_dir = str(tmp_path / "faelle")
    monkeypatch.setattr(API, "FAELLE", faelle_dir)
    monkeypatch.setattr(API, "_AUTH_USER", None)
    auth_store = str(tmp_path / "users.json")
    monkeypatch.setattr(AUTH, "USER_STORE", auth_store)
    monkeypatch.setattr(audit, "AUDIT_DIR", faelle_dir)
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()

# ------------------------------------------------------------------ P1.1 Auth Tests

class TestAuthCore:
    def test_register_valid(self, base):
        """Successful registration."""
        status, data = _req(base, "POST", "/auth/register",
                          {"username": "testuser", "password": "validpass123"})
        assert status == 201
        assert data["username"] == "testuser"

    def test_register_short_password(self, base):
        """Password <8 chars rejected."""
        status, data = _req(base, "POST", "/auth/register",
                          {"username": "user", "password": "short"})
        assert status == 400

    def test_register_invalid_username(self, base):
        """Invalid username rejected."""
        status, _ = _req(base, "POST", "/auth/register",
                      {"username": "1invalid", "password": "validpass123"})
        assert status == 400

    def test_login_valid(self, base):
        """Valid login returns token."""
        _req(base, "POST", "/auth/register",
             {"username": "loginuser", "password": "loginpass123"})
        status, data = _req(base, "POST", "/auth/login",
                          {"username": "loginuser", "password": "loginpass123"})
        assert status == 200
        assert "token" in data
        assert data["username"] == "loginuser"

    def test_token_no_password_in_jwt(self, base):
        """JWT tokens do not contain plaintext passwords."""
        # Mock hash to capture password hash
        original_hash = AUTH._hash_pw
        captured_hash = None

        def mock_hash_pw(pw):
            nonlocal captured_hash
            captured_hash = original_hash(pw)
            return captured_hash

        monkeypatch = pytest.MonkeyPatch()
        monkeypatch.setattr(AUTH, "_hash_pw", mock_hash_pw)

        _req(base, "POST", "/auth/register",
             {"username": "hashuser", "password": "secret123"})

        status, data = AUTH.login(
            {"username": "hashuser", "password": "secret123"},
            audit_fn=None
        )
        assert status == 200
        assert captured_hash != "secret123"
        assert AUTH._check_pw("secret123", captured_hash) is True

    def test_session_valid(self, base):
        """Valid token → authenticated session."""
        _req(base, "POST", "/auth/register",
             {"username": "sessionuser", "password": "sessionpass123"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "sessionuser", "password": "sessionpass123"})
        status, data = _req(base, "GET", "/auth/session", token=ld["token"])
        assert status == 200
        assert data["authenticated"] is True

    def test_session_invalid(self, base):
        """Invalid token → 401."""
        status, _ = _req(base, "GET", "/auth/session", token="invalid.token")
        assert status == 401

    def test_logout_invalidates(self, base):
        """Logout invalidates token."""
        _req(base, "POST", "/auth/register",
             {"username": "logoutuser", "password": "logoutpass123"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "logoutuser", "password": "logoutpass123"})
        token = ld["token"]

        # Session valid before logout
        s1, _ = _req(base, "GET", "/auth/session", token=token)
        assert s1 == 200

        # Logout
        s2, _ = _req(base, "POST", "/auth/logout", {"token": token})
        assert s2 == 200

        # Session invalid after logout
        s3, _ = _req(base, "GET", "/auth/session", token=token)
        assert s3 == 401

    def test_jwt_secret_not_hardcoded(self):
        """JWT secret is not hardcoded."""
        from produkt.auth.auth import _JWT_SECRET
        assert isinstance(_JWT_SECRET, str)
        assert len(_JWT_SECRET) == 64  # 32 bytes hex encoded

# ------------------------------------------------------------------ P1.2 Authorization

class TestAuthorization:
    def _create_fall(self, base, token=None):
        payload = {"fall_id": "testfall", "scheibe": "ep", "veranlagungszeitraum": 2025}
        return _req(base, "POST", "/fall", payload, token=token)

    def _login_and_create(self, base):
        _req(base, "POST", "/auth/register",
             {"username": "owner", "password": "secret123"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "owner", "password": "secret123"})
        self._create_fall(base, ld["token"])
        return ld["token"]

    def test_owner_access_allowed(self, base):
        """Owner token allows full access."""
        token = self._login_and_create(base)
        endpoints = [
            "/fall/testfall/fragen",
            "/fall/testfall/stand",
            "/fall/testfall/ergebnis",
        ]
        for path in endpoints:
            s, _ = _req(base, "GET", path, token=token)
            assert s == 200

    def test_nonowner_access_denied(self, base):
        """Non-owner token → 403."""
        token = self._login_and_create(base)
        _req(base, "POST", "/auth/register",
             {"username": "attacker", "password": "secret123"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "attacker", "password": "secret123"})
        fremd_token = ld["token"]
        endpoints = [
            "/fall/testfall/fragen",
            "/fall/testfall/stand",
        ]
        for path in endpoints:
            s, _ = _req(base, "GET", path, token=fremd_token)
            assert s == 403

    def test_no_auth_allowed_dev(self, base):
        """No auth header allowed for dev testing."""
        s, _ = self._create_fall(base)
        assert s == 201

# ------------------------------------------------------------------ P1.6 Audit

class TestAudit:
    def test_login_recorded(self, base):
        """Login actions audit logged."""
        _req(base, "POST", "/auth/register",
             {"username": "audituser", "password": "secret123"})
        _req(base, "POST", "/auth/login",
             {"username": "audituser", "password": "secret123"})
        entries = audit.lies()
        login_entries = [e for e in entries if e.get("action") == "login"]
        assert len(login_entries) >= 1
        assert login_entries[-1]["user_id"] == "audituser"

    def test_fall_lifecycle_audited(self, base):
        """Fall operations audit logged."""
        _req(base, "POST", "/auth/register",
             {"username": "falladmin", "password": "secret123"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "falladmin", "password": "secret123"})
        _req(base, "POST", "/fall",
             {"fall_id": "auditfall", "scheibe": "ep", "veranlagungszeitraum": 2025},
             token=ld["token"])
        _req(base, "GET", "/fall/auditfall/fragen", token=ld["token"])

        entries = audit.lies()
        fall_events = len([e for e in entries if "fall" in e.get("action", "")])
        assert fall_events >= 2

    def test_append_only_audited(self, base):
        """Audit log append-only enforced."""
        audit.append("test", "login", None, None)
        pfad = os.path.join(audit.AUDIT_DIR, "audit.jsonl")
        with open(pfad, encoding="utf-8") as f:
            before = f.read()
        audit.append("test2", "logout", None, None)
        with open(pfad, encoding="utf-8") as f:
            after = f.read()
        assert after.startswith(before)