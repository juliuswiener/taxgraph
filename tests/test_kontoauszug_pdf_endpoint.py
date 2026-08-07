"""Kontoauszug-PDF-Endpoint (produkt/haut/api.py kontoauszug(), fmt=="pdf") — reiner Handler-Test:
base64-Decode/tmp-Datei/Response-Schema/PII-Cleanup. Der OCR/Text-Extraktions-Pfad selbst (pdftotext/
tesseract, kontoauszug_writer.lies_kontoauszug_pdf/parse_pdf_zeilen) ist dev-2s eigener Unit-Test —
hier NUR die Handler-Schicht, `lies_kontoauszug_pdf`/`parse_pdf_zeilen` sind plain Fixture-Fakes
(kein echtes PDF nötig, kein Mock, kein echter Call).

Deckt:
  - test_pdf_base64_upload_verworfen_feld ... 200, `verworfen`+`hinweis` bei unsicheren Zeilen,
                                               tmp-Datei ist NACH dem Call weg (PII-Cleanup-Beweis).
  - test_pdf_ungueltiges_base64_400 ......... kaputtes base64 → ApiError(400), kein Crash.
  - test_pdf_fehlender_inhalt_400 ........... leerer `inhalt` → ApiError(400).
  - test_csv_traegt_verworfen_0_ohne_hinweis  Response-Schema bleibt EINHEITLICH über alle Formate.
  - test_pdf_true_e2e_textlayer_extraktion .. TRUE e2e OHNE Fakes: echtes Text-Layer-PDF (dev-2s
                                               _write_text_pdf-Helfer) → base64 → Endpoint → ECHTE
                                               pdftotext-Extraktion → echte parse_pdf_zeilen → Store.
                                               Deckt die Naht Endpoint→Extraktion, die die obigen
                                               Fake-Tests bewusst NICHT prüfen.
"""
from __future__ import annotations

import base64
import os
import sys
import tempfile

import pytest

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
for _sub in ("produkt/haut", "produkt/import", "produkt/store", "produkt/traverser"):
    sys.path.insert(0, os.path.join(ROOT, _sub))

import api as API                    # noqa: E402
import kontoauszug_writer as KW      # noqa: E402
import store as ST                   # noqa: E402
from test_kontoauszug_writer import _write_text_pdf  # noqa: E402 -- dev-2s echtes-PDF-Helfer, kein Mock


@pytest.fixture
def fall(tmp_path, monkeypatch):
    """Ein leerer gesamt-Fall in einem tmp-FAELLE-Verzeichnis (kein Zugriff auf echte Fälle)."""
    monkeypatch.setattr(API, "FAELLE", str(tmp_path))
    st, _ = API.fall_anlegen({"scheibe": "gesamt", "veranlagungszeitraum": 2025, "fall_id": "pdf1"})
    assert st == 201
    return "pdf1"


def _tmp_pdfs():
    """Nur die tmp-Dateien, die der Endpunkt selbst anlegen kann: tempfile.mkstemp(suffix=".pdf")
    in api.py:2552, also gettempprefix() + Zufall + ".pdf".

    Vorher verglich der Test GANZ /tmp und sah damit jeden Fremdprozess. Einmal rot mit
    `assert {'go-build2607090677'} == set()` — ein paralleler Go-Compiler, nicht der Endpunkt.
    Die Einschraenkung schwaecht den PII-Guard nicht, sie schaerft ihn: geleakt waere genau
    eine Datei dieser Form."""
    pre = tempfile.gettempprefix()
    return {n for n in os.listdir(tempfile.gettempdir())
            if n.startswith(pre) and n.endswith(".pdf")}


def test_pdf_base64_upload_verworfen_feld(fall, monkeypatch):
    """base64-kodiertes PDF → 200, Response trägt `verworfen`+`hinweis` bei unsicheren Zeilen; die
    tmp-Datei existiert NACH dem Call nicht mehr (PII-Leck-Guard, Bank-PDF darf nie liegen bleiben)."""
    monkeypatch.setattr(KW, "lies_kontoauszug_pdf", lambda pfad: ("fake-text", {0: 0.4}))
    monkeypatch.setattr(
        KW, "parse_pdf_zeilen",
        lambda text, conf_map, schwelle=0.6: (
            [{"datum": "2025-03-15", "betrag": -120000, "verwendungszweck": "Spende Verein"}], 1))
    vor = _tmp_pdfs()
    b64 = base64.b64encode(b"%PDF-1.4 fake").decode("ascii")
    st, body = API.kontoauszug(fall, {"format": "pdf", "inhalt": b64})
    assert st == 200
    assert body["transaktionen"] == 1 and body["verworfen"] == 1
    assert "hinweis" in body
    assert _tmp_pdfs() - vor == set(), (
        "der Endpunkt hat seine tmp-PDF liegen lassen — das ist der ROHE Bank-Auszug "
        "(IBAN/PII vor der Writer-Maskierung). api.py:2559 os.unlink im finally pruefen.")


def test_pdf_ungueltiges_base64_400(fall):
    with pytest.raises(API.ApiError) as exc:
        API.kontoauszug(fall, {"format": "pdf", "inhalt": "!!!nicht-base64!!!"})
    assert exc.value.status == 400


def test_pdf_fehlender_inhalt_400(fall):
    with pytest.raises(API.ApiError) as exc:
        API.kontoauszug(fall, {"format": "pdf", "inhalt": ""})
    assert exc.value.status == 400


def test_csv_traegt_verworfen_0_ohne_hinweis(fall):
    """Response-Schema bleibt EINHEITLICH über alle Formate (kein format-abhängiges Schema)."""
    csv = "datum;betrag;verwendungszweck\n15.03.2025;-50,00;Spende Verein\n"
    st, body = API.kontoauszug(fall, {"format": "csv", "inhalt": csv})
    assert st == 200
    assert body["verworfen"] == 0
    assert "hinweis" not in body


def test_pdf_true_e2e_textlayer_extraktion(fall, tmp_path):
    """TRUE e2e OHNE Fakes: echtes Text-Layer-PDF (pdftotext, kein tesseract) → base64 → Endpoint →
    echte Extraktion (lies_kontoauszug_pdf + parse_pdf_zeilen) → Store. Deckt die Naht Endpoint→
    Extraktion, die die gefakten Tests oben bewusst nicht prüfen."""
    pfad = str(tmp_path / "echt.pdf")
    _write_text_pdf(["15.03.2025 Spende Rotes Kreuz e.V. -200,00"], pfad)
    with open(pfad, "rb") as f:
        b64 = base64.b64encode(f.read()).decode("ascii")
    st, body = API.kontoauszug(fall, {"format": "pdf", "inhalt": b64})
    assert st == 200
    assert body["transaktionen"] == 1
    assert body["verworfen"] == 0
    assert "hinweis" not in body
    aktiv = ST._aktives(API.lade_fall(fall))
    assert aktiv["spenden_betrag"]["wert"] == 20000
    assert aktiv["spenden_betrag"]["signal"]["signal_1"]["quelle"] == "heuristik"
