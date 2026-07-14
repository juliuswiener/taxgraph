"""Verified-Snapshot-Mechanik (runs/-Blocker-Fix, 2026-07-14).

`pipeline/runs/` ist gitignored -> ein frischer Checkout hat keine report.json und
kann die Per-Regel-Gates (equivalence, clerk, roundtrip, ...) NICHT regaten. Bisher
lag das komplette Verdikt jeder Regel nur im lokalen runs/-Baum der Session, die den
Lauf gefahren hat (dokumentierter runs/-Blocker).

Der Snapshot committet die DETERMINISTISCH pruefbaren Teile jedes VERIFIZIERTEN
Reports - catala_a/b, module_name, judge_verdict (roh), gates, bedingungen - plus
sha256(catala_a) als Manipulations-/Drift-Waechter. Damit rekonstruiert
`run.py --regate` auf einem frischen Clone das deterministische Verdikt aus dem
Snapshot, ohne Modellkosten.

Instructor-Praezisierungen (2026-07-14):
  1. Snapshot = deterministisch pruefbare Teile + sha256(catala_a) in der Datei.
  2. Vorrang: eine LIVE report.json in runs/ schlaegt den Snapshot (mit Warnung) -
     in-flight-Arbeit ist kanonischer als das Archiv. Der Snapshot ist kanonisch
     NUR, wenn kein Live-Report existiert.
  3. Integritaet: load_snapshot verifiziert sha256(catala_a). Ein manipulierter
     Snapshot (catala_a geaendert, Hash nicht) FAILt hart - nie stiller PASS. Der
     Negativtest in tests/test_snapshot.py haelt das fest.

Nur verified*-Regeln werden gesnapshottet: der Snapshot ist ein Vertrauensanker,
kein Arbeitsstand. flagged_for_review/discovery_triage/... bleiben in runs/.

CLI:
    python pipeline/snapshot.py write [--all | <rule_id> ...]
    python pipeline/snapshot.py verify [--all | <rule_id> ...]
"""

from __future__ import annotations

import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
OUT_ROOT = os.path.join(HERE, "runs", "produktion")
SNAP_DIR = os.path.join(HERE, "snapshots")

SCHEMA_VERSION = 1
# Nur diese Felder aus dem Report wandern in den Snapshot (deterministisch pruefbar).
# KEINE Kosten/Provenance/Timestamps - der Snapshot ist reproduzierbar, nicht auditlog.
SNAP_FIELDS = ("candidate_id", "queue_status", "module_name",
               "catala_a", "catala_b", "judge_verdict", "gates", "bedingungen")


class SnapshotIntegrityError(Exception):
    """catala_a stimmt nicht mit dem gespeicherten sha256 ueberein - der Snapshot
    ist manipuliert oder korrumpiert. Muss hart failen, nie still PASS."""


def _sha(text: str) -> str:
    return hashlib.sha256((text or "").encode("utf-8")).hexdigest()


def live_path(rid: str) -> str:
    return os.path.join(OUT_ROOT, rid, "report.json")


def snapshot_path(rid: str) -> str:
    return os.path.join(SNAP_DIR, f"{rid}.json")


def is_snapshotwuerdig(rep: dict) -> bool:
    """Ein Snapshot ist ein Vertrauensanker: nur verified*-Regeln mit catala_a."""
    return bool(rep.get("catala_a")) and str(rep.get("queue_status", "")).startswith("verified")


def build_snapshot(rep: dict) -> dict:
    snap = {"schema_version": SCHEMA_VERSION}
    for f in SNAP_FIELDS:
        snap[f] = rep.get(f)
    snap["catala_a_sha256"] = _sha(rep.get("catala_a") or "")
    return snap


def write_snapshot(rid: str, rep: dict | None = None) -> tuple[str, str]:
    """Schreibt den Snapshot einer verifizierten Regel. Rueckgabe (pfad, status).
    status: 'geschrieben' | 'uebersprungen:<grund>'."""
    if rep is None:
        p = live_path(rid)
        if not os.path.exists(p):
            return snapshot_path(rid), "uebersprungen:kein_live_report"
        rep = json.load(open(p, encoding="utf-8"))
    if not is_snapshotwuerdig(rep):
        return snapshot_path(rid), f"uebersprungen:{rep.get('queue_status', 'kein_verdikt')}"
    os.makedirs(SNAP_DIR, exist_ok=True)
    sp = snapshot_path(rid)
    with open(sp, "w", encoding="utf-8") as f:
        json.dump(build_snapshot(rep), f, ensure_ascii=False, indent=2, sort_keys=True)
    return sp, "geschrieben"


def load_snapshot(rid: str) -> dict:
    """Laedt einen Snapshot und verifiziert sha256(catala_a). Mismatch -> hart FAIL.

    Rueckgabe: ein report-artiges Dict (catala_a/b, module_name, judge_verdict,
    gates, bedingungen, queue_status), das `regate` identisch zu einem Live-Report
    konsumiert.
    """
    sp = snapshot_path(rid)
    if not os.path.exists(sp):
        raise FileNotFoundError(sp)
    snap = json.load(open(sp, encoding="utf-8"))
    ist = _sha(snap.get("catala_a") or "")
    soll = snap.get("catala_a_sha256")
    if ist != soll:
        raise SnapshotIntegrityError(
            f"{rid}: catala_a-sha256 stimmt nicht (Datei {soll!r}, berechnet {ist!r}) "
            f"- Snapshot manipuliert/korrumpiert. KEIN Verdikt aus diesem Snapshot.")
    rep = {f: snap.get(f) for f in SNAP_FIELDS}
    rep["aus_snapshot"] = True
    return rep


def resolve_report(rid: str) -> tuple[dict | None, str, str, str]:
    """Loest den Report einer Regel auf (Instructor-Vorrangregel 2).

    Rueckgabe: (report | None, quelle, live_pfad, warnung).
      quelle: 'live'     - Live-Report in runs/ (kanonisch; in-flight schlaegt Archiv)
              'snapshot' - kein Live-Report, aus dem committeten Snapshot rekonstruiert
              'none'     - weder noch
    Ein manipulierter Snapshot laesst load_snapshot hart failen (SnapshotIntegrityError)
    - das propagiert bewusst, nie stiller Fallback.
    """
    live = live_path(rid)
    has_live = os.path.exists(live)
    has_snap = os.path.exists(snapshot_path(rid))
    if has_live:
        rep = json.load(open(live, encoding="utf-8"))
        warn = ("Live-Report UND Snapshot vorhanden - live gilt (in-flight schlaegt "
                "Archiv); Snapshot via 'snapshot.py write' nachziehen, wenn der Lauf "
                "abgenommen ist") if has_snap else ""
        return rep, "live", live, warn
    if has_snap:
        return load_snapshot(rid), "snapshot", live, ""
    return None, "none", live, ""


# -- CLI ----------------------------------------------------------------------

def _rule_ids() -> list[str]:
    sys.path.insert(0, ROOT)   # yamlstrict liegt im Repo-Root (wie run.py/item_registry)
    from yamlstrict import load_yaml
    cfg = load_yaml(os.path.join(HERE, "produktion", "rules.yaml"))
    return [r["rule_id"] for r in cfg["regeln"]]


def _targets(argv: list[str]) -> list[str]:
    if "--all" in argv or not argv:
        return _rule_ids()
    return argv


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in ("write", "verify"):
        print("Verwendung:\n"
              "  python pipeline/snapshot.py write [--all | <rule_id> ...]\n"
              "  python pipeline/snapshot.py verify [--all | <rule_id> ...]")
        return 2
    cmd = sys.argv[1]
    targets = _targets(sys.argv[2:])
    if cmd == "write":
        geschrieben = 0
        for rid in targets:
            _, st = write_snapshot(rid)
            if st == "geschrieben":
                geschrieben += 1
            elif not st.startswith("uebersprungen:kein_live_report") or "--all" not in sys.argv:
                print(f"  {rid}: {st}")
        print(f"{geschrieben} Snapshot(s) geschrieben -> {SNAP_DIR}")
        return 0
    # verify: jeden Snapshot laden (prueft sha256) - Mismatch = harter Fehler.
    fehler = 0
    geprueft = 0
    for rid in targets:
        if not os.path.exists(snapshot_path(rid)):
            continue
        geprueft += 1
        try:
            load_snapshot(rid)
        except SnapshotIntegrityError as e:
            fehler += 1
            print(f"  INTEGRITAET VERLETZT: {e}")
    print(f"{geprueft} Snapshot(s) geprueft, {fehler} Integritaets-Fehler.")
    return 1 if fehler else 0


if __name__ == "__main__":
    sys.exit(main())
