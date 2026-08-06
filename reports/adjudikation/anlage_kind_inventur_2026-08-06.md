# Anlage Kind — Inventur: Kz ohne Feld

**Auftrag:** Alle Kz in E10/Kind (Kind_67907_CType) melden, sortieren, ob wir ein Feld brauchen.
**Stand:** 2026-08-06, E10-2025.xsd, HEAD 6cb2f31.

## Methode

Vollständiger XSD-Walk von `Kind_67907_CType` (Z.11511). Jedes Kz mit Pfad, xs:documentation
(wörtlich), Typ, Kardinalität protokolliert. Dann gegen produkt/bindung/*.yaml + est_mapping.py
abgeglichen.

## Ergebnis: 103 Kz, davon 0 mit minOccurs≥1

**Kein Pflicht-Kz ohne Feld.** Kein Blocker.

## Bekannt gebunden (6)

| Kz | Pfad | Feld |
|---|---|---|
| E0500406 | Kind/Ang_Kind/Allg | kind_idnr |
| E0500807 | Kind/K_Verh/K_Verh_A | kind_kindschaftsverhaeltnis_a |
| E0500808 | Kind/K_Verh/K_Verh_B | kind_kindschaftsverhaeltnis_b |
| E0500601 | Kind/K_Verh/K_Verh_A | kind_kindschaftsverh_zeitraum_a |
| E0500805 | Kind/K_Verh/K_Verh_B | kind_kindschaftsverh_zeitraum_b |
| E0506105 | Kind/KBK/Art/Sum | kinderbetreuungskosten (via E0506105, §10 Abs.1 Nr.5) |

## Bekannt dokumentiert (2)

| Kz | Status | Grund |
|---|---|---|
| E0500701 | TYP-MISMATCH, endgueltig | GeburtsDATUM (DatumTTpMMpJJJJ), Feld ist nur Jahr. `kind_geburtsjahr` |
| E0500702 | KEIN FELD NÖTIG | Kindergeld-Anspruch, Werte-Kodierung nicht verifizierbar (2026-07-18) |

## Restliche ~95 Kz — Sortierung in drei Klassen

### (A) FORMULARDETAIL — kein Handlungsbedarf (MVP trägt)

14 Gruppen. Kein eigenes Feld nötig, weil:

- Namens-/Adressdaten: werden vom Finanzamt aus dem Melderegister bezogen
- Zeiträume für Wohnsitz/Ausbildung/Arbeitslosigkeit/Behinderung: Detailangaben, die
  der Steuerpflichtige im Formular handschriftlich ergänzt oder die das FA schon hat
- Übertragungs-Anträge (KFB, Beh-Pauschbetrag): betreffen die Aufteilung zwischen
  Eltern, nicht den Steuerbetrag selbst
- Aufteilungs-Prozentsätze (Elt_k_ZV): betreffen Verteilung zwischen Eltern, nicht
  den Steuerbetrag. Dazu gehört **E0506202** ("Laut gesondertem gemeinsamen Antrag ist
  der Freibetrag zur Abgeltung eines Sonderbedarfs bei Berufsausbildung in einem anderen
  Verhältnis als je zur Hälfte aufzuteilen. Der bei mir zu berücksichtigende Anteil
  beträgt (in %)") — das ist der Aufteilungs-Antrag für §33a Abs.2, nicht der
  Sonderbedarf selbst. Der Sonderbedarf selbst wird bereits gerechnet
  (p33a_ausbildungsfreibetrag, 1200€ Festbetrag).

### (B) EIGENER ABZUGSTATBESTAND, NICHT gerechnet — Rechen-Lücke

Diese Kz gehören zu Steuer-Normen, die wir gar nicht rechnen. Ein Nutzer, der diese
Abzüge geltend machen könnte, zahlt bei uns zu viel Steuer.

#### B1: Schulgeld (§ 10 Abs. 1 Nr. 9 EStG)

| Kz | Pfad | Typ | xs:documentation |
|---|---|---|---|
| E0505607 | Kind/Schulgeld/Sum | GanzzahlOhneFuehrNull_MaxVK12 | "berücksichtigungsfähige Gesamtaufwendungen der Eltern" |
| E0504405 | Kind/Schulgeld/Einz | GanzzahlOhneFuehrNull_MaxVK12 | "berücksichtigungsfähige Gesamtaufwendungen der Eltern: Einzelbetrag" |
| E0505606 | Kind/Schulgeld/Einz | String | "Bezeichnung der Schule oder deren Träger" |
| E0504505 | Kind/Schulgeld/Elt_k_ZV | GanzzahlOhneFuehrNull_MaxVK12 | "Das von mir übernommene Schulgeld beträgt" |
| E0504603 | Kind/Schulgeld/Elt_k_ZV | GanzzahlNichtNeg_MaxW100 | "Laut gesondertem gemeinsamen Antrag ist für das Kind der Höchstbetrag für das Schulgeld in einem anderen Verhältnis als je zur Hälfte aufzuteilen. Der bei mir zu berücksichtigende Anteil beträgt (in %)" |

**Norm:** § 10 Abs. 1 Nr. 9 EStG (sources/gesetze-im-internet/estg_p10_2026-07-11.txt):
"30 Prozent des Entgelts, höchstens 5 000 Euro, das der Steuerpflichtige für ein Kind,
für das er Anspruch auf einen Freibetrag nach § 32 Absatz 6 oder auf Kindergeld hat,
für dessen Besuch einer Schule in freier Trägerschaft oder einer überwiegend privat
finanzierten Schule entrichtet, mit Ausnahme des Entgelts für Beherbergung, Betreuung
und Verpflegung."

**Rechnen wir?** NEIN. Kein Feld, keine Catala-Regel, kein Eintrag in rules.yaml.
Nur Fundstelle in der Anleitung (sources/bfinv/anleitungen/anl_kind_2025.txt Z.56-58).

#### B2: KV/PV-Beiträge des Kindes (§ 10 Abs. 1 Nr. 3 S. 2 EStG)

| Kz | Pfad | Typ | xs:documentation |
|---|---|---|---|
| E0503110 | Kind/KV_PV/AW_Stpfl | GanzzahlNichtNeg_MaxVK12 | "Beiträge zu Krankenversicherungen des Kindes (nur Basisabsicherung, keine Wahlleistungen)" |
| E0503310 | Kind/KV_PV/AW_Stpfl | GanzzahlNichtNeg_MaxVK12 | "Beiträge zur sozialen Pflegeversicherung und / oder zur privaten Pflege-Pflichtversicherung" |
| E0503409 | Kind/KV_PV/AW_Stpfl | GanzzahlNichtNeg_MaxVK12 | "Von den Versicherungen … erstattete Beträge" |
| E0503609 | Kind/KV_PV/AW_Stpfl | GanzzahlOhneFuehrNull_MaxVK12 | "Über die Basisabsicherung hinausgehende Beiträge … abzüglich erstatteter Beiträge" |
| E0503111 | Kind/KV_PV/AW_Kind | GanzzahlNichtNeg_MaxVK12 | dto. (AW_Kind = wenn Kind selbst versichert?) |
| E0503209 | Kind/KV_PV/AW_Kind | GanzzahlNichtNeg_MaxVK12 | "In Zeile E0503111 enthaltene Beiträge, aus denen sich ein Anspruch auf Krankengeld ergibt" |
| E0503311 | Kind/KV_PV/AW_Kind | GanzzahlNichtNeg_MaxVK12 | "Beiträge zur sozialen Pflegeversicherung und / oder zur privaten Pflege-Pflichtversicherung" |
| E0503410 | Kind/KV_PV/AW_Kind | GanzzahlNichtNeg_MaxVK12 | "erstattete Beträge" |
| E0503509 | Kind/KV_PV/AW_Kind | GanzzahlNichtNeg_MaxVK12 | "In Zeile E0503410 enthaltene Beiträge, aus denen sich ein Anspruch auf Krankengeld ergibt" |
| E0503610 | Kind/KV_PV/AW_Kind | GanzzahlNichtNeg_MaxVK12 | "Zuschuss von dritter Seite zu den Beiträgen" |
| E0503822 | Kind/KV_PV_ausl/AW | GanzzahlNichtNeg_MaxVK12 | "Beiträge … zu ausländischen Kranken- und Pflegeversicherungen des Kindes" |
| E0503823 | Kind/KV_PV_ausl/AW | GanzzahlNichtNeg_MaxVK12 | "In Zeile E0503822 enthaltene Beiträge, aus denen sich ein Anspruch auf Krankengeld ergibt" |

**Norm:** § 10 Abs. 1 Nr. 3 S. 2 EStG (sources/…/estg_p10_2026-07-11.txt):
"Als eigene Beiträge des Steuerpflichtigen können auch eigene Beiträge im Sinne der
Buchstaben a oder b eines Kindes behandelt werden, wenn der Steuerpflichtige die
Beiträge des Kindes … durch Leistungen in Form von Bar- oder Sachunterhalt
wirtschaftlich getragen hat … Voraussetzung für die Berücksichtigung beim
Steuerpflichtigen ist die Angabe der erteilten Identifikationsnummer des Kindes."

**Rechnen wir?** NEIN. Wir rechnen KV/PV nur für den Steuerpflichtigen selbst
(p10_1_3_3a_kv_pv). Die Kind-Beiträge sind ein eigener Abzugstatbestand mit
Voraussetzung (IdNr des Kindes). Ein Elternteil, der die KV/PV des Kindes zahlt,
verliert den Abzug.

#### B3: Behinderten-Pauschbetrag des Kindes, Übertragung auf Elternteil (§ 33b Abs. 5 EStG)

| Kz | Pfad | Typ | xs:documentation |
|---|---|---|---|
| E0505809 | Kind/Ueb_PB_Beh_Hbl/Beh/Ausw_Rentb_Besch | String_MinL2_MaxL3 | "Grad der Behinderung" |
| E0504601 | Kind/Ueb_PB_Beh_Hbl/Beh/Ausw_Rentb_Besch | DATUM_MMJJ | "gültig von" |
| E0504602 | Kind/Ueb_PB_Beh_Hbl/Beh/Ausw_Rentb_Besch | DATUM_MMJJ | "gültig bis" |
| E0505908 | Kind/Ueb_PB_Beh_Hbl/Beh/Ausw_Rentb_Besch | JaX | "unbefristet gültig" |
| E0505808 | Kind/Ueb_PB_Beh_Hbl/Beh/Geh_Steh | Ja1 | "erheblich gehbehindert (Merkzeichen G) / außergewöhnlich gehbehindert (Merkzeichen aG)" |
| E0505807 | Kind/Ueb_PB_Beh_Hbl/Beh/Blind_Hilfl | Ja1 | "blind / taubblind / ständig hilflos (Merkzeichen Bl, TBl und/oder H)" |
| E0505805 | Kind/Ueb_PB_Beh_Hbl/Hbl | Ja1 | "Die Übertragung des Hinterbliebenen-Pauschbetrags wird beantragt" |
| E0506007 | Kind/Ueb_PB_Beh_Hbl/Elt_k_ZV | Ganzzahl_MaxW100 | "Laut gesondertem gemeinsamen Antrag sind die für das Kind zu gewährenden Pauschbeträge für Behinderte / Hinterbliebene in einem anderen Verhältnis als je zur Hälfte aufzuteilen. Der bei mir zu berücksichtigende Anteil beträgt (in %)" |
| E0507302 | Kind/Beh_Fk_Pausch | Ja1 | "Das Kind hat einen Grad der Behinderung von mindestens 80 oder einen Grad der Behinderung von mindestens 70 und Merkzeichen G" |
| E0507403 | Kind/Beh_Fk_Pausch | Ja1 | "Das Kind ist außergewöhnlich gehbehindert / blind / taubblind / ständig hilflos …" |
| E0507507 | Kind/Beh_Fk_Pausch/Elt_k_ZV | Ganzzahl_MaxW100 | "Laut gesondertem gemeinsamen Antrag ist die … behinderungsbedingte Fahrtkostenpauschale in einem anderen Verhältnis als je zur Hälfte aufzuteilen. Der bei mir zu berücksichtigende Anteil beträgt (in %)" |

**Norm:** § 33b Abs. 5 EStG (sources/…/estg_p33b_2026-07-13.txt):
"Steht der Behinderten-Pauschbetrag oder der Hinterbliebenen-Pauschbetrag einem Kind
zu, für das der Steuerpflichtige Anspruch auf einen Freibetrag nach § 32 Absatz 6
oder auf Kindergeld hat, so wird der Pauschbetrag auf Antrag auf den Steuerpflichtigen
übertragen, wenn ihn das Kind nicht in Anspruch nimmt. Dabei ist der Pauschbetrag
grundsätzlich auf beide Elternteile je zur Hälfte aufzuteilen …"

**Rechnen wir?** NEIN. Wir rechnen `catala_behinderten_pb` / `catala_pflege_pb` /
`catala_hinterbliebenen_pb` nur für den Steuerpflichtigen selbst (grad_der_behinderung,
ist_hilflos_blind_taubblind, pflegegrad). Der gesamte Container `Ueb_PB_Beh_Hbl`
existiert im Modell nicht. Ein Elternteil mit behindertem Kind verliert den Pauschbetrag.
Die Binding-Kommentare in bindung_rentner.yaml (Z.275, 342, 359) dokumentieren diese
Kz bereits als "strukturell fremd (§33b Abs.5-Kind-Übertragung, nicht implementiert)".

### (C) DETAIL ZU VORHANDENER REGEL — Deklarations-Lücke

Keine Kz in dieser Kategorie. Die vermeintlichen (C)-Kandidaten aus meiner ersten
Durchsicht waren:

- **E0506202** (FB_Abgelt_Sbed_BA/Elt_k_ZV): Aufteilungs-Antrag zwischen Eltern für
  §33a Abs. 2 Sonderbedarf, nicht der Sonderbedarf selbst. → (A)
- **E0505809/E0505807/E0505805** (Ueb_PB_Beh_Hbl): §33b Abs. 5 Kind-Übertragung,
  ganzer Container nicht implementiert → (B)
- **E0507302/E0507403** (Beh_Fk_Pausch): §33b Abs. 5 Fahrtkostenpauschale Kind,
  ebenfalls nicht implementiert → (B)

## Zusammenfassung

| Kategorie | Anzahl Erwähnungen | Handlungsbedarf |
|---|---|---|
| (A) Formulardetail | ~80 | Keiner |
| (B) Rechen-Lücke | 3 Bereiche (Schulgeld, KV/PV-Kind, §33b Kind-Übertragung) | **Produktentscheidung** — Julius entscheidet |
| (C) Deklarations-Lücke | 0 | — |

**B1 Schulgeld** (§ 10 Abs. 1 Nr. 9): 30 % des Entgelts, max. 5.000 € je Kind.
Neue Catala-Regel + Feld + Bindung nötig.

**B2 KV/PV-Kind** (§ 10 Abs. 1 Nr. 3 S. 2): Kind-Beiträge als eigene Sonderausgabe
des Steuerpflichtigen. Erweiterung der bestehenden p10_kv_pv-Regel um Kind-Parameter
+ Voraussetzung IdNr des Kindes.

**B3 §33b Kind-Übertragung** (§ 33b Abs. 5): Behinderten-/Hinterbliebenen-Pauschbetrag
des Kindes auf Elternteil übertragbar. Neue Regel oder Erweiterung der bestehenden
p33b-Regeln um Kind-Parameter + Antragserfordernis.