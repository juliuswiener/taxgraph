# Präzisions-Lint (Klasse 5) — Stufe 1 gebaut + Bestandssweep

Instructor-Freigabe 2026-07-12 (msg 1328): GO für Stufe 1 (informativ), Sweep direkt mit dem Bau.
Stufe 2 (blockierend) bleibt gated auf Stufe-1-Empirie + Julius. Vorregistrierung:
[[2026-07-12-praezisions-lint-vorregistrierung]].

## Gebaut

- `gates.py:praezisions_lint_gate(catala_src, candidate)` — deterministisch, $0.
  Erkennung v2 (typ-bewusst) + **Fluss-Sensitivität**: ein money×decimal flaggt nur, wenn sein
  Ergebnis (transitiv über let-Bindungen, mehrzeilige Bodies via `_praez_let_bodies`) in den finalen
  Cent-Schnitt fließt, ODER inline in der Schnitt-Expression steht. money×decimal NACH dem Schnitt
  oder in einer Nebenvariable → kein Flag.
- **Status INFO** (neu): confident-Befund meldet, kippt aber KEIN Gate (weder FAIL noch SKIP → die
  Queue-Entscheidung in cascade/run.py ignoriert ihn). Umschalter `_PRAEZISION_BLOCKIEREND = False`;
  Stufe 2 flippt confident → FAIL (eine Zeile, nach Julius).
- Verdrahtet in `cascade.py` nach `rundungs_lint` (Schritt 5a).
- 10 Tests in `tests/test_gate_semantik.py` (inkl. die 3 Adversarial-Fälle des Instructors +
  Stufe-2-Umschalter), pytest **112/112**.

## Bestandssweep (alle 21 Regeln mit catala_a)

| Ergebnis | Regeln |
|---|---|
| **INFO (Klasse-5-Verdacht)** | **solzg_solidaritaetszuschlag** (1) |
| PASS | die übrigen **20** — davon alle mit Detail „kein finaler Cent-Schnitt-Idiom" |

**0 False Positives, 1 echter Fund (solzg), exakt das vorregistrierte Kriterium.** Kein latenter
Zusatzfund: keine andere Regel trägt überhaupt das Cent-Schnitt-Idiom, also besteht dort keine
Klasse-5-Gefahr. solzg-Befund nennt beide money×decimal-Zeilen (`bemessungsgrundlage * 0.055`,
`unterschiedsbetrag * 0.119`), die vor dem `truncate`-Schnitt liegen.

## Empfehlung für Stufe 2 (an Instructor → Julius)

Die Stufe-1-Empirie ist eindeutig (1 Fund, 0 FP, Negativtests grün inkl. Fluss-Sensitivität). Das
spricht dafür, Stufe 2 (blockierend) freizugeben — ABER erst nachdem solzg selbst gefixt ist (sonst
blockiert das Gate die einzige betroffene Regel dauerhaft, ohne dass ein grüner Pfad existiert). Reihenfolge:
1. solzg auf die decimal-Fix-Form umstellen (Charge 3 / §2-Arithmetik-Nachbarschaft), clerk-Seed
   20351→0,11 grün.
2. Dann `_PRAEZISION_BLOCKIEREND = True` (Julius-Freigabe), Gate wird blockierend, solzg bleibt grün,
   künftige Klasse-5-Fälle kippen.

## Interaktion mit rundungs_lint (unverändert dokumentiert)

Klasse 4 (rundungs_lint) prüft die RICHTUNG des Schnitts → solzg PASS. Klasse 5 (praezisions_lint)
prüft die ORDNUNG davor → solzg INFO. Getrennte Meldungen, verschiedene Zeilen, keine Doppel-Flagge.
