"""Gate (L2, team-lead-Auftrag): JEDE append_event-Aufrufstelle unter produkt/ übergibt bindung=.

store.py:append_event() prüft _pruefe_typ_konformitaet() (Auflage T, Stille-Null-Klasse) NUR,
wenn bindung is not None — ohne bindung= ist der Aufruf ein blinder Fleck, der ein
falsch-typisiertes Feld schweigend durchlässt (genau die Lücke, die L1 in
produkt/import/vorjahr_writer.py:50 schloss: der einzige Aufrufer, der bindung NICHT
übergab, obwohl es längst als Parameter vorlag).

Findet Aufrufstellen per AST (gleiches Muster wie test_llm_import_boundary.py), nicht per
Textsuche — eine Definitionsstelle (`def append_event`) ist kein Call-Knoten und taucht hier
nie auf. Ausnahmen (Muster: BLOCKIERTE_BLOECKE in test_checkest_blockmatrix.py) sind benannt
und begründet, kein stilles Ausklammern.
"""
import ast
import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
PRODUKT = os.path.join(ROOT, "produkt")

# Aufrufstellen, die bindung NICHT literal als Keyword tragen — mit Begründung, warum das
# hier kein Auflage-T-Loch ist. Jeder Eintrag ist eine geprüfte Ausnahme: beide Tests unten
# wachen, dass sie erstens wirklich nur **kwargs weiterreicht und zweitens noch existiert.
AUSNAHMEN = {
    ("store/file_backend.py", 88): (
        "FileBackend.append_event(self, **kwargs) ist ein reiner Passthrough zu "
        "ST.append_event(self, **kwargs) — bindung reist mit, WENN der eigentliche Aufrufer "
        "(z.B. api.py) es setzt. Die Prüfung greift an der Stelle, die den Wert kennt, nicht "
        "an diesem Weiterreicher; ein bindung=None des Aufrufers würde dort selbst auffallen."
    ),
}


def _call_sites(pfad: str) -> list[ast.Call]:
    """Alle Call-Knoten in `pfad`, deren Ziel auf `.append_event` endet."""
    with open(pfad, encoding="utf-8") as f:
        baum = ast.parse(f.read(), filename=pfad)
    return [
        node for node in ast.walk(baum)
        if isinstance(node, ast.Call) and (
            (isinstance(node.func, ast.Attribute) and node.func.attr == "append_event")
            or (isinstance(node.func, ast.Name) and node.func.id == "append_event")
        )
    ]


def _hat_kwargs_weiterreichung(call: ast.Call) -> bool:
    """True, wenn der Aufruf **kwargs weiterreicht (arg=None-Keyword)."""
    return any(kw.arg is None for kw in call.keywords)


def _hat_bindung(call: ast.Call) -> bool:
    return any(kw.arg == "bindung" for kw in call.keywords)


def _alle_py_dateien():
    for dirpath, _, filenames in os.walk(PRODUKT):
        for fn in filenames:
            if fn.endswith(".py"):
                yield os.path.join(dirpath, fn)


def test_jede_append_event_aufrufstelle_hat_bindung():
    """Kern-Gate: ohne bindung= prüft Auflage T dort nicht — fail-closed nur, wo der Aufruf
    es überhaupt zulässt."""
    fehlend = []
    for pfad in _alle_py_dateien():
        rel = os.path.relpath(pfad, PRODUKT)
        for call in _call_sites(pfad):
            schluessel = (rel, call.lineno)
            if schluessel in AUSNAHMEN or _hat_kwargs_weiterreichung(call):
                continue
            if not _hat_bindung(call):
                fehlend.append(f"{rel}:{call.lineno}")
    assert not fehlend, (
        "append_event()-Aufrufstellen ohne bindung= (Auflage T greift dort NICHT):\n"
        + "\n".join(f"  - {f}" for f in fehlend))


def test_kwargs_weiterreicher_sind_benannt():
    """Ein **kwargs-Weiterreicher ohne bindung im eigenen Aufruf ist nur dann kein blinder
    Fleck, wenn er in AUSNAHMEN steht — sonst würde ein NEUER Passthrough den Gate-Test
    stillschweigend umgehen."""
    unbenannt = []
    for pfad in _alle_py_dateien():
        rel = os.path.relpath(pfad, PRODUKT)
        for call in _call_sites(pfad):
            if _hat_kwargs_weiterreichung(call) and not _hat_bindung(call):
                schluessel = (rel, call.lineno)
                if schluessel not in AUSNAHMEN:
                    unbenannt.append(f"{rel}:{call.lineno}")
    assert not unbenannt, (
        "**kwargs-Weiterreicher ohne AUSNAHMEN-Eintrag (Begründung fehlt):\n"
        + "\n".join(f"  - {f}" for f in unbenannt))


def test_ausnahmen_sind_begruendet():
    """Jeder Ausnahme-Eintrag nennt einen Grund — kein stilles Ausklammern (Muster:
    BLOCKIERTE_BLOECKE in test_checkest_blockmatrix.py)."""
    for schluessel, grund in AUSNAHMEN.items():
        assert grund and len(grund) > 20, f"{schluessel}: Ausnahme ohne (ausreichende) Begründung"


def test_ausnahmeliste_hat_keine_toten_eintraege():
    """Jeder AUSNAHMEN-Eintrag muss auf eine tatsächlich existierende Aufrufstelle zeigen —
    sonst täuscht die Liste eine Abdeckung vor, die durch Umbau längst verschoben ist und der
    Gate-Test hätte eine echte neue Lücke gerade NICHT gesehen."""
    alle_calls = set()
    for pfad in _alle_py_dateien():
        rel = os.path.relpath(pfad, PRODUKT)
        for call in _call_sites(pfad):
            alle_calls.add((rel, call.lineno))
    for schluessel in AUSNAHMEN:
        assert schluessel in alle_calls, (
            f"{schluessel} steht in AUSNAHMEN, zeigt aber auf keine Aufrufstelle mehr — "
            f"toter Eintrag, Zeile hat sich verschoben oder der Code ist weg.")
