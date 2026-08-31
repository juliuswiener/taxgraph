#!/usr/bin/env python3
"""Repro-Skript: HTTP-500 bei zusammenveranlagten Rentner-Paaren.

Hypothese "Weg 2" (Instructor-Auftrag 2026-08-31): rentner_renten_beginn_jahr_partner wird nie
beantwortet, obwohl rentner_renten_art_partner (+ evtl. weitere Kernfelder) gesetzt sind. Eine
Lesehilfe wandelt das fehlende Feld in `0` um (Verwechslung mit einer echten Antwort "0"), 0 < VZ
gilt als aa-Folgejahr, der Ring verlangt dafuer einen fixierten Rentenfreibetrag und crasht.

Lauf: python3 reports/repro/repro_rentner_partner_beginn_jahr_500.py

Faelle liegen als flache JSON-Dateien in API.FAELLE (echtes Verzeichnis, KEIN tmp_path wie in
den Tests) -- Skript raeumt seine drei Fall-IDs am Ende selbst weg.

Hinweis 2026-08-31: das Skript patcht flag_check.flag_widersprueche zur Laufzeit auf eine reine
HEAD-Kopie der Pruefregel (s. _HEAD_FLAG_WIDERSPRUECHE unten) und nimmt den Patch am Ende zurueck.
Grund: ein paralleler, ungecommitteter Umbau in produkt/konsistenz/flag_check.py (fremdes Terrain,
hier nicht angefasst) behandelt ein NIE gestelltes Screening-Flag (kein_sonstige_partner) testweise
wie bestaetigt-true; kein_sonstige_partner ist auf Scheibe rentner_gesamt aber gar nicht erreichbar
(400 "nicht in dieser Scheibe"), laesst sich also nicht durch eine explizite Antwort neutralisieren.
Ohne den Patch feuert flag_konsistenz_offen und verdeckt den eigentlichen Rentner-Partner-Befund
dieses Skripts. Gemessen per git diff, s. Meldung an den Instructor.
"""
import os
import subprocess
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
for sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/konsistenz"):
    sys.path.insert(0, os.path.join(ROOT, sub))

os.environ["TAXGRAPH_NO_AUTH"] = "1"   # wie tests/conftest.py -- sonst 401 auf /fall

import api as API  # noqa: E402
import flag_check as FC  # noqa: E402


def _head_flag_widersprueche_installieren():
    """flag_check.flag_widersprueche() zur Laufzeit durch die HEAD-Fassung ersetzen (git show
    HEAD:...), damit ein paralleler ungecommitteter Umbau derselben Datei dieses Skript nicht
    verfaelscht. Gibt eine restore()-Funktion zurueck."""
    quelle = subprocess.run(
        ["git", "show", "HEAD:produkt/konsistenz/flag_check.py"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    ).stdout
    ns = {}
    exec(compile(quelle, "flag_check@HEAD", "exec"), ns)  # noqa: S102 -- vertrauenswuerdige eigene Repo-Quelle
    orig = FC.flag_widersprueche
    FC.flag_widersprueche = ns["flag_widersprueche"]

    def _restore():
        FC.flag_widersprueche = orig
    return _restore


def _laie(fld, w):
    return {"feld_id": fld, "wert": w, "zustand": "bestaetigt",
            "herkunft": {"herkunft": "laie", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
            "schreiber": "ui:laie", "signal": {"signal_1": None, "signal_2": f"ok@{fld}"}}


_BASIS_ZUSAMMEN = [
    ("rentner_renten_art", "gesetzliche_rente"), ("rentner_jahresrente", 2000000),
    ("rentner_renten_beginn_jahr", 2025), ("rentner_alter_bei_rentenbeginn", 0),
    ("rentner_grad_der_behinderung", 0), ("rentner_hilflos_blind_taubblind", False),
    ("rentner_pflegegrad", 0), ("rentner_gepflegter_hilflos", False),
    ("rentner_hinterbliebenenbezuege", False), ("veranlagung", "zusammen"),
    ("kein_gewinn", True), ("kein_kap", True), ("kein_vuv", True), ("kein_sonstige", False),
    ("vor_an_anteil_rv", 0), ("vor_ag_anteil_rv", 0), ("vor_rv_ausserhalb_lstb", 0),
    ("versicherungsart", "gesetzlich_an"), ("basis_kv", 0), ("basis_pv", 0),
    ("vorsorge_arbeitslosenversicherung", 0), ("vorsorge_erwerbsunfaehigkeit", 0),
    ("vorsorge_unfall_haftpflicht", 0), ("vorsorge_rv_alt_mit_ueberschuss", 0),
    ("vorsorge_rv_alt_ohne_ueberschuss", 0), ("mit_anspruch_auf_zuschuss", False),
]


def _anlegen_und_fuellen(fall_id, zusatz_felder):
    st, resp = API.fall_anlegen({"fall_id": fall_id, "scheibe": "rentner_gesamt",
                                  "veranlagungszeitraum": 2025})
    assert st == 201, f"fall_anlegen: {st} {resp}"
    for fid, wert in _BASIS_ZUSAMMEN + zusatz_felder:
        st, resp = API.event(fall_id, _laie(fid, wert))
        assert st == 201, f"event {fid}: {st} {resp}"
    return API.ergebnis(fall_id)


def _aufraeumen(fall_id):
    pfad = os.path.join(API.FAELLE, f"{fall_id}.json")
    if os.path.isfile(pfad):
        os.remove(pfad)


def main():
    ausfaelle = []
    _restore_flag_check = _head_flag_widersprueche_installieren()

    # ---- Fall A (Weg 2): renten_art_partner gesetzt, beginn_jahr_partner NIE beantwortet ----
    fall_a = "repro_500_beginn_jahr_partner_fehlt"
    _aufraeumen(fall_a)
    print(f"=== Fall A: {fall_a} — rentner_renten_art_partner allein, KEIN beginn_jahr_partner ===")
    try:
        st, erg = _anlegen_und_fuellen(fall_a, [
            ("rentner_renten_art_partner", "gesetzliche_rente"),
            ("rentner_jahresrente_partner", 1500000),
            ("rentner_alter_bei_rentenbeginn_partner", 65),
            # rentner_renten_beginn_jahr_partner ABSICHTLICH NICHT gesetzt
        ])
        print(f"HTTP-Status: {st}")
        print(f"Ergebnis: {erg}")
        if st == 500:
            print("REPRODUZIERT: HTTP 500.")
            ausfaelle.append("A")
        elif st == 200 and erg.get("grund") not in ("bestaetigt",):
            print(f"KEIN Crash — Sperrgrund {erg.get('grund')!r} statt 500 (Guard aktiv?).")
        else:
            print("KEIN Crash und KEIN Sperrgrund — unerwartetes Ergebnis, siehe oben.")
    except Exception:
        print("Python-Exception statt HTTP-Response:")
        traceback.print_exc()
        ausfaelle.append("A (Exception statt HTTP-500-Response)")
    finally:
        _aufraeumen(fall_a)

    print()

    # ---- Fall B (Kontrolle): derselbe Fall, aber beginn_jahr_partner beantwortet ----
    fall_b = "repro_500_beginn_jahr_partner_kontrolle"
    _aufraeumen(fall_b)
    print(f"=== Fall B (Kontrolle): {fall_b} — dieselben Felder, beginn_jahr_partner=2025 ===")
    try:
        st, erg = _anlegen_und_fuellen(fall_b, [
            ("rentner_renten_art_partner", "gesetzliche_rente"),
            ("rentner_jahresrente_partner", 1500000),
            ("rentner_renten_beginn_jahr_partner", 2025),
            ("rentner_alter_bei_rentenbeginn_partner", 65),
        ])
        print(f"HTTP-Status: {st}")
        print(f"Ergebnis: {erg}")
        if st == 200 and erg.get("grund") == "bestaetigt" and isinstance(erg.get("zahl_cent"), int):
            print("Kontrolle OK: sauberer Durchlauf, grund=bestaetigt.")
        else:
            print("KONTROLLE SCHLAEGT AUS — eigener Befund, kein Fixture-Fehler.")
            ausfaelle.append("B (Kontrollzeile weicht ab)")
    except Exception:
        print("Python-Exception in der Kontrollzeile:")
        traceback.print_exc()
        ausfaelle.append("B (Exception in Kontrollzeile)")
    finally:
        _aufraeumen(fall_b)

    print()

    # ---- Rot-Gruen-Beweis: Guard in bescheid_deklaration.py (_an_gesamt_sperrgrund) in-process
    # neutralisieren (KEIN Datei-Edit -- die Datei ist fremdes Terrain), um zu zeigen, dass die
    # Kegel-Luecke (Weg 1, RENTNER_22_PARTNER fehlt in RENTNER_KEGEL) den Crash-Mechanismus darunter
    # unveraendert laesst. Patch wird danach zurueckgenommen (finally).
    print("=== Rot-Gruen-Beweis: Guard temporaer neutralisiert (in-process, kein Datei-Edit) ===")
    orig = API._an_gesamt_sperrgrund

    def _guard_ohne_rente_b_kern(felder, cfg, vz, store, bindung):
        g = orig(felder, cfg, vz, store, bindung)
        return None if g in ("rente_instanz_offen", "partner_kegel_offen") else g

    fall_c = "repro_500_ohne_guard"
    _aufraeumen(fall_c)
    API._an_gesamt_sperrgrund = _guard_ohne_rente_b_kern
    try:
        st, erg = _anlegen_und_fuellen(fall_c, [
            ("rentner_renten_art_partner", "gesetzliche_rente"),
            ("rentner_jahresrente_partner", 1500000),
            ("rentner_alter_bei_rentenbeginn_partner", 65),
        ])
        print(f"HTTP-Status (Guard aus): {st}")
        print(f"Ergebnis (Guard aus): {erg}")
        if st == 500:
            print("BESTAETIGT: ohne den Guard crasht derselbe Fall mit HTTP 500 -- "
                  "der Crash-Mechanismus existiert, der Guard ist die einzige Deckung.")
        else:
            print("Ohne Guard KEIN 500 -- Crash-Mechanismus nicht bestaetigt (Hypothese widerlegt "
                  "oder eine weitere Deckung greift).")
    except Exception:
        print("Python-Exception (Guard aus) statt HTTP-Response:")
        traceback.print_exc()
        print("BESTAETIGT (als Exception statt sauberem 500): Crash-Mechanismus existiert.")
    finally:
        API._an_gesamt_sperrgrund = orig
        _aufraeumen(fall_c)
        assert API._an_gesamt_sperrgrund is orig, "Guard-Patch nicht zurueckgenommen!"

    _restore_flag_check()

    print()
    if ausfaelle:
        print(f"BEFUND: Auffaellig: {ausfaelle}")
        sys.exit(1)
    else:
        print("Kein Ausfall gemessen — beide reguraeren Faelle liefen wie erwartet "
              "(Crash nur ohne Guard, s. Rot-Gruen-Beweis oben).")
        sys.exit(0)


if __name__ == "__main__":
    main()
