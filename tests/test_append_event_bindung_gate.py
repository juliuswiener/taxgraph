"""Gate (L2, team-lead-Auftrag): JEDE append_event-Aufrufstelle unter produkt/ übergibt bindung=.

store.py:append_event() prüft _pruefe_typ_konformitaet() (Auflage T, Stille-Null-Klasse) NUR,
wenn bindung is not None — ohne bindung= ist der Aufruf ein blinder Fleck, der ein
falsch-typisiertes Feld schweigend durchlässt (genau die Lücke, die L1 in
produkt/eingang/vorjahr_writer.py:50 schloss: der einzige Aufrufer, der bindung NICHT
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
# LEER seit 2026-08-19. Der einzige Eintrag war FileBackend.append_event — ein reiner
# Passthrough (self, **kwargs) zu ST.append_event, an dem die Auflage-T-Prüfung nichts zu
# suchen hatte, weil er den Wert nicht kennt. Die Datei ist mit der Store-Abstraktion gelöscht
# worden (null Produktionsaufrufer, Entscheidung Julius); der Tote-Einträge-Wächter unten hat
# den Übergang gemeldet, statt ihn stumm durchgehen zu lassen.
#
# Die Liste bleibt als Ort für die nächste begründete Ausnahme. Leer heisst: JEDER Aufruf von
# append_event im Repo reicht `bindung` mit — kein Sonderfall, keine Erklärung nötig.
# Hier fehlt das Keyword ganz; der Fall "Keyword da, Wert zur Laufzeit None" steht darunter.
AUSNAHMEN: dict[tuple[str, int], str] = {}

# Aufrufstellen, deren bindung= einen Parameter mit Vorgabe None durchreicht: das Keyword steht da,
# der Kern-Gate ist zufrieden, zur Laufzeit kann der Wert trotzdem None sein. Schlüssel ist
# (Datei, Funktion), nicht die Zeile — ein Umbau darüber verschiebt den Eintrag nicht.
OPTIONAL_WEITERGEREICHT: dict[tuple[str, str], str] = {
    (os.path.join("eingang", "elster_writer.py"), "uebernehme_edaten"):
        "kein Produktionsaufrufer: nur tests/test_elster_writer.py und tests/test_vast_mapping.py rufen "
        "den eDaten-Import (gemessen 2026-09-26). Wer ihn verdrahtet, macht bindung zur Pflicht und "
        "streicht diesen Eintrag.",
}

# Aufrufstellen mit WOERTLICHEM bindung=None: das Keyword steht da, der Kern-Gate ist zufrieden,
# und zur Laufzeit wird NICHTS geprueft — dieselbe Klasse wie slots.get(name, 0), ein Vorgabewert,
# der wie eine Antwort aussieht. Abgrenzung zum Durchreicher oben: dort reicht der Aufruf einen
# PARAMETER durch, der None sein KANN (ast.Name); hier steht das None im Aufruf selbst.
# Leer heisst: keine Stelle schreibt ein woertliches None hin. Wer es doch muss, traegt sie hier
# mit Grund ein, statt dass der Kern-Gate sie stillschweigend durchwinkt.
WOERTLICH_NONE: dict[tuple[str, str], str] = {}


def _ruft_append_event(call: ast.Call) -> bool:
    return ((isinstance(call.func, ast.Attribute) and call.func.attr == "append_event")
            or (isinstance(call.func, ast.Name) and call.func.id == "append_event"))


def _call_sites(pfad: str) -> list[ast.Call]:
    """Alle Call-Knoten in `pfad`, deren Ziel auf `.append_event` endet."""
    with open(pfad, encoding="utf-8") as f:
        baum = ast.parse(f.read(), filename=pfad)
    return [node for node in ast.walk(baum) if isinstance(node, ast.Call) and _ruft_append_event(node)]


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


def _optionale_weiterreicher() -> set[tuple[str, str]]:
    """(Datei, Funktion) je append_event-Aufruf, dessen bindung= einen Parameter der umschließenden
    Funktion mit Vorgabe None durchreicht."""
    gefunden = set()
    for pfad in _alle_py_dateien():
        rel = os.path.relpath(pfad, PRODUKT)
        with open(pfad, encoding="utf-8") as f:
            baum = ast.parse(f.read(), filename=pfad)
        for fn in ast.walk(baum):
            if not isinstance(fn, (ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            a = fn.args
            pos = a.posonlyargs + a.args
            paare = list(zip(pos[len(pos) - len(a.defaults):], a.defaults)) + list(zip(a.kwonlyargs, a.kw_defaults))
            mit_none = {p.arg for p, d in paare if isinstance(d, ast.Constant) and d.value is None}
            # ponytail: ast.walk zählt Aufrufe in verschachtelten Funktionen auch der äußeren zu — das
            # ergibt höchstens einen Fehlalarm (rot), nie ein stilles Grün.
            for call in ast.walk(fn):
                if isinstance(call, ast.Call) and _ruft_append_event(call) and any(
                        kw.arg == "bindung" and isinstance(kw.value, ast.Name) and kw.value.id in mit_none
                        for kw in call.keywords):
                    gefunden.add((rel, fn.name))
    return gefunden


def _woertliches_none() -> set[tuple[str, str]]:
    """(Datei, Funktion) je append_event-Aufruf mit `bindung=None` als WOERTLICHEM Wert.

    Streng getrennt von _optionale_weiterreicher() daneben: der Durchreicher schreibt `bindung=b`
    mit einem Parameter `b` (ast.Name, darf None sein und ist benannt), dieser Fall schreibt das
    None direkt hin (ast.Constant). Nur der zweite ist immer ein blinder Fleck — der erste ist eine
    begruendete Ausnahme, sobald der Writer verdrahtet ist.
    """
    gefunden = set()
    for pfad in _alle_py_dateien():
        rel = os.path.relpath(pfad, PRODUKT)
        with open(pfad, encoding="utf-8") as f:
            baum = ast.parse(f.read(), filename=pfad)
        for fn in ast.walk(baum):
            if not isinstance(fn, (ast.FunctionDef, ast.AsyncFunctionDef)):
                continue
            # ponytail: ast.walk zaehlt einen Aufruf in einer VERSCHACHTELTEN Funktion auch der
            # aeusseren zu — der Schluessel nennt dann die aeussere. Heute gibt es null solche
            # Faelle unter produkt/ (gemessen 2026-09-26); tritt einer auf, ist die Meldung eine
            # Zeile daneben, aber immer noch rot, nie still gruen.
            for call in ast.walk(fn):
                if isinstance(call, ast.Call) and _ruft_append_event(call) and any(
                        kw.arg == "bindung" and isinstance(kw.value, ast.Constant)
                        and kw.value.value is None
                        for kw in call.keywords):
                    gefunden.add((rel, fn.name))
    return gefunden


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


def test_scan_menge_ist_nicht_leer_und_enthaelt_reale_module():
    """Selbstprüfung des Scans: _alle_py_dateien() darf nicht leer sein und muss mindestens die
    zum Zeitpunkt dieses Tests bekannten realen Module unter produkt/ enthalten. Ohne diese
    Zusicherung wäre test_jede_append_event_aufrufstelle_hat_bindung bei einem stumm
    kollabierten Scan (verschobenes produkt/-Verzeichnis, falscher PRODUKT-Pfad) grün, obwohl
    er nichts geprüft hätte (Muster: test_llm_import_boundary.py::
    test_scan_menge_ist_nicht_leer_und_enthaelt_reale_module)."""
    gescannt = {os.path.relpath(p, PRODUKT) for p in _alle_py_dateien()}
    UNTERGRENZE = 30  # gemessen 2026-09-08: 36 .py-Dateien unter produkt/
    assert len(gescannt) >= UNTERGRENZE, (
        f"Nur {len(gescannt)} .py-Dateien unter produkt/ gefunden, erwartet mindestens "
        f"{UNTERGRENZE} — die Scan-Menge ist unerwartet klein geworden"
    )
    bekannte_module = {os.path.join("haut", "api.py"), os.path.join("store", "store.py")}
    fehlend = bekannte_module - gescannt
    assert not fehlend, f"Bekannte Module fehlen in der Scan-Menge: {sorted(fehlend)}"


def test_scan_findet_die_gemessenen_aufrufstellen():
    """Selbstprüfung der Aufrufstellen: alle Dateien da, aber keine Aufrufstelle gefunden — dann
    ist der Kern-Gate leer grün. So geschieht es, wenn append_event umbenannt wird und der Scan
    den alten Namen sucht: mit umbenannten Aufrufen meldete die Datei ohne diesen Test 5 passed."""
    anzahl = sum(len(_call_sites(p)) for p in _alle_py_dateien())
    UNTERGRENZE = 7  # gemessen 2026-09-26: haut/api.py x3, eingang/{beleg,elster,kontoauszug,vorjahr}_writer.py
    assert anzahl >= UNTERGRENZE, (
        f"Nur {anzahl} append_event-Aufrufstellen unter produkt/ gefunden, erwartet mindestens "
        f"{UNTERGRENZE} — der Scan sucht womöglich einen Namen, den keiner mehr ruft")


def test_optionale_weiterreicher_sind_benannt_und_begruendet():
    """bindung= mit einem Parameter, der None als Vorgabe hat, besteht den Kern-Gate, prüft zur
    Laufzeit aber womöglich nichts. Jeder solche Weiterreicher steht mit Grund in
    OPTIONAL_WEITERGEREICHT, jeder Eintrag dort zeigt auf einen, und die Liste wächst nicht still."""
    gefunden = _optionale_weiterreicher()
    benannt = set(OPTIONAL_WEITERGEREICHT)
    assert gefunden == benannt, (
        f"Optional weitergereichte bindung ohne Eintrag: {sorted(gefunden - benannt)}; "
        f"tote Einträge: {sorted(benannt - gefunden)}")
    for schluessel, grund in OPTIONAL_WEITERGEREICHT.items():
        assert len(grund) > 20, f"{schluessel}: Ausnahme ohne (ausreichende) Begründung"
    assert len(OPTIONAL_WEITERGEREICHT) == 1


def test_woertliches_none_ist_benannt_und_begruendet():
    """`bindung=None` besteht den Kern-Gate, weil er nur das Vorhandensein des Keywords prueft —
    geprueft wird dann nichts. Gemessen 2026-09-26: eine Sonde unter produkt/ mit woertlichem
    bindung=None liess den Kern-Gate auf 7 passed, erst diese Pruefung wird rot.

    Der Durchreicher daneben (bindung=<Parameter mit Vorgabe None>) ist NICHT dasselbe und darf
    hier nicht mitgerissen werden — er ist eine benannte Ausnahme in OPTIONAL_WEITERGEREICHT.

    Die Liste ist leer, solange keine Stelle ein woertliches None schreibt. Wer eines braucht,
    traegt sie hier mit Grund ein; ein stilles Durchwinken gibt es nicht."""
    gefunden = _woertliches_none()
    benannt = set(WOERTLICH_NONE)
    assert not gefunden - benannt, (
        "append_event-Aufrufstellen mit woertlichem bindung=None — der Kern-Gate sieht nur das "
        "Keyword und laesst sie durch, zur Laufzeit prueft Auflage T dort NICHTS:\n"
        + "\n".join(f"  - {rel}:{fn}" for rel, fn in sorted(gefunden - benannt)))
    assert not benannt - gefunden, (
        f"WOERTLICH_NONE nennt Stellen, die es nicht (mehr) gibt: {sorted(benannt - gefunden)} — "
        f"toter Eintrag, tauescht eine Abdeckung vor, die es nicht gibt.")
    for schluessel, grund in WOERTLICH_NONE.items():
        assert len(grund) > 20, f"{schluessel}: Ausnahme ohne (ausreichende) Begründung"
