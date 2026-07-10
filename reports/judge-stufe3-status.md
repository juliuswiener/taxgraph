# Judge-Stabilität Stufe 3 — Umsetzungsstand und ein neuer Befund

Alle sechs Punkte des Dekrets sind gebaut. Ein Validierungslauf auf echten Daten
hat dabei einen Befund geliefert, der die Prämisse von Punkt 2 in Frage stellt.

## Umgesetzt

| Punkt | Inhalt | Status |
|---|---|---|
| 2 | Union-until-Saturation statt fixer 3 Läufe | gebaut, getestet |
| 3 | Neue Kriterien (Gate-Replikation, gb-Split ≤15 %, undeclared-Schwankung ≤1) | im Auswertungscode |
| 4 | Grenzfall-Registry + Gate, Dauersplitter routen fest in Review | gebaut, getestet |
| 5 | Restrisiko wörtlich in Gate-Doku + `docs/setup.md` | erledigt |
| 6 | Judge-Provider Fireworks → Together (geloggter Hash-Sprung) | erledigt |
| 1 | Vervollständigungs-Werkzeug + Draft-Pakete je Regel | Werkzeug gebaut, Drafts erzeugt |

51 Tests grün. Alles committet und gepusht.

## Der neue Befund: das Inventar sättigt nicht

Ein realer Validierungslauf von § 33 Abs. 3 mit dem neuen Union-until-Saturation
(Provider Together, Grenzfall-Gate aktiv) lieferte diese **Sättigungskurve** — neue
Anker je Inventarlauf:

```
[5, 2, 1, 5, 2]
```

Das ist kein Abklingen. Nach dem erwarteten Abflachen (5 → 2 → 1) bringt Lauf 4
wieder **5 neue** Items. Der Deckel bei 5 greift, bevor die Union konvergiert; das
letzte Delta ist 2, nicht 0.

**Interpretation:** Der Recall des Inventars konvergiert nicht gegen einen festen
Item-Satz, sondern sampelt aus einer großen latenten Menge möglicher Prüf-Items.
Union-until-Saturation mildert das (jeder Lauf trägt zur Union bei), aber die
Prämisse „ein paar Läufe reichen, dann steht der Satz" trifft für diese Regel
nicht zu. Ein höherer Deckel würde die Union weiter wachsen lassen, nicht
stabilisieren — die Kurve oszilliert, sie fällt nicht monoton.

Das ist die schärfere Fassung des Befunds von vorhin: nicht nur „zwei Läufe finden
teils verschiedene Items", sondern „auch fünf Läufe finden noch neue".

## Was das für den Plan heißt

Die Reihenfolge des Dekrets — erst vervollständigen (Punkt 1), dann messen
(Punkt 3) — bleibt richtig, aber mit einer Einschränkung, die deine Entscheidung
braucht:

- **Die Vervollständigungs-Pakete werden nie „vollständig".** Wenn das Inventar
  aus einer großen latenten Menge sampelt, findet jede neue Messung Norm-Teile,
  die die Pakete nicht abdecken. Die Bedingungslisten jagen einem beweglichen Ziel
  nach.
- **Das spricht dafür, den Zuschnitt zu verengen, nicht die Listen zu verlängern.**
  Je enger die Signatur, desto kleiner die latente Item-Menge, desto eher sättigt
  das Inventar. § 33 Abs. 3 mit drei Eingaben und einer 2D-Tabellen-Norm ist ein
  großer Ausschnitt; die 8 Annahmen und ~9 Norm-Teile je Lauf spiegeln das.
- **Alternativ** akzeptiert man, dass `geltungsbereich` ein Review-Trigger bleibt
  und nie grün wird, und verlässt sich für die Freigabe auf Test-Gate +
  Human-Review. Dann ist die Judge-Instabilität kein Blocker, sondern eine
  Eigenschaft, mit der man lebt.

## Vervollständigungs-Pakete (Punkt 1)

`reports/review/2026-07-11-vervollstaendigung-<regel>.md`, je Regel eines. Jedes
listet die ungedeckten `wirkt_hinein`-Norm-Teile (mit Referenz und Zitat) und die
undeklarierten Annahmen (mit `betrifft`/`kategorie`) als Bedingungs-Kandidaten mit
Zitatanker-Platzhalter.

**Vorbehalt, im Kopf jedes Pakets:** die aktuellen Pakete stammen aus je einem
gespeicherten Verdikt (die meisten noch aus der Zeit vor den Ankern). Für ein
belastbares Paket müssten sie aus einem frischen Union-Lauf je Regel erzeugt
werden — und der obige Befund heißt, dass selbst der die Union nicht abschließt.
Ich habe die Pakete deshalb als Startpunkt erzeugt, nicht als fertige Vorlage.

## Meine Empfehlung

Vor dem Geld für die Punkt-3-Messung: **eine Zuschnitt-Entscheidung.** Wenn das
Inventar nicht sättigt, ist die billigste Stabilisierung, den Ausschnitt zu
verengen (kleinere Signaturen, mehr Regeln), nicht die Bedingungslisten gegen ein
bewegliches Ziel zu verlängern. Das ist eine Richtungsfrage, die ich dir nicht
abnehme. Danach lohnt die Messung; vorher misst sie ein Ziel, das sich noch
bewegt.

Kosten dieser Stufe: Validierungslauf § 33 0,22 USD; sonst nur Tests (kostenlos).
