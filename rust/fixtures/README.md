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

## `interview_orakel.json`

Szenarien fuer die Crate `interview` samt den Antworten des Python-Orakels
(`produkt/traverser/traverser.py`, `produkt/haut/bindung_rollen.py`). Konsument:
`rust/interview/tests/orakel_werte.rs` (hermetisch, ohne Python). Je Szenario: Events (Reihenfolge ist
Semantik), eine Sicht (voll, Teil-Bindung oder selbstgebaute Felder) und die erwarteten Antworten.
Die Frageliste steht als Positionen in der Sicht-Reihenfolge, die Relevanz als Abweichung zur
Relevanz des leeren Stores. Der Unsicherheits-Beitrag steht nicht im Fixture, die Formel steht in
Generator und Test.

```
python3 tools/parity/extract_interview_orakel.py
```

Aendert sich `traverser.py` oder die Bindung, ist das Fixture neu zu erzeugen; der Rust-Test zeigt
dann, wo Rust vom Orakel abweicht. `git diff` auf der Datei zeigt, was sich am Orakel geaendert hat.
