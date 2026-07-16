# DBA-Geltungsbedingungs-Katalog — Luxemburg (W4, Zwei-Stück-Overlay, Instructor-Review)

**Kein Kaskaden-Lauf, $0.** Vierter DBA-Methoden-Katalog (Paket 3). **OVERLAY, aber
einfassig**: die geltende Fassung für VZ 2024–2026 ist durchgängig „2012 i.d.F.
Änderungsprotokoll 06.07.2023" (Protokoll anwendbar ab 01.01.2024, **kein VZ-Split** wie
CH/NL). Zwei Quell-Stücke: unveränderte Artikel ankern in `dba_lu_abkommen_2012`
(BGBl 2012 II S. 1402), die vom **Protokoll 2023** (BGBl 2023 II Nr. 334) GEÄNDERTEN Artikel
ankern in `dba_lu_protokoll_2023`. Je Katalog-Zeile ist die Quelle ausgewiesen.

Methodenartikel **Art. 22 Abs. 1** (Deutschland als Ansässigkeitsstaat). Alle Anker VOLL-Länge
via `gates._normalize` verifiziert (Skript-Ausgabe: alle `OK`). Andockung wie AT/US/CH an das
`dba_methode`/`dba_staat`-Interface (Charge 20): **Freistellung → § 32b** (`p32b_progressions
vorbehalt`), **Anrechnung → § 34c** (`p34c_1`/`p34c_2`). **Kein neuer Rechenkern.**

## ⚠ Protokoll-2023-Änderungen (VZ 2024+), die den Katalog betreffen

- **Art. 22 Abs. 1 Buchst. a (Freistellung) NEU gefasst** (Protokoll Art. 10 Abs. 1): der neue
  Buchst. a nennt **nicht** mehr „tatsächlich besteuert" — der Subject-to-tax-Vorbehalt wandert
  in den **neuen Buchst. f** (ex-e). Anker daher aus dem **Protokoll**, nicht der Base.
- **Bagatell Art. 14 Abs. 1a NEU eingefügt** (Protokoll Art. 7 Abs. 2): 34-Tage-Regel (Wortlaut
  „weniger als 35 Arbeitstagen"). Hält die AN-Besteuerung beim Tätigkeitsstaat LU.
- **Buchst. e NEU** (subject-to-tax-Umschaltung bei DE-seitiger Freistellung/Quellensteuer),
  **bisheriger Buchst. e → Buchst. f neu gefasst** (Rückfall bei Nicht-/Notifikations-Besteuerung).

## Methodenartikel Art. 22 Abs. 1 (Grundstruktur)

- **Buchst. a — FREISTELLUNG (Default) → § 32b-Kanal** (Quelle: **Protokoll 2023**, neu gefasst).
  Anker: `Von der Bemessungsgrundlage der deutschen Steuer werden die Einkünfte aus Luxemburg sowie
  die in Luxemburg gelegenen Vermögenswerte ausgenommen, die nicht unter Buchstabe b fallen`. →
  `dba_methode = freistellung`, Andockung `p32b_progressionsvorbehalt` (Progressionsvorbehalt über
  Buchst. d, Base).
- **Buchst. b — ANRECHNUNG (Ausnahmeliste) → § 34c-Kanal** (Quelle: **Base 2012**, unverändert).
  Anker: `wird unter Beachtung der Vorschriften des deutschen Steuerrechts über die Anrechnung
  ausländischer Steuern die Steuer Luxemburgs angerechnet`. Betrifft aa) Dividenden (nicht Buchst. a),
  bb) Lizenzgebühren, cc) Art. 13 Abs. 2, dd) Art. 14 Abs. 3, ee) Aufsichts-/Verwaltungsrat,
  ff) Art. 16. → `dba_methode = anrechnung`, Andockung `p34c_1_anrechnung_hoechstbetrag`.
- **Buchst. c** — Aktivitätsvorbehalt (§ 8 Abs. 1 AStG) für Art. 7/10 (Base) — Bedingung.
- **Buchst. d** — Progressionsvorbehalt (Base): `… von der deutschen Steuer ausgenommenen Einkünfte
  und Vermögenswerte bei der Festsetzung ihres Steuersatzes zu berücksichtigen`.
- **Buchst. f** (ex-e, Protokoll) — Rückfall zur Anrechnung: `Ungeachtet der Bestimmungen des
  Buchstabens a wird die Doppelbesteuerung durch Steueranrechnung nach Buchstabe b vermieden, soweit`
  aa) LU tatsächlich nicht besteuert, bb) DE-Notifikation.

## Katalog: Einkunftsart-Artikel → Methode → Kanal → Quelle

| DBA-Artikel (Einkunftsart) | Methode (Art. 22) | `dba_methode` | Kanal | Quell-Stück | Anker |
|---|---|---|---|---|---|
| Art. 6 unbewegliches Vermögen | Freistellung + Prog | freistellung | § 32b | Protokoll (a) / Base (d) | Freistellungs-Anker a |
| Art. 7 Unternehmensgewinne (Betriebsstätte) | Freistellung + Prog¹ | freistellung | § 32b | Protokoll (a) | Freistellungs-Anker a |
| Art. 10 Dividenden (nicht Schachtel) | **Anrechnung** | anrechnung | § 34c | Base (b aa) | Anrechnungs-Anker b |
| Art. 10 Schachteldividende (≥ 10 %) | Freistellung | freistellung | § 32b | Protokoll (a S. 2) | „…mindestens 10 Prozent unmittelbar…" |
| Art. 12 Lizenzgebühren | **Anrechnung** | anrechnung | § 34c | Base (b bb) | Anrechnungs-Anker b |
| Art. 13 Abs. 2 Veräußerungsgewinne | **Anrechnung** | anrechnung | § 34c | Base (b cc) | Anrechnungs-Anker b |
| Art. 14 Abs. 3 (bestimmte n.s. Arbeit) | **Anrechnung** | anrechnung | § 34c | Base (b dd) | Anrechnungs-Anker b |
| Art. 14 Abs. 1/1a nichtselbst. Arbeit | Freistellung + Prog | freistellung | § 32b | Protokoll (a); Bagatell 14 Abs. 1a | Freistellungs-Anker a |
| Aufsichts-/Verwaltungsratsvergütungen | **Anrechnung** | anrechnung | § 34c | Base (b ee) | Anrechnungs-Anker b |
| Art. 16 Künstler/Sportler | **Anrechnung** | anrechnung | § 34c | Base (b ff) | Anrechnungs-Anker b |
| Art. 17 Ruhegehälter/Renten | Freistellung + Prog² | freistellung | § 32b | Protokoll (a) | Freistellungs-Anker a |
| übrige LU-Quellen-Einkünfte (Default) | Freistellung + Prog | freistellung | § 32b | Protokoll (a) | Freistellungs-Anker a |

¹ Vorbehaltlich Aktivitätsvorbehalt Buchst. c (§ 8 AStG) → sonst Anrechnung.
² Art. 17 Abs. 3 (Protokoll Art. 8): rückgeförderte DE-geförderte Ruhegehälter → nur DE — Sonderfall, Nachtrag.

## Andockung + Nachträge

Andockung wie AT/US/CH: (`dba_staat = LU`, Einkunftsart) → `dba_methode` → `p32b` / `p34c_1`
(per-country `dba_staat = LU`). Geltungsbedingungs-Paket `dba_methode_lu_katalog`, kein Rechenkern.

**Nachträge / Nicht-Gegenstand:** Bagatell Art. 14 Abs. 1a (34-Tage, Homeoffice/Dienstreise —
Konsultationsvereinbarung BMF 11.01.2024, BStBl 2024 I S. 201) als Geltungsbedingung, keine eigene
Rechnung; öff. Dienst Art. 18 Abs. 1 b/c analog 34-Tage (Protokoll Art. 9); Schachtel-Missbrauchs-
Ausnahmen (Buchst. a S. 2 neu: steuerbefreite/abziehbare Dividenden); Rückfall Buchst. e/f (subject-
to-tax + Notifikation) — benannt; Protokoll-Klarstellung „keine Anrechnung auf die Gewerbesteuer"
(Nachtrag). Keine echte Grenzgängerregelung (Arbeitsortprinzip Art. 14) — anders als FR.

## Voll-Länge-Anker-Verifikation

Skript `reports/review/2026-07-16-dba-katalog-frlunl-anker-verify.py` (importiert `gates._normalize`): LU-Anker über
`dba_lu_abkommen_2012` (b/c/d, unverändert) UND `dba_lu_protokoll_2023` (a/Schachtel/f/Bagatell,
neu gefasst) sämtlich `OK` voll-Länge. Alle deutschen Blöcke zusammenhängend (LU-Freeze nicht
zweisprachig-interleaved, keine Spaltengrenze).
