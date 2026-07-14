# Gesamt-Abschlussbericht: "EÜR-Programm, DBA, BMF-Mini-Charge 14. alles machen" (Instructor, 2026-07-14)

**Der komplette Auftrag ist an einem Tag abgearbeitet. LLM-Kosten des Tages: $1,25.**

| Phase | Umfang | Stand |
|---|---|---|
| 1 — AN-nahe ESt | 40/40 Regelungsbereiche | ✅ (Chargen 4–13, seit heute früh inkl. Nachträge) |
| — Charge 14 BMF | Riester-Hinzurechnung + §35c-Energieberater-Deckel | ✅ $0 (beide Fragen BMF-wortlautklar gelöst) |
| 2 — EÜR/Selbständige | 14/14 Zeilen, eigener Nenner | ✅ (Chargen 15–19, ~$0,63) |
| 3a — DBA-Anrechnung | §34c Anrechnung/Abzug + §34d-Paket | ✅ (Charge 20, $0,17; Freistellung lief schon über §32b) |
| 3b — Abkommens-Texte | ~100 DBA | 🚫 begründeter Nicht-Gegenstand mit Interface (dba_methode/dba_staat als Inputs) — Formalisierung einzelner DBA nur auf separates Wort |

## Endstand des Regelwerks

**70 Regeln im Manifest** (66 Snapshots verified, fresh-checkout-reproduzierbar für $0),
alle verified_bedingt mit eingefrorener Quelle, Zitatankern, Grenzfall-Seeds und
deklarierten Geltungsbedingungen. Quellenbasis: 83 verifizierte Freezes (Gesetz +
amtliche Vordrucke + neu: BMF-verwaltung mit Nachrang-Härtung). pytest 144, clerk 45/45,
golden 60/60 — durchgehend grün, jede Charge doppelt verifiziert (dev + unabhängiger
Instructor-Nachlauf).

## Was die Reviews vor den Läufen gefangen haben (Auswahl)

§6 Nr. 5 S. 2 fortgeführte AK (Einlage-Deckel wäre falsch gewesen) · 200k-Konstanten-
Schmuggel beim IAB (Konstanten-Doktrin) · Geschenke-Jahressumme je Empfänger ·
§34d-Anker-Mismatch (Nachlauf C20). Judge-Über-Flags sauber neutralisiert: äquivalente
Umformung (4 Belege), Boundary-Richtung (5), Sondersatz-Überlese — alles per
Wortlaut-Grep + clerk-Seed adjudiziert, Registry dokumentiert jede Entscheidung.

## Heute mitgebaute Infrastruktur (alles $0)

- **runs/-Blocker strukturell gelöst:** Snapshot-Mechanik (hash-gesichert, Tamper→FAIL,
  Fresh-Clone-Beweis per echtem git clone).
- **Kosten-Disziplin bewährt:** budget_abbruch-Gate griff im Ernstfall korrekt
  (konservative Schätzung > Cap → sauberer Stopp statt Weiterlauf); Multi-only-Flag
  macht Caps kumulativ.
- **Neue Quellen-Klasse verwaltung** produktiv (BMF-Schreiben, PDF-Extraktion,
  Nachrang-Satz im Prompt mit Byte-Identitäts-Beweis).
- **Dauerhafte Lehren dokumentiert:** auszug-Leitlinie gilt auch für den Judge (C18
  empirisch, C19/C20 proaktiv angewandt → 0 vermeidbare Flags) · money-fremde Regeln
  als Handregel (§11) · Anker nur _normalize-verifiziert (NBSP-Falle).

## Offen (wartet auf dich bzw. extern)

1. **Hersteller-ID** → checkESt-Vollbeweis läuft automatisch an ($ELSTER_HERSTELLER_ID).
2. **Feldmapping Anlage EÜR/AUS** (ELSTER) — sinnvoll nach Hersteller-ID.
3. **Großkomplexe je auf separates Wort:** Bilanzierung (§4 Abs 1/§5) · PersGes/
   Feststellung · einzelne Abkommens-Texte (CH/AT/US) · Benannte klein-Nachträge
   (§4 Abs 4a Überentnahmen, §7g-State, Sonder-AfA-20%-Kohorte, PAP-Freeze).

**Budget:** Tag $1,25; Programm gesamt seit Start ~$13,3 von 24,70 — Rest ~$11,4.
