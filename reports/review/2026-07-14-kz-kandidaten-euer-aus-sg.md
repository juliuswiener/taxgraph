# Kz-Kandidaten-Tabelle — Anlage EÜR / AUS / S / G 2025 (W1, Instructor-Review)

Feldmapping der formalisierten Selbständigen-Regeln (Chargen 15–20) auf die amtlichen
ELSTER-Kennziffern. Deterministisch, kein LLM.

## Methodik je Datenart (WICHTIG — dokumentierte Abweichungen)

| Datenart | Primärquelle | Zweitbeleg | Sektions-Pfad |
|---|---|---|---|
| **Anlage EÜR (E77)** | Vordruck-**Zeilenordnung** `sources/bfinv/euer_2025.txt` | E77-2025.xsd (Kz→Label) | **entfällt** — E77 ist EIGENE Datenart, rohe XSD ohne gerendertes HTML; `_ANCHOR`-Sektions-Regex = **0 Treffer** (Negativ-Beleg). E77-complexTypes sind Datentyp-Namen (`DezimalzahlNichtNeg…`), KEINE Semantik. Einzelformular → keine cross-Anlage-Label-Kollision → Zeilenordnung reicht (Instructor-Ruling b). |
| **Anlage AUS** | Vordruck `sources/bfinv/aus_2025.txt` | E10-2025.html (AUS-Kz liegen in der ESt-Datenart E10) | XSD-Sektion aus E10-HTML |
| **Anlage S / G** | **KEIN Papier-Vordruck 2025** (E-Übermittlungspflicht, aus FMS entfernt) → **ERiC-XSD PRIMÄR** (dokumentierte Ausnahme von Vordruck-primär) | — | E10-HTML-Sektion |

⚠ **Warnung für künftige Fälle:** die Zeilen-primär-Abkürzung (E77) gilt NUR für Einzel-Datenarten
ohne Sektions-Anker. Ein E10-artiger Multi-Anlage-Fall MUSS den XSD-Sektionspfad nutzen (Label allein
kollidiert cross-Anlage). Nicht stillschweigend übernehmen.

**Identifier:** primärer Submission-Identifier = XSD-E-Nummer (E77: `E6xxxxxx`; E10: `E0xxxxxx`).
Die 3-stellige Form-Kennzahl aus dem Vordruck ist pdftotext-spaltenweise unsicher → Orientierung.

## Anlage EÜR — Kandidaten (Regeln Chargen 15–19)

| Regel (Output/Input) | Vordruck-Zeile (Konzept) | Form-Kz | E77-E-Nr | wörtliches E77-Label | Konfidenz |
|---|---|---|---|---|---|
| p4_3_gewinn: betriebseinnahmen | Z. 23 „Summe Betriebseinnahmen (Übertrag in Zeile 76)" | 159 | *Betragsfeld, Zeilen-Übertrag* | Summe Betriebseinnahmen | **STARK** |
| p4_3_gewinn: betriebsausgaben | Z. 77 „abzüglich Summe der Betriebsausgaben (Übertrag aus Zeile 75)" | — | *Betragsfeld* | Summe der Betriebsausgaben | **STARK** |
| p4_3_gewinn: gewinn | Z. 76–77 „Summe BE − Summe BA" → Z. 91 Gewinn | — | E6006801 / E6007002 | Korrigierter / Steuerpflichtiger Gewinn/Verlust | **STARK** |
| p6_2_gwg_sofortabzug: sofortabzug | Z. 36 „Aufwendungen für geringwertige Wirtschaftsgüter nach § 6 Abs. 2 EStG" | 132 | E6002301 | Aufwendungen für geringwertige Wirtschaftsgüter nach § 6 Abs. 2 EStG | **STARK** |
| p6_2a_sammelposten_aufloesung: jahresaufloesung | Z. „Auflösung Sammelposten nach § 6 Abs. 2a EStG" | — | E6003302 | Auflösung Sammelposten nach § 6 Abs. 2a EStG | **STARK** |
| p6_2a_sammelposten_zufuehrung: zufuehrung | (Bildung im WJ — Bestandteil des Sammelpostens) | — | E6003302-Umfeld | Sammelposten-Bildung | MITTEL (Zeile-Cross-Check) |
| p7g_1_iab_bildung: iab | Z. „abzüglich Investitionsabzugsbeträge nach § 7g Abs. 1 EStG" | — | E6004804 | abzüglich Investitionsabzugsbeträge nach § 7g Abs. 1 EStG | **STARK** |
| p7g_5_sonder_afa: sonder_afa_gesamt | Z. 35 „Sonderabschreibungen nach § 7b EStG und § 7g Abs. 5 und 6 EStG" | — | E6002201 | Sonderabschreibungen nach § 7b EStG und § 7g Abs. 5 und 6 EStG | **STARK** (Sammelfeld §7b+§7g) |
| p6_1_4_kfz_nutzungswert: nutzungswert_monat | Z. „Entnahmen einschließlich … Nutzungsentnahmen" | — | E6006601 | Entnahmen einschließlich Sach-, Leistungs- und Nutzungsentnahmen | MITTEL (Kfz-Privatanteil = Nutzungsentnahme, Sammelfeld — kein separates 1 %-Kz) |
| p7_1_lineare_afa: afa_betrag | Z. „Absetzung für Abnutzung (AfA)" | — | *AfA-Zeilenblock, Cross-Check* | AfA §7 | MITTEL (Zeile-Cross-Check offen) |

**Befund:** Der EÜR-Vordruck ist ein SUMMEN-Formular — der Steuerpflichtige deklariert Positions-Summen
(BE, BA, GWG-Summe, IAB-Betrag), nicht einzel-WG. Unsere Regeln liefern genau diese Positions-Beträge
je WG/Jahr → Andockung an die Übertrags-Zeilen (23/75/76/77) + die §-Positions-Zeilen (36/35/…).
Kfz-1 %-Nutzung fließt als **Nutzungsentnahme** (E6006601 Sammelfeld), NICHT als eigenes 1 %-Feld —
Mapping-Insight für die Integration.

## Anlage AUS — Kandidaten (Charge 20, § 34c)

| Regel | Konzept | E10-E-Nr | wörtliches Label | Konfidenz |
|---|---|---|---|---|
| p34c_2_abzug_statt_anrechnung: abzug | abgezogene ausl. Steuer § 34c Abs. 2 | E0600920 | In Zeile … abgezogene ausländische Steuern nach § 34c Abs. 2 EStG | **STARK** |
| p34c_1_anrechnung_hoechstbetrag: anrechnung | anzurechnende ausl. Steuer § 34c Abs. 1 | *E10-AUS-Sektion, Cross-Check aus_2025* | anrechenbare ausländische Steuern | MITTEL (Zeile-Cross-Check aus_2025 offen) |

## Anlage S / G — XSD-primär (kein Vordruck 2025)

Deklarations-Größen (Gewinn aus selbständiger Arbeit / Gewerbebetrieb) sind Übertrags-Felder aus
der EÜR/Bilanz in die ESt (E10). Konkrete E10-E-Nr je Einkunftsart-Zeile: **Cross-Check gegen E10-XSD
im nächsten Paket** (§ 15/§ 18-Gewinn-Übertrag). Keine eigene Rechenregel (Geltungsbedingungs-Pakete
C18), daher Mapping = die Gewinn-Übertragsfelder.

## Offene Punkte / nächstes Paket

1. **Betragsfeld-E-Nr** je Summen-Zeile (23/75/76/77) aus E77-XSD nachziehen (die Label-Suche traf
   Positions-Kz; die Summen-Übertrags-E-Nr brauchen den Zeilen-Cross-Check euer_2025 ⋂ E77).
2. **AUS Zeile-Cross-Check** (aus_2025 ⋂ E10) für die Anrechnungs-/Abzugs-Zeilen.
3. **S/G Gewinn-Übertragsfelder** (E10-XSD).
4. Feldmapping-YAML: die STARK-Kandidaten als `status: mapped` gesetzt (elster/feldmapping.stub.yaml),
   Rest `stub`. validate_mapping grün.
