# runs/-Restore + frisches Regate — 2026-07-11 (Nachtsession, Schritt 2)

## Restore

Quelle: `arch-think:/home/julius/00_projects/165_TaxGraph/taxgraph/pipeline/runs/produktion/`
(Checkout heißt `165_TaxGraph`, nicht `168`; passwordless ssh). rsync -a, exit 0,
898 KB, 9 report.json gespiegelt nach `pipeline/runs/produktion/` (gitignored, lokal).

## Integritätscheck

Alle 7 aktiven Regeln: `catala_a`, `catala_b`, `judge_verdict` befüllt, module_name gesetzt.
Zusätzlich zwei zuschnitt_offen-Regeln: p9_1_3_nr7_afa (vollständig), p9_1_3_nr5a_uebernachtung
(catala_b fehlt — erwartet, Zuschnitt offen).

## Frisches Regate (mit Toolchain, $0)

`run.py --regate`: Quellen-Gate ok (7 Regeln), 13 Gate-Ergebnisse geändert, keine Modellkosten.

| Regel | queue_status | equivalence | rundungs_lint | geltungsb. | roundtrip | grenzfall | defekt |
|---|---|---|---|---|---|---|---|
| p10_1_7_berufsausbildung | verified_bedingt | PASS | PASS | PASS | PASS | PASS | PASS |
| p9_6_erstausbildung_abgrenzung | verified_bedingt | PASS | PASS | PASS | PASS | PASS | PASS |
| p9_1_3_nr5_doppelte_haushaltsfuehrung | verified_bedingt | PASS | PASS | PASS | PASS | PASS | PASS |
| p33_3_zumutbare_belastung | verified_bedingt | PASS | PASS | PASS | PASS | PASS | PASS |
| p24b_entlastungsbetrag | discovery_triage | PASS | PASS | PASS | PASS | PASS | PASS |
| p9_4a_verpflegungsmehraufwand | discovery_triage | PASS | PASS | PASS | PASS | PASS | PASS |
| p35a_2_3_haushaltsnahe | discovery_triage | PASS | PASS | PASS | PASS | PASS | PASS |

**verified_bedingt vs discovery_triage**: die drei discovery_triage-Regeln
(p24b/p9_4a/p35a) sind **exakt** die drei mit gesperrten Abweichungs-Items (4/7/1).
Die stehen als un-triagierte Discovery an → Status bleibt discovery_triage bis
Anker-Fix + Abweichungs-Triage. Die vier ohne Abweichungen → verified_bedingt.
Konsistent, kein Falschgrün.

**Vorbehalt 3 aufgelöst**: `rundungs_lint = PASS` für alle 7 → die als
`nicht_material, konv:keine_zusaetzliche_rundung` geseedeten Rundungs-Items sind
legitim. `equivalence = PASS` → A==B auf dem Raster für alle 7.

Zuschnitt_offen (außerhalb Scope): nr5a flagged_for_review (equivalence FAIL, kein
catala_b), nr7 flagged_for_review (geltungsbereich/roundtrip FAIL, leere Registry).

## Statusgrenze / Bestätigung

Das Regate hat verified_bedingt/discovery_triage **deterministisch berechnet** und in
die (gitignored) report.json geschrieben.

**BESTÄTIGT verified_bedingt** (Instructor-Verifikation msg 1140, Nacht-Delegation,
`bestaetigt_von: instructor, 2026-07-11 Nacht`, Widerrufsvorbehalt Julius' Morgen-Review):
`p10_1_7_berufsausbildung`, `p9_6_erstausbildung_abgrenzung`,
`p9_1_3_nr5_doppelte_haushaltsfuehrung`, `p33_3_zumutbare_belastung`. Begründung:
alle registrierten Items abgedeckt, alle deterministischen Gates grün, doppelt
unabhängig gerechnet (dev + instructor), Status folgt rein der Registry — kein
Judge-Wurf beteiligt.

`p24b`, `p9_4a`, `p35a` bleiben `discovery_triage` — die 12 gesperrten Abweichungs-Items
sind der einzige Rest; nach Anker-Fix (Schritt 4/5) + Abweichungs-Nachtriage erreichbar.

Nacht-Summe: $0.
