# p2_festzusetzung_* Slot-Verstoesse: A/B-Klassifikation je Fall

Datum: 2026-08-12. Auftrag: team-lead, "Ordne jeden der 28 Verstoesse einzeln in (A) oder (B) ein,
mit Beleg je Fall." NUR ANALYSE, keine Produktions-/Testdatei geaendert.

Baut auf `reports/adjudikation/p2_festzusetzung_slot_verstoesse_2026-08-08.md` auf (4 Tage alt,
608 Zeilen, bereits die Vorarbeit fuer genau diese Frage). Dieser Bericht **wiederholt sie nicht**,
sondern (1) verifiziert ihre Kernaussage heute unabhaengig gegen aktuellen HEAD, (2) praezisiert
eine Ungenauigkeit im Auftrag (die Verstoesse sind heute nicht live als Testversagen sichtbar),
(3) misst eine Drift (28 → 42 seit 2026-08-08), (4) liefert die geforderte Zeile-fuer-Zeile A/B-Tabelle
fuer alle 42, (5) beantwortet die Nebenfrage.

**Sofort-Befund bereits an team-lead gemeldet** (SendMessage, vor diesem Bericht): der einzige
(B)-Fall, Zeile 3 unten.

## 0. Praezisierung: was der Auftrag "28 Verstoesse" nennt, ist heute keine Testversagen-Zahl

**Gemessen**, `tests/test_bindungstabelle.py:1159-1193` (`_n_gefundene_verstoesse`), Zeile 1184:

```python
inputs = _catala_inputs(rid)
if not inputs and not glob.glob(os.path.join(ROOT, "rules", "estg", rid, "*.catala_en")):
    uebersprungen.add(rid)
    continue
```

`p2_festzusetzung_einzel`/`_zusammen` stehen in `REGELN_OHNE_GROUND_TRUTH`
(`tests/test_bindungstabelle.py:1052-1058`, mit eigener Begruendung im Kommentar). Ihre Bindungen
erreichen die Slot-Pruef-Schleife (Zeilen 1188 ff.) **nie** — `continue` greift vorher.
`test_n_bindung_zeigt_auf_existierende_bedingung` prueft das sogar explizit gegenlaeufig
(Zeile 1099: `assert uebersprungene_regeln == REGELN_OHNE_GROUND_TRUTH`) und ist damit heute **gruen**,
nicht rot.

Frisch nachgefahren:

```bash
python3 -c "
import tests.test_bindungstabelle as T
daten = {f: T._load(f) for f in T._bindung_files()}
gb, slot, skipped = T._n_gefundene_verstoesse(daten, T._rules())
print('p2_festzusetzung_einzel' in skipped, 'p2_festzusetzung_zusammen' in skipped)
print(len(slot))
"
# -> True True
# -> 13
```

Die 13 real gefundenen `gefunden_slot`-Eintraege (ohne Ausnahmeliste) gehoeren zu anderen Regeln
(p9_4a_verpflegungsmehraufwand/p7_1_lineare_afa, dort seit laengerem als
`SIGNATUR_SLOT_ZEIGT_INS_LEERE` dokumentiert) — nicht zu p2_festzusetzung.

Die "28" (bzw. "49" gesamt) im Auftrag stammen aus dem Bericht vom 2026-08-08, der **explizit als
Simulation** deklariert ist ("Skip-Fallback (`inputs=set()`/`gbs=set()` statt `continue`)" — dessen
eigene Formulierung). Das ist **keine Korrektur der Auftragsstellung** — die A/B-Frage ist genau so
richtig und wichtig gestellt, nur die Zahl ist eine manuelle Diagnose, kein Gate-Rot. Wird unten
fortgesetzt, mit derselben Methodik, frisch gegen heutigen HEAD.

## 1. Zwei unabhaengige Belegwege fuer die Scope-Wahrheit (heute erneut gemessen)

**Weg A — Quelle**, `rules/estg/p32a/einkommensteuertarif.catala_en`:

```bash
grep -n "^declaration scope FestzusetzendeEst\|^  input " rules/estg/p32a/einkommensteuertarif.catala_en
```

- `FestzusetzendeEstEinzel` (Zeile 319): Inputs `bruttoarbeitslohn`, `werbungskosten`,
  `sonderausgaben`, `veranlagungszeitraum` — **4**.
- `FestzusetzendeEstZusammen` (Zeile 382): Inputs `bruttoarbeitslohn_a`, `werbungskosten_a`,
  `bruttoarbeitslohn_b`, `werbungskosten_b`, `sonderausgaben_gemeinsam`, `veranlagungszeitraum` —
  **6**.
- Daneben existieren `FestzusetzendeEstGesamt` (Zeile 457) und `FestzusetzendeEstGesamtZusammen`
  (Zeile 554) mit 18+ Inputs (u.a. `einkuenfte_gewinn`, `freibetraege_kinder`,
  `steuerermaessigungen`) — das ist ein **eigener** Pfad (`catala_gesamt`, ueber
  `sachverhalt["gesamtfall"]`), nicht der von `p2_festzusetzung_*` gebundene. Ein Slotname, der
  dort passt, zaehlt hier trotzdem nicht.

**Weg B — kompilierte Signatur**, `golden/runner.py`:

- `catala_est()` (Zeile 1665) baut `E.FestzusetzendeEstEinzelIn(bruttoarbeitslohn_in=...,
  werbungskosten_in=..., sonderausgaben_in=..., veranlagungszeitraum_in=...)` — 4 Felder, 1:1 mit
  Weg A (nach Abzug des `_in`-Suffix).
- `catala_est_zusammen()` (Zeile 1779) baut `E.FestzusetzendeEstZusammenIn(bruttoarbeitslohn_a_in=...,
  bruttoarbeitslohn_b_in=..., werbungskosten_a_in=..., werbungskosten_b_in=...,
  sonderausgaben_gemeinsam_in=..., veranlagungszeitraum_in=...)` — 6 Felder, 1:1 mit Weg A.

Beide Wege stimmen exakt überein. **Gemessen**, unveraendert gegenueber dem Stand vom 2026-08-08
(Zeilennummern verschoben durch 4 Tage fremde Commits, Inhalt identisch).

## 2. Frische Simulation gegen heutigen HEAD: 42 statt 28

```bash
python3 <<'EOF'
import tests.test_bindungstabelle as T
daten = {f: T._load(f) for f in T._bindung_files()}
rows = []
for f, d in daten.items():
    scheibe = f.split("bindung_")[-1][:-5]
    for b in d["bindungen"]:
        if b["quelle"]["regel_id"] in ("p2_festzusetzung_einzel", "p2_festzusetzung_zusammen"):
            q = b["quelle"]
            if "signatur_slot" in q:
                rows.append((scheibe, b["feld_id"], q["regel_id"], q["signatur_slot"]))
    for l in d.get("luecken", []):
        if l["regel_id"] in ("p2_festzusetzung_einzel", "p2_festzusetzung_zusammen") and l.get("signatur_slot"):
            rows.append((scheibe, "[Luecke]", l["regel_id"], l["signatur_slot"]))
print(len(rows))
EOF
# -> 42
```

Aufschluesselung: **21 `p2_festzusetzung_zusammen` (unveraendert zum 2026-08-08-Bericht) + 21
`p2_festzusetzung_einzel` (gewachsen von 7 auf 21, +14)**.

```bash
git log --oneline --since="2026-08-08" -- produkt/bindung/bindung_an_gesamt.yaml \
  produkt/bindung/bindung_kap_vv_familie.yaml produkt/bindung/bindung_rentner.yaml \
  produkt/bindung/bindung_p3_nr72_pv.yaml produkt/bindung/bindung_p51a_kirchensteuer.yaml
```
→ 8 Commits (Stammdaten Name/Adresse/Geburtsdatum/Steuernummer, IBAN/BIC, Steuerklasse/Kirchensteuer,
KAP-Antragsgrund + q-Anrechnung Stufe 3). Jedes neue Feld wurde — normale, korrekte Praxis — an
`p2_festzusetzung_einzel` gebunden und erbt damit denselben Skip. **Kein Bug**, aber die 14 neuen
Zeilen wurden nie gegen die A/B-Frage geprueft, weil der alte Bericht vor ihrer Einfuehrung
geschrieben wurde.

Nebenbefund: Der Code-Kommentar in `REGELN_OHNE_GROUND_TRUTH` selbst (Zeile 1056: "Ein Anschluss
wuerde 5 bzw. ~39 Schein-Verstoesse erzeugen") ist heute **auch** ungenau — real 21/21, nicht 5/~39.
Reine Dokumentationsdrift, keine Funktionsfrage; der Hinweis steht direkt im Testfile, nicht nur
im BACKLOG, deshalb hier vermerkt.

## 3. Klassifikation aller 42 Zeilen

Methodik: jeder `signatur_slot` gegen die 4 Einzel- bzw. 6 Zusammen-Inputs aus Abschnitt 1 verglichen
(exakter Namensvergleich, beide Belegwege stimmen ueberein). `(A)` = kein Treffer, Scope wirklich
schmaler. `(B)` = Treffer, aber `_catala_inputs()` findet ihn nicht (Verzeichnisnavigation).
`[Luecke]` = eigene, bereits dokumentierte Kategorie (kein Slot, sondern ein deklarierter Deckungsluecke-Eintrag mit eigenem `grund`).

| # | Datei (Scheibe) | feld_id | regel_id | signatur_slot | Urteil | Beleg |
|---|---|---|---|---|---|---|
| 1 | an_gesamt | basis_kv_partner | zusammen | basis_kv_pv_partner | A | kein Zusammen-Input heisst so |
| 2 | an_gesamt | basis_pv_partner | zusammen | basis_kv_pv_partner | A | s.o. |
| 3 | an_gesamt | bruttoarbeitslohn | einzel | **bruttoarbeitslohn** | **B** | Einzel-Input Nr.1 (Abschnitt 1) — Namensgleich, `_catala_inputs("p2_festzusetzung_einzel")` sucht in `rules/estg/p2_festzusetzung_einzel/*.catala_en`, Datei liegt aber unter `rules/estg/p32a/` |
| 4 | an_gesamt | bruttoarbeitslohn_partner | zusammen | bruttoarbeitslohn_partner | A | Zusammen-Inputs heissen `_a`/`_b`, nicht `_partner` |
| 5 | an_gesamt | einkuenfte_gewinn | einzel | einkuenfte_gewinn | A | nur Input von `FestzusetzendeEstGesamt` (anderer Scope/Pfad), nicht von `FestzusetzendeEstEinzel` |
| 6 | an_gesamt | geburtsjahr_partner | zusammen | geburtsjahr_partner | A | kein Zusammen-Input |
| 7 | an_gesamt | gewst_hebesatz | einzel | gewst_hebesatz | A | kein Einzel-Input; laeuft ueber `catala_gewst`, eigener Dispatch-Zweig in `catala_est()` |
| 8 | an_gesamt | gewst_messbetrag | einzel | gewst_messbetrag | A | s.o. |
| 9 | an_gesamt | mit_anspruch_auf_zuschuss_partner | zusammen | mit_anspruch_auf_zuschuss_partner | A | kein Zusammen-Input |
| 10 | an_gesamt | stammdaten_art_est_erklaerung | einzel | stammdaten_art_est_erklaerung | A | Stammdatum, Dispatch-Ebene, kein Scope-Input |
| 11 | an_gesamt | stammdaten_bic | einzel | stammdaten_bic | A | s.o. |
| 12 | an_gesamt | stammdaten_geburtsdatum | einzel | stammdaten_geburtsdatum | A | s.o. |
| 13 | an_gesamt | stammdaten_hausnummer | einzel | stammdaten_hausnummer | A | s.o. |
| 14 | an_gesamt | stammdaten_iban | einzel | stammdaten_iban | A | s.o. |
| 15 | an_gesamt | stammdaten_keine_bankverbindung | einzel | stammdaten_keine_bankverbindung | A | s.o. |
| 16 | an_gesamt | stammdaten_nachname | einzel | stammdaten_nachname | A | s.o. |
| 17 | an_gesamt | stammdaten_plz | einzel | stammdaten_plz | A | s.o. |
| 18 | an_gesamt | stammdaten_steuernummer | einzel | stammdaten_steuernummer | A | s.o. |
| 19 | an_gesamt | stammdaten_strasse | einzel | stammdaten_strasse | A | s.o. |
| 20 | an_gesamt | stammdaten_vorname | einzel | stammdaten_vorname | A | s.o. |
| 21 | an_gesamt | stammdaten_wohnort | einzel | stammdaten_wohnort | A | s.o. |
| 22 | an_gesamt | steuerklasse | einzel | steuerklasse | A | kein Einzel-Input |
| 23 | an_gesamt | veranlagung | einzel | veranlagung | A | Einzel-Input heisst `veranlagungszeitraum`, nicht `veranlagung` — kein exakter Treffer trotz Namensnaehe |
| 24 | an_gesamt | vor_ag_anteil_rv_partner | zusammen | vor_gesamtbeitraege_partner | A | kein Zusammen-Input |
| 25 | an_gesamt | vor_an_anteil_rv_partner | zusammen | vor_gesamtbeitraege_partner | A | s.o. |
| 26 | an_gesamt | vor_rv_ausserhalb_lstb_partner | zusammen | vor_gesamtbeitraege_partner | A | s.o. |
| 27 | an_gesamt | vorsorge_arbeitslosenversicherung_partner | zusammen | weitere_vorsorgeaufwendungen_partner | A | kein Zusammen-Input |
| 28 | an_gesamt | vorsorge_erwerbsunfaehigkeit_partner | zusammen | weitere_vorsorgeaufwendungen_partner | A | s.o. |
| 29 | an_gesamt | vorsorge_rv_alt_mit_ueberschuss_partner | zusammen | weitere_vorsorgeaufwendungen_partner | A | s.o. |
| 30 | an_gesamt | vorsorge_rv_alt_ohne_ueberschuss_partner | zusammen | weitere_vorsorgeaufwendungen_partner | A | s.o. |
| 31 | an_gesamt | vorsorge_unfall_haftpflicht_partner | zusammen | weitere_vorsorgeaufwendungen_partner | A | s.o. |
| 32 | kap_vv_familie | kap_gewinn_aktien_partner | zusammen | kap_gewinn_aktien_partner | A | kein Zusammen-Input |
| 33 | kap_vv_familie | kap_gewinn_sonstige_partner | zusammen | kap_gewinn_sonstige_partner | A | s.o. |
| 34 | kap_vv_familie | kap_kapitalertraege_partner | zusammen | kap_kapitalertraege_partner | A | s.o. |
| 35 | kap_vv_familie | kap_verlust_aktien_partner | zusammen | kap_verlust_aktien_partner | A | s.o. |
| 36 | kap_vv_familie | kap_verlust_sonstige_partner | zusammen | kap_verlust_sonstige_partner | A | s.o. |
| 37 | p3_nr72_pv | [Luecke] | einzel | pv_entnahmen | **[Luecke]** | eigener `grund` in der YAML (§ 3 Nr.72 Entnahme = Bewertungsfrage, Stufe-2) — keine A/B-Frage, bereits dokumentiert |
| 38 | p3_nr72_pv | pv_einnahmen | einzel | einkuenfte_gewinn | A | wie Zeile 5 — Input existiert nur im `FestzusetzendeEstGesamt`-Scope |
| 39 | p51a_kirchensteuer | kirchensteuer_arbeitgeber | einzel | kirchensteuer_arbeitgeber | A | kein Einzel-Input |
| 40 | rentner | rentner_grad_der_behinderung_partner | zusammen | grad_der_behinderung_partner | A | kein Zusammen-Input |
| 41 | rentner | rentner_hilflos_blind_taubblind_partner | zusammen | ist_hilflos_blind_taubblind_partner | A | s.o. |
| 42 | rentner | rentner_jahresrente_partner | zusammen | rentner_jahresrente_partner | A | s.o. |

**Ergebnis: 1×(B), 1×[Luecke], 40×(A).**

Alle (A)-Faelle sind — konsistent mit dem 2026-08-08-Bericht, dort mit Datei:Zeile fuer jede
Groesse belegt — echte Werte, die im Ring direkt per Feld-ID gelesen werden (api.py, ausserhalb des
schmalen Catala-Scopes), nicht fehlende Berechnung. Architektur, keine Luecke.

## 4. Der (B)-Fall im Detail (bereits per SendMessage an team-lead eskaliert)

Zeile 3: `bruttoarbeitslohn` / `p2_festzusetzung_einzel` / `bruttoarbeitslohn`, gebunden in
`produkt/bindung/bindung_an_gesamt.yaml:13-15`.

```bash
grep -n "bruttoarbeitslohn" produkt/bindung/bindung_an_gesamt.yaml | head -3
# 13:  - feld_id: bruttoarbeitslohn
# 15:    quelle: {regel_id: p2_festzusetzung_einzel, signatur_slot: bruttoarbeitslohn}
```

Der Slot **existiert wirklich** (Abschnitt 1, beide Wege). `_catala_inputs()`
(`tests/test_bindungstabelle.py:76-86`) sucht aber unter `rules/estg/<rule_id>/*.catala_en` —
also `rules/estg/p2_festzusetzung_einzel/`. Diese Datei liegt unter `rules/estg/p32a/`. Reine
Verzeichnisnamens-Konvention, kein Parsing-Defekt: sobald eine Datei am erwarteten Ort gefunden
wird, extrahiert die Regex korrekt (belegt an den 13 real funktionierenden `gefunden_slot`-Faellen
anderer Regeln). Hier wird die Datei nie gefunden, das Ergebnis ist ein leeres Set — ununterscheidbar
vom Fall "Regel hat wirklich nur 3 statt 4 Inputs".

**Geldrelevanz**, aus dem 2026-08-08-Bericht uebernommen und heute stichprobenartig bestaetigt (Datei
und Testfunktion existieren unveraendert am aktuellen HEAD, s.o. Grep-Nachweis): der `bescheid_via_slots`-
Mechanismus liest `bruttoarbeitslohn`/`veranlagung` per `slots.get(key, default)`, nicht
`slots[key]`. Eine Mutationsprobe des Kollegen "main" (dokumentiert im alten Bericht, Nachtrag 3)
zeigte per echtem HTTP-`/ergebnis`-Aufruf: Umbenennung `bruttoarbeitslohn`→`bruttoarbeitslohn_x` in
der Bindung liess den berechneten Betrag von 1.356.800 ct auf 0 ct fallen, `grund` blieb
`"bestaetigt"` — kein Fehler sichtbar. Aufgefangen wird das im Repo nur von
`tests/test_paket_b_e2e_http.py::test_an_gesamt_durchstich` (bestaetigt heute noch vorhanden,
Zeile 615), und dort nur zufaellig ueber einen harten Centwert-Assert, nicht durch eine
Namens-Strukturpruefung. `test_n_bindung_zeigt_auf_existierende_bedingung` — die Regel, deren
Blindstelle dieser Bericht untersucht — waere strukturell die richtige Stelle, um genau diese
Umbenennung zu fangen, tut es aber nicht, weil sie fuer `p2_festzusetzung_einzel` komplett
uebersprungen wird (Abschnitt 0).

Diese Money-Aussage ist **vermutet** aus dem alten Bericht uebernommen, nicht heute frisch per
HTTP-Aufruf nachgefahren (Zeitbudget) — die Code-Existenz beider beteiligter Stellen (Slot-Reader,
E2E-Test) wurde heute frisch **gemessen** (Grep oben).

## 5. Nebenfrage: die 7 uebersprungenen Regeln (`uebersprungen=7`)

```bash
python3 -c "
import tests.test_bindungstabelle as T
daten = {f: T._load(f) for f in T._bindung_files()}
_, _, skipped = T._n_gefundene_verstoesse(daten, T._rules())
print(sorted(skipped))
"
# -> ['p10_1_3_kv_pv_kind', 'p19_2_versorgungsfreibetrag', 'p22_3_leistungen',
#     'p2_einkunftsarten', 'p2_festzusetzung_einzel', 'p2_festzusetzung_zusammen',
#     'p33b_abs5_kind_uebertragung']
```

Deckt sich exakt mit `REGELN_OHNE_GROUND_TRUTH` (`tests/test_bindungstabelle.py:1052-1073`,
per Assert Zeile 1099 erzwungen). Je Regel der im Code hinterlegte Grund:

| Regel | Grund (Kommentar im Code) | Belastbarkeit |
|---|---|---|
| `p2_festzusetzung_einzel` / `_zusammen` | Catala-Scope schmaler als Bindung (4 bzw. 6 Inputs, Bindung fuehrt mehr) | **Heute unabhaengig geprueft** (Abschnitt 1-3): im Kern richtig, aber mit dem (B)-Ausreisser Zeile 3 |
| `p2_einkunftsarten` | Pseudoregel, hat wirklich keine Signatur | Nicht heute nachgeprueft (kein Scope zu pruefen — Aussage "es gibt keine Signatur" ist nicht durch Gegen-Grep widerlegbar in der verfuegbaren Zeit) |
| `p19_2_versorgungsfreibetrag` | Feld-ID- vs. Slot-Namen-Ambiguitaet, braucht Entscheidung (Verweis auf BACKLOG `offene_frage_p19_2`) | Laut BACKLOG bereits 2026-08-08 gemessen, kein Geldfehler, reine Sichtbarkeitsfrage — nicht heute erneut verifiziert |
| `p10_1_3_kv_pv_kind`, `p33b_abs5_kind_uebertragung` | Aggregationsbruch Kind- vs. Fall-Achse | Plausibel (Kind-Instanzen sind strukturell eine andere Achse als die hier gepruefte Fall-Ebene), nicht heute im Detail nachgemessen |
| `p22_3_leistungen` | Positionale Signatur (`catala_p22_nr3_einkuenfte(betrag_cent: int)`), kein dict-Parameter | Nachvollziehbar aus dem `RUNNER_ACCESSOR_FUER_REGEL`-Mechanismus selbst (Zeile 1139-1156 verlangt genau EINEN dict-Parameter; positionale Signatur passt strukturell nicht) |

Alle 7 Begruendungen sind **auf den ersten Blick tragfaehig**. Nur die ersten zwei
(`p2_festzusetzung_*`) wurden in diesem Bericht vollstaendig nachgemessen — dort zeigt sich: die
Grundaussage stimmt fuer 40 von 42 Faellen, ist aber nicht fluessig genug, um den einen echten
Namenstreffer (Zeile 3) zu erkennen. Die uebrigen 5 Begruendungen wurden nur auf Plausibilitaet
gelesen, nicht mit derselben Zwei-Wege-Methodik durchgemessen — das war ausserhalb des in diesem
Auftrag gesetzten Zeitrahmens ("Nebenfrage, wenn Zeit bleibt").

## 6. Zusammenfassung

- **(B) kommt genau einmal vor**: `bruttoarbeitslohn`/`p2_festzusetzung_einzel` — bereits separat
  eskaliert.
- **40 von 42 sind (A)**, echte Architektur (schmaler Scope, Werte kommen direkt per Feld-ID aus
  api.py, nicht aus dem Catala-Scope).
- **1 ist eine bereits dokumentierte `[Luecke]`**, keine A/B-Frage.
- Die Ausgangszahl "28" ist auf 42 gewachsen (7→21 bei `_einzel`, unveraendert 21 bei `_zusammen`) —
  reines Feature-Wachstum seit 2026-08-08, kein neuer (B)-Fall darunter.
- Die Verstoesse sind **heute nicht live testrot** — sie werden strukturell uebersprungen. Der
  Test ist insofern korrekt gruen UND strukturell blind fuer genau den einen (B)-Fall, den er
  fangen sollte. Kein Widerspruch, zwei verschiedene Ebenen (Testergebnis vs. Diagnosetiefe).
- Empfehlung (nicht umgesetzt, nur Analyse-Auftrag): `_catala_inputs()` koennte den Scope-Namen
  statt des Verzeichnisnamens als Suchschluessel nutzen (Scope `FestzusetzendeEstEinzel`/`Zusammen`
  ist bereits im Quelltext benannt), um Zeile 3 aus `REGELN_OHNE_GROUND_TRUTH` zu loesen — das wuerde
  aber sofort 40 neue (A)-"Verstoesse" produzieren, die dann in `SIGNATUR_SLOT_ZEIGT_INS_LEERE`
  einzeln dokumentiert werden muessten (dieser Bericht liefert dafuer bereits die vollstaendige
  Liste, Abschnitt 3).

## Anhang (2026-08-12, zweite Runde): die uebrigen 6 der 7 auf reale Catala-Scopes geprueft

Team-lead-Nachfrage: (1) p2_festzusetzung_zusammen unabhaengig pruefen statt annehmen, (2) fuer die
uebrigen 7 pruefen, ob irgendwo unter `rules/` ein `declaration scope` zu ihnen gehoert, (3) sagen,
ob `_catala_inputs()` reparierbar ist und was das je Regel GEMESSEN aendern wuerde.

### A1. Zahlenkorrektur: 7, nicht 9

Team-lead hatte 9 uebersprungene Regeln gemessen (inkl. `p10_1_9_schulgeld`,
`p33_2a_fahrtkostenpauschale`). **Gemessen** gegen den echten `_n_gefundene_verstoesse`-Lauf
(nicht nur `rules.yaml` + Verzeichnis-Fallback, sondern inklusive der dritten Ground-Truth-Quelle
`RUNNER_ACCESSOR_FUER_REGEL`, Zeile 1133):

```bash
python3 -c "
import tests.test_bindungstabelle as T
daten = {f: T._load(f) for f in T._bindung_files()}
_, _, skipped = T._n_gefundene_verstoesse(daten, T._rules())
print(sorted(skipped))
print('p10_1_9_schulgeld' in skipped, 'p33_2a_fahrtkostenpauschale' in skipped)
"
# -> ['p10_1_3_kv_pv_kind', 'p19_2_versorgungsfreibetrag', 'p22_3_leistungen', 'p2_einkunftsarten',
#     'p2_festzusetzung_einzel', 'p2_festzusetzung_zusammen', 'p33b_abs5_kind_uebertragung']
# -> False False
```

`p10_1_9_schulgeld`/`p33_2a_fahrtkostenpauschale` haben Ground Truth ueber
`RUNNER_ACCESSOR_FUER_REGEL` (golden/runner.py-Accessor-Funktionen, AST-gelesen) — sie werden im
`elif`-Zweig behandelt (Zeile 1179-1181) und erreichen den `_catala_inputs`-Fallback nie. Team-leads
Skript hat vermutlich nur (1) rules.yaml und (2) Verzeichnis-Fallback geprueft und diese dritte
Quelle nicht mitgezaehlt — daher 9 statt 7. Bleibt bei den **7** aus `REGELN_OHNE_GROUND_TRUTH`.

### A2. p2_festzusetzung_zusammen: NICHT angenommen, sondern geprueft — bestaetigt (B)-artig

```bash
grep -n "^declaration scope FestzusetzendeEst\|^  input " rules/estg/p32a/einkommensteuertarif.catala_en
# Zeile 382: declaration scope FestzusetzendeEstZusammen:
#   input bruttoarbeitslohn_a / werbungskosten_a / bruttoarbeitslohn_b / werbungskosten_b /
#         sonderausgaben_gemeinsam / veranlagungszeitraum content ...
```

Selbe Datei wie `p2_festzusetzung_einzel` (`rules/estg/p32a/einkommensteuertarif.catala_en`), 63
Zeilen weiter unten. `_catala_inputs("p2_festzusetzung_zusammen")` sucht unter
`rules/estg/p2_festzusetzung_zusammen/*.catala_en` — Verzeichnis existiert nicht (bestaetigt unten,
A3). Damit gilt fuer **beide** p2_festzusetzung-Regeln: Ground Truth existiert, ist real, liegt aber
unter einem Verzeichnis, das nicht dem `rule_id` entspricht. Der Kommentar in
`REGELN_OHNE_GROUND_TRUTH` ("Catala-Scope ist schmaler als die Bindung") ist damit fuer beide
Regeln in der Sache richtig (schmaler ist er wirklich — Abschnitt 3 oben, 40/42 sind echte (A)),
aber in der BEGRUENDUNG unpraezise: der Lookup liefert nicht "schmaler", er liefert NULL — die
Aussage "schmaler" war bisher nie pruefbar, nur angenommen.

### A3. Verzeichnis-Check aller 7 — keines hat ein Verzeichnis mit eigenem Namen

```bash
for rid in p2_einkunftsarten p2_festzusetzung_einzel p2_festzusetzung_zusammen \
           p19_2_versorgungsfreibetrag p10_1_3_kv_pv_kind p33b_abs5_kind_uebertragung p22_3_leistungen; do
  [ -d "rules/estg/$rid" ] && echo "$rid: DIR" || echo "$rid: kein Verzeichnis"
done
# -> alle 7: kein Verzeichnis
```

### A4. Die uebrigen 5 (nicht p2_festzusetzung_*) gegen ALLE Scopes im Repo geprueft — kein Treffer

```bash
grep -rn "^declaration scope" rules/ --include="*.catala_en" \
  | grep -v "declaration scope Test\|declaration scope Falle\|declaration scope Fall[0-9]"
```

Liefert 39 echte (Nicht-Test-)Scopes im ganzen `rules/`-Baum (vollstaendige Liste im Bericht-Log).
Keiner davon heisst oder handelt semantisch von: `p2_einkunftsarten`, `p19_2_versorgungsfreibetrag`,
`p10_1_3_kv_pv_kind`, `p33b_abs5_kind_uebertragung`, `p22_3_leistungen`.

- `p2_einkunftsarten`: "Einkunftsart" kommt nur in Prosa-Kommentaren vor (`p32a/einkommensteuertarif.catala_en:341/444/458/495`,
  `arbeitnehmerfall/tests_veranlagung_gesamt.catala_en`), nie als Scope-Name — geprueft per Grep, nicht angenommen.
- `p19_2_versorgungsfreibetrag`: kein Treffer fuer "Versorgungsfreibetrag"/"versorgung" als Scope irgendwo.
- `p10_1_3_kv_pv_kind`: es gibt `KrankenPflegeVorsorge` (`p10_1_3_3a/krankenpflegevorsorge.catala_en:33`) — das ist
  aber die ERWACHSENEN-Vorsorge (Fall-Achse), keine Kind-Variante. Bestaetigt exakt den im Code
  hinterlegten Grund ("Aggregationsbruch: Kind-Achse gegen Fall-Achse") — es gibt wirklich keinen
  zweiten, Kind-spezifischen Scope, der uebersehen wuerde.
- `p33b_abs5_kind_uebertragung`: kein Treffer.
- `p22_3_leistungen`: kein Treffer (auch nicht unter anderem Namen fuer "sonstige Leistungen"/§22 Nr.3).

**Fuer diese 5 ist die Begruendung in `REGELN_OHNE_GROUND_TRUTH` damit heute unabhaengig bestaetigt**
(nicht nur plausibel gelesen wie im Hauptbericht Abschnitt 5): es gibt tatsaechlich keinen Catala-Scope,
egal unter welchem Verzeichnisnamen. Nur die p2_festzusetzung-Regeln sind der (B)-artige Sonderfall.

### A5. Ist `_catala_inputs()` reparierbar — und was aendert das GEMESSEN?

Reparierbar ja, aber **nicht generisch**: `rule_id` ("p2_festzusetzung_einzel", snake_case) und
Scope-Name ("FestzusetzendeEstEinzel", PascalCase mit anderem Wortstamm — "festzusetzung" vs.
"festzusetzende", plus eingefuegtes "Est") stehen in keiner algorithmisch ableitbaren Beziehung.
Ein Fix braucht eine explizite `rule_id -> scope_name`-Zuordnungstabelle, strukturell identisch zu
`RUNNER_ACCESSOR_FUER_REGEL` (Zeile 1133) — kein Parser-Umbau, sondern ein zweiter Mapping-Eintrag
pro Regel.

Gemessen (nicht geschaetzt), was ein solcher Fix je Regel aendern wuerde — Simulation mit den
ECHTEN Scope-Inputs statt `set()`, gegen die 42 Zeilen aus Abschnitt 3:

| Regel | Zeilen betroffen | Sofort korrekt aufgeloest (kein Verstoss mehr) | Neu als `gefunden_slot` faellig (muesste in SIGNATUR_SLOT_ZEIGT_INS_LEERE dokumentiert werden) |
|---|---|---|---|
| `p2_festzusetzung_einzel` | 21 | **1** (Zeile 3, `bruttoarbeitslohn` — der (B)-Fall) | 20 |
| `p2_festzusetzung_zusammen` | 21 | 0 (kein einziger Slotname trifft `_a`/`_b`/`sonderausgaben_gemeinsam`/`veranlagungszeitraum`) | 21 |
| **Summe** | **42** | **1** | **41** |

Herleitung: Der `luecken`-Zweig (Zeile 1199-1205) prueft `signatur_slot` genauso gegen `inputs` wie
der `bindungen`-Zweig — die dokumentierte `[Luecke]` (Zeile 37, `pv_entnahmen`) bekaeme bei einem
Fix ebenfalls einen `gefunden_slot`-Eintrag, obwohl sie bereits einen eigenen `grund` hat. Ein Fix
waere also kein Nullsummenspiel: er loest **1** echten blinden Fleck (den (B)-Fall) und erzeugt
**41** neue, einzeln zu dokumentierende Eintraege in der Ausnahmeliste — mehr Dokumentationsaufwand
als Ertrag, es sei denn, man haelt gerade den (B)-Fall fuer wichtig genug (Geldpfad, Abschnitt 4).

### A6. Fazit des Anhangs

- Kein systematischer Blindspot ueber die 9 (recte: 7) hinaus — team-leads Vermutung war berechtigt
  zu pruefen, das Ergebnis ist negativ (bestaetigt: NUR die p2_festzusetzung-Regeln betroffen).
  Die vorhandene Dokumentation ist fuer 5 von 7 Regeln korrekt und fuer 2 von 7 (p2_festzusetzung_*)
  in der Sache richtig, aber mit einer unpraezisen Begruendung ("schmaler" statt "Lookup findet
  nichts, ist zufaellig plausibel").
- `_catala_inputs()` ist reparierbar, aber nur mit einer expliziten Zuordnungstabelle (kein
  genereller Algorithmus) — Aufwand vergleichbar mit dem bereits bestehenden
  `RUNNER_ACCESSOR_FUER_REGEL`-Muster.
- Gemessener Nutzen eines Fixes: 1 geloester Blindspot (Zeile 3), 41 neue Dokumentationspflichten.
