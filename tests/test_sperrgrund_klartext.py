"""Sperrgrund-Gate: jeder Sperrgrund hat einen Satz, den ein Laie lesen kann.

Anlass 2026-08-27: ein vollstaendig ausgefuellter Fragebogen (134 Antworten, 0 offene Fragen)
endete mit `grund="flag_konsistenz_offen", offen=[]`. Der Nutzer sah eine Maschinen-Kennung und
eine leere Liste. _an_gesamt_sperrgrund kann 36 solcher Kennungen zurueckgeben; SPERRGRUND_KLARTEXT
gibt jeder eine Stimme nach aussen.

Der Kern dieses Gates ist die QUELLE der Pruefliste: die Gruende werden per AST aus dem Quelltext
gelesen, nicht von Hand gepflegt. Eine zweite, handgepflegte Liste liefe von der ersten weg —
dieselbe Bauart ist in diesem Repo schon mehrfach auseinandergelaufen (zwei Repraesentationen,
ungetestete Uebergabe). So faellt der naechste neue Grund ohne Klartext SOFORT auf, ohne dass
jemand daran denken muss.

DREI Listen fuehren dieselben Gruende, und dieses Gate haelt sie zusammen:
  (1) DER CODE — die einzige QUELLE. _an_gesamt_sperrgrund (bescheid_deklaration.py) und die
      grund-Werte, die _ergebnis_roh (api.py) selbst setzt. Was hier steht, kann eintreten.
  (2) SPERRGRUND_KLARTEXT — ein Abbild: zu jedem Grund ein Satz fuer den Nutzer.
  (3) das enum auf `grund` in api_schema/ergebnis.json — ein Abbild, das aber zur Laufzeit BINDET.
`ergebnis()` uebersetzt JEDEN grund ausser None/"bestaetigt", ohne zu unterscheiden, woher er
stammt; das Schema validiert JEDE Antwort, ohne zu wissen, was der Code kann.

Jede der drei Listen ist schon einmal von den anderen weggelaufen, alle drei binnen eines Tages
gefunden:
  - Gate las nur _an_gesamt_sperrgrund -> `input_kegel_nicht_bestaetigt` stand mit 33 benannten
    offenen Feldern auf dem Schirm, daneben der Ersatztext "laesst sich nicht in Worte fassen".
  - enum kannte behinderungsbedingte_aufwendungen_wahlrecht_partner_offen nicht, obwohl der Code
    ihn liefern kann -> die Antwort waere in dem Moment schema-widrig gewesen. Niemand merkte es,
    weil kein Test diesen Pfad gegen das Schema validierte.
  - enum fuehrt(e) vier Werte, die kein Code-Pfad mehr erzeugt -> eine Aufzaehlung, die wie ein
    Vertrag aussieht und teils Archiv ist. Sie hat beim Abgleich in die Irre gefuehrt.

Geprueft wird deshalb in beide Richtungen zwischen allen dreien:
  (a) jeder Grund aus dem Code hat einen Klartext      -> kein Maschinenstring auf dem Schirm
  (b) jeder Klartext gehoert zu einem echten Grund     -> kein toter Text
  (c) jeder Grund aus dem Code steht im enum           -> keine schema-widrige Antwort
  (d) jeder enum-Wert ist erzeugbar                    -> keine tote Aufzaehlung (s. TOTE_ENUM_WERTE)

Dazu Formpruefungen ueber ALLE Texte (keine Feld-Kennungen, keine Paragraphen, Mindestlaenge),
der Ersatztext fuer einen unbekannten Grund, und Waechter dagegen, dass eine der Extraktionen
mangels Fundstellen still gruen wird.

NICHT abgedeckt, bewusst — beides an team-lead gemeldet:
  - api.einreichen() setzt neun weitere grund-Werte (eigenes Vokabular, u. a. xml_nicht_baubar,
    eric_nicht_verfuegbar) und reicht in einem Fall sogar denselben Sperr-Guard-Wert durch. Diese
    Antworten laufen NICHT durch die Klartext-Zeile in `ergebnis()`. Dieses Gate haengt an der
    Verdrahtung, nicht an jedem grund-Literal im Repo; sonst forderte es Texte fuer Antworten, die
    sie nie ausliefern.
  - app.js fuehrt in `GUARD` eine VIERTE Liste derselben Gruende (18 Laientexte, darunter drei der
    vier toten enum-Werte). Sie ist seit der Klartext-Verdrahtung entbehrlich — die Antwort bringt
    den Satz jetzt mit. Sie hier mitzupruefen hiesse, eine Liste festzuschreiben, die verschwinden
    sollte.
"""

from __future__ import annotations

import ast
import json
import os
import pathlib
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/unsicherheit",
             "produkt/mapping", "produkt/konsistenz", "produkt/import", "produkt/engine",
             "produkt/bescheid", "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import api as API  # noqa: E402  (nur als Wegweiser auf api.py — der Test liest den Quelltext)
import bescheid_deklaration as BD  # noqa: E402


def _funktion(quelle: pathlib.Path, name: str) -> ast.FunctionDef:
    """Die FunctionDef `name` aus `quelle`, oder ein sprechender Fehlschlag.

    Nie None zurueckgeben: ein Gate, das seine Fundstelle nicht mehr findet, muss ROT werden und
    nicht mangels Kandidaten gruen. Der Pfad kommt bei beiden Quellen aus `__file__` des schon
    importierten Moduls, nicht aus einer festen Zeichenkette — sonst liest der Test im Zweifel eine
    andere Datei als die, die zur Laufzeit gilt.
    """
    baum = ast.parse(quelle.read_text(encoding="utf-8"), filename=str(quelle))
    for knoten in ast.walk(baum):
        if isinstance(knoten, (ast.FunctionDef, ast.AsyncFunctionDef)) and knoten.name == name:
            return knoten
    raise AssertionError(
        f"{name} nicht in {quelle} gefunden — umbenannt oder verschoben? Dieses Gate prueft dann "
        "nichts mehr und muss nachgezogen werden.")


def _konstanten(knoten: ast.AST) -> set[str]:
    """Alle String-Konstanten in einem Ausdruck — deckt `"x"`, `a if b else "x"`, `a or "x"` ab."""
    return {n.value for n in ast.walk(knoten)
            if isinstance(n, ast.Constant) and isinstance(n.value, str)}


# ---------------------------------------------------------------- Quelle 1: der Sperr-Guard

def _gruende_aus_sperrgrund_guard() -> set[str]:
    """Die Rueckgabe-Strings von _an_gesamt_sperrgrund.

    ast.walk steigt auch in die inneren Funktionen ab (_dhf_vpf_grund und die uebrigen Helfer) —
    deren `return "..."` sind ebenso Gruende, die beim Aufrufer landen.
    """
    fn = _funktion(pathlib.Path(BD.__file__), "_an_gesamt_sperrgrund")
    return {n.value.value for n in ast.walk(fn)
            if isinstance(n, ast.Return) and isinstance(n.value, ast.Constant)
            and isinstance(n.value.value, str)}


# ---------------------------------------------------------------- Quelle 2: api._ergebnis_roh

def _gruende_aus_ergebnis_roh() -> set[str]:
    """Die `"grund"`-Werte, die _ergebnis_roh (api.py) SELBST setzt.

    Diese drei laufen nie durch den Sperr-Guard, landen aber in derselben Antwort und damit in
    derselben Klartext-Zeile in `ergebnis()`. Ohne sie prueft dieses Gate nur die halbe Wahrheit —
    genau die Klasse Luecke, gegen die es gebaut ist (zwei Quellen, eine geprueft).

    Zwei Bauarten kommen vor, und beide werden aufgeloest:
      `"grund": "kein_scheiben_gesamtbescheid"`  — Konstante direkt im Dict
      `"grund": grund` nach `grund = "a" if ... else "b"`  — Variable, im Rumpf zugewiesen

    FAIL-CLOSED: taucht eine dritte Bauart auf — eine Variable, die weder im Rumpf mit Konstanten
    belegt noch erkennbar das Ergebnis von _an_gesamt_sperrgrund ist (dann deckt Quelle 1 sie ab) —
    wird der Test ROT statt sie stillschweigend zu uebergehen. Ein uebersehener Grund ist genau der
    Maschinenstring auf dem Schirm, den diese Zuordnung beenden soll.
    """
    fn = _funktion(pathlib.Path(API.__file__), "_ergebnis_roh")

    # Welche Variablen werden im Rumpf womit belegt?
    belegt: dict[str, list[ast.AST]] = {}
    for n in ast.walk(fn):
        if isinstance(n, ast.Assign):
            for ziel in n.targets:
                if isinstance(ziel, ast.Name):
                    belegt.setdefault(ziel.id, []).append(n.value)

    def _ist_guard_aufruf(wert: ast.AST) -> bool:
        return any(isinstance(k, ast.Call) and isinstance(k.func, ast.Name)
                   and k.func.id == "_an_gesamt_sperrgrund" for k in ast.walk(wert))

    gruende: set[str] = set()
    for n in ast.walk(fn):
        if not isinstance(n, ast.Dict):
            continue
        for schluessel, wert in zip(n.keys, n.values):
            if not (isinstance(schluessel, ast.Constant) and schluessel.value == "grund"):
                continue
            if isinstance(wert, ast.Constant) and isinstance(wert.value, str):
                gruende.add(wert.value)
                continue
            if isinstance(wert, ast.Name):
                zuweisungen = belegt.get(wert.id, [])
                if zuweisungen and all(_ist_guard_aufruf(z) for z in zuweisungen):
                    continue                      # kommt aus dem Sperr-Guard -> Quelle 1
                aufgeloest = {k for z in zuweisungen for k in _konstanten(z)}
                if aufgeloest:
                    gruende |= aufgeloest
                    continue
            raise AssertionError(
                f"_ergebnis_roh (api.py Zeile {wert.lineno}) setzt einen 'grund', den dieses Gate "
                f"nicht aufloesen kann ({ast.dump(wert)[:120]}). Bau die Aufloesung hier nach — "
                "uebergehen hiesse, den naechsten Grund ohne Klartext durchzulassen.")
    return gruende


def _klartext_ausnahmen() -> set[str]:
    """Die grund-Werte, fuer die `ergebnis()` bewusst KEINEN Klartext setzt.

    Gelesen aus der Bedingung selbst (`obj.get("grund") not in (None, "bestaetigt")`) statt hier
    nachgepflegt: "bestaetigt" ist ein regulaerer grund-Wert von _ergebnis_roh, nur eben einer, der
    kein Problem meldet. Haette dieser Test seine eigene Ausnahmeliste, liefe sie von der echten
    Bedingung weg — dieselbe Zwei-Listen-Bauart, gegen die das ganze Gate gebaut ist.
    """
    fn = _funktion(pathlib.Path(API.__file__), "ergebnis")
    for n in ast.walk(fn):
        if not isinstance(n, ast.Compare) or not n.ops:
            continue
        if not isinstance(n.ops[0], (ast.NotIn, ast.In)):
            continue
        if "grund" not in _konstanten(n.left):
            continue                              # nicht der Vergleich auf obj["grund"]
        return {k for v in n.comparators for k in _konstanten(v)}
    raise AssertionError(
        "In api.ergebnis() steht kein grund-Vergleich mehr, aus dem sich die Klartext-Ausnahmen "
        "lesen liessen. Die Verdrahtung wurde umgebaut — dieses Gate muss nachgezogen werden.")


def _erwartete_gruende() -> set[str]:
    """Beide Code-Quellen zusammen, ohne die, fuer die bewusst kein Klartext gesetzt wird."""
    return (_gruende_aus_sperrgrund_guard() | _gruende_aus_ergebnis_roh()) - _klartext_ausnahmen()


# ---------------------------------------------------------------- Quelle 3: das Schema-enum

SCHEMA_PFAD = pathlib.Path(ROOT) / "produkt" / "haut" / "api_schema" / "ergebnis.json"

# Enum-Werte, die KEIN Code-Pfad mehr erzeugt. Nachrecherchiert am 2026-08-27, repo-weit ueber alle
# Dateitypen: kein einziger Produzent. Zwei davon sagen es im Quelltext selbst —
# bescheid_deklaration.py Z. 1034-1037 ("erstattungsueberhang_offen bleibt im Schema-Enum als
# Alt-Grund erhalten, feuert aber nicht mehr") und Z. 858-860 (dasselbe fuer partner_vorsorge_offen,
# nachdem die Person-B-Vorsorge additiv verdrahtet wurde). Die anderen zwei stammen aus dem
# an_gesamt-Zuschnitt, dessen Guards laengst in _dhf_vpf_grund aufgegangen sind. Unabhaengig
# bestaetigt vom Audit .audit/2026-08-16 ("four enum values have no producer anywhere").
#
# Sie stehen hier als AUSNAHME und nicht als Verstoss, weil das Streichen team-lead gehoert
# (produkt/haut/api_schema/ergebnis.json ist seine Datei) und weil ein rotes Gate fuer eine
# Altlast, die niemand heute verursacht hat, nur Laerm waere. Diese Liste darf NUR SCHRUMPFEN:
# der Waechter darunter wird rot, sobald ein Name hier steht, den das enum nicht mehr fuehrt.
TOTE_ENUM_WERTE = frozenset({
    "erstattungsueberhang_offen",
    "partner_vorsorge_offen",
    "sonderausgaben_nicht_ring_faehig",
    "werbungskosten_nicht_ring_faehig",
})


def _enum_aus_schema() -> set[str]:
    """Die zulaessigen grund-Werte aus api_schema/ergebnis.json.

    Dritte Liste derselben Gruende, von Hand gepflegt — und die gefaehrlichste, weil sie zur
    Laufzeit BINDET: das Schema hat additionalProperties:false, ein Wert ausserhalb des enums macht
    die Antwort schema-widrig. Genau so ist
    behinderungsbedingte_aufwendungen_wahlrecht_partner_offen durchgerutscht: der Code konnte ihn
    liefern, das enum kannte ihn nicht, und kein Test validierte diesen Pfad je gegen das Schema.
    """
    schema = json.loads(SCHEMA_PFAD.read_text(encoding="utf-8"))
    try:
        return set(schema["properties"]["grund"]["enum"])
    except KeyError as e:
        raise AssertionError(
            f"{SCHEMA_PFAD} fuehrt kein enum unter properties.grund mehr ({e}) — Schema umgebaut? "
            "Dieses Gate prueft dann nichts mehr und muss nachgezogen werden.") from e


# ---------------------------------------------------------------- (0) Der Prueflauf selbst

def test_ast_extraktion_findet_wirklich_gruende():
    """Der Prueflauf darf nicht mangels Fundstellen gruen sein.

    Ohne diesen Test waeren (a) und (b) bei einer leeren Extraktion still gruen — ein Gate, das
    seine eigene Blindheit als Erfolg meldet. Die Untergrenzen sind bewusst grob (30 bzw. 2 statt
    exakt 36 bzw. 3): sie fangen den Totalausfall EINER der beiden Quellen, ohne bei jedem neuen
    oder entfallenen Grund rot zu werden.
    """
    guard = _gruende_aus_sperrgrund_guard()
    roh = _gruende_aus_ergebnis_roh()
    assert len(guard) >= 30, (
        f"nur {len(guard)} Gruende aus _an_gesamt_sperrgrund gelesen — die AST-Extraktion greift "
        f"nicht mehr: {sorted(guard)}")
    assert len(roh) >= 2, (
        f"nur {len(roh)} Gruende aus _ergebnis_roh gelesen — die AST-Extraktion greift nicht "
        f"mehr: {sorted(roh)}")


def test_beide_quellen_sind_verschieden():
    """Beide Extraktionen muessen wirklich verschiedene Stellen lesen.

    Zeigten sie versehentlich auf dieselbe Funktion (etwa nach einem Umzug), waere die Vereinigung
    unauffaellig korrekt und die zweite Quelle trotzdem ungeprueft.
    """
    nur_roh = _gruende_aus_ergebnis_roh() - _gruende_aus_sperrgrund_guard()
    assert nur_roh, (
        "_ergebnis_roh liefert keinen einzigen Grund, den der Sperr-Guard nicht schon liefert — "
        "lesen beide Extraktionen dieselbe Stelle?")


# ---------------------------------------------------------------- (a)+(b) Beide Richtungen

def test_jeder_grund_hat_klartext():
    """Kein Grund ohne Satz — sonst steht die Kennung auf dem Schirm des Nutzers.

    Ueber BEIDE Quellen: der Sperr-Guard und das, was _ergebnis_roh selbst setzt.
    """
    ohne_text = sorted(_erwartete_gruende() - set(BD.SPERRGRUND_KLARTEXT))
    assert not ohne_text, (
        "Gruende ohne Klartext (der Nutzer saehe die rohe Kennung): " + ", ".join(ohne_text))


def test_kein_toter_klartext():
    """Kein Eintrag, den es als Grund gar nicht gibt — sonst sammelt sich toter Text an."""
    tot = sorted(set(BD.SPERRGRUND_KLARTEXT) - _erwartete_gruende())
    assert not tot, (
        "Klartext-Eintraege ohne zugehoerigen Grund (tot, oder Tippfehler im Schluessel): "
        + ", ".join(tot))


def test_was_der_code_liefert_steht_im_schema():
    """Der scharfe Teil: ein grund ausserhalb des enums macht die Antwort schema-widrig.

    `ergebnis.json` hat additionalProperties:false und ein enum auf `grund` — kann der Code einen
    Wert liefern, den das enum nicht kennt, ist die /ergebnis-Antwort in genau dem Moment
    ungueltig, in dem dieser Zustand eintritt. Am 2026-08-27 war das
    behinderungsbedingte_aufwendungen_wahlrecht_partner_offen: seit Wochen im Code, nie im enum,
    und kein Test lief je gegen diesen Pfad.
    """
    fehlt = sorted((_gruende_aus_sperrgrund_guard() | _gruende_aus_ergebnis_roh())
                   - _enum_aus_schema())
    assert not fehlt, (
        "Der Code kann diese grund-Werte liefern, das Schema-enum kennt sie nicht — die "
        "/ergebnis-Antwort waere in dem Moment schema-widrig: " + ", ".join(fehlt))


def test_kein_toter_enum_wert():
    """Die Gegenrichtung: eine Aufzaehlung, die niemand mehr erzeugt, legt den naechsten Leser rein.

    Sie sieht aus wie ein Vertrag ueber moegliche Zustaende, ist aber teils Archiv. Die vier
    bekannten Altlasten stehen in TOTE_ENUM_WERTE und sind dort begruendet; ein FUENFTER faellt
    hier auf.
    """
    tot = sorted(_enum_aus_schema() - (_gruende_aus_sperrgrund_guard()
                                       | _gruende_aus_ergebnis_roh()) - TOTE_ENUM_WERTE)
    assert not tot, (
        "enum-Werte, die kein Code-Pfad erzeugt (tote Aufzaehlung): " + ", ".join(tot)
        + " — entweder streichen oder, mit Begruendung, in TOTE_ENUM_WERTE aufnehmen.")


def test_keine_toten_eintraege_in_der_ausnahmeliste():
    """Waechter ueber die Ausnahmeliste selbst: sie darf nur schrumpfen.

    Ohne ihn ueberlebt ein Name in TOTE_ENUM_WERTE das Streichen im Schema und deckt spaeter still
    einen NEUEN toten Wert desselben Namens. Eine Ausnahmeliste ohne Verfallsdatum ist ein
    Freibrief — dieselbe Bauart wie AUSNAHMEN in test_zweig_duplikation_differential.py.
    """
    verwaist = sorted(TOTE_ENUM_WERTE - _enum_aus_schema())
    assert not verwaist, (
        "TOTE_ENUM_WERTE fuehrt Namen, die das Schema gar nicht mehr kennt: "
        + ", ".join(verwaist) + " — hier streichen, die Altlast ist erledigt.")


def test_tote_enum_werte_haben_keinen_klartext():
    """Was der Code nicht liefern kann, braucht auch keinen Satz — sonst waere es toter Text."""
    ueberfluessig = sorted(TOTE_ENUM_WERTE & set(BD.SPERRGRUND_KLARTEXT))
    assert not ueberfluessig, (
        "Klartext fuer einen grund, den kein Code-Pfad erzeugt: " + ", ".join(ueberfluessig))


def test_ausnahmen_bekommen_keinen_klartext():
    """Was `ergebnis()` bewusst ueberspringt, darf hier auch nicht stehen.

    "bestaetigt" ist ein regulaerer grund-Wert, aber kein Problem — ein Klartext dazu waere toter
    Text, den niemand je zu sehen bekaeme.
    """
    ueberfluessig = sorted(_klartext_ausnahmen() & set(BD.SPERRGRUND_KLARTEXT))
    assert not ueberfluessig, (
        "Klartext fuer einen Grund, den ergebnis() gar nicht uebersetzt: "
        + ", ".join(ueberfluessig))


# ---------------------------------------------------------------- (c) Form aller Texte

def test_texte_sind_nicht_leer():
    leer = sorted(g for g, t in BD.SPERRGRUND_KLARTEXT.items() if not t or not t.strip())
    assert not leer, "leerer Klartext bei: " + ", ".join(leer)


def test_texte_enthalten_keine_feld_kennungen():
    """Unterstriche verraten eine kopierte Feld-Kennung oder Regel-Id (vpf_monate_am_ort).

    Der Nutzer hat diese Namen nie gesehen — sie erklaeren ihm nichts und sehen aus wie ein Defekt.
    """
    mit_kennung = sorted(g for g, t in BD.SPERRGRUND_KLARTEXT.items() if "_" in t)
    assert not mit_kennung, (
        "Klartext enthaelt einen Unterstrich (Feld-Kennung/Regel-Id?) bei: "
        + ", ".join(mit_kennung))


def test_texte_enthalten_keine_paragraphen():
    """Paragraphen-Kuerzel ohne Auflösung sagen einem Laien nichts.

    Die Norm gehoert in den Satz uebersetzt ("der Freibetrag fuer den Betriebsverkauf"), nicht
    zitiert. Das Zeichen ganz zu verbieten ist die Regel, die sich pruefen laesst.
    """
    mit_para = sorted(g for g, t in BD.SPERRGRUND_KLARTEXT.items() if "§" in t)
    assert not mit_para, "Klartext zitiert einen Paragraphen bei: " + ", ".join(mit_para)


def test_texte_sind_ausformuliert():
    """Mindestlaenge: ein Stichwort ist keine Erklaerung.

    Ein Satz muss sagen, WAS die Software nicht entscheiden kann UND WAS der Nutzer tun soll —
    das geht nicht in fuenf Woertern.
    """
    zu_kurz = sorted(f"{g} ({len(t)} Zeichen)"
                     for g, t in BD.SPERRGRUND_KLARTEXT.items() if len(t.strip()) < 80)
    assert not zu_kurz, "Klartext zu kurz fuer eine Erklaerung bei: " + ", ".join(zu_kurz)


def test_texte_beginnen_als_satz():
    """Keine reine Kleinschrift: der Text ist Prosa, kein weiterer Maschinenstring."""
    klein = sorted(g for g, t in BD.SPERRGRUND_KLARTEXT.items()
                   if t == t.lower() or not t.strip()[0].isupper())
    assert not klein, "Klartext beginnt nicht als Satz bei: " + ", ".join(klein)


# ---------------------------------------------------------------- (d) Die Funktion

def test_bekannter_grund_liefert_seinen_text():
    for grund, text in BD.SPERRGRUND_KLARTEXT.items():
        assert BD.sperrgrund_klartext(grund) == text, grund


def test_unbekannter_grund_liefert_ehrlichen_satz():
    """Ein unbekannter Grund darf weder None noch die rohe Kennung ergeben.

    Das Gate oben faengt neue Sperrgruende im Test. Kommt trotzdem einer durch — etwa ein Grund
    aus einer anderen Quelle als dieser Funktion — ist ein ehrlicher Satz besser als ein
    Maschinenstring auf dem Schirm.
    """
    text = BD.sperrgrund_klartext("voellig_neuer_grund_ohne_text")
    assert text is not None
    assert text != "voellig_neuer_grund_ohne_text"
    assert "_" not in text
    assert len(text) >= 80


def test_keine_sperre_bleibt_none():
    """None heisst: es gibt keine Sperre. Dann gibt es auch nichts zu erklaeren."""
    assert BD.sperrgrund_klartext(None) is None
