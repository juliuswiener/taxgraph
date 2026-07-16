# DBA-Vollausbau — Wellen-Plan (Paket 6 / Backup-Task, Stufe A)

Autor: taxgraph-instructor, 2026-07-16. Status: BACKUP-Priorität (Julius: nur bearbeiten,
wenn andere Arbeit geblockt ist). Grundlage: Scraper-Korpus 168_TaxGraph
(529 BMF-PDFs `dba_pdfs/`, 132 NWB-Volltexte `dba_text_nwb/`, 11 MinerU-OCR, README_DBA.md).

## Quellen-Doktrin

- **Anker-Grundlage NUR amtlich**: BMF-PDF (wenn Text-Layer; BEL-Fix-Route für
  U+0007-Layer) oder BGBl-Faksimile. Von 529 PDFs tragen 363 Text-Layer.
- **NWB-Volltexte = Lesehilfe/Zweitbeleg/Fassungs-Navigator** (konsolidierte i.d.F. mit
  Fundstellen) — NIE Anker-Grundlage (nicht amtlich).
- Je Staat Gültigkeits-Check: Änderungsprotokolle, **MLI-Status**, VZ-Splits im
  Fenster 2024–2026, Sonderstatus (gekündigt/suspendiert/ausgelaufen).

## Bestand produktiv (6): AT, US, CH, FR, LU, NL — W4-Katalog-Muster etabliert.

## Wellen

| Welle | Staaten | Begründung |
|---|---|---|
| **W1 (8)** | PL, IT, ES, BE, DK, CZ, GB, TR | höchste Praxisrelevanz dt. ESt-Erklärungen (Arbeitnehmer-Entsendung, Renten, Immobilien, große Diaspora); EU-Nachbarn + GB/TR |
| **W2 (12)** | PT, GR, HR, RO, HU, SK, SI, BG, SE, NO, FI, IE | EU/EWR-Rest mit relevantem Pendler-/Renten-/Immobilienaufkommen |
| **W3 (Sonderstatus, 4)** | RU (SUSPENDIERT seit 2023 — Teilaussetzung prüfen!), BR (GEKÜNDIGT 2005 → kein DBA → reiner § 34c), VAE (AUSGELAUFEN → § 34c), plus 1 Prüffall Hongkong (nur Schiff/Luft) | Negativ-/Sonderkatalog: `dba_vorhanden=false`-Pfad ist eigene Geltungsbedingungs-Klasse und produktiv wichtig (Anrechnung pur) |
| **W4+ (Langschwanz)** | Rest (~70) chargenweise alphabetisch nach Bedarf | geringe Einzelrelevanz; Korpus liegt bereit, verdirbt nicht |

## Ablauf je Staat (W4-Muster wie FR/LU/NL)

1. Quellen-Sichtung: BMF-PDF-Bestand je Staat (Text-Layer? BEL? Scan+NWB-Pendant?),
   Fassungskette aus NWB-Fundstellen ableiten, amtlich verifizieren.
2. Instructor-Freeze der Anker-tragenden Dokumente (Grundtext + wirksame Protokolle).
3. dev: Methoden-Katalog (Einkunftsart → Freistellung→p32b / Anrechnung→p34c),
   Anker voll-Länge, VZ-Splits als Overlay (CH/NL-Muster), Anker-Gate-Skript.
4. Instructor-Nachlauf.

## Kosten/Takt

LLM-frei (Methoden-Zuordnung deterministisch), $0 — Aufwand ist Freeze-/Prüf-Zeit.
Takt: W1 als erste Charge (8 Staaten ≈ 2–3 Arbeitssitzungen im Backup-Modus).
Pausiert sofort, wenn Haupt-Arbeit (Hersteller-ID → Runde 3/Produktisierung,
neue Julius-Pakete) eintrifft.
