"""Eine Rechtsquelle einfrieren: abrufen, entstrippen, pruefen, sha256, Metadaten.

Warum ein Skript und kein Einzeiler: beim Einfrieren von § 9 und § 6 hat ein
falsches Start-Muster zwei LEERE Quellen erzeugt. `sources-check` war trotzdem
gruen - er prueft nur, ob die Datei zu ihrem sha256 passt, und eine leere Datei
passt zu ihrem sha256. Eine leere Quelle faellt erst auf, wenn Wochen spaeter ein
Zitatanker ins Leere zeigt.

Die Pruefungen hier laufen VOR dem Schreiben:
  * der Text ist laenger als `--min-laenge` (Vorgabe 500 Zeichen),
  * jede mit `--erwarte` genannte Passage kommt woertlich vor.

Ohne beides wird nichts geschrieben.

    python scripts/freeze_source.py \\
        --url https://www.gesetze-im-internet.de/estg/__9.html \\
        --name estg_p9_2026-07-10 \\
        --norm "§ 9 EStG (Werbungskosten)" \\
        --start "(1) 1 Werbungskosten sind" --ende "Fußnote" \\
        --erwarte "Nummer 7 bleibt unberührt" --erwarte "Entfernungspauschale"
"""

from __future__ import annotations

import argparse
import hashlib
import html
import os
import re
import sys
import urllib.request

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
OUT = os.path.join(ROOT, "sources", "gesetze-im-internet")


def fetch(url: str) -> str:
    req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return r.read().decode("utf-8", "replace")


def entstrippen(raw: str, start: str, ende: str | None) -> str:
    t = re.sub(r"<script.*?</script>", "", raw, flags=re.S)
    t = re.sub(r"</p>|<br\s*/?>", "\n", t)
    t = re.sub(r"<[^>]+>", " ", t)
    t = html.unescape(t)
    # geschuetzte Leerzeichen brechen jedes spaetere grep auf "4 000 Euro"
    t = t.replace("\xa0", " ")
    t = re.sub(r"[ \t]+", " ", t)
    i = t.find(start)
    if i < 0:
        raise SystemExit(f"Start-Passage nicht gefunden: {start!r}")
    j = t.find(ende, i + 10) if ende else -1
    body = t[i:j] if j > i else t[i:]
    return "\n".join(l.strip() for l in body.split("\n") if l.strip())


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--url", required=True)
    ap.add_argument("--name", required=True, help="Dateiname ohne Endung")
    ap.add_argument("--norm", required=True)
    ap.add_argument("--start", required=True)
    ap.add_argument("--ende")
    ap.add_argument("--erwarte", action="append", default=[],
                    help="Passage, die woertlich vorkommen MUSS (mehrfach moeglich)")
    ap.add_argument("--min-laenge", type=int, default=500)
    ap.add_argument("--fassung", default="geltende Fassung 2026")
    ap.add_argument("--abrufdatum", required=True)
    ap.add_argument("--verwendet-in", default="")
    args = ap.parse_args()

    body = entstrippen(fetch(args.url), args.start, args.ende)

    if len(body) < args.min_laenge:
        raise SystemExit(f"Text zu kurz ({len(body)} < {args.min_laenge} Zeichen). "
                         f"Nichts geschrieben - vermutlich hat das Start-Muster "
                         f"nicht getroffen.")
    fehlend = [e for e in args.erwarte if e not in body]
    if fehlend:
        raise SystemExit("Erwartete Passage(n) fehlen, nichts geschrieben:\n  "
                         + "\n  ".join(repr(f) for f in fehlend))

    txt = os.path.join(OUT, f"{args.name}.txt")
    kopf = (f"Quelle: {args.url}\nAbgerufen: {args.abrufdatum}\n"
            f"Norm: {args.norm}\nFassung: {args.fassung}\n"
            f"Einfrier-Ebene: ganzer Paragraph (Konvention, siehe sources/README.md).\n"
            f"Hinweis: geschuetzte Leerzeichen (U+00A0) durch normale ersetzt.\n\n"
            f"--- Wortlaut (abgerufener Ausschnitt) ---\n\n")
    with open(txt, "w", encoding="utf-8") as f:
        f.write(kopf + body + "\n")

    sha = hashlib.sha256(open(txt, "rb").read()).hexdigest()
    verwendet = f'["{args.verwendet_in}"]' if args.verwendet_in else "[]"
    with open(os.path.join(OUT, f"{args.name}.meta.yaml"), "w", encoding="utf-8") as f:
        f.write(f'''dokument:
  norm_uri: "{args.name.replace('_', '/')}"
  norm: "{args.norm}"
  fassung: "{args.fassung}"
  quelle_url: "{args.url}"
  abrufdatum: "{args.abrufdatum}"
  datei: "{args.name}.txt"
  sha256: "{sha}"
  authority: gesetz
  redistributable: true
  einfrier_ebene: paragraph
  verwendet_in: {verwendet}
''')
    print(f"{args.name}: {len(body)} Zeichen, {len(args.erwarte)} Passage(n) verifiziert, "
          f"sha256 {sha[:16]}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
