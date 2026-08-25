"""Die Texte, die der Nutzer LIEST, müssen lesbar sein — Umlaute und keine internen Kennungen.

ANLASS, gemessen 2026-08-24 an Julius' echtem Durchgang. Im Screenshot der Rückfrage stand als
Hilfetext:

    „Diese Leistungen sind steuerfrei, erhoehen aber den Steuersatz auf dein uebriges Einkommen
     (Progressionsvorbehalt). … NUR falls dein Sozialversicherungstraeger die Daten NICHT
     elektronisch gemeldet hat."

Drei kaputte Wörter in dem einen Text, den der Nutzer lesen MUSS, um antworten zu können. Kein
Rechenfehler — aber die Software wirkt unfertig genau an der Stelle, an der sie Recht erklärt.

Ein zweiter Fall derselben Art, im selben Durchgang gefunden: „Wie kist_konfession, aber für den
Ehe-/Lebenspartner." Eine Feld-Kennung mitten in einem Satz an den Laien, der sie nie gesehen hat.

WARUM EINE LISTE UND KEIN MUSTER. Von 132 Wörtern mit einer `ae/oe/ue/ss`-Folge in den
Anzeigetexten sind nur 15 echte Umschrift. Der Rest ist korrektes Deutsch: „Steuer", „dass",
„Zuschuss", „Kasse", „individuell", „Lohnsteuerbescheinigung". Ein Muster hätte aus „Steuer"
„Stäuer" gemacht — deshalb steht hier eine geprüfte Liste, und daneben ein zweites, datengetriebenes
Netz für Fälle, die noch niemand aufgeschrieben hat.
"""
from __future__ import annotations

import itertools
import pathlib
import re

BINDUNG = pathlib.Path(__file__).resolve().parent.parent / "produkt" / "bindung"

# Nur was der Nutzer liest. feld_id/enum_werte/beispielwert/regel_id/anker_ref sind WERTE — dort
# wäre eine geänderte Schreibweise ein Datenfehler, kein Textfehler.
ANZEIGE = ("fragetext_laie", "hilfe_kurz", "frage_laie", "titel", "erklaerung")

# Die am 2026-08-24 gefundenen und behobenen. Rückfall-Schutz.
VERBOTEN = {
    "Haelftelung": "Hälftelung", "Massnahme": "Maßnahme",
    "Sozialversicherungstraeger": "Sozialversicherungsträger", "Verhaeltnis": "Verhältnis",
    "Zwoelftelung": "Zwölftelung", "auswaertiger": "auswärtiger", "erhoehen": "erhöhen",
    "fuer": "für", "ganzjaehriger": "ganzjähriger", "heisst": "heißt", "noetig": "nötig",
    "ueber": "über", "uebriges": "übriges", "volljaehrigen": "volljährigen",
    "zustaendig": "zuständig", "zaahlen": "zählen",
}


def _anzeigetexte() -> list[tuple[str, int, str, str]]:
    """(datei, zeile, schlüssel, text) für jeden Text, den ein Nutzer zu sehen bekommt."""
    out = []
    for p in sorted(BINDUNG.glob("*.yaml")):
        for i, z in enumerate(p.read_text(encoding="utf-8").splitlines(), 1):
            m = re.match(r"\s*(\w+):\s*(.*)$", z)
            if m and m.group(1) in ANZEIGE and m.group(2).strip():
                out.append((p.name, i, m.group(1), m.group(2)))
    return out


def test_ueberhaupt_texte_gefunden():
    """Untergrenze gegen den stillen Leerlauf: griffe der Sammler ins Leere — anderer Pfad,
    umbenannte Schlüssel —, wären alle Prüfungen hier grün und keine davon hätte etwas gemessen."""
    texte = _anzeigetexte()
    assert len(texte) > 400, f"Nur {len(texte)} Anzeigetexte gefunden — Sammler prüfen."


def test_keine_ascii_umschrift_in_anzeigetexten():
    """Die geprüfte Liste. Jedes dieser Wörter stand am 2026-08-24 in einem Text für den Nutzer."""
    treffer = []
    for datei, zeile, schluessel, text in _anzeigetexte():
        for falsch, richtig in VERBOTEN.items():
            if re.search(rf"\b{re.escape(falsch)}\b", text):
                treffer.append(f"{datei}:{zeile} ({schluessel}): {falsch!r} statt {richtig!r}")
    assert not treffer, (
        "ASCII-Umschrift in Texten, die der Nutzer liest:\n  " + "\n  ".join(treffer))


def test_keine_neue_umschrift_deren_richtige_form_es_schon_gibt():
    """Das zweite Netz, und es braucht keine Pflege: steht irgendwo im Korpus „berücksichtigen"
    und woanders „beruecksichtigen", ist das zweite eine Umschrift — belegt durch das erste.

    Fängt Fälle, die in der Liste oben (noch) nicht stehen. Fängt NICHT alles: „übriges" kam nur
    in der falschen Schreibweise vor und wäre hier durchgerutscht. Zwei Netze, weil keines von
    beiden allein reicht."""
    korpus = " ".join(p.read_text(encoding="utf-8") for p in sorted(BINDUNG.glob("*.yaml")))
    bekannt = set(re.findall(r"\b\w+\b", korpus, re.UNICODE))
    um = {"ae": "ä", "oe": "ö", "ue": "ü", "ss": "ß", "Ae": "Ä", "Oe": "Ö", "Ue": "Ü"}

    def mit_umlaut(w):
        stellen = [m.start() for m in re.finditer(r"ae|oe|ue|ss|Ae|Oe|Ue", w)]
        if not stellen or len(stellen) > 4:
            return []
        for wahl in itertools.product([0, 1], repeat=len(stellen)):
            if not any(wahl):
                continue
            teile, letzte = [], 0
            for s, nimm in zip(stellen, wahl):
                teile.append(w[letzte:s])
                teile.append(um[w[s:s + 2]] if nimm else w[s:s + 2])
                letzte = s + 2
            teile.append(w[letzte:])
            yield "".join(teile)

    treffer = []
    for datei, zeile, schluessel, text in _anzeigetexte():
        for w in re.findall(r"\b\w+\b", text):
            for v in mit_umlaut(w):
                if v in bekannt and v != w:
                    treffer.append(f"{datei}:{zeile} ({schluessel}): {w!r} — {v!r} steht anderswo")
                    break
    assert not treffer, (
        "Wörter, die anderswo im Korpus mit Umlaut geschrieben werden:\n  " + "\n  ".join(treffer))


def test_keine_feld_kennung_im_laientext():
    """Zweite Fehlerklasse, im selben Durchgang gefunden: „Wie kist_konfession, aber für den
    Ehe-/Lebenspartner." Die Kennung ist ein internes Wort — der Laie hat sie nie gesehen und kann
    aus ihr nichts ableiten.

    Geprüft wird gegen die TATSÄCHLICHEN Feld-IDs der Bindung, nicht gegen ein Unterstrich-Muster:
    ein Text darf „TT.MM-TT.MM" oder „Anlage_N" enthalten, ohne dass das eine Kennung wäre.

    UND: exakt, also unter Beachtung der Gross-/Kleinschreibung. Der erste Anlauf verlangte einen
    Unterstrich im Wort — eine Mutationsprobe schrieb daraufhin „Siehe bruttoarbeitslohn." in einen
    Hilfetext und der Test blieb GRÜN. `bruttoarbeitslohn` hat keinen Unterstrich.

    Die Gross-/Kleinschreibung trägt die Unterscheidung sauberer: „Wie hoch war dein
    Bruttoarbeitslohn?" ist ein deutsches Substantiv, `bruttoarbeitslohn` mitten im Satz ist die
    Kennung. Fällt ein Treffer doch mal anders aus, ist er ein Gross-/Kleinschreibfehler — und
    gehört ebenfalls gemeldet."""
    korpus = " ".join(p.read_text(encoding="utf-8") for p in sorted(BINDUNG.glob("*.yaml")))
    ids = set(re.findall(r"^\s*-?\s*feld_id:\s*['\"]?(\w+)", korpus, re.M))
    assert len(ids) > 100, f"Nur {len(ids)} feld_ids gefunden — Sammler prüfen."

    treffer = []
    for datei, zeile, schluessel, text in _anzeigetexte():
        for w in re.findall(r"\b\w+\b", text):
            if w in ids:
                treffer.append(f"{datei}:{zeile} ({schluessel}): Feld-Kennung {w!r}")
    assert not treffer, (
        "Interne Feld-Kennungen in Texten für den Laien:\n  " + "\n  ".join(treffer))
