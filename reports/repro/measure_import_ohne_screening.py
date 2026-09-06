#!/usr/bin/env python3
"""Messung: kann Betrag übers Import kommen, ohne Screening-Kreuz?

Hypothese: import:beleg/import:kontoauszug schreiben Betragsfeld, ohne dass
das zugehörige Screening-Kreuz je gepostet wurde.

Szenarien:
1. IMPORT OHNE KREUZ: nur Betrag (import:beleg), kein Screening-Kreuz vorher
   → misst die Hypothese

2. KONTROLLE 1: Betrag NACH Kreuz beantwortet
   → wenn beide dasselbe liefern, ist Reihenfolge nicht das Problem

3. KONTROLLE 2: Import ohne Betrag
   → wenn sich nichts ändert, war der Betrag nicht die Ursache

Messwerte: /ergebnis, /einreichen, /deklaration nebeneinander mit Zahlen+Einheiten.
Zusätzlich: Ist das Screening-Feld nach Import wirklich ABWESEND oder nur leer?
"""
import json
import os
import sys
import tempfile
import threading

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))

sys.path.insert(0, os.path.join(ROOT, "produkt", "haut"))
sys.path.insert(0, os.path.join(ROOT, "produkt", "store"))
sys.path.insert(0, os.path.join(ROOT, "tests"))

import api as API
import server as SRV
import store as ST
import test_paket_b_e2e_http as T

def measure_endpoints(base, fall_id, scenario_name):
    """Misst /ergebnis, /einreichen, /deklaration für einen Fall."""
    print(f"\n{'='*80}")
    print(f"SZENARIO: {scenario_name}")
    print(f"{'='*80}")

    # /ergebnis
    print("\n→ GET /fall/{id}/ergebnis")
    st_e, erg = T._req(base, "GET", f"/fall/{fall_id}/ergebnis")
    print(f"  Status: {st_e}")
    print(f"  grund: {erg.get('grund')}")
    print(f"  zahl_cent: {erg.get('zahl_cent')}")
    print(f"  eingaben_konsistent: {erg.get('eingaben_konsistent')}")

    # /einreichen
    print("\n→ POST /fall/{id}/einreichen")
    try:
        st_r, res = T._req(base, "POST", f"/fall/{fall_id}/einreichen", {}, erwarte=None)
        # nicht mit erwarte=409, weil wir auch 201 akzeptieren
    except AssertionError as e:
        # Fehler aufgefangen, aber weitermachen
        st_r = None
        res = {"fehler": str(e)}

    if st_r:
        print(f"  Status: {st_r}")
        print(f"  grund: {res.get('grund')}")
        print(f"  eingereicht: {res.get('eingereicht')}")
        if st_r >= 400:
            print(f"  → Submission gesperrt/abgelehnt")
        else:
            print(f"  → Submission OK")
    else:
        print(f"  Fehler: {res.get('fehler')}")

    # /deklaration
    print("\n→ GET /fall/{id}/deklaration")
    st_d, dek = T._req(base, "GET", f"/fall/{fall_id}/deklaration")
    print(f"  Status: {st_d}")
    print(f"  eingaben_konsistent: {dek.get('eingaben_konsistent')}")
    print(f"  vollstaendig: {dek.get('vollstaendig')}")

    return {"ergebnis": erg, "einreichen": res, "deklaration": dek, "st_e": st_e, "st_r": st_r, "st_d": st_d}

def check_field_presence(store, field_id):
    """Prüft, ob ein Feld im Store abwesend oder vorhanden-mit-Wert ist."""
    events = store.get("events", [])
    for ev in events:
        if ev.get("feld_id") == field_id:
            return f"vorhanden, zustand={ev.get('zustand')}, wert={ev.get('wert')}"
    return "ABWESEND (kein Event)"

def main():
    with tempfile.TemporaryDirectory() as tmp_path:
        API.FAELLE = os.path.join(tmp_path, "faelle")
        os.makedirs(API.FAELLE, exist_ok=True)

        srv = SRV.make_server(0)
        th = threading.Thread(target=srv.serve_forever, daemon=True)
        th.start()

        base = f"http://{srv.server_address[0]}:{srv.server_address[1]}"

        try:
            # Wähle ein Feld mit Screening-Kreuz und Betrag.
            # Beispiel: vv_einnahmen (Vermietung Einnahmen)
            # Screening-Kreuz: kein_vuv (False = Vermietung vorhanden)
            # Betrag-Feld: vv_einnahmen (Cent)

            # Szenario 1: IMPORT OHNE KREUZ (nur Betrag über Import)
            print("\n" + "="*80)
            print("SZENARIO 1: IMPORT OHNE SCREENING-KREUZ")
            print("="*80)
            FALL_1 = "import_ohne_kreuz"

            # Fall anlegen
            st, _ = T._req(base, "POST", "/fall", {
                "scheibe": "gesamt",
                "veranlagungszeitraum": 2025,
                "fall_id": FALL_1
            })
            assert st == 201, f"Fall-Anlage fehlgeschlagen: {st}"

            # NUR Betrag posten (import:beleg), NICHT das Screening-Kreuz
            # kein_vuv=False heißt "hat Vermietung", aber das Kreuz wollen wir NICHT posten
            st, _ = T._req(base, "POST", f"/fall/{FALL_1}/event", {
                "feld_id": "vv_einnahmen",
                "wert": 3000000,  # 30.000 € in Cent
                "zustand": "vorlaeufig",
                "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                "schreiber": "import:beleg",
                "signal": {"signal_1": None, "signal_2": None}
            })
            assert st == 201, f"Betrag-Event fehlgeschlagen: {st}"

            # Store laden um Feld-Status zu prüfen
            store_1 = ST.lade(os.path.join(API.FAELLE, FALL_1, "store.json"))
            kein_vuv_status = check_field_presence(store_1, "kein_vuv")
            vv_einnahmen_status = check_field_presence(store_1, "vv_einnahmen")

            print(f"\nFeld-Status nach Import:")
            print(f"  kein_vuv (Screening-Kreuz): {kein_vuv_status}")
            print(f"  vv_einnahmen (Betrag): {vv_einnahmen_status}")

            res_1 = measure_endpoints(base, FALL_1, "Import ohne Kreuz")

            # Szenario 2: KONTROLLE 1 — Kreuz vorher beantwortet
            print("\n" + "="*80)
            print("KONTROLLE 1: KREUZ VORHER BEANTWORTET")
            print("="*80)
            FALL_2 = "kreuz_dann_betrag"

            st, _ = T._req(base, "POST", "/fall", {
                "scheibe": "gesamt",
                "veranlagungszeitraum": 2025,
                "fall_id": FALL_2
            })
            assert st == 201

            # Erst Kreuz beantwortet
            st, _ = T._req(base, "POST", f"/fall/{FALL_2}/event", T._laie("kein_vuv", False))
            assert st == 201

            # Dann Betrag über Import
            st, _ = T._req(base, "POST", f"/fall/{FALL_2}/event", {
                "feld_id": "vv_einnahmen",
                "wert": 3000000,  # gleicher Betrag
                "zustand": "vorlaeufig",
                "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                "schreiber": "import:beleg",
                "signal": {"signal_1": None, "signal_2": None}
            })
            assert st == 201

            store_2 = ST.lade(os.path.join(API.FAELLE, FALL_2, "store.json"))
            kein_vuv_status_2 = check_field_presence(store_2, "kein_vuv")
            vv_einnahmen_status_2 = check_field_presence(store_2, "vv_einnahmen")

            print(f"\nFeld-Status nach Kreuz + Import:")
            print(f"  kein_vuv (Screening-Kreuz): {kein_vuv_status_2}")
            print(f"  vv_einnahmen (Betrag): {vv_einnahmen_status_2}")

            res_2 = measure_endpoints(base, FALL_2, "Kreuz beantwortet, dann Betrag")

            # Szenario 3: KONTROLLE 2 — Import ohne Betrag
            print("\n" + "="*80)
            print("KONTROLLE 2: IMPORT OHNE BETRAG")
            print("="*80)
            FALL_3 = "import_ohne_betrag"

            st, _ = T._req(base, "POST", "/fall", {
                "scheibe": "gesamt",
                "veranlagungszeitraum": 2025,
                "fall_id": FALL_3
            })
            assert st == 201

            # Import mit leerem Betrag oder gar kein Betrag-Event
            # Nur ein Dummy-Event von import:beleg, aber kein vv_einnahmen
            # (das ist etwas konstruiert, aber relevant: zeigt ob die Sperre vom Betrag kommt)
            st, _ = T._req(base, "POST", f"/fall/{FALL_3}/event", T._laie("vv_erhaltungsaufwand", 100000))
            assert st == 201

            store_3 = ST.lade(os.path.join(API.FAELLE, FALL_3, "store.json"))
            kein_vuv_status_3 = check_field_presence(store_3, "kein_vuv")
            vv_einnahmen_status_3 = check_field_presence(store_3, "vv_einnahmen")

            print(f"\nFeld-Status ohne Import-Betrag:")
            print(f"  kein_vuv (Screening-Kreuz): {kein_vuv_status_3}")
            print(f"  vv_einnahmen (Betrag): {vv_einnahmen_status_3}")

            res_3 = measure_endpoints(base, FALL_3, "Kein Import-Betrag")

            # Vergleich
            print("\n" + "="*80)
            print("VERGLEICH DER SZENARIOS")
            print("="*80)

            print("\n1. Import ohne Kreuz:")
            print(f"   /ergebnis zahl_cent: {res_1['ergebnis'].get('zahl_cent')}")
            print(f"   /ergebnis grund: {res_1['ergebnis'].get('grund')}")

            print("\n2. Kreuz + dann Betrag:")
            print(f"   /ergebnis zahl_cent: {res_2['ergebnis'].get('zahl_cent')}")
            print(f"   /ergebnis grund: {res_2['ergebnis'].get('grund')}")

            print("\n3. Kein Betrag:")
            print(f"   /ergebnis zahl_cent: {res_3['ergebnis'].get('zahl_cent')}")
            print(f"   /ergebnis grund: {res_3['ergebnis'].get('grund')}")

            if res_1['ergebnis'].get('zahl_cent') != res_2['ergebnis'].get('zahl_cent'):
                print("\n⚠️ BEFUND: Szenario 1 und 2 UNTERSCHEIDEN sich in zahl_cent!")
                print("   → Reihenfolge Kreuz/Betrag hat Effekt")
            else:
                print("\n✓ Szenario 1 und 2 liefern DASSELBE zahl_cent")
                print("   → Reihenfolge ist NICHT das Problem")

            if res_1['ergebnis'].get('zahl_cent') != res_3['ergebnis'].get('zahl_cent'):
                print("\n⚠️ BEFUND: Szenario 1 und 3 UNTERSCHEIDEN sich in zahl_cent!")
                print("   → Der Betrag selbst ist relevant")
            else:
                print("\n✓ Szenario 1 und 3 liefern DASSELBE zahl_cent")
                print("   → Der Betrag selbst ist NICHT relevant")

        finally:
            srv.shutdown()
            th.join(timeout=5)
            srv.server_close()

if __name__ == "__main__":
    main()
