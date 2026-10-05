#!/usr/bin/env python3
"""Echtakten-Aufzeichnung: die Antworten des Python-Dienstes auf den echten Akten, einmal festgehalten.

    python3 tools/parity/echtakten_aufzeichnen.py --selbsttest
    python3 tools/parity/echtakten_aufzeichnen.py aufzeichnen        # braucht `make build-python`, kein PARITY

Warum: `rauchprobe_echtdaten.py` vergleicht Rust und Python live auf KOPIEN der Akten. Faellt Python weg, faellt
der Vergleich weg. Dieses Skript haelt die Python-Seite fest (Pruefsummen, keine Antworten); der Rust-Test
`rust/api/tests/echtakten_vergleich.rs` (`#[ignore]`, `TAXGRAPH_AUFZEICHNUNG=<Verzeichnis>`) prueft Rust dagegen.

Je Fall und lesender Route (stand, fragen, ergebnis, preflight, deklaration, graph und fuer die ersten
FELDER_JE_FALL offenen Fragen feld/frage, feld/warum): Status und kanonische Pruefsumme der Antwort. Je Fall
ausserdem SHA-256 der Eingabedatei, Zahl der Ereignisse, Pruefsumme der Liste der `event_id`.

DATENSCHUTZ: Die Aufzeichnung enthaelt nur Pruefsummen und Zaehler. Der Fall-Schluessel ist die erste Haelfte von
SHA-256(Fall-ID); weder Fall-ID noch Feldname noch Wert steht darin. Sie liegt unter
`~/.local/share/taxgraph-aufzeichnung/<Datum>/` (0700/0600), nie im Repo. Bericht und Konsole nennen nur Zaehler.
Das Original der Akten wird nie geschrieben: der Dienst laeuft auf einer Kopie, die Quelle wird vorher und nachher
verglichen (Dateien, Bytes, juengste mtime).

Kanonische Pruefsumme (Python und Rust byte-gleich, Known-Answer-Vektor in beiden): typmarkierte Folge ueber den
JSON-Baum. n / t / f; Ganzzahl und ganzzahliger Gleitwert unter 2^53 als `i<dezimal>;`; sonstiger Gleitwert als
`d` + 8 Byte Big-Endian-Bits; Zeichenkette `s<Bytelaenge>:<UTF-8>`; Liste `[<n>:` + Elemente; Objekt `{<n>:` +
je Schluessel (als Zeichenkette) und Wert, Schluessel nach Codepunkt sortiert. Keine Entsprechung fuer Ganzzahlen
ausserhalb von i64/u64: das Skript bricht dann ab, statt eine Abweichung vorzutaeuschen.
"""
from __future__ import annotations

import argparse
import collections
import datetime
import hashlib
import json
import os
import struct
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rauchprobe_echtdaten as R  # noqa: E402  -- Kopie, Dienststart, Abfrage, Fingerabdruck der Quelle

FORMAT = 1
AUSGABE = R.HOME / ".local" / "share" / "taxgraph-aufzeichnung"
ZIEL = R.HOME / ".cache" / "taxgraph-tmp" / "echt-aufzeichnung"
# Known-Answer-Vektor: derselbe Wert und dieselbe Pruefsumme stehen in `echtakten_vergleich.rs`.
VEKTOR = {"b": [1, 2.0, 2.5, "äö", None, True, False],
          "a": {"x": -0.0, "y": 1e300, "z": "", "w": -7, "ké": [[], {}]}}
VEKTOR_SHA256 = "3ffffed035be23fb820d672437451d5d555315c4991f6797cce0b870cc27f110"
GRENZE_GLEITKOMMA = 2 ** 53


def _speise(h, x) -> None:
    if x is None:
        h.update(b"n")
    elif x is True:
        h.update(b"t")
    elif x is False:
        h.update(b"f")
    elif isinstance(x, int):
        if not -(2 ** 63) <= x < 2 ** 64:
            raise ValueError("Ganzzahl ausserhalb von i64/u64")
        h.update(b"i" + str(x).encode() + b";")
    elif isinstance(x, float):
        if x == x and abs(x) < GRENZE_GLEITKOMMA and x == int(x):
            h.update(b"i" + str(int(x)).encode() + b";")
        else:
            h.update(b"d" + struct.pack(">d", x))
    elif isinstance(x, str):
        b = x.encode("utf-8")
        h.update(b"s" + str(len(b)).encode() + b":" + b)
    elif isinstance(x, list):
        h.update(b"[" + str(len(x)).encode() + b":")
        for e in x:
            _speise(h, e)
    elif isinstance(x, dict):
        h.update(b"{" + str(len(x)).encode() + b":")
        for k in sorted(x):
            _speise(h, k)
            _speise(h, x[k])
    else:
        raise TypeError(type(x).__name__)


def pruefsumme(x) -> str:
    h = hashlib.sha256()
    _speise(h, x)
    return h.hexdigest()


def antwort_pruefsumme(status: int, body: bytes) -> list:
    """[Status, Pruefsumme]; kein JSON: `r` + SHA-256 der Rohbytes, `-1` bei Verbindungsfehler."""
    if status == -1:
        return [-1, "verbindungsfehler"]
    try:
        return [status, pruefsumme(json.loads(body))]
    except ValueError:
        return [status, "r" + hashlib.sha256(body).hexdigest()]


def fall_schluessel(fall_id: str) -> str:
    return hashlib.sha256(fall_id.encode()).hexdigest()[:16]


def selbsttest() -> None:
    # Gleiche Gestalt, gleiche Summe; Typ und Reihenfolge der Schluessel zaehlen.
    assert pruefsumme({"a": 1, "b": 2}) == pruefsumme({"b": 2, "a": 1}), "Schluesselreihenfolge ist egal"
    assert pruefsumme(1) == pruefsumme(1.0), "ganzzahliger Gleitwert gleich Ganzzahl"
    assert pruefsumme(-0.0) == pruefsumme(0), "-0.0 ist 0"
    assert pruefsumme(True) != pruefsumme(1), "bool ist keine Zahl"
    assert pruefsumme(2.5) != pruefsumme(2), "Gleitwert mit Nachkomma ist keine Ganzzahl"
    assert pruefsumme([1, 2]) != pruefsumme([2, 1]), "Listenreihenfolge zaehlt"
    assert pruefsumme("ab") != pruefsumme(["a", "b"]), "Zeichenkette ist keine Liste"
    assert pruefsumme({"a": ["b"]}) != pruefsumme({"a": "b"}), "Verschachtelung zaehlt"
    assert pruefsumme("ä") != pruefsumme("a"), "UTF-8 geht ein"
    assert antwort_pruefsumme(404, b"kein json")[1].startswith("r")
    assert antwort_pruefsumme(-1, b"") == [-1, "verbindungsfehler"]
    assert len(fall_schluessel("x")) == 16 and fall_schluessel("x") != fall_schluessel("y")
    try:
        pruefsumme(2 ** 70)
    except ValueError:
        pass
    else:
        raise AssertionError("Ganzzahl ausserhalb von i64/u64 muss abbrechen")
    assert pruefsumme(VEKTOR) == VEKTOR_SHA256, f"Known-Answer-Vektor weicht ab: {pruefsumme(VEKTOR)}"
    print("selbsttest ok")


def eingabe_daten(pfad: Path) -> dict:
    roh = pfad.read_bytes()
    events = json.loads(roh).get("events", [])
    ids = [e.get("event_id") for e in events if isinstance(e, dict)]
    return {"eingabe": hashlib.sha256(roh).hexdigest(), "ereignisse": len(events), "event_ids": pruefsumme(ids)}


def aufzeichnen(quelle: Path, ausgabe: Path, ziel: Path, max_mb: int) -> int:
    quelle = quelle.resolve()
    if not (quelle / "faelle").is_dir():
        print("Abbruch: Quelle hat kein Verzeichnis faelle/.", file=sys.stderr)
        return 2
    if ziel.resolve() == quelle or quelle in ziel.resolve().parents or ziel.resolve() in quelle.parents:
        print("Abbruch: Ziel und Quelle duerfen sich nicht enthalten.", file=sys.stderr)
        return 2
    kb = R.groesse_kb(quelle)
    print(f"Quelle: {kb / 1024:.0f} MB (Grenze {max_mb} MB)")
    if kb > max_mb * 1024:
        print("Stopp: Quelle groesser als die Grenze, nichts kopiert.")
        return 3
    vorher = R.fingerprint(quelle)
    if ziel.exists():
        R.raeume(ziel)
    ziel.mkdir(parents=True)
    (ziel / R.MARKE).write_text("von echtakten_aufzeichnen.py angelegt, darf geloescht werden\n")
    proc = None
    faelle: dict[str, dict] = {}
    n_ungueltig = 0
    status_zaehler: collections.Counter = collections.Counter()
    try:
        R.kopiere(quelle, ziel / "python")
        proc, basis = R.starte([sys.executable, "-u", str(R.REPO / "produkt/haut/server.py"), "0"],
                               ziel / "python", ziel / "python.log")
        for datei in sorted((ziel / "python" / "faelle").glob("*.json")):
            fid = datei.stem
            if not R.ID.match(fid):
                n_ungueltig += 1
                continue
            eintrag = eingabe_daten(datei)
            routen: dict[str, list] = {}
            fragen = None
            for route in R.FALL_ROUTEN:
                status, body = R.hole(basis, f"/fall/{fid}/{route}")
                routen[route] = antwort_pruefsumme(status, body)
                status_zaehler[(route, status)] += 1
                if route == "fragen" and status == 200:
                    try:
                        fragen = [q.get("feld_id") for q in json.loads(body).get("fragen", []) if isinstance(q, dict)]
                    except ValueError:
                        fragen = None
            felder = [f for f in (fragen or []) if isinstance(f, str) and R.FID.match(f)][:R.FELDER_JE_FALL]
            for i, feld in enumerate(felder):
                for route in R.FELD_ROUTEN:
                    status, body = R.hole(basis, f"/fall/{fid}/feld/{feld}/{route}")
                    routen[f"feld/{route}/{i}"] = antwort_pruefsumme(status, body)
                    status_zaehler[(f"feld/{route}", status)] += 1
            eintrag["routen"] = routen
            faelle[fall_schluessel(fid)] = eintrag
    finally:
        R.stoppe(proc)
        R.raeume(ziel)
    nachher = R.fingerprint(quelle)
    if vorher != nachher:
        print("FEHLER: die Quelle hat sich waehrend des Laufs veraendert (Dateien, Bytes oder mtime).", file=sys.stderr)
        return 1
    commit = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=R.REPO, capture_output=True, text=True).stdout.strip()
    aufz = {
        "format": FORMAT,
        "datum": datetime.datetime.now().isoformat(timespec="seconds"),
        "commit": commit,
        "vektor": VEKTOR_SHA256,
        "felder_je_fall": R.FELDER_JE_FALL,
        "n_faelle": len(faelle),
        "n_uebersprungen": n_ungueltig,
        "status": {f"{r} {s}": n for (r, s), n in sorted(status_zaehler.items())},
        "faelle": faelle,
    }
    tag = ausgabe / datetime.date.today().isoformat()
    ausgabe.mkdir(parents=True, exist_ok=True)
    os.chmod(ausgabe, 0o700)
    tag.mkdir(exist_ok=True)
    os.chmod(tag, 0o700)
    datei = tag / "aufzeichnung.json"
    fd = os.open(datei, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        json.dump(aufz, f, sort_keys=True, indent=1)
    n_abfragen = sum(len(e["routen"]) for e in faelle.values())
    print(f"Faelle: {len(faelle)}, uebersprungen: {n_ungueltig}, Abfragen: {n_abfragen}")
    print("Status je Route: " + ", ".join(f"{k}={v}" for k, v in aufz["status"].items()))
    print("Quelle unveraendert: ja")
    print(f"Aufzeichnung: {datei}")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("befehl", nargs="?", choices=["aufzeichnen"])
    ap.add_argument("--selbsttest", action="store_true")
    ap.add_argument("--quelle", type=Path, default=R.QUELLE)
    ap.add_argument("--ausgabe", type=Path, default=AUSGABE)
    ap.add_argument("--ziel", type=Path, default=ZIEL)
    ap.add_argument("--max-mb", type=int, default=2048)
    a = ap.parse_args()
    if a.selbsttest:
        selbsttest()
        return 0
    if a.befehl != "aufzeichnen":
        ap.print_usage(sys.stderr)
        return 2
    try:
        return aufzeichnen(a.quelle, a.ausgabe, a.ziel, a.max_mb)
    except Exception as e:  # noqa: BLE001 -- nur der Klassenname: eine Meldung koennte Daten enthalten
        print(f"Abbruch: {type(e).__name__}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
