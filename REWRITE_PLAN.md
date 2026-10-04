# REWRITE_PLAN — produkt/ nach Rust

Stand 2026-09-28, gemessen gegen HEAD `607aedb` plus fremdes, uncommittetes WIP in 12 Dateien
(`runner.py`, `api.py`, `store.py`, `bescheid_*.py`, `traverser.py`, `est_mapping.py`, zwei Tests).
Evidenz: vier Audit-Berichte (Kern · API/ELSTER/YAML · Pipeline/Catala · Testklassifikation), deren
Kernaussagen unten mit `datei:zeile` stehen. Grade nach CLAUDE.md: **explicit** = gelesen oder gelaufen,
**derived** = daraus berechnet, **conditional** = Annahme.
Die Zahlen in §1, §2 und §6 sind Stand 2026-09-28 (`607aedb`). Wo `2bc35bd4` (2026-10-03) abweicht, steht der neu
gemessene Wert in Klammern, mit dem Befehl.

---

## Fortschritt (Stand 2026-10-04, HEAD `bc092a74`; Gates von main auf `057b7ec3`; das Tracking-Ref `origin/claude/implementation-start-ypyyqw` steht lokal auf `55e9164d`, HEAD liegt 209 Commits voraus)

Messregel dieses Abschnitts: Jede Zahl ist am Baum `bc092a74` neu gemessen, mit dem Befehl daneben (Worktree `~/.cache/taxgraph-tmp/wt-plan`,
Zweig `orch/plan-nachzug`), außer wo „gemessen auf `<Commit>`“, „laut Merge-Nachricht“ oder „nicht gemessen“ steht. Die Gates stammen von
main (Logs `~/.cache/taxgraph-tmp/gate-main-057b/`); ein Tor auf `bc092a74` läuft dort noch und ist hier nicht eingetragen.

CI-Lauf 37010730963 auf `8e48cf7`: **alle fünf Jobs grün** (erster ganz grüner Lauf seit dem 20.08). Das Vault-Ticket
`github-ci-seit-20-08-rot` ist geschlossen (`backlog/archive/taxgraph/github-ci-seit-20-08-rot.md`). Gepusht ist bis `55e9164d`
(Reflog des Tracking-Refs: „update by push“, 2026-10-03 02:42; `git rev-list --count origin/claude/implementation-start-ypyyqw..HEAD`
→ 209, mit `--no-merges` 132, mit `--first-parent` 75); diese 209 Commits liegen nur lokal; seit `2bc35bd4` sind es 127
(`git rev-list --count 2bc35bd4..HEAD`), davon 42 Merges auf dem ersten Elternpfad. CI auf `55e9164d` und auf HEAD nicht abgefragt
(nicht gemessen); gepusht wird nur auf Julius' Wort.

**Seit `8e48cf7` in main** (je Spitze per `git log --first-parent 8e48cf7..HEAD`):

- `opt-level = 1` im dev-Profil (`6fbe3d5`, `rust/Cargo.toml:63`, `grep -n opt-level rust/Cargo.toml`): der Beschluss vom 2026-10-02 ist gebaut.
- 9c: `_cfg`/`_scheibe_bindung` verdrahtet, `GET /stand`, `flow` im Rust-Server (`ecdfcb9`); **alle GET-Routen** im
  Rust-Server (`8d1bd96`); die POST-Routen `event`, `kontoauszug` und `vorjahr` sind gebaut und gegen Python im
  Harness gemessen (gemergt als `82bbf1e`); `chat` und `entfernung` folgten mit `c949f86`, `einreichen` mit `8c89556`.
  **Alle 24 Routen sind portiert** (Routentabelle des Harness: Python 24, Rust 24; `PARITY=1 cargo test -p parity --test
  api_http_paritaet routentabelle_gleich_python -- --nocapture` auf `bc092a74`: „Python 24 Routen, Rust 24 Routen“, Test grün, Methode,
  Muster und Reihenfolge gleich). Die Liste `NICHT_PORTIERT` gibt es
  nicht mehr (`grep -rn NICHT_PORTIERT rust` → kein Treffer, auf `bc092a74` wiederholt; entfernt mit `d9d51a6`); eine Antwort `501 nicht_portiert`
  ist im Harness eine Abweichung (`rust/parity/tests/api_http_paritaet.rs:15`). `einreichen` prüft nur: ERiC läuft mit
  `ERIC_VALIDIERE`, lokal, ohne Netz und ohne Zertifikat, gesendet wird nichts (`rust/api/src/einreichen.rs:1-8`).
- K8-Rest und `Kz`/`Vz`/`EventId`/`BasisId` als Typen in bindung, elster, konsistenz (`80743f2`); K9 `interview`
  Tor 2 (`97d3846`), `py_eq` gegen echtes CPython und `wert_paritaet` (`619f5dd`), Typfragen auf `PyWert` gegen
  echtes CPython, Folge 3 Block B und C (`6f90e76`).
- Härtung: Wortlaut `persistenz`, Bereichs-/Negativ-/`ts_herkunft`-Türen (`08b3d4a`); Rust-Tür weist eine Ganzzahl
  außerhalb von i64 mit 400 ab (`e24748a`); Python-Schreibwege gegen Bereich, Minus, Form: 422 statt 500 (`9cb1daa`);
  `signal`-Zusatzschlüssel 422 und Kontoauszug verwirft nur die Buchung (`0ccbeca`); `repr(float)` rundet den
  Gleichstand wie CPython (`c54da3a`).
- Seit `b540590` gemergt (Auswahl aus `git log b540590..HEAD`): Kontoauszug meldet fehlendes Werkzeug mit 503
  und die Rückgabecodes von `pdftoppm`/`tesseract` (`9a58e8a`); `chat`/`entfernung` in Rust, `ORS_API_BASE`, Stub je Server
  (`c949f86`); Begleitfelder-Tabelle gegen die Rust-Route, Kontoauszug `datum`/Zweck 422 (`fa181f0`); `/deklaration` sperrt
  bei Sperrgrund wie `/einreichen` (`e4f2c07`); `rc-diagnose`: 409 mit Klartext (`b4fbe8b`); `einreichen` in Rust
  (`8c89556`); Protokollzeile nennt den Status der Antwort (`6d4cc2c`); Login weist Namen außerhalb des Musters und
  Nicht-Text mit 401 ab (`2720ea3`) und schreibt einen Nicht-Text-Namen als `unbekannt` ins Protokoll (`3275a31`, `4368ebb`);
  Wächter-Pin der Kind-Frage (`0914e13`); Fremddienst-Harness, 501-Liste entfernt (`d9d51a6`); Rentenbeginn-Jahr <= 0
  sperrt, KAP-Vorschau liest nur Bestätigtes (`2bc35bd`).
- Seit `2bc35bd4` gemergt (127 Commits, 42 Merges auf dem ersten Elternpfad; die Punkte `make ui-rust`, `PARITY_N` und TESTMAP auf
  `bc092a74` nachgemessen, danach die übrigen Merges):
  - **`make ui-rust`** (`5d212c31`, Merge `081e1d94`) startet die 22 UI-Dateien gegen das Rust-Binary
    (`make -n ui-rust | tr ' ' '\n' | grep -c '^tests/'` → 22, auf `bc092a74` wiederholt). Die 23 Tests, die gegen Rust nicht grün werden
    können (15 LLM-Stub, 8 ERiC-Stub; `grep -v '^#' tools/ui_rust/ausschluss.tsv | cut -f2 | sort | uniq -c`, auf `bc092a74`
    wiederholt), stehen mit Ursache in `tools/ui_rust/ausschluss.tsv` und laufen als
    `xfail(strict)`. Lauf auf `5d212c31` (`UI_N=3`): 249 passed, 23 xfailed, 149 s; davon starten 216 einen
    Rust-Server, 33 sind reine Python-Prüfungen im Prozess (die Zeile `ui-rust:` am Lauf-Ende zählt das).
    `make ui-rust-gegenprobe` (Rust endet sofort): 6 failed, wie gewollt. Auf `bc092a74` nicht neu gelaufen (nicht gemessen);
    nicht in der CI (kein Playwright, kein Chromium). Der Rust-Server schreibt dabei die Antwort-Kopfnamen in
    Title-Case wie Python (`rust/api/src/dienen.rs`, `rust/api/tests/kopfnamen.rs`).
  - **`PARITY_N`** (`aaaad5a8`, `rust/parity/src/fallzahl.rs`): ein Env-Knopf für die Fallzahl der Zufalls- und
    Generator-Läufe. 17 der 22 `*_paritaet.rs` lesen ihn (`ls rust/parity/tests/*_paritaet.rs | wc -l` → 22; `grep -l 'fallzahl::' rust/parity/tests/*_paritaet.rs | wc -l`
    → 17; auf `2bc35bd4` waren es 16 von 21); `api_http`, `eingang_werkzeug`, `feld_kennung`, `flow` und `store_zeichensatz` lesen ihn nicht
    (`wert` seit `dd23965d`, gemergt mit `94882f7a`, ja). Ohne die Variable gilt
    der Standard der Suite; ein Wert, der keine ganze Zahl ab 1 ist, bricht ab (`rust/parity/tests/fallzahl_env.rs`).
    Wächter, die an die Standardzahl gebunden sind, laufen nur bei Standard-N (`fallzahl.rs`, `wache_gilt`).
    `PARITY_N=7` lief laut Merge-Nachricht `94882f7a` grün in allen 16 Suiten plus `wert_paritaet` (Log nicht eingesehen); `PARITY_N=10000`
    (die Cutover-Zahl) ist nicht gemessen.
  - **TESTMAP-Test** (`3342d515`, Merge `d32c3443`; Zahlen auf `bc092a74` gemessen): `rust/TESTMAP.tsv` hat 439 Zeilen ohne Kopf (`wc -l` → 440),
    342 `tests/…` und 97 `rust/…` (397 = 331 + 66 am 2026-10-03; `cut -f1 rust/TESTMAP.tsv | grep -c '^tests/'` → 342, `'^rust/'` → 97).
    `tests/test_testmap_vollstaendig.py` (läuft in `make unit`) wird rot, wenn eine Datei in `tests/` oder `rust/*/tests/` keine Zeile hat;
    `python3 tests/test_testmap_vollstaendig.py` → „439 Testdateien, 0 ohne Zeile“ (main: 436/0 auf `057b7ec3`). Bericht `berichte/testmap.md`.
    Die Spalte `lines` prüft kein Tor: Vault-Ticket `testmap-zeilen-spalte-veraltet-und-kein-tor-prueft-sie` (unqualifiziert in `tickets/`;
    Titel: „… in 36 Python-Zeilen veraltet“, Stand `19554121`, nicht neu gemessen).
  - **Weitere Merges seit `2bc35bd4`, je Spitze** (`git log --first-parent --reverse 2bc35bd4..bc092a74`; „laut Merge“ = aus der Merge-Nachricht,
    nicht nachgemessen). Die fünf jüngsten Befunde zuerst:
    - **Überlauf-422, Eintrag 1c** (`057b7ec3`, Fix `a388883e`): Ein Betrag außerhalb von i64 über `POST /event` ist in Rust ein 422 mit Klartext
      statt eines 500 (`CatalaError`/`OverflowError`). Gewollte Abweichung — Python rechnet weiter und antwortet 200 —, eingetragen als 1c in
      `rust/parity/tests/api_http_paritaet.rs:5782`. `cargo test -p api --test ueberlauf_klassen_hermetisch` → 5 passed (`bc092a74`). Ein Jahr
      außerhalb u16 ist über HTTP nicht erreichbar, daher keine Änderung (laut Merge). Davor `a966b94c`: 127 Überlauf-Mutanten, 119 rot an
      benannten Tests, 8 Überlebende (5 unerreichbar, 2 gleichwertig, 1 ungeklärt; laut Merge).
    - **Kette-Endstand** (`cc28b334`): Die Kette der Zweige (Rechenweg) steht in Python und Rust auf demselben Endstand. Neue Tests:
      `cargo test -p api --test kette_endstand_hermetisch` → 12 passed, `python3 -m pytest tests/test_rechenweg_endstand.py -q` → 13 passed
      (`bc092a74`); 7 eigene Mutanten rot (laut Merge).
    - **Kz-Wache** (`a9f6aec8`, Commit `82a357c7`): `tabellen::tests::jedes_kz_literal_ist_von_der_fixture_gelesen` und `rust/elster/src/kz_wache.rs`
      verlangen für jedes exakte Kz-Literal in `rust/elster/src` (221: `deklaration.rs` 14, `kz_format.rs` 131, `tabellen.rs` 64, `xml.rs` 12;
      gemessen auf `82a357c7` mit einer vorübergehenden Zählzeile, `rust/elster/src` seitdem unverändert, `git diff --stat 82a357c7 bc092a74 --
      rust/elster/src` leer) einen von drei Gründen: von der Fixture gelesen, Item in `tabellen.rs`, `Zuordnung::Verhalten`. Zwei Lücken im
      Fixture-Test (`iban_weiche`, `bankverbindung`) geschlossen; nur Test-Code. `cargo test -p elster --lib -- kz_wache jedes_kz_literal` → 5 passed
      (`bc092a74`). Vorläufer: `707937ab` (Fixture `rust/fixtures/kz_tabellen.json` gegen `est_mapping.py`, ohne `PARITY=1`) und `7d9a6c39`
      (89/89 Tabellen-Mutanten tot; Nr 59, die IBAN-Ausland-Kz in `abgabe_pruefen`, durch 4 Verhaltenstests geschlossen; laut Merge).
    - **Golden-Rust-Prüfer** (`b13a0669`): `rust/engine/tests/golden_werte.rs` rechnet die 135 Golden-Fälle gegen die Rust-Engine auf die Erwartung,
      Fallzahl gepinnt (`cargo test -p engine --test golden_werte` → 2 passed; `python3 -c "import json; print(len(json.load(open('rust/fixtures/golden_cases.json'))))"`
      → 135). 22 Mutanten, 16 erschlagen, 6 `floor`/`ceil`-Überlebende klassifiziert (laut Merge). Davor `3fabb4ed`: ein Rust-Test hält die Form der
      Fixture. Das Tor für den Wert bleibt `make golden`.
    - **`korpus_faelle`-Exit** (`bc092a74`): `tools/parity/korpus_faelle.py` endet nach einer Abweisung mit Exit 1 und nennt ihre Zahl
      (`python3 -m pytest tests/test_korpus_faelle_exit.py -q` → 8 passed, `bc092a74`). Davor `2ec08c52`: `lege_an` nennt Status und Grund statt
      `AttributeError`; das Vault-Ticket `korpus-faelle-liest-ein-feld-das-apierror-nicht-hat` liegt weiter in `tickets/`.
    - **Verhalten, Python und Rust gleich** (laut Merge): GWG ohne Sofortabzug sperrt sichtbar (`gwg_mehrwertsteuer_offen`, `gwg_abschreibung_offen`;
      `f05b0247`); die 2.000-€-Auslandsgrenze der Doppelten Haushaltsführung gilt erst ab VZ 2026, VZ 2024 ist Folgerung, der Wortlaut a. F. liegt nicht in
      `sources/` (`583d70f1`); § 34 Abs. 3: der Antrag schreibt `p34_abs3_antragsbetrag` (`4295f49e`, Partner-Pfad dort ausdrücklich nicht gebaut),
      Gewinn beim Ehegatten sperrt mit `abs3_partner_gewinn_offen` (`0749aab3`, 57 Sperrgründe); Kind-Qualifikation je Instanz (`f22e2e76`); eine
      Ableitung schreibt keinen Wert außerhalb `bereich` (`4b2cf0d2`); `rentner_pflege_weitere_personen` höchstens 9 (`431ca41c`); zwei Renten einer
      Person sind ein `<R>` mit zwei `<Einz>` (`f37c2fb3`); Zeichensatz `Standard_E_V2` beim Speichern (`d635627c`, siehe „Entschieden“ unten);
      `stammdaten_hausnummerzusatz` E0101207 (`f77a7776`); Verpflegungs-Vorschau liest nur Bestätigtes (`cbc376a6`); VaSt-Import lehnt Beträge
      außerhalb i64 ab, Python (`94882f7a`).
    - **Wächter ohne `PARITY=1`, nur Tests** (laut Merge): Gewerbesteuer-Hebesatz und Haushaltsnahe (`149cde0b`), § 35a-Abbildung und `rentner_gesamt`
      (`57944f48`), p24a/p24b/p31 und weitere Posten (`af9bed11`), zwei § 23-Sperren über `GET /ergebnis` (`4a2f6ea4`), kein Aufrufer von
      `lies_beleg_text` unter `produkt/` (`f80187db`), kein `und_feld`-Ziel ist ein Zahlfeld (`c00649ea`), Rentenbeginn und KAP-Leck (`0c20203b`),
      OpenAPI-Router gegen Dokument (`593d94b4`), Bindungswerte gegen das ELSTER-Schema (`9ad51c19`), `rentner_gesamt`-Sperren, Zugriffe,
      Kind-Abzüge und Überlauf-Pfade (`931b68f8`), 74 Tests je Schalter mit 53 Mutanten, 10 überlebt, alle Äquivalenzen (`0aa91677`), Parity: ein Block
      ohne beurteilte Zeile ist rot (`604022c8`).
    - **Werkzeug und Doku:** UI-Hilfen melden Banner und Knopfzustand bei Last (`19554121`, `b7b4e55e`), der Mitschnitt-Leser kennzeichnet
      „(vorläufig)“ (`08104fce`); `7e81ada2`: Tor auf `9ad51c19` laut Merge clippy 0, ws 1110/0/21, PARITY 178/0, unit 4146/15/22, golden 135/135.

**Gates auf `057b7ec3`** (2026-10-04 06:41–06:50, main, Skript `~/.cache/taxgraph-tmp/gate-main-057b/lauf.sh`, Logs ebenda, Baum sauber laut
`00-kopf.txt`; von main übernommen, nicht wiederholt; die Summe über die `test result`-Zeilen ist aus `4-cargo.log` nachgezählt): `make unit`
4405 passed / 15 skipped / 22 xfailed (458,58 s); `make golden` 135/135; `cargo clippy --workspace --all-targets --exclude parity -- -D warnings`
rc=0; `cargo test --workspace --exclude parity --no-fail-fast` 1369 passed / 0 failed / 21 ignored (93 `test result`-Zeilen, Doctests
eingeschlossen); `python3 tests/test_testmap_vollstaendig.py` 436 Testdateien, 0 ohne Zeile. Gegenüber `b540590` (3878 Tests in 188 s) brauchte
`make unit` 458,58 s für 4405 Tests; die Ursache (Testzahl, Last) ist nicht untersucht. Nicht Teil dieses Tors: `PARITY=1` (unten),
Gegenproben, echte Akten. Ein Tor auf `bc092a74` läuft bei main und ist hier nicht eingetragen; die TESTMAP-Zahl auf `bc092a74` (439/0, oben)
ist eigene Messung.

**Gates auf `b540590`** (2026-10-03 02:28–02:39, Instructor, Hauptbaum, Skript `~/.cache/taxgraph-tmp/gate-runde.sh`,
Log `gate-runde.log`; jede Stufe einzeln): `make unit` 3878 passed / 0 failed (14 skipped, 25 xfailed, 188 s);
`make golden` 135/135; clippy `--workspace --all-targets -D warnings` rc=0; `cargo test --workspace --exclude parity
--no-fail-fast` 1005 passed / 0 failed / 25 ignored (Summe über 51 `test result`-Zeilen, Doctests eingeschlossen);
**19 von 19 Parity-Suiten** je einzeln `PARITY=1 … -- --test-threads 3` rc=0, die langsamsten `interview` 138 s,
`api_http` 76 s, `elster` 72 s, `bescheid_zweige` 68 s (Suite-Zeit; vorher 156 s, `bescheid_deklaration` 86 → 29 s).
Gesamte Parity-Runde ≈ 10 min statt ≈ 20 min. **Nicht nachgemessen:** Gegenproben G1–G6 (zuletzt auf `8e48cf7`
je rot), `PARITY=1 cargo test --workspace` in einem Lauf, echte Akten. Seit `b540590` ist hier keine Runde über alle Parity-Suiten belegt (nicht gemessen); die Zahl der
Parity-Suiten ist von 19 auf 22 gestiegen (`ls rust/parity/tests/*_paritaet.rs | wc -l` → 22 auf `bc092a74`; 21 auf `2bc35bd4`, 19 auf
`b540590`). Laut Merge-Nachricht `7e81ada2` lief auf `9ad51c19` ein Tor mit PARITY 178/0 (Log nicht eingesehen, nicht nachgemessen).

**Offen**

1. Cutover (Schritt 10). Alle Routen aus 9c sind portiert (oben). Der Harness-Pfad für `chat`/`entfernung`/`einreichen` steht:
   Abschnitte `extern/chat`, `extern/entfernung`, `extern/einreichen` in `api_http_paritaet.rs` mit Fremddienst-Stubs je Server;
   das Chat-Surrogat ist dokumentierte Abweichung C (`115ddac`). **Offen, Entscheidung Julius.** Die Voraussetzungen laut Plan (§ 5 „Cutover: 10 000“,
   § 7 Schritt 10 „nur bei vollständiger Parität“) sind offen oder nicht gemessen: (a) 10 000 generierte Fälle: nicht gemessen; (b) Gegenproben
   G1–G6: zuletzt auf `8e48cf7` je rot (G1–G9 auf `a941ea7`), danach nicht nachgemessen; (c) `PARITY=1 cargo test --workspace` in einem Lauf: nicht
   gemessen; (d) Parity-Harness-Binärdefekt: Der Harness `api_http_paritaet` startet die Rust-Server vom Pfad `<CARGO_TARGET_DIR>/debug/taxgraph-api`;
   jedes andere `cargo test -p api` im selben Zielverzeichnis ersetzt die Datei durch einen Bau ohne `festzeit`, die Server haben dann die echte Uhr, und
   die `event_id`-Abweichungen sehen wie ein Produktfehler aus (laut Commit-Nachricht, auf `bc092a74` nicht nachgemessen). Die Behebung steht als
   Commit `eebe4578` (2026-10-04 07:06, ein Commit über `057b7ec3`, ändert `rust/parity/tests/api_http_paritaet.rs` und `rust/TESTMAP.tsv`) auf dem
   Zweig `orch/parity-binaer-kopie`, in Arbeit bei `p24a-bau` (laut main); der Zweig ist nicht in main
   (Stand 07:13: `git log --oneline 057b7ec3..orch/parity-binaer-kopie` → 1 Zeile, `git merge-base --is-ancestor eebe4578 bc092a74` → nein);
   (e) die Entscheidung selbst.
2. Reste aus den Berichten `haertung` und `json-leser`: alle vier Einträge (`python-schreibt-akte-die-der-rust-leser-sperrt`,
   `bindungsbereich-prueft-nur-der-browser`, `negativer-aufwand-umgeht-pflichtfrage`,
   `python-schreibt-ganzzahl-ueber-i64-in-die-fallakte`) liegen im Archiv (`backlog/archive/taxgraph/`, geprüft per
   `find ~/00_projects/vault -name '<slug>.md'`). Der Kodierungstest ist behoben (`cd6c789`); die Produktfrage „Protokollzeile
   vor die Antwort" ist gebaut (`6d4cc2c`, Python + Rust; `audit-status-spalte-bleibt-auf-500` liegt im Archiv). Die Doctest-Zählung
   `catala-sys` ist überholt: 27 Doctests, 27 passed (Gate-Log `gate-main-057b/4-cargo.log`, Abschnitt „Doc-tests catala_sys“;
   `git diff --stat 057b7ec3 bc092a74 -- rust/catala-sys` leer, also gilt es für `bc092a74`). Ob 27 jede `pub fn` deckt: nicht gemessen (die
   Nachmessung 2026-09-30 in § 7 nennt noch `catala-sys 0/26`).
3. Weitere Hebel für die Testzeit, laut Bericht `orakelcache`, nicht nachgemessen: `traverser.gate_gewicht` (etwa 70 %
   der Orakelzeit von `interview_paritaet`), `xsd_verify` (etwa die Hälfte von `elster_paritaet`), 50 s Wartezeit in
   `store_paritaet`/`tarif_paritaet` (Vault `research/taxgraph-bauzeit-vs-testzeit`). `make unit` dauerte auf `057b7ec3` 458,58 s (4405 Tests)
   gegenüber 188 s auf `b540590` (3878 Tests; `gate-main-057b/1-unit.log`), Ursache nicht untersucht.
4. Tickets: Von den sechs Entscheidungen für Julius, die hier am 2026-10-03 in Vault `tickets/` standen, liegt keine mehr
   dort (`find ~/00_projects/vault -name '<slug>.md'`): `p24a-rueckfall-nennt-falsches-feld` und
   `chat-ablehnungsgrund-enthaelt-den-wert` im Backlog-Archiv, `worktree-erbt-hook-mit-fremdem-testpfad` in
   `tickets/archive/`, `p32-6-kinderfreibetraege-verwaiste-regel`, `p32-abs6-satz-2-und-5-lesen-keine-regel` und
   `instanz-kennung-mit-suffix-eins-wird-verschieden-gelesen` in `backlog/taxgraph/` (48 Dateien, `ls ~/00_projects/vault/backlog/taxgraph | wc -l`,
   2026-10-04; 49 am 2026-10-03). Unqualifiziert in `tickets/` liegen jetzt sieben Taxgraph-Tickets (`grep -l 'project: "taxgraph"' tickets/*.md | wc -l`
   → 7; zwei am 2026-10-03): `fehlertexte-nennen-den-eingegebenen-wert-in-422-500-und-ablaufprotokoll`,
   `minus-null-im-json-text-ist-in-python-null-und-in-rust-minus-null`, `korpus-faelle-liest-ein-feld-das-apierror-nicht-hat` (Fix laut Merge `2ec08c52`
   gemergt, das Ticket liegt weiter in `tickets/`), `neues-params-jahr-ohne-vz-erweiterung-macht-in-rust-500` (2026-10-04, Commit `057b7ec3`),
   `p34-abs3-fuer-den-partner-fehlt-ganz` (2026-10-03; danach sperrt `0749aab3` den Gewinn beim Ehegatten, ob das das Ticket schließt: nicht geprüft),
   `testmap-zeilen-spalte-veraltet-und-kein-tor-prueft-sie` und `zwei-messungen-widersprechen-sich-bei-der-partner-vorsorge` (Titel: „Vier
   Messungen an einem Fall, drei Richtungen: keine Rust/Python-Differenz, aber ein Loch in beiden Sprachen“).
5. In Arbeit laut Roster (`mcp__orch__roster`, 2026-10-04 07:13): `haertung8`, `k9`, `last-fehler`, `p24a-bau` (alle busy), `main` idle; `neunc` steht
   nicht in der Liste (2026-10-03 17:27 waren es `haertung8`, `k9`, `neunc`). Der frühere Stand
   (kein Worker; HTTP 400/429 bei allen außer `b1-2`) gilt nicht mehr.
6. Aufräumen für Julius: `wt-orakelcache`, `wt-orakelcache-vorher` und `wt-nachmessung` gibt es weiterhin nicht. Von den Namen des Stands 2026-10-03
   gibt es noch `wt-neunc`, `wt-k9` und `wt-haertung8`; `target-wt-neunc`, `target-wt-k9`, `target-wt-haertung8`, `target-b1-2`, `target-wt-k9fh` und
   `target-wt-rente` gibt es nicht mehr (`ls -d ~/.cache/taxgraph-tmp/target*` → 14 Verzeichnisse: `target-main`, `target-p24a`, `target-p24a-end`,
   `target-parity-kopie`, `target-last-fehler`, `target-wt-haertung8e/f/g/i`, `target-wt-k9-e2/-fnb/-iv/-mer`, `target-wt-ktr`; zusammen 88 GB,
   `du -sch ~/.cache/taxgraph-tmp/target*`). `git worktree list | wc -l` → 67 (33 am 2026-10-03); Verzeichnisse `wt-*` unter
   `~/.cache/taxgraph-tmp/`: 60 (`find ~/.cache/taxgraph-tmp -maxdepth 1 -type d -name 'wt-*' | wc -l`). `/home` ist zu 92 % belegt, 44 GB frei
   (`df -h /home`). Dazu die Arbeitsverzeichnisse der `neunc`-Aufträge mit eigenem `target`: `~/.cache/taxgraph-tmp/{kls,ktr,kzg,plan}` (0,74 / 5,6 /
   7,4 / 2,0 GB, `du -sh`; `plan` wächst mit jedem Lauf). Ob der Guard `git worktree remove` bei fremden Worktrees weiter blockt, ist nicht gemessen;
   bei einem eigenen sauberen Worktree (`wt-kzg-basis`) lief es am 2026-10-04 ohne Blockade. Der Symlink `oracle/.venv312` steht in `.gitignore`
   (`git check-ignore -v oracle/.venv312` → `.gitignore:8`), nie mit `git add -f` committen. `cargo fmt --all` (150 Hunks in 35 Dateien auf `2bc35bd4`,
   `cd rust && cargo fmt --all --check`) erst, wenn kein Zweig mehr offen ist; auf `bc092a74` nicht neu gemessen (Auftrag: kein `cargo fmt`).

| Schritt | Stand | Commits |
|---|---|---|
| 1–6, 8 | fertig: catala-sys, domain, bindung, store, engine, interview, konsistenz, intervall, elster, auth, llm, eingang | bis `f165889` |
| 7 `bescheid` | fertig | `75d8955` abzuege/einkuenfte · `5c6418b` deklaration · `da0cc0d` zweige + `bescheid_fn` |
| 9a `api`-Gerüst | fertig: 24 Routen (9 fertig, 15 Stubs `501 nicht_portiert` — Stand 9a; heute sind alle 24 portiert, Zeile „9c `api`-Handler“), `EigenerFall`, HTTP-Differenz-Harness | `054a281` |
| 9b-A Härtung additiv | fertig für bescheid, engine (Doctests, `debug_assert!`, Properties; kein `f64` für Geld in `bescheid` und seit K7c-2 (`33f56ad`) in `engine`); Doctests in catala-sys, eingang, elster, llm, interview | `b149f1f` · `9e2b00d` · `b047e7c` |
| 9b-F Fuzz | fertig: `rust/fuzz` (nightly, nicht im Workspace), 5 Targets | `d2e1e15` |
| Format | `cargo fmt --all`, `llm_dialog` geteilt | `2f9cd2d` |
| Steuerzeichen-Fix | fertig in Python **und** Rust: Abweisen beim Speichern, `erzeuge_xml` fail-closed, Fuzz-Skip entfernt | `2c17f70` |
| Vorjahr überspringt Altwert | fertig in Python und `eingang` (die Rust-Route war bis 9c ein Stub; seit `82bbf1e` ist `vorjahr` portiert) | `e64a8c4` |
| Null in Kz ohne Null | fertig, Py **und** Rust: 384 echte Fälle, ungültig 214 → 38, Abweichungen 0 (Vault `decisions/elster-null-in-kz-ohne-null-weglassen`) | `3c7bb01` |
| Textformat aus XSD beim Speichern | fertig, Py **und** Rust | `529eaa3` |
| **9b-B Typisierung** | K0 fertig (Vault `research/taxgraph-wertformen-echte-stores`); **K1 `domain` fertig** (794 Rust-Tests, 19 Parity-Suiten, clippy 0, Mutation 2/2 rot); **K2 `store` trägt `PyWert` — gemergt** (`PyWert::zu_json` gibt `Result`, NaN/inf → `PyFehler::DezimalGrenze`, 56 Dateien); **K3 `interview` gegenstandslos** (READ-ONLY, 0 Events, kein Produktpfad-Konsument); **K4 `konsistenz` liest die Veranlagung typisiert — gemergt** (Fassung A: `Option<Veranlagung>` + `bool abweichend`, Log #119; Gegenprobe 1 rot); **K5 `elster` gemergt** (Veranlagung über `Lage`, `Konfession`/`Rentenart` als `domain`-Enums, Kz-Regel nur in `domain::Kz`); **K6 `intervall` gemergt**; **K7a `bescheid`-Helfer gemergt** (`VeranlagungWert` über `Lage`, exhaustiv; `py_wahr` entfernt); **K7b gemergt** (Konfession/Rentenart in `bescheid` über `Lage`, exhaustiv; Bundesland → K9 mit `engine::KistEingabe`; `Quantitaet` nur gemessen, 8 Stellen, nicht gebaut); **K7c `f64` für Geld erledigt** seit `b149f1f`/`0139fcc` (gemessen: 10 Treffer, 1 Produktionsstelle = Eingang aus `PyWert::Gleit`, Fallzahl 0); **K7c-2 gemergt** — `engine` rechnet Sätze exakt (`int_mal_satz` über `zu_bruch`, i128 statt `f64`), `domain::Satz`/`domain::Km` als eigene Typen, `Km::volle_km` schneidet ab, `zehntel` rundet den Dezimalwert; Gegenproben: `f64` zurück → rot, Satz/Km getauscht → E0308. Die Lücke (`round` statt `volle_km` fing nur der Paritätslauf, 63/1000) schließt **K7d gemergt**: ein Unit-Test rechnet 103 Tage zu 101,5 km (VZ 2026) und erwartet 3170 € wie das Python-Orakel, gerundet käme 3171. `eingang`/`konsistenz` bleiben wie Python (Vault-Nachtrag `rust-port-geld-cent-saetze-decimal`; Kontoauszugbetrag wird mit Vault `decisions/kontoauszug-betrag-cent-genau-oder-verworfen` ganzzahlig); **K8 Tor 1/2 gemergt** (`d1c5e00`, Rest in §7), danach K9 | `1f01bc7` K1 · `ee46881` K4 · `bfb6a35` K2 · `7f81242` K6 · `91204fd` K7a · `200415d` K5 · `b9b7b10` K7b · `e94ba09` K7c-Doku · `33f56ad` K7c-2 · `6f283ad` K7d |
| auth ohne stille Fehlerpfade | fertig: Sperrliste erholt sich von vergiftetem Mutex, `py_json` liefert `Result` statt still `""` | `22459db` |
| Unlesbarer CSV-Betrag | fertig, Py **und** Rust: unlesbare Zeile zählt in `verworfen` mit Hinweis, Betrag ganzzahlig ohne `float`/`f64`, über i64 oder 4300 Ziffern verworfen statt 500; der Rust-HTTP-Test lief bis zur Portierung der Route unter `#[ignore]` und läuft seit `91bfd72` ohne (Vault `backlog/archive/taxgraph/kontoauszug-zeile-mit-unlesbarem-betrag-verschwindet-still`). Negatives Geldfeld → Vault-Backlog `negativer-aufwand-umgeht-pflichtfrage`, nach dem JSON-Leser | `065fc93` · `215b046` |
| Aufwands-Einzelposten runden auf | fertig, Py **und** Rust: 14 Einzelposten-Kz runden wie ihre Summen auf, § 35a-Summen aus den gerundeten Posten; ERiC rc=0 statt 610001002 (Vault `decisions/aufwand-einzelposten-aufrunden-summe-aus-posten`) | `a46e525` |
| Kaputtes PDF / Beleg-Regex | fertig, Py **und** Rust: `pdftotext`-Exit ≠ 0/3 (auch fehlende Datei, Exit 1) → `/kontoauszug` 422 statt 0 Buchungen; Nr-Anker-Regex einmal je Nummer (≈ 40×). Reiner Scan als Beleg liest leer: Vault-Backlog `beleg-pdf-ohne-textlayer-wird-leer-gelesen` (kein Produktpfad; Wächter `f80187db`: kein Aufrufer von `lies_beleg_text` unter `produkt/`) | `908820e` |
| Stille Schema-Skips | fertig: alle Rust-Tests, die das ERiC-Schema brauchen, fragen `elster::testhilfe::schemas_da` — ohne Schema rot, Skip nur mit `TAXGRAPH_OHNE_XSD=1`. Der XSD-Pfad läuft ohne Schema gegen eine selbst gebaute Mini-XSD, der leere Art-Test verlangt eine nicht leere Menge (`94b5319`). Der Rundungs-Sweep prüft die Abzugs-Kz beider Listen, ein Test verlangt Mengengleichheit (`d118ecf`). Offen: der Zufallsgenerator trifft vier der neun § 35c-Kz nie (Vault-Backlog `fuzzer-generierte-stores-erreicht-vier-p35c-kz-nie`; je Sanierungsart ein handgebauter Paritätsfall seit `3fb18c0`) | `86e9c91` · `94b5319` · `d118ecf` |
| Veranlagungsjahr in Rust-`elster` | fertig: `deklariere(snapshot, bindung, vz, id)` wählt die Null-Verbots-Liste je Jahr wie `est_mapping.null_unzulaessig` (0 → Fehler; 2024/2025 eigene Menge; 2026–2100 Vereinigung; sonst Fehler). `elster_paritaet reale_faelle` 4 → 0 Abweichungen (fall#52/#53 eg_huge/eg_neg). Gegenprobe am Aufrufort rot. Bericht `~/.cache/taxgraph-tmp/berichte/vz.md` | `d8d5f1f` · `faef9ee` |
| End-to-End Eingabe → Bescheid → ELSTER-XML | fertig, byte-gleich Python, 3 Fixtures VZ 2025 gegen XSD | `479deb9` |
| Python-xfail → Rust `#[ignore]` | fertig: 24 Gegenstücke in `rust/{api,bescheid,elster}/tests/offene_defekte.rs` (`90f91aa`). **Auf `bc092a74` stehen dort 21 `#[ignore]`-Tests** (api 1, bescheid 10, elster 10; `grep -cE '^\s*#\[ignore' rust/*/tests/offene_defekte.rs`; ebenso auf `cbc376a6`, auf `2bc35bd4` waren es 22 mit bescheid 11): `api::deklaration_umgeht_den_waechter_nicht` (`436521e`), `bescheid::kap_vorlaeufiger_topf_leckt_in_deklaration` (`69119b9`) und `bescheid::verpflegung_vorlaeufige_tage_lecken_in_deklaration` (`cbc376a6`) laufen ohne `#[ignore]`, weil der Defekt behoben ist. Unter `--ignored` rot am Defekt — Ausnahme `p23_eric_prueft_zwei_verkaeufe`: im Hauptbaum grün, weil `.env*` eine echte Hersteller-ID trägt (Umgebungs-Gate, im `#[ignore]`-Grund benannt); `test_datenwurzel_ausserhalb_repo` als grüner Rust-Test (`api/tests/datenwurzel.rs`); TESTMAP nennt je xfail das Gegenstück. Unter K2 kompilierbar erst mit `faef9ee` | `90f91aa` · `faef9ee` |
| GitHub-CI wieder grün machen | drei Ursachen, je eine behoben: Quell-Hash locale-abhängig (`LC_ALL=C`, `e94357f`); `ttsim-backend` ungepinnt (`==1.2.1`, eine Pin-Quelle + Wächter, `1993b8d`/`f057f70`-Merge); fehlendes ERiC-Schema → laut rot, Skip nur mit `TAXGRAPH_OHNE_XSD=1`, `ci.yml` setzt es (`5b77799`). Der `rust`-Job wäre auf `5b77799` an E0308 + 6 Clippy-Fehlern aus `90f91aa` rot gewesen, behoben in `faef9ee`. **Gepusht mit `beef16b`; Lauf 36985004342: vier von fünf Jobs grün, `rust` rot an einer vierten Ursache:** die drei `parity`-Lib-Tests liefen ohne PARITY-Gate und fanden `pkg` nicht, `cargo test` brach ab, die Ziele danach liefen nicht. Behoben mit `1e3f6fb` (Lib-Tests überspringen laut, poppler im `rust`-Job, `--no-fail-fast`). **Lauf 37010730963 auf `8e48cf7`: alle fünf Jobs grün.** Vault `backlog/archive/taxgraph/github-ci-seit-20-08-rot` | `e94357f` · `5b77799` · `bb41b61` |
| Wackliger Test durch Ablage-Leck | fertig: `audit.AUDIT_DIR`/`api.FAELLE` enden an der Testdatei (autouse-Fixture in `tests/conftest.py`), Ursache pytest-randomly | `d1b0422` |
| Pflegegrad im XSD-Enum | fertig, Py **und** Rust: 5→4, Block-Wegfall ohne H, nur E0161606 weg mit H; xmllint invalide 38 → 22; Testloch Enum-Schranke geschlossen (Vault `decisions/pflegegrad-ausserhalb-des-schemas-abbilden-oder-weglassen`) | `29d66c2` |
| Wertformen-Messwerkzeug | fertig: `re.fullmatch` statt `re.match`, Regressionstest in `tests/` (Vault `decisions/wertformen-prueft-format-ueber-den-ganzen-wert`) | `3436fa9` |
| Rente über 97, Rentenbeginn nach dem VZ, § 35a | fertig, Py **und** Rust: ein Alter bei Rentenbeginn über 97 nimmt die Zeile „ab 97" statt HTTP 500; ein Rentenbeginn nach dem VZ sperrt benannt mit `rentenbeginn_nach_vz` (Sperrgrund 53) statt 500, erst für aa (`5275dc3`), mit `4d6dd9a` auch für die bb-Leibrente; § 35a rechnet die Catala-Regel in Cent und rundet erst die Summe ab (`db2de9f`). 78 `xfail(strict)` hielten fest, dass der Server Zahlen außerhalb `bereich` annimmt (`c97662a`, Vault-Backlog `bindungsbereich-prueft-nur-der-browser`); behoben mit `bb7bc8d` (Abweisen beim Speichern): `tests/test_bindung_bereich_serverseitig.py` trägt 0 `xfail` (`grep -c xfail`) und läuft grün (149 passed auf `bc092a74`, `python3 -m pytest tests/test_bindung_bereich_serverseitig.py -q`; früher 119), der Backlog-Eintrag liegt im Archiv. Echte Akten: 0 von 192 ändern sich | `a826efb` · `f1e92db` |
| Partner-Hebesatz offen | fertig, Py **und** Rust: Zusammenveranlagung mit `gewst_messbetrag_partner` > 0 und unbeantwortetem `gewst_hebesatz_partner` sperrt mit `gewst_hebesatz_offen` wie bei Person A, statt still ohne Partner-Anrechnung zu rechnen (`45b0cb1`, Vault `decisions/partner-hebesatz-und-leibrente-nach-dem-steuerjahr-sperren-wie-ihr-gegenstueck`). Hebesatz 0 oder darunter bei Messbetrag > 0 sperrt wie ein fehlender, A und B: `d0de8f5`, `e46d4f5`, Klartext `48d009e`, gemergt mit `16cc0fd` | `f1e92db` · `d0de8f5` |
| 9b Rest | gemergt: `store` (`9cbaf76`: 39 Doctests, Eigenschaften P1–P6, `debug_assert!` „ein aktives Event je Feld"); die zwei `#[ignore]`-Befunde behebt `d4babec`: `EventId::parse` weist A–F und `+` ab (`1359d13`), `store::lade` liest JSON statt YAML, NaN/±Infinity/±1e400/Ganzzahl außerhalb i64/u64 sperren mit `PersistenzFehler::Sperrform` (P11), die Python-Tür weist nicht endliche Zahlen mit 400 ab. auth, catala-sys, llm, eingang (`cb275b4`); konsistenz (`3da715e`: `debug_assert!` D1/D2, zwei Eigenschaften); interview, intervall (`4ec2d2f`: je ein `debug_assert!` und eine Eigenschaft über 1000 Fälle); engine (`a941ea7`: Doctest `tarif::fuenftel`, Eigenschaften SolZ-Gleitzone und GewSt-Messbetrag; dazu verlangt `schema.json` genau eines von `wert`/`wert_nicht`). Tür nach dem JSON-Leser: nur UTF-8 ohne BOM, ein Schreibfehler hinterlässt keine `*.tmp`, NaN-Login ohne Abweichung (`3f27bb8`); ein doppeltes Struct-Feld sperrt die Akte, `api.py`-Zeilenratsche zurück auf 1324 (`2b53e64`; heute `API_ZEILEN_OBERGRENZE = 1342` in `tests/test_bescheid_grenze.py:262`, `wc -l produkt/haut/api.py` → 1342). `api` mit 9c: erledigt (Zeile „9c `api`-Handler“); der Wächter `test_ci_konfiguration` prüft die installierte Fassung für `gettsim` **und** `ttsim-backend` (`b2b4ae7`, `tests/test_ci_konfiguration.py:297`) | `d4babec` · `3da715e` · `4ec2d2f` · `a941ea7` · `3f27bb8` · `2b53e64` |
| Format | `cargo fmt --all --check`: 150 Hunks in 35 Dateien (`cd rust && cargo fmt --all --check`, gemessen auf `2bc35bd4`; auf `bc092a74` nicht neu gemessen, Auftrag ohne `cargo fmt`), bewusst vertagt (CI prüft nur clippy: `.github/workflows/ci.yml:320-321`; Log #165) | — |
| dev-Profil nur Zeilentabellen | fertig: `[profile.dev] debug = "line-tables-only"` statt `debug = 2` (Auftrag Julius, Plattenplatz), `profile.test` erbt. Backtraces behalten Datei und Zeile, der Debugger sieht keine Variablen. `libstore-*.rlib` 11 804 166 B → 5 428 230 B (auf 46 %); clippy 0, `cargo test -p store` grün, Gegenprobe ohne Abschnitt → `debuginfo=2`. PARITY 17/17 grün auf `8e48cf7` | `bb01e0f` · `beef16b` |
| Orakel liest eine Bindungsdatei einmal | fertig: `traverser.lade_datei_felder` ist je Prozess gecacht, `api._datei_felder` und beide Orakel-Skripte nutzen sie. YAML-Lesevorgänge 1509 → 32. `bescheid_deklaration_paritaet` 205 → 91 s laut Worker-Bericht, 86 s in der Nachmessung auf `8e48cf7`; die anderen 16 Suiten nicht schneller. Kosten: eine geänderte Bindungs-YAML wirkt erst nach Neustart des Prozesses (wie `lade_bindung`). Gegenproben G5/G6 in der Nachmessung je rot; Vault `research/taxgraph-bauzeit-vs-testzeit` | `6bba00d` |
| 9c `api`-Handler | Handler fertig (gemessen auf `2bc35bd4`, Routenzahl auf `bc092a74` nachgemessen): alle 24 Routen portiert (Routentabelle des Harness: Python 24, Rust 24, Befehl oben); alle GET-Routen gemergt (`8d1bd96`), POST-Routen `event`, `kontoauszug`, `vorjahr` (`82bbf1e`), `chat` und `entfernung` (`c949f86`), `einreichen` (`8c89556`; nur `ERIC_VALIDIERE`, kein Versand); 9c/0b (`Username`/`FallId`-Newtypes im Owner-Check) gemergt (`0b31195`), 9c/0c und 9c/0e gemergt (`ecdfcb9`); Landkarte gemessen (15 Routen, Bericht `berichte/9c-karte.md`); Harness-Generator je Route fertig (`88f60c7`: echte Eingaben für 11 Routen aus Stufe 1–3); `NICHT_PORTIERT` entfällt (`d9d51a6`), eine Antwort `501 nicht_portiert` ist im Harness eine Abweichung (`d432948` in `4ec2d2f`, danach `d9d51a6`); `flow` ist portiert (Python-Teil `1321f3b`, Rust-Teil `ecdfcb9`) und der Mitschnitt `flow.jsonl` wird in `Modus::Voll` verglichen (Lauf auf `4368ebb` mit `--nocapture`: 60 bzw. 977 Zeilen in zwei Abschnitten, sonst 0); Vorbedingung JSON-Leser erfüllt (`d4babec`). Die Abnahme „Kontrakttest, Playwright gegen Rust“ (§7, Schritt 9c) stand hier offen (Vault `decisions/rust-9c-generator-je-route-und-flow-portieren`); den Lauf gibt es seit `081e1d94` als `make ui-rust` (Punkt „Seit `2bc35bd4` gemergt“: 249 passed, 23 xfailed auf `5d212c31`, auf `bc092a74` nicht neu gelaufen, nicht in der CI) | `88f60c7` · `4ec2d2f` |
| 10 Cutover | offen, Entscheidung Julius; Voraussetzungen unter „Offen“ 1 | — |

Gates auf `8e48cf7` (2026-10-02 15:05, Instructor nachgemessen im frischen Worktree `wt-nachmessung`, Log
`~/.cache/taxgraph-tmp/nachmessung-neustart.log`): `make unit` 3548 passed / 0 failed (79 skipped, 102 xfailed,
151 s); `make golden` 135/135; clippy rc=0; `cargo test --workspace` ohne `parity` 960 passed / 0 failed /
26 ignored; 17/17 PARITY grün (längste: `interview` 160 s, `bescheid_zweige` 156 s, `elster` 87 s,
`bescheid_deklaration` 86 s); Gegenproben G1–G6 je rot, danach zurück und sauber; echte Akten 197 unverändert;
Docker-CI-Nachbau (`git archive 8e48cf7`) 68 Ziele, 1087 passed / 0 failed / 26 ignored.
Gates auf `a941ea7` (2026-10-02, Instructor nachgemessen, Merge-Nachricht `2b53e64`): 17/17 PARITY grün,
`make golden` 135/135, clippy rc=0, `cargo test --workspace` 957/0, Gegenproben G1–G9 je rot, echte Akten 197
unverändert. `2b53e64` (Kommentar plus ein Unit-Test) und `beef16b` (dev-Profil) ohne eigenen PARITY-Lauf.
Gates auf `faef9ee` (2026-10-01 23:51, Instructor nachgemessen): `clippy --workspace --all-targets
-D warnings` exit 0; `PARITY=1 cargo test --workspace` 992 passed / 0 failed / 26 ignored über 66
Testbinaries; `rust/fuzz` `cargo check` grün; `make unit` 3478 passed / 0 failed (auf `5b77799`, außerhalb
`rust/` kein Diff zu `faef9ee`). Gates auf `2f9cd2d`: `cargo build`, `clippy`, `fmt --all --check`,
727 Tests, `cargo +nightly fuzz build`, alle 17 Parity-Suiten (`PARITY=1`) grün. Volle Nachmessung
≈ 20 min; Skript-Muster: jede Suite einzeln `PARITY=1 cargo test -p parity --test <name> -- --test-threads 3`.

**Für 9c festgehalten (aus dem Steuerzeichen-Fix, `2c17f70`):**
- Abweisung eines Textwerts: `/event`, `/entfernung` → 422; `/chat` → Eintrag in
  `abgelehnt_gruende`; `/einreichen` → 422 `{"eingereicht": false, "grund": "xml_nicht_baubar", "detail": …}`.
- `/vorjahr`: Python erzeugt keinen Teilimport, weil erst geprüft und dann gespeichert wird. Der Writer ist
  aber nicht atomar — der Rust-Handler muss diese Reihenfolge übernehmen.
- `/vorjahr` seit `e64a8c4`: eine Typ-/Format-Abweisung überspringt das Feld, Antwort 200 mit
  `uebersprungen` (nur feld_ids); jede andere Abweisung 422. Der Handler gibt
  `VorjahrErgebnis.uebersprungen` weiter (Vault `decisions/vorjahr-unpassenden-altwert-ueberspringen`).
- `/chat`: der Grund nennt Klasse und `feld_id`, nie den Wert: entschieden und gebaut mit `89eff75` (Python und Rust; Vault-Backlog
  `chat-ablehnungsgrund-enthaelt-den-wert`, `abschluss: gebaut`, im Archiv).

**Entschieden (Julius, 2026-10-01), gebaut mit `7bc78493` (Merge `d635627c`, 2026-10-03):** Das ELSTER-Schema erlaubt in Textfeldern nur den
Zeichensatz „Standard_E_V2", strenger als XML 1.0. Ein Zeichen ausserhalb wird beim Speichern
abgewiesen, die Meldung nennt Zeichen und Vorschlag, Py **und** Rust (Vault
`decisions/elster-zeichensatz-beim-speichern-abweisen`, Backlog `elster-zeichensatz-strenger-als-xml`; der Backlog-Eintrag liegt weiter in `backlog/taxgraph/` mit fünf offenen Kästchen AK1–AK5,
`grep -c '\- \[ \] AK'` → 5; Code: `rust/domain/src/zeichensatz.rs`, `rust/store/src/abweisung.rs`, `produkt/store/zeichensatz.py`).

**Offen aus 2026-10-01 (gemessen, noch nicht entschieden):**
- **Nutzerpfad-Karte** (`sperre`; im Roster vom 2026-10-04 steht kein Arbeiter `sperre`): `rust/interview` ist fertig portiert und trägt **keinen** Nutzerpfad. Falls das für weitere Crates gilt, ist „fertig" in der Tabelle oben eine Aussage über den Code, nicht über das Produkt. Nachgeprüft am 2026-10-04 (die Karte
  `berichte/nutzerpfad-karte.md` ist vom 2026-10-01 und nicht neu gemacht): `rust/api` hängt an `interview` (`rust/api/Cargo.toml:25`, seit `199bbeca`;
  Aufrufe in `rust/api/src/ergebnis.rs` und `fragen.rs`), und `taxgraph-api` startet nicht mehr nur ein Parity-Test: `scripts/starte-api.sh`
  (`cd8687d3`) und `make ui-rust` (`Makefile:235-253`) starten es (`grep -rn 'taxgraph-api'` über Makefile, scripts, tools). Ob ein
  Produktionsweg es startet: nicht gemessen; produktiv läuft bis zum Cutover Python. Die Karte (13:35 Uhr) ist älter als `cd8687d3` (13:52) und `199bbeca` (13:59) vom selben Tag. Von ihren 14 Bausteinen sind hier nur `interview` und
  `api` nachgeprüft; die übrigen 12: nicht neu geprüft.
- **Rentner-Scheibe** fragt Lohnsteuer ohne die zwei ERiC-Pflichtfelder. Beide naheliegenden Ausgänge falsch; entschieden ist (c): Ring zuerst. Vault-Backlog `rentner-scheibe-fragt-lohnsteuer-ohne-die-zwei-pflichtfelder`, `naht` leitet die Naht-Paare aus dem XSD ab. Auf `bc092a74` nicht neu geprüft.
- **Audit-Leck** (Fix `d1b0422`, Zeile „Wackliger Test" oben): `API.FAELLE` und `audit.AUDIT_DIR` sind zwei Modul-Globals, die halbe Isolierung leckt 1145 Zeilen ins Nutzerverzeichnis. 12 von 169 Wegwerf-Skripten tragen das Muster. Zweites Ticket: die `status`-Spalte steht bei fünf Aktionsnamen auf 500 trotz 409/422 am Client — gebaut mit `6d4cc2c`.
- **Parity-Blindstellen** (Vault `decisions/parity-elster-vergleich-gegenstandslos-fuer-ring`): der ELSTER-Vergleich nimmt auf beiden Seiten denselben verkürzten Weg — für die Ring-Injektion 0 Aussage; die Komposition `mit_ring_werten → elster::deklariere` hat für Rust keine Zeile. `intervall`/„B alle" ist **strukturell leer** (299 von 366 Achsen nullen die Zeile, 192/192 NULL). Seit `604022c8` (laut Merge) ist ein Block ohne beurteilte Zeile rot, `zugriff_teil1`/`zugriff_teil2`
  und `interview` zählen Werte statt Fälle, leere Zeilen sind gepinnt; `interview` war bei leerem Korpus grün. Ob „B alle“ noch strukturell leer ist: nicht neu gemessen.
**Kleinere offene Befunde:** Float-Rentenfreibetrag
fehlt im Parity-Korpus (nur Unit-Tests); reale Fälle decken Kinder/§ 23/DBA kaum (Golden-Fälle vor Cutover);
Mutationen überleben bei § 31-Gleichstand und `true` im Rentenbeginn-Jahr (Stand 2026-10-03, auf `bc092a74` nicht neu geprüft). Neue
Mutations-Überlebende seit `2bc35bd4` (laut Merge, nicht nachgemessen): `0aa91677` 10 von 53, alle Äquivalenzen (`m47` `Gesamt.rentner=true` ist tot);
`b13a0669` 6 von 22, alle `floor`/`ceil`, klassifiziert; `a966b94c` 8 von 127 (5 `unreachable`, 2 `equivalent`, 1 ungeklärt: A3, `api/src/stand.rs`,
`IntervallFehler::Ueberlauf` → `ValueError`, Bericht `berichte/h8-hermetisch5.md`).

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
| 148 YAML-Dateien | **525** getrackt (`2bc35bd4`: 527, `git ls-files '*.yaml' '*.yml' \| wc -l`); das Produkt lädt zur Laufzeit **91** (bindung_* 25, params 58, kohorten 8; auf `2bc35bd4` unverändert, `ls` je Verzeichnis) | explicit (`git ls-files`, Lader-Grep) | Phase 2 lädt die 91 typisiert. Die 148 sind vermutlich `sources/**/*.meta.yaml` ohne `bfinv` (derived) — Tooling, nicht Produkt. |
| ELSTER-XML in `produkt/` | Writer in `produkt/eingang/elster_xml.py`, ERiC in `elster/` (1 779 Z.) | explicit | `elster/checkest_gate.py`, `smoke_test.py`, `submission/validate_xsd.py` gehören in den Port. |
| 22 xfail-Tests | stimmt: **22 xfailed** zur Laufzeit (`make unit`, 2026-09-29) aus **15** Markern in **13** Dateien (parametrisierte Marker zählen je Fall) | explicit | Alle 22 Fälle werden `#[ignore = "…"]` mit Grund. |
| Catala-Backend für Rust | Catala 1.2.1 hat **kein** Rust-Backend (c, java, python, ocaml, interpret) | explicit (`catala --help`) | C-Backend + FFI, §3. |
| `catala_*` = Catala | **30 von 70** `catala_*`-Funktionen rufen Catala, **40 sind Hand-Python** (`catala_solz`, `catala_kist`, § 35a, § 23 …) | explicit (AST-Scan `runner.py`) | Die 40 werden von Hand portiert und per Parität gesichert — dort gibt es keine Regelquelle. |
| Tests 301 Dateien | 301 `.py` unter `tests/`, 297 davon `test_*`; 75 075 Zeilen (`2bc35bd4`: 328 / 323 / 82 187, `git ls-files 'tests/*.py'`) | explicit | §6. |

---

## 2. Modulkarte

Produktcode: `produkt/` 36 Dateien / 17 017 Z. (`2bc35bd4`: 17 767, `git ls-files 'produkt/*.py' \| xargs cat \| wc -l`), `elster/` 9 / 1 779 Z. `pipeline/` 26 / 6 528 Z.

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
| Fehlende Angabe sperrt statt 0 | 47 Sperrgrund-Literale `bescheid_deklaration.py:781-1443`; 152 × `.get(…,0)` in `runner.py` | `enum Sperrgrund` (47 Varianten beim Entwurf; auf `7ec1ccd8` 58: 57 Sperrgründe und `Bestaetigt`, gezählt in `rust/domain/src/sperrgrund.rs`; exhaustiver `klartext`); Eingabe-Structs je Scope **ohne** `Default` |
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
| P10 | 32 echte Fälle tragen Alt-Herkunft `{"herkunft": …}` ohne `pruef_tiefe`/`haftung`; 5 Fälle haben VZ 2099, −5, 10^38 (API-Sicherheitstests) | Rust-Store lädt sie seit `f165889` (`rust/parity/tests/store_paritaet.rs:159` und `:193`: „store::lade sollte alle realen Faelle laden“) — Entscheidung: tolerant laden wie Python, keine Migration fremder Nutzerdaten |
| P11 | Fallakte mit `NaN`, `±Infinity`, `±1e400` oder Ganzzahl über `u64` unter `events`: Python liest still eine Zahl, Rust sperrt die Akte mit `PersistenzFehler::Sperrform` (Zeile, Spalte, Form) — keine falsche Zahl in der Rechnung; real 0 von 192 | `api.py:138-143` (`json.load`); Tests `b4_nan_infinity_und_ueberlauf_sperren_mit_namen`, `zahlform_im_text_und_grosser_vz_laden` (`rust/store/src/persistenz.rs`); Vault `decisions/fallakte-mit-nan-oder-ueberlauf-sperrt-mit-namen` |

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

Stand `607aedb`. Auf `2bc35bd4` zählt `tests/` 328 `.py`-Dateien mit 82 187 Zeilen, auf `cbc376a6` 330 mit 82 481, mit dem Zweig `orch/testmap` 331 mit 82 557 (Befehl: `ls tests/*.py | wc -l`, `cat tests/*.py | wc -l`); die Tabelle ist nicht neu klassifiziert. Die Karte `rust/TESTMAP.tsv` führt seit `3342d515` auch Rust-Dateien: 397 Zeilen = 331 mit Präfix `tests/` plus 66 mit Präfix `rust/`. Die Zahlen der Tabelle unten sind Python-Zahlen (Präfix `tests/`), nicht die der ganzen Karte. Die Spalte „Dateien“ der UI-Zeile zählt 20; `make ui-rust` startet 22 Dateien mit `sync_playwright` (Stand `cbc376a6`).

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
damit genau der End-to-End-Test „Frontend gegen Rust-API" aus Phase 4. Der Lauf steht als `make ui-rust`
(Abschnitt Fortschritt); die Liste der 23 Tests, die gegen Rust nicht grün werden können, ist
`tools/ui_rust/ausschluss.tsv`.

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
**K8-Stand auf `8e48cf7`: Tor 1/2 gemergt (`d1c5e00`, Spitze `99f50b6`, Basis `ba6ec34`).** Tor 1
(`_ =>` auf Domain-Enums): Workspace-Test `rust/api/tests/domain_enums_exhaustiv.rs` über clippys
`wildcard_enum_match_arm`; eine Ausnahme offen (`OFFEN`, `rust/api/tests/domain_enums_exhaustiv.rs:23`: der Arm `store/src/abweisung.rs:140`; K8 liegt jetzt auf main, der Arm fällt mit der `OFFEN`-Zeile in einem Commit) — **erledigt in `80743f2`**.
Tor 2 (`serde_json::Value`): `clippy.toml` verbietet `Value` und `Map` in `bescheid` und `konsistenz`;
`intervall` nutzt `Value` nur in Testmodulen, `interview` weiter im Produktcode (`interview/src/fragen.rs:121`,
gemessen auf `99f50b6`) — für `interview` war das Tor dort nicht erreicht.
**Seit `b540590`:** Tor 2 sperrt auch `interview` (`rust/interview/clippy.toml`, Merge `97d3846`), und die Liste
`OFFEN` in `rust/api/tests/domain_enums_exhaustiv.rs:23` ist leer (Merge `80743f2`). Beides gelesen am 2026-10-03,
Rust-Gates grün (Absatz „Gates auf `b540590`" oben).
**K9 Stufe 1 gemessen** (Worker `k7b`, Bericht `berichte/k9-karte.md`, nicht nachgemessen): eigene Typen nur,
wo heute ein falscher Text still durchrutscht (Vault `decisions/k9-typen-nur-wo-heute-ein-falscher-text-durchrutscht`).
Stufe 2 in den Commits 0–4: `bindung` Kz → `elster` `Vz` → `domain`+`elster` `&Kz` →
`elster` `deklariere(.., Option<&EventId>)` → `konsistenz` `BasisId`. Commit 5 (`bescheid_fn(Quantitaet)`)
entfällt; Nachschlage-Schlüssel bleiben Text, alle 26 statt der 8 unter „Entscheidungen".
Entscheidungen: `Lage<T>` nur für Enum-Felder (Veranlagung, Konfession, Bundesland, Rentenart) und
Cent-Summen in `bescheid`, voller bindungstypisierter Snapshot nach Cutover · `auth`/`audit`-Newtypes
(`Username`, `FallId`) in 9c mit den API-Handlern · Listen/Objekte und Ganzzahlen > u64 in Fremddaten
bleiben ladbar, Grenze dokumentiert · `PyWert` in `domain` · 8 Nachschlage-Schlüssel bleiben `&str`.
Korrigierter Messstand: 92 `pub fn` mit `&str` (≈32 tragen Regeln); 75 `_ =>`, davon 7 auf Domain-Enums,
35 auf `Value`, 28 legitim. Grob 13–18 Worker-Läufe; Risiko hoch bei K2 (`event_id`) und K6/K7a (Breite).

Arbeitsregeln ab jetzt: ein Commit je Schritt; nach jedem Schritt `cargo build`, `cargo clippy -- -D warnings`,
`cargo test` und **alle** Parity-Suiten; jeder nicht portierte Python-Test nennt in Commit-Nachricht und
`rust/TESTMAP.tsv` den Typ oder die Property, die ihn ersetzt; jede neue Testdatei in `tests/` oder
`rust/*/tests/` braucht eine Zeile in `rust/TESTMAP.tsv`, sonst ist `make unit` rot
(`tests/test_testmap_vollstaendig.py`); Routen, Payloads, Fehlerformate,
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
