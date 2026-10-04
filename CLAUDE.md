# TaxGraph — German tax rules in Catala, with a Python pipeline around them

## Commands

```
make unit               python3 -m pytest tests/ -q -n 6 --dist loadfile
make tests              clerk test -W rules/          the Catala scope tests
make all                unit + tests + s02
make golden             golden/golden_lauf.py          full-return regression
make snapshot-verify    pipeline/snapshot.py verify --all
make serve              Rust-Dienst (das Produkt seit dem Cutover 2026-10-04), sichert den Bestand vorher
make serve-python       Python-Dienst: Rückfall, Referenz und Orakel
```

`make serve` baut im dev-Profil, nie mit `--release` (dort fehlen `overflow-checks`; nur der dev-Bau wird
von `rust/parity` und `make ui-rust` gemessen). Beide Ziele laufen gegen den echten Bestand
`~/.local/share/taxgraph`: Tests und Messungen setzen `TAXGRAPH_DATEN` auf eine Kopie oder nutzen
`scripts/starte-api.sh`, nie `make serve`. Rückfall auf Python: `make serve-python`; der letzte Stand mit
Python als Standard-Start ist der lokale Tag `<TAG>` (Platzhalter). Python bleibt Orakel der `rust/parity`-Suiten
und wird nicht gelöscht (`REWRITE_PLAN.md`, Absatz „Cutover vollzogen“). `BACKUP_DIR` wächst je Start;
`AUTH_USERS` folgt `TAXGRAPH_USER_STORE` nicht von selbst.

`make unit` runs plain `python3 -m pytest`, without `OPAM_ENV`/`VENV312` from the Makefile.
`make tests` runs under `OPAM_ENV` and `make golden` under `VENV312` — call those targets,
not their commands directly, or the Catala toolchain and the 3.12 venv are missing.

**`make unit` hängt nicht von `build-python` ab.** In einem frisch angelegten Worktree fehlen
`_build/` und `oracle/gettsim/_catala/` (beide gitignored). Der Skip-Guard in
`tests/conftest.py` greift nur bei direkten `runner`/`pkg`-Importen; die Testdateien, die
`api`/`server` importieren, laufen trotzdem und scheitern an `engine_unavailable`. Ergebnis:
129 failed + 2 errors, und der Lauf bleibt rot (kein Skip: ein grüner Lauf ohne Engine hätte
nichts gemessen). Er nennt seine Ursache selbst in der Schlusszeile nach der Fehlerliste
(`Catala-Engine fehlt in diesem Baum`). Vor jedem Tor in einem neuen Worktree einmal
`make build-python` laufen lassen. Gemessen 2026-10-03 auf `b540590`.

A single test: `python3 -m pytest tests/path/to/test_x.py::test_name -q`.

**Neue Testdatei braucht eine Zeile in `rust/TESTMAP.tsv`, sonst ist `make unit` rot.** Das gilt für jede Datei
in `tests/` und jede `.rs`/`.py`/`.c` in `rust/*/tests/`. Der Test `tests/test_testmap_vollstaendig.py` nennt die
fehlenden Dateien; dasselbe zeigt `python3 tests/test_testmap_vollstaendig.py`. Eine Zeile hat 7 Spalten, durch
Tab getrennt (`file`, `lines`, `n_tests`, `xfail`, `category`, `target_module`, Notiz), und Spalte 7 nennt nur
Ersatz, der im Baum existiert. Wer einen Zweig mergt, ruft den Test nach dem Merge auf: ein Zweig, der älter ist als
der Test, bringt Testdateien ohne Zeile mit (so fielen `rentenbeginn_jahr.rs` und `fallzahl_env.rs` beim ersten
Lauf auf). Bei einem Konflikt in der TSV bleiben
beide Zeilen stehen; die Python-Zeilen sind nach Pfad sortiert (ohne Satzzeichen), die Rust-Zeilen folgen. Kein
Test prüft die Reihenfolge.

## graphify — code graph

The repo has a graph under `graphify-out/` (12,463 nodes, 18,058 edges). Trigram hits on
the query terms pick the start nodes, then BFS walks the edges — strong on bundles of
terms, weak on a single known identifier.

| Question | Command |
|---|---|
| domain question, surroundings of a term | `graphify query "…" --budget 6000` |
| "who calls / writes X" | `graphify query "…" --context call` |
| what breaks if I change X | `graphify affected "X" --depth 2` |
| first look, architecture | `graphify god-nodes --top 15` |
| one node and its neighbours | `graphify explain "X"` |
| after changing code | `graphify update .` (AST-only, no API cost) |

**`--budget` is not optional.** The default of 2,000 tokens truncates 70% of answers —
median 58 nodes shown of 157, with no indication of what is missing. A truncated answer
looks exactly like a complete one. Not to be confused with `--token-budget`, which
applies to `extract` and sets LLM chunk size, never answer size.

**`--context call` for structural questions only.** 10,544 of 17,091 edges carry no
context at all; the filter discards them, including the `rationale_for` edges that carry
the domain half. On "who writes events to the store" it takes 982 hits down to 14 and
the answer becomes complete; on a domain question it gains 6% and risks the answer.

**A known identifier belongs to `Grep`.** The graph knows files and symbols, not every
identifier inside them — a field name from a YAML never becomes a node.
`graphify query "stammdaten_keine_bankverbindung"` answers "No matching nodes" while the
name appears 33 times in the repo.

**Not indexed:** `sources/gesetze-im-internet/` (32 files), and one `.sql` file for want
of `tree_sitter_sql` (`pip install "graphifyy[sql]"`).
