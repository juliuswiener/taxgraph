# S0.3 Default-Logic-Ergonomietest

Frage: Bildet Catalas Grundregel/Ausnahme-Mechanik (`label` / `exception` /
`under condition ... consequence`) die Struktur von § 4 Abs. 5 Nr. 6b und 6c
EStG inklusive des gegenseitigen Ausschlusses natuerlich ab, oder braucht es
Workarounds?

Formalisierung: `rules/estg/p04_arbeitszimmer_homeoffice/`. Tests: 8 Faelle,
alle gruen (`make s03`).

## Fazit

Die Abbildung gelingt natuerlich. Die gesetzliche Grundregel/Ausnahme-Struktur
laesst sich fast eins zu eins in Catalas Default-Logik uebersetzen. Der 2025 an
der Graph-Modellierung gescheiterte Fall (gegenseitiger Ausschluss zwischen
Arbeitszimmer und Homeoffice) ist hier ohne Verrenkung darstellbar.

## Was gut ging

**Grundregel plus Ausnahmen (Nr. 6b).** Satz 1 (nicht abziehbar) ist die
Basisregel, Satz 2 (Mittelpunkt) und Satz 3/4 (Jahrespauschale) sind Ausnahmen
mit Bedingung. Das liest sich direkt wie der Gesetzestext:

```catala
# Grundregel Satz 1: nicht abziehbar.
label grundregel_6b
definition abzug equals $0.00

# Ausnahme Satz 2: Arbeitszimmer ist Mittelpunkt -> tatsaechliche Aufwendungen.
exception grundregel_6b
definition abzug under condition
  arbeitszimmer_vorhanden and ist_mittelpunkt and not jahrespauschale_gewaehlt
consequence equals tatsaechliche_aufwendungen

# Ausnahme Satz 3 und 4: Jahrespauschale, je Monat ohne Mittelpunkt um 1/12 gekuerzt.
exception grundregel_6b
definition abzug under condition
  arbeitszimmer_vorhanden and jahrespauschale_gewaehlt
consequence equals
  $1,260.00 - ($1,260.00 * (decimal of monate_ohne_mittelpunkt / 12.0))
```

Mehrere Ausnahmen unter demselben Label koexistieren, solange ihre Bedingungen
sich nicht ueberschneiden. Bei Ueberschneidung meldet Catala einen Konflikt zur
Laufzeit, statt still einen Zweig zu bevorzugen. Das entspricht dem
Verifikationsprinzip (Divergenzen eskalieren, nicht mitteln).

**Gegenseitiger Ausschluss (Nr. 6c Satz 3).** Der Ausschluss ist selbst eine
Ausnahme: die Tagespauschale entfaellt, sobald ein Abzug nach Nr. 6b vorgenommen
wurde. Modelliert als Ausnahme, deren Bedingung vom Ergebnis der Nr. 6b abhaengt:

```catala
# Grundregel: 6 Euro je Tag, gedeckelt auf 1 260 Euro.
label grundregel_6c
definition tagespauschale equals
  if tagespauschale_ungedeckelt > $1,260.00 then $1,260.00
  else tagespauschale_ungedeckelt

# Ausnahme Satz 3: Ausschluss bei Abzug nach Nr. 6b.
exception grundregel_6c
definition tagespauschale under condition abzug_6b_vorgenommen
consequence equals $0.00
```

Die Verdrahtung der beiden Regeln (das Ergebnis der Nr. 6b speist die Bedingung
der Nr. 6c) geschieht ueber einen Scope-Aufruf im zusammenfuehrenden Scope
`Raumkostenabzug`. Damit bleibt die Abhaengigkeitsrichtung explizit und die
einzelnen Normen bleiben je fuer sich testbar.

## Wo es hakte (kleinere Reibung, keine Blocker)

1. **Cross-Modul-Scope-Aufrufe brauchen `--whole-program`.** Ruft ein Testmodul
   per `output of Modul.Scope with { ... }` einen Scope aus einem anderen Modul
   auf, scheitert `catala interpret` ohne die Option `--whole-program` (bzw.
   `clerk test -W`) mit "Could not resolve reference". Mit der Option laeuft es.
   Deshalb nutzt das Makefile durchgaengig `clerk test -W`.

2. **Zwischenergebnisse muessen deklariert werden.** Ein `definition r equals
   output of ...` erfordert, dass `r` vorher als `internal r content <Scope>` im
   Scope-Kopf deklariert ist. Ad-hoc-Lokale gibt es nur via `let ... in` innerhalb
   eines Ausdrucks. Das ist konsistent, aber beim ersten Mal ueberraschend.

3. **Literate-Zitate duerfen nicht mit `>` beginnen.** Ein `>` am Zeilenanfang ist
   in Catala eine Modul-Direktive (`> Module`, `> Using`), kein Markdown-Zitat.
   Gesetzeszitate werden daher eingerueckt statt als Blockquote gesetzt.

Keiner dieser Punkte erzwingt eine semantische Abweichung vom Gesetzestext. Es
sind Werkzeug- und Syntaxdetails, die einmalig zu lernen sind.

## Bewertung fuer Gate G0

Kriterium 3 (Default Logic bildet S0.3 ohne Verrenkungen ab) ist erfuellt. Die
Grundregel/Ausnahme-Mechanik ist fuer diese Klasse von Normen das passende
Werkzeug und war beim gegenseitigen Ausschluss deutlich ergonomischer als der
Graph-Ansatz aus 2025.
