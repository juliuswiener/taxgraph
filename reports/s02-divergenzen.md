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

Anzahl divergierender Splitting-Faelle: 1750. Die Abweichungen betragen 1 oder 2 Euro. Verteilung von (Catala - GETTSIM) je VZ:

| VZ | Diff -2 | Diff -1 | Diff +1 |
|----|----|----|----|
| 2024 | 102 | 480 | 1 |
| 2025 | 89 | 475 | 5 |
| 2026 | 102 | 496 | 0 |

Paritaeten (aus dem Lauf ermittelt):

| VZ | Diff | GETTSIM-Ergebnis | gemeinsames zvE Z |
|----|------|------------------|-------------------|
| 2024 | -2 | gerade | ungerade |
| 2024 | -1 | ungerade | gemischt |
| 2024 | +1 | ungerade | gerade |
| 2025 | -2 | gerade | ungerade |
| 2025 | -1 | ungerade | gemischt |
| 2025 | +1 | ungerade | gerade |
| 2026 | -2 | gerade | gemischt |
| 2026 | -1 | ungerade | gemischt |

**Ursache: drei getrennte Effekte.**

1. **Rundungsreihenfolge.** Das literale § 32a Abs. 5 i.V.m. Abs. 1 Satz 6 berechnet `2 * abrunden(Tarif(abrunden(Z/2)))`; der Steuerbetrag der Haelfte ist bereits auf volle Euro abgerundet, das Ergebnis daher stets gerade (so die amtliche Splittingtabelle). GETTSIM rundet erst am Ende: `abrunden(2 * Tarif(Z/2))`. Hat der Halbtarif einen Nachkommaanteil >= 0,5, liegt GETTSIM um 1 Euro hoeher (Diff -1); das GETTSIM-Ergebnis ist dann ungerade.

2. **Fehlende Abrundung von Z/2.** GETTSIM halbiert das gemeinsame zvE ohne Abrundung auf volle Euro. Bei ungeradem Z ist Z/2 = x,5 und GETTSIM wertet den Tarif an x,5 statt an x aus. In Kombination mit Effekt 1 entsteht eine Abweichung von 2 Euro (Diff -2); das GETTSIM-Ergebnis ist dann gerade. Diese Faelle treten praktisch nur bei ungeradem gemeinsamem zvE auf.

3. **Koeffizienten-Approximation am halbierten Einkommen (Klasse A).** Vereinzelt fuehrt GETTSIMs voll aufgeloeste Koeffizienten-Rekonstruktion (siehe Klasse A) am Wert Z/2 dazu, dass Catala um 1 Euro hoeher liegt (Diff +1).


**Reproduktionsbeispiel (Diff -2).** VZ 2024, gemeinsames zvE 43 139 (ungerade): literal `2 * abrunden(Tarif(21 569))` = 4 244 Euro, GETTSIM 4 246 Euro.


**Bewertung: erklaert (GETTSIM-Vereinfachung).** Entscheidung vom 2026-07-09: der Gesetzeswortlaut ist massgeblich, Catala bleibt auf `2 * abrunden(Tarif(abrunden(Z/2)))`. Die Abweichungen sind Vereinfachungen in GETTSIM (Effekte 1 und 2) bzw. eine GETTSIM-Approximation (Effekt 3), kein Fehler in Catala.


**Amtliche Bestaetigung (drittes Oracle, BMF-Steuerrechner, bmf-steuerrechner.de, 2026-07-09).** Zwei Splitting-Divergenzfaelle wurden am amtlichen BMF-Lohn- und Einkommensteuerrechner geprueft:

| VZ | gemeinsames zvE | Catala | GETTSIM | BMF-Steuerrechner |
|----|-----------------|--------|---------|-------------------|
| 2024 | 23 634 | 8 | 9 | **8** |
| 2025 | 24 342 | 20 | 21 | **20** |

In beiden Faellen bestaetigt der amtliche Rechner die Wortlaut-Lesart und damit das Catala-Ergebnis. Divergenzklasse B ist damit endgueltig geschlossen.


## Nicht ausgeloeste, aber bekannte Unterschiede

- **Abrundung des zvE auf volle Euro (§ 32a Abs. 1 S. 1).** Catala rundet das zvE auf volle Euro ab; GETTSIM floort das zvE im Tarif nicht. Der Testgrid verwendet ausschliesslich ganzzahlige Euro-Werte, daher wird dieser Unterschied hier nicht ausgeloest. Er ist bei nicht ganzzahligem zvE relevant und separat zu pruefen.

