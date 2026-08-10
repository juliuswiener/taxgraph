"""Kontoauszug-Übernahme-Writer (import:kontoauszug — 4. Store-Writer). Deterministik-first, LLM-Fallback.

Workaround statt PSD2: der Nutzer LÄDT einen Kontoauszug HOCH (CSV/CAMT.053/MT940 = strukturiert;
PDF/Freitext = LLM). Transaktionen werden nach Steuer-Relevanz KLASSIFIZIERT und je relevanter Transaktion
als VORLAEUFIGES Event über den einen Store-Schreibpfad (`store.append_event`, herkunft=kontoauszug,
schreiber=import:kontoauszug) geschrieben. K2 = Chat-Berater-Grenze: die Klassifikation SCHLÄGT VOR, setzt
NIE einen Wert (der Store-Guard erzwingt vorlaeufig+signal_2=null); der Mensch bestätigt neben dem Auszug.

DETERMINISTIK-FIRST (Kosten $0 für den Großteil): strukturierte Formate parst ein deterministischer Parser,
klare Verwendungszwecke klassifiziert eine Keyword-Heuristik. Das LLM (OpenRouterClient, injiziert) ist NUR
Fallback für mehrdeutige Zwecke / unstrukturiertes PDF — es ist NICHT im deterministischen Pfad und wird in
den Unit-Tests NIE aufgerufen (llm_klassifikator=None). Privacy: nur Buchungstext+Betrag+Datum an das LLM,
IBAN/Kontonummern maskiert; lokal-first (faelle/), read-only.

FAIL-CLOSED: eine Transaktion OHNE sichere Kategorie ODER ohne existierendes Zielfeld ⇒ KEIN Vorschlag
(kein Rate-Wert). Nur EXPENSE-Transaktionen (betrag < 0, Ausgabe) werden für §35a/§10b/§10 klassifiziert.
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

# Kategorie -> Ziel-feld_id (NUR MVP-Kategorien mit EXISTIERENDEM Feld; Instructor-Ruling 2026-07-18).
# Andere Kategorien = benannte GAP (kein Vorschlag ohne Zielfeld).
KATEGORIE_FELD = {
    # § 35a-Ziele zeigen seit der Einzelaufstellung (2026-08-10) auf die Instanz-Basisfelder
    # (Instanz 1 = bare feld_id) statt auf die Sum-Kz — die Sum ist askable:false, kein Schreiber
    # darf sie mehr direkt setzen (_mit_ring_werten injiziert sie aus der Instanz-Σ).
    "handwerker":    "hh_handwerker_betrag",           # § 35a Abs. 3
    "dienstleistung": "hh_dienstleistung_betrag",       # § 35a Abs. 2
    "minijob":       "hh_minijob_betrag",               # § 35a Abs. 1
    "spende":        "spenden_betrag",                 # § 10b
    "vorsorge":      "vor_rv_ausserhalb_lstb",         # § 10 (Vorsorge außerhalb LStB)
}

# Keyword-Heuristik: erste passende Kategorie gewinnt. Bewusst konservativ (nur eindeutige Zwecke) —
# mehrdeutige Zeilen fallen durch (None) und gehen an das LLM (Fallback) bzw. bleiben unklassifiziert.
_KEYWORDS = [
    ("handwerker",    ("maler", "sanitär", "elektr", "dachdeck", "handwerk", "installat", "klempner",
                       "heizung", "renovier", "modernisier")),
    ("dienstleistung", ("reinigung", "putzhilfe", "gartenpfleg", "hausmeister", "gebäudereinig",
                        "fensterputz", "winterdienst")),
    ("minijob",       ("minijob", "haushaltshilfe", "minijob-zentrale")),
    ("spende",        ("spende", "zuwendung", "hilfswerk", "stiftung", " e.v", "e. v.", "tierheim",
                       "unicef", "rotes kreuz")),
    ("vorsorge",      ("rentenversicherung", "rürup", "basisrente", "altersvorsorge", "rürup-rente")),
]

_IBAN = re.compile(r"\b([A-Z]{2}\d{2})[\sA-Z0-9]{10,30}\b")
_KTO = re.compile(r"\b(\d{8,12})\b")


def maskiere(text: str) -> str:
    """IBAN/Kontonummern maskieren (Präfix + Rest verdeckt) — Privacy vor LLM/Speicherung."""
    if not text:
        return text
    t = _IBAN.sub(lambda m: m.group(1) + "****", text)
    t = _KTO.sub(lambda m: m.group(1)[:2] + "****", t)
    return t


def klassifiziere_det(verwendungszweck: str) -> str | None:
    """Deterministische Keyword-Klassifikation. None = unklar (→ LLM-Fallback / kein Vorschlag)."""
    z = (verwendungszweck or "").lower()
    for kategorie, keys in _KEYWORDS:
        if any(k in z for k in keys):
            return kategorie
    return None


def parse_csv(text: str) -> list[dict]:
    """Deterministischer CSV-Parser (Bank-Export). Erwartet Spalten datum/betrag/verwendungszweck
    (case-insensitiv, gängige Alias-Namen). Betrag in Euro (Komma/Punkt) → Cent (signed). Kein LLM."""
    out = []
    reader = csv.DictReader(io.StringIO(text), delimiter=";")
    if not reader.fieldnames:
        return out
    norm = {(f or "").strip().lower(): f for f in reader.fieldnames}
    def col(*aliases):
        for a in aliases:
            if a in norm:
                return norm[a]
        return None
    c_dat = col("datum", "buchungstag", "buchungsdatum", "date")
    c_bet = col("betrag", "umsatz", "amount", "betrag (eur)")
    c_zwk = col("verwendungszweck", "buchungstext", "zweck", "beschreibung", "description")
    for row in reader:
        if c_bet is None or not (row.get(c_bet) or "").strip():
            continue
        out.append({
            "datum": (row.get(c_dat) or "").strip() if c_dat else "",
            "betrag": _eur_cent_signed(row.get(c_bet, "")),
            "verwendungszweck": (row.get(c_zwk) or "").strip() if c_zwk else "",
        })
    return out


def _eur_cent_signed(s: str) -> int:
    """'-480,00' / '-480.00' / '1.234,56' → signed Cent."""
    s = (s or "").strip().replace("€", "").replace(" ", "")
    neg = s.startswith("-")
    s = s.lstrip("+-")
    if "," in s and "." in s:            # 1.234,56 (deutsch)
        s = s.replace(".", "").replace(",", ".")
    elif "," in s:                        # 480,00
        s = s.replace(",", ".")
    try:
        cent = int(round(float(s) * 100))
    except ValueError:
        return 0
    return -cent if neg else cent


_DATUM_ZEILE_RE = re.compile(r'^(\d{2}\.\d{2}\.\d{4})\s+(.*)$')
_BETRAG_TOKEN_RE = re.compile(r'[+-]?\d{1,3}(?:\.\d{3})*,\d{2}')
# Suffix-Match auf "saldo"/"kontostand" (kein Wortübergang VOR dem Keyword nötig) fängt Compound-
# Varianten wie "Tagessaldo"/"Gesamtsaldo"/"Kontokorrentsaldo"/"Anfangssaldo"/"Endsaldo" alle;
# "zwischensumme" bleibt Vollwort (killt "Rechnungssumme"/"Ratensumme"/"Bausparsumme"); "summe"/
# "übertrag" ganz raus (zu breit / treffen echte Umbuchungen wie "Dauerauftrag Übertrag ...").
_SALDO_KEYWORD_RE = re.compile(r'(kontostand|saldo)\b|\bzwischensumme\b', re.IGNORECASE)


def parse_pdf_zeilen(text: str, conf_map: dict, schwelle: float = 0.6) -> tuple[list[dict], int]:
    """Deterministischer PDF-Zeilen-Parser (Textlayer ODER OCR-Text aus lies_kontoauszug_pdf()).
    Summen-/Saldozeilen (Keyword) werden übersprungen (keine Transaktion, kein Zähler). Zeilen mit
    Datum DD.MM.YYYY vorn: genau 1 Betrags-Token im Rest + conf>=schwelle -> Vorschlag; 0 Beträge ->
    ignoriert (kein Betrag im Text); >1 Beträge (z.B. Saldo-Spalte) ODER conf<schwelle -> VERWORFEN
    (Lücke gezählt, NICHT geraten — K2 Under-tax > Over-tax).

    Bekannte Grenzen: (1) mehrdeutige Multi-Betrag-Zeilen (Saldo-Spalten-Layout) -> Lücke, bewusst kein
    Spalten-Raten (ein plausibel aussehender Falschwert ist schlechter als eine Lücke, die den Nutzer
    zur korrekten Handeingabe zwingt). Betrags-Extraktion aus Saldo-Layouts = Folge-Nachtrag sobald
    echte Bank-Samples vorliegen (Julius-Cap). (2) ein Komma-Dezimalwert im Zweck, der kein zweiter
    Betrag ist (z.B. Zinssatz "Sollzinsen 3,50% -45,00"), zählt als Multi-Betrag-Token und landet
    ebenfalls als sichtbare Lücke im Zähler — niedrige Prio, safe-by-omission, nicht extra gefixt."""
    transaktionen = []
    n_verworfen = 0
    for i, zeile in enumerate(text.splitlines()):
        z = zeile.strip()
        if not z:
            continue
        if _SALDO_KEYWORD_RE.search(z):
            # Zeile trägt ein Saldo-/Summen-Keyword. Reine Saldo-/Summenzeile (kein negativer Betrag)
            # -> stumm übersprungen (harmlos). Trägt sie TROTZDEM einen negativen Betrag, könnte es eine
            # echte Ausgabe sein, deren Zweck zufällig das Keyword enthält -> NICHT spurlos verschwinden,
            # sondern als sichtbare/auditierbare Lücke zählen (Transparenz-Regel dev-3).
            if any(b.startswith("-") for b in _BETRAG_TOKEN_RE.findall(z)):
                n_verworfen += 1
            continue
        m = _DATUM_ZEILE_RE.match(z)
        if not m:
            continue                                    # kein Datum vorn -> ignoriert
        rest = m.group(2)
        betraege = _BETRAG_TOKEN_RE.findall(rest)
        if not betraege:
            continue                                    # kein Betrag erkennbar -> ignoriert
        if len(betraege) > 1:
            n_verworfen += 1                             # mehrdeutig (z.B. Saldo-Spalte) -> NICHT raten
            continue
        conf = conf_map.get(i, 1.0)
        if conf < schwelle:
            n_verworfen += 1                             # unsicher (OCR) -> NICHT raten
            continue
        betrag_str = betraege[0]
        zweck = rest.replace(betrag_str, "", 1)
        zweck = re.sub(r'\s*(?:EUR|€)\s*$', '', zweck, flags=re.I)
        zweck = re.sub(r'\s+', ' ', zweck).strip()
        transaktionen.append({
            "datum": m.group(1),
            "betrag": _eur_cent_signed(betrag_str),
            "verwendungszweck": zweck,
        })
    return transaktionen, n_verworfen


# ---- PDF-Extraktion (Textlayer zuerst, Bild-Scan -> tesseract-deu-OCR; kein externer Dienst/API-Key) ------
# Reine Text-Extraktion -- speist NUR den bestehenden Transaktions-/Klassifikations-Pfad (parse_csv-Form),
# baut KEINE neue Klassifikation.

def _fix_bel(text: str) -> str:
    """Alt-Layer-Bug: manche PDF-Textlayer trennen Wörter mit U+0007 (BEL) statt Leerzeichen
    (`[[bgbl-scan-textlayer-ocr-route]]`)."""
    return text.replace("\x07", " ")


def _textlayer_ist_plausibel(seiten_text: str) -> bool:
    """Eine Seite gilt als plausibel textlayer-erfasst ab einer Mindest-Zeichendichte. Konservativ (K2:
    lieber unnötige Extra-OCR als eine still verpasste Bild-Seite) — killt sowohl leere Scan-Seiten
    (0 Zeichen) als auch Garbage-Reste (z.B. eine eingebettete Seitenzahl/Fußzeile als einziges echtes
    Textobjekt auf einer sonst gescannten Seite). Schwellwert ist reine Technik-Heuristik, kein
    Gesetzeswert.
    BACKLOG (non-blocking, dev-3-Review 2026-07-20): ein gescannter Kontoauszug mit einer echten
    Kopfzeile/Fußzeile ≥20 Zeichen (z.B. Bankname+Adresse) könnte hier fälschlich als "plausibel"
    durchgehen, obwohl der Tabelleninhalt selbst reines Bild ist -> stille OCR-Lücke. Braucht echte
    Scan-Samples zur Kalibrierung (Julius-Cap), siehe [[pdf-teil-textlayer-luecke]]."""
    return len(seiten_text.strip()) >= 20


def lies_kontoauszug_pdf(pfad: str) -> tuple[str, dict]:
    """PDF-Kontoauszug -> (text, confidence_map je Zeilenindex). Textlayer zuerst (pdftotext -layout,
    exakt, Confidence implizit 1.0 — Präzedenz beleg_writer.lies_beleg_text); BEL-Fix vor dem Leer-Check.
    Kein Text überhaupt -> echter Voll-Bild-Scan -> tesseract-deu (lokal, kein externer Dienst/API-Key).
    Teil-Textlayer (z.B. Seite 1 Textlayer + Seite 2 Scan): pdftotext trennt Seiten mit \\x0c (ein
    Trailing-\\x0c auch nach der letzten Seite, daher [:-1]) — jede implausible Seite wird EINZELN
    nach-OCR't, plausible Seiten behalten ihren Textlayer (kein unnötiges Voll-Rastern)."""
    text = subprocess.run(["pdftotext", "-layout", pfad, "-"], capture_output=True, text=True).stdout
    text = _fix_bel(text)
    if not text.strip():
        return _ocr_tesseract_zeilen(pfad)

    seiten = text.split("\x0c")[:-1]
    if all(_textlayer_ist_plausibel(s) for s in seiten):
        return text, {}

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
        # rstrip VOR Zählen+Append: ein Textlayer-Segment endet meist schon auf "\n" — würde das
        # UNGEKÜRZT mit "\n".join() verbunden, entstünde an der Seitengrenze ein zusätzliches Leerzeichen
        # (Doppel-"\n"), das den Zeilenindex ALLER folgenden Zeilen um 1 verschiebt -> conf_map zeigt
        # dann auf die falsche (leere) Zeile, die echte (evtl. unsichere) OCR-Zeile fällt auf den
        # conf_map.get(i,1.0)-Fallback = STILLE Voll-Konfidenz (unterläuft K2 conf<0.6->Lücke).
        teil_text = teil_text.rstrip("\n")
        text_teile.append(teil_text)
        zeilen_offset += teil_text.count("\n") + 1
    return "\n".join(text_teile), conf_map


def _ocr_tesseract_seite(pfad: str, seiten_nr: int) -> tuple[str, dict]:
    """Wie _ocr_tesseract_zeilen, aber NUR eine Seite (1-indiziert) rastern+OCR'n — für den
    Teil-Textlayer-Fall (der Rest der Seiten hat schon einen brauchbaren Textlayer)."""
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


def _ocr_tesseract_zeilen(pfad: str) -> tuple[str, dict]:
    """Bild-Scan: pdftoppm rastert jede Seite (200dpi), tesseract --tsv liefert Wort-Confidence je Zeile
    — min über die Zeilen-Wörter (EINE unsichere Ziffer disqualifiziert die ganze Zeile, konservativ:
    K2 Under-tax > Over-tax). Zeilen-Reihenfolge = Lesereihenfolge (block/par/line, Seiten sequenziell)."""
    with tempfile.TemporaryDirectory() as td:
        praefix = os.path.join(td, "seite")
        subprocess.run(["pdftoppm", "-png", "-r", "200", pfad, praefix], capture_output=True)
        seiten = sorted(f for f in os.listdir(td) if f.endswith(".png"))
        zeilen = []
        for seite in seiten:
            tsv = subprocess.run(["tesseract", os.path.join(td, seite), "stdout", "-l", "deu", "tsv"],
                                 capture_output=True, text=True).stdout
            zeilen.extend(_tsv_zu_zeilen(tsv))
    text = "\n".join(z for z, _ in zeilen)
    conf_map = {i: c for i, (_, c) in enumerate(zeilen)}
    return text, conf_map


def _tsv_zu_zeilen(tsv: str) -> list[tuple[str, float]]:
    """tesseract --tsv-Ausgabe -> [(zeilentext, confidence 0..1)], gruppiert nach (block,par,line)."""
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


# ---- LLM-Fallback-Schicht (nur für mehrdeutige Zwecke / PDF; OpenRouter-REUSE, kein neues Key-Handling) --
# Prompt zwingt JSON-Kategorie aus der MVP-Menge; das LLM SCHLÄGT VOR (K2), der Store-Guard erzwingt
# vorlaeufig. Modell-Slug = billig-schnell (Klassifikation ist simpel, kein Frontier) — Julius/Instructor
# wählt den finalen Slug. Getestet über dry_run+Recorded-Fixture (kein Mock).
_LLM_PROMPT = (
    "Du klassifizierst EINE Bank-Transaktion nach deutscher Steuer-Relevanz. Antworte NUR mit JSON "
    '{"kategorie": X} wobei X eines von: handwerker (§35a Handwerkerleistung), dienstleistung '
    "(§35a haushaltsnahe Dienstleistung), minijob (§35a Minijob im Haushalt), spende (§10b Zuwendung), "
    "vorsorge (§10 Altersvorsorge-Beitrag), oder null (keine dieser Kategorien / unklar). Kein Fließtext.")


def _parse_llm_kategorie(text: str) -> str | None:
    import json
    try:
        m = re.search(r"\{.*\}", text, re.S)
        k = (json.loads(m.group(0)) if m else {}).get("kategorie")
        return k if k in KATEGORIE_FELD else None
    except (ValueError, AttributeError):
        return None


def llm_klassifikator_factory(client, role, *, fixture_id: str | None = None):
    """Gibt einen Klassifikator(zweck, betrag) zurück, der EINEN OpenRouter-Call macht (dry_run→Fixture-
    Replay). Der Aufrufer (api.py/dev-1) baut client+role; hier nur der Adapter. Nie im deterministischen Pfad."""
    def klassifikator(zweck: str, betrag: int) -> str | None:
        msgs = [{"role": "system", "content": _LLM_PROMPT},
                {"role": "user", "content": f"Zweck: {zweck}\nBetrag: {betrag / 100:.2f} EUR"}]
        comp = client.complete(role, msgs, fixture_id=fixture_id)
        return _parse_llm_kategorie(comp.text)
    return klassifikator


def uebernehme_kontoauszug(store: dict, transaktionen: list[dict], bindung: dict, *,
                           llm_klassifikator=None, ts: str | None = None,
                           katalog: dict | None = None) -> int:
    """Je AUSGABEN-Transaktion (betrag < 0): Kategorie deterministisch, sonst LLM-Fallback (falls injiziert),
    sonst kein Vorschlag. Kategorie mit existierendem Ziel-Feld → ein vorlaeufiges Event (Vorschlag). Gibt
    die Anzahl geschriebener Vorschläge zurück. Kein Überschreiben aktiver Events; keine Aggregation (je
    Transaktion ein eigenständiger Vorschlag — bei mehreren derselben Kategorie gewinnt der Store-One-Active-
    Guard: nur die erste, weitere brauchen Nutzer-Merge = benannter Folge-Nachtrag Multi-Transaktion).

    `katalog` (optional) = der GLOBALE Feld-Katalog (dev-2-Kontrakt msg 4365: die Vorschlags-Autorisierung
    hängt am Feld, nicht an der Scheibe). Wird er nicht übergeben, fällt der Check auf den per-Scheibe-Katalog
    aus `bindung` zurück (rückwärts-kompatibel). Das TARGETING (feld in bindung, unten) bleibt bewusst
    per-Scheibe — nur die ENFORCEMENT-Untergrenze ist global (decoupled, defense-in-depth)."""
    katalog = katalog if katalog is not None else ST.lade_katalog(bindung)   # K1: import:kontoauszug nur kontoauszug-Felder
    aktiv = set(ST._aktives(store))
    n = 0
    for tx in transaktionen:
        betrag = int(tx.get("betrag", 0))
        if betrag >= 0:                            # nur Ausgaben sind § 35a/§ 10b/§ 10-Kandidaten
            continue
        zweck = tx.get("verwendungszweck", "")
        kategorie = klassifiziere_det(zweck)
        if kategorie is None and llm_klassifikator is not None:
            kategorie = llm_klassifikator(maskiere(zweck), betrag)   # LLM-Fallback (maskiert)
        feld = KATEGORIE_FELD.get(kategorie or "")
        if not feld or feld not in bindung or feld in aktiv:
            continue                               # kein Zielfeld / schon belegt -> kein Vorschlag
        sig1 = {"typ": "kontoauszug", "datum": tx.get("datum", ""), "betrag": betrag,
                "verwendungszweck": maskiere(zweck), "kategorie": kategorie,
                "quelle": "heuristik" if klassifiziere_det(zweck) else "llm"}
        ST.append_event(store, feld_id=feld, wert=abs(betrag), zustand="vorlaeufig",
                        herkunft={"herkunft": "kontoauszug", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="import:kontoauszug",
                        signal={"signal_1": sig1, "signal_2": None}, ts=ts, katalog=katalog)
        aktiv.add(feld)
        n += 1
    return n
