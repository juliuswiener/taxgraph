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

## Bewusste Grenzen (Increment 1) und nächste Schritte

- **Zusammenveranlagung** analog (`Splittingtarif` statt `Grundtarif`) — nächstes Increment.
- **§ 32b (Progressionsvorbehalt) / § 34 (Fünftel, ermäßigter Satz)** modifizieren die
  Tarifgröße; die Regeln p32b/p34_fuenftel/p34_3 liefern die modifizierte tarifliche ESt.
  Sie treten unter ihrer Geltungsbedingung an die Stelle des `Grundtarif`-Aufrufs — als
  Tarif-Andockung im Scope zu verdrahten (noch als Andockpunkt-Kommentar markiert).
- **§ 2 Abs. 6 Exoten** (§ 32c-Unterschiedsbetrag, § 34c Abs. 5, Forstschäden-Zuschlag)
  bewusst als 0-Slots ausgelassen — AN-fern.
- **End-to-end-Golden/Harness**, die echte Regel-Outputs durch den Gesamt-Scope kettet
  (statt vorberechneter Slot-Werte) — schließt die Integration gegen einen Gesamtfall.

Increment 1 ist die tragende Stufenfolge; die Restpunkte sind Andockungen an bereits
formalisierte Regeln, keine neue Steuermechanik.
