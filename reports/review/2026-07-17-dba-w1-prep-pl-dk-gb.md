# DBA W1-Vorbereitung PL / DK / GB (AUFTRAG 3, Stufe A — Sichtung, KEINE Freezes)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Quelle:** `corpus/dba_pdfs/` (amtliche BGBl-PDFs). Ziel: je Staat regierende Fassung + Text-Layer-Route
+ Methodenartikel + Anker-Kandidaten. Zitatanker MÜSSEN amtlich (PDF-Textlayer/BEL-Fix/OCR), NWB nur
Zweitbeleg. **Keine Freezes** — das ist Vorbereitung, kein Katalog-Bau.

## Übersicht (Routen-Matrix)

| Staat | Grundabkommen | Body-Text-Layer | **Route** | Methodenartikel | Änderungen im Korpus |
|---|---|---|---|---|---|
| **PL** | 2004-04-29 (BGBl 2004 II Nr. 29) | **Bildscan** | **OCR** (tesseract-deu) | **Art. 24** | 2015 Verständigungsvereinbarung |
| **DK** | 1996-11-06 (BGBl 1996 II S. 2565) | **Bildscan** | **OCR** + 2021-Prot. direkt | **Art. 24, geändert 2021** | 2021 Protokoll + 2021 Berichtigung + 2022 Bekanntmachung |
| **GB** | 2010-11-23 (BGBl 2010 II) | **sauber** (159k chars) | **direct pdftotext** | **Art. 23** | 2021 Änderungsgesetz + 2022 Bekanntmachung |

tesseract vorhanden (`deu`+`eng`). OCR-Route **empirisch bewiesen**: PL-Body-Seite 10 → sauberer
deutscher Vertragstext (Art. 12 Lizenzgebühren). Alle drei Abkommen sind zweisprachig (DE/PL, DE/DK,
DE/EN) — für den amtlichen deutschen Anker OCR/pdftotext auf der **deutschen Spalte**.

## Polen

- **Regierende Fassung:** DBA-Polen 2004 (Zustimmungsgesetz vom 15.09.2004, BGBl 2004 II Nr. 29, ausgeg.
  20.09.2004; in Kraft ab 2005). 25 Seiten, zweisprachig DE/PL.
- **Text-Layer-Befund (KORREKTUR gegenüber Erst-Sichtung):** Der **Vertragstext-Body ist ein BILDSCAN**.
  Seiten 5–20 tragen je nur ~97 Zeichen = ausschließlich die laufende Kopfzeile
  („Bundesgesetzblatt Jahrgang 2004 Teil II Nr. 29…"). Die 13.369 Gesamt-Zeichen aus `pdftotext` stammen
  nur aus Cover (S. 1) + Denkschrift/Protokoll (S. 25, echter Text-Layer). Der Artikel-Body braucht **OCR**.
- **Methodenartikel:** **Art. 24** (Vermeidung der Doppelbesteuerung) — auf S. 25 (Denkschrift, Text-Layer)
  referenziert als „Artikel 24 Absatz 1 Buchstabe b". Der **Wortlaut** des Art. 24 liegt im gescannten
  Body → Anker per OCR (deutsche Spalte).
- **Anker-Kandidat:** noch KEIN verifizierter Volltext-Anker (OCR ausstehend). Fundstelle steht fest
  (Art. 24 Abs. 1 Buchst. a/b). OCR-Feasibility bewiesen.
- **MLI / Gültigkeit:** kein bilaterales Änderungsprotokoll zum Methodenartikel im Korpus; nur eine
  2015er Verständigungsvereinbarung (Max-Weber-Stiftung, Sonderfall). **MLI-Anwendbarkeit separat prüfen**
  (BMF-MLI-Anwendungsschreiben; DE+PL sind MLI-Vertragsstaaten) — kein MLI-PDF im Korpus.

## Dänemark

- **Regierende Fassung:** „Deutsch-dänisches Steuerabkommen" vom 22.11.1995 (BGBl 1996 II S. 2565;
  Zustimmungsgesetz 1996-11-06; in Kraft 1997). Umfasst Einkommen/Vermögen **und** Nachlass-/Erbschaft-/
  Schenkungsteuer + Beistandsleistung. Zweisprachig DE/DK.
- **Text-Layer-Befund:** 1996-Grundabkommen = **reiner BILDSCAN** (32 Zeichen `pdftotext`) → **OCR**.
- **Methodenartikel:** **Art. 24** — **AKTUELL GEÄNDERT durch das 2021-Protokoll**: das 2021-Gesetz hebt
  wörtlich „Artikel 24 Absatz 1 Buchstabe a Satz 2" auf. Der geltende Methodenartikel ist also
  **1996 Art. 24 (OCR) ∘ 2021-Änderung (sauberer Text-Layer)** — beide Fassungen zusammen.
- **Anker-Kandidat:** 2021-Änderungstext direkt verfügbar (58.922 Zeichen, sauber) — Änderungsbefehl an
  Art. 24 amtlich zitierbar OHNE OCR. Der Basis-Art.-24-Wortlaut (1996) per OCR. ⚠ Altfassung aus
  Änderungsbefehl-Freeze ist planbar heikel (vgl. [[altfassung-aenderungsbefehl-judge-artefakt]]) —
  konsolidierte Fassung sauber führen.
- **Gültigkeit:** 2021 Protokoll (06.02.) + 2021 Berichtigung (10.12.) + 2022 Bekanntmachung (24.02.).
  Der Stand ist der jüngste im Korpus. **MLI separat prüfen.**

## Großbritannien

- **Regierende Fassung:** DBA-GB 2010 (Zustimmungsgesetz 2010-11-23, BGBl 2010 II; in Kraft 2011; löste
  das alte 1964/1971er Abkommen ab). 26 Seiten, zweisprachig DE/EN.
- **Text-Layer-Befund:** **voll sauberer Text-Layer** (159.085 Zeichen, kein U+0007) → **direct pdftotext**,
  kein OCR, kein BEL-Fix.
- **Methodenartikel:** **Art. 23** (Vermeidung der Doppelbesteuerung / Article 23).
- **Anker-Kandidaten (amtlich, direkt extrahiert):**
  - Freistellung: „… von der deutschen Steuer **ausgenommen** …" (Freistellungsmethode für bestimmte
    Einkünfte/Vermögen).
  - Anrechnung: „… wird … die **Steuer des Vereinigten Königreichs angerechnet** …" (Anrechnungsmethode,
    Buchst. b).
  - Progressionsvorbehalt: „Deutschland behält aber das Recht, die … ausgenommenen Einkünfte … bei der
    **Festsetzung seines Steuersatzes** zu berücksichtigen." (Art. 23 Buchst. d).
- **Gültigkeit:** 2021 Änderungsgesetz (23.07., 17.701 Zeichen, Text-Layer) + 2022 Änderungs-Bekanntmachung
  (03.03.). **MLI separat prüfen** (DBA 2010 ist bereits post-2010, aber MLI-Overlay möglich).

## Zusammenfassung für die Freeze-Phase (W1, wenn beauftragt)

1. **GB zuerst** — sauberer Text-Layer, Anker direkt zitierbar (Art. 23 Freistellung/Anrechnung/
   Progressionsvorbehalt), 2021-Änderung ebenfalls Text-Layer. Geringstes Risiko.
2. **DK** — konsolidierte Art. 24 = 1996-Basis (OCR) ∘ 2021-Änderung (direkt). Änderungsbefehl-Doktrin
   beachten (Alt-/Neufassung sauber trennen).
3. **PL** — reiner OCR-Fall für den Body; Feasibility bewiesen, aber jeder Anker MUSS voll via `_normalize`
   gegen den OCR-Text geprüft werden (vgl. [[anker-verifikation-volllaenge]], [[bgbl-scan-textlayer-ocr-route]]).
4. **MLI-Overlay** ist für ALLE drei ungeklärt (kein MLI-PDF im Korpus) → vor jedem Freeze der
   MLI-Anwendbarkeits-Status je Staat gegen das BMF-MLI-Anwendungsschreiben prüfen
   (vgl. [[gueltigkeits-check-direktive]]).

Keine Freezes ausgeführt (auftragsgemäß). Anker-Kandidaten nur GB direkt verifizierbar; PL/DK erst nach OCR.
