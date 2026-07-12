# Feldmapping ESt1A/Anlagen — Methodik (Kz-Kuratierung)

Instructor-Richtung (a, 2026-07-12): das Feldmapping bildet DEKLARATIONS-Größen (was der
Steuerpflichtige in ESt1A/Anlagen einträgt) auf amtliche ELSTER-Kz ab. Berechnete Größen
(tarifliche/festzusetzende Steuer) gehören NICHT in die Deklaration (Gegencheck: GETTSIM-
Differential; ERiC checkESt validiert die Deklaration). Kein Kz-Raten — Zitatanker-Doktrin auf
Kz-Ebene.

## Quellen-Rangfolge (evidenzbasiert umgekehrt, 2026-07-12)

Ursprünglich geplant: XSD primär, Vordruck als Kreuzcheck. Der Bau hat das umgekehrt:

- **PRIMÄRQUELLE: amtlicher Vordruck** (ESt 1 A 2025, Anlage N 2025; formulare-bfinv.de, öffentlich).
  Der Vordruck zeigt menschenlesbar und amtlich **Zeile ↔ Konzept ↔ Kz** — dort, wo der
  Steuerpflichtige den Betrag deklariert. Das ist die zuverlässige Konzept→Primär-Kz-Zuordnung.
- **ZWEITBELEG: ESt-Schemadok E10-<vz>.html** (ERiC-Auslieferung). Bestätigt Kz-Existenz, Typ und
  Sektions-Kontext. `elster/kz_extract.py` liefert je Kz Sektions-Pfad + wörtliches Label aus den
  SVG-Anker-IDs (`#<SEKTION>_<hash>_CType-E<Kz>`).

**Warum die Umkehr (Bau-Fund):** die E10-XSD-Labels sind Vordruckzeilen-quervernetzt („in Zeile
$E….Vordruckzeile$ enthaltene Aufwendungen…"). Die Konzept-Sektionen (AgB, Hhn_BV_DL_HL) enthalten
überwiegend Quer-/Ableitungs-Kz; die PRIMÄREN Deklarations-Felder sind die referenzierten
Vordruckzeilen-Kz, verstreut. Der XSD-Baum gibt Sektions-KONTEXT (mittel), aber nicht sauber die
Primär-Kz je Konzept. Ein Kandidaten-Tabelle nur aus dem XSD wäre geraten.

## Konfidenz je Kandidaten-Zeile (Instructor-Klassifikation)

- **STARK**: Vordruck-Zeilentext UND XSD-Sektionspfad zeigen dasselbe Konzept.
- **MITTEL**: nur XSD-Sektionspfad (Vordruck noch nicht gegengeprüft).
- **KONFLIKT**: Vordruck und XSD widersprechen → Eskalation an Instructor.

## Tabellen-Format (Review-Artefakt, je Zeile)

| unser Regel-Input | Vordruck-Zeile (Nr. + Text) | Kz-Kandidat(en) | wörtliches XSD-Label | XSD-Sektion | E10-Ref | Favorit + Ein-Satz-Begründung | Konfidenz |

## Ablauf (sobald die Vordruck-PDFs lokal liegen)

Der Netzwerk-Download der PDFs ist NICHT meine Aktion (User-Boundary; Kanal-Freigabe zählt nicht) —
Julius lädt sie selbst (wie ERiC) ODER gibt direkt OK. Danach rein lokal:

1. `scripts/freeze_pdf_local.py <pfad>` friert das lokale PDF nach `sources/bfinv/` ein: pdftotext,
   `--erwarte`-Anker-Prüfung („Außergewöhnliche Belastungen" im ESt1A, „Entfernungspauschale" in
   Anlage N), sha256 des PDF + Text, `.meta.yaml` (URL, Abrufdatum, authority: amtlicher_vordruck).
   `make sources-check` grün.
2. Vordruck-Zeilen ↔ Kz für die MVP-Konzepte extrahieren; mit `kz_extract.py` (XSD-Sektion) kreuzen.
3. STARKE Kandidaten-Tabelle Mantelbogen/ESt1A → Instructor-Review. Danach Anlage N.

## MVP-Konzepte (unsere Regel-Inputs, Mantelbogen/ESt1A zuerst)

- Kirchensteuer gezahlt/erstattet (p10_1_4) — Sonderausgaben.
- außergewöhnliche Belastungen allgemeiner Art (p33).
- haushaltsnahe Dienstleistungen / Handwerkerleistungen (§35a, p35a).
- (Anlage N, danach) Entfernungspauschale (p09), Homeoffice/Arbeitszimmer (p04),
  Arbeitnehmer-Pauschbetrag.
