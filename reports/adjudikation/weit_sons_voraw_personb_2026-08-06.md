# Adjudikation: VOR/Weit_Sons_VorAW (Person A+B) — fünf Sum-Kz + Pers-Zweig

**Datum:** 2026-08-06
**HEAD:** d059721
**Prüfer:** dev-2

## Zusammenfassung

| Punkt | Status |
|-------|--------|
| (1) Fünf Sum-Kz gegenprüfen | ✅ Alle bestätigt (Cent-Beträge, korrekte Pfade) |
| (2) Einz/Sum-Struktur | ✅ Dokumentiert; ERiC-Plausibilität offen |
| (3) Person B | ⚠️ NICHT person-multipliziert → siehe Detail |
| (4) E2004403 einordnen | ✅ Alternativ zu E2001403, nicht additiv |

---

## (1) Fünf Sum-Kz — Drei Pflichtzeilen je Kz

Geprüft am E10-2025.xsd (ERiC 44.2.4.0). Alle fünf Kz teilen denselben XSD-Typ: `GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE`.

**Typ-Kette (vollständig aufgelöst):**
```
GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE
  → complexType simpleContent/extension
    base=GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType
      → complexType simpleContent/restriction
        base=GanzzahlOhneFuehrNullBaseCType
          → complexType simpleContent/restriction
            base=IntegerBaseWithAliasCType
              → complexType simpleContent/extension
                base=xs:integer
```

**Fazit:** Alle fünf Kz sind `xs:integer`-basierte Cent-Beträge (MaxVK12 = max 12 Vorkommastellen). Kein Textfeld, kein Ja-Feld. **E0901005-Falle besteht nicht.**

### E2001403 — AL_Vers/Sum (Arbeitslosenversicherung)
- **Pfad:** `E10/VOR/Weit_Sons_VorAW/A_B_LP/AL_Vers/Sum/E2001403`
- **xs:documentation:** `Summe`
- **Kardinalität Elternknoten (Sum):** maxOccurs=1, minOccurs=0
- **Typ:** GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE (→ xs:integer, Cent)
- **Kz selbst:** minOccurs=0, maxOccurs=1

### E2001503 — ErwU_BU_Vers/Sum (Erwerbs-/Berufsunfähigkeit)
- **Pfad:** `E10/VOR/Weit_Sons_VorAW/A_B_LP/ErwU_BU_Vers/Sum/E2001503`
- **xs:documentation:** `Summe`
- **Kardinalität Elternknoten (Sum):** maxOccurs=1, minOccurs=0
- **Typ:** GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE (→ xs:integer, Cent)
- **Kz selbst:** minOccurs=0, maxOccurs=1

### E2001803 — U_HP_Ris_Vers/Sum (Unfall/Haftpflicht/Risiko)
- **Pfad:** `E10/VOR/Weit_Sons_VorAW/A_B_LP/U_HP_Ris_Vers/Sum/E2001803`
- **xs:documentation:** `Summe`
- **Kardinalität Elternknoten (Sum):** maxOccurs=1, minOccurs=0
- **Typ:** GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE (→ xs:integer, Cent)
- **Kz selbst:** minOccurs=0, maxOccurs=1

### E2001903 — RV_m_WR_KapLV/Sum (Rentenversicherung mit Überschuss/Kapitallebensversicherung)
- **Pfad:** `E10/VOR/Weit_Sons_VorAW/A_B_LP/RV_m_WR_KapLV/Sum/E2001903`
- **xs:documentation:** `Summe`
- **Kardinalität Elternknoten (Sum):** maxOccurs=1, minOccurs=0
- **Typ:** GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE (→ xs:integer, Cent)
- **Kz selbst:** minOccurs=0, maxOccurs=1

### E2002003 — RV_o_WR_o_AV/Sum (Rentenversicherung ohne Überschuss)
- **Pfad:** `E10/VOR/Weit_Sons_VorAW/A_B_LP/RV_o_WR_o_AV/Sum/E2002003`
- **xs:documentation:** `Summe`
- **Kardinalität Elternknoten (Sum):** maxOccurs=1, minOccurs=0
- **Typ:** GanzzahlOhneFuehrNull_MaxVK12_Muster1442570515_CType_RABE (→ xs:integer, Cent)
- **Kz selbst:** minOccurs=0, maxOccurs=1

---

## (2) Einz/Sum-Struktur

Jede der fünf Kategorien unter `A_B_LP` hat exakt dieselbe Struktur — dieselbe wie bei Anlage G (Gewinn) und S (Gewinn):

```
<AL_Vers maxOccurs=1>
  <Einz maxOccurs=99 minOccurs=0>
    E2001401 (Bezeichnung)     Typ: String_MinL1_MaxL999_CType_RABE
    E2001402 (Betrag)          Typ: GanzzahlOhneFuehrNull_MaxVK12_..._RABE
  <Sum maxOccurs=1 minOccurs=0>
    E2001403 (Summe)           Typ: GanzzahlOhneFuehrNull_MaxVK12_..._RABE
```

**Was das für uns bedeutet:**
Wir liefern nur Sum (Gesamtbetrag je Kategorie). Einz mit Einzelbeträgen pro Betrieb/Versicherung ist für uns nicht relevant, da wir die Quelle nicht nach Einzelbeträgen aufschlüsseln.

**ERiC-Plausibilität (Vorbehalt):**
Das XSD setzt minOccurs=0 für beide Einz und Sum. Ob ERiC bei gefülltem Sum mindestens einen Einz-Eintrag erwartet, ist im XSD nicht geregelt. Das wären Applikationsbedingungen (Prüfregeln der Finanzverwaltung), die ERiC zur Laufzeit validiert. Wir haben keine lokale Kopie der Applikationsbedingungen für die ESt-Datenart. Mögliche Szenarien:
- ERiC akzeptiert Sum ohne Einz → kein Problem
- ERiC verlangt mindestens einen Einz-Eintrag → wir müssten einen Dummy-Eintrag erzeugen
- ERiC prüft Sum = Σ(Einz) → wir müssten Sum = Einz garantieren

**Empfehlung:** Beim ersten XML-Integrationstest prüfen, ob ERiC Sum ohne Einz akzeptiert. Falls nicht, reicht ein Dummy-Einz mit identischem Betrag (da wir nur einen Gesamtwert pro Kategorie haben).

---

## (3) Person B — die entscheidende Frage

### Struktur von Weit_Sons_VorAW

```
Weit_Sons_VorAW maxOccurs=1        ← EINMALIG, nicht person-multipliziert
  Pers maxOccurs=2                 ← Person-Zweig (mit <Person>)
    Person maxOccurs=1
    E2004403 maxOccurs=1           ← ALV aus LStB Nr. 27
  A_B_LP maxOccurs=1               ← KEIN <Person>-Element
    AL_Vers maxOccurs=1
      Einz/Sum                     ← E2001403 (Summe)
    ErwU_BU_Vers maxOccurs=1       ← E2001503 (Summe)
    U_HP_Ris_Vers maxOccurs=1      ← E2001803 (Summe)
    RV_m_WR_KapLV maxOccurs=1      ← E2001903 (Summe)
    RV_o_WR_o_AV maxOccurs=1       ← E2002003 (Summe)
```

**Vergleich mit Beitr_g_KV_PV_Inl (Basis KV/PV, maxOccurs=2):**
```
Beitr_g_KV_PV_Inl maxOccurs=2     ← person-multipliziert
  Person maxOccurs=1               ← <Person>-Pflichtkind
  AN maxOccurs=1                   ← Arbeitnehmer-Pfad
    E2001203 (KV-Beiträge)
    ...
  And_Pers maxOccurs=1             ← andere Person (Rentner)
    E2001805 (KV-Beiträge)
    ...
```

**Befund:**
- `Weit_Sons_VorAW` hat **maxOccurs=1** — kein Pendant zu `Beitr_g_KV_PV_Inl` maxOccurs=2.
- `A_B_LP` hat **kein** `<Person>`-Element — kein Pendant zum Person-Bucket.
- `Pers` (maxOccurs=2) existiert nur **neben** `A_B_LP`, nicht als dessen Container.
- `Pers` führt ausschließlich `E2004403` (ALV aus LStB Nr. 27) — keine Entsprechung für die anderen vier Kategorien.
- `VOR` selbst hat maxOccurs=1 — kein Container für zwei Personen.

### Konsequenz für dev-1

1. **Die fünf Sum-Kz (E2001403–E2002003) sind shared amounts.** Person A+B tragen denselben Wert im ELSTER-XML. Wenn Person A und B unterschiedliche Werte haben, müssen sie vor dem Befüllen summiert werden.

2. **Person-B-Werte für den Ring:** Die _partner-Felder (`vorsorge_arbeitslosenversicherung_partner` etc.) haben zurecht `kz_status: offen` — sie haben kein eigenes Kz im ELSTER-Schema. Der Ring rechnet getrennt, aber ELSTER bekommt die Summe.

3. **E2004403 als Alternative für Person B ALV:** Pers maxOccurs=2 erlaubt getrennte ALV-Werte pro Person. Dies könnte für Person B genutzt werden. Siehe Punkt (4).

4. **Die anderen vier Kategorien (Erwerbsunfähigkeit, Unfall/Haftpflicht, RV mit/ohne Überschuss) haben gar keinen Pers-Zweig.** Für sie gibt es keine person-getrennte ELSTER-Möglichkeit.

---

## (4) E2004403 einordnen

### Pflichtzeilen
- **Pfad:** `E10/VOR/Weit_Sons_VorAW/Pers/E2004403`
- **xs:documentation:** `Arbeitnehmerbeiträge zur Arbeitslosenversicherung laut Nr. 27 der Lohnsteuerbescheinigung`
- **Kardinalität Elternknoten (Pers):** maxOccurs=2, minOccurs=0
- **Kz selbst:** minOccurs=0, maxOccurs=1
- **Typ:** `GanzzahlOhneFuehrNull_MaxVK4_CType_RABE` (→ xs:integer, Cent, MaxVK4 = kleinere Stellenzahl als MaxVK12, da nur ALV-Beiträge)

### Verhältnis zu E2001403

| Merkmal | E2001403 (A_B_LP/AL_Vers/Sum) | E2004403 (Pers/) |
|---------|-------------------------------|-------------------|
| Pfad | A_B_LP/AL_Vers/Sum | Pers |
| maxOccurs Eltern | 1 (A_B_LP) | 2 (Pers) |
| Person-Differenzierung | Nein | Ja (über <Person>) |
| Wertebereich | MaxVK12 | MaxVK4 |
| Kategorie | Alle 5 Kategorien | Nur ALV |
| Quelle | Summenfeld | LStB Nr. 27 direkt |

**E2004403 ist eine ALTERNATIVE zu E2001403, nicht additiv.** Beide messen dieselbe Größe (Arbeitnehmerbeiträge zur Arbeitslosenversicherung), aber:
- E2001403 ist der aggregierte Summenwert (Sum) für die gemeinsame Erklärung
- E2004403 ist der personen-individuelle Einzelwert aus der LStB

**Empfehlung:**
- **E2001403 bleibt die richtige Wahl für Person A (oder Summe A+B).** Es ist das Sum-Feld der AL_Vers-Kategorie, das wir mit dem Gesamtwert befüllen. Das ist konsistent mit den anderen vier Kategorien, die ebenfalls nur Sum (kein Pers) haben.
- **E2004403 könnte für die Person-B-ALV genutzt werden** — der Pers-Zweig (maxOccurs=2) erlaubt einen zweiten Eintrag mit <Person>B. Allerdings müsste dann aus dem Gesamtwert (A+B) der A-Anteil abgezogen werden, was eine zusätzliche Aufteilung erfordert. Zudem deckt Pers nur ALV ab, nicht die anderen vier Kategorien.
- **Praktikabelste Lösung:** A_B_LP/AL_Vers/Sum mit A+B-Wert befüllen, Pers/E2004403 freilassen. Das ist der Status quo (dev-1 hat E2001403 gebunden, E2004403 ist ungebunden).

### Import-Situation (vast_mapping)
`vast_mapping.py` mapped `ArbnAnteilArblVers` → `vorsorge_arbeitslosenversicherung` (LSTB_SUMMEN). Dieser Wert kommt aus der LStB (Nr. 27) und geht in dasselbe Modell-Feld, das dann an E2001403 gebunden ist. Der Import-Pfad ist korrekt: ArbnAnteilArblVers ist der personen-individuelle LStB-Wert, der in das gemeinsame Sum-Feld fließt.

---

## Anhang: XSD-Kardinalitäten der VOR-Kinder

| Container | maxOccurs | Person-Kind | Bemerkung |
|-----------|-----------|-------------|-----------|
| AVor | 2 | Person | Altersvorsorge (wie erwartet person-multipliziert) |
| Beitr_g_KV_PV_Inl | 2 | Person | Basis KV/PV (wie erwartet person-multipliziert) |
| Beitr_p_KV_PV_Inl | 2 | Person | Private KV/PV (wie erwartet person-multipliziert) |
| Beitr_g_p_KV_PV_Ausl | 2 | Person | Ausländische KV/PV (wie erwartet person-multipliziert) |
| Stfr_AG_Zusch | 2 | Person | Steuerfreie AG-Zuschüsse (wie erwartet person-multipliziert) |
| Weit_Sons_VorAW | **1** | **Kein** | **HIER: NICHT person-multipliziert** |
| Uebern_KV_PV_Beitr | 10 | Kein Person | Mitversicherte (eigene Struktur) |
| Erg_Ang | 2 | Person | Ergänzungsangaben (wie erwartet person-multipliziert) |

**Fazit:** Weit_Sons_VorAW ist die einzige Kategorie unter VOR, die nicht person-multipliziert ist (maxOccurs=1, kein Person-Kind). Die fünf Sum-Kz sind shared amounts. Der Pers-Zweig deckt nur ALV ab.