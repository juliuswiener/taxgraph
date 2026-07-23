"""SCRATCHPAD-Draft v2 (dev-2, Pre-Review nach dev-3-Fund). NICHT importiert, NICHT Teil des Trees —
zum Transplantieren nach kontoauszug_writer.py + test_kontoauszug_writer.py sobald main das Go gibt.

parse_pdf_zeilen(text, conf_map, schwelle=0.6) -> (transaktionen, n_verworfen)

v1-BUG (dev-3-Fund): Anker-Regex mit `$`+lazy `.+?` konnte (a) Saldo-/Summenzeilen als Phantom-
Transaktion lesen, (b) bei Saldo-Spalten-Layout den Saldo statt des echten Betrags greifen.

v2-FIX (K2-konservativ, im Zweifel Lücke statt raten):
  1. Summen-/Saldozeilen (Keyword-Blacklist) -> übersprungen, KEIN Zähler (keine Transaktion, keine Lücke).
  2. >1 Betrags-Token in der Zeile (z.B. Transaktionsbetrag + Saldo-Spalte) -> VERWORFEN (Lücke),
     NICHT geraten welcher der beiden der Transaktionsbetrag ist.
  3. Nur bei GENAU 1 Betrag + keine Summenzeile + conf>=schwelle wird ein Vorschlag gebaut.
Konsequenz: eine mehrdeutige Zeile verliert die Transaktion als LÜCKE (Mensch trägt nach) statt sie
mit falschem oder geratenem Betrag zu übernehmen — der Saldo wird NIE als Transaktionsbetrag genommen,
weil bei Mehrdeutigkeit gar kein Betrag genommen wird (safe-by-omission, nicht Disambiguierung).
"""
from __future__ import annotations

import re

# ---- Ziel-Platzierung: direkt unter parse_csv() in kontoauszug_writer.py (gleicher Deterministik-Block) ---

_DATUM_ZEILE_RE = re.compile(r'^(\d{2}\.\d{2}\.\d{4})\s+(.*)$')
_BETRAG_TOKEN_RE = re.compile(r'[+-]?\d{1,3}(?:\.\d{3})*,\d{2}')
# v3 (dev-3-Fund #2): Wortgrenze statt Substring (killt "Rechnungssumme"/"Ratensumme"/"Bausparsumme"),
# "summe"/"übertrag" ganz raus (zu breit / treffen echte Umbuchungen wie "Dauerauftrag Übertrag ...").
_SALDO_KEYWORD_RE = re.compile(r'\b(kontostand|saldo|anfangssaldo|endsaldo|zwischensumme)\b')


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
        if _SALDO_KEYWORD_RE.search(z.lower()):
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


# _eur_cent_signed wird aus kontoauszug_writer.py REUSED (bereits vorhanden, Zeile 104) — hier nur
# für den Standalone-Test des Scratchpads dupliziert, NICHT beim Transplantieren erneut definieren.
def _eur_cent_signed(s: str) -> int:
    s = (s or "").strip().replace("€", "").replace(" ", "")
    neg = s.startswith("-")
    s = s.lstrip("+-")
    if "," in s and "." in s:
        s = s.replace(".", "").replace(",", ".")
    elif "," in s:
        s = s.replace(",", ".")
    try:
        cent = int(round(float(s) * 100))
    except ValueError:
        return 0
    return -cent if neg else cent


# ============================================================================================
# Geplante Unit-Tests (Ziel: test_kontoauszug_writer.py, Block "PDF-Extraktion" nach
# test_lies_kontoauszug_pdf_textlayer) — 4 Tests (3 Basis + 1 neuer Saldo/Summenzeilen-Fixture-Test).
# ============================================================================================

def _draft_tests():
    """Nur zur Pre-Review lokal lauffähig. Beim Transplantieren: als `def test_...():` mit `KW.` statt
    lokalem Aufruf, assert-Bodies unverändert übernehmen."""

    # 1) Regex-Parse auf Textlayer-Zeilen (conf_map={} -> immer 1.0, kein Gate)
    text = ("12.03.2025 Malermeister Schmidt Renovierung -480,00\n"
            "15.03.2025 Spende Rotes Kreuz e.V. -200,00\n"
            "31.03.2025 Gehalt Arbeitgeber 2.500,00\n")
    tx, verworfen = parse_pdf_zeilen(text, {})
    assert len(tx) == 3 and verworfen == 0
    assert tx[0]["datum"] == "12.03.2025" and tx[0]["betrag"] == -48000
    assert "Malermeister" in tx[0]["verwendungszweck"]
    assert tx[1]["betrag"] == -20000
    assert tx[2]["betrag"] == 250000                     # Einnahme positiv, kein Vorzeichen im Text

    # 2) Schwelle-Gate: synthetische conf_map mit einer Zeile <0.6 -> Lücke (verworfen++, kein Eintrag)
    text2 = ("12.03.2025 Malermeister Schmidt Renovierung -480,00\n"
             "15.03.2025 Spende Rotes Kreuz e.V. -200,00\n")
    conf_map2 = {0: 0.9, 1: 0.4}                          # Zeile 1 (index 1) unsicher
    tx2, verworfen2 = parse_pdf_zeilen(text2, conf_map2)
    assert len(tx2) == 1 and verworfen2 == 1
    assert tx2[0]["betrag"] == -48000                     # nur die sichere Zeile übernommen

    # 3) Textlayer-Pfad: conf_map={} -> Schwelle nie angewendet (kein Verwerfen möglich)
    text3 = "12.03.2025 Malermeister Schmidt Renovierung -480,00\n"
    tx3, verworfen3 = parse_pdf_zeilen(text3, {})
    assert len(tx3) == 1 and verworfen3 == 0

    # 4) NEU: realistisches Layout mit Saldo-Spalte + Summenzeilen — beweist kein Phantom aus
    #    Saldo-/Summenzeilen UND dass eine mehrdeutige Saldo-Spalten-Zeile als Lücke verworfen wird
    #    (der Saldo wird NIE als Transaktionsbetrag übernommen — weder korrekt noch falsch geraten).
    text4 = ("01.03.2025 Anfangssaldo 5.000,00\n"                        # Summenzeile -> skip, kein Zähler
             "12.03.2025 Miete Vermieter GmbH -800,00 4.200,00\n"        # Transaktion+Saldo-Spalte (2 Beträge)
             "20.03.2025 Malermeister Schmidt -350,00\n"                 # normale Transaktion (1 Betrag)
             "31.03.2025 Endsaldo 3.850,00\n")                           # Summenzeile -> skip, kein Zähler
    tx4, verworfen4 = parse_pdf_zeilen(text4, {})
    assert len(tx4) == 1 and verworfen4 == 1              # kein Phantom aus Saldo-Zeilen, Ambiguität = Lücke
    assert tx4[0]["betrag"] == -35000 and tx4[0]["datum"] == "20.03.2025"
    assert all(b["betrag"] not in (-80000, 420000) for b in tx4)   # Saldo NIE als Transaktionsbetrag übernommen

    # 5) NEU: Nicht-Betrags-Zahlen im Zweck (Rechnungsnummer, Mengenangabe) duerfen NICHT faelschlich
    #    als Multi-Betrag-Zeile verworfen werden — _BETRAG_TOKEN_RE matcht NUR Komma-Dezimal-Beträge
    #    (\d…,dd), keine bloßen Integer/Rechnungsnummern ohne Komma-Nachkommastellen.
    text5 = ("12.03.2025 Rechnung Nr 2025-0042 Handwerker -150,00\n"
             "15.03.2025 3 Raten Ratenzahlung -150,00\n")
    tx5, verworfen5 = parse_pdf_zeilen(text5, {})
    assert len(tx5) == 2 and verworfen5 == 0
    assert tx5[0]["betrag"] == -15000 and tx5[1]["betrag"] == -15000

    # 6) NEU (v3, dev-3-Fund #2): Keyword-Kollisionen dürfen echte Transaktionen NICHT spurlos
    #    schlucken. "Übertrag" (echte Umbuchung) ist kein Blacklist-Keyword mehr; "Rechnungssumme"
    #    matcht "summe" nicht mehr, weil "summe" komplett raus ist (+ Wortgrenze würde es ohnehin killen).
    text6 = ("15.03.2025 Dauerauftrag Übertrag Tagesgeldkonto -500,00\n"
             "20.03.2025 Zahlung Rechnungssumme Malerarbeiten -300,00\n")
    tx6, verworfen6 = parse_pdf_zeilen(text6, {})
    assert len(tx6) == 2 and verworfen6 == 0
    assert tx6[0]["betrag"] == -50000 and "Übertrag" in tx6[0]["verwendungszweck"]
    assert tx6[1]["betrag"] == -30000

    # 7) NEU (v3): Saldo-Keyword-Zeile MIT negativem Betrag verschwindet nicht spurlos, sondern wird
    #    sichtbar als Lücke gezählt (Transparenz-Regel) — kein Vorschlag, aber auditierbar.
    text7 = "12.03.2025 Anpassung Saldo Korrektur -75,00\n"
    tx7, verworfen7 = parse_pdf_zeilen(text7, {})
    assert len(tx7) == 0 and verworfen7 == 1

    print("draft-tests OK: 7/7")


if __name__ == "__main__":
    _draft_tests()
