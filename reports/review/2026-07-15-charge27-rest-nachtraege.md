# Charge 27 — Rest-Nachträge Sammelcharge (Zuschnitt, Stufe A, 2026-07-15)

Sammelcharge der benannten Nachträge aus C22-C25. Quellen: `estg_p5_2026-07-14` (§ 5 Abs. 5),
`estg_p15a_2026-07-14` (§ 15a Abs. 1a/3/5), `estg_p6_2026-07-14` (§ 6 Abs. 1 Nr. 3a e — Abzinsungs-
faktor), `estg_p6_2026-07-14`/`estg_p7g` (Kohorten, teils freeze-blockiert). Alle Zitatanker VOLL-Länge
via `_normalize` verifiziert. **Freeze-unabhängige Teile zuerst (Instructor-Auflage).**

## Gültigkeits-Zeilen je Quelle (Julius-Direktive 2026-07-15)

| Quelle | Fassungsstand | Bekannte/anstehende Änderungen |
|---|---|---|
| § 5 Abs. 5 EStG (RAP) | Freeze 2026-07-14, aktuelle Fassung | keine anstehende; Nr. 1/2 stabil |
| § 15a Abs. 1a/3/5 EStG | Freeze 2026-07-14, aktuelle Fassung | stabil |
| § 6 Abs. 1 Nr. 3a e (5,5 % Abzinsung) | Freeze 2026-07-14 | ⚠ Verbindlichkeiten-Abzinsung (Nr. 3 a. F.) ABGESCHAFFT durch 4. Corona-StHG (WJ-Ende > 31.12.2022); RÜCKSTELLUNGEN Nr. 3a e UNVERÄNDERT — dieser Nachtrag betrifft NUR Rückstellungen |
| § 6 Abs. 1 Nr. 4 (Kfz-BLP-E-Grenze) | Freeze trägt AKTUELL 100 000 Euro (Wachstumschancengesetz) | historische Schwellen 60k/70k/80k je Anschaffungsjahr NICHT im Freeze → Alt-Freezes nötig |
| § 7g Abs. 5 (Sonder-AfA) | Freeze trägt 40 % (ab 2024) | 20-%-Altfassung (Anschaffung vor 2024) NICHT im Freeze → Alt-Freeze nötig |

## Teil A — freeze-vorhanden, formalisierbar

### A1 — § 5 Abs. 5 S. 1 Nr. 2: passiver RAP (`p5_5_passiver_rap`)

**Zitatanker (105 Zeichen voll-verifiziert):** „Einnahmen vor dem Abschlussstichtag, soweit sie Ertrag
für eine bestimmte Zeit nach diesem Tag darstellen." **Spiegel-Regel zu `p5_5_aktiver_rap`** (C23),
Passivseite.

- **Signatur** `PassiverRap`: `einnahme: money`, `monate_nach_stichtag: int`, `monate_gesamt: int`
  → `passiver_rap: money`.
- **Rechenkern:** `passiver_rap = einnahme · monate_nach_stichtag / monate_gesamt` (der auf die Zeit
  nach dem Stichtag entfallende Ertrag). Struktur identisch zum aktiven RAP (Division ZULETZT, /12-Lehre,
  Monatsgranularität = Konvention, Boundary monate_nach=0 → 0).
- **Seeds:** (Einnahme 1200, nach 9, gesamt 12) → 900 · (2400, 6, 24) → 600 · **(1200, 0, 12) → 0
  (WÄCHTER)** · (1200, 12, 12) → 1200.

### A2 — § 15a Abs. 3: Einlageminderung-Gewinnhinzurechnung (`p15a_3_einlageminderung`)

**Zitatanker (77 Zeichen voll-verifiziert):** „ist dem Kommanditisten der Betrag der Einlageminderung
als Gewinn zuzurechnen." + Deckel-Anker (86): „in den zehn vorangegangenen Wirtschaftsjahren ausgleichs-
oder abzugsfähig gewesen ist."

- **Rechenkern:** `gewinnhinzurechnung = min(einlageminderung, ausgleichsfaehige_verluste_10j)`.
  Wird das neg. Kapitalkonto durch Entnahmen erhöht (Einlageminderung), ist der Betrag als Gewinn
  hinzuzurechnen (S. 1), GEDECKELT auf die im WJ + 10 vorangegangenen WJ ausgleichs-/abzugsfähig
  gewesenen Verlustanteile (S. 2).
- **Signatur** `Einlageminderung`: `einlageminderung: money` (Betrag, um den neg. KK durch Entnahmen
  entsteht/sich erhöht), `ausgleichsfaehige_verluste_10j: money` (10-Jahres-Aggregat = Sachverhalts-/
  State-Input, mehrjährig wie § 10d) → `gewinnhinzurechnung: money`.
- **Geltungsbedingungen:** `einlageminderung_durch_entnahmen` (deckt_ab „durch Entnahmen entsteht oder
  sich erhöht"), `deckel_10_jahre_ausgleichsfaehig` (deckt_ab 10-J-Anker; das 10-J-Aggregat = Input,
  die mehrjährige Verfolgung = State-Nachtrag), `keine_haftung_abs1_s2` (Abs. 3 S. 1 „soweit nicht …
  Haftung besteht" = Abgrenzung).
- **Seeds:** (Einlageminderung 5000, verluste_10j 8000) → 5000 (Einlageminderung bindet) · (10000, 6000)
  → 6000 (10-J-Deckel bindet) · (0, 8000) → 0 · **(6000, 6000) → 6000 (Grenzfall gleich)**.

### A3 — § 15a Abs. 1a + Abs. 5: Bedingungs-Zeilen (keine Rechenkerne)

- **Abs. 1a nachträgliche Einlagen** (Anker 92: „Nachträgliche Einlagen führen weder zu einer nach-
  träglichen Ausgleichs- oder Abzugsfähigkeit"): NEGATIV-Regel — nachträgliche Einlagen stellen die
  Ausgleichsfähigkeit eines verrechenbaren Verlusts NICHT wieder her. Reine Geltungsbedingung
  `nachtraegliche_einlage_keine_wiederherstellung` an `p15a_1_verrechenbarer_verlust` (C24) = Nachtrag,
  kein money-Output.
- **Abs. 5 andere Unternehmer** (Anker 103: „gelten sinngemäß für andere Unternehmer, soweit deren
  Haftung der eines Kommanditisten vergleichbar ist"): Applicability-Erweiterung (stille Ges., GbR,
  ausl. PersGes, Mitreeder) → Geltungsbedingung `andere_unternehmer_abs5_sinngemaess` an den §-15a-
  Regeln = Nachtrag.

### A4 — Abzinsungsfaktor-params-Tabelle (schließt C22-Auflage B)

**Deterministische Ableitung (Instructor-Ruling msg 1998):** KEIN Mirror-Freeze nötig — die Tabelle
wird aus der GEFROREN­EN Norm-Konstante 5,5 % abgeleitet: `faktor(n) = 1/1,055^n`, kaufmännisch auf
3 Dezimalen (ROUND_HALF_UP). **Stützwert-Verifikation (reproduziert BMF v. 26.05.2005, BStBl I S. 699,
Tabelle 2 EXAKT):** RLZ 1 → 0,948 ✓, RLZ 10 → 0,585 ✓, RLZ 19 → 0,362 ✓ (eigenes Skript bestätigt).
- **⚠ Gültigkeit (Corona-StHG, Instructor-Auflage):** Die Tabelle gilt NUR für **Rückstellungen**
  (§ 6 Abs. 1 Nr. 3a e). Der Verbindlichkeiten-Teil des BMF-Schreibens v. 26.05.2005 ist **obsolet ab
  WJ-Ende > 31.12.2022** (4. Corona-Steuerhilfegesetz — Nr. 3 a. F. abgeschafft).
- **Lieferform:** `params/abzinsung/rueckstellung_5komma5.yaml` (Formel + Herleitungs-Doku + BMF-
  Stützwert-Referenz als Zitat + Corona-StHG-Gültigkeitszeile) + Generier-Skript + pytest-Test
  (Stützwerte 0,948@1 / 0,585@10 / 0,362@19 deterministisch). Ersetzt den `abzinsungsfaktor`-Input von
  `p6_1_3a_abzinsung` (C22) durch Kohorten-Lookup (Restlaufzeit → Faktor), hebt den Konstanten-Schmuggel
  auf. **Amtliche EStH-Anhang-9-Quelle Radware-gesperrt (beide bfinv-Hosts) — dokumentiert; die
  mathematische Ableitung ist stärker als ein Sekundär-Mirror.**

### A5 — Kfz-BLP-E-Grenze aktuell (params-Zeile aus Freeze)

**Zitatanker (76 Zeichen voll-verifiziert):** „der Bruttolistenpreis des Kraftfahrzeugs nicht mehr als
100 000 Euro beträgt." Der p6-Freeze trägt die AKTUELLE Schwelle **100 000 Euro** (Wachstumschancen-
gesetz) für die 0,25-%-Bemessung (E-Kfz, keine CO2-Emission). params-Zeile für VZ mit aktueller
Fassung — die historischen Schwellen sind freeze-blockiert (Teil B).

## Teil B — freeze-blockiert (Instructor-Meldung, Alt-Freezes nötig)

- **B1 Sonder-AfA-20%-Kohorte:** § 7g Abs. 5 Altfassung (Anschaffung VOR 2024 = 20 %). Der p7g-Freeze
  trägt NUR die aktuelle Fassung (**40 %** ab 2024, 50 % IAB) — **kein „20 Prozent" im Freeze**. →
  Alt-Freeze (BGBl-Fassung vor Wachstumschancengesetz) nötig. Melde-Punkt.
- **B2 Kfz-BLP historische Kohorten:** 60k (bis 2023) / 70k / 80k je Anschaffungsjahr. Der p6-Freeze
  trägt nur die aktuelle 100k-Schwelle. → Alt-Freezes je Fassungsgrenze nötig (koppelt an das Multi-VZ-
  Programm M2: VZ-Kohorten). Melde-Punkt.
- **B3 Übergangsverlust-Regel (H 4.6):** bleibt benannter Nachtrag (Instructor msg 1998: EStH Radware-
  gesperrt, R-Mirror trägt kein H; der aktuelle Zustand — Geltungsbedingung `uebergangsgewinn_positiv`
  + Guard — deckt den Verlustfall defensiv). Kein Zwang.

## Offene Punkte für deine Review

1. **A1 passiver RAP** als Spiegel-Regel (struktur- und hinweis-identisch zu p5_5_aktiver_rap, nur
   Einnahme/Ertrag) — bestätigen.
2. **A2 Einlageminderung** min(einlageminderung, 10-J-Aggregat), 10-J-Verfolgung = State-Nachtrag,
   Aggregat als Input — bestätigen; oder ist die 10-J-Mechanik Teil dieser Charge?
3. **A3 Abs1a/Abs5** als reine Geltungsbedingungen (dock an C24-Regeln) — bestätigen.
4. **A4 Abzinsungs-params** als deterministische Ableitung + pytest-Stützwert-Test (LLM-frei, direkt
   baubar) — soll ich das Skript+params+pytest DIREKT bauen (wie DBA-Kataloge/Handregeln, $0), oder erst
   nach deinem Stufe-A-OK?
5. **A5 Kfz-100k** params-Zeile aktuell; B1/B2 warten auf deine Alt-Freezes.
6. Cap-Wort Stufe B: A1 (1 Regel) + A2 (1 Regel) = 2 Pipeline-Regeln → Vorschlag `--cost-cap 0.20`.
   A3/A4/A5 sind LLM-frei (Bedingungen/params/Handableitung).
