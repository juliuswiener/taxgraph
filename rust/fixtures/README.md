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
Abweichungsliste steht im Abschnitt „Abweichungsliste“ weiter unten.

Ausnahmen: `golden_cases.json` (Extrakt aus `golden/cases/*.yaml`, `tools/parity/extract_golden.py` bleibt) und
`begleitfelder_formen.json` (von Hand gepflegt). `api_stand_fragen_orakel.json` bleibt vorerst eingefroren; der
Umbau zu einem Rust-eigenen Golden-Master (S2.2) wartet auf die Waechter der Stufe 3.

## Abweichungsliste

Jede Stelle, an der Rust absichtlich von Python abweicht, mit Grund und dem Test, der sie haelt. Stand: Stufe 2,
Python-Stand `2dd056a6`.

Regeln:

1. Eine gewollte Abweichung hat einen Eintrag in dieser Liste UND einen Test ohne `PARITY=1`, der das heutige
   Rust-Verhalten festhaelt. Gleicht jemand Rust an Python an, wird der Test rot. Wer den Test aendert, aendert
   oder streicht den Eintrag im selben Commit.
2. Die eingefrorenen Fixtures (siehe oben) werden dafuer nie angefasst. Eine Abweichung steht in der Liste eines
   Tests oder in einem eigenen Test, nie als Aenderung in der Datei.
3. Spalte „Pruefung“: **M** = Mutant gemessen. Die Abweichung im Quelltext zurueckgebaut oder verschoben, der
   genannte Test wurde rot, die Basis war gruen. **T** = Test vorhanden, nicht mutiert (die Zusage steht im
   Kommentar des Tests).

| Nr | Ort | Python | Rust | Grund | Test (ohne `PARITY=1`) | Pruefung |
|----|-----|--------|------|-------|------------------------|----------|
| 1 | `eingang`, VaSt-Betrag, vier Eingaben `1__2`, `1_`, `_1`, `1e1_0` | `Decimal` streicht Unterstriche | `NichtLesbar` | VaSt-XML traegt keine Unterstriche | `rust/eingang/tests/orakel_werte.rs::vast_betraege_wie_orakel` (`UNTERSTRICH_ABWEICHUNG`, genau vier) | T |
| 2 | `domain`, Sperrgruende `KindFreibetragVerteilungOffen`, `KindZeitraumUnlesbar` | kennt sie nicht | Klartext und Kennung laufen rund | Entscheidung Julius 2026-10-04 | `rust/domain/src/sperrgrund.rs::tests::rust_eigene_gruende_haben_klartext_und_laufen_rund` | T |
| 3 | `api`, `elster`: Ganzzahl ausserhalb `i64` | rechnet mit beliebig grossen `int` weiter | HTTP 422, nie 500, nie Umbruch | Korrektheit vor Paritaet (`REWRITE_PLAN.md` §4) | `rust/api/tests/ueberlauf_klassen_hermetisch.rs` (elf Eingaben); `rust/elster/src/deklaration.rs::tests::p23_gewinn_ausserhalb_i64_ist_ein_fehler_statt_umbruch` | T |
| 4 | `domain`, Feld-Kennung `"x\n"` | `$` passt auch vor `\n`, das Schema nimmt es an | abgelehnt | fail-closed | `rust/domain/src/feld_id.rs::tests::basis_id_nimmt_genau_die_schema_regel_an` | M |
| 5 | `llm`, HTTP-Client, vier Eingaben (`Content-Length: abc`, URL ohne Schema, fremdes Schema, IPv6 ohne Port) | liest die Antwort bis zum Verbindungsende bzw. scheitert voruebergehend (drei Versuche) | endgueltiger Fehler nach einem Versuch | Befund der Messung, keine Absicht | `rust/llm/tests/client_netz.rs::weicht_von_python_ab_vier_eingaben_enden_in_rust_anders` | T |
| 6 | `llm`, `content` der Antwort ist Liste, Zahl oder Objekt | `.strip()` wirft `AttributeError` ungefangen (Dienst: 500) | endgueltiger Fehler `AttributeError` | der Aufruf bricht nicht ab | `rust/llm/tests/client_netz.rs::weicht_von_python_ab_inhalt_ohne_zeichenkette_ist_endgueltig` | M |
| 7 | `llm`, Fehlerkoerper mit Schluessel ueber der 300-Zeichen-Kante | kuerzt, dann maskiert; der Anfang des Schluessels bleibt stehen | maskiert, dann kuerzt | Sicherheit | `rust/llm/tests/client_netz.rs::weicht_von_python_ab_schluessel_an_der_schnittkante_wird_maskiert` | M |
| 8 | `llm`, Aussage-Nummer ausserhalb `i64` (`1e30`, `2^64-1`) | `int(...)` gibt die grosse Zahl; `int(inf)` wirft ungefangen | `None` | `inf` kommt ueber `serde_json` nicht an; nur die Zahl ausserhalb `i64` ist erreichbar | `rust/llm/tests/parse_entscheidungen.rs::weicht_von_python_ab_index_ausserhalb_i64_ist_none` | M |
| 9 | `llm`, `kategorie` ist Liste oder Objekt | `TypeError` (unhashable) ungefangen | `None`, Buchung bleibt unklassifiziert | der Aufruf bricht nicht ab | `rust/llm/tests/kontoauszug_entscheidungen.rs::weicht_von_python_ab_kategorie_liste_oder_objekt_ist_none` | M |
| 10 | `konsistenz`, Betragspruefungen (`bestaetigter_betrag`, `vorlaeufige_ring_betraege`): ein Float | zaehlt als Betrag (`isinstance(w, (int, float))`) | zaehlt nicht | Wertebereich: der Store laesst auf `cent`-Feldern nur Ganzzahlen zu (Auflage T); 0 Floats in 192 echten Faellen | `rust/konsistenz/src/preflight.rs::tests::float_ist_kein_betrag_abweichung_von_python` | M |
| 11 | `auth`, Audit scheitert | kein `try`, bricht ab | Anmelden, Abweisen und Abmelden laufen durch | das Audit ist ein Nebenkanal | `rust/auth/tests/entscheidungen.rs::ein_audit_fehler_kippt_keine_anmeldung` | T |

### Offen: Rust-Seite nicht hermetisch geprueft

Diese Abweichungen stehen heute in einer `PARITY=1`-Suite, im Kommentar des Quelltexts oder in beidem. Ob ein Test
ohne `PARITY=1` die Rust-Seite haelt, ist nicht gemessen. Faellt Python weg, fallen die Parity-Suiten mit, und
diese Zusagen haetten keinen Halter mehr. Vor der Loeschung von Python je Zeile pruefen und mit einem Mutanten
belegen.

- `rust/parity/tests/extern_stub/fremd_abweichungen.rs`: A1 (`NaN`, `Infinity`, `1e400` in `rechenweg` und
  `vorschlag_wert`: Rust gueltiges JSON), A3 (ORS-Entfernung `NaN`: Rust 503), C (einzelnes Surrogat im Text: Rust
  verwirft die ganze Antwort). B und A2 stehen oben als Nr. 6, 8 und 9.
- `rust/parity/tests/wert_paritaet.rs`: `D1_ABWEICHUNGEN` (vier Eintraege: Ganzzahl ausserhalb
  `i64::MIN..=u64::MAX` wird `Gleit`), die JSON-Lader (`NaN`/`Infinity`, `-0`, Surrogat; 28 Abweichungen), die
  YAML-Lader (YAML 1.2 gegen 1.1; 227 Abweichungen), `repr_str` escapet Cf, Co und Cn nicht (`ponytail`).
- `rust/parity/tests/store_append_paritaet.rs::d20_reihenfolge_typ_vor_signal`: Rust meldet `ZweiSignalFehlend`
  vor `TypInkonform`.
- `rust/parity/tests/elster_paritaet.rs` (Luecke bei Instanzen): dicht gezaehlt, kein leeres `<Einz>`.
- `rust/store/src/fehler_log.rs`: drei Abweichungen der Bauart (Aufrufstelle statt Traceback, `Meta` als Struct,
  Fall-Kennung mit Muster als Parameter).
- `rust/elster/tests/offene_defekte.rs` und `rust/bescheid/tests/offene_defekte.rs`: je eine „gewollte Abweichung“
  im Kommentar eines Tests.

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
`rust/api/tests/kette_endstand_hermetisch.rs`. Eingefroren; der Umbau zu einem Rust-eigenen
Golden-Master (S2.2) wartet auf die Waechter der Stufe 3.
