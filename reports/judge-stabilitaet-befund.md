# Judge-Stabilität nach der Dekomposition — Befund und Einschränkung

Vorregistrierter Messplan (Protokolldekret 2026-07-10). Ein Durchgang, 7 Regeln,
je 3 Läufe. Kosten der Messung 4,9373 USD, der Spot-Replikation 1,4327 USD.

## Ergebnis nach dem vorregistrierten Kriterium

**Item-Splitrate 19,3 %** (59 Splits auf 305 beurteilte Items) → Band
„10–20 %" → **Spot-Replikation**, wie im Dekret vorgesehen.

Getrennt nach blockierendem Gate:

| Gate | Splits | Items | Rate |
|---|---|---|---|
| `roundtrip` | 11 | 155 | **7,1 %** |
| `geltungsbereich` | 48 | 150 | **32,0 %** |

Der Mehrheitsentscheid je Item wirkt also dort, wo er gedacht war: die Zuordnung
einer Annahme auf eine Bedingungs-ID ist mit 7,1 % Splits weitgehend stabil. Die
Klassifikation eines Norm-Teils (`wirkt_hinein` vs. `unabhaengig`, und welche
Bedingung ihn abdeckt) ist es mit 32 % nicht.

Null Parse-Fehler in 21 Läufen. Das war vorher 11 % — die kurzen, schemagebundenen
Antworten haben das Budget-Problem strukturell beseitigt, wie erwartet.

## Was das Kriterium nicht misst, und warum es hier entscheidet

Die Splitrate misst die Uneinigkeit **über ein Item**. Sie sagt nichts darüber, ob
zwei Läufe **dieselben Items finden**. Genau dort sitzt die verbliebene Streuung:

- 249 von 305 Items wurden **nicht in allen drei** Inventarläufen gesehen.
- Die Zahl der beurteilten Items schwankt je Lauf erheblich: § 10 Abs. 1 Nr. 7
  zwischen 5 und 14, § 9 Abs. 6 zwischen 8 und 14, § 35a zwischen 24 und 31.
- Fünf von sieben Regeln haben weiterhin **wechselnde Gate-Urteile**, obwohl die
  Item-Urteile mehrheitlich stabil sind. Ein zusätzlich gefundener
  `wirkt_hinein`-Norm-Teil kippt `geltungsbereich`.

Die Spot-Replikation bestätigt das. Zweimal derselbe Input, identisches
Gesamtverdikt?

| Regel | identisch? | Annahmen gemeinsam | Norm-Teile gemeinsam |
|---|---|---|---|
| § 9 Abs. 4a (vorher instabilste) | **nein** | 2 von 15 | 8 von 16 |
| § 24b (Kontrolle, vorher stabil) | **nein** | 0 von 13 | 4 von 9 |

## Einschränkung: ein Teil davon ist meine Messung, nicht der Judge

Bei § 24b habe ich die angeblich disjunkten Annahmen angesehen. Sie sind
**semantisch dieselben**:

```
Lauf A: "Die Eingabe 'alleinstehend' wird als zutreffend gemäß der
         Legaldefinition in § 24b Abs. 3 EStG vorausgesetzt"
Lauf B: "Die Eingabe 'alleinstehend' setzt voraus, dass die Definition des
         Alleinstehenden nach Abs. 3 beachtet wird"
```

Mein Ähnlichkeitsabgleich bewertet dieses Paar mit einer Überdeckung von 0,20 und
trennt es. Nach dem Entfernen der Floskeln bleiben zu wenige tragende Wörter übrig
(`legaldefinition` gegen `definition`, `zutreffend` gegen `beachtet`), und bei
kleinen Wortmengen ist die Überdeckung sprunghaft.

**Die Zahl „0 von 13 gemeinsam" misst also zu einem unbekannten Teil die Schwäche
meines Vergleichers, nicht die des Judge.** Ich lasse sie so stehen, statt sie
nachträglich zu schönen, und benenne die Konsequenz: die absolute Höhe der
Inventar-Streuung ist eine **Obergrenze**, kein Punktwert.

Was davon unberührt bleibt: die Gate-Urteile kippen tatsächlich zwischen Läufen
(das wird ohne jeden Textvergleich beobachtet), und die Zahl der gefundenen Items
schwankt um den Faktor zwei bis drei.

## Diagnose

Die Dekomposition hat gelöst, was sie lösen sollte:

- Parse-Fehler von 11 % auf 0 %,
- Truncation strukturell weg (Budget wächst mit Items, nicht mit Prosa),
- Mapping einer Annahme auf eine Bedingung stabil (7,1 % Splits).

Ungelöst ist die **Inventarstufe**. Sie ist immer noch ein Freitext-Schritt: das
Modell formuliert jedes Item neu, und weder der Judge noch mein Code können zwei
Formulierungen zuverlässig als dasselbe Item erkennen. Damit ist jede Statistik
über „dieselben Items" wackelig, und `geltungsbereich` erbt die Streuung.

## Vorschlag: Items an einen stabilen Schlüssel binden

Nicht der Text soll das Item identifizieren, sondern ein **Anker**, den das Modell
nicht frei erfinden kann:

```json
{"annahmen":   [{"betrifft": "<Name einer Signatur-Eingabe>", "aussage": "..."}],
 "norm_teile": [{"referenz": "<§ und Absatz/Satz>",           "zitat":   "..."}],
 "abweichungen":[{"betrifft": "<Eingabe oder 'ergebnis'>",    "aussage": "..."}]}
```

Der Abgleich läuft dann über `betrifft` bzw. `referenz`, nicht über Prosa. Beide
Werte sind aus dem Kontext gebunden: die Eingabenamen stehen in der Signatur, die
Paragraphen im Normtext. Zwei Läufe, die dieselbe Annahme über `alleinstehend`
machen, tragen denselben Schlüssel.

Das ist eine Änderung an `inventar@1` und am Abgleich, keine am Protokoll: der
Mehrheitsentscheid, die konservative Auflösung und die Split-Eskalation bleiben.

**Erwartung, prüfbar:** die Inventar-Streuung fällt deutlich; die Gate-Urteile
werden stabil; die Item-Splitrate bleibt, wo sie ist (sie misst etwas anderes).

## Entscheidungsvorlage

1. **Anker-Schlüssel bauen und einmal nachmessen** (dieselben 7 Regeln, 3 Läufe,
   rund 5 USD). Meine Empfehlung. Erst danach ist die Frage „Zweit-Judge?"
   überhaupt beantwortbar, weil bisher die Messung selbst mitrauscht.
2. **Trigger nach Dekret Punkt 3 sofort ziehen** (Splitrate 32 % auf
   `geltungsbereich` liegt über 20 %) und den Zweit-Judge einer anderen Familie
   einführen. Dagegen spricht: zwei Protokolländerungen gleichzeitig, deren
   Wirkung sich nicht trennen lässt — genau das, was du vermeiden wolltest.
3. Nichts tun und `geltungsbereich` als unzuverlässig führen. Halte ich nicht für
   vertretbar.

Die Zahlen liegen in `reports/judge-stabilitaet.md`,
`reports/nachtschicht/judge-stabilitaet-dekomponiert.json` und
`reports/nachtschicht/judge-replikation.json`.
