# E-Bilanz Paket 2 — Muss-Feld-Katalog + Muss∩W2-Re-Verifikation (taxgraph-dev-2, 2026-07-15)

Fortsetzung zu `2026-07-15-ebilanz-taxonomie-versionen-wj.md` (e63f1b1). Instructor hat 6.7+6.8
beschafft + committet (04650f8, `sources/ebilanz/6.7|6.8/`); Autorisierung Julius direkt im Chat
(„alles nacheinander", Entscheidungsliste mit E-Bilanz/XBRL — in beiden meta.yaml dokumentiert).
Auftrag: (1) voller Muss-Feld-Katalog je Version, (2) 6 W2-Andock-Cluster auf 6.7/6.8-Pfade
re-verifizieren (bisher nur 6.9-Beleg), (3) Lücken-Liste, Auflage Kontennachweis-Fundstelle.
Rein lokal, read-only, $0. Alle Zahlen unabhängig aus `reference-fiscal.xml` re-abgeleitet
(nicht der meta-Notiz vertraut — falsches-grün). Skripte: `scratchpad/ebilanz_{inspect,katalog,find2,verify}.py`.

## 0. Unabhängige Freeze-Prüfung
`hgbref:fiscalRequirement` in `de-gaap-ci-<datum>-reference-fiscal.xml` (Rolle
`mandatoryDisclosureRef`) trägt die Muss-Klassifikation; `hgbref:legalFormEU/PG/KSt` die
Rechtsform-Geltung. Selbst geparst, nicht aus Excel-Vis (die ist unverbindlich). Deckt sich
mit den committeten sha256.

## 1. Muss-Feld-Katalog (de-gaap-ci Kerntaxonomie)

**fiscalRequirement-Kategorien je Version** (concepts mit gesetztem fiscalRequirement):

| Kategorie | 6.7 (WJ 2024) | 6.8 (WJ 2025) | Muss-Charakter |
|---|---|---|---|
| `Mussfeld` | 461 | 472 | Pflichtfeld |
| `Summenmussfeld` | 183 | 183 | Summen-Pflichtfeld (rechnerische Oberpositionen) |
| `Mussfeld, Kontennachweis erwünscht` | 52 | 52 | Pflichtfeld + Konten-Detail erwünscht |
| **MUSS gesamt (weit)** | **696** | **707** | Mussfeld+Summenmussfeld+Kontennachweis |
| — davon eng (ohne Summenmussfeld) | 513 | 524 | = Instructor „514/525" ±1 (Tokenisierung) |
| `Rechnerisch notwendig, soweit vorhanden` | 455 | 455 | KEIN Muss (bedingt) |
| (kein fiscalRequirement) | 1301 | 1342 | Kann/Struktur |

**Zähl-Abgleich Instructor:** die meta-Notiz „514×/525× Mussfeld" ist die ENGE Definition
(`Mussfeld` + `Mussfeld, Kontennachweis` = 513/524, ±1 String-Tokenisierung). Meine WEITE
Definition zählt `Summenmussfeld` (+183) mit = 696/707. **Für den Katalog gilt die weite
Definition** (Summenmussfeld ist rechnerisch pflichtig), die enge als Teilmenge dokumentiert.

**Rechtsform-Filter (W2-Nenner = Einzelunternehmer/PersG):** von den MUSS-Feldern gelten
**671/696 (6.7)** bzw. **682/707 (6.8)** für `legalFormEU=true` ODER `legalFormPG=true`;
nur **25** sind KSt-only (Kapitalgesellschafts-Positionen) = außerhalb unseres Nenners.

**GCD-Stammdatenmodul (de-gcd):** **58 (6.7) / 60 (6.8)** Muss-Felder, alle `Mussfeld`
(Dokumentkopf: Bilanzart, WJ, Steuernummer, Rechtsform …). **W2-fremd** — kommt aus Fall-
Stammdaten, nicht aus Bewertungsregeln; für gültige §5b-Übermittlung dennoch Pflicht.

## 2. Element-Wanderung 6.7 → 6.8 (Gültigkeits-Direktive auf Element-Ebene)

MUSS-Set-Diff (committete reference-fiscal, kein PDF nötig):
- **+11 neue MUSS in 6.8, 0 entfallen.** Alle 11 betreffen **Personengesellschafts-/Mit-
  unternehmer-Positionen** (relevant für W2-Nenner PersG!): `equity.subscribed.{un,}limitedLiablePartners`,
  `deficitNotCoveredByCapital.{loss,withdrawal…}LiablePartner`, `is…otherCost.{liability,other}RemunerationPartners`,
  `fpl.additions.minst`.
- **9 fiscalRequirement-Höherstufungen** `Rechnerisch notwendig → Mussfeld` (dieselben PersG-
  Positionen) — d. h. WJ 2025 verlangt diese Mitunternehmer-Angaben nun zwingend.
- Kern-Bilanz/GuV-Struktur sonst **stabil** 6.7↔6.8. `Änderungsnachweis_Tax_6.{7,8}.pdf` liegt
  lokal für tiefere Label-/Definition-Wanderungen (nicht Muss-relevant → hier nicht ausgewertet).

**Konsequenz:** ein WJ-2024-Fall (6.7) und WJ-2025-Fall (6.8) unterscheiden sich im MUSS-Set nur
durch diese 11 PersG-Felder — für Einzelunternehmer praktisch identisch, für PersG-Fälle beachten.

## 3. Muss ∩ W2 — 6 Andock-Cluster, auf 6.7/6.8 RE-VERIFIZIERT

⚠ **3 Pfade aus dem Scoping (16f0aa6) waren aus einem 6.9-ERiC-Beispiel abgeleitet und lösen in
6.7/6.8 NICHT auf** — genau der Grund der Re-Verifikation. Korrigiert:

| Scoping-Pfad (6.9-abgeleitet) | Status | Korrekter 6.7/6.8-Pfad |
|---|---|---|
| `…netIncome.EStGs` | **existiert nicht** | entfällt — `netIncome` hat nur EIN Kind `taxBalanceGenerally` |
| `…netIncome.transferDiffTaxAccounts` | Pfad falsch | `…netIncome.taxBalanceGenerally.transferDiffTaxAccounts` (1 Ebene tiefer) |
| `bs.ass.prepaidExpense` | Name falsch | `bs.ass.prepaidExp` (+ Kinder `.loadRedempt/.other/…`) |

**Re-verifizierte Cluster-Tabelle (✓ = Element vorhanden; [Kategorie] = fiscalRequirement):**

| W2-Regel | de-gaap-ci-Pfad (6.7/6.8) | 6.7 | 6.8 | W2 füllt |
|---|---|---|---|---|
| p4_1_bv_vergleich §4 I | `bs.eqLiab.equity.netIncome.taxBalanceGenerally` | ✓ Summenmussfeld | ✓ Summenmussfeld | steuerbil. Gewinn |
| p4_1 Überleitung (B2) | `…taxBalanceGenerally.transferDiffTaxAccounts` | ✓ Mussfeld | ✓ Mussfeld | Überleitungswert |
| **B2 Maßgeblichkeit** | `hbst.transfer.*` (19 Elem., HandelsBil→SteuerBil) | ✓ | ✓ | **Überleitungsrechnung** |
| p6_1_1 AV-Bewertung §6 I Nr.1 | `bs.ass.fixAss` (+ Unterbaum) | ✓ Summenmussfeld | ✓ Summenmussfeld | AV-Wert |
| p6_1_3a Abzinsung §6 I Nr.3a | `bs.eqLiab.accruals` / `.other` | ✓ Summenmussfeld / ✓ Muss+KN | ✓ / ✓ | Rückstellungswert |
| p6a Pension §6a | `bs.eqLiab.accruals.pensions{.direct/.externalFunds/.shareholder}` | ✓ Summenmuss / ✓ Muss | ✓ / ✓ | Pensionsrückst. |
| p5_5 aktiver RAP §5 V | `bs.ass.prepaidExp` | ✓ Mussfeld | ✓ Mussfeld | aktiver RAP |

**Alle 6 Cluster in 6.7 UND 6.8 identisch belegt** (Kategorie stabil über beide Versionen) →
kein VZ-/WJ-Drift in den W2-Andockpunkten. **Neuer Fund B2:** das **`hbst.transfer.*`-Modul**
(HandelsBilanz-STeuerbilanz-Überleitung: `bsAss`/`bsEqLiab`/`isChangeNetIncome` × `changeValue`/
`reclassification`) ist die eigentliche Maßgeblichkeits-/Überleitungs-Mechanik (§5 I / W2-B2) —
reicher als das Einzelfeld `transferDiffTaxAccounts` und der präzisere Andock für die
Überleitungsseite unserer W2-Regeln. Selbst Tupel-Tabelle (kein Einzel-Muss), aber Pflicht-
Struktur sobald Handelsbilanz+Überleitung statt direkter Steuerbilanz übermittelt wird.

## 4. Kontennachweis-Pflicht (JStG 2024) — dokumentierte Lücke (Auflage)

**Präzise Fundstelle:**
- **Norm:** § 5b Abs. 1 Satz 1 EStG i. d. F. JStG 2024 — eingefügt: „… *einschließlich der
  unverdichteten Kontennachweise mit Kontensalden sowie der Anlagenspiegel und das diesem
  zugrundeliegende Anlagenverzeichnis* …" (Quelle: gesetze-im-internet.de/estg/__5b.html).
- **Gesetz:** Jahressteuergesetz 2024 v. 02.12.2024, **BGBl. 2024 I Nr. 387** (ausg. 05.12.2024),
  Art. 1 (Änderung EStG).
- **Inkrafttreten:** Kontennachweise (+ Anlagenverzeichnis) für **WJ, die nach dem 31.12.2024
  beginnen (= WJ 2025)**; Anlagenspiegel/Anhang/Lagebericht/Prüfungsbericht/Verzeichnisse §5 I S.2,
  §5a IV erst WJ nach 31.12.2027 (§ 52-Anwendungsstaffelung).
- **Taxonomische Umsetzung:** BMF-Schreiben v. 10.06.2025 (Tax. 6.9), IV C 6-S 2133-b/00064/002/006,
  **BStBl 2025 I S. 1450** — Nichtbeanstandungsregelung zur Übermittlung der Kontennachweise.

**Warum Lücke, kein Feld:** die 52 Taxonomie-Felder `Mussfeld, Kontennachweis erwünscht` (in 6.7
UND 6.8) sind das positions-Flag „Konten-Detail erwünscht" — **nicht** die JStG-Übermittlungs-
pflicht. Die Pflicht betrifft die **unverdichteten Kontennachweise mit Kontensalden** = ein
zusätzlicher Dokument-Bestandteil auf **Konten-Ebene**, nicht als einzelnes de-gaap-ci-Muss-Element
abbildbar. **Timing-Mismatch:** rechtlich Pflicht ab WJ 2025 (= Taxonomie 6.8), taxonomischer
Übermittlungsweg aber erst mit 6.9 (WJ-2026-Version) geregelt → BMF-Nichtbeanstandung überbrückt
6.8-Fälle. **W2-Bezug: Nicht-Gegenstand** — unsere Bilanzregeln arbeiten positions-, nicht
kontenweise; Kontennachweis ist ein Submission-Bestandteil, den dev-1 beim Extraktor als
Pflicht-Komponente (nicht als Muss-Feld) vermerken muss.

## 5. Lücken-Liste
- **Voll-Labels fehlen im Freeze:** committet ist nur `label-fiscal-de.xml` (Fiskal-Subset,
  ~108 dt. Labels für 2505 concepts). Die allgemeine `label-de`-Linkbase ist NICHT im Paket →
  für einen lesbaren Muss-Feld-Katalog mit deutschen Bezeichnern muss dev-1 die allgemeine
  Label-Linkbase nachbeschaffen (aus dem lokalen XBRL-Gesamt-ZIP entpackbar, sha256 in meta).
  **Beschaffungs-Nachtrag, kein neuer Download.**
- **GuV-Population `is.*`, Kontennachweis-Konten-Ebene, GCD-Stammdaten** = Nicht-Gegenstand /
  Fall-Input (W2 berührt sie nicht), wie im W2-Nenner vermerkt.
- **Anlagenspiegel/Anlagenverzeichnis (§5b JStG2024):** ab WJ 2028 Pflicht — außerhalb WJ24/25-Scope,
  nur als Vorwarnung notiert.
- **25 KSt-only Muss-Felder** außerhalb W2-Nenner (Kapitalgesellschaft) — Nicht-Gegenstand.

## 6. Status + Übergabe an dev-1 (Bau, Paket 2)
- **(1) Muss-Katalog:** ✅ 696/707 (weit) bzw. 513/524 (eng) + 58/60 GCD; Rechtsform-gefiltert
  671/682 im W2-Nenner. Kategorien + Zählung reproduzierbar (Skripte).
- **(2) Muss∩W2:** ✅ 6 Cluster auf 6.7/6.8 re-verifiziert; **3 Scoping-Pfade korrigiert**;
  B2-Andock auf `hbst.transfer.*` präzisiert. Kein WJ-Drift in den Andockpunkten.
- **(3) Lücken:** ✅ oben; Haupt-Handlungspunkt = allgemeine Label-Linkbase nachbeschaffen.
- **Kein Extraktor-Code** (Auflage) — Bau = dev-1: XBRL-Linkbase-Parser auf die committeten
  reference-/presentation-Dateien, Scope = Muss ∩ 6 W2-Cluster (kuratiert, nicht ganze Taxonomie).
  **LLM-frei, $0, keine Cascade/Formalisierer** (Deklarations-Zuordnung, kein Regel-Bau).
- Report → Instructor-Sammelmeldung → Instructor-Nachlauf → Bau-Order dev-1.
