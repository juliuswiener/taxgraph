# KV/PV Versicherungsart — Vorarbeit für Schritt 3

**Datum:** 2026-08-05
**Status:** READ-ONLY, kein Code. Auftrag main.

---

## (1) Enum-Design — Versicherungsart

### Drei Wege im XSD → drei Enum-Werte

Geprüft am E10-2025.xsd via `xsd_verify.walk`:

| Wert-Feld | XSD-Pfad (VOR/…) | Kz KV | Kz PV | xs:documentation |
|-----------|------------------|-------|-------|-----------------|
| gesetzlich (AN) | `Beitr_g_KV_PV_Inl/AN` | E2001203 | E2001505 | "Arbeitnehmerbeiträge zu Krankenversicherungen laut Nr. 25 der Lohnsteuerbescheinigung" |
| gesetzlich (And_Pers) | `Beitr_g_KV_PV_Inl/And_Pers` | E2001805 | E2002105 | "Beiträge zu Krankenversicherungen – ohne Beiträge, die in Zeile E2001203 geltend gemacht werden – (z. B. bei Rentnern, bei freiwillig gesetzlich versicherten Selbstzahlern)" |
| privat | `Beitr_p_KV_PV_Inl` | E2003104 | E2003202 | "Beiträge zu privaten Krankenversicherungen (nur Basisabsicherung, keine Wahlleistungen)" |

### Enum-Wert "gesetzlich_rentner" ist ZU ENG

Die xs:documentation für And_Pers nennt:
- Rentner
- freiwillig gesetzlich versicherte Selbstzahler (Freiberufler, Selbstständige, Beamte mit freiwilliger GKV, etc.)

Vorschlag:
```
versicherungsart (enum):
  - gesetzlich_an           # Arbeitnehmer mit sozialversicherungspflichtiger Beschäftigung, LStB-Daten
  - gesetzlich_freiwillig   # Rentner + freiwillig gesetzlich Versicherte (And_Pers-Semantik)
  - privat                  # privat Krankenversicherte (auch Arbeitnehmer mit PKV)
```

Fragetext_laie: "Wie bist du krankenversichert?"
hilfe_kurz: "Gesetzlich als Arbeitnehmer (Beiträge stehen auf der Lohnsteuerbescheinigung), gesetzlich als Rentner oder freiwillig Selbstzahler (keine Lohnsteuerbescheinigung), oder privat versichert."

### Können MEHRERE Wege gleichzeitig aktiv sein? → JA im XSD, aber fachlich NEIN für Basisabsicherung.

Die Container `Beitr_g_KV_PV_Inl` und `Beitr_p_KV_PV_Inl` sind BEIDE in derselben `xs:sequence` unter `VOR` — nicht in einer `xs:choice`. Alle drei Sub-Pfade (AN, And_Pers, privat) sind optional (`minOccurs=0`), kein xs:choice schliesst sie aus. Das XSD erlaubt also technisch, dass alle drei Pfade gleichzeitig gefüllt werden.

**Fachlich:** ein Steuerpflichtiger hat genau EINE Basisabsicherung (gesetzlich ODER privat). Die `WL_Zvers`-Elemente unter `Beitr_p_KV_PV_Inl` decken darüberhinausgehende Wahlleistungen ab (eigenes Kz E2003502) — das ist der Fall "Angestellter mit privater Zusatzabsicherung", der aber NICHT in den drei Basis-Kz landet.

**Schlussfolgerung:** ein einzelnes Enum `versicherungsart` ist DAS RICHTIGE Modell für die Basis-KV/PV-Kz. Die Basisabsicherung ist exklusiv (genau ein Weg). Kein Set/Mehrfachauswahl nötig.

### PARTNER-Variante

Spiegelbildlich: `versicherungsart_partner` mit denselben drei Werten. Steuert die Partner-Kz in `PARTNER_VERZWEIGUNG`.

---

## (2) Verzweigungs-Mechanik — VERZWEIGUNG+Klasse g×f passt 1:1

est_mapping.py hat zwei relevante Mechanismen:

**Zeile 88-110 — VERZWEIGUNG (Klasse f):**
```python
VERZWEIGUNG = {
    "rentner_jahresrente": {"art_feld": "rentner_renten_art", "kz": {
        "gesetzliche_rente": "E1800301", "berufsstaendische_versorgung": "E1800301",
        "private_basisrente": "E1800301", "private_leibrente": "E1801601",
        "sonstige_leibrente": "E1803102"}},
    ...
}
```
Ein Feld → N Kz, je nach Art-Feld-Wert. Der Code (Z. 263-276) prüft: (a) Art-Feld bestätigt? → fail-closed. (b) Art-Wert in cfg.kz? → Kz zuweisen. (c) Sonst → nicht_deklariert.

**Zeile 136-145 — PARTNER_VERZWEIGUNG (Klasse g×f):**
```python
PARTNER_VERZWEIGUNG = {
    "rentner_jahresrente_partner": {"art_feld": "rentner_renten_art_partner", "kz": {
        "gesetzliche_rente": "E1800301", ...}},
    ...
}
```
Selbe Logik, Ausgabe in `person_b`-Bucket.

### KV/PV-Verzweigung passt exakt

Basis KV → 3 mögliche Kz (E2001203/E2001805/E2003104), Pivot über versicherungsart
Basis PV → 3 mögliche Kz (E2001505/E2002105/E2003202), SELBE versicherungsart

→ ZWEI Einträge in VERZWEIGUNG, die auf DASSELBE art_feld schauen:
```python
"basis_kv": {"art_feld": "versicherungsart", "kz": {
    "gesetzlich_an": "E2001203", "gesetzlich_freiwillig": "E2001805", "privat": "E2003104"}},
"basis_pv": {"art_feld": "versicherungsart", "kz": {
    "gesetzlich_an": "E2001505", "gesetzlich_freiwillig": "E2002105", "privat": "E2003202"}},
```

**Vorteil:** beide lesen DASSELBE `snapshot["versicherungsart"]` — der Code ist dafür ausgelegt. Fail-closed bei unbestätigter Art = beide Felder gemeinsam unvollständig.

**PARTNER_VERZWEIGUNG analog:**
```python
"basis_kv_partner": {"art_feld": "versicherungsart_partner", "kz": {…}},
"basis_pv_partner": {"art_feld": "versicherungsart_partner", "kz": {…}},
```

**Kein neuer Mechanismus nötig.** VERZWEIGUNG + PARTNER_VERZWEIGUNG passen 1:1.

---

## (3) Ratsche 21 → realistische Zielzahl: 19, nicht 17

Geprüft: `tests/test_nicht_deklariert_inventar.py` Z. 151-158, Block "KV/PV-Vorsorge §10":

```python
"KV/PV-Vorsorge §10": ["basis_kv_pv", "basis_kv_pv_partner",
                        "weitere_vorsorgeaufwendungen", "weitere_vorsorgeaufwendungen_partner"],
```

Julius' Vermutung 21 → 17 (-4) basiert auf der Annahme, ALLE 4 Felder dieses Blocks bekämen ein Kz. **Das stimmt NICHT.**

### Beleg XSD: weitere_vorsorgeaufwendungen (E2001403) ist § 10 Abs. 1 Nr. 3a — ANDERER XSD-Zweig

| Feld | XSD-Pfad (walk) | Kz | ws:documentation |
|------|-----------------|----|------------------|
| `weitere_vorsorgeaufwendungen` | `E10/VOR/Weit_Sons_VorAW/A_B_LP/AL_Vers/Sum/E2001403` | E2001403 | "Arbeitnehmerbeiträge zur Arbeitslosenversicherung laut Nr. 27 der Lohnsteuerbescheinigung" (gekürzt; das Kz ist die Summe für AL-Vers) |
| Person-spezifisch | `E10/VOR/Weit_Sons_VorAW/Pers/E2004403` | E2004403 | "Arbeitnehmerbeiträge zur Arbeitslosenversicherung laut Nr. 27 der Lohnsteuerbescheinigung" (per Pers-Container) |

Der Pfad `VOR/Weit_Sons_VorAW` ist NEBEN `VOR/Beitr_g_KV_PV_Inl` — keine Überschneidung mit der Basis-KV/PV-Gruppe. Der Grund in der Bindung sagt selbst: "Bindung noch offen" — das ist echte Kz-Arbeit, die der KV/PV-Split NICHT anfasst.

### Ziel-Ratsche

| Änderung | OFFEN-Effekt |
|----------|-------------|
| `basis_kv_pv` gelöscht (ersetzt durch basis_kv + basis_pv) | −1 (aus OFFEN entfernt) |
| `basis_kv_pv_partner` gelöscht | −1 |
| `basis_kv` NEU, elster_kz: null, Grund="Kz via est_mapping VERZWEIGUNG" | ENDGUELTIG (kein OFFEN-Marker, wie rentner_jahresrente) |
| `basis_pv` NEU, analog | ENDGUELTIG |
| `basis_kv_partner` NEU, analog | ENDGUELTIG |
| `basis_pv_partner` NEU, analog | ENDGUELTIG |
| `weitere_vorsorgeaufwendungen` unverändert | bleibt OFFEN (Grund enthält "offen") |
| `weitere_vorsorgeaufwendungen_partner` unverändert | bleibt OFFEN |

**Netto-Reduktion: −2** → 21 → **19** (nicht 17).

Die 2 weiteren Felder bleiben OFFEN — das ist eine eigene Kz-Runde für E2001403/E2004403.

---

## (4) Erstattungs-Kz — optional, kein Blocker

Geprüft am XSD via Pfad-Walk:

| Kz | Pfad | minOccurs | maxOccurs | xs:documentation (gekürzt) |
|----|------|-----------|-----------|--------------------------|
| E2001605 | `VOR/Beitr_g_KV_PV_Inl/AN/E2001605` | 0 | 1 | "Von der Kranken- und / oder sozialen Pflegeversicherung erstattete Beiträge" |
| E2002207 | `VOR/Beitr_g_KV_PV_Inl/And_Pers/E2002207` | 0 | 1 | "Von der Kranken- und / oder sozialen Pflegeversicherung erstattete Beiträge" |
| E2003302 | `VOR/Beitr_p_KV_PV_Inl/E2003302` | 0 | 1 | "Von der privaten Kranken- und / oder Pflege-Pflichtversicherung erstattete Beiträge" |

Alle drei: **minOccurs=0** → optional. ERiC akzeptiert die Deklaration auch ohne diese Kz.

**Kein Blocker für Schritt 3.** Die Erstattungs-Kz sind ein separates Arbeitspaket (neue store-Felder `kv_erstattung` + `pv_erstattung` mit zugehöriger `_partner`-Variante) und können später implementiert werden. Der deklarierte Abzug ist ohne Erstattung zu hoch (die Beiträge werden nicht offsetiert), aber ERiC wird nicht ablehnen.

---

## Entscheidungen für main

1. **Enum-Auswahl bestätigen:** `gesetzlich_an` / `gesetzlich_freiwillig` / `privat`? Oder andere Namen?

2. **Ratsche:** 21 → 19 (nicht 17). `weitere_vorsorgeaufwendungen` bleibt OFFEN. OK?

3. **Erstattungs-Kz:** separate Runde. OK?

4. **Designdetails Enum:** Fragetext_laie oben passt?