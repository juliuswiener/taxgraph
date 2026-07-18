# Charge 29 — §35a Haushaltsnahe Leistungen + §10b Spendenabzug (Zuschnitt, Stufe A, 2026-07-18)

Aktiviert die heute **inerte** Deklarations-Bindung `bindung_sonder_agb_35a.yaml` (Pseudoregel-Scope,
regel_id `p35a_2_3_haushaltsnahe` referenziert, aber **keine Regel gebaut**). Produkt-Anlass: dev-1s
`haushalt_gesamt`-Kachel (§19-Basis + §35a + §10b) braucht die Rechenkerne live. **2 Regeln.**
Quellen: `estg_p35a_2026-07-09` (§ 35a Abs. 1–5), `estg_p10b_2026-07-13` (§ 10b Abs. 1 S. 1 Nr. 1).
Kein Stufe-B ohne Cap-Wort. Alle Anker VOLL-Länge via `_normalize` verifizieren (Skript-Ausgabe je Anker).

**DOKTRIN-BEGRÜNDUNG (warum Regel, nicht Haut-Accessor):** die Caps (20 % + min-Deckel 510/4000/1200;
20 % GdE) sind STEUERLOGIK. p35c-Präzedenz: „die Teilregeln bleiben rein und liefern ihre Ermäßigung
als Eingabe" an den tarif/festsetzung-scope (`input steuerermaessigungen` / `input sonderausgaben`).
Cap-Rechnung im Python-Accessor = §10c-Pfad-Bruch-Klasse (stille Divergenz je Pfad). → reine Catala-
Teilregeln, Caps als **params/<vz>** (nicht in-Regel-magic, nicht p35c-in-rule — §10c/§20/§32d-Muster).

## Gültigkeit (Fassungs-Check, Direktive 2026-07-15)

- **§ 35a:** Quelle-Header „geltende Fassung 2026". Höchstbeträge 510/4000/1200 € + Satz 20 % **seit
  2009 unverändert** (kein Änderungsgesetz 2024–2026, insb. Wachstumschancengesetz berührt § 35a-Caps
  NICHT). → params identisch für VZ 2024/2025/2026.
- **§ 10b Abs. 1:** 20 %-GdE-Quote (bzw. 4-‰-Alternative) **langfristig stabil**, keine VZ-Schwelle
  2024–2026. → params identisch alle drei VZ.

## Cap-/Sondersatz-Sweep (verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 35a Abs. 1 | **„um 20 Prozent, höchstens 510 Euro, der Aufwendungen"** (Minijob) | 20 % · min-Deckel 510; KEINE Unbar-Voraussetzung (S.3 nennt nur Abs. 2/3). |
| S2 | § 35a Abs. 2 S. 1 | **„um 20 Prozent, höchstens 4 000 Euro, der Aufwendungen"** (haushaltsnahe Dienstl.) | 20 % · min-Deckel 4000; Unbar (S.3) + nur-Arbeitskosten (S.2). |
| S3 | § 35a Abs. 3 S. 1 | **„um 20 Prozent der Aufwendungen … höchstens jedoch um 1 200 Euro"** (Handwerker) | 20 % · min-Deckel 1200; Unbar (S.3) + nur-Arbeitskosten (S.2). |
| S4 | § 35a Abs. 5 S. 3 | **„Rechnung erhalten … Zahlung auf das Konto des Erbringers"** | Unbar-Voraussetzung NUR Abs. 2/3 → bool-Guard; verletzt → Abs2/3-Ermäßigung 0. |
| S5 | § 35a Abs. 5 S. 2 | **„gilt nur für Arbeitskosten"** (Abs. 2 und 3) | Materialkosten raus → im Feld `*_arbeitskosten` gebunden (Sachverhalt). |
| S6 | § 35a Abs. 1/2/3 | **„die tarifliche Einkommensteuer, vermindert um die sonstigen Steuerermäßigungen"** | §35a-Betrag ≤ verfügbare ESt (nicht erstattungsfähig, Überhang verfällt) → **festsetzung-scope-Deckelung, NICHT Teilregel**. |
| S7 | § 10b Abs. 1 S. 1 | **„bis zu 1. 20 Prozent des Gesamtbetrags der Einkünfte oder 2. 4 Promille …"** | 20 % · GdE (Alt. 1, privat); 4-‰-Alt. (Betrieb) = Nachtrag. |

## Regel 1 — § 35a Abs. 1–3: Haushaltsnahe Steuerermäßigung (`p35a_haushaltsnahe`)

**⚠ regel_id-Korrektur:** die Bindung nennt `p35a_2_3_haushaltsnahe` — irreführend, weil die Regel
**auch Abs. 1 (Minijob)** trägt (so auch der Quelle-Header: „die Signatur … braucht auch Abs. 1 …
Abs. 5"). Empfehlung: Regel `p35a_haushaltsnahe` (Abs. 1–3), **Bindung-regel_id 1-Zeilen-Update** beim
Wiring. — *Offen 1: bestätigen oder alten id behalten für Bindungs-Stabilität.*

**Anker (voll-Länge Stufe B):** je Absatz der Höchstbetrags-Satz aus S1/S2/S3 oben.

- **Signatur** `HaushaltsnaheErmaessigung`: `minijob_aufwendungen: money` (Abs. 1),
  `dienstleistung_arbeitskosten: money` (Abs. 2), `handwerker_arbeitskosten: money` (Abs. 3),
  `rechnung_unbar: bool` (Abs. 5 S. 3, wirkt nur Abs. 2/3) → `steuerermaessigung: money`.
- **Rechenkern (drei UNABHÄNGIGE Töpfe, eigener Deckel je Topf, dann Summe):**
  - `e_minijob    = min(satz · minijob_aufwendungen; minijob_hoechstbetrag)`   *(Abs. 1, ohne Unbar)*
  - `e_dienstl    = if rechnung_unbar then min(satz · dienstleistung_arbeitskosten; dienstleistung_hoechstbetrag) else 0`
  - `e_handwerker = if rechnung_unbar then min(satz · handwerker_arbeitskosten; handwerker_hoechstbetrag) else 0`
  - `steuerermaessigung = e_minijob + e_dienstl + e_handwerker`
- **⚠ Klasse-2/Präzision:** `satz · money` (20 %), Cent-Schnitt am Topf-Ergebnis, Satz aus params
  NICHT vorrunden. Die drei Deckel sind additiv (510 + 4000 + 1200 gleichzeitig möglich) — KEIN
  Gesamt-Deckel (Wortlaut: getrennte Absätze).
- **Caps aus params/<vz>** `steuerermaessigung_haushaltsnah_p35a.yaml`: `satz: 0.20`,
  `minijob_hoechstbetrag: 510`, `dienstleistung_hoechstbetrag: 4000`, `handwerker_hoechstbetrag: 1200`
  (je `datenquelle`-Anker § 35a Abs. 1/2/3 + Stand).
- **Geltungsbedingungen:** `unbare_zahlung_abs5s3` (Rechnung + Konto-Zahlung, NUR Abs. 2/3),
  `nur_arbeitskosten_abs5s2` (Material raus, im Feldnamen gebunden), `eu_ewr_haushalt_abs4`
  (Leistung in EU/EWR-Haushalt, dokumentiert-bool), `keine_doppelberuecksichtigung_abs5s1` (nicht schon
  BA/WK/SA/agB; § 10 Abs. 1 Nr. 5 ausgeschlossen — Abgrenzung, dokumentiert), `caps_als_params`,
  `est_deckelung_im_festsetzung_scope` (S6: §35a ≤ verfügbare ESt = tarif-scope-Wiring, NICHT hier).
- **Seeds (Grenzfälle):**
  - (minijob 2800, 0, 0, unbar egal) → min(560;510) = **510** (Minijob-Deckel; Unbar irrelevant Abs.1)
  - (0, 0, handwerker 4500, unbar true) → min(900;1200) = **900** (Handwerker unter Deckel)
  - (0, 0, handwerker 10000, unbar true) → min(2000;1200) = **1200** (Handwerker-Deckel)
  - **(0, 0, handwerker 5000, unbar FALSE) → 0** (Abs. 5 S. 3 verletzt → keine Ermäßigung)
  - (0, dienstl 3000, 0, unbar true) → min(600;4000) = **600**
  - (minijob 2800, 0, handwerker 10000, unbar true) → 510 + 1200 = **1710** (zwei Töpfe, additive Deckel)

## Regel 2 — § 10b Abs. 1 S. 1 Nr. 1: Spendenabzug 20 % GdE (`p10b_spenden`)

**Anker (voll-Länge Stufe B, 155 Zeichen):** „Zuwendungen (Spenden und Mitgliedsbeiträge) zur Förderung
steuerbegünstigter Zwecke im Sinne der §§ 52 bis 54 der Abgabenordnung können insgesamt bis zu 1. 20
Prozent des Gesamtbetrags der Einkünfte … als Sonderausgaben abgezogen werden."

- **Signatur** `Spendenabzug`: `zuwendungen: money`, `gesamtbetrag_der_einkuenfte: money` →
  `spenden_abzug: money`.
- **Rechenkern:** `spenden_abzug = min(zuwendungen; quote_gesamtbetrag · gesamtbetrag_der_einkuenfte)`
  (Alt. 1, 20 % GdE).
- **⚠ GdE als INPUT (Naht zu est_einzel):** Basis ist der **Gesamtbetrag der Einkünfte** (VOR
  Sonderausgaben) — liegt in der est-Rechnung fest bevor § 10b als SA greift → keine Zirkularität.
  dev-1 extrahiert `gesamtbetrag_der_einkuenfte` aus `est_einzel(§19)`, NICHT `summe_der_einkuenfte`.
- **Caps aus params/<vz>** `spendenabzug_p10b.yaml`: `quote_gesamtbetrag: 0.20` (`datenquelle` § 10b
  Abs. 1 S. 1 Nr. 1 + Stand). 4-‰-Alternative (Betrieb) = Nachtrag.
- **Geltungsbedingungen:** `empfaenger_steuerbeguenstigt_abs1s2` (§§ 52–54 AO, jur. Person öff. Rechts /
  § 5 Abs. 1 Nr. 9 KStG — dokumentiert-bool, Sachverhalt), `keine_ausgeschlossenen_mitgliedsbeitraege_abs1s8`
  (Sport/Freizeit/Heimat raus — dokumentiert), `zuwendungsbestaetigung_abs4` (Spendenbescheinigung —
  Sachverhalt), `quote_als_param`.
- **Seeds:** (zuwendungen 15000, GdE 50000) → min(15000;10000) = **10000** (20 %-Deckel greift) ·
  (5000, 50000) → **5000** (unter Deckel) · (10000, 50000) → **10000** (Grenzfall gleich) ·
  (0, 50000) → **0**.

## Benannte Nachträge Charge 29

- **§ 35a Abs. 2 S. 2** Pflege-/Betreuungs-/Heimkosten (eigener Tatbestand, teilt den 4000-Topf) = Nachtrag.
- **§ 35a Abs. 4** EU/EWR-Haushalt-Detail + Heim-Ort (Abs. 2 S. 2 2. Hs.) = dokumentiert/Nachtrag.
- **§ 35a Abs. 5 S. 4** haushaltsbezogener Höchstbetrag (zwei Alleinstehende in einem Haushalt →
  Höchstbeträge nur einmal) = Nachtrag (Haushalts-Zusammensetzung).
- **§ 35a Abs. 5 S. 1** Doppelberücksichtigungs-Abgrenzung (BA/WK/SA/agB; § 10 Abs. 1 Nr. 5) = dokumentiert.
- **§ 10b Abs. 1 S. 1 Nr. 2** 4-‰-Umsatz/Lohn-Alternative (Betriebe) = Nachtrag.
- **§ 10b Abs. 1 S. 9** Spendenvortrag (Überhang in Folge-VZ, § 10d-analog) = Multi-VZ-Nachtrag.
- **§ 10b Abs. 1a** Vermögensstock-Stiftung (1 Mio / 2 Mio zusammen, 10-Jahr) = Nachtrag.
- **§ 10b Abs. 2** Parteispenden (3 300 / 6 600 € + § 34g-Ermäßigung) = eigener Tatbestand, Nachtrag.
- **§ 10b Abs. 3** Sachzuwendungen (gemeiner Wert / Buchwert), **Abs. 4** Vertrauensschutz/Haftung = Nachträge.
- **§35a-ESt-Deckelung (S6)** = festsetzung-scope-Wiring (§35a ≤ verfügbare ESt, Überhang verfällt) —
  dev-1-Auflage beim `haushalt_gesamt`-Wiring, NICHT Teilregel.

## Offene Punkte für Julius/meine Review

1. **regel_id** `p35a_haushaltsnahe` (Abs. 1–3 akkurat) vs. Bindungs-Altname `p35a_2_3_haushaltsnahe`
   behalten (1-Zeilen-Bindungs-Update beim Wiring). Empfehlung: akkurater Name + Bindung nachziehen.
2. **params/<vz>** für die Caps (§10c/§20-Muster) statt p35c-in-rule — bestätigen. Werte identisch
   VZ 2024/2025/2026 (Fassungs-Check oben: keine Schwelle).
3. **§35a-ESt-Deckelung** (S6, „vermindert um sonstige Steuerermäßigungen", Überhang verfällt) im
   festsetzung-scope beim Wiring — bestätigen, dass der scope steuerermaessigungen bei 0 floored
   (kein negativer Steuerbetrag, K2). Verify-Auflage ans Wiring.
4. **hh_rechnung_unbar als conditional-mandatory Kegel-Feld** (nur wenn dienstleistung/handwerker > 0;
   unbeantwortet → vorlaeufig; explizit false → Abs2/3-Ermäßigung 0 justiziert) — Scheibe-Auflage dev-1.
5. **Kachel-Scope** `haushalt_gesamt` = nur §35a + §10b? Die Bindung `bindung_sonder_agb_35a` bündelt
   auch agB § 33 (p33_1_2) + p10-SA/KiSt (p10_1_4/p10_1_7) — bleiben die eine SEPARATE spätere Kachel?
   (dev-1-Rückfrage offen.)
6. **Cap-Wort Stufe B:** 2 Regeln, enge auszüge (§ 35a Abs. 1–3+5, § 10b Abs. 1 S. 1 Nr. 1) →
   Vorschlag `--cost-cap 0.25`.
