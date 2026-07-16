#!/usr/bin/env python3
"""BMF DBA-PDF Scraper.

Zieht alle staatenbezogenen DBA-/Steuer-PDFs von der Bundesfinanzministerium-Seite.
Umgeht die Radware-Bot-Wall via Session-Warmup (Cookies) + realistische Header + Retry.

Ablauf:
  Uebersicht -> Laenderseiten -> Artikelseiten -> Blob-PDF-Downloads.
"""
import os
import re
import sys
import time
import random
import argparse
from urllib.parse import urljoin, urlparse, parse_qs

import requests
from bs4 import BeautifulSoup

BASE = "https://www.bundesfinanzministerium.de"
OVERVIEW = (BASE + "/Web/DE/Themen/Steuern/Internationales_Steuerrecht/"
            "Staatenbezogene_Informationen/staatenbezogene_info.html")
OUTDIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), "dba_pdfs")

HEADERS = {
    "User-Agent": ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
                   "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"),
    "Accept": ("text/html,application/xhtml+xml,application/xml;q=0.9,"
               "image/avif,image/webp,*/*;q=0.8"),
    "Accept-Language": "de-DE,de;q=0.9,en;q=0.8",
    "Upgrade-Insecure-Requests": "1",
    "Sec-Fetch-Dest": "document",
    "Sec-Fetch-Mode": "navigate",
    "Sec-Fetch-Site": "same-origin",
    "Sec-Fetch-User": "?1",
}

DELAY = (0.8, 1.6)   # Jitter-Range zwischen Requests
MAX_RETRY = 5


def sleep_jitter():
    time.sleep(random.uniform(*DELAY))


def blocked(resp) -> bool:
    return ("perfdrive" in resp.url) or ("validate.perfdrive" in resp.text[:2000])


class Fetcher:
    def __init__(self):
        self.s = requests.Session()
        self.s.headers.update(HEADERS)
        self.warm()

    def warm(self):
        """Uebersicht laden -> Radware-Cookies (__uzm*) setzen."""
        h = dict(self.s.headers)
        self.s.headers["Sec-Fetch-Site"] = "none"
        try:
            self.s.get(OVERVIEW, timeout=40)
        except Exception:
            pass
        self.s.headers["Sec-Fetch-Site"] = "same-origin"
        self.s.headers["Referer"] = OVERVIEW

    def get(self, url, referer=None):
        if referer:
            self.s.headers["Referer"] = referer
        for attempt in range(1, MAX_RETRY + 1):
            try:
                r = self.s.get(url, timeout=60)
            except Exception as e:
                if attempt == MAX_RETRY:
                    raise
                time.sleep(2 * attempt)
                continue
            if r.status_code == 200 and not blocked(r):
                return r
            # blockiert oder Fehler -> re-warm + backoff
            time.sleep(1.5 * attempt + random.uniform(0, 1))
            self.warm()
        raise RuntimeError(f"Nach {MAX_RETRY} Versuchen blockiert: {url}")


def clean_filename(url: str) -> str:
    """Dateiname aus Blob-URL ableiten (letztes Pfadsegment, .pdf erzwungen)."""
    path = urlparse(url).path
    name = path.rstrip("/").split("/")[-1]
    if not name.lower().endswith(".pdf"):
        name += ".pdf"
    name = re.sub(r'[^A-Za-z0-9._-]', '_', name)
    return name


def discover_countries(f: Fetcher):
    r = f.get(OVERVIEW, referer=OVERVIEW)
    soup = BeautifulSoup(r.text, "html.parser")
    countries = set()
    for a in soup.find_all("a", href=True):
        h = a["href"]
        # Laenderseiten: .../Staatenbezogene_Informationen/<Land>/<land>.html
        if ("Staatenbezogene_Informationen" in h
                and h.lower().endswith(".html")
                and "Laender_A_Z" not in h
                and "staatenbezogene_info" not in h):
            countries.add(urljoin(BASE + "/", h))
    return sorted(countries)


def article_links(soup):
    out = set()
    for a in soup.find_all("a", href=True):
        h = a["href"]
        if "Standardartikel" in h and h.lower().endswith(".html"):
            out.add(urljoin(BASE + "/", h))
    return out


def pdf_links(soup):
    """Nur DBA-PDFs: Blob-URL muss im Staatenbezogene-Pfad liegen.
    Filtert BMF-News-Teaser (Koalitionsausschuss etc.) aus Sidebars raus."""
    out = set()
    for a in soup.find_all("a", href=True):
        h = a["href"]
        if "blob=publicationFile" in h or h.lower().endswith(".pdf"):
            full = urljoin(BASE + "/", h)
            if "Staatenbezogene_Informationen" in full:
                out.add(full)
    return out


def download(f: Fetcher, url, referer, seen_names, log):
    fn = clean_filename(url)
    # Kollisions-Schutz: wenn Name existiert aber URL anders -> suffix
    fp = os.path.join(OUTDIR, fn)
    if fn in seen_names:
        return "dup-name"
    if os.path.exists(fp):
        seen_names.add(fn)
        return "exists"
    for attempt in range(1, MAX_RETRY + 1):
        try:
            r = f.s.get(url, timeout=90, headers={**f.s.headers, "Referer": referer,
                                                  "Sec-Fetch-Dest": "document"})
        except Exception:
            time.sleep(2 * attempt); continue
        if r.status_code == 200 and not blocked(r) and r.content[:4] == b"%PDF":
            with open(fp, "wb") as fh:
                fh.write(r.content)
            seen_names.add(fn)
            return f"ok {len(r.content)//1024}KB"
        if r.status_code == 200 and r.content[:4] != b"%PDF":
            # evtl. blockiert -> re-warm
            f.warm()
        time.sleep(1.5 * attempt)
    return "FAIL"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--limit", type=int, default=0, help="nur N Laender (Test)")
    ap.add_argument("--only", type=str, default="", help="nur Laender mit Substring")
    args = ap.parse_args()

    os.makedirs(OUTDIR, exist_ok=True)
    f = Fetcher()

    countries = discover_countries(f)
    if args.only:
        countries = [c for c in countries if args.only.lower() in c.lower()]
    if args.limit:
        countries = countries[:args.limit]
    print(f"[i] {len(countries)} Laenderseiten", flush=True)

    seen_names = set()
    stats = {"pdf_ok": 0, "exists": 0, "fail": 0, "articles": 0}

    for ci, curl in enumerate(countries, 1):
        cname = urlparse(curl).path.split("/")[-2]
        try:
            sleep_jitter()
            r = f.get(curl, referer=OVERVIEW)
        except Exception as e:
            print(f"[{ci}/{len(countries)}] {cname}: LADEN FEHLGESCHLAGEN {e}", flush=True)
            continue
        soup = BeautifulSoup(r.text, "html.parser")

        # 1) direkte PDFs auf Laenderseite
        pdfs = set(pdf_links(soup))
        # 2) Artikelseiten -> deren PDFs
        arts = article_links(soup)
        stats["articles"] += len(arts)
        for au in sorted(arts):
            try:
                sleep_jitter()
                ra = f.get(au, referer=curl)
            except Exception:
                continue
            pdfs |= pdf_links(BeautifulSoup(ra.text, "html.parser"))

        got = 0
        for pu in sorted(pdfs):
            sleep_jitter()
            res = download(f, pu, curl, seen_names, None)
            if res.startswith("ok"):
                stats["pdf_ok"] += 1; got += 1
            elif res == "exists":
                stats["exists"] += 1
            elif res == "FAIL":
                stats["fail"] += 1
                print(f"      FAIL {pu[:120]}", flush=True)
        print(f"[{ci}/{len(countries)}] {cname}: {len(arts)} Artikel, "
              f"{len(pdfs)} PDFs ({got} neu)", flush=True)

    print(f"\n=== FERTIG === neu:{stats['pdf_ok']} vorhanden:{stats['exists']} "
          f"fail:{stats['fail']} artikel:{stats['articles']}", flush=True)


if __name__ == "__main__":
    main()
