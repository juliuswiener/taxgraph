# Unsicherheits-Derivat — [min,max]-Bescheid + Beitrag je Feld (Task #11, Julius #6)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Zone:** `produkt/unsicherheit/` (neu, additiv, **NULL LLM**, rein deterministisch).

## Dateien

- `produkt/unsicherheit/KONZEPT.md` — abgenommene Konzept-Skizze (+ Ranking-Heuristik-/Nicht-fixierbar-Notiz).
- `produkt/unsicherheit/intervall.py` — der Mechanismus: `intervall(snapshot, bindung, bescheid_fn)` +
  Engine-Adapter `bescheid_via_slots`.
- `produkt/bindung/{schema.json,SCHEMA.md,bindung_n_vor_gwg.yaml}` — `bereich`-Nachzug (Bindungstabelle).
- `tests/test_unsicherheit.py` — Gate, **10 Tests grün** (inkl. Real-Engine + Negativtests).

## Was das Derivat leistet (Julius #6)

`intervall()` liest einen Store-Snapshot und liefert:
- **`[min,max]`-Bescheid-Intervall** über alle unsicheren (offen ∨ vorlaeufig) askable Felder;
  bestätigte Felder sind fix. Verengt sich beim Bestätigen (Test `test_bestaetigen_verengt_spanne`).
- **Beitrag je Feld** (One-at-a-time-Spanne), absteigend sortiert → Frage-Reihenfolge / Steuer-at-Risk.

Zweistufig: (1) One-at-a-time für die Beiträge (Ranking-Heuristik, kann Interaktionen unterschätzen —
das Intervall bleibt via `gedeckelt`-Flag ehrlich); (2) gedeckelter Kartesischer über die Top-K-Treiber
(Cap 256) für das exakte Intervall bzgl. der stärksten Felder.

## Ehrlichkeit (Falsch-Grün-Analog)

- **Offene Achse** (unbounded cent/int mit Vorschlag): Intervall auf beiden Seiten offen markiert
  (`min_offen`/`max_offen`) — die wahre Spanne reicht darüber hinaus (Test `test_offene_achse_...`).
- **Nicht fixierbar** (unbounded, kein Vorschlag): kein numerischer Bescheid, beide Seiten offen, KEIN
  Ersatzwert erfunden, Feld gelistet (Test `test_nicht_fixierbar_kein_ersatzwert`).
- **Gedeckelt**: übersteigt der Kartesische Raum den Cap, wird das Intervall exakt bzgl. der Top-K
  gemeldet + `gedeckelt=true` + `rest_felder` — nie als „volles exaktes Intervall" (Test
  `test_gedeckelt_flag_bei_kleinem_cap`).

## Engine-Bindung

Engine-agnostisch: `bescheid_fn(feld_werte) → steuer` ist injiziert. Produktions-Adapter
`bescheid_via_slots(bindung, slot_fn)` übersetzt `feld_id → signatur_slot` über die Bindungstabelle und
addiert Summanden-Slots (Summen-Konvention). **Real-Engine end-to-end bewiesen:** der Gate-Test
`test_real_engine_entfernungspauschale` rechnet über `golden/runner.catala_entfernungspauschale`
(echtes Catala) und erhält für `arbeitstage 0..366` ein reales `[0, max]`-Intervall — **lief, nicht
geskippt** (Toolchain verfügbar). Fehlt die Toolchain (opam-Env/`_catala`), skippt der Test sauber mit
Meldung (gettsim-Muster, nie stilles Grün).

## `bereich`-Nachzug (Bindungstabelle)

Neues optionales Feld `bereich: {min, max, grund?}` (nur cent/int). Gate `test_f_bereich`:
min≤max, ganzzahlig, negativer cent nur mit `grund` (Verluste). Gesetzt für die Kalender-int-Felder
(`ep_arbeitstage 0..366`, `dhf_monate 0..12`, `vpf_abwesenheit_stunden 0..24`); Geldfelder bleiben
bewusst unbounded → offene Achse (ehrlich, keine erfundene Obergrenze). Bindungstabelle jetzt 13 Tests.

## Gesamtstand Paket A (UI-Kern-Unterbau)

Bindungstabelle (was gefragt werden darf) + Store (was geantwortet wurde) + Unsicherheits-Derivat
(wie sicher der Bescheid ist) — **zusammen 42 Gate-Tests grün** (13 + 19 + 10). Alle drei LLM-frei,
deterministisch, source-/typ-verankert.

## Offener Baustein (kein Blocker)

Geldfeld-`bereich` aus params (z.B. VOR-Beiträge ≤ Höchstbeitrag) würde weitere Achsen schließen —
Enrichment-Nachtrag, sobald priorisiert.

## Reproduktion

```bash
python3 -m pytest tests/test_unsicherheit.py -q          # inkl. Real-Engine (skippt ohne Toolchain)
```
