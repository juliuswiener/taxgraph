#!/usr/bin/env python3
"""Friert ein LOKAL vorliegendes amtliches PDF (z.B. ELSTER-Vordruck) nach sources/ ein.

Anders als `freeze_source.py` (das aus einer URL FETCHT) liest dieses Skript eine bereits
lokal liegende Datei — kein Netzwerk. Genutzt fuer amtliche Vordrucke (formulare-bfinv.de),
die der Nutzer selbst heruntergeladen und abgelegt hat (Download ist Nutzer-Aktion, nicht
meine — externe Downloads brauchen direkte Nutzer-Freigabe).

Ablauf: pdftotext -> Anker-Pruefung (--erwarte muss woertlich im Text stehen) -> sha256 von
PDF und Text -> schreibt sources/<unter>/<name>.txt + <name>.meta.yaml (Format wie
freeze_source, plus pdf_sha256/pdf_datei). `make sources-check` prueft danach die Text-sha256.

Beispiel:
  python3 scripts/freeze_pdf_local.py \\
    --datei ~/Downloads/ESt1A_2025.pdf --unter bfinv \\
    --name est1a_2025 --titel "ESt 1 A 2025 (Mantelbogen)" \\
    --url https://www.formulare-bfinv.de/.../ESt1A_2025.pdf --abrufdatum 2026-07-12 \\
    --erwarte "Außergewöhnliche Belastungen"
"""
from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import subprocess
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))


def _sha256(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for blk in iter(lambda: f.read(65536), b""):
            h.update(blk)
    return h.hexdigest()


def _pdftotext(pdf: str) -> str:
    if not shutil.which("pdftotext"):
        raise SystemExit("pdftotext nicht gefunden (poppler-utils installieren).")
    out = subprocess.run(["pdftotext", "-layout", pdf, "-"],
                         capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit(f"pdftotext fehlgeschlagen: {out.stderr[:300]}")
    return out.stdout


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--datei", required=True, help="lokaler PDF-Pfad (Nutzer-Download)")
    ap.add_argument("--unter", default="bfinv", help="Unterordner in sources/ (Default bfinv)")
    ap.add_argument("--name", required=True, help="Dateiname ohne Endung")
    ap.add_argument("--titel", required=True)
    ap.add_argument("--url", required=True, help="amtliche Herkunfts-URL (Provenance)")
    ap.add_argument("--abrufdatum", required=True)
    ap.add_argument("--erwarte", action="append", default=[],
                    help="Passage, die woertlich im Text vorkommen MUSS (mehrfach)")
    ap.add_argument("--min-laenge", type=int, default=500)
    ap.add_argument("--authority", default="amtlicher_vordruck",
                    help="amtlicher_vordruck (Formular) | verwaltung (Anleitung/Auslegung)")
    ap.add_argument("--verwendet-in", default="")
    args = ap.parse_args()

    pdf = os.path.expanduser(args.datei)
    if not os.path.isfile(pdf):
        raise SystemExit(f"PDF nicht gefunden: {pdf} — Julius muss es zuerst lokal ablegen.")

    text = _pdftotext(pdf)
    if len(text) < args.min_laenge:
        raise SystemExit(f"Text zu kurz ({len(text)} < {args.min_laenge}) — Extraktion pruefen.")
    fehlend = [e for e in args.erwarte if e not in text]
    if fehlend:
        raise SystemExit("Anker fehlen woertlich im Text (Freeze abgebrochen): "
                         + "; ".join(fehlend))

    out_dir = os.path.join(ROOT, "sources", args.unter)
    os.makedirs(out_dir, exist_ok=True)
    txt_path = os.path.join(out_dir, f"{args.name}.txt")
    pdf_path = os.path.join(out_dir, f"{args.name}.pdf")
    header = (f"Quelle: {args.url}\nAbgerufen: {args.abrufdatum}\n"
              f"Titel: {args.titel}\nHerkunft: amtlicher Vordruck (lokal eingefroren)\n"
              f"--- Textextraktion (pdftotext -layout) ---\n\n")
    body = header + text
    with open(txt_path, "w", encoding="utf-8") as f:
        f.write(body)
    shutil.copyfile(pdf, pdf_path)

    txt_sha = _sha256(txt_path)
    pdf_sha = _sha256(pdf_path)
    meta = (
        "# Eingefrorener amtlicher Vordruck (lokal, Nutzer-Download). Provenance-Archiv.\n"
        "dokument:\n"
        f'  titel: "{args.titel}"\n'
        f'  quelle_url: "{args.url}"\n'
        f'  abrufdatum: "{args.abrufdatum}"\n'
        f'  datei: "{args.name}.txt"\n'
        f'  pdf_datei: "{args.name}.pdf"\n'
        f'  sha256: "{txt_sha}"\n'
        f'  pdf_sha256: "{pdf_sha}"\n'
        f"  authority: {args.authority}\n"
        "  redistributable: true\n"
        f'  erwartete_anker: {args.erwarte!r}\n'
        f'  verwendet_in:\n    - "{args.verwendet_in}"\n'
    )
    with open(os.path.join(out_dir, f"{args.name}.meta.yaml"), "w", encoding="utf-8") as f:
        f.write(meta)

    print(f"[freeze] {txt_path}  (Text sha256 {txt_sha[:12]}…)")
    print(f"[freeze] {pdf_path}  (PDF  sha256 {pdf_sha[:12]}…)")
    print(f"[freeze] Anker ok: {args.erwarte}")
    print("[freeze] make sources-check sollte jetzt gruen sein.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
