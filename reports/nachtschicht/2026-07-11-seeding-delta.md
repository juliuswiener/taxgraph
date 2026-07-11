# Seeding-Delta-Report — 7 Regeln (2026-07-11, Nachtsession)

Autoritative Triage: Gesamtauftrag msg 1126 + Julius-Entscheide msg 1130 (K2).
Deterministisch, key-getrieben (`_key = art+anker`), Skript mit harten Asserts:
kein offenes Item ohne Entscheidung, jede Key-Kollision explizit aufgelöst, jede
`bedingung_neu`-Bedingung im Manifest deklariert. ABWEICHUNGS-SPERRE aktiv.

## Registry-Delta (Version, Items, vier Registry-Gates)

| Regel | Version | Items | geltungsbereich | roundtrip | grenzfall | defekt |
|---|---|---|---|---|---|---|
| p24b_entlastungsbetrag | 0→1 | 0→16 | PASS | PASS | PASS | PASS |
| p10_1_7_berufsausbildung | 0→1 | 0→9 | PASS | PASS | PASS | PASS |
| p9_6_erstausbildung_abgrenzung | 0→1 | 0→12 | PASS | PASS | PASS | PASS |
| p9_1_3_nr5_doppelte_haushaltsfuehrung | 0→1 | 0→14 | PASS | PASS | PASS | PASS |
| p9_4a_verpflegungsmehraufwand | 0→1 | 0→18 | PASS | PASS | PASS | PASS |
| p35a_2_3_haushaltsnahe | 0→1 | 0→22 | PASS | PASS | PASS | PASS |
| p33_3_zumutbare_belastung | 2→3 | 7→15 | PASS | PASS | PASS | PASS |

Gates frisch aus `item_registry.py` (reg+manifest, runs/-unabhängig). Monotone
Aufnahme, kein Clobber (p33 v2→v3, +8 Items neben den bestehenden 7).

## Gesperrte Abweichungs-Items (offen, warten auf Anker-Fix Punkt 5)

p24b 4, p35a 1, p9_4a 7 = **12** (Schlüssel je Regel degeneriert `[betrifft,"?"]`).
0 `art:abweichung` in irgendeiner Registry — Sperre verifiziert.

## Toolchain-Verdikte (frisch)

- `clerk test -W rules/`: 24/24 tests, 6/6 files, exit 0.
- `make unit` (pytest): 69/69 (keine Regression aus Registry-/Konventionen-Edits).
- `make s02` (GETTSIM-Differential): exit 0. Divergenzen = Baseline
  (`reports/s02-divergenzen.md` unverändert vs HEAD; §32a-Splitting, vorbestehend,
  seeding-unabhängig — ich fasste Tarif/params nicht an).

## Weitere Änderungen

- `signatur_konventionen.yaml`: `nichtnegative_betraege`-Beschreibung um
  Zählgrößen (Kinder/Monate/Tage) erweitert (Julius-Entscheid K2, msg 1130).
- `rules.yaml` p9_4a: die zwei quelle-Label-Fixes (Satz 12→11; Satz 3 Nr. 2 und
  Satz 5 → Nr. 2 und Nr. 3) waren zwischenzeitlich revertiert (user/linter,
  Datei = HEAD) und wurden erneut angewandt. **Persistenz braucht Commit.**

## Statusgrenze

KEIN Statuswechsel auf verified/verified_bedingt. Die vier Registry-Gates sind
notwendig, nicht hinreichend — equivalence/rundungs_lint/clerk-Compile hängen an
`runs/report.json` (catala_a/b) und kommen erst mit dem arch-think-Restore. Kosten
diese Runde: $0 (kein Modell-Lauf). Laufende Nacht-Summe: $0.
