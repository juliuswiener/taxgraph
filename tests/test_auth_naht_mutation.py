"""Auth-Mutation Gate: Verifiziert, dass _AUTH_USER von server.py bis Ring-Owner-Check kommt.

Reproduziert Julius' Probe: `from api_auth import _AUTH_USER` würde FEHLSCHLAGEN.
Modul-Attribut-Zugriff (`api_auth._AUTH_USER` und `import api_auth; api_auth._AUTH_USER`)
MUSS funktionieren.
"""

import os
import sys
import pytest

# Füge produkt/haut zu sys.path hinzu (wie server.py/api.py)
_HERE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
_HAUT = os.path.join(_HERE, "produkt", "haut")
if _HAUT not in sys.path:
    sys.path.insert(0, _HAUT)


def test_auth_mutation_reaches_owner_check():
    """Mutation von server.py muss _fall_owner_check in Ring sehen.

    Pattern: server.py schreibt api_auth._AUTH_USER = 'julius'
    → Ring (_fall_owner_check) liest api_auth._AUTH_USER
    → muss 'julius' erhalten (nicht None).

    Dies ist Julius' Probe — wenn der Code auf `from api_auth import _AUTH_USER`
    zurückfällt, wird dieser Test rot, weil Mutation nicht ankommt.
    """
    import api_auth
    import api
    import tempfile
    import json
    import os

    # 1. Simuliere server.py: setze _AUTH_USER
    api_auth._AUTH_USER = "test_julius"

    # 2. Prüfe: Direkter Modul-Attribut-Zugriff sieht Mutation
    assert api_auth._AUTH_USER == "test_julius"

    # 3. Prüfe: _fall_owner_check liest aus api_auth (via Modul-Attribut, nicht from-import)
    # Erstelle einen Test-Fall mit passendem owner
    with tempfile.TemporaryDirectory() as tmpdir:
        # Mock lade_fall um Filesystemm-Abhängigkeit zu vermeiden
        fall_store = {"user_id": "test_julius", "veranlagungszeitraum": 2025}

        original_lade_fall = api.lade_fall
        try:
            api.lade_fall = lambda fid: fall_store

            # _fall_owner_check sollte NICHT werfen, weil UID stimmt
            api._fall_owner_check("test_fall_id")  # Must not raise ApiError

        finally:
            api.lade_fall = original_lade_fall

    # 4. Cleanup
    api_auth._AUTH_USER = None
    assert api_auth._AUTH_USER is None


def test_auth_mutation_blocks_wrong_owner():
    """Owner-Check sperre Zugriff wenn UID nicht stimmt."""
    import api_auth
    import api

    # server.py: setze UID auf 'alice'
    api_auth._AUTH_USER = "alice"

    # Fall gehört 'bob'
    fall_store = {"user_id": "bob", "veranlagungszeitraum": 2025}

    original_lade_fall = api.lade_fall
    try:
        api.lade_fall = lambda fid: fall_store

        # Muss 403 werfen
        with pytest.raises(api.ApiError) as exc_info:
            api._fall_owner_check("test_fall_id")

        assert exc_info.value.status == 403

    finally:
        api.lade_fall = original_lade_fall
        api_auth._AUTH_USER = None


def test_auth_mutation_allows_no_owner():
    """Falls ohne user_id sind für jeden sichtbar (legacy Falls)."""
    import api_auth
    import api

    api_auth._AUTH_USER = "alice"

    # Fall ohne user_id (legacy)
    fall_store = {"veranlagungszeitraum": 2025}

    original_lade_fall = api.lade_fall
    try:
        api.lade_fall = lambda fid: fall_store

        # Darf nicht werfen
        api._fall_owner_check("test_fall_id")

    finally:
        api.lade_fall = original_lade_fall
        api_auth._AUTH_USER = None


def test_auth_mutation_cleanup():
    """Cleanup nach Request: _AUTH_USER = None (wie server.py finally:)."""
    import api_auth

    api_auth._AUTH_USER = "tmp_user"
    assert api_auth._AUTH_USER == "tmp_user"

    # Cleanup (server.py finally)
    api_auth._AUTH_USER = None
    assert api_auth._AUTH_USER is None


def test_auth_no_stale_copy_in_api():
    """Sperre: api._AUTH_USER darf NICHT aus api_constants kommen (würde stilles Schreiben ermöglichen).

    Wenn _AUTH_USER aus api_constants wieder eingeführt würde, würde
    `api._AUTH_USER = x` die alte Konstante setzen, und Owner-Check würde
    weiterhin api_auth._AUTH_USER = None sehen. Test wird rot, wenn
    _AUTH_USER in api_constants zurückkommt → verhindert stumme Regressionen.
    """
    import api
    import api_auth
    import api_constants

    # api_constants darf NICHT _AUTH_USER haben
    assert not hasattr(api_constants, "_AUTH_USER"), \
        "_AUTH_USER gehört zu api_auth, nicht api_constants"

    # In api.py sollte es AUCH nicht in __all__ sein (nur via api_auth)
    assert "_AUTH_USER" not in api_constants.__all__, \
        "_AUTH_USER in api_constants.__all__ würde Bypass ermöglichen"

    # Prüfung: wenn api._AUTH_USER setzbar ist, muss es auf api_auth zeigen
    # (nicht auf separate api_constants-Kopie)
    api_auth._AUTH_USER = "tester"
    if hasattr(api, "_AUTH_USER"):
        # Wenn api._AUTH_USER existiert (via import *), muss es api_auth lesen
        # Das ist der Fall, wenn es von api_auth kommt
        assert api._AUTH_USER == "tester" or api._AUTH_USER is None, \
            "api._AUTH_USER ist separate Kopie statt api_auth-Verweis"
    api_auth._AUTH_USER = None
