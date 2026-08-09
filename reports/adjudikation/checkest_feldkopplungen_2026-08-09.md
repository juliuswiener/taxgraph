# checkESt-Feldkopplungen — Fallstrick-Suche vor dem Weiterbau der restlichen 9

Auftrag: bevor zwei Worker parallel an den 9 Einzel-Beanstandungen bauen, systematisch messen,
welche Feldkombinationen NEUE Fehler erzeugen statt bestehende zu schließen. Basis: der
9-Fehler-Einzelveranlagungsfall aus `tests/test_checkest_durchstich.py::_fall_einzel` (identisch
nachgebaut, s. Messmethode unten). Gemessen 2026-08-09/10 gegen ERiC 44.2.4.0, `ESt_2025`.

**Kein Produktionscode geändert.** Alle Kz wurden per `copy.deepcopy(result)` +
Dict-Injektion in `result["deklaration"]` NACH `est_mapping.deklariere()` gesetzt — das umgeht
jede Bindung, `erzeuge_xml()` ist bindungs-agnostisch. Skripte liegen in Scratch:
`fallstrick_kopplungen.py` (19 Varianten) + `fallstrick_kopplungen2.py` (3 Nachtrag-Varianten,
korrektes Dezimalformat). Hersteller-ID nirgends im Klartext (immer `sed s/$ELSTER_HERSTELLER_ID/<ID>/g`).

Schema-Fundstellen unten beziehen sich auf
`~/02_Software/eric/doc_extract/ERiC-44.2.4.0/Dokumentation/Datenarten/ElsterErklaerung/ESt/Schema/2025/E10-2025.xsd`.
**Beide Schema-Dateien geprüft**: `elster11_E10_2025_extern.xsd` enthält für alle 5 hier
untersuchten Bereiche 0 Treffer (`grep -c` je Typname) — bestätigt erneut, dass die Inhalte
ausschließlich in `E10-2025.xsd` stehen (s. `vorsatz_block_2026-08-09.md`).

```
SCHEMA_DIR=~/02_Software/eric/doc_extract/ERiC-44.2.4.0/Dokumentation/Datenarten/ElsterErklaerung/ESt/Schema/2025
for pat in 'name="Vorsatz"' 'Art_Erkl' 'BV_m275349613' \
           'Enum_N_ArbL_LStB_1_5_Sum_E0200002' 'Enum_Religionsschluessel_ab_VZ_2014_3'; do
  grep -c "$pat" "$SCHEMA_DIR/elster11_E10_2025_extern.xsd"
done
# -> 0 0 0 0 0
```

## Vorab-Korrektur der eigenen Annahme — ZURÜCKGENOMMEN (2026-08-10)

**Dieser Abschnitt stand ursprünglich hier und war falsch — stehen gelassen als Beleg, nicht
gelöscht.** Behauptet wurde: Geburtsdatum, Adresse, Religion, BV-Sonderfeld und Art_Erkl E0100001
seien "bereits gebunden", 7 der 9 Beanstandungen also ein reines Fixture-Problem statt Bau. Das
kam aus `grep produkt/bindung/*.yaml` gegen den **Arbeitsbaum** — dort lagen die Zeilen, weil ein
anderer Worker sie gerade uncommitted anlegte. Gegen `git show HEAD:` geprüft (team-lead, dann von
mir bestätigt):

```
git show HEAD:produkt/bindung/bindung_an_gesamt.yaml | wc -l          # -> 938
wc -l < produkt/bindung/bindung_an_gesamt.yaml                        # -> 1174 (Arbeitsbaum)
git show HEAD:produkt/bindung/bindung_an_gesamt.yaml \
  | grep -c "E0100201\|E0100301\|E0100401\|E0100402\|E0100601\|E0100602\|E0101104\|E0102002\|E0100001\b"
# -> 0
git show HEAD:produkt/bindung/bindung_p51a_kirchensteuer.yaml | grep -n "E0100402"
# -> kein Treffer
```

In HEAD (`f2dd9e3`) existiert **keine** der 8 Bindungen. Es lief keine Doppelarbeit — der
Stammdaten-Worker baute sie gerade, korrekt und zum ersten Mal. Meine Warnung hätte ihn fälschlich
gestoppt, wäre sie ungeprüft geglaubt worden. Einzig `E0200002` (Steuerklasse) ist tatsächlich
weder in HEAD noch im damaligen Arbeitsbaum vorhanden — der einzige Teil der ursprünglichen
Korrektur, der stand hält.

**Lehre, dieselbe wie beim Guard-"Bug" einen Tag zuvor**: jede Aussage über den Ist-Zustand des
Repos gegen `git show HEAD:<datei>` prüfen, nie gegen den Arbeitsbaum, solange andere Agenten im
selben Checkout laufen. Die fünf Feldkopplungen weiter unten sind davon unberührt — sie kamen aus
direkter Kz-Injektion in `result["deklaration"]` nach `deklariere()`, nie aus einer Bindungsdatei,
also unabhängig davon, was zu dem Zeitpunkt gebunden war oder nicht.

## Messmethode

```python
# Basis: identisch zu tests/test_checkest_durchstich.py::_fall_einzel (12 Store-Felder, veranlagung=einzel)
basis = est_mapping.deklariere(snapshot, traverser.lade_bindung())
r = copy.deepcopy(basis)
r["deklaration"]["E0100402"] = "11"          # Kz direkt injiziert, keine Bindung angefasst
xml = elster_xml.erzeuge_xml(r, vz=2025, hersteller_id=HID, abgabefaehig=True, **_ABSENDER)
rc, texte = checkest_gate.validate(xml, "ESt_2025")   # rc, Liste amtlicher <Text>-Meldungen
```

Baseline (Befehl: `python3 fallstrick_kopplungen.py`, erste Sektion):

```
rc=610001002  n=9
  Kein Hauptvordruck ESt 1 A vorhanden.
  Auf dem Hauptvordruck ESt 1 A ist anzugeben, ob es sich um eine Einkommensteuererklärung ...
  Religion nicht angegeben oder kein gültiger Wert (... Person A).
  Bitte geben Sie den Namen und Vornamen an (... Person A).
  Bitte geben Sie die vollständige derzeitige Adresse an ... (... Person A).
  Bitte geben Sie Ihre Bankverbindungsdaten an oder erklären Sie ... keine Bankverbindung ...
  Auf den Anlagen KAP und/oder KAP-BET wurden Kapitalerträge erklärt ... Grund ...
  Die Steuerklasse wurde ... auf der Anlage N nicht eingetragen (... Person A).
  Arbeitslohn laut Lohnsteuerbescheinigung(en) Steuerklassen 1 - 5 angegeben, Lohnsteuer jedoch nicht ...
```

## Meta-Befund: checkESt kaskadiert selbst (Geburtsdatum-Maske)

Fast jede Variante, die den Hauptvordruck-Container zum ersten Mal entstehen lässt (Religion,
Art_Erkl, Adresse — alles, was in `ESt1A/Allg/A` liegt), bringt **eine neue** Meldung mit:
„Es wurde kein Geburtsdatum angegeben (steuerpflichtige Person / Ehemann / Person A)." Das ist
**kein Bug der Injektion** — es ist checkESt selbst, das den Geburtsdatum-Check erst prüft, sobald
der Container existiert. Solange „Kein Hauptvordruck ESt 1 A vorhanden" steht, ist der
Geburtsdatum-Check unerreichbar (genau das Maskierungs-Muster aus
`kegel-feld-weglassen-verliert-still-geld`, nur diesmal im amtlichen Prüfer statt im eigenen Code).
**Konsequenz für die zwei Worker:** ein Fortschrittsschritt, der zum ersten Mal Person-A-Stammdaten
in den Hauptvordruck bringt, wird fast immer +1 zeigen, bis Geburtsdatum mitgeliefert wird —
kein Kreis, sondern ein vorhersagbarer Einmal-Effekt. `stammdaten_geburtsdatum` wird im laufenden
Stammdaten-Bau ohnehin mitgebaut (s. Korrektur oben) — beim Zusammenführen einfach mitschicken
vermeidet den Sprung.

## Befund-Tabelle je Kandidat

### A) Religion / Kirchensteuerpflicht (E0100402)

| Variante | Kz | Δ ggü. Baseline | XSD-sichtbar? |
|---|---|---|---|
| `religion_11_allein` | `E0100402="11"` | −2 (Religion-, Hauptvordruck-Fehler weg), +1 Geburtsdatum-Kaskade | Enum-Wert ja, Pflicht-Charakter nein |
| `religion_03_allein` | `E0100402="03"` | −2/+2: Religion-/Hauptvordruck-Fehler weg, aber **neu**: „Arbeitslohn laut Lohnsteuerbescheinigung(en) ... Kirchensteuer jedoch nicht ... Gegebenenfalls ist die Kirchensteuer mit dem Wert 0 zu erklären" + Geburtsdatum | **rein empirisch**, im XSD nirgends |
| `religion03_plus_kirchensteuer_KOMMA` | `+E0200501="0,00"` | Meldung **kippt nur die Richtung**: jetzt „Kirchensteuer ... angegeben, die Lohnsteuer jedoch nicht" | rein empirisch |

**Befund**: sobald Religion kirchensteuerpflichtig ist (jeder Wert außer `"11"`), verlangt checkESt
zusätzlich **sowohl** Lohnsteuer (`E0200301`) **als auch** Kirchensteuer (`E0200501`) auf der
Anlage N — as-a-pair, nicht einzeln (0 EUR reicht als Wert, s.u.). Das ist exakt der von Julius in
`bindung_p36_abschlusszahlung.yaml` ausgeschlossene Lohnsteuer-Kz. **Fallstrick**: ein Worker, der
KiSt-Konfession bindet (bereits erledigt, `bindung_p51a_kirchensteuer.yaml`), OHNE gleichzeitig
Lohnsteuer/Kirchensteuer auf Anlage N zu setzen, tauscht einen Fehler gegen einen anderen, sobald
`kist_konfession` real auf einen kirchensteuerpflichtigen Wert gesetzt wird — bisher unsichtbar,
weil der Fixture-Fall keine Konfession deklariert.

- Art_Erkl `E0100009` (KiSt auf Kapitalerträge) hat **keinen** Effekt auf diese Kopplung
  (`religion_03_plus_artErkl_E0100009`: dieselben 2 neuen Fehler wie ohne). Es ist ein anderer
  KiSt-Begriff (Kapitalertragsteuer-Anmeldung, nicht Lohnsteuerklasse-KiSt).

### B) Bankverbindung (BV_m275349613_CType, E10-2025.xsd:8714)

| Variante | Kz | Ergebnis |
|---|---|---|
| `bv_keine_allein` | `E0102002="X"` | sauber: −1 (BV-Fehler weg), +1 Geburtsdatum-Kaskade |
| `bv_iban_allein` | `E0102102=IBAN` | **neu**: „Bei den Bankverbindungsdaten fehlt die Angabe des Kontoinhabers." — `Kto_Inh`-Block (E10-2025.xsd:8739) ist trotz `minOccurs="0"` faktisch Pflicht, sobald eine IBAN steht |
| `bv_beides` | beide gleichzeitig | **neu, hart**: „Es wurde angegeben, dass keine Bankverbindung vorhanden ist, es wurden aber gleichzeitig Angaben zu einer Bankverbindung getätigt." — echte Exklusivitätsregel |

**Befund**: `E0102002` und `E0102102`/`Kto_Inh` sind im Schema drei unabhängig optionale
Geschwister — die Exklusivität und die Kto_Inh-Pflichtkopplung existieren **nur** in checkESt,
nirgends im XSD. Der laufende Stammdaten-Bau (uncommitted zum Messzeitpunkt, `Korrektur oben`)
bindet unter `stammdaten_keine_bankverbindung` ausschließlich das Sonderfeld — laut Kommentar dort
bewusst „IBAN/BIC-Erfassung NICHT gebaut". Das trifft die sichere Seite dieser Kopplung; kein
Fallstrick, solange niemand zusätzlich IBAN bindet, ohne `Kto_Inh` mitzudenken.

### C) Art_Erkl (5 Kz, E10-2025.xsd:8422) ↔ Vorsatz-Unterfallart

Unterfallart steht in `_vorsatz()` fest auf `"10"` (Produktionscode, hier nicht verändert) — keine
Variante hat daran gerüttelt. Gemessen wurde nur, ob die 5 Art_Erkl-Kz sich untereinander vertragen:

| Variante | Δ ggü. Baseline |
|---|---|
| `E0100001` allein (Einkommensteuererklärung — die einzige gebundene) | sauber: −2, +1 Geburtsdatum-Kaskade |
| `E0100002` allein (Arbeitnehmer-Sparzulage) | **neu**: „... jedoch nicht angegeben, dass für alle vom Arbeitgeber ... [Antrag gestellt wurde]" — braucht ein weiteres, nicht getestetes Begleit-Kz |
| `E0100003` allein (Verlustvortrag) | sauber wie `E0100001` |
| `E0100302` allein (Mobilitätsprämie) | **neu, doppelt**: „verpflichtend auch eine Einkommensteuererklärung abzugeben (§ 105 Abs. 2 EStG)" + „Anlage Mobilitätsprämie wurde nicht übermittelt" |
| alle 5 zusammen | dieselben 3 neuen Fehler wie `E0100302` allein + `E0100002`s Kopplung — `E0100001`-Mitnahme neutralisiert NICHT die Anforderungen der anderen 4 |

**Befund**: die Bindungs-Entscheidung im laufenden Stammdaten-Bau (nur `E0100001`, Kommentar
`bindung_an_gesamt.yaml:1018` „die anderen 4 ... NICHT gebaut" — uncommitted zum Messzeitpunkt,
s. Korrektur oben) trifft genau die kopplungsfreie Variante. Jeder der 4 unbebundenen Flags zieht
eine eigene, XSD-unsichtbare Zusatzpflicht nach sich — kein einzelner davon ist mit einem Kz allein
abschließbar.

### D) Steuerklasse (E0200002) ↔ Lohnsteuer (E0200301), LStB_1_5_Sum (E10-2025.xsd:17667)

| Variante | Kz | Ergebnis |
|---|---|---|
| `steuerklasse_allein` | `E0200002="1"` | **sauber**: −1, 0 neu |
| `lohnsteuer_allein` (int, `13500`) | `E0200301=13500` | **neu, Format**: „Feld '.../E0200301[1]': Geldbeträge müssen vom Format '0,00' mit genau 2 Nachkommastellen sein." |
| `steuerklasse_plus_lohnsteuer_KOMMA` | `E0200002="1", E0200301="13500,00"` | **sauber**: −2, 0 neu |

**Doppel-Fallstrick, konkret produktionsrelevant**: `E0200301` ist eine
`DezimalzahlNichtNegOhneFuehrNull_..._MinNK2_MaxNK2`-Type — verlangt Komma+2-Nachkommastellen als
String. Die bestehende cent→Kz-Konvertierung `_cent_nach_kz()` (`est_mapping.py:65`) gibt für
JEDEN Nicht-`E60`-Kz einen reinen Integer zurück (Ganzzahl-EUR, kein Komma) — das passt für
`E0200201` (Bruttoarbeitslohn, `GanzzahlOhneFuehrNull`-Type), würde aber bei einer künftigen
Lohnsteuer-Bindung ohne Sonderbehandlung ein **falsch formatiertes** Kz erzeugen und exakt den
oben gezeigten neuen Fehler auslösen. Betrifft aber nur eine hypothetische künftige Bindung:
`E0200301` bleibt laut `bindung_p36_abschlusszahlung.yaml:24` **dauerhaft ausgeschlossen**
(Julius-Entscheidung, Doppel-Erfassung ggü. eLStB) — Steuerklasse allein ist der einzige der beiden
Kz, den Task #7 tatsächlich bauen sollte, und der schließt sauber ohne Nebenwirkung.

### E) Adresse ↔ Empfänger-Finanzamt

| Variante | Kz | Ergebnis |
|---|---|---|
| `adresse_voll_bayern_plz` | Straße+Hausnr.+PLZ `80331`+Wohnort München | sauber: −2, +1 Geburtsdatum-Kaskade |
| `adresse_voll_nicht_bayern_plz` | dieselben Felder, PLZ `10115` Berlin, gegen `empfaenger_finanzamt="9181"` (Bayern) | **identisch** zur Bayern-Variante — kein zusätzlicher Fehler |
| `adresse_nur_plz_ohne_strasse` / `adresse_nur_strasse_ohne_plz` | Teilangabe | „Adresse unvollständig" bleibt korrekt stehen (erwartetes Verhalten, keine Überraschung) |

**Negativ-Befund (wichtig gegen die eigene Hypothese)**: anders als bei der StNr/OrdNrArt-Falle
(`vorsatz_block_2026-08-09.md`) prüft checkESt **keine** Konsistenz zwischen der Wohnadresse der
Person und dem `empfaenger_finanzamt`-Parameter — eine Berliner PLZ gegen ein bayerisches
Finanzamt zieht keinerlei zusätzliche Beanstandung. Das ist plausibel: die StNr-Präfix-Prüfung ist
eine Weiterleitungs-Konsistenz (an wen wird das Verfahren zugestellt), die Wohnadresse ist reine
Auskunft ohne Routing-Funktion. Kein Fallstrick hier — die Analogie trägt nicht.

## Zusammenfassung für die zwei Worker

1. **Reihenfolge einhalten**: Stammdaten-Felder (Geburtsdatum, Adresse, Religion, BV-Sonderfeld,
   Art_Erkl E0100001) zusammen ausfüllen, nicht einzeln — sonst tritt die Geburtsdatum-Kaskade
   pro Zwischenschritt neu auf. Stand HEAD `f2dd9e3`: keine dieser Bindungen existiert dort noch
   (s. Korrektur oben) — der Stammdaten-Worker legt sie gerade neu an, das ist Bau, kein
   Fixture-Fix.
2. **Religion ≠ nur ein Kz**: sobald `kist_konfession` real auf kirchensteuerpflichtig gesetzt
   wird, müssen Lohnsteuer UND Kirchensteuer auf Anlage N mitgeliefert werden (0,00 reicht) —
   sonst 2 neue statt 0 alte Fehler. Kollidiert mit der bestehenden Lohnsteuer-Ausschluss-
   Entscheidung (`bindung_p36_abschlusszahlung.yaml:24`) — braucht Julius' Wort, bevor
   `kist_konfession` produktiv gebunden wird.
3. **Art_Erkl**: bei `E0100001` bleiben, die anderen 4 NICHT antasten ohne die jeweilige
   Zusatzpflicht (Sparzulage-Begleitangabe, Mobilitätsprämie-Anlage) mitzubauen.
4. **Steuerklasse ja, Lohnsteuer nein**: `E0200002` (Steuerklasse) ist ungebunden in HEAD UND im
   damaligen Arbeitsbaum — echte Lücke, offener BACKLOG-Punkt #7, und der saubere Teil davon
   (0 neue Fehler bei Alleinstellung, s. Tabelle D). `E0200301` (Lohnsteuer) bleibt
   Julius-Adjudikation (BACKLOG #11), nicht Bau-Aufgabe der beiden Worker.
5. **Bankverbindung**: die Bindung im laufenden Stammdaten-Bau (nur Sonderfeld) trifft die
   kopplungsfreie Seite — beim Zusammenführen nicht um IBAN/BIC erweitern, ohne `Kto_Inh`
   mitzubauen.
6. **Adresse/Finanzamt**: keine Kopplung gefunden — Adresse einfach vollständig ausfüllen, kein
   Präfix-artiges Risiko wie bei StNr.

## Reproduktion

```
set -a; . ./.env; set +a
python3 <scratch>/fallstrick_kopplungen.py  2>&1 | sed "s/$ELSTER_HERSTELLER_ID/<ID>/g"
python3 <scratch>/fallstrick_kopplungen2.py 2>&1 | sed "s/$ELSTER_HERSTELLER_ID/<ID>/g"
```
Volle Rohausgabe (HID bereits maskiert) in
`<scratch>/fallstrick_lauf1.txt` und `<scratch>/fallstrick_lauf2.txt`
(`/tmp/claude-1000/-home-julius-00-projects-168-TaxGraph-taxgraph/db772487-ddbf-429b-a0e4-37a31085356e/scratchpad/`).
