# Externe Befunde zu TaxGraph — von der legalHelper-Session, 2026-07-26

Kontext: Der Nutzer liess legalHelper (`~/00_projects/legalHelper`) mit TaxGraph
vergleichen. Dabei fiel Substanz an, die fuer dein Repo-Review relevant ist.
Alles unten ist **selbst am Code/an Laeufen verifiziert**, nicht aus eurer Doku
uebernommen. Wo ich eurer Autopsy widerspreche, steht es dabei.

Ich habe **nichts** in eurem Repo geaendert. Nur diese Datei geschrieben.

---

## 1. Testsuite: 124 rot, nicht 105 — die Zahl waechst

Selbst gefahren, `python -m pytest tests/ -q`, 2026-07-26:

```
124 failed, 988 passed, 4 skipped, 1 warning in 196.86s
```

`docs/autopsy-20260724/08_honest_conclusions.md` nennt durchgaengig **105**
pre-existing failures. Zwei Tage spaeter sind es 124. Die Autopsy stuft sie als
"annoying but not dangerous" ein — die Zahl bewegt sich aber nach oben, und
solange sie das tut, ist "pre-existing" keine tragfaehige Einordnung mehr.

**Bitte im Review nachziehen:** ist der Delta von 19 wirklich dieselbe Klasse
(Schema/Enum), oder sind neue Ursachen dazugekommen? Das laesst sich nur
beantworten, wenn die 124 nach Fehlerursache gruppiert werden, nicht nach
Testdatei.

## 2. Wahrscheinliche Wurzel: stale `_build/`

Stichprobe `tests/test_solz_ring.py::test_solz_ring_gesamt_hochverdiener`:

```
jsonschema ValidationError
On instance:
    {'fehler': "KeyError: 'steuerermaessigungen'"}
```

Zeitstempel:
- `_build/` → **24. Jul 04:07**
- `produkt/haut/api.py` → **24. Jul 12:02**, juengste Commits **25. Jul**

Der Ring faellt also gegen kompilierte Artefakte, die aelter sind als der Code,
der sie benutzt. `steuerermaessigungen` wird in `api.py` an 9 Stellen gesetzt
(u.a. 1004, 1018, 1262, 1532, 1543) — der `KeyError` kommt sehr wahrscheinlich
aus einem Catala-Modul, das den Slot noch nicht kennt.

Das deckt sich mit eurem eigenen Must-fix Nr. 3 ("Automate `make build-python`
as CI pre-step"). Aus der Aussenperspektive ist das **nicht** Prioritaet 3
sondern 1: solange der Build stale sein kann, ist jeder rote Test doppeldeutig
(echter Bug vs. Artefakt-Drift), und genau diese Doppeldeutigkeit ist der Grund,
warum 124 rote Tests toleriert werden koennen.

**Konkrete Bitte ans Review:** einmal `make build-python` frisch, dann die Suite.
Wenn die Zahl deutlich faellt, ist die Priorisierung in der Autopsy falsch
gewichtet und gehoert korrigiert.

## 3. Blindes Regressionsnetz — genau dort, wo ihr es braucht

Die roten Tests konzentrieren sich auf `test_ring_regression_kampagne.py` und
`test_solz_ring.py`, also die **Ring-Regression** und den `gesamt`-Scope. Eure
Autopsy sagt es selbst ("a bug in DBA, SolZ, or the gesamt Ring would go
undetected"), aber ordnet es unter UNCERTAIN ein. Es ist nicht uncertain: die
Tests laufen nicht, also decken sie nichts ab. Das ist der Punkt, an dem die
Autopsy zu freundlich mit sich selbst ist.

## 4. Was an TaxGraph aus der Aussensicht stark ist

Damit das Review nicht nur Mangelliste wird — das hier ist substanziell besser
als in legalHelper und sollte im Review als tragende Saeule behandelt (und
geschuetzt) werden:

- **Oracle-Differentialtest (GETTSIM).** Eine unabhaengig gebaute
  Zweitimplementierung, ueber tausende Seeds gegengerechnet. legalHelper hat
  *keine einzige* Pruefachse gegen eine unabhaengige Quelle — nur
  Selbstkonsistenz. Das ist euer groesster struktureller Vorsprung.
- **Modell-Dekorrelation** (`pipeline/models.yaml`). A/B/Judge aus drei
  Modellfamilien, Provider gepinnt, `allow_fallbacks: false`, mit dokumentierter
  Begruendung und Re-Evaluations-Triggern. Ich empfehle legalHelper gerade,
  genau das zu uebernehmen.
- **SKIP != PASS.** Jedes Gate in `pipeline/gates.py` meldet SKIP, wenn sein
  Werkzeug fehlt (`clerk missing; textual-proxy ...`), nie stillen Durchlass.
  Dazu `hat_split_auf_blockierendem_gate()`. Das ist die richtige Haltung und
  seltener als es klingt.
- **Snapshot-Ratsche.** `sha256(catala_a)`, `make snapshot-verify`,
  manipulierter Snapshot failt hart. Reproduzierbarkeit ohne Modellkosten.
- **Ehrliche Selbstauskunft.** 11 Stufe-2-Marker, "lücken"-Sektionen, eine
  Autopsy mit TRUE/UNCERTAIN/FALSE. Das ist wirklich selten.

## 5. Gegenrichtung: was TaxGraph von legalHelper uebernehmen sollte

legalHelper ist viel kleiner (3.827 LOC prod) und in einem Punkt disziplinierter:

- **Gruene Suite als Abnahmegate.** Dort gilt: kein Meilenstein ist fertig ohne
  gruene Tests. Ergebnis: 270/270 gruen. Das ist der einzige Grund, warum es dort
  keine 124-Fehler-Situation geben *kann*.
- **`mypy --strict` im Makefile.** Bei euch sehe ich kein Typcheck-Target in den
  Make-Zielen. Bei 48k LOC Python waere das ein billiger Netzgewinn.
- **Strukturelle Bright Lines mit Guard-Tests.** In legalHelper importiert genau
  *eine* Prod-Datei das LLM-SDK; Gates/Store/Sources sind LLM-frei, und ein Test
  grept den Quelltext darauf. Bei euch importiert `produkt/haut/api.py` — die
  2.520-LOC-Produktions-API — einen LLM-Client. Das ist nicht per se falsch
  (Chat-Feature), aber es gibt keinen Test, der die Grenze festhaelt. Ein
  Guard-Test "Berechnungspfad importiert keinen LLM-Client" waere billig.

## 6. Eine Einordnung, die im Review helfen koennte

Die beiden Repos haben spiegelbildliche Schwaechen:

- **legalHelper:** strukturell sauber, aber **unbewiesen**. Perfekte Gates, die
  nie gegen ein echtes Modell und einen echten Mirror liefen. Kein git.
- **TaxGraph:** empirisch belastbar, aber **strukturell verwahrlost**. Echte
  Oracles, echte Divergenzberichte, und eine Suite, die zu einem Viertel rot ist.

Fuer euer Review heisst das: die Frage ist nicht "was fehlt fachlich", sondern
"warum darf der Build stale sein und die Suite rot bleiben". Das ist eine
Prozessfrage, keine Steuerrechtsfrage.

---

Rueckfragen an die legalHelper-Session gehen ueber den Nutzer; der agentbus war
beim Schreiben nicht erreichbar ("Unable to connect").
