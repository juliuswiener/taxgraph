# Kz-Kandidaten-Tabelle — Anlage N 2025 (erstes Paket, Instructor-Review)

Multi-Source (Instructor 2026-07-12): **Primär = amtlicher Vordruck** (Anlage N 2025, eingefroren
`sources/bfinv/anlage_n_2025.txt`, sha256-verifiziert), **Zweitbeleg = XSD E10-2025** (`elster/kz_extract.py`,
Sektions-Pfad). Konfidenz: **STARK** = Vordruck-Zeilentext UND XSD-Label zeigen dasselbe Konzept
(verbatim); MITTEL = nur eine Quelle; KONFLIKT = Eskalation.

**Identifier-Hinweis:** der zuverlässige Submission-Feld-Identifier ist die **XSD-E-Nummer**
(Paar A/B = Ehemann/Person A `E020xxxx` / Ehefrau/Person B `E022xxxx`, zwei Formularspalten). Die
3-stellige Form-Kennzahl aus dem Vordruck ist per pdftotext-Layout spaltenweise UNSICHER → nur als
Orientierung geführt, nicht als primärer Identifier. Das ELSTER-Submission-XML nutzt die E-Nummer.

## Konzept → Anlage-Übersicht (Struktur-Befund, wichtig)

ESt 1 A ist seit der 2019er-Reform schlank; viele MVP-Konzepte leben in EIGENEN Anlagen (von Julius
noch zu exportieren):

| Konzept | Regel | Anlage | Status |
|---|---|---|---|
| Werbungskosten nichtselbst. Arbeit (Entfernung, Arbeitsmittel, Reisekosten) | p09, nr6_7, p9_4a | **Anlage N** | ✅ hier |
| doppelte Haushaltsführung (Übernachtung) | nr5a | **Anlage N-Doppelte Haushaltsführung** (eigene Anlage, Vordruck-Zeile 288) | ⏳ Export offen |
| Sonderausgaben (KiSt, §10er) | p10_1_4, p10_1_2 | Anlage Sonderausgaben / Vorsorgeaufwand | ⏳ Export offen |
| außergewöhnliche Belastungen | p33 | **Anlage Außergewöhnliche Belastungen** (nicht mehr in ESt1A) | ⏳ Export offen |
| haushaltsnahe (§35a) | p35a | Anlage Haushaltsnahe Aufwendungen | ⏳ Export offen |
| Kind (§31/32/24b) | p31, p32_6, p24b | Anlage Kind | ⏳ Export offen |

## Anlage N — Kandidaten (STARK, Vordruck ⋂ XSD verbatim)

| Regel-Input | Vordruck-Zeile (Konzept) | Form-Kz¹ | XSD E-Nr (A / B) | XSD-Sektion | wörtliches Label | Konfidenz |
|---|---|---|---|---|---|---|
| p09: aufgesuchte Arbeitstage | Zeile 29 „aufgesucht an Tagen" | 110² | E0203503 / E0223701 | Erste_Taetig | aufgesucht an Tagen | **STARK** |
| p09: einfache Entfernung (km) | Zeile 30 „einfache Entfernung in Kilometern (auf volle Kilometer abgerundet)" | 111 | E0203504 / E0223702 | Erste_Taetig | einfache Entfernung in Kilometern (auf volle Kilometer abgerundet) | **STARK** |
| p09: ÖPNV-Fahrtkosten | Zeile 34 „Aufwendungen für Fahrten mit öffentlichen Verkehrsmitteln" | 114 | E0203611 / E0223405 | Erste_Taetig | Aufwendungen für Fahrten mit öffentlichen Verkehrsmitteln (ohne Fähr- und Flugkosten) | **STARK** |
| nr6_7: Arbeitsmittel (Art) | Zeile 54 „Art der Arbeitsmittel" | — | E0204401 / E0224401 | Einz | Art der Arbeitsmittel | **STARK** |
| nr6_7: Arbeitsmittel (Summe/Betrag) | Zeile 56 „Summe" Arbeitsmittel | 320³ | (E-Nr Betragsfeld, s. Nachtrag) | Einz/Sum | — | MITTEL |
| p9_4a: Übernachtungskosten | Zeile 67 „Übernachtungskosten" | — | E0206301 / E0226104 | Uebernacht | Übernachtungskosten | **STARK** |

¹ Form-Kennzahl aus pdftotext, spaltenweise unsicher — Orientierung, nicht Identifier.
² Kollidiert scheinbar mit Bruttoarbeitslohn-Kz 110 (Zeile 5) — pdftotext-Layout-Artefakt, deshalb
E-Nr maßgeblich; die E-Nr trennt die Konzepte eindeutig.
³ 320 = Summenfeld Arbeitsmittel (Zeile 56 „Summe").

## Offene Punkte (Instructor / Nachtrag)

- **Betragsfelder** (die eigentlichen €-Kz je Werbungskosten-Position) sauber je Zeile zuordnen —
  die STARKEN Zeilen oben treffen das Konzept/den Struktur-Anker; die zugehörigen €-Betrags-E-Nr
  liefere ich im Nachtrag mit gezieltem Vordruck-Zeilen↔XSD-Sequenz-Abgleich (die E-Nr laufen je
  Sektion sequentiell).
- **Person A/B**: je Konzept ein E-Nr-Paar (Ehemann/Ehefrau). Unsere Regeln sind personen-agnostisch
  → beide Spalten sind dasselbe Konzept; die Signatur mappt auf die Person-A-Spalte (Default), B
  analog.
- **dHf (nr5a)**: eigene Anlage N-Doppelte Haushaltsführung — wartet auf Julius-Export.

## Nächste Schritte

1. Instructor-Review dieser STARKEN Zeilen (Konzept↔E-Nr).
2. Betragsfeld-E-Nr-Nachtrag (Sequenz-Abgleich).
3. Anlage N-Doppelte Haushaltsführung + die vier weiteren Anlagen, sobald Julius sie exportiert.
