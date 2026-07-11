# Abweichungs-Triage der 12 Items + K1 — 2026-07-11 Nacht

Instructor-Triage msg 1154, per Item-TEXT (die neuen Text-Anker aus Schritt 5
machen wortverschiedene Formulierungen derselben Sache zu separaten Items, alle
mit derselben Triage). Ablauf: frisches `discover_draft` je Regel (neue Anker aus
restauriertem report.json) → Text-Match-Triage (Skript mit Asserts: kein Item ohne
Entscheidung, alle neuen sind `art:abweichung`) → `aufnehmen` → Regate.

## K1 (Julius-Entscheid, Nacht-Delegation)

Neue Konvention in `signatur_konventionen.yaml`: `stunden_je_kalendertag`
(zeitbezug) — "Stundenangaben je Kalendertag liegen in 0..24; ein Kalendertag hat
hoechstens 24 Stunden, keine tagesuebergreifende Aggregation im Input."

## Triage (12 Items)

- **p24b** (4): alle → `nicht_material, konv:ganzzahl_monate` (negativer
  Entlastungsbetrag bei monate>12 liegt ausserhalb der Signatur-Domaene 0..12).
- **p9_4a** (7):
  - Uebernachtung-am-selben-Tag (4) + >8h ohne Uebernachtung (1) → `bedingung_neu,
    uebernachtung_oder_eintaegig_ueber_acht_stunden`.
  - 28 Euro bei >=24h (1) → `nicht_material, konv:stunden_je_kalendertag` (>=24 ≡ ==24 in 0..24).
  - Ausgabe in Dollar (1) → `nicht_material, konv:betraege_in_euro` (Catala-Money-Rendering).
- **p35a** (1): 20% auf Gesamtbetrag statt nur Arbeitskosten → `bedingung_neu,
  dienstleistungsbetrag_enthaelt_nur_arbeitskosten` (Input enthaelt nur Arbeitskosten).

## Registry-Delta + Gates (frisch)

| Regel | Version | Items | Status | equiv | rund_lint | clerk | 4 Registry |
|---|---|---|---|---|---|---|---|
| p24b_entlastungsbetrag | 1→2 | 16→20 | verified_bedingt | PASS | PASS | PASS | alle PASS |
| p9_4a_verpflegungsmehraufwand | 1→2 | 18→25 | verified_bedingt | PASS | PASS | PASS | alle PASS |
| p35a_2_3_haushaltsnahe | 1→2 | 22→23 | verified_bedingt | PASS | PASS | PASS | alle PASS |

12 Abweichungen registriert (0 offen). `--regate`: 3 Gate-Ergebnisse geaendert
(die drei discovery_triage → verified_bedingt), keine Modellkosten.

**Damit sind alle 7 aktiven Regeln `verified_bedingt`** — die 12 gesperrten
Abweichungen waren der einzige Rest.

## Statusgrenze

verified_bedingt vom Regate deterministisch berechnet, in die (gitignored)
report.json geschrieben. NICHT als final erklaert — Meldung an den Instructor vor
Bestaetigung/Julius-Freigabe (Widerrufsvorbehalt Morgen-Review). Nacht-Summe: $0.
