# DBA W1-Quellen-Sichtung (Paket 6, dev-2, Backup-Modus)

taxgraph-dev-2, 2026-07-16. Read-only-Sichtung der W1-Staaten (PL, IT, ES, BE, DK, CZ,
GB, TR) im DBA-Korpus `/home/julius/00_projects/168_TaxGraph/{dba_pdfs,dba_text_nwb}/`
(529 BMF-PDFs, 132 NWB-Volltexte; README_DBA.md). LLM-frei, $0. **KEINE Freezes**
(Instructor-Zone), **KEINE Kataloge** (dev-1 nach Freeze). Alle Fassungsketten = HYPOTHESE,
amtliche Verifikation beim Freeze.

## Kern-Befund vorab
- **KEIN W1-Staat in den 11 MinerU-OCR-Lücken** (die betreffen nur BR/CO/VAE/HK). ⇒ **alle 8
  W1-Staaten haben einen sauberen NWB-Volltext (konsolidierte i.d.F.) — kein OCR nötig.**
- **2 BEL-Verdachts-PDFs** (U+0007-kodierte Wortabstände, Fix `tr '\a' ' '`, KEIN OCR):
  Spanien `2012-01-20-…-DBA-Gesetz.pdf` (428× U+0007), Türkei `2012-05-30-…-DBA-Gesetz.pdf`
  (456× U+0007). Muster wie [[bgbl-scan-textlayer-ocr-route]].
- **Reine Scans ohne Textlayer** (IT-1990, ES-1968, DK-1996, CZ-alle, GB-1966/71, TR-1989):
  alle mit NWB-Pendant → NWB nutzen, kein OCR.
- **Methodenartikel uneinheitlich nummeriert** (Art. 22/23/24) — je Staat einzeln ankern
  (wie FR-Abweichung).
- **MLI**: die NWB-Konsolidate zeigen KEINEN MLI-Overlay. HYPOTHESE: DE setzt DBA-Änderungen
  überwiegend bilateral (Protokolle) statt MLI-synthetisiert um; MLI-Wirksamkeit je Staat
  **separat amtlich zu prüfen** (BMF-MLI-Anwendungsliste). Für W1 vorläufig: kein wirksamer
  MLI-Overlay im VZ-Fenster angenommen — Instructor-Verifikation.

## Staaten-Tabelle

| Staat | PDFs | Textlayer-Status Kern-DBA | Basis-Fassung (NWB i.d.F.) | Fassungskette VZ 2024–2026 (HYPOTHESE) | Methodenart. | Anker-Quelle (Empfehlung) | Besonderheiten |
|---|---|---|---|---|---|---|---|
| **PL** Polen | 3 | DBA-Gesetz 2004 = TEXT | 14.05.2003 (Gesetz 2004) | stabil, kein VZ-Split erkennbar | Art. 24 | NWB Polen.md (clean); PDF-Text als Cross-Check | Verständigungsvereinb. 2015 (Max-Weber-Stiftung, Nische) |
| **IT** Italien | 5 | DBA-Gesetz 1990 = **SCAN** | 18.10.1989 (Gesetz 1990) | stabil, kein Split | Art. 24 | **NWB Italien.md** (PDF Scan, kein OCR — NWB-Pendant) | Erstattungs-Verfahren-PDF 2003 (Nische) |
| **ES** Spanien | 6 | DBA-Gesetz 2012 = **BEL** (428×U+0007) | 03.02.2011 (Gesetz 2012) | stabil, kein Split | Art. 22 | NWB Spanien.md (clean) ODER PDF via `tr '\a' ' '` | BEL-Route; Ortskräfte-/Amtshilfe-Verständ. 2013 |
| **BE** Belgien | 10 | Kern-DBA-PDF nicht im Bestand; 10× KonsVerh | 21.01.2010 (Zusatzabk. 5.11.2002, BGBl 2003 II S. 1616 mod. Art. 15) | Basis stabil; COVID-Grenzpendler-KonsVerh 2020–2022 ausgelaufen (nicht VZ-relevant) | Art. 23 (zu pinnen) | **NWB Belgien.md** (Kern-DBA-PDF fehlt im dba_pdfs!) | ⚠ Kern-DBA-PDF-Lücke; Grenzpendler-COVID-Serie; NL/FR-Zweisprachigkeit möglich; „Grenz"-Treffer = Grenzabfertigung, NICHT Steuer-Grenzgänger |
| **DK** Dänemark | 6 | DBA-Gesetz 2021 = TEXT (1996 = SCAN) | 22.11.1995 (Gesetz 1996) | ⚠ **Änderungsprotokoll wirksam 23.12.2021** → VZ 2024–2026 = 2021-Fassung (Berichtigung 2021-12, Bekanntm. 2022-02) | Art. 24 (+ Art. 26) | NWB Dänemark.md (2021-Fassung) + PDF 2021-Gesetz (TEXT) | 2021-Änderungsschicht; Amtshilfe-Absprache 2005 |
| **CZ** Tschechien | 3 | alle = **SCAN** (1982/83/93) | 24.03.1993 (CSSR-DBA 1980, Gesetz 1982) | stabil (Alt-CSSR-DBA, unverändert) | **Art. 23 „Beseitigung"** | **NWB Tschechien.md** (alle PDFs Scan, NWB-Pendant) | Nachfolge CSSR→CZ; „Beseitigung" statt „Vermeidung" (Wortlaut-Anker!) |
| **GB** Großbritann. | 16 | DBA-Gesetz 2010 = TEXT; 2015/2021 = TEXT | 30.03.2010 (Gesetz 2010) | ⚠ **Protokolle 17.03.2014 (BGBl 2015) + 12.01.2021 (BGBl 2021)** → VZ 2024–2026 voll-amendiert | Art. 23 | NWB Großbritannien.md (voll-konsolidiert) + PDF-Text | Reichste Kette (2010+2014+2021); Abfindungs-Zuordnungs-Verständ. 2011 (parallel FR-Abfindungs-Thema); Flugpersonal-Absprachen |
| **TR** Türkei | 7 | DBA-Gesetz 2012 = **BEL** (456×U+0007) | 19.09.2011 (Gesetz 2012; ersetzt Alt-1985) | stabil (2011-DBA), kein Split | Art. 22 | NWB Türkei.md (clean) ODER PDF via `tr '\a' ' '` | BEL-Route; **Renten-/Alterseinkünfte-Sonderregelung** (Verständ. 2014, große Diaspora); Alt-1985-DBA (1989-Gesetz Scan) abgelöst |

## Anker-Quellen-Empfehlung (Zusammenfassung)
- **Primär für ALLE 8: NWB-Volltext** (`dba_text_nwb/<Land>.md`) — konsolidierte i.d.F.,
  Methodenartikel voll-Länge anker-fähig, sauberer als BGBl-Scans.
- **PDF-Cross-Check des Methodenartikels**: PL / DK(2021) / GB — direkt aus Textlayer-PDF.
  ES / TR — via BEL-Route (`tr '\a' ' '` auf den 2012-Gesetz-PDF). IT / CZ — nur NWB
  (Kern-PDF ist Scan, NWB-Pendant genügt).
- **KEIN OCR für W1** (kein Staat in mineru_gaps).

## HYPOTHESE-Flags (amtliche Verifikation beim Freeze durch Instructor)
1. **DK**: Änderungsprotokoll 23.12.2021 wirksam → maßgebliche Fassung für VZ 2024–2026 ist
   die 2021-Fassung, NICHT die 1995-Basis. Prüfen, ob NWB-Konsolidat das bereits einarbeitet.
2. **GB**: Protokolle 2014 + 2021 → VZ 2024–2026 voll-amendiert. Methodenartikel-Wortlaut
   ggf. protokoll-geändert (Art. 23) — Anker am konsolidierten Stand.
3. **BE**: **Kern-DBA-PDF fehlt im dba_pdfs-Bestand** (nur 10 COVID-KonsVerh vorhanden). Für den
   Freeze entweder NWB-only oder Kern-PDF (1967 + Zusatzabk. 2002) nachbeschaffen. MELDUNG.
4. **MLI**: für alle 8 separat gegen die BMF-MLI-Anwendungsliste prüfen (NWB zeigt keinen
   MLI-Overlay; Annahme: bilateral, nicht MLI-synthetisiert — unbestätigt).
5. **Methodenartikel-Nummerierung** uneinheitlich (22/23/24) — je Staat einzeln pinnen; CZ nutzt
   „Beseitigung der Doppelbesteuerung" (nicht „Vermeidung") als Wortlaut-Anker.

## Repro
```
cd /home/julius/00_projects/168_TaxGraph
# Textlayer/BEL je PDF:  pdftotext -f 1 -l 2 <pdf> -   → Zeichenzahl + U+0007-Count (\x07)
# NWB-Signale: grep 'i.d.F.' / 'Protokoll' / 'Artikel 2[234]' / 'Grenzgänger' in dba_text_nwb/<Land>.md
# OCR-Lücken: mineru_gaps.json (11 Einträge, kein W1-Staat)
```
Read-only, kein Korpus-/Repo-Schreibzugriff außer diesem Report.

## Fazit
W1 ist quellen-seitig **freeze-bereit ohne OCR**: NWB-Volltexte tragen alle 8 Methodenartikel
anker-fähig; 2 BEL-PDFs (ES/TR) via `tr '\a' ' '` als Cross-Check. Zwei echte Fassungsketten-
Signale im/vor dem VZ-Fenster (DK-2021, GB-2014/2021). Eine Bestands-Lücke (BE-Kern-DBA-PDF).
MLI + exakte VZ-Wirksamkeit = Instructor-Freeze-Verifikation.
