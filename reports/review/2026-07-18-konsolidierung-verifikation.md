# Konsolidierungs-Verifikations-Sweep — dev-2, 2026-07-18

**Verdikt: GRÜN — das ganze Produkt (UI + Person-B + 3 Input-Kanäle + Repeated-Instance-Trias) solide.**
Unabhängige Gesamt-Verifikation nach der vollen Runde. Keine Funde.

## Belege
| Prüfung | Ergebnis |
|---|---|
| Volle Suite Run 1/3 | 482 passed, 2 skipped (139.8s) |
| Volle Suite Run 2/3 | 482 passed, 2 skipped (139.8s) |
| Volle Suite Run 3/3 | 482 passed, 2 skipped (141.2s) |
| → Stabilität | 482/482/482 identisch — KEIN order-/timing-Flaky; Daemon-Thread-Fix (daemon_threads=False+server_close) hält |
| Order-Isolation (e2e-HTTP zuerst → catala-e2e) | 46 passed (113.8s) — cross-file catala-Global-State-Leak-Fix hält |
| Golden Engine-Truth (`python golden/runner.py`) | 120/120 bestanden, EXIT=0 |
| Store-Guards (llm/beleg/vorjahr/kontoauszug/berechnet fail-closed) | 10 passed — alle 5 Schreiber-scoped Guards erzwingen vorlaeufig+signal_2=null |
| Drift-Wächter (instanz + A/B + art-bewusst) + Anker-Gate | 21 passed |
| ERiC E10-Kz-Existenz-Check (test_c, ERIC-Env aktiv) | 2 passed — NICHT geskippt (alle elster_kz gegen E10-2025.html XSD verifiziert) |

## Abgedeckte Runde (alles grün)
- **UI-Re-Freeze** (87ec06b): K2-Concurrency-Race-Fix (single-thread HTTPServer), 4-Punkt-Gegencheck grün.
- **Person-B** (Zusammenveranlagung): Kapital §20 (Klasse g) + Rente §22 (Klasse g×f).
- **3 Input-Kanäle / Store-Writer**: ^import:vorjahr, ^import:kontoauszug (+ ^import:beleg), ^berechnet:maps (defense-in-depth-Guard).
- **Repeated-Instance-Trias**: Kern (d91bdb1, `base__<n>`-Separator store-invariant) → Multi-Objekt-§21 (5451fb6, 1:1+Aggregat je Objekt) → Per-Kind (b5eb2df, Anlage-Kind-Formfelder, Kz-Review §32-Anker) → Multi-Rente (b4a7a7d, VERZWEIGUNG-je-Instanz Kern-Extension strikt additiv) + Multi-Rente-Komplettierung (alter/rentenfreibetrag per-Rente, ring-ready).

## Offen (kein Blocker, bekannt/geflaggt)
- **dev-1 Ring-Σ-Nachtrag** (dev-1-Zone): Multi-Objekt `einkuenfte_vermietung` über Objekte + Multi-Rente per-Rente-Ertragsanteil (catala_renten_einkuenfte je Instanz). Deklarations-Seite liefert die per-Instanz-Daten (anlage_instanzen + per-Rente-alter/rentenfreibetrag im Snapshot); die Ring-Verdrahtung ist dev-1s.
- **2 skips** (unverändert, bekannt): Kontoauszug-LLM-Recorded-Fixture (Julius-Cap-Live-Call offen), + 1 weiterer bekannter Env-Skip.

## Fazit
Kein Fund. Alle Gates deterministisch grün, mehrfach-run-stabil, engine-truth-belegt. Produkt konsolidierungs-fest.
