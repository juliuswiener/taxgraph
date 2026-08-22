"""Ein Eingabefeld, ein Knopf, ein Aufruf — Werte UND Antwort.

Julius 2026-08-14: „‚Ein Satz an die KI' kann aber auch einfach eine Nachfrage sein." Und:
„erklär mir sollte aber rückfragen erlauben."

Vorher gab es zwei Knöpfe und zwei Endpunkte. Damit musste der Nutzer seinen eigenen Satz vorher
einsortieren — obwohl ein Satz oft beides ist („Ich fahre 15 km, zählt Homeoffice als
Arbeitstag?"). Eine Klassifikation im Code wäre dieselbe Zwangswahl gewesen, bloß unsichtbar und
mit einer Fehlerquelle mehr. Also entscheidet niemand: das Modell füllt aus, was der Text hergibt,
und antwortet, wenn gefragt wurde.

Was dadurch NEU zu prüfen ist:

  1. **Beide Hälften kommen an** — und jede darf leer sein, ohne die andere zu beschädigen.
  2. **Die Antwort wird nirgends gespeichert.** Die Grenze liegt jetzt nicht mehr im Kanal,
     sondern im Umgang mit dem Ergebnis: aus `vorschlaege` werden vorläufige Events, `antwort` ist
     reiner Text. Ein Modell, das im Fließtext „ich trage dir 220 Tage ein" behauptet, ändert
     nichts.
  3. **Hypothesen sind keine Angaben.** Das ist die Lücke, die der zusammengelegte Kanal aufreißt:
     „Was wäre, wenn ich 62000 verdient hätte?" trägt eine Zahl UND einen wörtlichen Beleg — das
     Beleg-Gate greift also NICHT, es prüft nur, ob das Zitat im Text steht. Die Grenze steht im
     Prompt, und die Verstanden-Seite ist die zweite Verteidigung: dort liest der Nutzer sein
     eigenes Zitat neben dem Wert.
  4. **Der Kontext für die Antwort** (offenes Feld, Zitatanker, schon bestätigte Angaben) muss
     lesbar übersetzt sein — 6200000 liest ein Modell als sechs Millionen, `kein_kap=true` als
     Anwesenheit. Ein falsch übersetzter Kontext erzeugt eine falsche Erklärung, und die klingt
     genauso überzeugend wie eine richtige.

NULL LLM: der Cap-Gate-Fall läuft echt (conftest entfernt den Key), alles andere über eine
Fixture-Funktion an llm_client.complete — Prompt-Bau, Parser, Beleg-Gate, Audit und Handler
laufen dabei scharf.
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
import store as ST       # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

braucht_browser = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")

ANTWORT = ("Arbeitstage sind die Tage, an denen du tatsächlich zu deiner ersten Tätigkeitsstätte "
           "gefahren bist. Urlaub, Krankheit und Homeoffice zählen nicht mit.")


def _fixture_complete(*, vorschlaege=(), antwort="", unsicher=False, roh=None):
    """Ersetzt den Netz-Call, nicht den Prompt-/Parse-/Gate-/Audit-Pfad."""
    gesehen = {}

    def complete(role, messages, fixture_id=None, schema=None):
        gesehen["messages"] = messages
        gesehen["schema"] = schema
        text = roh if roh is not None else json.dumps(
            {"vorschlaege": [dict(v) for v in vorschlaege], "antwort": antwort, "unsicher": unsicher})
        return LC.Completion(text=text)

    complete.gesehen = gesehen
    return complete


def _v(feld, wert, beleg):
    return {"feld_id": feld, "wert": wert, "beleg": beleg, "begruendung": "egal"}


@pytest.fixture
def fall(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "d1"})
    assert st == 201
    return "d1"


def _bestaetige(fall_id, feld_id, wert):
    st, _ = API.event(fall_id, {
        "feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
        "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
        "schreiber": "ui:laie",
        "signal": {"signal_1": None, "signal_2": f"klick@{feld_id}"}})
    assert st in (200, 201), st


# ---------------------------------------------------------------- Beide Hälften
def test_reine_frage_liefert_antwort_und_keinen_wert(fall, monkeypatch):
    """Der Fall, der vorher den falschen Knopf gebraucht hätte."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(antwort=ANTWORT))
    vorher = len(API.lade_fall(fall)["events"])
    st, body = API.chat(fall, {"text": "Zählt ein Tag im Homeoffice als Arbeitstag?",
                               "feld_id": "ep_arbeitstage"})
    assert st == 200
    assert body["antwort"] == ANTWORT
    assert body["vorschlaege"] == []
    assert len(API.lade_fall(fall)["events"]) == vorher, "Eine Frage hat etwas geschrieben."


def test_reine_angabe_liefert_werte_und_keine_antwort(fall, monkeypatch):
    """Die Gegenrichtung: kein erfundener Belehrungstext, wenn niemand gefragt hat."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(
        vorschlaege=[_v("ep_arbeitstage", 220, "220 Tage")]))
    st, body = API.chat(fall, {"text": "Ich fahre an 220 Tagen zur Arbeit.",
                               "feld_id": "ep_arbeitstage"})
    assert st == 200
    assert [v["feld_id"] for v in body["vorschlaege"]] == ["ep_arbeitstage"]
    assert body["antwort"] == ""


def test_beides_in_einem_satz(fall, monkeypatch):
    """Der eigentliche Grund für die Zusammenlegung: ein Satz, der beides ist. Vorher hätte der
    Nutzer sich für einen der zwei Knöpfe entscheiden müssen — und eine Hälfte verloren."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(
        vorschlaege=[_v("ep_entfernung_km", 15, "15 km")], antwort=ANTWORT))
    st, body = API.chat(fall, {"text": "Ich fahre 15 km zur Arbeit — zählt Homeoffice als Arbeitstag?",
                               "feld_id": "ep_arbeitstage"})
    assert st == 200
    assert [v["feld_id"] for v in body["vorschlaege"]] == ["ep_entfernung_km"]
    assert body["antwort"] == ANTWORT
    assert ST._aktives(API.lade_fall(fall))["ep_entfernung_km"]["zustand"] == "vorlaeufig"


def test_antwort_wird_nicht_gespeichert(fall, monkeypatch):
    """Die Antwort ist Text und bleibt Text. Ein Modell, das im Fließtext behauptet, etwas
    einzutragen, ändert nichts — geschrieben wird nur, was durch Beleg-Gate und Katalog kommt."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(
        antwort="Ich habe dir 220 Arbeitstage und 62000 Euro Bruttolohn eingetragen."))
    vorher = len(API.lade_fall(fall)["events"])
    st, body = API.chat(fall, {"text": "Trag mir bitte alles ein.", "feld_id": "ep_arbeitstage"})
    assert st == 200 and body["antwort"]
    assert len(API.lade_fall(fall)["events"]) == vorher


def test_unsicher_kommt_durch(fall, monkeypatch):
    """Das Modell muss im Schema angeben, ob die Antwort aus dem Gesetzestext folgt. Verschwindet
    das Flag auf dem Weg zur Oberfläche, sieht eine Vermutung aus wie eine Auskunft."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(antwort=ANTWORT, unsicher=True))
    st, body = API.chat(fall, {"text": "Gilt das auch für mich?", "feld_id": "ep_arbeitstage"})
    assert st == 200 and body["unsicher"] is True


def test_ohne_key_bleibt_es_bei_der_erklaer_grenze(fall):
    """conftest entfernt LLM_API_KEY → echter Cap-Gate-Pfad, kein Netz, kein Fake-Text.
    „Erklär mir" bleibt trotzdem benutzbar: seine erste Antwort baut die Oberfläche selbst."""
    st, body = API.chat(fall, {"text": "Was sind Arbeitstage?", "feld_id": "ep_arbeitstage"})
    assert st == 501 and body["fehler"] == "not_implemented"


def test_kaputte_antwort_verliert_die_antwort_nicht_die_vorschlaege(fall, monkeypatch):
    """Asymmetrie mit Absicht: `_chat_parse` und `_antwort_parse` lesen dieselbe Antwort getrennt.
    Ein Modell, das die Vorschläge sauber liefert und beim Antworttext patzt, soll nicht beide
    Hälften verlieren — und umgekehrt ist ein halb geparster Satz keine Erklärung."""
    behalten, _ = api_llm._beleg_geprueft(api_llm._chat_parse('{"vorschlaege": []'), "x")
    assert behalten == []
    assert api_llm._antwort_parse('{"vorschlaege": []') == ("", False)
    assert api_llm._antwort_parse('{"vorschlaege": [], "antwort": "hallo", "unsicher": true}') \
        == ("hallo", True)


def test_beleg_gate_ist_im_dialog_pfad_verdrahtet(fall, monkeypatch):
    """Gefunden durch eine Mutationsprobe: `_beleg_geprueft` abzuschalten machte KEINEN Test rot.
    tests/test_chat_beleg_gate.py prüft die Funktion — dass sie im Vorschlags-Pfad auch AUFGERUFEN
    wird, prüfte nichts. Genau die Bauart, die hier mehrfach auffiel: zwei Hälften einzeln grün,
    die Übergabe dazwischen ungetestet.

    Die Probe: ein Vorschlag, dessen Beleg im Text nicht vorkommt. Er darf weder in der Antwort
    auftauchen noch geschrieben werden."""
    monkeypatch.setattr(LC, "complete", _fixture_complete(vorschlaege=[
        _v("ep_arbeitstage", 220, "an 220 Tagen"),                 # steht im Text
        _v("bruttoarbeitslohn", 6200000, "62000 Euro brutto"),     # steht NICHT im Text
    ]))
    st, body = API.chat(fall, {"text": "Ich fahre an 220 Tagen zur Arbeit."})
    assert st == 200
    assert [v["feld_id"] for v in body["vorschlaege"]] == ["ep_arbeitstage"], (
        "Der unbelegte Vorschlag kam durch — das Beleg-Gate hängt nicht im Pfad.")
    assert "bruttoarbeitslohn" not in ST._aktives(API.lade_fall(fall)), (
        "Der unbelegte Vorschlag wurde sogar geschrieben.")


def test_schema_erzwingt_beide_haelften():
    s = api_llm.DIALOG_SCHEMA
    assert s.get("strict") is True
    # 2026-08-21 um `rueckfragen` erweitert (Drei-Stufen-Dialog). Die Begründung dieses Tests gilt
    # unverändert und trägt die dritte Hälfte mit: gemessen liess das Modell die Liste in FÜNFZEHN
    # Läufen leer, solange sie optional war, und füllte sie in 3 von 3, sobald sie verpflichtend
    # wurde. Ein optionales Feld ist eines, das man weglässt.
    assert set(s["schema"]["required"]) == {"vorschlaege", "rueckfragen", "antwort", "unsicher"}, (
        "Fehlt eine Hälfte in `required`, hängt die Struktur davon ab, ob das Modell den Text für "
        "eine Frage hielt.")
    assert s["schema"]["additionalProperties"] is False
    assert s["schema"]["properties"]["unsicher"]["type"] == "boolean"


def test_prompt_nennt_beide_aufgaben_und_verbietet_hypothesen():
    """Die Prompt-Regeln, die das Beleg-Gate NICHT abdecken kann. Ein wörtliches Zitat aus einer
    hypothetischen Frage besteht den Filter — deshalb muss die Grenze im Prompt stehen."""
    system = api_llm._dialog_prompt("egal", [{"feld_id": "x", "fragetext_laie": "y", "typ": "cent"}],
                                    "Kontext-Zeile")[0]["content"]
    assert "AUFGABE 1" in system and "AUFGABE 2" in system
    assert "HYPOTHESEN" in system.upper()
    assert "Kontext-Zeile" in system, "Der Kontext kommt nicht im Prompt an"
    assert "LEERER String" in system, "Ohne diese Regel erfindet das Modell eine Belehrung"
    # Jeder Zurückhaltungs-Regel muss die Regel gegenüberstehen, die den Normalfall benennt —
    # sonst färbt die Vorsicht ab (gemessen 2026-08-14: ein „Im Zweifel weglassen" drückte acht
    # Vorschläge auf einen). Mit der Zusammenlegung stehen zwei Vorsichtsregeln nebeneinander.
    assert "TATSACHEN ÜBERSETZT DU IMMER" in system, (
        "Die Gegenregel zum Normalfall fehlt — zwei Zurückhaltungs-Regeln ohne Gegengewicht.")


# ---------------------------------------------------------------- Kontext für die Antwort
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


def test_kontext_geht_wirklich_in_den_prompt(fall, monkeypatch):
    """Naht-Prüfung: der Kontext wird gebaut UND gesendet. Beide Hälften einzeln grün und die
    Übergabe dazwischen ungetestet ist genau die Bauart, die hier schon mehrfach auffiel."""
    _bestaetige(fall, "ep_arbeitstage", 220)
    stub = _fixture_complete(antwort=ANTWORT)
    monkeypatch.setattr(LC, "complete", stub)
    API.chat(fall, {"text": "Was zählt zu den Arbeitstagen?", "feld_id": "ep_arbeitstage"})
    system = stub.gesehen["messages"][0]["content"]
    assert "bereits bestätigt" in system and "220" in system, (
        f"Der Kontext kam nicht im Prompt an:\n{system[:400]}")
    assert stub.gesehen["schema"] is api_llm.DIALOG_SCHEMA, (
        "Ohne Schema sendet der Client json_object — dann fehlt die erzwungene Struktur.")


# ---------------------------------------------------------------- Browser
@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    monkeypatch.setattr(LC, "complete", _fixture_complete(antwort=ANTWORT, unsicher=True))
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
def test_berater_ist_ohne_klick_da_mit_genau_einem_knopf(seite):
    """„Die KI sollte immer offen sein, sodass man ihr ständig einfach auch was hinschmeißen
    kann." Und genau ein Absende-Knopf — zwei hießen, den eigenen Satz vorher einzusortieren."""
    page = seite
    assert page.is_visible("#berater"), "Der Berater ist nicht sichtbar."
    assert page.is_visible("#chat-text"), "Das Eingabefeld ist nicht erreichbar."
    assert page.query_selector("#chat-overlay") is None, (
        "Das Chat-Modal existiert noch — es lag zuletzt über der ganzen Oberfläche.")
    assert page.query_selector("#chat-frage") is None, "Es gibt noch einen zweiten Knopf."
    assert page.is_visible("#wegpunkt"), "Der Fragefluss ist verdeckt."


@braucht_browser
def test_erklaer_mir_erklaert_sofort_und_ohne_modell(seite):
    """Der Kern von Julius' Beschwerde: der Knopf öffnete nur ein Eingabefeld. Jetzt steht die
    Erklärung sofort da — Fragetext, Kurzhilfe und der wörtliche Gesetzestext. Ohne LLM-Aufruf:
    diese drei Stücke liegen aus /fragen bereits vor."""
    page = seite
    page.click("#chat")
    page.wait_for_selector(".chat-frage-titel", timeout=5000)
    assert page.text_content(".chat-frage-titel").strip(), "Kein Fragetext in der Erklärung"
    assert page.query_selector(".chat-gesetz") is not None, (
        "Kein Gesetzestext in der Erklärung — der Zitatanker fehlt.")
    assert "§" in page.text_content(".chat-gesetz")


@braucht_browser
def test_rueckfrage_nach_erklaer_mir_bleibt_im_selben_verlauf(seite):
    """„erklär mir sollte aber rückfragen erlauben." Die Erklärung bleibt stehen, die Rückfrage und
    ihre Antwort kommen darunter — nur so sieht der Nutzer, worauf er sich bezieht."""
    page = seite
    page.click("#chat")
    page.wait_for_selector(".chat-frage-titel", timeout=5000)
    page.fill("#chat-text", "Und zählt Homeoffice dazu?")
    page.click("#chat-send")
    page.wait_for_selector(".chat-antwort", timeout=5000)
    assert page.query_selector(".chat-frage-titel") is not None, (
        "Die Erklärung wurde von der Rückfrage überschrieben — der Bezug ist weg.")
    assert page.query_selector(".chat-du") is not None, "Die eigene Frage steht nicht im Verlauf."
    assert "Arbeitstage sind" in page.text_content(".chat-antwort")
    assert page.query_selector(".chat-unsicher") is not None, (
        "unsicher=true wird dem Nutzer nicht angezeigt.")
    assert page.input_value("#chat-text") == "", "Das Eingabefeld wurde nicht geleert."


@braucht_browser
def test_frage_veraendert_den_fall_nicht(seite):
    """Dieselbe Grenze wie serverseitig, hier durch die ganze Kette bis zur Oberfläche."""
    page = seite
    vorher = page.evaluate("async () => Object.keys((await jget(`/fall/${FALL}/stand`)).body.felder)")
    page.fill("#chat-text", "Trag mir bitte 220 Arbeitstage und 62000 Euro Bruttolohn ein.")
    page.click("#chat-send")
    page.wait_for_selector(".chat-antwort", timeout=5000)
    nachher = page.evaluate("async () => Object.keys((await jget(`/fall/${FALL}/stand`)).body.felder)")
    assert nachher == vorher, f"Der Fall hat sich verändert: {set(nachher) - set(vorher)}"
