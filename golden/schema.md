# Golden-Korpus - Fallschema

Jeder Fall ist eine YAML-Datei unter `golden/cases/`. Das Schema ist generisch
angelegt, damit spaeter Feldwert-Erwartungen (ELSTER) ohne Umbau ergaenzt werden
koennen. In v1 sind nur Tarif-/§ 32a-Faelle enthalten.

```yaml
id: string                     # eindeutige Fall-ID, stabil
beschreibung: string           # kurze Beschreibung

sachverhalt:                   # typisierte Eingaben (Faktenmodell-Vorstufe)
  veranlagungszeitraum: 2024|2025|2026
  veranlagung: einzel|zusammen
  zu_versteuerndes_einkommen: int   # ganze Euro

erwartung:
  tarifliche_est: int          # erwartete tarifliche ESt in ganzen Euro
  # spaeter optional:
  # feldwerte: { "<elster-feld-id>": <wert>, ... }

quelle:
  authority: gesetz|verwaltung|bfh|fg   # Quellenklasse
  redistributable: bool
  fundstelle: string           # praezise Fundstelle inkl. VZ-spezifischer Quelle
  datei: string                # Datei in sources/, gegen die der Anker geprueft wird
  zitatanker: string           # woertliches Kurzzitat, im Quelltext verifizierbar
```

## Zitatanker (hartes Gate)

`golden/runner.py` prueft je Fall deterministisch:
1. Der `zitatanker` kommt als Teilstring (nach Normalisierung: Kleinschreibung,
   Whitespace zusammengefasst) im referenzierten `sources/`-Dokument vor.
2. Die von der Catala-Formalisierung berechnete tarifliche ESt entspricht
   `erwartung.tarifliche_est`.

Faelle ohne verifizierbaren Anker oder mit abweichendem Ergebnis lassen den
Lauf fehlschlagen (Exit != 0).

## Herkunft der Erwartungswerte

Die Tarif-Erwartungswerte sind aus dem publizierten geschlossenen § 32a-Tarif
(literal bestaetigte Koeffizienten, siehe `params/`) unabhaengig von der
Catala-Implementierung berechnet (`golden/generate_cases.py`). Fuer den
Einzeltarif sind sie zusaetzlich gegen GETTSIM abgesichert (siehe
`reports/s02-divergenzen.md`). Fuer das Splitting gilt der Gesetzeswortlaut
`2 * abrunden(Tarif(abrunden(Z/2)))`.
