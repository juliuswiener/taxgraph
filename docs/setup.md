# Setup (Phase 0)

Nachvollziehbares Setup der Toolchain von Null. Getestete Umgebung:

| Baustein | Version |
|----------|---------|
| OS (Referenz-Setup) | Ubuntu 24.04.4 LTS |
| opam | 2.1.5 |
| OCaml | 4.14.2 (opam switch `taxgraph`) |
| Catala | 1.2.0 |
| Clerk | 1.2.0 |
| Python (Oracle + Catala-Python) | 3.12.3 |
| GETTSIM | 1.2.1 |
| NumPy | siehe `oracle/.venv312` |

Hinweis zur Umgebung: Der Handover nennt Arch Linux (arch-desk, Fish). Dieses
Referenz-Setup lief auf Ubuntu 24.04 (Bash). Der einzige Unterschied ist der
Paketmanager fuer opam (`apt` statt `pacman`); alles Weitere ist identisch, da
Catala und GETTSIM ueber opam bzw. pip/venv kommen. Auf Arch entsprechend
`pacman -S opam` statt `apt-get install opam`.

## 1. Catala und Clerk (opam)

```bash
# System-Abhaengigkeiten
sudo apt-get update
sudo apt-get install -y opam m4 build-essential pkg-config libgmp-dev
# (Arch: sudo pacman -S opam m4 base-devel pkgconf gmp)

# opam initialisieren und dedizierten Switch anlegen
opam init --bare -y
opam switch create taxgraph 4.14.2
eval $(opam env --switch=taxgraph --set-switch)

# Catala (bringt Clerk mit)
opam install -y catala

catala --version   # 1.2.0
clerk --version    # 1.2.0
```

Das Skript `scripts/install-catala.sh` fasst diese Schritte zusammen (auf Ubuntu
getestet). Bei jeder neuen Shell muss die opam-Umgebung geladen werden:
`eval $(opam env --switch=taxgraph --set-switch)`. Das Makefile erledigt das pro
Target selbst.

### Toolchain verifizieren

```bash
make tests    # baut Catala-Runtime + stdlib und laeuft alle Clerk-Tests (12 gruen)
```

## 2. GETTSIM und Catala-Python-Runtime (Python)

Es werden zwei virtuelle Umgebungen genutzt (System-Python bleibt unangetastet):

- `oracle/.venv` (Python 3.11): reines GETTSIM, optional.
- `oracle/.venv312` (Python 3.12): GETTSIM **und** die Catala-Python-Runtime.
  Die von Catala erzeugte Python-Runtime nutzt PEP-695-Syntax
  (`class Array[T: Value]`) und benoetigt daher Python >= 3.12. Der
  Differentialtest laeuft in dieser Umgebung.

```bash
# uv wird als Environment-Manager verwendet (schnell, reproduzierbar)
uv venv oracle/.venv312 --python 3.12
uv pip install --python oracle/.venv312/bin/python gettsim   # zieht 1.2.1
```

`scripts/install-gettsim.sh` legt die 3.11-Umgebung an; fuer den Differentialtest
ist die 3.12-Umgebung noetig (Kommandos oben).

## 3. Differentialtest ausfuehren

```bash
make s02
```

Das Kommando kompiliert das Catala-Modul `Einkommensteuertarif` nach Python
(`clerk build p32a-python`), fasst das Python-Paket zusammen
(`oracle/gettsim/assemble_catala.sh`) und startet den Harness
(`oracle/gettsim/harness.py`), der pro Veranlagungszeitraum 1000 geseedete
zvE-Werte plus Randwerte durch Catala und GETTSIM schickt und
`reports/s02-divergenzen.md` neu erzeugt.

## Makefile-Ziele

| Ziel | Wirkung |
|------|---------|
| `make tests` | Alle Clerk-Scope-Tests (S0.1 Tarif + S0.3 Arbeitszimmer/Homeoffice). |
| `make s01` | Nur die § 32a-Tarif-Tests. |
| `make s03` | Nur die Arbeitszimmer/Homeoffice-Tests. |
| `make build-python` | Catala -> Python-Paket bauen und zusammenfuegen. |
| `make s02` | Differentialtest gegen GETTSIM, erzeugt den Divergenzreport. |
| `make params-check` | Ableitung/Validierung der Tarifkoeffizienten. |
| `make clean` | Build- und Zwischenartefakte entfernen. |

## Quellenbeschaffung: Fallback bei gesetze-im-internet-503

gesetze-im-internet.de liefert unter Last zeitweise HTTP 503 und hat nur die
jeweils geltende Fassung. Standard-Fallback fuer datierte/historische Fassungen:

- **Gesetzesfassungen (BGBl):** recht.bund.de, z. B.
  `https://www.recht.bund.de/bgbl/1/2024/386/regelungstext.pdf` fuer BGBl 2024
  I Nr. 386.
- **Konsolidierte Tarifwerte:** Amtliches Einkommensteuer-Handbuch (EStH/LStH),
  `esth.bundesfinanzministerium.de`, § 32a.

Abgerufene Texte werden nach dem Einfrier-Prinzip in `sources/` mit URL,
Abrufdatum und SHA256 abgelegt (`make sources-check`).

## Bekannte Reibungspunkte (dokumentiert, keine Blocker)

- **Cross-Modul-Scope-Aufrufe** erfordern `catala --whole-program` bzw.
  `clerk test -W`. Das Makefile nutzt durchgaengig `-W`.
- **Modulname == Dateiname.** Ein `> Module Foo` verlangt die Datei `Foo.catala_en`
  (Grossschreibung des Basenamens). Beispiel hier: Modul `Einkommensteuertarif`
  in `einkommensteuertarif.catala_en`.
- **`Decimal.floor` / `Decimal.ceiling`** der stdlib sind Cap-Funktionen
  (min/max), kein ganzzahliges Ab-/Aufrunden. Fuer die gesetzliche Abrundung auf
  volle Euro wird `Decimal.truncate` verwendet (bei nichtnegativen Werten gleich
  dem Abrunden).

## OpenRouter: `provider.only` ist keine Praeferenzliste

Gelernt im G2-Bake-off (2026-07-09), nachdem drei Laeufe daran gescheitert sind.

`provider: {only: [...], allow_fallbacks: false}` schraenkt die zulaessigen Hoster
ein, aber **OpenRouter waehlt frei aus der Liste**. Die Reihenfolge ist keine
Praeferenz. Im Lauf war der Worker auf `["morph", "fireworks", "venice"]` gesetzt;
geroutet wurde auf Venice, das mit `429 temporarily rate-limited upstream`
antwortete, und der Task starb - obwohl Morph verfuegbar war.

Konsequenz, verbindlich:

- **Pro Rolle exakt ein gepinnter Provider** in `pipeline/models.yaml`.
- **Failover ist kein Listeneintrag**, sondern eine eigene, bewusste
  Konfigurationsaenderung. Sie aendert den `models.yaml`-Hash und ist damit in
  jedem Provenienz-Stempel sichtbar. Ein Lauf bleibt nur gueltig, wenn alle
  Rollen gegen genau eine eingefrorene Konfiguration liefen.
- **DigitalOcean ist raus.** Es antwortet auf `deepseek/deepseek-v4-flash` mit
  `403` und unter Last auch auf `deepseek/deepseek-v4-pro` - obwohl ein einzelner
  Probe-Call durchging. Ein Einzel-Call beweist nichts ueber das Verhalten unter
  Last.

Weitere Fallstricke, jeweils an echten Fehlern gelernt:

- `provider.only` erwartet den **Provider-Tag** (`fireworks`, `fireworks/fast`,
  `together`, `anthropic`, `morph`), nicht den Anzeigenamen (`Fireworks`).
- Endpoints eines Modells wechseln. Vor dem Pinnen abfragen, nicht raten:
  `GET /api/v1/models/<slug>/endpoints`. Quantisierte Endpoints (`fp4`, `fp8`)
  bleiben ausgeschlossen - Quantisierung bricht den Determinismus bei
  Temperatur 0.
- Ein `403` bei `allow_fallbacks: false` ist eine providerseitige Ablehnung, die
  OpenRouter durchreicht; ein ungueltiger Key gibt `401`. Der Client wiederholt
  deshalb auch `403`.
- Ein Backoff von einer Sekunde hilft gegen ein Upstream-Rate-Limit nicht. Der
  Client wartet bei `403`/`429` 10s und 20s, bei `5xx` 5s und 10s - bei
  unveraenderten zwei Retries.

## OpenRouter: ein Fehler kommt auch mit HTTP 200

Gelernt in der ersten Produktionscharge (2026-07-09), die daran drei von zehn
Regeln verlor.

OpenRouter beantwortet Upstream-Probleme nicht immer mit einem Fehlerstatus. Es
kommt ein `HTTP 200` mit einem Body, der statt `choices` ein `error`-Objekt
traegt:

```json
{"error": {"code": 429, "message": "Provider returned error", ...}}
```

Ein Zugriff auf `data["choices"][0]` crasht dort mit `KeyError`. Der Statuscode
allein ist also kein hinreichender Erfolgsindikator - der Body muss geprueft
werden. Der Client behandelt ein `200` ohne `choices` als transienten Fehler
(zwei Retries, 5s/10s), meldet danach `role_timeout` bei `code: 429` und sonst
`role_error`. Ein Body, der ueberhaupt kein JSON ist (Gateway-HTML), faellt in
denselben Pfad statt in eine `ValueError`.

## Ein zu knappes `max_tokens` sieht aus wie ein Modellfehler

Ebenfalls aus der ersten Charge. Der Judge meldete "judge output not valid JSON".
Das Verdikt war aber nicht kaputt, sondern am Limit abgeschnitten:
`completion_tokens == max_tokens == 4096`, `finish_reason == "length"`.

DeepSeek V4 Pro schreibt vor der eigentlichen Antwort ausfuehrlich
Reasoning-Tokens; mit `roundtrip_diff@3` liegt der gemessene Bedarf bei rund
8.000 Tokens. Zwei Konsequenzen:

- `max_tokens` steht in `pipeline/models.yaml` je Rolle explizit und mit einer
  Notiz, wofuer das Budget gebraucht wird. Es ist kein Wert, den man beilaeufig
  von einer Rolle zur naechsten kopiert.
- `Completion` und `Provenance` fuehren ein `truncated`-Flag. Faellt es, nennen
  die Gates die Ursache ("judge answer truncated at max_tokens") statt eines
  Parse-Fehlers. Eine abgeschnittene Antwort ist kein Urteil und darf weder als
  "ungueltiges JSON" noch als "keine Abweichung" durchgehen.

Merksatz: bevor eine Modellantwort als inhaltlich falsch gilt, pruefe
`finish_reason`.

## Stille YAML-Semantik ist eine Gate-Umgehung

Gelernt beim Neuschnitt von § 35a (2026-07-09).

`yaml.safe_load` nimmt bei zwei gleichen Schluesseln im selben Mapping
kommentarlos den letzten:

```yaml
regel:
  test_seed:
  - {expected: 1410.00}   # neu, drei Kategorien
  test_seed:
  - {expected: 1200.00}   # alter Block, vergessen
```

Beim Neuschnitt blieb ein alter `test_seed:`-Block stehen. Geladen wurde nur der
alte. Die Regel haette mit einem **gruenen** Test-Gate dagestanden, das eine
voellig andere Signatur prueft - der Fehler waere nicht als Fehler erschienen,
sondern als Erfolg.

Ein Gate, das man durch einen vergessenen Block umgehen kann, ist kein Gate.
Deshalb laedt jedes Manifest dieses Projekts ueber `yamlstrict.load_yaml`; ein
doppelter Schluessel ist dort ein Fehler, kein Ueberschreiben. Umgestellt sind
`pipeline/` (models.yaml, rules.yaml, tasks.yaml, Leakage-Guard), `golden/`,
`params/`, `docstore/` und `elster/`.

Zwei Nebenbefunde, die die Regel bestaetigen:

- Ein `ast.parse`-Check haette den Umbau fuer gruen erklaert. `golden/runner.py`
  importierte `yamlstrict` vor dem `sys.path`-Setup und war kaputt. Umgestellte
  Leser gehoeren gegen echte Daten getestet, nicht gegen den Parser.
- Deterministische Pruefungen duerfen nicht an der Toolchain haengen. Das
  Clerk-Gate prueft Herkunft, Rechenweg und Zitatanker seiner Test-Seeds jetzt
  **bevor** es `clerk` auf dem PATH sucht. Vorher war seine Strenge davon
  abhaengig, ob die opam-Umgebung geladen war - dieselbe Regel konnte je nach
  Shell `FAIL` oder `SKIP` liefern.

## Ein Gate ohne frisches Verdikt hat KEINEN Zustand, niemals den alten

Dieselbe Fehlerklasse wie die stille YAML-Semantik: **falsches Grün**.

`--regate` rechnet die deterministischen Gates aus gespeicherten Quellen neu. Traf
es dabei ein Judge-Verdikt, das gar keines war — abgeschnitten am Token-Limit oder
kein gültiges JSON —, übersprang es die drei Judge-Gates und ließ deren **alte
`PASS`-Werte stehen**. § 9 Abs. 4a und § 35a standen so auf `verified_bedingt`,
während ihr Judge in Wahrheit nie geurteilt hatte.

Die Regel lautet deshalb: ein Gate, für das kein frisches, gültiges Verdikt
vorliegt, ist `FAIL`. Nicht „unverändert", nicht „unbekannt", nicht der alte Wert.
Die Entscheidung liegt in `judge_gates()` und ist in beide Richtungen getestet.

Damit die Herkunft eines Urteils prüfbar bleibt, trägt jeder Verdikt-Report seit
2026-07-10 die `lauf_id` und den `timestamp` des Judge-Calls, der ihn erzeugt hat.
Ein Report, der ein Urteil behauptet, muss sagen können, woher es stammt.

Verwandte Fallstricke derselben Klasse:

- Eine **leere Quelle** passt zu ihrem eigenen SHA256. `sources-check` war grün,
  während zwei Paragraphen leer eingefroren waren. Der Verifier misst jetzt den
  Wortlaut, nicht die Dateigröße, und `scripts/freeze_source.py` prüft vor dem
  Schreiben.
- Ein **doppelter YAML-Schlüssel** überschreibt still den vorherigen Wert. Ein
  vergessener `test_seed:`-Block hätte eine Regel mit einem grünen Test-Gate
  dastehen lassen, das eine andere Signatur prüft.

Gemeinsamer Nenner: Der Fehler zeigt sich nicht als Fehler, sondern als Erfolg.
Deshalb muss jeder dieser Pfade einen expliziten Negativtest haben.

## Das Round-Trip-Gate garantiert keine Vollständigkeit

Wörtlich ins Protokoll (Dekret 2026-07-10, Punkt 5), damit niemand dem Gate eine
Eigenschaft zuschreibt, die es nicht hat:

> Nach der Vervollständigung der Bedingungslisten ist die verbleibende Aufgabe des
> Round-Trip-Gates die Erkennung **undeklarierter** Annahmen, und dort bleibt
> Recall-Rauschen ein False-PASS-Risiko — eine Annahme, die alle Inventarläufe
> verpassen. Gegenlager: Union-until-Saturation, Golden-Tests, Human-Review.

Konkret: der Judge listet das Inventar der Prüf-Items als Freitext-Schritt, und
sein Recall streut (gemessen: in einer Kampagne waren von je ~11 Annahmen einer
Regel nur 1 in beiden Läufen). Ein `roundtrip=PASS` heißt daher „unter den
gefundenen Items war keine undeklarierte Annahme", nicht „es gibt keine". Die
Union-until-Saturation-Strategie mildert das (sie läuft, bis ein Inventarlauf nichts
Neues mehr bringt), beseitigt es aber nicht. Vollständigkeit kommt erst aus dem
Zusammenspiel mit dem Golden-Test-Gate und dem Human-Review.
