# Charge 28 — degressive AfA (§7 Abs2) + E-Kfz-Sonderabschreibung (§7 Abs2a) (Zuschnitt, Stufe A, 2026-07-15)

M4 Multi-VZ-Strukturregel (FEHLENDE Regel auch für VZ 2026). Quellen: `estg_p7_2026-07-11` (§ 7 Abs. 2
Booster-Fassung + Abs. 2a E-Kfz), `wtchanceng_2024_art3_estg` (bgbl, 2024er-Fenster 2×/20 %). **2 Regeln.**
Kein Stufe-B ohne Cap-Wort. Alle Anker VOLL-Länge via `_normalize` verifiziert (beide Freezes).

## Gültigkeits-Zeilen je Quelle (Julius-Direktive)

| Quelle | Fassungsstand | Bekannte/anstehende Änderungen |
|---|---|---|
| § 7 Abs. 2 EStG (degressive AfA) | Freeze 2026-07-11: **Booster-Fassung** (Anschaffung 30.6.2025–1.1.2028, 3×/30 %) | 2024er-Fenster (31.3.2024–1.1.2025, 2×/20 %) NICHT in dieser Fassung → separater bgbl-Freeze; Corona-Fenster 2020–2022 (2,5×/25 %) = außerhalb VZ ≥ 2024 |
| § 7 Abs. 2a EStG (E-Kfz-Sonder-AfA) | Freeze 2026-07-11 (Investitionssofortprogramm­G, verkündet 18.7.2025) | neu; Anschaffung 30.6.2025–1.1.2028 |
| bgbl wtchanceng_2024_art3 (2024er degressive) | BGBl 2024 I Nr. 108 (Änderungsbefehl) | ⚠ ÄNDERUNGSTEXT (Einfüge-/Ersetz-Befehle), gemischte Anführungszeichen — Anker exakt |

## Sondersatz-Sweep (verbatim Freeze-Grep)

| # | Fundstelle | Konstruktion | Konsequenz |
|---|---|---|---|
| S1 | § 7 Abs. 2 S. 1 | **„kann der Steuerpflichtige statt der Absetzung … in gleichen Jahresbeträgen"** | Wahlrecht (statt linear) → bool-Bedingung. |
| S2 | § 7 Abs. 2 S. 2 | **„höchstens das Dreifache des … Prozentsatzes … und 30 Prozent nicht übersteigen"** (Booster); bgbl: **„Zweifache … 20 Prozent"** (2024er) | degressive-% = **min(faktor_max × linear-%, prozent_cap)**; Fenster-Kohorte (2024: 2/20; Booster: 3/30). |
| S3 | § 7 Abs. 2 S. 1 | „in fallenden Jahresbeträgen" | Bemessung vom **Restbuchwert** (declining balance) → Restbuchwert = Input, Mehrjahres-Fortschreibung = State-Nachtrag. |
| S4 | § 7 Abs. 2a | **„abweichend von Absatz 1 oder 2 … folgende Beträge in Prozent der Anschaffungskosten"** | E-Kfz: FESTE Staffel 75/10/5/5/3/2 % der **AK** (kein Restbuchwert) → Jahr-Kohorte. |

## Regel 1 — § 7 Abs. 2: degressive AfA (`p7_2_degressive_afa`)

**Wortlaut (Zitatanker `das Dreifache des bei der Absetzung für Abnutzung in gleichen Jahresbeträgen in
Betracht kommenden Prozentsatzes betragen und 30 Prozent nicht übersteigen`, 154 Zeichen voll-verifiziert;
bgbl-Anker „Zweifache … 20 Prozent" 150 Zeichen):** degressive AfA in fallenden Jahresbeträgen, Prozentsatz
höchstens faktor_max × linear-% und höchstens prozent_cap.

- **Rechenkern:** `degressive_prozent = min(faktor_max · linear_prozent, prozent_cap)`; `jahres_afa =
  restbuchwert · degressive_prozent / 100`. **/100-Encoding (Klasse-5-Vermeidung), Cent-Schnitt zuletzt.**
- **Signatur** `DegressiveAfa`: `restbuchwert: money` (Buchwert zu Jahresbeginn; Mehrjahres-Fortschreibung
  = State-Input), `linear_prozent: decimal` (= 100 / Nutzungsdauer, der „in gleichen Jahresbeträgen in
  Betracht kommende" Satz), `faktor_max: decimal` (2 oder 3, Fenster-Kohorte), `prozent_cap: decimal`
  (20 oder 30, Fenster-Kohorte) → `jahres_afa: money`.
- **Fenster-Kohorten (params/kohorten, Andockung wie Kfz-Teiler C17):** Anschaffungsfenster → (faktor_max,
  prozent_cap): **2024er** (31.3.2024–1.1.2025) = (2, 20); **Booster** (30.6.2025–1.1.2028) = (3, 30). Die
  Norm-Konstanten (2/3, 20/30) sind in der Fenster-Kohorte gebunden (nicht freier Caller — Konstanten-Doktrin).
- **Geltungsbedingungen:** `wahlrecht_statt_linear` (S. 1 „kann … statt", bool), `bewegliches_anlagevermoegen`
  (S. 1), `anschaffungsfenster_kohorte` (faktor_max/prozent_cap je Fenster), `restbuchwert_state_fortschreibung`
  (Restbuchwert-Fortschreibung über Jahre = Nachtrag, §7-Abs3-Wechsel degressiv→linear = Nachtrag),
  `obergrenze_faktor_und_cap` (Doppel-Deckel min).
- **Seeds (Grenzfälle):** ND 10 → linear 10 %. **Booster (3/30):** min(3·10, 30) = 30 %; Restbuchwert 10000
  → 3000. **2024er (2/20):** min(2·10, 20) = 20 %; 10000 → 2000. **Cap-Grenzfall** ND 5 → linear 20 %;
  Booster min(3·20=60, 30) = 30 % (Cap bindet) → 10000 → 3000; 2024er min(2·20=40, 20) = 20 % (Cap bindet).

## Regel 2 — § 7 Abs. 2a: E-Kfz-Sonderabschreibung (`p7_2a_ekfz_75`)

**Wortlaut (Zitatanker `75 Prozent, im ersten darauf folgenden Jahr zehn Prozent, im zweiten und dritten
darauf folgenden Jahr jeweils fünf Prozent`, 123 Zeichen voll-verifiziert):** für Elektrofahrzeuge (§ 9
Abs. 2 KraftStG) im Anlagevermögen, Anschaffung 30.6.2025–1.1.2028, abweichend von Abs. 1/2 feste Beträge
in Prozent der AK: **Jahr 0 = 75 %, +1 = 10 %, +2 = 5 %, +3 = 5 %, +4 = 3 %, +5 = 2 %** (Summe 100 %).

- **Rechenkern:** `jahres_afa = anschaffungskosten · prozent_jahr / 100` (fester %-Satz der AK je Jahr-nach-
  Anschaffung, KEIN Restbuchwert). /100-Encoding, Cent-Schnitt zuletzt.
- **Signatur** `EkfzSonderafa`: `anschaffungskosten: money`, `prozent_jahr: decimal` (aus der Staffel-
  Kohorte) → `jahres_afa: money`.
- **Staffel-Kohorte (params/kohorten):** Jahr-nach-Anschaffung → % : {0: 75, 1: 10, 2: 5, 3: 5, 4: 3, 5: 2}.
- **Geltungsbedingungen:** `ekfz_kraftstg_9abs2` (Elektrofahrzeug i.S.d. § 9 Abs. 2 KraftStG),
  `anschaffungsfenster_booster` (30.6.2025–1.1.2028), `staffel_75_10_5_5_3_2_kohorte` (Jahr→%),
  `abweichend_von_abs1_2` (Sonderabschreibung, statt regulärer AfA).
- **Seeds:** (AK 60000, Jahr 0 → 75 %) → 45000 · (60000, Jahr 1 → 10 %) → 6000 · (60000, Jahr 5 → 2 %)
  → 1200 · Summen-Wächter: 75+10+5+5+3+2 = 100 % (voll abgeschrieben nach 6 Jahren).

## Benannte Nachträge Charge 28

- **Restbuchwert-Mehrjahres-Fortschreibung** (degressive AfA über die Nutzungsdauer, State wie § 10d) +
  **§ 7 Abs. 3** Wechsel degressiv → linear (wenn linear günstiger) = eigener Vortrags-/State-Komplex.
- **Corona-Fenster 2020–2022** (2,5×/25 %, § 7 Abs. 2 a. F.) = außerhalb VZ ≥ 2024, nur Doku.
- **§ 7a Abs. 8, Abs. 1 S. 4** (gemeinsame Vorschriften, Kürzung im Anschaffungsjahr pro rata temporis) = Nachtrag.
- E-Kfz: Zusammenspiel mit § 6 Abs. 1 Nr. 4 (1-%-Bemessung) + Restwert nach der Staffel = Nachtrag.

## Offene Punkte für deine Review

1. **R1 degressive AfA** `min(faktor_max · linear_prozent, prozent_cap)`, Fenster-Kohorte (faktor_max/
   prozent_cap) als params/kohorten-Andockung — bestätigen. linear_prozent als Input (= 100/ND) oder
   Nutzungsdauer als int-Input + Regel rechnet 100/ND? Empfehlung: linear_prozent-Input (vermeidet
   Division-Reihenfolge, Kfz-Teiler-Präzedenz).
2. **R1 Restbuchwert** als State-Input (Fortschreibung = Nachtrag) — bestätigen.
3. **R2 E-Kfz** feste Staffel 75/10/5/5/3/2 als Jahr-Kohorte, prozent_jahr-Input — bestätigen; Absatz 2a
   bestätigt (eigener Absatz, abweichend von Abs 1/2).
4. **Fenster-Kohorten-params** (2024er 2/20, Booster 3/30) baue ich LLM-frei direkt (wie A4 Abzinsung) —
   oder erst nach Stufe-A-OK?
5. Cap-Wort Stufe B: 2 Regeln (R1 multi-quellig p7+bgbl, R2 1-quellig p7) → Vorschlag `--cost-cap 0.20`.
