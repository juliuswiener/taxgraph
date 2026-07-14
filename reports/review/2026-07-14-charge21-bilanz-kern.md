# Charge 21 — Bilanz-Kern (Zuschnitt, Stufe A, 2026-07-14)

W2 Bilanzierung, Zeilen B1/B4/B5 (Landkarte `2026-07-14-bilanz-persges-landkarte.md`). Erste
Bilanz-Charge. Quellen: `estg_p4_2026-07-14` (§ 4 Abs. 1), `estg_p6_2026-07-14` (§ 6 Abs. 1 Nr. 1).
**3 Regeln.** Kein Stufe-B ohne Cap-Wort. Alle Zitatanker VOLL-Länge via `_normalize` verifiziert.

## Sondersatz-Sweep (verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 4 Abs. 1 S. 2/4 | „Entnahmen sind alle Wirtschaftsgüter …" / „Einlagen sind alle …" | Legaldefinition (Katalog) → Geltungsbedingung, keine Rechenmechanik im BV-Vergleich. |
| S2 | § 6 Abs. 1 Nr. 1 S. 2 | **„so KANN dieser angesetzt werden"** | Teilwertabschreibung = **Wahlrecht** (nicht Pflicht) → bool-Geltungsbedingung. |
| S3 | § 6 Abs. 1 Nr. 1 S. 4 | **„es sei denn, der Steuerpflichtige weist nach"** | Wertaufholung = Zuschreibungs-GEBOT mit **Beweislast-Umkehr** → Nachweis als bool-Geltungsbedingung. |
| S4 | § 6 Abs. 1 Nr. 1 S. 2 | „voraussichtlich dauernden Wertminderung" | Dauerhaftigkeit = Prognose/Sachverhalt → bool-Input, nicht gerechnet. |

## Regel 1 — § 4 Abs. 1 S. 1: Betriebsvermögensvergleich (`p4_1_bv_vergleich`)

**Wortlaut (Zitatanker `Gewinn ist der Unterschiedsbetrag zwischen dem Betriebsvermögen`, 246
Zeichen voll-verifiziert):** „Gewinn ist der Unterschiedsbetrag zwischen dem Betriebsvermögen am
Schluss des Wirtschaftsjahres und dem Betriebsvermögen am Schluss des vorangegangenen
Wirtschaftsjahres, vermehrt um den Wert der Entnahmen und vermindert um den Wert der Einlagen."

- **Signatur** `BvVergleich`: `betriebsvermoegen_ende: money`, `betriebsvermoegen_anfang: money`,
  `entnahmen: money`, `einlagen: money` → `gewinn: money`.
- **Rechenkern:** `gewinn = betriebsvermoegen_ende − betriebsvermoegen_anfang + entnahmen − einlagen`
  (kann negativ sein — Verlust, keine Kappung). Cent-genau.
- **Geltungsbedingungen:** `bv_werte_sachverhalt` (BV-Ende/-Anfang = Sachverhalts-Inputs; die
  Handelsbilanz-Ableitung/Maßgeblichkeit § 5 = Bedingung B2, Charge 23), `entnahmen_einlagen_p4abs1s2_definiert`
  (Legaldefinition S. 2/4, Inputs bereits bewertet), `bilanzierung_kein_p4abs3` (BV-Vergleich, nicht EÜR).
- **Seeds:** (BV-Ende 150000, BV-Anfang 100000, Entn 30000, Einl 10000) → 70000 · (90000, 100000, 0, 0) →
  −10000 (Verlust) · (100000, 100000, 0, 0) → 0 · (100000, 100000, 20000, 20000) → 0 (Entn = Einl).

## Regel 2 — § 6 Abs. 1 Nr. 1 S. 1–2: Bewertung Anlagevermögen (`p6_1_1_bewertung_av`)

**Wortlaut (Zitatanker `Ist der Teilwert auf Grund einer voraussichtlich dauernden Wertminderung
niedriger, so kann dieser angesetzt werden`, 115 Zeichen voll-verifiziert):** S. 1 — Ansatz mit
AK/HK vermindert um AfA (fortgeführte AK); S. 2 — bei voraussichtlich dauernder Wertminderung KANN
der niedrigere Teilwert angesetzt werden.

- **Wahlrecht (S2 „kann", C17-Teilwert-Präzedenz):** Teilwertabschreibung ist optional; nur wenn
  gewählt UND dauernde Wertminderung → niedrigerer Teilwert.
- **Signatur** `BewertungAv`: `fortgefuehrte_ak: money` (AK/HK − AfA, bereits fortgeführt),
  `teilwert: money` (Schätzwert-Input, C17-Präzedenz), `dauernde_wertminderung: bool`,
  `teilwertabschreibung_gewaehlt: bool` → `ansatz: money`.
- **Rechenkern:** `ansatz = if (dauernde_wertminderung and teilwertabschreibung_gewaehlt) then
  min(fortgefuehrte_ak; teilwert) else fortgefuehrte_ak`. `min` sichert: Teilwert nur wenn niedriger.
- **Geltungsbedingungen:** `teilwert_ist_schaetzwert_input` (S. 3-Definition, C17), `dauerhaftigkeit_prognose`
  (S. 2 „voraussichtlich dauernd" = Sachverhalt), `abnutzbares_av_afa_fortgefuehrt` (S. 1).
- **Seeds (Grenzfälle):** (fAK 10000, TW 8000, dauernd true, gewählt true) → 8000 (Abschreibung) ·
  (10000, 8000, true, false) → 10000 (Wahlrecht nicht ausgeübt) · (10000, 8000, false, true) → 10000
  (keine dauernde WM) · **(10000, 10000, true, true) → 10000 (TW = fAK exakt, kein Abschlag — Grenzfall).**

## Regel 3 — § 6 Abs. 1 Nr. 1 S. 4: Wertaufholung/Zuschreibungsgebot (`p6_1_1_wertaufholung`)

**Wortlaut (Zitatanker `sind in den folgenden Wirtschaftsjahren gemäß Satz 1 anzusetzen, es sei denn,
der Steuerpflichtige weist nach, dass ein niedrigerer Teilwert nach Satz 2 angesetzt werden kann`,
174 Zeichen voll-verifiziert):** WG, die bereits im Vorjahr zum AV gehörten, sind wieder nach S. 1
(fortgeführte AK) anzusetzen — ES SEI DENN, der Steuerpflichtige weist den niedrigeren Teilwert nach.

- **Zuschreibungs-GEBOT mit Beweislast-Umkehr (S4 „es sei denn … weist nach"):** Default =
  Wertaufholung auf fortgeführte AK (frühere Teilwertabschreibung rückgängig); nur bei NACHWEIS des
  weiter niedrigeren Teilwerts bleibt der Teilwert.
- **Signatur** `Wertaufholung`: `fortgefuehrte_ak: money`, `teilwert: money`,
  `niedrigerer_teilwert_nachgewiesen: bool` → `ansatz: money`.
- **Rechenkern:** `ansatz = if niedrigerer_teilwert_nachgewiesen then min(fortgefuehrte_ak; teilwert)
  else fortgefuehrte_ak` (Default = Zuschreibung auf fortgeführte AK).
- **Geltungsbedingungen:** `wg_bereits_vorjahr_av` (S. 4), `nachweis_beweislast_steuerpflichtiger`
  (S. 4 Beweislast-Umkehr), `teilwert_ist_schaetzwert_input`.
- **Seeds:** (fAK 10000, TW 8000, nachgewiesen true) → 8000 (Teilwert bleibt) · (10000, 8000, false) →
  10000 (Zuschreibung — GEBOT greift) · **(10000, 10000, true) → 10000 (TW = fAK, Grenzfall).**

## Abgrenzung B4 ↔ B5 (wichtig)

Beide rechnen `min`/`fortgefuehrte_ak`, aber INVERSE Default-Logik: **B4** (S2) = Default fortgeführte
AK, Teilwert nur bei **Wahlrecht** (kann); **B5** (S4) = Default fortgeführte AK (Zuschreibung), Teilwert
nur bei **Nachweis** (Beweislast). Der Sondersatz-Unterschied „kann" vs „es sei denn nachgewiesen" ist
der Kern — zwei Regeln, nicht eine.

## Benannte Nachträge Charge 21

- § 5 Maßgeblichkeit (Handelsbilanz → Steuerbilanz-Ableitung der BV-Werte) = Charge 23 (B2).
- § 4 Abs. 1 S. 3 ff. (Entstrickung/Verstrickung, Auslandsbezug) = Nachtrag.
- § 6 Abs. 1 Nr. 1 S. 1a (anschaffungsnahe HK, 15-%-Grenze) = eigene Regel (Charge 22 B-Bereich).
- § 6 Abs. 1 Nr. 2 (nicht-abnutzbares AV/Umlaufvermögen) = strukturgleich, Charge 22.

## Offene Punkte für deine Review

1. **B1** BV-Werte als reine Sachverhalts-Inputs (Maßgeblichkeit § 5 = separate Bedingung/Charge 23) —
   bestätigen.
2. **B4/B5 als ZWEI Regeln** (Wahlrecht „kann" vs Beweislast „es sei denn") — bestätigen; oder eine
   Regel mit kombinierter Bedingung? Empfehlung: zwei (verschiedene Sondersätze, verschiedene Default-Logik).
3. **Teilwert = Schätzwert-Input** (C17-Präzedenz), `dauernde_wertminderung`/`nachgewiesen` = bool —
   bestätigen.
4. Cap-Wort Stufe B: 3 Regeln (§ 4/§ 6, teils multi-Bedingung) → Vorschlag `--cost-cap 0.30`.
