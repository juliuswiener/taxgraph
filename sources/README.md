# sources/ - Fassungsarchiv (eingefrorene Quellabrufe)

Einfache, dateibasierte Vorstufe des spaeteren Dokumentstores (Roadmap M1.5).
Jeder abgerufene Rechtstext wird hier versioniert und unveraenderlich abgelegt:
Originaltext plus eine Metadatei mit URL, Abrufdatum und SHA256-Hash. Das
VZ-Scoping und die spaetere Migration in Postgres brauchen dieses Archiv, weil
NeuRIS historische Fassungen noch nicht vollstaendig liefert.

## Konventionen

- Pro Abruf zwei Dateien: `<norm>_<datum>.txt` (Wortlaut) und
  `<norm>_<datum>.meta.yaml` (Metadaten).
- Metadaten enthalten mindestens: `norm_uri`, `quelle_url`, `abrufdatum`,
  `sha256`, `authority`, `redistributable`.
- `authority`: Quellenklasse nach dem Quellenmodell (`gesetz`, `verwaltung`,
  `bfh`, `fg`, `literatur`). In Phase 0 ausschliesslich `gesetz`.
- `redistributable`: ob der Inhalt in Open-Source-Exporte darf. Gesetzestexte:
  `true`. Kommentar/Literatur: `false` (Phase 0 nutzt keine solchen Quellen).
- Dateien in diesem Verzeichnis werden nach dem Anlegen nicht mehr editiert
  (immutable). Eine neue Fassung ist ein neuer Abruf mit neuem Datum.

## Integritaet pruefen

```bash
make sources-check
```

Vergleicht jeden Wortlaut deterministisch gegen den in der Metadatei
hinterlegten Hash. Grundlage fuer das spaetere Zitatanker-Gate (woertliches
Kurzzitat gegen Quellsegment).

## Aktueller Bestand

| Norm | Fassung | Abruf | authority |
|------|---------|-------|-----------|
| § 32a EStG | ab VZ 2026 | 2026-07-09 | gesetz |
| § 4 Abs. 5 S. 1 Nr. 6b/6c EStG | ab 2023 | 2026-07-09 | gesetz |
