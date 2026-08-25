"""Jeder Bindungstyp muss ein Eingabefeld bekommen, das seinen eigenen Wert annimmt.

ANLASS, gemessen 2026-08-24 im Live-Lauf des KI-Wegs: die Rückfrage „Wie heissen deine Kinder mit
Vornamen?" stand über einem `<input type="number">` mit dem Platzhalter „Anna". `baueEingabe()`
kannte drei Zweige — `bool`, `enum`, und „alles andere ist eine Zahl". In „alles andere" lagen
neben `cent` und `int` auch **56 `text`- und 5 `datum`-Felder** der Bindung. Ein Zahlenfeld nimmt
keine Buchstaben an: der Nutzer konnte tippen, was er wollte, das Feld blieb leer. Der automatische
Durchlauf trug `1` ein und bekam vom Server HTTP 422 — sichtbar nur in der Browser-Konsole.

Der Fehler traf Fragebogen UND Rückfragen-Wizard, denn beide rufen dieselbe Funktion
(`zeigeFrage()` / `zeigeRueckfrage()`). Er war seit dem Bau dieser Funktion da, und 2532 grüne
Tests haben ihn nicht bemerkt: die UI-Tests wählen ihre Felder nach Bedarf, und gebraucht wurden
immer `bool`, `cent` oder `enum`.

WARUM DIESER TEST BEIDE SEITEN MISST. Ein Eingabefeld, das einen Wert annimmt, den der Store
danach mit 422 abweist, ist nicht repariert — es ist nur anders kaputt. Deshalb läuft jeder Fall
durch beide Hälften der Naht:

    Browser: baueEingabe(q, ...) -> Feld -> Nutzer tippt -> leseWert(q, feld) -> Wert
    Server:  store._typ_konform(Wert, typ, enum_werte) -> muss True sein

Genau dazwischen lag der Datums-Fallstrick: `<input type="date">` liefert IMMER ISO (2025-07-15),
der Store verlangt TT.MM.JJJJ (`store.py:_typ_konform`, amtliches ELSTER-Format, im XSD verankert).
Ein Kalenderfeld ohne Umrechnung hätte JEDE Datumseingabe abgewiesen — das wäre derselbe Fehler
gewesen, nur eine Ebene weiter.

KEIN LLM, KEIN /chat: hier wird nur die Feld-Naht gemessen.
"""
from __future__ import annotations

import os
import sys
import threading

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import audit             # noqa: E402
import server as SRV     # noqa: E402
import store as STORE    # noqa: E402
from ui_hilfen import zum_fragebogen  # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None

pytestmark = pytest.mark.skipif(sync_playwright is None, reason="playwright fehlt")


@pytest.fixture
def base(tmp_path, monkeypatch):
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
def page(base):
    with sync_playwright() as p:
        browser = p.chromium.launch()
        seite = browser.new_page(viewport={"width": 1280, "height": 900})
        seite.goto(base)
        seite.wait_for_load_state("networkidle")
        # Bis in den Fluss: #eingabe existiert erst dort, und baueEingabe schreibt dort hinein.
        seite.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        seite.wait_for_selector("#weg-fragebogen", timeout=5000).click()
        zum_fragebogen(seite)   # Ankreuzliste am Anfang, s. tests/ui_hilfen.py
        yield seite
        browser.close()


# Ein Fall je Bindungstyp: (typ, was der Nutzer eingibt, wie er es eingibt).
# Die Werte sind echte Beispielwerte aus produkt/bindung/ — „Anna" ist der beispielwert von
# kind_vorname, „05.05.1955" der von stammdaten_geburtsdatum.
FAELLE = [
    ("cent",  {"typ": "cent"},                                  "620.50",    62050),
    ("int",   {"typ": "int"},                                   "220",         220),
    ("text",  {"typ": "text", "beispielwert": "Anna"},          "Anna",     "Anna"),
    ("datum", {"typ": "datum", "beispielwert": "05.05.1955"},   "1955-05-05",
                                                                       "05.05.1955"),
]


@pytest.mark.parametrize("name,q,eingabe,erwartet", FAELLE, ids=[f[0] for f in FAELLE])
def test_jeder_typ_nimmt_seinen_wert_an_und_der_store_akzeptiert_ihn(page, name, q, eingabe,
                                                                     erwartet):
    """Die ganze Naht in einem Durchgang: Feld bauen, Wert eintragen, auslesen, Store fragen."""
    q = {"feld_id": f"pruef_{name}", "frage_laie": "Prüffrage", **q}
    ergebnis = page.evaluate("""([q, eingabe]) => {
      const box = document.getElementById('eingabe');
      const el = baueEingabe(q, box, 'feld-input', 'frage', null);
      // Genau der Weg des Nutzers: hineinschreiben, was er tippt.
      el.value = eingabe;
      return {art: el.type || el.tagName.toLowerCase(),
              angekommen: el.value,
              gelesen: leseWert(q, el)};
    }""", [q, eingabe])

    assert ergebnis["angekommen"] == eingabe, (
        f"Das Feld für `{name}` hat die Eingabe {eingabe!r} nicht angenommen (Feldart: "
        f"{ergebnis['art']!r}, drin steht {ergebnis['angekommen']!r}). Ein <input type=number> "
        f"verwirft Buchstaben stillschweigend — genau so verschwand „Anna“.")
    assert ergebnis["gelesen"] == erwartet, (
        f"leseWert() liest {ergebnis['gelesen']!r} statt {erwartet!r}.")
    assert STORE._typ_konform(ergebnis["gelesen"], name, None), (
        f"Der Store weist {ergebnis['gelesen']!r} für typ={name} ab (HTTP 422) — das Feld nimmt "
        f"den Wert an, aber der Server nicht. Halb repariert ist nicht repariert.")


def test_text_ist_kein_zahlenfeld(page):
    """Der Befund selbst, so eng wie möglich: `text` DARF nicht als Zahlenfeld gebaut werden.

    Der Test darüber misst die Wirkung, dieser die Ursache — er bliebe auch dann rot, wenn jemand
    das Zahlenfeld beibehielte und die Buchstaben anderswo hineinschmuggelte."""
    art = page.evaluate("""() => {
      const el = baueEingabe({feld_id: 'x', typ: 'text', beispielwert: 'Anna'},
                             document.getElementById('eingabe'), 'feld-input', 'frage', null);
      return el.type;
    }""")
    assert art != "number", "`text` wird wieder als Zahlenfeld gebaut."
    assert art == "text", f"Unerwartete Feldart für `text`: {art!r}"


def test_datum_wird_in_das_amtliche_format_uebersetzt(page):
    """`<input type=\"date\">` liefert IMMER ISO — unabhängig davon, wie der Browser es anzeigt.
    Der Store verlangt TT.MM.JJJJ und weist ISO mit 422 ab (store.py, mit Begründung: amtliches
    ELSTER-Format, im XSD verankert, nichts in der Pipeline konvertiert).

    Die Umrechnung ist deshalb keine Kosmetik, sondern die Bedingung dafür, dass ein Kalenderfeld
    überhaupt benutzbar ist."""
    r = page.evaluate("""() => {
      const q = {feld_id: 'x', typ: 'datum'};
      const el = baueEingabe(q, document.getElementById('eingabe'), 'feld-input', 'frage', null);
      el.value = '2025-07-15';
      const iso = leseWert(q, el);
      // Und die Gegenrichtung: ein Feld, in dem schon deutsch steht, darf nicht kaputtgehen.
      const el2 = document.createElement('input');
      el2.value = '15.07.2025';
      return {aus_kalender: iso, aus_text: leseWert(q, el2),
              leer: leseWert(q, Object.assign(document.createElement('input'), {value: ''})),
              muell: leseWert(q, Object.assign(document.createElement('input'), {value: 'morgen'}))};
    }""")
    assert r["aus_kalender"] == "15.07.2025", (
        f"ISO wurde nicht übersetzt: {r['aus_kalender']!r} — der Store antwortet darauf mit 422.")
    assert r["aus_text"] == "15.07.2025", f"Deutsches Datum kaputtgemacht: {r['aus_text']!r}"
    # Leer und Unsinn schreiben NICHTS (Stille-Null-Regel): eine 0 oder eine kaputte Zeichenkette
    # wäre hier so falsch wie ein stiller Nullbetrag bei einem Geldfeld.
    assert r["leer"] is None, f"Leeres Datumsfeld liefert {r['leer']!r} statt nichts."
    assert r["muell"] is None, f"„morgen“ liefert {r['muell']!r} statt nichts."


def test_beim_korrigieren_steht_der_alte_wert_wieder_da(page):
    """`baueEingabe` bekommt beim Korrigieren eines belegten Feldes den gespeicherten Wert
    (korrigiereBestaetigt → zeigeFrage → baueEingabe mit `vorbelegung`). Die neuen Zweige haben
    ihn zuerst ignoriert: der Nutzer hätte einen Namen komplett neu tippen müssen, um einen
    Buchstaben zu ändern.

    Beim Datum läuft dieselbe Umrechnung rückwärts: gespeichert ist TT.MM.JJJJ, das Kalenderfeld
    will ISO. Ohne sie stünde es leer da — und ein leeres Feld beim Korrigieren sieht aus, als
    wäre der alte Wert weg."""
    r = page.evaluate("""() => {
      const box = document.getElementById('eingabe');
      const t = baueEingabe({feld_id: 'x', typ: 'text'}, box, 'feld-input', 'frage', 'Anna');
      const wert_text = t.value;
      const d = baueEingabe({feld_id: 'y', typ: 'datum'}, box, 'feld-input', 'frage', '05.05.1955');
      return {text: wert_text, datum_im_feld: d.value,
              datum_gelesen: leseWert({feld_id: 'y', typ: 'datum'}, d)};
    }""")
    assert r["text"] == "Anna", f"Textfeld verliert die Vorbelegung: {r['text']!r}"
    assert r["datum_im_feld"] == "1955-05-05", (
        f"Das Kalenderfeld zeigt die Vorbelegung nicht: {r['datum_im_feld']!r}")
    # Und der Rundweg muss stimmen: rein wie gespeichert, raus wie gespeichert.
    assert r["datum_gelesen"] == "05.05.1955", (
        f"Rundweg kaputt — hinein 05.05.1955, heraus {r['datum_gelesen']!r}")


ZEITRAUM_FELD = "kind_wohnsitz_inland_zeitraum"


def test_ein_wert_gegen_das_muster_wird_nicht_geschrieben(page):
    """ANLASS, gemessen in Julius' Durchgang 2026-08-25: im Fall stand

        kind_wohnsitz_inland_zeitraum: "01.01-31.122"

    Ein Tippfehler — eine 2 zu viel. `typ: text` heisst „beliebiger String", also nahm ihn niemand
    krumm. Der Wert geht später als Zeitraum ins ELSTER-Feld; aufgefallen wäre er beim Finanzamt.

    Die Bindung sagt seitdem ein `muster` zu. Hier wird die Oberflächen-Hälfte gemessen: ein Wert,
    der nicht passt, ergibt `undefined` — und `undefined` schreibt NICHTS (dieselbe Regel wie beim
    Stille-Null-Fix)."""
    q = {"feld_id": ZEITRAUM_FELD, "typ": "text",
         "muster": r"^\d{2}\.\d{2}-\d{2}\.\d{2}$", "standardwert": "01.01-31.12"}
    r = page.evaluate("""(q) => {
      const box = document.getElementById('eingabe');
      const el = baueEingabe(q, box, 'feld-input', 'frage', null);
      const lies = v => { el.value = v; return leseWert(q, el); };
      return {art: el.type, pattern: el.getAttribute('pattern'),
              gut: lies('01.01-31.12'), julius: lies('01.01-31.122'),
              leer: lies(''), quatsch: lies('immer')};
    }""", q)
    assert r["gut"] == "01.01-31.12", f"Der gültige Wert kommt nicht durch: {r['gut']!r}"
    assert r["julius"] is None, (
        f"„01.01-31.122“ wird immer noch angenommen: {r['julius']!r}")
    assert r["quatsch"] is None and r["leer"] is None, f"{r}"
    assert r["pattern"], "Das Feld trägt kein HTML-`pattern` — der Browser warnt nicht von selbst."


def test_der_store_weist_einen_wert_gegen_das_muster_ab():
    """Die andere Hälfte, und die entscheidende: dieser Pfad trägt JEDEN Schreiber — HTTP,
    Beleg-Import, Kontoauszug, eDaten. Ein Muster, das nur der Browser kennt, gilt für die anderen
    vier nicht."""
    import traverser as TR
    bindung = TR.lade_bindung()
    assert bindung[ZEITRAUM_FELD].get("muster"), (
        f"{ZEITRAUM_FELD} hat gar kein Muster in der Bindung — dann misst dieser Test nichts.")

    # `leerer_store` ist die Fabrik (erst `neuer_store` geraten — die gibt es nicht, und der Test
    # übersprang sich selbst; ein Skip ist ein Test, der nichts misst).
    store = STORE.leerer_store(veranlagungszeitraum=2025, fall_id="format-probe")
    for wert, soll_gehen in (("01.01-31.122", False), ("immer", False), ("01.01-31.12", True)):
        try:
            STORE.append_event(store, feld_id=ZEITRAUM_FELD, wert=wert, zustand="bestaetigt",
                               herkunft={"herkunft": "laie", "pruef_tiefe": "ungeprueft",
                                         "haftung": "nutzer"},
                               schreiber="test", signal={"signal_1": None, "signal_2": "t"},
                               bindung=bindung)
            ging = True
        except ValueError as e:
            ging = False
            assert "Format" in str(e), f"Falsche Fehlermeldung für {wert!r}: {e}"
        assert ging is soll_gehen, (
            f"{wert!r}: erwartet {'angenommen' if soll_gehen else 'abgewiesen'}, war das Gegenteil.")
        if ging:
            break        # Auflage B: ein zweites Event am selben Feld bräuchte `ersetzt`


def test_der_uebliche_wert_laesst_sich_mit_einem_klick_uebernehmen(page):
    """Julius 2026-08-25: „wenn in 95% der default fall eintritt sollte man evtl einen button
    ergänzen der diesen default wert übernimmt."

    Es sind die Zeitraum-Felder, deren eigene Hilfe „Meist das ganze Jahr" sagt — den Nutzer
    01.01-31.12 abtippen zu lassen ist Arbeit ohne Ertrag."""
    q = {"feld_id": ZEITRAUM_FELD, "typ": "text",
         "muster": r"^\d{2}\.\d{2}-\d{2}\.\d{2}$", "standardwert": "01.01-31.12"}
    r = page.evaluate("""(q) => {
      const box = document.getElementById('eingabe');
      const el = baueEingabe(q, box, 'feld-input', 'frage', null);
      const b = box.querySelector('.standardwert');
      if (!b) return {knopf: false};
      b.click();
      return {knopf: true, text: b.textContent, wert: el.value, gelesen: leseWert(q, el)};
    }""", q)
    assert r["knopf"], "Kein Knopf für den üblichen Wert."
    assert "01.01-31.12" in r["text"], f"Der Knopf sagt nicht, was er einträgt: {r['text']!r}"
    assert r["gelesen"] == "01.01-31.12", (
        f"Der übernommene Wert kommt nicht durch die Prüfung: {r}")


def test_ohne_standardwert_kein_knopf(page):
    """Die Gegenrichtung, und sie ist hier keine Formsache: `beispielwert` steht an fast JEDEM Feld.
    Würde der Knopf daraus gebaut, stünde unter „Wie hoch war dein Bruttoarbeitslohn?" ein Knopf
    „Üblichen Wert übernehmen: 62.000 €" — eine Vorgabe, die der Nutzer womöglich anklickt."""
    da = page.evaluate("""() => {
      const box = document.getElementById('eingabe');
      baueEingabe({feld_id: 'bruttoarbeitslohn', typ: 'cent', beispielwert: 6200000},
                  box, 'feld-input', 'frage', null);
      return !!box.querySelector('.standardwert');
    }""")
    assert not da, ("Ein Geldfeld bekommt einen „üblichen Wert“ angeboten — der stammt dann aus "
                    "`beispielwert`, und der ist ein Beispiel, keine Vorgabe.")


def test_die_bindung_enthaelt_diese_typen_wirklich():
    """Gegen den stillen Leerlauf: würden `text`/`datum` in der Bindung gar nicht vorkommen, wäre
    die ganze Datei ein Test über einen Fall, den es nicht gibt — grün und wertlos.

    Die Zahlen sind die vom 2026-08-24 gemessenen; sie dürfen wachsen, aber nicht auf null fallen.
    """
    import pathlib
    import re
    texte = " ".join(p.read_text(encoding="utf-8")
                     for p in pathlib.Path(ROOT, "produkt", "bindung").glob("*.yaml"))
    zahl = {t: len(re.findall(rf"^\s*typ:\s*{t}\s*$", texte, re.M)) for t in ("text", "datum")}
    assert zahl["text"] >= 50, f"Nur noch {zahl['text']} text-Felder — Messung prüfen: {zahl}"
    assert zahl["datum"] >= 5, f"Nur noch {zahl['datum']} datum-Felder — Messung prüfen: {zahl}"
