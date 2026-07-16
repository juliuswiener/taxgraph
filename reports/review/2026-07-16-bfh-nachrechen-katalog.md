# BFH-Nachrechen-Katalog (Paket 9 Runde 2, dev-2)

taxgraph-dev-2, 2026-07-16. Zwei gefreezte BFH-Urteile auf nachrechenbare Zahlenketten
(rule-covered) + qualitative Rechtssatz-Anker geprüft. Gleiche Methodik wie der amtliche-
Beispiele-Katalog. LLM-frei, $0, read-only. Anker voll-Länge _normalize-verifiziert.

## A. Nachrechenbare Zahlenketten

| # | Urteil | Zitatanker (voll-Länge, _normalize-OK) | Kette | Regel | Status |
|---|---|---|---|---|---|
| A1 | VI R 75/14 (19.01.2017) | „bis 15.340 € 2 % 306,80 € bis 51.130 € 3 % 1.073,70 € bis 51.835 € 4 % 28,20 € zumutbare Belastung 1.408,70 €" | GdE 51.835 → stufenweise 2/3/4 % → 1.408,70 € | p33_3_zumutbare_belastung | **nachrechenbar HIT, aber BEREITS test_seed** |
| A2 | VI R 52/20 (01.08.2024) | „steuerpflichtigen Arbeitslohn in Höhe von 94.775,51 € (= 129/245 von 180.000 €)" + „Steuerfrei sind 116/245 von 180.000 € = 85.224,49 €" | Abfindung 180.000 × 129/245 = 94.775,51 (DE-steuerpflichtig) / × 116/245 = 85.224,49 (DBA-frei) | **none** | nachrechenbar (Quote), aber KEINE DBA-Abfindungs-Aufteilungs-Regel |

**A1-Nachrechnung (bestätigt):** 15.340×2 % = 306,80; (51.130−15.340)=35.790×3 % = 1.073,70;
(51.835−51.130)=705×4 % = 28,20; Summe = **1.408,70 €**. Deckt sich mit p33_3 (stufenweise,
BFH-Leitsatz). Ist bereits als amtlicher test_seed in der Regel verankert → **kein neuer
Golden-Kandidat**, sondern der bekannte Exemplar.

**A2-Nachrechnung (bestätigt):** 180.000 × 129/245 = 94.775,51 €; × 116/245 = 85.224,49 €
(Summe 180.000,00, Rundungs-komplementär). Reine Quoten-Aufteilung nach inländischen/DBA-
Arbeitstagen. **abgedeckt_von_regel = none** — es existiert keine produktive Regel für die
zeitanteilige DBA-Abfindungs-Zuordnung (Kausalitätsprinzip). Die im Urteil genannten
Lohnsteuer/SolZ-Beträge (31.131 € / 1.712,20 €) stammen aus der Lohnsteuer-Bescheinigung
(Steuerklasse-I-Tabelle) — ebenfalls nicht rule-covered. → Coverage-Hinweis, kein Kandidat.

## B. Rechtssatz-Anker (qualitativ)

| # | Urteil | Rechtssatz-Anker (voll-Länge, _normalize-OK) | Stützt |
|---|---|---|---|
| B1 | VI R 52/20 | „die sogenannte Grenzgängerregelung des Art. 13 Abs. 5 DBA-Frankreich 1959/2001 nicht entgegen" (Leitsatz: DE-Besteuerungsrecht für Abfindung nach Art. 13 Abs. 1, soweit auf Inlands-Wohn-/Arbeitszeit entfallend) | **FR-DBA-Katalog**: Abfindungs-Zuordnung Grenzgänger (Art. 13 Abs. 1 vs. Abs. 5 DBA-FR); Abfindung fällt NICHT unter die Grenzgängerregelung, soweit inländisch veranlasst |
| B2 | VI R 75/14 | „bis 15.340 € 2 % 306,80 € … zumutbare Belastung 1.408,70 €" (Leitsatz: stufenweise Ermittlung der zumutbaren Belastung, nicht Gesamtquote) | **p33_3_zumutbare_belastung**: die stufenweise (nicht einheitliche) Prozentanwendung je GdE-Stufe |

B1 ist der zentrale Rechtssatz-Anker für dev-1s FR-DBA-Katalog: er ordnet die Abfindung
eines (ehemaligen) Grenzgängers dem deutschen Besteuerungsrecht zu (Art. 13 Abs. 1), soweit
sie auf die inländische Tätigkeitszeit entfällt — die Grenzgängerregelung (Abs. 5) sperrt
das nicht. Normen laut Urteilskopf: EStG § 19, § 24 Nr. 1, § 49 Abs. 1 Nr. 4 Buchst. d;
DBA-FRA Art. 13 Abs. 1 + Abs. 5; OECD-MA Art. 15 Abs. 1. VZ 2015.

## Fazit
- **0 NEUE rule-covered nachrechenbare Golden-Kandidaten.** Das ist ein VALIDES Ergebnis
  (BFH-Urteile tragen selten vollständige, von unseren Regeln abgedeckte Input→Output-Ketten):
  A1 ist nachrechenbar+rule-covered, aber bereits geerntet; A2 ist nachrechenbar, aber nicht
  rule-covered. Nichts konstruiert.
- **Wert dieser Runde = 2 Rechtssatz-Anker** (B1 FR-DBA-Abfindung, B2 p33_3-Stufenweise) +
  1 Coverage-Hinweis (DBA-Abfindungs-Aufteilung = none).

## Repro
```
cd taxgraph-multivz
# Euro-Kontexte je BFH-Urteil scannen + Anker/Arithmetik prüfen:
python3 - <<'EOF'
import sys; sys.path.insert(0,"pipeline"); from gates import _normalize
from decimal import Decimal as D, ROUND_HALF_UP
q=lambda x:x.quantize(D("0.01"),rounding=ROUND_HALF_UP)
print(q(D(15340)*D("0.02"))+q(D(35790)*D("0.03"))+q(D(705)*D("0.04")))  # 1408.70
print(q(D(180000)*129/245), q(D(180000)*116/245))                        # 94775.51 85224.49
EOF
```
Kein Code/Registry-Touch. Reiner Recherche-Katalog.

## Rückfrage an Instructor
1. Soll die DBA-Abfindungs-Aufteilung (zeitanteilig, Kausalitätsprinzip, A2) als Regel-
   Kandidat in den Backlog (analog GewSt/§8b-Ketten) — oder bleibt sie als qualitative
   FR-Katalog-Bedingung (B1) ohne eigene Rechen-Regel?
