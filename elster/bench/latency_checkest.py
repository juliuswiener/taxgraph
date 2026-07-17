#!/usr/bin/env python3
"""ERiC checkESt Latenz-Messung (AUFTRAG 1) — entscheidet UI synchron vs. asynchron.

Der einzige offene Widerspruch aus dem Lab-Board ist: darf die Sachverhalts-Eingabe die
ELSTER-Plausibilitaet (drittes Orakel) SYNCHRON im Request-Pfad aufrufen, oder muss sie
ASYNCHRON in einen Worker (Daemon) ausgelagert werden? Das haengt allein an der Latenz von
EricBearbeiteVorgang(ERIC_VALIDIERE). Diese wird hier gemessen — nicht geschaetzt.

Drei Latenzgroessen, je Fall, ~N Wiederholungen, Median + p95:

  cold_wall        voller Fresh-Prozess: python-Interp-Start + import + EricInitialisiere +
                   Plugin-Load (libcheckESt) + erste Validierung. = reale Kosten pro Request,
                   falls die UI pro Aufruf einen frischen Prozess forkt (naivstes Modell).
  cold_eric        NUR der ERiC-Anteil im Fresh-Prozess (EricInitialisiere + Plugin-Load +
                   erste Validierung), ohne python-Interp-Start. Isoliert die ERiC-Latenz von
                   der python-Startzeit.
  warm             Steady-State-Validierung in warmem Prozess (Init + Plugin bereits geladen).
                   = Kosten pro Request im Daemon-Modell (ein langlebiger ERiC-Prozess).

  Zerlegung cold_eric = t_init (EricInitialisiere) + t_first_validate (Plugin-Load + Pruefung),
  damit sichtbar ist, WO die Kaltstart-Kosten sitzen (Init vs. lazy Plugin-Load).

Offline, ERIC_VALIDIERE (KEIN ERIC_SENDE), keine Datei-Credentials. Hersteller-ID NUR aus
$ELSTER_HERSTELLER_ID (nie im Code). Falsch-Gruen-Sperre: jede Messung MUSS rc==0 liefern
(der volle Plausibilitaetspfad); ein rc!=0 (z.B. 610301202 GESPERRT) kurzschliesst am
Hersteller-ID-Gate VOR dem Plugin-Load und wuerde eine truncierte, zu kurze Latenz messen —
solche Laeufe werden hart abgebrochen, nicht stillschweigend als "schnell" verbucht.

Report untracked; Commit ueber Instructor.

Aufruf:
    ERIC_DIR=~/02_Software/eric ELSTER_HERSTELLER_ID=... \
        python3 elster/bench/latency_checkest.py [--reps 10] [--json OUT.json]
"""
from __future__ import annotations

import argparse
import json
import os
import statistics
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ELSTER = os.path.dirname(HERE)
sys.path.insert(0, ELSTER)

import checkest_gate as CE  # noqa: E402  (validate / _mit_hersteller_id / beende)

CASES = {
    "minimal_2025": (os.path.join(ELSTER, "submission", "testfall_est2025_minimal.xml"), "ESt_2025"),
    "realistisch_2020": (os.path.join(ELSTER, "testdaten", "est_2020_amtliches_beispiel.xml"), "ESt_2020"),
}


def _load_case(name: str) -> tuple[bytes, str]:
    path, datenart = CASES[name]
    with open(path, "rb") as f:
        raw = f.read()
    xml, hid = CE._mit_hersteller_id(raw)
    if not hid:
        sys.exit("ABBRUCH: $ELSTER_HERSTELLER_ID leer — ohne registrierte ID misst man nur den "
                 "GESPERRT-Kurzschluss (rc=610301202), nicht den vollen Plausibilitaetspfad.")
    return xml, datenart


def _cold_one(name: str) -> None:
    """Subprozess-Einstieg: EINE Kaltmessung. Gibt {t_init, t_first_validate, rc, bytes} als JSON.

    Laeuft in frischem python-Prozess; ERiC ist hier garantiert ungeladen -> echter Kaltstart.
    """
    xml, datenart = _load_case(name)
    t0 = time.perf_counter()
    CE._load_and_init()                       # EricInitialisiere (+ ctypes CDLL laden)
    t1 = time.perf_counter()
    rc, _ = CE.validate(xml, datenart)        # erste Validierung -> lazy Plugin-Load (libcheckESt)
    t2 = time.perf_counter()
    CE.beende()
    print(json.dumps({"t_init": t1 - t0, "t_first_validate": t2 - t1, "rc": rc, "bytes": len(xml)}))


def _stats(xs: list[float]) -> dict:
    xs = sorted(xs)
    n = len(xs)
    # p95 per nearest-rank (bei kleinem n ~ Maximum); dokumentiert, damit nicht ueberinterpretiert.
    k = max(0, min(n - 1, -(-95 * n // 100) - 1))
    return {"n": n, "min_ms": xs[0] * 1e3, "median_ms": statistics.median(xs) * 1e3,
            "p95_ms": xs[k] * 1e3, "max_ms": xs[-1] * 1e3,
            "mean_ms": statistics.fmean(xs) * 1e3}


def _fmt(s: dict) -> str:
    return (f"n={s['n']:2d}  median={s['median_ms']:8.1f}ms  p95={s['p95_ms']:8.1f}ms  "
            f"min={s['min_ms']:8.1f}ms  max={s['max_ms']:8.1f}ms")


def measure(name: str, reps: int) -> dict:
    print(f"\n[bench] Fall '{name}' ({CASES[name][1]}, {reps} Wdh.)", flush=True)

    # --- COLD: je Messung ein frischer Subprozess (ERiC garantiert ungeladen) ---
    cold_wall, cold_eric, t_init, t_first = [], [], [], []
    env = dict(os.environ)
    for i in range(reps):
        t0 = time.perf_counter()
        p = subprocess.run([sys.executable, os.path.abspath(__file__), "--cold-one", name],
                           capture_output=True, text=True, env=env)
        wall = time.perf_counter() - t0
        if p.returncode != 0:
            sys.exit(f"ABBRUCH: Kalt-Subprozess Fall '{name}' rc={p.returncode}\n{p.stderr[-800:]}")
        rec = json.loads(p.stdout.strip().splitlines()[-1])
        if rec["rc"] != 0:
            sys.exit(f"ABBRUCH (Falsch-Gruen-Sperre): Kaltlauf Fall '{name}' lieferte ELSTER-rc="
                     f"{rec['rc']} (kein rc==0). Latenz waere truncierter Pfad — nicht verwertbar.")
        cold_wall.append(wall)
        cold_eric.append(rec["t_init"] + rec["t_first_validate"])
        t_init.append(rec["t_init"])
        t_first.append(rec["t_first_validate"])
        print(f"  cold[{i+1:2d}] wall={wall*1e3:8.1f}ms  eric={ (rec['t_init']+rec['t_first_validate'])*1e3:8.1f}ms  "
              f"(init={rec['t_init']*1e3:.1f} + first={rec['t_first_validate']*1e3:.1f})", flush=True)

    # --- WARM: ein Prozess, Init+Plugin einmal geladen, dann Steady-State ---
    xml, datenart = _load_case(name)
    rc0, _ = CE.validate(xml, datenart)       # Warm-up: laedt Plugin, verwirft Zeit
    if rc0 != 0:
        sys.exit(f"ABBRUCH (Falsch-Gruen-Sperre): Warm-up Fall '{name}' rc={rc0} != 0.")
    warm = []
    for i in range(reps):
        t0 = time.perf_counter()
        rc, _ = CE.validate(xml, datenart)
        dt = time.perf_counter() - t0
        if rc != 0:
            sys.exit(f"ABBRUCH: Warm-Lauf Fall '{name}' rc={rc} != 0.")
        warm.append(dt)
        print(f"  warm[{i+1:2d}] {dt*1e3:8.1f}ms", flush=True)
    CE.beende()

    s = {"case": name, "datenart": datenart, "bytes": len(xml),
         "cold_wall": _stats(cold_wall), "cold_eric": _stats(cold_eric),
         "t_init": _stats(t_init), "t_first_validate": _stats(t_first),
         "warm": _stats(warm)}
    print(f"  => cold_wall  {_fmt(s['cold_wall'])}")
    print(f"  => cold_eric  {_fmt(s['cold_eric'])}   (init {_fmt(s['t_init'])})")
    print(f"  => warm       {_fmt(s['warm'])}")
    return s


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--cold-one", metavar="CASE", help="(intern) eine Kaltmessung als Subprozess")
    ap.add_argument("--reps", type=int, default=10)
    ap.add_argument("--json", metavar="OUT", help="Ergebnis als JSON schreiben")
    args = ap.parse_args()

    if args.cold_one:
        _cold_one(args.cold_one)
        return 0

    if not os.environ.get("ELSTER_HERSTELLER_ID", "").strip():
        return int(bool(sys.stderr.write(
            "ABBRUCH: $ELSTER_HERSTELLER_ID nicht gesetzt — Messung braucht den rc==0-Pfad.\n")) ) or 2

    print(f"[bench] ERiC checkESt Latenz — reps={args.reps}, ERIC_VALIDIERE offline, HID gesetzt.")
    results = [measure(name, args.reps) for name in CASES]

    # --- datengetriebenes Verdikt (Entscheidungsvorlage, finale Entscheidung Instructor) ---
    print("\n=== VERDIKT (Entscheidungsvorlage sync/async) ===")
    for s in results:
        cw, wm = s["cold_wall"]["p95_ms"], s["warm"]["p95_ms"]
        print(f"[{s['case']:16s}] cold_wall p95={cw:.0f}ms  warm p95={wm:.0f}ms  "
              f"Faktor={cw/max(wm,1e-6):.0f}x")
    worst_cold = max(s["cold_wall"]["p95_ms"] for s in results)
    best_warm_p95 = max(s["warm"]["p95_ms"] for s in results)
    print(f"\nKaltstart p95 (schlechtester Fall): {worst_cold:.0f}ms — dominiert von "
          f"EricInitialisiere+Plugin-Load. Warm p95 (schlechtester Fall): {best_warm_p95:.0f}ms.")
    print("Lesart: cold ~ Fork-per-Request (naive Synchron-UI). warm ~ langlebiger ERiC-Daemon.")

    if args.json:
        with open(args.json, "w") as f:
            json.dump({"reps": args.reps, "results": results}, f, indent=2)
        print(f"\n[bench] JSON -> {args.json}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
