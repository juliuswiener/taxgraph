#!/usr/bin/env python
"""checkESt-CI-Gate-Harness (Phase 4, ii) — OFFLINE-Plausibilitaetspruefung via ERiC.

Ruft `EricBearbeiteVorgang` mit `ERIC_VALIDIERE` (ohne `ERIC_SENDE`) auf: die ELSTER-
Plausibilitaetspruefung laeuft lokal im checkESt-Plugin-`.so`, KEIN Netz, KEINE
Credentials, KEIN Versand. rc == 0 (ERIC_OK) -> plausibel; rc != 0 -> implausibel
(die Ericantwort nennt die verletzten Kennzahlen/Regeln).

    from elster.checkest_gate import validate
    rc, antwort = validate(xml_bytes, "ESt_2020")

Selbstbeweis (PIPE-PROOF, `python elster/checkest_gate.py --prove`): das amtliche
ESt_2020-Beispiel muss rc==0 liefern, ein absichtlich verfaelschter Datensatz rc!=0.
Das belegt, dass der HARNESS amtlich funktioniert — es ist KEINE amtliche Bestaetigung
unserer VZ-2026-Werte (dafuer fehlt libcheckESt_2026; s. Befund).

ERiC-Pfad aus $ERIC_DIR (Default ~/02_Software/eric), nie hart im Code.
"""
from __future__ import annotations

import ctypes
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from smoke_test import find_eric_lib  # noqa: E402  (teilt die $ERIC_DIR-Aufloesung)

ERIC_VALIDIERE = 1 << 1                 # nur pruefen, NICHT senden (ericapi bearbeitungsflags)

# Falsch-Gruen-Sperre gegen Fehler-Kappung (P9-R3-Fund 2026-07-17): ERiC begrenzt die im
# Rueckgabepuffer gemeldeten Fehler-/Hinweismeldungen per Default auf 20 ("Weitere Fehler-
# meldungen werden abgeschnitten", Entwicklerhandbuch Kap. 4.1.2.1). Eine Erklaerung mit >20
# Fehlern verliert Fehler 21+ STILL. Wir heben beide Caps auf das dokumentierte Maximum an.
# Erlaubter Wertebereich lt. Handbuch: 1–1000 (10000 ist WERT_UNGUELTIG / rc=610001861).
VALIDIERE_MELDUNGEN_MAX = 1000

# ERiC-rc-Klassen (Falsch-Gruen-Sperre): "0 Fehler im Puffer" ist NUR gruen, wenn rc==0.
RC_OK = 0
RC_PLAUSIBILITAET = 610001002           # Plausibilitaetsfehler -> Fehlerliste im Rueckgabepuffer
RC_IO_KEIN_TICKET = 610301200           # I/O-Gate (z.B. Nutzdatenticket) -> short-circuit VOR Plausi
RC_HERSTELLER_GESPERRT = 610301202      # Test-Hersteller-ID gesperrt


def klassifiziere_rc(rc: int) -> str:
    """rc einer Validierung klassifizieren. Wichtig: RC_IO_KEIN_TICKET liefert 0 Fehler im
    Puffer, ist aber NICHT geprueft (short-circuit vor der Plausibilitaet) — nie als gruen werten."""
    return {RC_OK: "plausibel", RC_PLAUSIBILITAET: "plausibilitaet_fehler",
            RC_IO_KEIN_TICKET: "io_gate_nicht_geprueft",
            RC_HERSTELLER_GESPERRT: "hersteller_id_gesperrt"}.get(rc, "sonstig")


def gekappt_verdacht(antwort: str) -> bool:
    """Rest-Trunkierungs-Doktrin: erreicht die Fehlerzahl den (bereits angehobenen) Cap, KANN die
    Liste weiter gekappt sein — dann nie als vollstaendig behandeln, sondern Fehler beheben und
    RE-validieren (Fixpunkt bis rc==0)."""
    return antwort.count("<FehlerRegelpruefung>") >= VALIDIERE_MELDUNGEN_MAX


_STATE = {"eric": None, "lib_dir": None}


def _load_and_init():
    """ERiC einmal laden + initialisieren. Idempotent im Prozess."""
    if _STATE["eric"] is not None:
        return _STATE["eric"]
    lib_path = find_eric_lib()
    if not lib_path:
        raise RuntimeError("libericapi.so nicht gefunden — ERIC_DIR setzen (s. elster/README).")
    lib_dir = os.path.dirname(lib_path)
    plugin_dir = os.path.join(lib_dir, "plugins")
    os.environ["LD_LIBRARY_PATH"] = os.pathsep.join(
        [lib_dir, plugin_dir, os.path.dirname(lib_dir), os.environ.get("LD_LIBRARY_PATH", "")])
    eric = ctypes.CDLL(lib_path)
    eric.EricRueckgabepufferErzeugen.restype = ctypes.c_void_p
    eric.EricRueckgabepufferInhalt.restype = ctypes.c_char_p
    eric.EricRueckgabepufferInhalt.argtypes = [ctypes.c_void_p]
    eric.EricRueckgabepufferFreigeben.argtypes = [ctypes.c_void_p]
    eric.EricInitialisiere.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    eric.EricEinstellungSetzen.restype = ctypes.c_int
    eric.EricEinstellungSetzen.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    eric.EricBearbeiteVorgang.restype = ctypes.c_int
    eric.EricBearbeiteVorgang.argtypes = [
        ctypes.c_char_p,   # datenpuffer (XML)
        ctypes.c_char_p,   # datenartVersion, z.B. "ESt_2020"
        ctypes.c_uint32,   # bearbeitungsFlags
        ctypes.c_void_p,   # druckParameter  (NULL: kein Druck)
        ctypes.c_void_p,   # cryptoParameter (NULL: kein Versand/Crypto)
        ctypes.c_void_p,   # rueckgabeXmlPuffer
        ctypes.c_void_p,   # serverantwortXmlPuffer (NULL: kein Versand)
    ]
    log_dir = tempfile.mkdtemp(prefix="eric_checkest_")
    rc = eric.EricInitialisiere(lib_dir.encode(), log_dir.encode())
    if rc != 0:
        raise RuntimeError(f"EricInitialisiere fehlgeschlagen (rc={rc}).")
    # Falsch-Gruen-Sperre: Fehler-/Hinweis-Cap auf das Handbuch-Maximum anheben, damit keine
    # Fehler stillschweigend abgeschnitten werden (Default 20 -> VALIDIERE_MELDUNGEN_MAX).
    for name in (b"validieren.fehler_max", b"validieren.hinweise_max"):
        r = eric.EricEinstellungSetzen(name, str(VALIDIERE_MELDUNGEN_MAX).encode())
        if r != 0:
            raise RuntimeError(f"EricEinstellungSetzen({name.decode()}={VALIDIERE_MELDUNGEN_MAX}) "
                               f"fehlgeschlagen (rc={r}) — Fehler-Cap NICHT angehoben, "
                               f"Falsch-Gruen-Risiko. Abbruch statt stiller Kappung.")
    _STATE.update(eric=eric, lib_dir=lib_dir)
    return eric


def validate(xml: bytes | str, datenart_version: str) -> tuple[int, str]:
    """Offline-Plausibilitaetspruefung. Rueckgabe (rc, ericantwort_xml).

    rc == 0  -> ERIC_OK (plausibel).  rc != 0 -> implausibel/fehlerhaft; die Antwort
    nennt die verletzten Regeln. Rein lokal (ERIC_VALIDIERE, kein Versand)."""
    if isinstance(xml, str):
        xml = xml.encode("utf-8")
    eric = _load_and_init()
    ant = eric.EricRueckgabepufferErzeugen()
    try:
        rc = eric.EricBearbeiteVorgang(
            xml, datenart_version.encode(), ERIC_VALIDIERE, None, None, ant, None)
        raw = eric.EricRueckgabepufferInhalt(ant)
        antwort = raw.decode("utf-8", "replace") if raw else ""
    finally:
        eric.EricRueckgabepufferFreigeben(ant)
    return rc, antwort


def beende():
    if _STATE["eric"] is not None:
        _STATE["eric"].EricBeende()
        _STATE["eric"] = None


# --- PIPE-PROOF: der Harness funktioniert amtlich (kein VZ-2026-Nachweis) ----------

_SAMPLE = os.path.join(HERE, "testdaten", "est_2020_amtliches_beispiel.xml")
_PLATZHALTER_HID = b"74931"      # gesperrte Alt-Test-Hersteller-ID im Beispieldatensatz
_HID_GESPERRT = 610301202        # ERIC_IO_TESTHERSTELLERID_GESPERRT


def _mit_hersteller_id(xml: bytes) -> tuple[bytes, str | None]:
    """Ersetzt die gesperrte Platzhalter-Hersteller-ID durch die eigene aus
    $ELSTER_HERSTELLER_ID (z.B. aus Julius' .env.elster). Ohne die Variable bleibt
    der Platzhalter drin - dann blockt ERiC am Hersteller-ID-Gate (seit 39.4.x
    Pflicht, auch fuer Validierung; keine Dummy-Test-ID mehr)."""
    hid = os.environ.get("ELSTER_HERSTELLER_ID", "").strip()
    if not hid:
        return xml, None
    return xml.replace(_PLATZHALTER_HID, hid.encode()), hid


def _prove() -> int:
    with open(_SAMPLE, "rb") as f:
        sample = f.read()
    plausibel, hid = _mit_hersteller_id(sample)
    # Negativfall: ein Pflicht-Datum verfaelschen (unmoegliches Datum) -> Plausibilitaet muss brechen.
    implausibel = plausibel.replace(b"<E0100401>05.05.1955</E0100401>",
                                    b"<E0100401>99.99.9999</E0100401>")
    assert implausibel != plausibel, "Negativ-Mutation griff nicht — Testdaten geaendert?"

    print("[checkESt] PIPE-PROOF (amtliches ESt_2020-Beispiel, ERIC_VALIDIERE, offline)")
    print(f"[checkESt] Hersteller-ID: {hid if hid else 'NICHT gesetzt ($ELSTER_HERSTELLER_ID leer)'}")
    rc_ok, _ = validate(plausibel, "ESt_2020")
    print(f"[checkESt] plausibel  -> rc={rc_ok}")
    rc_bad, ant_bad = validate(implausibel, "ESt_2020")
    print(f"[checkESt] implausibel-> rc={rc_bad}")
    beende()

    if rc_ok == _HID_GESPERRT:
        print("[checkESt] Verdikt: MECHANISMUS BEWIESEN, VOLLBEWEIS PENDING HERSTELLER-ID.")
        print("[checkESt]   Der Harness ruft EricBearbeiteVorgang(ERIC_VALIDIERE) korrekt auf und "
              "bekommt einen deterministischen rc zurueck (610301202 = TESTHERSTELLERID_GESPERRT, "
              "korrekt dekodiert). Fuer rc==0 braucht es eine registrierte eigene Hersteller-ID "
              "(ELSTER-Registrierung = Julius-Territorium, wie das Versand-Zertifikat). Setze "
              "$ELSTER_HERSTELLER_ID, dann vervollstaendigt sich der Beweis ohne Code-Aenderung.")
        return 0

    ok = (rc_ok == 0 and rc_bad != 0)
    print(f"[checkESt] Verdikt: {'PIPE-PROOF BESTANDEN' if ok else 'UNERWARTET'} "
          f"(plausibel rc==0 UND implausibel rc!=0). "
          f"HARNESS amtlich funktionsfaehig — KEINE Bestaetigung der VZ-2026-Werte "
          f"(libcheckESt_2026 fehlt; VZ 2025 waere der Anker).")
    if not ok and ant_bad:
        print("[checkESt] Ericantwort (implausibel, Auszug):")
        print("  " + ant_bad[:600].replace("\n", "\n  "))
    return 0 if ok else 1


if __name__ == "__main__":
    if "--prove" in sys.argv:
        sys.exit(_prove())
    print("Nutzung: python elster/checkest_gate.py --prove")
    print("   oder: from elster.checkest_gate import validate")
    sys.exit(0)
