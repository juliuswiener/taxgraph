# Golden-Korpus × GETTSIM 1.2.1 — Cross-Check (Paket 9, Verifikations-Haertung)

Erzeugt von `oracle/gettsim/golden_crosscheck.py` (deterministisch, LLM-frei, $0). Vergleicht die gepinnten Golden-Werte fallweise gegen GETTSIM 1.2.1, wo GETTSIM einen Vergleichswert liefert. Keine stille Toleranz: jede Abweichung ist gelistet.


## GETTSIM-Issue-Status (github ttsim-dev/gettsim, gelesen 2026-07-16)

| Issue | Titel | Stand |
|-------|-------|-------|
| [#1209](https://github.com/ttsim-dev/gettsim/issues/1209) | § 32a Abs. 5 Splitting rundet nach Verdopplung | **OPEN**, Label `bug`, keine PR/Resolution |
| [#1210](https://github.com/ttsim-dev/gettsim/issues/1210) | § 32a Abs. 1 Progressionsfaktor 1-Euro-Abweichung | **OPEN**, Label `bug`, keine PR/Resolution |

Beide von juliuswiener am 2026-07-09 eroeffnet, unveraendert offen. GETTSIM 1.2.1 traegt beide Bugs weiter. In diesem Lauf zeigen sich 8 Splitting-Abweichungen (#1209) und 0 Grundtarif-Abweichungen (#1210) — GETTSIM-Signatur, kein Catala-Fehler.


## Coverage (alle 135 Golden-Faelle)

| Domaene | Faelle | GETTSIM-deckungsfaehig | Begruendung |
|---------|--------|------------------------|-------------|
| arbeitnehmer (Bruttolohn->ESt) | 6 | NEIN | Werbungskosten-/Feed-in-Groesse, kein eigenstaendiger GETTSIM-Vergleichswert |
| ep (Entfernungspauschale) | 4 | NEIN | Werbungskosten-/Feed-in-Groesse, kein eigenstaendiger GETTSIM-Vergleichswert |
| g32a (§ 32a-Tarif) | 47 | JA (47) | § 32a-Tarif direkt vs GETTSIM einkommensteuer.betrag |
| gesamt (festzusetzende ESt) | 26 | NEIN (verschoben) | volle festzusetzende ESt: § 31-Guenstigerpruefung/§ 35a/§ 24a/Vorsorge-Deckel; Node-Mapping nicht 1:1, separater Harness noetig — NICHT forciert (falsche Deltas vermeiden) |
| gewst (Gewerbesteuer) | 10 | NEIN | koerperschafts-/gewerbesteuerlich, KEIN GETTSIM-Analog |
| ho (Homeoffice) | 3 | NEIN | Werbungskosten-/Feed-in-Groesse, kein eigenstaendiger GETTSIM-Vergleichswert |
| kst_nenner_b (Koerperschaftsteuer) | 15 | NEIN | koerperschafts-/gewerbesteuerlich, KEIN GETTSIM-Analog |
| sonstige | 24 | NEIN | Werbungskosten-/Feed-in-Groesse, kein eigenstaendiger GETTSIM-Vergleichswert |

Deckungsfaehig und verglichen: **47** § 32a-Tarif-Faelle. Nicht deckungsfaehige Domaenen sind oben ausgewiesen (kein stiller Ausschluss); ihre Verifikation laeuft ueber den eigenen Golden-Gate (value + Zitatanker) bzw. — fuer gesamt — einen kuenftigen festzusetzende-ESt-Harness (Backlog).


## § 32a-Tarif-Cross-Check: 47 Faelle, 39 deckungsgleich, 8 Abweichungen

Jede Abweichung ist ein Item mit Cent-Delta und Ursachen-Hypothese. Delta = (unser Golden-Wert − GETTSIM), positiv = Catala hoeher.

| Fall | VZ | Verfahren | zvE | unser Wert | GETTSIM | Delta (ct) | Ursachen-Hypothese |
|------|----|-----------|-----|-----------|---------|-----------|--------------------|
| g32a_2024_split_100000 | 2024 | zusammen | 100000 | 21744 | 21745 | -100 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |
| g32a_2024_split_23634 | 2024 | zusammen | 23634 | 8 | 9 | -100 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |
| g32a_2024_split_43139 | 2024 | zusammen | 43139 | 4244 | 4246 | -200 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |
| g32a_2025_split_24342 | 2025 | zusammen | 24342 | 20 | 21 | -100 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |
| g32a_2025_split_52150 | 2025 | zusammen | 52150 | 6430 | 6431 | -100 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |
| g32a_2025_split_55555 | 2025 | zusammen | 55555 | 7360 | 7361 | -100 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |
| g32a_2025_split_60000 | 2025 | zusammen | 60000 | 8606 | 8607 | -100 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |
| g32a_2026_split_77777 | 2026 | zusammen | 77777 | 13718 | 13719 | -100 | GETTSIM-Bug #1209 (offen): Splitting rundet NACH Verdopplung statt den bereits gerundeten Halbtarif zu verdoppeln (+ fehlende Abrundung von Z/2). § 32a Abs. 5 i.V.m. Abs. 1 S. 6 verlangt 2*abrunden(Tarif(abrunden(Z/2))); Catala = Wortlaut = BMF-Steuerrechner. NICHT unser Fehler. |

**Triage:** alle Abweichungen fallen auf die zwei offenen GETTSIM-Bugs (#1209 Splitting, #1210 Grundtarif). Catala folgt dem Gesetzeswortlaut und ist am amtlichen BMF-Steuerrechner bestaetigt (siehe reports/s02-divergenzen.md, 2026-07-09). Kein Catala-Fund; Adjudikation der Triage-Klasse ueber Instructor.


## Deckungsgleiche Faelle (Regressions-Anker)

39 von 47 Tarif-Faellen sind mit GETTSIM cent-exakt deckungsgleich (im Wesentlichen der Grundtarif und die geraden/aufgehenden Splitting-Faelle). Diese Menge ist ueber `tests/test_gettsim_crosscheck.py` als deterministischer Gate verankert: die deckungsgleichen Faelle MUESSEN deckungsgleich bleiben, jede neue Abweichung schlaegt rot.

