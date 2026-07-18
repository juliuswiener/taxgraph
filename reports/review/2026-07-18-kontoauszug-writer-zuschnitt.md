# Kontoauszug-Writer (Workaround statt PSD2) — Stufe-A-Zuschnitt

**Status:** concept-first, KEIN Bau. 4. Store-Writer (nach laie · import:beleg · import:vorjahr). hosted-LLM
als Start (Julius), REUSE der bestehenden OpenRouter-Infra. Der Live-LLM-Call ist eine ausgehende Aktion
(Finanzdaten verlassen das Gerät) → Julius-Cap [[ausgehende-aktionen-nur-julius]]. LLM-frei bis auf die
Klassifikations-Schicht.

## Architektur: 4. Writer im Beleg-Writer-Muster (kein Umbau)
`produkt/import/kontoauszug_writer.py`, Store-Schreibpfad `store.append_event(schreiber=import:kontoauszug,
herkunft=kontoauszug, zustand=vorlaeufig, signal_2=null)`. Kette:
```
Upload (CSV/CAMT.053/MT940/PDF) → PARSE → Transaktionen [{datum, betrag, verwendungszweck}]
→ KLASSIFIKATION je Transaktion (Steuer-Kategorie: §35a-Handwerker / §10b-Spende / §10-Vorsorge / …)
→ je relevante Transaktion 1 VORSCHLAG (feld_id + betrag) als vorlaeufiges Event
```
- **Neue herkunft-Kategorie:** Enum (store/schema.json) hat kontoauszug NOCH NICHT [laie, llm_vorschlag,
  beleg_import, vorjahr, berechnet, orakel] → +`kontoauszug` (genau EINE neue Kategorie, wie KONZEPT_BELEGQUELLEN vorsah).
- **Store-Guard:** `^import:kontoauszug ⇒ herkunft=kontoauszug + vorlaeufig + signal_2=null` (K2, symmetrisch
  zu ^import:beleg/^import:vorjahr, store.py).
- **signal_1** = {typ: kontoauszug, datum, betrag, verwendungszweck (maskiert), klassifikation, confidence} —
  Justification-Kante „aus Auszug: 12.03. −480€ ‚Malermeister Schmidt' → § 35a Handwerker".

## Deterministik-vs-LLM-SPLIT (Kosten-minimal, Empfehlung)
NICHT LLM-für-alles. Zwei Achsen:
1. **PARSE:** strukturierte Formate (CSV mit bekanntem Bank-Schema, CAMT.053/MT940-XML) → DETERMINISTISCHER
   Parser ($0, kein LLM). NUR PDF/unbekanntes Freitext-Layout → LLM-Universal-Parse (OpenRouterClient).
2. **KLASSIFIKATION** (Verwendungszweck → Steuer-Kategorie): erst DETERMINISTISCHE Heuristik (Keyword-Map:
   „Handwerker/Maler/Sanitär"→§35a, „Spende/Zuwendung/e.V."→§10b, „Beitrag/Versicherung"→§10), LLM nur als
   FALLBACK für mehrdeutige Zwecke. Das drückt den LLM-Anteil auf die wirklich unklaren Zeilen.
→ Effekt: die häufigen Fälle (Bank-CSV + klare Zwecke) laufen $0/deterministisch; LLM nur PDF-Parse +
mehrdeutige Klassifikation. Julius' „LLM-basiert" bleibt erfüllt (LLM ist da wo's nötig ist), nur nicht verschwenderisch.

## TEST-STRATEGIE (LLM-Cost-bewusst, MOCK VERBOTEN)
- **Deterministisch (kein LLM):** CSV/CAMT-Parse + Keyword-Klassifikation → normale Unit-Tests mit
  Sample-Auszügen (synthetische Fixtures, keine echten Kontodaten).
- **LLM-Schicht:** via AUFGEZEICHNETE Response-Fixture — EIN echter OpenRouter-Call einmal captured →
  als JSON-Fixture abgelegt, in CI deterministisch replayed (KEIN Mock — echte Antwort-Struktur). +
  Store-Guard/K2-Tests (kontoauszug schreibt NIE bestaetigt) rein deterministisch.
- **EIN Live-Integrationstest** (opt-in-Marker, Cap-bewusst, Julius-Wort) der einen echten Call macht —
  nie in der Standard-CI-Suite. Muster wie der GETTSIM-importorskip-Gate.

## Cap-Kosten-Schätzung
- Parse deterministisch + Heuristik-Klassifikation: **$0** (kein LLM) für Bank-CSV + klare Zwecke (Großteil).
- LLM-Klassifikation der mehrdeutigen Zeilen: EIN batch-Call je Auszug (~50-200 Transaktionen, gebündelter
  Prompt Buchungstext+Betrag → Kategorie, ~1-3k Tokens) ≈ **$0,001-0,01/Auszug**. PDF-Parse ähnlich.
- CI: **$0** (Recorded-Fixture). Live-Test: **1 Cap-bewusster Call** auf Julius-Wort.

## PRIVACY (nicht verhandelbar)
- Finanzdaten → hosted LLM = raus. Julius akzeptiert hosted-als-Start; MINIMIEREN: nur Buchungstext+Betrag
  an das LLM, **IBAN/Kontonummer maskiert** (Prefix+Länge, nie voll), keine Namen mehr als nötig.
- **Lokal-first:** Auszug + Store bleiben faelle/ (gitignored, 127.0.0.1). Kein Cloud-Upload ausser dem
  LLM-Klassifikations-Call selbst. **Read-only** (kein PSD2/Zahlung — das ist der Upload-Workaround).
- **K2 = Chat-Berater-Grenze:** das LLM SCHLÄGT VOR, setzt NIE einen Wert. Keine Steuer-Zahl bewegt sich bis
  Zwei-Signal (Nutzer bestätigt die Transaktion neben dem Auszug). Identisch zur beleg/chat-Grenze.

## Bau-Umfang nach OK
1. store.py: ^import:kontoauszug-Guard + schema.json herkunft-Enum +kontoauszug.
2. produkt/import/kontoauszug_writer.py: CSV/CAMT-Parser (det.) + Keyword-Klassifikator (det.) +
   OpenRouterClient-Fallback (LLM) + append_event je Vorschlag.
3. Klassifikations-Keyword-Map + feld_id-Ziel je Kategorie (§35a→hh_handwerker_arbeitskosten, §10b→spenden_betrag …).
4. Tests: det. Parse/Klassifikation + Recorded-LLM-Fixture + Guard/K2 + 1 opt-in Live-Test.

## Zur Abnahme
(1) Split det.-Parse + Heuristik-Klassifikation + LLM-Fallback OK, oder LLM-für-alles (teurer)? (2) OpenRouter-
Reuse via pipeline/client.py.OpenRouterClient — eigene RoleConfig „kontoauszug_klassifikation"? (3) Welche
Steuer-Kategorien im MVP (§35a/§10b/§10 zuerst)? (4) Recorded-Fixture-Strategie + 1 Live-Cap-Test OK?
(5) IBAN-Maskierung + Buchungstext-only ans LLM OK? → dann Bau (Live-LLM-Anbindung = Julius-Cap).
