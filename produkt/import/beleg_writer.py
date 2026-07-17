"""Beleg-Upload-Writer — Stufe 1: Lohnsteuerbescheinigung → vorlaeufige Store-Events (Task #11).

Deterministisch, NULL LLM, lokal. Extrahiert Kandidatenwerte aus dem TEXT einer Lohnsteuerbescheinigung
(pdftotext-Textlayer oder tesseract-OCR) und schreibt sie als VORLAEUFIGE Events über den einen
Store-Schreibpfad (`store.append_event`, herkunft=beleg_import, schreiber=import:beleg). Ein Beleg-Wert
ist ein VORSCHLAG (Signal 1) — der Store-Guard erzwingt vorlaeufig+signal_2=null, ein Beleg bestätigt
nie direkt (K2/fail-closed). Die Zwei-Signal-Bestätigung (K3) bleibt der Mensch.

Anker: die Bindungstabellen-`herkunft_slots` tragen die Beleg-Positions-Anker („Lohnsteuerbescheinigung
Nr. 3/22/23") — kein Rate-Match. Confidence (bei OCR aus tesseract --tsv) wandert in signal_1, ändert
aber NIE den Zustand. Unlesbar/nicht gefunden → KEIN geratener Wert, benannte Lücke (Feld bleibt offen).

Stufe 1 = Lohnsteuerbescheinigung (Positions-Anker Nr. N). Stufe 1b = Zuwendungsbestätigung (§ 10b,
spenden_betrag) + Handwerker-Rechnung (§ 35a, hh_handwerker_arbeitskosten) über einen Label-Anker.
§ 35a-Missbrauchsschutz: nur ein getrennt ausgewiesener Arbeitskosten-Betrag wird extrahiert, sonst
Lücke (nie Material/Gesamt als Arbeitskosten raten). Dienstleistungen/Minijob, Kontoauszug, LLM-Freitext
= spätere Stufen (eigener Julius-Cap).
"""
from __future__ import annotations

import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "store"))
import store as ST   # noqa: E402

_EUR = re.compile(r"\d{1,3}(?:\.\d{3})*,\d{2}")

# Beleg-Typen (Stufe 1 + 1b). marker = Kopf-Erkennung (im Text), hs_prefix = Präfix der Bindungs-
# herkunft_slots, das ein Feld diesem Beleg-Typ zuordnet. LStB nutzt einen Positions-Anker (Nr. N),
# Fließtext-Belege (Spende/Handwerker) einen Label-Anker ('… : Label'). LLM-frei.
BELEG_TYPEN = {
    "lstb":       {"marker": "lohnsteuerbescheinigung", "hs_prefix": "lohnsteuerbescheinigung"},
    "spende":     {"marker": "zuwendungsbestätigung",   "hs_prefix": "zuwendungsbestätigung"},
    "handwerker": {"marker": "handwerker",              "hs_prefix": "rechnung"},
}


def erkenne_beleg_typ(text: str):
    """Beleg-Typ aus dem Kopf-/Inhalts-Marker (lstb/spende/handwerker) oder None. Reihenfolge: die
    spezifischen Marker (LStB/Zuwendungsbestätigung) vor dem generischen Handwerker-Marker."""
    t = text.lower()
    for typ in ("lstb", "spende", "handwerker"):
        if BELEG_TYPEN[typ]["marker"] in t:
            return typ
    return None


def erkenne_lstb(text: str) -> bool:            # Rückwärts-kompatibel (Stufe 1)
    return erkenne_beleg_typ(text) == "lstb"


def _anker(hs: str):
    """Anker-Modus aus einem herkunft_slots-Eintrag: ('nr','3') für '… Nr. 3' (Formular-Position),
    sonst ('label','Arbeitskosten') für '… : Label' (Fließtext-Label)."""
    m = re.search(r"Nr\.?\s*(\d+)", hs)
    if m:
        return ("nr", m.group(1))
    return ("label", hs.split(":", 1)[1].strip() if ":" in hs else hs.strip())


def beleg_felder(bindung: dict, beleg_typ: str) -> dict:
    """{feld_id -> (modus, ankerwert)} für alle cent-Felder, deren herkunft_slots zum Beleg-Typ
    (hs_prefix) passen."""
    prefix = BELEG_TYPEN[beleg_typ]["hs_prefix"]
    out: dict = {}
    for fid, b in bindung.items():
        if b.get("typ") != "cent":
            continue
        for hs in (b.get("herkunft_slots") or []):
            if hs.lower().startswith(prefix):
                out[fid] = _anker(hs)
    return out


def lstb_felder(bindung: dict) -> dict:         # Rückwärts-kompatibel: {feld_id -> lstb_nr}
    return {fid: wert for fid, (modus, wert) in beleg_felder(bindung, "lstb").items() if modus == "nr"}


def _parse_eur_cent(betrag: str) -> int:
    """'45.000,00' -> 4500000 (Cent, Ganzzahl)."""
    return int(betrag.replace(".", "").replace(",", ""))


def _finde_betrag_zu_nr(text: str, nr: str):
    """Formular-Position: Zeile mit dem Kennzahl-Anker (Form 'Nr. N' ODER 'N.' als Kennzahl, nie als
    Teil einer Zahl) -> (cent, roh_zeile) des LETZTEN EUR-Betrags darauf, sonst None."""
    nr_re = re.compile(rf"(?:Nr\.?\s*{re.escape(nr)}\b|(?:^|\s){re.escape(nr)}\.(?!\d))")
    for zeile in text.splitlines():
        if nr_re.search(zeile):
            betraege = _EUR.findall(zeile)
            if betraege:
                return _parse_eur_cent(betraege[-1]), zeile.strip()
    return None


def _finde_betrag_zu_label(text: str, label: str):
    """Fließtext-Label: Zeile, die das Label enthält -> (cent, roh_zeile) des LETZTEN EUR-Betrags.
    Fehlt das Label (z.B. Handwerker-Rechnung OHNE getrennt ausgewiesene Arbeitskosten) -> None
    (benannte Lücke; NIE den Gesamtbetrag als Arbeitskosten raten, § 35a Abs. 5 S. 2)."""
    lab = label.lower()
    for zeile in text.splitlines():
        if lab in zeile.lower():
            betraege = _EUR.findall(zeile)
            if betraege:
                return _parse_eur_cent(betraege[-1]), zeile.strip()
    return None


def extrahiere(text: str, bindung: dict, *, confidence_map: dict | None = None) -> list:
    """Beleg-Text → Kandidaten [{feld_id, wert(cent), confidence, roh_text, beleg_typ, anker}].
    confidence_map: optional {ankerwert -> conf in [0,1]} (aus tesseract --tsv); Default 1.0 (Textlayer).
    Kein Beleg-Typ erkannt → []; nicht gefundenes Feld → weggelassen (benannte Lücke, kein Rate-Wert)."""
    typ = erkenne_beleg_typ(text)
    if typ is None:
        return []
    conf = confidence_map or {}
    kandidaten = []
    for fid, (modus, ankerwert) in sorted(beleg_felder(bindung, typ).items()):
        treffer = (_finde_betrag_zu_nr(text, ankerwert) if modus == "nr"
                   else _finde_betrag_zu_label(text, ankerwert))
        if treffer is None:
            continue                                  # unlesbar/nicht ausgewiesen -> Lücke
        cent, roh = treffer
        kandidaten.append({"feld_id": fid, "wert": cent, "confidence": float(conf.get(ankerwert, 1.0)),
                           "roh_text": roh, "beleg_typ": typ, "anker": ankerwert})
    return kandidaten


def schreibe_kandidaten(store: dict, kandidaten: list, *, beleg_ref: str, ts: str | None = None) -> list:
    """Schreibt je Kandidat ein VORLAEUFIGES Event (herkunft=beleg_import, schreiber=import:beleg).
    signal_1 = Beleg-Herkunfts-Objekt {typ, ref, confidence, roh_text}; signal_2=null (der Store-Guard
    erzwingt das ohnehin). Überschreiben eines aktiven Felds via ersetzt liegt beim Aufrufer."""
    events = []
    for k in kandidaten:
        sig1 = {"typ": "beleg", "ref": f"{beleg_ref}#{k.get('beleg_typ', 'beleg')}:{k.get('anker', '')}",
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
