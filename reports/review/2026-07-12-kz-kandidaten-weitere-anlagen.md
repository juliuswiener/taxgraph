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
