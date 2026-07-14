# DBA-Programm — Scoping-Report Phase 3 (Instructor, 2026-07-14)

Julius-Wort 2026-07-14 ("alles machen"). Kernentscheid dieses Reports: **was von "DBA" ist
formalisierbare EStG-Mechanik, was ist Abkommens-Territorium.**

## Die Struktur des Problems

Ein DBA wirkt in der deutschen ESt über genau ZWEI Mechanik-Kanäle, die BEIDE im EStG
stehen (nicht im Abkommen):

1. **Freistellungsmethode** → freigestellte Einkünfte raus aus der Bemessungsgrundlage,
   rein in den **Progressionsvorbehalt § 32b Abs. 1 Nr. 3** — `p32b_progressionsvorbehalt`
   ist seit Charge 4 **verified_bedingt**. Kanal fertig.
2. **Anrechnungsmethode** → ausländische Steuer wird angerechnet, gedeckelt durch den
   **Anrechnungshöchstbetrag § 34c Abs. 1** — formalisierbar (s. u.).

**WELCHE Methode für WELCHE Einkunft aus WELCHEM Staat gilt, steht im jeweiligen
Abkommen** — das ist der nicht-formalisierbare Teil (Phase 3b, s. u.).

## Phase 3a — EStG-Mechanik (formalisierbar, ~1 Charge)

Freezes liegen (2026-07-14): `estg_p34c` (sha a2b4163d…), `estg_p34d` (sha fca15f27…).

| Regel (Vorschlag) | Norm | Mechanik |
|---|---|---|
| `p34c_1_anrechnung_hoechstbetrag` | § 34c Abs. 1 S. 1–3 | anrechenbar = min(gezahlte ausl. Steuer, durchschnittssatz × ausl. Einkünfte je Staat). Durchschnittssatz = Andockungs-Input (Veranlagung inkl. ausl. Einkünfte nach §§ 32a/32b/34/34a/34b — **exakt das § 34-Abs-3-Durchschnittssatz-Muster**, Wortlaut S. 2 wörtlich im Freeze). **Per-country limitation**: "die auf die Einkünfte aus **diesem Staat** entfällt" → Regel rechnet EINEN Staat; Mehr-Staaten-Schleife = § 2-Integration. KapESt-Ausnahme (§ 32d-Fälle, S. 1 Hs. 2) = Geltungsbedingung. |
| `p34c_2_abzug_statt_anrechnung` | § 34c Abs. 2 | Antrag: ausl. Steuer als Abzug bei Ermittlung der Einkünfte statt Anrechnung — kleine Regel (Abzug = ausl. Steuer), Wahlrecht = bool-Input/Geltungsbedingung. |
| Geltungsbedingungs-Paket | § 34d | Katalog "ausländische Einkünfte" (Zuordnungsfrage) → Geltungsbedingungen, keine Rechenregel. |

Anlage AUS = Feldmapping-Schritt danach (wie Anlage N).
Geschätzt: 2 Regeln + Bedingungspaket ≈ **$0,3–0,5** (multi-quellig-Kalibrierung).

## Phase 3b — Abkommens-Ebene (NENNER-ENTSCHEID, Empfehlung)

**Empfehlung: Abkommens-Texte = begründeter Nicht-Gegenstand mit sauberem Interface.**
- Es gibt ~100 deutsche DBA, jedes ein eigener Staatsvertrag mit eigenem Wortlaut,
  Verhandlungsstand, Protokollen. Formalisierung wäre ein eigenes Programm größer als
  alles bisherige — und die RECHENwirkung läuft vollständig durch die zwei Kanäle oben.
- **Interface-Design:** die Methodenwahl kommt als Sachverhalts-Input/Geltungsbedingung
  (`dba_methode: freistellung|anrechnung`, `dba_staat`) — der Anwender/die Integration
  liefert sie aus dem konkreten Abkommen. Damit ist jede DBA-Konstellation RECHENBAR,
  ohne dass ein Abkommen formalisiert ist.
- Optionaler späterer Ausbau (nur auf eigenes Julius-Wort): einzelne Hochfrequenz-DBA
  (CH, AT, US) als Geltungsbedingungs-Kataloge mit Artikel-Ankern (typ `staatsvertrag`,
  neue Quellen-Klasse analog verwaltung).

**Rest-Themen, benannt:** § 50d/§ 50e (Verfahren/Quellensteuer-Erstattung) = Verfahren,
Nicht-Gegenstand. Anrechnungs-Vortrag gibt es im EStG nicht (kein State-Problem).
Wegzugsbesteuerung § 6 AStG = anderes Gesetz, Nicht-Gegenstand.

## Vorgeschlagene Reihenfolge

1. Charge 15–19 (EÜR) laufen wie geplant; **DBA-3a als Charge 20** danach
   (oder vorgezogen, falls EÜR blockiert — Freezes liegen schon).
2. Phase 3b: Nicht-Gegenstand-Disposition wie empfohlen in die Landkarte, Interface
   dokumentiert. Kein Abkommens-Text wird formalisiert ohne separates Julius-Wort.

## Phase 3a ABGESCHLOSSEN (Charge 20, 2026-07-14)

✅ **p34c_1_anrechnung_hoechstbetrag** (6 Bed.) + **p34c_2_abzug_statt_anrechnung** (3 Bed.)
verified_bedingt. Stufe B $0,1694. anrechnung = min(gezahlte ausl. Steuer, deutsche_est ×
(ausl.Eink/zvE)); Deckel-Grenzfall clerk-bewiesen (10000 > 8000 → 8000). §34d-Katalog als
Geltungsbedingung an p34c_1. 2 abweichung (algebraische Identität est×(ausl/zvE) =
Durchschnittssatz-Anwendung) → nicht_echt (Instructor, 3. Beleg äquivalente-Umformung nach
C13/C16). Per-country + KapESt-§32d-Ausnahme deklariert. Phase 3b (Abkommens-Texte) bleibt
begründeter Nicht-Gegenstand mit Interface (dba_methode/dba_staat).

**Damit ist das gesamte "alles machen"-Programm (Phase 1 AN-nah 40/40 + Phase 2 EÜR 14/14 +
Phase 3a DBA-Anrechnung) durch.** commit-Kette bis f564c32 (EÜR) + C20.
