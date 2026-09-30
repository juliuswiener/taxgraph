# REWRITE_PLAN — produkt/ nach Rust

Stand 2026-09-28, gemessen gegen HEAD `607aedb` plus fremdes, uncommittetes WIP in 12 Dateien
(`runner.py`, `api.py`, `store.py`, `bescheid_*.py`, `traverser.py`, `est_mapping.py`, zwei Tests).
Evidenz: vier Audit-Berichte (Kern · API/ELSTER/YAML · Pipeline/Catala · Testklassifikation), deren
Kernaussagen unten mit `datei:zeile` stehen. Grade nach CLAUDE.md: **explicit** = gelesen oder gelaufen,
**derived** = daraus berechnet, **conditional** = Annahme.

---

## Fortschritt (Stand 2026-10-01, HEAD `2f9cd2d`)

| Schritt | Stand | Commits |
|---|---|---|
| 1–6, 8 | fertig: catala-sys, domain, bindung, store, engine, interview, konsistenz, intervall, elster, auth, llm, eingang | bis `f165889` |
| 7 `bescheid` | fertig | `75d8955` abzuege/einkuenfte · `5c6418b` deklaration · `da0cc0d` zweige + `bescheid_fn` |
| 9a `api`-Gerüst | fertig: 24 Routen (9 fertig, 15 Stubs `501 nicht_portiert`), `EigenerFall`, HTTP-Differenz-Harness | `054a281` |
| 9b-A Härtung additiv | fertig für bescheid, engine (Doctests, `debug_assert!`, Properties, kein `f64` für Geld); Doctests in catala-sys, eingang, elster, llm, interview | `b149f1f` · `9e2b00d` · `b047e7c` |
| 9b-F Fuzz | fertig: `rust/fuzz` (nightly, nicht im Workspace), 5 Targets | `d2e1e15` |
| Format | `cargo fmt --all`, `llm_dialog` geteilt | `2f9cd2d` |
| **9b-B Typisierung** | **als nächstes**: K0 → K9 (§7) | — |
| 9b Rest | `debug_assert!`/Properties für übrige Crates; 22 xfail als `#[ignore]`; End-to-End Eingabe → XML | — |
| 9c `api`-Handler | offen (danach), inkl. `Username`/`FallId`-Newtypes | — |
| 10 Cutover | offen | — |

Gates auf `2f9cd2d`: `cargo build`, `clippy --workspace --all-targets -D warnings`, `fmt --all --check`,
727 Tests, `cargo +nightly fuzz build`, alle 17 Parity-Suiten (`PARITY=1`) grün. Volle Nachmessung
≈ 20 min; Skript-Muster: jede Suite einzeln `PARITY=1 cargo test -p parity --test <name> -- --test-threads 3`.

**Entschieden, noch nicht umgesetzt:**
- **Steuerzeichen im ELSTER-XML** (Fuzz-Fund, `rust/elster/src/xml.rs:247`, Python `elster_xml.py` identisch):
  Steuerzeichen (XML-1.0-unzulässig) in `typ: text` beim Speichern abweisen (Auflage T, Rust
  `domain/src/wert.rs` bzw. `store::pruefe_bindung`, Python `store.py:_typ_konform` Z. 188) **und**
  `erzeuge_xml` fail-closed (`XmlFehler`). Auch im Python-Produkt, weil es bis zum Cutover live ist.
  Danach den Skip im Fuzz-Target `elster` entfernen; Regression `rust/fuzz/regressions/elster/`.
  Vault-Ticket `elster-xml-steuerzeichen-im-textwert`.

**Kleinere offene Befunde:** `eingang::beleg::extrahiere` kompiliert Regex je Aufruf (73–87 ms);
`eingang::ocr::lies_kontoauszug_pdf` liefert bei fehlender Datei `Ok(leer)`; Float-Rentenfreibetrag
fehlt im Parity-Korpus (nur Unit-Tests); reale Fälle decken Kinder/§ 23/DBA kaum (Golden-Fälle vor Cutover);
Mutationen überleben bei § 31-Gleichstand und `true` im Rentenbeginn-Jahr.

---

## 0. Worum es geht

Das Produkt (Fragen stellen, Steuer rechnen, Bescheid erklären, ELSTER-XML bauen) wird in Rust neu
gebaut. Das Verhalten nach außen bleibt gleich. Python bleibt Referenz, bis jede Zahl gleich ist.

**Warum es zählt:** Viele der 2 700 Python-Tests prüfen Dinge, die ein Typ unmöglich machen kann
(stille 0 bei fehlender Angabe, vorläufig als bestätigt gerechnet, Person-B-Feld bei
Einzelveranlagung). Die teuersten Befunde der letzten Wochen (13.568 €, 39.007 €, 21.000 €) waren genau
diese Klasse.

---

## 1. Korrekturen am Auftrag (gemessen, nicht angenommen)

| Auftrag sagt | Repo zeigt | Grad | Folge |
|---|---|---|---|
| 148 YAML-Dateien | **525** getrackt; das Produkt lädt zur Laufzeit **91** (bindung_* 25, params 58, kohorten 8) | explicit (`git ls-files`, Lader-Grep) | Phase 2 lädt die 91 typisiert. Die 148 sind vermutlich `sources/**/*.meta.yaml` ohne `bfinv` (derived) — Tooling, nicht Produkt. |
| ELSTER-XML in `produkt/` | Writer in `produkt/eingang/elster_xml.py`, ERiC in `elster/` (1 779 Z.) | explicit | `elster/checkest_gate.py`, `smoke_test.py`, `submission/validate_xsd.py` gehören in den Port. |
| 22 xfail-Tests | stimmt: **22 xfailed** zur Laufzeit (`make unit`, 2026-09-29) aus **15** Markern in **13** Dateien (parametrisierte Marker zählen je Fall) | explicit | Alle 22 Fälle werden `#[ignore = "…"]` mit Grund. |
| Catala-Backend für Rust | Catala 1.2.1 hat **kein** Rust-Backend (c, java, python, ocaml, interpret) | explicit (`catala --help`) | C-Backend + FFI, §3. |
| `catala_*` = Catala | **30 von 70** `catala_*`-Funktionen rufen Catala, **40 sind Hand-Python** (`catala_solz`, `catala_kist`, § 35a, § 23 …) | explicit (AST-Scan `runner.py`) | Die 40 werden von Hand portiert und per Parität gesichert — dort gibt es keine Regelquelle. |
| Tests 301 Dateien | 301 `.py` unter `tests/`, 297 davon `test_*`; 75 075 Zeilen | explicit | §6. |

---

## 2. Modulkarte

Produktcode: `produkt/` 36 Dateien / 17 017 Z., `elster/` 9 / 1 779 Z. `pipeline/` 26 / 6 528 Z.

### 2.1 Entscheidung `pipeline/`: bleibt Python-Tooling

Kein Modul in `produkt/` oder `elster/` importiert `pipeline/` (explicit, Grep). Die einzige
Lesestelle `traverser.py:40` (`lade_rules`) ruft nur `tests/test_traverser.py:233`. `pipeline/` ist ein
LLM-gestütztes Offline-Werkzeug (Gesetz → Catala) mit `requests`/`fastapi`/Subprozess an `catala`. Sein
Produkt ist Catala-Quelltext unter `rules/` — genau den konsumiert Rust über das C-Backend.
Ebenso bleiben Python: `oracle/` (GETTSIM ist Python), `golden/golden_lauf.py` (Parity-Treiber),
`scripts/`, `docstore/`, `ebilanz/`, `corpus/`, `reports/`.
Acht Löschkandidaten in `pipeline/` (bakeoff/*, judge_stabilitaet*, judge_replikation, run_smoke,
backlog) sind eine eigene Entscheidung, nicht Teil des Ports.

### 2.2 Crates

Workspace unter `rust/`. Zyklenfrei, von unten nach oben:

| Crate | Python-Quelle | Verantwortung |
|---|---|---|
| `catala-sys` | `rules/` via C-Backend | generiertes C + flacher Shim, `#[repr(C)]`-Structs, `unsafe` nur hier |
| `domain` | `store/store.py` (Typen, Meet, Hash), `api_constants.py` (Scheiben, Zustände) | `Cent`, `Euro`, `Vz`, `FeldId`, `Feldzustand`, `Wert`, `Schreiber`, `Sperrgrund`, `Veranlagung`, `Person`, `Snapshot<Roh/Bestaetigt>` |
| `bindung` | `produkt/bindung/*.yaml`, `params/`, `params/kohorten/` | typisiertes Laden, `deny_unknown_fields`, Duplikat = Fehler |
| `store` | `store/store.py`, `store/audit.py`, `store/fehler_log.py` | Event-Store, einziger Schreibpfad, `event_id`-Hash byte-gleich; Audit, PII-sicheres Fehlerlog |
| `engine` | `engine/runner.py` | 30 Catala-Aufrufe über `catala-sys`, 40 Hand-Accessoren |
| `interview` | `traverser/traverser.py`, `haut/bindung_rollen.py` | Relevanz, nächste Fragen, Justification |
| `konsistenz` | `konsistenz/*` | Flag-/Partner-Widersprüche, Preflight-Ampel |
| `bescheid` | `bescheid/*`, `unsicherheit/intervall.py` | vier Zweige, Sperrgründe, Intervall |
| `elster` | `mapping/est_mapping.py`, `mapping/xsd_verify.py`, `eingang/elster_xml.py`, `elster/checkest_gate.py`, `elster/smoke_test.py` | Deklaration, Kz-Format, XML, ERiC-FFI (`libloading`) |
| `eingang` | `eingang/kontoauszug_writer.py`, `vorjahr_writer.py`, `beleg_writer.py`, `vast_mapping.py`, `elster_writer.py` | Importe als Vorschläge; OCR per Subprozess |
| `llm` | `haut/llm_client.py`, `haut/api_llm.py`, `haut/pii_filter.py` | Client, 3-Stufen-Chat, strikte Parser, `Gefiltert`-Typ |
| `auth` | `auth/auth.py` | bcrypt, JWT HS256, Nutzer-Datei 0600 |
| `api` | `haut/server.py`, `haut/api.py`, `haut/flow.py`, `haut/ors_client.py`, `haut/api_auth.py` | axum, typisierte Handler, utoipa, statisches Frontend |
| `parity` (dev) | — | liest Python-Korpus, ruft Rust, diff |

`elster/versand.py` (Echtversand, kein Produktionsaufrufer, explicit) wird **nicht** portiert: Echtversand
ist Julius vorbehalten. `elster/{kz_extract,validate_mapping,bench,fuzz,eric_gate}.py` sind Tooling.

---

## 3. Catala-Anbindung

**Mechanismus: Catala-C-Backend, statisch gelinkt über das `cc`-Crate, hinter einem generierten
flachen C-Shim.** Geprobt (explicit, Audit C §2.3): alle 30 Module → C → `libtaxrules.a` → Rust-Binary.
Tarif (Grund-/Splitting, VZ 2024–2026, 24 327 Punkte) bit-identisch zum heutigen Python-Pfad,
gleicher SHA-256; Negativkontrolle macht den Diff rot. ~3 µs je Aufruf gegen ~287 µs.

Drei Hürden, alle in der Probe gelöst:

1. **Scopes sind modul-privat.** Die Deklarationen stehen in ```` ```catala ````-Blöcken, das C-Backend
   macht `static`-Funktionen daraus. Lösung ohne Änderung an `rules/`: das Generator-Skript
   `rust/catala-sys/gen.sh` kopiert `rules/estg` in ein Build-Verzeichnis, stellt dort die
   Deklarationsblöcke auf ```` ```catala-metadata ```` um (135 Blöcke) und ruft `clerk build`.
   `rules/` bleibt Quelle der Wahrheit und unverändert. Semantik-Beleg der Umstellung in der Probe:
   `clerk test` 136/136 Interpreter und 136/136 `--backend c`.
2. **GCC 16 scheitert an `catala_runtime.c:588` (`__gmp_vprintf`).** Lösung `-std=c89`, wie clerk selbst.
3. **Catala-Fehler springen per `longjmp`.** Jeder Scope-Aufruf läuft im C-Shim unter `catala_do`;
   kein `longjmp` durch Rust-Frames. Ungültiger Enum-Code → `abort()` → Enums in Rust typisiert.

**Entscheidung Toolchain:** Das generierte C wird **eingecheckt** (`rust/catala-sys/generated/`), damit
`cargo build` ohne opam läuft. `make catala-c` erzeugt es neu; ein Test vergleicht den Hash der Quelle
(`rules/**/*.catala_en`) mit dem im Generat gespeicherten und wird rot, wenn das Generat veraltet ist.

**GMP:** `catala_init` biegt die GMP-Speicherverwaltung prozessweit um. Kein zweiter GMP-Nutzer im
Prozess (kein `rug`). Die Catala-Arena ist `__thread`; der Shim ist darum pro Thread sicher, Aufrufe
serialisiert `catala-sys` trotzdem über einen `Mutex`, bis Last-Parität gemessen ist
(`ponytail:`-Grenze).

**Parität** (§5) läuft an der Naht `runner.catala_*`: sie deckt Catala-Scopes und Hand-Python gleich ab.
Unabhängige Orakel: `clerk test --backend c`, `golden/cases/*.yaml` (135), GETTSIM.

---

## 4. Invarianten je Modul → Rust-Typ

Vollständige Tabellen: Audit A §1/§3, Audit B §2/§3. Die tragenden:

| Invariante | Heute erzwungen | Rust |
|---|---|---|
| Geld in ganzen Cent; Euro nur an der Catala-Naht | Konvention, `// 100` an >100 Stellen | `Cent(i64)`, `Euro(i64)`, private Felder; `Cent::floor_euro()` = `div_euclid(100)` (Python `//` rundet gegen −∞, Rust `/` gegen 0 — tragend bei Verlusten, `bescheid_einkuenfte.py:292`) |
| Sätze, km, Zwischenprodukte exakt; kein Float für Geld | Python-`float`/`Decimal` gemischt | `Decimal` nur in Newtypes (`Satz`, `Km` …); `Decimal → Cent` nur über benannte Rundungsfunktionen je Rechtsgrundlage. Vault `decisions/rust-port-geld-cent-saetze-decimal.md` |
| Rückgabe-Einheit je Funktion | String-Tabelle `intervall.NATIV_EINHEIT` (`intervall.py:36`) | Rückgabetyp `Euro` oder `Cent`; Tabelle entfällt |
| Feldzustand offen / vorläufig / bestätigt | Strings + Abwesenheit, überall | `enum Feldzustand` |
| `bestaetigt` braucht `signal_2` | `store.py:365` | `Bestaetigt { signal_2: Signal2 }`, `Signal2` nicht leer |
| Ring rechnet nur mit Bestätigtem | EINE Stelle `bescheid_zweige.py:1487` | Typestate `Snapshot<Roh>` → `nur_bestaetigt()` → `Snapshot<Bestaetigt>`; Zweige nehmen nur Letzteres |
| Vorschlags-Schreiber (llm, beleg, vorjahr, kontoauszug) schreiben nur vorläufig | Auflage A `store.py:262-308` | `VorschlagEvent` hat keinen Zustand-Parameter |
| höchstens ein aktives Event je Feld, `ersetzt` gültig | Auflage B `store.py:368-381` | `Store::append -> Result<EventId, Abweisung>`, kein anderer `pub` Mutator |
| Wert passt zum Bindungstyp | Auflage T `store.py:167-248`, **nur wenn Bindung übergeben** | `Wert` wird immer gegen die Bindung geparst; kein Pfad ohne |
| `event_id` = sha256(canonical_json) | `store.py:22-34` | Serializer byte-gleich zu `json.dumps(sort_keys, ensure_ascii=False, separators=(",",":"))`; Parität über alle vorhandenen Stores |
| Fehlende Angabe sperrt statt 0 | 47 Sperrgrund-Literale `bescheid_deklaration.py:781-1443`; 152 × `.get(…,0)` in `runner.py` | `enum Sperrgrund` (47 Varianten, exhaustiver `klartext`); Eingabe-Structs je Scope **ohne** `Default` |
| Jeder Sperrgrund hat Klartext | Test `test_sperrgrund_klartext.py` | exhaustiver `match` |
| Partnerdaten nur bei Zusammenveranlagung | Handliste `partner_check.py:17-29` | `Veranlagung::Einzel { a } \| Zusammen { a, b }` |
| Nur VZ 2024–2026 | `VZ_ENUM[year]` KeyError | `enum Vz` mit `TryFrom<u16>` |
| Instanz-ID `basis`, `basis__n` | zwei Regeln im Widerspruch (`est_mapping.py:543` vs `traverser.py:152`) | `FeldId { basis, instanz: NonZeroU16 }`, ein `FromStr`; Parität mit **beiden** Python-Pfaden je Aufrufstelle, Widerspruch im Bericht |
| Bindung eindeutig je `feld_id` | nicht erzwungen (`traverser.py:33`, `safe_load`) | Ladefehler bei Duplikat und bei doppeltem Schlüssel |
| Kz-Format (floor/ceil/Komma/Datum) | Handmengen `est_mapping.py:35-165` | `enum KzFormat` je Kz |
| LLM-Ausgabe | `json.loads` → `[]` bei Fehler | serde-Struct `deny_unknown_fields`; Parität erhält „leer bei Fehler" als explizite Variante |
| Nur `filtere()`-Text geht ans LLM | Konvention `api_llm.py:1016` | `Gefiltert(String)`, nur `pii::filtere` erzeugt ihn; Client nimmt nur `Gefiltert` |
| ERiC grün nur bei rc==0 | `checkest_gate.py:66-92` | `enum EricRc` |
| Owner-Check vor Fall-Zugriff | `api.py:114-127` je Handler | Extractor `EigenerFall`; Handler ohne ihn bekommen keinen `Fall` |

**Korrektheit vor Parität** (Entscheidung Julius 2026-09-30, ersetzt „Parität vor Korrektur").
Maßstab ist das korrekte Steuerergebnis und korrektes Verhalten gegenüber dem Nutzer, nicht Python.
Der Python-Vergleich bleibt Pflicht — als Messinstrument gegen *unbeabsichtigte* Abweichungen.
Eine *gewollte* Abweichung braucht: (1) Eintrag in der Abweichungsliste des jeweiligen Parity-Tests
mit Begründung (Gesetzesstelle aus `sources/` oder Nutzerwirkung), (2) einen Test, der das korrekte
Verhalten festhält, (3) Erklärung in der Commit-Nachricht. Bestehende `// PARITÄT:`-Nachbauten
nachweislich falschen Verhaltens werden korrigiert, sobald ihr Modul angefasst wird; wo unklar ist,
was korrekt ist, bleibt Python und die Stelle geht als Frage an Julius. Bekannte Fälle:

| # | Befund | Anker |
|---|---|---|
| P1 | `_cent_nach_kz(-150)` → `"-2,50"` statt `"-1,50"` | `est_mapping.py:161-162` (explicit, ausgeführt) |
| P2 | `catala_einkuenfte_versorgung` schluckt `VersorgungsfreibetragOffen` → 0 € Einkünfte | `runner.py:1041-1046` |
| P3 | Verpflegungskürzung `except Exception: 0` | `bescheid_deklaration.py:109-110` |
| P4 | `/elster-ampel` ohne Owner-Check | `server.py:92` |
| P5 | 500-Body enthält `str(e)` (Wert aus Store-Meldung) | `server.py:252`, `store.py:230` |
| P6 | `/kontoauszug` Store-Abweisung → 500, `/event` → 422 | `api.py:534` vs `:1010` |
| P7 | `graph.js` ohne Auth-Header → 401 | `graph.js:13,70` |
| P8 | A-Renten Instanz-Σ, B-Rente Flat-Feld | `bescheid_zweige.py:1110-1127` |
| P9 | 200 von 332 echten ELSTER-XML schema-ungültig: `0` in GanzzahlPos-Kz (14 Kz fehlen in `_NULL_UNZULAESSIG_KZ`) | `est_mapping.py`; Vault-Ticket `elster-xml-null-in-ganzzahlpos-kz` |
| P10 | 32 echte Fälle tragen Alt-Herkunft `{"herkunft": …}` ohne `pruef_tiefe`/`haftung`; 5 Fälle haben VZ 2099, −5, 10^38 (API-Sicherheitstests) | Rust-Store lädt sie noch nicht — Entscheidung: tolerant laden wie Python, keine Migration fremder Nutzerdaten |

---

## 5. Parity-Harness

1. **Aufzeichnen (Python):** `tools/parity/record.py` wickelt jede öffentliche Funktion der
   portierten Module ein (zuerst die 70 `runner.catala_*`) und schreibt `{fn, args, ergebnis | fehler}`
   als JSONL. Quellen: `make unit`, `make golden`, `pipeline/produktion/rules.yaml`-Raster (105 Regeln),
   dichte Sweeps (Schritt 1 € um Zonengrenzen, negative Beträge für Floor-Pfade).
2. **Generieren:** `proptest`-Strategien je Funktion erzeugen Eingaben; `tools/parity/oracle.py`
   (JSON rein, JSON raus, ein Prozess für alle Aufrufe über stdin/stdout) beantwortet sie mit Python.
3. **Vergleichen (Rust):** `rust/parity` ruft die Rust-Funktion gleichen Namens, diff je Aufruf;
   Fehler-Parität: Python-Exception-Typ ↔ Rust-`Err`-Variante.
4. **Wirksamkeit:** jeder Lauf enthält eine Negativkontrolle (ein Ergebnis um 1 Cent gestört → rot)
   und nennt die Zeilenzahl beider Seiten vor „0 Abweichungen".
5. **Referenzstand:** der Korpus trägt `git describe --dirty` und den Hash von `git diff` des
   Python-Baums. Fremdes WIP ändert die Referenz; ein Korpus gilt nur für seinen Stand.

Abnahme je Modul: alle Golden-Eingaben + 1 000 generierte ohne Diff. Cutover: 10 000.

---

## 6. Testklassifikation (301 Dateien)

Quelle: `tests_part{1,2}.tsv` (Klassifikation nach Docstring, Imports und Stichproben — **derived**,
nicht jede Datei vollständig gelesen; Zeilen/Testzahlen nachgezählt, **explicit**).

| Kategorie | Dateien | Zeilen | Tests | Rust | Faktor | Rust-Zeilen |
|---|---:|---:|---:|---|---:|---:|
| GOLDEN | 67 | 14 742 | 671 | Fixtures `rust/fixtures/*.json` + ein Runner je Crate | 0,25 | ~3 700 |
| PROPERTY | 61 | 12 672 | 500 | ~25 proptest-Properties | 0,15 | ~1 900 |
| INTEGRATION (ohne UI) | 51 | 17 926 | — | Rust-Integrationstests | 0,6 | ~10 800 |
| INTEGRATION UI (Playwright) | 20 | 7 735 | — | **bleiben Python**, laufen gegen den Rust-Server (Phase 4 E2E) | 0 | 0 |
| NOT_PORTED | 80 | 17 028 | 514 | ersetzt durch Typ/serde/Lint, je Datei in der TSV benannt | 0 | ~500 (Lade-Validierung) |
| TOOLING | 22 | 4 972 | 279 | bleiben mit `pipeline/`/`oracle/` | 0 | 0 |
| **Summe** | **301** | **75 075** | | | | **~17 000** |

Tragende Ersetzungen der NOT_PORTED-Gruppe (Details je Datei in der TSV, wird als
`rust/TESTMAP.tsv` eingecheckt):

- YAML-Form, Duplikate, unbekannte Schlüssel → `serde(deny_unknown_fields)` + Duplikat-Fehler in `bindung`.
- Sperrgrund-Klartext vollständig → exhaustiver `match`.
- AST-Scans auf Python-Quelltext (Zeilen-Ratschen, Import-Nähte, Literal-`ort`) → `clippy::too_many_lines`, Crate-Graph, `&'static str`.
- vorläufig als bestätigt gerechnet → `Snapshot<Bestaetigt>`.
- Einheit Euro/Cent verwechselt → Newtypes.
- Partner ohne Zusammenveranlagung → `Veranlagung`-Enum.

Die UI-Tests (Playwright, Python) bleiben: sie prüfen das Frontend, das unverändert bleibt, und sind
damit genau der End-to-End-Test „Frontend gegen Rust-API" aus Phase 4.

---

## 7. Port-Reihenfolge (blattzuerst)

| Schritt | Crate | Abnahme |
|---|---|---|
| 1 | Workspace, Lints, CI, `catala-sys` (Generat + Shim für die 19 benutzten Module), Parity-Harness, Fixtures | `cargo build/clippy/test` grün; Tarif-Parität 24 327 Punkte |
| 2 | `domain` + `bindung` | alle 91 Laufzeit-YAMLs laden; Fehler gelistet, Datenfixes in eigenem Commit |
| 3 | `store` | `event_id` byte-gleich über alle vorhandenen Fälle; append-Parität |
| 4 | `engine` (70 Accessoren) | Parität Korpus + 1 000 generiert je Funktion |
| 5 | `interview`, `konsistenz`, `bescheid::intervall` | Parität |
| 6 | `elster` (Deklaration, Kz-Format, XML, ERiC) | XML byte-gleich; XSD-valide |
| 7 | `bescheid` | Parität `/ergebnis` auf allen Golden-Fällen |
| 8 | `eingang`, `llm`, `auth` | Parität |
| 9a | `api`-Gerüst: 24 Routen, Auth, `EigenerFall`, Fehlerformate, HTTP-Differenz-Harness | Harness 0 Abweichungen auf den implementierten Routen |
| 9b | **Härtung aller portierten Crates** (Entscheidung Julius 2026-09-30, vor 9c) | siehe unten |
| 9c | `api`-Handler lesen/schreiben + Frontend-E2E | Kontrakttest, Playwright gegen Rust |
| 10 | Cutover — **nur bei vollständiger Parität** | 10 000 generiert |

Scheitert ein Schritt nach drei Versuchen, wird er zurückgenommen und im Bericht geführt.

### Schritt 9b — Härtung (Abnahme je Crate, ein Commit je Crate)

Nachmessung 2026-09-30 (HEAD `bb00573`, Grep über `rust/*/src`) gegen die Zielstandards (§9):

| Standard | Stand | Ziel |
|---|---|---|
| Nichts Untypisiertes nach innen | `bescheid` rechnet auf `serde_json::Value`-Feldern; die meisten der 75 `_ =>`-Arme stehen auf `Value` | `Felder` an der Grenze einmal in typisierte Structs parsen; Python-Eigenheiten (`bool` als `int`, `TypeError` bei Text) als benannte Varianten des Parse-Ergebnisses, Parität über diesen Adapter |
| Kein roher String über Modulgrenzen | 78 `pub fn` mit `&str`-Parameter (z. B. `bescheid_fn(quantitaet: &str)`) | Newtypes/Enums, wo der String Regeln trägt |
| Domain-Enums exhaustiv, kein `_` | 75 `_ =>` (Mehrzahl auf `Value`) | keiner auf Domain-Enums |
| Kein Float für Geld | 2 Geld-Pfade in `bescheid` | 0; Regel §4 |
| `debug_assert!` für Invarianten | 0 | an nicht offensichtlichen Invarianten |
| Ein Doctest je `pub fn` | u. a. bescheid 2/43, store 5/44, catala-sys 0/26 | vollständig |
| 22 xfail als `#[ignore = "Grund"]` | 0 | 22 |
| `cargo fuzz` für externe Parser | 0 | XML, Uploads (CSV/PDF/Beleg), LLM-Ausgabe, API-Payloads |
| End-to-End Eingabe → Berechnung → Bescheid → ELSTER-XML | 0 | einige Hauptflüsse, XML gegen XSD |
| Property-Tests reiner Rechenlogik | nur Parität per proptest | Schranken, Monotonie, Roundtrips, Symmetrie der Zweige |

**9b-B Typisierung** (Plan 2026-09-30, Entscheidungen Julius übernommen). Zwei Schichten an der
Store-Grenze: `domain::PyWert` (verlustfreies typisiertes Abbild des Python-Werts; die heute sechsfach
nachgebaute Python-Semantik `int()`/Wahrheitswert/`isinstance`/`==`/`repr` genau einmal, exhaustiv) und
`Lage<T>` = `Fehlt | Null | Gueltig(T) | Abweichend(&PyWert)` gegen den Bindungstyp. Commits blattzuerst
(Strangler, neuer Typ neben dem alten, Alt-Typ am Ende weg):
K0 Messung der Wertformen in 192 Stores + Golden · K1 `domain` (PyWert, Lage, Kz, VorschlagTyp, FallId;
Äquivalenz-Proptest gegen die Alt-Helfer) · K2 `store` (typisierte Felder; `event_id`-Roundtrip über alle
realen Stores Pflicht) · K3 `interview` · K4 `konsistenz` · K5 `elster` · K6+K7a `intervall` + `bescheid`-Helfer
· K7b `bescheid` `Lage<Enum>` + `Quantitaet` · K7c Geldpfade · K8 Alt-Typen entfernen, Grep-Gates
(`serde_json::Value` in bescheid/konsistenz/intervall/interview = 0; `_ =>` auf Domain-Enums = 0) · K9 `&str`-Rest.
Entscheidungen: `Lage<T>` nur für Enum-Felder (Veranlagung, Konfession, Bundesland, Rentenart) und
Cent-Summen in `bescheid`, voller bindungstypisierter Snapshot nach Cutover · `auth`/`audit`-Newtypes
(`Username`, `FallId`) in 9c mit den API-Handlern · Listen/Objekte und Ganzzahlen > u64 in Fremddaten
bleiben ladbar, Grenze dokumentiert · `PyWert` in `domain` · 8 Nachschlage-Schlüssel bleiben `&str`.
Korrigierter Messstand: 92 `pub fn` mit `&str` (≈32 tragen Regeln); 75 `_ =>`, davon 7 auf Domain-Enums,
35 auf `Value`, 28 legitim. Grob 13–18 Worker-Läufe; Risiko hoch bei K2 (`event_id`) und K6/K7a (Breite).

Arbeitsregeln ab jetzt: ein Commit je Schritt; nach jedem Schritt `cargo build`, `cargo clippy -- -D warnings`,
`cargo test` und **alle** Parity-Suiten; jeder nicht portierte Python-Test nennt in Commit-Nachricht und
`rust/TESTMAP.tsv` den Typ oder die Property, die ihn ersetzt; Routen, Payloads, Fehlerformate,
Bescheid-Text und XML bleiben identisch, jeder Fixture-Diff wird im Commit erklärt.

---

## 8. Offene Fragen (nicht blockierend, Standard gewählt)

| # | Frage | Gewählter Standard |
|---|---|---|
| F1 | Negativformat P1 korrigieren? | Ja, korrekt bauen (Korrektheit vor Parität, §4); Abweichung mit Fixture-Diff im Commit erklärt |
| F2 | Instanz `x__1`: welche Regel? | Traverser-Regel (`__1` ist keine Instanz); `est_mapping`-Aufrufstellen per Parität prüfen |
| F3 | Hartkodierte Gesetzeswerte (`runner.py:462-467,1052,1703-1706`) nach `params/`? | Port übernimmt sie als benannte `const` mit § im Doc-Kommentar; Umzug separat |
| F4 | `versand.py` (Echtversand) portieren? | Nein — Julius-Vorbehalt |
| F5 | Löschkandidaten in `pipeline/` | Nicht Teil des Ports |
| F6 | Cutover löscht Python | Nur wenn Schritt 10 grün; sonst bleibt Python Referenz und der Bericht nennt die Lücke |
| F7 | YAML-Crate (`serde_yaml` archiviert) | `serde_yaml_ng` oder `serde_norway` nach Doku-Check; Duplikat-Schlüssel-Verhalten per Test belegt |

---

## 9. Zielstandards (verbindlich)

Vorgabe Julius, 2026-09-30, wörtlich übernommen. Sie gilt für jeden Schritt und für jede Abnahme.
Wo der Plan davon abweicht, steht die Entscheidung als Fußnote dabei. Der Messstand gegen diese
Standards und der Weg dorthin stehen in §7, Schritt 9b.

### Types
- Wrap validated values in newtypes with private fields and a fallible constructor. No raw String or
  numbers across module boundaries when they carry rules. Money is rust_decimal::Decimal, never float.¹
- Model states as enums, not flags or Option fields. Use typestate where operations are only valid in
  certain states (e.g. a Bescheid that cannot be rendered before all Zweige are resolved). Match domain
  enums exhaustively, no `_` arm.
- Every external input is parsed once at the boundary into a typed struct: YAML (Bindungen, Regeln,
  Parameter) via serde with `deny_unknown_fields`, API payloads, LLM responses, ELSTER data. Nothing
  untyped travels inward.
- YAML is validated at build time or first load. Invalid YAML is a data bug: list it, fix in a
  separate commit.

### Errors and panics
- Library code returns `Result` with typed errors (thiserror), one error enum per area. No `unwrap`,
  `expect`, direct indexing or `panic!` outside tests and main.
- State non-obvious internal invariants with `debug_assert!`.

### Lints
```toml
[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
indexing_slicing = "deny"
panic = "deny"
too_many_lines = "deny"
pedantic = { level = "warn", priority = -1 }
```
Allow these inside `#[cfg(test)]`. `too_many_lines` replaces the Python size ratchets.

### Tests to write
- Golden tests: legal examples (Musterfälle), Bescheid text, ELSTER XML validated against XSD. Store as
  data fixtures, not code. These are acceptance tests against the law and never get removed.
- Parity tests: same input to Python and Rust, outputs must match, on all golden inputs plus
  proptest-generated inputs. See Phase 1. (In diesem Plan: §5.)
- Pure calculation logic: proptest properties (bounds, monotonicity, roundtrips, symmetry between Zweige).
- Public API: one doctest per public function.
- Main flows: a few end-to-end tests, Eingabe to Berechnung to Bescheid to ELSTER XML.
- Parsers of external input (XML, uploads, LLM output): cargo fuzz target.
- The 22 xfail tests: port as `#[ignore = "reason"]` with the reason.

### Tests not to port
- Tests of validation that a type or serde now rejects: wrong type, missing field, unknown key, out of range.
- Tests of YAML shape.
- Tests of getters, constructors without logic, glue code, framework wiring.
- Tests coupled to Python implementation details or call order.
- Groups of example tests that one property subsumes.

### Working rules
- Work on one module per step, in plan order. After every step run `cargo build`,
  `cargo clippy -- -D warnings`, `cargo test`, and the parity harness for all ported modules. All must
  pass before continuing.
- A module is done only when its parity tests pass on every golden input and 1000 generated inputs.
- For every Python test not ported, name the type or property that replaces it in the plan and the
  commit message.
- Keep routes, payload shapes, error formats, Bescheid text and XML output identical. Any fixture diff
  must be explained in the commit message.
- One commit per step.
- If a step still fails verification after 3 fix attempts, revert it, note it in the report, and
  continue with the next module.

¹ **Entscheidung 2026-09-30 (Julius):** Beträge bleiben ganzzahlig `Cent`/`Euro`. Sätze, km und
Zwischenprodukte sind `rust_decimal::Decimal` in Newtypes. Der Übergang `Decimal → Cent` läuft nur über
benannte Rundungsfunktionen je Rechtsgrundlage. Kein Float für Geld. Grund: Rundungszwang, bitgleiche
Parität zu Python/Catala, eindeutige Serialisierung (`event_id`). Siehe §4 und Vault
`decisions/rust-port-geld-cent-saetze-decimal.md`.
