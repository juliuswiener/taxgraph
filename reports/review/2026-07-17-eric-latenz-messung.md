# ERiC checkESt — Latenz-Messung (AUFTRAG 1)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Zweck:** Den einzigen offenen Lab-Board-Widerspruch entscheiden — darf die Sachverhalts-Eingabe
das dritte Orakel (ELSTER-Plausibilität, `EricBearbeiteVorgang(ERIC_VALIDIERE)`) **synchron** im
Request-Pfad aufrufen, oder muss es in einen **persistenten Worker** ausgelagert werden?

Harness: `elster/bench/latency_checkest.py` (additiv, neu). Offline, `ERIC_VALIDIERE` ohne
`ERIC_SENDE`, keine Datei-Credentials. Hersteller-ID nur aus `$ELSTER_HERSTELLER_ID` (nur env,
nie im Code). Falsch-Grün-Sperre: jede Messung erzwingt `rc==0` (voller Plausibilitätspfad); ein
`rc!=0` (z.B. 610301202 GESPERRT) kurzschließt am HID-Gate **vor** dem Plugin-Load und würde eine
truncierte, zu kurze Latenz messen → Lauf bricht hart ab, wird nicht als „schnell" verbucht.

ERiC 44.2.4.0, `~/02_Software/eric`, VZ-Fälle `ESt_2025` (rc=0-Referenz) + `ESt_2020` (realistischer,
voller amtlicher Beispieldatensatz). Je 10 Wiederholungen. p95 = nearest-rank (bei n=10 ≈ Maximum;
Streuung klein, s.u.).

## Messwerte (Millisekunden)

| Fall | Datenart | cold_wall median / p95 | cold_eric median / p95 | t_init median | **warm median / p95** |
|---|---|---|---|---|---|
| minimal_2025 | ESt_2025 | 204,5 / 212,6 | 144,2 / 151,5 | 21,9 | **74,4 / 76,2** |
| realistisch_2020 | ESt_2020 | 203,4 / 222,2 | 143,0 / 161,8 | 22,0 | **68,7 / 74,6** |

Streuung eng: warm min–max ≈ 67–76ms, cold_wall min–max ≈ 195–222ms über beide Fälle. 10 Wdh. genügen.

**Zerlegung Kaltstart** (`cold_eric = t_init + t_first_validate`):
- `EricInitialisiere` (+ ctypes-CDLL-Laden): **~22ms**, konstant.
- **erste** `validate`: **~120ms** — lazy Plugin-Load von `libcheckESt` beim ersten
  `EricBearbeiteVorgang` je Datenart. Das ist der Kaltstart-Kostentreiber, nicht das Init.
- python-Interp-Start + import: `cold_wall − cold_eric` ≈ **~60ms**.

## Interpretation

1. **Kaltstart-Kosten sitzen im Plugin-Load, nicht im Init.** Wer pro Request einen frischen
   ERiC-Prozess forkt, zahlt jedes Mal ~130ms Plugin-Tax + ~60ms python-Start = **~205ms/Request**.
2. **Warm-Steady-State ~70ms** — das ist die reine Plausibilitätsprüfung, sobald das Plugin geladen
   ist. Faktor cold/warm ≈ **3×**.
3. **Payload-Größe irrelevant** in dieser Größenordnung: minimal (3,8KB) und realistisch (4,2KB)
   liegen warm bei 74 vs. 69ms — innerhalb des Rauschens. Die Latenz ist Fixkosten der Prüf-Engine,
   nicht der Datenmenge. (Große reale Erklärungen bleiben zu messen — s. Grenzen.)

## Verdikt — Entscheidungsvorlage sync/async

**Empfehlung: persistenter ERiC-Worker (warm gehalten), NIE Fork-per-Call.** Begründung:

- Fork-per-Call (~205ms p95) ist die naive Synchron-UI und der teuerste Weg — verwirft bei jedem
  Aufruf den geladenen Plugin-Zustand. Ausschließen.
- Warm gehalten kostet eine Validierung **~70ms (p95 76ms)**. Das ist niedrig genug, um das dritte
  Orakel **near-synchron auf diskreten Ereignissen** zu zeigen (Feld-Blur, Abschnitts-Commit,
  Validieren-Button) — <100ms fühlt sich unmittelbar an.
- Es ist zu langsam für den **Per-Tastendruck-Heißpfad**. Live-Validierung bei jedem Keystroke
  scheidet aus; ERiC gehört an Ereignisgrenzen, nicht in die Eingabe-Schleife.

**Konkret für die UI-Architektur (schärft Lab-P3 „drittes Orakel asynchron"):**
„Async" heißt hier **persistenter Prozess, nicht zwingend deferred/polling**. Ein warmer ERiC-Daemon,
der auf Blur/Section-Commit **synchron in ~70ms** antwortet, reicht — kein Long-Running-Job mit
Polling-UI nötig. Auslagern in einen Worker ist wegen (a) Plugin-Tax (nie neu laden) und (b)
ctypes/GIL-Isolation und (c) Prozess-Absturz-Eindämmung geboten, nicht wegen der Latenz selbst.

## Grenzen (nicht überinterpretieren)

- p95 aus n=10 (nearest-rank ≈ beobachtetes Maximum). Streuung klein → stabil, aber kein
  Tail-Verhalten unter Last gemessen.
- **Nebenläufigkeit ungemessen:** ein einzelner Daemon serialisiert (single-threaded ctypes-Aufruf).
  Bei M gleichzeitigen UI-Nutzern → Queue oder Worker-Pool. Skalierungsfrage, berührt sync/async-Wahl
  nicht, aber vor Mehrbenutzer-Betrieb zu klären.
- Nur kleine XML (~4KB). Realitäts-Obergrenze (volle Erklärung mit vielen Anlagen) noch offen —
  erwartbar höher, aber Fixkosten-dominiert; im P9-R3-Fuzzing (AUFTRAG 2) fallen größere Fälle an,
  dort mitmessbar.

## Reproduktion

```bash
ERIC_DIR=~/02_Software/eric ELSTER_HERSTELLER_ID=<id> \
    python3 elster/bench/latency_checkest.py --reps 10 --json out.json
```
Rohdaten (diese Messung): `scratchpad/latency.json`.
