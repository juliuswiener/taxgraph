# Ideation Lab — TaxGraph UI-Konzept (Synthese, dev-1)

**Salvage-Synthese.** Der Lab-Run `wf_3bbf7742-0f8` (3 Runden geplant) failte in **Runde 3** am
**Session-Limit** („resets 2am Berlin") + temporären Research-Rate-Limits; der Synthese-Agent crashte
mangels Runde-3-Board. **17/27 Agents fertig**: Frame + Runde 1 komplett (5 Discuss + Weave + 4 Research)
+ Runde 2 Discuss/Weave. Dieses Deliverable ist aus dem gecachten **Weave-r2-Board (13 Karten, 2 Runden,
research-informiert)** + den 4 Runde-1-Research-Funden **von Hand synthetisiert** (kein Runde-3-Diskurs).
Kosten Lab: 576k Subagent-Token, 27 Agents, 98 Tool-Uses.

---

## Themen (Cluster + Kern-Einsicht)

1. **Ein Graph, zwei Richtungen.** Der Registry-Abhängigkeitsgraph vorwärts gelesen = Beweis/Trace
   (Glass-Box-Bescheid), rückwärts gelesen = Interview. Fragebogen und Bescheid sind DIESELBE Struktur.
   „Warum werde ich das gefragt?" IST die partielle Upstream-Trace. (Karten Glass-Box, Räderwerk,
   Bidirektionale Maschine, Nebel-Fragebogen)
2. **Der unbestätigte Wert als TYP, nicht als Overlay.** `Vorlaeufig<T, Herkunft>` vs. `Bestaetigt<T>`:
   die Engine rechnet auf Vorläufig, aber das ERiC-Festsetzungs-Gate ist eine TYP-Bedingung — jedes
   cent-bewegende erreichte Blatt MUSS Bestaetigt sein. Damit ist „LLM nie in der Berechnung" eine
   compiler-erzwungene Schranke, keine Disziplin. Zwei-Signal-Membran (Immunologie) = die Bestätigungs-
   Geste. (Karten Vorschlags-Schicht, Membran, Vorlaeufig<T>)
3. **Symmetrische Provenance.** Ein Herkunftsanker pro Sachverhalt-Feld, strukturgleich zum Zitatanker
   auf der Regel-Seite: {quelle ∈ human|beleg|vorjahr|llm, verweis, konfidenz, zustand}. Doppelte
   Buchführung — jedes cent-bewegende Feld bucht gegen genau eine Custody-Quelle; „nur-LLM" ist
   unausgeglichen → Verwahrkonto. (Karten Herkunfts-Bilanz, Replay-Beweis)
4. **Bestätigungslast als abgeleitete Größe.** Nicht Design-Entscheidung, sondern: Bestätigung genau
   dann, wenn triage==grenzfall ODER eine Störung innerhalb der Konfidenz-Bande die festzusetzende
   Steuer bewegt. EINE billige Counterfactual-Engine treibt zusätzlich Frage-Reihenfolge (Value-of-
   Information), Pruning schon-beantworteter Fragen und das Steuer-at-Risk-Gate. NULL LLM. (Karten
   Sensitivitäts-Scheduler, schrumpfender Bescheid, Steuer-at-Risk)
5. **Der Paket-Schnitt fällt aus dem Graphen.** Kern (LLM-frei, deterministisch) vs. Haut (LLM +
   Darstellung) sind natürlich kollisionsfrei trennbar — die Antwort auf die Zwei-Sessions-Frage. (Karte
   Kern-vs-Haut)

---

## Vielversprechendste (mit Research-Backing)

1. **Bidirektionale Trace-Maschine + `Vorlaeufig<T>` (Keystone).** Derselbe Graph erzeugt Beweis
   (vorwärts) UND Interview (rückwärts); der unbestätigte Wert ist ein Typ, das FA-Gate eine Typ-
   Bedingung. **Research-Backing:** „Regeln → Fragebogen" ist in **vier** Ökosystemen production-proven
   (Docassemble seit ~2014, TurboTax/Intuit-Patente, franz. **Publicodes**, **DMN/Camunda**) — Kern-
   mechanismus überall: Regel-Abhängigkeiten → Graph, Variable wird zur Frage gdw. zur Zielauswertung
   gebraucht UND unbekannt. ERiC checkESt liefert **strukturierten** Status (4 Codes + Regel-XML,
   niedrige Latenz) → trägt das Typ-Gate als synchrones Live-Badge.
2. **Sensitivitäts-Scheduler / Steuer-at-Risk.** Bestätigung nur, wo es die Steuer bewegen kann;
   VaR-Tacho schrumpft mit jeder Bestätigung, Abgabe erst bei Steuer-at-Risk ≈ 0. **Research-Backing:**
   „ein Input, minimale Änderung, kippt Schwelle" ist in 3 Feldern gelöst und battle-tested (Goal
   Seek/Tornado; Steuer-Microsimulation **OpenFisca/GETTSIM/PolicyEngine CliffWatch**; XAI-
   Counterfactuals) — **NULL LLM-Aufrufe**, nur reine Engine-Reruns (passt exakt zu „ohne LLM voll
   bedienbar").
3. **Symmetrische Provenance / Herkunfts-Bilanz.** Custody-Anker je Feld, Spiegel des Zitatankers.
   **Research-Backing:** ERiC bringt eine eigene Prüfregel-DSL (FeldAngegeben/FeldNichtAngegeben/
   AlleFelderAngegeben) — unser Manifest-Vokabular dockt daran an, statt es neu zu erfinden.

---

## Neuartigste

- **`Vorlaeufig<T>` als compiler-erzwungene Schranke** — nicht UI-Overlay, sondern Typ; „LLM nie in
  der Berechnung" wird mechanisch garantiert.
- **Der schrumpfende Bescheid / Intervall** — festzusetzende Steuer ab Frage null als [min,max] über
  alle noch unbekannten erreichbaren Inputs; jede Bestätigung verengt; LLM-Werte = gestricheltes
  Innenband (verengen, committen nichts). Fortschritt ohne Fortschrittsbalken-Lüge.
- **Nebel-des-Krieges-Fragebogen** — der backward-chained Fragen-DAG als RTS-Karte unter Nebel;
  abgeleitete/un-askable Knoten decken sich selbst auf oder melden „es kommt darauf an".
- **Zwei-Signal-Membran (Immunologie)** — LLM-Vorschlag = Signal 1 (anerg, inert, null Cent), betritt
  den Engine-Input erst mit Signal 2 (menschliche Bestätigung); abgelehnte Vorschläge hinterlassen eine
  Narbe („erwogen und verworfen").

---

## Spannungen

1. **Einspruchs-Waffe WIDERLEGT (Research).** KEIN amtlicher Kanal (§ 357 AO / ELSTER-Einspruchsformular
   / ERiC / E-Bilanz-XBRL) nimmt ein strukturiertes, maschinenlesbares Provenance-Bündel an — § 357 AO
   verlangt Begründung nur als Soll (Freitext), ELSTER hat 1 Freitextfeld (2000 Zeichen) + PDF-Anhang.
   → Das signierte Provenance-Bündel ist ein **menschenlesbarer Audit-/Berater-Beleg**, KEIN Maschinen-
   Einspruch ans FA. Umdeuten, nicht töten.
2. **Zielnutzer offen — divergiert die HAUT, nicht den Kern.** Glass-Box/Nebel-Fragebogen ziehen zum
   Selbst-Ersteller (Delight/Vertrauen); Herkunfts-Bilanz + Audit-Bündel zum Steuerberater; der Graph-
   Traverser zur API+dünne-UI. Der Keystone dient allen dreien; nur die erste Haut ist zu wählen.
3. **`Vorlaeufig<T>`-Invasivität.** Echter Typ berührt Sachverhalt-YAML-Schema + Runner + ERiC-Gate;
   UI-Overlay wäre schneller, aber schwächer (Disziplin statt Garantie).

---

## Zu entscheiden (Forks)

1. **Erste Haut / Primär-Zielnutzer:** Selbst-Ersteller (Nebel-Fragebogen + schrumpfender Bescheid) vs.
   Steuerberater (Herkunfts-Bilanz + Audit-Bündel) vs. API+dünne UI (Traverser). Kern für alle gleich.
2. **`Vorlaeufig<T>` als echter Typ** (compiler-Gate, invasiver, starke Garantie) vs. UI-Overlay
   (schneller, schwächer). Empfehlung: Typ.
3. **Provenance-Bündel-Scope** nach Widerlegung: nur menschenlesbarer Audit-Trail + Berater-Beleg
   (kein FA-Maschinenkanal) — bestätigen.
4. **Intervall/Steuer-at-Risk im MVP** ja/nein (novel, aber Zusatzaufwand über den Punktschätzer).

---

## ralplan_brief — Zwei parallel baubare, kollisionsfreie Pakete

**Vertrag/Schnittstelle zwischen A und B:** die **Traverser-API** + die **Bindungstabelle**
(`bedingung_id → typisiertes Feld`) + der **`Vorlaeufig<T>`/`Bestaetigt<T>`-Typ**. A liefert Graph +
Typen + deterministische Läufe; B konsumiert sie. Kein geteilter Schreibpfad.

**Paket A — Kern (dev, LLM-frei, deterministisch, testbar):**
- Registry→Fragen-DAG-Compiler: aus den Geltungsbedingungen den backward-chained Abhängigkeitsgraphen.
- Die **heute fehlende** `bedingung_id → typisiertes Feld`-Bindungstabelle: Typ, Einheit, Enum,
  askable?, Fragetext, Hilfe + Beispielwert (DMN-Input-Data-Vokabular; dockt an ERiC-Prüfregel-DSL an).
- Bidirektionaler Graph-Traverser: vorwärts = Trace/Beweis, rückwärts = Interview/„warum diese Frage".
- Sensitivitäts-/Intervall-Engine: reine Engine-Reruns (Goal-Seek/Finite-Differenz), NULL LLM →
  Bestätigungslast, VoI-Frage-Reihenfolge, schrumpfender Bescheid [min,max], Steuer-at-Risk.
- ERiC-Live-Gate-Andockung: checkESt structured status → synchrones Badge + Typ-Gate.
- Assets: `golden/runner.py`, Registry-Geltungsbedingungen, ERiC-Offline-Gate, Sachverhalt-YAML.

**Paket B — Haut (dev, LLM-Vorschlags-Schicht + Darstellung):**
- LLM-Patch-Proposer: Freitext-Interview, Beleg-/PDF-Extraktion (Seite+BBox), Vorjahres-Übernahme als
  Replay unter neuen VZ-Params. Schreibt AUSSCHLIESSLICH `Vorlaeufig<T>`-Patches mit Herkunft +
  Konfidenz — nie einen Endwert.
- Zwei-Signal-Bestätigungs-UI: Beleg ↔ extrahiertes Feld nebeneinander, Konfidenz-Ampel, Pflicht-
  Bestätigung für alles Cent-Bewegende, „Was hat das LLM angefasst"-Diff.
- Die drei Häute auf der Traverser-API: Glass-Box-Bescheid (Zahl→Paragraph ≤2 Klicks), Nebel-
  Fragebogen (Input-Seite), Herkunfts-Bilanz + Audit-Bündel (Berater/Replay).
- Kollisionsfrei zu A: konsumiert nur Traverser-API + Bindungstabelle + Typ.

**Empfehlung Handoff:** Paket A ist der risikoarme, sofort testbare Kern (LLM-frei, an vorhandene
Assets andockend); die Bindungstabelle ist die einzige echte Neu-Arbeit und der kritische Pfad. Paket B
wählt EINE erste Haut nach Zielnutzer-Entscheid. → `ralplan` für Paket A zuerst.
