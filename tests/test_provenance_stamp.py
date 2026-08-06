"""Test: provenance — der Hash-Beweis und die Falschgrün-nahen Kanten.

Die Lauf-Gültigkeit hängt an EINEM Invariant: jeder Stamp trägt denselben
models.yaml-Hash. Wenn stamp() den Hash verliert (oder models_yaml_hash instabil
ist), merkt die Lauf-Prüfung in cascade.run_candidate (len(hashes) > 1) das nie,
weil alle Stamps leer sind — genau die stille Naht, die hier weh tut.

Der Lauf-Pfad selbst (hashes pruefen) liegt in cascade.run_candidate und wird hier
nicht aufgerufen (braucht LLM); getestet wird die Baustufe darunter: der Hash ist
deterministisch, der Stamp traegt ihn, die Rollen-Ladung ist strikt.
"""
import sys
import os

sys.path.insert(0, os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "pipeline"))

import provenance as PV  # noqa: E402
from client import RoleConfig, Completion  # noqa: E402


# -- models_yaml_hash: deterministisch und im erwarteten Format -----------------

def test_hash_format():
    h = PV.models_yaml_hash()
    assert h.startswith("sha256:")
    assert len(h) == len("sha256:") + 16  # hexdigest[:16]


def test_hash_stabil():
    h1 = PV.models_yaml_hash()
    h2 = PV.models_yaml_hash()
    assert h1 == h2


def test_hash_different_file_same_content(tmp_path):
    """Inhalt bestimmt den Hash, nicht der Pfad."""
    p = tmp_path / "models.yaml"
    p.write_bytes(open(PV.MODELS_YAML, "rb").read())
    assert PV.models_yaml_hash(str(p)) == PV.models_yaml_hash()


def test_hash_kollision_versch_across_len(tmp_path):
    """Zwei Dateien, gleiche Laenge, verschiedener Inhalt -> NIE gleicher Hash.

    Fängt eine Degeneration, die den Hash nur laengenabhaengig macht (z.B.
    hexdigest[:16] an eine dateigroessen-Abhaengigkeit gekoppelt): solange der
    Inhalt nicht eingeht, kollidieren zwei gleich lange, verschiedene Dateien.
    """
    a = tmp_path / "a.yaml"
    b = tmp_path / "b.yaml"
    a.write_text("role1\n")
    b.write_text("role2\n")  # gleiche Laenge, anderer Inhalt
    assert len(a.read_text()) == len(b.read_text())
    assert PV.models_yaml_hash(str(a)) != PV.models_yaml_hash(str(b))


# -- load_roles: Rollen-Ladung + Hash in einem ---------------------------------

MIN_YAML = """\
roles:
  worker:
    slug: some/model
    providers: [p1]
"""


def test_load_roles_minimal(tmp_path):
    p = tmp_path / "models.yaml"
    p.write_text(MIN_YAML)
    roles, h = PV.load_roles(str(p))
    assert set(roles) == {"worker"}
    assert roles["worker"].role == "worker"
    assert roles["worker"].slug == "some/model"
    assert roles["worker"].providers == ["p1"]
    assert h == PV.models_yaml_hash(str(p))


def test_load_roles_defaults(tmp_path):
    """Nicht gesetzte optional keys fallen auf Defaults, kein Crash."""
    p = tmp_path / "models.yaml"
    p.write_text(MIN_YAML)
    roles, _ = PV.load_roles(str(p))
    w = roles["worker"]
    assert w.temperature == 0.0
    assert w.max_tokens == 4096
    assert w.prompt_template_id == ""
    assert w.fewshot_set_id == ""
    assert w.repair_template_id == ""


def test_load_roles_doppelter_schluessel_failt(tmp_path):
    """yamlstrict: doppelte roles-Schluessel sind ein Fehler (fail-closed)."""
    p = tmp_path / "models.yaml"
    p.write_text("roles:\n  worker:\n    slug: a\n    slug: b\n    providers: [x]\n")
    try:
        PV.load_roles(str(p))
        assert False, "doppelter Schluessel muss failen"
    except Exception:
        pass


def test_load_roles_ohne_roles_failt(tmp_path):
    p = tmp_path / "models.yaml"
    p.write_text("irgendwas: [1]\n")
    try:
        PV.load_roles(str(p))
        assert False, "fehlende roles-Sektion muss failen"
    except Exception:
        pass


# -- stamp: der Hash-Beweis haelt -----------------------------------------------

def _comp(**kw):
    base = dict(text="x", role="worker", slug="some/model", provider="p1",
                prompt_tokens=10, completion_tokens=5, cost_usd=0.01)
    base.update(kw)
    return Completion(**base)


def test_stamp_traegt_models_hash():
    role = RoleConfig(role="worker", slug="some/model", providers=["p1"],
                      prompt_template_id="pt_1", fewshot_set_id="fs_1")
    h = "sha256:deadbeefdeadbeef"
    prov = PV.stamp(role, _comp(), h)
    # WICHTIG: der Stamp bekommt den Hash des LAUFS, nicht einen eigenen.
    assert prov.models_yaml_hash == h
    assert prov.role == "worker"
    assert prov.slug == "some/model"
    assert prov.provider == "p1"
    assert prov.prompt_template_id == "pt_1"
    assert prov.fewshot_set_id == "fs_1"
    assert prov.prompt_tokens == 10
    assert prov.completion_tokens == 5
    assert prov.cost_usd == 0.01
    assert prov.truncated is False


def test_stamp_traegt_truncated_flag():
    """finish_reason=length muss als truncated auf dem Stamp ankommen."""
    role = RoleConfig(role="worker", slug="some/model", providers=["p1"])
    prov = PV.stamp(role, _comp(truncated=True), "sha256:abc")
    assert prov.truncated is True


def test_stamp_to_dict_vollstaendig():
    role = RoleConfig(role="worker", slug="some/model", providers=["p1"])
    prov = PV.stamp(role, _comp(), "sha256:abc")
    d = prov.to_dict()
    assert d["models_yaml_hash"] == "sha256:abc"
    assert d["prompt_tokens"] == 10
    assert "timestamp" in d and d["timestamp"]  # ISO gesetzt


def test_now_iso_utc_mit_sekunden():
    s = PV.now_iso()
    assert "T" in s
    assert s.endswith("+00:00") or s.endswith("Z")
    # timespec="seconds": kein Mikrosekunden-Teil
    assert "." not in s.split("T")[1]


# -- redact: Key-Maskierung fuer Logs -------------------------------------------

def test_redact_maskt_sk_key():
    result = PV.redact({"api": "sk-or-abcdef0123456789"})
    assert "sk-or-***" in result
    assert "sk-or-abcdef0123456789" not in result


def test_redact_laesst_normalen_text():
    assert PV.redact("kein key hier") == "kein key hier"
