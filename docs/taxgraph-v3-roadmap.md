# TaxGraph v3 – Roadmap

Stand: 09.07.2026
Prinzip: LLM schlägt vor, deterministische Tools verifizieren, Mensch entscheidet (AINA).

## Zielbild

Eine deutsche EStG-Regelbibliothek in Catala (literate, Default Logic), verifiziert gegen zwei unabhängige Oracles (GETTSIM differentiell, ERiC amtlich), befüllt durch eine LLM-Formalisierungspipeline mit externem Verifikationskernel, nutzbar über einen Interview-Layer, der Fragen aus den Regelabhängigkeiten generiert und Ergebnisse über catala-explain begründet.

MVP-Scope: Arbeitnehmerfall. Mantelbogen, Anlage N, Anlage Vorsorgeaufwand, Anlage Kind. Einzel- und Zusammenveranlagung. Kein Gewerbe, keine V+V, keine KAP im MVP.

Nicht-Ziele (v3): Steuerberatung im Sinne individueller Gestaltungsempfehlungen, Mehrjahresvergleiche, andere Steuerarten als ESt.

## Stack (Entscheidung aus Recherche 07/2026)

| Baustein | Rolle | Lizenz |
|---|---|---|
| Catala + Clerk | Regelsprache, Build, Tests, PDF-Export | Apache 2 |
| tree-sitter-catala | Syntax-Gate für LLM-Output | Apache 2 |
| catala-explain | Erklärungsdokumente aus Execution-Trace | Apache 2 |
| GETTSIM | Differentielles Rechen-Oracle, Parameterquelle | Open Source (PyPi/conda) |
| ERiC (checkESt) | Amtliches Validierungs-Oracle, Versand | proprietär, registrierungspflichtig |
| Erica (DigitalService) | Referenzcode für ERiC-Anbindung (Pyeric) | offen, Wartungsmodus |
| ELSTER-Schema | Feldmodell Ground Truth | amtlich |

OpenFisca: keine Dependency, nur Patterns (Parameter als Daten, YAML-Testfälle, Legislation Explorer als UI-Idee).
Rulemapping/SPRIND: beobachten. Kein Fundament, aber Format-Anschlussfähigkeit prüfen, sobald deren Standard oder Open-Source-Teile erscheinen.

---

## Phase 0: Spike und Grundsatzentscheidung (1–2 Wochen)

Ziel: Die komplette Kette einmal durchstechen, bevor irgendetwas Größeres gebaut wird.

**S0.1 Catala-Setup und §32a.**
Compiler + Clerk + VSCode-LSP installieren (arch-desk). Tutorial durcharbeiten. §32a EStG (Tarif 2024, 2025, 2026) als literate Catala formalisieren, inkl. Splittingverfahren §32a Abs. 5.

**S0.2 Differentialtest gegen GETTSIM.**
Catala nach Python kompilieren. 1.000 zufällige zvE-Werte pro Jahr, Grund- und Splittingtarif, durch beide Implementierungen. Erwartung: exakte Übereinstimmung auf Cent-Ebene (Rundungsregeln §32a beachten). Jede Divergenz wird analysiert und dokumentiert.

**S0.3 Default-Logic-Ergonomietest.**
Eine Regel mit echter Ausnahmestruktur formalisieren: Homeoffice-Pauschale (§4 Abs. 5 Nr. 6c) mit gegenseitigem Ausschluss zum häuslichen Arbeitszimmer (§4 Abs. 5 Nr. 6b). Genau der Fall, der 2025 die Graph-Modellierung gesprengt hat. Frage: Bildet Catalas Grundregel/Ausnahme-Mechanik das natürlich ab?

**S0.4 ERiC-Zugang beantragen.**
Registrierung im ELSTER-Entwicklerportal sofort starten (Vorlaufzeit). Parallel Erica-Repo klonen und Pyeric-Struktur lesen.

**S0.5 Rulemap Builder antesten (0,5 Tage).**
Einen EStG-Paragraphen im kostenlosen Builder modellieren. Ziel: deren Tatbestandsmerkmal-Schichtung als Strukturvorbild für die eigene Regelextraktion verstehen. Keine Abhängigkeit aufbauen.

**Gate G0 (go/no-go Catala):**
- Compiler stabil genug für den Spike-Umfang (keine Blocker-Bugs)
- Python-Backend produziert korrekte, aufrufbare Artefakte
- Default Logic bildet S0.3 ohne Verrenkungen ab
- Differentialtest S0.2 grün oder Divergenzen erklärbar

Fallback bei No-Go: minimale eigene Regel-IR (JSON/YAML mit Default-Logic-Semantik) plus Python-Interpreter. Deutlich mehr Eigenaufwand, daher nur bei hartem Scheitern.

---

## Phase 1: Fundament (2–3 Wochen)

**M1.1 Repo und Build.**
Monorepo `taxgraph`: `rules/` (Catala, literate, ein Verzeichnis pro Anlage/Themenblock), `params/`, `oracle/`, `pipeline/`, `elster/`, `interview/`. Clerk als Testrunner, CI lokal (arch-desk) plus nordserver für Services.

**M1.2 Parameterschicht.**
Jahreswerte (Grundfreibetrag, Pauschbeträge, Höchstbeträge, Tarifformelkonstanten) als versionierte Daten, getrennt von Formeln, OpenFisca-Pattern. Import-Skript aus GETTSIM-Parameterdateien mit Herkunftsvermerk. Jeder Parameter trägt: Wert, Veranlagungszeitraum, Rechtsquelle (Gesetz, Paragraph, Absatz, Satz), Quelle des Datums (GETTSIM-Version / BGBl).

**M1.3 ELSTER-Feldmodell.**
ELSTER-Schema für ESt (MVP-Anlagen) parsen: Felder, Typen, Hierarchie, Pflichtstatus. Als deterministische Datenbasis, kein LLM. Mapping-Tabelle Regel-Output → Feld-ID als eigenes, reviewbares Artefakt.

**M1.4 Golden-Test-Korpus v1.**
Quellen: Rechenbeispiele aus BMF-Ausfüllanleitungen, publizierte BFH-Fälle mit Zahlen, synthetische Fälle. Format: YAML (Input-Sachverhalt, erwartete Feldwerte, erwartete festzusetzende ESt, Quelle). Ziel Phase 1: 30–50 Fälle. Der Korpus ist das zentrale Verifikationsasset und wächst über alle Phasen.

**Deliverable Phase 1:** §32a + Werbungskostenpauschbetrag + Sonderausgabenpauschbetrag end-to-end: Sachverhalt rein, festzusetzende ESt raus, differentiell grün gegen GETTSIM.

---

## Phase 2: LLM-Formalisierungspipeline (3–4 Wochen)

Architektur nach dem verifizierten Rezept (arXiv 2606.23913, 2606.16118) und deinen Benchmark-Ergebnissen (externe Verifier +12,8pt, Selbstverifikation versagt).

**Pipeline-Stufen pro Regelkandidat:**
1. **Extraktion:** LLM segmentiert Quelltext (EStG-Norm, BMF-Schreiben) in Regelkandidaten mit Pflicht-Metadaten (Gesetz, §, Abs., Satz, VZ-Gültigkeit). Worker-Tier tauglich.
2. **Doppelformalisierung:** zwei unabhängige Formalisierungen (verschiedene Modelle oder disjunkte Prompts) nach Catala.
3. **Syntax-Gate:** tree-sitter-Parse, dann Catala-Compiler-Typecheck. Billig, deterministisch, vor allem Teurem.
4. **Äquivalenzcheck:** beide Formalisierungen auf generierten Input-Rastern ausführen und Outputs vergleichen (extensionale Äquivalenz). Divergenz = Flag, niemals automatische Auflösung.
5. **Round-Trip:** Rückübersetzung der Formalisierung in natürliche Sprache, Diff gegen Originalnorm durch separates Modell, Abweichungen und stillschweigende Zusatzannahmen als explizite Liste.
6. **Testausführung:** Clerk-Tests aus dem Golden-Korpus plus regelspezifische Fälle.
7. **Human Approval:** Review-Queue mit Status extracted → formalized → verified → approved. Angezeigt werden: literate Diff (Gesetzestext neben Code), Äquivalenzreport, Round-Trip-Abweichungen, Testresultate.

**M2.1 Pipeline-Backend** als FastAPI-Service auf nordserver, resumable Prozesse mit Checkpoint pro Item (Dezember-Design übernehmen), Postgres für Zustände und Claims-Metadaten.
**M2.2 Review-UI** minimal (kann Artefakt/SvelteKit-Prototyp sein), eine Queue, Approve/Reject/Kommentar.
**M2.3 Metriken** von Tag 1: Syntaxvaliditätsrate, Äquivalenz-Divergenzrate, Round-Trip-Abweichungsrate, Eskalationsrate, Kosten pro approved Regel. Referenzwerte aus der Literatur: ~89 % syntaktische Validität Few-Shot, ~99 % Soundness mit redundanter Formalisierung.
**M2.4 Worker-Routing** nach deiner Taxonomie: Formalisierung ist hochverifizierbar und selbstcontained, also Worker-Tier (DeepSeek/Kimi/GLM) mit Opus als Orchestrator und Eskalationspfad bei wiederholtem Gate-Fail.

**Gate G2:** 10 reale Regeln aus Anlage N durch die komplette Pipeline, Eskalationsrate und Kosten pro Regel gemessen. Entscheidung über Modellmix.

---

## Phase 3: Regelbibliothek Anlage N + Sonderausgaben (4–6 Wochen, pipelinegetrieben)

Reihenfolge nach Abdeckungsnutzen für den Standard-Arbeitnehmerfall:

1. Entfernungspauschale §9 Abs. 1 Nr. 4 (inkl. Günstigerprüfung ÖPNV-Tatsächlichkeit)
2. Arbeitsmittel, Arbeitszimmer/Homeoffice-Paar (aus S0.3 übernehmen)
3. Fortbildungskosten §9 Abs. 1 Nr. 7 / Abgrenzung Erstausbildung §9 Abs. 6
4. Doppelte Haushaltsführung §9 Abs. 1 Nr. 5
5. Vorsorgeaufwendungen §10 Abs. 1 Nr. 2, 3, 3a inkl. Höchstbetragsrechnung
6. Anlage Kind: Kindergeld/Kinderfreibetrag-Günstigerprüfung §31/§32 (GETTSIM deckt das ab, differentiell testbar)
7. Außergewöhnliche Belastungen §33 inkl. zumutbarer Belastung
8. Haushaltsnahe Dienstleistungen/Handwerker §35a

Pro Regel Definition of Done: literate Catala approved, Clerk-Tests grün, GETTSIM-Differentialtest wo abgedeckt, ELSTER-Feldmapping eingetragen, Quellenfelder vollständig.

Parallel: Golden-Korpus auf 150+ Fälle ausbauen.

---

## Phase 4: ELSTER-Integration (3–4 Wochen, abhängig von ERiC-Zugang)

**M4.1** Pyeric/Erica-Studium abschließen, ERiC lokal lauffähig (Testzertifikat).
**M4.2** XML-Erzeugung aus Feldmodell + Regel-Outputs.
**M4.3** checkESt-Validierung als CI-Schritt: jede Golden-Korpus-Erklärung muss das amtliche Plugin fehlerfrei passieren. Damit sind beide Oracles aktiv.
**M4.4** Testversand mit Testmerker gegen ELSTER-Testumgebung.

Risiko: ERiC ist proprietär, C-Bibliothek, jährliche Plugin-Zyklen, Portal-Bürokratie. Deshalb Antrag bereits in Phase 0 und Erica als Referenz. Fallback für v3: Ausgabe als ausgefüllte Feldliste zur manuellen Übernahme in Mein ELSTER; Versand wird v3.1.

---

## Phase 5: Interview-Layer und Erklärung (3–4 Wochen)

**M5.1 Relevanzpropagation.** Fragegraph deterministisch aus Regelabhängigkeiten generieren: welche Inputs sind unbekannt und für aktivierte Regeln erforderlich; Ausschlüsse schneiden Teilbäume weg. Kein LLM in der Logik.
**M5.2 LLM als Interface.** Fragen natürlichsprachlich formulieren, Freitextantworten in typisierte Inputs parsen (mit Validierung gegen Feldtypen), Rückfragen bei Ambiguität. Temperature 0, Tool-Use für strukturierte Outputs (muesli-Pattern).
**M5.3 Erklärungen.** catala-explain-Integration: pro Ergebnis ein Dokument mit Berechnungsweg, Rechtsquellen, User-Inputs. Deutsch-Lokalisierung des Templates prüfen.
**M5.4 Frontend.** SvelteKit (Entscheidung aus 03/2026 bestätigen oder kippen, kleiner Prototyp genügt).

---

## Phase 6: Dogfooding und Richtungsentscheidung (laufend ab Phase 5)

**M6.1** Eigene ESt-Erklärung 2026 (du + Sandra, Zusammenveranlagung, Arbeitnehmerfall) komplett durch TaxGraph, Ergebnis gegen kommerzielle Software gegenrechnen (drittes, informelles Oracle).
**M6.2** Richtungsentscheidung dokumentieren:
- **Pfad A, Open Source:** deutsche EStG-Catala-Bibliothek publizieren. Alleinstellungsmerkmal, anschlussfähig an SPRIND/Forschung/GETTSIM-Community, baut Reputation.
- **Pfad B, Produkt:** vorher StBerG-Prüfung (Selbstanwendungssoftware ist zulässig, geschäftsmäßige Hilfeleistung in Steuersachen ist reguliert; Grenze sauber ziehen, ggf. anwaltlich klären). AGPL-Fragen entfallen durch Catala/Apache-2-Stack weitgehend, GETTSIM-Lizenz prüfen falls redistribuiert.
- Pfade schließen sich nicht aus (Open-Core).

---

## Querschnitt

**Verifikationsprinzipien (unverhandelbar):**
- Kein LLM-Output wird ohne deterministisches Gate persistiert.
- Divergenzen (Formalisierungen untereinander, Catala vs. GETTSIM, checkESt-Fehler) werden eskaliert, nie gemittelt oder stillschweigend aufgelöst.
- Jede Regel und jeder Parameter trägt maschinenlesbare Rechtsquelle und VZ-Gültigkeit.
- Golden-Korpus wächst monoton; kein Release bei rotem Korpus.

**Risiken:**
| Risiko | Eintritt | Mitigation |
|---|---|---|
| Catala-Compiler-Instabilität | mittel | Gate G0, Fallback eigene IR, Zulip-Kontakt zum Inria-Team |
| ERiC-Zugang verzögert | mittel–hoch | Antrag Phase 0, Fallback Feldliste für Mein ELSTER |
| Scope-Explosion (Steuerrecht ist fraktal) | hoch | harter MVP-Scope, neue Anlagen nur nach Phase 6 |
| Formalization Drift trotz Gates | niedrig–mittel | Round-Trip + Annahmen-Liste + Human Approval, Korpus |
| Jahreswechsel-Pflege (neue Parameter, Gesetzesänderungen) | sicher | Parameterschicht + GETTSIM-Import + Pipeline-Rerun pro VZ |
| StBerG bei Produktisierung | offen | Prüfung vor Pfad B, bis dahin Eigennutzung/Open Source |

**Grobe Zeitlinie (Nebenprojekt-Tempo, LLM-gestützt):**
Phase 0 im Juli, Phasen 1–2 August–September, Phase 3 Oktober–November, Phase 4 parallel ab ERiC-Zugang, Phase 5 Dezember, Dogfooding zur Steuersaison Anfang 2027. Puffer eingeplant; die Erklärung 2026 ist erst ab Q1/2027 fällig, die Deadline ist also natürlich.

**Sofort-Aktionen:**
1. ELSTER-Entwicklerportal-Registrierung starten
2. Catala + Clerk auf arch-desk installieren
3. S0.1 §32a-Formalisierung beginnen
4. GETTSIM installieren, Parameterstruktur sichten
