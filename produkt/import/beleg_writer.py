"""Beleg-Upload-Writer — Stufe 1: Lohnsteuerbescheinigung → vorlaeufige Store-Events (Task #11).

Deterministisch, NULL LLM, lokal. Extrahiert Kandidatenwerte aus dem TEXT einer Lohnsteuerbescheinigung
(pdftotext-Textlayer oder tesseract-OCR) und schreibt sie als VORLAEUFIGE Events über den einen
Store-Schreibpfad (`store.append_event`, herkunft=beleg_import, schreiber=import:beleg). Ein Beleg-Wert
ist ein VORSCHLAG (Signal 1) — der Store-Guard erzwingt vorlaeufig+signal_2=null, ein Beleg bestätigt
nie direkt (K2/fail-closed). Die Zwei-Signal-Bestätigung (K3) bleibt der Mensch.

Anker: die Bindungstabellen-`herkunft_slots` tragen die Beleg-Positions-Anker („Lohnsteuerbescheinigung
Nr. 3/22/23") — kein Rate-Match. Confidence (bei OCR aus tesseract --tsv) wandert in signal_1, ändert
aber NIE den Zustand. Unlesbar/nicht gefunden → KEIN geratener Wert, benannte Lücke (Feld bleibt offen).

Stufe 1 = NUR Lohnsteuerbescheinigung, NUR cent-Felder mit LStB-Anker. Spende/Handwerker,
Kontoauszug, LLM-Freitext = spätere Stufen (eigener Julius-Cap).
"""
from __future__ import annotations

import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "store"))
import store as ST   # noqa: E402

LSTB_KOPF_MARKER = "lohnsteuerbescheinigung"
_EUR = re.compile(r"\d{1,3}(?:\.\d{3})*,\d{2}")


def erkenne_lstb(text: str) -> bool:
    """Beleg-Typ-Erkennung Stufe 1: nur Lohnsteuerbescheinigung (Kopf-Marker)."""
    return LSTB_KOPF_MARKER in text.lower()


def lstb_felder(bindung: dict) -> dict:
    """{feld_id -> lstb_nr} für alle cent-Felder, deren herkunft_slots einen LStB-Nummern-Anker tragen."""
    out: dict = {}
    for fid, b in bindung.items():
        if b.get("typ") != "cent":
            continue
        for hs in (b.get("herkunft_slots") or []):
            m = re.search(r"Lohnsteuerbescheinigung\s+Nr\.?\s*(\d+)", hs)
            if m:
                out[fid] = m.group(1)
    return out


def _parse_eur_cent(betrag: str) -> int:
    """'45.000,00' -> 4500000 (Cent, Ganzzahl)."""
    return int(betrag.replace(".", "").replace(",", ""))


def _finde_betrag_zu_nr(text: str, nr: str):
    """Sucht die Zeile mit dem LStB-Kennzahl-Anker (Form 'Nr. N' ODER 'N.' als Kennzahl, nie als
    Teil einer Zahl) und liefert (cent, roh_zeile) des LETZTEN EUR-Betrags darauf, sonst None."""
    nr_re = re.compile(rf"(?:Nr\.?\s*{re.escape(nr)}\b|(?:^|\s){re.escape(nr)}\.(?!\d))")
    for zeile in text.splitlines():
        if nr_re.search(zeile):
            betraege = _EUR.findall(zeile)
            if betraege:
                return _parse_eur_cent(betraege[-1]), zeile.strip()
    return None


def extrahiere(text: str, bindung: dict, *, confidence_map: dict | None = None) -> list:
    """LStB-Text → Kandidaten [{feld_id, wert(cent), confidence, roh_text, lstb_nr}].
    confidence_map: optional {lstb_nr -> conf in [0,1]} (aus tesseract --tsv); Default 1.0 (Textlayer).
    Kein LStB erkannt → []; nicht gefundenes Feld → weggelassen (benannte Lücke, kein Rate-Wert)."""
    if not erkenne_lstb(text):
        return []
    conf = confidence_map or {}
    kandidaten = []
    for fid, nr in sorted(lstb_felder(bindung).items()):
        treffer = _finde_betrag_zu_nr(text, nr)
        if treffer is None:
            continue                                  # unlesbar/nicht gefunden -> Lücke
        cent, roh = treffer
        kandidaten.append({"feld_id": fid, "wert": cent, "confidence": float(conf.get(nr, 1.0)),
                           "roh_text": roh, "lstb_nr": nr})
    return kandidaten


def schreibe_kandidaten(store: dict, kandidaten: list, *, beleg_ref: str, ts: str | None = None) -> list:
    """Schreibt je Kandidat ein VORLAEUFIGES Event (herkunft=beleg_import, schreiber=import:beleg).
    signal_1 = Beleg-Herkunfts-Objekt {typ, ref, confidence, roh_text}; signal_2=null (der Store-Guard
    erzwingt das ohnehin). Überschreiben eines aktiven Felds via ersetzt liegt beim Aufrufer."""
    events = []
    for k in kandidaten:
        sig1 = {"typ": "beleg", "ref": f"{beleg_ref}#lstb_nr={k['lstb_nr']}",
                "confidence": k["confidence"], "roh_text": k["roh_text"]}
        ev = ST.append_event(store, feld_id=k["feld_id"], wert=k["wert"], zustand="vorlaeufig",
                             herkunft={"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft",
                                       "haftung": "nutzer"},
                             schreiber="import:beleg",
                             signal={"signal_1": sig1, "signal_2": None}, ts=ts)
        events.append(ev)
    return events


def lies_beleg_text(pfad: str) -> str:
    """Realbetrieb (nicht im Gate): Textlayer zuerst (pdftotext), sonst tesseract-deu (lokal, kein
    externer Dienst). Reine I/O-Schale um extrahiere(); die deterministische Kernlogik ist textbasiert."""
    if pfad.lower().endswith((".txt",)):
        return open(pfad, encoding="utf-8").read()
    txt = subprocess.run(["pdftotext", "-layout", pfad, "-"], capture_output=True, text=True).stdout
    if txt.strip():
        return txt
    return subprocess.run(["tesseract", pfad, "-", "-l", "deu"], capture_output=True, text=True).stdout
