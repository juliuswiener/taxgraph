#!/usr/bin/env python3
"""Rauchprobe: Rust-Dienst gegen Python-Dienst auf KOPIEN der echten Daten, nur lesende Routen.

    PARITY=1 CARGO_TARGET_DIR=<Verzeichnis auf Platte, nicht /tmp> python3 tools/parity/rauchprobe_echtdaten.py
    python3 tools/parity/rauchprobe_echtdaten.py --selbsttest        # braucht weder PARITY noch Dienste

Ablauf: 1. Groesse der Quelle (`du -sk`); ueber --max-mb (Standard 2048) meldet das Skript und stoppt.
2. Je Dienst eine Kopie der Quelle unter --ziel (nie das Original schreiben, nie ein Dienst auf dem Original).
3. Je ein Rust- und ein Python-Dienst auf ihrer Kopie, TAXGRAPH_NO_AUTH=1 wie in den Parity-Suiten.
4. Je Fall die lesenden Routen stand, fragen, ergebnis, preflight, deklaration, graph (`ergebnis` ist die
   Bescheid-Rechnung; eine Route `bescheid` gibt es in keinem Dienst) und je Fall fuer die ersten
   FELDER_JE_FALL offenen Fragen `feld/<id>/frage` und `feld/<id>/warum`; Antworten (Status, geparstes JSON) vergleichen.
5. Bericht NUR mit Zaehlern, Routennamen und Feldpfaden. KEINE Werte, keine Fall- oder Feldnamen aus den Daten,
   keine Kennungen: weder im Bericht noch auf der Konsole (Daten koennen personenbezogen sein). Pfade zeigen
   Listenindizes als `[]`, Schluessel mit langen Ziffern- oder Hex-Folgen als `<id>`.
6. Beide Dienste beenden, die Kopien loeschen, die Quelle auf Veraenderung pruefen (Dateizahl, Bytes, juengste mtime).

Ohne PARITY=1 laeuft das Skript nicht (es braucht beide Dienste: Python-Orakel, Catala-Bau).
Exit 0: alles gleich; 1: Abweichungen; 2: nicht erlaubt/Aufruffehler; 3: Quelle zu gross.
"""
from __future__ import annotations

import argparse
import collections
import datetime
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
HOME = Path.home()
QUELLE = HOME / ".local" / "share" / "taxgraph"
ZIEL = HOME / ".cache" / "taxgraph-tmp" / "rauch-daten"
BERICHT = HOME / ".cache" / "taxgraph-tmp" / "berichte" / "rauchprobe-echtdaten.md"
MARKE = ".rauchprobe-kopie"
FALL_ROUTEN = ("stand", "fragen", "ergebnis", "preflight", "deklaration", "graph")
FELD_ROUTEN = ("frage", "warum")
# ponytail: nur die ersten drei offenen Fragen je Fall; alle Felder je Fall vervielfachen die Abfragen
# um den Faktor der Fragenzahl. Wer mehr will, hebt die Zahl an.
FELDER_JE_FALL = 3
ID = re.compile(r"^[A-Za-z0-9_-]{1,64}$")
FID = re.compile(r"^[A-Za-z0-9_]{1,64}$")
BANNER = re.compile(r"http://127\.0\.0\.1:(\d+)")


# ---------------------------------------------------------------- Vergleich (ohne Werte)

def _schluessel(k: str) -> str:
    return "<id>" if re.search(r"\d{6,}|[0-9a-f]{20,}", k) else k


def diffs(a, b, pfad: str = "$", out: list | None = None, grenze: int = 5) -> list[str]:
    """Pfade der ersten `grenze` Unterschiede; nie ein Wert. Bool ist keine Zahl, int und float vergleichen als Zahl."""
    out = [] if out is None else out
    if len(out) >= grenze:
        return out
    zahl = lambda x: isinstance(x, (int, float)) and not isinstance(x, bool)  # noqa: E731
    if zahl(a) and zahl(b):
        if a != b:
            out.append(pfad)
    elif type(a) is not type(b):
        out.append(pfad)
    elif isinstance(a, dict):
        for k in sorted(set(a) | set(b)):
            if k not in a or k not in b:
                out.append(f"{pfad}.{_schluessel(k)}")
            else:
                diffs(a[k], b[k], f"{pfad}.{_schluessel(k)}", out, grenze)
    elif isinstance(a, list):
        if len(a) != len(b):
            out.append(f"{pfad}[]")
        else:
            for x, y in zip(a, b):
                diffs(x, y, f"{pfad}[]", out, grenze)
    elif a != b:
        out.append(pfad)
    return out[:grenze]


def vergleiche(rust: tuple[int, bytes], python: tuple[int, bytes]) -> list[str]:
    """Leer = gleich. Sonst Feldpfade, oder ein Statuspaar, wenn die Status verschieden sind."""
    (sr, br), (sp, bp) = rust, python
    if sr == -1 or sp == -1:
        return ["Verbindungsfehler " + "/".join(n for n, s in (("Rust", sr), ("Python", sp)) if s == -1)]
    if sr != sp:
        return [f"Status {sr}/{sp}"]
    try:
        jr, jp = json.loads(br), json.loads(bp)
    except ValueError:
        return [] if br == bp else ["kein JSON, Rohtext verschieden"]
    return diffs(jr, jp)


def selbsttest() -> None:
    assert diffs({"a": [1, {"b": 2}]}, {"a": [1, {"b": 2}]}) == []
    assert diffs({"a": 1}, {"a": 1.0}) == [], "int und float gleichen Werts sind gleich"
    assert diffs({"a": True}, {"a": 1}) == ["$.a"], "bool ist keine Zahl"
    assert diffs({"a": [1, {"b": 2}]}, {"a": [1, {"b": 3}]}) == ["$.a[].b"], "Listenindex wird []"
    assert diffs({"k123456789": 1}, {"k123456789": 2}) == ["$.<id>"], "Schluessel mit langer Ziffernfolge wird <id>"
    assert diffs({"x": 1}, {}) == ["$.x"]
    assert vergleiche((500, b"{}"), (200, b"{}")) == ["Status 500/200"]
    assert vergleiche((200, b'{"a":1}'), (200, b'{"a": 1}')) == []
    assert vergleiche((-1, b""), (200, b"{}")) == ["Verbindungsfehler Rust"]
    assert "GEHEIMWERT" not in "".join(diffs({"a": "GEHEIMWERT"}, {"a": "anderer"})), "ein Pfad nennt nie einen Wert"
    print("selbsttest ok")


# ---------------------------------------------------------------- Dateisystem

def fingerprint(pfad: Path) -> tuple[int, int, int]:
    """(Dateien, Bytes, juengste mtime in ns) ohne Symlinks zu folgen: erkennt jede Veraenderung der Quelle."""
    n = b = m = 0
    for wurzel, ordner, dateien in os.walk(pfad, followlinks=False):
        for name in [*ordner, *dateien]:
            st = os.lstat(os.path.join(wurzel, name))
            m = max(m, st.st_mtime_ns)
            if name in dateien:
                n += 1
                b += st.st_size
    return n, b, max(m, os.lstat(pfad).st_mtime_ns)


def groesse_kb(pfad: Path) -> int:
    return int(subprocess.run(["du", "-sk", str(pfad)], capture_output=True, text=True, check=True).stdout.split()[0])


def kopiere(quelle: Path, ziel: Path) -> None:
    """Kopie ohne Symlinks: ein Link, der aus der Kopie auf das Original zeigt, liesse den Dienst es schreiben."""
    shutil.copytree(quelle, ziel, symlinks=True)
    for wurzel, ordner, dateien in os.walk(ziel):
        for name in [*ordner, *dateien]:
            if os.path.islink(os.path.join(wurzel, name)):
                raise RuntimeError("Symlink in den Daten")


def raeume(ziel: Path) -> None:
    """Loescht nur ein Verzeichnis, das dieses Skript angelegt hat (Marke) und das unter ~/.cache/taxgraph-tmp liegt."""
    if not ziel.exists():
        return
    erlaubt = (HOME / ".cache" / "taxgraph-tmp").resolve()
    if erlaubt not in ziel.resolve().parents or not (ziel / MARKE).exists():
        raise RuntimeError("Ziel ist nicht von diesem Skript angelegt oder liegt ausserhalb von ~/.cache/taxgraph-tmp")
    shutil.rmtree(ziel)


# ---------------------------------------------------------------- Dienste

def umgebung(daten: Path) -> dict[str, str]:
    env = dict(os.environ)
    env.pop("XDG_DATA_HOME", None)
    env.update({
        "TAXGRAPH_DATEN": str(daten), "TAXGRAPH_AUDIT_DIR": str(daten / "faelle"),
        "TAXGRAPH_USER_STORE": str(daten / "users.json"), "TAXGRAPH_JWT_SECRET": "rauchprobe",
        "TAXGRAPH_NO_AUTH": "1", "TAXGRAPH_FLOW": "0", "TAXGRAPH_KI_DEBUG": "0", "TAXGRAPH_ROOT": str(REPO),
        # Echte Schluessel aus der Shell und aus .env/.env.llm (die Dienste laden nur NICHT gesetzte Schluessel):
        # leer, beziehungsweise die oeffentliche Test-Hersteller-ID.
        "LLM_API_KEY": "", "LLM_API_BASE": "", "LLM_MODEL": "", "ORS_API_KEY": "", "ORS_API_BASE": "",
        "ELSTER_HERSTELLER_ID": "74931", "ELSTER_ZERTIFIKAT_PFAD": "", "ELSTER_ZERTIFIKAT_PIN": "",
    })
    return env


def starte(kommando: list[str], daten: Path, log: Path) -> tuple[subprocess.Popen, str]:
    proc = subprocess.Popen(kommando, cwd=REPO, env=umgebung(daten), stdout=open(log, "wb"),
                            stderr=subprocess.STDOUT, start_new_session=True)
    ende = time.time() + 90
    while time.time() < ende:
        m = BANNER.search(log.read_text(errors="replace"))
        if m:
            return proc, f"http://127.0.0.1:{m.group(1)}"
        if proc.poll() is not None:
            raise RuntimeError("Dienst endete beim Start")
        time.sleep(0.2)
    raise RuntimeError("Dienst meldet keinen Port")


def stoppe(proc: subprocess.Popen | None) -> None:
    if proc is None or proc.poll() is not None:
        return
    proc.send_signal(signal.SIGTERM)
    try:
        proc.wait(15)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()


def hole(basis: str, pfad: str) -> tuple[int, bytes]:
    try:
        with urllib.request.urlopen(basis + pfad, timeout=120) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()
    except Exception:  # noqa: BLE001 -- nur zaehlen: eine Meldung koennte Pfad mit Fallkennung enthalten
        return -1, b""


# ---------------------------------------------------------------- Lauf

def lauf(quelle: Path, ziel: Path, bericht: Path, max_mb: int) -> int:
    if os.environ.get("PARITY") != "1":
        print("Abbruch: ohne PARITY=1 laeuft die Rauchprobe nicht (sie braucht beide Dienste).", file=sys.stderr)
        return 2
    cargo_ziel = os.environ.get("CARGO_TARGET_DIR", "")
    if not cargo_ziel or Path(cargo_ziel).resolve().is_relative_to("/tmp"):
        print("Abbruch: CARGO_TARGET_DIR auf Platte setzen (nicht /tmp, dort ist RAM).", file=sys.stderr)
        return 2
    quelle, ziel = quelle.resolve(), ziel.resolve()
    if not (quelle / "faelle").is_dir():
        print("Abbruch: Quelle hat kein Verzeichnis faelle/.", file=sys.stderr)
        return 2
    if ziel == quelle or quelle in ziel.parents or ziel in quelle.parents:
        print("Abbruch: Ziel und Quelle duerfen sich nicht enthalten.", file=sys.stderr)
        return 2
    kb = groesse_kb(quelle)
    print(f"Quelle: {kb / 1024:.0f} MB (Grenze {max_mb} MB)")
    if kb > max_mb * 1024:
        print("Stopp: Quelle groesser als die Grenze, nichts kopiert, kein Dienst gestartet.")
        return 3

    vorher = fingerprint(quelle)
    if ziel.exists():
        raeume(ziel)                    # nur ein Rest eines eigenen frueheren Laufs; sonst Fehler
    ziel.mkdir(parents=True)
    (ziel / MARKE).write_text("von rauchprobe_echtdaten.py angelegt, darf geloescht werden\n")
    rust_proc = py_proc = None
    zaehler: dict[str, collections.Counter] = {r: collections.Counter() for r in (*FALL_ROUTEN, *(f"feld/{r}" for r in FELD_ROUTEN))}
    abw: collections.Counter = collections.Counter()
    n_faelle = n_ungueltig = 0
    try:
        for dienst in ("rust", "python"):
            kopiere(quelle, ziel / dienst)
        print("Bau: cargo build -p api --bin taxgraph-api")
        subprocess.run(["cargo", "build", "-q", "-p", "api", "--bin", "taxgraph-api"], cwd=REPO / "rust", check=True,
                       env={**os.environ, "CARGO_TARGET_DIR": cargo_ziel}, capture_output=True)
        rust_proc, rust = starte([f"{cargo_ziel}/debug/taxgraph-api", "0"], ziel / "rust", ziel / "rust.log")
        py_proc, py = starte([sys.executable, "-u", str(REPO / "produkt/haut/server.py"), "0"], ziel / "python", ziel / "python.log")
        faelle = sorted(p.stem for p in (ziel / "rust" / "faelle").glob("*.json"))
        for fid in faelle:
            if not ID.match(fid):
                n_ungueltig += 1
                continue
            n_faelle += 1
            fragen = None
            for route in FALL_ROUTEN:
                r, p = hole(rust, f"/fall/{fid}/{route}"), hole(py, f"/fall/{fid}/{route}")
                pfade = vergleiche(r, p)
                zaehler[route]["abweichend" if pfade else "gleich"] += 1
                for pf in pfade:
                    abw[(route, pf)] += 1
                if route == "fragen" and r[0] == 200:
                    try:
                        fragen = [q.get("feld_id") for q in json.loads(r[1]).get("fragen", []) if isinstance(q, dict)]
                    except ValueError:
                        fragen = None
            for feld in [f for f in (fragen or []) if isinstance(f, str) and FID.match(f)][:FELDER_JE_FALL]:
                for route in FELD_ROUTEN:
                    name = f"feld/{route}"
                    r, p = hole(rust, f"/fall/{fid}/feld/{feld}/{route}"), hole(py, f"/fall/{fid}/feld/{feld}/{route}")
                    pfade = vergleiche(r, p)
                    zaehler[name]["abweichend" if pfade else "gleich"] += 1
                    for pf in pfade:
                        abw[(name, pf)] += 1
    finally:
        stoppe(rust_proc)
        stoppe(py_proc)
        raeume(ziel)
    nachher = fingerprint(quelle)

    n_gleich = sum(c["gleich"] for c in zaehler.values())
    n_abw = sum(c["abweichend"] for c in zaehler.values())
    routen = [r for r, c in zaehler.items() if c["gleich"] or c["abweichend"]]
    commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=REPO, capture_output=True, text=True).stdout.strip()
    zeilen = [
        "# Rauchprobe Rust gegen Python auf Kopien der echten Daten",
        "",
        f"Lauf {datetime.datetime.now().isoformat(timespec='seconds')}, Skript `tools/parity/rauchprobe_echtdaten.py` auf `{commit}`. "
        "Der Bericht nennt nur Zaehler, Routennamen und Feldpfade; keine Werte, Namen oder Kennungen.",
        "",
        f"- Quelle `{quelle}` ({kb / 1024:.0f} MB), je Dienst eine eigene Kopie; "
        f"Kopien danach geloescht: {'ja' if not ziel.exists() else 'NEIN'}",
        f"- Quelle unveraendert (Dateien, Bytes, juengste mtime): {'ja' if vorher == nachher else 'NEIN'}",
        f"- Faelle: {n_faelle} verglichen, {n_ungueltig} mit einem Dateinamen ausserhalb des Routenmusters uebersprungen",
        f"- Routen: {len(routen)} ({', '.join(routen)})",
        f"- Abfragen je Dienst: {n_gleich + n_abw}; gleich: {n_gleich}; abweichend: {n_abw}",
        f"- je Fall hoechstens {FELDER_JE_FALL} offene Fragen fuer `feld/frage` und `feld/warum`",
        "- Nicht gemessen: schreibende Routen, Anmeldung (TAXGRAPH_NO_AUTH=1), feste Uhr, Fragen jenseits der ersten "
        f"{FELDER_JE_FALL} je Fall, `chat`, `entfernung`, `einreichen`, `kontoauszug`.",
        "",
        "| Route | gleich | abweichend |", "|---|---:|---:|",
        *[f"| {r} | {zaehler[r]['gleich']} | {zaehler[r]['abweichend']} |" for r in routen],
        "",
    ]
    if abw:
        zeilen += ["## Abweichungen (Route, Feldpfad oder Statuspaar Rust/Python, Zahl der Abfragen)", "",
                   "| Route | Feldpfad | Abfragen |", "|---|---|---:|",
                   *[f"| {r} | `{pf}` | {n} |" for (r, pf), n in sorted(abw.items())], ""]
    else:
        zeilen += ["Keine Abweichung.", ""]
    bericht.parent.mkdir(parents=True, exist_ok=True)
    bericht.write_text("\n".join(zeilen), encoding="utf-8")
    print("\n".join(z for z in zeilen[4:11] if z))
    print(f"Bericht: {bericht}")
    return 0 if not n_abw and vorher == nachher else 1


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--selbsttest", action="store_true")
    ap.add_argument("--quelle", type=Path, default=QUELLE)
    ap.add_argument("--ziel", type=Path, default=ZIEL)
    ap.add_argument("--bericht", type=Path, default=BERICHT)
    ap.add_argument("--max-mb", type=int, default=2048)
    a = ap.parse_args()
    if a.selbsttest:
        selbsttest()
        return 0
    try:
        return lauf(a.quelle, a.ziel, a.bericht, a.max_mb)
    except Exception as e:  # noqa: BLE001 -- nur der Klassenname: eine Meldung koennte Daten enthalten
        print(f"Abbruch: {type(e).__name__}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
