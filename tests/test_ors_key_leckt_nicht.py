"""Der ORS-Schlüssel darf unsere eigenen Ausgaben nie verlassen — auch nicht über eine Ursachenkette.

DER FUND (Audit 2026-08-16, sec-ors-key-in-query-string): geocode() hängt den Schlüssel als
`api_key=` an die URL, während der Routing-Aufruf im selben Modul korrekt den
Authorization-Header nutzt. Query-Strings landen in Zugriffsprotokollen, in Proxys und in jeder
Telemetrie, die Anfrage-URLs mitschneidet — anders als Header. Der Moduldocstring ist in genau
diesem Punkt ausdrücklich: "nie im Repo, nie geloggt".

WAS AM VORGESCHLAGENEN FIX NICHT GEHT, nachgeschlagen am 2026-08-18: den Schlüssel für
/geocode/search in den Header zu verlegen ist nicht belegt. Dieser Endpunkt ist keine
ORS-Route, sondern eine vorgeschaltete Pelias-Instanz ("This endpoint is not part of
openrouteservice, but of our public API"), und sämtliche offiziellen Beispiele dafür — auch die
des ORS-Supports im eigenen Forum — verwenden api_key im Query. Blind umzustellen hiesse
riskieren, die Entfernungsfunktion stillzulegen; die Prüfung gegen den echten Dienst steht aus.

WAS SEHR WOHL IN UNSERER HAND LIEGT und dieser Test festhält: dass der Schlüssel über unsere
eigenen Ausnahmen nicht ERREICHBAR ist. `raise ... from e` behielt die urllib.error.HTTPError
als `__cause__`, und die führt in `.url` die vollständige angefragte URL mit — samt Schlüssel.

Die erste Fassung dieses Tests behauptete mehr, als zutraf: sie druckte den Traceback und
suchte den Schlüssel darin. Er steht dort nicht — `str(HTTPError)` ist "HTTP Error 403:
Forbidden", die URL erscheint im Standarddruck gar nicht. Der Test war grün, gleich ob `from e`
oder `from None` dastand; die Mutationsprobe hat ihn überführt. Geprüft wird jetzt, was der Fix
wirklich bewirkt: die Kette ist gekappt, `e.__cause__` ist None, und damit gibt es kein
`__cause__.url` mehr für alles, was Ausnahmen samt Attributen serialisiert (Fehler-Meldedienste,
Diagnose-Dumps). Der Moduldocstring verspricht "nie geloggt" ohne Einschränkung — dann darf der
Schlüssel auch nicht an einer Ausnahme hängen, die irgendwohin weitergereicht wird.

NULL LLM, kein Netzzugriff: urlopen ist ersetzt.
"""
from __future__ import annotations

import io
import os
import sys
import traceback
import urllib.error

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))

import ors_client as OC  # noqa: E402

SCHLUESSEL = "5b3ce3597851110001cf6248-TESTSCHLUESSEL-UNVERWECHSELBAR"


@pytest.fixture
def mit_schluessel(monkeypatch):
    monkeypatch.setenv("ORS_API_KEY", SCHLUESSEL)


def _http_fehler_mit_url(url: str) -> urllib.error.HTTPError:
    """Eine HTTPError, wie urllib sie wirft — mit der vollständigen URL in `.url`."""
    return urllib.error.HTTPError(url, 403, "Forbidden", {}, io.BytesIO(b"{}"))


def test_schluessel_steht_nicht_in_der_meldung(mit_schluessel, monkeypatch):
    """Die schlichte Erwartung, die schon vorher erfüllt war — hier festgehalten, damit sie
    nicht bei der nächsten Verbesserung der Fehlermeldung verlorengeht."""
    def _boom(req, timeout=None):
        raise _http_fehler_mit_url(f"{OC._BASE}/geocode/search?api_key={SCHLUESSEL}&text=x")

    monkeypatch.setattr(OC.urllib.request, "urlopen", _boom)
    with pytest.raises(OC.OrsNichtVerfuegbar) as e:
        OC.geocode("Musterstraße 1, Musterstadt")
    assert SCHLUESSEL not in str(e.value), f"Schlüssel in der Meldung: {e.value}"


def test_die_ursachenkette_wird_gekappt(mit_schluessel, monkeypatch):
    """Der Anhang, nicht die Meldung. `from e` behielt die HTTPError als `__cause__`, und die
    trägt in `.url` die vollständige angefragte URL — mit Schlüssel.

    PRÄZISION, WEIL DIE ERSTE FASSUNG DIESES TESTS WERTLOS WAR: sie druckte den Traceback und
    suchte den Schlüssel darin. Er stand nicht drin — `str(HTTPError)` ist "HTTP Error 403:
    Forbidden", die URL taucht im Standarddruck gar nicht auf. Der Test war grün, egal ob
    `from e` oder `from None` dastand (Mutationsprobe 2026-08-18). Eine Prüfung, die ihren
    eigenen Fehlerfall nicht erkennt, ist eine Behauptung.

    Was wirklich zutrifft und hier geprüft wird: mit erhaltener Kette ist der Schlüssel über
    `e.__cause__.url` ZUGÄNGLICH — für alles, was Ausnahmen mitsamt Attributen serialisiert
    (Fehler-Meldedienste, Diagnose-Dumps). Kein Standard-Traceback tut das; ein Werkzeug wie
    Sentry sehr wohl, und der Moduldocstring verspricht "nie geloggt" ohne diese Einschränkung.
    `from None` schliesst den Zugang, statt sich darauf zu verlassen, dass niemand hinsieht."""
    def _boom(req, timeout=None):
        raise _http_fehler_mit_url(f"{OC._BASE}/geocode/search?api_key={SCHLUESSEL}&text=x")

    monkeypatch.setattr(OC.urllib.request, "urlopen", _boom)
    with pytest.raises(OC.OrsNichtVerfuegbar) as e:
        OC.geocode("Musterstraße 1, Musterstadt")

    ursache = e.value.__cause__
    assert ursache is None, (
        f"Die Ursachenkette ist erhalten ({type(ursache).__name__}) — über "
        f"e.__cause__.url ist der Schlüssel erreichbar: "
        f"{str(getattr(ursache, 'url', ''))[:60]}...\n"
        f"`from None` statt `from e` an der Fangstelle in _hole.")
    # Und der Vollständigkeit halber: auch der gedruckte Traceback bleibt sauber. Dieser Teil
    # war schon vorher erfüllt und ist NICHT das, was der Fix bewirkt — er steht hier, damit
    # eine spätere Änderung der Meldung ihn nicht unbemerkt hineinschreibt.
    gedruckt = "".join(traceback.format_exception(type(e.value), e.value, e.value.__traceback__))
    assert SCHLUESSEL not in gedruckt, f"Schlüssel im Traceback:\n{gedruckt[-600:]}"


def test_diagnose_bleibt_trotz_gekappter_kette(mit_schluessel, monkeypatch):
    """`from None` kostet die ursprüngliche Ausnahme. Der Ausnahmetyp muss deshalb in der
    Meldung bleiben, sonst sieht ein Netzausfall aus wie ein kaputtes JSON — dieselbe Lehre,
    die der LLM-Client 2026-08-14 teuer gelernt hat."""
    def _boom(req, timeout=None):
        raise _http_fehler_mit_url(f"{OC._BASE}/geocode/search?api_key={SCHLUESSEL}")

    monkeypatch.setattr(OC.urllib.request, "urlopen", _boom)
    with pytest.raises(OC.OrsNichtVerfuegbar) as e:
        OC.geocode("Musterstraße 1")
    assert "HTTPError" in str(e.value), (
        f"Ausnahmetyp fehlt in der Meldung: {e.value} — ohne ihn ist die gekappte Kette ein "
        f"reiner Verlust.")


def test_routing_nutzt_weiterhin_den_header(mit_schluessel, monkeypatch):
    """Die eine Hälfte, die schon richtig war. Ohne diesen Test könnte jemand beim
    'Vereinheitlichen' der beiden Aufrufe den Header ZUM Query-String machen statt umgekehrt —
    und die Richtung wäre in keiner Prüfung festgehalten."""
    gesehen = {}

    def _merke(req, timeout=None):
        gesehen["url"] = req.full_url
        gesehen["headers"] = dict(req.headers)
        raise _http_fehler_mit_url(req.full_url)      # Ergebnis egal, nur der Request zählt

    monkeypatch.setattr(OC.urllib.request, "urlopen", _merke)
    with pytest.raises(OC.OrsNichtVerfuegbar):
        OC._distanz_meter([8.68, 49.41], [8.69, 49.42])

    assert "api_key" not in gesehen["url"], (
        f"Der Routing-Aufruf hat den Schlüssel in die URL genommen: {gesehen['url']}")
    kopfwerte = " ".join(str(v) for v in gesehen["headers"].values())
    assert SCHLUESSEL in kopfwerte, (
        f"Der Routing-Aufruf schickt den Schlüssel nicht mehr im Header: {gesehen['headers']}")


def test_geocode_traegt_den_schluessel_noch_im_query():
    """Festgehalten, was HEUTE gilt — nicht als Billigung, sondern damit die offene Stelle
    sichtbar bleibt und nicht als erledigt durchgeht.

    Der Endpunkt ist eine vorgeschaltete Pelias-Instanz; für sie ist der Authorization-Header
    nicht belegt. Wird das gegen den echten Dienst geprüft und funktioniert er, gehört dieser
    Test umgedreht — dann liest sich sein Fehlschlag als 'die Umstellung ist möglich und noch
    nicht gemacht' statt als stillschweigendes Weiter-so."""
    quelle = open(os.path.join(ROOT, "produkt", "haut", "ors_client.py"), encoding="utf-8").read()
    geocode_teil = quelle.split("def geocode(")[1].split("\ndef ")[0]
    assert "api_key" in geocode_teil, (
        "geocode nutzt den Query-Schlüssel nicht mehr — falls auf den Header umgestellt wurde "
        "und das gegen den echten Dienst geprüft ist: diesen Test umdrehen und den Kommentar "
        "in _hole entsprechend kürzen.")
