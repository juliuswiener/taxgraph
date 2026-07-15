# E-Bilanz-Vorarbeit (Paket 2) — Taxonomie-Version×WJ + Beschaffungsplan + Muss-Feld-Kartierung (taxgraph-dev-2, 2026-07-15)

Instructor-Auftrag (msg 2055, JULIUS-WORT Paket 2): Kerntaxonomie beschaffen (Version je WJ
prüfen, Gültigkeits-Direktive), Ablage `sources/ebilanz/` + meta.yaml, Report Muss-Feld ∩ W2.
Read-only Beschaffung+Kartierung, KEIN Extraktor-Code, KEIN Eingriff in pipeline/ (Bau = dev-1
Paket 2). Basis: mein Scoping `2026-07-15-ebilanz-taxonomie-scoping.md` (commit 16f0aa6).

## ⚠ BOUNDARY — Download-Schritt wartet auf Julius DIREKT im Chat
Instructor meldet „Download von Julius autorisiert". Per stehendem Protokoll
([[ausgehende-aktionen-nur-julius]], Push-Vorfall 2026-07-12): **Download = ausgehende Aktion,
braucht Julius direkt im Chat — Instructor-Relay genügt NICHT.** Deshalb hier NUR die
read-only-Teile geliefert (Teil A Version-Gültigkeit vollständig; Teil B Beschaffungs-SPEC;
Teil C Kartierungs-RAHMEN). Der eigentliche Binär-Download nach `sources/ebilanz/` +
sha256/abrufdatum-Fixierung + der voll extrahierte Muss-Feld-Katalog sind auf den Julius-
Download blockiert. Nichts nach `sources/` geschrieben.

---

## Teil A — Version × Wirtschaftsjahr (Gültigkeits-Direktive) — VOLLSTÄNDIG

**Kernbefund: E-Bilanz-Taxonomie-Version folgt dem WIRTSCHAFTSJAHR (WJ), nicht dem VZ.**
Jede Version wird zum 01.04. des Jahres N veröffentlicht und ist grds. **verpflichtend** für
WJ, die nach dem 31.12.N beginnen (= WJ N+1); für das WJ N ist sie **optional nicht-beanstandet**.

| Taxonomie | BMF-Schreiben (GZ) | verpflichtend: WJ beginnt nach | = **WJ (Kalender)** | optional (nicht beanstandet) |
|---|---|---|---|---|
| **6.7** | 09.06.2023 · IV C 6-S 2133-b/22/10002:002 | 31.12.2023 | **WJ 2024** (o. 2024/25) | WJ 2023 |
| **6.8** | 27.05.2024 · IV C 6-S 2133-b/24/10001:002 | 31.12.2024 | **WJ 2025** (o. 2025/26) | WJ 2024 |
| **6.9** | 10.06.2025 · IV C 6-S 2133-b/00064/002/006 | 31.12.2025 | WJ 2026 (o. 2026/27) | WJ 2025 |
| 6.10 | 08.06.2026 | 31.12.2026 | WJ 2027 (o. 2027/28) | WJ 2026 |

**Für die Programm-Zieljahre:**
- **WJ 2024 → verpflichtend Taxonomie 6.7** (6.8 optional zulässig).
- **WJ 2025 → verpflichtend Taxonomie 6.8** (6.9 optional zulässig).
- 6.9 ist die **WJ-2026**-Version — NICHT WJ 2024/2025.

**Korrektur zum Scoping (16f0aa6, Z.106-109):** dort stand ungenau „VZ 2024 = 6.7/6.8, VZ 2025
= 6.9" (VZ/WJ vermischt, 6.9 zu früh angesetzt). Mein Scoping hatte zudem im Body „Kerntaxonomie
6.9" als Produktiv-Version genommen = **die-neueste-still-nehmen**, genau der von der Direktive
verbotene Fehler. Jetzt WJ-präzise: **6.7 (WJ24) / 6.8 (WJ25)**. esteuer.de führt inzwischen
6.10 als neueste → „neueste nehmen" wäre doppelt daneben.

**WJ vs VZ (Multi-VZ-Andock):** bei kalenderjahrgleichem WJ gilt WJ = Kalenderjahr = i.d.R. VZ.
Bei **abweichendem Wirtschaftsjahr** (z. B. 01.07.24–30.06.25) driften WJ-Achse (Taxonomie) und
VZ-Achse (Tarif/params) auseinander — die Taxonomie-Version bemisst sich nach WJ-BEGINN. Für den
Freeze je Artefakt WJ-Fenster dokumentieren, nicht VZ.

**JStG 2024 — neue Pflicht-Bestandteile ab 6.9/WJ 2025:** das BMF-Schreiben zu 6.9 setzt die
JStG-2024-Neuerungen um (u. a. **Kontennachweis-Übermittlungspflicht**) mit Nichtbeanstandungs-
regelung für die Kontennachweise unter 6.9. Für den Muss-Feld-Scope WJ 2025+ relevant (Teil C).

Triangulation: BMF primär (bundesfinanzministerium.de + esteuer.de) + DATEV-Magazin + Haufe +
Otto Schmidt + Zwirner/beck je Version — Wortlaut „grundsätzlich … nach dem 31.12.N … nicht
beanstandet … WJ N" identisch über alle Quellen.

---

## Teil B — Beschaffungsplan + meta.yaml-Skelett (SPEC, Download = Julius)

**Bezugsquelle:** `www.esteuer.de` → Abschnitte „Taxonomien vom 01.04.2023 (6.7)" und
„01.04.2024 (6.8)" (Archiv-Sektion). Amtlich verbindlich ist allein das **XBRL-Paket** (Excel-
Visualisierungen sind unverbindliches Arbeitswerkzeug — aber für die Muss-Feld-LESUNG bequem).

**Zu beschaffende Artefakte je Version (6.7 UND 6.8):**
1. XBRL-Gesamtpaket (ZIP) — enthält `de-gcd` (Stammdaten) + `de-gaap-ci` (Kerntaxonomie
   Bilanz+GuV) + Branchen-/Spezialtaxonomien + Linkbases. **Groß** (XSD + presentation/label/
   definition-Linkbases, mehrere MB entpackt).
2. Excel-Visualisierung **Kerntaxonomie** (Mussfeld-Spalten lesbar) — für Muss-Feld-Extrakt.
3. (optional) Änderungsnachweis 6.8↔6.7 bzw. 6.9↔6.8 — Provenienz der Feldwanderung.

**Ablage-Politik (analog PDF/Vordruck-Freeze):**
- `sources/ebilanz/6.7/` und `sources/ebilanz/6.8/`.
- **Große XBRL-ZIPs NICHT roh ins Repo** (gitignore-analog zu PDFs): entpackt nur die relevanten
  Kern-Dateien einzeln ablegen — `de-gaap-ci-<datum>-shell-fiscal.xsd` + zugehörige presentation-
  & label-Linkbase + `de-gcd`-Shell; sha256 des **Gesamt-ZIP** in meta dokumentieren.
- meta.yaml je Artefakt:
```yaml
# sources/ebilanz/6.8/meta.yaml (Skelett — Werte beim Julius-Download füllen)
artefakt: de-gaap-ci Kerntaxonomie + de-gcd (Taxonomie 6.8)
version: "6.8"
gueltig_wj: "WJ 2025 verpflichtend (WJ 2024 optional nicht-beanstandet)"
gueltig_ab_wj_beginn_nach: "2024-12-31"
quelle_url: "https://www.esteuer.de/  # exakte Datei-URL beim Download fixieren"
bmf_schreiben: "27.05.2024, IV C 6-S 2133-b/24/10001:002, BStBl I S. 928"
abrufdatum: "<YYYY-MM-DD beim Download>"
sha256_gesamtpaket: "<beim Download>"
sha256_einzeldateien:  # je entpackter XSD/Linkbase
  de-gaap-ci-2024-04-01-shell-fiscal.xsd: "<beim Download>"
authority: amtlicher_vordruck-analog  # amtlich vorgeschriebener Datensatz § 5b EStG
```

---

## Teil C — Muss-Feld ∩ W2 (Kartierungs-RAHMEN; voller Katalog = Download-gated)

**Modul-Aufteilung (was ist was):**
- **GCD-Modul** (`de-gcd`, Global Common Document) = **Stammdaten/Dokumentkopf**: Unternehmens-
  Identifikation, Bilanzart (Handels-/Steuerbilanz), Berichts-/Wirtschaftsjahr, Steuernummer,
  Rechtsform. Header-Mussfelder, KEINE Wertpositionen. **W2 berührt GCD nicht** (liefern wir aus
  Fall-Stammdaten, nicht aus Bewertungsregeln).
- **GAAP-Modul** (`de-gaap-ci`) = **Kerntaxonomie Bilanz + GuV** (commercial/industrial). Hier
  liegen die §5b-Muss-Felder der Wertpositionen. **Das ist der W2-Schnitt.** Branchentaxonomien
  (Bank/Versicherung/Landwirt/…) = Nicht-Gegenstand (W2-Nenner = natürl. Person/Einzelunt./PersG).

**W2 ∩ de-gaap-ci — 6 Andock-Cluster** (aus Scoping, ERiC-Beispiel `SteuerbilanzAutoverkaeufer`
Tax. 6.9 belegt; Element-Pfade in 6.7/6.8 zu re-verifizieren, s. u.):

| W2-Regel | de-gaap-ci-Element | Muss/Kann | füllbar heute? |
|---|---|---|---|
| p4_1_bv_vergleich (§4 I) | `bs.eqLiab.equity.netIncome.taxBalanceGenerally` (+`.EStGs`, `.transferDiffTaxAccounts`) | Muss (Kern) | ✅ W2 liefert steuerl. Gewinn/Überleitung |
| p6_1_1_bewertung_av (§6 I Nr.1) | `bs.ass.fixAss.*` + Anlagenspiegel | Muss (Rumpf) / Anlagenspiegel Kann | ⚠ Wert ja, Anlagenspiegel-Tiefe offen |
| p6_1_1_wertaufholung | Anlagenspiegel (Zuschreibung) | Kann | ⚠ nur bei Anlagenspiegel-Übermittlung |
| p6_1_3a_abzinsung (§6 I Nr.3a) | `bs.eqLiab.accruals.other.uncertainLiab`, `.accruals.tax` | Muss (Rückst.-Summe) | ✅ Wert füllbar |
| p6a_pension (§6a) | `bs.eqLiab.accruals.pensions.{direct,externalFunds,shareholder}` | Muss/Kann gemischt | ✅ Wert füllbar |
| p5_5_aktiver_rap (§5 V) | `bs.ass.prepaidExpense` | Muss | ✅ Wert füllbar |

**Maßgeblichkeit/Überleitung (W2-B2, §5 I):** entweder Steuerbilanz direkt ODER Handelsbilanz +
`transferDiffTaxAccounts`-Überleitung. Unsere W2-Regeln = die STEUERliche Bewertungs-/Überleitungs-
seite → docken an den Überleitungs-Zweig an. Eigener Mapping-Block bei dev-1s Bau.

**Was der volle Extrakt (nach Download) liefern muss:**
1. **Muss-Feld-Katalog Kerntaxonomie** je Version (6.7 + 6.8) — aus Excel-Visualisierung
   (Mussfeld-Spalte) ODER Linkbase — Skala ~mehrere hundert Muss-Elemente (Scoping: minimale
   Steuerbilanz nutzt schon 215 distinkte Elemente).
2. **Muss ∩ W2** = die 6 Cluster oben, präzise auf die **6.7-/6.8-Element-Pfade** (nicht 6.9!)
   re-verifiziert — Feld-Umbenennungen/-Wanderungen zwischen 6.7→6.8→6.9 via Änderungsnachweis
   prüfen (Gültigkeits-Direktive auf Element-Ebene).
3. **Lücken-Liste**: Muss-Felder außerhalb der 6 Cluster, die wir NICHT aus Regeln füllen (GuV-
   Population `is.*`, Kontennachweise ab WJ25, Stammdaten außer GCD-Trivial) = Nicht-Gegenstand
   bzw. Fall-Input, wie im W2-Nenner vermerkt.

**Kosten:** Extrakt + Schnitt = LLM-frei, $0, Agent-Fanout möglich (analog E77). KEINE Cascade/
Formalisierer (Deklarations-Zuordnung, kein Regel-Bau).

---

## Teil D — Status + nächste Schritte
- **A (Version×WJ):** ✅ fertig, trianguliert. **6.7=WJ24, 6.8=WJ25** (nicht 6.9).
- **B (Beschaffung):** SPEC fertig; **Download blockiert auf Julius-direkt-im-Chat**.
- **C (Muss∩W2):** Rahmen + 6 Cluster fertig; **voller Katalog + 6.7/6.8-Pfad-Re-Verifikation
  blockiert auf Download** (Element-Pfade bislang nur an 6.9-Beispiel belegt).
- **D (kein Code):** eingehalten. Extraktor-Bau + `sources/ebilanz/`-Befüllung = dev-1 Paket 2
  nach Julius-Download.
- **Zone:** Instructor hat `sources/ebilanz/` mit dieser Zuweisung freigegeben (löst offene
  Zonen-Frage aus Scoping Z.110) — Befüllung aber erst nach Download.

**Benötigt von Julius (direkt im Chat):** Freigabe für Download der Taxonomie-Pakete 6.7 + 6.8
(GCD + Kerntaxonomie de-gaap-ci + Excel-Visualisierung) von esteuer.de nach `sources/ebilanz/`.
Danach rein lokal, $0.
