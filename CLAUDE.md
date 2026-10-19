# TaxGraph — German tax rules in Catala, with a Python pipeline around them

## Commands

```
make rust-test          cargo build, clippy -D warnings, nextest, doctests (das Tor)
make tests              clerk test -W rules/          the Catala scope tests
make all                tests
make golden             die 135 Golden-Fälle in Rust (golden_werte, golden_kopf, anker)
make serve              Rust-Dienst (das Produkt seit dem Cutover 2026-10-04), sichert den Bestand vorher
```

`make serve` baut im dev-Profil, nie mit `--release` (dort fehlen `overflow-checks`;
`rust/api/tests/ueberlauf_profil_waechter.rs` hält `overflow-checks = true` im `[profile.dev]` fest). Das Ziel läuft gegen den echten Bestand
`~/.local/share/taxgraph`: Tests und Messungen setzen `TAXGRAPH_DATEN` auf eine Kopie oder nutzen
`scripts/starte-api.sh`, nie `make serve`. Einen Rückfall auf Python gibt es seit L3 nicht mehr; der letzte Stand mit
Python als Standard-Start ist der Tag `python-standard-letzter` (`904f6215`, auf origin). `rust/parity` ist seit L1, `tests/` und `tools/ui_rust` sind seit L2, `produkt/` ist seit L3 gelöscht; `pipeline/`, `oracle/` und die Python-Skripte außer `scripts/freeze_source.py`, `scripts/freeze_pdf_local.py`, `scripts/verify_sources.py` fielen mit L4 (2026-10-19).
Die Löschung ist seit 2026-10-06 beschlossen (nach bestandener Abnahme) und vollzogen bis L5 (nur lokale Reste, L6, stehen aus); Reihenfolge und Vorbedingungen stehen im Vault
(`audits/python-loeschplan-2026-10-06.md`) und im `REWRITE_PLAN.md` (Nachtrag 2026-10-06). Neue Arbeit entsteht nur in Rust, in Python wird nichts mehr
gepflegt. Der echte ELSTER-Versand läuft in Rust (`rust/versand`, noch nie echt gesendet; Einreichen löst nur Julius aus). Nach einem erfolgreichen Backup bleiben in `BACKUP_DIR`
die letzten `BACKUP_BEHALTEN` (30) Archive; `AUTH_USERS` folgt `TAXGRAPH_USER_STORE` nicht von selbst.

`make tests` runs under `OPAM_ENV` and `make params-check` under `VENV312` — call those targets,
not their commands directly, or the Catala toolchain and the 3.12 venv are missing.

A single test: `cargo nextest run -p <crate> <name>` (in `rust/`).

**Neue Testdatei braucht eine Zeile in `rust/TESTMAP.tsv`, sonst ist `make rust-test` rot.** Das gilt für jede Datei
in `rust/*/tests/` (`.rs`, `.py`, `.c`). Der Test `rust/bindung/tests/testmap_vollstaendig.rs`
nennt die fehlenden Dateien und die Zeilen ohne Datei; Aufruf: `cargo test -p bindung --test it testmap_vollstaendig::`. Eine Zeile hat 7 Spalten, durch
Tab getrennt (`file`, `lines_bei_anlage`, `n_tests_bei_anlage`, `xfail`, `category`, `target_module`, Notiz), und Spalte 7 nennt nur
Ersatz, der im Baum existiert. Spalte 2 und 3 halten den Stand beim Anlegen der Zeile; niemand zieht sie nach (kein Tor prüft sie). Wer einen Zweig mergt, ruft den Test nach dem Merge auf: ein Zweig, der älter ist als
der Test, bringt Testdateien ohne Zeile mit (so fielen `rentenbeginn_jahr.rs` und `fallzahl_env.rs` beim ersten
Lauf auf). Bei einem Konflikt in der TSV bleiben
beide Zeilen stehen. Kein Test prüft die Reihenfolge.

**Neue Rust-Testdatei in einem Crate mit `tests/it.rs` gehört dort hinein** (`#[path = "x.rs"] mod x;`), nicht als eigene
Datei daneben: `autotests = false` übersetzt nur Dateien, die `it.rs` oder ein `[[test]]` nennt; die Tests einer anderen laufen still nicht mehr. Der Wächter
`rust/bindung/tests/testbinaries_vollstaendig.rs` prüft das. Eigenes `[[test]]`-Ziel nur für Dateien, die `set_var`,
`remove_var` oder `set_current_dir` aufrufen oder per `--test <name>` von Makefile, Skript oder Wächter gerufen werden.
Aufruf eines Moduls: `cargo test -p <crate> --test it <modul>::`.

## graphify — code graph

The repo has a graph under `graphify-out/` (23,752 nodes, 46,638 edges). Trigram hits on
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

**`--context call` for structural questions only.** 19,273 of 46,638 edges carry no
context at all; the filter discards them, including the `rationale_for` edges that carry
the domain half. On "who writes events to the store" it takes 982 hits down to 14 and
the answer becomes complete; on a domain question it gains 6% and risks the answer.

**A known identifier belongs to `Grep`.** The graph knows files and symbols, not every
identifier inside them — a field name from a YAML never becomes a node.
`graphify query "stammdaten_keine_bankverbindung"` answers "No matching nodes" while the
name appears 33 times in the repo.

**Not indexed:** `sources/gesetze-im-internet/` (32 files), and one `.sql` file for want
of `tree_sitter_sql` (`pip install "graphifyy[sql]"`).
