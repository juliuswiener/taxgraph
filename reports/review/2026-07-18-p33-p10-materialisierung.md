# § 33 agB + § 10 Kirchensteuer Regel-Materialisierung — dev-2, 2026-07-18

**Status: gebaut + grün, freeze-ready.** Vollendet die bindung_sonder_agb_35a-Familie (gleiche Promotion
wie §35a/§10b). + 2 Pre-Check-Flags (nicht geraten).

## Materialisierte Module (aus verified_bedingt-Snapshots, kein Re-Run)
| Datei | Module | Snapshot | catala_a byte-gleich | typecheck | Seeds |
|---|---|---|---|---|---|
| `rules/estg/p33/agbabzug.catala_en` | AgbAbzug | p33_1_2_agb_abzug.json (sha b64650b1…) | ✓ True | ✓ | 3/3 |
| `rules/estg/p10/kirchensteuerabzug.catala_en` | Kirchensteuerabzug | p10_1_4_kirchensteuer.json (sha cc2c4928…) | ✓ True | ✓ | 3/3 |

- AgbAbzug: `abzug_agb = max(0; aussergewoehnliche_belastungen − zumutbare_belastung)`. Anker § 33 Abs. 1
  "zumutbare Belastung (Absatz 3) übersteigt, vom Gesamtbetrag der Einkünfte abgezogen" (voll-Länge verifiziert).
- Kirchensteuerabzug: `abziehbare_kirchensteuer = max(0; gezahlt − erstattet)`. Anker § 10 Abs. 1 Nr. 4
  "gezahlte Kirchensteuer" (voll-Länge verifiziert).
- Export: beide in clerk.toml (include_dirs rules/estg/p33 + p10, [[target]] p32a-python modules). Import
  verifiziert: `from pkg import AgbAbzug, Kirchensteuerabzug` → scope-fns `agb_abzug` / `kirchensteuerabzug`.
- golden 121/121 (NO-OP — noch keine Verrechnung angedockt), volle Suite 495 passed.
- Andock: §33 → p32a aussergewoehnliche_belastungen, §10 KiSt → p32a sonderausgaben (dev-1-Accessor).

## ⚠ PRE-CHECK-FLAGS

### FLAG 1 (§33 zumutbare_belastung): Regel EXISTIERT, aber unmaterialisiert
`zumutbare_belastung` ist AgbAbzug-INPUT. Die § 33 Abs. 3-Staffelung (1-7 % GdE nach Einkommen/Kinderzahl)
existiert als VERIFIZIERTER Snapshot: `p33_3_zumutbare_belastung.json`, **module ZumutbareBelastung,
verified_bedingt, faithful=True** — aber NOCH NICHT nach rules/estg/ materialisiert.
→ KEIN fehlende-Regel-Blocker: AgbAbzug ist als reine Teilregel (Input) baubar (erledigt). ABER die agB-Kachel
ist erst KOMPLETT, wenn ZumutbareBelastung AUCH materialisiert + upstream verdrahtet ist (sonst muss der
Accessor zumutbar raten). **EMPFEHLUNG: ZumutbareBelastung im selben Muster materialisieren (verified, trivial)
— soll ich das als Follow bauen?** (kein stiller Halb-Bau: hiermit explizit geflaggt.)

### FLAG 2 (§10 Kirchensteuer erstattungsueberhang): Hinzurechnung ist Nachtrag
Der Snapshot berechnet `internal erstattungsueberhang = max(0; erstattet − gezahlt)`, gibt aber NUR
`abziehbare_kirchensteuer` aus. § 10 Abs. 4b EStG: übersteigt die Erstattung die Zahlung, wird der Überhang
dem GdE HINZUGERECHNET. Dafür gibt es KEINE Regel und KEINEN p32a-Slot (p32a hat hinzurechnung_kindergeld/
zulage, aber keinen Kirchensteuer-Erstattungsüberhang-Slot).
→ Die §10-Abs.4b-Hinzurechnung ist ein benannter NACHTRAG (Regel + p32a-hinzurechnung-Slot). Kirchensteuerabzug
ist byte-gleich materialisiert (erstattungsueberhang bleibt internal/ungenutzt); der Überhang-Fall (erstattet >
gezahlt) liefert korrekt abziehbar=0, aber die Hinzurechnung fehlt. **FLAG: Nachtrag nötig, wenn Erstattungs-
überhänge steuerlich erfasst werden sollen.**

## Zur Freigabe
Instructor verifiziert Byte-Gleichheit + Seeds. Dann: (a) ZumutbareBelastung-Follow? (b) §10-Abs.4b-Nachtrag
priorisieren? — dein Call.
