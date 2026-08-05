# Kapital/Mitunternehmer/Par.33a - Restblock-Adjudikation 2026-08-05

**Datum:** 2026-08-05
**Status:** READ-ONLY
**HEAD:** 6563f50 (Arbeitsbaum: dev-1 est_mapping.py + tests)

---

## Par.15 Mitunternehmer (4 Felder): gewinnanteil, verguetung_taetigkeit, verguetung_darlehen, verguetung_ueberlassung

### Teilbaum G/Gew/M_Unt - vollstaendig abgelaufen

| Pfad | Kz | xs:documentation | minOcc/maxOcc |
|------|----|-----------------|---------------|
| G/Gew/M_Unt/Beteil/E0800601 | E0800601 | "genaue Bezeichnung der Gesellschaft" | 0/1 |
| G/Gew/M_Unt/Beteil/E0800602 | E0800602 | "Betrag" | 0/1 |
| G/Gew/M_Unt/Beteil/E0800908 | E0800908 | "Finanzamt" | 0/1 |
| G/Gew/M_Unt/Beteil/E0801008 | E0801008 | "Steuernummer" | 0/1 |

Das Schema fuehrt eine **Beteiligungs-Zeile** pro Mitunternehmerschaft (Bezeichnung + Betrag + Finanzamt + Steuernummer), KEINE Aufteilung nach Verguetungsarten (Taetigkeit, Darlehen, Ueberlassung).

### Urteil: DOKUMENTIERTE AGGREGATION (Klasse a)

Die vier Eingabe-Felder sind Ring-Inputs. Sie gehen in den Gesamtgewinn ein, der via E0800502 (G/Gew/Ges_Fest/Sum) deklariert wird. Das Schema kennt die drei Verguetungsarten nicht einzeln - sie sind im "Betrag" der Mitunternehmerschaft enthalten.

**Kein eigenes Kz fuer die drei Sonderverguetungen** bestaetigt. Der Gesamtgewinn laeuft ueber Ges_Fest/Sum.

**Dokumentierte Aggregation** passt: wie E0703838 fuer Par.21-WK sind die Quell-Felder (4 Mitunternehmer-Felder) im dokumentiert-Bucket sichtbar, die Summe geht in Gesamtgewinn ein.

| Feld | Urteil | Begruendung |
|------|--------|-----------|
| gewinnanteil | DOKUMENTIERTE AGGREGATION | Geht in G/Gew/Ges_Fest/Sum/E0800502 ein |
| verguetung_taetigkeit | DOKUMENTIERTE AGGREGATION | Sonderverguetung, im Betrag der MU-Beteiligung aufgehend |
| verguetung_darlehen | DOKUMENTIERTE AGGREGATION | dito |
| verguetung_ueberlassung | DOKUMENTIERTE AGGREGATION | dito |

---

## Par.20 Kapital (2 Felder): kap_gewinn_sonstige, kap_gewinn_sonstige_partner

### 4-Topf-Modell gegen XSD-KAP

Unser Modell (je Topf ein Feld):
1. kap_gewinn_aktien -> Aktiengewinne -> E1900901 (drin in E1900701) OK
2. kap_verlust_aktien -> Aktienverluste -> E1901301 OK
3. kap_gewinn_sonstige -> Sonstige Gewinne -> KEIN passendes Kz
4. kap_verlust_sonstige -> Sonstige Verluste -> E1901201 OK
5. kap_kapitalertraege -> Total -> E1900701 "Kapitalertraege" OK

### Schema Kz in Betr_lt_StBesch (alle minOccurs=0, maxOccurs=1)

| Kz | xs:documentation (gekuerzt) | Gebunden | Deckt unser Feld? |
|----|---------------------------|----------|-------------------|
| E1900701 | "Kapitalertraege" (Total) | kap_kapitalertraege OK | - |
| E1900901 | "enthaltene Gewinne aus Aktienveraeusserungen" | kap_gewinn_aktien OK | - |
| **E1900904** | "enthaltene Einkuenfte aus Stillhalterpraemien und Gewinne aus Termingeschaeften" | NEIN | NEIN Unser Feld umfasst Fonds, Zertifikate etc. - breiter als Stillhalterpraemien |
| E1900804 | "enthaltene Gewinne aus bestandsgeschuetzten Alt-Anteilen" | NEIN | NEIN Nischenfall |
| E1901101 | "enthaltene Ersatzbemessungsgrundlage" | NEIN | NEIN Sonderfall |
| E1901201 | "Nicht ausgeglichene Verluste ohne Verluste aus Aktien" | kap_verlust_sonstige OK | - |
| E1901301 | "Nicht ausgeglichene Verluste aus Aktienveraeusserungen" | kap_verlust_aktien OK | - |

### Ist die Abbildung VERLUSTBEHAFTET?

**Ja - dreifach.** Das Schema hat drei Kz fuer Teilmengen der sonstigen Kapitalgewinne (Stillhalterpraemien E1900904, Alt-Anteile E1900804, Ersatzbemessung E1901101). Unser Feld kap_gewinn_sonstige ist breiter als jedes einzelne (laut Fragetext "Fonds oder Zertifikaten") und kann keinem eindeutig zugeordnet werden. Die Deklaration bleibt gueltig, weil E1900701 (total) korrekt summiert, aber die Sub-Aufteilung bleibt ungenutzt.

**Bindungsbegruendung korrekt:** Die urspruengliche "MODELL-MISMATCH" war richtig - nicht weil es kein Kz gaebe, sondern weil keines der drei Kz unser breites Feld deckt. Die drei Sub-Kategorien sind minOccurs=0 -> optional.

| Feld | Urteil | Begruendung |
|------|--------|-----------|
| kap_gewinn_sonstige | RING-INPUT (Modell-Mismatch bestaetigt) | Unser Feld breiter als E1900904/etc. Geht in E1900701 total auf. |
| kap_gewinn_sonstige_partner | RING-INPUT (Modell-Mismatch bestaetigt) | Dito Person B, via PARTNER_INSTANZ E1900701. |

---

## Par.33a_andere_einkuenfte_bezuege

**Urteil: RING-INPUT (kein Kz)**

Das Feld betrifft Par. 33a Abs. 1 S. 5: eigene Einkuenfte/Bezuege der unterstuetzten Person ueber 624 EUR mindern den Hoechstbetrag. Das E10 fragt diese Information unter E10/Sonst/Unterhalt/Einz/... ab - pro unterstuetzter Person gibt es Felder fuer Einkuenfte/Bezuege (E0181802 "Betrag", E0181801 "Beschreibung").

**Kein separates Kz** fuer "Einkuenfte der unterstuetzten Person" als Aggregat - es ist Teil der Anlage Unterhalt (bereits gebunden via p33a_unterhalt_aufwendungen -> E0120103 in Block 3).

p33a_andere_einkuenfte_bezuege ist Ring-Input. Geht in den Ring ein, der den Hoechstbetrag kuerzt.

---

## Zusammenfassung

| Feld | Bisher | Urteil | Kz | Begruendung |
|------|--------|--------|----|-----------|
| kap_gewinn_sonstige | OFFEN (Modell-Mismatch) | RING-INPUT | - | Modell-Mismatch bestaetigt. Kein passendes Kz. |
| kap_gewinn_sonstige_partner | OFFEN | RING-INPUT | - | Dito Person B. |
| p33a_andere_einkuenfte_bezuege | OFFEN | RING-INPUT | - | Ring-Input fuer Hoechstbetragskuerzung. |
| gewinnanteil | OFFEN | DOKUMENTIERTE AGGREGATION | - | Geht in G/Gew/Ges_Fest/Sum auf. |
| verguetung_taetigkeit | OFFEN | DOKUMENTIERTE AGGREGATION | - | Sonderverguetung, im Betrag aufgehend. |
| verguetung_darlehen | OFFEN | DOKUMENTIERTE AGGREGATION | - | dito |
| verguetung_ueberlassung | OFFEN | DOKUMENTIERTE AGGREGATION | - | dito |

**Bindbar: 0 von 7.** Keines der sieben Felder braucht ein eigenes neues Kz - sie sind entweder Ring-Input oder gehen in bestehende Kz ein.