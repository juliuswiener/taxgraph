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

## 2. Judge-Instabilität bei Temperatur 0 — gemessen, nicht vermutet

**Das ist der wichtigste Befund der Nacht.** Ich habe die Streuung gemessen statt sie
zu behaupten: `pipeline/judge_stabilitaet.py`, dieselbe Regel, derselbe Catala-Quelltext,
drei Judge-Läufe. Rohdaten in `reports/nachtschicht/judge-stabilitaet.json`.

**§ 9 Abs. 4a — instabil:**

| Lauf | Abweichungen | Annahmen (undeklariert) | wirkt_hinein | roundtrip | geltungsbereich |
|---|---|---|---|---|---|
| 1 | — | — | — | Truncation am Token-Limit | — |
| 2 | 2 | 9 (3) | 5 | FAIL | PASS |
| 3 | 1 | 7 (1) | 6 | FAIL | PASS |

**§ 33 Abs. 3 — stabil:** dreimal identisch, `abweichungen=1` (A's Abrundung),
`wirkt_hinein=0`, gleiche Gate-Urteile. Der Befund gegen A ist damit robust.

**Die Konsequenz ist unangenehm.** Der gespeicherte Report von § 9 Abs. 4a zeigte
`roundtrip=PASS` bei 5/5 gemappten Annahmen und stand auf `verified_bedingt`. **Kein
einziger der drei frischen Läufe reproduziert das.** Dasselbe bei § 35a: der Report
zeigte 12/12 gemappt und alle Gates grün, ein frischer Lauf liefert 12/15 und einen
roten Geltungsbereich.

Beide Regeln stehen jetzt wieder auf `flagged_for_review`. Ihr grüner Zustand war ein
Zufallstreffer der Judge-Streuung, kein Befund über die Formalisierung. Ich habe ihn
nicht stehengelassen.

**Optionen, unverändert deine Entscheidung:**

1. **Hinnehmen und protokollieren.** Der Judge ist ein Vorschlag, das Review entscheidet.
   Kostet nichts, macht `verified_bedingt` aber bedeutungslos: der Status hängt davon ab,
   welchen Wurf man erwischt hat.
2. **Mehrheitsentscheid**, drei Läufe, Mehrheit zählt. Verdreifacht die Judge-Kosten
   (rund 0,05 USD je Regel), macht das Gate stabil und die Streuung selbst messbar.
3. **Zweiter Judge einer anderen Familie**, Dissens eskaliert. Dieselbe
   Dekorrelations-Logik wie beim Formalisierer-Paar, der sauberste Weg — aber eine
   Protokolländerung.

Ich neige zu 2, weil es die Streuung sichtbar macht, bevor wir sie wegdefinieren.
Punkt 1 halte ich nach dieser Messung nicht mehr für vertretbar.

### Zwei Fehler, die die Messung nebenbei aufgedeckt hat

- **Das Judge-Budget war weiterhin zu klein.** § 9 Abs. 4a lief in das erhöhte Limit von
  12.288 Tokens (`finish_reason=length`), § 35a braucht 14.584. Je mehr Geltungsbedingungen
  eine Regel deklariert, desto länger das Verdikt — das Budget wächst mit der Regel, nicht
  mit dem Modell. Jetzt 24.576.
- **`--regate` übersprang bei einem kaputten Verdikt die drei Judge-Gates** und ließ alte
  `PASS`-Werte stehen. So kamen § 9 Abs. 4a und § 35a überhaupt erst zu ihrem grünen
  Status: der Judge war abgeschnitten worden, und der Report behauptete trotzdem, er habe
  geurteilt. Gefixt, mit Regressionstest (`judge_gates`).

---

## 3. Geltungsbedingungen zur Absegnung (Paket)

Insgesamt 26 Bedingungen über fünf Regeln. Alle mit `bedingung`, `deckt_ab`,
`quelle`, `beschreibung`; jeder `deckt_ab`-Anker wörtlich gegen den eingefrorenen
Normtext geprüft.

| Regel | Bedingungen | Annahme-Mapping (letzter Lauf) |
|---|---|---|
| § 9 Abs. 1 S. 3 Nr. 5 | 4 | 1/1 |
| § 9 Abs. 4a | 6 | schwankt, siehe Punkt 2 |
| § 10 Abs. 1 Nr. 7 | 1 | – |
| § 24b | 3 | 3/3 |
| § 35a | 12 | 12/15 |

Die Mapping-Zahlen sind Momentaufnahmen eines Judge-Laufs, keine Eigenschaft der
Regel. Solange Punkt 2 offen ist, sagen sie nur, wie gut der letzte Wurf war.

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

- Gate-Semantik nach Protokolldekret gebaut, 25 Regressionstests (`make unit`).
- Strikte YAML-Loader überall; 93 Bestands-Manifeste geprüft, keines mit Duplikaten.
- Drei Regeln stehen grün (§ 9 Abs. 6 `verified`; § 10 Abs. 1 Nr. 7 und § 24b
  `verified_bedingt`). Zwei weitere standen zwischenzeitlich grün und stehen nach der
  Stabilitätsmessung wieder auf `flagged_for_review` — siehe Punkt 2. Auch die drei
  grünen beruhen auf je einem einzelnen Judge-Wurf; ihre Reproduzierbarkeit ist
  ungeprüft.
- `scripts/freeze_source.py` mit Plausibilitätsprüfung, weil ich beim Einfrieren von
  § 9 und § 6 zwei **leere** Quellen erzeugt habe. `sources-check` war grün — er prüfte
  nur, ob der Hash zum Inhalt passt, und ein leerer Inhalt passt zu seinem Hash. Der
  Verifier misst jetzt den Wortlaut, nicht die Datei.
- Zwei eigene Fehler, von den Guards gefangen: das Numeric-Cheatsheet enthielt eine
  wörtliche Zeile der p09-Referenzregel (Leakage-Guard), und die Strenge des
  Clerk-Gates hing am `PATH` (ohne opam-Umgebung `SKIP` statt `FAIL`).

Kosten der Nachtschicht: 0.3176 USD (Judge-Laeufe und die Stabilitaetsmessung; kein einziger voller Kaskadenlauf).
Charge 1 gesamt: 1.0121 USD.
