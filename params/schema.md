# Parameterschicht - kanonisches Format

Jahreswerte (Freibetraege, Pauschbetraege, Hoechstbetraege, Tarifkonstanten)
liegen als versionierte Daten getrennt von den Formeln, ein Verzeichnis je
Veranlagungszeitraum (`params/<vz>/`), eine Datei je Parameterthema. Muster nach
OpenFisca (Parameter als Daten).

## Dateiaufbau

```yaml
# Kopfkommentar: Rechtsquelle des Themas, Datenquelle, Besonderheiten.

parameter: <thema>                 # optionaler Themenname
veranlagungszeitraum: <jahr>
authority: gesetz|verwaltung|bfh|fg   # Quellenklasse (Quellenmodell)
redistributable: true|false           # Exportierbarkeit
gueltig_ab: "<jahr>-01-01"            # Gueltigkeitsbeginn

<parametername>:
  wert: <zahl>
  einheit: euro|prozent|faktor
  veranlagungszeitraum: <jahr>
  rechtsquelle: {gesetz: EStG, paragraph: "9a", absatz: "1", nummer: "1a"}
  datenquelle: "<Herkunftsvermerk>"
  # weitere Parameter analog ...
```

## Pflichtfelder je Parameter

- `wert`: der Zahlenwert.
- `veranlagungszeitraum`: Geltungsjahr.
- `rechtsquelle`: Gesetz, Paragraph, Absatz, Satz bzw. Nummer.
- `datenquelle`: woher der Wert stammt. Bei GETTSIM-Import mit Version, Pfad
  innerhalb GETTSIM, dem verwendeten Datumseintrag und dem Importdatum.
- `authority`, `redistributable` (auf Dateiebene, gelten fuer alle Parameter der
  Datei).

## GETTSIM-Import

`params/import_gettsim.py` liest GETTSIM-Parameterdateien (date-keyed Eintraege),
waehlt je VZ den zum 1. Januar gueltigen Eintrag und schreibt eine Parameterdatei
im obigen Format. Der Herkunftsvermerk (`datenquelle`) enthaelt GETTSIM-Version,
den relativen Pfad, den verwendeten Datumseintrag samt BGBl-Referenz und das
Importdatum. Import per `make params-import`.

Wichtig (Verifikationsprinzip): GETTSIM ist Datenquelle und Prueinstanz, nicht
Rechtsquelle. Die `rechtsquelle` verweist immer auf das Gesetz; die
GETTSIM-Herkunft steht in `datenquelle`.
