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
| Werbungskosten nichtselbst. Arbeit (Entfernung, Arbeitsmittel, Reisekosten, Übernachtung+Verpflegung bei Auswärtstätigkeit) | p09, nr6_7, **nr5a** (Übernachtung Auswärtstätigkeit), p9_4a (Verpflegung) | **Anlage N** | ✅ hier |
| doppelte Haushaltsführung | **p9_1_3_nr5** (NICHT nr5a!) | **Anlage N-Doppelte Haushaltsführung** (eigene Anlage, ID 034027d_25, Vordruck-Zeile 288) | ⏳ Julius lädt |
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
| **nr5a**: Übernachtungskosten (Auswärtstätigkeit) | Zeile 67 „Übernachtungskosten" | — | E0206301 / E0226104 | Uebernacht | Übernachtungskosten | **STARK** |

¹ Form-Kennzahl aus pdftotext, spaltenweise unsicher — Orientierung, nicht Identifier.
² Kollidiert scheinbar mit Bruttoarbeitslohn-Kz 110 (Zeile 5) — pdftotext-Layout-Artefakt, deshalb
E-Nr maßgeblich; die E-Nr trennt die Konzepte eindeutig.
³ 320 = Summenfeld Arbeitsmittel (Zeile 56 „Summe").

## Betragsfeld-Nachtrag — Reisekosten/Verpflegung (nr5a, p9_4a), inkl. Arbeitgeber-Erstattung

Instructor-Auflage: je Position gehören die **Erstattungs-Felder** („vom Arbeitgeber steuerfrei
ersetzt") mit in die Tabelle — unsere Regeln rechnen NETTO (erstattete Beträge mindern), Phase 5
muss sie abfragen. Alle Zeilen STARK (Vordruck ⋂ XSD verbatim):

| Regel-Input | Vordruck-Zeile | XSD E-Nr (A / B) | XSD-Sektion | wörtliches Label | Art | Konfidenz |
|---|---|---|---|---|---|---|
| nr5a: Übernachtungskosten (Betrag) | Zeile 67 | E0206301 / E0226104 | Uebernacht | Übernachtungskosten | Aufwand € | **STARK** |
| nr5a/Reisek.: Reisenebenkosten | Zeile 68 | E0206402 / E0226207 | Reisenebenk | Reisenebenkosten | Aufwand € | **STARK** |
| nr5a/Reisek.: **Arbeitgeber-Erstattung** (mindert) | Zeile 71 (Kz 420) | E0205608 / E0224703 | Rk_Ersatz | Vom Arbeitgeber steuerfrei ersetzt | Erstattung € | **STARK** |
| p9_4a: Tage Abwesenheit > 8 h | Zeile 72 (Kz 470) | E0205201 | Inl | Anzahl der Tage mit einer Abwesenheit von mehr als 8 Stunden … | Anzahl | **STARK** |
| p9_4a: Tage An-/Abreise | Zeile 73 (Kz 471) | E0205302 | Inl | Anzahl der An- und Abreisetage … | Anzahl | **STARK** |
| p9_4a: Tage Abwesenheit 24 h | Zeile 74 (Kz 472) | E0205409 | Inl | Anzahl der Tage mit einer Abwesenheit von 24 Stunden | Anzahl | **STARK** |
| p9_4a/nr5a: Kürzung Mahlzeitengestellung | Zeile 75 (Kz 473) | E0205508 | Inl | Kürzungsbeträge wegen Mahlzeitengestellung … | Kürzung € | **STARK** |
| p9_4a: **Verpflegung Arbeitgeber-Erstattung** (mindert) | Zeile 77 (Kz 490) | E0205108 / … | VMA_Ersatz | Vom Arbeitgeber steuerfrei ersetzt | Erstattung € | **STARK** |

Anmerkung: „Kürzung Mahlzeitengestellung" (E0205508) berührt direkt unsere nr5a-Geltungsbedingung
`keine_mahlzeitengestellung` / §9 Abs. 4a S. 8 — amtlicher Anker für die Bedingung, nicht nur Betrag.
Verpflegung ist bei uns kein €-Input, sondern Tage×Pauschale (p9_4a rechnet die Pauschbeträge selbst)
→ die Anzahl-Kz (470/471/472) sind die Deklarations-Inputs, nicht ein €-Betrag.

### Validierungs-Notizen (amtliche Deckung, Instructor 2026-07-12)

- **p9_4a-Signatur ↔ amtliche Deklarationsstruktur DECKUNGSGLEICH:** der Vordruck-Wortlaut („Anzahl
  der Tage mit Abwesenheit > 8 h **ohne** Übernachtung" / „An- und Abreisetage bei mehrtägiger
  Auswärtstätigkeit **mit** Übernachtung" / „24 Stunden") ist eins zu eins unsere Tages-Kategorisierung
  — inklusive der nachts adjudizierten Übernachtungs-Semantik. Das amtliche Formular BESTÄTIGT die
  Signatur; kein Zuschnitt-Zweifel mehr an p9_4a.
- **Phase-5-Vorgriff:** E0205508 (Kürzung Mahlzeitengestellung) ist später die Interview-/Abfrage-
  Kennzahl zur Geltungsbedingung `keine_mahlzeitengestellung` — die Engine fragt genau dieses Feld
  ab, um die Bedingung zu prüfen. Amtlicher Anker Bedingung↔Kz steht damit.

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
