"""Zwei Grenzen um den LLM-Aufruf im Anfragepfad — und die eine Stelle, an der ein Retry schadet.

AUSGANGSLAGE (2026-08-18): `uebernehme_kontoauszug` ruft den LLM-Klassifikator JE mehrdeutiger
Buchung. Es gab keinen Deckel. Ein Auszug mit 200 unklaren Zwecken erzeugte 200 Aufrufe à 30 s
Zeitlimit — im einfädigen Server (server.py:make_server nutzt HTTPServer, nicht
ThreadingHTTPServer) bis zu 100 Minuten Stillstand, und 200 mal Kosten. Dieselbe Bauart wie die
Unterprozesse ohne Zeitlimit, eine Schicht höher: Einzelgrenze vorhanden, Anzahlgrenze nicht.

Der Audit-Befund an derselben Stelle (res-product-clients-no-retry) empfahl das Gegenteil:
Wiederholungsversuche, wie sie pipeline/client.py für dieselbe Anbieterklasse längst hat. Beides
zusammen ist ein Zielkonflikt — 50 Aufrufe × 3 Versuche × 30 s wären 75 Minuten. Aufgelöst durch
eine dritte Grenze: eine Wanduhr über die ganze Schleife (LLM_ZEITBUDGET_S). Der Retry rettet
den einzelnen Aufruf, das Budget deckelt den Schaden, wenn er nicht hilft.

WAS DIESE DATEI VOR ALLEM PRÜFT, ist nicht dass die Deckel greifen, sondern dass sie EHRLICH
sind: eine Buchung, die wegen der Grenze nie klassifiziert wurde, sieht im Store exakt aus wie
eine, die geprüft und für unklar befunden wurde (beides: kein Event). Ohne die zurückgegebene
Zahl könnte niemand beides unterscheiden, und ein halb angesehener Auszug sähe aus wie ein ganz
angesehener. Genau diese Klasse hat hier schon Geld gekostet (slot-fail-open-get-default).

NULL LLM: alle Klassifikatoren sind plain-python-Stubs, die den Injektionspunkt testen — nie ein
echter Aufruf, nie ein Mock eines Anbieters.
"""
from __future__ import annotations

import os
import sys
import urllib.error

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/store", "produkt/import", "produkt/traverser",
             "produkt/unsicherheit", "produkt/mapping", "produkt/konsistenz", "produkt/bescheid",
             "golden", "elster"):
    _p = os.path.join(ROOT, _sub)
    if _p not in sys.path:
        sys.path.insert(0, _p)

import kontoauszug_writer as KW   # noqa: E402
import llm_client as LC           # noqa: E402
import store as ST                # noqa: E402
import traverser as TR            # noqa: E402

TS = "2026-08-18T00:00:00Z"


@pytest.fixture
def bindung():
    return TR.lade_bindung()


def _unklare(n: int) -> list[dict]:
    """n Ausgaben-Buchungen, deren Zweck die Heuristik NICHT einordnet — jede eine, die den
    LLM-Fallback auslöst. Verschiedene Referenznummern, damit nichts zusammenfällt."""
    return [{"datum": "1.1.", "betrag": -1000 - i,
             "verwendungszweck": f"KRYPTISCH-REF-{i:05d}"} for i in range(n)]


# ----------------------------------------------------------------------- Anzahl-Deckel

def test_deckel_begrenzt_die_aufrufe(bindung):
    """Über der Grenze wird nicht mehr klassifiziert — und die Zahl der Aufrufe wird wirklich
    gezählt, nicht nur die der Buchungen."""
    aufrufe = []
    stub = lambda zweck, betrag: aufrufe.append(zweck) or None      # noqa: E731
    s = ST.leerer_store(2025, fall_id="ka-deckel")
    KW.uebernehme_kontoauszug(s, _unklare(KW.LLM_AUFRUFE_HOECHSTZAHL + 25), bindung,
                              llm_klassifikator=stub, ts=TS)
    assert len(aufrufe) == KW.LLM_AUFRUFE_HOECHSTZAHL, (
        f"{len(aufrufe)} LLM-Aufrufe statt höchstens {KW.LLM_AUFRUFE_HOECHSTZAHL} — der Deckel "
        f"greift nicht, ein grosser Auszug blockiert den einfädigen Server beliebig lange.")


def test_uebersprungene_werden_gemeldet(bindung):
    """Der eigentliche Punkt. Ohne diese Zahl ist der Deckel eine STILLE Kürzung: übersprungene
    und geprüft-unklare Buchungen sind im Store nicht zu unterscheiden."""
    ueberzahl = 25
    s = ST.leerer_store(2025, fall_id="ka-melde")
    _n, uebersprungen = KW.uebernehme_kontoauszug(
        s, _unklare(KW.LLM_AUFRUFE_HOECHSTZAHL + ueberzahl), bindung,
        llm_klassifikator=lambda z, b: None, ts=TS)
    assert uebersprungen == ueberzahl, (
        f"{uebersprungen} gemeldet, {ueberzahl} übersprungen — wer die Zahl nicht bekommt, kann "
        f"dem Nutzer nicht sagen, dass ein Teil seines Auszugs nie angesehen wurde.")


def test_unter_dem_deckel_wird_nichts_uebersprungen(bindung):
    """Der Normalfall. Ohne diesen Test wäre ein Deckel von 0 die grünste Lösung — dieselbe
    Klasse wie ein Gate, das seine eigene Voraussetzung mitbringt."""
    s = ST.leerer_store(2025, fall_id="ka-normal")
    aufrufe = []
    stub = lambda z, b: aufrufe.append(z) or None                   # noqa: E731
    _n, uebersprungen = KW.uebernehme_kontoauszug(s, _unklare(5), bindung,
                                                  llm_klassifikator=stub, ts=TS)
    assert len(aufrufe) == 5 and uebersprungen == 0


def test_ohne_llm_kein_ueberspringen(bindung):
    """Ist gar kein Klassifikator injiziert (der Regelfall ohne API-Schlüssel, $0), darf der
    Deckel nichts zählen — sonst meldete die Oberfläche einen Verzicht, den es nie gab."""
    s = ST.leerer_store(2025, fall_id="ka-ohne")
    _n, uebersprungen = KW.uebernehme_kontoauszug(
        s, _unklare(KW.LLM_AUFRUFE_HOECHSTZAHL + 10), bindung, llm_klassifikator=None, ts=TS)
    assert uebersprungen == 0


def test_zeitbudget_greift_auch_unter_dem_anzahldeckel(bindung, monkeypatch):
    """Die zweite Grenze, und der Grund für ihre Existenz: seit der Client wiederholt, kann ein
    einzelner Aufruf ein Vielfaches seines Zeitlimits brauchen. Der Anzahl-Deckel allein liesse
    50 × 3 × 30 s zu."""
    uhr = {"t": 0.0}
    monkeypatch.setattr(KW.time, "monotonic", lambda: uhr["t"])

    def _langsam(zweck, betrag):
        uhr["t"] += KW.LLM_ZEITBUDGET_S / 4      # vier Aufrufe reissen das Budget
        return None

    s = ST.leerer_store(2025, fall_id="ka-zeit")
    _n, uebersprungen = KW.uebernehme_kontoauszug(
        s, _unklare(20), bindung, llm_klassifikator=_langsam, ts=TS)
    assert uebersprungen > 0, (
        "das Zeitbudget greift nicht — bei langsamen Aufrufen bleibt nur der Anzahl-Deckel, "
        "und der lässt ein Vielfaches der beabsichtigten Blockade zu")
    assert uebersprungen < 20, "es wurde gar nichts klassifiziert — Budget zu eng oder Uhr falsch"


def test_hinweis_erreicht_den_nutzer(tmp_path, monkeypatch, bindung):
    """Die Naht bis zur Antwort. Die gezählte Zahl nützt nichts, wenn sie im Endpunkt liegen
    bleibt — dann ist der Deckel für den Nutzer weiterhin unsichtbar, und genau darum ging es.

    Dass eine Prüfung auf der Writer-Ebene stehen bleibt und die Verdrahtung offen lässt, ist
    hier schon vorgekommen (das Beleg-Gate war nie als VERDRAHTET geprüft)."""
    import api as API
    import api_auth
    import api_llm
    import audit

    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(api_auth, "_AUTH_USER", None)
    monkeypatch.setenv("TAXGRAPH_NO_AUTH", "1")
    status, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025,
                                  "fall_id": "ka_hinweis"})
    assert status == 201

    # Klassifikator injizieren, ohne je einen Anbieter anzusprechen: derselbe Injektionspunkt,
    # den die Produktion nutzt, nur mit einem plain-python-Stub dahinter.
    monkeypatch.setattr(api_llm, "_kontoauszug_llm_klassifikator", lambda: (lambda z, b: None))

    ueberzahl = 7
    zeilen = ["datum;betrag;verwendungszweck"]
    for i in range(KW.LLM_AUFRUFE_HOECHSTZAHL + ueberzahl):
        zeilen.append(f"01.03.2025;-10,0{i % 10};KRYPTISCH-REF-{i:05d}")
    status, koerper = API.kontoauszug("ka_hinweis", {"format": "csv", "inhalt": "\n".join(zeilen)})

    assert status == 200
    assert koerper.get("llm_uebersprungen") == ueberzahl, (
        f"Antwort meldet {koerper.get('llm_uebersprungen')} übersprungene statt {ueberzahl} — "
        f"der Nutzer hält einen halb angesehenen Auszug für einen ganz angesehenen.\n{koerper}")
    assert "hinweis" in koerper and str(KW.LLM_AUFRUFE_HOECHSTZAHL) in koerper["hinweis"], (
        f"Hinweistext nennt die Grenze nicht: {koerper.get('hinweis')!r}")


# ------------------------------------------------------------- Wiederholung im llm_client

class _FakeAntwort:
    """Minimaler urlopen-Ersatz. KEIN Mock eines LLM: prüft ausschliesslich, wie der Client auf
    Statuscodes reagiert, nicht was ein Anbieter inhaltlich antwortet."""
    def __init__(self, text: str):
        self._text = text

    def read(self):
        return self._text.encode()

    def __enter__(self):
        return self

    def __exit__(self, *a):
        return False


def _antwort(inhalt: str = "ok") -> _FakeAntwort:
    import json
    return _FakeAntwort(json.dumps({"choices": [{"message": {"content": inhalt}}]}))


def _http_fehler(code: int) -> urllib.error.HTTPError:
    import io
    return urllib.error.HTTPError("http://x", code, "fehler", {}, io.BytesIO(b"detail"))


@pytest.fixture
def konfiguriert(monkeypatch):
    monkeypatch.setenv("LLM_API_KEY", "test-schluessel-nicht-echt")
    monkeypatch.setenv("LLM_API_BASE", "http://127.0.0.1:1/v1")
    monkeypatch.setenv("LLM_MODEL", "testmodell")
    monkeypatch.setattr(LC.time, "sleep", lambda s: None)     # keine echten Wartezeiten im Test


def test_voruebergehender_fehler_wird_wiederholt(konfiguriert, monkeypatch):
    """503 einmal, dann Erfolg — genau der Fall, den die Entwicklungs-Pipeline als gemessene
    Realität dokumentiert und der bisher sofort zu LlmNichtVerfuegbar wurde."""
    versuche = []

    def _urlopen(req, timeout=None):
        versuche.append(1)
        if len(versuche) == 1:
            raise _http_fehler(503)
        return _antwort("gelungen")

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    assert LC._call([{"role": "user", "content": "x"}]) == "gelungen"
    assert len(versuche) == 2, f"{len(versuche)} Versuche — es wurde nicht wiederholt"


def test_dauerhafte_stoerung_gibt_nach_begrenzt_vielen_versuchen_auf(konfiguriert, monkeypatch):
    """Unbegrenzt zu wiederholen wäre in einem einfädigen Server schlimmer als der ursprüngliche
    Zustand."""
    versuche = []

    def _urlopen(req, timeout=None):
        versuche.append(1)
        raise _http_fehler(503)

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    with pytest.raises(LC.LlmNichtVerfuegbar):
        LC._call([{"role": "user", "content": "x"}])
    assert len(versuche) == LC._VERSUCHE, (
        f"{len(versuche)} Versuche statt {LC._VERSUCHE} — die Wiederholung ist unbegrenzt oder "
        f"findet nicht statt.")


@pytest.mark.parametrize("code", [400, 401, 403, 404, 422])
def test_dauerhafter_fehler_wird_nicht_wiederholt(konfiguriert, monkeypatch, code):
    """Der Punkt, an dem die Pipeline-Lösung NICHT übernommen werden durfte: dort gilt 403 als
    vorübergehend, ausdrücklich begründet mit einer OpenRouter-Eigenheit ('an invalid key gives
    401, not 403'). Dieser Client ist provider-agnostisch, und sein eigener Kommentar hält fest,
    wofür 403 hier steht: 'Budget limit exceeded'. Ein erschöpftes Budget dreimal anzufragen
    kostet nur Zeit — im einfädigen Server dreimal so lange Stillstand für dieselbe Antwort."""
    versuche = []

    def _urlopen(req, timeout=None):
        versuche.append(1)
        raise _http_fehler(code)

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    with pytest.raises(LC.LlmNichtVerfuegbar):
        LC._call([{"role": "user", "content": "x"}])
    assert len(versuche) == 1, (
        f"HTTP {code} wurde {len(versuche)}× versucht — dieser Fehler heilt nicht durch "
        f"Wiederholen, jeder weitere Versuch ist nur Blockade.")


def test_erfolg_beim_ersten_mal_wiederholt_nicht(konfiguriert, monkeypatch):
    """Regelfall: keine zusätzliche Latenz, wo nichts schiefgeht."""
    versuche = []
    monkeypatch.setattr(LC.urllib.request, "urlopen",
                        lambda req, timeout=None: (versuche.append(1), _antwort("ok"))[1])
    assert LC._call([{"role": "user", "content": "x"}]) == "ok"
    assert len(versuche) == 1


def test_schluessel_bleibt_auch_nach_wiederholung_maskiert(konfiguriert, monkeypatch):
    """Die bestehende Maskierung darf der neue Pfad nicht umgehen: manche Anbieter spiegeln den
    Schlüssel in der Fehlerantwort zurück, und die Ausnahme landet im Server-Log."""
    import io
    schluessel = os.environ["LLM_API_KEY"]

    def _urlopen(req, timeout=None):
        raise urllib.error.HTTPError(
            "http://x", 503, "fehler", {},
            io.BytesIO(f"invalid key {schluessel}".encode()))

    monkeypatch.setattr(LC.urllib.request, "urlopen", _urlopen)
    with pytest.raises(LC.LlmNichtVerfuegbar) as e:
        LC._call([{"role": "user", "content": "x"}])
    assert schluessel not in str(e.value), (
        "der API-Schlüssel steht in der Ausnahme — die Maskierung greift auf dem "
        "Wiederholungspfad nicht")
