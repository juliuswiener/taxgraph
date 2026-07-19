# Kz-Kandidaten §§ 13-18-Front (Anlage EÜR / G / S / L) — Read-only-Recon dev-2, 2026-07-19

Akkumulierte Kz=null-Schuld der §§ 13-18-Front (Stufe 1 + 2-I/II/III). Methode: `elster/kz_extract.py`
Sektions-Pfad (E10-2025.html, 2242 Kz) + E77-2025.xsd (Anlage EÜR, 1169 Kz, Sektion leer → Vordruck-primär),
Kreuz gegen amtliches ELSTER-Schema. **KEINE Binding-Writes** — Kz-Zuordnung braucht Instructor-Adjudikation
(Zitatanker-Doktrin auf Kz-Ebene). Confidence: XSD-Label allein = MITTEL; exakter §-Wortlaut + est_mapping/
grund-Match = STARK.

## Proposal-Tabelle

| feld_id | Verzweigung | Anlage | Kz-Kandidat | Schema-Beleg (Label) | Confidence |
|---|---|---|---|---|---|
| rentner_veraeusserungsgewinn | gewinn_betriebsart=gewerbe | G | **E0801301** [VAe_G_FB_Antr] | „Veräußerungsgewinn vor Abzug des Freibetrags nach § 16 Abs. 4 EStG" | **STARK** (est_mapping-Match + Label exakt § 16 Abs. 4) |
| rentner_veraeusserungsgewinn | =selbständig | S | **E0901201** [VAe_G_FB_Antr] | idem (Anlage-S-Instanz) | **STARK** (est_mapping-Match verifiziert) |
| gwg_anschaffungskosten_netto | (EÜR-Instanz) | EÜR | **E6002301** | „Aufwendungen für geringwertige Wirtschaftsgüter nach § 6 Abs. 2 EStG" | **STARK** (elster_kz_grund-Match + Label exakt § 6 Abs. 2) |
| sonstige_betriebsausgaben | (Summand) | EÜR | **E6004901** | „Übrige unbeschränkt abziehbare Betriebsausgaben" | **STARK** (Konzept-Match: laufende Ausgaben ohne AfA) |
| afa_jahresbetrag | (Summand) | EÜR | **E6002101** | „AfA auf bewegliche Wirtschaftsgüter (Übertrag aus Anlage AVEÜR)" | MITTEL-STARK (bewegliche AV; Form splittet AfA nach Asset-Typ: E6002001 immateriell / Gebäude separat) |
| betriebseinnahmen | (1:1-Wert) | EÜR | E6001201 (Summe) **oder** E6000401 (umsatzsteuerpfl.) | „Summe Betriebseinnahmen" / „Umsatzsteuerpflichtige Betriebseinnahmen" | MITTEL (mein Feld = vorberechnetes Aggregat; Form itemisiert Einnahmen-Arten E6000101/301/401/501 → dokumentiert-Aggregat-Mapping wie V+V-WK) |
| einkuenfte_gewinn | =gewerbe | G | [Einz]-Block (E0304703/E0306801/E0307701 „Gewinn/Verlust", Kandidaten) | E10 Sektion [Einz] | SCHWACH-MITTEL (mehrere [Einz]-Zeilen; Vordruck-Zeile-Cross-Check offen — welche = laufender Einzelunternehmer-Gewinn) |
| einkuenfte_gewinn | =selbständig | S | E0900202 „Gewinn VZ-1/VZ (VZ)" **oder** E0900301 „Gewinn VZ/VZ+1" [P4_Abs_1_3] | E10 Sektion [P4_Abs_1_3] | MITTEL (2 Kandidaten; Wirtschaftsjahr-Nuance: Kalenderjahr-MVP → welche „(VZ)"-Zeile) |
| einkuenfte_gewinn | =land_forst | L | E0900405 / E0900502 [P13a] „Gewinn" (§ 13a Durchschnittssatz) | E10 Sektion [P13a] | SCHWACH (§ 13a ≠ § 13-EÜR-Gewinn; land_forst-EÜR ist per Instructor AUSGESCHLOSSEN → land_forst läuft über Direktwert-Pfad einkuenfte_gewinn, nicht EÜR — die Anlage-L-Gewinnermittlungsart-Zeile ist zu klären) |

## Adjudikations-Punkte (Instructor)

1. **STARK-Block sofort promotbar** (E0801301/E0901201 §16-vg, E6002301 gwg, E6004901 sonstige_BA): exakter
   Schema-Label-Match + bereits in est_mapping/grund dokumentiert. Nur Promotion elster_kz=null → Kz + est_mapping-1:1.
2. **EÜR-Aggregat-Frage** (betriebseinnahmen/afa): meine Felder sind vorberechnete Skalare, die Anlage EÜR
   itemisiert (Einnahmen-Arten; AfA nach Asset-Typ via Anlage AVEÜR). Entweder Aggregat-Zeile (E6001201/E6002101)
   ODER dokumentiert-Aggregat-Bucket (wie V+V). = deine Entscheidung Modell-Granularität.
3. **einkuenfte_gewinn (laufender Gewinn) = niedrigste Konfidenz**: Anlage G/S/L haben je eine „Gewinn"-Zeile,
   aber mit Wirtschaftsjahr-Splits (VZ-1/VZ vs VZ/VZ+1) + [Einz]-Multiplizität in Anlage G. Braucht amtlichen
   Vordruck-Zeile-Cross-Check (Anlage G/S 2025 — liegen NICHT in sources/bfinv/, nur euer_2025/aus_2025). GAP:
   Anlage-G/S-Vordruck-PDF fehlt lokal; nur E10-Schema-Sektion verfügbar.
4. **land_forst-Gewinn**: EÜR ist ausgeschlossen (§ 15/§ 18 only) → einkuenfte_gewinn mit betriebsart=land_forst
   läuft rein über den Direktwert. Anlage-L-Gewinnermittlungsart (§ 13a vs § 4 Abs. 3 LuF) zu adjudizieren.

## Lücke
Anlage-G/S-Vordruck-PDF (formulare-bfinv) fehlt lokal → einkuenfte_gewinn-Kz bleibt E10-Schema-only (MITTEL,
kein STARK-Hub). Beschaffung = Julius-Cap (Download). §16-vg/gwg/EÜR-Ausgaben brauchen es nicht (Schema+grund reichen).

## Promotion-Patch (STARK-Block) — STATUS: ANGEWANDT (Kz-Commit-1, 2026-07-19, nach Instructor-Schema-Verify E6002301/E6004901 gegen E77-2025.xsd)

**ANGEWANDT:** gwg_anschaffungskosten_netto → E6002301, sonstige_betriebsausgaben → E6004901 (Option a, nur
bindung elster_kz, kein est_mapping/api-Change). ⚠ FUND dabei: der Kz-Existenz-Gate (test_bindungstabelle
test_c / e10_kz-Fixture) war E10-ONLY → E60xx (EUER) abgelehnt → Gate-Fix e10_kz += E77-Kz-Quelle (kz_extract
datenart e77) mitcommittet. MITTEL (betriebseinnahmen/afa) + SCHWACH (einkuenfte_gewinn) bleiben deferred.
rentner_veraeusserungsgewinn UNVERÄNDERT (schon in est_mapping VERZWEIGUNG, G/S-Richtung Vordruck-pending).


**Befund 1 — rentner_veraeusserungsgewinn = BEREITS gemappt, KEIN Diff.** est_mapping VERZWEIGUNG:70-71 trägt
schon `{"gewerbe": "E0801301", "selbstaendig": "E0901201"}` (Anlage G/S, Datenart E10). bindung elster_kz:null
ist KORREKT (VERZWEIGUNG-Design, kein 1:1-Kz). → nur Instructor-Schema-Verify (E0801301/E0901201 gegen E10 =
schon STARK bestätigt). Kein Write.

**Befund 2 — Datenart-Naht (WICHTIG, Adjudikation):** E6002301/E6004901 sind Datenart EUER (E60xx), nicht E10.
est_mapping hat KEIN Datenart-Konzept — `deklariere` schreibt jeden elster_kz in EINEN `deklaration`-dict
(deklariere:246-247). E60xx landet dort neben E10-Kz. Split = Submission-Layer (ERiC) per Kz-Präfix
(E60→EUER / E0-E2→E10). Options: (a) simpel — elster_kz setzen, ERiC routet per Präfix (kein est_mapping-Change,
Gates passieren: erlaubte_kz/Phantom/Kollision alle grün); (b) datenart-aware EUER-Bucket in est_mapping (wie
person_b, struktureller Nachtrag). Empfehlung: (a) für MVP, (b) als benannter Nachtrag. DEINE Entscheidung.

**Verbleibende STARK-Writes (2 Felder, elster_kz null→Kz, Option (a)):**

1. `produkt/bindung/bindung_n_vor_gwg.yaml` — gwg_anschaffungskosten_netto (Z.597):
   - `elster_kz: null` → `elster_kz: E6002301`
   - grund → „Anlage-EÜR Kz E6002301 „Aufwendungen für geringwertige Wirtschaftsgüter nach § 6 Abs. 2 EStG"
     (Datenart EUER/E77, ERiC routet per Präfix E60xx). Instanz-Reuse je GWG-Asset (instanz_gruppe:gwg)."
   - Instanz-Effekt: Basis (Instanz 1) → deklaration[E6002301]; __n → anlage_instanzen[gwg] (Kz-Reuse). Kein est_mapping-Eintrag (1:1 via bindung).

2. `produkt/bindung/bindung_an_gesamt.yaml` — sonstige_betriebsausgaben:
   - `elster_kz: null` → `elster_kz: E6004901`
   - grund → „Anlage-EÜR Kz E6004901 „Übrige unbeschränkt abziehbare Betriebsausgaben" (Datenart EUER/E77).
     Speist zugleich den betriebsausgaben-Ring-Slot (slot_beitrag:summand) — Deklaration + Rechnung getrennt."
   - 1:1 (Klasse-1), deklariere auto-schreibt deklaration[E6004901]. Kein est_mapping-Eintrag.

**Gate-Erwartung nach Apply:** test_deklarations_abdeckung grün (E6002301/E6004901 ∈ erlaubte_kz via bindung;
keine 1:1-Kollision; Basis-gwg → deklaration erfüllt Assertion 1). test_bindungstabelle: erfundene-Kz-Negativtest
prüft E-Nr-Format (E\d{7}) — E6002301/E6004901 passen. MITTEL-Block (betriebseinnahmen E6001201/afa E6002101) +
SCHWACH (einkuenfte_gewinn) NICHT in diesem Patch — deferred (Aggregat-Granularität / Vordruck-GAP).

## Schema-Verifikations-Evidenz (unabhängige Re-Verifikation 2026-07-19)

**E77-Datei (EÜR-Kz):** `~/02_Software/eric/doc_extract/ERiC-44.2.4.0/Dokumentation/Datenarten/ElsterErklaerung/EUER/Schema/2025/E77-2025.xsd`
(NICHT elster11_E77_2025_extern). Extraktion = xs:documentation je xs:element.
- E6002301 @Z.1353 → „Aufwendungen für geringwertige Wirtschaftsgüter nach § 6 Abs. 2 EStG"
- E6004901 @Z.2015 → „Übrige unbeschränkt abziehbare Betriebsausgaben (auch zurückgezahlte Hilfen/Zuschüsse …Corona…)"

**§16-vg CType-Hash-Disambiguierung (E10-2025.xsd/HTML):**
- E0801301 = complexType `VAe_G_FB_Antr_592812681_CType` (byte 4.9M im HTML)
- E0901201 = complexType `VAe_G_FB_Antr_163190041_CType` (byte 12.3M)
- → 2 DISTINKTE Felder (verschiedener Hash), KEIN Duplikat. `_G_` = Label-Code (Veräußerungsgewinn/Gewinn/
  FreiBetrag/Antrag), NICHT „Anlage G" — beide §16-vg-Felder tragen `_G_`, kein G/S-Diskriminator.
- ⚠ GRENZE: das Schema (HTML SVG-Diagramm + monolith-XSD hash-CTypes + 7-Zeilen-Nutzdaten-Wrapper) trägt KEINE
  Plaintext-Anlage-Attribution. WELCHE = Anlage G vs S ist schema-allein NICHT STARK-bestätigbar → Anlage-G/S-
  Vordruck (GAP) ODER ELSTER-interne hash→Anlage-Tabelle nötig. est_mapping-Richtung = Prior-Review; Swap wäre
  over-tax-neutral (falsche Anlage-Attribution, gleicher Betrag). Bis Vordruck: est_mapping so lassen.
