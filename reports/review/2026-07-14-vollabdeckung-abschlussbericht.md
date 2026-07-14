# Vollabdeckungs-Programm — Abschlussbericht (Instructor, 2026-07-14)

**Ergebnis: 40/40 = 100 % des AN-nahen Nenners.** Start 2026-07-13 bei 22/40 ≈ 55 %.
Jeder der 40 AN-nahen Regelungsbereiche ist entweder **formalisiert (verified_bedingt)**
oder **begründeter Nicht-Gegenstand** mit Wortlaut-Beleg. Messbasis:
`reports/review/2026-07-13-coverage-landkarte.md` (Nenner-Definition dort, Abschnitt 1).

## Was das heißt — und was nicht

- **Heißt:** Alle Rechenmechaniken, die eine typische Arbeitnehmer-nahe ESt-Erklärung
  (inkl. Kapital, Renten, Vermietung, Tarif-Mechanismen, Förderung) braucht, existieren
  als geprüfte Catala-Regeln: 51 Regeln im Manifest, jede mit eingefrorener Quelle,
  Zitatankern, clerk-Seeds (inkl. Grenzfall-Wächtern) und deklarierten Geltungsbedingungen.
- **Heißt NICHT:** (a) verified_bedingt = gilt UNTER den deklarierten Bedingungen — die
  Bedingungen sind Teil des Ergebnisses, nicht Kleingedrucktes. (b) Benannte Nachträge
  (s.u.) sind bewusst offene, dokumentierte Restmechaniken. (c) Die §2-Integration
  (Zusammenspiel aller Regeln) trägt weiter die arithmetischen Abschlüsse und
  Mehrjahres-States. (d) Amtlicher Vollbeweis (ELSTER checkESt) wartet auf die
  Hersteller-ID.

## Chargen-Bilanz (4–12)

| Charge | Inhalt | Ergebnis |
|---|---|---|
| 4 | § 32b Progressionsvorbehalt | ✅ ~$0,10 |
| 5 | Pauschbeträge/Familie (9 Regeln: §33b-Trio, §24a, §10b, §33a, Kinderbetreuung, Realsplitting) | ✅ ~$0,22 |
| 6 | Kapital §20/§32d (3 Regeln) | ✅ ~$0,06 |
| 7 | Renten §22 Nr. 1 (Kohorten-params) | ✅ ~$0,03 |
| 8 | Vermietung §21 + Gebäude-AfA §7 Abs 4 | ✅ ~$0,11 |
| 9 | §34 Fünftelregelung + §10d Abs 2 Verlustvortrag | ✅ ~$0,10 |
| 10 | §35c energetisch (inkl. Energieberater-Sondersatz-Teilregel) + §21 Abs 2 | ✅ $0,193 |
| 11 | Riester §10a + §§79–86 (5 Regeln, größter Zuschnitt) | ✅ $0,487 |
| 12 | §23 + §22 Nr. 3 + §101 Mobilitätsprämie + FW-Disposition | ✅ $0,209 |
| 13 | Nachtrags-Sammelcharge: §23 Abs 3 S 7 Verlusttopf + §32d KiSt-Formel + §34 Abs 3 (56 %-Satz) | ✅ $0,39 |

Chargen-Kosten gesamt **~$1,89**. LLM-Gesamtverbrauch des Programms inkl. Judge-Batches,
Bake-off und Messläufen **~$11–12** (Ursprungsbudget 24,70); Charge 13 lief auf einem separaten
Tages-Key (~$0,39 von $5).

## Nachtrag Charge 13 (2026-07-14) — benannte Rechen-Nachträge geschlossen

Drei der benannten Nachträge sind jetzt formalisiert und **verified_bedingt**, jedes deterministische
Gate grün (equivalence A≡B, clerk, roundtrip, scope_gap, geltungsbereich, grenzfall):

- `p23_3_verlusttopf` (§ 23 Abs 3 S 7) — gleichjähriger PVG-Verlusttopf, `max(0; gewinn − verlust)`.
- `p32d_1_kirchensteuer` (§ 32d Abs 1 S 4) — KiSt-ermäßigte Abgeltung `(e − 4q)/(4 + k)`.
- `p34_3_ermaessigter_durchschnittssatz` (§ 34 Abs 3) — `max(0,56·ds; 0,14) · min(ao; 5 Mio)`.

**QS-Verschärfung:** die § 34-Abweichungen des mistral-Judge wurden per **unabhängiger
Zweitmeinung** (openai/gpt-5.5, vierte Familie, Provider gepinnt, neutraler Prompt) adjudiziert —
volle Übereinstimmung „alle Fehlalarm", Roh-Verdikt als Audit-Trail
(`pipeline/item_registry/discovery/charge13/adjudikation-gpt-5.5-crosscheck.json`). Details:
`reports/review/2026-07-14-charge13-nachtraege.md`. § 21 Abs 2 Prognosekorridor 50–< 66 % bleibt
**bestätigter Nicht-Gegenstand** (kein Norm-Wortlaut, BMF-Totalüberschussprognose).

## Qualitätssicherung (unverändert scharf)

- Registry-Ratsche: Judge = Detektor, Triage schreibt die Registry, Gates deterministisch.
  nicht_echt/abweichung/grenzfall IMMER vorab durch Instructor-Adjudikation; Whitelist-Buckets
  (nicht_material/backlog/bedingung_neu) darf dev seit Charge 10 selbst triagieren.
- Doppelte Verifikation jeder Charge: eigenes Regate + pytest + Wortlaut-Greps + unabhängige
  Seed-Nachrechnung (Charge 10: 10/10, Charge 11: 21/21, Charge 12: 16/16).
- Judge-Memo, 3 dokumentierte Mistral-Über-Flag-Typen (reife Designs, Sondersatz-Überlese,
  Freigrenzen-Richtung) — alle durch die Ratsche neutralisiert; Praxisregel: abweichung
  immer Wortlaut-Grep + clerk-Seed-Gegencheck, beidseitig.
- Neues Pflicht-Tooling nach 2. Cap-Riss: `run.py --cost-cap` (deterministischer Pre-Call-
  Abbruch, Falschgrün-Sperre leere-Gates→unbewertet in beiden _queue_status-Pfaden,
  Checkpoint-Falle gefixt, 6 Negativtests, pytest 124).

## Offene Ebene (jenseits des Nenners)

**Benannte Nachträge** — nach Charge 13 verbleibend (alle klein, je BMF-/State-gebunden):
Riester-Hinzurechnung voll vs. bereinigt (BMF) · §35c Energieberater-Deckel-Interaktion (BMF) ·
VZ-2028 §10d-Rückfall-Param · §34 Abs 3 S 3 Rest-zvE-Verzahnung (§2-Integration).
*Erledigt durch Charge 13:* §23 Abs 3 S 7 Verlusttopf, §32d KiSt-Formel, §34 Abs 3 (56 %-Satz).
*Bestätigter Nicht-Gegenstand:* §21 Abs 2 Prognosekorridor 50–< 66 % (kein Wortlaut, BMF-Prognose).

**Großkomplexe** (eigene Programme, nur auf explizites Wort):
Selbständige/EÜR (§§4/15/18, Anlage EÜR/S/G, §35) · Ausland/DBA (größter, zuletzt).

**Externe Wecker:** Hersteller-ID → checkESt-Vollbeweis (Stufe b, auto via
$ELSTER_HERSTELLER_ID) · ERiC libcheckESt_2026 · PAP-Freeze (solzg 4-Dezimal) ·
GETTSIM #1209/#1210 · Catala-Releases.

## Empfehlung nächste Richtung (Entscheidungsblock)

1. ~~Charge 13 = Nachtrags-Sammelcharge~~ **✅ erledigt 2026-07-14** ($0,39, alle drei
   verified_bedingt, gpt-5.5-Zweitmeinung). Es sind **keine benannten Rechen-Nachträge mit
   Norm-Wortlaut mehr offen** — die Reste sind BMF-/State-gebunden (s. „Offene Ebene").
2. ELSTER-Vollbeweis, sobald Hersteller-ID da (kein LLM, wartet extern). **← nächster Wert ohne
   Neuzuschnitt.**
3. Optionale Mini-Charge 14 (BMF-Nachträge): Riester-Hinzurechnung + §35c-Energieberater-Deckel —
   nur mit vorher eingefrorener BMF-Quelle (sonst AINA-Verstoß: keine Heuristik ohne Wortlaut).
4. EÜR-Programm (neuer Nenner, eigene Landkarte) — erst nach Julius-Wort.
5. Ausland/DBA zuletzt (größter Komplex).
