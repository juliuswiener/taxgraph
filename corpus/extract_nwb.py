#!/usr/bin/env python3
"""NWB-Gesetze DBA-Volltext-Extraktor.

NWB serviert die DBA-Artikeltexte frei unter /Dokument/<docid>_<N>/.
Zieht pro Land: Praeambel (Hauptdoc) + jeden Artikel/Anlage -> sauberes Markdown.

Input : nwb_index.json  {docid: "DBA <Land> ... i.d.F. <datum>"}
Output: dba_text_nwb/<Land>.md
"""
import os
import re
import json
import time
import random
import requests
from bs4 import BeautifulSoup

BASE = "https://datenbank.nwb.de"
HERE = os.path.dirname(os.path.abspath(__file__))
OUTDIR = os.path.join(HERE, "dba_text_nwb")
INDEX = os.path.join(HERE, "nwb_index.json")

HEADERS = {
    "User-Agent": ("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 "
                   "(KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36"),
    "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    "Accept-Language": "de-DE,de;q=0.9,en;q=0.8",
}
DELAY = (0.4, 0.9)
MAX_RETRY = 4


def get(s, url):
    for attempt in range(1, MAX_RETRY + 1):
        try:
            r = s.get(url, timeout=45)
            if r.status_code == 200 and len(r.text) > 500:
                return r
        except Exception:
            pass
        time.sleep(1.2 * attempt)
    return None


def content_div(soup):
    """Haupt-Inhaltscontainer der Dokumentseite."""
    d = soup.find("div", class_="dokumentinhaltcontent")
    if d is None:
        d = soup.find("div", class_="main-content")
    return d


def block_text(div):
    """Textblock: pro Absatz-Element flach (Soft-Wraps zusammengefuehrt),
    Absaetze durch Leerzeile getrennt."""
    if div is None:
        return ""
    blocks = div.find_all(["p", "li", "h1", "h2", "h3", "h4", "td", "div"],
                          recursive=True)
    lines = []
    seen = set()
    for b in blocks:
        # nur Blaetter (keine Container, die andere Bloecke enthalten)
        if b.find(["p", "li", "td"]):
            continue
        t = b.get_text(" ", strip=True).replace("\xa0", " ")
        t = re.sub(r"\s+", " ", t).strip()
        if t and t not in seen:
            seen.add(t)
            lines.append(t)
    if not lines:  # Fallback
        t = div.get_text(" ", strip=True).replace("\xa0", " ")
        lines = [re.sub(r"\s+", " ", t)]
    return "\n\n".join(lines).strip()


def sublinks(soup, docid):
    """Artikel-/Anlagen-Sublinks <docid>_<N> in Dokumentreihenfolge (dedupe).
    N = arabisch (1,2,..) ODER roemisch (I,II,..) bei alten Abkommen."""
    order = []
    seen = set()
    for a in soup.find_all("a", href=True):
        m = re.search(rf"/Dokument/{docid}_([IVXLCDM]+|\d+)(?:/|$|\?)", a["href"])
        if m:
            n = m.group(1)
            if n not in seen:
                seen.add(n)
                order.append(n)   # Dokumentreihenfolge beibehalten
    return order


def country_from_title(title):
    m = re.match(r"DBA\s+(.*?)\s+DBA", title)
    name = m.group(1) if m else title
    name = re.sub(r'[^\wÄÖÜäöüß /()-]', '', name).strip()
    return re.sub(r'[ /]+', '_', name)


def strip_breadcrumb(body, docid_title):
    """Fuehrende Breadcrumb-Zeile 'DBA X Artikel N i.d.F. datum' entfernen."""
    # bis zur ersten 'i.d.F. <datum>' Zeile abschneiden
    m = re.search(r"i\.d\.F\.\s*\d{2}\.\d{2}\.\d{4}\s*\n?", body)
    if m:
        return body[m.end():].strip()
    return body


def main():
    os.makedirs(OUTDIR, exist_ok=True)
    index = json.load(open(INDEX, encoding="utf-8"))
    s = requests.Session(); s.headers.update(HEADERS)

    items = list(index.items())
    print(f"[i] {len(items)} DBA-Dokumente", flush=True)
    done = fail = 0

    for i, (docid, title) in enumerate(items, 1):
        country = country_from_title(title)
        outfp = os.path.join(OUTDIR, f"{country}.md")
        if os.path.exists(outfp) and os.path.getsize(outfp) > 400:
            done += 1
            print(f"[{i}/{len(items)}] {country}: skip (vorhanden)", flush=True)
            continue

        main_url = f"{BASE}/Dokument/{docid}/"
        r = get(s, main_url)
        if r is None:
            fail += 1; print(f"[{i}/{len(items)}] {country}: HAUPTDOC FAIL", flush=True); continue
        soup = BeautifulSoup(r.text, "html.parser")
        preamble = block_text(content_div(soup))
        subs = sublinks(soup, docid)

        parts = [f"# {title}", "",
                 f"> Quelle: NWB Gesetze — {main_url}", ""]
        if preamble:
            parts += ["## Präambel / Kopf", "", preamble, ""]

        got_arts = 0
        for n in subs:
            time.sleep(random.uniform(*DELAY))
            ra = get(s, f"{BASE}/Dokument/{docid}_{n}/")
            if ra is None:
                continue
            body = block_text(content_div(BeautifulSoup(ra.text, "html.parser")))
            body = strip_breadcrumb(body, title)
            if body:
                parts += [body, ""]
                got_arts += 1

        text = "\n".join(parts).strip() + "\n"
        with open(outfp, "w", encoding="utf-8") as fh:
            fh.write(text)
        done += 1
        print(f"[{i}/{len(items)}] {country}: {got_arts}/{len(subs)} Artikel, "
              f"{len(text)//1024}KB", flush=True)
        time.sleep(random.uniform(*DELAY))

    print(f"\n=== NWB FERTIG === ok:{done} fail:{fail}", flush=True)


if __name__ == "__main__":
    main()
