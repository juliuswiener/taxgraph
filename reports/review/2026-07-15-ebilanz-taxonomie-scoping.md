# E-Bilanz-Taxonomie-Scoping (§ 5b EStG) — W2-Anschluss (taxgraph-dev-2, 2026-07-15)

Instructor-Auftrag: read-only Befund-Report analog E77/EÜR-Feldmapping. Alle Funde aus
ERiC 44.2.4.0 (`~/02_Software/eric`, $ERIC_DIR-Env leer, Default-Pfad greift) + Bestands-
Infra (`elster/`). KEIN Bau. Meine Zone (reports/); elster/, sources/ nicht angefasst.

## (a) ELSTER-Datenart + Schema-Lage

**Datenart: `ElsterBilanz` / DatenArt `Bilanz`** (§ 5b EStG E-Bilanz). Beleg: ERiC
`Dokumentation/Datenarten/ElsterBilanz/Bilanz/` + Beispiel-XML-Header
`<Verfahren>ElsterBilanz</Verfahren> <DatenArt>Bilanz</DatenArt>`.

**Struktur grundverschieden zu E10/E77:** die Nutzdaten sind KEINE flache Kz-Liste,
sondern eine **XBRL-Instanz**. Der Elster-Envelope (v11 TransferHeader/DatenTeil) umschließt
`<ebilanz:EBilanz>` (NS `rzf.fin-nrw.de/RMS/EBilanz/2016`) → darin `<xbrli:xbrl>` mit
zwei `schemaRef` auf die Fach-Taxonomie.

**Schema in $ERIC_DIR — TEILWEISE:**
- ✅ ELSTER-Transport-Envelope: `.../ElsterBilanz/Bilanz/Schema/ebilanz_000002.xsd`
  (+ `ebilanz_elster_000001.xsd`). Definiert nur die Hülle, NICHT die Bilanzpositionen.
- ✅ 15 Beispiel-Instanzen je Taxonomie-Version (`Beispiele/Taxonomie_5_0 … 6_9`),
  Handels-/Steuerbilanz × PersG/GmbH/Bank/Landwirt.
- ❌ **Fach-Taxonomie (`de-gaap-ci`, `de-gcd`) NICHT in ERiC enthalten** — nur per URL
  referenziert (`http://www.xbrl.de/taxonomies/de-gaap-ci-2025-04-01/…-shell-fiscal.xsd`).
  `find ~/02_Software/eric -iname "*gaap*"` = 0 Treffer. → separater Taxonomie-Download
  (XBRL Deutschland / esteuer.de) nötig, analog Vordruck-PDF-Boundary (Julius-Aktion).

## (b) Taxonomie-Struktur ↔ W2-Bilanz-Regeln

**Zwei Module (XBRL):**
- `de-gcd` = Global Common Document (Stammdaten: Unternehmen, Bilanzart, WJ, Steuernummer).
- `de-gaap-ci` = Kerntaxonomie Bilanz + GuV ("commercial/industrial"). Produktiv-Version
  **6.9 (2025-04-01)** für WJ 2025 (Beispiel nutzt genau diese).

**Kerntaxonomie vs Branchentaxonomie:** de-gaap-ci ist die Kerntaxonomie (alle Nicht-
Spezialbranchen). Branchentaxonomien (Bank, Versicherung, Landwirt, Krankenhaus, Wohnungs-
wirtschaft, Verkehr) sind Erweiterungen — in ERiC als eigene Beispiele belegt
(`HandelsbilanzBank.xml`, `HandelsbilanzLandwirt_GmbH.xml`). **Für unseren W2-Nenner
(natürliche Personen / Einzelunternehmer / PersG) = Kerntaxonomie de-gaap-ci; Branchen =
Nicht-Gegenstand.**

**Element-Modell:** hierarchischer Punkt-Pfad (`bs`=Bilanz, `ass`=Aktiva, `eqLiab`=Passiva,
`is`=GuV). W2-Regel-Outputs docken DIREKT an konkrete Elemente an — belegt aus
`SteuerbilanzAutoverkaeufer` (Tax. 6.9):

| W2-Regel | de-gaap-ci-Element (Beleg) | Bezug |
|---|---|---|
| p4_1_bv_vergleich (§4 I) | `bs.eqLiab.equity.netIncome.taxBalanceGenerally` (+`.EStGs`, `.transferDiffTaxAccounts`) | steuerbilanzieller Gewinn/EK-Bewegung — Kern-Andock |
| p6_1_1_bewertung_av (§6 I Nr.1) | `bs.ass.fixAss.*` (Anlagevermögen) + Anlagenspiegel | Bewertungs-min AV |
| p6_1_1_wertaufholung | Anlagenspiegel (Zuschreibung) | Zuschreibungsgebot |
| p6_1_3a_abzinsung (§6 I Nr.3a) | `bs.eqLiab.accruals.other.uncertainLiab`, `.accruals.tax` | Rückstellungs-Abzinsung |
| p6a_pension (§6a) | `bs.eqLiab.accruals.pensions` (`.direct`/`.externalFunds`/`.shareholder`) | Pensionsrückstellung |
| p5_5_aktiver_rap (§5 V) | `bs.ass.prepaidExpense` (aktiver RAP) | Rechnungsabgrenzung |

**Steuerbilanz vs Handelsbilanz+Überleitung** (ERiC-Beispiele trennen beide) = direkter
Bezug zu W2-B2 Maßgeblichkeit (§5 I): entweder Steuerbilanz direkt ODER Handelsbilanz +
`transferDiffTaxAccounts`-Überleitungsrechnung. Unsere W2-Regeln liefern die
STEUERlichen Bewertungs-Korrekturen = die Überleitungs-/Steuerbilanz-Seite.

## (c) kz_extract-Readiness — NEUER Extraktor nötig

**kz_extract.py ist reines Flach-Kz-Modell und für E-Bilanz UNBRAUCHBAR:** es parst
7-stellige Kz (`E\d{7}`) aus E10-HTML-Sektionsankern bzw. E77-XSD
(`_ANCHOR = href="#SEKTION_hash_CType-E\d{7}"`, `_DOC = name="E\d{7}"`). **E-Bilanz hat
KEINE Kz** — die Felder sind XBRL-Taxonomie-Elemente (`de-gaap-ci:bs.ass…`) mit Kontext-
Referenz + Dimensionen. Kein einziger `E\d{7}`-Treffer möglich.

→ **Neuer Extraktor** `elster/ebilanz_taxonomie_extract.py`: parst die de-gaap-ci-`.xsd` +
presentation-/label-Linkbase → je Element: technischer Name, deutsches Label, Hierarchie-
Pfad, Muss/Kann-Feld-Klassifikation, Kontenzuordnung. Grundlegend andere Mechanik als das
E10/E77-Flach-Kz-Mapping (XBRL-Linkbase-Parser statt Anker-Regex). Bestands-Doktrin bleibt:
Extraktor RÄT NICHT die Regel↔Element-Zuordnung — Kuratierung + Instructor-Review,
Zitatanker-Doktrin auf Element-Ebene.

## (d) Aufwands-Schätzung Feldmapping

**Skalen-Warnung:** eine MINIMALE Steuerbilanz (Autoverkäufer-PersG) nutzt bereits
**215 distinkte Taxonomie-Elemente**; die volle de-gaap-ci-Kerntaxonomie hat mehrere
Tausend. E-Bilanz ist damit VIEL größer als eine Anlage (E77 = 1169 Kz einer Datenart).
Unsere sechs W2-Regeln decken nur die BEWERTUNGS-MECHANIK eines Bruchteils der Positionen
ab — der Großteil (GuV-Population `is.*`, Kontennachweise, Stammdaten, alle nicht von einer
Bewertungsregel berührten Bilanzzeilen) ist **Nicht-Gegenstand** wie im W2-Nenner
bereits vermerkt (E-Bilanz-Taxonomie = eigener Feldmapping-Schritt nach den Kern-Regeln).

**Arbeitspakete (geschätzt):**
1. **Taxonomie-Beschaffung** (Julius-Download de-gaap-ci/de-gcd 6.9 von esteuer.de, wie
   ERiC/Vordrucke) + Freeze `sources/xbrl/` + `sources-check`-Anker. — Boundary, $0 für mich.
2. **Extraktor** `ebilanz_taxonomie_extract.py` (XBRL-xsd/linkbase-Parser). — mittel, LLM-frei.
3. **Scope-Eingrenzung** auf (i) Muss-Felder der Kerntaxonomie ∩ (ii) von W2-Regeln berührte
   Positionen (~6 Andock-Cluster oben). Ergebnis: kleine kuratierte Regel↔Element-Tabelle,
   NICHT die ganze Taxonomie. — klein.
4. **Mapping-Tabelle** je W2-Regel-Output → de-gaap-ci-Element (Format analog E77-Methodik:
   Regel-Output | Element-Pfad | Label | Muss/Kann | Konfidenz) → Instructor-Review. — klein.
5. **Überleitungs-/Maßgeblichkeits-Seite** (B2): Handelsbilanz→Steuerbilanz-Überleitung
   (`transferDiffTaxAccounts`) als eigener Mapping-Block. — mittel.

**Grob:** Beschaffung (Julius) → Extraktor + Muss-Feld-Scope (1 Arbeitsblock, $0, LLM-frei,
Agent-Fanout möglich) → kuratierte 6-Cluster-Mapping-Tabelle (Review). KEINE Cascade-/
Formalisierer-Kosten (Feldmapping ist Deklarations-Zuordnung, kein Regel-Bau) — analog
W1/E77 ($0). Größenordnung Kurier-Aufwand > E77 wegen Taxonomie-Tiefe, aber durch
Muss-Feld ∩ W2-Schnitt beherrschbar.

## Offene Punkte / Boundary
- Taxonomie-Download = Julius-Aktion (Netzwerk, User-Boundary; wie Vordruck-PDFs). Danach
  rein lokal.
- Taxonomie-Versions-Politik: produktiv 6.9 (WJ 2025). Multi-VZ-Bezug: E-Bilanz-Version
  folgt dem WIRTSCHAFTSJAHR, nicht dem VZ — eigene Versions-Achse (Gültigkeits-Direktive:
  je WJ die gültige Taxonomie-Version). Für VZ 2024 = Tax. 6.7/6.8, VZ 2025 = 6.9 — beim
  Freeze zu fixieren.
- Neuer Extraktor + sources/xbrl/ liegen in elster/-/sources/-Zone (nicht meine) →
  Zonen-Zuweisung durch Instructor nötig, falls ich baue.
