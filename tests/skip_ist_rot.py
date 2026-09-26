"""Ein uebersprungener Test ist kein bestandener — fuer `make abgabeweg-freigabe`.

Warum es das gibt (BACKLOG ci-beweist-den-abgabeweg-nicht, Entscheidung
eric-abgabeweg-bleibt-lokaler-manueller-nachweis): der Abgabeweg braucht das
lizenzpflichtige ERiC-SDK und eine Herstellerkennung. In der oeffentlichen CI fehlt
beides, also ueberspringen die ERiC-bewehrten Faelle in
tests/test_einreichen_durchstich.py — und der Lauf meldet trotzdem exit 0. "CI gruen"
und "Abgabeweg geprueft" klingen dann wie dieselbe Aussage und sind es nicht.

Dieses Plugin dreht das fuer EINEN Aufruf um: wird irgendein Test uebersprungen, endet
der Lauf mit exit != 0. Damit ist der lokale Freigabenachweis ein Nachweis und keine
Zeremonie.

Bewusst NICHT in tests/conftest.py: dort gilt es fuer JEDEN Lauf, und ein uebersprungener
Test ist im Rest der Suite normal (fehlende Toolchain, kein ERiC-Schema). Erzwungen wird
das nur dort, wo eine Freigabe daran haengt:

    python3 -m pytest tests/test_einreichen_durchstich.py -q -rs -p skip_ist_rot

Das Plugin sammelt jeden Skip, auch den in setup/teardown — ein Skip ausserhalb des
Testaufrufs verhindert den Nachweis genauso.
"""
from __future__ import annotations

_GESKIPPT: list[tuple[str, str]] = []


def pytest_runtest_logreport(report):
    if not report.skipped:
        return
    grund = ""
    if isinstance(report.longrepr, tuple) and len(report.longrepr) == 3:
        grund = str(report.longrepr[2]).splitlines()[0] if str(report.longrepr[2]) else ""
    _GESKIPPT.append((report.nodeid, grund))


def pytest_sessionfinish(session, exitstatus):
    if not _GESKIPPT:
        return
    print(f"\n{len(_GESKIPPT)} Test(s) uebersprungen — hier ist das KEIN Bestand:")
    for nodeid, grund in _GESKIPPT:
        print(f"  SKIP {nodeid}")
        if grund:
            print(f"       {grund}")
    print("Der Abgabeweg gilt damit als NICHT geprueft (make abgabeweg-freigabe).")
    session.exitstatus = 1
