"""Wer im „Verstanden"-Panel auf „Stimmt" klickt, bestätigt ungefragt — hier prüft niemand,
ob die Frage gerade dran ist.

Gemessen 2026-08-30: app.js hat zwei Wege, ein KI-vorgeschlagenes Feld zu bestätigen.
`starteRueckfragen()` (Z.1576-1627) holt vorher `/fall/${FALL}/fragen`, baut daraus einen
`katalog` und filtert die Rückfragen dagegen — was der Fragebogen gerade nicht mehr anbietet,
`entfallen` diese, und der Nutzer erfährt "hat sich durch deine Bestätigung erledigt".
`verstandenBestaetigen()` (Z.1412-1435), der zweite Bestätigen-Knopf im selben Fragebogen (fürs
"Verstanden"-Panel der freien Chat-Vorschläge), postet direkt an `/event` — ohne Aufruf von
`/fragen`, ohne Katalog, an keiner Stelle im Funktionskörper. Serverseitig bestätigt
api.py::event() (Z.514-522) denselben Sachverhalt von der anderen Seite: schreiber="ui:laie"
(beide Wege benutzen ihn) fällt nicht unter `_vorschlag` — nur llm:/berechnet:/import:beleg/
import:kontoauszug tun das —, also katalog=None, keine Reihenfolge-Prüfung durch
ST.append_event.

WICHTIG, das hier ist NICHT der Defekt: der freie Chat-Pfad existiert, damit der Nutzer erzählen
kann, statt sich durch den Fragebogen zu klicken — ausser der Reihe zu antworten ist dort Zweck,
nicht Versehen. Wer diese Asymmetrie "behebt", indem er in verstandenBestaetigen() einen Filter
gegen /fragen einbaut, zwingt dem freien Gespräch die Reihenfolge des Fragebogens auf und macht
damit das Merkmal kaputt, das dieser Weg hat.

Was tatsächlich fehlt, ist die Behandlung der FOLGE, nicht die Asymmetrie selbst: bestätigt der
Nutzer z.B. `kein_kap_partner=True`, bevor die fünf Felder gefragt wurden, die es gatet, nimmt
der Traverser diese fünf per `feld_bedingung` aus der Warteschlange — sie werden nie gefragt,
während bescheid_deklaration.py weiter `bestaetigt` von ihnen verlangt (Sperrgrund
`partner_kegel_offen`, api.py::_an_gesamt_sperrgrund). Das trifft jeden Weg, der ein Kreuz früh
setzt, nicht nur den Chat — ein anderer Worker baut dazu gerade die systematische Messung. Ob
die Lösung im Chat-Pfad, in der Bindung oder im Sperrgrund liegt, ist offen und liegt bei Julius.

Keine Sackgasse zwischendurch: ein Korrektur-Event (`ersetzt=<event_id>`, wie
app.js::korrigiereBestaetigt es baut) gibt gegatete Felder sofort wieder frei — dieselbe
Mechanik, die test_store.py::test_c_ersetzt_aufloesung allgemein abdeckt, hier nur am Rande
erwähnt, nicht erneut gemessen.

Rein strukturell (Text-Extraktion, kein Fragebogen-Lauf nötig) — braucht weder Catala noch
gettsim.
"""
from __future__ import annotations

import os

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
APP_JS = os.path.join(ROOT, "produkt/haut/static/app.js")
API_PY = os.path.join(ROOT, "produkt/haut/api.py")

START_RUECKFRAGEN = "async function starteRueckfragen("
END_RUECKFRAGEN = "function zeigeRueckfrage("
START_VERSTANDEN = "async function verstandenBestaetigen("
END_VERSTANDEN = "async function verstandenAendern("
START_EVENT = "def event(fall_id: str, body: dict) -> tuple[int, dict]:"
END_EVENT = "def warum(fall_id: str, feld_id: str) -> tuple[int, dict]:"


def _lies(pfad: str) -> str:
    with open(pfad, encoding="utf-8") as f:
        return f.read()


def _funktionskoerper(text: str, start_anker: str, end_anker: str) -> str:
    start = text.index(start_anker)
    ende = text.index(end_anker, start)
    assert ende > start, f"{end_anker!r} liegt vor {start_anker!r} -- falsche Ankerreihenfolge."
    return text[start:ende]


def test_anker_werden_gefunden():
    """Blindheitswaechter: alle drei Anker-Paare existieren noch, in der erwarteten Reihenfolge,
    und liefern nicht-triviale Koerper. Ohne diesen Test wuerde eine umbenannte Funktion die
    beiden Tests unten stumm auf leeren Text laufen lassen -- beide waeren dann sinnlos gruen."""
    app_js = _lies(APP_JS)
    api_py = _lies(API_PY)
    rueckfragen = _funktionskoerper(app_js, START_RUECKFRAGEN, END_RUECKFRAGEN)
    verstanden = _funktionskoerper(app_js, START_VERSTANDEN, END_VERSTANDEN)
    event = _funktionskoerper(api_py, START_EVENT, END_EVENT)
    assert len(rueckfragen) > 200, "starteRueckfragen()-Koerper verdaechtig kurz"
    assert len(verstanden) > 200, "verstandenBestaetigen()-Koerper verdaechtig kurz"
    assert len(event) > 500, "event()-Koerper verdaechtig kurz"


def test_starteRueckfragen_prueft_gegen_fragen_katalog():
    """Kontrolle: dieser Weg prueft nachweislich -- ohne ihn haette der Befund unten keine
    Vergleichsbasis, waere also nicht aussagekraeftig, sondern zufaellig."""
    body = _funktionskoerper(_lies(APP_JS), START_RUECKFRAGEN, END_RUECKFRAGEN)
    assert "/fragen" in body and "katalog" in body


def test_verstandenBestaetigen_bestaetigt_ohne_katalog_pruefung():
    """Befund (s. Modul-Docstring): dieser Weg bestaetigt ungefragt -- kein /fragen-Aufruf, kein
    Katalog, an keiner Stelle im Funktionskoerper. Das ist heute so gebaut und gewollt fuer den
    freien Chat -- also gruen, nicht rot: dieser Test haelt den gemessenen Zustand fest, er
    fordert keine Reparatur."""
    body = _funktionskoerper(_lies(APP_JS), START_VERSTANDEN, END_VERSTANDEN)
    assert "/fragen" not in body
    assert "katalog" not in body


def test_api_event_schliesst_ui_laie_von_der_katalog_pflicht_aus():
    """Serverseitige Haelfte desselben Befunds: schreiber="ui:laie" (beide app.js-Wege oben
    benutzen ihn) faellt nicht unter _vorschlag -- also katalog=None, keine Reihenfolge-Pruefung
    durch ST.append_event."""
    body = _funktionskoerper(_lies(API_PY), START_EVENT, END_EVENT)
    vorschlag_zeile = (
        'schreiber.startswith(("llm:", "berechnet:", "import:beleg", "import:kontoauszug")))')
    assert vorschlag_zeile in body, (
        "Die _vorschlag-Klassifikation hat sich geaendert -- dieser Test prueft dann die "
        "falsche Zeile.")
    assert "ui:" not in vorschlag_zeile
    assert "katalog=(ST.lade_katalog(TR.lade_bindung()) if _vorschlag else None)" in body
