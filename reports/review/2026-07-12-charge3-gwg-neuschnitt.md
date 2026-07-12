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

## Addendum — deterministischer Lauf-Befund (skip-judge, 2026-07-12)

Der Zuschnitt wurde formalisiert. Nach zwei durch Infrastruktur verdorbenen Läufen (typecheck-
Run-Varianz; Judge-Provider DeepInfra-429, $0) lieferte ein `--skip-judge`-Lauf ($0,028) den
**deterministischen Zuschnitts-Nachweis** ohne Judge-Abhängigkeit:

- **typecheck_a=PASS** (A kompiliert sauber — der vorige typecheck-Fehler war Run-Varianz).
- **Judge-Gates SKIP** (Falschgrün-Sperre wirkt: `judge_verdict={skipped:true}`, queue_status würde
  ohne clerk-FAIL `strukturgeprueft_judge_offen`, nie verified).
- `catala_a`: `ist_gwg = netto <= 800`, GWG-Abzug = **brutto**, JahresAfA = **brutto**/ND. **hinweis
  gelandet.**

**clerk per-Seed (selbst gefahren, Ground-Truth):**

| Seed | netto | brutto | ND | M | jahre_seit | erwartet | clerk |
|---|---|---|---|---|---|---|---|
| 0 | 500 | 500 | 3 | 1 | 0 | 500,00 | PASS |
| 1 | 800 | 800 | 3 | 1 | 0 | 800,00 | PASS |
| 2 | 801 | 801 | 3 | 1 | 0 | 267,00 | PASS |
| 3 | 1200 | 1200 | 3 | 7 | **0** | **200,00** | **FAIL** (liefert 400) |
| 4 | 1200 | 1200 | 3 | 7 | **1** | **400,00** | **FAIL** (liefert 200) |
| 5 (KREUZ GWG) | 800 | 952 | 3 | 1 | 0 | 952,00 | **PASS** |
| 6 (KREUZ AfA) | 850 | 1011,50 | 5 | 1 | 0 | 202,30 | **PASS** |

**Netto/Brutto-Achse GELÖST:** die beiden Kreuz-Seeds (5, 6) — der Kern-Beweis, dass netto die
Grenze prüft und brutto den Abzug/die AfA trägt — **passen**. hinweis-Kanal-Erfolg deterministisch
belegt (Klasse 1, Netto/Brutto).

**Zweiter, orthogonaler Defekt (Klasse 2, Boundary-Kodierung):** Seeds 3/4 fallen, weil das Modell
die pro-rata-Kürzung an `jahre_seit_anschaffung = 1` bindet; die Seeds definieren das Anschaffungs-
jahr als `jahre_seit_anschaffung = 0`. Off-by-one in der Jahr-Index-Konvention — sauber getrennt von
der Netto/Brutto-Sache. Der auszug (§ 7 Abs. 1 S. 4: „Im Jahr der Anschaffung … ein Zwölftel für
jeden vollen Monat") liefert die Input-Encoding-Semantik (0 = Anschaffungsjahr) nicht → erneut ein
**hinweis-Kandidat**, keine auszug-Weitung. Vorschlag: hinweis um einen Satz erweitern
(„`jahre_seit_anschaffung = 0` ist das Anschaffungsjahr, anteilig; 1..ND-1 sind Volljahre"), dann
EIN Neulauf — pendet auf Instructor-Freigabe (die „genau einer"-Marke ist mit dem Nachweis-Lauf
verbraucht).

Status: `strukturgeprueft_judge_offen` + clerk-rot auf 2 Konventions-Seeds. Rule-Spec in `rules.yaml`
bis zum Konventions-Fix uncommitted gehalten. Infra `--skip-judge` + Falschgrün-Sperre + Test:
Commit `53b6ec5` (Suite 115 passed).
