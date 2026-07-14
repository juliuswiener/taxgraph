# § 2-Integration — verallgemeinerte Veranlagung (Increment 1, 2026-07-14)

Der nächste Wert nach der 40/40-Regelabdeckung: die einzeln verifizierten Regeln zu **einer**
durchgehenden Berechnung verdrahten. Bisher war nur der Arbeitnehmerfall (§ 19 →
festzusetzende ESt) als Kette da; die übrigen Einkunftsarten, Abzüge und Tarif-Mechanismen
hingen als Einzelregeln daneben.

## Was dieses Increment liefert

Neuer Scope **`FestzusetzendeEstGesamt`** (Einzelveranlagung), co-located im Tarifmodul
`Einkommensteuertarif` (er ruft `Grundtarif`, braucht also den VZ-Enum). Er rechnet die
**vollständige § 2-Abs-3-bis-6-Stufenfolge**, wörtlich aus der eingefrorenen Quelle
`sources/gesetze-im-internet/estg_p2_2026-07-14.txt` (sha256-geprüft, `make sources-check`).

Jede Andockstelle ist eine `money`-Eingabe = das Ergebnis der jeweils eigenen, bereits
`verified_bedingt`-Regel:

| § 2-Stufe | Rechnung | Andock-Regeln |
|---|---|---|
| Abs. 3 S. 1 — Summe der Einkünfte | Σ der (tariflichen) Einkunftsarten | § 19 (p09/p04), § 20 tariflich, § 21 (p21), § 22/§ 23 (p22/p23_3), § 13/15/18 (Slot) |
| Abs. 3 — Gesamtbetrag der Einkünfte | SdE − § 24a − § 24b | p24a, p24b |
| Abs. 4 — Einkommen | GdE − Sonderausgaben − agB | § 10/10a/10b/10c/10d, § 33/33a/33b |
| Abs. 5 S. 1 — zu versteuerndes Einkommen | Einkommen − § 32-Freibeträge − sonstige | p32_6 |
| Abs. 5, § 32a — tarifliche ESt | Grundtarif(zvE) | p32a |
| Abs. 6 — festzusetzende ESt | tarifl. − ausl. St. − Ermäßigungen + § 32d Abs. 3/4 + § 31-Kindergeld + § 10a-Zulage | § 34c, § 35a/35c, p32d, p31, p10a |

**Design-Entscheidung (§ 2 Abs. 5b, wörtlich belegt):** abgeltend nach § 32d Abs. 1 / § 43
Abs. 5 besteuerte Kapitalerträge sind **nicht** in die Summe der Einkünfte einzubeziehen —
sie durchlaufen die Tarif-Kette nicht, ihre Steuer erscheint erst in Abs. 6 (soweit nach
§ 32d Abs. 3/4 veranlagt). Das hält den Sondertarif sauber getrennt.

## Verifikation

Fünf clerk-Testfälle (`rules/estg/arbeitnehmerfall/tests_veranlagung_gesamt.catala_en`),
alle grün — **32/32 Scope-Tests gesamt**:

1. **Reduktion auf den Arbeitnehmerfall** — nur § 19, alle Slots 0: liefert exakt die
   MVP-Werte (zvE 58 734, ESt 13 747, VZ 2026).
2. **Mehrere Einkunftsarten** — § 19 + § 21: Summe der Einkünfte über Arten.
3. **Abzüge (Abs. 3/4) + Steuerermäßigung (Abs. 6)** — § 24a, Sonderausgaben, agB, § 35a.
4. **Kinderfreibetrag (Abs. 5) + Hinzurechnungen (Abs. 6)** — § 32-Freibetrag, § 31-Kindergeld,
   § 32d Abs. 3/4.
5. **Horizontaler Verlustausgleich (Abs. 3)** — negative Vermietungseinkünfte gegen § 19,
   negatives zvE → Tarif 0.

Die Tarif-Erwartungswerte sind an denselben zvE-Punkten verankert, die die
Arbeitnehmerfall-Tests bereits gegen die GETTSIM-Oracle pinnen — dieses File testet also
**nur die § 2-Stufenarithmetik**, nicht den Tarif. Golden 57/57 und Sources 69/69 unverändert grün.

## Increment 2-4 (2026-07-14, im selben Zug) — vollzogen

- **Increment 2 — Zusammenveranlagung.** Neuer Scope `FestzusetzendeEstGesamtZusammen`
  (§ 26b: Einkünfte zusammengerechnet, Ehegatten als ein Steuerpflichtiger), strukturgleich
  zur Einzelveranlagung mit `Splittingtarif` (§ 32a Abs. 5). Verankert an den Splitting-zvE-
  Punkten der Arbeitnehmerfall-Tests (zvE 97 468 → 20 212, 78 698 → 14 008).
- **Increment 3 — § 32b/§ 34-Tarif-Andockung.** `tarifliche_est` per **Default-Logic-Ausnahme**:
  Grundfall § 32a (bzw. Splitting), unter `tarif_modifiziert` (Progressionsvorbehalt § 32b,
  Fünftel/ermäßigter Satz § 34) tritt die von p32b/p34 gelieferte tarifliche ESt an seine
  Stelle. Idiomatisches Catala (zwei sich ausschließende `under condition`-Zweige), kein
  Sonderpfad. Test: modifizierter Tarif ersetzt den Grundtarif, Abs-6-Arithmetik wirkt weiter
  auf den modifizierten Betrag.
- **Increment 4 — End-to-end-Kette echter Regel-Outputs.** Ein clerk-Test kettet drei reale
  Module: § 9 Entfernungspauschale (30 km, 220 Tage, VZ 2026 → 2 508 € WK) → § 19/§ 9a-
  Einkünfte (Brutto 31 278 − 2 508 → 28 770) → `FestzusetzendeEstGesamt` (zvE 28 734 →
  festzusetzende 3 862). Beweist die Integration gegen einen echten Gesamtfall, nicht nur
  vorberechnete Slot-Werte. **35/35 Scope-Tests grün.**

## Verbleibende bewusste Grenzen

- **§ 2 Abs. 6 Exoten** (§ 32c-Unterschiedsbetrag, § 34c Abs. 5, Forstschäden-Zuschlag)
  bleiben 0-Slots — AN-fern.
- **Python-Golden über den Gesamt-Scope** (statt clerk-Kette): der golden/runner.py-Zweig
  für einen Gesamtfall ist noch offen; die clerk-End-to-end-Kette deckt den Beweis bereits ab.
- **§ 32b/§ 34 als echte Verzahnung in der Integration** (die Bedingung + der modifizierte
  Wert aus p32b/p34 automatisch bestimmt) — der Andock-Mechanismus steht, die automatische
  Fallunterscheidung in einem Gesamt-Runner ist der nächste Schritt.

Die tragende § 2-Stufenfolge steht damit für Einzel- und Zusammenveranlagung inklusive
Tarif-Modifikation; die Restpunkte sind Runner-Verdrahtung, keine neue Steuermechanik.
