# VZ-Golden-Vorbereitung 2024/2025 + Stufe-1-Loader-Befund (taxgraph-dev-2, 2026-07-15)

Instructor-Bau-GO 2026-07-15: DU = Stufe 1 (Loader) + VZ-Golden-Vorbereitung
(Erwartungswerte 2024/2025 hand-rechnen, je VZ dokumentiert). Deliverable liegt in
reports/ (meine Zone). Werte hand-gerechnet, deterministisch, $0; M3-params sind
GETTSIM-1.2.1-verifiziert.

## ZONEN-BEFUND (Stufe 1) — Loader NICHT angefasst

`golden/runner.py` UND `golden/cases/` liegen außerhalb meiner additiven Zonen
(params/, reports/) und sind nicht dev-1-TABU → Instructor-Regel „Befund melden statt
anfassen". Ich liefere den fertigen Patch als Befund; Einbau durch golden/-Eigner.

**Loader-Erweiterung (ready-to-apply, analog `_ep_saetze`):**
```python
def _kindergeld(year: int) -> int:
    p = load_yaml_fh(open(os.path.join(ROOT, "params", str(year),
                     "kindergeld_p66.yaml"), encoding="utf-8"))
    return p["kindergeld_monatlich_je_kind"]["wert"]      # 250 / 255 / 259

def _vorsorge_hb(year: int) -> int:
    p = load_yaml_fh(open(os.path.join(ROOT, "params", str(year),
                     "vorsorge_hoechstbetrag_p10.yaml"), encoding="utf-8"))
    return p["hoechstbeitrag"]["wert"]                    # 27566 / 29344 / 30826
```

**NUANCE (nicht trivialer Passthrough):**
- `kindergeld` speist im Gesamt-Scope `hinzurechnung_kindergeld_in` (catala_gesamt,
  runner.py:~130) — das ist der Jahres-Hinzurechnungsbetrag der Günstigerprüfung
  (= kindergeld_monatlich × zu berücksichtigende Kind-Monate), NICHT die 250/255 direkt.
  Der Loader liefert den Monatswert; die Monats-/Kinder-Aggregation ist Integrations-
  logik (familie-Scope oder Sachverhalt-Input). → Wiring-Entscheidung golden/-Eigner.
- `vorsorge_hb` speist den Cap-Input von p10_1_2_altersvorsorge (`hoechstbeitrag_
  knappschaft`). Direkter Passthrough, sobald die Regel im Scope verdrahtet ist.

## VZ-GOLDEN-VORBEREITUNG — Erwartungswerte je VZ (hand-gerechnet)

Kreuzvalidierung: (1) Tarif = exakte Replik `golden/generate_cases.py::tarif` (BMF-
Rechner-bestätigtes Corpus); (2) 2026-Werte matchen die FROZEN rule-seeds
(p32_6 expected [4878/9756/19512] ✓, solzg [20351→0,11] ✓); (3) params GETTSIM-1.2.1-
verifiziert (M3, 27/27). Damit ist die 2024/2025-Extrapolation triangduliert.

### A. Grundtarif § 32a (Kontrolle — Cases existieren schon)
| VZ | zvE 30.000 | zvE 60.000 |
|---|---|---|
| 2024 | 4.412 | 14.646 |
| 2025 | 4.303 | 14.415 |

### B. Solidaritätszuschlag (solzg) — BMG = tarifl. ESt, cent-genau
Soli = 0 bei BMG ≤ Freigrenze; sonst min(5,5 % × BMG; 11,9 % × (BMG − FG)), Cent abgeschnitten.
| VZ | FG einzel / split | BMG=FG (einzel) | BMG=FG+1 | BMG=40.000 einzel | BMG=FG_split (split) | BMG=70.000 split |
|---|---|---|---|---|---|---|
| 2024 | 18.130 / 36.260 | 0 | 0,11 | 2.200,00 | 0 | 3.850,00 |
| 2025 | 19.950 / 39.900 | 0 | 0,11 | 2.200,00 | 0 | 3.581,90 |
| 2026 | 20.350 / 40.700 | 0 | 0,11 | 2.200,00 | 0 | 3.486,70 |

### C. Kinderfreibetrag § 32 VI (p32_6) — freibetraege_kinder
| VZ | 1 Kind einzel | 1 Kind zusammen | 2 Kinder zusammen | (KFB + BEA) |
|---|---|---|---|---|
| 2024 | 4.770 | 9.540 | 19.080 | 3.306 + 1.464 |
| 2025 | 4.800 | 9.600 | 19.200 | 3.336 + 1.464 |
| 2026 | 4.878 | 9.756 | 19.512 | 3.414 + 1.464 (✓ rule-seed) |

### D. Kindergeld (p31-Input) + Jahres-Hinzurechnung (ganzjährig)
| VZ | €/Monat | 1 Kind | 2 Kinder |
|---|---|---|---|
| 2024 | 250 | 3.000 | 6.000 |
| 2025 | 255 | 3.060 | 6.120 |
| 2026 | 259 | 3.108 | 6.216 |

### E. Unterhalt § 33a (p33a): abzug = min(aufw, HB + kv − max(0, eigene−624))
| VZ | HB | aufw 15.000, eig 0 | aufw 15.000, eig 1.000 | aufw 8.000 |
|---|---|---|---|---|
| 2024 | 11.784 | 11.784 | 11.408 | 8.000 |
| 2025 | 12.096 | 12.096 | 11.720 | 8.000 |

### F. Kinderbetreuung § 10 I 5 (p10_1_5) — STRUKTUR-Drift: min(satz × kosten, Deckel)
| VZ | Satz/Deckel | kosten 3.000 | kosten 6.000 | kosten 9.000 |
|---|---|---|---|---|
| 2024 | ⅔ / 4.000 | 2.000 | 4.000 (Deckel) | 4.000 (Deckel) |
| 2025 | 80 % / 4.800 | 2.400 | 4.800 (Deckel) | 4.800 (Deckel) |
Hinweis: ⅔ exakt-rational rechnen (M5). Golden-Werte hier bei durch 3 teilbaren kosten
gewählt, damit ⅔ ganzzahlig ist (keine Cent-Rundungs-Ambiguität).

### G. Vorsorge-Höchstbetrag § 10 III (p10_1_2-Input, Cap)
| VZ | Cap | = ⌈BBG-knappsch × 24,7 %⌉ |
|---|---|---|
| 2024 | 27.566 | ⌈111.600 × 0,247⌉ |
| 2025 | 29.344 | ⌈118.800 × 0,247⌉ |
| 2026 | 30.826 | ⌈124.800 × 0,247⌉ |

## Verwendung
Diese Tabellen sind die reviewte Erwartungswert-Vorlage für golden/cases/-Dateien je VZ
(Format `id/beschreibung/sachverhalt/erwartung/quelle`, s. golden/schema.md). Sie sind
die Ziel-Werte, die die nach Stufe 2/3/4 VZ-parametrisierten Regeln treffen müssen.
Erzeugung der golden/cases/-Dateien = golden/-Eigner-Zone (Befund, nicht angefasst).

## Offene Punkte
- Golden-Fall-Erzeugung (golden/cases/) + Loader-Einbau (golden/runner.py): golden/-Zone
  — wer ist Eigner (dev-1 oder Julius direkt)? Ich liefere Werte + Patch, baue nicht.
- Kindergeld-Hinzurechnung: Monats-/Kinder-Aggregation als Sachverhalt-Input oder
  Integrations-Rechnung? (siehe Nuance oben).
