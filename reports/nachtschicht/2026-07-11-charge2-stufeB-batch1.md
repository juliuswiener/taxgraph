# Charge 2 Stufe B — Batch 1 (nr6_7 + nr5a) + Infra-Fix — 2026-07-11 Nacht

## Infra-Fix: Judge-Provider-Pin (models.yaml)

Der erste Kaskaden-Lauf (nr6_7) hing im Judge (`role_timeout`, wall 116s, nichts
produziert). Diagnose: OpenRouter-Endpoint-Status für `deepseek/deepseek-v4-pro`
bei **Together = -2 (degradiert)** — die Judge-Calls hingen >300s in der Bounded-
Retry-Schleife. Die Probe nr5a hing genauso → Infra, nicht Regel.

**Provider-Pin** (Instructor msg 1165, NUR Provider, KEIN Modellwechsel):
`roles.judge.providers: ["together"] → ["deepinfra"]`. DeepInfra = US/westlicher
Hoster, NICHT auf der Chinesisch-Ausschlussliste (data-sovereignty-konform);
Probe-Call 0,7 s / „OK". models.yaml-Hash bewusst geändert. Danach liefen beide
Regeln sauber durch (Judge-Latenz normal).

## Batch-Ergebnisse (frisch)

| Regel | status | wall | Kosten | A | B | equiv | rund_lint | clerk |
|---|---|---|---|---|---|---|---|---|
| p9_1_3_nr6_7_arbeitsmittel_afa | flagged_for_review | 150s | $0,0838 | syntax/typecheck PASS | **FAIL (kein Catala-Block)** | FAIL (B fehlt) | PASS | FAIL (3/6) |
| p9_1_3_nr5a_uebernachtung | flagged_for_review | 48s | $0,0329 | PASS | PASS | **FAIL (4/4 divergieren)** | PASS | FAIL (1/3) |

## Befunde (Discoveries → Instructor-Triage)

**nr6_7 — echter Formalisierungs-Bug in A (kein Seed-Fehler):**
A rechnet die AfA-Zwölftelung mit `(12 − anschaffungsmonat)/12` statt
`(13 − M)/12`. Die Norm (§ 7 Abs. 1 S. 4) mindert „um ein Zwölftel je vollem Monat,
der dem Monat der Anschaffung VORANGEHT" = (M−1) Monate → Jahr-0-Faktor (13−M)/12.
A ist um einen Monat daneben. **Die 3 fehlgeschlagenen Seeds sind korrekt** (mein
Oracle 200,00 bei M=7 folgt der Norm; A liefert 166,67). Der Judge fing denselben
Fehler unabhängig (3 Abweichungen zur Zwölftelung). Die 3 bestehenden Seeds: GWG
(500/800) + Folgejahr (400). B lieferte gar kein Catala-Block (syntax_b FAIL).
Judge-Discoveries: 3 abweichungen, 14 annahmen, 3 scope_gap.

**nr5a — B badly wrong, A ≈ korrekt:**
Beide kompilieren, aber equivalence 4/4 divergiert. Erster Rasterpunkt
{800/Monat, 12, 10}: A = $9.600,00 (= mein Oracle 800×12, korrekt), B = $1.600,00
(mis-formalisiert). 2/3 Seeds bestehen gegen A; der eine Fehler liegt an der
Cap-Boundary (48/49). Judge-Discoveries: 0 abweichungen, 7 annahmen, 2 scope_gap.

## Bewertung

Beide Regeln ehrlich **flagged_for_review** — die Kaskade surft echte
Formalisierungs-Divergenzen, kein Falschgrün. Manifest/Seeds sind belastbar (die
Seeds fingen A's Zwölftelungs-Bug). Nächster Schritt liegt bei dir: Discoveries
triagieren; ob A/B neu formalisiert werden (redo_a/redo_b — Prompts UNVERÄNDERT
per Dekret) oder die Befunde ins Morgen-Paket.

## Kosten

Clean-Runs: nr6_7 $0,0838 + nr5a $0,0329 = **$0,1167** (≈ Schätzung 0,12).
Infra-Schleife (2 hängende Läufe + Provider-Proben): ~$0,11. **Nacht-Summe
kumuliert (OpenRouter key-usage): $0,23** — unter dem 0,5-Infra-Deckel und dem
10-USD-Nachtbudget.
