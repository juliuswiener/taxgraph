# §34-Fünftel-Accessor + e2e-Golden (Task #11 Follow-up, 10c Block 7)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor · LLM-frei.

## Was gebaut wurde (Ruling i)

- **`golden/runner.py` `catala_fuenftel(s)`** — § 34 Abs. 1 Fünftelregelung, Kernfall verbleibendes
  zvE ≥ 0. **Orchestriert bestehende Scopes** (catala_gesamt-Muster, kein neuer Rechenpfad, keine
  Nachbildung der Tariflogik): der § 32a-Tarif kommt aus dem Catala-Scope `Einkommensteuertarif`
  (`grundtarif`/`splittingtarif`); der Faktor 5 ist die exakte Struktur-Konstante der Regel
  `p34_fuenftel_ao_est`. Glue = verbleibendes-zvE-Subtraktion, 1/5-Aufteilung, Summe.
  Dispatch in `catala_est` bei `ausserordentliche_einkuenfte`.
- **e2e-Golden `golden/cases/p34_fuenftel_ao_zv_72150.yaml`** — EStH 2025 H 34.2 Beispiel 1
  (ZV, zvE 72.150, ao 25.000 → tarifl. ESt **11.742**), authority `verwaltung`, Gesetz-Anker
  `estg_p34` (Zitatanker „beträgt das Fünffache des Unterschiedsbetrags", gegen den Freeze verifiziert).

## Verifikation

- Zwischen-Steuerbeträge über den echten § 32a-Splittingtarif VZ 2025 reproduziert:
  verbl. zvE 47.150 → **5.102**; +1/5 ao (52.150) → **6.430**; Unterschiedsbetrag 1.328 × 5 = **6.640**;
  5.102 + 6.640 = **11.742** — deckungsgleich mit dem amtlichen H-34.2-Beispiel.
- **Golden-Gate: 101/101 bestanden** (`python golden/runner.py`), neuer Fall `OK (est=11742)`.
- **Anker-Freeze-Gate grün** (`tests/test_golden_anker_freeze.py`, 4 passed).
- **Falsch-Grün-Guard:** ein negatives verbleibendes zvE (Abs.-1-S.-3-Fall) wirft `ValueError`
  („§ 34 Abs. 1 S. 3 … nicht modelliert"), rechnet NIE still einen falschen Wert.

## Benannte Lücke → Backlog Task #12

**H 34.2 Beispiel 2** (§ 34 Abs. 1 S. 3, NEGATIVES verbleibendes zvE, § 16-Veräußerungsgewinn,
amtliche Erwartung **12.010**) ist NICHT als Golden gebaut. Grund: die Regel `p34_fuenftel_ao_est`
klammert den Abs.-1-S.-3-Sonderpfad (`est_ao = 5 × est(zvE/5)`, dritter Tarif-Input) ausdrücklich aus
(deckt_ab vorhanden). Wird Golden, sobald der Sonderpfad als Catala-Scope modelliert ist (rules/estg,
dev-1-Zone). Die Erwartung 12.010 liegt geparkt (Golden-Kommentar + hier). Modellierungs-Posten in
**Backlog Task #12**.

## Geänderte Dateien

- `golden/runner.py` — `catala_fuenftel` + Dispatch (additiv).
- `golden/cases/p34_fuenftel_ao_zv_72150.yaml` — neuer e2e-Golden.

## Reproduktion

```bash
python3 golden/runner.py    # 101/101, u.a. p34_fuenftel_ao_zv_72150 OK (est=11742)
```
