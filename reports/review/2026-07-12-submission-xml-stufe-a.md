# Submission-XML-Bau — Stufe (a) Struktur-Gate + Builder-Design

Julius-GO Phase 4: Submission-XML-Bau. Alles offline, $0, kein Versand. VZ 2025 = Pipe-Proof
(E10-2025-Schema liegt raw vor); amtliche 2026-Werte, wenn das VZ-2026-Modul nachzieht (Instructor-
External-Watch).

## Toolchain (offline, verifiziert)

Die ERiC-Auslieferung enthält die **rohen XSD-Schemata** (nicht nur die HTML-Renders):
`…/ElsterErklaerung/ESt/Schema/2025/{E10-2025.xsd, E10-2025-Nutzdaten.xsd, elster11_E10_2025_extern.xsd}`
+ `ElsterBasisSchema/Schema/{th000011_extern.xsd (TransferHeader), ndh000011.xsd (NutzdatenHeader)}`.
`xmllint` validiert dagegen — **kein Netz, keine Hersteller-ID**. Envelope-Struktur:
`Elster → TransferHeader + DatenTeil → Nutzdatenblock → NutzdatenHeader + Nutzdaten → E10 (ESt1A/…)`.

## Stufe (a): Struktur-Gate — FUNKTIONSFÄHIG

`elster/submission/validate_xsd.py --prove` (ERIC_DIR gesetzt):
- **Valider Testfall** `testfall_est2025_minimal.xml` → **PASS** (`xmllint … validates`).
- **Kaputter Testfall** (schema-fremdes Element E9999999) → **FAIL** (erwartet).
- **Verdikt: STRUKTUR-GATE FUNKTIONSFÄHIG** (valide→PASS, kaputt→FAIL), offline, ohne Hersteller-ID.

Der valide Testfall ist ein minimales, gegen `elster11_E10_2025_extern.xsd` validierendes ESt-2025-
XML (Envelope + Vorsatz + minimale ESt1A, synthetische Testdaten, Testmerker). Er belegt: das amtliche
2025-Struktur-Gate greift lokal. Bau-Fund: die 2020→2025-Feld-Deltas (umbenannte/entfernte Kz,
Reihenfolge) sind real — der Minimal-Testfall wurde iterativ gegen `xmllint` auf 2025 gebracht.

## Stufe (b): checkESt-Plausibilität — dockt automatisch an

`elster/checkest_gate.py` (Mechanismus bewiesen) läuft die amtliche Plausibilitätsprüfung, sobald
`$ELSTER_HERSTELLER_ID` gesetzt ist (Registrierung läuft). Ehrlich getrennt von (a): (a) ist
Struktur (jetzt grün), (b) ist amtliche Inhaltsprüfung (wartet auf ID).

## Builder-Design (rule-outputs → ESt-Datensatz)

Zielfluss: **kanonischer Sachverhalt + Regel-Outputs → E10-Nutzdaten**. Bausteine:
1. **Envelope** (TransferHeader DatenArt=ESt, Testmerker; NutzdatenHeader; Vorsatz mit
   Pflichtfeldern) — steht als validierende Vorlage.
2. **Feld-Population**: je aktive Regel die deklarierten Inputs in ihr E10-Sub-Element setzen, über
   die **kartierten E-Nummern** aus den Mapping-Tabellen (Anlage N: Entfernung E0203504, ÖPNV
   E0203611, Arbeitsmittel E0204401, Übernachtung E0206301, Verpflegung E0205201/302/409; Vorsorge
   E2000401/E2000801; N-DHF E0207611/116/117). **Werte aus bestehenden clerk-Seeds** → jede Zahl
   hat einen belegten Rechenweg.
3. **Kz→E-Nr-Abschluss-Check**: die im Feldmapping vertagten E-Nrn (KiSt 103/104, €-Summen) lösen
   sich hier über die Schema-Element-Namen deterministisch auf — als Gegen-Check gegen die
   Mapping-Tabellen dokumentieren.

## Ehrlicher Stand + nächstes Inkrement

- **JETZT grün:** Toolchain offline, Struktur-Gate (a) funktionsfähig (valide/kaputt), Pipe-Proof-
  Testfall validiert amtlich, checkESt (b) andockbereit.
- **Nächstes Inkrement:** die covered-rule-Felder mit clerk-Seed-Werten in die validierende Vorlage
  einsetzen (E-Nrn liegen bereit) + je Feld gegen `xmllint` nachziehen (E10-Sub-Element-Platzierung
  ist die iterative Arbeit — die 2020→2025-Deltas zeigen, dass Platzierung/Reihenfolge feld-genau
  geprüft werden muss, kein Raten). Danach: Kz→E-Nr-Abschluss-Check.
- **VZ-Disziplin:** 2025-Struktur = Pipe-Proof, NICHT amtliche Bestätigung unserer 2026-Werte.
