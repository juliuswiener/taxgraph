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

## `interview_orakel.json`

Szenarien fuer die Crate `interview` samt den Antworten des Python-Orakels
(`produkt/traverser/traverser.py`, `produkt/haut/bindung_rollen.py`). Konsument:
`rust/interview/tests/orakel_werte.rs` (hermetisch, ohne Python). Je Szenario: Events (Reihenfolge ist
Semantik), eine Sicht (voll, Teil-Bindung oder selbstgebaute Felder) und die erwarteten Antworten.
Die Frageliste steht als Positionen in der Sicht-Reihenfolge, die Relevanz als Abweichung zur
Relevanz des leeren Stores. Der Unsicherheits-Beitrag steht nicht im Fixture, die Formel steht in
Generator und Test.

```
python3 tools/parity/extract_interview_orakel.py
```

Aendert sich `traverser.py` oder die Bindung, ist das Fixture neu zu erzeugen; der Rust-Test zeigt
dann, wo Rust vom Orakel abweicht. `git diff` auf der Datei zeigt, was sich am Orakel geaendert hat.

## `konsistenz_orakel.json`

Snapshots (Feld -> `[wert, "b"|"v"]`, Scheibe, Vorjahr) fuer die Crate `konsistenz` samt den Antworten des
Python-Orakels (`produkt/konsistenz/*`, ueber `tools/parity/oracle_konsistenz.py`): `preflight(...)` und
`unvollstaendige_instanzen` je Snapshot, dazu `eur` und die fuenf Tabellen. Lange Texte (`grund`, `hinweis`)
stehen einmal in `texte` und im Szenario als `{"$t": index}`. Konsument:
`rust/konsistenz/tests/orakel_werte.rs` (hermetisch, ohne Python). Floats und Ganzzahlen ueber `i64` stehen nur
dort, wo Rust sie wie Python behandelt (`flag_check`, `check_pauschalen`, `partner_check`); die Betrags-Pruefungen
aus `preflight.py` zaehlen einen Float in Rust absichtlich nicht (`// PARITAET:` in der Crate-Doku).

```
python3 tools/parity/extract_konsistenz_orakel.py
```

## `intervall_orakel.json`

Szenarien fuer die Crate `intervall` samt den Antworten von `produkt/unsicherheit/intervall.py`: `sicht` (Pythons
Rohlesung aller echten Bindungen, Reihenfolge der Rust-Registry), `iv` (`intervall(...)` ueber synthetische Bindung,
Snapshot, Deckel) und `slots` (`bescheid_via_slots(...)`). Beide Seiten rechnen ueber dieselbe synthetische Engine
(`oracle_konsistenz._synth`: Summe ueber `gewicht(name) * zahl(wert)`). Konsument:
`rust/intervall/tests/orakel_werte.rs` (hermetisch, ohne Python).

```
python3 tools/parity/extract_intervall_orakel.py
```

Aendert sich ein Orakel (`produkt/konsistenz/*`, `intervall.py`) oder die Bindung, sind die Fixtures neu zu
erzeugen; der Rust-Test zeigt dann, wo Rust vom Orakel abweicht. Beide Generatoren sind deterministisch
(fester Seed, sortierte Ausgabe): zweimal erzeugt ergibt dieselben Bytes, unabhaengig vom Hash-Seed.

## `wertwache_orakel.json`

Feste Grenzfaelle fuer `engine::zugriff` samt den Antworten des laufenden Python-Orakels (`runner.catala_*` ueber
`tools/parity/oracle.py`): `solz`, `fuenftel`, `p32b_1`, `p34c_1`, `kst_nenner_b`, `behinderten_pb`, `p33a_unterhalt`,
`renten_einkuenfte`, `p3_nr72_photovoltaik`, `p101_mobilitaetspraemie` und `_cent`. Je Fall: `fn`, `args` (der rohe
Sachverhalt-dict, wie ihn die Parity-Suiten schicken) und `py` (`{"ok": wert}` oder `{"err": klasse, "catala": bool}`).
Die Faelle liegen an den Schwellen und Saetzen der Zugriffsfunktionen (0/0, -1/-1, genau an der Grenze, +-1), weil der
Zufallsgenerator von `zugriff_teil{1,2}_paritaet` sie nur selten und nie gemeinsam zieht. Konsument:
`rust/engine/tests/wertwache_orakel_werte.rs` (hermetisch, ohne `PARITY=1`, ohne Python; benutzt die Adapter der
Parity-Suiten).

```
python3 tools/parity/extract_wertwache_orakel.py
```

Der Generator braucht die gebaute Catala-Engine (`make build-python`) und ist deterministisch (feste Gitter, sortierte
Ausgabe): zweimal erzeugt ergibt dieselben Bytes, unabhaengig vom Hash-Seed (geprueft bei `PYTHONHASHSEED` 1 und 777).
Aendert sich ein Orakel (`runner.py`, Catala-Regeln), ist das Fixture neu zu erzeugen; der Rust-Test zeigt dann, wo
Rust vom Orakel abweicht.
