# Weitere Vorsorgeaufwendungen § 10 Abs. 1 Nr. 3a — Adjudikation 2026-08-05

**Datum:** 2026-08-05
**Status:** READ-ONLY
**HEAD:** c1757a2 (Arbeitsbaum dreckig — dev-1 Schritt 3 in Arbeit)
**Auftrag:** Prüfung von `weitere_vorsorgeaufwendungen` und `_partner`

---

## 1. Vollständige Kz-Gruppe unter VOR/Weit_Sons_VorAW

Der XSD-Teilbaum (via `xsd_verify.walk` am E10-2025.xsd) gliedert sich in zwei Pfade:

### Pfad A: A_B_LP → 5 Versicherungskategorien, je mit Einz(2) + Sum

| XSD-Container | Zweck | Sum-Kz | Einz-Kz (Bezeichnung/Betrag) |
|--------------|-------|--------|-----------------------------|
| AL_Vers | Arbeitslosenversicherung (Nr. 27 LStB) | E2001403 (Summe) | E2001401 "Bezeichnung", E2001402 "Betrag" |
| ErwU_BU_Vers | Erwerbs-/Berufsunfähigkeitsversicherung | E2001503 (Summe) | E2001501 "Bezeichnung", E2001502 "Betrag" |
| U_HP_Ris_Vers | Unfall-/Haftpflicht-/Risikolebensversicherung | E2001803 (Summe) | E2001801 "Bezeichnung", E2001802 "Betrag" |
| RV_m_WR_KapLV | Rentenversicherung mit Überschussbeteiligung/kapitalbildend (Alt) | E2001903 (Summe) | E2001901 "Bezeichnung", E2001902 "Betrag" |
| RV_o_WR_o_AV | Rentenversicherung ohne Überschussbeteiligung/o. AV (Alt) | E2002003 (Summe) | E2002001 "Bezeichnung", E2002002 "Betrag" |

Alle 5 Container: `minOccurs=0`, `maxOccurs=1`. Alle Sum-Kz: xs:documentation "Summe".

**Wichtig:** A_B_LP selbst hat KEIN Summen-Kz, das alle 5 Kategorien aggregiert. Das Schema führt jede Versicherungsart einzeln.

### Pfad B: Pers → Person-spezifisch

| Kz | Pfad | xs:documentation |
|----|------|----------------|
| E2004403 | `VOR/Weit_Sons_VorAW/Pers` (maxOccurs=2) | "Arbeitnehmerbeiträge zur Arbeitslosenversicherung laut Nr. 27 der Lohnsteuerbescheinigung" |

Das Pers-Kz ist der LStB-gestützte Pfad (nur AL_Vers, wie Nr. 27). Die A_B_LP-Einz/Sum sind der manuelle/explizite Pfad.

---

## 2. Gesetzeskontext — § 10 Abs. 1 Nr. 3a EStG

Quelle: `sources/gesetze-im-internet/estg_p10_2026-07-11.txt`, Zeile 10:

> "3a. Beiträge zu Kranken- und Pflegeversicherungen, soweit diese nicht nach Nummer 3 zu berücksichtigen sind; Beiträge zu Versicherungen gegen Arbeitslosigkeit, zu Erwerbs- und Berufsunfähigkeitsversicherungen, die nicht unter Nummer 2 Satz 1 Buchstabe b fallen, zu Unfall- und Haftpflichtversicherungen sowie zu Risikoversicherungen, die nur für den Todesfall eine Leistung vorsehen"

Die 5 XSD-Kategorien unter A_B_LP decken das Gesetz ab (ohne den KV/PV-Rest, der in die Basis gehört):
1. AL_Vers → Arbeitslosigkeit ✅
2. ErwU_BU_Vers → Erwerbs-/Berufsunfähigkeit ✅
3. U_HP_Ris_Vers → Unfall/Haftpflicht/Risikoleben ✅
4. RV_m_WR_KapLV → Altverträge (Riester-/Rürup-artig vor 2005) ✅
5. RV_o_WR_o_AV → Altverträge (ohne Überschuss) ✅

---

## 3. Verdacht bestätigt: E2001403 ist NUR Arbeitslosenversicherung — nicht das Aggregat

Die Bindung (Z.882) notiert korrekt: `"Kz E2001403 (E10/VOR/Weit_Sons_VorAW/A_B_LP/AL_Vers/Sum/E2001403) belegt"`. Aber die Zuordnung des einen Feldes `weitere_vorsorgeaufwendungen` auf EIN Kategorie-Kz ist falsch — es müssten 5 Kz sein.

Der Fragetext verspricht: "private Zusatz-Krankenversicherung, Haftpflicht, Arbeitslosen-, Unfall- oder Risikolebensversicherung" — das sind 4 der 5 Kategorien. Die beiden RV-Alt-Kategorien sind im Fragetext nicht genannt, gehören aber rechtlich dazu (§ 10 Abs. 1 Nr. 3a letzter Teilsatz).

**Das Schema hat KEIN Aggregat-Kz für § 10 Abs. 1 Nr. 3a** — jede Kategorie wird separat deklariert.

---

## 4. Urteil: MODELLPROBLEM — aber niedrige Priorität

### MODELLPROBLEM — das Feld bildet 5 XSD-Kz nicht ab

Um die Deklaration korrekt zu füllen, bräuchten wir **5 Store-Felder** (plus _partner = 10):

1. `vorsorge_arbeitslosenversicherung` → AL_Vers/Sum/E2001403
2. `vorsorge_erwerbsunfaehigkeitsversicherung` → ErwU_BU_Vers/Sum/E2001503
3. `vorsorge_unfall_haftpflicht` → U_HP_Ris_Vers/Sum/E2001803
4. `vorsorge_rv_alt_mit_wr` → RV_m_WR_KapLV/Sum/E2001903
5. `vorsorge_rv_alt_ohne_wr` → RV_o_WR_o_AV/Sum/E2002003

plus `Pers/E2004403` als LStB-Import-Pfad (optional, ersetzt AL_Vers).

**Aufwand:** ~3h (5+5 Felder, Bindung, Ring-Integration, est_mapping Verzweigung? Nein — jedes Kz ist 1:1 mit einem Feld, keine Verzweigung).

### Priorität: NIEDRIG — steuerlich meist wirkungslos

§ 10 Abs. 4 S. 4 regelt: Übersteigen die BASIS-Vorsorgeaufwendungen (Nr. 3) den Höchstbetrag (2.800/1.900 €), sind diese voll abziehbar **und ein Abzug von Vorsorgeaufwendungen im Sinne des Absatzes 1 Nummer 3a scheidet aus** (Abs. 4 S. 4 a.E.).

Für die typischen Fälle (gesetzlich Versicherte mit Basisbeitrag > 2.800 €) ist der §10-Nr.3a-Abzug gesetzlich ausgeschlossen → die fehlerhafte Deklaration auf AL_Vers-Kz ist steuerlich wirkungslos. Nur bei niedrigen Basisbeiträgen (< 2.800 €) oder wenn § 10 Abs. 4 S. 4 nicht greift (private Basis < HB) hätte die Aufteilung steuerliche Wirkung.

### Empfehlung: UNGEBUNDEN lassen — nicht auf E2001403 binden

Zwei Alternativen, beide von mir im ersten Report übersehen:

**Option A — UNGEBUNDEN (kz_status: offen, elster_kz: null, Grund = dieser Befund).** Empfohlen.
- Das Feld rechnet weiter im Ring (der den § 3a-Abzug begrenzt oder ausschließt).
- Wird NICHT deklariert → over-tax-safe (der Abzug fehlt in der XML, aber er wäre ohnehin meist gesetzlich ausgeschlossen).
- § 150 Abs. 2 AO: keine falsche Tatsachenerklärung, weil wir nichts auf das falsche Kz schreiben.
- Erfordert: den elster_kz_grund der Bindung aktualisieren ("MODELLPROBLEM: 1 Feld ≠ 5 Kategorie-Kz; E2001403 nur AL_Vers, kein Aggregat-Kz. Ungebunden, weil keine Kategorie-Kz-Zuordnung für das Aggregat.").

**Option B — auf E2001403 binden (nicht empfohlen).**
- Deklariert Haftpflicht+Risikoleben+etc. als Arbeitslosenversicherung → falsche Tatsachenerklärung (§ 150 Abs. 2 AO).
- Zwar steuerlich meist wirkungslos (s.o.), aber eine falsche Deklaration bleibt falsch.
- Dieselbe Bauart wie § 22 Nr. 3 Block-2-Fehler.

**Fazit:** Das Feld bleibt ungebunden, bis wir die 5 Kategorie-Kz aufteilen. Das ist ehrlicher als eine falsche Zuordnung — und billiger (kein Code).

---

## 6. LStB-Import-Pfad: E2004403 ist ein eigener, sauber bindbarer Fall

Der Import in `vast_mapping.py` (Z. 78-81) mappt `ArbnAnteilArblVers` (Nr. 27 LStB) → Store-Feld `weitere_vorsorgeaufwendungen`. Das ist **tatsächlich reine Arbeitslosenversicherung** — für Import-Nutzer enthält das Feld exakt das, was E2004403 (Pers-Pfad) erwartet.

### Bindbarkeit

**Import-Fall — BINDBAR auf E2004403:**
- Für AN mit LStB-Import: der Feldwert = `ArbnAnteilArblVers` = AL_Vers-Beiträge.
- E2004403 (`VOR/Weit_Sons_VorAW/Pers`) xs:documentation: "Arbeitnehmerbeiträge zur Arbeitslosenversicherung laut Nr. 27 der Lohnsteuerbescheinigung" — exakter Match.
- `minOccurs=0`, `maxOccurs=2` (Person A/B) → passt.
- Kz-Wert: Store-Cent → ceiling (Abzugskz).

**Allerdings:** eine Bindung müsste zwischen Import- und Handeingabe unterscheiden können. Technisch möglich: der Snapshot, den `deklariere()` bekommt, enthält `{..., "herkunft": {"herkunft": "edaten", ...}}` pro Feld — eine Verzweigung "wenn herkunft == edaten → E2004403, sonst nichts" wäre mechanisch machbar. Aber `deklariere()` reicht die Herkunft aktuell nicht an die Kz-Entscheidung weiter; der Code würde eine Schnittstellen-Erweiterung brauchen. Für ein steuerlich meist wirkungsloses Feld ist das Over-Engineering.

**Praktisch:** bei Import-Nutzern füllt `aus_lstb()` das Feld, der Wert ist dann "weitere Vorsorge, die AL_Vers ist". Bei Handeingabe kommt der Wert vom Nutzer = Aggregat. Wir müssten das herkunftsabhängig machen — das ist Over-Engineering für ein meist wirkungsloses Feld.

**Empfehlung:** E2004403 als zukünftige Bindung notieren, aber nicht jetzt einbauen. Wenn wir irgendwann die 5 Felder aufteilen, bekommt `vorsorge_arbeitslosenversicherung` sein 1:1-Kz E2004403 (Import) + E2001403 (manuell/Sum). Bis dahin bleibt alles ungebunden — so wie der Status quo.

### Änderung: elster_kz_grund in der Bindung aktualisieren

Aktuell (Z. 882): `"Kz E2001403 (E10/VOR/Weit_Sons_VorAW/A_B_LP/AL_Vers/Sum/E2001403) belegt 2026-08-05 am amtlichen E10-2025.xsd. Bindung noch offen."`

Neu: `"MODELLPROBLEM: 1 Feld (Aggregat) ≠ 5 Kategorie-Kz im XSD (AL_Vers/E2001403, ErwU_BU/E2001503, U_HP_Ris/E2001803, RV_m_WR/E2001903, RV_o_WR/E2002003). Einzige bindbare Teilmenge: LStB-Import (Nr. 27) auf E2004403 (Pers-Pfad). Feld bleibt ungebunden bis Aufteilung."`

Diesen Eintrag nimmt dev-1 oder ein späterer Schritt mit, wenn die Aufteilung kommt. Nicht jetzt.