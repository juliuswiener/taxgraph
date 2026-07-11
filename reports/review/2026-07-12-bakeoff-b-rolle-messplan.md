# B-Bake-off — Vorregistrierter Messplan (zur Freigabe VOR dem Lauf)

Julius-Go (Besetzungs-Re-Eval, msg 1230). Ziel: bessere Besetzung der Formalisierer-
B-Rolle (aktuell z-ai/glm-5.2 — lieferte bei nr6_7 kein Catala, kappte nr5a unbedingt).
A (anthropic/claude-sonnet-4.6) und Judge (deepseek-v4-pro @ deepinfra) unveraendert.
Prompts UNVERAENDERT. models.yaml-Aenderung NUR B-Rolle, Hash-Stempel wie gehabt.
**Dieser Plan ist vor dem Lauf fixiert; nachtraegliche Kriterien-Aenderung = ungueltig.**

## Kandidaten für B (Dekorrelation: andere Familie als A=Anthropic UND Judge=DeepSeek)

| # | Modell | Familie | Provider-Pin (Vorschlag) | Begründung |
|---|---|---|---|---|
| B1 | google/gemini-2.5-pro | Google | google-vertex (westlich) | starker Code/Reasoning, andere Familie |
| B2 | mistralai/mistral-large-2512 | Mistral (EU) | mistral (EU-Host) | EU-Hosting = Data-Sovereignty-Plus fuer Legal |
| B3 | meta-llama/llama-4-maverick | Meta (US) | (bei Run-Setup pinnen) | dritter Datenpunkt, US-westlich |

Provider-Status wird bei Run-Setup verifiziert (muss status=0, westlich/EU, unquantisiert —
KEINE fp4/fp8, keine chinesischen Endpoints). Kandidaten mit degradiertem/quantisiertem
Pin fallen raus. Qwen/GLM ausgeschlossen (Alibaba/Z.AI-Data-Sovereignty bzw. aktueller
Schwachpunkt). Instructor kann auf 2 Kandidaten kuerzen.

## Testset (3 Regeln)

- **nr6_7** (defekt, Waechter-Seed j3 = Letztjahr-AfA-Rest 200,00): harter Fall,
  mehrjaehrige AfA-Verteilung.
- **nr5a** (defekt, Waechter-Seed {47} ungekappt 16.800 / {48} gekappt 12.000):
  48-Monats-Gate, das A UND B bisher verfehlten.
- **p24b** (Kontrolle, bekannt verified_bedingt): darf NICHT regredieren — misst, ob
  ein Kandidat gute Faelle kaputtmacht.

## Messung je Kandidat (B-spezifisch, da Standard-clerk catala_a misst)

Pro Lauf wird catala_B separat ausgewertet:
- **syntax_b / typecheck_b** (aus report.json): kompiliert B ueberhaupt?
- **equivalence** (A==B auf dem Raster): stimmt B mit A ueberein?
- **B-clerk-Seed-Quote**: die Seeds werden gegen catala_B gefahren (per-Seed clerk_gate,
  $0 lokal) — insb. die WAECHTER-Seeds (nr6_7 j3, nr5a 47/48).
- **Kosten/Lauf** (usage-Delta).

Stabilitaet: **2 Laeufe je (Kandidat x Regel)** (Run-Varianz ist real — A regressierte
in einem redo_a). Ein Kandidat, der nur in 1 von 2 Laeufen den Waechter kriegt, ist instabil.

## VORREGISTRIERTE Kriterien (gewichtet, fix)

| Kriterium | Gewicht | Messung |
|---|---|---|
| Waechter-Seeds bestanden (B-clerk j3 + 47/48) | **0,40** | Anteil bestandener Waechter ueber 2 Laeufe (der eigentliche Zweck) |
| liefert-Catala + syntax_b/typecheck_b first-pass | 0,25 | kompiliert ohne Repair, in beiden Laeufen |
| equivalence (A==B) auf dem Raster | 0,15 | A==B je Lauf |
| Kontrolle p24b nicht regrediert | 0,15 | p24b bleibt verified_bedingt-faehig (Gates gruen) |
| Kosten/Lauf | 0,05 | invers, Tiebreaker |

## Entscheidungsregel (fix)

- **Eligibilitaet**: ein Kandidat, der die Waechter-Seeds in KEINEM der 2 Laeufe je Regel
  bekommt, ist fuer diese Regel disqualifiziert (loest das Problem nicht).
- **Gewinner** = hoechster gewichteter Gesamtscore ueber die 3 Regeln, UNTER den fuer
  nr6_7 UND nr5a eligiblen Kandidaten.
- **Kein Kandidat eligibel fuer beide defekt-Regeln** → Eskalation an Instructor
  (dann evtl. auch A-Rolle oder Zuschnitts-Feedback-Schleife erwaegen).
- **Gleichstand** (Score-Differenz < 0,05) → Eskalation an Instructor, kein
  dev-Alleingang.

## Budget

3 Kandidaten x 3 Regeln x 2 Laeufe = 18 Laeufe x ~0,08 USD ≈ **~1,5 USD** (bei 2
Kandidaten ~1,0). Rahmen bis 4 USD (Puffer fuer Repair-Runden). Nacht-Rest ~9,5 USD.
Buchfuehrung: usage-Delta je Batch.

## Ablauf nach Freigabe

1. models.yaml B-Rolle je Kandidat pinnen (Hash-Stempel), Lauf, report archivieren
   (--force-Archival greift jetzt), catala_B auswerten. 2. Rohdaten-Tabelle + Score an
   Instructor → Besetzungsentscheid (Julius-Widerrufsvorbehalt). 3. Danach: nr6_7 +
   nr5a Neulauf mit Gewinner-B; defekt-Items erloeschen NUR bei gruenen Waechter-Seeds.
   4. Stufe B Teil 2 in 2er-Batches.

## Frage an dich

Freigabe so? Besonders: (a) 3 oder 2 Kandidaten (B3 Llama streichen)? (b) 2 Laeufe je
Zelle ok, oder reicht 1 + 2. Lauf nur beim Fuehrenden (billiger, aber schwaecheres
Stabilitaetssignal)? (c) Gewichte ok (Waechter 0,40 dominant)?
