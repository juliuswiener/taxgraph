# nr5a Teilregel-Split — Stufe-A-Zuschnitt (Spec-Fix statt Besetzung)

Bake-off-Befund: 0/6 Kandidaten-Läufe lösen das 48-Gate (alle 4 Modellfamilien bauen
unabhängig „immer kappen"). Instructor-Adjudikation msg 1236: bedingte Kappung übersteigt
die Formalisierer-Fähigkeit → zerlegen, bis jedes Stück formalisierbar ist. Die 48er-Schwelle
wird je Teilregel BINÄRE Geltungsbedingung; die Auswahl macht die § 2-Integration/Relevanz-
propagation. Anker per `grep -oF|wc -l` gegen `estg_p9_2026-07-10.txt`, alle [1] verifiziert.
Kein erzwingender Seed (Seeds haben als Erzwinger nachweislich versagt — Repair-Loop-Evidenz).

Ersetzt die Monolith-Regel `p9_1_3_nr5a_uebernachtung` (→ als defekt/aufgelöst markieren).

---

## Teilregel 1 — `p9_1_3_nr5a_uebernachtung_vor_48` (ungekappt)

Zeitraum vollständig VOR Ablauf der 48 Monate → tatsächliche Monatskosten, KEINE Kappung.
Keine Cap-Quelle, keine Inland/Ausland-Bedingung nötig (ungekappt gilt so oder so).

```yaml
- rule_id: p9_1_3_nr5a_uebernachtung_vor_48
  norm: § 9 Abs. 1 S. 3 Nr. 5a EStG (Zeitraum vor Ablauf 48 Monate)
  quellen:
  - typ: gesetz
    label: § 9 Abs. 1 S. 3 Nr. 5a Satz 1 EStG (Grundnorm, ungekappt)
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    zitatanker: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"  # [1]
    auszug: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"
  geltungsbedingungen:
  - bedingung: zeitraum_vollstaendig_vor_48_monate
    deckt_ab: "bis zur Höhe des Betrags nach Nummer 5"                         # [1]  Satz 4 (Schwelle)
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 4 EStG"
    beschreibung: "BINÄRE Auswahlbedingung: gilt nur, wenn der Abrechnungszeitraum vollständig VOR Ablauf der 48 Monate liegt (monate_bisher_am_ort < 48, laufender Monat noch pre-Kappung). Innerhalb dieser Teilregel variiert die Schwelle nichts mehr - die Auswahl vor_48/nach_48 macht die § 2-Integration. Überspannende Zeiträume splittet der Aufrufer an der Schwelle."
  - bedingung: keine_unterbrechung_mit_neubeginn
    deckt_ab: "wenn die Unterbrechung mindestens sechs Monate dauert"          # [1]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 5 EStG"
    beschreibung: "Eigene Neubeginn-Regel Nr. 5a: sechs Monate. Scope kennt die Unterbrechungshistorie nicht."
  - bedingung: kosten_bei_alleiniger_nutzung
    deckt_ab: "die bei alleiniger Nutzung durch den Arbeitnehmer angefallen wären"  # [1]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 3 EStG"
    beschreibung: "Bei gemeinsamer Nutzung nur Kosten bei alleiniger Nutzung. Input-Semantik von uebernachtungskosten_monat."
  - bedingung: erste_taetigkeitsstaette_liegt_nicht_am_ort
    deckt_ab: "an einer Tätigkeitsstätte, die nicht erste Tätigkeitsstätte ist"  # [1]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 1 EStG"
    beschreibung: "Übernachtung an einer Tätigkeitsstätte, die nicht erste Tätigkeitsstätte ist."
  signature:
    scope: UebernachtungskostenVor48
    inputs:
      uebernachtungskosten_monat: money
      monate: int
      monate_bisher_am_ort: int      # nur für die binäre Geltungsbedingung / § 2-Routing; Output hängt NICHT davon ab
    output: abziehbare_uebernachtungskosten   # = uebernachtungskosten_monat * monate (ungekappt)
  test_seed:
  - quelle: reports/review/2026-07-12-nr5a-teilregel-split.md
    zitatanker: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"
    herkunft: synthetisch
    rechenweg: "monate_bisher_am_ort 10 < 48 -> vor_48, ungekappt; 800 x 12 = 9.600,00."
    inputs: {uebernachtungskosten_monat: 800, monate: 12, monate_bisher_am_ort: 10}
    expected: 9600.0
  - quelle: reports/review/2026-07-12-nr5a-teilregel-split.md
    zitatanker: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"
    herkunft: synthetisch
    rechenweg: "WÄCHTER 47er: monate_bisher_am_ort 47 < 48 -> vor_48, ungekappt; 1.400 x 12 = 16.800,00. Genau der Fall, den alle Bake-off-Modelle unbedingt kappten."
    inputs: {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 47}
    expected: 16800.0
  raster:
  - {uebernachtungskosten_monat: 800, monate: 12, monate_bisher_am_ort: 10}
  - {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 10}
  - {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 47}
```

---

## Teilregel 2 — `p9_1_3_nr5a_uebernachtung_nach_48` (gekappt, 1.000/Monat)

Zeitraum vollständig NACH Ablauf der 48 Monate → Kappung auf 1.000 Euro/Monat (Verweis Nr. 5).

```yaml
- rule_id: p9_1_3_nr5a_uebernachtung_nach_48
  norm: § 9 Abs. 1 S. 3 Nr. 5a i.V.m. Nr. 5 Satz 4 EStG (Zeitraum nach Ablauf 48 Monate)
  quellen:
  - typ: gesetz
    label: § 9 Abs. 1 S. 3 Nr. 5a Satz 1 EStG (Grundnorm)
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    zitatanker: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"  # [1]
    auszug: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"
  - typ: gesetz
    label: § 9 Abs. 1 S. 3 Nr. 5 Satz 4 EStG (1.000-Euro-Kappung via Verweis Nr. 5a Satz 4)
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    zitatanker: "1 000 Euro im Monat"                                          # [1]
    auszug: "höchstens 1 000 Euro im Monat bei einer Unterkunft im Inland"
  geltungsbedingungen:
  - bedingung: zeitraum_vollstaendig_nach_48_monate
    deckt_ab: "bis zur Höhe des Betrags nach Nummer 5"                         # [1]  Satz 4 (Schwelle)
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 4 EStG"
    beschreibung: "BINÄRE Auswahlbedingung: gilt nur, wenn der Abrechnungszeitraum vollständig NACH Ablauf der 48 Monate liegt (monate_bisher_am_ort >= 48). Innerhalb dieser Teilregel greift die 1.000er-Kappung ausnahmslos. Auswahl vor_48/nach_48 macht die § 2-Integration; überspannende Zeiträume splittet der Aufrufer."
  - bedingung: unterkunft_im_inland
    deckt_ab: "2 000 Euro im Monat bei einer Unterkunft im Ausland"           # [1]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5 Satz 4 EStG (via Verweis Nr. 5a Satz 4)"
    beschreibung: "MVP-Linie Inland (1.000). Der Verweis auf Nr. 5 zieht die 2.000-Euro-Auslandsgrenze mit; Auslandsfall zurückgestellt."
  - bedingung: keine_unterbrechung_mit_neubeginn
    deckt_ab: "wenn die Unterbrechung mindestens sechs Monate dauert"          # [1]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 5 EStG"
    beschreibung: "Eigene Neubeginn-Regel Nr. 5a: sechs Monate. Scope kennt die Unterbrechungshistorie nicht."
  - bedingung: kosten_bei_alleiniger_nutzung
    deckt_ab: "die bei alleiniger Nutzung durch den Arbeitnehmer angefallen wären"  # [1]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 3 EStG"
    beschreibung: "Bei gemeinsamer Nutzung nur Kosten bei alleiniger Nutzung. Input-Semantik von uebernachtungskosten_monat."
  - bedingung: erste_taetigkeitsstaette_liegt_nicht_am_ort
    deckt_ab: "an einer Tätigkeitsstätte, die nicht erste Tätigkeitsstätte ist"  # [1]
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a Satz 1 EStG"
    beschreibung: "Übernachtung an einer Tätigkeitsstätte, die nicht erste Tätigkeitsstätte ist."
  signature:
    scope: UebernachtungskostenNach48
    inputs:
      uebernachtungskosten_monat: money
      monate: int
      monate_bisher_am_ort: int      # nur für die binäre Geltungsbedingung / § 2-Routing
    output: abziehbare_uebernachtungskosten   # = min(uebernachtungskosten_monat, 1000 Euro) * monate
  test_seed:
  - quelle: reports/review/2026-07-12-nr5a-teilregel-split.md
    zitatanker: "1 000 Euro im Monat"
    herkunft: synthetisch
    rechenweg: "WÄCHTER 48er: monate_bisher_am_ort 48 >= 48 -> nach_48, Kappung; min(1.400, 1.000) x 12 = 12.000,00."
    inputs: {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 48}
    expected: 12000.0
  - quelle: reports/review/2026-07-12-nr5a-teilregel-split.md
    zitatanker: "1 000 Euro im Monat"
    herkunft: synthetisch
    rechenweg: "Unter-Cap nach 48: min(800, 1.000) x 12 = 9.600,00 (Kosten unter der Kappungsgrenze -> tatsächliche Kosten). Zeigt, dass die Kappung eine Obergrenze ist, kein Fixbetrag."
    inputs: {uebernachtungskosten_monat: 800, monate: 12, monate_bisher_am_ort: 48}
    expected: 9600.0
  raster:
  - {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 48}
  - {uebernachtungskosten_monat: 800, monate: 12, monate_bisher_am_ort: 48}
  - {uebernachtungskosten_monat: 1400, monate: 12, monate_bisher_am_ort: 60}
```

---

## Warum das formalisierbar ist, wo der Monolith scheiterte

Der Monolith zwang das Modell zu `if monate_bisher >= 48 then min(k,1000)*m else k*m` — eine
bedingte Kappung, bei der ALLE vier Modellfamilien den else-Zweig fallenließen und unbedingt
kappten. Nach dem Split enthält **jede** Teilregel nur noch EINE Rechenformel ohne Fallunter-
scheidung: vor_48 = `k*m` (kein `min`, keine Bedingung), nach_48 = `min(k,1000)*m` (immer).
Die Schwelle ist reine Geltungsbedingung (Doku + § 2-Routing), nicht mehr Teil der Formel.
Damit gibt es keinen Zweig mehr, den das Modell fallenlassen kann — der Wächter 47er lebt in
einer Regel, die gar keine Kappung KENNT.

## Abgrenzungsregel (Selbstcheck)

- vor_48: monate_bisher_am_ort variiert den Output NICHT (immer ungekappt) → korrekt, die
  Signatur-Varianz kommt aus kosten_monat × monate. Schwelle = Geltungsbedingung, nicht Formel.
- nach_48: dito, Output = min(kosten_monat,1000) × monate; monate_bisher nur Auswahl.
- Beide teilen die vier materiellen Nr.-5a-Bedingungen (Unterbrechung, Alleinnutzung, erste
  Tätigkeitsstätte); nur die Cap-bezogene Inland-Bedingung lebt allein in nach_48.

## Fragen an Instructor

(a) Split-Zuschnitt so freigegeben? (b) Monolith `p9_1_3_nr5a_uebernachtung` als `defekt`/
`aufgeloest_durch_split` markieren (Registry) — oder anders? (c) Nach Freigabe: beide Teilregeln
ins Manifest + regate + Stufe-B-Lauf (glm) — als eigener Mini-Batch oder mit Teil 2 gebündelt?
