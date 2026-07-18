# § 35a / § 10b Regel-Materialisierung + § 35a-Floor-Fix — dev-2, 2026-07-18

**Status: gebaut + grün, freeze-ready.** Materialisierung der verifizierten Snapshots + K2-Floor-Fix.
Instructor verifiziert Byte-Gleichheit + Seeds vor Commit.

## Materialisierte Module (aus verified_bedingt-Snapshots, kein Pipeline-Re-Run)
| Datei | Module | Snapshot | catala_a byte-gleich | Seeds |
|---|---|---|---|---|
| `rules/estg/p35a/haushaltsnahe.catala_en` | Haushaltsnahe | p35a_2_3_haushaltsnahe.json (sha 7d084e5a…) | ✓ True | 5/5 (clerk test) |
| `rules/estg/p10b/spendenabzug.catala_en` | SpendenAbzug | p10b_spenden.json (sha e352aeeb…) | ✓ True | 3/3 (clerk test) |

- Die ```catala```-Blöcke sind BYTE-GLEICH aus `catala_a` (python-verifiziert); nur Module-Header +
  §-Zitatanker (estg_p35a/estg_p10b) + Grenzfall-Seeds ergänzt.
- ⚠ DATEINAME-ABWEICHUNG: p10b heißt `spendenabzug.catala_en` (nicht `spenden.catala_en` wie im Auftrag):
  clerk erzwingt Dateiname = Modulname; das Snapshot-Modul heißt `SpendenAbzug` (byte-gleich behalten) →
  Datei muss `spendenabzug` heißen (case-insensitiv wie haushaltsnahe↔Haushaltsnahe). Modul bleibt faithful.
- clerk typecheck: beide + p32a erfolgreich.

## § 35a-Floor-Fix (K2-Befund Instructor: ungefloorte steuerermaessigungen-Subtraktion → negativ)
`rules/estg/p32a/einkommensteuertarif.catala_en`, BEIDE Scopes (FestzusetzendeEstGesamt + …Zusammen):
```
wirksame_ermaessigung = min(steuerermaessigungen; max(0; tarifliche_est − anzurechnende_auslaendische_steuern))
festzusetzende_est    = tarifliche_est − anzurechnende_auslaendische_steuern − wirksame_ermaessigung + …
```
(als `internal wirksame_ermaessigung` + `if … then … else` da der Code min/max via if-then-else ausdrückt.)
- § 35a/§ 35c/§ 34g nicht erstattungsfähig → Ermäßigung auf verfügbare Steuer gefloort, Überhang verfällt.
- NO-OP für die 120 Bestandsgoldens (steuerermaessigungen=0 → min(0;…)=0). Faithfulness-VERBESSERUNG.
- Anker § 35a Abs. 1-3 "ermäßigt sich die tarifliche Einkommensteuer, vermindert um die sonstigen
  Steuerermäßigungen, auf Antrag" (estg_p35a_2026-07-09.txt) + § 2 Abs. 6.

## Belege
- Golden 121/121 (neuer `gesamt_2026_einzel_ermaessigung_floor`: § 19 20000, steuerermaessigungen 50000 →
  festzusetzende_est **0**, nicht negativ; Bestandscase abzuege_ermaessigung 13000 unverändert). EXIT 0.
- Volle pytest-Suite 491 passed / 2 skipped gegen die frisch re-assemblierte gefloorte Engine.
- ⚠ BUILD-LEHRE: `assemble_catala.sh` KOPIERT nur aus `_build`; es kompiliert NICHT. Sequenz für einen
  rules/-Edit im Golden: `clerk build p32a-python` → `bash oracle/gettsim/assemble_catala.sh` → `golden/runner.py`.
  (Ohne den clerk-build-Schritt läuft der Golden gegen die STALE Engine — Floor-Case erst nach Rebuild grün.)

## Naht offen (dev-1)
Die Module sind reine Teilregeln (Haushaltsnahe.steuerermaessigung / SpendenAbzug.spenden_abzug). Die
Verrechnung: dev-1s Accessor ruft sie + speist den Roh-Wert in p32a `steuerermaessigungen` (§35a) /
`sonderausgaben` (§10b, additiv). Damit der Accessor sie importieren kann, müssen Haushaltsnahe + SpendenAbzug
in die assemblierte Python-Package (clerk.toml [[target]] p32a-python modules-Liste) — Naht mit dev-1 geklärt.
