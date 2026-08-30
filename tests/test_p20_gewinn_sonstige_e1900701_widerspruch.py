"""main-Auftrag 2026-08-30 (nach Korrektur der Bypass-Messung): richtungsneutraler
Regressionstest fuer den ERREICHBAREN Kap.-Sonstige-Defekt.

Befund: kap_gewinn_sonstige (Topf, kein eigenes Kz -- Bindung sagt "Modell-Mismatch") wird ueber
runner.catala_kapital_verrechnung real in die Steuer eingerechnet (Toepfe-Zweig von
_p20_kapitaleinkuenfte, bescheid_einkuenfte.py), aber im ELSTER-XML erscheint E1900701 (das Kz
fuer kap_kapitalertraege, das Aggregat) mit dem WERT 0 -- eine geschriebene, falsche
Tatsachenbehauptung, keine Leerstelle. Nur erreichbar, wenn kap_kapitalertraege selbst 0 bleibt
(Topf-only); bei GLEICHZEITIG positivem Aggregat sperrt der echte Waechter
kapital_semantik_offen (_an_gesamt_sperrgrund, bescheid_deklaration.py) die Zahl komplett -- das
ist bereits getestet (test_kapital_semantik_offen_co_okkurrenz,
tests/test_paket_b_e2e_http.py) und wird hier NICHT wiederholt. Die ERSTE Messung dieses Defekts
ging ueber API._bescheid_fn direkt und umging genau diesen Waechter -- die dabei gefundene Zahl
war real, aber fuer einen Nutzer NICHT erreichbar. Dieser Test erbt die Korrektur.

Messweg: die Steuerzahl kommt AUSSCHLIESSLICH ueber den echten HTTP-Endpunkt
GET /fall/{id}/ergebnis (api.ergebnis() -> _ergebnis_roh(), inkl. Waechter-Aufruf) -- NICHT ueber
_bescheid_fn direkt. Fuer das rohe XML gibt es keinen HTTP-Weg (POST /einreichen liefert nur
xml_bytes-Laenge zurueck, nie den Text) -- deshalb wird api.einreichen() hier EIN EINZIGES Mal
direkt (in-process, gleicher Fall, gleiches FAELLE-Verzeichnis wie der HTTP-Server) aufgerufen,
mit elster_xml.erzeuge_xml() nur BEOBACHTET (monkeypatch faengt den Rueckgabewert ab, aendert
nichts an Argumenten oder Verhalten) -- der Waechter-Aufruf in einreichen() selbst (identischer
Code wie in ergebnis()) laeuft unveraendert und ungebypasst mit.

Gegengeprueft gegen einen sauberen Klon von HEAD 5af945c (kein_p23_verkauf/P23_SCREENING existiert
dort NICHT -- das ist eine fremde, im Arbeitsbaum noch uncommittete Aenderung an
produkt/haut/api_constants.py; SCHEIBEN["gesamt"]["kegel"] verlangt es an diesem HEAD nicht). Diese
Fassung setzt das Feld deshalb NICHT -- sie ist die HEAD-5af945c-reine Fassung.

Was dieser Test NICHT behauptet: er belegt einen Widerspruch zwischen der festgesetzten Steuer und
dem Vordruck IM SELBEN LAUF -- nicht, dass 187,00 EUR der materiell richtige Steuerbetrag sind
(die Toepfe-Verrechnung selbst wird hier nicht auf Richtigkeit geprueft), und nicht, dass 1.750 EUR
in E1900701 oder ein anderes Kennzeichen gehoeren. Er prueft nur: dieselbe 0 steht in Dict UND XML,
waehrend die Steuerberechnung sie nicht als 0 behandelt. Die Reparaturrichtung ist offen.
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


# Minimale vollstaendige gesamt-Fixtur, uebernommen aus tests/test_einreichen_durchstich.py
# (dort 2026-08-10 team-lead-gemessen gegen den echten Endpunkt: _STAMM_A/_BASIS_A).
_STAMM = (("stammdaten_nachname", "Maier"), ("stammdaten_vorname", "Hans"),
          ("stammdaten_geburtsdatum", "05.05.1955"),
          ("stammdaten_strasse", "Musterstr."), ("stammdaten_hausnummer", "55"),
          ("stammdaten_plz", "55555"), ("stammdaten_wohnort", "Musterort"),
          ("stammdaten_keine_bankverbindung", True),
          ("stammdaten_art_est_erklaerung", True),
          ("kist_konfession", "keine"),
          ("stammdaten_steuernummer", "9181081508155"),
          ("steuerklasse", "1"), ("p36_lohnsteuer", 1200000))

# "kegel" von /ergebnis (api_constants.SCHEIBEN["gesamt"]["kegel"]) verlangt zusaetzlich
# EP_FELDER + KV_PV_FELDER unbedingt (kein "kein_X"-Schalter unterdrueckt sie) -- gemessen per
# Probe-Skript: /ergebnis lieferte ohne diese 13 Felder grund="input_kegel_nicht_bestaetigt".
# Werte identisch zu den Defaults in tests/test_paket_b_e2e_http.py::_gesamt_kegel.
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


def _kap(kein_kap, ertraege, gewinn_sonstige):
    return (("kein_kap", kein_kap),
            ("kap_kapitalertraege", ertraege), ("kap_gewinn_aktien", 0),
            ("kap_verlust_aktien", 0), ("kap_gewinn_sonstige", gewinn_sonstige),
            ("kap_verlust_sonstige", 0))


_FAELLE = {
    "zero": _kap(True, 0, 0),                 # Baseline: kein Kapital
    "green": _kap(False, 175000, 0),          # Kontrolle: Aggregat 1.750 EUR, Toepfe 0
    "red": _kap(False, 0, 175000),             # zu pruefender Fall: Topf 1.750 EUR, Aggregat 0
}


@pytest.fixture(scope="module")
def gemessen(tmp_path_factory):
    """Baut alle drei Faelle ueber den echten HTTP-Server, misst die Steuer ausschliesslich ueber
    GET /ergebnis, und holt das echte XML ueber EINEN direkten, unveraenderten Aufruf von
    api.einreichen() (Waechter laeuft darin mit, erzeuge_xml() wird nur beobachtet)."""
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

    ergebnisse = {}
    try:
        for name, kap_events in _FAELLE.items():
            fid = f"p20test_{name}"
            st, r = _req(base, "POST", "/fall",
                         {"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
            assert st == 201, (name, "fall_anlegen", st, r)
            for fld, w in _GRUND + kap_events:
                st, r = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
                assert st == 201, (name, fld, st, r)

            st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
            assert st == 200, (name, "ergebnis", st, erg)

            st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
            assert st == 200, (name, "deklaration", st, dek)

            xml_erfasst.pop("letztes", None)
            st_e, ein = API.einreichen(fid, {})
            xml_text = xml_erfasst.get("letztes")

            ergebnisse[name] = {"ergebnis": erg, "deklaration": dek,
                                "einreichen": (st_e, ein), "xml": xml_text}
    finally:
        EX.erzeuge_xml = orig_erzeuge_xml
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()
    return ergebnisse


def test_gruenkontrolle_aggregat_konsistent(gemessen):
    """Aggregat allein (Topf=0): Steuer steigt gegenueber der Kapital-Null-Baseline, UND E1900701
    zeigt im echten XML genau den deklarierten Betrag -- Steuer und Deklaration passen zueinander.
    Kein xfail: Kontrollfall, an dem die Messmechanik selbst geprueft wird, bevor der rote Fall
    unten ueberhaupt aussagekraeftig ist."""
    zero, green = gemessen["zero"], gemessen["green"]

    for name in ("zero", "green"):
        assert gemessen[name]["ergebnis"]["grund"] != "kapital_semantik_offen", (
            f"{name}: Waechter feuert unerwartet -- Fixtur pruefen")
        assert gemessen[name]["ergebnis"]["zahl_cent"] is not None, gemessen[name]["ergebnis"]
        assert gemessen[name]["einreichen"][1].get("grund") != "kapital_semantik_offen", (
            f"{name}: Waechter feuert in einreichen() unerwartet")

    delta = green["ergebnis"]["zahl_cent"] - zero["ergebnis"]["zahl_cent"]
    assert delta > 0, (
        f"Aggregat 1.750 EUR erhoeht die Steuer nicht (delta={delta} cent) -- "
        f"Kontrollfall selbst schon inkonsistent, Messung unbrauchbar")

    assert green["deklaration"]["deklaration"].get("E1900701") == 1750, green["deklaration"]

    assert green["xml"] is not None, "kein XML erfasst -- einreichen() vor erzeuge_xml() abgebrochen"
    assert "<E1900701>1750</E1900701>" in green["xml"], (
        "E1900701 fehlt im echten XML oder zeigt einen anderen Wert als deklariert")

    print(f"\n[gruen] delta zahl_cent = {delta} ({delta/100:.2f} EUR), E1900701=1750 in Dict+XML")


@pytest.mark.xfail(strict=True, reason=(
    "kap_gewinn_sonstige (Topf, kein eigenes Kz) wird real besteuert, aber E1900701 "
    "(kap_kapitalertraege-Kz) steht im echten ELSTER-XML explizit auf 0 -- eine geschriebene "
    "Tatsachenbehauptung, kein fehlendes Feld. Steuer und Deklaration widersprechen sich im "
    "selben Lauf. Reparaturrichtung offen (eigenes Kz fuer den Topf, Aggregat mit einrechnen, "
    "oder E1900701 unterdruecken statt 0 zu schreiben) -- dieser Test bindet sich an keine "
    "davon, nur an den Widerspruch."))
def test_topf_only_steuer_und_deklaration_widersprechen_sich(gemessen):
    zero, red = gemessen["zero"], gemessen["red"]

    assert red["ergebnis"]["grund"] != "kapital_semantik_offen", (
        "Waechter feuert -- dieser Fall waere dann NICHT der erreichbare Defekt "
        "(s. test_kapital_semantik_offen_co_okkurrenz), sondern korrekt gesperrt")
    assert red["ergebnis"]["zahl_cent"] is not None, red["ergebnis"]

    delta = red["ergebnis"]["zahl_cent"] - zero["ergebnis"]["zahl_cent"]
    steuer_erhoeht = delta > 0
    assert steuer_erhoeht, (
        f"Kapital wird nicht wie erwartet besteuert (delta={delta} cent) -- "
        f"Vorbedingung fuer den Widerspruch fehlt")

    dek = red["deklaration"]["deklaration"]
    e1900701_geschrieben_null = ("E1900701" in dek) and (dek["E1900701"] == 0)

    assert red["xml"] is not None, "kein XML erfasst -- einreichen() vor erzeuge_xml() abgebrochen"
    e1900701_im_xml_null = "<E1900701>0</E1900701>" in red["xml"]

    print(f"\n[rot] delta zahl_cent = {delta} ({delta/100:.2f} EUR), "
          f"E1900701 dict={dek.get('E1900701')!r}, im XML als 0 gefunden={e1900701_im_xml_null}")

    # DIE Kernaussage: Steuer wurde erhoben (oben bewiesen), UND E1900701 behauptet im
    # abgabefertigen XML explizit 0 -- nicht "fehlt", sondern geschrieben und falsch.
    assert not (e1900701_geschrieben_null and e1900701_im_xml_null), (
        f"Widerspruch bestaetigt: Steuer +{delta/100:.2f} EUR auf kap_gewinn_sonstige, "
        f"aber E1900701 steht im Dict UND im echten XML auf 0 (dict={dek.get('E1900701')!r})")
