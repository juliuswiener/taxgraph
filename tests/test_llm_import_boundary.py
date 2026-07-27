"""Guard-Test: LLM-Client ist auf api_llm.py isoliert. Keine anderen Haut-Module dürfen llm_client importieren."""

import os
import ast


def _parse_imports(source: str) -> tuple[set[str], set[str]]:
    """Parse Python source, return (top-level imports, from-imports)."""
    top_level = set()
    from_imports = set()
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return top_level, from_imports

    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                top_level.add(alias.name)
        elif isinstance(node, ast.ImportFrom):
            if node.module:
                from_imports.add(node.module)
    return top_level, from_imports


def test_llm_client_not_in_api():
    """api.py darf NICHT llm_client importieren."""
    api_path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "produkt", "haut", "api.py"
    )
    with open(api_path, 'r') as f:
        source = f.read()
    top_level, from_imports = _parse_imports(source)
    assert "llm_client" not in top_level, f"api.py importiert llm_client (sollte nur in api_llm.py sein)"
    assert "llm_client" not in from_imports, f"api.py hat 'from llm_client' (sollte nur in api_llm.py sein)"


def test_llm_client_only_in_api_llm():
    """NUR api_llm.py darf llm_client importieren."""
    api_llm_path = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "produkt", "haut", "api_llm.py"
    )
    with open(api_llm_path, 'r') as f:
        source = f.read()
    top_level, from_imports = _parse_imports(source)
    # api_llm.py MUSS llm_client haben (oder lazy importieren)
    assert "llm_client" in top_level or "llm_client" in from_imports, \
        f"api_llm.py sollte llm_client importieren"


def test_no_llm_client_in_other_haut_modules():
    """api_engines.py, api_guards.py, api_helpers.py dürfen NICHT llm_client importieren (falls sie existieren)."""
    haut_dir = os.path.join(
        os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
        "produkt", "haut"
    )

    # Module die KEIN llm_client importieren dürfen
    non_llm_modules = ["api_engines.py", "api_guards.py", "api_helpers.py"]

    for module_name in non_llm_modules:
        path = os.path.join(haut_dir, module_name)
        if not os.path.exists(path):
            continue  # Modul existiert noch nicht (OK, nicht failen)

        with open(path, 'r') as f:
            source = f.read()
        top_level, from_imports = _parse_imports(source)
        assert "llm_client" not in top_level, \
            f"{module_name} importiert llm_client (sollte nur in api_llm.py sein)"
        assert "llm_client" not in from_imports, \
            f"{module_name} hat 'from llm_client' (sollte nur in api_llm.py sein)"
