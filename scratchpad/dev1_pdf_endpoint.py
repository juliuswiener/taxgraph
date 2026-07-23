"""DRAFT (Pre-Review, NICHT integriert) — dev-1s geplanter PDF-Increment für produkt/haut/api.py
kontoauszug(). Referenziert dev-2s NOCH NICHT gebaute `KW.parse_pdf_zeilen` (kontoauszug_writer.py)
und `KW.lies_kontoauszug_pdf` (Tuple-Rückgabe `(text, conf_map)`, laut Instructor bereits vorhanden).
NICHT lauffähig als Standalone-Skript — reiner Transplant-Entwurf für api.py, wartet auf Build-Go
NACH complete()-Commit + dev-2s parse_pdf_zeilen-Commit.

Transplant-Diff (konzeptuell) gegen die aktuelle kontoauszug()-Funktion:

  1. Neuer Import am Dateikopf von api.py: `import base64` (fehlt bisher; `tempfile` ist schon
     importiert).
  2. Der bestehende `elif fmt == "pdf": return 501, KONTOAUSZUG_PDF_501`-Zweig (aktuell api.py:
     1564-1565) wird durch den PDF-Zweig unten ersetzt (`n_verworfen` wird VOR dem katalog-Aufruf
     auf 0 initialisiert, damit csv/json denselben Response-Key OHNE Sonderfall tragen).
  3. `KONTOAUSZUG_PDF_501` wird danach toter Code -> vor dem Löschen Grep-Beweis
     (`grep -rn "KONTOAUSZUG_PDF_501" produkt/ tests/`), Guardrail-2-Stil.

Sicherheits-Pflicht (nicht optional): die tmp-Datei trägt den ROHEN Bank-Kontoauszug (PII/IBAN vor
Writer-Maskierung) -> IMMER im `finally` löschen, auch bei Exception im OCR/Parse-Pfad.
"""

# --- Ausschnitt: neuer PDF-Zweig innerhalb kontoauszug(), ersetzt den 501-Stub -------------------
#
#     n_verworfen = 0
#     if fmt == "csv":
#         tx = KW.parse_csv(inhalt if isinstance(inhalt, str) else "")
#     elif fmt == "json":
#         try:
#             tx = inhalt if isinstance(inhalt, list) else json.loads(inhalt or "[]")
#         except (ValueError, TypeError):
#             raise ApiError(400, "json-Inhalt nicht parsebar")
#         if not isinstance(tx, list):
#             raise ApiError(400, "json muss eine Liste von Transaktionen sein")
#     elif fmt == "pdf":
#         if not isinstance(inhalt, str) or not inhalt.strip():
#             raise ApiError(400, "pdf-Inhalt fehlt (erwartet: base64-kodierte PDF-Bytes in `inhalt`)")
#         try:
#             pdf_bytes = base64.b64decode(inhalt, validate=True)
#         except ValueError:              # binascii.Error IST eine ValueError-Unterklasse (verifiziert)
#             raise ApiError(400, "pdf-Inhalt nicht gueltig base64-kodiert")
#         fd, pfad = tempfile.mkstemp(suffix=".pdf")
#         try:
#             with os.fdopen(fd, "wb") as fh:
#                 fh.write(pdf_bytes)
#             text, conf_map = KW.lies_kontoauszug_pdf(pfad)
#             tx, n_verworfen = KW.parse_pdf_zeilen(text, conf_map)
#         finally:
#             os.unlink(pfad)          # PII/IBAN-tmp-Datei nie liegen lassen -- auch bei Exception
#     else:
#         raise ApiError(400, "format muss csv, json oder pdf sein")
#
# --- Ausschnitt: Response-Bau (ersetzt die letzte Zeile der Funktion) ----------------------------
#
#     n = KW.uebernehme_kontoauszug(store, tx, bindung, llm_klassifikator=_kontoauszug_llm_klassifikator(),
#                                   katalog=ST.lade_katalog(TR.lade_bindung()))
#     speichere_fall(fall_id, store)
#     out = {"uebernommen": n, "transaktionen": len(tx), "verworfen": n_verworfen}
#     if n_verworfen > 0:
#         out["hinweis"] = f"{n_verworfen} Zeile(n) unsicher erkannt (Confidence < 60%) — bitte manuell pruefen/nachtragen."
#     return 200, out
#
# Geklärt (Instructor, verifiziert):
#   - `except ValueError` allein genuegt fuer base64.b64decode-Fehlformat: `binascii.Error` IST eine
#     ValueError-Unterklasse (issubclass(binascii.Error, ValueError) == True) -- KEIN `import binascii`
#     noetig (dev-3-Fund: die binascii-Referenz ohne Import haette NameError statt 400 geworfen).
#   - `schwelle` bleibt fest 0.6, NICHT per Body ueberschreibbar (K2-Sicherheitsparameter, kein
#     Nutzer-Tuning). Falls je gewuenscht: Folge-Nachtrag.


# --- Ausschnitt: e2e-Test-Entwurf (wandert nach tests/test_paket_b_e2e_http.py oder eigene Datei) -
#
# def test_kontoauszug_pdf_base64_upload_verworfen_feld(fall, monkeypatch, tmp_path):
#     """base64-kodiertes PDF -> 200, Response traegt `verworfen` (>0 bei confidence < schwelle),
#     tmp-Datei existiert NACH dem Call nicht mehr (PII-Cleanup-Beweis)."""
#     pdf_bytes = _minimal_test_pdf_bytes()   # TODO: Fixture -- kein echtes Bank-PDF, synthetisch
#     b64 = base64.b64encode(pdf_bytes).decode("ascii")
#     vor_tmp = set(os.listdir(tempfile.gettempdir()))
#     st, body = API.kontoauszug(fall, {"format": "pdf", "inhalt": b64})
#     assert st == 200
#     assert "verworfen" in body
#     if body["verworfen"] > 0:
#         assert "hinweis" in body
#     nach_tmp = set(os.listdir(tempfile.gettempdir()))
#     assert nach_tmp - vor_tmp == set()   # keine liegen gebliebene PDF-tmp-Datei (PII-Leck-Guard)
#
# def test_kontoauszug_pdf_ungueltiges_base64_400():
#     st, body = API.kontoauszug(fall, {"format": "pdf", "inhalt": "!!!nicht-base64!!!"})
#     assert st == 400
#
# def test_kontoauszug_csv_json_tragen_verworfen_0_konsistenz():
#     """Response-Schema bleibt EINHEITLICH ueber alle Formate (kein format-abhaengiges Schema)."""
#     csv = "datum;betrag;verwendungszweck\n15.03.2025;-50,00;Spende Verein\n"
#     st, body = API.kontoauszug(fall, {"format": "csv", "inhalt": csv})
#     assert st == 200
#     assert body["verworfen"] == 0
#     assert "hinweis" not in body
