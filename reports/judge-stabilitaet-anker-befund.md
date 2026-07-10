# Anker-Nachmessung — Befund und Entscheidungsvorlage

Vorregistrierter Messplan durchgeführt (7 Regeln, 3 Läufe, 6,79 USD; Spot-Replikation
teilweise, siehe unten). Rohdaten in `reports/nachtschicht/judge-stabilitaet-anker.json`,
generierter Report in `reports/judge-stabilitaet.md`.

## Die drei vorregistrierten Kriterien — alle verfehlt

| Kriterium | Ziel | Ergebnis |
|---|---|---|
| Inventar-Deckung | ≥ 80 % | **19,7 %** (80 von 406 Items in allen 3 Läufen) |
| `geltungsbereich`-Splitrate | ≤ 15 % | **29,8 %** |
| Spot-Replikation identisch | True | **False** (§ 9 Abs. 4a; § 24b durch Fireworks-429 blockiert) |

Nach dem vorregistrierten Entscheidungsbaum: **`spot_diagnose`** → Entscheid bei
Julius. Der Trigger für den Zweit-Judge feuert *nicht* automatisch, weil er an eine
stabile Inventar-Deckung gebunden war — und die haben wir nicht.

## Was der Anker gelöst hat — und was er offengelegt hat

Der Anker war die richtige Idee, aber er hat das Problem nicht behoben, sondern
**verortet**.

**Gelöst: das Matching.** Der Vergleich läuft jetzt über den Anker, nicht die Prosa.
Zwei Umformulierungen derselben Annahme mergen korrekt (im Merge-Log nachweisbar).
Der § 24b-Fehlalarm der letzten Messung („0 von 13 gemeinsam") ist damit weg —
`norm_teile` matchen jetzt gut (8 von ~11 gemeinsam über die Paragraphen-Referenz).

**Offengelegt: die eigentliche Instabilität liegt im Inventar-*Recall*.** Das Modell
listet je Durchlauf einen anderen *Satz* von Items. Konkret aus § 33, dokumentiert:
die Annahme über `anzahl_kinder` erscheint in Lauf 1 und 2, in Lauf 3 fehlt sie
ganz. Kein Cluster-Abgleich kann ein Item herbeiführen, das ein Durchlauf nicht
produziert hat.

Der Recall spaltet sich nach Item-Art:

- **`norm_teile`** sind vergleichsweise stabil — die Paragraphen-Referenz ist ein
  diskreter, kleiner Raum, und das Modell trifft ihn meist gleich.
- **`annahmen`** sind sehr instabil — in der § 9 Abs. 4a-Replikation waren von je
  ~11 Annahmen nur **1** in beiden Läufen. Das Modell erfindet viele feingranulare
  Annahmen, und der Raum (`betrifft` × `kategorie`) ist fein genug, dass zwei Läufe
  kaum dieselben treffen.

## Warum `geltungsbereich` kippt

Die 29,8 % sind fast vollständig eine Folge von zwei Dingen, die zusammenwirken:

1. Der Norm-Teil-Recall streut (ein `wirkt_hinein`-Teil taucht in einem Lauf auf,
   im nächsten nicht).
2. Die `geltungsbedingungen`-Listen sind **unvollständig** (das Re-Gate hat das
   gezeigt: viele ungemappte Annahmen, z. B. 7 bei § 10 Nr. 7).

Ein neu aufgetauchter `wirkt_hinein`-Teil, für den keine Bedingung deklariert ist,
lässt `geltungsbereich` fallen. Wäre die Bedingungsliste vollständig, wäre jeder
aufgetauchte Teil abgedeckt — dann würde das Gate nicht mehr davon abhängen, *welche*
Teile ein Lauf gerade findet.

## Anmerkung zum Deckungs-Kriterium

Ich ändere die vorregistrierte Zahl nicht, aber ich muss einen Konstruktionsfehler
benennen: **die Deckung (Anteil Items in *allen* Läufen) bestraft die
Union-Strategie strukturell.** Je besser der Recall über mehr Läufe, desto mehr Items
insgesamt, desto *kleiner* der Anteil, der in allen Läufen steht. Für eine
Vereinigung ist „in allen Läufen gesehen" das falsche Stabilitätsmaß. Das entwertet
das 80-%-Ziel nicht als Signal — 19,7 % ist eindeutig schlecht —, aber es heißt: die
Deckung allein sagt nicht, ob die *Gate-Ausgabe* stabil ist. Dafür ist die
Splitrate-je-Gate das bessere Maß, und die sagt: `roundtrip` 10,1 % (grenzwertig),
`geltungsbereich` 29,8 % (schlecht).

## Entscheidungsvorlage

Der Zweit-Judge ist nach dem Dekret jetzt **nicht** dran: der Trigger verlangt eine
stabile Inventar-Deckung, sonst erbt der zweite Judge dasselbe Recall-Rauschen. Drei
Wege, meine Empfehlung zuerst:

1. **Bedingungslisten zuerst vervollständigen, dann neu messen.** Das Dekret hatte
   das aufgeschoben („nicht gegen ein instabiles Inventar spezifizieren"), aber die
   Diagnose dreht das Argument um: die `geltungsbereich`-Instabilität *ist* zu einem
   großen Teil die fehlende Abdeckung. Das Re-Gate hat die ungemappten Annahmen schon
   gesammelt; daraus wird pro Regel ein Vervollständigungs-Paket mit Ankern, du
   segnest ab, dann eine Nachmessung nur auf `geltungsbereich`. **Billigster Angriff
   auf die tatsächliche Ursache.** Widerspricht deiner Reihenfolge-Auflage 3 — deshalb
   deine Entscheidung, nicht meine.

2. **Inventar-Recall stabilisieren, bevor irgendetwas anderes.** Zwei Unter-Optionen:
   (a) mehr Inventarläufe (5–7) mit Union — die Vereinigung konvergiert gegen den
   wahren Satz, Kosten steigen linear; (b) ein zweistufiges Inventar: Lauf 2 bekommt
   die Liste aus Lauf 1 und wird gefragt „was fehlt?" (Vollständigkeits-Kritiker).
   Beides greift den Recall direkt an, ändert aber `inventar@2`.

3. **Die Stabilitätsfrage aufs Gate heben statt aufs Item.** Nicht „liefern zwei
   Läufe dasselbe Item-Set", sondern „liefern drei *volle* Verdikte dieselbe
   *Gate-Ausgabe*", Mehrheit zählt. Das ist teuer (Judge × 3 je Regel), misst aber
   direkt, was zählt, und macht die Item-Set-Streuung irrelevant, solange die
   Gate-Ausgabe mehrheitlich stabil ist.

Meine Empfehlung: **1 vor 2 vor 3.** Punkt 1 ist billig und trifft die belegte
Ursache von `geltungsbereich`; erst wenn das Gate danach immer noch streut, ist der
Recall (Punkt 2) das nächste Ziel, und der Zweit-Judge erst, wenn Recall und Abdeckung
stabil sind und die Streuung *trotzdem* bleibt.

## Kosten

Diese Nachmessung 6,79 USD, Replikation § 9 Abs. 4a 1,11 USD, § 24b-Replikation
durch Fireworks-429 abgebrochen (Provider-Flakiness, nicht Code — der Client hat
sauber nach zwei Retries abgebrochen). Die Produktions-Reports der Charge stehen bei
2,94 USD; die Judge-Stabilitätsmessungen (mehrere Durchgänge) sind der teurere
Posten und liegen zusammen im zweistelligen Dollarbereich. Ein einzelnes volles
Verdikt kostet je nach Item-Zahl 0,08–0,49 USD.
