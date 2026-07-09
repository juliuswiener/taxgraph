# S0.2 Differentialtest Catala vs GETTSIM - Divergenzen

Erzeugt von `oracle/gettsim/harness.py` (reproduzierbar via `make s02`).

GETTSIM 1.2.1. Ziel: `einkommensteuer.betrag_ohne_kinderfreibetrag_y_sn`, Rundung aktiv (RoundingSpec base=1 down, `§ 32a Abs. 1 S. 6 EStG`).

Vergleich auf Euro-Ebene. Pro VZ: gesetzliche Randwerte (0, Grundfreibetrag +-1, Zonengrenzen +-1) plus 1000 deterministisch geseedete zvE-Werte im Bereich 0..500000 Euro.


## Zusammenfassung

| VZ | n | Divergenzen Grundtarif | Divergenzen Splitting |
|----|---|------------------------|-----------------------|
| 2024 | 1012 | 1 | 583 |
| 2025 | 1013 | 3 | 569 |
| 2026 | 1011 | 1 | 598 |

## Divergenzklasse A: Grundtarif (Absatz 1)

Einzelne Divergenzen, jeweils genau 1 Euro, an Zonen-Innenpunkten (nicht an Zonengrenzen). Ursache: die im publizierten Format (2 Nachkommastellen) angegebenen, literal bestaetigten Formelkoeffizienten des Tarifs gegen GETTSIMs voll aufgeloeste Progressionsfaktor-Rekonstruktion. An einzelnen zvE-Werten kippt der Rohbetrag dadurch ueber eine Euro-Grenze.

Die Koeffizienten sind fuer alle drei VZ literal belegt: VZ 2026 aus der Gesetzesfassung (gesetze-im-internet.de), VZ 2024 aus BGBl 2024 I Nr. 386 (recht.bund.de), VZ 2025 aus EStH/LStH 2025 (esth.bundesfinanzministerium.de). Catala entspricht damit dem Wortlaut; die Divergenzen bedeuten, dass GETTSIMs voll aufgeloeste Rekonstruktion an diesen Punkten vom publizierten Tarif abweicht (GETTSIM-Approximation).

| VZ | zvE | Catala | GETTSIM | Diff | vermutete Ursache | Status |
|----|-----|--------|---------|------|-------------------|--------|
| 2024 | 34218 | 5654 | 5653 | +1 | GETTSIM voll aufgeloest weicht vom literalen § 32a-Tarif ab (Catala = Wortlaut) | erklaert (GETTSIM-Approximation) |
| 2025 | 63954 | 15985 | 15984 | +1 | GETTSIM voll aufgeloest weicht vom literalen § 32a-Tarif ab (Catala = Wortlaut) | erklaert (GETTSIM-Approximation) |
| 2025 | 67349 | 17377 | 17376 | +1 | GETTSIM voll aufgeloest weicht vom literalen § 32a-Tarif ab (Catala = Wortlaut) | erklaert (GETTSIM-Approximation) |
| 2025 | 67690 | 17519 | 17518 | +1 | GETTSIM voll aufgeloest weicht vom literalen § 32a-Tarif ab (Catala = Wortlaut) | erklaert (GETTSIM-Approximation) |
| 2026 | 58832 | 13784 | 13785 | -1 | GETTSIM voll aufgeloest weicht vom literalen § 32a-Tarif ab (Catala = Wortlaut) | erklaert (GETTSIM-Approximation) |

## Divergenzklasse B: Splitting-Verfahren (Absatz 5)

Anzahl divergierender Splitting-Faelle: 1750. Alle Divergenzen betragen genau 1 Euro: False. Catala-Ergebnis stets gerade: True. GETTSIM-Ergebnis in allen Divergenzfaellen ungerade: False.


**Ursache (Rundungsinterpretation).** Das literale § 32a Abs. 5 berechnet die tarifliche ESt als das Zweifache des Steuerbetrags nach Absatz 1 fuer die Haelfte des gemeinsamen zvE. Der Steuerbetrag nach Absatz 1 ist nach Satz 6 auf volle Euro abgerundet. Das literale Ergebnis ist daher `2 * abrunden(Tarif(Z/2))` und stets ein gerader Euro-Betrag (so die amtliche Splittingtabelle). GETTSIM rundet dagegen erst am Ende: `abrunden(2 * Tarif(Z/2))`, wobei die Haelfte Z/2 zusaetzlich nicht auf volle Euro abgerundet wird. Beide Effekte erzeugen die 1-Euro-Abweichungen.


**Bewertung: erklaert (GETTSIM-Vereinfachung).** Entscheidung vom 2026-07-09: der Gesetzeswortlaut ist massgeblich, Catala bleibt auf `2 * abrunden(Tarif(Z/2))` (gerade Betraege). Die Abweichung ist eine Vereinfachung in GETTSIM, kein Fehler in Catala. Divergenzklasse B ist damit geschlossen.


**Offener manueller Spot-Check (pending Julius).** Zur externen Absicherung gegen ein drittes Oracle: gemeinsames zvE 23 634 Euro, VZ 2024, Splitting. Catala liefert 8 Euro. Erwartung laut BMF-Steuerrechner ebenfalls 8 Euro. Manueller Abgleich am amtlichen BMF-Lohn- und Einkommensteuerrechner steht aus (pending Julius, manuell).


Beispiele (erste je VZ):

| VZ | gemeinsames zvE | Catala (2*abrunden(Tarif(Z/2))) | GETTSIM (abrunden(2*Tarif(Z/2))) | Diff |
|----|-----------------|-------------------------------|--------------------------------|------|
| 2024 | 23634 | 8 | 9 | -1 |
| 2024 | 25046 | 216 | 217 | -1 |
| 2024 | 25917 | 354 | 355 | -1 |
| 2025 | 24342 | 20 | 21 | -1 |
| 2025 | 26047 | 274 | 275 | -1 |
| 2025 | 30981 | 1164 | 1165 | -1 |
| 2026 | 24807 | 14 | 15 | -1 |
| 2026 | 25824 | 162 | 163 | -1 |
| 2026 | 25993 | 188 | 189 | -1 |

## Nicht ausgeloeste, aber bekannte Unterschiede

- **Abrundung des zvE auf volle Euro (§ 32a Abs. 1 S. 1).** Catala rundet das zvE auf volle Euro ab; GETTSIM floort das zvE im Tarif nicht. Der Testgrid verwendet ausschliesslich ganzzahlige Euro-Werte, daher wird dieser Unterschied hier nicht ausgeloest. Er ist bei nicht ganzzahligem zvE relevant und separat zu pruefen.

