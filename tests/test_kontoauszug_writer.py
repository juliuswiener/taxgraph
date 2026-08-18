"""Gate für den Kontoauszug-Writer (produkt/import/kontoauszug_writer.py) + Store-Guard
(^import:kontoauszug). Deterministisch, NULL LLM (die LLM-Fallback-Schicht wird über einen injizierten
PLAIN-Stub getestet — kein Mock eines LLM-Calls; die echte LLM-Extraktion prüft ein Recorded-Fixture-Test
separat). Cost = $0.

Prüft: CSV-Parse, Keyword-Klassifikation, IBAN-Maskierung, K2-Vorschlag-Fluss (nur Ausgaben, vorlaeufig,
herkunft=kontoauszug), fail-closed (unklassifiziert/kein Zielfeld/Einnahme = kein Vorschlag), Store-Guard.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/store", "produkt/traverser", "produkt/import"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import store as ST              # noqa: E402
import traverser as TR          # noqa: E402
import kontoauszug_writer as KW  # noqa: E402

TS = "2026-07-18T14:00:00+00:00"
KA = {"herkunft": "kontoauszug", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"}


# ---- PDF-Fixture-Helfer (kein committetes Binary — on-the-fly, reproduzierbar) -----------------

def _write_text_pdf(zeilen: list, pfad: str) -> None:
    """Minimales ECHTES PDF mit Textlayer (roher Content-Stream, keine Lib nötig) — für den
    Textlayer-Erkennungspfad (pdftotext liest es direkt, kein OCR)."""
    content = "BT /F1 11 Tf 50 750 Td 14 TL\n"
    for zeile in zeilen:
        esc = zeile.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)")
        content += f"({esc}) Tj T*\n"
    content += "ET"
    content_bytes = content.encode("latin-1")
    objs = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> "
        b"/MediaBox [0 0 612 792] /Contents 5 0 R >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        b"<< /Length %d >>\nstream\n" % len(content_bytes) + content_bytes + b"\nendstream",
    ]
    out = b"%PDF-1.4\n"
    offsets = [0]
    for i, o in enumerate(objs, start=1):
        offsets.append(len(out))
        out += f"{i} 0 obj\n".encode() + o + b"\nendobj\n"
    xref_off = len(out)
    out += f"xref\n0 {len(objs) + 1}\n0000000000 65535 f \n".encode()
    for off in offsets[1:]:
        out += f"{off:010d} 00000 n \n".encode()
    out += f"trailer\n<< /Size {len(objs) + 1} /Root 1 0 R >>\nstartxref\n{xref_off}\n%%EOF".encode()
    with open(pfad, "wb") as f:
        f.write(out)


def _write_scan_pdf(zeilen: list, pfad: str) -> None:
    """Bild-PDF OHNE Textlayer (PIL rendert Zeilen auf ein PNG, img2pdf verpackt) — erzwingt den
    tesseract-Fallback-Pfad (pdftotext liefert hier nichts)."""
    from PIL import Image, ImageDraw, ImageFont
    img = Image.new("RGB", (1000, 80 + 40 * len(zeilen)), "white")
    d = ImageDraw.Draw(img)
    font = ImageFont.truetype("/usr/share/fonts/TTF/DejaVuSans.ttf", 22)
    for i, zeile in enumerate(zeilen):
        d.text((20, 20 + 40 * i), zeile, fill="black", font=font)
    png_pfad = pfad + ".png"
    img.save(png_pfad)
    subprocess.run(["img2pdf", png_pfad, "-o", pfad], check=True)


def _write_multiseiten_pdf(text_zeilen: list, scan_zeilen: list, pfad: str) -> None:
    """Seite 1 = Textlayer (_write_text_pdf), Seite 2 = Bild-Scan (_write_scan_pdf), via pdfunite
    zusammengefügt (poppler-utils, schon Dependency von pdftotext/pdftoppm — kein neues Tool) — für den
    Teil-Textlayer-Pfad (kein echtes Bank-Sample nötig)."""
    p1, p2 = pfad + ".seite1.pdf", pfad + ".seite2.pdf"
    _write_text_pdf(text_zeilen, p1)
    _write_scan_pdf(scan_zeilen, p2)
    subprocess.run(["pdfunite", p1, p2, pfad], check=True)


try:
    import PIL  # noqa: F401
    _PIL_OK = True
except ImportError:
    _PIL_OK = False
_SCAN_FIXTURE_OK = _PIL_OK and shutil.which("img2pdf") is not None
_MULTISEITEN_FIXTURE_OK = _SCAN_FIXTURE_OK and shutil.which("pdfunite") is not None


@pytest.fixture(scope="module")
def bindung():
    return TR.lade_bindung()


# ---- CSV-Parse (deterministisch) --------------------------------------------

def test_parse_csv_deutsche_betraege():
    csv_text = ("Buchungstag;Betrag;Verwendungszweck\n"
                "12.03.2025;-480,00;Malermeister Schmidt Renovierung\n"
                "15.03.2025;-1.234,56;Spende Tierheim e.V.\n"
                "31.03.2025;2.500,00;Gehalt Arbeitgeber\n")
    tx = KW.parse_csv(csv_text)
    assert len(tx) == 3
    assert tx[0]["betrag"] == -48000 and "Malermeister" in tx[0]["verwendungszweck"]
    assert tx[1]["betrag"] == -123456
    assert tx[2]["betrag"] == 250000                 # Einnahme positiv


# ---- Keyword-Klassifikation --------------------------------------------------

def test_klassifiziere_det_eindeutig():
    assert KW.klassifiziere_det("Malermeister Schmidt Renovierung") == "handwerker"
    assert KW.klassifiziere_det("Spende Tierheim e.V.") == "spende"
    assert KW.klassifiziere_det("Beitrag Rürup-Rente") == "vorsorge"
    assert KW.klassifiziere_det("Gebäudereinigung Musterfirma") == "dienstleistung"


def test_klassifiziere_det_mehrdeutig_none():
    assert KW.klassifiziere_det("Überweisung Firma XY GmbH") is None    # → LLM-Fallback/kein Vorschlag


# ---- IBAN-Maskierung ---------------------------------------------------------

def test_maskiere_iban():
    m = KW.maskiere("Zahlung an DE89370400440532013000 Malermeister")
    assert "DE89****" in m and "0532013000" not in m


# ---- K2-Vorschlag-Fluss ------------------------------------------------------

def test_uebernahme_ausgaben_als_vorschlag(bindung):
    s = ST.leerer_store(2025, fall_id="ka")
    tx = [{"datum": "12.03.", "betrag": -48000, "verwendungszweck": "Malermeister Schmidt"},
          {"datum": "15.03.", "betrag": -20000, "verwendungszweck": "Spende Rotes Kreuz"},
          {"datum": "31.03.", "betrag": 250000, "verwendungszweck": "Gehalt"}]         # Einnahme
    n, _uebersprungen = KW.uebernehme_kontoauszug(s, tx, bindung, ts=TS)
    assert n == 2
    felder, _ = ST.materialisiere(s)
    assert felder["hh_handwerker_betrag"]["wert"] == 48000
    assert felder["hh_handwerker_betrag"]["zustand"] == "vorlaeufig"
    assert felder["hh_handwerker_betrag"]["herkunft"]["herkunft"] == "kontoauszug"
    assert felder["spenden_betrag"]["wert"] == 20000
    ev = ST._aktives(s)["hh_handwerker_betrag"]
    assert ev["signal"]["signal_1"]["kategorie"] == "handwerker" and ev["signal"]["signal_2"] is None


def test_unklassifiziert_kein_vorschlag(bindung):
    s = ST.leerer_store(2025, fall_id="ka-unklar")
    n, _uebersprungen = KW.uebernehme_kontoauszug(s, [{"datum": "1.1.", "betrag": -5000,
                                       "verwendungszweck": "Firma XY Zahlung"}], bindung, ts=TS)
    assert n == 0 and not ST._aktives(s)


def test_einnahme_kein_vorschlag(bindung):
    s = ST.leerer_store(2025, fall_id="ka-ein")
    n, _uebersprungen = KW.uebernehme_kontoauszug(s, [{"datum": "1.1.", "betrag": 300000,
                                       "verwendungszweck": "Spende erhalten"}], bindung, ts=TS)
    assert n == 0                                    # positive Beträge sind keine § 35a/Spenden-AUSGABE


# ---- LLM-Fallback über injizierten PLAIN-Stub (kein Mock eines LLM-Calls) ----

def test_llm_fallback_injiziert(bindung):
    s = ST.leerer_store(2025, fall_id="ka-llm")
    # ein plain-python-Stub simuliert NICHT den LLM-Call, er testet den Injektions-Punkt:
    stub = lambda zweck, betrag: "spende" if "kryptisch" in zweck.lower() else None
    tx = [{"datum": "1.1.", "betrag": -7500, "verwendungszweck": "KRYPTISCH-REF-4711"}]
    n, _uebersprungen = KW.uebernehme_kontoauszug(s, tx, bindung, llm_klassifikator=stub, ts=TS)
    assert n == 1
    ev = ST._aktives(s)["spenden_betrag"]
    assert ev["signal"]["signal_1"]["quelle"] == "llm" and ev["signal"]["signal_1"]["kategorie"] == "spende"


# ---- LLM-Antwort-Parsing (deterministisch, kein Live-Call) -------------------

def test_parse_llm_kategorie_json():
    assert KW._parse_llm_kategorie('{"kategorie": "spende"}') == "spende"
    assert KW._parse_llm_kategorie('Antwort: {"kategorie": "handwerker"} .') == "handwerker"
    assert KW._parse_llm_kategorie('{"kategorie": "unbekannt"}') is None      # nicht in MVP-Menge
    assert KW._parse_llm_kategorie('{"kategorie": null}') is None
    assert KW._parse_llm_kategorie('kein json') is None


def test_llm_replay_recorded_fixture(bindung):
    """Full-Round-Trip über die AUFGEZEICHNETE OpenRouter-Fixture (dry_run, kein Live-Call, kein Mock).
    SKIP bis die Fixture via 1 Cap-Call aufgezeichnet ist (Julius-Cap + Modell-Slug + openrouter.env-Key)."""
    client_mod = pytest.importorskip("client",
                                     reason="pipeline/client (OpenRouter) nicht importierbar")
    fixtures_dir = os.path.join(ROOT, "pipeline", "fixtures")
    if not os.path.exists(os.path.join(fixtures_dir, "kontoauszug_klassifikation.json")):
        pytest.skip("Recorded-Fixture kontoauszug_klassifikation.json noch nicht captured "
                    "(1 Julius-Cap-Live-Call + Modell-Slug-Wahl offen)")
    role = client_mod.RoleConfig(role="kontoauszug_klassifikation", slug="<slug>",
                                 providers=[], reason="kontoauszug", kind="klassifikation")
    client = client_mod.OpenRouterClient(dry_run=True, fixtures_dir=fixtures_dir)
    klass = KW.llm_klassifikator_factory(client, role, fixture_id="kontoauszug_klassifikation")
    assert klass("kryptischer Zweck", -5000) in (set(KW.KATEGORIE_FELD) | {None})


# ---- PDF-Extraktion: Textlayer-vs-Scan-Detektion + BEL-Fix + tesseract-OCR (kein Mock) --------

def test_fix_bel():
    assert KW._fix_bel("12.03.2025\x07Malermeister") == "12.03.2025 Malermeister"
    assert KW._fix_bel("kein bel hier") == "kein bel hier"


def test_lies_kontoauszug_pdf_textlayer(tmp_path):
    """Digitaler Bank-Export-PDF (echter Textlayer) -> pdftotext reicht, kein OCR, confidence_map leer."""
    zeilen = ["12.03.2025 Malermeister Schmidt Renovierung -480,00",
              "15.03.2025 Spende Rotes Kreuz e.V. -200,00"]
    pfad = str(tmp_path / "textlayer.pdf")
    _write_text_pdf(zeilen, pfad)
    text, conf_map = KW.lies_kontoauszug_pdf(pfad)
    for zeile in zeilen:
        assert zeile in text
    assert conf_map == {}                            # Textlayer: keine OCR-Confidence nötig


@pytest.mark.skipif(not _SCAN_FIXTURE_OK, reason="Pillow/img2pdf für Scan-PDF-Fixture nicht verfügbar")
def test_lies_kontoauszug_pdf_scan_ocr(tmp_path):
    """Echter Bild-Scan (kein Textlayer, PIL->PNG->img2pdf) -> tesseract-deu OCR (lokal, kein Mock,
    kein API-Key). Wort-Confidence je Zeile > 0 belegt den echten OCR-Pfad (nicht den Textlayer-Pfad)."""
    zeilen = ["12.03.2025 Malermeister Schmidt Renovierung -480,00",
              "15.03.2025 Spende Rotes Kreuz e.V. -200,00"]
    pfad = str(tmp_path / "scan.pdf")
    _write_scan_pdf(zeilen, pfad)
    text, conf_map = KW.lies_kontoauszug_pdf(pfad)
    assert "Malermeister" in text and "Spende" in text
    assert conf_map                                   # OCR-Pfad liefert Confidence je Zeilenindex
    assert all(0.0 <= c <= 1.0 for c in conf_map.values())
    assert max(conf_map.values()) > 0.6                # mindestens eine Zeile solide erkannt


@pytest.mark.skipif(not _MULTISEITEN_FIXTURE_OK, reason="Pillow/img2pdf/pdfunite für Multi-Seiten-Fixture nicht verfügbar")
def test_lies_kontoauszug_pdf_teil_textlayer_seite2_wird_nachocrt(tmp_path):
    """Seite 1 Textlayer + Seite 2 Bild-Scan (kein OCR ohne Fix) -> beide Transaktionen im Ergebnis,
    conf_map trägt NUR für die nach-OCR'te Seite-2-Zeile einen Eintrag (Seite-1-Zeilen bleiben implizit
    Confidence 1.0, kein unnötiges Voll-Rastern)."""
    text_zeilen = ["12.03.2025 Malermeister Schmidt Renovierung -480,00"]
    scan_zeilen = ["20.03.2025 Spende Rotes Kreuz -200,00"]
    pfad = str(tmp_path / "teil_textlayer.pdf")
    _write_multiseiten_pdf(text_zeilen, scan_zeilen, pfad)
    text, conf_map = KW.lies_kontoauszug_pdf(pfad)
    assert "Malermeister" in text
    assert "Spende" in text and "Rotes" in text          # Seite 2 sonst spurlos verloren
    assert conf_map                                       # OCR-Confidence für die nach-OCR'te Seite
    assert all(0.0 <= c <= 1.0 for c in conf_map.values())


@pytest.mark.skipif(not _MULTISEITEN_FIXTURE_OK, reason="Pillow/img2pdf/pdfunite für Multi-Seiten-Fixture nicht verfügbar")
def test_lies_kontoauszug_pdf_teil_textlayer_conf_index_alignment(tmp_path, monkeypatch):
    """dev-3-Fund: Seitengrenze darf den conf_map-Index NICHT verschieben (Doppel-"\\n" beim Join wäre
    ein Offset-Bug) — eine niedrig-konfidente OCR-Zeile muss unter ihrem ECHTEN Index in text.splitlines()
    landen, sonst fällt parse_pdf_zeilen's conf_map.get(i,1.0)-Fallback auf stille Voll-Konfidenz zurück
    und die Zeile wird NICHT verworfen (K2-Bruch). _ocr_tesseract_seite ist deterministisch
    monkeypatched (echte tesseract-Confidence für ein sauberes Synthetic-Bild ist unzuverlässig <0.6 zu
    erzwingen — die Index-Arithmetik selbst ist der Prüfgegenstand, nicht der OCR-Call)."""
    pfad = str(tmp_path / "conf_align.pdf")
    _write_multiseiten_pdf(["12.03.2025 Malermeister Schmidt Renovierung -480,00"],
                           ["Platzhalter-Scan-Seite"], pfad)
    monkeypatch.setattr(KW, "_ocr_tesseract_seite",
                        lambda pfad, seiten_nr: ("20.03.2025 Spende Rotes Kreuz -200,00", {0: 0.4}))
    text, conf_map = KW.lies_kontoauszug_pdf(pfad)
    zeilen = text.splitlines()
    ocr_idx = zeilen.index("20.03.2025 Spende Rotes Kreuz -200,00")
    assert conf_map.get(ocr_idx) == 0.4                  # exakt ausgerichtet, kein Offset-Fehler
    tx, verworfen = KW.parse_pdf_zeilen(text, conf_map)
    assert not any("Spende" in t["verwendungszweck"] for t in tx)   # <0.6 -> verworfen, nicht 1.0-Fallback
    assert verworfen == 1


# ---- parse_pdf_zeilen: Zeilen-Regex, Schwelle-Gate, Saldo-/Summenzeilen-Schutz (kein Mock) ----

def test_parse_pdf_zeilen_textlayer():
    text = ("12.03.2025 Malermeister Schmidt Renovierung -480,00\n"
            "15.03.2025 Spende Rotes Kreuz e.V. -200,00\n"
            "31.03.2025 Gehalt Arbeitgeber 2.500,00\n")
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 3 and verworfen == 0
    assert tx[0]["datum"] == "12.03.2025" and tx[0]["betrag"] == -48000
    assert "Malermeister" in tx[0]["verwendungszweck"]
    assert tx[1]["betrag"] == -20000
    assert tx[2]["betrag"] == 250000                     # Einnahme positiv, kein Vorzeichen im Text


def test_parse_pdf_zeilen_schwelle_gate():
    text = ("12.03.2025 Malermeister Schmidt Renovierung -480,00\n"
            "15.03.2025 Spende Rotes Kreuz e.V. -200,00\n")
    conf_map = {0: 0.9, 1: 0.4}                          # Zeile 1 (Index 1) unsicher
    tx, verworfen = KW.parse_pdf_zeilen(text, conf_map)
    assert len(tx) == 1 and verworfen == 1
    assert tx[0]["betrag"] == -48000                     # nur die sichere Zeile übernommen


def test_parse_pdf_zeilen_textlayer_kein_gate():
    text = "12.03.2025 Malermeister Schmidt Renovierung -480,00\n"
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 1 and verworfen == 0               # leere conf_map -> Schwelle nie aktiv


def test_parse_pdf_zeilen_saldo_spalte_und_summenzeile_konservativ():
    """Realistisches Layout: Anfangssaldo + Transaktion mit Saldo-Spalte (2 Beträge) + normale
    Transaktion + Endsaldo. Beweist: kein Phantom aus Saldo-Zeilen, Saldo wird NIE als Transaktions-
    betrag übernommen (Mehrdeutigkeit -> Lücke statt Rate-Wert)."""
    text = ("01.03.2025 Anfangssaldo 5.000,00\n"
            "12.03.2025 Miete Vermieter GmbH -800,00 4.200,00\n"
            "20.03.2025 Malermeister Schmidt -350,00\n"
            "31.03.2025 Endsaldo 3.850,00\n")
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 1 and verworfen == 1
    assert tx[0]["betrag"] == -35000 and tx[0]["datum"] == "20.03.2025"
    assert all(b["betrag"] not in (-80000, 420000) for b in tx)


def test_parse_pdf_zeilen_keine_falsche_multibetrag_verwerfung():
    """Rechnungsnummer/Mengenangabe ohne Komma-Dezimalform darf nicht als zweiter Betrag zählen."""
    text = ("12.03.2025 Rechnung Nr 2025-0042 Handwerker -150,00\n"
            "15.03.2025 3 Raten Ratenzahlung -150,00\n")
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 2 and verworfen == 0
    assert tx[0]["betrag"] == -15000 and tx[1]["betrag"] == -15000


def test_parse_pdf_zeilen_keyword_kollision_keine_stille_verwerfung():
    """"Übertrag"/"summe" als Wortfragment in echten Zwecken darf die Transaktion nicht schlucken."""
    text = ("15.03.2025 Dauerauftrag Übertrag Tagesgeldkonto -500,00\n"
            "20.03.2025 Zahlung Rechnungssumme Malerarbeiten -300,00\n")
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 2 and verworfen == 0
    assert tx[0]["betrag"] == -50000 and "Übertrag" in tx[0]["verwendungszweck"]
    assert tx[1]["betrag"] == -30000


def test_parse_pdf_zeilen_saldo_keyword_mit_negativbetrag_sichtbare_luecke():
    """Saldo-Keyword-Zeile MIT negativem Betrag verschwindet nicht spurlos, sondern zählt als Lücke."""
    text = "12.03.2025 Anpassung Saldo Korrektur -75,00\n"
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 0 and verworfen == 1


def test_parse_pdf_zeilen_compound_saldo_varianten():
    """Compound-Saldo-Varianten (Präfix vor "saldo") dürfen nicht als Phantom-Vorschlag durchgehen."""
    text = "31.03.2025 Tagessaldo -1.234,56\n"
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 0 and verworfen == 1               # negativer Betrag -> sichtbare Lücke, kein Vorschlag


def test_parse_pdf_zeilen_saldo_praefix_ohne_wortende_keine_kollision():
    """"saldo" als Wortanfang OHNE Wortende danach (z.B. Firmenname) triggert das Keyword NICHT —
    nur ein Suffix-Treffer (Präfix+"saldo" am Wortende) zählt als Saldo-Zeile."""
    text = "12.03.2025 Zahlung Saldotronic GmbH Beratung -220,00\n"
    tx, verworfen = KW.parse_pdf_zeilen(text, {})
    assert len(tx) == 1 and verworfen == 0
    assert tx[0]["betrag"] == -22000


# ---- Store-Guard fail-closed -------------------------------------------------

def test_guard_kontoauszug_kann_nie_bestaetigen():
    s = ST.leerer_store(2025, fall_id="ka-guard")
    with pytest.raises(ValueError, match="import:kontoauszug"):
        ST.append_event(s, feld_id="spenden_betrag", wert=20000, zustand="bestaetigt", herkunft=KA,
                        schreiber="import:kontoauszug",
                        signal={"signal_1": {"typ": "kontoauszug"}, "signal_2": "erschlichen"}, ts=TS)
