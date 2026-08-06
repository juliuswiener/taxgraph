# Anlage Kind — Bauplan für drei neue Abzugstatbestände

**Stand:** 2026-08-06, HEAD d9107c9, Suite 1488 passed/4 skipped
**Basis:** `reports/adjudikation/anlage_kind_inventur_2026-08-06.md`
**Status:** Phase 1 (Bauplan, kein Produktivcode)

---

## VORBEMERKUNG: §33 Abs.2a S.6/7 — Fahrtkostenpauschale in der §33-Kette

**Korrektur auf Julius' Anweisung.** Der Bauplan V1 ordnete die Fahrtkostenpauschale (§33 Abs.2a) als Summand zu `ausserg` (p33b-Pauschbeträge) ein. Das ist falsch.

§33 Abs.2a S.6/7, `sources/gesetze-im-internet/estg_p33_2026-07-11.txt` Z.10:

> S.6: "Über die Fahrtkostenpauschale nach Satz 1 hinaus sind keine weiteren behinderungsbedingten Fahrtkosten als außergewöhnliche Belastung nach Absatz 1 berücksichtigungsfähig."
> S.7: "Die Pauschale ist bei der Ermittlung des Teils der Aufwendungen im Sinne des Absatzes 1, der die zumutbare Belastung übersteigt, einzubeziehen."

S.6: die Pauschale ERSETZT individuelle behinderungsbedingte Fahrtkosten (kein Add-on).
S.7: die Pauschale ist Teil der agB-Aufwendungen, die um die zumutbare Belastung gekürzt werden.

**Konsequenz:** Die Fahrtkostenpauschale geht in den `aussergewoehnliche_belastungen`-Input von `catala_p33_agb` (runner.py:484-492), nicht in `ausserg` (die p33b-Pauschbeträge). Der bestehende Ring-Call in `api.py` Z.807-810:

```python
g["aussergewoehnliche_belastungen"] = ausserg + runner.catala_p33_agb({
    "aussergewoehnliche_belastungen": _c("agb_aufwendungen") // 100,
    ...})
```

wird zu:

```python
g["aussergewoehnliche_belastungen"] = ausserg + runner.catala_p33_agb({
    "aussergewoehnliche_belastungen": (_c("agb_aufwendungen") + fahrtkostenpauschale) // 100,
    ...})
```

Die §33-Abs.2a-Kette (900/4.500 €) wird in einem neuen Accessor `catala_p33_2a_fahrtkosten()` abgebildet, der als Input die Kind-GdB-Merkmale nimmt und die Pauschale (0/900/4.500) zurückgibt.

**Aufwands-Korrektur:** +1 h für den Accessor + Ring-Korrektur. B3 neu: 6–8 h.

---

## B1: Schulgeld (§ 10 Abs. 1 Nr. 9 EStG) — 30 %, max. 5.000 €

### 1. Norm-Beleg

**Quelle:** `sources/gesetze-im-internet/estg_p10_2026-07-11.txt`, Zeile 10 (§ 10 Abs. 1 Nr. 9 S. 1):

> "30 Prozent des Entgelts, höchstens 5 000 Euro, das der Steuerpflichtige für ein Kind, für das er Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld hat, für dessen Besuch einer Schule in freier Trägerschaft oder einer überwiegend privat finanzierten Schule entrichtet, mit Ausnahme des Entgelts für Beherbergung, Betreuung und Verpflegung."

Satz 2 (Schule in EU/EWR mit anerkanntem oder gleichwertigem Abschluss), Satz 3 (Vorbereitung auf solchen Abschluss), Satz 4 (Deutsche Schule im Ausland). Satz 5:

> "Der Höchstbetrag nach Satz 1 wird für jedes Kind, bei dem die Voraussetzungen vorliegen, je Elternpaar nur einmal gewährt."

**Gültigkeit VZ 2024/2025/2026:** `estg_p10_2026-07-11.meta.yaml`: geltende Fassung 2026. Seit JStG 2009 (BGBl. I 2008, 2850) unverändert. Keine Änderung VZ 2024–2026 bekannt.

**Beträge/Prozentsätze:**
| Größe | Wert | Fundstelle |
|---|---|---|
| Abzugssatz | 30 % | § 10 Abs. 1 Nr. 9 S. 1, estg_p10_2026-07-11.txt Z. 10 |
| Höchstbetrag | 5.000 € | § 10 Abs. 1 Nr. 9 S. 1, estg_p10_2026-07-11.txt Z. 10 |
| Höchstbetrag | 5.000 € je Kind, je Elternpaar | § 10 Abs. 1 Nr. 9 S. 5, estg_p10_2026-07-11.txt Z. 10 |

### 2. Vorlage: Kinderbetreuungskosten (§ 10 Abs. 1 Nr. 5)

Die strukturell identische Norm. Vollständige Kette:

| Schritt | Vorlage (kinderbetreuungskosten) | Datei |
|---|---|---|
| Item-Registry | `p10_1_5_kinderbetreuung.yaml` | `pipeline/item_registry/p10_1_5_kinderbetreuung.yaml` |
| Parameter | `abzugssatz=0.8`, `hoechstbetrag_je_kind=4800` | `params/*/kinderbetreuung_p10.yaml` |
| Accessor | `catala_p10_1_5_kinderbetreuung()` | `golden/runner.py:1062-1077` |
| Ring-Call (gesamt) | `api.py:767-768` | `produkt/haut/api.py:767-768` |
| Ring-Call (rentner) | `api.py:1341-1342` | `produkt/haut/api.py:1341-1342` |
| Bindung | `kinderbetreuungskosten` → E0506105 | `produkt/bindung/bindung_p10_1_5_gesamt.yaml` |
| Bindung Multi | `kinderbetreuung_anzahl_kinder` (kein Kz) | selbe YAML, Z.33-49 |
| Unit-Test | `test_p10_1_5_accessor.py` | `tests/test_p10_1_5_accessor.py` |
| Ring-Test | `test_p10_1_5_ring.py` | `tests/test_p10_1_5_ring.py` |

Der Accessor ist pure-Python (kein Catala-Modul). Die Multi-Kind-Komposition verwendet `anzahl_kinder` als Multiplikator und verteilt die Summe gleichmäßig (`aufwand_pro_kind = aufw // anzahl`).

**Offene Stelle der Vorlage (runner.py:1069-1073):** Der Autor markiert die Gleichverteilungs-Annahme selbst als ungeklärt: "Falls 'aufwendungen' pro Kind gemeint ist, müsste das Wiring das regeln." B1 erbt diese Annahme — für MVP akzeptabel, da die ELSTER-Kz pro Kind ohnehin eine separate Summe erwarten.

### 3. Delta (was muss neu entstehen)

**B1 — Schulgeld:**

| Was | Name | Typ | Kz | Anmerkung |
|---|---|---|---|---|
| Item-Registry | `pipeline/item_registry/p10_1_9_schulgeld.yaml` | — | — | analog p10_1_5_kinderbetreuung.yaml |
| Parameter | `params/*/schulgeld_p10.yaml` | yaml | — | abzugssatz=0.3, hoechstbetrag=5000 |
| Accessor | `catala_p10_1_9_schulgeld()` | pure-Python | — | in `golden/runner.py`, analog Z.1062-1077 |
| Ring-Call gesamt | `api.py` Z. 767-768 erweitern | += | — | nach `catala_p10_1_5_kinderbetreuung` |
| Ring-Call rentner | `api.py` Z. 1341-1342 erweitern | += | — | nach `catala_p10_1_5_kinderbetreuung` |
| Bindung (Summe) | `kind_schulgeld` | cent | E0505607 | `instanz_gruppe: kind` |
| Bindung (Einzelb.) | `kind_schulgeld_einzel` | cent | E0504405 | `instanz_gruppe: kind` |
| Bindung (Schulname) | `kind_schulname` | text | E0505606 | `instanz_gruppe: kind` |
| Bindung (Eltern-Anteil) | `kind_schulgeld_eltern_anteil` | cent | E0504505 | `instanz_gruppe: kind` |
| Bindung (Prozentsatz) | `kind_schulgeld_aufteilung_prozent` | int(100) | E0504603 | `instanz_gruppe: kind` |
| Unit-Test | `tests/test_p10_1_9_schulgeld_accessor.py` | — | — | 5-6 Seeds |
| Ring-Test | `tests/test_p10_1_9_schulgeld_ring.py` | — | — | Differential + Erreichbarkeit |

**Vereinfachung (MVP):** Der Ring-Rechnung reicht ein globales Feld `kind_schulgeld` (cent, Summe aller Kinder) + `kind_schulgeld_anzahl_kinder` (int). Accessor teilt Summe durch Anzahl, deckelt pro Kind bei 5.000 € → 30 % = 1.500 €. **Exakt das Muster von kinderbetreuungskosten.**

**Aufwand: ~4–5 h** (1 h Accessor + Parameter + Registry, 1 h Bindung + Ring-Call (per-Kind mit EM.instanzen), 1,5 h Tests inkl. Differential mit 2 Kindern + XML, 1,5 h wegen Option 2 statt Option 1).

### 4. Haken

**"30 % des Entgelts, höchstens 5 000 Euro" — je Kind oder je Elternpaar?**
Satz 5: "je Elternpaar nur einmal." Der Höchstbetrag ist **5.000 € pro Kind, aber je Elternpaar.** Bei Einzelveranlagung eines Elternteils: der Höchstbetrag ist hälftig zu teilen (2.500 € pro Elternteil × Kind). Die hM bestätigt das.

**Korrektur (Bauplan V1 → V2):** Mein MVP-Ansatz "5.000 € × Kind, unabhängig von der Veranlagung" ist **under-tax** (zu hoher Höchstbetrag macht den Abzug größer). Der Accessor muss bei Einzelveranlagung 2.500 € deckeln, bei Zusammenveranlagung 5.000 €. Oder MVP: den Höchstbetrag auf 2.500 € pro Kind setzen (over-tax-safe bei Zusammenveranlagung, weil der Abzug kleiner sein könnte), und bei Zusammenveranlagung verdoppeln. **Empfehlung: MVP mit 2.500 € Deckel + Verdopplung bei Zusammenveranlagung (wie § 10 Abs. 4).**

**E0504603 (Aufteilungs-Prozentsatz zwischen Eltern):**
ELSTER-Detail für die Aufteilung zwischen getrennt veranlagten Eltern. Für MVP: der Nutzer gibt seinen Anteil direkt an (Feld `kind_schulgeld_eltern_anteil`).

**"Mit Ausnahme des Entgelts für Beherbergung, Betreuung und Verpflegung":**
Keine Nutzerfrage — der Nutzer muss den Betrag selbst bereinigen. Im Fragetext dokumentieren.

**Schulgeld für mehrere Kinder:**
Pro-Kind-Deckel 5.000 €. MVP-Ansatz: globaler Summen-Feld + Anzahl-Multiplikator, Verteilung gleichmäßig. Bei ungleichen Schulgeldern over-tax-safe (der geringere Abzug wird geltend gemacht).

---

## B2: KV/PV-Beiträge des Kindes (§ 10 Abs. 1 Nr. 3 S. 2 EStG) — 12 Kz

### 1. Norm-Beleg

**Quelle:** `sources/gesetze-im-internet/estg_p10_2026-07-11.txt`, Zeile 10 (§ 10 Abs. 1 Nr. 3 S. 2):

> "Als eigene Beiträge des Steuerpflichtigen können auch eigene Beiträge im Sinne der Buchstaben a oder b eines Kindes behandelt werden, wenn der Steuerpflichtige die Beiträge des Kindes, für das ein Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld besteht, durch Leistungen in Form von Bar- oder Sachunterhalt wirtschaftlich getragen hat, unabhängig von Einkünften oder Bezügen des Kindes; Voraussetzung für die Berücksichtigung beim Steuerpflichtigen ist die Angabe der erteilten Identifikationsnummer (§ 139b der Abgabenordnung) des Kindes in der Einkommensteuererklärung des Steuerpflichtigen."

**Gültigkeit VZ 2024/2025/2026:** Gleiche Fassung wie § 10 insgesamt. Seit Einführung strukturell unverändert, nur Verweise auf § 32 Abs. 6 angepasst.

**Beträge/Prozentsätze:**
Keine eigenen Beträge — Kind-Beiträge werden "als eigene Beiträge" behandelt, unterliegen § 10 Abs. 4-Höchstbetrag (1.900/2.800 €).

### 2. Vorlage: bestehende KV/PV-Rechnung + Kinderbetreuung (Mischung)

Die KV/PV-Kind-Beiträge sind eine Erweiterung der *bestehenden* `catala_p10_kv_pv`-Accessor-Struktur. Sie werden als "eigene Beiträge" des Steuerpflichtigen in denselben § 10 Abs. 4-Höchstbetrag (1.900/2.800 €) eingerechnet.

Die Vorlage für die per-Kind-Felder ist `kind_idnr` (instanz_gruppe: kind) in `bindung_kap_vv_familie.yaml`.

### 3. Delta

**Was muss neu entstehen:**

| Was | Name | Typ | Kz | Anmerkung |
|---|---|---|---|---|
| Item-Registry | `pipeline/item_registry/p10_1_3_3a_kv_pv_kind.yaml` | — | — | neu |
| Bindung (KV) | `kind_kv_basis` | cent | E0503110 | `instanz_gruppe: kind` |
| Bindung (PV) | `kind_pv_basis` | cent | E0503310 | `instanz_gruppe: kind` |
| Bindung (KV-Erstattung) | `kind_kv_erstattung` | cent | E0503409 | `instanz_gruppe: kind` |
| Bindung (KV über Basis) | `kind_kv_ueber_basis` | cent | E0503609 | `instanz_gruppe: kind` |
| Bindung (KV-AW_Kind) | `kind_kv_kind_selbst` | cent | E0503111 | `instanz_gruppe: kind` |
| Bindung (KV-mit-KG) | `kind_kv_kind_kg` | cent | E0503209 | `instanz_gruppe: kind` |
| Bindung (PV-AW_Kind) | `kind_pv_kind_selbst` | cent | E0503311 | `instanz_gruppe: kind` |
| Bindung (Erstattung AW_Kind) | `kind_kv_erstattung_kind` | cent | E0503410 | `instanz_gruppe: kind` |
| Bindung (Erstattung KG) | `kind_kv_erstattung_kg` | cent | E0503509 | `instanz_gruppe: kind` |
| Bindung (Zuschuss) | `kind_kv_zuschuss` | cent | E0503610 | `instanz_gruppe: kind` |
| Bindung (ausl KV) | `kind_kv_ausl` | cent | E0503822 | `instanz_gruppe: kind` |
| Bindung (ausl KG) | `kind_kv_ausl_kg` | cent | E0503823 | `instanz_gruppe: kind` |
| Ring-Erweiterung | `api.py` Z. 763-765 | +Kind-Beiträge | — | Summe per-Kind-KV zu `basis_kv_pv` addieren |
| Test | `tests/test_p10_kv_pv_kind.py` | — | — | Ring-Differential |

**Vereinfachung (MVP):** Statt 12 Einzelfelder pro Kind: **ein globales Feld** `kind_kv_pv_beitraege` (cent, Summe aller Kinder). Der Accessor addiert das zur `basis_kv_pv`. Die 12 per-Kind-Kz sind für die ELSTER-Deklaration — können in Stufe 2 kommen.

**Aufwand: ~4–5 h** (2 h 12 Kz binden + Registry, 1 h Ring-Erweiterung, 1,5 h Tests, 0,5 h IdNr-Gate).

### 4. Haken

**§ 10 Abs. 4-Deckel greift: JA.** Kind-Beiträge als "eigene Beiträge" → gemeinsamer HB 1.900/2.800 €. Die bestehende `catala_p10_kv_pv` rechnet korrekt — Kind-Beiträge zur `basis_kv_pv` addieren.

**IdNr des Kindes als Voraussetzung: JA.** § 10 Abs. 1 Nr. 3 S. 2: "Voraussetzung ... ist die Angabe der erteilten Identifikationsnummer des Kindes." `kind_idnr` (E0500406) mit `instanz_gruppe: kind` **trägt**. Der Ring muss prüfen, ob `kind_idnr` für mindestens ein Kind befüllt ist, bevor der KV/PV-Kind-Abzug gewährt wird.

**AW_Stpfl vs AW_Kind:** XSD unterscheidet zwei Zahlwege (Elternteil zahlt vs Kind selbst versichert). Gesetz sagt "wirtschaftlich getragen" — beides fällt darunter. MVP: ein globales Feld.

---

## B3: Behinderten-Pauschbetrag des Kindes, Übertragung auf Elternteil (§ 33b Abs. 5 EStG) — 11 Kz + Fahrtkostenpauschale (§ 33 Abs. 2a) — 3 Kz

### 1. Norm-Beleg

**§ 33b Abs. 5:** `sources/gesetze-im-internet/estg_p33b_2026-07-13.txt`, Z. 10:

> "(5) 1 Steht der Behinderten-Pauschbetrag oder der Hinterbliebenen-Pauschbetrag einem Kind zu, für das der Steuerpflichtige Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld hat, so wird der Pauschbetrag auf Antrag auf den Steuerpflichtigen übertragen, wenn ihn das Kind nicht in Anspruch nimmt. 2 Dabei ist der Pauschbetrag grundsätzlich auf beide Elternteile je zur Hälfte aufzuteilen, es sei denn, der Kinderfreibetrag wurde auf den anderen Elternteil übertragen. 3 Auf gemeinsamen Antrag der Eltern ist eine andere Aufteilung möglich. 4 In diesen Fällen besteht für Aufwendungen, für die der Behinderten-Pauschbetrag gilt, kein Anspruch auf eine Steuerermäßigung nach § 33. 5 Voraussetzung für die Übertragung nach Satz 1 ist die Angabe der erteilten Identifikationsnummer (§ 139b der Abgabenordnung) des Kindes in der Einkommensteuererklärung des Steuerpflichtigen."

**§ 33 Abs. 2a (Fahrtkostenpauschale):** `sources/gesetze-im-internet/estg_p33_2026-07-11.txt`, Z. 10:

> S.1: "Abweichend von Absatz 1 wird für Aufwendungen für durch eine Behinderung veranlasste Fahrten nur eine Pauschale gewährt (behinderungsbedingte Fahrtkostenpauschale)."
> S.2: "Die Pauschale erhalten: 1. Menschen mit einem Grad der Behinderung von mindestens 80 oder mit einem Grad der Behinderung von mindestens 70 und dem Merkzeichen 'G', 2. Menschen mit dem Merkzeichen 'aG', mit dem Merkzeichen 'Bl', mit dem Merkzeichen 'TBl' oder mit dem Merkzeichen 'H'."
> S.3: "Bei Erfüllung der Anspruchsvoraussetzungen nach Satz 2 Nummer 1 beträgt die Pauschale 900 Euro."
> S.4: "Bei Erfüllung der Anspruchsvoraussetzungen nach Satz 2 Nummer 2 beträgt die Pauschale 4 500 Euro."
> S.6: "Über die Fahrtkostenpauschale nach Satz 1 hinaus sind keine weiteren behinderungsbedingten Fahrtkosten als außergewöhnliche Belastung nach Absatz 1 berücksichtigungsfähig."
> S.7: "Die Pauschale ist bei der Ermittlung des Teils der Aufwendungen im Sinne des Absatzes 1, der die zumutbare Belastung übersteigt, einzubeziehen."
> S.8: "Sie kann auch gewährt werden, wenn ein Behinderten-Pauschbetrag nach § 33b Absatz 5 übertragen wurde."
> S.9: "§ 33b Absatz 5 ist entsprechend anzuwenden."

**Gültigkeit VZ 2024/2025/2026:** § 33b EStG geltende Fassung 2026. 2021er-Reform (verdoppelte GdB-Staffel) seit VZ 2021 unverändert. § 33 Abs. 2a eingeführt durch JStG 2020 (BGBl. I 2020, 3096), unverändert seit VZ 2021.

**Beträge:**
| Größe | Wert | Fundstelle |
|---|---|---|
| Behinderten-PB (GdB 20–100) | 384–2.840 € | § 33b Abs. 3 S. 2, estg_p33b_2026-07-13.txt Z. 11 |
| Behinderten-PB (blind/hilflos) | 7.400 € | § 33b Abs. 3 S. 3, estg_p33b_2026-07-13.txt Z. 12 |
| Hinterbliebenen-PB | 370 € | § 33b Abs. 4, estg_p33b_2026-07-13.txt Z. 12 |
| Pflege-PB (PG2/3/4/5) | 600/1.100/1.800/1.800 € | § 33b Abs. 6 S. 3, estg_p33b_2026-07-13.txt Z. 12 |
| Fahrtkostenpauschale (GdB≥80/70+G) | 900 € | § 33 Abs. 2a S. 3, estg_p33_2026-07-11.txt Z. 10 |
| Fahrtkostenpauschale (aG/Bl/TBl/H) | 4.500 € | § 33 Abs. 2a S. 4, estg_p33_2026-07-11.txt Z. 10 |

### 2. Vorlage: bestehende p33b-Accessors + kind_idnr-Instanz

Die Infrastruktur für die Pauschbetrag-Berechnung EXISTIERT bereits:
- `catala_behinderten_pb()` (`golden/runner.py:1003`)
- `catala_pflege_pb()` (`golden/runner.py:1016`)
- `catala_hinterbliebenen_pb()` (`golden/runner.py:1026`)
- Parameter `params/*/behinderten_pauschbetrag_p33b.yaml`
- Item-Registries: `p33b_behinderten_pauschbetrag.yaml`, `p33b_pflege_pauschbetrag.yaml`, `p33b_hinterbliebenen_pauschbetrag.yaml`

Die per-Kind-Felder folgen dem `kind_idnr`-Muster (`instanz_gruppe: kind` in `bindung_kap_vv_familie.yaml`).

### 3. Delta

**Was muss neu entstehen:**

| Was | Name | Typ | Kz | Anmerkung |
|---|---|---|---|---|
| Item-Registry | `pipeline/item_registry/p33b_5_kind_uebertragung.yaml` | — | — | neu |
| Bindung (GdB Kind) | `kind_grad_der_behinderung` | int | E0505809 | `instanz_gruppe: kind` |
| Bindung (GdB von) | `kind_gdb_gueltig_von` | text | E0504601 | `instanz_gruppe: kind` |
| Bindung (GdB bis) | `kind_gdb_gueltig_bis` | text | E0504602 | `instanz_gruppe: kind` |
| Bindung (GdB unbefristet) | `kind_gdb_unbefristet` | bool | E0505908 | `instanz_gruppe: kind` |
| Bindung (Merkzeichen G/aG) | `kind_gehbehindert` | bool | E0505808 | `instanz_gruppe: kind` |
| Bindung (blind/hilflos) | `kind_blind_hilflos` | bool | E0505807 | `instanz_gruppe: kind` |
| Bindung (Antrag Hbl-Übertragung) | `kind_hbl_uebertragung` | bool | E0505805 | `instanz_gruppe: kind` |
| Bindung (Aufteilungs-% Beh) | `kind_pb_aufteilung_prozent` | int(100) | E0506007 | `instanz_gruppe: kind` |
| Bindung (Fk-Pausch: GdB≥80/70+G) | `kind_fk_pausch_gdb` | bool | E0507302 | `instanz_gruppe: kind` |
| Bindung (Fk-Pausch: aG/Bl/TBl/H) | `kind_fk_pausch_schwer` | bool | E0507403 | `instanz_gruppe: kind` |
| Bindung (Fk-Aufteilungs-%) | `kind_fk_aufteilung_prozent` | int(100) | E0507507 | `instanz_gruppe: kind` |
| **Nutzerfrage 1** | `kind_pb_uebertragung_beantragt` | bool | — | **NEU**: "Antrag" § 33b Abs. 5 S. 1 |
| **Nutzerfrage 2** | `kind_pb_kind_nimmt_nicht_in_anspruch` | bool | — | **NEU**: "wenn ihn das Kind nicht in Anspruch nimmt" |
| Accessor (Pauschbetrag) | `catala_p33b_5_kind_uebertragung()` | pure-Python | — | in `golden/runner.py` |
| Accessor (Fahrtkostenpauschale) | `catala_p33_2a_fahrtkosten()` | pure-Python | — | 900/4.500 €, Input: Kind-Merkmale |
| Ring-Call §33b | `api.py` Z. 791-799 erweitern | += | — | zu `ausserg` addieren |
| Ring-Call §33 Abs.2a | `api.py` Z. 807-810 erweitern | += | — | in `catala_p33_agb`-Input, nicht `ausserg` |
| Ring-Call §33-Ausschluss | `api.py` Z. 807-810 | — | — | wenn Kind-PB übertragen: agb_aufwendungen kürzen? |
| Test | `tests/test_p33b_5_kind_uebertragung.py` | — | — | Unit + Ring-Differential |

**Vereinfachung (MVP):** Die bestehenden `catala_behinderten_pb`-Accessors können **wiederverwendet** werden — sie brauchen nur andere Input-Felder (kind_grad_der_behinderung statt rentner_grad_der_behinderung). Die 14 Kz sind für die ELSTER-Deklaration, nicht für die Rechnung.

**Aufwand: ~6–8 h** (2 h Bindung 14 Kz + 2 Nutzerfragen, 1,5 h Accessor-Wrapper + Fahrtkostenpauschale, 1 h Ring-Call + IdNr-Gate + §33-Ausschluss, 1,5 h Tests, 1 h Korrektur §33 Abs.2a S.6/7).

### 4. Haken

**"Auf Antrag" und "wenn ihn das Kind nicht in Anspruch nimmt" — zwei neue Nutzerfragen.**
Der Steuerpflichtige muss einen Antrag stellen (keine automatische Übertragung). Das Kind darf den Pauschbetrag nicht selbst in Anspruch nehmen. Beide sind `askable: true`-Felder.

**§ 33 Abs. 2a S.6/7 — Fahrtkostenpauschale in der §33-Kette (Korrektur V1→V2):**
Die Fahrtkostenpauschale ist KEIN Summand zu `ausserg` (p33b-Pauschbeträge). S.7: "Die Pauschale ist bei der Ermittlung des Teils der Aufwendungen im Sinne des Absatzes 1, der die zumutbare Belastung übersteigt, einzubeziehen." Sie geht IN den `aussergewoehnliche_belastungen`-Input von `catala_p33_agb` (runner.py:484-492), nicht daneben.

Der Ring-Call in `api.py` Z.807-810 ändert sich von:
```python
catala_p33_agb({"aussergewoehnliche_belastungen": _c("agb_aufwendungen") // 100, ...})
```
zu:
```python
catala_p33_agb({"aussergewoehnliche_belastungen": (_c("agb_aufwendungen") + fahrtkostenpauschale) // 100, ...})
```

**Fahrtkostenpauschale (§ 33 Abs. 2a) — vierter Abzug, in B3 integriert.**
Die Kz E0507302/E0507403 unter `Kind/Beh_Fk_Pausch` sind § 33 Abs. 2a, nicht § 33b Abs. 5. Die Norm hat eigene Beträge (900/4.500 €), eigene GdB-Voraussetzungen, und S.8/9 stellt die Kopplung an § 33b Abs. 5 her. Integration in B3, da das Gesetz die Kopplung herstellt. **Aufwands-Korrektur: +1 h für den Accessor + Ring-Korrektur.**

**§ 33b Abs. 5 S. 4 — Ausschluss § 33 bei Wahl des Pauschbetrags.**
"In diesen Fällen besteht für Aufwendungen, für die der Behinderten-Pauschbetrag gilt, kein Anspruch auf eine Steuerermäßigung nach § 33." Wechselwirkung mit `catala_p33_agb`. Wenn der Kind-PB übertragen wird, darf der § 33-Abzug für behinderungsbedingte Aufwendungen nicht parallel gewährt werden. **Muss im Ring-Call geprüft werden.**

**Hälftige Aufteilung zwischen Eltern (Satz 2):**
"grundsätzlich auf beide Elternteile je zur Hälfte" — bei Zusammenveranlagung egal, bei Einzelveranlagung relevant. Die Kz E0506007/E0507507 decken das für ELSTER ab.

---

## Instanz-Achse: alle drei Normen sind pro Kind

**Stand:** Die Instanz-Mechanik ist gebaut, getestet und in Betrieb:
- `parse_instanz()` in `est_mapping.py:188` — Enumerations-Wahrheit
- `_deklariere_instanz()` in `est_mapping.py:201` — per-Instanz-Routing
- `anlage_instanzen`-Bucket in `est_mapping.py:259,373`
- `instanzen()`-Reader in `est_mapping.py:449` — Ring-Naht
- `instanz_gruppe: kind` bereits in `bindung_kap_vv_familie.yaml` Z. 462-565 (kind_idnr, kind_geburtsjahr, etc.)
- ELSTER-XML-Writer: `elster_xml.py` Z. 286-296 verarbeitet `kind_anlagen` + `anlage_instanzen["kind"]`

**Lücke:** Der Ring (`api.py`) liest per-Kind-Felder NICHT per-Instanz aus. Die Sonderausgaben-Rechnung in `api.py` verwendet globale Felder (`kinderbetreuungskosten`, `kinderbetreuung_anzahl_kinder`). Die Instanz-Mechanik routet per-Kind-Felder in den ELSTER-Writer, aber der Ring bekommt nur die Summe.

**Zwei Optionen:**

**Option 1 (Globales Summen-Feld + Anzahl-Multiplikator) — wie derzeitige kinderbetreuungskosten.**
- Ring liest `kind_schulgeld` (cent, Summe) + `kind_schulgeld_anzahl_kinder` (int).
- Accessor teilt Summe durch Anzahl, deckelt pro Kind, multipliziert zurück.
- XML: `kind_schulgeld` → E0505607 im ersten Kind-Container (Instanz 1). Die Instanz-Mechanik erzeugt die korrekte XML-Struktur, aber der Betrag landet beim ersten Kind.
- **Fail-closed? JA.** Ring und XML lesen DASSELBE globale Feld (`kind_schulgeld`). Die Summe ist identisch. Das per-Kind-Layout im XML ist anders (erster Container trägt die Summe), aber der Gesamtbetrag im XML = der Ring-Betrag. Ein Differential-Test, der `zahl_cent` gegen die XML-Kz-Summe prüft, würde die Übereinstimmung halten — es gibt keine zwei unabhängigen Repräsentationen, nur eine.
- **Risiko:** Bei mehreren Kindern mit ungleichen Beträgen ist die ELSTER-Zuordnung falsch (alles beim ersten Kind). Das Finanzamt könnte die Angabe beanstanden. Aber: die Kz sind optional (minOccurs=0), und der Ring-Betrag ist korrekt.
- **Kosten:** 0 h extra (im MVP enthalten).

**Option 2 (Per-Kind-Ring-Leser via EM.instanzen) — wie gwg/vv_objekt/rente.**
- Ring iteriert über `EM.instanzen(store, bindung, "kind")`, summiert pro Kind den gedeckelten Betrag.
- Genauer: bei 2 Kindern mit 8.000 € und 2.000 € würde Option 1 gleichmäßig 5.000/5.000 verteilen (= 3.000 € Abzug), Option 2 korrekt 5.000 + 600 = 5.600 € Abzug deckeln.
- **Kosten für B1: +2 h** (Ring-Änderung + per-Kind-Read-Keys + Test-Anpassung + bestehende kinderbetreuungskosten müssten ebenfalls umgestellt werden, da sie das gleiche Muster haben).
- **Nutzen:** Nur bei ungleichen Beträgen pro Kind relevant. Bei gleichen Beträgen identisch zu Option 1.

**Entscheidung (Julius, 2026-08-06): Option 2 (per-Kind-Weg).** Option 1 getestet und widerlegt: der Kz liegt im Kind-Container (XSD-Pfad: `E10/Kind/KBK/Art/Sum/E0506105`). Der Ring liest die Feld-ID als Summe über alle Kinder, der Kz bedeutet an seiner Schema-Stelle "Betrag für DIESES Kind". Zwei Semantiken, eine Feld-ID. Fehlerklasse "zwei Repräsentationen, ungetestete Übergabe" — reproduziert.

Messung (Julius, an der bestehenden Vorlage kinderbetreuungskosten):

| Aufwand | Kinder | Ring | FA aus XML | Diff |
|---|---|---|---|---|
| 6.000 | 2 | 4.800 | 4.800 | 0 |
| 9.600 | 2 | 7.680 | 4.800 | +2.880 |
| 12.000 | 2 | 9.600 | 4.800 | +4.800 |
| 20.000 | 3 | 14.400 | 4.800 | +9.600 |

Richtung: Ring zu hoch (Erklärung zu niedrig), Nutzer zahlt zu viel Steuer. Produktlich schlimmer: wir zeigen eine Erstattung an, die nie kommt.

**Kosten Option 2 für B1: +2 h** (Ring-Änderung von globalem Summen-Feld zu `EM.instanzen(store, bindung, "kind")`, per-Kind-Read-Keys, Test-Anpassung, Differential-Test mit 2 Kindern und ungleichen Beträgen gegen XML).

---

## Zusammenfassung

| Norm | Kz | Aufwand (h) | Größte Unsicherheit |
|---|---|---|---|
| **B1** Schulgeld § 10 Abs. 1 Nr. 9 | 5 | 4–5 | per-Kind-Ring-Leser (Option 2); "je Elternpaar nur einmal" — 2.500 € Deckel + Verdopplung bei ZV |
| **B2** KV/PV-Kind § 10 Abs. 1 Nr. 3 S. 2 | 12 | 4–5 | MVP globales Feld vs alle 12 Kz per-kind; IdNr-Gate nötig |
| **B3** § 33b Abs. 5 + Fahrtkostenpauschale § 33 Abs. 2a | 14 | 6–8 | Zwei Nutzerfragen + §33-Abs.2a S.6/7 in §33-Kette + §33-Ausschluss S.4 |
| **Σ** | 31 | **14–18** | |

### Empfohlene Reihenfolge

1. **B1 (Schulgeld)** — 4–5 h. per-Kind-Ring-Leser (Option 2). Setzt das Muster (Vorlage kinderbetreuungskosten, Parameter, Accessor, Ring-Call, Bindung, Tests). Keine neuen Nutzerfragen, keine Wechselwirkung mit bestehenden Regeln. **Damit beginnen.**

2. **B2 (KV/PV-Kind)** — 4–5 h. Erweiterung der bestehenden KV/PV-Rechnung. IdNr-Gate, § 10 Abs. 4-Deckel, AW_Stpfl/AW_Kind-Unterscheidung. Rechnung selbst ist trivial (additiv zur `basis_kv_pv`).

3. **B3 (§33b Abs. 5 + Fahrtkostenpauschale §33 Abs.2a)** — 6–8 h. Die komplexeste Norm. Zwei neue Nutzerfragen, §33-Abs.2a S.6/7-Korrektur (Fahrtkostenpauschale in §33-Kette, nicht daneben), Wechselwirkung mit § 33-Abzug, 14 Kz, Aufteilung zwischen Eltern. **Letzte, wenn die ersten beiden stehen.**

---

## Offene Fragen (an Julius zur Entscheidung)

1. **B1: MVP-Deckel** — 2.500 € × Kind bei Einzelveranlagung, 5.000 € × Kind bei Zusammenveranlagung? Oder fix 2.500 € × Kind (over-tax-safe bei Zusammenveranlagung)?

2. **B2: MVP nur ein globales Feld?** Oder alle 12 Kz sofort per-kind binden? (s.o. Instanz-Achse)

3. **B3: § 33-Ausschluss (Satz 4) — automatisch oder Hinweis?** "In diesen Fällen besteht für Aufwendungen, für die der Behinderten-Pauschbetrag gilt, kein Anspruch auf eine Steuerermäßigung nach § 33." Automatische Kürzung des § 33-Abzugs bei Kind-PB-Übertragung, oder Hinweis an den Nutzer?

4. **B3: Fahrtkostenpauschale integriert in B3?** Oder als separaten B4-Baustein? Die Norm ist § 33 Abs. 2a, nicht § 33b Abs. 5, aber die Kz liegen im selben XSD-Container und S.8/9 stellt die Kopplung her. Empfehlung: integriert.