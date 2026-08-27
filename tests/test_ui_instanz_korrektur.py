"""Eine Antwort fuer das zweite Kind laesst sich wieder aendern.

ANLASS: die Belegt-Liste („Schon beantwortet") macht JEDE Zeile anklickbar, auch
`kind_vorname__2`. Der Klick landete in `korrigiereBestaetigt`, und die suchte die Kennung in
/fragen. Dort steht sie nie: der Traverser fuehrt das Basisfeld EINMAL und legt die Zahl als
`instanz_anzahl` daneben — gemessen 2026-08-27, **0 von 120 Fragen tragen ein `__n`**.

Der Nutzer las daraufhin:

    „Diese Frage ist durch eine andere Antwort entfallen und laesst sich nicht mehr aendern."

Das war doppelt falsch. Die Frage ist nicht entfallen — sie steht unter ihrem Basisnamen da. Und
aendern liesse sie sich sehr wohl. Der Satz schickte den Nutzer von einer moeglichen Korrektur weg.

WAS DIESE DATEI **NICHT** ABDECKT, und das ist der groessere Befund derselben Messung: /fragen ist
die Queue der UNBEANTWORTETEN Felder (`traverser.naechste_fragen` -> `_unbeantwortet`). Ein
BESTAETIGTES Feld faellt heraus, ein vorlaeufiges bleibt drin. Damit ist der Korrekturweg fuer jedes
vollstaendig bestaetigte Feld tot, mit oder ohne Instanz-Achse — gemessen an einem Feld ganz ohne
Achse: `korrigiereBestaetigt('fam_anzahl_kinder')` -> false, dieselbe Meldung. Das laesst sich in
app.js nicht heilen: `baueEingabe` braucht typ/enum_werte/muster/beispielwert/instanz_anzahl, und
/stand fuehrt davon nur einen Bruchteil (api._anzeige_metadaten). Gemeldet an team-lead; hier
steht dafuer `test_ein_bestaetigtes_feld_sagt_die_wahrheit_statt_entfallen`, der genau diese Grenze
festhaelt, statt sie zu verschweigen.

Die Faelle unten sind deshalb der Zustand, in dem der Weg HEUTE erreichbar ist — und der ist kein
Kunstgriff, sondern der haeufige: solange irgendeine Instanz des Feldes noch offen ist, steht das
Basisfeld in /fragen. Wer die erste Zeile leer gelassen oder einen KI-Vorschlag noch nicht
bestaetigt hat, ist genau dort.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store", "produkt/traverser", "produkt/mapping",
            "produkt/unsicherheit", "golden", "produkt/auth"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API           # noqa: E402
import audit                # noqa: E402
import server as SRV        # noqa: E402
from ui_hilfen import zum_fragebogen  # noqa: E402

try:
    from playwright.sync_api import sync_playwright
except ImportError:
    sync_playwright = None


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
    if sync_playwright is None:
        pytest.skip("playwright fehlt")
    with sync_playwright() as p:
        browser = p.chromium.launch()
        page = browser.new_page(viewport={"width": 1280, "height": 900})
        page.fehler = []
        page.on("pageerror", lambda e: page.fehler.append(str(e)))
        page.goto(base)
        page.wait_for_load_state("networkidle")
        page.evaluate("document.querySelector(\".kachel[data-scheibe='gesamt']\").click()")
        page.wait_for_selector("#wegwahl:not([hidden])", timeout=5000)
        page.evaluate("document.getElementById('weg-fragebogen').click()")
        zum_fragebogen(page, ankreuzen=["kein_kind"])
        yield page, base
        browser.close()


def _stand(base, fall):
    """Der Store ueber echtes HTTP — nicht `STAND` im Browser. Der wird erst nach dem Freigeben
    des Knopfes nachgezogen, und ein Test, der ihn liest, misst die Anzeige statt der Wirkung."""
    with urllib.request.urlopen(f"{base}/fall/{fall}/stand") as a:
        return json.loads(a.read())["felder"]


def _schreibe(page, fid, wert, zustand="bestaetigt"):
    """Ein Event auf dem normalen Weg (POST /event) — derselbe Endpunkt, den die Oberflaeche nutzt."""
    return page.evaluate("""async ([fid, wert, zustand]) => {
      const r = await jpost(`/fall/${FALL}/event`, {
        feld_id: fid, wert, zustand,
        herkunft: {herkunft: zustand === 'vorlaeufig' ? 'llm_vorschlag' : 'laie',
                   pruef_tiefe: 'ungeprueft', haftung: 'nutzer'},
        schreiber: zustand === 'vorlaeufig' ? 'llm:chat' : 'ui:laie',
        signal: {signal_1: null, signal_2: zustand === 'vorlaeufig' ? null : 'klick@' + fid}});
      await refresh();
      return r.status;
    }""", [fid, wert, zustand])


def _zwei_kinder_eine_zeile_offen(page):
    """Der Ausgangszustand fuer den Korrekturweg: zwei Kinder, Kind 1 als KI-Vorschlag (vorlaeufig,
    also noch unbeantwortet), Kind 2 vom Nutzer selbst bestaetigt.

    Warum so und nicht beide bestaetigt: dann faellt das Basisfeld aus /fragen und der Weg endet
    vor der Frage — s. Modulkopf. Der Zustand hier ist der eines Nutzers, der einen Vorschlag noch
    nicht abgenickt hat, und das ist der Normalfall im KI-Weg."""
    assert _schreibe(page, "fam_anzahl_kinder", 2) in (200, 201)
    assert _schreibe(page, "kind_vorname", "Anna", "vorlaeufig") in (200, 201)
    assert _schreibe(page, "kind_vorname__2", "Ben") in (200, 201)


def _klick_belegt(page, fid):
    """Die Zeile der Belegt-Liste anklicken, die zu `fid` gehoert — ueber die Kennung im title,
    weil der sichtbare Text die Frage ist."""
    return page.evaluate("""(fid) => {
      const li = [...document.querySelectorAll('#belegt-liste li')]
        .find(x => x.querySelector('.z-name').title === fid);
      if (!li) return false;
      li.click();
      return true;
    }""", fid)


def _banner(page):
    return (page.evaluate(
        "(document.getElementById('netz-banner') || {}).textContent || ''") or "").strip()


def _halten(page, sel="#bestaetigen"):
    """Ein KI-Vorschlag wird GEHALTEN, nicht geklickt (holdGeste, 600 ms). Ein `click` schreibt
    hier nichts — beim ersten Entwurf dieser Datei blieben genau daran zwei Tests haengen.

    UND DAS IST EINE BEOBACHTUNG UEBER DIE ACHSE, nicht bloss Testmechanik: die Geste richtet sich
    nach dem BASISFELD, nicht nach der angeklickten Instanz. Kind 1 traegt hier einen KI-Vorschlag,
    Kind 2 hat der Nutzer selbst getippt — geklickt hat er auf Kind 2, halten muss er trotzdem. Das
    ist eher richtig als falsch (er nickt den Vorschlag ja mit ab), aber es heisst: EINE vorlaeufige
    Instanz macht die ganze Frage zur Halte-Geste."""
    page.hover(sel)
    page.mouse.down()
    page.wait_for_timeout(800)   # 600 ms Haltedauer plus Luft
    page.mouse.up()


# ------------------------------------------------------------------ die Zerlegung selbst

def test_die_zerlegung_misst_sich_am_hinweg(seite):
    """`basisFeldId` ist die Umkehrung von `instanzFeldId` — und prueft sich an ihr, statt ein
    viertes `__n`-Muster neben die drei vorhandenen zu stellen.

    Die Faelle unten sind die, die ein handgeschriebenes Regex gern durchlaesst. Ginge einer davon
    als Instanz durch, schriebe die Oberflaeche gleich darauf an eine Kennung, die der ELSTER-Writer
    nie findet."""
    page, _ = seite
    r = page.evaluate("""() => {
      const f = ['kind_vorname', 'kind_vorname__2', 'kind_vorname__10',
                 'kind_vorname__1', 'kind_vorname__02', 'kind_vorname__0',
                 'kind_vorname__', 'kind_vorname__2x', 'kind_vorname__2__3'];
      return Object.fromEntries(f.map(x => [x, basisFeldId(x)]));
    }""")
    assert r["kind_vorname"] == {"basis": "kind_vorname", "instanz": 1}
    assert r["kind_vorname__2"] == {"basis": "kind_vorname", "instanz": 2}
    assert r["kind_vorname__10"] == {"basis": "kind_vorname", "instanz": 10}
    # `__1` schreibt instanzFeldId nie (Instanz 1 ist die Basis) — also ist es keine Instanz.
    assert r["kind_vorname__1"] == {"basis": "kind_vorname__1", "instanz": 1}
    for kaputt in ("kind_vorname__02", "kind_vorname__0", "kind_vorname__", "kind_vorname__2x"):
        assert r[kaputt] == {"basis": kaputt, "instanz": 1}, (
            f"{kaputt} wurde als Instanz gelesen — instanzFeldId haette es nie so geschrieben.")
    # Mehrfaches Suffix: nur das LETZTE zaehlt, sonst faellt der Rundweg auseinander.
    assert r["kind_vorname__2__3"] == {"basis": "kind_vorname__2", "instanz": 3}

    rund = page.evaluate("""() => {
      const out = [];
      for (const i of [1, 2, 3, 17]) {
        const fid = instanzFeldId('kind_vorname', i);
        const z = basisFeldId(fid);
        out.push([i, fid, z.basis, z.instanz]);
      }
      return out;
    }""")
    for i, fid, basis, instanz in rund:
        assert basis == "kind_vorname" and instanz == i, (
            f"Rundweg kaputt bei Instanz {i}: {fid} -> {basis}/{instanz}")


# ------------------------------------------------------------------ (a) der Klickweg

def test_klick_auf_die_zweite_instanz_oeffnet_die_frage_mit_beiden_zeilen(seite):
    """DER BEFUND. Vorher endete dieser Klick in „durch eine andere Antwort entfallen"; jetzt liegt
    die Frage mit BEIDEN Zeilen vor, jede mit dem Wert, der schon da war.

    Die Vorbelegung ist kein Komfort: stuenden die Zeilen leer, muesste der Nutzer annehmen, seine
    Antworten seien weg — und wer nur einen Buchstaben aendern will, tippt alles neu."""
    page, _ = seite
    _zwei_kinder_eine_zeile_offen(page)

    assert _klick_belegt(page, "kind_vorname__2"), (
        "Die Zeile fuer kind_vorname__2 steht gar nicht in der Belegt-Liste.")
    page.wait_for_selector("#eingabe .instanz-zeile", timeout=8000)

    assert "entfallen" not in _banner(page), (
        f"Die alte Fehlmeldung steht noch da: {_banner(page)!r}")

    m = page.evaluate("""() => ({
      frage: document.getElementById('frage').textContent,
      feld: AKTUELL.feld_id,
      werte: [...document.querySelectorAll('#eingabe .instanz-zeile input')].map(el => el.value),
      marken: [...document.querySelectorAll('#eingabe .instanz-marke')].map(el => el.textContent),
    })""")
    assert m["feld"] == "kind_vorname", (
        f"Die Korrektur laeuft auf {m['feld']!r} statt auf dem Basisfeld.")
    assert len(m["werte"]) == 2, f"{len(m['werte'])} Eingabefelder statt zwei: {m}"
    assert m["werte"] == ["Anna", "Ben"], (
        f"Die schon gegebenen Antworten stehen nicht in den Zeilen: {m['werte']} — so liest es "
        f"sich, als waeren sie weg.")
    assert m["marken"] == ["Kind 1", "Kind 2"], (
        f"Ohne Nummer weiss der Nutzer nicht, welche Zeile welches Kind ist: {m['marken']}")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


# ------------------------------------------------------------------ (b) das Schreiben

def test_die_geaenderte_zweite_zeile_trifft_genau_ihre_instanz(seite):
    """Gemessen wird der STORE, nicht das Bild: zwei Felder dastehen und ins falsche schreiben
    waere derselbe Verlust mit besserer Optik.

    Die erste Zeile bleibt dabei UNANGETASTET — auch ihr Zustand. Sie war ein KI-Vorschlag und ist
    noch keiner bestaetigten Antwort gleichzusetzen, nur weil daneben eine korrigiert wurde."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    _zwei_kinder_eine_zeile_offen(page)
    vorher = _stand(base_url, fall)["kind_vorname__2"]

    assert _klick_belegt(page, "kind_vorname__2")
    page.wait_for_selector("#eingabe .instanz-zeile", timeout=8000)
    page.evaluate("""() => {
      const els = [...document.querySelectorAll('#eingabe .instanz-zeile input')];
      els[1].value = 'Bernd';        // nur die ZWEITE Zeile
    }""")
    _halten(page)
    page.wait_for_timeout(1500)

    felder = _stand(base_url, fall)
    assert felder["kind_vorname__2"]["wert"] == "Bernd", (
        f"Die Korrektur kam nicht an: {felder.get('kind_vorname__2')}")
    assert felder["kind_vorname__2"]["zustand"] == "bestaetigt"
    assert felder["kind_vorname__2"]["event_id"] != vorher["event_id"], (
        "Es steht noch dasselbe Event da — dann wurde nichts geschrieben.")
    # Die erste Zeile behaelt IHREN Wert — der korrigierte darf nicht auf sie ueberlaufen.
    assert felder["kind_vorname"]["wert"] == "Anna", (
        f"Kind 1 hat den Wert von Kind 2 mitbekommen: {felder.get('kind_vorname')}")
    # FESTGEHALTEN, was dabei ausserdem passiert (gemessen, nicht gewuenscht): Kind 1 war ein
    # vorlaeufiger KI-Vorschlag und ist nach dem Halten BESTAETIGT. Das ist vertretbar — der
    # Vorschlag stand sichtbar in der Zeile darueber und der Knopf verlangt die Halte-Geste, also
    # hat der Nutzer ihn mit abgenickt. Uebersehen sollte es trotzdem niemand: eine Korrektur an
    # Zeile 2 entscheidet einen offenen Vorschlag in Zeile 1 mit.
    assert felder["kind_vorname"]["zustand"] == "bestaetigt", (
        "Wenn Kind 1 vorlaeufig bleibt, hat sich der Schreibweg geaendert — dann gehoert dieser "
        "Kommentar ueberprueft, nicht der Assert angepasst.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_die_korrektur_materialisiert_keine_leeren_instanzen(seite):
    """Der schaerfere Beweis fuer „trifft genau ihre Instanz": eine Zeile, die LEER war, muss leer
    bleiben. Ein Fix, der beim Schreiben durchzaehlt statt die angeklickte Instanz zu treffen, faellt
    hier auf — er legte fuer Kind 1 und 2 Werte an, die nie jemand eingetragen hat.

    Aufbau ohne KI-Vorschlag, also ohne Halte-Geste: drei Kinder, nur Kind 3 beantwortet. Das
    Basisfeld ist damit unbeantwortet und steht in /fragen."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    assert _schreibe(page, "fam_anzahl_kinder", 3) in (200, 201)
    assert _schreibe(page, "kind_vorname__3", "Cem") in (200, 201)

    assert _klick_belegt(page, "kind_vorname__3"), "Die Zeile fehlt in der Belegt-Liste."
    page.wait_for_selector("#eingabe .instanz-zeile", timeout=8000)
    werte = page.evaluate(
        "[...document.querySelectorAll('#eingabe .instanz-zeile input')].map(el => el.value)")
    assert werte == ["", "", "Cem"], (
        f"Die Vorbelegung sitzt in der falschen Zeile: {werte}")

    page.evaluate("""() => {
      const els = [...document.querySelectorAll('#eingabe .instanz-zeile input')];
      els[2].value = 'Cemal';
    }""")
    page.click("#bestaetigen")
    page.wait_for_timeout(1500)

    felder = _stand(base_url, fall)
    assert felder["kind_vorname__3"]["wert"] == "Cemal", (
        f"Die Korrektur kam nicht an: {felder.get('kind_vorname__3')}")
    for leer in ("kind_vorname", "kind_vorname__2"):
        assert leer not in felder, (
            f"{leer} wurde angelegt, obwohl die Zeile leer war — Stille-Null-Regel gebrochen: "
            f"{felder.get(leer)}")


def test_die_erste_zeile_laesst_sich_ueber_dieselbe_korrektur_mitgeben(seite):
    """Die Gegenrichtung zum Test darueber: wer beide Zeilen aendert, aendert auch beide. Sonst
    waere „nur die zweite trifft" mit einem Fix erfuellt, der schlicht immer nur eine schreibt."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    _zwei_kinder_eine_zeile_offen(page)

    assert _klick_belegt(page, "kind_vorname__2")
    page.wait_for_selector("#eingabe .instanz-zeile", timeout=8000)
    page.evaluate("""() => {
      const els = [...document.querySelectorAll('#eingabe .instanz-zeile input')];
      els[0].value = 'Anne'; els[1].value = 'Bernd';
    }""")
    _halten(page)
    page.wait_for_timeout(1500)

    felder = _stand(base_url, fall)
    assert felder["kind_vorname"]["wert"] == "Anne"
    assert felder["kind_vorname"]["zustand"] == "bestaetigt", (
        "Der Nutzer hat den Vorschlag hier selbst ueberschrieben — das ist eine Antwort.")
    assert felder["kind_vorname__2"]["wert"] == "Bernd"


# ------------------------------------------------------------------ (c) ohne Achse

def test_ein_feld_ohne_achse_geht_denselben_weg_wie_bisher(seite):
    """Die Aufloesung darf ein Feld ohne Achse nicht anfassen. `veranlagung` traegt kein `__`,
    `basisFeldId` gibt es unveraendert zurueck, und die Korrektur laeuft wie vorher."""
    page, _base_url = seite
    # `veranlagung` vorlaeufig belegen -> es bleibt in /fragen (nur bestaetigt faellt heraus)
    assert _schreibe(page, "veranlagung", "einzel", "vorlaeufig") in (200, 201)

    ok = page.evaluate("async () => await korrigiereBestaetigt('veranlagung')")
    page.wait_for_timeout(500)
    assert ok is True, f"Die Korrektur eines Feldes ohne Achse schlug fehl. Banner: {_banner(page)!r}"

    m = page.evaluate("""() => ({
      feld: AKTUELL.feld_id,
      instanz_zeilen: document.querySelectorAll('#eingabe .instanz-zeile').length,
    })""")
    assert m["feld"] == "veranlagung", f"AKTUELL steht auf {m['feld']!r}."
    assert m["instanz_zeilen"] == 0, (
        f"Ein Feld ohne Achse hat {m['instanz_zeilen']} Instanz-Zeilen bekommen.")
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_ein_bestaetigtes_feld_sagt_die_wahrheit_statt_entfallen(seite):
    """DIE GRENZE, festgehalten statt verschwiegen (s. Modulkopf).

    Ein BESTAETIGTES Feld faellt aus /fragen, die Korrektur kommt also nicht bis zur Frage — das
    liegt an der Fragen-Queue und ist in app.js nicht zu heilen. Was hier zu heilen war, ist die
    MELDUNG: „durch eine andere Antwort entfallen" ist bei einer Frage, die der Nutzer gerade selbst
    beantwortet hat, schlicht falsch und schickt ihn eine Antwort suchen, die es nicht gibt.

    Gemessen an einem Feld ganz ohne Instanz-Achse — der Befund ist keiner der Achse."""
    page, _ = seite
    assert _schreibe(page, "fam_anzahl_kinder", 2) in (200, 201)

    ok = page.evaluate("async () => await korrigiereBestaetigt('fam_anzahl_kinder')")
    page.wait_for_timeout(500)
    assert ok is False, "Wenn das jetzt geht, ist /fragen erweitert worden — dann diesen Test ersetzen."
    b = _banner(page)
    assert "entfallen" not in b, (
        f"Die Meldung behauptet weiter, die Frage sei entfallen: {b!r}")
    assert "gespeichert" in b, f"Die Meldung sagt nicht, was wirklich los ist: {b!r}"


# ------------------------------------------------------------------ die Belegt-Zeile

def test_die_instanz_zeile_ist_lesbar_statt_kennung_und_rohwert(seite):
    """Eine Zeile, die der Nutzer nicht als seine Frage erkennt, klickt er nicht an — dann bliebe
    der Korrekturweg dahinter unerreichbar.

    GEMESSEN vor dem Fix standen untereinander:
        Wie heisst dein Kind mit Vornamen?    Anna
        kind_vorname__2                       "Ben"
    Kennung und JSON-Rohwert, also genau die zwei Dinge, die diese Liste ersparen soll."""
    page, _ = seite
    _zwei_kinder_eine_zeile_offen(page)

    z = page.evaluate("""() => {
      const li = [...document.querySelectorAll('#belegt-liste li')]
        .find(x => x.querySelector('.z-name').title === 'kind_vorname__2');
      return li ? {name: li.querySelector('.z-name').textContent,
                   wert: li.querySelector('.z-wert').textContent} : null;
    }""")
    assert z, "Die Zeile fehlt in der Belegt-Liste."
    assert z["name"] != "kind_vorname__2", (
        "Die Zeile zeigt weiter die Kennung — die hat der Nutzer nie gesehen.")
    assert "Vornamen" in z["name"], f"Das ist nicht die Frage des Feldes: {z['name']!r}"
    assert "2" in z["name"], (
        f"Ohne Nummer stehen zwei gleich benannte Zeilen untereinander: {z['name']!r}")
    assert z["wert"] == "Ben", (
        f"Der Wert steht als Rohwert da statt lesbar: {z['wert']!r}")


# ------------------------------------------------------------------ der Zeichen-Index

def test_ein_einzelner_wert_wird_nicht_buchstabenweise_auf_die_zeilen_verteilt(seite):
    """GEMESSEN 2026-08-27, und der Fehler war schon erreichbar: `baueInstanzEingaben` las seine
    Vorbelegung als `vorbelegungen[i]`. Kommt dort ein einzelner Wert an statt einer Zuordnung je
    Instanz, indiziert das eine Zeichenkette — bei zwei Kindern und „Anna" stand in BEIDEN Zeilen
    der Buchstabe „n".

    Ein Buchstabe im Feld sieht aus wie eine Antwort, nicht wie ein Fehler. Deshalb faellt ein
    einzelner Wert jetzt auf „nicht vorbelegt" zurueck, statt zerlegt zu werden."""
    page, _ = seite
    assert _schreibe(page, "fam_anzahl_kinder", 2) in (200, 201)

    r = page.evaluate("""async () => {
      const rr = await jget(`/fall/${FALL}/fragen`);
      const q = (rr.body.fragen || []).find(x => x.feld_id === 'kind_vorname');
      if (!q) return {fehler: 'kind_vorname nicht in /fragen'};
      const box = document.getElementById('eingabe');
      baueEingabe(q, box, 'probe-input', 'frage', 'Anna');
      const skalar = [...box.querySelectorAll('input')].map(el => el.value);
      baueEingabe(q, box, 'probe-input', 'frage', {1: 'Anna', 2: 'Ben'});
      const je_instanz = [...box.querySelectorAll('input')].map(el => el.value);
      return {anzahl: q.instanz_anzahl, skalar, je_instanz};
    }""")
    assert not r.get("fehler"), r
    assert r["anzahl"] == 2, f"Der Aufbau stimmt nicht: {r}"
    assert r["skalar"] == ["", ""], (
        f"Ein einzelner Wert wurde buchstabenweise verteilt: {r['skalar']} — genau der Befund.")
    assert r["je_instanz"] == ["Anna", "Ben"], (
        f"Die Zuordnung je Instanz kommt nicht an: {r['je_instanz']}")


# ------------------------------------------------------------------ was beim Leeren passiert

def test_eine_geleerte_vorbelegte_zeile_loescht_nichts_und_sagt_das_nicht(seite):
    """FESTGEHALTEN, WAS HEUTE PASSIERT — kein Wunschverhalten, sondern die Messung.

    Wer eine vorbelegte Zeile LEERT, will sie vermutlich loeschen. Das kann die Software nicht: ein
    Event laesst sich nicht zuruecknehmen, nur ersetzen, und die Stille-Null-Regel ueberspringt eine
    leere Instanz beim Schreiben. Der alte Wert bleibt also stehen.

    STILL GEHT DABEI NICHTS VERLOREN — der Wert ueberlebt, das ist die sichere Richtung. Was fehlt,
    ist die AUSKUNFT: der Nutzer leert die Zeile, drueckt bestaetigen, und nichts widerspricht ihm.
    Beim naechsten Blick in die Liste steht der alte Wert wieder da, ohne dass je gesagt wurde, dass
    das Leeren nicht zaehlt. Dieser Test haelt den Zustand fest, damit die Luecke nicht in einem
    gruenen Lauf verschwindet."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    _zwei_kinder_eine_zeile_offen(page)
    vorher = _stand(base_url, fall)

    assert _klick_belegt(page, "kind_vorname__2")
    page.wait_for_selector("#eingabe .instanz-zeile", timeout=8000)
    page.evaluate("""() => {
      const els = [...document.querySelectorAll('#eingabe .instanz-zeile input')];
      els[1].value = '';           // die vorbelegte zweite Zeile LEEREN
    }""")
    _halten(page)
    page.wait_for_timeout(1500)

    nachher = _stand(base_url, fall)
    assert nachher["kind_vorname__2"]["wert"] == "Ben", (
        "Der Wert ist verschwunden — dann laesst sich eine Instanz doch loeschen, und dieser Test "
        "beschreibt den falschen Zustand.")
    assert nachher["kind_vorname__2"]["event_id"] == vorher["kind_vorname__2"]["event_id"], (
        "Es wurde ein neues Event geschrieben, obwohl die Zeile leer war.")
    # Und die Auskunft, die HEUTE fehlt: der Nutzer erfaehrt nicht, dass sein Leeren folgenlos war.
    # Wird das eines Tages gebaut, faellt dieser Test — und das ist dann die richtige Meldung.
    assert "gelöscht" not in _banner(page).lower(), (
        "Es wird ein Loeschen behauptet, das nicht stattgefunden hat.")
