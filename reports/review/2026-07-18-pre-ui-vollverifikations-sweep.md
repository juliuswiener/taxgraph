# Pre-UI-Vollverifikations-Sweep (dev-2, 2026-07-18)

**Auftrag:** unabhängige Gesamt-Verifikation, dass der Rechen-Ring live-komplett + solide ist, BEVOR die
UI-Politur drüberkommt. Frisch gelaufen, Grün aktiv misstraut ([[falsches-gruen]]). HEAD 8270bd6.

## ERGEBNIS: FUNDAMENT SOLIDE ✅ (ein env-gated Caveat)

| Gate | Ergebnis |
|---|---|
| **golden-runner** (alle Fälle, engine-truth) | **120/120 bestanden** |
| **volle pytest-Suite** (inkl. HTTP-e2e) | **416 passed, 1 skipped** |
| Anker-Gate + Drift-Wächter + est_mapping + konsistenz (frisch, isoliert) | 92 passed, 1 skipped |
| **Negativtest-Substanz** (Nicht-Vakuität) | ~30 tamper/fail-closed-Tests grün |
| git-Tree | clean (nur untracked Reports, KEIN Code-Rest) |
| checkESt/ERiC-Pfad | **SKIP** — HID env-gated (settings.json, nicht im Shell-Profil); zuletzt VOLL GRÜN rc=0 committet d4156b8 |

## Nicht-Vakuität belegt (Grün-Misstrauen)
Die grüne Suite ist NICHT leer-grün — ~30 Negativ-/Tamper-Tests beweisen, dass die Gates auf Manipulation
FEUERN: `test_neg_kz_wegnahme_wird_rot`, `test_negativtest_realer_anker_manipuliert`, `test_neg_guard_
feuert_wirklich`, `test_neg_schema_manipuliertes_event`, `test_negativtest_reales_fragment_manipuliert`,
`test_neg_null_kz_als_enr_wird_rot`, `test_neg_import_beleg_bestaetigt_schema_invalid` u.a. Kz-Eindeutigkeit,
Anker-Deckung, fail-closed-Guards, Store-Zwei-Signal alle mit rot-bei-Tamper-Gegenprobe.

## checkESt-Caveat (ehrlich)
Der ERiC-Plausibilitäts-Pfad (elster/checkest_gate.py --prove) braucht $ELSTER_HERSTELLER_ID, das im
settings.json-env liegt und NICHT ins Bash-Subshell-Profil exportiert ist (HID-Länge 0). Kein Neu-Lauf in
diesem Sweep möglich. Letzter verifizierter Stand: VOLL GRÜN rc=0 (commit d4156b8, Falsch-Grün-Sperre gegen
ERiC-Fehler-Kappung aktiv). Falls du einen frischen checkESt willst: braucht die HID im Shell-env (Julius).

## Fazit
Der Rechen-Ring ist live-komplett + solide: alle 120 Goldens engine-truth, volle Suite grün, Gates
non-vakuös, Baum sauber. KEIN Regressions-/Anker-Bruch-Fund. Die UI-Politur kann auf solidem Fundament
aufsetzen. Einziger offener Punkt = frischer checkESt (env-gated, zuletzt grün) — auf Julius' HID falls du
ihn vor UI willst.
