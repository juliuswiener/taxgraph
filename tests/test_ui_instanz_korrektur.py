"""Eine Antwort fuer das zweite Kind laesst sich wieder aendern.

ANLASS: die Belegt-Liste („Schon beantwortet") macht JEDE Zeile anklickbar, auch
`kind_vorname__2`. Der Klick landete in `korrigiereBestaetigt`, und die suchte die Kennung in
/fragen. Dort steht sie nie: der Traverser fuehrt das Basisfeld EINMAL und legt die Zahl als
`instanz_anzahl` daneben — gemessen 2026-08-27, **0 von 120 Fragen tragen ein `__n`**.

Der Nutzer las daraufhin:

    „Diese Frage ist durch eine andere Antwort entfallen und laesst sich nicht mehr aendern."

Das war doppelt falsch. Die Frage ist nicht entfallen — sie steht unter ihrem Basisnamen da. Und
aendern liesse sie sich sehr wohl. Der Satz schickte den Nutzer von einer moeglichen Korrektur weg.

DAHINTER LAG EIN GROESSERER BEFUND derselben Messung: /fragen ist die Queue der UNBEANTWORTETEN
Felder (`traverser.naechste_fragen` -> `_unbeantwortet`). Ein BESTAETIGTES Feld faellt heraus, ein
vorlaeufiges bleibt drin. Damit war der Korrekturweg fuer JEDES vollstaendig bestaetigte Feld tot,
mit oder ohne Instanz-Achse — gemessen an einem Feld ganz ohne Achse:
`korrigiereBestaetigt('fam_anzahl_kinder')` -> false, dieselbe Meldung. Dass „Ändern" auf der
Pruefliste lief, lag allein daran, dass KI-Vorschlaege vorlaeufig sind.

Behoben mit `GET /fall/<id>/feld/<fid>/frage` (e7f9f2a): die Frage zu EINEM Feld, auch einem
beantworteten, mit demselben Schluesselsatz wie ein Eintrag aus /fragen und serverseitiger
`__n`-Aufloesung. /fragen bleibt die Antwort auf „was ist noch offen".

Damit fiel zunaechst der Hinweis „durch eine andere Antwort entfallen" fuer den Fall weg, fuer den
er gedacht war: eine abgeschaltete Frage antwortet 200 mit voller Frage, genau wie eine bloss
beantwortete. Seit `_frage_metadaten` eine `regel_id` mitfuehrt, kommt die Oberflaeche an
`relevanz` aus /stand und trennt die beiden wieder — NUR bei `ausgeschlossen`, denn `relevanz`
kennt drei Werte und `unentschieden` ist der Normalfall eines Feldes mit offenen Gates
(gemessen: 39 / 24 / 13).
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


def _zwei_kinder_beide_bestaetigt(page):
    """DER EIGENTLICHE NUTZERFALL: zwei Kinder, beide Namen eingetragen und bestaetigt — und
    danach faellt jemandem auf, dass der zweite falsch geschrieben ist.

    Bis e7f9f2a war genau dieser Zustand der unerreichbare: beide Instanzen bestaetigt heisst, das
    Basisfeld ist beantwortet und steht in /fragen nicht mehr."""
    assert _schreibe(page, "fam_anzahl_kinder", 2) in (200, 201)
    assert _schreibe(page, "kind_vorname", "Anna") in (200, 201)
    assert _schreibe(page, "kind_vorname__2", "Ben") in (200, 201)


def _zwei_kinder_eine_zeile_offen(page):
    """Die Variante mit einem offenen KI-Vorschlag: Kind 1 vorlaeufig, Kind 2 vom Nutzer bestaetigt.
    Der Knopf verlangt hier die Halte-Geste (s. `_halten`)."""
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
    """DER BEFUND, im echten Nutzerfall: BEIDE Kinder eingetragen und bestaetigt, dann die zweite
    Zeile korrigieren wollen. Vorher endete dieser Klick in „durch eine andere Antwort entfallen" —
    aus zwei sich ueberlagernden Gruenden (Kennung mit `__n`, und bestaetigt = raus aus /fragen).

    Jetzt liegt die Frage mit BEIDEN Zeilen vor, jede mit dem Wert, der schon da war. Die
    Vorbelegung ist kein Komfort: stuenden die Zeilen leer, muesste der Nutzer annehmen, seine
    Antworten seien weg — und wer nur einen Buchstaben aendern will, tippt alles neu."""
    page, _ = seite
    _zwei_kinder_beide_bestaetigt(page)

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
    _zwei_kinder_beide_bestaetigt(page)
    vorher = _stand(base_url, fall)

    assert _klick_belegt(page, "kind_vorname__2")
    page.wait_for_selector("#eingabe .instanz-zeile", timeout=8000)
    page.evaluate("""() => {
      const els = [...document.querySelectorAll('#eingabe .instanz-zeile input')];
      els[1].value = 'Bernd';        // nur die ZWEITE Zeile
    }""")
    page.click("#bestaetigen")
    page.wait_for_timeout(1500)

    felder = _stand(base_url, fall)
    assert felder["kind_vorname__2"]["wert"] == "Bernd", (
        f"Die Korrektur kam nicht an: {felder.get('kind_vorname__2')}")
    assert felder["kind_vorname__2"]["zustand"] == "bestaetigt"
    assert felder["kind_vorname__2"]["event_id"] != vorher["kind_vorname__2"]["event_id"], (
        "Es steht noch dasselbe Event da — dann wurde nichts geschrieben.")
    # Die erste Zeile behaelt IHREN Wert — der korrigierte darf nicht auf sie ueberlaufen.
    assert felder["kind_vorname"]["wert"] == "Anna", (
        f"Kind 1 hat den Wert von Kind 2 mitbekommen: {felder.get('kind_vorname')}")
    assert felder["kind_vorname"]["zustand"] == "bestaetigt"
    assert not page.fehler, f"Konsolenfehler: {page.fehler}"


def test_eine_korrektur_schreibt_auch_die_unveraenderten_zeilen_neu(seite):
    """FESTGEHALTEN, WAS DABEI AUSSERDEM PASSIERT — gemessen, nicht gewuenscht.

    `schreibeInstanzen` schreibt ALLE gefuellten Zeilen, nicht nur die geaenderte. Wer Kind 2
    korrigiert, erzeugt also auch fuer Kind 1 ein neues Ereignis mit demselben Wert. Fuer den Nutzer
    aendert das nichts — der Wert bleibt, der Zustand bleibt —, aber die Ereigniskette traegt einen
    Eintrag, dem keine Handlung entspricht, und `/warum` zeigt fuer Kind 1 danach ein Ereignis von
    einem Zeitpunkt, an dem der Nutzer Kind 1 gar nicht angefasst hat.

    Das ist keine Regression dieses Fixes — der Schreibweg tat das schon vorher, es war nur nie
    erreichbar, weil die Korrektur nie bis dorthin kam. Dieser Test haelt es fest, damit es nicht
    unbemerkt bleibt."""
    page, base_url = seite
    fall = page.evaluate("FALL")
    _zwei_kinder_beide_bestaetigt(page)
    vorher = _stand(base_url, fall)

    assert _klick_belegt(page, "kind_vorname__2")
    page.wait_for_selector("#eingabe .instanz-zeile", timeout=8000)
    page.evaluate("""() => {
      [...document.querySelectorAll('#eingabe .instanz-zeile input')][1].value = 'Bernd';
    }""")
    page.click("#bestaetigen")
    page.wait_for_timeout(1500)

    nachher = _stand(base_url, fall)
    assert nachher["kind_vorname"]["wert"] == vorher["kind_vorname"]["wert"], (
        "Der Wert von Kind 1 hat sich geaendert — das waere mehr als ein neues Ereignis.")
    assert nachher["kind_vorname"]["event_id"] != vorher["kind_vorname"]["event_id"], (
        "Kind 1 traegt noch sein altes Ereignis — dann schreibt der Weg nur noch die geaenderte "
        "Zeile, und dieser Test beschreibt den falschen Zustand (die bessere Richtung).")


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


def test_ein_bestaetigtes_feld_ohne_achse_laesst_sich_jetzt_korrigieren(seite):
    """DER GROESSERE BEFUND, und er hat mit der Instanz-Achse nichts zu tun: bis e7f9f2a endete die
    Korrektur JEDES bestaetigten Feldes in „durch eine andere Antwort entfallen", weil /fragen nur
    unbeantwortete Felder fuehrt.

    Gemessen an `fam_anzahl_kinder` — ein Feld ganz ohne Achse, das der Nutzer eine Frage zuvor
    selbst beantwortet hat."""
    page, _ = seite
    assert _schreibe(page, "fam_anzahl_kinder", 2) in (200, 201)

    ok = page.evaluate("async () => await korrigiereBestaetigt('fam_anzahl_kinder')")
    page.wait_for_timeout(500)
    assert ok is True, (
        f"Die Korrektur eines bestaetigten Feldes schlaegt weiter fehl. Banner: {_banner(page)!r}")
    assert "entfallen" not in _banner(page)
    m = page.evaluate("() => ({feld: AKTUELL.feld_id, frage: document.getElementById('frage').textContent})")
    assert m["feld"] == "fam_anzahl_kinder", f"AKTUELL steht auf {m['feld']!r}."
    assert m["frage"] and "Kinder" in m["frage"], f"Die falsche Frage liegt vor: {m['frage']!r}"


def test_eine_regel_mit_offenen_gates_sperrt_die_korrektur_nicht(seite):
    """DIE GEGENRICHTUNG ZUR SPERRE, und sie ist der eigentlich gefaehrliche Fall.

    `relevanz` kennt DREI Werte, nicht zwei: gemessen im selben Fall 39 `ausgeschlossen`, 24
    `relevant`, 13 `unentschieden`. `unentschieden` heisst „diese Regel hat noch offene Gates" —
    der voellig normale Zustand eines Feldes, das mitten im Fragebogen steht. Genau darauf steht
    `kind_vorname` hier, solange die uebrigen Kind-Fragen offen sind.

    Eine Sperre auf `!== "relevant"` waere gruen durch jeden Test darueber gelaufen und haette die
    Korrektur trotzdem fuer den Normalfall verboten — dieselbe Klasse Fehler wie der, der hier
    behoben wurde, nur eine Ebene tiefer. Deshalb misst dieser Test den Status AUSDRUECKLICH und
    dann, dass die Korrektur trotzdem laeuft."""
    page, _ = seite
    _zwei_kinder_beide_bestaetigt(page)

    lage = page.evaluate("""async () => {
      const q = (await jget(`/fall/${FALL}/feld/kind_vorname__2/frage`)).body.frage || {};
      const s = (await jget(`/fall/${FALL}/stand`)).body;
      return {regel_id: q.regel_id, status: ((s.relevanz || {})[q.regel_id] || {}).status};
    }""")
    assert lage["status"] == "unentschieden", (
        f"Der Aufbau trifft den Fall nicht mehr — Status ist {lage['status']!r} statt "
        f"'unentschieden'. Dann prueft dieser Test die Polaritaet nicht mehr.")

    ok = page.evaluate("async () => await korrigiereBestaetigt('kind_vorname__2')")
    page.wait_for_timeout(600)
    assert ok is True, (
        f"Eine Regel mit offenen Gates sperrt die Korrektur — das trifft den Normalfall. "
        f"Banner: {_banner(page)!r}")
    assert "entfallen" not in _banner(page)


def test_ein_feld_ohne_ereignis_legt_keine_frage_vor(seite):
    """Ein Fehlschlag darf den Nutzer nicht vor einem leeren Eingabefeld stehen lassen, und er darf
    die Frage nicht ersetzen, an der er gerade sitzt.

    DIESER TEST MISST DIE `/warum`-SCHRANKE, NICHT DIE FRAGE-SCHRANKE, und das gehoert
    dazugeschrieben, weil der Name sonst mehr verspricht: bei einem unbekannten Feld antworten BEIDE
    Endpunkte 404 (gemessen), und `/warum` wird zuerst gerufen. Die 404-Behandlung hinter
    `/feld/<fid>/frage` ist damit im Normalbetrieb gar nicht erreichbar — sie faengt jeden
    Nicht-200-Status ab (Netz- oder Serverfehler) und verhindert, dass gleich darauf `frage.feld_id`
    auf `undefined` zugreift. Belegt ist sie NICHT: die Mutationsprobe „404 wird verschluckt" laesst
    diese Datei vollstaendig gruen. Herstellbar waere der Fall nur mit einem Feld, das ein Ereignis
    traegt und trotzdem keine Frage hat — dafuer muesste ein bestehender Fall die Scheibe wechseln,
    was die Oberflaeche nicht anbietet (ein Kachel-Klick legt einen neuen Fall an)."""
    page, _ = seite
    vorher = page.evaluate("document.getElementById('frage').textContent")

    ok = page.evaluate("async () => await korrigiereBestaetigt('gibt_es_dieses_feld_nicht')")
    page.wait_for_timeout(500)
    assert ok is False, "Fuer ein Feld ohne Ereignis wurde trotzdem eine Frage vorgelegt."
    b = _banner(page)
    assert b, "Der Klick tat nichts und sagte nichts."
    assert "gibt_es_dieses_feld_nicht" not in b, (
        f"Die Kennung steht in der Meldung — die hat der Nutzer nie gesehen: {b!r}")
    assert page.evaluate("document.getElementById('frage').textContent") == vorher, (
        "Die vorher offene Frage wurde ersetzt, obwohl die Korrektur fehlschlug.")


def test_eine_abgeschaltete_frage_bekommt_den_hinweis_statt_der_frage(seite):
    """DER FALL, FUER DEN DIE MELDUNG GEDACHT WAR — und der einzige, der sie noch bekommt.

    Aufbau: Kinder eingetragen, danach „keine Kinder" geantwortet. `kind_vorname` bleibt bestaetigt
    im Stand, faellt aus /fragen, und der Endpunkt antwortet 200 mit voller Frage — von einem bloss
    beantworteten Feld ist das an der Antwort allein nicht zu unterscheiden. Der Unterschied steht
    in `relevanz`, und seit die Frage eine `regel_id` mittraegt, kommt die Oberflaeche dorthin.

    Ohne das legte sie dem Nutzer die Frage nach dem Vornamen seines Kindes vor, kurz nachdem er
    gesagt hatte, er habe keine — das war der Preis des Fixes und ist jetzt bezahlt."""
    page, _ = seite
    _zwei_kinder_beide_bestaetigt(page)
    # Das Gate umdrehen: `kein_kind` ist invertiert (Frage „Hast du Kinder?", Feld benennt die
    # Abwesenheit). Ersetzen, weil Auflage B ein zweites aktives Event abweist.
    vor = page.evaluate("async () => (await jget(`/fall/${FALL}/stand`)).body.felder.kein_kind")
    assert page.evaluate("""async (neu) => {
      const ev = {feld_id: 'kein_kind', wert: neu, zustand: 'bestaetigt',
        herkunft: {herkunft: 'laie', pruef_tiefe: 'ungeprueft', haftung: 'nutzer'},
        schreiber: 'ui:laie', signal: {signal_1: null, signal_2: 'klick@kein_kind'}};
      const j = (await jget(`/fall/${FALL}/feld/kein_kind/warum`)).body.justification || {};
      if (j.event_id) { ev.ersetzt = j.event_id; ev.signal.signal_1 = j.event_id; }
      const r = await jpost(`/fall/${FALL}/event`, ev);
      await refresh();
      return r.status;
    }""", not vor["wert"]) in (200, 201)

    lage = page.evaluate("""async () => {
      const f = (await jget(`/fall/${FALL}/fragen`)).body.fragen || [];
      const s = (await jget(`/fall/${FALL}/stand`)).body;
      const q = (await jget(`/fall/${FALL}/feld/kind_vorname__2/frage`)).body.frage || {};
      return {in_fragen: f.some(x => x.feld_id === 'kind_vorname'),
              zustand: (s.felder.kind_vorname || {}).zustand,
              regel_id: q.regel_id,
              regel_status: ((s.relevanz || {})[q.regel_id] || {}).status,
              endpunkt_liefert_frage: !!q.feld_id};
    }""")
    assert lage["zustand"] == "bestaetigt" and not lage["in_fragen"], (
        f"Der Aufbau stimmt nicht — das Feld ist nicht abgeschaltet-aber-beantwortet: {lage}")
    assert lage["endpunkt_liefert_frage"], (
        "Der Endpunkt liefert gar keine Frage mehr — dann misst dieser Test die falsche Schranke.")
    assert lage["regel_status"] == "ausgeschlossen", (
        f"Die Regel gilt nicht als ausgeschlossen ({lage}) — dann gibt es hier nichts zu erkennen.")

    vorher = page.evaluate("document.getElementById('frage').textContent")
    ok = page.evaluate("async () => await korrigiereBestaetigt('kind_vorname__2')")
    page.wait_for_timeout(600)
    assert ok is False, (
        f"Die abgeschaltete Frage wurde vorgelegt. Im Bild steht: "
        f"{page.evaluate('document.getElementById(\"frage\").textContent')!r}")
    b = _banner(page)
    assert "entfallen" in b, f"Der Hinweis nennt den Grund nicht: {b!r}"
    assert "abgeschaltet" in b, (
        f"Der Hinweis sagt nicht, was der Nutzer tun kann, um sie zurueckzuholen: {b!r}")
    assert page.evaluate("document.getElementById('frage').textContent") == vorher, (
        "Die Frage im Bild wurde ersetzt, obwohl die Korrektur abgelehnt wurde.")


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
