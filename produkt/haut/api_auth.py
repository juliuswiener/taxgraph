"""Request-scoped Authentifizierung — Modul-Attribut für Mutation-Sicherheit.

Pattern: Modul-Objekt statt Name-Import. server.py schreibt api_auth._AUTH_USER,
Ring liest api_auth._AUTH_USER. Attribut-Zugriff wird JEDEM Zugriff neu aufgelöst,
Mutation kommt an (im Gegensatz zu `from api_auth import _AUTH_USER`).
"""

_AUTH_USER: str | None = None
"""Request-Benutzer, von server._dispatch gesetzt/gelöscht."""
