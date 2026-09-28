# `rust/fixtures/`

`golden_cases.json` — Extrakt von `golden/cases/*.yaml` (135 Faelle), erzeugt von
`tools/parity/extract_golden.py`. Kein Rust-Runner konsumiert diese Datei in Schritt 1
(„Foundation") — sie liegt bereit fuer den Golden-Runner ab Schritt 4 (`REWRITE_PLAN.md` §7).

## Format

Ein JSON-Array, ein Objekt je Fall:

```json
{
  "id": "an_2025_einzel_ep",
  "beschreibung": "...",
  "sachverhalt": { "bruttoarbeitslohn": 40000, "veranlagungszeitraum": 2025, "...": "..." },
  "erwartung": { "festzusetzende_est": 6629 },
  "quelle": { "authority": "gesetz", "fundstelle": "...", "zitatanker": "..." }
}
```

- `sachverhalt`/`erwartung`: unveraendert aus dem YAML uebernommen, ein freies Dict je Fall
  (keine feste Feldliste — jeder Fall traegt nur die Schluessel, die er braucht).
- `quelle`: die Zitat-/Herkunftsangabe, fuer die Anker-Verifikation gegen `sources/`.

## Einheit: EURO, nicht Cent

Alle Zahlenwerte stehen wie im Quell-YAML in **vollen EURO** (`zu_versteuerndes_einkommen:
100000` = 100.000 €), nicht in Cent.

Ein blindes ×100 auf jeden Integer waere hier falsch: `sachverhalt`/`erwartung` sind freie
Dicts ohne Schema, das Geld- von Nicht-Geld-Schluesseln unterscheidet (`veranlagungszeitraum:
2025` ist ein Jahr, kein Betrag). Die Cent-Umrechnung braucht die `domain`-Typen (welches Feld
ist `Cent`, welches nicht) — die entstehen erst mit dem Golden-Runner in Schritt 4. Bis dahin
bleibt der Extrakt 1:1 zum YAML, und jeder Verbraucher rechnet selbst um, mit dem Wissen, was
Geld ist.

## Neu erzeugen

```
python3 tools/parity/extract_golden.py
```
