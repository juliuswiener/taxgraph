# Golden-Test-Korpus

Das zentrale Verifikationsasset (Roadmap M1.4). Kuratierte Testfaelle
(Sachverhalt -> erwartetes Ergebnis, mit Quelle und Zitatanker), gegen die die
Regelbibliothek gehalten wird. Der Korpus waechst monoton ueber alle Phasen; kein
Release bei rotem Korpus.

## Stand v1

48 Faelle: § 32a-Tarif und Arbeitnehmerfall end-to-end (Grund- und Splittingtarif, VZ 2024/2025/2026),
inklusive der gesetzlichen Randwerte (Grundfreibetrag, Zonengrenzen) und des
BMF-Rechner-Spot-Check-Falls (gemeinsames zvE 23 634, VZ 2024, erwartet 8 Euro).
Das Fallschema ist bereits generisch fuer spaetere Feldwert-Erwartungen angelegt
(siehe `schema.md`).

## Ausfuehren

```bash
make golden
```

Prueft je Fall (a) den Zitatanker deterministisch gegen den eingefrorenen
Quelltext in `sources/` und (b) die von Catala berechnete tarifliche ESt gegen
den Erwartungswert. Exit != 0 bei jeder Abweichung.

## Struktur

```
golden/
  schema.md            # Fallschema
  cases/*.yaml         # die Testfaelle (kuratiertes Artefakt)
  generate_cases.py    # erzeugt die § 32a-Faelle aus dem publizierten Tarif
  runner.py            # Wert- und Zitatanker-Pruefung
```

## Herkunft der Erwartungswerte

Berechnet aus dem publizierten, literal bestaetigten § 32a-Tarif (siehe `params/`),
unabhaengig von der Catala-Implementierung. Der Einzeltarif ist zusaetzlich gegen
GETTSIM abgesichert (`reports/s02-divergenzen.md`). Splitting folgt dem
Gesetzeswortlaut `2 * abrunden(Tarif(abrunden(Z/2)))`.

## Naechste Schritte (spaetere Phasen)

- Feldwert-Erwartungen (`erwartung.feldwerte`) sobald das ELSTER-Feldmodell steht.
- Publizierte BFH-Faelle mit Zahlen (authority `bfh`), sobald die
  Rechtsprechungs-Ingestion laeuft.
- Rechenbeispiele aus BMF-Ausfuellanleitungen und EStH/LStH mit eigenem
  Quell-Freeze in `sources/`.
