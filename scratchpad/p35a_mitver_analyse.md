# §35a Mitveranlagung Zähler — Analyse & Backlog

**TASK:** `§35a Mitveranlagung Zähler` — noch nicht angefangen. Status: **nichts gebaut**.

## Was existiert
- §35a Haushaltsnahe Bindung: `bindung_sonder_agb_35a.yaml` (hh_minijob, hh_dienstleistungen, hh_handwerker_arbeitskosten, hh_rechnung_unbar etc.)
- §35a Slot-Felder im Ring: `HAUSHALT_35A` + `HAUSHALT_35A_ABS23` + `hh_rechnung_unbar`  
- §35a Accessor: `runner.catala_p35a_haushaltsnahe()` in api.py (gesamt Z.997-1001 + rentner Z.1523-1527)
- Zugriffs-Flag: `abs23_aus` = `f.get("hh_rechnung_unbar", {}).get("wert") is False` (Z.998 + Z.1524)

## Was FEHLT
### 1. NEW FIELD: `p35a_mitveranlagung` (bool, optional)
- Fragetext: "Haben Sie Zusammenveranlagung beantragt (mitveranlagung bei Ehegatten)?"
- elster_kz: null → dev-2-Folgeticket (kein XSD-Mapping vorhanden)
- vz_gueltigkeit: [2024, 2025, 2026]

### 2. ACCESSOR: `catala_p35a_mitveranlagung`
- Input: `haushaltsnahe_gesamt` (summierte Beträge aus bindung_sonder_agb_35a), `mitveranlagung` (bool)
- Output: `p35a_mitver_cent` (EURO-Äquivalent × Anrechnungsfaktor)
- Logik: wenn `mitveranlagung=true` → Abzug halbiert (jeder Ehegatte erhält nur die Hälfte des §35a-Betrags)

### 3. RING-WIRING
#### gesamt:
```python
# api.py nach §35a Abs.2/3-Weg-ii-Fix ~Z.1001
if f.get("p35a_mitveranlagung", {}).get("wert") is True:
    g["steuerermaessigungen"] += runner.catala_p35a_mitveranlagung({...}) // 2  # halber Abzug
else:
    g["steuerermaessigungen"] += p35a_gesamt   # voller Abzug
```

#### rentner_gesamt:
```python
# api.py nach §35a-Abschnitt ~Z.1527
# identisches Muster wie gesamt — halber Abzug bei zusammen/gesplittet
```

### 4. BINDUNGSTABELLE
NEU: `produkt/bindung/bindung_p35a_mitveranlagung.yaml`
- feld_id: `p35a_mitveranlagung` (enum: ja/nein)
- anker_ref: §35a Abs.1 S.2 i.V.m. §26b EStG
- frage: "Zusammenveranlagung für §35a Abzug?"

### 5. TESTS
- Erreichbarkeit: POST `p35a_mitveranlagung=true/false` → 201
- Ring-Differential: zusammen MIT §35a (halber Abzug) vs OHNE → Differenz 50% des §35a-Werts

## DEV-4 TODO-LISTE
1. [ ] Bindingstabelle erstellen (`bindung_p35a_mitveranlagung.yaml`)
2. [ ] Accessor `catala_p35a_mitveranlagung` in runner.py implementieren
3. [ ] Feld an GESAMT_FREIBETRAEGE + RENTNER_FELDER anhängen
4. [ ] Ring-Wiring (gesamt + rentner) für halben §35a-Abzug bei Mitveranlagung
5. [ ] golden/ cases erstellen (§35a zusammen vs einzel)
6. [ ] Unit-Tests (runner.py `catala_p35a_mitveranlagung` Test-Suite)
7. [ ] Ring-Level-Differential-Tests (HTTP-e2e über API)

## DEV-2 REVIEW-HILFE (bereits erledigt)
- Keine Änderung an existierendem §35a-Code notwendig
- Neue Bindung isoliert (nur `p35a_mitveranlagung` boolean)
- Accessor folgt Pattern von `catala_p10_kv_pv` (bool-Gated Accessor mit halbiertem Wert)
