"""main-Auftrag 2026-08-31 (Nachmessung eines gemeldeten dritten Formel-Duplikats): richtungs-
neutraler Regressionstest fuer einen ERREICHBAREN Zustand-Leck in Copy 3 der Toepfe-XOR-Aggregat-
Formel (Paragraph 20 Kapitaleinkuenfte).

Ausgangslage: dieselbe Toepfe-vs-Aggregat-Prioritaetsregel existiert DREIFACH im Code --
(1)+(2) _p20_kapitaleinkuenfte (bescheid_einkuenfte.py:211-250, von beiden _zweig_*-Funktionen in
bescheid_zweige.py aufgerufen -- SINGLE-SOURCE zwischen den beiden Zweigen seit 2026-08-17), und
(3) inline in _mit_ring_werten (bescheid_deklaration.py:116-168), das E1900401/E1901401 injiziert.
tests/test_zweig_duplikation_differential.py deckt NUR (1)/(2) gegeneinander ab -- es importiert
bescheid_deklaration nirgends, VERGLEICHBAR enthaelt catala_kapital_verrechnung/catala_sparer_pb
seit derselben 2026-08-17-Aenderung bewusst NICHT mehr (Kommentar dort: zwischen den ZWEIGEN
single-source geworden -- das sagt nichts ueber Copy 3). Copy 3 ist also tatsaechlich UNGEDECKT.

Befund (live gemessen, s.u.): Copy 1/2 lesen `felder` NUR nach dem `nur_bestaetigt`-Filter in
_bescheid_fn (bescheid_zweige.py ~1332, "Zwei-Signal-Invariante AM RING") -- ein vorlaeufiges
(unbestaetigtes) KAP-Feld bewegt die Steuer NIE. Copy 3s `_kap_positiv`/`_c2` (bescheid_deklaration.
py:117-119, 136-137) lesen `felder.get(fid).get("wert")` OHNE zustand-Check -- im GEGENSATZ zur
Nachbarfunktion `_instanz_summe` (Zeile 173-183) IN DERSELBEN DATEI, die zustand=="bestaetigt"
explizit prueft. Ein vorlaeufiger KAP-Topf-Wert (z.B. eine noch nicht bestaetigte Vorjahres-
Uebernahme) loest deshalb `kap_erklaert=True` aus und injiziert E1900401=True sowie ein aus dem
UNBESTAETIGTEN Wert berechnetes E1901401 -- DESSEN eigene injizierte Events selbst hartkodiert
zustand="bestaetigt" tragen (Zeile 128, 164), unabhaengig vom Bestaetigungsstatus der Quelle.

ERWARTUNG vor der Messung: /ergebnis bewegt sich NICHT (Copy 1 durch nur_bestaetigt geschuetzt) --
das ist NICHT der Befund, sondern die Vorbedingung dafuer, dass ueberhaupt ein Steuer/Deklaration-
Widerspruch entstehen kann (identisches Muster wie test_p20_gewinn_sonstige_e1900701_widerspruch.py).
GEGENERWARTUNG: /deklaration bleibt bei der Baseline (Copy 3 doch irgendwo zustand-gefiltert) --
das war die Nullhypothese, die die Messung unten widerlegt.

Was Part 3 (reicht der Leck bis ins abgabefertige XML?) angeht: NEIN, fuer GENAU diesen Mechanismus
nicht. EM.deklariere() (est_mapping.py:574-585) prueft in einer EIGENEN, allgemeinen Haupt-Schleife
JEDES in der Bindung gefuehrte Feld im materialisierten Snapshot einzeln auf zustand=="bestaetigt" --
unabhaengig davon, ob es ein eigenes Kz hat. Das faengt kap_gewinn_sonstige (das selbst KEIN Kz hat,
"Modell-Mismatch", s. nicht_deklariert) trotzdem ab und setzt eingaben_konsistent=False ->
einreichen() bricht mit 409 deklaration_unvollstaendig VOR erzeuge_xml() ab (live bestaetigt: kein
XML erfasst). Das ist ein ANDERER, allgemeinerer Wächter als kapital_semantik_offen (der nur bei
GLEICHZEITIG positivem Aggregat+Topf feuert, hier irrelevant, da Aggregat=0) -- er schuetzt hier
zufaellig mit, weil er JEDES vorlaeufige Feld in der Bindung greift, nicht weil er den KAP-Leck kennt.

Der Leck reicht aber bis in die LIVE-Antwort von GET /fall/{id}/deklaration: derselbe Aufruf, der
eingaben_konsistent=False UND unvollstaendig=[kap_gewinn_sonstige] zurueckgibt, traegt im selben
JSON-Objekt einen deklaration-Unterdict mit E1900401=True/E1901401>0 -- intern widerspruechlich.
Kein Frontend in produkt/haut/static/app.js ruft /deklaration ueberhaupt auf (grep: 0 Treffer) --
End-Nutzer-Sichtbarkeit ist NICHT belegt, nur die HTTP-Erreichbarkeit des Endpunkts selbst.

Was dieser Test NICHT behauptet: dass 1.750 EUR oder 10,00 EUR (E1901401) materiell richtige
Betraege sind (die Sparer-Pauschbetrag-Rechnung selbst wird hier nicht geprueft); dass der Leck
das abgesendete XML erreicht (belegt das GENAUE GEGENTEIL fuer den hier gemessenen Fall -- ein
anderer Wächter faengt ihn ab, siehe oben); dass ein Nutzer diesen JSON-Widerspruch je zu Gesicht
bekommt (keine Frontend-Kopplung gefunden). Er behauptet nur: derselbe Store-Zustand liefert an
zwei verschiedenen, beide live erreichbaren HTTP-Endpunkten zwei verschiedene Aussagen ueber
dasselbe KAP-Engagement -- /ergebnis "keins", /deklaration "beantragt, 10,00 EUR genutzt" --
UND /deklaration widerspricht sich in sich selbst (eingaben_konsistent=False neben injizierten
"bestaetigt"-Werten aus genau dem Feld, das als unvollstaendig gemeldet wird).

BAUFORM, die dieser Messweg NICHT sieht: nur EIN Eingabe-Vektor wurde live geprueft (ein einzelnes
vorlaeufiges Topf-Feld, Einzelveranlagung, Aggregat=0). NICHT geprueft: das Aggregat-Feld selbst
vorlaeufig statt eines Topfs; Partner-Kapitalfelder (KAP_TOEPFE_PARTNER/KAP_ERTRAEGE_PARTNER) unter
Zusammenveranlagung; eine GEMISCHTE Eingabe (ein Topf bestaetigt, ein zweiter vorlaeufig); und die
anderen VIER Injektions-Bloecke derselben Funktion _mit_ring_werten (Verpflegungskuerzung (1),
Paragraph-35a-Haushaltsnahe (4), Paragraph-35c (5) -- (4) filtert nachweislich zustand, (1) und (5)
wurden hier NICHT auf dieselbe Asymmetrie geprueft). Ein AST-/Grep-basierter Ansatz haette diese
Asymmetrie selbst nicht gefunden -- `_kap_positiv` und `_c2` sehen syntaktisch wie normale
Feld-Reader aus, kein Name/Muster unterscheidet sie von einem zustand-gefilterten Reader; nur der
Vergleich mit der Nachbarfunktion _instanz_summe IN DERSELBEN DATEI und die LIVE-Messung legen die
Asymmetrie offen.

HEAD zum Zeitpunkt der Messung: 3bfca6b.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

import pytest

ROOT = os.environ.get("TAXGRAPH_ROOT", "/home/julius/00_projects/168_TaxGraph/taxgraph")
for _sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/mapping", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

os.environ["TAXGRAPH_NO_AUTH"] = "1"   # wie tests/conftest.py -- sonst 401 auf /fall
os.environ.setdefault("ELSTER_HERSTELLER_ID", "00000000000")  # nur lokale XML-Erzeugung, kein Versand

import api as API              # noqa: E402
import server as SRV           # noqa: E402
import audit                   # noqa: E402
import elster_xml as EX        # noqa: E402


def _req(base, method, path, body=None):
    data = json.dumps(body).encode("utf-8") if body is not None else None
    req = urllib.request.Request(base + path, data=data, method=method,
                                  headers={"Content-Type": "application/json"})
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return r.status, json.loads(r.read())
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


def _laie(fld, wert):
    return {"feld_id": fld, "wert": wert, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


def _vorjahr_vorschlag(fld, wert):
    """Ein VORLAEUFIGER Vorjahres-Vorschlag: store.append_event erzwingt zustand=vorlaeufig,
    herkunft=vorjahr, signal_2=None fuer schreiber import:vorjahr (store.py:284-289) -- und
    import:vorjahr ist Katalog-EXEMPT (store.py:333, _vorschlag_typ->None), kann also jedes
    Feld vorschlagen, auch ein KAP-Feld ohne vorschlagbar_von-Eintrag."""
    return {"feld_id": fld, "wert": wert, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "import:vorjahr", "signal": {"signal_1": None, "signal_2": None}}


# Minimale vollstaendige gesamt-Fixtur -- identisch zu tests/test_p20_gewinn_sonstige_e1900701_
# widerspruch.py::_STAMM/_GRUND (dort bereits 2026-08-30 gegen den echten Endpunkt gemessen).
_STAMM = (("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
          ("stammdaten_geburtsdatum", "05.05.1955"),
          ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
          ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
          ("stammdaten_keine_bankverbindung", True),
          ("stammdaten_art_est_erklaerung", True),
          ("kist_konfession", "keine"),
          ("stammdaten_steuernummer", "9181081508155"),
          ("steuerklasse", "1"), ("p36_lohnsteuer", 1200000))

_GRUND = (("bruttoarbeitslohn", 6000000), ("vor_an_anteil_rv", 4200000),
          ("vor_ag_anteil_rv", 1200000), ("vor_rv_ausserhalb_lstb", 0),
          ("kein_gewinn", True), ("kein_vuv", True), ("kein_sonstige", True),
          ("veranlagung", "einzel"),
          ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0),
          ("ep_eigenes_kfz", False), ("versicherungsart", "gesetzlich_an"),
          ("basis_kv", 0), ("basis_pv", 0), ("vorsorge_arbeitslosenversicherung", 0),
          ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0),
          ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
          ("mit_anspruch_auf_zuschuss", False)) + _STAMM

_KAP_NULL = (("kein_kap", True), ("kap_kapitalertraege", 0),
             ("kap_gewinn_aktien", 0), ("kap_verlust_aktien", 0), ("kap_verlust_sonstige", 0))


@pytest.fixture(scope="module")
def gemessen(tmp_path_factory):
    """Baut zwei Faelle ueber den echten HTTP-Server: 'gruen' (kap_gewinn_sonstige VOLL bestaetigt,
    Kontrolle der Messmechanik) und 'leck' (identisch, aber kap_gewinn_sonstige NUR als
    vorlaeufiger Vorjahres-Vorschlag -- nie bestaetigt). Misst /ergebnis, /deklaration UND das
    echte XML ueber einen einzigen, unveraenderten api.einreichen()-Aufruf (Waechter laeuft mit)."""
    faelle_dir = tmp_path_factory.mktemp("faelle")
    API.FAELLE = str(faelle_dir)
    audit.AUDIT_DIR = str(faelle_dir)
    srv = SRV.make_server(0)
    assert srv.server_address[0] == "127.0.0.1"
    th = threading.Thread(target=srv.serve_forever, daemon=True)
    th.start()
    base = f"http://{srv.server_address[0]}:{srv.server_address[1]}"

    xml_erfasst = {}
    orig_erzeuge_xml = EX.erzeuge_xml

    def _spion(*a, **kw):
        text = orig_erzeuge_xml(*a, **kw)
        xml_erfasst["letztes"] = text
        return text

    EX.erzeuge_xml = _spion

    def _messe(fid, kap_events):
        st, r = _req(base, "POST", "/fall",
                     {"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
        assert st == 201, (fid, "fall_anlegen", st, r)
        for fld, w in _GRUND + _KAP_NULL:
            st, r = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
            assert st == 201, (fid, fld, st, r)
        for ev in kap_events:
            st, r = _req(base, "POST", f"/fall/{fid}/event", ev)
            assert st == 201, (fid, ev["feld_id"], st, r)
        st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
        assert st == 200, (fid, "ergebnis", st, erg)
        st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
        assert st == 200, (fid, "deklaration", st, dek)
        xml_erfasst.pop("letztes", None)
        st_e, ein = API.einreichen(fid, {})
        return {"ergebnis": erg, "deklaration": dek, "einreichen": (st_e, ein),
                "xml": xml_erfasst.get("letztes")}

    ergebnisse = {}
    try:
        ergebnisse["baseline"] = _messe("kapleck_baseline", [])
        ergebnisse["gruen"] = _messe("kapleck_gruen", [_laie("kap_gewinn_sonstige", 175000)])
        ergebnisse["leck"] = _messe("kapleck_leck", [_vorjahr_vorschlag("kap_gewinn_sonstige", 175000)])
    finally:
        EX.erzeuge_xml = orig_erzeuge_xml
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()
    return ergebnisse


def test_gruenkontrolle_bestaetigter_topf_konsistent(gemessen):
    """Kontrollfall (KEIN xfail): kap_gewinn_sonstige VOLL bestaetigt erhoeht die Steuer, UND
    /deklaration + das echte XML zeigen dieselbe E1901401-Zahl, UND eingaben_konsistent=True.
    Beweist, dass die Messmechanik selbst funktioniert, bevor der 'leck'-Fall unten aussagekraeftig ist."""
    baseline, gruen = gemessen["baseline"], gemessen["gruen"]

    assert baseline["ergebnis"]["zahl_cent"] is not None, baseline["ergebnis"]
    assert gruen["ergebnis"]["zahl_cent"] is not None, gruen["ergebnis"]
    delta = gruen["ergebnis"]["zahl_cent"] - baseline["ergebnis"]["zahl_cent"]
    assert delta > 0, f"bestaetigter KAP-Topf erhoeht die Steuer nicht (delta={delta}) -- Kontrolle unbrauchbar"

    assert gruen["deklaration"]["eingaben_konsistent"] is True, gruen["deklaration"]
    e1901401_dict = gruen["deklaration"]["deklaration"].get("E1901401")
    assert isinstance(e1901401_dict, int) and e1901401_dict >= 0, gruen["deklaration"]

    assert gruen["xml"] is not None, "kein XML erfasst -- Kontrollfall selbst schon blockiert"
    assert f"<E1901401>{e1901401_dict}</E1901401>" in gruen["xml"], (
        "E1901401 im echten XML weicht vom deklarierten Wert ab oder fehlt")

    print(f"\n[gruen] delta zahl_cent = {delta} ({delta/100:.2f} EUR), E1901401={e1901401_dict} "
          f"in Dict UND XML, eingaben_konsistent=True")


@pytest.mark.xfail(strict=True, reason=(
    "_kap_positiv/_c2 (bescheid_deklaration.py:117-119, 136-137) lesen felder.get(fid)['wert'] OHNE "
    "zustand-Check -- ein NIE bestaetigter (vorlaeufiger) KAP-Topf-Wert loest trotzdem kap_erklaert=True "
    "aus und injiziert E1900401=True/E1901401>0 in die LIVE-Antwort von GET /deklaration, obwohl "
    "derselbe Aufruf eingaben_konsistent=False meldet (die Nachbarfunktion _instanz_summe IN DERSELBEN "
    "DATEI filtert zustand=='bestaetigt' korrekt -- die Asymmetrie ist original, nicht Bauabsicht). "
    "Erreicht NICHT das abgesendete XML (EM.deklariere() blockt einreichen() unabhaengig ueber "
    "eingaben_konsistent=False, 409 deklaration_unvollstaendig, VOR erzeuge_xml()) -- nur der "
    "/deklaration-Dict-Inhalt widerspricht seiner eigenen eingaben_konsistent-Aussage. "
    "Reparaturrichtung offen (zustand-Filter in _kap_positiv/_c2 wie in _instanz_summe, oder die "
    "Injektion an eingaben_konsistent koppeln) -- dieser Test bindet sich an keine davon."))
def test_vorlaeufiger_topf_leckt_in_deklaration_trotz_unvollstaendig(gemessen):
    baseline, leck = gemessen["baseline"], gemessen["leck"]

    # Vorbedingung (kein Teil des Befunds): Copy 1 (Steuer) ist geschuetzt -- sonst waere dies
    # der VIEL groessere Zwei-Signal-Bruch aus test_ui_zwei_signal_sicherheit.py, nicht dieser hier.
    assert leck["ergebnis"]["zahl_cent"] == baseline["ergebnis"]["zahl_cent"], (
        "Steuer bewegt sich schon durch den vorlaeufigen Wert -- anderer, groesserer Befund, "
        "nicht der hier gemessene Deklarations-Leck")

    # Der Waechter, der einreichen() schuetzt, muss auch hier feuern (Kontrolle gegen den
    # eigenen Docstring-Claim oben) -- sonst ist die 'nicht ins XML'-Aussage unbelegt.
    assert leck["deklaration"]["eingaben_konsistent"] is False, leck["deklaration"]
    assert leck["einreichen"][0] == 409, leck["einreichen"]
    assert leck["einreichen"][1].get("grund") == "deklaration_unvollstaendig", leck["einreichen"]
    assert leck["xml"] is None, "XML wurde trotz eingaben_konsistent=False erzeugt -- anderer Befund"

    # DIE Kernaussage: derselbe /deklaration-Aufruf, der eingaben_konsistent=False UND
    # kap_gewinn_sonstige als unvollstaendig meldet, injiziert im selben JSON trotzdem
    # E1900401/E1901401 aus genau diesem unbestaetigten Wert.
    dek = leck["deklaration"]["deklaration"]
    print(f"\n[leck] eingaben_konsistent={leck['deklaration']['eingaben_konsistent']}, "
          f"unvollstaendig={leck['deklaration']['unvollstaendig']}, "
          f"E1900401={dek.get('E1900401')!r}, E1901401={dek.get('E1901401')!r}")

    assert dek.get("E1900401") is None and not dek.get("E1901401"), (
        f"Widerspruch bestaetigt: /deklaration meldet eingaben_konsistent=False UND "
        f"kap_gewinn_sonstige als unvollstaendig, injiziert aber im selben Aufruf "
        f"E1900401={dek.get('E1900401')!r}/E1901401={dek.get('E1901401')!r} aus dem unbestaetigten Wert")
