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

STAND 2026-10-03: behoben, in Python UND Rust (Entscheid kap-vorschau-liest-nur-bestaetigte-werte): der
Antrag-Block liest nur bestaetigte Werte, der xfail-Marker ist entfallen. Die oben als „NICHT geprueft"
genannten Eingaben sind jetzt je ein Fall (AK4): das Aggregat, ein gemischter Zustand (ein Topf bestaetigt,
einer vorlaeufig) und der Partner — in `gesamt` ist der vorlaeufige Partner-KAP-Wert NICHT erreichbar
(`partner_kegel_offen` sperrt /deklaration vorher), in `rentner_gesamt` erreichbar und gefiltert. Die
uebrigen Einspeise-Bloecke von `_mit_ring_werten` (Verpflegung, § 35c) sind nicht Gegenstand dieser Datei.
"""
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
for _sub in ("produkt/haut", "produkt/eingang", "produkt/store", "produkt/mapping", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

os.environ["TAXGRAPH_NO_AUTH"] = "1"   # wie tests/conftest.py -- sonst 401 auf /fall

import api as API              # noqa: E402
import server as SRV           # noqa: E402
import audit                   # noqa: E402
import elster_xml as EX        # noqa: E402


@pytest.fixture(scope="module", autouse=True)
def _fake_hersteller_id():
    """Hersteller-ID NUR fuer die Dauer dieser Datei setzen -- nur lokale XML-Erzeugung, kein Versand.

    Vorher stand hier `os.environ.setdefault(...)` auf MODULEBENE. Das leckt: pytest importiert
    beim Sammeln jede Datei des Workers, bevor der erste Test laeuft, die Fake-ID lag damit auch
    fuer fremde Dateien in der Umgebung. Gemessen 2026-09-26: in test_checkest_feldmatrix.py
    hielten sich die ERiC-Tests dadurch fuer lauffaehig und scheiterten mit rc=610301200 statt zu
    skippen, und test_p35a_einzelaufstellung_alle_drei_toepfe_amtlich_plausibel riss den ganzen
    `make unit`-Lauf mit. Ein Test, der die Umgebung eines anderen veraendert, ist kein Test.
    """
    alt = os.environ.get("ELSTER_HERSTELLER_ID")
    if alt is None:                       # wie setdefault: eine echte ID bleibt unangetastet
        os.environ["ELSTER_HERSTELLER_ID"] = "00000000000"
    try:
        yield
    finally:
        if alt is None:
            os.environ.pop("ELSTER_HERSTELLER_ID", None)
        else:
            os.environ["ELSTER_HERSTELLER_ID"] = alt


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

# GEBAUT, nicht kopiert (tests/_kegel.py). Von Hand standen hier 20 Felder; 17 trugen genau den
# Abwesenheitswert (gemessen 2026-09-26) und waren reine Kopie. Nur die drei unten sind echte Werte.
# Die Handliste kannte 14 Kegel-Mitglieder nicht -- darunter `agb_zwangslaeufig` und
# `agb_notwendig_angemessen`; sobald die im Kegel standen, sperrte die Datei, statt zu messen.
from _kegel import kegel_fuer  # noqa: E402

_GRUND_BASIS = {
    "bruttoarbeitslohn": 6000000,
    "vor_an_anteil_rv": 4200000,
    "vor_ag_anteil_rv": 1200000,
}
# _KAP_NULL ist entfallen: kap_kapitalertraege/_gewinn_aktien/_verlust_aktien/_verlust_sonstige
# tragen alle den Abwesenheitswert 0 und kommen jetzt aus dem Bauer.


def _grund(auslassen=(), zusammen=False):
    """Voller gesamt-Kegel mit den echten Fallwerten, plus _STAMM (das sind KEINE Kegel-Felder).

    `auslassen` sind Felder, die der Test DANACH als eigenes Event schickt (hier: die kap_events,
    darunter ein bewusst vorlaeufiger Wert). Sie duerfen im Bauer nicht vorbelegt sein -- sonst
    kaeme dasselbe Feld zweimal (Store weist das nach Auflage B mit 422 ab) oder der Bauer
    ueberschriebe genau den Messgegenstand. `zusammen`: Zusammenveranlagung, der Partner-Kegel
    kommt mit dem Abwesenheitswert aus dem Bauer.
    """
    basis = {**_GRUND_BASIS, "veranlagung": "zusammen"} if zusammen else _GRUND_BASIS
    raus = [(f, w) for f, w in kegel_fuer("gesamt", basis)
            if f not in set(auslassen)]
    return raus + list(_STAMM)


# Partner-Kegel unter Zusammenveranlagung in `gesamt`: GESAMT_PARTNER_19 + GESAMT_PARTNER_KAP gehoeren
# NICHT zum Bauer-Kegel, der Guard verlangt sie aber bestaetigt (sonst partner_kegel_offen).
_PARTNER_NULL = ("bruttoarbeitslohn_partner", "kap_kapitalertraege_partner", "kap_gewinn_aktien_partner",
                 "kap_gewinn_sonstige_partner", "kap_verlust_aktien_partner", "kap_verlust_sonstige_partner")


def _grund_rentner(auslassen=()):
    """Voller rentner_gesamt-Kegel, Zusammenveranlagung, eine Rente. Die Scheibe fuehrt KEINE KAP-Betraege
    im Kegel (nur `kein_kap`) und hat keinen Partner-KAP-Guard; vorlaeufige KAP-Werte erreichen hier
    `/deklaration`, ohne dass `an_gesamt_sperrgrund` etwas meldet."""
    basis = {"rentner_jahresrente": 2000000, "rentner_renten_beginn_jahr": 2025,
             "kein_sonstige": False, "veranlagung": "zusammen"}
    return [(f, w) for f, w in kegel_fuer("rentner_gesamt", basis) if f not in set(auslassen)]


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
    # Hersteller-ID nur fuer die Dauer dieser Fixtur (lokale XML-Erzeugung, kein Versand). Auf
    # Modulebene gesetzt galt der elfstellige Dummy ab der Sammlung fuer JEDEN Test im Prozess:
    # ERiC-Tests bauten damit schema-widriges XML (rc=610301200), statt mangels ID zu skippen.
    hid = pytest.MonkeyPatch()
    if "ELSTER_HERSTELLER_ID" not in os.environ:
        hid.setenv("ELSTER_HERSTELLER_ID", "00000000000")

    def _messe(fid, kap_events, zusammen=False, scheibe="gesamt", partner_null=False, erwarte_dek=200):
        st, r = _req(base, "POST", "/fall",
                     {"fall_id": fid, "scheibe": scheibe, "veranlagungszeitraum": 2025})
        assert st == 201, (fid, "fall_anlegen", st, r)
        # Die kap_events des Aufrufers bleiben draussen: der Test schickt sie gleich selbst,
        # und der 'leck'-Fall schickt dort bewusst einen VORLAEUFIGEN Wert -- der Bauer darf
        # den Messgegenstand nicht vorbelegen.
        eigene = [ev["feld_id"] for ev in kap_events]
        grund = (_grund_rentner(auslassen=eigene) if scheibe == "rentner_gesamt"
                 else _grund(auslassen=eigene, zusammen=zusammen))
        if partner_null:
            grund += [(f, 0) for f in _PARTNER_NULL if f not in eigene]
        for fld, w in grund:
            st, r = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
            assert st == 201, (fid, fld, st, r)
        for ev in kap_events:
            st, r = _req(base, "POST", f"/fall/{fid}/event", ev)
            assert st == 201, (fid, ev["feld_id"], st, r)
        st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
        assert st == 200, (fid, "ergebnis", st, erg)
        st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
        assert st == erwarte_dek, (fid, "deklaration", st, dek)
        xml_erfasst.pop("letztes", None)
        st_e, ein = API.einreichen(fid, {})
        return {"ergebnis": erg, "deklaration": dek, "einreichen": (st_e, ein),
                "xml": xml_erfasst.get("letztes")}

    ergebnisse = {}
    try:
        ergebnisse["baseline"] = _messe("kapleck_baseline", [_laie("kein_kap", True)])
        # kein_kap=False noetig: ein bestaetigter kap_gewinn_sonstige>0 neben kein_kap=True
        # widerspricht sich selbst -- flag_check.flag_widersprueche() sperrt das zu Recht
        # (live gemessen 2026-08-31: grund=flag_konsistenz_offen ohne diese Zeile).
        ergebnisse["gruen"] = _messe("kapleck_gruen",
                                      [_laie("kein_kap", False), _laie("kap_gewinn_sonstige", 175000)])
        ergebnisse["leck"] = _messe("kapleck_leck",
                                     [_laie("kein_kap", False), _vorjahr_vorschlag("kap_gewinn_sonstige", 175000)])
        # AK4: das Aggregat statt eines Topfs; ein Topf bestaetigt, ein zweiter vorlaeufig (unter dem
        # Sparer-Pauschbetrag, damit 400 gegen 700 EUR unterscheidbar bleibt).
        ergebnisse["aggregat"] = _messe("kapleck_aggregat",
                                         [_laie("kein_kap", False), _vorjahr_vorschlag("kap_kapitalertraege", 175000)])
        ergebnisse["gemischt_kontrolle"] = _messe("kapleck_gem_kontrolle",
                                                   [_laie("kein_kap", False), _laie("kap_gewinn_aktien", 40000)])
        ergebnisse["gemischt"] = _messe("kapleck_gemischt",
                                         [_laie("kein_kap", False), _laie("kap_gewinn_aktien", 40000),
                                          _vorjahr_vorschlag("kap_gewinn_sonstige", 30000)])
        # AK4 Partner-KAP in `gesamt`: der Guard (GESAMT_PARTNER_KAP) sperrt /deklaration vorher.
        ergebnisse["partner_gesamt_gruen"] = _messe(
            "kapleck_pg_gruen", [_laie("kein_kap_partner", False), _laie("kap_gewinn_sonstige_partner", 175000)],
            zusammen=True, partner_null=True)
        ergebnisse["partner_gesamt_leck"] = _messe(
            "kapleck_pg_leck", [_laie("kein_kap_partner", False), _vorjahr_vorschlag("kap_gewinn_sonstige_partner", 175000)],
            zusammen=True, partner_null=True, erwarte_dek=409)
        # AK4 in `rentner_gesamt` (Zusammenveranlagung): hier erreichen vorlaeufige KAP-Werte /deklaration.
        ergebnisse["rentner_gruen"] = _messe(
            "kapleck_r_gruen", [_laie("kap_gewinn_sonstige_partner", 175000)], scheibe="rentner_gesamt")
        ergebnisse["rentner_eigen"] = _messe(
            "kapleck_r_eigen", [_laie("kein_kap", False), _vorjahr_vorschlag("kap_gewinn_sonstige", 175000)],
            scheibe="rentner_gesamt")
        ergebnisse["rentner_partner"] = _messe(
            "kapleck_r_partner", [_vorjahr_vorschlag("kap_gewinn_sonstige_partner", 175000)],
            scheibe="rentner_gesamt")
        ergebnisse["rentner_partner_aggregat"] = _messe(
            "kapleck_r_partner_aggr", [_vorjahr_vorschlag("kap_kapitalertraege_partner", 175000)],
            scheibe="rentner_gesamt")
    finally:
        EX.erzeuge_xml = orig_erzeuge_xml
        hid.undo()
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()
    return ergebnisse


def test_gruenkontrolle_bestaetigter_topf_konsistent(gemessen, braucht_echtes_xsd):
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


def test_vorbedingung_leck_fall_ist_nirgends_abgabefaehig(gemessen):
    """Die Vorbedingung des Tests unten (der bis 2026-10-03 xfail(strict) war), als eigener Test (Entscheid
    ein-erwarteter-fehlschlag-traegt-nur-die-kernaussage-..., 2026-10-03). Sie bleibt getrennt: scheitert
    sie, misst der Test unten nichts mehr (kein Leck-Fall ohne Zahl, ohne 409, ohne XML).

    Der Leck-Fall ist ueberall anderweitig gesperrt: `/ergebnis` liefert keine Zahl (der Kegel ist
    nicht bestaetigt -- NICHT der Guard `_an_gesamt_sperrgrund`, deshalb sperrt `/deklaration`
    hier auch nicht, s. u.), `einreichen()` bricht mit 409 `deklaration_unvollstaendig` VOR dem XML
    ab, und es entsteht kein XML.
    """
    leck = gemessen["leck"]
    assert leck["ergebnis"]["zahl_cent"] is None, leck["ergebnis"]
    assert leck["ergebnis"]["grund"] == "input_kegel_nicht_bestaetigt", leck["ergebnis"]
    assert leck["einreichen"][0] == 409, leck["einreichen"]
    assert leck["einreichen"][1].get("grund") == "deklaration_unvollstaendig", leck["einreichen"]
    assert leck["xml"] is None, "XML wurde trotz eingaben_konsistent=False erzeugt -- anderer Befund"


def test_vorlaeufiger_topf_leckt_in_deklaration_trotz_unvollstaendig(gemessen):
    """Behoben 2026-10-03 (Entscheid kap-vorschau-liest-nur-bestaetigte-werte): `_kap_positiv`/`_c2` in
    `_mit_ring_werten` lesen nur Werte mit Zustand „bestaetigt", wie die Nachbarfunktion `_instanz_summe`.
    Vorher xfail(strict): ein NIE bestaetigter KAP-Topf loeste kap_erklaert aus und injizierte
    E1900401=True/E1901401>0 in GET /deklaration, obwohl dieselbe Antwort eingaben_konsistent=False meldet.
    Der Leck erreichte nie das XML (409 deklaration_unvollstaendig VOR erzeuge_xml); die Sperre von
    /deklaration bei Sperrgrund erfasst den Fall nicht (der Guard meldet nichts)."""
    leck = gemessen["leck"]

    # DIE Kernaussage: derselbe /deklaration-Aufruf, der eingaben_konsistent=False UND
    # kap_gewinn_sonstige als unvollstaendig meldet, injiziert im selben JSON trotzdem
    # E1900401/E1901401 aus genau diesem unbestaetigten Wert. Die Vorbedingungen (Zahl, einreichen,
    # XML) stehen im eigenen Test oben; ein anderer Fehler als AssertionError (z. B. ein KeyError,
    # wenn /deklaration ploetzlich 409 liefert) macht diesen Test rot statt "erwartet".
    assert leck["deklaration"]["eingaben_konsistent"] is False, leck["deklaration"]
    unvollstaendig_felder = {u["feld_id"] for u in leck["deklaration"]["unvollstaendig"]}
    assert "kap_gewinn_sonstige" in unvollstaendig_felder, leck["deklaration"]["unvollstaendig"]

    dek = leck["deklaration"]["deklaration"]
    print(f"\n[leck] eingaben_konsistent={leck['deklaration']['eingaben_konsistent']}, "
          f"unvollstaendig={leck['deklaration']['unvollstaendig']}, "
          f"E1900401={dek.get('E1900401')!r}, E1901401={dek.get('E1901401')!r}")

    assert dek.get("E1900401") is None and not dek.get("E1901401"), (
        f"Widerspruch bestaetigt: /deklaration meldet eingaben_konsistent=False UND "
        f"kap_gewinn_sonstige als unvollstaendig, injiziert aber im selben Aufruf "
        f"E1900401={dek.get('E1900401')!r}/E1901401={dek.get('E1901401')!r} aus dem unbestaetigten Wert")


def _kein_antrag(fall, unvollstaendig_feld):
    """Gemeinsame Kernaussage der AK4-Faelle: /deklaration meldet das Feld als unvollstaendig, injiziert aber
    weder den Antrag noch einen genutzten Sparer-Pauschbetrag aus dem unbestaetigten Wert."""
    dek = fall["deklaration"]
    assert dek["eingaben_konsistent"] is False, dek
    assert unvollstaendig_feld in {u["feld_id"] for u in dek["unvollstaendig"]}, dek["unvollstaendig"]
    assert dek["deklaration"].get("E1900401") is None and not dek["deklaration"].get("E1901401"), (
        f"{unvollstaendig_feld} nur vorlaeufig, aber E1900401={dek['deklaration'].get('E1900401')!r}/"
        f"E1901401={dek['deklaration'].get('E1901401')!r}")


def test_vorlaeufiges_aggregat_leckt_nicht_in_deklaration(gemessen):
    """AK4: dieselbe Aussage fuer `kap_kapitalertraege` (das Aggregat statt eines Topfs)."""
    _kein_antrag(gemessen["aggregat"], "kap_kapitalertraege")


def test_gemischter_zustand_zaehlt_nur_den_bestaetigten_topf(gemessen):
    """AK4: Aktiengewinn 400 EUR bestaetigt, sonstiger Gewinn 300 EUR nur vorlaeufig. Der Antrag steht (ein
    bestaetigter Topf erklaert Kapitalertraege), der genutzte Sparer-Pauschbetrag zaehlt aber nur die 400 EUR
    des bestaetigten Topfs — gleich der Kontrolle ohne den vorlaeufigen Topf. Vorher: 700."""
    kontrolle, gemischt = gemessen["gemischt_kontrolle"], gemessen["gemischt"]
    kd, gd = kontrolle["deklaration"]["deklaration"], gemischt["deklaration"]["deklaration"]
    assert kontrolle["deklaration"]["eingaben_konsistent"] is True, kontrolle["deklaration"]
    assert (kd.get("E1900401"), kd.get("E1901401")) == (True, 400), kd      # KONTROLLE: der Aufbau misst
    assert gemischt["deklaration"]["eingaben_konsistent"] is False, gemischt["deklaration"]
    assert "kap_gewinn_sonstige" in {u["feld_id"] for u in gemischt["deklaration"]["unvollstaendig"]}
    assert (gd.get("E1900401"), gd.get("E1901401")) == (kd.get("E1900401"), kd.get("E1901401")), (gd, kd)


def test_partner_kap_in_gesamt_sperrt_der_guard_vor_der_vorschau(gemessen):
    """AK4 „nicht erreichbar": in `gesamt` verlangt der Guard (`GESAMT_PARTNER_KAP`) alle Partner-KAP-Felder
    bestaetigt. Ein vorlaeufiger Partner-Wert ergibt 409 partner_kegel_offen, noch bevor `_mit_ring_werten`
    liest. Kontrolle: derselbe Fall mit bestaetigtem Wert antwortet 200 und traegt den Antrag."""
    gruen, leck = gemessen["partner_gesamt_gruen"], gemessen["partner_gesamt_leck"]
    dg = gruen["deklaration"]
    assert dg["eingaben_konsistent"] is True and dg["deklaration"].get("E1900401") is True, dg
    assert leck["deklaration"]["grund"] == "partner_kegel_offen", leck["deklaration"]


def test_rentner_partner_gruenkontrolle(gemessen):
    """Kontrolle fuer die Rentner-Faelle: ein bestaetigter Partner-Topf traegt Antrag und Pauschbetrag."""
    d = gemessen["rentner_gruen"]["deklaration"]
    assert d["eingaben_konsistent"] is True, d
    assert (d["deklaration"].get("E1900401"), d["deklaration"].get("E1901401")) == (True, 1750), d["deklaration"]


@pytest.mark.parametrize("fall,feld", [
    ("rentner_eigen", "kap_gewinn_sonstige"),
    ("rentner_partner", "kap_gewinn_sonstige_partner"),
    ("rentner_partner_aggregat", "kap_kapitalertraege_partner"),
])
def test_rentner_vorlaeufiger_kap_wert_leckt_nicht_in_deklaration(gemessen, fall, feld):
    """AK4 in `rentner_gesamt` (Zusammenveranlagung): die Scheibe hat keinen Partner-KAP-Guard und keine KAP-
    Betraege im Kegel, `/ergebnis` meldet ring_betrag_vorlaeufig — der Guard schweigt, /deklaration antwortet
    200. Eigener Topf, Partner-Topf und Partner-Aggregat: keiner darf einen Antrag ausloesen."""
    assert gemessen[fall]["ergebnis"]["grund"] == "ring_betrag_vorlaeufig", gemessen[fall]["ergebnis"]
    _kein_antrag(gemessen[fall], feld)
