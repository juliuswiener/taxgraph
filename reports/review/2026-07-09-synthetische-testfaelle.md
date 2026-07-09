# Synthetische Testfälle — abgesegnet von Julius am 2026-07-09

Für diese Regeln existiert **kein amtliches Rechenbeispiel**, das auf die Signatur
passt. Recherchiert wurden EStH/EStR, BMF-Schreiben und BFH-Rechtsprechung;
Steuerberater-Portale sind keine zulässige Quelle.

Die Fälle sind **synthetisch**: die Erwartungswerte sind aus dem eingefrorenen
Normtext hergeleitet, nicht aus einem amtlichen Beispiel abgeschrieben und nicht
von einem Modell geraten. Jeder trägt seinen `rechenweg`.

Dieses Dokument ist der Beleg (`quelle`) der freigegebenen Test-Seeds. Jeder
`zitatanker` unten steht wörtlich **sowohl hier als auch im eingefrorenen
Normtext** — die Prüfung ist damit nicht zirkulär.

## Entscheidungen

| Regel | Entscheidung |
|---|---|
| § 9 Abs. 1 S. 3 Nr. 5 (doppelte Haushaltsführung) | **Übernommen**, alle vier Fälle inkl. Auslandsgrenze. `im_inland` bleibt in der Signatur. |
| § 9 Abs. 1 S. 3 Nr. 6 (Arbeitsmittel) | **Zurückgestellt.** Neuschnitt in Charge 2. |
| § 9 Abs. 1 S. 3 Nr. 7 (AfA) | **Zurückgestellt.** Neuschnitt in Charge 2, gemeinsam mit Nr. 6. |
| § 9 Abs. 6 (Erstausbildung) | **Übernommen**, alle drei Fälle. |
| § 10 Abs. 1 Nr. 7 (Berufsausbildung) | **Übernommen**, alle drei Fälle, plus Geltungsbedingung. |
| § 24b (Entlastungsbetrag) | Kein synthetischer Fall — `herkunft: abgeleitet` aus der Gesetzesmechanik. |
| § 9 Abs. 1 S. 3 Nr. 5a (Übernachtung) | `status: zuschnitt_offen`, erst neu schneiden. |

**Warum `im_inland` bei Nr. 5 in der Signatur bleibt, bei § 9 Abs. 4a aber nicht:**
Bei Abs. 4a erfordert das Ausland ganze BMF-Ländertabellen mit länderweise
unterschiedlichen Pauschbeträgen — das ist eine eigene Datenquelle und gehört in
eine Geltungsbedingung. Bei Nr. 5 ist es ein einzelner Cap im selben Normsatz
(1 000 € Inland, 2 000 € Ausland). Die Grenze verläuft dort, wo eine externe
Datenquelle nötig würde.

---

## 1. § 9 Abs. 1 S. 3 Nr. 5 — doppelte Haushaltsführung

Signatur: `unterkunftskosten_monat: money`, `monate: int`, `im_inland: bool`
→ `abziehbare_unterkunftskosten: money`

**Auflage Julius, umgesetzt:** der Zitatanker `"2 000 Euro"` war zu kurz. Er kommt
im Normtext **zweimal** vor — einmal im Cap („höchstens 2 000 Euro im Monat bei
einer Unterkunft im Ausland") und einmal im Ausnahmesatz („die Grenze von 2 000
Euro … gilt nicht, wenn eine Dienst- oder Werkswohnung …"). Ein Match auf die
falsche Stelle wäre unbemerkt geblieben. Beide Anker sind jetzt auf die umgebende
Wortlaut-Passage verlängert und im Normtext eindeutig (je 1 Treffer).

Vier Geltungsbedingungen decken die Norm-Teile ab, die in den Scope hineinwirken:
`beruflich_veranlasste_doppelte_haushaltsfuehrung`,
`eigener_hausstand_ausserhalb_des_taetigkeitsorts`,
`finanzielle_beteiligung_an_lebensfuehrungskosten`,
`keine_verpflichtende_dienst_oder_werkswohnung`.

```yaml
- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "höchstens 1 000 Euro im Monat bei einer Unterkunft im Inland"
  herkunft: synthetisch
  rechenweg: "Monatsmiete 800 EUR liegt unter der Kappungsgrenze; 800 x 12 = 9.600,00 EUR."
  inputs: {unterkunftskosten_monat: 800, monate: 12, im_inland: true}
  expected: 9600.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "höchstens 1 000 Euro im Monat bei einer Unterkunft im Inland"
  herkunft: synthetisch
  rechenweg: "Monatsmiete 1.400 EUR wird auf 1.000 EUR gekappt; 1.000 x 12 = 12.000,00 EUR."
  inputs: {unterkunftskosten_monat: 1400, monate: 12, im_inland: true}
  expected: 12000.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "höchstens 1 000 Euro im Monat bei einer Unterkunft im Inland"
  herkunft: synthetisch
  rechenweg: "Kappung wirkt je Monat, nicht auf das Jahr: 1.000 x 6 = 6.000,00 EUR."
  inputs: {unterkunftskosten_monat: 1400, monate: 6, im_inland: true}
  expected: 6000.00

- quelle: reports/review/2026-07-09-synthetische-testfaelle.md
  zitatanker: "höchstens 2 000 Euro im Monat bei einer Unterkunft im Ausland"
  herkunft: synthetisch
  rechenweg: "Auslandsunterkunft: Kappung bei 2.000 EUR je Monat; 2.000 x 12 = 24.000,00 EUR."
  inputs: {unterkunftskosten_monat: 2500, monate: 12, im_inland: false}
  expected: 24000.00
```

---

## 2. § 9 Abs. 1 S. 3 Nr. 6 — Arbeitsmittel (zurückgestellt)

Isoliert ist die Regel eine Identitätsfunktion: kein Cap, keine Schwelle. Ein Test
darauf prüft nichts und gibt Scheinsicherheit. Nr. 6 verweist zudem auf Nr. 7
(„Nummer 7 bleibt unberührt"), was der Judge als `wirkt_hinein` gemeldet hat.

Neuschnitt in Charge 2 als **gemeinsame Regel Nr. 6 + Nr. 7**.

---

## 3. § 9 Abs. 1 S. 3 Nr. 7 — AfA (zurückgestellt)

Zurückgestellt aus zwei Gründen:

1. Die 800-Euro-GWG-Grenze steht **nicht** in § 9, sondern ergibt sich aus dem
   Verweis in Satz 2 auf § 6 Abs. 2 EStG. Der Neuschnitt bekommt § 6 Abs. 2 als
   zweite Quelle (`typ: gesetz`, eingefroren) — dasselbe Muster wie § 33 Abs. 3
   mit dem BFH-Leitsatz.
2. **Der Anschaffungsmonat fehlt in der Signatur.** Die AfA ist im Anschaffungsjahr
   zeitanteilig. Eine Signatur aus nur `anschaffungskosten` und
   `nutzungsdauer_jahre` kann das nicht abbilden und bräuchte eine stille
   Volljahr-Annahme — genau die Art von Annahme, die der Round-Trip-Judge zu Recht
   als `stille_zusatzannahme` melden würde.

Der Fall `anschaffungskosten: 800, nutzungsdauer_jahre: 1` war schon im Entwurf
weggelassen: sein Ergebnis hängt an der GWG-Grenze, die die Regel nicht sieht. Ein
Erwartungswert wäre dort geraten gewesen.

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

Geltungsbedingung `hoechstbetrag_gilt_je_person`: Satz 2 gewährt den Höchstbetrag
jedem Ehegatten einzeln. Die Signatur bildet **eine Person** ab; die
Zusammenveranlagungs-Mechanik kommt bei der § 2-Integration.

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
