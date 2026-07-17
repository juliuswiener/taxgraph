# DBA-Methoden-Katalog — Polen (W4-Standard, Art. 24, Instructor-Review) — LETZTER W1-Staat

**Kein Kaskaden-Lauf, $0, LLM-frei.** DBA-Methoden-Katalog Republik Polen (Paket 6, W1 — **achter und
letzter Staat**). Geltend: **Abkommen Deutschland–Polen vom 14.05.2003** (BGBl. 2004 II Nr. 29 S. 1304),
**einfassig VZ 2024–2026** (EIN Freeze, kein Änderungsprotokoll im Fenster; Korpus enthält nur die
Verständigungsvereinbarung 2015 als Zweitbeleg-Vermerk). Methodenartikel **Art. 24 Abs. 1** (deutsche
Seite; Abs. 2 = PL-Seite, nicht Gegenstand). Andockung: **Freistellung → § 32b**, **Anrechnung →
§ 34c_1**. Kein Rechenkern.

## ⚠ OCR-Disziplin (Grundtext = BGBl-Bildscan, Spalten-Crop)

Layer-Provenance: BGBl-Bildscan → Spalten-Crop-OCR (tesseract-deu 300 dpi, linke 52 % = deutsche
Spalte; DK-Route). **Kern-Anker S. 1318/1319 bildverifiziert.** Alle Anker aus der deutschen OCR-Spalte,
**Silbentrennungs-Hyphene exakt wie OCR** (`gele- genen`, `kön- nen`, `unmittel- bar`, `Wirtschafts-
zonen`, `Arti- kel`, `Außen- steuergesetzes`). **OCR-Artefakte gemieden**: Spalten-Bleed-Einzelzeichen
am Zeilenende (`a`, `b`, `C)`, `d`, `si`, `P`, `W`) + **`$`/`$&` statt Paragraph-Zeichen** in der
AStG-Passage (Bild zeigt korrektes `§`; Anker enden vor/beginnen nach dem `$`). Verify-Skript
`2026-07-17-dba-katalog-pl-anker-verify.py` (16 OK / 0 Fehler; Negativtest: die §-korrekte Fassung
`unter § 8 …` FEHLT, weil der Freeze das OCR-`$` trägt — mein Anker meidet die Stelle). Die 8
Instructor-Anker aus der Freeze-meta (`erwartete_anker`) selbst voll-Länge nachgeprüft + aa/c-AStG ergänzt.

## Methodenartikel Art. 24 Abs. 1 (deutsche Methode)

- **a — FREISTELLUNG (vorbehaltlich b) → § 32b.** Anker: `werden vorbehaltlich des Buchstabens b die
  Einkünfte aus der Republik Polen sowie die in der Republik Polen gele- genen Vermögenswerte
  ausgenommen, die nach diesem Abkommen in der Republik Polen besteuert werden kön- nen`. →
  `dba_methode = freistellung`.
- **Schachteldividenden-Freistellung (a) — Beteiligungsschwelle 10 %** (wie GB/DK). Anker: `deren
  Kapital zu mindestens 10 vom Hundert unmittel- bar der deutschen Gesellschaft gehört`. An DE-
  Gesellschaft (nicht PersGes), ≥ 10 % Direktbeteiligung, beim Ausschütter nicht abgezogen.
- **⚠ WIRTSCHAFTSZONEN-RÜCKAUSNAHME (a) — PL-SPEZIFIKUM (eigene Geltungsbedingung).** Anker:
  `Anspruch auf die Steuervergünstigung nach dem Gesetz vom 20. Oktober 1994 über die besonderen
  Wirtschafts- zonen in der Republik Polen hat`. Die Schachtel-**Freistellung entfällt** für
  Dividenden von einer in Polen ansässigen Gesellschaft, die die Steuervergünstigung nach dem
  polnischen **Sonderwirtschaftszonen-Gesetz vom 20.10.1994** in Anspruch nimmt (subject-to-tax-artige
  Rückausnahme gegen doppelte Nichtbesteuerung). → eigene Geltungsbedingung
  `keine_wirtschaftszonen_verguenstigung_1994` (Umschaltung Schachtel-Freistellung → Anrechnung b aa);
  Anker im Freeze, kein Rechenkern.
- **b — ANRECHNUNG (enumerierte Ausnahmen) → § 34c.** Anker (Intro): `Beachtung der Vorschriften des
  deutschen Steuerrechts über die Anrechnung ausländischer Steuern die polnische Steuer angerechnet`.
  Enum aa–bb:
  - aa) `aa) Dividenden, die nicht unter Buchstabe a Satz 2 fallen` (Streubesitz < 10 % **bzw.
    Wirtschaftszonen-Dividenden**, die nicht der Schachtel-Freistellung a S. 2 unterfallen);
  - bb) `Einkünfte, die nach Artikel 11 Absatz 2, Artikel 12 Absatz 2, Artikel 13 Absatz 2, Artikel 15
    Absatz 3, Arti- kel 16 Absatz 1 und Artikel 17 in der Republik Polen besteuert werden können` —
    **Artikel-Liste**: Art. 11 Abs. 2 (Zinsen, Quellensteuer), Art. 12 Abs. 2 (Lizenzgebühren,
    Quellensteuer), Art. 13 Abs. 2 (Veräußerungsgewinne), Art. 15 Abs. 3 (Bordpersonal Schiff/Luftfahrzeug),
    Art. 16 Abs. 1 (Aufsichts-/Verwaltungsrat), Art. 17 (Künstler/Sportler).
- **c — AKTIVITÄTSKLAUSEL (Umschaltung a→b) → Anrechnung.** Anker: `Statt der Bestimmungen des
  Buchstabens a sind die` [Bestimmungen des Buchstabens b … Artikel 7 und 10] + `Absatz 1 Nummern 1 bis
  6 des deutschen Außen- steuergesetzes fallenden Tätigkeiten` [+ § 8 Abs. 2 Beteiligungen]. Für
  **Art. 7 (Unternehmensgewinne) und Art. 10 (Dividenden)** gilt statt Freistellung die Anrechnung,
  wenn kein Nachweis aktiver Tätigkeit (**§ 8 Abs. 1 Nr. 1–6 UND § 8 Abs. 2 AStG** — breiter als GB,
  das nur § 8 Abs. 1 zitiert). → Geltungsbedingung. (Das `§` erscheint im OCR als `$`/`$&`; Anker
  meiden das Artefakt, zitieren `Nummern 1 bis 6 des deutschen Außensteuergesetzes`.)
- **d — PROGRESSIONSVORBEHALT.** Anker: `nach diesem Abkommen von der deutschen Besteuerung
  ausgenommenen Einkünfte und Vermögenswerte bei der Festsetzung des Steuersatzes für andere Einkünfte
  und Vermögenswerte zu berücksichtigen`. → materialisiert über `p32b_progressionsvorbehalt`.

## Katalog: Einkunftsart → Methode → Kanal → Quelle

| Einkunftsart (PL-Zählung) | Methode | `dba_methode` | Kanal | Quelle | Anker |
|---|---|---|---|---|---|
| Immobilien (Art. 6) | Freistellung + Prog | freistellung | § 32b | Art. 24 Abs. 1 a | a-freistellung |
| Unternehmensgewinne (Art. 7) — mit Aktivitätsnachweis | Freistellung + Prog | freistellung | § 32b | Art. 24 Abs. 1 a | a-freistellung |
| **Art. 7 / Art. 10 OHNE Aktivitätsnachweis (§ 8 Abs. 1 Nr. 1–6/Abs. 2 AStG)** | **Anrechnung** | anrechnung | § 34c | Art. 24 Abs. 1 c | c-aktivitaet |
| Schachteldividenden (≥ 10 %, Art. 10) | Freistellung | freistellung | § 32b | Art. 24 Abs. 1 a S. 2 | a-schachtel-10% |
| **Schachteldividenden mit Wirtschaftszonen-Vergünstigung (Gesetz 20.10.1994)** | **Anrechnung** | anrechnung | § 34c | Art. 24 Abs. 1 a S. 3 + b aa | a-wirtschaftszonen |
| Streubesitzdividenden (< 10 %, Art. 10) | **Anrechnung** | anrechnung | § 34c | b aa | b-aa-streubesitz |
| Zinsen (Art. 11 Abs. 2) | **Anrechnung** | anrechnung | § 34c | b bb | b-bb-artikelliste |
| Lizenzgebühren (Art. 12 Abs. 2) | **Anrechnung** | anrechnung | § 34c | b bb | b-bb-artikelliste |
| Veräußerungsgewinne (Art. 13 Abs. 2) | **Anrechnung** | anrechnung | § 34c | b bb | b-bb-artikelliste |
| Bordpersonal (Art. 15 Abs. 3) | **Anrechnung** | anrechnung | § 34c | b bb | b-bb-artikelliste |
| Aufsichts-/Verwaltungsrat (Art. 16 Abs. 1) | **Anrechnung** | anrechnung | § 34c | b bb | b-bb-artikelliste |
| Künstler/Sportler (Art. 17) | **Anrechnung** | anrechnung | § 34c | b bb | b-bb-artikelliste |
| nichtselbst. Arbeit (Art. 15 Abs. 1) | Freistellung + Prog | freistellung | § 32b | Art. 24 Abs. 1 a | a-freistellung |
| Ruhegehälter/Renten (Art. 18) | Freistellung + Prog¹ | freistellung | § 32b | Art. 24 Abs. 1 a | a-freistellung |
| übrige PL-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | Art. 24 Abs. 1 a | a-freistellung |

¹ Ruhegehälter (Art. 18) NICHT in der Anrechnungs-Enum b aa/bb → Default-Freistellung; öff. Kassen
(Art. 19) Sonderzuordnung = benannter Nachtrag.

## ⚠ MLI-Randnotiz (Auflage 4) — PL bleibt VZ 2024–2026 einfassig

**Abwesenheits-Argument (TR-Muster):** In der bmf_stand-I.2-Positivliste „Abkommen, auf die das
BEPS-MLI-Anwendungsgesetz anzuwenden ist" (FR/GR/HR/MT/SK/ES/HU je ab 01.01.2025) ist **Polen NICHT
gelistet**. PL erscheint nur in der allgemeinen DBA-Fundstellen-Tabelle (`Polen 14.05.2003 …`, BGBl
2004 II S. 1304). Negativtest `Polen …01.01.2025` FEHLT (Verify-Skript). → **VZ 2024–2026 einfassig am
Wortlaut 2003**, kein MLI-Overlay. Das Fehlen ist die Beweisform. (Der BEPS-Mindeststandard für PL wird
außerhalb dieses DBA-Fensters/über spätere Instrumente adressiert; im Freeze-Korpus liegt nur die
Verständigungsvereinbarung 2015 = Auslegungs-Zweitbeleg, KEIN Änderungsprotokoll → einfassig bestätigt.)

## Andockung + Nachträge

Andockung wie AT/US/CH/FR/LU/NL/ES/TR/GB/DK: (`dba_staat = PL`, Einkunftsart) → `dba_methode` → `p32b`
/ `p34c_1` (per-country `dba_staat = PL`). Geltungsbedingungs-Paket `dba_methode_pl_katalog`, kein Rechenkern.

**Cross-Rule-Referenz `kein_dba_mit_quellenstaat` (Paket 10c Block 3, 32c74e7):** PL ist ein
**DBA-Staat** ⇒ die Geltungsbedingung `kein_dba_mit_quellenstaat` (§ 34c Abs. 6 S. 1) **SPERRT den
unilateralen § 34c** für PL-Einkünfte — der Katalog regiert: Freistellung → § 32b, Anrechnung (b aa/bb,
c, Wirtschaftszonen) → § 34c_1 als DBA-Anrechnungskanal (§ 34c Abs. 6 S. 2). Der Katalog materialisiert
den § 34c-Abs.-6-DBA-Vorrang für PL.

**Nachträge / Nicht-Gegenstand:** Wirtschaftszonen-Rückausnahme (Gesetz 20.10.1994) als eigene
Geltungsbedingung (oben, PL-Spezifikum); Aktivitäts-Tatbestand § 8 Abs. 1 Nr. 1–6 + Abs. 2 AStG
(Sachverhalts-Vorfrage); Verständigungsvereinbarung 2015 (Auslegungs-Zweitbeleg, NIE Primäranker); öff.
Kassen (Art. 19); PL-seitige Methode (Abs. 2) = Nicht-Gegenstand. Zweitbelege (NWB/Kommentare) NUR
Gegenprobe (siehe [[dba-anker-nur-amtlich]]).

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-17-dba-katalog-pl-anker-verify.py` (`gates._normalize`): 10 PL-Anker
(Titel, a Freistellung, Schachtel 10 %, **Wirtschaftszonen-Rückausnahme**, b Intro, aa Streubesitz,
bb Artikel-Liste, c Umschalt + c-AStG, d Prog) **voll-Länge OK** aus der deutschen OCR-Spalte mit
exakten Silbentrennungs-Hyphenen. Negativtests: `25 %`-Schachtel (statt 10 %) FEHLT, erfundenes
Wirtschaftszonen-Gesetz `20.10.1999` FEHLT, **§-korrekte AStG-Fassung `unter § 8 …` FEHLT** (Freeze
trägt OCR-`$`, Anker meidet die Stelle). OCR-Disziplin: kein `$`/`$&` in Ankern. MLI: PL NICHT in
I.2-Liste (Fundstelle 14.05.2003 präsent). **Gesamt 16 OK / 0 Fehler.**

---
**Nach diesem Katalog + Instructor-Nachlauf ist die W1-Welle KOMPLETT (8 Staaten).** PL war der letzte.
Die drei Sonderfall-tragenden Abschlüsse: GB (Switch-over-Klausel e), DK (Zwei-Freeze + OCR + ErbSt-
Falle), PL (Wirtschaftszonen-Rückausnahme Gesetz 20.10.1994 + OCR-`$`-statt-`§`-Artefakt). Offiziellen
W1-Gesamtstand stellt der Instructor im Nachlauf fest.
