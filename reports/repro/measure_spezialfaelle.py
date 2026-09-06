#!/usr/bin/env python3
"""Messung: Spezialfälle — ändert sich das Formular, wenn das Feld gefüllt wird?

Für jedes Feld:
1. Fall leer (nur Stammdaten) → Formular merken
2. Denselben Fall, dieses Feld mit Betrag → Formular merken
3. Unterschied in Kennzahlen? ja/nein
"""
import json
import os
import sys
import tempfile
import threading
import urllib.error
import urllib.request
import xml.etree.ElementTree as ET

ROOT = os.environ.get("TAXGRAPH_ROOT", "/home/julius/00_projects/168_TaxGraph/taxgraph")
for _sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/mapping", "golden"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

os.environ["TAXGRAPH_NO_AUTH"] = "1"
os.environ.setdefault("ELSTER_HERSTELLER_ID", "00000000000")

import api as API
import server as SRV
import elster_xml as EX

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

# Minimale Stammdaten (nur essentiell)
_STAMM = (("stammdaten_nachname", "Test"), ("stammdaten_vorname", "Fall"),
          ("stammdaten_geburtsdatum", "05.05.1950"),
          ("stammdaten_strasse", "Str."), ("stammdaten_hausnummer", "1"),
          ("stammdaten_plz", "12345"), ("stammdaten_wohnort", "Ort"),
          ("stammdaten_keine_bankverbindung", True),
          ("stammdaten_art_est_erklaerung", True),
          ("kist_konfession", "keine"),
          ("stammdaten_steuernummer", "9181081508155"),
          ("steuerklasse", "1"), ("veranlagung", "einzel"),
          ("p36_lohnsteuer", 1000000))  # 10.000 EUR Lohnsteuer als Basis

# Spezialfälle zum Prüfen
# Kontrollzeilen + ausgewählte Spezialfälle (Start mit einfachen)
SPEZIAL = [
    # Kontrollzeilen (MUSS funktionieren)
    ("_control_gewinn_gewerbe", [("einkuenfte_gewinn", 50000), ("gewinn_betriebsart", "gewerbe"), ("gewinn_bezeichnung", "Test")]),
    ("_control_gewinn_land_forst", [("einkuenfte_gewinn", 50000), ("gewinn_betriebsart", "land_forst"), ("gewinn_bezeichnung", "Test")]),

    # Die 15 Spezialfälle — aber mit minimalen Bedingungen
    # Vermietung-Felder (VV): setzen kein_vuv=False
    ("vv_schuldzinsen", [("kein_vuv", False), ("vv_schuldzinsen", 30000)]),
    ("vv_gebaeude_afa", [("kein_vuv", False), ("vv_gebaeude_afa", 80000)]),
    ("vv_sonstige_wk", [("kein_vuv", False), ("vv_sonstige_wk", 25000)]),
    ("vv_erhaltungsaufwand", [("kein_vuv", False), ("vv_erhaltungsaufwand", 50000)]),

    # Andere (probieren ohne extra Bedingung)
    ("am_anschaffungskosten", [("am_anschaffungskosten", 100000)]),
    ("behinderungsbedingte_aufwendungen", [("behinderungsbedingte_aufwendungen", 50000)]),
    ("p36_vorauszahlungen", [("p36_vorauszahlungen", 20000)]),
]

def extract_kz_values(xml_text):
    """Extrahiere alle Kz-Werte aus XML als Dict."""
    if not xml_text:
        return {}
    try:
        root = ET.fromstring(xml_text)
    except:
        return {}

    kz_values = {}
    for elem in root.iter():
        tag = elem.tag
        if "}" in tag:
            tag = tag.rsplit("}", 1)[-1]
        if len(tag) == 10 and tag.startswith("E") and elem.text:
            kz_values[tag] = elem.text
    return kz_values

def main():
    with tempfile.TemporaryDirectory() as tmp_path:
        API.FAELLE = os.path.join(tmp_path, "faelle")
        os.makedirs(API.FAELLE, exist_ok=True)

        srv = SRV.make_server(0)
        th = threading.Thread(target=srv.serve_forever, daemon=True)
        th.start()
        base = f"http://{srv.server_address[0]}:{srv.server_address[1]}"

        xml_cache = {}
        orig_erzeuge_xml = EX.erzeuge_xml

        def _spion(*a, **kw):
            text = orig_erzeuge_xml(*a, **kw)
            xml_cache["latest"] = text
            return text

        EX.erzeuge_xml = _spion

        print("="*100)
        print("MESSUNG: Spezialfälle — Formeländerung bei Betrag")
        print("="*100)
        print()

        results = []

        try:
            for test_name, zusatz_felder in SPEZIAL:
                fid_leer = f"{test_name}_leer"
                fid_betrag = f"{test_name}_mit"

                # Fall 1: Leer
                st, _ = _req(base, "POST", "/fall", {
                    "scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid_leer
                })
                assert st == 201
                for fld, wert in _STAMM:
                    st, r = _req(base, "POST", f"/fall/{fid_leer}/event", _laie(fld, wert))
                    if st != 201:
                        print(f"FEHLER: {test_name} / {fld} -> {st}: {r}")
                    assert st == 201, (fid_leer, fld, st, r)

                xml_cache.pop("latest", None)
                _req(base, "POST", f"/fall/{fid_leer}/einreichen", {})
                xml_leer = extract_kz_values(xml_cache.get("latest", ""))

                # Fall 2: Mit Betrag
                st, _ = _req(base, "POST", "/fall", {
                    "scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": fid_betrag
                })
                assert st == 201
                for fld, wert in _STAMM:
                    st, _ = _req(base, "POST", f"/fall/{fid_betrag}/event", _laie(fld, wert))
                    assert st == 201
                for fld, wert in zusatz_felder:
                    st, _ = _req(base, "POST", f"/fall/{fid_betrag}/event", _laie(fld, wert))
                    assert st == 201

                xml_cache.pop("latest", None)
                _req(base, "POST", f"/fall/{fid_betrag}/einreichen", {})
                xml_betrag = extract_kz_values(xml_cache.get("latest", ""))

                # Unterschied finden
                changed_kz = []
                for kz in sorted(set(xml_leer.keys()) | set(xml_betrag.keys())):
                    leer_wert = xml_leer.get(kz, "−")
                    betrag_wert = xml_betrag.get(kz, "−")
                    if leer_wert != betrag_wert:
                        changed_kz.append(f"{kz}({leer_wert}→{betrag_wert})")

                aendert_str = ", ".join(changed_kz) if changed_kz else "NICHTS"
                results.append((test_name, aendert_str))
                print(f"{test_name:45s}  aendert: {aendert_str}")

        finally:
            EX.erzeuge_xml = orig_erzeuge_xml
            srv.shutdown()
            th.join(timeout=5)
            srv.server_close()

        print()
        print("="*100)
        print("ERGEBNISSE")
        print("="*100)
        for test_name, aendert_str in results:
            print(f"{test_name:45s}  aendert: {aendert_str}")

        nichts_count = sum(1 for _, a in results if a == "NICHTS")
        total = len(results)
        print()
        print(f"Geprüft: {total}, davon aendert NICHTS: {nichts_count}")

if __name__ == "__main__":
    main()
