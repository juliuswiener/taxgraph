# KV/PV-Kz Folge-Adjudikation: Person-B-Blockade weg, Feldaufteilung bleibt

**Datum:** 2026-08-05
**Status:** read-only Analyse, kein Code geändert. Baut auf kvpv_kz_2026-07-31.md auf.

---

## 1. Kz-Tabelle (alle KV/PV-Kz aus Analyse)

Geprüft am amtlichen E10-2025.xsd, Pfad via `xsd_verify.walk`, Dokumentation wörtlich aus `xs:documentation`.

### Kz mit Pfad + <Person>-Diskriminator

| Kz | Pfad | maxOccurs Container | Person-Pflicht? | xs:documentation (gekürzt) |
|----|------|--------------------|-----------------|---------------------------|
| E2001203 | E10/VOR/Beitr_g_KV_PV_Inl/AN/E2001203 | Beitr_g_KV_PV_Inl maxOccurs=2 | ✓ (Pflichtkind Person) | "Arbeitnehmerbeiträge zu Krankenversicherungen laut Nr. 25 der Lohnsteuerbescheinigung" |
| E2001505 | E10/VOR/Beitr_g_KV_PV_Inl/AN/E2001505 | selber Container | ✓ | "Arbeitnehmerbeiträge zu sozialen Pflegeversicherungen laut Nr. 26 der Lohnsteuerbescheinigung" |
| E2001805 | E10/VOR/Beitr_g_KV_PV_Inl/And_Pers/E2001805 | selber Container | ✓ | "Beiträge zu Krankenversicherungen – ohne Beiträge, die in Zeile E2001203 geltend gemacht werden – (z. B. bei Rentnern, bei freiwillig gesetzlich versicherten Selbstzahlern)" |
| E2002105 | E10/VOR/Beitr_g_KV_PV_Inl/And_Pers/E2002105 | selber Container | ✓ | "Beiträge zu sozialen Pflegeversicherungen – ohne Beiträge, die in Zeile E2001505 geltend gemacht werden – (z. B. bei Rentnern, bei freiwillig gesetzlich versicherten Selbstzahlern)" |
| E2003104 | E10/VOR/Beitr_p_KV_PV_Inl/E2003104 | Beitr_p_KV_PV_Inl maxOccurs=2 | ✓ | "Beiträge zu privaten Krankenversicherungen (nur Basisabsicherung, keine Wahlleistungen)" |
| E2003202 | E10/VOR/Beitr_p_KV_PV_Inl/E2003202 | selber Container | ✓ | "Beiträge zu Pflege-Pflichtversicherungen" |

### Erstattungs-Kz

| Kz | Pfad | Container maxOccurs=2? | xs:documentation |
|----|------|-----------------------|-----------------|
| E2001605 | E10/VOR/Beitr_g_KV_PV_Inl/AN/E2001605 | ✓ | "Von der Kranken- und / oder sozialen Pflegeversicherung erstattete Beiträge" (AN) |
| E2002207 | E10/VOR/Beitr_g_KV_PV_Inl/And_Pers/E2002207 | ✓ | "Von der Kranken- und / oder sozialen Pflegeversicherung erstattete Beiträge" (And_Pers) |
| E2003302 | E10/VOR/Beitr_p_KV_PV_Inl/E2003302 | ✓ | "Von der privaten Kranken- und / oder Pflege-Pflichtversicherung erstattete Beiträge" |

### Zusatz-Kz (nicht in Adjudikation, aber relevant)

| Kz | Pfad | Hinweis |
|----|------|---------|
| E2001405 | E10/VOR/Beitr_g_KV_PV_Inl/AN/E2001405 | KV-Anteil ohne Krankengeld-Anspruch (Subset) |
| E2002005 | E10/VOR/Beitr_g_KV_PV_Inl/And_Pers/E2002005 | KV-Anteil MIT Krankengeld (And_Pers-spezifisch) |
| E2003502 | E10/VOR/Beitr_p_KV_PV_Inl/WL_Zvers/E2003502 | Wahlleistungen über Basisabsicherung hinaus (separates Feld, nicht basis_kv_pv) |

### Person-B-Container: ✓ bestätigt

Alle drei Beitragswege (`Beitr_g_KV_PV_Inl`, `Beitr_p_KV_PV_Inl`) haben:
- `maxOccurs=2` (Person A + Person B)
- `<Person>` als Pflicht-Kind (`minOccurs=1 maxOccurs=1`)
- Damit exakt derselbe Mechanismus wie Anlage R (E10/R, in Zone C `test_differential_zone_c` nachgewiesen für rentner_jahresrente_partner → E1800301 im person_b-Bucket mit PersonB-Diskriminator)

**Der Writer-Blocker aus der Juli-Analyse ist weg.** Kein Code im elster_xml.py mehr nötig. Die Kz-Instanzierung für Person B läuft über den bestehenden person_b-Bucket und `_bestimme_person_container` + `instanz={container: 1}`.

---

## 2. KV/PV-Trennung: Analyse bestätigt

Die Analyse sagt: `basis_kv_pv` summiert KV und PV, das Schema führt zwei getrennte Kz pro Beitragsweg.

**Prüfung bestanden.** Das XSD zeigt klar getrennte Kz-Paare:
- AN: E2001203 (KV) + E2001505 (PV)
- And_Pers: E2001805 (KV) + E2002105 (PV)
- Privat: E2003104 (KV) + E2003202 (PV)

**LStB-Import stellt die Trennung bereit, ebnet sie aber ein:**
`produkt/import/vast_mapping.py` Zeilen 77-80:
```python
LSTB_SUMMEN = {
    "basis_kv_pv": (
        ("ArbnAnteilKrankVers", "Arbeitnehmerbeiträge zur gesetzlichen Krankenversicherung"),
        ("ArbnAnteilPflegVers", "Arbeitnehmerbeiträge zur sozialen Pflegeversicherung"),
    ),
}
```
`aus_lstb()` (Z.138-145) summiert beide Beleg-Felder in ein Feld `basis_kv_pv`. Die Information KV_Anteil / PV_Anteil ist im Beleg vorhanden (`ArbnAnteilKrankVers` ≠ `ArbnAnteilPflegVers`), geht aber bei der Aggregation verloren — exakt wie die Analyse beschreibt.

Der Import müsste entsprechend angepasst werden, wenn die Felder getrennt werden.

---

## 3. Aufwandsschätzung (revidiert)

### Analyse Juli schätzte 6-8h + 3 neue Oberflächenfelder.

Die Analyse nannte drei Blocker:
1. **Person-B-Writer** → ❌ **Blockade weg** (Proven in Zone C: person_b-Bucket + instanz-Achse funktioniert)
2. **KV/PV-Feldaufteilung** → ⚠️ Unverändert: ein Summenfeld kann nicht auf zwei Kz verteilt werden
3. **Versicherungsart-Unterscheidung** → ⚠️ Unverändert: AN ↔ And_Pers ↔ privat braucht Nutzerangabe

### Revidierte Schätzung: 5-7h (minus ~1h Writer-Arbeit, nichts Grundsätzliches billiger)

| Schritt | Aufwand | Begründung |
|---------|---------|------------|
| Neue Store-Felder `basis_kv`, `basis_pv` für Person A (statt `basis_kv_pv`-Gesamtsumme) | 1h | inkl. Bindung + Migration bestehender `basis_kv_pv`-Events |
| Versicherungsart-Feld (`versicherungsart`: gesetzlich_AN / gesetzlich_Rentner / privat) | 1h | inkl. Bindung + Verzweigungslogik in est_mapping |
| est_mapping KV/PV-Verzweigung (Klasse f) | 1h | Art-Feld steuert AN/And_Pers/privat, je Paar KV+PV Kz |
| Person-B-Varianten (`basis_kv_partner`, `basis_pv_partner`, `versicherungsart_partner`) | 1.5h | Person-Multiplikation (Klasse g): Partner-Versionen jedes neuen Felds |
| Erstattungs-Kz (E2001605/E2002207/E2003302) | 0.5h | separates Feld oder Third-Kz je Verzweigung; fraglich ob user-facing |
| Import-Anpassung vast_mapping.py | 0.5h | LStB-Import: KV/PV getrennt ausgeben | 0.5h |
| Tests (Ring, Differential, Bindung) | 1.5h | _partner-Varianten + Verzweigungs-Tests + Import-Regression |
| **Summe** | **~6h** | |

**Einsparung:** Person-B-Writer war in der Juli-Analyse der letzte Blocker ("❌ Bau blockiert"). Die Revision streicht diesen Posten komplett — ca. 1h eingespart. Der Kernaufwand (Feldaufteilung + Versicherungsart-Unterscheidung) bleibt gleich.

### Was die Analyse unterschätzt hat:
1. **Migrieren bestehender `basis_kv_pv`-Events im Store** — es gibt echte Nutzerdaten (zumindest in der Test-Suite und ggf. in persistenten Fällen), die von einem Summenfeld auf zwei getrennte Felder umgestellt werden müssen. Die Analyse nannte das nicht.
2. **Erstattungs-Kz** — die Juli-Analyse listete sie, aber ohne Implementierungs-Aufwand. Sie sind echter Teil der Deklaration (offsetieren die Beiträge), nicht optional.
3. **Import-Naht** — `vast_mapping.py` muss beim Übergang von einem Summen- auf zwei Einzelfelder angepasst werden. Das ist billig, aber nicht null.

---

## 4. Entscheidungen für Julius

### A) Architekturentscheid: Summenfeld behalten oder aufteilen?

Drei Optionen:

**Option 1 — Summenfeld mit Verzweigung (billig, verlustbehaftet)**
- `basis_kv_pv` bleibt ein Summenfeld
- est_mapping schreibt KV+Kz (z.B. E2001203) mit dem Gesamtwert
- PV-Kz (z.B. E2001505) wird = 0 geschrieben (kein separater Wert)
- ⚠️ **Problem:** falsche Deklaration. Das Finanzamt bekommt die PV-Beiträge nicht. Bei Sofware-gestützter Prüfung (LStB-Abgleich) fällt das auf. Nicht ERiC-konform.
- **Nicht empfohlen.**

**Option 2 — Aufteilen in `basis_kv` + `basis_pv` (empfohlen)**
- Zwei Cent-Felder statt einem
- LStB-Import kann beide getrennt befüllen (Daten sind da!)
- Manuelle Eingabe verlangt zwei Felder (Mehraufwand für Nutzer, aber sachlich richtig)
- Person-B: `basis_kv_partner` + `basis_pv_partner`
- **Aufwand:** ~6h

**Option 3 — Aufteilen mit Automatik (Premium)**
- Wie Option 2, plus: bei AN mit LStB-Import automatisch KV/PV aufteilen
- Bei manueller Eingabe: KV/PV-Verhältnis aus LStB-Import des Vorjahres schätzen (Fragwürdig)
- **Aufwand:** ~8h (zusätzliche Heuristik)
- **Nutzen fraglich:** der Nutzer kann den Bescheid ablesen.

### B) Versicherungsart: implizit oder explizit?

**Implizit (weniger Felder):**
- AN ↔ And_Pers aus `kein_gewinn` + Existenz von `vor_an_anteil_rv` ableiten
- privat: braucht negatives Signal (LStB-BeitrPrKrankVers = NICHT_GEMAPPT)

**Explizit (sicherer):**
- Neues Enum-Feld `vorsorge_versicherungsart` = `gesetzlich_an` / `gesetzlich_rentner` / `privat`
- fail-closed: ohne Angabe → kein Kz (nicht_deklariert), → Veranlagung UNVOLLSTÄNDIG?
- Nutzer muss sagen, wie er versichert ist

**Empfehlung: explizit.** Implizite Ableitung ist eine stille Fehlerquelle (Rentner mit LStB aus Teilzeit-Job? Privatpatient mit Mini-Job?). Einmalige Angabe beim Profil, kein jährlicher Overhead.

### C) Erstattungs-Kz: zeigen oder verstecken?

Die Erstattungs-Kz sind Pflicht im Schema (offsetieren den Abzug). Werden sie nie gefüllt, deklariert man einen zu hohen Abzug. Das sind die Werte, die die Krankenkasse jährlich bescheinigt — nicht direkt vom Nutzer erfragbar.

**Empfehlung:** Als separate Nutzerfelder `kv_erstattung` / `pv_erstattung` (mit _partner) — bei LStB-Import aus dem Beleg befüllbar, sonst manuelle Eingabe. Der Betrag steht auf der Jahresbescheinigung der Krankenkasse.

---

## 5. Zusammenfassung

| Frage | Antwort |
|-------|---------|
| Person-B-Blockade? | **Weg.** Zone C beweist: person_b-Bucket + instanz-Achse funktioniert. Kein Writer-Code nötig. |
| KV/PV-Trennung nötig? | **Ja.** Schema hat zwei Kz (KV+PV), Import trennt sie, Nutzerfeld summiert. Muss aufgeteilt werden. |
| Versicherungsart-Weiche nötig? | **Ja.** AN/And_Pers/privat sind alternative Kz-Pfade im Schema. |
| Kz richtig? | **Ja.** Alle 6 Dokumentationen aus xs:documentation zitierbar. Pfade via `xsd_verify.walk` bestätigt. |
| Revidierter Aufwand? | **~6h** (5-7h Band), minus ~1h Person-B-Writer gegenüber Juli-Schätzung. |
| Mindest-Entscheid? | (A) Summenfeld aufteilen oder verlustbehaftet lassen, (B) Versicherungsart implizit/explizit |