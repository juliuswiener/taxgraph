# Charge 2 Stufe B — Delta-Report + Morgen-Paket — 2026-07-11 Nacht

## Endstand beide Regeln (final, ehrlich blockiert)

| Regel | geltungsb | roundtrip | grenzfall | defekt | equiv | clerk | Status |
|---|---|---|---|---|---|---|---|
| p9_1_3_nr6_7_arbeitsmittel_afa | PASS | PASS | PASS | **FAIL** | FAIL | FAIL | flagged_for_review |
| p9_1_3_nr5a_uebernachtung | PASS | PASS | PASS | **FAIL** | PASS | FAIL | flagged_for_review |

Registry-Gates (geltungsbereich/roundtrip/grenzfall) beider Regeln **grün** — die
Triage ist vollständig. Beide bleiben durch `defekt_gate` + `clerk` ehrlich rot,
kein Falschgrün. Registry-Versionen: nr6_7 v2 (20 Items), nr5a v1 (5 Items).

## Registrierte defekt_formalisierer (blocken bis A-Neulauf)

- **nr6_7**: (1) Letztjahr-AfA-Rest (j=3) nicht modelliert; (2) year0-Zwölftelung
  im aktuellen 3/6-A regressiert (Anschaffungsmonat zählt fälschlich nicht mit).
  Beide gekoppelt an denselben A-Neulauf.
- **nr5a**: 48-Monats-Gate fehlt — A UND B kappen die 1.000-€-Grenze UNBEDINGT
  (per-Seed-Probe: gekappt bei jedem monate_bisher_am_ort ∈ [0..48]). Der
  `{47→16.800}`-Seed ist der deterministische Wächter für den Neulauf.

## Triage-Details

- nr6_7 Judge-Abweichungen (year0-Formel) → nicht_echt (Falschpositive, A's
  (12−(M−1))/12 = (13−M)/12 korrekt, per-Seed bestätigt). §7-Zwölftel-norm_teil →
  nicht_echt (Kernmechanismus). maßgebliche_ak/gwg → bedingung_neu. Rest → konv.
- nr5a: monate_bisher_am_ort|sonstige → defekt (Detektor-keine_unterbrechung war
  Fehl-Mapping); uebernachtungskosten_monat → nicht_material; Nr.5a Satz 1 → nicht_echt.
- **Geflaggt an Instructor (kein Fuzzy)**: nr6_7 keine mehr (die 2 adjudiziert);
  nr5a zwei `monate|interpretation`-Items ("monate als Multiplikator") — passen in
  keine Textklasse.

## Manifest-Design bleibt (Instructor msg 1187)

nr5a: Semantik-Pin `monate_bisher_am_ort` (= abgelaufene Monate), Seeds {47}/{48},
Bedingung `zeitraum_ohne_schwellenuebertritt` sind korrektes Design — NICHT
zurückbauen. `{47}`-Seed = Wächter.

## Kosten

Nacht-Summe (OpenRouter key-usage): **~$0,50 / 10 USD**. Aufschlüsselung: Judge-Hang-
Infra-Schleife ~$0,11, Clean-Kaskaden + Redos (nr6_7 3×, nr5a 4× inkl. 1 transient)
~$0,39. Alle lokalen Regates/Probes/Triage: $0.

## MORGEN-PAKET (Julius)

1. **Formalisierer-Besetzung: Re-Evaluations-Trigger ausgelöst.** Dritter Schwäche-
   Fall in Folge: p33-Rundung → nr6_7 (Letztjahr-Rest + year0-Regress) → nr5a
   (48-Gate) — und bei nr5a **A UND B** (nicht nur A). Der models.yaml-Trigger
   (rollende Eskalation / Human-Review-Miss) ist damit klar berührt. Julius-Entscheid:
   andere Modelle, andere Provider-Konfiguration, oder Zuschnitts-Feedback in die
   Kaskade. Kein dev/instructor-Alleingang (Besetzung = Julius).
2. **Instructor-(b)-Fehlentscheidung offen dokumentiert.** Der Instructor entschied
   zunächst (b) "kein defekt, nur Seed-Korrektur" unter der Prämisse "A kappt ab 48
   korrekt". Die Per-Seed-Probe (dev) falsifizierte das: A kappt unbedingt ab 0. Der
   Instructor zog die Konsequenz und entschied (a) defekt. Kultur: Fehlentscheidung
   + Falsifikation sichtbar, nicht geglättet.
