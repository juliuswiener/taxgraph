# Ideation-Lab-Vergleich — UI-/Eingabe-Schicht (Instructor-Synthese)

Instructor, 2026-07-17. Grundlage: zwei UNABHÄNGIGE Lab-Läufe mit identischem Seed, kein
Austausch bis Abgabe (Julius-Weisung 2026-07-16). dev-1: `2026-07-17-ideation-lab-ui-konzept.md`
(65a0217, Salvage 2 Runden, 27 Agents/576k Token). dev-2: Synthese via Bus (3 Runden komplett,
28 Agents/81k Token). Drittbein (Instructor-Lauf) auf Julius-Wort verworfen (Fable zu teuer).
Methodik wie Golden-Triangulation: unabhängige Konvergenz = stärkstes Signal.

## Konvergenzen (beide unabhängig — belastbar)

| # | Konzept | dev-1-Form | dev-2-Form |
|---|---|---|---|
| K1 | **EIN Regel-Graph, zwei Leserichtungen**: vorwärts = Beweis/Glass-Box-Bescheid, rückwärts = Interview/Fragebogen; Fragen BERECHNET aus Geltungsbedingungen, nie kuratiert | „Bidirektionale Trace-Maschine" (Keystone) | „Fragebogen = Lazy Evaluation des Regel-DAG" (P2) |
| K2 | **Unbestätigter Wert mechanisch gesperrt** — „LLM darf nie eine Zahl SEIN" als erzwungene Struktur, nicht UI-Disziplin | `Vorlaeufig<T>`/`Bestaetigt<T>`-TYP, ERiC-Gate = Typ-Bedingung | Fail-closed Aggregation: Meet über Input-Kegel, Summe strukturell keine Zahl (P4) |
| K3 | **Zwei-Signal-Bestätigung** — beide wählten unabhängig dieselbe Immunologie-Metapher (Kostimulation): LLM-Vorschlag = Signal 1 (inert), menschlicher Akt = Signal 2 | „Zwei-Signal-Membran", Narbe „erwogen und verworfen" | „Zwei-Signal-/T-Zell-Modell", entschieden_via-Audit |
| K4 | **Provenance je Sachverhalts-Feld, strukturgleich zum Zitatanker** — „Warum diese Frage" und „Warum dieser Euro" = dasselbe rekursive Objekt | „Symmetrische Provenance / Herkunfts-Bilanz" (doppelte Buchführung) | „Vertrauen ist die Kante" (Justification-Objekt, N3) |
| K5 | **ERiC als unabhängiges Orakel/Gate; Falsch-Grün = benannter Feind** | checkESt-Live-Badge + Typ-Gate | Drittes Orakel + fehler_max-Trunkierungs-Sperre |
| K6 | **Kern/Haut-Schnitt = natürlicher Zwei-Dev-Schnitt**; Zielnutzer-Fork divergiert NUR die Haut, nie den Kern | Paket A Kern (LLM-frei) / Paket B Haut (LLM+Views) | AP-1 Substrat / AP-2 ERiC-Worker+Justification |

## Widerspruch (einziger echter)

**ERiC-Timing:** dev-1 behauptet checkESt „niedrige Latenz" → synchrones Live-Badge machbar;
dev-2 (aus ERiC-Handbuch + erica-Quellstudie): Plugin-Laden = Kostentreiber, asynchron Pflicht,
Feldzustand „in Prüfung". Beide Boards nennen selbst die Auflösung: **reale lokale ERiC-Latenz
messen** (ERiC 44.2.4.0 liegt unter ~/02_Software/eric, EBV-1 offline) — billigster Aufklärer
der teuersten Unsicherheit, $0, LLM-frei.

## Komplementär (nur je ein Board — Prüfkandidaten, kein Doppel-Beleg)

**Nur dev-1:** Sensitivitäts-Scheduler/Steuer-at-Risk (Bestätigungslast + Frage-Reihenfolge +
Abgabe-Gate aus reinen Engine-Reruns, NULL LLM; production-proven-Referenzen OpenFisca/GETTSIM/
Goal-Seek); schrumpfender Bescheid als [min,max]-Intervall; **Bindungstabelle
`bedingung_id → typisiertes Feld` als heute fehlendes Artefakt + kritischer Pfad**;
NEGATIV-Fund: § 357 AO/ELSTER nehmen KEIN maschinenlesbares Provenance-Bündel (Bündel =
Audit-/Berater-Beleg, kein FA-Kanal); Vier-Ökosysteme-Beleg für Regel→Fragebogen
(Docassemble/TurboTax/Publicodes/DMN).

**Nur dev-2:** Vertrauen als VEKTOR Herkunft×Prüftiefe×Haftung statt Leiter (§ 93c/§ 150 Abs. 7/
§ 175b AO-Recherche; IFRS-13-Alternative); Store-Modell-Frage (Event-Log vs. content-adressierter
Snapshot vs. beides); Drei-Orakel-Cockpit mit Split-Annunciator (Uneinigkeit = Oberfläche, nie
auto-versöhnt; ELSTER-Lampe nie grün vor Send); geierlein-Anti-Pattern (Eigen-Reimplementierung
zerstört Orakel-Unabhängigkeit); ERiC-Feldidentifikator-Falle (2 inkompatible Adress-Schemata →
versioniertes Adress-Objekt + Round-Trip-Golden).

Kombinierbar statt konkurrierend: K2-Mechanik = Typ als Enforcement (dev-1), Vertrauens-Vektor
als Payload IM Typ (dev-2); Meet pro Achse läuft über dem Typ.

## Entscheidungsblock für Julius (konsolidiert, mit Empfehlung)

1. **Erste Haut/Zielnutzer** (einziger Produkt-Fork; Kern identisch für alle drei):
   Empfehlung **Berater-/Prosumer-Werkzeug zuerst** — spielt die Provenienz-/Audit-Stärke aus
   (Herkunfts-Bilanz, Audit-Bündel), verträgt Hersteller-ID-Wartezeit, Selbst-Ersteller-Haut
   später auf demselben Traverser.
2. **Enforcement:** `Vorlaeufig<T>`-Typ echt einziehen (invasiv, aber Garantie) + Herkunfts-
   Vektor als Typ-Payload. Empfehlung: JA.
3. **ERiC-Betriebsmodus:** ERST Latenz-Messung (dev-2-Auftrag, $0), DANN synchron/asynchron
   entscheiden. Empfehlung: Messung sofort als nächster 10er-Schritt.
4. **Store-Modell** (Event-Log/Snapshot/dual): bei Paket-A-Design entscheiden; Tendenz Event-Log
   + content-adressierte Snapshots (passt zur Registry-Ratsche).
5. **Erster Bau-Scope:** Paket A Kern zuerst, Start = Bindungstabelle + Store-Schema (Substrat),
   dann eine vertikale Scheibe end-to-end. Paket B erst nach Zielnutzer-Entscheid (1.).
6. **Intervall/Steuer-at-Risk:** kein MVP-Blocker, aber Sensitivitäts-Engine als A-Baustein
   einplanen (reine Reruns, billig, NULL LLM).
7. **Kein Lab-Neulauf Runde 3** (dev-1-Rückfrage): Konvergenzlage K1–K6 ist belastbar,
   Grenznutzen einer dritten Runde klein. Empfehlung: NEIN, Tokens sparen.

## Kosten
dev-1-Lab 576k Subagent-Token (27 Agents, Salvage), dev-2-Lab 81k (28 Agents, komplett),
Instructor-Drittbein 462k VOR Abbruch (verworfen; Lehre in Memory: keine Labs in der
Fable-Session). Kein externes Paid-LLM.
