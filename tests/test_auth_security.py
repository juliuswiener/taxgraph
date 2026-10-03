"""Auth Security Tests — P1 Security Module Testing.

P1 Security Module tests covering JWT-based auth, authorization, audit logging,
password hashing, session management, and all security-critical auth functionality.
"""

import json
import os
import sys
import re
import threading
from datetime import datetime, timedelta, timezone

import jwt
import pytest
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)

# Import auth components for testing
for sub in ("produkt/haut", "produkt/auth", "produkt/store"):
    sys_path = os.path.join(ROOT, sub)
    if sys_path not in sys.path:
        sys.path.insert(0, sys_path)

import auth as AUTH
import api as API
import api_auth
import server as SRV
import audit
import store as ST

# ------------------------------------------------------------------ HTTP Helper

def _req(base: str, method: str, path: str, body: dict | None = None,
         token: str | None = None, erwarte: int | None = None):
    """HTTP-Request mit optionalem Status-Check.

    Prüft selbst:
    - 5xx → AssertionError (nie unterdrückbar)
    - 4xx → AssertionError, es sei denn `erwarte=<code>` ist gesetzt
    - 2xx → durch
    - erwarte=N → assert status == N
    """
    data = json.dumps(body).encode("utf-8") if body is not None else None
    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"
    req = urllib.request.Request(base + path, data=data, method=method, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, (
            f"erwarte={erwarte}, erhalten={status} {method} {path} {body}")
    elif status >= 500:
        raise AssertionError(
            f"Serverfehler {status} {method} {path} {body}: {content}")
    elif status >= 400:
        raise AssertionError(
            f"Fehler {status} {method} {path} {body}: {content}")
    return status, content

# ------------------------------------------------------------------ Base Fixture

@pytest.fixture
def base(tmp_path, monkeypatch):
    """Fixture: spun-up test server with temporary stores."""
    faelle_dir = str(tmp_path / "faelle")
    monkeypatch.setattr(API, "FAELLE", faelle_dir)
    monkeypatch.setattr(api_auth, "_AUTH_USER", None)
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

# ------------------------------------------------------------------ P1.1 Auth Module Tests

class TestAuthModule:
    def test_password_hashing_via_bcrypt(self):
        """Auth module uses bcrypt for password hashing."""
        password = "testpassword1"
        hashed = AUTH._hash_pw(password)
        assert AUTH._check_pw(password, hashed) is True
        assert AUTH._check_pw("wrongpass", hashed) is False

    def test_jwt_secret_fallback_not_hardcoded(self):
        """JWT secret is not hardcoded (dev fallback only)."""
        # Fallback should be random hex on startup (32 bytes = 64 hex chars)
        assert isinstance(AUTH._JWT_SECRET, str)
        assert len(AUTH._JWT_SECRET) == 64

    def test_token_creation_and_verification(self):
        """JWT tokens can be created and verified."""
        username = "testuser"
        token = AUTH._create_token(username)
        decoded = jwt.decode(token, AUTH._JWT_SECRET, algorithms=[AUTH.JWT_ALG])
        assert decoded["sub"] == username
        assert "iat" in decoded
        assert "exp" in decoded

    def test_token_expiry(self):
        """JWT tokens expire after defined TTL (24h)."""
        username = "expirytest"
        token = AUTH._create_token(username)
        decoded = jwt.decode(token, AUTH._JWT_SECRET, algorithms=[AUTH.JWT_ALG])
        exp_time = decoded["exp"]
        now = datetime.now(timezone.utc).timestamp()
        # Token should expire in ~24h (allow 1h tolerance)
        assert 23 * 3600 <= (exp_time - now) <= 25 * 3600

    def test_token_blacklist_on_logout(self):
        """Logged out tokens are invalidated via JWT ID blacklist."""
        username = "logoutuser"
        token = AUTH._create_token(username)
        decoded = jwt.decode(token, AUTH._JWT_SECRET, algorithms=[AUTH.JWT_ALG])
        jti = decoded["jti"]
        AUTH._invalidate(jti)
        assert AUTH._is_invalidated(jti) is True

# ------------------------------------------------------------------ P1.2 Password Policy

class TestPasswordPolicy:
    def test_password_requirements(self):
        """Password policy requires min 8 characters."""
        assert AUTH._PW_RE.match("short") is None
        assert AUTH._PW_RE.match("validpassword123") is not None

    def test_username_requirements(self):
        """Username policy requires starting letter and allowed chars (A-Za-z0-9_-)."""
        assert AUTH._USER_RE.match("validUser123") is not None
        assert AUTH._USER_RE.match("1invalid") is None  # starts with digit
        assert AUTH._USER_RE.match("-hyphen") is None   # starts with hyphen
        assert AUTH._USER_RE.match("invalid-user") is not None  # dash inside word = valid

    def test_username_case_sensitive(self, tmp_path, monkeypatch):
        """Usernames are case-sensitive in the system.

        Braucht `tmp_path`/`monkeypatch` statt der `base`-Fixture: der Test kommt ohne
        Server aus, lenkt den Store aber selbst um. Ohne die Umlenkung las er die ECHTE
        produkt/auth/users.json und schrieb sie zurueck — so entstand dort am
        2026-08-20 ein Konto "TestUser" (Befund B2). Er blieb dabei gruen, weil sein
        eigener Rueckstand aus dem Vorlauf die Zusicherung erfuellte.
        """
        monkeypatch.setattr(AUTH, "USER_STORE", str(tmp_path / "users.json"))
        store = AUTH._lade_users()
        store["users"]["TestUser"] = {
            "password_hash": AUTH._hash_pw("password1"),
            "created_at": datetime.now(timezone.utc).isoformat()
        }
        AUTH._speichere_users(store)
        reload_store = AUTH._lade_users()
        assert "TestUser" in reload_store["users"]

    # --- login prueft das Namensmuster (Vault decisions/login-prueft-das-namensmuster-vor-dem-nachschlagen)
    #
    # register() vergibt nur Namen nach _USER_RE. login() prueft das Muster nicht: stand in der
    # Nutzerdatei ein anderer Name (nur durch Handarbeit moeglich), bekam er ein gueltiges Token. Rust
    # weist ein solches Token danach an jeder Route mit 401 ab ("angemeldet, aber ueberall abgewiesen").
    # Zugangsdaten stehen nur im tmp_path-Store dieser Tests; sie sind Testwerte, keine echten.

    @staticmethod
    def _nutzerdatei_mit(tmp_path, monkeypatch, *namen):
        """Nutzerdatei in tmp_path, in der jeder Name (auch einer, den register() ablehnt) das
        Passwort `password1` hat — so wie nach Handarbeit an der echten Datei."""
        monkeypatch.setattr(AUTH, "USER_STORE", str(tmp_path / "users.json"))
        store = AUTH._lade_users()
        for name in namen:
            store["users"][name] = {"password_hash": AUTH._hash_pw("password1"),
                                    "created_at": datetime.now(timezone.utc).isoformat()}
        AUTH._speichere_users(store)

    @pytest.mark.parametrize("name", ["a b", "ab", "1abc", "name\n", "ä_name", "x" * 33])
    def test_login_weist_namen_ausserhalb_des_musters_ab(self, tmp_path, monkeypatch, name):
        """AK1: ein Name ausserhalb von _USER_RE bekommt kein Token, auch mit dem richtigen Passwort —
        401 mit demselben Text wie bei einem falschen Passwort. Ein gueltiger Name in derselben Datei
        loggt weiter ein (Positivkontrolle: ohne sie wuerde jede Ablehnung den Test erfuellen)."""
        self._nutzerdatei_mit(tmp_path, monkeypatch, name, "gueltig_er")
        status, body = AUTH.login({"username": "gueltig_er", "password": "password1"})
        assert status == 200 and body["username"] == "gueltig_er"

        with pytest.raises(AUTH.AuthError) as e:
            AUTH.login({"username": name, "password": "password1"})
        assert e.value.status == 401
        assert str(e.value) == "username oder password falsch"

    def test_login_ausserhalb_des_musters_liest_die_nutzerdatei_nicht_und_protokolliert(
            self, tmp_path, monkeypatch):
        """Die Pruefung steht VOR dem Nachschlagen: ein Name ausserhalb des Musters erreicht die Datei
        nie (hier wirft jeder Zugriff darauf). Der Fehlschlag wird protokolliert wie bisher: ein
        audit_fn-Aufruf `login_fehlgeschlagen` mit dem Namen."""
        self._nutzerdatei_mit(tmp_path, monkeypatch, "a b")

        def _nie(*a, **k):
            raise AssertionError("die Nutzerdatei wurde gelesen")
        monkeypatch.setattr(AUTH, "_lade_users", _nie)
        aufrufe = []
        with pytest.raises(AUTH.AuthError) as e:
            AUTH.login({"username": "a b", "password": "password1"},
                       audit_fn=lambda *args: aufrufe.append(args))
        assert e.value.status == 401
        assert aufrufe == [("a b", "login_fehlgeschlagen", None, None)]

    @pytest.mark.parametrize("wert", [["x"], {"a": 1}, 5, None, True])
    def test_login_username_kein_text_ist_401(self, tmp_path, monkeypatch, wert):
        """AK2: ein username, der kein Text ist, ergibt 401 — keine Liste/kein Objekt als TypeError
        (unhashable) und keine Zahl/None als Treffer. `True` ist kein Text, auch wenn ein Nutzer
        namens "True" existiert."""
        self._nutzerdatei_mit(tmp_path, monkeypatch, "True", "None")
        aufrufe = []
        with pytest.raises(AUTH.AuthError) as e:
            AUTH.login({"username": wert, "password": "password1"},
                       audit_fn=lambda *args: aufrufe.append(args))
        assert e.value.status == 401
        assert str(e.value) == "username oder password falsch"
        assert aufrufe == [(wert, "login_fehlgeschlagen", None, None)]

    @pytest.mark.parametrize("wert", [["x"], {"a": 1}])
    def test_login_username_liste_ueber_http_ist_401_nicht_500(self, base, wert):
        """AK2 am echten Server: vorher antwortete der Sammelfaenger in server.py mit 500
        ("TypeError: unhashable type"). Mit dem echten audit_fn, damit auch das Protokoll eine Liste
        als Nutzerkennung verkraftet."""
        status, body = _req(base, "POST", "/auth/login", {"username": wert, "password": "password1"},
                            erwarte=401)
        assert body == {"fehler": "username oder password falsch"}, body

# ------------------------------------------------------------------ P1.6 Audit Integration

class TestAuditIntegration:
    def test_auth_operations_audited(self, base):
        """Login/logout operations are audit-logged (registration not yet)."""
        # Register + login (login is what gets audited)
        _req(base, "POST", "/auth/register",
             {"username": "audituser1", "password": "password1"})
        _req(base, "POST", "/auth/login",
             {"username": "audituser1", "password": "password1"})
        _req(base, "POST", "/auth/logout",
             {"token": ""})

        entries = audit.lies()
        auth_entries = [e for e in entries if e.get("action") in ("login", "logout")]
        assert len(auth_entries) >= 1

    def test_user_creation_audited(self, base):
        """AK1: Die Anlage eines Nutzers schreibt einen Audit-Eintrag mit der Nutzerkennung."""
        _req(base, "POST", "/auth/register",
             {"username": "neunutzer", "password": "ein-geheimnis-123"}, erwarte=201)
        eintraege = [e for e in audit.lies() if e.get("action") == "register"]
        assert len(eintraege) == 1, f"kein register-Eintrag im Protokoll: {audit.lies()}"
        assert eintraege[0]["user_id"] == "neunutzer", eintraege[0]

    def test_register_audit_ohne_passwort(self, base):
        """AK3: Der register-Eintrag darf den Passwortwert nicht tragen.

        Kontrolle ueber den INHALT, nicht ueber den heutigen Code: der Wert ist ein Kanarienvogel.
        Findet ihn jemand im Protokoll, hat ein kuenftiger Fix den Request-Body mitgeschrieben —
        und das faellt nur auf, wenn der Wert gesucht wird."""
        kanarienvogel = "kanarienvogel-nicht-in-audit"
        _req(base, "POST", "/auth/register",
             {"username": "vogelhalter", "password": kanarienvogel}, erwarte=201)
        roh = json.dumps(audit.lies(), ensure_ascii=False)
        assert kanarienvogel not in roh, (
            f"Das Passwort steht im Audit-Protokoll: {audit.lies()}")

    def test_failed_login_attempt_audited(self, base):
        """AK2: Ein Fehl-Login schreibt einen Audit-Eintrag, die 401 bleibt."""
        _req(base, "POST", "/auth/register",
             {"username": "fehlversuch", "password": "das-richtige-123"}, erwarte=201)
        _req(base, "POST", "/auth/login",
             {"username": "fehlversuch", "password": "das-falsche-123"}, erwarte=401)
        eintraege = [e for e in audit.lies() if e.get("action") == "login_fehlgeschlagen"]
        assert len(eintraege) == 1, f"kein Eintrag zum Fehlversuch: {audit.lies()}"
        assert eintraege[0]["user_id"] == "fehlversuch", eintraege[0]

    def test_fehl_login_audit_ohne_passwort(self, base):
        """AK3 fuer den Fehl-Login: Der Eintrag darf den versuchten Passwortwert nicht tragen.

        Kanarienvogel statt Code-Kontrolle: der Wert ist erkennbar, und gesucht wird nach ihm.
        Ein kuenftiger Fix, der den Request-Body mitschreibt, faellt hier auf — bei einer Pruefung
        auf "der heutige Code schreibt nichts" nicht."""
        kanarienvogel = "kanarienvogel-nicht-in-audit"
        _req(base, "POST", "/auth/register",
             {"username": "fehlvogel", "password": "ein-echtes-123"}, erwarte=201)
        _req(base, "POST", "/auth/login",
             {"username": "fehlvogel", "password": kanarienvogel}, erwarte=401)
        roh = json.dumps(audit.lies(), ensure_ascii=False)
        assert kanarienvogel not in roh, (
            f"Das versuchte Passwort steht im Audit-Protokoll: {audit.lies()}")

# ------------------------------------------------------------------ P1.3 Session Security

class TestSessionSecurity:
    def test_session_timeout(self, monkeypatch):
        """Session tokens expire after configured TTL."""
        # Control: a fresh token verifies — otherwise the None below would prove nothing.
        assert AUTH.verify_token(AUTH._create_token("ttluser")) == "ttluser"
        monkeypatch.setattr(AUTH, "JWT_TTL_H", -1)  # exp one hour in the past
        assert AUTH.verify_token(AUTH._create_token("ttluser")) is None

    def test_concurrent_sessions_allowed(self):
        """Multiple concurrent sessions per user are allowed."""
        username = "multiuser"
        AUTH._invalidate("dummy")
        token1 = AUTH._create_token(username)
        token2 = AUTH._create_token(username)
        assert token1 != token2

    def test_secure_token_generation(self):
        """JWT tokens use secure random components."""
        token1 = AUTH._create_token("user1")
        token2 = AUTH._create_token("user1")
        assert token1 != token2  # Different JTI on each creation

# ------------------------------------------------------------------ P1.5 Rate Limiting (Placeholder)

class TestRateLimiting:
    def test_rate_limit_placeholder(self):
        """Rate limiting not implemented (placeholder for future dev)."""
        pytest.skip("Funktion fehlt: /auth/login drosselt Fehlversuche nicht, keine Sperre je Konto oder IP; "
                    "Vault-Ticket auth-audit-und-login-drosselung-fehlen")

# ------------------------------------------------------------------ P1.4 Authorization

class TestAuthorization:
    def _create_fall(self, base, token=None):
        """Helper: create a fall (unauthorized if no token)."""
        payload = {"fall_id": "authfall", "scheibe": "ep", "veranlagungszeitraum": 2025}
        return _req(base, "POST", "/fall", payload, token=token)

    def _login_and_create(self, base):
        """Helper: register, login, create fall."""
        _req(base, "POST", "/auth/register",
             {"username": "authowner", "password": "ownerpass"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "authowner", "password": "ownerpass"})
        self._create_fall(base, ld["token"])
        return ld["token"]

    def test_owner_access(self, base):
        """Owner tokens allow full system access."""
        token = self._login_and_create(base)
        endpoints = ["/fall/authfall/fragen", "/fall/authfall/stand"]
        for path in endpoints:
            s, _ = _req(base, "GET", path, token=token)
            assert s == 200

    def test_nonowner_access_denied(self, base):
        """Non-owner tokens → 403."""
        token = self._login_and_create(base)
        _req(base, "POST", "/auth/register",
             {"username": "attacker", "password": "password1"})
        _, ld = _req(base, "POST", "/auth/login",
                  {"username": "attacker", "password": "password1"})
        fremd_token = ld["token"]
        endpoints = ["/fall/authfall/fragen"]
        for path in endpoints:
            s, _ = _req(base, "GET", path, token=fremd_token, erwarte=403)

    def test_unauthorized_access_allowed_dev(self, base):
        """No auth header → development path allowed."""
        s, _ = self._create_fall(base)
        assert s == 201

# ------------------------------------------------------------------ Integration Smoke Tests

class TestSmokeTests:
    def test_full_auth_flow(self, base):
        """Complete authentication and authorization workflow."""
        # 1. Register
        _req(base, "POST", "/auth/register",
             {"username": "smoketest", "password": "smoke123"})
        # 2. Login
        status, data = _req(base, "POST", "/auth/login",
                          {"username": "smoketest", "password": "smoke123"})
        assert status == 200
        token = data["token"]
        # 3. Create fall
        s, _ = _req(base, "POST", "/fall",
                  {"fall_id": "smoke", "scheibe": "ep", "veranlagungszeitraum": 2025},
                  token=token)
        assert s == 201
        # 4. Access fall
        s, _ = _req(base, "GET", "/fall/smoke/fragen", token=token)
        assert s == 200
        # 5. Logout
        s, _ = _req(base, "POST", "/auth/logout", {"token": token})
        assert s == 200