"""Höchstens EINE Rückfrage je Aussage — und was dabei NICHT verloren gehen darf.

ANLASS, gemessen 2026-08-24 im Live-Lauf mit echtem Modell. Der Satz

    „ich bin verheiratet, habe 2 kinder, fuhre 20km mit dem auto zur arbeit.
     vor juli 2025 habe ich 50k pro jahr verdient, seit juli bin ich arbeitslos."

erzeugte **21 Rückfragen**. Frühere Läufe mit demselben Text lagen bei 5 bis 8. Darunter waren

    „Wie heissen die anderen Elternteile deiner Kinder (Vor- und Nachname)?"
    „Wann sind die anderen Elternteile deiner Kinder geboren?"
    „In welchem Zeitraum dieses Jahr bestand das Verhältnis zwischen dir und den Kindern?"

Das ist kein Nachfragen bei Unklarheit mehr, sondern der Fragebogen im Gewand des Gesprächs — und
davon gibt es daneben bereits einen.

URSACHE, und sie stand im Prompt: „gibt der Text zu einem Feld das Thema her, aber nicht den Wert,
dann MUSST du eine RÜCKFRAGE stellen." „Zwei Kinder" gibt JEDEM Kind-Feld sein Thema. Das Modell
hat wörtlich befolgt, was dastand. Der Prompt sagt es jetzt anders — und `_rueckfragen_gebuendelt`
setzt es zusätzlich deterministisch durch, aus demselben Grund wie das Beleg-Gate und
`_rueckfrage_verdraengt`: eine Regel, die nur im Prompt steht, gilt, solange das Modell mag.

WAS DABEI SCHIEFGEHEN KANN, und wogegen die Hälfte dieser Tests steht: Bündeln ist Wegwerfen.
Zwei Arten von stillem Verlust liegen dabei nahe —

  1. Der Vorschlag geht mit. `_rueckfrage_verdraengt` entfernt zu jedem gefragten Feld den
     Vorschlag. Liefe es VOR dem Bündeln, nähme eine Rückfrage, die gleich wegfällt, den Wert zu
     ihrem Feld mit ins Nichts: der Nutzer verlöre die Zahl UND die Frage danach.
  2. Der Nutzer erfährt es nicht. Eine gekürzte Liste sieht aus wie eine vollständige.

KEIN NETZ: `urlopen` ist ersetzt, wie in tests/test_dialog_drei_stufen.py.
"""
from __future__ import annotations

import json
import os
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api_llm                      # noqa: E402
import audit as AUDIT               # noqa: E402
import llm_client as LC             # noqa: E402

KATALOG = [
    {"feld_id": "kind_vorname", "fragetext_laie": "Wie heisst dein Kind?",
     "typ": "text", "regel_id": "p32_kinder"},
    {"feld_id": "kind_geburtsdatum", "fragetext_laie": "Wann ist es geboren?",
     "typ": "datum", "regel_id": "p32_kinder"},
    {"feld_id": "lohnersatz", "fragetext_laie": "Wie viel Lohnersatz?",
     "typ": "cent", "regel_id": "p32b_progression"},
    {"feld_id": "agb_aufwendungen", "fragetext_laie": "Größere außergewöhnliche Ausgaben?",
     "typ": "cent", "regel_id": "p33_agb"},
]
TEXT = "Ich habe 2 Kinder, bin seit Juli arbeitslos und hatte 620 Euro Arztkosten."


# ---------------------------------------------------------------- die Regel selbst

def _rf(frage, feld_id="", aussage=0):
    return {"frage": frage, "feld_id": feld_id, "aussage": aussage}


def test_eine_rueckfrage_je_aussage():
    """Der Kern. Zu einer Aussage gibt es genau eine Unklarheit — die, die sie offen lässt."""
    roh = [_rf("Wie heissen die Kinder?", "kind_vorname", 0),
           _rf("Wann sind sie geboren?", "kind_geburtsdatum", 0),
           _rf("Wie heissen die anderen Elternteile?", "", 0),
           _rf("Wie viel Lohnersatz?", "lohnersatz", 1)]
    behalten, zurueck = api_llm._rueckfragen_gebuendelt(roh)

    assert [r["frage"] for r in behalten] == ["Wie heissen die Kinder?", "Wie viel Lohnersatz?"], (
        f"Nicht genau eine je Aussage: {[r['frage'] for r in behalten]}")
    assert zurueck == 2, f"Falsch gezählt: {zurueck}"


def test_die_erste_je_aussage_gewinnt():
    """Nicht die letzte, nicht eine zufällige: die erste. Das Modell nennt zuerst, was es für am
    wichtigsten hält — und eine Regel ohne festgelegte Reihenfolge ist bei jedem Lauf eine andere."""
    roh = [_rf("zuerst", "a", 3), _rf("danach", "b", 3), _rf("zuletzt", "c", 3)]
    behalten, _ = api_llm._rueckfragen_gebuendelt(roh)
    assert [r["frage"] for r in behalten] == ["zuerst"]


def test_ohne_zuordnung_ist_eine_eigene_gruppe():
    """`aussage: -1` heisst „zu keiner Aussage". Zählte das nicht als eigene Gruppe, käme eine Flut
    heimatloser Fragen ungefiltert durch — und nur die absolute Grenze griffe noch."""
    roh = [_rf("A", "", -1), _rf("B", "", -1), _rf("C", "x", 0)]
    behalten, zurueck = api_llm._rueckfragen_gebuendelt(roh)
    assert [r["frage"] for r in behalten] == ["A", "C"], f"{behalten}"
    assert zurueck == 1


def test_absolute_obergrenze_greift_auch_bei_vielen_aussagen():
    """Die Regel darüber ist die inhaltliche; diese hier fängt den Fall ab, dass Stufe 1 selbst
    viele Aussagen liefert. Eine Einzelgrenze ohne Anzahlgrenze ist in diesem Haus schon dreimal
    aufgelaufen — deshalb gibt es beide."""
    roh = [_rf(f"Frage {i}", f"feld{i}", i) for i in range(30)]
    behalten, zurueck = api_llm._rueckfragen_gebuendelt(roh)
    assert len(behalten) == api_llm.RUECKFRAGEN_MAX, f"{len(behalten)} statt Obergrenze"
    assert zurueck == 30 - api_llm.RUECKFRAGEN_MAX


def test_der_normalfall_wird_nicht_angetastet():
    """Die Gegenrichtung, ohne die „gib immer nur eine zurück" eine bestandene Lösung wäre. Fünf
    Aussagen mit je einer offenen Frage sind genau das, wofür die Rückfrage gebaut ist."""
    roh = [_rf(f"Frage {i}", f"feld{i}", i) for i in range(5)]
    behalten, zurueck = api_llm._rueckfragen_gebuendelt(roh)
    assert behalten == roh, "Der Normalfall wurde gekürzt."
    assert zurueck == 0


def test_leere_liste_bleibt_leer():
    assert api_llm._rueckfragen_gebuendelt([]) == ([], 0)


# ---------------------------------------------------------------- durch den ganzen Aufruf

class _Antwort:
    def __init__(self, roh: bytes):
        self._roh = roh

    def read(self):
        return self._roh

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False


def _stufen(monkeypatch, *, s1, s2, s3):
    """urlopen ersetzen, Antwort NACH SCHEMA-NAMEN — dieselbe Attrappe wie in
    tests/test_dialog_drei_stufen.py."""
    nach_name = {"aussagen": s1, "zuordnung": s2, "dialog": s3}
    gesendet = []

    def _urlopen(req, timeout=None):
        body = json.loads(req.data.decode())
        gesendet.append(body)
        name = body.get("response_format", {}).get("json_schema", {}).get("name", "")
        return _Antwort(json.dumps({"provider": "TestAnbieter",
                                    "choices": [{"finish_reason": "stop",
                                                 "message": {"content": json.dumps(
                                                     nach_name.get(name) or {})}}]}).encode())

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    monkeypatch.setattr(LC.time, "sleep", lambda s: None)
    return gesendet


@pytest.fixture
def konfiguriert(monkeypatch, tmp_path):
    monkeypatch.setenv("LLM_API_KEY", "test-schluessel-nicht-echt")
    monkeypatch.setenv("LLM_API_BASE", "https://example.test/v1")
    monkeypatch.setenv("LLM_MODEL", "test/modell")
    monkeypatch.setattr(AUDIT, "AUDIT_DIR", str(tmp_path))
    return tmp_path


S1 = {"aussagen": [{"text": "Der Nutzer hat 2 Kinder", "beleg": "Ich habe 2 Kinder"},
                   {"text": "Der Nutzer hatte Arztkosten", "beleg": "620 Euro Arztkosten"}]}
S2 = {"zuordnungen": [{"aussage": 0, "regeln": ["p32_kinder"]},
                      {"aussage": 1, "regeln": ["p33_agb"]}]}


def test_die_flut_kommt_gebuendelt_an(konfiguriert, monkeypatch):
    """Der gemessene Fall, nachgestellt: viele Rückfragen zu wenigen Aussagen."""
    viele = ([_rf(f"Kinderfrage {i}", "kind_vorname", 0) for i in range(15)]
             + [_rf(f"Arztfrage {i}", "agb_aufwendungen", 1) for i in range(6)])
    _stufen(monkeypatch, s1=S1, s2=S2,
            s3={"vorschlaege": [], "rueckfragen": viele, "antwort": "", "unsicher": False})

    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert len(erg["rueckfragen"]) == 2, (
        f"{len(erg['rueckfragen'])} Rückfragen statt einer je Aussage — die Bündelung läuft im "
        f"echten Aufruf nicht mit: {[r['frage'] for r in erg['rueckfragen']]}")
    assert erg["rueckfragen_zurueckgestellt"] == 19, (
        f"Die Zahl der zurückgestellten stimmt nicht: {erg['rueckfragen_zurueckgestellt']}")


def test_ein_vorschlag_geht_nicht_mit_einer_weggebuendelten_rueckfrage_verloren(konfiguriert,
                                                                                monkeypatch):
    """DER TEUERSTE FEHLER, den diese Änderung machen könnte, und er wäre lautlos.

    `_rueckfrage_verdraengt` entfernt zu JEDEM gefragten Feld den Vorschlag — zu Recht: fragen und
    gleichzeitig raten war der Befund vom 2026-08-21 (70.000 EUR zu viel Bruttolohn). Liefe es aber
    VOR dem Bündeln, verdrängte auch eine Rückfrage, die danach gar nicht mehr gestellt wird. Der
    Nutzer verlöre den Wert UND die Frage: das Feld bliebe leer, ohne dass irgendwo etwas dazu
    steht.

    Hier steht zu `agb_aufwendungen` ein belegter Vorschlag, und die Rückfrage zu genau diesem Feld
    ist die ZWEITE ihrer Aussage — sie fällt also weg. Der Vorschlag muss überleben."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3={
        "vorschlaege": [{"feld_id": "agb_aufwendungen", "wert": 62000,
                         "beleg": "620 Euro Arztkosten", "begruendung": "egal", "aussage": 1}],
        "rueckfragen": [_rf("Welche Art von Arztkosten?", "", 1),
                        _rf("Wie hoch genau?", "agb_aufwendungen", 1)],
        "antwort": "", "unsicher": False})

    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    felder = [v["feld_id"] for v in erg["vorschlaege"]]
    assert "agb_aufwendungen" in felder, (
        "Der Vorschlag wurde von einer Rückfrage verdrängt, die gar nicht gestellt wird — Wert weg, "
        "Frage weg, und der Nutzer erfährt von beidem nichts. Reihenfolge in _llm_dialog prüfen: "
        "erst bündeln, dann verdrängen.")
    assert len(erg["rueckfragen"]) == 1


def test_eine_gestellte_rueckfrage_verdraengt_ihren_vorschlag_weiterhin(konfiguriert, monkeypatch):
    """Die Gegenprobe zum Test darüber: bleibt die Rückfrage stehen, MUSS sie verdrängen. Ohne
    diesen Test wäre „nie verdrängen" eine bestandene Lösung — und das war der 70.000-EUR-Fehler."""
    _stufen(monkeypatch, s1=S1, s2=S2, s3={
        "vorschlaege": [{"feld_id": "agb_aufwendungen", "wert": 62000,
                         "beleg": "620 Euro Arztkosten", "begruendung": "egal", "aussage": 1}],
        "rueckfragen": [_rf("Wie hoch genau?", "agb_aufwendungen", 1)],
        "antwort": "", "unsicher": False})

    erg = api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    assert [v["feld_id"] for v in erg["vorschlaege"]] == [], (
        "Die Rückfrage steht, aber der geratene Wert steht daneben — genau der Fall, der am "
        "2026-08-21 fast 70.000 EUR zu viel Bruttolohn ergab.")
    assert len(erg["rueckfragen"]) == 1


def test_der_prompt_sagt_es_auch(konfiguriert, monkeypatch):
    """Deterministisch UND im Prompt, nicht eines von beidem. Der Filter allein bekäme vom Modell
    weiter 21 Fragen und würfe 19 weg — dann entschiede die Reihenfolge des Modells darüber, welche
    eine Frage der Nutzer sieht, statt einer Überlegung."""
    gesendet = _stufen(monkeypatch, s1=S1, s2=S2,
                       s3={"vorschlaege": [], "rueckfragen": [], "antwort": "", "unsicher": False})
    api_llm._llm_dialog(TEXT, KATALOG, user_id="prüfer")

    system = [b for b in gesendet
              if b.get("response_format", {}).get("json_schema", {}).get("name") == "dialog"
              ][0]["messages"][0]["content"]
    assert "HÖCHSTENS EINE RÜCKFRAGE JE ZEILE" in system, (
        "Die Regel steht nicht im Prompt — dann arbeitet das Modell weiter gegen den Filter.")
    assert "bündle nie zwei Fragen in einen Satz" in system, (
        "Die Bündelungs-Regel fehlt: „Wie heissen sie und wann sind sie geboren?“ ist EINE "
        "Rückfrage mit EINEM Eingabefeld für ZWEI Antworten.")
