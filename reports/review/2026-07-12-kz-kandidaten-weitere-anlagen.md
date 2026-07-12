# Kz-Kandidaten — weitere Anlagen 2025 (rollierendes Instructor-Review)

Gleiches Verfahren wie Anlage N (`2026-07-12-kz-kandidaten-anlage-n.md`): Primär = eingefrorener
amtlicher Vordruck (`sources/bfinv/<name>.txt`), Zweitbeleg = XSD E10-2025 (`elster/kz_extract.py`).
STARK = verbatim Konzept-Match beide Quellen. Identifier = XSD-E-Nr (A/B = Formularspalten);
Form-Kz nur Orientierung. Reihenfolge nach Regel-Dichte (Instructor).

---

## (1) Anlage N-Doppelte Haushaltsführung — p9_1_3_nr5

Quelle: `sources/bfinv/dhf_2025.txt`. Kern der Regel: Unterkunftskosten mit **1000-€/Monat-Cap**
(die Kappung RECHNET die Regel; deklariert wird der volle Aufwand) + Familienheimfahrten.

| Regel-Input | Vordruck-Zeile | XSD E-Nr | XSD-Sektion | wörtliches Label | Konfidenz |
|---|---|---|---|---|---|
| Unterkunftskosten (Miete etc.) — Cap-Basis | Zeile 23 (Kz 530) | E0207611 | Unterkunft | Aufwendungen (z. B. Miete einschließlich Stellplatz- / Garagenkosten, Nebenkosten) | **STARK** |
| Familienheimfahrten: einfache Entfernung km | Zeile 17 (Kz 514) | E0207116 | Woech_Heimf | einfache Entfernung in km (ohne Flugstrecken) | **STARK** |
| Familienheimfahrten: Anzahl | Zeile 17 (Kz 515) | E0207117 | Woech_Heimf | Anzahl der Familienheimfahrten | **STARK** |
| Familienheimfahrten: ÖPNV-Kosten | (Woech_Heimf) | E0207211 | Woech_Heimf | Kosten für öffentliche Verkehrsmittel (ohne Fähr- und Flugkosten) | **STARK** |
| Größe Zweitwohnung Ausland (m²) | Zeile 24 (Kz 531) | E0207702 | Unterkunft | Größe der Zweitwohnung des doppelten Haushalts im Ausland in m² | **STARK** |

Notiz: der **1000-€-Monats-Cap** (§ 9 Abs. 1 S. 3 Nr. 5 S. 4) ist kein Vordruck-Feld — er ist
Rechenlogik der Regel auf dem deklarierten Unterkunftsaufwand (E0207611). Deckt sich mit unserem
Zuschnitt (Cap wird berechnet, nicht deklariert). Verpflegung/Ausland-Sonderfälle (E020xxxx,
Sektion Inl/Ausl) analog zur Anlage-N-Verpflegung, hier nicht MVP-kritisch.

---

## (2) Anlage Haushaltsnahe Aufwendungen — p35a (Topf-Trennung geprüft)

Quelle: `sources/bfinv/haushaltsnahe_2025.txt`. **Instructor-Check bestanden:** der Vordruck trennt
GENAU DREI Töpfe, deckungsgleich mit unseren drei p35a-Inputs:

| p35a-Input (Topf) | Vordruck-Zeile | Form-Kz | XSD E-Nr (Summe) | wörtliches Label | Konfidenz |
|---|---|---|---|---|---|
| Minijobs (geringfügige Beschäftigung Privathaushalt) | Zeile 4 | 202 | E0161404 | Summe der Aufwendungen (Minijob-Topf) | **STARK** |
| haushaltsnahe Beschäftigungen / Dienstleistungen / Pflege | Zeile 5 | 212 | E0161504 | Summe der Aufwendungen (Dienstleistungs-Topf) | **STARK** |
| Handwerkerleistungen | Zeile 6 | — | E0161804 | Summe der Aufwendungen (Handwerker-Topf) | **STARK** |

Befund: die drei getrennten Höchstbeträge/Sätze des § 35a (20 % / 20 % / 20 % mit je eigenem Cap:
Minijob 510 €, Dienstleistungen 4.000 €, Handwerker 1.200 €) mappen 1:1 auf drei getrennte Vordruck-
Töpfe → unser 3-Input-Zuschnitt ist amtlich gedeckt. Das Vordruck-Feld nennt „Aufwendungen abzüglich
[Erstattungen]" → Erstattung ist schon im deklarierten Betrag verrechnet (nicht separates Feld).

---

## (3) Anlage Außergewöhnliche Belastungen — p33

Quelle: `sources/bfinv/agb_2025.txt`. Primär Vordruck.

| Regel-Input | Vordruck-Zeile | Form-Kz | XSD E-Nr | wörtliches Label | Konfidenz |
|---|---|---|---|---|---|
| p33: andere agB allgemeiner Art (§33) | Zeile 24 „Andere Aufwendungen / Summe" | 302 | E0104109 / E0107208 (Sektion Sum) | Summe der Aufwendungen (abzüglich Erstattungen) | **STARK** |
| p33_3: zumutbare Belastung | — (KEIN Feld) | — | — | wird vom FA/unserer Regel BERECHNET, nicht deklariert | — |

Befund: „Summe der Aufwendungen (**abzüglich Erstattungen**)" → Erstattung ist im deklarierten Betrag
verrechnet (kein separates Feld). zumutbare Belastung ist Rechenlogik (p33_3), kein Vordruck-Input →
deckt sich mit unserem Zuschnitt.

---

## (4) Anlage Sonderausgaben — p10_1_4 (KiSt), p10_1_7 (Berufsausbildung)

Quelle: `sources/bfinv/sonderausgaben_2025.txt`.

| Regel-Input | Vordruck-Zeile | Form-Kz | wörtliches Label | Konfidenz |
|---|---|---|---|---|
| p10_1_4: **gezahlte** Kirchensteuer | Zeile 4 „2025 gezahlt" | **103** | Kirchensteuer … gezahlt | **STARK (Vordruck)** |
| p10_1_4: **erstattete** Kirchensteuer | Zeile 4 „2025 erstattet" | **104** | Kirchensteuer … erstattet | **STARK (Vordruck)** |
| p10_1_7: Aufwendungen eigene Berufsausbildung | Zeile ~53 | (§ 10 Abs. 1 Nr. 7) | Aufwendungen für die eigene Berufsausbildung | **STARK (Vordruck)** |

**Instructor-Check bestanden:** KiSt gezahlt (103) UND erstattet (104) sind ZWEI getrennte Vordruck-
Felder = exakt p10_1_4's zwei Inputs (das Erstattungsfeld ist real, netto-relevant). XSD-E-Nr für 103/104
als Sektions-Lookup-Nachtrag (Label-Phrasing weicht ab, kein Rate-Mapping).

---

## (5) Anlage Vorsorgeaufwand — p10 v1/v2 (AN/AG-Trennung geprüft)

Quelle: `sources/bfinv/vorsorgeaufwand_2025.txt`. **Instructor-Check bestanden:** der Vordruck
DEKLARIERT Arbeitnehmer- und Arbeitgeberanteil GETRENNT:

| Regel-Input | Vordruck-Zeile | Form-Kz (A/B) | wörtliches Label | Konfidenz |
|---|---|---|---|---|
| p10 v1/v2: Arbeitnehmeranteil geset. RV | Zeile 4 „Arbeitnehmeranteil laut Nr. 23 a/b LStB" | 300 / 400 | Arbeitnehmeranteil laut Nr. 23 a / b der Lohnsteuerbescheinigung | **STARK (Vordruck)** |
| p10 v1/v2: Arbeitgeberanteil / -zuschuss | Zeile 5 „Arbeitgeberanteil / -zuschuss laut Nr. 22 a/b LStB" | (getrennt) | Arbeitgeberanteil / -zuschuss laut Nr. 22 a / b der Lohnsteuerbescheinigung | **STARK (Vordruck)** |
| p10 v1/v2: Basis-KV/PV inländisch gesetzlich | Zeile ~9 | — | Beiträge zur inländischen gesetzlichen Kranken- und Pflegeversicherung | **STARK (Vordruck)** |

**Validiert unsere `gesamtbeitraege_inkl_ag`-Semantik amtlich:** weil AN- und AG-Anteil getrennt
deklariert werden, kann unsere Regel beide als Inputs führen und den AG-Zuschuss korrekt behandeln —
der Vordruck stützt den Zuschnitt eins zu eins. XSD-E-Nr als Nachtrag.

---

## (6) Anlage Kind — p24b (Entlastungsbetrag), p31/p32 (Kindergeld/Freibetrag)

Quelle: `sources/bfinv/kind_2025.txt`. Je Kind eine Anlage.

| Regel-Input | Vordruck-Zeile | wörtliches Label | Konfidenz |
|---|---|---|---|
| p31/p32: Anspruch auf Kindergeld | „Anspruch auf Kindergeld oder ver[gleichbare Leistungen]" | Anspruch auf Kindergeld … | **STARK (Vordruck)** |
| p32: Übertragung Kinderfreibetrag/BEA | Zeile ~ „Übertragung des Kinderfreibetrags / Freibetrags für Betreuung/Erziehung/Ausbildung" | Übertragung des Kinderfreibetrags … | **STARK (Vordruck)** |
| p24b: Entlastungsbetrag Alleinerziehende | „Entlastungsbetrag für Alleinerziehende" | Entlastungsbetrag für Alleinerziehende | **STARK (Vordruck)** |
| p31: Kindergeld-Auszahlungszeitraum | Zeile 46 (Kz 44) „Für das Kind wurde mir Kindergeld ausgezahlt im Zeitraum" | Für das Kind wurde mir Kindergeld ausgezahlt … | **STARK (Vordruck)** |

Befund: die p24b/p31/p32-Konzepte sind alle im Vordruck verankert. Kinderfreibetrag/BEA werden vom FA
BERECHNET (Freibeträge, nicht deklarierter Betrag); die Anlage Kind deklariert die
Anspruchs-/Übertragungs-Sachverhalte → deckt sich mit unseren Regeln (die die Freibeträge rechnen).

---

## Zusammenfassung (rollierend)

Alle sechs Anlagen eingefroren (`sources/bfinv/`, sources-check grün). Alle Instructor-Struktur-Checks
BESTANDEN: N-DHF-Trennung, Haushaltsnahe-3-Töpfe, agB-Erstattung-verrechnet, KiSt gezahlt/erstattet
getrennt, Vorsorge AN/AG getrennt (validiert gesamtbeitraege_inkl_ag), Kind-Konzepte verankert.
STARKE Konzept↔Zeile-Mappings via Vordruck; XSD-E-Nr verbatim wo Label deckt (N-DHF/Haushaltsnahe/agB
komplett), sonst Sektions-Lookup-Nachtrag (Sonderausgaben/Vorsorge — kein Rate-Mapping).
