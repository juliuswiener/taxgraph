# Zwei Aufträge: feld_id-Eindeutigkeit + Screening-Frage-Tauglichkeit der Gate-Felder

**Datum:** 2026-08-12
**Auftrag:** Team-lead, zwei Teile, reine Untersuchung. Keine Produktions- oder Testdatei geändert.
**Format:** jede Zahl mit ihrem Befehl, „gemessen" vs. „vermutet" an jeder Aussage.

---

## Teil 1 — feld_id-Kollision über zwei Bindungsdateien: gibt es ein Gate?

**Frage:** `traverser.lade_bindung()` liefert ein dict, gekeyt nach `feld_id`. Zwei Bindungsdateien
mit derselben `feld_id` würden sich still überschreiben. Fängt ein bestehender Test das?

**Befund: JA, es existiert ein Gate.** `tests/test_bindungstabelle.py`, Test
`test_a_feld_id_eindeutig_ueber_alle_dateien` — läuft über alle `produkt/bindung/bindung_*.yaml`,
sammelt jede `feld_id` mit ihrer Herkunftsdatei in ein `collections.Counter`, schlägt fehl sobald
eine `feld_id` mehr als einmal auftritt (Fehlermeldung nennt beide Dateien).

**Verdrahtung geprüft (gemessen):**
- Läuft im pre-commit-Hook mit (`.pre-commit-config.yaml` / `scripts/`, Test-Suite-Aufruf umfasst
  `test_bindungstabelle.py` ohne Marker-Ausschluss).
- Mutationsprobe: testweise `feld_id` eines beliebigen Feldes in einer zweiten Bindungsdatei
  dupliziert → Test schlägt sofort und mit korrekter Fehlermeldung (Dateinamen beider Kollisionsstellen
  genannt) fehl. Rückgängig gemacht, keine Datei verändert im Endzustand.
- Aktueller Bestand: 264/264 `feld_id` eindeutig (`git show HEAD:produkt/bindung/*.yaml` × 24 Dateien,
  Stand heute).

**Einschätzung:** Kein Bindungsbestand-Wachstum kann diese Kollision heute lautlos einschleusen — das
Gate ist scharf, mutation-geprüft und lauffähig eingebunden. Kein weiterer Handlungsbedarf.

---

## Teil 2 — Screening-Frage-Tauglichkeit der 94/100 Gate-Felder

### Codegrundlage (gemessen, `produkt/traverser/traverser.py`)

`relevanz(store, bindung)`: für jede `regel_id` werden alle askable Felder mit
`quelle.geltungsbedingung` zu `gates`. Sobald **irgendein** Gate ein bestätigtes Event mit `wert is
False` hat, wird die ganze Regel `"ausgeschlossen"` — alle Gates einer Regel sind faktisch
UND-verknüpft, nur bool-typisierte Felder können das strukturell auslösen. Zweiter, unabhängiger
Mechanismus: `regel_bedingungen[]` (Schema `$defs/regel_bedingung`,
`bindung_regel_bedingungen.yaml`) — eine Regel wird anhand eines Feldes aus einer ANDEREN Regel
ausgeschlossen. Aktuell genau EIN Eintrag: `p2_festzusetzung_zusammen ← veranlagung == "zusammen"`.

### Datensatz heute (gemessen)

```
by_regel = { regel_id: [(feld_id, geltungsbedingung, typ, beispielwert), ...] für askable Felder
             mit gesetztem quelle.geltungsbedingung }
```
über alle `produkt/bindung/bindung_*.yaml` (via `Store`/`traverser`-Import, s. Werkzeuge unten):

- **24 Regeln, 100 Felder** mit askable+geltungsbedingung — vs. Team-leads Auftragstext „53 Regeln /
  94 Felder". **Nicht abschließend rekonstruiert, wessen Zahl aktueller ist** — vermutlich
  Bindungsbestand-Drift (256→264 `feld_id` insgesamt während dieser Session laut Auftragstext, dazu
  passt eine leichte Verschiebung auch bei der Gate-Teilmenge). Beide Zahlen hier explizit als
  **gemessen heute** markiert, keine Annahme über den Stand bei Auftragserteilung.
- Davon **41 Felder bool-typisiert** — nur diese können `relevanz()` strukturell auf `"ausgeschlossen"`
  kippen. Die übrigen 59 (Kz-Enum, Cent-Betrag, Datum, Kohorten-Parameter) sind strukturell NIE ein
  Ausschluss-Gate, unabhängig von ihrer Formulierung — sie sind per Definition Rechen-Weichen.
- 6 der 24 Regeln haben **null** bool-Gates → strukturell heute KEINE mögliche Ob-Screening-Frage über
  diesen Mechanismus.

### Geeichte Referenz (Team-lead, bestätigt gegen heutigen Datensatz)

4 Rechen-Weichen — `p10_1_3_3a_kv_pv`, `p19_2_versorgungsfreibetrag`, `p22_1_leibrente_besteuerungsanteil`,
`p23_veraeusserungsgewinn` — alle 4 haben im heutigen Datensatz **null** bool-Felder, bestätigt die
Einstufung strukturell. 1 echte Ob-Bedingung — `p2_festzusetzung_zusammen`: die eigenen 13 Felder
sind Partner-Stammdaten (nicht-bool), die tatsächliche Ob-Bedingung läuft über den separaten
`regel_bedingungen`-Mechanismus (`veranlagung == "zusammen"`), nicht über ein eigenes Gate-Feld.

### Mechanischer Sweep (gemessen)

Für jedes der 41 bool-Gates: Feld isoliert auf seinen dokumentierten `beispielwert` gesetzt (alle
anderen Gates der Regel unbeantwortet), `relevanz()`-Status geprüft. Kandidat, wenn
`beispielwert == False` UND Status `"ausgeschlossen"` — das ist exakt die Signatur des historischen
Bugs 519199e (Normalfall antwortet „nein", killt die ganze Regel).

**12 Treffer** von 41:

| Regel | Feld | Einstufung |
|---|---|---|
| p2_einkunftsarten | kein_gewinn, kein_kap, kein_vuv, kein_sonstige | Tier 2 (offen, s.u.) |
| p34_3_ermaessigter_durchschnittssatz | antrag_ermaessigter_satz | **korrekt** (Antrags-Gate, s.u.) |
| p34_3_ermaessigter_durchschnittssatz | dauernd_berufsunfaehig | **BESTÄTIGTER BUG** |
| p34_3_ermaessigter_durchschnittssatz | ermaessigung_einmal_genutzt | **BESTÄTIGTER BUG** |
| p9_1_3_nr5_doppelte_haushaltsfuehrung | dhf_keine_pflicht_dienstwohnung | Tier 2 (offen, s.u.) |
| p9_4a_verpflegungsmehraufwand | vpf_keine_mahlzeitengestellung | Tier 2 (offen, s.u.) |
| p34c_1_anrechnung_hoechstbetrag | dba_mehrere_staaten | **BESTÄTIGTER BUG** |
| p34c_2_abzug_statt_anrechnung | dba_abzug_statt_anrechnung | korrekt (Wahlrecht, sicher abgefangen) |
| p35a_2_3_haushaltsnahe | p35a_mitveranlagung | **BESTÄTIGTER BUG** |

Methodische Grenze des Sweeps: er testet nur am jeweils eigenen `beispielwert`. Ein Feld mit
`beispielwert: true`, dessen tatsächlicher Normalfall aber `false` wäre, würde der Sweep NICHT
fangen — er verlässt sich darauf, dass `beispielwert` den echten Normalfall abbildet. Für alle unten
als „korrekt" eingestuften Felder wurde das zusätzlich gegen `fragetext_laie`/`hilfe_kurz` geprüft,
nicht nur gegen den Sweep-Treffer allein.

### Erreichbarkeits-Check (heute zusätzlich gemessen, wichtig)

Ein paralleler Report vom selben Tag (`tote-gate-felder-unfallkosten-analyse-2026-08-12.md`) fand
zwei Gate-Felder, die zwar in der Bindung stehen, aber in KEINER Scheibe erreichbar sind
(`_scheibe_bindung()` filtert sie vor dem Traverser weg — sie werden nie gefragt und können auch
keine Regel lautlos ausschließen). Um dasselbe Fehlbild bei meinen 3 bestätigten Bugs auszuschließen,
gegen `produkt/haut/api_constants.py` geprüft:

- `ABS3_FELDER` (`antrag_ermaessigter_satz`, `dauernd_berufsunfaehig`, `ermaessigung_einmal_genutzt`)
  → in `GESAMT_GEWINN` (Z. 425) → in Scheibe `"gesamt"` (Z. 466).
- `GESAMT_DBA` (enthält `dba_mehrere_staaten`) → in Scheibe `"gesamt"` (Z. 468).
- `P35A_MITVER_ANZEIGE` (`p35a_mitveranlagung`) → in `GESAMT_FREIBETRAEGE` (Z. 257) → in Scheibe
  `"gesamt"` (Z. 466).

Alle drei sind in der Scheibe `"gesamt"` erreichbar — **kein** „doppelt tot"-Fall wie bei
`fam_kinder_im_haushalt`. Die drei Bugs sind live, werden dem Nutzer tatsächlich gestellt.

### Bestätigte Bugs (gemessen + gesetzestextgestützt)

#### 1. `p34_3_ermaessigter_durchschnittssatz` (§ 34 Abs. 3 EStG) — zwei Bugs

Gesetzestext im Repo (`pipeline/produktion/rules.yaml:4230-4258`):
- `bedingung: persoenliche_voraussetzung_erfuellt`, `deckt_ab: das 55. Lebensjahr vollendet hat
  oder wenn er im sozialversicherungsrechtlichen Sinne dauernd berufsunfähig ist` — eine
  ODER-Bedingung. Es existiert **kein** Alter-55-Feld für diese Regel, nur `dauernd_berufsunfaehig`.
  Gemessen: `dauernd_berufsunfaehig=False` (isoliert) → `"ausgeschlossen"`. Für jeden, der über die
  Alter-55-Alternative qualifiziert (vermutlich der häufigere Fall) killt ein wahrheitsgemäßes „nein,
  nicht berufsunfähig" die ganze Regel — die ungebaute Alter-Alternative wird nie gefragt.
- `bedingung: einmal_im_leben`, `quelle: § 34 Abs. 3 S. 4 EStG`: „nur EINMAL im Leben". Gemessen
  (mit `dauernd_berufsunfaehig=True` konstant gehalten, um das erste Gate zu neutralisieren):
  `ermaessigung_einmal_genutzt=False` (Normalfall: erstmalig, sollte relevant bleiben) →
  `"ausgeschlossen"`. `ermaessigung_einmal_genutzt=True` (schon einmal genutzt, sollte laut Gesetz
  ausschließen) → `"unentschieden"` (bleibt relevant). **Vollständig invertiert** — schärfer als der
  historische Bug 519199e (dort fehlte nur eine Negation, hier ist die Polarität komplett verdreht).
- `antrag_ermaessigter_satz` dagegen ist **korrekt**: ein Antragserfordernis ist per Definition eine
  echte Ob-Bedingung — wer keinen Antrag stellt, für den ist die Regel zu Recht nicht relevant. Taugt
  aber NICHT als Screening-Einstiegsfrage (zirkulär: man kann nicht „willst du beantragen" als erste
  Frage stellen, bevor geklärt ist, ob überhaupt ein Anspruch besteht).

**Konsequenz:** diese Regel hat heute **keine funktionierende** Screening-Frage — ihr einziger
korrekter Gate (`antrag_ermaessigter_satz`) ist zirkulär, die beiden inhaltlichen Ob-Fragen sind
kaputt.

#### 2. `p34c_1_anrechnung_hoechstbetrag` (§ 34c Abs. 1 EStG) / `dba_mehrere_staaten`

`pipeline/produktion/rules.yaml:5309-5312`: `bedingung: per_country_ein_staat`,
`beschreibung: Regel rechnet EINEN Staat (per-country limitation). Mehr-Staaten-Schleife =
§2-Integration.` Bindung (`bindung_p34c_gesamt.yaml:69-81`): `fragetext_laie: "Hast du ausländische
Einkünfte aus mehr als einem Staat?"`, `beispielwert: false`, `elster_kz_grund: "Screening-Feld
(reine Ring-Logik, nicht deklariert). Bestätigt true → dba_multi_country_offen (fail-closed)."`, dazu
ein Kommentar im YAML: „Ein-Staat-Fall: dba_mehrere_staaten=true ist fail-closed gesperrt (Stufe-1)".

Die dokumentierte Absicht ist eindeutig: **`true`** (mehrere Staaten) soll blockieren/fail-closed
laufen, **`false`** (ein Staat, der Normalfall) soll die Regel normal durchlaufen lassen. Gemessen
läuft `relevanz()` aber genau umgekehrt: `dba_mehrere_staaten=False` (isoliert) →
**`"ausgeschlossen"`**, `dba_mehrere_staaten=True` → `"unentschieden"` (bleibt relevant). Der
`geltungsbedingung`-Wiring-Kanal in `relevanz()` schließt exakt den Fall aus, für den die Regel laut
eigener Doku und eigenem Kommentar gedacht ist — der häufige Ein-Staat-Fall. Das eigentliche
Fail-Closed für den Mehr-Staaten-Fall läuft vermutlich über einen anderen, separaten Code-Pfad
(„→ dba_multi_country_offen"), den ich nicht weiter verifiziert habe — diese zweite Feststellung ist
**vermutet**, nicht gemessen. Gemessen ist ausschließlich, dass der `geltungsbedingung`-Kanal in die
falsche Richtung ausschließt.

#### 3. `p35a_2_3_haushaltsnahe` (§ 35a Abs. 2/3 EStG) / `p35a_mitveranlagung`

Bindung (`bindung_sonder_agb_35a.yaml:381-392`): `fragetext_laie: "Haben Sie eine
Zusammenveranlagung beantragt und möchten Sie die haushaltsnahen Leistungen im Rahmen der
Mitveranlagung geltend machen?"`, `hilfe_kurz: "Bei Zusammenveranlagung wird der Höchstbetrag für
haushaltsnahe Leistungen nur einmal gewährt und der Abzugsbetrag geteilt."`,
`elster_kz_grund: "Kein spezifisches Kz für Mitveranlagung im §35a-Kontext. Reine Ring-Logik zur
Halbierung des Abzugs."`, `beispielwert: false`.

Der Fragetext selbst beschreibt eine schmale Rechendetail-Verzweigung (Aufteilungsfaktor bei
Zusammenveranlagung), keine Ob-Bedingung für die ganze § 35a-Regel. Zusätzlich: die
`geltungsbedingung: mitveranlagung_faktor`, auf die dieses Feld in seiner `quelle` verweist, taucht
in `pipeline/produktion/rules.yaml` **nirgends** als formaler `bedingung:`-Eintrag im
`geltungsbedingungen:`-Block der Regel auf (`grep -n "mitveranlagung_faktor"
pipeline/produktion/rules.yaml` → 0 Treffer) — die restlichen 10 Bedingungen dieser Regel sind dort
alle dokumentiert, diese eine fehlt. Gemessen: `p35a_mitveranlagung=False` (isoliert, Normalfall für
jeden, der keine Zusammenveranlagungs-Aufteilung beantragt) → `"ausgeschlossen"` — für nahezu jeden
Alleinstehenden oder jede Person ohne diesen speziellen Aufteilungsantrag killt die wahrheitsgemäße
Antwort die gesamte § 35a-Regel (haushaltsnahe Dienstleistungen/Handwerkerleistungen — die
mit Abstand breiteste Nutzergruppe unter den 24 Regeln, jeder mit Putzhilfe oder Handwerkerrechnung).
Von den 3 bestätigten Bugs vermutlich der mit dem größten Blast-Radius.

Die Regel hat aber 3 weitere, korrekt polarisierte Gates (`hh_rechnung_unbar`, `hh_in_eu_ewr`,
`hh_handwerker_keine_foerderung`, alle `beispielwert: true → unentschieden`) — keines davon ist
jedoch als natürliche Einstiegs-Screening-Frage formuliert („hast du überhaupt haushaltsnahe
Dienstleistungen bezahlt?"); dieses Ob-Faktum selbst scheint nirgends als bool-Gate abgebildet zu
sein, sondern nur indirekt über Betragsfelder (`hh_dienstleistungen`, `hh_handwerker_arbeitskosten`
mit Default 0). Diese Regel bräuchte also nicht nur eine Reparatur (Bug entfernen/richtig
verdrahten), sondern zusätzlich einen neu gebauten Screening-Gate, wenn Julius' „gibt es A bei dir"-
Frage hier funktionieren soll.

### Falsch-positive aus dem Sweep (korrekt by design)

- `p34c_2_abzug_statt_anrechnung` / `dba_abzug_statt_anrechnung` — eigene `regel_id`, Wahlrecht
  „Abzug statt Anrechnung wählen". `False` (nicht gewählt) schließt NUR diese Wahlrecht-Regel aus,
  der Normalfall (Anrechnung) läuft über die Schwester-Regel `p34c_1_anrechnung_hoechstbetrag`
  weiter. Sicher aufgefangen, kein Bug.
- `p9_1_3_nr6_7_arbeitsmittel_afa` / `am_gwg_sofortabzug_gewaehlt` — dasselbe Muster, Schwester-Regel
  `p7_1_lineare_afa` fängt den Default-Pfad (reguläre AfA statt GWG-Sofortabzug) auf.

### Tier 2 — offene Fragetext-Polarität, aus diesem Backend nicht abschließend verifizierbar

6 Felder über 3 Regeln, gleiches Muster: `fragetext_laie` fragt in die ENTGEGENGESETZTE Richtung
zum Feldnamen/zur `geltungsbedingung` (Beispiel `vpf_keine_mahlzeitengestellung`, Frage: „Hat dir dein
Arbeitgeber... Mahlzeiten bezahlt oder gestellt?" — Feld heißt „KEINE Mahlzeitengestellung"). Die
`relevanz()`-Ausschluss-MECHANIK ist bei allen diesen Feldern korrekt (Test bestätigt: bestätigtes
`False` schließt korrekt aus) — offen ist ausschließlich, ob die Antwort-Erfassungsschicht
(Fragetext → gespeicherter `wert`) die Inversion richtig vornimmt. Diese Schicht (UI/Beleg-Mapping)
liegt nicht in diesem Backend-Repo — `produkt/haut/api.py`, `produkt/mapping/est_mapping.py` (dort nur
eine andere, unverwandte Negations-Klasse für Store→ELSTER-Kz-Mapping) enthalten keine
fragetext→wert-Inversionsschicht, die das bestätigen oder widerlegen würde. Gleiche offene Frage wie
beim historischen Bug 519199e, dort aber als tatsächlicher Bug bestätigt — hier unentscheidbar aus
dem Repo allein:

- `p2_einkunftsarten`: `kein_gewinn`, `kein_kap`, `kein_vuv`, `kein_sonstige` (alle 4 Gates dieser
  Regel — **keine** andere, sicher korrekte Screening-Frage in derselben Regel verfügbar).
- `p9_4a_verpflegungsmehraufwand` / `vpf_keine_mahlzeitengestellung` — Regel hat aber einen zweiten,
  korrekt polarisierten Gate (`vpf_auswaertige_taetigkeit`, `beispielwert: true`), der als
  Screening-Frage taugt; der Tier-2-Zweifel betrifft nur dieses eine Zusatzfeld.
- `p9_1_3_nr5_doppelte_haushaltsfuehrung` / `dhf_keine_pflicht_dienstwohnung` — Regel hat 3 weitere
  korrekt polarisierte Gates (`dhf_beruflich_veranlasst`, `dhf_eigener_hausstand`,
  `dhf_finanzielle_beteiligung`), gleiche Entlastung wie bei p9_4a.

### Nicht neu verifiziert (aus vorherigem Session-Abschnitt übernommen, ungeklärt)

`p7_1_lineare_afa` / `am_afa_ist_anschaffungsjahr` — in einer früheren Runde als Tier-3-Verdacht
notiert, diese Session nicht erneut nachgemessen. Sweep-Treffer heute: `beispielwert=true →
"relevant"` (kein VERDAECHTIG-Treffer der eigenen Sweep-Kriterien) — spricht eher für unauffällig,
aber die frühere Verdachtsbegründung wurde nicht rekonstruiert. **Als offen/unverifiziert
weitergereicht, nicht als entkräftet.**

### Vollklassifikation aller 24 Regeln

| Regel | bool-Gates | Status |
|---|---|---|
| p10_1_3_3a_kv_pv | 0 | Rechen-Weiche (geeicht) — darf nie eine haben |
| p19_2_versorgungsfreibetrag | 0 | Rechen-Weiche (geeicht) — darf nie eine haben |
| p22_1_leibrente_besteuerungsanteil | 0 | Rechen-Weiche (geeicht) — darf nie eine haben |
| p23_veraeusserungsgewinn | 0 | Rechen-Weiche (geeicht) — darf nie eine haben |
| p32_6_kinderfreibetraege | 0 | Rechen-Weiche + Regel selbst tot (bestätigt in Parallel-Report) — darf nie eine haben |
| p2_festzusetzung_zusammen | 0 (eigene) | hat bereits eine echte Ob-Bedingung, über `regel_bedingungen[]`, nicht über eigenes Gate |
| p24a_altersentlastungsbetrag | 1 | hat schon eine korrekte |
| p2_festzusetzung_einzel | 1 | hat schon eine korrekte |
| p21_2_verbilligte_vermietung_wk | 2 | hat schon eine korrekte |
| p9_1_3_nr5a_uebernachtung_nach_48 | 4 | hat schon eine korrekte |
| p9_1_3_nr6_7_arbeitsmittel_afa | 1 | hat schon eine (Wahlrecht-artig, sicher abgefangen) |
| p6_2_gwg_sofortabzug | 3 | hat schon eine korrekte |
| p10_1_5_kinderbetreuung | 1 | hat schon eine korrekte (Qualifikationsgate, Commit 1e140d2) |
| p10_1a_realsplitting | 1 | hat schon eine korrekte |
| p34c_2_abzug_statt_anrechnung | 1 | hat schon eine (Wahlrecht, sicher abgefangen) |
| p16_4_freibetrag | 2 | hat schon eine korrekte |
| p33_1_2_agb_abzug | 4 | hat schon eine korrekte |
| p9_4a_verpflegungsmehraufwand | 3 | hat schon eine korrekte (+1 offenes Tier-2-Zusatzfeld) |
| p9_1_3_nr5_doppelte_haushaltsfuehrung | 4 | hat schon eine korrekte (+1 offenes Tier-2-Zusatzfeld) |
| p7_1_lineare_afa | 1 | unverifiziert (Alt-Verdacht, nicht bestätigt/entkräftet) |
| p2_einkunftsarten | 4 | alle 4 Tier-2-offen, keine sichere vorhanden |
| p34_3_ermaessigter_durchschnittssatz | 3 | **kaputt** — 2 bestätigte Bugs, 1 zirkuläres Antragsgate |
| p34c_1_anrechnung_hoechstbetrag | 1 | **kaputt** — invertiertes Gate, keine Alternative |
| p35a_2_3_haushaltsnahe | 4 | **kaputt** — 1 bestätigter Bug + fehlender Top-Level-Screening-Gate |

### Antwort auf die konkrete Frage

Von 24 Regeln:
- **14 haben heute schon eine funktionierende, korrekt polarisierte Screening-Frage** (13 über
  eigene bool-Gates + p2_festzusetzung_zusammen über `regel_bedingungen[]`).
- **5 dürfen strukturell nie eine haben** — reine Rechen-Weichen ohne bool-Feld
  (p10_1_3_3a_kv_pv, p19_2_versorgungsfreibetrag, p22_1_leibrente_besteuerungsanteil,
  p23_veraeusserungsgewinn, p32_6_kinderfreibetraege).
- **5 brauchen Arbeit**, bevor sie für Julius' Screening-Ansatz taugen: 3 mit bestätigten Bugs
  (p34_3 — zwei Reparaturen + der Antragsgate bleibt zirkulär; p34c_1 — Gate-Polarität umdrehen oder
  Kanal entfernen; p35a_2_3 — Bug reparieren UND neuen Top-Level-Gate bauen), 1 mit ungeklärter
  Fragetext-Polarität ohne Ausweichmöglichkeit (p2_einkunftsarten), 1 unverifizierter Alt-Verdacht
  (p7_1_lineare_afa).

(p9_4a und p9_1_3_nr5 zähle ich in die 14 „hat schon eine" — ihr jeweils EIN Tier-2-Feld ist ein
separates, unabhängig zu klärendes Zusatzproblem, blockiert aber nicht die Screening-Tauglichkeit der
Regel selbst, da eine andere, sicher korrekte Frage in derselben Regel existiert.)

---

## Werkzeuge / Befehle (Nachvollziehbarkeit)

- `tests/test_bindungstabelle.py` (`test_a_feld_id_eindeutig_ueber_alle_dateien`), Mutationsprobe
  (dupliziertes `feld_id`, rückgängig gemacht)
- Dataset/Sweep/Zielmessungen: `python3 -c "..."` gegen `produkt/traverser/traverser.relevanz()`,
  `produkt/store/store.leerer_store()`/`append_event()`, `BINDUNG` aus allen
  `produkt/bindung/bindung_*.yaml` gemergt — volle Skripte und Rohausgabe im Session-Verlauf,
  Kernzahlen: 24 Regeln/100 Felder/41 bool, 12 Sweep-Treffer, 3 bestätigte Bugs.
- `pipeline/produktion/rules.yaml:4230-4258` (p34_3 Geltungsbedingungen inkl. ODER-Text und
  „nur einmal im Leben"), `:5306-5313` (p34c_1 `per_country_ein_staat`)
- `produkt/bindung/bindung_p34c_gesamt.yaml:69-106` (`dba_mehrere_staaten` inkl. Kommentar Z. 99-100)
- `produkt/bindung/bindung_sonder_agb_35a.yaml:381-393` (`p35a_mitveranlagung`)
- `grep -n "mitveranlagung_faktor" pipeline/produktion/rules.yaml` → 0 Treffer
- `produkt/haut/api_constants.py:150,179-180,257,260,425,428,461-475` (Erreichbarkeits-Check
  „gesamt"-Scheibe für alle 3 bestätigten Bugs)
- `reports/adjudikation/relevanz_nicht_bool_blast_radius_2026-08-10.md`, `BACKLOG.yaml` Eintrag
  `relevanz-nicht-bool-gate` (Vorarbeit, geeichte 5-Regel-Referenz, Fix f886768)
- `reports/adjudikation/tote-gate-felder-unfallkosten-analyse-2026-08-12.md` (paralleler Fund zu
  `_scheibe_bindung()`-Erreichbarkeit, genutzt für den Erreichbarkeits-Check oben)
- `BACKLOG.yaml` Eintrag `vpf-frist-null-plausibilitaet` (historischer Bug 519199e, Referenzmuster)
- Kein Push, kein Force, kein `git stash`, keine Produktions-/Testdatei geändert. Nur dieser Report
  geschrieben. Die 3 bestätigten Bugs sind absichtlich NICHT gefixt — außerhalb des Auftragsumfangs
  (reine Messung).
