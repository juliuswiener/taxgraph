# UI/UX-Gestaltungs-Lab (Quick, Haiku) — Privat-Oberfläche

Instructor, 2026-07-17. Julius-Order „noch ein ideation lab (quick, haiku)". Lauf in der
Instructor-Session mit hartem Haiku-Override auf allen Lab-Agents (Ausnahme vom Fable-Lab-Bann
per Julius-Wort; 14 Agents, 2 Runden × 4 Thinker + 2 Research, 0 Fehler, ~342k Haiku-Token,
~20 min). Seed = der gefallene Produktentscheid (privat, einfacher Input, LLM-Chat-Hilfe) +
alle harten Constraints. Voll-JSON: Scratchpad `uiux_synth.json` (Session-lokal).

## KURZFASSUNG IN KLARTEXT

**Frage:** Wie sieht die App konkret aus und wie fühlt sie sich an?

**Die fünf Gestaltungs-Dimensionen (zusammenhängend):**

1. **Herkunft zum Anfassen.** Jedes Feld trägt sichtbar, woher sein Wert kommt: kleines
   Herkunfts-Abzeichen (Beleg = solide, KI-Vorschlag = schimmernd), antippbar zur ganzen Kette
   Beleg → Extraktion → Vorschlag → Paragraph. Die Kette zeigt sich IM Bestätigungsmoment,
   nicht versteckt in einem Info-Popup vorher.
2. **Bestätigen mit Gefühl für Unsicherheit.** Je unsicherer der Wert, desto bewusster die
   Geste: sicherer Beleg-Wert = ein Tipp; mittlere KI-Konfidenz = gedrückt halten (Fortschritts-
   Ring); niedrige = doppelte Bestätigung. Vertrauen wird körperlich spürbar statt als
   Prozentzahl gelesen. (Research: Ein-Tipp hat 30–40 % Versehens-Quote, Halten ≈ null.)
3. **Navigation ohne 200-Fragen-Wand.** Mobil: immer nur der nächste freigeschaltete
   Wegpunkt + was er am Ergebnis ändert. Desktop: zusätzlich die ganze Karte (Abhängigkeits-
   Graph) für Überblick und gezieltes Springen. Nie ein Fake-Fortschrittsbalken.
4. **Der Bescheid als schrumpfender Ring.** Ergebnis startet als Spanne [min…max]; jede
   Bestätigung zieht sie sichtbar enger. Mobil als konzentrische Ringe: außen wächst das
   Erledigte, innen schrumpft der Möglichkeitsraum. Fortschritt = Form, nicht Zahl.
5. **Der Chat als Berater daneben, nie als Ausfüller.** Erklär-Kanal gleichberechtigt NEBEN
   dem Bestätigen-Knopf (gleiche Größe), situativ angeboten (z. B. bei ELSTER-Widerspruch
   oder großem Vorjahres-Sprung) — er erklärt, verlinkt Paragraph+Beleg, setzt nie Werte.
   Grenze wird als Feature inszeniert: „KI erklärt, du entscheidest."

**Auffälligste neue Ideen:**
- **Gesten-Richtung = Herkunft:** Wisch nach links = aus Beleg übernehmen, nach oben = aus
  Gesetz, nach unten = vorläufig parken; die Richtung wandert mit ins Audit-Log (Beweis
  bewusster Entscheidung).
- **Fehler als Lernquelle statt Stoppschild:** ELSTER-Beanstandung wird als „ELSTER sagt X,
  hier der Paragraph, hier euer Unterschied — prüfen / ändern / begründen" präsentiert.
- **Ring-Geometrie statt Fortschrittsbalken** (s. Dimension 4).

**Wichtigste Spannungen (Design-Entscheide im Bau, nicht Julius-pflichtig):**
Gesten-Grammatik lernen vs. entdecken; Chat immer sichtbar vs. nur situativ; Spanne aktiv
ziehbar vs. nur passiv schrumpfend; welche ELSTER-Fehler hart blocken (deutsche Haftungslage)
vs. nur warnen; Signal-Schwellen gegen Alarm-Müdigkeit.

**Bereits durch Fakten erledigt:** Die Lab-Frage „ist 76 ms echt?" ist gemessen (warm p95
76 ms, seriell je Slot) — Prüfung bei Feld-/Abschnitts-Commit, nie per Tastendruck; Worker-
Pool für Parallelität.

**Einziger sinnvoller Julius-Entscheid:** Mobile-first oder Desktop-first?
Empfehlung: **responsive mit Mobile-Wegpunkt-Fluss als Primärpfad** (Zielgruppe Laien),
Desktop bekommt die Graph-Übersicht als Zusatzansicht. Rest = Paket-B-Design-Raum.

## Technischer Teil

**Board (7 Karten):** Dependency&Navigation (Graph vs. Map, Mobile Waypoints) · Provenance
als Multi-Modal Field Property (Badges/Threads/Swipes) · Decree Range als interaktive,
konfidenz-bewusste Constraint · Confirmation Ritual als Confidence-Choreographie ·
Advisor als Ambient Presence mit symmetrischer Friktion · LLM-Chat als Explain-Channel
(on-demand + kontextgetriggert) · Live-ELSTER-Validierung als interaktive Fehler-Pädagogik.

**Research-Backing (Auswahl):** Hold-to-confirm/2FA-Studien (Versehens-Quoten), Nielsen-
Norman-Touch-Targets (≥1 cm²), Baymard „Demonstrate Intent", California AB 489 (KI-Disclosure
im Interaktionsmoment), Motion.dev-Gesten-Engagement.

**Offene Design-Entscheide (8, für Paket-B-Order):** Mobile/Desktop-Schnitt · Gesten-
Grammatik (vereinheitlichen? A/B Discovery vs. Scaffolding) · KI-Konfidenz-Schwellen
(85/60-Breakpoints, feldtyp-abhängig?) · Chat-Zugriffsmodell · Spannen-Interaktivität ·
ELSTER-Fehler-Tiering (Block vs. Advisory, Haftung) · Latenz-Degradation (gemessen: erledigt)
· Ambient-Signal-Schwellen.

**Einordnung:** Quick-Lab (2 Runden, Haiku) = Ideen-Landkarte fürs Paket-B-Design, kein
Ersatz für die Produkt-Konvergenzen der beiden großen Labs (K1–K6 bleiben Fundament).
Kosten: Haiku-Session-Token (~342k), kein Paid-LLM.
