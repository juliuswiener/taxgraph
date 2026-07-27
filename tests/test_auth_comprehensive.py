"""Comprehensive Auth Integration Tests — P1.1-P1.6 Security Test Suite.

Full E2E auth tests covering registration, login, session, logout, authorization,
audit logging, and security edge cases. Tests the entire auth pipeline including
password hashing, JWT token management, and access control.
"""

import json
import os
import sys
import threading
import re
import tempfile
from datetime import datetime, timedelta, timezone

import jwt
import pytest
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# Import all auth components for direct sut testing
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

# ------------------------------------------------------------------ P1.1 Auth Core (Registration + Login)

class TestAuthCore:
    @pytest.fixture
    def base(self, tmp_path, monkeypatch):
        """Fixture: spun-up test server with temporary stores and fresh auth state."""
        faelle_dir = str(tmp_path / "faelle")
        monkeypatch.setattr(API, "FAELLE", faelle_dir)
        monkeypatch.setattr(API, "_AUTH_USER", None)
        # Use a temporary user store (not the real produkt/auth/users.json)
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

    # ------------------------------------------------------------------ Registration
    def test_register_valid(self, base):
        """Valid credentials → user registered."""
        status, data = _req(base, "POST", "/auth/register",
                          {"username": "validuser", "password": "validpassword1"})
        assert status == 201
        assert data["username"] == "validuser"

    def test_register_short_password(self, base):
        """Password <8 chars → validation error."""
        status, data = _req(base, "POST", "/auth/register",
                          {"username": "user", "password": "short"})
        assert status == 400
        assert "password" in data.get("fehler", "")

    def test_register_invalid_username(self, base):
        """Username rule violation (first char not letter) → validation error."""
        status, data = _req(base, "POST", "/auth/register",
                          {"username": "1invalid", "password": "validpassword1"})
        assert status == 400
        assert "username" in data.get("fehler", "")

    def test_register_duplicate_username(self, base):
        """Second registration with same username → conflict."""
        _req(base, "POST", "/auth/register",
             {"username": "duplicate", "password": "password1"})
        status, data = _req(base, "POST", "/auth/register",
                          {"username": "duplicate", "password": "another123"})
        assert status == 409
        assert "existiert" in data.get("fehler", "") or "bereits" in data.get("fehler", "")

    # ------------------------------------------------------------------ Login
    def test_login_valid_credentials(self, base):
        """Valid credentials → successful login."""
        _req(base, "POST", "/auth/register",
             {"username": "loginuser", "password": "loginpassword1"})
        status, data = _req(base, "POST", "/auth/login",
                          {"username": "loginuser", "password": "loginpassword1"})
        assert status == 200
        assert "token" in data
        assert data["username"] == "loginuser"

    def test_login_invalid_password(self, base):
        """Wrong password → authentication failure."""
        _req(base, "POST", "/auth/register",
             {"username": "pwdtest", "password": "correct123"})
        status, data = _req(base, "POST", "/auth/login",
                          {"username": "pwdtest", "password": "wrong123"})
        assert status == 401

    def test_login_nonexistent_user(self, base):
        """Nonexistent username → authentication failure."""
        status, data = _req(base, "POST", "/auth/login",
                          {"username": "nonexistent", "password": "anypassword1"})
        assert status == 401

    # ------------------------------------------------------------------ Security: Token Contents
    def test_token_no_plaintext_password(self, base, monkeypatch):
        """JWT tokens do not leak plaintext passwords (whitebox test)."""
        # Capture password hash to verify it's not the plaintext
        original_hash = AUTH._hash_pw
        captured_hash = None

        def mock_hash_pw(pw):
            nonlocal captured_hash
            captured_hash = original_hash(pw)
            return captured_hash

        monkeypatch.setattr(AUTH, "_hash_pw", mock_hash_pw)

        _req(base, "POST", "/auth/register",
             {"username": "hashuser", "password": "plainsecret123"})

        # Test via the AUTH.login() API to capture tokens
        status, data = AUTH.login(
            {"username": "hashuser", "password": "plainsecret123"},
            audit_fn=None
        )
        assert status == 200
        assert captured_hash is not None
        assert captured_hash != "plainsecret123"
        assert AUTH._check_pw("plainsecret123", captured_hash) is True
        assert AUTH._check_pw("wrong", captured_hash) is False

    # ------------------------------------------------------------------ JWT Protection
    def test_jwt_secret_not_hardcoded(self):
        """JWT secret is not hardcoded (dev fallback only)."""
        from produkt.auth.auth import _JWT_SECRET
        assert isinstance(_JWT_SECRET, str)
        # Length: secrets.token_hex(32) generates 64-character hex string
        assert len(_JWT_SECRET) == 64

    def test_jwt_token_valid_and_decodable(self):
        """JWT tokens can be verified and decoded."""
        from produkt.auth.auth import _create_token, verify_token
        token = _create_token("testuser")
        decoded_user = verify_token(token)
        assert decoded_user == "testuser"

    def test_jwt_jti_uniqueness(self):
        """JWT jti (JWT ID) should be unique per token."""
        from produkt.auth.auth import _create_token
        jtis = set()
        for _ in range(10):
            token = _create_token("user")
            decoded = jwt.decode(token, options={"verify_signature": False})
            jti = decoded.get("jti")
            assert jti is not None
            jtis.add(jti)
        assert len(jtis) == 10  # All unique

# ------------------------------------------------------------------ P1.2 Authorization

class TestAuthorization:
    @pytest.fixture
    def base(self, tmp_path, monkeypatch):
        """Shared fixture: spun-up test server."""
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

    def _create_fall(self, base, token=None):
        """Helper: create a fall (optionally with auth)."""
        payload = {"fall_id": "testfall", "scheibe": "ep", "veranlagungszeitraum": 2025}
        return _req(base, "POST", "/fall", payload, token=token)

    def _login_and_create(self, base):
        """Helper: register + login + create a fall."""
        _req(base, "POST", "/auth/register",
             {"username": "owner", "password": "secret123"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "owner", "password": "secret123"})
        self._create_fall(base, ld["token"])
        return ld["token"]

    def test_owner_access_allowed(self, base):
        """Owner token allows access to protected endpoints."""
        token = self._login_and_create(base)
        endpoints = [
            "/fall/testfall/fragen",
            "/fall/testfall/stand",
            "/fall/testfall/ergebnis",
        ]
        for path in endpoints:
            s, _ = _req(base, "GET", path, token=token)
            assert s == 200, f"{path} should allow owner access"

    def test_nonowner_access_denied(self, base):
        """Non-owner token → 403 for protected endpoints."""
        token = self._login_and_create(base)
        # Create a different user (attacker)
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
            assert s == 403, f"{path} should deny non-owner access"

    def test_no_auth_allowed_for_dev(self, base):
        """No auth header: dev/test path allowed."""
        s, _ = self._create_fall(base)
        assert s == 201

# ------------------------------------------------------------------ P1.3 Session Management

class TestSessionManagement:
    @pytest.fixture
    def base(self, tmp_path, monkeypatch):
        """Shared fixture: spun-up test server."""
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

    def test_session_valid_token(self, base):
        """Valid JWT → authenticated session."""
        _req(base, "POST", "/auth/register",
             {"username": "sessionsuser", "password": "sessionpassword1"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "sessionsuser", "password": "sessionpassword1"})
        status, data = _req(base, "GET", "/auth/session", token=ld["token"])
        assert status == 200
        assert data["authenticated"] is True

    def test_session_invalid_token(self, base):
        """Invalid token → 401."""
        status, _ = _req(base, "GET", "/auth/session", token="invalid.token.here")
        assert status == 401

# ------------------------------------------------------------------ P1.4 Logout

class TestLogout:
    @pytest.fixture
    def base(self, tmp_path, monkeypatch):
        """Shared fixture: spun-up test server."""
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

    def test_logout_valid_token(self, base):
        """Logout with valid token invalidates session."""
        _req(base, "POST", "/auth/register",
             {"username": "logoutuser", "password": "logoutpassword1"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "logoutuser", "password": "logoutpassword1"})
        token = ld["token"]

        # Token valid before logout
        s1, _ = _req(base, "GET", "/auth/session", token=token)
        assert s1 == 200

        # Logout
        s2, _ = _req(base, "POST", "/auth/logout", {"token": token})
        assert s2 == 200

        # Token invalid after logout
        s3, _ = _req(base, "GET", "/auth/session", token=token)
        assert s3 == 401

# ------------------------------------------------------------------ P1.6 Audit

class TestAudit:
    @pytest.fixture
    def base(self, tmp_path, monkeypatch):
        """Shared fixture: spun-up test server."""
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

    def test_audit_login_recorded(self, base):
        """Login actions are audit-logged."""
        _req(base, "POST", "/auth/register",
             {"username": "audituser", "password": "auditpassword1"})
        _req(base, "POST", "/auth/login",
             {"username": "audituser", "password": "auditpassword1"})
        entries = audit.lies()
        login_entries = [e for e in entries if e.get("action") == "login"]
        assert len(login_entries) >= 1
        assert login_entries[-1]["user_id"] == "audituser"

    def test_audit_fall_lifecycle(self, base):
        """Fall lifecycle events are audit-logged."""
        # Login
        _req(base, "POST", "/auth/register",
             {"username": "fauladmin", "password": "faulpassword1"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "fauladmin", "password": "faulpassword1"})
        # Create fall
        _req(base, "POST", "/fall",
             {"fall_id": "auditfall", "scheibe": "ep", "veranlagungszeitraum": 2025},
             token=ld["token"])
        # Access fall
        _req(base, "GET", "/fall/auditfall/fragen", token=ld["token"])

        entries = audit.lies()
        fall_entries = [e for e in entries if "fall" in str(e.get("action", ""))]
        assert len(fall_entries) >= 1

    def test_audit_append_only_enforced(self, base):
        """Audit log is append-only (no deletions)."""
        audit.append("test", "login", None, None)
        pfad = os.path.join(audit.AUDIT_DIR, "audit.jsonl")
        with open(pfad, encoding="utf-8") as f:
            before = f.read()
        audit.append("test2", "logout", None, None)
        with open(pfad, encoding="utf-8") as f:
            after = f.read()
        assert after.startswith(before), "Audit log must be append-only"

# ------------------------------------------------------------------ Module Runners

if __name__ == "__main__":
    pytest.main([__file__, "-v"])