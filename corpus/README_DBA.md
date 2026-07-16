# DBA-Korpus — Deutsche Doppelbesteuerungsabkommen

Stand: 2026-07-16. Quelle-PDFs: Bundesfinanzministerium (BMF). Volltexte: NWB Gesetze (frei) + lokales OCR.
Seit 2026-07-16 unter `taxgraph/corpus/` (vorher Projekt-Root 168_TaxGraph). Skripte arbeiten
`__file__`-relativ, Layout unverändert.

## Struktur

| Ordner | Inhalt | Anzahl | Git |
|---|---|---|---|
| `dba_pdfs/` | Alle BMF-Original-PDFs (staatenbezogene Informationen), amtlich, inkl. alte BGBl-Scans; zzgl. `2024-01-15-stand-DBA-1-januar-2024.pdf` + NL-2012-Gesetz (manuelle Downloads, einsortiert) | 531 | lokal (225M, reproduzierbar via `scrape_dba.py`) |
| `dba_text_nwb/` | Saubere konsolidierte Volltexte (Markdown), aktuelle Fassung + Red.-Anmerkungen + Fundstellen | 132 | lokal (NWB-Urheberrecht — NIE committen) |
| `dba_text_mineru/` | OCR-Volltexte (Markdown) für Abkommen OHNE NWB-Pendant | 11 | committet |
| `kommentare/` | Kommentar-Literatur (Kirchhof/Seer EStG 22. Aufl. 2023, Rödder/Herlinghaus/Neumann KStG 2. Aufl. 2023, HHR-Jahresband 2011, Wassermeyer/Baumhoff Verrechnungspreise 2014) — Auslegungs-Zweitbeleg, NIE Anker-Grundlage | 4 | lokal (Urheberrecht — NIE committen) |
| `logs/` | Scrape-/Extraktions-/OCR-Logs (Provenance) | 3 | committet |

## Herkunft & Methode

**PDFs** (`scrape_dba.py`): BMF-Seite ist hinter Radware-Bot-Wall. Umgangen via requests-Session
(Übersicht laden → `__uzm*`-Cookies → Referer/Browser-Header). Übersicht → 134 Länderseiten →
Artikelseiten → Blob-PDF (`?__blob=publicationFile`). Pfad-Filter `Staatenbezogene_Informationen`
hält BMF-News-Teaser raus. 0 Download-Fehler.

**NWB-Volltexte** (`extract_nwb.py`): NWB Gesetze serviert DBA-Artikeltexte frei unter
`datenbank.nwb.de/Dokument/<docid>_<N>/` (kein Login; N arabisch oder römisch bei alten Abkommen).
Index über gespeicherte Suche (`nwb_index.json`, 132 DBAs). Pro Land: Präambel + jeder Artikel →
`dokumentinhaltcontent`-Container → Markdown. Konsolidierte i.d.F.-Fassung, besser als die BGBl-Scans.

**OCR-Lücken** (`check_ocr.py` → `mineru_gaps.json`): Von 529 PDFs sind 166 reine Scans ohne
Textlayer (alte BGBl `-Gesetz`/`-Bekanntmachung`). 155 davon haben ein NWB-Pendant (sauberer Text
existiert → kein OCR). Nur **11 echte Lücken** per MinerU (pipeline-Backend, GPU-OCR, `deu`):
- **Brasilien** (DBA 1975 + Schifffahrt 1951) — DBA 2005 von DE gekündigt, nicht in NWB
- **VAE** (DBA 1996 + Gegenseitigkeitsfeststellung 2022) — ausgelaufen, nicht in NWB
- **Kolumbien** (Schifffahrt/Luftfahrt 1967) — kein Einkommen-DBA
- **Hongkong** (Luftfahrt 1998 + Schifffahrt 2004)

MinerU-Hinweis: Default-Backend `hybrid-engine` (VLM) war zu langsam (~2h) + crashte; `-b pipeline -m ocr`
löst beides (Minuten, sauberes deutsches Markdown).

## Skripte

- `scrape_dba.py` — PDF-Scraper (BMF)
- `extract_nwb.py` — NWB-Volltext-Extraktor
- `check_ocr.py` — Textlayer-Analyse (welche PDFs gescannt)
- `nwb_index.json` / `mineru_gaps.json` — Index bzw. Lückenliste
