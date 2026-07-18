# UI-Politur — Build-Design-Spec (dev-1 → Instructor → Julius)

Begleitet den HTML-Mockup **reports/review/2026-07-18-ui-mockup.html** (5 Phone-Screens + Dim-Legende).
Der Mockup zeigt die OPTIK; dieser Spec die TECHNIK (was sich in api.py/app.js ändert). Ein-Rutsch nach
Julius' Optik-Steuerung. Entscheide bereits fix: volle Lab-Vision, 2 Kacheln, mobile-first.

## Flow (mobile-first Wegpunkt)
Start-Screen (2 Kacheln) → POST /fall mit gewählter scheibe → Wegpunkt-Schleife (/fragen → Eingabe →
/event → /stand) → /ergebnis. Der Desktop-Graph (graph.html) bleibt Zusatzansicht.

## Technische Häppchen (Ein-Rutsch, Reihenfolge P0→P2)

### P0 — Erreichbarkeit (ohne das ist der Rechen-Ring UI-tot)
- **Scheiben-Wahl**: neuer Start-Screen in index.html/app.js; `start()` entfernt das hardcodierte
  `scheibe:"ep"` (app.js:28) → nimmt die Kachel-Wahl. 2 Kacheln → `gesamt` / `rentner_gesamt`. Server
  unverändert (fall_anlegen nimmt scheibe schon).
- **bool-Prefill**: app.js bool-select `option.selected = (String(o.value)===String(q.beispielwert))`.
  dev-2s beispielwert-Flips (kein_-Flags False, 800dca0) greifen automatisch. Reine Frontend-Änderung.

### P1 — Signatur-Optik (Herkunft + Ring)
- **Ring = schrumpfende Spanne (Dim 4)**: `#ring` zeigt statt Feld-Anteil die Bescheid-Spanne. Daten da:
  stand.intervall.{min_cent,max_cent}. Geometrie: äußerer Bogen = bestätigter Anteil (wächst), innere
  Fläche = (max−min)/startbreite (schrumpft auf Punkt). Reine app.js/CSS-Änderung, Server unverändert.
- **Per-Quelle-Badge (Dim 1)**: **_badge() (api.py:530) reicher machen** — aktuell binär
  solide/schimmernd. Ableitung aus herkunft.herkunft (6 Werte, store/schema.json): laie→"selbst"(grau),
  beleg_import→"beleg"(solide grün), vorjahr→"vorjahr", berechnet→"berechnet", orakel→"orakel",
  llm_vorschlag→"ki"(schimmernd violett). pruef_tiefe (ungeprueft/plausibilisiert/orakel_bestaetigt/
  amtlich) = optionale Tiefe-Badge + Konfidenz-Stufe für die Hold-Geste. app.js rendert die Badge-Klassen.
- **Beleg-Kette (Dim 1) — KEINE api.py-Erweiterung nötig** (dev-2-Korrektur, Naht-Doc A2): justification()
  gibt `signal.signal_1` schon zurück = für beleg_import das Beleg-Objekt {typ, ref, confidence, roh_text}.
  Die Kette Euro→regel_id→anker_ref(Norm)→signal_1(Beleg) ist KOMPLETT im justification-Objekt. app.js
  rendert sie im Bestätigungsmoment (Badge-tap). Optional nice-to-have: für laie-Felder mit
  bindung[feld].herkunft_slots ein „ließe sich aus LStB Nr.3 importieren"-Hint (statisch aus Bindung).

### P2 — Vertrauen/Erklärung
- **Chat-Slot (Dim 5)**: „💬 Erklär mir" gleichgroß NEBEN Bestätigen. Ruft POST /chat → **501 bleibt
  FEST** (KI-Sperre), UI zeigt den 501-Erklär-Vertrag als „KI erklärt, du entscheidest"-Platzhalter
  (kein Fake-200, kein Wert-Setzen). Situativ einblenden (Lab: ELSTER-Widerspruch/Vorjahres-Sprung) =
  später; MVP: immer sichtbar neben Bestätigen.
- **Hold-to-confirm (Dim 2)**: bei Badge=ki (mittlere Konfidenz via pruef_tiefe) → Halten-Geste
  (Fortschritts-Fill) statt Ein-Tipp; Beleg=solide → Ein-Tipp. Frontend-Geste, das /event-Zwei-Signal
  bleibt unverändert (der Klick/Hold ist signal_2). Schwellen = Design (Lab 85/60).

### P3 — Backlog (nicht Ein-Rutsch): Gesten-Richtung ins Audit-Log, ELSTER-Fehler-Pädagogik.

## Server-Berührung (minimal, meine Zone) — noch kleiner als gedacht
- **NUR _badge() (api.py:530)** — reicher, aus vorhandenem herkunft.herkunft (6 Werte). Das ist die
  EINZIGE api.py-Änderung.
- justification() UNVERÄNDERT — die Beleg-Kette (signal_1) ist schon in der Antwort (dev-2-Naht-Doc A2).
- Alles andere = app.js/index.html/style.css. KEINE Steuerlogik ins Frontend, KI-Sperre 501 fest,
  Server bindet 127.0.0.1, faelle/ gitignored — unverändert.

## Julius-Steuerung offen (Optik)
Farbwelt (Mockup = dunkel; hell möglich), Badge-Symbole (✓ Beleg / ✦ KI / grau selbst), Gesten-Grammatik
(lernen vs. entdecken), Chat immer-sichtbar vs. situativ, Ring-Farbe/Geometrie-Detail. Der Mockup ist
Vorschlag — Julius steuert, dann Ein-Rutsch-Bau. Freeze zu Instructor.
