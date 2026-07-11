# Charge 2 — Zuschnitts-Report Teil 1 (Neuschnitte) — zur Prüfung

Stufe A, kein Modell-Lauf ($0). Zwei beschlossene Neuschnitte aus
`reports/review/2026-07-10-charge2-zuschnitt.md`. Quellen eingefroren
(Paragraphen-Ebene), Zitatanker gegen die eingefrorenen Dateien geprüft
(je 1 Treffer, verbatim). Signaturen nach der Abgrenzungsregel. Test-Seeds:
kein amtliches Rechenbeispiel → synthetisch mit `rechenweg` **nach deiner
Abgrenzungs-Freigabe** (bis dahin „fehlt bewusst", Clerk-Gate fällt korrekt).

---

## 1. `p9_1_3_nr6_7_arbeitsmittel_afa` — ersetzt nr6 + nr7 (Multi-Source)

Grund: Nr. 6 verweist auf Nr. 7 („Nummer 7 bleibt unberührt" → wirkt_hinein);
isoliert ist Nr. 6 eine Identitätsfunktion. Die 800-Euro-Grenze steht nicht in § 9,
sondern folgt aus Nr. 7 Satz 2 → § 6 Abs. 2 → zweite, etikettierte Quelle (Muster § 33/BFH).

```yaml
- rule_id: p9_1_3_nr6_7_arbeitsmittel_afa
  norm: § 9 Abs. 1 S. 3 Nr. 6 und Nr. 7 EStG
  quellen:
  - typ: gesetz
    label: "§ 9 Abs. 1 S. 3 Nr. 6 und Nr. 7 EStG"
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    zitatanker: "Nummer 7 bleibt unberührt"        # [1 Treffer, verbatim]
    auszug: "Aufwendungen für Arbeitsmittel"        # [1 Treffer]  (Nr. 6)
  - typ: gesetz
    label: "§ 6 Abs. 2 EStG (geringwertige Wirtschaftsgüter)"
    datei: sources/gesetze-im-internet/estg_p6_2026-07-10.txt
    zitatanker: "800 Euro nicht übersteigen"        # [1 Treffer, verbatim]
    auszug: "einer selbständigen Nutzung fähig"     # [1 Treffer]
  signature:
    scope: ArbeitsmittelUndAfa
    inputs:
      anschaffungskosten: money      # netto, ohne Vorsteuer
      nutzungsdauer_jahre: int
      anschaffungsmonat: int         # 1..12
    output: abziehbar
  raster:
  - {anschaffungskosten: 500,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # GWG
  - {anschaffungskosten: 800,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # Grenze (<=)
  - {anschaffungskosten: 801,  nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # knapp drüber -> AfA
  - {anschaffungskosten: 1200, nutzungsdauer_jahre: 3, anschaffungsmonat: 1}   # volles Jahr
  - {anschaffungskosten: 1200, nutzungsdauer_jahre: 3, anschaffungsmonat: 7}   # 6/12
  - {anschaffungskosten: 5000, nutzungsdauer_jahre: 5, anschaffungsmonat: 12}  # 1/12
```

**Abgrenzung:** `anschaffungsmonat` in die SIGNATUR (variiert den Betrag über die
zeitanteilige AfA im Anschaffungsjahr), nicht als Geltungsbedingung. Keine
Geltungsbedingung nötig — die zweite Quelle (§ 6 Abs. 2) deckt den wirkt_hinein-
Verweis extensional ab. Grenzfall 800/801 prüft `<=` vs `<` (echter Divergenzbefund,
kein Rundungsartefakt).

**Seeds (nach Freigabe, synthetisch):** GWG-Zweig (≤800 → Sofortabzug =
anschaffungskosten), AfA-Zweig (anschaffungskosten / nutzungsdauer × Monatsanteil).
Rechenweg je Fall; die Zeitanteil-Rundung (z.B. 5000/5×1/12 = 83,33) ist die
Modell-Entscheidung, die der Grenzfall-Judge sichtbar macht.

---

## 2. `p9_1_3_nr5a_uebernachtung` — Neuschnitt (Monatsbetrag)

Alter Zuschnitt: 5× wirkt_hinein bei 0 unabhängigen — die Signatur (`uebernachtungskosten`,
`monate_taetigkeit`) verfehlte den Ausschnitt: die Norm kappt nach 48 Monaten auf
1 000 Euro **pro Monat** und kennt die Neubeginn-Regel.

```yaml
- rule_id: p9_1_3_nr5a_uebernachtung
  norm: § 9 Abs. 1 S. 3 Nr. 5a EStG
  quellen:
  - typ: gesetz
    label: "§ 9 Abs. 1 S. 3 Nr. 5a EStG"
    datei: sources/gesetze-im-internet/estg_p9_2026-07-10.txt
    zitatanker: "1 000 Euro im Monat"                          # [1 Treffer, verbatim]
    auszug: "notwendige Mehraufwendungen eines Arbeitnehmers für beruflich veranlasste Übernachtungen"  # [1 Treffer]
  signature:
    scope: Uebernachtungskosten
    inputs:
      uebernachtungskosten_monat: money   # statt Jahresbetrag
      monate: int                         # Monate der Auswärtstätigkeit im VZ
      monate_bisher_am_ort: int           # für die 48-Monats-Grenze
    output: abziehbare_uebernachtungskosten
  geltungsbedingungen:
  - bedingung: keine_unterbrechung_mit_neubeginn
    deckt_ab: "..."                        # § 9 Abs. 4a Satz 7 entsprechend; Wortlaut beim Eintrag
    quelle: "§ 9 Abs. 4a Satz 7 EStG (entsprechend)"
  - bedingung: erste_taetigkeitsstaette_liegt_nicht_am_ort
    deckt_ab: "..."
    quelle: "§ 9 Abs. 1 S. 3 Nr. 5a EStG"
```

**Abgrenzung:** `uebernachtungskosten_monat` + `monate` variieren den Betrag →
Signatur. Die 1 000-€/Monat-Kappung greift ab `monate_bisher_am_ort > 48`. Die
Neubeginn-Regel und die Ortsvoraussetzung sind binäre Anwendbarkeit → Geltungs-
bedingungen (Parallele zu § 9 Abs. 4a, wo dieselbe Neubeginn-Regel eine Bedingung trägt).

**Seeds (nach Freigabe, synthetisch):** unter/über der 1 000-€-Monatsgrenze, und
`monate_bisher_am_ort` unter/über 48 (Kappung an vs. aus).

---

## Was ich von dir brauche

Pro Regel: **so**, **anders** (wie), oder **zurückstellen**. Prüf besonders die
Zitatanker (verbatim gegen die eingefrorenen Dateien) und die Signatur/Abgrenzung.
Nach deiner Freigabe: `deckt_ab`/`auszug`-Wortlaute vervollständigen, Seeds mit
rechenweg vorschlagen, dann Manifest-Eintrag → Stufe B.

## Teil 2 (Rest-Charge) — Vorschau, noch NICHT gedraftet

§ 10 Vorsorge (r1–r5), § 31/§ 32 (Kind), § 33 Abs. 1/2, § 10 Abs. 1 Nr. 4,
§ 36 Abs. 2, SolzG. **Blocker:** die Quellen sind noch NICHT eingefroren (nur
§ 10 Abs. 1 Nr. 7, § 10c, § 33 Abs. 3 liegen). Freeze via `scripts/freeze_source.py`
(Netz + Plausibilitätsprüfung, leere-Quellen-Lektion) ist der erste Schritt.
Frage: Teil 2 als nächsten Block (erst alle Quellen einfrieren, dann Signaturen),
oder erst Teil 1 durch Stufe B?
