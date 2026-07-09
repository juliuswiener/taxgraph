# Offene Entscheidungen — Stand 2026-07-10, Ende der Nachtschicht

Nichts davon habe ich entschieden. Reihenfolge nach Dringlichkeit.

---

## 1. § 33 Abs. 3: Freigabe blockiert, zwei Wege

Der Rechtsfehler ist behoben. Mit dem BFH-Leitsatz als zweiter Quelle formalisiert
**B** die stufenweise Berechnung korrekt und besteht den amtlichen Testfall
(GdE 51.835 € → 1.408,70 €).

**A rundet ab.** Nach dem Numeric-Cheatsheet typecheckt A beim ersten Versuch, aber
es schneidet das Ergebnis auf volle Euro ab (`Decimal.truncate`), was die Norm nicht
tut. Drei unabhängige Gates sagen dasselbe:

| Gate | Befund |
|---|---|
| equivalence | A `$1,346.00` vs B `$1,346.60`, 3 von 5 Rasterpunkten |
| roundtrip | Judge benennt die stille Zusatzannahme „Abrunden auf ganze Euro" |
| clerk | amtlicher BFH-Fall 1.408,70 € fällt bei A |

Das Redundanz-Gate hat also gefeuert, wie es soll. Deine Bedingung war: Freigabe erst,
wenn **entweder** A baut und die Äquivalenz trägt, **oder** wir dokumentiert auf
Einzel-Formalisierung mit verschärftem Review ausweichen.

**Meine Empfehlung: A erneut ansetzen.** A macht den Fehler an genau einer Stelle,
und das Cheatsheet erwähnt Rundung überhaupt nicht — es zeigt `Decimal.truncate` nur
als *erlaubtes* Idiom, ohne zu sagen, wann es fehl am Platz ist. Ein Satz im
Formalisierer-Template („runde nur, wo die Norm es anordnet") ist eine
Prompt-Änderung, keine Protokolländerung, und wirkt auf A und B symmetrisch.

Dagegen spricht: es ist der zweite Prompt-Fix für dieselbe Rolle. Wenn A auch danach
danebenliegt, ist der Re-Evaluations-Trigger für die A-Besetzung ausgelöst.

---

## 2. Judge-Instabilität bei Temperatur 0

Bei `p9_4a_verpflegungsmehraufwand` hat der Judge in zwei Läufen mit **identischem
Input** unterschiedlich geurteilt:

- Lauf 1: eine `abweichung` („gewährt 14 Euro für An- und Abreisetage, ohne die
  Übernachtungsvoraussetzung zu prüfen"), `geltungsbereich` grün.
- Lauf 2: keine `abweichung`, dafür ein zusätzlicher `wirkt_hinein` (Neubeginn-Regel),
  `geltungsbereich` rot.

Beide Verdikte sind vertretbar. Das Problem ist nicht der Inhalt, sondern dass ein
Gate, das über Rechtsregeln entscheidet, bei gleichem Input verschiedene Antworten
gibt. Temperatur ist 0; die Streuung kommt vom Provider (Fireworks, unquantisiert)
und vom Reasoning-Sampling.

**Ich habe das nicht angefasst.** Optionen, die ich sehe:

1. **Hinnehmen und protokollieren.** Der Judge ist ein Vorschlag, das Review
   entscheidet. Kostet nichts, macht `flagged_for_review` aber unzuverlässig.
2. **Mehrheitsentscheid**: den Judge dreimal laufen lassen, Mehrheit zählt. Verdreifacht
   die Judge-Kosten (aktuell rund 0,03 USD je Regel), macht das Gate stabil und die
   Streuung selbst messbar.
3. **Zweiter Judge einer anderen Familie**, Dissens eskaliert. Das ist dieselbe
   Dekorrelations-Logik wie beim Formalisierer-Paar und der sauberste Weg — aber es
   ändert das Protokoll.

Ich neige zu 2, weil es die Streuung sichtbar macht, bevor wir sie wegdefinieren.

---

## 3. Geltungsbedingungen zur Absegnung (Paket)

Insgesamt 26 Bedingungen über fünf Regeln. Alle mit `bedingung`, `deckt_ab`,
`quelle`, `beschreibung`; jeder `deckt_ab`-Anker wörtlich gegen den eingefrorenen
Normtext geprüft.

| Regel | Bedingungen | Annahme-Mapping |
|---|---|---|
| § 9 Abs. 1 S. 3 Nr. 5 | 4 | 1/1 |
| § 9 Abs. 4a | 6 | 5/5 |
| § 10 Abs. 1 Nr. 7 | 1 | – |
| § 24b | 3 | 3/3 |
| § 35a | 12 | 12/12 |

Drei davon hat **der Judge erzwungen**, indem er sich weigerte, eine Annahme auf eine
nur ungefähr passende Bedingung zu mappen — bei § 35a (Arbeitskosten gelten für Abs. 2
*und* 3, meine Bedingung nannte nur Abs. 3), bei § 9 Abs. 4a (Neubeginn nach
vierwöchiger Unterbrechung) und bei § 24b (drei Eingaben tragen Legaldefinitionen).
Das explizite Mapping hat sich damit sofort bezahlt gemacht.

---

## 4. Test-Seed-Vorschläge

- `reports/review/2026-07-10-p35a-cap4000.md` — zwei synthetische Fälle für den
  4.000-Euro-Cap des § 35a Abs. 2, der bisher **ungetestet** ist. Der Randfall
  (20.000 € → genau 4.000 €) unterscheidet eine korrekte Kappung von einer, die schon
  *ab* der Grenze kappt.

## 5. Charge-2-Zuschnitt

- `reports/review/2026-07-10-charge2-zuschnitt.md` — § 9 Abs. 1 S. 3 Nr. 6 + Nr. 7 als
  gemeinsame Multi-Source-Regel mit § 6 Abs. 2 und dem Anschaffungsmonat in der
  Signatur; Neuschnitt von Nr. 5a mit Monatsbetrag statt Jahresbetrag.

---

## Was in der Nacht ohne Rückfrage passiert ist

Alles unter dieser Linie war mechanisch oder folgte einer bereits getroffenen
Entscheidung.

- Gate-Semantik nach Protokolldekret gebaut, 22 Regressionstests (`make unit`).
- Strikte YAML-Loader überall; 93 Bestands-Manifeste geprüft, keines mit Duplikaten.
- Sechs Regeln erreichen einen grünen Zustand (eine `verified`, fünf `verified_bedingt`).
- `scripts/freeze_source.py` mit Plausibilitätsprüfung, weil ich beim Einfrieren von
  § 9 und § 6 zwei **leere** Quellen erzeugt habe. `sources-check` war grün — er prüfte
  nur, ob der Hash zum Inhalt passt, und ein leerer Inhalt passt zu seinem Hash. Der
  Verifier misst jetzt den Wortlaut, nicht die Datei.
- Zwei eigene Fehler, von den Guards gefangen: das Numeric-Cheatsheet enthielt eine
  wörtliche Zeile der p09-Referenzregel (Leakage-Guard), und die Strenge des
  Clerk-Gates hing am `PATH` (ohne opam-Umgebung `SKIP` statt `FAIL`).

Kosten der Nachtschicht: 0,0986 USD (nur `--redo-judge`, kein einziger voller Lauf).
Charge 1 gesamt: 0,9196 USD.
