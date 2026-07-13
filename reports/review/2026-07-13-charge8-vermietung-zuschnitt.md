# Charge-8-Zuschnitt: Vermietung und Verpachtung (§ 21, Anlage V)

Dritte neue Einkunftsart, bisher GRÖSSTER Schnitt. Stufe A, $0. **NUR Stufe A** — Stufe B pausiert,
bis die Judge-Modell-Frage entschieden + die 10er-Queue abgebaut ist (Instructor-Grenze: Judge-Queue
nicht zweistellig). Report definiert Architektur + Freeze-Liste; Läufe folgen nach Judge-Entscheid.

## Freeze-Liste (an Instructor)
- § 21 EStG (ganzer Paragraph) — https://www.gesetze-im-internet.de/estg/__21.html
  --erwarte: „Einkünfte aus Vermietung und Verpachtung" / „Überlassung" / (Abs. 2:) „66 Prozent"
- § 7 Abs. 4 Gebäude-AfA: bereits im Bestands-Freeze `estg_p7_2026-07-11.txt` (Sätze 3 % / 2 % / 2,5 %
  verifiziert) — kein neuer Freeze.
- § 9 Schuldzinsen/WK: im Bestands-`estg_p9_2026-07-10.txt` — kein neuer Freeze.

## Architektur — Vorschlag: zwei Teilregeln + WK als Eingangsgrößen

Vermietung ist eine EINNAHMEN-minus-WERBUNGSKOSTEN-Rechnung. Die WK sind heterogen (AfA, Zinsen,
Erhaltung, Sonstiges). Sauberer Schnitt: die **Gebäude-AfA** als eigene Rechtssatz-Regel (§ 7 Abs. 4,
tabellierte Sätze), die restlichen WK als bereits ermittelte Eingangsgrößen in die **Einkünfte-Regel**
(die § 2-Integration aggregiert Zinsen/Erhaltung/Sonstiges — keine eigene Rechtsfolge, nur Summierung).

### (A) p7_4_gebaeude_afa — lineare Gebäude-AfA (§ 7 Abs. 4)

Wortlaut Abs. 4 Nr. 2: Wohngebäude, fertiggestellt a) nach 2022-12-31 → **3 %**, b) 1925–2022 → **2 %**,
c) vor 1925 → **2,5 %** der Anschaffungs-/Herstellungskosten. (Nr. 1 Betriebsgebäude 3 % = außerhalb
Vermietung/AN-nah.)

Rechenkern: `gebaeude_afa = (afa_satz_prozent / 100) × gebaeude_ak`. Signatur: `gebaeude_ak money,
afa_satz_prozent decimal -> gebaeude_afa money`. Andockung: `afa_satz_prozent` (3/2/2,5) kommt aus der
Fertigstellungs-Kategorie-Zuordnung (§ 2-Integration; 3 Kategorien, kein Kohorten-Kontinuum) als Input.
**/100-Encoding-hinweis PFLICHT** (%-Wert, wie § 24a/§ 22 — Leitlinie: %-Tabelle braucht /100-Pin).
Präzision: decimal, Cent-Schnitt zuletzt.
Seeds: ak 300.000 / 2 % → 6.000; ak 300.000 / 3 % → 9.000; ak 300.000 / 2,5 % → 7.500; ak 0 → 0.

### (B) p21_vermietung_einkuenfte — Einkünfte aus V+V (§ 21 i.V.m. § 2 Abs. 2 Nr. 2, § 9)

Rechenkern: `vermietung_einkuenfte = einnahmen − (gebaeude_afa + schuldzinsen + erhaltungsaufwand +
sonstige_werbungskosten)`. Signatur: `einnahmen, gebaeude_afa, schuldzinsen, erhaltungsaufwand,
sonstige_werbungskosten (money) -> vermietung_einkuenfte money`.
**WICHTIG — kann NEGATIV sein:** ein Vermietungsverlust ist mit anderen Einkunftsarten verrechenbar
(§ 2 Abs. 3, anders als KAP § 20 Abs. 6!) — also KEIN max(0)-Boden, echtes Vorzeichen durchreichen.
Das ist der Kreuz-Wächter zu KAP: dieselbe Signatur-Form, aber KEINE Topf-/Boden-Bedingung.
Seeds: einn. 12.000, afa 6.000, zinsen 3.000, erhaltung 1.000, sonst 0 → 12.000−10.000 = 2.000;
Verlust-Fall: einn. 8.000, afa 6.000, zinsen 4.000, rest 0 → 8.000−10.000 = **−2.000** (Verlust
durchgereicht, kein Boden); einn. 12.000, alle WK 0 → 12.000.

## Scope-Grenzen (dokumentiert)
- **Degressive Gebäude-AfA** (§ 7 Abs. 5a, 5 % degressiv für neue Wohngebäude 2023–2029) = Alternative
  zur linearen, eigener Zuschnitt (Wahlrecht).
- **Erhaltungsaufwand-Verteilung** (§ 82b EStDV, 2–5 Jahre) = mehrjähriger State, § 2-Integration/Backlog.
- **Verbilligte Vermietung** (§ 21 Abs. 2, < 66 % ortsüblich → WK-Kürzung anteilig) = eigene
  Bedingung/Teilregel (WK-Kürzungs-Mechanik).
- **Ferienwohnung/Liebhaberei-Prüfung** (Überschussprognose) = Ermessens-/Verfahrensfrage, außerhalb.
- **AfaA / Sonderabschreibungen** (§ 7 Abs. 4 S. 3, § 7b) = eigene Zuschnitte.
- WK-Aggregation (Zinsen/Erhaltung/Sonstiges als Summe) = § 2-Integration, keine eigene Rechtsfolge-
  Regel (wie WK-PB-102 bei Renten — Subtraktionsschritt, nicht Rechtssatz).

## Offene Architektur-Frage an Instructor
Genügen (A) Gebäude-AfA + (B) Einkünfte, oder soll die **verbilligte Vermietung (§ 21 Abs. 2,
66-%-Grenze)** als dritte Teilregel gleich in Charge 8 (WK-Kürzung ist eine echte Rechtsfolge mit
Schwelle — Boundary-Fall)? Empfehlung: Charge 8 = (A)+(B) Kern, § 21 Abs. 2 als benannter Nachtrag
(eigener Boundary-Zuschnitt), damit der Kern schlank bleibt.

## Nächste Schritte
1. Instructor-Review (2-Teilregel-Architektur, Verlust-durchreichen-vs-KAP-Boden, § 21 Abs. 2 rein/raus).
2. § 21-Freeze durch Instructor.
3. Nach Freeze + JUDGE-ENTSCHEID + Queue-Abbau: Signaturen + Seeds, Stufe B. (Stufe B pausiert bis dahin.)
4. Landkarte: Vermietung § 21 → andere Einkunftsarten 3/4.
