#!/usr/bin/env python3
"""Messung: kann Betrag kommen, ohne Screening-Kreuz?

Hypothese: Betragsfeld kann gespeichert werden, ohne dass das zugehörige
Screening-Kreuz je ein Event bekommen hat (kein_vuv z.B. ABWESEND).

Drei Szenarien über HTTP:
1. NUR BETRAG (vv_einnahmen vorlaeufig), kein_vuv ABWESEND
2. KREUZ (False) ERST, DANN BETRAG — Kontrolle 1
3. NUR KREUZ, KEIN BETRAG — Nullzeile

Messwerte: /ergebnis grund+zahl_cent, /einreichen status+grund, /deklaration.
"""
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

def measure(base, fall_id, name):
    """Misst alle drei Endpunkte."""
    print(f"\n{'='*70}")
    print(f"{name}")
    print(f"{'='*70}")

    st_e, erg = T._req(base, "GET", f"/fall/{fall_id}/ergebnis")
    print(f"/ergebnis: grund={erg.get('grund')}, zahl_cent={erg.get('zahl_cent')}")

    try:
        st_r, res = T._req(base, "POST", f"/fall/{fall_id}/einreichen", {}, erwarte=None)
    except AssertionError:
        st_r = None
        res = {}
    print(f"/einreichen: status={st_r}, grund={res.get('grund')}")

    try:
        st_d, dek = T._req(base, "GET", f"/fall/{fall_id}/deklaration")
        print(f"/deklaration: status={st_d}, eingaben_konsistent={dek.get('eingaben_konsistent')}")
    except AssertionError as e:
        print(f"/deklaration: Fehler {str(e)[:50]}")

    return erg

def main():
    with tempfile.TemporaryDirectory() as tmp:
        API.FAELLE = os.path.join(tmp, "faelle")
        os.makedirs(API.FAELLE, exist_ok=True)

        srv = SRV.make_server(0)
        th = threading.Thread(target=srv.serve_forever, daemon=True)
        th.start()
        base = f"http://{srv.server_address[0]}:{srv.server_address[1]}"

        try:
            # Szenario 1: nur Betrag
            print("\nSZENARIO 1: NUR BETRAG, KREUZ ABWESEND")
            st, _ = T._req(base, "POST", "/fall", {
                "scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "sz1"
            })
            assert st == 201
            st, _ = T._req(base, "POST", "/fall/sz1/event", T._laie("vv_einnahmen", 3000000))
            assert st == 201

            store1 = API.lade_fall("sz1")
            has_kreuz = any(e.get("feld_id") == "kein_vuv" for e in store1.get("events", []))
            has_betrag = any(e.get("feld_id") == "vv_einnahmen" for e in store1.get("events", []))
            print(f"Store: kein_vuv present={has_kreuz}, vv_einnahmen present={has_betrag}")

            res1 = measure(base, "sz1", "Messung 1: Nur Betrag")

            # Szenario 2: Kreuz, dann Betrag
            print("\nSZENARIO 2: KREUZ (False) ERST, DANN BETRAG")
            st, _ = T._req(base, "POST", "/fall", {
                "scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "sz2"
            })
            assert st == 201
            st, _ = T._req(base, "POST", "/fall/sz2/event", T._laie("kein_vuv", False))
            assert st == 201
            st, _ = T._req(base, "POST", "/fall/sz2/event", T._laie("vv_einnahmen", 3000000))
            assert st == 201

            store2 = API.lade_fall("sz2")
            has_kreuz = any(e.get("feld_id") == "kein_vuv" for e in store2.get("events", []))
            has_betrag = any(e.get("feld_id") == "vv_einnahmen" for e in store2.get("events", []))
            print(f"Store: kein_vuv present={has_kreuz}, vv_einnahmen present={has_betrag}")

            res2 = measure(base, "sz2", "Messung 2: Kreuz + Betrag")

            # Szenario 3: nur Kreuz
            print("\nSZENARIO 3: NUR KREUZ, KEIN BETRAG")
            st, _ = T._req(base, "POST", "/fall", {
                "scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "sz3"
            })
            assert st == 201
            st, _ = T._req(base, "POST", "/fall/sz3/event", T._laie("kein_vuv", False))
            assert st == 201

            store3 = API.lade_fall("sz3")
            has_kreuz = any(e.get("feld_id") == "kein_vuv" for e in store3.get("events", []))
            has_betrag = any(e.get("feld_id") == "vv_einnahmen" for e in store3.get("events", []))
            print(f"Store: kein_vuv present={has_kreuz}, vv_einnahmen present={has_betrag}")

            res3 = measure(base, "sz3", "Messung 3: Nur Kreuz")

            # Analyse
            print(f"\n{'='*70}")
            print("ANALYSE")
            print(f"{'='*70}")

            g1 = res1.get('grund')
            g2 = res2.get('grund')
            g3 = res3.get('grund')

            print(f"\nGrund (Sperrlogik):")
            print(f"  Sz1 (Betrag, kein Kreuz): {g1}")
            print(f"  Sz2 (Betrag + Kreuz):     {g2}")
            print(f"  Sz3 (nur Kreuz):          {g3}")

            if g1 and not g2:
                print(f"\n✓ BEFUND: Reihenfolge IS entscheidend (Sz1 sperrt, Sz2 nicht)")
            elif g1 == g2:
                print(f"\n✗ BEFUND: Reihenfolge NOT entscheidend (beide {g1})")

            if g3 in (None, "bestaetigt"):
                print(f"✓ Sz3 (Nullzeile) ist grün")
            else:
                print(f"✗ Sz3 (Nullzeile) hat Grund {g3}")

        finally:
            srv.shutdown()
            th.join(timeout=5)
            srv.server_close()

if __name__ == "__main__":
    main()
