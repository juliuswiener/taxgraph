# charge29 Konsolidierungs-Verifikations-Sweep — dev-2, 2026-07-18

**Verdikt: GRÜN — 5 Materialisierungen + p32a-Floor + §35a-Scheibe solide. Keine Funde.**
Unabhängige Re-Verifikation (nicht Commit-Claim). Read-only, kollisionsfrei mit dev-1s golden/runner.py-#8.

## Belege
| Prüfung | Ergebnis |
|---|---|
| Volle Suite Run 1/3 | 518 passed, 2 skipped (190.7s) |
| Volle Suite Run 2/3 | 518 passed, 2 skipped (194.3s) |
| Volle Suite Run 3/3 | 518 passed, 2 skipped (191.7s) |
| → Stabilität | 518/518/518 identisch — KEIN Flaky; Daemon-Thread-Fix hält |
| Order-Isolation (e2e-HTTP zuerst → catala-e2e) | 64 passed — cross-file catala-Global-State-Leak-Fix hält |
| Golden Engine-Truth | 121/121, EXIT 0, Floor-Case gesamt_2026_einzel_ermaessigung_floor est=0 |
| Drift-Wächter + Anker-Gate + ERiC-E10-Kz | 21 passed (E10-Kz nicht geskippt) |
| clerk build p32a-python (alle 7 Module) | Build successful (Typechecking successful) |

## Byte-Gleichheit RE-verifiziert (python-Blockvergleich catala_a ↔ Snapshot)
| Modul | Datei | byte-gleich |
|---|---|---|
| Haushaltsnahe | p35a/haushaltsnahe.catala_en | ✓ |
| SpendenAbzug | p10b/spendenabzug.catala_en | ✓ |
| AgbAbzug | p33/agbabzug.catala_en | ✓ |
| Kirchensteuerabzug | p10/kirchensteuerabzug.catala_en | ✓ |
| ZumutbareBelastung | p33/zumutbarebelastung.catala_en | ✓ |

## Anker voll-Länge (Zitatanker im Modul-Doc ↔ amtliche Quelle, normalisiert)
- Haushaltsnahe → "20 Prozent, höchstens 510 Euro, der Aufwendungen des Steuerpflichtigen" (estg_p35a) ✓
- SpendenAbzug → "20 Prozent des Gesamtbetrags der Einkünfte" (estg_p10b) ✓
- AgbAbzug → "zumutbare Belastung (Absatz 3) übersteigt, vom Gesamtbetrag der Einkünfte abgezogen" (estg_p33) ✓
- Kirchensteuerabzug → "gezahlte Kirchensteuer" (estg_p10) ✓
- ZumutbareBelastung → "über 15 340 EUR bis 51 130 EUR" (estg_p33_abs3) ✓

## Bekannt / geflaggt (kein Blocker)
- 2 Skips unverändert (Kontoauszug-LLM-Fixture Julius-Cap + 1 Env-Skip).
- §10 Abs.4b Erstattungsüberhang-Hinzurechnung = benannter Nachtrag (dev-1-KiSt-Scheibe fail-closed bei erstattet>gezahlt).
- §35a-Floor NO-OP für die 120 Bestandsgoldens (nur der Floor-Case testet den Deckel).

## Fazit
Kein Fund. Alle 5 Materialisierungen byte-gleich + anker-fest, p32a-Floor korrekt (Floor-Case=0, Bestand no-op),
Suite mehrfach-run-stabil, engine-truth 121/121, alle 7 Module typechecken. charge29 konsolidierungs-fest.
