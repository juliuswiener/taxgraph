# Charge-3-Zuschnitt: GWG Netto/Brutto — Neuschnitt `p9_1_3_nr6_7_afa_laufend`

Erster Charge-3-Zuschnitts-Report. Löst den GWG-Netto/Brutto-Gap (B5), jetzt amtlich spezifiziert:
die 800-€-GWG-Grenze prüft die **Netto**-AK (Anleitung N verifiziert), der Abzug/die AfA-Bemessung
läuft auf der **Brutto**-AK (§ 9b: Vorsteuer beim AN i.d.R. nicht abziehbar).

## Problem (bisheriger Interim-Zustand)

`afa_laufend` hat EINEN Input `anschaffungskosten`, der Grenzprüfung (§ 6 Abs. 2 S. 1: ≤ 800 →
Sofortabzug) UND Abzugsbetrag/AfA-Bemessung trägt. Netto vs. Brutto lässt sich mit einem Input nicht
trennen. Zwei Interim-Geltungsbedingungen dokumentieren das:
`grenzwert_und_abzugsbetrag_fallen_zusammen` + `anschaffungskosten_sind_massgebliche_ak`.

## Neuschnitt — Signatur (SPLIT)

```
inputs:
  anschaffungskosten_netto  money   # § 6 Abs. 2 S. 1: massgeblich fuer die 800-Euro-GRENZPRUEFUNG
  anschaffungskosten_brutto money   # § 9b Abs. 1: massgeblich fuer ABZUG / AfA-Bemessung (AN: brutto)
  nutzungsdauer_jahre       int
  anschaffungsmonat         int
  jahre_seit_anschaffung    int
output: abziehbar           money
```

Rechenlogik: `if anschaffungskosten_netto <= 800 EUR then Sofortabzug(anschaffungskosten_brutto)`
`else lineare AfA auf anschaffungskosten_brutto` (mit unterjährigem Anteil + Volljahren wie bisher).

## Drei Quellen (Multi-Source, alle vorhanden)

1. **gesetz** § 6 Abs. 2 EStG (GWG-Grenze 800 €, Sofortabzug) — `sources/gesetze-im-internet/…` (§ 6-Freeze).
2. **gesetz** § 9b Abs. 1 EStG (Vorsteuer nur raus, soweit bei USt abziehbar → beim AN i.d.R. Brutto-AK).
3. **verwaltung** Anleitung Anlage N 2025 — `sources/bfinv/anleitungen/anl_n_2025.txt:307`:
   „Arbeitsmittel, die höchstens **800 € (ohne Umsatzsteuer)** … (sog. geringwertige Wirtschaftsgüter)".
   **Amtlicher Anker: Grenze NETTO.** Verifiziert.

## Wächter-Seeds (Kreuz-Fälle netto/brutto)

| Fall | ak_netto | ak_brutto | ND | Monat | Jahr | erwartet | Rechenweg |
|---|---|---|---|---|---|---|---|
| GWG-Kreuz (netto ≤ 800 < brutto) | 800 | 952 | – | 1 | 0 | **952,00** | netto 800 ≤ 800 → GWG → Sofortabzug BRUTTO 952 |
| Grenze exakt | 800 | 952 | – | 1 | 0 | 952,00 | ≤ 800 schließt 800 ein (Grenze, nicht Freibetrag) |
| AfA-Kreuz (netto > 800) | 850 | 1011,50 | 5 | 1 | 0 | **202,30** | netto 850 > 800 → AfA auf BRUTTO 1011,50 / 5 = 202,30 (Volljahr) |
| AfA Volljahr | 850 | 1011,50 | 5 | 1 | 1 | 202,30 | laufendes Jahr, volle JahresAfA auf Brutto |

Der Kern-Beweis ist der GWG-Kreuz-Fall: netto entscheidet die Grenze (800), brutto liefert den Abzug
(952) — genau die Trennung, die ein Einzel-Input nicht kann. (Bei USt-Satz 19 %: 800 × 1,19 = 952;
850 × 1,19 = 1011,50.)

## Ersetzungs-Verhältnis (Interim erlischt)

Mit dem Split **erlöschen beide Interim-Geltungsbedingungen**:
- `grenzwert_und_abzugsbetrag_fallen_zusammen` — der Grund (ein Input für beides) entfällt.
- `anschaffungskosten_sind_massgebliche_ak` (Netto/Brutto nicht festgelegt) — jetzt explizit zwei
  Inputs, netto/brutto benannt.
Der alte Monolith-Input `anschaffungskosten` wird zu `zuschnitt_ersetzt` (Historie bleibt); die
Teilregel-auszüge tragen § 6 Abs. 2 + § 9b + die Anleitungs-Passage. `afa_laufend` (aktuell
verified_bedingt mit Interim) wird durch die neu geschnittene Regel abgelöst.

## Nächste Schritte (nach Instructor-Freigabe → Stufe B)

1. Signatur + auszüge + drei Quellen (inkl. verwaltung-Anleitung) in `rules.yaml` (Experiment-Zweig
   erst? — nein, produktions-Zuschnitt nach Freigabe).
2. Wächter-Seeds als test_seed (Rechenweg belegt, keine LLM-Erwartungswerte).
3. Doppelformalisierung + Gates; die Präzisions-/Rundungs-Lints greifen automatisch.
4. clerk-Gate grün auf den vier Seeds. Erst dann verified_bedingt.
