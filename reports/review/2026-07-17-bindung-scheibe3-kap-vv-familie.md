# Bindungstabelle Scheibe 3 — Kapital §20 + V+V §21 + Familie (Task #11)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Zone:** `produkt/` (additiv). Kein neues Schema.

## Dateien

- `produkt/bindung/bindung_kap_vv_familie.yaml` — **20 Bindungen + 18 benannte Lücken** über 7 Regeln.
- `produkt/traverser/guenstiger_liste.yaml` — **p31 von `ausnahmen` → `guenstiger_regeln`** (Wanderung
  beim Scheiben-Eintritt, wie angewiesen).
- `tests/test_bindungstabelle.py` — 2 Negativtests datei-unabhängig gemacht (s.u.).
- Alle Paket-A-Gates zusammen: **53/53 grün, 0 Skips.**

## Abdeckung

| Regel | Bindungen | Kern |
|---|---|---|
| p20_9_sparer_pauschbetrag | 2 | kapitalertraege + zusammenveranlagung |
| p20_6_verlustverrechnung | 4 | Aktien-/Sonstige-Gewinn/Verlust-Töpfe |
| p21_vermietung_einkuenfte | 5 | Einnahmen, Gebäude-AfA, Schuldzinsen, Erhaltung, sonstige WK |
| p21_2_verbilligte_vermietung_wk | 4 | Werbungskosten + **Entgelt-Quote % (bereich 0..100)** + 2 Gates |
| p24b_entlastungsbetrag | 4 | alleinstehend, anzahl_kinder (bereich 0..20), monate (0..12), Kinder-im-Haushalt-Gate |
| p32_6_kinderfreibetraege | 1 | Gate „Kinder zu berücksichtigen" (Rest geteilt/Lücke) |
| p31_familienleistungsausgleich | 0 (7 Lücken) | Günstiger-Knoten, alle Inputs berechnet |

Alle 20 Anker voll-Länge via `_normalize` gegen die Freezes verifiziert; Vollständigkeit deterministisch
(jeder Slot + jede Geltungsbedingung → Bindung ODER benannte Lücke). elster_kz für §20/§21/Kind: im
2026-07-12-Katalog konzeptbelegt, XSD-E-Nr Sektions-Lookup-Nachtrag → `null` + Grund (kein Rate-Mapping).

## p31 Günstiger-Wanderung (§ 31 Familienleistungsausgleich)

Beim Scheiben-Eintritt von p31 ist der Eintrag von `ausnahmen` nach `guenstiger_regeln` gewandert —
mit `zweig_felder: [fam_anzahl_kinder, kap_zusammenveranlagung]` (beide Zweige — Kindergeld-Anspruch vs.
Kinderfreibeträge § 32 Abs. 6 — hängen von den Kinderdaten und der Veranlagungsart ab) und Anker
§ 31 S. 1 (verifiziert). p31 hat keine eigenen askable Slots: est_ohne/est_mit_freibetraegen/kindergeld
sind berechnete Tarif-/Anspruchsgrößen → 7 benannte Lücken. Sweep-Netz bleibt grün (Günstiger-Anker-
Test verifiziert den neuen §-31-Anker).

## Geteilte Slots (einmal binden + Lücke)

`zusammenveranlagung` (p20_9 ⟷ p32_6) und `anzahl_kinder` (p24b ⟷ p32_6) sind geteilte Laien-Felder:
einmal gebunden (kap_zusammenveranlagung / fam_anzahl_kinder), im UI einmalige Abfrage, für die zweite
Regel als Lücke mit Verweis geführt.

## Negativtest-Härtung (Falsch-Grün-Robustheit)

Die neue, alphabetisch erste Scheiben-Datei hat weder elster_kz noch summand-Felder → die beiden
Negativtests, die vorher „die erste Datei" nahmen, wären still übersprungen (skip ≠ bewiesen). Fix:
`test_neg_erfundene_kz` prüft direkt gegen das XSD (E9999999 ∉ E10-2025), `test_neg_gemischte_summanden`
sucht die Summand-Datei über ALLE Scheiben. Jetzt 0 Skips.

## Hinweis geteilter Tree

`pipeline/produktion/rules.yaml` zeigt `M` (dev-1 mid-edit, nicht meins; parst sauber, Gate grün).
Meine Beiträge: `bindung_kap_vv_familie.yaml` (neu), `guenstiger_liste.yaml` (p31-Wanderung),
`tests/test_bindungstabelle.py` (Negativtest-Härtung) + dieser Report.

## Reproduktion

```bash
ERIC_DIR=~/02_Software/eric python3 -m pytest tests/test_bindungstabelle.py tests/test_traverser.py -q
```
