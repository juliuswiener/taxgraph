# Präzisions-Lint (Klasse 5) — Vorregistrierung (VOR dem Bau)

Instructor-Auftrag 2026-07-12. **Neues Produktions-Gate → braucht Instructor-Freigabe + (für Stufe 2)
Julius.** Dieser Plan ist vor dem Bau fixiert. Der Prototyp (`scratchpad/praezisions_lint_proto.py`)
diente NUR der Design-Validierung; `gates.py` ist unberührt bis zur Freigabe.

## Problem (Klasse 5, aus solzg)

Catala-`money` ist cent-präzise: `money × decimal` rundet das Produkt SOFORT auf Cent. Rechnet eine
Regel einen Prozentsatz auf eine money-Größe und schneidet ERST danach final auf Cent, entsteht
Doppelrundung. solzg `unterschiedsbetrag * 0.119` bei BMG 20351: 0,119 € → als money $0,12; der
folgende `truncate`-Cent-Schnitt sieht schon 0,12 → Ergebnis 0,12 statt 0,11. Fix: den Prozentanteil
in `decimal` rechnen (money per `/ $1.00` oder `decimal of` konvertieren), Cent-Schnitt am Ende.

## 1. Erkennungsregel (syntaktisch, typ-bewusst) + Legitim-Abgrenzung

FLAG (Klasse-5-Verdacht) genau dann, wenn **(A) UND (B)**:

- **(A) finaler Cent-Schnitt vorhanden**: entweder das Catala-Idiom
  `(truncate|floor|round|ceil) of ( <expr> / $0.01 ) ) * $0.01`, ODER eine `rundung`-Deklaration mit
  `richtung: floor` deren `deckt_ab`/`zitatanker` „Cent"/„Bruchteil" nennt.
- **(B) money × decimal-Literal, das money produziert und NICHT der Cent-Schnitt selbst ist**: ein
  money-typisierter Operand (money-Input/-Output oder ein `let`, das money erbt) multipliziert/geteilt
  mit einem decimal-Literal (`\d+\.\d+`, NICHT `$`-präfixiert), außerhalb der Cent-Schnitt-Zeile.

**Typ-Tracking ist PFLICHT (nicht optional).** Ein `let Y equals RHS` erbt money nur, wenn RHS money
trägt UND NICHT durch ein money-Literal/-Name teilt (`money / money → decimal`) und kein `decimal of`
trägt. Ohne dieses Tracking flaggt der Lint die FIX-Form falsch (s. §5, Fund v1).

**Legitim (kein FLAG), bewusst so:**
- `money × decimal` OHNE deklarierten/vorhandenen finalen Cent-Schnitt → die Cent-Rundung IST das
  gewollte Ergebnis (die Norm rechnet cent-genau). Kein (A) → kein Flag.
- Prozentrechnung, die bereits in `decimal` geführt wird (money per `/ $1.00`/`decimal of` konvertiert)
  und erst am Ende schneidet → das ist die Fix-Form, kein money×decimal-Produkt (B). Kein Flag.
- money ± money, money-Vergleiche, Cent-Schnitt-Zeile selbst (`/ $0.01`) → kein (B).

## 2. FP-Analyse gegen den Bestand (ausgeführt, $0)

Korpus: `pipeline/runs/produktion/*/report.json`, **21 Regeln mit `catala_a`** (die runs/-Reports sind
lokal vorhanden — der frühere „runs/ absent"-Blocker ist überholt). Prototyp v2 über alle 21:

- **FLAGGED: nur `solzg_solidaritaetszuschlag`.** 20/21 sauber. **0 False Positives, 0 „cut-only"**
  (keine Regel hat einen finalen Cent-Schnitt ohne money×decimal). Das erfüllt das Instructor-
  Kriterium („nur solzg flaggen") exakt.
- solzg-Treffer: Zeile 13 `bemessungsgrundlage * 0.055` und Zeile 15 `unterschiedsbetrag * 0.119`
  (letzteres ist die Bug-Quelle) — beide vor dem Cent-Schnitt. Kein latenter Zusatzfund im Bestand.

## 3. Rollout — zweistufig

- **Stufe 1 (informativ, sofort nach Freigabe):** Gate läuft über den Bestand + jeden neuen Lauf,
  meldet, **kippt KEIN Gate** (wie das discovery-Gate). Befunde in den Report (`praezisions_lint:
  MELDUNG`), Instructor-Review der Flaggen. Ziel: Empirie sammeln, ob außer solzg je etwas flaggt und
  ob es echte Funde oder FPs sind.
- **Stufe 2 (blockierend, erst nach Julius-Freigabe auf Basis der Stufe-1-Empirie):** FAIL kippt das
  Gate und geht als Repair-Signal in die Runde. **Gate-Voraussetzung:** die Negativtests unten stehen
  grün UND Stufe 1 zeigte keine unerklärten FPs. Ein Gate, das nie kontrolliert FAIL/PASS gezeigt hat,
  ist unbewiesen → Negativtests pflicht.

**Negativtest-Suite (empirisch bereits am Prototyp bestätigt):**

| Fall | Erwartung | Prototyp v2 |
|---|---|---|
| buggy solzg (money×dec vor Cent-Schnitt) | FAIL | FAIL (B=2) ✓ |
| Fix-Form (decimal bis Ende, Schnitt am Ende) | PASS | PASS ✓ |
| money×decimal OHNE finalen Cent-Schnitt | PASS (legitim) | PASS ✓ |
| Bestand-Regression (20 andere Regeln) | PASS | PASS (solzg-only) ✓ |

## 4. Interaktion mit rundungs_lint (Klasse 4 vs. 5 — getrennt, keine Doppel-Flagge)

- **rundungs_lint (Klasse 4)** prüft: ist die Rundungs-OP deklariert, und stimmt ihre RICHTUNG
  (floor/ceil/kaufmännisch)? Auf solzg: **PASS** — der finale `truncate` ist als `richtung: floor`
  deklariert und floort korrekt.
- **praezisions_lint (Klasse 5)** prüft: ORDNUNG — rundet ein money×decimal auf Cent VOR dem
  (korrekt gerichteten) finalen Schnitt? Auf solzg: **FAIL**.
- Beide melden getrennt und über verschiedene Zeilen (Klasse 4 = die Cent-Schnitt-OP; Klasse 5 = die
  money×decimal-Produktzeilen). Keine Doppel-Flagge derselben Ursache: der finale `truncate` ist für
  Klasse 4 korrekt, für Klasse 5 ist er nur der Kontext, der die vorgelagerte money-Rundung fatal macht.

## 5. Fund der Vorregistrierung: naive Regel flaggt die Fix-Form falsch (v1 → v2)

Prototyp v1 (money-Name × decimal-Literal + Cut, OHNE Typ-Tracking) flaggte die FIX-Form fälschlich als
FAIL: er hielt `unterschied_dec` (aus `(bmg − freigrenze) / $1.00`, also money/money → decimal) für
money und meldete das nachfolgende `* 0.119` als money×decimal. **Ein blockierendes Gate auf v1-Basis
würde also genau die korrekte Fix-Form ablehnen, die es erzwingen soll — fataler Blocker.** v2 mit
`money/money → decimal` + `decimal of`-Tracking behebt das (Fix-Form → PASS, buggy → FAIL, Bestand
solzg-only). **Konsequenz: das Gate darf erst blockierend werden, wenn das Typ-Tracking robust ist;
Stufe 1 informativ ist der Beweis-Sammler.** Genau dafür der Prototyp vor dem Gate.

## 6. Repair-Signal-Format (Gate-Output bei FAIL, KEIN Prompt-Change)

Wörtliche Meldung in die Repair-Runde (wie ein Compiler-Fehler), nennt das decimal-Idiom:

> Klasse-5 Präzisions-Ordnung: Zeile 15 `unterschiedsbetrag * 0.119` rechnet einen Prozentsatz auf
> eine money-Größe (Ergebnis wird sofort auf Cent gerundet) VOR dem finalen Cent-Schnitt (Zeile 20).
> Rechne den Prozentanteil in `decimal`: konvertiere die money-Größe per `/ $1.00` (oder `decimal of`)
> nach decimal, multipliziere mit dem Satz, und schneide ERST am Ende mit
> `(Decimal.truncate of (X / 0.01)) * $0.01` auf Cent. Runde keinen Zwischenwert auf Cent.

Zeilennummern + Fragmente aus (A)/(B) werden eingesetzt. Das ist Gate-Output; der Formalisierer-Prompt
ändert sich nicht.

## Budget / Aufwand

Gate deterministisch, **$0** (kein Modell, kein Netz), läuft wie rundungs_lint vor der Äquivalenz.
Bau nach Freigabe: ~1 Funktion in `gates.py` (`praezisions_lint_gate`) + Verdrahtung in cascade
(Stufe 1: nur append, kein FAIL-Pfad) + Negativtests in `tests/test_gate_semantik.py`.

## Fragen an Instructor

(a) Erkennungsregel v2 (money/money→decimal + decimal-of-Tracking) als Basis ok, oder willst du die
Typ-Erkennung noch strenger (z. B. echten Mini-Parser statt Zeilen-Heuristik)? (b) Stufe-1-Umfang: nur
Bestand + neue Läufe, oder auch ein Sweep-Report über alle 21 jetzt? (c) Repair-Meldung so (nennt das
`/ $1.00`-Idiom konkret), oder abstrakter halten? (d) Baue ich Stufe 1 auf deine Freigabe, oder willst
du erst die Erkennungsregel gegen einen von dir konstruierten Zusatzfall sehen?
