# Charge 15 — EÜR-Kern (Zuschnitt, Stufe A, 2026-07-14)

Erste Charge des EÜR-Programms (Phase 2). Zeilen 1–4 der EÜR-Landkarte
(`reports/review/2026-07-14-euer-landkarte.md`): § 4 Abs. 3 Grundmechanik, § 11
Zufluss/Abfluss, § 6 Abs. 2 GWG-Sofortabzug, § 6 Abs. 2a Sammelposten. **4 Regeln.**

Quellen eingefroren + committet (verify_sources OK, Anker grep-verifiziert):
`estg_p4_2026-07-14`, `estg_p6_2026-07-14`, `estg_p11_2026-07-14`.

Stufe A = Zuschnitt (Signaturen, Rechenkern, Geltungsbedingungen, Seeds inkl.
Grenzfälle, Encoding-Hinweise, Sondersatz-Sweep). **Kein Stufe-B-Lauf ohne dein
Cap-Wort.** Kalibrierung: 3 der 4 sind 1-quellig (~$0,07), Sammelposten multi-quellig
(~$0,15) → Chargen-Schätzung ~$0,35–0,45; Vorschlag `--cost-cap 0.50`.

## Sondersatz-Sweep (Pflicht, verbatim Freeze-Grep, beidseitig)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 6 Abs. 2a S. 1 | **"Abweichend von Absatz 2 Satz 1"** | Sammelposten ist Wahlrecht-ALTERNATIVE zum GWG-Sofortabzug; im Überlappungsband 250 < netto ≤ 800 exklusiv wählbar. |
| S2 | § 6 Abs. 2a S. 5 | **"einheitlich anzuwenden"** | Das Wahlrecht gilt pro Wirtschaftsjahr für ALLE WG einheitlich (Pool-Konvention, kein Cherry-Pick je WG). |
| S3 | § 11 Abs. 1 S. 2 | **"gelten als in diesem Kalenderjahr bezogen"** | gilt-als-Fiktion: regelmäßig wiederkehrende Zahlung "kurze Zeit" um den Jahreswechsel → wirtschaftlich zugehöriges Jahr statt Zahlungsjahr. |
| S4 | § 6 Abs. 2 S. 1 | **"vermindert um einen darin enthaltenen Vorsteuerbetrag (§ 9b Absatz 1)"** | Die 800/250/1000-Grenzen prüfen den NETTO-Wert (ohne abziehbare USt) — die Klasse-2-Boundary-Lektion. |

## Encoding-Leiter (Klasse-2, ab Stufe A eingeplant — Instructor-Auflage)

Alle drei GWG/Sammelposten-Schwellen sind **Netto-Grenzen** (§ 9b-Vorsteuer heraus). Per
[[klasse2-encoding-hinweis-leiter]]: **erst Encoding-Hinweis, Split nur Fallback.**
Signatur-Input = `anschaffungskosten_netto` (bereits um abziehbare Vorsteuer gemindert;
bei Nicht-Vorsteuerabzugsberechtigten ist netto = brutto — Geltungsbedingung). Richtung
der Grenzen wörtlich:
- "800 Euro **nicht übersteigen**" → netto **≤ 800,00** (bei genau 800,00: Sofortabzug JA).
- Sammelposten "250 Euro, aber nicht 1 000 Euro **übersteigen**" → **250,00 < netto ≤ 1 000,00**
  (bei genau 250,00: NICHT Sammelposten; bei genau 1 000,00: Sammelposten JA).

Grenzfall-Seeds an exakt 250,00 / 800,00 / 1 000,00 sind Pflicht (s. je Regel). Kein Split
in Teilregeln — eine Regel je Norm-Absatz, Boundary im Rechenkern.

## Regel 1 — § 4 Abs. 3: Grundmechanik EÜR (`p4_3_gewinn`)

**Wortlaut (Zitatanker `Überschuss der Betriebseinnahmen über die Betriebsausgaben`):**
"… können als Gewinn den Überschuss der Betriebseinnahmen über die Betriebsausgaben
ansetzen." (S. 2: durchlaufende Posten scheiden aus.)

- **Signatur** `EuerGewinn`: `betriebseinnahmen: money`, `betriebsausgaben: money` → `gewinn: money`.
- **Rechenkern:** `gewinn = betriebseinnahmen − betriebsausgaben` (kann negativ sein — Verlust,
  Passthrough; keine max(0)-Kappung).
- **Geltungsbedingungen:** `nicht_buchfuehrungspflichtig_und_keine_buecher` (S. 1 Voraussetzung),
  `durchlaufende_posten_ausgeschieden` (S. 2), `kein_uebergangsgewinn_wechsel` (Wechsel BV-Vergleich
  ↔ EÜR = eigener Nachtrag).
- **Seeds:** (100000/60000)→40000 · (50000/70000)→−20000 (Verlust) · (0/0)→0 · (30000/30000)→0.

## Regel 2 — § 11: Zufluss/Abfluss-Zuordnung (`p11_zufluss_abfluss`)

**Wortlaut (Zitatanker `gelten als in diesem Kalenderjahr bezogen`):** Abs. 1 S. 1 "Einnahmen
sind … in dem … zugeflossen"; S. 2 "Regelmäßig wiederkehrende Einnahmen, die … kurze Zeit vor
Beginn oder … nach Beendigung des Kalenderjahres … zugeflossen sind, gelten als in diesem
Kalenderjahr bezogen." Abs. 2 S. 1/S. 2 spiegeln das für Ausgaben.

- **Signatur** `ZuflussAbflussJahr`: `zahlungsjahr: integer`, `wirtschaftliches_jahr: integer`,
  `regelmaessig_wiederkehrend: boolean`, `innerhalb_kurze_zeit_fenster: boolean` →
  `zurechnungsjahr: integer`.
- **Rechenkern:** `zurechnungsjahr = if (regelmaessig_wiederkehrend and innerhalb_kurze_zeit_fenster)
  then wirtschaftliches_jahr else zahlungsjahr`.
- **⚠ Nicht-Formalisierung (Landkarte-Nachtrag):** "kurze Zeit" = **10-Tage-Regel = H 11 EStH /
  Rechtsprechung, KEIN Norm-Wortlaut.** Deshalb `innerhalb_kurze_zeit_fenster` als **bool-Input
  (Geltungsbedingung)**, NICHT aus Tagen gerechnet. Der Wortlaut (die Wenn-Struktur + gilt-als) wird
  formalisiert, die Schwellen-Ermittlung bleibt außerhalb (ggf. später verwaltung-Quelle H 11).
- **Geltungsbedingungen:** `kurze_zeit_ist_h11_nicht_wortlaut` (dokumentiert die Auslagerung),
  `keine_vorauszahlungs_verteilung_abs2s3` (Nutzungsüberlassung > 5 J gleichmäßig = eigener Nachtrag,
  s. u.), `keine_nutzungsueberlassungs_verteilung_abs1s3`.
- **Seeds:** (zahlungsjahr 2025, wj 2024, wiederkehrend true, fenster true)→2024 · (…, fenster false)→2025 ·
  (…, wiederkehrend false, fenster true)→2025 (beides nötig) · (2026,2026,false,false)→2026.
- **Benannter Nachtrag:** Abs. 2 S. 3 (Vorauszahlung Nutzungsüberlassung > 5 J → gleichmäßige
  Verteilung) — eigene Mechanik/State, nicht in dieser Zuordnungsregel.

## Regel 3 — § 6 Abs. 2: GWG-Sofortabzug 800 € netto (`p6_2_gwg_sofortabzug`)

**Wortlaut (Zitatanker `800 Euro nicht übersteigen`):** "… können im Wirtschaftsjahr der
Anschaffung … in voller Höhe als Betriebsausgaben abgezogen werden, wenn die Anschaffungs-
oder Herstellungskosten, vermindert um einen darin enthaltenen Vorsteuerbetrag (§ 9b Absatz 1),
… für das einzelne Wirtschaftsgut 800 Euro nicht übersteigen."

- **Signatur** `GwgSofortabzug`: `anschaffungskosten_netto: money` (bereits § 9b-bereinigt),
  `selbstaendig_nutzbar: boolean` → `sofortabzug: money`.
- **Rechenkern:** `sofortabzug = if (selbstaendig_nutzbar and anschaffungskosten_netto <= 800)
  then anschaffungskosten_netto else 0` (0 = kein GWG-Sofortabzug, dann AfA/Sammelposten anderswo).
  Encoding-Hinweis: **Netto-Grenze ≤ 800,00, "nicht übersteigen" = ≤**, Cent-genau.
- **Geltungsbedingungen:** `bewegliches_abnutzbares_anlagevermoegen` (S. 1),
  `selbststaendig_nutzbar_s2_s3` (S. 2/3), `verzeichnis_ab_250_gefuehrt` (S. 4: WG > 250 ins
  laufende Verzeichnis — Dokumentationspflicht, Geltungsbedingung), `wahlrecht_nicht_sammelposten`
  (Abgrenzung zu Abs. 2a, S1/S2 unten).
- **Seeds (Grenzfälle Pflicht):** netto 800,00 → 800,00 (**≤-Boundary**) · netto 800,01 → 0 ·
  netto 250,00 → 250,00 (Sofortabzug, aber Verzeichnis erst > 250) · netto 500,00 → 500,00 ·
  netto 900,00 → 0 · nicht selbständig nutzbar 400,00 → 0.

## Regel 4 — § 6 Abs. 2a: Sammelposten 250–1 000 €, 1/5 p. a. (`p6_2a_sammelposten`)

**Wortlaut (Zitatanker `250 Euro, aber nicht 1 000 Euro`):** "Abweichend von Absatz 2 Satz 1
kann … ein Sammelposten gebildet werden, wenn die Anschaffungs- … kosten, vermindert um einen
… Vorsteuerbetrag (§ 9b Absatz 1), … 250 Euro, aber nicht 1 000 Euro übersteigen." S. 2: "Der
Sammelposten ist im Wirtschaftsjahr der Bildung und den folgenden vier Wirtschaftsjahren mit
jeweils einem Fünftel gewinnmindernd aufzulösen."

- **Zwei Teil-Mechaniken** (eine Regel, zwei Outputs oder zwei Seeds-Gruppen):
  1. **Aufnahmefähigkeit** (S. 1): `sammelposten_faehig = 250 < netto <= 1000`. Encoding: **250,00
     exklusiv (>), 1 000,00 inklusiv (≤)**.
  2. **Auflösung** (S. 2): `jahresaufloesung = sammelposten_bestand / 5` (Bildungsjahr + 4 Folgejahre,
     je 1/5, linear; Abgang mindert NICHT, S. 3).
- **Signatur** `Sammelposten`: `sammelposten_bestand: money` (Summe der im Jahr gebildeten WG) →
  `jahresaufloesung: money`. (Aufnahmefähigkeits-Prüfung je WG als zweite Signatur oder als
  Geltungsbedingung `wg_netto_im_band_250_1000`.)
- **Rechenkern Auflösung:** `jahresaufloesung = sammelposten_bestand / 5` (decimal-Division, Cent-Schnitt).
- **Geltungsbedingungen:** `wahlrecht_einheitlich_pro_wj` (**S1 "Abweichend"** + **S5 "einheitlich"**
  — S1/S2-Sondersatz: pro WJ einheitlich Sofortabzug ODER Sammelposten, kein Cherry-Pick),
  `bewegliches_abnutzbares_anlagevermoegen`, `abgang_mindert_nicht_s3`.
- **Seeds (Grenzfälle Pflicht):** Aufnahme: netto 250,00 → nicht fähig (**>-Boundary**) · 250,01 → fähig ·
  1000,00 → fähig (**≤-Boundary**) · 1000,01 → nicht fähig. Auflösung: bestand 5000,00 → 1000,00/J ·
  bestand 1000,00 → 200,00/J · bestand 0 → 0.

## Band-Struktur (Abgrenzung der drei Regeln, zur Review)

| netto (§ 9b-bereinigt) | Behandlung | Regel |
|---|---|---|
| ≤ 250,00 | Sofortabzug (ohne Verzeichnis) | R3 (Abs. 2), auch Abs. 2a S. 4 |
| 250,00 < netto ≤ 800,00 | Sofortabzug (mit Verzeichnis) **ODER** Sammelposten — Wahlrecht, WJ-einheitlich | R3 **oder** R4 |
| 800,00 < netto ≤ 1 000,00 | nur Sammelposten | R4 |
| > 1 000,00 | lineare AfA § 7 | Charge 17 |

Das Wahlrecht im mittleren Band (S1 "Abweichend", S5 "einheitlich") ist der Kern-Sondersatz —
als Geltungsbedingung `wahlrecht_einheitlich_pro_wj` an beiden Regeln deklariert, nicht als
stille Annahme.

## Benannte Nachträge Charge 15 (dokumentiert, nicht stillschweigend)

- § 11 Abs. 2 S. 3 (Vorauszahlung Nutzungsüberlassung > 5 J, gleichmäßige Verteilung) — eigene
  Mehrjahres-Mechanik.
- § 11 "kurze Zeit"/10-Tage = H 11 (kein Wortlaut) — als bool-Geltungsbedingung ausgelagert.
- § 4 Abs. 3 S. 2 ff. (durchlaufende Posten Detailabgrenzung), Übergangsgewinn beim
  Gewinnermittlungs-Wechsel — Geltungsbedingung/Nachtrag.
- Sammelposten-Bildung als Jahres-Pool über konkrete WG-Liste (die Summierung `sammelposten_bestand`
  ist Integrations-/Anlage-EÜR-Aufgabe; die Regel liefert Fähigkeit + Auflösung).

## Offene Punkte für deine Review

1. **§ 11-Zuschnitt:** `zurechnungsjahr` als `integer`-Ausgabe (Jahres-Zuordnung) statt `money` —
   erste Nicht-money-Regel der Pipeline. Trägt die Signatur-/Gate-Maschinerie `integer`-Output, oder
   soll § 11 als reine Geltungsbedingungs-/bool-Regel geschnitten werden (wiederkehrend-Ausnahme als
   Prädikat)? **Meine Empfehlung:** integer-zurechnungsjahr, sauberster Wortlaut-Abbild — aber
   Pipeline-Kompatibilität ist deine Kenntnis.
2. **Sammelposten-Signatur:** Aufnahmefähigkeit + Auflösung in EINER Regel (zwei Outputs) oder zwei
   Regeln (`p6_2a_sammelposten_faehig` bool + `p6_2a_sammelposten_aufloesung` money)? Empfehlung: zwei
   Regeln, sauberer je ein Rechenkern + eigene Seeds.
3. **Netto-Input-Konvention:** `anschaffungskosten_netto` als Input (Vorsteuer schon heraus) — der
   § 9b-Abzug selbst ist nicht Gegenstand dieser Charge (Vorsteuer-Mechanik). Bestätigen.
4. Cap-Wort für Stufe B (Vorschlag `--cost-cap 0.50`).
