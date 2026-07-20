# Session-Handover — 2026-07-20 (Abend, ~20:35 GMT+2)

Übergabe an die nächste taxgraph-Lead/Instructor-Session. Branch `claude/implementation-start-ypyyqw`.
Lies zuerst diese Datei + `MEMORY.md` (+ verlinkte). Vorherige Übergabe: `2026-07-20-instructor-handover.md` (243f824).
Gleiche Maschine, gleicher lokaler Repo. **Nichts gepusht** (Push = Julius-direct im Chat).

---

## 1. WAS DIESE SESSION LANDETE (Headline: PDF-Import END-TO-END nutzbar)

5 Commits, linear auf `243f824` (Vorsession-Handover). Alle Voll-Suite-verifiziert (split-Gate wo e2e-http degradiert), doppelt gereviewt (dev-3-Real-Diff + verify-1-Voll-Suite).

| SHA | Front | Inhalt |
|---|---|---|
| `ba2922b` | K1-Live-Refactor (haut) | `llm_client.complete(role,msgs,fixture_id)` = EINE low-level Wahrheit; `vorschlaege`+kontoauszug-Klassifikator als Handler-Wrapper (api.py); kontoauszug()-Handler `llm_klassifikator=factory`; chat()-except verengt. fixture_id no-op (Replay lebt in pipeline/client.py). |
| `8c21e5b` | PDF-Extraktion (import) | `lies_kontoauszug_pdf` → (text, conf_map): Textlayer-first (pdftotext -layout + BEL-Fix) → tesseract-deu-OCR bei Scan; min-Confidence pro Zeile. |
| `442eb38` | PDF-Wiring end-to-end (import+haut) | `parse_pdf_zeilen` (K2-konservativ: Saldo-Suffix-Match, Multi-Betrag→Lücke, conf<0.6→Lücke, Transparenz-Zähler) + api.py-Endpoint (base64→tmp→finally-PII-unlink, verworfen/hinweis, except ValueError→400, 501-Stub raus). True-e2e. |
| `5a45fd1` | Haut-Front (haut) | PDF-Upload-UI-Wiring (accept+.pdf, `_dateiAlsBase64` readAsDataURL→base64, hinweis-Anzeige) + entfernung-except-Verengung 2-teilig (ors_client.geocode()-Shape-Guard ZUERST gegen 503→500-Regression, dann api.py import-vor-try+except-Verengung) + Cleanup (server.py/K3-Report/ENTFERNUNG_FALLBACK-Kommentar). |
| `4b03e7a` | Teil-Textlayer (import) | Multi-Seiten-Teil-Textlayer-Fix in kontoauszug_writer + beleg_writer (Pro-Seite-`\x0c`-Split + `_textlayer_ist_plausibel` ≥20 Zeichen + nur-implausible-Seiten-OCR); conf_map-Offset-Bug (dev-3-gefunden, Join-Extra-\n) gefixt via rstrip+unconditional-+1, non-vacuous alignment-Test beide Writer. est_mapping-Kommentar-Fix (E0500702). |

**PDF-Import ist jetzt für den Nutzer live:** UI akzeptiert PDF → base64 → Endpoint → pdftotext/tesseract-Extraktion → parse_pdf_zeilen → Store (vorläufig, Zwei-Signal). Backend war seit 442eb38 fertig, `5a45fd1` zog die UI nach (schloss den Loop).

## 2. K1/K3-LIVE READINESS (Code-fertig, wartet auf Julius-Config/Cap)

Detail-Sektion: dev-1-Recon (end-to-end gegen echten Code verifiziert). Kernaussage:

**K3-Live (ORS-Entfernung, §9 Abs.1 S.3 Nr.4):** GENAU 1 Julius-Schritt — `$ORS_API_KEY` in `.env.maps` → sofort scharf. Base-URL hartkodiert (offizieller ORS-Endpunkt), 0 Code, 0 Entscheidung. geocode() hat jetzt Shape-Guard (5a45fd1). Nutzer: Adresse → „Entfernung berechnen" → km als VORLÄUFIGER Vorschlag (Zwei-Signal). Kein Key/Netzfehler → sauberer 503-Fallback auf manuelle Eingabe, nie Fake/Crash.

**K1-Live (/chat + kontoauszug-LLM):** 0 Code, ABER 1 Config-ENTSCHEIDUNG — `.env.llm` braucht ALLE DREI: `LLM_API_KEY` + `LLM_API_BASE` (OpenAI-kompat Provider-Endpunkt) + `LLM_MODEL` (Modell-Slug). Julius muss Provider+Modell wählen (Client bewusst provider-agnostisch, kein Anbieter hartkodiert). complete()-Refactor (ba2922b) + /chat + kontoauszug-factory alle verdrahtet, except verengt. Sekundär-Risiko: `response_format:json_object` — Provider-ohne-Support → sauber LlmNichtVerfuegbar (kein Fake), ggf. Provider wechseln. Zwei-Signal strukturell erzwungen (KI setzt nie selbst einen Wert).

Beide: ohne Key → sauberer Fallback (K1→501, K3→503), NIE Mock/Fake/Crash. Echter Logik-Bug propagiert bewusst (except-Verengung).

## 3. NÄCHSTER BAUSTEIN — XSD-Kz-Verifikationspass (design-fertig, ~1-1.5h Build)

**Status: Design KOMPLETT + dev-3-BUILD-FREIGEGEBEN + gehärtet. NUR NOCH BAUEN.** Design-Doc: `reports/review/2026-07-20-xsd-kz-verifikationspass-design.md` (dev-2, 170+ Zeilen mit Pseudocode + empirischen Beweisen).

Zweck: alle `elster_kz` in `produkt/bindung/*.yaml` gegen die lokalen ELSTER-XSDs (`~/02_Software/eric/doc_extract/ERiC-44.2.4.0/…/E10-<VZ>.xsd`) abgleichen → fängt Kz-Tippfehler VOR echter Submission. Reiner Verifikations-/Drift-Report, NICHTS auto-editiert, jeder Fund → Instructor/Julius (Gesetzeswert-Disziplin).

**Soundness bewiesen (dev-3s kritische Frage widerlegt):** Sektions-Pfad via TOP-DOWN-Walk ab Schema-Wurzel = Kette LOKALER Element-Namen (complexType nur zum Reinschauen, NIE type→path-Reverse-Lookup, der bei Typ-Wiederverwendung mehrdeutig wäre). Beweis: synthetischer geteilter complexType (2 Elemente/1 Typ) → 2 korrekte je-Instanz-Pfade. Real-Schema-Probe E10-2025: K_Verh_A/B reproduziert, 2242 Kz, 11ms.

**Build-Anforderungen (dev-3, alle im Design-Doc §6):** H1 Gate-Exit-Code (exit 1 bei AMBIGUOUS/NOT_FOUND), H2 §1-Regressions-Fixture MUSS durch den VOLLEN Report-Treiber laufen + AMBIGUOUS assertieren (non-vacuous, wie beim conf_map-Fund), H3 MAX_DEPTH-Abbrüche zählen+sichtbar. attributeGroup kategorisch ausgeschlossen (führt nur xs:attribute), substitutionGroup 0-verifiziert.

Aufwand ~1-1.5h (Kernrisiko gelöst, Rest = lade_bindung()-Reuse + Report-Format + 2 Tests). **dev-2 baut es — Build-Go steht, war nur auf diesen Checkpoint verschoben.**

## 4. BACKLOG (non-cap, buildbar; + Julius-Cap)

- **beleg_writer Teil-Textlayer-Restrisiko** (non-cap, klein): `_textlayer_ist_plausibel` (≥20 Zeichen) kann durch eine Scan-Seite mit echter Kopf-/Fußzeile getäuscht werden (Tabelle bleibt Bild, nie OCR't, keine sichtbare Lücke). Code-Kommentar-verankert beide Writer, [[pdf-teil-textlayer-luecke]]. Braucht echte Scan-Samples zur Kalibrierung (Julius-Cap).
- **Deklaration nicht_material-Posten** (§10d-Deckelung >1Mio€ + ~8 weitere): für Bürger/Rentner/kleine-Selbständige immateriell → DEFERRED. Basis-§10d-Vortrag ist committet (6fa945a, echter Zielgruppenfall). NUR die >1Mio-Deckelungsformel fehlt = out-of-scope.
- **E0500702** (Kindergeld-Anspruch als 6. per-Kind-Feld): NICHT gebunden — Werte-Kodierung nicht amtlich verifizierbar (Platzhalter 2448) + schon 2026-07-18 abgelehnt. Kommentar in est_mapping korrigiert (5/6 per-Kind gebunden).
- **Julius-Cap:** §34-Stufe-2b (H 34.2 EStH-2021), eDaten-Import-Writer (eDaten-Kanal ELSTER-API/ERiC), echte Bank/Beleg-Samples, LLM-Provider+Key, ORS-Key, Push-Go.

## 5. SCHLÜSSEL-DISZIPLINEN (unverändert, bewährt diese Session)

- **[[falsches-gruen]]**: grüne Gates aktiv misstrauen — dev-3 fing diese Session 3 echte Bugs die targeted-grün überlebten (geocode 503→500-Regression, conf_map-Offset-off-by-1 unterlief conf<0.6-K2, XSD-Design-Soundness-Frage) + 2 vacuous-Tests. Non-Vacuous-Disziplin: Test MUSS ohne den Fix rot laufen (empirisch gegen buggy-revert prüfen).
- **[[instructor-gesetzeswert-nie-aus-gedaechtnis]]**: E0500702-Werte-Kodierung NICHT geraten (Platzhalter erkannt → GAP statt Fake-Wert).
- **K2**: kein silent-wrong; Under-tax > Over-tax; fail-closed. Teil-Textlayer-Silent-Vanish-Fund war K2 (unsichtbare Vollständigkeits-Lücke).
- **Commit-Mechanik**: agent-delegiert (`git commit -F`), HEAD-Guard + explizites `git add` je Datei (NIE `-A` — Tree hatte oft 2 Fronten parallel, disjunkte Zonen getrennt committet).
- **pytest-Race**: nur EIN Voll-Suite-Owner (verify-1); e2e-http degradiert (~13-17min, geleakter Daemon-Thread) — Split A (Bulk `--ignore=test_paket_b_e2e_http`) + B (e2e-http isoliert) zur Isolation/Attribution.
- **Non-strukturelle Commits** (UI/plumbing, kein Ring/Graph/bindung) auf targeted+Review committet, Voll-Suite folgt mit dem nächsten Commit (5a45fd1).

## 6. FLEET-STAND

4 orch-Worker (permanent), warm:
- **dev-1** (Ring/Haut, impl): Zone ERSCHÖPFT für diese Session (alle Fronten committed). Warm für K1/K3-Live bei Cap-Unblock (0 Code, nur Config).
- **dev-2** (Import/Deklaration, impl): baut als nächstes den XSD-Kz-Pass (design-ready). Danach beleg-Restrisiko / weitere Deklaration.
- **dev-3** (read-only Reviewer): Star-Reviewer diese Session (K1-Sweep + Whitelist + Person-B + 3 Bug-Fänge + Design-Reviews). Re-reviewt jeden Diff, fängt vacuous-Tests + Soundness-Löcher.
- **verify-1** (Bash Gate-Runner): einziger Voll-Suite-Owner, isolierte Worktrees + Split-Gate. Baseline-Worktree `.claude/worktrees/verify-baseline-60d1281` liegt noch (aufräumbar).

Muster diese Session: Scratchpad-Draft während Wartezeit (transplant-ready nach Commit), Design-File-Review-vor-Build (fing die XSD-Soundness-Frage vor dem ~3h-Bau, senkte auf ~1-1.5h), Menu-Recon für Nächste-Front-Entscheidung (kein Cap-Pause, reiche non-cap-Front gefunden).
