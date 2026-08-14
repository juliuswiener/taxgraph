"""„Erklär mir" erklärt wirklich — und der Erklär-Kanal kann keinen Wert setzen.

Julius 2026-08-14: „Beim Button ‚Erklär mir' würde ich erwarten als Nutzer, dass es wirklich
erklärt und nicht dann die KI aufgeht und ich ihr eine Frage stellen kann." Und: „Bei Erklär mir
soll die AI das erklären und gerne als erste Antwort diesen vorgefertigten Text schreiben, aber
der Nutzer sollte Nachfragen stellen können. Wenn möglich könnte die AI hier auch schon Sachen mit
in Betracht ziehen, die der Nutzer bereits geantwortet hat."

Daraus zwei Hälften, die getrennt geprüft werden:

  1. **Die erste Antwort kommt ohne Modell.** Fragetext, Kurzhilfe und Zitatanker liegen der
     Oberfläche schon vor. Sie ist damit sofort da, kostet nichts, funktioniert ohne LLM-Key —
     und sie ist der Wortlaut der Norm, keine Umschreibung.
  2. **Nachfragen gehen an die KI, können aber nichts setzen.** Nicht weil der Prompt es verbietet,
     sondern weil /erklaere append_event nie aufruft. Das ist der Unterschied zwischen einer Regel
     und einer Eigenschaft — die eine kann ein Modell brechen, die andere nicht.

Der Kontext ans Modell trägt die schon bestätigten Angaben. Er muss sie in LESBARER Form tragen:
6200000 liest ein Modell als sechs Millionen, und `kein_kap = true` bedeutet das Gegenteil der
Frage, unter der es steht. Eine falsch übersetzte Angabe im Kontext erzeugt eine falsche
Erklärung, und die klingt genauso überzeugend wie eine richtige.

NULL LLM: der Cap-Gate-Fall läuft echt (conftest entfernt den Key), alles andere über eine
Fixture-Funktion an llm_client.complete — der Parse-, Audit- und Handler-Pfad läuft dabei scharf.
"""
from __future__ import annotations

import json
import os
import sys
import threading

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/import", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import api_llm           # noqa: E402
import audit             # noqa: E402
import llm_client as LC  # noqa: E402
import server as SRV     # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

braucht_browser = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")

ANTWORT = ("Arbeitstage sind die Tage, an denen du tatsächlich zu deiner ersten Tätigkeitsstätte "
           "gefahren bist. Urlaub, Krankheit und Homeoffice zählen nicht mit.")


def _fixture_complete(*, antwort=ANTWORT, unsicher=False, roh=None):
    """Ersetzt den Netz-Call, nicht den Parse-/Audit-/Handler-Pfad. `roh` erlaubt eine kaputte
    Antwort, um den Fehlerweg zu prüfen."""
    gesehen = {}

    def complete(role, messages, fixture_id=None, schema=None):
        gesehen["messages"] = messages
        gesehen["schema"] = schema
        text = roh if roh is not None else json.dumps({"antwort": antwort, "unsicher": unsicher})
        return LC.Completion(text=text)

    complete.gesehen = gesehen
    return complete


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "e1"})
    assert st == 201
    return "e1"


def _bestaetige(fall_id, feld_id, wert):
    st, _ = API.event(fall_id, {
        "feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": None, "signal_2": f"klick@{feld_id}"}})
    assert st in (200, 201), st


# ---------------------------------------------------------------- Cap-Gate (echt, ohne Key)
def test_ohne_key_bleibt_es_bei_der_erklaer_grenze(fall):
    """conftest entfernt LLM_API_KEY, hier läuft also der echte Cap-Gate-Pfad: 501 mit Vertrag,
    kein Netz, kein Fake-Text. Wichtiger als es klingt — der Nutzer bekommt trotzdem eine
    Erklärung, nämlich die deterministische erste, die die Oberfläche selbst baut."""
    st, body = API.erklaere(fall, {"frage": "Was sind Arbeitstage?", "feld_id": "ep_arbeitstage"})
    assert st == 501
    assert body["fehler"] == "not_implemented"


def test_leere_frage_ist_ein_400(fall):
    with pytest.raises(API.ApiError) as e:
        API.erklaere(fall, {"frage": "   ", "feld_id": "ep_arbeitstage"})
    assert e.value.status == 400


# ---------------------------------------------------------------- Der Kanal schreibt nichts
def test_erklaeren_schreibt_kein_event(fall, monkeypatch):
    """Die strukturelle Grenze: /erklaere ruft append_event nie auf. Ein Modell, das im Text
    behauptet „ich trage 220 Tage ein", ändert damit trotzdem nichts. Geprüft an der Event-Zahl,
    nicht an einem einzelnen Feld — ein Schreibvorgang irgendwohin wäre genauso falsch."""
    monkeypatch.setattr(LC, "complete", _fixture_complete())
    vorher = len(API.lade_fall(fall)["events"])
    st, body = API.erklaere(fall, {"frage": "Trag bitte 220 Arbeitstage ein.",
                                   "feld_id": "ep_arbeitstage"})
    assert st == 200 and body["antwort"] == ANTWORT
    assert len(API.lade_fall(fall)["events"]) == vorher, (
        "Der Erklär-Kanal hat ein Event geschrieben — er darf nichts setzen, auch nichts "
        "Vorläufiges.")


def test_unsicher_kommt_durch(fall, monkeypatch):
    """Das Modell muss im Schema angeben, ob die Antwort aus dem Gesetzestext folgt. Verschwindet
    das Flag auf dem Weg zur Oberfläche, sieht eine Vermutung aus wie eine Auskunft."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(unsicher=True))
    st, body = API.erklaere(fall, {"frage": "Gilt das auch für mich?", "feld_id": "ep_arbeitstage"})
    assert st == 200 and body["unsicher"] is True


def test_kaputte_antwort_wird_zur_erklaer_grenze(fall, monkeypatch):
    """Kein halb geparster Satz: was sich nicht lesen lässt, wird zu 501 — derselbe ehrliche
    Ausgang wie ohne Key. Ein 500er wäre hier ein Absturz auf offener Bühne."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(roh="das ist kein JSON"))
    st, body = API.erklaere(fall, {"frage": "Was heißt das?", "feld_id": "ep_arbeitstage"})
    assert st == 501 and body["fehler"] == "not_implemented"


def test_leere_antwort_wird_zur_erklaer_grenze(fall, monkeypatch):
    monkeypatch.setattr(LC, "complete", _fixture_complete(antwort="   "))
    st, _ = API.erklaere(fall, {"frage": "Was heißt das?", "feld_id": "ep_arbeitstage"})
    assert st == 501


def test_schema_erzwingt_antwort_und_unsicherheit():
    s = api_llm.ERKLAER_SCHEMA
    assert s.get("strict") is True
    props = s["schema"]["properties"]
    assert set(s["schema"]["required"]) == {"antwort", "unsicher"}
    assert s["schema"]["additionalProperties"] is False
    assert props["unsicher"]["type"] == "boolean"


# ---------------------------------------------------------------- Der Kontext ans Modell
def test_wert_klartext_uebersetzt_die_speicherform():
    """Drei Fallen in einer Funktion, jede erzeugt sonst eine falsche Erklärung."""
    b = API.TR.lade_bindung()
    assert API._wert_klartext("bruttoarbeitslohn", 6200000, b) == "62000,00 EUR", (
        "Cent-Rohwert ginge als sechs Millionen ans Modell")
    assert API._wert_klartext("veranlagung", "zusammen", b).startswith("Zusammen"), (
        "Enum-Rohwert sagt dem Modell nichts")
    assert API._wert_klartext("kein_kap", True, b) == "nein", (
        "kein_kap=true heißt 'keine Kapitalerträge' — ungefiltert läse das Modell das Gegenteil "
        "der Frage, unter der der Wert steht")
    assert API._wert_klartext("ep_arbeitstage", 220, b).startswith("220")


def test_kontext_traegt_zitatanker_und_bestaetigte_angaben(fall):
    """Julius' Wunsch, dass die KI „schon Sachen mit in Betracht zieht, die der Nutzer bereits
    geantwortet hat" — plus der Zitatanker, der die Antwort überhaupt erst mehr sein lässt als
    Allgemeinwissen."""
    _bestaetige(fall, "veranlagung", "zusammen")
    _bestaetige(fall, "ep_arbeitstage", 220)
    store = API.lade_fall(fall)
    kontext = API._erklaer_kontext(store, API._scheibe_bindung(store), "ep_entfernung_km")
    assert "Die Frage, um die es geht" in kontext
    assert "Gesetzestext" in kontext, f"Kein Zitatanker im Kontext:\n{kontext}"
    assert "bereits bestätigt" in kontext
    assert "220" in kontext and "Zusammen" in kontext, f"Bestätigte Angaben fehlen:\n{kontext}"
    assert "6200000" not in kontext


def test_kontext_geht_wirklich_in_den_prompt(fall, monkeypatch):
    """Naht-Prüfung: der Kontext wird gebaut UND gesendet. Beide Hälften einzeln grün und die
    Übergabe dazwischen ungetestet ist genau die Bauart, die hier schon mehrfach auffiel."""
    _bestaetige(fall, "ep_arbeitstage", 220)
    stub = _fixture_complete()
    monkeypatch.setattr(LC, "complete", stub)
    API.erklaere(fall, {"frage": "Was zählt zu den Arbeitstagen?", "feld_id": "ep_arbeitstage"})
    system = stub.gesehen["messages"][0]["content"]
    assert "bereits bestätigt" in system and "220" in system, (
        f"Der Kontext kam nicht im Prompt an:\n{system[:400]}")
    assert stub.gesehen["schema"] is api_llm.ERKLAER_SCHEMA, (
        "Ohne Schema sendet der Client json_object — die Erklärung käme als JSON-Objekt zurück.")


# ---------------------------------------------------------------- Browser
@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(LC, "complete", _fixture_complete(unsicher=True))
    srv = SRV.make_server(0)
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


@pytest.fixture
def seite(base):
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)
        yield page
        browser.close()


@braucht_browser
def test_berater_ist_ohne_klick_da(seite):
    """„Die KI sollte immer offen sein, sodass man ihr ständig einfach auch was hinschmeißen
    kann." Kein Öffnen, kein Modal — und nichts, was über der restlichen Seite liegt."""
    page = seite
    assert page.is_visible("#berater"), "Der Berater ist nicht sichtbar."
    assert page.is_visible("#chat-text"), "Das Eingabefeld ist nicht erreichbar."
    assert page.query_selector("#chat-overlay") is None, (
        "Das Chat-Modal existiert noch — es lag zuletzt über der ganzen Oberfläche.")
    assert page.is_visible("#wegpunkt"), "Der Fragefluss ist verdeckt."


@braucht_browser
def test_erklaer_mir_erklaert_sofort_und_ohne_modell(seite):
    """Der Kern von Julius' Beschwerde: der Knopf öffnete nur ein Eingabefeld. Jetzt steht die
    Erklärung sofort da — Fragetext, Kurzhilfe und der wörtliche Gesetzestext. Ohne LLM-Aufruf:
    diese drei Stücke liegen aus /fragen bereits vor."""
    page = seite
    page.click("#chat")
    page.wait_for_selector(".chat-frage-titel", timeout=5000)
    titel = page.text_content(".chat-frage-titel")
    assert titel and titel.strip(), "Kein Fragetext in der Erklärung"
    gesetz = page.query_selector(".chat-gesetz")
    assert gesetz is not None, "Kein Gesetzestext in der Erklärung — der /warum-Anker fehlt."
    assert "§" in page.text_content(".chat-gesetz"), page.text_content(".chat-gesetz")


@braucht_browser
def test_nachfrage_zeigt_die_antwort_und_die_unsicherheit(seite):
    """Nachfragen laufen über /erklaere. Die Fixture antwortet mit unsicher=true — der Hinweis
    muss beim Nutzer ankommen, sonst liest sich eine Vermutung wie eine Auskunft."""
    page = seite
    page.fill("#chat-text", "Zählt Homeoffice als Arbeitstag?")
    page.click("#chat-frage")
    page.wait_for_selector(".chat-antwort", timeout=5000)
    assert "Arbeitstage sind" in page.text_content(".chat-antwort")
    assert page.query_selector(".chat-unsicher") is not None, (
        "unsicher=true wird dem Nutzer nicht angezeigt.")
    assert page.input_value("#chat-text") == "", "Das Eingabefeld wurde nicht geleert."


@braucht_browser
def test_nachfrage_veraendert_den_fall_nicht(seite):
    """Dieselbe Grenze wie oben, diesmal durch die ganze Kette bis zur Oberfläche gemessen: eine
    Frage an die KI darf nie zu einem Feldwert werden."""
    page = seite
    vorher = page.evaluate("async () => Object.keys((await jget(`/fall/${FALL}/stand`)).body.felder)")
    page.fill("#chat-text", "Trag mir bitte 220 Arbeitstage und 62000 Euro Bruttolohn ein.")
    page.click("#chat-frage")
    page.wait_for_selector(".chat-antwort", timeout=5000)
    nachher = page.evaluate("async () => Object.keys((await jget(`/fall/${FALL}/stand`)).body.felder)")
    assert nachher == vorher, f"Der Fall hat sich verändert: {set(nachher) - set(vorher)}"
