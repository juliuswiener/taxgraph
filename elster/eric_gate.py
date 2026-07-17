#!/usr/bin/env python
"""ERiC-Offline-CI-Gate (`make eric-gate`) — VZ 2025, deterministisch, credential-frei.

Zweistufig, beide Stufen rein lokal (kein Netz, KEIN Versand, KEINE Credentials aus
Dateien gelesen):

  Stufe A  STRUKTUR (credential-frei, gate-tragend): das minimale ESt-2025-XML wird
           gegen das amtliche ELSTER-Schema `elster11_E10_2025_extern.xsd` aus der
           ERiC-Auslieferung validiert (xmllint). Ein absichtlich schema-fremd
           mutiertes XML MUSS fallen (Tamper-Selbstcheck -> das Gate ist RED-faehig,
           kein Vakuum-Gruen).

  Stufe B  checkESt (`EricBearbeiteVorgang`, Flag ERIC_VALIDIERE, OHNE ERIC_SENDE):
           die amtliche Plausibilitaetspruefung laeuft lokal im Plugin-`.so`. Der
           Returncode wird klassifiziert:
             rc == 0                         -> PLAUSIBEL (nur mit registrierter
                                                Hersteller-ID erreichbar) -> gruen
             rc == 610301202 (GESPERRT)      -> Hersteller-ID-Grenze: ELSTER verlangt
                                                seit 39.4.x die eigene registrierte
                                                Hersteller-ID auch fuer die reine
                                                Validierung; keine Dummy-Test-ID mehr.
                                                Das ist Julius-Territorium (wie das
                                                Versand-Zertifikat), vordokumentiert im
                                                Smoke-Befund 2026-07-12 -> ERWARTETE
                                                Grenze im credential-freien CI, NICHT
                                                als Gate-Fehler gewertet.
             sonst                           -> UNERWARTET -> RED (echtes Regressions-
                                                signal: XML bricht vor dem ID-Gate,
                                                Datenart/Plugin-Problem).

Gate-Exit: 0 gdw. Stufe A besteht (valide -> PASS, mutiert -> FAIL) UND Stufe B in
{PLAUSIBEL, GESPERRT-Grenze}. Sonst != 0.

Hersteller-ID: NUR aus der Umgebungsvariable $ELSTER_HERSTELLER_ID, falls exportiert;
diese Datei liest NIEMALS .env.elster o. ae. Ohne die Variable laeuft das Gate
vollstaendig durch (Stufe A + Stufe-B-Grenze) — genau der Login-freie CI-Fall.

ERiC-Pfad aus $ERIC_DIR (Default ~/02_Software/eric), nie hart im Code.
"""
from __future__ import annotations

import os
import re
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(HERE, "submission"))

import checkest_gate as CE          # noqa: E402  (EricBearbeiteVorgang/ERIC_VALIDIERE)
import validate_xsd as XSD          # noqa: E402  (offline E10_2025-Schemapruefung)

VZ = "2025"
DATENART = "ESt_2025"
MINIMAL_XML = os.path.join(HERE, "submission", "testfall_est2025_minimal.xml")
HID_GESPERRT = 610301202            # ERIC_IO_TESTHERSTELLERID_GESPERRT


def _stufe_a() -> bool:
    """Struktur-Gate: valide -> PASS, schema-fremd mutiert -> FAIL. RED-faehig."""
    ok_valid, msg = XSD.validate(MINIMAL_XML, VZ)
    print(f"[eric-gate] Stufe A  valide {os.path.basename(MINIMAL_XML)} -> "
          f"{'PASS' if ok_valid else 'FAIL'}: {msg[:70]}")
    with open(MINIMAL_XML, encoding="utf-8") as f:
        src = f.read()
    mutiert = src.replace("<E0100201>", "<E9999999>").replace("</E0100201>", "</E9999999>")
    tmp = tempfile.NamedTemporaryFile("w", suffix=".xml", delete=False, encoding="utf-8")
    tmp.write(mutiert)
    tmp.close()
    try:
        ok_bad, _ = XSD.validate(tmp.name, VZ)
    finally:
        os.unlink(tmp.name)
    print(f"[eric-gate] Stufe A  Tamper (schema-fremdes Element) -> "
          f"{'FAIL (erwartet)' if not ok_bad else 'PASS (FEHLER: Gate wirkungslos!)'}")
    return ok_valid and not ok_bad


def _stufe_b() -> tuple[bool, str]:
    """checkESt (ERIC_VALIDIERE, offline). Rueckgabe (gate_ok, verdikt). rc explizit klassifiziert
    (Falsch-Gruen-Sperre): RC_IO_KEIN_TICKET (610301200) liefert 0 Fehler, ist aber NICHT geprueft."""
    with open(MINIMAL_XML, "rb") as f:
        xml, hid = CE._mit_hersteller_id(f.read())
    rc, antwort = CE.validate(xml, DATENART)
    klasse = CE.klassifiziere_rc(rc)
    print(f"[eric-gate] Stufe B  Hersteller-ID: "
          f"{hid if hid else 'NICHT gesetzt ($ELSTER_HERSTELLER_ID leer)'}")
    print(f"[eric-gate] Stufe B  EricBearbeiteVorgang({DATENART}, ERIC_VALIDIERE) -> rc={rc} "
          f"[{klasse}]")
    if rc == CE.RC_OK:
        return True, "PLAUSIBEL (rc==0, Hersteller-ID gesetzt)"
    if rc == HID_GESPERRT:
        return True, ("GESPERRT-Grenze (rc=610301202): Hersteller-ID Pflicht auch fuer "
                      "Validierung — Julius-Territorium, erwartete credential-freie Grenze")
    if rc == CE.RC_IO_KEIN_TICKET:
        return False, ("I/O-Gate (rc=610301200): short-circuit VOR der Plausibilitaet, 0 Fehler "
                       "≠ fehlerfrei -> RED (NICHT als gruen werten)")
    kurz = (antwort[:300].replace("\n", " ") if antwort else "(keine Ericantwort)")
    return False, f"UNERWARTETER rc={rc} [{klasse}] -> RED. Ericantwort: {kurz}"


def _stufe_c() -> tuple[bool, str]:
    """Trunkierungs-Haertung (Falsch-Gruen-Sperre gegen Fehler-Kappung). Ein Fixture mit >20
    Fehlern MUSS mit angehobenem Cap >20 Fehler liefern; mit Default-Cap 20 genau 20 (Tamper).
    RED-faehig: entfernt jemand die Cap-Anhebung in checkest_gate, faellt diese Stufe."""
    with open(MINIMAL_XML, "rb") as f:
        base, hid = CE._mit_hersteller_id(f.read())
    if not hid:
        # Ohne registrierte Hersteller-ID kurzschliesst jede Validierung am GESPERRT-Gate (0 Fehler)
        # -> die Trunkierung ist nicht pruefbar. Wie Stufe-B-Grenze: uebersprungen, nicht RED.
        print("[eric-gate] Stufe C  Trunkierung: uebersprungen ($ELSTER_HERSTELLER_ID leer — "
              "Plausibilitaetspfad nicht erreichbar).")
        return True, "uebersprungen (credential-frei, wie Stufe-B-Grenze)"
    corrupt = re.sub(rb"<(E\d{7})>([^<]*)</\1>", rb"<\1>ZZ99XX</\1>", base)
    eric = CE._load_and_init()          # Produktionspfad: hebt Cap auf CE.VALIDIERE_MELDUNGEN_MAX

    def _cap(w):
        for nm in (b"validieren.fehler_max", b"validieren.hinweise_max"):
            eric.EricEinstellungSetzen(nm, str(w).encode())

    # (1) PRODUKTIONSPFAD (ohne manuelles Setzen) MUSS >20 liefern -> beweist die Auto-Anhebung in
    #     _load_and_init. Wird sie entfernt, faellt DIESE Stufe (echter Regressions-Guard).
    _rc, ant_p = CE.validate(corrupt, DATENART)
    n_prod = ant_p.count("<FehlerRegelpruefung>")
    # (2) TAMPER: Cap zurueck auf 20 -> genau 20 (beweist, dass das Fixture >20 Fehler HAT und der
    #     Default kappt). Danach wieder anheben, damit der Prozess-Zustand sauber bleibt.
    _cap(20)
    _rc, ant_d = CE.validate(corrupt, DATENART)
    n_default = ant_d.count("<FehlerRegelpruefung>")
    _cap(CE.VALIDIERE_MELDUNGEN_MAX)
    ok = n_prod > 20 and n_default == 20
    print(f"[eric-gate] Stufe C  Trunkierung: Produktionspfad -> {n_prod} Fehler (soll >20), "
          f"Tamper Cap=20 -> {n_default} Fehler (soll 20)")
    return ok, (f"Auto-Anhebung wirksam ({n_prod}>20), Default kappt ({n_default}=20)" if ok
                else f"FAIL: Produktion={n_prod} (soll >20), Tamper-Default={n_default} (soll 20)")


def run_gate() -> int:
    print(f"[eric-gate] ERiC-Offline-CI-Gate VZ {VZ} — ERIC_VALIDIERE, kein Versand, "
          f"keine Datei-Credentials.")
    a_ok = _stufe_a()
    b_ok, verdikt_b = _stufe_b()
    print(f"[eric-gate] Stufe B  Verdikt: {verdikt_b}")
    c_ok, verdikt_c = _stufe_c()
    print(f"[eric-gate] Stufe C  Verdikt: {verdikt_c}")
    CE.beende()
    gate_ok = a_ok and b_ok and c_ok
    print(f"[eric-gate] GATE: {'GRUEN' if gate_ok else 'ROT'} "
          f"(Stufe A {'ok' if a_ok else 'FAIL'}, Stufe B {'ok' if b_ok else 'FAIL'}, "
          f"Stufe C {'ok' if c_ok else 'FAIL'})")
    return 0 if gate_ok else 1


if __name__ == "__main__":
    sys.exit(run_gate())
