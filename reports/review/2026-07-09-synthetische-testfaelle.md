# Synthetische Testfälle zur Absegnung — 2026-07-09

Für diese Regeln existiert **kein amtliches Rechenbeispiel**, das auf die Signatur
passt. Recherchiert wurden EStH/EStR, BMF-Schreiben und BFH-Rechtsprechung;
Steuerberater-Portale sind keine zulässige Quelle.

Die Fälle unten sind **synthetisch**: die Erwartungswerte sind aus dem
eingefrorenen Normtext hergeleitet, nicht aus einem amtlichen Beispiel abgeschrieben
und nicht von einem Modell geraten. Jeder trägt seinen `rechenweg`. Der
`zitatanker` verweist auf die Norm, aus der sich der Wert ergibt, und ist gegen den
eingefrorenen Text geprüft.

**Nichts davon ist im Manifest aktiv.** Erst deine Absegnung macht sie zu
Test-Seeds; dann tragen sie `quelle: reports/review/2026-07-09-synthetische-testfaelle.md`
und `herkunft: synthetisch`.

Alle unten verwendeten `zitatanker` wurden zusätzlich gegen den **eingefrorenen
Normtext** geprüft (nicht nur gegen dieses Dokument), damit die Prüfung nicht
zirkulär ist: 8 von 8 stehen dort wörtlich.

Nicht enthalten:
- **§ 24b** — bekam stattdessen `herkunft: abgeleitet` aus der Gesetzesmechanik
  (Abs. 2 Sätze 1/2, Abs. 4). Test-Gate ist grün, keine Absegnung nötig.
- **§ 9 Abs. 1 S. 3 Nr. 5a** — `status: zuschnitt_offen`. Erst neu schneiden,
  dann Testfälle.

---

## 1. § 9 Abs. 1 S. 3 Nr. 5 — doppelte Haushaltsführung

Signatur: `unterkunftskosten_monat: money`, `monate: int`, `im_inland: bool`
→ `abziehbare_unterkunftskosten: money`

Kernsatz der Norm (eingefroren, wörtlich geprüft):
`sources/gesetze-im-internet/estg_p9_abs1nr5_2026-07-09.txt`

Zu deiner Prüfung: Die Norm kappt bei **1 000 Euro im Monat** für Unterkünfte im
Inland. Für Auslandsunterkünfte gilt seit 2024 eine eigene Grenze von 2 000 Euro,
die der Judge zu Recht als `wirkt_hinein` gemeldet hat — Fall 4 unten prüft sie.
Wenn du die Auslandsgrenze **nicht** in der Signatur haben willst, muss Fall 4
entfallen und `im_inland` als Geltungsbedingung deklariert werden. Das ist die
Entscheidung, die ich dir nicht abnehmen kann.

```yaml
- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "höchstens 1 000 Euro im Monat"
  herkunft: synthetisch
  rechenweg: "Monatsmiete 800 EUR liegt unter der Kappungsgrenze; 800 x 12 = 9.600,00 EUR."
  inputs: {unterkunftskosten_monat: 800, monate: 12, im_inland: true}
  expected: 9600.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "höchstens 1 000 Euro im Monat"
  herkunft: synthetisch
  rechenweg: "Monatsmiete 1.400 EUR wird auf 1.000 EUR gekappt; 1.000 x 12 = 12.000,00 EUR."
  inputs: {unterkunftskosten_monat: 1400, monate: 12, im_inland: true}
  expected: 12000.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "höchstens 1 000 Euro im Monat"
  herkunft: synthetisch
  rechenweg: "Kappung wirkt je Monat, nicht auf das Jahr: 1.000 x 6 = 6.000,00 EUR."
  inputs: {unterkunftskosten_monat: 1400, monate: 6, im_inland: true}
  expected: 6000.00
```

Offen (Fall 4, Auslandsgrenze) — nur aufnehmen, wenn `im_inland` in der Signatur
bleiben soll:

```yaml
- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "2 000 Euro"
  herkunft: synthetisch
  rechenweg: "Auslandsunterkunft: Kappung bei 2.000 EUR je Monat; 2.000 x 12 = 24.000,00 EUR."
  inputs: {unterkunftskosten_monat: 2500, monate: 12, im_inland: false}
  expected: 24000.00
```

---

## 2. § 9 Abs. 1 S. 3 Nr. 6 — Arbeitsmittel

Signatur: `aufwendungen: money` → `abziehbar: money`

Die Norm kennt **keinen Cap und keine Schwelle**: Aufwendungen für Arbeitsmittel
sind Werbungskosten. Die Identitätsabbildung ist der ganze Inhalt. Die Testfälle
sind entsprechend arm — das ist kein Mangel der Fälle, sondern der Regel.

```yaml
- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "Aufwendungen für Arbeitsmittel"
  herkunft: synthetisch
  rechenweg: "Kein Cap, keine Schwelle: Aufwendungen sind in voller Hoehe abziehbar."
  inputs: {aufwendungen: 300}
  expected: 300.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "Aufwendungen für Arbeitsmittel"
  herkunft: synthetisch
  rechenweg: "Randfall null: kein Aufwand, kein Abzug."
  inputs: {aufwendungen: 0}
  expected: 0.00
```

**Meine Empfehlung:** diese Regel *nicht* mit einem Test-Gate belegen, sondern
zusammen mit Nr. 7 (AfA) neu schneiden. Nr. 6 verweist auf Nr. 7 („Nummer 7 bleibt
unberührt"), und der Judge hat genau das als `wirkt_hinein` gemeldet. Isoliert ist
Nr. 6 eine Identitätsfunktion, die nichts prüft. Deine Entscheidung.

---

## 3. § 9 Abs. 1 S. 3 Nr. 7 — AfA / GWG-Grenze

Signatur: `anschaffungskosten: money`, `nutzungsdauer_jahre: int`
→ `afa_jahresbetrag: money`

Die 800-Euro-Grenze steht **nicht** in § 9, sondern ergibt sich aus dem Verweis in
Satz 2 auf § 6 Abs. 2 EStG. Der Zitatanker unten verweist auf den Verweis, nicht
auf den Betrag. Sauberer wäre ein Multi-Source-Task mit § 6 Abs. 2 als zweiter
Quelle — analog zu § 33 mit dem BFH-Leitsatz.

**Meine Empfehlung:** § 6 Abs. 2 einfrieren und als `typ: gesetz` zweite Quelle
anhängen, statt die 800 Euro über einen synthetischen Fall einzuschmuggeln. Bis
dahin nur der Fall ohne GWG-Bezug:

```yaml
- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "Absetzungen für Abnutzung"
  herkunft: synthetisch
  rechenweg: "Lineare AfA ueber die Nutzungsdauer: 5.000 / 5 = 1.000,00 EUR je Jahr."
  inputs: {anschaffungskosten: 5000, nutzungsdauer_jahre: 5}
  expected: 1000.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "Absetzungen für Abnutzung"
  herkunft: synthetisch
  rechenweg: "1.200 / 3 = 400,00 EUR je Jahr."
  inputs: {anschaffungskosten: 1200, nutzungsdauer_jahre: 3}
  expected: 400.00
```

Der Fall `anschaffungskosten: 800, nutzungsdauer_jahre: 1` ist **bewusst
weggelassen**: sein Ergebnis hängt an der GWG-Grenze aus § 6 Abs. 2, die die Regel
nicht sieht. Ein Erwartungswert wäre hier geraten.

---

## 4. § 9 Abs. 6 — Erstausbildung

Signatur: `erstausbildung_abgeschlossen: bool`, `im_dienstverhaeltnis: bool`,
`aufwendungen: money` → `abziehbare_werbungskosten: money`

```yaml
- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "Aufwendungen des Steuerpflichtigen für seine Berufsausbildung oder für sein Studium sind nur dann Werbungskosten"
  herkunft: synthetisch
  rechenweg: "Erstausbildung abgeschlossen -> kein Abzugsverbot; 3.000 EUR voll abziehbar."
  inputs: {erstausbildung_abgeschlossen: true, im_dienstverhaeltnis: false, aufwendungen: 3000}
  expected: 3000.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "wenn der Steuerpflichtige zuvor bereits eine Erstausbildung"
  herkunft: synthetisch
  rechenweg: "Erstausbildung nicht abgeschlossen, kein Dienstverhaeltnis -> Abzugsverbot, 0,00 EUR (Abzug nur als Sonderausgabe, § 10 Abs. 1 Nr. 7)."
  inputs: {erstausbildung_abgeschlossen: false, im_dienstverhaeltnis: false, aufwendungen: 3000}
  expected: 0.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "im Rahmen eines Dienstverhältnisses stattfindet"
  herkunft: synthetisch
  rechenweg: "Ausbildungsdienstverhaeltnis -> Abzugsverbot greift nicht; 3.000 EUR voll abziehbar."
  inputs: {erstausbildung_abgeschlossen: false, im_dienstverhaeltnis: true, aufwendungen: 3000}
  expected: 3000.00
```

---

## 5. § 10 Abs. 1 Nr. 7 — Berufsausbildungskosten

Signatur: `aufwendungen: money` → `abziehbare_sonderausgaben: money`

```yaml
- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "bis zu 6 000 Euro im Kalenderjahr"
  herkunft: synthetisch
  rechenweg: "Unterhalb des Hoechstbetrags: 2.000 EUR voll abziehbar."
  inputs: {aufwendungen: 2000}
  expected: 2000.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "bis zu 6 000 Euro im Kalenderjahr"
  herkunft: synthetisch
  rechenweg: "Genau am Hoechstbetrag: 6.000 EUR voll abziehbar."
  inputs: {aufwendungen: 6000}
  expected: 6000.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "bis zu 6 000 Euro im Kalenderjahr"
  herkunft: synthetisch
  rechenweg: "Oberhalb des Hoechstbetrags: Kappung auf 6.000,00 EUR."
  inputs: {aufwendungen: 9000}
  expected: 6000.00
```

Hinweis: Satz 2 verdoppelt den Höchstbetrag bei zusammenveranlagten Ehegatten. Die
Signatur kennt keinen Familienstand. Wenn du die Regel so freigibst, braucht sie
eine Geltungsbedingung `keine_zusammenveranlagung`.

---

## Was ich von dir brauche

Pro Regel eine von drei Antworten:

1. **Fälle so übernehmen** — ich trage sie als `herkunft: synthetisch` ins
   Manifest, `--regate` prüft sie ohne Modellkosten.
2. **Fälle ändern** — sag mir welche Werte, ich trage deine ein.
3. **Regel zurückstellen** — wie § 9 Abs. 1 S. 3 Nr. 5a, Neuschnitt in Charge 2.

Für Nr. 6 (Arbeitsmittel) und Nr. 7 (AfA) empfehle ich Antwort 3 mit Neuschnitt als
gemeinsame Regel plus § 6 Abs. 2 als zweiter Quelle. Ein Test, der nur die
Identitätsfunktion prüft, gibt falsche Sicherheit.
