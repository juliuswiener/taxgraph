#!/usr/bin/env python3
"""Prueft welche DBA-PDFs keinen (oder kaum) Textlayer haben -> gescannt/OCR noetig.

Heuristik: chars_pro_seite < THRESHOLD  => wahrscheinlich Bild-PDF ohne Textlayer.
Nutzt pdftotext (poppler) + pdfinfo.
"""
import os
import re
import subprocess
import sys

OUTDIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dba_pdfs")
THRESHOLD = 100  # chars/seite; darunter = verdaechtig

def pages(fp):
    try:
        out = subprocess.run(["pdfinfo", fp], capture_output=True, text=True, timeout=30).stdout
        m = re.search(r"Pages:\s+(\d+)", out)
        return int(m.group(1)) if m else 0
    except Exception:
        return 0

def text_len(fp):
    try:
        out = subprocess.run(["pdftotext", "-q", fp, "-"], capture_output=True, timeout=60).stdout
        return len(out.strip())
    except Exception:
        return -1

def main():
    files = sorted(f for f in os.listdir(OUTDIR) if f.lower().endswith(".pdf"))
    scanned, thin, ok, err = [], [], [], []
    for fn in files:
        fp = os.path.join(OUTDIR, fn)
        p = pages(fp)
        tl = text_len(fp)
        if tl < 0:
            err.append(fn); continue
        cpp = tl / p if p else tl
        rec = (fn, p, tl, round(cpp))
        if tl < 20:
            scanned.append(rec)          # praktisch kein Text = Bild-PDF
        elif cpp < THRESHOLD:
            thin.append(rec)             # sehr wenig Text/Seite = verdaechtig
        else:
            ok.append(rec)
    def dump(title, arr):
        print(f"\n=== {title} ({len(arr)}) ===")
        for fn, p, tl, cpp in arr:
            print(f"  {p:>3}S {tl:>7}z {cpp:>5}z/S  {fn}")
    print(f"TOTAL: {len(files)} PDFs")
    dump("KEIN TEXTLAYER (gescannt, OCR noetig)", scanned)
    dump("DUENN (<%d z/S, evtl. Teil-Scan)" % THRESHOLD, thin)
    print(f"\n=== OK (Textlayer vorhanden) === {len(ok)}")
    if err:
        dump("FEHLER beim Lesen", err)

if __name__ == "__main__":
    main()
