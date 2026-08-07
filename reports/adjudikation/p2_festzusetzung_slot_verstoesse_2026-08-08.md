# p2_festzusetzung_zusammen / p2_festzusetzung_einzel — 28 signatur_slot-Verstöße geprüft

Datum: 2026-08-08. Auftrag: der größte Cluster der 49 repo-weiten `signatur_slot`-Verstöße
(`p2_festzusetzung_zusammen` 21 + `p2_festzusetzung_einzel` 7 = 28, 57 %) war im Vortagesreport
nur mit „Catala-Scope schmaler als Bindung (dokumentiert)" beschrieben, nicht geprüft. Reiner
Messauftrag: KEIN Code geändert, nichts umbenannt, keine Ausnahmeliste erweitert, nichts committed.

---

## 1. Alle 28 Verstöße (scheibe, feld_id, regel_id, signatur_slot)

Live-Reproduktion von `_n_gefundene_verstoesse` (`test_bindungstabelle.py:1118`) mit dem
Skip-Fallback (`inputs=set()`/`gbs=set()` statt `continue`, exakt wie im Vortagesreport).

| # | scheibe | feld_id | regel_id | signatur_slot |
|---|---|---|---|---|
| 1 | an_gesamt | basis_kv_partner | p2_festzusetzung_zusammen | basis_kv_pv_partner |
| 2 | an_gesamt | basis_pv_partner | p2_festzusetzung_zusammen | basis_kv_pv_partner |
| 3 | an_gesamt | bruttoarbeitslohn | p2_festzusetzung_einzel | bruttoarbeitslohn |
| 4 | an_gesamt | bruttoarbeitslohn_partner | p2_festzusetzung_zusammen | bruttoarbeitslohn_partner |
| 5 | an_gesamt | einkuenfte_gewinn | p2_festzusetzung_einzel | einkuenfte_gewinn |
| 6 | an_gesamt | geburtsjahr_partner | p2_festzusetzung_zusammen | geburtsjahr_partner |
| 7 | an_gesamt | gewst_hebesatz | p2_festzusetzung_einzel | gewst_hebesatz |
| 8 | an_gesamt | gewst_messbetrag | p2_festzusetzung_einzel | gewst_messbetrag |
| 9 | an_gesamt | mit_anspruch_auf_zuschuss_partner | p2_festzusetzung_zusammen | mit_anspruch_auf_zuschuss_partner |
| 10 | an_gesamt | veranlagung | p2_festzusetzung_einzel | veranlagung |
| 11 | an_gesamt | vor_ag_anteil_rv_partner | p2_festzusetzung_zusammen | vor_gesamtbeitraege_partner |
| 12 | an_gesamt | vor_an_anteil_rv_partner | p2_festzusetzung_zusammen | vor_gesamtbeitraege_partner |
| 13 | an_gesamt | vor_rv_ausserhalb_lstb_partner | p2_festzusetzung_zusammen | vor_gesamtbeitraege_partner |
| 14 | an_gesamt | vorsorge_arbeitslosenversicherung_partner | p2_festzusetzung_zusammen | weitere_vorsorgeaufwendungen_partner |
| 15 | an_gesamt | vorsorge_erwerbsunfaehigkeit_partner | p2_festzusetzung_zusammen | weitere_vorsorgeaufwendungen_partner |
| 16 | an_gesamt | vorsorge_rv_alt_mit_ueberschuss_partner | p2_festzusetzung_zusammen | weitere_vorsorgeaufwendungen_partner |
| 17 | an_gesamt | vorsorge_rv_alt_ohne_ueberschuss_partner | p2_festzusetzung_zusammen | weitere_vorsorgeaufwendungen_partner |
| 18 | an_gesamt | vorsorge_unfall_haftpflicht_partner | p2_festzusetzung_zusammen | weitere_vorsorgeaufwendungen_partner |
| 19 | kap_vv_familie | kap_gewinn_aktien_partner | p2_festzusetzung_zusammen | kap_gewinn_aktien_partner |
| 20 | kap_vv_familie | kap_gewinn_sonstige_partner | p2_festzusetzung_zusammen | kap_gewinn_sonstige_partner |
| 21 | kap_vv_familie | kap_kapitalertraege_partner | p2_festzusetzung_zusammen | kap_kapitalertraege_partner |
| 22 | kap_vv_familie | kap_verlust_aktien_partner | p2_festzusetzung_zusammen | kap_verlust_aktien_partner |
| 23 | kap_vv_familie | kap_verlust_sonstige_partner | p2_festzusetzung_zusammen | kap_verlust_sonstige_partner |
| 24 | p3_nr72_pv | [Lücke] | p2_festzusetzung_einzel | pv_entnahmen |
| 25 | p3_nr72_pv | pv_einnahmen | p2_festzusetzung_einzel | einkuenfte_gewinn |
| 26 | rentner | rentner_grad_der_behinderung_partner | p2_festzusetzung_zusammen | grad_der_behinderung_partner |
| 27 | rentner | rentner_hilflos_blind_taubblind_partner | p2_festzusetzung_zusammen | ist_hilflos_blind_taubblind_partner |
| 28 | rentner | rentner_jahresrente_partner | p2_festzusetzung_zusammen | rentner_jahresrente_partner |

`scheibe` hier ist der Dateiname-Fragment aus `bindung_<scheibe>.yaml` (Bindungs-Herkunft) — NICHT
identisch mit `store["scheibe"]` (den API-Ring-Modus, `SCHEIBEN`-Keys `ep`/`an_gesamt`/`gesamt`/
`rentner_gesamt`). `kap_vv_familie`, `p3_nr72_pv`, `rentner` sind Bindungsdateien ohne eigenen
Ring-Modus — ihre feld_ids werden über `TR.lade_bindung()` (Zusammenführung ALLER
`bindung_*.yaml`) in die Ring-Scheiben `gesamt`/`rentner_gesamt` eingemischt (bestätigt via
`api_constants.py`: `GESAMT_PARTNER_KAP` enthält `KAP_ERTRAEGE_PARTNER`/`KAP_TOEPFE_PARTNER`,
`RENTNER_FELDER` enthält die Rentner-Partner-Gruppen).

20 unique `signatur_slot`-Namen (mehrere feld_ids teilen sich einen Summen-Slot).

## 2. Was IST der tatsächliche Input-Satz?

**`_catala_inputs("p2_festzusetzung_zusammen")` und `_catala_inputs("p2_festzusetzung_einzel")`
liefern beide `set()`** — live geprüft:

```python
>>> _catala_inputs("p2_festzusetzung_einzel")
set()
>>> glob.glob("rules/estg/p2_festzusetzung_einzel/*.catala_en")
[]
```

**Ursache: kein Parser-Bug, keine leere Datei — die Datei existiert schlicht nicht unter dem
erwarteten Pfad.** `_catala_inputs` globbt auf `rules/estg/<rule_id>/*.catala_en` — erwartet
also ein VERZEICHNIS namens exakt wie der `rule_id`-String. `find rules/estg -iname
"*p2_festzusetzung*"` liefert nichts. Die beiden Regeln stehen NICHT in `pipeline/produktion/
rules.yaml` (`grep rule_id: rules.yaml` — kein Treffer) — sie fallen in `_n_gefundene_
verstoesse`s dritten Zweig (`else: inputs = _catala_inputs(rid)`), der strukturell NUR den
Verzeichnis-Pfad kennt, keinen alternativen Datei-Fundort.

**Die tatsächliche Catala-Implementierung liegt in `rules/estg/p32a/einkommensteuertarif.
catala_en`** (BACKLOG.yaml Zeile 528 zeigt exakt dorthin), organisiert um den §32a-Tarif, nicht
um die beiden rule_ids:

```
declaration scope FestzusetzendeEstEinzel:           (Zeile 319)
  input bruttoarbeitslohn content money
  input werbungskosten content money
  input sonderausgaben content money
  input veranlagungszeitraum content Veranlagungszeitraum
→ 4 Inputs

declaration scope FestzusetzendeEstZusammen:          (Zeile 382)
  input bruttoarbeitslohn_a content money
  input werbungskosten_a content money
  input bruttoarbeitslohn_b content money
  input werbungskosten_b content money
  input sonderausgaben_gemeinsam content money
  input veranlagungszeitraum content Veranlagungszeitraum
→ 6 Inputs
```

**Bestätigt, dass GENAU diese beiden (engen) Scopes die tatsächliche Produktionsimplementierung
sind** — `golden/runner.py`s zentraler Dispatcher `catala_est(sachverhalt)` (Zeile 1654)
verzweigt bei `"bruttoarbeitslohn_a" in sachverhalt` auf `catala_est_zusammen` (→
`FestzusetzendeEstZusammen`) und bei `"bruttoarbeitslohn" in sachverhalt` auf
`E.festzusetzende_est_einzel` (→ `FestzusetzendeEstEinzel`) — dieselben zwei engen Scopes, keine
Weiterleitung an die breiteren `FestzusetzendeEstGesamt(-Zusammen)`-Scopes (Zeile 457/573
desselben Catala-Files, 18+ Inputs inkl. `einkuenfte_gewinn`), die stattdessen von
`catala_gesamt` (§21 V+V-Pfad, `sachverhalt.get("gesamtfall")`) genutzt werden — ein
UNABHÄNGIGER Rechenpfad, kein Aufruf-Verhältnis zwischen den beiden Scope-Paaren.

**Antwort: „Scope schmaler" trifft zu, ist aber genauer: der Scope existiert, ist eng (4/6
Inputs), UND liegt an einem Pfad, den `_catala_inputs`s Verzeichnis-Konvention nicht findet.**
Beide Befunde stapeln sich hier — nicht nur der eine oder der andere. Selbst wenn der Datei-Pfad
korrekt gefunden würde, blieben 28 Verstöße bestehen: die Bindung enthält Partner-Felder,
Renten-/GewSt-Zusatzfelder und Kapital-Töpfe, die in KEINEM der beiden engen Scopes als `input`
deklariert sind.

## 3. Rechen-Konsument: liest irgendeine `slot_fn` `slots[<einer der 28 Namen>]`?

Alle 3 relevanten `bescheid_via_slots`-Aufrufstellen in `api.py` per AST auf `slots[...]`/
`slots.get(...)`-Zugriffe innerhalb der jeweiligen `slot_fn`-Closure durchsucht
(`festzusetzende_est` Zeile 483-629, `festzusetzende_est_gesamt` Zeile 718-1249,
`festzusetzende_est_rentner` Zeile 1315-1627):

**2 von 20 unique Slot-Namen werden über den generischen `slots`-Parameter gelesen:**

| slot-Name | wo | Zeilen |
|---|---|---|
| `bruttoarbeitslohn` | `festzusetzende_est`-slot_fn | `api.py:574,592,623` (`slots.get("bruttoarbeitslohn", 0)`) |
| `veranlagung` | `festzusetzende_est`-slot_fn | `api.py:590` (`slots.get("veranlagung", "einzel")`) |

Beide liegen exakt in der `slot_fn`, die am Ende `IV.bescheid_via_slots(bindung, slot_fn,
quantitaet="festzusetzende_est")` (Zeile 630) übergibt — und `festzusetzende_est` ist
`gesamt_ring` der `an_gesamt`-Scheibe (`api_constants.py:366`). Innerhalb dieser slot_fn ruft
der `zusammen`-Zweig `runner.catala_est_zusammen(...)` auf (Zeile 572, → `FestzusetzendeEstZusammen`),
der `einzel`-Zweig `runner.catala_est(...)` (Zeile 588, → `FestzusetzendeEstEinzel` via
Dispatcher) — **das ist derselbe Scope-Paar, das Q2 als tatsächliche Implementierung von
`p2_festzusetzung_zusammen`/`_einzel` identifiziert hat.** `bruttoarbeitslohn`/`veranlagung`
sind also nicht nur „irgendein" Konsument, sondern GENAU der Rechenweg dieser beiden regel_ids.

**Die anderen 18 Namen sind NICHT über `slots[...]` gelesen** (0 Treffer in allen drei
`slot_fn`-Rümpfen) — sie werden, wie schon bei p19_2 bestätigt, über DIREKTEN Feld-ID-Zugriff
gelesen (Bypass des generischen Slot-Mechanismus). Fundstellen:

| feld_id(s) | slot-Name | wo gelesen (Datei:Zeile) |
|---|---|---|
| `bruttoarbeitslohn_partner` | `bruttoarbeitslohn_partner` | `api.py:575` (`_cent("bruttoarbeitslohn_partner")`, in `festzusetzende_est`-slot_fn, zusammen-Zweig), auch `api.py:864,908` in `festzusetzende_est_gesamt` |
| `basis_kv_partner`+`basis_pv_partner` | `basis_kv_pv_partner` | `api.py:569` (`_cent("basis_kv_partner")+_cent("basis_pv_partner")`, `festzusetzende_est`-slot_fn) |
| `mit_anspruch_auf_zuschuss_partner` | `mit_anspruch_auf_zuschuss_partner` | `api.py:571` (`f.get("mit_anspruch_auf_zuschuss_partner", {})`) |
| 5× `vorsorge_*_partner` | `weitere_vorsorgeaufwendungen_partner` | `api.py:570` (Summe direkt aus 5 Feld-IDs) |
| `einkuenfte_gewinn` | `einkuenfte_gewinn` | `api.py:204` (`_c("einkuenfte_gewinn")`, in `_laufender_gewinn`, aufgerufen aus `festzusetzende_est_gesamt`-slot_fn `api.py:885` und `festzusetzende_est_rentner`-slot_fn `api.py:1393`) |
| `gewst_hebesatz`/`gewst_messbetrag` | dito | `api.py:1067/1487` bzw. `1066/1486` |
| `geburtsjahr_partner` | dito | `api.py:907` |
| `vor_an/ag_anteil_rv_partner`+`vor_rv_ausserhalb_lstb_partner` | `vor_gesamtbeitraege_partner` | `api.py:930-932` (`festzusetzende_est_gesamt`), `api.py:1475-1477` (`festzusetzende_est_rentner`) |
| `kap_gewinn/verlust_aktien/sonstige_partner` | dito | `api.py:1017-1020` (`festzusetzende_est_gesamt`), `api.py:1447-1450` (`festzusetzende_est_rentner`) |
| `kap_kapitalertraege_partner` | dito | `api.py:1022,1452,1893` — via Modul-Konstante `KAP_ERTRAEGE_PARTNER = "kap_kapitalertraege_partner"` (`api_constants.py:93`), NICHT als Inline-Literal (Literal-Grep würde das übersehen) |
| `rentner_grad_der_behinderung_partner` | `grad_der_behinderung_partner` | `api.py:987,1372` — Feld-ID-Präfix `rentner_` weicht vom Slot-Namen ab, gleiches Muster wie p19_2 |
| `rentner_hilflos_blind_taubblind_partner` | `ist_hilflos_blind_taubblind_partner` | `api.py:988,1373` — dito |
| `rentner_jahresrente_partner` | dito | `api.py:1339`, innerhalb `_rente_instanz()`-Closure |

**`basis_kv_pv_partner`, `vor_gesamtbeitraege_partner`, `weitere_vorsorgeaufwendungen_partner`
existieren nirgends als eigene `feld_id`** (`grep "feld_id.*<name>" produkt/bindung/*.yaml` →
leer für alle drei) — erwartungsgemäß, sie sind reine Aggregations-Zielnamen mehrerer feld_ids,
kein eigenständiges Datenfeld.

**1 von 20 Namen ist eine dokumentierte, bewusste Lücke:** `pv_entnahmen`
(`bindung_p3_nr72_pv.yaml:88-93`, `[Lücke]`-Eintrag): „§ 3 Nr. 72 nennt Einnahmen UND Entnahmen.
Der Selbstverbrauch (Entnahme) ist im Feld pv_einnahmen mit erfasst (Laien-Formulierung 'plus
Wert des selbst verbrauchten Stroms'); ein getrenntes Entnahme-Feld wäre eine eigene
Bewertungsfrage (Teilwert) und ist Stufe-2." Kein Leser, weil bewusst noch nicht gebaut — kein
stiller Fund.

**Antwort: JA — 2 der 28 Verstöße (`bruttoarbeitslohn`, `veranlagung`) treffen einen echten
Leser über exakt den generischen Mechanismus, den `test_n`s Rückrichtung prüfen würde, UND
dieser Leser ist derselbe Rechenweg, den Q2 als die tatsächliche Implementierung dieser beiden
regel_ids identifiziert hat. Die restlichen 26 sind KEIN KeyError/None-Risiko, weil sie den
generischen Mechanismus gar nicht durchlaufen — sie werden per Feld-ID direkt gelesen (25) oder
sind eine dokumentierte Lücke (1).**

## 4. Geldrelevanz — gemessen mit `/ergebnis`

Für die beiden einzigen echten `slots[...]`-Leser (`bruttoarbeitslohn`, `veranlagung`) empirisch
über `/fall/<id>/ergebnis` gemessen (Scheibe `an_gesamt`, VZ 2025, voller bestätigter Kegel,
identischer Aufbau wie `test_an_gesamt_durchstich`):

```
=== bruttoarbeitslohn 40.000€ vs 80.000€ (veranlagung=einzel) ===
40.000 €: 662900 ct  grund=bestaetigt
80.000 €: 2176700 ct  grund=bestaetigt
Delta: 1513800 ct (15.138 €)

=== veranlagung einzel vs zusammen (je 40.000€ Brutto, Partner-Kegel vollständig) ===
einzel:   662900 ct  grund=bestaetigt
zusammen: 1354600 ct  grund=bestaetigt
```

Beide Slots bewegen die festgesetzte Steuer eindeutig und in die erwartete Richtung (höheres
Einkommen → mehr Steuer; Zusammenveranlagung bei identischem Doppel-Brutto → andere Steuer als
Einzelveranlagung, Splitting-Effekt). **Geldrelevant, gemessen — nicht nur gelesen.**

Für die 25 direkt-gelesenen Feld-IDs wurde KEIN eigener `/ergebnis`-Differenzbeweis gefahren
(Auftrag verlangte das nur „wenn 3 einen echten Leser findet" — die 25 sind keine `slots[...]`-
Leser im Sinne von Frage 3, sondern Feld-ID-Direktzugriffe; ihre Geldwirkung ist an anderer
Stelle bereits belegt, z. B. `basis_kv_partner`/`basis_pv_partner` fließen sichtbar in
`kv_pv_b` → `sonderausgaben_gemeinsam` bei `catala_est_zusammen`, Zeile 568-577 — aber das ist
Code-Lesen, kein eigener Messlauf).

## Zusammenfassung, wenn gefragt

**Der zentrale Festsetzungspfad hat genau 2 Verstöße mit echter Konsequenz, nicht 28.**
`bruttoarbeitslohn`/`veranlagung` sind reale, geldwirksame Leser über exakt den Mechanismus, den
`test_n` prüfen würde — kein KeyError-Risiko heute (weil `bescheid_via_slots` per `.get()` mit
Default liest), aber ein Fund, der bei einer Bindungs-Umbenennung ODER bei künftiger
`test_n`-Anbindung dieser regel_ids sofort relevant würde. Die übrigen 26 sind architektonisch
dasselbe Muster wie p19_2 (Feld-ID-Direktzugriff, Slot-Mechanismus umgangen) — folgenlos für den
`signatur_slot`-Namens-Mismatch selbst, aber NICHT folgenlos für die Steuerberechnung insgesamt:
die zugrundeliegenden Daten (Partner-Kapitalerträge, GewSt-Anrechnung, Renten-Partner-Freibeträge
usw.) fließen sehr wohl in die festgesetzte Steuer ein — nur eben nicht über den `signatur_slot`,
den die Bindung dafür angibt. Der Bindungs-Eintrag selbst ist an diesen 26 Stellen irreführende
Dokumentation (Slot-Name verspricht einen Mechanismus, der nicht benutzt wird), aber keine
Rechenlücke.

## Nicht gemessen

1. `/ergebnis`-Differenzbeweis für die 25 direkt-gelesenen Feld-IDs einzeln (nur `bruttoarbeitslohn`/
   `veranlagung` gemessen, per Auftrag auf „wenn 3 einen echten Leser findet" begrenzt — die 25
   sind keine `slots[...]`-Leser, s.o.).
2. Ob die 21 `an_gesamt`/`kap_vv_familie`/`rentner`-Verstöße, die über Feld-ID-Direktzugriff laufen,
   dieselbe systematische Umbenennungs- oder Ausnahmelisten-Lösung wie p19_2/p7_1_lineare_afa
   verdienen — Bewertungsfrage, außerhalb des Messauftrags.
3. Warum `p2_festzusetzung_zusammen`/`_einzel` nicht in `rules.yaml` stehen (Design-Entscheidung
   oder historische Lücke) — nicht recherchiert, für die gestellten 4 Fragen nicht nötig.

## GATE

Befehl: `timeout 590 python3 -m pytest -q` (viertes Attempt — zwei vorherige liefen wegen
konkurrierender Prozesse in Timeout/Terminierung, s. u.):
```
1655 passed, 4 skipped, 1 warning in 240.47s (0:04:00)
```
Exit-Code: 0. Identisch zur Referenz (1655/4). Kein Code geändert in dieser Sitzung — keine
Verschiebung erwartet, keine gemessen.

(Zwei frühere Läufe in dieser Sitzung liefen in Timeout/Terminierung, verursacht durch parallele
konkurrierende pytest-Prozesse anderer Agents im selben Repo — kein Befund, nur Ressourcen-
Konkurrenz. Der zitierte Lauf ist der erste, der sauber durchlief.)

## Status

Kein Code geändert. Nichts umbenannt. Keine Ausnahmeliste erweitert. Nichts committed.
Temp-Skript `/tmp/p2_q4/measure.py` (Q4-Messung) bleibt in `/tmp`, nicht im Repo.

---

## Nachtrag — main's Korrektur + Mutationsmessung (2026-08-08, zweiter Durchgang)

### Korrektur 1: die "28" waren zu einem Teil Simulationsartefakt

Meine Q1-Zahl (28) kam aus dem Skip-Fallback `inputs=set()`/`gbs=set()` — wenn `_catala_inputs`
leer liefert, wird JEDE Bindung dieser regel_id automatisch als "Verstoß" gezählt, unabhängig
davon, ob der Slot-Name wirklich falsch ist. Das ist eine Eigenschaft der Simulation, kein
Befund über die echten Scope-Inputs.

Main hat gegen die ECHTEN, jetzt lokalisierten Scope-Inputs (`FestzusetzendeEstEinzel`:
`bruttoarbeitslohn`/`werbungskosten`/`sonderausgaben`/`veranlagungszeitraum`;
`FestzusetzendeEstZusammen`: `bruttoarbeitslohn_a/_b`/`werbungskosten_a/_b`/
`sonderausgaben_gemeinsam`/`veranlagungszeitraum`) nachgemessen:

| | mit `inputs=set()` (meine Simulation) | gegen echte Scope-Inputs (main) |
|---|---|---|
| `p2_festzusetzung_einzel` | 6 | **5 echte Mismatches** — nur `bruttoarbeitslohn` korrekt benannt |
| `p2_festzusetzung_zusammen` | 21 | **21 echte Mismatches — 0 von 21 korrekt benannt** |
| Summe | 27 | 26 |

(Meine Tabelle in Abschnitt 1 zählte 28, nicht 27 — Differenz ist der `[Lücke]`-Eintrag #24
`pv_entnahmen`, der in main's Zählung separat behandelt wird, s. Abschnitt 3 oben. Ändert nichts
an der Kernaussage.)

Für `zusammen` ist das eine schärfere Aussage als "Catala-Scope schmaler dokumentiert": es ist
nicht nur schmaler (6 statt ~34 Inputs), sondern die Bindung trifft in KEINEM einzigen Fall den
Scope-Namen — `bruttoarbeitslohn_partner` vs. Scope-Wunsch `bruttoarbeitslohn_b`, gleiches
Muster bei allen anderen. 0/21 korrekt.

### Korrektur 2: kein "slots[...]"-Lesen — `.get()` MIT Default, fail-open

Meine Formulierung in Abschnitt 3 ("2 von 20 ... werden über den generischen `slots`-Parameter
gelesen") war technisch zutreffend zitiert, aber zu schwach charakterisiert. Präzise:

```
api.py:589   "veranlagung":        slots.get("veranlagung", "einzel")
api.py:592   "bruttoarbeitslohn":  int(slots.get("bruttoarbeitslohn", 0)) // 100
api.py:574   "bruttoarbeitslohn_a": int(slots.get("bruttoarbeitslohn", 0)) // 100
```

Das sind `.get(key, default)`-Aufrufe, kein `slots[key]`. Ein falscher/umbenannter Slot-Name
wirft KEIN `KeyError` — er liefert still den Default (`0` bzw. `"einzel"`) und rechnet damit
weiter. Fail-open, nicht fail-closed. Dieselbe Bauart, die bei der Gate-Polarität
([[gate-polaritaet-normalfall-antwort]]) und beim vorläufig-Filter bereits echte Geldfehler
verursacht hat.

### Mutationsmessung — main's Auftrag, ausgeführt

**Setup**: Backup `/tmp/slot_probe/bindung_an_gesamt.yaml` von main vorab angelegt, verifiziert
(60463 Bytes, `diff` byte-identisch zur Live-Datei vor jeder Mutation). Restore ausschließlich
via `cp`, nie `git checkout` — eingehalten.

**Mutation**: `produkt/bindung/bindung_an_gesamt.yaml:15`, `signatur_slot` von
`bruttoarbeitslohn` → `bruttoarbeitslohn_x` (einziger geänderter Wert).

**Basisfall**: Scheibe `an_gesamt`, VZ 2025, voller bestätigter Pflicht-Kegel,
`bruttoarbeitslohn=6000000` (60.000 €), `veranlagung=einzel` — via echtem HTTP-Server
(`server.make_server`), `POST /fall` → `POST /fall/<id>/event` (Laien-Herkunft, bestätigt) →
`GET /fall/<id>/ergebnis`.

| | zahl_cent | grund |
|---|---|---|
| VOR Mutation | **1.356.800** (13.568 €) | bestaetigt |
| NACH Mutation | **0** | bestaetigt |

**Main's Hypothese bestätigt, nicht falsifiziert**: die Steuer fällt still auf den 0-Lohn-Wert.
`grund` bleibt `"bestaetigt"` — keine Fehlermeldung, kein Statuswechsel, nichts wird rot im
Ergebnis selbst. **Differenz: 1.356.800 ct = 13.568 € verschwinden spurlos.** Kein Naming-Problem
mehr, sondern eine scharfe Kante: jede Umbenennung von `signatur_slot: bruttoarbeitslohn` ist ein
stiller Steuerfehler, kein Crash.

**Scoped Tests mit aktiver Mutation:**

```
$ timeout 590 python3 -m pytest -q tests/test_bindungstabelle.py
26 passed in 53.40s
```

**Grün.** Kein Bindungs-Struktur-Gate fängt die Umbenennung — `test_n` prüft ja gerade NICHT
diesen Rechenpfad (`p2_festzusetzung_einzel` steht in `REGELN_OHNE_GROUND_TRUTH`, s. Bonusfrage
unten), und kein anderer Test in dieser Datei prüft `/ergebnis`-Zahlen.

```
$ timeout 590 python3 -m pytest -q tests/test_paket_b_e2e_http.py -k test_an_gesamt_durchstich
FAILED tests/test_paket_b_e2e_http.py::test_an_gesamt_durchstich - assert 0 == 662900
1 failed, 194 deselected in 2.80s
```

**Rot.** Genau ein gezielter Ring-Wert-Test (`assert stand["intervall"]["min_cent"] ==
stand["intervall"]["max_cent"] == 662900`) fängt die Mutation — weil er den exakten Cent-Betrag
hart asserted, nicht weil er den Slot-Namen prüft. Ändere den erwarteten Wert oder lösche den
Assert, und auch dieser Test bliebe blind.

**Restore**: `cp /tmp/slot_probe/bindung_an_gesamt.yaml produkt/bindung/bindung_an_gesamt.yaml`
(kein `git checkout`, wie verlangt). Danach:

```
$ git diff --stat produkt/bindung/bindung_an_gesamt.yaml
(leer)
```

Repo-weiter `git diff --stat` zeigt zum Zeitpunkt dieses Reports nur `produkt/haut/api.py`
(dev-b's parallele, unabhängige Änderung — nicht angefasst, wie verlangt).

**Fazit Mutationsmessung**: genau EIN Test im ganzen Repo bewacht diesen 13.568-€-Pfad, und der
bewacht ihn zufällig (Cent-Betrag-Assert), nicht strukturell. `test_bindungstabelle.py`, das
eigentliche Bindungs-Gate, sieht die Umbenennung nicht.

### Bonusfrage: warum stehen `p2_festzusetzung_einzel`/`_zusammen` nicht in `rules.yaml`?

**Bewusste, bereits dokumentierte Entscheidung — keine offene Lücke, keine Vergesslichkeit.**
Beleg direkt im Code: `tests/test_bindungstabelle.py:1011-1017` (Kommentar zu
`REGELN_OHNE_GROUND_TRUTH`):

> "Catala-Scope ist schmaler als die Bindung: FestzusetzendeEstEinzel kennt 4 Inputs, die
> Bindung fuehrt zusaetzlich veranlagung/gewst_*/einkuenfte_gewinn (+ bei _zusammen ~34
> Partner-Slots). [...] Ein Anschluss wuerde 5 bzw. ~39 Schein-Verstoesse erzeugen — gemessen
> und deshalb verworfen."

Deckungsgleich in `BACKLOG.yaml:481-486` (`rest_offen`): ein `rules.yaml`-Eintrag für diese
beiden regel_ids WÜRDE `test_n` eine Ground Truth geben — genau main's Punkt, die Skip-Liste
würde um 2 kürzer. Der Grund, warum das NICHT einfach gemacht wurde: ein `rules.yaml`-Eintrag
mit dem schmalen Scope als `geltungsbedingungen`/Signatur würde die 5 (einzel) bzw. 21 (zusammen)
echten Feld-ID-Direktzugriffe (Abschnitt 3 oben: `bruttoarbeitslohn_partner`,
`einkuenfte_gewinn`, `gewst_*`, alle Partner-Kapital-/Renten-/Vorsorge-Felder) automatisch als
`test_n`-Verstöße aufdecken — nicht weil sie falsch sind, sondern weil sie NICHT über den
schmalen `p2_festzusetzung_*`-Scope laufen, sondern über direkte Feld-ID-Zugriffe in
`festzusetzende_est_gesamt`/`_rentner` (andere Ring-Zweige, andere Scopes). Ein naiver
`rules.yaml`-Anschluss würde also den echten Bug (Korrektur 1/2 oben, `bruttoarbeitslohn`/
`veranlagung` via `.get()`-Default) NICHT gezielt aufdecken, sondern zusätzlich ~26 Schein-Alarme
erzeugen, die main's eigenes Team schon einmal gemessen und deshalb bewusst zurückgestellt hat.

**Das ist KEIN Widerspruch zur Mutationsmessung oben** — im Gegenteil, es erklärt, warum die
Lücke bis heute nicht sichtbar wurde: `test_n` könnte `bruttoarbeitslohn`/`veranlagung` gezielt
prüfen, WENN die Ground-Truth-Frage (welcher Scope ist maßgeblich für diese 2 Slots, getrennt
von den 26 Direktzugriffs-Feldern) zuerst geklärt wäre. Das ist eine Bewertungsfrage
(rules.yaml-Struktur), keine Messfrage — außerhalb dieses Auftrags, aber jetzt mit der
Mutationsmessung als konkretem Beleg dafür, dass die Lücke geldrelevant ist (13.568 €), nicht
nur kosmetisch.

## Status (Nachtrag)

Mutation angewendet und wieder entfernt, ausschließlich via `cp` aus `/tmp/slot_probe/`, nie
`git checkout`. `git diff --stat produkt/bindung/bindung_an_gesamt.yaml` leer nach Restore.
`produkt/haut/api.py` nicht angefasst (dev-b's Datei). Nur `tests/test_bindungstabelle.py` +
`test_an_gesamt_durchstich` gelaufen, nicht die volle Suite. Nichts committed. Kein Code-Fix —
reiner Messauftrag, wie verlangt.
