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
| § 9a EStG | geltend (Nr. 1a 1230 Euro) | 2026-07-09 | gesetz |
| § 10c EStG | geltend (36 Euro) | 2026-07-09 | gesetz |
| § 9 Abs. 1 Nr. 4/Abs. 2 EStG | 2026 (StAendG 2025, 0,38 ab km 1) | 2026-07-09 | gesetz |
| BMF-Schreiben Entfernungspauschalen | 18.11.2021 (BStBl I 2021, 2315) | 2026-07-09 | verwaltung |

## Einfrier-Ebene: mindestens ein ganzer Paragraph

Verbindlich ab 2026-07-10 (Protokolldekret Julius).

Quellen werden **mindestens auf Paragraphen-Ebene** eingefroren, nie als
Absatz-Ausschnitt. `estg_p35a.txt`, nicht `estg_p35a_abs2_3.txt`.

Grund, an einem konkreten Schaden gelernt: Der Ausschnitt `§ 35a Abs. 2 und 3`
enthielt weder Abs. 1 (Minijob-Höchstbetrag 510 Euro) noch Abs. 5 (Rechnung,
unbare Zahlung, „gilt nur für Arbeitskosten"). Beim Neuschnitt der Regel nach der
Abgrenzungsregel stellte sich heraus, dass **drei von fünf** Geltungsbedingungen
gar nicht zitierfähig waren — der Wortlaut, auf den sie sich stützen, lag
außerhalb des eingefrorenen Ausschnitts. Ein Zitatanker, der ins Leere zeigt,
fällt zwar auf; ein *fehlender* Absatz fällt nicht auf, weil niemand nach etwas
sucht, das nicht da ist.

Ein Absatz-Ausschnitt trifft implizit eine Aussage darüber, was für die Regel
relevant ist. Diese Aussage gehört ins Manifest (Signatur, Geltungsbedingungen,
`auszug`), nicht in den Dateinamen der Quelle.

Wo ein Paragraph sehr lang ist (§ 32a, § 9), bleibt die Datei trotzdem der ganze
Paragraph; der für eine Regel maßgebliche Ausschnitt wird über das Feld `auszug`
im Manifest gebildet und dort **wörtlich gegen die Quelle geprüft** (siehe
`pipeline/quellen.py`).

### Bestandsaufnahme (2026-07-10)

Zwölf Quellen liegen noch auf Absatz-Ebene. Sie werden bei der nächsten Berührung
nachgezogen — ein Nachziehen ändert den Prompt und erzwingt einen neuen Lauf der
betroffenen Regel, deshalb nicht auf Vorrat:

| Quelle | genutzt von |
|---|---|
| `estg_p9_abs1nr5` | p9_1_3_nr5_doppelte_haushaltsfuehrung |
| `estg_p9_abs1nr5a` | p9_1_3_nr5a_uebernachtung (Neuschnitt Charge 2) |
| `estg_p9_abs1nr6` | p9_1_3_nr6_arbeitsmittel (Neuschnitt Charge 2) |
| `estg_p9_abs1nr7` | p9_1_3_nr7_afa (Neuschnitt Charge 2) |
| `estg_p9_abs4a` | p9_4a_verpflegungsmehraufwand |
| `estg_p9_abs6` | p9_6_erstausbildung_abgrenzung |
| `estg_p10_abs1nr7` | p10_1_7_berufsausbildung |
| `estg_p33_abs3` | p33_3_zumutbare_belastung |
| `estg_p6_abs2` | Charge 2 (Neuschnitt Nr. 6+7) |
| `estg_p04_abs5` | Handregel Arbeitszimmer/Homeoffice |
| `estg_p9_abs1nr4_abs2` | Handregel Entfernungspauschale |
| `estg_p35a_abs2_3` | **verwaist** — ersetzt durch `estg_p35a` (Abs. 1-5) |

Die fünf `§ 9`-Ausschnitte fallen bei der Zusammenlegung von Nr. 6 und Nr. 7 in
Charge 2 ohnehin zusammen: dort wird der ganze § 9 eingefroren.
