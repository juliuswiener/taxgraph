"""Die Antwort des Nutzers muss den Wert ergeben, den seine Antwort bedeutet.

Ein bool-Feld kann eine ABWESENHEIT benennen (`kein_kap`, `vpf_keine_mahlzeitengestellung`),
während `fragetext_laie` nach der ANWESENHEIT fragt. Dann ist die Antwort des Nutzers die
Verneinung des gespeicherten Werts, und die Erfassungsschicht muss umkehren.

Bis 2026-08-20 entschied darüber eine Präfix-Heuristik an DREI Stellen unabhängig voneinander:

    leseWert()          app.js:394   q.feld_id.startsWith("kein_") ? !ja : ja      (Schreibpfad)
    verstandenWertText()app.js:499   dito                                          (Anzeige)
    _wert_klartext()    api.py:1078  fid.startswith("kein_")                       (KI-Kontext)

Die Heuristik traf `kein_kap` und brach bei jedem Feld, das die Verneinung in der MITTE des
Namens trägt. Gemessen wurden zwei solche Felder — beide Gates, beide an Geld:

    vpf_keine_mahlzeitengestellung  „Hat dir dein Arbeitgeber ... Mahlzeiten ... gestellt?"
    dhf_keine_pflicht_dienstwohnung „Ist deine Zweitwohnung ... eine Dienst- oder Werkswohnung ...?"

Euro-Wirkung, gemessen am echten HTTP-Ring (an_gesamt, VZ 2025, Kegel aus
test_paket_b_e2e_http.py) — BEIDE Richtungen sind falsch:

    vpf, 10 volle Reisetage (280 EUR Pauschale)
      Nutzer sagt „nein, keine Mahlzeiten" (der Normalfall)
        -> vorher gespeichert: false -> Ring GESPERRT, grund=verpflegung_reduktion_offen
           (der Nutzer bekommt überhaupt kein Ergebnis)
        -> richtig:            true  -> 654200 Cent
      Nutzer sagt „ja, Mahlzeiten gestellt"
        -> vorher gespeichert: true  -> 654200 Cent, volle Pauschale OHNE Kürzung
           = bis zu 8700 Cent (87,00 EUR) zu wenig Steuer  [Obergrenze: volle Kürzung = 662900]

    dhf, Miete 1400 EUR/Monat x 12, gekappt auf 1000 -> 12.000 EUR Abzug
      Nutzer sagt „nein, keine Pflicht-Dienstwohnung" (der Normalfall)
        -> vorher gespeichert: false -> 662900 Cent
        -> richtig:            true  -> 314300 Cent
        = 348600 Cent (3.486,00 EUR) ZU VIEL Steuer

Fix: die Umkehr ist jetzt in der Bindung DEKLARIERT (`frage_invertiert: true`) und wird über
/fragen bzw. die Verstanden-Metadaten an die Oberfläche gereicht. Kein Ratespiel am Feldnamen
mehr — `stammdaten_keine_bankverbindung` etwa trägt dieselbe Verneinung im Namen und darf
gerade NICHT invertiert werden, weil seine Frage die Verneinung selbst führt.

Der Gate gegen den nächsten Fall steht in tests/test_bindung_frage_polaritaet.py.

NULL LLM.
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
def seite(base):
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 360, "height": 780})
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#wegpunkt:not([hidden])", timeout=5000)
        yield page, base
        browser.close()


def _rendere(page, fid):
    """Die Frage direkt als aktuelle Frage rendern — Muster aus test_ui_wahl_buttons.py. Der
    gemessene Weg bleibt der echte: zeigeFrage() baut die Buttons, der Klick füllt #feld-input,
    leseWert() liest daraus. Nur das Durchklicken der davorliegenden Fragen entfällt."""
    page.evaluate("""async (fid) => {
        const r = await jget(`/fall/${FALL}/fragen`);
        const q = r.body.fragen.find(f => f.feld_id === fid);
        if (q) { AKTUELL = q; zeigeFrage(q, STAND); }
    }""", fid)
    assert page.evaluate("AKTUELL && AKTUELL.feld_id") == fid, (
        f"{fid} ist in der Scheibe 'gesamt' nicht erreichbar — der Test misst sonst nichts.")


# feld_id, Klick des Nutzers, was der Klick bedeutet, erwarteter Feldwert
INVERTIERT = [
    ("vpf_keine_mahlzeitengestellung", "true",
     "ja, mein Arbeitgeber hat Mahlzeiten gestellt", False),
    ("vpf_keine_mahlzeitengestellung", "false",
     "nein, es gab keine Mahlzeiten (Normalfall)", True),
    ("dhf_keine_pflicht_dienstwohnung", "true",
     "ja, es ist eine verpflichtende Dienstwohnung", False),
    ("dhf_keine_pflicht_dienstwohnung", "false",
     "nein, keine Pflicht-Dienstwohnung (Normalfall)", True),
    # Gegenprobe: die fünf Screening-Flags müssen weiter invertieren. Sie liefen bisher über die
    # Präfix-Heuristik; wenn die Umstellung auf die Deklaration sie vergisst, kippt lautlos jede
    # Einkunftsart — genau die Falle, die test_ui_wahl_buttons.py schon einmal beschrieben hat.
    ("kein_kap", "true", "ja, ich hatte Kapitalerträge", False),
    ("kein_kap", "false", "nein, keine Kapitalerträge", True),
    ("kein_gewinn", "true", "ja, ich hatte Gewinneinkünfte", False),
    ("kein_vuv", "true", "ja, ich hatte Mieteinnahmen", False),
    ("kein_sonstige", "true", "ja, ich hatte sonstige Einkünfte", False),
    ("kein_kind", "true", "ja, ich habe Kinder", False),
]

# Felder, deren Frage die Verneinung SELBST trägt — hier wäre eine Umkehr der Fehler.
NICHT_INVERTIERT = [
    ("vv_nebenkosten_nicht_vereinbart", "true",
     "ja, es wurden KEINE Nebenkosten vereinbart", True),
    ("hh_handwerker_keine_foerderung", "true",
     "ja, die Leistungen waren NICHT gefördert", True),
    ("uebernachtung_keine_lange_unterbrechung", "false",
     "nein, es gab eine Pause von sechs Monaten", False),
]


@pytest.mark.parametrize("fid,klick,bedeutung,erwartet", INVERTIERT)
def test_invertierte_frage_speichert_die_bedeutung_der_antwort(seite, fid, klick, bedeutung, erwartet):
    page, _ = seite
    _rendere(page, fid)
    page.click(f".wahl-opt[data-wert='{klick}']")
    wert = page.evaluate("leseWert(AKTUELL)")
    assert wert is erwartet, (
        f"Nutzer antwortet „{bedeutung}\" -> {fid} muss {erwartet} sein, ist {wert!r}. "
        f"Die Umkehr fehlt: der Store hält das Gegenteil dessen, was der Nutzer gesagt hat.")


@pytest.mark.parametrize("fid,klick,bedeutung,erwartet", NICHT_INVERTIERT)
def test_selbstverneinende_frage_wird_nicht_umgekehrt(seite, fid, klick, bedeutung, erwartet):
    """Die Kehrseite. Eine Regel, die jede Verneinung im Feldnamen umkehrt, dreht genau diese
    Felder falsch herum — ihre Frage trägt die Verneinung schon."""
    page, _ = seite
    _rendere(page, fid)
    page.click(f".wahl-opt[data-wert='{klick}']")
    wert = page.evaluate("leseWert(AKTUELL)")
    assert wert is erwartet, (
        f"Nutzer antwortet „{bedeutung}\" -> {fid} muss {erwartet} sein, ist {wert!r}.")


@pytest.mark.parametrize("fid,gespeichert,erwartet_text", [
    ("vpf_keine_mahlzeitengestellung", True, "Nein"),   # kein Mahlzeit -> Frage „gestellt?" = Nein
    ("vpf_keine_mahlzeitengestellung", False, "Ja"),
    ("dhf_keine_pflicht_dienstwohnung", True, "Nein"),
    ("kein_kap", True, "Nein"),
    ("vv_nebenkosten_nicht_vereinbart", True, "Ja"),    # Frage verneint selbst -> keine Umkehr
])
def test_verstanden_seite_zeigt_die_antwort_nicht_den_rohwert(seite, fid, gespeichert, erwartet_text):
    """Auf der Verstanden-Seite bestätigt der Nutzer mit EINEM Klick, was er dort liest. Steht
    dort „Ja", wo der Store das Gegenteil führt, bestätigt er das Gegenteil seiner Absicht."""
    page, _ = seite
    meta = page.evaluate("""async (fid) => {
        const r = await jget(`/fall/${FALL}/fragen`);
        const q = r.body.fragen.find(f => f.feld_id === fid);
        return q ? {feld_id: q.feld_id, typ: q.typ, frage_invertiert: q.frage_invertiert} : null;
    }""", fid)
    assert meta is not None, f"{fid} nicht in der Scheibe 'gesamt' erreichbar"
    text = page.evaluate("v => verstandenWertText(v)", {**meta, "wert": gespeichert})
    assert text == erwartet_text, (
        f"{fid}={gespeichert} muss auf der Verstanden-Seite als „{erwartet_text}\" erscheinen, "
        f"steht als „{text}\".")
