# DBA-Fassungs-Recherche — Frankreich / Luxemburg / Niederlande (Paket 3, Stufe-A-artig, Instructor-Review)

**Read-only, $0, keine Downloads, keine Commits in `sources/`.** Zuarbeit für den amtlichen
Quellen-Freeze durch den Instructor (Beschaffung = Instructor-Arbeit). Analog Gültigkeits-Direktive:
je Land Fassung/Änderungsgesetze/Anwendbarkeit + BGBl-II-Fundstellen + Katalog-relevante Besonderheiten.
Der spätere Katalog ordnet — wie W4 (AT/US/CH) — je Einkunftsart-Artikel die **Methode**
(Freistellung → `p32b_progressionsvorbehalt`, Anrechnung → `p34c_1`/`p34c_2`) über das
`dba_methode`/`dba_staat`-Interface (Charge 20) zu; **kein neuer Rechenkern, Formalisierer-los**.

**Primärquellen:** BMF „Stand der DBA am 1.1.2025" (BMF 20.01.2025, IV B 2 - S 1301/01499/004/003,
amtliche Übersichtstabelle) + BMF „Stand am 1.1.2026" (BMF 07.01.2026, IV B 2 - S 1301/01499/005/004,
BStBl 2026 I S. 132) + recht.bund.de/BGBl + BMF „Staatenbezogene Informationen". BGBl-II-Fundstellen
aus der amtlichen BMF-Tabelle (Vertragsgesetz-Spalte) übernommen; unabhängig gegengeprüft.

---

## 1. FRANKREICH — DBA 21.07.1959 i.d.F. Zusatzabkommen 31.03.2015

### 1.1 Fassungshistorie (BMF-Tabelle, Vertragsgesetz-Spalte)

| Instrument | Datum | BGBl II (Vertragsges.) | Inkrafttr.-Bekanntm. | Anwendbar ab |
|---|---|---|---|---|
| Grundabkommen | 21.07.1959 | 1961 II S. 397 (Abkommenstext S. 398 ff.) | — | 01.01.1957 |
| Revisionsprotokoll | 09.06.1969 | 1970 II S. 717 | — | 01.01.1968 |
| Zusatzabkommen | 28.09.1989 | 1990 II S. 770 | — | 01.01.1990 |
| Zusatzabkommen | 20.12.2001 | 2002 II S. 2370 (Abk.text S. 2372) | 2003 II S. 542 | 01.01.2002 |
| **Zusatzabkommen** | **31.03.2015** | **2015 II S. 1332** (Vertragsges. 20.11.2015) | 2016 II S. 227 | **01.01.2016** |
| BEPS-MLI-Overlay | — | (MLI BGBl 2020 II; FR-Notifik. 2024 II Nr. 205) | — | **01.01.2025** |

### 1.2 VZ-Mapping
- **VZ 2024:** 1959 i.d.F. 31.03.2015 (konsolidierter Stand). Anwendungsregel-Wortlaut (Art. 6
  Zustimmungsgesetz 20.11.2015): „erstmals für den Veranlagungszeitraum anzuwenden, der dem Jahr des
  Inkrafttretens des Zusatzabkommens folgt" → Inkrafttreten 24.12.2015 → **anwendbar ab 01.01.2016**.
- **VZ 2025 / VZ 2026:** 1959 i.d.F. 2015 **+ BEPS-MLI-Overlay ab 01.01.2025** (FR im BMF-Schreiben
  1.1.2025 als MLI-anwendbar notifiziert — Katalog-Randnotiz, betrifft v.a. Missbrauchsklauseln/PPT,
  nicht die Methoden-Grundzuordnung).

### 1.3 Grenzgänger + Methode
- **Echte Grenzgängerregelung Art. 13 Abs. 5** (Grenzzone beidseits des Oberrheins): Arbeitslohn
  wird **im Ansässigkeitsstaat allein** besteuert (nicht am Arbeitsort). Grenzzone: FR-seitig Depts.
  Haut-Rhin/Bas-Rhin/Moselle, DE-seitig Gemeinden ≤ 30 km (FR-Wohner) bzw. ≤ 20 km (DE-Wohner) von der
  Grenze. Nichtrückkehr-Toleranz **45 Arbeitstage/Kalenderjahr** (bzw. 20 % der Arbeitstage bei
  unterjähriger Tätigkeit); Homeoffice im Wohnsitz-Grenzgebiet unschädlich. Grenzgebiets-Städteliste:
  **BMF 16.11.2021, BStBl I S. 2230**. Fiskalausgleich **Art. 13a** (Grenzgängerfiskalausgleich,
  **1,5 % der Bruttojahresvergütungen**, Zahlung Wohnsitz- → Tätigkeitsstaat) + **Art. 13c**
  (Rentenfiskalausgleich, 2015 neu — Sozialversicherungsrenten künftig im Ansässigkeitsstaat). Vorrang
  **Art. 14** (öff. Kassen) vor Art. 13 Abs. 5 (st. BFH-Rspr.).
- **Methodenartikel Art. 20 Abs. 1** (D als Ansässigkeitsstaat): Buchst. a **Freistellung m.
  Progressionsvorbehalt** (Default) → § 32b; Buchst. c **Anrechnung** für best. Dividenden, Zinsen und
  Arbeitnehmerüberlassung → § 34c; Buchst. d **Umschaltklausel** (Notifikation) → Anrechnung.
  **CAVEAT: DBA-FR nutzt eigene, nicht-OECD-konforme Artikelnummerierung** (Quellen widersprüchlich:
  Dividenden Art. 9 vs. Art. 10, Zinsen Art. 10 vs. Art. 11, Arbeitnehmerverleih Art. 13 Abs. 6). Die
  **exakten Art.-Nr. sind erst am gefreezten Text zu pinnen**, nicht aus Sekundärquellen zu übernehmen.

### 1.4 Amtliche Quellen (URLs für Freeze)
- Abkommen 1959/BMF: `bundesfinanzministerium.de/.../Laender_A_Z/Frankreich/1961-04-22-frankreich-Abkommen-DBA.html`
- Vertragsges. ZusAbk 2015: `gesetze-im-internet.de/dbazusabkg_fra_2015/DBAZusAbkG_FRA_2015.pdf`
- Grenzgängerfiskalausgleich BMF 30.03.2017; Grenzgebiets-Liste BMF 16.11.2021 (BStBl I S. 2230)
- **Konsolidierte amtliche Fassung: NEIN als ein Dokument** — Grundtext 1959 + 4 Zusatzabkommen;
  BMF stellt einen *synthetisierten* Lesetext bereit (nicht amtlich-verkündet). → **W4-Abweichung**
  (AT/US hatten je eine amtliche Neufassung).

---

## 2. LUXEMBURG — DBA 23.04.2012 i.d.F. Änderungsprotokoll 06.07.2023

### 2.1 Fassungshistorie

| Instrument | Datum | BGBl II (Vertragsges.) | Inkrafttr.-Bekanntm. | Anwendbar ab |
|---|---|---|---|---|
| Grundabkommen (ersetzt 1958, BGBl 1959 II S. 1270) | 23.04.2012 | 2012 II S. 1402 (Abk.text S. 1403) | 2014 II S. 728 (Inkrafttr. 30.09.2013) | 01.01.2014 |
| **Änderungsprotokoll** | **06.07.2023** | **2023 II Nr. 334** (Zustimmungsges. 08.12.2023) | 2024 II Nr. 147 (Bekanntm. 26.04.2024) | **01.01.2024** |

Ratifikationsurkunden ausgetauscht **29.12.2023**, damit noch 2023 in Kraft; Anwendung ab 01.01.2024
(Art. 14 Abs. 2 Protokoll: Quellensteuern/übrige/Bagatell je ab 01.01.2024).

### 2.2 VZ-Mapping
- **VZ 2024 / 2025 / 2026:** durchgehend **2012 i.d.F. 06.07.2023** (34-Tage-Bagatell aktiv über
  das gesamte Fenster; kein VZ-Split).

### 2.3 Grenzgänger/Bagatell + Methode
- **Keine echte Grenzgängerregelung** — Arbeitsortprinzip (Art. 14). **Bagatellregelung Art. 14
  Abs. 1a (34 Tage/Kalenderjahr)**: Besteuerung bleibt beim Tätigkeitsstaat LU, wenn die Tätigkeit
  an **nicht mehr als 34 Tagen** im Ansässigkeitsstaat/Drittstaat ausgeübt wird (Wortlaut „weniger als
  35 Arbeitstagen"; Homeoffice/Dienstreise). Vorher **19-Tage-Verständigungsvereinbarung 26.05.2011
  (BStBl I 2011 S. 576)**, seit 2023-Protokoll rechtssicher im DBA. Analoge Regel öff. Dienst.
  Konsultationsvereinbarung zur Auslegung: **BMF 11.01.2024, BStBl 2024 I S. 201** (34 Tage auch bei
  Teilzeit/unterjährig nicht gekürzt).
- **Methodenartikel Art. 22 Abs. 1** (D als Ansässigkeitsstaat): Buchst. a **Freistellung m.
  Progressionsvorbehalt** → § 32b; Schachteldividende Freistellung ab **≥10 %** Beteiligung; Buchst. b
  **Anrechnung** → § 34c; **Rückfall zur Anrechnung** bei Nicht-/Minderbesteuerung + Notifikationsklausel
  (subject-to-tax). Protokoll-Klarstellung: keine Anrechnung auf die **Gewerbesteuer**. (Art. 22 Buchst. a
  durch das 2023-Protokoll neu gefasst.)

### 2.4 Amtliche Quellen (URLs für Freeze)
- Grundabkommen-Gesetz BMF: `bundesfinanzministerium.de/.../Laender_A_Z/Luxemburg/2012-12-10-Luxemburg-Abkommen-DBA-Gesetz.pdf`
- Änderungsprotokoll 2023 (BGBl 2023 II Nr. 334): `recht.bund.de/bgbl/2/2023/334/VO.html`
- Inkrafttretens-Bekanntmachung BGBl 2024 II Nr. 147 (`recht.bund.de/bgbl/2/2024/147/VO.html`);
  Konsultationsvereinbarung BMF 11.01.2024 (BStBl 2024 I S. 201)
- **Konsolidierte amtliche DE-Fassung: NEIN** — keine im BGBl neu verkündete durchgängige Neufassung;
  Rechtslage = Zusammenschau Grundabkommen 2012 + Protokoll 2023. LU-Steuerverwaltung stellt einen
  „Koordinierten Text" bereit (Lesehilfe, nicht amtlich-DE). → Freeze = **base + Protokoll** (zwei
  Quell-Stücke, eine effektive VZ-Fassung 2024-26). Damit W4-artig wie FR, nicht wie AT/US-Neufassung.

---

## 3. NIEDERLANDE — DBA 12.04.2012; VZ-SPLIT (2021-Fassung ↔ 2025-Protokoll ab VZ 2026)

### 3.1 Fassungshistorie

| Instrument | Datum | BGBl II (Vertragsges./Umsetzung) | Inkrafttr.-Bekanntm. | Anwendbar ab |
|---|---|---|---|---|
| Grundabkommen (ersetzt 1959) | 12.04.2012 | 2012 II S. 1414 | (Inkrafttr. 01.12.2015) | 01.01.2016 |
| Erstes Änderungsprotokoll | 11.01.2016 | 2016 II S. 866 | 2016 II S. 1352 | 01.01.2017 |
| Zweites Änderungsprotokoll | 24.03.2021 | 2021 II S. 735 | 2022 II S. 467 | 01.01.2023 |
| **Drittes Änderungsprotokoll** | **14.04.2025** | **2025 II Nr. 270** (Umsetzungsges. 23.10.2025) | **2025 II Nr. 307** (Bekanntm. 19.12.2025) | **01.01.2026** |

Drittes Protokoll: Bundestag 16.10.2025, Bundesrat 17.10.2025, in Kraft **31.12.2025** (Art. VII Abs. 2:
letzter Tag des Folgemonats nach Ratifikationsaustausch), Anwendung ab 01.01.2026. **Konsolidierte
Neubekanntmachung des Abkommens 11.05.2026** (fasst alle Protokolle inkl. 14.04.2025 zusammen).

### 3.2 VZ-Mapping — **ZWEI Freeze-Fassungen nötig**
- **VZ 2024 / 2025:** 2012 i.d.F. **24.03.2021** (BGBl 2021 II S. 735). **Keine** Bagatellregelung.
- **VZ 2026:** 2012 i.d.F. **14.04.2025** (Neubekanntmachung 11.05.2026). **Neu: Bagatell Art. 14
  Abs. 1a (34 Tage)** + Art. 18 öff. Dienst analog; Anpassungen § 1a KStG / Investmentsteuer / REIT.
- → **Direkte Parallele zum CH-2025-Protokoll-Caveat**: NL braucht einen **VZ-2026-Overlay** wie CH.

### 3.3 Grenzgänger/Bagatell + Methode
- **Keine echte Grenzgängerregelung** — Arbeitsortprinzip (Art. 14). Bagatell **erst ab VZ 2026**
  (Art. 14 Abs. 1a, 34 Tage; Wortlaut „weniger als 35 Tage"). Ca. 45.000 DE→NL-Pendler.
- **Methodenartikel Art. 22 Abs. 1** (D als Ansässigkeitsstaat): Buchst. a **Freistellung m.
  Progressionsvorbehalt** (Buchst. d) → § 32b, **mit ausdrücklicher Subject-to-tax-Klausel**
  („die … **tatsächlich in den Niederlanden besteuert** werden"); Buchst. b **Anrechnung** → § 34c
  (u.a. Art. 10 Abs. 2/6 Dividenden, Art. 13 Abs. 2, Art. 16); **Aktivitätsvorbehalt** für
  Unternehmensgewinne/Dividenden. Stärkerer Rückfall als AT (BFH bestätigt Art. 22 Abs. 1 a/b/d).

### 3.4 Amtliche Quellen (URLs für Freeze)
- Grundabkommen BMF: `bundesfinanzministerium.de/.../Laender_A_Z/Niederlande/2012-12-10-Niederlande-Abkommen-DBA.html`
- Drittes Protokoll: Umsetzungsgesetz **BGBl 2025 II Nr. 270** (23.10.2025); Inkrafttretens-Bekanntm.
  **BGBl 2025 II Nr. 307** (19.12.2025); Neubekanntmachung 11.05.2026 (konsolidiert)
- **Konsolidierte amtliche Fassung: JA (zwei Stände)** — 2021-Fassung (VZ ≤ 2025) + Neubekanntmachung
  11.05.2026 (VZ ≥ 2026). Zwei Freeze-Files.

---

## 4. W4-Abweichungen (AT/US/CH → FR/LU/NL)

1. **VZ-Split innerhalb des Fensters (NL):** wie CH ein **VZ-2026-Overlay** — zwei Freeze-Fassungen
   (≤2025 vs. 2026). AT/US waren einfassig. LU/FR einfassig über das Fenster.
2. **Echte Grenzgängerregelung (nur FR):** Art. 13 Abs. 5 weist das Besteuerungsrecht dem
   **Ansässigkeitsstaat** zu (45-Tage-Toleranz). Katalog-Sonderfall: für den DE-ansässigen Grenzgänger
   entfällt die ausländische Besteuerung → weder § 32b noch § 34c auf diesen Arbeitslohn (reine
   DE-Besteuerung). AT/US/CH kennen das so nicht. **Nachtrag-Bedingung, kein Rechenkern.**
3. **Bagatell-Tage (LU ab VZ 2024, NL ab VZ 2026):** de-minimis Homeoffice/Dienstreise-Tage
   (34 Tage), hält die Zuordnung beim Tätigkeitsstaat. Als Geltungsbedingung dokumentierbar, keine
   eigene Rechnung.
4. **Subject-to-tax / Aktivitätsvorbehalt (NL stark, LU mittel):** Freistellung nur bei tatsächlicher
   Auslandsbesteuerung → Rückfall zur Anrechnung. Als Bedingung benannt (wie AT Art. 23 Abs. 1 c).
5. **Keine amtliche DE-Einzel-Neufassung bei FR UND LU:** FR = 1959 + 4 Zusatzabkommen (Freeze:
   Grundtext + ZusAbk-2015, + BMF-Synthese als Lesehilfe); LU = base 2012 + Protokoll 2023 (LU-
   „Koordinierter Text" als Lesehilfe). Nur **NL** hat eine amtliche konsolidierte Neubekanntmachung
   (11.05.2026, für VZ 2026). AT/US hatten je eine amtliche Neufassung — FR/LU weichen davon ab.

## 5. Rückfall-Kaskaden-Check
**Keine.** Wie W4 formalisiert der Katalog keine neue Rechenregel — reine staatsvertragliche
Methoden-Zuordnung (`dba_methode ∈ {freistellung, anrechnung}` je `dba_staat ∈ {FR, LU, NL}` ×
Einkunftsart), Rechenwirkung durch die bestehenden verified_bedingt-Kanäle § 32b / § 34c. Grenzgänger-
(FR), Bagatell- (LU/NL) und Subject-to-tax-/Aktivitätsvorbehalt-Klauseln werden als **Geltungsbedingungen
benannt, nicht gerechnet** (Nachtrag-Charakter, analog AT-Report Abschnitt „Nicht-Gegenstand").

## 6. Freeze-Empfehlung (Instructor)
- **FR:** `sources/dba/dba_fr_1959_idf_2015.txt` (Grundtext + ZusAbk 2015; BMF-Synthese). Einfassig für VZ 2024-26 (+ MLI-Randnotiz ab 2025).
- **LU:** `sources/dba/dba_lu_2012_idf_2023.txt` — assembliert aus base 2012 (BGBl 2012 II S. 1402) +
  Protokoll 2023 (BGBl 2023 II Nr. 334); keine amtliche DE-Neufassung. Einfassig VZ 2024-26.
- **NL:** **zwei** Files — `dba_nl_2012_idf_2021.txt` (VZ ≤ 2025) + `dba_nl_2012_idf_2025.txt`
  (Neubekanntmachung 11.05.2026, VZ ≥ 2026). Methodenartikel je Fassung Art. 22 gegenprüfen.
- Methodenartikel-Anker (Art. 20 FR / Art. 22 LU / Art. 22 NL) nach Freeze via `quellen._normalize`
  volllängen-verifizieren (zweispaltige BGBl-PDFs → pdftotext OHNE -layout).

## 7. Offene Punkte / Nachträge
- NL-Neubekanntmachung 11.05.2026: Volltext-Fundstelle (BGBl-Nr.) beim Freeze final ziehen — hier aus
  Sekundärquelle referenziert, primär noch zu bestätigen (**UNVERIFIZIERT bis BGBl-Direktabruf**).
- FR-BEPS-MLI ab VZ 2025: Auswirkung auf die Methoden-Zuordnung gering (Missbrauchsklauseln); als
  Randnotiz, kein eigener Katalog-Zweig.
- Schachteldividenden-Schwellen (LU ≥10 %, NL Aktivitätsvorbehalt) — Sonderbedingung, Nachtrag wie AT.
