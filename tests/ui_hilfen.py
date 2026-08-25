"""Gemeinsame Wege durch die Oberfläche, die mehrere Testdateien brauchen.

Entstanden 2026-08-25 mit der Ankreuzliste: seit der Fragebogen mit ihr beginnt, kommt zwischen
der Wegwahl und der ersten Frage ein Bildschirm dazu. 23 Fixtures in 15 Dateien warteten direkt
nach dem Wegwahl-Klick auf `#wegpunkt` und liefen alle in denselben Timeout.

Die Alternative wäre gewesen, in jede dieser Fixturen dieselben drei Zeilen zu kopieren — und beim
nächsten Schritt im Fluss dieselbe Arbeit noch einmal. Ein Weg, eine Stelle.
"""
from __future__ import annotations


def zum_fragebogen(page, ankreuzen=None, timeout: int = 8000):
    """Von der Wegwahl bis zur ersten echten Frage.

    ZWEI MODI, und die Vorgabe ist mit Absicht der zweite:

    `ankreuzen=None` (Vorgabe) — die Ankreuzliste wird WEGGERÄUMT, OHNE zu antworten. Die zehn
    Screening-Felder bleiben damit offen und stehen wie bisher im Fragebogen. Das ist der Modus für
    alle Tests, deren Gegenstand der Fragefluss ist und nicht die Liste: sie prüfen Polarität,
    Bestätigungen, Fortschritt, Rechenweg — und ihre Prüffelder sind zum Teil genau diese zehn.
    Beantwortete die Hilfe sie, hätte ein Testhilfsmittel den Prüfgegenstand entfernt.

    `ankreuzen=[...]` (auch `[]`) — die Liste wird ECHT bedient: die genannten Felder angekreuzt,
    die übrigen verneint, „Weiter" geklickt. Das ist der Weg des Nutzers; `tests/
    test_ui_screening_liste.py` misst ihn vollständig.

    Die Liste wird in beiden Fällen OPTIONAL behandelt: sind alle Screening-Felder schon
    beantwortet, erscheint sie gar nicht, und der Test soll trotzdem weiterlaufen.
    """
    da = page.query_selector("#screening") and not page.is_hidden("#screening")
    if not da:
        # Sie kann noch im Kommen sein (zeigeScreening holt erst /fragen). Kurz darauf warten,
        # aber nicht darauf bestehen.
        try:
            page.wait_for_selector("#screening:not([hidden])", timeout=1500)
            da = True
        except Exception:
            da = False

    if da and ankreuzen is None:
        # Wegräumen ohne zu schreiben. Kein Produktionsweg — es gibt in der Oberfläche keinen
        # Knopf dafür, und das ist richtig so: der Nutzer soll die Liste beantworten.
        page.evaluate("""() => {
          SCREENING_OFFEN = false;
          document.getElementById('screening').hidden = true;
          return refresh();
        }""")
    elif da:
        for feld in ankreuzen:
            box = page.query_selector(f'#screening-liste .sc-box[data-feld="{feld}"]')
            assert box is not None, (
                f"{feld} steht nicht in der Ankreuzliste — dann kann der Test es dort nicht "
                f"bejahen. Ist es noch ein `screening`-Feld?")
            box.check()
        page.click("#screening-weiter")
        page.wait_for_selector("#screening", state="hidden", timeout=timeout)

    page.wait_for_selector("#wegpunkt:not([hidden])", timeout=timeout)
    return page
