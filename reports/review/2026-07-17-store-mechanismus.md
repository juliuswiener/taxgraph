# Sachverhalts-Store — Schema + Mechanismus (Task #11, Paket A)

**Datum:** 2026-07-17 · **Autor:** dev-2 · **Status:** untracked, Commit über Instructor
**Zone:** `produkt/store/` (neu, additiv, LLM-frei im Kern). Julius-Entscheide #2/#4.

## Dateien

- `produkt/store/schema.json` — JSON-Schema 2020-12 (Auflagen A/C mechanisiert).
- `produkt/store/SCHEMA.md` — Design: Typ, Herkunfts-Vektor + Gitter, Meet-Semantik, Content-
  Adressierung, Zwei-Signal, ERiC-Bindung, Auflagen A–D, Worked Example.
- `produkt/store/store.py` — der Mechanismus: **der EINE Schreibpfad** `append_event` (fail-closed) +
  `materialisiere` + Content-Adressierung + Typ-Algebra (`meet_zustand`/`meet_herkunft`) + atomare
  Persistenz.
- `tests/test_store.py` — Gate, **19 Tests grün** (inkl. Negativtests + Meet-Algebra + Roundtrip).

## Was der Store leistet (Julius #2/#4)

- **Event-Log append-only + content-adressierte Snapshots** in einer Datei je Fall/VZ. Der Log ist die
  Wahrheit; ein Korrektur-Event trägt `ersetzt` und gewinnt bei der Materialisierung.
- **`Vorlaeufig<T>`/`Bestaetigt<T>` als echter Typ** (`zustand`): `meet_zustand` = fail-closed (Aggregat
  nur `bestaetigt`, wenn ALLE Eingaben `bestaetigt`) — der „Meet über den Input-Kegel".
- **Herkunfts-Vektor als Payload** (Herkunft × Prüftiefe × Haftung): `meet_herkunft` = Prüftiefe-Minimum
  (Kette) + Konflikt-Markierung bei Herkunfts-/Haftungs-Uneinigkeit (nie auto-versöhnt).
- **Content-Adressierung** (`sha256(canonical_json)`, wie `pipeline/snapshot.py`): jede Manipulation
  ändert den Hash. **YAML-Roundtrip-fest** verifiziert (save/load erhält event_id + snapshot_id).
- **ERiC-Befund bindet an `snapshot_id`** — eine Prüfung gilt nachweislich für EINEN Zustand.

## Auflagen A–D (umgesetzt + getestet)

- **A Schreiber↔Herkunft-Kopplung:** Schema-Conditional + harte Prüfung in `append_event` — ein
  `^llm:`-Schreiber MUSS `herkunft=llm_vorschlag`, `zustand=vorlaeufig`, `signal_2=null` tragen; jeder
  Versuch, über eine gefälschte Herkunft einen `bestaetigt`-Wert zu schmuggeln, wirft `ValueError`
  (Test `test_A_llm_schreiber_gekoppelt`).
- **B Ein aktives Event je feld_id:** `append_event` weist ein zweites Event auf dasselbe `feld_id`
  ohne gültiges `ersetzt` ab (Ziel muss existieren, selbes feld_id, nicht bereits ersetzt) — Tests
  `test_B_*`. Verhindert stille Materialisierungs-Divergenz.
- **C `eric_befund.gekappt_verdacht` (Pflicht):** Trunkierungs-Sperre bis in den Store. Downstream-Regel
  `_ist_gruen`: `plausibel` MIT `gekappt_verdacht=true` ist NICHT grün (Test
  `test_C_plausibel_aber_gekappt_ist_nicht_gruen`).
- **D Typ-Konformität:** `wert` wird gegen den Bindungstabellen-`typ` geprüft (cent/int→Ganzzahl,
  bool→boolean, enum→∈`enum_werte`, datum→ISO) — Test `test_D_typ_konformitaet`. Bindet Store an
  Bindungstabelle (Test `test_e_feld_id_in_bindung`).

## Falsch-Grün-Sperre (Gate rot-fähig)

Negativtests: manipuliertes Event → Content-Adresse bricht; ERiC an falschem Hash; `bestaetigt` ohne
`signal_2`; `llm`+`bestaetigt`. Meet-Algebra separat getestet (ein `vorlaeufig` → Aggregat
`vorlaeufig`). Beide Gates zusammen (Store + Bindungstabelle): **30/30 grün.**

## Verhältnis zur Bindungstabelle

Die Bindungstabelle definiert, WAS gefragt werden darf (feld_id → typ/anker/kz); der Store hält, was
geantwortet wurde (feld_id → wert + zustand + herkunft). Der Gate hält beide konsistent (feld_id-Existenz
+ Typ-Konformität). Zusammen: das „Store ist Wahrheit + jede Kante trägt Herkunft"-Paar aus dem Lab.

## Offener Produkt-Punkt (kein Blocker)

- `[min,max]`-Unsicherheits-Intervall (Julius #6) ist ein DERIVAT über dem Store (Engine-Rerun mit/ohne
  vorlaeufige Werte), bewusst KEIN Store-Feld. Nächster Paket-A-Baustein-Kandidat.

## Reproduktion

```bash
python3 -m pytest tests/test_store.py -q
```
