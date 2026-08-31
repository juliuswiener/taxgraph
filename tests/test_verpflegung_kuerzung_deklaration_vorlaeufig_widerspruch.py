"""main-Auftrag 2026-08-31 (Folgeauftrag zum KAP-Zustand-Leck, s. tests/test_kap_deklaration_
vorlaeufig_leck_ohne_bestaetigung.py): dieselbe Zustand-Blindheit, ein ANDERER Block DERSELBEN
Funktion (_mit_ring_werten, bescheid_deklaration.py).

Volle Lektuere der gesamten Funktion (Zeilen 58-345, nicht nur ein Block) ergibt eine Tabelle mit
GENAU ZWEI blinden Bloecken -- kein dritter:

  (1) Verpflegungskuerzung E0205508      Z. 97-114   BLIND (dieser Test)
  (2)+(3) KAP Antrag/Sparer-PB           Z. 116-168  BLIND (bereits getestet, s.o.)
  (4) Sec35a Haushaltsnahe Summen        Z. 173-197  KORREKT (_instanz_summe Z.179 prueft zustand)
  Anlage V Mieteinnahmen-Summe           Z. 199-251  KORREKT (3x expliziter zustand-Check)
  Sec35c Sanierung Einzelbetrag          Z. 253-262  KORREKT (Z.254)
  GewSt zu zahlen                        Z. 264-284  KORREKT (Z.276-277, beide Faktoren)
  Sec22 Nr.3 Einzelbetrag+WK             Z. 286-314  KORREKT (Z.295-296, beide Eingaben)
  Sec10 Berufsausbildung Einzelbetrag    Z. 316-332  KORREKT (Z.324)
  Sec35c Foerderung-Umkehr               Z. 334-343  KORREKT (Z.335)

Befund (live gemessen, s.u.): Zeile 102-103 baut `s = {fid: e["wert"] if isinstance(e, dict) else e
for fid, e in felder.items()}` OHNE zustand-Filter und reicht das an runner._verpflegung_kuerzung_
cent() durch. Reichweite bestaetigt: tage_24h/tage_an_abreise/tage_ueber_8h_eintaegig tragen alle
drei `vorjahr: vorschlag` (bindung_n_vor_gwg.yaml Z.774/791/808) -- derselbe reale Kanal
(import:vorjahr, store.py Z.284-289 erzwingt herkunft=vorjahr/zustand=vorlaeufig/signal_2=null)
wie beim KAP-Fund.

ANDERS als beim KAP-Fund: hier existiert BEREITS ein zustandssensibler Waechter, der GET /ergebnis
und einreichen() vor der Zahl selbst schuetzt (bescheid_deklaration.py, `zahlen_bestaetigt = any(
felder.get(f).get("zustand") == "bestaetigt" ...)` fuer die Mahlzeiten-Anzahl-Felder) --
"verpflegung_reduktion_offen" feuert VOR jeder Zahl, nicht erst bei der Abgabe. Staerker geschuetzt
als KAP, wo /ergebnis eine (zufaellig unveraenderte) Zahl zurueckgab. Der Injektions-Block selbst
(Zeile 97-114) ist trotzdem blind: er laeuft VOR diesem Waechter, innerhalb von /deklaration
(deklaration() in api.py ruft _mit_ring_werten() OHNE den _an_gesamt_sperrgrund-Waechter davor zu
pruefen -- das ist derselbe Aufbau wie bei KAP).

Was Teil 3 angeht (reicht der Leck bis ins abgabefertige XML?): NEIN. EM.deklariere() prueft jedes
Feld eigenstaendig auf zustand=="bestaetigt" (est_mapping.py:582) -- tage_24h selbst bleibt
deshalb aus dem Dict UND dem XML draussen (E0205409 fehlt), waehrend E0205508 (die Kuerzung, aus
genau diesem unbestaetigten Wert berechnet) trotzdem im Dict auftaucht: eine Kuerzung ohne die
Position, die sie kuerzen soll. einreichen() blockt mit 409 "verpflegung_reduktion_offen" VOR
erzeuge_xml() (live bestaetigt: kein XML erfasst) -- derselbe Waechter, der schon /ergebnis
sperrt, schuetzt hier zufaellig auch die Abgabe mit (er kennt den Injektions-Leck nicht, er
reagiert nur auf die unbeantwortete Mahlzeiten-Frage).

Der Leck reicht aber bis in die LIVE-Antwort von GET /fall/{id}/deklaration: derselbe Aufruf, der
eingaben_konsistent=False UND tage_24h/vpf_fruehstuecke_gestellt_anzahl als unvollstaendig
zurueckgibt, traegt im selben JSON-Objekt E0205508=28 (in Cent-Vordruckeinheit: 28 EURO) --
intern widerspruechlich, exakt dieselbe Bauform wie der KAP-Fund.

Was dieser Test NICHT behauptet: dass 28 EUR (E0205508) oder 100 Tage/492 EUR Steuerwirkung
(Kontrollfall) materiell richtige Betraege sind (die Mahlzeitenkuerzungs-Rechnung selbst wird hier
nicht auf Richtigkeit geprueft); dass der Leck das abgesendete XML erreicht (belegt das GENAUE
GEGENTEIL fuer den hier gemessenen Fall -- ein Waechter faengt ihn ab, siehe oben); dass ein
Nutzer diesen JSON-Widerspruch je zu Gesicht bekommt (Frontend-Kopplung von /deklaration nicht
erneut geprueft, s. KAP-Test dazu). Er behauptet nur: derselbe /deklaration-Aufruf widerspricht
sich in sich selbst (eingaben_konsistent=False neben einem injizierten "bestaetigt"-Wert aus
genau den Feldern, die als unvollstaendig gemeldet werden).

BAUFORM, die dieser Messweg NICHT sieht: nur EIN Eingabe-Vektor (Einzelveranlagung, ein
Mahlzeiten-Anzahl-Feld vorlaeufig, tage_24h vorlaeufig, tage_an_abreise/tage_ueber_8h_eintaegig
gar nicht gesetzt). Nicht geprueft: gemischte Zustaende (z.B. tage_24h bestaetigt, Mahlzeiten-
Anzahl vorlaeufig, oder umgekehrt); die Drei-Monats-Frist-Felder (vpf_tage_*_nach_drei_monaten);
Zusammenveranlagung/Partner-Felder (es gibt keine Partner-Variante dieses Blocks in der Bindung).

HEAD zum Zeitpunkt der Messung: e8387f69c5247755dc5dc8aeb2745ee8db53348b (sauberer Klon,
/tmp/tg_clean_clone, .env nicht kopiert).
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
    import:vorjahr ist Katalog-EXEMPT (store.py:333, _vorschlag_typ->None)."""
    return {"feld_id": fld, "wert": wert, "zustand": "vorlaeufig",
            "herkunft": {"herkunft": "vorjahr", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "import:vorjahr", "signal": {"signal_1": None, "signal_2": None}}


# Minimale vollstaendige gesamt-Fixtur -- identisch zu tests/test_p20_gewinn_sonstige_e1900701_
# widerspruch.py::_STAMM/_GRUND (dort 2026-08-30 gegen den echten Endpunkt gemessen).
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
          ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", True),
          ("veranlagung", "einzel"),
          ("ep_arbeitstage", 0), ("ep_entfernung_km", 0), ("ep_oepnv_kosten", 0),
          ("ep_eigenes_kfz", False), ("versicherungsart", "gesetzlich_an"),
          ("basis_kv", 0), ("basis_pv", 0), ("vorsorge_arbeitslosenversicherung", 0),
          ("vorsorge_erwerbsunfaehigkeit", 0), ("vorsorge_unfall_haftpflicht", 0),
          ("vorsorge_rv_alt_mit_ueberschuss", 0), ("vorsorge_rv_alt_ohne_ueberschuss", 0),
          ("mit_anspruch_auf_zuschuss", False)) + _STAMM

# 100 Tage * 28 EUR = 2.800 EUR Pauschale, klar ueber dem AN-Pauschbetrag (1.230 EUR/2025) --
# sonst maskiert der Pauschbetrag jede Steuerwirkung auch im bestaetigten Kontrollfall.
TAGE_24H = 100
FRUEHSTUECKE = 5


@pytest.fixture(scope="module")
def gemessen(tmp_path_factory):
    """Baut drei Faelle ueber den echten HTTP-Server: 'baseline' (keine Verpflegungs-Felder),
    'gruen' (tage_24h + Mahlzeiten-Anzahl VOLL bestaetigt) und 'leck' (identische Werte, aber
    NUR als vorlaeufiger Vorjahres-Vorschlag). Misst /ergebnis, /deklaration, /stand UND das
    echte XML ueber einen einzigen, unveraenderten api.einreichen()-Aufruf je Fall."""
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

    def _messe(fid, verpflegung_events):
        st, r = _req(base, "POST", "/fall",
                     {"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
        assert st == 201, (fid, "fall_anlegen", st, r)
        for fld, w in _GRUND:
            st, r = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
            assert st == 201, (fid, fld, st, r)
        for ev in verpflegung_events:
            st, r = _req(base, "POST", f"/fall/{fid}/event", ev)
            assert st == 201, (fid, ev["feld_id"], st, r)

        st, stand = _req(base, "GET", f"/fall/{fid}/stand")
        assert st == 200, (fid, "stand", st, stand)
        tage_zustand = (stand.get("felder", {}).get("tage_24h") or {}).get("zustand")

        st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
        assert st == 200, (fid, "ergebnis", st, erg)
        st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
        assert st == 200, (fid, "deklaration", st, dek)
        xml_erfasst.pop("letztes", None)
        st_e, ein = API.einreichen(fid, {})
        return {"ergebnis": erg, "deklaration": dek, "einreichen": (st_e, ein),
                "xml": xml_erfasst.get("letztes"), "tage_24h_zustand": tage_zustand}

    ergebnisse = {}
    try:
        ergebnisse["baseline"] = _messe("vpflk_baseline", [])
        ergebnisse["gruen"] = _messe("vpflk_gruen", [
            _laie("tage_24h", TAGE_24H), _laie("vpf_fruehstuecke_gestellt_anzahl", FRUEHSTUECKE)])
        ergebnisse["leck"] = _messe("vpflk_leck", [
            _vorjahr_vorschlag("tage_24h", TAGE_24H),
            _vorjahr_vorschlag("vpf_fruehstuecke_gestellt_anzahl", FRUEHSTUECKE)])
    finally:
        EX.erzeuge_xml = orig_erzeuge_xml
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()
    return ergebnisse


def test_erreichbarkeit_vorlaeufig_via_import_vorjahr(gemessen):
    """Schritt 1 (main: 'Erreichbarkeit zuerst, nicht den Schaden'): GET /stand bestaetigt, dass
    tage_24h ueber den echten import:vorjahr-Kanal tatsaechlich mit zustand=vorlaeufig persistiert
    -- kein Hand-Dict, echte store.append_event-Validierung ueber HTTP."""
    assert gemessen["gruen"]["tage_24h_zustand"] == "bestaetigt", gemessen["gruen"]
    assert gemessen["leck"]["tage_24h_zustand"] == "vorlaeufig", gemessen["leck"]


def test_gruenkontrolle_bestaetigte_tage_konsistent(gemessen):
    """Kontrollfall (KEIN xfail): tage_24h+Mahlzeiten-Anzahl VOLL bestaetigt senken die Steuer
    (Pauschale > AN-Pauschbetrag), UND /deklaration + das echte XML zeigen dieselben E0205409/
    E0205508-Zahlen, UND eingaben_konsistent=True. Beweist, dass die Messmechanik selbst
    funktioniert, bevor der 'leck'-Fall unten aussagekraeftig ist."""
    baseline, gruen = gemessen["baseline"], gemessen["gruen"]

    assert baseline["ergebnis"]["zahl_cent"] is not None, baseline["ergebnis"]
    assert gruen["ergebnis"]["zahl_cent"] is not None, gruen["ergebnis"]
    delta = gruen["ergebnis"]["zahl_cent"] - baseline["ergebnis"]["zahl_cent"]
    assert delta < 0, (
        f"bestaetigte Verpflegungspauschale senkt die Steuer nicht (delta={delta} cent) -- "
        f"Kontrolle unbrauchbar")

    assert gruen["deklaration"]["eingaben_konsistent"] is True, gruen["deklaration"]
    dek = gruen["deklaration"]["deklaration"]
    assert dek.get("E0205409") == TAGE_24H, dek
    kuerzung = dek.get("E0205508")
    assert isinstance(kuerzung, int) and kuerzung > 0, dek

    assert gruen["xml"] is not None, "kein XML erfasst -- Kontrollfall selbst schon blockiert"
    assert f"<E0205409>{TAGE_24H}</E0205409>" in gruen["xml"], (
        "E0205409 im echten XML weicht vom deklarierten Wert ab oder fehlt")
    assert f"<E0205508>{kuerzung}</E0205508>" in gruen["xml"], (
        "E0205508 im echten XML weicht vom deklarierten Wert ab oder fehlt")

    print(f"\n[gruen] delta zahl_cent = {delta} ({delta/100:.2f} EUR), E0205409={TAGE_24H}, "
          f"E0205508={kuerzung} in Dict UND XML, eingaben_konsistent=True")


@pytest.mark.xfail(strict=True, reason=(
    "Zeile 102-103 in _mit_ring_werten (bescheid_deklaration.py) baut die Eingabe fuer "
    "runner._verpflegung_kuerzung_cent() OHNE zustand-Filter (`e['wert'] if isinstance(e, dict) "
    "else e` fuer JEDES Feld) -- ein NIE bestaetigter (vorlaeufiger) Vorjahres-Vorschlag fuer "
    "tage_24h/vpf_fruehstuecke_gestellt_anzahl loest trotzdem eine Kuerzungsberechnung aus und "
    "injiziert E0205508>0 in die LIVE-Antwort von GET /deklaration, obwohl derselbe Aufruf "
    "eingaben_konsistent=False meldet und tage_24h/vpf_fruehstuecke_gestellt_anzahl selbst als "
    "unvollstaendig listet (est_mapping.py:582 filtert PRO FELD zustand=='bestaetigt', deshalb "
    "fehlt E0205409 im selben Dict -- eine Kuerzung ohne die Position, die sie kuerzen soll). "
    "Derselbe Zustand-Blindheit-Fehler wie in _kap_positiv/_c2 (s. "
    "test_kap_deklaration_vorlaeufig_leck_ohne_bestaetigung.py), hier in einem ANDEREN Block "
    "derselben Funktion. Erreicht NICHT das abgesendete XML (ein bereits bestehender, "
    "zustand-sensitiver Waechter [verpflegung_reduktion_offen] blockt GET /ergebnis UND "
    "einreichen() schon VOR jeder Zahl, unabhaengig von diesem Injektions-Leck) -- nur der "
    "/deklaration-Dict-Inhalt widerspricht seiner eigenen eingaben_konsistent-Aussage. "
    "Reparaturrichtung offen (zustand-Filter in Zeile 102-103 wie in _instanz_summe, oder die "
    "Injektion an eingaben_konsistent koppeln) -- dieser Test bindet sich an keine davon."))
def test_vorlaeufige_tage_leckt_kuerzung_in_deklaration_trotz_unvollstaendig(gemessen):
    baseline, leck = gemessen["baseline"], gemessen["leck"]

    # Vorbedingung (kein Teil des Befunds): /ergebnis liefert HIER GAR KEINE Zahl -- ein
    # bereits bestehender, zustand-sensitiver Waechter (verpflegung_reduktion_offen) sperrt
    # staerker als beim KAP-Fund (dort kam noch eine unveraenderte Zahl zurueck).
    assert leck["ergebnis"]["zahl_cent"] is None, leck["ergebnis"]
    assert leck["ergebnis"]["grund"] == "verpflegung_reduktion_offen", leck["ergebnis"]

    # Der Waechter, der einreichen() schuetzt, muss auch hier feuern (Kontrolle gegen den
    # eigenen Docstring-Claim oben) -- sonst ist die "nicht ins XML"-Aussage unbelegt.
    assert leck["einreichen"][0] == 409, leck["einreichen"]
    assert leck["einreichen"][1].get("grund") == "verpflegung_reduktion_offen", leck["einreichen"]
    assert leck["xml"] is None, "XML wurde trotz gesperrtem Ergebnis erzeugt -- anderer Befund"

    # DIE Kernaussage: derselbe /deklaration-Aufruf, der eingaben_konsistent=False UND
    # tage_24h/vpf_fruehstuecke_gestellt_anzahl als unvollstaendig meldet, injiziert im selben
    # JSON trotzdem E0205508 aus genau diesen unbestaetigten Werten.
    assert leck["deklaration"]["eingaben_konsistent"] is False, leck["deklaration"]
    unvollstaendig_felder = {u["feld_id"] for u in leck["deklaration"]["unvollstaendig"]}
    assert {"tage_24h", "vpf_fruehstuecke_gestellt_anzahl"} <= unvollstaendig_felder, (
        leck["deklaration"]["unvollstaendig"])

    dek = leck["deklaration"]["deklaration"]
    print(f"\n[leck] eingaben_konsistent={leck['deklaration']['eingaben_konsistent']}, "
          f"unvollstaendig={sorted(unvollstaendig_felder)}, "
          f"E0205409={dek.get('E0205409')!r}, E0205508={dek.get('E0205508')!r}")

    assert dek.get("E0205409") is None and not dek.get("E0205508"), (
        f"Widerspruch bestaetigt: /deklaration meldet eingaben_konsistent=False UND "
        f"tage_24h/vpf_fruehstuecke_gestellt_anzahl als unvollstaendig, injiziert aber im "
        f"selben Aufruf E0205508={dek.get('E0205508')!r} EUR Kuerzung ohne die zugehoerige "
        f"Pauschale (E0205409={dek.get('E0205409')!r}) je deklariert zu haben")
