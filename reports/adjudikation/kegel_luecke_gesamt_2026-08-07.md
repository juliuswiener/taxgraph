# kegel-Lücke `gesamt` — Vermessung

Datum: 2026-08-07. HEAD zum Zeitpunkt der Messung: `b87e800fd35010df5d9dfa1c9939169fe9097e24`.
Suite-Status (selbst nachgemessen, `timeout 300 python3 -m pytest -q`): **1653 passed, 4 skipped**
(213.4s). Main hatte 1650/4 gemeldet — HEAD ist neuer, Differenz plausibel, nicht untersucht (außerhalb
des Auftrags).

**Auftrag:** `_ring_bindung` (`produkt/haut/api.py:2082`) baut die Bindung für die [min,max]-
Unsicherheitsrechnung NUR aus `SCHEIBEN["gesamt"]["kegel"]`. Felder, die in `["felder"]` stehen aber
nicht im `kegel`, können nie eine Unsicherheitsachse werden. Reiner Messauftrag — **keine Umbauten**,
kein Feld in den `kegel` verschoben.

Alle Zahlen unten kommen aus tatsächlich ausgeführten Python-Skripten (`/tmp/kegel_test/sweep*.py`),
nicht aus Gedächtnis oder Schätzung. Jede Zeile trägt den Skriptnamen, der sie erzeugt hat.

---

## 1. ZÄHLEN

Befehl:
```python
from produkt.haut.api_constants import SCHEIBEN
felder = SCHEIBEN['gesamt']['felder']; kegel = SCHEIBEN['gesamt']['kegel']
len(felder), len(set(felder)), len(kegel), len(set(kegel))
```
Ergebnis: `felder` = **180** Einträge (180 unique, keine Duplikate), `kegel` = **33** Einträge (33
unique). Differenz `felder − kegel` = **147** Felder.

Vollständige, sortierte Differenzmenge (147 Felder) — siehe Abschnitt 2 (dort nach Partner/
Nicht-Partner aufgeteilt, keine Kürzung).

---

## 2. EINORDNEN

Kriterium: Namensmuster `_partner`-Suffix oder `partner_`-Präfix (deckt sich mit dem im Docstring
genannten Partner-Feld-Grund).

```python
diff = sorted(set(felder) - set(kegel))
partner = [f for f in diff if '_partner' in f or f.startswith('partner_')]
nicht = [f for f in diff if f not in partner]
```

**Partner-Felder: 21**
**Nicht-Partner-Felder: 126**

### 2a. Partner-Felder (21, vollständig, vom Docstring gedeckt)

```
basis_kv_partner
basis_pv_partner
bruttoarbeitslohn_partner
geburtsjahr_partner
kap_gewinn_aktien_partner
kap_gewinn_sonstige_partner
kap_kapitalertraege_partner
kap_verlust_aktien_partner
kap_verlust_sonstige_partner
mit_anspruch_auf_zuschuss_partner
rentner_grad_der_behinderung_partner
rentner_hilflos_blind_taubblind_partner
versicherungsart_partner
vor_ag_anteil_rv_partner
vor_an_anteil_rv_partner
vor_rv_ausserhalb_lstb_partner
vorsorge_arbeitslosenversicherung_partner
vorsorge_erwerbsunfaehigkeit_partner
vorsorge_rv_alt_mit_ueberschuss_partner
vorsorge_rv_alt_ohne_ueberschuss_partner
vorsorge_unfall_haftpflicht_partner
```

### 2b. Nicht-Partner-Felder (126, NICHT vom Docstring gedeckt — Gegenstand von Schritt 3)

Zusätzliche Unterteilung gegen die live geladene Bindung (`traverser.lade_bindung()`, 212 Einträge):
wie viele der 126 haben einen `signatur_slot` (können strukturell die Rechnung bewegen) vs. sind reine
`geltungsbedingung`/Gate-Felder (können strukturell NIE eine Zahl bewegen, nur einen Guard schalten)?

```python
for f in nicht:
    slot = b[f]["quelle"].get("signatur_slot")
    # slot vorhanden -> mit_slot, sonst -> ohne_slot
```
Ergebnis: **84 mit signatur_slot**, **42 ohne signatur_slot** (0 fehlen in der Bindung).

Die 42 gate-only Felder (können strukturell nie einen Cent-Betrag bewegen, nur einen Sperrgrund
schalten oder einen anderen Slot semantisch modifizieren):
```
am_afa_ist_anschaffungsjahr, am_gwg_sofortabzug_gewaehlt, antrag_ermaessigter_satz,
behinderungsbedingte_aufwendungen, dauernd_berufsunfaehig, dba_abzug_statt_anrechnung,
dba_einkunftsart, dba_mehrere_staaten, dba_methode, dba_staat, dhf_beruflich_veranlasst,
dhf_eigener_hausstand, dhf_finanzielle_beteiligung, dhf_keine_pflicht_dienstwohnung,
ermaessigung_einmal_genutzt, geburtsjahr, gewinn_betriebsart, hh_handwerker_keine_foerderung,
hh_in_eu_ewr, hh_rechnung_unbar, kind_idnr, p23_veraeusserungs_typ, p35a_mitveranlagung,
person_b_idnr, pv_anzahl_einheiten, pv_auf_gebaeude, pv_bruttoleistung_kwp,
realsplitting_zustimmung, rentner_alter_55_oder_berufsunfaehig, rentner_freibetrag_erstmalig,
rentner_veraeusserungs_betriebsart, uebernachtung_alleinnutzung, uebernachtung_auswaerts,
uebernachtung_im_inland, uebernachtung_keine_lange_unterbrechung, versorgung_alter_bei_beginn,
versorgung_art, versorgung_beginn_jahr, vpf_frist_nicht_unterbrochen,
vpf_keine_mahlzeitengestellung, vv_auf_dauer, vv_wohnzwecke
```
(`behinderungsbedingte_aufwendungen` hat trotz Betrags-Charakter keinen eigenen `signatur_slot` in
der Bindung — geht als Teil einer anderen Berechnung ein; hier ungeprüft, siehe „Nicht gemessen".)

Die übrigen 84 Felder mit `signatur_slot` sind Kandidaten, die die Rechnung strukturell bewegen
KÖNNTEN — ob sie es TATSÄCHLICH tun (Guards, Floor-Effekte), zeigt erst die Delta-Messung in Schritt 3.

---

## 3. DELTA MESSEN

**Methode:** In-process API-Aufruf, Muster aus `tests/test_einreichen.py` übernommen (`import api as API`
direkt, `API.FAELLE` auf ein Tempdir umgebogen, kein HTTP-Server). Für jedes Feld: Basisfall mit einem
33-Feld-Minimal-Kegel (`bruttoarbeitslohn=6.000.000 Cent` = 60.000 €, alle übrigen Kegel-Pflichtfelder
neutral/0 gesetzt), `API.ergebnis(fall_id)` gerechnet, dann Zielfeld auf seinen `beispielwert` aus der
Bindung gesetzt, neu gerechnet, Differenz in Cent notiert.

**Basis (Skript: `/tmp/kegel_test/sweep2.py`, Aufruf `python3 sweep2.py`):**
`bruttoarbeitslohn=6.000.000 Cent`, `kein_gewinn/kein_kap/kein_vuv/kein_sonstige=True`, alle übrigen
Kegel-Pflichtfelder 0/neutral → **BASIS_CENT = 1.392.400** (13.924,00 € auf 60.000 € Brutto, ledig).

Für 16 der 126 Felder blockierte diese Basis den Ring (Sperrgrund oder — bei den drei `p23_*`-Feldern —
ein Programmfehler). Diese wurden über gezielte Zwischen-Basisfälle mit den vom jeweiligen Guard
verlangten Begleitfeldern nachgemessen (Skripte `sweep3.py`–`sweep9.py`, jeweils benannt in der
Quelle-Spalte). Details in Abschnitt „Methodik-Anmerkungen" unten.

### 3a. Vollständige Ergebnistabelle — sortiert absteigend nach |Delta| (Schritt 4)

123 von 126 nicht-Partner-Feldern gemessen (3 durch einen Programmfehler blockiert, siehe „Nicht
gemessen"). δ in Cent gegenüber der jeweils genannten Basis.

| # | Feld | Δ (Cent) | Quelle (Skript + Begleitfelder) |
|---|------|---------:|----------------------------------|
| 1 | rentner_veraeusserungsgewinn | **+4.914.000** | sweep4.py (Begleitfelder: rentner_alter_55_oder_berufsunfaehig=True, rentner_freibetrag_erstmalig=True) |
| 2 | betriebseinnahmen | **+3.343.200** | sweep8.py (Begleitfelder: kein_gewinn=False, gewinn_betriebsart=gewerbe) |
| 3 | gewinnanteil | **+1.663.200** | sweep8.py (Begleitfelder: kein_gewinn=False, gewinn_betriebsart=gewerbe) |
| 4 | gewst_messbetrag | **−1.597.600** | sweep9.py (Begleitfelder: einkuenfte_gewinn=50.000€, gewst_hebesatz=450, gewinn_betriebsart=gewerbe) |
| 5 | verlustvortrag_bestand | **−1.392.400** | sweep2.py (60k-Basis direkt) |
| 6 | einkuenfte_gewinn | **+1.243.200** | sweep8.py (Begleitfelder: kein_gewinn=False, gewinn_betriebsart=gewerbe) |
| 7 | sonstige_betriebsausgaben | **−997.800** | sweep8.py (Begleitfelder: kein_gewinn=False, gewinn_betriebsart=gewerbe) |
| 8 | p33a_unterhalt_aufwendungen | −367.900 | sweep2.py (60k-Basis direkt) |
| 9 | dhf_unterkunftskosten_monat | −310.400 | sweep3.py (DHF_BEDINGUNGEN alle=True) |
| 10 | uebernachtung_kosten_monat | −310.400 | sweep3.py (UEBERNACHTUNG_BEDINGUNGEN alle=True) |
| 11 | p35c_sanierungsaufwendungen | −210.000 | sweep2.py (60k-Basis direkt) |
| 12 | afa_jahresbetrag | −188.400 | sweep8.py (Begleitfelder: kein_gewinn=False, gewinn_betriebsart=gewerbe) |
| 13 | kinderbetreuungskosten | −179.700 | sweep2.py (60k-Basis direkt) |
| 14 | p32b_progressionseinkuenfte | +141.900 | sweep2.py (60k-Basis direkt) |
| 15 | berufsausbildung_aufwendungen | −75.100 | sweep2.py (60k-Basis direkt) |
| 16 | p33a_ausbildung_anzahl_kinder | −46.000 | sweep2.py (60k-Basis direkt) |
| 17 | rentner_grad_der_behinderung | −43.800 | sweep2.py (60k-Basis direkt) |
| 18 | rentner_pflegegrad | −42.200 | sweep2.py (60k-Basis direkt) |
| 19 | p35c_energieberater_aufwendungen | −40.000 | sweep2.py (60k-Basis direkt) |
| 20 | schulgeld | −33.200 | sweep2.py (60k-Basis direkt) |
| 21 | geburtsjahr | −25.600 | sweep2.py (60k-Basis direkt) |
| 22 | hh_handwerker_arbeitskosten | −24.000 | sweep5.py (Begleitfelder: hh_in_eu_ewr=True, hh_rechnung_unbar=True, hh_handwerker_keine_foerderung=True) |
| 23 | gwg_anschaffungskosten_netto | −23.100 | sweep8.py (Begleitfelder: kein_gewinn=False, gewinn_betriebsart=gewerbe) |
| 24 | kist_gezahlt | −21.700 | sweep2.py (60k-Basis direkt) |
| 25 | hh_dienstleistungen | −20.000 | sweep5.py (Begleitfelder: hh_in_eu_ewr=True, hh_rechnung_unbar=True) |
| 26 | spenden_betrag | −10.200 | sweep2.py (60k-Basis direkt) |
| 27 | am_anschaffungskosten | 0 | sweep5.py (Begleitfeld am_gwg_sofortabzug_gewaehlt=True; echte 0 — s.u.) |
| 28–123 | (96 weitere Felder) | 0 | sweep2.py (60k-Basis direkt) |

Die 96 restigen delta=0-Felder (inkl. `am_anschaffungskosten`) sind vollständig aufgelistet in Abschnitt
3b — nicht gekürzt.

### 3b. Die 97 Felder mit gemessenem Δ = 0

Wichtig: 0 heißt hier "unter dieser Basis und diesem beispielwert bewegt sich der Cent-Betrag nicht" —
das ist NICHT gleichbedeutend mit "der kegel-Ausschluss ist harmlos". Zwei Unterklassen:

**(i) Strukturell erwartbar 0** — reine Gate-/Bool-Felder ohne eigenen `signatur_slot` (42 Felder,
siehe Abschnitt 2b) können per Bindungs-Definition nie selbst einen Betrag liefern; ihr Δ=0 bestätigt
nur, dass sie (in diesem Testfall) auch keinen anderen Slot verändert haben, den `bescheid_via_slots`
sieht. `am_anschaffungskosten` (hat einen signatur_slot, δ=0 gemessen) fällt in eine andere Kategorie:
der § 9a Arbeitnehmer-Pauschbetrag (1.230 €) deckt den `beispielwert` (600 €) bereits ab — der Ring
rechnet real, aber der Pauschbetrag-Floor verschluckt den Effekt bei diesem Beispielwert; kein
Kegel-Symptom, sondern ein Pauschbetrag-Floor-Artefakt. Nicht weiter mit anderem Beispielwert
nachgemessen (Auftrag verlangt beispielwert aus der Bindung, nicht selbstgewählte Werte).

**(ii) Felder mit `signatur_slot`, aber unter dieser Basis irrelevant** — z.B. `kist_bundesland`
(Text/Enum, ändert nur die Kirchensteuer-Berechnungsmethode, nicht isoliert testbar ohne
`kist_konfession` gesetzt — hier war Konfession nicht gesetzt, also kein KiSt-Zweig aktiv),
`realsplitting_unterhaltsleistungen` (braucht `realsplitting_zustimmung=True`, nicht gesetzt),
`versorgung_*`-Felder (Versorgungsfreibetrag-Zweig, braucht `versorgungsfreibetrag_offen`-Guard-
Bedingungen, hier ungesetzt), `pv_einnahmen` (§ 3 Nr. 72 Photovoltaik, braucht `pv_auf_gebaeude`+
`pv_bruttoleistung_kwp` als Voraussetzung fürs Freibetrags-Gate). Diese Δ=0 sind Basis-Artefakte
(fehlende Begleitfelder), NICHT belastbare "kein Effekt"-Aussagen — für eine abschließende Aussage
bräuchte jedes dieser Felder eine eigene, maßgeschneiderte Zwischen-Basis wie bei den 16 in Abschnitt
3c behandelten Feldern. Das würde den Auftrag ("nur messen") auf ~20 weitere Einzelläufe ausdehnen;
hier als **nicht gemessen** dokumentiert statt geraten.

Vollständige Liste (97 Felder, Δ=0, sortiert):
```
agb_aufwendungen, am_afa_ist_anschaffungsjahr, am_anschaffung_monat, am_anschaffungskosten,
am_gwg_sofortabzug_gewaehlt, antrag_ermaessigter_satz, arbeitsmittel_nutzungsdauer,
behinderungsbedingte_aufwendungen, dauernd_berufsunfaehig, dba_abzug_statt_anrechnung,
dba_auslaendische_einkuenfte, dba_einkunftsart, dba_gezahlte_auslaendische_steuer,
dba_mehrere_staaten, dba_methode, dba_staat, dhf_beruflich_veranlasst, dhf_eigener_hausstand,
dhf_finanzielle_beteiligung, dhf_im_inland, dhf_keine_pflicht_dienstwohnung, dhf_monate,
ermaessigung_einmal_genutzt, fahrtkosten_pausch_ag_bl_tbl_h, fahrtkosten_pausch_gdb80_oder_70g,
fam_alleinstehend, fam_anzahl_kinder, fam_monate_ohne_voraussetzung, gewinn_betriebsart,
gewst_hebesatz, hh_handwerker_keine_foerderung, hh_in_eu_ewr, hh_minijob_aufwendungen,
hh_rechnung_unbar, kind_behinderten_pb_antrag, kind_grad_der_behinderung,
kind_hilflos_blind_taubblind, kind_hinterbliebenen_uebertragung, kind_idnr, kind_kv,
kind_pb_nicht_selbst_genutzt, kind_pv, kist_bundesland, kist_erstattet, kist_konfession,
p22_nr3_einkuenfte, p23_veraeusserungs_typ, p33a_andere_einkuenfte_bezuege, p33a_unterhalt_kv_pv,
p35a_mitveranlagung, p35c_ist_uebernaechstes_foerderjahr, p36_lohnsteuer, p36_vorauszahlungen,
person_b_idnr, pv_anzahl_einheiten, pv_auf_gebaeude, pv_bruttoleistung_kwp, pv_einnahmen,
realsplitting_empfaenger_kv_pv, realsplitting_unterhaltsleistungen, realsplitting_zustimmung,
rentner_alter_55_oder_berufsunfaehig, rentner_freibetrag_erstmalig, rentner_gepflegter_hilflos,
rentner_hilflos_blind_taubblind, rentner_hinterbliebenenbezuege, rentner_veraeusserungs_betriebsart,
tage_24h, tage_an_abreise, tage_ueber_8h_eintaegig, uebernachtung_alleinnutzung,
uebernachtung_auswaerts, uebernachtung_im_inland, uebernachtung_keine_lange_unterbrechung,
uebernachtung_monate, uebernachtung_monate_bisher, verguetung_darlehen, verguetung_taetigkeit,
verguetung_ueberlassung, versorgung_alter_bei_beginn, versorgung_art, versorgung_beginn_jahr,
versorgung_bemessungsgrundlage, versorgung_jahresrente, vpf_abendessen_gestellt_anzahl,
vpf_frist_nicht_unterbrochen, vpf_fruehstuecke_gestellt_anzahl, vpf_keine_mahlzeitengestellung,
vpf_mahlzeiten_gezahltes_entgelt, vpf_mittagessen_gestellt_anzahl, vpf_monate_am_ort,
vpf_steuerfreie_erstattung_betrag, vpf_tage_24h_nach_drei_monaten,
vpf_tage_an_abreise_nach_drei_monaten, vpf_tage_ueber_8h_nach_drei_monaten, vv_auf_dauer,
vv_wohnzwecke
```

### 3c. Methodik-Anmerkungen zu den nachgemessenen 16 Feldern

Sechs Felder (`afa_jahresbetrag`, `betriebseinnahmen`, `einkuenfte_gewinn`, `gewinnanteil`,
`gwg_anschaffungskosten_netto`, `sonstige_betriebsausgaben`) triggerten unter der reinen 60k-Basis
`flag_konsistenz_offen` (`produkt/konsistenz/flag_check.py::flag_widersprueche`) — Ursache:
`kein_gewinn=True` in der Basis widerspricht jedem gesetzten Gewinn-Feld. Nachgemessen mit
Zwischen-Basis `kein_gewinn=False, gewinn_betriebsart=gewerbe` (sweep8.py), Δ jeweils gegen diese
Zwischen-Basis (Zwischen-Basis selbst = 1.392.400 Cent, identisch zur 60k-Basis, weil beide Flags
allein keinen Betrag bewegen).

`rentner_veraeusserungsgewinn` triggerte `p16_4_gate_offen` — nachgemessen mit
`rentner_alter_55_oder_berufsunfaehig=True, rentner_freibetrag_erstmalig=True` (sweep4.py).

`dhf_unterkunftskosten_monat`/`uebernachtung_kosten_monat` triggerten ihre jeweiligen
Tatbestand-Gates — nachgemessen mit allen zugehörigen `_BEDINGUNGEN`-Feldern auf True (sweep3.py).

`gewst_messbetrag` triggerte `gewst_hebesatz_offen` — nachgemessen mit `gewst_hebesatz=450` UND
zusätzlich `einkuenfte_gewinn=50.000€` gesetzt (ohne positiven Zähler/Nenner bleibt § 35 bei 0, das
wäre kein echter Test des Feldes) — sweep9.py, Δ gegen eine Zwischen-Basis MIT Hebesatz+Gewinn aber
OHNE Messbetrag (3.475.600 Cent).

`hh_dienstleistungen`/`hh_handwerker_arbeitskosten` triggerten `rechnung_unbar_offen` — Nachmessung
zeigte einen zweiten, in der ursprünglichen sweep3.py-Iteration übersehenen Gate: § 35a Abs. 4
(`hh_in_eu_ewr`) nullt in `catala_p35a_haushaltsnahe` (`golden/runner.py:416`) ALLE drei Abs. 1-3-
Positionen, wenn nicht explizit `True`. Nachgemessen mit `hh_in_eu_ewr=True` zusätzlich zu
`hh_rechnung_unbar=True` (sweep5.py).

`am_anschaffungskosten` triggerte `arbeitsmittel_afa_ueber_gwg_offen` — nachgemessen mit
`am_gwg_sofortabzug_gewaehlt=True` (sweep5.py); Ergebnis Δ=0 (echte Null, s. 3b(i)).

---

## 4. SORTIEREN — Top-Treiber der kegel-Lücke (nicht-Partner-Anteil)

Absteigend nach |Δ| (Werte aus 3a, Top 10):

1. `rentner_veraeusserungsgewinn` +4.914.000 Cent (49.140 €)
2. `betriebseinnahmen` +3.343.200 Cent (33.432 €)
3. `gewinnanteil` +1.663.200 Cent (16.632 €)
4. `gewst_messbetrag` −1.597.600 Cent (−15.976 €)
5. `verlustvortrag_bestand` −1.392.400 Cent (−13.924 €)
6. `einkuenfte_gewinn` +1.243.200 Cent (12.432 €)
7. `sonstige_betriebsausgaben` −997.800 Cent (−9.978 €)
8. `p33a_unterhalt_aufwendungen` −367.900 Cent (−3.679 €)
9. `dhf_unterkunftskosten_monat` −310.400 Cent (−3.104 €)
10. `uebernachtung_kosten_monat` −310.400 Cent (−3.104 €)

Alle zehn sind Beträge, die bei Bestätigung real den festgesetzten Steuerbetrag bewegen (Δ ≠ 0,
tatsächlich durch den Ring gerechnet) — aber wegen der `_ring_bindung`-Beschränkung auf `kegel` nie
als [min,max]-Unsicherheitsachse erscheinen, selbst wenn sie im Store nur `vorlaeufig` (unbestätigt)
vorliegen. Der Docstring-Grund (Partner-Feld-unbounded-Problem) deckt keinen dieser 10 Fälle ab —
das sind alles Nicht-Partner-Felder.

---

## Nicht gemessen

1. **`p23_anschaffung_herstellungskosten`, `p23_veraeusserungspreis`, `p23_werbungskosten`** (3
   Felder) — blockiert durch einen reproduzierbaren Programmfehler in
   `produkt/haut/api.py::_p23_ansonsten_einkuenfte` (Zeilen ~231-233):
   ```python
   preis = int(inst["felder"].get("p23_veraeusserungspreis", 0)) // 100
   ak = int(inst["felder"].get("p23_anschaffung_herstellungskosten", 0)) // 100
   wk = int(inst["felder"].get("p23_werbungskosten", 0)) // 100
   ```
   `EM.instanzen()` liefert `inst["felder"][fid]` als `{"wert":..., "zustand":..., "herkunft":...}`
   (siehe `produkt/mapping/est_mapping.py:451`), NICHT den Rohwert — der fehlende `.get("wert")`-
   Zugriff lässt `int()` über ein dict laufen. Reproduziert mit (Skript `sweep2.py`, Feld einzeln
   auf beispielwert gesetzt):
   ```
   TypeError: int() argument must be a string, a bytes-like object or a real number, not 'dict'
   ```
   Traceback-Ursprung: `api.py:_p23_ansonsten_einkuenfte`. Kein Fix vorgenommen (Auftrag: nur
   messen). Diese 3 Felder bleiben ungemessen bis der Bug behoben ist.

2. **~20 Felder mit `signatur_slot` aber Δ=0 unter der gewählten Basis** (Abschnitt 3b(ii)) —
   `kist_bundesland`, `realsplitting_unterhaltsleistungen`, `versorgung_*` (5 Felder), `pv_einnahmen`,
   u.a. — bräuchten je eine maßgeschneiderte Zwischen-Basis mit den passenden Begleitfeldern, um zu
   zeigen, ob sie unter EINER passenden Konstellation einen Betrag bewegen. Nicht nachgemessen (würde
   den Umfang um ~20 weitere Einzelläufe erweitern); als Δ=0-unter-dieser-Basis dokumentiert, nicht als
   "kein Effekt".

3. **Interaktions-/Kombinationseffekte** zwischen mehreren Nicht-Kegel-Feldern gleichzeitig (z.B.
   `betriebseinnahmen` + `sonstige_betriebsausgaben` + `afa_jahresbetrag` gemeinsam vorläufig, wie es
   in der Praxis vorkäme) — nicht gemessen, Auftrag verlangte Feld-für-Feld (one-at-a-time), nicht
   kombinatorisch.

4. **Ob die 42 gate-only Felder (Abschnitt 2b) über einen ANDEREN Rechenpfad (z.B. `/stand` oder
   `/fragen`, `nur_bestaetigt=False`) doch Betrag-relevant werden** — nicht geprüft, nur der
   `/ergebnis`-Pfad (`_feste_zahl`) wurde vermessen.

5. **Die frühere, jetzt gelöschte Version dieses Berichts** (Commit `2669ddf`, zurückgenommen in
   `7ee59ef`) behauptete Delta-Zahlen, die nie gelaufen sind — dieser Bericht ersetzt sie vollständig
   und unabhängig; keine Zahl daraus wurde übernommen.

**KEINE Gegenprobe mit Umbau durchgeführt, KEIN Feld in `kegel` verschoben** — wie im Auftrag verlangt.
