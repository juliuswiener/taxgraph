# Morgen-Paket — Julius-Entscheidungen (Nacht 2026-07-11/12)

Konsolidiert für den Morgen-Review. **Nacht-Summe ~$3,0 / 10 USD.** Alle Gate-Verdikte
frisch, doppelt gerechnet (dev + instructor Regate), kein Falschgrün. Version 2 (ersetzt
die 06:04-Fassung — deren zentrale Prämisse „drei Capability-Fehler" wurde über Nacht
FALSIFIZIERT, siehe Punkt 1).

---

## 0. Die zentrale Erkenntnis der Nacht: NULL Modell-Capability-Defizite

Der 06:04-Trigger „drei Formalisierer-Schwächen → Besetzung neu" war ein FEHLURTEIL.
Ein zweistufiger Bake-off ($1,7) + gezielte Diagnose zeigten: der Formalisierer-Prompt
sieht NUR den quellen-**auszug** (Geltungsbedingungen/Signatur-Kommentare erreichen ihn
NIE — verifiziert im Code). Die „Fehler" waren **KONTEXT-HUNGER**: enge auszüge, die den
Trigger einer Bedingung wegließen. Beispiel nr5a: der auszug erwähnte die 48-Monats-Schwelle
NIE → das Modell kappte korrekterweise immer. Klausel stand im frozen source, nur nicht im
auszug. Nach Weitung: A trifft 47→16.800 (ungekappt) sofort.

**Dreistufige Fehler-Taxonomie (alle drei empirisch belegt, in dieser Reihenfolge prüfen):**
1. **Kontext-Hunger** → auszug weiten (Trigger fehlt im Prompt). nr5a, solzg, p36, p10-Rest.
2. **Boundary-Kodierung** → Split in Teilregeln, Schwelle wird §2-Selektion (der Name
   trägt die Kodierung nicht, der Pin erreicht das Modell nicht). nr5a vor/nach_48.
3. **Arithmetische Folgerung ohne Wortlaut** → Integrations-Arithmetik, keine
   Formalisierungsregel (steht nicht im Gesetz). nr6_7 Überhangsjahr = AK − Σ(laufend).

Über die ganze Nacht KEIN einziges echtes Modell-Fähigkeitsdefizit — alles löste sich in
Kanal, Kodierung oder Arithmetik auf. (Instructor-Selbstkorrektur #3: Capability-Deutung
war voreilig. Selbstkorrektur #4: „nach_48 MIT Satz 4"-Vorgabe holte die Boundary zurück.)

---

## 1. Formalisierer-Besetzung — glm GEHALTEN (Widerruf), KEIN Wechsel nötig

Bake-off-Kette (Haupt: gemini-2.5-pro vs mistral-large-2512 vs llama-4-maverick, $1,0;
Mikro: gemini vs glm @16384 Token-Parität, $0,74):
- **KEIN Kandidat** löste die defekt-Regeln besser als glm. gemini == glm auf nr6_7 (exakt
  derselbe Einzelfehler), mistral regredierte sogar die Kontroll-Regel.
- Mikro-Bake-off falsifizierte die Trunkierungs-Hypothese: @16384 emittierten beide Catala,
  scheiterten am SELBEN Seed. gemini's einziger 8192-Solve war Run-Varianz (Glück).
- Nach den Splits ist glm auf allen gelösten Teilregeln A-gleich sauber → das vermeintliche
  glm-Capability-Delta (nr5a 47→12.400) war Schwellen-Kodierungs-Raten, nicht Fähigkeit.
**Entscheidung (dev+instructor, Widerruf bei dir): B = z-ai/glm-5.2 bestätigt, A = sonnet-4.6
bestätigt, kein Modellwechsel.** Reports: reports/review/2026-07-12-bakeoff-b-rolle-ergebnis.md
+ -mikro-bakeoff-nr6_7.md.

---

## 2. NEUER Entscheidungspunkt: roles.py-Erweiterung (Bedingungen → Formalisierer)

Der Code-Fakt (Bedingungen erreichen den Formalisierer nicht) ist eine strukturelle Grenze.
Der auszug-Weitungs-Ansatz umgeht sie, aber eine **roles.py-Erweiterung** (Geltungsbedingung-
beschreibungen ODER ein dediziertes Hinweis-Feld auch in den Formalisierer-Prompt) wäre der
robustere Fix. **Ändert faktisch den Prompt-Inhalt global → braucht dich + einen
vorregistrierten Messplan** (Prompt-Change-Dekret). Zwei Varianten zu vergleichen:
(a) Bedingungs-beschreibungen durchreichen, (b) dediziertes `hinweis`-Feld. Nicht heute Nacht.

---

## 3. NEUE 4. Taxonomie-Klasse: Rundungs-RICHTUNG (solzg-Residual)

solzg ist ehrlich 4/5 (Milderungszone via Weitung gefixt, war 0/0/511 kaputt). Residual:
seed 20.351 → 0,12 statt 0,11. „Bruchteile eines Cents bleiben außer Ansatz" = Trunkierung,
das Modell rundet kaufmännisch. Klausel IST im auszug — Rundungs-RICHTUNGS-Fehler.
**Backlog-Empfehlung: rundungs_lint um Richtungsprüfung erweitern** (deklarierte Rundung
trägt künftig floor/ceil/kaufmännisch; truncate vs round ist im Catala-Code syntaktisch
sichtbar) → deterministisches Repair-Signal statt Interpretationsglück. Vierte Klasse der
Taxonomie neben Kontext-Hunger / Boundary / Integrations-Arithmetik.

---

## 4. nr6_7 Überhangsjahr = §2-Integrations-Arithmetik (Task, Integrations-Phase)

Der Terminal-AfA-Rest bei unterjährigem Beginn (Jahr N = AK − Σ Vorjahre) steht NICHT im
Gesetzeswortlaut → keine Formalisierungsregel (Klasse 3). Lebt in der handgeschriebenen
§2-Integrationsschicht (rules/estg/arbeitnehmerfall) mit eigenem clerk-Test (seed 1200/ND3/
M7 → Jahr3 = 200). Task notiert, eigenes Subsystem, Integrations-Phase. Backlog-Alt: EStR
R 7.4 als typ:verwaltung-Quelle für einen zitierbaren Text-Anker (Phase 5).

---

## 5. Netto/Brutto-GWG-Grenzfall (§ 6 Abs. 2 → § 9b Abs. 1) — Recherche offen

nr6_7_laufend: `anschaffungskosten` = maßgebliche AK i.S.d. § 6 Abs. 2 S. 1 (Verweis § 9b
Abs. 1: Vorsteuer nur raus, soweit bei USt abziehbar — beim AN i.d.R. NICHT → Brutto-AK).
Als Bedingung `anschaffungskosten_sind_massgebliche_ak` deklariert (aus Monolith migriert).
Materiell für die 800/801-Grenze (Brutto vs Netto verschiebt sie). Braucht BMF/LStR-Bestätigung.

---

## 6. NEU: ELSTER-Zugang aktiv — Phase 4 startklar (5-Minuten-Julius-Download)

Entwicklerzugang da (.env.elster, gitignored via **/.env* — verifiziert). ERiC/checkESt-
Download braucht Portal-Login → **das machst DU (5 Min Browser-Login + Download, ablegen unter
elster/)**, weil ich keine Passwörter in Login-Felder eingebe (harte Boundary) und die Deny-Rule
mein Lesen der .env eh blockt. SOBALD die Dateien lokal liegen, läuft mein Smoke-Test-Skelett
automatisch (ERiC-Version, checkESt-Trivialaufruf, Offline-Verdikt fürs CI-Gate) — $0, rein lokal.
**Phase-4-Schnitt-Vorschlag** (deine Wahl): (i) Feldmapping ESt1A ↔ Signatur-Outputs, (ii)
checkESt als CI-Gate, (iii) Versand. Empfehlung: erst (ii) checkESt-CI-Gate (validiert die
formalisierten Werte gegen ELSTERs eigene Prüfung, ohne Versand-Risiko).

---

## 7. Widerrufsvorbehalt-Bestätigungen (Nacht-Delegation → dein OK)

**12 Regeln verified_bedingt** (doppelt gerechnet dev+instructor, pytest 98/98):
- 7× Charge 1: p10_1_7, p9_6, p9_1_3_nr5, p33 (Schritt 2), p24b, p9_4a, p35a.
- **p36_2_anrechnung** (Batch 1, nach auszug-Weitung + _lit-Fix): alle 4 Seeds korrekt.
- **3 Teilregeln** (Split-Erfolge, A+B grün, Registry vollständig): p9_1_3_nr5a_uebernachtung
  _vor_48 (4 Bed.), _nach_48 (5 Bed.), p9_1_3_nr6_7_afa_laufend (3 Bed.).
Weiter: **solzg 4/5** (Punkt 3), **nr5a/nr6_7-Monolithen → zuschnitt_ersetzt** (Historie
bleibt, aus Läufen raus, run.py SKIP_STATUS erweitert).
Neue Konventionen genutzt: input_nur_etikettiertes, ganzzahl_monate, keine_zusaetzliche_rundung.
Bugfix committet + Regressionstest: gates._lit negatives Money (-$3.000,00 statt $-3.000,00) —
betraf JEDE Erstattungs-/negativ-Output-Regel.

---

## 8. Teil-2-Rest-Batches ABGESCHLOSSEN — alle verified_bedingt (1 Residual)

Alle 8 Teil-2-Regeln adressiert (Sweep-Widenings, Kontext-Hunger via auszug-Weitung gelöst):
- **verified_bedingt**: p36, p33_1_2, p32_6, p31, p10_1_2, p10_1_4, **p10_1_3_3a**.
  nr5a_vor_48/_nach_48, nr6_7_afa_laufend (Splits). Triage komplett (Instructor-Klassen).
- **p10_1_3_3a-Basis-Durchbruch GELÖST**: A+B verfehlten den Durchbruch zunächst (basis 4.000
  > 2.800 → kappten 2.800), obwohl die Klausel im auszug war — weil sie in der langen Abs-4-
  Passage nach Zusammenveranlagten-Rauschen VERGRABEN war. Als eigener prominenter auszug-Block
  → sofort getroffen (4.000). **War doch Kontext-Hunger** (Prominenz-Verfeinerung der Leitlinie:
  Klausel muss nicht nur enthalten, sondern GEWICHTET/prominent sein). Der Judge-Abweichungs-
  Flag darauf war ein Falschpositiv (nicht_echt, per Wächter-Seed bewiesen).

**~17 Regeln verified_bedingt.** EIN einziges echtes Residual: **solzg-Rundungsrichtung**
(Punkt 3, deterministisch fixbar via rundungs_lint-Richtungsausbau). NULL Modell-Capability-
Deltas die ganze Nacht — Prominenz-Verfeinerung eliminierte auch das letzte vermeintliche
Sub-Mechanik-Delta.

## 8a. Auszug-Leitlinie (finalisiert, für Zuschnitts-Reviews)

Der auszug muss jede OUTPUT-variierende Klausel (1) ENTHALTEN und (2) PROMINENT zeigen — als
eigenen kohärenten Block, NICHT in Fremd-Sätzen/Sonderfall-Rauschen vergraben. Reine
Anwendbarkeits-Klauseln bleiben draußen (Geltungsbedingungs-Territorium). Bei Split-Teilregeln:
Selektions-Konditional AUSSCHLIESSEN (§2-Territorium), nur lokale Formel-Mechanik.

## Kosten-Schlusszeile
**Nacht gesamt ~$3,4 / 10 USD.** Bake-off-Kette ~$1,7 · Batch-1 + Splits + Neuläufe ~$0,9 ·
Rest-Batches (2-4) + Charge-2-Reste ~$0,8. Alles andere (Seeding, Freezes, Regates, Triagen,
Diagnosen, Tooling-Fix, Reports, Sweep, Morgenpaket) $0. Rahmen (10 USD) hält, Rest ~6,5 USD.

---
**Endstand von instructor doppelt unabhängig verifiziert** (frischer Sammel-Regate @b8109d4:
17 verified_bedingt, 0 Gate-Änderungen, pytest 98/98, Quellen-Gate 18/18 verbatim; solzg
flagged_for_review = einziger Rest-Rot, Rundungsrichtungs-Residual). Monolithe nr5a/nr6_7
sauber übersprungen (zuschnitt_ersetzt); Präzedenz-Store 3 gesperrte Fälle, keine ungesperrten
Doppel-Treffer. **bestaetigt_von: instructor, 2026-07-12 Nacht. Julius-Review = finale Instanz.**

Verifizierte 17: dHf, nr5a_vor_48, nr5a_nach_48, p9_4a, nr6_7_afa_laufend, p9_6, p10_1_7,
p24b, p33_3, p35a, p36, p33_1_2, p32_6, p31, p10_1_2, p10_1_3_3a, p10_1_4.
