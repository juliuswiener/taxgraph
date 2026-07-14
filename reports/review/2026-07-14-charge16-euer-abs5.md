# Charge 16 — § 4 Abs. 5 quantitative Grenzen + § 12 (Zuschnitt, Stufe A, 2026-07-14)

EÜR-Landkarte Zeilen 5–6. § 4 Abs. 5 nicht-abziehbare BA (quantitative Grenzen als
Regeln) + § 12 als Geltungsbedingungs-Paket. Quellen: `estg_p4_2026-07-14`,
`estg_p04_abs5_2026-07-09`, `estg_p12_2026-07-14`.

**Rahmen (Zitatanker `Die folgenden Betriebsausgaben dürfen den Gewinn nicht mindern`):**
§ 4 Abs. 5 S. 1 zählt BA auf, die den Gewinn NICHT mindern dürfen. Die Regeln liefern den
**abziehbaren** Rest-Betrag (das, was BA bleibt) — Andockung an die EÜR-Grundmechanik
(`p4_3_gewinn`, die BA summiert).

## Bestand-Check (WICHTIG: 6b/6c ist schon fertig)

**§ 4 Abs. 5 Nr. 6b (Arbeitszimmer, Jahrespauschale 1 260 €) + Nr. 6c (Tagespauschale
6 €/Tag, Deckel 1 260 €) sind BEREITS formalisiert** — Handregel
`rules/estg/p04_arbeitszimmer_homeoffice/arbeitszimmer_homeoffice.catala_en` +
Golden-Fälle (`ho_2024_jahrespauschale`, `ho_2024_tagespauschale_cap`, `ho_2024_ausschluss`,
alle grün). Charge 16 fügt hier **KEINE neue Regel** — nur Bestätigung + Landkarten-Eintrag.
**Fassungs-Konsistenz geprüft:** die Nr-6b/6c-Region ist in `estg_p4_2026-07-14` und
`estg_p04_abs5_2026-07-09` **byte-identisch** (diff leer) — keine Fassungs-Divergenz.

→ **Neue Pipeline-Regeln Charge 16: nur Nr. 1 (Geschenke) + Nr. 2 (Bewirtung).** 2 Regeln.

## Sondersatz-Sweep (Pflicht, verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | Nr. 1 S. 2 | **"Satz 1 gilt nicht, wenn … 50 Euro nicht übersteigen"** | gilt-nicht-Ausnahme = **FREIGRENZE** (nicht Freibetrag): ≤ 50 € voll abziehbar, > 50 € GANZ nicht abziehbar. |
| S2 | Nr. 2 | **"soweit sie 70 Prozent der Aufwendungen übersteigen"** | nur der Teil ÜBER 70 % der angemessenen Aufwendungen ist nicht abziehbar → abziehbar = 70 % des angemessenen Betrags. |
| S3 | Nr. 6c | **"Anstelle der"** (Jahrespauschale) | Tagespauschale tritt an die Stelle — bereits in der p04-Handregel abgebildet. |

## Regel 1 — § 4 Abs. 5 Nr. 1: Geschenke-Freigrenze 50 € (`p4_5_1_geschenke`)

**Wortlaut (Zitatanker `50 Euro nicht übersteigen`):** "Aufwendungen für Geschenke an
Personen, die nicht Arbeitnehmer des Steuerpflichtigen sind. Satz 1 gilt nicht, wenn die
Anschaffungs- oder Herstellungskosten der dem Empfänger im Wirtschaftsjahr zugewendeten
Gegenstände insgesamt 50 Euro nicht übersteigen."

- **⚠ FREIGRENZE (Klasse-2, wie § 23/§ 22 Nr. 3):** ≤ 50 € → voll abziehbar; > 50 € → der
  GANZE Betrag nicht abziehbar (NICHT nur der 50-€-übersteigende Teil). Kein `betrag − 50`.
- **Signatur** `GeschenkeFreigrenze`: `geschenke_je_empfaenger: money` (Summe pro Empfänger/WJ)
  → `abziehbar: money`.
- **Rechenkern:** `abziehbar = if geschenke_je_empfaenger <= 50 then geschenke_je_empfaenger
  else 0`. Encoding-Hinweis: **"nicht übersteigen" = ≤ 50,00** (bei genau 50,00: voll abziehbar).
- **Geltungsbedingungen:** `empfaenger_nicht_arbeitnehmer` (S. 1), `je_empfaenger_und_wj_summiert`
  (S. 2 "insgesamt", "dem Empfänger im Wirtschaftsjahr" — die Summierung je Empfänger/Jahr ist
  Integrations-/Anlage-Aufgabe, Input bereits summiert), `aufzeichnung_einzeln_p4abs7`
  (§ 4 Abs. 7 gesonderte Aufzeichnung).
- **Seeds (Grenzfälle Pflicht):** 50,00 → 50,00 (**≤-Boundary, Freigrenze**) · 50,01 → 0
  (Freigrenze reißt, GANZ nicht abziehbar) · 35,00 → 35,00 · 100,00 → 0 · 0 → 0.

## Regel 2 — § 4 Abs. 5 Nr. 2: Bewirtung 70 % (`p4_5_2_bewirtung`)

**Wortlaut (Zitatanker `70 Prozent der Aufwendungen übersteigen`):** "Aufwendungen für die
Bewirtung von Personen aus geschäftlichem Anlass, soweit sie 70 Prozent der Aufwendungen
übersteigen, die nach der allgemeinen Verkehrsauffassung als angemessen anzusehen und deren
Höhe und betriebliche Veranlassung nachgewiesen sind."

- **Rechenrichtung:** nicht abziehbar ist der Teil, der 70 % der ANGEMESSENEN Aufwendungen
  ÜBERsteigt → abziehbar = **70 % der angemessenen Aufwendungen** (30 % + jeder unangemessene
  Teil fallen weg).
- **Signatur** `Bewirtung70`: `angemessene_bewirtungsaufwendungen: money` → `abziehbar: money`.
- **Rechenkern:** `abziehbar = 0.70 · angemessene_bewirtungsaufwendungen`. **Rundungs-Richtung
  (Klasse 5):** money × decimal, Cent-Schnitt ZULETZT (0,70 als decimal, nicht vorrunden).
- **Geltungsbedingungen:** `geschaeftlicher_anlass` (S. 1), `angemessenheit_verkehrsauffassung`
  (die "angemessen"-Bewertung ist Verkehrsauffassung/Sachverhalt — Input ist der bereits als
  angemessen bewertete Betrag, NICHT gerechnet), `hoehe_und_veranlassung_nachgewiesen` (S. 1
  Nachweis), `aufzeichnung_einzeln_p4abs7`.
- **Seeds:** 1 000,00 → 700,00 · 100,00 → 70,00 · 0 → 0 · 333,33 → 233,33 (Cent-Schnitt-Wächter:
  0,70 × 333,33 = 233,331 → 233,33).

## § 12 — Privat/Betrieb-Abgrenzung (Geltungsbedingungs-Paket, keine Rechenregel)

§ 12 zählt nicht abziehbare Aufwendungen auf (Lebensführung, Repräsentation, Personensteuern,
Geldstrafen, bestimmte Zuwendungen). Keine quantitative Rechnung → **kein Pipeline-Rechenkern**,
sondern Geltungsbedingungs-Paket an `p4_3_gewinn` / EÜR-Integration:
`keine_lebensfuehrungskosten_p12_nr1`, `keine_repraesentation_gemischt_p12_nr1`,
`keine_personensteuern_p12_nr3`, `keine_geldstrafen_p12_nr4`. Anker aus `estg_p12_2026-07-14`.
Disposition wie andere Abgrenzungs-Pakete (§ 15/§ 18-Muster): deklariert, nicht gerechnet.

## Benannte Nachträge Charge 16

- § 4 Abs. 5 Nr. 1 „insgesamt … je Empfänger/Wirtschaftsjahr" — die Aggregation je Empfänger ist
  Anlage-/Integrations-Aufgabe (die Regel prüft die Freigrenze auf dem bereits summierten Betrag).
- § 4 Abs. 5 weitere Nummern (Nr. 3 Gästehäuser, Nr. 4 Jagd/Segeln, Nr. 5 Mehrverpflegung →
  eigene Sätze, Nr. 7 Unangemessenes, Nr. 8 Geldbußen, Nr. 8a Hinterziehungszinsen, Nr. 10
  Bestechung, Nr. 13 Zuschläge) — AN-fern/Spezial, Nicht-Gegenstand oder späterer Nachtrag.
- § 4 Abs. 5b (Gewerbesteuer nicht abziehbar) → gehört zu § 35-Andockung (Charge 19).

## Offene Punkte für deine Review

1. **Geschenke-Freigrenze-Richtung** bestätigen (≤ 50 abziehbar / > 50 ganz weg — Freigrenze,
   NICHT Freibetrag). Grenzfall-Seed 50,00 → 50,00 ist der Klasse-2-Wächter.
2. **Bewirtung-Rundung:** 0,70 als decimal, Cent-Schnitt zuletzt (Klasse 5). Seed 333,33 → 233,33
   als Wächter. Bestätigen.
3. **§ 12 als reines Bedingungs-Paket** (kein Rechenkern) — bestätigen; welche Nummern deklarieren?
4. **6b/6c**: Bestätigung, dass die p04-Handregel als Charge-16-Abdeckung zählt (Landkarte Zeile 5
   ✅ ohne neuen Lauf).
5. Cap-Wort Stufe B: 2 Regeln, 1-quellig (~$0,07/Stk) → Vorschlag `--cost-cap 0.25`.
