# DBA-Methoden-Katalog — Dänemark (W4-Standard, Art. 24, ZWEI-FREEZE, Instructor-Review)

**Kein Kaskaden-Lauf, $0, LLM-frei.** DBA-Methoden-Katalog Königreich Dänemark (Paket 6, W1).
Geltend: **Abkommen Deutschland–Dänemark vom 22.11.1995** (BGBl. 1996 II S. 2565) **in der Fassung
des Änderungsprotokolls vom 01.10.2020** (Gesetz 26.05.2021, BGBl 2021 II S. 483; Berichtigung
24.11.2021, BGBl 2021 II Nr. 25; Inkrafttreten Bek. 25.01.2022, BGBl 2022 II). **Einfassig VZ
2024–2026** für den Methodenartikel (MLI-Negativ-Beleg unten). Methodenartikel **Art. 24 Abs. 1**
(deutsche Seite; Abs. 2 = DK-Seite, nicht Gegenstand). Andockung: **Freistellung → § 32b**,
**Anrechnung → § 34c_1**. Kein Rechenkern.

## ⚠ ZWEI-FREEZE-Ankerung (zentrale Besonderheit gegenüber GB)

Der Methodenartikel wird durch das Protokoll 2020 **teilgeändert** — Ankerung daher aus ZWEI Freezes:
- **`dba_dk_abkommen_1995`** (OCR-Grundtext, BGBl 1996 II S. 2565): **a-Freistellung, Schachtel 10 %,
  b-Anrechnung + Enum (aa/bb)** — im Wortlaut unverändert, ankern hier.
- **`dba_dk_protokoll_2020`**: **PROGRESSIONSVORBEHALT** ankert hier. Denn: **Art. 24 Abs. 1
  Buchst. a Satz 2 (Alt-Prog) wurde AUFGEHOBEN** und **Buchst. c NEU gefasst** (= der Progressions-
  vorbehalt). Der **Alt-Prog-Satz im Grundtext 1995 (a S. 2) ist NUR Fassungs-Vermerk, NIE aktiver
  Anker** — verifiziert per Negativtest (die aktive Prog-Formulierung „…Festsetzung **ihres**
  Steuersatzes…" existiert NUR im 2020er-Freeze, NICHT im 1995-Grundtext, wo die aufgehobene Fassung
  „…Festsetzung **des** Steuersatzes…" mit „behält **aber** das Recht" steht).

## ⚠ OCR-Disziplin (Grundtext 1995 = Bildscan-OCR)

Alle Grundtext-Anker aus zusammenhängenden deutschen Blöcken, **hyphen-frei gewählt** (Silbentrennungs-
Hyphenen wie „Vermögens-\nwerte", „Bun-\ndesrepublik", „deut-\nschen" gemieden — Anker enden vor dem
Umbruch). **Verbotene OCR-Fehlerwörter** (`verstehenden` [statt vorstehenden, Z. 1157], `ebenfalis`
[statt ebenfalls, Z. 1165]) sind NICHT in Ankern. Der **echte BGBl-Druckfehler `Anrechung`** (statt
Anrechnung, Z. 1172, bildbestätigt) IST amtlich und **bleibt im b-Intro-Anker** — er ist der amtliche
Druck. Verify-Skript `2026-07-17-dba-katalog-dk-anker-verify.py` (15 OK / 0 Fehler).

## Methodenartikel Art. 24 Abs. 1 (deutsche Methode)

- **a — FREISTELLUNG (Default) → § 32b** (Grundtext 1995). Anker: `Soweit nicht Buchstabe b anzuwenden
  ist, werden von der Bemessungsgrundlage der deutschen Steuer die Einkünfte aus Dänemark sowie die in
  Dänemark gelegenen` [Vermögenswerte ausgenommen, die … in Dänemark besteuert werden können]. →
  `dba_methode = freistellung`.
- **Schachteldividenden-Freistellung (a) — Beteiligungsschwelle 10 %** (wie GB). Anker: `von einer in
  Dänemark ansässigen Gesellschaft gezahlt werden, deren Kapital zu mindestens 10 vom Hundert
  unmittelbar der` [deutschen Gesellschaft gehört]. Nur an DE-Gesellschaft, ≥ 10 % Direktbeteiligung.
  Sonst → Anrechnung (b aa).
- **b — ANRECHNUNG (enumerierte Ausnahmen) → § 34c** (Grundtext 1995). Anker (Intro, mit amtl.
  Druckfehler): `der Vorschriften des deutschen Steuerrechts über die Anrechung ausländischer Steuern
  die dänische Steuer` [angerechnet]. Enum aa–bb:
  - aa) `aa) Dividenden, die nicht unter Buchstabe a fallen` (Streubesitz < 10 %);
  - bb) `Einkünfte, die in Dänemark nach den Artikeln 13 Absatz 1 Satz 2, 15 Absatz 4, 16, 17, 18
    Absatz 4 und 23 besteuert` [werden können] — **Artikel-Liste**: Art. 13 Abs. 1 S. 2
    (Veräußerungsgewinne), Art. 15 Abs. 4 (Geschäftsführer-/Vorstandsvergütung), Art. 16
    (Aufsichts-/Verwaltungsrat), Art. 17 (Künstler/Sportler), Art. 18 Abs. 4 (bestimmte Ruhegehälter/
    Sozialversicherung), Art. 23 (andere Einkünfte, soweit DK besteuern darf).
- **KEIN c-Aktivitätsklausel-Analog** (Unterschied zu ES/TR/GB!): Art. 24 hatte 1995 als Buchst. c die
  **Ausschüttungsbelastungs-Klausel** (körperschaftsteuerliches Anrechnungsverfahren) — obsolet seit
  dem Halbeinkünfte-/Teileinkünfteverfahren und durch das **Protokoll 2020 NEU gefasst** (Buchst. c =
  jetzt der Progressionsvorbehalt, s. u.). Es gibt **keinen** Aktivitätsvorbehalt (§ 8 AStG) im
  DK-Methodenartikel; der Aktivitätstest der anderen W1-Staaten hat hier kein Gegenstück.
- **PROGRESSIONSVORBEHALT (Buchst. c NEU, Protokoll 2020) → `p32b_progressionsvorbehalt`.** Anker
  (ZWEITER FREEZE): `Die Bundesrepublik Deutschland behält das Recht, die nach diesem Abkommen von der
  deutschen Steuer ausgenommenen Einkünfte und Vermögenswerte bei der Festsetzung ihres Steuersatzes
  zu berücksichtigen`. Der frühere Prog-Standort (a S. 2, 1995) ist aufgehoben — NIE als Anker verwenden.

## Katalog: Einkunftsart → Methode → Kanal → Quelle

| Einkunftsart (DK-Zählung) | Methode | `dba_methode` | Kanal | Freeze / Quelle | Anker |
|---|---|---|---|---|---|
| Immobilien (Art. 6) | Freistellung + Prog | freistellung | § 32b | 1995 a / 2020 c | a-freistellung / prog-c |
| Unternehmensgewinne (Art. 7) | Freistellung + Prog | freistellung | § 32b | 1995 a / 2020 c | a-freistellung |
| Schachteldividenden (≥ 10 %, Art. 10) | Freistellung | freistellung | § 32b | 1995 a | a-schachtel-10% |
| Streubesitzdividenden (< 10 %, Art. 10) | **Anrechnung** | anrechnung | § 34c | 1995 b aa | b-aa-dividenden |
| Zinsen (Art. 11) | Freistellung + Prog¹ | freistellung | § 32b | 1995 a / 2020 c | a-freistellung |
| Lizenzgebühren (Art. 12) | Freistellung + Prog¹ | freistellung | § 32b | 1995 a / 2020 c | a-freistellung |
| Veräußerungsgewinne (Art. 13 Abs. 1 S. 2) | **Anrechnung** | anrechnung | § 34c | 1995 b bb | b-bb-artikelliste |
| Geschäftsführer/Vorstand (Art. 15 Abs. 4) | **Anrechnung** | anrechnung | § 34c | 1995 b bb | b-bb-artikelliste |
| Aufsichts-/Verwaltungsrat (Art. 16) | **Anrechnung** | anrechnung | § 34c | 1995 b bb | b-bb-artikelliste |
| Künstler/Sportler (Art. 17) | **Anrechnung** | anrechnung | § 34c | 1995 b bb | b-bb-artikelliste |
| bestimmte Ruhegehälter (Art. 18 Abs. 4) | **Anrechnung** | anrechnung | § 34c | 1995 b bb | b-bb-artikelliste |
| andere Einkünfte, soweit DK besteuert (Art. 23) | **Anrechnung** | anrechnung | § 34c | 1995 b bb | b-bb-artikelliste |
| nichtselbst. Arbeit (Art. 15 Abs. 1–3) | Freistellung + Prog | freistellung | § 32b | 1995 a / 2020 c | a-freistellung |
| übrige DK-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | 1995 a / 2020 c | a-freistellung |

¹ Zinsen/Lizenzgebühren: Quellenstaat regelmäßig 0 %, DE Freistellung mit Progressionsvorbehalt.

## ⚠ MLI-Randnotiz (Auflage 4) — DK bleibt VZ 2024–2026 einfassig; ErbSt-FALLE beachtet

**Abwesenheits-Argument (TR-Muster):** In der bmf_stand-I.2-Positivliste „Abkommen, auf die das
BEPS-MLI-Anwendungsgesetz anzuwenden ist" (FR/GR/HR/MT/SK/ES/HU je ab 01.01.2025) ist **Dänemark NICHT
gelistet**. DK erscheint in der allgemeinen Einkommen-DBA-Fundstellen-Tabelle (`Dänemark 22.11.1995 …
01.01.1997`, BGBl 1996 II S. 2565). Negativtest `Dänemark …01.01.2025` FEHLT (Verify-Skript). → **VZ
2024–2026 einfassig** (Grundtext 1995 i. d. F. Protokoll 2020), kein MLI-Overlay.

**⚠ ErbSt-FALLE (Instructor-Warnung):** Im bmf_stand steht `Dänemark 22.11.1995` MEHRFACH — u. a. in
der **Erbschaftsteuer-Liste (I.3)** (das DE-DK-**ErbSt**-DBA trägt dasselbe Unterzeichnungsdatum
22.11.1995). Der Methoden-Katalog referenziert **ausschließlich die EINKOMMEN-DBA-Zeile** (I.1/
Fundstellen, → 01.01.1997), **NICHT** die ErbSt-Zeile. Verify prüft die Einkommen-DBA-Fundstelle
gezielt (22.11.1995 → 01.01.1997).

**BEPS lief bilateral (Protokoll 2020):** Der BEPS-Mindeststandard (Präambel/Missbrauch, DBA-
Modernisierung) wurde für DK **bilateral durch das Protokoll vom 01.10.2020** umgesetzt (das Protokoll
änderte u. a. Art. 24, hob Art. 1/14/31–40 auf, fasste Art. 3/4/23 neu). Kein multilaterales MLI-Overlay.

## ⚠ Fassungsketten-Vermerk (Auflage 3) — Ketten-Belege

Vier Quell-Stücke:
- `dba_dk_abkommen_1995` — Grundtext (BGBl 1996 II S. 2565), **a/Schachtel/b-Enum anker-tragend**.
- `dba_dk_protokoll_2020` (01.10.2020, Gesetz 26.05.2021 BGBl 2021 II S. 483) — **Art. 24 Abs. 1 a S. 2
  aufgehoben; Buchst. c neu gefasst (= Prog)**; **Prog anker-tragend**. Marker im Freeze:
  `Artikel 24 Absatz 1 Buchstabe a Satz 2 wird aufgehoben` + `Artikel 24 Absatz 1 Buchstabe c wird wie
  folgt gefasst` (Verify-belegt).
- `dba_dk_berichtigung_2021` (24.11.2021, BGBl 2021 II Nr. 25) — Berichtigung des Zustimmungsgesetzes
  zum Protokoll 01.10.2020 (Ketten-Beleg).
- `dba_dk_bekanntmachung_2022` (25.01.2022, BGBl 2022 II) — Inkrafttreten des Protokolls (Ketten-Beleg).
→ Für den Methodenartikel führt DK **einfassig** (1995 i. d. F. 2020) für VZ 2024–2026.

## Andockung + Nachträge

Andockung wie AT/US/CH/FR/LU/NL/ES/TR/GB: (`dba_staat = DK`, Einkunftsart) → `dba_methode` → `p32b` /
`p34c_1` (per-country `dba_staat = DK`). Geltungsbedingungs-Paket `dba_methode_dk_katalog`, kein Rechenkern.

**Cross-Rule-Referenz `kein_dba_mit_quellenstaat` (Paket 10c Block 3, 32c74e7):** DK ist ein
**DBA-Staat** ⇒ die Geltungsbedingung `kein_dba_mit_quellenstaat` (§ 34c Abs. 6 S. 1) **SPERRT den
unilateralen § 34c** für DK-Einkünfte — der Katalog regiert: Freistellung → § 32b, Anrechnung
(b aa/bb) → § 34c_1 als DBA-Anrechnungskanal (§ 34c Abs. 6 S. 2). Der Katalog materialisiert den
§ 34c-Abs.-6-DBA-Vorrang für DK.

**Nachträge / Nicht-Gegenstand:** Prog aus dem 2020er-Freeze (oben); Rückfall-/Subject-to-tax-Details
(Protokoll 2020 Art. 4/23-Änderungen); DK-seitige Methode (Abs. 2 a–g) = Nicht-Gegenstand; obsolete
Ausschüttungsbelastungs-c (1995) = historischer Fassungs-Vermerk. Zweitbelege (NWB/Kommentare) NUR
Gegenprobe, nie Primäranker (siehe [[dba-anker-nur-amtlich]]).

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-17-dba-katalog-dk-anker-verify.py` (`gates._normalize`): 5 Grundtext-
1995-Anker (a Freistellung, Schachtel 10 %, b-Intro mit amtl. Druckfehler `Anrechung`, aa Streubesitz,
bb Artikel-Liste) + 1 Protokoll-2020-Anker (Prog) **voll-Länge OK**. OCR-Disziplin: kein verbotenes
OCR-Wort (`verstehenden`/`ebenfalis`) im Anker, `Anrechung` amtlich beibehalten. **Zwei-Freeze-
Negativtest**: Prog-neu-c FEHLT im 1995-Grundtext (Katalog ankert Prog NICHT am aufgehobenen a S. 2).
Fassungskette: a-S.2-aufgehoben- + c-neu-gefasst-Marker im Protokoll-Freeze; Berichtigung 2021 +
Bekanntmachung 2022 als Ketten-Belege. MLI: DK NICHT in I.2-Liste (ErbSt-Falle I.3 gemieden, Einkommen-
DBA-Fundstelle gezielt geprüft). **Gesamt 15 OK / 0 Fehler.**
