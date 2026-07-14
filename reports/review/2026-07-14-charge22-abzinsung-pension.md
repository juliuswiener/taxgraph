# Charge 22 — Abzinsung + Pensionsrückstellung (Zuschnitt, Stufe A, 2026-07-14)

W2 Bilanzierung B6/B7, der Klasse-5-lastigste Schnitt. Quellen: `estg_p6_2026-07-14` (§ 6 Abs. 1
Nr. 3a Buchst. e), `estg_p6a_2026-07-14` (§ 6a). **2 Regeln.** Kein Stufe-B ohne Cap-Wort. Alle
Anker VOLL-Länge via `_normalize` verifiziert (Skript-Ausgabe je Anker).

**ENGER ZUSCHNITT (Landkarten-Warnung):** § 6 Abs. 1 Nr. 3a hat SECHS Buchstaben-Kataloge a–f.
Charge 22 formalisiert NUR **Buchst. e (5,5-%-Abzinsung)** — der auszug enthält NUR Buchst. e,
kein Monster-auszug über a–f. Buchst. a–d/f (Wahrscheinlichkeits-/Sachleistungs-/Ansammlungs-
Grundsätze) = benannte Nachträge.

## Sondersatz-Sweep (verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 6 Abs. 1 Nr. 3a e | **„mit einem Zinssatz von 5,5 Prozent abzuzinsen"** | 5,5 % = Norm-Konstante; Abzinsung = Barwert. |
| S2 | § 6 Abs. 1 Nr. 3a e | **„ausgenommen … Laufzeit … weniger als zwölf Monate … verzinslich … Anzahlung oder Vorausleistung"** | DREI Ausnahmen von der Abzinsung → bool-Bedingung (keine Abzinsung). |
| S3 | § 6a Abs. 3 | **„Rechnungszinsfuß von 6 Prozent und die anerkannten Regeln der Versicherungsmathematik"** | 6 % = Norm-Konstante; Versicherungsmathematik = INPUT (Teilwert extern berechnet). |
| S4 | § 6a Abs. 4 | **„darf höchstens mit dem Teilwert … angesetzt werden"** | Höchstbetrags-Cap (min-Mechanik). |

## Regel 1 — § 6 Abs. 1 Nr. 3a Buchst. e: Rückstellungs-Abzinsung 5,5 % (`p6_1_3a_abzinsung`)

**Wortlaut (Zitatanker `mit einem Zinssatz von 5,5 Prozent abzuzinsen`, 338 Zeichen voll-verifiziert):**
„Rückstellungen für Verpflichtungen sind mit einem Zinssatz von 5,5 Prozent abzuzinsen; ausgenommen
von der Abzinsung sind Rückstellungen für Verpflichtungen, deren Laufzeit am Bilanzstichtag weniger
als zwölf Monate beträgt, und Rückstellungen für Verpflichtungen, die verzinslich sind oder auf einer
Anzahlung oder Vorausleistung beruhen."

- **Signatur** `AbzinsungRueckstellung`: `nominalwert: money`, `abzinsungsfaktor: decimal` (= 1/1,055^n
  aus der amtlichen Tabelle / Restlaufzeit — Catala hat keinen sauberen Potenz-Operator, daher der
  Faktor als Sachverhalts-/Tabellen-Input, wie Kfz-teiler C17), `abzinsung_ausgenommen: bool` →
  `abgezinster_wert: money`.
- **Rechenkern:** `abgezinster_wert = if abzinsung_ausgenommen then nominalwert else nominalwert ·
  abzinsungsfaktor`.
- **⚠ Klasse-5 (Präzisions-hinweis ab Start):** money × decimal (Barwertfaktor), Cent-Schnitt ZULETZT,
  Faktor NICHT vorrunden (Rundungs-Richtung gegen Wortlaut: der 5,5-%-Zins wirkt exakt, Rundung erst
  am Cent-Ergebnis).
- **Geltungsbedingungen:** `abzinsung_ausgenommen_katalog` (Buchst. e Ausnahmen: Laufzeit < 12 Monate
  ODER verzinslich ODER Anzahlung/Vorausleistung — als EIN bool zusammengefasst, Katalog dokumentiert),
  `faktor_ist_5komma5prozent_tabelle` (abzinsungsfaktor = 1/1,055^Restlaufzeit, 5,5 % Norm-Konstante
  im Faktor gebunden), `nur_buchst_e` (a–d/f = Nachträge).
- **Seeds (Grenzfälle):** (Nominal 10000, Faktor 0.90, ausgenommen false) → 9000,00 (abgezinst) ·
  **(10000, Faktor egal, ausgenommen true [< 12 Monate]) → 10000,00 (KEINE Abzinsung — Laufzeit-Grenzfall)**
  · (10000, 0.75, false) → 7500,00 · (10000, 1.0, false) → 10000,00 (Faktor 1 = keine Wirkung).
- **Laufzeit-Grenzfall (deine Auflage):** „weniger als zwölf Monate" = **< 12 M** → ausgenommen; **genau
  12 M = NICHT weniger** → Abzinsung greift. Der bool `abzinsung_ausgenommen` trägt diese Wortlaut-Grenze
  (Sachverhalt), Grenzfall-Seed für beide Seiten.

## Regel 2 — § 6a Abs. 4: Pensionsrückstellungs-Höchstbetrag (`p6a_pension_hoechstbetrag`)

**Wortlaut (Zitatanker `darf höchstens mit dem Teilwert der Pensionsverpflichtung angesetzt werden`,
100 Zeichen voll-verifiziert):** Abs. 4 S. 1 — höchstens Teilwert; Abs. 3 — Teilwert mit 6 %
Rechnungszinsfuß + Versicherungsmathematik.

- **Versicherungsmathematik = INPUT (deine Vorgabe):** der Teilwert der Pensionsverpflichtung (mit
  6 % Rechnungszins, versicherungsmathematisch berechnet) kommt als Input. Formalisiert wird NUR der
  **Höchstbetrags-Cap** (Abs. 4). Die 6 % sind Norm-Konstante, im Teilwert-Input gebunden (nicht Regel-
  Signatur — Konstanten-Doktrin: der Aufrufer darf den Zins nicht verstellen).
- **Signatur** `PensionHoechstbetrag`: `bilanzieller_ansatz: money` (angestrebte Rückstellung),
  `teilwert_pensionsverpflichtung: money` (6-%-Teilwert, versicherungsmathematischer Input) →
  `pensionsrueckstellung: money`.
- **Rechenkern:** `pensionsrueckstellung = min(bilanzieller_ansatz; teilwert_pensionsverpflichtung)`
  (Abs. 4 „höchstens Teilwert").
- **Geltungsbedingungen:** `teilwert_6prozent_versicherungsmathematik_input` (Abs. 3: 6 % +
  anerkannte Regeln, extern berechnet), `zufuehrungs_hoechstbetrag_abs4s2` (die Jahres-Zuführungs-
  Feinregeln Abs. 4 S. 2 ff. = Nachtrag; hier nur der Teilwert-Deckel S. 1), `pensionszusage_voraussetzungen_abs1`
  (Abs. 1 Rechtsanspruch/Schriftform = Bedingung).
- **Seeds:** (Ansatz 100000, Teilwert 90000) → 90000 (Deckel greift) · (80000, 90000) → 80000
  (unter Teilwert) · (90000, 90000) → 90000 (Grenzfall gleich).

## Benannte Nachträge Charge 22

- § 6 Abs. 1 Nr. 3a Buchst. a–d, f (Rückstellungs-Grundsätze: Wahrscheinlichkeit, Sachleistung,
  künftige Vorteile, zeitanteilige Ansammlung, Preisverhältnisse) = eigene Mechaniken, Nachträge.
- § 6 Abs. 1 Nr. 3 Verbindlichkeiten-Abzinsung = ABGESCHAFFT (Viertes Corona-Steuerhilfegesetz,
  ab 2023 keine 5,5-%-Abzinsung mehr für Verbindlichkeiten) → Nicht-Gegenstand, nur sinngemäß Nr. 2.
- Abzinsungsfaktor-Tabelle (1/1,055^n je Restlaufzeit) = amtliche Tabelle/Sachverhalt (kein Rechen-
  Wortlaut für die Potenz) — Faktor als Input; die Tabelle selbst als params-Nachtrag denkbar.
- § 6a Abs. 4 S. 2 ff. Zuführungs-/Nachhol-Feinregeln, Abs. 2 Alters-/Wartezeit-Voraussetzungen = Nachträge.

## Offene Punkte für deine Review

1. **Abzinsungsfaktor als decimal-INPUT** (Potenz 1/1,055^n extern/Tabelle, Catala kein Potenz-Op) vs.
   in-Regel-Iteration — Empfehlung: Input (sauber, Klasse-5-Risiko minimiert, Kfz-teiler-Präzedenz).
2. **Ausnahme-Katalog** (< 12 M / verzinslich / Anzahlung) als EIN bool `abzinsung_ausgenommen` — oder
   drei bools? Empfehlung: ein bool (die drei sind ODER-verknüpft, das Ergebnis „ausgenommen" zählt).
3. **§ 6a nur Höchstbetrags-Cap** (min mit 6-%-Teilwert-Input), Zuführungs-Feinregeln = Nachtrag —
   bestätigen.
4. Laufzeit-Grenzfall genau 12 Monate → Abzinsung (nicht ausgenommen) — bestätigen.
5. Cap-Wort Stufe B: 2 Regeln → Vorschlag `--cost-cap 0.25`.
