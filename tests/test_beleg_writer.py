"""Gate für den Beleg-Upload-Writer (produkt/import/, Stufe 1 LStB). Deterministisch, NULL LLM.

Prüft: (a) Beleg-Typ-Erkennung + Anker-Extraktion aus dem synthetischen Muster-LStB gegen die
erwarteten Kandidatenwerte; (b) fail-closed — der Writer schreibt NUR vorlaeufig (herkunft=beleg_import,
schreiber=import:beleg, signal_2=null), signal_1 trägt Beleg-Ref+confidence; (c) Confidence ändert NIE
den Zustand (keine Auto-Bestätigung); (d) unlesbar/nicht gefunden → benannte Lücke (kein Rate-Wert);
(e) Guard-Tamper — import:beleg + bestaetigt wirft (Runtime + Schema). Test-LStB ist SYNTHETISCH.
"""
from __future__ import annotations

import os
import shutil
import subprocess
import sys

import pytest

yaml = pytest.importorskip("yaml")
jsonschema = pytest.importorskip("jsonschema")

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for sub in ("produkt/import", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, sub))

import beleg_writer as BW   # noqa: E402
import store as ST         # noqa: E402
import traverser as TR     # noqa: E402

TS = "2026-07-18T09:00:00+00:00"


def _fix(name):
    return open(os.path.join(HERE, "fixtures", name), encoding="utf-8").read()


MUSTER = _fix("muster_lstb_2025.txt")
SPENDE = _fix("muster_zuwendungsbestaetigung_2025.txt")
HANDWERKER = _fix("muster_handwerkerrechnung_2025.txt")
HANDWERKER_OHNE_SPLIT = _fix("muster_handwerkerrechnung_ohne_split_2025.txt")
MINIJOB = _fix("muster_minijob_bescheinigung_2025.txt")
DIENSTLEISTUNG = _fix("muster_dienstleistungsrechnung_2025.txt")
AMBIG = _fix("muster_rechnung_ambig_2025.txt")


# ---- PDF-Fixture-Helfer (kein committetes Binary — on-the-fly, reproduzierbar; kopiert aus
# test_kontoauszug_writer.py, geteiltes Muster) --------------------------------------------------

def _write_text_pdf(zeilen: list, pfad: str) -> None:
    """Minimales ECHTES PDF mit Textlayer (roher Content-Stream, keine Lib nötig)."""
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
    """Bild-PDF OHNE Textlayer (PIL rendert Zeilen auf ein PNG, img2pdf verpackt)."""
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
    """Seite 1 = Textlayer, Seite 2 = Bild-Scan, via pdfunite zusammengefügt (Teil-Textlayer-Fixture,
    kein echtes Sample nötig)."""
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


@pytest.fixture(scope="module")
def store_schema():
    import json
    return json.load(open(os.path.join(ROOT, "produkt", "store", "schema.json")))


# ---- (a) Erkennung + Extraktion ----------------------------------------------

def test_erkenne_lstb():
    assert BW.erkenne_lstb(MUSTER) is True
    assert BW.erkenne_lstb("Irgendein anderer Beleg, Rechnung Nr. 5") is False


def test_lstb_felder_aus_herkunft_slots(bindung):
    felder = BW.lstb_felder(bindung)
    assert felder.get("bruttoarbeitslohn") == "3"
    assert felder.get("vor_ag_anteil_rv") == "22"
    assert felder.get("vor_an_anteil_rv") == "23"


def test_extrahiere_kandidaten(bindung):
    k = {x["feld_id"]: x for x in BW.extrahiere(MUSTER, bindung)}
    assert k["bruttoarbeitslohn"]["wert"] == 4500000        # 45.000,00 -> Cent
    assert k["vor_ag_anteil_rv"]["wert"] == 418500          # Nr. 22
    assert k["vor_an_anteil_rv"]["wert"] == 418500          # Nr. 23
    assert all(x["confidence"] == 1.0 for x in k.values())  # Textlayer-Default


def test_kein_beleg_typ_keine_extraktion(bindung):
    assert BW.extrahiere("Irgendein Kassenbon Supermarkt 12,34", bindung) == []


# ---- Stufe 1b: Beleg-Typ-Erkennung + Spende + Handwerker + Material-Guard -----

def test_erkenne_beleg_typ():
    assert BW.erkenne_beleg_typ(MUSTER) == "lstb"
    assert BW.erkenne_beleg_typ(SPENDE) == "spende"
    assert BW.erkenne_beleg_typ(HANDWERKER) == "handwerker"
    assert BW.erkenne_beleg_typ(HANDWERKER_OHNE_SPLIT) == "handwerker"   # erkannt, aber kein Arbeitskosten-Wert
    assert BW.erkenne_beleg_typ(MINIJOB) == "minijob"
    assert BW.erkenne_beleg_typ(DIENSTLEISTUNG) == "dienstleistung"
    assert BW.erkenne_beleg_typ(AMBIG) is None                           # §35a Abs.2/3 mehrdeutig → K2: nicht raten
    assert BW.erkenne_beleg_typ("Kassenbon 5,00") is None


def test_extrahiere_spende(bindung):
    k = {x["feld_id"]: x for x in BW.extrahiere(SPENDE, bindung)}
    assert k["spenden_betrag"]["wert"] == 30000                 # 300,00 EUR -> Cent
    assert k["spenden_betrag"]["beleg_typ"] == "spende" and k["spenden_betrag"]["anker"] == "Betrag"


def test_extrahiere_handwerker_nur_arbeitskosten(bindung):
    """Nur der getrennt ausgewiesene Arbeitskosten-Anteil (1.200) — NICHT Material (800) / Gesamt (2.000)."""
    k = {x["feld_id"]: x for x in BW.extrahiere(HANDWERKER, bindung)}
    assert k["hh_handwerker_betrag"]["wert"] == 120000   # 1.200,00, nicht 800/2000
    assert k["hh_handwerker_betrag"]["beleg_typ"] == "handwerker"


def test_neg_handwerker_ohne_split_material_luecke(bindung):
    """§35a-Missbrauchsschutz: Rechnung OHNE getrennte Arbeitskosten -> KEIN Wert (Lücke), nie Gesamt raten."""
    fids = {x["feld_id"] for x in BW.extrahiere(HANDWERKER_OHNE_SPLIT, bindung)}
    assert "hh_handwerker_betrag" not in fids            # keine Arbeitskosten-Zeile -> Lücke


def test_schreibe_spende_vorlaeufig(bindung):
    s = ST.leerer_store(2025, fall_id="spende-test")
    kand = BW.extrahiere(SPENDE, bindung)
    events = BW.schreibe_kandidaten(s, kand, beleg_ref="beleg:muster_zuwendung", bindung=bindung, ts=TS)
    assert len(events) == 1
    ev = events[0]
    assert ev["feld_id"] == "spenden_betrag" and ev["zustand"] == "vorlaeufig"
    assert ev["herkunft"]["herkunft"] == "beleg_import" and ev["schreiber"] == "import:beleg"
    assert ev["signal"]["signal_2"] is None and ev["signal"]["signal_1"]["typ"] == "beleg"


# ---- Stufe 1c: Minijob (§35a Abs.1) + Dienstleistung (§35a Abs.2) + Typ-Scoping/Fail-closed ----

def test_extrahiere_minijob_voll_kein_guard(bindung):
    """Minijob (§35a Abs.1): volle Aufwendungen (reine Lohn-/Haushaltsscheck-Kosten, keine Material-Zeile)."""
    k = {x["feld_id"]: x for x in BW.extrahiere(MINIJOB, bindung)}
    assert k["hh_minijob_betrag"]["wert"] == 250000        # 2.500,00
    assert k["hh_minijob_betrag"]["beleg_typ"] == "minijob"


def test_extrahiere_dienstleistung_nur_arbeitskosten(bindung):
    """Dienstleistung (§35a Abs.2): NUR Arbeitskosten (1.000), NICHT Material (200)/Gesamt (1.200)."""
    k = {x["feld_id"]: x for x in BW.extrahiere(DIENSTLEISTUNG, bindung)}
    assert k["hh_dienstleistung_betrag"]["wert"] == 100000            # 1.000,00, Material-Guard greift
    assert "hh_handwerker_betrag" not in k               # Typ-Scoping: NICHT das Handwerker-Feld


def test_typ_scoping_handwerker_nicht_dienstleistung(bindung):
    """Ein handwerker-Dokument füllt NUR hh_handwerker, nie hh_dienstleistung_betrag (Typ-Scoping)."""
    fids = {x["feld_id"] for x in BW.extrahiere(HANDWERKER, bindung)}
    assert "hh_handwerker_betrag" in fids and "hh_dienstleistung_betrag" not in fids


def test_neg_ambig_rechnung_fail_closed(bindung):
    """§35a Abs.2 (4.000€) vs Abs.3 (1.200€) mehrdeutig → NICHTS füllen (K2), Mensch entscheidet."""
    kand = BW.extrahiere(AMBIG, bindung)
    assert kand == []                                           # kein Feld geraten
    fids = {x["feld_id"] for x in kand}
    assert "hh_handwerker_betrag" not in fids and "hh_dienstleistung_betrag" not in fids


# ---- (b) fail-closed: Writer schreibt nur vorlaeufig --------------------------

def test_schreibe_vorlaeufig_beleg_import(bindung):
    s = ST.leerer_store(2025, fall_id="beleg-test")
    kand = BW.extrahiere(MUSTER, bindung)
    events = BW.schreibe_kandidaten(s, kand, beleg_ref="beleg:muster_lstb_2025", bindung=bindung, ts=TS)
    assert len(events) == 3
    for ev in events:
        assert ev["zustand"] == "vorlaeufig"
        assert ev["herkunft"]["herkunft"] == "beleg_import"
        assert ev["schreiber"] == "import:beleg"
        assert ev["signal"]["signal_2"] is None
        assert ev["signal"]["signal_1"]["typ"] == "beleg"
        assert "confidence" in ev["signal"]["signal_1"]
    # materialisiert -> die Werte stehen (als vorlaeufig) im Snapshot
    snap, _ = ST.materialisiere(s)
    assert snap["bruttoarbeitslohn"]["wert"] == 4500000
    assert snap["bruttoarbeitslohn"]["zustand"] == "vorlaeufig"


# ---- (c) Confidence ändert nie den Zustand -----------------------------------

def test_confidence_bleibt_vorlaeufig(bindung):
    s = ST.leerer_store(2025, fall_id="beleg-conf")
    kand = BW.extrahiere(MUSTER, bindung, confidence_map={"3": 0.99, "22": 0.20, "23": 0.55})
    events = BW.schreibe_kandidaten(s, kand, beleg_ref="beleg:x", bindung=bindung, ts=TS)
    for ev in events:                                       # egal wie hoch/niedrig die Confidence
        assert ev["zustand"] == "vorlaeufig"                # NIE Auto-Bestätigung (K2)
    conf = {ev["feld_id"]: ev["signal"]["signal_1"]["confidence"] for ev in events}
    assert conf["bruttoarbeitslohn"] == 0.99 and conf["vor_ag_anteil_rv"] == 0.20


# ---- (d) unlesbar -> benannte Lücke (kein Rate-Wert) -------------------------

def test_unlesbar_feld_wird_luecke(bindung):
    ohne_nr22 = "\n".join(z for z in MUSTER.splitlines() if "Nr. 22" not in z)
    fids = {x["feld_id"] for x in BW.extrahiere(ohne_nr22, bindung)}
    assert "vor_ag_anteil_rv" not in fids                   # nicht gefunden -> Lücke, kein Wert
    assert "bruttoarbeitslohn" in fids                      # der Rest bleibt extrahiert


# ---- (e) Guard-Tamper: import:beleg kann nicht bestätigen --------------------

def test_neg_import_beleg_kann_nicht_bestaetigen_runtime():
    s = ST.leerer_store(2025, fall_id="beleg-guard")
    with pytest.raises(ValueError, match="import:beleg"):
        ST.append_event(s, feld_id="bruttoarbeitslohn", wert=4500000, zustand="bestaetigt",
                        herkunft={"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
                        schreiber="import:beleg",
                        signal={"signal_1": {"typ": "beleg"}, "signal_2": "klick@beleg"}, ts=TS)


def _store_mit_event(ev: dict) -> dict:
    return {"version": 1, "veranlagungszeitraum": 2025, "events": [ev], "snapshots": []}


def _fehler(store_schema, ev: dict) -> list:
    """Validiert ein Event im vollen Store gegen das GESAMTE Schema (die $ref lösen nur so auf)."""
    V = jsonschema.Draft202012Validator(store_schema)
    return [e for e in V.iter_errors(_store_mit_event(ev)) if "events" in list(e.path)]


def test_neg_import_beleg_bestaetigt_schema_invalid(store_schema):
    """Schema-Guard (schreiber-scoped): import:beleg + bestaetigt ist ungültig (allOf-Regel)."""
    ev = {"event_id": "a" * 64, "ts": TS, "feld_id": "bruttoarbeitslohn", "wert": 4500000,
          "zustand": "bestaetigt",
          "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
          "schreiber": "import:beleg", "signal": {"signal_1": None, "signal_2": "klick"}, "ersetzt": None}
    assert _fehler(store_schema, ev), "import:beleg+bestaetigt müsste das Schema brechen"


def test_import_elster_beleg_import_bestaetigt_bleibt_gueltig(store_schema):
    """Regression: die Provenance beleg_import bleibt nach K3-Bestätigung erhalten — ein ANDERER
    Importer (import:elster-Vorfüllung) darf beleg_import+bestaetigt schreiben (Guard ist schreiber-,
    nicht herkunft-scoped)."""
    ev = {"event_id": "c" * 64, "ts": TS, "feld_id": "vor_an_anteil_rv", "wert": 3500000,
          "zustand": "bestaetigt",
          "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "plausibilisiert", "haftung": "nutzer"},
          "schreiber": "import:elster", "signal": {"signal_1": None, "signal_2": "lstb_z23"}, "ersetzt": None}
    assert not _fehler(store_schema, ev), "import:elster+beleg_import+bestaetigt sollte gültig bleiben"


def test_signal1_objekt_schema_gueltig(store_schema):
    """signal_1 darf ein Beleg-Herkunfts-Objekt sein (Schema-Erweiterung)."""
    ev = {"event_id": "b" * 64, "ts": TS, "feld_id": "bruttoarbeitslohn", "wert": 4500000,
          "zustand": "vorlaeufig",
          "herkunft": {"herkunft": "beleg_import", "pruef_tiefe": "ungeprueft", "haftung": "nutzer"},
          "schreiber": "import:beleg",
          "signal": {"signal_1": {"typ": "beleg", "ref": "beleg:x#lstb_nr=3", "confidence": 1.0},
                     "signal_2": None}, "ersetzt": None}
    assert not _fehler(store_schema, ev), "signal_1-Objekt sollte gültig sein"


# ---- lies_beleg_text: Textlayer-vs-Scan-Detektion + Teil-Textlayer (kein Mock) -----------------

def test_lies_beleg_text_textlayer(tmp_path):
    """Digitaler Beleg (echter Textlayer) -> pdftotext reicht, kein OCR, confidence_map leer."""
    pfad = str(tmp_path / "textlayer.pdf")
    _write_text_pdf(["Lohnsteuerbescheinigung 2025", "Nr. 3 45.000,00"], pfad)
    text, conf_map = BW.lies_beleg_text(pfad)
    assert "Lohnsteuerbescheinigung" in text
    assert conf_map == {}


@pytest.mark.skipif(not _MULTISEITEN_FIXTURE_OK, reason="Pillow/img2pdf/pdfunite für Multi-Seiten-Fixture nicht verfügbar")
def test_lies_beleg_text_teil_textlayer_seite2_wird_nachocrt(tmp_path):
    """Seite 1 Textlayer + Seite 2 Bild-Scan (kein OCR ohne Fix) -> beide Seiten im Ergebnis,
    conf_map trägt NUR für die nach-OCR'te Seite-2-Zeile einen Eintrag."""
    pfad = str(tmp_path / "teil_textlayer.pdf")
    _write_multiseiten_pdf(["Lohnsteuerbescheinigung 2025", "Nr. 3 45.000,00"],
                           ["Nr. 22 4.185,00"], pfad)
    text, conf_map = BW.lies_beleg_text(pfad)
    assert "Lohnsteuerbescheinigung" in text
    assert "22" in text                                   # Seite 2 sonst spurlos verloren
    assert conf_map
    assert all(0.0 <= c <= 1.0 for c in conf_map.values())


@pytest.mark.skipif(not _MULTISEITEN_FIXTURE_OK, reason="Pillow/img2pdf/pdfunite für Multi-Seiten-Fixture nicht verfügbar")
def test_lies_beleg_text_teil_textlayer_conf_index_alignment(tmp_path, monkeypatch):
    """dev-3-Fund (geteilter Bug mit kontoauszug_writer): Seitengrenze darf den conf_map-Index NICHT
    verschieben (Doppel-"\\n" beim Join wäre ein Offset-Bug) — eine niedrig-konfidente OCR-Zeile muss
    unter ihrem ECHTEN Index in text.splitlines() landen. _ocr_tesseract_seite ist deterministisch
    monkeypatched (Index-Arithmetik ist der Prüfgegenstand, nicht der OCR-Call)."""
    pfad = str(tmp_path / "conf_align.pdf")
    _write_multiseiten_pdf(["Lohnsteuerbescheinigung 2025", "Nr. 3 45.000,00"],
                           ["Platzhalter-Scan-Seite"], pfad)
    monkeypatch.setattr(BW, "_ocr_tesseract_seite", lambda pfad, seiten_nr: ("Nr. 22 4.185,00", {0: 0.4}))
    text, conf_map = BW.lies_beleg_text(pfad)
    zeilen = text.splitlines()
    ocr_idx = zeilen.index("Nr. 22 4.185,00")
    assert conf_map.get(ocr_idx) == 0.4                  # exakt ausgerichtet, kein Offset-Fehler
