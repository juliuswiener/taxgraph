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
spenden_betrag) + Handwerker-Rechnung (§ 35a, hh_handwerker_betrag) über einen Label-Anker.
§ 35a-Missbrauchsschutz: nur ein getrennt ausgewiesener Arbeitskosten-Betrag wird extrahiert, sonst
Lücke (nie Material/Gesamt als Arbeitskosten raten). Dienstleistungen/Minijob, Kontoauszug, LLM-Freitext
= spätere Stufen (eigener Julius-Cap).
"""
from __future__ import annotations

import csv
import io
import os
import re
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "store"))
import store as ST   # noqa: E402

_EUR = re.compile(r"\d{1,3}(?:\.\d{3})*,\d{2}")

# Beleg-Typen (Stufe 1/1b/1c). hs_prefix = TYP-Tag im Bindungs-herkunft_slots (Provenienz + Typ-Scoping:
# ein als Typ X erkanntes Dokument füllt NUR die Felder mit diesem Präfix — der Beleg-TYP disambiguiert,
# nicht die Anker-Prosa). LStB nutzt Positions-Anker (Nr. N), Fließtext-Belege einen Label-Anker.
# §35a-Absatz je Feld: in der Bindung über anker_ref.quelle (Abs.1 Minijob / Abs.2 Dienstleistung /
# Abs.3 Handwerker) — dev-1s Ring wendet den richtigen Höchstbetrag an (Ring-Folgearbeit). LLM-frei.
BELEG_TYPEN = {
    "lstb":            {"hs_prefix": "lohnsteuerbescheinigung"},
    "spende":          {"hs_prefix": "zuwendungsbestätigung"},
    "handwerker":      {"hs_prefix": "handwerkerrechnung"},
    "dienstleistung":  {"hs_prefix": "dienstleistungsrechnung"},
    "minijob":         {"hs_prefix": "minijob-bescheinigung"},
}


def erkenne_beleg_typ(text: str):
    """Beleg-Typ aus dem Kopf/Inhalt oder None. FAIL-CLOSED bei Handwerker/Dienstleistung-Mehrdeutigkeit:
    § 35a Abs. 2 (Dienstleistung, max 4.000 €) und Abs. 3 (Handwerker, max 1.200 €) haben VERSCHIEDENE
    Höchstbeträge — eine Fehlklassifikation ist ein falscher Bescheid. Ist der Typ nicht sicher (beide
    oder keiner der Marker), wird NICHT geraten und NICHT still ein Feld gefüllt (K2): None → Mensch
    entscheidet (die Haut fragt 'Handwerker- oder haushaltsnahe Dienstleistung?')."""
    t = text.lower()
    if "lohnsteuerbescheinigung" in t:
        return "lstb"
    if "zuwendungsbestätigung" in t or "geldzuwendung" in t:
        return "spende"
    if "minijob" in t or "haushaltsscheck" in t:
        return "minijob"
    hw = "handwerker" in t
    dl = "dienstleistung" in t or "haushaltsnah" in t
    if hw and not dl:
        return "handwerker"
    if dl and not hw:
        return "dienstleistung"
    return None                                     # mehrdeutig/unklar → K2: nicht raten


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


def schreibe_kandidaten(store: dict, kandidaten: list, *, beleg_ref: str, bindung: dict,
                        ts: str | None = None) -> list:
    """Schreibt je Kandidat ein VORLAEUFIGES Event (herkunft=beleg_import, schreiber=import:beleg).
    signal_1 = Beleg-Herkunfts-Objekt {typ, ref, confidence, roh_text}; signal_2=null (der Store-Guard
    erzwingt das ohnehin). Überschreiben eines aktiven Felds via ersetzt liegt beim Aufrufer.
    `bindung` = für den Feld-Katalog-Check (K1): ein import:beleg-Schreiber darf nur beleg-freigegebene
    Felder setzen (store.lade_katalog); ein Beleg für ein human-only-Feld fällt fail-closed."""
    katalog = ST.lade_katalog(bindung)
    events = []
    for k in kandidaten:
        sig1 = {"typ": "beleg", "ref": f"{beleg_ref}#{k.get('beleg_typ', 'beleg')}:{k.get('anker', '')}",
                "confidence": k["confidence"], "roh_text": k["roh_text"]}
        ev = ST.append_event(store, feld_id=k["feld_id"], wert=k["wert"], zustand="vorlaeufig",
                             herkunft={"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft",
                                       "haftung": "nutzer"},
                             schreiber="import:beleg",
                             signal={"signal_1": sig1, "signal_2": None}, ts=ts, katalog=katalog)
        events.append(ev)
    return events


def _textlayer_ist_plausibel(seiten_text: str) -> bool:
    """Wie kontoauszug_writer._textlayer_ist_plausibel (geteiltes Muster, gleicher Schwellwert):
    Mindest-Zeichendichte, konservativ (K2: lieber unnötige Extra-OCR als eine still verpasste
    Bild-Seite).
    BACKLOG (non-blocking, dev-3-Review 2026-07-20): eine gescannte Seite mit echter Kopf-/Fußzeile
    ≥20 Zeichen könnte fälschlich "plausibel" durchgehen, obwohl der eigentliche Belegtext Bild ist ->
    stille OCR-Lücke. Braucht echte Scan-Samples (Julius-Cap), siehe [[pdf-teil-textlayer-luecke]]."""
    return len(seiten_text.strip()) >= 20


def lies_beleg_text(pfad: str) -> tuple[str, dict]:
    """Realbetrieb (nicht im Gate): Textlayer zuerst (pdftotext), sonst tesseract-deu (lokal, kein
    externer Dienst). Reine I/O-Schale um extrahiere(); die deterministische Kernlogik ist textbasiert.
    Teil-Textlayer (mehrseitiger Beleg, z.B. Seite 1 Textlayer + Seite 2 Scan): jede implausible Seite
    wird EINZELN nach-OCR't (kein unnötiges Voll-Rastern) — geteiltes Muster mit
    kontoauszug_writer.lies_kontoauszug_pdf. Rückgabe (text, confidence_map je Zeilenindex; leer bei
    reinem Textlayer/.txt)."""
    if pfad.lower().endswith((".txt",)):
        return open(pfad, encoding="utf-8").read(), {}
    txt = subprocess.run(["pdftotext", "-layout", pfad, "-"], capture_output=True, text=True).stdout
    if not txt.strip():
        return subprocess.run(["tesseract", pfad, "-", "-l", "deu"], capture_output=True, text=True).stdout, {}

    seiten = txt.split("\x0c")[:-1]
    if all(_textlayer_ist_plausibel(s) for s in seiten):
        return txt, {}

    text_teile = []
    conf_map: dict = {}
    zeilen_offset = 0
    for seiten_nr, seiten_text in enumerate(seiten, start=1):    # pdftoppm/tesseract sind 1-indiziert
        if _textlayer_ist_plausibel(seiten_text):
            teil_text = seiten_text
        else:
            teil_text, ocr_conf = _ocr_tesseract_seite(pfad, seiten_nr)
            for i, c in ocr_conf.items():
                conf_map[zeilen_offset + i] = c
        # rstrip VOR Zählen+Append (geteilter Bug-Fix mit kontoauszug_writer.lies_kontoauszug_pdf):
        # ein Textlayer-Segment endet meist schon auf "\n" — ungekürzt mit "\n".join() verbunden
        # entstünde an der Seitengrenze ein Doppel-"\n", das den Zeilenindex aller folgenden Zeilen
        # um 1 verschiebt -> conf_map zeigt auf die falsche Zeile.
        teil_text = teil_text.rstrip("\n")
        text_teile.append(teil_text)
        zeilen_offset += teil_text.count("\n") + 1
    return "\n".join(text_teile), conf_map


def _ocr_tesseract_seite(pfad: str, seiten_nr: int) -> tuple[str, dict]:
    """Ein-Seiten-tesseract-OCR (1-indiziert) für den Teil-Textlayer-Fall — analog
    kontoauszug_writer._ocr_tesseract_seite (geteiltes Muster, eigene Kopie: kein Cross-Modul-Import
    zwischen den zwei Writern)."""
    with tempfile.TemporaryDirectory() as td:
        praefix = os.path.join(td, "seite")
        subprocess.run(["pdftoppm", "-png", "-r", "200", "-f", str(seiten_nr), "-l", str(seiten_nr),
                        pfad, praefix], capture_output=True)
        png = sorted(os.listdir(td))[0]
        tsv = subprocess.run(["tesseract", os.path.join(td, png), "stdout", "-l", "deu", "tsv"],
                             capture_output=True, text=True).stdout
        zeilen = _tsv_zu_zeilen(tsv)
    text = "\n".join(z for z, _ in zeilen)
    conf_map = {i: c for i, (_, c) in enumerate(zeilen)}
    return text, conf_map


def _tsv_zu_zeilen(tsv: str) -> list[tuple[str, float]]:
    """tesseract --tsv-Ausgabe -> [(zeilentext, confidence 0..1)], gruppiert nach (block,par,line) —
    identische Logik zu kontoauszug_writer._tsv_zu_zeilen (geteiltes Muster, eigene Kopie)."""
    reader = csv.DictReader(io.StringIO(tsv), delimiter="\t")
    gruppen: dict[tuple, list] = {}
    reihenfolge = []
    for row in reader:
        if row.get("level") != "5":            # nur Wort-Zeilen (level 5 = word)
            continue
        key = (row["block_num"], row["par_num"], row["line_num"])
        if key not in gruppen:
            gruppen[key] = []
            reihenfolge.append(key)
        try:
            conf = float(row["conf"])
        except (TypeError, ValueError):
            conf = -1.0
        gruppen[key].append((row["text"], conf))
    out = []
    for key in reihenfolge:
        woerter = gruppen[key]
        zeilentext = " ".join(w for w, _ in woerter if w.strip())
        confs = [c / 100.0 for _, c in woerter if c >= 0]
        out.append((zeilentext, min(confs) if confs else 0.0))
    return out
