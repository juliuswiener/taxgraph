#!/usr/bin/env python3
"""Prueft die Zahlen der Cutover-Doku (README.md, CLAUDE.md, REWRITE_PLAN.md) gegen ihre Quelle.

    python3 tools/docs_zahlen_pruefen.py                 # Tabelle `Zahl | Doc:Zeile | Kommando | Soll | Ist | Status`
    python3 tools/docs_zahlen_pruefen.py --selbsttest    # prueft das Skript selbst, ohne die Doku
    python3 tools/docs_zahlen_pruefen.py --gegenprobe    # aendert je Zeile die Doc-Zahl im Speicher: jede Zeile muss rot werden

Je Zeile: der Anker (Regex mit Gruppen) sucht die Zahl im Doc-Text und ergibt Soll und Doc:Zeile. Ein Kommando oder eine
Quelldatei liefert den Ist-Wert. Status: OK (Soll = Ist), ABWEICHUNG (Ist anders, oder der Anker trifft nicht genau eine
Zeile: der Doc-Text wurde geaendert), NUR ENDTOR (nur ein schwerer Lauf liefert die Zahl, wird nicht ausgefuehrt).
Quellen: Baum (git-Stand dieser Arbeitskopie), Logs der Trockenlaeufe und des Endtors (log-endtor-A, log-endtor-B) unter ~/.cache/taxgraph-tmp/gate-final/, Berichte unter
~/.cache/taxgraph-tmp/berichte/, Vault-Notizen unter ~/00_projects/vault/audits/. Bei Logs, Berichten und Vault-Notizen heisst OK:
der Doc-Text gibt die genannte Quelle richtig wieder, nicht: die Zahl gilt auf dem heutigen Baum.
Leichte Kommandos nur: grep, wc, ls, git merge-base/cat-file/rev-parse/diff/grep, python3 tests/test_testmap_vollstaendig.py, python3 -c.
Exit 0: keine ABWEICHUNG; 1: mindestens eine; 2: Aufruffehler.
"""
from __future__ import annotations

import argparse
import glob
import json
import math
import os
import re
import shlex
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Optional

REPO = Path(__file__).resolve().parents[1]
HOME = Path.home()
T = HOME / ".cache" / "taxgraph-tmp"
LOG1 = T / "gate-final" / "log-trocken2"
LOG2 = T / "gate-final" / "log-1fdc6c0a"
BER = T / "berichte"
VAULT = HOME / "00_projects" / "vault" / "audits"
DOCS = ("README.md", "CLAUDE.md", "REWRITE_PLAN.md")
ERLAUBT = {"grep", "wc", "ls", "git", "python3"}
WORTE = {"zwei": "2", "drei": "3", "vier": "4", "fuenf": "5", "fünf": "5", "sechs": "6", "elf": "11"}
LEER = re.compile(r"[\s  ]")


@dataclass
class P:
    zahl: str                      # Beschriftung
    doc: str                       # README.md | CLAUDE.md | REWRITE_PLAN.md
    anker: str                     # Regex, Gruppen = Soll-Werte; trifft genau eine Zeile (oder erste=True)
    kommando: str = ""             # baum/abgeleitet: leichtes Kommando
    ist: str = r"(\d+)"            # Regex mit Gruppen auf der Kommando-Ausgabe
    datei: str = ""                # quelle: Datei, in der der Doc-Wert stehen muss
    muster: str = ""               # quelle: ERE mit {0},{1} (Soll-Gruppen), {0t} (mit Tausender-Leerzeichen)
    modus: str = "="               # = | quelle | bereich | rund | kleiner
    endtor: bool = False           # nur ein schwerer Lauf liefert die Zahl
    erste: bool = False            # erster Treffer genuegt (Hashes)
    lauf: Optional[Callable[[], str]] = None   # statt Kommando: reine Dateilesung


# ---------------------------------------------------------------- Hilfen


def norm(s: str) -> str:
    s = LEER.sub("", s)
    return WORTE.get(s.lower(), s)


def tausend(wert: str) -> str:
    """`6860` -> Regex fuer `6 860` (Leerzeichen, geschuetzt oder schmal)."""
    w = norm(wert)
    teile = []
    while len(w) > 3:
        teile.insert(0, w[-3:])
        w = w[:-3]
    teile.insert(0, w)
    return "[   ]?".join(re.escape(t) for t in teile)


def einsetzen(vorlage: str, gruppen: list[str]) -> str:
    """{0} Zahl (Leerzeichen und Zahlwort normalisiert), {0t} Zahl mit Tausender-Leerzeichen, {0r} Text unveraendert,
    {0d} Dezimalzahl mit Punkt statt Komma (so stehen sie in den Logs)."""
    def ersatz(m: re.Match) -> str:
        roh = gruppen[int(m.group(1))].strip()
        if m.group(2) == "r":
            return re.escape(roh)
        if m.group(2) == "d":
            return re.escape(norm(roh).replace(",", "."))
        wert = norm(roh) if re.fullmatch(r"[\d.,\s\u00a0\u202f]+|\w+", roh) and not re.search(r"[A-Za-z]-", roh) else roh
        if m.group(2) == "t":
            return tausend(wert)
        return re.escape(wert).replace("\\,", ",").replace("\\-", "-")
    return re.sub(r"\{(\d)([trd]?)\}", ersatz, vorlage)


def pfad(s: str) -> str:
    return s.replace("~/", str(HOME) + "/")


def fuehre_aus(kommando: str) -> str:
    argv = shlex.split(pfad(kommando))
    if argv[0] not in ERLAUBT:
        raise ValueError(f"Kommando nicht erlaubt: {argv[0]}")
    if argv[0] == "git" and argv[1] not in ("merge-base", "cat-file", "rev-parse", "diff", "grep"):
        raise ValueError(f"git-Unterbefehl nicht erlaubt: {argv[1]}")
    if argv[0] == "python3" and argv[1] not in ("tests/test_testmap_vollstaendig.py", "-c"):
        raise ValueError(f"python3-Aufruf nicht erlaubt: {argv[1]}")
    r = subprocess.run(argv, cwd=REPO, capture_output=True, text=True, timeout=60)
    return f"{r.stdout}{r.stderr}rc={r.returncode}\n"


def datei_text(rel: str) -> str:
    return (REPO / rel).read_text(encoding="utf-8")


def suche_doc(p: P, texte: dict[str, str]) -> tuple[Optional[int], list[str], str]:
    zeilen = texte[p.doc].splitlines()
    treffer = [(i + 1, re.search(p.anker, z)) for i, z in enumerate(zeilen)]
    treffer = [(i, m) for i, m in treffer if m]
    if not treffer:
        return None, [], "ANKER FEHLT"
    if len(treffer) > 1 and not p.erste:
        return treffer[0][0], [], f"ANKER MEHRDEUTIG ({len(treffer)} Zeilen)"
    i, m = treffer[0]
    return i, list(m.groups()), ""


def vergleiche(modus: str, soll: list[str], ist: list[str]) -> bool:
    def zahl(s: str) -> float:
        return float(norm(s).replace(",", "."))
    try:
        if modus == "bereich":
            lo, hi = (int(x) for x in norm(soll[0]).split("-"))
            return lo <= int(norm(ist[0])) <= hi
        if modus == "da":
            return bool(ist) and bool(ist[0])
        if modus == "rund1":
            return round(zahl(ist[0]), 1) == zahl(soll[0])
        if modus == "kleiner":
            return zahl(ist[0]) < zahl(soll[0])
        if modus == "rund":
            return abs(zahl(ist[0]) - zahl(soll[0])) <= 0.05 * zahl(soll[0])
        return [norm(x) for x in soll] == [norm(x) for x in ist] or all(
            math.isclose(zahl(a), zahl(b)) for a, b in zip(soll, ist)) and len(soll) == len(ist)
    except (ValueError, IndexError):
        return False


def pruefe(p: P, texte: dict[str, str]) -> tuple[str, str, str, str, str]:
    """(Doc:Zeile, Kommando, Soll, Ist, Status)"""
    zeile, soll, fehler = suche_doc(p, texte)
    ort = f"{p.doc}:{zeile}" if zeile else p.doc
    if fehler:
        return ort, p.kommando or p.muster, "-", "-", fehler
    soll_s = " / ".join(norm(x) for x in soll)
    if p.endtor:
        return ort, p.kommando, soll_s, "-", "NUR ENDTOR"
    try:
        if p.muster:
            muster = einsetzen(p.muster, soll)
            kom = f'grep -c -E "{muster}" {p.datei}'
            ausgabe = fuehre_aus(f'grep -c -E {shlex.quote(muster)} {shlex.quote(p.datei)}')
            n = re.search(r"^(\d+)$", ausgabe, re.M)
            if not n and "rc=2" in ausgabe:
                return ort, kom, soll_s, "Quelle fehlt", "ABWEICHUNG"
            ist_n = int(n.group(1)) if n else 0
            return ort, kom, soll_s, f"{ist_n} Treffer", "OK" if ist_n >= 1 else "ABWEICHUNG"
        kom = einsetzen(p.kommando, soll)
        ausgabe = p.lauf() if p.lauf else fuehre_aus(kom)
        m = re.search(p.ist, ausgabe, re.M)
        if not m:
            return ort, kom, soll_s, "kein Treffer", "ABWEICHUNG"
        ist = list(m.groups())
        return ort, kom, soll_s, " / ".join(norm(x) for x in ist), "OK" if vergleiche(p.modus, soll, ist) else "ABWEICHUNG"
    except (OSError, ValueError, subprocess.SubprocessError) as e:
        return ort, p.kommando or p.muster, soll_s, f"Fehler: {e}", "ABWEICHUNG"


# ---------------------------------------------------------------- Tabelle


def lies_log(rel: Path, regex: str) -> Callable[[], str]:
    return lambda: "\n".join(re.findall(regex, rel.read_text(encoding="utf-8"), re.M))


def q(zahl, doc, anker, datei, muster, **kw) -> P:
    """Quelle: Datei muss das Doc-Muster enthalten (grep -c >= 1)."""
    return P(zahl, doc, anker, datei=str(datei), muster=muster, modus="quelle", **kw)


def b(zahl, doc, anker, kommando, ist=r"(\d+)", **kw) -> P:
    return P(zahl, doc, anker, kommando=kommando, ist=ist, **kw)


R, RD = "REWRITE_PLAN.md", "README.md"
PV = VAULT / "parity-voll-stufe-1-und-2-2026-10-04.md"
GG = VAULT / "g-gegenproben-15-von-15-rot-und-solz-konstante-2026-10-04.md"
WF = VAULT / "wertwache-fixture-ohne-orakel-2026-10-04.md"
UG = VAULT / "ueberlauf-guard-am-uebersetzer-2026-10-04.md"
UW = VAULT / "ueberlauf-waechter-text-2026-10-04.md"
UC = VAULT / "ueberlauf-casts-ueber-http-keine-stille-falschzahl-2026-10-04.md"
SW = BER / "wertwache-sweep.md"
RE_, RA = BER / "rauchprobe-echtdaten.md", BER / "rauchprobe-abweichungen.md"
CB = BER / "cutover-bau.md"
ZF1, ZF2 = LOG1 / "zusammenfassung.txt", LOG2 / "zusammenfassung.txt"
DEKL = "rust/elster/src/deklaration.rs"
APIT = "rust/parity/tests/api_http_paritaet.rs"
FIXTURE = "rust/fixtures/wertwache_orakel.json"
ARCHIV = str(T / "cutover-smoke" / "backups" / "*.tar.gz")


def fixture_zahlen() -> dict:
    d = json.loads(datei_text(FIXTURE))
    return {"faelle": len(d["faelle"]), "funktionen": len({c["fn"] for c in d["faelle"]})}


def zeile(label, doc, anker, datei, muster, **kw) -> P:
    """Fundstelle datei:Zeile: das Muster steht in der angegebenen Zeile (Bereich a-b erlaubt)."""
    return P(label, doc, anker, kommando=f'grep -n -m1 -E "{muster}" {datei}', ist=r"^(\d+):", modus="bereich" if "-" in anker else "=", **kw)


def fundstellen() -> list[P]:
    rows = []
    for label, doc, anker, datei, muster in [
        ("einreichen.rs Kopf", R, r"\(`rust/api/src/einreichen\.rs:(1-8)`, `produkt", "rust/api/src/einreichen.rs", "ERIC_VALIDIERE"),
        ("api.py einreichen", R, r"api\.py:(685-691)", "produkt/haut/api.py", "^def einreichen"),
        ("store.rs veranlagungszeitraum", R, r"store\.rs:(412-414)", "rust/store/src/store.rs", "pub fn veranlagungszeitraum"),
        ("auth lib.rs Geheimnis", R, r"auth/src/lib\.rs:(112-113)", "rust/auth/src/lib.rs", "zufaellig je Start"),
        ("shim.c TG_AUS", R, r"csrc/shim\.c:(50)", "rust/catala-sys/csrc/shim.c", "^#define TG_AUS"),
        ("lib.rs Ausgabe::cent", R, r"src/lib\.rs:(118)", "rust/catala-sys/src/lib.rs", "pub fn cent"),
        ("Cargo.toml overflow-checks (Waechter)", R, r"`rust/Cargo\.toml:(68)`\), und der Test", "rust/Cargo.toml", "^overflow-checks = true"),
        ("Cargo.toml overflow-checks (Betriebsfolge)", R, r"`rust/Cargo\.toml:(68)`\);", "rust/Cargo.toml", "^overflow-checks = true"),
        ("Makefile --release", R, r"`Makefile:(113)`", "Makefile", "Ein Bau mit `--release`"),
        ("Test 1h/1i Block", R, r"^\s*`api_http_paritaet\.rs:(6027)`", APIT, "// 1h/1i"),
        ("Test dokumentierte_abweichungen", R, r"beginnt bei Zeile (5848)", APIT, "fn dokumentierte_abweichungen"),
        ("Test 1g Kommentar", R, r"api_http_paritaet\.rs:(5964)", APIT, "1g: Scheibe `gesamt`, EIN Betrag"),
        ("Test 1g Test", R, r"Test ab Zeile (6309)", APIT, "// 1g: Summen des Gesamt-Scopes"),
        ("deklaration.rs p23_gewinn", R, r"elster/src/deklaration\.rs:(959-975)", DEKL, "checked_sub"),
        ("api deklaration.rs Ueberlauf", R, r"api/src/deklaration\.rs:(27-28)", "rust/api/src/deklaration.rs", "matches!\\(e, DeklarationsFehler::Ueberlauf"),
        ("deklaration.rs wrapping_add", R, r"`deklaration\.rs:(936)`", DEKL, "ponytail: `wrapping_add`"),
    ]:
        rows.append(zeile(label, doc, anker, datei, muster))
    return rows


def hashes() -> list[P]:
    rows = []
    for h in ("f81dba31", "5f346eb5", "23003002", "c9d13e6f", "1fdc6c0a", "1805712b", "7babaac8", "eebe4578", "7d08e023",
              "6efc1c72", "81892228", "b7eb0c01", "0197bf76", "b6516035", "057b7ec3", "7cd5e048", "88ee0bf", "904f6215"):
        rows.append(P(f"Commit {h}", R, rf"`({h})`", kommando=f"git cat-file -t {h}", ist=r"^(commit)", erste=True,
                      modus="da"))
    return rows


def tabelle() -> list[P]:
    rows: list[P] = []
    # --- Baum
    rows += [
        P("Parity-Suiten", R, r"→ (\d+); `make golden`", lauf=lambda: str(len(glob.glob(str(REPO / "rust/parity/tests/*_paritaet.rs")))),
          kommando="ls rust/parity/tests/*_paritaet.rs | wc -l"),
        b("Golden-Faelle (Pin im Test)", R, r"Vergleichssuiten und die (\d+) Golden-Fälle", 'grep -n "^N_FAELLE = " tests/test_golden_fallzahl.py', ist=r"= (\d+)"),
        P("Suiten mit Fallzahl-Schalter", R, r"jede der (\d+) Suiten mit Fallzahl-Schalter", kommando='grep -l "fallzahl::" rust/parity/tests/*_paritaet.rs | wc -l',
          lauf=lambda: str(sum("fallzahl::" in Path(f).read_text(encoding="utf-8") for f in glob.glob(str(REPO / "rust/parity/tests/*_paritaet.rs"))))),
        P("Suiten ohne Schalter", R, r"für die (fünf) Suiten ohne Schalter", kommando='grep -L "fallzahl::" rust/parity/tests/*_paritaet.rs | wc -l',
          lauf=lambda: str(sum("fallzahl::" not in Path(f).read_text(encoding="utf-8") for f in glob.glob(str(REPO / "rust/parity/tests/*_paritaet.rs"))))),
        P("Fixture-Faelle", R, r"das Fixture hat jetzt (\d+) Fälle", kommando=f"python3 -c len(json {FIXTURE})", lauf=lambda: str(fixture_zahlen()["faelle"])),
        P("Fixture-Funktionen (Doc: 11)", R, r"Python-Lauf, (11) Funktionen\)", kommando=f"python3 -c distinct fn {FIXTURE}", lauf=lambda: str(fixture_zahlen()["funktionen"])),
        P("Fixture-Funktionen (Doc: elf)", R, r"Rümpfe der (elf) Funktionen", kommando=f"python3 -c distinct fn {FIXTURE}", lauf=lambda: str(fixture_zahlen()["funktionen"])),
        P("Fixture-Funktionen (Doc: ausserhalb der 11)", R, r"außerhalb der (11) \(", kommando=f"python3 -c distinct fn {FIXTURE}", lauf=lambda: str(fixture_zahlen()["funktionen"])),
        P("Fixture-Funktionen (F6)", R, r"für (11) Funktionen liegen sie", kommando=f"python3 -c distinct fn {FIXTURE}", lauf=lambda: str(fixture_zahlen()["funktionen"])),
        b("Uebersetzer-Funktionen", R, r"12 der (26) Übersetzer-Funktionen", 'grep -c -E "^(int|long|void) tg_" rust/catala-sys/csrc/shim.c'),
        b("[profile.release] Zeilen", R, r"profile\.release rust/Cargo\.toml` → (\d+)\)", "grep -c profile.release rust/Cargo.toml"),
        b("opt-level Cargo.toml", R, r"dev-Profil \(opt-level (\d+)\)", 'grep -n "^opt-level" rust/Cargo.toml', ist=r"= (\d+)"),
        b("opt-level README", RD, r"dev-Profil \(opt-level (\d+)\)", 'grep -n "^opt-level" rust/Cargo.toml', ist=r"= (\d+)"),
        b("Solz-Faktor 119 im Code", R, r"Solz-Faktor 118 statt (119)", 'grep -n -o -m1 "119" rust/engine/src/zugriff/teil2/solz.rs', ist=r"^\d+:(\d+)"),
        b("Veranlagungsjahr unten", R, r"Jahr außerhalb (2024)\.\.2026", 'grep -n "Self::Vz2024 => " rust/domain/src/vz.rs', ist=r"=> (\d+)"),
        b("Veranlagungsjahr oben", R, r"Jahr außerhalb 2024\.\.(2026)", 'grep -n "Self::Vz2026 => " rust/domain/src/vz.rs', ist=r"=> (\d+)"),
        b("lesende Routen", R, r"192 Fälle, (8) lesende Routen", r"grep -c -E '^(FALL|FELD)_ROUTEN' tools/parity/rauchprobe_echtdaten.py", lauf=lambda: str(
            len(re.findall(r'"[a-z]+"', "".join(z for z in datei_text("tools/parity/rauchprobe_echtdaten.py").splitlines() if z.startswith(("FALL_ROUTEN", "FELD_ROUTEN"))))))),
        b("SERVE_PORT Plan", R, r"`SERVE_PORT` \(Standard (\d+)\)", 'grep -n "^SERVE_PORT" Makefile', ist=r"\?= (\d+)"),
        b("SERVE_PORT README", RD, r"127\.0\.0\.1:(\d+)", 'grep -n "^SERVE_PORT" Makefile', ist=r"\?= (\d+)"),
        b("Bindung Plan", R, r"bindet nur (127\.0\.0\.1)", 'grep -n "const HOST" rust/api/src/main.rs', ist=r'"([0-9.]+)"'),
        b("Bindung README", RD, r"bindet nur (127\.0\.0\.1)", 'grep -n "const HOST" rust/api/src/main.rs', ist=r'"([0-9.]+)"'),
        b("TESTMAP ohne Zeile (heute)", R, r"TESTMAP 451 Dateien, (0) ohne Zeile", "python3 tests/test_testmap_vollstaendig.py", ist=r"(\d+) ohne Zeile"),
        q("Betrag 9223372036854775800 im Test", R, r"Bruttolohn von (9223372036854775800) ct", APIT, r"{0}"),
        q("Vorzeichenkipper im Code", R, r"\(\+(446 744 073 709 551 616) ct", DEKL, "446744073709551616"),
        q("3,7e17 im Code", R, r"höchstens (3),7·10\^17", DEKL, "<= 3,7e17"),
        b("HTTP 422 Ueberlauf (Code)", R, r"in Rust ein (422) \(", 'grep -n "^/// 422" rust/api/src/stand.rs', ist=r"/// (\d+) "),
        b("HTTP 409 Guard (Code)", R, r"jede § 23-Eingabe vorher mit (409)", 'grep -n "Antwort 409" rust/api/src/deklaration.rs', ist=r"Antwort (\d+)"),
        b("HTTP 403 Besitzer (Code)", R, r"Konto gesperrt \((403), wie in Python\)", 'grep -n "Fall ohne user_id" produkt/haut/api.py', ist=r"user_id → (\d+)"),
        b("HTTP 401 ohne Token (Code)", R, r"Anfrage ohne Token ergibt (401)", 'grep -n "kein Auth-Kontext" produkt/haut/api.py', ist=r"Auth-Kontext → (\d+)"),
        b("HTTP 403 README (Code)", RD, r"Konto gesperrt \((403)\)", 'grep -n "Fall ohne user_id" produkt/haut/api.py', ist=r"user_id → (\d+)"),
        b("Commit eebe4578 ist in HEAD", R, r"`(eebe4578)` ist in main", "git merge-base --is-ancestor eebe4578 HEAD", ist=r"rc=(0)", modus="da"),
    ]
    rows += hashes() + fundstellen()
    return rows + tabelle_quellen()


def tabelle_quellen() -> list[P]:
    rows: list[P] = []
    # --- Vault parity-voll (a), (c)
    rows += [
        q("(a) Tests je Suite", R, r"(\d+) passed / 0 failed, Summe", PV, r"{0} passed"),
        q("(a) Summe s", R, r"0 failed, Summe (\d+) s", PV, r"{0} s Summe"),
        q("(a) abgeleitet min", R, r"abgeleitet etwa (49,7) min", PV, r"{0} min"),
        q("(a) 2858 s im Doc-Zusatz", R, r"min: (2858) s \+", PV, r"{0} s"),
        q("(a) 126,5 s", R, r"\+ (126,5) s für die fünf", PV, r"{0} s"),
        q("(a) 0,2 s", R, r"Schalter \+ (0,2) s\)", PV, r"\+ {0} s"),
        q("(a) PARITY_N", R, r"`PARITY_N=(10000)`, jede", PV, r"PARITY_N={0}"),
        q("(c) passed", R, r"(1580) passed / 0 failed", PV, r"{0} passed"),
        q("(c) ignored", R, r"1580 passed / 0 failed / (21) ignored", PV, r"{0} ignored"),
        q("(c) Binaries", R, r"(120) Binaries", PV, r"{0} Binaries"),
        q("(c) Sekunden mit Bau", R, r"Binaries, (782) s mit Bau", PV, r"{0} s mit Bau"),
        q("Baum 23003002", R, r"Baum `(23003002)`", PV, r"{0}"),
    ]
    # --- (b)
    rows += [
        q("(b) Gegenproben", R, r"(15) von 15 \(N-G1", GG, r"{0} von 15"),
        q("(b) N-G1 bis N-G6", R, r"(N-G1 bis N-G6)", GG, r"{0}"),
        q("(b) A-G1 bis A-G9", R, r"(A-G1 bis A-G9)", GG, r"{0}"),
        q("(b) Solz 118", R, r"Solz-Faktor (118) statt 119", GG, r"Konstante 119 durch {0} ersetzt"),
        q("(b) 18 von 23", R, r"(18) von (23) Mutanten", WF, r"{0} von {1}"),
        q("(b) V1 bis V23", R, r"V(1) bis V(23) wie erwartet", WF, r"V{0}–V{1}"),
        q("(b) V15 bis V17", R, r"V(15) bis V(17) jetzt rot", WF, r"V{0}, V16, V{1} jetzt rot"),
        q("(b) drei Gitter-Luecken", R, r"Die (drei) Gitter-Lücken", WF, r"{0r} Nachbarstellen"),
        q("(b) V6", R, r"(V6) \(gleichwertig\)", WF, r"{0} \(gleichwertig\)"),
        q("(b) V23 gewst", R, r"(V23) \(`gewst`", WF, r"{0} \(`gewst`"),
        q("(b) 249 Mutanten", R, r"(249) Operator-Mutanten, 227", SW, r"gesamt \(Operator-Sweep\) \| \*\*{0}\*\*"),
        q("(b) 227 rot", R, r"249 Operator-Mutanten, (227) rot", SW, r"\| rot \| \*\*{0}\*\*"),
        q("(b) 21 gruen", R, r"227 rot, (21) grün", SW, r"Ueberlebende \| \*\*{0}\*\*"),
        q("(b) 1 ohne Kompilat", R, r"21 grün, (1) ohne Kompilat", SW, r"davon {0} ohne Kompilat"),
        q("(b) vorher 83 gruen", R, r"waren (83) grün", SW, r"{0} gruen"),
        q("(b) 62 Luecken", R, r"schloss (62) Lücken", SW, r"{0} Luecken"),
        q("(b) 249 des Workers (Instruktor)", R, r"die (249) Operator-Mutanten des Workers", WF, r"die {0} Operator-Mutanten"),
        q("(b) elf Funktionen im Vault", R, r"Rümpfe der (elf) Funktionen", WF, r"{0r} Fixture-Funktionen"),
    ]
    # --- Rauchprobe
    rows += [
        q("Bestand MB", R, r"Bestand (37) MB, 192", RE_, r"\({0} MB\)"),
        q("Faelle im Bestand", R, r"Bestand 37 MB, (192) Fälle", RE_, r"{0}"),
        q("Routen", R, r"192 Fälle, (8) lesende Routen", RE_, r"Routen: {0}"),
        q("Abfragen je Dienst", R, r"(2218) Abfragen je Dienst", RE_, r"Abfragen je Dienst: {0}"),
        q("gleich", R, r"\*\*(2202) gleich, 16 abweichend", RE_, r"gleich: {0}"),
        q("abweichend", R, r"2202 gleich, (16) abweichend", RE_, r"abweichend: {0}"),
        q("je 5-mal", R, r"je (5)-mal Rust 500 / Python 200", RE_, r"\| {0} \|"),
        q("1-mal anderer Text", R, r"`deklaration` (1)-mal ein anderer Text", RE_, r"\| {0} \|"),
        q("fuenf Faelle", R, r"an (fünf) Fällen mit einem Jahr", RA, r"{0}|5 Faelle"),
        q("Jahre 2099/-5/38 Neunen", R, r"\((2099, −5, eine\s*$|2099, −5, eine)", RA, r"2099, −5, 38 Neunen", erste=True),
        q("Rust 500 / Python 200", R, r"Rust (500) / Python (200)", RE_, r"Status {0}/{1}"),
        q("15 von 15 Abfragen", R, r"\((15) von (15) Abfragen gemessen", RA, r"= {0} Abfragen, Python {1} × 403"),
        q("39 von 192", R, r"haben (39) von (192)\s*$", RA, r"Von den {1} Fällen haben {0}"),
        q("Bestand 37 MB (Betriebsfolge 3)", R, r"bei (37) MB Bestand", RE_, r"\({0} MB\)"),
        q("Ereignisse im Rundlauf", R, r"Fall mit (46) Ereignissen", T / "cutover-smoke" / "rundlauf.out", r"Ereignisse: {0}"),
        q("HTTP 401 im Produktstart", R, r"Anfrage ohne Token ergibt (401)", CB, r"→ {0}"),
    ]
    # --- Ueberlauf-Regel / Waechter / p23
    rows += [
        q("fuenf Einzelpruefungen", R, r"an (fünf) Stellen einzeln geprüft", UG, r"{0r} einzelnen Stellen"),
        q("Guard-Mutanten rot", R, r"(23) von 25 eigenen", UG, r"= {0} rot"),
        q("Guard-Mutanten gesamt", R, r"23 von (25) eigenen", UG, r"{0} eigene Mutanten"),
        q("Guard gleichwertig", R, r"Mutanten rot, (2) gleichwertig", UG, r"dazu {0} Überlebende"),
        q("Funktionen ohne roten Test", R, r"(12) der 26 Übersetzer", UG, r"{0} der 26 Funktionen"),
        q("Uebersetzer-Funktionen im Vault", R, r"12 der (26) Übersetzer", UG, r"Alle {0} C-Funktionen"),
        q("Waechter Testfaelle", R, r": (84) Testfälle", UW, r"{0} Fälle"),
        q("Waechter Laeufe", R, r"mit (28) Läufen", UW, r"{0} Läufe"),
        q("Waechter Mutanten rot", R, r"Läufen \((22) Mutanten rot", UW, r"{0} Mutanten"),
        q("Waechter Kontrollen gruen", R, r"rot, (6) Kontrollen grün", UW, r"{0} Kontrollen"),
        q("p23 Vorzeichenkipper (Vault)", R, r"\(\+(446 744 073 709 551 616) ct", UC, r"{0t} ct"),
        q("p23 9e18 (Vault)", R, r"je (9)·10\^18 ct", UC, r"{0} · 10¹⁸"),
        q("p23 -1,8e19 (Vault)", R, r"statt −(1,8)·10\^19 ct", UC, r"−{0} · 10¹⁹"),
        q("p23 6 von 6", R, r"\(Worker: (6) von (6) Kombinationen", UC, r"{0} von {1} Kombinationen"),
        q("p23 12 von 12", R, r"Instruktor: (12) von (12) eigene", UC, r"{0} von {1} rot"),
        q("6860 Faelle", R, r"(6860) Fälle gegen die echten", UC, r"{0t} Fälle"),
    ]
    # --- Abdeckung der uebrigen Zahlen der hinzugefuegten Doc-Zeilen (Ergaenzung nach dem Abdeckungslauf)
    DEC = Path.home() / "00_projects" / "vault" / "decisions" / "betrag-ausserhalb-i64-rechnung-antwortet-422-statt-500.md"
    BEREIT = VAULT / "cutover-bereitschaft-rust-port-2026-10-03.md"
    HTTP = "rust/parity/tests/api_http_paritaet.rs"
    rows += [
        q("(a) 0 failed je Suite", R, r"141 passed / (0) failed, Summe", PV, r"141 passed, {0} failed"),
        q("(c) 0 failed Gesamtlauf", R, r"1580 passed / (0) failed / 21 ignored", PV, r"1580 passed, {0} failed"),
        q("(c) 21 ignorierte", R, r"die (21) ignorierten sind", PV, r"{0} ignorierten"),
        q("Solz 118 (Instruktor-Satz)", R, r"darunter der Solz-Faktor (118);", GG, r"Konstante 119 durch {0} ersetzt"),
        q("21 gruene belegt", R, r"die (21) grünen belegt", SW, r"Ueberlebende \| \*\*{0}\*\*"),
        q("Alle 16 liegen an", R, r"Alle (16) liegen an", RE_, r"abweichend: {0}"),
        q("16 Abweichungen sichtbar", R, r"dort wären die (16) Abweichungen", RE_, r"abweichend: {0}"),
        q("Jahr 2099 (1h)", R, r"1h \(Jahr (2099),", RA, r"{0} dreimal"),
        q("Jahr -5 (Mechanismus)", R, r"mit Jahr (−5) gilt derselbe", RA, r"{0} einmal"),
        q("1h beide 500", R, r"mit vollständiger Akte antworten beide (500)\)", RA, r"beide Dienste mit {0}\."),
        q("beide 403", R, r"beide Dienste antworten mit (403)$", RA, r"beide Dienste antworten dann mit {0}"),
        q("Python 200 (1c)", R, r"Python rechnet weiter und antwortet (200) \(Vault", DEC, r"antwortet {0}: gewollte Abweichung, Eintrag 1c"),
        q("1g Rust 422 / Python 200", R, r"in Rust (422), in Python (200); Kommentar", HTTP, r"Rust {0} auf `stand`, `fragen` und `ergebnis` .*, Python {1}: der Fehler"),
        q("API 422 (p23)", R, r"die API antwortet (422) \(`rust/api", "rust/api/src/deklaration.rs", r"ApiFehler::Status\({0}, ref t\)"),
        q("Summe hoechstens 3,7e17", R, r"Summe höchstens (3,7)·10\^17", "rust/elster/src/deklaration.rs", r"<= {0}e17"),
        q("0 stille Falschzahlen", R, r"^\s+(0) stille Falschzahlen über HTTP", UC, r"\*\*{0} stille Falschzahlen:"),
        q("ERiC-Skips mit .env", R, r"Hauptbaum\) sind es (0);", ZF2, r"^SKIPS unit gesamt=19 ERiC/Hersteller-ID={0} "),
        q("Test-ID 74931", R, r"Test-ID (74931) hilft", "tools/parity/e2e_faelle.py", r'TEST_HERSTELLER_ID = "{0}"'),
        q("ERiC-Dateien Anzahl", R, r"plus die (15) ERiC-Dateien", T / "gate-final" / "eric-dateien.log", r"^# {0} Testdateien"),
        q("Befunde 4 bis 7", R, r"Befunde (4) bis (7)\)", BEREIT, r"\(Befunde {0}–{1}\)"),
        q("Befund 7", R, r"dort Befund (7)\)", BEREIT, r"Python-Antworten \(Befund {0}\)"),
    ]
    # --- Betriebsfolgen: Kaltbau, Sicherung
    rows += [
        P("Kaltbau Crates", R, r"Zielordner: (123) Crates", kommando="grep -c Compiling cutover-smoke/cargo-build.log",
          lauf=lambda: str((T / "cutover-smoke" / "cargo-build.log").read_text(encoding="utf-8").count("Compiling "))),
        P("Kaltbau Sekunden", R, r"123 Crates und (40) s laut", kommando="grep -o 'in [0-9.]*s' cutover-smoke/cargo-build.log", ist=r"in (\d+)\.\d+s",
          lauf=lambda: (T / "cutover-smoke" / "cargo-build.log").read_text(encoding="utf-8")),
        q("Kaltbau MB", R, r"(870) MB laut Bericht", CB, r"{0} MB"),
        P("Sicherungsarchiv Bytes", R, r"(29 455 291) B", kommando="wc -c cutover-smoke/backups/*.tar.gz",
          ist=r"(\d+)", lauf=lambda: str(os.path.getsize(glob.glob(ARCHIV)[0]))),
        P("Sicherung MB (Plan)", R, r"rund (29),5 MB je Start", kommando="wc -c cutover-smoke/backups/*.tar.gz / 1e6", ist=r"([0-9.]+)", modus="rund",
          lauf=lambda: f"{os.path.getsize(glob.glob(ARCHIV)[0]) / 1e6:.2f}"),
        P("Sicherung MB (README 1)", RD, r"rund (30) MB je Start, nach", kommando="wc -c cutover-smoke/backups/*.tar.gz / 1e6", ist=r"([0-9.]+)", modus="rund",
          lauf=lambda: f"{os.path.getsize(glob.glob(ARCHIV)[0]) / 1e6:.2f}"),
        P("Sicherung MB (README 2)", RD, r"waechst um rund (30) MB je Start", kommando="wc -c cutover-smoke/backups/*.tar.gz / 1e6", ist=r"([0-9.]+)", modus="rund",
          lauf=lambda: f"{os.path.getsize(glob.glob(ARCHIV)[0]) / 1e6:.2f}"),
        P("Erstbau Sekunden README", RD, r"gemessen (40) s", kommando="grep -o 'in [0-9.]*s' cutover-smoke/cargo-build.log", ist=r"in (\d+)\.\d+s",
          lauf=lambda: (T / "cutover-smoke" / "cargo-build.log").read_text(encoding="utf-8")),
    ]
    rows += trockenlaeufe()
    rows += endtor()
    return rows


def trockenlaeufe() -> list[P]:
    rows: list[P] = []

    def lg(zahl, anker, zf, muster, **kw):
        rows.append(q(zahl, R, anker, zf, muster, **kw))
    # Lauf 1 (b6516035, N=200)
    rows.append(P("L1 build-python s", R, r"build-python unter (1) s", kommando="grep SCHRITT 1 log-trocken2/zusammenfassung.txt", ist=r"^SCHRITT 1 build-python +rc=0 dauer=([0-9.]+)s",
                  modus="kleiner", lauf=lambda: ZF1.read_text(encoding="utf-8")))
    lg("L1 clippy s", r"clippy (23) s, 0 Warnungen", ZF1, r"^SCHRITT 2 clippy +rc=0 dauer={0}s warning=0")
    lg("L1 cargo s", r"`cargo test` (196) s, 1437", ZF1, r"^SCHRITT 3 cargo-test +rc=0 dauer={0}s")
    lg("L1 cargo passed", r"196 s, (1437) passed / 0 failed / 21 ignored", ZF1, r"passed={0} failed=0 ignored=21")
    lg("L1 TESTMAP", r"TESTMAP (451) Dateien", LOG1 / "4-testmap.log", r"{0} Testdateien, 0 ohne Zeile")
    lg("L1 unit s", r"`make unit` (436) s, 4362", ZF1, r"^SCHRITT 5 unit +rc=0 dauer={0}s")
    lg("L1 unit passed", r"436 s, (4362) passed / 87 skipped / 21 xfailed", ZF1, r"{0} passed, 87 skipped, 21 xfailed")
    lg("L1 unit skipped", r"4362 passed / (87) skipped", ZF1, r"4362 passed, {0} skipped")
    lg("L1 unit xfailed", r"87 skipped / (21) xfailed", ZF1, r"87 skipped, {0} xfailed")
    lg("L1 golden s", r"golden 135/135 in (1) s", ZF1, r"^SCHRITT 6 golden +rc=0 dauer={0}s")
    lg("L1 golden Faelle", r"golden (135)/135 in 1 s", ZF1, r"{0}/135 Faelle bestanden")
    lg("L1 ui-rust s", r"`make ui-rust` (236) s", ZF1, r"^SCHRITT 7 ui-rust +rc=0 dauer={0}s")
    lg("L1 ui-rust passed", r"236 s, (249) passed / 23 xfailed", ZF1, r"{0} passed, 23 xfailed")
    lg("L1 ui-rust xfailed", r"236 s, 249 passed / (23) xfailed", ZF1, r"249 passed, {0} xfailed")
    lg("L1 parity s", r"Parity (637) s, 192", ZF1, r"^SCHRITT 8 parity +rc=0 dauer={0}s")
    lg("L1 parity passed", r"637 s, (192) passed / 0 failed / 0 ignored in 25 Binaries", ZF1, r"passed={0} failed=0 ignored=0")
    lg("L1 parity Binaries", r"0 ignored in (25) Binaries\. Summe", ZF1, r"testresult_zeilen={0}")
    lg("L1 Summe s", r"Summe (1529) s \(25,5 min\)", ZF1, r"^GESAMT dauer={0}s")
    rows.append(P("L1 Summe min", R, r"Summe 1529 s \((25,5) min\)", kommando="GESAMT dauer / 60", ist=r"([0-9.]+)",
                  lauf=lambda: f"{_s(ZF1, r'^GESAMT dauer=(\d+)s') / 60:.1f}"))
    # Lauf 2 (1fdc6c0a, N=std)
    lg("L2 build-python s", r"warme Ziele: (0,2) s; clippy", ZF2, r"^SCHRITT 1 build-python +rc=0 dauer=0\.2s")
    lg("L2 clippy s", r"clippy (7,4) s, 0;", ZF2, r"^SCHRITT 2 clippy +rc=0 dauer=7\.4s warning=0")
    lg("L2 cargo s", r"`cargo test` (84,3) s, 1463", ZF2, r"^SCHRITT 3 cargo-test +rc=0 dauer=84\.3s")
    lg("L2 cargo passed", r"84,3 s, (1463) / 0 / 21", ZF2, r"passed={0} failed=0 ignored=21")
    lg("L2 cargo Zeilen", r"cargo test` 84,3 s, 1463 / 0 / (21);", ZF2, r"ignored={0} testresult")
    lg("L2 TESTMAP", r"TESTMAP (453) / 0", LOG2 / "4-testmap.log", r"{0} Testdateien, 0 ohne Zeile")
    lg("L2 unit s", r"`make unit` (308,7) s, 4429", ZF2, r"^SCHRITT 5 unit +rc=0 dauer=308\.7s")
    lg("L2 unit passed", r"308,7 s, (4429) passed / 19 skipped / 22 xfailed", ZF2, r"{0} passed, 19 skipped, 22 xfailed")
    lg("L2 unit skipped", r"4429 passed / (19) skipped", ZF2, r"4429 passed, {0} skipped")
    lg("L2 unit xfailed", r"4429 passed / 19 skipped / (22) xfailed", ZF2, r"19 skipped, {0} xfailed")
    lg("L2 golden Faelle", r"golden (135)/135; `make ui-rust`", ZF2, r"{0}/135 Faelle bestanden")
    lg("L2 ui-rust s", r"`make ui-rust` (132,9) s", ZF2, r"^SCHRITT 7 ui-rust +rc=0 dauer=132\.9s")
    lg("L2 ui-rust passed", r"132,9 s, (249) / 23", ZF2, r"{0} passed, 23 xfailed")
    lg("L2 ui-rust xfailed", r"132,9 s, 249 / (23);", ZF2, r"249 passed, {0} xfailed")
    lg("L2 parity s", r"Parity (645,2) s, 192", ZF2, r"^SCHRITT 8 parity +rc=0 dauer=645\.2s")
    lg("L2 parity passed", r"645,2 s, (192) / 0 / 0 in 25", ZF2, r"passed={0} failed=0 ignored=0")
    lg("L2 parity Binaries", r"0 / 0 in (25) Binaries; die drei", ZF2, r"testresult_zeilen={0}")
    rows.append(P("L2 drei Testdateien", R, r"die (drei) Testdateien, deren 4 Tests", kommando="grep -c 'tests/test_' im s9() von gate-final/run.sh",
                  lauf=lambda: str(re.search(r"^s9\(\) \{.*?^\s+(?:.*?)\}", (T / "gate-final" / "run.sh").read_text(encoding="utf-8"), re.S | re.M).group(0).count("tests/test_")
                                  if re.search(r"^s9\(\) \{.*?^\s+(?:.*?)\}", (T / "gate-final" / "run.sh").read_text(encoding="utf-8"), re.S | re.M) else "")))
    lg("L2 Korpus s", r"Korpus \(Kopie\) (6,8) s", ZF2, r"^SCHRITT 9 korpus-tests +rc=0 dauer=6\.8s")
    lg("L2 Korpus passed", r"6,8 s, (18) passed / 1 xfailed / 0 skipped", ZF2, r"{0} passed, 1 xfailed")
    lg("L2 Korpus xfailed", r"6,8 s, 18 passed / (1) xfailed / 0 skipped", ZF2, r"18 passed, {0} xfailed")
    lg("L2 Korpus skipped", r"6,8 s, 18 passed / 1 xfailed / (0) skipped", ZF2, r"^SKIPS korpus-tests gesamt={0} ")
    lg("L2 Summe s", r"Summe (1186) s \(19,8 min\)", ZF2, r"^GESAMT dauer={0}s")
    rows.append(P("L2 Summe min", R, r"Summe 1186 s \((19,8) min\)", kommando="GESAMT dauer / 60", ist=r"([0-9.]+)",
                  lauf=lambda: f"{_s(ZF2, r'^GESAMT dauer=(\d+)s') / 60:.1f}"))
    # Befunde der Laeufe
    skiplog = T / "gate-final" / "diag-skips" / "unit-rs-isoliert.log"

    def skips(regex: str) -> Callable[[], str]:
        def f() -> str:
            n = 0
            for z in skiplog.read_text(encoding="utf-8", errors="replace").splitlines():
                m = re.match(r"SKIPPED \[(\d+)\] \S+?:\d+: (.*)", z)
                if m and re.search(regex, m.group(2)):
                    n += int(m.group(1))
            return str(n)
        return f
    rows += [
        P("ERiC-Skips ohne .env", R, r"überspringt `make unit` (68) ERiC-Tests", kommando="Summe SKIPPED-Zeilen 'ERiC oder Hersteller-ID' in diag-skips/unit-rs-isoliert.log",
          lauf=skips(r"ERiC oder Hersteller-ID")),
        P("Skips leerer Bestand", R, r"dadurch skippen (4) Tests wegen leerem Bestand", kommando="Summe SKIPPED-Zeilen 'Kein Bestand|Korpus leer|Datenverzeichnis' in diag-skips/unit-rs-isoliert.log",
          lauf=skips(r"Kein Bestand|Korpus leer|Datenverzeichnis")),
        P("Skips leerer Bestand (Dateien-Satz)", R, r"deren (4) Tests bei", kommando="Summe SKIPPED-Zeilen 'Kein Bestand|Korpus leer|Datenverzeichnis' in diag-skips/unit-rs-isoliert.log",
          lauf=skips(r"Kein Bestand|Korpus leer|Datenverzeichnis")),
        q("xfail mit .env", R, r"Der eine xfail mehr \((22) statt 21\)", ZF2, r"{0} xfailed"),
        q("xfail ohne .env", R, r"Der eine xfail mehr \(22 statt (21)\)", ZF1, r"{0} xfailed"),
        q("xfail-Lauf ohne .env", R, r"mit `\.env` xfail \((21) xfailed \+ 1 skipped", T / "gate-final" / "xfail-ohne-env.log", r"1 skipped, {0} xfailed"),
        q("xfail-Lauf Skip", R, r"21 xfailed \+ (1) skipped bei", T / "gate-final" / "xfail-ohne-env.log", r"{0} skipped, 21 xfailed"),
        P("Node-IDs", R, r"bei denselben (22) Node-IDs", kommando="grep -c '^XFAIL' gate-final/log-1fdc6c0a/5-unit.xfail-namen.txt", lauf=lambda: str(
            sum(z.startswith("XFAIL") for z in (LOG2 / "5-unit.xfail-namen.txt").read_text(encoding="utf-8").splitlines()))),
        P("Waechter-Meldungen bei N=200", R, r"bei `N=200` meldet das Log (11) Stellen", kommando="grep -c 'PARITY_N=200: Abdeckungs-Waechter' log-trocken2/8-parity.log",
          lauf=lambda: str((LOG1 / "8-parity.log").read_text(encoding="utf-8").count("PARITY_N=200: Abdeckungs-Waechter"))),
        P("ERiC-Dateien s", R, r"15 ERiC-Dateien \((10,5) s,", kommando="grep -o 'in [0-9.]*s' gate-final/eric-dateien.log", ist=r"in ([0-9.]+)s", modus="rund1",
          lauf=lambda: (T / "gate-final" / "eric-dateien.log").read_text(encoding="utf-8")),
        P("Last Lauf 1 (Log-Kopf)", R, r"Basis `b6516035`.*Last laut Log-Kopf ([0-9,]+) / ([0-9,]+) / ([0-9,]+)", kommando="grep load log-trocken2/00-kopf.txt", ist=r"load=([0-9.]+) ([0-9.]+) ([0-9.]+)", lauf=lambda: (LOG1 / "00-kopf.txt").read_text(encoding="utf-8")),
        P("Last Lauf 2 (Log-Kopf)", R, r"Basis `1fdc6c0a`.*Last laut Log-Kopf ([0-9,]+) / ([0-9,]+) / ([0-9,]+)", kommando="grep load log-1fdc6c0a/00-kopf.txt", ist=r"load=([0-9.]+) ([0-9.]+) ([0-9.]+)", lauf=lambda: (LOG2 / "00-kopf.txt").read_text(encoding="utf-8")),
    ]
    rows += [
        P("Planwert Endtor s", R, r"Planwert (1600) s, aufgerundet", kommando="aufrunden(100, L1 GESAMT + L2 Korpus + ERiC + (L2 Parity - L1 Parity))", ist=r"(\d+)",
          lauf=lambda: str(math.ceil((_s(ZF1, r"^GESAMT dauer=(\d+)s") + _s(ZF2, r"^SCHRITT 9 .*dauer=([0-9.]+)s") + _s(T / "gate-final" / "eric-dateien.log", r"in ([0-9.]+)s")
                                      + _s(ZF2, r"^SCHRITT 8 .*dauer=([0-9.]+)s") - _s(ZF1, r"^SCHRITT 8 .*dauer=([0-9.]+)s")) / 100) * 100)),
        P("Planungslast 30", R, r"Zeitplan Endtor bei Last (30):", kommando="grep load log-trocken2/00-kopf.txt", ist=r"load=([0-9.]+)", modus="rund",
          lauf=lambda: (LOG1 / "00-kopf.txt").read_text(encoding="utf-8")),
        P("Planwert Endtor min", R, r"etwa (27) min \(Planwert", kommando="Planwert / 60 gerundet", ist=r"(\d+)", modus="=",
          lauf=lambda: str(round(1600 / 60))),
    ]
    return rows


def _s(datei: Path, regex: str) -> float:
    m = re.search(regex, datei.read_text(encoding="utf-8"), re.M)
    if not m:
        raise ValueError(f"{regex!r} nicht in {datei}")
    return float(m.group(1))


def endtor() -> list[P]:
    """Der Endtor auf 7cd5e048 (Lauf A: alle neun Schritte, N=std; Lauf B: nur Parity, N=10000), gelesen aus den Logs.
    Zeilen mit endtor=True (Zahl liefert nur ein schwerer Lauf, wird nicht ausgefuehrt) gibt es seitdem keine mehr."""
    rows: list[P] = []
    GF = T / "gate-final"
    ZEA, ZEB = GF / "log-endtor-A" / "zusammenfassung.txt", GF / "log-endtor-B" / "zusammenfassung.txt"

    def lg(zahl, anker, zf, muster, **kw):
        rows.append(q(zahl, R, anker, zf, muster, **kw))

    def dauer(zf: Path) -> str:
        return str(int(_s(zf, r"^GESAMT dauer=(\d+)s")))

    def kleiner(zahl, anker, zf, kommando):
        rows.append(P(zahl, R, anker, kommando=kommando, ist=r"(\d+)", modus="kleiner", lauf=lambda: dauer(zf)))

    def zaehle(datei: Path, regex: str) -> Callable[[], str]:
        return lambda: str(len(re.findall(regex, datei.read_text(encoding="utf-8", errors="replace"), re.M)))
    A, B = r"Lauf A \(alle neun.*", r"Lauf B \(nur Parity.*"
    # --- Kopf: Basis, Diff nach dem Endtor
    rows.append(P("Diff 7cd5e048..88ee0bf", R, r"→ (1) file changed, (35) insertions", kommando="git diff --shortstat 7cd5e048 88ee0bf",
                  ist=r"(\d+) file changed, (\d+) insertions"))
    lg("Status vor/nach Lauf A", r"`git status` (0) Zeilen vor und nach", ZEA, r"^status={0} Zeilen")
    lg("Status vor/nach Lauf B", r"`git status` (0) Zeilen vor und nach", ZEB, r"^status={0} Zeilen")
    # --- Lauf A
    lg("EA Start", A + r"Start (17:15:23), Ende", ZEA, r"^start=2026-10-04T{0}\+02:00")
    lg("EA Ende", A + r"Ende (17:32:29), Last", ZEA, r"^GESAMT dauer=[0-9]+s ende=2026-10-04T{0}\+02:00")
    rows.append(P("EA Last (Log-Kopf)", R, A + r"Last laut Log-Kopf ([0-9,]+) / ([0-9,]+) / ([0-9,]+)", kommando="grep load log-endtor-A/00-kopf.txt",
                  ist=r"load=([0-9.]+) ([0-9.]+) ([0-9.]+)", lauf=lambda: (GF / "log-endtor-A" / "00-kopf.txt").read_text(encoding="utf-8")))
    lg("EA build-python s", r"Lauf A.*build-python (0,1) s; clippy", ZEA, r"^SCHRITT 1 build-python +rc=0 dauer={0d}s")
    lg("EA clippy s", r"Lauf A.*clippy (4,4) s, 0 Warnungen", ZEA, r"^SCHRITT 2 clippy +rc=0 dauer={0d}s warning=0")
    lg("EA cargo s", r"`cargo test` (57,2) s, 1673", ZEA, r"^SCHRITT 3 cargo-test +rc=0 dauer={0d}s")
    lg("EA cargo passed", r"57,2 s, (1673) passed / 0 failed / 24 ignored", ZEA, r"passed={0} failed=0 ignored=24")
    lg("EA cargo ignored", r"1673 passed / 0 failed / (24) ignored in 131", ZEA, r"failed=0 ignored={0} testresult")
    lg("EA cargo Binaries", r"24 ignored in (131) Binaries", ZEA, r"testresult_zeilen={0} ")
    lg("EA TESTMAP", r"TESTMAP (480) Dateien, 0 ohne Zeile", GF / "log-endtor-A" / "4-testmap.log", r"{0} Testdateien, 0 ohne Zeile")
    lg("EA unit s", r"`make unit` (276,1) s, 4516", ZEA, r"^SCHRITT 5 unit +rc=0 dauer={0d}s")
    lg("EA unit passed", r"276,1 s, (4516) passed / 19 skipped / 22 xfailed", ZEA, r"{0} passed, 19 skipped, 22 xfailed")
    lg("EA unit skipped", r"4516 passed / (19) skipped / 22 xfailed", ZEA, r"4516 passed, {0} skipped")
    lg("EA unit xfailed", r"4516 passed / 19 skipped / (22) xfailed", ZEA, r"19 skipped, {0} xfailed")
    lg("EA golden s", r"golden 135/135 in (0,4) s", ZEA, r"^SCHRITT 6 golden +rc=0 dauer={0d}s")
    lg("EA golden Faelle", r"xfailed; golden (135)/135 in 0,4 s", ZEA, r"{0}/135 Faelle bestanden")
    lg("EA ui-rust s", r"`make ui-rust` (118,9) s, 249", ZEA, r"^SCHRITT 7 ui-rust +rc=0 dauer={0d}s")
    lg("EA ui-rust passed", r"118,9 s, (249) passed / 23 xfailed", ZEA, r"{0} passed, 23 xfailed")
    lg("EA ui-rust xfailed", r"118,9 s, 249 passed / (23) xfailed", ZEA, r"249 passed, {0} xfailed")
    lg("EA parity s", r"Parity (562,4) s, 192", ZEA, r"^SCHRITT 8 parity +rc=0 dauer={0d}s")
    lg("EA parity passed", r"562,4 s, (192) passed / 0 failed / 0 ignored in 25 Binaries;", ZEA, r"passed={0} failed=0 ignored=0")
    lg("EA parity Binaries", r"562,4 s, 192 passed / 0 failed / 0 ignored in (25) Binaries;", ZEA, r"testresult_zeilen={0} ")
    lg("EA korpus s", r"Korpus \(Kopie\) (6,0) s, 18", ZEA, r"^SCHRITT 9 korpus-tests +rc=0 dauer={0d}s")
    lg("EA korpus passed", r"6,0 s, (18) passed / 1 xfailed / 0 skipped\. Summe", ZEA, r"{0} passed, 1 xfailed")
    lg("EA korpus xfailed", r"6,0 s, 18 passed / (1) xfailed / 0 skipped\. Summe", ZEA, r"18 passed, {0} xfailed")
    lg("EA korpus skipped", r"6,0 s, 18 passed / 1 xfailed / (0) skipped\. Summe", ZEA, r"^SKIPS korpus-tests gesamt={0} ")
    lg("EA Summe s", r"Summe (1026) s \(17,1 min\)", ZEA, r"^GESAMT dauer={0}s")
    rows.append(P("EA Summe min", R, r"Summe 1026 s \((17,1) min\)", kommando="GESAMT dauer / 60 (Lauf A)", ist=r"([0-9.]+)",
                  lauf=lambda: f"{_s(ZEA, r'^GESAMT dauer=(\d+)s') / 60:.1f}"))
    kleiner("EA unter Limit", r"unter dem Limit von (2000) s und unter", ZEA, "GESAMT dauer Lauf A < Limit")
    kleiner("EA unter Planwert", r"unter dem Planwert (1600) s\. Nachprüfung", ZEA, "GESAMT dauer Lauf A < Planwert")
    kleiner("Zeitlimit Lauf A (Vorausschau-Satz)", r"Limit (2000) s \(Vorgabe des Instruktors, keine Messung\)", ZEA, "GESAMT dauer Lauf A < Limit")
    lg("EA Nachpruefung Worktree", r"Planwert 1600 s\. Nachprüfung im Log: Worktree (0) Zeilen", ZEA, r"^NACHPRUEFUNG worktree-status={0} Zeilen")
    lg("EA Nachpruefung Bestand", r"Planwert 1600 s\..*echter Bestand (0) Dateien", ZEA, r"^NACHPRUEFUNG echter-Bestand: {0} Dateien")
    lg("EA Nachpruefung Korpus", r"Planwert 1600 s\..*Korpus (0)\.$", ZEA, r"^NACHPRUEFUNG korpus: {0} Dateien")
    # --- Lauf A gegen Lauf 2 (Differenzen aus beiden Logs)

    def diff(regex_a: str, zf_a: Path, regex_b: str, zf_b: Path) -> Callable[[], str]:
        return lambda: str(int(_s(zf_a, regex_a) - _s(zf_b, regex_b)))
    rows.append(P("A-L2 cargo passed", R, r"`cargo test` \+(210) passed", kommando="passed (Lauf A) - passed (Lauf 2)", ist=r"(\d+)",
                  lauf=diff(r"^SCHRITT 3 .*passed=(\d+)", ZEA, r"^SCHRITT 3 .*passed=(\d+)", ZF2)))
    rows.append(P("A-L2 cargo ignored", R, r"und \+(3) ignored", kommando="ignored (Lauf A) - ignored (Lauf 2)", ist=r"(\d+)",
                  lauf=diff(r"^SCHRITT 3 .*ignored=(\d+)", ZEA, r"^SCHRITT 3 .*ignored=(\d+)", ZF2)))
    rows.append(P("A-L2 TESTMAP", R, r"TESTMAP \+(27) Dateien", kommando="Testdateien (Lauf A) - Testdateien (Lauf 2)", ist=r"(\d+)",
                  lauf=diff(r"(\d+) Testdateien", GF / "log-endtor-A" / "4-testmap.log", r"(\d+) Testdateien", LOG2 / "4-testmap.log")))
    rows.append(P("A-L2 unit passed", R, r"`make unit` \+(87) passed", kommando="passed (Lauf A) - passed (Lauf 2)", ist=r"(\d+)",
                  lauf=diff(r"^SCHRITT 5 unit .* (\d+) passed", ZEA, r"^SCHRITT 5 unit .* (\d+) passed", ZF2)))
    for n, anker in (("301", r"sie warten (301) s, 30 s"), ("30", r"sie warten 301 s, (30) s und"), ("61", r"und (61) s\)")):
        lg(f"Handtest wartet {n} s", anker, GF / "log-endtor-A" / "3-cargo-test.log", r"wartet {0} s: manuell mit --ignored")

    def werte(zf: Path) -> tuple:
        t = zf.read_text(encoding="utf-8")

        def z(regex: str):
            m = re.search(regex, t, re.M)
            return m.groups() if m else None
        return (z(r"^SCHRITT 5 unit .* (\d+) skipped, (\d+) xfailed"), z(r"^SCHRITT 6 golden .* (\d+)/135"),
                z(r"^SCHRITT 7 ui-rust .* (\d+) passed, (\d+) xfailed"), z(r"^SCHRITT 8 parity .* passed=(\d+) failed=(\d+) ignored=(\d+)"))

    def gleich() -> str:
        a, b = werte(ZEA), werte(ZF2)
        return "gleich" if None not in a and a == b else ""
    rows.append(P("A = Lauf 2: skipped, xfailed, golden, ui-rust, Parity", R, r"(skipped, xfailed, golden, `make ui-rust` und Parity) sind unverändert",
                  kommando="skipped/xfailed (unit), Faelle (golden), passed/xfailed (ui-rust), passed/failed/ignored (Parity): Lauf A gegen Lauf 2",
                  ist=r"(gleich)", modus="da", lauf=gleich))
    rows.append(P("A-L2 ignorierte Handtests", R, r"\+3 ignored \((drei) Handtests", kommando="ignored (Lauf A) - ignored (Lauf 2)", ist=r"(\d+)",
                  lauf=diff(r"^SCHRITT 3 .*ignored=(\d+)", ZEA, r"^SCHRITT 3 .*ignored=(\d+)", ZF2)))
    # --- Lauf B
    lg("(a) EB Binaries", r"über alle (25) Binaries \(Lauf B", ZEB, r"testresult_zeilen={0} ")
    lg("(a) EB passed", r"ist grün: (192) passed / 0 failed in 2644 s", ZEB, r"passed={0} failed=0")
    lg("(a) EB failed", r"192 passed / (0) failed in 2644 s", ZEB, r"passed=192 failed={0} ignored")
    lg("(a) EB s", r"192 passed / 0 failed in (2644) s", ZEB, r"^GESAMT dauer={0}s")
    lg("EB Start", B + r"Start (17:33:41), Ende", ZEB, r"^start=2026-10-04T{0}\+02:00")
    lg("EB Ende", B + r"Ende (18:17:45), Last", ZEB, r"^GESAMT dauer=[0-9]+s ende=2026-10-04T{0}\+02:00")
    rows.append(P("EB Last (Log-Kopf)", R, B + r"Last laut Log-Kopf ([0-9,]+) / ([0-9,]+) / ([0-9,]+)", kommando="grep load log-endtor-B/00-kopf.txt",
                  ist=r"load=([0-9.]+) ([0-9.]+) ([0-9.]+)", lauf=lambda: (GF / "log-endtor-B" / "00-kopf.txt").read_text(encoding="utf-8")))
    lg("EB PARITY_N", r"Lauf B \(nur Parity, `PARITY_N=(10000)`", ZEB, r"^N={0}$")
    lg("EB SCHRITTE", r"`SCHRITTE=(8)`, eigenes Ziel", ZEB, r"^schritte={0}$")
    lg("EB Korpus-Dateien", r"Korpus-Kopie mit (192) Dateien", ZEB, r"^korpus=.* \({0} Dateien\)")
    lg("EB parity s", r"Parity (2643,9) s, 192", ZEB, r"^SCHRITT 8 parity +rc=0 dauer={0d}s")
    lg("EB parity passed", r"2643,9 s, (192) passed / 0 failed / 0 ignored in 25 Binaries; Summe", ZEB, r"passed={0} failed=0 ignored=0")
    lg("EB parity Binaries", r"2643,9 s, 192 passed / 0 failed / 0 ignored in (25) Binaries; Summe", ZEB, r"testresult_zeilen={0} ")
    lg("EB Summe s", r"Summe (2644) s \(44,1 min\)", ZEB, r"^GESAMT dauer={0}s")
    rows.append(P("EB Summe min", R, r"Summe 2644 s \((44,1) min\)", kommando="GESAMT dauer / 60 (Lauf B)", ist=r"([0-9.]+)",
                  lauf=lambda: f"{_s(ZEB, r'^GESAMT dauer=(\d+)s') / 60:.1f}"))
    kleiner("EB unter Limit", r"unter dem Limit von (4500) s \(`gate-final/run.sh`", ZEB, "GESAMT dauer Lauf B < Limit")
    lg("EB Limit steht in run.sh", r"unter dem Limit von (4500) s \(`gate-final/run.sh`", GF / "run.sh", r"timeout -k 60 {0} bash")
    rows.append(P("EB 26 Meldungen N=10000", R, r"meldet (26)-mal „laeuft mit 10000 Faellen“", kommando="grep -c 'PARITY_N=10000: .* laeuft mit 10000 Faellen' log-endtor-B/8-parity.log",
                  ist=r"(\d+)", lauf=zaehle(GF / "log-endtor-B" / "8-parity.log", r"PARITY_N=10000: .* laeuft mit 10000 Faellen")))
    rows.append(P("EB 11 Waechter uebersprungen", R, r"und (11)-mal „Abdeckungs-Waechter", kommando="grep -c 'PARITY_N=10000: Abdeckungs-Waechter .* uebersprungen' log-endtor-B/8-parity.log",
                  ist=r"(\d+)", lauf=zaehle(GF / "log-endtor-B" / "8-parity.log", r"PARITY_N=10000: Abdeckungs-Waechter .* uebersprungen")))
    lg("EB Nachpruefung Worktree", r"gewollt\)\. Nachprüfung im Log: Worktree (0) Zeilen", ZEB, r"^NACHPRUEFUNG worktree-status={0} Zeilen")
    lg("EB Nachpruefung Bestand", r"gewollt\)\..*echter Bestand (0) Dateien", ZEB, r"^NACHPRUEFUNG echter-Bestand: {0} Dateien")
    lg("EB Nachpruefung Korpus", r"gewollt\)\..*Korpus (0)\.$", ZEB, r"^NACHPRUEFUNG korpus: {0} Dateien")
    # --- Tag
    rows.append(P("Tag python-standard-letzter", R, r"→ `([0-9a-f]{40})`, nicht gepusht", kommando="git rev-parse python-standard-letzter", ist=r"^([0-9a-f]{40})"))
    rows.append(P("Tag = Elterncommit von f81dba31", R, r"`git rev-parse f81dba31\^` → (derselbe Hash)\),", kommando="git rev-parse f81dba31^ gegen git rev-parse python-standard-letzter",
                  ist=r"(derselbe Hash)", modus="da", lauf=lambda: "derselbe Hash" if re.search(r"^([0-9a-f]{40})", fuehre_aus("git rev-parse f81dba31^"), re.M).group(1)
                  == re.search(r"^([0-9a-f]{40})", fuehre_aus("git rev-parse python-standard-letzter"), re.M).group(1) else ""))
    rows.append(P("904f6215 ohne make serve", R, r"`git grep -c '\^serve' 904f6215 -- Makefile` (findet nichts)\),", kommando="git grep -c ^serve 904f6215 -- Makefile",
                  ist=r"(findet nichts)", modus="da", lauf=lambda: "findet nichts" if not re.search(r"Makefile:\d+", fuehre_aus("git grep -c ^serve 904f6215 -- Makefile")) else ""))
    for doc, anker in ((RD, r"Tag `(python-standard-letzter)` \(`904f6215`\)"), ("CLAUDE.md", r"Tag `(python-standard-letzter)` \(`904f6215`")):
        rows.append(P(f"Tag-Name in {doc}", doc, anker, kommando="git rev-parse python-standard-letzter", ist=r"^([0-9a-f]{40})", modus="da"))
    for doc in (RD, "CLAUDE.md"):
        rows.append(P(f"Commit 904f6215 in {doc}", doc, r"`(904f6215)`", kommando="git cat-file -t 904f6215", ist=r"^(commit)", modus="da", erste=True))
    return rows


# ---------------------------------------------------------------- Lauf


def lies_docs() -> dict[str, str]:
    return {d: (REPO / d).read_text(encoding="utf-8") for d in DOCS}


def kopf() -> str:
    return "Zahl | Doc:Zeile | Kommando | Soll | Ist | Status"


def lauf(rows: list[P], texte: dict[str, str]) -> tuple[list[tuple], dict]:
    erg = [(p.zahl,) + pruefe(p, texte) for p in rows]
    zaehler: dict[str, int] = {}
    for e in erg:
        zaehler[e[5]] = zaehler.get(e[5], 0) + 1
    return erg, zaehler


def selbsttest() -> int:
    """Das Skript meldet eine falsche Zahl, eine verschobene Zeile und ein verbotenes Kommando; es laesst eine richtige durch."""
    texte = {"X.md": "Wert 22 Suiten\nBereich 3-5 hier\nSumme 6 860 Faelle\n"}
    gut = P("g", "X.md", r"(\d+) Suiten", lauf=lambda: "22\n")
    schlecht = P("s", "X.md", r"(\d+) Suiten", lauf=lambda: "23\n")
    bereich = P("b", "X.md", r"Bereich (3-5)", lauf=lambda: "4\n", modus="bereich")
    ausserhalb = P("b2", "X.md", r"Bereich (3-5)", lauf=lambda: "6\n", modus="bereich")
    fehlt = P("f", "X.md", r"(\d+) Gurken", lauf=lambda: "1\n")
    erwartet = [(gut, "OK"), (schlecht, "ABWEICHUNG"), (bereich, "OK"), (ausserhalb, "ABWEICHUNG"), (fehlt, "ANKER FEHLT")]
    for p, soll in erwartet:
        ist = pruefe(p, texte)[4]
        assert ist == soll, (p.zahl, ist, soll)
    assert tausend("6860") == "6[   ]?860"
    assert norm("fünf") == "5" and norm("1 234") == "1234"
    try:
        fuehre_aus("rm -rf /tmp/nichts")
    except ValueError:
        pass
    else:
        raise AssertionError("rm waere ausgefuehrt worden")
    # Gegenprobe an der echten Tabelle: aendere eine Doc-Zahl im Speicher, die Zeile muss ABWEICHUNG melden.
    echt = lies_docs()
    echt[R] = echt[R].replace("→ 22; `make golden`", "→ 23; `make golden`")
    zeile_ = next(p for p in tabelle() if p.zahl == "Parity-Suiten")
    assert pruefe(zeile_, echt)[4] == "ABWEICHUNG", "echte Zeile erkennt die geaenderte Zahl nicht"
    print("Selbsttest ok")
    return 0


def mutiere(p: P, texte: dict[str, str]) -> Optional[dict[str, str]]:
    """Kopie der Doc-Texte, in der die erste Anker-Gruppe der Zeile eine andere Zahl traegt (None: nicht moeglich)."""
    zeilen = texte[p.doc].split("\n")
    getan = False
    for i, z in enumerate(zeilen):
        m = re.search(p.anker, z)
        if m and m.groups() and m.group(1) is not None:
            alt = m.group(1)
            ziffern = re.search(r"\d+", alt)
            neu = alt[:ziffern.start()] + str(int(ziffern.group()) + 1) + alt[ziffern.end():] if ziffern else alt + "X"
            if alt in WORTE:
                neu = "sieben"
            zeilen[i] = z[:m.start(1)] + neu + z[m.end(1):]
            getan = True
            if not p.erste:
                break
    return {**texte, p.doc: "\n".join(zeilen)} if getan else None


def gegenprobe() -> int:
    """Jede nicht-NUR-ENDTOR-Zeile muss mit einer um 1 geaenderten Doc-Zahl ABWEICHUNG (oder ANKER FEHLT) melden.
    Eine Zeile, die gruen bleibt, prueft die Zahl nicht (zu loses Muster)."""
    texte = lies_docs()
    blind = []
    n = 0
    for p in tabelle():
        if p.endtor:
            continue
        m = mutiere(p, texte)
        if m is None:
            blind.append((p.zahl, "Anker trifft nicht"))
            continue
        n += 1
        status = pruefe(p, m)[4]
        if status == "OK":
            blind.append((p.zahl, "bleibt OK trotz geaenderter Doc-Zahl"))
    print(f"Gegenprobe: {n} Zeilen mutiert, {len(blind)} blind")
    for z, grund in blind:
        print(f"  BLIND {z}: {grund}")
    return 1 if blind else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--selbsttest", action="store_true")
    ap.add_argument("--gegenprobe", action="store_true")
    a = ap.parse_args()
    if a.selbsttest:
        return selbsttest()
    if a.gegenprobe:
        return gegenprobe()
    texte = lies_docs()
    erg, zaehler = lauf(tabelle(), texte)
    print(kopf())
    for zahl, ort, kom, soll, ist, status in erg:
        print(f"{zahl} | {ort} | {kom} | {soll} | {ist} | {status}")
    gesamt = len(erg)
    print(f"\nGESAMT {gesamt} | OK {zaehler.get('OK', 0)} | ABWEICHUNG {sum(v for k, v in zaehler.items() if k.startswith(('ABWEICHUNG', 'ANKER')))}"
          f" | NUR ENDTOR {zaehler.get('NUR ENDTOR', 0)}")
    return 1 if any(k.startswith(("ABWEICHUNG", "ANKER")) for k in zaehler) else 0


if __name__ == "__main__":
    sys.exit(main())
