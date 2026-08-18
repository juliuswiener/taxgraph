"""P1.1 Authentifizierung — JWT-basiert, Passwort-Hashing mit bcrypt, User-Store als JSON.

Env-var `TAXGRAPH_JWT_SECRET` für Signing-Key; ohne (dev) → zufälliger Key pro Start.
Startet ohne User (erster POST /auth/register legt den ersten Admin an — wer zuerst kommt …).
"""
from __future__ import annotations

import json
import os
import re
import secrets
from datetime import datetime, timedelta, timezone

import bcrypt
import jwt

HERE = os.path.dirname(os.path.abspath(__file__))
USER_STORE = os.environ.get("TAXGRAPH_USER_STORE") or os.path.join(HERE, "users.json")
JWT_ALG = "HS256"
JWT_TTL_H = 24  # Session gilt 24h

# JWT-Secret: Env-Var oder dev-fallback (zufällig pro Start — invalidiert ALLE Tokens)
_JWT_SECRET = os.environ.get("TAXGRAPH_JWT_SECRET") or secrets.token_hex(32)

_USER_RE = re.compile(r"^[a-zA-Z][a-zA-Z0-9_-]{2,31}$")
_PW_RE = re.compile(r"^.{8,128}$")


class AuthError(ValueError):
    def __init__(self, status: int, msg: str):
        super().__init__(msg)
        self.status = status


# ------------------------------------------------------------------ User-Store

def _lade_users() -> dict:
    if not os.path.exists(USER_STORE):
        return {"users": {}}
    with open(USER_STORE, encoding="utf-8") as f:
        return json.load(f)


def _speichere_users(store: dict) -> None:
    """Schreibt die Nutzerdatenbank atomar und NUR für den Eigentümer lesbar.

    `open(tmp, "w")` erbt die umask — gemessen 0644 auf der Platte (Audit 2026-08-16,
    sec-users-json-world-readable). Darin stehen bcrypt-Hashes: jeder Nutzer des Rechners
    konnte sie mitlesen und offline gegen sie rechnen. bcrypt macht das teuer, nicht unmöglich,
    und ein Hash, den niemand lesen kann, muss gar nicht erst standhalten.

    os.open mit 0o600 statt chmod NACH dem Schreiben: dazwischen läge ein Zeitfenster, in dem
    die Datei mit Hashes bereits existiert und noch für alle lesbar ist. Der Modus gilt nur bei
    Neuanlage — eine vorhandene tmp-Datei behielte ihren; deshalb O_EXCL, das eine solche
    ablehnt, statt sie stillschweigend weiterzuverwenden."""
    d = os.path.dirname(USER_STORE) or "."
    os.makedirs(d, exist_ok=True)
    tmp = USER_STORE + ".tmp"
    if os.path.exists(tmp):
        os.unlink(tmp)          # Rest eines abgebrochenen Laufs; Rechte davon sind unbekannt
    fd = os.open(tmp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(store, f, ensure_ascii=False, sort_keys=True)
        f.flush()
        os.fsync(f.fileno())
    os.replace(tmp, USER_STORE)


# ------------------------------------------------------------------ Passwort

def _hash_pw(pw: str) -> str:
    return bcrypt.hashpw(pw.encode("utf-8"), bcrypt.gensalt()).decode("utf-8")


def _check_pw(pw: str, hashed: str) -> bool:
    return bcrypt.checkpw(pw.encode("utf-8"), hashed.encode("utf-8"))


# ------------------------------------------------------------------ JWT

def _create_token(user_id: str) -> str:
    now = datetime.now(timezone.utc)
    payload = {
        "sub": user_id,
        "iat": now,
        "exp": now + timedelta(hours=JWT_TTL_H),
        "jti": secrets.token_hex(16),
    }
    return jwt.encode(payload, _JWT_SECRET, algorithm=JWT_ALG)


def verify_token(token: str) -> str | None:
    """Gibt user_id zurück oder None bei ungültigem/abgelaufenem/invalidiertem Token. PUBLIC — genutzt vom Dispatch."""
    try:
        payload = jwt.decode(token, _JWT_SECRET, algorithms=[JWT_ALG])
    except jwt.PyJWTError:
        return None
    if _is_invalidated(payload.get("jti", "")):
        return None
    return payload.get("sub")


def _extract_bearer(raw: str) -> str:
    if raw.startswith("Bearer "):
        return raw[7:]
    return raw


# ------------------------------------------------------------------ Blacklist (In-Memory)
# Bei Neustart verloren → alle Sessions ungültig (dev-fallback ähnlich).
_logout_blacklist: set = set()


def _invalidate(jti: str) -> None:
    _logout_blacklist.add(jti)


def _is_invalidated(jti: str) -> bool:
    return jti in _logout_blacklist


# ------------------------------------------------------------------ API-Handler

_REGISTER_FELDER = {"username", "password"}


def register(body: dict) -> tuple[int, dict]:
    missing = _REGISTER_FELDER - set(body)
    if missing:
        raise AuthError(400, f"Pflichtfelder fehlen: {sorted(missing)}")
    username = body["username"]
    password = body["password"]
    if not _USER_RE.fullmatch(username):
        raise AuthError(400, "username: 3-32 Zeichen, beginnt mit Buchstabe, nur A-Za-z0-9_-")
    if not _PW_RE.fullmatch(password):
        raise AuthError(400, "password: 8-128 Zeichen")
    store = _lade_users()
    if username in store["users"]:
        raise AuthError(409, f"User {username!r} existiert bereits")
    store["users"][username] = {
        "password_hash": _hash_pw(password),
        "created_at": datetime.now(timezone.utc).isoformat(),
    }
    _speichere_users(store)
    return 201, {"username": username, "message": "registriert"}


_LOGIN_FELDER = {"username", "password"}


def login(body: dict, audit_fn=None) -> tuple[int, dict]:
    missing = _LOGIN_FELDER - set(body)
    if missing:
        raise AuthError(400, f"Pflichtfelder fehlen: {sorted(missing)}")
    username = body["username"]
    password = body["password"]
    store = _lade_users()
    user = store["users"].get(username)
    if not user or not _check_pw(password, user["password_hash"]):
        raise AuthError(401, "username oder password falsch")
    token = _create_token(username)
    if audit_fn:
        audit_fn(username, "login", None, None)
    return 200, {"token": token, "username": username}


def logout(body: dict, audit_fn=None) -> tuple[int, dict]:
    token = _extract_bearer(body.get("token") or "")
    if token:
        try:
            payload = jwt.decode(token, _JWT_SECRET, algorithms=[JWT_ALG])
            _invalidate(payload.get("jti", ""))
            user_id = payload.get("sub", "")
            if audit_fn and user_id:
                audit_fn(user_id, "logout", None, None)
        except jwt.PyJWTError:
            pass
    return 200, {"message": "abgemeldet"}
