# GewStG Stufe-A-Landkarte (Paket 4, read-only, Instructor-Review)

**Read-only, $0, KEINE Freezes (macht Instructor), keine Cascade/Formalisierer, Registry
unberuehrt.** Landkarte der GewSt-Kern-Kette §§ 6→7→8→9→10a→11→14/16 + Andock an die
bestehenden Regeln + Gueltigkeits-Direktive (VZ/EZ 2024-2026) + Chargen-Schnitt-Vorschlag
mit Kosten + KSt-Nenner-Entscheidungsvorlage. Kein $-Lauf; $-Chargen erst mit Instructor-Cap-Wort.

**Fassung:** GewStG neugefasst 15.10.2002 (BGBl I S. 4167), zuletzt geaendert Art. 4 Gesetz
28.02.2025 (BGBl 2025 I Nr. 69). Aenderungsgesetze im Fenster: **Wachstumschancengesetz**
(27.03.2024, BGBl 2024 I Nr. 108), **JStG 2024** (02.12.2024, BGBl 2024 I Nr. 387).
Amtl. Handbuch GewStH 2024. Quellen-Triangulation BMF/gesetze-im-internet/Haufe/Baker Tilly/NWB.

---

## 1. Kern-Kette (Struktur + Rechenweg + Andock)

| § | Groesse | Rechenweg (Stufe-A) | Andock |
|---|---|---|---|
| **§ 6** | Besteuerungsgrundlage | = Gewerbeertrag | Rahmen |
| **§ 7 S. 1** | Gewerbeertrag | = nach EStG/KStG ermittelter **Gewinn aus Gewerbebetrieb** + Hinzurechnungen (§ 8) − Kuerzungen (§ 9) | **← §15/W2-Gewinn (bereits formalisiert)** |
| **§ 8 Nr. 1** | Hinzurechnungen | 25 % der Summe der Finanzierungsanteile (a–f), soweit > **200.000 € Freibetrag** | eigener Rechenkern |
| **§ 9** | Kuerzungen | Nr. 1 S. 1 Grundbesitz · Nr. 1 S. 3 erweiterte Kuerzung · Nr. 2/2a Schachtel | eigener Rechenkern |
| **§ 10a** | Gewerbeverlust | Sockel **1 Mio €** + **60 %** des uebersteigenden Gewerbeertrags; STATEFUL | **← p10d_2-Muster** |
| **§ 11** | Steuermessbetrag | (Gewerbeertrag auf volle 100 € abrunden − Freibetrag **24.500 €** [EU/PersG]) × **3,5 %** | **→ p35_1 (Messbetrag produktiv)** |
| **§ 14 / § 16** | Festsetzung / Hebesatz | Messbetrag × Hebesatz (Gemeinde) | **Hebesatz = Input-Slot, kein Bundes-Anker** |

### § 8 Nr. 1 Finanzierungsanteile (Ansaetze, Buchst. a–f)
- a Entgelte fuer Schulden (100 %), b Renten/dauernde Lasten, c Gewinnanteile stiller Gesellschafter,
  d Miete/Pacht bewegliche WG **20 %** (E-/Hybrid-Kfz 10 %), e unbewegliche WG **50 %**, f Rechte **25 %**.
- Summe a–f → **minus 200.000 € Freibetrag** → Rest × **25 %** (Reihenfolge: erst Freibetrag, dann Viertel).
- Negativ-Summe: Freibetrag NICHT spiegelbildlich (Rand-Nachtrag).

### § 9 Kuerzungen (VZ-relevant, s. Abschn. 2)
- **Nr. 1 S. 1 einfache Kuerzung Grundbesitz** — VZ-SPLIT (JStG 2024, s. u.).
- **Nr. 1 S. 3 ff. erweiterte Kuerzung** (Grundstuecksunternehmen) — Unschaedlichkeitsgrenze **20 %** (WtChancenG, ab EZ 2023).
- **Nr. 2** Gewinnanteile aus Mitunternehmerschaften; **Nr. 2a** Schachteldividenden (≥ 15 % Beteiligung).

---

## 2. Gueltigkeits-Direktive je § (VZ/EZ 2024–2026)

| § | 2024 (EZ) | 2025 | 2026 | Aenderungsgesetz | Charge-Konsequenz |
|---|---|---|---|---|---|
| § 7 S. 1 | stabil | stabil | stabil | — | einfassig |
| § 7 S. 8 (passive ausl. Eink.) | alt | **neugefasst** | neu | JStG 2024, ab EZ 2025 | Rand (DBA-nah, s. Nicht-Gegenstand) |
| § 8 Nr. 1 (200k / 25 %) | **stabil** | stabil | stabil | Hybrid-Verschaerfung im Vermittlungsausschuss GESTRICHEN | einfassig |
| **§ 9 Nr. 1 S. 1** | **1,2 % Einheitswert** | **tatsaechl. Grundsteuer (Betriebsausgabe)** | tatsaechl. Grundsteuer | **JStG 2024 Art. 9, § 36 Abs. 4b, ab EZ 2025** | **VZ-SPLIT: EZ 2024 ≠ EZ 2025** |
| § 9 Nr. 1 S. 3 b (20 %) | 20 % | 20 % | 20 % | WtChancenG, ab EZ 2023 (schon aktiv) | einfassig im Fenster |
| **§ 10a** (1 Mio + **60 %**) | **60 %** | 60 % | 60 % | **WtChancenG-70 % gilt NUR § 10d EStG/KStG; § 10a-GewStG-E GESTRICHEN** | einfassig; 60 ≠ 70 (kein EStG-Gleichlauf!) |
| § 11 (3,5 % / 24.500 €) | stabil | stabil | stabil | keine Aenderung (24.500 seit 2008) | einfassig |
| § 35 EStG (4,0×) | stabil | stabil | stabil | seit 2. Corona-StHG 2020 | Bestandsregel p35_1 |

**Zwei harte Gueltigkeits-Befunde (Stolpersteine):**
1. **§ 9 Nr. 1 S. 1 VZ-Split EZ 2024/EZ 2025** (JStG 2024): EZ 2024 = 1,2 % Einheitswert;
   EZ 2025+ = im EZ als Betriebsausgabe erfasste Grundsteuer. Zwei Fassungen im Fenster —
   analog zum NL-DBA-VZ-Split (Paket 3). Freeze je Fassung.
2. **§ 10a bleibt 60 %** — die WtChancenG-Anhebung auf 70 % (VZ 2024–2027) betrifft
   ausdruecklich NUR § 10d EStG + KStG, NICHT die Gewerbesteuer (§ 10a-GewStG-E im
   Vermittlungsausschuss gestrichen; Gesetzestext unveraendert 60 %). **Kein EStG-Gleichlauf:
   p10a darf NICHT den 70-%-Wert von p10d_2 erben.**

---

## 3. Andock-Analyse (konkret, Bestandsregeln)

- **§ 7 ← §15/W2-Gewinn:** Der Gewinn aus Gewerbebetrieb (§ 15 EStG) ist bereits formalisiert
  (einkunftsart_gewerbe_p15_abs2 Charge 18, p15_1_2_mitunternehmer_einkuenfte; steuerbil. Gewinn
  aus p4_1/p6_*/W2). § 7 GewStG dockt hier an (Gewinn ± §§ 8/9). Kein Neubau der Gewinnermittlung.
- **§ 11 → p35_1 (Messbetrag PRODUKTIV):** p35_1_gewst_anrechnung (§ 35, 4,0× Messbetrag, drei
  Deckel) nimmt `gewerbesteuer_messbetrag` HEUTE als **Sachverhalt-Input** ("GewStG Nicht-
  Gegenstand"). Die GewStG-Kette macht diesen Input produktiv — § 11 Steuermessbetrag speist
  p35_1. Verdrahtung LLM-frei (Runner-/Integrations-Ebene, analog _vorsorge_hb/_kindergeld);
  Bestandsregel p35_1 bleibt strukturell unberuehrt.
- **§ 10a ← p10d_2-Muster (STATE-FRAGE, wie § 10d):** p10d_2_verlustvortrag_abzug ist bereits
  als **Ein-VZ-Regel** gebaut: `verlustabzug = min(bestand, sockel + %·max(0, GdE − sockel))`,
  **`bestand` = Input/State, Mehrjahres-Fortschreibung (Abs. 4) BEWUSST ausserhalb** (Backlog-
  Grenze, im hinweis fixiert). **§ 10a spiegelt das exakt:** `gewerbeverlust_abzug = min(fehlbetrag_
  bestand, 1.000.000 + 0,60·max(0, gewerbeertrag − 1.000.000))`, `fehlbetrag_bestand` = Input/State,
  EZ-Fortschreibung ausserhalb. **Unterschiede zu p10d_2:** (a) **60 % statt 70 %**; (b) **kein
  Zusammenveranlagungs-Sockel** (2 Mio) — GewSt ist betriebsbezogen, immer 1-Mio-Sockel.

**STATE-Antwort:** Wie bei § 10d wird KEINE Mehrjahres-State-Maschine gebaut. `fehlbetrag_bestand`
ist Sachverhalt/State-Input; die Fortschreibung ueber Erhebungszeitraeume bleibt Backlog-Grenze
(identische Architektur-Linie wie p10d_2 / p20_6). Kein neues State-Konzept noetig.

---

## 4. Chargen-Schnitt-Vorschlag (mit Kosten je Charge)

Kalibrierung Instructor: 1-quellig ~$0,07, multi-quellig ~$0,15.

| Charge | Inhalt | Quellen | Kosten | Anmerkung |
|---|---|---|---|---|
| **G1** | § 7 S. 1 + § 6 Gewerbeertrag-Geruest (Gewinn ± §§ 8/9) | § 7, § 6 GewStG | ~$0,07 | Andock §15/W2; Slot-Regel |
| **G2** | § 8 Nr. 1 Hinzurechnungen (a–f, 200k, 25 %) | § 8 GewStG | ~$0,10 | reich (6 Quoten), Klasse-5-Anteile |
| **G3a** | § 9 Nr. 1 S. 1 einfache Kuerzung — **EZ 2024** (1,2 % Einheitswert) | § 9 GewStG | ~$0,07 | VZ-Split Teil 1 |
| **G3b** | § 9 Nr. 1 S. 1 — **EZ 2025+** (tatsaechl. Grundsteuer) + Nr. 1 S. 3 (20 %) + Nr. 2/2a | § 9 + JStG-2024-Freeze + § 36 | ~$0,15 | VZ-Split Teil 2 + erweiterte Kuerzung/Schachtel |
| **G4** | § 10a Gewerbeverlust (1 Mio + 60 %, STATEFUL, p10d_2-Muster) | § 10a GewStG | ~$0,07 | 60 ≠ 70; bestand=Input |
| **G5** | § 11 Steuermesszahl 3,5 % + Freibetrag 24.500 → Messbetrag; **p35_1-Verdrahtung** | § 11 GewStG | ~$0,07 | Messbetrag produktiv; Verdrahtung LLM-frei |
| — | § 14/§ 16 Hebesatz | — | $0 | Input-Slot, kein Anker/keine Charge |

**Summe geschaetzt ~$0,53** (6 Chargen). Schlanker Schnitt (G3a+G3b und G1+G5 je zusammen)
druecke auf ~$0,40, verliert aber die VZ-Split-Sauberkeit bei § 9. **Empfehlung: 6-Charge-Schnitt**
(VZ-Split explizit getrennt, wie bei Multi-VZ etabliert).

---

## 5. Entscheidungsvorlage: KSt-Nenner (nur GewSt-Kette vs. voller KSt-Pfad)

§ 7 GewStG knuepft an den **nach EStG ODER KStG** ermittelten Gewinn an. Zwei Nenner moeglich:

- **A) NUR GewSt-Kette auf EStG-Nenner (natuerliche Person/Einzelunt./PersG) — EMPFOHLEN.**
  Deckt sich mit dem bisherigen W2-/E-Bilanz-Nenner (Paket 2) und mit p35_1 (§ 35 Anrechnung gilt
  NUR fuer natuerliche Personen). Der Gewinn kommt aus §15/W2 (bereits da). GewStG-Kette baut
  Gewerbeertrag→Messbetrag, verdrahtet in p35_1. **Kein KStG noetig.** Geschlossener, produktiver Nutzen.
- **B) Zusaetzlich KSt-Gewinn-Pfad (Kapitalgesellschaft-Nenner).** Erfordert den ganzen KStG-Apparat
  (§ 8 KStG, verdeckte Gewinnausschuettung, § 8b Beteiligungsertrag, Zinsschranke § 4h EStG/§ 8a
  KStG …) — eine EIGENE Statute, deutlich groesser, und § 35 EStG greift dort gar nicht (KSt kennt
  keine GewSt-Anrechnung). → **eigenes kuenftiges Paket, nicht in Paket 4.**

**Empfehlung: A** — GewSt-Kette auf dem bestehenden EStG-/§15-/W2-Nenner, KStG-Pfad zurueckstellen.
Damit ist Paket 4 in sich geschlossen und macht p35_1 produktiv.

---

## 6. Nicht-Gegenstand / Nachtraege
- § 14/§ 16 Hebesatz = Gemeinde-Input (kein Bundes-Anker), reiner Sachverhalt-Slot.
- § 7 S. 8 (passive ausl. Betriebsstaetten-Eink., JStG 2024 ab EZ 2025), § 29 Zerlegung
  (Energiespeicher, JStG 2024) — ausserhalb W2-Kern, Randbereich.
- § 8 Nr. 1 Negativ-Summe (Freibetrag nicht spiegelbildlich) — Rand-Nachtrag.
- § 9 Nr. 1 S. 3 erweiterte Kuerzung: Vollkatalog der Unschaedlichkeitsgrenzen — Nachtrag.
- KStG-Gewinn-Pfad (Nenner B) — eigenes Paket.

## 7. Offene Punkte fuer Instructor-Freeze
- § 9 Nr. 1 S. 1: ZWEI Freeze-Fassungen (EZ 2024 Einheitswert / EZ 2025+ tatsaechl. Grundsteuer, JStG 2024).
- § 8/§ 10a/§ 11: je eine Fassung 2024-2026 (stabil). § 10a-Anker: 60-%-Wortlaut explizit gegen
  gesetze-im-internet pinnen (NICHT den 70-%-§-10d-Wert uebernehmen).
- Exakte Buchst.-Quoten § 8 Nr. 1 d/e/f + § 121a BewG (140-%-Faktor Einheitswert) am gefreezten Text pinnen.
