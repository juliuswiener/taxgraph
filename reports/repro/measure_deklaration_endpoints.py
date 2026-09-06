#!/usr/bin/env python3
"""Messung: /deklaration vs. /ergebnis vs. /einreichen beim selben Sperrfall.

Sperrfall: KAP Semantik offen (E1900701 + E1900901 gleichzeitig bestaetigt).
- GET /ergebnis → grund=kapital_semantik_offen, zahl_cent=None (GRUEN)
- POST /einreichen → 409, grund=kapital_semantik_offen (GRUEN)
- GET /deklaration → HTTP 200, eingaben_konsistent=True (BUG: keine Sperrmeldung)

Script lädt den Test-Fall und misst alle drei Endpunkte nebeneinander.
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

def main():
    with tempfile.TemporaryDirectory() as tmp_path:
        # Patch API und Server auf temp-dir
        API.FAELLE = os.path.join(tmp_path, "faelle")
        os.makedirs(API.FAELLE, exist_ok=True)

        # Server starten
        srv = SRV.make_server(0)
        th = threading.Thread(target=srv.serve_forever, daemon=True)
        th.start()

        base = f"http://{srv.server_address[0]}:{srv.server_address[1]}"

        try:
            # Fall anlegen (aus test_deklaration_umgeht_waechter_gepinnt)
            FALL = "deklaration_waechter_gepinnt"
            kegel = T._gesamt_kegel(
                0, kein_vuv=True, kein_kap=False,
                kap_ertraege=500000, kap_gewinn_aktien=300000
            )
            T._gesamt_anlegen(base, FALL, kegel)

            # Messung 1: /ergebnis
            print("\n" + "="*80)
            print("MESSUNG 1: GET /fall/{id}/ergebnis")
            print("="*80)
            st_e, erg = T._req(base, "GET", f"/fall/{FALL}/ergebnis")
            print(f"Status: {st_e}")
            print(f"grund: {erg.get('grund')}")
            print(f"zahl_cent: {erg.get('zahl_cent')}")
            print(f"eingaben_konsistent: {erg.get('eingaben_konsistent')}")

            # Messung 2: POST /einreichen
            print("\n" + "="*80)
            print("MESSUNG 2: POST /fall/{id}/einreichen")
            print("="*80)
            st_r, res = T._req(base, "POST", f"/fall/{FALL}/einreichen", {}, erwarte=409)
            print(f"Status: {st_r}")
            print(f"grund: {res.get('grund')}")
            print(f"eingereicht: {res.get('eingereicht')}")

            # Messung 3: GET /deklaration
            print("\n" + "="*80)
            print("MESSUNG 3: GET /fall/{id}/deklaration")
            print("="*80)
            st_d, dek = T._req(base, "GET", f"/fall/{FALL}/deklaration")
            print(f"Status: {st_d}")
            print(f"eingaben_konsistent: {dek.get('eingaben_konsistent')}")
            print(f"vollstaendig: {dek.get('vollstaendig')}")

            # Die relevanten KAP-Felder aus der Deklaration
            dekl_content = dek.get("deklaration", {})
            e1900701 = dekl_content.get("E1900701")  # Aggregat (kap_kapitalertraege)
            e1900901 = dekl_content.get("E1900901")  # Topf (kap_gewinn_aktien)

            print(f"\nDeklaration-Felder:")
            print(f"  E1900701 (kap_kapitalertraege Aggregat): {e1900701}")
            print(f"  E1900901 (kap_gewinn_aktien Topf): {e1900901}")

            # Vergleich
            print("\n" + "="*80)
            print("VERGLEICH")
            print("="*80)
            print(f"✓ /ergebnis sperrt: grund={erg.get('grund')}, zahl_cent={erg.get('zahl_cent')}")
            print(f"✓ /einreichen sperrt: status={st_r}, grund={res.get('grund')}")
            print(f"✗ /deklaration: status={st_d}, eingaben_konsistent={dek.get('eingaben_konsistent')}")
            print(f"  → zeigt beide KAP-Felder: E1900701={e1900701}, E1900901={e1900901}")

        finally:
            srv.shutdown()
            th.join(timeout=5)
            srv.server_close()

if __name__ == "__main__":
    main()
