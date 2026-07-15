# Recherche-Dossier: Kfz-BLP-E-Grenzen-Historie (§ 6 Abs. 1 Nr. 4 EStG) — Fundstellen

taxgraph-dev-2, 2026-07-15. Instructor-Auftrag: NUR Fundstellen, KEIN Freeze (Freeze macht
Instructor). Betrifft die Viertel-Regelung (0,25 %-Ansatz) für reine Elektro-/Brennstoffzellen-
Kfz: gilt nur bis zur Bruttolistenpreis-Höchstgrenze; darüber greift die Halb-Regelung
(0,5 %). Normen: **§ 6 Abs. 1 Nr. 4 Satz 2 Nr. 3 EStG** (1-%-/BLP-Methode) + **Satz 3 Nr. 3**
(Fahrtenbuch-Methode), Anwendungsvorschrift **§ 52 Abs. 12 EStG**. Über § 8 Abs. 2 EStG
entsprechend bei AN-Überlassung.

## Schwellen-Historie (Anschaffungszeitpunkt-Kohorten)

| BLP-Grenze | Anschaffung im Zeitfenster | Änderungsgesetz | BGBl-Fundstelle | Anwendungsvorschrift |
|---|---|---|---|---|
| 40.000 € | ab 1.1.2019 (bis 31.12.2019) | G. z. weiteren steuerl. Förderung der Elektromobilität ("JStG 2019"-Begleit) | BGBl. 2019 I S. 2451 (v. 12.12.2019) | § 52 Abs. 12 EStG |
| 60.000 € | nach 31.12.2019 (bis 31.12.2023) | **Zweites Corona-Steuerhilfegesetz** | BGBl. 2020 I S. 1512 (v. 29.6.2020) | § 52 Abs. 12 EStG |
| **70.000 €** | **nach 31.12.2023 (bis 30.6.2025)** | **Wachstumschancengesetz**, Art. 3 | **BGBl. 2024 I Nr. 108 (v. 27.3.2024)** | § 52 Abs. 12 EStG (erstmals Kfz angeschafft nach 31.12.2023) |
| **100.000 €** | **nach 30.6.2025 (bis vor 1.1.2031)** | **Gesetz f. ein steuerl. Investitionssofortprogramm (StInvSofortPG)** | **BGBl. 2025 I Nr. 161 (ausgegeben 18.7.2025, in Kraft 19.7.2025)** | § 52 Abs. 12 EStG (erstmals Kfz angeschafft nach 30.6.2025) |

## Die zwei vom Instructor angefragten Schwellen — Detail

### 60.000 → 70.000 €
- **Gesetz:** Wachstumschancengesetz, Artikel 3 (Änderung EStG).
- **BGBl:** BGBl. 2024 I Nr. 108, Gesetz v. 27. März 2024.
- **Zeitfenster:** gilt für Kraftfahrzeuge, die **nach dem 31. Dezember 2023** angeschafft
  werden (§ 52 Abs. 12 EStG; d. h. praktisch ab VZ 2024).
- Wortlaut-Kern § 6 Abs. 1 Nr. 4 S. 2 Nr. 3: „… und dessen Bruttolistenpreis … nicht mehr
  als 70 000 Euro beträgt …" (+ gleichlautend S. 3 Nr. 3 Fahrtenbuch).

### 70.000 → 100.000 €
- **Gesetz:** Gesetz für ein steuerliches Investitionssofortprogramm zur Stärkung des
  Wirtschaftsstandorts Deutschland (StInvSofortPG).
- **BGBl:** BGBl. 2025 I Nr. 161, ausgegeben 18. Juli 2025 (in Kraft 19.7.2025).
  Verfahren: BR-Drs. 233/25 + 281/25; DIP-Vorgang 322244.
- **Zeitfenster:** gilt für Kraftfahrzeuge, die **nach dem 30. Juni 2025** (bis vor
  1.1.2031) angeschafft werden; Altfälle (Anschaffung bis 30.6.2025) behalten 70.000 €.
- ⚠ **Beim Freeze zu klären (Quellen uneinheitlich):** exakte Satznummer der Anwendungs-
  vorschrift in § 52 Abs. 12 EStG (Satz 4/5/6 je nach Kommentar) — literarisch am amtlichen
  Regelungstext (recht.bund.de/bgbl/1/2025/161) zu verifizieren, nicht aus Sekundärquellen.

## Einordnung für unser System (Gültigkeits-Direktive)
- Die BLP-Grenze ist eine **Anschaffungszeitpunkt-Kohorte**, KEINE VZ-Schwelle — der maßgebende
  Höchstbetrag richtet sich nach dem Anschaffungsdatum des Kfz, nicht nach dem VZ der Nutzung
  (analog Sonder-AfA-§7g-Kohorten, C27). Ein Kfz mit BLP 65.000 €, angeschafft 2022, bleibt
  bei 60.000-Grenze (Halb-Regelung) auch in VZ 2025; ein 2024er-Kfz nutzt 70.000.
- Bezug Bestandsregel: `p6_1_4_kfz_nutzungswert` (§ 6 Abs. 1 Nr. 4 S. 2, Kfz-Privatnutzung
  1 %, E-Bruchteile) — trägt die Viertel-/Halb-Mechanik. Die BLP-Grenze als Kohorten-param
  (Anschaffungsjahr → maßgebliche Grenze) wäre der M4/C28-Andock, analog degressive-AfA-Fenster.
- ⚠ **Nicht verwechseln:** die BLP-Grenzen-Fenster (nach 31.12.2023 / nach 30.6.2025) sind
  NICHT deckungsgleich mit den degressive-AfA-Fenstern der M2-Landkarte (WachstumschancenG
  1.4.–31.12.2024; Investitionsbooster 1.7.2025–31.12.2027). Getrennte Zeitachsen je Norm.

## Quellen
- recht.bund.de/bgbl/1/2025/161 (amtl. Regelungstext StInvSofortPG) — für Freeze.
- Haufe: „Investitionssofortprogramm / Anhebung BLP-Grenze § 6 Abs. 1 Nr. 4"; „Steuerliche
  Förderung Elektro-/Hybridfahrzeuge".
- KPMG „StInvSofortPG-BGBl"; lohnsteuer-kompakt; auren.com — Sekundär, Fundstellen-Bestätigung.
