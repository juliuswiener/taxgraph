# UI-Politur — Daten-Naht-Doc + E2E-Szenario-Liste (dev-2, für dev-1s Ein-Rutsch-Bau)

**Rolle:** Datenlieferant. Kein api.py-Edit (dev-1s Haut-Zone). Dieses Doc = (A) die exakte Daten-FORM,
die dev-1s Badge-/Ketten-UI konsumiert + Vollständigkeits-Beleg, (B) die E2E-Szenarien als Kreuzprobe.

## (A) DATEN-NAHT — die Form, die dev-1s Haut konsumiert

### A1. Badge-Rohdaten — via GET /stand (api.py:583), je Feld:
`{wert, zustand, herkunft:{herkunft, pruef_tiefe, haftung}, herkunft_badge}`
- **zustand**: `vorlaeufig` | `bestaetigt` (Zwei-Signal; bestaetigt braucht signal_2).
- **herkunft.herkunft** (Enum store/schema.json): `laie · llm_vorschlag · beleg_import · vorjahr · berechnet · orakel`.
  → dev-1s reiche Badges mappen DIREKT auf diese Kategorie (statt binär _badge). Alle Rohdaten DA.
- **herkunft.pruef_tiefe**: `ungeprueft · plausibilisiert · orakel_bestaetigt · amtlich` (optionale Badge-Tiefe).
- KEINE Daten fehlen für Badges — nur die Ableitung (_badge binär) ist dev-1s Arbeit.

### A2. Folge-der-Kante — via justification() (api.py:655) + /ergebnis-Trace (api.py:688), je Feld:
`{feld_id, wert, zustand, herkunft, event_id, signal, regel_id, signatur_slot, geltungsbedingung, anker_ref}`
- **anker_ref** = NORM-Anker {quelle, zitatanker, datei} → die „→ Paragraph"-Kante. VOLLSTÄNDIG.
- **KORREKTUR zu meinem Vor-Zuschnitt:** die BELEG-Kante ist SCHON da — `signal.signal_1` trägt für
  beleg_import-Werte das **Beleg-Herkunfts-Objekt `{typ, ref, confidence, roh_text}`** (beleg_writer:151):
  - `typ` = Beleg-Typ (lstb/spende/handwerker/…), `ref` = Beleg-Positions-Anker (= herkunft_slots-Eintrag,
    z.B. „Lohnsteuerbescheinigung Nr. 3"), `confidence` = OCR-Konfidenz, `roh_text` = extrahierte Zeile.
  → justification liefert Euro → Regel(regel_id) → Norm(anker_ref) → Beleg(signal_1) KOMPLETT. dev-1
  rendert die Kette direkt aus dem justification-Objekt, KEINE Daten-Erweiterung nötig.
- **Optionaler Zusatz (nice-to-have, nicht Pflicht):** für LAIE-eingegebene Felder, die einen Beleg-Anker
  HÄTTEN, kann dev-1 `bindung[feld].herkunft_slots` als Hint zeigen („dieser Wert ließe sich aus
  Lohnsteuerbescheinigung Nr. 3 importieren"). Statisch aus der Bindung, kein Store-Bezug.

### A3. Vollständigkeits-Beleg (Beleg-importierbare Felder mit herkunft_slots):
9 Felder tragen den Beleg-Positions-Anker: bruttoarbeitslohn (LStB Nr.3), vor_an_anteil_rv (Nr.23 a/b),
vor_ag_anteil_rv (Nr.22 a/b), vor_rv_ausserhalb_lstb, vpf_keine_mahlzeitengestellung, hh_minijob_
aufwendungen, hh_dienstleistungen, hh_handwerker_arbeitskosten, spenden_betrag (Zuwendungsbestätigung).
Alle 92 Felder tragen den herkunft-Vektor (jeder Store-Wert). Store-Writer-Guards fail-closed:
`^llm:` + `^import:beleg` erzwingen vorlaeufig+signal_2=null (nie Direkt-Bestätigung durch KI/Beleg).

## (B) UI-E2E-SZENARIO-LISTE (Kreuzprobe für dev-1s e2e — dev-1 schreibt die Tests)

| # | Szenario | Eingabe/Aktion | Erwarteter UI-Zustand |
|---|---|---|---|
| U1 | 2-Kachel-Wahl → Ring erreichbar | Kachel „Arbeitnehmer/gesamt" bzw. „Rentner" wählen | Scheibe `gesamt` bzw. `rentner_gesamt` aktiv, deren Felder-Queue startet |
| U2 | bool-Prefill Normalfall | Frage „Hattest du Kapitalerträge?" (kein_kap) | Prefill zeigt **„Nein"** (beispielwert=False + dev-1 bool-select-Prefill); Submit → kein_kap=true (abwesend) |
| U3 | Badge je Herkunft | Feld laie-eingegeben / beleg_import / vorjahr | Badge unterscheidet Quelle (Selbst ✓ / Beleg / Vorjahr); llm_vorschlag = schimmernd „KI" |
| U4 | Ring-Schrumpf | mehr Felder bestätigen | /stand-Intervall (Spanne) schrumpft; bei vollem Kegel → feste Zahl (/ergebnis) |
| U5 | Beleg-Kette (Folge der Kante) | beleg_import-Wert (z.B. bruttoarbeitslohn aus LStB) → justification | zeigt Euro → §19 (anker_ref) → Beleg {typ lstb, ref „LStB Nr.3", confidence, roh_text} aus signal_1 |
| U6 | chat → 501-Sperre | POST auf die LLM-chat-Route | 501 (LLM-Freitext gesperrt, benannter späterer Cap) — fail-closed |
| U7 | Splitting-Kegel-Guard | veranlagung=zusammen, Partner-Pflichtfeld offen | kein halber Bescheid (grund partner_kegel_offen) |
| U8 | Rentner-Fixierung-Guard | gesetzl. Rente, renten_beginn<VZ, rentenfreibetrag fehlt | fail-closed (rentenfreibetrag_fixierung_offen), UI-Laientext |
| U9 | Partner-Behinderung live | zusammen + rentner_grad_der_behinderung_partner | partner_check lässt durch (kein Widerspruch); einzel + Partner-Feld → Widerspruch surfacet |
| U10 | Kapital-Semantik-Guard | E0121709-Aggregat UND Töpfe beide gesetzt | grund kapital_semantik_offen (kein additiv-Rate, fail-closed) |
| U11 | Herkunft antippbar → Kette expandiert (Dim 1) | Badge/Feld antippen | justification-Objekt expandiert: Euro → regel_id → anker_ref (Norm) → signal_1 (Beleg {typ,ref,confidence,roh_text}) |
| U12 | Hold-to-confirm bei KI-Konfidenz (Dim 2) | Hold-Geste auf einem llm_vorschlag-Wert (schimmernd) | Wert wird bestätigt: zustand vorlaeufig → bestaetigt (Hold liefert signal_2, Zwei-Signal); Badge schimmernd → solide |

## Hand-Kreuzprobe-Werte (für dev-1s e2e, engine-truth aus dem Sweep 2026-07-18)
- **U4 Ring-Schrumpf (reiner AN):** bruttoarbeitslohn 40000, VZ2025 einzel → voller Kegel = **festzusetzende_est 6919**.
  Vor vollem Kegel liefert /stand eine SPANNE (Intervall), die mit jedem bestätigten Feld schrumpft; bei
  vollem Kegel → feste Zahl (/ergebnis). Weitere Ring-Werte: die 120 Goldens sind engine-truth (Sweep grün).
- **Rentner-Ring (U8/U9):** S1 gesetzl-Erstjahr 811, S2 Folgejahr-mit-RF 458, **S3 Folgejahr-OHNE-RF →
  fail-closed grund `rentenfreibetrag_fixierung_offen`**, S4 private-Leibrente 346, S5 §33b-Kombo 568,
  S6 Ehegatte-Behinderung-zusammen 660 (partner_check live).
- **Guard-grund-Strings (exakt, die die UI surfacen muss, U7-U10):** `partner_kegel_offen`,
  `partner_vor_offen`, `rentenfreibetrag_fixierung_offen`, `kapital_semantik_offen`, `kein_scheiben_gesamtbescheid`.
- **U6 chat→501:** POST /chat + POST /elster-ampel → 501 `{"fehler": "not_implemented", …}` (CHAT_501, api.py:726) — fail-closed, benannter späterer Cap.
- **U3/U11 Badge-Enum:** herkunft ∈ {laie, llm_vorschlag, beleg_import, vorjahr, berechnet, orakel}; zustand ∈ {vorlaeufig, bestaetigt}; pruef_tiefe ∈ {ungeprueft, plausibilisiert, orakel_bestaetigt, amtlich}.

## Naht zu dev-1 / Zur Abnahme
- Kein Daten-Bau nötig: Badge-Rohdaten (/stand) + Beleg-Kette (justification.signal_1) sind KOMPLETT.
- dev-1 baut: reiche Badge-Ableitung, Ketten-UI (aus justification), bool-Prefill, Lab-Optik, e2e (gegen U1-U10).
- Offen an dich: (1) reicht die Daten-Naht so (kein herkunft_slots-Zusatz in justification nötig)?
  (2) E2E-Liste U1-U10 vollständig für Julius' Lab-Vision, oder Szenarien ergänzen?
