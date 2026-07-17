# Ideation-Lab-Vergleich — UI-/Eingabe-Schicht (Instructor-Synthese)

Instructor, 2026-07-17. Zwei UNABHÄNGIGE Ideen-Läufe mit identischer Aufgabenstellung, ohne
Austausch (Julius-Weisung). dev-1: `2026-07-17-ideation-lab-ui-konzept.md` (65a0217).
dev-2: Synthese via Bus. Was BEIDE unabhängig gefunden haben, ist belastbar (gleiches Prinzip
wie unsere Golden-Triangulation).

---

## KURZFASSUNG IN KLARTEXT

**Worum ging es:** Wie soll die Eingabe-Oberfläche aussehen, mit der jemand seinen Steuerfall
in TaxGraph eingibt — mit optionaler KI-Hilfe, aber ohne dass die KI je selbst rechnet?

**Was beide Läufe unabhängig sagen (die 6 sicheren Erkenntnisse):**

1. **Wir müssen keinen Fragebogen erfinden — er steckt schon im System.** Unsere Regeln wissen
   heute schon, WANN sie gelten (Geltungsbedingungen). Liest man das rückwärts, ergibt sich
   automatisch: welche Fragen nötig sind, in welcher Reihenfolge, und warum. Dieselbe Struktur
   vorwärts gelesen erklärt jeden Euro im Bescheid ("diese Zahl kommt aus diesem Paragraphen").
   Fragebogen und Bescheid-Erklärung = ein und dasselbe Ding, zweimal benutzt.

2. **KI-Werte werden technisch blockiert, nicht nur per Regel verboten.** Ein Wert, den die KI
   vorgeschlagen hat und den noch kein Mensch bestätigt hat, KANN gar nicht in die Endsumme
   fließen — das System weigert sich, eine Steuerzahl auszugeben, solange irgendwo ein
   unbestätigter Wert drinhängt. Kein "bitte nicht", sondern "geht nicht".

3. **Bestätigen heißt: zwei Signale.** KI schlägt vor (Signal 1, bewirkt allein NICHTS),
   Mensch bestätigt mit Klick neben dem Beleg (Signal 2) — erst dann zählt der Wert.
   Beide Läufe kamen unabhängig auf exakt dieses Bild.

4. **Jeder Eingabewert bekommt einen Herkunftsnachweis** — genau wie jede Regel bei uns einen
   Gesetzes-Anker hat: Woher kommt der Wert (selbst getippt / Beleg / Vorjahr / KI-Vorschlag)?
   Wer hat ihn bestätigt? Damit ist der komplette Bescheid lückenlos rückverfolgbar: von der
   Endsumme bis zum Beleg UND bis zum Gesetzestext.

5. **ELSTER-Prüfung (ERiC) als eingebaute zweite Meinung** — läuft lokal mit, meldet "das würde
   das Finanzamt so annehmen / nicht annehmen". Wichtig: abgeschnittene Prüfläufe dürfen nie
   als "alles ok" durchgehen (unser bekanntes Falsches-Grün-Thema).

6. **Die Arbeit teilt sich sauber in zwei Pakete** für die zwei Dev-Sessions: Paket A = der
   Rechenkern-Unterbau (ohne jede KI, sofort testbar), Paket B = die Oberfläche + KI-Vorschläge.
   Die beiden berühren sich nur über eine klar definierte Schnittstelle → kollisionsfrei.

**Einziger Streitpunkt der beiden Läufe:** Ist die ELSTER-Prüfung schnell genug, um live beim
Tippen mitzulaufen, oder muss sie im Hintergrund laufen? → Wird schlicht GEMESSEN (ERiC liegt
lokal, kostet nichts). Order an dev-2 ist raus.

**Was DU entscheiden musst (Empfehlung jeweils fett):**

| # | Frage | Optionen | Empfehlung |
|---|---|---|---|
| 1 | Für wen bauen wir die erste Oberfläche? | Privatperson / **Steuerberater** / nur API | **Steuerberater zuerst** — unsere Stärke ist Nachweisbarkeit, das zahlt dort am meisten; Privat-Oberfläche später auf demselben Unterbau |
| 2 | KI-Sperre fest ins Datenmodell einbauen (aufwendiger, garantiert) oder nur in der Oberfläche (schneller, umgehbar)? | Typ vs. Oberfläche | **Fest einbauen** |
| 3 | ELSTER-Prüfung live oder Hintergrund? | live / async | **Erst messen** (läuft schon) |
| 4 | Wie speichern wir Fälle: Änderungs-Protokoll, Schnappschüsse, oder beides? | — | Bei Paket-A-Design entscheiden, Tendenz: Protokoll + Schnappschüsse |
| 5 | Womit anfangen? | Kern vs. Demo-Durchstich | **Paket A Kern zuerst**; erster Baustein = Tabelle "Bedingung → Eingabefeld" (fehlt heute, alles hängt daran) |
| 6 | Extra-Feature "Steuer-Unsicherheits-Anzeige" (Bescheid als Spanne, die sich beim Bestätigen verengt) gleich mitbauen? | ja/nein | Kein Muss fürs MVP, aber Unterbau dafür gleich mitplanen (billig, keine KI) |
| 7 | Dritte Ideen-Runde nachholen? | ja/nein | **Nein** — 6 Doppelfunde reichen, Kosten sparen (bereits verfügt) |

Ein "Ja, Empfehlungen so umsetzen" von dir genügt; abweichende Einzelentscheide einfach per Nummer.
für privat. einfacher input. hilfestellungen durch llm chat. Ki sperre fest. eric erst messen. protokoll und schnappschüsse. unsicherheit mit einbauen. ideenrunde nicht nachholen. 

## ✅ JULIUS-ENTSCHEID 2026-07-17 (Wortlaut oben, im Dokument hinterlegt)

| # | Entscheid | Konsequenz |
|---|---|---|
| 1 | **PRIVAT** (Selbst-Ersteller) — abweichend von Empfehlung Berater. „Einfacher Input, Hilfestellungen durch LLM-Chat" | Paket-B-Haut = Privat-Fragebogen (Nebel-Muster, einfache Sprache) + LLM-Chat als HILFE-Kanal (erklärt Fragen, extrahiert Vorschläge) — Chat schreibt nur `Vorlaeufig`-Patches, nie Werte |
| 2 | KI-Sperre **fest ins Datenmodell** | `Vorlaeufig<T>`/`Bestaetigt<T>` als echter Typ; Herkunfts-Vektor als Payload; Meet über Input-Kegel |
| 3 | **LIVE-Check** (Julius-Nachtrag nach Messung 7f36939: warm p95 76 ms) | ERiC-Prüfung läuft LIVE über persistenten warmen Daemon, ausgelöst bei Feld-/Abschnitts-Bestätigung (76 ms = unmerklich); nie Keystroke-Spam, nie Fork-per-Call; „in Prüfung"-Zustand nur als Übergangs-Flicker |
| 4 | Store: **Protokoll + Schnappschüsse** (beides) | Event-Log + content-adressierte Snapshots; ERiC-Befund bindet an Snapshot-Hash |
| 5 | (implizit bestätigt) | Paket A Kern zuerst; erster Baustein Bindungstabelle `bedingung_id → typisiertes Feld` |
| 6 | Unsicherheits-Anzeige **MIT einbauen** | Sensitivitäts-/Intervall-Engine ([min,max]-Bescheid, Steuer-at-Risk) fest in Paket A; reine Engine-Reruns, NULL LLM |
| 7 | Keine dritte Ideen-Runde | erledigt |

**NACHTRAG 2026-07-17 (Julius „ok"):** Mobile-first BESTÄTIGT (Wegpunkt-Fluss primär,
Desktop-Graph als Zusatzansicht) + Stack BESTÄTIGT: responsive Web-App, Python-Backend
direkt auf dem produkt/-Kern, schlankes Frontend ohne Framework-Zoo. Paket B startet.

Status: Produktentscheid Eingabe-Schicht = GEFALLEN. Zusammen mit aktiver Hersteller-ID ist
Task #11 (Produktisierung E2E) vollständig entsperrt. Umsetzung als Paket A (Kern) / Paket B
(Privat-Haut + LLM-Chat) an die devs nach Abschluss ihrer laufenden Aufträge.

---

## Technischer Teil (Details zu oben, mit Board-Zuordnung)

### Konvergenzen (beide unabhängig — belastbar)

| # | Konzept | dev-1-Form | dev-2-Form |
|---|---|---|---|
| K1 | Ein Regel-Graph, zwei Leserichtungen: vorwärts = Beweis/Glass-Box-Bescheid, rückwärts = Interview; Fragen aus Geltungsbedingungen berechnet, nie kuratiert | „Bidirektionale Trace-Maschine" (Keystone) | „Fragebogen = Lazy Evaluation des Regel-DAG" |
| K2 | Unbestätigter Wert mechanisch gesperrt | `Vorlaeufig<T>`/`Bestaetigt<T>`-TYP, ERiC-Gate = Typ-Bedingung | Fail-closed Aggregation: Meet über Input-Kegel, Summe strukturell keine Zahl |
| K3 | Zwei-Signal-Bestätigung (beide unabhängig Immunologie-Metapher) | „Zwei-Signal-Membran", Narbe „erwogen und verworfen" | „Zwei-Signal-/T-Zell-Modell", entschieden_via-Audit |
| K4 | Provenance je Feld, strukturgleich zum Zitatanker; „Warum diese Frage" = „Warum dieser Euro" | „Symmetrische Provenance / Herkunfts-Bilanz" | „Vertrauen ist die Kante" (Justification-Objekt) |
| K5 | ERiC als unabhängiges Orakel; Falsch-Grün = benannter Feind | checkESt-Live-Badge + Typ-Gate | Drittes Orakel + fehler_max-Trunkierungs-Sperre |
| K6 | Kern/Haut-Schnitt = Zwei-Dev-Schnitt; Zielnutzer-Fork ändert nur die Haut | Paket A Kern / Paket B Haut | AP-1 Substrat / AP-2 ERiC-Worker+Justification |

### Widerspruch (einziger)

ERiC-Timing: dev-1 „checkESt niedrige Latenz → synchron machbar" vs. dev-2 „Plugin-Laden teuer,
asynchron Pflicht, Feldzustand ‚in Prüfung'". Auflösung: Latenz-Messung (Order an dev-2:
Kaltstart vs. warme Instanz, Median+p95, ESt-Minimalfall + realistischer Fall).

### Komplementär (nur je ein Board — Prüfkandidaten, kein Doppel-Beleg)

**Nur dev-1:** Sensitivitäts-Scheduler/Steuer-at-Risk (Bestätigungslast, Frage-Reihenfolge,
Abgabe-Gate aus reinen Engine-Reruns, NULL LLM; Referenzen OpenFisca/GETTSIM/Goal-Seek);
schrumpfender Bescheid als [min,max]-Intervall; Bindungstabelle `bedingung_id → typisiertes Feld`
als heute fehlendes Artefakt + kritischer Pfad; NEGATIV-Fund: § 357 AO/ELSTER nehmen KEIN
maschinenlesbares Provenance-Bündel (Bündel = Audit-/Berater-Beleg, kein FA-Kanal);
Vier-Ökosysteme-Beleg Regel→Fragebogen (Docassemble/TurboTax/Publicodes/DMN).

**Nur dev-2:** Vertrauen als VEKTOR Herkunft×Prüftiefe×Haftung statt Leiter (§ 93c/§ 150 Abs. 7/
§ 175b AO-Recherche; IFRS-13-Alternative); Store-Modell-Frage (Event-Log vs. content-adressierter
Snapshot vs. beides); Drei-Orakel-Cockpit mit Split-Annunciator (Uneinigkeit = Oberfläche, nie
auto-versöhnt; ELSTER-Lampe nie grün vor Send); geierlein-Anti-Pattern (Eigen-Reimplementierung
zerstört Orakel-Unabhängigkeit); ERiC-Feldidentifikator-Falle (2 inkompatible Adress-Schemata →
versioniertes Adress-Objekt + Round-Trip-Golden).

Kombinierbar: Typ als Enforcement (dev-1) + Vertrauens-Vektor als Payload im Typ (dev-2);
Meet pro Achse läuft über dem Typ.

### Kosten
dev-1-Lab 576k Subagent-Token (27 Agents, Salvage 2 Runden), dev-2-Lab 81k (28 Agents, komplett),
Instructor-Drittbein 462k vor Abbruch (verworfen; Lehre: keine Labs in der Fable-Session).
Kein externes Paid-LLM.
