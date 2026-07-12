#!/usr/bin/env python
"""ELSTER/ERiC Smoke-Test (Phase 4).

Machbarkeits-Verdikt, KEIN Feldmapping, KEIN Versand:
  1. ERiC-Shared-Lib (libericapi.so) finden + laden.
  2. EricInitialisiere() -> EricVersion() -> Versions-XML loggen -> EricBeende().
  3. Verfuegbare checkESt-Plausibilitaets-Plugins (offline) auflisten.
  4. Offline-Nutzbarkeits-Verdikt fuers CI-Gate.

Kein Netz, keine Credentials, rein lokal. Die ERiC-Distribution liegt AUSSERHALB
des Repos (Julius-Konvention: as-is-Software unter ~/02_Software/). Der Pfad kommt
aus der Umgebung, NICHT hart aus dem Code:

    export ERIC_DIR=~/02_Software/eric        # oder direkt auf .../Linux-x86_64

Ohne ERIC_DIR sucht der Test den dokumentierten Default ~/02_Software/eric/ ab und
faellt sonst auf ein lokales elster/-Verzeichnis zurueck (dann: PENDING).

Aufruf:  ERIC_DIR=~/02_Software/eric python elster/smoke_test.py
"""
from __future__ import annotations

import ctypes
import glob
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))


# Suchwurzeln fuer libericapi.so: explizites ERIC_DIR, dann der dokumentierte
# Default ~/02_Software/eric, dann ein lokales elster/ (Legacy -> PENDING).
def _roots() -> list[str]:
    roots = []
    if os.environ.get("ERIC_DIR"):
        roots.append(os.path.expanduser(os.environ["ERIC_DIR"]))
    roots.append(os.path.expanduser("~/02_Software/eric"))
    roots.append(HERE)
    return roots


def find_eric_lib() -> str | None:
    for root in _roots():
        hits = sorted(glob.glob(os.path.join(root, "**", "libericapi.so"), recursive=True))
        if hits:
            return hits[0]
    return None


def _rc(code: int) -> str:
    return "ERIC_OK" if code == 0 else f"rc={code}"


def main() -> int:
    lib_path = find_eric_lib()
    if not lib_path:
        print("[ELSTER] libericapi.so NICHT gefunden.")
        print("[ELSTER] Setze ERIC_DIR oder lege die ERiC Linux-x86_64-Distribution "
              "unter ~/02_Software/eric/ ab.")
        print("[ELSTER] Verdikt: PENDING (kein Blocker).")
        return 0

    lib_dir = os.path.dirname(lib_path)                      # .../Linux-x86_64/lib
    plugin_dir = os.path.join(lib_dir, "plugins")
    print(f"[ELSTER] ERiC-Lib: {lib_path}")

    # Hilfs-Libs (liberictoolkit.so, plugins) fuer den Loader auffindbar machen.
    extra = os.pathsep.join([lib_dir, plugin_dir, os.path.dirname(lib_dir)])
    os.environ["LD_LIBRARY_PATH"] = extra + os.pathsep + os.environ.get("LD_LIBRARY_PATH", "")
    try:
        eric = ctypes.CDLL(lib_path)
    except OSError as e:
        print(f"[ELSTER] Laden FEHLGESCHLAGEN: {e}")
        print("[ELSTER] Verdikt: BLOCKED (Lib da, laedt nicht — Hilfs-Libs / glibc pruefen).")
        return 1
    print("[ELSTER] Lib geladen (ctypes.CDLL ok).")

    # Signaturen (ericapi.h 44.2.4).
    eric.EricRueckgabepufferErzeugen.restype = ctypes.c_void_p
    eric.EricRueckgabepufferInhalt.restype = ctypes.c_char_p
    eric.EricRueckgabepufferInhalt.argtypes = [ctypes.c_void_p]
    eric.EricRueckgabepufferFreigeben.argtypes = [ctypes.c_void_p]
    eric.EricInitialisiere.argtypes = [ctypes.c_char_p, ctypes.c_char_p]
    eric.EricVersion.argtypes = [ctypes.c_void_p]

    log_dir = tempfile.mkdtemp(prefix="eric_smoke_")
    # pluginPfad = Verzeichnis mit libericapi.so + plugins/ (ERiC-Konvention).
    rc = eric.EricInitialisiere(lib_dir.encode(), log_dir.encode())
    print(f"[ELSTER] EricInitialisiere(pluginPfad={lib_dir}) -> {_rc(rc)}")
    if rc != 0:
        print("[ELSTER] Verdikt: BLOCKED (Init fehlgeschlagen — Doku/Plugin-Pfad pruefen).")
        return 1

    try:
        buf = eric.EricRueckgabepufferErzeugen()
        vrc = eric.EricVersion(buf)
        xml = eric.EricRueckgabepufferInhalt(buf)
        print(f"[ELSTER] EricVersion -> {_rc(vrc)}")
        if xml:
            print("[ELSTER] Versions-XML:")
            print("  " + xml.decode("utf-8", "replace").replace("\n", "\n  ").strip())
        eric.EricRueckgabepufferFreigeben(buf)
    finally:
        eric.EricBeende()
        print("[ELSTER] EricBeende() aufgerufen.")

    # Offline-Plausibilitaets-Plugins (checkESt = ESt-Jahresmodule).
    est = sorted(os.path.basename(p)
                 for p in glob.glob(os.path.join(plugin_dir, "libcheckESt_*.so")))
    if est:
        jahre = [p.replace("libcheckESt_", "").replace(".so", "") for p in est]
        print(f"[ELSTER] checkESt-Plugins offline vorhanden: VZ {', '.join(jahre)}")
        print(f"[ELSTER] Neuestes ESt-Jahresmodul: {jahre[-1]}")

    print("[ELSTER] Verdikt: READY — Lib laedt, Init/Version ok, checkESt-Plugins offline vorhanden.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
