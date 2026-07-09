"""YAML-Laden ohne stille Semantik.

`yaml.safe_load` nimmt bei einem doppelten Schluessel im selben Mapping
kommentarlos den letzten. Beim Neuschnitt von § 35a blieb ein alter
`test_seed:`-Block stehen; die Regel haette mit einem GRUENEN Test-Gate
dagestanden, das eine voellig andere Signatur prueft. Ein Gate, das man durch
einen vergessenen Block umgehen kann, ist kein Gate.

Deshalb wird jedes Manifest dieses Projekts strikt geladen: doppelte Schluessel
sind ein Fehler, kein Ueberschreiben.

    from yamlstrict import load_yaml
    cfg = load_yaml("pipeline/produktion/rules.yaml")

Betrifft alle Manifeste, die Gates steuern: rules.yaml, models.yaml, tasks.yaml,
golden/cases/*, params/*, docstore- und sources-Metadaten.
"""

from __future__ import annotations

import yaml


class StrictLoader(yaml.SafeLoader):
    """SafeLoader, der doppelte Mapping-Schluessel als Fehler meldet."""


def _no_duplicates(loader: StrictLoader, node, deep: bool = False) -> dict:
    mapping: dict = {}
    for key_node, value_node in node.value:
        key = loader.construct_object(key_node, deep=deep)
        if key in mapping:
            raise yaml.constructor.ConstructorError(
                None, None,
                f"doppelter Schluessel {key!r} - stille YAML-Semantik wuerde den "
                f"vorherigen Wert verwerfen (Zeile {key_node.start_mark.line + 1})",
                key_node.start_mark)
        mapping[key] = loader.construct_object(value_node, deep=deep)
    return mapping


StrictLoader.add_constructor(
    yaml.resolver.BaseResolver.DEFAULT_MAPPING_TAG, _no_duplicates)


class DuplicateKeyError(RuntimeError):
    """Ein Manifest enthaelt einen doppelten Schluessel."""


def load_str(text: str, herkunft: str = "<string>"):
    try:
        return yaml.load(text, Loader=StrictLoader)
    except yaml.constructor.ConstructorError as e:
        raise DuplicateKeyError(f"{herkunft}: {e.problem}") from None


def load_yaml(path: str):
    with open(path, encoding="utf-8") as f:
        return load_str(f.read(), herkunft=path)
