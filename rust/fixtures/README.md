# `rust/fixtures/`

`golden_cases.json` — Extrakt von `golden/cases/*.yaml` (135 Faelle), erzeugt von
`tools/parity/extract_golden.py`. Kein Rust-Runner konsumiert diese Datei in Schritt 1
(„Foundation") — sie liegt bereit fuer den Golden-Runner ab Schritt 4 (`REWRITE_PLAN.md` §7).

## Format

Ein JSON-Array, ein Objekt je Fall:

```json
{
  "id": "an_2025_einzel_ep",
  "beschreibung": "...",
  "sachverhalt": { "bruttoarbeitslohn": 40000, "veranlagungszeitraum": 2025, "...": "..." },
  "erwartung": { "festzusetzende_est": 6629 },
  "quelle": { "authority": "gesetz", "fundstelle": "...", "zitatanker": "..." }
}
```

- `sachverhalt`/`erwartung`: unveraendert aus dem YAML uebernommen, ein freies Dict je Fall
  (keine feste Feldliste — jeder Fall traegt nur die Schluessel, die er braucht).
- `quelle`: die Zitat-/Herkunftsangabe, fuer die Anker-Verifikation gegen `sources/`.

## Einheit: EURO, nicht Cent

Alle Zahlenwerte stehen wie im Quell-YAML in **vollen EURO** (`zu_versteuerndes_einkommen:
100000` = 100.000 €), nicht in Cent.

Ein blindes ×100 auf jeden Integer waere hier falsch: `sachverhalt`/`erwartung` sind freie
Dicts ohne Schema, das Geld- von Nicht-Geld-Schluesseln unterscheidet (`veranlagungszeitraum:
2025` ist ein Jahr, kein Betrag). Die Cent-Umrechnung braucht die `domain`-Typen (welches Feld
ist `Cent`, welches nicht) — die entstehen erst mit dem Golden-Runner in Schritt 4. Bis dahin
bleibt der Extrakt 1:1 zum YAML, und jeder Verbraucher rechnet selbst um, mit dem Wissen, was
Geld ist.

## Neu erzeugen

```
python3 tools/parity/extract_golden.py
```

## Eingefroren (Stufe 2)

Die Fixtures mit Antworten von Python — `interview_`, `konsistenz_`, `intervall_`, `wertwache_`, `eingang_`,
`zeichensatz_`, `api_stand_fragen_orakel.json`, `kz_tabellen.json`, `sperrgrund_klartext.json` und `e2e/` — sind
eingefroren. Ihre Erzeuger sind geloescht; der letzte Stand liegt im Verlauf (`git show 2dd056a6:tools/parity/<name>.py`).
Die Dateien werden nicht neu erzeugt und nicht von Hand geaendert. Eine gewollte Abweichung von Rust zu Python
steht als Eintrag mit Grund in einer Liste im Test; der Test verlangt, dass die Abweichung weiter besteht. Die
Abweichungsliste der Fixtures steht in diesem Verzeichnis (folgt mit Stufe 2, Phase B).

Ausnahmen: `golden_cases.json` (Extrakt aus `golden/cases/*.yaml`, `tools/parity/extract_golden.py` bleibt) und
`begleitfelder_formen.json` (von Hand gepflegt). `api_stand_fragen_orakel.json` wird ein Rust-eigener
Golden-Master (Stufe 2, Phase B).

## `interview_orakel.json`

Szenarien fuer die Crate `interview` samt den Antworten des Python-Orakels
(`produkt/traverser/traverser.py`, `produkt/haut/bindung_rollen.py`). Konsument:
`rust/interview/tests/orakel_werte.rs` (hermetisch, ohne Python). Je Szenario: Events (Reihenfolge ist
Semantik), eine Sicht (voll, Teil-Bindung oder selbstgebaute Felder) und die erwarteten Antworten.
Die Frageliste steht als Positionen in der Sicht-Reihenfolge, die Relevanz als Abweichung zur
Relevanz des leeren Stores. Der Unsicherheits-Beitrag steht nicht im Fixture, die Formel steht in
Test (der Erzeuger ist geloescht).

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht.

## `konsistenz_orakel.json`

Snapshots (Feld -> `[wert, "b"|"v"]`, Scheibe, Vorjahr) fuer die Crate `konsistenz` samt den Antworten des
Python-Orakels (`produkt/konsistenz/*`, ueber `tools/parity/oracle_konsistenz.py`): `preflight(...)` und
`unvollstaendige_instanzen` je Snapshot, dazu `eur` und die fuenf Tabellen. Lange Texte (`grund`, `hinweis`)
stehen einmal in `texte` und im Szenario als `{"$t": index}`. Konsument:
`rust/konsistenz/tests/orakel_werte.rs` (hermetisch, ohne Python). Floats und Ganzzahlen ueber `i64` stehen nur
dort, wo Rust sie wie Python behandelt (`flag_check`, `check_pauschalen`, `partner_check`); die Betrags-Pruefungen
aus `preflight.py` zaehlen einen Float in Rust absichtlich nicht (`// PARITAET:` in der Crate-Doku).

## `intervall_orakel.json`

Szenarien fuer die Crate `intervall` samt den Antworten von `produkt/unsicherheit/intervall.py`: `sicht` (Pythons
Rohlesung aller echten Bindungen, Reihenfolge der Rust-Registry), `iv` (`intervall(...)` ueber synthetische Bindung,
Snapshot, Deckel) und `slots` (`bescheid_via_slots(...)`). Beide Seiten rechnen ueber dieselbe synthetische Engine
(`oracle_konsistenz._synth`: Summe ueber `gewicht(name) * zahl(wert)`). Konsument:
`rust/intervall/tests/orakel_werte.rs` (hermetisch, ohne Python).

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht. Die Erzeuger waren deterministisch (fester Seed,
sortierte Ausgabe).

## `wertwache_orakel.json`

Feste Grenzfaelle fuer `engine::zugriff` samt den Antworten des laufenden Python-Orakels (`runner.catala_*` ueber
`tools/parity/oracle.py`): `solz`, `fuenftel`, `p32b_1`, `p34c_1`, `kst_nenner_b`, `behinderten_pb`, `p33a_unterhalt`,
`renten_einkuenfte`, `p3_nr72_photovoltaik`, `p101_mobilitaetspraemie` und `_cent` (3683 Faelle, 689 029 Bytes). Je Fall: `fn`, `args` (der rohe
Sachverhalt-dict, wie ihn die Parity-Suiten schicken) und `py` (`{"ok": wert}` oder `{"err": klasse, "catala": bool}`).
Die Faelle liegen an den Schwellen und Saetzen der Zugriffsfunktionen (0/0, -1/-1, genau an der Grenze, +-1), weil der
Zufallsgenerator von `zugriff_teil{1,2}_paritaet` sie nur selten und nie gemeinsam zieht. Die Gitter der Koerperschaftsteuer
(Zinsschranke, Verlustabzug, Spendenabzug), der Rente (Basisrente aa) und des Unterhalts-Bodens stammen aus einem
Operator-Sweep ueber die Rumpfe der elf Funktionen: je Mutant, der ohne `PARITY=1` gruen blieb, kam der kleinste Fall dazu,
der ihn faengt. Konsument:
`rust/engine/tests/wertwache_orakel_werte.rs` (hermetisch, ohne `PARITY=1`, ohne Python; benutzt die Adapter der
Parity-Suiten).

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht. Der Erzeuger brauchte die gebaute Catala-Engine
(`make build-python`) und war deterministisch (feste Gitter, sortierte Ausgabe).

## `eingang_orakel.json`

Eingaben fuer die Crate `eingang` samt den Antworten des Python-Orakels (`produkt/eingang/*` ueber
`tools/parity/schritt8_oracle.py`). Konsumenten: `rust/eingang/tests/orakel_werte.rs` (Werte: CSV-
Zeilenmaschine, Betragsparser, Schluesselwoerter, Rundung, Vorjahr, eDaten, Beleg) und
`rust/eingang/tests/ocr_pfade.rs` (Abschnitt `ocr`: je Fall drei Shell-Skripte als `pdftotext`/`pdftoppm`/
`tesseract`, dieselben unter Python und Rust). Beide hermetisch, ohne Python zur Laufzeit. Der Erzeuger
legte `OMP_THREAD_LIMIT=7` in die Umgebung, damit sichtbar wird, dass nur `tesseract` ihn auf 1 setzt.

Eingefroren: Der Rust-Test zeigt, wo Rust vom Orakel abweicht. Das Fixture war deterministisch (Seed im Erzeuger).

## `zeichensatz_orakel.json`

Das Orakel `produkt/store/zeichensatz.py` fuer `domain::zeichensatz`, je Codepunkt (ohne Surrogate, bis U+10FFFF): `erlaubt`
(zusammenhaengende Bereiche, die `erstes_unerlaubtes_zeichen` durchlaesst) und `laeufe` (`[von, bis, name, rat]`: wie die
Meldung das Zeichen nennt, `null` = das Zeichen selbst, und der Rat), dazu volle Meldungen aus `feld_meldung` und
`element_meldung`. Konsument: `rust/domain/tests/zeichensatz_hermetisch.rs` (hermetisch, ohne `PARITY=1`, ohne Python).
Der Paritaetstest `rust/parity/tests/store_zeichensatz_paritaet.rs` prueft dasselbe live gegen Python.

Eingefroren; der Erzeuger war deterministisch.

## `api_stand_fragen_orakel.json`

Zwanzig Faelle (Scheiben `gesamt`, `an_gesamt`, `rentner_gesamt`, `ep`, `n_vor_gwg`; darunter zwei Vermietungsobjekte,
eine zweite Rente, ein vorlaeufiges Einzelfeld und ein Rentenbeginn im Folgejahr ohne Freibetrag, je bestaetigt und
vorlaeufig, damit die Faelle den Unterschied zwischen Ring mit und ohne Store, zwischen "nur bestaetigt" und
"auch vorlaeufig" und den Fehler `RentenfreibetragFixierungOffen` sehen; dazu zwei Faelle mit einer offenen Achse, damit die
Gewichte der Fragen-Reihenfolge sichtbar werden: Teil-Ring `ep_werbungskosten` und Gesamt-Ring ohne Gewichte wegen des Fehlers) samt den Antworten des Python-Servers
(`api.stand`, `api.fragen`, `api.frage_einzeln` aus `produkt/haut/api.py`, im selben Prozess mit denselben Ereignissen):
`events` (die Rumpfe von `POST /fall/<id>/event`, Reihenfolge ist Semantik), `stand` (die ganze Antwort), `fragen` (die
ganze Antwort, nur bei den grossen Faellen; sonst `fragen_ids` und der Sperrgrund), `kopf` (was der Mitschnitt fuer
`fragen` schreibt) und `einzeln` (Antwort je Probe-Feld). `event_id` jedes Felds in `stand` steht als `<event_id>` da:
der Server haengt die Uhrzeit an das Ereignis. Konsument: `rust/api/tests/stand_fragen_orakel_hermetisch.rs`
(hermetisch, ohne `PARITY=1`, ohne Python). Die Ereignislisten der Basisfaelle las der Erzeuger aus
`rust/api/tests/kette_endstand_hermetisch.rs`. Eingefroren; Stufe 2, Phase B macht daraus einen Rust-eigenen
Golden-Master.
