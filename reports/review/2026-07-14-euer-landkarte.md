# EÜR-Programm — Nenner-Definition + Landkarte (Instructor, 2026-07-14)

Julius-Wort 2026-07-14: "EÜR-Programm, DBA, BMF-Mini-Charge 14. alles machen." Dies ist der
Nenner für Phase 2 (EÜR). Muster wie AN-Landkarte: jede Zeile wird **formalisiert** oder
**begründeter Nicht-Gegenstand**. Prozess unverändert (Stufe A → Instructor-Review → Cap-Wort).

## Nenner-Definition

**Drin:** Gewinnermittlung nach **§ 4 Abs. 3 EStG (Einnahmenüberschussrechnung)** für
natürliche Personen (Einzelunternehmer, Freiberufler) — Anlage EÜR, Anlage S, Anlage G,
plus die ESt-seitige GewSt-Anrechnung (§ 35).

**Nicht-Gegenstand (begründet, von Anfang an):**
- Bilanzierung/Betriebsvermögensvergleich (§ 4 Abs. 1, § 5) — eigene Welt, nicht EÜR.
- Personengesellschaften/einheitliche Feststellung (§ 15 Abs. 1 Nr. 2, § 15a, § 180 AO) — Verfahren/Feststellung.
- GewStG-eigene Berechnung (Messbetrag, Hebesatz) — eigenes Gesetz; Messbetrag = Andockungs-Input in § 35.
- Land- und Forstwirtschaft (§ 13, § 13a) — eigener Komplex.
- Umsatzsteuer-Recht selbst (UStG) — nur USt-Zahlungen als BE/BA-Positionen (Zeile 4).

## Landkarte (14 Zeilen)

**Stand 2026-07-14: Chargen 15-18 (Zeilen 1–12) ✅ verified_bedingt** — p4_3_gewinn, p6_2_gwg_sofortabzug, p6_2a_sammelposten_zufuehrung/_aufloesung (Pipeline, $0,1797) + p11_zufluss_abfluss (Handregel). 4/14 Zeilen.

| # | Bereich | Norm | Freeze | Charge (Plan) |
|---|---|---|---|---|
| 1 | Grundmechanik BE − BA | § 4 Abs. 3 | estg_p4_2026-07-14 ✓ | 15 ✅ |
| 2 | Zufluss/Abfluss + wiederkehrende Zahlungen "kurze Zeit" | § 11 | estg_p11_2026-07-14 ✓ | 15 ✅ (Handregel) |
| 3 | GWG-Sofortabzug (800 € netto) | § 6 Abs. 2 | estg_p6_2026-07-14 ✓ | 15 ✅ |
| 4 | Sammelposten (250–1 000 €, 1/5 p. a.) | § 6 Abs. 2a | estg_p6_2026-07-14 ✓ | 15 ✅ |
| 5 | Nicht abziehbare BA quantitativ: Geschenke 50 €, Bewirtung 70 %, Arbeitszimmer/Jahrespauschale 1 260 €, Tagespauschale 6c | § 4 Abs. 5 Nr. 1/2/6b/6c | p4 + p04_abs5 ✓ | 16 ✅ |
| 6 | Privat/Betrieb-Abgrenzung | § 12 | estg_p12_2026-07-14 ✓ | 16 ✅ (Geltungsbed.) |
| 7 | Lineare AfA bewegliche WG | § 7 Abs. 1 | estg_p7_2026-07-14 ✓ | 17 ✅ |
| 8 | Kfz-Privatnutzung 1-%-Regelung (+ E-Fahrzeug-Bruchteile) | § 6 Abs. 1 Nr. 4 | estg_p6_2026-07-14 ✓ | 17 ✅ |
| 9 | Einlagen/Entnahmen-Bewertung | § 6 Abs. 1 Nr. 4/5 | estg_p6_2026-07-14 ✓ | 17 ✅ (teilw./Geltungsbed.) |
| 10 | Investitionsabzugsbetrag (50 %, Deckel) + Sonder-AfA | § 7g | estg_p7g_2026-07-14 ✓ | 18 ✅ |
| 11 | Einkunftsart Gewerbe (Anlage G) | § 15 Abs. 1/2 | estg_p15_2026-07-14 ✓ | 18 ✅ (Geltungsbed.) |
| 12 | Einkunftsart selbständige Arbeit (Anlage S) | § 18 | estg_p18_2026-07-14 ✓ | 18 ✅ (Geltungsbed.) |
| 13 | GewSt-Anrechnung (Vierfache des Messbetrags, Deckel) | § 35 | estg_p35_2026-07-14 ✓ | 19 |
| 14 | Betriebsveräußerung/-aufgabe (Freibetrag, Zusammenspiel § 34) | § 16 | Freeze nachziehen | 19 |

**Benannte Nachträge ab Start** (nicht stillschweigend): § 4 Abs. 4a Schuldzinsen/Überentnahmen
(komplex, eigener Schnitt); § 11 "kurze Zeit" = 10-Tage-Regel ist **H 11 EStH/Rechtsprechung,
nicht Norm-Wortlaut** → Geltungsbedingung + ggf. verwaltung-Quelle; § 17 (Anteile an
KapGes ≥ 1 %) = benannter Rand (Phase-2-Ende oder Nicht-Gegenstand-Entscheid).

## Chargen-Plan (Reihenfolge nach Häufigkeit × Formalisierbarkeit)

- **Charge 15 (Kern):** Zeilen 1–4 (§ 4 Abs. 3 Grundmechanik, § 11, GWG, Sammelposten) — ~4 Regeln.
  ⚠ GWG-Encoding ist die dokumentierte Klasse-2-Lektion aus dem MVP (Boundary-Kodierung,
  Encoding-hinweis-Leiter zuerst).
- **Charge 16:** Zeilen 5–6 (§ 4 Abs. 5 quantitative Grenzen als Regeln, § 12 als Geltungsbedingungs-Paket).
- **Charge 17:** Zeilen 7–9 (AfA linear, Kfz 1 %, Einlagen/Entnahmen).
- **Charge 18:** Zeilen 10–12 (§ 7g mit VZ-Params, § 15/§ 18 Geltungsbedingungs-Pakete).
- **Charge 19:** Zeilen 13–14 (§ 35 Andockung, § 16 + § 34-Verzahnung).
- Danach: Anlage-EÜR-Feldmapping (ELSTER, Kz-Extraktion wie Anlage N).

**Freezes:** 8 neue heute (p4, p6, p7g, p11, p12, p15, p18, p35, alle Anker verifiziert,
shas in .meta.yaml); nachzuziehen: § 7 Abs. 1 (voller § 7), § 16.

**Budget-Rahmen:** ~$1–2 über 5 Chargen (multi-quellige Kalibrierung ~$0,15/Regel).
Cap je Charge durch Instructor-Freigabe, --cost-cap Pflicht.
