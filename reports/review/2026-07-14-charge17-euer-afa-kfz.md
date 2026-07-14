# Charge 17 — AfA linear + Kfz-1 % + Einlagen/Entnahmen (Zuschnitt, Stufe A, 2026-07-14)

EÜR-Landkarte Zeilen 7–9. Der komplexeste EÜR-Schnitt (Kfz-1 % mit drei Bruchteil-Stufen).
Quellen: `estg_p7_2026-07-14` (AfA), `estg_p6_2026-07-14` (Kfz, Einlagen). **3 Regeln.**

Stufe A = Zuschnitt. Kein Stufe-B-Lauf ohne Cap-Wort. Alle Zitatanker grep-verifiziert.

## Sondersatz-Sweep (Pflicht, verbatim Freeze-Grep — Kfz-Nr-4 ist lang)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 7 Abs. 1 S. 1 | **"in gleichen Jahresbeträgen"** | lineare AfA = AK/HK ÷ Nutzungsdauer, gleichmäßig. |
| S2 | § 7 Abs. 1 (Monatsregel) | **"um jeweils ein Zwölftel für jeden vollen Monat, der dem Monat der Anschaffung … vorangeht"** | Zwölftelung im Anschaffungsjahr (pro rata temporis), Wortlaut-basiert (NICHT nur H). |
| S3 | § 6 Abs. 1 Nr. 4 S. 2 | **"1 Prozent des inländischen Listenpreises"** je Monat | Kfz-Privatnutzung Grundstufe (voll). |
| S4 | § 6 Abs. 1 Nr. 4 S. 2 | **"nur zur Hälfte anzusetzen"** / **"nur zu einem Viertel anzusetzen"** | E/Hybrid-Bruchteile ½ und ¼ — mehrere Bedingungs-Kataloge (Anschaffungsjahr, CO₂, Reichweite, BLP-Cap). |
| S5 | § 6 Abs. 1 Nr. 4 S. 3 | **"kann abweichend von Satz 2"** (Fahrtenbuch) | Fahrtenbuch-Alternative zur 1 %-Pauschale = Wahlrecht → Geltungsbedingung (Pauschale-Fall). |
| S6 | § 6 Abs. 1 Nr. 5 | **"höchstens mit den Anschaffungs- oder Herstellungskosten"** | Einlage-Deckel (Teilwert, gekappt auf AK/HK) unter Bedingungen. |

## Regel 1 — § 7 Abs. 1: lineare AfA + Zwölftelung (`p7_1_lineare_afa`)

**Wortlaut (Zitatanker `in gleichen Jahresbeträgen`):** "… ist jeweils für ein Jahr der Teil
der Anschaffungs- oder Herstellungskosten abzusetzen, der bei gleichmäßiger Verteilung dieser
Kosten auf die Gesamtdauer der Verwendung oder Nutzung auf ein Jahr entfällt (Absetzung für
Abnutzung in gleichen Jahresbeträgen)." Monatsregel: "… vermindert sich für dieses Jahr der
Absetzungsbetrag nach Satz 1 um jeweils ein Zwölftel für jeden vollen Monat, der dem Monat der
Anschaffung oder Herstellung vorangeht."

- **Signatur** `LineareAfa`: `anschaffungs_herstellungskosten: money`, `nutzungsdauer_jahre: integer`,
  `anzurechnende_monate: integer` (1–12; 12 in vollen Jahren, im Anschaffungsjahr 12 − vorangehende
  volle Monate) → `afa_betrag: money`.
- **Rechenkern:** `jahres_afa = anschaffungs_herstellungskosten / nutzungsdauer_jahre`;
  `afa_betrag = jahres_afa · (anzurechnende_monate / 12)`.
- **⚠ Klasse-5 (Präzisions-hinweis ab Start):** money ÷ integer UND money × decimal (monate/12) —
  Cent-Schnitt ZULETZT, nicht zwischenrunden. `nutzungsdauer_jahre`/`anzurechnende_monate` als
  integer (nicht decimal-vorgerundet).
- **Geltungsbedingungen:** `abnutzbares_wirtschaftsgut_ueber_ein_jahr` (S. 1 „mehr als einem
  Jahr"), `lineare_methode_keine_leistungs_afa` (Abs. 1 S. 6 Leistungs-AfA = eigener Nachtrag),
  `zwoelftelung_ganze_monate` (Monatsregel: nur VOLLE Monate vor Anschaffung zählen).
- **Seeds:** (10000/10 ND/12 Monate)→1000,00 · (10000/10/6)→500,00 (Zwölftelung Halbjahr) ·
  (10000/10/1)→83,33 (1 Monat, Cent-Schnitt 83,333→83,33) · (3000/3/12)→1000,00.

## Regel 2 — § 6 Abs. 1 Nr. 4 S. 2: Kfz-Privatnutzung 1 % (`p6_1_4_kfz_nutzungswert`)

**Wortlaut (Zitatanker `1 Prozent des inländischen Listenpreises`):** "Die private Nutzung eines
Kraftfahrzeugs, das zu mehr als 50 Prozent betrieblich genutzt wird, ist für jeden Kalendermonat
mit 1 Prozent des inländischen Listenpreises im Zeitpunkt der Erstzulassung zuzüglich der Kosten
für Sonderausstattung einschließlich Umsatzsteuer anzusetzen …" E/Hybrid: BLP „nur zur Hälfte"
(½) bzw. „nur zu einem Viertel" (¼) unter je eigenen Bedingungs-Katalogen.

### Muster-Entscheidung: Zähl-/Stufen-Input (Riester-Präzedenz), NICHT Kohorten-params

Die drei Stufen (voll / ½ / ¼) hängen NICHT nur am Anschaffungsjahr, sondern an einem
**Konjunktions-Katalog** je Stufe: Anschaffungs-Zeitfenster UND CO₂-Emission UND Reichweite UND
BLP-Cap (100 000 € für ¼). Eine Anschaffungsjahr × BLP-Kohorten-Tabelle (§ 24a/§ 22-Muster) kann
CO₂/Reichweite nicht abbilden — sie würde explodieren oder die Bedingungen verstecken.

→ **Empfehlung: `bruchteils_teiler: integer` (1 | 2 | 4) als Sachverhalts-Input** (die
Stufen-Zuordnung geschieht upstream aus dem Bedingungs-Katalog — wie `innerhalb_kurze_zeit_fenster`
§ 11 / `einmal_im_leben` § 34 Abs. 3). Der Bedingungs-Katalog je Stufe wird als **Geltungs-
bedingungen** deklariert (auditierbar, nicht gerechnet). Die Regel bleibt rein:
`nutzungswert = 0,01 · bruttolistenpreis / bruchteils_teiler`. **Kein Pseudo-Kohortentabellen-
Konstrukt** (das wäre die von dir gewarnte Falle).

- **Signatur** `KfzNutzungswert`: `bruttolistenpreis: money` (BLP inkl. USt + Sonderausstattung,
  auf volle 100 € abgerundet — H-Konvention, s. u.), `bruchteils_teiler: integer` → `nutzungswert_monat: money`.
- **Rechenkern:** `nutzungswert_monat = bruttolistenpreis · 0.01 / bruchteils_teiler`.
- **⚠ Klasse-5:** money × decimal (0,01) ÷ integer — Cent-Schnitt zuletzt. Monatswert (× 12 für
  Jahr = Integration).
- **Geltungsbedingungen:** `kfz_ueber_50_prozent_betrieblich` (S. 2 „mehr als 50 Prozent"),
  `keine_fahrtenbuchmethode` (S. 3 „abweichend von Satz 2" = Wahlrecht, Pauschale-Fall),
  `blp_auf_volle_100_abgerundet` (H-Konvention BLP-Rundung), `teiler_1_voll` / `teiler_2_halb_katalog`
  (CO₂/Reichweite/Anschaffungsjahr-Voraussetzungen S. 2 Nr. 2–5) / `teiler_4_viertel_katalog`
  (kein CO₂ + BLP ≤ 100 000, S. 2 Nr. 3) — die je-Stufe-Kataloge als Bedingungen, teiler ist ihr Ergebnis.
- **Seeds:** BLP 50000 teiler 1 → 500,00 (voll 1 %) · 50000 teiler 2 → 250,00 (½) · 50000 teiler 4 →
  125,00 (¼) · 40000 teiler 1 → 400,00 · BLP 33333 teiler 1 → 333,33 (Cent-Schnitt-Wächter, aber
  BLP ist eh auf 100 gerundet → 33300 → 333,00; nehme 33300/1 → 333,00).

## Regel 3 — § 6 Abs. 1 Nr. 5: Einlage-Deckel (`p6_1_5_einlage`)

**Wortlaut (Zitatanker `höchstens mit den Anschaffungs- oder Herstellungskosten`):** "Einlagen
sind mit dem Teilwert für den Zeitpunkt der Zuführung anzusetzen; sie sind jedoch höchstens mit
den Anschaffungs- oder Herstellungskosten anzusetzen, wenn das zugeführte Wirtschaftsgut a)
innerhalb der letzten drei Jahre vor dem Zeitpunkt der Zuführung aus dem Privatvermögen
angeschafft oder hergestellt worden ist …"

- **Teilwert = SCHÄTZWERT** (kein Rechenwortlaut) → **Input**, nicht gerechnet (deine Vorgabe).
  Die Regel rechnet nur die Deckelung.
- **Signatur** `EinlageDeckel`: `teilwert: money`, `anschaffungs_herstellungskosten: money`,
  `innerhalb_drei_jahre_aus_pv: boolean` → `einlagewert: money`.
- **Rechenkern:** `einlagewert = if innerhalb_drei_jahre_aus_pv then min(teilwert; ak_hk) else teilwert`.
- **Geltungsbedingungen:** `teilwert_ist_schaetzwert_input` (Teilwert vorab bewertet),
  `nur_fall_a_drei_jahre` (b/c = § 17-Anteile / § 20-WG = eigene Nachträge), `keine_afa_kuerzung_s2`
  (abnutzbares WG → AK um AfA gekürzt, S. 2 = eigener Nachtrag/Input).
- **Seeds:** (Teilwert 8000, AK 10000, innerhalb true) → 8000 (Teilwert < AK) · (12000, 10000, true)
  → 10000 (Deckel greift) · (12000, 10000, false) → 12000 (kein Deckel) · (10000, 10000, true) → 10000.

## Entnahme (§ 6 Abs. 1 Nr. 4 S. 1) — kein eigener Rechenkern

Entnahme = Teilwert (Schätzwert, Input). Keine quantitative Rechnung → Geltungsbedingung/Input
an der EÜR-Integration (`entnahme_mit_teilwert_angesetzt`). Die einzige Rechenmechanik in Nr. 4
ist die 1 %-Pauschale (S. 2) = Regel 2 oben.

## Benannte Nachträge Charge 17

- § 7 Abs. 1 S. 6 Leistungs-AfA (nach Maßgabe der Leistung) — eigener Rechenweg, Nachtrag.
- § 7 Abs. 2 degressive AfA (soweit wieder zulässig) — VZ-/Wahlrecht-Nachtrag.
- Kfz-Fahrtenbuchmethode (Nr. 4 S. 3, tatsächliche Aufwendungen) — Alternative zur Pauschale,
  eigener Rechenweg; hier Pauschale-Fall (Geltungsbedingung).
- BLP-Rundung auf volle 100 € — H-Konvention (kein Norm-Wortlaut), als Geltungsbedingung/Input.
- Einlage Nr. 5 b/c (§ 17-Anteile, § 20-WG) + S. 2 AfA-Kürzung — Nachträge.

## Offene Punkte für deine Review

1. **Kfz-Muster:** `bruchteils_teiler`-Input (1/2/4) + Bedingungs-Kataloge als Geltungsbedingungen —
   bestätigen, ODER willst du die Stufen-Zuordnung selbst als Regel (dann Multi-Kondition-Prädikat,
   money-Hardcode-Problem → Handregel)? Meine Empfehlung: Input-Teiler, Kataloge deklariert.
2. **§ 7 Zwölftelung:** `anzurechnende_monate`-Input (1–12) statt Datums-Rechnung — die „volle Monate
   vorangehen"-Zählung ist Sachverhalt/Kalender, nicht Norm-Rechnung. Bestätigen.
3. **BLP-Rundung** (volle 100 €) als Geltungsbedingung (H-Konvention) statt Rechenschritt — bestätigen.
4. **Einlage-Signatur** mit `innerhalb_drei_jahre_aus_pv`-bool + `min`-Deckel — bestätigen.
5. Cap-Wort Stufe B: 3 Regeln (§6/§7, teils 1-quellig) → Vorschlag `--cost-cap 0.30`.

## Stufe-B-Ergebnis + Korrekturen (2026-07-14)

**Instructor-Korrektur (§ 6 Abs. 1 Nr. 5 S. 2, Sweep-Nachtrag):** der Einlage-Deckel ist die
**fortgeführte AK** (AK − zwischenzeitliche AfA), nicht die rohe AK. Fundstelle S7-analog:
"sind die Anschaffungs- oder Herstellungskosten um Absetzungen für Abnutzung zu kürzen, die auf
den Zeitraum zwischen der Anschaffung … und der Einlage entfallen". Angewandt: Input
`fortgefuehrte_anschaffungskosten` + Geltungsbedingung `ak_um_zwischen_afa_gekuerzt_s2`.

**int-Input-Probe (read-only, $0):** integer-INPUTS sind bewiesen — Manifest-Konvention ist die
Kurzform `int`/`bool` (24× int, 18× bool im Bestand); `gates.py:_lit` rendert `int` via
`str(int(value))`. Kein decimal-Fallback nötig. Signaturen nutzen `int` (teiler, monate, ND).
Encoding: § 7 `AK/ND × Monate/12` (money/int × int/int, keine decimal-1/12-Falle);
Kfz `BLP/100/Teiler` (Prozent-/100-Encoding, money/int/int) — beide Cent-Schnitt zuletzt.

**Lauf:** $0,1446 (Cap 0,30), wall 116s. Alle 3 **verified_bedingt**, jedes deterministische Gate
grün (equivalence, clerk inkl. Zwölftelung 83,33 / Teiler 1|2|4 / Einlage-min-cap). KEINE
abweichung — alle Discoveries Whitelist (annahme/norm_teil → nicht_material / bedingung_neu).
Notiz: Judge-annahme "bruchteils_teiler = Monatsanzahl" ist Fehlinterpretation, aber harmlos
(clerk-Seeds Teiler 1/2/4 disproven), nicht_material triagiert.
