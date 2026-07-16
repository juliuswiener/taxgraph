# DEV-2 Paket 8 — E-Bilanz-Feldmapping (msg 2129)

taxgraph-dev-2, 2026-07-16. Mapping-Schicht produktive Regel-Outputs → de-gaap-ci-
Taxonomie-Positionen. Deterministisch, LLM-frei, additiv (ebilanz/ + tests/), Registry/
rules.yaml/pipeline nur READ. HARTE REGEL befolgt: keine geratenen Zuordnungen —
unsichere explizit `status: unklar` mit Begründung.

## P8-1 — ebilanz/feldmapping.yaml

8 Einträge, alle taxonomie_konzept + kategorie gegen `katalog_6.7.json` UND
`katalog_6.8.json` verifiziert (beide WJ kategorie-gleich).

### SICHER (5) — W2-Bilanzpositionen, Regel-Output speist die Position
| rule_id | output | taxonomie_konzept | kategorie |
|---|---|---|---|
| p4_1_bv_vergleich | gewinn | bs.eqLiab.equity.netIncome.taxBalanceGenerally | Summenmussfeld |
| p6_1_1_bewertung_av | ansatz | bs.ass.fixAss | Summenmussfeld |
| p6_1_3a_abzinsung | abgezinster_wert | bs.eqLiab.accruals.other | Mussfeld, KN erwünscht |
| p6a_pension_hoechstbetrag | pensionsrueckstellung | bs.eqLiab.accruals.pensions | Summenmussfeld |
| p5_5_aktiver_rap | aktiver_rap | bs.ass.prepaidExp | Mussfeld |

fixAss/pensions sind Aggregat-Summenfelder → Output ist verifizierte **Komponente**
(hinweis je Eintrag), nicht die Gesamtsumme. rule_id+output je gegen `signature.output`
der Registry geprüft.

### UNKLAR (3) — Konzept existiert + kategorie stimmt, aber Wert-Bezug NICHT 1:1
| konzept | (Kandidat-)rule | Grund status:unklar |
|---|---|---|
| is.netIncome.tax.gewst | p11_steuermessbetrag.steuermessbetrag | Output = **Messbetrag** (Bemessungsgröße), Position braucht GewSt-**Betrag** = Messbetrag×Hebesatz (kein Regel-Output, Runner-Ebene); Buch-Steueraufwand ≠ festgesetzte Steuer |
| is.netIncome.tax | p23_koerperschaftsteuer_satz.koerperschaftsteuer | Kein KSt-spezifisches GuV-Konzept — nur Aggregat (KSt+GewSt+SolZ); festgesetzte KSt ≠ bilanzieller Steueraufwand (Rückstellung/latente Steuer) |
| …taxBalanceGenerally.transferDiffTaxAccounts | — (kein rule_id) | Steuerlicher Ausgleichsposten aus Maßgeblichkeits-/Überleitungsrechnung (hbst.transfer-Tupel); NICHT als produktive Regel formalisiert |

Bewusst NICHT gemappt (kein Rate-Zwang): GewSt-Ketten-Zwischenwerte (p7 gewerbeertrag,
p8_1 hinzurechnungsbetrag, p9 kuerzung_gesamt) = Steuerberechnungs-Intermediäre, keine
Bilanz-/GuV-Positionen. hbst.transfer.* Tupeltabelle = kein Einzelkonzept-Muss.

## P8-2 — Coverage-Report Muss-Felder

| | 6.7 (WJ 2024) | 6.8 (WJ 2025) |
|---|---|---|
| muss_weit (Nenner) | 697 | 708 |
| davon Mussfeld / Summenmuss / KN-erwünscht | 462 / 183 / 52 | 473 / 183 / 52 |
| EU/PG-relevant | 672 | 683 |
| KSt-only | 25 | 25 |
| de-gcd-Stammdaten-Muss (separat) | 58 | 60 |

**(a) aus Regel-Outputs befüllbar:** **5 sicher** (obige Tabelle) + **3 unklar** (Wert-
Bezug offen) = 8 Positionen mit Regel-Bezug. Solide befüllbar heute: **5**.

**(b) reine Sachverhalts-Felder:** die **überwiegende Mehrheit** (~689 von 697/708).
E-Bilanz ist handels-/steuerbilanzielle Rechnungslegung — die meisten Muss-Positionen
werden aus der Buchführung des Steuerpflichtigen befüllt, NICHT aus unseren EStG/GewSt/
KSt-Berechnungsregeln abgeleitet. Unsere Regeln berechnen Steuer(bemessungs)größen, keine
Bilanzposten.

**(c) n.a. für unseren Scope:** **KSt-only = 25** (nur bei KapGes-Veranlagung Muss, außerhalb
EStG-Einzel/PersG-Scope) als konkrete Untergrenze; zusätzlich branchen-/spezialpositionen
(Kreditinstitute/Versicherungen etc.) innerhalb der (b)-Menge.

⚠ **Ehrliche Grenze (melde statt improvisiere):** eine **vollständige per-Feld-Partition
(b) vs (c)** über alle ~689 Rest-Positionen ist NICHT maschinell aus den Regel-Outputs
ableitbar — sie bräuchte eine per-Position-Klassifikation (Sachverhalt vs. Scope-fremd),
die ohne Rate-Zwang nicht seriös aus dem vorhandenen Material zu ziehen ist. Geliefert
sind die **belegbaren** Zahlen (mapped=8, kst_only=25, Gesamt 697/708); die feinere b/c-
Aufteilung ist eine eigene Klassifikations-Aufgabe (Kandidat für Folge-Paket), bewusst
nicht geraten.

**Kernbefund:** die Regel→Taxonomie-Kopplung ist naturgemäß **dünn** (5 solide von ~700) —
das ist kein Lückenbefund, sondern die Struktur: E-Bilanz-Muss = Rechnungslegungsdaten,
unsere Regeln = Steuerberechnung. Die 5 sicheren sind die echten Andockpunkte (steuerbil.
Gewinn, AV, Rückstellungen inkl. Pension, aktiver RAP).

## P8-3 — Gate tests/test_ebilanz_feldmapping.py

7 Tests, Catala-frei, in `make unit`:
- konzepte_existieren_und_kategorie_stimmt (jedes Konzept in jeder gelisteten WJ + kategorie);
- keine_konzept_duplikate; sicher_hat_registry_output (rule_id+output gegen `signature.output`);
- unklar_hat_begruendung; bestand_und_status; **2 Negativtests** (erfundenes Konzept / falsche
  kategorie → Verletzung).

**Gate-Beweis:** `make unit` 181 passed (174 + 7), exit 0. **Tamper (Report):** echtes Konzept
`bs.ass.prepaidExp` in feldmapping.yaml auf Platte → `bs.ass.ERFUNDEN_TAMPER` verbogen →
`test_konzepte_existieren_und_kategorie_stimmt` FAILED (»Konzept fehlt im Katalog 6.7«+6.8);
nach Restore grün.

## Repro (Symlink-Lehre: Repro-Schritt mitliefern)
```
cd taxgraph-multivz
python3 ebilanz/katalog.py                       # regeneriert katalog_6.7/6.8.json (falls nötig)
python3 -m pytest tests/test_ebilanz_feldmapping.py -q   # 7 passed
make unit                                        # 181 passed
```
Kein Catala/venv/Symlink nötig (reine stdlib + yamlstrict). Katalog-JSONs sind committet.

## Commits (feat/multivz)
- P8-1 ebilanz/feldmapping.yaml
- P8-3 tests/test_ebilanz_feldmapping.py
- P8-2 dieser Report

## Rückfragen an Instructor
1. Sollen die 3 `unklar`-Einträge (GewSt-Betrag/KSt-Aufwand/transferDiff) als **Folge-Paket**
   aufgelöst werden? Konkret bräuchte es: (i) eine GewSt-Betrags-Regel (Messbetrag×Hebesatz)
   als Registry-Output; (ii) eine Buch-Steueraufwand-/Steuerrückstellungs-Abgrenzung; (iii)
   die Maßgeblichkeits-/Überleitungsrechnung (b2) als produktive Regel.
2. Ist die feinere (b)/(c)-Partition (~689 Rest-Muss-Felder) als eigenes Klassifikations-Paket
   gewünscht, oder reicht die belegbare Untergrenze (kst_only=25) + Gesamtzahl?
