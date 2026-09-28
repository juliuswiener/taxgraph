"""`GET /fall/{id}/fragen` (Scheibe rentner_gesamt) endet mit HTTP 500 für JEDEN aa-Folgejahr-Fall
(`rentner_renten_beginn_jahr < veranlagungszeitraum`) -- UNABHÄNGIG vom Zustand von
`rentner_rentenfreibetrag`. Erste Fassung dieser Datei nahm an, der Absturz hänge am Zustand
(vorläufig vs. bestätigt); ein selbst gebauter grüner Kontrollfall (bestätigt statt vorläufig)
widerlegte das -- er stürzte identisch ab. Diese Fassung hält die BREITERE, durch den
Kontrollfall erzwungene Aussage fest.

Gemessen über vollen HTTP-Weg (register→login→fall→event→fragen, kein direkter API.fragen()-
Aufruf, kein TAXGRAPH_NO_AUTH), dieselbe `base`-Fixture wie test_paket_b_e2e_http.py. Der
vorläufige Fall im roten Test kommt über den ECHTEN Vorjahresübernahme-Kanal (herkunft=vorjahr,
bindung_rentner.yaml: `vorjahr: vorschlag`), kein Testkonstrukt.

Fangstelle (gezeigt, nicht hergeleitet -- api.py:328-335, api_constants.py:363,306-307,
est_mapping.py:865-895, engine/runner.py:895-919):
  `api.fragen()` ruft `_gesamt_beitrag()`, die die Bindung für die Fragen-Gewichtsrechnung über
  `_ring_bindung(cfg, bindung)` (api.py:321-325) auf `cfg["kegel"]` beschneidet -- bewusst, laut
  Kommentar dort, damit unbeantwortete Partnerfelder das Intervall nicht auf "nicht_fixierbar"
  ziehen. `RENTNER_KEGEL` (api_constants.py:363) zieht `RENTNER_22` (api_constants.py:306-307:
  nur rentner_renten_art, rentner_jahresrente, rentner_renten_beginn_jahr,
  rentner_alter_bei_rentenbeginn) -- OHNE rentner_rentenfreibetrag, das nur in der breiteren
  RENTNER_FELDER-Liste steht, nie im Kegel. Dadurch sieht `est_mapping.instanzen(store, rb,
  "rente")` (aufgerufen mit der kegel-beschnittenen Bindung `rb`, bescheid_zweige.py:~992) das
  Feld STRUKTURELL nie -- unabhängig davon, was im Store steht. `_rente_instanz` liest also immer
  `None`, und bei jedem aa-Folgejahr wirft `runner.catala_renten_einkuenfte`
  (RentenfreibetragFixierungOffen, runner.py:916) IMMER. server.py:236 fängt die Exception,
  liefert aber weiterhin HTTP 500 -- kein roher Traceback, der Dialog bricht trotzdem ab.

Gegenprobe zur Ursache (NUR im isolierten Klon /tmp/taxgraph_http_clean gemessen, NICHT im
geteilten Baum, Änderung sofort verworfen): rentner_rentenfreibetrag testweise in RENTNER_KEGEL
aufgenommen -- alle drei Zustände (unbeantwortet, bestätigt, vorläufig) liefern danach HTTP 200
mit Fragenliste. Die Erklärung trägt.

Reichweite (gemessen, nicht vermutet):
  - `/ergebnis` ist NICHT betroffen -- eigener Codepfad (`_feste_zahl`, api.py:217-237) übergibt
    die VOLLE Bindung, nicht die kegel-beschnittene. Unbeantwortet: HTTP 200,
    grund="rentenfreibetrag_fixierung_offen", zahl_cent=None (sauber gesperrt). Bestätigt: HTTP
    200, grund="bestaetigt", zahl_cent=45800 (ein echter Wert). Nur /fragen stürzt.
  - Nur die Scheibe `rentner_gesamt` ist betroffen. Die Scheibe `gesamt` hat keine
    Renten-Kegel-Felder (api_constants.py:717-731, kein RENTNER_22/RENTNER_33B,
    "multi_objekt": "vv_objekt" statt "multi_rente"); ein Event auf `rentner_jahresrente` gegen
    Scheibe `gesamt` wird vom Server selbst abgelehnt (400 "feld_id 'rentner_jahresrente' nicht
    in dieser Scheibe") -- der Zustand ist dort gar nicht herstellbar.
  - Ein einziges Jahr Abstand reicht bereits: beginn=veranlagungszeitraum-1 stürzt identisch ab
    wie beginn=veranlagungszeitraum-10. Nur beginn==veranlagungszeitraum (Erstjahr) ist
    ausgenommen.

WAS DIESER TEST NICHT BEHAUPTET:
  - Er behauptet nicht, dass der Freibetrag-Zustand irrelevant IST -- er zeigt, dass er in diesem
    Zweig (der Fragen-Gewichtsrechnung) strukturell nicht erreicht wird.
  - Er behauptet nichts über den Anteil betroffener Nutzer in der echten Nutzerverteilung --
    "vermutlich der Normalfall für die meisten echten Rentner" bleibt eine Einschätzung, keine
    Messung über echte Nutzerdaten.
  - Er behauptet nichts über andere Endpunkte als `/fragen` (siehe Reichweite oben: `/ergebnis`
    ist separat gemessen und NICHT betroffen).
  - Keine Aussage, ob die zugrundeliegende Sperre (§ 22 Nr. 1 S. 3 Buchst. a Doppelbuchst. aa S. 4
    EStG) selbst berechtigt ist. Keine Aussage über die richtige Reparaturrichtung (Kegel
    erweitern, Fragen-Gewichtsrechnung robuster gegen fehlende Instanzfelder machen, oder etwas
    drittes). Keine Aussage über die separate 0-Anomalie in `/ergebnis` bei unbestätigtem
    Freibetrag (dort wird die Instanz aus der Summe ausgeschlossen statt eine Exception zu
    werfen -- ein anderer, hier nicht geprüfter Mechanismus).

NULL LLM."""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/haut", "produkt/store"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import api as API        # noqa: E402
import server as SRV     # noqa: E402
import audit              # noqa: E402

from _kegel import kegel_fuer  # noqa: E402


def _req(base: str, method: str, path: str, body: dict | None = None, erwarte: int | None = None):
    """Wie test_paket_b_e2e_http.py: 5xx -> AssertionError (nie unterdrückbar), es sei denn
    `erwarte` sagt es ausdrücklich voraus."""
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                 headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=10) as r:
            status = r.status
            content = json.loads(r.read())
    except urllib.error.HTTPError as e:
        status = e.code
        content = json.loads(e.read())
    if erwarte is not None:
        assert status == erwarte, f"erwarte={erwarte}, erhalten={status} {method} {path} {body}"
    elif status >= 500:
        raise AssertionError(f"Serverfehler {status} {method} {path} {body}: {content}")
    elif status >= 400:
        raise AssertionError(f"Fehler {status} {method} {path} {body}: {content}")
    return status, content


@pytest.fixture
def base(tmp_path, monkeypatch):
    monkeypatch.setattr(API, "FAELLE", str(tmp_path / "faelle"))
    monkeypatch.setattr(audit, "AUDIT_DIR", str(tmp_path / "faelle"))
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    try:
        yield f"http://{srv.server_address[0]}:{srv.server_address[1]}"
    finally:
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()


def _bestaetigt(feld_id, wert):
    return {"feld_id": feld_id, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{feld_id}"}}


def _vorjahr_vorlaeufig(feld_id, wert):
    """Exakter Store-Kontrakt für import:vorjahr (store.py, Auflage A): herkunft=vorjahr,
    zustand=vorlaeufig, signal_2=None -- derselbe Kanal, den POST /fall/{id}/vorjahr über die Haut
    tatsächlich schreibt (vorjahr_writer.uebernehme_vorjahr, Zeile 50-57)."""
    return {"feld_id": feld_id, "wert": wert, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "vorjahr"}, "schreiber": "import:vorjahr",
            "signal": {"signal_1": {"typ": "vorjahr", "vz": 2024}, "signal_2": None}}


def _basis(beginn_jahr):
    """aa-Folgejahr, wenn beginn_jahr < 2025 (veranlagungszeitraum); Erstjahr, wenn ==."""
    # GEBAUT, nicht kopiert (tests/_kegel.py). Von Hand standen hier 25 Felder; 22 trugen genau den
    # Abwesenheitswert (gemessen 2026-09-26). Nur die drei unten sind echte Werte.
    # Die Handliste kannte `rentner_renten_beginn_jahr` gar nicht -- und genau dieses Feld ist der
    # Messgegenstand dieses Tests (aa-Folgejahr). `agb_zwangslaeufig`/`agb_notwendig_angemessen`
    # fehlten ebenfalls, wodurch die Faelle auf `input_kegel_nicht_bestaetigt` sperrten.
    return kegel_fuer("rentner_gesamt", {
        "rentner_jahresrente": 2_000_000,
        "rentner_renten_beginn_jahr": beginn_jahr,
        "rentner_alter_bei_rentenbeginn": 60,
        "kein_sonstige": False,
    })


RF = 500_000  # 5.000,00 EUR Rentenfreibetrag


def _anlegen(base, fid, beginn_jahr, extra_event=None):
    st, _ = _req(base, "POST", "/fall",
                {"fall_id": fid, "scheibe": "rentner_gesamt", "veranlagungszeitraum": 2025})
    assert st == 201
    for feld, wert in _basis(beginn_jahr):
        st, _ = _req(base, "POST", f"/fall/{fid}/event", _bestaetigt(feld, wert))
        assert st == 201
    if extra_event is not None:
        st, _ = _req(base, "POST", f"/fall/{fid}/event", extra_event)
        assert st == 201


def test_erstjahr_liefert_normale_fragenliste(base):
    """GRÜNER Kontrollfall: Rente beginnt IM Veranlagungsjahr (kein aa-Folgejahr), Freibetrag gar
    nicht erst beantwortet -- die § 22-Sperre greift dort nicht. Beweist: die Scheibe
    rentner_gesamt funktioniert über /fragen grundsätzlich, nur der aa-Folgejahr-Zweig nicht.
    Schwächere Kontrolle als "bestätigt statt vorläufig" (ursprünglich geplant) -- die war nicht
    baubar, weil ausnahmslos JEDER Zustand des Freibetrags im aa-Folgejahr abstürzt (siehe
    Moduldocstring, Gegenprobe). Diese Kontrolle ist dafür die wahre: sie schließt aus, dass die
    ganze Scheibe kaputt ist."""
    _anlegen(base, "rf_erstjahr", beginn_jahr=2025)
    st, b = _req(base, "GET", "/fall/rf_erstjahr/fragen")
    assert st == 200
    assert isinstance(b.get("fragen"), list)
    assert len(b["fragen"]) > 0


@pytest.mark.parametrize("label, extra_event", [
    ("bestaetigt", _bestaetigt("rentner_rentenfreibetrag", RF)),
    ("vorlaeufig_vorjahr", _vorjahr_vorlaeufig("rentner_rentenfreibetrag", RF)),
])
def test_aa_folgejahr_mit_freibetrag_liefert_fragen_ohne_500(base, label, extra_event):
    """Der aa-Folgejahr-Zweig stürzt NICHT mehr ab, wenn der Freibetrag in der Enumeration sichtbar ist.

    Bis 2026-09-26 warf `GET /fragen` hier für JEDEN Zustand HTTP 500
    (`RentenfreibetragFixierungOffen: aa-Folgejahr 2015<2025 ohne fixierten Rentenfreibetrag`).
    Ursache war nicht der Zustand, sondern die kegel-beschnittene Bindung an
    `_gesamt_beitrag`: `RENTNER_KEGEL` enthält `rentner_rentenfreibetrag` nicht, also sah
    `est_mapping.instanzen` das Feld strukturell nie. `ad2ef61` trennt Aufbau-Bindung (voll,
    enumeriert) von Achsen-Bindung (Kegel), seitdem findet die Enumeration das Feld.

    ZWEI ZUSTÄNDE MIT ZAHL, nicht „stürzt nicht ab": ein Test, der nur HTTP 200 prüft, wäre auch
    dann grün, wenn der Ring danach 0 € nennte (`gruen-weil-der-pruefer-weniger-sieht`).
    Gemessen 2026-09-26, aa-Folgejahr 2015 < VZ 2025, Freibetrag 5.000 €:

      Zustand          /fragen       /ergebnis
      bestaetigt       200, 132 Fr.  200, zahl=45800, grund="bestaetigt"
      vorlaeufig       200, 133 Fr.  200, zahl=0,     grund="bestaetigt"

    Der `vorlaeufig`-Fall trägt die Zahl 0: der Zwei-Signal-Filter
    (`nur_bestaetigt=True` in `_feste_zahl`) lässt eine vorläufige Instanz korrekt aus der
    festgesetzten Zahl heraus — die 0 ist gewollt, nicht der Defekt. Die Zahl steht hier
    trotzdem als Erwartung, damit ein späterer Umbau sie nicht still verschiebt."""
    _anlegen(base, f"rf_{label}", beginn_jahr=2015, extra_event=extra_event)
    st, b = _req(base, "GET", f"/fall/rf_{label}/fragen")
    assert st == 200, f"aa-Folgejahr mit Freibetrag ({label}) darf nicht mehr abstürzen: {b}"
    assert isinstance(b.get("fragen"), list)
    assert len(b["fragen"]) > 0, (label, b)

    st_e, e = _req(base, "GET", f"/fall/rf_{label}/ergebnis")
    assert st_e == 200, e
    erwartet = 45800 if label == "bestaetigt" else 0
    assert e["zahl_cent"] == erwartet, (
        f"{label}: /ergebnis nennt {e['zahl_cent']}, erwartet {erwartet} (grund={e['grund']!r})")


def test_aa_folgejahr_ohne_freibetrag_sperrt_mit_klartext_statt_500(base):
    """Dritte Parametrisierung des alten Tests, jetzt getrennt: ohne jeden Freibetrag.

    Bis 2026-09-26 lieferte `GET /fragen` hier HTTP 500. Ursache war nicht die Sichtbarkeit des
    Feldes (`ad2ef61` hat die Enumeration geheilt, s. Test oben), sondern der ungeschützte Ring
    in `api._gesamt_beitrag`: `runner.catala_renten_einkuenfte` warf
    `RentenfreibetragFixierungOffen`, und niemand fing sie. Jetzt fängt `_gesamt_beitrag` genau
    diese Ausnahme (keine Gewichte), und `fragen()` meldet den Sperrgrund wie `/stand`.

    Gemessen 2026-09-26 auf cdc05bb, Scheibe rentner_gesamt, aa-Folgejahr 2015 < VZ 2025:

      Endpunkt                  vorher            nachher
      /fragen                   HTTP 500          HTTP 200, ring_gesperrt="rentenfreibetrag_fixierung_offen"
      /stand                    HTTP 200 gesperrt  HTTP 200 gesperrt   (unverändert)
      /ergebnis                 HTTP 200 gesperrt  HTTP 200 gesperrt   (unverändert)

    Der Test prüft DREI Dinge, nicht eine: den Status, den Grund als Maschinenwort UND seinen
    Klartext. Ein Test, der nur HTTP 200 prüft, wäre auch dann grün, wenn die Fragenliste leer
    zurückkäme oder der Grund verschwiegen würde (`gruen-weil-der-pruefer-weniger-sieht`)."""
    _anlegen(base, "rf_ohne", beginn_jahr=2015, extra_event=None)
    st, b = _req(base, "GET", "/fall/rf_ohne/fragen", erwarte=200)
    assert isinstance(b.get("fragen"), list)
    assert len(b["fragen"]) > 0, "eine gesperrte Kopfzeile zeigt trotzdem die offenen Fragen"
    assert b.get("ring_gesperrt") == "rentenfreibetrag_fixierung_offen", (
        f"ohne fixierten Freibetrag muss /fragen sperren statt zu werfen: {b.get('ring_gesperrt')!r}")
    kt = b.get("ring_gesperrt_klartext")
    assert isinstance(kt, str) and kt, "der Sperrgrund muss als Klartext ankommen, nicht nur als Kennung"
    assert "Rentenfreibetrag" in kt or "steuerfreie Teil der Rente" in kt, (
        f"der Klartext muss die fehlende Angabe nennen, nicht bloß 'gesperrt' sagen: {kt!r}")
