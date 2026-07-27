"""Sachverhalts-Store — Factory + Backend-Abstraktion.

create_store(backend="file"|"sql") wählt das Backend.
STORE_BACKEND=sql|file env-var überschreibt.

FileStore  → YAML-Datei (wie bestehendes store.py)
SQLStore   → SQLite (mit Optimistic Locking)
"""

from __future__ import annotations

import os

from produkt.store.file_backend import FileStore
from produkt.store.sql_backend import SQLStore, ConcurrencyError


def create_store(backend: str | None = None, *,
                 path: str | None = None,
                 db_path: str | None = None,
                 fall_id: str | None = None,
                 veranlagungszeitraum: int | None = None,
                 store_dict: dict | None = None) -> FileStore | SQLStore:
    """Factory — erzeugt Store-Instanz für given backend.

    backend: "file" | "sql" (default: STORE_BACKEND env oder "file")
    """
    backend = backend or os.environ.get("STORE_BACKEND", "file")
    if backend == "sql":
        return SQLStore(
            db_path=db_path or os.environ.get("STORE_DB_PATH", "store.sqlite"),
            fall_id=fall_id,
            veranlagungszeitraum=veranlagungszeitraum,
        )
    return FileStore(
        path=path or "store.yaml",
        store_dict=store_dict,
    )


__all__ = ["create_store", "FileStore", "SQLStore", "ConcurrencyError"]
