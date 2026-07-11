# B1 — Ergebnis: Bedingungen/Hinweise → Formalisierer (Rohdaten + Auswertung)

Vorregistrierter Plan: [[2026-07-12-b1-rolespy-messplan]]. Experiment in Experiment-Kopie (in-memory
narrow-Zuschnitt), **Produktions-rules.yaml/-Prompts unberührt**. `skip_judge=True` (Judge für B1
nicht gebraucht). models.yaml-Hash gestempelt, A/Judge unverändert, B=glm-5.2.

## Ergebnis in einem Satz

**Arm B (kuratiertes `hinweis`-Feld + Numeric-Idiom) löst 5/5 Kontext-Hunger-Fälle mit NARROW
auszug (A-Formalisierer clerk, 2/2 Läufe je Zelle), Kontrolle nicht regrediert. Arm A (bloße
Bedingungs-beschreibungen) nur 2/5.** Vorregistrierte Entscheidungsregel (≥4/5 + Kontrolle grün →
wirksam) → **Arm B wirksam, Arm A nicht.** Empfehlung: siehe unten (Julius entscheidet Prompt-Change).

## Primärmetrik — Hunger-Fix-Rate (A-Formalisierer clerk auf Wächter-Seeds, NARROW auszug)

Zelle grün = ALLE Wächter-Seeds `ok` im Lauf. Eligibilität: 2/2 = Fix, 1/2 = instabil, 0/2 = Fail.

| Fall (Klasse) | Wächter-Seeds | Arm A (r1,r2) | Arm B (r1,r2) |
|---|---|---|---|
| nr5a (1, 48-Grenze) | 47→16800 / 48→12000 | 0/2 **FAIL** | 2/2 **FIX** |
| nr6_7_afa_laufend (1) | Jahr0→200 / Jahr1→… | 2/2 FIX | 2/2 FIX |
| p10_1_3_3a (1, Basis-Durchbruch) | 4000→4000 | 0/2 **FAIL** | 2/2 **FIX** |
| p33_1_2 (1) | 5000/1408,7→3591,3 | 2/2 FIX | 2/2 FIX |
| solzg (1, Milderungszone) | 25000→553,35 / 45000→2475 | 0/2 **FAIL** | 2/2 **FIX** |
| **Hunger-Fix-Rate** | | **2/5** | **5/5** |
| KONTROLLE p10_1_4 | 1200/200→1000 | 2/2 ✓ | 2/2 ✓ (kein Regress) |

Arm B fixt genau die Fälle, an denen die Nacht mit auszug-Weitung + Prominenz von Hand arbeiten
musste (nr5a-Grenze, p10v2-Durchbruch, solzg-Milderung) — **mit engem auszug**, allein durch den
kuratierten Hinweis. Das ist das strukturelle Signal, das der Plan gesucht hat.

## Entscheidungsregel (vorregistriert, wörtlich angewandt)

> Ein Arm ≥ 4/5 Hunger-Fälle mit narrow auszug gelöst UND Kontrolle grün → strukturell wirksam.
> Kein Arm ≥ 3/5 → Hinweise ersetzen die auszug-Leitlinie nicht.

- **Arm B: 5/5 ≥ 4/5, Kontrolle grün → WIRKSAM.**
- **Arm A: 2/5 < 3/5 → nicht wirksam** (bloße `beschreibung` reicht nicht; der Hinweis muss
  kuratiert-operativ sein, nicht nur die Bedeutung nennen).
- Ein einziger wirksamer Arm → kein Gesamtscore-Tiebreak nötig. Sieger eindeutig B.

## Ehrliche Gaps / Residuen (nicht überverkaufen)

1. **Klasse-5-Präzision GEMESSEN — Numeric-Idiom fixt sie NICHT (negativ).** Korrektur einer
   früheren Fassung dieses Reports, die den Präzisions-Seed fälschlich „ungemessen" nannte: der
   Runner speicherte ihn separat in den `A_prec`/`B_prec`-Feldern (Seed `20351→0,11`), ich hatte
   die Felder zunächst übersehen. Ergebnis solzg Arm B, beide Läufe, beide Formalisierer:
   **`got $0,12` statt `0,11` → 2/2 FAIL.** Das Idiom („rechne in decimal, Cent-Schnitt erst am
   Ende, floor nicht kaufmännisch") änderte die Ausgabe NICHT — das Modell rundet 0,119 weiter
   Cent-mittig auf 0,12. **Klasse-5 ist ein struktureller Rest, den ein Prompt-Hinweis nicht
   schließt; er braucht Code (decimal-Refactor / Präzisions-Lint), nicht Kurierung.** Der
   solzg-**Milderungs**-Fix (25000/45000, Klasse-1) durch Arm B steht davon unberührt und zählt.
2. **B-Formalisierer (glm) trägt den Kanal nur teilweise** (Sekundärmetrik, Gewicht 0,10). glm-B
   clerk grün, Arm B: nr5a 2/2, p33 2/2, solzg 2/2, nr6_7 1/2, **p10v2 0/2**. Der Hinweis hilft dem
   A-Formalisierer robust; glm zieht bei p10v2 nicht mit (0/2) und ist bei nr6_7 instabil. Zwei
   B-Zellen kamen als `None`/`dict` zurück (glm-Parse-/Ausgabefehler, nicht als Seed-Liste) — als
   Fail gewertet. Kanal wirkt, aber modell-abhängig.
3. **Kleines N.** 2 Wächter-Seeds je Fall, 2 Läufe. Richtungsstark, nicht erschöpfend. Kein
   Anspruch, dass der Hinweis JEDEN künftigen Hunger-Fall trägt — nur diese 5 Regressions-Fälle.
4. **Kurierungs-Kosten verschoben, nicht eliminiert.** Der `hinweis` muss je Regel von Hand
   geschrieben werden (wie heute der auszug-Zuschnitt). Der Gewinn ist Robustheit/Prominenz bei
   engem auszug, nicht Wegfall der Handarbeit. Ob netto billiger als auszug-Weitung: offen, nicht
   gemessen.

## Kosten

Gesamt **$0,55** über 24 Zellen (2 Arme × 6 Fälle × 2 Läufe). Rahmen war ≤3. Nacht-Rest reichlich.

## Empfehlung (an Instructor → Julius entscheidet)

Arm B ist strukturell wirksam (5/5, Kontrolle grün). **Empfehlung: `hinweis`-Feld als offiziellen
Prompt-Kanal adoptieren** — optionales Feld je Regel, kuratierter operativer Hinweis (kein
Gesetzestext-Ersatz), geht als eigener Block VOR die Formalisier-Anweisung (`zusatz`-Mechanik,
bereits produktions-sicher mit Default leer in `roles.py:formalize`/`cascade.py`).

**Vorbehalte, die in die Julius-Vorlage gehören:**
- Prompt-Change = Julius-Freigabe (Produktion). Der `zusatz`-Parameter ist bereits verdrahtet und
  bei leerem Default prompt-neutral; „adoptieren" heißt: `hinweis`-Feld in rules.yaml + Befüllung
  je Regel + Prompt-Template-Bump/Hash-Stempel — kein neuer Code-Pfad.
- Klasse-5-Numeric-Idiom **widerlegt** (solzg `20351→0,11`, Arm B 2/2 FAIL `$0,12`): der Hinweis-
  Kanal löst Klasse-1-Hunger, aber KEINE Präzisions-Ordnung. Klasse-5 bleibt Code-Aufgabe.
- glm-B-Schwäche bei p10v2 dokumentieren; der Kanal ist auf A verlässlich, auf B nicht garantiert.
- Auszug-Leitlinie NICHT abschaffen — Hinweis ist Ergänzung, nicht Ersatz; die grüne Produktion
  läuft weiter auf geweiteten auszügen.

## Rohdaten

`scratchpad/b1_results.json` (24 Zellen, per-Seed inp/exp/got/ok für A und B), `b1_results.log`.
Zusammenfassung oben deterministisch daraus reproduzierbar (Zelle grün ⟺ alle Seeds ok).
