"""Instructor-Auftrag 2026-08-31: kap_gewinn_sonstige (und sein Person-B-Zwilling) tragen einen
elster_kz_grund, der ein Ziel-Kz BEHAUPTET ("Wert in E1900701"), aber es gibt dafuer keine
Additionslogik in est_mapping.py -- der Betrag landet in KEINER Kz, weder im Dict noch im
echten XML.

Live gemessen (HEAD 2729b0d): kap_gewinn_sonstige=175000 Cent (1.750,00 EUR) erhoeht die Steuer
um 18700 Cent (187,00 EUR) gegenueber einer Kontrollzeile mit 0 -- der Ring rechnet den Betrag
real ein. Trotzdem: /deklaration liefert einen nicht_deklariert-Eintrag mit GENAU dem
elster_kz_grund-Text aus der Bindung, UND das echte XML (via einreichen(), Waechter laeuft mit)
enthaelt die Ziffernfolge "1750" kein einziges Mal.

Anker geprueft (E10-2025.xsd, ERiC 44.2.4.0, Zeile 19420-19447): E1900701 = "Kapitalertraege"
(bereits korrekt an kap_kapitalertraege gebunden). Die vier Geschwister-Kz (E1900901/E1900904/
E1900804/E1901101) sind ALLE als Teilmengen-Zeilen von E1900701 beschriftet ("in Zeile E1900701
enthalten") -- keine davon ist ein eigenstaendiges Additions-Ziel, keine passt auf "sonstige
Kapitalgewinne, ausser Aktien". E1900701 SELBST ist laut amtlichem Vordruck (Anlage_KAP_2025.pdf
Zeile 7 + 040_Anleitung_Anlage_KAP_2025.pdf S.1) eine echte Summenzeile -- aber zweckgebunden auf
Betraege "laut Steuerbescheinigung(en)" mit inlaendischem Steuerabzug. Die Bindung erhebt fuer
kap_gewinn_sonstige (Signatur-Slot p20_6_verlustverrechnung/gewinn_sonstige, fachlich fuer die
Verlustverrechnungs-Topf-Logik gedacht) NICHT, ob der Betrag aus einer solchen Bescheinigung
stammt. Ein additiver Fix wuerde deshalb eine unbelegte Bescheinigungs-Aussage ins Formular
schreiben -- Instructor-Adjudikation 2026-08-31: NICHT BAUEN. Kein amtliches, sicher belegtes
Ziel fuer diesen Betrag gefunden; dies ist Festnagelung des Ist-Zustands, keine Reparatur.

Zweiter, identischer Kandidat (Spiegelbild): kap_gewinn_sonstige_partner
(bindung_kap_vv_familie.yaml:267-280), grund "Wert in E1900701 (person_b)" -- hier NICHT
mitgemessen (eigener Zusammenveranlagungs-Pfad noetig), aber derselbe Code-Zweig
(est_mapping.py Klasse c) traegt ihn ebenso ohne Mapping-Eintrag.

Pin-Form: keine bare xfail(strict=True) (die pinnt nur "scheitert", nicht WORAN) -- stattdessen
direkte Zusicherung auf den exakten aktuellen (fehlerhaften) Rueckgabewert. Repariert jemand die
Verdrahtung, faellt dieser Test LAUT durch (AssertionError mit dem neuen Wert), nicht xfail-leise.
"""
from __future__ import annotations

import json
import os
import sys
import threading
import urllib.error
import urllib.request

ROOT = os.environ.get("TAXGRAPH_ROOT", "/home/julius/00_projects/168_TaxGraph/taxgraph")
for _sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/mapping", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

os.environ["TAXGRAPH_NO_AUTH"] = "1"   # wie tests/conftest.py -- sonst 401 auf /fall
os.environ.setdefault("ELSTER_HERSTELLER_ID", "00000000000")  # nur lokale XML-Erzeugung, kein Versand

import api as API              # noqa: E402
import server as SRV           # noqa: E402
import audit                   # noqa: E402
import elster_xml as EX        # noqa: E402

import pytest


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


def _kap(gewinn_sonstige):
    return (("kein_kap", False),
            ("kap_kapitalertraege", 0), ("kap_gewinn_aktien", 0),
            ("kap_verlust_aktien", 0), ("kap_gewinn_sonstige", gewinn_sonstige),
            ("kap_verlust_sonstige", 0))


@pytest.fixture(scope="module")
def gemessen(tmp_path_factory):
    """Zwei Faelle ueber den echten HTTP-Server: 'kontrolle_null' (kap_gewinn_sonstige=0) und
    'betrag_175000' (1.750,00 EUR). Misst /ergebnis, /deklaration UND das echte XML ueber einen
    einzigen, unveraenderten api.einreichen()-Aufruf (Waechter laeuft mit)."""
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
        for name, gs in (("kontrolle_null", 0), ("betrag_175000", 175000)):
            fid = f"kzloch_{name}"
            st, r = _req(base, "POST", "/fall",
                         {"fall_id": fid, "scheibe": "gesamt", "veranlagungszeitraum": 2025})
            assert st == 201, (name, "fall_anlegen", st, r)
            for fld, w in _GRUND + _kap(gs):
                st, r = _req(base, "POST", f"/fall/{fid}/event", _laie(fld, w))
                assert st == 201, (name, fld, st, r)
            st, erg = _req(base, "GET", f"/fall/{fid}/ergebnis")
            assert st == 200, (name, "ergebnis", st, erg)
            st, dek = _req(base, "GET", f"/fall/{fid}/deklaration")
            assert st == 200, (name, "deklaration", st, dek)
            xml_erfasst.pop("letztes", None)
            st_e, ein = API.einreichen(fid, {})
            ergebnisse[name] = {"ergebnis": erg, "deklaration": dek,
                                "einreichen": (st_e, ein), "xml": xml_erfasst.get("letztes")}
    finally:
        EX.erzeuge_xml = orig_erzeuge_xml
        srv.shutdown()
        th.join(timeout=5)
        srv.server_close()
    return ergebnisse


def test_gruenkontrolle_ring_rechnet_den_betrag_ein(gemessen):
    """Kontrollzeile (KEIN xfail): 1.750 EUR kap_gewinn_sonstige erhoeht die Steuer gegenueber
    0 EUR -- der Ring behandelt den Betrag als real, das Loch unten ist also kein Nichts-
    eingegeben-Fall, sondern ein Deklarations-Leck bei einem tatsaechlich wirksamen Betrag."""
    kontrolle, betrag = gemessen["kontrolle_null"], gemessen["betrag_175000"]
    assert kontrolle["ergebnis"]["zahl_cent"] is not None, kontrolle["ergebnis"]
    assert betrag["ergebnis"]["zahl_cent"] is not None, betrag["ergebnis"]
    delta = betrag["ergebnis"]["zahl_cent"] - kontrolle["ergebnis"]["zahl_cent"]
    assert delta == 18700, (
        f"1.750 EUR kap_gewinn_sonstige aendert die Steuer nicht um die erwarteten 187,00 EUR "
        f"(delta={delta} Cent) -- Kontrollzeile selbst schon abweichend, Messung unbrauchbar")
    print(f"\n[kontrolle] delta zahl_cent = {delta} Cent ({delta/100:.2f} EUR) -- Ring rechnet mit")


def test_1750_euro_landet_in_keiner_kz(gemessen):
    """DIE Kernaussage, exakt festgenagelt (kein bare xfail): der nicht_deklariert-Eintrag traegt
    woertlich den elster_kz_grund-Text aus der Bindung, und '1750' kommt im echten XML NULL Mal
    vor. Aendert sich einer der beiden Werte (Reparatur ODER Regression), faellt dieser Test mit
    dem NEUEN Wert in der AssertionError-Meldung durch -- kein stilles Weiterlaufen."""
    betrag = gemessen["betrag_175000"]

    nd = betrag["deklaration"].get("nicht_deklariert", [])
    treffer = [e for e in nd if e.get("feld_id") == "kap_gewinn_sonstige"]
    assert len(treffer) == 1, (
        f"erwarte genau EINEN nicht_deklariert-Eintrag fuer kap_gewinn_sonstige, gefunden: {treffer!r} "
        f"(voller nicht_deklariert: {nd!r})")
    assert treffer[0]["grund"] == (
        "ENDGUELTIG: Modell-Mismatch. Wert in E1900701. E1900904 enger als unser Feld."
    ), treffer[0]

    dek = betrag["deklaration"]["deklaration"]
    # E1900701 steht im Dict (von kap_kapitalertraege=0), aber NICHT mit dem 1750-Betrag addiert.
    assert dek.get("E1900701") == 0, (
        f"E1900701 traegt nicht mehr 0 -- pruefen, ob kap_gewinn_sonstige jetzt eingerechnet wird "
        f"(dann ist DIESER Fund repariert): {dek.get('E1900701')!r}")

    assert betrag["xml"] is not None, "kein XML erfasst -- einreichen() vor erzeuge_xml() abgebrochen"
    assert "1750" not in betrag["xml"], (
        "'1750' steht jetzt im XML -- die 1.750 EUR erreichen eine Kz, DIESER Fund ist repariert; "
        "Test muss aktualisiert werden (nicht nur gruen laufen lassen)")

    print(f"\n[betrag_175000] nicht_deklariert-Grund={treffer[0]['grund']!r}, "
          f"E1900701 im Dict={dek.get('E1900701')!r}, '1750' im XML=False -- "
          f"1.750 EUR erreichen keine Kz, obwohl der Ring sie einrechnet (delta oben)")
