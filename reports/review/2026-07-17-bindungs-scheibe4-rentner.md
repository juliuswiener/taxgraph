# Bindungs-Scheibe 4 — Rentner-Familie (Task #11, Paket A)

**Datei:** `produkt/bindung/bindung_rentner.yaml` (neu, additiv, NULL LLM).
**Gate:** `tests/test_bindungstabelle.py` **13/13 grün**; volle Paket-A-Suite **67/67 grün** (keine Regression).
**Anker:** alle 11 Binding-Anker literal in der Quelldatei + Tamper-verifiziert (test_d rot bei Manipulation, grün nach Revert).

## Abgedeckte Regeln (6) + p34 (nur Lücken)

| Regel | Norm | Bindungen (askable) | Lücken |
|---|---|---|---|
| p22_1_leibrente_besteuerungsanteil | § 22 Nr. 1 | rentner_jahresrente, **rentner_renten_beginn_jahr** | besteuerungsanteil_prozent (Kohorte), rentenfreibetrag_fixierung (State) |
| p24a_altersentlastungsbetrag | § 24a | rentner_alter_64_erfuellt | arbeitslohn/positive_andere_einkuenfte (geteilt/berechnet), prozentsatz (Kohorte), 2 gb |
| p33b_behinderten_pauschbetrag | § 33b Abs. 1–3 | rentner_grad_der_behinderung (20..100), rentner_hilflos_blind_taubblind | gdb_stufenfunktion, hilflos_override |
| p33b_hinterbliebenen_pauschbetrag | § 33b Abs. 4 | rentner_hinterbliebenenbezuege | hinterbliebenenbezuege_bewilligt |
| p33b_pflege_pauschbetrag | § 33b Abs. 6 | rentner_pflegegrad (1..5), rentner_gepflegter_hilflos | pflegegrad_staffel |
| p16_4_freibetrag | § 16 Abs. 4 | rentner_veraeusserungsgewinn, rentner_alter_55_oder_berufsunfaehig, rentner_freibetrag_erstmalig | auf_antrag |
| p34_fuenftel_ao_est | § 34 Abs. 1 | — | 2 berechnete Tarif-Slots + 2 gb (siehe offene Frage) |

**11 Bindungen, 16 Lücken.** Alle elster_kz = null + Grund (Rentner-Anlagen R/Behinderung/Betriebsveräußerung noch nicht XSD-gemappt; kein Rate-Mapping).

## Modellierungs-Entscheide

- **Kohorten-Parameter als Lücke, Jahr-Feld als askable:** `besteuerungsanteil_prozent` (§ 22) / `prozentsatz` (§ 24a) sind aus dem Renten-Beginn-Jahr bzw. Kohortenjahr abgeleitet — kein Laien-Feld. Der Laie gibt **rentner_renten_beginn_jahr** ("Seit welchem Jahr bekommst du deine Rente?", bereich 1955..2060) ein; die Regel schlägt den Prozentsatz in der Kohortentabelle nach. Das Jahr-Feld bindet an die Geltungsbedingung `besteuerungsanteil_aus_kohortentabelle` (Anker "maßgebenden Prozentsatz aus der nachstehenden Tabelle").
- **§ 24a-Bemessung:** `arbeitslohn`/`positive_andere_einkuenfte` = geteilte/berechnete Größen aus anderen Scheiben (kein Rentner-eigenes Feld); askable ist nur das Alters-Gate. `hoechstbetrag` = params-Key (auto-exempt).
- **§ 33b-Pauschbeträge = Enum-Staffel:** askable ist der Grad/Pflegegrad + die Merkzeichen-/Bezüge-Flags; die Staffelfunktion (Betrag je Stufe) ist Rechenlogik → Lücke.

## Offene Frage an Instructor (§ 34 ao-Betrag)

`p34_fuenftel_ao_est` hat als Inputs **zwei berechnete Tarif-Werte** (est_verbleibendes_zve, est_verbleibendes_zve_plus_fuenftel_ao) — beide Lücke. Der **ao-Einkünfte-BETRAG** (Abfindung/mehrjährige Vergütung, § 34 Abs. 2 Nr. 2/4), den der Laie tatsächlich eingäbe, hat **keinen rules.yaml-Input-Slot** — er lebt nur im golden-Accessor als `ausserordentliche_einkuenfte` und wird laut Regel-`hinweis` in der § 2-Integration qualifiziert. Ein askable ao-Feld braucht daher einen **Integrations-Andock-Entscheid** (analog EP-Catala-Scope). Bis dahin: p34 nur als Lücken abgebildet — **kein erfundener Slot** (Anker-Doktrin). Bitte um Entscheid, ob ein § 34-Integrations-Andockpunkt aufgemacht wird.
