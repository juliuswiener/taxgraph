# HANDOVER: TaxGraph v3 – Phase 0 (Spike)

Kontext für Claude Code. Lies dieses Dokument vollständig, bevor du etwas ausführst.

## Projektkontext

TaxGraph ist ein langlaufendes Projekt von Julius zur regelbasierten Erstellung deutscher Einkommensteuererklärungen. Historie: v1 (2025) Knowledge Graph in Neo4j, verworfen. v2 (Dez 2025) Decision Tree + Rule Engine, konzeptionell bestätigt, aber nie gebaut. v3 (Juli 2026) ersetzt die Eigenbau-Engine durch Catala und fügt zwei externe Verifikations-Oracles hinzu. Vollständige Roadmap liegt in `docs/taxgraph-v3-roadmap.md` (falls vorhanden, sonst bei Julius anfragen).

Leitprinzip (AINA): **LLM schlägt vor, deterministische Tools verifizieren, Mensch entscheidet.** Das gilt auch für dich: Du erzeugst Code und Formalisierungen, aber jede fachliche Aussage über Steuerrecht muss durch Tests, Quellenangabe oder Oracle-Abgleich gedeckt sein. Erfinde niemals Steuerwerte oder Paragrapheninhalte aus dem Gedächtnis.

## Feststehende Stack-Entscheidungen (nicht neu diskutieren)

- **Catala** (Apache 2, Inria): Regelsprache, literate programming, Default Logic. Build/Test über **Clerk**.
- **GETTSIM** (PyPi/conda): deutsches Steuer-Transfer-Simulationsmodell, dient als differentielles Rechen-Oracle und Parameterquelle. Nicht als Basis, nur als Prüfinstanz.
- **ERiC/ELSTER**: kommt in Phase 4, für Phase 0 irrelevant (Registrierung läuft separat).
- Python als Compile-Target von Catala für die Oracle-Vergleiche.

## Ziel von Phase 0

Die komplette Kette einmal durchstechen und Gate G0 (go/no-go für Catala) mit Daten beantworten. Drei Arbeitspakete:

### S0.1: Setup + §32a-Formalisierung
1. Catala-Compiler + Clerk installieren (Arch Linux, opam-basiert; dokumentiere die Installationsschritte in `docs/setup.md`, inkl. Versionen).
2. Catala-Tutorial-Beispiele bauen, um die Toolchain zu verifizieren.
3. §32a EStG (Einkommensteuertarif) als literate Catala formalisieren:
   - Veranlagungszeiträume 2024, 2025, 2026 (drei Parametersätze, eine Formelstruktur)
   - Grundtarif (Abs. 1) und Splitting-Verfahren (Abs. 5)
   - Rundungsregeln exakt nach Gesetz (Abrundung des zvE, Rundung des Steuerbetrags)
   - Der Gesetzestext (aktuelle Fassung je VZ, Quelle: gesetze-im-internet.de, per Fetch holen, NICHT aus Gedächtnis) steht als Prosa im literate File, direkt gefolgt von der Formalisierung.
4. Die Tarifkonstanten (Grundfreibetrag, Zonengrenzen, Formelkoeffizienten) liegen NICHT hartcodiert im Code, sondern in einer separaten Parameterdatei pro VZ mit Feldern: wert, veranlagungszeitraum, rechtsquelle (gesetz/paragraph/absatz/satz), datenquelle (URL oder GETTSIM-Version).

### S0.2: Differentialtest gegen GETTSIM
1. GETTSIM installieren (venv oder pixi, dokumentieren).
2. Catala-Programm nach Python kompilieren, aufrufbares Interface bauen.
3. Testharness: pro VZ 1.000 deterministisch geseedete zvE-Werte (Bereich 0 bis 500.000 €, inkl. Randwerte: 0, Grundfreibetrag ±1, Zonengrenzen ±1), Grund- und Splittingtarif.
4. Vergleich Catala-Output vs. GETTSIM-Output auf Cent-Ebene.
5. Jede Divergenz einzeln analysieren und in `reports/s02-divergenzen.md` dokumentieren: Wert, beide Outputs, vermutete Ursache (eigener Bug / GETTSIM-Abweichung / Rundungsinterpretation), Status. Divergenzen werden NIE stillschweigend wegtoleriert oder der Vergleich aufgeweicht, um grün zu werden.

### S0.3: Default-Logic-Ergonomietest
1. Formalisiere die Homeoffice-Tagespauschale (§4 Abs. 5 Satz 1 Nr. 6c EStG i.V.m. §9 Abs. 5) und das häusliche Arbeitszimmer (§4 Abs. 5 Satz 1 Nr. 6b) inkl. des gegenseitigen Ausschlusses.
2. Ziel ist nicht Vollständigkeit, sondern die Frage: Bildet Catalas Grundregel/Ausnahme-Mechanik (default/exception) diese Struktur natürlich ab, oder braucht es Workarounds?
3. Schreibe 5–10 Clerk-Testfälle (z.B. nur Homeoffice, nur Arbeitszimmer, beide beantragt, Tage über Höchstgrenze).
4. Kurzes Fazit in `reports/s03-ergonomie.md`: was ging gut, wo hakte es, Codebeispiele.

## Gate-G0-Report (Abschluss-Deliverable)

`reports/gate-g0.md` mit expliziter Bewertung der vier Kriterien:
1. Compiler stabil genug (keine Blocker-Bugs im Spike-Umfang)? Belege.
2. Python-Backend produziert korrekte, aufrufbare Artefakte? Belege.
3. Default Logic bildet S0.3 ohne Verrenkungen ab? Verweis auf Ergonomie-Report.
4. Differentialtest grün oder alle Divergenzen erklärt? Verweis auf Divergenz-Report.

Empfehlung go/no-go mit Begründung. Die Entscheidung trifft Julius, nicht du.

## Repo-Layout (anlegen)

```
taxgraph/
  rules/estg/p32a/          # literate Catala + Tests
  rules/estg/p04_arbeitszimmer_homeoffice/
  params/2024/ 2025/ 2026/  # Parameterdateien mit Quellen
  oracle/gettsim/           # Differentialtest-Harness
  reports/                  # s02-divergenzen, s03-ergonomie, gate-g0
  docs/                     # setup.md, roadmap
```

## Konventionen

- Sprache im Code/Kommentaren: Englisch; Gesetzestexte und Reports: Deutsch.
- Keine em-dashes (—) in irgendwelchen Texten oder Dokumenten. Sätze umstrukturieren statt Gedankenstriche.
- Commits klein und thematisch, Conventional-Commits-Stil.
- Jede fachliche Zahl (Freibetrag, Grenze, Prozentsatz) trägt eine Quellenangabe. Gesetzestexte per Web-Fetch von gesetze-im-internet.de holen und die abgerufene Fassung samt Abrufdatum vermerken.
- Bei Unsicherheit über steuerfachliche Interpretation: als offene Frage in den Report, nicht raten.
- Umgebung: Arch Linux (arch-desk), Fish shell. System-Python nicht anfassen, venv/pixi nutzen. opam-Setup dokumentieren.

## Eskalation an Julius (Arbeit unterbrechen und fragen)

- Catala-Installation scheitert hart (Blocker) → melden mit Fehlerbild, nicht stundenlang workarounden.
- GETTSIM und Gesetzestext widersprechen sich → dokumentieren und fragen.
- Eine Catala-Sprachbeschränkung erzwingt eine semantische Abweichung vom Gesetzestext → niemals stillschweigend approximieren.

## Definition of Done Phase 0

- Alle drei Reports + Gate-G0-Report liegen vor.
- `clerk test` läuft grün auf allen geschriebenen Tests.
- Differentialtest reproduzierbar per einzelnem Kommando (`make s02` o.ä.).
- Setup von Null nachvollziehbar aus docs/setup.md.
