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

Chargen-Kosten gesamt **~$1,50**. LLM-Gesamtverbrauch des Programms inkl. Judge-Batches,
Bake-off und Messläufen **~$11–12 von 24,70** (Rest ~$13).

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

**Benannte Nachträge** (dokumentiert in Landkarte, je klein):
§23 Abs 3 S 7 Verlusttopf (formalisierbar, p20_6-Präzedenz) · §21 Abs 2 Prognosekorridor
50–66 % (BMF) · Riester-Hinzurechnung voll vs. bereinigt (BMF) · §34 Abs 3 (56 %-Satz) ·
§32d KiSt-Formel · §35c Energieberater-Deckel-Interaktion · VZ-2028 §10d-Rückfall-Param.

**Großkomplexe** (eigene Programme, nur auf explizites Wort):
Selbständige/EÜR (§§4/15/18, Anlage EÜR/S/G, §35) · Ausland/DBA (größter, zuletzt).

**Externe Wecker:** Hersteller-ID → checkESt-Vollbeweis (Stufe b, auto via
$ELSTER_HERSTELLER_ID) · ERiC libcheckESt_2026 · PAP-Freeze (solzg 4-Dezimal) ·
GETTSIM #1209/#1210 · Catala-Releases.

## Empfehlung nächste Richtung (Entscheidungsblock)

1. **Charge 13 = Nachtrags-Sammelcharge** (§23-S7-Verlusttopf, §32d-KiSt, §34 Abs 3,
   §21-Abs2-Korridor-Prüfung): rundet den Nenner auf „keine benannten Rechen-Nachträge
   offen" ab. Geschätzt ~$0,3–0,5. **← Empfehlung: sofort.**
2. ELSTER-Vollbeweis, sobald Hersteller-ID da (kein LLM, wartet extern).
3. EÜR-Programm (neuer Nenner, eigene Landkarte) — erst nach Julius-Wort.
4. DBA zuletzt.
