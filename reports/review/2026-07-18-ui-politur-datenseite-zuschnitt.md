# UI-Politur DATEN-SEITE — Stufe-A-Recon (dev-2)

**Rolle:** die Daten-Seite, die dev-1s Haut/Optik konsumiert. concept-first, KEIN Bau. Koordiniert an der
UI-Daten-Naht mit dev-1s Haut-Recon. LLM-frei.

## (1) HERKUNFTS-BADGES — Daten DA, Badge-Ableitung binär
- **herkunft-Vektor komplett + exponiert:** Store liefert je Feld {wert, zustand, herkunft:{herkunft,
  pruef_tiefe, haftung}}; /stand (api.py:583) gibt {wert, zustand, herkunft, herkunft_badge} an die UI.
- **herkunft-Enum** (store/schema.json): [laie, llm_vorschlag, beleg_import, vorjahr, berechnet, orakel].
  Badge-Quellen laie / beleg_import / vorjahr sind DA. **kontoauszug FEHLT** — KONZEPT_BELEGQUELLEN.md
  dokumentiert es als GENAU EINE künftige neue Kategorie (Paket-B-Kontoauszug-Stufe, PSD2). = benannter GAP.
- **GAP (Haut-Ableitung):** `_badge()` (api.py:530) kollabiert auf BINÄR — „schimmernd" (llm_vorschlag) vs
  „solide" (sonst). Für reiche per-Quelle-Badges (Beleg / Vorjahr / Selbst / KI + vorlaeufig/bestätigt +
  pruef_tiefe amtlich/plausibilisiert) braucht es eine reichere Badge-Ableitung. **Die Daten TRAGEN es
  schon** (herkunft.herkunft + zustand + pruef_tiefe) — nur die Ableitung ist binär. = dev-1-Haut auf
  bestehende Daten. Meine Zone = herkunft-Vektor korrekt befüllt (ist er, Store-Writer-Guards fail-closed).

## (2) FOLGE DER KANTE (Euro → Paragraph → Beleg) — Norm DA, Beleg-Kante fehlt
- **Exponiert:** per-Feld justification (api.py:655) + /ergebnis Vorwärts-Trace (trace_ergebnis, api.py:688).
- justification() liefert: wert, zustand, herkunft-Vektor, regel_id, signatur_slot, geltungsbedingung,
  **anker_ref (NORM)**. → Euro → Regel → Norm-Anker VOLLSTÄNDIG (die „zum Paragraphen"-Kante).
- **GAP: herkunft_slots (BELEG-Anker) NICHT in justification()** → die „→ Beleg"-Kante (aus welcher
  Beleg-Position der Wert stammt) fehlt im Output. Für beleg_import-Werte liegt der Beleg-Bezug im
  Store-signal_1 (beleg_writer confidence/Anker), ist aber nicht strukturiert durchgereicht.
  → VORSCHLAG (traverser-Naht): justification() um `herkunft_slots` (aus bindung) + den Beleg-signal-Bezug
  ergänzen, damit die Justification-UI „Euro → §X → Beleg-Zeile Y" vollständig zeigen kann.
- **GAP: per-Cent-Attribution** = benannter Nachtrag (trace_ergebnis-docstring / KONZEPT.md) — heute
  regel/slot/feld-exakt, nicht cent-genau. Für „welcher Cent kommt woher" später.

## (3) BINDUNGSTABELLE-VOLLSTÄNDIGKEIT (UI-Render) — VOLLSTÄNDIG
Je Feld für die UI-Anzeige da: fragetext_laie/hilfe_kurz/beispielwert (auditiert+committet 800dca0),
elster_kz + elster_kz_grund (Kz-Anzeige/Tooltip), anker_ref (Norm-Zitat), einheit, typ, bereich/enum_werte
(Eingabe-Validierung), herkunft_slots (Beleg-Mapping). Kein Render-Loch.

## (4) GAPS-Zusammenfassung (was UI-Gestaltung braucht vs. was da ist)
| Bedarf UI | Status | Owner |
|---|---|---|
| Badge-Rohdaten (Quelle + zustand + pruef_tiefe) je Feld | DA (herkunft-Vektor, /stand) | — |
| Reiche per-Quelle-Badge-Ableitung (statt binär) | GAP — _badge() binär, Daten tragen es | dev-1 Haut |
| kontoauszug-Herkunftskategorie | GAP (künftig, Paket-B, dokumentiert) | Nachtrag |
| Euro→Regel→Norm-Anker (justification) | DA (api.py:655/688) | — |
| Euro→Beleg-Anker (herkunft_slots in justification) | GAP | traverser-Naht (dev-2/geteilt) |
| per-Cent-Attribution | GAP (Nachtrag) | traverser-Nachtrag |
| Feld-Render-Daten (Laientext/Kz/Anker/Einheit) | VOLLSTÄNDIG | — |

## Meine Zone-Beiträge (Vorschlag, auf OK)
1. **traverser.justification() um herkunft_slots + Beleg-signal-Bezug erweitern** (die „→ Beleg"-Kante) —
   falls traverser meine/geteilte Zone; sonst Naht mit dev-1. Kleiner additiver Output-Zusatz + Test.
2. herkunft-Vektor bleibt korrekt (Store-Writer-Guards) — kein Bau nötig.
3. kontoauszug-Kategorie + per-Cent = benannte Nachträge (nicht diese Runde).

## Naht zu dev-1 (Haut-Recon)
dev-1 baut: reiche Badge-Ableitung (aus dem vorhandenen herkunft-Vektor), Lab-Optik, bool-select-Prefill
(#4). Ich liefere: den vollständigen herkunft-Vektor + (auf OK) die erweiterte justification mit Beleg-Kante.
Treffpunkt = /stand (Badges) + /ergebnis/justification (Folge-der-Kante). Melde existiert/fehlt oben.

## Zur Abnahme
(1) Soll ich justification() um die Beleg-Kante (herkunft_slots + Beleg-signal) erweitern — ist traverser
meine/geteilte Zone oder dev-1? (2) kontoauszug + per-Cent bleiben Nachträge? (3) Badge-Ableitung + Lab =
dev-1, ich nur Datenlieferant? → dann Bau/Naht-Meldung.
