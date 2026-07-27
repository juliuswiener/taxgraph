"""File-backed store — YAML-Persistenz um dict-API."""

from __future__ import annotations

import json
import os
import tempfile
from collections.abc import MutableMapping
from typing import Any


def canonical_json(obj) -> str:
    return json.dumps(obj, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


class FileStore(MutableMapping):
    """Dict-artiges Store-Interface backed by YAML-Datei.

    Selbe Schlüssel wie store.py: version, veranlagungszeitraum,
    fall_id, events, snapshots.
    Persistenz atomar via temp + os.replace.
    """

    def __init__(self, path: str, store_dict: dict | None = None):
        self._path = path
        if store_dict is not None:
            self._data = store_dict
        else:
            self._data = self._lade()

    # ---- Persistenz ---------------------------------------------------------

    def _lade(self) -> dict:
        if not os.path.exists(self._path):
            return {}
        import yaml
        with open(self._path, encoding="utf-8") as f:
            return yaml.safe_load(f) or {}

    def _speichere(self) -> None:
        import yaml
        d = os.path.dirname(self._path) or "."
        os.makedirs(d, exist_ok=True)
        tmp = tempfile.NamedTemporaryFile(
            "w", dir=d, delete=False, encoding="utf-8", suffix=".tmp"
        )
        try:
            yaml.safe_dump(self._data, tmp, allow_unicode=True, sort_keys=False)
            tmp.flush()
            os.fsync(tmp.fileno())
        finally:
            tmp.close()
        os.replace(tmp.name, self._path)

    # ---- MutableMapping -----------------------------------------------------

    _DICT_KEYS = frozenset({"version", "veranlagungszeitraum", "fall_id",
                            "events", "snapshots"})

    def __getitem__(self, key: str) -> Any:
        if key not in self._DICT_KEYS:
            raise KeyError(key)
        if key not in self._data:
            self._data[key] = [] if key in ("events", "snapshots") else None
        return self._data[key]

    def __setitem__(self, key: str, value: Any) -> None:
        if key not in self._DICT_KEYS:
            raise KeyError(key)
        self._data[key] = value

    def __delitem__(self, key: str) -> None:
        raise NotImplementedError("FileStore unterstützt kein Löschen von Schlüsseln.")

    def __iter__(self):
        return iter(self._DICT_KEYS)

    def __len__(self) -> int:
        return len(self._DICT_KEYS)

    def __contains__(self, key: object) -> bool:
        return key in self._DICT_KEYS

    # ---- Store-Methoden (delegieren an store.py) ----------------------------

    def append_event(self, **kwargs) -> dict:
        import produkt.store.store as ST
        return ST.append_event(self, **kwargs)

    def materialisiere(self, bis_event: str | None = None) -> tuple[dict, str]:
        import produkt.store.store as ST
        return ST.materialisiere(self, bis_event)

    def erzeuge_snapshot(self, **kwargs) -> dict:
        import produkt.store.store as ST
        return ST.erzeuge_snapshot(self, **kwargs)

    def lade_katalog(self, bindung: dict) -> dict:
        import produkt.store.store as ST
        return ST.lade_katalog(bindung)

    # ---- Persistenz explizit ------------------------------------------------

    def save(self) -> None:
        self._speichere()

    @classmethod
    def open(cls, path: str) -> "FileStore":
        return cls(path)

    @property
    def data(self) -> dict:
        return self._data
