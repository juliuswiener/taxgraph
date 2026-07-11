#!/usr/bin/env python
"""ELSTER/ERiC Smoke-Test-Skelett (Phase 4).

Laeuft AUTOMATISCH, sobald Julius die ERiC-Bibliothek (Linux x86_64) unter elster/
abgelegt hat — bis dahin meldet es sauber "wartet auf Download" und tut nichts.

Zweck (Machbarkeits-Verdikt, KEIN Feldmapping):
  1. ERiC-Shared-Lib finden + laden, Version loggen.
  2. checkESt (Validierung OHNE Versand) gegen einen trivialen Datensatz aufrufbar?
  3. Offline-Nutzbarkeits-Verdikt fuer das geplante CI-Gate.

Aufruf:  python elster/smoke_test.py
Kein Netz, keine Credentials, rein lokal. Die eigentlichen ERiC-API-Calls (EricInitialisiere/
EricBearbeiteVorgang/EricBeende) werden mit der Lib + Entwicklerdoku praezisiert (TODO unten) -
ihre exakte Signatur haengt an der ERiC-Version, daher hier nur die Verdrahtung.
"""
from __future__ import annotations

import ctypes
import glob
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))

# ERiC-Lib: Julius legt die entpackte Linux-x86_64-Distribution hier ab. Der
# eigentliche Lib-Name ist typ. libericapi.so; die Hilfs-Libs (liberic*.so) muessen
# im selben Verzeichnis liegen (LD-Suchpfad).
LIB_GLOBS = [
    os.path.join(HERE, "**", "libericapi.so"),
    os.path.join(HERE, "**", "libERiCAPI.so"),
    os.path.join(HERE, "**", "lib*eric*.so"),
]


def find_eric_lib() -> str | None:
    for pat in LIB_GLOBS:
        hits = sorted(glob.glob(pat, recursive=True))
        if hits:
            return hits[0]
    return None


def main() -> int:
    lib_path = find_eric_lib()
    if not lib_path:
        print("[ELSTER] ERiC-Bibliothek NICHT gefunden — wartet auf Julius-Download.")
        print(f"[ELSTER] Erwarteter Ablageort: {HERE}/  (entpackte ERiC Linux-x86_64-Distribution)")
        print("[ELSTER] Benoetigt: ERiC-Lib (libericapi.so + Hilfs-libs), Entwicklerdoku, checkESt-Material.")
        print("[ELSTER] Verdikt: PENDING (kein Blocker — Smoke-Test laeuft automatisch nach dem Download).")
        return 0

    print(f"[ELSTER] ERiC-Lib gefunden: {lib_path}")
    lib_dir = os.path.dirname(lib_path)
    # Hilfs-Libs im selben Verzeichnis auffindbar machen.
    os.environ["LD_LIBRARY_PATH"] = lib_dir + os.pathsep + os.environ.get("LD_LIBRARY_PATH", "")
    try:
        eric = ctypes.CDLL(lib_path)
    except OSError as e:
        print(f"[ELSTER] Laden FEHLGESCHLAGEN: {e}")
        print("[ELSTER] Verdikt: BLOCKED (Lib da, laedt aber nicht — fehlende Hilfs-Libs / glibc-Version pruefen).")
        return 1

    print("[ELSTER] Lib geladen (ctypes.CDLL ok).")
    # Version: ERiC exportiert die Version i.d.R. ueber EricVersion(...) mit Puffer;
    # die exakte Signatur kommt aus der Doku. Wir tasten defensiv ab.
    exported = [n for n in ("EricVersion", "EricInitialisiere", "EricBearbeiteVorgang",
                            "EricBeende", "EricRueckgabepufferErzeugen") if hasattr(eric, n)]
    print(f"[ELSTER] Erwartete Symbole vorhanden: {exported}")

    # TODO (mit Lib + Doku praezisieren, wenn Julius heruntergeladen hat):
    #   1. EricRueckgabepufferErzeugen() -> Puffer-Handles.
    #   2. EricInitialisiere(pluginPfad, logPfad) -> 0 bei Erfolg.
    #   3. Version via EricVersion(puffer) auslesen + loggen.
    #   4. EricBearbeiteVorgang(xml, datenartVersion="ESt_YYYY", bearbeitungsFlags=CHECK-only,
    #      ...) gegen einen Trivial-ESt-XML-Datensatz -> Rueckgabecode == 0 (validierbar offline?).
    #   5. EricBeende().
    ready = "EricInitialisiere" in exported and "EricBearbeiteVorgang" in exported
    print(f"[ELSTER] Verdikt: {'READY (Init+BearbeiteVorgang exportiert — checkESt-Verdrahtung fuellen)' if ready else 'PARTIAL (Symbole unvollstaendig — Doku pruefen)'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
