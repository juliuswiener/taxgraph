"""„Das habe ich verstanden" — jeder KI-Vorschlag mit seinem Zitat, einzeln zu bestätigen.

Julius 2026-08-14: „dann jedes Mal, wenn der Nutzer Daten [gibt], die KI das übersetzen in Felder
und dann eine Seite anzeigen und dem Nutzer sagen, okay, das habe ich jetzt verstanden … soll ich
das so eintragen? Und dann schon, dass der Nutzer das dann nochmal bestätigt."

Vorher verschwanden die Vorschläge im Fragefluss: der Nutzer schrieb einen Satz, bekam „4
Vorschläge gemacht" zu lesen und traf sie danach einzeln und ohne Zusammenhang wieder. Jetzt
stehen sie auf einer Seite, jeder mit dem Satzteil, aus dem er stammt.

Drei Dinge, die hier schiefgehen können und deshalb geprüft werden:

  1. **Die Anzeige lügt.** Der Store führt `bruttoarbeitslohn = 6200000` und `kein_kap = true`.
     Ungefiltert angezeigt heißt das für den Nutzer „6200000" und — unter der positiv gestellten
     Frage „Hattest du Kapitalerträge?" — ein „Ja", das im Store das Gegenteil bedeutet. Er
     bestätigt hier per Klick, was er liest; eine falsche Anzeige ist damit eine falsche Erklärung.
  2. **Bestätigen wirkt auf zu viel.** „Einzeln" ist der ganze Punkt der Seite. Ein Klick auf
     „Stimmt" darf genau ein Feld bestätigen, der Rest bleibt vorläufig — und damit außerhalb
     jeder Summe.
  3. **Die Seite schiebt sich selbst weg.** `refresh()` zeigt sonst nach jedem Schreibvorgang die
     nächste Frage. Ohne Sperre säße der Nutzer nach der ersten Bestätigung im Fragefluss und
     seine restliche Liste wäre fort.

NULL LLM: `_llm_dialog` ist durch eine Fixture-Funktion ersetzt (kein Netz, kein Mock-Framework).
Geprüft wird alles NACH dem Modellaufruf — Anzeige, Bestätigung, Schreibpfad.
"""
from __future__ import annotations

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
import server as SRV     # noqa: E402
import store as ST       # noqa: E402
from ui_hilfen import zum_fragebogen  # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

braucht_browser = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")

TEXT = ("Ich bin verheiratet, fahre an 220 Tagen zur Arbeit, habe 62000 Euro brutto verdient "
        "und hatte keine Kapitalerträge.")

# Was das Modell aus TEXT gemacht hätte — vier Typen, damit jede Anzeigeregel einmal vorkommt:
# enum (veranlagung), Zahl ohne Einheit (ep_arbeitstage), cent (bruttoarbeitslohn) und das
# invertierte bool (kein_kap).
VORSCHLAEGE = [
    {"feld_id": "veranlagung", "wert": "zusammen", "beleg": "verheiratet",
     "begruendung": "Ehe → Zusammenveranlagung möglich"},
    {"feld_id": "ep_arbeitstage", "wert": 220, "beleg": "an 220 Tagen",
     "begruendung": "Arbeitstage direkt genannt"},
    {"feld_id": "bruttoarbeitslohn", "wert": 6200000, "beleg": "62000 Euro brutto",
     "begruendung": "Bruttolohn in Euro, als Cent übernommen"},
    {"feld_id": "kein_kap", "wert": True, "beleg": "keine Kapitalerträge",
     "begruendung": "Abwesenheit ausdrücklich genannt"},
]


@pytest.fixture
def stub_llm(monkeypatch):
    """Ersetzt den Modellaufruf, nicht den Rest: Katalog-Check, Auflage A/B, Store-Schreibpfad und
    die Anreicherung mit Anzeige-Metadaten laufen echt."""
    monkeypatch.setattr(api_llm, "_llm_dialog",
                        lambda freitext, katalog, kontext="", user_id=None: {
                            "vorschlaege": [dict(v) for v in VORSCHLAEGE],
                            "antwort": "", "unsicher": False})


@pytest.fixture
def base(tmp_path, monkeypatch, stub_llm):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
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
    """Fall angelegt, Chat abgeschickt, Verstanden-Seite offen — der Zustand, um den es geht."""
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        zum_fragebogen(page)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
        page.fill("#chat-text", TEXT)   # der Berater ist dauerhaft offen, nichts aufzuklappen
        page.click("#chat-send")
        page.wait_for_selector("#verstanden:not([hidden])", timeout=5000)
        yield page
        browser.close()


def _stand(page) -> dict:
    return page.evaluate("async () => (await jget(`/fall/${FALL}/stand`)).body")


# ---------------------------------------------------------------- Server: die Anzeige-Metadaten
def test_chat_liefert_anzeige_metadaten(tmp_path, monkeypatch, stub_llm):
    """Ohne `frage`/`typ`/`einheit`/`enum_labels` in der Antwort kann die Oberfläche nur feld_id
    und Rohwert zeigen — genau die zwei Dinge, die ein Laie nicht lesen kann. Die Metadaten kommen
    aus derselben Bindung wie in /fragen, damit beide Seiten dasselbe sagen."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    st, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "v1"})
    assert st == 201
    st, body = API.chat("v1", {"text": TEXT})
    assert st == 200
    nach_feld = {v["feld_id"]: v for v in body["vorschlaege"]}
    assert set(nach_feld) == {v["feld_id"] for v in VORSCHLAEGE}, (
        f"Nicht alle Vorschläge wurden geschrieben: {sorted(nach_feld)}")

    lohn = nach_feld["bruttoarbeitslohn"]
    assert lohn["typ"] == "cent", "ohne typ weiß die Oberfläche nicht, dass 6200000 Cent sind"
    assert lohn["frage"], "fragetext_laie fehlt — die Zeile hätte nur die feld_id als Überschrift"
    assert lohn["beleg"] == "62000 Euro brutto"
    assert lohn["event_id"], "ohne event_id kann die Seite den Vorschlag nicht ersetzen (Auflage B)"

    ver = nach_feld["veranlagung"]
    assert ver["enum_labels"] and ver["enum_labels"].get("zusammen"), (
        f"enum_labels fehlen — der Nutzer läse den Rohwert 'zusammen': {ver.get('enum_labels')!r}")


def test_vorschlaege_bleiben_vorlaeufig(tmp_path, monkeypatch, stub_llm):
    """Die Grenze, die die Seite sichtbar macht: bis der Mensch klickt, zählt nichts. Der Test
    steht hier und nicht nur in den Store-Goldens, weil die Seite den Eindruck erweckt, es sei
    schon eingetragen — der Store muss dem widersprechen."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "v2"})
    API.chat("v2", {"text": TEXT})
    aktiv = ST._aktives(API.lade_fall("v2"))
    for v in VORSCHLAEGE:
        ev = aktiv[v["feld_id"]]
        assert ev["zustand"] == "vorlaeufig", f"{v['feld_id']} ist nicht vorläufig"
        assert ev["signal"]["signal_2"] is None, f"{v['feld_id']} kam mit zweitem Signal durch"


# ---------------------------------------------------------------- Browser: Anzeige
@braucht_browser
def test_seite_zeigt_jede_zeile_mit_ihrem_zitat(seite):
    """Das Zitat ist der Unterschied zwischen „bestätige 62.000 €" und „bestätige 62.000 €, weil
    du sagtest: 62000 Euro brutto". Der Nutzer prüft die Behauptung an seinem eigenen Satz."""
    page = seite
    felder = page.eval_on_selector_all(".v-zeile", "els => els.map(e => e.dataset.feld)")
    assert set(felder) == {v["feld_id"] for v in VORSCHLAEGE}, f"Zeilen: {felder}"
    belege = page.eval_on_selector_all(".v-beleg", "els => els.map(e => e.textContent)")
    assert len(belege) == len(VORSCHLAEGE), f"Nicht jede Zeile trägt ihren Beleg: {belege}"
    assert any("62000 Euro brutto" in b for b in belege), f"Zitat fehlt: {belege}"


@braucht_browser
def test_cent_wird_als_euro_gezeigt(seite):
    """6200000 ist die Speicherform. Wer das liest, kann nicht beurteilen, ob es stimmt."""
    page = seite
    wert = page.text_content(".v-zeile[data-feld='bruttoarbeitslohn'] .v-wert")
    assert "62.000,00" in wert, f"Cent-Rohwert steht auf der Seite: {wert!r}"


@braucht_browser
def test_enum_zeigt_den_anzeigetext(seite):
    page = seite
    wert = page.text_content(".v-zeile[data-feld='veranlagung'] .v-wert")
    assert "Zusammenveranlagung" in wert, f"Rohwert statt Anzeigetext: {wert!r}"


@braucht_browser
def test_kein_feld_wird_invertiert_angezeigt(seite):
    """Die gefährlichste Zeile der Seite. Gespeichert wird `kein_kap = true`; darüber steht die
    positiv gestellte Frage „Hattest du dieses Jahr Kapitalerträge?". Ein ungefiltertes „Ja"
    behauptete dort das Gegenteil des gespeicherten Werts — und der Nutzer bestätigt, was er
    liest. Dieselbe Inversion wie in leseWert(); sie muss an beiden Stellen gelten."""
    page = seite
    zeile = page.text_content(".v-zeile[data-feld='kein_kap'] .v-frage")
    assert "Kapitalerträge" in zeile, f"Unerwarteter Fragetext: {zeile!r}"
    wert = page.text_content(".v-zeile[data-feld='kein_kap'] .v-wert").strip()
    assert wert == "Nein", (
        f"'keine Kapitalerträge' steht als {wert!r} unter der Frage {zeile!r} — die kein_-"
        f"Inversion fehlt, die Seite sagt das Gegenteil des gespeicherten Werts.")


# ---------------------------------------------------------------- Browser: Bestätigen
@braucht_browser
def test_stimmt_bestaetigt_genau_ein_feld(seite):
    """„Einzeln" ist der ganze Punkt: ein Klick bewegt ein Feld über die Grenze, alle anderen
    bleiben vorläufig und damit außerhalb jeder Summe."""
    page = seite
    page.click(".v-zeile[data-feld='bruttoarbeitslohn'] .v-ok")
    page.wait_for_selector(".v-zeile[data-feld='bruttoarbeitslohn'].v-fertig", timeout=5000)
    felder = _stand(page)["felder"]
    assert felder["bruttoarbeitslohn"]["zustand"] == "bestaetigt"
    for fid in ("veranlagung", "ep_arbeitstage", "kein_kap"):
        assert felder[fid]["zustand"] == "vorlaeufig", (
            f"{fid} wurde mitbestätigt — ein Klick hat mehr als eine Zeile bewegt.")


@braucht_browser
def test_bestaetigen_schiebt_die_seite_nicht_weg(seite):
    """Ohne die VERSTANDEN_OFFEN-Sperre zeigt refresh() nach jedem Schreibvorgang die nächste
    Frage — der Nutzer säße nach der ersten Bestätigung im Fragefluss, seine Liste wäre fort. Die
    Sperre ist die einzige Stelle, die das verhindert."""
    page = seite
    page.click(".v-zeile[data-feld='veranlagung'] .v-ok")
    page.wait_for_selector(".v-zeile[data-feld='veranlagung'].v-fertig", timeout=5000)
    assert page.is_visible("#verstanden"), "Die Verstanden-Seite ist nach einer Bestätigung weg."
    assert not page.is_visible("#wegpunkt"), "Der Fragefluss hat sich davorgeschoben."
    assert len(page.query_selector_all(".v-zeile")) == len(VORSCHLAEGE), (
        "Die restlichen Zeilen sind verschwunden.")


@braucht_browser
def test_weiter_fuehrt_in_den_fragefluss_und_verliert_nichts(seite):
    """Nicht bestätigte Zeilen bleiben vorläufig — also weiter im Fragefluss, wo der Nutzer sie
    erneut trifft. „Weiter" darf keine stille Zustimmung sein und auch kein stilles Verwerfen."""
    page = seite
    page.click("#verstanden-weiter")
    # Hinter „Weiter zu den Fragen" liegt seit 2026-08-25 die Ankreuzliste — der Fragebogen
    # beginnt mit ihr, auch wenn man über die Bestätigungen dorthin kommt.
    zum_fragebogen(page)
    assert not page.is_visible("#verstanden")
    felder = _stand(page)["felder"]
    for v in VORSCHLAEGE:
        assert felder[v["feld_id"]]["zustand"] == "vorlaeufig", (
            f"{v['feld_id']} hat sich durch 'Weiter' verändert — das wäre stille Zustimmung.")
    offen = page.evaluate("async () => (await jget(`/fall/${FALL}/fragen`)).body.fragen"
                          ".map(q => q.feld_id)")
    assert "bruttoarbeitslohn" in offen, (
        "Der unbestätigte Vorschlag steht nicht mehr im Fragefluss — er wäre lautlos verloren.")


@braucht_browser
def test_aendern_oeffnet_das_feld_im_fragefluss(seite):
    """„Ändern" ist der Weg für einen falsch verstandenen Wert. Er verwirft nichts, sondern legt
    die Frage vor — der Nutzer korrigiert und bestätigt dort, was denselben Vorschlag ersetzt."""
    page = seite
    page.click(".v-zeile[data-feld='ep_arbeitstage'] .v-aendern")
    # Gewartet wird auf das ENDE der Handlung, und das ist das Verschwinden der Liste — nicht auf
    # `#wegpunkt`, das schon vorher sichtbar wird. Gemessen 2026-08-27 mit 350 ms je Netzaufruf:
    # auf `#wegpunkt` gewartet, stand hier `veranlagung` statt `ep_arbeitstage`.
    # `state="attached"`, nicht der Standard: ein verstecktes Element ist nie „sichtbar", der
    # Standard-Zustand wartet also auf etwas, das nie eintritt.
    page.wait_for_selector("#verstanden[hidden]", state="attached", timeout=8000)
    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)
    assert page.evaluate("AKTUELL && AKTUELL.feld_id") == "ep_arbeitstage"
    assert not page.is_visible("#verstanden")
    assert _stand(page)["felder"]["ep_arbeitstage"]["zustand"] == "vorlaeufig"


# ---------------------------------------------------------------- Konflikte
# Schlägt die KI etwas für ein Feld vor, das schon einen Wert trägt, darf sie es nicht
# überschreiben (Auflage B — höchstens ein aktives Event je Feld). Der Server meldete solche Fälle
# seit jeher in `konflikte`, die Oberfläche zeigte sie NIRGENDS: für den Nutzer sah es aus, als
# hätte die KI seine Angabe überhört. Julius 2026-08-14 als offener Punkt benannt.
KONFLIKT_TEXT = "Ich verdiene 70000 Euro brutto."


@pytest.fixture
def stub_llm_konflikt(monkeypatch):
    monkeypatch.setattr(api_llm, "_llm_dialog",
                        lambda freitext, katalog, kontext="", user_id=None: {
                            "vorschlaege": [{"feld_id": "bruttoarbeitslohn", "wert": 7000000,
                                             "beleg": "70000 Euro brutto", "begruendung": "genannt"}],
                            "antwort": "", "unsicher": False})


def test_konflikt_traegt_anzeige_metadaten(tmp_path, monkeypatch, stub_llm_konflikt):
    """Ein Konflikt zeigt ZWEI Werte nebeneinander. Ohne Anzeige-Metadaten stünde dort zweimal
    Speicherform — und genau hier muss der Nutzer zwei Zahlen vergleichen können."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "k1"})
    API.event("k1", {"feld_id": "bruttoarbeitslohn", "wert": 6200000, "zustand": "bestaetigt",
                     "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                     "schreiber": "ui:laie",
                     "signal": {"signal_1": None, "signal_2": "klick@bruttoarbeitslohn"}})
    st, body = API.chat("k1", {"text": KONFLIKT_TEXT})
    assert st == 200
    assert body["vorschlaege"] == [], "Ein belegtes Feld darf nicht überschrieben werden"
    assert len(body["konflikte"]) == 1
    k = body["konflikte"][0]
    assert k["aktueller_wert"] == 6200000 and k["vorschlag_wert"] == 7000000
    assert k["typ"] == "cent", "ohne typ zeigt die Oberfläche zwei Cent-Rohwerte"
    assert k["frage"], "ohne Fragetext weiß der Nutzer nicht, worum gestritten wird"
    assert k["aktuelles_event_id"], "ohne event_id kann die Übernahme nichts ersetzen"


@braucht_browser
def test_konflikt_erscheint_und_keine_seite_gewinnt_von_allein(base, stub_llm_konflikt):
    """Der Kern: beide Werte sichtbar, und ohne Klick ändert sich nichts. Vorher verschwand der
    Vorschlag lautlos — der Nutzer erfuhr nie, dass die KI etwas anderes verstanden hatte."""
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        zum_fragebogen(page)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
        page.evaluate("""async () => {
            await jpost(`/fall/${FALL}/event`, {
                feld_id: "bruttoarbeitslohn", wert: 6200000, zustand: "bestaetigt",
                herkunft: {herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer"},
                schreiber: "ui:laie",
                signal: {signal_1: null, signal_2: "klick@bruttoarbeitslohn"}});
        }""")
        page.fill("#chat-text", KONFLIKT_TEXT)
        page.click("#chat-send")
        page.wait_for_selector(".v-konflikt", timeout=5000)

        werte = page.eval_on_selector_all(".v-konflikt .v-seite-wert", "els => els.map(e => e.textContent)")
        assert any("62.000,00" in w for w in werte), f"bisheriger Wert fehlt: {werte}"
        assert any("70.000,00" in w for w in werte), f"Vorschlag fehlt: {werte}"

        stand = page.evaluate("async () => (await jget(`/fall/${FALL}/stand`)).body.felder")
        assert stand["bruttoarbeitslohn"]["wert"] == 6200000, (
            "Der Wert hat sich ohne Klick verändert — ein Konflikt darf sich nicht selbst auflösen.")

        page.click(".v-konflikt .v-uebernehmen")
        page.wait_for_selector(".v-konflikt.v-fertig", timeout=5000)
        stand2 = page.evaluate("async () => (await jget(`/fall/${FALL}/stand`)).body.felder")
        assert stand2["bruttoarbeitslohn"]["wert"] == 7000000, "Die Übernahme hat nicht gewirkt."
        assert stand2["bruttoarbeitslohn"]["zustand"] == "bestaetigt"
        browser.close()


@braucht_browser
def test_meins_behalten_schreibt_nichts(base, stub_llm_konflikt):
    """„Meins behalten" ist bewusst ein reiner Anzeige-Vorgang. Ein Event auf denselben Wert wäre
    ein zweites Signal, das der Nutzer nie gegeben hat."""
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        # P0b (2026-08-23): zwischen Fallart und Fluss liegt jetzt die Wegwahl (Fragebogen / erst KI).
        page.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        zum_fragebogen(page)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
        page.evaluate("""async () => {
            await jpost(`/fall/${FALL}/event`, {
                feld_id: "bruttoarbeitslohn", wert: 6200000, zustand: "bestaetigt",
                herkunft: {herkunft: "laie", pruef_tiefe: "ungeprueft", haftung: "nutzer"},
                schreiber: "ui:laie",
                signal: {signal_1: null, signal_2: "klick@bruttoarbeitslohn"}});
        }""")
        page.fill("#chat-text", KONFLIKT_TEXT)
        page.click("#chat-send")
        page.wait_for_selector(".v-konflikt", timeout=5000)
        vorher = page.evaluate("async () => (await jget(`/fall/${FALL}/stand`)).body.felder")
        page.click(".v-konflikt .v-ok")
        page.wait_for_selector(".v-konflikt.v-fertig", timeout=5000)
        nachher = page.evaluate("async () => (await jget(`/fall/${FALL}/stand`)).body.felder")
        assert nachher == vorher, 'Meins behalten hat etwas geschrieben.'
        browser.close()


@braucht_browser
def test_keine_horizontale_scrollbar_bei_360px(seite):
    """Dieselbe Falle wie bei den enum-Labels (gemessen scrollWidth 458 > 360): Zitate und
    Anzeigetexte sind beliebig lang und müssen umbrechen."""
    page = seite
    breite = page.evaluate("document.documentElement.scrollWidth")
    assert breite <= 360, f"scrollWidth={breite} > 360 — die Seite scrollt seitlich."
